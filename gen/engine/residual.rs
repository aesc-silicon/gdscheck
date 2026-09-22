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

use crate::helpers::{
    chamfered_tr, flat_array, layer, library, poly, rect, ref_array, text, write_gz,
};
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

    // --- The hardening patterns: what a rule manual's residual asks of any layer,
    // drawn once here for every deck of every PDK (hardening/SPEC.md).  A residual has
    // no value; its bound is the coincident edge, and the classes are the shapes, the
    // unions, the tile lines, fifty at once and the extremes.  The deck's rules read
    // the same layers against each other - Inner under Outer or Diode is R.uncovered's,
    // Inner over Outer is R.overlap's, Inner beside Via or Diode is R.apart's and
    // R.touching's, Diode is R.bare's - so a cover is drawn on Diode, Outer overlaps
    // and nothing else, and every count below reads every shape in its layout.

    // 45° geometry.  An Inner square under a Diode cover whose chamfer runs through
    // the square's corner exactly leaves nothing; the chamfer 0.005 further in cuts a
    // sliver off the corner.  An Outer diamond whose tip lies on the wall of a covered
    // Inner square overlaps nothing; the tip 0.005 in overlaps.  R.uncovered: 1;
    // R.polygon: 1; R.overlap: 1; R.apart: 0; R.touching: 4.
    write(
        "bound_45",
        vec![
            rect(inner, 30.0, 30.0, 32.0, 32.0),
            chamfered_tr(diode, 29.0, 29.0, 33.0, 33.0, 64.0), // x + y = 64 through (32, 32)
            rect(inner, 40.0, 30.0, 42.0, 32.0),
            chamfered_tr(diode, 39.0, 29.0, 43.0, 33.0, 73.995), // R.uncovered, R.polygon
            rect(inner, 50.0, 30.0, 52.0, 32.0),
            rect(diode, 50.0, 30.0, 52.0, 32.0),
            crate::helpers::diamond(outer, 54.0, 31.0, 2.0), // tip on the wall at (52, 31)
            rect(inner, 60.0, 30.0, 62.0, 32.0),
            rect(diode, 60.0, 30.0, 62.0, 32.0),
            crate::helpers::diamond(outer, 63.995, 31.0, 2.0), // R.overlap, tip 0.005 in
        ],
    );

    // Shapes that merge.  An Inner bar drawn as six 1 µm slices under two Diode
    // covers that leave 32.5..33.5 bare is one bare part on one polygon; a cover
    // drawn as two overlapping boxes ending on the target's edge leaves nothing; two
    // Diode squares overlapping are one bare region; an Outer box drawn as two
    // abutting halves over a covered Inner square overlaps it once.  R.uncovered: 1;
    // R.polygon: 1; R.overlap: 1; R.bare: 5 (the two covers of the bar, the union
    // covering the second target, the overlapping pair, the cover of the last).
    let mut e = vec![];
    for i in 0..6 {
        let x = 30.0 + i as f64;
        e.push(rect(inner, x, 30.0, x + 1.0, 32.0));
    }
    e.extend([
        rect(diode, 30.0, 30.0, 32.5, 32.0),
        rect(diode, 33.5, 30.0, 36.0, 32.0), // one bare part 32.5..33.5
        rect(inner, 40.0, 30.0, 44.0, 32.0),
        rect(diode, 40.0, 30.0, 42.5, 32.0),
        rect(diode, 41.5, 30.0, 44.0, 32.0), // covered
        rect(diode, 50.0, 30.0, 52.0, 32.0),
        rect(diode, 51.0, 30.0, 53.0, 32.0), // one bare region
        rect(inner, 60.0, 30.0, 62.0, 32.0),
        rect(diode, 60.0, 30.0, 62.0, 32.0),
        rect(outer, 61.0, 29.0, 62.0, 33.0),
        rect(outer, 62.0, 29.0, 63.0, 33.0), // one overlap, 61..62
    ]);
    write("merge", e);

    // Tile lines.  Three Inner bars from 5 to 50 under Diode covers with bare parts
    // ending on x = 20, inside a tile, across 40 and across 42 on the first, starting
    // on 20 and inside a tile on the second, across 20 and across 21 on the third; an
    // Inner bar covered by Diode that an Outer bar overlaps by 0.505 across 20; an
    // Inner square across 20 under Outer with its Via partner across it too, and one
    // under Outer whose Diode partner shares its wall on the line; a Diode square
    // across (20, 20) and one at (1000, 1000).  R.uncovered: 8; R.polygon: 3;
    // R.overlap: 3; R.apart: 0; R.touching: 6; R.bare: 15 (twelve covers, a partner,
    // two squares).
    let mut e = vec![];
    let bars: [(f64, &[(f64, f64)]); 3] = [
        (
            2.0,
            &[(15.0, 20.0), (24.75, 25.25), (39.5, 40.5), (41.75, 42.25)],
        ),
        (6.0, &[(20.0, 22.0), (30.0, 32.0)]),
        (10.0, &[(19.5, 20.5), (20.75, 21.25)]),
    ];
    for (y, spans) in bars {
        e.push(rect(inner, 5.0, y, 50.0, y + 2.0));
        let mut x = 5.0;
        for &(b0, b1) in spans {
            e.push(rect(diode, x, y, b0, y + 2.0));
            x = b1;
        }
        e.push(rect(diode, x, y, 50.0, y + 2.0));
    }
    e.extend([
        rect(inner, 10.0, 14.0, 20.5, 16.0),
        rect(diode, 10.0, 14.0, 20.5, 16.0),
        rect(outer, 19.995, 14.0, 30.0, 16.0), // R.overlap across 20
        rect(inner, 19.0, 24.0, 21.0, 26.0),
        rect(outer, 19.0, 24.0, 21.0, 26.0), // R.overlap
        rect(via, 19.5, 24.5, 20.5, 25.5),   // partner across 20: not apart, touching
        rect(inner, 18.0, 28.0, 20.0, 30.0),
        rect(outer, 18.0, 28.0, 20.0, 30.0),         // R.overlap
        rect(diode, 20.0, 28.0, 22.0, 30.0),         // partner sharing the wall on 20: touching
        rect(diode, 19.0, 19.0, 21.0, 21.0),         // R.bare across (20, 20)
        rect(diode, 1000.0, 1000.0, 1002.0, 1002.0), // R.bare
    ]);
    write("tile_lines", e);

    // Fifty of each: a Diode square, an Inner square under a Diode cover 0.005 short,
    // a covered Inner square that an Outer box overlaps by 0.005; flat and as an
    // array reference.  R.bare: 150; R.uncovered: 50; R.polygon: 50; R.overlap: 50.
    let cell = vec![
        rect(diode, 0.2, 0.2, 1.2, 1.2),
        rect(inner, 2.0, 0.2, 3.0, 1.2),
        rect(diode, 2.0, 0.2, 2.995, 1.2),
        rect(inner, 4.0, 0.2, 5.0, 1.2),
        rect(diode, 4.0, 0.2, 5.0, 1.2),
        rect(outer, 4.995, 0.2, 6.0, 1.2),
    ];
    write("array_flat", flat_array(&cell, 5, 10, 8.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 5, 10, 8.0),
    );

    // Small and long.  A 0.005 sliver of Diode; a 300 µm Inner bar under Diode but for
    // 0.005 at its far end; a 300 µm Outer bar reaching 0.005 into a covered Inner
    // square.  R.bare: 3; R.uncovered: 1; R.polygon: 1; R.overlap: 1.
    write(
        "extremes",
        vec![
            rect(diode, 2.0, 2.0, 2.005, 4.0),
            rect(inner, 2.0, 6.0, 302.0, 8.0),
            rect(diode, 2.0, 6.0, 301.995, 8.0),
            rect(inner, 2.0, 10.0, 4.0, 12.0),
            rect(diode, 2.0, 10.0, 4.0, 12.0),
            rect(outer, 3.995, 10.0, 303.995, 12.0),
        ],
    );

    // Via arrays across the tile line.  An Inner plate across x = 20 with a 2×2 grid
    // across it is not apart; with the grid's rows 1.005 apart it is.  R.array: 1.
    write(
        "array_tile_lines",
        vec![
            rect(inner, 15.0, 30.0, 25.0, 36.0),
            rect(via, 19.5, 31.0, 20.0, 31.5),
            rect(via, 20.5, 31.0, 21.0, 31.5),
            rect(via, 19.5, 32.0, 20.0, 32.5),
            rect(via, 20.5, 32.0, 21.0, 32.5), // a 2×2 across 20
            rect(inner, 15.0, 40.0, 25.0, 46.0),
            rect(via, 19.5, 41.0, 20.0, 41.5),
            rect(via, 20.5, 41.0, 21.0, 41.5),
            rect(via, 19.5, 42.005, 20.0, 42.505),
            rect(via, 20.5, 42.005, 21.0, 42.505), // R.array, two rows 1.005 apart
        ],
    );
}
