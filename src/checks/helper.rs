// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared helpers used by several checks: the tile drivers and pairing engines that more
//! than one rule reads - spacing, enclosure, extension, angle and extent - and the
//! marker plumbing they share.  A family with a home of its own lives there instead; the
//! width scan is under [`super::width`].

use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{
    Core, MergedCache, MergedPoly, VirtualOp, compose_tile, merged_centroid_dbu,
    representative_point,
};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashSet;

// ===========================================================================
// Region-to-region spacing engine.
//
// Spacing is measured between **merged** regions on the cached tiles; a pair within
// `value` is reported only if a caller-supplied `gate(a, b)` holds.  A plain
// `min_space` passes `|_, _| true`; its gates (`angle`, `net`, `width`, `length` - see
// [`super::space`]) supply a predicate.  Each violation is kept only if its gap
// midpoint lies in the tile core, so a pair seen from several overlapping tiles is
// reported once.
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

/// A region's float outline, built the first time a reading asks for it: the spacing
/// and the gates are measured exactly on the integer boundary, and only the square
/// metric and the facing scan of an overlapping pair read the µm contour.
struct LazyPoly<'a> {
    m: &'a MergedPoly,
    dbu_to_um: f64,
    cell: std::cell::OnceCell<Option<Poly>>,
}

impl LazyPoly<'_> {
    /// The µm outline, `None` for a degenerate region.
    fn poly(&self) -> Option<&Poly> {
        self.cell
            .get_or_init(|| poly_from_merged(self.m, self.dbu_to_um))
            .as_ref()
    }
}

#[allow(clippy::too_many_arguments)]
fn check_tile<'a, G: Fn(&Outline, &Outline, Marker, Marker) -> bool>(
    a_polys: &'a [MergedPoly],
    b_polys: &'a [MergedPoly],
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
    // The bound on the grid: a minimum rounds up, and every gap is then an integer
    // under it or not.  The float readings below - the square metric, the facing scan
    // of an overlapping pair - keep half a DBU of slack instead.
    let limit = Limit::at_least(value, dbu_to_um);
    // Each region keeps its DBU marker: a point *on* the shape, not its centroid - a
    // net-aware gate resolves the net by looking the marker up, and a ring's centroid
    // sits in its hole, where in GF180's DN.2b fixture an unrelated island sits.
    //
    // Whether a merged piece is real material or a shaving the merge left behind.  A
    // 45° wall is drawn with a one-nanometre chamfer at each corner, and rounding that
    // corner detaches the chamfer as a triangle of half a square nanometre touching the
    // body at a vertex.  The width checks want it - KLayout keeps the same feature and
    // reports the notch - but a *gap* of nothing to a shaving is not a spacing
    // violation, and reading it as one is five false positives each on LRES.2 and
    // PRES.2.  Nothing drawn is this small: a hundred square DBU is a ten-nanometre
    // square.
    struct Side<'a> {
        outline: Outline<'a>,
        lazy: LazyPoly<'a>,
        marker: Marker,
        material: bool,
    }
    let prep = |ms: &'a [MergedPoly]| -> Vec<Side<'a>> {
        ms.iter()
            .filter(|m| m.outer.len() >= 3)
            .map(|m| Side {
                outline: Outline::new(m),
                lazy: LazyPoly {
                    m,
                    dbu_to_um,
                    cell: std::cell::OnceCell::new(),
                },
                marker: representative_point(m),
                material: crate::merge::merged_area_dbu(m) >= SHAVING_DBU2,
            })
            .collect()
    };
    let sa = prep(a_polys);
    let sb = if same_layer {
        Vec::new()
    } else {
        prep(b_polys)
    };
    let bs: &[Side] = if same_layer { &sa } else { &sb };

    let mut out = Vec::new();
    for (i, a) in sa.iter().enumerate() {
        for (j, b) in bs.iter().enumerate() {
            if same_layer && j <= i {
                continue;
            }
            if !a.outline.possibly_within(&b.outline, limit.dbu()) {
                continue;
            }
            let overlaps = regions_overlap(&a.outline, &b.outline);
            if overlaps != mode.overlapping {
                continue; // this rule is about the other kind of pair
            }
            let shaving = !a.material || !b.material;
            // The gap and the two points that measure it, in µm.
            let found: Option<Gap> = if mode.overlapping {
                // The pair shares area, so its closest approach is zero and meaningless.
                // Take the narrowest facing gap that is genuinely empty instead.
                let (Some(pa), Some(pb)) = (a.lazy.poly(), b.lazy.poly()) else {
                    continue;
                };
                let mut best: Option<Gap> = None;
                for (gap, p, q) in facing_gaps(pa, pb, value, half, mode.inward) {
                    let (px, py) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
                    let wrong = if mode.inward {
                        // An overlap has to be material of both, or the "facing" pair
                        // reaches across a notch in one of them.
                        !(pa.contains_point(px, py) && pb.contains_point(px, py))
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
                best.filter(|(d, _, _)| *d < value - half)
            } else if mode.square {
                let (Some(pa), Some(pb)) = (a.lazy.poly(), b.lazy.poly()) else {
                    continue;
                };
                let m = closest(pa, pb, half, true);
                if m.0 < half && (shaving || shares_boundary_run(pa, pb, half)) {
                    continue;
                }
                (m.0 < value - half).then_some(m)
            } else {
                // A contact between two layers is a separation of *zero*, and reported:
                // two shapes meeting at a corner have a gap that happens to be nothing
                // wide, which is the worst spacing there is.  KLayout reads it that way
                // and this engine used to drop it, silently, wherever it occurred.
                //
                // Within one layer it is a gap of zero too, and KLayout reports it: two
                // wells meeting corner to corner merge into one self-touching shape, and
                // `space` marks the touch point.  This engine excluded the same-layer
                // case for a while, on the grounds that such a contact is a pinch and
                // belongs to the width checks; that reading cost 82 logical violations
                // across eighteen gf180mcu decks - the whole of what `nwell` and
                // `lvpwell` were missing on their well-spacing rules - and the pinch is
                // reported anyway, by whichever width rule covers the layer.
                //
                // A contact along a *run* is not a gap: two shapes drawn edge to edge
                // abut, with no space between them anywhere.  IHP's butted substrate
                // ties are exactly that by construction.  Only a contact at isolated
                // points is a separation of zero, whichever layers it is between.
                match closest_approach(&a.outline, &b.outline, limit.dbu()) {
                    None => None,
                    Some((0, _, p, q)) => {
                        if shaving || share_boundary_run(&a.outline, &b.outline) {
                            None
                        } else {
                            Some((0.0, p, q))
                        }
                    }
                    Some((num, den, p, q)) => limit.broken_by_sq(num, den).then(|| {
                        let d = (num as f64 / den as f64).sqrt() * dbu_to_um;
                        (d, p, q)
                    }),
                }
                .map(|(d, (px, py), (qx, qy))| {
                    (
                        d,
                        (px * dbu_to_um, py * dbu_to_um),
                        (qx * dbu_to_um, qy * dbu_to_um),
                    )
                })
            };
            let Some((min_dist, (ax, ay), (bx, by))) = found else {
                continue;
            };
            if !gate(&a.outline, &b.outline, a.marker, b.marker) {
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

pub fn run_gated<G: Fn(&Outline, &Outline, Marker, Marker) -> bool + Sync>(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    gate: G,
) -> Vec<Violation> {
    run_gated_with(rule, layout, dbu_to_um, merged, None, gate)
}

/// `forced` overrides what the `pairs` param would say, for a check that *is* a mode.
fn run_gated_with<G: Fn(&Outline, &Outline, Marker, Marker) -> bool + Sync>(
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
    let overlapping_pairs = match rule.word("pairs") {
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
    let square = match rule.word("metric") {
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

    let forbidden = rule.num("angle"); // specific forbidden orientation
    let tol = rule.num("tolerance").unwrap_or(1.0);
    // Allowed orientations are multiples of `step` degrees; 90 (the default) permits
    // only axis-aligned edges, 45 also permits the diagonals.  GF180's ACUTE rules want
    // the latter - they allow 0, 45, 90 and -45 and flag everything else.
    let step = rule.num("step").unwrap_or(90.0);

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
    let per_tile: Vec<Vec<(usize, crate::merge::BBoxDbu)>> = labeled
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
