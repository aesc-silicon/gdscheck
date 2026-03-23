// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/resistor";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    rsil(pdk);
    rsil_body(pdk);
    rsil_c(pdk);
    rppd_body(pdk);
    rppd_c(pdk);
    rhigh(pdk);
    rhi_d(pdk);
    rhi_b(pdk);
}

fn lr(pdk: &PdkConfig, name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    rect(layer(pdk, name), x0, y0, x1, y1)
}

/// Rsil: a too-narrow RES box (Rsil.f) and a RES box too close to a Cont (Rsil.b).
fn rsil(pdk: &PdkConfig) {
    let o = OFFSET;
    let e: Vec<GdsElement> = vec![
        lr(pdk, "RES", o, o, o + 0.40, o + 3.0),                       // Rsil.f (narrow RES)
        lr(pdk, "RES", o + 5.0, o, o + 5.60, o + 3.0),                 // clean RES, Cont 0.10 away
        lr(pdk, "Cont", o + 5.70, o, o + 5.86, o + 0.16),             // → Rsil.b
    ];
    write_gz(&format!("{DIR}/Rsil.gds.gz"), library("TOP", e));
}

/// A full (narrow) Rsil resistor: GatPoly+RES inside an EXTBlock, no SalBlock/NWell/nBuLay.
/// Body 0.40 wide → Rsil.a (and Rsil.f on the RES); EXTBlock margin 0.10 < 0.18 → Rsil.e.
fn rsil_body(pdk: &PdkConfig) {
    let o = OFFSET;
    let e = vec![
        lr(pdk, "GatPoly", o, o, o + 0.40, o + 3.0),
        lr(pdk, "RES", o, o, o + 0.40, o + 3.0),
        lr(pdk, "EXTBlock", o - 0.10, o - 0.10, o + 0.50, o + 3.10),
        // pSD 0.10 µm from the resistor GatPoly (< 0.18) → Rsil.d / pSD.m.
        lr(pdk, "pSD", o + 0.50, o, o + 1.0, o + 3.0),
    ];
    write_gz(&format!("{DIR}/RsilBody.gds.gz"), library("TOP", e));
}

/// Rsil.c — RES (a back-annotation marker, not physical geometry) protrudes 0.3 µm past
/// the resistor's GatPoly body on the left.  First instance: uncovered → violation.
/// Second instance: the same protrusion, but a Cont sits inside the protruding RES
/// sliver (so it overlaps RES directly and doesn't itself trip Rsil.b) → clean.
fn rsil_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "RES", o - 0.3, o, o + 1.0, o + 3.0), // protrudes 0.3 past GatPoly, left
        lr(pdk, "GatPoly", o, o, o + 1.0, o + 3.0),
        lr(pdk, "EXTBlock", o - 0.6, o - 0.3, o + 1.3, o + 3.3),
    ];
    let o2 = o + 10.0;
    e.extend(vec![
        lr(pdk, "RES", o2 - 0.3, o, o2 + 1.0, o + 3.0), // same protrusion
        lr(pdk, "GatPoly", o2, o, o2 + 1.0, o + 3.0),
        lr(pdk, "EXTBlock", o2 - 0.6, o - 0.3, o2 + 1.3, o + 3.3),
        // Sits inside the protruding RES sliver (x in [o2-0.3, o2)).
        lr(pdk, "Cont", o2 - 0.25, o + 1.0, o2 - 0.05, o + 1.2),
    ]);
    write_gz(&format!("{DIR}/Rsil.c.gds.gz"), library("TOP", e));
}

/// A full (narrow) Rppd resistor: GatPoly+pSD+SalBlock inside an EXTBlock, away from
/// Activ/nSD.  Body 0.40 wide → Rppd.a / Rppd.e; pSD coincident → Rppd.b; EXTBlock
/// margin 0.10 → Rppd.d.
fn rppd_body(pdk: &PdkConfig) {
    let o = OFFSET;
    let e = vec![
        lr(pdk, "GatPoly", o, o, o + 0.40, o + 3.0),
        lr(pdk, "pSD", o, o, o + 0.40, o + 3.0),
        lr(pdk, "SalBlock", o, o, o + 0.40, o + 3.0),
        lr(pdk, "EXTBlock", o - 0.10, o - 0.10, o + 0.50, o + 3.10),
    ];
    write_gz(&format!("{DIR}/RppdBody.gds.gz"), library("TOP", e));
}

/// One Rppd resistor instance: SalBlock/pSD body `w`×3.0 at `(x, y)`, with GatPoly
/// extending 1.0 µm past each end (to host the end-cap Cont), and a Cont at `gap` from the
/// body's right edge.
fn rppd_c_instance(pdk: &PdkConfig, x: f64, y: f64, w: f64, gap: f64) -> Vec<GdsElement> {
    vec![
        lr(pdk, "SalBlock", x, y, x + w, y + 3.0),
        lr(pdk, "pSD", x, y, x + w, y + 3.0),
        lr(pdk, "GatPoly", x - 1.0, y, x + w + 1.0, y + 3.0),
        lr(pdk, "EXTBlock", x - 1.3, y - 0.3, x + w + 1.3, y + 3.3),
        lr(pdk, "Cont", x + w + gap, y + 1.0, x + w + gap + 0.16, y + 1.16),
    ]
}

/// Rppd.c — three instances: boundary-exact 0.20 gap (clean, both bounds satisfied
/// simultaneously), 0.10 gap (< 0.20 → min_space violation), 0.50 gap (> 0.20 → the
/// Cont no longer touches grown SalBlock → RppdContTooFar violation).
fn rppd_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rppd_c_instance(pdk, o, o, 1.0, 0.20);
    e.extend(rppd_c_instance(pdk, o + 10.0, o, 1.0, 0.10));
    e.extend(rppd_c_instance(pdk, o + 20.0, o, 1.0, 0.50));
    write_gz(&format!("{DIR}/Rppd.c.gds.gz"), library("TOP", e));
}

/// A stack forming an Rhigh body (GatPoly ∩ pSD ∩ nSD ∩ SalBlock).
fn rhigh_body(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    ["GatPoly", "pSD", "nSD", "SalBlock"]
        .iter()
        .map(|l| rect(layer(pdk, l), x, y, x + w, y + h))
        .collect()
}

/// Rhigh: a too-narrow body (Rhi.a width, Rhi.f SalBlock width; coincident pSD_nSD → Rhi.c)
/// plus a clean wide one.
fn rhigh(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rhigh_body(pdk, o, o, 0.40, 3.0);
    e.extend(rhigh_body(pdk, o + 5.0, o, 1.0, 3.0)); // clean width, but coincident pSD_nSD
    write_gz(&format!("{DIR}/Rhigh.gds.gz"), library("TOP", e));
}

/// One Rhigh resistor instance, same end-cap/Cont pattern as `rppd_c_instance` but with
/// the Rhigh recognition body (pSD+nSD+SalBlock, not just pSD+SalBlock).
fn rhi_d_instance(pdk: &PdkConfig, x: f64, y: f64, w: f64, gap: f64) -> Vec<GdsElement> {
    vec![
        lr(pdk, "SalBlock", x, y, x + w, y + 3.0),
        lr(pdk, "pSD", x, y, x + w, y + 3.0),
        lr(pdk, "nSD", x, y, x + w, y + 3.0),
        lr(pdk, "GatPoly", x - 1.0, y, x + w + 1.0, y + 3.0),
        lr(pdk, "EXTBlock", x - 1.3, y - 0.3, x + w + 1.3, y + 3.3),
        lr(pdk, "Cont", x + w + gap, y + 1.0, x + w + gap + 0.16, y + 1.16),
    ]
}

/// Rhi.b — drawn nSD must be identical to pSD inside an Rhigh.  Four cases:
/// - a realistic Rhigh (GatPoly running THROUGH the pSD==nSD+SalBlock stack, out to its
///   contacts) with nSD extending 0.2 µm past pSD → fires HERE but not in KLayout: its
///   recognition (`stack.ext_covering(GatPoly)` = strict containment) is empty whenever
///   the poly extends beyond the stack — i.e. for every realistic resistor — so the
///   shipped check has a hole.  The PDF rule text is unambiguous ("nSD:drawing is only
///   permitted within Rhigh resistors"), so we deliberately keep the stricter behaviour.
/// - the same body with nSD == pSD exactly → clean.
/// - an isolated nSD blob far from any Rhigh → clean per the KLayout formula (only
///   mismatches touching a recognition region are flagged).
/// - GatPoly fully INSIDE the stack + the same overhang → fires in BOTH tools (validated
///   exact-location match), proving the mismatch/abutment machinery itself is faithful.
fn rhi_b(pdk: &PdkConfig) {
    let o = OFFSET;
    let body = |x: f64, nsd_over: f64| -> Vec<GdsElement> {
        vec![
            lr(pdk, "GatPoly", x - 1.0, o, x + 2.0, o + 3.0),
            lr(pdk, "pSD", x, o, x + 1.0, o + 3.0),
            lr(pdk, "nSD", x, o, x + 1.0 + nsd_over, o + 3.0),
            lr(pdk, "SalBlock", x, o, x + 1.0, o + 3.0),
            lr(pdk, "EXTBlock", x - 1.3, o - 0.3, x + 2.3, o + 3.3),
        ]
    };
    let mut e = body(o, 0.2); // mismatch → violation
    e.extend(body(o + 10.0, 0.0)); // identical → clean
    e.push(lr(pdk, "nSD", o + 20.0, o, o + 21.0, o + 3.0)); // isolated blob → clean
    // Case 4: GatPoly fully INSIDE the implant stack (the only shape KLayout's strict
    // `ext_covering` recognition accepts) with the same nSD overhang → fires in both tools.
    let x4 = o + 30.0;
    e.extend(vec![
        lr(pdk, "GatPoly", x4 + 0.25, o + 0.5, x4 + 0.75, o + 2.5),
        lr(pdk, "pSD", x4, o, x4 + 1.0, o + 3.0),
        lr(pdk, "nSD", x4, o, x4 + 1.2, o + 3.0),
        lr(pdk, "SalBlock", x4, o, x4 + 1.0, o + 3.0),
        lr(pdk, "EXTBlock", x4 - 0.3, o - 0.3, x4 + 2.3, o + 3.3),
    ]);
    write_gz(&format!("{DIR}/Rhi.b.gds.gz"), library("TOP", e));
}

/// Rhi.d — same three-instance pattern as Rppd.c (boundary-exact clean, too-close, too-far).
fn rhi_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rhi_d_instance(pdk, o, o, 1.0, 0.20);
    e.extend(rhi_d_instance(pdk, o + 10.0, o, 1.0, 0.10));
    e.extend(rhi_d_instance(pdk, o + 20.0, o, 1.0, 0.50));
    write_gz(&format!("{DIR}/Rhi.d.gds.gz"), library("TOP", e));
}
