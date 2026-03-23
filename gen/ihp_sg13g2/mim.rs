// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/mim";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    mim_all(pdk);
    mim_gr(pdk);
}

/// A grid of individually-clean caps whose *total* MIM area exceeds the MIM.gR chip cap
/// (174800 µm²): 32 caps of 74×74 µm = 175232 µm², each region 5476 µm² < the 5625 µm²
/// per-device MIM.g limit, so only MIM.gR fires.
fn mim_gr(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    for i in 0..8 {
        for j in 0..4 {
            e.extend(cap(pdk, o + i as f64 * 80.0, o + j as f64 * 80.0, 74.0, 74.0, 0.5, true));
        }
    }
    write_gz(&format!("{DIR}/MIM.gR.gds.gz"), library("TOP", e));
}

/// A MIM cap at `(x, y)`, `w`×`h` µm: Metal5 bottom plate enclosing by 0.70, a TopVia1 at
/// `via_margin` from the MIM corner (omitted if `via == false`).
fn cap(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64, via_margin: f64, via: bool) -> Vec<GdsElement> {
    let mut e = vec![
        rect(layer(pdk, "MIM"), x, y, x + w, y + h),
        rect(layer(pdk, "Metal5"), x - 0.7, y - 0.7, x + w + 0.7, y + h + 0.7),
    ];
    if via {
        let v = layer(pdk, "TopVia1");
        e.push(rect(v, x + via_margin, y + via_margin, x + via_margin + 0.3, y + via_margin + 0.3));
    }
    e
}

/// One layout exercising the MIM rules; each cap perturbs a single aspect.
fn mim_all(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    e.extend(cap(pdk, o, o, 3.0, 3.0, 0.5, true)); // clean
    e.extend(cap(pdk, o + 10.0, o, 3.0, 3.0, 0.2, true)); // via margin 0.20 < 0.36 → MIM.d
    e.extend(cap(pdk, o + 20.0, o, 3.0, 3.0, 0.0, false)); // no via → MIM.h
    e.extend(cap(pdk, o + 30.0, o, 1.0, 3.0, 0.36, true)); // width 1.0 < 1.14 → MIM.a
    e.extend(cap(pdk, o + 40.0, o, 1.2, 1.0, 0.36, true)); // area 1.2 < 1.30 → MIM.f
    // Two caps 0.50 µm apart → MIM.b (their Metal5 overlaps, fine).
    e.extend(cap(pdk, o, o + 12.0, 2.0, 2.0, 0.5, true));
    e.extend(cap(pdk, o + 2.5, o + 12.0, 2.0, 2.0, 0.5, true));
    write_gz(&format!("{DIR}/MIM.gds.gz"), library("TOP", e));
}
