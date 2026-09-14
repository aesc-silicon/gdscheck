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
use std::collections::{HashMap, HashSet};

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

/// The box of `poly` if it is one: four corners, no holes, every edge on an axis.
pub fn axis_rect(poly: &MergedPoly) -> Option<(i32, i32, i32, i32)> {
    let o = &poly.outer;
    if o.len() != 4 || !poly.holes.is_empty() {
        return None;
    }
    for i in 0..4 {
        let (a, b) = (o[i], o[(i + 1) % 4]);
        if a.x != b.x && a.y != b.y {
            return None;
        }
    }
    let (x0, x1) = (o.iter().map(|p| p.x).min()?, o.iter().map(|p| p.x).max()?);
    let (y0, y1) = (o.iter().map(|p| p.y).min()?, o.iter().map(|p| p.y).max()?);
    Some((x0, y0, x1, y1))
}

pub fn sorted_unique(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable();
    v.dedup();
    v
}

/// A width rule's bound, stated on the grid.
///
/// The rule's value is a µm figure and the spans it bounds are integers, so the value is
/// put on the grid once, rounded the way that keeps the rule's meaning: a minimum rounds
/// up (a span under a fractional limit is under the next whole DBU too), a maximum rounds
/// down, an exact value rounds to nearest.  After that every comparison is exact.  An
/// axis-aligned span is the difference of two coordinates; an oblique one is a square
/// root, so it is compared *squared*, as the ratio of two integers - which is why the
/// bound is asked two ways.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Limit {
    /// No span under this many DBU.
    AtLeast(i64),
    /// No span over this many DBU.
    AtMost(i64),
    /// Every span this many DBU.
    Exactly(i64),
}

impl Limit {
    pub fn at_least(um: f64, dbu_to_um: f64) -> Self {
        Limit::AtLeast(on_grid(um / dbu_to_um, f64::ceil))
    }

    pub fn at_most(um: f64, dbu_to_um: f64) -> Self {
        Limit::AtMost(on_grid(um / dbu_to_um, f64::floor))
    }

    pub fn exactly(um: f64, dbu_to_um: f64) -> Self {
        Limit::Exactly(on_grid(um / dbu_to_um, f64::round))
    }

    /// The bound in DBU.
    pub fn dbu(self) -> i64 {
        match self {
            Limit::AtLeast(v) | Limit::AtMost(v) | Limit::Exactly(v) => v,
        }
    }

    /// Whether a span of `w` DBU breaks the bound.
    pub fn broken_by(self, w: i64) -> bool {
        match self {
            Limit::AtLeast(v) => w < v,
            Limit::AtMost(v) => w > v,
            Limit::Exactly(v) => w != v,
        }
    }

    /// Whether a span whose *square* is `num / den` breaks the bound.  `den` is positive.
    pub fn broken_by_sq(self, num: i128, den: i128) -> bool {
        let v = self.dbu() as i128;
        let rhs = v * v * den;
        match self {
            Limit::AtLeast(_) => num < rhs,
            Limit::AtMost(_) => num > rhs,
            Limit::Exactly(_) => num != rhs,
        }
    }
}

/// `x` on the grid: the integer it already is, allowing for the noise a µm-to-DBU
/// division leaves (0.15 / 0.001 is 149.99999999999997), else `round` applied.
pub fn on_grid(x: f64, round: fn(f64) -> f64) -> i64 {
    let n = x.round();
    if (x - n).abs() < 1e-6 {
        n as i64
    } else {
        round(x) as i64
    }
}

/// The walls of a region that lie on another region's boundary - or the ones that do not.
///
/// A gate has two kinds of wall.  The ones the poly brought with it stand across the
/// channel, and the distance between them is the gate's length; the ones the active cut
/// stand at the ends, and the distance between them is the transistor's width.  Both are
/// widths of the same region, and only which walls take part tells them apart.  A width
/// scan run with this filter keeps, of every facing pair it finds, the stretch along
/// which *both* walls lie on the reference boundary (`on`), or along which neither does
/// (`off`), and drops the pair where that stretch is empty.
///
/// Collinearity is exact: a wall lies on the reference boundary where a reference
/// segment runs along the same line and the two overlap.  The overlap itself is kept as
/// an interval of the wall - integers for an axis-aligned wall, fractions of its length
/// for an oblique one, where only the clipping of the marker depends on it.
///
/// An `outside` region cuts each wall's stretches further to the parts lying outside it
/// (KLayout's `edges.not(region)`): a native gate's length is read off the poly walls
/// that are not under the well.
pub struct WallFilter<'a> {
    on: bool,
    outside: Option<&'a [MergedPoly]>,
    /// Reference vertical segments by x, as `(ylo, yhi)`.
    verticals: HashMap<i32, Vec<(i64, i64)>>,
    /// Reference horizontal segments by y, as `(xlo, xhi)`.
    horizontals: HashMap<i32, Vec<(i64, i64)>>,
    /// Reference oblique segments, undirected.
    obliques: Vec<((i64, i64), (i64, i64))>,
}

impl<'a> WallFilter<'a> {
    /// Keep the stretches on the boundary of `reference` (`on`), or off it, and in either
    /// case only where they lie outside `outside`.
    pub fn new(reference: &[MergedPoly], on: bool, outside: Option<&'a [MergedPoly]>) -> Self {
        let mut f = WallFilter {
            on,
            outside,
            verticals: HashMap::new(),
            horizontals: HashMap::new(),
            obliques: Vec::new(),
        };
        for m in reference {
            for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
                let n = ring.len();
                for i in 0..n {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    if a.x == b.x && a.y != b.y {
                        f.verticals
                            .entry(a.x)
                            .or_default()
                            .push((a.y.min(b.y) as i64, a.y.max(b.y) as i64));
                    } else if a.y == b.y && a.x != b.x {
                        f.horizontals
                            .entry(a.y)
                            .or_default()
                            .push((a.x.min(b.x) as i64, a.x.max(b.x) as i64));
                    } else if a != b {
                        f.obliques
                            .push(((a.x as i64, a.y as i64), (b.x as i64, b.y as i64)));
                    }
                }
            }
        }
        f
    }

    /// Whether the filter keeps a *point* - one on the reference boundary for `shared`,
    /// one off it for `unshared` - which is what a pinch, a width of zero at a vertex,
    /// asks of it.
    pub fn keeps_point(&self, x: i32, y: i32) -> bool {
        let on = self
            .verticals
            .get(&x)
            .is_some_and(|v| v.iter().any(|&(lo, hi)| lo <= y as i64 && y as i64 <= hi))
            || self
                .horizontals
                .get(&y)
                .is_some_and(|v| v.iter().any(|&(lo, hi)| lo <= x as i64 && x as i64 <= hi))
            || self.obliques.iter().any(|&(a, b)| {
                let (dx, dy) = ((b.0 - a.0) as i128, (b.1 - a.1) as i128);
                let (wx, wy) = ((x as i64 - a.0) as i128, (y as i64 - a.1) as i128);
                let t = wx * dx + wy * dy;
                wx * dy - wy * dx == 0 && t >= 0 && t <= dx * dx + dy * dy
            });
        on == self.on
    }

    /// The stretches of a vertical wall at `x` over `[lo, hi]` that this filter keeps.
    fn keep_v(&self, x: i32, lo: i64, hi: i64) -> Vec<(i64, i64)> {
        let kept = self.keep(self.verticals.get(&x).map_or(&[][..], |v| v), lo, hi);
        self.cut_axis(kept, (x as f64, 0.0), (0.0, 1.0))
    }

    /// The stretches of a horizontal wall at `y` over `[lo, hi]` that this filter keeps.
    fn keep_h(&self, y: i32, lo: i64, hi: i64) -> Vec<(i64, i64)> {
        let kept = self.keep(self.horizontals.get(&y).map_or(&[][..], |v| v), lo, hi);
        self.cut_axis(kept, (0.0, y as f64), (1.0, 0.0))
    }

    fn keep(&self, on_line: &[(i64, i64)], lo: i64, hi: i64) -> Vec<(i64, i64)> {
        let covered = union(
            on_line
                .iter()
                .map(|&(a, b)| (a.max(lo), b.min(hi)))
                .filter(|(a, b)| b > a)
                .collect(),
        );
        if self.on {
            covered
        } else {
            complement(&covered, lo, hi)
        }
    }

    /// Axis-aligned stretches, as coordinates along `u` from `origin`, cut to the parts
    /// outside the `outside` region.  A cut lands on the grid for a rectilinear region;
    /// against a chamfer it is rounded, which moves a marker's end by under a DBU.
    fn cut_axis(
        &self,
        kept: Vec<(i64, i64)>,
        origin: (f64, f64),
        u: (f64, f64),
    ) -> Vec<(i64, i64)> {
        let Some(region) = self.outside else {
            return kept;
        };
        kept.into_iter()
            .flat_map(|(a, b)| outside_part(region, origin, u, a as f64, b as f64))
            .map(|(a, b)| (a.round() as i64, b.round() as i64))
            .filter(|(a, b)| b > a)
            .collect()
    }

    /// The stretches of the oblique wall `a → b` this filter keeps, as fractions of its
    /// length.
    fn keep_o(&self, a: (i64, i64), b: (i64, i64)) -> Vec<(f64, f64)> {
        let (dx, dy) = ((b.0 - a.0) as i128, (b.1 - a.1) as i128);
        let len2 = dx * dx + dy * dy;
        let along = |p: (i64, i64)| -> Option<i128> {
            let (wx, wy) = ((p.0 - a.0) as i128, (p.1 - a.1) as i128);
            (wx * dy - wy * dx == 0).then_some(wx * dx + wy * dy)
        };
        let mut covered = Vec::new();
        for &(c, d) in &self.obliques {
            let (Some(tc), Some(td)) = (along(c), along(d)) else {
                continue; // not on this wall's line
            };
            let (t0, t1) = (tc.min(td).max(0), tc.max(td).min(len2));
            if t1 > t0 {
                covered.push((t0 as f64 / len2 as f64, t1 as f64 / len2 as f64));
            }
        }
        let covered = union(covered);
        let kept = if self.on {
            covered
        } else {
            complement(&covered, 0.0, 1.0)
        };
        let Some(region) = self.outside else {
            return kept;
        };
        let len = (len2 as f64).sqrt();
        let u = (dx as f64 / len, dy as f64 / len);
        kept.into_iter()
            .flat_map(|(s0, s1)| {
                outside_part(region, (a.0 as f64, a.1 as f64), u, s0 * len, s1 * len)
            })
            .map(|(s0, s1)| (s0 / len, s1 / len))
            .collect()
    }
}

/// The parts of the line `origin + s·u` for `s` in `[lo, hi]` that lie outside `region`.
///
/// The interval is split wherever a boundary segment of the region crosses the line, and
/// each piece is classified by its midpoint - a piece never straddles a boundary, so its
/// midpoint speaks for all of it.  A wall running *along* a boundary is decided by the
/// midpoint too, which is the one place the answer depends on which side of the line the
/// ray-casting falls; the rules that cut this way keep their regions clear of the walls
/// they measure.
fn outside_part(
    region: &[MergedPoly],
    origin: (f64, f64),
    u: (f64, f64),
    lo: f64,
    hi: f64,
) -> Vec<(f64, f64)> {
    let mut cuts = vec![lo, hi];
    for m in region {
        for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
            let n = ring.len();
            for i in 0..n {
                let (p, q) = (ring[i], ring[(i + 1) % n]);
                let (px, py) = (p.x as f64 - origin.0, p.y as f64 - origin.1);
                let (ex, ey) = ((q.x - p.x) as f64, (q.y - p.y) as f64);
                let denom = u.0 * ey - u.1 * ex;
                if denom.abs() < 1e-12 {
                    continue; // parallel: no crossing, or collinear
                }
                let s = (px * ey - py * ex) / denom;
                let v = (px * u.1 - py * u.0) / denom;
                if (0.0..=1.0).contains(&v) && s > lo && s < hi {
                    cuts.push(s);
                }
            }
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup();
    let inside = |x: f64, y: f64| {
        region.iter().any(|m| {
            let ring = |r: &[IntPoint]| -> Vec<(f64, f64)> {
                r.iter().map(|p| (p.x as f64, p.y as f64)).collect()
            };
            point_in_polygon(x, y, &ring(&m.outer))
                && !m.holes.iter().any(|h| point_in_polygon(x, y, &ring(h)))
        })
    };
    cuts.windows(2)
        .filter(|w| w[1] > w[0])
        .filter(|w| {
            let mid = (w[0] + w[1]) * 0.5;
            !inside(origin.0 + mid * u.0, origin.1 + mid * u.1)
        })
        .map(|w| (w[0], w[1]))
        .collect()
}

/// Sorted, merged, non-empty intervals.
fn union<T: Copy + PartialOrd>(mut v: Vec<(T, T)>) -> Vec<(T, T)> {
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<(T, T)> = Vec::new();
    for (a, b) in v {
        if b <= a {
            continue;
        }
        match out.last_mut() {
            Some(last) if a <= last.1 => {
                if b > last.1 {
                    last.1 = b;
                }
            }
            _ => out.push((a, b)),
        }
    }
    out
}

/// The common stretches of two interval lists, each sorted and merged.
fn intersect<T: Copy + PartialOrd>(a: &[(T, T)], b: &[(T, T)]) -> Vec<(T, T)> {
    let mut out = Vec::new();
    for &(a0, a1) in a {
        for &(b0, b1) in b {
            let lo = if a0 > b0 { a0 } else { b0 };
            let hi = if a1 < b1 { a1 } else { b1 };
            if hi > lo {
                out.push((lo, hi));
            }
        }
    }
    out
}

/// What `[lo, hi]` has left once the (sorted, merged) intervals are taken out.
fn complement<T: Copy + PartialOrd>(covered: &[(T, T)], lo: T, hi: T) -> Vec<(T, T)> {
    let mut out = Vec::new();
    let mut at = lo;
    for &(a, b) in covered {
        if a > at {
            out.push((at, a));
        }
        if b > at {
            at = b;
        }
    }
    if hi > at {
        out.push((at, hi));
    }
    out
}

/// Every facing-wall pair of `poly` whose span breaks `limit`, as `(x1, y1, x2, y2, width)`
/// in DBU - two entries per pair, one for each wall.
///
/// This is the measurement without the reporting: the width checks format these into
/// violations, and an edge layer built from a width can take the same pairs as geometry.
/// With a [`WallFilter`] the pairs are cut to the stretches whose walls the filter keeps,
/// and a pair with no such stretch is dropped; the mixed pass is not filtered, since the
/// rules that filter do not ask for it.
///
/// The arithmetic is exact.  Coordinates are integers, so an axis-aligned span is one,
/// and an oblique span - a square root - is compared squared, as a ratio of two integers
/// in `i128`.  A chip is under 2^28 DBU across, so a cross product of two edge vectors is
/// under 2^57 and its square under 2^114, which leaves room.  Only the reported
/// coordinates and the width in the message are floats, since the walls of an oblique
/// pair are cut at points that are not on the grid.
#[allow(clippy::too_many_arguments)]
pub fn width_pairs(
    poly: &MergedPoly,
    core: Core,
    limit: Limit,
    walls: Option<&WallFilter>,
    oblique_only: bool,
    mixed: bool,
    min_run: i64,
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut out = Vec::new();
    // An axis-aligned rectangle - every via, most contacts - has one width and one
    // height, and the sweep below would find exactly the two pairs the box gives
    // directly.  Same pairs, same order, same ownership test, without the sweep.  A
    // filtered scan goes through the sweep, which is where the cutting happens.
    if walls.is_none()
        && let Some((x0, y0, x1, y1)) = axis_rect(poly)
    {
        if oblique_only {
            return out;
        }
        let (cx, cy) = ((x0 + x1) as f64 * 0.5, (y0 + y1) as f64 * 0.5);
        if core.owns(cx, cy) {
            let (w, h) = ((x1 - x0) as i64, (y1 - y0) as i64);
            if w > 0 && limit.broken_by(w) {
                out.push((x0 as f64, y0 as f64, x0 as f64, y1 as f64, w as f64));
                out.push((x1 as f64, y0 as f64, x1 as f64, y1 as f64, w as f64));
            }
            if h > 0 && limit.broken_by(h) {
                out.push((x0 as f64, y0 as f64, x1 as f64, y0 as f64, h as f64));
                out.push((x0 as f64, y1 as f64, x1 as f64, y1 as f64, h as f64));
            }
        }
        return out;
    }
    let mut vedges = Vec::new();
    let mut hedges = Vec::new();
    let mut oedges = Vec::new();
    collect_edges(poly, &mut vedges, &mut hedges, &mut oedges);

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
                    let width = r.x as i64 - l.x as i64;
                    if width > 0 && limit.broken_by(width) {
                        let (yb, yb1) = (yb as i64, yb1 as i64);
                        let stretches = match walls {
                            None => vec![(yb, yb1)],
                            Some(f) => intersect(&f.keep_v(l.x, yb, yb1), &f.keep_v(r.x, yb, yb1)),
                        };
                        let cx = (l.x as f64 + r.x as f64) * 0.5;
                        for (s0, s1) in stretches {
                            let cy = (s0 as f64 + s1 as f64) * 0.5;
                            if core.owns(cx, cy) {
                                push_edge(
                                    l.x as f64,
                                    s0 as f64,
                                    l.x as f64,
                                    s1 as f64,
                                    width as f64,
                                );
                                push_edge(
                                    r.x as f64,
                                    s0 as f64,
                                    r.x as f64,
                                    s1 as f64,
                                    width as f64,
                                );
                            }
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
                    let height = t.y as i64 - b.y as i64;
                    if height > 0 && limit.broken_by(height) {
                        let (xb, xb1) = (xb as i64, xb1 as i64);
                        let stretches = match walls {
                            None => vec![(xb, xb1)],
                            Some(f) => intersect(&f.keep_h(b.y, xb, xb1), &f.keep_h(t.y, xb, xb1)),
                        };
                        let cy = (b.y as f64 + t.y as f64) * 0.5;
                        for (s0, s1) in stretches {
                            let cx = (s0 as f64 + s1 as f64) * 0.5;
                            if core.owns(cx, cy) {
                                push_edge(
                                    s0 as f64,
                                    b.y as f64,
                                    s1 as f64,
                                    b.y as f64,
                                    height as f64,
                                );
                                push_edge(
                                    s0 as f64,
                                    t.y as f64,
                                    s1 as f64,
                                    t.y as f64,
                                    height as f64,
                                );
                            }
                        }
                    }
                }
            }
        }
    } // end !oblique_only

    if mixed {
        mixed_widths(&oedges, &vedges, &hedges, core, &mut push_edge, limit);
    }
    if !oblique_only && walls.is_none() && matches!(limit, Limit::AtLeast(_)) {
        corner_widths(&vedges, &hedges, core, &mut push_edge, limit);
        acute_corners(poly, core, &mut push_edge);
    }
    oblique_widths(&oedges, core, &mut push_edge, limit, min_run, walls);
    out
}

/// The nearest ends of two bars that share no stretch: the squared distance and the two
/// ends, in DBU.
type EndPair = (i128, (i128, i128), (i128, i128));

/// An **acute corner**: two edges meeting at a vertex with less than a right angle of
/// material between them.  The wedge narrows to nothing at the tip, so there is no
/// minimum it satisfies, and KLayout's `width` reports the pair of edges at any value:
/// they are at distance zero and within its angle limit.  Reported as a width of zero at
/// the vertex, once per corner; the sweeps never see it, since the two edges share a
/// vertex and every pass drops a pair that touches.  Material is on the left of every
/// ring, so an acute corner is a left turn through more than a right angle - more by a
/// margin the grid cannot produce on its own: a 45° bar's square end, rounded to the
/// DBU, is a right angle a hair off, and the dot product of its two edges is then at
/// most the longer edge's length, where a real acute corner's is a good fraction of the
/// product of the two.
fn acute_corners(
    poly: &MergedPoly,
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
) {
    for ring in std::iter::once(&poly.outer).chain(poly.holes.iter()) {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let u = ((v.x - p.x) as i128, (v.y - p.y) as i128);
            let w = ((q.x - v.x) as i128, (q.y - v.y) as i128);
            let left_turn = u.0 * w.1 - u.1 * w.0 > 0;
            let dot = u.0 * w.0 + u.1 * w.1;
            let longest2 = (u.0 * u.0 + u.1 * u.1).max(w.0 * w.0 + w.1 * w.1);
            let past_right_angle = dot < 0 && 4 * dot * dot > 9 * longest2;
            if left_turn && past_right_angle && core.owns(v.x as f64, v.y as f64) {
                push_edge(v.x as f64, v.y as f64, v.x as f64, v.y as f64, 0.0);
            }
        }
    }
}

/// A span between two grid points, filed with its lower end first so that the same span
/// read from either end compares equal.
type Span = ((i64, i64), (i64, i64));

/// Widths read **across a corner**: two facing axis-aligned walls whose projections do
/// not overlap, measured from the near end of one to the near end of the other.
///
/// The band sweeps pair walls along the stretch they share, and a pair that shares none
/// (the bottom wall of one step of a jog and the top wall of the next, the two short
/// stubs either side of a chamfer) was measured by nothing.  KLayout's `width` reads
/// those under its euclidian metric: two edges with their material sides toward each
/// other, closer than the value at their nearest points, are a width however they sit,
/// and a jog whose steps are a few DBU apart is pinched there as surely as a neck.  The
/// facing test is the band sweep's: a left wall against a right wall to its right, a
/// bottom wall against a top wall above it.  Only a minimum reads this way; a maximum
/// or an exact width is about the span the walls share.
///
/// Every right wall's two ends are filed by a cell the size of the limit, so a left wall
/// asks only the cells around its own ends and a comb of a thousand fingers does not
/// pay a million pairs.
fn corner_widths(
    vedges: &[VEdge],
    hedges: &[HEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    limit: Limit,
) {
    let l = limit.dbu();
    if l <= 0 {
        return;
    }
    let cell = |v: i64| v.div_euclid(l);
    // A wall as (position across, lo, hi along) with the material on its high side
    // (`near` = true: left or bottom wall) or its low side.
    let run = |walls: Vec<(i64, i64, i64, bool)>,
               swap: bool,
               push: &mut dyn FnMut(f64, f64, f64, f64, f64),
               seen: &mut HashSet<Span>| {
        let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, &(x, lo, hi, near)) in walls.iter().enumerate() {
            if !near {
                for y in [lo, hi] {
                    grid.entry((cell(x), cell(y))).or_default().push(i);
                }
            }
        }
        for &(x, lo, hi, near) in &walls {
            if !near {
                continue;
            }
            let mut tried: HashSet<usize> = HashSet::new();
            for y in [lo, hi] {
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let Some(v) = grid.get(&(cell(x) + dx, cell(y) + dy)) else {
                            continue;
                        };
                        for &j in v {
                            if !tried.insert(j) {
                                continue;
                            }
                            let (rx, rlo, rhi, _) = walls[j];
                            let across = rx - x;
                            if across <= 0 || across >= l {
                                continue; // behind, or too far to be under the limit
                            }
                            // The gap along the walls between their near ends; negative
                            // means they share a stretch, which the band sweep measured.
                            let gap = (rlo - hi).max(lo - rhi);
                            if gap < 0 {
                                continue;
                            }
                            let dist2 =
                                (across as i128) * (across as i128) + (gap as i128) * (gap as i128);
                            if dist2 == 0 || !limit.broken_by_sq(dist2, 1) {
                                continue;
                            }
                            let (p, q) = if rlo >= hi {
                                ((x, hi), (rx, rlo))
                            } else {
                                ((x, lo), (rx, rhi))
                            };
                            let (p, q) = if swap {
                                ((p.1, p.0), (q.1, q.0))
                            } else {
                                (p, q)
                            };
                            let (mx, my) = ((p.0 + q.0) as f64 * 0.5, (p.1 + q.1) as f64 * 0.5);
                            if !core.owns(mx, my) {
                                continue;
                            }
                            // A diagonal neck has a vertical and a horizontal wall at
                            // each of its two corners, so both passes find it; the
                            // span is filed by its ends so that it is written once.
                            if seen.insert((p.min(q), p.max(q))) {
                                push(
                                    p.0 as f64,
                                    p.1 as f64,
                                    q.0 as f64,
                                    q.1 as f64,
                                    (dist2 as f64).sqrt(),
                                );
                            }
                        }
                    }
                }
            }
        }
    };
    let mut seen: HashSet<Span> = HashSet::new();
    run(
        vedges
            .iter()
            .map(|v| (v.x as i64, v.ylo as i64, v.yhi as i64, v.left_wall))
            .collect(),
        false,
        push_edge,
        &mut seen,
    );
    run(
        hedges
            .iter()
            .map(|h| (h.y as i64, h.xlo as i64, h.xhi as i64, h.bottom_wall))
            .collect(),
        true,
        push_edge,
        &mut seen,
    );
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

/// The closest approach of a point to a segment, exactly: the squared distance as
/// `num / den`, the closest point, and the vector from the point to it scaled by `den` -
/// a facing test needs only the direction, and `den` is positive, so the scale keeps the
/// sign and keeps it an integer.
struct Approach {
    num: i128,
    den: i128,
    at: (f64, f64),
    towards: (i128, i128),
}

fn point_seg_sq(p: (i64, i64), q0: (i64, i64), q1: (i64, i64)) -> Approach {
    let (dx, dy) = ((q1.0 - q0.0) as i128, (q1.1 - q0.1) as i128);
    let len2 = dx * dx + dy * dy;
    let (wx, wy) = ((p.0 - q0.0) as i128, (p.1 - q0.1) as i128);
    let t = wx * dx + wy * dy;
    let endpoint = |q: (i64, i64)| {
        let v = ((q.0 - p.0) as i128, (q.1 - p.1) as i128);
        Approach {
            num: v.0 * v.0 + v.1 * v.1,
            den: 1,
            at: (q.0 as f64, q.1 as f64),
            towards: v,
        }
    };
    if len2 == 0 || t <= 0 {
        return endpoint(q0);
    }
    if t >= len2 {
        return endpoint(q1);
    }
    // Inside the segment: the foot of the perpendicular, at fraction `t / len2` along.
    let cross = wx * dy - wy * dx;
    let f = t as f64 / len2 as f64;
    Approach {
        num: cross * cross,
        den: len2,
        at: (q0.0 as f64 + f * dx as f64, q0.1 as f64 + f * dy as f64),
        towards: (-wx * len2 + t * dx, -wy * len2 + t * dy),
    }
}

/// Closest points of two segments, exactly: the squared distance as `num / den`, the
/// point on each, and the direction from `a`'s point to `b`'s scaled by `den`.
struct Closest {
    num: i128,
    den: i128,
    on_a: (f64, f64),
    on_b: (f64, f64),
    towards: (i128, i128),
}

/// Segments here are polygon edges that never properly cross, so the minimum sits at an
/// endpoint of one projected onto the other, or at a shared endpoint.  The four
/// candidates are ranked as floats - which is nearest is never a close call on real
/// geometry - and the winner keeps its exact ratio for the comparison against the limit.
fn seg_seg_closest_sq(a0: (i64, i64), a1: (i64, i64), b0: (i64, i64), b1: (i64, i64)) -> Closest {
    let mut best: Option<Closest> = None;
    let mut consider = |c: Closest| {
        let d = c.num as f64 / c.den as f64;
        if best
            .as_ref()
            .is_none_or(|b| d < b.num as f64 / b.den as f64)
        {
            best = Some(c);
        }
    };
    for p in [a0, a1] {
        let x = point_seg_sq(p, b0, b1);
        consider(Closest {
            num: x.num,
            den: x.den,
            on_a: (p.0 as f64, p.1 as f64),
            on_b: x.at,
            towards: x.towards,
        });
    }
    for p in [b0, b1] {
        let x = point_seg_sq(p, a0, a1);
        consider(Closest {
            num: x.num,
            den: x.den,
            on_a: x.at,
            on_b: (p.0 as f64, p.1 as f64),
            towards: (-x.towards.0, -x.towards.1),
        });
    }
    best.expect("four candidates")
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
/// a vertex and so measure zero, which drops them along with coincident-edge noise the
/// other passes filter the same way.
fn mixed_widths(
    oedges: &[OEdge],
    vedges: &[VEdge],
    hedges: &[HEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    limit: Limit,
) {
    for o in oedges {
        let (a, b) = ((o.ax as i64, o.ay as i64), (o.bx as i64, o.by as i64));
        let (dx, dy) = ((b.0 - a.0) as i128, (b.1 - a.1) as i128);
        if dx == 0 && dy == 0 {
            continue;
        }
        // Interior normal of the oblique edge, unnormalised: metal is on the left of a → b.
        let (nox, noy) = (-dy, dx);

        // (segment endpoints, interior normal) for every axis-aligned edge.
        let axis = vedges
            .iter()
            .map(|v| {
                let n = if v.left_wall { (1, 0) } else { (-1, 0) };
                ((v.x as i64, v.ylo as i64), (v.x as i64, v.yhi as i64), n)
            })
            .chain(hedges.iter().map(|h| {
                let n = if h.bottom_wall { (0, 1) } else { (0, -1) };
                ((h.xlo as i64, h.y as i64), (h.xhi as i64, h.y as i64), n)
            }));

        for (q0, q1, (nax, nay)) in axis {
            let c = seg_seg_closest_sq(a, b, q0, q1);
            if c.num == 0 || !limit.broken_by_sq(c.num, c.den) {
                continue;
            }
            // Each edge's interior must face the other, or the gap is outside the shape.
            let (vx, vy) = c.towards;
            if vx * nox + vy * noy <= 0 || -vx * nax - vy * nay <= 0 {
                continue;
            }
            let (mx, my) = ((c.on_a.0 + c.on_b.0) * 0.5, (c.on_a.1 + c.on_b.1) * 0.5);
            if !core.owns(mx, my) {
                continue;
            }
            let dist = (c.num as f64 / c.den as f64).sqrt();
            push_edge(c.on_a.0, c.on_a.1, c.on_b.0, c.on_b.1, dist);
        }
    }
}

/// Oblique widths: anti-parallel edge pairs with metal between them.  A pair is only
/// reported when the parallel run exceeds `min_run` DBU — small chamfers are ignored, and
/// a 45°-bent-width rule can require a minimum bent length.
///
/// Two edges pair when they are anti-parallel *as far as the grid can say*.  A boolean
/// cuts a 45° wall and rounds its new end to the DBU, so the two sides of one bar come
/// out as (1160, 1160) and (1160, 1161): 0.05° apart, and a test for an exact cross
/// product of zero paired nothing and measured nothing between them - the miss that made
/// PL.7's own fixture read clean.  What matters is whether the gap stays put along the
/// run the two share.  The cross product over the product of the lengths is the sine of
/// the angle, so times the shorter run it is how far the far end drifts, and a drift of
/// a DBU and a half is a straight gap on this grid: `|cross| · min(li, lj) / (li · lj) ≤
/// 1.5`, which is `4·cross² ≤ 9·max(len2)` in integers.  The gap then varies by under
/// that along the run, and the rule is read at its worst: the nearer end for a minimum,
/// the farther for a maximum.
///
/// The perpendicular distance is `c / |d|` for an integer `c`, so it is compared squared;
/// the run they share is measured along `d` in units of `|d|²`, and compared squared the
/// same way.
fn oblique_widths(
    oedges: &[OEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    limit: Limit,
    min_run: i64,
    walls: Option<&WallFilter>,
) {
    let min_run2 = (min_run as i128) * (min_run as i128);
    let n = oedges.len();
    for i in 0..n {
        let ei = &oedges[i];
        let (dix, diy) = ((ei.bx - ei.ax) as i128, (ei.by - ei.ay) as i128);
        let len2 = dix * dix + diy * diy;
        if len2 == 0 {
            continue;
        }
        let li = (len2 as f64).sqrt();
        let (ux, uy) = (dix as f64 / li, diy as f64 / li);
        let (nx, ny) = (-diy as f64 / li, dix as f64 / li);
        for ej in &oedges[i + 1..] {
            let (djx, djy) = ((ej.bx - ej.ax) as i128, (ej.by - ej.ay) as i128);
            let lenj2 = djx * djx + djy * djy;
            let cross = dix * djy - diy * djx;
            if 4 * cross * cross > 9 * len2.max(lenj2) || dix * djx + diy * djy >= 0 {
                continue;
            }
            // Signed distance of each end of ej from ei's line, along ei's interior
            // normal, times |d|: positive means ej lies on the material side.  The two
            // differ only by the drift, and the rule reads the worse one.
            let (wx, wy) = ((ej.ax - ei.ax) as i128, (ej.ay - ei.ay) as i128);
            let (vx, vy) = ((ej.bx - ei.ax) as i128, (ej.by - ei.ay) as i128);
            let (ca, cb) = (dix * wy - diy * wx, dix * vy - diy * vx);
            let c = match limit {
                Limit::AtMost(_) => ca.max(cb),
                Limit::AtLeast(_) | Limit::Exactly(_) => ca.min(cb),
            };
            if ca <= 0 || cb <= 0 || !limit.broken_by_sq(c * c, len2) {
                continue;
            }
            // Where ej's ends project onto ei, in units of len2 along d.
            let taj = wx * dix + wy * diy;
            let tbj = vx * dix + vy * diy;
            let lo = taj.min(tbj).max(0);
            let hi = taj.max(tbj).min(len2);
            let run = hi - lo;
            if run <= 0 {
                // No shared stretch: two bars end to end, offset.  A minimum still reads
                // the distance between their nearest ends, as `corner_widths` does for
                // axis-aligned walls, when the filter is not choosing walls.
                if walls.is_none() && matches!(limit, Limit::AtLeast(_)) {
                    let ends_i = [
                        (ei.ax as i128, ei.ay as i128),
                        (ei.bx as i128, ei.by as i128),
                    ];
                    let ends_j = [
                        (ej.ax as i128, ej.ay as i128),
                        (ej.bx as i128, ej.by as i128),
                    ];
                    let mut best: Option<EndPair> = None;
                    for p in ends_i {
                        for q in ends_j {
                            let d2 = (q.0 - p.0) * (q.0 - p.0) + (q.1 - p.1) * (q.1 - p.1);
                            if best.is_none_or(|b| d2 < b.0) {
                                best = Some((d2, p, q));
                            }
                        }
                    }
                    let (d2, p, q) = best.expect("two ends each");
                    if d2 > 0 && limit.broken_by_sq(d2, 1) {
                        let (mx, my) = ((p.0 + q.0) as f64 * 0.5, (p.1 + q.1) as f64 * 0.5);
                        if core.owns(mx, my) {
                            push_edge(
                                p.0 as f64,
                                p.1 as f64,
                                q.0 as f64,
                                q.1 as f64,
                                (d2 as f64).sqrt(),
                            );
                        }
                    }
                }
                continue;
            }
            if run * run <= min_run2 * len2 {
                continue;
            }
            let dist = c as f64 / li;
            // Everything from here is in DBU along ei, measured from its start.
            let (lo_f, hi_f) = (lo as f64 / li, hi as f64 / li);
            let (taj_f, tbj_f) = (taj as f64 / li, tbj as f64 / li);
            let stretches = match walls {
                None => vec![(lo_f, hi_f)],
                Some(f) => {
                    let on_i: Vec<(f64, f64)> = f
                        .keep_o((ei.ax as i64, ei.ay as i64), (ei.bx as i64, ei.by as i64))
                        .into_iter()
                        .map(|(a, b)| (a * li, b * li))
                        .collect();
                    // ej runs the other way: its fraction s sits at taj + s·(tbj − taj).
                    let on_j: Vec<(f64, f64)> = union(
                        f.keep_o((ej.ax as i64, ej.ay as i64), (ej.bx as i64, ej.by as i64))
                            .into_iter()
                            .map(|(a, b)| {
                                let (p, q) =
                                    (taj_f + a * (tbj_f - taj_f), taj_f + b * (tbj_f - taj_f));
                                (p.min(q), p.max(q))
                            })
                            .collect(),
                    );
                    intersect(&intersect(&on_i, &on_j), &[(lo_f, hi_f)])
                }
            };
            for (s0, s1) in stretches {
                let mid = (s0 + s1) * 0.5;
                let mx = ei.ax as f64 + mid * ux + nx * dist * 0.5;
                let my = ei.ay as f64 + mid * uy + ny * dist * 0.5;
                if !core.owns(mx, my) {
                    continue;
                }
                push_edge(
                    ei.ax as f64 + s0 * ux,
                    ei.ay as f64 + s0 * uy,
                    ei.ax as f64 + s1 * ux,
                    ei.ay as f64 + s1 * uy,
                    dist,
                );
                let span = tbj_f - taj_f;
                let (f_lo, f_hi) = ((s0 - taj_f) / span, (s1 - taj_f) / span);
                push_edge(
                    ej.ax as f64 + f_lo * djx as f64,
                    ej.ay as f64 + f_lo * djy as f64,
                    ej.ax as f64 + f_hi * djx as f64,
                    ej.ay as f64 + f_hi * djy as f64,
                    dist,
                );
            }
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

#[cfg(test)]
mod width_tests {
    use super::*;

    fn poly(pts: &[(i32, i32)]) -> MergedPoly {
        MergedPoly {
            outer: pts.iter().map(|&(x, y)| IntPoint::new(x, y)).collect(),
            holes: vec![],
        }
    }

    /// Walls reported on `p` under `limit`, every pass on.
    fn walls(p: &MergedPoly, limit: Limit) -> usize {
        let core = Core {
            x0: -1_000_000,
            y0: -1_000_000,
            x1: 1_000_000,
            y1: 1_000_000,
        };
        width_pairs(p, core, limit, None, false, true, 0).len()
    }

    /// 0.15 / 0.001 is 149.99999999999997 in floating point.  That is 150 on the grid,
    /// and a value genuinely off the grid rounds the way that keeps the rule's meaning.
    #[test]
    fn a_micron_limit_lands_on_the_grid() {
        assert_eq!(Limit::at_least(0.15, 0.001), Limit::AtLeast(150));
        assert_eq!(Limit::at_most(0.15, 0.001), Limit::AtMost(150));
        assert_eq!(Limit::exactly(0.15, 0.001), Limit::Exactly(150));
        assert_eq!(Limit::at_least(0.1504, 0.001), Limit::AtLeast(151));
        assert_eq!(Limit::at_most(0.1504, 0.001), Limit::AtMost(150));
    }

    /// A span equal to the limit satisfies a minimum and a maximum alike, and one DBU
    /// past it breaks them - on the box fast path and through the sweep.
    #[test]
    fn a_span_on_the_limit_is_on_the_right_side_of_it() {
        let boxed = |w: i32, h: i32| poly(&[(0, 0), (w, 0), (w, h), (0, h)]);
        assert_eq!(walls(&boxed(150, 1000), Limit::AtLeast(150)), 0);
        assert_eq!(walls(&boxed(149, 1000), Limit::AtLeast(150)), 2);
        assert_eq!(walls(&boxed(150, 150), Limit::AtMost(150)), 0);
        assert_eq!(walls(&boxed(151, 150), Limit::AtMost(150)), 2);
        assert_eq!(walls(&boxed(150, 150), Limit::Exactly(150)), 0);
        assert_eq!(walls(&boxed(150, 151), Limit::Exactly(150)), 2);
        // An L with two arms `w` wide: not a box, so the sweep measures it.
        let ell = |w: i32| poly(&[(0, 0), (1000, 0), (1000, w), (w, w), (w, 1000), (0, 1000)]);
        assert_eq!(walls(&ell(150), Limit::AtLeast(150)), 0);
        assert_eq!(walls(&ell(150), Limit::AtLeast(151)), 4);
    }

    /// An oblique width is a square root and is compared squared, so it is exact too.
    /// A 45° bar with square ends whose walls are offset by (-113, 113) is 159.81 wide:
    /// under 160 by a fifth of a DBU, which is a violation and not noise; offset by
    /// (-114, 114) it is 161.2 and passes.  A bar along (300, 400) whose walls are offset
    /// by (-128, 96) is exactly 160 wide - the cross product is 80000 over a length of
    /// 500 - and passes.
    #[test]
    fn an_oblique_span_is_compared_exactly() {
        let bar = |k: i32| poly(&[(0, 0), (1000, 1000), (1000 - k, 1000 + k), (-k, k)]);
        assert_eq!(walls(&bar(113), Limit::AtLeast(160)), 2);
        assert_eq!(walls(&bar(114), Limit::AtLeast(160)), 0);
        let exact = poly(&[(0, 0), (300, 400), (172, 496), (-128, 96)]);
        assert_eq!(walls(&exact, Limit::AtLeast(160)), 0);
        assert_eq!(walls(&exact, Limit::AtLeast(161)), 2);
    }
}

#[cfg(test)]
mod wall_filter_tests {
    use super::*;

    fn poly(pts: &[(i32, i32)]) -> MergedPoly {
        MergedPoly {
            outer: pts.iter().map(|&(x, y)| IntPoint::new(x, y)).collect(),
            holes: vec![],
        }
    }

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// Wall markers on `body` under `limit`, between the walls the filter keeps.
    fn walls(body: &MergedPoly, limit: Limit, f: &WallFilter) -> Vec<(f64, f64, f64, f64, f64)> {
        let core = Core {
            x0: -1_000_000,
            y0: -1_000_000,
            x1: 1_000_000,
            y1: 1_000_000,
        };
        width_pairs(body, core, limit, Some(f), false, false, 0)
    }

    /// A poly stripe 300 tall crossing a channel mask 200 wide: the stripe's two long
    /// walls are shared with the mask's boundary over the crossing and nowhere else, so
    /// the gate length is measured there and cut to it.  The stripe's own width is 300;
    /// a 301 rule sees the gate, a 300 rule does not.
    #[test]
    fn a_gate_is_measured_where_the_walls_share_the_mask() {
        let stripe = rect(0, 0, 1000, 300);
        let mask = [rect(400, 0, 600, 300)];
        let f = WallFilter::new(&mask, true, None);
        let v = walls(&stripe, Limit::AtLeast(301), &f);
        assert_eq!(v.len(), 2, "{v:?}");
        for (x1, _, x2, _, w) in &v {
            assert_eq!((x1.min(*x2), x1.max(*x2), *w), (400.0, 600.0, 300.0));
        }
        assert!(walls(&stripe, Limit::AtLeast(300), &f).is_empty());
    }

    /// The gate at the far end of a long stripe, nowhere near the stripe's own midpoint:
    /// a midpoint mask missed it, a wall filter finds it.
    #[test]
    fn a_gate_far_from_the_stripes_midpoint_is_found() {
        let stripe = rect(0, 0, 20_000, 300);
        let mask = [rect(19_000, 0, 19_200, 300)];
        let f = WallFilter::new(&mask, true, None);
        assert_eq!(walls(&stripe, Limit::AtLeast(301), &f).len(), 2);
    }

    /// An LDMOS body 1000 by 300 whose short walls lie on the active's boundary: the
    /// shared walls give the device width (1000), the unshared ones the channel length
    /// (300), and a rule reading one never sees the other.
    #[test]
    fn shared_and_unshared_walls_measure_the_two_directions() {
        let body = rect(0, 0, 1000, 300);
        let active = [rect(-500, -100, 0, 400), rect(1000, -100, 1500, 400)];
        let shared = WallFilter::new(&active, true, None);
        let unshared = WallFilter::new(&active, false, None);
        assert_eq!(walls(&body, Limit::AtMost(999), &shared).len(), 2);
        assert!(walls(&body, Limit::AtLeast(301), &shared).is_empty());
        assert_eq!(walls(&body, Limit::AtLeast(301), &unshared).len(), 2);
        assert!(walls(&body, Limit::AtMost(999), &unshared).is_empty());
    }

    /// A native gate under a well on one half of its crossing: the stretch under the well
    /// is cut away, and the marker covers only what is left.
    #[test]
    fn stretches_under_the_excluded_region_are_cut() {
        let stripe = rect(0, 0, 1000, 300);
        let mask = [rect(400, 0, 600, 300)];
        let well = [rect(500, -100, 2000, 400)];
        let f = WallFilter::new(&mask, true, Some(&well));
        let v = walls(&stripe, Limit::AtLeast(301), &f);
        assert_eq!(v.len(), 2, "{v:?}");
        for (x1, _, x2, _, _) in &v {
            assert_eq!((x1.min(*x2), x1.max(*x2)), (400.0, 500.0));
        }
        let all = [rect(0, -100, 2000, 400)];
        let f = WallFilter::new(&mask, true, Some(&all));
        assert!(walls(&stripe, Limit::AtLeast(301), &f).is_empty());
    }
}
