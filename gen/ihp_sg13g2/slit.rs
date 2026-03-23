// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, rect, write_gz, library};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;

const DIR: &str = "tests/data/ihp-sg13g2/slit";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    slt_a(pdk);
    slt_b(pdk);
    slt_c(pdk);
    slt_e(pdk);
    slt_f(pdk);
    slt_h1(pdk);
    slt_i(pdk);
}

/// A square Metal1 plate with one slit slot inside it.  `sw`×`sh` slit centred in a
/// `plate`×`plate` plate (≥ 1 µm enclosure as long as the slit is small enough).
fn plated_slit(m: (i16, i16), s: (i16, i16), ox: f64, oy: f64, plate: f64, sw: f64, sh: f64) -> Vec<GdsElement> {
    let cx = ox + plate / 2.0;
    let cy = oy + plate / 2.0;
    vec![
        rect(m, ox, oy, ox + plate, oy + plate),
        rect(s, cx - sw / 2.0, cy - sh / 2.0, cx + sw / 2.0, cy + sh / 2.0),
    ]
}

/// Slt.a — min. slit width 2.80.  Clean 2.80-wide slit, plus one only 2.795 wide.
fn slt_a(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 12.0, 2.80, 6.0); // clean
    elems.extend(plated_slit(m, s, o + 20.0, o, 12.0, 2.795, 6.0)); // too narrow → Slt.a
    write_gz(&format!("{DIR}/Slt.a.gds.gz"), library("TOP", elems));
}

/// Slt.b — max. slit width 20.00.  Clean 20.0-wide slit, plus one 20.005 wide.
fn slt_b(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Plate 24 wide keeps width < 30 (no Slt.c) and < 35 (no Slt.i); 1 µm enclosure.
    let mut elems = plated_slit(m, s, o, o, 24.0, 20.0, 6.0); // clean
    elems.extend(plated_slit(m, s, o + 40.0, o, 24.0, 20.005, 6.0)); // too wide → Slt.b
    write_gz(&format!("{DIR}/Slt.b.gds.gz"), library("TOP", elems));
}

/// Slt.c — max. metal width without a slit 30.00.  A 32×32 plate with no slit is wider than
/// 30 µm everywhere → violation; an identical plate with a slit is clean; a 20-µm-wide wire
/// is never wide enough to need one.
fn slt_c(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Wide plate, no slit → Slt.c (32 < 35, so no Slt.i).
    let mut elems = vec![rect(m, o, o, o + 32.0, o + 32.0)];
    // Wide plate with a centred slit → clean.
    let bx = o + 40.0;
    elems.extend(plated_slit(m, s, bx, o, 32.0, 2.80, 6.0));
    // 20-µm-wide wire (≤ 30 wide) → opened away, never flagged.
    elems.push(rect(m, o, o + 40.0, o + 20.0, o + 100.0));
    // Self-slotted plate (as IO power buses are drawn): a 40-µm footprint cut into 6-µm
    // bars by 2-µm slots in the metal itself.  Every piece is < 30 µm, so it must stay
    // clean — the erosion would fill the slots, but the exact disk check sees them.
    let sx = o + 100.0;
    for i in 0..6 {
        let y = o + i as f64 * 8.0;
        elems.push(rect(m, sx, y, sx + 40.0, y + 6.0));
    }
    write_gz(&format!("{DIR}/Slt.c.gds.gz"), library("TOP", elems));
}

/// Slt.e — no slits on pads:
/// - a pad (TopMetal2 + dfpad + Passiv opening) with a TopMetal2 slit AND a Metal1
///   plate with a slit under it → fires twice (once per metal — the pad region is the
///   whole dfpad shape, on every layer);
/// - a plain slotted TopMetal2 plate away from any pad → clean;
/// - canary: a dfpad with NO Passiv opening (not a pad) over a slotted plate → clean.
fn slt_e(pdk: &PdkConfig) {
    let tm2 = layer(pdk, "TopMetal2");
    let ts = layer(pdk, "TopMetal2.slit");
    let o = OFFSET;
    let c = o + 45.0;
    let mut elems = vec![
        rect(tm2, o, o, o + 90.0, o + 90.0),
        rect(layer(pdk, "dfpad"), c - 40.0, c - 40.0, c + 40.0, c + 40.0),
        rect(layer(pdk, "Passiv"), c - 35.0, c - 35.0, c + 35.0, c + 35.0),
        rect(ts, c - 20.0, c - 5.0, c - 17.0, c + 5.0), // on the pad → fires
    ];
    elems.extend(plated_slit(
        layer(pdk, "Metal1"), layer(pdk, "Metal1.slit"),
        c + 5.0, c - 12.5, 25.0, 3.0, 10.0,
    )); // under the same pad → fires
    elems.extend(plated_slit(tm2, ts, o + 120.0, o, 25.0, 3.0, 10.0)); // no pad → clean
    let bx = o + 160.0;
    elems.extend(plated_slit(tm2, ts, bx, o, 25.0, 3.0, 10.0));
    elems.push(rect(layer(pdk, "dfpad"), bx + 2.5, o + 2.5, bx + 22.5, o + 22.5)); // no Passiv → clean
    write_gz(&format!("{DIR}/Slt.e.gds.gz"), library("TOP", elems));
}

/// Slt.f — min. metal enclosure of slit 1.00.  Clean 1.0-µm enclosure, plus one where the
/// slit reaches 0.995 µm from the plate's right edge.
fn slt_f(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 12.0, 2.80, 6.0); // 4.6 µm enclosure → clean
    // Plate 6 wide, slit 2.8 wide pushed right so right enclosure = 0.995 < 1.00.
    let bx = o + 20.0;
    elems.push(rect(m, bx, o, bx + 6.0, o + 8.0));
    elems.push(rect(s, bx + 6.0 - 0.995 - 2.8, o + 1.0, bx + 6.0 - 0.995, o + 7.0));
    write_gz(&format!("{DIR}/Slt.f.gds.gz"), library("TOP", elems));
}

/// Slt.h1 — min. Metal1:slit space to Cont and Via1 0.30.  A slit 0.295 µm from a Via1.
fn slt_h1(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let v = layer(pdk, "Via1");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 16.0, 2.80, 6.0);
    // Via1 just 0.295 µm to the right of the slit's right edge (slit right = cx + 1.4).
    let cx = o + 8.0;
    let sright = cx + 1.4;
    elems.push(rect(v, sright + 0.295, o + 6.0, sright + 0.295 + 0.19, o + 6.19));
    write_gz(&format!("{DIR}/Slt.h1.gds.gz"), library("TOP", elems));
}

/// Slt.i — min. slit density 6 % on metal plates > 35×35 µm.  A 40×40 plate with a single
/// small slit (≈1 %) fails; a 40×40 plate slit at ≥6 % is clean.
fn slt_i(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Starved plate: 40×40 = 1600 µm²; one 2.8×6 = 16.8 µm² slit → ~1.05 % → Slt.i.
    let mut elems = vec![rect(m, o, o, o + 40.0, o + 40.0)];
    elems.push(rect(s, o + 18.6, o + 17.0, o + 21.4, o + 23.0));
    // Healthy plate: 40×40 with slits totalling ≥ 6 % (96 µm²).  Six 3×6 slits = 108 µm².
    let bx = o + 50.0;
    elems.push(rect(m, bx, o, bx + 40.0, o + 40.0));
    for i in 0..3 {
        for j in 0..2 {
            let sx = bx + 6.0 + i as f64 * 12.0;
            let sy = o + 8.0 + j as f64 * 18.0;
            elems.push(rect(s, sx, sy, sx + 3.0, sy + 6.0));
        }
    }
    write_gz(&format!("{DIR}/Slt.i.gds.gz"), library("TOP", elems));
}
