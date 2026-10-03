// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for `facing` and `sides: opposite`: a rule on one axis, read on shapes that
//! break the bound across one axis and keep it across the other, and a via enclosed on
//! one pair of opposite sides, on the other, on both and on neither.
//!
//! Every value in `decks/facing.yml` is 0.5 µm.  A width or enclosure short on two
//! opposite walls reports each wall; a spacing or notch reports once.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/facing";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A bar 0.4 across in x and 3 tall, and the same turned: narrow across one axis,
    // wide across the other.  W.x: 2 and 0; W.y: 0 and 2.
    write("narrow_x", vec![rect(outer, 30.0, 30.0, 30.4, 33.0)]);
    write("narrow_y", vec![rect(outer, 30.0, 30.0, 33.0, 30.4)]);

    // Two 2 µm squares 0.4 apart side by side, the same stacked, and the same offset
    // corner to corner by 0.3 in each axis - 0.424 as the crow flies, and no wall of
    // one facing a wall of the other.  S.x: 1, 0, 0; S.y: 0, 1, 0; S.none: 0, 0, 1.
    write(
        "gap_x",
        vec![
            rect(outer, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 32.4, 30.0, 34.4, 32.0),
        ],
    );
    write(
        "gap_y",
        vec![
            rect(outer, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 30.0, 32.4, 32.0, 34.4),
        ],
    );
    write(
        "gap_corner",
        vec![
            rect(outer, 30.0, 30.0, 32.0, 32.0),
            rect(outer, 32.3, 32.3, 34.3, 34.3),
        ],
    );

    // A block with a slot 0.4 wide cut down into it from the top, and one cut in from
    // the side.  N.x: 1 and 0; N.y: 0 and 1.
    write(
        "notch_x",
        vec![poly(
            outer,
            &[
                (30.0, 30.0),
                (33.0, 30.0),
                (33.0, 33.0),
                (31.7, 33.0),
                (31.7, 31.0),
                (31.3, 31.0),
                (31.3, 33.0),
                (30.0, 33.0),
            ],
        )],
    );
    write(
        "notch_y",
        vec![poly(
            outer,
            &[
                (30.0, 30.0),
                (33.0, 30.0),
                (33.0, 31.3),
                (31.0, 31.3),
                (31.0, 31.7),
                (33.0, 31.7),
                (33.0, 33.0),
                (30.0, 33.0),
            ],
        )],
    );

    // A 3 µm square of Outer round an Inner 0.3 in from the sides and 1 in from top and
    // bottom; the same turned; one 0.3 in all round; and one flush with both sides,
    // the width of the wire it sits on.
    //
    // E.x reads the side walls: 2, 0, 2, 2.  E.opp wants one pair of opposite sides at
    // 0.5: 0, 0, 1, 0.  E.max_opp wants one pair flush: 1, 1, 1, 0.
    let square = || rect(outer, 30.0, 30.0, 33.0, 33.0);
    write("enc_x", vec![square(), rect(inner, 30.3, 31.0, 32.7, 32.0)]);
    write("enc_y", vec![square(), rect(inner, 31.0, 30.3, 32.0, 32.7)]);
    write(
        "enc_short",
        vec![square(), rect(inner, 30.3, 30.3, 32.7, 32.7)],
    );
    write(
        "enc_flush",
        vec![square(), rect(inner, 30.0, 31.0, 33.0, 32.0)],
    );
}
