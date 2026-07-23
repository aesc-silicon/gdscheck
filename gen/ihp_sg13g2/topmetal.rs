// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, max_width_pattern, notch_pattern, mixed_notch_pattern, density_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    for index in 1..3 {
        let dir = format!("tests/data/ihp-sg13g2/topmetal{}", index);
        std::fs::create_dir_all(&dir).expect("failed to create output directory");

        topmetal_a(pdk, index, &dir);
        topmetal_b_space(pdk, index, &dir);
        topmetal_b_notch(pdk, index, &dir);
        topmetal_b_mixed_notch(pdk, index, &dir);
        topmetal_c(pdk, index, &dir);
        topmetal_d(pdk, index, &dir);

        tmfil_c(pdk, index, &dir);
        tmfil_a(pdk, index, &dir);
        tmfil_a1(pdk, index, &dir);
        tmfil_b(pdk, index, &dir);
        tmfil_d(pdk, index, &dir);

        if index == 2 {
            tm2_br(pdk, &dir);
        }
    }
}

fn topmetal_a(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 1.64 } else { 2.00 };
    let l = layer(pdk, &format!("TopMetal{}", index));
    let elems = min_width_pattern(l, width, width, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}.a.gds.gz"), library("TOP", elems));
}

fn topmetal_b_space(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 1.64 } else { 2.00 };
    let space = if index == 1 { 1.64 } else { 2.00 };
    let l = layer(pdk, &format!("TopMetal{}", index));
    let elems = space_pattern(l, l, width, space, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}.b.space.gds.gz"), library("TOP", elems));
}

fn topmetal_b_notch(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 1.64 } else { 2.00 };
    let notch = if index == 1 { 1.64 } else { 2.00 };
    let l = layer(pdk, &format!("TopMetal{}", index));
    let elems = notch_pattern(l, width, notch, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}.b.notch.gds.gz"), library("TOP", elems));
}

/// TM{n}.b, mixed-orientation notch: a straight wall facing a *diagonal* wall of the
/// same region (a diagonal font-glyph stroke closing in on a straight stem is exactly
/// this shape) — the case a same-orientation-only facing-wall scan cannot see at all.
/// Found via a KLayout reference-deck DRC run on hand-drawn TopMetal2 text that
/// gdscheck's min_notch missed entirely before it paired rectilinear against oblique
/// edges too.
fn topmetal_b_mixed_notch(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 1.64 } else { 2.00 };
    let notch = if index == 1 { 1.64 } else { 2.00 };
    let l = layer(pdk, &format!("TopMetal{}", index));
    let elems = mixed_notch_pattern(l, width, notch, width, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}.b.mixed_notch.gds.gz"), library("TOP", elems));
}

fn topmetal_c(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("TopMetal{}", index));
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let mask = layer(pdk, &format!("TopMetal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density: bottom TopMetal stripe drops below the 25 % floor when too short.
    let stripes = |h: f64| [(met, 0.0, h), (fill, 500.0, 600.0), (mask, 900.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(50.0));
    write_gz(&format!("{dir}/TM{index}.c.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(49.99));
    write_gz(&format!("{dir}/TM{index}.c.fail.gds.gz"), library("TOP", elems_fail));
}

fn topmetal_d(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("TopMetal{}", index));
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let mask = layer(pdk, &format!("TopMetal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // max_density: bottom TopMetal stripe rises above the 70 % ceiling when too tall.
    let stripes = |h: f64| [(met, 0.0, h), (fill, 400.0, 600.0), (mask, 800.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(300.0));
    write_gz(&format!("{dir}/TM{index}.d.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(300.01));
    write_gz(&format!("{dir}/TM{index}.d.fail.gds.gz"), library("TOP", elems_fail));
}

fn tmfil_c(pdk: &PdkConfig, index: i32, dir: &str) {
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let met = layer(pdk, &format!("TopMetal{}", index));
    let elems = space_pattern(fill, met, 5.0, 3.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}Fil.c.gds.gz"), library("TOP", elems));
}

fn tmfil_a(pdk: &PdkConfig, index: i32, dir: &str) {
    // TM{n}Fil.a: min_width 5.0 — filler features must be at least 5 µm wide.
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let elems = min_width_pattern(fill, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}Fil.a.gds.gz"), library("TOP", elems));
}

fn tmfil_a1(pdk: &PdkConfig, index: i32, dir: &str) {
    // TM{n}Fil.a1: max_width 10.0 — filler features must be at most 10 µm wide.
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let elems = max_width_pattern(fill, 10.0, 10.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}Fil.a1.gds.gz"), library("TOP", elems));
}

fn tmfil_b(pdk: &PdkConfig, index: i32, dir: &str) {
    // TM{n}Fil.b: min_space 3.0 between filler features.  Shapes 6 µm wide stay clear
    // of the filler width limits (5..10), so only the spacing rule fires.
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let elems = space_pattern(fill, fill, 6.0, 3.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}Fil.b.gds.gz"), library("TOP", elems));
}

fn tmfil_d(pdk: &PdkConfig, index: i32, dir: &str) {
    // TM{n}Fil.d: min_space 4.90 between filler and TRANS.
    let fill = layer(pdk, &format!("TopMetal{}.filler", index));
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(fill, trans, 6.0, 4.90, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TM{index}Fil.d.gds.gz"), library("TOP", elems));
}

/// TM2.bR: wide-line spacing — 5 µm min space when a line is wider than 5 µm and the
/// parallel run exceeds 50 µm; not checked inside IND regions.
fn tm2_br(pdk: &PdkConfig, dir: &str) {
    let m = layer(pdk, "TopMetal2");
    let ind = layer(pdk, "IND");

    // Fail: two 6 µm-wide lines, 60 µm long, 4 µm gap (< 5) — wide and parallel
    // run 60 > 50, so the 5 µm spacing applies and is violated.
    let fail = vec![rect(m, 0.0, 0.0, 6.0, 60.0), rect(m, 10.0, 0.0, 16.0, 60.0)];
    write_gz(&format!("{dir}/TM2.bR.fail.gds.gz"), library("TOP", fail));

    // Clean: each pair sits exactly on one threshold (so `>`/`<` boundaries stay
    // green) while the other two conditions are met.
    let clean = vec![
        // line exactly 5 µm wide (not *wider than* 5) — 4 µm gap, 60 µm run
        rect(m, 100.0, 0.0, 105.0, 60.0),
        rect(m, 109.0, 0.0, 114.0, 60.0),
        // parallel run exactly 50 µm (not *more than* 50) — wide, 4 µm gap
        rect(m, 200.0, 0.0, 206.0, 50.0),
        rect(m, 210.0, 0.0, 216.0, 50.0),
       // parallel run exactly 50 µm (not *more than* 50) — wide, 4 µm gap
        rect(m, 300.0, 0.0, 306.0, 50.1),
        rect(m, 310.0, 0.0, 316.0, 50.0),
        // spacing exactly 5 µm (not *less than* 5) — wide, 60 µm run
        rect(m, 400.0, 0.0, 406.0, 60.0),
        rect(m, 411.0, 0.0, 417.0, 60.0),
    ];
    write_gz(&format!("{dir}/TM2.bR.gds.gz"), library("TOP", clean));

    // IND-exempt: the failing geometry, fully covered by an IND region.
    let exempt = vec![
        rect(m, 0.0, 0.0, 6.0, 60.0),
        rect(m, 10.0, 0.0, 16.0, 60.0),
        rect(ind, -5.0, -5.0, 21.0, 65.0),
    ];
    write_gz(&format!("{dir}/TM2.bR.ind.gds.gz"), library("TOP", exempt));
}
