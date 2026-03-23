// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/sealring";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    seal_ef(pdk);
    seal_b(pdk);
    seal_d(pdk);
}

/// Seal.d — the EdgeSeal-Activ ring must enclose each EdgeSeal-via ring by 1.30.  Two
/// seal frames (EdgeSeal + coincident Activ, 4.0 µm wide): the first has its Cont ring's
/// outer edge only 0.80 from the frame's outer edge (< 1.30 → fires); the second at 1.50
/// (clean).  Both Activ rings are clipped to EdgeSeal, so their coincident frame edges
/// exercise the skip_coincident path.
fn seal_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e: Vec<GdsElement> = Vec::new();
    for (dx, inset) in [(0.0, 0.80), (60.0, 1.50)] {
        let (x0, y0, x1, y1) = (o + dx, o, o + dx + 40.0, o + 40.0);
        e.extend(ring(pdk, "EdgeSeal", x0, y0, x1, y1, 4.0));
        e.extend(ring(pdk, "Activ", x0, y0, x1, y1, 4.0));
        e.extend(ring(pdk, "Cont", x0 + inset, y0 + inset, x1 - inset, y1 - inset, 0.16));
    }
    write_gz(&format!("{DIR}/Seal.d.gds.gz"), library("TOP", e));
}

/// A rectangular ring (frame with a hole) on `layer_name`, as four overlapping strips.
fn ring(pdk: &PdkConfig, name: &str, x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<GdsElement> {
    let l = layer(pdk, name);
    vec![
        rect(l, x0, y0, x1, y0 + w),         // bottom
        rect(l, x0, y1 - w, x1, y1),         // top
        rect(l, x0, y0, x0 + w, y1),         // left
        rect(l, x1 - w, y0, x1, y1),         // right
    ]
}

/// A passivation ring outside the seal: frame 3.0 µm wide (< 4.20 → Seal.e), with its
/// left edge 0.5 µm from a seal-Activ ring (< 1.00 → Seal.f.Activ).
fn seal_ef(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = ring(pdk, "Passiv", o + 5.0, o, o + 25.0, o + 20.0, 3.0);
    // A seal-Activ bar (Activ ∩ EdgeSeal) 0.5 µm left of the ring's outer edge (o+5).
    let sx1 = o + 4.5;
    e.push(rect(layer(pdk, "Activ"), o, o, sx1, o + 20.0));
    e.push(rect(layer(pdk, "EdgeSeal"), o, o, sx1, o + 20.0));
    write_gz(&format!("{DIR}/Seal.ef.gds.gz"), library("TOP", e));
}

/// Seal.b — always "circuit Activ space to EdgeSeal-<conductor>", never <conductor> itself
/// (the KLayout body's first `.sep` argument is always `activ_drw`).  Two blocks:
/// - a seal-Activ block (Activ ∩ EdgeSeal) with a circuit Activ 2.0 µm away (< 4.90 →
///   violation) and a far one (10.0 µm, clean) — confirms the ring's own Activ doesn't
///   self-flag (SealActiv ⊆ Activ; min_space skips overlapping A/B pairs).
/// - a seal-Metal1 block (Metal1 ∩ EdgeSeal) with a circuit **Activ** (not Metal1) 2.0 µm
///   away (< 4.90 → violation), exercising one of the cross-layer Seal<X> variants.
fn seal_b(pdk: &PdkConfig) {
    let o = OFFSET + 40.0;
    let mut e = vec![
        rect(layer(pdk, "Activ"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "Activ"), o + 12.0, o, o + 14.0, o + 10.0), // gap 2.0 → violation
        rect(layer(pdk, "Activ"), o + 20.0, o, o + 22.0, o + 10.0), // gap 10.0 → clean
    ];
    let om = o + 30.0;
    e.push(rect(layer(pdk, "Metal1"), om, o, om + 10.0, o + 10.0));
    e.push(rect(layer(pdk, "EdgeSeal"), om, o, om + 10.0, o + 10.0));
    e.push(rect(layer(pdk, "Activ"), om + 12.0, o, om + 14.0, o + 10.0)); // gap 2.0 → violation
    write_gz(&format!("{DIR}/Seal.b.gds.gz"), library("TOP", e));
}
