// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Directional extension: where the cover (`layers[0]`) sits over a target region
//! (`layers[1]`), it must extend at least - or at most - `value` past the target's edges
//! it crosses.  Edge-based and local: a target edge counts only where the cover overlaps
//! the target just *inside* it, so the target's free edges - a resistor's ends, or the
//! boundary of a big active the cover merely sits inside - are exempt of themselves, and
//! only the edges the cover actually crosses are read.

use super::Kind;
use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

/// Drive a directional extension check (e.g. Sal.c: SalBlock over Activ/GatPoly).
///
/// Edge-based and local: every `layers[1]` (target) contour edge segment that the
/// `layers[0]` (cover) sits over must have the cover extending at least `value`
/// perpendicular beyond it.  A target edge counts as "covered" only where the cover
/// overlaps the target just *inside* that edge — so the target's free edges (a
/// resistor's ends, or the boundary of a big active the cover merely sits inside) are
/// exempt automatically, and only the long edges the cover actually crosses are checked.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let max = kind == Kind::Max;
    let cover = &rule.layers[0];
    let target = rule.layers.get(1).unwrap_or(cover);
    let (cl, cd) = (cover.gds_layer as i16, cover.gds_datatype as i16);
    let (tl, td) = (target.gds_layer as i16, target.gds_datatype as i16);
    merged.ensure(layout, cl, cd);
    merged.ensure(layout, tl, td);

    println!(
        "[{}] Checking {} {} {:.2} µm of {} over {}",
        rule.id,
        kind.extension_name(),
        kind.op(),
        rule.value,
        cover.name,
        target.name
    );
    let title = match kind {
        Kind::Min => "Minimum extension violation",
        Kind::Max => "Maximum extension violation",
    };

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
                    // How far the cover actually reaches outward past this edge point,
                    // for reporting the measured extension: a minimum needs it only up
                    // to the value, a maximum a little past it.
                    let reach_to = if max { 2.0 * value } else { value };
                    let measure = |px: f64, py: f64| {
                        let mut d = eps;
                        let mut reached = 0.0;
                        while d <= reach_to + eps {
                            if covered(px + onx * d, py + ony * d) {
                                reached = d;
                                d += dbu_to_um;
                            } else {
                                break;
                            }
                        }
                        reached.min(reach_to)
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
                        let how = if max {
                            format!("{cn} extends {worst:.3} µm over {tn} (at most {value:.2} µm)")
                        } else {
                            format!(
                                "{cn} extends only {worst:.3} µm over {tn} (needs {value:.2} µm)"
                            )
                        };
                        Some(Violation::edge(
                            rid,
                            title,
                            format!("{how} at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"),
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
                        // `value` so an exactly-`value` extension counts as covered - and
                        // for a maximum just past it, so exactly `value` is not too much.
                        let over = covered(px + inx * eps, py + iny * eps);
                        let failing = over
                            && if max {
                                covered(px + onx * (value + eps), py + ony * (value + eps))
                            } else {
                                !covered(px + onx * (value - eps), py + ony * (value - eps))
                            };
                        if failing {
                            if span_start.is_none() {
                                span_start = Some((px, py));
                                worst = if max { 0.0 } else { value };
                            }
                            span_end = (px, py);
                            let m = measure(px, py);
                            worst = if max { worst.max(m) } else { worst.min(m) };
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
