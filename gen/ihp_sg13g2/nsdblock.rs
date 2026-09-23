// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::psd::{L, ring, space_suite, sq, width_suite, write};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, diamond, layer, library, min_width_pattern, notch_pattern, rect, space_pattern,
    write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nsdblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    nsdb_a(pdk);
    nsdb_b_space(pdk);
    nsdb_b_notch(pdk);
    nsdb_c(pdk);
    nsdb_e(pdk);

    hardening(pdk);
}

/// nSDB.a — min. nSD:block width 0.31 µm.
fn nsdb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = min_width_pattern(l, 0.31, 0.31, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.a.gds.gz"), library("TOP", elems));
}

/// nSDB.b — min. nSD:block space 0.31 µm.  1 µm shapes clear the 0.31 µm min width.
fn nsdb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = space_pattern(l, l, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.b.space.gds.gz"), library("TOP", elems));
}

/// nSDB.b — min. nSD:block notch 0.31 µm.  0.5 µm arms stay above the min width.
fn nsdb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let elems = notch_pattern(l, 0.5, 0.31, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.b.notch.gds.gz"), library("TOP", elems));
}

/// nSDB.c — min. nSD:block space to pSD 0.31 µm.  (Overlap with pSD is allowed —
/// nSDB.d — and min_space skips overlapping pairs, so only true gaps are measured.)
fn nsdb_c(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let psd = layer(pdk, "pSD");
    let elems = space_pattern(l, psd, 0.5, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/nSDB.c.gds.gz"), library("TOP", elems));
}

/// nSDB.e — nSD:block and Cont must not overlap.  One nSD:block sits over a Cont
/// (violation); a second is clear of its Cont (clean).
fn nsdb_e(pdk: &PdkConfig) {
    let l = layer(pdk, "nSD.block");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 0.5, o + 0.5),
        rect(cont, o + 0.2, o + 0.2, o + 0.36, o + 0.36), // inside the block → overlap → violation
        rect(l, o + 2.0, o, o + 2.5, o + 0.5),
        rect(cont, o + 3.0, o, o + 3.16, o + 0.16), // clear of the block → clean
    ];
    write_gz(&format!("{DIR}/nSDB.e.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the implant decks: section 5.7 ThickGateOxide (TGO.a-TGO.f),
// section 5.10 pSD (pSD.a-pSD.n) and section 5.11 nSD:block (nSDB.a-nSDB.e) of the
// SG13G2 layout rules, with section 4.2's derivations (N+/P+ Activ by drawn nSD/pSD or
// by default, NFET/PFET, the ties).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.

/// nSDB.c, "Min. nSD:block space to pSD 0.31", with nSDB.d, "Overlap of nSD:block and pSD
/// is allowed".
fn nsdb_c_h(l: &L) {
    // h1: 0.31 clean; 0.305 in x and y and 0.304 corner to corner fire; a pSD abutting the
    // block and one overlapping it are allowed; a pSD overlapping one arm of a U-shaped
    // block and 0.2 from its other arm fires; a pSD in a block ring's hole 0.305 from the
    // inner wall fires; 0.305 at (1000, 1000) fires.
    let mut e = vec![
        sq(l.nsdb, 2.0, 2.0, 1.0),
        sq(l.psd, 3.31, 2.0, 1.0),
        sq(l.nsdb, 6.0, 2.0, 1.0),
        sq(l.psd, 7.305, 2.0, 1.0),
        sq(l.nsdb, 10.0, 2.0, 1.0),
        sq(l.psd, 10.0, 3.305, 1.0),
        sq(l.nsdb, 14.0, 2.0, 1.0),
        sq(l.psd, 15.215, 3.215, 1.0),
        sq(l.nsdb, 2.0, 6.0, 1.0),
        sq(l.psd, 3.0, 6.0, 1.0),
        sq(l.nsdb, 6.0, 6.0, 1.0),
        sq(l.psd, 6.5, 6.0, 1.0),
        rect(l.nsdb, 10.0, 6.0, 10.5, 8.0),
        rect(l.nsdb, 11.0, 6.0, 11.5, 8.0),
        rect(l.nsdb, 10.0, 6.0, 11.5, 6.5),
        rect(l.psd, 10.2, 7.0, 10.8, 7.8),
    ];
    e.extend(ring(l.nsdb, 14.0, 5.0, 18.0, 9.0, 15.0, 6.0, 17.0, 8.0));
    e.push(sq(l.psd, 15.305, 6.5, 1.0));
    e.push(sq(l.nsdb, 1000.0, 1000.0, 1.0));
    e.push(sq(l.psd, 1001.305, 1000.0, 1.0));
    write("nsdblock", "nSDB.c.h1", e);

    // h2: a block's chamfered corner 0.304 from a pSD's corner (the walls farther) and a
    // diamond pSD's corner 0.305 from a block's wall fire.
    write(
        "nsdblock",
        "nSDB.c.h2",
        vec![
            chamfered_tr(l.nsdb, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.psd, 4.165, 3.765, 1.0),
            rect(l.nsdb, 7.0, 2.0, 8.195, 4.0),
            diamond(l.psd, 9.0, 3.0, 0.5),
        ],
    );
}

/// nSDB.e, "Min. nSD:block space to Cont 0.00 (nSD:block and Cont do not overlap)".
fn nsdb_e_h(l: &L) {
    // h1: a Cont inside a block, one half over its edge and one 0.005 over it fire; one
    // abutting the edge, one touching at a corner and one in a block ring's hole are no
    // overlap; a Cont half over a block's edge on x = 20, one in a block straddling 21, one
    // in a block at (1000, 1000) and a 0.16 × 0.5 bar in a block fire.
    let mut e = vec![
        sq(l.nsdb, 2.0, 2.0, 1.0),
        sq(l.cont, 2.4, 2.4, 0.16),
        sq(l.nsdb, 5.0, 2.0, 1.0),
        sq(l.cont, 5.92, 2.4, 0.16),
        sq(l.nsdb, 8.0, 2.0, 1.0),
        sq(l.cont, 9.0, 2.4, 0.16),
        sq(l.nsdb, 11.0, 2.0, 1.0),
        sq(l.cont, 12.0, 3.0, 0.16),
    ];
    e.extend(ring(l.nsdb, 14.0, 1.0, 17.0, 4.0, 15.0, 2.0, 16.0, 3.0));
    e.push(sq(l.cont, 15.42, 2.42, 0.16));
    e.push(sq(l.nsdb, 2.0, 6.0, 1.0));
    e.push(sq(l.cont, 2.995, 6.4, 0.16));
    e.push(sq(l.nsdb, 19.0, 6.0, 1.0));
    e.push(sq(l.cont, 19.92, 6.4, 0.16));
    e.push(sq(l.nsdb, 20.5, 10.0, 1.0));
    e.push(sq(l.cont, 20.92, 10.4, 0.16));
    e.push(sq(l.nsdb, 1000.0, 1000.0, 1.0));
    e.push(sq(l.cont, 1000.4, 1000.4, 0.16));
    e.push(sq(l.nsdb, 5.0, 6.0, 1.0));
    e.push(rect(l.cont, 5.4, 6.2, 5.56, 6.7));
    write("nsdblock", "nSDB.e.h1", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/nsdblock")
        .expect("failed to create output directory");
    let l = L::new(pdk);
    width_suite("nsdblock", "nSDB.a", l.nsdb, 0.31);
    space_suite("nsdblock", "nSDB.b", l.nsdb, 0.31, 1.0);
    nsdb_c_h(&l);
    nsdb_e_h(&l);
}
