// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the residual family ([`residual`](gdscheck::checks::residual)): the
//! layers themselves, a forbidden edge layer, the uncovered part of a target and the
//! whole polygon, an overlap, partners apart and touching, a plate without its via
//! array, shapes beyond a boundary, and a label that exempts.
//!
//! A residual has no numeric bound; its exact case is the coincident edge.  A cover
//! ending on the target's edge leaves nothing, one ending 0.005 µm short leaves a
//! sliver; two shapes sharing an edge overlap nowhere and touch everywhere along it.

use crate::helpers::{layer, library, poly, rect, text, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/residual";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    let diode = layer(pdk, "Diode");
    let label = layer(pdk, "Text");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // Three Diode squares, the third across the tile line at 20 µm: R.bare 3, the
    // straddler once.  An Outer square across the same line has four edges: R.edges 4.
    write(
        "bare",
        vec![
            rect(diode, 30.0, 30.0, 32.0, 32.0),
            rect(diode, 35.0, 30.0, 37.0, 32.0),
            rect(diode, 15.0, 30.0, 25.0, 32.0),
            rect(outer, 15.0, 40.0, 25.0, 50.0),
        ],
    );

    // An Inner square under an Outer cover ending on its edge: nothing uncovered, R.uncovered
    // 0 and R.polygon 0.  The cover ending 0.005 short leaves a sliver: 1 and 1.
    write(
        "covered_exact",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 30.0, 30.0, 32.0, 32.0),
        ],
    );
    write(
        "covered_short",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 30.0, 30.0, 31.995, 32.0),
        ],
    );

    // The covers are read as their union: Outer over the left half and Diode over the
    // right, meeting on a line, cover the whole bar - R.uncovered 0, R.polygon 0.  Diode
    // starting 0.005 late leaves a gap between them: 1 and 1.
    write(
        "union_exact",
        vec![
            rect(inner, 30.0, 30.0, 34.0, 32.0),
            rect(outer, 30.0, 30.0, 32.0, 32.0),
            rect(diode, 32.0, 30.0, 34.0, 32.0),
        ],
    );
    write(
        "union_gap",
        vec![
            rect(inner, 30.0, 30.0, 34.0, 32.0),
            rect(outer, 30.0, 30.0, 32.0, 32.0),
            rect(diode, 32.005, 30.0, 34.0, 32.0),
        ],
    );

    // A 45° diamond of Inner whose four tips lie on the sides of an Outer square:
    // covered, 0 and 0.  The square's left side moved in by 0.005 leaves the left tip
    // out: 1 and 1.
    let diamond = poly(
        inner,
        &[(32.0, 30.0), (34.0, 32.0), (32.0, 34.0), (30.0, 32.0)],
    );
    write(
        "diag_exact",
        vec![diamond.clone(), rect(outer, 30.0, 30.0, 34.0, 34.0)],
    );
    write(
        "diag_out",
        vec![diamond, rect(outer, 30.005, 30.0, 34.0, 34.0)],
    );

    // One Inner bar with two Outer patches on it: three bare parts, R.uncovered 3, and
    // one polygon not inside, R.polygon 1.
    write(
        "parts",
        vec![
            rect(inner, 30.0, 30.0, 36.0, 32.0),
            rect(outer, 31.0, 30.0, 32.0, 32.0),
            rect(outer, 34.0, 30.0, 35.0, 32.0),
        ],
    );

    // A bare part across the tile line at 20 µm, the covers at both ends: R.uncovered 1.
    write(
        "span",
        vec![
            rect(inner, 10.0, 30.0, 30.0, 32.0),
            rect(outer, 10.0, 30.0, 12.0, 32.0),
            rect(outer, 28.0, 30.0, 30.0, 32.0),
        ],
    );

    // Inner and Outer sharing an edge overlap nowhere: R.overlap 0.  Outer reaching
    // 0.005 into Inner: 1.
    write(
        "overlap_touch",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 32.0, 30.0, 34.0, 32.0),
        ],
    );
    write(
        "overlap_slim",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 31.995, 30.0, 34.0, 32.0),
        ],
    );

    // Three Inner squares and their partners: a Via inside the first, a Diode sharing an
    // edge with the second, a Via 0.005 off the third.  Touching counts the shared edge,
    // so the first two have a partner and the third has none: R.apart 1, R.touching 2.
    write(
        "partners",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            rect(via, 30.5, 30.5, 31.5, 31.5),
            rect(inner, 34.0, 30.0, 36.0, 32.0),
            rect(diode, 36.0, 30.0, 37.0, 32.0),
            rect(inner, 38.0, 30.0, 40.0, 32.0),
            rect(via, 40.005, 30.5, 41.0, 31.5),
        ],
    );

    // An Inner plate with four Vias on it, 0.5 µm squares on a 1 µm pitch: a 2x2 grid
    // is an array, R.array 0; the same four in a line or bent into an L are not, 1 and
    // 1.  A grid whose upper row sits exactly 1 µm above the lower is one location, 0;
    // 0.005 further it is two, 1.
    let plate = |vias: &[(f64, f64)]| {
        let mut v = vec![rect(inner, 30.0, 30.0, 36.0, 36.0)];
        v.extend(vias.iter().map(|&(x, y)| rect(via, x, y, x + 0.5, y + 0.5)));
        v
    };
    write(
        "array_grid",
        plate(&[(31.0, 31.0), (32.0, 31.0), (31.0, 32.0), (32.0, 32.0)]),
    );
    write(
        "array_line",
        plate(&[(31.0, 31.0), (32.0, 31.0), (33.0, 31.0), (34.0, 31.0)]),
    );
    write(
        "array_bent",
        plate(&[(31.0, 31.0), (32.0, 31.0), (33.0, 31.0), (31.0, 32.0)]),
    );
    write(
        "array_reach",
        plate(&[(31.0, 31.0), (32.0, 31.0), (31.0, 32.0), (32.0, 32.0)]),
    );
    write(
        "array_split",
        plate(&[(31.0, 31.0), (32.0, 31.0), (31.0, 32.005), (32.0, 32.005)]),
    );

    // An Outer frame drawn as four bars, 10 to 50 µm.  An Inner square well inside and
    // one whose corner lies on the frame's outer edge are both inside: R.beyond 0.  The
    // corner 0.005 past the edge is out: 1.  A Diode past the frame is on the ignore
    // list either way.
    let frame = |corner: f64| {
        vec![
            rect(outer, 10.0, 10.0, 50.0, 11.0),
            rect(outer, 10.0, 49.0, 50.0, 50.0),
            rect(outer, 10.0, 10.0, 11.0, 50.0),
            rect(outer, 49.0, 10.0, 50.0, 50.0),
            rect(inner, 20.0, 20.0, 22.0, 22.0),
            rect(inner, 48.0, 48.0, corner, corner),
            rect(diode, 60.0, 60.0, 62.0, 62.0),
        ]
    };
    write("beyond_edge", frame(50.0));
    write("beyond_out", frame(50.005));

    // Two Diode squares, one carrying the label `keep`: R.bare 2, R.label 1.
    write(
        "labelled",
        vec![
            rect(diode, 30.0, 30.0, 32.0, 32.0),
            rect(diode, 35.0, 30.0, 37.0, 32.0),
            text(label, "keep", 36.0, 31.0),
        ],
    );
}
