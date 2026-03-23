// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/tgo";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    tgo_a(pdk);
    tgo_b(pdk);
    tgo_c(pdk);
    tgo_d(pdk);
    tgo_e(pdk);
    tgo_f(pdk);
}

/// TGO.e — min. space between ThickGateOx regions 0.86 µm.
fn tgo_e(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let elems = space_pattern(tgo, tgo, 2.0, 0.86, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.e.gds.gz"), library("TOP", elems));
}

/// TGO.a — ThickGateOx must extend ≥ 0.27 µm over Activ.  A TGO covering an Activ with
/// 0.27 µm all round (clean) and one extending only 0.26 µm on the left (violation).
fn tgo_a(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o + 0.3, o + 0.3, o + 1.3, o + 1.3),
        rect(tgo, o + 0.03, o + 0.03, o + 1.57, o + 1.57), // 0.27 all round → clean
        rect(activ, o + 4.0, o + 4.0, o + 5.0, o + 5.0),
        rect(tgo, o + 4.0 - 0.26, o + 4.0 - 0.27, o + 5.0 + 0.27, o + 5.0 + 0.27), // 0.26 left → fail
    ];
    write_gz(&format!("{DIR}/TGO.a.gds.gz"), library("TOP", elems));
}

/// TGO.b — min. ThickGateOx space to Activ outside the TGO region 0.27 µm.
fn tgo_b(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let elems = space_pattern(tgo, activ, 1.0, 0.27, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.b.gds.gz"), library("TOP", elems));
}

/// A transistor (Activ + a gate crossing it) at `(x, y)`, Activ `aw`×1 µm, gate 0.4 µm
/// wide centred, extending 0.5 µm past the Activ top/bottom.
fn transistor(activ: (i16, i16), gp: (i16, i16), x: f64, y: f64, aw: f64) -> Vec<gds21::GdsElement> {
    let gx = x + aw / 2.0 - 0.2;
    vec![
        rect(activ, x, y, x + aw, y + 1.0),
        rect(gp, gx, y - 0.5, gx + 0.4, y + 1.5),
    ]
}

/// TGO.c — ThickGateOx∩Activ must extend ≥ 0.34 µm past the GatPoly sides (the gate's
/// source/drain-facing edges); the cover is clipped to Activ, so gate caps and endcaps
/// poking past the active are exempt.  Clean device: a centred gate with wide source/drain
/// so the TGO-over-active reaches ≥ 0.34 past each side.  Fail device: a gate only 0.20 µm
/// from the Activ's left edge, so the TGO-over-active extends just 0.20 µm past that side.
fn tgo_c(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let mut elems = transistor(activ, gp, o, o, 2.0);
    elems.push(rect(tgo, o - 0.34, o - 0.34, o + 2.34, o + 1.34)); // 0.8 S/D each side → clean

    // Fail device: Activ o+6.0..o+8.0, gate left side at o+6.20 (0.20 µm of source/drain).
    // TGO covers the whole Activ + 0.34 (TGO.a clean), but TGO∩Activ extends only 0.20 µm
    // past the gate's left side before the Activ ends → TGO.c violation on that side.
    let (ax0, ax1) = (o + 6.0, o + 8.0);
    elems.push(rect(activ, ax0, o, ax1, o + 1.0));
    elems.push(rect(gp, ax0 + 0.2, o - 0.5, ax0 + 0.6, o + 1.5)); // left side 0.20 from Activ edge
    elems.push(rect(tgo, ax0 - 0.34, o - 0.34, ax1 + 0.34, o + 1.34)); // covers Activ + 0.34
    write_gz(&format!("{DIR}/TGO.c.gds.gz"), library("TOP", elems));
}

/// TGO.d — min. ThickGateOx space to gate-over-channel outside the TGO region 0.34 µm.
/// A TGO and a separate transistor whose channel sits 0.34 µm away (clean) / 0.33 (fail).
fn tgo_d(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    // Gate at the left edge of its Activ, so channel-to-TGO == Activ-to-TGO.
    let dev = |x: f64| {
        vec![
            rect(activ, x, o, x + 1.0, o + 0.5),
            rect(gp, x, o - 0.3, x + 0.16, o + 0.8), // channel left edge = x
        ]
    };
    let mut elems = vec![rect(tgo, o, o, o + 1.0, o + 1.0)];
    elems.extend(dev(o + 1.0 + 0.34)); // channel 0.34 from TGO → clean
    elems.push(rect(tgo, o + 5.0, o, o + 6.0, o + 1.0));
    elems.extend(dev(o + 6.0 + 0.33)); // channel 0.33 from TGO → violation
    write_gz(&format!("{DIR}/TGO.d.gds.gz"), library("TOP", elems));
}

fn tgo_f(pdk: &PdkConfig) {
    let l = layer(pdk, "ThickGateOx");
    let elems = min_width_pattern(l, 0.86, 0.86, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.f.gds.gz"), library("TOP", elems));
}
