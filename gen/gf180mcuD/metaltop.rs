// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Top metal: a good and a bad pattern for every rule in the `metaltop` deck.
//!
//! Four rules over one layer, which makes this the deck to read first if you are adding
//! patterns for another.  Each bad half carries one drawn violation of its own rule and
//! nothing else, so the harness can say plainly which rule a pattern is for; the good
//! half is the same geometry with the offending dimension put back to the limit.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/metaltop";
/// Off the origin, so a sign error reads as a wrong answer rather than a shape that
/// happens to straddle (0, 0).
const O: f64 = 10.0;

/// Half the manufacturing grid: enough to break a limit, too little to look like slop.
const D: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    let m = layer(pdk, "metal5_drawn");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // MT.1, min width 0.44.  Long enough that the bar clears MT.4's area on either half.
    for (name, w) in [("good", 0.44), ("bad", 0.44 - D)] {
        write(&format!("MT.1.{name}"), vec![rect(m, O, O, O + w, O + 3.0)]);
    }

    // MT.2a, min space 0.46 - and the notch, which the deck checks under the same id, so
    // both entries are exercised by the one pattern.
    for (name, g) in [("good", 0.46), ("bad", 0.46 - D)] {
        write(
            &format!("MT.2a.{name}"),
            vec![
                rect(m, O, O, O + 2.0, O + 2.0),
                rect(m, O + 2.0 + g, O, O + 4.0 + g, O + 2.0),
                // A C, its opening the same gap: the notch is a space within one shape.
                rect(m, O, O + 5.0, O + 2.0, O + 5.0 + g + 2.0),
                rect(m, O + 2.0, O + 5.0, O + 4.0, O + 6.0),
                rect(m, O + 2.0, O + 5.0 + g + 1.0, O + 4.0, O + 5.0 + g + 2.0),
            ],
        );
    }

    // MT.2b, space to *wide* top metal: 0.6 where the neighbour is over 10 µm across in
    // both directions.  The gap stays clear of MT.2a's 0.46 either way, so only the wide
    // rule can speak.
    for (name, g) in [("good", 0.6), ("bad", 0.6 - D)] {
        write(
            &format!("MT.2b.{name}"),
            vec![
                rect(m, O, O, O + 12.0, O + 12.0),
                rect(m, O + 12.0 + g, O, O + 13.0 + g, O + 1.0),
            ],
        );
    }

    // MT.4, min area 0.5625.  Both halves are square and well over the 0.44 width, so the
    // only dimension in play is the area: 0.75² clears it and 0.7² does not.
    for (name, s) in [("good", 0.75), ("bad", 0.70)] {
        write(&format!("MT.4.{name}"), vec![rect(m, O, O, O + s, O + s)]);
    }
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
