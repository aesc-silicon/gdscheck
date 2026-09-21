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

/// How far past its own bound an *enclosure* maximum looks for the outer wall, in
/// multiples of the bound.  An enclosure's margin is bounded by the outer wall nearest
/// the inner one; past a small multiple of the bound what the search finds is the far
/// side of the enclosing shape, which never bounded this margin at all - the check would
/// rather miss a margin than invent one to a wall that is not really opposite.  On
/// GF180's MDN.10c a reach of four invents eighteen markers where two invents none.
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

/// What a pair of facing walls has between them: the material of a width, or the empty
/// ground of a notch.  One scan reads both - a notch is the width scan with every wall
/// facing the other way, a slot in a shape or a thin hole through it - and only the
/// facing test and the marker differ.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Between {
    Material,
    Empty,
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
/// rules that filter do not ask for it.  `min_run` is the stretch two walls must share -
/// the projection of one onto the other, taken over the whole walls and not the band the
/// sweep happens to be in - before their width counts at all: a rule that binds only
/// lines longer than so much.  Under a [`WallFilter`] the run is the stretch the filter
/// kept, since that stretch is the thing being measured.  With a run required, the
/// readings that have none (across a corner, at a pinch, at an acute tip, between a
/// chamfer and a wall) are off.
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
    facing_pairs(
        poly,
        core,
        limit,
        walls,
        oblique_only,
        mixed,
        min_run,
        Between::Material,
    )
}

/// Every notch of `poly` narrower than `limit` - the empty gap between two walls of one
/// region facing away from each other, a slot or a thin hole - as one span across the
/// gap, `(x1, y1, x2, y2, gap)` in DBU.  The width scan read the other way round: the
/// same sweeps, the same oblique and mixed passes, with material on the far side of
/// each wall.  What a notch has no use for is left out: a box has none, and the corner
/// and acute-tip readings are about material narrowing to nothing.
pub fn notch_pairs(
    poly: &MergedPoly,
    core: Core,
    limit: Limit,
    oblique_only: bool,
    mixed: bool,
    min_run: i64,
) -> Vec<(f64, f64, f64, f64, f64)> {
    facing_pairs(
        poly,
        core,
        limit,
        None,
        oblique_only,
        mixed,
        min_run,
        Between::Empty,
    )
}

#[allow(clippy::too_many_arguments)]
fn facing_pairs(
    poly: &MergedPoly,
    core: Core,
    limit: Limit,
    walls: Option<&WallFilter>,
    oblique_only: bool,
    mixed: bool,
    min_run: i64,
    between: Between,
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut out = Vec::new();
    let empty = between == Between::Empty;
    // A wall pair faces across material when the near wall has it on its far side and
    // the far wall on its near side; across a notch, the other way round.
    let facing = |near_has_far: bool, far_has_far: bool| {
        if empty {
            !near_has_far && far_has_far
        } else {
            near_has_far && !far_has_far
        }
    };
    // An axis-aligned rectangle - every via, most contacts - has one width and one
    // height, and the sweep below would find exactly the two pairs the box gives
    // directly.  Same pairs, same order, same ownership test, without the sweep.  A
    // filtered scan goes through the sweep, which is where the cutting happens.
    if walls.is_none()
        && let Some((x0, y0, x1, y1)) = axis_rect(poly)
    {
        if oblique_only || empty {
            return out;
        }
        let (cx, cy) = ((x0 + x1) as f64 * 0.5, (y0 + y1) as f64 * 0.5);
        if core.owns(cx, cy) {
            let (w, h) = ((x1 - x0) as i64, (y1 - y0) as i64);
            if w > 0 && h > min_run && limit.broken_by(w) {
                out.push((x0 as f64, y0 as f64, x0 as f64, y1 as f64, w as f64));
                out.push((x1 as f64, y0 as f64, x1 as f64, y1 as f64, w as f64));
            }
            if h > 0 && w > min_run && limit.broken_by(h) {
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
        // The sweep reads the polygon band by band between the ends of its vertical
        // edges, and a pair of walls facing across several bands - any edge elsewhere
        // in the polygon ends a band - is one pair: its stretches are joined before
        // it is reported, so a notch is one report whatever else the polygon's copy
        // in this tile happens to hold.
        let y_events = sorted_unique(vedges.iter().flat_map(|e| [e.ylo, e.yhi]).collect());
        let mut runs: Vec<(PairKey, Vec<(i64, i64)>)> = Vec::new();
        let mut run_at: HashMap<PairKey, usize> = HashMap::new();
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
                if facing(l.left_wall, r.left_wall) {
                    let width = r.x as i64 - l.x as i64;
                    let run = l.yhi.min(r.yhi) as i64 - l.ylo.max(r.ylo) as i64;
                    if width > 0 && (walls.is_some() || run > min_run) && limit.broken_by(width) {
                        let (yb, yb1) = (yb as i64, yb1 as i64);
                        let stretches = match walls {
                            None => vec![(yb, yb1)],
                            Some(f) => intersect(&f.keep_v(l.x, yb, yb1), &f.keep_v(r.x, yb, yb1)),
                        };
                        let key = (l.x, l.ylo, l.yhi, r.x, r.ylo, r.yhi);
                        let i = *run_at.entry(key).or_insert_with(|| {
                            runs.push((key, Vec::new()));
                            runs.len() - 1
                        });
                        runs[i].1.extend(stretches);
                    }
                }
            }
        }
        for ((lx, _, _, rx, _, _), stretches) in runs {
            let width = rx as i64 - lx as i64;
            let cx = (lx as f64 + rx as f64) * 0.5;
            for (s0, s1) in join_stretches(stretches) {
                if walls.is_some() && s1 - s0 <= min_run {
                    continue;
                }
                let cy = (s0 as f64 + s1 as f64) * 0.5;
                if !core.owns(cx, cy) {
                    continue;
                }
                if empty {
                    push_edge(lx as f64, cy, rx as f64, cy, width as f64);
                } else {
                    push_edge(lx as f64, s0 as f64, lx as f64, s1 as f64, width as f64);
                    push_edge(rx as f64, s0 as f64, rx as f64, s1 as f64, width as f64);
                }
            }
        }

        // Vertical widths: scan x bands, pair horizontal edges across y.
        let x_events = sorted_unique(hedges.iter().flat_map(|e| [e.xlo, e.xhi]).collect());
        let mut runs: Vec<(PairKey, Vec<(i64, i64)>)> = Vec::new();
        let mut run_at: HashMap<PairKey, usize> = HashMap::new();
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
                if facing(b.bottom_wall, t.bottom_wall) {
                    let height = t.y as i64 - b.y as i64;
                    let run = b.xhi.min(t.xhi) as i64 - b.xlo.max(t.xlo) as i64;
                    if height > 0 && (walls.is_some() || run > min_run) && limit.broken_by(height) {
                        let (xb, xb1) = (xb as i64, xb1 as i64);
                        let stretches = match walls {
                            None => vec![(xb, xb1)],
                            Some(f) => intersect(&f.keep_h(b.y, xb, xb1), &f.keep_h(t.y, xb, xb1)),
                        };
                        let key = (b.y, b.xlo, b.xhi, t.y, t.xlo, t.xhi);
                        let i = *run_at.entry(key).or_insert_with(|| {
                            runs.push((key, Vec::new()));
                            runs.len() - 1
                        });
                        runs[i].1.extend(stretches);
                    }
                }
            }
        }
        for ((by, _, _, ty, _, _), stretches) in runs {
            let height = ty as i64 - by as i64;
            let cy = (by as f64 + ty as f64) * 0.5;
            for (s0, s1) in join_stretches(stretches) {
                if walls.is_some() && s1 - s0 <= min_run {
                    continue;
                }
                let cx = (s0 as f64 + s1 as f64) * 0.5;
                if !core.owns(cx, cy) {
                    continue;
                }
                if empty {
                    push_edge(cx, by as f64, cx, ty as f64, height as f64);
                } else {
                    push_edge(s0 as f64, by as f64, s1 as f64, by as f64, height as f64);
                    push_edge(s0 as f64, ty as f64, s1 as f64, ty as f64, height as f64);
                }
            }
        }
    } // end !oblique_only

    if mixed && min_run == 0 {
        mixed_widths(
            &oedges,
            &vedges,
            &hedges,
            core,
            &mut push_edge,
            limit,
            between,
        );
    }
    if !oblique_only
        && !empty
        && walls.is_none()
        && min_run == 0
        && matches!(limit, Limit::AtLeast(_))
    {
        corner_widths(&vedges, &hedges, core, &mut push_edge, limit);
        acute_corners(poly, core, &mut push_edge);
    }
    oblique_widths(
        &oedges,
        core,
        &mut push_edge,
        limit,
        min_run,
        walls,
        between,
    );
    out
}

/// Two axis-aligned walls facing each other, by their coordinates.
type PairKey = (i32, i32, i32, i32, i32, i32);

/// Stretches along a line joined where they touch or overlap, in order.
fn join_stretches(mut stretches: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    stretches.sort_unstable();
    let mut out: Vec<(i64, i64)> = Vec::new();
    for (s0, s1) in stretches {
        match out.last_mut() {
            Some(last) if s0 <= last.1 => last.1 = last.1.max(s1),
            _ => out.push((s0, s1)),
        }
    }
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
#[allow(clippy::too_many_arguments)]
fn mixed_widths(
    oedges: &[OEdge],
    vedges: &[VEdge],
    hedges: &[HEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    limit: Limit,
    between: Between,
) {
    let empty = between == Between::Empty;
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
            // Each edge's interior must face the other, or the gap is outside the shape -
            // and for a notch each must face away, or the gap is inside it.
            let (vx, vy) = c.towards;
            let (toward_o, toward_a) = (vx * nox + vy * noy, -vx * nax - vy * nay);
            let facing = if empty {
                toward_o < 0 && toward_a < 0
            } else {
                toward_o > 0 && toward_a > 0
            };
            if !facing {
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
#[allow(clippy::too_many_arguments)]
fn oblique_widths(
    oedges: &[OEdge],
    core: Core,
    push_edge: &mut impl FnMut(f64, f64, f64, f64, f64),
    limit: Limit,
    min_run: i64,
    walls: Option<&WallFilter>,
    between: Between,
) {
    let empty = between == Between::Empty;
    // Across a notch the other edge lies on the exterior side, where the signed
    // distances come out negative; read them the other way up.
    let sign: i128 = if empty { -1 } else { 1 };
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
            let (ca, cb) = (sign * (dix * wy - diy * wx), sign * (dix * vy - diy * vx));
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
                // axis-aligned walls, when the filter is not choosing walls and no run is
                // required.
                if walls.is_none() && !empty && min_run == 0 && matches!(limit, Limit::AtLeast(_)) {
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
            // `length` on a 45° pair is each wall's own length, not the stretch the two
            // share: "a bend longer than 0.39" is the bend's wall, and IHP's deck takes
            // the edges of that length and measures between them.  A 45° band drawn
            // with square ends has walls offset along their run by the band's width,
            // so the shared stretch is shorter than either wall by that, and a 0.396
            // bend read as one of 0.237.
            if walls.is_none() && (len2 < min_run2 || lenj2 < min_run2) {
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
                if walls.is_some() && s1 - s0 <= min_run as f64 {
                    continue;
                }
                let mid = (s0 + s1) * 0.5;
                let side = sign as f64;
                let mx = ei.ax as f64 + mid * ux + side * nx * dist * 0.5;
                let my = ei.ay as f64 + mid * uy + side * ny * dist * 0.5;
                if !core.owns(mx, my) {
                    continue;
                }
                if empty {
                    let (px, py) = (ei.ax as f64 + mid * ux, ei.ay as f64 + mid * uy);
                    push_edge(px, py, px - nx * dist, py - ny * dist, dist);
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

/// A box in DBU, `(x0, y0, x1, y1)`: the zone a tile's copies are exact in.
pub type Zone = (i64, i64, i64, i64);

/// The depth behind a wall along it: stretches `(from, to)` in units of the wall's
/// squared length, each with the nearest anti-parallel wall's separation times the
/// length, or `None` where nothing faces it.
pub type DepthProfile = Vec<(i128, i128, Option<i128>)>;

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

    /// A bar with a stub on top: the stub splits the sweep's bands, but the run two walls
    /// share is the projection of the whole walls, so a `min_run` reads the bar's length
    /// and not the band's.  The bottom wall and the top wall left of the stub share 400.
    #[test]
    fn the_run_is_the_walls_projection_not_the_band() {
        let bar = poly(&[
            (0, 0),
            (1000, 0),
            (1000, 150),
            (600, 150),
            (600, 300),
            (400, 300),
            (400, 150),
            (0, 150),
        ]);
        let core = Core {
            x0: -1_000_000,
            y0: -1_000_000,
            x1: 1_000_000,
            y1: 1_000_000,
        };
        let with = |run: i64| width_pairs(&bar, core, Limit::AtLeast(151), None, false, true, run);
        // Both 150-high stretches beside the stub, two walls each.
        assert_eq!(with(0).len(), 4);
        assert_eq!(with(399).len(), 4);
        assert_eq!(with(400).len(), 0);
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

// ===========================================================================
// Exact spacing between two regions, in DBU.
//
// The closest approach of two regions is the closest approach of two of their boundary
// segments, and on integer coordinates that is exact: an axis-aligned gap is a
// difference, a corner-to-corner gap a square root, compared squared as a ratio of two
// integers.  Whether two regions overlap, touch at a point or abut along a run is a
// matter of signs of cross products, and never of a tolerance.
// ===========================================================================

/// A boundary segment in DBU.
pub type Seg = ((i64, i64), (i64, i64));

/// A region's boundary as integer segments - outer ring and holes alike, since both are
/// material boundary - with its box, for exact tests against another.
pub struct Outline<'a> {
    poly: &'a MergedPoly,
    segs: Vec<Seg>,
    /// `(x0, y0, x1, y1)`.
    pub bbox: (i64, i64, i64, i64),
    /// The segments filed by cell, built on the first query that pays for it.
    grid: std::cell::OnceCell<SegGrid>,
    /// The depth profiles read so far, by segment: a rail's facing wall is asked for
    /// its profile by every wire beside it.
    depths: std::cell::RefCell<HashMap<usize, std::rc::Rc<DepthProfile>>>,
}

/// An outline's segments filed under the cells of a grid over its box, so a query
/// near one spot of a plate with thousands of walls reads the few walls there.  A
/// segment lies in every cell its own box touches, so a long wall is filed along its
/// run and a query may meet it from several cells; the callers take a minimum, which
/// a repeat does not move.
struct SegGrid {
    x0: i64,
    y0: i64,
    cell: i64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

/// Fewer walls than this are read straight, the grid costing more than it saves.
const GRID_FROM: usize = 48;

impl SegGrid {
    fn build(segs: &[Seg], (x0, y0, x1, y1): (i64, i64, i64, i64)) -> Self {
        let n = ((segs.len() as f64).sqrt().ceil() as usize).clamp(1, 64);
        let cell = ((x1 - x0).max(y1 - y0) / n as i64).max(1);
        let nx = ((x1 - x0) / cell + 1) as usize;
        let ny = ((y1 - y0) / cell + 1) as usize;
        let mut cells = vec![Vec::new(); nx * ny];
        for (i, &(a, b)) in segs.iter().enumerate() {
            let (cx0, cx1) = ((a.0.min(b.0) - x0) / cell, (a.0.max(b.0) - x0) / cell);
            let (cy0, cy1) = ((a.1.min(b.1) - y0) / cell, (a.1.max(b.1) - y0) / cell);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    cells[cy as usize * nx + cx as usize].push(i as u32);
                }
            }
        }
        SegGrid {
            x0,
            y0,
            cell,
            nx,
            ny,
            cells,
        }
    }

    /// Every segment index filed in a cell the box touches, repeats included.
    fn near(&self, (bx0, by0, bx1, by1): (i64, i64, i64, i64), mut f: impl FnMut(u32)) {
        let cx0 = ((bx0 - self.x0) / self.cell).clamp(0, self.nx as i64 - 1);
        let cx1 = ((bx1 - self.x0) / self.cell).clamp(0, self.nx as i64 - 1);
        let cy0 = ((by0 - self.y0) / self.cell).clamp(0, self.ny as i64 - 1);
        let cy1 = ((by1 - self.y0) / self.cell).clamp(0, self.ny as i64 - 1);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                for &i in &self.cells[cy as usize * self.nx + cx as usize] {
                    f(i);
                }
            }
        }
    }
}

fn cross_i(o: (i64, i64), a: (i64, i64), b: (i64, i64)) -> i128 {
    (a.0 - o.0) as i128 * (b.1 - o.1) as i128 - (a.1 - o.1) as i128 * (b.0 - o.0) as i128
}

/// Whether `p` lies on the segment, ends included.
fn on_segment(p: (i64, i64), (a, b): Seg) -> bool {
    cross_i(a, b, p) == 0
        && p.0 >= a.0.min(b.0)
        && p.0 <= a.0.max(b.0)
        && p.1 >= a.1.min(b.1)
        && p.1 <= a.1.max(b.1)
}

/// Whether `p` lies inside `ring` by the crossing count of a ray to the right: an edge
/// counts when its ends straddle the ray's height and the crossing lies right of `p`.
/// The crossing's x is `xi + (py - yi)(xj - xi)/(yj - yi)`, and `px < x` is a sign test
/// on integers once multiplied out.  A point on the ring is on no side in particular
/// here; [`Outline`] asks about the boundary separately.
fn in_ring(p: (i64, i64), ring: &[IntPoint]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let (px, py) = p;
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (ring[i].x as i64, ring[i].y as i64);
        let (xj, yj) = (ring[j].x as i64, ring[j].y as i64);
        if (yi > py) != (yj > py) {
            let num = (py - yi) as i128 * (xj - xi) as i128;
            let den = (yj - yi) as i128;
            let lhs = (px - xi) as i128 * den;
            if (den > 0 && lhs < num) || (den < 0 && lhs > num) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

impl<'a> Outline<'a> {
    pub fn new(poly: &'a MergedPoly) -> Self {
        let mut segs = Vec::new();
        for ring in std::iter::once(&poly.outer).chain(poly.holes.iter()) {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if a != b {
                    segs.push(((a.x as i64, a.y as i64), (b.x as i64, b.y as i64)));
                }
            }
        }
        let (x0, y0, x1, y1) = crate::merge::poly_bbox(poly);
        Outline {
            poly,
            segs,
            bbox: (x0 as i64, y0 as i64, x1 as i64, y1 as i64),
            grid: std::cell::OnceCell::new(),
            depths: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// The region this is the outline of.
    pub fn poly(&self) -> &'a MergedPoly {
        self.poly
    }

    /// The boundary segments, outer ring then holes.
    pub fn segs(&self) -> &[Seg] {
        &self.segs
    }

    /// Call `f` on every segment whose box may come within `reach` of `(bx0, by0,
    /// bx1, by1)`: through the grid on an outline with many walls, straight through
    /// the few of a small one.  A segment may be visited more than once.
    pub fn for_segs_near(
        &self,
        (bx0, by0, bx1, by1): (i64, i64, i64, i64),
        reach: i64,
        mut f: impl FnMut(Seg),
    ) {
        if self.segs.len() < GRID_FROM {
            for &s in &self.segs {
                f(s);
            }
            return;
        }
        let grid = self
            .grid
            .get_or_init(|| SegGrid::build(&self.segs, self.bbox));
        grid.near((bx0 - reach, by0 - reach, bx1 + reach, by1 + reach), |i| {
            f(self.segs[i as usize])
        });
    }

    fn vertices(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        std::iter::once(&self.poly.outer)
            .chain(self.poly.holes.iter())
            .flatten()
            .map(|p| (p.x as i64, p.y as i64))
    }

    fn on_boundary(&self, p: (i64, i64)) -> bool {
        self.segs.iter().any(|&s| on_segment(p, s))
    }

    /// Inside the outer ring and in no hole, off the boundary.
    fn strictly_contains(&self, p: (i64, i64)) -> bool {
        in_ring(p, &self.poly.outer)
            && !self.poly.holes.iter().any(|h| in_ring(p, h))
            && !self.on_boundary(p)
    }

    /// Inside, or on the boundary.
    fn contains_or_on(&self, p: (i64, i64)) -> bool {
        self.on_boundary(p)
            || (in_ring(p, &self.poly.outer) && !self.poly.holes.iter().any(|h| in_ring(p, h)))
    }

    /// Whether the boxes could be within `limit` DBU of each other.
    pub fn possibly_within(&self, other: &Outline, limit: i64) -> bool {
        let (ax0, ay0, ax1, ay1) = self.bbox;
        let (bx0, by0, bx1, by1) = other.bbox;
        (ax0 - bx1).max(bx0 - ax1) < limit && (ay0 - by1).max(by0 - ay1) < limit
    }
}

/// Whether two segments properly cross - each strictly straddles the other's line.
/// Touching at an end is not a crossing.
fn segs_cross_i((p0, p1): Seg, (q0, q1): Seg) -> bool {
    let (d1, d2) = (cross_i(q0, q1, p0), cross_i(q0, q1, p1));
    let (d3, d4) = (cross_i(p0, p1, q0), cross_i(p0, p1, q1));
    ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0))
}

/// Whether two regions share area.  A point on the boundary is shared *boundary*, not
/// shared area: two shapes meeting at one corner do not overlap, and are exactly the
/// pair a spacing rule is about.  Three ways to share area, since a vertex test alone
/// misses the ordinary one: a vertex of one strictly inside the other; one wholly
/// inside the other with boundaries that may coincide, every vertex inside or on and
/// none need be strictly inside - two identical regions being the limiting case; and
/// two boundaries properly crossing, which is what a poly stripe over a COMP does with
/// no vertex of either inside the other.
pub fn regions_overlap(a: &Outline, b: &Outline) -> bool {
    // Boxes with daylight between them hold shapes with daylight between them; boxes
    // that touch may hold shapes that touch, which is a pair this asks about.
    if !a.possibly_within(b, 1) {
        return false;
    }
    // The same shape on both layers - a plate against the wide plates selected out
    // of its own layer - has no vertex strictly inside the other and every vertex on
    // it, which the tests below find only after casting each against every wall.
    if a.bbox == b.bbox && a.segs == b.segs {
        return true;
    }
    if a.vertices().any(|p| b.strictly_contains(p)) || b.vertices().any(|p| a.strictly_contains(p))
    {
        return true;
    }
    if a.vertices().all(|p| b.contains_or_on(p)) || b.vertices().all(|p| a.contains_or_on(p)) {
        return true;
    }
    a.segs
        .iter()
        .any(|&s| b.segs.iter().any(|&t| segs_cross_i(s, t)))
}

/// Whether two regions share a *run* of boundary rather than meeting at isolated points.
/// Shapes drawn edge to edge abut, and a separation of zero between them would be a
/// fiction; shapes meeting at one corner have a gap that happens to be nothing wide.
pub fn share_boundary_run(a: &Outline, b: &Outline) -> bool {
    for &(a0, a1) in &a.segs {
        let d = ((a1.0 - a0.0) as i128, (a1.1 - a0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        for &(b0, b1) in &b.segs {
            let e = ((b1.0 - b0.0) as i128, (b1.1 - b0.1) as i128);
            if d.0 * e.1 - d.1 * e.0 != 0 || cross_i(a0, a1, b0) != 0 {
                continue; // not parallel, or parallel off the line
            }
            // Where the other's ends fall along this one, in units of len2.
            let t0 = (b0.0 - a0.0) as i128 * d.0 + (b0.1 - a0.1) as i128 * d.1;
            let t1 = (b1.0 - a0.0) as i128 * d.0 + (b1.1 - a0.1) as i128 * d.1;
            if t0.max(t1).min(len2) > t0.min(t1).max(0) {
                return true; // a shared stretch, not a shared point
            }
        }
    }
    false
}

/// A closest approach: the squared distance as `num / den`, and the point on each region.
pub type ClosestPair = (i128, i128, (f64, f64), (f64, f64));

/// The closest approach of two regions among the segment pairs that can be under
/// `limit` DBU: the squared distance as `num / den`, and the point on each, in DBU.
/// `None` when every pair is at least `limit` apart.  A pair whose boxes are `limit` or
/// more apart in either axis is at least that far apart and is not looked at; a contact
/// ends the search, since nothing is closer.
pub fn closest_approach(a: &Outline, b: &Outline, limit: i64) -> Option<ClosestPair> {
    let mut best: Option<(ClosestPair, Seg, Seg)> = None;
    let ratio = |n: i128, d: i128| n as f64 / d as f64;
    let boxes_apart = |(p0, p1): Seg, (q0, q1): Seg| {
        (p0.0.min(p1.0) - q0.0.max(q1.0)).max(q0.0.min(q1.0) - p0.0.max(p1.0)) >= limit
            || (p0.1.min(p1.1) - q0.1.max(q1.1)).max(q0.1.min(q1.1) - p0.1.max(p1.1)) >= limit
    };
    for &(a0, a1) in &a.segs {
        let sbox = (
            a0.0.min(a1.0),
            a0.1.min(a1.1),
            a0.0.max(a1.0),
            a0.1.max(a1.1),
        );
        let mut touched = false;
        b.for_segs_near(sbox, limit, |(b0, b1)| {
            if touched || boxes_apart((a0, a1), (b0, b1)) {
                return;
            }
            let c = seg_seg_closest_sq(a0, a1, b0, b1);
            if best
                .as_ref()
                .is_none_or(|&((n, d, _, _), _, _)| ratio(c.num, c.den) < ratio(n, d))
            {
                best = Some(((c.num, c.den, c.on_a, c.on_b), (a0, a1), (b0, b1)));
                touched = c.num == 0;
            }
        });
        if touched {
            break;
        }
    }
    let (pair, sa, sb) = best?;
    // Two walls running alongside are closest all along the stretch they share, and
    // the point the search lands on is whichever end it met first - which depends on
    // the walls' ends as the tile's copy has them.  A shape drawn as two abutting boxes
    // has, in the tile that sees one box, a stretch ending where the other box starts,
    // and the copy's end became a second marker.  The marker is the stretch's lowest
    // end (then leftmost) instead, one point for every copy that sees it, and the
    // tile owning that point reports the pair.
    if pair.0 > 0
        && let Some((on_a, on_b)) = stretch_low_end(sa, sb)
    {
        return Some((pair.0, pair.1, on_a, on_b));
    }
    Some(pair)
}

/// The lowest end (then leftmost) of the stretch two parallel facing walls share, as
/// the point on each: `None` when they are not parallel or share nothing.
fn stretch_low_end((a0, a1): Seg, (b0, b1): Seg) -> Option<((f64, f64), (f64, f64))> {
    let d = ((a1.0 - a0.0) as i128, (a1.1 - a0.1) as i128);
    let e = ((b1.0 - b0.0) as i128, (b1.1 - b0.1) as i128);
    if d.0 * e.1 - d.1 * e.0 != 0 {
        return None;
    }
    let len2 = d.0 * d.0 + d.1 * d.1;
    if len2 == 0 {
        return None;
    }
    let along = |p: (i64, i64)| (p.0 - a0.0) as i128 * d.0 + (p.1 - a0.1) as i128 * d.1;
    let (u0, u1) = (along(b0), along(b1));
    let (lo, hi) = (u0.min(u1).max(0), u0.max(u1).min(len2));
    if hi <= lo {
        return None;
    }
    let at = |u: i128| {
        let t = u as f64 / len2 as f64;
        (a0.0 as f64 + d.0 as f64 * t, a0.1 as f64 + d.1 as f64 * t)
    };
    let (p_lo, p_hi) = (at(lo), at(hi));
    let p = if (p_lo.1, p_lo.0) <= (p_hi.1, p_hi.0) {
        p_lo
    } else {
        p_hi
    };
    // The foot of `p` on the other wall: the offset between the lines, perpendicular.
    let c = cross_i(a0, a1, b0) as f64 / len2 as f64;
    let (nx, ny) = (-(d.1 as f64) * c, d.0 as f64 * c);
    Some((p, (p.0 + nx, p.1 + ny)))
}

/// Whether `a` and `b` run parallel as far as the grid can say - the drift test of
/// [`oblique_widths`], since a boolean cuts a 45° wall and rounds its new end.
fn parallel_i(d: (i128, i128), e: (i128, i128)) -> bool {
    let cross = d.0 * e.1 - d.1 * e.0;
    let (l2, m2) = (d.0 * d.0 + d.1 * d.1, e.0 * e.0 + e.1 * e.1);
    4 * cross * cross <= 9 * l2.max(m2)
}

impl Outline<'_> {
    /// Where a segment's ends fall along `(s0, s1)`, as the shared stretch in units of
    /// the segment's squared length: positive when they overlap in projection.
    pub(crate) fn shared_run((s0, s1): Seg, (t0, t1): Seg) -> i128 {
        let d = ((s1.0 - s0.0) as i128, (s1.1 - s0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        let along = |p: (i64, i64)| (p.0 - s0.0) as i128 * d.0 + (p.1 - s0.1) as i128 * d.1;
        let (u0, u1) = (along(t0), along(t1));
        u0.max(u1).min(len2) - u0.min(u1).max(0)
    }

    /// The material behind segment `i`, stretch by stretch: along the segment, in units
    /// of its squared length, the distance times the length to the nearest anti-parallel
    /// segment of the same region behind that stretch on its material side - the local
    /// line width there - or `None` where nothing faces it.  Material is on the left of
    /// every segment, holes included.  A line 0.2 wide with a 0.5 part on its far side
    /// is 0.5 deep along that part and 0.2 elsewhere, and a rule about wide lines reads
    /// the part.
    fn depth_profile(&self, i: usize) -> std::rc::Rc<DepthProfile> {
        if let Some(p) = self.depths.borrow().get(&i) {
            return p.clone();
        }
        let (s0, s1) = self.segs[i];
        let d = ((s1.0 - s0.0) as i128, (s1.1 - s0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        let along = |p: (i64, i64)| (p.0 - s0.0) as i128 * d.0 + (p.1 - s0.1) as i128 * d.1;
        // The walls behind as events along the segment: a wall opens at its start and
        // closes at its end, and between two events the nearest open wall is the depth.
        let mut events: Vec<(i128, bool, i128)> = Vec::new();
        for (j, &(t0, t1)) in self.segs.iter().enumerate() {
            if j == i {
                continue;
            }
            let e = ((t1.0 - t0.0) as i128, (t1.1 - t0.1) as i128);
            if d.0 * e.0 + d.1 * e.1 >= 0 || !parallel_i(d, e) {
                continue;
            }
            let c = cross_i(s0, s1, t0);
            if c <= 0 {
                continue; // on the empty side
            }
            let (u0, u1) = (along(t0), along(t1));
            let (lo, hi) = (u0.min(u1).max(0), u0.max(u1).min(len2));
            if hi > lo {
                events.push((lo, true, c));
                events.push((hi, false, c));
            }
        }
        // Closings before openings at one position, so a wall ending where the next
        // starts leaves no open gap; the map holds each open depth with its count.
        events.sort_unstable_by_key(|&(u, open, c)| (u, open, c));
        let mut open: std::collections::BTreeMap<i128, usize> = std::collections::BTreeMap::new();
        let mut profile: DepthProfile = Vec::new();
        let mut at: i128 = 0;
        let mut push = |from: i128, to: i128, depth: Option<i128>| {
            if to > from {
                match profile.last_mut() {
                    Some(last) if last.1 == from && last.2 == depth => last.1 = to,
                    _ => profile.push((from, to, depth)),
                }
            }
        };
        for (u, opens, c) in events {
            push(at, u, open.keys().next().copied());
            at = u;
            if opens {
                *open.entry(c).or_insert(0) += 1;
            } else if let Some(n) = open.get_mut(&c) {
                *n -= 1;
                if *n == 0 {
                    open.remove(&c);
                }
            }
        }
        push(at, len2, open.keys().next().copied());
        let p = std::rc::Rc::new(profile);
        self.depths.borrow_mut().insert(i, p.clone());
        p
    }
}

/// Whether a 45° wall of `a` lies within `limit` DBU of `b`: the bend has to be at the
/// gap, not somewhere else on a long net.
pub fn has_diagonal_within(a: &Outline, b: &Outline, limit: i64) -> bool {
    let lim = Limit::AtLeast(limit);
    a.segs
        .iter()
        .filter(|&&(p, q)| p.0 != q.0 && p.1 != q.1)
        .any(|&(p, q)| {
            b.segs.iter().any(|&(r, s)| {
                let c = seg_seg_closest_sq(p, q, r, s);
                lim.broken_by_sq(c.num, c.den)
            })
        })
}

/// Whether some pair of facing walls, one of each region, runs alongside across a gap
/// under `limit` for more than `min_run` DBU with a line deeper than `wide` behind at
/// least one of them - the wide-line spacing rule, read at the gap that is the
/// violation.  Everything is read off the walls themselves: an L-shaped narrow trace
/// has a wide box but a narrow line, and a stepped pad's box overlaps a neighbour for
/// tens of microns while the metal runs alongside for a fraction of that.  A `wide` of
/// zero asks nothing of the depth.
pub fn parallel_run_applies(
    a: &Outline,
    b: &Outline,
    limit: i64,
    wide: i64,
    min_run: i64,
    zone: Option<Zone>,
) -> bool {
    parallel_run(a, b, limit, wide, min_run, zone) == RunRead::Applies
}

/// What [`parallel_run`] found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunRead {
    /// A qualifying run.
    Applies,
    /// No qualifying run, and every facing stretch under the limit lay whole within
    /// the zone: the reading is complete.
    Clean,
    /// No qualifying run within the zone, but a facing stretch under the limit reached
    /// the zone's edge and may carry on past it.
    Cut,
}

/// [`parallel_run_applies`], saying also whether the zone cut a stretch short.
pub fn parallel_run(
    a: &Outline,
    b: &Outline,
    limit: i64,
    wide: i64,
    min_run: i64,
    zone: Option<Zone>,
) -> RunRead {
    // Read along the walls of either region: a stepped wall is several walls, each
    // sharing part of the run with the straight one opposite, and only the straight
    // one sees the run whole.
    match run_along(a, b, limit, wide, min_run, zone) {
        RunRead::Applies => RunRead::Applies,
        first => match run_along(b, a, limit, wide, min_run, zone) {
            RunRead::Clean => first,
            second => second,
        },
    }
}

/// The longest of the intervals once those that touch or overlap are joined.
fn longest_joined(iv: &mut [(i128, i128)]) -> i128 {
    iv.sort_unstable();
    let mut longest = 0;
    let mut cur: Option<(i128, i128)> = None;
    for &(lo, hi) in iv.iter() {
        match cur {
            Some((clo, chi)) if lo <= chi => cur = Some((clo, chi.max(hi))),
            _ => {
                if let Some((clo, chi)) = cur {
                    longest = longest.max(chi - clo);
                }
                cur = Some((lo, hi));
            }
        }
    }
    if let Some((clo, chi)) = cur {
        longest = longest.max(chi - clo);
    }
    longest
}

/// [`parallel_run`] read along the walls of `a`: for each wall, the stretches every
/// facing wall of `b` under the limit shares with it, joined where they touch - a
/// neighbour whose facing wall steps or carries a nick is still one line running
/// alongside - and the depth read over the joined run on either side.
fn run_along(
    a: &Outline,
    b: &Outline,
    limit: i64,
    wide: i64,
    min_run: i64,
    zone: Option<Zone>,
) -> RunRead {
    let mut cut = false;
    let (lim2, wide2, run2) = (
        (limit as i128) * (limit as i128),
        (wide as i128) * (wide as i128),
        (min_run as i128) * (min_run as i128),
    );
    // The stretches of a profile deeper than `wide` within `[lo, hi]`, in the
    // profile's own units; nothing behind at all is as deep as it gets.
    let deep_stretches = |profile: &DepthProfile, len2: i128, lo: i128, hi: i128| {
        profile
            .iter()
            .filter_map(|&(p0, p1, depth)| {
                let (p0, p1) = (p0.max(lo), p1.min(hi));
                let deep = match depth {
                    None => true,
                    Some(c) => c * c > wide2 * len2,
                };
                (p1 > p0 && deep).then_some((p0, p1))
            })
            .collect::<Vec<_>>()
    };
    for (i, &(s0, s1)) in a.segs.iter().enumerate() {
        let d = ((s1.0 - s0.0) as i128, (s1.1 - s0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        if len2 == 0 {
            continue;
        }
        let along_a = |p: (i64, i64)| (p.0 - s0.0) as i128 * d.0 + (p.1 - s0.1) as i128 * d.1;
        // Every facing wall of `b` under the limit: the stretch shared along this
        // wall, and where it lies along that wall.
        let mut shared: Vec<(i128, i128, usize, i128, i128)> = Vec::new();
        for (j, &(t0, t1)) in b.segs.iter().enumerate() {
            let e = ((t1.0 - t0.0) as i128, (t1.1 - t0.1) as i128);
            if d.0 * e.0 + d.1 * e.1 >= 0 || !parallel_i(d, e) {
                continue; // not facing, or not alongside
            }
            // The other wall on this one's empty side, under the limit: `c` is the
            // separation times the length.
            let c = -cross_i(s0, s1, t0);
            if c <= 0 || c * c >= lim2 * len2 {
                continue;
            }
            // The stretch the two walls share, along this one in units of its squared
            // length, and within the zone: the copies are whole shapes, exact in the
            // zone alone, and a run past it is read by the tile that owns it.
            let (ua0, ua1) = (along_a(t0), along_a(t1));
            let (mut lo_a, mut hi_a) = (ua0.min(ua1).max(0), ua0.max(ua1).min(len2));
            if hi_a <= lo_a {
                continue;
            }
            if let Some(z) = zone {
                let (zl, zh) = stretch_in_zone((s0, s1), len2, z);
                if zl > lo_a || zh < hi_a {
                    cut = true;
                }
                lo_a = lo_a.max(zl);
                hi_a = hi_a.min(zh);
            }
            if hi_a <= lo_a {
                continue;
            }
            let lenb2 = e.0 * e.0 + e.1 * e.1;
            let along_b = |p: (i64, i64)| (p.0 - t0.0) as i128 * e.0 + (p.1 - t0.1) as i128 * e.1;
            let (ub0, ub1) = (along_b(s0), along_b(s1));
            let (mut lo_b, mut hi_b) = (ub0.min(ub1).max(0), ub0.max(ub1).min(lenb2));
            if let Some(z) = zone {
                let (zl, zh) = stretch_in_zone((t0, t1), lenb2, z);
                lo_b = lo_b.max(zl);
                hi_b = hi_b.min(zh);
            }
            shared.push((lo_a, hi_a, j, lo_b, hi_b));
        }
        if shared.is_empty() {
            continue;
        }
        // The run is the joined stretch, whichever walls of `b` make it up.
        let mut runs: Vec<(i128, i128)> =
            shared.iter().map(|&(lo, hi, _, _, _)| (lo, hi)).collect();
        let run = longest_joined(&mut runs);
        if run * run <= run2 * len2 {
            continue;
        }
        if wide == 0 {
            return RunRead::Applies;
        }
        // The wide line must run alongside for the length within the joined run: this
        // wall's own depth over it, or the depth of the walls of `b` making it up,
        // each read on its own profile and laid along this wall.
        let (lo, hi) = (
            shared.iter().map(|s| s.0).min().unwrap_or(0),
            shared.iter().map(|s| s.1).max().unwrap_or(0),
        );
        let pa = a.depth_profile(i);
        let mut deep_a = deep_stretches(&pa, len2, lo, hi);
        if longest_joined(&mut deep_a) * longest_joined(&mut deep_a) > run2 * len2 {
            return RunRead::Applies;
        }
        let mut deep_b: Vec<(i128, i128)> = Vec::new();
        for &(lo_a, hi_a, j, lo_b, hi_b) in &shared {
            if hi_b <= lo_b {
                continue;
            }
            let (t0, t1) = b.segs[j];
            let e = ((t1.0 - t0.0) as i128, (t1.1 - t0.1) as i128);
            let lenb2 = e.0 * e.0 + e.1 * e.1;
            let ed = e.0 * d.0 + e.1 * d.1; // negative: the walls run opposite ways
            let base = along_a(t0);
            let pb = b.depth_profile(j);
            for (q0, q1) in deep_stretches(&pb, lenb2, lo_b, hi_b) {
                let (x, y) = (base + ed * q0 / lenb2, base + ed * q1 / lenb2);
                let (x, y) = (x.min(y).max(lo_a), x.max(y).min(hi_a));
                if y > x {
                    deep_b.push((x, y));
                }
            }
        }
        let l = longest_joined(&mut deep_b);
        if l * l > run2 * len2 {
            return RunRead::Applies;
        }
    }
    if cut { RunRead::Cut } else { RunRead::Clean }
}

/// The stretch of segment `s` lying in the box, in units of the segment's squared
/// length `len2` (from `0` at its start to `len2` at its end), rounded inward; empty
/// as `(len2, 0)` when the segment misses the box.
fn stretch_in_zone((s0, s1): Seg, len2: i128, (x0, y0, x1, y1): Zone) -> (i128, i128) {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, d, lo, hi) in [(s0.0, s1.0 - s0.0, x0, x1), (s0.1, s1.1 - s0.1, y0, y1)] {
        if d == 0 {
            if p < lo || p > hi {
                return (len2, 0);
            }
            continue;
        }
        let (ta, tb) = ((lo - p) as f64 / d as f64, (hi - p) as f64 / d as f64);
        t0 = t0.max(ta.min(tb));
        t1 = t1.min(ta.max(tb));
    }
    if t1 <= t0 {
        return (len2, 0);
    }
    let l = len2 as f64;
    ((t0 * l).ceil() as i128, (t1 * l).floor() as i128)
}

// ===========================================================================
// Exact enclosure of one region by another, in DBU.
//
// A margin is the distance from a wall of the enclosed shape to the enclosing wall
// facing it across its outside: under the projection metric the perpendicular offset
// of a parallel wall over the stretch they share, under the euclidian one the closest
// approach of any wall on the outside.  Both are exact on integer coordinates the way a
// gap is - compared squared, as a ratio of two integers - and whether a vertex is
// inside, on or off a boundary is a matter of signs.
// ===========================================================================

/// A margin read on one facing pair: squared as `num / den`, the stretch of the inner
/// wall it was read on, and a probe point just beyond the outer wall, both in DBU.
pub struct MarginPair {
    pub num: i128,
    pub den: i128,
    pub edge: (f64, f64, f64, f64),
    pub probe: (f64, f64),
    /// The inner wall the margin was read on.
    pub wall: Seg,
    /// Read against an outer wall at an angle - a closest approach, not a facing run.
    pub oblique: bool,
    /// The outer wall the margin was read against.
    pub outer: Seg,
}

impl MarginPair {
    /// The margin in DBU, for a message.
    pub fn dbu(&self) -> f64 {
        (self.num as f64 / self.den as f64).sqrt()
    }
}

/// The closest approach of two segments: the squared distance as `num / den` and the
/// point on each, in DBU.
pub fn seg_closest(a: Seg, b: Seg) -> (i128, i128, (f64, f64), (f64, f64)) {
    let c = seg_seg_closest_sq(a.0, a.1, b.0, b.1);
    (c.num, c.den, c.on_a, c.on_b)
}

/// Whether every vertex of `inner` lies inside `outer` or on its boundary - a shape
/// touching the enclosing wall is enclosed by nothing there, and that is what a rule at
/// zero is about.
pub fn all_inside(inner: &Outline, outer: &Outline) -> bool {
    // A shape inside another has its box inside the other's; the boxes settle nearly
    // every pair in a tile before a vertex is cast against a wall.
    let (ix0, iy0, ix1, iy1) = inner.bbox;
    let (ox0, oy0, ox1, oy1) = outer.bbox;
    if ix0 < ox0 || iy0 < oy0 || ix1 > ox1 || iy1 > oy1 {
        return false;
    }
    // Every vertex inside is not enough: a contact lying across a slot in the metal has
    // its four corners on metal and a strip of itself over the slot, which the slot's
    // walls crossing its own give away.
    if !inner.vertices().all(|p| outer.contains_or_on(p)) {
        return false;
    }
    let mut crossed = false;
    outer.for_segs_near(inner.bbox, 0, |t| {
        crossed = crossed || inner.segs().iter().any(|&s| segs_cross_i(s, t));
    });
    !crossed
}

/// Whether two segments meet at all: properly crossing, or touching at a point.  Two
/// collinear segments that overlap have an end of one on the other.
fn segs_meet_i((p0, p1): Seg, (q0, q1): Seg) -> bool {
    segs_cross_i((p0, p1), (q0, q1))
        || on_segment(p0, (q0, q1))
        || on_segment(p1, (q0, q1))
        || on_segment(q0, (p0, p1))
        || on_segment(q1, (p0, p1))
}

/// Whether two regions share any area or boundary.  A vertex test alone misses the
/// ordinary case: two bars crossing in a plus share a large area and have no vertex of
/// either inside the other, which is what a gate over its COMP looks like.
pub fn regions_interact(a: &Outline, b: &Outline) -> bool {
    if !a.possibly_within(b, 1) {
        return false;
    }
    a.vertices().any(|p| b.contains_or_on(p))
        || b.vertices().any(|p| a.contains_or_on(p))
        || a.segs
            .iter()
            .any(|&s| b.segs.iter().any(|&t| segs_meet_i(s, t)))
}

/// The facing pairs of `inner` against `outer`, each with its margin, and whether any
/// pair was coincident - an inner wall lying on the outer contour, which is either a
/// margin of nothing or the cut a boolean left, and `skip_coincident` says which.  A
/// `cutoff` keeps only the pairs under it, which is all a minimum reads; a maximum
/// passes `None` and sees them all.  Under the projection metric only parallel walls
/// pair, over the stretch they share; under the euclidian one a wall at any angle
/// pairs at its closest approach, as long as it lies on the inner wall's outside - the
/// far wall across a concave shape is not an enclosure of anything.
pub fn margin_pairs(
    inner: &Outline,
    outer: &Outline,
    cutoff: Option<i64>,
    skip_coincident: bool,
    euclidian: bool,
) -> (Vec<MarginPair>, bool) {
    let cut2 = cutoff.map(|c| (c as i128) * (c as i128));
    let under = |num: i128, den: i128| cut2.is_none_or(|c2| num < c2 * den);
    // Two segments whose boxes lie the cutoff apart are at least that far apart, and
    // a minimum has nothing to read there: a contact's four walls against the two
    // thousand of the metal plate round it were each measured against every one.
    let boxes_apart = |(p0, p1): Seg, (q0, q1): Seg| {
        cutoff.is_some_and(|c| {
            (p0.0.min(p1.0) - q0.0.max(q1.0)).max(q0.0.min(q1.0) - p0.0.max(p1.0)) >= c
                || (p0.1.min(p1.1) - q0.1.max(q1.1)).max(q0.1.min(q1.1) - p0.1.max(p1.1)) >= c
        })
    };
    let mut pairs = Vec::new();
    let mut saw_coincident = false;
    for &(a0, a1) in &inner.segs {
        let d = ((a1.0 - a0.0) as i128, (a1.1 - a0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        let len = (len2 as f64).sqrt();
        let (ux, uy) = (d.0 as f64 / len, d.1 as f64 / len);
        // The right-hand normal of a CCW contour points outward, toward the enclosing
        // wall; a hole runs the other way and its outward is into the hole, which is
        // where its enclosing wall is.
        let (nx, ny) = (uy, -ux);
        for &(b0, b1) in &outer.segs {
            if boxes_apart((a0, a1), (b0, b1)) {
                continue;
            }
            let e = ((b1.0 - b0.0) as i128, (b1.1 - b0.1) as i128);
            if !parallel_i(d, e) {
                if !euclidian {
                    continue;
                }
                // Walls that cross are no margin: the shape is not enclosed there at
                // all, which `all_inside` says, or the rule reads a shape it does not
                // expect enclosed (`interacting_only`).
                if segs_cross_i((a0, a1), (b0, b1)) {
                    continue;
                }
                let c = seg_seg_closest_sq(a0, a1, b0, b1);
                if !under(c.num, c.den) {
                    continue;
                }
                if c.num == 0 {
                    saw_coincident = true;
                    if skip_coincident {
                        continue;
                    }
                    pairs.push(MarginPair {
                        num: 0,
                        den: 1,
                        edge: (c.on_a.0, c.on_a.1, c.on_b.0, c.on_b.1),
                        probe: (c.on_a.0 + nx * 2.0, c.on_a.1 + ny * 2.0),
                        wall: (a0, a1),
                        oblique: true,
                        outer: (b0, b1),
                    });
                    continue;
                }
                // Outward: the closest approach runs from the inner wall toward the
                // outer one on the outside, or it is the far wall.
                let (vx, vy) = c.towards;
                if vx * d.1 - vy * d.0 <= 0 {
                    continue;
                }
                let dist = (c.num as f64 / c.den as f64).sqrt();
                let (wx, wy) = ((c.on_b.0 - c.on_a.0) / dist, (c.on_b.1 - c.on_a.1) / dist);
                pairs.push(MarginPair {
                    num: c.num,
                    den: c.den,
                    edge: (c.on_a.0, c.on_a.1, c.on_b.0, c.on_b.1),
                    probe: (c.on_a.0 + wx * (dist + 2.0), c.on_a.1 + wy * (dist + 2.0)),
                    wall: (a0, a1),
                    oblique: true,
                    outer: (b0, b1),
                });
                continue;
            }
            let run = Outline::shared_run((a0, a1), (b0, b1));
            if run <= 0 {
                continue; // no stretch in common: not facing
            }
            // The outer wall's two ends, as outward distance times the inner length;
            // the same on exactly parallel walls, and the nearer one otherwise.
            let (ca, cb) = (-cross_i(a0, a1, b0), -cross_i(a0, a1, b1));
            if ca.max(cb) < 0 {
                continue; // on the interior side: the far wall, not this one
            }
            let c = ca.min(cb).max(0);
            let (num, den) = (c * c, len2);
            if c == 0 {
                saw_coincident = true;
                if skip_coincident {
                    continue;
                }
            }
            if !under(num, den) {
                continue;
            }
            let along = |p: (i64, i64)| (p.0 - a0.0) as i128 * d.0 + (p.1 - a0.1) as i128 * d.1;
            let (t0, t1) = (along(b0), along(b1));
            let (s0, s1) = (t0.min(t1).max(0), t0.max(t1).min(len2));
            let at = |s: i128| {
                let f = s as f64 / len;
                (a0.0 as f64 + ux * f, a0.1 as f64 + uy * f)
            };
            let (p0, p1) = (at(s0), at(s1));
            let dist = c as f64 / len;
            let mid = ((p0.0 + p1.0) * 0.5, (p0.1 + p1.1) * 0.5);
            pairs.push(MarginPair {
                num,
                den,
                edge: (p0.0, p0.1, p1.0, p1.1),
                probe: (mid.0 + nx * (dist + 2.0), mid.1 + ny * (dist + 2.0)),
                wall: (a0, a1),
                oblique: false,
                outer: (b0, b1),
            });
        }
    }
    (pairs, saw_coincident)
}

/// The endcap margin of `inner` within `outer`: the largest of the four box margins,
/// in DBU.  A wire's endcap is read off the boxes on purpose - an edge-to-contour
/// distance is corner-limited and would understate a long endcap run.
pub fn endcap_margin(inner: &Outline, outer: &Outline) -> i64 {
    let (ix0, iy0, ix1, iy1) = inner.bbox;
    let (ox0, oy0, ox1, oy1) = outer.bbox;
    (ix0 - ox0).max(ox1 - ix1).max(iy0 - oy0).max(oy1 - iy1)
}

/// The margin of each side of `inner` within `outers`, in `inner.segs()` order, in DBU:
/// the nearest outward wall over the stretch that projects onto the side, or 0 for a
/// side nothing faces.  A wall parallel to the side is at one distance along it, an
/// integer times the side's own length; one at another angle - a chamfer facing a via
/// on a power ring corner - is nearest at one end of the stretch, found by
/// interpolation, which is the one reading here that is not exact.
pub fn side_margins(inner: &Outline, outers: &[&Outline]) -> Vec<f64> {
    inner
        .segs
        .iter()
        .map(|&(a0, a1)| {
            let d = ((a1.0 - a0.0) as f64, (a1.1 - a0.1) as f64);
            let len = d.0.hypot(d.1);
            let (ux, uy) = (d.0 / len, d.1 / len);
            let (nx, ny) = (uy, -ux);
            let mut best = f64::INFINITY;
            for o in outers {
                for &(b0, b1) in &o.segs {
                    let along =
                        |p: (i64, i64)| ((p.0 - a0.0) as f64) * ux + ((p.1 - a0.1) as f64) * uy;
                    let out =
                        |p: (i64, i64)| ((p.0 - a0.0) as f64) * nx + ((p.1 - a0.1) as f64) * ny;
                    let (t0, t1) = (along(b0), along(b1));
                    let (n0, n1) = (out(b0), out(b1));
                    let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(len));
                    if hi - lo <= 0.0 {
                        continue; // no projected overlap
                    }
                    let at = |t: f64| {
                        if (t1 - t0).abs() < 1e-9 {
                            n0.min(n1)
                        } else {
                            n0 + (n1 - n0) * (t - t0) / (t1 - t0)
                        }
                    };
                    let (m0, m1) = (at(lo), at(hi));
                    if m0.max(m1) < 0.0 {
                        continue; // wholly behind the side: the far wall
                    }
                    best = best.min(m0.min(m1).max(0.0));
                }
            }
            if best.is_finite() { best } else { 0.0 }
        })
        .collect()
}

/// The line-end segments of `a`: the caps across the tip of any track narrower than
/// `max_width` that runs for at least `min_length`, in DBU.  A track is two of the
/// region's own walls facing each other across its inside closer than `max_width`, and
/// the cap is the segment joining them both; a segment that is itself a wall of some
/// narrow pair is not a cap, so a small square is not a line end - it has no line.
pub fn line_end_segs(a: &Outline, max_width: i64, min_length: i64) -> Vec<Seg> {
    let (w2, l2) = (
        (max_width as i128) * (max_width as i128),
        (min_length as i128) * (min_length as i128),
    );
    let n = a.segs.len();
    let len2 = |(p, q): Seg| {
        let d = ((q.0 - p.0) as i128, (q.1 - p.1) as i128);
        d.0 * d.0 + d.1 * d.1
    };
    let mut walls: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let (a0, a1) = a.segs[i];
        let d = ((a1.0 - a0.0) as i128, (a1.1 - a0.1) as i128);
        let li2 = d.0 * d.0 + d.1 * d.1;
        if li2 < l2 {
            continue;
        }
        for j in (i + 1)..n {
            let (b0, b1) = a.segs[j];
            let e = ((b1.0 - b0.0) as i128, (b1.1 - b0.1) as i128);
            if len2((b0, b1)) < l2 || d.0 * e.0 + d.1 * e.1 >= 0 || !parallel_i(d, e) {
                continue; // too short, or not a facing wall
            }
            // Across the inside of the shape, strictly narrower than `max_width`: a
            // track exactly the width is not a narrow line.
            let c = cross_i(a0, a1, b0);
            if c <= 0 || c * c >= w2 * li2 {
                continue;
            }
            let run = Outline::shared_run((a0, a1), (b0, b1));
            if run <= 0 || run * run < l2 * li2 {
                continue; // the narrow run is not long enough to be a line
            }
            walls.push((i, j));
        }
    }
    if walls.is_empty() {
        return Vec::new();
    }
    let touches = |e: usize, w: usize| {
        let ((p0, p1), (q0, q1)) = (a.segs[e], a.segs[w]);
        p0 == q0 || p0 == q1 || p1 == q0 || p1 == q1
    };
    let is_wall: HashSet<usize> = walls.iter().flat_map(|&(i, j)| [i, j]).collect();
    (0..n)
        .filter(|&e| {
            len2(a.segs[e]) < w2
                && !is_wall.contains(&e)
                && walls.iter().any(|&(i, j)| touches(e, i) && touches(e, j))
        })
        .map(|e| a.segs[e])
        .collect()
}

/// The margin of an inner segment behind the caps facing it, squared as `num / den`,
/// or `None` if no cap faces it: a cap is parallel, shares a stretch, and lies on the
/// outside.
pub fn margin_to_caps((a0, a1): Seg, caps: &[Seg]) -> Option<(i128, i128)> {
    let d = ((a1.0 - a0.0) as i128, (a1.1 - a0.1) as i128);
    let len2 = d.0 * d.0 + d.1 * d.1;
    let mut best: Option<i128> = None;
    for &(c0, c1) in caps {
        let e = ((c1.0 - c0.0) as i128, (c1.1 - c0.1) as i128);
        if !parallel_i(d, e) || Outline::shared_run((a0, a1), (c0, c1)) <= 0 {
            continue;
        }
        let c = -cross_i(a0, a1, c0);
        if c < 0 {
            continue;
        }
        if best.is_none_or(|b| c < b) {
            best = Some(c);
        }
    }
    best.map(|c| (c * c, len2))
}

/// Indices of the segments bordering segment `i` of `a`, by shared endpoint - the same
/// test for a ring and for a shape with holes, whose segments are several rings end to
/// end.
pub fn bordering(a: &Outline, i: usize) -> Vec<usize> {
    let (p0, p1) = a.segs[i];
    (0..a.segs.len())
        .filter(|&j| j != i)
        .filter(|&j| {
            let (q0, q1) = a.segs[j];
            p0 == q0 || p0 == q1 || p1 == q0 || p1 == q1
        })
        .collect()
}

// ===========================================================================
// Exact facing pairs of two regions that share area, in DBU.
// ===========================================================================

/// The facing pairs of two regions, each as its gap squared `num / den` and the point
/// on each region, in DBU: two walls whose outward normals oppose, with the other on
/// this one's outside - empty ground between them, a gap - or, `inward`, on its inside,
/// material of both between them, an overlap.  Parallel walls pair over the stretch
/// they share at their perpendicular offset; walls at an angle pair at their closest
/// approach, as long as each lies on the other's chosen side, which is what keeps an
/// ordinary convex corner of empty space from reading as a gap.  A touch is no pair: it
/// has no side to be on.  Only pairs under `limit` come back.
pub fn facing_pairs_i(a: &Outline, b: &Outline, limit: i64, inward: bool) -> Vec<ClosestPair> {
    let lim2 = (limit as i128) * (limit as i128);
    let sign: i128 = if inward { -1 } else { 1 };
    let mut out = Vec::new();
    for &(s0, s1) in &a.segs {
        let d = ((s1.0 - s0.0) as i128, (s1.1 - s0.1) as i128);
        let len2 = d.0 * d.0 + d.1 * d.1;
        for &(t0, t1) in &b.segs {
            let e = ((t1.0 - t0.0) as i128, (t1.1 - t0.1) as i128);
            if d.0 * e.0 + d.1 * e.1 >= 0 {
                continue; // the same way round: back to back, not facing
            }
            if parallel_i(d, e) {
                // The other wall's ends as outward distance times this wall's length -
                // inward, the other way up - and the nearer one on drifted walls.
                let (ca, cb) = (-sign * cross_i(s0, s1, t0), -sign * cross_i(s0, s1, t1));
                let c = ca.min(cb);
                if c <= 0 {
                    continue; // touching, or behind
                }
                if Outline::shared_run((s0, s1), (t0, t1)) <= 0 {
                    // No stretch in common: two walls end to end, offset - the flanks
                    // of a notch and of the arrow tip pointing into it.  KLayout reads
                    // the distance between their nearest ends under the euclidian
                    // metric, and so does the width scan across a corner.
                    let c = seg_seg_closest_sq(s0, s1, t0, t1);
                    if c.num > 0 && c.num < lim2 * c.den {
                        out.push((c.num, c.den, c.on_a, c.on_b));
                    }
                    continue;
                }
                if c * c >= lim2 * len2 {
                    continue; // far enough apart
                }
                let len = (len2 as f64).sqrt();
                let (ux, uy) = (d.0 as f64 / len, d.1 as f64 / len);
                let (nx, ny) = (sign as f64 * uy, -(sign as f64) * ux);
                let along = |p: (i64, i64)| (p.0 - s0.0) as i128 * d.0 + (p.1 - s0.1) as i128 * d.1;
                let (u0, u1) = (along(t0), along(t1));
                let mid = (u0.min(u1).max(0) + u0.max(u1).min(len2)) as f64 / (2.0 * len);
                let (px, py) = (s0.0 as f64 + ux * mid, s0.1 as f64 + uy * mid);
                let gap = c as f64 / len;
                out.push((c * c, len2, (px, py), (px + nx * gap, py + ny * gap)));
                continue;
            }
            // Not parallel, so there is no constant gap to project: each wall has to
            // reach the other's chosen side - a corner resting on a wall only touches
            // its line and never reaches past it, and drops out here - and the pair is
            // read at its closest approach, as long as that approach is between the
            // parts of the walls on those sides and not behind one of them.
            let beyond = |p: (i64, i64), (q0, q1): Seg| sign * -cross_i(q0, q1, p) > 0;
            if !(beyond(s0, (t0, t1)) || beyond(s1, (t0, t1)))
                || !(beyond(t0, (s0, s1)) || beyond(t1, (s0, s1)))
            {
                continue;
            }
            let c = seg_seg_closest_sq(s0, s1, t0, t1);
            if c.num == 0 || c.num >= lim2 * c.den {
                continue;
            }
            let side = |(px, py): (f64, f64), (q0, q1): Seg| {
                let (ex, ey) = ((q1.0 - q0.0) as f64, (q1.1 - q0.1) as f64);
                let f = (px - q0.0 as f64) * ey - (py - q0.1 as f64) * ex;
                sign as f64 * f / ex.hypot(ey)
            };
            if side(c.on_a, (t0, t1)) < -0.5 || side(c.on_b, (s0, s1)) < -0.5 {
                continue; // the approach is behind a wall, not across the gap
            }
            out.push((c.num, c.den, c.on_a, c.on_b));
        }
    }
    out
}

#[cfg(test)]
mod space_tests {
    use super::*;

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        MergedPoly {
            outer: vec![
                IntPoint::new(x0, y0),
                IntPoint::new(x1, y0),
                IntPoint::new(x1, y1),
                IntPoint::new(x0, y1),
            ],
            holes: vec![],
        }
    }

    /// An axis-aligned gap is the difference of two coordinates, and a diagonal one is
    /// the sum of two squares: both are integers, and the limit reads them exactly.
    #[test]
    fn gaps_are_exact() {
        let (a, b) = (rect(0, 0, 100, 100), rect(130, 0, 200, 100));
        let (oa, ob) = (Outline::new(&a), Outline::new(&b));
        let (n, d, p, q) = closest_approach(&oa, &ob, 1000).expect("in reach");
        assert_eq!((n, d), (900, 1));
        assert_eq!((p, q), ((100.0, 0.0), (130.0, 0.0)));
        assert!(Limit::AtLeast(31).broken_by_sq(n, d));
        assert!(!Limit::AtLeast(30).broken_by_sq(n, d));
        let c = rect(103, 104, 200, 200);
        let oc = Outline::new(&c);
        let (n, d, _, _) = closest_approach(&oa, &oc, 1000).expect("in reach");
        assert_eq!((n, d), (25, 1), "3² + 4²");
        assert!(Limit::AtLeast(6).broken_by_sq(n, d));
        assert!(!Limit::AtLeast(5).broken_by_sq(n, d));
    }

    /// A pair whose boxes are the limit apart is not looked at.
    #[test]
    fn out_of_reach_is_not_measured() {
        let (a, b) = (rect(0, 0, 100, 100), rect(130, 0, 200, 100));
        assert!(closest_approach(&Outline::new(&a), &Outline::new(&b), 30).is_none());
    }

    /// Meeting at a corner is a contact, a gap of nothing; drawn edge to edge is an
    /// abutment, no gap at all.  Both approach to zero, and the run tells them apart.
    #[test]
    fn a_corner_touch_and_an_abutment_differ_by_the_run() {
        let a = rect(0, 0, 100, 100);
        let corner = rect(100, 100, 200, 200);
        let abut = rect(100, 20, 200, 120);
        let (oa, oc, ob) = (Outline::new(&a), Outline::new(&corner), Outline::new(&abut));
        assert_eq!(closest_approach(&oa, &oc, 10).map(|c| c.0), Some(0));
        assert!(!share_boundary_run(&oa, &oc));
        assert_eq!(closest_approach(&oa, &ob, 10).map(|c| c.0), Some(0));
        assert!(share_boundary_run(&oa, &ob));
        assert!(!regions_overlap(&oa, &oc));
        assert!(!regions_overlap(&oa, &ob));
    }

    /// Two squares overlapping by 40 penetrate each other by 40, read inward; two squares
    /// 30 apart face each other across 30, read outward; a touch is no pair either way.
    #[test]
    fn facing_pairs_read_a_gap_or_an_overlap() {
        let (a, b) = (rect(0, 0, 100, 100), rect(60, 0, 160, 100));
        let (oa, ob) = (Outline::new(&a), Outline::new(&b));
        // The shallowest pair is the 40 across the overlap; the top and bottom walls
        // face too, 100 apart, the whole height of the squares.
        let shallowest = |ps: &[ClosestPair]| {
            ps.iter()
                .map(|p| p.0 as f64 / p.1 as f64)
                .min_by(|a, b| a.total_cmp(b))
        };
        let inward = facing_pairs_i(&oa, &ob, 1000, true);
        assert_eq!(shallowest(&inward), Some(1600.0), "40²");
        assert!(facing_pairs_i(&oa, &ob, 1000, false).is_empty());
        let c = rect(130, 0, 200, 100);
        let oc = Outline::new(&c);
        let outward = facing_pairs_i(&oa, &oc, 1000, false);
        assert_eq!(shallowest(&outward), Some(900.0), "30²");
        assert!(
            facing_pairs_i(&oa, &oc, 30, false).is_empty(),
            "30 is not under 30"
        );
        let touch = rect(100, 0, 200, 100);
        assert!(facing_pairs_i(&oa, &Outline::new(&touch), 1000, false).is_empty());
    }

    /// A line exactly the width is not wider, five nanometres more is; a run exactly the
    /// length is not longer.  Read off the walls, not the boxes.
    #[test]
    fn the_wide_line_gate_reads_depth_and_run_exactly() {
        let (a, b) = (rect(0, 0, 1000, 300), rect(0, 850, 1000, 1150));
        let (oa, ob) = (Outline::new(&a), Outline::new(&b));
        assert!(
            !parallel_run_applies(&oa, &ob, 600, 300, 0, None),
            "0.3 deep is not over 0.3"
        );
        assert!(parallel_run_applies(&oa, &ob, 600, 299, 0, None));
        assert!(
            !parallel_run_applies(&oa, &ob, 600, 0, 1000, None),
            "a 1 µm run is not over 1 µm"
        );
        assert!(parallel_run_applies(&oa, &ob, 600, 0, 999, None));
        assert!(
            !parallel_run_applies(&oa, &ob, 550, 0, 0, None),
            "the gap is 550, not under it"
        );
        let l = MergedPoly {
            outer: [
                (0, 0),
                (1000, 0),
                (1000, 1000),
                (800, 1000),
                (800, 200),
                (0, 200),
            ]
            .iter()
            .map(|&(x, y)| IntPoint::new(x, y))
            .collect(),
            holes: vec![],
        };
        let ol = Outline::new(&l);
        // A 0.3 by 0.1 bar over the L's arm, 0.2 above it and 0.5 short of its upright.
        let c = rect(0, 400, 300, 500);
        let oc = Outline::new(&c);
        assert!(
            !parallel_run_applies(&ol, &oc, 300, 250, 0, None),
            "an L's box is 1 µm deep, the arm facing the gap 0.2, the line across it 0.1"
        );
        assert!(parallel_run_applies(&ol, &oc, 300, 150, 0, None));
    }

    /// A bend counts only at the gap.
    #[test]
    fn a_diagonal_counts_within_the_limit_alone() {
        let a = MergedPoly {
            outer: [(0, 0), (1000, 0), (1000, 500), (500, 1000), (0, 1000)]
                .iter()
                .map(|&(x, y)| IntPoint::new(x, y))
                .collect(),
            holes: vec![],
        };
        let oa = Outline::new(&a);
        let near = rect(1200, 0, 1500, 1000);
        let far = rect(-800, 0, -500, 1000);
        assert!(has_diagonal_within(&oa, &Outline::new(&near), 600));
        assert!(!has_diagonal_within(&oa, &Outline::new(&far), 600));
    }

    /// Shared area is a vertex strictly inside, a nesting with coincident walls, or a
    /// proper crossing with no vertex of either inside the other.
    #[test]
    fn overlap_is_shared_area() {
        let a = rect(0, 0, 100, 100);
        let oa = Outline::new(&a);
        assert!(regions_overlap(&oa, &Outline::new(&rect(50, 50, 150, 150))));
        assert!(regions_overlap(&oa, &Outline::new(&rect(0, 0, 40, 100))));
        assert!(
            regions_overlap(&oa, &Outline::new(&rect(40, -50, 60, 150))),
            "a plus"
        );
        assert!(!regions_overlap(
            &oa,
            &Outline::new(&rect(100, 0, 200, 100))
        ));
        let ring = MergedPoly {
            outer: rect(0, 0, 100, 100).outer,
            holes: vec![rect(30, 30, 70, 70).outer],
        };
        let island = rect(40, 40, 60, 60);
        assert!(
            !regions_overlap(&Outline::new(&ring), &Outline::new(&island)),
            "an island in a hole shares no area with the ring"
        );
    }
}
