// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The density family driven the way a deck drives it: a rule, a layout, a cache, and
//! the violations that come out.  Every expected percentage is read off the drawing.

use super::{Kind, run};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::{Layer, Param, RuleDefinition};
use crate::violation::{Violation, ViolationGeometry};
use gds21::{GdsBoundary, GdsPoint};
use std::collections::HashMap;

const A: (i16, i16) = (1, 0);
const B: (i16, i16) = (2, 0);
const FRAME: (i16, i16) = (9, 0);
const DBU: f64 = 0.001;

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

fn rule(layers: &[(i16, i16)], value: f64, params: &[(&str, f64)]) -> RuleDefinition {
    RuleDefinition {
        id: "T".into(),
        check: "density".into(),
        layers: layers.iter().map(|&l| layer(l)).collect(),
        value,
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), Param::Num(*v)))
            .collect(),
        ignore: vec![],
        text: None,
    }
}

/// The rule read per window: `scope: window`.
fn windowed(mut r: RuleDefinition) -> RuleDefinition {
    r.params
        .insert("scope".into(), Param::Word("window".into()));
    r
}

fn boundary(l: (i16, i16)) -> [(&'static str, f64); 2] {
    [("boundary", l.0 as f64), ("boundary_dt", l.1 as f64)]
}

/// A cache tiled at 20 µm with a 1 µm halo, as a run has it.
fn cache() -> MergedCache {
    MergedCache::new(20_000, 1_000, HashMap::new())
}

fn density_of(v: &Violation) -> f64 {
    let s = v.message.split("density ").nth(1).expect("a density");
    s.split('%').next().unwrap().parse().expect("a number")
}

/// A 100 µm frame with A covering 40 µm of its height: 40 % over the frame's box.  A
/// second layer B over another 10 µm adds to it; a B stripe on top of the A stripe adds
/// nothing, since coverage is merged per layer and the layers are summed.
#[test]
fn chip_density_is_the_layers_coverage_over_the_box() {
    let lay = layout(vec![
        brect(FRAME, 0, 0, 100_000, 100_000),
        brect(A, 0, 0, 100_000, 40_000),
        brect(B, 0, 50_000, 100_000, 60_000),
    ]);
    let mut m = cache();
    let go = |layers: &[(i16, i16)], kind: Kind, value: f64, m: &mut MergedCache| {
        run(kind, &rule(layers, value, &boundary(FRAME)), &lay, DBU, m)
    };
    let v = go(&[A], Kind::Min, 50.0, &mut m);
    assert_eq!(v.len(), 1);
    assert_eq!(density_of(&v[0]), 40.0);
    assert!(go(&[A], Kind::Min, 40.0, &mut m).is_empty());
    assert!(go(&[A], Kind::Max, 40.0, &mut m).is_empty());
    assert_eq!(go(&[A], Kind::Max, 39.9, &mut m).len(), 1);
    let v = go(&[A, B], Kind::Min, 60.0, &mut m);
    assert_eq!(density_of(&v[0]), 50.0);
}

/// Without a boundary the box of every shape is the die; with one, the boundary's box
/// is, even where the frame is a hollow ring whose own material is a sliver.
#[test]
fn the_denominator_is_a_box_not_material() {
    let ring = vec![
        brect(FRAME, 0, 0, 100_000, 1_000),
        brect(FRAME, 0, 99_000, 100_000, 100_000),
        brect(FRAME, 0, 0, 1_000, 100_000),
        brect(FRAME, 99_000, 0, 100_000, 100_000),
        brect(A, 0, 0, 100_000, 25_000),
    ];
    let lay = layout(ring);
    let mut m = cache();
    let v = run(
        Kind::Min,
        &rule(&[A], 100.0, &boundary(FRAME)),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(density_of(&v[0]), 25.0);
    let v = run(Kind::Min, &rule(&[A], 100.0, &[]), &lay, DBU, &mut m);
    assert_eq!(
        density_of(&v[0]),
        25.0,
        "the shapes' own box is the same die here"
    );
    // A narrower design without the ring: A alone is the die, so it covers all of it.
    let lay = layout(vec![brect(A, 0, 0, 100_000, 25_000)]);
    let v = run(Kind::Min, &rule(&[A], 100.0, &[]), &lay, DBU, &mut m);
    assert!(v.is_empty());
}

/// A 100 µm die in 50 µm windows: four windows, A filling the lower-left one and half
/// of the one above it.  A 60 % floor fails the three others; a 60 % cap fails the full
/// one.  The stripe across the tile line at 20 µm is counted once.
#[test]
fn windowed_density_reads_every_window() {
    let lay = layout(vec![
        brect(FRAME, 0, 0, 100_000, 100_000),
        brect(A, 0, 0, 50_000, 75_000),
    ]);
    let mut m = cache();
    let go = |kind: Kind, value: f64, m: &mut MergedCache| {
        let mut p = vec![("window", 50.0)];
        p.extend(boundary(FRAME));
        run(kind, &windowed(rule(&[A], value, &p)), &lay, DBU, m)
    };
    let v = go(Kind::Min, 60.0, &mut m);
    assert_eq!(v.len(), 3, "{v:?}");
    let mut ds: Vec<f64> = v.iter().map(density_of).collect();
    ds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ds, vec![0.0, 0.0, 50.0]);
    let v = go(Kind::Max, 60.0, &mut m);
    assert_eq!(v.len(), 1);
    assert_eq!(density_of(&v[0]), 100.0);
    let ViolationGeometry::Edge { x1, y1, x2, y2 } = v[0].geometry else {
        panic!("{v:?}");
    };
    assert_eq!((x1, y1, x2, y2), (0.0, 0.0, 50.0, 50.0));
}

/// The grid starts at the chip's box, and a window is measured against its overlap
/// with the boundary's box: a die 130 µm wide in 50 µm windows has a 30 µm third
/// column, whose coverage is judged against 30 × 50 and not 50 × 50.  A window past the
/// boundary altogether is skipped.
#[test]
fn a_partial_window_is_measured_against_what_is_there() {
    let lay = layout(vec![
        brect(FRAME, 0, 0, 130_000, 50_000),
        brect(A, 0, 0, 130_000, 20_000),
        brect(B, 150_000, 0, 160_000, 50_000),
    ]);
    let mut m = cache();
    let mut p = vec![("window", 50.0)];
    p.extend(boundary(FRAME));
    let v = run(
        Kind::Min,
        &windowed(rule(&[A], 50.0, &p)),
        &lay,
        DBU,
        &mut m,
    );
    // Three windows over the boundary, each 40 % - and none over the B island at 150.
    assert_eq!(v.len(), 3, "{v:?}");
    assert!(v.iter().all(|v| density_of(v) == 40.0), "{v:?}");
    let v = run(
        Kind::Min,
        &windowed(rule(&[A], 50.0, &[("window", 50.0)])),
        &lay,
        DBU,
        &mut m,
    );
    // Without a boundary the third window is nominally 50 wide: 30 × 20 over 50 × 50 is
    // 24 %; and the fourth, over the island, is empty of A.
    let mut ds: Vec<f64> = v.iter().map(density_of).collect();
    ds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ds, vec![0.0, 24.0, 40.0, 40.0]);
}

/// A rule without a window, or with a boundary nobody drew, says so or falls back.
#[test]
fn a_malformed_or_unbounded_rule() {
    let lay = layout(vec![brect(A, 0, 0, 10_000, 5_000)]);
    let mut m = cache();
    assert!(
        run(
            Kind::Min,
            &windowed(rule(&[A], 50.0, &[])),
            &lay,
            DBU,
            &mut m
        )
        .is_empty()
    );
    // A boundary layer with no shapes: the die is the shapes' box, which is A itself.
    let v = run(
        Kind::Max,
        &rule(&[A], 99.9, &boundary(FRAME)),
        &lay,
        DBU,
        &mut m,
    );
    assert_eq!(v.len(), 1);
    assert_eq!(density_of(&v[0]), 100.0);
}
