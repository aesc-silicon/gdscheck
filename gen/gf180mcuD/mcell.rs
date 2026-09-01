// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Memory-cell marker: a good and a bad pattern for every rule in the `mcell` deck.
//!
//! Five rules over one marker layer, and the only deck here that bounds an *enclosed*
//! area - MC.4 is about the hole in a ring rather than the ring.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/mcell";
const O: f64 = 10.0;
const D: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    let m = layer(pdk, "mcell_feol_mk");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // MC.1, min width 0.4.  Two microns long, so the area rule has nothing to say.
    for (name, w) in [("good", 0.4), ("bad", 0.4 - D)] {
        write(&format!("MC.1.{name}"), vec![rect(m, O, O, O + w, O + 2.0)]);
    }

    // MC.2, min space 0.4, and the notch under the same id.
    for (name, g) in [("good", 0.4), ("bad", 0.4 - D)] {
        write(
            &format!("MC.2.{name}"),
            vec![
                rect(m, O, O, O + 2.0, O + 2.0),
                rect(m, O + 2.0 + g, O, O + 4.0 + g, O + 2.0),
                rect(m, O, O + 5.0, O + 2.0, O + 5.0 + g + 2.0),
                rect(m, O + 2.0, O + 5.0, O + 4.0, O + 6.0),
                rect(m, O + 2.0, O + 5.0 + g + 1.0, O + 4.0, O + 5.0 + g + 2.0),
            ],
        );
    }

    // MC.3, min area 0.35.  Square and comfortably over the 0.4 width either way.
    for (name, s) in [("good", 0.65), ("bad", 0.55)] {
        write(&format!("MC.3.{name}"), vec![rect(m, O, O, O + s, O + s)]);
    }

    // MC.4, min *enclosed* area 0.35: the hole in the ring, not the ring.  Arms stay a
    // micron thick so nothing else has anything to say about them.
    for (name, h) in [("good", 0.65), ("bad", 0.55)] {
        let a = 1.0; // arm thickness
        write(
            &format!("MC.4.{name}"),
            vec![poly(
                m,
                &[
                    (O, O),
                    (O + h + 2.0 * a, O),
                    (O + h + 2.0 * a, O + h + 2.0 * a),
                    (O, O + h + 2.0 * a),
                    (O, O + a),
                    (O + a, O + a),
                    (O + a, O + a + h),
                    (O + a + h, O + a + h),
                    (O + a + h, O + a),
                    (O, O + a),
                ],
            )],
        );
    }
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
