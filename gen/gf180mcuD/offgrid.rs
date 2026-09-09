// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Off-grid vertices: one good and one bad fixture per rule in the `offgrid` deck.

use super::{GRID, OFFSET, octagon, rules_of};
use crate::helpers::{library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/offgrid";

/// Off-grid shift. 3 nm is not a multiple of the 5 nm grid, and is small enough that
/// nothing about the shape except its grid alignment changes.
const OFF: f64 = 0.003;

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    for (id, l) in rules_of(pdk, "offgrid") {
        // Good: a plain rectangle and an octagon, every vertex a multiple of the grid.
        // The octagon is the load-bearing half — a 45° edge lands its vertices on the
        // grid just as an orthogonal one does, and the check must not care.
        let good = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            octagon(l, OFFSET + 3.0, OFFSET, 2.0, 0.5),
        ];
        write_gz(&format!("{DIR}/{id}.good.gds.gz"), library("TOP", good));

        // Bad: the same rectangle with its right edge pushed 3 nm off the grid.
        let x1 = OFFSET + 3.0;
        let bad = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            rect(l, x1, OFFSET, x1 + 1.0 + OFF, OFFSET + 1.0),
        ];
        write_gz(&format!("{DIR}/{id}.bad.gds.gz"), library("TOP", bad));
    }

    // A grid that is not the rule's grid would make every fixture here meaningless.
    assert_eq!(GRID, 0.005, "fixtures assume the 5 nm grid");
}
