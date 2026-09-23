// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pwellblock::{P, enclosure_kit, space2_kit, width_kit};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    enclosure_pattern, layer, library, min_width_pattern, notch_pattern, poly, rect, space_pattern,
    write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nbulayblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nblb_a(pdk);
    nblb_b_space(pdk);
    nblb_b_notch(pdk);
    nblb_c(pdk);
    nblb_d(pdk);

    hardening(pdk);
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

fn nbulayblock(p: &P) {
    let deck = "nbulayblock";
    let b = p.nblb;
    width_kit(p, deck, "NBLB.a", b, 1.5, 1.0);

    // NBLB.c - nBuLay enclosure of nBuLay:block 1.00.  The kit with a 2 × 2 block.
    let blk = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(b, x0, y0, x1, y1)];
    enclosure_kit(
        p,
        deck,
        "NBLB.c",
        1.0,
        &|pts| vec![poly(p.nbl, pts)],
        &blk,
        2.0,
        3.2,
    );

    // NBLB.c.h5 - a block with no drawn nBuLay.  A bare 2 × 2 block at (2, 2) (nothing
    // encloses it and nothing is near it: clean); a block 2.5 inside every edge of a
    // 10 × 10 well (the well's generated nBuLay, inset by 1.0, encloses it by 1.5:
    // clean); a block 1.5 inside a 10 × 10 well (enclosed by 0.5 of generated nBuLay:
    // fires); a block over a whole 6 × 6 well and 1.0 beyond it (no nBuLay is left to
    // enclose anything: clean).
    let e = vec![
        rect(b, 2.0, 2.0, 4.0, 4.0),
        rect(p.nw, 8.0, 2.0, 18.0, 12.0),
        rect(b, 10.5, 4.5, 15.5, 9.5),
        rect(p.nw, 22.0, 2.0, 32.0, 12.0),
        rect(b, 23.5, 3.5, 30.5, 10.5),
        rect(p.nw, 36.0, 2.0, 42.0, 8.0),
        rect(b, 35.0, 1.0, 43.0, 9.0),
    ];
    p.write(deck, "NBLB.c.h5", e);

    // NBLB.d - nBuLay:block space to unrelated nBuLay 1.50.  The kit with a 2 × 2
    // nBuLay as the fixed box and the block as the free shape; the abutting pair is in h5.
    let nbl = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.nbl, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "NBLB.d",
        1.5,
        &|pts| vec![poly(b, pts)],
        2.5,
        &nbl,
        2.0,
        false,
    );

    // NBLB.d.h5 - the relation.  A block inside an nBuLay with 1.0 margins (NBLB.c's,
    // clean for d); a block crossing the nBuLay edge (NBLB.c, not d); a block abutting the
    // nBuLay from outside (unrelated, 0 away: fires); a block 0.4 outside a 6 × 6 well (the
    // generated nBuLay, inset by 1.0, is 1.4 away: fires) and one 0.5 outside (1.5: clean).
    let e = vec![
        rect(p.nbl, 2.0, 2.0, 6.0, 6.0),
        rect(b, 3.0, 3.0, 5.0, 5.0),
        rect(p.nbl, 9.0, 2.0, 13.0, 6.0),
        rect(b, 12.0, 3.0, 14.0, 5.0),
        rect(p.nbl, 17.0, 2.0, 21.0, 6.0),
        rect(b, 21.0, 3.0, 23.0, 5.0),
        rect(p.nw, 2.0, 10.0, 8.0, 16.0),
        rect(b, 8.4, 12.0, 10.4, 14.0),
        rect(p.nw, 14.0, 10.0, 20.0, 16.0),
        rect(b, 20.5, 12.0, 22.5, 14.0),
    ];
    p.write(deck, "NBLB.d.h5", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/nbulayblock")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    nbulayblock(&p);
}
