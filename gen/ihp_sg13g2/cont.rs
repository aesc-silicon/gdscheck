// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/cont";

/// A 0.16 µm contact (exact Cont width) with lower-left corner at `(x, y)`.
fn cont16(l: (i16, i16), x: f64, y: f64) -> GdsElement {
    rect(l, x, y, x + 0.16, y + 0.16)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    cnt_a(pdk);
    cnt_b(pdk);
    cnt_e(pdk);
    cnt_f(pdk);
    cnt_g(pdk);
    cnt_g1(pdk);
    cnt_g2(pdk);
    cnt_h(pdk);
    cnt_j(pdk);
}

/// Cnt.e — min. space of a gate contact (Cont on GatPoly) to Activ (0.14 µm).  Both
/// contacts sit on a shared GatPoly; one Activ is 0.14 µm away (clean), the other
/// 0.13 µm (violation).
fn cnt_e(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(gp, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(activ, 0.80, 0.0, 1.2, 1.0), // 0.80 - 0.66 = 0.14 → clean
        cont16(cont, 1.5, 0.42),
        rect(activ, 1.79, 0.0, 2.2, 1.0), // 1.79 - 1.66 = 0.13 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.e.gds.gz"), library("TOP", elems));
}

/// Cnt.f — min. space of a diffusion contact (Cont on Activ) to GatPoly (0.11 µm).
fn cnt_f(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(gp, 0.77, 0.0, 1.2, 1.0), // 0.77 - 0.66 = 0.11 → clean
        cont16(cont, 1.5, 0.42),
        rect(gp, 1.76, 0.0, 2.2, 1.0), // 1.76 - 1.66 = 0.10 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.f.gds.gz"), library("TOP", elems));
}

/// Cnt.g — Cont must be within Activ or GatPoly.  One contact on Activ, one on
/// GatPoly (both clean); one over neither → coverage violation.
fn cnt_g(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(activ, 0.0, 0.0, 1.0, 1.0),
        cont16(cont, 0.4, 0.4), // inside Activ
        rect(gp, 2.0, 0.0, 3.0, 1.0),
        cont16(cont, 2.4, 0.4),  // inside GatPoly
        cont16(cont, 5.0, 0.4),  // outside both → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g.gds.gz"), library("TOP", elems));
}

/// Cnt.g1 — min. pSD space to a contact on nSD-Activ (0.09 µm).  Contacts sit on an
/// n+ active (Activ ∩ nSD); a pSD region is 0.09 µm away (clean) / 0.08 µm (violation).
fn cnt_g1(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let psd = layer(pdk, "pSD");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        rect(nsd, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(psd, 0.75, 0.0, 1.2, 1.0), // 0.75 - 0.66 = 0.09 → clean
        cont16(cont, 1.5, 0.42),
        rect(psd, 1.74, 0.0, 2.2, 1.0), // 1.74 - 1.66 = 0.08 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g1.gds.gz"), library("TOP", elems));
}

/// Cnt.g2 — min. pSD overlap (enclosure) of a contact on pSD-Activ (0.09 µm).  Both
/// contacts sit on a p+ active (Activ ∩ pSD); one pSD encloses by 0.09 (clean), the
/// other by 0.08 (violation).
fn cnt_g2(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(psd, 0.41, 0.33, 0.75, 0.67), // 0.09 margin on the contact at 0.5..0.66 / 0.42..0.58
        cont16(cont, 1.5, 0.42),
        rect(psd, 1.42, 0.34, 1.74, 0.66), // 0.08 margin → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g2.gds.gz"), library("TOP", elems));
}

/// Cnt.h — Cont must be covered with Metal1.  One contact inside Metal1 (clean), one
/// uncovered → coverage violation.
fn cnt_h(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let m1 = layer(pdk, "Metal1");
    let elems = vec![
        rect(m1, 0.0, 0.0, 1.0, 1.0),
        cont16(cont, 0.4, 0.4), // covered
        cont16(cont, 3.0, 0.4), // uncovered → violation
    ];
    write_gz(&format!("{DIR}/Cnt.h.gds.gz"), library("TOP", elems));
}

/// Cnt.j — a contact on GatPoly that is also over Activ is not allowed.  GatPoly and
/// Activ overlap; one contact sits in the overlap (violation), one on GatPoly only
/// (clean).
fn cnt_j(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(gp, 0.0, 0.0, 2.0, 1.0),
        rect(activ, 0.8, 0.0, 2.0, 1.0), // overlaps GatPoly for x ∈ [0.8, 2.0]
        cont16(cont, 0.3, 0.42), // on GatPoly, not over Activ → clean
        cont16(cont, 1.2, 0.42), // on GatPoly and over Activ → violation
    ];
    write_gz(&format!("{DIR}/Cnt.j.gds.gz"), library("TOP", elems));
}

/// `Cnt.a` runs `exact_width` on `ContSquare` (square contacts only; bars are checked
/// by the CntBar width rules).  Square contacts must be exactly 0.16 µm: a 0.155 and a
/// 0.165 µm square each fail on all four walls (8 total).  A seal-covered off-size
/// square confirms `ContSquare` (built on `ContNoSealring`) drops the seal ring.
fn cnt_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Cont");
    let edgeseal = layer(pdk, "EdgeSeal");
    let o = OFFSET;
    let sq = |x: f64, side: f64| rect(l, x, o, x + side, o + side);
    let elems = vec![
        sq(o, 0.16),        // exact → clean
        sq(o + 1.0, 0.155), // too small → 4 walls
        sq(o + 2.0, 0.165), // too large → 4 walls
        // seal-covered off-size square: removed by ContNoSealring → not in ContSquare
        sq(o + 10.0, 0.155),
        rect(edgeseal, o + 9.0, o - 1.0, o + 12.0, o + 2.0),
    ];
    write_gz(&format!("{DIR}/Cnt.a.gds.gz"), library("TOP", elems));
}

fn cnt_b(pdk: &PdkConfig) {
    let l = layer(pdk, "Cont");
    let elems = space_pattern(l, l, 0.16, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Cnt.b.gds.gz"), library("TOP", elems));
}
