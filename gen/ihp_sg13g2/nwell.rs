// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, min_width_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/nwell";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    nw_a(pdk);
    nw_b(pdk);
    nw_b1(pdk);
    nw_c(pdk);
    nw_c1(pdk);
    nw_d(pdk);
    nw_d1(pdk);
    nw_e(pdk);
    nw_e1(pdk);
    nw_f(pdk);
    nw_f1(pdk);
    nw_dig(pdk);
}

/// P+Activ footprint (Activ ∩ pSD) at `(x, y)`.
fn pact(activ: (i16, i16), psd: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![rect(activ, x, y, x + w, y + h), rect(psd, x, y, x + w, y + h)]
}

/// N+Activ footprint (Activ ∩ nSD) at `(x, y)`.
fn nact(activ: (i16, i16), nsd: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![rect(activ, x, y, x + w, y + h), rect(nsd, x, y, x + w, y + h)]
}

fn nw_a(pdk: &PdkConfig) {
    let l = layer(pdk, "NWell");
    let elems = min_width_pattern(l, 0.62, 0.62, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NW.a.gds.gz"), library("TOP", elems));
}

/// NW.b/b1 — same-net merge then different-net space.  Three NWell pairs: gap 0.50 µm
/// (< 0.62) merges (same net) → clean; gap 1.00 µm (in [0.62, 1.80)) → NW.b1; gap 2.00 µm
/// (≥ 1.80) → clean.  Exercises the `close` (size-merge) op behind NWellMerged.
/// NW.b — min. NWell space or notch 0.62: a pair at 0.50 (< 0.62 → fires) and at
/// 0.62 exactly (clean).  Neither pair draws NW.b1: both gaps close in NWellMerged
/// (the close radius 0.31 bridges gaps up to and including exactly 0.62).
fn nw_b(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let elems = vec![
        rect(nw, o, o, o + 2.0, o + 2.0),
        rect(nw, o + 2.5, o, o + 4.5, o + 2.0), // gap 0.50 → NW.b
        rect(nw, o + 10.0, o, o + 12.0, o + 2.0),
        rect(nw, o + 12.62, o, o + 14.62, o + 2.0), // gap 0.62 → clean for NW.b
    ];
    write_gz(&format!("{DIR}/NW.b.gds.gz"), library("TOP", elems));
}

fn nw_b1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let pair = |y: f64, gap: f64| {
        vec![
            rect(nw, o, y, o + 2.0, y + 2.0),
            rect(nw, o + 2.0 + gap, y, o + 4.0 + gap, y + 2.0),
        ]
    };
    let mut elems = pair(o, 0.50); // merged → clean
    elems.extend(pair(o + 5.0, 1.00)); // NW.b1
    elems.extend(pair(o + 10.0, 2.00)); // clean
    write_gz(&format!("{DIR}/NW.b1.gds.gz"), library("TOP", elems));
}

/// NW.c — min. NWell enclosure of P+Activ (PMOS S/D) not in ThickGateOx, 0.31 µm.  One
/// P+Activ enclosed by 0.31 (clean); one with a 0.30 left margin (violation).
fn nw_c(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o - 0.31, o - 0.31, o + 0.81, o + 0.81)];
    elems.extend(pact(activ, psd, o, o, 0.5, 0.5)); // 0.31 all round → clean
    elems.push(rect(nw, o + 3.0 - 0.30, o - 0.31, o + 3.81, o + 0.81));
    elems.extend(pact(activ, psd, o + 3.0, o, 0.5, 0.5)); // 0.30 left → violation
    write_gz(&format!("{DIR}/NW.c.gds.gz"), library("TOP", elems));
}

/// NW.c1 — min. NWell enclosure of P+Activ inside ThickGateOx, 0.62 µm.
fn nw_c1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![
        rect(nw, o - 0.62, o - 0.62, o + 1.12, o + 1.12),
        rect(tgo, o - 0.1, o - 0.1, o + 0.6, o + 0.6),
    ];
    elems.extend(pact(activ, psd, o, o, 0.5, 0.5)); // 0.62 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.61, o - 0.62, o + 4.0 + 1.12, o + 1.12));
    elems.push(rect(tgo, o + 4.0 - 0.1, o - 0.1, o + 4.6, o + 0.6));
    elems.extend(pact(activ, psd, o + 4.0, o, 0.5, 0.5)); // 0.61 left → violation
    write_gz(&format!("{DIR}/NW.c1.gds.gz"), library("TOP", elems));
}

/// NW.d — min. NWell space to external N+Activ not in ThickGateOx, 0.31 µm.  N+ is the
/// full derived implant: drawn nSD fires, but so does plain undoped Activ (N+ by
/// default); Activ under nSD:block (without drawn nSD) is not N+ and stays clean.
fn nw_d(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    elems.extend(nact(activ, nsd, o + 1.0 + 0.31, o, 0.5, 0.5)); // gap 0.31 → clean
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    elems.extend(nact(activ, nsd, o + 5.0 + 0.30, o, 0.5, 0.5)); // gap 0.30 → violation
    // Plain Activ (no implant drawn — N+ by default) at 0.30 → violation.
    elems.push(rect(nw, o + 8.0, o, o + 9.0, o + 1.0));
    elems.push(rect(activ, o + 9.0 + 0.30, o, o + 9.8, o + 0.5));
    // Activ under nSD:block, no drawn nSD (default implant suppressed) at 0.30 → clean.
    elems.push(rect(nw, o + 12.0, o, o + 13.0, o + 1.0));
    elems.push(rect(activ, o + 13.0 + 0.30, o, o + 13.8, o + 0.5));
    elems.push(rect(layer(pdk, "nSD.block"), o + 13.0 + 0.30, o, o + 13.8, o + 0.5));
    write_gz(&format!("{DIR}/NW.d.gds.gz"), library("TOP", elems));
}

/// NW.d1 — min. NWell space to external N+Activ inside ThickGateOx, 0.62 µm.
fn nw_d1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + 0.62;
    elems.extend(nact(activ, nsd, x1, o, 0.5, 0.5)); // gap 0.62 → clean
    elems.push(rect(tgo, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + 0.61;
    elems.extend(nact(activ, nsd, x2, o, 0.5, 0.5)); // gap 0.61 → violation
    elems.push(rect(tgo, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    write_gz(&format!("{DIR}/NW.d1.gds.gz"), library("TOP", elems));
}

/// NW.e — min. NWell enclosure of the NWell tie (N+Activ in NWell), not in TGO, 0.24 µm.  The
/// tie is plain Activ (no pSD → N+).  One enclosed by 0.30 (clean); one with 0.20 left → NW.e.
fn nw_e(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o - 0.30, o - 0.30, o + 0.80, o + 0.80)];
    elems.push(rect(activ, o, o, o + 0.5, o + 0.5)); // 0.30 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.20, o - 0.30, o + 4.0 + 0.80, o + 0.80));
    elems.push(rect(activ, o + 4.0, o, o + 4.5, o + 0.5)); // 0.20 left → NW.e
    // A tie CROSSING the NWell boundary: not "surrounded entirely by NWell", so NW.e
    // does not apply (skip_clipped) even though its clipped-in piece has small margins.
    elems.push(rect(nw, o + 8.0, o - 0.30, o + 9.1, o + 0.80));
    elems.push(rect(activ, o + 8.9, o, o + 10.0, o + 0.5)); // extends 0.9 past NWell
    write_gz(&format!("{DIR}/NW.e.gds.gz"), library("TOP", elems));
}

/// NW.e1 — NWell enclosure of the NWell tie inside ThickGateOx, 0.62 µm.
fn nw_e1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![
        rect(nw, o - 0.70, o - 0.70, o + 1.20, o + 1.20),
        rect(tgo, o - 0.1, o - 0.1, o + 0.6, o + 0.6),
    ];
    elems.push(rect(activ, o, o, o + 0.5, o + 0.5)); // 0.70 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.50, o - 0.70, o + 4.0 + 1.20, o + 1.20));
    elems.push(rect(tgo, o + 4.0 - 0.1, o - 0.1, o + 4.6, o + 0.6));
    elems.push(rect(activ, o + 4.0, o, o + 4.5, o + 0.5)); // 0.50 left → NW.e1
    write_gz(&format!("{DIR}/NW.e1.gds.gz"), library("TOP", elems));
}

/// Chapter 8 DigiBnd relaxation of the HV rules.  Inside one DigiBnd region:
/// - TGO N+Activ at 0.30 from NWell → NW.d1.dig (< 0.31); at 0.45 → clean (the strict
///   0.62 does not apply in the digital split);
/// - TGO P+Activ in NWell with 0.25 worst margin → NW.c1.dig; 0.45 margins → clean;
/// - TGO NWell tie with 0.20 worst margin → NW.e1.dig; 0.30 margins → clean.
///
/// Outside the DigiBnd, a TGO N+Activ at 0.45 still draws the strict NW.d1.
fn nw_dig(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(layer(pdk, "DigiBnd"), o - 2.0, o - 2.0, o + 25.0, o + 3.0)];
    // d1.dig fires (gap 0.30) / relaxed-clean (gap 0.45).
    let d1 = |x: f64, gap: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + 1.0, o + 1.0));
        e.extend(nact(activ, nsd, x + 1.0 + gap, o, 0.5, 0.5));
        e.push(rect(tgo, x + 1.0 + gap - 0.1, o - 0.1, x + 1.0 + gap + 0.6, o + 0.6));
    };
    d1(o, 0.30, &mut elems);
    d1(o + 4.0, 0.45, &mut elems);
    // c1.dig fires (left margin 0.25, rest 0.45) / relaxed-clean (0.45 all round).
    let c1 = |x: f64, left: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + left + 0.5 + 0.45, o + 1.4));
        e.extend(pact(activ, psd, x + left, o + 0.45, 0.5, 0.5));
        e.push(rect(tgo, x + left - 0.1, o + 0.35, x + left + 0.6, o + 1.05));
    };
    c1(o + 8.0, 0.25, &mut elems);
    c1(o + 12.0, 0.45, &mut elems);
    // e1.dig fires (left margin 0.20, rest 0.30) / relaxed-clean (0.30 all round).
    let e1 = |x: f64, left: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + left + 0.5 + 0.30, o + 1.1));
        e.push(rect(activ, x + left, o + 0.30, x + left + 0.5, o + 0.80));
        e.push(rect(tgo, x + left - 0.1, o + 0.20, x + left + 0.6, o + 0.90));
    };
    e1(o + 16.0, 0.20, &mut elems);
    e1(o + 20.0, 0.30, &mut elems);
    // Outside the DigiBnd: strict NW.d1 still fires at 0.45.
    d1(o + 30.0, 0.45, &mut elems);
    write_gz(&format!("{DIR}/NW.dig.gds.gz"), library("TOP", elems));
}

/// NW.f — min. NWell space to the substrate tie (P+Activ in PWell), not in TGO, 0.24 µm.
fn nw_f(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    elems.extend(pact(activ, psd, o + 1.0 + 0.30, o, 0.5, 0.5)); // gap 0.30 → clean
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    elems.extend(pact(activ, psd, o + 5.0 + 0.20, o, 0.5, 0.5)); // gap 0.20 → NW.f
    write_gz(&format!("{DIR}/NW.f.gds.gz"), library("TOP", elems));
}

/// NW.f1 — NWell space to the substrate tie inside ThickGateOx, 0.62 µm.
fn nw_f1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + 0.62;
    elems.extend(pact(activ, psd, x1, o, 0.5, 0.5)); // gap 0.62 → clean
    elems.push(rect(tgo, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + 0.50;
    elems.extend(pact(activ, psd, x2, o, 0.5, 0.5)); // gap 0.50 → NW.f1
    elems.push(rect(tgo, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    write_gz(&format!("{DIR}/NW.f1.gds.gz"), library("TOP", elems));
}
