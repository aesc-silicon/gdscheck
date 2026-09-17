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
use std::collections::HashMap;

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
) -> Option<Vec<MergedPoly>> {
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in &bm.outer {
        x0 = x0.min(p.x as i64);
        y0 = y0.min(p.y as i64);
        x1 = x1.max(p.x as i64);
        y1 = y1.max(p.y as i64);
    }
    let g = value_dbu + 1;
    let (x0, y0, x1, y1) = (x0 - g, y0 - g, x1 + g, y1 + g);
    if x0 >= core.x0 - halo && y0 >= core.y0 - halo && x1 <= core.x1 + halo && y1 <= core.y1 + halo
    {
        return None;
    }
    Some(crate::merge::assemble_over(
        map_a,
        tile as i32,
        (x0, y0, x1, y1),
    ))
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
    let (Some(polys), Some(bs)) = (map.get(&(tx, ty)), boxes.get(&(tx, ty))) else {
        return false;
    };
    // A probe is cast only against the polygons whose box holds it: a contact's probe
    // against every plate of metal in the tile was most of an enclosure rule.
    polys.iter().zip(bs).any(|(m, &(x0, y0, x1, y1))| {
        px >= x0 as f64
            && px <= x1 as f64
            && py >= y0 as f64
            && py <= y1 as f64
            && crate::merge::point_in_merged(px, py, m)
    })
}

/// The bounding box of every polygon of every tile.
type Boxes = HashMap<(i32, i32), Vec<(i64, i64, i64, i64)>>;

fn boxes_of(map: &TileMap) -> Boxes {
    map.par_iter()
        .map(|(k, polys)| {
            (
                *k,
                polys
                    .iter()
                    .map(|m| crate::merge::outer_bbox(&m.outer))
                    .collect(),
            )
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
    let a_boxes = boxes_of(map_a);
    let empty: Vec<MergedPoly> = Vec::new();
    let b_keys: Vec<(i32, i32)> = map_b.keys().copied().collect();

    b_keys
        .par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let b_polys = &map_b[&(tx, ty)];
            let a_tile: &Vec<MergedPoly> = map_a.get(&(tx, ty)).unwrap_or(&empty);
            let a_conv: Vec<Outline> = a_tile.iter().map(Outline::new).collect();

            let mut out = Vec::new();
            for bm in b_polys {
                let (cxd, cyd) = merged_centroid_dbu(bm);
                if !core.owns_region(cxd, cyd) || bm.outer.len() < 3 {
                    continue;
                }
                let bp = Outline::new(bm);
                let assembled: Vec<MergedPoly>;
                let assembled_o: Vec<Outline>;
                let a_here: &[Outline] =
                    match outer_over(map_a, tile, a_halo, &core, bm, limit.dbu()) {
                        Some(polys) => {
                            assembled = polys;
                            assembled_o = assembled.iter().map(Outline::new).collect();
                            &assembled_o
                        }
                        None => &a_conv,
                    };
                // The stretch of the inner wall a pair was read on, in µm.
                let edge_um =
                    |(x1, y1, x2, y2): (f64, f64, f64, f64)| (um(x1), um(y1), um(x2), um(y2));

                // Best-case enclosing shape (greatest margin) among those containing B.
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
                    let worst: Option<Read> = if sides == Sides::Any && !max {
                        let m = endcap_margin(&bp, a) as i128;
                        Some(((m * m, 1), first_edge))
                    } else {
                        let (pairs, coincident) =
                            margin_pairs(&bp, a, cutoff, skip_coincident, euclidian);
                        clipped |= coincident;
                        // Wall reality check: a pair measured against outer geometry
                        // beyond this bucket's reliable zone can see a fake wall where
                        // the union was truncated; probing just past the wall in the
                        // probe's own tile (complete there) exposes and drops it.
                        let mut worst: Option<Read> = None;
                        for p in pairs {
                            if worst.is_some_and(|(w, _)| !worse(largest, (p.num, p.den), w)) {
                                continue;
                            }
                            if !max && point_in_layer_at_own_tile(map_a, &a_boxes, tile, p.probe) {
                                continue;
                            }
                            worst = Some(((p.num, p.den), p.edge));
                        }
                        // A minimum with no pair under the value has every wall clear
                        // of it; a maximum with no facing wall at all has nothing to
                        // read.  Neither is a margin, and the candidate is as good as
                        // it gets for a minimum and says nothing for a maximum.
                        if worst.is_none() && !max {
                            Some(((i128::MAX / 4, 1), first_edge))
                        } else {
                            worst
                        }
                    };
                    if let Some((m, e)) = worst
                        && best.is_none_or(|(b, _)| worse(largest, b, m))
                    {
                        best = Some((m, e));
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
                        .filter(|a| all_inside(&bp, a) || regions_interact(&bp, a))
                        .flat_map(|a| line_end_segs(a, max_width, min_length))
                        .collect();
                    for &seg in bp.segs() {
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
                        out.push(Violation::edge(
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
                        ));
                        break;
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
                        out.push(Violation::edge(
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
                        let mut worst: Option<Read> = None;
                        for (am, a) in touching {
                            let (pairs, _) =
                                margin_pairs(&bp, a, cutoff, skip_coincident, euclidian);
                            for p in pairs {
                                let (x1, y1, x2, y2) = p.edge;
                                let mid = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                                if !inside_or_on(am, a, mid) {
                                    continue; // pair on the protruding part
                                }
                                if worst.is_some_and(|(w, _)| !worse(largest, (p.num, p.den), w)) {
                                    continue;
                                }
                                if !max
                                    && point_in_layer_at_own_tile(map_a, &a_boxes, tile, p.probe)
                                {
                                    continue;
                                }
                                worst = Some(((p.num, p.den), p.edge));
                            }
                        }
                        if let Some(((num, den), e)) = worst
                            && limit.broken_by_sq(num, den)
                        {
                            let d = um((num as f64 / den as f64).sqrt());
                            let (x1, y1, x2, y2) = edge_um(e);
                            out.push(Violation::edge(
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
                            ));
                        }
                        continue;
                    }
                    let (cx, cy) = (um(cxd), um(cyd));
                    out.push(Violation::point(
                        rid,
                        title,
                        format!(
                            "shape on {bname} not enclosed by {aname} at ({cx:.4}, {cy:.4}) µm"
                        ),
                        cx,
                        cy,
                    ));
                } else if let Some(((num, den), e)) = best
                    && num < i128::MAX / 4
                    && limit.broken_by_sq(num, den)
                {
                    let d = um((num as f64 / den as f64).sqrt());
                    let (x1, y1, x2, y2) = edge_um(e);
                    out.push(Violation::edge(
                        rid,
                        title,
                        format!(
                            "enclosure {d:.4} µm {cmp} {value:.2} µm of {bname} within {aname} \
                             at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                        ),
                        x1,
                        y1,
                        x2,
                        y2,
                    ));
                }
            }
            out.into_iter()
        })
        .collect()
}
