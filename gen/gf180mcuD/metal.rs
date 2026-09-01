// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Metal 1-4: a good and a bad pattern for every rule the `metal` deck declares.
//!
//! The same five rules at each of four levels, differing only in the width and space they
//! ask for, so the fixtures are generated from a table rather than drawn one at a time.
//! Every rule reads the drawn metal directly - `metal1_no_sram` is the drawn layer minus
//! the SRAM core, and no pattern here draws one - which is what makes this deck the
//! easiest of the family to cover.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/metal";
const O: f64 = 10.0;
/// Half the manufacturing grid: enough to break a limit, too little to look like slop.
const D: f64 = 0.005;
/// Space to metal over 10 µm across in both directions, the same at every level.
const WIDE_SPACE: f64 = 0.3;

/// Level, its drawn layer, and the width and space it asks for.
const LEVELS: &[(usize, &str, f64, f64)] = &[
    (1, "metal1_drawn", 0.23, 0.23),
    (2, "metal2_drawn", 0.28, 0.28),
    (3, "metal3_drawn", 0.28, 0.28),
    (4, "metal4_drawn", 0.28, 0.28),
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    for &(n, lname, width, space) in LEVELS {
        let m = layer(pdk, lname);

        // M#.1, min width.  Long enough that the bar clears the area rule either way.
        for (name, w) in [("good", width), ("bad", width - D)] {
            write(
                &format!("M{n}.1.{name}"),
                vec![rect(m, O, O, O + w, O + 2.0)],
            );
        }

        // M#.2a, min space, and the notch the deck checks under the same id.  Both shapes
        // stay well under 10 µm, so the wide-metal rule has nothing to look at.
        for (name, g) in [("good", space), ("bad", space - D)] {
            write(
                &format!("M{n}.2a.{name}"),
                vec![
                    rect(m, O, O, O + 1.0, O + 1.0),
                    rect(m, O + 1.0 + g, O, O + 2.0 + g, O + 1.0),
                    rect(m, O, O + 3.0, O + 1.0, O + 5.0 + g),
                    rect(m, O + 1.0, O + 3.0, O + 2.0, O + 4.0),
                    rect(m, O + 1.0, O + 4.0 + g, O + 2.0, O + 5.0 + g),
                ],
            );
        }

        // M#.2b, space to *wide* metal - over 10 µm across both ways.  The gap stays over
        // the ordinary space at every level, so only the wide rule can speak.
        for (name, g) in [("good", WIDE_SPACE), ("bad", WIDE_SPACE - D)] {
            write(
                &format!("M{n}.2b.{name}"),
                vec![
                    rect(m, O, O, O + 12.0, O + 12.0),
                    rect(m, O + 12.0 + g, O, O + 13.0 + g, O + 1.0),
                ],
            );
        }

        // M#.3, min area.  Square and over the width at every level, so area is the only
        // dimension in play: 0.4² clears 0.1444 and 0.35² does not.
        for (name, s) in [("good", 0.40), ("bad", 0.35)] {
            // 0.1444 µm² is the limit at every level, and 0.35² = 0.1225 is under it
            // while staying over the widest width the deck asks for.
            debug_assert!(s > width, "the area fixture must not also be too narrow");
            write(&format!("M{n}.3.{name}"), vec![rect(m, O, O, O + s, O + s)]);
        }
    }
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
