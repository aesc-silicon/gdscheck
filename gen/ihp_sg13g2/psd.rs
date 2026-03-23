// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, min_width_pattern, rect, space_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/psd";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    psd_a(pdk);
    psd_b(pdk);
    psd_d(pdk);
    psd_k(pdk);
    psd_l(pdk);
    psd_g(pdk);
    psd_e(pdk);
    psd_f(pdk);
    psd_c1(pdk);
}

/// One abutted substrate tie: an Activ bar of height `h` whose right portion is covered
/// by a pSD rect overlapping it by `ov` (the P+ overlap sliver = `ov`×`h`); the uncovered
/// left portion is the N+ tap.  The pSD extends 1.5 µm past the Activ so it is a healthy
/// standalone pSD region (pSD.a/k quiet), and 0.1 vertically past it so pSD.c1's 0.03
/// enclosure stays quiet (a flush sliver is a real pSD.c1 violation — both tools agree).
fn tie(pdk: &PdkConfig, x: f64, y: f64, h: f64, ov: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Activ"), x, y, x + 1.5, y + h),
        rect(layer(pdk, "pSD"), x + 1.5 - ov, y - 0.1, x + 3.0, y + h + 0.1),
    ]
}

/// pSD.e — "min. pSD overlap of Activ at one position" (0.30):
/// - A: uniform 0.30 sliver (exactly at value) → clean.
/// - B: uniform 0.20 sliver → violation (nowhere ≥0.30).
/// - C: 0.20 sliver with a 0.40-deep × 0.5-tall pocket (extra pSD rect) → clean
///   (wide enough at one position, narrow elsewhere — must NOT fire).
/// - D: 0.20 × 0.5 tab → violation (0.5 long but only 0.2 in the narrow direction;
///   the real rule fails this too — width, not length, is the metric).
fn psd_e(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = tie(pdk, o, o, 1.0, 0.30); // A
    e.extend(tie(pdk, o + 8.0, o, 1.0, 0.20)); // B
    e.extend(tie(pdk, o + 16.0, o, 1.0, 0.20)); // C base...
    // ...plus the wide pocket: pSD reaching 0.40 into the Activ over 0.5 of its height.
    e.push(rect(layer(pdk, "pSD"), o + 16.0 + 1.5 - 0.40, o + 0.25, o + 16.0 + 3.0, o + 0.75));
    e.extend(tie(pdk, o + 24.0, o, 0.5, 0.20)); // D (sliver area 0.10 ≥ 0.09 keeps pSD.g quiet)
    write_gz(&format!("{DIR}/pSD.e.gds.gz"), library("TOP", e));
}

/// One abutted NWell tie at `cx`: NWell 10×10, a P+ body (Activ 4×2 under pSD ending
/// flush at the abutment line) and an N+ tab of `w`×`d` on top (drawn nSD over it, as
/// KLayout's recognition wants; ours derives N+ as Activ−pSD).
fn nwell_tie(pdk: &PdkConfig, cx: f64, cy: f64, w: f64, d: f64) -> Vec<gds21::GdsElement> {
    let mut e = nwell_tie_base(pdk, cx, cy);
    let tx = cx + 5.0 - w / 2.0;
    e.push(rect(layer(pdk, "Activ"), tx, cy + 6.0, tx + w, cy + 6.0 + d));
    e.push(rect(layer(pdk, "nSD"), tx, cy + 6.0, tx + w, cy + 6.0 + d));
    e
}

fn nwell_tie_base(pdk: &PdkConfig, cx: f64, cy: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "NWell"), cx, cy, cx + 10.0, cy + 10.0),
        rect(layer(pdk, "Activ"), cx + 3.0, cy + 4.0, cx + 7.0, cy + 6.0),
        rect(layer(pdk, "pSD"), cx + 2.8, cy + 3.8, cx + 7.2, cy + 6.0),
    ]
}

/// pSD.f — min. Activ extension over pSD at ONE position (abutted NWell tie, 0.30).
/// Tab depth `d` is the metric, not its width:
/// - d=0.20 → fires;  d=0.30 → clean;  d=0.295 → fires (grid boundary);
/// - 0.2-wide × 0.45-deep tab → clean (dies under a min-width/opening formulation —
///   the pSD.e reformulation is provably wrong here; the depth reaches 0.45);
/// - 2.0-wide × 0.20-deep → fires;
/// - L-shaped tab (0.1-deep stem + sideways arm at ≤0.2) → fires HERE ONLY: nowhere
///   extends 0.30 past the abutment (PDF-first); the shipped KLayout body's "bad band"
///   covers only the area directly in front of the abutment edge, letting the arm
///   escape.  Tab widths ≥0.5 (or area ≥0.09) keep pSD.g quiet.
fn psd_f(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nwell_tie(pdk, o, o, 0.5, 0.20); // fires
    e.extend(nwell_tie(pdk, o + 20.0, o, 1.0, 0.30)); // clean
    e.extend(nwell_tie(pdk, o + 40.0, o, 0.5, 0.295)); // fires
    e.extend(nwell_tie(pdk, o + 60.0, o, 0.2, 0.45)); // clean (area exactly 0.09)
    e.extend(nwell_tie(pdk, o + 80.0, o, 2.0, 0.20)); // fires
    let cx = o + 100.0;
    e.extend(nwell_tie_base(pdk, cx, o)); // L-shape: fires (ours only)
    e.push(rect(layer(pdk, "Activ"), cx + 4.9, o + 6.0, cx + 5.1, o + 6.1)); // stem
    e.push(rect(layer(pdk, "Activ"), cx + 4.9, o + 6.1, cx + 5.9, o + 6.2)); // arm
    e.push(rect(layer(pdk, "nSD"), cx + 4.9, o + 6.0, cx + 5.1, o + 6.1));
    e.push(rect(layer(pdk, "nSD"), cx + 4.9, o + 6.1, cx + 5.9, o + 6.2));
    write_gz(&format!("{DIR}/pSD.f.gds.gz"), library("TOP", e));
}

/// pSD.c1 — min. pSD enclosure of P+Activ in implicit PWell (0.03), all cases
/// validated 1:1 against the FEOL driver (`enclosed` euclidian semantics):
/// - margin 0.02 → fires;  0.05 → clean;  exactly 0.03 → clean;
/// - abutted substrate tie crossing the pSD edge, lateral margins 0.10 → CLEAN
///   (the protruding part is ignored — crossing edges don't pair);
/// - flush left edge → fires (0 < 0.03 — no skip_coincident here, Rppd.b-style);
/// - crossing tie with a bad 0.02 lateral margin → fires (the inside portion of the
///   crossing edge still pairs — run_enclosure's partial-overlap measurement).
fn psd_c1(pdk: &PdkConfig) {
    let o = OFFSET;
    let m = |cx: f64, margin: f64| -> Vec<gds21::GdsElement> {
        vec![
            rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0),
            rect(layer(pdk, "Activ"), cx + margin, o + margin, cx + 4.0 - margin, o + 4.0 - margin),
        ]
    };
    let mut e = m(o, 0.02); // fires
    e.extend(m(o + 20.0, 0.05)); // clean
    e.extend(m(o + 40.0, 0.03)); // clean (boundary)
    let cx = o + 60.0; // crossing tie, margins 0.10 → clean
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(layer(pdk, "Activ"), cx + 0.1, o + 0.1, cx + 3.9, o + 4.5));
    e.push(rect(layer(pdk, "nSD"), cx + 0.1, o + 4.0, cx + 3.9, o + 4.5));
    let cx = o + 80.0; // flush left edge → fires
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(layer(pdk, "Activ"), cx, o + 0.1, cx + 3.9, o + 3.9));
    let cx = o + 100.0; // crossing tie, bad 0.02 lateral margin → fires
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(layer(pdk, "Activ"), cx + 0.02, o + 0.1, cx + 3.9, o + 4.5));
    e.push(rect(layer(pdk, "nSD"), cx + 0.02, o + 4.0, cx + 3.9, o + 4.5));
    write_gz(&format!("{DIR}/pSD.c1.gds.gz"), library("TOP", e));
}

/// pSD.a — min width 0.31 (one clean + two under-width shapes).
fn psd_a(pdk: &PdkConfig) {
    let elems = min_width_pattern(layer(pdk, "pSD"), 0.31, 2.0, 3.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.a.gds.gz"), library("TOP", elems));
}

/// pSD.b — min space 0.31 between pSD regions (two clean, two too-close).
fn psd_b(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let elems = space_pattern(p, p, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.b.gds.gz"), library("TOP", elems));
}

/// pSD.d — min space 0.18 of pSD to N+Activ in PWell.  The neighbours are bare Activ
/// (no pSD, no NWell) → they derive as NActivInPWell.
fn psd_d(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let a = layer(pdk, "Activ");
    let elems = space_pattern(p, a, 1.0, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.d.gds.gz"), library("TOP", elems));
}

/// pSD.k — min area 0.25 µm² (a 0.4×0.4 = 0.16 µm² dab fails; a 0.6×0.6 passes).
fn psd_k(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let o = OFFSET;
    let elems = vec![
        rect(p, o, o, o + 0.4, o + 0.4),               // 0.16 µm² → violation
        rect(p, o + 5.0, o, o + 5.6, o + 0.6),         // 0.36 µm² → clean
    ];
    write_gz(&format!("{DIR}/pSD.k.gds.gz"), library("TOP", elems));
}

/// pSD.l — min enclosed (hole) area 0.25 µm²: a pSD frame around a 0.4×0.4 = 0.16 µm² hole.
fn psd_l(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let o = OFFSET;
    // Frame around the hole (o+1, o+1)-(o+1.4, o+1.4); thickness 1.0.
    let elems = vec![
        rect(p, o, o, o + 2.4, o + 1.0),               // bottom
        rect(p, o, o + 1.4, o + 2.4, o + 2.4),         // top
        rect(p, o, o + 1.0, o + 1.0, o + 1.4),         // left
        rect(p, o + 1.4, o + 1.0, o + 2.4, o + 1.4),   // right
    ];
    write_gz(&format!("{DIR}/pSD.l.gds.gz"), library("TOP", elems));
}

/// pSD.g — min. abutted-tie area 0.09 µm², checked on both the N-tap (Activ in NWell) and
/// P-tap (Activ+pSD outside NWell) flavours: a 0.06 µm² tie fails, a 0.12+ µm² one is clean.
fn psd_g(pdk: &PdkConfig) {
    let o = OFFSET;
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let nwell = layer(pdk, "NWell");
    let elems = vec![
        // N-tap: bare Activ (no pSD) inside NWell — the whole tie area is the tiny one, so
        // it doesn't collide with any raw-pSD rule.
        rect(nwell, o - 1.0, o - 1.0, o + 1.0, o + 1.0),
        rect(activ, o, o, o + 0.2, o + 0.3), // 0.06 µm² → violation
        rect(nwell, o + 5.0 - 1.0, o - 1.0, o + 5.0 + 1.0, o + 1.0),
        rect(activ, o + 5.0, o, o + 5.4, o + 0.3), // 0.12 µm² → clean
        // P-tap: a 1.0×1.0 Activ square with pSD extended 0.2 µm past it on every side (so
        // pSD's own boundary never coincides with the NWell-clipped tie edge — avoiding the
        // pre-existing pSD.c coincident-edge false positive, see [[gdscheck-min-enclosure-
        // coincident-bug]]).  NWell covers all of Activ but a small corner sliver — only
        // that NWell-subtracted sliver (Activ+pSD outside NWell) is what pSD.g measures.
        rect(activ, o + 10.0, o, o + 11.0, o + 1.0),
        rect(psd, o + 9.8, o - 0.2, o + 11.2, o + 1.2),
        rect(nwell, o + 9.0, o - 1.0, o + 11.0, o + 0.7),
        rect(nwell, o + 9.0, o - 1.0, o + 10.8, o + 1.0), // sliver left: 0.2×0.3 = 0.06 → violation
        rect(activ, o + 15.0, o, o + 16.0, o + 1.0),
        rect(psd, o + 14.8, o - 0.2, o + 16.2, o + 1.2),
        rect(nwell, o + 14.0, o - 1.0, o + 16.0, o + 0.7),
        rect(nwell, o + 14.0, o - 1.0, o + 15.6, o + 1.0), // sliver left: 0.4×0.3 = 0.12 → clean
    ];
    write_gz(&format!("{DIR}/pSD.g.gds.gz"), library("TOP", elems));
}
