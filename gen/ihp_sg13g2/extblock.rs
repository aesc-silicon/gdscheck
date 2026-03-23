// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, space_pattern, min_width_pattern, notch_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/extblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    extb_a(pdk);
    extb_b_space(pdk);
    extb_b_notch(pdk);
    extb_c(pdk);
}

fn extb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "EXTBlock");
    let elems = min_width_pattern(l, 0.31, 0.31, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/EXTB.a.gds.gz"), library("TOP", elems));
}

fn extb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "EXTBlock");
    let elems = space_pattern(l, l, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/EXTB.b.space.gds.gz"), library("TOP", elems));
}

fn extb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "EXTBlock");
    // 0.5 µm arms stay above the 0.31 µm min width, so only the notch rule fires.
    let elems = notch_pattern(l, 0.5, 0.31, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/EXTB.b.notch.gds.gz"), library("TOP", elems));
}

fn extb_c(pdk: &PdkConfig) {
    // EXTB.c — min. EXTBlock space to pSD (0.31 µm).
    let l = layer(pdk, "EXTBlock");
    let psd = layer(pdk, "pSD");
    let elems = space_pattern(l, psd, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/EXTB.c.gds.gz"), library("TOP", elems));
}
