// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, text, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/npn";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    npn_ties(pdk);
    npn_emitters(pdk);
}

/// A rectangular ring centred at `(cx, cy)` with outer half-extent `oh` and width `w`.
fn ring(pdk: &PdkConfig, name: &str, cx: f64, cy: f64, oh: f64, w: f64) -> Vec<GdsElement> {
    let l = layer(pdk, name);
    let ih = oh - w;
    vec![
        rect(l, cx - oh, cy - oh, cx + oh, cy - ih), // bottom
        rect(l, cx - oh, cy + ih, cx + oh, cy + oh), // top
        rect(l, cx - oh, cy - ih, cx - ih, cy + ih), // left
        rect(l, cx + ih, cy - ih, cx + oh, cy + ih), // right
    ]
}

/// The core of a recognised npn device at `(cx, cy)`: a TRANS square (half-extent `th`),
/// its flavour text label, an emitter window `ww`×`wl` and the E-labelled Metal2 pin
/// (which KLayout's flavour recognition additionally requires; ours doesn't need it but
/// the fixture carries it so the container cross-check exercises the real rules).
fn core(pdk: &PdkConfig, cx: f64, cy: f64, th: f64, label: &str, ww: f64, wl: f64) -> Vec<GdsElement> {
    vec![
        rect(layer(pdk, "TRANS"), cx - th, cy - th, cx + th, cy + th),
        text(layer(pdk, "TEXT"), label, cx - th + 0.1, cy - th + 0.1),
        rect(layer(pdk, "EmWind"), cx - ww / 2.0, cy - wl / 2.0, cx + ww / 2.0, cy + wl / 2.0),
        rect(layer(pdk, "Metal2.pin"), cx - 0.1, cy - 0.1, cx + 0.1, cy + 0.1),
        text(layer(pdk, "TEXT"), "E", cx, cy),
    ]
}

/// Substrate-tie (npnG2.*) cases:
/// - clean device: pSD/Activ ring (0.20 margins), TRANS with 1.0 hole margin, 0.9 emitter.
/// - npnG2.b: an "npn*"-labelled ring with NO TRANS inside.
/// - npnG2.d + npnG2.e: an NWell 1.0 µm and a Cont 0.20 µm from the tie'd TRANS.
/// - npnG2.c: the ring's Activ enclosed by pSD by only 0.05 (< 0.20).
fn npn_ties(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e: Vec<GdsElement> = Vec::new();

    // Clean device.
    let (cx, cy) = (o, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.3, 0.6));
    e.extend(core(pdk, cx, cy, 1.5, "npn13G2", 0.07, 0.9));

    // npnG2.b: labelled tie, no TRANS.
    let (cx, cy) = (o + 30.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.3, 0.6));
    e.push(text(layer(pdk, "TEXT"), "npnCustom", cx, cy));

    // npnG2.d + npnG2.e: bigger hole so the offenders fit inside it.
    let (cx, cy) = (o + 60.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 4.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 4.3, 0.6));
    e.extend(core(pdk, cx, cy, 1.5, "npn13G2", 0.07, 0.9));
    e.push(rect(layer(pdk, "NWell"), cx + 2.5, cy - 1.0, cx + 3.3, cy + 1.0)); // 1.00 < 1.21
    e.push(rect(layer(pdk, "Cont"), cx - 0.08, cy - 1.5 - 0.2 - 0.16, cx + 0.08, cy - 1.5 - 0.2)); // 0.20 < 0.27

    // npnG2.c: Activ ring margins 0.05 (< 0.20) inside the pSD ring.  TRANS exactly
    // fills the hole (flush with the pSD ring's inner edge) because KLayout anchors
    // this rule on TRANS touching the ring — otherwise the container cross-check
    // would not exercise it.
    let (cx, cy) = (o + 90.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.45, 0.9));
    e.extend(core(pdk, cx, cy, 2.5, "npn13G2", 0.07, 0.9));

    write_gz(&format!("{DIR}/npnG2.gds.gz"), library("TOP", e));
}

/// Emitter-length (npn13G2*) cases — one minimal device (TRANS + label + pin + window)
/// per case; the 0.8-long G2L window is also the exact-text canary (it must NOT draw
/// npn13G2.a, whose (0.07, 0.9) band it would fall into if "npn13G2" glob-matched it).
fn npn_emitters(pdk: &PdkConfig) {
    let o = OFFSET;
    let y = o + 30.0;
    let mut e: Vec<GdsElement> = Vec::new();
    e.extend(core(pdk, o, y, 1.5, "npn13G2", 0.07, 0.7)); // npn13G2.a (min)
    e.extend(core(pdk, o + 30.0, y, 1.5, "npn13G2", 0.07, 1.2)); // npn13G2.a (max; ours only)
    e.extend(core(pdk, o + 60.0, y, 1.5, "npn13G2L", 0.07, 0.8)); // npn13G2L.a + canary
    e.extend(core(pdk, o + 90.0, y, 2.0, "npn13G2L", 0.07, 3.0)); // npn13G2L.b
    e.extend(core(pdk, o + 120.0, y, 1.5, "npn13G2V", 0.12, 0.8)); // npn13G2V.a
    e.extend(core(pdk, o + 150.0, y, 3.5, "npn13G2V", 0.12, 6.0)); // npn13G2V.b
    e.extend(core(pdk, o + 180.0, y, 1.5, "npn13G2", 0.07, 0.9)); // clean G2
    write_gz(&format!("{DIR}/npn13G2.gds.gz"), library("TOP", e));
}
