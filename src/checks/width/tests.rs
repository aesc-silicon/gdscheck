// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The width family driven the way a deck drives it: a rule, a layout, a cache, and the
//! violations that come out.  The measurement itself is tested next to it in `geom`;
//! these ask what the drivers add - tile ownership, the pinch points, the parameters,
//! and the ways a rule can be malformed.

use super::scan::{pinch_points, scan_widths};
use super::{Kind, run, run_gate};
use crate::geom::Limit;
use crate::layout::FlatLayout;
use crate::merge::{Core, EdgeOp, MergedCache, MergedPoly};
use crate::pdk::{Layer, Param, RuleDefinition};
use crate::violation::{Violation, ViolationGeometry};
use gds21::{GdsBoundary, GdsPoint};
use i_overlay::i_float::int::point::IntPoint;
use std::collections::HashMap;

const A: (i16, i16) = (1, 0);
const B: (i16, i16) = (2, 0);
const C: (i16, i16) = (3, 0);
const DBU: f64 = 0.001;

fn pt(x: i32, y: i32) -> IntPoint {
    IntPoint::new(x, y)
}

fn core() -> Core {
    Core {
        x0: -1_000_000,
        y0: -1_000_000,
        x1: 1_000_000,
        y1: 1_000_000,
    }
}

fn poly(pts: &[(i32, i32)]) -> MergedPoly {
    MergedPoly {
        outer: pts.iter().map(|&(x, y)| pt(x, y)).collect(),
        holes: vec![],
    }
}

fn mrect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
    poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

/// An axis-aligned boundary on `l`, in DBU.
fn brect(l: (i16, i16), x0: i32, y0: i32, x1: i32, y1: i32) -> ((i16, i16), GdsBoundary) {
    (
        l,
        GdsBoundary {
            layer: l.0,
            datatype: l.1,
            xy: GdsPoint::vec(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]),
            ..Default::default()
        },
    )
}

fn layout(shapes: Vec<((i16, i16), GdsBoundary)>) -> FlatLayout {
    let mut out = FlatLayout::new();
    for (l, b) in shapes {
        out.insert(l.0, l.1, b);
    }
    out
}

fn layer(l: (i16, i16)) -> Layer {
    Layer {
        name: format!("L{}", l.0),
        gds_layer: l.0 as u16,
        gds_datatype: l.1 as u16,
    }
}

fn rule(
    check: &str,
    layers: &[(i16, i16)],
    value_um: f64,
    params: &[(&str, Param)],
) -> RuleDefinition {
    RuleDefinition {
        id: "T".into(),
        check: check.into(),
        layers: layers.iter().map(|&l| layer(l)).collect(),
        value: value_um,
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        ignore: vec![],
        text: None,
    }
}

fn word(w: &str) -> Param {
    Param::Word(w.into())
}

fn num(v: f64) -> Param {
    Param::Num(v)
}

/// A cache with one tile the size of the world, and no halo: nothing about tiling.
fn cache() -> MergedCache {
    MergedCache::new(10_000_000, 0, HashMap::new())
}

/// A cache tiled at `tile` DBU with a halo of `halo` DBU on every layer.
fn tiled(tile: i32, halo: i32) -> MergedCache {
    MergedCache::new(tile, halo, HashMap::new())
}

/// The measured width a violation's message names, in µm.
fn width_of(v: &Violation) -> f64 {
    let s = v.message.split("width ").nth(1).expect("a width");
    s.split(' ').next().unwrap().parse().expect("a number")
}

fn points(v: &[Violation]) -> usize {
    v.iter()
        .filter(|v| matches!(v.geometry, ViolationGeometry::Point { .. }))
        .count()
}

// ---------------------------------------------------------------------------------------
// pinch_points
// ---------------------------------------------------------------------------------------

/// Two squares corner to corner, merged into two polygons that touch: one pinch, at the
/// shared vertex.
#[test]
fn two_pieces_meeting_at_a_vertex_pinch_there() {
    let polys = [mrect(0, 0, 100, 100), mrect(100, 100, 200, 200)];
    assert_eq!(pinch_points(&polys), vec![(100.0, 100.0)]);
}

/// The same two squares as one contour through the shared corner twice - the other way
/// a merge can hand a bow-tie back - is the same pinch.
#[test]
fn one_contour_through_a_vertex_twice_pinches_there() {
    let bowtie = poly(&[
        (0, 0),
        (100, 0),
        (100, 100),
        (200, 100),
        (200, 200),
        (100, 200),
        (100, 100),
        (0, 100),
    ]);
    assert_eq!(pinch_points(&[bowtie]), vec![(100.0, 100.0)]);
}

/// A notch whose tip rests on a straight wall: the wall keeps no vertex there, so the
/// touch has a vertex on one side only.
#[test]
fn a_notch_tip_on_a_straight_wall_pinches_there() {
    let shape = poly(&[
        (0, 0),
        (300, 0),
        (300, 100),
        (200, 100),
        (150, 50),
        (100, 100),
        (0, 100),
    ]);
    // The wall from (300, 0) to (0, 0) runs under the tip (150, 50)?  No - the tip sits
    // 50 above it.  Put it on the wall:
    let touching = poly(&[
        (0, 0),
        (300, 0),
        (300, 100),
        (200, 100),
        (150, 0),
        (100, 100),
        (0, 100),
    ]);
    assert!(pinch_points(&[shape]).is_empty());
    assert_eq!(pinch_points(&[touching]), vec![(150.0, 0.0)]);
}

/// Two pieces sharing a run of boundary are one wide shape, not a pinch - the way a
/// layer delivered as core-clipped pieces meets itself along every tile line.
#[test]
fn pieces_sharing_a_run_do_not_pinch() {
    let polys = [mrect(0, 0, 100, 100), mrect(100, 0, 200, 100)];
    assert!(pinch_points(&polys).is_empty());
    // Sharing part of a run, with a vertex of one on the other's edge: still abutting.
    let polys = [mrect(0, 0, 100, 100), mrect(100, 50, 200, 150)];
    assert!(pinch_points(&polys).is_empty());
}

// ---------------------------------------------------------------------------------------
// run (min_width, max_width, exact_width)
// ---------------------------------------------------------------------------------------

/// A bar 400 DBU wide across a tile line, seen whole from both tiles through the halo:
/// its two walls are reported once, by the tile owning the pair's midpoint.  (A midpoint
/// exactly *on* the line is claimed by both tiles and folded later by `run_drc`, so the
/// bar here has its midpoint off the line.)
#[test]
fn a_pair_across_a_tile_line_is_reported_once() {
    let lay = layout(vec![brect(A, 15_000, 10_000, 27_000, 10_400)]);
    let mut m = tiled(20_000, 1_000);
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    // The bar's length, 10 000, is no width under a 0.5 µm rule either way.
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.4, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
}

/// A maximum reads the same pair with the bound the other way, and the exact width
/// reports either dimension that is off.
#[test]
fn the_three_bounds_read_one_box() {
    let lay = layout(vec![brect(A, 0, 0, 300, 320)]);
    let mut m = cache();
    let go = |k: Kind, check: &str, v: f64, m: &mut MergedCache| {
        run(k, &rule(check, &[A], v, &[]), &lay, DBU, m).len()
    };
    assert_eq!(go(Kind::Min, "min_width", 0.3, &mut m), 0);
    assert_eq!(go(Kind::Min, "min_width", 0.31, &mut m), 2);
    assert_eq!(go(Kind::Max, "max_width", 0.32, &mut m), 0);
    assert_eq!(go(Kind::Max, "max_width", 0.31, &mut m), 2);
    assert_eq!(go(Kind::Exact, "exact_width", 0.3, &mut m), 2);
    assert_eq!(go(Kind::Exact, "exact_width", 0.31, &mut m), 4);
}

/// A pinch is a width of zero: a minimum reports it as a point, a maximum or an exact
/// width has no span there to bound.
#[test]
fn a_pinch_is_a_minimum_violation_only() {
    let lay = layout(vec![
        brect(A, 0, 0, 1_000, 1_000),
        brect(A, 1_000, 1_000, 2_000, 2_000),
    ]);
    let mut m = cache();
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!((v.len(), points(&v)), (1, 1), "{v:?}");
    let v = run(
        Kind::Max,
        &rule("max_width", &[A], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(points(&v), 0);
    let v = run(
        Kind::Exact,
        &rule("exact_width", &[A], 1.0, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(points(&v), 0);
}

/// `angle: bent` reads the 45° runs alone; `length` binds any rule to runs longer than
/// so much.  A 45° bar 600 across beside a straight bar 400 wide: the bent rule at 0.7
/// sees the diagonal and not the straight bar, a plain rule at 0.5 the reverse, and a
/// length of 2 µm puts the 1.4 µm diagonal out of the bent rule's reach.
#[test]
fn angle_bent_and_length_select_the_runs() {
    let diag = GdsBoundary {
        layer: A.0,
        datatype: A.1,
        xy: GdsPoint::vec(&[
            (5_000, 5_000),
            (6_000, 6_000),
            (5_576, 6_424),
            (4_576, 5_424),
            (5_000, 5_000),
        ]),
        ..Default::default()
    };
    let lay = layout(vec![(A, diag), brect(A, 0, 0, 3_000, 400)]);
    let mut m = cache();
    let bent = [("angle", word("bent"))];
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.7, &bent),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(
        v.iter().all(|v| (width_of(v) - 0.5996).abs() < 0.001),
        "{v:?}"
    );
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(v.iter().all(|v| width_of(v) == 0.4), "{v:?}");
    let long = [("angle", word("bent")), ("length", num(2.0))];
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.7, &long),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty(), "{v:?}");
    // The straight bar's walls share 3 000: a plain rule with length 2 µm keeps it,
    // with 3 µm drops it.
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[("length", num(2.0))]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2);
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[("length", num(3.0))]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
}

/// A rule that names the wrong kind of value, or an edge layer, runs nothing rather
/// than something else.
#[test]
fn a_malformed_width_rule_reports_nothing() {
    let lay = layout(vec![brect(A, 0, 0, 300, 3_000)]);
    let mut m = cache();
    let ok = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(ok.len(), 2);
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[("angle", word("steep"))]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
    let v = run(
        Kind::Min,
        &rule("min_width", &[A], 0.5, &[("angle", num(45.0))]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
    m.register_edge(C, EdgeOp::Edges, vec![A]);
    let v = run(
        Kind::Min,
        &rule("min_width", &[C], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
}

// ---------------------------------------------------------------------------------------
// run_gate
// ---------------------------------------------------------------------------------------

/// A stripe of A 800 tall and a 2 000-wide piece of B cut out of it, the way a channel
/// mask is the poly over the active: the stripe's long walls are shared with B's
/// boundary across the piece, and the gate length is read there and cut to it.  The
/// piece is nowhere near the stripe's own midpoint.
#[test]
fn a_gate_is_read_where_the_walls_are_shared() {
    let lay = layout(vec![
        brect(A, 0, 0, 30_000, 800),
        brect(B, 26_000, 0, 28_000, 800),
    ]);
    let mut m = tiled(20_000, 1_000);
    let v = run_gate(
        Kind::Min,
        &rule("min_gate_length", &[A, B], 1.0, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    for v in &v {
        let ViolationGeometry::Edge { x1, x2, .. } = v.geometry else {
            panic!("{v:?}");
        };
        assert_eq!((x1.min(x2), x1.max(x2)), (26.0, 28.0), "{v:?}");
    }
    let v = run_gate(
        Kind::Min,
        &rule("min_gate_length", &[A, B], 0.8, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty());
}

/// A body whose two ends the reference cut: the shared walls give one dimension, the
/// unshared ones the other, and each bound reads only the walls it was told to.
#[test]
fn shared_and_unshared_walls_are_two_dimensions() {
    let lay = layout(vec![
        brect(A, 10_000, 10_000, 14_000, 11_000),
        brect(B, 5_000, 9_000, 10_000, 12_000),
        brect(B, 14_000, 9_000, 19_000, 12_000),
    ]);
    let mut m = cache();
    let unshared = [("walls", word("unshared"))];
    let go = |k: Kind, v: f64, p: &[(&str, Param)], m: &mut MergedCache| {
        run_gate(k, &rule("gate", &[A, B], v, p), &lay, DBU, m).len()
    };
    // Between the ends: 4 000.  Between top and bottom: 1 000.
    assert_eq!(go(Kind::Max, 3.0, &[], &mut m), 2);
    assert_eq!(go(Kind::Max, 4.0, &[], &mut m), 0);
    assert_eq!(go(Kind::Max, 3.0, &unshared, &mut m), 0);
    assert_eq!(go(Kind::Min, 1.1, &unshared, &mut m), 2);
    assert_eq!(go(Kind::Min, 1.0, &unshared, &mut m), 0);
    assert_eq!(go(Kind::Exact, 4.0, &[], &mut m), 0);
    assert_eq!(go(Kind::Exact, 1.0, &[], &mut m), 2);
}

/// `outside` cuts the kept stretches to a region's exterior, and `length` asks a run of
/// them.
#[test]
fn outside_and_length_cut_the_gate() {
    let lay = layout(vec![
        brect(A, 0, 0, 10_000, 800),
        brect(B, 4_000, 0, 6_000, 800),
        brect(C, 5_000, -1_000, 20_000, 2_000),
    ]);
    let mut m = cache();
    let outside = [
        ("outside", num(C.0 as f64)),
        ("outside_dt", num(C.1 as f64)),
    ];
    let v = run_gate(
        Kind::Min,
        &rule("gate", &[A, B], 1.0, &outside),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    for v in &v {
        let ViolationGeometry::Edge { x1, x2, .. } = v.geometry else {
            panic!("{v:?}");
        };
        assert_eq!((x1.min(x2), x1.max(x2)), (4.0, 5.0), "{v:?}");
    }
    let v = run_gate(
        Kind::Min,
        &rule("gate", &[A, B], 1.0, &[("length", num(2.0))]),
        &lay,
        DBU,
        &mut m,
    );
    assert!(v.is_empty(), "the crossing's run is 2 000, not more");
    let v = run_gate(
        Kind::Min,
        &rule("gate", &[A, B], 1.0, &[("length", num(1.9))]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 2);
}

/// A pinch of the body counts where the filter keeps the point, and for a minimum only.
#[test]
fn a_gate_pinch_counts_where_the_filter_keeps_it() {
    let lay = layout(vec![
        brect(A, 0, 0, 1_000, 1_000),
        brect(A, 1_000, 1_000, 2_000, 2_000),
        brect(B, 1_000, -500, 3_000, 2_500),
    ]);
    let mut m = cache();
    // The pinch at (1 000, 1 000) lies on B's left wall: shared keeps it, unshared not.
    let v = run_gate(
        Kind::Min,
        &rule("gate", &[A, B], 0.5, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(points(&v), 1, "{v:?}");
    let v = run_gate(
        Kind::Min,
        &rule("gate", &[A, B], 0.5, &[("walls", word("unshared"))]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(points(&v), 0, "{v:?}");
    let v = run_gate(
        Kind::Exact,
        &rule("gate", &[A, B], 1.0, &[]),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(points(&v), 0, "{v:?}");
}

/// A gate rule missing its reference, or naming a mode that does not exist, runs
/// nothing.
#[test]
fn a_malformed_gate_rule_reports_nothing() {
    let lay = layout(vec![
        brect(A, 0, 0, 10_000, 800),
        brect(B, 4_000, 0, 6_000, 800),
    ]);
    let mut m = cache();
    assert_eq!(
        run_gate(
            Kind::Min,
            &rule("gate", &[A, B], 1.0, &[]),
            &lay,
            DBU,
            &mut m
        )
        .len(),
        2
    );
    assert!(run_gate(Kind::Min, &rule("gate", &[A], 1.0, &[]), &lay, DBU, &mut m).is_empty());
    let bad = [("walls", word("inner"))];
    assert!(
        run_gate(
            Kind::Min,
            &rule("gate", &[A, B], 1.0, &bad),
            &lay,
            DBU,
            &mut m
        )
        .is_empty()
    );
    let bad = [("walls", num(1.0))];
    assert!(
        run_gate(
            Kind::Min,
            &rule("gate", &[A, B], 1.0, &bad),
            &lay,
            DBU,
            &mut m
        )
        .is_empty()
    );
}

// ---------------------------------------------------------------------------------------
// scan_widths: the pairs written out as violations
// ---------------------------------------------------------------------------------------

/// Thin 45° trace (141 DBU across) flagged by a `< 160` (min-width) predicate:
/// both walls reported, nothing from the end-caps, which are square to the trace.
#[test]
fn oblique_45_thin_trace_flags_both_walls() {
    let poly = MergedPoly {
        outer: vec![pt(0, 0), pt(1000, 1000), pt(900, 1100), pt(-100, 100)],
        holes: vec![],
    };
    let v = scan_widths(
        &poly,
        core(),
        0.001,
        "T",
        "min",
        "L",
        0.16,
        "<",
        Limit::AtLeast(160),
        false,
        true,
        0,
        None,
    );
    assert_eq!(v.len(), 2, "got {}", v.len());
}

#[test]
fn oblique_45_wide_trace_is_clean() {
    let poly = MergedPoly {
        outer: vec![pt(0, 0), pt(1000, 1000), pt(0, 2000), pt(-1000, 1000)],
        holes: vec![],
    };
    let v = scan_widths(
        &poly,
        core(),
        0.001,
        "T",
        "min",
        "L",
        0.16,
        "<",
        Limit::AtLeast(160),
        false,
        true,
        0,
        None,
    );
    assert!(v.is_empty(), "got {}", v.len());
}

/// A region with a 45° chamfer across one corner: the strip above the chamfer is only
/// 500 DBU wide, bounded by the chamfer on one side and the vertical edge on the
/// other.  None of the like-with-like passes pairs those two - vertical with
/// vertical, horizontal with horizontal, oblique with anti-parallel oblique - so
/// without the mixed pass this shape reads clean at any value.
#[test]
fn chamfered_corner_narrows_against_the_opposite_wall() {
    let poly = MergedPoly {
        outer: vec![
            pt(0, 0),
            pt(1000, 0),
            pt(1000, 2000),
            pt(500, 2000),
            pt(0, 1500),
        ],
        holes: vec![],
    };
    let scan = |limit_dbu: i64, mixed: bool| {
        scan_widths(
            &poly,
            core(),
            0.001,
            "T",
            "min",
            "L",
            limit_dbu as f64 / 1000.0,
            "<",
            Limit::AtLeast(limit_dbu),
            false,
            mixed,
            0,
            None,
        )
    };
    // 860 DBU rule: the 500-wide strip violates, and the marker spans the gap.
    let v = scan(860, true);
    assert_eq!(v.len(), 1, "got {}", v.len());
    assert!(
        v[0].message.contains("width 0.5000"),
        "measured the wrong span: {}",
        v[0].message
    );
    // Below the narrow strip the shape is a clean 1000 wide, so a 500 rule passes.
    assert!(scan(500, true).is_empty());
    // And without the mixed pass the violation is invisible — the regression itself.
    assert!(scan(860, false).is_empty());
}

/// A 200×200 DBU square flagged by a `> 150` (max-width) predicate: both
/// dimensions exceed, two walls each → 4.
#[test]
fn max_width_square_flags_four_walls() {
    let poly = MergedPoly {
        outer: vec![pt(0, 0), pt(200, 0), pt(200, 200), pt(0, 200)],
        holes: vec![],
    };
    let v = scan_widths(
        &poly,
        core(),
        0.001,
        "T",
        "max",
        "L",
        0.15,
        ">",
        Limit::AtMost(150),
        false,
        false,
        0,
        None,
    );
    assert_eq!(v.len(), 4, "got {}", v.len());
}
