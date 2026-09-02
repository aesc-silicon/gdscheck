// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Geometry the rest of the crate measures with.
//!
//! This is the measurement library, kept apart from both the merge engine and the checks
//! because both need it. A rule reads a measurement and formats violations; an edge or
//! virtual *layer* reads the same measurement and keeps the geometry. Neither is the
//! owner, so the scans live here and both call in.
//!
//! What belongs here is anything that answers a question about shapes — how wide, how far
//! apart, how far enclosed — and nothing that knows what a rule or a violation is.

use crate::merge::{Core, MergedPoly};

/// How far past its own limit a maximum looks for the facing wall, in multiples of the
/// limit.  A width that exceeds the limit by more than this is not measured and so not
/// reported: the check would rather miss one than invent a width to a wall that is not
/// really opposite.
pub const MAX_REACH: f64 = 4.0;

/// The same idea for an *enclosure* maximum, and tighter, because the two measurements
/// look for different things.  A width's two walls bound the same material, so a wide
/// shape really does have its opposite wall far off and the search has to reach for it.
/// An enclosure's margin is bounded by the outer wall nearest the inner one; past a small
/// multiple of the bound what the search finds is the far side of the enclosing shape,
/// which never bounded this margin at all.  On GF180's MDN.10c the width's own four
/// invents eighteen markers where two invents none.
pub const ENCLOSURE_MAX_REACH: f64 = 2.0;
use i_overlay::i_float::int::point::IntPoint;

// ===========================================================================
// Facing-wall measurement
// ===========================================================================
// The width scan lives here rather than with the checks because it is geometry, not
// reporting, and both a rule and an *edge layer* are built from it: a rule formats the
// pairs into violations, a layer keeps them as segments.  `merge` sits below `checks`, so
// a layer built from a measurement can only reach the scan if the scan is here.

/// Vertical edge: `left_wall` true ⇒ metal to its right (edge directed down).
pub struct VEdge {
    pub x: i32,
    pub ylo: i32,
    pub yhi: i32,
    pub left_wall: bool,
}
/// Horizontal edge: `bottom_wall` true ⇒ metal above it (edge directed right).
pub struct HEdge {
    pub y: i32,
    pub xlo: i32,
    pub xhi: i32,
    pub bottom_wall: bool,
}
/// An oblique directed edge `a → b`; metal is on its left.
pub struct OEdge {
    pub ax: i32,
    pub ay: i32,
    pub bx: i32,
    pub by: i32,
}

/// Split a merged region's contours (outer + holes) into axis-aligned and oblique
/// directed edges — the shared input of the width scan and its notch dual.
pub fn collect_edges(
    poly: &MergedPoly,
    vedges: &mut Vec<VEdge>,
    hedges: &mut Vec<HEdge>,
    oedges: &mut Vec<OEdge>,
) {
    let mut add = |contour: &[IntPoint]| {
        let n = contour.len();
        if n < 3 {
            return;
        }
        for i in 0..n {
            let a = contour[i];
            let b = contour[if i + 1 == n { 0 } else { i + 1 }];
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            if dx == 0 && dy != 0 {
                vedges.push(VEdge {
                    x: a.x,
                    ylo: a.y.min(b.y),
                    yhi: a.y.max(b.y),
                    left_wall: dy < 0,
                });
            } else if dy == 0 && dx != 0 {
                hedges.push(HEdge {
                    y: a.y,
                    xlo: a.x.min(b.x),
                    xhi: a.x.max(b.x),
                    bottom_wall: dx > 0,
                });
            } else if dx != 0 && dy != 0 {
                oedges.push(OEdge {
                    ax: a.x,
                    ay: a.y,
                    bx: b.x,
                    by: b.y,
                });
            }
        }
    };
    add(&poly.outer);
    for h in &poly.holes {
        add(h);
    }
}

pub fn sorted_unique(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Find facing-wall widths in one merged region and report both walls of any
/// width for which `viol(width_dbu)` holds.
#[allow(clippy::too_many_arguments)]
/// Every facing-wall pair of `poly` whose span satisfies `viol`, as
/// `(x1, y1, x2, y2, width)` in DBU - two entries per pair, one for each wall.
///
/// This is the measurement without the reporting: the width checks format these into
/// violations, and an edge layer built from a width can take the same pairs as geometry.
/// The mask is a predicate rather than a region list so that the scan needs no notion of
/// what a `Poly` is.
pub fn width_pairs(
    poly: &MergedPoly,
    core: Core,
    viol: impl Fn(f64) -> bool,
    in_mask: impl Fn(f64, f64) -> bool,
    oblique_only: bool,
    mixed: bool,
    min_run: f64,
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut vedges = Vec::new();
    let mut hedges = Vec::new();
    let mut oedges = Vec::new();
    collect_edges(poly, &mut vedges, &mut hedges, &mut oedges);

    let mut out = Vec::new();
    let mut push_edge = |x1: f64, y1: f64, x2: f64, y2: f64, w_dbu: f64| {
        out.push((x1, y1, x2, y2, w_dbu));
    };
    // Rectilinear widths (skipped for oblique-only rules such as a 45° width check).
    if !oblique_only {
        // Horizontal widths: scan y bands, pair vertical edges across x.
        let y_events = sorted_unique(vedges.iter().flat_map(|e| [e.ylo, e.yhi]).collect());
        for w in y_events.windows(2) {
            let (yb, yb1) = (w[0], w[1]);
            if yb1 <= yb {
                continue;
            }
            let mut active: Vec<&VEdge> = vedges
                .iter()
                .filter(|e| e.ylo <= yb && e.yhi >= yb1)
                .collect();
            active.sort_unstable_by_key(|e| (e.x, e.left_wall));
            for pair in active.windows(2) {
                let (l, r) = (pair[0], pair[1]);
                if l.left_wall && !r.left_wall {
                    let width = r.x - l.x;
                    if width > 0 && viol(width as f64) {
                        let cx = (l.x as f64 + r.x as f64) * 0.5;
                        let cy = (yb as f64 + yb1 as f64) * 0.5;
                        if core.owns(cx, cy) && in_mask(cx, cy) {
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
            if xb1 <= xb {
                continue;
            }
            let mut active: Vec<&HEdge> = hedges
                .iter()
                .filter(|e| e.xlo <= xb && e.xhi >= xb1)
                .collect();
            active.sort_unstable_by_key(|e| (e.y, e.bottom_wall));
            for pair in active.windows(2) {
                let (b, t) = (pair[0], pair[1]);
                if b.bottom_wall && !t.bottom_wall {
                    let height = t.y - b.y;
                    if height > 0 && viol(height as f64) {
                        let cx = (xb as f64 + xb1 as f64) * 0.5;
                        let cy = (b.y as f64 + t.y as f64) * 0.5;
                        if core.owns(cx, cy) && in_mask(cx, cy) {
                            push_edge(xb as f64, b.y as f64, xb1 as f64, b.y as f64, height as f64);
                            push_edge(xb as f64, t.y as f64, xb1 as f64, t.y as f64, height as f64);
                        }
                    }
                }
            }
        }
    } // end !oblique_only

    if mixed {
        mixed_widths(&oedges, &vedges, &hedges, core, &mut push_edge, &viol);
    }
    oblique_widths(&oedges, core, &mut push_edge, viol, min_run);
    out
}

/// Closest points of two segments and the distance between them, all in DBU.  Segments
/// here are polygon edges that never properly cross, so the minimum always sits at an
/// endpoint of one of them projected onto the other (or at a shared endpoint, distance 0).
pub fn seg_seg_closest(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> (f64, (f64, f64), (f64, f64)) {
    // Closest point to `p` on the segment `q0→q1`, clamped to the segment.
    let on = |p: (f64, f64), q0: (f64, f64), q1: (f64, f64)| -> (f64, f64) {
        let (dx, dy) = (q1.0 - q0.0, q1.1 - q0.1);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return q0;
        }
        let t = (((p.0 - q0.0) * dx + (p.1 - q0.1) * dy) / len2).clamp(0.0, 1.0);
        (q0.0 + t * dx, q0.1 + t * dy)
    };
    let mut best = (f64::INFINITY, a0, b0);
    for (p, q, flip) in [
        (a0, on(a0, b0, b1), false),
        (a1, on(a1, b0, b1), false),
        (b0, on(b0, a0, a1), true),
        (b1, on(b1, a0, a1), true),
    ] {
        let d = (p.0 - q.0).hypot(p.1 - q.1);
        if d < best.0 {
            best = if flip { (d, q, p) } else { (d, p, q) };
        }
    }
    best
}

/// Widths bounded by an **oblique edge facing an axis-aligned one** — the chamfered
/// corner of a well against the straight edge opposite it.
///
/// The three passes above each pair edges of one kind: vertical with vertical, horizontal
/// with horizontal, oblique with anti-parallel oblique.  A 45° edge facing a vertical one
/// belongs to none of them, so the narrow strip between a chamfer and the opposite wall
/// was measured by nothing at all — a silent miss, and a common shape, since chamfering
/// is how a layout avoids acute angles in the first place.
///
/// Unlike a parallel pair the separation varies along the edges, so what is measured is
/// the **minimum** distance between the two segments (KLayout's `euclidian` metric) and
/// the marker is the connecting span itself rather than two facing walls.
///
/// A pair only bounds material when each edge's interior lies toward the other, which the
/// two interior normals decide: for an oblique edge `a → b` the interior is to its left,
/// and for an axis-aligned edge it is the side its wall flag names.  Adjacent edges share
/// a vertex and so measure zero, which the `dist <= 0.5` guard drops along with the
/// coincident-edge noise the other passes filter the same way.
fn mixed_widths(
    oedges: &[OEdge],
    vedges: &[VEdge],
    hedges: &[HEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    viol: impl Fn(f64) -> bool,
) {
    for o in oedges {
        let (ax, ay) = (o.ax as f64, o.ay as f64);
        let (bx, by) = (o.bx as f64, o.by as f64);
        let len = (bx - ax).hypot(by - ay);
        if len == 0.0 {
            continue;
        }
        // Interior normal of the oblique edge: metal is on the left of a → b.
        let (nox, noy) = (-(by - ay) / len, (bx - ax) / len);

        // (segment endpoints, interior normal) for every axis-aligned edge.
        let axis = vedges
            .iter()
            .map(|v| {
                let n = if v.left_wall { (1.0, 0.0) } else { (-1.0, 0.0) };
                ((v.x as f64, v.ylo as f64), (v.x as f64, v.yhi as f64), n)
            })
            .chain(hedges.iter().map(|h| {
                let n = if h.bottom_wall {
                    (0.0, 1.0)
                } else {
                    (0.0, -1.0)
                };
                ((h.xlo as f64, h.y as f64), (h.xhi as f64, h.y as f64), n)
            }));

        for (q0, q1, (nax, nay)) in axis {
            let (dist, po, pa) = seg_seg_closest((ax, ay), (bx, by), q0, q1);
            if dist <= 0.5 || !viol(dist) {
                continue;
            }
            // Each edge's interior must face the other, or the gap is outside the shape.
            let (vx, vy) = (pa.0 - po.0, pa.1 - po.1);
            if vx * nox + vy * noy <= 0.0 || -vx * nax - vy * nay <= 0.0 {
                continue;
            }
            let (mx, my) = ((po.0 + pa.0) * 0.5, (po.1 + pa.1) * 0.5);
            if !core.owns(mx, my) {
                continue;
            }
            push_edge(po.0, po.1, pa.0, pa.1, dist);
        }
    }
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
        if li == 0.0 {
            continue;
        }
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
            if hi - lo <= min_run {
                continue;
            }
            let mid = (lo + hi) * 0.5;
            let mx = ei.ax as f64 + mid * ux + nx * dist * 0.5;
            let my = ei.ay as f64 + mid * uy + ny * dist * 0.5;
            if !core.owns(mx, my) {
                continue;
            }
            push_edge(
                ei.ax as f64 + lo * ux,
                ei.ay as f64 + lo * uy,
                ei.ax as f64 + hi * ux,
                ei.ay as f64 + hi * uy,
                dist,
            );
            let span = tbj - taj;
            let (f_lo, f_hi) = ((lo - taj) / span, (hi - taj) / span);
            push_edge(
                ej.ax as f64 + f_lo * djx,
                ej.ay as f64 + f_lo * djy,
                ej.ax as f64 + f_hi * djx,
                ej.ay as f64 + f_hi * djy,
                dist,
            );
        }
    }
}

/// Below this, a merged piece is a shaving rather than material: the merge rounds a 45°
/// corner drawn with a one-nanometre chamfer and leaves the chamfer behind as a triangle
/// of half a square DBU.  A hundred square DBU is a ten-nanometre square, which is orders
/// below anything a layout draws.
pub const SHAVING_DBU2: f64 = 100.0;

/// Distance from `p` to segment `a-b`, plus the closest point on the segment.
pub fn point_to_segment_closest(
    px: f64,
    py: f64,
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
) -> (f64, f64, f64) {
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
pub fn cross2(px: f64, py: f64, qx: f64, qy: f64, rx: f64, ry: f64) -> f64 {
    (qx - px) * (ry - py) - (qy - py) * (rx - px)
}

#[allow(clippy::too_many_arguments)]
pub fn segments_intersect(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
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
pub fn segment_closest_points(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> (f64, (f64, f64), (f64, f64)) {
    if segments_intersect(ax, ay, bx, by, cx, cy, dx, dy) {
        return (0.0, (ax, ay), (ax, ay));
    }
    let (d1, q1x, q1y) = point_to_segment_closest(ax, ay, cx, cy, dx, dy);
    let (d2, q2x, q2y) = point_to_segment_closest(bx, by, cx, cy, dx, dy);
    let (d3, p3x, p3y) = point_to_segment_closest(cx, cy, ax, ay, bx, by);
    let (d4, p4x, p4y) = point_to_segment_closest(dx, dy, ax, ay, bx, by);
    let mut best = (d1, (ax, ay), (q1x, q1y));
    if d2 < best.0 {
        best = (d2, (bx, by), (q2x, q2y));
    }
    if d3 < best.0 {
        best = (d3, (p3x, p3y), (cx, cy));
    }
    if d4 < best.0 {
        best = (d4, (p4x, p4y), (dx, dy));
    }
    best
}

/// True if either region has a vertex strictly inside the other (containment or
/// positive-gap overlap).  Merged same-layer regions never overlap; for two layers
/// an overlap is allowed (not a spacing violation), so such pairs are skipped.
/// Hole-aware: a region sitting inside the other's *hole* (e.g. an iso-PWell Activ
/// inside its NWell isolation ring) does not overlap it — its spacing to the hole
/// boundary is a real, checkable gap (nmosi.c).
pub fn overlapping(a: &Poly, b: &Poly) -> bool {
    // Strictly inside, not merely inside-or-on. A point on the boundary is shared
    // *boundary*, not shared area, and counting it as area is what made two shapes
    // meeting at one corner read as overlapping - so the pair never reached the spacing
    // scan that is precisely about it.
    if a.vertices().any(|&(x, y)| b.strictly_contains(x, y))
        || b.vertices().any(|&(x, y)| a.strictly_contains(x, y))
    {
        return true;
    }
    // One wholly inside the other, with boundaries that may coincide: every vertex is
    // inside or on, and none need be strictly inside - two identical regions being the
    // limiting case.
    if a.vertices().all(|&(x, y)| b.contains_point(x, y))
        || b.vertices().all(|&(x, y)| a.contains_point(x, y))
    {
        return true;
    }
    // A vertex test alone misses the case a gate is: a poly stripe crossing a COMP shares
    // a large area with it and has *no vertex of either inside the other*. Read as
    // disjoint, such a pair is measured for spacing (finding zero, since the boundaries
    // cross) or dropped from an `overlapping` rule that is precisely about it.
    //
    // Crossing must be *proper*: two shapes meeting at a boundary share no area, and that
    // is what separates this from `polys_interact` next door.
    a.edges.iter().any(|&(ax, ay, bx, by)| {
        b.edges
            .iter()
            .any(|&(cx, cy, dx, dy)| segs_properly_cross((ax, ay), (bx, by), (cx, cy), (dx, dy)))
    })
}

/// Whether two segments *properly* cross — each strictly straddles the other's line.
/// Touching at an endpoint is not a crossing: the two meet without interpenetrating, and
/// zero-area contact is `interacting`, not `overlapping`.
pub fn segs_properly_cross(p0: Marker, p1: Marker, q0: Marker, q1: Marker) -> bool {
    let cross =
        |o: Marker, a: Marker, b: Marker| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    // Coordinates are µm on a nanometre grid, so a cross product this small is zero.
    const EPS: f64 = 1e-9;
    let (d1, d2) = (cross(q0, q1, p0), cross(q0, q1, p1));
    let (d3, d4) = (cross(p0, p1, q0), cross(p0, p1, q1));
    ((d1 > EPS && d2 < -EPS) || (d1 < -EPS && d2 > EPS))
        && ((d3 > EPS && d4 < -EPS) || (d3 < -EPS && d4 > EPS))
}

#[derive(Clone, Copy)]
pub struct BBox {
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
}

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
        if xmin == f64::INFINITY {
            None
        } else {
            Some(BBox {
                xmin,
                ymin,
                xmax,
                ymax,
            })
        }
    }

    /// True if the boxes could be within `threshold` (L∞ lower bound).
    pub fn possibly_within(&self, other: &BBox, threshold: f64) -> bool {
        let gap_x = (self.xmin - other.xmax)
            .max(other.xmin - self.xmax)
            .max(0.0);
        let gap_y = (self.ymin - other.ymax)
            .max(other.ymin - self.ymax)
            .max(0.0);
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
    pub pts: Vec<(f64, f64)>,
    /// Hole contours (CW, material on the left — same convention as `MergedPoly`).
    pub holes: Vec<Vec<(f64, f64)>>,
    pub bbox: BBox,
    /// Outer *and* hole edges: both are real region boundary (spacing, width and
    /// enclosure are all measured against holes too).
    pub edges: Vec<(f64, f64, f64, f64)>,
}

impl Poly {
    /// All contour vertices, outer ring and holes.
    fn vertices(&self) -> impl Iterator<Item = &(f64, f64)> {
        self.pts.iter().chain(self.holes.iter().flatten())
    }

    /// Point strictly inside the region: inside the outer ring and in no hole.
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        point_in_polygon(x, y, &self.pts) && !self.holes.iter().any(|h| point_in_polygon(x, y, h))
    }

    /// Inside the region and not on its boundary.  Coordinates are µm on a nanometre
    /// grid, so a point meant to lie on an edge lies on it to well under this tolerance.
    fn strictly_contains(&self, x: f64, y: f64) -> bool {
        if !self.contains_point(x, y) {
            return false;
        }
        !self
            .edges
            .iter()
            .any(|&(ax, ay, bx, by)| point_to_segment_closest(x, y, ax, ay, bx, by).0 <= 1e-9)
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

pub fn poly_from_merged(m: &MergedPoly, dbu_to_um: f64) -> Option<Poly> {
    let scale = |ring: &[i_overlay::i_float::int::point::IntPoint]| -> Vec<(f64, f64)> {
        ring.iter()
            .map(|p| (p.x as f64 * dbu_to_um, p.y as f64 * dbu_to_um))
            .collect()
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
    let holes: Vec<Vec<(f64, f64)>> = m
        .holes
        .iter()
        .map(|h| scale(h))
        .filter(|h| h.len() >= 3)
        .collect();
    for h in &holes {
        ring_edges(h, &mut edges);
    }
    Some(Poly {
        pts,
        holes,
        bbox,
        edges,
    })
}

/// Closest edge-to-edge distance between two regions, with the closest point on
/// each (first on `a`, second on `b`) so the marker can span the gap.  Stops early
/// once a touching pair is found (`< half_dbu`).
/// Closest approach of two segments under KLayout's `square` metric — the L-infinity
/// distance, `max(|dx|, |dy|)`, whose unit ball is a square rather than a circle.  That
/// is what a rule worded "must not fall within a 0.56 x 0.56 um square at the corner"
/// asks for, and it is a *weaker* separation than euclidian: two shapes 0.5 um apart
/// diagonally are 0.71 apart euclidian but only 0.5 apart here, so measuring the wrong
/// one goes quiet on real violations rather than inventing them.
///
/// Exact, not sampled.  Over the two segments' parameter square, `dx` and `dy` are
/// affine, so `max(|dx|, |dy|)` is convex and piecewise linear with folds only along
/// `dx = 0`, `dy = 0` and `dx = ±dy`.  A convex piecewise-linear function attains its
/// minimum over a polygon at a vertex of the subdivision those folds induce, so
/// evaluating the corners, the fold-boundary crossings and the fold-fold crossings
/// finds it outright.
pub fn seg_seg_closest_square(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> (f64, (f64, f64), (f64, f64)) {
    let (dax, day) = (a1.0 - a0.0, a1.1 - a0.1);
    let (dbx, dby) = (b1.0 - b0.0, b1.1 - b0.1);
    // dx(t, u) and dy(t, u) as `const + t*ct + u*cu`.
    let dx = (a0.0 - b0.0, dax, -dbx);
    let dy = (a0.1 - b0.1, day, -dby);
    // The folds, each as `c + t*ct + u*cu = 0`.
    let folds = [
        dx,
        dy,
        (dx.0 - dy.0, dx.1 - dy.1, dx.2 - dy.2),
        (dx.0 + dy.0, dx.1 + dy.1, dx.2 + dy.2),
    ];

    let mut cands: Vec<(f64, f64)> = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];
    // Where each fold crosses the parameter square's own sides.
    for f in &folds {
        for e in [0.0, 1.0] {
            // t = e: solve for u.  u = e: solve for t.
            if f.2.abs() > 1e-12 {
                cands.push((e, -(f.0 + f.1 * e) / f.2));
            }
            if f.1.abs() > 1e-12 {
                cands.push((-(f.0 + f.2 * e) / f.1, e));
            }
        }
    }
    // Where two folds cross each other.
    for i in 0..folds.len() {
        for j in i + 1..folds.len() {
            let (p, q) = (folds[i], folds[j]);
            let det = p.1 * q.2 - p.2 * q.1;
            if det.abs() > 1e-12 {
                cands.push((
                    (-p.0 * q.2 + p.2 * q.0) / det,
                    (-p.1 * q.0 + p.0 * q.1) / det,
                ));
            }
        }
    }

    let mut best = (f64::INFINITY, (0.0, 0.0), (0.0, 0.0));
    for (t, u) in cands {
        if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
            continue;
        }
        let pa = (a0.0 + t * dax, a0.1 + t * day);
        let pb = (b0.0 + u * dbx, b0.1 + u * dby);
        let d = (pa.0 - pb.0).abs().max((pa.1 - pb.1).abs());
        if d < best.0 {
            best = (d, pa, pb);
        }
    }
    best
}

pub fn closest(a: &Poly, b: &Poly, half_dbu: f64, square: bool) -> (f64, (f64, f64), (f64, f64)) {
    let mut min_dist = f64::INFINITY;
    let mut pa = (0.0, 0.0);
    let mut pb = (0.0, 0.0);
    'outer: for &(ax, ay, bx, by) in &a.edges {
        for &(cx, cy, dx, dy) in &b.edges {
            let (d, qa, qb) = if square {
                seg_seg_closest_square((ax, ay), (bx, by), (cx, cy), (dx, dy))
            } else {
                segment_closest_points(ax, ay, bx, by, cx, cy, dx, dy)
            };
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

/// One facing gap: its width, and the two points that measure it.
pub type Gap = (f64, (f64, f64), (f64, f64));

/// One facing gap with the *runs* that face each other, not just the pair of points that
/// measure the distance between them.  A rule only needs the measurement; a layer built
/// from the same scan needs the stretch it spans, which is the polygon KLayout's
/// `.polygons` makes out of an edge pair.
pub struct FacingRun {
    pub gap: f64,
    /// The facing sub-segment of the first region's edge, and of the second's.
    pub a: (f64, f64, f64, f64),
    pub b: (f64, f64, f64, f64),
    /// The two points [`facing_gaps`] reports, kept so both read the same scan.
    pub closest: ((f64, f64), (f64, f64)),
}

/// The stretch of a segment lying strictly outside the halfplane boundary through `q`
/// with outward normal `n`, as a parameter range of the segment, or `None` if none of it
/// reaches there.  A segment that only *touches* the boundary yields an empty range, and
/// so is rejected by the length test at the call site — which is the whole point: a
/// corner resting on a wall is not a gap.
pub fn halfplane_span(
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    (qx, qy): (f64, f64),
    (nx, ny): (f64, f64),
) -> Option<(f64, f64)> {
    let f0 = (x0 - qx) * nx + (y0 - qy) * ny;
    let f1 = (x1 - qx) * nx + (y1 - qy) * ny;
    match (f0 > 0.0, f1 > 0.0) {
        (false, false) => None,
        (true, true) => Some((0.0, 1.0)),
        (true, false) => Some((0.0, f0 / (f0 - f1))),
        (false, true) => Some((f0 / (f0 - f1), 1.0)),
    }
}

/// Gaps between the facing edges of two regions, for a rule whose two shapes *overlap*.
///
/// The ordinary spacing scan measures a region pair's closest approach and skips any pair
/// that overlaps or touches, which is right: two shapes that share area have no single
/// "space" between them. But a rule can be about exactly that shape. GF180's `S.PL.5b_MV`
/// asks the space from a poly to the COMP it *gates* — so the pair overlaps by
/// definition, and the gap it means is somewhere else along the same two shapes.
///
/// A pair here is two anti-parallel edges facing each other across empty ground: their
/// directions oppose, they project onto one another, and the second lies on the outward
/// side of the first. Both outward normals therefore point into the gap, so the strip
/// between them is outside both shapes locally — but only locally, since another arm of
/// either shape can lie in it, which the caller's emptiness probe rules out.
pub fn facing_gaps(a: &Poly, b: &Poly, value: f64, tol: f64, inward: bool) -> Vec<Gap> {
    facing_runs(a, b, value, tol, inward)
        .into_iter()
        .map(|r| (r.gap, r.closest.0, r.closest.1))
        .collect()
}

/// The same scan, keeping the facing runs.  [`facing_gaps`] is this with the runs dropped.
pub fn facing_runs(a: &Poly, b: &Poly, value: f64, tol: f64, inward: bool) -> Vec<FacingRun> {
    // Space and overlap are the same measurement in opposite directions: both pair two
    // edges whose outward normals oppose, and differ only in whether the other edge sits
    // on this one's outside (empty ground between them, a gap) or its inside (material of
    // both between them, an overlap).  One sign carries that.
    let sgn = if inward { -1.0 } else { 1.0 };
    let mut out = Vec::new();
    for &(ax, ay, bx, by) in &a.edges {
        let (dax, day) = (bx - ax, by - ay);
        let la = dax.hypot(day);
        if la <= 0.0 {
            continue;
        }
        let (ux, uy) = (dax / la, day / la);
        let (nx, ny) = (uy, -ux); // outward for a CCW contour
        for &(cx, cy, dx, dy) in &b.edges {
            let (dbx, dby) = (dx - cx, dy - cy);
            let lb = dbx.hypot(dby);
            if lb <= 0.0 {
                continue;
            }
            if (dax * dby - day * dbx).abs() > 1e-6 * la * lb {
                // Not parallel, so there is no constant gap to project - the SRAM COMP is
                // a five-point arrow whose 45° flank runs at the poly's wall, closing to
                // nothing at the tip. Clip each edge to the other's outside halfplane and
                // measure what is left. The clip is what separates this from a poly
                // sitting *inside* a COMP with a coincident wall: there a corner rests on
                // the wall, so one edge only touches the other's boundary line and never
                // reaches its outside, and the pair drops out with an empty span.
                let (nbx, nby) = (dby / lb, -dbx / lb);
                if nx * nbx + ny * nby >= 0.0 {
                    // The normals must oppose, or the edges are not facing each other
                    // across the gap - this is the anti-parallel test above, generalised.
                    // Perpendicular edges meeting at a point fail it, which is what keeps
                    // an ordinary convex corner of empty space from reading as a gap.
                    continue;
                }
                let (Some((sa0, sa1)), Some((sb0, sb1))) = (
                    halfplane_span((ax, ay, bx, by), (cx, cy), (sgn * nbx, sgn * nby)),
                    halfplane_span((cx, cy, dx, dy), (ax, ay), (sgn * nx, sgn * ny)),
                ) else {
                    continue;
                };
                if (sa1 - sa0) * la <= tol || (sb1 - sb0) * lb <= tol {
                    continue;
                }
                let (ra, rb) = (
                    (
                        ax + sa0 * dax,
                        ay + sa0 * day,
                        ax + sa1 * dax,
                        ay + sa1 * day,
                    ),
                    (
                        cx + sb0 * dbx,
                        cy + sb0 * dby,
                        cx + sb1 * dbx,
                        cy + sb1 * dby,
                    ),
                );
                let (gap, pa, pb) =
                    seg_seg_closest((ra.0, ra.1), (ra.2, ra.3), (rb.0, rb.1), (rb.2, rb.3));
                if gap < value {
                    out.push(FacingRun {
                        gap,
                        a: ra,
                        b: rb,
                        closest: (pa, pb),
                    });
                }
                continue;
            }
            if dax * dbx + day * dby >= 0.0 {
                continue; // same direction: back to back, not facing
            }
            let gap = sgn * ((cx - ax) * nx + (cy - ay) * ny);
            if gap <= tol || gap >= value {
                continue; // touching, behind, or far enough apart
            }
            let t0 = (cx - ax) * ux + (cy - ay) * uy;
            let t1 = (dx - ax) * ux + (dy - ay) * uy;
            let (s0, s1) = (t0.min(t1).max(0.0), t0.max(t1).min(la));
            if s1 - s0 <= tol {
                continue; // no projected overlap: they do not face each other
            }
            let mid = (s0 + s1) * 0.5;
            let (px, py) = (ax + mid * ux, ay + mid * uy);
            let (ox, oy) = (sgn * nx * gap, sgn * ny * gap);
            let ra = (ax + s0 * ux, ay + s0 * uy, ax + s1 * ux, ay + s1 * uy);
            out.push(FacingRun {
                gap,
                a: ra,
                b: (ra.0 + ox, ra.1 + oy, ra.2 + ox, ra.3 + oy),
                closest: ((px, py), (px + ox, py + oy)),
            });
        }
    }
    out
}

/// A merged region's marker point in DBU — its [`merged_centroid_dbu`].  Passed to the
/// gate so a net-aware rule can resolve each region to a net without a second merge.
pub type Marker = (f64, f64);

/// Absolute area (DBU²) and centroid (DBU) of a closed contour, either winding.
pub fn ring_area_centroid(c: &[IntPoint]) -> (f64, f64, f64) {
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
        let (sx, sy) = c
            .iter()
            .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x as f64, sy + p.y as f64));
        let m = n.max(1) as f64;
        return (area, sx / m, sy / m);
    }
    (area, cx / (3.0 * sum), cy / (3.0 * sum))
}

pub fn point_to_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    point_to_segment_closest(px, py, ax, ay, bx, by).0
}

/// Vertex inside the outer region or on its boundary (within `tol`) — the boundary
/// case lets value-0 rules pass when the inner shape touches the outer edge.
/// Hole-aware: a vertex inside the outer region's hole is *not* inside (its edges,
/// which include the hole contours, still grant the on-boundary tolerance).
pub fn vertex_inside_or_on(px: f64, py: f64, outer: &Poly, tol: f64) -> bool {
    outer.contains_point(px, py)
        || outer
            .edges
            .iter()
            .any(|&(ax, ay, bx, by)| point_to_segment_dist(px, py, ax, ay, bx, by) <= tol)
}

pub fn all_vertices_inside(inner: &Poly, outer: &Poly, tol: f64) -> bool {
    inner
        .vertices()
        .all(|&(x, y)| vertex_inside_or_on(x, y, outer, tol))
}

/// Whether two segments meet — properly crossing, or touching at a point.
pub fn segs_meet(p0: Marker, p1: Marker, q0: Marker, q1: Marker) -> bool {
    let cross =
        |o: Marker, a: Marker, b: Marker| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    // Coordinates are µm on a nanometre grid, so a cross product this small is zero.
    const EPS: f64 = 1e-9;
    let (d1, d2) = (cross(q0, q1, p0), cross(q0, q1, p1));
    let (d3, d4) = (cross(p0, p1, q0), cross(p0, p1, q1));
    if ((d1 > EPS && d2 < -EPS) || (d1 < -EPS && d2 > EPS))
        && ((d3 > EPS && d4 < -EPS) || (d3 < -EPS && d4 > EPS))
    {
        return true; // the segments properly cross
    }
    // Collinear or endpoint-touching: a zero cross product with the point in range.
    let within = |a: Marker, b: Marker, p: Marker| {
        p.0 >= a.0.min(b.0) - EPS
            && p.0 <= a.0.max(b.0) + EPS
            && p.1 >= a.1.min(b.1) - EPS
            && p.1 <= a.1.max(b.1) + EPS
    };
    (d1.abs() <= EPS && within(q0, q1, p0))
        || (d2.abs() <= EPS && within(q0, q1, p1))
        || (d3.abs() <= EPS && within(p0, p1, q0))
        || (d4.abs() <= EPS && within(p0, p1, q1))
}

/// Whether two polygons share a *run* of boundary rather than meeting at isolated points.
///
/// Shapes drawn edge to edge abut: there is no space anywhere between them, and a
/// separation of zero would be a fiction. Shapes meeting at one corner have a real gap
/// that happens to be nothing wide. Both have a closest approach of zero, and only the
/// second is a spacing violation.
pub fn shares_boundary_run(a: &Poly, b: &Poly, tol: f64) -> bool {
    for &(ax, ay, bx, by) in &a.edges {
        let (ux, uy) = (bx - ax, by - ay);
        let la = ux.hypot(uy);
        if la <= 0.0 {
            continue;
        }
        let (ux, uy) = (ux / la, uy / la);
        for &(cx, cy, dx, dy) in &b.edges {
            let (vx, vy) = (dx - cx, dy - cy);
            let lb = vx.hypot(vy);
            if lb <= 0.0 || (ux * vy - uy * vx).abs() > 1e-6 * lb {
                continue; // not parallel
            }
            if ((cx - ax) * -uy + (cy - ay) * ux).abs() > tol {
                continue; // parallel but not collinear
            }
            let t0 = (cx - ax) * ux + (cy - ay) * uy;
            let t1 = (dx - ax) * ux + (dy - ay) * uy;
            let (s0, s1) = (t0.min(t1).max(0.0), t0.max(t1).min(la));
            if s1 - s0 > tol {
                return true; // a shared stretch, not a shared point
            }
        }
    }
    false
}

/// Whether two polygons share any area or boundary.
///
/// A vertex test alone is not enough, and the case it misses is the ordinary one: two
/// rectangles crossing in a plus shape have a large shared area and *no vertex of either
/// inside the other*. That is what a transistor gate over its COMP looks like, so
/// `interacting_only` enclosure rules — "COMP extend beyond gate", "poly end cap" —
/// silently measured nothing at all on every real device. Edge crossings are tested too.
pub fn polys_interact(a: &Poly, b: &Poly) -> bool {
    // Overlapping boxes have a clamped gap of exactly 0.0, so the threshold must be
    // positive — with 0.0 the prefilter rejects every pair (`gap < 0.0` never holds).
    if !a.bbox.possibly_within(&b.bbox, f64::MIN_POSITIVE) {
        return false;
    }
    if a.vertices().any(|&(x, y)| b.contains_point(x, y))
        || b.vertices().any(|&(x, y)| a.contains_point(x, y))
    {
        return true;
    }
    a.edges.iter().any(|&(ax, ay, bx, by)| {
        b.edges
            .iter()
            .any(|&(cx, cy, dx, dy)| segs_meet((ax, ay), (bx, by), (cx, cy), (dx, dy)))
    })
}

/// One candidate enclosure violation: a facing inner/outer edge pair below the rule
/// value, with a probe point just beyond the measured outer wall for the reality check.
pub struct EnclosurePair {
    pub dist: f64,
    pub edge: (f64, f64, f64, f64),
    pub probe: (f64, f64),
}

#[allow(clippy::too_many_arguments)]
pub fn enclosure_pairs(
    inner: &Poly,
    outer: &Poly,
    cutoff: f64,
    skip_coincident: bool,
    euclidian: bool,
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
            if lo <= 0.0 {
                continue;
            }
            if (dix * doy - diy * dox).abs() > 1e-6 * li * lo {
                // Not parallel, so there is no facing run to project onto. Under the
                // projection metric that is the end of it; under the euclidian one the
                // wall is still a wall, and the margin is the shortest distance to it.
                // This is what an arrow's tip or a chamfered corner needs: GF180's SRAM
                // case sets every orthogonal margin to exactly the limit and then cuts
                // one corner at 45°, so the only deficit is one no parallel pair can see.
                if !euclidian {
                    continue;
                }
                let (d, near_in, near_out) =
                    seg_seg_closest((ax, ay), (bx, by), (cx, cy), (dx, dy));
                if d >= cutoff {
                    continue;
                }
                // Outward test. Without it the search finds the far wall across a concave
                // shape and reports a margin that is not an enclosure at all. A touch
                // (d ~ 0) has no direction to test, and is the same clip artifact that
                // `skip_coincident` governs on the parallel path.
                if d <= tol {
                    saw_coincident = true;
                    if skip_coincident {
                        continue;
                    }
                } else if (near_out.0 - near_in.0) * nx + (near_out.1 - near_in.1) * ny <= tol {
                    continue;
                }
                let (px, py) = if d <= tol {
                    (near_in.0 + nx * 2.0 * tol, near_in.1 + ny * 2.0 * tol)
                } else {
                    let inv = 1.0 / d;
                    let (vx, vy) = (
                        (near_out.0 - near_in.0) * inv,
                        (near_out.1 - near_in.1) * inv,
                    );
                    (
                        near_in.0 + vx * (d + 2.0 * tol),
                        near_in.1 + vy * (d + 2.0 * tol),
                    )
                };
                pairs.push(EnclosurePair {
                    dist: d,
                    edge: (near_in.0, near_in.1, near_out.0, near_out.1),
                    probe: (px, py),
                });
                continue;
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

/// Even-odd ray-casting point-in-polygon test.
pub fn point_in_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
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
