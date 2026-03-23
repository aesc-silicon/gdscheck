// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, poly, rect, space_pattern, min_width_pattern, max_width_pattern, notch_pattern, density_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;
use std::f64::consts::SQRT_2;

/// Drawing grid (5 nm); 45° band edges are snapped to it so the fixtures carry no
/// incidental off-grid vertices.
const GRID: f64 = 0.005;
fn snap(v: f64) -> f64 {
    (v / GRID).round() * GRID
}

/// A 45°-bent metal band: a parallelogram with horizontal top/bottom edges and two
/// parallel 45° walls.  `w` is the (approximate) perpendicular width of the diagonal
/// trace and `run` its length along the 45° direction; the lower-left corner sits at
/// `(x, y)`.  The horizontal edge `w·√2` and the slant `run/√2` are snapped to the
/// grid, so the realised width/run differ from the nominal values by < 0.4 nm.
fn band(l: (i16, i16), x: f64, y: f64, w: f64, run: f64) -> gds21::GdsElement {
    let wt = snap(w * SQRT_2); // horizontal edge so the perpendicular wall spacing is w
    let h = snap(run / SQRT_2); // slant so the diagonal run length is `run`
    poly(l, &[(x, y), (x + wt, y), (x + wt + h, y + h), (x + h, y + h)])
}

pub fn generate(pdk: &PdkConfig) {
    for index in 1..6 {
        let dir = format!("tests/data/ihp-sg13g2/metal{}", index);
        std::fs::create_dir_all(&dir).expect("failed to create output directory");

        metal_a(pdk, index, &dir);
        metal_b_space(pdk, index, &dir);
        metal_b_notch(pdk, index, &dir);
        metal_j(pdk, index, &dir);
        metal_k(pdk, index, &dir);

        mfil_c_space(pdk, index, &dir);
        mfil_h(pdk, index, &dir);
        mfil_k(pdk, index, &dir);
        mfil_h_boundary(pdk, index, &dir);
        mfil_h_boundary_ring(pdk, index, &dir);

        // Metal2-5 share the same extended rule set (filler width/spacing plus the
        // enclosure, area, parallel-run and 45° checks).  Metal1 differs and is
        // generated separately.
        if index >= 2 {
            mfil_a1(pdk, index, &dir);
            mfil_a2(pdk, index, &dir);
            mfil_b(pdk, index, &dir);
            mfil_d(pdk, index, &dir);
            m_c(pdk, index, &dir);
            m_c1(pdk, index, &dir);
            m_d(pdk, index, &dir);
            m_e(pdk, index, &dir);
            m_f(pdk, index, &dir);
            m_g(pdk, index, &dir);
            m_i(pdk, index, &dir);
        }
    }
}

/// M{n}.c — Metal{n} encloses Via{n-1} on all sides by 0.005 µm (`enclosure_pattern`:
/// one clean pair plus four with a short margin on each side → 4 violations).  The via
/// is drawn wide (0.5 µm) so the enclosing metal clears the M{n}.d area floor; the
/// 0.005 µm margins fall short of the M{n}.c1 endcap, which is ignored in the test.
fn m_c(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));
    let v = layer(pdk, &format!("Via{}", index - 1));
    let elems = enclosure_pattern(m, v, 0.005, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}.c.gds.gz"), library("TOP", elems));
}

/// M{n}.c1 — Metal{n} endcap enclosure of Via{n-1} (0.05 µm on at least one side).  A
/// via with one 0.05 endcap (other sides 0.02) passes; a via with 0.02 on every side
/// fails.  Both metal regions clear the M{n}.d area floor and the 0.005 µm M{n}.c rule.
fn m_c1(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));
    let v = layer(pdk, &format!("Via{}", index - 1));
    let elems = vec![
        // clean: left side is a 0.05 endcap, the other three sides are 0.02
        rect(v, 0.0, 0.0, 0.5, 0.5),
        rect(m, -0.05, -0.02, 0.52, 0.52),
        // fail: 0.02 on every side — no side reaches the 0.05 endcap
        rect(v, 2.0, 0.0, 2.5, 0.5),
        rect(m, 1.98, -0.02, 2.52, 0.52),
    ];
    write_gz(&format!("{dir}/M{index}.c1.gds.gz"), library("TOP", elems));
}

/// M{n}.d — min. area 0.144 µm².  A 0.4×0.4 = 0.16 µm² region passes; a 0.4×0.35 =
/// 0.14 µm² region falls below the floor.
fn m_d(pdk: &PdkConfig, index: i32, dir: &str) {
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = vec![
        rect(l, OFFSET, OFFSET, OFFSET + 0.40, OFFSET + 0.40),       // 0.160 µm² -> clean
        rect(l, OFFSET, OFFSET + 5.0, OFFSET + 0.40, OFFSET + 5.35), // 0.140 µm² -> violation
    ];
    write_gz(&format!("{dir}/M{index}.d.gds.gz"), library("TOP", elems));
}

/// M{n}.e — min. space 0.24 µm between lines wider than 0.39 µm running parallel for
/// more than 1 µm.  Fail: two 0.5 µm-wide, 2 µm-long lines 0.22 µm apart (the 0.22 µm
/// gap clears the plain M{n}.b 0.21 µm space, which is ignored) — once vertical
/// (parallel run measured along y) and once horizontal (run along x, offset away in x),
/// so both axes of `parallel_run` are exercised → two violations.  Clean file: each
/// pair sits exactly on one threshold so the `>`/`<` boundaries stay green.
fn m_e(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));

    let fail = vec![
        // vertical lines, 0.22 µm horizontal gap
        rect(m, 0.0, 0.0, 0.5, 2.0),
        rect(m, 0.72, 0.0, 1.22, 2.0),
        // horizontal lines, 0.22 µm vertical gap (offset away in x)
        rect(m, 5.0, 0.0, 7.0, 0.5),
        rect(m, 5.0, 0.72, 7.0, 1.22),
    ];
    write_gz(&format!("{dir}/M{index}.e.fail.gds.gz"), library("TOP", fail));

    let clean = vec![
        // line 0.385 µm wide (one grid step under the 0.39 µm "wide" threshold, so
        // not wide) — 0.22 µm gap, 2 µm run
        rect(m, 10.0, 0.0, 10.385, 2.0),
        rect(m, 10.605, 0.0, 10.99, 2.0),
        // parallel run exactly 1.0 µm (not *more than* 1.0) — wide, 0.22 µm gap
        rect(m, 20.0, 0.0, 20.5, 1.0),
        rect(m, 20.72, 0.0, 21.22, 1.0),
        // spacing exactly 0.24 µm (not *less than* 0.24) — wide, 2 µm run
        rect(m, 30.0, 0.0, 30.5, 2.0),
        rect(m, 30.74, 0.0, 31.24, 2.0),
    ];
    write_gz(&format!("{dir}/M{index}.e.gds.gz"), library("TOP", clean));
}

/// M{n}.f — min. space 0.60 µm between lines wider than 10 µm running parallel for more
/// than 10 µm.  Fail: two 12 µm-wide, 12 µm-long lines 0.5 µm apart.  Clean file: each
/// pair sits exactly on one threshold.
fn m_f(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));

    let fail = vec![rect(m, 0.0, 0.0, 12.0, 12.0), rect(m, 12.5, 0.0, 24.5, 12.0)];
    write_gz(&format!("{dir}/M{index}.f.fail.gds.gz"), library("TOP", fail));

    let clean = vec![
        // line exactly 10 µm wide (not *wider than* 10) — 0.5 µm gap, 12 µm run
        rect(m, 100.0, 0.0, 110.0, 12.0),
        rect(m, 110.5, 0.0, 120.5, 12.0),
        // parallel run exactly 10 µm (not *more than* 10) — wide, 0.5 µm gap
        rect(m, 200.0, 0.0, 212.0, 10.0),
        rect(m, 212.5, 0.0, 224.5, 10.0),
        // spacing exactly 0.60 µm (not *less than* 0.60) — wide, 12 µm run
        rect(m, 300.0, 0.0, 312.0, 12.0),
        rect(m, 312.6, 0.0, 324.6, 12.0),
    ];
    write_gz(&format!("{dir}/M{index}.f.gds.gz"), library("TOP", clean));
}

/// M{n}.g — min. 45°-bent width (0.24 µm) where the bent run is > 0.5 µm.  A narrow
/// band (0.20 µm) with a 1 µm run fails on both walls; a wide band (0.30 µm) and a
/// narrow band whose run is only 0.40 µm (below the bent-length threshold) are clean.
fn m_g(pdk: &PdkConfig, index: i32, dir: &str) {
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = vec![
        band(l, OFFSET, OFFSET, 0.20, 1.00),       // narrow + long run  -> 2 violations
        band(l, OFFSET + 5.0, OFFSET, 0.30, 1.00), // wide enough        -> clean
        band(l, OFFSET + 10.0, OFFSET, 0.20, 0.40), // narrow but short  -> clean (run gate)
    ];
    write_gz(&format!("{dir}/M{index}.g.gds.gz"), library("TOP", elems));
}

/// M{n}.i — min. space (0.24 µm) between metal lines of which at least one is 45°-bent.
/// Two parallel 45° bands with a 0.20 µm perpendicular gap violate; their own width
/// (0.50 µm) keeps M{n}.g quiet.  (The 0.20 µm gap also trips the plain M{n}.b space
/// rule, which is ignored in the test.)
fn m_i(pdk: &PdkConfig, index: i32, dir: &str) {
    let l = layer(pdk, &format!("Metal{}", index));
    let w = 0.50;
    let gap = 0.20;
    let x0 = OFFSET;
    let x1 = OFFSET + snap(w * SQRT_2) + snap(gap * SQRT_2); // shift band B perpendicular by the gap
    let elems = vec![
        band(l, x0, OFFSET, w, 1.00),
        band(l, x1, OFFSET, w, 1.00),
    ];
    write_gz(&format!("{dir}/M{index}.i.gds.gz"), library("TOP", elems));
}

fn mfil_a1(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.a1: min_width 1.0 — filler features must be at least 1 µm wide.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = min_width_pattern(fill, 1.0, 1.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}Fil.a1.gds.gz"), library("TOP", elems));
}

fn mfil_a2(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.a2: max_width 5.0 — filler features must be at most 5 µm wide.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = max_width_pattern(fill, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}Fil.a2.gds.gz"), library("TOP", elems));
}

fn mfil_b(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.b: min_space 0.42 between filler features.  Shapes 2 µm wide stay clear
    // of the width limits (1..5), so only the spacing rule fires.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = space_pattern(fill, fill, 2.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}Fil.b.gds.gz"), library("TOP", elems));
}

fn mfil_d(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.d: min_space 1.0 between filler and TRANS.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(fill, trans, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}Fil.d.gds.gz"), library("TOP", elems));
}

fn metal_a(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 0.16 } else { 0.20 };
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = min_width_pattern(l, width, width, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}.a.gds.gz"), library("TOP", elems));
}

fn metal_b_space(pdk: &PdkConfig, index: i32, dir: &str) {
    let space = if index == 1 { 0.18 } else { 0.21 };
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = space_pattern(l, l, 1.0, space, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}.b.space.gds.gz"), library("TOP", elems));
}

fn metal_b_notch(pdk: &PdkConfig, index: i32, dir: &str) {
    let notch = if index == 1 { 0.18 } else { 0.21 };
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = notch_pattern(l, 0.25, notch, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}.b.notch.gds.gz"), library("TOP", elems));
}

fn metal_j(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let mask = layer(pdk, &format!("Metal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal");
    // min_density: bottom Metal stripe drops below the 35 % floor when too short.
    let stripes = |h: f64| [(met, 0.0, h), (fill, 500.0, 600.0), (mask, 900.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{dir}/M{index}.j.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(149.99));
    write_gz(&format!("{dir}/M{index}.j.fail.gds.gz"), library("TOP", elems_fail));
}

fn metal_k(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let mask = layer(pdk, &format!("Metal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal");
    // max_density: top Metal stripe rises above the 60 % ceiling when too tall.
    let stripes = |h: f64| [(met, 0.0, h), (fill, 400.0, 600.0), (mask, 800.0, 1000.0)];

    let mut elems = density_pattern(boundary, 1000.0, &stripes(200.0));
    // duplicated shapes to check merging (absorbed, so density is unchanged)
    elems.extend([
        rect(met, 50.0, 50.0, 950.0, 150.0),
        rect(fill, 50.0, 450.0, 950.0, 550.0),
        rect(mask, 50.0, 850.0, 950.0, 950.0),
    ]);
    write_gz(&format!("{dir}/M{index}.k.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(200.01));
    write_gz(&format!("{dir}/M{index}.k.fail.gds.gz"), library("TOP", elems_fail));
}

fn mfil_c_space(pdk: &PdkConfig, index: i32, dir: &str) {
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let met = layer(pdk, &format!("Metal{}", index));
    let elems = space_pattern(fill, met, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/M{index}Fil.c.gds.gz"), library("TOP", elems));
}

fn mfil_h(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal");
    // min_windowed_density: every 800 µm window must stay above the floor; the
    // shapes are split per window rather than spanning the full width.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend([
        rect(met, 0.0, 0.0, 800.0, 200.0),
        rect(met, 800.0, 0.0, 1000.0, 200.0),
        rect(met, 0.0, 800.0, 800.0, 850.0),
        rect(met, 800.0, 800.0, 1000.0, 850.0),
        // Nested duplicates: absorbed by the merge so coverage is unchanged;
        // without merging they would double-count and inflate the density.
        rect(met, 100.0, 50.0, 700.0, 150.0),
        rect(met, 850.0, 50.0, 950.0, 150.0),
        rect(met, 100.0, 810.0, 700.0, 840.0),
        rect(met, 850.0, 810.0, 950.0, 840.0),
    ]);
    write_gz(&format!("{dir}/M{index}Fil.h.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend([
        rect(met, 0.0, 0.0, 800.0, 199.99),
        rect(met, 800.0, 0.0, 1000.0, 199.99),
        rect(met, 0.0, 800.0, 800.0, 849.99),
        rect(met, 800.0, 800.0, 1000.0, 849.9),
    ]);
    write_gz(&format!("{dir}/M{index}Fil.h.fail.gds.gz"), library("TOP", elems_fail));
}

/// M{n}Fil.h/k boundary handling: the chip's raw bounding box (from *all* shapes)
/// extends past the true EdgeSeal — a small unrelated marker on TRANS sits outside the
/// seal ring, at (950, 950)-(1000, 1000), stretching the overall bbox from the sealed
/// 900x900 die out to 1000x1000.  With an 800 µm window this makes the last row/column
/// of tiles straddle the seal boundary, so their `boundary_layer`-clipped area (only the
/// part actually inside EdgeSeal) must be used as the density denominator — not the
/// nominal (and here doubled) window footprint.
///
/// `ok`: uniform 40% fill (period-100, height-40 stripes) everywhere inside the 900x900
/// EdgeSeal, including the boundary-straddling tiles — every tile's *true* (seal-clipped)
/// density is 40%, comfortably inside [25%, 75%].  Without the boundary fix, the last
/// row/column would be measured against a doubled denominator (only half of which can
/// ever be filled, since fill stops at the seal) and register a false ~20% underfill.
///
/// `fail`: identical, except the (800, 800)-(900, 900) corner — inside EdgeSeal but in
/// the boundary-straddling tile — is starved down to a tiny block, so it genuinely reads
/// under the 25% floor even measured against the correct (seal-clipped) denominator.
/// Proves the boundary fix narrows the denominator without ever *suppressing* a real
/// violation in an edge/corner tile.
fn mfil_h_boundary(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal");
    let trans = layer(pdk, "TRANS");

    let mut elems = vec![rect(boundary, 0.0, 0.0, 900.0, 900.0), rect(trans, 950.0, 950.0, 1000.0, 1000.0)];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(met, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(&format!("{dir}/M{index}Fil.h.boundary_ok.gds.gz"), library("TOP", elems));

    let mut elems_fail = vec![rect(boundary, 0.0, 0.0, 900.0, 900.0), rect(trans, 950.0, 950.0, 1000.0, 1000.0)];
    for k in 0..=7 {
        let y0 = k as f64 * 100.0;
        elems_fail.push(rect(met, 0.0, y0, 900.0, y0 + 40.0));
    }
    // Row 8 (y = 800-840): full width everywhere except the last (boundary-straddling)
    // column, which gets only a small starved block instead of the usual stripe.
    elems_fail.push(rect(met, 0.0, 800.0, 800.0, 840.0));
    elems_fail.push(rect(met, 800.0, 800.0, 820.0, 820.0));
    write_gz(&format!("{dir}/M{index}Fil.h.boundary_fail.gds.gz"), library("TOP", elems_fail));
}

/// M{n}Fil.h/k boundary handling, ring-shaped: a real EdgeSeal is a hollow frame around
/// the die, not a solid square — its own merged *area* is only the thin frame material,
/// far smaller than the 900x900 it encloses.  `boundary_layer` must fall back on the
/// ring's bounding box (its die extent), not its drawn area, or the density denominator
/// collapses to almost nothing and every window reads a wildly inflated (1000%+)
/// density.  Same uniform 40% fill and out-of-seal TRANS marker as `mfil_h_boundary`;
/// expect a clean DRC exactly as with a solid boundary square.
fn mfil_h_boundary_ring(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal");
    let trans = layer(pdk, "TRANS");
    let frame = 20.0;

    let mut elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, frame),
        rect(boundary, 0.0, 900.0 - frame, 900.0, 900.0),
        rect(boundary, 0.0, 0.0, frame, 900.0),
        rect(boundary, 900.0 - frame, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
    ];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(met, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(&format!("{dir}/M{index}Fil.h.boundary_ring.gds.gz"), library("TOP", elems));
}

fn mfil_k(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal");
    // max_windowed_density: every 800 µm window must stay below the ceiling.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend([
        rect(met, 0.0, 0.0, 800.0, 600.0),
        rect(met, 800.0, 0.0, 1000.0, 600.0),
        rect(met, 0.0, 800.0, 800.0, 950.0),
        rect(met, 800.0, 800.0, 1000.0, 950.0),
        // Nested duplicates: absorbed by the merge so coverage is unchanged;
        // without merging they would double-count and inflate the density.
        rect(met, 100.0, 100.0, 700.0, 500.0),
        rect(met, 850.0, 100.0, 950.0, 500.0),
        rect(met, 100.0, 810.0, 700.0, 940.0),
        rect(met, 850.0, 810.0, 950.0, 940.0),
    ]);
    write_gz(&format!("{dir}/M{index}Fil.k.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend([
        rect(met, 0.0, 0.0, 800.0, 600.01),
        rect(met, 800.0, 0.0, 1000.0, 600.01),
        rect(met, 0.0, 800.0, 800.0, 950.01),
        rect(met, 800.0, 800.0, 1000.0, 950.01),
    ]);
    write_gz(&format!("{dir}/M{index}Fil.k.fail.gds.gz"), library("TOP", elems_fail));
}
