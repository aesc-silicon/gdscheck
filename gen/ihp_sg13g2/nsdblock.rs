// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, notch_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nsdblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    nsdb_a(pdk);
    nsdb_b_space(pdk);
    nsdb_b_notch(pdk);
    nsdb_c(pdk);
    nsdb_e(pdk);
}

/// nSDB.a — min. nSD:block width 0.31 µm.
fn nsdb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = min_width_pattern(l, 0.31, 0.31, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.a.gds.gz"), library("TOP", elems));
}

/// nSDB.b — min. nSD:block space 0.31 µm.  1 µm shapes clear the 0.31 µm min width.
fn nsdb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = space_pattern(l, l, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.b.space.gds.gz"), library("TOP", elems));
}

/// nSDB.b — min. nSD:block notch 0.31 µm.  0.5 µm arms stay above the min width.
fn nsdb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = notch_pattern(l, 0.5, 0.31, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.b.notch.gds.gz"), library("TOP", elems));
}

/// nSDB.c — min. nSD:block space to pSD 0.31 µm.  (Overlap with pSD is allowed —
/// nSDB.d — and min_space skips overlapping pairs, so only true gaps are measured.)
fn nsdb_c(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let psd = layer(pdk, "pSD");
    let elems = space_pattern(l, psd, 0.5, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.c.gds.gz"), library("TOP", elems));
}

/// nSDB.e — nSD:block and Cont must not overlap.  One nSD:block sits over a Cont
/// (violation); a second is clear of its Cont (clean).
fn nsdb_e(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 0.5, o + 0.5),
        rect(cont, o + 0.2, o + 0.2, o + 0.36, o + 0.36), // inside the block → overlap → violation
        rect(l, o + 2.0, o, o + 2.5, o + 0.5),
        rect(cont, o + 3.0, o, o + 3.16, o + 0.16), // clear of the block → clean
    ];
    write_gz(&format!("{DIR}/nSDB.e.gds.gz"), library("TOP", elems));
}
