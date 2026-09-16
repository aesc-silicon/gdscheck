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

use crate::helpers::{layer, library, poly, rect, write_gz};
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
}
