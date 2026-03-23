// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum notch — the dual of [`min_width`](super::min_width).
//!
//! A notch is the gap between two inward-facing edges of the **same** merged
//! region where the area between them is empty (a concave slot, or a thin hole).
//! It runs on the shared [`MergedCache`] with the same rectilinear scan + oblique
//! pass as width, but the pairing is inverted (an empty gap between facing walls
//! instead of metal), and each notch is reported once.

use super::helper::{collect_edges, segment_closest_points, sorted_unique, HEdge, OEdge, VEdge};
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

fn check_poly(
    poly: &MergedPoly,
    core: Core,
    min_n_dbu: f64,
    dbu_to_um: f64,
    rule_id: &str,
    layer: &str,
) -> Vec<Violation> {
    let mut vedges = Vec::new();
    let mut hedges = Vec::new();
    let mut oedges = Vec::new();
    collect_edges(poly, &mut vedges, &mut hedges, &mut oedges);

    let mut out = Vec::new();
    let limit_um = min_n_dbu * dbu_to_um;
    // One violation per notch: a single edge spanning the empty gap.
    let mut emit = |x1: f64, y1: f64, x2: f64, y2: f64, gap_dbu: f64| {
        let g = gap_dbu * dbu_to_um;
        out.push(Violation::edge(
            rule_id,
            "Minimum notch violation",
            format!(
                "{}: notch {:.4} µm < {:.2} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                layer, g, limit_um,
                x1 * dbu_to_um, y1 * dbu_to_um, x2 * dbu_to_um, y2 * dbu_to_um
            ),
            x1 * dbu_to_um, y1 * dbu_to_um, x2 * dbu_to_um, y2 * dbu_to_um,
        ));
    };

    // Horizontal-direction notches: scan y bands, pair vertical edges across x.
    // An empty gap is `right wall` (metal to -x) then `left wall` (metal to +x).
    let y_events = sorted_unique(vedges.iter().flat_map(|e| [e.ylo, e.yhi]).collect());
    for w in y_events.windows(2) {
        let (yb, yb1) = (w[0], w[1]);
        if yb1 <= yb { continue; }
        let mut active: Vec<&VEdge> = vedges.iter().filter(|e| e.ylo <= yb && e.yhi >= yb1).collect();
        active.sort_unstable_by_key(|e| (e.x, e.left_wall));
        for pair in active.windows(2) {
            let (l, r) = (pair[0], pair[1]);
            if !l.left_wall && r.left_wall {
                let gap = r.x - l.x;
                if gap > 0 && (gap as f64) < min_n_dbu - 0.5 {
                    let cx = (l.x as f64 + r.x as f64) * 0.5;
                    let cy = (yb as f64 + yb1 as f64) * 0.5;
                    if core.contains(cx, cy) {
                        emit(l.x as f64, cy, r.x as f64, cy, gap as f64);
                    }
                }
            }
        }
    }

    // Vertical-direction notches: scan x bands, pair horizontal edges across y.
    let x_events = sorted_unique(hedges.iter().flat_map(|e| [e.xlo, e.xhi]).collect());
    for w in x_events.windows(2) {
        let (xb, xb1) = (w[0], w[1]);
        if xb1 <= xb { continue; }
        let mut active: Vec<&HEdge> = hedges.iter().filter(|e| e.xlo <= xb && e.xhi >= xb1).collect();
        active.sort_unstable_by_key(|e| (e.y, e.bottom_wall));
        for pair in active.windows(2) {
            let (b, t) = (pair[0], pair[1]);
            if !b.bottom_wall && t.bottom_wall {
                let gap = t.y - b.y;
                if gap > 0 && (gap as f64) < min_n_dbu - 0.5 {
                    let cx = (xb as f64 + xb1 as f64) * 0.5;
                    let cy = (b.y as f64 + t.y as f64) * 0.5;
                    if core.contains(cx, cy) {
                        emit(cx, b.y as f64, cx, t.y as f64, gap as f64);
                    }
                }
            }
        }
    }

    oblique_notches(&oedges, core, min_n_dbu, &mut emit);
    mixed_notches(&vedges, &hedges, &oedges, core, min_n_dbu, &mut emit);
    out
}

/// True if directed segments `a-b` and `c-d` share an endpoint — they meet directly
/// along the contour, so any "gap" between them is zero-width by construction, not a
/// real notch.  Coordinates come straight from the integer grid, so exact equality is
/// safe (no arithmetic has been done on them yet).
#[allow(clippy::too_many_arguments)]
fn shares_endpoint(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, dx: f64, dy: f64) -> bool {
    (ax == cx && ay == cy) || (ax == dx && ay == dy) || (bx == cx && by == cy) || (bx == dx && by == dy)
}

/// Notches between a rectilinear edge (vertical or horizontal) and an oblique edge of
/// the same region — the case neither the banded rectilinear scan (which only pairs
/// two vertical or two horizontal edges) nor `oblique_notches` (which only pairs two
/// mutually anti-parallel diagonals) can see, since the pair isn't parallel at all.  A
/// diagonal stroke closing in on a straight stem — the tip of a "V", the leg of a "K"
/// or "M" — is exactly this shape.
///
/// Uses the general segment-to-segment closest-point primitive (already proven for
/// inter-region spacing) instead of a parallel-corridor projection.  Classification
/// mirrors [`oblique_notches`]: every edge carries metal on its left (the same
/// convention `collect_edges` gives every edge, rectilinear or oblique, since they all
/// derive from the same walk of the region's outer/hole contours), so the pair is a
/// real notch only when each edge's closest point on the *other* edge falls on its own
/// right (empty) side — each wall must face away from its own metal, toward the gap.
fn mixed_notches(
    vedges: &[VEdge],
    hedges: &[HEdge],
    oedges: &[OEdge],
    core: Core,
    min_n_dbu: f64,
    emit: &mut impl FnMut(f64, f64, f64, f64, f64),
) {
    if oedges.is_empty() || (vedges.is_empty() && hedges.is_empty()) {
        return;
    }
    // Directed rectilinear segments, metal on the left of travel — the same convention
    // `collect_edges` already gives oblique edges (both derive from the same walk of
    // the original CCW outer / CW hole contours).
    let rect = vedges
        .iter()
        .map(|v| {
            if v.left_wall {
                (v.x as f64, v.yhi as f64, v.x as f64, v.ylo as f64)
            } else {
                (v.x as f64, v.ylo as f64, v.x as f64, v.yhi as f64)
            }
        })
        .chain(hedges.iter().map(|h| {
            if h.bottom_wall {
                (h.xlo as f64, h.y as f64, h.xhi as f64, h.y as f64)
            } else {
                (h.xhi as f64, h.y as f64, h.xlo as f64, h.y as f64)
            }
        }))
        .collect::<Vec<_>>();

    for &(ax, ay, bx, by) in &rect {
        let (rdx, rdy) = (bx - ax, by - ay);
        for o in oedges {
            let (cx, cy, dx, dy) = (o.ax as f64, o.ay as f64, o.bx as f64, o.by as f64);
            if shares_endpoint(ax, ay, bx, by, cx, cy, dx, dy) {
                continue;
            }
            let (odx, ody) = (dx - cx, dy - cy);
            let (dist, (px, py), (qx, qy)) = segment_closest_points(ax, ay, bx, by, cx, cy, dx, dy);
            if dist <= 0.5 || dist >= min_n_dbu - 0.5 {
                continue;
            }
            // Q (on O) must sit right of R, and P (on R) must sit right of O — both
            // walls facing their shared gap, not each other's metal.
            let cross_r = rdx * (qy - ay) - rdy * (qx - ax);
            let cross_o = odx * (py - cy) - ody * (px - cx);
            if cross_r >= 0.0 || cross_o >= 0.0 {
                continue;
            }
            let mx = (px + qx) * 0.5;
            let my = (py + qy) * 0.5;
            if !core.contains(mx, my) {
                continue;
            }
            emit(px, py, qx, qy, dist);
        }
    }
}

/// Oblique notches: anti-parallel edge pairs of one region with **empty** between
/// them (the metal is on the outer sides).  Mirror of the oblique width pass with
/// the gap on the non-metal side of each edge.
fn oblique_notches(
    oedges: &[OEdge],
    core: Core,
    min_n_dbu: f64,
    emit: &mut impl FnMut(f64, f64, f64, f64, f64),
) {
    let n = oedges.len();
    for i in 0..n {
        let ei = &oedges[i];
        let (dix, diy) = ((ei.bx - ei.ax) as f64, (ei.by - ei.ay) as f64);
        let li = dix.hypot(diy);
        if li == 0.0 { continue; }
        let (ux, uy) = (dix / li, diy / li);
        let (nx, ny) = (-diy / li, dix / li); // left (metal-side) normal
        for ej in &oedges[i + 1..] {
            let (djx, djy) = ((ej.bx - ej.ax) as f64, (ej.by - ej.ay) as f64);
            if (dix * djy - diy * djx).abs() > 1e-6 || (dix * djx + diy * djy) >= 0.0 {
                continue;
            }
            // Notch: ej on the non-metal (right) side of ei ⇒ negative distance;
            // the gap is the empty span between them.
            let dist = (ej.ax - ei.ax) as f64 * nx + (ej.ay - ei.ay) as f64 * ny;
            let gap = -dist;
            if gap <= 0.5 || gap >= min_n_dbu - 0.5 {
                continue;
            }
            let taj = (ej.ax - ei.ax) as f64 * ux + (ej.ay - ei.ay) as f64 * uy;
            let tbj = (ej.bx - ei.ax) as f64 * ux + (ej.by - ei.ay) as f64 * uy;
            let lo = taj.min(tbj).max(0.0);
            let hi = taj.max(tbj).min(li);
            if hi - lo <= 0.5 {
                continue;
            }
            let mid = (lo + hi) * 0.5;
            let eix = ei.ax as f64 + mid * ux;
            let eiy = ei.ay as f64 + mid * uy;
            let ejx = eix + nx * dist;
            let ejy = eiy + ny * dist;
            if !core.contains((eix + ejx) * 0.5, (eiy + ejy) * 0.5) {
                continue;
            }
            emit(eix, eiy, ejx, ejy, gap);
        }
    }
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let min_n_dbu = rule.value / dbu_to_um;
    let tile = merged.tile_dbu() as i64;

    for layer in &rule.layers {
        let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, gl, gd);

        println!(
            "[{}] Checking min_notch >= {:.2} µm on layer {} ({}/{})",
            rule.id, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        let rid = rule.id.as_str();
        let lname = layer.name.as_str();
        let mut layer_violations: Vec<Violation> = merged
            .tiles(gl, gd)
            .par_iter()
            .flat_map_iter(|(&(tx, ty), polys)| {
                let core = Core {
                    x0: tx as i64 * tile, y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
                };
                polys
                    .iter()
                    .flat_map(move |poly| check_poly(poly, core, min_n_dbu, dbu_to_um, rid, lname))
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .collect();

        violations.append(&mut layer_violations);
    }

    violations
}
