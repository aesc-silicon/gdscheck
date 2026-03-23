// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    let layers = vec!["BiWind", "PEmWind", "BasPoly", "DeepCo", "PEmPoly", "EmPoly", "LDMOS", "PBiWind", "NoDRC", "Flash", "ColWind"];
    let mut elems = vec![];

    for (i, name) in layers.into_iter().enumerate() {
        elems.push(rect(layer(pdk, name), 0.0, 0.0, (i * 10) as f64 + 5.00, (i * 10) as f64 + 5.00))
    }

    write_gz(
        "tests/data/ihp-sg13g2/forbidden.gds.gz",
        library("TOP", elems),
    );
}
