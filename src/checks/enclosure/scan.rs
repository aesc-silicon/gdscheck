// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The enclosure scan as the checks drive it: every region of the enclosed layer owned by
//! the tile holding its centroid, tested once against the enclosing regions there, its
//! margin read as facing edge pairs on the integer outlines and the verdict taken per
//! [`Sides`].  A minimum and a maximum are one scan with the comparison turned round;
//! what differs beyond that is said where it differs.
//!
//! An inner wall lying exactly on the outer contour - a **coincident** pair - is either a
//! genuinely flush margin of nothing or the cut a boolean left when the enclosed layer was
//! intersected with the enclosing one.  The geometry cannot tell the two apart, and
//! neither can KLayout: its rules pick per flag (`consider_intersecting_edges`,
//! `without_distance(0)`), which `skip_coincident` (pair-level) and `skip_clipped`
//! (region-level, "surrounded entirely by") mirror.

use super::Kind;
use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly, TileMap, merged_centroid_dbu};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

/// What one tile scans: the tile, the enclosed shapes with their regions, and whether
/// each shape is whole (put together from its pieces) or one tile's piece of it.
type Job<'a> = ((i32, i32), Shapes<'a>, bool);

/// Enclosed shapes with their regions, each borrowed from the layer or put together
/// from its pieces.
type Shapes<'a> = Vec<(Cow<'a, MergedPoly>, usize)>;

/// One tile's core pieces of the enclosed layer: each as its index in the tile's list,
/// with its region.
type TilePieces = ((i32, i32), Vec<(usize, usize)>);

/// What one tile reports: the enclosed region, the wall the margin was read on, the
/// margin, the tile, and the report itself.
type Report = (
    usize,
    Option<Seg>,
    Option<(i128, i128)>,
    (i32, i32),
    Violation,
);

/// Which metric an enclosure rule measures its margin in.
///
/// KLayout's own default is euclidian, and most of GF180's enclosure rules ask for it
/// explicitly; `projection` restricts the measurement to facing parallel runs. The two
/// agree on orthogonal geometry and part company at any corner that is not square.
fn euclidian(rule: &RuleDefinition, name: &str) -> Option<bool> {
    match super::super::params::mode(rule, name, "metric") {
        Ok(None) | Ok(Some("projection")) => Some(false),
        Ok(Some("euclidian")) => Some(true),
        Ok(Some(other)) => {
            eprintln!(
                "[{}] {name}: metric can only be `projection` or `euclidian`, not `{other}`",
                rule.id
            );
            None
        }
        Err(super::super::params::NotAWord) => None,
    }
}

/// The enclosing layer over an enclosed shape that reaches past the zone this tile's
/// copy is exact in, assembled from the cores the shape's box grown by the value
/// touches; `None` when the tile's own copy covers it.  A copy is exact out to its halo
/// and no further, and an enclosed shape can be longer than that - a row of abutting
/// cells' Activ merges into one bar - so the tile's copy of the enclosing layer ended
/// short of the bar's far end, and NW.c reported the bar not enclosed at all.
fn outer_over(
    map_a: &TileMap,
    tile: i64,
    halo: i64,
    core: &Core,
    bm: &MergedPoly,
    value_dbu: i64,
    within: bool,
) -> Option<Vec<MergedPoly>> {
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in &bm.outer {
        x0 = x0.min(p.x as i64);
        y0 = y0.min(p.y as i64);
        x1 = x1.max(p.x as i64);
        y1 = y1.max(p.y as i64);
    }
    let g = value_dbu + 1;
    let bx = (x0 - g, y0 - g, x1 + g, y1 + g);
    if bx.0 >= core.x0 - halo
        && bx.1 >= core.y0 - halo
        && bx.2 <= core.x1 + halo
        && bx.3 <= core.y1 + halo
    {
        return None;
    }
    // A piece cut to its tile's zone measures nothing past its box, and the layer
    // within the box is all it needs; a whole shape is read out to the cores' edges,
    // so a maximum sees no wall the box cut.
    Some(if within {
        crate::merge::assemble_within(map_a, tile as i32, bx)
    } else {
        crate::merge::assemble_over(map_a, tile as i32, bx)
    })
}

/// The squared length of the stretch a report is on, in µm².
fn edge_len2(v: &Violation) -> f64 {
    match v.geometry {
        crate::violation::ViolationGeometry::Edge { x1, y1, x2, y2 } => {
            (x2 - x1).powi(2) + (y2 - y1).powi(2)
        }
        _ => 0.0,
    }
}

/// Whether two margins, each the square of a ratio, are one margin.
fn same((an, ad): (i128, i128), (bn, bd): (i128, i128)) -> bool {
    an * bd == bn * ad
}

/// Whether the stretch `a` is longer than `b`.
fn longer(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let len2 = |(x1, y1, x2, y2): (f64, f64, f64, f64)| (x2 - x1).powi(2) + (y2 - y1).powi(2);
    len2(a) > len2(b)
}

/// The walls of one enclosed shape with the margin read on each: a shape has a few,
/// and a map per contact was an allocation per contact.
struct Walls(Vec<(Seg, Read)>);

impl Walls {
    fn new() -> Walls {
        Walls(Vec::new())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn get(&self, w: &Seg) -> Option<&Read> {
        self.0.iter().find(|(s, _)| s == w).map(|(_, r)| r)
    }

    /// The read on `w`, set to `m` when the wall is new.  The caller keeps the worse
    /// of the two on the wall; at the same margin the longer stretch stays, so a
    /// corner on the outer contour does not stand for a wall lying along it.
    fn entry(&mut self, w: Seg, m: Read) -> &mut Read {
        match self.0.iter().position(|(s, _)| *s == w) {
            Some(i) => &mut self.0[i].1,
            None => {
                self.0.push((w, m));
                &mut self.0.last_mut().expect("pushed").1
            }
        }
    }
}

/// Whether a point (DBU) lies inside the layer's merged geometry, tested against the
/// bucket of the tile that *contains the point* - where that bucket's union is complete
/// by construction (every polygon covering a point inside `tile + halo` has a bounding
/// box intersecting the bucket).  This is the enclosure engine's "wall reality check":
/// a facing pair measured in a *neighbouring* tile's bucket can see a fake wall where
/// the outer union was truncated at that bucket's halo (a partial-union seam); probing
/// just beyond the wall in the probe's own tile exposes it - if the probe is still
/// inside the layer, the wall does not exist in the true merge and the pair is dropped.
fn point_in_layer_at_own_tile(
    map: &TileMap,
    boxes: &Boxes,
    tile_dbu: i64,
    (px, py): (f64, f64),
) -> bool {
    let (tx, ty) = (
        (px / tile_dbu as f64).floor() as i32,
        (py / tile_dbu as f64).floor() as i32,
    );
    let (Some(polys), Some((bs, grid))) = (map.get(&(tx, ty)), boxes.get(&(tx, ty))) else {
        return false;
    };
    // A probe is cast only against the polygons whose box holds it, found in the
    // tile's grid: a contact's probe against every plate of metal in the tile was
    // most of an enclosure rule, and against every box of a dense active layer still
    // half of one.
    grid.at(px.round() as i32, py.round() as i32)
        .iter()
        .any(|&i| {
            let (x0, y0, x1, y1) = bs[i as usize];
            px >= x0 as f64
                && px <= x1 as f64
                && py >= y0 as f64
                && py <= y1 as f64
                && crate::merge::point_in_merged(px, py, &polys[i as usize])
        })
}

/// The bounding box of every polygon of every tile, and the tile's grid over them.
type Boxes = HashMap<(i32, i32), (Vec<(i32, i32, i32, i32)>, crate::merge::CellGrid)>;

fn boxes_of(map: &TileMap, tile_dbu: i64) -> Boxes {
    map.par_iter()
        .map(|(k, polys)| {
            let boxes: Vec<_> = polys.iter().map(crate::merge::poly_bbox).collect();
            let (x0, y0) = (
                (k.0 as i64 * tile_dbu) as i32,
                (k.1 as i64 * tile_dbu) as i32,
            );
            let grid = crate::merge::CellGrid::new(x0, y0, tile_dbu as i32, &boxes);
            (*k, (boxes, grid))
        })
        .collect()
}

/// Whether a point (DBU) lies inside a region or on its boundary.
fn inside_or_on(m: &MergedPoly, o: &Outline, (px, py): (f64, f64)) -> bool {
    crate::merge::point_in_merged(px, py, m)
        || o.segs().iter().any(|&((ax, ay), (bx, by))| {
            point_to_segment_dist(px, py, ax as f64, ay as f64, bx as f64, by as f64) <= 0.5
        })
}

/// Which sides of an enclosed shape have to make the margin.
///
/// All four read the same per-side numbers and differ only in the verdict, which is why
/// they are one check rather than four.  `Adjacent` needs a second threshold and knows
/// which side borders which, so it is the one genuine extension of the family.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sides {
    /// Every side (the worst side ≥ value).  The default.
    All,
    /// At least one side (the best side ≥ value) — a wire endcap.
    Any,
    /// A side below `trigger` forces the sides bordering it to reach `value`.
    Adjacent,
    /// Only the side facing a *line end* of the enclosing layer: the cap across the tip
    /// of a track narrower than `max_width` and at least `min_length` long.
    ///
    /// This is the one mode whose condition comes from the enclosing layer's own shape
    /// rather than from the enclosure margins, because that is what the rule is about: a
    /// narrow line's tip pulls back during processing, so metal that merely reaches the
    /// via on paper may not reach it on silicon. The sidewalls are governed by the
    /// ordinary rule.
    LineEnd,
}

impl Sides {
    /// The `sides` a rule names; `None` if it names one that does not exist, which the
    /// rule has said and refuses to run on.
    pub fn of(rule: &RuleDefinition, name: &str) -> Option<Self> {
        match super::super::params::mode(rule, name, "sides") {
            Ok(None) | Ok(Some("all")) => Some(Sides::All),
            Ok(Some("any")) => Some(Sides::Any),
            Ok(Some("adjacent")) => Some(Sides::Adjacent),
            Ok(Some("line_end")) => Some(Sides::LineEnd),
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] {name}: sides can only be `all`, `any`, `adjacent` or `line_end`, \
                     not `{other}`",
                    rule.id
                );
                None
            }
            Err(super::super::params::NotAWord) => None,
        }
    }
}

/// A distance param in DBU, on the grid.
fn dbu(rule: &RuleDefinition, key: &str, dbu_to_um: f64) -> Option<i64> {
    rule.num(key).map(|um| on_grid(um / dbu_to_um, f64::round))
}

/// A margin squared as `num / den`, and the stretch of the inner wall it was read on.
type Read = ((i128, i128), (f64, f64, f64, f64));

/// A margin as `num / den` squared, ranked as a float: which of two is larger is never
/// a close call on real geometry, and the comparison against the bound is exact.
fn larger(a: (i128, i128), b: (i128, i128)) -> bool {
    a.0 as f64 / a.1 as f64 > b.0 as f64 / b.1 as f64
}

/// The margin a bound is judged on - and among the regions containing a shape, the
/// most favourable of those.  A minimum on every side reads the smallest margin and a
/// maximum on every side the largest; `sides: any` turns each round, at least one side
/// making the bound: the largest for a minimum, the smallest for a maximum.
fn worse(largest: bool, a: (i128, i128), b: (i128, i128)) -> bool {
    if largest { larger(a, b) } else { larger(b, a) }
}

/// Tiled enclosure engine.  Each enclosed region is owned by the tile holding its
/// centroid and tested once against the enclosing regions in that tile (core +
/// halo) — suited to the small features (pins, vias) enclosure targets.
///
/// A maximum reads the same margins and turns the comparison round: the largest margin
/// of a shape, where a minimum reads the smallest.  Two things it leaves out on
/// purpose: a shape no enclosing region contains has no margin to be too large, so it
/// is skipped rather than reported - that absence is the minimum's concern - and the
/// coincidence flags and the wall reality check, which only ever shrink a measured
/// margin and for an upper bound err toward passing.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    sides: Sides,
) -> Vec<Violation> {
    let max = kind == Kind::Max;
    // Which margin of a shape the bound is judged on: see `worse`.
    let largest = max != (sides == Sides::Any);
    let Some(euclidian) = euclidian(rule, kind.name()) else {
        return vec![];
    };
    let enclosing_layer = &rule.layers[0];
    let enclosed_layer = &rule.layers[1];
    let (al, ad) = (
        enclosing_layer.gds_layer as i16,
        enclosing_layer.gds_datatype as i16,
    );
    let (bl, bd) = (
        enclosed_layer.gds_layer as i16,
        enclosed_layer.gds_datatype as i16,
    );

    merged.ensure(layout, al, ad);
    merged.ensure(layout, bl, bd);
    let a_halo = merged.halo_of(al, ad) as i64;
    let b_halo = merged.halo_of(bl, bd) as i64;

    println!(
        "[{}] Checking {} {} {:.2} µm of {} within {}",
        rule.id,
        kind.name(),
        kind.op(),
        rule.value,
        enclosed_layer.name,
        enclosing_layer.name
    );
    let (title, cmp) = (kind.label(), kind.cmp());

    let tile = merged.tile_dbu() as i64;
    // The bound on the grid: a minimum rounds up, a maximum down, and every margin is
    // then an integer - or the root of a ratio of integers - under it or not.
    let limit = match kind {
        Kind::Min => Limit::at_least(rule.value, dbu_to_um),
        Kind::Max => Limit::at_most(rule.value, dbu_to_um),
    };
    let value = rule.value;
    let rid = rule.id.as_str();
    let aname = enclosing_layer.name.as_str();
    let bname = enclosed_layer.name.as_str();
    // KLayout's `enclosed` only checks enclosed shapes that actually overlap an enclosing
    // region (a via far from any MIM is not a MIM via).  Opt-in via the `interacting_only`
    // param so the default "must be inside" behaviour (e.g. Cont within Metal1) is unchanged.
    let interacting_only = rule.num("interacting_only").is_some_and(|v| v != 0.0);
    // Ignore inner edges coincident with the enclosing contour (clip artifacts of an
    // `intersection`-derived enclosed layer) — mirrors KLayout's `consider_intersecting_
    // edges: false` / `without_distance(0)` rule flags.  Off by default: a genuinely flush
    // edge is a real 0-margin violation (e.g. Rppd.b).
    let skip_coincident = !max && rule.num("skip_coincident").is_some_and(|v| v != 0.0);
    // Stronger, region-level variant: skip the *whole* enclosed region if any of its edges
    // is coincident with the enclosing contour — i.e. the region reaches the boundary and
    // is not "surrounded entirely by" the enclosing layer.  NW.e's title says exactly that:
    // a tie crossing the NWell edge is external-tie territory (NW.d), not NW.e's.  This is
    // also halo-robust: the clip is a local property of the region, unlike its remaining
    // margins whose tile ownership can shift with per-suite halos.
    let skip_clipped = !max && rule.num("skip_clipped").is_some_and(|v| v != 0.0);
    // `sides: adjacent` only: the margin below which a side starts asking something of
    // the sides bordering it.
    let trigger = dbu(rule, "trigger", dbu_to_um).unwrap_or(0) as f64;
    // `sides: line_end` only: what counts as a narrow track, and how far it must run
    // before it is a line rather than a notch.
    let max_width = dbu(rule, "max_width", dbu_to_um).unwrap_or(i64::MAX / 4);
    let min_length = dbu(rule, "min_length", dbu_to_um).unwrap_or(0);
    // A minimum reads only the pairs under the value; a maximum has to see them all to
    // find the largest.
    let cutoff = (!max).then_some(limit.dbu());
    let um = |x: f64| x * dbu_to_um;

    let map_a = merged.tiles(al, ad);
    let map_b = merged.tiles(bl, bd);
    let a_boxes = boxes_of(map_a, tile);
    let b_boxes = boxes_of(map_b, tile);
    // Whether an inner wall is the shape's own and not where a tile cut it: just past
    // the wall, on its empty side, the enclosed layer goes on if it is a cut.  Read in
    // the tile that holds the point, whose copy is exact there.  A cut wall lies in
    // the enclosing layer's material and faces its far wall at whatever distance the
    // tile line happened to fall, a margin that is no one's - a 45 µm shape under a
    // maximum read a margin at every tile line it crossed.
    let real_wall = |(p, q): Seg| {
        let (dx, dy) = ((q.0 - p.0) as f64, (q.1 - p.1) as f64);
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return false;
        }
        let (mx, my) = ((p.0 + q.0) as f64 * 0.5, (p.1 + q.1) as f64 * 0.5);
        // Material lies on the left of every segment of an outline; the right is out.
        let probe = (mx + dy / len * 1.5, my - dx / len * 1.5);
        !point_in_layer_at_own_tile(map_b, &b_boxes, tile, probe)
    };
    let empty: Vec<MergedPoly> = Vec::new();
    // The enclosed layer as stitched regions, so a shape spanning tiles - a seal
    // ring's contact ring, a MIM plate - is one shape with one worst margin whatever
    // the tile, and not a report per piece of it that moved with the tile size.
    // Every piece with a core part is scanned in its own tile and the reports of a
    // region are reduced to the worst afterwards.
    // The stitch names the pieces at the tile lines; a copy lying strictly inside its
    // core is a whole shape and a region of its own, numbered on from the stitch's.
    let labeled = crate::merge::stitch_labeled_indexed(map_b, tile as i32);
    let mut keys: Vec<(i32, i32)> = map_b.keys().copied().collect();
    keys.sort_unstable();
    // Each tile numbers its whole shapes from its own base, past every earlier tile's
    // copies, so the tiles are done side by side and no two ids meet.
    let mut base = labeled.regions.len();
    let bases: Vec<usize> = keys
        .iter()
        .map(|k| {
            let b = base;
            base += map_b[k].len();
            b
        })
        .collect();
    let per_tile: Vec<TilePieces> = keys
        .par_iter()
        .zip(bases.par_iter())
        .map(|(&k, &b)| {
            let mut next = b;
            let pieces = labeled.pieces_of(k, &map_b[&k], tile as i32, &mut next);
            (k, pieces)
        })
        .collect();
    // A reading of the shape as a whole - a maximum, the endcap, the bordering sides,
    // and whether it is enclosed at all - needs every wall in one view: the region is
    // put together from its pieces and read once, in the tile that holds its marker.
    // A minimum reads wall by wall, and every piece in its own tile will do.
    let whole = max || sides == Sides::Any || sides == Sides::Adjacent;
    let jobs: Vec<Job> = if whole {
        let mut by_owner: HashMap<(i32, i32), Shapes> = HashMap::new();
        let mut pieces: Vec<Vec<&MergedPoly>> = vec![Vec::new(); labeled.regions.len()];
        for (k, ps) in &per_tile {
            for &(i, rid) in ps {
                let poly = &map_b[k][i];
                if rid < labeled.regions.len() {
                    pieces[rid].push(poly);
                } else {
                    by_owner
                        .entry(*k)
                        .or_default()
                        .push((Cow::Borrowed(poly), rid));
                }
            }
        }
        for (rid, ps) in pieces.into_iter().enumerate() {
            let (mx, my) = labeled.regions[rid].marker;
            let owner = (
                (mx / tile as f64).floor() as i32,
                (my / tile as f64).floor() as i32,
            );
            let shape = if ps.len() == 1 {
                Cow::Borrowed(ps[0])
            } else {
                let owned: Vec<MergedPoly> = ps.into_iter().cloned().collect();
                let unioned = crate::merge::compose_tile(crate::merge::VirtualOp::Union, &[&owned]);
                match unioned
                    .into_iter()
                    .find(|m| crate::merge::point_in_merged(mx, my, m))
                {
                    Some(m) => Cow::Owned(m),
                    None => continue,
                }
            };
            by_owner.entry(owner).or_default().push((shape, rid));
        }
        by_owner.into_iter().map(|(k, v)| (k, v, true)).collect()
    } else {
        per_tile
            .par_iter()
            .map(|(k, ps)| {
                let polys = &map_b[k];
                let v = ps
                    .iter()
                    .map(|&(i, rid)| (Cow::Borrowed(&polys[i]), rid))
                    .collect();
                (*k, v, false)
            })
            .collect()
    };

    let found: Vec<Report> = jobs
        .par_iter()
        .flat_map_iter(|&((tx, ty), ref b_polys, exact)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            // A tile's copy of the enclosed layer is exact within its core and halo and
            // no further: a wall past that may be where the copy ran out, not where the
            // shape ends - a piece of active whose left part lay outside this tile's
            // reach showed a wall a coincident poly edge measured as no margin at all.
            // Every real wall lies in some tile's core, and is measured there.
            let zone = (
                core.x0 - b_halo,
                core.y0 - b_halo,
                core.x1 + b_halo,
                core.y1 + b_halo,
            );
            let in_zone = |(x1, y1, x2, y2): (f64, f64, f64, f64)| {
                if exact {
                    return true;
                }
                let (mx, my) = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                mx >= zone.0 as f64 && mx <= zone.2 as f64 && my >= zone.1 as f64 && my <= zone.3 as f64
            };
            let real_wall = |seg: Seg| exact || real_wall(seg);
            let a_tile: &Vec<MergedPoly> = map_a.get(&(tx, ty)).unwrap_or(&empty);
            let a_conv: Vec<Outline> = a_tile.iter().map(Outline::new).collect();
            // The line-end caps of each enclosing shape of the tile, found once: a
            // track's caps are the same for every via on it, and finding them walks
            // every pair of the track's walls.
            let a_caps: Vec<std::cell::OnceCell<Vec<Seg>>> =
                a_conv.iter().map(|_| std::cell::OnceCell::new()).collect();

            // What this tile found: the region, the wall and the margin the report is
            // about (none for a report without one), the tile, and the report.
            let mut out: Vec<Report> = Vec::new();
            for (bm, region) in b_polys {
                let bm: &MergedPoly = bm;
                let region = *region;
                // Every real wall lies in some tile's core and is read there, so a
                // tile reads the part of the shape in its core and no more: a piece
                // cut to the core has the same walls there, its cut walls are dropped
                // as any tile cut's are, and the enclosing layer it needs is the layer
                // over the core - not over the whole of a shape that runs on for forty
                // tiles, which every tile it ran through was assembling for its own
                // sliver.  Cut at the core and not the zone, so the two tiles' pieces
                // of one wall meet at a vertex and are one wall again in the reduction.
                let (bx0, by0, bx1, by1) = crate::merge::poly_bbox(bm);
                let in_core_whole = bx0 as i64 >= core.x0
                    && by0 as i64 >= core.y0
                    && bx1 as i64 <= core.x1
                    && by1 as i64 <= core.y1;
                // A copy lying within its core has no cut wall: a cut lies on the
                // zone's edge.  Nearly every contact is one, and its walls are read
                // without a probe apiece.
                let cuttable = !exact && !in_core_whole;
                let parts: Vec<Cow<MergedPoly>> = if exact || in_core_whole {
                    vec![Cow::Borrowed(bm)]
                } else {
                    crate::merge::clip_to_box(vec![bm.clone()], core.x0, core.y0, core.x1, core.y1)
                        .into_iter()
                        .map(Cow::Owned)
                        .collect()
                };
            // Where a report on the shape as a whole is placed: the region's marker,
            // the same from whichever tile's piece it comes, or the centroid of a shape
            // whole within its core.
            let (cxd, cyd) = match labeled.regions.get(region) {
                Some(r) => r.marker,
                None => merged_centroid_dbu(bm),
            };
            for bm in &parts {
                let bm: &MergedPoly = bm;
                if bm.outer.len() < 3 {
                    continue;
                }
                let bp = Outline::new(bm);
                let assembled: Vec<MergedPoly>;
                let assembled_o: Vec<Outline>;
                let mut cached_caps = true;
                let a_here: &[Outline] = match outer_over(
                    map_a,
                    tile,
                    a_halo,
                    &core,
                    bm,
                    limit.dbu(),
                    !exact,
                ) {
                    Some(polys) => {
                        assembled = polys;
                        assembled_o = assembled.iter().map(Outline::new).collect();
                        cached_caps = false;
                        &assembled_o
                    }
                    None => &a_conv,
                };
                // The stretch of the inner wall a pair was read on, in µm.
                let edge_um =
                    |(x1, y1, x2, y2): (f64, f64, f64, f64)| (um(x1), um(y1), um(x2), um(y2));

                // Per wall of the enclosed shape, its margin in the best case among the
                // enclosing shapes containing it - a wall is clear if any of them clears
                // it - each read as the worst of that shape's pairs on the wall.  A
                // maximum reads the shape as a whole: its largest margin.
                let mut walls: Walls = Walls::new();
                let mut best: Option<Read> = None;
                let mut any_contained = false;
                let mut clipped = false;
                for a in a_here {
                    if !all_inside(&bp, a) {
                        continue;
                    }
                    any_contained = true;
                    let first = bp.segs()[0];
                    let first_edge = (
                        first.0.0 as f64,
                        first.0.1 as f64,
                        first.1.0 as f64,
                        first.1.1 as f64,
                    );
                    if sides == Sides::Any && !max {
                        let m = endcap_margin(&bp, a) as i128;
                        let worst = ((m * m, 1), first_edge);
                        if best.is_none_or(|(b, _)| worse(largest, b, worst.0)) {
                            best = Some(worst);
                        }
                        continue;
                    }
                    let (pairs, coincident) =
                        margin_pairs(&bp, a, cutoff, skip_coincident, euclidian);
                    clipped |= coincident;
                    // Wall reality check: a pair measured against outer geometry beyond
                    // this bucket's reliable zone can see a fake wall where the union was
                    // truncated; probing just past the wall in the probe's own tile
                    // (complete there) exposes and drops it.
                    let mut here: Walls = Walls::new();
                    let mut worst: Option<Read> = None;
                    for p in pairs {
                        if cuttable && (!in_zone(p.edge) || !real_wall(p.wall)) {
                            continue;
                        }
                        if !max && point_in_layer_at_own_tile(map_a, &a_boxes, tile, p.probe) {
                            continue;
                        }
                        let m = ((p.num, p.den), p.edge);
                        if worst.is_none_or(|(w, _)| worse(largest, m.0, w)) {
                            worst = Some(m);
                        }
                        let e = here.entry(p.wall, m);
                        if worse(largest, m.0, e.0) || (same(m.0, e.0) && longer(m.1, e.1)) {
                            *e = m;
                        }
                    }
                    if max {
                        // A maximum with no facing wall at all has nothing to read.
                        if let Some(m) = worst
                            && best.is_none_or(|(b, _)| worse(largest, b, m.0))
                        {
                            best = Some(m);
                        }
                        continue;
                    }
                    // The best case per wall: the first containing shape sets it, a
                    // later one raises a wall it clears or reads with more room.
                    if walls.is_empty() && best.is_none() {
                        walls = here;
                        best = Some(((i128::MAX / 4, 1), first_edge));
                    } else {
                        for (w, m) in walls.0.iter_mut() {
                            match here.get(w) {
                                Some(h) if worse(largest, m.0, h.0) => *m = *h,
                                Some(_) => {}
                                None => *m = ((i128::MAX / 4, 1), m.1),
                            }
                        }
                    }
                }
                if skip_clipped && clipped {
                    continue; // reaches the enclosing boundary: not "surrounded entirely"
                }

                // `line_end` measures only where the enclosing shape's track ends: find
                // the caps, then the via side facing one.
                if sides == Sides::LineEnd {
                    let caps: Vec<Seg> = a_here
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| all_inside(&bp, a) || regions_interact(&bp, a))
                        .flat_map(|(i, a)| {
                            if cached_caps {
                                a_caps[i]
                                    .get_or_init(|| line_end_segs(a, max_width, min_length))
                                    .clone()
                            } else {
                                line_end_segs(a, max_width, min_length)
                            }
                        })
                        .collect();
                    for &seg in bp.segs() {
                        let ((sx1, sy1), (sx2, sy2)) = seg;
                        if cuttable
                            && (!in_zone((sx1 as f64, sy1 as f64, sx2 as f64, sy2 as f64))
                                || !real_wall(seg))
                        {
                            continue;
                        }
                        let Some((num, den)) = margin_to_caps(seg, &caps) else {
                            continue;
                        };
                        if !limit.broken_by_sq(num, den) {
                            continue;
                        }
                        let margin = um((num as f64 / den as f64).sqrt());
                        let ((ax, ay), (bx, by)) = seg;
                        let (ax, ay, bx, by) =
                            (um(ax as f64), um(ay as f64), um(bx as f64), um(by as f64));
                        out.push((
                            region,
                            Some(seg),
                            Some((num, den)),
                            (tx, ty),
                            Violation::edge(
                                rid,
                                title,
                                format!(
                                    "{bname} at a {aname} line end: enclosed {margin:.4} µm \
                                     {cmp} {value:.2} µm at ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"
                                ),
                                ax,
                                ay,
                                bx,
                                by,
                            ),
                        ));
                    }
                    continue;
                }

                // `adjacent` is decided per side rather than by a reduction: a side under
                // `trigger` is allowed to be short only if the sides bordering it are not.
                if sides == Sides::Adjacent {
                    let containing: Vec<&Outline> = a_here
                        .iter()
                        .filter(|a| all_inside(&bp, a) || regions_interact(&bp, a))
                        .collect();
                    if containing.is_empty() {
                        continue;
                    }
                    let m = side_margins(&bp, &containing);
                    let value_dbu = limit.dbu() as f64;
                    for i in 0..m.len() {
                        if m[i] >= trigger {
                            continue; // this side is not short: it asks nothing of its neighbours
                        }
                        let Some(&worst) = bordering(&bp, i)
                            .iter()
                            .map(|&j| &m[j])
                            .min_by(|a, b| a.total_cmp(b))
                        else {
                            continue;
                        };
                        if worst >= value_dbu {
                            continue;
                        }
                        let ((x1, y1), (x2, y2)) = bp.segs()[i];
                        let (x1, y1, x2, y2) =
                            (um(x1 as f64), um(y1 as f64), um(x2 as f64), um(y2 as f64));
                        out.push((
                            region,
                            None,
                            None,
                            (tx, ty),
                            Violation::edge(
                                rid,
                                title,
                                format!(
                                    "{bname} within {aname}: side enclosed {:.4} µm < {:.2} µm \
                                     and a bordering side only {:.4} µm < {value:.2} µm at \
                                     ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm",
                                    um(m[i]),
                                    um(trigger),
                                    um(worst)
                                ),
                                x1,
                                y1,
                                x2,
                                y2,
                            ),
                        ));
                        break; // one report per shape, not one per short side
                    }
                    continue;
                }

                if !any_contained {
                    if max && !interacting_only {
                        continue; // nothing contains it: no margin to be too large
                    }
                    if interacting_only {
                        // Skip shapes that overlap no enclosing region at all — they
                        // are not subject to this enclosure rule.
                        let touching: Vec<(&MergedPoly, &Outline)> = a_here
                            .iter()
                            .filter(|a| regions_interact(&bp, a))
                            .map(|a| (a.poly(), a))
                            .collect();
                        if touching.is_empty() {
                            continue;
                        }
                        // A shape crossing the enclosing boundary is *not* "surrounded
                        // entirely": under skip_clipped it is out of scope, same as a
                        // clip-coincident one.
                        if skip_clipped {
                            continue;
                        }
                        // Partial overlap: KLayout's `enclosed` still measures facing
                        // pairs whose inner edge lies inside the enclosing region and
                        // ignores pairs from the protruding part (verified empirically
                        // on pSD.c1: an abutted tie crossing the pSD edge is clean,
                        // while its inside lateral margins are still checked).  This is
                        // what an extension rule is - a cover reaching past the target
                        // it crosses by so much - and a maximum reads it the same way.
                        // Per wall for a minimum, the shape as a whole for a maximum.
                        let mut walls: Walls = Walls::new();
                        let mut worst: Option<Read> = None;
                        for (am, a) in touching {
                            let (pairs, _) =
                                margin_pairs(&bp, a, cutoff, skip_coincident, euclidian);
                            for p in pairs {
                                if cuttable && (!in_zone(p.edge) || !real_wall(p.wall)) {
                                    continue;
                                }
                                let (x1, y1, x2, y2) = p.edge;
                                let mid = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                                if !inside_or_on(am, a, mid) {
                                    continue; // pair on the protruding part
                                }
                                if !max
                                    && point_in_layer_at_own_tile(map_a, &a_boxes, tile, p.probe)
                                {
                                    continue;
                                }
                                let m = ((p.num, p.den), p.edge);
                                if worst.is_none_or(|(w, _)| worse(largest, m.0, w)) {
                                    worst = Some(m);
                                }
                                let e = walls.entry(p.wall, m);
                                if worse(largest, m.0, e.0) || (same(m.0, e.0) && longer(m.1, e.1)) {
                                    *e = m;
                                }
                            }
                        }
                        let reports: Vec<(Option<Seg>, Read)> = if max {
                            worst.into_iter().map(|m| (None, m)).collect()
                        } else {
                            walls.0.into_iter().map(|(w, m)| (Some(w), m)).collect()
                        };
                        for (wall, ((num, den), e)) in reports {
                            if !limit.broken_by_sq(num, den) {
                                continue;
                            }
                            let d = um((num as f64 / den as f64).sqrt());
                            let (x1, y1, x2, y2) = edge_um(e);
                            out.push((
                                region,
                                wall,
                                Some((num, den)),
                                (tx, ty),
                                Violation::edge(
                                    rid,
                                    title,
                                    format!(
                                        "enclosure {d:.4} µm {cmp} {value:.2} µm of {bname} within \
                                         {aname} at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                                    ),
                                    x1,
                                    y1,
                                    x2,
                                    y2,
                                ),
                            ));
                        }
                        continue;
                    }
                    let (cx, cy) = (um(cxd), um(cyd));
                    out.push((
                        region,
                        None,
                        None,
                        (tx, ty),
                        Violation::point(
                            rid,
                            title,
                            format!(
                                "shape on {bname} not enclosed by {aname} at ({cx:.4}, {cy:.4}) µm"
                            ),
                            cx,
                            cy,
                        ),
                    ));
                } else if max || sides == Sides::Any {
                    if let Some(((num, den), e)) = best
                        && num < i128::MAX / 4
                        && limit.broken_by_sq(num, den)
                    {
                        let d = um((num as f64 / den as f64).sqrt());
                        let (x1, y1, x2, y2) = edge_um(e);
                        out.push((
                            region,
                            None,
                            Some((num, den)),
                            (tx, ty),
                            Violation::edge(
                                rid,
                                title,
                                format!(
                                    "enclosure {d:.4} µm {cmp} {value:.2} µm of {bname} within \
                                     {aname} at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                                ),
                                x1,
                                y1,
                                x2,
                                y2,
                            ),
                        ));
                    }
                } else {
                    for (wall, ((num, den), e)) in walls.0 {
                        if num >= i128::MAX / 4 || !limit.broken_by_sq(num, den) {
                            continue;
                        }
                        let d = um((num as f64 / den as f64).sqrt());
                        let (x1, y1, x2, y2) = edge_um(e);
                        out.push((
                            region,
                            Some(wall),
                            Some((num, den)),
                            (tx, ty),
                            Violation::edge(
                                rid,
                                title,
                                format!(
                                    "enclosure {d:.4} µm {cmp} {value:.2} µm of {bname} within \
                                     {aname} at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                                ),
                                x1,
                                y1,
                                x2,
                                y2,
                            ),
                        ));
                    }
                }
            }
            }
            out.into_iter()
        })
        .collect();

    // One report per run of violating walls of a region: walls that share a vertex
    // are one run, which joins the two sides of a corner into one report and the two
    // pieces of a wall cut at a tile line back into one wall, so a shape short on one
    // side is one report whatever the tile, and a comb short at three fingers is
    // three.  Each run reports its worst wall.  A report without a wall - a shape not
    // enclosed at all, a maximum, an endcap, a bordering-side reading - is one per
    // region, the worst or else the first in tile order, so the pick does not depend
    // on the order the tiles were scanned in.
    let mut by_region: HashMap<usize, Vec<Report>> = HashMap::new();
    for r in found {
        by_region.entry(r.0).or_default().push(r);
    }
    let mut regions: Vec<usize> = by_region.keys().copied().collect();
    regions.sort_unstable();
    let mut out = Vec::new();
    for region in regions {
        let reports = by_region.remove(&region).expect("keyed");
        let (walled, whole): (Vec<Report>, Vec<Report>) =
            reports.into_iter().partition(|r| r.1.is_some());
        // A shape not enclosed at all is reported as that and nothing else, as it was
        // when it was read whole: the piece of it inside the enclosing shape has its
        // walls' margins, and the piece outside says they are not the point.
        let unenclosed = whole.iter().any(|r| r.2.is_none());
        let mut walled = if unenclosed { Vec::new() } else { walled };
        // A corner on the outer contour is read as a margin of nothing on both walls
        // meeting there; where one of them lies along the contour, that wall is the
        // violation and the point on the other is not one of its own.  Dropped, so
        // it does not tie the walls either side of it into one run - which it did in
        // the tiles where the wall it sat on was whole and not in the ones where it
        // was cut.
        let along: Vec<(i64, i64)> = walled
            .iter()
            .filter(|r| edge_len2(&r.4) > 0.0 && r.2.is_some_and(|m| m.0 == 0))
            .flat_map(|r| {
                let (p, q) = r.1.expect("walled");
                [p, q]
            })
            .collect();
        walled.retain(|r| {
            if edge_len2(&r.4) > 0.0 {
                return true;
            }
            let (p, q) = r.1.expect("walled");
            !(along.contains(&p) || along.contains(&q))
        });
        if let Some(pick) = whole.into_iter().reduce(|a, b| {
            let better = match (b.2, a.2) {
                (Some(x), Some(y)) => worse(largest, x, y),
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => b.3 < a.3,
            };
            if better { b } else { a }
        }) {
            out.push(pick.4);
        }
        if walled.is_empty() {
            continue;
        }
        let mut uf = crate::merge::UnionFind::new(walled.len());
        let mut at: HashMap<(i64, i64), usize> = HashMap::new();
        for (i, r) in walled.iter().enumerate() {
            let (p, q) = r.1.expect("walled");
            for v in [p, q] {
                match at.get(&v) {
                    Some(&j) => uf.union(i, j),
                    None => {
                        at.insert(v, i);
                    }
                }
            }
        }
        let mut pick: HashMap<usize, usize> = HashMap::new();
        for i in 0..walled.len() {
            let root = uf.find(i);
            match pick.get(&root) {
                Some(&j) => {
                    let (mi, mj) = (walled[i].2.expect("margin"), walled[j].2.expect("margin"));
                    // At one margin the longer stretch, then the earlier tile: a
                    // corner touching the outer contour is the wall along it, not
                    // the wall ending there.
                    let (li, lj) = (edge_len2(&walled[i].4), edge_len2(&walled[j].4));
                    if worse(largest, mi, mj)
                        || (same(mi, mj) && (li > lj || (li == lj && walled[i].3 < walled[j].3)))
                    {
                        pick.insert(root, i);
                    }
                }
                None => {
                    pick.insert(root, i);
                }
            }
        }
        let mut chosen: Vec<(usize, usize)> = pick.into_iter().map(|(r, i)| (i, r)).collect();
        chosen.sort_unstable();
        // A wall the tiles cut is read as its pieces, one per tile; the run has them
        // all, and the report is the wall: the pick's stretch extended over the
        // stretches of the run's other walls on its line at its margin.
        let roots: Vec<usize> = (0..walled.len()).map(|i| uf.find(i)).collect();
        let mut walled: Vec<Option<Report>> = walled.into_iter().map(Some).collect();
        for (i, root) in chosen {
            let m = walled[i].as_ref().expect("once").2.expect("margin");
            let wall = walled[i].as_ref().expect("once").1.expect("walled");
            let mut v = walled[i].take().expect("once").4;
            let mates: Vec<&Report> = walled
                .iter()
                .enumerate()
                .filter(|(j, r)| roots[*j] == root && r.is_some())
                .map(|(_, r)| r.as_ref().expect("some"))
                .filter(|r| same(r.2.expect("margin"), m) && collinear(r.1.expect("walled"), wall))
                .collect();
            if !mates.is_empty() {
                extend_over(&mut v, &mates);
            }
            out.push(v);
        }
    }
    out
}

/// Whether two walls lie on one line.
fn collinear((a0, a1): Seg, (b0, b1): Seg) -> bool {
    let cross = |p: (i64, i64), q: (i64, i64), r: (i64, i64)| {
        (q.0 - p.0) as i128 * (r.1 - p.1) as i128 - (q.1 - p.1) as i128 * (r.0 - p.0) as i128
    };
    cross(a0, a1, b0) == 0 && cross(a0, a1, b1) == 0
}

/// The report's stretch extended to the ends of the mates' stretches on its line, in
/// the geometry and in the message, which names the stretch the same way.
fn extend_over(v: &mut Violation, mates: &[&Report]) {
    let crate::violation::ViolationGeometry::Edge { x1, y1, x2, y2 } = v.geometry else {
        return;
    };
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return;
    }
    let along = |x: f64, y: f64| ((x - x1) * dx + (y - y1) * dy) / len2;
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for r in mates {
        if let crate::violation::ViolationGeometry::Edge {
            x1: a,
            y1: b,
            x2: c,
            y2: d,
        } = r.4.geometry
        {
            for (x, y) in [(a, b), (c, d)] {
                let t = along(x, y);
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
    }
    if lo == 0.0 && hi == 1.0 {
        return;
    }
    let (nx1, ny1) = (x1 + dx * lo, y1 + dy * lo);
    let (nx2, ny2) = (x1 + dx * hi, y1 + dy * hi);
    let was = format!("({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4})");
    let now = format!("({nx1:.4}, {ny1:.4})-({nx2:.4}, {ny2:.4})");
    v.message = v.message.replace(&was, &now);
    v.geometry = crate::violation::ViolationGeometry::Edge {
        x1: nx1,
        y1: ny1,
        x2: nx2,
        y2: ny2,
    };
}
