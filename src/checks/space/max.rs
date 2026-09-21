// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `max_space`: everything of one layer within reach of another.  A latch-up rule says
//! it of every part of the diffusion - "every source/drain within 20 µm of a well
//! tie" - and a well-tap rule of every polygon - "a diffusion is fine once any part of
//! it has a tap within 15 µm" - and a guard-ring rule of every edge.  `scope` says
//! which: `part` (the default), `polygon` or `edge`.
//!
//! This is the "reach" half of the distance rules, and it reads backwards from the
//! rest of the engine.  A spacing rule fails on the *closest* pair and is satisfied by
//! there being nothing nearby; this one fails on the *absence* of a neighbour, so an
//! empty partner layer makes everything a violation rather than nothing.
//!
//! `part` and `polygon` run on the tiled merge as core pieces against the reference
//! grown by the value (see [`MergedCache::max_space_gaps`] and
//! [`MergedCache::max_space_unreached`]), so a dense reference - the contacts on ties -
//! is never globally unioned and neither layer pays the value as a halo.  A `within`
//! layer confines the `polygon` reach to it, the way the well-tap rules grow the tap
//! inside the well.  `edge`
//! gathers the partner by tile lookup over each edge's own box grown by the limit,
//! since an edge is not clipped to a tile and a halo cannot bound it.

use super::super::params::{NotAWord, mode};
use crate::layout::FlatLayout;
use crate::merge::{Edge, MergedCache, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// What of `layers[0]` has to be in reach.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// Every part: what lies beyond reach is the violation, wherever on a shape it is.
    Part,
    /// Every polygon: one no part of which is in reach.
    Polygon,
    /// Every edge of an edge layer.
    Edge,
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!(
            "[{}] max_space needs two layers (target, reference)",
            rule.id
        );
        return vec![];
    }
    let scope = match mode(rule, "max_space", "scope") {
        Ok(None) | Ok(Some("part")) => Scope::Part,
        Ok(Some("polygon")) => Scope::Polygon,
        Ok(Some("edge")) => Scope::Edge,
        Ok(Some(other)) => {
            eprintln!(
                "[{}] max_space: scope can only be `part`, `polygon` or `edge`, not `{other}`",
                rule.id
            );
            return vec![];
        }
        Err(NotAWord) => return vec![],
    };
    if scope == Scope::Edge {
        return run_edges(rule, layout, dbu_to_um, merged);
    }
    let target = &rule.layers[0];
    let reference = &rule.layers[1];
    let what = match scope {
        Scope::Part => "every part of",
        _ => "every polygon of",
    };
    println!(
        "[{}] Checking max_space <= {:.2} µm from {what} {} to {}",
        rule.id, rule.value, target.name, reference.name
    );
    let (a, b) = (
        (target.gds_layer as i16, target.gds_datatype as i16),
        (reference.gds_layer as i16, reference.gds_datatype as i16),
    );
    let value_dbu = rule.value / dbu_to_um;
    // `within` (a layer param) confines the reach to a layer, grown in half-micron
    // steps - under any well's own spacing, so the reach never crosses to another well.
    let within = rule.num("within").map(|l| {
        let dt = rule.num("within_dt").unwrap_or(0.0);
        ((l as i16, dt as i16), 0.5 / dbu_to_um)
    });
    // A polygon exactly the value away is in reach - KLayout's `interacting` counts a
    // touch - and the grown reference touches it with no area to share, so the reach is
    // grown a DBU further: what is then shared is the DBU strip, and a polygon a DBU
    // beyond the value touches that and shares nothing.
    let markers = match scope {
        Scope::Part => merged.max_space_gaps(layout, a, b, value_dbu, within),
        _ => merged.max_space_unreached(layout, a, b, value_dbu + 1.0, within),
    };
    markers
        .into_iter()
        .map(|(cx, cy)| {
            let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
            let of = match scope {
                Scope::Part => "",
                _ => " as a whole",
            };
            Violation::point(
                &rule.id,
                "Maximum space violation",
                format!(
                    "{}{of} more than {:.2} µm from {} at ({:.4}, {:.4}) µm",
                    target.name, rule.value, reference.name, x, y
                ),
                x,
                y,
            )
        })
        .collect()
}

/// Squared distance from point `p` to segment `a → b`.
fn point_seg_d2(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    let (qx, qy) = (ax + dx * t, ay + dy * t);
    (px - qx).powi(2) + (py - qy).powi(2)
}

/// Euclidian distance between two segments, zero if they cross.
fn seg_seg_dist(e: (f64, f64, f64, f64), f: (f64, f64, f64, f64)) -> f64 {
    let (ax, ay, bx, by) = e;
    let (cx, cy, dx, dy) = f;
    let cross = |ux: f64, uy: f64, vx: f64, vy: f64| ux * vy - uy * vx;
    let (r1, r2) = (bx - ax, by - ay);
    let (s1, s2) = (dx - cx, dy - cy);
    let denom = cross(r1, r2, s1, s2);
    if denom.abs() > 1e-12 {
        let t = cross(cx - ax, cy - ay, s1, s2) / denom;
        let u = cross(cx - ax, cy - ay, r1, r2) / denom;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            return 0.0;
        }
    }
    point_seg_d2(ax, ay, cx, cy, dx, dy)
        .min(point_seg_d2(bx, by, cx, cy, dx, dy))
        .min(point_seg_d2(cx, cy, ax, ay, bx, by))
        .min(point_seg_d2(dx, dy, ax, ay, bx, by))
        .sqrt()
}

/// Distance from a segment to a merged region: zero if the segment meets it at all,
/// otherwise the closest approach to its contour.  Holes are contour too - a segment in
/// the middle of a ring's opening is not touching the ring.
fn seg_poly_dist(e: (f64, f64, f64, f64), m: &MergedPoly, best_so_far: f64) -> f64 {
    // Inside counts as touching, and the contour scan below would not say so: a segment
    // in the middle of a wide region is far from every wall of it and zero from the
    // region.  Testing an endpoint is enough - a segment with neither end inside either
    // misses the region entirely or crosses its contour, and the scan finds that.
    if crate::merge::point_in_merged(e.0, e.1, m) || crate::merge::point_in_merged(e.2, e.3, m) {
        return 0.0;
    }
    let mut best = best_so_far;
    for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
        let n = ring.len();
        for i in 0..n {
            let (p, q) = (&ring[i], &ring[(i + 1) % n]);
            let d = seg_seg_dist(e, (p.x as f64, p.y as f64, q.x as f64, q.y as f64));
            if d < best {
                best = d;
                if best <= 0.0 {
                    return 0.0;
                }
            }
        }
    }
    best
}

/// `scope: edge`: every edge of `layers[0]`, an edge layer, has `layers[1]` within
/// `value`, and an edge that has none is the violation.
fn run_edges(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let (a, b) = (&rule.layers[0], &rule.layers[1]);
    let akey = (a.gds_layer as i16, a.gds_datatype as i16);
    let bkey = (b.gds_layer as i16, b.gds_datatype as i16);
    if !merged.is_edge_layer(akey) {
        eprintln!(
            "[{}] max_space with `scope: edge` measures from an edge layer; '{}' is a \
             polygon layer",
            rule.id, a.name
        );
        return vec![];
    }
    println!(
        "[{}] Checking every edge of {} reaches {} within {:.2} µm",
        rule.id, a.name, b.name, rule.value
    );
    merged.ensure_edges(layout, akey);
    merged.ensure(layout, bkey.0, bkey.1);

    let limit = rule.value / dbu_to_um; // DBU
    let tile = merged.tile_dbu() as i64;
    // Collect the edges first: `tiles` borrows the cache, and an edge is owned by the
    // tile its midpoint falls in so a segment reaching across tiles is judged once.
    let mut edges: Vec<Edge> = Vec::new();
    let mut seen: std::collections::HashSet<(i32, i32, i32, i32)> =
        std::collections::HashSet::new();
    for (&(tx, ty), es) in merged.edges(akey) {
        let core = crate::merge::Core {
            x0: tx as i64 * tile,
            y0: ty as i64 * tile,
            x1: (tx as i64 + 1) * tile,
            y1: (ty as i64 + 1) * tile,
        };
        for e in es {
            let (mx, my) = e.midpoint();
            if core.owns(mx, my) && seen.insert((e.a.x, e.a.y, e.b.x, e.b.y)) {
                edges.push(*e);
            }
        }
    }

    let btiles = merged.tiles(bkey.0, bkey.1);
    let mut out = Vec::new();
    for e in edges {
        let seg = (e.a.x as f64, e.a.y as f64, e.b.x as f64, e.b.y as f64);
        let (x0, x1) = (seg.0.min(seg.2) - limit, seg.0.max(seg.2) + limit);
        let (y0, y1) = (seg.1.min(seg.3) - limit, seg.1.max(seg.3) + limit);
        let mut best = f64::INFINITY;
        'tiles: for tx in (x0 as i64).div_euclid(tile)..=(x1 as i64).div_euclid(tile) {
            for ty in (y0 as i64).div_euclid(tile)..=(y1 as i64).div_euclid(tile) {
                let Some(polys) = btiles.get(&(tx as i32, ty as i32)) else {
                    continue;
                };
                for m in polys {
                    best = seg_poly_dist(seg, m, best);
                    if best <= limit {
                        break 'tiles; // near enough; the exact distance does not matter
                    }
                }
            }
        }
        if best <= limit {
            continue;
        }
        let (ax, ay) = (seg.0 * dbu_to_um, seg.1 * dbu_to_um);
        let (bx, by) = (seg.2 * dbu_to_um, seg.3 * dbu_to_um);
        let how = if best.is_finite() {
            format!("{:.4} µm away", best * dbu_to_um)
        } else {
            "nowhere in reach".to_string()
        };
        out.push(Violation::edge(
            &rule.id,
            "Maximum distance violation",
            format!(
                "{}: nearest {} is {} > {:.2} µm from the edge at \
                 ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm",
                a.name, b.name, how, rule.value
            ),
            ax,
            ay,
            bx,
            by,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::IntPoint;

    fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        MergedPoly {
            outer: vec![
                IntPoint::new(x0, y0),
                IntPoint::new(x1, y0),
                IntPoint::new(x1, y1),
                IntPoint::new(x0, y1),
            ],
            holes: Vec::new(),
        }
    }

    /// Crossing segments are zero apart, and parallel ones are their perpendicular gap —
    /// the two cases a naive endpoint-only distance gets wrong in opposite directions.
    #[test]
    fn segment_distance_handles_crossing_and_parallel() {
        assert_eq!(
            seg_seg_dist((0.0, 0.0, 10.0, 0.0), (5.0, -5.0, 5.0, 5.0)),
            0.0
        );
        assert_eq!(
            seg_seg_dist((0.0, 0.0, 10.0, 0.0), (0.0, 3.0, 10.0, 3.0)),
            3.0
        );
        // Skew and non-overlapping: the closest approach is endpoint to endpoint.
        assert_eq!(
            seg_seg_dist((0.0, 0.0, 0.0, 10.0), (3.0, 14.0, 3.0, 20.0)),
            5.0
        );
    }

    /// A segment inside a region is zero from it, and one outside is measured to the
    /// contour — including a hole's, so a segment in a ring's opening is not touching it.
    #[test]
    fn segment_to_region_measures_every_contour() {
        let s = square(0, 0, 100, 100);
        assert_eq!(
            seg_poly_dist((10.0, 10.0, 20.0, 20.0), &s, f64::INFINITY),
            0.0
        );
        assert_eq!(
            seg_poly_dist((150.0, 0.0, 150.0, 100.0), &s, f64::INFINITY),
            50.0
        );
        let ring = MergedPoly {
            outer: square(0, 0, 100, 100).outer,
            holes: vec![square(40, 40, 60, 60).outer],
        };
        assert_eq!(
            seg_poly_dist((50.0, 50.0, 50.0, 50.0), &ring, f64::INFINITY),
            10.0,
            "the middle of the hole is 10 from the ring's inner wall"
        );
    }

    /// The search stops as soon as something is near enough, so a caller passing a bound
    /// gets that bound back rather than the true distance — which is all the check needs
    /// and is what lets it walk out of the tile loop early.
    #[test]
    fn a_running_bound_is_never_widened() {
        let s = square(0, 0, 100, 100);
        assert_eq!(seg_poly_dist((500.0, 0.0, 500.0, 10.0), &s, 7.0), 7.0);
    }
}
