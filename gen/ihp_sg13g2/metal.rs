// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Drawing grid (5 nm); 45° band edges are snapped to it so the fixtures carry no
/// incidental off-grid vertices.
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, cont_at, density_pattern, diamond, enclosure_pattern, layer, library,
    max_width_pattern, min_width_pattern, mixed_notch_pattern, notch_pattern, poly, rect,
    space_pattern, strap, strip45, stripes, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;
use std::f64::consts::SQRT_2;

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
    poly(
        l,
        &[(x, y), (x + wt, y), (x + wt + h, y + h), (x + h, y + h)],
    )
}

pub fn generate(pdk: &PdkConfig) {
    for index in 1..6 {
        let dir = format!("tests/data/ihp-sg13g2/metal{}", index);
        std::fs::create_dir_all(&dir).expect("failed to create output directory");

        metal_a(pdk, index, &dir);
        metal_b_space(pdk, index, &dir);
        metal_b_notch(pdk, index, &dir);
        metal_corner(pdk, index, &dir);
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

    hardening_metal1(pdk);

    hardening_metaln(pdk);
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

/// M{n}.c1 — Metal{n} endcap enclosure of Via{n-1} (0.05 µm on the ends of the line
/// running past: one side, or two opposite sides).  A via with 0.05 left and right
/// (top and bottom 0.02) passes; a via with 0.02 on every side fails.  Both metal
/// regions clear the M{n}.d area floor and the 0.005 µm M{n}.c rule.
fn m_c1(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));
    let v = layer(pdk, &format!("Via{}", index - 1));
    let elems = vec![
        // clean: a line running past left to right, 0.05 endcaps, 0.02 above and below
        rect(v, 0.0, 0.0, 0.5, 0.5),
        rect(m, -0.05, -0.02, 0.55, 0.52),
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
        rect(l, OFFSET, OFFSET, OFFSET + 0.40, OFFSET + 0.40), // 0.160 µm² -> clean
        rect(l, OFFSET, OFFSET + 5.0, OFFSET + 0.40, OFFSET + 5.35), // 0.140 µm² -> violation
    ];
    write_gz(&format!("{dir}/M{index}.d.gds.gz"), library("TOP", elems));
}

/// M{n}.e — min. space 0.24 µm between lines wider than 0.39 µm running parallel for
/// more than 1 µm.  Fail: two 0.5 µm-wide, 2 µm-long lines 0.22 µm apart (the 0.22 µm
/// gap clears the plain M{n}.b 0.21 µm space, which is ignored) — once vertical
/// (parallel run measured along y) and once horizontal (run along x, offset away in x),
/// so both axes of `length` are exercised → two violations.  Clean file: each
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
    write_gz(
        &format!("{dir}/M{index}.e.fail.gds.gz"),
        library("TOP", fail),
    );

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

    let fail = vec![
        rect(m, 0.0, 0.0, 12.0, 12.0),
        rect(m, 12.5, 0.0, 24.5, 12.0),
    ];
    write_gz(
        &format!("{dir}/M{index}.f.fail.gds.gz"),
        library("TOP", fail),
    );

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
        band(l, OFFSET, OFFSET, 0.20, 1.00), // narrow + long run  -> 2 violations
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
    let elems = vec![band(l, x0, OFFSET, w, 1.00), band(l, x1, OFFSET, w, 1.00)];
    write_gz(&format!("{dir}/M{index}.i.gds.gz"), library("TOP", elems));
}

fn mfil_a1(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.a1: min_width 1.0 — filler features must be at least 1 µm wide.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = min_width_pattern(fill, 1.0, 1.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}Fil.a1.gds.gz"),
        library("TOP", elems),
    );
}

fn mfil_a2(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.a2: max_width 5.0 — filler features must be at most 5 µm wide.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = max_width_pattern(fill, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}Fil.a2.gds.gz"),
        library("TOP", elems),
    );
}

fn mfil_b(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.b: min_space 0.42 between filler features.  Shapes 2 µm wide stay clear
    // of the width limits (1..5), so only the spacing rule fires.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let elems = space_pattern(fill, fill, 2.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}Fil.b.gds.gz"),
        library("TOP", elems),
    );
}

fn mfil_d(pdk: &PdkConfig, index: i32, dir: &str) {
    // M{n}Fil.d: min_space 1.0 between filler and TRANS.
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(fill, trans, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}Fil.d.gds.gz"),
        library("TOP", elems),
    );
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
    write_gz(
        &format!("{dir}/M{index}.b.space.gds.gz"),
        library("TOP", elems),
    );
}

fn metal_b_notch(pdk: &PdkConfig, index: i32, dir: &str) {
    let notch = if index == 1 { 0.18 } else { 0.21 };
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = notch_pattern(l, 0.25, notch, 1.0, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}.b.notch.gds.gz"),
        library("TOP", elems),
    );
}

/// Two 10 µm squares of Metal{n} touching at one corner.  The layer's width there is
/// zero - a pinch, M{n}.a - and so is the space between the two, M{n}.b; KLayout reports
/// both on this drawing, one marker each.  The two squares merge into one region, and
/// the width of zero is at a vertex no pair of facing walls measures, which is the miss
/// this drawing came in as.
fn metal_corner(pdk: &PdkConfig, index: i32, dir: &str) {
    let l = layer(pdk, &format!("Metal{}", index));
    let elems = vec![
        rect(l, OFFSET - 5.0, OFFSET - 5.0, OFFSET + 5.0, OFFSET + 5.0),
        rect(l, OFFSET + 5.0, OFFSET + 5.0, OFFSET + 15.0, OFFSET + 15.0),
    ];
    write_gz(
        &format!("{dir}/M{index}.corner.gds.gz"),
        library("TOP", elems),
    );
}

fn metal_j(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let mask = layer(pdk, &format!("Metal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density: bottom Metal stripe drops below the 35 % floor when too short.
    let stripes = |h: f64| [(met, 0.0, h), (fill, 500.0, 600.0), (mask, 900.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{dir}/M{index}.j.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(149.99));
    write_gz(
        &format!("{dir}/M{index}.j.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

fn metal_k(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let mask = layer(pdk, &format!("Metal{}.mask", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
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
    write_gz(
        &format!("{dir}/M{index}.k.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

fn mfil_c_space(pdk: &PdkConfig, index: i32, dir: &str) {
    let fill = layer(pdk, &format!("Metal{}.filler", index));
    let met = layer(pdk, &format!("Metal{}", index));
    let elems = space_pattern(fill, met, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{dir}/M{index}Fil.c.gds.gz"),
        library("TOP", elems),
    );
}

fn mfil_h(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density in any 800 µm window: the windows slide a tile at a time and one is
    // laid against each far edge.  Uniform stripes read the same in every window: 26 %
    // is clean, 24 % fails everywhere - one violation, the windows overlap.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend(stripes(met, 1000.0, 26.0));
    // Nested duplicates: absorbed by the merge so coverage is unchanged; without
    // merging they would double-count and inflate the density.
    elems.push(rect(met, 100.0, 5.0, 700.0, 20.0));
    write_gz(
        &format!("{dir}/M{index}Fil.h.gds.gz"),
        library("TOP", elems),
    );

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend(stripes(met, 1000.0, 24.0));
    write_gz(
        &format!("{dir}/M{index}Fil.h.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

/// M{n}Fil.h/k boundary handling: the chip's raw bounding box (from *all* shapes)
/// extends past the true EdgeSeal — a small unrelated marker on TRANS sits outside the
/// seal ring, at (950, 950)-(1000, 1000), stretching the overall bbox from the sealed
/// 900x900 die out to 1000x1000.  The windows are laid over the `boundary` layer's
/// box, so the 900x900 die gets four 800 µm windows (at 0 and at 100, each way) and
/// none reaches past the seal into the empty strip the marker adds.
///
/// `ok`: uniform 40% fill (period-100, height-40 stripes) everywhere inside the 900x900
/// EdgeSeal - every window reads 40%, comfortably inside [25%, 75%].  Laid over the raw
/// bounding box instead, a window against the far edge would take in 100 µm of nothing
/// and read 35% - still clean here, but not the die's density.
///
/// `fail`: the same 40% stripes with the corner (400, 400)-(900, 900) left empty.  The
/// windows slide a tile at a time; the one against the far corner, (100, 100)-(900, 900),
/// holds the whole hole and reads 24.4%, the windows next to it less of it - the
/// violating windows overlap and are one violation, reported at the worst.  The window
/// at the origin reads 30%, one against a single far edge 27.5%.  The die is at 27.7%,
/// which the global M{n}.j (35%) reports; the case ignores it.
fn mfil_h_boundary(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    let trans = layer(pdk, "TRANS");

    let mut elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
    ];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(met, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(
        &format!("{dir}/M{index}Fil.h.boundary_ok.gds.gz"),
        library("TOP", elems),
    );

    let mut elems_fail = vec![
        rect(boundary, 0.0, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
    ];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        let x1 = if y0 >= 400.0 { 400.0 } else { 900.0 };
        elems_fail.push(rect(met, 0.0, y0, x1, y0 + 40.0));
    }
    write_gz(
        &format!("{dir}/M{index}Fil.h.boundary_fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

/// M{n}Fil.h/k boundary handling, ring-shaped: a real EdgeSeal is a hollow frame around
/// the die, not a solid square — its own merged *area* is only the thin frame material,
/// far smaller than the 900x900 it encloses.  `boundary` must fall back on the
/// ring's bounding box (its die extent), not its drawn area, or the density denominator
/// collapses to almost nothing and every window reads a wildly inflated (1000%+)
/// density.  Same uniform 40% fill and out-of-seal TRANS marker as `mfil_h_boundary`;
/// expect a clean DRC exactly as with a solid boundary square.
fn mfil_h_boundary_ring(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
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
    write_gz(
        &format!("{dir}/M{index}Fil.h.boundary_ring.gds.gz"),
        library("TOP", elems),
    );
}

fn mfil_k(pdk: &PdkConfig, index: i32, dir: &str) {
    let met = layer(pdk, &format!("Metal{}", index));
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // max_density in any 800 µm window, as `mfil_h`: 74 % is clean, 76 % fails
    // everywhere, one violation.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend(stripes(met, 1000.0, 74.0));
    // Nested duplicates: absorbed by the merge so coverage is unchanged; without
    // merging they would double-count and inflate the density.
    elems.push(rect(met, 100.0, 5.0, 700.0, 60.0));
    write_gz(
        &format!("{dir}/M{index}Fil.k.gds.gz"),
        library("TOP", elems),
    );

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend(stripes(met, 1000.0, 76.0));
    write_gz(
        &format!("{dir}/M{index}Fil.k.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------

const DIR: &str = "tests/data/ihp-sg13g2/metal1";

/// Layers the patterns draw on.
struct L {
    m1: (i16, i16),
    cont: (i16, i16),
    activ: (i16, i16),
    via1: (i16, i16),
    m2: (i16, i16),
    fill: (i16, i16),
    mask: (i16, i16),
    slit: (i16, i16),
    m5fill: (i16, i16),
    trans: (i16, i16),
    seal: (i16, i16),
    boundary: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            m1: layer(pdk, "Metal1"),
            cont: layer(pdk, "Cont"),
            activ: layer(pdk, "Activ"),
            via1: layer(pdk, "Via1"),
            m2: layer(pdk, "Metal2"),
            fill: layer(pdk, "Metal1.filler"),
            mask: layer(pdk, "Metal1.mask"),
            slit: layer(pdk, "Metal1.slit"),
            m5fill: layer(pdk, "Metal5.filler"),
            trans: layer(pdk, "TRANS"),
            seal: layer(pdk, "EdgeSeal"),
            boundary: layer(pdk, "EdgeSeal.boundary"),
        }
    }

    /// Square ring of `layer`: outer box `(x0, y0)-(x1, y1)` minus the hole
    /// `(hx0, hy0)-(hx1, hy1)`, four overlapping wall boxes that merge into one ring.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        layer: (i16, i16),
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
            rect(layer, x0, y0, hx0, y1),
            rect(layer, hx1, y0, x1, y1),
            rect(layer, x0, y0, x1, hy0),
            rect(layer, x0, hy1, x1, y1),
        ]
    }

    /// The same ring as one keyhole polygon (the outline runs in along a zero-width cut,
    /// round the hole and back out).
    #[allow(clippy::too_many_arguments)]
    fn keyhole(
        &self,
        layer: (i16, i16),
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        hx0: f64,
        hy0: f64,
        hx1: f64,
        hy1: f64,
    ) -> GdsElement {
        poly(
            layer,
            &[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, hy0),
                (hx0, hy0),
                (hx0, hy1),
                (hx1, hy1),
                (hx1, hy0),
                (x0, hy0),
            ],
        )
    }

    /// A Cont at `(cx, cy)` on a 0.4 Activ tap, with the Metal1 the caller draws.
    fn tapped_cont(&self, cx: f64, cy: f64) -> Vec<GdsElement> {
        vec![
            rect(self.activ, cx - 0.2, cy - 0.2, cx + 0.2, cy + 0.2),
            cont_at(self.cont, cx, cy),
        ]
    }

    /// A Via1 at `(cx, cy)` (0.19 square) with a 0.3 × 0.3 Metal2 pad: the Metal2 strap
    /// between two of these joins two Metal1 shapes into one net.
    fn via_up(&self, cx: f64, cy: f64) -> Vec<GdsElement> {
        vec![rect(
            self.via1,
            cx - 0.095,
            cy - 0.095,
            cx + 0.095,
            cy + 0.095,
        )]
    }

    fn m2_strap(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
        rect(self.m2, x0, y0, x1, y1)
    }
}

/// A 0.16 Metal1 line running horizontally from `(x, y)` for 2 µm, jogging up-right (or
/// down-right when `down`) at 45° by `h`, then on for 2 µm.  The jog's walls are `wt/√2`
/// apart and each 45° wall is `h·√2` long.
fn zroute(l: (i16, i16), x: f64, y: f64, wt: f64, h: f64, down: bool) -> GdsElement {
    let w = 0.16;
    let pts = [
        (x, y),
        (x + 2.0, y),
        (x + 2.0 + h, y + h),
        (x + 4.0 + h, y + h),
        (x + 4.0 + h, y + h + w),
        (x + 2.0 - wt + w + h, y + h + w),
        (x + 2.0 - wt + w, y + w),
        (x, y + w),
    ];
    if down {
        let m: Vec<(f64, f64)> = pts.iter().map(|&(px, py)| (px, 2.0 * y - py)).collect();
        poly(l, &m)
    } else {
        poly(l, &pts)
    }
}

/// An L of 0.16 lines, arms `len` long from the outer corner `(x, y)`, that corner cut
/// along `X + Y = x + y + k` and the inner corner along `X + Y = x + y + kin`.  The two 45°
/// walls are `(kin − k)/√2` apart; the outer one is `k·√2` long, the inner `(kin − 0.32)·√2`.
fn chamfered_l(l: (i16, i16), x: f64, y: f64, len: f64, k: f64, kin: f64) -> GdsElement {
    let w = 0.16;
    poly(
        l,
        &[
            (x + k, y),
            (x + len, y),
            (x + len, y + w),
            (x + kin - w, y + w),
            (x + w, y + kin - w),
            (x + w, y + len),
            (x, y + len),
            (x, y + k),
        ],
    )
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening_metal1(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let l = L::new(pdk);
    m1_a_h(&l);
    m1_b_h(&l);
    m1_c_h(&l);
    m1_c1_h(&l);
    m1_d_h(&l);
    m1_e_h(&l);
    m1_f_h(&l);
    m1_g_h(&l);
    m1_i_h(&l);
    m1_seal_h(&l);
    m1_j_h(&l);
    m1fil_a1_h(&l);
    m1fil_a2_h(&l);
    m1fil_b_h(&l);
    m1fil_c_h(&l);
    m1fil_d_h(&l);
}

// --- M1.a: min. Metal1 width 0.16 ---

fn m1_a_h(l: &L) {
    let m = l.m1;

    // h1 — the bound, long and far.  0.16 is legal, 0.155 fires in x and in y; a 300 µm
    // bar 0.155 tall counts once (two walls); a 0.005 sliver; a bar at (1000, 1000).
    write(
        "M1.a.h1",
        vec![
            rect(m, 2.0, 2.0, 2.16, 4.0),              // clean
            rect(m, 4.0, 2.0, 4.155, 4.0),             // M1.a
            rect(m, 6.0, 2.0, 8.0, 2.155),             // M1.a
            rect(m, 2.0, 6.0, 302.0, 6.16),            // clean, 300 µm
            rect(m, 2.0, 8.0, 302.0, 8.155),           // M1.a, 300 µm
            rect(m, 2.0, 10.0, 2.005, 12.0),           // M1.a, sliver
            rect(m, 1000.0, 1000.0, 1000.155, 1002.0), // M1.a
        ],
    );

    // h2 — 45° geometry.  A diamond's width is a·√2: a = 0.115 → 0.1626 clean, a = 0.11 →
    // 0.1556 fires (four walls); a 45° strip d·√2 the same (two walls; the 3 µm strips are
    // M1.g as well, 0.1626 < 0.20); a chamfered box and an L with a chamfered inner corner
    // are wide everywhere and stay clean.
    write(
        "M1.a.h2",
        vec![
            diamond(m, 3.0, 3.0, 0.115),               // clean
            diamond(m, 6.0, 3.0, 0.11),                // M1.a
            strip45(m, 8.0, 2.0, 3.0, 0.115),          // clean (M1.g)
            strip45(m, 13.0, 2.0, 3.0, 0.11),          // M1.a (M1.g)
            chamfered_tr(m, 2.0, 6.0, 4.0, 8.0, 11.5), // clean
            poly(
                m,
                &[
                    (6.0, 6.0),
                    (9.0, 6.0),
                    (9.0, 7.0),
                    (7.5, 7.0),
                    (7.0, 7.5),
                    (7.0, 9.0),
                    (6.0, 9.0),
                ],
            ), // clean
        ],
    );

    // h3 — shapes that merge.  Two overlapping 0.10 boxes whose union is 0.16 wide are
    // clean, 0.155 fires once; four abutting 0.04 slices make 0.16 (clean), 0.04 × 3 +
    // 0.035 make 0.155 (fires); a 0.16 bar drawn as a 4 × 10 grid is clean; a ring with
    // one 0.155 wall fires once; an island 0.155 wide in a ring's hole fires once.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.1, 4.0),
        rect(m, 2.06, 2.0, 2.16, 4.0), // union 0.16 → clean
        rect(m, 4.0, 2.0, 4.1, 4.0),
        rect(m, 4.055, 2.0, 4.155, 4.0), // union 0.155 → M1.a
    ];
    for i in 0..4 {
        let x = 6.0 + 0.04 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.04, 4.0)); // 0.16 → clean
    }
    for i in 0..3 {
        let x = 8.0 + 0.04 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.04, 4.0));
    }
    e.push(rect(m, 8.12, 2.0, 8.155, 4.0)); // 0.155 → M1.a
    for i in 0..4 {
        for j in 0..10 {
            let (x, y) = (10.0 + 0.04 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(m, x, y, x + 0.04, y + 0.2)); // grid 0.16 → clean
        }
    }
    e.extend(l.ring(m, 2.0, 6.0, 5.0, 9.0, 2.155, 6.5, 4.5, 8.5)); // left wall 0.155 → M1.a
    e.extend(l.ring(m, 6.0, 6.0, 9.0, 9.0, 6.5, 6.5, 8.5, 8.5));
    e.push(rect(m, 7.0, 7.0, 8.0, 8.0)); // island 1.0 wide → clean
    e.extend(l.ring(m, 10.0, 6.0, 13.0, 9.0, 10.5, 6.5, 12.5, 8.5));
    e.push(rect(m, 11.0, 7.0, 11.155, 8.0)); // island 0.155 → M1.a
    write("M1.a.h3", e);

    // h4 — tile lines.  0.155 bars ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, inside a tile at 10; 0.155-tall bars
    // across 20/21 and 40/42; an L cornered on x = 20 with the vertical arm narrow.  Ten
    // whatever the tile; the 0.16 controls are clean.
    write(
        "M1.a.h4",
        vec![
            rect(m, 9.845, 2.0, 10.0, 4.0),
            rect(m, 19.845, 2.0, 20.0, 4.0),
            rect(m, 19.92, 6.0, 20.075, 8.0),
            rect(m, 20.0, 10.0, 20.155, 12.0),
            rect(m, 20.92, 14.0, 21.075, 16.0),
            rect(m, 39.845, 2.0, 40.0, 4.0),
            rect(m, 41.92, 6.0, 42.075, 8.0),
            rect(m, 15.0, 18.0, 25.0, 18.155),
            rect(m, 35.0, 18.0, 45.0, 18.155),
            poly(
                m,
                &[
                    (20.0, 22.0),
                    (23.0, 22.0),
                    (23.0, 23.0),
                    (20.155, 23.0),
                    (20.155, 26.0),
                    (20.0, 26.0),
                ],
            ),
            rect(m, 19.92, 28.0, 20.08, 30.0), // 0.16 straddling 20 → clean
            rect(m, 15.0, 32.0, 25.0, 32.16),  // 0.16 across 20 → clean
        ],
    );

    // h7 — a comb with three 0.155 teeth (three violations of one polygon) and a U with
    // 0.16 arms (clean).
    write(
        "M1.a.h7",
        vec![
            poly(
                m,
                &[
                    (2.0, 2.0),
                    (3.465, 2.0),
                    (3.465, 4.0),
                    (3.31, 4.0),
                    (3.31, 3.0),
                    (2.81, 3.0),
                    (2.81, 4.0),
                    (2.655, 4.0),
                    (2.655, 3.0),
                    (2.155, 3.0),
                    (2.155, 4.0),
                    (2.0, 4.0),
                ],
            ),
            poly(
                m,
                &[
                    (6.0, 2.0),
                    (8.0, 2.0),
                    (8.0, 4.0),
                    (7.84, 4.0),
                    (7.84, 3.0),
                    (6.16, 3.0),
                    (6.16, 4.0),
                    (6.0, 4.0),
                ],
            ),
        ],
    );
}

// --- M1.b: min. Metal1 space or notch 0.18 ---

fn m1_b_h(l: &L) {
    let m = l.m1;

    // h1 — the bound and both metrics.  Gap 0.175 fires; a diagonal offset of 0.13/0.13 is
    // 0.1838 corner to corner (clean), 0.125/0.125 is 0.1768 (fires); an x-gap of 0.175
    // between boxes that meet corner-on in projection fires; 0.18 is clean.
    write(
        "M1.b.h1",
        vec![
            rect(m, 2.0, 2.0, 3.0, 3.0),
            rect(m, 3.175, 2.0, 4.175, 3.0), // M1.b
            rect(m, 6.0, 2.0, 7.0, 3.0),
            rect(m, 7.13, 3.13, 8.13, 4.13), // clean (0.1838)
            rect(m, 10.0, 2.0, 11.0, 3.0),
            rect(m, 11.125, 3.125, 12.125, 4.125), // M1.b (0.1768)
            rect(m, 2.0, 6.0, 3.0, 7.0),
            rect(m, 3.175, 7.0, 4.175, 8.0), // M1.b (corner-on 0.175)
            rect(m, 6.0, 6.0, 7.0, 7.0),
            rect(m, 7.18, 6.5, 8.18, 7.5), // clean
        ],
    );

    // h2 — 45° geometry at 0.175: a diamond tip above a wall, two parallel 45° strips,
    // a box corner facing a chamfer, tip to tip; every one of them is M1.i as well (0.22
    // for a 45° neighbour) and the case ignores M1.i.  Diamond a = 0.5 keeps M1.a quiet.
    // The strips: d = 0.4 → 0.566 wide; a second strip dy higher is (dy − 0.8)/√2 away,
    // dy = 1.05 → 0.1768.  The chamfer along X + Y = 16.5 is 0.1768 from the box corner
    // (12.75, 4.0), whose foot (12.625, 3.875) lies on the chamfer.
    write(
        "M1.b.h2",
        vec![
            rect(m, 2.0, 2.0, 5.0, 3.0),
            diamond(m, 3.5, 3.675, 0.5), // M1.b: tip 0.175 above the wall
            strip45(m, 7.0, 2.0, 2.0, 0.4),
            strip45(m, 7.0, 3.05, 2.0, 0.4), // M1.b: strips 0.1768 apart
            chamfered_tr(m, 11.0, 2.0, 13.0, 4.0, 16.5),
            rect(m, 12.75, 4.0, 14.5, 5.5), // M1.b: corner 0.1768 from the chamfer
            diamond(m, 17.0, 3.0, 0.5),
            diamond(m, 18.175, 3.0, 0.5), // M1.b: tips 0.175 apart
        ],
    );

    // h3 — "space or notch".  A straight U notch of 0.175 (with a 0.18 control), a
    // straight-vs-45° notch of 0.175 (control 0.18), a comb with three 0.175 slots, a 0.175
    // slot into a plate, a keyhole ring with a 0.175 hole, two Ls facing across 0.175 and
    // an island 0.175 from a ring's inner wall.  Eleven; the 45° notch is M1.i too.
    let u = |x: f64, nd: f64| {
        poly(
            m,
            &[
                (x, 2.0),
                (x + 1.0, 2.0),
                (x + 1.0, 2.25),
                (x + 0.5, 2.25),
                (x + 0.5, 2.25 + nd),
                (x + 1.0, 2.25 + nd),
                (x + 1.0, 3.0),
                (x, 3.0),
            ],
        )
    };
    let u45 = |x: f64, nd: f64| {
        poly(
            m,
            &[
                (x, 2.0),
                (x + 1.0, 2.0),
                (x + 1.0, 2.25),
                (x + 0.5, 2.25),
                (x + 0.5, 2.25 + nd),
                (x + 1.0, 2.75 + nd),
                (x + 1.0, 3.2),
                (x, 3.2),
            ],
        )
    };
    let mut e = vec![u(2.0, 0.175), u(4.0, 0.18), u45(6.0, 0.175), u45(8.0, 0.18)];
    e.push(poly(
        m,
        &[
            (10.0, 2.0),
            (12.0, 2.0),
            (12.0, 3.0),
            (11.825, 3.0),
            (11.825, 2.4),
            (11.65, 2.4),
            (11.65, 3.0),
            (11.325, 3.0),
            (11.325, 2.4),
            (11.15, 2.4),
            (11.15, 3.0),
            (10.825, 3.0),
            (10.825, 2.4),
            (10.65, 2.4),
            (10.65, 3.0),
            (10.0, 3.0),
        ],
    )); // comb: three 0.175 slots
    e.push(poly(
        m,
        &[
            (14.0, 2.0),
            (16.0, 2.0),
            (16.0, 4.0),
            (15.085, 4.0),
            (15.085, 3.0),
            (14.91, 3.0),
            (14.91, 4.0),
            (14.0, 4.0),
        ],
    )); // slot 0.175 wide into a plate
    e.push(l.keyhole(m, 2.0, 6.0, 4.0, 8.0, 2.91, 6.5, 3.085, 7.5)); // hole 0.175 wide
    e.push(poly(
        m,
        &[
            (6.0, 6.0),
            (8.0, 6.0),
            (8.0, 6.5),
            (6.5, 6.5),
            (6.5, 8.0),
            (6.0, 8.0),
        ],
    ));
    e.push(poly(
        m,
        &[
            (6.8, 6.675),
            (8.5, 6.675),
            (8.5, 8.5),
            (8.0, 8.5),
            (8.0, 7.175),
            (6.8, 7.175),
        ],
    )); // Ls facing across 0.175
    e.extend(l.ring(m, 10.0, 6.0, 13.0, 9.0, 10.5, 6.5, 12.5, 8.5));
    e.push(rect(m, 11.0, 6.675, 12.0, 7.675)); // island 0.175 above the inner wall
    write("M1.b.h3", e);

    // h4 — unions.  Overlapping, abutting and gridded boxes each 0.175 from a third box:
    // one marker each, the gap read against the merged shape.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.6, 3.0),
        rect(m, 2.4, 2.0, 3.0, 3.0),
        rect(m, 3.175, 2.0, 4.0, 3.0), // M1.b
        rect(m, 6.0, 2.0, 6.5, 3.0),
        rect(m, 6.5, 2.0, 7.0, 3.0),
        rect(m, 7.175, 2.0, 8.0, 3.0), // M1.b
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (10.0 + 0.2 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(m, x, y, x + 0.2, y + 0.2));
        }
    }
    e.push(rect(m, 11.175, 2.0, 12.0, 3.0)); // M1.b
    write("M1.b.h4", e);

    // h5 — tile lines.  0.175 gaps ending on x = 20, straddling 20 (19.9..20.075),
    // starting on 20, straddling 21, on 40, straddling 42, inside a tile at 10; two in y
    // running across 20/21 and 40/42; a corner-to-corner pair (0.1768) across (20, 20).
    // Ten.
    let pair = |x: f64, y: f64| {
        vec![
            rect(m, x - 1.0, y, x, y + 1.0),
            rect(m, x + 0.175, y, x + 1.175, y + 1.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(9.825, 2.0));
    e.extend(pair(19.825, 2.0));
    e.extend(pair(19.9, 4.0));
    e.extend(pair(20.0, 6.0));
    e.extend(pair(20.9, 8.0));
    e.extend(pair(39.825, 2.0));
    e.extend(pair(41.9, 4.0));
    e.push(rect(m, 15.0, 12.0, 25.0, 13.0));
    e.push(rect(m, 15.0, 13.175, 25.0, 14.0));
    e.push(rect(m, 35.0, 12.0, 45.0, 13.0));
    e.push(rect(m, 35.0, 13.175, 45.0, 14.0));
    e.push(rect(m, 19.0, 19.0, 20.0, 20.0));
    e.push(rect(m, 20.125, 20.125, 21.0, 21.0));
    write("M1.b.h5", e);

    // h8 — small, long, far.  A 0.005 sliver 0.175 from a box (the sliver is M1.a and
    // M1.d), two 300 µm bars 0.175 apart (one marker), a pair at (1000, 1000).
    write(
        "M1.b.h8",
        vec![
            rect(m, 2.0, 2.0, 3.0, 3.0),
            rect(m, 3.175, 2.0, 3.18, 3.0),
            rect(m, 2.0, 6.0, 302.0, 7.0),
            rect(m, 2.0, 7.175, 302.0, 8.0),
            rect(m, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(m, 1001.175, 1000.0, 1002.0, 1001.0),
        ],
    );

    // h9 — other layers and nets.  Metal1 0.175 from Metal1:filler is M1Fil.c's (0.42), from
    // Metal1:mask nobody's; two Metal1 shapes on one net (joined by Via1 and a Metal2
    // strap) 0.175 apart are still M1.b - the rule says nothing about nets; a Cont under
    // each of two shapes changes nothing.  Two M1.b.
    let mut e = vec![
        rect(m, 2.0, 2.0, 3.0, 3.0),
        rect(l.fill, 3.175, 2.0, 4.175, 3.0), // M1Fil.c, not M1.b
        rect(m, 6.0, 2.0, 7.0, 3.0),
        rect(l.mask, 7.175, 2.0, 8.175, 3.0), // nothing
        rect(m, 10.0, 2.0, 11.0, 3.0),
        rect(m, 11.175, 2.0, 12.175, 3.0), // M1.b, same net
    ];
    e.extend(l.via_up(10.5, 2.5));
    e.extend(l.via_up(11.675, 2.5));
    e.push(l.m2_strap(10.3, 2.3, 11.875, 2.7));
    e.push(rect(m, 14.0, 2.0, 15.0, 3.0));
    e.push(rect(m, 15.175, 2.0, 16.175, 3.0)); // M1.b
    e.extend(l.tapped_cont(14.5, 2.5));
    e.extend(l.tapped_cont(15.675, 2.5));
    write("M1.b.h9", e);
}

// --- M1.c: min. Metal1 enclosure of Cont 0.00 ---

fn m1_c_h(l: &L) {
    let m = l.m1;
    // A Cont centred on (cx, cy) spans cx ± 0.08; every Cont sits on a 0.4 Activ tap.
    let c = |cx: f64, cy: f64| l.tapped_cont(cx, cy);

    // h1 — the bound.  A Cont with 0.12 all round, one flush on the left (0.00) and one
    // in a 0.16 line (flush on both long sides) are clean; a Cont sticking 0.005 out on
    // the left, right, bottom or top fires once each; a Cont half out of the metal fires;
    // a Cont with no Metal1 at all is enclosed by nothing (M1.c and Cnt.h).  Six.
    let mut e = vec![];
    e.extend(c(2.5, 2.5));
    e.push(rect(m, 2.3, 2.3, 2.7, 2.7)); // clean
    e.extend(c(4.5, 2.5));
    e.push(rect(m, 4.42, 2.3, 4.8, 2.7)); // clean, flush left
    e.extend(c(6.5, 2.5));
    e.push(rect(m, 5.5, 2.42, 7.5, 2.58)); // clean, 0.16 line
    e.extend(c(2.5, 5.0));
    e.push(rect(m, 2.425, 4.8, 2.7, 5.2)); // M1.c: 0.005 out on the left
    e.extend(c(4.5, 5.0));
    e.push(rect(m, 4.3, 4.8, 4.575, 5.2)); // M1.c: right
    e.extend(c(6.5, 5.0));
    e.push(rect(m, 6.3, 4.925, 6.7, 5.2)); // M1.c: bottom
    e.extend(c(8.5, 5.0));
    e.push(rect(m, 8.3, 4.8, 8.7, 5.075)); // M1.c: top
    e.extend(c(2.5, 7.5));
    e.push(rect(m, 2.5, 7.3, 2.9, 7.7)); // M1.c: half out
    e.extend(c(4.5, 7.5)); // M1.c: no Metal1
    write("M1.c.h1", e);

    // h2 — shapes.  A Cont at a plate's corner, flush on two sides, is covered (M1.c1's
    // business, ignored); a Cont over the seam of two abutting boxes and one over two
    // overlapping boxes are covered by the union; a Cont across a 0.005 gap between two
    // boxes has a 0.005 strip uncovered (fires; the gap is M1.b); a Cont across the 0.1
    // hole of a ring fires; a 0.1 chamfer cutting the Cont's corner (X + Y = 7.6 against
    // the corner at 7.66, the walls 0.02 off the Cont so no wedge is narrow) fires, a
    // chamfer exactly through the corner (X + Y = 7.66) is clean; a Cont under a 3 × 3
    // grid of boxes that cover it is clean.  Three; the 0.02 pads are M1.c1.
    let mut e = vec![];
    e.extend(c(2.5, 2.5));
    e.push(rect(m, 2.42, 2.42, 3.0, 3.0)); // clean (M1.c1)
    e.extend(c(4.5, 2.5));
    e.push(rect(m, 4.3, 2.3, 4.5, 2.7));
    e.push(rect(m, 4.5, 2.3, 4.7, 2.7)); // clean, seam through the Cont
    e.extend(c(6.5, 2.5));
    e.push(rect(m, 6.3, 2.3, 6.55, 2.7));
    e.push(rect(m, 6.45, 2.3, 6.7, 2.7)); // clean, overlap
    e.extend(c(8.5, 2.5));
    e.push(rect(m, 8.3, 2.3, 8.5, 2.7));
    e.push(rect(m, 8.505, 2.3, 8.7, 2.7)); // M1.c: 0.005 uncovered
    e.extend(c(10.5, 2.5));
    e.extend(l.ring(m, 9.5, 1.5, 11.5, 3.5, 10.45, 2.0, 10.55, 3.0)); // M1.c: hole
    e.extend(c(2.5, 5.0));
    e.push(chamfered_tr(m, 2.32, 4.82, 2.6, 5.1, 7.6)); // M1.c: corner cut (X + Y = 7.6 < 7.66)
    e.extend(c(4.5, 5.0));
    e.push(chamfered_tr(m, 4.32, 4.82, 4.6, 5.1, 9.66)); // clean, chamfer through the corner
    e.extend(c(6.5, 5.0));
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = (6.3 + 0.14 * i as f64, 4.8 + 0.14 * j as f64);
            e.push(rect(m, x, y, x + 0.14, y + 0.14)); // clean, grid covers 6.3..6.72
        }
    }
    write("M1.c.h2", e);

    // h3 — tile lines.  Conts sticking 0.005 out to the right of a metal ending on x = 20,
    // 21, 40, 42 and at 10, one sticking out at (1000, 1000); a Cont straddling x = 20 and
    // one straddling 40 with 0.12 all round are clean.  Six.
    let mut e = vec![];
    for x in [10.0, 20.0, 21.0, 40.0, 42.0, 1000.0] {
        e.extend(c(x - 0.075, 2.5)); // Cont x − 0.155 .. x + 0.005
        e.push(rect(m, x - 0.4, 2.3, x, 2.7)); // M1.c
    }
    e.extend(c(20.0, 5.0));
    e.push(rect(m, 19.8, 4.8, 20.2, 5.2)); // clean
    e.extend(c(40.0, 5.0));
    e.push(rect(m, 39.8, 4.8, 40.2, 5.2)); // clean
    write("M1.c.h3", e);

    // h4/h5 — fifty Conts sticking 0.005 out, flat and as an array reference.
    let mut cell = c(0.5, 0.5);
    cell.push(rect(m, 0.3, 0.3, 0.575, 0.7));
}

// --- M1.c1: min. Metal1 endcap enclosure of Cont 0.05 (note 1: at a Metal1 corner at
// least one side must be an endcap, the other sides may be M1.c's 0.00) ---

fn m1_c1_h(l: &L) {
    let m = l.m1;
    let c = |cx: f64, cy: f64| l.tapped_cont(cx, cy);

    // Reading: a side under 0.05 is a line running past the Cont (M1.c), and a line has
    // one such side (at a corner, the other being the endcap) or two opposite ones.  Two
    // adjacent sides under 0.05, or three, or four, leave a corner of the Cont with no
    // endcap: M1.c1.  This is IHP's `one_side_allowed, two_opposite_sides_allowed`.

    // h1 — line ends.  A Cont in a 0.16 line: endcap 0.05 is clean, 0.045 fires, 0.00
    // fires, the Cont mid-line is clean.  A Cont at the end of a 0.30 line (0.07 sides)
    // with a 0.045 endcap has one short side: clean by the reading above (debatable, see
    // the report); at the end of a 0.26 line (0.05 sides) the same; at the end of a 0.25
    // line (0.045 sides) three sides are short: fires.  A 0.255 stub (0.05 left, 0.045
    // right) round a Cont fires.  Four.
    let mut e = vec![];
    e.extend(c(2.5, 2.5));
    e.push(rect(m, 1.5, 2.42, 2.63, 2.58)); // clean, endcap 0.05
    e.extend(c(4.5, 2.5));
    e.push(rect(m, 3.5, 2.42, 4.625, 2.58)); // M1.c1: endcap 0.045
    e.extend(c(6.5, 2.5));
    e.push(rect(m, 5.5, 2.42, 6.58, 2.58)); // M1.c1: endcap 0.00
    e.extend(c(8.5, 2.5));
    e.push(rect(m, 7.5, 2.42, 9.5, 2.58)); // clean, mid-line
    e.extend(c(2.5, 5.0));
    e.push(rect(m, 1.5, 4.85, 2.625, 5.15)); // clean: one short side (0.30 line)
    e.extend(c(4.5, 5.0));
    e.push(rect(m, 3.5, 4.87, 4.625, 5.13)); // clean: one short side (0.26 line)
    e.extend(c(6.5, 5.0));
    e.push(rect(m, 5.5, 4.875, 6.625, 5.125)); // M1.c1: three short sides (0.25 line)
    e.extend(c(8.5, 5.0));
    e.push(rect(m, 8.37, 4.92, 8.625, 5.08)); // M1.c1: stub 0.05/0.045, sides 0.00
    write("M1.c1.h1", e);

    // h2 — corners of a 0.7 plate.  Flush on two sides (0.00/0.00) fires; 0.05 from the
    // left edge and flush at the bottom is clean; 0.045/0.045 fires; 0.045 left and 0.05
    // bottom is clean.  In a pad: 0.02 all round fires; 0.05 left and 0.02 elsewhere fires
    // (three short sides); 0.05 left and right with 0.02 top and bottom is clean (two
    // opposite short sides), and so is 0.02 left/right with 0.05 top/bottom.  Four.
    let mut e = vec![];
    e.extend(c(2.38, 2.38));
    e.push(rect(m, 2.3, 2.3, 3.0, 3.0)); // M1.c1: flush left and bottom
    e.extend(c(4.43, 2.38));
    e.push(rect(m, 4.3, 2.3, 5.0, 3.0)); // clean: 0.05 left, flush bottom
    e.extend(c(6.425, 2.425));
    e.push(rect(m, 6.3, 2.3, 7.0, 3.0)); // M1.c1: 0.045/0.045
    e.extend(c(8.425, 2.43));
    e.push(rect(m, 8.3, 2.3, 9.0, 3.0)); // clean: 0.045 left, 0.05 bottom
    e.extend(c(2.5, 5.0));
    e.push(rect(m, 2.4, 4.9, 2.6, 5.1)); // M1.c1: 0.02 all round
    e.extend(c(4.5, 5.0));
    e.push(rect(m, 4.37, 4.9, 4.6, 5.1)); // M1.c1: 0.05 left, 0.02 elsewhere
    e.extend(c(6.5, 5.0));
    e.push(rect(m, 6.37, 4.9, 6.63, 5.1)); // clean: 0.05 left/right, 0.02 top/bottom
    e.extend(c(8.5, 5.0));
    e.push(rect(m, 8.4, 4.87, 8.6, 5.13)); // clean: 0.02 left/right, 0.05 top/bottom
    write("M1.c1.h2", e);

    // h3 — pads that are no rectangle.  A Cont in the inner corner of an L (arms 0.31
    // wide, its corner 0.05 short of the inner corner in x and in y, the arms running on
    // past it) is clean: each of its edges is enclosed by the arm running past; the same
    // with 0.045 leaves the Cont's corner under a diagonal 0.064 of metal and is clean by
    // projection too (noted in the report).  A Cont under two abutting boxes, the seam
    // through it: 0.05 all round is clean, 0.045 on the top and the right fires.  A plate
    // corner chamfered along X + Y = 7.77, passing 0.078 from the Cont's corner (2.58,
    // 5.08) with both walls 0.12 away, is clean by projection (settled).  One.
    let mut e = vec![];
    e.extend(c(2.5, 2.5));
    e.push(rect(m, 2.32, 2.32, 3.2, 2.63));
    e.push(rect(m, 2.32, 2.32, 2.63, 3.2)); // clean: L, 0.05 to the inner corner
    e.extend(c(4.5, 2.5));
    e.push(rect(m, 4.32, 2.32, 5.2, 2.625));
    e.push(rect(m, 4.32, 2.32, 4.625, 5.2)); // clean: L, 0.045 to the inner corner
    e.extend(c(6.5, 2.5));
    e.push(rect(m, 6.37, 2.37, 6.5, 2.63));
    e.push(rect(m, 6.5, 2.37, 6.63, 2.63)); // clean: seam, 0.05 all round
    e.extend(c(8.5, 2.5));
    e.push(rect(m, 8.37, 2.37, 8.5, 2.625));
    e.push(rect(m, 8.5, 2.37, 8.625, 2.625)); // M1.c1: seam, 0.045 top and right
    e.extend(c(2.5, 5.0));
    e.push(chamfered_tr(m, 2.3, 4.8, 2.7, 5.2, 7.77)); // clean: chamfer 0.078 from the corner
    write("M1.c1.h3", e);

    // h4 — tile lines.  0.045 endcaps with the line's end on x = 20 (Cont 19.795..19.955),
    // on 21, 40, 42 and 10, at (1000, 1000); a Cont straddling x = 20 with its 0.045
    // endcap at 20.125; a 0.05 endcap straddling 40 is clean.  Seven.
    let mut e = vec![];
    for (x, y) in [
        (10.0, 2.5),
        (20.0, 2.5),
        (21.0, 8.0),
        (40.0, 2.5),
        (42.0, 8.0),
        (1000.0, 2.5),
    ] {
        e.extend(c(x - 0.125, y));
        e.push(rect(m, x - 1.0, y - 0.08, x, y + 0.08)); // M1.c1
    }
    e.extend(c(20.0, 5.0));
    e.push(rect(m, 19.0, 4.92, 20.125, 5.08)); // M1.c1
    e.extend(c(40.0, 5.0));
    e.push(rect(m, 39.0, 4.92, 40.13, 5.08)); // clean
    write("M1.c1.h4", e);

    // h5/h6 — fifty 0.045 endcaps, flat and as an array reference.
    let mut cell = c(0.5, 0.5);
    cell.push(rect(m, 0.0, 0.42, 0.625, 0.58));
}

// --- M1.d: min. Metal1 area 0.09 µm² ---

fn m1_d_h(l: &L) {
    let m = l.m1;

    // h1 — the bound.  0.30 × 0.30 (0.09) is clean; 0.30 × 0.295 (0.0885), a 0.16 × 0.56
    // line (0.0896), an L of 0.16 arms 0.35 long (0.0864), a diamond a = 0.21 (0.0882) and
    // a 0.30 square with 0.05 chamfers (0.085) fire; 0.16 × 0.565 (0.0904) and a diamond
    // a = 0.215 (0.0925) are clean.  Five.
    write(
        "M1.d.h1",
        vec![
            rect(m, 2.0, 2.0, 2.3, 2.3),    // clean
            rect(m, 3.0, 2.0, 3.3, 2.295),  // M1.d
            rect(m, 4.0, 2.0, 4.16, 2.56),  // M1.d
            rect(m, 5.0, 2.0, 5.16, 2.565), // clean
            poly(
                m,
                &[
                    (6.0, 2.0),
                    (6.35, 2.0),
                    (6.35, 2.16),
                    (6.16, 2.16),
                    (6.16, 2.35),
                    (6.0, 2.35),
                ],
            ), // M1.d
            diamond(m, 7.5, 2.5, 0.21),     // M1.d
            diamond(m, 8.5, 2.5, 0.215),    // clean
            poly(
                m,
                &[
                    (9.05, 2.0),
                    (9.25, 2.0),
                    (9.3, 2.05),
                    (9.3, 2.25),
                    (9.25, 2.3),
                    (9.05, 2.3),
                    (9.0, 2.25),
                    (9.0, 2.05),
                ],
            ), // M1.d
        ],
    );

    // h2 — shapes that merge.  Two overlapping 0.2 × 0.3 boxes (0.06 each) whose union is
    // 0.2 × 0.35 (0.07) fire once; two 0.2 × 0.3 boxes meeting at one corner are two
    // regions and fire twice; two abutting 0.2 × 0.3 boxes (0.12) are clean; a 0.3 square
    // drawn as a 3 × 3 grid is clean; an island 0.2 × 0.2 in a ring's hole fires, the ring
    // does not.  Four.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.2, 2.3),
        rect(m, 2.0, 2.05, 2.2, 2.35), // M1.d, union 0.07
        rect(m, 3.0, 2.0, 3.2, 2.3),
        rect(m, 3.2, 2.3, 3.4, 2.6), // M1.d × 2, corner touch
        rect(m, 4.0, 2.0, 4.2, 2.3),
        rect(m, 4.2, 2.0, 4.4, 2.3), // clean, 0.12
    ];
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = (5.0 + 0.1 * i as f64, 2.0 + 0.1 * j as f64);
            e.push(rect(m, x, y, x + 0.1, y + 0.1)); // clean, 0.09
        }
    }
    e.extend(l.ring(m, 6.0, 2.0, 8.0, 4.0, 6.5, 2.5, 7.5, 3.5));
    e.push(rect(m, 6.9, 2.9, 7.1, 3.1)); // M1.d, island 0.04
    write("M1.d.h2", e);

    // h3 — tile lines.  0.30 × 0.295 boxes ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, inside a tile at 10; a 0.16 × 0.56 bar
    // across x = 20; one at (1000, 1000); a 0.30 square straddling 20 is clean.  Nine.
    let mut e = vec![];
    for (i, x) in [9.7, 19.7, 19.85, 20.0, 20.85, 39.7, 41.85]
        .iter()
        .enumerate()
    {
        let y = 2.0 + i as f64;
        e.push(rect(m, *x, y, x + 0.3, y + 0.295));
    }
    e.push(rect(m, 19.72, 10.0, 20.28, 10.16));
    e.push(rect(m, 1000.0, 1000.0, 1000.3, 1000.295));
    e.push(rect(m, 19.85, 12.0, 20.15, 12.3)); // clean
    write("M1.d.h3", e);

    // h6 — small and long.  A 0.005 × 0.5 sliver (0.0025) fires; a 0.005 × 18 sliver is
    // 0.09 and clean by area (both are M1.a); a 300 µm bar is clean.  One.
    write(
        "M1.d.h6",
        vec![
            rect(m, 2.0, 2.0, 2.005, 2.5),
            rect(m, 4.0, 2.0, 4.005, 20.0),
            rect(m, 2.0, 24.0, 302.0, 24.5),
        ],
    );
}

// --- M1.e: min. space 0.22 of Metal1 lines if at least one line is wider than 0.3 and
// the parallel run is more than 1.0 ---

fn m1_e_h(l: &L) {
    let m = l.m1;

    // h1 — the width bound.  A 0.305 line 2 µm long beside a 0.16 line at 0.20 fires; a
    // 0.30 line (not wider than 0.3) beside one at 0.20 is clean; two 0.16 lines at 0.20
    // are clean; two 0.5 lines at 0.215 fire, at 0.22 they are clean; a 2 × 2 plate with a
    // 0.16 line along it at 0.20 fires.  Three.
    write(
        "M1.e.h1",
        vec![
            rect(m, 2.0, 2.0, 2.305, 4.0),
            rect(m, 2.505, 2.0, 2.665, 4.0), // M1.e
            rect(m, 5.0, 2.0, 5.3, 4.0),
            rect(m, 5.5, 2.0, 5.66, 4.0), // clean
            rect(m, 8.0, 2.0, 8.16, 4.0),
            rect(m, 8.36, 2.0, 8.52, 4.0), // clean
            rect(m, 11.0, 2.0, 11.5, 4.0),
            rect(m, 11.715, 2.0, 12.215, 4.0), // M1.e
            rect(m, 14.0, 2.0, 14.5, 4.0),
            rect(m, 14.72, 2.0, 15.22, 4.0), // clean
            rect(m, 17.0, 2.0, 19.0, 4.0),
            rect(m, 19.2, 2.0, 19.36, 4.0), // M1.e
        ],
    );

    // h2 — the run bound, at 0.20 with a 0.5 line.  Aligned lines 1.0 long are clean, 1.005
    // long fire; a 3 µm pair offset so the facing walls share 1.0 is clean, 1.005 fires; a
    // 0.8 stub beside a 5 µm wide line is clean, and so are two 0.6 stubs 0.5 apart; a 5 µm
    // 0.16 line broken by a 0.3 gap into two 2.35 pieces fires twice.  Four.
    write(
        "M1.e.h2",
        vec![
            rect(m, 2.0, 2.0, 2.5, 3.0),
            rect(m, 2.7, 2.0, 2.86, 3.0), // clean, run 1.0
            rect(m, 5.0, 2.0, 5.5, 3.005),
            rect(m, 5.7, 2.0, 5.86, 3.005), // M1.e, run 1.005
            rect(m, 8.0, 2.0, 8.5, 5.0),
            rect(m, 8.7, 4.0, 8.86, 7.0), // clean, shared 1.0
            rect(m, 11.0, 2.0, 11.5, 5.0),
            rect(m, 11.7, 3.995, 11.86, 7.0), // M1.e, shared 1.005
            rect(m, 14.0, 2.0, 14.5, 7.0),
            rect(m, 14.7, 3.0, 14.86, 3.8), // clean, stub 0.8
            rect(m, 17.0, 2.0, 17.5, 7.0),
            rect(m, 17.7, 3.0, 17.86, 3.6),
            rect(m, 17.7, 4.1, 17.86, 4.7), // clean, stubs 0.6
            rect(m, 20.0, 2.0, 20.5, 7.0),
            rect(m, 20.7, 2.0, 20.86, 4.35),
            rect(m, 20.7, 4.65, 20.86, 7.0), // M1.e × 2, pieces 2.35
        ],
    );

    // h3 — where the line is wide.  A 0.16 line 4 µm long carrying a 0.5 pad 1.5 long,
    // with a straight 0.16 line 0.20 from the pad: the wide part runs 1.5 beside the
    // neighbour, fires; the same pad 0.8 long is clean (the lines run parallel for 4 µm,
    // the wide part for 0.8), and a pad exactly 1.0 long is clean; an L of 0.5 arms with
    // a 0.16 line along the outside of one arm for 1.5 fires.  Two.
    let padded = |x: f64, plen: f64| {
        poly(
            m,
            &[
                (x, 2.0),
                (x + 0.16, 2.0),
                (x + 0.16, 3.0),
                (x + 0.5, 3.0),
                (x + 0.5, 3.0 + plen),
                (x + 0.16, 3.0 + plen),
                (x + 0.16, 6.0),
                (x, 6.0),
            ],
        )
    };
    write(
        "M1.e.h3",
        vec![
            padded(2.0, 1.5),
            rect(m, 2.7, 2.0, 2.86, 6.0), // M1.e
            padded(5.0, 0.8),
            rect(m, 5.7, 2.0, 5.86, 6.0), // clean
            padded(8.0, 1.0),
            rect(m, 8.7, 2.0, 8.86, 6.0), // clean
            poly(
                m,
                &[
                    (11.0, 2.0),
                    (13.0, 2.0),
                    (13.0, 2.5),
                    (11.5, 2.5),
                    (11.5, 4.0),
                    (11.0, 4.0),
                ],
            ),
            rect(m, 11.0, 1.64, 12.5, 1.8), // M1.e: along the arm's outside for 1.5
        ],
    );

    // h4 — 45° lines and both metrics.  A 0.566 45° band with a 0.17 45° strip parallel
    // to it at 0.198, the facing walls aligned end to end: walls 2.83 long fire, 0.85
    // long are clean, 1.06 long (spanning 0.75 in x and in y) fire - the run is along the
    // walls.  Two 2 × 2
    // plates corner to corner at 0.15/0.15 (0.212, no parallel run) are clean; a 0.16 line
    // ending 0.20 short of a plate, end-on, is clean; two plates stepped so their facing
    // walls share 1.0 are clean, sharing 1.005 they fire.  Three.
    write(
        "M1.e.h4",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.4),
            strip45(m, 1.46, 2.54, 2.0, 0.12), // M1.e
            strip45(m, 6.0, 2.0, 0.6, 0.4),
            strip45(m, 5.46, 2.54, 0.6, 0.12), // clean
            strip45(m, 2.0, 6.0, 0.75, 0.4),
            strip45(m, 1.46, 6.54, 0.75, 0.12), // M1.e, walls 1.06
            rect(m, 9.0, 2.0, 11.0, 4.0),
            rect(m, 11.15, 4.15, 13.15, 6.15), // clean
            rect(m, 15.0, 2.0, 17.0, 4.0),
            rect(m, 15.9, 4.2, 16.06, 6.0), // clean, end-on
            rect(m, 19.0, 2.0, 21.0, 4.0),
            rect(m, 21.2, 3.0, 23.2, 5.0), // clean, shared 1.0
            rect(m, 25.0, 2.0, 27.0, 4.0),
            rect(m, 27.2, 2.995, 29.2, 5.0), // M1.e, shared 1.005
        ],
    );

    // h5 — unions.  A wide line drawn as two overlapping 0.16 boxes (0.305) beside a 0.16
    // line at 0.20 fires; as two abutting slices 0.16 + 0.145 it fires; 0.16 + 0.14 (0.30)
    // is clean; a 0.5 line beside a 0.16 neighbour drawn as two abutting 0.08 halves fires
    // once; beside a neighbour drawn as three collinear 1 µm boxes it fires once.  Four.
    write(
        "M1.e.h5",
        vec![
            rect(m, 2.0, 2.0, 2.16, 4.0),
            rect(m, 2.145, 2.0, 2.305, 4.0),
            rect(m, 2.505, 2.0, 2.665, 4.0), // M1.e
            rect(m, 5.0, 2.0, 5.16, 4.0),
            rect(m, 5.16, 2.0, 5.305, 4.0),
            rect(m, 5.505, 2.0, 5.665, 4.0), // M1.e
            rect(m, 8.0, 2.0, 8.16, 4.0),
            rect(m, 8.16, 2.0, 8.3, 4.0),
            rect(m, 8.5, 2.0, 8.66, 4.0), // clean
            rect(m, 11.0, 2.0, 11.5, 4.0),
            rect(m, 11.7, 2.0, 11.78, 4.0),
            rect(m, 11.78, 2.0, 11.86, 4.0), // M1.e, once
            rect(m, 14.0, 2.0, 14.5, 5.0),
            rect(m, 14.7, 2.0, 14.86, 3.0),
            rect(m, 14.7, 3.0, 14.86, 4.0),
            rect(m, 14.7, 4.0, 14.86, 5.0), // M1.e, once
        ],
    );

    // h6 — tile lines.  The 0.305/0.16 pair at 0.20, 2 µm long, with the gap straddling
    // x = 20, ending on 20, starting on 20, straddling 21, on 40, straddling 42, inside a
    // tile at 10; horizontal pairs running across x = 20 and x = 40; a horizontal pair
    // 1.005 long ending on x = 20; a pair at (1000, 1000).  Eleven.
    let pair = |x: f64, y: f64| {
        vec![
            rect(m, x - 0.305, y, x, y + 2.0),
            rect(m, x + 0.2, y, x + 0.36, y + 2.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(19.9, 2.0));
    e.extend(pair(19.8, 5.0));
    e.extend(pair(20.0, 8.0));
    e.extend(pair(20.9, 11.0));
    e.extend(pair(39.8, 2.0));
    e.extend(pair(41.9, 5.0));
    e.extend(pair(10.0, 2.0));
    e.extend(pair(1000.0, 1000.0));
    e.push(rect(m, 15.0, 15.0, 25.0, 15.305));
    e.push(rect(m, 15.0, 15.505, 25.0, 15.665));
    e.push(rect(m, 35.0, 15.0, 45.0, 15.305));
    e.push(rect(m, 35.0, 15.505, 45.0, 15.665));
    e.push(rect(m, 18.995, 18.0, 20.0, 18.305));
    e.push(rect(m, 18.995, 18.505, 20.0, 18.665));
    write("M1.e.h6", e);

    // h9 — nets, notches, long and small.  Two 0.5 lines on one net (Via1 and a Metal2
    // strap) 0.20 apart fire - the rule says nothing about nets; a U of 0.5 arms with a
    // 0.20 slot 2 µm deep is a notch, and M1.e says "space of lines" where M1.b says "space
    // or notch": clean by the wording (see the report); a 0.5 × 300 line beside a 0.16 × 300
    // line at 0.20 fires once; a 0.005 sliver 2 µm long 0.20 from a 0.5 line fires.  Three.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.5, 4.0),
        rect(m, 2.7, 2.0, 3.2, 4.0), // M1.e, same net
    ];
    e.extend(l.via_up(2.25, 3.0));
    e.extend(l.via_up(2.95, 3.0));
    e.push(l.m2_strap(2.05, 2.8, 3.15, 3.2));
    e.push(poly(
        m,
        &[
            (5.0, 2.0),
            (6.2, 2.0),
            (6.2, 4.5),
            (5.7, 4.5),
            (5.7, 2.5),
            (5.5, 2.5),
            (5.5, 4.5),
            (5.0, 4.5),
        ],
    )); // notch: clean by the wording
    e.push(rect(m, 2.0, 8.0, 302.0, 8.5));
    e.push(rect(m, 2.0, 8.7, 302.0, 8.86)); // M1.e, 300 µm
    e.push(rect(m, 8.0, 2.0, 8.5, 4.0));
    e.push(rect(m, 8.7, 2.0, 8.705, 4.0)); // M1.e, sliver
    write("M1.e.h9", e);
}

// --- M1.f: min. space 0.60 of Metal1 lines if at least one line is wider than 10.0 and
// the parallel run is more than 10.0 ---

fn m1_f_h(l: &L) {
    let m = l.m1;

    // h1 — the bounds.  A 10.005 × 12 plate beside a 0.5 × 12 line at 0.5 fires; a 10.0 ×
    // 12 plate (not wider than 10) at 0.5 is clean; 10.005 wide at 0.595 fires, at 0.60 is
    // clean; a 10.005 × 10 plate beside a 0.5 × 10 line at 0.5 (run 10.0) is clean, 10.005
    // long fires.  Three.  Every gap is over 0.22, so M1.e stays out.
    write(
        "M1.f.h1",
        vec![
            rect(m, 2.0, 2.0, 12.005, 14.0),
            rect(m, 12.505, 2.0, 13.005, 14.0), // M1.f
            rect(m, 16.0, 2.0, 26.0, 14.0),
            rect(m, 26.5, 2.0, 27.0, 14.0), // clean
            rect(m, 30.0, 2.0, 40.005, 14.0),
            rect(m, 40.6, 2.0, 41.1, 14.0), // M1.f, 0.595
            rect(m, 44.0, 2.0, 54.005, 14.0),
            rect(m, 54.605, 2.0, 55.105, 14.0), // clean, 0.60
            rect(m, 2.0, 18.0, 12.005, 28.0),
            rect(m, 12.505, 18.0, 13.005, 28.0), // clean, run 10.0
            rect(m, 16.0, 18.0, 26.005, 28.005),
            rect(m, 26.505, 18.0, 27.005, 28.005), // M1.f, run 10.005
        ],
    );

    // h2 — where the line is wide, and 45°.  A 0.5 line 30 µm long carrying a 10.005 ×
    // 12 pad, with a 0.5 line running the whole 30 µm at 0.5 from the pad: the wide part
    // runs 12 beside it, fires; with an 8 µm pad the lines run parallel for 30 µm but the
    // wide part for 8: clean.  Two 12 × 12 plates stepped to share 10.0 are clean, 10.005
    // fires.  A 10.04-wide 45° band with a 0.509 45° strip parallel at 0.502, the facing
    // walls aligned end to end: walls 10.6 long (spanning 7.5 in x and in y) fire - the
    // run is along the walls; walls 9.19 long are clean.  Three.
    let padded = |x: f64, plen: f64| {
        poly(
            m,
            &[
                (x, 2.0),
                (x + 0.5, 2.0),
                (x + 0.5, 10.0),
                (x + 10.005, 10.0),
                (x + 10.005, 10.0 + plen),
                (x + 0.5, 10.0 + plen),
                (x + 0.5, 32.0),
                (x, 32.0),
            ],
        )
    };
    write(
        "M1.f.h2",
        vec![
            padded(2.0, 12.0),
            rect(m, 12.505, 2.0, 13.005, 32.0), // M1.f
            padded(16.0, 8.0),
            rect(m, 26.505, 2.0, 27.005, 32.0), // clean
            rect(m, 30.0, 2.0, 42.0, 14.0),
            rect(m, 42.5, 4.0, 54.5, 16.0), // clean, shared 10.0
            rect(m, 30.0, 18.0, 42.0, 30.0),
            rect(m, 42.5, 19.995, 54.5, 32.0), // M1.f, shared 10.005
            strip45(m, 70.0, 2.0, 7.5, 7.1),
            strip45(m, 62.545, 9.455, 7.5, 0.36), // M1.f, walls 10.6
            strip45(m, 92.0, 2.0, 6.5, 7.1),
            strip45(m, 84.545, 9.455, 6.5, 0.36), // clean, walls 9.19
        ],
    );

    // h3 — tile lines, long and far.  The 10.005/0.5 pair at 0.5 with the gap straddling
    // x = 20, ending on 20, starting on 20, straddling 40, straddling 42; a horizontal pair
    // 32 µm long across x = 20 and 40; a pair at (1000, 1000); a 10.005 × 300 plate beside
    // a 0.5 × 300 line.  Eight.
    let pair = |x: f64, y: f64| {
        vec![
            rect(m, x - 10.005, y, x, y + 12.0),
            rect(m, x + 0.5, y, x + 1.0, y + 12.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(19.75, 2.0));
    e.extend(pair(19.5, 16.0));
    e.extend(pair(20.0, 30.0));
    e.extend(pair(39.75, 44.0));
    e.extend(pair(41.75, 58.0));
    e.push(rect(m, 14.0, 72.0, 46.0, 82.005));
    e.push(rect(m, 14.0, 82.505, 46.0, 83.005));
    e.extend(pair(1000.0, 1000.0));
    e.push(rect(m, 2.0, 90.0, 302.0, 100.005));
    e.push(rect(m, 2.0, 100.505, 302.0, 101.005));
    write("M1.f.h3", e);

    // h6 — nets, notches, an L.  Two 12 × 12 plates on one net (Via1 and a Metal2 strap)
    // 0.5 apart fire; a U with 10.005 arms and a 0.5 slot 12 deep is a notch, clean by
    // the wording (report); an L of 10.005 arms with a 0.5 line along the outside of one
    // arm for 12 fires.  Two.
    let mut e = vec![
        rect(m, 2.0, 2.0, 14.0, 14.0),
        rect(m, 14.5, 2.0, 26.5, 14.0), // M1.f, same net
    ];
    e.extend(l.via_up(13.0, 8.0));
    e.extend(l.via_up(15.5, 8.0));
    e.push(l.m2_strap(12.8, 7.8, 15.7, 8.2));
    e.push(poly(
        m,
        &[
            (30.0, 2.0),
            (50.51, 2.0),
            (50.51, 16.0),
            (40.505, 16.0),
            (40.505, 4.0),
            (40.005, 4.0),
            (40.005, 16.0),
            (30.0, 16.0),
        ],
    )); // notch
    e.push(poly(
        m,
        &[
            (2.0, 20.0),
            (27.0, 20.0),
            (27.0, 30.005),
            (12.005, 30.005),
            (12.005, 45.0),
            (2.0, 45.0),
        ],
    ));
    e.push(rect(m, 14.0, 30.505, 26.0, 31.005)); // M1.f, along the arm
    write("M1.f.h6", e);
}

// --- M1.g: min. 45°-bent Metal1 width 0.20 if the bent metal length is > 0.5 ---

fn m1_g_h(l: &L) {
    let m = l.m1;

    // h1 — the bound.  45° strips (rotated rectangles, `strip45`: width d·√2, walls
    // len·√2): 0.205 wide with 4.24 walls is clean; 0.198 wide with 4.24 walls fires
    // (two walls); 0.198 with 0.509 walls fires, with 0.495 walls (not > 0.5) is clean; a
    // 0.155 strip 4.24 long is M1.a and M1.g (two each); a 0.198 diamond has 0.198 edges
    // and is clean, a 0.707 diamond too.  M1.g six, M1.a two.
    write(
        "M1.g.h1",
        vec![
            strip45(m, 2.0, 2.0, 3.0, 0.145),  // clean
            strip45(m, 6.0, 2.0, 3.0, 0.14),   // M1.g
            strip45(m, 10.0, 2.0, 0.36, 0.14), // M1.g, walls 0.509
            strip45(m, 12.0, 2.0, 0.35, 0.14), // clean, walls 0.495
            strip45(m, 14.0, 2.0, 3.0, 0.11),  // M1.a + M1.g
            diamond(m, 19.0, 3.0, 0.14),       // clean
            diamond(m, 21.0, 3.0, 0.5),        // clean
        ],
    );

    // h2 — real routes.  A 0.16 Z route whose 45° jog is 0.198 wide with 0.509 walls
    // fires; the same jog with 0.495 walls is clean; a 0.2015 jog is clean; the Z
    // mirrored (running down, drawn clockwise) fires.  An L with a chamfered corner: the
    // 45° walls 0.198 apart, the outer 0.566 and the inner 0.509 long, fires; with the
    // inner 0.4525 long it is clean (a bend is as long as each of its walls, settled);
    // 0.2015 apart it is clean.  Six.
    write(
        "M1.g.h2",
        vec![
            zroute(m, 2.0, 2.0, 0.28, 0.36, false),      // M1.g
            zroute(m, 2.0, 4.0, 0.28, 0.35, false),      // clean
            zroute(m, 2.0, 6.0, 0.285, 0.36, false),     // clean
            zroute(m, 2.0, 10.0, 0.28, 0.36, true),      // M1.g
            chamfered_l(m, 10.0, 2.0, 2.0, 0.40, 0.68),  // M1.g
            chamfered_l(m, 13.0, 2.0, 2.0, 0.36, 0.64),  // clean, inner 0.4525
            chamfered_l(m, 16.0, 2.0, 2.0, 0.40, 0.685), // clean, 0.2015
        ],
    );

    // h3 — tile lines and far.  The firing Z route with its jog straddling x = 20
    // (19.88..20.36), starting on 20, straddling 40 and 42, and at (1000, 1000).  Ten.
    write(
        "M1.g.h3",
        vec![
            zroute(m, 18.0, 2.0, 0.28, 0.36, false),
            zroute(m, 18.12, 5.0, 0.28, 0.36, false),
            zroute(m, 38.0, 2.0, 0.28, 0.36, false),
            zroute(m, 40.0, 5.0, 0.28, 0.36, false),
            zroute(m, 1000.0, 1000.0, 0.28, 0.36, false),
        ],
    );

    // h6 — long and small.  A 300 µm 45° strip 0.198 wide fires (two walls); a 45° sliver
    // 0.007 wide fires (and is M1.a and M1.d).  Four.
    write(
        "M1.g.h6",
        vec![
            strip45(m, 2.0, 2.0, 212.0, 0.14),
            strip45(m, 2.0, 220.0, 2.0, 0.005),
        ],
    );
}

// --- M1.i: min. space 0.22 of Metal1 lines of which at least one is bent by 45° ---

fn m1_i_h(l: &L) {
    let m = l.m1;

    // h1 — the bound, in every direction the value can be taken.  Two parallel 45° strips
    // (0.566 wide) 0.2157 apart fire, 0.2227 apart they are clean; a box corner 0.2121
    // from a chamfer fires, 0.2263 is clean; a diamond tip 0.215 above a wall fires, 0.22
    // is clean; a 45° strip whose tip is 0.215 from a vertical wall fires; a box corner
    // 0.2157 from a strip's 45° wall (its foot on the wall) fires, 0.2298 is clean.  Five.
    write(
        "M1.i.h1",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.4),
            strip45(m, 2.0, 3.105, 2.0, 0.4), // M1.i
            strip45(m, 6.0, 2.0, 2.0, 0.4),
            strip45(m, 6.0, 3.115, 2.0, 0.4), // clean
            chamfered_tr(m, 10.0, 2.0, 12.0, 4.0, 15.5),
            rect(m, 11.8, 4.0, 13.0, 5.0), // M1.i: corner (11.8, 4.0) 0.2121 off X + Y = 15.5
            chamfered_tr(m, 14.0, 2.0, 16.0, 4.0, 19.5),
            rect(m, 15.82, 4.0, 17.0, 5.0), // clean: 0.2263
            rect(m, 18.0, 2.0, 21.0, 3.0),
            diamond(m, 19.5, 3.715, 0.5), // M1.i: tip 0.215 above the wall
            rect(m, 22.0, 2.0, 25.0, 3.0),
            diamond(m, 23.5, 3.72, 0.5), // clean
            strip45(m, 26.0, 2.0, 1.0, 0.4),
            rect(m, 27.215, 2.0, 27.715, 4.0), // M1.i: tip (27, 3) 0.215 from the wall
            strip45(m, 2.0, 8.0, 2.0, 0.4),
            rect(m, 1.0, 9.105, 2.0, 10.105), // M1.i: corner 0.2157 from y = x + 6.8
            strip45(m, 6.0, 8.0, 2.0, 0.4),
            rect(m, 5.0, 9.125, 6.0, 10.125), // clean: 0.2298
        ],
    );

    // h2 — the figure, a notch and a net.  A 0.16 Z route whose 45° jog runs 0.2157 from
    // the parallel 45° wall of a plate above it (figure 5.16's "i") fires; a 45° U (a
    // 1.35-wide 45° strip with a 0.2192 slot 2.26 deep, square-cut) is a notch, and M1.i
    // says "space of lines" where M1.b says "space or notch": clean by the wording (see
    // the report); two 45° strips on one net (Via1 and a Metal2 strap) 0.2157 apart fire.
    // Two.
    let mut e = vec![
        zroute(m, 2.0, 2.0, 0.5, 1.0, false),
        poly(
            m,
            &[
                (2.0, 2.5),
                (3.695, 2.5),
                (4.695, 3.5),
                (4.695, 4.5),
                (2.0, 4.5),
            ],
        ), // M1.i: wall y = x − 1.195 against the jog's y = x − 1.5
        poly(
            m,
            &[
                (10.0, 2.0),
                (12.0, 4.0),
                (11.6, 4.4),
                (10.0, 2.8),
                (9.845, 2.955),
                (11.445, 4.555),
                (11.045, 4.955),
                (9.045, 2.955),
            ],
        ), // the 45° U: a notch
        strip45(m, 16.0, 2.0, 2.0, 0.4),
        strip45(m, 16.0, 3.105, 2.0, 0.4), // M1.i, same net
    ];
    e.extend(l.via_up(17.0, 3.4));
    e.extend(l.via_up(17.0, 4.5));
    e.push(l.m2_strap(16.8, 3.2, 17.2, 4.7));
    write("M1.i.h2", e);

    // h3 — tile lines and far.  The 0.2157 strip pair with its gap straddling x = 20, a
    // pair whose tips end on x = 20, pairs across 40 and 42, one at (1000, 1000).  Five.
    let pair = |x: f64, y: f64| {
        vec![
            strip45(m, x, y, 2.0, 0.4),
            strip45(m, x, y + 1.105, 2.0, 0.4),
        ]
    };
    let mut e = vec![];
    e.extend(pair(19.0, 2.0));
    e.extend(pair(18.0, 8.0));
    e.extend(pair(39.0, 2.0));
    e.extend(pair(41.0, 8.0));
    e.extend(pair(1000.0, 1000.0));
    write("M1.i.h3", e);

    // h6 — long and small.  Two 300 µm 45° strips 0.2157 apart fire once; a 0.007 45°
    // sliver 0.2157 from a strip fires (the sliver is M1.a, M1.d and M1.g).  Two.
    write(
        "M1.i.h6",
        vec![
            strip45(m, 2.0, 2.0, 212.0, 0.4),
            strip45(m, 2.0, 3.105, 212.0, 0.4),
            strip45(m, 2.0, 230.0, 2.0, 0.4),
            strip45(m, 2.0, 231.105, 2.0, 0.005),
        ],
    );

    // h7 — with the other rules.  Two 45° strips 0.1768 apart are M1.b and M1.i; a
    // Metal1:filler corner 0.2157 from a strip's wall is M1Fil.c's (0.42), not M1.i.
    write(
        "M1.i.h7",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.4),
            strip45(m, 2.0, 3.05, 2.0, 0.4), // M1.b + M1.i
            strip45(m, 8.0, 2.0, 2.0, 0.4),
            rect(l.fill, 7.0, 3.105, 8.0, 4.105), // M1Fil.c
        ],
    );
}

// --- Section 6.10: "standard metal and via rules are not checked within EdgeSeal
// regions" ---

fn m1_seal_h(l: &L) {
    let m = l.m1;

    // h1 — inside an EdgeSeal 2..12: a 0.155 bar (M1.a), two boxes 0.175 apart (M1.b), a
    // 0.30 × 0.295 box (M1.d), a Cont in a 0.20 pad (M1.c1) and one sticking 0.005 out
    // (M1.c), a 0.305/0.16 pair at 0.20 (M1.e), two 45° strips 0.2157 apart (M1.i), a
    // 0.198 45° strip (M1.g): none of it is checked.  Outside, a 0.155 bar fires (two
    // walls); a 0.155 bar crossing the seal's edge is a 0.155 line outside the seal as
    // far as it sticks out and fires too.  M1.a four.
    let mut e = vec![
        rect(l.seal, 2.0, 2.0, 12.0, 12.0),
        rect(m, 3.0, 3.0, 3.155, 5.0),
        rect(m, 4.0, 3.0, 5.0, 4.0),
        rect(m, 5.175, 3.0, 6.175, 4.0),
        rect(m, 7.0, 3.0, 7.3, 3.295),
        rect(m, 8.4, 2.9, 8.6, 3.1),
        rect(m, 9.425, 2.8, 9.7, 3.2),
        rect(m, 3.0, 6.0, 3.305, 8.0),
        rect(m, 3.505, 6.0, 3.665, 8.0),
        strip45(m, 5.0, 6.0, 2.0, 0.4),
        strip45(m, 5.0, 7.105, 2.0, 0.4),
        strip45(m, 9.0, 6.0, 2.0, 0.14),
        rect(m, 15.0, 3.0, 15.155, 5.0),   // M1.a
        rect(m, 11.0, 10.0, 11.155, 14.0), // M1.a: crosses the seal's edge
    ];
    e.extend(l.tapped_cont(8.5, 3.0));
    e.extend(l.tapped_cont(9.5, 3.0));
    write("M1.seal.h1", e);
}

// --- M1.j/M1.k, M1Fil.h/k: the density rules ---

fn m1_j_h(l: &L) {
    // h1 — coincident layers.  Ten 30 µm stripes at a 100 µm pitch drawn on Metal1,
    // Metal1:filler and Metal1:mask alike: 30 % of the die is metal.  M1.j (35 %) fires,
    // nothing else does (M1Fil.h's 25 % is met in every window).  The filler stripes are
    // M1Fil.a2 by their length, ignored.
    let mut e = vec![rect(l.boundary, 0.0, 0.0, 1000.0, 1000.0)];
    for layer in [l.m1, l.fill, l.mask] {
        e.extend(stripes(layer, 1000.0, 30.0));
    }
    write("M1.j.h1", e);

    // h2 — slits.  Ten 36 µm Metal1 stripes (36 %) each holding eight 20 × 30 Metal1:slit
    // boxes (4.8 % of the die): Metal1:slit is where the metal is cut away at mask
    // generation (section 7.3), so 31.2 % of the die is metal and M1.j fires.
    let mut e = vec![rect(l.boundary, 0.0, 0.0, 1000.0, 1000.0)];
    e.extend(stripes(l.m1, 1000.0, 36.0));
    let slit = l.slit;
    for k in 0..10 {
        let y0 = k as f64 * 100.0 + 3.0;
        for i in 0..8 {
            let x0 = 100.0 + i as f64 * 100.0;
            e.push(rect(slit, x0, y0, x0 + 20.0, y0 + 30.0));
        }
    }
    write("M1.j.h2", e);
}

// --- M1Fil.a1: min. Metal1:filler width 1.00 ---

fn m1fil_a1_h(l: &L) {
    let f = l.fill;

    // h1 — the bound, long, small and far.  1.0 is legal, 0.995 fires in x and in y; a
    // 300 × 0.995 bar fires once (and is M1Fil.a2 by its length, ignored); a 0.005 sliver;
    // a bar at (1000, 1000).  Ten.
    write(
        "M1Fil.a1.h1",
        vec![
            rect(f, 2.0, 2.0, 3.0, 5.0),
            rect(f, 5.0, 2.0, 5.995, 5.0),
            rect(f, 8.0, 2.0, 11.0, 2.995),
            rect(f, 2.0, 8.0, 302.0, 8.995),
            rect(f, 2.0, 12.0, 2.005, 15.0),
            rect(f, 1000.0, 1000.0, 1000.995, 1003.0),
        ],
    );

    // h3 — unions.  Two overlapping 0.6 boxes making 1.0 are clean, making 0.995 fire
    // once; two abutting slices 0.5 + 0.495 fire once; a 1.0 bar as a 4 × 6 grid is
    // clean; a ring with one 0.995 wall fires once; a 0.995 island in a ring's hole fires
    // once.  Four.
    let mut e = vec![
        rect(f, 2.0, 2.0, 2.6, 5.0),
        rect(f, 2.4, 2.0, 3.0, 5.0),
        rect(f, 5.0, 2.0, 5.6, 5.0),
        rect(f, 5.395, 2.0, 5.995, 5.0),
        rect(f, 8.0, 2.0, 8.5, 5.0),
        rect(f, 8.5, 2.0, 8.995, 5.0),
    ];
    for i in 0..4 {
        for j in 0..6 {
            let (x, y) = (11.0 + 0.25 * i as f64, 2.0 + 0.5 * j as f64);
            e.push(rect(f, x, y, x + 0.25, y + 0.5));
        }
    }
    e.extend(l.ring(f, 2.0, 7.0, 6.0, 11.0, 2.995, 8.0, 5.0, 10.0));
    e.extend(l.ring(f, 8.0, 7.0, 12.0, 11.0, 9.0, 8.0, 11.0, 10.0));
    e.push(rect(f, 9.5, 8.5, 10.495, 9.5));
    write("M1Fil.a1.h3", e);

    // h7 — a comb with three 0.995 teeth (six) and a U with 1.0 arms (clean); the slots
    // are 0.5 wide.
    write(
        "M1Fil.a1.h7",
        vec![
            poly(
                f,
                &[
                    (2.0, 2.0),
                    (5.985, 2.0),
                    (5.985, 5.0),
                    (4.99, 5.0),
                    (4.99, 3.0),
                    (4.49, 3.0),
                    (4.49, 5.0),
                    (3.495, 5.0),
                    (3.495, 3.0),
                    (2.995, 3.0),
                    (2.995, 5.0),
                    (2.0, 5.0),
                ],
            ),
            poly(
                f,
                &[
                    (8.0, 2.0),
                    (11.0, 2.0),
                    (11.0, 5.0),
                    (10.0, 5.0),
                    (10.0, 3.0),
                    (9.0, 3.0),
                    (9.0, 5.0),
                    (8.0, 5.0),
                ],
            ),
        ],
    );
}

// --- M1Fil.a2: max. Metal1:filler width 5.00 ---

fn m1fil_a2_h(l: &L) {
    let f = l.fill;

    // Read as the deck's owner settled it for the metal fillers: no two opposite walls
    // of a filler more than 5.00 apart (upstream's bounding box), two markers per
    // oversized dimension.
    // h1 — the bound and unions.  5 × 5 is legal; 5.005 × 5 (2), 5 × 5.005 (2) and 5.005 ×
    // 5.005 (4) fire; two overlapping boxes making 5.005 × 3 fire (2); an L with 2 arms
    // spanning 5.005 fires (2); a ring 5.005 across fires (4, each wall split by the
    // hole); a diamond a = 2.6 (3.68 between its walls, bounding box 5.2) is clean by the
    // manual's width.  Sixteen.
    let mut e = vec![
        rect(f, 2.0, 2.0, 7.0, 7.0),
        rect(f, 9.0, 2.0, 14.005, 7.0),
        rect(f, 16.0, 2.0, 21.0, 7.005),
        rect(f, 23.0, 2.0, 28.005, 7.005),
        rect(f, 2.0, 9.0, 5.0, 12.0),
        rect(f, 4.0, 9.0, 7.005, 12.0),
        poly(
            f,
            &[
                (9.0, 9.0),
                (14.005, 9.0),
                (14.005, 11.0),
                (11.0, 11.0),
                (11.0, 14.0),
                (9.0, 14.0),
            ],
        ),
    ];
    e.extend(l.ring(f, 16.0, 9.0, 21.005, 14.0, 17.5, 10.5, 19.5, 12.5));
    e.push(diamond(f, 26.0, 12.0, 2.6));
    write("M1Fil.a2.h1", e);
}

// --- M1Fil.b: min. Metal1:filler space 0.42 ---

fn m1fil_b_h(l: &L) {
    let f = l.fill;

    // h3 — notches.  A U with a 0.415 slot is a notch and M1Fil.b says "space" (M1.b says
    // "space or notch"): clean by the wording; two Ls facing across 0.415 fire; an island
    // 0.415 from a ring's inner wall fires.  Two.
    let mut e = vec![
        poly(
            f,
            &[
                (2.0, 2.0),
                (4.415, 2.0),
                (4.415, 5.0),
                (3.415, 5.0),
                (3.415, 3.0),
                (3.0, 3.0),
                (3.0, 5.0),
                (2.0, 5.0),
            ],
        ),
        poly(
            f,
            &[
                (7.0, 2.0),
                (10.0, 2.0),
                (10.0, 3.0),
                (8.0, 3.0),
                (8.0, 5.0),
                (7.0, 5.0),
            ],
        ),
        poly(
            f,
            &[
                (8.5, 3.415),
                (11.0, 3.415),
                (11.0, 6.0),
                (10.0, 6.0),
                (10.0, 4.415),
                (8.5, 4.415),
            ],
        ),
    ];
    e.extend(l.ring(f, 13.0, 2.0, 18.0, 7.0, 14.0, 3.0, 17.0, 6.0));
    e.push(rect(f, 14.5, 3.415, 16.5, 4.415));
    write("M1Fil.b.h3", e);

    // h4 — unions.  Overlapping, abutting and gridded boxes each 0.415 from a third:
    // one each.
    let mut e = vec![
        rect(f, 2.0, 2.0, 3.2, 4.0),
        rect(f, 2.8, 2.0, 4.0, 4.0),
        rect(f, 4.415, 2.0, 6.415, 4.0),
        rect(f, 8.0, 2.0, 9.0, 4.0),
        rect(f, 9.0, 2.0, 10.0, 4.0),
        rect(f, 10.415, 2.0, 12.415, 4.0),
    ];
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (14.0 + 0.5 * i as f64, 2.0 + 0.5 * j as f64);
            e.push(rect(f, x, y, x + 0.5, y + 0.5));
        }
    }
    e.push(rect(f, 16.415, 2.0, 18.415, 4.0));
    write("M1Fil.b.h4", e);

    // h8 — small, long, far.  A 0.005 sliver 0.415 from a filler (the sliver is M1Fil.a1),
    // two 300 µm bars 0.415 apart (once; both are M1Fil.a2 by their length), a pair at
    // (1000, 1000).  Three.
    write(
        "M1Fil.b.h8",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(f, 4.415, 2.0, 4.42, 4.0),
            rect(f, 2.0, 8.0, 302.0, 10.0),
            rect(f, 2.0, 10.415, 302.0, 12.415),
            rect(f, 1000.0, 1000.0, 1002.0, 1002.0),
            rect(f, 1002.415, 1000.0, 1004.415, 1002.0),
        ],
    );

    // h9 — other layers.  A filler 0.415 from Metal1 is M1Fil.c's; from Metal1:mask
    // nobody's; two fillers that overlap or abut are one filler.  M1Fil.c once.
    write(
        "M1Fil.b.h9",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(l.m1, 4.415, 2.0, 6.415, 4.0),
            rect(f, 8.0, 2.0, 10.0, 4.0),
            rect(l.mask, 10.415, 2.0, 12.415, 4.0),
            rect(f, 14.0, 2.0, 16.0, 4.0),
            rect(f, 15.5, 2.0, 17.5, 4.0),
            rect(f, 17.5, 2.0, 19.5, 4.0),
        ],
    );
}

// --- M1Fil.c: min. Metal1:filler space to Metal1 0.42 ---

fn m1fil_c_h(l: &L) {
    let f = l.fill;
    let m = l.m1;

    // h2 — 45°.  A Metal1 diamond tip 0.415 above a filler; a filler chamfer 0.4136 from a
    // Metal1 corner; a Metal1 45° strip parallel to a filler strip at 0.4136; a Metal1
    // strip's tip 0.415 from a filler wall.  Four.
    write(
        "M1Fil.c.h2",
        vec![
            rect(f, 2.0, 2.0, 6.0, 4.0),
            diamond(m, 4.0, 4.915, 0.5),
            chamfered_tr(f, 8.0, 2.0, 11.0, 5.0, 14.5),
            rect(m, 10.585, 4.5, 12.0, 6.0),
            strip45(f, 14.0, 2.0, 2.0, 0.71),
            strip45(m, 14.0, 4.005, 2.0, 0.4),
            strip45(m, 20.0, 2.0, 1.0, 0.4),
            rect(f, 21.415, 2.0, 23.415, 4.0),
        ],
    );

    // h3 — no distance at all.  A filler abutting Metal1 along an edge is 0.00 from it
    // and fires; a filler touching Metal1 at one corner point fires; a filler crossing the
    // Metal1 edge and a filler wholly inside Metal1 share area with it and are no pair
    // (settled: neither tool reports them).  Two.
    write(
        "M1Fil.c.h3",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(m, 4.0, 2.5, 5.0, 3.5),
            rect(f, 8.0, 2.0, 10.0, 4.0),
            rect(m, 10.0, 4.0, 11.0, 5.0),
            rect(f, 14.0, 2.0, 16.0, 4.0),
            rect(m, 15.0, 2.5, 17.0, 3.5),
            rect(f, 20.0, 2.0, 22.0, 4.0),
            rect(m, 19.0, 1.0, 23.0, 5.0),
        ],
    );

    // h7 — small, long, far.  A 0.005 Metal1 sliver 0.415 from a filler (M1.a, M1.d), a
    // 300 µm filler bar 0.415 from a 300 µm Metal1 bar (once; the filler is M1Fil.a2 by
    // its length), a pair at (1000, 1000).  Three.
    write(
        "M1Fil.c.h7",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(m, 4.415, 2.0, 4.42, 4.0),
            rect(f, 2.0, 8.0, 302.0, 10.0),
            rect(m, 2.0, 10.415, 302.0, 11.0),
            rect(f, 1000.0, 1000.0, 1002.0, 1002.0),
            rect(m, 1002.415, 1000.0, 1004.0, 1002.0),
        ],
    );
}

// --- M1Fil.d: min. Metal1:filler space to TRANS 1.00 ---

fn m1fil_d_h(l: &L) {
    let f = l.fill;
    let t = l.trans;

    // h1 — the bound and both metrics.  0.995 fires, 1.0 is clean; a diagonal 0.70/0.70
    // (0.99) fires, 0.71/0.71 (1.004) is clean; corner-on 0.995 fires; a TRANS diamond tip
    // 0.995 above a filler fires; a filler abutting a TRANS is 0.00 from it and fires; a
    // filler crossing the TRANS edge and one wholly inside it are no pair (settled).
    // Five.
    write(
        "M1Fil.d.h1",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(t, 4.995, 2.0, 6.995, 4.0),
            rect(f, 9.0, 2.0, 11.0, 4.0),
            rect(t, 12.0, 2.0, 14.0, 4.0),
            rect(f, 16.0, 2.0, 18.0, 4.0),
            rect(t, 18.7, 4.7, 20.7, 6.7),
            rect(f, 23.0, 2.0, 25.0, 4.0),
            rect(t, 25.71, 4.71, 27.71, 6.71),
            rect(f, 2.0, 9.0, 4.0, 11.0),
            rect(t, 4.995, 11.0, 6.995, 13.0),
            rect(f, 9.0, 9.0, 11.0, 11.0),
            diamond(t, 10.0, 12.995, 1.0),
            rect(f, 16.0, 9.0, 18.0, 11.0),
            rect(t, 18.0, 9.5, 20.0, 11.5),
            rect(f, 23.0, 9.0, 25.0, 11.0),
            rect(t, 24.0, 9.5, 27.0, 11.5),
            rect(f, 30.0, 9.0, 32.0, 11.0),
            rect(t, 29.0, 8.0, 33.0, 12.0),
        ],
    );

    // h2 — tile lines and far.  0.995 gaps ending on x = 20, straddling 20, starting on
    // 20, straddling 21, on 40, straddling 42, at 10; two in y across 20/21 and 40/42; a
    // pair at (1000, 1000); a 300 µm TRANS 0.995 from a 300 µm filler (once).  Eleven.
    let pair = |x: f64, y: f64| {
        vec![
            rect(f, x - 2.0, y, x, y + 2.0),
            rect(t, x + 0.995, y, x + 2.995, y + 2.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(9.005, 2.0));
    e.extend(pair(19.005, 2.0));
    e.extend(pair(19.5, 5.0));
    e.extend(pair(20.0, 8.0));
    e.extend(pair(20.5, 11.0));
    e.extend(pair(39.005, 2.0));
    e.extend(pair(41.5, 5.0));
    e.extend(pair(1000.0, 1000.0));
    e.push(rect(f, 15.0, 14.0, 25.0, 16.0));
    e.push(rect(t, 15.0, 16.995, 25.0, 18.995));
    e.push(rect(f, 35.0, 14.0, 45.0, 16.0));
    e.push(rect(t, 35.0, 16.995, 45.0, 18.995));
    e.push(rect(f, 2.0, 22.0, 302.0, 24.0));
    e.push(rect(t, 2.0, 24.995, 302.0, 26.995));
    write("M1Fil.d.h2", e);

    // h5 — the other fillers.  A Metal5:filler 0.995 from a TRANS, with no Metal1:filler
    // anywhere: nothing for the metal1 deck (M5Fil.d is the metal5 deck's).
    write(
        "M1Fil.d.h5",
        vec![
            rect(l.m5fill, 2.0, 2.0, 4.0, 4.0),
            rect(t, 4.995, 2.0, 6.995, 4.0),
        ],
    );
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the Metal(n=2-5) decks: section 5.17 (Mn.a-Mn.k) and section
// 5.18 (MnFil.*) of the SG13G2 layout rules, one rule set on four layers.  Every
// layout is `tests/data/ihp-sg13g2/metaln/M<rule>.h<k>.gds.gz` and carries the same
// geometry on Metal2, Metal3, Metal4 and Metal5 at the same place (Via(n-1) and
// Metal(n-1) below): a Metal(n) deck reads its own layers of it and nothing else, so
// one file serves the four decks, and the layers can be read against one another.

/// The four layers' elements of each layout, gathered by name.
type Gathered =
    std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, Vec<GdsElement>>>>;

/// The layers of one Metal(n) deck.
struct Ln {
    n: i32,
    /// The layouts gathered across the four layers, written once by `flush`.
    out: Gathered,
    /// Metal(n).
    m: (i16, i16),
    /// Via(n-1), the via the deck's Mn.c/c1 enclose.
    vb: (i16, i16),
    /// Metal(n-1), for the same-net strap under two Metal(n) lines.
    mb: (i16, i16),
    fil: (i16, i16),
    mask: (i16, i16),
    trans: (i16, i16),
    seal: (i16, i16),
    bnd: (i16, i16),
}

const METALN_DIR: &str = "tests/data/ihp-sg13g2/metaln";

impl Ln {
    fn new(pdk: &PdkConfig, n: i32, out: Gathered) -> Self {
        Ln {
            n,
            out,
            m: layer(pdk, &format!("Metal{n}")),
            vb: layer(pdk, &format!("Via{}", n - 1)),
            mb: layer(pdk, &format!("Metal{}", n - 1)),
            fil: layer(pdk, &format!("Metal{n}.filler")),
            mask: layer(pdk, &format!("Metal{n}.mask")),
            trans: layer(pdk, "TRANS"),
            seal: layer(pdk, "EdgeSeal"),
            bnd: layer(pdk, "EdgeSeal.boundary"),
        }
    }

    /// Gathers this layer's elements of `M<rule>.h<k>`, e.g. `write(".a.h1", ..)` into
    /// `M.a.h1.gds.gz`.
    fn write(&self, rule: &str, mut elems: Vec<GdsElement>) {
        self.out
            .borrow_mut()
            .entry(format!("M{rule}"))
            .or_default()
            .append(&mut elems);
    }

    /// Writes every gathered layout.
    fn flush(out: Gathered) {
        std::fs::create_dir_all(METALN_DIR).expect("failed to create output directory");
        for (name, v) in out.borrow_mut().iter_mut() {
            write_gz(
                &format!("{METALN_DIR}/{name}.gds.gz"),
                library("TOP", std::mem::take(v)),
            );
        }
    }

    /// A rectangular frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, drawn
    /// as four abutting boxes that merge into one ring.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        l: (i16, i16),
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
            rect(l, x0, y0, x1, hy0),
            rect(l, x0, hy1, x1, y1),
            rect(l, x0, hy0, hx0, hy1),
            rect(l, hx1, hy0, x1, hy1),
        ]
    }

    /// A 0.19 µm Via(n-1) (Vn.a is exact) with its lower-left corner at `(x, y)`.
    fn via(&self, x: f64, y: f64) -> GdsElement {
        rect(self.vb, x, y, x + VIA, y + VIA)
    }
}

/// Via side, Vn.a.
const VIA: f64 = 0.19;

/// 45° strip running up-left from `(x0, y0)`: the mirror of `helpers::strip45`.  Its
/// lower-left wall lies on `x + y = x0 + y0`, the body on the far side of it; the
/// perpendicular width is `d·√2`.  A box corner at `(cx, cy)` with `cx + cy < x0 + y0`
/// is `(x0 + y0 − cx − cy)/√2` from the wall.
fn strip135(l: (i16, i16), x0: f64, y0: f64, len: f64, d: f64) -> GdsElement {
    poly(
        l,
        &[
            (x0, y0),
            (x0 - len, y0 + len),
            (x0 - len + d, y0 + len + d),
            (x0 + d, y0 + d),
        ],
    )
}

/// A route of width `w` that runs right along `y ∈ [y0, y0+w]`, jogs up-right at 45° by
/// `h` and runs right again.  The jog's two walls are `h·√2` long each; the upper wall is
/// offset `off` in x from the lower one, so the jog is `off/√2` wide (0.285 → 0.2015,
/// 0.34 → 0.2404: the grid does not allow `w·√2` exactly).
#[allow(clippy::too_many_arguments)]
fn jog(l: (i16, i16), x0: f64, y0: f64, w: f64, off: f64, a: f64, h: f64, run: f64) -> GdsElement {
    poly(
        l,
        &[
            (x0, y0),
            (x0 + a, y0),
            (x0 + a + h, y0 + h),
            (x0 + a + h + run, y0 + h),
            (x0 + a + h + run, y0 + h + w),
            (x0 + a + h - off + w, y0 + h + w),
            (x0 + a - off + w, y0 + w),
            (x0, y0 + w),
        ],
    )
}

/// An Ln route 0.2 wide from `(x0, y0)` with its bend chamfered at 45°: the outer chamfer
/// cuts `k` off the corner, the inner one runs parallel 0.285 in (0.2015 wide).  Outer
/// wall `k·√2` long, inner `(k − 0.115)·√2`.
fn chamfered_ln(l: (i16, i16), x0: f64, y0: f64, k: f64) -> GdsElement {
    let (x, y) = (x0 + 3.0, y0 + 3.0);
    poly(
        l,
        &[
            (x0, y0),
            (x - k, y0),
            (x, y0 + k),
            (x, y),
            (x - 0.2, y),
            (x - 0.2, y0 + k + 0.085),
            (x - k - 0.085, y0 + 0.2),
            (x0, y0 + 0.2),
        ],
    )
}

fn hardening_metaln(pdk: &PdkConfig) {
    let out: Gathered = Default::default();
    for n in 2..6 {
        let l = Ln::new(pdk, n, out.clone());
        mn_a(&l);
        mn_b(&l);
        mn_c(&l);
        mn_c1(&l);
        mn_d(&l);
        mn_e(&l);
        mn_f(&l);
        mn_g(&l);
        mn_i(&l);
        mn_density(&l);
        mnfil(&l);
    }
    Ln::flush(out);
}

// --- Mn.a: min. Metal(n) width 0.20 ---

fn mn_a(l: &Ln) {
    let m = l.m;

    // h1 — the bound.  0.20 is legal, 0.195 (one grid step under) is not, in x and in y;
    // a 300 µm bar crossing every tile line counts once.
    l.write(
        ".a.h1",
        vec![
            rect(m, 2.0, 2.0, 2.2, 4.0),       // clean
            rect(m, 5.0, 2.0, 5.195, 4.0),     // Mn.a (x)
            rect(m, 8.0, 2.0, 10.0, 2.195),    // Mn.a (y)
            rect(m, 2.0, 8.0, 302.0, 8.2),     // clean, 300 µm
            rect(m, 2.0, 12.0, 302.0, 12.195), // Mn.a, 300 µm → one violation
        ],
    );

    // h2 — 45° geometry.  A diamond's width is the distance between opposite walls
    // (a·√2): 0.145 → 0.205 clean, 0.14 → 0.198 fires; the same for a 45° strip.  The
    // strips are longer than 0.5 and under 0.24 wide, so Mn.g reads them as well; the
    // case sets Mn.g aside.  A chamfered box and an Ln with a chamfered inner corner are
    // wide everywhere.
    l.write(
        ".a.h2",
        vec![
            diamond(m, 3.0, 3.0, 0.145),                  // clean (0.205)
            diamond(m, 7.0, 3.0, 0.14),                   // Mn.a (0.198)
            strip45(m, 10.0, 2.0, 3.0, 0.145),            // clean
            strip45(m, 15.0, 2.0, 3.0, 0.14),             // Mn.a
            chamfered_tr(m, 10.0, 8.0, 12.0, 10.0, 21.5), // clean
            poly(
                m,
                &[
                    (14.0, 8.0),
                    (17.0, 8.0),
                    (17.0, 9.0),
                    (15.5, 9.0),
                    (15.0, 9.5),
                    (15.0, 11.0),
                    (14.0, 11.0),
                ],
            ), // clean: Ln, inner corner chamfered, arms 1.0 wide
        ],
    );

    // h3 — shapes that merge.  Two overlapping 0.12 boxes whose union is 0.20 wide are
    // clean, 0.195 fires once; four abutting 0.05 slices make 0.20, three plus a 0.045
    // make 0.195; a 0.20 × 2 bar as a 4 × 10 grid of boxes is clean; a ring with one 0.195
    // side fires once; an island in a ring's hole is a shape of its own (0.195 → fires).
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.12, 4.0),
        rect(m, 2.08, 2.0, 2.2, 4.0), // union 0.20 → clean
        rect(m, 5.0, 2.0, 5.12, 4.0),
        rect(m, 5.075, 2.0, 5.195, 4.0), // union 0.195 → Mn.a
    ];
    for i in 0..4 {
        let x = 8.0 + 0.05 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.05, 4.0)); // 0.20 → clean
    }
    for i in 0..3 {
        let x = 11.0 + 0.05 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.05, 4.0));
    }
    e.push(rect(m, 11.15, 2.0, 11.195, 4.0)); // 0.195 → Mn.a
    for i in 0..4 {
        for j in 0..10 {
            let (x, y) = (14.0 + 0.05 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(m, x, y, x + 0.05, y + 0.2)); // 0.20 × 2 grid → clean
        }
    }
    e.extend(l.ring(m, 2.0, 7.0, 6.0, 11.0, 2.195, 8.0, 5.0, 10.0)); // left side 0.195
    e.extend(l.ring(m, 8.0, 7.0, 12.0, 11.0, 8.8, 7.8, 11.2, 10.2));
    e.push(rect(m, 9.6, 8.6, 10.4, 9.4)); // island 0.8 wide → clean
    e.extend(l.ring(m, 14.0, 7.0, 18.0, 11.0, 14.8, 7.8, 17.2, 10.2));
    e.push(rect(m, 15.6, 8.6, 15.795, 9.4)); // island 0.195 → Mn.a
    l.write(".a.h3", e);

    // h4 — tile lines.  0.195 bars ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, inside a tile at 10; 0.195-tall bars
    // running across 20/21 and 40/42; an Ln cornered on x = 20 with its vertical arm
    // narrow.  Ten violations whatever the tile; the 0.20 controls are clean.
    l.write(
        ".a.h4",
        vec![
            rect(m, 9.805, 2.0, 10.0, 4.0),
            rect(m, 19.805, 2.0, 20.0, 4.0),
            rect(m, 19.9, 6.0, 20.095, 8.0),
            rect(m, 20.0, 10.0, 20.195, 12.0),
            rect(m, 20.9, 14.0, 21.095, 16.0),
            rect(m, 39.805, 2.0, 40.0, 4.0),
            rect(m, 41.9, 6.0, 42.095, 8.0),
            rect(m, 15.0, 18.0, 25.0, 18.195),
            rect(m, 35.0, 18.0, 45.0, 18.195),
            poly(
                m,
                &[
                    (20.0, 22.0),
                    (23.0, 22.0),
                    (23.0, 23.0),
                    (20.195, 23.0),
                    (20.195, 26.0),
                    (20.0, 26.0),
                ],
            ),
            rect(m, 19.9, 28.0, 20.1, 30.0), // 0.20 straddling 20 → clean
            rect(m, 15.0, 32.0, 25.0, 32.2), // 0.20 tall across 20 → clean
        ],
    );

    // h7 — a 0.005 sliver (one grid step, also under Mn.d) and a bar at (1000, 1000).
    l.write(
        ".a.h7",
        vec![
            rect(m, 2.0, 2.0, 2.005, 4.0),
            rect(m, 1000.0, 1000.0, 1000.195, 1002.0),
        ],
    );

    // h8 — a comb with three 0.195 teeth (three violations of one polygon) and a U whose
    // 0.20 arms are clean.
    l.write(
        ".a.h8",
        vec![
            poly(
                m,
                &[
                    (2.0, 2.0),
                    (4.195, 2.0),
                    (4.195, 5.0),
                    (4.0, 5.0),
                    (4.0, 3.0),
                    (3.195, 3.0),
                    (3.195, 5.0),
                    (3.0, 5.0),
                    (3.0, 3.0),
                    (2.195, 3.0),
                    (2.195, 5.0),
                    (2.0, 5.0),
                ],
            ),
            poly(
                m,
                &[
                    (7.0, 2.0),
                    (9.0, 2.0),
                    (9.0, 5.0),
                    (8.8, 5.0),
                    (8.8, 3.0),
                    (7.2, 3.0),
                    (7.2, 5.0),
                    (7.0, 5.0),
                ],
            ),
        ],
    );

    // h9 — the sealring.  Section 6.10: "standard metal and via rules are not checked
    // within EdgeSeal regions".  A 0.195 bar and a 0.205 pair wholly under an EdgeSeal
    // are exempt; the same outside it fire (Mn.a and Mn.b).
    l.write(
        ".a.h9",
        vec![
            rect(l.seal, 1.0, 1.0, 5.0, 5.0),
            rect(m, 2.0, 2.0, 2.195, 3.0), // exempt
            rect(m, 2.0, 3.5, 3.0, 4.0),   // exempt pair, 0.205 apart
            rect(m, 3.205, 3.5, 4.0, 4.0),
            rect(m, 8.0, 2.0, 8.195, 3.0), // Mn.a
            rect(m, 8.0, 3.5, 9.0, 4.0),   // Mn.b
            rect(m, 9.205, 3.5, 10.0, 4.0),
        ],
    );
}

// --- Mn.b: min. Metal(n) space or notch 0.21 ---

fn mn_b(l: &Ln) {
    let m = l.m;

    // h1 — the bound and both metrics.  1 × 1 boxes 0.21 apart are clean, 0.205 fire (x
    // and y); corner to corner dx = dy = 0.145 is 0.205 euclidian (fires), 0.15 is 0.212
    // (clean); dx = dy = 0.2 is under 0.21 on either axis but 0.283 euclidian (clean); a
    // corner 0.205 off a wall with its projection half over it fires.
    l.write(
        ".b.h1",
        vec![
            rect(m, 2.0, 2.0, 3.0, 3.0),
            rect(m, 3.21, 2.0, 4.21, 3.0),   // 0.21 → clean
            rect(m, 2.0, 3.205, 3.0, 4.205), // 0.205 (y) → Mn.b
            rect(m, 6.0, 2.0, 7.0, 3.0),
            rect(m, 7.205, 2.0, 8.205, 3.0), // 0.205 (x) → Mn.b
            rect(m, 10.0, 2.0, 11.0, 3.0),
            rect(m, 11.145, 3.145, 12.145, 4.145), // diagonal 0.205 → Mn.b
            rect(m, 14.0, 2.0, 15.0, 3.0),
            rect(m, 15.15, 3.15, 16.15, 4.15), // diagonal 0.212 → clean
            rect(m, 17.0, 2.0, 18.0, 3.0),
            rect(m, 18.2, 3.2, 19.2, 4.2), // 0.2/0.2 axes, 0.283 euclidian → clean
            rect(m, 2.0, 6.0, 3.0, 7.0),
            rect(m, 3.205, 6.5, 4.205, 7.5), // corner-on 0.205 → Mn.b
        ],
    );

    // h2 — 45° geometry.  A diamond tip 0.205 above a wall (Mn.b, and Mn.i since the tip's
    // edges are 45°), 0.21 (Mn.i only), 0.24 (clean); two 0.354-wide 45° strips 0.205 apart (both) and
    // 0.212 apart (Mn.i only); a chamfer passing 0.205 from a box corner (both); two
    // diamond tips 0.205 apart (both).
    let e = vec![
        rect(m, 2.0, 2.0, 4.0, 3.0),
        diamond(m, 3.0, 3.705, 0.5), // tip 0.205 → Mn.b + Mn.i
        rect(m, 6.0, 2.0, 8.0, 3.0),
        diamond(m, 7.0, 3.71, 0.5), // tip 0.21 → Mn.i
        rect(m, 10.0, 2.0, 12.0, 3.0),
        diamond(m, 11.0, 3.74, 0.5), // tip 0.24 → clean
        strip45(m, 14.0, 2.0, 2.0, 0.25),
        strip45(m, 14.0, 2.79, 2.0, 0.25), // gap 0.205 → Mn.b + Mn.i
        strip45(m, 19.0, 2.0, 2.0, 0.25),
        strip45(m, 19.0, 2.8, 2.0, 0.25), // gap 0.212 → Mn.i
        chamfered_tr(m, 2.0, 8.0, 4.0, 10.0, 13.9),
        rect(m, 4.095, 10.095, 5.095, 11.095), // corner 0.205 from the chamfer → both
        diamond(m, 8.0, 9.0, 0.5),
        diamond(m, 9.205, 9.0, 0.5), // tips 0.205 apart → both
    ];
    l.write(".b.h2", e);

    // h3 — notches.  A straight U notch (0.205 fires, 0.21 clean, both orientations); a
    // straight wall facing a 45° wall of the same shape (0.205 fires, 0.21 clean; the 45°
    // wall draws Mn.i on all four); a comb with three 0.205 slots; a 0.205 slot cut into a
    // plate; a keyhole-drawn ring with a 0.205 hole; two Ls facing across 0.205; an island
    // 0.205 from a ring's inner wall.
    let mut e = notch_pattern(m, 0.25, 0.21, 1.0, 2.0, -0.005);
    e.extend(mixed_notch_pattern(m, 0.25, 0.21, 0.71, 1.0, 12.0, -0.005));
    e.push(poly(
        m,
        &[
            (24.0, 2.0),
            (25.815, 2.0),
            (25.815, 5.0),
            (25.515, 5.0),
            (25.515, 3.0),
            (25.31, 3.0),
            (25.31, 5.0),
            (25.01, 5.0),
            (25.01, 3.0),
            (24.805, 3.0),
            (24.805, 5.0),
            (24.505, 5.0),
            (24.505, 3.0),
            (24.3, 3.0),
            (24.3, 5.0),
            (24.0, 5.0),
        ],
    ));
    e.push(poly(
        m,
        &[
            (28.0, 2.0),
            (31.0, 2.0),
            (31.0, 5.0),
            (29.805, 5.0),
            (29.805, 3.5),
            (29.6, 3.5),
            (29.6, 5.0),
            (28.0, 5.0),
        ],
    ));
    e.push(poly(
        m,
        &[
            (2.0, 7.0),
            (5.0, 7.0),
            (5.0, 10.0),
            (3.5, 10.0),
            (3.5, 9.5),
            (3.605, 9.5),
            (3.605, 8.0),
            (3.4, 8.0),
            (3.4, 9.5),
            (3.5, 9.5),
            (3.5, 10.0),
            (2.0, 10.0),
        ],
    ));
    e.push(poly(
        m,
        &[
            (7.0, 7.0),
            (10.0, 7.0),
            (10.0, 8.0),
            (8.0, 8.0),
            (8.0, 10.0),
            (7.0, 10.0),
        ],
    ));
    e.push(poly(
        m,
        &[
            (8.205, 8.205),
            (11.0, 8.205),
            (11.0, 11.0),
            (10.0, 11.0),
            (10.0, 9.205),
            (8.205, 9.205),
        ],
    ));
    e.extend(l.ring(m, 13.0, 7.0, 17.0, 11.0, 13.8, 7.8, 16.2, 10.2));
    e.push(rect(m, 14.005, 8.5, 15.0, 9.5));
    l.write(".b.h3", e);

    // h4 — unions.  Two overlapping boxes whose union's wall faces a third box at 0.205
    // (once); a box drawn as a 5 × 5 grid facing another at 0.205 (once).
    let mut e = vec![
        rect(m, 2.0, 2.0, 3.0, 3.0),
        rect(m, 2.5, 2.5, 3.5, 3.5),
        rect(m, 3.705, 2.0, 4.705, 4.0),
        rect(m, 7.205, 2.0, 8.205, 3.0),
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (6.0 + 0.2 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(m, x, y, x + 0.2, y + 0.2));
        }
    }
    l.write(".b.h4", e);

    // h5 — tile lines.  0.205 gaps straddling x = 20, with a wall on 20, straddling 21, on
    // 40, straddling 42, well inside a tile; a horizontal 0.205 gap running across 20; two
    // boxes corner to corner on x = 20, 0.205 apart in y.  Eight violations.
    l.write(
        ".b.h5",
        vec![
            rect(m, 19.0, 2.0, 19.9, 3.0),
            rect(m, 20.105, 2.0, 21.105, 3.0),
            rect(m, 19.0, 5.0, 20.0, 6.0),
            rect(m, 20.205, 5.0, 21.205, 6.0),
            rect(m, 20.0, 8.0, 20.9, 9.0),
            rect(m, 21.105, 8.0, 22.105, 9.0),
            rect(m, 39.0, 2.0, 40.0, 3.0),
            rect(m, 40.205, 2.0, 41.205, 3.0),
            rect(m, 41.0, 5.0, 41.9, 6.0),
            rect(m, 42.105, 5.0, 43.105, 6.0),
            rect(m, 9.0, 2.0, 9.9, 3.0),
            rect(m, 10.105, 2.0, 11.105, 3.0),
            rect(m, 15.0, 12.0, 25.0, 13.0),
            rect(m, 15.0, 13.205, 25.0, 14.205),
            rect(m, 18.0, 16.0, 20.0, 17.0),
            rect(m, 20.0, 17.205, 21.0, 18.205),
        ],
    );

    // h8 — a 0.005 sliver 0.205 from a box (the sliver is under Mn.a and Mn.d, set
    // aside); 300 µm bars 0.205 apart (one violation); a pair at (1000, 1000).
    l.write(
        ".b.h8",
        vec![
            rect(m, 1.0, 2.0, 2.0, 4.0),
            rect(m, 2.205, 2.0, 2.21, 4.0),
            rect(m, 2.0, 8.0, 302.0, 9.0),
            rect(m, 2.0, 9.205, 302.0, 10.205),
            rect(m, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(m, 1001.205, 1000.0, 1002.205, 1001.0),
        ],
    );

    // h9 — nets.  Mn.b has no net condition: two lines 0.205 apart fire whether joined
    // through Via(n-1) and a Metal(n-1) strap (left) or not (right).  Each layer's pair
    // stands 20 µm further right, so the strap under one layer's pair - Metal(n-1), the
    // layer below's own metal - lands where that layer draws nothing.
    let dx = 20.0 * (l.n - 2) as f64;
    let e = vec![
        rect(m, dx + 2.0, 2.0, dx + 3.0, 4.0),
        rect(m, dx + 3.205, 2.0, dx + 4.205, 4.0),
        l.via(dx + 2.405, 2.405),
        l.via(dx + 3.605, 2.405),
        strap(l.mb, &[(dx + 2.5, 2.5), (dx + 3.7, 2.5)]),
        rect(m, dx + 8.0, 2.0, dx + 9.0, 4.0),
        rect(m, dx + 9.205, 2.0, dx + 10.205, 4.0),
    ];
    l.write(".b.h9", e);
}

// --- Mn.c: min. Metal(n) enclosure of Via(n-1) 0.005 ---

fn mn_c(l: &Ln) {
    let m = l.m;

    // h1 — the bound.  A via in a 0.2-wide line with 0.005 above and below is clean; a
    // via whose bottom edge lies on the line's edge (0.000) fires; one sticking 0.005 out
    // fires; 0.005 either side in a vertical line is clean; 0.005 on the left of a pad is
    // clean; a via in a pad's corner, on two edges, fires.
    l.write(
        ".c.h1",
        vec![
            rect(m, 2.0, 2.0, 3.0, 2.2),
            l.via(2.4, 2.005), // clean
            rect(m, 5.0, 2.0, 6.0, 2.2),
            l.via(5.4, 2.0), // bottom 0.000 → Mn.c
            rect(m, 8.0, 2.0, 9.0, 2.2),
            l.via(8.4, 1.995), // sticks 0.005 out → Mn.c
            rect(m, 11.0, 2.0, 11.2, 3.0),
            l.via(11.005, 2.4), // clean
            rect(m, 14.0, 2.0, 14.6, 2.6),
            l.via(14.005, 2.2), // clean
            rect(m, 17.0, 2.0, 17.6, 2.6),
            l.via(17.0, 2.0), // corner, 0.000 on two sides → Mn.c
        ],
    );

    // h2 — no cover.  A bare via with no Metal(n) anywhere, a via half out of a line's
    // end (also a negative endcap, Mn.c1), and a via 0.1 beside a line: none is enclosed
    // by 0.005 of Metal(n).
    l.write(
        ".c.h2",
        vec![
            l.via(2.0, 2.0),
            rect(m, 5.0, 2.0, 6.0, 2.2),
            l.via(5.9, 2.005),
            rect(m, 8.0, 2.0, 9.0, 2.2),
            l.via(9.1, 2.005),
        ],
    );

    // h3 — a chamfer at the via's corner.  A 0.6 pad with the via 0.005 from its top and
    // right edges; the pad's corner is chamfered along x + y = k.  k = 5.195 passes
    // 0.0035 from the via's corner with both walls' margins 0.005 (the settled projection
    // reading: clean); 5.19 passes through the via's corner (touch → 0.000); 5.18 cuts it.
    let pad = |x: f64, k: f64| {
        vec![
            chamfered_tr(m, x, 2.0, x + 0.6, 2.6, k),
            l.via(x + 0.405, 2.405),
        ]
    };
    let mut e = pad(2.0, 5.195);
    e.extend(pad(5.0, 8.19));
    e.extend(pad(8.0, 11.18));
    l.write(".c.h3", e);

    // h4 — unions.  Metal(n) from two overlapping boxes encloses the via by 0.005 though
    // each box alone clips it (clean); two boxes whose union has the via on its edge fire
    // once; a line drawn as ten abutting slices is clean.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.5, 2.2),
        rect(m, 2.4, 2.0, 3.0, 2.2),
        l.via(2.4, 2.005),
        rect(m, 5.0, 2.0, 5.5, 2.2),
        rect(m, 5.4, 2.0, 6.0, 2.2),
        l.via(5.4, 2.0),
    ];
    for i in 0..10 {
        let x = 8.0 + 0.1 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.1, 2.2));
    }
    e.push(l.via(8.4, 2.005));
    l.write(".c.h4", e);

    // h5 — tile lines.  Vias with their bottom edge on the line's edge straddling x = 20,
    // 21, 40 and 42; a via whose left edge lies on x = 20 and on the metal edge; a via
    // ending 0.005 short of x = 20 and of its line's end (Mn.c clean, a 0.005 endcap
    // with 0.005 sides for Mn.c1); a via straddling 20 in a 10 µm line (clean).
    l.write(
        ".c.h5",
        vec![
            rect(m, 19.0, 2.0, 22.0, 2.2),
            l.via(19.905, 2.0),
            l.via(20.905, 2.0),
            rect(m, 39.0, 2.0, 43.0, 2.2),
            l.via(39.905, 2.0),
            l.via(41.905, 2.0),
            rect(m, 20.0, 5.0, 20.2, 8.0),
            l.via(20.0, 6.0),
            rect(m, 19.0, 10.0, 20.0, 10.2),
            l.via(19.805, 10.005),
            rect(m, 15.0, 12.0, 25.0, 12.2),
            l.via(19.905, 12.005),
        ],
    );

    // h8 — a via on the edge of a line at (1000, 1000); one on the edge of a 300 µm line.
    l.write(
        ".c.h8",
        vec![
            rect(m, 1000.0, 1000.0, 1001.0, 1000.2),
            l.via(1000.4, 1000.0),
            rect(m, 2.0, 2.0, 302.0, 2.2),
            l.via(150.0, 2.0),
        ],
    );

    // h9 — the sealring.  A via on the line's edge under an EdgeSeal is exempt (section
    // 6.10); the same outside fires; a via across the EdgeSeal edge has its outside part
    // enclosed by 0.000 below → fires.
    l.write(
        ".c.h9",
        vec![
            rect(l.seal, 1.0, 1.0, 4.0, 4.0),
            rect(m, 2.0, 2.0, 3.0, 2.2),
            l.via(2.4, 2.0),
            rect(m, 8.0, 2.0, 9.0, 2.2),
            l.via(8.4, 2.0),
            rect(l.seal, 11.0, 1.0, 12.0, 4.0),
            rect(m, 11.0, 2.0, 13.0, 2.2),
            l.via(11.905, 2.0),
        ],
    );
}

// --- Mn.c1: min. Metal(n) endcap enclosure of Via(n-1) 0.05 ---

fn mn_c1(l: &Ln) {
    let m = l.m;

    // h1 — line ends.  A via at the end of a 0.2-wide line (0.005 either side, the line
    // continuing on the other side): endcap 0.05 clean, 0.045 fires, 0.000 fires (with
    // Mn.c); a via in the middle of a line is clean; the same at the top of a vertical
    // line and at the left end; two vias at a line's end - the outer one's 0.045 fires,
    // the inner one is covered by the outer.
    l.write(
        ".c1.h1",
        vec![
            rect(m, 2.0, 2.0, 4.0, 2.2),
            l.via(3.76, 2.005), // endcap 0.05 → clean
            rect(m, 6.0, 2.0, 8.0, 2.2),
            l.via(7.765, 2.005), // endcap 0.045 → Mn.c1
            rect(m, 10.0, 2.0, 12.0, 2.2),
            l.via(11.81, 2.005), // endcap 0.000 → Mn.c + Mn.c1
            rect(m, 14.0, 2.0, 16.0, 2.2),
            l.via(14.9, 2.005), // middle → clean
            rect(m, 18.0, 2.0, 18.2, 4.0),
            l.via(18.005, 3.765), // top endcap 0.045 → Mn.c1
            rect(m, 2.0, 5.0, 4.0, 5.2),
            l.via(2.045, 5.005), // left endcap 0.045 → Mn.c1
            rect(m, 6.0, 5.0, 8.0, 5.2),
            l.via(7.765, 5.005), // outer of two → Mn.c1
            l.via(7.355, 5.005),
        ],
    );

    // h2 — corners (note 1: "at least one side must be treated as an endcap").  An Ln
    // route with 0.3 arms and the via in its corner square, under the vertical arm and
    // beside the horizontal one, so its left and top sides continue into the arms; the
    // outer margins (right, bottom) are: (0.05, 0.005) clean; (0.045, 0.005) fires;
    // (0.005, 0.005) fires; (0.05, 0.05) clean; (0.005, 0.05) clean.
    let corner = |x: f64, r: f64, b: f64| {
        vec![
            poly(
                m,
                &[
                    (x - 2.0, 2.0),
                    (x, 2.0),
                    (x, 4.0),
                    (x - 0.3, 4.0),
                    (x - 0.3, 2.3),
                    (x - 2.0, 2.3),
                ],
            ),
            l.via(x - r - VIA, 2.0 + b),
        ]
    };
    let mut e = corner(3.5, 0.05, 0.005);
    e.extend(corner(8.0, 0.045, 0.005));
    e.extend(corner(12.5, 0.005, 0.005));
    e.extend(corner(17.0, 0.05, 0.05));
    e.extend(corner(24.0, 0.005, 0.05));
    l.write(".c1.h2", e);

    // h3 — isolated pads: a via with margins (left, right, bottom, top).  These pads are
    // tiny, under Mn.d, which the case sets aside.  (0.05, 0.005, 0.005, 0.005): a short
    // end in every direction → fires; (0.05, 0.05, 0.005, 0.005): a line passing through,
    // sides tight → clean; (0.05, 0.005, 0.05, 0.005): two good sides adjacent, the corner
    // has no endcap → fires; 0.045 all round fires; three good sides clean; four clean;
    // 0.005 all round fires.
    let pad = |x: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        vec![
            rect(m, x - ml, 2.0 - mb, x + VIA + mr, 2.0 + VIA + mt),
            l.via(x, 2.0),
        ]
    };
    let mut e = pad(2.0, 0.05, 0.005, 0.005, 0.005);
    e.extend(pad(4.0, 0.05, 0.05, 0.005, 0.005));
    e.extend(pad(6.0, 0.05, 0.005, 0.05, 0.005));
    e.extend(pad(8.0, 0.045, 0.045, 0.045, 0.045));
    e.extend(pad(10.0, 0.05, 0.05, 0.05, 0.005));
    e.extend(pad(12.0, 0.05, 0.05, 0.05, 0.05));
    e.extend(pad(14.0, 0.005, 0.005, 0.005, 0.005));
    l.write(".c1.h3", e);

    // h4 — wide lines and junctions, all clean.  A via at the end of a 0.5-wide line with
    // 0.155 either side and a 0.045 endcap: the 0.155 sides are an opposite pair over
    // 0.05, which is what note 1 asks for (an endcap in one direction), so the short end
    // is Mn.c's business; the same with 0.05; a via at a T junction (three sides continue,
    // 0.005 below); a via in the middle of a cross.
    l.write(
        ".c1.h4",
        vec![
            rect(m, 2.0, 2.0, 4.0, 2.5),
            l.via(3.765, 2.155), // endcap 0.045, sides 0.155 → clean
            rect(m, 6.0, 2.0, 8.0, 2.5),
            l.via(7.76, 2.155), // endcap 0.05 → clean
            rect(m, 10.0, 2.0, 13.0, 2.2),
            rect(m, 11.4, 2.2, 11.6, 4.0),
            l.via(11.405, 2.005), // T → clean
            rect(m, 15.0, 3.0, 18.0, 3.2),
            rect(m, 16.4, 2.0, 16.6, 5.0),
            l.via(16.405, 3.005), // cross → clean
        ],
    );

    // h5 — tile lines.  Line-end vias with a 0.045 endcap: the end on x = 20, the via
    // straddling 20, the end on 21, on 40, the via straddling 42; an end on 20 with a 0.05
    // endcap (clean).
    l.write(
        ".c1.h5",
        vec![
            rect(m, 18.0, 2.0, 20.0, 2.2),
            l.via(19.765, 2.005),
            rect(m, 18.1, 5.0, 20.1, 5.2),
            l.via(19.865, 5.005),
            rect(m, 19.0, 8.0, 21.0, 8.2),
            l.via(20.765, 8.005),
            rect(m, 38.0, 2.0, 40.0, 2.2),
            l.via(39.765, 2.005),
            rect(m, 40.1, 5.0, 42.1, 5.2),
            l.via(41.865, 5.005),
            rect(m, 18.0, 11.0, 20.0, 11.2),
            l.via(19.76, 11.005),
        ],
    );

    // h8 — a 0.045 endcap at (1000, 1000) and at the far end of a 300 µm line.
    l.write(
        ".c1.h8",
        vec![
            rect(m, 1000.0, 1000.0, 1002.0, 1000.2),
            l.via(1001.765, 1000.005),
            rect(m, 2.0, 2.0, 302.0, 2.2),
            l.via(301.765, 2.005),
        ],
    );
}

// --- Mn.d: min. Metal(n) area 0.144 µm² ---

fn mn_d(l: &Ln) {
    let m = l.m;

    // h1 — the bound.  0.4 × 0.36 = 0.144 clean; 0.4 × 0.355 = 0.142 fires (both ways);
    // an Ln of 0.144 clean and of 0.142 fires; a diamond of 0.1458 clean, 0.1405 fires.
    l.write(
        ".d.h1",
        vec![
            rect(m, 2.0, 2.0, 2.4, 2.36),
            rect(m, 4.0, 2.0, 4.4, 2.355),
            rect(m, 6.0, 2.0, 6.355, 2.4),
            poly(
                m,
                &[
                    (8.0, 2.0),
                    (8.5, 2.0),
                    (8.5, 2.2),
                    (8.2, 2.2),
                    (8.2, 2.42),
                    (8.0, 2.42),
                ],
            ),
            poly(
                m,
                &[
                    (10.0, 2.0),
                    (10.5, 2.0),
                    (10.5, 2.2),
                    (10.2, 2.2),
                    (10.2, 2.41),
                    (10.0, 2.41),
                ],
            ),
            diamond(m, 13.0, 2.5, 0.27),
            diamond(m, 15.0, 2.5, 0.265),
        ],
    );

    // h2 — shapes that merge.  A cross of two 0.4 × 0.2 boxes is 0.12 as a union (0.16 as
    // a sum) → fires; two abutting 0.2 × 0.4 boxes are 0.16 → clean; two 0.3 boxes
    // sharing one corner are two regions of 0.09 → two; a 4 × 4 grid of boxes making
    // 0.4 × 0.36 is clean; a 0.7 ring with 0.2 walls (0.40) is clean; a 0.3 island in a
    // ring's hole fires; a 0.2 × 0.7 bar (0.14) fires, 0.2 × 0.72 (0.144) is clean.
    let mut e = vec![
        rect(m, 2.0, 2.1, 2.4, 2.3),
        rect(m, 2.1, 2.0, 2.3, 2.4),
        rect(m, 4.0, 2.0, 4.2, 2.4),
        rect(m, 4.2, 2.0, 4.4, 2.4),
        rect(m, 6.0, 2.0, 6.3, 2.3),
        rect(m, 6.3, 2.3, 6.6, 2.6),
    ];
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (8.0 + 0.1 * i as f64, 2.0 + 0.09 * j as f64);
            e.push(rect(m, x, y, x + 0.1, y + 0.09));
        }
    }
    e.extend(l.ring(m, 10.0, 2.0, 10.7, 2.7, 10.2, 2.2, 10.5, 2.5));
    e.extend(l.ring(m, 12.0, 2.0, 13.4, 3.4, 12.3, 2.3, 13.1, 3.1));
    e.push(rect(m, 12.55, 2.55, 12.85, 2.85));
    e.push(rect(m, 15.0, 2.0, 15.2, 2.7));
    e.push(rect(m, 16.0, 2.0, 16.2, 2.72));
    l.write(".d.h2", e);

    // h3 — tile lines.  0.2 × 0.7 bars (0.14) straddling x = 20, ending on 20, starting on
    // 20, straddling 21, straddling 40 and 42, inside a tile; a vertical one across
    // y = 20; a 0.72 bar across 20 is clean.
    l.write(
        ".d.h3",
        vec![
            rect(m, 19.65, 2.0, 20.35, 2.2),
            rect(m, 19.3, 4.0, 20.0, 4.2),
            rect(m, 20.0, 6.0, 20.7, 6.2),
            rect(m, 20.65, 8.0, 21.35, 8.2),
            rect(m, 39.65, 2.0, 40.35, 2.2),
            rect(m, 41.65, 4.0, 42.35, 4.2),
            rect(m, 9.65, 2.0, 10.35, 2.2),
            rect(m, 15.0, 19.65, 15.2, 20.35),
            rect(m, 19.64, 10.0, 20.36, 10.2),
        ],
    );

    // h6 — a 0.005 × 2 sliver (0.01, also Mn.a); a 0.01 × 14.4 sliver (0.144, Mn.a only);
    // a 0.14 bar at (1000, 1000); a 300 µm line is clean.
    l.write(
        ".d.h6",
        vec![
            rect(m, 2.0, 2.0, 2.005, 4.0),
            rect(m, 2.0, 6.0, 16.4, 6.01),
            rect(m, 1000.0, 1000.0, 1000.2, 1000.7),
            rect(m, 2.0, 10.0, 302.0, 10.2),
        ],
    );

    // h7 — the sealring: a 0.14 bar under an EdgeSeal is exempt (section 6.10), one
    // outside fires.
    l.write(
        ".d.h7",
        vec![
            rect(l.seal, 1.0, 1.0, 4.0, 4.0),
            rect(m, 2.0, 2.0, 2.2, 2.7),
            rect(m, 8.0, 2.0, 8.2, 2.7),
        ],
    );
}

// --- Mn.e: min. space 0.24 of lines if one is wider than 0.39 and the parallel run is
// more than 1.0 ---

fn mn_e(l: &Ln) {
    let m = l.m;

    // h1 — the three bounds.  0.5-wide lines 2 long: 0.235 apart fire (vertical and
    // horizontal), 0.24 is clean; a 0.39 line beside a 0.2 line at 0.235 is not "wider
    // than 0.39" (clean), 0.395 is (fires); a parallel run of exactly 1.0 is clean,
    // 1.005 fires.  (0.235 clears Mn.b's 0.21.)
    l.write(
        ".e.h1",
        vec![
            rect(m, 2.0, 2.0, 2.5, 4.0),
            rect(m, 2.735, 2.0, 3.235, 4.0), // 0.235 → Mn.e
            rect(m, 5.0, 2.0, 5.5, 4.0),
            rect(m, 5.74, 2.0, 6.24, 4.0), // 0.24 → clean
            rect(m, 8.0, 2.0, 10.0, 2.5),
            rect(m, 8.0, 2.735, 10.0, 3.235), // 0.235 (y) → Mn.e
            rect(m, 12.0, 2.0, 12.39, 4.0),
            rect(m, 12.625, 2.0, 12.825, 4.0), // 0.39 wide → clean
            rect(m, 15.0, 2.0, 15.395, 4.0),
            rect(m, 15.63, 2.0, 15.83, 4.0), // 0.395 wide → Mn.e
            rect(m, 18.0, 2.0, 18.5, 3.0),
            rect(m, 18.735, 2.0, 19.235, 3.0), // run 1.0 → clean
            rect(m, 2.0, 6.0, 2.5, 7.005),
            rect(m, 2.735, 6.0, 3.235, 7.005), // run 1.005 → Mn.e
        ],
    );

    // h2 — which line is wide.  Wide left / narrow right fires; narrow left / wide right
    // fires; two narrow lines are clean; two wide lines fire once; a wide line between two
    // narrow ones fires twice.
    l.write(
        ".e.h2",
        vec![
            rect(m, 2.0, 2.0, 2.5, 4.0),
            rect(m, 2.735, 2.0, 2.935, 4.0),
            rect(m, 5.0, 2.0, 5.2, 4.0),
            rect(m, 5.435, 2.0, 5.935, 4.0),
            rect(m, 8.0, 2.0, 8.2, 4.0),
            rect(m, 8.435, 2.0, 8.635, 4.0),
            rect(m, 11.0, 2.0, 11.5, 4.0),
            rect(m, 11.735, 2.0, 12.235, 4.0),
            rect(m, 14.0, 2.0, 14.2, 4.0),
            rect(m, 14.435, 2.0, 14.935, 4.0),
            rect(m, 15.17, 2.0, 15.37, 4.0),
        ],
    );

    // h3 — where the width is.  A 0.2 line with a 0.5-wide bump 0.8 long facing a narrow
    // line at 0.235: the wide part runs 0.8 beside it → clean; the bump 1.005 long fires;
    // a wide line and a narrow one staggered so their runs overlap 1.0 (clean) and 1.005
    // (fires); a line whose wide part is on its far side, its facing wall straight: the
    // wide part is 2 long → fires.
    l.write(
        ".e.h3",
        vec![
            rect(m, 2.0, 2.0, 2.2, 5.0),
            rect(m, 2.2, 3.0, 2.5, 3.8),
            rect(m, 2.735, 2.0, 2.935, 5.0),
            rect(m, 5.0, 2.0, 5.2, 5.0),
            rect(m, 5.2, 3.0, 5.5, 4.005),
            rect(m, 5.735, 2.0, 5.935, 5.0),
            rect(m, 8.0, 2.0, 8.5, 4.0),
            rect(m, 8.735, 3.0, 8.935, 6.0),
            rect(m, 11.0, 2.0, 11.5, 4.0),
            rect(m, 11.735, 2.995, 11.935, 6.0),
            rect(m, 14.0, 2.0, 14.2, 5.0),
            rect(m, 13.7, 2.5, 14.0, 4.5),
            rect(m, 14.435, 2.0, 14.635, 5.0),
        ],
    );

    // h4 — shapes.  An Ln pad with 0.5 arms and a narrow line 0.235 from its vertical arm
    // (run 1.2) fires; a pad stepping wider on its far side, facing wall straight (run 2)
    // fires; a pad with a 0.6-long step towards the line, the rest 0.735 away, is clean;
    // a 2 × 2 plate beside a narrow line fires; a 0.5 line's end facing a narrow line
    // broadside (run 0.5) is clean; a 2 × 2 plate's end facing one (run 2) fires.
    l.write(
        ".e.h4",
        vec![
            poly(
                m,
                &[
                    (2.0, 2.0),
                    (4.0, 2.0),
                    (4.0, 2.5),
                    (2.5, 2.5),
                    (2.5, 4.0),
                    (2.0, 4.0),
                ],
            ),
            rect(m, 2.735, 2.8, 2.935, 4.8),
            rect(m, 6.0, 2.0, 6.2, 5.0),
            rect(m, 5.7, 2.5, 6.0, 4.5),
            rect(m, 5.4, 3.5, 5.7, 4.5),
            rect(m, 6.435, 2.0, 6.635, 5.0),
            rect(m, 9.0, 2.0, 9.5, 5.0),
            rect(m, 9.5, 3.2, 10.0, 3.8),
            rect(m, 10.235, 2.0, 10.435, 5.0),
            rect(m, 13.0, 2.0, 15.0, 4.0),
            rect(m, 15.235, 2.0, 15.435, 4.0),
            rect(m, 2.0, 6.0, 2.5, 8.0),
            rect(m, 1.0, 8.235, 4.0, 8.435),
            rect(m, 6.0, 6.0, 8.0, 8.0),
            rect(m, 5.0, 8.235, 9.0, 8.435),
        ],
    );

    // h5 — 45°.  Two 0.509-wide 45° strips 0.2333 apart (run 2.8) fire Mn.e and, being
    // bent, Mn.i; at 0.2404 both are clean; a wide strip beside a 0.2404 one at 0.2333
    // fires both; two 0.2404 strips at 0.2333 fire Mn.i only.
    l.write(
        ".e.h5",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.36),
            strip45(m, 2.0, 3.05, 2.0, 0.36),
            strip45(m, 7.0, 2.0, 2.0, 0.36),
            strip45(m, 7.0, 3.06, 2.0, 0.36),
            strip45(m, 12.0, 2.0, 2.0, 0.36),
            strip45(m, 12.0, 3.05, 2.0, 0.17),
            strip45(m, 17.0, 2.0, 2.0, 0.17),
            strip45(m, 17.0, 2.67, 2.0, 0.17),
        ],
    );

    // h6 — unions.  A wide line drawn as two overlapping 0.2 boxes (0.395) fires; as two
    // abutting boxes making 0.39 it is clean; a wide line in two abutting pieces along its
    // run (0.6 + 0.6) fires; a wide line facing two separate 0.8 lines 0.25 apart (each
    // run 0.8, 1.6 together) is clean.
    l.write(
        ".e.h6",
        vec![
            rect(m, 2.0, 2.0, 2.2, 4.0),
            rect(m, 2.195, 2.0, 2.395, 4.0),
            rect(m, 2.63, 2.0, 2.83, 4.0),
            rect(m, 5.0, 2.0, 5.2, 4.0),
            rect(m, 5.2, 2.0, 5.39, 4.0),
            rect(m, 5.625, 2.0, 5.825, 4.0),
            rect(m, 8.0, 2.0, 8.5, 2.6),
            rect(m, 8.0, 2.6, 8.5, 3.2),
            rect(m, 8.735, 2.0, 8.935, 3.2),
            rect(m, 11.0, 2.0, 11.5, 4.0),
            rect(m, 11.735, 2.0, 11.935, 2.8),
            rect(m, 11.735, 3.05, 11.935, 3.85),
        ],
    );

    // h7 — a notch.  A U whose 0.5 arms are 0.235 apart for 2 µm: Mn.e says "space", not
    // "space or notch" as Mn.b does; read as Mn.b's wording is read, a notch is not a
    // space and the U is clean.
    l.write(
        ".e.h7",
        vec![poly(
            m,
            &[
                (2.0, 2.0),
                (3.235, 2.0),
                (3.235, 4.5),
                (2.735, 4.5),
                (2.735, 2.5),
                (2.5, 2.5),
                (2.5, 4.5),
                (2.0, 4.5),
            ],
        )],
    );

    // h8 — tile lines.  A 1.2 run centred on x = 20 (0.6 in each 20 µm tile) fires; a 1.0
    // run centred on 20 is clean; a 1.4 run centred on 21 fires; a 0.235 gap straddling
    // 20 fires; a wall on 20 fires; a run across 40 and a gap straddling 42 fire; a 10 µm
    // run across 20 and 21 fires once; a 2 × 2 plate cornered on (20, 20) beside a narrow
    // line fires.
    l.write(
        ".e.h8",
        vec![
            rect(m, 19.4, 2.0, 20.6, 2.5),
            rect(m, 19.4, 2.735, 20.6, 2.935),
            rect(m, 19.5, 4.0, 20.5, 4.5),
            rect(m, 19.5, 4.735, 20.5, 4.935),
            rect(m, 20.3, 6.0, 21.7, 6.5),
            rect(m, 20.3, 6.735, 21.7, 6.935),
            rect(m, 19.4, 8.0, 19.9, 10.0),
            rect(m, 20.135, 8.0, 20.635, 10.0),
            rect(m, 19.5, 12.0, 20.0, 14.0),
            rect(m, 20.235, 12.0, 20.735, 14.0),
            rect(m, 39.4, 2.0, 40.6, 2.5),
            rect(m, 39.4, 2.735, 40.6, 2.935),
            rect(m, 41.4, 8.0, 41.9, 10.0),
            rect(m, 42.135, 8.0, 42.635, 10.0),
            rect(m, 15.0, 16.0, 25.0, 16.5),
            rect(m, 15.0, 16.735, 25.0, 16.935),
            rect(m, 18.0, 18.0, 20.0, 20.0),
            rect(m, 20.235, 18.0, 20.435, 20.0),
        ],
    );

    // h11 — a 300 µm pair (one violation) and a pair at (1000, 1000).
    l.write(
        ".e.h11",
        vec![
            rect(m, 2.0, 2.0, 302.0, 2.5),
            rect(m, 2.0, 2.735, 302.0, 2.935),
            rect(m, 1000.0, 1000.0, 1000.5, 1002.0),
            rect(m, 1000.735, 1000.0, 1000.935, 1002.0),
        ],
    );

    // h12 — under both: two wide lines 0.205 apart for 2 µm fire Mn.b and Mn.e.
    l.write(
        ".e.h12",
        vec![rect(m, 2.0, 2.0, 2.5, 4.0), rect(m, 2.705, 2.0, 3.205, 4.0)],
    );
}

// --- Mn.f: min. space 0.60 of lines if one is wider than 10.0 and the parallel run is
// more than 10.0 ---

fn mn_f(l: &Ln) {
    let m = l.m;

    // h1 — the three bounds.  12 × 12 plates 0.595 apart fire, 0.60 is clean; a plate
    // exactly 10.0 wide beside a 0.2 line at 0.595 is not "wider than 10.0" (clean),
    // 10.005 is (fires); a run of exactly 10.0 is clean, 10.005 fires.  (0.595 clears
    // Mn.e's 0.24 and Mn.b.)
    l.write(
        ".f.h1",
        vec![
            rect(m, 2.0, 2.0, 14.0, 14.0),
            rect(m, 14.595, 2.0, 26.595, 14.0),
            rect(m, 30.0, 2.0, 42.0, 14.0),
            rect(m, 42.6, 2.0, 54.6, 14.0),
            rect(m, 60.0, 2.0, 70.0, 14.0),
            rect(m, 70.595, 2.0, 70.795, 14.0),
            rect(m, 75.0, 2.0, 85.005, 14.0),
            rect(m, 85.6, 2.0, 85.8, 14.0),
            rect(m, 90.0, 2.0, 102.0, 12.0),
            rect(m, 102.595, 2.0, 114.595, 12.0),
            rect(m, 2.0, 20.0, 14.0, 30.005),
            rect(m, 14.595, 20.0, 26.595, 30.005),
        ],
    );

    // h2 — which line is wide.  A 0.2 line 12 long beside a 12 × 12 plate at 0.595 fires;
    // two 10 × 12 plates 0.595 apart are clean; an Ln plate with 12 arms and a narrow line
    // 0.595 from its vertical arm (run 10.5) fires; a 0.2 line with a 10.005 × 10.005 pad
    // on it, 0.595 from another line, fires; with a 10.005 × 9 pad it is clean.
    l.write(
        ".f.h2",
        vec![
            rect(m, 2.0, 2.0, 14.0, 14.0),
            rect(m, 14.595, 2.0, 14.795, 14.0),
            rect(m, 20.0, 2.0, 30.0, 14.0),
            rect(m, 30.595, 2.0, 40.595, 14.0),
            poly(
                m,
                &[
                    (45.0, 2.0),
                    (69.0, 2.0),
                    (69.0, 14.0),
                    (57.0, 14.0),
                    (57.0, 26.0),
                    (45.0, 26.0),
                ],
            ),
            rect(m, 57.595, 15.0, 57.795, 25.5),
            rect(m, 75.0, 2.0, 75.2, 32.0),
            rect(m, 75.2, 10.0, 85.205, 20.005),
            rect(m, 85.8, 2.0, 86.0, 32.0),
            rect(m, 90.0, 2.0, 90.2, 32.0),
            rect(m, 90.2, 10.0, 100.205, 19.0),
            rect(m, 100.8, 2.0, 101.0, 32.0),
        ],
    );

    // h3 — tile lines.  The plates are bigger than a 7 µm tile.  A pair whose run crosses
    // y = 20 fires; a 0.595 gap straddling x = 20 fires; a wall on 20 fires; on 40 and
    // straddling 42 fire; a 10.005 run split 5/5.005 by y = 20 fires; a 10.0 run split by
    // y = 20 is clean.
    l.write(
        ".f.h3",
        vec![
            rect(m, 2.0, 14.0, 14.0, 26.0),
            rect(m, 14.595, 14.0, 26.595, 26.0),
            rect(m, 7.9, 30.0, 19.9, 42.0),
            rect(m, 20.495, 30.0, 32.495, 42.0),
            rect(m, 8.0, 46.0, 20.0, 58.0),
            rect(m, 20.595, 46.0, 32.595, 58.0),
            rect(m, 28.0, 62.0, 40.0, 74.0),
            rect(m, 40.595, 62.0, 52.595, 74.0),
            rect(m, 29.9, 78.0, 41.9, 90.0),
            rect(m, 42.495, 78.0, 54.495, 90.0),
            rect(m, 40.0, 15.0, 52.0, 25.005),
            rect(m, 52.595, 15.0, 64.595, 25.005),
            rect(m, 70.0, 15.0, 82.0, 25.0),
            rect(m, 82.595, 15.0, 94.595, 25.0),
        ],
    );

    // h6 — 300 µm plates 0.595 apart (one violation) and a pair at (1000, 1000).
    l.write(
        ".f.h6",
        vec![
            rect(m, 2.0, 2.0, 302.0, 14.0),
            rect(m, 2.0, 14.595, 302.0, 26.595),
            rect(m, 1000.0, 1000.0, 1012.0, 1012.0),
            rect(m, 1012.595, 1000.0, 1024.595, 1012.0),
        ],
    );

    // h7 — 45°.  Two 12.02-wide 45° strips 20 long, 0.594 apart, fire Mn.f (Mn.i is
    // quiet, the gap is over 0.24); at 0.601 they are clean.
    l.write(
        ".f.h7",
        vec![
            strip45(m, 2.0, 2.0, 20.0, 8.5),
            strip45(m, 2.0, 19.84, 20.0, 8.5),
            strip45(m, 50.0, 2.0, 20.0, 8.5),
            strip45(m, 50.0, 19.85, 20.0, 8.5),
        ],
    );
}

// --- Mn.g: min. 45° bent width 0.24 if the bent length is > 0.5 ---

fn mn_g(l: &Ln) {
    let m = l.m;

    // h1 — the bounds.  45° strips (ends cut square to the run, no acute corner): 0.2404
    // wide with 1.41 walls is clean, 0.2333 fires; 0.2333 wide with 0.495 walls is clean
    // (not longer than 0.5), with 0.502 walls fires.
    l.write(
        ".g.h1",
        vec![
            strip45(m, 2.0, 2.0, 1.0, 0.17),
            strip45(m, 5.0, 2.0, 1.0, 0.165),
            strip45(m, 8.0, 2.0, 0.35, 0.165),
            strip45(m, 11.0, 2.0, 0.355, 0.165),
        ],
    );

    // h2 — routes.  A 0.2 line jogging up-right at 45° (jog 0.2015 wide): walls 0.509
    // fire, 0.495 are clean; a 0.24 line's jog (0.2404) is clean; a 0.205-wide arm off
    // the end of a 0.29 bar, its lower wall from the bar's corner and its upper wall
    // 0.29 higher, so the walls are `a·√2` and `(a − 0.145)·√2` long: 0.601 and 0.396 is
    // clean under the settled reading (each wall its own length), 0.707 and 0.502 fires.
    let arm = |x0: f64, a: f64| {
        poly(
            m,
            &[
                (x0, 6.0),
                (x0 + 2.0, 6.0),
                (x0 + 2.0 + a, 6.0 + a),
                (x0 + 2.0 + a - 0.145, 6.0 + a + 0.145),
                (x0 + 2.0, 6.29),
                (x0, 6.29),
            ],
        )
    };
    let e = vec![
        jog(m, 2.0, 2.0, 0.2, 0.285, 1.0, 0.36, 1.0),
        jog(m, 6.0, 2.0, 0.2, 0.285, 1.0, 0.35, 1.0),
        jog(m, 10.0, 2.0, 0.24, 0.34, 1.0, 1.0, 1.0),
        arm(2.0, 0.425),
        arm(6.0, 0.5),
    ];
    l.write(".g.h2", e);

    // h3 — an Ln with a 45° bend.  The 0.2 route's bend is chamfered on both corners so the
    // bend is a 0.2015-wide 45° segment: outer/inner walls 0.707/0.544 fire; 0.566/0.403
    // are clean under the settled reading; 0.424/0.262 are clean.  A 0.2333 diamond's
    // walls are 0.2333 long: clean.
    l.write(
        ".g.h3",
        vec![
            chamfered_ln(m, 2.0, 2.0, 0.5),
            chamfered_ln(m, 8.0, 2.0, 0.4),
            chamfered_ln(m, 14.0, 2.0, 0.3),
            diamond(m, 3.0, 8.0, 0.165),
        ],
    );

    // h4 — long and on the tile lines.  A 0.2333 strip 300 µm long (one violation); a
    // 1.41-wall strip across x = 20; 0.502 walls split by x = 20 (each part under 0.5)
    // fire; 0.495 walls across 20 are clean; 0.502 across 21, 40, 42 and across y = 20.
    l.write(
        ".g.h4",
        vec![
            strip45(m, 2.0, 2.0, 300.0, 0.165),
            strip45(m, 19.5, 6.0, 1.0, 0.165),
            strip45(m, 19.8, 10.0, 0.355, 0.165),
            strip45(m, 19.8, 13.0, 0.35, 0.165),
            strip45(m, 20.8, 16.0, 0.355, 0.165),
            strip45(m, 39.8, 6.0, 0.355, 0.165),
            strip45(m, 41.8, 10.0, 0.355, 0.165),
            strip45(m, 10.0, 19.8, 0.355, 0.165),
        ],
    );

    // h7 — a strip at (1000, 1000); a 0.198-wide strip (Mn.a as well as Mn.g).
    l.write(
        ".g.h7",
        vec![
            strip45(m, 1000.0, 1000.0, 1.0, 0.165),
            strip45(m, 2.0, 2.0, 1.0, 0.14),
        ],
    );
}

// --- Mn.i: min. space 0.24 of lines of which one is bent by 45° ---

fn mn_i(l: &Ln) {
    let m = l.m;

    // h1 — the bound.  Two 0.304-wide 45° strips (Mn.g quiet) 0.2333 apart fire; 0.2404
    // apart are clean.  Both clear Mn.b's 0.21.
    l.write(
        ".i.h1",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.215),
            strip45(m, 2.0, 2.76, 2.0, 0.215),
            strip45(m, 8.0, 2.0, 2.0, 0.215),
            strip45(m, 8.0, 2.77, 2.0, 0.215),
        ],
    );

    // h2 — straight against bent.  A box corner 0.2333 from a 45° wall fires (Mn.b clean),
    // 0.2404 is clean; a diamond tip 0.235 above a wall fires, 0.24 is clean; a chamfer
    // passing 0.2333 from a box corner fires, 0.2404 is clean; two straight boxes corner
    // to corner at 0.2333 have no 45° edge: clean.
    l.write(
        ".i.h2",
        vec![
            rect(m, 2.0, 2.0, 3.0, 3.0),
            strip135(m, 4.33, 2.0, 2.5, 0.215),
            rect(m, 6.0, 2.0, 7.0, 3.0),
            strip135(m, 8.34, 2.0, 2.5, 0.215),
            rect(m, 10.0, 2.0, 12.0, 3.0),
            diamond(m, 11.0, 3.735, 0.5),
            rect(m, 14.0, 2.0, 16.0, 3.0),
            diamond(m, 15.0, 3.74, 0.5),
            chamfered_tr(m, 2.0, 7.0, 4.0, 9.0, 12.87),
            rect(m, 4.1, 9.1, 5.1, 10.1),
            chamfered_tr(m, 7.0, 7.0, 9.0, 9.0, 17.86),
            rect(m, 9.1, 9.1, 10.1, 10.1),
            rect(m, 12.0, 7.0, 13.0, 8.0),
            rect(m, 13.165, 8.165, 14.165, 9.165),
        ],
    );

    // h3 — ends and one polygon.  A 45° strip's square-cut end 0.235 above a straight
    // wall fires (the strip is bent); a hairpin of two 45° strips 0.2333 apart, their ends
    // on one perpendicular and joined there by a 0.304 bar square to them, is one
    // polygon with a notch - "space", not "space or notch": clean by the wording,
    // KLayout's edge-based check reports it.
    l.write(
        ".i.h3",
        vec![
            rect(m, 1.0, 1.0, 4.0, 2.0),
            strip45(m, 2.5, 2.235, 1.0, 0.215),
            strip45(m, 2.0, 6.0, 1.5, 0.215),
            strip45(m, 2.0, 6.76, 1.12, 0.215),
            strip135(m, 3.5, 7.5, 0.595, 0.215),
        ],
    );

    // h4 — tile lines.  0.2333 strip pairs across x = 20, 21, 40, 42 and inside a tile; a
    // box cornered on x = 20 with a 45° wall 0.2333 from the corner.
    let pair = |x0: f64, y0: f64| {
        vec![
            strip45(m, x0, y0, 1.0, 0.215),
            strip45(m, x0, y0 + 0.76, 1.0, 0.215),
        ]
    };
    let mut e = pair(19.5, 2.0);
    e.extend(pair(20.5, 6.0));
    e.extend(pair(39.5, 2.0));
    e.extend(pair(41.5, 6.0));
    e.extend(pair(9.5, 2.0));
    e.push(rect(m, 18.0, 10.0, 20.0, 11.0));
    e.push(strip135(m, 21.33, 10.0, 2.5, 0.215));
    l.write(".i.h4", e);

    // h7 — 300 µm strips 0.2333 apart (one violation) and a pair at (1000, 1000).
    l.write(
        ".i.h7",
        vec![
            strip45(m, 2.0, 2.0, 300.0, 0.215),
            strip45(m, 2.0, 2.76, 300.0, 0.215),
            strip45(m, 1000.0, 1000.0, 1.0, 0.215),
            strip45(m, 1000.0, 1000.76, 1.0, 0.215),
        ],
    );

    // h8 — under both: strips 0.2051 apart fire Mn.b and Mn.i.
    l.write(
        ".i.h8",
        vec![
            strip45(m, 2.0, 2.0, 2.0, 0.215),
            strip45(m, 2.0, 2.72, 2.0, 0.215),
        ],
    );
}

// --- Mn.j/k and MnFil.h/k: density ---

fn mn_density(l: &Ln) {
    // j.h1 — the same 30 % stripes on Metal(n), Metal(n):filler and Metal(n).mask: the
    // union is 30 % of the die → Mn.j (35 %) fires, Mn.k and the windows are clean.  (The
    // 300 µm filler stripes are under MnFil.a2; the case sets the filler rules aside.)
    let mut e = vec![rect(l.bnd, 0.0, 0.0, 1000.0, 1000.0)];
    for k in 0..10 {
        let y0 = k as f64 * 100.0;
        e.push(rect(l.m, 0.0, y0, 1000.0, y0 + 30.0));
        e.push(rect(l.fil, 0.0, y0, 1000.0, y0 + 30.0));
        e.push(rect(l.mask, 0.0, y0, 1000.0, y0 + 30.0));
    }
    l.write(".j.h1", e);

    // Fil.h.h1 — a 1000 µm die covered in Metal(n) but for a 700 × 700 hole at (150, 150):
    // 51 % overall (Mn.j and Mn.k quiet); the 800 window at (100, 100) holds the whole
    // hole and reads 23.4 % → MnFil.h; no window reaches 75 %.
    l.write(
        "Fil.h.h1",
        vec![
            rect(l.bnd, 0.0, 0.0, 1000.0, 1000.0),
            rect(l.m, 0.0, 0.0, 1000.0, 150.0),
            rect(l.m, 0.0, 850.0, 1000.0, 1000.0),
            rect(l.m, 0.0, 150.0, 150.0, 850.0),
            rect(l.m, 850.0, 150.0, 1000.0, 850.0),
        ],
    );
}

// --- MnFil.a1/a2/b/c/d: the fillers ---

fn mnfil(l: &Ln) {
    let (f, m, t) = (l.fil, l.m, l.trans);

    // Fil.a1.h1 — min. filler width 1.0: 1.0 × 3 clean, 0.995 fires (both ways); a diamond
    // 1.004 across clean, 0.99 fires; an Ln with 1.0 arms clean; two overlapping 0.6 boxes
    // whose union is 0.995 fire.
    l.write(
        "Fil.a1.h1",
        vec![
            rect(f, 2.0, 2.0, 3.0, 5.0),
            rect(f, 5.0, 2.0, 5.995, 5.0),
            rect(f, 8.0, 2.0, 11.0, 2.995),
            diamond(f, 14.0, 3.5, 0.71),
            diamond(f, 18.0, 3.5, 0.70),
            poly(
                f,
                &[
                    (2.0, 8.0),
                    (5.0, 8.0),
                    (5.0, 9.0),
                    (3.0, 9.0),
                    (3.0, 11.0),
                    (2.0, 11.0),
                ],
            ),
            rect(f, 8.0, 8.0, 8.6, 11.0),
            rect(f, 8.395, 8.0, 8.995, 11.0),
        ],
    );

    // Fil.a2.h1 — max. filler width 5.0.  A 5 × 5 square is clean, 5.005 × 5.005 fires; a
    // 5.005 × 5.0 filler is 5.0 wide (clean); a 3 × 20 bar, an Ln with 3-wide arms in an
    // 8 × 8 box and a 6 × 6 frame with 2.25 walls are under 5 wide everywhere: clean.
    // (A width is a shape's smaller span, figure 4.1; IHP's KLayout deck takes the bounding
    // box instead.)
    let mut e = vec![
        rect(f, 2.0, 2.0, 7.0, 7.0),
        rect(f, 9.0, 2.0, 14.005, 7.005),
        rect(f, 16.0, 2.0, 21.005, 7.0),
        rect(f, 23.0, 2.0, 26.0, 22.0),
        poly(
            f,
            &[
                (2.0, 10.0),
                (10.0, 10.0),
                (10.0, 13.0),
                (5.0, 13.0),
                (5.0, 18.0),
                (2.0, 18.0),
            ],
        ),
    ];
    e.extend(l.ring(f, 12.0, 10.0, 18.0, 16.0, 14.25, 12.25, 15.75, 13.75));
    l.write("Fil.a2.h1", e);

    // Fil.b.h1 — filler space 0.42: 2 × 2 fillers 0.42 apart clean, 0.415 fire (x and y);
    // corner to corner 0.29/0.29 (0.41) fires, 0.30/0.30 (0.424) is clean; a diamond tip
    // 0.415 above a filler fires; a U filler with a 0.415 notch is "space", not notch:
    // clean by the wording.
    l.write(
        "Fil.b.h1",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(f, 4.42, 2.0, 6.42, 4.0),
            rect(f, 2.0, 4.415, 4.0, 6.415),
            rect(f, 8.0, 2.0, 10.0, 4.0),
            rect(f, 10.415, 2.0, 12.415, 4.0),
            rect(f, 14.0, 2.0, 16.0, 4.0),
            rect(f, 16.29, 4.29, 18.29, 6.29),
            rect(f, 20.0, 2.0, 22.0, 4.0),
            rect(f, 22.3, 4.3, 24.3, 6.3),
            rect(f, 26.0, 2.0, 30.0, 4.0),
            diamond(f, 28.0, 5.415, 1.0),
            poly(
                f,
                &[
                    (2.0, 9.0),
                    (4.415, 9.0),
                    (4.415, 12.0),
                    (3.415, 12.0),
                    (3.415, 10.0),
                    (3.0, 10.0),
                    (3.0, 12.0),
                    (2.0, 12.0),
                ],
            ),
        ],
    );

    // Fil.b.h2 — tile lines: 0.415 gaps straddling x = 20, 21, 40, 42 and inside a tile.
    let pair = |x: f64, y: f64| {
        vec![
            rect(f, x - 2.0, y, x, y + 2.0),
            rect(f, x + 0.415, y, x + 2.415, y + 2.0),
        ]
    };
    let mut e = pair(19.8, 2.0);
    e.extend(pair(20.8, 6.0));
    e.extend(pair(39.8, 2.0));
    e.extend(pair(41.8, 6.0));
    e.extend(pair(9.8, 2.0));
    l.write("Fil.b.h2", e);

    // Fil.c.h1 — filler space to Metal(n) 0.42: a 1 × 1 metal 0.42 from a 2 × 2 filler is
    // clean, 0.415 fires (x and y); a metal abutting the filler's edge is at no distance
    // (fires); one overlapping it by 0.2 shares area with it and is no pair (settled);
    // corner to corner 0.29/0.29 fires.
    l.write(
        "Fil.c.h1",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(m, 4.42, 2.5, 5.42, 3.5),
            rect(m, 2.5, 4.415, 3.5, 5.415),
            rect(f, 8.0, 2.0, 10.0, 4.0),
            rect(m, 10.415, 2.5, 11.415, 3.5),
            rect(f, 14.0, 2.0, 16.0, 4.0),
            rect(m, 16.0, 2.5, 17.0, 3.5),
            rect(f, 20.0, 2.0, 22.0, 4.0),
            rect(m, 21.8, 2.5, 22.8, 3.5),
            rect(f, 26.0, 2.0, 28.0, 4.0),
            rect(m, 28.29, 4.29, 29.29, 5.29),
        ],
    );

    // Fil.c.h2 — tile lines: metal 0.415 from a filler across x = 20, 21, 40, 42, inside.
    let pair = |x: f64, y: f64| {
        vec![
            rect(f, x - 2.0, y, x, y + 2.0),
            rect(m, x + 0.415, y + 0.5, x + 1.415, y + 1.5),
        ]
    };
    let mut e = pair(19.8, 2.0);
    e.extend(pair(20.8, 6.0));
    e.extend(pair(39.8, 2.0));
    e.extend(pair(41.8, 6.0));
    e.extend(pair(9.8, 2.0));
    l.write("Fil.c.h2", e);

    // Fil.d.h1 — filler space to TRANS 1.0: 1.0 clean, 0.995 fires (x and y); corner to
    // corner 0.70/0.70 (0.99) fires, 0.71/0.71 (1.004) is clean; a filler inside a TRANS
    // marker fires (the marker is what the rule keeps fillers out of); a filler across the
    // marker's edge shares area with it and is no pair (settled).
    l.write(
        "Fil.d.h1",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            rect(t, 5.0, 2.0, 7.0, 4.0),
            rect(t, 2.0, 4.995, 4.0, 6.995),
            rect(f, 10.0, 2.0, 12.0, 4.0),
            rect(t, 12.995, 2.0, 14.995, 4.0),
            rect(f, 18.0, 2.0, 20.0, 4.0),
            rect(t, 20.7, 4.7, 22.7, 6.7),
            rect(f, 26.0, 2.0, 28.0, 4.0),
            rect(t, 28.71, 4.71, 30.71, 6.71),
            rect(t, 2.0, 10.0, 8.0, 16.0),
            rect(f, 4.0, 12.0, 6.0, 14.0),
            rect(t, 12.0, 10.0, 18.0, 16.0),
            rect(f, 17.0, 12.0, 19.0, 14.0),
        ],
    );
}
