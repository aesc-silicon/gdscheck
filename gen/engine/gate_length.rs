// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the gate rules: the width of Outer between the walls it shares with the
//! boundary of Inner, or between the ones it does not, driven from a deck so that the
//! parameters arrive the way a PDK writes them.
//!
//! The tile is 20 µm with lines at its multiples.  Every expected count is read off the
//! drawing: a gate is both walls of one stretch, cut to it.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/gate_length";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A stripe 0.8 µm tall from x = 30 to 70 and a 2 µm piece of Inner cut out of it at
    // 58..60 - the way a channel mask is the poly over the active - across the tile
    // line at 40 from the stripe's midpoint.  The stripe's walls are shared with the
    // piece's boundary there, and 0.8 is under G.min's 1 µm.  G.min: 2, G.len: 0 (the
    // shared run is 2 µm, not more than 3), G.outside: 2 (no Via to be outside of), and
    // G.max_unshared: 2 - the stripe's ends are unshared walls 40 µm apart.
    write(
        "stripe_far",
        vec![
            rect(outer, 30.0, 30.0, 70.0, 30.8),
            rect(inner, 58.0, 30.0, 60.0, 30.8),
        ],
    );

    // The same stripe with a 4 µm piece: a run over G.len's 3 µm.  G.min: 2, G.len: 2,
    // and G.outside and G.max_unshared as above.
    write(
        "stripe_long_run",
        vec![
            rect(outer, 30.0, 30.0, 70.0, 30.8),
            rect(inner, 56.0, 30.0, 60.0, 30.8),
        ],
    );

    // A body 4 µm by 1 µm whose two ends Inner cuts: the shared walls are the ends, 4
    // apart, the unshared the top and bottom, 1 apart.  G.max_shared: 2 (4 > 3),
    // G.max_unshared: 0 (1 is not over 3), G.min: 0 (4 is not under 1).
    write(
        "ends_on_reference",
        vec![
            rect(outer, 30.0, 30.0, 34.0, 31.0),
            rect(inner, 25.0, 29.0, 30.0, 32.0),
            rect(inner, 34.0, 29.0, 39.0, 32.0),
        ],
    );

    // The same body 3.5 µm tall: now the unshared pair is over 3 as well.
    // G.max_shared: 2, G.max_unshared: 2.
    write(
        "ends_on_reference_tall",
        vec![
            rect(outer, 30.0, 30.0, 34.0, 33.5),
            rect(inner, 25.0, 29.0, 30.0, 34.5),
            rect(inner, 34.0, 29.0, 39.0, 34.5),
        ],
    );

    // A 10 µm stripe with the piece at 34..36 and Via over the right half of the piece
    // from 35 on: G.outside keeps the stretch 34..35 only, G.min all of it.  G.outside:
    // 2, G.min: 2, G.max_unshared: 2 (the stripe's ends, 10 apart).  With Via over the
    // whole piece, G.outside: 0, G.min: 2.
    write(
        "outside_half",
        vec![
            rect(outer, 30.0, 30.0, 40.0, 30.8),
            rect(inner, 34.0, 30.0, 36.0, 30.8),
            rect(via, 35.0, 29.0, 50.0, 32.0),
        ],
    );
    write(
        "outside_all",
        vec![
            rect(outer, 30.0, 30.0, 40.0, 30.8),
            rect(inner, 34.0, 30.0, 36.0, 30.8),
            rect(via, 33.0, 29.0, 50.0, 32.0),
        ],
    );
}
