// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PDK-agnostic tests for the virtual-layer operators (`src/merge.rs`).
//!
//! Every op is exercised on geometry built here rather than on a foundry fixture: an
//! operator is engine behaviour, so what it does to two rectangles must be pinned down
//! independently of whether any PDK happens to use it.  Deck- and PDK-level wiring of
//! these ops (`op:` string parsing, eager vs. lazy materialisation) lives in
//! `tests/engine.rs`; the ihp-* suites then check them against reference results.
//!
//! The cases are chosen around the distinctions that are easy to get wrong and silent
//! when wrong: `overlapping` vs. `interacting` on zero-area contact, `inside` vs.
//! `overlapping` on a partly-covered region, `covering` vs. `overlapping` on a filter
//! region that straddles the candidate's edge, and the directional sizes leaving the
//! perpendicular axis exactly as drawn.

use gds21::{GdsBoundary, GdsPoint};
use gdscheck::layout::FlatLayout;
use gdscheck::merge::{MergedCache, MergedPoly, VirtualOp};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Candidate/source layer, filter layer, and the synthetic key the virtual lands on.
const A: (i16, i16) = (1, 0);
const B: (i16, i16) = (2, 0);
const OUT: (i16, i16) = (900, 0);

/// An axis-aligned rectangle on layer `l`, in DBU.
fn rect(l: (i16, i16), x0: i32, y0: i32, x1: i32, y1: i32) -> GdsBoundary {
    GdsBoundary {
        layer: l.0,
        datatype: l.1,
        xy: GdsPoint::vec(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]),
        ..Default::default()
    }
}

/// Bounding box of a merged region, in DBU.
fn bbox(m: &MergedPoly) -> (i32, i32, i32, i32) {
    let xs = m.outer.iter().map(|p| p.x);
    let ys = m.outer.iter().map(|p| p.y);
    (
        xs.clone().min().unwrap(),
        ys.clone().min().unwrap(),
        xs.max().unwrap(),
        ys.max().unwrap(),
    )
}

/// Run `op` over sources `a` (and `b`, if any) and return the resulting regions' bounding
/// boxes, sorted.  One tile large enough to hold everything and no halo, so a region is
/// never split and per-tile results need no stitching.
fn run(op: VirtualOp, a: &[GdsBoundary], b: &[GdsBoundary]) -> Vec<(i32, i32, i32, i32)> {
    run_tiled(op, a, b, 10_000_000)
}

/// As [`run`], but with an explicit tile size so a region can be made to span tiles.
fn run_tiled(
    op: VirtualOp,
    a: &[GdsBoundary],
    b: &[GdsBoundary],
    tile_dbu: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let mut layout = FlatLayout::new();
    for s in a {
        layout.insert(A.0, A.1, s.clone());
    }
    for s in b {
        layout.insert(B.0, B.1, s.clone());
    }
    let sources = if b.is_empty() { vec![A] } else { vec![A, B] };

    let mut cache = MergedCache::new(tile_dbu, 0, HashMap::new());
    cache.register_virtual(OUT, op, sources, None);
    cache.ensure(&layout, OUT.0, OUT.1);

    // `build_tiled_merge` buckets a shape into every tile its bbox touches, un-clipped,
    // so a region spanning tiles is reported once per tile.  That duplication is an
    // artifact of the tiling, not of the operator, so collapse it — with a single giant
    // tile (the default) there is nothing to collapse anyway.
    let mut out: Vec<(i32, i32, i32, i32)> = cache
        .tiles(OUT.0, OUT.1)
        .values()
        .flatten()
        .map(bbox)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

// ---------------------------------------------------------------------------
// Region selection: overlapping / interacting
//
// The two differ *only* on zero-area contact, so both are tested against the same
// three candidates: one overlapping the filter, one touching it edge-to-edge, one clear
// of it.  Getting these two confused is silent — the layer still builds, just with the
// wrong regions — so the distinction is pinned here rather than inferred from a PDK.
// ---------------------------------------------------------------------------

/// Candidates at x = 50..150 (overlaps the filter), 200..300 (touches it at x=200) and
/// 400..500 (clear); filter = 0..200.
fn contact_case() -> (Vec<GdsBoundary>, Vec<GdsBoundary>) {
    let a = vec![
        rect(A, 50, 0, 150, 100),
        rect(A, 200, 0, 300, 100),
        rect(A, 400, 0, 500, 100),
    ];
    let b = vec![rect(B, 0, 0, 200, 100)];
    (a, b)
}

#[test]
fn overlapping_needs_area_and_drops_a_touching_region() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::Overlapping, &a, &b),
        vec![(50, 0, 150, 100)],
        "only the region sharing positive area with the filter should survive"
    );
}

#[test]
fn not_overlapping_keeps_the_touching_region() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::NotOverlapping, &a, &b),
        vec![(200, 0, 300, 100), (400, 0, 500, 100)],
        "edge contact is not overlap, so the touching region is kept"
    );
}

#[test]
fn interacting_counts_edge_contact() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::Interacting, &a, &b),
        vec![(50, 0, 150, 100), (200, 0, 300, 100)],
        "interacting keeps both the overlapping and the merely touching region"
    );
}

#[test]
fn not_interacting_keeps_only_the_clear_region() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::NotInteracting, &a, &b),
        vec![(400, 0, 500, 100)]
    );
}

/// A shared *corner* is the weakest form of contact — zero-area and a single point, so a
/// segment-intersection test has to catch it at the vertices rather than mid-edge.
#[test]
fn interacting_counts_a_shared_corner() {
    let a = vec![rect(A, 100, 100, 200, 200)];
    let b = vec![rect(B, 0, 0, 100, 100)];
    assert_eq!(
        run(VirtualOp::Interacting, &a, &b),
        vec![(100, 100, 200, 200)]
    );
    assert!(run(VirtualOp::Overlapping, &a, &b).is_empty());
}

/// An empty filter matches nothing, so the positive selectors keep nothing and the
/// negative ones keep everything — including `inside`, since nothing is contained in
/// nothing.  This is the short-circuit path in `build_selection_tiles`.
#[test]
fn empty_filter_matches_nothing() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    for op in [
        VirtualOp::Overlapping,
        VirtualOp::Interacting,
        VirtualOp::Inside,
        VirtualOp::Covering,
    ] {
        assert!(run(op, &a, &[]).is_empty(), "{op:?} should match nothing");
    }
    for op in [
        VirtualOp::NotOverlapping,
        VirtualOp::NotInteracting,
        VirtualOp::NotInside,
        VirtualOp::NotCovering,
    ] {
        assert_eq!(run(op, &a, &[]), vec![(0, 0, 100, 100)], "{op:?}");
    }
}

// ---------------------------------------------------------------------------
// Region selection: inside / covering
// ---------------------------------------------------------------------------

/// `inside` needs the *whole* region covered — a region that merely overlaps the filter
/// is dropped, which is what separates it from `overlapping`.
#[test]
fn inside_requires_full_containment() {
    let a = vec![
        rect(A, 10, 10, 90, 90),   // fully inside the filter
        rect(A, 150, 10, 250, 90), // straddles the filter's right edge
        rect(A, 300, 10, 400, 90), // clear of the filter
    ];
    let b = vec![rect(B, 0, 0, 200, 100)];
    assert_eq!(run(VirtualOp::Inside, &a, &b), vec![(10, 10, 90, 90)]);
    assert_eq!(
        run(VirtualOp::NotInside, &a, &b),
        vec![(150, 10, 250, 90), (300, 10, 400, 90)]
    );
    // The straddling region *does* overlap, so `overlapping` keeps it — the distinction
    // `inside` exists to make.
    assert_eq!(
        run(VirtualOp::Overlapping, &a, &b),
        vec![(10, 10, 90, 90), (150, 10, 250, 90)]
    );
}

/// A region touching the filter's edge from outside shares its boundary but no area, so
/// it is not inside it.
#[test]
fn inside_rejects_a_region_that_only_touches() {
    let a = vec![rect(A, 200, 0, 300, 100)];
    let b = vec![rect(B, 0, 0, 200, 100)];
    assert!(run(VirtualOp::Inside, &a, &b).is_empty());
}

/// `inside` is the one selector that reduces with AND across a region's tile pieces: with
/// a tile smaller than the region, the region is split into several pieces and *every*
/// piece must be covered.  Here a 300-wide candidate spans three 100-wide tiles while the
/// filter covers only the first two, so the region must be dropped as a unit — an OR
/// reduction (like the other selectors use) would wrongly keep it.
#[test]
fn inside_reduces_with_and_across_tiles() {
    let a = vec![rect(A, 0, 0, 300, 50)];
    let covered = vec![rect(B, -10, -10, 310, 60)];
    let partial = vec![rect(B, -10, -10, 200, 60)];
    assert_eq!(
        run_tiled(VirtualOp::Inside, &a, &covered, 100),
        vec![(0, 0, 300, 50)],
        "a fully covered region survives even when split across tiles"
    );
    assert!(
        run_tiled(VirtualOp::Inside, &a, &partial, 100).is_empty(),
        "one uncovered piece must drop the whole region"
    );
}

/// `covering` is containment the other way round: keep candidates that hold a whole
/// filter region.  A filter region straddling the candidate's edge does not count — the
/// case where the old alias-to-`interacting` behaviour differed.
#[test]
fn covering_requires_containing_a_whole_filter_region() {
    let a = vec![
        rect(A, 0, 0, 100, 100),   // contains the small filter region
        rect(A, 200, 0, 300, 100), // the filter only straddles its edge
        rect(A, 400, 0, 500, 100), // no filter near it
    ];
    let b = vec![
        rect(B, 20, 20, 80, 80),   // wholly inside the first candidate
        rect(B, 250, 20, 350, 80), // half in, half out of the second
    ];
    assert_eq!(run(VirtualOp::Covering, &a, &b), vec![(0, 0, 100, 100)]);
    assert_eq!(
        run(VirtualOp::NotCovering, &a, &b),
        vec![(200, 0, 300, 100), (400, 0, 500, 100)]
    );
    // `overlapping` keeps the straddled candidate too — the distinction `covering` makes.
    assert_eq!(
        run(VirtualOp::Overlapping, &a, &b),
        vec![(0, 0, 100, 100), (200, 0, 300, 100)]
    );
}

// ---------------------------------------------------------------------------
// Shape filters
// ---------------------------------------------------------------------------

/// An L-shape (two merged rectangles) is not a rectangle; a plain rectangle is, whether
/// or not it is square.  A rectangle with a hole is not filled, so it fails too.
#[test]
fn rectangle_filter_splits_filled_rectangles_from_the_rest() {
    let a = vec![
        rect(A, 0, 0, 100, 50),      // rectangle
        rect(A, 200, 0, 300, 100),   // square (also a rectangle)
        rect(A, 400, 0, 500, 100),   // \ these two merge into an L
        rect(A, 400, 100, 450, 200), // /
    ];
    assert_eq!(
        run(VirtualOp::Rectangle, &a, &[]),
        vec![(0, 0, 100, 50), (200, 0, 300, 100)]
    );
    assert_eq!(
        run(VirtualOp::NotRectangle, &a, &[]),
        vec![(400, 0, 500, 200)],
        "the merged L-shape is the only non-rectangle"
    );
    // `square` is the stricter special case: equal sides.
    assert_eq!(run(VirtualOp::Square, &a, &[]), vec![(200, 0, 300, 100)]);
}

/// A ring is a rectangle by outline but not filled, so it is a non-rectangle.
#[test]
fn rectangle_filter_rejects_a_region_with_a_hole() {
    let a = vec![
        rect(A, 0, 0, 100, 20),
        rect(A, 0, 80, 100, 100),
        rect(A, 0, 0, 20, 100),
        rect(A, 80, 0, 100, 100),
    ];
    assert!(run(VirtualOp::Rectangle, &a, &[]).is_empty());
    assert_eq!(
        run(VirtualOp::NotRectangle, &a, &[]),
        vec![(0, 0, 100, 100)]
    );
}

/// `with_bbox_min` filters on the *shorter* bbox side, `with_bbox_max` on the longer, and
/// the range is half-open `[min, max)` — so a side sitting exactly on `min` is kept and
/// one exactly on `max` is not.
#[test]
fn bbox_filters_use_the_short_and_long_side() {
    let a = vec![
        rect(A, 0, 0, 100, 10),    // short 10,  long 100
        rect(A, 200, 0, 250, 50),  // short 50,  long 50
        rect(A, 400, 0, 500, 200), // short 100, long 200
    ];
    assert_eq!(
        run(VirtualOp::WithBBoxMin(Some(50), None), &a, &[]),
        vec![(200, 0, 250, 50), (400, 0, 500, 200)],
        "min is inclusive: the 50-wide region is kept"
    );
    assert_eq!(
        run(VirtualOp::WithBBoxMin(None, Some(50)), &a, &[]),
        vec![(0, 0, 100, 10)],
        "max is exclusive: the 50-wide region is not kept"
    );
    assert_eq!(
        run(VirtualOp::WithBBoxMax(Some(100), Some(200)), &a, &[]),
        vec![(0, 0, 100, 10)],
        "long side in [100, 200): 100 qualifies, 200 does not"
    );
    // Same bounds, different metric: nothing has its *short* side in [100, 200) except
    // the third region, whose short side is exactly 100.
    assert_eq!(
        run(VirtualOp::WithBBoxMin(Some(100), Some(200)), &a, &[]),
        vec![(400, 0, 500, 200)]
    );
}

// ---------------------------------------------------------------------------
// Directional sizing
//
// The point of these over `grow`/`open` is that the perpendicular axis is left exactly
// as drawn, so every case asserts the untouched extent as well as the changed one.
// ---------------------------------------------------------------------------

#[test]
fn grow_x_extends_only_the_x_extent() {
    let a = vec![rect(A, 0, 0, 100, 50)];
    assert_eq!(run(VirtualOp::GrowX(10), &a, &[]), vec![(-10, 0, 110, 50)]);
    assert_eq!(run(VirtualOp::GrowY(10), &a, &[]), vec![(0, -10, 100, 60)]);
}

/// When the grow distance exceeds the region's own width the two endpoint translates no
/// longer overlap, and the result is only correct because the swept edge bands close the
/// gap between them.  A naive "union of two shifted copies" would return two fragments.
#[test]
fn grow_x_stays_connected_on_a_narrow_region() {
    let a = vec![rect(A, 0, 0, 5, 50)];
    assert_eq!(
        run(VirtualOp::GrowX(20), &a, &[]),
        vec![(-20, 0, 25, 50)],
        "one region, not two shifted copies"
    );
}

#[test]
fn shrink_x_erodes_only_the_x_extent() {
    let a = vec![rect(A, 0, 0, 100, 50)];
    assert_eq!(run(VirtualOp::ShrinkX(10), &a, &[]), vec![(10, 0, 90, 50)]);
    assert_eq!(run(VirtualOp::ShrinkY(10), &a, &[]), vec![(0, 10, 100, 40)]);
}

/// A region narrower than `2 * radius` along the shrink axis disappears; the same region
/// survives a shrink along the other axis.  This is what makes the erode usable as a
/// "narrower than X" test.
#[test]
fn shrink_x_removes_a_region_narrower_than_twice_the_radius() {
    let a = vec![rect(A, 0, 0, 15, 200)];
    assert!(run(VirtualOp::ShrinkX(10), &a, &[]).is_empty());
    assert_eq!(run(VirtualOp::ShrinkY(10), &a, &[]), vec![(0, 10, 15, 190)]);
}

/// Grow and shrink are inverses on a region wide enough to survive the erode: the
/// round trip restores the original extent exactly.
#[test]
fn shrink_then_grow_restores_a_wide_region() {
    let a = vec![rect(A, 0, 0, 1000, 800)];
    let shrunk = run(VirtualOp::ShrinkX(100), &a, &[]);
    assert_eq!(shrunk, vec![(100, 0, 900, 800)]);
    let back = run(VirtualOp::GrowX(100), &[rect(A, 100, 0, 900, 800)], &[]);
    assert_eq!(back, vec![(0, 0, 1000, 800)]);
    let _ = shrunk;
}

/// The chain the GF180 wide-metal rules are built on: erode in x, erode in y, then grow
/// both back (KLayout `sized(-r,0).sized(0,-r).sized(r,0).sized(0,r)`).  It is a
/// morphological opening with a box structuring element, so it keeps regions at least
/// `2 * r` across in *both* axes and erases everything else — a wide plate survives, a
/// long thin wire does not.
#[test]
fn directional_open_chain_keeps_only_wide_regions() {
    let r = 50;
    // A 400×400 plate (wide both ways) and a 400×60 wire (too thin in y).
    let a = vec![rect(A, 0, 0, 400, 400), rect(A, 0, 1000, 400, 1060)];

    let mut layout = FlatLayout::new();
    for s in &a {
        layout.insert(A.0, A.1, s.clone());
    }
    let mut cache = MergedCache::new(10_000_000, 0, HashMap::new());
    cache.register_virtual((901, 0), VirtualOp::ShrinkX(r), vec![A], None);
    cache.register_virtual((902, 0), VirtualOp::ShrinkY(r), vec![(901, 0)], None);
    cache.register_virtual((903, 0), VirtualOp::GrowX(r), vec![(902, 0)], None);
    cache.register_virtual((904, 0), VirtualOp::GrowY(r), vec![(903, 0)], None);
    cache.ensure(&layout, 904, 0);

    let mut out: Vec<(i32, i32, i32, i32)> =
        cache.tiles(904, 0).values().flatten().map(bbox).collect();
    out.sort_unstable();
    assert_eq!(
        out,
        vec![(0, 0, 400, 400)],
        "the 400x400 plate is restored; the 400x60 wire is erased"
    );
}
