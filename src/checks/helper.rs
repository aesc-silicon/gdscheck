// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared helpers used by several checks.
//!
//! For now this is the facing-edge **width scan** behind `min_width`, `max_width`
//! and `exact_width`: it measures the perpendicular span of metal between facing
//! edges of a merged region — a rectilinear scan (vertical edges → horizontal
//! widths, horizontal edges → vertical widths) plus an oblique anti-parallel pass
//! for 45° geometry — and reports **both walls** of any width the rule's predicate
//! rejects.  Each check supplies that predicate (`< min`, `> max`, `≠ exact`).
//! Other common check utilities can move here as they're factored out.

use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{
    Core, MergedCache, MergedPoly, VirtualOp, compose_tile, merged_centroid_dbu,
    representative_point,
};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// The width scan as a rule reports it: [`width_pairs`] measured, then each wall written
/// out as one violation.
#[allow(clippy::too_many_arguments)]
fn scan_widths(
    poly: &MergedPoly,
    core: Core,
    dbu_to_um: f64,
    rule_id: &str,
    label: &str,
    // Message context: the measured layer, the rule limit (µm) and the failing
    // comparison symbol ("<", ">", "≠").
    layer: &str,
    limit_um: f64,
    cmp: &str,
    viol: impl Fn(f64) -> bool,
    oblique_only: bool,
    mixed: bool,
    min_run: f64,
    mask: Option<&[Poly]>,
) -> Vec<Violation> {
    let in_mask = |cx: f64, cy: f64| match mask {
        None => true,
        Some(m) => m
            .iter()
            .any(|p| p.contains_point(cx * dbu_to_um, cy * dbu_to_um)),
    };
    width_pairs(poly, core, viol, in_mask, oblique_only, mixed, min_run)
        .into_iter()
        .map(|(x1, y1, x2, y2, w_dbu)| {
            let w = w_dbu * dbu_to_um;
            Violation::edge(
                rule_id,
                label,
                format!(
                    "{}: width {:.4} µm {} {:.4} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                    layer,
                    w,
                    cmp,
                    limit_um,
                    x1 * dbu_to_um,
                    y1 * dbu_to_um,
                    x2 * dbu_to_um,
                    y2 * dbu_to_um
                ),
                x1 * dbu_to_um,
                y1 * dbu_to_um,
                x2 * dbu_to_um,
                y2 * dbu_to_um,
            )
        })
        .collect()
}

/// Points where a layer's own material narrows to nothing: two pieces of it meeting at
/// an isolated vertex.
///
/// Both are one shape pinched to a point — two squares corner to corner are drawn as a
/// bow-tie and merge into two polygons that touch, so the width there is zero and no pair
/// of facing edges exists to measure it between. The spacing checks deliberately send
/// this case here rather than calling it a gap of zero, so this is where it has to be
/// caught.
///
/// A shared *run* of boundary is not a pinch: two pieces drawn edge to edge are one wide
/// shape, and the width across it is whatever the scan measures.
fn pinch_points(polys: &[MergedPoly]) -> Vec<(f64, f64)> {
    // Every vertex filed once under its coordinates; a pinch is a vertex two polygons
    // share.  Trying every pair of polygons through a set intersection was quadratic in
    // the tile, and a tile of five hundred vias - none of which touch anything - paid
    // a hundred thousand intersections to learn that, on every width rule of the deck.
    let mut at: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, p) in polys.iter().enumerate() {
        for q in std::iter::once(&p.outer).chain(p.holes.iter()).flatten() {
            let owners = at.entry((q.x, q.y)).or_default();
            if owners.last() != Some(&i) {
                owners.push(i);
            }
        }
    }
    // The shared vertices of each pair, so a pair is judged once however many it shares.
    let mut shared_by: HashMap<(usize, usize), Vec<(i32, i32)>> = HashMap::new();
    for (&v, owners) in &at {
        for a in 0..owners.len() {
            for b in a + 1..owners.len() {
                shared_by.entry((owners[a], owners[b])).or_default().push(v);
            }
        }
    }
    let mut out = Vec::new();
    let mut pairs: Vec<_> = shared_by.into_iter().collect();
    pairs.sort_unstable();
    for ((i, j), mut shared) in pairs {
        let (a, b) = (
            poly_from_merged(&polys[i], 1.0),
            poly_from_merged(&polys[j], 1.0),
        );
        if let (Some(a), Some(b)) = (a, b)
            && shares_boundary_run(&a, &b, 0.5)
        {
            continue; // abutting, not pinched
        }
        shared.sort_unstable();
        out.extend(shared.into_iter().map(|(x, y)| (x as f64, y as f64)));
    }
    out
}

/// Drive a width check over the cached tiles: `viol(width_dbu)` decides a
/// violation, `op`/`check_name`/`label` shape the log and the report.
#[allow(clippy::too_many_arguments)]
pub fn run_width(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    check_name: &str,
    op: &str,
    label: &str,
    viol: impl Fn(f64) -> bool + Copy + Sync,
    oblique_only: bool,
    mixed: bool,
    min_run_dbu: f64,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let tile = merged.tile_dbu() as i64;

    for layer in &rule.layers {
        let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, gl, gd);

        println!(
            "[{}] Checking {} {} {:.2} µm on layer {} ({}/{})",
            rule.id, check_name, op, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        let rid = rule.id.as_str();
        // The failing comparison is the inverse of the requirement op.
        let cmp = match op {
            ">=" => "<",
            "<=" => ">",
            _ => "≠",
        };
        let lname = layer.name.as_str();
        let limit = rule.value;
        let mut layer_violations: Vec<Violation> = merged
            .tiles(gl, gd)
            .par_iter()
            .flat_map_iter(|(&(tx, ty), polys)| {
                let core = Core {
                    x0: tx as i64 * tile,
                    y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile,
                    y1: (ty as i64 + 1) * tile,
                };
                let mut pinches: Vec<Violation> = Vec::new();
                if !oblique_only && viol(0.0) {
                    for (px, py) in pinch_points(polys) {
                        if !core.owns(px, py) {
                            continue; // owned by the tile the point falls in
                        }
                        let (x, y) = (px * dbu_to_um, py * dbu_to_um);
                        pinches.push(Violation::point(
                            rid,
                            label,
                            format!(
                                "{lname}: width 0.0000 µm {cmp} {limit:.2} µm at \
                                 ({x:.4}, {y:.4}) µm — the layer pinches to a point"
                            ),
                            x,
                            y,
                        ));
                    }
                }
                let scanned: Vec<Violation> = polys
                    .iter()
                    .flat_map(|poly| {
                        scan_widths(
                            poly,
                            core,
                            dbu_to_um,
                            rid,
                            label,
                            lname,
                            limit,
                            cmp,
                            viol,
                            oblique_only,
                            mixed,
                            min_run_dbu,
                            None,
                        )
                    })
                    .collect();
                pinches.into_iter().chain(scanned)
            })
            .collect();

        violations.append(&mut layer_violations);
    }

    violations
}

/// Gate-length check (Gat.a1–a4): measure the facing-wall width of `layers[0]` (GatPoly)
/// but only where it forms the device gate given by the mask `layers[1]` (e.g.
/// GatPolyOverPsdActivTGO).  Measuring the poly — not the clipped channel — gives the gate
/// *length* (the poly width); the channel *width* W, bounded by Activ edges, never enters.
pub fn run_gate_length(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let poly = &rule.layers[0];
    let mask = &rule.layers[1];
    let (pl, pd) = (poly.gds_layer as i16, poly.gds_datatype as i16);
    let (ml, md) = (mask.gds_layer as i16, mask.gds_datatype as i16);
    merged.ensure(layout, pl, pd);
    merged.ensure(layout, ml, md);

    println!(
        "[{}] Checking gate_length >= {:.2} µm of {} over {}",
        rule.id, rule.value, poly.name, mask.name
    );

    let min_w_dbu = rule.value / dbu_to_um;
    let viol = move |w: f64| w < min_w_dbu - 0.5;
    let tile = merged.tile_dbu() as i64;
    let pmap = merged.tiles(pl, pd);
    let mmap = merged.tiles(ml, md);
    let rid = rule.id.as_str();
    let pname = poly.name.as_str();
    let limit = rule.value;
    let empty: Vec<MergedPoly> = Vec::new();

    pmap.par_iter()
        .flat_map_iter(move |(&(tx, ty), polys)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let mps: Vec<Poly> = mmap
                .get(&(tx, ty))
                .unwrap_or(&empty)
                .iter()
                .filter_map(|m| poly_from_merged(m, dbu_to_um))
                .collect();
            let mut out = Vec::new();
            if mps.is_empty() {
                return out.into_iter();
            }
            for p in polys {
                out.extend(scan_widths(
                    p,
                    core,
                    dbu_to_um,
                    rid,
                    "Minimum gate-length violation",
                    pname,
                    limit,
                    "<",
                    viol,
                    false,
                    // Gate length measures the poly's facing-wall width under a mask; a
                    // mixed pair has no single width to attribute to a mask region.
                    false,
                    0.5,
                    Some(&mps),
                ));
            }
            out.into_iter()
        })
        .collect()
}

// ===========================================================================
// Region-to-region spacing engine.
//
// Spacing is measured between **merged** regions on the cached tiles; a pair within
// `value` is reported only if a caller-supplied `gate(a, b)` holds.  `min_space`
// passes `|_, _| true`; conditional rules (e.g. `min_space_prl` / TM2.bR) supply a
// width / parallel-run predicate.  Each violation is kept only if its gap midpoint
// lies in the tile core, so a pair seen from several overlapping tiles is reported
// once.
// ===========================================================================

/// Which pairs a spacing rule is about, in which direction, and in which metric.
#[derive(Clone, Copy)]
pub struct SpaceMode {
    /// Scan pairs that share area for their narrowest empty gap, rather than pairs that
    /// share none for their closest approach.  See [`facing_gaps`].
    overlapping: bool,
    /// Measure L-infinity rather than euclidian.  See [`seg_seg_closest_square`].
    square: bool,
    /// Measure how deeply the pair penetrates rather than how far apart it is - the
    /// same facing-edge scan run inward.  Implies `overlapping`.
    inward: bool,
}

#[allow(clippy::too_many_arguments)]
fn check_tile<G: Fn(&Poly, &Poly, Marker, Marker) -> bool>(
    a_polys: &[MergedPoly],
    b_polys: &[MergedPoly],
    same_layer: bool,
    core: Core,
    value: f64,
    dbu_to_um: f64,
    rule_id: &str,
    name_a: &str,
    name_b: &str,
    mode: SpaceMode,
    gate: &G,
) -> Vec<Violation> {
    let half = dbu_to_um * 0.5;
    // Each polygon keeps its source region's DBU marker: `poly_from_merged` can drop a
    // polygon, so zipping here is what keeps `Poly` and `MergedPoly` aligned for the gate.
    //
    // The marker must be a point *on* the shape, not its centroid: a net-aware gate
    // resolves the net by looking the marker up, and a ring's centroid sits in its hole -
    // where, in GF180's DN.2b fixture, an unrelated island sits. Both sides then read as
    // one net and the rule goes quiet on a real violation.
    let conv =
        |m: &MergedPoly| poly_from_merged(m, dbu_to_um).map(|p| (p, representative_point(m)));
    // Whether a merged piece is real material or a shaving the merge left behind.  A 45°
    // wall is drawn with a one-nanometre chamfer at each corner, and rounding that corner
    // detaches the chamfer as a triangle of half a square nanometre touching the body at
    // a vertex.  The width checks want it - KLayout keeps the same feature and reports
    // the notch - but a *gap* of nothing to a shaving is not a spacing violation, and
    // reading it as one is five false positives each on LRES.2 and PRES.2.  Nothing drawn
    // is this small: a hundred square DBU is a ten-nanometre square.
    let material = |m: &MergedPoly| crate::merge::merged_area_dbu(m) >= SHAVING_DBU2;
    let prep = |ms: &[MergedPoly]| -> (Vec<(Poly, Marker)>, Vec<bool>) {
        ms.iter()
            .filter_map(|m| conv(m).map(|p| (p, material(m))))
            .unzip()
    };
    let (pa, mat_a) = prep(a_polys);
    let (pb, mat_b) = if same_layer {
        (Vec::new(), Vec::new())
    } else {
        prep(b_polys)
    };
    let bs: &[(Poly, Marker)] = if same_layer { &pa } else { &pb };
    let mat_bs: &[bool] = if same_layer { &mat_a } else { &mat_b };

    let mut out = Vec::new();
    for (i, (a, ma)) in pa.iter().enumerate() {
        for (j, (b, mb)) in bs.iter().enumerate() {
            if same_layer && j <= i {
                continue;
            }
            if !a.bbox.possibly_within(&b.bbox, value) {
                continue;
            }
            let overlaps = overlapping(a, b);
            if overlaps != mode.overlapping {
                continue; // this rule is about the other kind of pair
            }
            let (min_dist, (ax, ay), (bx, by)) = if mode.overlapping {
                // The pair shares area, so its closest approach is zero and meaningless.
                // Take the narrowest facing gap that is genuinely empty instead.
                let mut best: Option<Gap> = None;
                for (gap, p, q) in facing_gaps(a, b, value, half, mode.inward) {
                    let (px, py) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
                    let wrong = if mode.inward {
                        // An overlap has to be material of both, or the "facing" pair
                        // reaches across a notch in one of them.
                        !(a.contains_point(px, py) && b.contains_point(px, py))
                    } else {
                        let (mx, my) = (px / dbu_to_um, py / dbu_to_um);
                        a_polys
                            .iter()
                            .chain(b_polys)
                            .any(|m| crate::merge::point_in_merged(mx, my, m))
                    };
                    if wrong {
                        continue; // another arm of one of the shapes lies in the gap
                    }
                    if best.is_none_or(|(d, _, _)| gap < d) {
                        best = Some((gap, p, q));
                    }
                }
                match best {
                    Some(v) => v,
                    None => continue,
                }
            } else {
                let m = closest(a, b, half, mode.square);
                // A contact between two layers is a separation of *zero*, and reported:
                // two shapes meeting at a corner have a gap that happens to be nothing
                // wide, which is the worst spacing there is. KLayout reads it that way and
                // this engine used to drop it, silently, wherever it occurred.
                //
                // Within one layer it is a gap of zero too, and KLayout reports it: two
                // wells meeting corner to corner merge into one self-touching shape, and
                // `space` marks the touch point. This engine excluded the same-layer case
                // for a while, on the grounds that such a contact is a pinch and belongs
                // to the width checks; that reading cost 82 logical violations across
                // eighteen gf180mcu decks - the whole of what `nwell` and `lvpwell` were
                // missing on their well-spacing rules - and the pinch is reported anyway,
                // by whichever width rule covers the layer.
                //
                // A contact along a *run* is not a gap: two shapes drawn edge to edge
                // abut, with no space between them anywhere. IHP's butted substrate ties
                // are exactly that by construction. Only a contact at isolated points is
                // a separation of zero, whichever layers it is between.
                let shaving = !mat_a[i] || !mat_bs[j];
                if m.0 < half && (shaving || shares_boundary_run(a, b, half)) {
                    continue;
                }
                m
            };
            if min_dist < value - half {
                if !gate(a, b, *ma, *mb) {
                    continue;
                }
                // Own the violation by the gap midpoint; mark the gap itself.
                let mx = (ax + bx) * 0.5;
                let my = (ay + by) * 0.5;
                if !core.owns(mx / dbu_to_um, my / dbu_to_um) {
                    continue;
                }
                let (title, what) = if mode.inward {
                    ("Minimum overlap violation", "overlap")
                } else {
                    ("Minimum space violation", "space")
                };
                out.push(Violation::edge(
                    rule_id,
                    title,
                    format!(
                        "{what} {:.4} µm < {:.2} µm between {} and {} at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                        min_dist, value, name_a, name_b, ax, ay, bx, by
                    ),
                    ax, ay, bx, by,
                ));
            }
        }
    }
    out
}

/// Tiled region-pair spacing over the cached merge.  A pair within `value` is
/// reported only if `gate(a, b, marker_a, marker_b)` holds, letting conditional spacing
/// rules add width / parallel-run / same-net conditions without duplicating the merge,
/// tiling and edge-distance work.
/// Depth of mutual penetration where two layers overlap - KLayout's `overlap` check.
/// The facing-edge scan of [`run_gated`] run inward: see [`facing_gaps`].
pub fn run_overlap(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let mode = SpaceMode {
        overlapping: true,
        square: false,
        inward: true,
    };
    run_gated_with(rule, layout, dbu_to_um, merged, Some(mode), |_, _, _, _| {
        true
    })
}

pub fn run_gated<G: Fn(&Poly, &Poly, Marker, Marker) -> bool + Sync>(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    gate: G,
) -> Vec<Violation> {
    run_gated_with(rule, layout, dbu_to_um, merged, None, gate)
}

/// `forced` overrides what `str_params` would say, for a check that *is* a mode.
fn run_gated_with<G: Fn(&Poly, &Poly, Marker, Marker) -> bool + Sync>(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    forced: Option<SpaceMode>,
    gate: G,
) -> Vec<Violation> {
    let layer_a = &rule.layers[0];
    let layer_b = rule.layers.get(1).unwrap_or(layer_a);
    let (al, ad) = (layer_a.gds_layer as i16, layer_a.gds_datatype as i16);
    let (bl, bd) = (layer_b.gds_layer as i16, layer_b.gds_datatype as i16);
    let same_layer = al == bl && ad == bd;

    println!(
        "[{}] Checking {} >= {:.2} µm between layer {} and {}",
        rule.id, rule.check, rule.value, layer_a.name, layer_b.name
    );

    merged.ensure(layout, al, ad);
    if !same_layer {
        merged.ensure(layout, bl, bd);
    }

    let value = rule.value;
    // Which pairs the rule is about.  `disjoint` (the default) is the ordinary reading:
    // two shapes that share no area, measured at their closest approach.  `overlapping`
    // is for a rule whose two shapes overlap *by definition* - GF180's S.PL.5b_MV asks
    // the space from a poly to the COMP it gates - where the closest approach is zero and
    // the gap meant is between facing edges elsewhere along the same two shapes.
    let overlapping_pairs = match rule.str_params.get("pairs").map(String::as_str) {
        Some("overlapping") => true,
        Some("disjoint") | None => false,
        Some(other) => {
            eprintln!(
                "[{}] unknown pairs '{other}' — expected disjoint or overlapping; \
                 using disjoint",
                rule.id
            );
            false
        }
    };
    // `square` is KLayout's L-infinity metric, which a rule words as "must not fall
    // within a d x d square at the corner".  It separates *less* than euclidian, so the
    // default stays euclidian and only a rule that asks for it pays the wider net.
    let square = match rule.str_params.get("metric").map(String::as_str) {
        Some("square") => true,
        Some("euclidian") | None => false,
        Some(other) => {
            eprintln!(
                "[{}] unknown metric '{other}' — expected euclidian or square; \
                 using euclidian",
                rule.id
            );
            false
        }
    };
    let mode = forced.unwrap_or(SpaceMode {
        overlapping: overlapping_pairs,
        square,
        inward: false,
    });
    let tile = merged.tile_dbu() as i64;
    let rid = rule.id.as_str();
    let name_a = layer_a.name.as_str();
    let name_b = layer_b.name.as_str();
    let map_a = merged.tiles(al, ad);
    let map_b = if same_layer {
        map_a
    } else {
        merged.tiles(bl, bd)
    };

    // Every spacing violation has an `a`-region within the halo of the tile that
    // owns its gap, so that tile is an `a` key — iterating `a`'s tiles covers all
    // pairs, and the core filter deduplicates.
    let keys: Vec<(i32, i32)> = map_a.keys().copied().collect();
    let empty: Vec<MergedPoly> = Vec::new();

    keys.par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let a_polys = &map_a[&(tx, ty)];
            let b_polys = map_b.get(&(tx, ty)).unwrap_or(&empty);
            check_tile(
                a_polys, b_polys, same_layer, core, value, dbu_to_um, rid, name_a, name_b, mode,
                &gate,
            )
            .into_iter()
        })
        .collect()
}

// ===========================================================================
// Per-tile boolean "residual" engine.
//
// Applies one boolean op to the rule's layers on each cached tile and reports every
// resulting region whose centroid lies in the tile core.  Containment-style rules use
// this with a single primitive:
//   * `Difference`   → `target − (other covers)` — the part of `layers[0]` not covered
//     by the union of the rest ("must be inside", e.g. Cnt.g / Cnt.h).
//   * `Intersection` → the overlap of all layers ("X over Y not allowed", e.g. Cnt.j).
// The two thin checks (`coverage`, `forbidden_overlap`) differ only in the op they pass.
// ===========================================================================

/// Drive a boolean-residual check: `op` over `rule.layers` per tile, one point
/// violation per residual region (owned by the tile whose core holds its centroid).
/// `descr` is the human-readable body of each violation message.
pub fn run_boolean_residual(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    op: VirtualOp,
    label: &str,
    descr: &str,
) -> Vec<Violation> {
    let keys_l: Vec<(i16, i16)> = rule
        .layers
        .iter()
        .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        .collect();

    println!("[{}] Checking {}: {}", rule.id, rule.check, descr);

    for &(l, d) in &keys_l {
        merged.ensure(layout, l, d);
    }
    let maps: Vec<&crate::merge::TileMap> =
        keys_l.iter().map(|&(l, d)| merged.tiles(l, d)).collect();

    // Tiles that can yield output: bounded by the base for Difference, by the shared
    // keys for Intersection.
    let tile_keys: Vec<(i32, i32)> = match op {
        VirtualOp::Difference => maps[0].keys().copied().collect(),
        VirtualOp::Intersection => {
            let mut acc: HashSet<(i32, i32)> = maps[0].keys().copied().collect();
            for m in &maps[1..] {
                acc.retain(|k| m.contains_key(k));
            }
            acc.into_iter().collect()
        }
        _ => maps
            .iter()
            .flat_map(|m| m.keys().copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
    };

    let tile = merged.tile_dbu() as i64;
    let rid = rule.id.as_str();

    tile_keys
        .par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let sources: Vec<&[MergedPoly]> = maps
                .iter()
                .map(|m| m.get(&(tx, ty)).map(Vec::as_slice).unwrap_or(&[]))
                .collect();
            compose_tile(op, &sources)
                .into_iter()
                .filter_map(move |m| {
                    let (cx, cy) = merged_centroid_dbu(&m);
                    if !core.owns_region(cx, cy) {
                        return None;
                    }
                    let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
                    Some(Violation::point(
                        rid,
                        label,
                        format!("{descr} at ({ux:.4}, {uy:.4}) µm"),
                        ux,
                        uy,
                    ))
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect()
}

// ===========================================================================
// Directional extension engine.
//
// For each `layers[0]` (cover) region that overlaps a `layers[1]` (target) region, the
// cover must extend at least `value` beyond the target's two **long** edges — i.e. in
// the target's *width* (short-axis) direction.  The target's ends (short edges) are
// exempt, since it legitimately runs out past the cover there.  Exact for the
// axis-aligned rectangles this targets (a SalBlock placed across a long Activ/GatPoly).
// ===========================================================================

/// Drive a directional extension check (e.g. Sal.c: SalBlock over Activ/GatPoly).
///
/// Edge-based and local: every `layers[1]` (target) contour edge segment that the
/// `layers[0]` (cover) sits over must have the cover extending at least `value`
/// perpendicular beyond it.  A target edge counts as "covered" only where the cover
/// overlaps the target just *inside* that edge — so the target's free edges (a
/// resistor's ends, or the boundary of a big active the cover merely sits inside) are
/// exempt automatically, and only the long edges the cover actually crosses are checked.
pub fn run_extension(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let cover = &rule.layers[0];
    let target = rule.layers.get(1).unwrap_or(cover);
    let (cl, cd) = (cover.gds_layer as i16, cover.gds_datatype as i16);
    let (tl, td) = (target.gds_layer as i16, target.gds_datatype as i16);
    merged.ensure(layout, cl, cd);
    merged.ensure(layout, tl, td);

    println!(
        "[{}] Checking min_extension >= {:.2} µm of {} over {}",
        rule.id, rule.value, cover.name, target.name
    );

    let value = rule.value; // µm
    let eps = 0.5 * dbu_to_um; // probe half a grid inside the target edge
    let step = (value * 0.5).max(dbu_to_um); // sampling step along an edge (≤ value/2)
    let tile = merged.tile_dbu() as i64;
    let cmap = merged.tiles(cl, cd);
    let tmap = merged.tiles(tl, td);
    let rid = rule.id.as_str();
    let (cn, tn) = (cover.name.as_str(), target.name.as_str());
    let empty: Vec<MergedPoly> = Vec::new();

    tmap.par_iter()
        .flat_map_iter(move |(&(tx, ty), tps)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let sps: Vec<Poly> = cmap
                .get(&(tx, ty))
                .unwrap_or(&empty)
                .iter()
                .filter_map(|m| poly_from_merged(m, dbu_to_um))
                .collect();
            let mut out = Vec::new();
            if sps.is_empty() {
                return out.into_iter();
            }
            let covered = |x: f64, y: f64| sps.iter().any(|s| s.contains_point(x, y));
            for tm in tps {
                let Some(a) = poly_from_merged(tm, dbu_to_um) else {
                    continue;
                };
                if !sps.iter().any(|s| a.bbox.possibly_within(&s.bbox, value)) {
                    continue;
                }
                for &(ax, ay, bx, by) in &a.edges {
                    let (dx, dy) = (bx - ax, by - ay);
                    let len = dx.hypot(dy);
                    if len == 0.0 {
                        continue;
                    }
                    let (ux, uy) = (dx / len, dy / len);
                    let (inx, iny) = (-uy, ux); // inward (outer contour is CCW)
                    let (onx, ony) = (uy, -ux); // outward
                    // How far the cover actually reaches outward past this edge point
                    // (capped at `value`), for reporting the measured extension.
                    let measure = |px: f64, py: f64| {
                        let mut d = eps;
                        let mut reached = 0.0;
                        while d <= value + eps {
                            if covered(px + onx * d, py + ony * d) {
                                reached = d;
                                d += dbu_to_um;
                            } else {
                                break;
                            }
                        }
                        reached.min(value)
                    };
                    // Build a violation edge along the under-extended span of this edge.
                    let make = |sx: f64, sy: f64, ex: f64, ey: f64, worst: f64| {
                        let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
                        if !core.owns(mx / dbu_to_um, my / dbu_to_um) {
                            return None;
                        }
                        // Expand a single-sample span into a short edge along the boundary.
                        let (mut x1, mut y1, mut x2, mut y2) = (sx, sy, ex, ey);
                        if (x1 - x2).abs() < 1e-9 && (y1 - y2).abs() < 1e-9 {
                            x1 -= ux * step * 0.5;
                            y1 -= uy * step * 0.5;
                            x2 += ux * step * 0.5;
                            y2 += uy * step * 0.5;
                        }
                        Some(Violation::edge(
                            rid,
                            "Minimum extension violation",
                            format!(
                                "{cn} extends only {worst:.3} µm over {tn} \
                                 (needs {value:.2} µm) at \
                                 ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                            ),
                            x1,
                            y1,
                            x2,
                            y2,
                        ))
                    };
                    let n = (len / step).ceil().max(1.0) as usize;
                    let mut span_start: Option<(f64, f64)> = None;
                    let mut span_end = (0.0, 0.0);
                    let mut worst = value;
                    for k in 0..=n {
                        let t = (len * k as f64 / n as f64).min(len);
                        let (px, py) = (ax + t * ux, ay + t * uy);
                        // Only edges the cover sits over (cover present just inside the
                        // edge) are subject to the extension; probe outward just short of
                        // `value` so an exactly-`value` extension counts as covered.
                        let failing = covered(px + inx * eps, py + iny * eps)
                            && !covered(px + onx * (value - eps), py + ony * (value - eps));
                        if failing {
                            if span_start.is_none() {
                                span_start = Some((px, py));
                                worst = value;
                            }
                            span_end = (px, py);
                            worst = worst.min(measure(px, py));
                        } else if let Some((sx, sy)) = span_start.take() {
                            out.extend(make(sx, sy, span_end.0, span_end.1, worst));
                        }
                    }
                    if let Some((sx, sy)) = span_start.take() {
                        out.extend(make(sx, sy, span_end.0, span_end.1, worst));
                    }
                }
            }
            out.into_iter()
        })
        .collect()
}

// ===========================================================================
// Enclosed-area engine.
//
// Reports holes (enclosed empty regions fully surrounded by the layer) whose area is
// below `value`.  Holes live in each merged region's `holes`; for the small enclosed
// regions this targets, the surrounding ring is local to a tile, so per-tile holes are
// reliable — each is owned by the tile whose core holds its centroid.
// ===========================================================================

/// Drive a minimum-enclosed-area check: report every hole smaller than `value` (µm²).
pub fn run_enclosed_area(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let value = rule.value;
    let d2 = dbu_to_um * dbu_to_um;
    let mut violations = Vec::new();

    for layer in &rule.layers {
        let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, gl, gd);
        println!(
            "[{}] Checking min_enclosed_area >= {:.4} µm² on layer {} ({}/{})",
            rule.id, value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        let tile = merged.tile_dbu() as i64;
        let rid = rule.id.as_str();
        let ln = layer.name.as_str();
        let mut v: Vec<Violation> = merged
            .tiles(gl, gd)
            .par_iter()
            .flat_map_iter(move |(&(tx, ty), polys)| {
                let core = Core {
                    x0: tx as i64 * tile, y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
                };
                let mut out = Vec::new();
                for m in polys {
                    for hole in &m.holes {
                        let (area_dbu, cx, cy) = ring_area_centroid(hole);
                        let area = area_dbu * d2;
                        if area >= value || !core.owns_region(cx, cy) {
                            continue;
                        }
                        let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
                        out.push(Violation::point(
                            rid,
                            "Minimum enclosed area violation",
                            format!(
                                "enclosed area {:.4} µm² < {:.4} µm² on layer {} at ({:.4}, {:.4}) µm",
                                area, value, ln, ux, uy
                            ),
                            ux, uy,
                        ));
                    }
                }
                out.into_iter()
            })
            .collect();
        violations.append(&mut v);
    }
    violations
}

/// Flag non-orthogonal edges of `layers[0]` (e.g. Gat.f: no 45° GatPoly over Activ — run
/// over the GatPoly∩Activ intersection so only the part crossing the channel is checked).
/// By default every edge that is not axis-aligned is forbidden; an optional `angle` param
/// (degrees) restricts the check to edges at that specific orientation (and its 180°
/// complement), with an optional `tolerance` (degrees, default 1.0).
pub fn run_no_angle(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let layer = &rule.layers[0];
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);

    let forbidden = rule.params.get("angle").copied(); // specific forbidden orientation
    let tol = rule.params.get("tolerance").copied().unwrap_or(1.0);
    // Allowed orientations are multiples of `step` degrees; 90 (the default) permits
    // only axis-aligned edges, 45 also permits the diagonals.  GF180's ACUTE rules want
    // the latter - they allow 0, 45, 90 and -45 and flag everything else.
    let step = rule.params.get("step").copied().unwrap_or(90.0);

    match forbidden {
        Some(a) => println!(
            "[{}] Checking no_angle: {} edges at {:.1}°",
            rule.id, layer.name, a
        ),
        None => println!(
            "[{}] Checking no_angle: non-orthogonal {} edges",
            rule.id, layer.name
        ),
    }

    let tile = merged.tile_dbu() as i64;
    let gmap = merged.tiles(gl, gd);
    let rid = rule.id.as_str();
    let ln = layer.name.as_str();
    // Orientation of a forbidden angle, folded into [0,180).
    let target = forbidden.map(|a| a.rem_euclid(180.0));

    gmap.par_iter()
        .flat_map_iter(move |(&(tx, ty), ps)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let mut out = Vec::new();
            for pm in ps {
                let Some(p) = poly_from_merged(pm, dbu_to_um) else {
                    continue;
                };
                for &(ax, ay, bx, by) in &p.edges {
                    let (dx, dy) = (bx - ax, by - ay);
                    if dx == 0.0 && dy == 0.0 {
                        continue;
                    }
                    let ang = dy.atan2(dx).to_degrees().rem_euclid(180.0);
                    let near =
                        |a: f64, b: f64| (a - b).abs() <= tol || (a - b).abs() >= 180.0 - tol;
                    let flag = match target {
                        Some(t) => near(ang, t),
                        None => {
                            // Not on the allowed lattice: distance to the nearest
                            // multiple of `step` exceeds the tolerance.
                            let k = (ang / step).round() * step;
                            !near(ang, k)
                        }
                    };
                    if !flag {
                        continue;
                    }
                    let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                    if !core.owns(mx / dbu_to_um, my / dbu_to_um) {
                        continue;
                    }
                    out.push(Violation::edge(
                        rid,
                        "Forbidden angle violation",
                        format!(
                            "{ln}: forbidden ({ang:.1}°) edge at \
                             ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"
                        ),
                        ax,
                        ay,
                        bx,
                        by,
                    ));
                }
            }
            out.into_iter()
        })
        .collect()
}

// ===========================================================================
// Bounding-box extent engine.
//
// Per merged region, takes one of its two bounding-box sides — the short side
// (`long = false`, the feature *width*) or the long side (`long = true`, the
// *length*) — and reports the region when `viol(extent_dbu)` holds.  Exact for the
// axis-aligned rectangles that make up contact bars; used by the `min_dim`/`max_dim`
// (width) and `min_length`/`max_length` checks.  Unlike the facing-wall width scan,
// this never confuses a bar's length for its width.
// ===========================================================================

/// Drive a bounding-box extent check over the layer's stitched regions: one point
/// violation per offending region, at the region's marker.  The extent is the union of
/// the region's pieces' bounding boxes, each piece cut to its tile core, so a region of
/// any size is measured whole without any tile holding a whole copy of it - which is
/// what the check used to need, a halo the size of its value on the drawn layers under
/// the region: MDP.13a's 50 µm on a dense COMP was 21 copies of every shape.
#[allow(clippy::too_many_arguments)]
pub fn run_extent(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    check_name: &str,
    op: &str,
    label: &str,
    long: bool,
    viol: impl Fn(f64) -> bool + Copy + Send + Sync,
) -> Vec<Violation> {
    let layer = &rule.layers[0];
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);

    println!(
        "[{}] Checking {} {} {:.2} µm on layer {}",
        rule.id, check_name, op, rule.value, layer.name
    );

    let tile = merged.tile_dbu() as i64;
    let rid = rule.id.as_str();
    let lname = layer.name.as_str();
    let limit = rule.value;
    let word = if long { "length" } else { "width" };
    let cmp = match op {
        ">=" => "<",
        "<=" => ">",
        _ => "≠",
    };

    let labeled = crate::merge::stitch_labeled(merged.tiles(gl, gd), merged.tile_dbu());
    let per_tile: Vec<Vec<(usize, (i32, i32, i32, i32))>> = labeled
        .by_tile
        .par_iter()
        .map(|(&(tx, ty), polys)| {
            let core = (
                tx as i64 * tile,
                ty as i64 * tile,
                (tx as i64 + 1) * tile,
                (ty as i64 + 1) * tile,
            );
            polys
                .iter()
                .filter_map(|(m, r)| crate::merge::core_clipped_bbox(m, core).map(|b| (*r, b)))
                .collect()
        })
        .collect();
    let mut bbox: Vec<Option<(i32, i32, i32, i32)>> = vec![None; labeled.regions.len()];
    for v in per_tile {
        for (r, b) in v {
            bbox[r] = Some(match bbox[r] {
                None => b,
                Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
            });
        }
    }
    bbox.iter()
        .enumerate()
        .filter_map(|(r, b)| {
            let (x0, y0, x1, y1) = (*b)?;
            let (w, h) = ((x1 - x0) as f64, (y1 - y0) as f64);
            let extent = if long { w.max(h) } else { w.min(h) };
            if !viol(extent) {
                return None;
            }
            let (cx, cy) = labeled.regions[r].marker;
            let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
            Some(Violation::point(
                rid,
                label,
                format!(
                    "{}: {} {:.4} µm {} {:.4} µm at ({:.4}, {:.4}) µm",
                    lname,
                    word,
                    extent * dbu_to_um,
                    cmp,
                    limit,
                    ux,
                    uy
                ),
                ux,
                uy,
            ))
        })
        .collect()
}

// ===========================================================================
// Enclosure engine.
//
// Every region on the enclosed layer (`layers[1]`) must sit inside an enclosing
// region (`layers[0]`) with a margin on its sides.  `min_enclosure` requires the
// margin on *every* side (the worst/min side ≥ value); `min_endcap_enclosure`
// requires it on at least *one* side (the best/max side ≥ value — a wire endcap).
// `endcap` selects the per-region reduction; everything else is shared.
// ===========================================================================

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
    match rule.str_params.get("metric").map(String::as_str) {
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
    pub fn of(rule: &RuleDefinition) -> Self {
        match rule.str_params.get("sides").map(String::as_str) {
            Some("any") => Sides::Any,
            Some("adjacent") => Sides::Adjacent,
            Some("line_end") => Sides::LineEnd,
            Some("all") | None => Sides::All,
            Some(other) => {
                eprintln!(
                    "[{}] unknown sides '{other}' — expected all, any, adjacent or \
                     line_end; using all",
                    rule.id
                );
                Sides::All
            }
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
pub fn run_enclosure(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    sides: Sides,
) -> Vec<Violation> {
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

    println!(
        "[{}] Checking {} >= {:.2} µm of {} within {}",
        rule.id, rule.check, rule.value, enclosed_layer.name, enclosing_layer.name
    );

    let tile = merged.tile_dbu() as i64;
    let tol = 0.5 * dbu_to_um;
    let value = rule.value;
    let rid = rule.id.as_str();
    let aname = enclosing_layer.name.as_str();
    let bname = enclosed_layer.name.as_str();
    // KLayout's `enclosed` only checks enclosed shapes that actually overlap an enclosing
    // region (a via far from any MIM is not a MIM via).  Opt-in via the `interacting_only`
    // param so the default "must be inside" behaviour (e.g. Cont within Metal1) is unchanged.
    let interacting_only = rule
        .params
        .get("interacting_only")
        .is_some_and(|v| *v != 0.0);
    // Ignore inner edges coincident with the enclosing contour (clip artifacts of an
    // `intersection`-derived enclosed layer) — mirrors KLayout's `consider_intersecting_
    // edges: false` / `without_distance(0)` rule flags.  Off by default: a genuinely flush
    // edge is a real 0-margin violation (e.g. Rppd.b).
    let skip_coincident = rule
        .params
        .get("skip_coincident")
        .is_some_and(|v| *v != 0.0);
    // Stronger, region-level variant: skip the *whole* enclosed region if any of its edges
    // is coincident with the enclosing contour — i.e. the region reaches the boundary and
    // is not "surrounded entirely by" the enclosing layer.  NW.e's title says exactly that:
    // a tie crossing the NWell edge is external-tie territory (NW.d), not NW.e's.  This is
    // also halo-robust: the clip is a local property of the region, unlike its remaining
    // margins whose tile ownership can shift with per-suite halos.
    let skip_clipped = rule.params.get("skip_clipped").is_some_and(|v| *v != 0.0);
    // `sides: adjacent` only: the margin below which a side starts asking something of
    // the sides bordering it.
    let trigger = rule.params.get("trigger").copied().unwrap_or(0.0);
    // `sides: line_end` only: what counts as a narrow track, and how far it must run
    // before it is a line rather than a notch.
    let max_width = rule
        .params
        .get("max_width")
        .copied()
        .unwrap_or(f64::INFINITY);
    let min_length = rule.params.get("min_length").copied().unwrap_or(0.0);
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

                // Best-case enclosing shape (greatest margin) among those containing B.
                let mut best_dist = f64::NEG_INFINITY;
                let mut best_edge = None;
                let mut any_contained = false;
                let mut clipped = false;
                for a in &a_conv {
                    if !all_vertices_inside(&bp, a, tol) {
                        continue;
                    }
                    any_contained = true;
                    let (dist, edge) = if sides == Sides::Any {
                        enclosure_dist_endcap(&bp, a)
                    } else {
                        let (pairs, coincident) =
                            enclosure_pairs(&bp, a, value, skip_coincident, euclidian, tol);
                        clipped |= coincident;
                        // Wall reality check: a pair measured against outer geometry
                        // beyond this bucket's reliable zone can see a fake wall where
                        // the union was truncated; probing just past the wall in the
                        // probe's own tile (complete there) exposes and drops it.
                        let mut worst = f64::INFINITY;
                        let mut worst_edge = bp.edges.first().copied().unwrap_or_default();
                        for p in pairs {
                            if p.dist < worst
                                && !point_in_layer_at_own_tile(map_a, tile, dbu_to_um, p.probe)
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
                    let caps: Vec<(f64, f64, f64, f64)> = a_conv
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
                    let containing: Vec<&Poly> = a_conv
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
                    if interacting_only {
                        // Skip shapes that overlap no enclosing region at all — they
                        // are not subject to this enclosure rule.
                        let touching: Vec<&Poly> =
                            a_conv.iter().filter(|a| polys_interact(&bp, a)).collect();
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
                } else if best_dist + tol < value {
                    let (x1, y1, x2, y2) = best_edge.unwrap_or_default();
                    out.push(Violation::edge(
                        rid,
                        "Minimum enclosure violation",
                        format!(
                            "enclosure {best_dist:.4} µm < {value:.2} µm of {bname} within {aname} \
                             at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                        ),
                        x1, y1, x2, y2,
                    ));
                }
            }
            out.into_iter()
        })
        .collect()
}

/// Maximum enclosure: every shape on the enclosed layer (`layers[1]`) that sits inside an
/// enclosing region (`layers[0]`) must have **no more than** the rule's margin on every
/// side.  Mirrors [`run_enclosure`]'s containment/margin computation (same `enclosure_dist`,
/// picking the containing candidate with the largest worst-side margin), but a shape not
/// contained by any enclosing region isn't a "too much margin" case, so it's silently
/// skipped rather than reported — that absence is [`run_enclosure`]'s concern.  Used for
/// the PDF's "min. and max." device-shape rules (e.g. Sdiod.a/b/c), paired with a
/// `min_enclosure` entry at the same value to pin the margin to (near) exactly that value.
pub fn run_max_enclosure(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let euclidian = enclosure_is_euclidian(rule);
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

    println!(
        "[{}] Checking {} <= {:.2} µm of {} within {}",
        rule.id, rule.check, rule.value, enclosed_layer.name, enclosing_layer.name
    );

    let tile = merged.tile_dbu() as i64;
    let tol = 0.5 * dbu_to_um;
    let value = rule.value;
    let rid = rule.id.as_str();
    let aname = enclosing_layer.name.as_str();
    let bname = enclosed_layer.name.as_str();

    let map_a = merged.tiles(al, ad);
    let map_b = merged.tiles(bl, bd);
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
                let Some(bp) = poly_from_merged(bm, dbu_to_um) else {
                    continue;
                };

                let mut best_dist = f64::NEG_INFINITY;
                let mut best_edge = None;
                for a in &a_conv {
                    if !all_vertices_inside(&bp, a, tol) {
                        continue;
                    }
                    // Coincident (0-margin) segments can never exceed a max bound, so the
                    // skip flag is irrelevant here; keep them (false) for the worst-margin.
                    // No wall reality check either: a fake wall only *shrinks* the measured
                    // worst margin, which for a max bound errs toward passing — harmless.
                    let (pairs, _) = enclosure_pairs(&bp, a, f64::INFINITY, false, euclidian, tol);
                    let mut dist = f64::INFINITY;
                    let mut edge = bp.edges.first().copied().unwrap_or_default();
                    for p in pairs {
                        if p.dist < dist {
                            dist = p.dist;
                            edge = p.edge;
                        }
                    }
                    if dist > best_dist {
                        best_dist = dist;
                        best_edge = Some(edge);
                    }
                }

                if best_edge.is_some() && best_dist - tol > value {
                    let (x1, y1, x2, y2) = best_edge.unwrap_or_default();
                    out.push(Violation::edge(
                        rid,
                        "Maximum enclosure violation",
                        format!(
                            "enclosure {best_dist:.4} µm > {value:.2} µm of {bname} within {aname} \
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

#[cfg(test)]
mod tests {
    use super::*;
    use i_overlay::i_float::int::point::IntPoint;

    fn pt(x: i32, y: i32) -> IntPoint {
        IntPoint::new(x, y)
    }

    fn core() -> Core {
        Core {
            x0: -1_000_000,
            y0: -1_000_000,
            x1: 1_000_000,
            y1: 1_000_000,
        }
    }

    /// The square metric is L-infinity, so a purely diagonal separation reads as the
    /// larger of the two axis distances and not the hypotenuse. Two collinear-facing
    /// points 3 across and 4 up are 5 apart euclidian and 4 apart here — which is the
    /// whole reason a rule worded as a square at a corner has to ask for it.
    #[test]
    fn square_metric_is_the_larger_axis_distance() {
        let (d, _, _) = seg_seg_closest_square((0.0, 0.0), (0.0, 0.0), (3.0, 4.0), (3.0, 4.0));
        assert!((d - 4.0).abs() < 1e-9, "expected 4, got {d}");
        let (e, _, _) = segment_closest_points(0.0, 0.0, 0.0, 0.0, 3.0, 4.0, 3.0, 4.0);
        assert!(
            (e - 5.0).abs() < 1e-9,
            "euclidian should still be 5, got {e}"
        );
    }

    /// The minimum sits on the `|dx| = |dy|` fold, not at the euclidian closest approach,
    /// and that is the case the fold-vertex search exists for. Two parallel 45° walls
    /// offset by 6 in y are 4.243 apart euclidian; the largest square that fits between
    /// them has side 3, reached by sliding along both until the axis distances match.
    #[test]
    fn square_metric_measures_across_the_diagonal_fold() {
        let a = ((0.0, 0.0), (4.0, 4.0));
        let b = ((0.0, 6.0), (4.0, 10.0));
        let (d, pa, pb) = seg_seg_closest_square(a.0, a.1, b.0, b.1);
        assert!((d - 3.0).abs() < 1e-9, "expected 3, got {d}");
        // Both axis distances equal the result: that is what being on the fold means.
        assert!((pa.0 - pb.0).abs() - 3.0 < 1e-9 && (pa.1 - pb.1).abs() - 3.0 < 1e-9);
        let (e, _, _) = segment_closest_points(0.0, 0.0, 4.0, 4.0, 0.0, 6.0, 4.0, 10.0);
        assert!(
            (e - 6.0 / 2f64.sqrt()).abs() < 1e-9,
            "euclidian is 4.243, got {e}"
        );
    }

    /// Thin 45° trace (~99 DBU walls) flagged by a `< 160` (min-width) predicate:
    /// both walls reported, nothing from the orthogonal end-caps.
    #[test]
    fn oblique_45_thin_trace_flags_both_walls() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(1000, 1000), pt(1000, 1140), pt(0, 140)],
            holes: vec![],
        };
        let v = scan_widths(
            &poly,
            core(),
            0.001,
            "T",
            "min",
            "L",
            0.16,
            "<",
            |w| w < 160.0 - 0.5,
            false,
            true,
            0.5,
            None,
        );
        assert_eq!(v.len(), 2, "got {}", v.len());
    }

    #[test]
    fn oblique_45_wide_trace_is_clean() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(1000, 1000), pt(1000, 2400), pt(0, 1400)],
            holes: vec![],
        };
        let v = scan_widths(
            &poly,
            core(),
            0.001,
            "T",
            "min",
            "L",
            0.16,
            "<",
            |w| w < 160.0 - 0.5,
            false,
            true,
            0.5,
            None,
        );
        assert!(v.is_empty(), "got {}", v.len());
    }

    /// A well with a 45° chamfer across one corner: the strip above the chamfer is only
    /// 500 DBU wide, bounded by the chamfer on one side and the vertical edge on the
    /// other.  No pass pairs those two — vertical-with-vertical, horizontal-with-
    /// horizontal, oblique-with-anti-parallel-oblique — so before `mixed_widths` this
    /// shape came back clean at any rule value.  Taken from the gf180mcu nwell test case,
    /// where it is the single most common miss.
    #[test]
    fn chamfered_corner_narrows_against_the_opposite_wall() {
        let poly = MergedPoly {
            outer: vec![
                pt(0, 0),
                pt(1000, 0),
                pt(1000, 2000),
                pt(500, 2000),
                pt(0, 1500),
            ],
            holes: vec![],
        };
        let scan = |limit_dbu: f64, mixed: bool| {
            scan_widths(
                &poly,
                core(),
                0.001,
                "T",
                "min",
                "L",
                limit_dbu / 1000.0,
                "<",
                |w| w < limit_dbu - 0.5,
                false,
                mixed,
                0.5,
                None,
            )
        };
        // 860 DBU rule: the 500-wide strip violates, and the marker spans the gap.
        let v = scan(860.0, true);
        assert_eq!(v.len(), 1, "got {}", v.len());
        assert!(
            v[0].message.contains("width 0.5000"),
            "measured the wrong span: {}",
            v[0].message
        );
        // Below the narrow strip the shape is a clean 1000 wide, so a 500 rule passes.
        assert!(scan(500.0, true).is_empty());
        // And without the mixed pass the violation is invisible — the regression itself.
        assert!(scan(860.0, false).is_empty());
    }

    /// A 200×200 DBU square flagged by a `> 150` (max-width) predicate: both
    /// dimensions exceed, two walls each → 4.
    #[test]
    fn max_width_square_flags_four_walls() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(200, 0), pt(200, 200), pt(0, 200)],
            holes: vec![],
        };
        let v = scan_widths(
            &poly,
            core(),
            0.001,
            "T",
            "max",
            "L",
            0.15,
            ">",
            |w| w > 150.0 + 0.5,
            false,
            false,
            0.5,
            None,
        );
        assert_eq!(v.len(), 4, "got {}", v.len());
    }
}
