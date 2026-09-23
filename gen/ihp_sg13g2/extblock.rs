// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pwellblock::{P, space2_kit, width_kit};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    layer, library, min_width_pattern, notch_pattern, poly, rect, space_pattern, write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/extblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    extb_a(pdk);
    extb_b_space(pdk);
    extb_b_notch(pdk);
    extb_c(pdk);

    hardening(pdk);
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

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the block decks: section 5.2 (PWell:block, PWB.a-PWB.f1), 5.3
// (nBuLay, NBL.a-NBL.f), 5.4 (nBuLay:block, NBLB.a-NBLB.d), 5.12 (EXTBlock, EXTB.a-c),
// 5.13 (SalBlock, Sal.a-e) and 5.15 (ContBar, CntB.a-CntB.j) of the SG13G2 layout rules,
// with section 4.2's derived layers (N+Activ, P+Activ, PWell, the generated nBuLay).
// Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//
// The width rules of the five block layers share a kit (`width_kit`), the two-layer
// space rules another (`space2_kit`); their plain space rules are read on the engine's
// patterns (`gen/engine/space.rs`), and the conditions of each rule - which Activ is N+,
// what "in PWell" or "unrelated" means, the generated nBuLay - are drawn rule by rule
// below.

fn extblock(p: &P) {
    let deck = "extblock";
    let l = p.extb;
    width_kit(p, deck, "EXTB.a", l, 0.31, 0.31);
    // EXTB.c - EXTBlock space to pSD 0.31; pSD alone is pSD.
    let psd = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.psd, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "EXTB.c",
        0.31,
        &|pts| vec![poly(l, pts)],
        1.0,
        &psd,
        0.5,
        true,
    );
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/extblock")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    extblock(&p);
}
