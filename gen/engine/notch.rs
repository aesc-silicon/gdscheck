// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`min_notch`](gdscheck::checks::space::notch): a slot exactly the value
//! wide and one five nanometres narrower, axis-aligned and at 45°, a thin hole, and the
//! bent and length gates met and not met.
//!
//! A 45° shape is drawn in the rotated frame `a = x - y`, `b = x + y`, where its walls
//! are the lines of constant `a` or `b` and a vertex is `((a + b) / 2, (b - a) / 2)` -
//! on the grid when every `a` and `b` is an even number of nanometres.  A slot `Δa` wide
//! in that frame is `Δa / √2` across, never on the grid, so the two patterns straddle
//! the value by the nearest even offsets, 0.708 and 0.706 µm.

use crate::helpers::{flat_array, layer, library, poly, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/notch";

/// A polygon given in the rotated frame, `(a, b)` per vertex.
fn rotated(l: (i16, i16), ab: &[(f64, f64)]) -> gds21::GdsElement {
    let pts: Vec<(f64, f64)> = ab
        .iter()
        .map(|&(a, b)| ((a + b) * 0.5, (b - a) * 0.5))
        .collect();
    poly(l, &pts)
}

/// A block from `(x0, y0)` to `(x1, y1)` with a slot `w` wide cut down from its top
/// wall at `sx`, `depth` deep.
fn slotted(
    l: (i16, i16),
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    sx: f64,
    w: f64,
    depth: f64,
) -> gds21::GdsElement {
    poly(
        l,
        &[
            (x0, y0),
            (x1, y0),
            (x1, y1),
            (sx + w, y1),
            (sx + w, y1 - depth),
            (sx, y1 - depth),
            (sx, y1),
            (x0, y1),
        ],
    )
}

/// The same block in the rotated frame: `a` from 0 to 6, `b` from 60 to 66, the slot
/// cut down from the `b = 66` wall.
fn slotted45(l: (i16, i16), sa: f64, w: f64, depth: f64) -> gds21::GdsElement {
    rotated(
        l,
        &[
            (0.0, 60.0),
            (6.0, 60.0),
            (6.0, 66.0),
            (sa + w, 66.0),
            (sa + w, 66.0 - depth),
            (sa, 66.0 - depth),
            (sa, 66.0),
            (0.0, 66.0),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };
    let block = (30.0, 30.0, 36.0, 34.0);

    // A 6 by 4 block with a slot 3 µm deep, exactly N.min's 0.5 wide and 0.495.
    // N.min: 0 and 1.
    for (name, w) in [("slot_exact", 0.5), ("slot_under", 0.495)] {
        write(name, vec![slotted(outer, block, 32.5, w, 3.0)]);
    }

    // The 0.495 slot closed off by a bar over the top: a hole 0.495 wide and 3 tall,
    // whose two long walls face across it.  N.min: 1.
    write(
        "hole_thin",
        vec![
            slotted(outer, block, 32.0, 0.495, 3.0),
            rect(outer, 30.0, 34.0, 36.0, 34.5),
        ],
    );

    // The block turned 45°, its slot 4 µm deep in the rotated frame: 0.708 wide there
    // is 0.5006 µm across, 0.706 is 0.4992.  N.min: 0 and 1.  Both are between 45°
    // walls under N.bent's 0.8 along a run over its 1 µm, so N.bent: 1 and 1.
    for (name, w) in [("diag_exact", 0.708), ("diag_under", 0.706)] {
        write(name, vec![slotted45(outer, 2.646, w, 4.0)]);
    }

    // A 45° slot 0.85 wide in the frame, 0.601 µm across: over N.min's 0.5, under
    // N.bent's 0.8.  Four deep in the frame it runs 2.83 µm; one deep, 0.71, short of
    // N.bent's 1 µm.  N.bent: 1 and 0; N.min: 0 and 0.
    write("bent_slot", vec![slotted45(outer, 2.574, 0.85, 4.0)]);
    write("bent_short", vec![slotted45(outer, 2.574, 0.85, 1.0)]);

    // A 0.4 µm slot in a block of Inner, exactly N.len's 2 µm deep and 2.005.
    // N.len: 0 and 1.
    for (name, depth) in [("len_exact", 2.0), ("len_over", 2.005)] {
        write(name, vec![slotted(inner, block, 32.5, 0.4, depth)]);
    }

    // --- The hardening patterns: what a rule manual's "space or notch" asks of one
    // shape, drawn once here for every deck of every PDK (hardening/SPEC.md).  N.min is
    // 0.5; a violation is one notch.

    // The shapes.  A straight U notch of 0.495 (with a 0.5 control), a straight-vs-45°
    // notch of 0.495 (control 0.5), a comb with three 0.495 slots, a 0.495 slot into a
    // plate, a keyhole ring with a 0.495 hole, two Ls facing across 0.495 (one shape,
    // joined at the base), and a channel 0.495 wide cut through a ring's wall.  N.min: 8.
    let u = |x: f64, nd: f64| {
        poly(
            outer,
            &[
                (x, 2.0),
                (x + 2.0, 2.0),
                (x + 2.0, 2.5),
                (x + 1.0, 2.5),
                (x + 1.0, 2.5 + nd),
                (x + 2.0, 2.5 + nd),
                (x + 2.0, 4.0),
                (x, 4.0),
            ],
        )
    };
    let u45 = |x: f64, nd: f64| {
        poly(
            outer,
            &[
                (x, 2.0),
                (x + 2.0, 2.0),
                (x + 2.0, 2.5),
                (x + 1.0, 2.5),
                (x + 1.0, 2.5 + nd),
                (x + 2.0, 3.5 + nd),
                (x + 2.0, 4.5),
                (x, 4.5),
            ],
        )
    };
    let mut e = vec![u(2.0, 0.495), u(5.0, 0.5), u45(8.0, 0.495), u45(11.0, 0.5)];
    e.push(poly(
        outer,
        &[
            (14.0, 2.0),
            (18.0, 2.0),
            (18.0, 4.0),
            (17.5, 4.0),
            (17.5, 3.0),
            (17.005, 3.0),
            (17.005, 4.0),
            (16.505, 4.0),
            (16.505, 3.0),
            (16.01, 3.0),
            (16.01, 4.0),
            (15.51, 4.0),
            (15.51, 3.0),
            (15.015, 3.0),
            (15.015, 4.0),
            (14.0, 4.0),
        ],
    )); // comb: three 0.495 slots
    e.push(poly(
        outer,
        &[
            (20.0, 2.0),
            (24.0, 2.0),
            (24.0, 5.0),
            (22.25, 5.0),
            (22.25, 3.5),
            (21.755, 3.5),
            (21.755, 5.0),
            (20.0, 5.0),
        ],
    )); // slot 0.495 wide into a plate
    e.push(poly(
        outer,
        &[
            (2.0, 6.0),
            (6.0, 6.0),
            (6.0, 10.0),
            (2.0, 10.0),
            (2.0, 7.0),
            (3.75, 7.0),
            (3.75, 9.0),
            (4.245, 9.0),
            (4.245, 7.0),
            (2.0, 7.0),
        ],
    )); // keyhole: a hole 0.495 wide
    e.push(poly(
        outer,
        &[
            (8.0, 6.0),
            (12.0, 6.0),
            (12.0, 10.0),
            (11.0, 10.0),
            (11.0, 7.5),
            (9.505, 7.5),
            (9.505, 10.0),
            (8.0, 10.0),
        ],
    )); // two arms 0.495 apart: a U reading as facing Ls
    e.extend([
        rect(outer, 14.0, 6.0, 20.0, 7.0),
        rect(outer, 14.0, 9.0, 16.75, 10.0),
        rect(outer, 17.245, 9.0, 20.0, 10.0),
        rect(outer, 14.0, 7.0, 15.0, 9.0),
        rect(outer, 19.0, 7.0, 20.0, 9.0),
    ]); // a ring with a 0.495 channel through its top wall
    write("shapes", e);

    // Tile lines.  0.495 slots into plates with the slot ending on x = 20, straddling
    // 20, starting on 20, straddling 21, on 40, straddling 42, inside a tile at 10; a
    // ring's channel straddling 20 and one straddling 40.  N.min: 9.
    let slot = |x: f64, y: f64| {
        poly(
            outer,
            &[
                (x - 1.0, y),
                (x + 1.495, y),
                (x + 1.495, y + 3.0),
                (x + 0.495, y + 3.0),
                (x + 0.495, y + 1.0),
                (x, y + 1.0),
                (x, y + 3.0),
                (x - 1.0, y + 3.0),
            ],
        )
    };
    let channel = |x: f64, y: f64| {
        vec![
            rect(outer, x - 3.0, y, x + 3.0, y + 1.0),
            rect(outer, x - 3.0, y + 3.0, x - 0.25, y + 4.0),
            rect(outer, x + 0.245, y + 3.0, x + 3.0, y + 4.0),
            rect(outer, x - 3.0, y + 1.0, x - 2.0, y + 3.0),
            rect(outer, x + 2.0, y + 1.0, x + 3.0, y + 3.0),
        ]
    };
    let mut e = vec![
        slot(9.505, 2.0),
        slot(19.505, 2.0),
        slot(19.75, 6.0),
        slot(20.0, 10.0),
        slot(20.75, 14.0),
        slot(39.505, 2.0),
        slot(41.75, 6.0),
    ];
    e.extend(channel(20.0, 22.0));
    e.extend(channel(40.0, 22.0));
    write("tile_lines", e);

    // Fifty 0.495 slots, flat and as an array reference.  N.min: 50 each.
    let cell = vec![poly(
        outer,
        &[
            (0.2, 0.2),
            (1.7, 0.2),
            (1.7, 1.7),
            (1.2, 1.7),
            (1.2, 0.7),
            (0.705, 0.7),
            (0.705, 1.7),
            (0.2, 1.7),
        ],
    )];
    write("array_flat", flat_array(&cell, 10, 5, 2.5));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.5),
    );

    // Long and far.  A 300 µm slot 0.495 wide (one notch), a slot at (1000, 1000).
    // N.min: 2.
    write(
        "extremes",
        vec![
            poly(
                outer,
                &[
                    (2.0, 2.0),
                    (302.0, 2.0),
                    (302.0, 4.0),
                    (301.0, 4.0),
                    (301.0, 3.0),
                    (3.0, 3.0),
                    (3.0, 3.495),
                    (301.0, 3.495),
                    (301.0, 4.0),
                    (2.0, 4.0),
                ],
            ),
            slot(1000.0, 1000.0),
        ],
    );
}
