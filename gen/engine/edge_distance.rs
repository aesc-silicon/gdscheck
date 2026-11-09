// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for an enclosure between two edge layers
//! ([`edge_distance`](gdscheck::checks::edge_distance)) along walls far longer than the
//! limit: a 60 µm Outer across three tile lines round an Inner 0.01 µm in, at the bound,
//! and with one short stretch 0.005 µm in, once on each side.  A wall that long is read
//! whole by every query rather than filed cell by cell, and must still find the short
//! one beside it.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/edge_distance";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };
    let wire = || rect(outer, 10.0, 30.0, 70.0, 31.0);
    let core = || rect(inner, 10.01, 30.01, 69.99, 30.99);

    // Inner 0.01 in all round: ED.enc 0.
    write("long_clean", vec![wire(), core()]);
    // Inner's top rising to 0.005 under Outer's long top wall from x 65 to 65.5: 1.
    write(
        "long_outer",
        vec![
            wire(),
            poly(
                inner,
                &[
                    (10.01, 30.01),
                    (69.99, 30.01),
                    (69.99, 30.99),
                    (65.5, 30.99),
                    (65.5, 30.995),
                    (65.0, 30.995),
                    (65.0, 30.99),
                    (10.01, 30.99),
                ],
            ),
        ],
    );
    // Outer's top dipping to 0.005 over Inner's long top wall from x 65 to 65.5: 1.
    write(
        "long_inner",
        vec![
            poly(
                outer,
                &[
                    (10.0, 30.0),
                    (70.0, 30.0),
                    (70.0, 31.0),
                    (65.5, 31.0),
                    (65.5, 30.995),
                    (65.0, 30.995),
                    (65.0, 31.0),
                    (10.0, 31.0),
                ],
            ),
            core(),
        ],
    );
}
