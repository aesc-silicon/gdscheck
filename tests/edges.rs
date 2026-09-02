// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PDK-agnostic tests for the edge engine (`src/merge.rs`).
//!
//! A polygon layer's atom is a filled region, which leaves a class of rules unsayable:
//! the ones about *one piece of a boundary* rather than about a region. A transistor's
//! channel width is the length of the active-area boundary running under the gate — no
//! region has that length, and the source/drain's own width is a different quantity. An
//! edge layer makes those segments first-class.
//!
//! The cases below pin the two behaviours that are easy to get wrong:
//!
//! * An edge boolean is **not** an area boolean. `and` between two edge layers keeps the
//!   stretch they have *collinearly in common*, which may be part of a segment rather
//!   than all of it. Getting this wrong yields whole edges where a fragment was meant.
//! * `inside_part` **cuts** rather than selects. An edge crossing a polygon boundary is
//!   split, and only the piece inside is kept — unlike every polygon selector, which
//!   keeps or drops a region whole.

use gds21::{GdsBoundary, GdsPoint};
use gdscheck::layout::FlatLayout;
use gdscheck::merge::{Edge, EdgeOp, MergedCache};
use std::collections::HashMap;

const A: (i16, i16) = (1, 0);
const B: (i16, i16) = (2, 0);

/// One edge-layer declaration: the synthetic key it lands on, its op, and its sources.
type EdgeDef = ((i16, i16), EdgeOp, Vec<(i16, i16)>);

/// An axis-aligned rectangle, in DBU.
fn rect(l: (i16, i16), x0: i32, y0: i32, x1: i32, y1: i32) -> GdsBoundary {
    GdsBoundary {
        layer: l.0,
        datatype: l.1,
        xy: GdsPoint::vec(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]),
        ..Default::default()
    }
}

/// Build a cache over layers A and B, register the edge layers `defs` names, and return
/// the resulting segments of the last one, as sorted `(x0,y0,x1,y1)` endpoints.
fn run(a: &[GdsBoundary], b: &[GdsBoundary], defs: &[EdgeDef]) -> Vec<(i32, i32, i32, i32)> {
    run_tiled(a, b, defs, 10_000_000)
}

/// [`run`] with the tile pitch spelled out, for the cases that are about tiling.
fn run_tiled(
    a: &[GdsBoundary],
    b: &[GdsBoundary],
    defs: &[EdgeDef],
    tile: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let mut layout = FlatLayout::new();
    for s in a {
        layout.insert(A.0, A.1, s.clone());
    }
    for s in b {
        layout.insert(B.0, B.1, s.clone());
    }
    let mut cache = MergedCache::new(tile, 0, HashMap::new());
    for (key, op, sources) in defs {
        cache.register_edge(*key, *op, sources.clone());
    }
    let last = defs.last().expect("at least one edge layer").0;
    cache.ensure_edges(&layout, last);

    let mut out: Vec<(i32, i32, i32, i32)> = cache
        .edges(last)
        .values()
        .flatten()
        .map(|e: &Edge| {
            // Normalise direction so a segment compares equal whichever way it is walked.
            let (p, q) = ((e.a.x, e.a.y), (e.b.x, e.b.y));
            if p <= q {
                (p.0, p.1, q.0, q.1)
            } else {
                (q.0, q.1, p.0, p.1)
            }
        })
        .collect();
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

#[test]
fn edges_returns_every_contour_segment() {
    let got = run(
        &[rect(A, 0, 0, 100, 50)],
        &[],
        &[((900, 0), EdgeOp::Edges, vec![A])],
    );
    assert_eq!(
        got,
        vec![
            (0, 0, 0, 50),     // left
            (0, 0, 100, 0),    // bottom
            (0, 50, 100, 50),  // top
            (100, 0, 100, 50), // right
        ]
    );
}

/// A region's holes are boundary too — a rule about the inner wall of a ring needs them.
#[test]
fn edges_includes_hole_contours() {
    let ring = vec![
        rect(A, 0, 0, 100, 20),
        rect(A, 0, 80, 100, 100),
        rect(A, 0, 0, 20, 100),
        rect(A, 80, 0, 100, 100),
    ];
    let got = run(&ring, &[], &[((900, 0), EdgeOp::Edges, vec![A])]);
    assert_eq!(got.len(), 8, "four outer and four hole segments: {got:?}");
    assert!(
        got.contains(&(20, 20, 20, 80)),
        "hole's left wall missing: {got:?}"
    );
}

// ---------------------------------------------------------------------------
// Edge booleans — collinear overlap, not shared area
// ---------------------------------------------------------------------------

/// The whole point of an edge `and`: two rectangles abutting along part of one side
/// share exactly that stretch, and the result is the *fragment*, not either full edge.
#[test]
fn and_keeps_only_the_common_stretch() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    let b = vec![rect(B, 100, 20, 200, 70)]; // touches A's right wall over y 20..70
    let got = run(
        &a,
        &b,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Edges, vec![B]),
            ((902, 0), EdgeOp::And, vec![(900, 0), (901, 0)]),
        ],
    );
    assert_eq!(
        got,
        vec![(100, 20, 100, 70)],
        "expected the shared 50-long fragment of the wall, got {got:?}"
    );
}

/// Two layers that overlap in *area* but share no boundary have no common edge at all —
/// the distinction between an edge boolean and an area one.
#[test]
fn and_is_empty_when_the_boundaries_do_not_coincide() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    let b = vec![rect(B, 40, 40, 200, 60)]; // overlaps A's interior, no shared wall
    let got = run(
        &a,
        &b,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Edges, vec![B]),
            ((902, 0), EdgeOp::And, vec![(900, 0), (901, 0)]),
        ],
    );
    assert!(got.is_empty(), "area overlap is not edge overlap: {got:?}");
}

/// `not` removes the shared stretch and keeps the gaps either side of it, so one edge
/// becomes two.
#[test]
fn not_splits_an_edge_around_the_shared_stretch() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    let b = vec![rect(B, 100, 20, 200, 70)];
    let got = run(
        &a,
        &b,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Edges, vec![B]),
            ((902, 0), EdgeOp::Not, vec![(900, 0), (901, 0)]),
        ],
    );
    assert!(
        got.contains(&(100, 0, 100, 20)),
        "lower remnant missing: {got:?}"
    );
    assert!(
        got.contains(&(100, 70, 100, 100)),
        "upper remnant missing: {got:?}"
    );
    assert!(
        !got.contains(&(100, 0, 100, 100)),
        "the full wall should not survive: {got:?}"
    );
}

// ---------------------------------------------------------------------------
// Edge against polygon — cutting, not selecting
// ---------------------------------------------------------------------------

/// `inside_part` cuts an edge where it crosses the polygon's boundary. Every polygon
/// selector keeps or drops a whole region; this one keeps a piece of a segment.
#[test]
fn inside_part_cuts_the_edge_at_the_boundary() {
    let a = vec![rect(A, 0, 0, 100, 10)]; // a long thin shape, bottom edge y=0
    let b = vec![rect(B, 30, -10, 60, 20)]; // covers x 30..60 of it
    let got = run(
        &a,
        &b,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::InsidePart, vec![(900, 0), B]),
        ],
    );
    assert!(
        got.contains(&(30, 0, 60, 0)),
        "bottom edge should be cut to x 30..60: {got:?}"
    );
    assert!(
        got.iter().all(|e| e.0 >= 30 && e.2 <= 60),
        "nothing outside the polygon should survive: {got:?}"
    );
}

#[test]
fn outside_part_keeps_the_complement() {
    let a = vec![rect(A, 0, 0, 100, 10)];
    let b = vec![rect(B, 30, -10, 60, 20)];
    let got = run(
        &a,
        &b,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::OutsidePart, vec![(900, 0), B]),
        ],
    );
    assert!(
        got.contains(&(0, 0, 30, 0)),
        "left remnant missing: {got:?}"
    );
    assert!(
        got.contains(&(60, 0, 100, 0)),
        "right remnant missing: {got:?}"
    );
}

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

#[test]
fn with_length_bounds_are_half_open() {
    let a = vec![rect(A, 0, 0, 100, 50)]; // sides of 100 and 50
    let defs = |op: EdgeOp| -> Vec<EdgeDef> {
        vec![
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), op, vec![(900, 0)]),
        ]
    };
    // [50, 100): keeps the two 50-long sides, not the 100-long ones.
    let got = run(&a, &[], &defs(EdgeOp::WithLength(Some(50), Some(100))));
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(
        got.iter().all(|e| e.0 == e.2),
        "expected the vertical sides: {got:?}"
    );
    // The complement keeps exactly the others.
    let got = run(&a, &[], &defs(EdgeOp::WithoutLength(Some(50), Some(100))));
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(
        got.iter().all(|e| e.1 == e.3),
        "expected the horizontal sides: {got:?}"
    );
}

/// Orientation is normalised to `[0, 180)`, so an edge reads the same whichever way the
/// contour walks it — otherwise a rule would catch a wall on one side of a shape and
/// miss the identical wall opposite.
#[test]
fn with_angle_selects_by_orientation() {
    let a = vec![rect(A, 0, 0, 100, 50)];
    let got = run(
        &a,
        &[],
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::WithAngle(90, 90), vec![(900, 0)]),
        ],
    );
    assert_eq!(got.len(), 2, "both vertical walls, not one: {got:?}");
    assert!(got.iter().all(|e| e.0 == e.2), "{got:?}");
}

// ---------------------------------------------------------------------------
// The composition this engine exists for
// ---------------------------------------------------------------------------

/// Channel width, reduced. The real rule is
/// `comp.not(poly2).edges.and(tgate.edges).with_length(...)`, and the `not(poly2)` is
/// load-bearing: a gate merely *crossing* an active shares no collinear boundary with
/// it, so an edge `and` between the two would be empty. Subtracting the gate first makes
/// the source/drain's inner wall coincide with the gate's outline — and the length of
/// that shared wall is the channel width W.
///
/// Here the source/drain is pre-cut into the two rectangles the subtraction would give,
/// with the gate between them. W is 50, the active's height, while each source/drain
/// region is 200 x 50 — so no region-level width measurement yields the same number.
#[test]
fn channel_width_is_the_active_boundary_under_the_gate() {
    let source_drain = vec![rect(A, 0, 0, 200, 50), rect(A, 300, 0, 500, 50)];
    let gate = vec![rect(B, 200, 0, 300, 50)];
    let got = run(
        &source_drain,
        &gate,
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Edges, vec![B]),
            ((902, 0), EdgeOp::And, vec![(900, 0), (901, 0)]),
        ],
    );
    assert_eq!(
        got,
        vec![(200, 0, 200, 50), (300, 0, 300, 50)],
        "expected the two channel-width segments, got {got:?}"
    );
    // Both are 50 long: that is W, and it is what min_edge_length bounds.
    for e in &got {
        let len = ((e.2 - e.0).abs()).max((e.3 - e.1).abs());
        assert_eq!(len, 50, "channel width should be 50: {e:?}");
    }
}

// ---------------------------------------------------------------------------
// centers
// ---------------------------------------------------------------------------

/// `centers` keeps the middle of each edge, trimming half the remainder from each end.
/// A 100-long edge at 0.9 keeps 90, centred: 5 off each end.
#[test]
fn centers_trims_both_ends_symmetrically() {
    let got = run(
        &[rect(A, 0, 0, 100, 100)],
        &[],
        &[
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Centers(0, 900), vec![(900, 0)]),
        ],
    );
    assert_eq!(
        got,
        vec![
            (0, 5, 0, 95),
            (5, 0, 95, 0),
            (5, 100, 95, 100),
            (100, 5, 100, 95),
        ]
    );
}

/// The absolute length and the fraction are alternatives and the *longer* wins, so a
/// generous absolute bound overrides a mean fraction — and neither may exceed the edge.
#[test]
fn centers_takes_the_longer_of_length_and_fraction() {
    let edges = |op: EdgeOp| -> Vec<(i32, i32, i32, i32)> {
        run(
            &[rect(A, 0, 0, 100, 100)],
            &[],
            &[
                ((900, 0), EdgeOp::Edges, vec![A]),
                ((901, 0), op, vec![(900, 0)]),
            ],
        )
    };
    // 80 absolute beats 50%, so 80 is kept, not 50.
    assert!(edges(EdgeOp::Centers(80, 500)).contains(&(10, 0, 90, 0)));
    // A length longer than the edge cannot grow it.
    assert!(edges(EdgeOp::Centers(500, 0)).contains(&(0, 0, 100, 0)));
}

/// Why the op exists: two edges meeting at a corner touch at that vertex, so an
/// `interacting` between them says yes for a reason the rule does not mean.  Trimming the
/// ends makes contact mean overlap.
#[test]
fn centers_breaks_contact_at_a_shared_corner() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    let b = vec![rect(B, 100, 100, 200, 200)]; // shares only the corner (100, 100)
    let defs = |trim: bool| -> Vec<EdgeDef> {
        let mut d = vec![
            ((900, 0), EdgeOp::Edges, vec![A]),
            ((901, 0), EdgeOp::Edges, vec![B]),
        ];
        if trim {
            d.push(((902, 0), EdgeOp::Centers(0, 990), vec![(901, 0)]));
            d.push(((903, 0), EdgeOp::InteractingEdges, vec![(900, 0), (902, 0)]));
        } else {
            d.push(((903, 0), EdgeOp::InteractingEdges, vec![(900, 0), (901, 0)]));
        }
        d
    };
    assert!(
        !run(&a, &b, &defs(false)).is_empty(),
        "untrimmed, the shared corner counts as contact"
    );
    assert!(
        run(&a, &b, &defs(true)).is_empty(),
        "trimmed, it should not"
    );
}

/// Two rectangles drawn edge to edge are one shape, so the wall they share is interior
/// and not boundary.  An `.edges` layer that keeps it reports a wall where the material
/// is continuous, and every edge boolean downstream inherits the mistake.
#[test]
fn edges_drops_the_wall_between_abutting_shapes() {
    let got = run(
        &[rect(A, 0, 0, 100, 100), rect(A, 100, 0, 200, 100)],
        &[],
        &[((900, 0), EdgeOp::Edges, vec![A])],
    );
    assert!(
        !got.contains(&(100, 0, 100, 100)),
        "the shared wall is interior: {got:?}"
    );
    assert_eq!(got.len(), 4, "one rectangle's worth of boundary: {got:?}");
}

/// The same where the abutment is partial — the fixture shape that found this: a wide pad
/// with a narrow link off one side.  The pad's wall survives only where the link is not.
#[test]
fn a_partial_abutment_splits_the_shared_wall() {
    let got = run(
        &[rect(A, 0, 0, 100, 100), rect(A, 100, 40, 200, 60)],
        &[],
        &[((900, 0), EdgeOp::Edges, vec![A])],
    );
    assert!(
        !got.contains(&(100, 0, 100, 100)),
        "the pad's wall must not span the link: {got:?}"
    );
    assert!(got.contains(&(100, 0, 100, 40)), "lower remnant: {got:?}");
    assert!(got.contains(&(100, 60, 100, 100)), "upper remnant: {got:?}");
}

/// Two abutting rectangles bound the same region as the single rectangle they make, so
/// `edges` should report the same four segments for both.  It does not, once the pair
/// spans more than one tile: each tile merges only what its own bucket holds, so the seam
/// is gone in the tiles that see both rectangles and present in the tiles that see one,
/// and the midpoint-ownership filter then hands each edge to a single tile without
/// checking that that tile drew the same boundary.  The bottom edge comes back three
/// times - whole, and as each half - and the right-hand edge does not come back at all,
/// because it lies on the far boundary of the last tile with core area and so is charged
/// to a tile that holds no copy of the polygon.
///
/// Found through GF180 GR.1 on a real seal ring, whose active is drawn as four corners
/// and four bars: the rule read four false violations off the corner pieces.  GR.1 is a
/// region boolean now and no longer depends on this, but every other `edges` layer still
/// does.
#[test]
#[ignore = "known defect: `edges` of a multi-piece region depends on the tiling"]
fn edges_of_abutting_rectangles_match_the_single_rectangle() {
    let defs: &[EdgeDef] = &[((90, 0), EdgeOp::Edges, vec![A])];
    let split = run_tiled(
        &[rect(A, 0, 0, 30, 30), rect(A, 30, 0, 60, 30)],
        &[],
        defs,
        20,
    );
    let whole = run_tiled(&[rect(A, 0, 0, 60, 30)], &[], defs, 20);
    assert_eq!(split, whole);
}
