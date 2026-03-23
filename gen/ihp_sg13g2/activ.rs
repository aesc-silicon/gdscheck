// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, max_width_pattern, notch_pattern, density_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/activ";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR)
        .expect("failed to create output directory");

    act_a(pdk);
    act_b_space(pdk);
    act_b_notch(pdk);
    act_c(pdk);
    act_e(pdk);
    act_d(pdk);
    act_d_merge(pdk);
    afil_a(pdk);
    afil_a1(pdk);
    afil_b(pdk);
    afil_c_cont(pdk);
    afil_c_gatpoly(pdk);
    afil_c1(pdk);
    afil_d_nwell(pdk);
    afil_d_nbulay(pdk);
    afil_e(pdk);
    afil_i(pdk);
    afil_j(pdk);
    afil_g(pdk);
    afil_g1(pdk);
    afil_g2(pdk);
    afil_g3(pdk);
    afil_g2_boundary(pdk);
    afil_g2_boundary_ring(pdk);
}

fn act_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = min_width_pattern(l, 0.15, 0.15, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.a.gds.gz"), library("TOP", elems));
}

fn act_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = space_pattern(l, l, 1.0, 0.21, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.b.space.gds.gz"), library("TOP", elems));
}

fn act_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = notch_pattern(l, 0.15, 0.21, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.b.notch.gds.gz"), library("TOP", elems));
}

/// Act.c — min. Activ drain/source extension 0.23 µm.  A gate crossing an Activ where
/// the S/D extends 0.30/1.34 µm (clean) and one where the right S/D extends only 0.14 µm
/// (violation); the gate-poly ends running out past the Activ are exempt.
fn act_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        // clean transistor: gate at x∈[o+0.5, o+0.66] across Activ x∈[o, o+2]
        rect(activ, o, o, o + 2.0, o + 1.0),
        rect(gp, o + 0.5, o - 0.3, o + 0.66, o + 1.3),
        // violation: right S/D extension only 0.14 µm (Activ ends at o+4.6, gate at o+4.46)
        rect(activ, o + 4.0, o, o + 4.6, o + 1.0),
        rect(gp, o + 4.3, o - 0.3, o + 4.46, o + 1.3),
    ];
    write_gz(&format!("{DIR}/Act.c.gds.gz"), library("TOP", elems));
}

/// Act.e — min. Activ enclosed area 0.15 µm².  An Activ ring around a 0.3×0.3 = 0.09 µm²
/// hole violates; a ring around a 0.5×0.5 = 0.25 µm² hole is clean.  Holes are 0.3/0.5 µm
/// (≥ 0.21) so they don't read as notches.
fn act_e(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    // A square Activ ring of arm width `w` around an empty `hs`×`hs` hole at (x, y).
    let ring = |x: f64, y: f64, hs: f64, w: f64| {
        vec![
            rect(l, x - w, y - w, x + hs + w, y),           // bottom
            rect(l, x - w, y + hs, x + hs + w, y + hs + w), // top
            rect(l, x - w, y, x, y + hs),                   // left
            rect(l, x + hs, y, x + hs + w, y + hs),         // right
        ]
    };
    let o = OFFSET;
    let mut elems = ring(o + 0.3, o + 0.3, 0.30, 0.3); // hole 0.09 µm² → violation
    elems.extend(ring(o + 4.0, o + 0.3, 0.50, 0.3)); // hole 0.25 µm² → clean
    write_gz(&format!("{DIR}/Act.e.gds.gz"), library("TOP", elems));
}

fn act_d(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = vec![
        // clean: exactly at the limit
        rect(l, 0.0, 0.0, 0.35, 0.35),
        // too small
        rect(l, 0.0, 5.0, 0.345, 5.35),
    ];

    write_gz(&format!("{DIR}/Act.d.gds.gz"), library("TOP", elems));
}

/// Edge case: two abutting rectangles, each below the 0.122 µm² floor (0.2×0.35 =
/// 0.07), merge into one 0.4×0.35 = 0.14 µm² region → clean.  Confirms `min_area`
/// is measured per merged region, not per shape (per shape, both would fail).
fn act_d_merge(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = vec![
        rect(l, 0.0, 0.0, 0.20, 0.35),
        rect(l, 0.20, 0.0, 0.40, 0.35), // abuts the first along x = 0.20
    ];

    write_gz(&format!("{DIR}/Act.d.merge.gds.gz"), library("TOP", elems));
}

fn afil_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let elems = max_width_pattern(l, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.a.gds.gz"), library("TOP", elems));
}

fn afil_a1(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let elems = min_width_pattern(l, 1.0, 1.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.a1.gds.gz"), library("TOP", elems));
}

fn afil_b(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let elems = space_pattern(l, l, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.b.gds.gz"), library("TOP", elems));
}

/// AFil.c — min. Activ:filler space to Cont (1.10 µm).
fn afil_c_cont(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let cont = layer(pdk, "Cont");
    let elems = space_pattern(l, cont, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.c.cont.gds.gz"), library("TOP", elems));
}

/// AFil.c — min. Activ:filler space to GatPoly (1.10 µm).
fn afil_c_gatpoly(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let gp = layer(pdk, "GatPoly");
    let elems = space_pattern(l, gp, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.c.gatpoly.gds.gz"), library("TOP", elems));
}

fn afil_c1(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let act = layer(pdk, "Activ");
    let elems = space_pattern(l, act, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.c1.gds.gz"), library("TOP", elems));
}

/// AFil.d — min. Activ:filler space to NWell (1.00 µm).
fn afil_d_nwell(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.d.nwell.gds.gz"), library("TOP", elems));
}

/// AFil.d — min. Activ:filler space to nBuLay (1.00 µm).
fn afil_d_nbulay(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let nbl = layer(pdk, "nBuLay");
    let elems = space_pattern(l, nbl, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.d.nbulay.gds.gz"), library("TOP", elems));
}

/// AFil.e — min. Activ:filler space to TRANS (1.00 µm).
fn afil_e(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(l, trans, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.e.gds.gz"), library("TOP", elems));
}

/// AFil.i — min. Activ:filler space to edges of PWell:block (1.50 µm).
fn afil_i(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let blk = layer(pdk, "PWell.block");
    let elems = space_pattern(l, blk, 2.0, 1.50, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.i.gds.gz"), library("TOP", elems));
}

/// AFil.j — (nSD:block ∩ SalBlock) must enclose Activ:filler-inside-PWell:block by
/// 0.25 µm.  One filler enclosed by 0.25 (clean); one with a 0.24 left margin (fail).
fn afil_j(pdk: &PdkConfig) {
    let fill = layer(pdk, "Activ.filler");
    let pwb = layer(pdk, "PWell.block");
    let nsdb = layer(pdk, "nSD.block");
    let sal = layer(pdk, "SalBlock");
    let o = OFFSET;
    // A filler inside PWell:block, enclosed by both nSD:block and SalBlock by `margin`.
    let cell = |x: f64, margin: f64| {
        vec![
            rect(fill, x, o, x + 1.0, o + 1.0),
            rect(pwb, x - 0.1, o - 0.1, x + 1.1, o + 1.1), // filler is inside the block
            rect(nsdb, x - margin, o - 0.25, x + 1.25, o + 1.25),
            rect(sal, x - margin, o - 0.25, x + 1.25, o + 1.25),
        ]
    };
    let mut elems = cell(o, 0.25); // 0.25 all round → clean
    elems.extend(cell(o + 4.0, 0.24)); // 0.24 left → violation
    write_gz(&format!("{DIR}/AFil.j.gds.gz"), library("TOP", elems));
}

fn afil_g(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let afil = layer(pdk, "Activ.filler");
    let amask = layer(pdk, "Activ.mask");
    let boundary = layer(pdk, "EdgeSeal");
    // min_density: bottom Activ stripe drops below the 35 % floor when too short.
    let stripes = |h: f64| [(act, 0.0, h), (afil, 500.0, 600.0), (amask, 900.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{DIR}/AFil.g.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(149.99));
    write_gz(&format!("{DIR}/AFil.g.fail.gds.gz"), library("TOP", elems_fail));
}

fn afil_g1(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let afil = layer(pdk, "Activ.filler");
    let amask = layer(pdk, "Activ.mask");
    let boundary = layer(pdk, "EdgeSeal");
    // max_density: bottom Activ stripe rises above the 55 % ceiling when too tall.
    let stripes = |h: f64| [(act, 0.0, h), (afil, 400.0, 600.0), (amask, 800.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{DIR}/AFil.g1.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(150.01));
    write_gz(&format!("{DIR}/AFil.g1.fail.gds.gz"), library("TOP", elems_fail));
}

fn afil_g2(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal");
    // min_windowed_density: shapes split per 800 µm window rather than full-width.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend([
        rect(act, 0.0, 0.0, 800.0, 200.0),
        rect(act, 800.0, 0.0, 1000.0, 200.0),
        rect(act, 0.0, 800.0, 800.0, 850.0),
        rect(act, 800.0, 800.0, 1000.0, 850.0),
    ]);
    write_gz(&format!("{DIR}/AFil.g2.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend([
        rect(act, 0.0, 0.0, 800.0, 199.99),
        rect(act, 800.0, 0.0, 1000.0, 199.99),
        rect(act, 0.0, 800.0, 800.0, 849.99),
        rect(act, 800.0, 800.0, 1000.0, 849.9),
    ]);
    write_gz(&format!("{DIR}/AFil.g2.fail.gds.gz"), library("TOP", elems_fail));
}

fn afil_g3(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal");
    // max_windowed_density: shapes split per 800 µm window rather than full-width.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend([
        rect(act, 0.0, 0.0, 800.0, 520.0),
        rect(act, 800.0, 0.0, 1000.0, 520.0),
        rect(act, 0.0, 800.0, 800.0, 930.0),
        rect(act, 800.0, 800.0, 1000.0, 930.0),
    ]);
    write_gz(&format!("{DIR}/AFil.g3.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend([
        rect(act, 0.0, 0.0, 800.0, 520.01),
        rect(act, 800.0, 0.0, 1000.0, 520.01),
        rect(act, 0.0, 800.0, 800.0, 930.01),
        rect(act, 800.0, 800.0, 1000.0, 930.01),
    ]);
    write_gz(&format!("{DIR}/AFil.g3.fail.gds.gz"), library("TOP", elems_fail));
}

/// AFil.g2/g3 boundary handling: the chip's raw bounding box (from *all* shapes)
/// extends past the true EdgeSeal — a small unrelated marker on TRANS sits outside the
/// seal ring, at (950, 950)-(1000, 1000), stretching the overall bbox from the sealed
/// 900x900 die out to 1000x1000.  With an 800 µm window this makes the last row/column
/// of tiles straddle the seal boundary, so their `boundary_layer`-clipped area (only the
/// part actually inside EdgeSeal) must be used as the density denominator — not the
/// nominal (and here doubled) window footprint.  Uniform 40% fill (period-100, height-40
/// stripes) sits comfortably inside [25%, 65%] everywhere, including the
/// boundary-straddling tiles, so a clean DRC here proves the fix; without it, the last
/// row/column would be measured against a doubled denominator and register a false ~20%
/// underfill.
fn afil_g2_boundary(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal");
    let trans = layer(pdk, "TRANS");

    let mut elems = vec![rect(boundary, 0.0, 0.0, 900.0, 900.0), rect(trans, 950.0, 950.0, 1000.0, 1000.0)];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(act, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(&format!("{DIR}/AFil.g2.boundary_ok.gds.gz"), library("TOP", elems));
}

/// Same as [`afil_g2_boundary`], but EdgeSeal is drawn as a hollow ring (4 strips)
/// around the die instead of a solid square — a real seal ring is a frame, not a filled
/// shape, so its own merged *area* is only the thin frame material, far smaller than the
/// 900x900 it encloses.  `boundary_layer` must fall back on the ring's bounding box (its
/// die extent), not its drawn area, or the density denominator collapses to almost
/// nothing and every window reads a wildly inflated (1000%+) density.  Same uniform 40%
/// fill and out-of-seal TRANS marker; expect a clean DRC exactly as with a solid
/// boundary square.
fn afil_g2_boundary_ring(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
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
        elems.push(rect(act, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(&format!("{DIR}/AFil.g2.boundary_ring.gds.gz"), library("TOP", elems));
}
