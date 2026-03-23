// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, space_pattern, min_width_pattern, notch_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nbulayblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nblb_a(pdk);
    nblb_b_space(pdk);
    nblb_b_notch(pdk);
    nblb_c(pdk);
    nblb_d(pdk);
}

/// NBLB.d — min. space from nBuLay:block to (a different) nBuLay 1.50 µm.
fn nblb_d(pdk: &PdkConfig) {
    let block = layer(pdk, "nBuLay.block");
    let nbulay = layer(pdk, "nBuLay");
    let elems = space_pattern(block, nbulay, 2.0, 1.50, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBLB.d.gds.gz"), library("TOP", elems));
}

/// NBLB.a — min. nBuLay:block width 1.50 µm.
fn nblb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay.block");
    let elems = min_width_pattern(l, 1.50, 1.50, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBLB.a.gds.gz"), library("TOP", elems));
}

/// NBLB.b — min. nBuLay:block space 1.00 µm.  2 µm shapes clear the 1.50 µm min width.
fn nblb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay.block");
    let elems = space_pattern(l, l, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBLB.b.space.gds.gz"), library("TOP", elems));
}

/// NBLB.b — min. nBuLay:block notch 1.00 µm.  2 µm arms stay above the min width.
fn nblb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay.block");
    let elems = notch_pattern(l, 2.0, 1.00, 3.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBLB.b.notch.gds.gz"), library("TOP", elems));
}

/// NBLB.c — min. nBuLay enclosure of nBuLay:block 1.00 µm.  The blocked region (2 µm,
/// above the 1.50 µm min width) must sit 1.00 µm inside the nBuLay.
fn nblb_c(pdk: &PdkConfig) {
    let nbl = layer(pdk, "nBuLay");
    let blk = layer(pdk, "nBuLay.block");
    let elems = enclosure_pattern(nbl, blk, 1.00, 2.0, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBLB.c.gds.gz"), library("TOP", elems));
}
