// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/sdiod";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    sdiod_all(pdk);
}

/// A Schottky-diode `schottky_nbl1` device: a bar-shaped Cont (`w`×`h`, becoming ContBar
/// since it isn't square) centered at `(cx, cy)`, enclosed by PWell:block/nSD:block/SalBlock
/// at the given margins, inside a generous nBuLay.
fn device(
    pdk: &PdkConfig, cx: f64, cy: f64, w: f64, h: f64,
    margins: (f64, f64, f64), // (pwb, nsd, sal)
) -> Vec<GdsElement> {
    let (pwb_margin, nsd_margin, sal_margin) = margins;
    let ring = |name: &str, margin: f64| {
        rect(layer(pdk, name), cx - w / 2.0 - margin, cy - h / 2.0 - margin, cx + w / 2.0 + margin, cy + h / 2.0 + margin)
    };
    vec![
        rect(layer(pdk, "Cont"), cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0),
        ring("PWell.block", pwb_margin),
        ring("nSD.block", nsd_margin),
        ring("SalBlock", sal_margin),
        ring("nBuLay", sal_margin + 1.0), // generously covers everything
    ]
}

/// Six instances, 5 µm apart in x: a clean baseline (all margins/dims exactly on target),
/// then one violation per rule (Sdiod.a min, Sdiod.b max, Sdiod.c min, Sdiod.d min-width,
/// Sdiod.e max-length), each isolated to just that one deviation.
fn sdiod_all(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    // Clean: ContBar 0.30×1.00, margins 0.25/0.40/0.45 exactly.
    e.extend(device(pdk, o, o, 0.30, 1.00, (0.25, 0.40, 0.45)));
    // Sdiod.a (min): PWell:block margin only 0.15 (< 0.25).
    e.extend(device(pdk, o + 5.0, o, 0.30, 1.00, (0.15, 0.40, 0.45)));
    // Sdiod.b (max): nSD:block margin 0.55 (> 0.40); SalBlock stays at its own clean 0.45.
    e.extend(device(pdk, o + 10.0, o, 0.30, 1.00, (0.25, 0.55, 0.45)));
    // Sdiod.c (min): SalBlock margin only 0.30 (< 0.45).
    e.extend(device(pdk, o + 15.0, o, 0.30, 1.00, (0.25, 0.40, 0.30)));
    // Sdiod.d (min-width): ContBar 0.20 wide (< 0.30) instead of 0.30.
    e.extend(device(pdk, o + 20.0, o, 0.20, 1.00, (0.25, 0.40, 0.45)));
    // Sdiod.e (max-length): ContBar 1.50 long (> 1.00) instead of 1.00.
    e.extend(device(pdk, o + 25.0, o, 0.30, 1.50, (0.25, 0.40, 0.45)));
    write_gz(&format!("{DIR}/Sdiod.gds.gz"), library("TOP", e));
}
