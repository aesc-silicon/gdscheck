// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The recommended (`*R`) pad and npn rules.  TM2.bR's and MIM.gR's fixtures come from
//! their layer's generator, into the same `recommended/` tree.

use super::OFFSET;
use crate::helpers::{layer, library, rect, text, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const PAD: &str = "tests/data/ihp-sg13g2/recommended/pad";
const NPN: &str = "tests/data/ihp-sg13g2/recommended/npn";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(PAD).expect("failed to create output directory");
    std::fs::create_dir_all(NPN).expect("failed to create output directory");

    pad_ar(pdk);
    pad_br(pdk);
    pad_dr(pdk);
    pad_d1r(pdk);
    pad_gr(pdk);
    pad_jr(pdk);
    pad_kr(pdk);

    npn_counts(pdk);
}

fn write(dir: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{dir}/{name}.gds.gz"), library("TOP", elems));
}

/// A pad opening (Passiv AND dfpad) with TopMetal2 under it.
fn opening(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    ["Passiv", "dfpad", "TopMetal2"]
        .iter()
        .map(|l| rect(layer(pdk, l), x, y, x + w, y + h))
        .collect()
}

/// Pad.aR — min. 30 pad width: a 29.995 square and a 29.995 × 60 bar fire, a 30 square
/// and a 30 × 60 bar are clean.
fn pad_ar(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = opening(pdk, o, o, 29.995, 29.995);
    e.extend(opening(pdk, o + 50.0, o, 29.995, 60.0));
    e.extend(opening(pdk, o + 100.0, o, 30.0, 30.0));
    e.extend(opening(pdk, o + 150.0, o, 30.0, 60.0));
    write(PAD, "Pad.aR", e);
}

/// Pad.bR — min. 8.4 pad space: two 40 openings 8.395 apart fire, 8.4 apart are clean;
/// corner to corner 5.93 × 5.93 (8.386 diagonal) fires, 6 × 6 (8.485) is clean.
fn pad_br(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = opening(pdk, o, o, 40.0, 40.0);
    e.extend(opening(pdk, o + 48.395, o, 40.0, 40.0));
    e.extend(opening(pdk, o + 200.0, o, 40.0, 40.0));
    e.extend(opening(pdk, o + 248.4, o, 40.0, 40.0));
    e.extend(opening(pdk, o, o + 200.0, 40.0, 40.0));
    e.extend(opening(pdk, o + 45.93, o + 245.93, 40.0, 40.0));
    e.extend(opening(pdk, o + 200.0, o + 200.0, 40.0, 40.0));
    e.extend(opening(pdk, o + 246.0, o + 246.0, 40.0, 40.0));
    write(PAD, "Pad.bR", e);
}

/// The seal's Activ (Activ AND EdgeSeal) as a box.
fn seal_activ(pdk: &PdkConfig, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![
        rect(layer(pdk, "Activ"), x0, y0, x1, y1),
        rect(layer(pdk, "EdgeSeal"), x0, y0, x1, y1),
    ]
}

/// Pad.dR — min. 25 pad space to the seal's Activ: 24.995 fires, 25 is clean; corner to
/// corner 17.67 × 17.67 (24.989) fires, 17.68 × 17.68 (25.003) is clean - the reach is
/// round, not a square's corner; plain Activ (no EdgeSeal) at 20 is Pad.d1R's.
fn pad_dr(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = seal_activ(pdk, o, o, o + 10.0, o + 40.0);
    e.extend(opening(pdk, o + 34.995, o, 40.0, 40.0));
    e.extend(seal_activ(pdk, o + 200.0, o, o + 210.0, o + 40.0));
    e.extend(opening(pdk, o + 235.0, o, 40.0, 40.0));
    e.extend(seal_activ(pdk, o, o + 200.0, o + 10.0, o + 210.0));
    e.extend(opening(pdk, o + 27.67, o + 227.67, 40.0, 40.0));
    e.extend(seal_activ(pdk, o + 200.0, o + 200.0, o + 210.0, o + 210.0));
    e.extend(opening(pdk, o + 227.68, o + 227.68, 40.0, 40.0));
    e.push(rect(layer(pdk, "Activ"), o, o + 400.0, o + 10.0, o + 440.0));
    e.extend(opening(pdk, o + 30.0, o + 400.0, 40.0, 40.0));
    write(PAD, "Pad.dR", e);
}

/// Pad.d1R — min. 11.2 pad space to Activ inside the chip: 11.195 fires, 11.2 is clean;
/// corner to corner 7.915 × 7.915 (11.194) fires, 7.925 × 7.925 (11.208) is clean;
/// Activ under the opening fires; the seal's Activ (Activ AND EdgeSeal) at 11 is not
/// this rule's (Pad.dR fires on it).
fn pad_d1r(pdk: &PdkConfig) {
    let o = OFFSET;
    let act = layer(pdk, "Activ");
    let mut e = vec![rect(act, o, o, o + 5.0, o + 40.0)];
    e.extend(opening(pdk, o + 16.195, o, 40.0, 40.0));
    e.push(rect(act, o + 200.0, o, o + 205.0, o + 40.0));
    e.extend(opening(pdk, o + 216.2, o, 40.0, 40.0));
    e.extend(opening(pdk, o, o + 200.0, 40.0, 40.0));
    e.push(rect(act, o + 10.0, o + 210.0, o + 15.0, o + 215.0));
    e.extend(seal_activ(pdk, o + 200.0, o + 200.0, o + 205.0, o + 240.0));
    e.extend(opening(pdk, o + 216.0, o + 200.0, 40.0, 40.0));
    e.push(rect(act, o, o + 400.0, o + 5.0, o + 405.0));
    e.extend(opening(pdk, o + 12.915, o + 412.915, 40.0, 40.0));
    e.push(rect(act, o + 200.0, o + 400.0, o + 205.0, o + 405.0));
    e.extend(opening(pdk, o + 212.925, o + 412.925, 40.0, 40.0));
    write(PAD, "Pad.d1R", e);
}

/// Pad.gR — min. 1.4 TopMetal1 (within dfpad) enclosure of TopVia2.  A dfpad 60 wide
/// around a 30 opening, TopMetal1 over the whole dfpad; vias in the ring between the
/// opening and the dfpad edge: 1.395 from the edge fires, 1.4 is clean.  TopMetal1
/// running on past the dfpad edge does not help (the margin is read within dfpad), and a
/// via outside every dfpad is not this rule's.
fn pad_gr(pdk: &PdkConfig) {
    let o = OFFSET;
    let (tv2, tm1, df) = (
        layer(pdk, "TopVia2"),
        layer(pdk, "TopMetal1"),
        layer(pdk, "dfpad"),
    );
    let via = |x: f64, y: f64| rect(tv2, x, y, x + 0.9, y + 0.9);
    let mut e = vec![rect(df, o, o, o + 60.0, o + 60.0)];
    e.extend(opening(pdk, o + 15.0, o + 15.0, 30.0, 30.0));
    e.push(rect(tm1, o, o, o + 60.0, o + 60.0));
    e.push(via(o + 1.395, o + 30.0));
    e.push(via(o + 60.0 - 1.4 - 0.9, o + 30.0));
    // TopMetal1 past the dfpad edge: still 1.395 inside dfpad.
    let x = o + 200.0;
    e.push(rect(df, x, o, x + 60.0, o + 60.0));
    e.extend(opening(pdk, x + 15.0, o + 15.0, 30.0, 30.0));
    e.push(rect(tm1, x - 20.0, o - 20.0, x + 80.0, o + 80.0));
    e.push(via(x + 1.395, o + 30.0));
    e.push(via(x + 30.0, o + 1.4));
    // No dfpad at all.
    e.push(rect(tm1, o, o + 200.0, o + 10.0, o + 210.0));
    e.push(via(o + 0.5, o + 200.5));
    write(PAD, "Pad.gR", e);
}

/// Pad.jR — no devices under a pad: a gate (Activ crossing GatPoly) and a MIM under the
/// opening each fire; a gate and a MIM under dfpad but beside the opening are clean.
fn pad_jr(pdk: &PdkConfig) {
    let o = OFFSET;
    let (act, gp, mim, df) = (
        layer(pdk, "Activ"),
        layer(pdk, "GatPoly"),
        layer(pdk, "MIM"),
        layer(pdk, "dfpad"),
    );
    let gate = |x: f64, y: f64| {
        vec![
            rect(act, x, y + 1.0, x + 3.0, y + 2.0),
            rect(gp, x + 1.0, y, x + 1.5, y + 3.0),
        ]
    };
    let mut e = vec![rect(df, o, o, o + 60.0, o + 60.0)];
    e.extend(opening(pdk, o + 15.0, o + 15.0, 30.0, 30.0));
    e.extend(gate(o + 20.0, o + 20.0));
    e.push(rect(mim, o + 30.0, o + 30.0, o + 35.0, o + 35.0));
    e.extend(gate(o + 2.0, o + 2.0));
    e.push(rect(mim, o + 50.0, o + 50.0, o + 55.0, o + 55.0));
    write(PAD, "Pad.jR", e);
}

/// Pad.kR — no TopVia2 under a pad: one under the opening fires, one straddling its edge
/// fires, one under dfpad beside the opening is clean.
fn pad_kr(pdk: &PdkConfig) {
    let o = OFFSET;
    let (tv2, df) = (layer(pdk, "TopVia2"), layer(pdk, "dfpad"));
    let via = |x: f64, y: f64| rect(tv2, x, y, x + 0.9, y + 0.9);
    let mut e = vec![rect(df, o, o, o + 60.0, o + 60.0)];
    e.extend(opening(pdk, o + 15.0, o + 15.0, 30.0, 30.0));
    e.push(via(o + 30.0, o + 30.0));
    e.push(via(o + 14.5, o + 20.0));
    e.push(via(o + 5.0, o + 5.0));
    write(PAD, "Pad.kR", e);
}

/// One npn13G2* device box labeled `label`, holding `n` emitter windows on `window`,
/// 0.2 square at a 0.5 pitch in rows of 64.  The grid starts 0.35 past the offset so a
/// column straddles the 20 µm tile line at x = 40: a window a tile line cuts counts once.
fn device(pdk: &PdkConfig, x: f64, y: f64, label: &str, window: &str, n: usize) -> Vec<GdsElement> {
    let (trans, txt, win) = (layer(pdk, "TRANS"), layer(pdk, "TEXT"), layer(pdk, window));
    let rows = n.div_ceil(64);
    let mut e = vec![
        rect(trans, x, y, x + 33.0, y + 0.5 * rows as f64 + 1.0),
        text(txt, label, x + 0.1, y + 0.1),
    ];
    for k in 0..n {
        let (cx, cy) = (
            x + 0.35 + 0.5 * (k % 64) as f64,
            y + 0.5 + 0.5 * (k / 64) as f64,
        );
        e.push(rect(win, cx, cy, cx + 0.2, cy + 0.2));
    }
    e
}

/// npn13G2.bR / npn13G2L.cR / npn13G2V.cR — at most 4000, 800 and 800 emitters per chip.
/// One file per count, each over and at the cap.  The L and V labels are not the plain
/// device's (exact label match), and the V emitters are drawn on EmWiHV, as IHP's pcell
/// draws them.
fn npn_counts(pdk: &PdkConfig) {
    let o = OFFSET;
    write(
        NPN,
        "npn13G2.bR",
        device(pdk, o, o, "npn13G2", "EmWind", 4001),
    );
    write(
        NPN,
        "npn13G2.bR.ok",
        device(pdk, o, o, "npn13G2", "EmWind", 4000),
    );
    write(
        NPN,
        "npn13G2L.cR",
        device(pdk, o, o, "npn13G2L", "EmWind", 801),
    );
    write(
        NPN,
        "npn13G2L.cR.ok",
        device(pdk, o, o, "npn13G2L", "EmWind", 800),
    );
    write(
        NPN,
        "npn13G2V.cR",
        device(pdk, o, o, "npn13G2V", "EmWiHV", 801),
    );
    write(
        NPN,
        "npn13G2V.cR.ok",
        device(pdk, o, o, "npn13G2V", "EmWiHV", 800),
    );
    // 3990 plain and 801 L windows: the L count fires, the plain one does not.
    let mut e = device(pdk, o, o, "npn13G2", "EmWind", 3990);
    e.extend(device(pdk, o + 100.0, o, "npn13G2L", "EmWind", 801));
    write(NPN, "npn13G2.mixed", e);
}
