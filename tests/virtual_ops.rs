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
        run(VirtualOp::Overlapping(None, None), &a, &b),
        vec![(50, 0, 150, 100)],
        "only the region sharing positive area with the filter should survive"
    );
}

#[test]
fn not_overlapping_keeps_the_touching_region() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::NotOverlapping(None, None), &a, &b),
        vec![(200, 0, 300, 100), (400, 0, 500, 100)],
        "edge contact is not overlap, so the touching region is kept"
    );
}

#[test]
fn interacting_counts_edge_contact() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::Interacting(None, None), &a, &b),
        vec![(50, 0, 150, 100), (200, 0, 300, 100)],
        "interacting keeps both the overlapping and the merely touching region"
    );
}

#[test]
fn not_interacting_keeps_only_the_clear_region() {
    let (a, b) = contact_case();
    assert_eq!(
        run(VirtualOp::NotInteracting(None, None), &a, &b),
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
        run(VirtualOp::Interacting(None, None), &a, &b),
        vec![(100, 100, 200, 200)]
    );
    assert!(run(VirtualOp::Overlapping(None, None), &a, &b).is_empty());
}

/// An empty filter matches nothing, so the positive selectors keep nothing and the
/// negative ones keep everything — including `inside`, since nothing is contained in
/// nothing.  This is the short-circuit path in `build_selection_tiles`.
#[test]
fn empty_filter_matches_nothing() {
    let a = vec![rect(A, 0, 0, 100, 100)];
    for op in [
        VirtualOp::Overlapping(None, None),
        VirtualOp::Interacting(None, None),
        VirtualOp::Inside,
        VirtualOp::Covering(None, None),
    ] {
        assert!(run(op, &a, &[]).is_empty(), "{op:?} should match nothing");
    }
    for op in [
        VirtualOp::NotOverlapping(None, None),
        VirtualOp::NotInteracting(None, None),
        VirtualOp::NotInside,
        VirtualOp::NotCovering(None, None),
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
        run(VirtualOp::Overlapping(None, None), &a, &b),
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
    assert_eq!(
        run(VirtualOp::Covering(None, None), &a, &b),
        vec![(0, 0, 100, 100)]
    );
    assert_eq!(
        run(VirtualOp::NotCovering(None, None), &a, &b),
        vec![(200, 0, 300, 100), (400, 0, 500, 100)]
    );
    // `overlapping` keeps the straddled candidate too — the distinction `covering` makes.
    assert_eq!(
        run(VirtualOp::Overlapping(None, None), &a, &b),
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

/// `shrink` erodes in every direction at once, where `shrink_x`/`shrink_y` take one
/// axis.  A region narrower than twice the radius vanishes entirely - there is no
/// dilate back, which is what separates it from `open`.
#[test]
fn shrink_erodes_isotropically() {
    let a = vec![rect(A, 0, 0, 1000, 800)];
    assert_eq!(
        run(VirtualOp::Shrink(100), &a, &[]),
        vec![(100, 100, 900, 700)]
    );
    // A shape thinner than 2 x radius in one axis disappears.
    let thin = vec![rect(A, 0, 0, 150, 800)];
    assert!(run(VirtualOp::Shrink(100), &thin, &[]).is_empty());
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

// ---------------------------------------------------------------------------
// Counted selection
// ---------------------------------------------------------------------------

/// Three candidates over one filter row: the left one meets two filter shapes, the middle
/// one, the right none.  Enough to separate every count bound from every other.
fn counted_case() -> (Vec<GdsBoundary>, Vec<GdsBoundary>) {
    let a = vec![
        rect(A, 0, 0, 300, 100),   // spans both filter shapes
        rect(A, 400, 0, 500, 100), // spans the third only
        rect(A, 800, 0, 900, 100), // spans none
    ];
    let b = vec![
        rect(B, 50, 0, 100, 100),
        rect(B, 200, 0, 250, 100),
        rect(B, 420, 0, 470, 100),
    ];
    (a, b)
}

/// `interacting(other, 2, 2)` is "exactly two", not "at least two": the candidate meeting
/// one is dropped as surely as the candidate meeting none.
#[test]
fn interacting_bounds_are_an_inclusive_count() {
    let (a, b) = counted_case();
    assert_eq!(
        run(VirtualOp::Interacting(Some(2), Some(2)), &a, &b),
        vec![(0, 0, 300, 100)]
    );
    assert_eq!(
        run(VirtualOp::Interacting(Some(1), Some(1)), &a, &b),
        vec![(400, 0, 500, 100)]
    );
    // An open maximum is "at least", so both meeting candidates survive.
    assert_eq!(
        run(VirtualOp::Interacting(Some(1), None), &a, &b),
        vec![(0, 0, 300, 100), (400, 0, 500, 100)]
    );
}

/// The uncounted form must stay exactly "at least one" — the counted path is an addition,
/// not a change of default.
#[test]
fn an_absent_count_is_at_least_one() {
    let (a, b) = counted_case();
    assert_eq!(
        run(VirtualOp::Interacting(None, None), &a, &b),
        run(VirtualOp::Interacting(Some(1), None), &a, &b)
    );
}

/// `not_interacting` with a count is the complement of the counted predicate, not of the
/// uncounted one: it keeps everything that does *not* meet exactly two, which includes the
/// candidate meeting one.
#[test]
fn not_interacting_complements_the_count() {
    let (a, b) = counted_case();
    assert_eq!(
        run(VirtualOp::NotInteracting(Some(2), Some(2)), &a, &b),
        vec![(400, 0, 500, 100), (800, 0, 900, 100)]
    );
}

/// The reason the filter is stitched as well.  One filter shape crossing a tile edge is
/// one neighbour; counting per tile piece would see two and let a `(2, 2)` selector keep a
/// candidate that touches a single shape.
#[test]
fn a_filter_region_spanning_tiles_counts_once() {
    let a = vec![rect(A, 0, 0, 300, 100)];
    let b = vec![rect(B, 50, 0, 250, 100)]; // straddles the tile edge at x = 200
    assert!(
        run_tiled(VirtualOp::Interacting(Some(2), Some(2)), &a, &b, 200).is_empty(),
        "one filter shape must not count as two"
    );
    assert_eq!(
        run_tiled(VirtualOp::Interacting(Some(1), Some(1)), &a, &b, 200),
        vec![(0, 0, 300, 100)]
    );
}

/// `covering` counts whole filter regions the candidate contains, so the same bounds
/// apply to it — and a filter shape running past the candidate's edge still counts for
/// nothing, exactly as in the uncounted form.
#[test]
fn covering_counts_only_wholly_contained_regions() {
    let a = vec![rect(A, 0, 0, 300, 300)];
    let b = vec![
        rect(B, 50, 50, 100, 100),
        rect(B, 150, 150, 200, 200),
        rect(B, 250, 250, 400, 350), // runs outside
    ];
    assert_eq!(
        run(VirtualOp::Covering(Some(2), Some(2)), &a, &b),
        vec![(0, 0, 300, 300)]
    );
    assert!(run(VirtualOp::Covering(Some(3), None), &a, &b).is_empty());
}

// ---------------------------------------------------------------------------
// extents
// ---------------------------------------------------------------------------

/// `extents` replaces each region with its bounding box, so an L becomes the rectangle it
/// occupies — and two separate shapes stay two boxes, not one around both.
#[test]
fn extents_boxes_each_region_separately() {
    let l = vec![rect(A, 0, 0, 100, 20), rect(A, 0, 0, 20, 100)];
    let far = vec![rect(A, 300, 300, 340, 320)];
    let mut a = l.clone();
    a.extend(far);
    assert_eq!(
        run(VirtualOp::Extents, &a, &[]),
        vec![(0, 0, 100, 100), (300, 300, 340, 320)]
    );
}

/// The box is the *stitched* region's, not a tile piece's: a shape wider than a tile has
/// one bounding box, and taking it per piece would return several smaller ones.
#[test]
fn extents_measures_the_whole_region_across_tiles() {
    let a = vec![rect(A, 0, 0, 500, 40)];
    assert_eq!(
        run_tiled(VirtualOp::Extents, &a, &[], 100),
        vec![(0, 0, 500, 40)]
    );
}

/// `separation_below` keeps the gap itself, not the shapes either side of it: two walls
/// 30 DBU apart with the limit at 50 give one region spanning exactly that gap, over the
/// stretch the two walls face each other along.  Past the limit there is nothing to keep.
///
/// This is a measurement used as geometry — the thing a maximum-distance rule needs, which
/// reports the walls such a region leaves untouched rather than the region itself.
#[test]
fn separation_below_spans_the_gap_between_facing_walls() {
    let a = [rect(A, 0, 0, 100, 100)];
    let b = [rect(B, 130, 20, 200, 80)];

    let got = run(VirtualOp::SeparationBelow(50), &a, &b);
    assert_eq!(got, vec![(100, 20, 130, 80)]);

    // The same pair with the limit under the gap: nothing faces closely enough.
    assert!(run(VirtualOp::SeparationBelow(20), &a, &b).is_empty());
}

/// It needs both sides: a gap has two walls, so a tile holding only one of them has none.
#[test]
fn separation_below_needs_both_sides() {
    let a = [rect(A, 0, 0, 100, 100)];
    assert!(run(VirtualOp::SeparationBelow(50), &a, &[]).is_empty());
}

/// `enclosure_below` keeps the short margin itself: an inner square sitting 10 DBU inside
/// an outer one, with the limit at 30, gives the four bands of margin that fall short.
/// Raise the enclosure past the limit and there is nothing short to keep.
#[test]
fn enclosure_below_spans_the_short_margin() {
    let outer = [rect(A, 0, 0, 200, 200)];
    let inner = [rect(B, 10, 10, 190, 190)];

    let got = run(VirtualOp::EnclosureBelow(30), &outer, &inner);
    assert_eq!(
        got,
        vec![
            (0, 10, 10, 190),
            (10, 0, 190, 10),
            (10, 190, 190, 200),
            (190, 10, 200, 190),
        ]
    );

    // The same pair measured against a limit the margin already clears.
    assert!(run(VirtualOp::EnclosureBelow(5), &outer, &inner).is_empty());
}

/// `enclosure_above` is the other side of the same measurement: with the inner square 10
/// DBU inside the outer one and the bound at 8, every margin exceeds it and all four bands
/// come back.  Raise the bound past the margin and none does.
///
/// The bound also sets how far the search looks — `ENCLOSURE_MAX_REACH` times it — so a
/// margin further off than that is deliberately not measured: the wall that far away is
/// the far side of the enclosing shape, not the one this margin runs to.  Here 8 reaches
/// 16, comfortably past the margin of 10; at a bound of 5 it would reach only 10 and the
/// margin would sit on the edge of being seen at all.
#[test]
fn enclosure_above_spans_the_margin_that_exceeds_the_bound() {
    let outer = [rect(A, 0, 0, 200, 200)];
    let inner = [rect(B, 10, 10, 190, 190)];

    let got = run(VirtualOp::EnclosureAbove(8), &outer, &inner);
    assert_eq!(
        got,
        vec![
            (0, 10, 10, 190),
            (10, 0, 190, 10),
            (10, 190, 190, 200),
            (190, 10, 200, 190),
        ]
    );

    assert!(run(VirtualOp::EnclosureAbove(30), &outer, &inner).is_empty());
}
