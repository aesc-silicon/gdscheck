// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Angles off the 45° lattice: one good and one bad fixture per rule in the `acute` deck.

use super::{OFFSET, octagon, rules_of};
use crate::helpers::{library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/acute";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    for (id, l) in rules_of(pdk, "acute") {
        // Good: a rectangle and an octagon. Between them they use every legal
        // orientation — 0°, 45°, 90° and 135° — so a check that allowed only right
        // angles would fail here rather than passing quietly.
        let good = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            octagon(l, OFFSET + 3.0, OFFSET, 2.0, 0.5),
        ];
        write_gz(&format!("{DIR}/{id}.good.gds.gz"), library("TOP", good));

        // Bad: a right triangle with legs 2 and 1, so its hypotenuse runs at 26.57° —
        // far enough off 0° and 45° that no tolerance excuses it, and a slope a real
        // layout could plausibly contain. Its vertices stay on the grid, so this fixture
        // is clean under the off-grid deck and the two cannot be confused.
        let (x, y) = (OFFSET + 3.0, OFFSET);
        let bad = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            poly(l, &[(x, y), (x + 2.0, y), (x, y + 1.0)]),
        ];
        write_gz(&format!("{DIR}/{id}.bad.gds.gz"), library("TOP", bad));
    }
}
