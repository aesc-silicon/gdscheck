// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::pwellblock::{Boxes, Free, G, P, diag_over, diag_under, ring, space2_kit};
use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

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

    hardening(pdk);
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
        rect(
            a,
            o + 2.0 - 0.06,
            o - 0.07,
            o + 2.0 + 0.16 + 0.07,
            o + 0.40 + 0.07,
        ), // 0.06 left → fail
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
        rect(
            m,
            o + 2.0 - 0.04,
            o - 0.05,
            o + 2.0 + 0.16 + 0.05,
            o + 0.40 + 0.05,
        ),
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
        bar(c, o + 0.3, o + 0.3, 0.16, 0.40),  // on GatPoly only → clean
        bar(c, o + 1.2, o + 0.3, 0.16, 0.40),  // on GatPoly and over Activ → violation
    ];
    write_gz(&format!("{DIR}/CntB.j.gds.gz"), library("TOP", elems));
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

fn contbar(p: &P) {
    let deck = "contbar";
    let c = p.cont;
    // A Cont bar `(x, y)-(x + w, y + h)` on Activ with 0.07 of it around, and Metal1
    // with 0.05.
    let bar = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            rect(c, x, y, x + w, y + h),
            rect(p.m1, x - 0.05, y - 0.05, x + w + 0.05, y + h + 0.05),
            rect(p.activ, x - 0.07, y - 0.07, x + w + 0.07, y + h + 0.07),
        ]
    };
    // A Cont polygon with Activ and Metal1 around its bounding box.
    let cpoly = |pts: &[(f64, f64)]| {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        vec![
            poly(c, pts),
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };

    // CntB.a/CntB.a1.h1 - the bound.  0.16 × 0.34 and 0.34 × 0.16 bars are clean; 0.155
    // and 0.165 wide bars (both orientations) are CntB.a; 0.16 × 0.335 and 0.16 × 0.165
    // (both orientations) are CntB.a1; a 0.16 × 0.5 bar with a 0.005 nick in its side is
    // 0.155 wide there (CntB.a); a 0.17 square is a square, Cont's, not a bar (clean
    // here); an L and a T of 0.16 arms 0.5 long are 0.16 wide everywhere (clean); a bar
    // drawn as two 0.08 halves, as two overlapping 0.16 × 0.3 boxes and as a 0.16 square
    // abutting a 0.16 × 0.5 bar end to end are one clean bar each; two 0.16 × 0.5 bars
    // overlapping sideways by 0.06 are 0.26 wide (CntB.a); a 45° bar of width
    // 0.115·√2 = 0.163 is over 0.16 (CntB.a).
    let mut e = vec![];
    let mut x = 2.0;
    for (w, h) in [
        (0.16, 0.34),
        (0.34, 0.16),
        (0.155, 0.5),
        (0.5, 0.155),
        (0.165, 0.5),
        (0.5, 0.165),
        (0.16, 0.335),
        (0.335, 0.16),
        (0.16, 0.165),
        (0.165, 0.16),
    ] {
        e.extend(bar(x, 2.0, w, h));
        x += 1.0;
    }
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.16, 2.0),
        (x + 0.16, 2.2),
        (x + 0.155, 2.2),
        (x + 0.155, 2.3),
        (x + 0.16, 2.3),
        (x + 0.16, 2.5),
        (x, 2.5),
    ]));
    x += 1.0;
    e.extend(bar(x, 2.0, 0.17, 0.17));
    x += 1.0;
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.5, 2.0),
        (x + 0.5, 2.16),
        (x + 0.16, 2.16),
        (x + 0.16, 2.5),
        (x, 2.5),
    ]));
    x += 1.0;
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.66, 2.0),
        (x + 0.66, 2.16),
        (x + 0.41, 2.16),
        (x + 0.41, 2.66),
        (x + 0.25, 2.66),
        (x + 0.25, 2.16),
        (x, 2.16),
    ]));
    x += 1.0;
    let cover = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };
    e.extend(cover(x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x, 2.0, x + 0.08, 2.5));
    e.push(rect(c, x + 0.08, 2.0, x + 0.16, 2.5));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x, 2.0, x + 0.16, 2.3));
    e.push(rect(c, x, 2.2, x + 0.16, 2.5));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.16, 2.66));
    e.push(rect(c, x, 2.0, x + 0.16, 2.16));
    e.push(rect(c, x, 2.16, x + 0.16, 2.66));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.26, 2.5));
    e.push(rect(c, x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x + 0.1, 2.0, x + 0.26, 2.5));
    x += 1.0;
    e.extend(cpoly(&[
        (x + 0.115, 2.0),
        (x + 0.615, 2.5),
        (x + 0.5, 2.615),
        (x, 2.115),
    ]));
    p.write(deck, "CntB.a.h1", e);

    // CntB.a.h2 - tile lines.  0.155 × 0.5 bars (CntB.a) straddling x = 20, ending on
    // 20, starting on 20, straddling 21, 40, 42, one straddling y = 20, one at (1000,
    // 1000) and a 0.155 × 300 bar at y = 30; 0.16 × 0.335 bars (CntB.a1) the same eight
    // ways; a 0.16 × 5 bar straddling x = 20 is clean.
    let mut e = vec![];
    for (k, (w, h)) in [(0.5, 0.155), (0.335, 0.16)].iter().enumerate() {
        let y = 2.0 + k as f64 * 6.0;
        e.extend(bar(19.9, y, *w, *h));
        e.extend(bar(20.0 - w, y + 1.0, *w, *h));
        e.extend(bar(20.0, y + 2.0, *w, *h));
        e.extend(bar(20.9, y + 3.0, *w, *h));
        e.extend(bar(39.9, y, *w, *h));
        e.extend(bar(41.9, y, *w, *h));
        e.extend(bar(10.0 + k as f64 * 3.0, 19.9, *h, *w));
        e.extend(bar(1000.0, 1000.0 + k as f64 * 3.0, *w, *h));
    }
    e.extend(bar(2.0, 30.0, 300.0, 0.155));
    e.extend(bar(17.5, 40.0, 5.0, 0.16));
    p.write(deck, "CntB.a.h2", e);

    // CntB.a.h3/h4 - fifty cells of a 0.155 × 0.5 bar (CntB.a) and a 0.16 × 0.335 bar
    // (CntB.a1), flat and as an array.
    let mut cell = bar(0.2, 0.2, 0.5, 0.155);
    cell.extend(bar(0.2, 1.2, 0.335, 0.16));

    // CntB.b.h1 - space 0.28.  Side by side at 0.28 (clean) and 0.275 (fires); end to
    // end at 0.275 (fires); a bar's end 0.275 from another's side (fires); corner to
    // corner 0.195/0.195 (0.276: fires) and 0.2/0.2 (0.283: clean); 0.1 in x with 0.28
    // in y (clean); two 0.16 × 5.5 bars 0.275 apart (CntB.b and, the run being over 5,
    // CntB.b1).
    let mut e = vec![];
    let mut x = 2.0;
    for g in [0.28, 0.275] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(bar(x + 0.16 + g, 2.0, 0.16, 0.5));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(bar(x, 2.5 + 0.275, 0.16, 0.5));
    x += 1.5;
    e.extend(bar(x, 2.0, 0.5, 0.16));
    e.extend(bar(x + 0.17, 2.16 + 0.275, 0.16, 0.5));
    x += 1.5;
    for d in [0.195, 0.2] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(bar(x + 0.16 + d, 2.5 + d, 0.16, 0.5));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(bar(x + 0.16 + 0.1, 2.5 + 0.28, 0.16, 0.5));
    x += 1.5;
    e.extend(bar(x, 2.0, 0.16, 5.5));
    e.extend(bar(x + 0.16 + 0.275, 2.0, 0.16, 5.5));
    p.write(deck, "CntB.b.h1", e);

    // CntB.b.h2 - tile lines.  0.275 gaps between 0.16 × 0.5 bars: end to end across
    // x = 20 (gap over the line, starting on it, ending on it), across 21, 40, 42; side by
    // side across y = 20 at x = 30; a corner pair 0.195/0.195 on (60, 20); two 300 µm bars
    // 0.275 apart at y = 60 (CntB.b and CntB.b1); a pair at (1000, 1000).
    let ee = |x: f64, y: f64| {
        let mut e = bar(x - 0.5, y, 0.5, 0.16);
        e.extend(bar(x + 0.275, y, 0.5, 0.16));
        e
    };
    let mut e = vec![];
    e.extend(ee(19.9, 2.0));
    e.extend(ee(20.0, 3.0));
    e.extend(ee(20.0 - 0.275, 4.0));
    e.extend(ee(20.9, 5.0));
    e.extend(ee(39.9, 2.0));
    e.extend(ee(41.9, 3.0));
    e.extend(bar(30.0, 19.9 - 0.16, 0.5, 0.16));
    e.extend(bar(30.0, 19.9 + 0.275, 0.5, 0.16));
    e.extend(bar(60.0 - 0.16, 20.0 - 0.5, 0.16, 0.5));
    e.extend(bar(60.0 + 0.195, 20.0 + 0.195, 0.16, 0.5));
    e.extend(bar(2.0, 60.0, 300.0, 0.16));
    e.extend(bar(2.0, 60.0 + 0.16 + 0.275, 300.0, 0.16));
    e.extend(ee(1000.0, 1000.0));
    p.write(deck, "CntB.b.h2", e);

    // CntB.b.h3/h4 - fifty 0.275 pairs, flat and as an array.
    let mut cell = bar(0.2, 0.2, 0.16, 0.5);
    cell.extend(bar(0.2 + 0.16 + 0.275, 0.2, 0.16, 0.5));

    // CntB.b1.h1 - space 0.36 with a common run over 5 µm.  Two 0.16 × 6 bars at 0.355
    // (fires) and 0.36 (clean); 0.16 × 5.0 bars at 0.355 (a run of exactly 5: clean) and
    // 0.16 × 5.005 (fires); 6 bars offset so the run is 4.5 (clean) and 5.005 (fires); a
    // 6.5 bar facing three collinear 1.9 bars 0.28 apart (each pair's common run is
    // 1.9: clean); a 6.5 bar facing a bar 0.355 away whose upper half jogs 0.05 closer
    // (one stepped wall running alongside for 6.0 under the value: fires; the 0.11
    // chord across the jog is a CntB.a width); five 6 bars at 0.355 (four pairs); a 6
    // bar whose end is 0.355 from a 6 bar's side (a run of 0.16: clean).
    let mut e = vec![];
    let mut x = 2.0;
    for (len, g) in [(6.0, 0.355), (6.0, 0.36), (5.0, 0.355), (5.005, 0.355)] {
        e.extend(bar(x, 2.0, 0.16, len));
        e.extend(bar(x + 0.16 + g, 2.0, 0.16, len));
        x += 1.5;
    }
    for off in [1.5, 0.995] {
        e.extend(bar(x, 2.0, 0.16, 6.0));
        e.extend(bar(x + 0.16 + 0.355, 2.0 + off, 0.16, 6.0));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 6.5));
    for k in 0..3 {
        e.extend(bar(x + 0.16 + 0.355, 2.2 + k as f64 * 2.18, 0.16, 1.9));
    }
    x += 1.5;
    e.extend(bar(x, 2.0, 0.16, 6.5));
    let jx = x + 0.16 + 0.355;
    e.extend(cpoly(&[
        (jx, 2.2),
        (jx + 0.16, 2.2),
        (jx + 0.16, 5.2),
        (jx + 0.11, 5.2),
        (jx + 0.11, 8.2),
        (jx - 0.05, 8.2),
        (jx - 0.05, 5.2),
        (jx, 5.2),
    ]));
    x += 1.5;
    for k in 0..5 {
        e.extend(bar(x + k as f64 * (0.16 + 0.355), 2.0, 0.16, 6.0));
    }
    x += 4.0;
    e.extend(bar(x, 2.0, 0.16, 6.0));
    e.extend(bar(x + 0.16 + 0.355, 4.0, 6.0, 0.16));
    p.write(deck, "CntB.b1.h1", e);

    // CntB.b1.h2 - tile lines.  Pairs of 0.16 × 6 bars 0.355 apart, horizontal, running
    // from x = 17 to 23 (the run cut by x = 20 and 21), from 14 to 20 (ending on the
    // line), from 20 to 26 (starting on it), from 37 to 43 (across 40 and 42), from 4 to
    // 10 (across 7); vertical from y = 17 to 23 at x = 30; a 5.005 pair from 17.5 to
    // 22.505; two 300 µm bars 0.355 apart at y = 60; a pair at (1000, 1000).
    let hp = |x: f64, y: f64, len: f64| {
        let mut e = bar(x, y, len, 0.16);
        e.extend(bar(x, y + 0.16 + 0.355, len, 0.16));
        e
    };
    let mut e = vec![];
    e.extend(hp(17.0, 2.0, 6.0));
    e.extend(hp(14.0, 3.0, 6.0));
    e.extend(hp(20.0, 4.0, 6.0));
    e.extend(hp(37.0, 2.0, 6.0));
    e.extend(hp(4.0, 3.0, 6.0));
    e.extend(bar(30.0, 17.0, 0.16, 6.0));
    e.extend(bar(30.0 + 0.16 + 0.355, 17.0, 0.16, 6.0));
    e.extend(hp(17.5, 5.0, 5.005));
    e.extend(hp(2.0, 60.0, 300.0));
    e.extend(hp(1000.0, 1000.0, 6.0));
    p.write(deck, "CntB.b1.h2", e);

    // CntB.b2.h1 - space to Cont 0.22.  A 0.16 square beside a 0.16 × 0.5 bar at 0.22
    // (clean) and 0.215 (fires); at the bar's end 0.215 (fires); corner to corner
    // 0.15/0.15 (0.212: fires) and 0.16/0.16 (0.226: clean); 0.1 in x and 0.22 in y
    // (clean).
    let sq = |x: f64, y: f64| bar(x, y, 0.16, 0.16);
    let mut e = vec![];
    let mut x = 2.0;
    for g in [0.22, 0.215] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(sq(x + 0.16 + g, 2.1));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(sq(x, 2.5 + 0.215));
    x += 1.5;
    for d in [0.15, 0.16] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(sq(x + 0.16 + d, 2.5 + d));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(sq(x + 0.16 + 0.1, 2.5 + 0.22));
    p.write(deck, "CntB.b2.h1", e);

    // CntB.b2.h2 - tile lines: 0.215 gaps (a square right of a bar) straddling x = 20
    // three ways, 21, 40, 42; a square above a bar across y = 20 at x = 30; a corner
    // 0.15/0.15 on (60, 20); a square 0.215 from a 300 µm bar at y = 60; (1000, 1000).
    let bs = |x: f64, y: f64| {
        let mut e = bar(x - 0.5, y, 0.5, 0.16);
        e.extend(sq(x + 0.215, y));
        e
    };
    let mut e = vec![];
    e.extend(bs(19.9, 2.0));
    e.extend(bs(20.0, 3.0));
    e.extend(bs(20.0 - 0.215, 4.0));
    e.extend(bs(20.9, 5.0));
    e.extend(bs(39.9, 2.0));
    e.extend(bs(41.9, 3.0));
    e.extend(bar(30.0, 19.9 - 0.16, 0.5, 0.16));
    e.extend(sq(30.1, 19.9 + 0.215));
    e.extend(bar(60.0 - 0.16, 20.0 - 0.5, 0.16, 0.5));
    e.extend(sq(60.0 + 0.15, 20.0 + 0.15));
    e.extend(bar(2.0, 60.0, 300.0, 0.16));
    e.extend(sq(150.0, 60.0 + 0.16 + 0.215));
    e.extend(bs(1000.0, 1000.0));
    p.write(deck, "CntB.b2.h2", e);

    // CntB.c/CntB.d - Activ / GatPoly enclosure of a 0.16 × 0.5 bar 0.07: the kit, the
    // inner box being the bar with Metal1 over it.
    let bare = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c, x0, y0, x1, y1),
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
        ]
    };
    let in_activ = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let mut e = bare(x0, y0, x1, y1);
        e.push(rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07));
        e
    };
    let in_poly = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let mut e = bare(x0, y0, x1, y1);
        e.push(rect(p.gp, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07));
        e
    };
    bar_enclosure_kit(
        p,
        deck,
        "CntB.c",
        0.07,
        &|pts| vec![poly(p.activ, pts)],
        &bare,
    );
    bar_enclosure_kit(p, deck, "CntB.d", 0.07, &|pts| vec![poly(p.gp, pts)], &bare);
    // CntB.g2 - pSD overlap of a bar on P+Activ 0.09: the kit, the bar on Activ.
    bar_enclosure_kit(
        p,
        deck,
        "CntB.g2",
        0.09,
        &|pts| vec![poly(p.psd, pts)],
        &in_activ,
    );
    // CntB.h1 - Metal1 enclosure of a bar 0.05: the kit, the bar on Activ with no Metal1
    // of its own.
    let on_activ_bare = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c, x0, y0, x1, y1),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };
    bar_enclosure_kit(
        p,
        deck,
        "CntB.h1",
        0.05,
        &|pts| vec![poly(p.m1, pts)],
        &on_activ_bare,
    );

    // CntB.g2.h5 - the pSD edge through the middle of a bar on Activ: the covered half is
    // on P+Activ enclosed by 0 (CntB.g2), the bare half is on N+Activ 0 from pSD (CntB.g1).
    let mut e = in_activ(2.0, 2.0, 2.16, 2.5);
    e.push(rect(p.psd, 1.5, 1.5, 2.08, 3.0));
    p.write(deck, "CntB.g2.h5", e);

    // CntB.e - a bar on GatPoly to Activ 0.14: the kit, the bar on poly as the fixed box
    // (0.16 × 0.5), Activ the free shape; the abutting pair is a space of nothing.
    let gbar = |x0: f64, y0: f64, _x1: f64, _y1: f64| in_poly(x0, y0, x0 + 0.16, y0 + 0.5);
    space2_kit(
        p,
        deck,
        "CntB.e",
        0.14,
        &|pts| vec![poly(p.activ, pts)],
        1.0,
        &gbar,
        0.16,
        true,
    );
    // CntB.f - a bar on Activ to GatPoly 0.11.
    let abar = |x0: f64, y0: f64, _x1: f64, _y1: f64| in_activ(x0, y0, x0 + 0.16, y0 + 0.5);
    space2_kit(
        p,
        deck,
        "CntB.f",
        0.11,
        &|pts| vec![poly(p.gp, pts)],
        1.0,
        &abar,
        0.16,
        true,
    );
    // CntB.g1 - pSD to a bar on nSD-Activ 0.09: the bar on plain Activ (N+ by default).
    space2_kit(
        p,
        deck,
        "CntB.g1",
        0.09,
        &|pts| vec![poly(p.psd, pts)],
        1.0,
        &abar,
        0.16,
        true,
    );

    // CntB.g1.h5 - what is nSD-Activ.  pSD 0.085 from a bar on plain Activ (fires), on
    // Activ under drawn nSD (fires), on Activ under nSD:block (clean), on P+Activ (CntB.g2's
    // at 0.09, and the bar is enclosed by 0.2: clean).
    let mut e = vec![];
    for (k, imp) in [(0, None), (1, Some(p.nsd)), (2, Some(p.nsdb))] {
        let x = 2.0 + k as f64 * 2.0;
        e.extend(in_activ(x, 2.0, x + 0.16, 2.5));
        e.push(rect(p.psd, x + 0.16 + 0.085, 1.9, x + 1.0, 2.6));
        if let Some(l) = imp {
            e.push(rect(l, x - 0.1, 1.9, x + 0.2, 2.6));
        }
    }
    e.extend(in_activ(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.psd, 7.8, 1.8, 9.0, 2.7));
    p.write(deck, "CntB.g1.h5", e);

    // CntB.g.h1 - a bar must be within Activ or GatPoly.  A bare bar (fires); a bar 0.05
    // past the Activ edge (CntB.g and CntB.c); a bar half out of its GatPoly (CntB.g and
    // CntB.d); a bar straddling the seam of an abutting Activ and GatPoly (covered by
    // their union: no CntB.g; enclosed by neither: CntB.c and CntB.d; its Activ half 0 from
    // poly: CntB.f; its poly half 0 from Activ: CntB.e); a bar in an Activ ring's hole
    // (fires); a bar abutting Activ from outside (fires); a bar on Activ under GatPoly
    // (CntB.j, not g); one at (1000, 1000) (fires).
    let mut e = bare(2.0, 2.0, 2.16, 2.5);
    e.extend(bare(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.activ, 3.93, 1.93, 4.11, 2.57));
    e.extend(bare(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.gp, 5.93, 1.93, 6.08, 2.57));
    e.extend(bare(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.activ, 7.8, 1.8, 8.08, 2.7));
    e.push(rect(p.gp, 8.08, 1.8, 8.4, 2.7));
    e.extend(bare(10.0, 2.0, 10.16, 2.5));
    e.extend(ring(p.activ, 9.5, 1.5, 10.66, 3.0, 9.8, 1.8, 10.36, 2.7));
    e.extend(bare(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.activ, 12.16, 1.8, 12.7, 2.7));
    e.extend(in_poly(14.0, 2.0, 14.16, 2.5));
    e.push(rect(p.activ, 13.8, 1.8, 14.4, 2.7));
    e.extend(bare(1000.0, 1000.0, 1000.16, 1000.5));
    p.write(deck, "CntB.g.h1", e);

    // CntB.g.h2 - bare bars straddling x = 20, ending on 20, starting on 20, straddling
    // 21, 40, 42 and y = 20 (fires each); a bar straddling x = 20 whose Activ ends on the
    // line (CntB.g and CntB.c); a bar ending on x = 20 with its Activ ending there (within,
    // enclosed by 0: CntB.c).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        e.extend(bare(*x, 2.0 + k as f64, x + 0.5, 2.16 + k as f64));
    }
    e.extend(bare(30.0, 19.9, 30.16, 20.4));
    e.extend(bare(19.9, 10.0, 20.4, 10.16));
    e.push(rect(p.activ, 19.5, 9.5, 20.0, 10.7));
    e.extend(bare(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.activ, 19.2, 11.5, 20.0, 12.7));
    p.write(deck, "CntB.g.h2", e);

    // CntB.h.h1 - a bar must be covered with Metal1.  No Metal1 (fires); a 0.005 strip
    // uncovered (fires); half covered (CntB.h, and CntB.h1 for the covered half enclosed by
    // 0); in a Metal1 ring's hole (fires); Metal1 abutting from outside (fires); Metal1
    // coincident with the bar (covers; CntB.h1 at 0); Metal1 with 0.05 all round (clean);
    // Metal1 as two abutting halves and as two overlapping boxes (clean); a bare bar at
    // (1000, 1000) (fires).
    let mut e = on_activ_bare(2.0, 2.0, 2.16, 2.5);
    e.extend(on_activ_bare(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.m1, 3.95, 1.95, 4.155, 2.55));
    e.extend(on_activ_bare(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.m1, 5.95, 1.95, 6.08, 2.55));
    e.extend(on_activ_bare(8.0, 2.0, 8.16, 2.5));
    e.extend(ring(p.m1, 7.5, 1.5, 8.66, 3.0, 7.9, 1.9, 8.26, 2.6));
    e.extend(on_activ_bare(10.0, 2.0, 10.16, 2.5));
    e.push(rect(p.m1, 10.16, 1.9, 10.6, 2.6));
    e.extend(on_activ_bare(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.m1, 12.0, 2.0, 12.16, 2.5));
    e.extend(on_activ_bare(14.0, 2.0, 14.16, 2.5));
    e.push(rect(p.m1, 13.95, 1.95, 14.21, 2.55));
    e.extend(on_activ_bare(16.0, 2.0, 16.16, 2.5));
    e.push(rect(p.m1, 15.95, 1.95, 16.08, 2.55));
    e.push(rect(p.m1, 16.08, 1.95, 16.21, 2.55));
    e.extend(on_activ_bare(18.0, 2.0, 18.16, 2.5));
    e.push(rect(p.m1, 17.95, 1.95, 18.12, 2.55));
    e.push(rect(p.m1, 18.04, 1.95, 18.21, 2.55));
    e.extend(on_activ_bare(1000.0, 1000.0, 1000.16, 1000.5));
    p.write(deck, "CntB.h.h1", e);

    // CntB.h.h2 - bare bars on Activ straddling x = 20, ending on 20, starting on 20,
    // straddling 21, 40, 42 and y = 20 (fires each); a bar straddling x = 20 whose Metal1
    // ends on the line (CntB.h, and CntB.h1 for the covered part); Metal1 ending where the
    // bar ends on x = 20 covers (CntB.h1 at 0); two Metal1 boxes meeting on x = 20 under a
    // bar cover it (clean).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        e.extend(on_activ_bare(*x, 2.0 + k as f64, x + 0.5, 2.16 + k as f64));
    }
    e.extend(on_activ_bare(30.0, 19.9, 30.16, 20.4));
    e.extend(on_activ_bare(19.9, 10.0, 20.4, 10.16));
    e.push(rect(p.m1, 19.5, 9.9, 20.0, 10.26));
    e.extend(on_activ_bare(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.m1, 19.2, 11.9, 20.0, 12.26));
    e.extend(on_activ_bare(19.7, 14.0, 20.2, 14.16));
    e.push(rect(p.m1, 19.5, 13.9, 20.0, 14.26));
    e.push(rect(p.m1, 20.0, 13.9, 20.5, 14.26));
    p.write(deck, "CntB.h.h2", e);

    // CntB.j.h1 - a bar on GatPoly over Activ is not allowed.  A bar in a gate (fires); a
    // bar on poly overlapping Activ by a 0.005 strip (fires; the strip is a bar on Activ
    // enclosed by 0: CntB.c) and by a 0.005 × 0.005 corner (the same); a bar on poly over
    // two 0.05 Activ fingers (two overlap pieces: two CntB.j, and CntB.c for each); a bar
    // on poly abutting Activ (not over it: no CntB.j, but 0 from it: CntB.e); a bar on
    // poly 0.14 from Activ (clean); one in a gate at (1000, 1000) (fires).
    let mut e = in_poly(2.0, 2.0, 2.16, 2.5);
    e.push(rect(p.activ, 1.8, 1.8, 2.4, 2.7));
    e.extend(in_poly(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.activ, 4.155, 1.8, 4.6, 2.7));
    e.extend(in_poly(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.activ, 6.155, 2.495, 6.6, 2.9));
    e.extend(in_poly(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.activ, 7.8, 2.1, 8.4, 2.15));
    e.push(rect(p.activ, 7.8, 2.3, 8.4, 2.35));
    e.extend(in_poly(10.0, 2.0, 10.16, 2.5));
    e.push(rect(p.activ, 10.16, 1.8, 10.6, 2.7));
    e.extend(in_poly(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.activ, 12.3, 1.8, 12.7, 2.7));
    e.extend(in_poly(1000.0, 1000.0, 1000.16, 1000.5));
    e.push(rect(p.activ, 999.8, 999.8, 1000.4, 1000.7));
    p.write(deck, "CntB.j.h1", e);

    // CntB.j.h2 - gates straddling x = 20, ending on 20, starting on 20, straddling 21,
    // 40, 42 and y = 20 (fires each); a bar on poly whose Activ begins on x = 20 under its
    // right half (CntB.j, and CntB.c for the half on Activ); one whose Activ begins where
    // the bar ends on x = 20 abuts it (no CntB.j, CntB.e at 0).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        let y = 2.0 + k as f64;
        e.extend(in_poly(*x, y, x + 0.5, y + 0.16));
        e.push(rect(p.activ, x - 0.2, y - 0.2, x + 0.7, y + 0.36));
    }
    e.extend(in_poly(30.0, 19.9, 30.16, 20.4));
    e.push(rect(p.activ, 29.8, 19.7, 30.36, 20.6));
    e.extend(in_poly(19.75, 10.0, 20.25, 10.16));
    e.push(rect(p.activ, 20.0, 9.8, 20.5, 10.36));
    e.extend(in_poly(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.activ, 20.0, 11.8, 20.5, 12.36));
    p.write(deck, "CntB.j.h2", e);

    // CntB.j.h3/h4 - fifty gate bars, flat and as an array.
    let mut cell = in_poly(0.2, 0.2, 0.7, 0.36);
    cell.push(rect(p.activ, 0.0, 0.0, 0.9, 0.56));
}

/// The enclosure kit for a 0.16 × 0.5 bar: `enclosure_kit` with the inner box a bar.
fn bar_enclosure_kit(p: &P, deck: &str, rule: &str, v: f64, outer: &Free<'_>, inner: &Boxes<'_>) {
    let d = v - G;
    let m = v + 0.5;
    let gap = 1.0;
    let (bw, bh) = (0.16, 0.5);
    let av = diag_under(v);
    let ac = diag_over(v);
    let obox =
        |x0: f64, y0: f64, x1: f64, y1: f64| outer(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
    let pair = |x: f64, y: f64, l: f64, r: f64, b: f64, t: f64| {
        let mut e = obox(x, y, x + l + bw + r, y + b + bh + t);
        e.extend(inner(x + l, y + b, x + l + bw, y + b + bh));
        e
    };

    // h1.  Margins v all round (clean); d on the left, right, bottom, top (one each); d
    // all round (one); the outer's corner chamfered av·√2 from the bar's corner with both
    // axis margins m (fires) and ac·√2 (clean); the bar half out of the outer (fires);
    // the bar's right edge on the outer's (fires).
    let y = 2.0;
    let mut x = 2.0;
    let mut e = pair(x, y, v, v, v, v);
    x += bw + 2.0 * v + gap;
    for (l, r, b, t) in [
        (d, v, v, v),
        (v, d, v, v),
        (v, v, d, v),
        (v, v, v, d),
        (d, d, d, d),
    ] {
        e.extend(pair(x, y, l, r, b, t));
        x += bw + l + r + gap;
    }
    for a in [av, ac] {
        let (ox, oy) = (bw + 2.0 * m, bh + 2.0 * m);
        let c = 2.0 * m - 2.0 * a;
        e.extend(outer(&[
            (x, y),
            (x + ox, y),
            (x + ox, y + oy - c),
            (x + ox - c, y + oy),
            (x, y + oy),
        ]));
        e.extend(inner(x + m, y + m, x + m + bw, y + m + bh));
        x += ox + gap;
    }
    let ox = bw + 2.0 * m;
    e.extend(obox(x, y, x + ox, y + bh + 2.0 * m));
    e.extend(inner(
        x + ox - bw / 2.0,
        y + m,
        x + ox + bw / 2.0,
        y + m + bh,
    ));
    x += ox + bw + gap;
    e.extend(pair(x, y, v, 0.0, v, v));
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - tile lines, as `enclosure_kit`: d right margins straddling x = 20 (three ways),
    // 21, 40, 42; a d top margin across y = 20 at x = 30; a corner on (60, 20); a bar d
    // from the top of a 300 µm strip at y = 60; (1000, 1000); a v margin across x = 20
    // is clean.
    let row = bh + 2.0 * v + gap;
    let mut e = vec![];
    e.extend(pair(19.9 - v - bw, 2.0, v, d, v, v));
    e.extend(pair(20.0 - d - v - bw, 2.0 + row, v, d, v, v));
    e.extend(pair(20.0 - v - bw, 2.0 + 2.0 * row, v, d, v, v));
    e.extend(pair(20.9 - v - bw, 2.0 + 3.0 * row, v, d, v, v));
    e.extend(pair(39.9 - v - bw, 2.0, v, d, v, v));
    e.extend(pair(41.9 - v - bw, 2.0 + row, v, d, v, v));
    e.extend(pair(30.0, 19.9 - v - bh, v, v, v, d));
    e.extend(pair(60.0 - v - bw, 20.0 - v - bh, v, d, v, d));
    e.extend(obox(2.0, 60.0, 302.0, 60.0 + bh + 2.0 * v));
    e.extend(inner(150.0, 60.0 + v + G, 150.0 + bw, 60.0 + v + bh + G));
    e.extend(pair(1000.0, 1000.0, v, d, v, v));
    e.extend(pair(19.9 - v - bw, 2.0 + 4.0 * row, v, v, v, v));
    p.write(deck, &format!("{rule}.h2"), e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/contbar")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    contbar(&p);
}
