// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, notch_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/pwellblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pwb_a(pdk);
    pwb_b_space(pdk);
    pwb_b_notch(pdk);
    pwb_c(pdk);
    pwb_e(pdk);
    pwb_e1(pdk);
    pwb_f(pdk);
    pwb_f1(pdk);
}

/// An implanted Activ footprint (Activ ∩ `imp`) at `(x, y)` — N+Activ with nSD or
/// P+Activ with pSD.
fn impl_activ(activ: (i16, i16), imp: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![rect(activ, x, y, x + w, y + h), rect(imp, x, y, x + w, y + h)]
}

/// PWell:block-to-(implanted Activ "in PWell") spacing fixture: a clean pair at exactly
/// `value` and a violating pair at `value - 0.01`.  `tgo` (if set) covers the Activ.
fn pwb_space_to_activ(
    pdk: &PdkConfig, name: &str, imp_name: &str, value: f64, tgo: bool,
) {
    let blk = layer(pdk, "PWell.block");
    let activ = layer(pdk, "Activ");
    let imp = layer(pdk, imp_name);
    let o = OFFSET;
    let mut elems = vec![rect(blk, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + value;
    elems.extend(impl_activ(activ, imp, x1, o, 0.5, 0.5)); // gap = value → clean
    elems.push(rect(blk, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + value - 0.01;
    elems.extend(impl_activ(activ, imp, x2, o, 0.5, 0.5)); // gap = value-0.01 → violation
    if tgo {
        let t = layer(pdk, "ThickGateOx");
        elems.push(rect(t, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
        elems.push(rect(t, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    }
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// PWB.e — min. PWell:block space to N+Activ (in PWell), not in ThickGateOx, 0.31 µm.
fn pwb_e(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.e", "nSD", 0.31, false);
}

/// PWB.e1 — same, inside ThickGateOx, 0.62 µm.
fn pwb_e1(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.e1", "nSD", 0.62, true);
}

/// PWB.f — min. PWell:block space to P+Activ (in PWell), not in ThickGateOx, 0.24 µm.
fn pwb_f(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.f", "pSD", 0.24, false);
}

/// PWB.f1 — same, inside ThickGateOx, 0.62 µm.
fn pwb_f1(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.f1", "pSD", 0.62, true);
}

/// PWB.a — min. PWell:block width 0.62 µm.
fn pwb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = min_width_pattern(l, 0.62, 0.62, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.a.gds.gz"), library("TOP", elems));
}

/// PWB.b — min. PWell:block space 0.62 µm.  1 µm shapes clear the 0.62 µm min width.
fn pwb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = space_pattern(l, l, 1.0, 0.62, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.b.space.gds.gz"), library("TOP", elems));
}

/// PWB.b — min. PWell:block notch 0.62 µm.
fn pwb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = notch_pattern(l, 1.0, 0.62, 2.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.b.notch.gds.gz"), library("TOP", elems));
}

/// PWB.c — min. PWell:block space to NWell 0.62 µm.  (Overlap with NWell is allowed —
/// PWB.d — and min_space skips overlapping pairs, so only true gaps are measured.)
fn pwb_c(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 1.0, 0.62, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.c.gds.gz"), library("TOP", elems));
}
