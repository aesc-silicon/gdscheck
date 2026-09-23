// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::sealring::{P, write};
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/lu";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    lu_a(pdk);
    lu_b(pdk);
    lu_c(pdk);
    lu_c1(pdk);

    hardening(pdk);
}

/// LU.a — a PMOS source/drain (P+Activ in NWell) more than 20 µm from the NWell tie.  The
/// tie itself is compact (Activ ≈ Cont + 1.5 µm), so LU.c/LU.d stay clean.
fn lu_a(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let nwell = layer(pdk, "NWell");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(nwell, o, o, o + 80.0, o + 20.0),
        // N+ NWell tie (no pSD): Activ 4×4, Cont 1×1 centred → 1.5 µm extension.
        rect(activ, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(cont, o + 3.5, o + 9.5, o + 4.5, o + 10.5),
        // PMOS S/D (P+) 64 µm from the tie → LU.a.
        rect(activ, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
        rect(psd, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
    ];
    write_gz(&format!("{DIR}/LU.a.gds.gz"), library("TOP", elems));
}

/// LU.b — an NMOS source/drain (N+Activ in the substrate) more than 20 µm from the
/// substrate tie.  The tie is compact, so LU.c1/LU.d1 stay clean.
fn lu_b(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        // P+ substrate tie (pSD), compact.
        rect(activ, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(psd, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(cont, o + 3.5, o + 9.5, o + 4.5, o + 10.5),
        // NMOS S/D (N+ = Activ with no pSD) 64 µm away → LU.b.
        rect(activ, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
    ];
    write_gz(&format!("{DIR}/LU.b.gds.gz"), library("TOP", elems));
}

/// LU.c / LU.d — an NWell tie whose Activ stretches ~20 µm past its only contact (> 6 µm).
fn lu_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let nwell = layer(pdk, "NWell");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(nwell, o, o, o + 40.0, o + 20.0),
        // N+ NWell tie, Activ 23 µm long, Cont at the left end.
        rect(activ, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(cont, o + 3.0, o + 6.5, o + 5.0, o + 8.5),
    ];
    write_gz(&format!("{DIR}/LU.c.gds.gz"), library("TOP", elems));
}

/// LU.c1 / LU.d1 — a substrate tie whose Activ stretches ~20 µm past its only contact.
fn lu_c1(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(psd, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(cont, o + 3.0, o + 6.5, o + 5.0, o + 8.5),
    ];
    write_gz(&format!("{DIR}/LU.c1.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts (hardening/SPEC.md) for the `sealring`, `slit`, `lbe` and `lu`
// decks: sections 6.10 (Sealring, Seal.*), 7.3 (Metal Slits, Slt.*), 9.1 (Localized
// Backside Etching, LBE.*) and 7.2.2 (Latch-up, LU.*) of the SG13G2 layout rules.
// Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`; each function's
// comment states the geometry and what the manual says about it, the expected answers
// are in the decks' tables of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/beol_misc.md.
//
// The seal frames are drawn as IHP's `sealring` pcell draws them: the EdgeSeal marker
// is the 4.2 µm ring itself, the conductors coincide with it, the via rings are 4.2
// long pieces overlapping into a ring, and the Passiv ring lies 3 µm outside.

/// A tie: a 1 × 1 Activ at `(x, y)` with a Cont at its centre, under `impl_` if given.
fn tie(p: &P, x: f64, y: f64, impl_: Option<(i16, i16)>, cont: bool) -> Vec<GdsElement> {
    let mut e = vec![rect(p.activ, x, y, x + 1.0, y + 1.0)];
    if let Some(i) = impl_ {
        e.push(rect(i, x, y, x + 1.0, y + 1.0));
    }
    if cont {
        e.push(rect(p.cont, x + 0.42, y + 0.42, x + 0.58, y + 0.58));
    }
    e
}

/// h1 - LU.a.  "Any portion" of the P+Activ is within the value, so a shape is read
/// by its farthest point.  Separate NWells, each with an N+ tie (1 × 1 Activ, Cont)
/// and a P+Activ under pSD: (a) a 2 × 2 P+ whose far edge is 20 from the tie, clean;
/// (b) 20.005: fires; (c) a 1.5 square whose far corner is dx = dy = 14.5 (20.5) from
/// the tie's corner, under 20 on either axis alone: fires; (d) one whose far corner is
/// dx = dy = 14.14 (19.997): clean; (e) a P+ bar 52 long whose near end is 5 from the
/// tie and whose far end 57: fires; (f) the tie in its own NWell 4 away from the
/// P+'s well, 9 from the P+: no tie in the P+'s well, fires; (g) an uncontacted
/// N+Activ 10 from the P+: section 4.2's tie needs no Cont, clean; (h) a P+ alone in
/// a well at (1000, 1000): fires.
fn lu_a_h(p: &P) {
    let psd = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![rect(p.activ, x0, y0, x1, y1), rect(p.psd, x0, y0, x1, y1)]
    };
    let mut e = vec![];
    for (i, dx) in [(0.0, 20.0), (1.0, 20.005)] {
        let y = i * 30.0;
        e.push(rect(p.nwell, 0.0, y, 40.0, y + 20.0));
        e.extend(tie(p, 2.0, y + 9.5, None, true));
        e.extend(psd(1.0 + dx, y + 9.0, 3.0 + dx, y + 11.0));
    }
    for (i, d) in [(2.0, 14.5), (3.0, 14.14)] {
        let y = i * 30.0;
        e.push(rect(p.nwell, 0.0, y, 40.0, y + 20.0));
        e.extend(tie(p, 2.0, y + 2.0, None, true));
        e.extend(psd(1.5 + d, y + 1.5 + d, 3.0 + d, y + 3.0 + d));
    }
    e.push(rect(p.nwell, 0.0, 120.0, 80.0, 140.0)); // (e)
    e.extend(tie(p, 2.0, 129.5, None, true));
    e.extend(psd(8.0, 129.0, 60.0, 131.0));
    e.push(rect(p.nwell, 0.0, 150.0, 6.0, 170.0)); // (f)
    e.push(rect(p.nwell, 10.0, 150.0, 50.0, 170.0));
    e.extend(tie(p, 2.0, 159.5, None, true));
    e.extend(psd(12.0, 159.0, 14.0, 161.0));
    e.push(rect(p.nwell, 0.0, 180.0, 40.0, 200.0)); // (g)
    e.extend(tie(p, 2.0, 189.5, None, false));
    e.extend(psd(13.0, 189.0, 15.0, 191.0));
    e.push(rect(p.nwell, 1000.0, 1000.0, 1040.0, 1020.0)); // (h)
    e.extend(psd(1010.0, 1009.0, 1012.0, 1011.0));
    write("lu", "LU.a.h1", e);

    // h2/h3 - fifty wells with a tie and a P+ whose far edge is 20.005 from it, flat
    // and as an array.
    let mut cell = vec![rect(p.nwell, 0.0, 0.0, 30.0, 10.0)];
    cell.extend(tie(p, 2.0, 4.5, None, true));
    cell.extend(psd(21.005, 4.0, 23.005, 6.0));
}

/// h1 - LU.b, as LU.a.  In the substrate, a P+ tie (1 × 1 Activ under pSD, Cont)
/// and an N+Activ (Activ, no pSD): (a) a 2 × 2 N+ whose far edge is 20 from the tie,
/// clean; (b) 20.005: fires; (c) a 1.5 square whose far corner is 20.5 from the
/// tie's corner (14.5 on each axis): fires; (d) 19.997 (14.14): clean; (e) an N+ bar
/// 52 long, 5 from the tie at its near end: fires; (f) an N+ under a PWell:block 9
/// from a tie: in no well (section 4.2), not read, clean; (g) an uncontacted P+ 10
/// from the N+: clean; (h) an N+ alone at (1000, 1000): fires.
fn lu_b_h(p: &P) {
    let n = |x0: f64, y0: f64, x1: f64, y1: f64| rect(p.activ, x0, y0, x1, y1);
    let mut e = vec![];
    for (i, dx) in [(0.0, 20.0), (1.0, 20.005)] {
        let y = i * 30.0;
        e.extend(tie(p, 2.0, y + 9.5, Some(p.psd), true));
        e.push(n(1.0 + dx, y + 9.0, 3.0 + dx, y + 11.0));
    }
    for (i, d) in [(2.0, 14.5), (3.0, 14.14)] {
        let y = i * 30.0;
        e.extend(tie(p, 2.0, y + 2.0, Some(p.psd), true));
        e.push(n(1.5 + d, y + 1.5 + d, 3.0 + d, y + 3.0 + d));
    }
    e.extend(tie(p, 2.0, 129.5, Some(p.psd), true)); // (e)
    e.push(n(8.0, 129.0, 60.0, 131.0));
    e.extend(tie(p, 2.0, 159.5, Some(p.psd), true)); // (f)
    e.push(rect(p.pwb, 10.0, 150.0, 30.0, 170.0));
    e.push(n(12.0, 159.0, 14.0, 161.0));
    e.extend(tie(p, 2.0, 189.5, Some(p.psd), false)); // (g)
    e.push(n(13.0, 189.0, 15.0, 191.0));
    e.push(n(1010.0, 1009.0, 1012.0, 1011.0)); // (h)
    write("lu", "LU.b.h1", e);
}

/// The tie extension layouts, in an NWell (`nwell`, N+ ties abutting a PMOS) or in
/// the substrate (P+ ties abutting an NMOS).  (a) a tie 1 × 12.16 with its Cont
/// centred: 6.0 each way, clean; (b) 12.17 with the Cont 6.005 from its left end:
/// fires (LU.d/d1 - the tie stands alone); (c) a square tie 12.16 with the Cont
/// centred: 6.0 beyond the Cont on every side, the corners 8.49 away diagonally -
/// the extension is 6, clean; (d) a tie 24.33 long with two Conts 12.01 apart: its
/// middle is 6.005 from either, fires (LU.d/d1); (e) figure 7.4's abutted tie: a
/// transistor (Activ 10 × 8 under a 1 wide gate, four Conts in its left source) with
/// an Activ 4 wide of the other type abutting below the source and reaching 5.92
/// below the lowest Cont: clean; (f) the same reaching 6.005 below: fires (LU.c/c1 -
/// abutted); (g) a 4 × 2 tie with no Cont at all: fires (LU.d/d1); (h) (b) again at
/// (1000, 1000).
fn lu_cd(p: &P, nwell: bool) {
    let (tie_impl, sd_impl) = if nwell {
        (None, Some(p.psd))
    } else {
        (Some(p.psd), None)
    };
    let mut e = vec![];
    let well = |e: &mut Vec<GdsElement>, y0: f64, y1: f64| {
        if nwell {
            e.push(rect(p.nwell, 0.0, y0, 40.0, y1));
        }
    };
    let tie_box = |e: &mut Vec<GdsElement>, x0: f64, y0: f64, x1: f64, y1: f64| {
        e.push(rect(p.activ, x0, y0, x1, y1));
        if let Some(i) = tie_impl {
            e.push(rect(i, x0, y0, x1, y1));
        }
    };
    let cont = |e: &mut Vec<GdsElement>, x: f64, y: f64| {
        e.push(rect(p.cont, x, y, x + 0.16, y + 0.16));
    };
    well(&mut e, 0.0, 20.0); // (a)
    tie_box(&mut e, 5.5, 9.0, 17.66, 10.0);
    cont(&mut e, 11.5, 9.42);
    well(&mut e, 30.0, 50.0); // (b)
    tie_box(&mut e, 5.495, 39.0, 17.665, 40.0);
    cont(&mut e, 11.5, 39.42);
    well(&mut e, 60.0, 80.0); // (c)
    tie_box(&mut e, 5.0, 63.0, 17.16, 75.16);
    cont(&mut e, 11.0, 69.0);
    well(&mut e, 90.0, 110.0); // (d)
    tie_box(&mut e, 0.0, 99.0, 24.33, 100.0);
    cont(&mut e, 6.0, 99.42);
    cont(&mut e, 18.17, 99.42);
    for (i, ext) in [(0.0, 5.92), (1.0, 6.005)] {
        // (e), (f): the transistor at y0 + 7..15, its lowest Cont at y0 + 7.92
        let y0 = 118.0 + i * 30.0;
        well(&mut e, y0, y0 + 22.0);
        e.push(rect(p.activ, 5.0, y0 + 7.0, 15.0, y0 + 15.0));
        if let Some(i) = sd_impl {
            e.push(rect(i, 4.5, y0 + 7.0, 15.5, y0 + 15.5));
        }
        e.push(rect(p.gp, 9.5, y0 + 6.0, 10.5, y0 + 16.0));
        for k in 0..4 {
            cont(&mut e, 6.92, y0 + 7.92 + k as f64 * 2.0);
        }
        tie_box(&mut e, 5.0, y0 + 7.92 - ext, 9.0, y0 + 7.0);
    }
    well(&mut e, 180.0, 200.0); // (g)
    tie_box(&mut e, 5.0, 189.0, 9.0, 191.0);
    if nwell {
        e.push(rect(p.nwell, 1000.0, 1000.0, 1040.0, 1020.0)); // (h)
    }
    tie_box(&mut e, 1005.495, 1009.0, 1017.665, 1010.0);
    cont(&mut e, 1011.5, 1009.42);
    write("lu", if nwell { "LU.c.h1" } else { "LU.c1.h1" }, e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/lu").expect("failed to create output directory");
    let p = P::new(pdk);
    lu_a_h(&p);
    lu_b_h(&p);
    lu_cd(&p, true);
    lu_cd(&p, false);
}
