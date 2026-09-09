// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`max_space`](gdscheck::checks::max_space): every part of one layer
//! within a distance of another, asked where the answer depends on how the reference is
//! grown and on where the tile lines fall.
//!
//! The reference grown by the value is a square structuring element - KLayout's `sized`
//! on rectilinear geometry - so a target diagonally off a corner is covered out to the
//! value in each axis, further than a circle reaches.  A tie ring is the shape that
//! tells a grown polygon from a grown bounding box: the ring's box grown by the value
//! covers its whole interior, the ring itself does not, and a device in the middle of a
//! large ring is exactly what the latch-up rule is for.  The tile lines are where a
//! target or a reference cut in two has to be read as one shape: a gap across a line is
//! one gap, and a reference across one covers as far as it would whole.  The deck's
//! value is 20 µm, and so is the tile, with lines at its multiples.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/max_space";
pub fn generate(pdk: &PdkConfig) {
    let target = layer(pdk, "Outer");
    let reference = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A square tie ring 100 µm across with a 2 µm band, drawn as four bars, spanning
    // several tiles.  A 4 µm target at its centre is 46 µm from the band and lies in a
    // gap; one 8 µm from the band is covered.
    let ring = |o: f64| {
        vec![
            rect(reference, o, o, o + 100.0, o + 2.0),
            rect(reference, o, o + 98.0, o + 100.0, o + 100.0),
            rect(reference, o, o + 2.0, o + 2.0, o + 98.0),
            rect(reference, o + 98.0, o + 2.0, o + 100.0, o + 98.0),
        ]
    };
    let mut v = ring(10.0);
    v.push(rect(target, 58.0, 58.0, 62.0, 62.0));
    write("ring_far", v);
    let mut v = ring(10.0);
    v.push(rect(target, 20.0, 58.0, 24.0, 62.0));
    write("ring_near", v);

    // A target across the tile line at x = 40, the reference 28 µm off: one gap, not
    // one per tile the target has a piece in.  At 5 µm off it is covered.
    for (name, rx) in [("straddle_far", 70.0), ("straddle_near", 47.0)] {
        write(
            name,
            vec![
                rect(target, 38.0, 30.0, 42.0, 34.0),
                rect(reference, rx, 30.0, rx + 2.0, 34.0),
            ],
        );
    }

    // A reference across the tile line at x = 40 covers two tiles further, as far as the
    // value: a target 18 µm off is covered, one 22 µm off is not.
    for (name, tx) in [("reach_far", 64.0), ("reach_near", 60.0)] {
        write(
            name,
            vec![
                rect(reference, 38.0, 30.0, 42.0, 34.0),
                rect(target, tx, 30.0, tx + 2.0, 34.0),
            ],
        );
    }

    // A target diagonally off the reference's corner, 18 µm away in each axis: within
    // the square's reach, and 25 µm as the crow flies.  At 22 µm in each axis it is out.
    for (name, off) in [("corner_far", 24.0), ("corner_near", 20.0)] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, 30.0 + off, 30.0 + off, 32.0 + off, 32.0 + off),
            ],
        );
    }
}
