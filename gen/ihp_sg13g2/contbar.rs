// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/contbar";

/// A contact bar (non-square Cont) with lower-left corner at `(x, y)`.
fn bar(l: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> GdsElement {
    rect(l, x, y, x + w, y + h)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    cntb_a(pdk);
    cntb_a1(pdk);
    cntb_b(pdk);
    cntb_b1(pdk);
    cntb_c(pdk);
    cntb_h1(pdk);
    cntb_g(pdk);
    cntb_j(pdk);
}

/// CntB.a — ContBar width (short side) must be exactly 0.16 µm.  A 0.16×0.40 bar is
/// clean; 0.15 wide fails min_dim and 0.17 wide fails max_dim.  Length 0.40 keeps
/// CntB.a1 quiet.
fn cntb_a(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 0.40),       // clean
        bar(c, o + 1.0, o, 0.15, 0.40), // too narrow → min_dim
        bar(c, o + 2.0, o, 0.17, 0.40), // too wide → max_dim
    ];
    write_gz(&format!("{DIR}/CntB.a.gds.gz"), library("TOP", elems));
}

/// CntB.a1 — ContBar length (long side) must be ≥ 0.34 µm.  A 0.16×0.34 bar is clean;
/// 0.16×0.33 fails.
fn cntb_a1(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 0.34),       // clean
        bar(c, o + 1.0, o, 0.16, 0.33), // too short → min_length
    ];
    write_gz(&format!("{DIR}/CntB.a1.gds.gz"), library("TOP", elems));
}

/// CntB.b — min. ContBar-to-ContBar space 0.28 µm.
fn cntb_b(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 0.40),
        bar(c, o + 0.16 + 0.27, o, 0.16, 0.40), // 0.27 gap → violation
    ];
    write_gz(&format!("{DIR}/CntB.b.gds.gz"), library("TOP", elems));
}

/// CntB.b1 — min. ContBar space 0.36 µm where the parallel run exceeds 5 µm.  Two
/// 6 µm-long bars 0.30 µm apart violate (run 6 > 5, gap 0.30 < 0.36); the 0.30 µm gap
/// clears the plain 0.28 µm CntB.b rule.
fn cntb_b1(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 6.0),
        bar(c, o + 0.16 + 0.30, o, 0.16, 6.0),
    ];
    write_gz(&format!("{DIR}/CntB.b1.gds.gz"), library("TOP", elems));
}

/// CntB.c — min. Activ enclosure of ContBar 0.07 µm.  One bar enclosed by 0.07 on all
/// sides (clean); one with a 0.06 left margin (violation).
fn cntb_c(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let a = layer(pdk, "Activ");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 0.40),
        rect(a, o - 0.07, o - 0.07, o + 0.16 + 0.07, o + 0.40 + 0.07), // 0.07 all round → clean
        bar(c, o + 2.0, o, 0.16, 0.40),
        rect(a, o + 2.0 - 0.06, o - 0.07, o + 2.0 + 0.16 + 0.07, o + 0.40 + 0.07), // 0.06 left → fail
    ];
    write_gz(&format!("{DIR}/CntB.c.gds.gz"), library("TOP", elems));
}

/// CntB.h1 — min. Metal1 enclosure of ContBar 0.05 µm.  One bar enclosed by 0.05
/// (clean); one with a 0.04 left margin (violation).  Metal1 covers both, so CntB.h
/// (coverage) stays quiet.
fn cntb_h1(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let m = layer(pdk, "Metal1");
    let o = OFFSET;
    let elems = vec![
        bar(c, o, o, 0.16, 0.40),
        rect(m, o - 0.05, o - 0.05, o + 0.16 + 0.05, o + 0.40 + 0.05),
        bar(c, o + 2.0, o, 0.16, 0.40),
        rect(m, o + 2.0 - 0.04, o - 0.05, o + 2.0 + 0.16 + 0.05, o + 0.40 + 0.05),
    ];
    write_gz(&format!("{DIR}/CntB.h1.gds.gz"), library("TOP", elems));
}

/// CntB.g — ContBar must be within Activ or GatPoly.  One bar on Activ, one on GatPoly
/// (clean); one over neither (coverage violation).
fn cntb_g(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let a = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(a, o, o, o + 1.0, o + 1.0),
        bar(c, o + 0.2, o + 0.2, 0.16, 0.40), // on Activ
        rect(gp, o + 2.0, o, o + 3.0, o + 1.0),
        bar(c, o + 2.2, o + 0.2, 0.16, 0.40), // on GatPoly
        bar(c, o + 5.0, o, 0.16, 0.40),       // on neither → violation
    ];
    write_gz(&format!("{DIR}/CntB.g.gds.gz"), library("TOP", elems));
}

/// CntB.j — a ContBar on GatPoly that is also over Activ is not allowed.  GatPoly and
/// Activ overlap; one bar sits in the overlap (violation), one on GatPoly only (clean).
fn cntb_j(pdk: &PdkConfig) {
    let c = layer(pdk, "Cont");
    let a = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(gp, o, o, o + 2.0, o + 1.0),
        rect(a, o + 0.8, o, o + 2.0, o + 1.0), // overlaps GatPoly for x ≥ o+0.8
        bar(c, o + 0.3, o + 0.3, 0.16, 0.40), // on GatPoly only → clean
        bar(c, o + 1.2, o + 0.3, 0.16, 0.40), // on GatPoly and over Activ → violation
    ];
    write_gz(&format!("{DIR}/CntB.j.gds.gz"), library("TOP", elems));
}
