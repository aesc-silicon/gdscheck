// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, poly, rect, space_pattern, min_width_pattern, max_width_pattern, notch_pattern, density_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use gds21::GdsElement;
use std::f64::consts::SQRT_2;

const DIR: &str = "tests/data/ihp-sg13g2/gatpoly";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    gat_a(pdk);
    gat_a1(pdk);
    gat_a2(pdk);
    gat_a3(pdk);
    gat_a4(pdk);
    gat_b_space(pdk);
    gat_b_notch(pdk);
    gat_b1(pdk);
    gat_c(pdk);
    gat_d(pdk);
    gat_e(pdk);
    gat_f(pdk);
    gat_g(pdk);
    gfil_a(pdk);
    gfil_b(pdk);
    gfil_c(pdk);
    gfil_d(pdk);
    gfil_e_nwell(pdk);
    gfil_e_nbulay(pdk);
    gfil_f(pdk);
    gfil_g(pdk);
    gfil_g_boundary(pdk);
    gfil_i(pdk);
    gfil_j(pdk);
}

/// GFil.a — max. GatPoly:filler width 5.00 µm.
fn gfil_a(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let elems = max_width_pattern(l, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.a.gds.gz"), library("TOP", elems));
}

/// GFil.b — min. GatPoly:filler width 0.70 µm.
fn gfil_b(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let elems = min_width_pattern(l, 0.70, 0.70, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.b.gds.gz"), library("TOP", elems));
}

/// GFil.c — min. GatPoly:filler space 0.80 µm.  1 µm shapes clear the width limits.
fn gfil_c(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let elems = space_pattern(l, l, 1.0, 0.80, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.c.gds.gz"), library("TOP", elems));
}

/// GFil.e — min. GatPoly:filler space to NWell 1.10 µm.
fn gfil_e_nwell(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.e.nwell.gds.gz"), library("TOP", elems));
}

/// GFil.e — min. GatPoly:filler space to nBuLay 1.10 µm.
fn gfil_e_nbulay(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let nbl = layer(pdk, "nBuLay");
    let elems = space_pattern(l, nbl, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.e.nbulay.gds.gz"), library("TOP", elems));
}

/// GFil.f — min. GatPoly:filler space to TRANS 1.10 µm.
fn gfil_f(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(l, trans, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.f.gds.gz"), library("TOP", elems));
}

/// GFil.i — max. GatPoly:nofill area 160000 µm² (400×400).  A 410×410 region violates;
/// a 399×399 region is clean.
fn gfil_i(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.nofill");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 410.0, o + 410.0),                 // 168100 µm² → violation
        rect(l, o + 500.0, o, o + 500.0 + 399.0, o + 399.0), // 159201 µm² → clean
    ];
    write_gz(&format!("{DIR}/GFil.i.gds.gz"), library("TOP", elems));
}

/// GFil.j — GatPoly:filler endcap: must extend ≥ 0.18 µm over Activ:filler.  A 0.8 µm
/// gate-filler over an Activ:filler extending 0.18 (clean) and 0.17 (violation).  The
/// gate is 0.8 µm wide so it clears the 0.70 µm GFil.b min width.
fn gfil_j(pdk: &PdkConfig) {
    let gf = layer(pdk, "GatPoly.filler");
    let af = layer(pdk, "Activ.filler");
    let o = OFFSET;
    let elems = vec![
        rect(af, o, o, o + 1.0, o + 1.0),
        rect(gf, o + 0.1, o - 0.18, o + 0.9, o + 1.18), // extends 0.18 → clean
        rect(af, o + 3.0, o, o + 4.0, o + 1.0),
        rect(gf, o + 3.1, o - 0.18, o + 3.9, o + 1.17), // extends 0.17 on top → violation
    ];
    write_gz(&format!("{DIR}/GFil.j.gds.gz"), library("TOP", elems));
}

fn gat_a(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = min_width_pattern(l, 0.13, 0.13, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.a.gds.gz"), library("TOP", elems));
}

fn gat_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = space_pattern(l, l, 1.0, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.b.space.gds.gz"), library("TOP", elems));
}

fn gat_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = notch_pattern(l, 0.15, 0.18, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.b.notch.gds.gz"), library("TOP", elems));
}

/// Gat.b1 — two 3.3 V gate fingers (GatPoly over Activ under ThickGateOx) spaced 0.2 µm
/// (< 0.25, but ≥ 0.18 so the general Gat.b stays clean) → Gat.b1.  No implant, so the NFET/
/// PFET gate-length rules don't apply.
fn gat_b1(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        rect(layer(pdk, "Activ"), o, o, o + 3.0, o + 1.0),
        rect(layer(pdk, "ThickGateOx"), o - 0.2, o - 0.2, o + 3.2, o + 1.2),
        rect(layer(pdk, "GatPoly"), o + 1.0, o - 0.3, o + 1.3, o + 1.3),
        rect(layer(pdk, "GatPoly"), o + 1.5, o - 0.3, o + 1.8, o + 1.3),
    ];
    write_gz(&format!("{DIR}/Gat.b1.gds.gz"), library("TOP", elems));
}

fn gat_d(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let act = layer(pdk, "Activ");
    let elems = space_pattern(l, act, 1.0, 0.07, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.d.gds.gz"), library("TOP", elems));
}

/// A transistor at `(x, OFFSET)`: a 2×1 µm Activ + implant `imp` (optionally covered by
/// ThickGateOx), crossed by a vertical GatPoly gate of length `gl` extending 0.3 µm past
/// the Activ top/bottom.
fn gate_device(pdk: &PdkConfig, imp: &str, x: f64, gl: f64, tgo: bool) -> Vec<GdsElement> {
    let o = OFFSET;
    let gx = x + 0.6;
    let mut e = vec![
        rect(layer(pdk, "Activ"), x, o, x + 2.0, o + 1.0),
        rect(layer(pdk, imp), x, o, x + 2.0, o + 1.0),
        rect(layer(pdk, "GatPoly"), gx, o - 0.3, gx + gl, o + 1.3),
    ];
    if tgo {
        e.push(rect(layer(pdk, "ThickGateOx"), x - 0.1, o - 0.4, x + 2.1, o + 1.4));
    }
    e
}

/// Gat.a1 — gate length of a 1.2 V NFET (nSD channel, no ThickGateOx) ≥ 0.13 µm.
fn gat_a1(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "nSD", OFFSET, 0.13, false);
    e.extend(gate_device(pdk, "nSD", OFFSET + 5.0, 0.12, false));
    write_gz(&format!("{DIR}/Gat.a1.gds.gz"), library("TOP", e));
}

/// Gat.a2 — gate length of a 1.2 V PFET (pSD channel, no ThickGateOx) ≥ 0.13 µm.
fn gat_a2(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "pSD", OFFSET, 0.13, false);
    e.extend(gate_device(pdk, "pSD", OFFSET + 5.0, 0.12, false));
    write_gz(&format!("{DIR}/Gat.a2.gds.gz"), library("TOP", e));
}

/// Gat.a3 — gate length of a 3.3 V NFET (nSD channel inside ThickGateOx) ≥ 0.45 µm.
fn gat_a3(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "nSD", OFFSET, 0.45, true);
    e.extend(gate_device(pdk, "nSD", OFFSET + 5.0, 0.44, true));
    write_gz(&format!("{DIR}/Gat.a3.gds.gz"), library("TOP", e));
}

/// Gat.a4 — gate length of a 3.3 V PFET (pSD channel inside ThickGateOx) ≥ 0.40 µm.
fn gat_a4(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "pSD", OFFSET, 0.40, true);
    e.extend(gate_device(pdk, "pSD", OFFSET + 5.0, 0.39, true));
    write_gz(&format!("{DIR}/Gat.a4.gds.gz"), library("TOP", e));
}

/// Gat.c — GatPoly endcap: must extend ≥ 0.18 µm over Activ.  A gate extending 0.18
/// (clean) and one extending only 0.17 µm on top (violation).
fn gat_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(gp, o + 0.3, o - 0.18, o + 0.7, o + 1.18),
        rect(activ, o + 3.0, o, o + 4.0, o + 1.0),
        rect(gp, o + 3.3, o - 0.18, o + 3.7, o + 1.17),
    ];
    write_gz(&format!("{DIR}/Gat.c.gds.gz"), library("TOP", elems));
}

/// Gat.e — min. GatPoly area 0.09 µm².  A 0.2×0.2 = 0.04 µm² region violates; a 0.4×0.4
/// = 0.16 µm² region is clean.  Both stay above the 0.13 µm min width.
fn gat_e(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 0.2, o + 0.2),         // 0.04 µm² → violation
        rect(l, o + 2.0, o, o + 2.4, o + 0.4),   // 0.16 µm² → clean
    ];
    write_gz(&format!("{DIR}/Gat.e.gds.gz"), library("TOP", elems));
}

/// Gat.f — no 45° GatPoly over Activ.  A straight orthogonal gate crossing an Activ (only
/// 90° edges over the channel → clean) and a 45°-running gate crossing an Activ, whose two
/// diagonal sides cross the channel (2 violations).
fn gat_f(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let mut elems = vec![
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(gp, o + 0.4, o - 0.3, o + 0.6, o + 1.3), // straight → orthogonal → clean
        rect(activ, o + 3.0, o, o + 5.0, o + 1.0),
    ];
    // 45° gate fully crossing the Activ: its horizontal ends sit outside the channel, so
    // only the two diagonal sides are over the Activ → exactly two forbidden-angle edges.
    elems.push(band(gp, o + 3.6, o - 0.3, 0.2, 2.3));
    write_gz(&format!("{DIR}/Gat.f.gds.gz"), library("TOP", elems));
}

/// A 45°-bent GatPoly band: perpendicular width `w`, run `run`, snapped to the grid.
fn band(l: (i16, i16), x: f64, y: f64, w: f64, run: f64) -> GdsElement {
    const GRID: f64 = 0.005;
    let snap = |v: f64| (v / GRID).round() * GRID;
    let wt = snap(w * SQRT_2);
    let h = snap(run / SQRT_2);
    poly(l, &[(x, y), (x + wt, y), (x + wt + h, y + h), (x + h, y + h)])
}

/// Gat.g — min. 45°-bent GatPoly width 0.16 µm where the bent run > 0.39 µm.  A 0.15 µm
/// band with a 0.6 µm run fails on both walls; a 0.20 µm band and a 0.30 µm-run band are
/// clean.
fn gat_g(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        band(l, o, o, 0.15, 0.6),        // narrow + long run → 2 violations
        band(l, o + 5.0, o, 0.20, 0.6),  // wide enough → clean
        band(l, o + 10.0, o, 0.15, 0.30), // short run → clean (bent-length gate)
    ];
    write_gz(&format!("{DIR}/Gat.g.gds.gz"), library("TOP", elems));
}

fn gfil_d(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let mut elems = vec![];
    let layers = vec!["Activ", "GatPoly", "Cont", "pSD", "nSD.block", "SalBlock"];
    for (i, name) in layers.into_iter().enumerate() {
        elems.append(&mut space_pattern(l, layer(pdk, name), 1.0, 1.10, i as f64 * OFFSET, SPACE_DELTA));
    }
    write_gz(&format!("{DIR}/GFil.d.gds.gz"), library("TOP", elems));
}

fn gfil_g(pdk: &PdkConfig) {
    let gat = layer(pdk, "GatPoly");
    let gfil = layer(pdk, "GatPoly.filler");
    let boundary = layer(pdk, "EdgeSeal");
    // min_density: bottom GatPoly stripe drops below the 15 % floor when too short.
    let stripes = |h: f64| [(gat, 0.0, h), (gfil, 925.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(75.0));
    write_gz(&format!("{DIR}/GFil.g.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(74.99));
    write_gz(&format!("{DIR}/GFil.g.fail.gds.gz"), library("TOP", elems_fail));
}

/// GFil.g boundary handling: an unrelated marker (TRANS) sits outside EdgeSeal and
/// stretches the chip's raw bounding box, so `boundary_layer`'s *own* bbox — not the raw
/// chip bbox — must be the density denominator (`ok`, solid EdgeSeal square).  `ring`
/// draws EdgeSeal as a hollow ring instead, as a real seal ring actually is: its own
/// merged area is only the thin frame, so this proves the bbox convention (not the
/// ring's drawn area) is what `min_density`/`max_density` already use — the same class
/// of bug the windowed-density boundary fix corrected.  Both fixtures cover the same
/// 900x900 seal at 20% GatPoly density, comfortably above the 15% floor.
fn gfil_g_boundary(pdk: &PdkConfig) {
    let gat = layer(pdk, "GatPoly");
    let boundary = layer(pdk, "EdgeSeal");
    let trans = layer(pdk, "TRANS");

    let elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
        rect(gat, 0.0, 0.0, 900.0, 180.0),
    ];
    write_gz(&format!("{DIR}/GFil.g.boundary_ok.gds.gz"), library("TOP", elems));

    let frame = 20.0;
    let elems_ring = vec![
        rect(boundary, 0.0, 0.0, 900.0, frame),
        rect(boundary, 0.0, 900.0 - frame, 900.0, 900.0),
        rect(boundary, 0.0, 0.0, frame, 900.0),
        rect(boundary, 900.0 - frame, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
        rect(gat, 0.0, 0.0, 900.0, 180.0),
    ];
    write_gz(&format!("{DIR}/GFil.g.boundary_ring.gds.gz"), library("TOP", elems_ring));
}
