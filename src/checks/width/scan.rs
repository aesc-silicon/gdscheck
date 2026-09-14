// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The width scan as the checks drive it: [`width_pairs`](crate::geom::width_pairs)
//! measured tile by tile, the pinch points no pair of walls can express, and each result
//! written out as a violation.  Every rule in [`super`](super) comes through
//! [`run_width`]; the gate length comes through [`run_gate_length`], which is the same
//! scan under a mask.

use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

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

/// Gate-length check: measure the facing-wall width of `layers[0]` (the gate poly)
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

    /// A region with a 45° chamfer across one corner: the strip above the chamfer is only
    /// 500 DBU wide, bounded by the chamfer on one side and the vertical edge on the
    /// other.  None of the like-with-like passes pairs those two - vertical with
    /// vertical, horizontal with horizontal, oblique with anti-parallel oblique - so
    /// without the mixed pass this shape reads clean at any value.
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
