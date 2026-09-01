// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dummy-fill exclude marker: a good and a bad pattern for every rule in the
//! `dummy_exclude` deck.
//!
//! The only deck here whose bad half has to be *large*: DE.3 caps a marker's area at
//! 15000 µm², so its fixture is a 130 µm square where every other pattern in the tree
//! fits in a few microns.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/dummy_exclude";
const O: f64 = 10.0;
const D: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    let ndmy = layer(pdk, "ndmy");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // DE.2, min width 0.8 - measured on the two exclude markers together.  A single
    // shape, so neither the area cap nor the spacing has anything to say.
    for (name, w) in [("good", 0.8), ("bad", 0.8 - D)] {
        write(
            &format!("DE.2.{name}"),
            vec![rect(ndmy, O, O, O + w, O + 5.0)],
        );
    }

    // DE.3, max area 15000 µm².  Square, so it clears the 0.8 width comfortably: 100²
    // is well under the cap and 130² is over it.
    for (name, s) in [("good", 100.0), ("bad", 130.0)] {
        write(
            &format!("DE.3.{name}"),
            vec![rect(ndmy, O, O, O + s, O + s)],
        );
    }

    // DE.4, min space 20 µm, and the notch under the same id.  Each shape is 50 µm square:
    // over the width, and at 2500 µm² far under the area cap.
    for (name, g) in [("good", 20.0), ("bad", 20.0 - D)] {
        write(
            &format!("DE.4.{name}"),
            vec![
                rect(ndmy, O, O, O + 50.0, O + 50.0),
                rect(ndmy, O + 50.0 + g, O, O + 100.0 + g, O + 50.0),
                // A C whose opening is the same gap: a notch is a space within one shape.
                rect(ndmy, O, O + 80.0, O + 50.0, O + 130.0 + g),
                rect(ndmy, O + 50.0, O + 80.0, O + 100.0, O + 105.0),
                rect(ndmy, O + 50.0, O + 105.0 + g, O + 100.0, O + 130.0 + g),
            ],
        );
    }
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
