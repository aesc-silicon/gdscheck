// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared helpers used by several checks: the tile drivers and pairing engines that more
//! than one rule reads - spacing, enclosure, extension, angle and extent - and the
//! marker plumbing they share.  A family with a home of its own lives there instead; the
//! width scan is under [`super::width`].

use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly, representative_point};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

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
    /// share none for their closest approach.  See [`facing_pairs_i`].
    overlapping: bool,
    /// Measure L-infinity rather than euclidian.  See [`seg_seg_closest_square`].
    square: bool,
    /// Measure how deeply the pair penetrates rather than how far apart it is - the
    /// same facing-edge scan run inward.  Implies `overlapping`.
    inward: bool,
}

/// A region's float outline, built the first time a reading asks for it: the spacing,
/// the gates and the facing scan are measured exactly on the integer boundary, and only
/// the square metric reads the µm contour.
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
    // under it or not.  The one float reading below, the square metric, keeps half a
    // DBU of slack instead.
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
                // Take the narrowest facing gap that is genuinely empty instead - or,
                // inward, the shallowest facing overlap that is genuinely material.
                let mut best: Option<ClosestPair> = None;
                for c in facing_pairs_i(&a.outline, &b.outline, limit.dbu(), mode.inward) {
                    let (p, q) = (c.2, c.3);
                    let (mx, my) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
                    let wrong = if mode.inward {
                        // An overlap has to be material of both, or the "facing" pair
                        // reaches across a notch in one of them.
                        !(crate::merge::point_in_merged(mx, my, a.outline.poly())
                            && crate::merge::point_in_merged(mx, my, b.outline.poly()))
                    } else {
                        a_polys
                            .iter()
                            .chain(b_polys)
                            .any(|m| crate::merge::point_in_merged(mx, my, m))
                    };
                    if wrong {
                        continue; // another arm of one of the shapes lies in the gap
                    }
                    if best.is_none_or(|b| (c.0 as f64 / c.1 as f64) < (b.0 as f64 / b.1 as f64)) {
                        best = Some(c);
                    }
                }
                best.map(|(num, den, p, q)| {
                    (
                        (num as f64 / den as f64).sqrt() * dbu_to_um,
                        (p.0 * dbu_to_um, p.1 * dbu_to_um),
                        (q.0 * dbu_to_um, q.1 * dbu_to_um),
                    )
                })
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
/// The facing-edge scan of [`run_gated`] run inward: see [`facing_pairs_i`].
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
// Directional extension engine.
//
// For each `layers[0]` (cover) region that overlaps a `layers[1]` (target) region, the
// cover must extend at least `value` beyond the target's two **long** edges — i.e. in
// the target's *width* (short-axis) direction.  The target's ends (short edges) are
// exempt, since it legitimately runs out past the cover there.  Exact for the
// axis-aligned rectangles this targets (a SalBlock placed across a long Activ/GatPoly).
// ===========================================================================

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
