// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Distance between two *edge* layers — KLayout's `enclosing`/`enclosed` and
//! `separation` in their edge-collection form.
//!
//! The region checks next door ask about whole shapes: this Pplus encloses that COMP by
//! so much, on the sides where they interact. A whole family of implant rules cannot be
//! said that way. GF180's `PP.5b` is "extension beyond COMP, for COMP inside NWELL or
//! outside LVPWELL but inside DNWELL", and both halves of it are *subsets of a boundary*:
//! the implant edges that are not butting edges and that fall inside the well, against
//! the COMP edges that are not implant-boundary edges. Neither subset is a region, and
//! the same implant polygon is measured against one limit along one of its walls and a
//! different limit along the next.
//!
//! So the pairing here is between segments, not shapes. Two edges pair when they are
//! parallel and each projects onto the other — KLayout's `projection` metric, which is
//! what every rule in this family asks for — and the two checks differ only in which way
//! their outward normals point:
//!
//! * **enclosure**: normals the same way, one boundary nested inside the other, and the
//!   margin is how far the inner sits inside the outer.
//! * **spacing**: normals opposed, the two facing each other across empty ground.
//!
//! Direction is what carries that, and it survives: an edge layer's segments keep the
//! direction of the contour they were extracted from, so an edge still knows which side
//! its material is on however many booleans later it is read.

use crate::layout::FlatLayout;
use crate::merge::{Core, Edge, MergedCache};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Whether a rule's two layers are edge layers, so the segment form of the check applies.
///
/// A rule says *what* must hold; the deck's layer declarations say what it holds between.
/// `min_space` between two regions is their closest approach, and between two edge layers
/// it is the gap across a facing pair — the same rule, measured on what it was given, the
/// way KLayout's `separation` reads on a Region and on an Edges alike.
///
/// Mixed kinds are a config error and say so. So is a rule that names an edge layer where
/// the check cannot use one: that used to build the edge key as an empty polygon layer
/// and pass in silence, which is the failure mode this engine works hardest to avoid.
pub fn on_edge_layers(rule: &RuleDefinition, merged: &MergedCache, check: &str) -> bool {
    let kinds: Vec<bool> = rule
        .layers
        .iter()
        .map(|l| merged.is_edge_layer((l.gds_layer as i16, l.gds_datatype as i16)))
        .collect();
    if kinds.iter().any(|&e| e) && !kinds.iter().all(|&e| e) {
        eprintln!(
            "[{}] {check}: mixes edge and polygon layers — a rule measures between two \
             of one kind or two of the other",
            rule.id
        );
    }
    kinds.first().copied().unwrap_or(false)
}

/// Which way a pair of edges has to face, and which side the measured span is on.
#[derive(Clone, Copy, PartialEq)]
enum Rel {
    /// Nested boundaries, normals the same way: how far the inner sits inside the outer.
    Enclosure,
    /// Facing boundaries, normals opposed, the span on the outside of both: empty ground.
    Space,
}

/// The outer layer's boundary must sit at least `value` outside the inner layer's.
pub fn run_enclosure(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Enclosure)
}

/// The two layers' boundaries must stay at least `value` apart.
pub fn run_space(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Space)
}

/// One offending pair: the margin measured, and the two points that measure it.
type Pair = (f64, (f64, f64), (f64, f64));

/// A segment as `origin`, unit direction, unit outward normal and length, all in DBU.
struct Seg {
    o: (f64, f64),
    u: (f64, f64),
    n: (f64, f64),
    len: f64,
}

impl Seg {
    fn of(e: &Edge) -> Option<Seg> {
        let o = (e.a.x as f64, e.a.y as f64);
        let (dx, dy) = (e.b.x as f64 - o.0, e.b.y as f64 - o.1);
        let len = dx.hypot(dy);
        if len <= 0.0 {
            return None;
        }
        let u = (dx / len, dy / len);
        Some(Seg {
            o,
            u,
            // The same convention the polygon checks use: outward for a CCW contour.
            n: (u.1, -u.0),
            len,
        })
    }
}

fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    rel: Rel,
) -> Vec<Violation> {
    let name = match rel {
        Rel::Enclosure => "min_enclosure",
        Rel::Space => "min_space",
    };
    let (Some(la), Some(lb)) = (rule.layers.first(), rule.layers.get(1)) else {
        eprintln!("[{}] {name} needs two layers", rule.id);
        return vec![];
    };
    let ka = (la.gds_layer as i16, la.gds_datatype as i16);
    let kb = (lb.gds_layer as i16, lb.gds_datatype as i16);
    for (k, l) in [(ka, la), (kb, lb)] {
        if !merged.is_edge_layer(k) {
            // A polygon layer here is a config error, not an empty result: the rule would
            // pass in silence, which is the failure this engine works hardest to avoid.
            eprintln!(
                "[{}] {name}: layer '{}' is not an edge layer — declare it under \
                 `edge_layers:`",
                rule.id, l.name
            );
            return vec![];
        }
    }
    merged.ensure_edges(layout, ka);
    merged.ensure_edges(layout, kb);

    println!(
        "[{}] Checking {name} >= {:.2} µm between edge layers {} and {}",
        rule.id, rule.value, la.name, lb.name
    );

    let limit = rule.value / dbu_to_um;
    // A boundary drawn flush with the one it must extend past has a zero margin that is
    // not an extension violation - the same reading `min_enclosure` takes, and the same
    // parameter name. GF180 draws Pplus flush to COMP wherever the implant is cut by a
    // neighbouring one, and upstream's `enclosing` reports nothing there.
    let skip_coincident = rule
        .params
        .get("skip_coincident")
        .is_some_and(|v| *v != 0.0);
    let tile = merged.tile_dbu() as i64;
    // Half a DBU: coordinates are integers, so anything under this is a rounding artefact.
    let tol = 0.5;
    let mut out = Vec::new();

    // A minimum pairs within one tile: it only looks as far as its own limit, and an edge
    // is filed under the tile its midpoint falls in.
    for (&(tx, ty), a_edges) in merged.edges(ka) {
        let Some(b_edges) = merged.edges(kb).get(&(tx, ty)) else {
            continue;
        };
        let core = Core {
            x0: tx as i64 * tile,
            y0: ty as i64 * tile,
            x1: (tx as i64 + 1) * tile,
            y1: (ty as i64 + 1) * tile,
        };
        for ea in a_edges {
            let Some(sa) = Seg::of(ea) else { continue };
            // The narrowest offending margin along this segment, and where it sits.
            let mut worst: Option<Pair> = None;
            for eb in b_edges {
                let Some(sb) = Seg::of(eb) else { continue };
                // Parallel enough to measure a width between, which is not the same as
                // parallel.  Coordinates are integers, so a wall that a boolean cut at
                // an angle keeps its direction only to the nearest DBU: the two sides of
                // one 45° bar can come out as (1160, 1160) and (1160, 1161), which is
                // 0.05° apart and was rejected outright by a test that wanted agreement
                // to six decimal places.  What matters is whether the gap between them
                // stays put along the run they share - the cross product of the unit
                // directions is the sine of the angle, so times the shorter run it is how
                // far the far end drifts, and a drift under one DBU is a straight gap as
                // far as the grid can say.
                // A wall one DBU out of true drifts by almost exactly one DBU over its
                // own length, whatever that length is, so the bound sits just above one.
                let drift = (sa.u.0 * sb.u.1 - sa.u.1 * sb.u.0).abs() * sa.len.min(sb.len);
                if drift > 1.5 {
                    continue; // not parallel: no steady gap to measure
                }
                let dot = sa.n.0 * sb.n.0 + sa.n.1 * sb.n.1;
                if (rel == Rel::Enclosure) != (dot > 0.0) {
                    // Enclosure wants nested boundaries (normals the same way); the other
                    // two want facing ones (normals opposed).
                    continue;
                }
                // Which side of a the span lies on: outward for a spacing, inward for an
                // enclosure. One sign covers both.
                let along = (sb.o.0 - sa.o.0) * sa.n.0 + (sb.o.1 - sa.o.1) * sa.n.1;
                let margin = if rel == Rel::Space { along } else { -along };
                // Behind this edge either way, or already far enough.
                if margin < -tol || margin >= limit {
                    continue;
                }
                if skip_coincident && margin <= tol {
                    continue; // flush, not short
                }
                // Projected overlap: only the stretch the two share counts.
                let t0 = (sb.o.0 - sa.o.0) * sa.u.0 + (sb.o.1 - sa.o.1) * sa.u.1;
                let t1 = t0 + sb.len * (sb.u.0 * sa.u.0 + sb.u.1 * sa.u.1);
                let (s0, s1) = (t0.min(t1).max(0.0), t0.max(t1).min(sa.len));
                if s1 - s0 <= tol {
                    continue;
                }
                if worst.is_some_and(|(m, _, _)| m <= margin) {
                    continue;
                }
                // The marker is the span measured, across the middle of the run the two
                // share, which is short enough to stand for where the violation is.
                let mid = (s0 + s1) * 0.5;
                let p = (sa.o.0 + mid * sa.u.0, sa.o.1 + mid * sa.u.1);
                let q = if rel != Rel::Space {
                    (p.0 - sa.n.0 * margin, p.1 - sa.n.1 * margin)
                } else {
                    (p.0 + sa.n.0 * margin, p.1 + sa.n.1 * margin)
                };
                worst = Some((margin, p, q));
            }
            let Some((margin, p, q)) = worst else {
                continue;
            };
            // The pair is owned by the tile holding the middle of what it measures, so an
            // edge seen from two tiles is reported once.
            let (mx, my) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
            if !core.owns(mx, my) {
                continue;
            }
            let (what, title) = match rel {
                Rel::Enclosure => ("enclosure", "Minimum enclosure violation"),
                Rel::Space => ("space", "Minimum space violation"),
            };
            let cmp = "<";
            out.push(Violation::edge(
                &rule.id,
                title,
                format!(
                    "{what} {:.4} µm {cmp} {:.2} µm between {} and {} at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                    margin * dbu_to_um,
                    rule.value,
                    la.name,
                    lb.name,
                    p.0 * dbu_to_um,
                    p.1 * dbu_to_um,
                    q.0 * dbu_to_um,
                    q.1 * dbu_to_um,
                ),
                p.0 * dbu_to_um,
                p.1 * dbu_to_um,
                q.0 * dbu_to_um,
                q.1 * dbu_to_um,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::IntPoint;

    fn seg(ax: i32, ay: i32, bx: i32, by: i32) -> Seg {
        Seg::of(&Edge {
            a: IntPoint { x: ax, y: ay },
            b: IntPoint { x: bx, y: by },
        })
        .expect("non-degenerate")
    }

    fn drift(a: &Seg, b: &Seg) -> f64 {
        (a.u.0 * b.u.1 - a.u.1 * b.u.0).abs() * a.len.min(b.len)
    }

    /// The two walls of one 45° bar come off a boolean a nanometre apart in direction.
    /// The gap between them is still straight as far as the grid can say, so it still
    /// bounds a width - these are the exact coordinates a 0.24 µm diagonal gate produced,
    /// where a test for exact parallelism found nothing to measure.
    #[test]
    fn a_wall_a_nanometre_off_parallel_still_bounds_a_width() {
        let exact = seg(13000, 11500, 11500, 10000);
        let snapped = seg(11840, 10000, 13000, 11161);
        assert!(
            drift(&exact, &snapped) <= 1.0,
            "drift {} should be within a DBU",
            drift(&exact, &snapped)
        );
        // A wall genuinely at another angle drifts far past it.
        let turned = seg(11840, 10000, 13000, 12000);
        assert!(
            drift(&exact, &turned) > 1.0,
            "drift {} should exceed a DBU",
            drift(&exact, &turned)
        );
        // One DBU out of true drifts by about one whatever the run, so a snapped wall is
        // measured against a long partner as readily as a short one...
        let flat = seg(0, 0, 100_000, 0);
        assert!(drift(&flat, &seg(0, 500, 100_000, 501)) <= 1.5);
        assert!(drift(&flat, &seg(0, 500, 1_000, 501)) <= 1.5);
        // ...while a wall at a real angle drifts far past it on any run worth measuring.
        assert!(drift(&flat, &seg(0, 500, 100_000, 600)) > 1.5);
        assert!(drift(&flat, &seg(0, 500, 1_000, 600)) > 1.5);
    }
}
