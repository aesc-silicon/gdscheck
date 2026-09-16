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
//!
//! The three scopes meet the bound differently.  Every *part* within the value: a
//! target whose far wall is exactly the value from the reference is clean, and one
//! five nanometres further has a strip beyond reach.  Every *polygon*: one whose near
//! wall is exactly the value away is in reach, five nanometres further it is not.  And
//! the reach confined to a layer goes round a slot in it, so a target on the far leg
//! of a U is out of reach across the slot at 16 µm and in reach along the leg at 13.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/max_space";
pub fn generate(pdk: &PdkConfig) {
    let target = layer(pdk, "Outer");
    let reference = layer(pdk, "Inner");
    let well = layer(pdk, "Via");
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

    // Every part: a 2 µm reference at 30..32 and a target from 34 to 52, whose far wall
    // is 20 from the reference's; then to 52.005.  SPC.max: 0 and 1.
    for (name, far) in [("part_exact", 52.0), ("part_over", 52.005)] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, 34.0, 30.0, far, 32.0),
            ],
        );
    }

    // Every polygon: a 2 µm target whose near wall is 20 from the reference, and one
    // 20.005.  M.poly: 0 and 1.  A block from 34 to 60 has its near part in reach and
    // its far part not: M.poly 0, SPC.max 1.
    for (name, x0, x1) in [
        ("poly_exact", 52.0, 54.0),
        ("poly_over", 52.005, 54.005),
        ("poly_block", 34.0, 60.0),
    ] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, x0, 30.0, x1, 32.0),
            ],
        );
    }

    // The reach confined to Via, drawn as a 30 µm U with a 6 µm slot from the bottom
    // up to y = 26, across the tile lines at 20.  The reference sits on the left leg
    // at 5..7; a target on the right leg at 23..25 is 16 µm away as the crow flies and
    // 48 round the slot: M.within 1, M.poly 0.  One on the same leg at y = 20..22 is
    // 13 up the leg: M.within 0.
    let u = || {
        vec![
            poly(
                well,
                &[
                    (0.0, 0.0),
                    (12.0, 0.0),
                    (12.0, 26.0),
                    (18.0, 26.0),
                    (18.0, 0.0),
                    (30.0, 0.0),
                    (30.0, 30.0),
                    (0.0, 30.0),
                ],
            ),
            rect(reference, 5.0, 5.0, 7.0, 7.0),
        ]
    };
    let mut v = u();
    v.push(rect(target, 23.0, 5.0, 25.0, 7.0));
    write("within_slot", v);
    let mut v = u();
    v.push(rect(target, 5.0, 20.0, 7.0, 22.0));
    write("within_leg", v);

    // Every edge: a 0.2 µm bar of Outer 2 µm tall, and the reference 20 from its right
    // wall: the left wall is 20.2 away and the other three reach.  M.edge: 1.  At
    // 20.005 none of the four reaches.  M.edge: 4.
    for (name, x) in [("edge_exact", 50.2), ("edge_over", 50.205)] {
        write(
            name,
            vec![
                rect(target, 30.0, 30.0, 30.2, 32.0),
                rect(reference, x, 30.0, x + 2.0, 32.0),
            ],
        );
    }
}
