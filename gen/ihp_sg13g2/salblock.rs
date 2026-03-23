// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, notch_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/salblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    sal_a(pdk);
    sal_b_space(pdk);
    sal_b_notch(pdk);
    sal_c(pdk);
    sal_d(pdk);
    sal_e(pdk);
}

/// Sal.c — SalBlock must extend ≥ 0.20 µm past the long edges of the Activ/GatPoly it
/// covers (the ends run out the short edges, which are exempt).  A block extending 0.20
/// on both long edges is clean; one extending only 0.19 on the top edge violates.
fn sal_c(pdk: &PdkConfig) {
    let sb = layer(pdk, "SalBlock");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let elems = vec![
        // clean: long Activ (3 µm × 0.3 µm), block across it extending 0.20 both sides
        rect(activ, o, o + 0.5, o + 3.0, o + 0.8),
        rect(sb, o + 1.0, o + 0.3, o + 1.5, o + 1.0),
        // fail: block extends only 0.19 past the top long edge
        rect(activ, o + 5.0, o + 0.5, o + 8.0, o + 0.8),
        rect(sb, o + 6.0, o + 0.3, o + 6.5, o + 0.99),
    ];
    write_gz(&format!("{DIR}/Sal.c.gds.gz"), library("TOP", elems));
}

/// Sal.a — min. SalBlock width 0.42 µm.
fn sal_a(pdk: &PdkConfig) {
    let l = layer(pdk, "SalBlock");
    let elems = min_width_pattern(l, 0.42, 0.42, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Sal.a.gds.gz"), library("TOP", elems));
}

/// Sal.b — min. SalBlock space 0.42 µm.  Shapes 1 µm wide clear the 0.42 µm min width.
fn sal_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "SalBlock");
    let elems = space_pattern(l, l, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Sal.b.space.gds.gz"), library("TOP", elems));
}

/// Sal.b — min. SalBlock notch 0.42 µm.  0.5 µm arms stay above the min width.
fn sal_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "SalBlock");
    let elems = notch_pattern(l, 0.5, 0.42, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Sal.b.notch.gds.gz"), library("TOP", elems));
}

/// Sal.d — min. SalBlock space to unrelated Activ or GatPoly (0.20 µm).  The Activ
/// neighbours are separate from the block, so min_space (which skips overlapping, i.e.
/// "related", pairs) measures them via the ActivOrGatPoly union.
fn sal_d(pdk: &PdkConfig) {
    let sb = layer(pdk, "SalBlock");
    let activ = layer(pdk, "Activ");
    let elems = space_pattern(sb, activ, 0.5, 0.20, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Sal.d.gds.gz"), library("TOP", elems));
}

/// Sal.e — min. SalBlock space to Cont (0.20 µm).
fn sal_e(pdk: &PdkConfig) {
    let sb = layer(pdk, "SalBlock");
    let cont = layer(pdk, "Cont");
    let elems = space_pattern(sb, cont, 0.5, 0.20, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Sal.e.gds.gz"), library("TOP", elems));
}
