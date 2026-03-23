// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SG13CMOS5L-specific fixtures.
//!
//! Everything shared with SG13G2 needs no fixtures here: the layer numbering is
//! identical and the shared decks are the same files, so the CMOS5L decks run
//! against the SG13G2 fixture tree directly (see the parity tests in
//! tests/ihp-sg13cmos5l.rs).  Only the rules that genuinely differ get fixtures:
//! TV1.c (TopVia1 lands on Metal4), Pas.c / Pad.i (TopMetal1 is the top metal),
//! the §3.2 extra forbidden layers, and the chapter-8 DigiBnd splits of Cnt.c and
//! NW.f1.

use crate::helpers::{enclosure_pattern, layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13cmos5l";
const OFFSET: f64 = 20.0;
const DELTA: f64 = -0.005;

pub fn generate(pdk: &PdkConfig) {
    tv1_c(pdk);
    pas_c(pdk);
    pad_i(pdk);
    forbidden(pdk);
    cnt_digi(pdk);
    nw_f1_digi(pdk);
}

fn dir(sub: &str) -> String {
    let d = format!("{DIR}/{sub}");
    std::fs::create_dir_all(&d).expect("failed to create output directory");
    d
}

/// TV1.c — min. Metal4 enclosure of TopVia1 (0.10; TopVia1 sits on Metal4 in the
/// CMOS5L stack).  Clean + four one-sided undershoots.  No TopMetal1 is drawn, so
/// the tests ignore TV1.d, exactly like the SG13G2 TV1.c fixture does.
fn tv1_c(pdk: &PdkConfig) {
    let elems = enclosure_pattern(
        layer(pdk, "Metal4"), layer(pdk, "TopVia1"),
        0.10, 0.42, 5.0, OFFSET, DELTA,
    );
    write_gz(&format!("{}/TV1.c.gds.gz", dir("topvia1")), library("TOP", elems));
}

/// Pas.c — min. TopMetal1 enclosure of Passiv inside the seal (2.10).  Same
/// geometry as the SG13G2 fixture with the top metal swapped, except the EdgeSeal
/// is a proper ring: the upstream deck anchors on `edgeseal.holes` (the chip
/// interior), which a solid box doesn't have.  The ringed pattern is checked
/// (4 violations); the identical copy outside the ring is exempt.
fn pas_c(pdk: &PdkConfig) {
    let tm1 = layer(pdk, "TopMetal1");
    let passiv = layer(pdk, "Passiv");
    let es = layer(pdk, "EdgeSeal");
    let (enc, width, dist) = (2.10, 10.0, 5.0);
    let outer = width + 2.0 * enc;
    let step = outer + dist;
    let span = 4.0 * step + outer;

    let mut elems = enclosure_pattern(tm1, passiv, enc, width, dist, OFFSET, DELTA);
    // EdgeSeal ring (width 2) around the checked pattern.
    let (x0, y0, x1, y1) = (OFFSET - 8.0, -8.0, OFFSET + span + 8.0, outer + 8.0);
    elems.push(rect(es, x0, y0, x1, y0 + 2.0)); // bottom
    elems.push(rect(es, x0, y1 - 2.0, x1, y1)); // top
    elems.push(rect(es, x0, y0 + 2.0, x0 + 2.0, y1 - 2.0)); // left
    elems.push(rect(es, x1 - 2.0, y0 + 2.0, x1, y1 - 2.0)); // right
    elems.extend(enclosure_pattern(tm1, passiv, enc, width, dist, OFFSET + span + 50.0, DELTA));
    write_gz(&format!("{}/Pas.c.gds.gz", dir("passiv")), library("TOP", elems));
}

/// Pad.i — a dfpad opening needs TopMetal1 underneath (TM1 is the CMOS5L pad
/// metal): one well-formed pad (clean) and one naked dfpad (fires).
fn pad_i(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        rect(layer(pdk, "TopMetal1"), o, o, o + 90.0, o + 90.0),
        rect(layer(pdk, "dfpad"), o + 5.0, o + 5.0, o + 85.0, o + 85.0),
        rect(layer(pdk, "Passiv"), o + 10.0, o + 10.0, o + 80.0, o + 80.0),
        rect(layer(pdk, "dfpad"), o + 120.0, o, o + 140.0, o + 20.0), // no TM1 → Pad.i
    ];
    write_gz(&format!("{}/Pad.i.gds.gz", dir("pad")), library("TOP", elems));
}

/// The §3.2 CMOS5L-forbidden layers: one shape each on Metal5, TRANS, nBuLay and
/// MIM → four violations.
fn forbidden(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut elems: Vec<GdsElement> = Vec::new();
    for (i, name) in ["Metal5", "TRANS", "nBuLay", "MIM"].iter().enumerate() {
        let x = o + i as f64 * 10.0;
        elems.push(rect(layer(pdk, name), x, o, x + 2.0, o + 2.0));
    }
    write_gz(&format!("{DIR}/forbidden.gds.gz"), library("TOP", elems));
}

/// One contact instance: Cont 0.16 with an Activ margin of `m`, covered by Metal1
/// (Cnt.h) with a healthy 0.1 margin.
fn cont_at(pdk: &PdkConfig, x: f64, y: f64, m: f64) -> Vec<GdsElement> {
    let a = 0.16 + 2.0 * m;
    vec![
        rect(layer(pdk, "Activ"), x, y, x + a, y + a),
        rect(layer(pdk, "Cont"), x + m, y + m, x + m + 0.16, y + m + 0.16),
        rect(layer(pdk, "Metal1"), x + m - 0.1, y + m - 0.1, x + m + 0.26, y + m + 0.26),
    ]
}

/// Cnt.c split by DigiBnd (0.07 analog / 0.05 digital):
/// - margin 0.065 outside the DigiBnd → Cnt.c fires;
/// - margin 0.065 inside → clean (only 0.05 applies there);
/// - margin 0.045 inside → Cnt.c.Digi fires.
fn cnt_digi(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut elems = cont_at(pdk, o, o, 0.065); // analog → Cnt.c
    elems.extend(cont_at(pdk, o + 10.0, o, 0.065)); // digital, relaxed → clean
    elems.extend(cont_at(pdk, o + 14.0, o, 0.045)); // digital → Cnt.c.Digi
    elems.push(rect(layer(pdk, "DigiBnd"), o + 8.0, o - 2.0, o + 18.0, o + 3.0));
    write_gz(&format!("{}/Cnt.c.digi.gds.gz", dir("cont")), library("TOP", elems));
}

/// One HV substrate tie: a P+Activ (Activ+pSD) under ThickGateOx at `gap` from a
/// 1×1 NWell.
fn hv_tie_at(pdk: &PdkConfig, x: f64, y: f64, gap: f64) -> Vec<GdsElement> {
    let tx = x + 1.0 + gap;
    vec![
        rect(layer(pdk, "NWell"), x, y, x + 1.0, y + 1.0),
        rect(layer(pdk, "Activ"), tx, y, tx + 0.5, y + 0.5),
        rect(layer(pdk, "pSD"), tx, y, tx + 0.5, y + 0.5),
        rect(layer(pdk, "ThickGateOx"), tx - 0.1, y - 0.1, tx + 0.6, y + 0.6),
    ]
}

/// NW.f1 split by DigiBnd (0.62 analog / 0.24 digital):
/// - gap 0.30 outside the DigiBnd → NW.f1 fires (< 0.62);
/// - gap 0.30 inside → clean (only 0.24 applies there);
/// - gap 0.20 inside → NW.f1.dig fires.
fn nw_f1_digi(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut elems = hv_tie_at(pdk, o, o, 0.30); // analog → NW.f1
    elems.extend(hv_tie_at(pdk, o + 10.0, o, 0.30)); // digital, relaxed → clean
    elems.extend(hv_tie_at(pdk, o + 14.0, o, 0.20)); // digital → NW.f1.dig
    elems.push(rect(layer(pdk, "DigiBnd"), o + 9.0, o - 2.0, o + 18.0, o + 3.0));
    write_gz(&format!("{}/NW.f1.digi.gds.gz", dir("nwell")), library("TOP", elems));
}
