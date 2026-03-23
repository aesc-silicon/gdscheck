// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, poly, rect, space_pattern, exact_width_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use std::f64::consts::TAU;

const DIR: &str = "tests/data/ihp-sg13g2/pad";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pad_a1(pdk);
    pad_d(pdk);
    pad_i(pdk);

    padb_a(pdk);
    padb_b(pdk);
    padb_c(pdk);
    padb_d(pdk);
    padc_d(pdk);

    padc_a(pdk);
    padc_b(pdk);
    padc_c(pdk);

    padb_f(pdk);
    padc_f(pdk);
}

/// A regular `n`-gon (multiple of 4, so its bbox is an exact `2r`×`2r` square) centered at
/// `(cx, cy)` with radius `r`, on the given layer.
fn regular_ngon(layer: (i16, i16), cx: f64, cy: f64, r: f64, n: usize) -> gds21::GdsElement {
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    poly(layer, &pts)
}

/// A regular octagon (square with 45°-chamfered corners) of bbox side `s`, `chamfer` cut
/// from each corner, centered at `(cx, cy)`.
fn octagon(layer: (i16, i16), cx: f64, cy: f64, s: f64, chamfer: f64) -> gds21::GdsElement {
    let h = s / 2.0;
    let c = chamfer;
    let pts = [
        (cx - h + c, cy - h), (cx + h - c, cy - h),
        (cx + h, cy - h + c), (cx + h, cy + h - c),
        (cx + h - c, cy + h), (cx - h + c, cy + h),
        (cx - h, cy + h - c), (cx - h, cy - h + c),
    ];
    poly(layer, &pts)
}

/// Padb.f — Allowed SBumpPad shape is Octagon or Circle only.  A plain-square SBumpPad
/// violates; an octagon-shaped and a circle-shaped one are both clean.  Each is fully
/// covered by TopMetal2 and far from EdgeSeal to isolate Padb.f from Padb.c/d/Pad.i.
fn padb_f(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let tm2 = layer(pdk, "TopMetal2");
    let o = OFFSET;
    let cover = |cx: f64, cy: f64, r: f64| rect(tm2, cx - r - 15.0, cy - r - 15.0, cx + r + 15.0, cy + r + 15.0);

    // Square — violation.
    let (cx, cy) = (o, o);
    let mut elems = vec![rect(sbump, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0)];
    elems.push(rect(dfpad, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0));
    elems.push(cover(cx, cy, 30.0));

    // Octagon — clean.
    let (cx, cy) = (o + 100.0, o);
    elems.push(octagon(sbump, cx, cy, 60.0, 15.0));
    elems.push(octagon(dfpad, cx, cy, 60.0, 15.0));
    elems.push(cover(cx, cy, 30.0));

    // Circle (64-gon) — clean.
    let (cx, cy) = (o + 200.0, o);
    elems.push(regular_ngon(sbump, cx, cy, 30.0, 64));
    elems.push(regular_ngon(dfpad, cx, cy, 30.0, 64));
    elems.push(cover(cx, cy, 30.0));

    write_gz(&format!("{DIR}/Padb.f.gds.gz"), library("TOP", elems));
}

/// Padc.f — Allowed CuPillarPad shape is Circle only (unlike Padb.f, an octagon is NOT
/// allowed here).  Square and octagon both violate; only the circle is clean.
fn padc_f(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let tm2 = layer(pdk, "TopMetal2");
    let o = OFFSET + 400.0;
    let cover = |cx: f64, cy: f64, r: f64| rect(tm2, cx - r - 15.0, cy - r - 15.0, cx + r + 15.0, cy + r + 15.0);

    // Square — violation.
    let (cx, cy) = (o, o);
    let mut elems = vec![rect(pillar, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5)];
    elems.push(rect(dfpad, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5));
    elems.push(cover(cx, cy, 17.5));

    // Octagon — violation (not circle).
    let (cx, cy) = (o + 100.0, o);
    elems.push(octagon(pillar, cx, cy, 35.0, 9.0));
    elems.push(octagon(dfpad, cx, cy, 35.0, 9.0));
    elems.push(cover(cx, cy, 17.5));

    // Circle (64-gon) — clean.
    let (cx, cy) = (o + 200.0, o);
    elems.push(regular_ngon(pillar, cx, cy, 17.5, 64));
    elems.push(regular_ngon(dfpad, cx, cy, 17.5, 64));
    elems.push(cover(cx, cy, 17.5));

    write_gz(&format!("{DIR}/Padc.f.gds.gz"), library("TOP", elems));
}

/// A pad opening (`Passiv AND dfpad`) at `(x, y)`, `w`×`h` µm.
fn opening(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Passiv"), x, y, x + w, y + h),
        rect(layer(pdk, "dfpad"), x, y, x + w, y + h),
    ]
}

/// An EdgeSeal-Activ ring fragment (`Activ AND EdgeSeal`) — the seal reference for Pad.d/Padc.d.
fn seal_activ(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Activ"), x, y, x + w, y + h),
        rect(layer(pdk, "EdgeSeal"), x, y, x + w, y + h),
    ]
}

/// Pad.a1 — a 160 µm pad opening exceeds the 150 µm max width.
fn pad_a1(pdk: &PdkConfig) {
    let mut elems = opening(pdk, OFFSET, OFFSET, 160.0, 160.0);
    elems.extend(opening(pdk, OFFSET + 200.0, OFFSET, 100.0, 100.0)); // clean
    write_gz(&format!("{DIR}/Pad.a1.gds.gz"), library("TOP", elems));
}

/// Pad.d — a pad opening 7.0 µm from the EdgeSeal-Activ ring (< 7.50).
fn pad_d(pdk: &PdkConfig) {
    let mut elems = seal_activ(pdk, OFFSET, OFFSET, 10.0, 10.0);
    elems.extend(opening(pdk, OFFSET + 10.0 + 7.0, OFFSET, 30.0, 10.0));
    write_gz(&format!("{DIR}/Pad.d.gds.gz"), library("TOP", elems));
}

/// Pad.i — a dfpad opening with no TopMetal2 underneath (clean pad has TopMetal2 added).
fn pad_i(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        rect(layer(pdk, "Passiv"), o, o, o + 30.0, o + 30.0),
        rect(layer(pdk, "dfpad"), o, o, o + 30.0, o + 30.0), // no TopMetal2 → violation
        rect(layer(pdk, "Passiv"), o + 50.0, o, o + 80.0, o + 30.0),
        rect(layer(pdk, "dfpad"), o + 50.0, o, o + 80.0, o + 30.0),
        rect(layer(pdk, "TopMetal2"), o + 50.0, o, o + 80.0, o + 30.0), // clean
    ];
    write_gz(&format!("{DIR}/Pad.i.gds.gz"), library("TOP", elems));
}

/// Padb.d — an SBumpPad 40.0 µm from raw EdgeSeal (< 50.0 → violation).  TopMetal2 fully
/// (over-)covers the pad so this doesn't collaterally trip Padb.c/Pad.i.
fn padb_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let (x0, x1) = (o + 10.0 + 40.0, o + 10.0 + 40.0 + 60.0);
    let elems = vec![
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "Passiv.sbump"), x0, o, x1, o + 60.0),
        rect(layer(pdk, "dfpad"), x0, o, x1, o + 60.0),
        rect(layer(pdk, "TopMetal2"), x0 - 15.0, o - 15.0, x1 + 15.0, o + 60.0 + 15.0),
    ];
    write_gz(&format!("{DIR}/Padb.d.gds.gz"), library("TOP", elems));
}

/// Padc.d — CuPillarPads 25.0 µm (< 30 → fires) and exactly 30.0 µm (clean) from an
/// EdgeSeal-Activ bar.  TopMetal2 over-covers both pads (keeps Padc.c/Pad.i quiet); the
/// square pads trip Padc.f (circle-only), which the test ignores.
fn padc_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut elems = vec![
        rect(layer(pdk, "Activ"), o, o, o + 10.0, o + 130.0),
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 130.0),
    ];
    // A real pillar-pad cell carries both marker conventions: the PDF's (Passiv:pillar +
    // dfpad — our CuPillarPad recognition) and the maximal deck's Padc.d input
    // (`cupPad_candidat = Passiv ∩ dfpad:pillar`); draw all four so the container
    // cross-check exercises the real rule.
    for (dy, gap) in [(0.0, 25.0), (80.0, 30.0)] {
        let (x0, y0) = (o + 10.0 + gap, o + dy);
        for l in ["Passiv.pillar", "dfpad", "Passiv", "dfpad.pillar"] {
            elems.push(rect(layer(pdk, l), x0, y0, x0 + 35.0, y0 + 35.0));
        }
        elems.push(rect(layer(pdk, "TopMetal2"), x0 - 10.0, y0 - 10.0, x0 + 45.0, y0 + 45.0));
    }
    write_gz(&format!("{DIR}/Padc.d.gds.gz"), library("TOP", elems));
}

fn padb_a(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut exact_width_pattern(sbump, 60.0, 60.0, 160.0, OFFSET, SPACE_DELTA));
    elems.append(&mut exact_width_pattern(dfpad, 60.0, 60.0, 160.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padb.a.gds.gz"), library("TOP", elems));
}

fn padb_b(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut space_pattern(sbump, sbump, 60.0, 70.0, OFFSET, SPACE_DELTA));
    elems.append(&mut space_pattern(dfpad, dfpad, 60.0, 70.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padb.b.gds.gz"), library("TOP", elems));
}

fn padb_c(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let tm = layer(pdk, "TopMetal2");
    let mut elems = vec![];
    elems.append(&mut enclosure_pattern(tm, sbump, 10.0, 60.0, 70.0, OFFSET, SPACE_DELTA));
    elems.append(&mut enclosure_pattern(tm, dfpad, 10.0, 60.0, 70.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padb.c.gds.gz"), library("TOP", elems));
}

fn padc_a(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut exact_width_pattern(pillar, 35.0, 35.0, 160.0, OFFSET, SPACE_DELTA));
    elems.append(&mut exact_width_pattern(dfpad, 35.0, 35.0, 160.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padc.a.gds.gz"), library("TOP", elems));
}

fn padc_b(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut space_pattern(pillar, pillar, 35.0, 40.0, OFFSET, SPACE_DELTA));
    elems.append(&mut space_pattern(dfpad, dfpad, 35.0, 40.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padc.b.gds.gz"), library("TOP", elems));
}

fn padc_c(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let tm = layer(pdk, "TopMetal2");
    let mut elems = vec![];
    elems.append(&mut enclosure_pattern(tm, pillar, 7.5, 35.0, 70.0, OFFSET, SPACE_DELTA));
    elems.append(&mut enclosure_pattern(tm, dfpad, 7.5, 35.0, 70.0, OFFSET, SPACE_DELTA));
    write_gz(&format!("{DIR}/Padc.c.gds.gz"), library("TOP", elems));
}
