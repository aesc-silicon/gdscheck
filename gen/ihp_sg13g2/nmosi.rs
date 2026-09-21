// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nmosi";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nmosi_b(pdk);
    nmosi_c(pdk);
    nmosi_d(pdk);
    nmosi_f(pdk);
    nmosi_g(pdk);
}

fn lr(pdk: &PdkConfig, name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    rect(layer(pdk, name), x0, y0, x1, y1)
}

/// An NWell ring as four rects: the box `(x0, y0)-(x1, y1)` less the hole `(hx0, hy0)-
/// (hx1, hy1)`.  Section 6.5's rules are tested inside a closed ring of NWell AND
/// nBuLay only, so every pattern below sits in one.
#[allow(clippy::too_many_arguments)]
fn nwell_ring(
    pdk: &PdkConfig,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    hx0: f64,
    hy0: f64,
    hx1: f64,
    hy1: f64,
) -> Vec<GdsElement> {
    vec![
        lr(pdk, "NWell", x0, y0, x1, hy0),
        lr(pdk, "NWell", x0, hy1, x1, y1),
        lr(pdk, "NWell", x0, hy0, hx0, hy1),
        lr(pdk, "NWell", hx1, hy0, x1, hy1),
    ]
}

/// nmosi.b — an iso-PWell Activ enclosed by nBuLay by only 1.235 µm (< 1.24), in a
/// ring 0.4 from the Activ that reaches the nBuLay's edge (0.835 wide).
fn nmosi_b(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 10.0, o + 10.0),
        lr(pdk, "Activ", o + 1.235, o + 1.235, o + 8.765, o + 8.765), // inset 1.235 < 1.24
    ];
    e.extend(nwell_ring(
        pdk,
        o,
        o,
        o + 10.0,
        o + 10.0,
        o + 0.835,
        o + 0.835,
        o + 9.165,
        o + 9.165,
    ));
    write_gz(&format!("{DIR}/nmosi.b.gds.gz"), library("TOP", e));
}

/// One nmosi.c instance: an NWell ring (outer half-extent 5.0, width 1.0) on nBuLay,
/// with the iso-PWell Activ sitting `gap` µm inside the ring's hole (hole half 4.0).
fn nmosi_c_ring(pdk: &PdkConfig, cx: f64, cy: f64, gap: f64) -> Vec<GdsElement> {
    let ah = 4.0 - gap;
    let mut e = vec![
        lr(pdk, "nBuLay", cx - 6.5, cy - 6.5, cx + 6.5, cy + 6.5),
        lr(pdk, "Activ", cx - ah, cy - ah, cx + ah, cy + ah),
    ];
    // NWell ring as four rects (outer 5.0, inner 4.0).
    e.push(lr(pdk, "NWell", cx - 5.0, cy - 5.0, cx + 5.0, cy - 4.0)); // bottom
    e.push(lr(pdk, "NWell", cx - 5.0, cy + 4.0, cx + 5.0, cy + 5.0)); // top
    e.push(lr(pdk, "NWell", cx - 5.0, cy - 4.0, cx - 4.0, cy + 4.0)); // left
    e.push(lr(pdk, "NWell", cx + 4.0, cy - 4.0, cx + 5.0, cy + 4.0)); // right
    e
}

/// nmosi.c — min. NWell space to iso-PWell Activ (0.39), rings only:
/// - ring with gap 0.34 → fires;
/// - ring with gap 0.50 → clean;
/// - canary: a plain (hole-free) NWell 0.35 from an iso-PWell Activ → silent, because
///   only `NWell.with_holes` (the isolation ring) anchors this rule (0.35 also clears
///   KLayout's NW.d "NWell space to external N+Activ = 0.31" so the container
///   cross-check stays collateral-free).
fn nmosi_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nmosi_c_ring(pdk, o, o, 0.34);
    e.extend(nmosi_c_ring(pdk, o + 30.0, o, 0.50));
    let cx = o + 60.0;
    e.push(lr(pdk, "nBuLay", cx - 4.0, o - 4.0, cx + 6.0, o + 4.0));
    e.push(lr(pdk, "Activ", cx - 1.5, o - 1.5, cx + 1.5, o + 1.5));
    e.push(lr(pdk, "NWell", cx + 1.85, o - 1.5, cx + 3.85, o + 1.5)); // 0.35 away, no hole
    write_gz(&format!("{DIR}/nmosi.c.gds.gz"), library("TOP", e));
}

/// nmosi.d — the NWell∩nBuLay ring round an iso-PWell Activ only 0.50 µm wide (< 0.62)
/// on its right leg: the ring 0.75 from the Activ, 0.8 wide on three sides, and the
/// nBuLay ending 0.5 into the right leg (1.25 past the Activ, so nmosi.b holds).
fn nmosi_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 8.25, o + 9.0),
        lr(pdk, "Activ", o + 2.0, o + 2.0, o + 7.0, o + 7.0),
    ];
    e.extend(nwell_ring(
        pdk,
        o + 0.45,
        o + 0.45,
        o + 9.0,
        o + 8.55,
        o + 1.25,
        o + 1.25,
        o + 7.75,
        o + 7.75,
    ));
    write_gz(&format!("{DIR}/nmosi.d.gds.gz"), library("TOP", e));
}

/// nmosi.f — an nSD:block strip 0.50 µm wide (< 0.62) touching the iso-PWell Activ.
fn nmosi_f(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 12.0, o + 10.0),
        lr(pdk, "Activ", o + 2.0, o + 2.0, o + 10.0, o + 8.0), // inset 2.0 > 1.24 → nmosi.b clean
        // nSD:block 0.50 wide strip overlapping the Activ.
        lr(pdk, "nSD.block", o + 4.0, o + 2.0, o + 4.5, o + 8.0),
    ];
    // The ring: 0.5 from the Activ, 0.8 wide.
    e.extend(nwell_ring(
        pdk,
        o + 0.7,
        o + 0.7,
        o + 11.3,
        o + 9.3,
        o + 1.5,
        o + 1.5,
        o + 10.5,
        o + 8.5,
    ));
    write_gz(&format!("{DIR}/nmosi.f.gds.gz"), library("TOP", e));
}

/// One nmosi.g instance: an iso-PWell Activ wider than its nSD:block on the right (the
/// ptap area), with SalBlock extending `ext` past the nSD:block edge toward the ptap.
/// nSD:block and SalBlock end flush with Activ on the other three sides (bands there fall
/// outside Activ and are correctly out of scope).
fn nmosi_g_instance(pdk: &PdkConfig, x: f64, y: f64, ext: f64) -> Vec<GdsElement> {
    let mut e = vec![
        lr(pdk, "nBuLay", x - 2.0, y - 2.0, x + 8.0, y + 5.0),
        lr(pdk, "Activ", x, y, x + 5.0, y + 3.0), // x+3..x+5 is the adjacent ptap area
        lr(pdk, "nSD.block", x, y, x + 3.0, y + 3.0),
        lr(pdk, "SalBlock", x, y, x + 3.0 + ext, y + 3.0),
    ];
    // The ring: 0.5 from the Activ, 0.8 wide.
    e.extend(nwell_ring(
        pdk,
        x - 1.3,
        y - 1.3,
        x + 6.3,
        y + 4.3,
        x - 0.5,
        y - 0.5,
        x + 5.5,
        y + 3.5,
    ));
    e
}

/// nmosi.g — SalBlock overlap of nSD:block over Activ ≥ 0.15:
/// - ext 0.05 (< 0.15) → fires (matches KLayout).
/// - ext 0.20 → clean (matches KLayout).
/// - ext 0.00, flush → fires HERE only: overlap 0 violates the PDF text, but KLayout's
///   coincident-pair marker has zero area and vanishes under its `.and(Activ)` — a
///   degenerate-marker artifact we deliberately don't reproduce.
fn nmosi_g(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nmosi_g_instance(pdk, o, o, 0.05);
    e.extend(nmosi_g_instance(pdk, o + 15.0, o, 0.20));
    e.extend(nmosi_g_instance(pdk, o + 30.0, o, 0.00));
    write_gz(&format!("{DIR}/nmosi.g.gds.gz"), library("TOP", e));
}
