// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, min_width_pattern, rect, space_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nbulay";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nbl_a(pdk);
    nbl_b(pdk);
    nbl_c(pdk);
    nbl_d(pdk);
    nbl_e(pdk);
    nbl_f(pdk);
}

/// NBL.b — min. nBuLay space or notch 1.50: a pair at 1.20 (< 1.50 → fires) and at
/// 1.50 exactly (clean).  Neither pair draws NBL.c: both gaps close in nBuLayMerged
/// (the close radius 0.75 bridges gaps up to and including exactly 1.50).
fn nbl_b(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let o = OFFSET;
    let elems = vec![
        rect(nb, o, o, o + 2.0, o + 2.0),
        rect(nb, o + 3.2, o, o + 5.2, o + 2.0), // gap 1.20 → NBL.b
        rect(nb, o + 10.0, o, o + 12.0, o + 2.0),
        rect(nb, o + 13.5, o, o + 15.5, o + 2.0), // gap 1.50 → clean for NBL.b
    ];
    write_gz(&format!("{DIR}/NBL.b.gds.gz"), library("TOP", elems));
}

/// NBL.c — same-net merge (gaps < 1.50 µm) then different-net space 3.20 µm.  Three nBuLay
/// pairs: gap 1.00 (merged → clean), gap 2.00 (in [1.50, 3.20) → NBL.c), gap 4.00 (clean).
fn nbl_c(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let o = OFFSET;
    let pair = |y: f64, gap: f64| {
        vec![
            rect(nb, o, y, o + 2.0, y + 2.0),
            rect(nb, o + 2.0 + gap, y, o + 4.0 + gap, y + 2.0),
        ]
    };
    let mut elems = pair(o, 1.00); // merged → clean
    elems.extend(pair(o + 6.0, 2.00)); // NBL.c
    elems.extend(pair(o + 12.0, 4.00)); // clean
    write_gz(&format!("{DIR}/NBL.c.gds.gz"), library("TOP", elems));
}

/// NBL.a — min. nBuLay width 1.00 µm.
fn nbl_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay");
    let elems = min_width_pattern(l, 1.0, 1.0, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.a.gds.gz"), library("TOP", elems));
}

/// NBL.d — min. nBuLay to NWell space 2.20 µm.
fn nbl_d(pdk: &PdkConfig) {
    let elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "NWell"), 3.0, 2.20, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.d.gds.gz"), library("TOP", elems));
}

/// NBL.e — min. nBuLay to N+Activ (Activ without pSD → NActiv) space 1.00 µm.
fn nbl_e(pdk: &PdkConfig) {
    let elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "Activ"), 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.e.gds.gz"), library("TOP", elems));
}

/// NBL.f — min. nBuLay to P+Activ (Activ ∩ pSD → PsdActiv) space 0.50 µm.  pSD covers the
/// whole pattern so every Activ neighbour reads as P+Activ rather than N+Activ.
fn nbl_f(pdk: &PdkConfig) {
    let mut elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "Activ"), 2.0, 0.50, OFFSET, SPACE_DELTA);
    let o = OFFSET;
    elems.push(rect(layer(pdk, "pSD"), o - 5.0, o - 5.0, o + 10.0, o + 10.0));
    write_gz(&format!("{DIR}/NBL.f.gds.gz"), library("TOP", elems));
}
