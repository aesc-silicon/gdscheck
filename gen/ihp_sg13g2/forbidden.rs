// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::helpers::{layer, library, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/forbidden";

pub fn generate(pdk: &PdkConfig) {
    let layers = vec![
        "BiWind", "PEmWind", "BasPoly", "DeepCo", "PEmPoly", "EmPoly", "LDMOS", "PBiWind", "NoDRC",
        "Flash", "ColWind",
    ];
    let mut elems = vec![];

    for (i, name) in layers.into_iter().enumerate() {
        elems.push(rect(
            layer(pdk, name),
            0.0,
            0.0,
            (i * 10) as f64 + 5.00,
            (i * 10) as f64 + 5.00,
        ))
    }

    write_gz(&format!("{DIR}/forbidden.gds.gz"), library("TOP", elems));

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Forbidden shapes across tile lines, overlapping, far away, in arrays.

fn hardening(pdk: &PdkConfig) {
    // h1: a BiWind across x = 20 and 40 (one shape, one report), one at
    // (1000, 1000), a BiWind overlapping a PEmWind (one each), a NoDRC 0.005 square;
    // h3: fifty LDMOS boxes as a GdsArrayRef.
    let bi = layer(pdk, "BiWind");
    let pem = layer(pdk, "PEmWind");
    let nodrc = layer(pdk, "NoDRC");
    let ldmos = layer(pdk, "LDMOS");
    write_gz(
        &format!("{DIR}/forbidden.h1.gds.gz"),
        library(
            "TOP",
            vec![
                rect(bi, 15.0, 2.0, 45.0, 3.0),
                rect(bi, 1000.0, 1000.0, 1001.0, 1001.0),
                rect(bi, 2.0, 6.0, 4.0, 8.0),
                rect(pem, 3.0, 7.0, 5.0, 9.0),
                rect(nodrc, 8.0, 6.0, 8.005, 6.005),
            ],
        ),
    );
    let cell = vec![rect(ldmos, 0.2, 0.2, 1.2, 1.2)];
    write_gz(
        &format!("{DIR}/forbidden.h3.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );
}
