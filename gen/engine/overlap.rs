// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`min_overlap`](gdscheck::checks::min_overlap): the space measured the
//! other way round, the shallowest penetration of two regions that share area, met
//! exactly and missed by five nanometres, and the hardening classes of
//! hardening/SPEC.md - unions, the tile lines, fifty at once, the extremes.  The value
//! is 0.5 µm; a violation is one pair of regions.

use crate::helpers::{flat_array, layer, library, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/overlap";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A 2 µm Outer square from (x, y) and a 2 µm Inner square reaching `dx` into it
    // from the right and `dy` from above; `dy` of 2 or more lies level with it.
    let pair = |x: f64, y: f64, dx: f64, dy: f64| {
        vec![
            rect(outer, x, y, x + 2.0, y + 2.0),
            rect(
                inner,
                x + 2.0 - dx,
                y + 2.0 - dy,
                x + 4.0 - dx,
                y + 4.0 - dy,
            ),
        ]
    };

    // The bound.  Squares overlapping by 0.5 sideways and by 0.495; by 0.5 and 0.495
    // upwards; corner to corner by 0.5 in x and 1.0 in y, whose shallowest penetration
    // is 0.5, and by 0.495 in x; a square wholly inside the other has no walls facing
    // across it.  OV.min: 3.
    let mut e = pair(2.0, 2.0, 0.5, 2.0);
    e.extend(pair(8.0, 2.0, 0.495, 2.0)); // OV.min
    e.extend(pair(14.0, 2.0, 2.0, 0.5));
    e.extend(pair(20.0, 2.0, 2.0, 0.495)); // OV.min
    e.extend(pair(2.0, 10.0, 0.5, 1.0));
    e.extend(pair(8.0, 10.0, 0.495, 1.0)); // OV.min
    e.extend([
        rect(outer, 14.0, 10.0, 18.0, 14.0),
        rect(inner, 15.0, 11.0, 17.0, 13.0), // wholly inside
    ]);
    write("bound", e);

    // Shapes that merge.  An Inner region drawn as two abutting boxes reaching 0.495
    // into an Outer square is one pair; an Outer drawn as two overlapping boxes with an
    // Inner reaching 0.495 into their union is one pair; two Inner squares reaching
    // 0.495 into one Outer bar are two.  OV.min: 4.
    write(
        "merge",
        vec![
            rect(outer, 2.0, 2.0, 4.0, 4.0),
            rect(inner, 3.505, 2.0, 4.5, 4.0),
            rect(inner, 4.5, 2.0, 5.505, 4.0), // one pair
            rect(outer, 8.0, 2.0, 9.5, 4.0),
            rect(outer, 9.0, 2.0, 10.0, 4.0),
            rect(inner, 9.505, 2.0, 11.505, 4.0), // one pair
            rect(outer, 14.0, 2.0, 24.0, 4.0),
            rect(inner, 13.0, 2.0, 14.495, 4.0),
            rect(inner, 23.505, 2.0, 25.0, 4.0), // two pairs
        ],
    );

    // Tile lines.  0.495 overlaps across x = 20, ending on 20, starting on 20, across
    // 21, 40 and 42, inside a tile at 10, across y = 20, at (1000, 1000); a 0.5 overlap
    // across 20 is clean.  OV.min: 9.
    let mut e = vec![];
    for (i, x1) in [10.245, 20.245, 20.0, 20.495, 21.245, 40.245, 42.245]
        .iter()
        .enumerate()
    {
        let y = 2.0 + 4.0 * i as f64;
        e.push(rect(outer, x1 - 3.0, y, *x1, y + 2.0));
        e.push(rect(inner, x1 - 0.495, y, x1 + 2.0, y + 2.0)); // overlap x1 - 0.495 .. x1
    }
    e.extend([
        rect(outer, 50.0, 17.0, 52.0, 20.245),
        rect(inner, 50.0, 19.75, 52.0, 22.0), // across y = 20
        rect(outer, 997.0, 1000.0, 1000.245, 1002.0),
        rect(inner, 999.75, 1000.0, 1002.0, 1002.0),
        rect(outer, 17.0, 32.0, 20.25, 34.0),
        rect(inner, 19.75, 32.0, 22.0, 34.0), // 0.5 across 20: clean
    ]);
    write("tile_lines", e);

    // Fifty 0.495 overlaps, flat and as an array reference.  OV.min: 50 each.
    let cell = vec![
        rect(outer, 0.2, 0.2, 2.2, 2.2),
        rect(inner, 1.705, 0.2, 3.705, 2.2),
    ];
    write("array_flat", flat_array(&cell, 10, 5, 5.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 5.0),
    );

    // Small and long.  A 0.005 sliver of Inner reaching 0.005 into an Outer square; two
    // 300 µm bars overlapping by 0.495 along their length, one pair; two overlapping
    // by 0.5 are clean.  OV.min: 2.
    write(
        "extremes",
        vec![
            rect(outer, 2.0, 2.0, 4.0, 4.0),
            rect(inner, 3.995, 2.0, 4.005, 4.0), // OV.min
            rect(outer, 2.0, 6.0, 302.0, 8.0),
            rect(inner, 2.0, 7.505, 302.0, 10.0), // OV.min
            rect(outer, 2.0, 12.0, 302.0, 14.0),
            rect(inner, 2.0, 13.5, 302.0, 16.0), // clean
        ],
    );
}
