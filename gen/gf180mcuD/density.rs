// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Global density: a good and a bad pattern for every rule in the `density` deck.
//!
//! These rules are the one family here that cannot be tested a layer at a time.  Each is
//! a coverage fraction of the whole die, so a pattern that draws only the layer under
//! test leaves every *other* layer at zero and trips all of them at once.  Each fixture
//! therefore lays down a passing baseline on every layer the deck reads and moves one of
//! them, which is also how a real die fails these: everything is filled and one layer is
//! short.
//!
//! A pattern is a boundary box with horizontal stripes across it, so the measured density
//! is the summed stripe height over the box side and the number is readable off the
//! source.  The drawn layer carries all of it and the dummy layer stays empty; the deck
//! sums the pair, and which of the two the coverage comes from is not what these check.

use crate::helpers::{density_pattern, layer, library, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/density";
/// The die, and the denominator of every fraction here.
const SIZE: f64 = 100.0;
/// Clears every floor in the deck (the highest is 30%) and stays under DCF.1d's 70% cap.
const BASE: f64 = 40.0;

/// Every layer the deck measures, and the rule that floors it.
const LAYERS: &[(&str, &str)] = &[
    ("comp", "DCF.1b"),
    ("poly2_drawn", "PL.8"),
    ("metal1_drawn", "M1.4"),
    ("metal2_drawn", "M2.4"),
    ("metal3_drawn", "M3.4"),
    ("metal4_drawn", "M4.4"),
    ("metal5_drawn", "M5.4"),
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // One fixture per floor: everything at BASE, and the rule's own layer taken under.
    for &(lname, id) in LAYERS {
        for (name, pct) in [("good", BASE), ("bad", 10.0)] {
            write(pdk, &format!("{id}.{name}"), Some((lname, pct)));
        }
    }
    // MT.3 measures the top metal that M5.4 does, so it shares M5.4's geometry; the two
    // are one measurement under two names and no pattern can separate them.
    for (name, pct) in [("good", BASE), ("bad", 10.0)] {
        write(pdk, &format!("MT.3.{name}"), Some(("metal5_drawn", pct)));
    }
    // DCF.1d is the ceiling over the floor DCF.1b sets, so its bad half goes *up*.
    for (name, pct) in [("good", BASE), ("bad", 75.0)] {
        write(pdk, &format!("DCF.1d.{name}"), Some(("comp", pct)));
    }
}

/// The die with every layer at [`BASE`], except `moved`, which is set as given.
fn write(pdk: &PdkConfig, name: &str, moved: Option<(&str, f64)>) {
    // Every stripe starts at the bottom, so they overlap: density is measured per layer,
    // never between them, and stacking keeps each fraction readable on its own.
    let stripes: Vec<((i16, i16), f64, f64)> = LAYERS
        .iter()
        .map(|&(lname, _)| {
            let pct = match moved {
                Some((m, p)) if m == lname => p,
                _ => BASE,
            };
            (layer(pdk, lname), 0.0, SIZE * pct / 100.0)
        })
        .collect();
    write_gz(
        &format!("{DIR}/{name}.gds.gz"),
        library("TOP", density_pattern((0, 0), SIZE, &stripes)),
    );
}
