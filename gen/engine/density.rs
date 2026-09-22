// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the density family: the coverage of Outer over the box of Inner, as a
//! chip number, per 50 µm window, and per large region for the Via it carries.
//!
//! Every expected percentage is read off the drawing: a stripe's height over the box's
//! side, a window's covered part over the window.

use crate::helpers::{flat_array, layer, library, poly, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/density";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A 100 µm box of Inner with Outer over its lower 40 µm, and over its lower 70.
    // D.min at 50 %: 1 and 0.  D.max at 60 %: 0 and 1.
    write(
        "chip_40",
        vec![
            rect(inner, 0.0, 0.0, 100.0, 100.0),
            rect(outer, 0.0, 0.0, 100.0, 40.0),
        ],
    );
    write(
        "chip_70",
        vec![
            rect(inner, 0.0, 0.0, 100.0, 100.0),
            rect(outer, 0.0, 0.0, 100.0, 70.0),
        ],
    );

    // Inner as a hollow 1 µm frame around the same 100 µm: its material is a sliver,
    // its box is the die, and Outer over the lower 25 µm is 25 % of the die.
    // D.min: 1.
    write(
        "ring_frame",
        vec![
            rect(inner, 0.0, 0.0, 100.0, 1.0),
            rect(inner, 0.0, 99.0, 100.0, 100.0),
            rect(inner, 0.0, 0.0, 1.0, 100.0),
            rect(inner, 99.0, 0.0, 100.0, 100.0),
            rect(outer, 0.0, 0.0, 100.0, 25.0),
        ],
    );

    // 50 µm windows sliding over a 100 µm box: Outer fills the lower-left quarter and
    // half of the quarter above it.  D.wmin at 50 %: every window off the Outer is
    // under, and they overlap into one violation (the window at (0, 50) is exactly
    // 50 and not under).  D.wmax at 60 %: the windows on the Outer, one violation.
    write(
        "window_quarters",
        vec![
            rect(inner, 0.0, 0.0, 100.0, 100.0),
            rect(outer, 0.0, 0.0, 50.0, 75.0),
        ],
    );

    // A die 130 by 50 in 50 µm windows: the windows slide along and one is laid against
    // the far edge.  Outer over the lower 20 µm is 40 % of every window: one D.wmin
    // violation, no D.wmax.
    write(
        "window_partial",
        vec![
            rect(inner, 0.0, 0.0, 130.0, 50.0),
            rect(outer, 0.0, 0.0, 130.0, 20.0),
        ],
    );

    // A 100 µm plate of Outer, wide enough to count, with Via over 5 % of it, then 10 %;
    // and a 20 µm plate too small to count, with no Via at all.  D.region at 6 %:
    // 1, 0 and 0.
    write(
        "region_05",
        vec![
            rect(outer, 0.0, 0.0, 100.0, 100.0),
            rect(via, 0.0, 0.0, 100.0, 5.0),
        ],
    );
    write(
        "region_10",
        vec![
            rect(outer, 0.0, 0.0, 100.0, 100.0),
            rect(via, 0.0, 0.0, 100.0, 10.0),
        ],
    );
    write("region_small", vec![rect(outer, 0.0, 0.0, 20.0, 20.0)]);

    // --- The hardening patterns: what a rule manual's density asks of any layer,
    // drawn once here for every deck of every PDK (hardening/SPEC.md).  A density is
    // one number per layout, so the bound is a layout apiece.

    // The bound.  Outer over exactly 50 % of the die is not under D.min's 50, over
    // 49.995 % it is; over 60 % it is not over D.max's 60, over 60.005 % it is.
    let die = || rect(inner, 0.0, 0.0, 100.0, 100.0);
    for (name, h) in [
        ("bound_50", 50.0),
        ("bound_49995", 49.995),
        ("bound_60", 60.0),
        ("bound_60005", 60.005),
    ] {
        write(name, vec![die(), rect(outer, 0.0, 0.0, 100.0, h)]);
    }

    // 45° geometry.  A right triangle over half the die is 50 %, one whose vertical
    // leg is 99.99 is 49.995 %.  D.min: 0 and 1.
    write(
        "bound_45",
        vec![
            die(),
            poly(outer, &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)]),
        ],
    );
    write(
        "bound_45_under",
        vec![
            die(),
            poly(outer, &[(0.0, 0.0), (100.0, 0.0), (100.0, 99.99)]),
        ],
    );

    // Shapes that merge, and shapes past the die.  Two bands 0..40 and 20..60, a union
    // of 60 % and a sum of 80; a square beyond the die, which the boundary cuts.
    // D.max: 0; D.min: 0.  The bands 0..40 and 20..60.005: D.max 1.
    write(
        "merge",
        vec![
            die(),
            rect(outer, 0.0, 0.0, 100.0, 40.0),
            rect(outer, 0.0, 20.0, 100.0, 60.0),
            rect(outer, 200.0, 200.0, 300.0, 300.0),
        ],
    );
    write(
        "merge_over",
        vec![
            die(),
            rect(outer, 0.0, 0.0, 100.0, 40.0),
            rect(outer, 0.0, 20.0, 100.0, 60.005),
        ],
    );

    // Tile lines.  A die from 3 to 103, off the lines, with Outer as four 10 µm bars
    // across x = 20, 40, 60 and 80, an 8 µm one across 100 to the die's edge and a 2 µm
    // one at its left edge, 50 in all: D.min 0; the left bar 0.005 short: D.min 1.
    let bars = |short: f64| {
        let mut v = vec![rect(inner, 3.0, 3.0, 103.0, 103.0)];
        for x in [15.0, 35.0, 55.0, 75.0] {
            v.push(rect(outer, x, 3.0, x + 10.0, 103.0));
        }
        v.push(rect(outer, 95.0, 3.0, 103.0, 103.0));
        v.push(rect(outer, 3.0, 3.0, 5.0 - short, 103.0));
        v
    };
    write("tile_lines", bars(0.0));
    write("tile_lines_under", bars(0.005));

    // Fifty 20 × 10 boxes on a 20 µm pitch over a 100 × 200 die, half of it, flat and
    // as an array reference: D.min 0, D.max 0.  With the boxes 9.999 tall, 49.995 %:
    // D.min 1 each.
    let big = || rect(inner, 0.0, 0.0, 100.0, 200.0);
    for (name, h) in [("array", 10.0), ("array_under", 9.999)] {
        let cell = vec![rect(outer, 0.0, 0.0, 20.0, h)];
        let mut flat = vec![big()];
        flat.extend(flat_array(&cell, 5, 10, 20.0));
        write(&format!("{name}_flat"), flat);
        let mut lib = ref_array(cell, 5, 10, 20.0);
        lib.structs
            .iter_mut()
            .find(|st| st.name == "TOP")
            .expect("TOP")
            .elems
            .push(big());
        write_gz(&format!("{DIR}/{name}_ref.gds.gz"), lib);
    }

    // Windows.  A 200 µm die under Outer but for an 80 µm hole at 90..170: a 50 µm
    // window lies whole in the hole from the anchors of any tile - 100 for the 20 and
    // 100 µm tiles, 98 for the 7 µm one.  D.wmin: 1.  (The windows on the Outer are
    // full and over D.wmax, but how many separate violations they make depends on
    // the step: overlapping windows are one, and at a 100 µm step none overlap.)
    write(
        "window_hole",
        vec![
            rect(inner, 0.0, 0.0, 200.0, 200.0),
            rect(outer, 0.0, 0.0, 200.0, 90.0),
            rect(outer, 0.0, 170.0, 200.0, 200.0),
            rect(outer, 0.0, 90.0, 90.0, 170.0),
            rect(outer, 170.0, 90.0, 200.0, 170.0),
        ],
    );

    // Regions.  A 40 µm plate across x = 20 with Via over 6 % of it is not under
    // D.region, over 5.995 % it is; a plate drawn as four overlapping boxes with Via
    // over 5 % of the union fires once; a plate 35.005 wide is wide enough and one
    // exactly 35 is not - the size is a strict bound, "larger than".  D.region: 0, 1,
    // 1, 1, 0.
    write(
        "region_tile_line",
        vec![
            rect(outer, 0.0, 0.0, 40.0, 40.0),
            rect(via, 0.0, 0.0, 40.0, 2.4),
        ],
    );
    write(
        "region_tile_line_under",
        vec![
            rect(outer, 0.0, 0.0, 40.0, 40.0),
            rect(via, 0.0, 0.0, 40.0, 2.398),
        ],
    );
    write(
        "region_merge",
        vec![
            rect(outer, 0.0, 0.0, 25.0, 25.0),
            rect(outer, 15.0, 0.0, 40.0, 25.0),
            rect(outer, 0.0, 15.0, 25.0, 40.0),
            rect(outer, 15.0, 15.0, 40.0, 40.0),
            rect(via, 0.0, 0.0, 40.0, 2.0),
        ],
    );
    write(
        "region_size_over",
        vec![
            rect(outer, 0.0, 0.0, 35.005, 35.005),
            rect(via, 0.0, 0.0, 35.005, 1.0),
        ],
    );
    write(
        "region_size_exact",
        vec![
            rect(outer, 0.0, 0.0, 35.0, 35.0),
            rect(via, 0.0, 0.0, 35.0, 1.0),
        ],
    );

    // Fifty 40 µm plates with Via over 5 % of each, at a pitch of 50, flat and as an
    // array reference.  D.region: 50 each.
    let cell = vec![
        rect(outer, 0.0, 0.0, 40.0, 40.0),
        rect(via, 0.0, 0.0, 40.0, 2.0),
    ];
    write("region_array_flat", flat_array(&cell, 10, 5, 50.0));
    write_gz(
        &format!("{DIR}/region_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 50.0),
    );
}
