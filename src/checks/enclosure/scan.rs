// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The enclosure scan as the checks drive it: every region of the enclosed layer owned by
//! the tile holding its centroid, tested once against the enclosing regions there, its
//! margin read as facing edge pairs and the verdict taken per [`Sides`].  A minimum and a
//! maximum are one scan with the comparison turned round; what differs beyond that is
//! said where it differs.

use super::Kind;
use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly, merged_centroid_dbu};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

// ===========================================================================
// Enclosure measurement: facing edge pairs (KLayout's `projection` metric).
//
// Only parallel outer edges with a positive projected overlap onto the inner edge
// count, at their perpendicular offset on the inner edge's *outward* side.  An offset
// of ~0 is a **coincident** segment — the inner edge lies on the outer contour, either
// genuinely flush (a real 0-margin violation) or as an artifact of the inner layer
// having been clipped to the outer (an `intersection` virtual).  The geometry cannot
// distinguish the two, and neither does KLayout: its rules pick per-flag
// (`consider_intersecting_edges` / `without_distance(0)`), which `skip_coincident`
// (pair-level) and `skip_clipped` (region-level, "surrounded entirely by") mirror.
// The projection restriction is what keeps clip-*adjacent* perpendicular edges from
// poisoning a shape: they never pair with the boundary they merely touch at an
// endpoint (the old euclidian segment-distance scan read 0 there — the root cause of
// the NW.e/Seal.d false-positive class).
// ===========================================================================

/// Facing-pair scan of `inner` against one containing `outer` candidate (see
/// the module comment above on the projection metric and coincidence semantics).
/// Returns the pairs with `dist < cutoff` plus whether any coincident segment was seen.
/// Which metric an enclosure rule measures its margin in.
///
/// KLayout's own default is euclidian, and most of GF180's enclosure rules ask for it
/// explicitly; `projection` restricts the measurement to facing parallel runs. The two
/// agree on orthogonal geometry and part company at any corner that is not square.
fn enclosure_is_euclidian(rule: &RuleDefinition) -> bool {
    match rule.word("metric") {
        Some("euclidian") => true,
        Some("projection") | None => false,
        Some(other) => {
            eprintln!(
                "[{}] unknown metric '{other}' — expected euclidian or projection; \
                 using projection",
                rule.id
            );
            false
        }
    }
}

/// The enclosing layer over an enclosed shape that reaches past the zone this tile's
/// copy is exact in, assembled from the cores the shape's box grown by the value
/// touches; `None` when the tile's own copy covers it.  A copy is exact out to its halo
/// and no further, and an enclosed shape can be longer than that - a row of abutting
/// cells' Activ merges into one bar - so the tile's copy of the enclosing layer ended
/// short of the bar's far end, and NW.c reported the bar not enclosed at all.
fn outer_over(
    map_a: &crate::merge::TileMap,
    tile: i64,
    halo: i64,
    core: &Core,
    bm: &MergedPoly,
    value_dbu: f64,
) -> Option<Vec<MergedPoly>> {
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in &bm.outer {
        x0 = x0.min(p.x as i64);
        y0 = y0.min(p.y as i64);
        x1 = x1.max(p.x as i64);
        y1 = y1.max(p.y as i64);
    }
    let g = value_dbu.ceil() as i64 + 1;
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

/// Whether a point (µm) lies inside the layer's merged geometry, tested against the
/// bucket of the tile that *contains the point* — where that bucket's union is complete
/// by construction (every polygon covering a point inside `tile + halo` has a bounding
/// box intersecting the bucket).  This is the enclosure engine's "wall reality check":
/// a facing pair measured in a *neighbouring* tile's bucket can see a fake wall where
/// the outer union was truncated at that bucket's halo (a partial-union seam); probing
/// just beyond the wall in the probe's own tile exposes it — if the probe is still
/// inside the layer, the wall does not exist in the true merge and the pair is dropped.
fn point_in_layer_at_own_tile(
    map: &crate::merge::TileMap,
    tile_dbu: i64,
    dbu_to_um: f64,
    p: (f64, f64),
) -> bool {
    let (px, py) = (p.0 / dbu_to_um, p.1 / dbu_to_um);
    let (tx, ty) = (
        (px / tile_dbu as f64).floor() as i32,
        (py / tile_dbu as f64).floor() as i32,
    );
    map.get(&(tx, ty)).is_some_and(|polys| {
        polys
            .iter()
            .any(|m| crate::merge::point_in_merged(px, py, m))
    })
}

/// Which sides of an enclosed shape have to make the margin.
///
/// All three read the same per-side numbers and differ only in the verdict, which is why
/// they are one check rather than three.  `Adjacent` needs a second threshold and knows
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

/// Per-edge enclosure margin of `inner` inside `outers`, in `inner.edges` order.
///
/// Unlike [`enclosure_pairs`] this keeps one number per side and does not clip the edge
/// to the projected overlap, because the `Adjacent` verdict has to know which side
/// borders which — and a clipped segment no longer shares an endpoint with its
/// neighbour. A side with no facing wall at all scores 0: nothing encloses it.
fn side_margins(inner: &Poly, outers: &[&Poly], tol: f64) -> Vec<f64> {
    inner
        .edges
        .iter()
        .map(|&(ax, ay, bx, by)| {
            let (dix, diy) = (bx - ax, by - ay);
            let li = dix.hypot(diy);
            if li <= 0.0 {
                return f64::INFINITY;
            }
            let (ux, uy) = (dix / li, diy / li);
            let (nx, ny) = (uy, -ux); // outward for a CCW contour
            let mut best = f64::INFINITY;
            for o in outers {
                for &(cx, cy, dx, dy) in &o.edges {
                    let (dox, doy) = (dx - cx, dy - cy);
                    let lo = dox.hypot(doy);
                    if lo <= 0.0 {
                        continue;
                    }
                    // The outer edge over the part of it that projects onto this side:
                    // where along the side each end lands, and how far out it is there.
                    // The margin is the projection metric's, the shortest normal distance
                    // over the overlap, which for an edge at any angle is at one end of
                    // it.  Only parallel edges were measured before, and a side whose
                    // facing wall is a 45° chamfer - every via on a power ring corner -
                    // came back as an enclosure of nothing at all, a quarter of a million
                    // times on one design.
                    let t0 = (cx - ax) * ux + (cy - ay) * uy;
                    let t1 = (dx - ax) * ux + (dy - ay) * uy;
                    let n0 = (cx - ax) * nx + (cy - ay) * ny;
                    let n1 = (dx - ax) * nx + (dy - ay) * ny;
                    let (lo_t, hi_t) = (t0.min(t1).max(0.0), t0.max(t1).min(li));
                    if hi_t - lo_t <= tol {
                        continue; // no projected overlap
                    }
                    // Normal distance at the two ends of the overlap, by interpolation.
                    let at = |t: f64| {
                        if (t1 - t0).abs() < 1e-12 {
                            n0.min(n1)
                        } else {
                            n0 + (n1 - n0) * (t - t0) / (t1 - t0)
                        }
                    };
                    let (m0, m1) = (at(lo_t), at(hi_t));
                    if m0.max(m1) < -tol {
                        continue; // wholly behind the side: the far wall, not this one
                    }
                    // An edge that crosses the side's line within the overlap touches it.
                    let m = if m0.min(m1) < -tol {
                        0.0
                    } else {
                        m0.min(m1).max(0.0)
                    };
                    best = best.min(m);
                }
            }
            if best.is_finite() { best } else { 0.0 }
        })
        .collect()
}

/// The line-end edges of `a`: the caps across the tip of any track narrower than
/// `max_width` that runs for at least `min_length`.
///
/// A track shows up as two of the polygon's own edges facing each other closer than
/// `max_width`; the cap is the edge joining them both. Requiring the facing run to reach
/// `min_length` is what keeps a small notch bitten out of a wide plate from reading as a
/// line — it has the two facing edges but not the length.
fn line_end_edges(
    a: &Poly,
    max_width: f64,
    min_length: f64,
    tol: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let n = a.edges.len();
    let mut walls: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let (ax, ay, bx, by) = a.edges[i];
        let (dix, diy) = (bx - ax, by - ay);
        let li = dix.hypot(diy);
        if li < min_length - tol {
            continue;
        }
        let (ux, uy) = (dix / li, diy / li);
        let (nx, ny) = (uy, -ux);
        for j in (i + 1)..n {
            let (cx, cy, dx, dy) = a.edges[j];
            let (dox, doy) = (dx - cx, dy - cy);
            let lo = dox.hypot(doy);
            if lo < min_length - tol || (dix * doy - diy * dox).abs() > 1e-6 * li * lo {
                continue; // too short, or not parallel
            }
            if dix * dox + diy * doy >= 0.0 {
                continue; // same direction: the far side of the shape, not a facing wall
            }
            // Facing across the *inside* of the shape: the other wall lies on the
            // inward side, which for a CCW contour is the negative normal.
            // Strictly narrower than `max_width`, at grid resolution: upstream keeps a
            // cap only while its length is under the width bound, so a track exactly
            // 0.34 µm wide is not a narrow line, and a float length a hair under 0.34
            // must not make it one (13,675 CO.6a markers on 0.340 µm stubs).
            let sep = -((cx - ax) * nx + (cy - ay) * ny);
            if sep <= tol || sep >= max_width - tol {
                continue;
            }
            let t0 = (cx - ax) * ux + (cy - ay) * uy;
            let t1 = (dx - ax) * ux + (dy - ay) * uy;
            if t0.max(t1).min(li) - t0.min(t1).max(0.0) < min_length - tol {
                continue; // the narrow run is not long enough to be a line
            }
            walls.push((i, j));
        }
    }
    if walls.is_empty() {
        return Vec::new();
    }
    // The cap is an edge short enough to span the track that touches both of its walls.
    let touches = |e: usize, w: usize| {
        let (ax, ay, bx, by) = a.edges[e];
        let (cx, cy, dx, dy) = a.edges[w];
        let same =
            |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() <= tol && (p.1 - q.1).abs() <= tol;
        same((ax, ay), (cx, cy))
            || same((ax, ay), (dx, dy))
            || same((bx, by), (cx, cy))
            || same((bx, by), (dx, dy))
    };
    // An edge that is itself a wall of some narrow pair is not a cap, even of a different
    // pair.  A pad narrower than `max_width` in *both* directions has every side facing
    // another, and taking one of them as the cap of the perpendicular pair would read a
    // small square as a line end - which it is not, having no line.  KLayout says the same
    // thing as `.not(first_edges).not(second_edges)`.
    let is_wall: std::collections::HashSet<usize> =
        walls.iter().flat_map(|&(i, j)| [i, j]).collect();
    (0..n)
        .filter(|&e| {
            let (ax, ay, bx, by) = a.edges[e];
            (bx - ax).hypot(by - ay) < max_width - tol
                && !is_wall.contains(&e)
                && walls.iter().any(|&(i, j)| touches(e, i) && touches(e, j))
        })
        .map(|e| a.edges[e])
        .collect()
}

/// Indices of the edges bordering edge `i`, by shared endpoint.
///
/// Index arithmetic would be wrong for a shape with holes, whose edge list is several
/// rings end to end; sharing a vertex is the same test and holds for both.
fn bordering(edges: &[(f64, f64, f64, f64)], i: usize, tol: f64) -> Vec<usize> {
    let (ax, ay, bx, by) = edges[i];
    let same = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() <= tol && (p.1 - q.1).abs() <= tol;
    (0..edges.len())
        .filter(|&j| j != i)
        .filter(|&j| {
            let (cx, cy, dx, dy) = edges[j];
            same((ax, ay), (cx, cy))
                || same((ax, ay), (dx, dy))
                || same((bx, by), (cx, cy))
                || same((bx, by), (dx, dy))
        })
        .collect()
}

/// Enclosure margin of `inner` within `outer` for the **endcap** reduction only: the
/// largest directional margin from the bounding boxes — edge-to-contour distance is
/// corner-limited and would understate a long endcap.
fn enclosure_dist_endcap(inner: &Poly, outer: &Poly) -> (f64, (f64, f64, f64, f64)) {
    let marker = inner.edges.first().copied().unwrap_or_default();
    (outer.bbox.max_side_margin(&inner.bbox), marker)
}

/// Tiled enclosure engine.  Each enclosed region is owned by the tile holding its
/// centroid and tested once against the enclosing regions in that tile (core +
/// halo) — suited to the small features (pins, vias) enclosure targets.
///
/// A maximum reads the same margins and turns the comparison round.  Two things it
/// leaves out on purpose: a shape no enclosing region contains has no margin to be too
/// large, so it is skipped rather than reported - that absence is the minimum's
/// concern - and the coincidence flags and the wall reality check, which only ever
/// shrink a measured margin and for an upper bound err toward passing.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    sides: Sides,
) -> Vec<Violation> {
    let max = kind == Kind::Max;
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
    let tol = 0.5 * dbu_to_um;
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
    let trigger = rule.num("trigger").unwrap_or(0.0);
    // `sides: line_end` only: what counts as a narrow track, and how far it must run
    // before it is a line rather than a notch.
    let max_width = rule.num("max_width").unwrap_or(f64::INFINITY);
    let min_length = rule.num("min_length").unwrap_or(0.0);
    let euclidian = enclosure_is_euclidian(rule);

    let map_a = merged.tiles(al, ad);
    let map_b = merged.tiles(bl, bd);
    let empty: Vec<MergedPoly> = Vec::new();
    let b_keys: Vec<(i32, i32)> = map_b.keys().copied().collect();

    b_keys
        .par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let core = Core {
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
            };
            let b_polys = &map_b[&(tx, ty)];
            let a_conv: Vec<Poly> = map_a
                .get(&(tx, ty))
                .unwrap_or(&empty)
                .iter()
                .filter_map(|m| poly_from_merged(m, dbu_to_um))
                .collect();

            let mut out = Vec::new();
            for bm in b_polys {
                let (cxd, cyd) = merged_centroid_dbu(bm);
                if !core.owns_region(cxd, cyd) {
                    continue;
                }
                let Some(bp) = poly_from_merged(bm, dbu_to_um) else { continue };
                let assembled: Vec<Poly>;
                let a_here: &Vec<Poly> =
                    match outer_over(map_a, tile, a_halo, &core, bm, value / dbu_to_um) {
                        Some(polys) => {
                            assembled = polys
                                .iter()
                                .filter_map(|m| poly_from_merged(m, dbu_to_um))
                                .collect();
                            &assembled
                        }
                        None => &a_conv,
                    };


                // Best-case enclosing shape (greatest margin) among those containing B.
                let mut best_dist = f64::NEG_INFINITY;
                let mut best_edge = None;
                let mut any_contained = false;
                let mut clipped = false;
                for a in a_here {
                    if !all_vertices_inside(&bp, a, tol) {
                        continue;
                    }
                    any_contained = true;
                    let (dist, edge) = if sides == Sides::Any {
                        enclosure_dist_endcap(&bp, a)
                    } else {
                        // A minimum reads only the pairs under the value; a maximum has
                        // to see them all to find the largest.
                        let cutoff = if max { f64::INFINITY } else { value };
                        let (pairs, coincident) =
                            enclosure_pairs(&bp, a, cutoff, skip_coincident, euclidian, tol);
                        clipped |= coincident;
                        // Wall reality check: a pair measured against outer geometry
                        // beyond this bucket's reliable zone can see a fake wall where
                        // the union was truncated; probing just past the wall in the
                        // probe's own tile (complete there) exposes and drops it.
                        let mut worst = f64::INFINITY;
                        let mut worst_edge = bp.edges.first().copied().unwrap_or_default();
                        for p in pairs {
                            if p.dist < worst
                                && (max
                                    || !point_in_layer_at_own_tile(
                                        map_a, tile, dbu_to_um, p.probe,
                                    ))
                            {
                                worst = p.dist;
                                worst_edge = p.edge;
                            }
                        }
                        (worst, worst_edge)
                    };
                    if dist > best_dist {
                        best_dist = dist;
                        best_edge = Some(edge);
                    }
                }
                if skip_clipped && clipped {
                    continue; // reaches the enclosing boundary: not "surrounded entirely"
                }

                // `line_end` measures only where the enclosing shape's track ends: find
                // the caps, then the via side facing one.
                if sides == Sides::LineEnd {
                    let caps: Vec<(f64, f64, f64, f64)> = a_here
                        .iter()
                        .filter(|a| all_vertices_inside(&bp, a, tol) || polys_interact(&bp, a))
                        .flat_map(|a| line_end_edges(a, max_width, min_length, tol))
                        .collect();
                    for &(ax, ay, bx, by) in &bp.edges {
                        let (dix, diy) = (bx - ax, by - ay);
                        let li = dix.hypot(diy);
                        if li <= 0.0 {
                            continue;
                        }
                        let (ux, uy) = (dix / li, diy / li);
                        let (nx, ny) = (uy, -ux);
                        let mut margin = f64::INFINITY;
                        for &(cx, cy, dx, dy) in &caps {
                            let (dox, doy) = (dx - cx, dy - cy);
                            let lo = dox.hypot(doy);
                            if lo <= 0.0 || (dix * doy - diy * dox).abs() > 1e-6 * li * lo {
                                continue;
                            }
                            let t0 = (cx - ax) * ux + (cy - ay) * uy;
                            let t1 = (dx - ax) * ux + (dy - ay) * uy;
                            if t0.max(t1).min(li) - t0.min(t1).max(0.0) <= tol {
                                continue;
                            }
                            let d0 = (cx - ax) * nx + (cy - ay) * ny;
                            if d0 >= -tol {
                                margin = margin.min(d0.max(0.0));
                            }
                        }
                        if margin.is_finite() && margin + tol < value {
                            out.push(Violation::edge(
                                rid,
                                "Minimum enclosure violation",
                                format!(
                                    "{bname} at a {aname} line end: enclosed {margin:.4} µm \
                                     < {value:.2} µm at ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"
                                ),
                                ax, ay, bx, by,
                            ));
                            break;
                        }
                    }
                    continue;
                }

                // `adjacent` is decided per side rather than by a reduction: a side under
                // `trigger` is allowed to be short only if the sides bordering it are not.
                if sides == Sides::Adjacent {
                    let containing: Vec<&Poly> = a_here
                        .iter()
                        .filter(|a| all_vertices_inside(&bp, a, tol) || polys_interact(&bp, a))
                        .collect();
                    if containing.is_empty() {
                        continue;
                    }
                    let m = side_margins(&bp, &containing, tol);
                    for i in 0..m.len() {
                        if m[i] + tol >= trigger {
                            continue; // this side is not short: it asks nothing of its neighbours
                        }
                        let Some(&worst) = bordering(&bp.edges, i, tol)
                            .iter()
                            .map(|&j| &m[j])
                            .min_by(|a, b| a.total_cmp(b))
                        else {
                            continue;
                        };
                        if worst + tol >= value {
                            continue;
                        }
                        let (x1, y1, x2, y2) = bp.edges[i];
                        out.push(Violation::edge(
                            rid,
                            "Minimum enclosure violation",
                            format!(
                                "{bname} within {aname}: side enclosed {:.4} µm < {trigger:.2} µm \
                                 and a bordering side only {worst:.4} µm < {value:.2} µm at \
                                 ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm",
                                m[i]
                            ),
                            x1, y1, x2, y2,
                        ));
                        break; // one report per shape, not one per short side
                    }
                    continue;
                }

                if !any_contained {
                    if max {
                        continue; // nothing contains it: no margin to be too large
                    }
                    if interacting_only {
                        // Skip shapes that overlap no enclosing region at all — they
                        // are not subject to this enclosure rule.
                        let touching: Vec<&Poly> =
                            a_here.iter().filter(|a| polys_interact(&bp, a)).collect();
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
                        // while its inside lateral margins are still checked).
                        let mut worst = f64::INFINITY;
                        let mut worst_edge = None;
                        for a in touching {
                            let (pairs, _) = enclosure_pairs(&bp, a, value, skip_coincident, euclidian, tol);
                            for p in pairs {
                                let (x1, y1, x2, y2) = p.edge;
                                let (mx, my) = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                                if !vertex_inside_or_on(mx, my, a, tol) {
                                    continue; // pair on the protruding part
                                }
                                if p.dist < worst
                                    && !point_in_layer_at_own_tile(map_a, tile, dbu_to_um, p.probe)
                                {
                                    worst = p.dist;
                                    worst_edge = Some(p.edge);
                                }
                            }
                        }
                        if worst + tol < value {
                            let (x1, y1, x2, y2) = worst_edge.unwrap_or_default();
                            out.push(Violation::edge(
                                rid,
                                "Minimum enclosure violation",
                                format!(
                                    "enclosure {worst:.4} µm < {value:.2} µm of {bname} within {aname} \
                                     at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                                ),
                                x1, y1, x2, y2,
                            ));
                        }
                        continue;
                    }
                    let (cx, cy) = (cxd * dbu_to_um, cyd * dbu_to_um);
                    out.push(Violation::point(
                        rid,
                        "Minimum enclosure violation",
                        format!("shape on {bname} not enclosed by {aname} at ({cx:.4}, {cy:.4}) µm"),
                        cx, cy,
                    ));
                } else if if max {
                    best_edge.is_some() && best_dist - tol > value
                } else {
                    best_dist + tol < value
                } {
                    let (x1, y1, x2, y2) = best_edge.unwrap_or_default();
                    out.push(Violation::edge(
                        rid,
                        title,
                        format!(
                            "enclosure {best_dist:.4} µm {cmp} {value:.2} µm of {bname} within \
                             {aname} at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                        ),
                        x1, y1, x2, y2,
                    ));
                }
            }
            out.into_iter()
        })
        .collect()
}
