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

    hardening(pdk);
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Section 10.8 of the manual is three rules on two marker layers, with a `#[case]` in
// the `hardening_dummy_exclude` table of `tests/gf180mcuD.rs` and the findings in
// `hardening/reports/gf180mcuD/dummy_exclude.md`.
//
// Every shape here is an exclude marker, and the two that are not part of a spacing
// probe stand 20 µm or more from any other NDMY so that DE.4 has nothing to say about
// them - the one rule in this deck that reaches between fixtures of the same layout.

fn hardening(pdk: &PdkConfig) {
    let (ndmy, pmndmy) = (layer(pdk, "ndmy"), layer(pdk, "pmndmy"));

    // DE.2: the minimum size, on each layer on its own.  0.8 is the bound and 0.795 the
    // step past it.
    let mut v = Vec::new();
    for (i, &(l, w)) in [
        (ndmy, 0.8),
        (ndmy, 0.8 - D),
        (pmndmy, 0.8),
        (pmndmy, 0.8 - D),
    ]
    .iter()
    .enumerate()
    {
        let x = O + i as f64 * 30.0;
        v.push(rect(l, x, O, x + w, O + 5.0));
    }
    write("DE.2.h1", v);

    // DE.2 again, with the two markers overlapping.  The rule is the minimum size of an
    // NDMY *or* a PMNDMY, so each layer answers for itself: a 0.795 NDMY strip lying on
    // a 20 µm PMNDMY plate is still a 0.795 NDMY, and the same the other way round.
    write(
        "DE.2.h2",
        vec![
            rect(pmndmy, O - 5.0, O - 5.0, O + 15.0, O + 15.0),
            rect(ndmy, O, O, O + 0.8 - D, O + 5.0),
            rect(ndmy, O + 55.0, O - 5.0, O + 75.0, O + 15.0),
            rect(pmndmy, O + 60.0, O, O + 60.8 - D, O + 5.0),
        ],
    );

    // DE.3: "Maximum NDMY size (um2) 15000.  If size greater than 15000um2 then two
    // sides should not be greater than (um) 80."  A marker over the area is allowed as
    // long as it is long rather than broad - which is the only thing the second half of
    // the rule can be for - so 75 x 200 (exactly the cap), 80 x 200 (over the cap, one
    // side over 80) are clean, and 100 x 200 and 80.005 x 200 have two sides over 80.
    let mut v = Vec::new();
    for (i, &w) in [75.0, 80.0, 100.0, 80.0 + D].iter().enumerate() {
        let x = O + i as f64 * 125.0;
        v.push(rect(ndmy, x, O, x + w, O + 200.0));
    }
    write("DE.3.h1", v);

    // DE.4: the bound wall to wall and corner to corner.  14.14 µm each way is 19.997
    // from corner to corner and 14.145 is 20.004.
    let mut v = Vec::new();
    for (i, &(g, diagonal)) in [
        (20.0, false),
        (20.0 - D, false),
        (14.14, true),
        (14.145, true),
    ]
    .iter()
    .enumerate()
    {
        let x = O + i as f64 * 120.0;
        v.push(rect(ndmy, x, O, x + 25.0, O + 25.0));
        let (ox, oy) = if diagonal {
            (x + 25.0 + g, O + 25.0 + g)
        } else {
            (x + 25.0 + g, O)
        };
        v.push(rect(ndmy, ox, oy, ox + 25.0, oy + 25.0));
    }
    write("DE.4.h1", v);

    // DE.4 on the tile lines: a gap opening on x = 40, a notch open across x = 140, a
    // gap opening on y = 20, and the same gap at exactly 20 µm on x = 40 again.
    let g = 20.0 - D;
    write(
        "DE.4.h2",
        vec![
            rect(ndmy, 14.5, 10.0, 39.5, 35.0),
            rect(ndmy, 39.5 + g, 10.0, 64.5 + g, 35.0),
            rect(ndmy, 120.0, 10.0, 155.0 + g, 20.0),
            rect(ndmy, 120.0, 20.0, 135.0, 45.0),
            rect(ndmy, 135.0 + g, 20.0, 155.0 + g, 45.0),
            rect(ndmy, 220.0, 5.0, 245.0, 19.5),
            rect(ndmy, 220.0, 19.5 + g, 245.0, 44.5 + g),
            rect(ndmy, 14.5, 100.0, 39.5, 125.0),
            rect(ndmy, 59.5, 100.0, 84.5, 125.0),
        ],
    );
}
