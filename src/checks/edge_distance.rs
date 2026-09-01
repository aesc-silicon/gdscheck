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
use crate::merge::{Core, Edge, IntPoint, MergedCache};
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
    /// Facing boundaries, normals opposed, the span on the *inside* of both: material.
    /// One edge layer against itself — the two walls of a gate, and the width between.
    Width,
}

/// The outer layer's boundary must sit at least `value` outside the inner layer's.
pub fn run_enclosure(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Enclosure, false)
}

/// The two layers' boundaries must stay at least `value` apart.
pub fn run_space(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Space, false)
}

/// An edge layer's own facing pairs must span at least `value` of material — KLayout's
/// `width` on an edge collection. GF180's `O.PL.2` is the OTP gate length: the two walls
/// of the poly where it crosses the active, and no region carries that distance.
pub fn run_width(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Width, false)
}

/// The same span, bounded from above: the material between two facing walls must not be
/// *more* than `value` thick.  A transistor's channel length is the width between the
/// gate's own sides and GF180 caps it - MDN.3b at 20 µm - which no region carries either.
///
/// The measurement is the same one and the comparison is the only difference, but the
/// *search* is not.  A minimum only ever looks within its own limit, and the pairing
/// takes advantage of that by pairing edges filed in one tile; a maximum is violated
/// exactly by the pairs beyond the limit, which are the ones that reading never
/// generates.  So this path gathers the partner from the tiles the edge can see out to
/// [`MAX_REACH`] limits and reports the *nearest* partner it finds - the nearest is the
/// one the width is measured to, a wall further off having material in between.
pub fn run_max_width(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, Rel::Width, true)
}

/// How far past its own limit a maximum looks for the facing wall, in multiples of the
/// limit.  A width that exceeds the limit by more than this is not measured and so not
/// reported: the check would rather miss one than invent a width to a wall that is not
/// really opposite.
const MAX_REACH: f64 = 4.0;

/// One offending pair: the margin measured, and the two points that measure it.
type Pair = (f64, (f64, f64), (f64, f64));

/// Where the open span `p`..`q` crosses the segment `a`..`b`, as a fraction along it.
fn crossing(p: (f64, f64), q: (f64, f64), a: IntPoint, b: IntPoint) -> Option<f64> {
    let r = (q.0 - p.0, q.1 - p.1);
    let sg = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let denom = r.0 * sg.1 - r.1 * sg.0;
    if denom.abs() < 1e-9 {
        return None; // parallel: running along a wall is not crossing it
    }
    let d = (a.x as f64 - p.0, a.y as f64 - p.1);
    let u = (d.0 * r.1 - d.1 * r.0) / denom;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    Some((d.0 * sg.1 - d.1 * sg.0) / denom)
}

/// Whether the span from `p` to `q` stays in `tiles`' material the whole way.
///
/// A width is the thickness of *something*.  Two walls can face each other, each with its
/// own material behind it, and still have nothing but field in between - the outer sides
/// of two fingers of one gate, the two arms of a comb - and the distance across that gap
/// is not a thickness, it is a gap plus two thicknesses.  Pairing edges is a local test
/// and cannot tell the two apart: it sees the normals oppose and the partner lie on the
/// inward side, which is as true across a device as it is across a wall.
///
/// So the span is put to the region the edges were cut from.  It begins on one wall and
/// ends on the other, and if anything but those two ends interrupts it then the material
/// stops somewhere in between and there is no width here to measure.
fn span_is_material(
    tiles: &crate::merge::TileMap,
    tile: i64,
    p: (f64, f64),
    q: (f64, f64),
) -> bool {
    let len = (q.0 - p.0).hypot(q.1 - p.1);
    if len <= 1.0 {
        return true; // under a DBU: nothing can fit in it
    }
    // The span's own two ends sit on walls, which are boundary too; skip them.
    let eps = (0.5 / len).min(0.05);
    let (x0, x1) = (p.0.min(q.0), p.0.max(q.0));
    let (y0, y1) = (p.1.min(q.1), p.1.max(q.1));
    for tx in (x0 as i64).div_euclid(tile)..=(x1 as i64).div_euclid(tile) {
        for ty in (y0 as i64).div_euclid(tile)..=(y1 as i64).div_euclid(tile) {
            let Some(polys) = tiles.get(&(tx as i32, ty as i32)) else {
                continue;
            };
            for m in polys {
                for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
                    for i in 0..ring.len() {
                        let a = ring[i];
                        let b = ring[(i + 1) % ring.len()];
                        if let Some(t) = crossing(p, q, a, b)
                            && t > eps
                            && t < 1.0 - eps
                        {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

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
    at_most: bool,
) -> Vec<Violation> {
    let name = match (rel, at_most) {
        (Rel::Enclosure, _) => "min_enclosure",
        (Rel::Space, _) => "min_space",
        (Rel::Width, false) => "min_width",
        (Rel::Width, true) => "max_width",
    };
    // A width is one layer against itself; the others take two.
    let (Some(la), lb) = (
        rule.layers.first(),
        rule.layers.get(1).or(rule.layers.first()),
    ) else {
        eprintln!("[{}] {name} needs a layer", rule.id);
        return vec![];
    };
    let Some(lb) = lb else {
        eprintln!("[{}] {name} needs a layer", rule.id);
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
    // A width is measured *through* material, so the region the walls were cut from has
    // to be on hand to say whether the span stays in it.  The other two relations do not
    // need it: an enclosure and a spacing are both spans across ground the layers do not
    // claim, and neither says anything about what is in between.
    let base = if rel == Rel::Width && ka == kb {
        merged.edge_base_region(ka)
    } else {
        None
    };
    if let Some(b) = base {
        merged.ensure(layout, b.0, b.1);
    }

    println!(
        "[{}] Checking {name} {} {:.2} µm between edge layers {} and {}",
        rule.id,
        if at_most { "<=" } else { ">=" },
        rule.value,
        la.name,
        lb.name
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
    let base_tiles = base.map(|b| merged.tiles(b.0, b.1));
    // Half a DBU: coordinates are integers, so anything under this is a rounding artefact.
    let tol = 0.5;
    let mut out = Vec::new();

    // A minimum pairs within one tile: it only looks as far as its own limit, and an edge
    // is filed under the tile its midpoint falls in.  A maximum has to reach further, so
    // it collects the partner from the block of tiles its reach covers.
    let reach = if at_most { limit * MAX_REACH } else { 0.0 };
    let span = (reach / tile as f64).ceil() as i32 + 1;
    for (&(tx, ty), a_edges) in merged.edges(ka) {
        let gathered: Vec<Edge>;
        let b_edges: &[Edge] = if at_most {
            let mut v = Vec::new();
            for dx in -span..=span {
                for dy in -span..=span {
                    if let Some(es) = merged.edges(kb).get(&(tx + dx, ty + dy)) {
                        v.extend(es.iter().copied());
                    }
                }
            }
            gathered = v;
            &gathered
        } else {
            match merged.edges(kb).get(&(tx, ty)) {
                Some(v) => v.as_slice(),
                None => continue,
            }
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
                // enclosure or a width. One sign covers all three.
                let along = (sb.o.0 - sa.o.0) * sa.n.0 + (sb.o.1 - sa.o.1) * sa.n.1;
                let margin = if rel == Rel::Space { along } else { -along };
                // Behind this edge either way.  A minimum also drops everything already
                // far enough; a maximum needs those, since the nearest partner is what it
                // compares, and it caps the search at its reach instead.
                if margin < -tol || (!at_most && margin >= limit) || margin > reach.max(limit) {
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
                if let Some(t) = base_tiles {
                    // Measure across the middle of the run the two share, which is where
                    // a width is thickest if it varies along it at all.
                    let mid = (s0 + s1) * 0.5;
                    let f = (sa.o.0 + mid * sa.u.0, sa.o.1 + mid * sa.u.1);
                    let g = (f.0 - sa.n.0 * margin, f.1 - sa.n.1 * margin);
                    if !span_is_material(t, tile, f, g) {
                        continue;
                    }
                }
                // A minimum marks the span it measured, which is short enough to stand
                // for where the violation is.  A maximum's span is by definition longer
                // than the rule allows, and its middle is nowhere near either wall - so
                // that one marks the offending wall itself, the stretch of this edge that
                // faces the far one, which is what the reference draws too.
                let (p, q) = if at_most {
                    (
                        (sa.o.0 + s0 * sa.u.0, sa.o.1 + s0 * sa.u.1),
                        (sa.o.0 + s1 * sa.u.0, sa.o.1 + s1 * sa.u.1),
                    )
                } else {
                    let mid = (s0 + s1) * 0.5;
                    let p = (sa.o.0 + mid * sa.u.0, sa.o.1 + mid * sa.u.1);
                    let q = if rel != Rel::Space {
                        (p.0 - sa.n.0 * margin, p.1 - sa.n.1 * margin)
                    } else {
                        (p.0 + sa.n.0 * margin, p.1 + sa.n.1 * margin)
                    };
                    (p, q)
                };
                worst = Some((margin, p, q));
            }
            let Some((margin, p, q)) = worst else {
                continue;
            };
            if at_most && margin <= limit + tol {
                continue; // the nearest facing wall is close enough
            }
            // The pair is owned by the tile holding the middle of what it measures, so an
            // edge seen from two tiles is reported once.
            let (mx, my) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
            if !core.owns(mx, my) {
                continue;
            }
            let what = match rel {
                Rel::Enclosure => "enclosure",
                Rel::Space => "space",
                Rel::Width => "width",
            };
            let (title, cmp) = if at_most {
                ("Maximum width violation", ">")
            } else {
                (
                    match rel {
                        Rel::Enclosure => "Minimum enclosure violation",
                        Rel::Space => "Minimum space violation",
                        Rel::Width => "Minimum width violation",
                    },
                    "<",
                )
            };
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

    fn poly(pts: &[(i32, i32)]) -> crate::merge::MergedPoly {
        crate::merge::MergedPoly {
            outer: pts.iter().map(|&(x, y)| IntPoint { x, y }).collect(),
            holes: vec![],
        }
    }

    /// Two bars 100 DBU apart with a gap between them: the span from the outer wall of
    /// one to the outer wall of the other is 300 wide and is not a width of anything.
    #[test]
    fn a_span_across_a_gap_is_not_a_width() {
        let mut tiles = crate::merge::TileMap::new();
        tiles.insert(
            (0, 0),
            vec![
                poly(&[(0, 0), (100, 0), (100, 500), (0, 500)]),
                poly(&[(200, 0), (300, 0), (300, 500), (200, 500)]),
            ],
        );
        // Inside one bar: material all the way.
        assert!(span_is_material(
            &tiles,
            20_000,
            (0.0, 250.0),
            (100.0, 250.0)
        ));
        // Across both bars and the gap between them: interrupted.
        assert!(!span_is_material(
            &tiles,
            20_000,
            (0.0, 250.0),
            (300.0, 250.0)
        ));
    }

    /// A U leaves its two arms facing each other with material behind each, which is what
    /// the pairing sees; the span between them crosses the opening.
    #[test]
    fn a_span_across_the_mouth_of_a_u_is_not_a_width() {
        let mut tiles = crate::merge::TileMap::new();
        tiles.insert(
            (0, 0),
            vec![poly(&[
                (0, 0),
                (300, 0),
                (300, 500),
                (200, 500),
                (200, 100),
                (100, 100),
                (100, 500),
                (0, 500),
            ])],
        );
        // Across the base of the U, which is solid.
        assert!(span_is_material(&tiles, 20_000, (0.0, 50.0), (300.0, 50.0)));
        // Across its mouth, which is not.
        assert!(!span_is_material(
            &tiles,
            20_000,
            (0.0, 300.0),
            (300.0, 300.0)
        ));
    }
}
