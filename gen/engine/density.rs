// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the density family: the coverage of Outer over the box of Inner, as a
//! chip number, per 50 µm window, and per large region for the Via it carries.
//!
//! Every expected percentage is read off the drawing: a stripe's height over the box's
//! side, a window's covered part over the window.

use crate::helpers::{layer, library, rect, write_gz};
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
}
