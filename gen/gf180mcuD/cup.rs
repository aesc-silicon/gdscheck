// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cu pillar: a good and a bad pattern for every rule in the `cup` deck.
//!
//! Two rules at each of five levels, and the first here whose layer has to be *made*
//! before it can be violated: the deck measures metal that a PAD marker lands on and no
//! guard ring claims, so every fixture draws the pad over the metal.  Without it the
//! layer is empty and both halves pass for the wrong reason - which is the failure a
//! drawn pattern exists to rule out.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/cup";
const O: f64 = 10.0;
const D: f64 = 0.005;
/// Both rules ask the same 1 µm of a bond-pad metal.
const LIMIT: f64 = 1.0;

const LEVELS: &[&str] = &[
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

pub fn generate(pdk: &PdkConfig) {
    let pad = layer(pdk, "pad");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // The deck names both rules once per level, so one fixture per rule covers all five:
    // each carries the pad over metal at every level, and the level under test is the one
    // drawn short.  A rule id is shared across levels, so a per-level fixture would need
    // a name the deck cannot give it.
    for (id, narrow) in [("CUP.2", true), ("CUP.3", false)] {
        for (name, v) in [("good", LIMIT), ("bad", LIMIT - D)] {
            let mut elems = Vec::new();
            for (i, lname) in LEVELS.iter().enumerate() {
                let m = layer(pdk, lname);
                let y = O + i as f64 * 20.0;
                if narrow {
                    // CUP.2: a bar one micron wide, or a whisker under it.
                    elems.push(rect(m, O, y, O + v, y + 8.0));
                    elems.push(rect(pad, O - 0.5, y + 1.0, O + v + 0.5, y + 3.0));
                } else {
                    // CUP.3: a C whose opening is the notch under test.
                    elems.push(rect(m, O, y, O + 2.0, y + 4.0 + v));
                    elems.push(rect(m, O + 2.0, y, O + 6.0, y + 2.0));
                    elems.push(rect(m, O + 2.0, y + 2.0 + v, O + 6.0, y + 4.0 + v));
                    // The pad sits on the notch: the rule keeps only the measurements
                    // that meet the pad, so a notch the pad does not reach is nobody's.
                    elems.push(rect(pad, O + 1.5, y + 1.5, O + 6.5, y + 2.5 + v));
                }
            }
            write(&format!("{id}.{name}"), elems);
        }
    }
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
