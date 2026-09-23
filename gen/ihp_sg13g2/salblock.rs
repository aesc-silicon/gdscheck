// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pwellblock::{P, enclosure_kit, space2_kit, width_kit};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    layer, library, min_width_pattern, notch_pattern, poly, rect, space_pattern, write_gz,
};
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

    hardening(pdk);
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

fn salblock(p: &P) {
    let deck = "salblock";
    let sb = p.sal;
    width_kit(p, deck, "Sal.a", sb, 0.42, 0.42);

    // Sal.c - SalBlock extension over Activ or GatPoly 0.20.  The kit with a 0.5 Activ.
    let act = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.activ, x0, y0, x1, y1)];
    enclosure_kit(
        p,
        deck,
        "Sal.c",
        0.2,
        &|pts| vec![poly(sb, pts)],
        &act,
        0.5,
        0.42,
    );

    // Sal.c.h5 - the extension and the union.  An Activ strip crossing a block that
    // extends 0.195 past its top edge (fires) and 0.2 (clean); a GatPoly strip crossing a
    // block with 0.195 (fires); a 0.5 Activ 0.195 from the block's bottom edge wholly
    // under a GatPoly that runs out of the block (the union's boundary there is the
    // poly's crossing, not the Activ's edge: clean); the same Activ with the poly inside
    // the block at 0.2 (the Activ's edge is the union's: fires).
    let e = vec![
        rect(p.activ, 2.0, 3.0, 6.0, 3.5),
        rect(sb, 3.0, 2.5, 4.0, 3.695),
        rect(p.activ, 8.0, 3.0, 12.0, 3.5),
        rect(sb, 9.0, 2.5, 10.0, 3.7),
        rect(p.gp, 14.0, 3.0, 18.0, 3.5),
        rect(sb, 15.0, 2.5, 16.0, 3.695),
        rect(sb, 19.8, 2.0, 21.2, 4.0),
        rect(p.activ, 20.2, 2.195, 20.8, 2.6),
        rect(p.gp, 20.1, 1.5, 20.9, 2.8),
        rect(sb, 22.8, 2.0, 24.2, 4.0),
        rect(p.activ, 23.2, 2.195, 23.8, 2.6),
        rect(p.gp, 23.2, 2.4, 23.8, 2.8),
    ];
    p.write(deck, "Sal.c.h5", e);

    // Sal.d - SalBlock space to unrelated Activ or GatPoly 0.20.  The kit with a 0.5
    // Activ; the abutting pair is a space of nothing.
    space2_kit(
        p,
        deck,
        "Sal.d",
        0.2,
        &|pts| vec![poly(sb, pts)],
        1.0,
        &act,
        0.5,
        true,
    );

    // Sal.d.h5 - GatPoly 0.195 from a block (fires); a U-shaped Activ whose left arm is
    // under the block (extended by 0.5) and whose right arm is 0.195 from the block's
    // right edge (the arm is not covered, so the space is measured: fires); a 0.5 Activ
    // 0.195 from a block that covers another Activ (fires).
    let e = vec![
        rect(sb, 2.0, 2.0, 3.0, 3.0),
        rect(p.gp, 3.195, 2.25, 3.695, 2.75),
        rect(sb, 6.0, 2.0, 7.0, 4.0),
        poly(
            p.activ,
            &[
                (6.5, 2.5),
                (6.5, 3.5),
                (7.195, 3.5),
                (7.195, 3.0),
                (7.7, 3.0),
                (7.7, 3.5),
                (8.2, 3.5),
                (8.2, 2.5),
            ],
        ),
        rect(sb, 11.0, 2.0, 12.0, 4.0),
        rect(p.activ, 11.2, 2.5, 11.8, 3.5),
        rect(p.activ, 12.195, 2.75, 12.695, 3.25),
    ];
    p.write(deck, "Sal.d.h5", e);

    // Sal.e - SalBlock space to Cont 0.20.  Every Cont sits at the right end of an Activ
    // strip that runs under the block (so the Activ is related and Sal.d stays quiet)
    // with Metal1 over it.
    // h1: 0.2 (clean), 0.195 (fires), corner to corner 0.14/0.14 (0.198: fires) and
    // 0.145/0.145 (0.205: clean), a Cont abutting the block (fires), a Cont inside the
    // block (a Schottky's bar: not a space, clean), a Cont crossing the block edge (its
    // outside part 0 away: fires), a 0.16 × 0.5 bar 0.195 away (fires).
    let cont = |x: f64, y: f64| {
        vec![
            rect(p.cont, x, y, x + 0.16, y + 0.16),
            rect(p.m1, x - 0.05, y - 0.05, x + 0.21, y + 0.21),
        ]
    };
    let mut e = vec![];
    for (k, (dx, dy)) in [
        (0.2, 0.0),
        (0.195, 0.0),
        (0.14, 0.14),
        (0.145, 0.145),
        (0.0, 0.0),
    ]
    .iter()
    .enumerate()
    {
        let x = 2.0 + k as f64 * 3.0;
        e.push(rect(sb, x, 2.0, x + 1.0, 3.0));
        if *dy > 0.0 {
            e.push(rect(p.activ, x + 0.5, 2.3, x + 1.5, 3.3 + dy));
            e.extend(cont(x + 1.0 + dx, 3.0 + dy));
        } else {
            e.push(rect(p.activ, x + 0.5, 2.3, x + 1.5, 2.7));
            e.extend(cont(x + 1.0 + dx, 2.42));
        }
    }
    e.push(rect(sb, 17.0, 2.0, 18.0, 3.0));
    e.push(rect(p.activ, 17.2, 2.2, 17.8, 2.8));
    e.extend(cont(17.42, 2.42));
    e.push(rect(sb, 20.0, 2.0, 21.0, 3.0));
    e.push(rect(p.activ, 20.5, 2.3, 21.5, 2.7));
    e.extend(cont(20.92, 2.42));
    e.push(rect(sb, 23.0, 2.0, 24.0, 3.0));
    e.push(rect(p.activ, 23.5, 2.3, 24.9, 2.7));
    e.push(rect(p.cont, 24.195, 2.42, 24.695, 2.58));
    e.push(rect(p.m1, 24.145, 2.37, 24.745, 2.63));
    p.write(deck, "Sal.e.h1", e);

    // Sal.e.h2 - tile lines: 0.195 gaps straddling x = 20 (three ways), 21, 40, 42, y = 20
    // at x = 30, a corner 0.14/0.14 on (60, 20), a 300 µm block at y = 60, (1000, 1000).
    let pair = |x: f64, y: f64, g: f64| {
        let mut e = vec![
            rect(sb, x - 1.0, y - 0.5, x, y + 0.5),
            rect(p.activ, x - 0.5, y - 0.2, x + g + 0.5, y + 0.2),
        ];
        e.extend(cont(x + g, y - 0.08));
        e
    };
    let mut e = vec![];
    e.extend(pair(19.9, 2.0, 0.195));
    e.extend(pair(20.0, 4.0, 0.195));
    e.extend(pair(20.0 - 0.195, 6.0, 0.195));
    e.extend(pair(20.9, 8.0, 0.195));
    e.extend(pair(39.9, 2.0, 0.195));
    e.extend(pair(41.9, 4.0, 0.195));
    e.push(rect(sb, 30.0, 18.9, 31.0, 19.9));
    e.push(rect(p.activ, 30.3, 19.4, 30.7, 20.5));
    e.extend(cont(30.42, 19.9 + 0.195));
    e.push(rect(sb, 59.0, 19.0, 60.0, 20.0));
    e.push(rect(p.activ, 59.5, 19.5, 60.5, 20.5));
    e.extend(cont(60.14, 20.14));
    e.push(rect(sb, 2.0, 60.0, 302.0, 61.0));
    e.push(rect(p.activ, 150.0, 60.5, 150.4, 61.6));
    e.extend(cont(150.12, 61.195));
    e.extend(pair(1000.0, 1000.0, 0.195));
    p.write(deck, "Sal.e.h2", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/salblock")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    salblock(&p);
}
