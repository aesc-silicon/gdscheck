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

use crate::layout::FlatLayout;
use crate::merge::{compose_tile, merged_centroid_dbu, Core, MergedCache, MergedPoly, VirtualOp};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use i_overlay::i_float::int::point::IntPoint;
use rayon::prelude::*;
use std::collections::HashSet;

/// Vertical edge: `left_wall` true ⇒ metal to its right (edge directed down).
pub(crate) struct VEdge { pub x: i32, pub ylo: i32, pub yhi: i32, pub left_wall: bool }
/// Horizontal edge: `bottom_wall` true ⇒ metal above it (edge directed right).
pub(crate) struct HEdge { pub y: i32, pub xlo: i32, pub xhi: i32, pub bottom_wall: bool }
/// An oblique directed edge `a → b`; metal is on its left.
pub(crate) struct OEdge { pub ax: i32, pub ay: i32, pub bx: i32, pub by: i32 }

/// Split a merged region's contours (outer + holes) into axis-aligned and oblique
/// directed edges — the shared input of the width scan and its notch dual.
pub(crate) fn collect_edges(
    poly: &MergedPoly,
    vedges: &mut Vec<VEdge>,
    hedges: &mut Vec<HEdge>,
    oedges: &mut Vec<OEdge>,
) {
    let mut add = |contour: &[IntPoint]| {
        let n = contour.len();
        if n < 3 { return; }
        for i in 0..n {
            let a = contour[i];
            let b = contour[if i + 1 == n { 0 } else { i + 1 }];
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            if dx == 0 && dy != 0 {
                vedges.push(VEdge { x: a.x, ylo: a.y.min(b.y), yhi: a.y.max(b.y), left_wall: dy < 0 });
            } else if dy == 0 && dx != 0 {
                hedges.push(HEdge { y: a.y, xlo: a.x.min(b.x), xhi: a.x.max(b.x), bottom_wall: dx > 0 });
            } else if dx != 0 && dy != 0 {
                oedges.push(OEdge { ax: a.x, ay: a.y, bx: b.x, by: b.y });
            }
        }
    };
    add(&poly.outer);
    for h in &poly.holes {
        add(h);
    }
}

pub(crate) fn sorted_unique(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Find facing-wall widths in one merged region and report both walls of any
/// width for which `viol(width_dbu)` holds.
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
    min_run: f64,
    // Optional mask: a rectilinear width pair is only reported when its centre lies inside
    // one of these regions (µm).  Used by gate-length rules to measure the poly width but
    // only where the poly forms the relevant device gate.
    mask: Option<&[Poly]>,
) -> Vec<Violation> {
    let in_mask = |cx: f64, cy: f64| match mask {
        None => true,
        Some(m) => m
            .iter()
            .any(|p| p.contains_point(cx * dbu_to_um, cy * dbu_to_um)),
    };
    let mut vedges = Vec::new();
    let mut hedges = Vec::new();
    let mut oedges = Vec::new();
    collect_edges(poly, &mut vedges, &mut hedges, &mut oedges);

    let mut out = Vec::new();
    let mut push_edge = |x1: f64, y1: f64, x2: f64, y2: f64, w_dbu: f64| {
        let w = w_dbu * dbu_to_um;
        out.push(Violation::edge(
            rule_id,
            label,
            format!(
                "{}: width {:.4} µm {} {:.4} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                layer, w, cmp, limit_um,
                x1 * dbu_to_um, y1 * dbu_to_um, x2 * dbu_to_um, y2 * dbu_to_um
            ),
            x1 * dbu_to_um, y1 * dbu_to_um, x2 * dbu_to_um, y2 * dbu_to_um,
        ));
    };

    // Rectilinear widths (skipped for oblique-only rules such as a 45° width check).
    if !oblique_only {
    // Horizontal widths: scan y bands, pair vertical edges across x.
    let y_events = sorted_unique(vedges.iter().flat_map(|e| [e.ylo, e.yhi]).collect());
    for w in y_events.windows(2) {
        let (yb, yb1) = (w[0], w[1]);
        if yb1 <= yb { continue; }
        let mut active: Vec<&VEdge> = vedges.iter().filter(|e| e.ylo <= yb && e.yhi >= yb1).collect();
        active.sort_unstable_by_key(|e| (e.x, e.left_wall));
        for pair in active.windows(2) {
            let (l, r) = (pair[0], pair[1]);
            if l.left_wall && !r.left_wall {
                let width = r.x - l.x;
                if width > 0 && viol(width as f64) {
                    let cx = (l.x as f64 + r.x as f64) * 0.5;
                    let cy = (yb as f64 + yb1 as f64) * 0.5;
                    if core.contains(cx, cy) && in_mask(cx, cy) {
                        push_edge(l.x as f64, yb as f64, l.x as f64, yb1 as f64, width as f64);
                        push_edge(r.x as f64, yb as f64, r.x as f64, yb1 as f64, width as f64);
                    }
                }
            }
        }
    }

    // Vertical widths: scan x bands, pair horizontal edges across y.
    let x_events = sorted_unique(hedges.iter().flat_map(|e| [e.xlo, e.xhi]).collect());
    for w in x_events.windows(2) {
        let (xb, xb1) = (w[0], w[1]);
        if xb1 <= xb { continue; }
        let mut active: Vec<&HEdge> = hedges.iter().filter(|e| e.xlo <= xb && e.xhi >= xb1).collect();
        active.sort_unstable_by_key(|e| (e.y, e.bottom_wall));
        for pair in active.windows(2) {
            let (b, t) = (pair[0], pair[1]);
            if b.bottom_wall && !t.bottom_wall {
                let height = t.y - b.y;
                if height > 0 && viol(height as f64) {
                    let cx = (xb as f64 + xb1 as f64) * 0.5;
                    let cy = (b.y as f64 + t.y as f64) * 0.5;
                    if core.contains(cx, cy) && in_mask(cx, cy) {
                        push_edge(xb as f64, b.y as f64, xb1 as f64, b.y as f64, height as f64);
                        push_edge(xb as f64, t.y as f64, xb1 as f64, t.y as f64, height as f64);
                    }
                }
            }
        }
    }
    } // end !oblique_only

    oblique_widths(&oedges, core, &mut push_edge, viol, min_run);
    out
}

/// Oblique widths: anti-parallel edge pairs with metal between them.  A pair is only
/// reported when the parallel run (`hi - lo`) exceeds `min_run` DBU — small chamfers
/// are ignored, and a 45°-bent-width rule can require a minimum bent length.
fn oblique_widths(
    oedges: &[OEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    viol: impl Fn(f64) -> bool,
    min_run: f64,
) {
    let n = oedges.len();
    for i in 0..n {
        let ei = &oedges[i];
        let (dix, diy) = ((ei.bx - ei.ax) as f64, (ei.by - ei.ay) as f64);
        let li = dix.hypot(diy);
        if li == 0.0 { continue; }
        let (ux, uy) = (dix / li, diy / li);
        let (nx, ny) = (-diy / li, dix / li);
        for ej in &oedges[i + 1..] {
            let (djx, djy) = ((ej.bx - ej.ax) as f64, (ej.by - ej.ay) as f64);
            if (dix * djy - diy * djx).abs() > 1e-6 || (dix * djx + diy * djy) >= 0.0 {
                continue;
            }
            let dist = (ej.ax - ei.ax) as f64 * nx + (ej.ay - ei.ay) as f64 * ny;
            if dist <= 0.5 || !viol(dist) {
                continue;
            }
            let taj = (ej.ax - ei.ax) as f64 * ux + (ej.ay - ei.ay) as f64 * uy;
            let tbj = (ej.bx - ei.ax) as f64 * ux + (ej.by - ei.ay) as f64 * uy;
            let lo = taj.min(tbj).max(0.0);
            let hi = taj.max(tbj).min(li);
            if hi - lo <= min_run { continue; }
            let mid = (lo + hi) * 0.5;
            let mx = ei.ax as f64 + mid * ux + nx * dist * 0.5;
            let my = ei.ay as f64 + mid * uy + ny * dist * 0.5;
            if !core.contains(mx, my) { continue; }
            push_edge(
                ei.ax as f64 + lo * ux, ei.ay as f64 + lo * uy,
                ei.ax as f64 + hi * ux, ei.ay as f64 + hi * uy,
                dist,
            );
            let span = tbj - taj;
            let (f_lo, f_hi) = ((lo - taj) / span, (hi - taj) / span);
            push_edge(
                ej.ax as f64 + f_lo * djx, ej.ay as f64 + f_lo * djy,
                ej.ax as f64 + f_hi * djx, ej.ay as f64 + f_hi * djy,
                dist,
            );
        }
    }
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
                    x0: tx as i64 * tile, y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
                };
                polys
                    .iter()
                    .flat_map(move |poly| scan_widths(
                        poly, core, dbu_to_um, rid, label, lname, limit, cmp,
                        viol, oblique_only, min_run_dbu, None,
                    ))
                    .collect::<Vec<_>>()
                    .into_iter()
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
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
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
                    p, core, dbu_to_um, rid, "Minimum gate-length violation",
                    pname, limit, "<", viol, false, 0.5, Some(&mps),
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

/// Distance from `p` to segment `a-b`, plus the closest point on the segment.
fn point_to_segment_closest(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> (f64, f64, f64) {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return ((px - ax).hypot(py - ay), ax, ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    let (qx, qy) = (ax + t * dx, ay + t * dy);
    ((px - qx).hypot(py - qy), qx, qy)
}

#[inline]
fn cross2(px: f64, py: f64, qx: f64, qy: f64, rx: f64, ry: f64) -> f64 {
    (qx - px) * (ry - py) - (qy - py) * (rx - px)
}

#[allow(clippy::too_many_arguments)]
fn segments_intersect(
    ax: f64, ay: f64, bx: f64, by: f64,
    cx: f64, cy: f64, dx: f64, dy: f64,
) -> bool {
    let d1 = cross2(cx, cy, dx, dy, ax, ay);
    let d2 = cross2(cx, cy, dx, dy, bx, by);
    let d3 = cross2(ax, ay, bx, by, cx, cy);
    let d4 = cross2(ax, ay, bx, by, dx, dy);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// Closest distance between segments `a-b` and `c-d`, plus the closest point on
/// each (first on `a-b`, second on `c-d`) — used to draw the spacing marker across
/// the gap rather than along one region's edge.
#[allow(clippy::too_many_arguments)]
pub(crate) fn segment_closest_points(
    ax: f64, ay: f64, bx: f64, by: f64,
    cx: f64, cy: f64, dx: f64, dy: f64,
) -> (f64, (f64, f64), (f64, f64)) {
    if segments_intersect(ax, ay, bx, by, cx, cy, dx, dy) {
        return (0.0, (ax, ay), (ax, ay));
    }
    let (d1, q1x, q1y) = point_to_segment_closest(ax, ay, cx, cy, dx, dy);
    let (d2, q2x, q2y) = point_to_segment_closest(bx, by, cx, cy, dx, dy);
    let (d3, p3x, p3y) = point_to_segment_closest(cx, cy, ax, ay, bx, by);
    let (d4, p4x, p4y) = point_to_segment_closest(dx, dy, ax, ay, bx, by);
    let mut best = (d1, (ax, ay), (q1x, q1y));
    if d2 < best.0 { best = (d2, (bx, by), (q2x, q2y)); }
    if d3 < best.0 { best = (d3, (p3x, p3y), (cx, cy)); }
    if d4 < best.0 { best = (d4, (p4x, p4y), (dx, dy)); }
    best
}

/// Even-odd ray-casting point-in-polygon test.
pub(crate) fn point_in_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
    let n = pts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// True if either region has a vertex strictly inside the other (containment or
/// positive-gap overlap).  Merged same-layer regions never overlap; for two layers
/// an overlap is allowed (not a spacing violation), so such pairs are skipped.
/// Hole-aware: a region sitting inside the other's *hole* (e.g. an iso-PWell Activ
/// inside its NWell isolation ring) does not overlap it — its spacing to the hole
/// boundary is a real, checkable gap (nmosi.c).
fn overlapping(a: &Poly, b: &Poly) -> bool {
    a.vertices().any(|&(x, y)| b.contains_point(x, y))
        || b.vertices().any(|&(x, y)| a.contains_point(x, y))
}

#[derive(Clone, Copy)]
pub struct BBox { xmin: f64, ymin: f64, xmax: f64, ymax: f64 }

impl BBox {
    fn from_pts(pts: &[(f64, f64)]) -> Option<Self> {
        let mut xmin = f64::INFINITY;
        let mut ymin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        for &(x, y) in pts {
            xmin = xmin.min(x);
            ymin = ymin.min(y);
            xmax = xmax.max(x);
            ymax = ymax.max(y);
        }
        if xmin == f64::INFINITY { None } else { Some(BBox { xmin, ymin, xmax, ymax }) }
    }

    /// True if the boxes could be within `threshold` (L∞ lower bound).
    fn possibly_within(&self, other: &BBox, threshold: f64) -> bool {
        let gap_x = (self.xmin - other.xmax).max(other.xmin - self.xmax).max(0.0);
        let gap_y = (self.ymin - other.ymax).max(other.ymin - self.ymax).max(0.0);
        gap_x < threshold && gap_y < threshold
    }

    /// Largest side margin by which `self` (the enclosing box) extends beyond
    /// `inner` — the best-enclosed side, used for the endcap rule.  Negative if
    /// `inner` sticks out on every side.
    pub fn max_side_margin(&self, inner: &BBox) -> f64 {
        (inner.xmin - self.xmin) // left
            .max(self.xmax - inner.xmax) // right
            .max(inner.ymin - self.ymin) // bottom
            .max(self.ymax - inner.ymax) // top
    }
}

/// A merged region's outer contour, in µm, prepared for distance queries.
pub struct Poly {
    pts: Vec<(f64, f64)>,
    /// Hole contours (CW, material on the left — same convention as `MergedPoly`).
    holes: Vec<Vec<(f64, f64)>>,
    pub bbox: BBox,
    /// Outer *and* hole edges: both are real region boundary (spacing, width and
    /// enclosure are all measured against holes too).
    edges: Vec<(f64, f64, f64, f64)>,
}

impl Poly {
    /// All contour vertices, outer ring and holes.
    fn vertices(&self) -> impl Iterator<Item = &(f64, f64)> {
        self.pts.iter().chain(self.holes.iter().flatten())
    }

    /// Point strictly inside the region: inside the outer ring and in no hole.
    fn contains_point(&self, x: f64, y: f64) -> bool {
        point_in_polygon(x, y, &self.pts) && !self.holes.iter().any(|h| point_in_polygon(x, y, h))
    }

    /// True if any 45°/angled edge of this region lies within `max_gap` of `other`.
    /// The bend must be near the spacing being checked, not on a distant Manhattan part
    /// of the same net — otherwise one diagonal anywhere would bump every spacing of the
    /// whole polygon to the wider value.
    pub fn has_diagonal_near(&self, other: &Poly, max_gap: f64) -> bool {
        for &(ax, ay, bx, by) in &self.edges {
            if ax == bx || ay == by {
                continue; // axis-aligned edge
            }
            for &(cx, cy, dx, dy) in &other.edges {
                if segment_closest_points(ax, ay, bx, by, cx, cy, dx, dy).0 < max_gap {
                    return true;
                }
            }
        }
        false
    }

    /// True parallel-run length between two regions: the longest projected overlap of
    /// a pair of anti-parallel facing edges (one from each region) whose perpendicular
    /// separation is below `max_gap`.
    ///
    /// A bounding-box overlap is exact only for plain rectangles; for an L-shaped,
    /// stepped or comb-like pad (common in IO cells) the boxes can overlap for tens of
    /// microns while the metal only truly runs alongside its neighbour for a fraction
    /// of that — which otherwise yields false parallel-run-spacing violations.
    ///
    /// `min_run` is the parallel-run threshold and `wide_width` the "wide line" width;
    /// returns true when some facing-edge pair within `max_gap` overlaps for more than
    /// `min_run` **and** at least one of the two lines is wider than `wide_width` there.
    /// Line width is the metal depth behind the facing edge (see [`Self::edge_depth`]),
    /// not the bounding-box dimension — an L-shaped narrow trace has a wide box but a
    /// narrow line, and must not satisfy the "wide" condition.
    pub fn prl_applies(&self, other: &Poly, max_gap: f64, wide_width: f64, min_run: f64) -> bool {
        let mut depth_a: Vec<Option<f64>> = vec![None; self.edges.len()];
        let mut depth_b: Vec<Option<f64>> = vec![None; other.edges.len()];
        for (i, &(ax, ay, bx, by)) in self.edges.iter().enumerate() {
            let (dx, dy) = (bx - ax, by - ay);
            let len = dx.hypot(dy);
            if len == 0.0 {
                continue;
            }
            let (ux, uy) = (dx / len, dy / len); // unit along this edge
            let (nx, ny) = (-uy, ux); // unit normal
            for (j, &(cx, cy, ex, ey)) in other.edges.iter().enumerate() {
                let (fx, fy) = (ex - cx, ey - cy);
                // Facing edges run in opposite directions and are collinear in angle.
                if dx * fx + dy * fy >= 0.0 {
                    continue;
                }
                let flen = fx.hypot(fy);
                if flen == 0.0 || (dx * fy - dy * fx).abs() > 1e-6 * len * flen {
                    continue;
                }
                // Perpendicular separation of the two parallel lines (the gap).
                let perp = ((cx - ax) * nx + (cy - ay) * ny).abs();
                if perp <= 0.0 || perp >= max_gap {
                    continue;
                }
                // Overlap of the two edges projected onto this edge's direction.
                let (tc0, tc1) = (
                    (cx - ax) * ux + (cy - ay) * uy,
                    (ex - ax) * ux + (ey - ay) * uy,
                );
                let run = (tc0.max(tc1).min(len)) - (tc0.min(tc1).max(0.0));
                if run <= min_run {
                    continue;
                }
                let da = *depth_a[i].get_or_insert_with(|| self.edge_depth(i));
                let db = *depth_b[j].get_or_insert_with(|| other.edge_depth(j));
                if da > wide_width || db > wide_width {
                    return true;
                }
            }
        }
        false
    }

    /// Metal depth behind contour edge `i`: the perpendicular distance, measured along
    /// the inward normal, to the nearest anti-parallel edge of this same region that
    /// overlaps edge `i` in projection.  This is the local line width at that edge — a
    /// thin trace reads narrow here even where its bounding box is large.
    fn edge_depth(&self, i: usize) -> f64 {
        let (ax, ay, bx, by) = self.edges[i];
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy);
        if len == 0.0 {
            return f64::INFINITY;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux); // inward normal (outer contour is CCW)
        let mut best = f64::INFINITY;
        for (j, &(cx, cy, ex, ey)) in self.edges.iter().enumerate() {
            if j == i {
                continue;
            }
            let (fx, fy) = (ex - cx, ey - cy);
            if dx * fx + dy * fy >= 0.0 {
                continue;
            }
            let flen = fx.hypot(fy);
            if flen == 0.0 || (dx * fy - dy * fx).abs() > 1e-6 * len * flen {
                continue;
            }
            let perp = (cx - ax) * nx + (cy - ay) * ny; // signed inward distance
            if perp <= 0.0 {
                continue;
            }
            let (tc0, tc1) = (
                (cx - ax) * ux + (cy - ay) * uy,
                (ex - ax) * ux + (ey - ay) * uy,
            );
            if tc0.max(tc1).min(len) - tc0.min(tc1).max(0.0) <= 0.0 {
                continue;
            }
            best = best.min(perp);
        }
        best
    }
}

fn poly_from_merged(m: &MergedPoly, dbu_to_um: f64) -> Option<Poly> {
    let scale = |ring: &[i_overlay::i_float::int::point::IntPoint]| -> Vec<(f64, f64)> {
        ring.iter().map(|p| (p.x as f64 * dbu_to_um, p.y as f64 * dbu_to_um)).collect()
    };
    let ring_edges = |pts: &[(f64, f64)], edges: &mut Vec<(f64, f64, f64, f64)>| {
        let n = pts.len();
        for i in 0..n {
            let (ax, ay) = pts[i];
            let (bx, by) = pts[(i + 1) % n];
            if ax != bx || ay != by {
                edges.push((ax, ay, bx, by));
            }
        }
    };
    let pts = scale(&m.outer);
    if pts.len() < 3 {
        return None;
    }
    let bbox = BBox::from_pts(&pts)?;
    let mut edges = Vec::new();
    ring_edges(&pts, &mut edges);
    let holes: Vec<Vec<(f64, f64)>> =
        m.holes.iter().map(|h| scale(h)).filter(|h| h.len() >= 3).collect();
    for h in &holes {
        ring_edges(h, &mut edges);
    }
    Some(Poly { pts, holes, bbox, edges })
}

/// Closest edge-to-edge distance between two regions, with the closest point on
/// each (first on `a`, second on `b`) so the marker can span the gap.  Stops early
/// once a touching pair is found (`< half_dbu`).
fn closest(a: &Poly, b: &Poly, half_dbu: f64) -> (f64, (f64, f64), (f64, f64)) {
    let mut min_dist = f64::INFINITY;
    let mut pa = (0.0, 0.0);
    let mut pb = (0.0, 0.0);
    'outer: for &(ax, ay, bx, by) in &a.edges {
        for &(cx, cy, dx, dy) in &b.edges {
            let (d, qa, qb) = segment_closest_points(ax, ay, bx, by, cx, cy, dx, dy);
            if d < min_dist {
                min_dist = d;
                pa = qa;
                pb = qb;
                if min_dist < half_dbu {
                    break 'outer;
                }
            }
        }
    }
    (min_dist, pa, pb)
}

#[allow(clippy::too_many_arguments)]
fn check_tile<G: Fn(&Poly, &Poly) -> bool>(
    a_polys: &[MergedPoly],
    b_polys: &[MergedPoly],
    same_layer: bool,
    core: Core,
    value: f64,
    dbu_to_um: f64,
    rule_id: &str,
    name_a: &str,
    name_b: &str,
    gate: &G,
) -> Vec<Violation> {
    let half = dbu_to_um * 0.5;
    let pa: Vec<Poly> = a_polys.iter().filter_map(|m| poly_from_merged(m, dbu_to_um)).collect();
    let pb: Vec<Poly> = if same_layer {
        Vec::new()
    } else {
        b_polys.iter().filter_map(|m| poly_from_merged(m, dbu_to_um)).collect()
    };
    let bs: &[Poly] = if same_layer { &pa } else { &pb };

    let mut out = Vec::new();
    for (i, a) in pa.iter().enumerate() {
        for (j, b) in bs.iter().enumerate() {
            if same_layer && j <= i {
                continue;
            }
            if !a.bbox.possibly_within(&b.bbox, value) {
                continue;
            }
            if overlapping(a, b) {
                continue;
            }
            let (min_dist, (ax, ay), (bx, by)) = closest(a, b, half);
            // Touching: shapes share a boundary -> no spacing.
            if min_dist < half {
                continue;
            }
            if min_dist < value - half {
                if !gate(a, b) {
                    continue;
                }
                // Own the violation by the gap midpoint; mark the gap itself.
                let mx = (ax + bx) * 0.5;
                let my = (ay + by) * 0.5;
                if !core.contains(mx / dbu_to_um, my / dbu_to_um) {
                    continue;
                }
                out.push(Violation::edge(
                    rule_id,
                    "Minimum space violation",
                    format!(
                        "space {:.4} µm < {:.2} µm between {} and {} at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
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
/// reported only if `gate(a, b)` holds, letting conditional spacing rules add
/// width / parallel-run conditions without duplicating the merge, tiling and
/// edge-distance work.
pub fn run_gated<G: Fn(&Poly, &Poly) -> bool + Sync>(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
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
    let tile = merged.tile_dbu() as i64;
    let rid = rule.id.as_str();
    let name_a = layer_a.name.as_str();
    let name_b = layer_b.name.as_str();
    let map_a = merged.tiles(al, ad);
    let map_b = if same_layer { map_a } else { merged.tiles(bl, bd) };

    // Every spacing violation has an `a`-region within the halo of the tile that
    // owns its gap, so that tile is an `a` key — iterating `a`'s tiles covers all
    // pairs, and the core filter deduplicates.
    let keys: Vec<(i32, i32)> = map_a.keys().copied().collect();
    let empty: Vec<MergedPoly> = Vec::new();

    keys.par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let core = Core {
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
            };
            let a_polys = &map_a[&(tx, ty)];
            let b_polys = map_b.get(&(tx, ty)).unwrap_or(&empty);
            check_tile(a_polys, b_polys, same_layer, core, value, dbu_to_um, rid, name_a, name_b, &gate)
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
    let maps: Vec<&crate::merge::TileMap> = keys_l.iter().map(|&(l, d)| merged.tiles(l, d)).collect();

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
        VirtualOp::Union
        | VirtualOp::Square
        | VirtualOp::NotSquare
        | VirtualOp::Close(_)
        | VirtualOp::Open(_)
        | VirtualOp::Grow(_)
        | VirtualOp::Interacting
        | VirtualOp::NotInteracting
        | VirtualOp::Covering
        | VirtualOp::NotCircleOrOctagon
        | VirtualOp::NotCircle
        | VirtualOp::Holes
        | VirtualOp::WithHoles
        | VirtualOp::WithText => maps
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
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
            };
            let sources: Vec<&[MergedPoly]> = maps
                .iter()
                .map(|m| m.get(&(tx, ty)).map(Vec::as_slice).unwrap_or(&[]))
                .collect();
            compose_tile(op, &sources)
                .into_iter()
                .filter_map(move |m| {
                    let (cx, cy) = merged_centroid_dbu(&m);
                    if !core.contains(cx, cy) {
                        return None;
                    }
                    let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
                    Some(Violation::point(
                        rid,
                        label,
                        format!("{descr} at ({ux:.4}, {uy:.4}) µm"),
                        ux, uy,
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
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
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
                let Some(a) = poly_from_merged(tm, dbu_to_um) else { continue };
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
                        if !core.contains(mx / dbu_to_um, my / dbu_to_um) {
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
                            x1, y1, x2, y2,
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

/// Absolute area (DBU²) and centroid (DBU) of a closed contour, either winding.
fn ring_area_centroid(c: &[IntPoint]) -> (f64, f64, f64) {
    let n = c.len();
    let (mut sum, mut cx, mut cy) = (0.0_f64, 0.0_f64, 0.0_f64);
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        let (xi, yi) = (c[i].x as f64, c[i].y as f64);
        let (xj, yj) = (c[j].x as f64, c[j].y as f64);
        let cross = xi * yj - xj * yi;
        sum += cross;
        cx += (xi + xj) * cross;
        cy += (yi + yj) * cross;
    }
    let area = sum.abs() / 2.0;
    if sum.abs() < 1e-9 {
        let (sx, sy) = c.iter().fold((0.0, 0.0), |(sx, sy), p| (sx + p.x as f64, sy + p.y as f64));
        let m = n.max(1) as f64;
        return (area, sx / m, sy / m);
    }
    (area, cx / (3.0 * sum), cy / (3.0 * sum))
}

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
                        if area >= value || !core.contains(cx, cy) {
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

    match forbidden {
        Some(a) => println!("[{}] Checking no_angle: {} edges at {:.1}°", rule.id, layer.name, a),
        None => println!("[{}] Checking no_angle: non-orthogonal {} edges", rule.id, layer.name),
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
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
            };
            let mut out = Vec::new();
            for pm in ps {
                let Some(p) = poly_from_merged(pm, dbu_to_um) else { continue };
                for &(ax, ay, bx, by) in &p.edges {
                    let (dx, dy) = (bx - ax, by - ay);
                    if dx == 0.0 && dy == 0.0 {
                        continue;
                    }
                    let ang = dy.atan2(dx).to_degrees().rem_euclid(180.0);
                    let near = |a: f64, b: f64| (a - b).abs() <= tol || (a - b).abs() >= 180.0 - tol;
                    let flag = match target {
                        Some(t) => near(ang, t),
                        None => !(near(ang, 0.0) || near(ang, 90.0)),
                    };
                    if !flag {
                        continue;
                    }
                    let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                    if !core.contains(mx / dbu_to_um, my / dbu_to_um) {
                        continue;
                    }
                    out.push(Violation::edge(
                        rid,
                        "Forbidden angle violation",
                        format!(
                            "{ln}: forbidden ({ang:.1}°) edge at \
                             ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"
                        ),
                        ax, ay, bx, by,
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

/// Drive a bounding-box extent check over the cached tiles.  One point violation per
/// offending region (owned by the tile whose core holds its centroid).
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

    merged
        .tiles(gl, gd)
        .par_iter()
        .flat_map_iter(move |(&(tx, ty), polys)| {
            let core = Core {
                x0: tx as i64 * tile, y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile, y1: (ty as i64 + 1) * tile,
            };
            polys
                .iter()
                .filter_map(move |m| {
                    let (mut x0, mut y0) = (i32::MAX, i32::MAX);
                    let (mut x1, mut y1) = (i32::MIN, i32::MIN);
                    for p in &m.outer {
                        x0 = x0.min(p.x); y0 = y0.min(p.y);
                        x1 = x1.max(p.x); y1 = y1.max(p.y);
                    }
                    let (w, h) = ((x1 - x0) as f64, (y1 - y0) as f64);
                    let extent = if long { w.max(h) } else { w.min(h) };
                    if !viol(extent) {
                        return None;
                    }
                    let (cx, cy) = merged_centroid_dbu(m);
                    if !core.contains(cx, cy) {
                        return None;
                    }
                    let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
                    Some(Violation::point(
                        rid,
                        label,
                        format!(
                            "{}: {} {:.4} µm {} {:.4} µm at ({:.4}, {:.4}) µm",
                            lname, word, extent * dbu_to_um, cmp, limit, ux, uy
                        ),
                        ux, uy,
                    ))
                })
                .collect::<Vec<_>>()
                .into_iter()
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

fn point_to_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    point_to_segment_closest(px, py, ax, ay, bx, by).0
}

/// Vertex inside the outer region or on its boundary (within `tol`) — the boundary
/// case lets value-0 rules pass when the inner shape touches the outer edge.
/// Hole-aware: a vertex inside the outer region's hole is *not* inside (its edges,
/// which include the hole contours, still grant the on-boundary tolerance).
fn vertex_inside_or_on(px: f64, py: f64, outer: &Poly, tol: f64) -> bool {
    outer.contains_point(px, py)
        || outer.edges.iter().any(|&(ax, ay, bx, by)| point_to_segment_dist(px, py, ax, ay, bx, by) <= tol)
}

fn all_vertices_inside(inner: &Poly, outer: &Poly, tol: f64) -> bool {
    inner.vertices().all(|&(x, y)| vertex_inside_or_on(x, y, outer, tol))
}

/// Whether two polygons overlap (one has a vertex inside the other).  A cheap bbox prefilter
/// guards the point-in-polygon tests; for the via-over-device case it cannot miss (a via
/// crossing the device edge has vertices inside it).
fn polys_interact(a: &Poly, b: &Poly) -> bool {
    // Overlapping boxes have a clamped gap of exactly 0.0, so the threshold must be
    // positive — with 0.0 the prefilter rejects every pair (`gap < 0.0` never holds).
    if !a.bbox.possibly_within(&b.bbox, f64::MIN_POSITIVE) {
        return false;
    }
    a.vertices().any(|&(x, y)| b.contains_point(x, y))
        || b.vertices().any(|&(x, y)| a.contains_point(x, y))
}

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

/// One candidate enclosure violation: a facing inner/outer edge pair below the rule
/// value, with a probe point just beyond the measured outer wall for the reality check.
struct EnclosurePair {
    dist: f64,
    edge: (f64, f64, f64, f64),
    probe: (f64, f64),
}

/// Facing-pair scan of `inner` against one containing `outer` candidate (see
/// the module comment above on the projection metric and coincidence semantics).
/// Returns the pairs with `dist < cutoff` plus whether any coincident segment was seen.
fn enclosure_pairs(
    inner: &Poly,
    outer: &Poly,
    cutoff: f64,
    skip_coincident: bool,
    tol: f64,
) -> (Vec<EnclosurePair>, bool) {
    let mut pairs = Vec::new();
    let mut saw_coincident = false;
    for &(ax, ay, bx, by) in &inner.edges {
        let (dix, diy) = (bx - ax, by - ay);
        let li = dix.hypot(diy);
        if li <= 0.0 {
            continue;
        }
        let (ux, uy) = (dix / li, diy / li);
        // Right-hand normal of a CCW contour points *outward* — toward the enclosing wall.
        let (nx, ny) = (uy, -ux);
        for &(cx, cy, dx, dy) in &outer.edges {
            let (dox, doy) = (dx - cx, dy - cy);
            let lo = dox.hypot(doy);
            if lo <= 0.0 || (dix * doy - diy * dox).abs() > 1e-6 * li * lo {
                continue; // not parallel: no projection pairing
            }
            // Projected overlap of the outer edge onto the inner edge's span.
            let t0 = (cx - ax) * ux + (cy - ay) * uy;
            let t1 = (dx - ax) * ux + (dy - ay) * uy;
            let (s0, s1) = (t0.min(t1).max(0.0), t0.max(t1).min(li));
            if s1 - s0 <= tol {
                continue;
            }
            // Perpendicular offset of the outer edge's line, signed outward.
            let d0 = (cx - ax) * nx + (cy - ay) * ny;
            let dist = if d0.abs() <= tol {
                saw_coincident = true;
                if skip_coincident {
                    continue;
                }
                0.0
            } else if d0 > 0.0 {
                d0
            } else {
                continue; // outer wall on the interior side: not an enclosure margin
            };
            if dist < cutoff {
                let mid = (s0 + s1) * 0.5;
                pairs.push(EnclosurePair {
                    dist,
                    edge: (ax + s0 * ux, ay + s0 * uy, ax + s1 * ux, ay + s1 * uy),
                    probe: (
                        ax + mid * ux + nx * (dist + 2.0 * tol),
                        ay + mid * uy + ny * (dist + 2.0 * tol),
                    ),
                });
            }
        }
    }
    (pairs, saw_coincident)
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
    map.get(&(tx, ty))
        .is_some_and(|polys| polys.iter().any(|m| crate::merge::point_in_merged(px, py, m)))
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
    endcap: bool,
) -> Vec<Violation> {
    let enclosing_layer = &rule.layers[0];
    let enclosed_layer = &rule.layers[1];
    let (al, ad) = (enclosing_layer.gds_layer as i16, enclosing_layer.gds_datatype as i16);
    let (bl, bd) = (enclosed_layer.gds_layer as i16, enclosed_layer.gds_datatype as i16);

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
    let interacting_only = rule.params.get("interacting_only").is_some_and(|v| *v != 0.0);
    // Ignore inner edges coincident with the enclosing contour (clip artifacts of an
    // `intersection`-derived enclosed layer) — mirrors KLayout's `consider_intersecting_
    // edges: false` / `without_distance(0)` rule flags.  Off by default: a genuinely flush
    // edge is a real 0-margin violation (e.g. Rppd.b).
    let skip_coincident = rule.params.get("skip_coincident").is_some_and(|v| *v != 0.0);
    // Stronger, region-level variant: skip the *whole* enclosed region if any of its edges
    // is coincident with the enclosing contour — i.e. the region reaches the boundary and
    // is not "surrounded entirely by" the enclosing layer.  NW.e's title says exactly that:
    // a tie crossing the NWell edge is external-tie territory (NW.d), not NW.e's.  This is
    // also halo-robust: the clip is a local property of the region, unlike its remaining
    // margins whose tile ownership can shift with per-suite halos.
    let skip_clipped = rule.params.get("skip_clipped").is_some_and(|v| *v != 0.0);

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
                if !core.contains(cxd, cyd) {
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
                    let (dist, edge) = if endcap {
                        enclosure_dist_endcap(&bp, a)
                    } else {
                        let (pairs, coincident) =
                            enclosure_pairs(&bp, a, value, skip_coincident, tol);
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
                            let (pairs, _) = enclosure_pairs(&bp, a, value, skip_coincident, tol);
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
    let enclosing_layer = &rule.layers[0];
    let enclosed_layer = &rule.layers[1];
    let (al, ad) = (enclosing_layer.gds_layer as i16, enclosing_layer.gds_datatype as i16);
    let (bl, bd) = (enclosed_layer.gds_layer as i16, enclosed_layer.gds_datatype as i16);

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
                if !core.contains(cxd, cyd) {
                    continue;
                }
                let Some(bp) = poly_from_merged(bm, dbu_to_um) else { continue };

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
                    let (pairs, _) = enclosure_pairs(&bp, a, f64::INFINITY, false, tol);
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
                        x1, y1, x2, y2,
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

    fn pt(x: i32, y: i32) -> IntPoint { IntPoint::new(x, y) }

    fn core() -> Core {
        Core { x0: -1_000_000, y0: -1_000_000, x1: 1_000_000, y1: 1_000_000 }
    }

    /// Thin 45° trace (~99 DBU walls) flagged by a `< 160` (min-width) predicate:
    /// both walls reported, nothing from the orthogonal end-caps.
    #[test]
    fn oblique_45_thin_trace_flags_both_walls() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(1000, 1000), pt(1000, 1140), pt(0, 140)],
            holes: vec![],
        };
        let v = scan_widths(&poly, core(), 0.001, "T", "min", "L", 0.16, "<", |w| w < 160.0 - 0.5, false, 0.5, None);
        assert_eq!(v.len(), 2, "got {}", v.len());
    }

    #[test]
    fn oblique_45_wide_trace_is_clean() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(1000, 1000), pt(1000, 2400), pt(0, 1400)],
            holes: vec![],
        };
        let v = scan_widths(&poly, core(), 0.001, "T", "min", "L", 0.16, "<", |w| w < 160.0 - 0.5, false, 0.5, None);
        assert!(v.is_empty(), "got {}", v.len());
    }

    /// A 200×200 DBU square flagged by a `> 150` (max-width) predicate: both
    /// dimensions exceed, two walls each → 4.
    #[test]
    fn max_width_square_flags_four_walls() {
        let poly = MergedPoly {
            outer: vec![pt(0, 0), pt(200, 0), pt(200, 200), pt(0, 200)],
            holes: vec![],
        };
        let v = scan_widths(&poly, core(), 0.001, "T", "max", "L", 0.15, ">", |w| w > 150.0 + 0.5, false, 0.5, None);
        assert_eq!(v.len(), 4, "got {}", v.len());
    }
}
