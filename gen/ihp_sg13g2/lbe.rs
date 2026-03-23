// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, max_width_pattern, notch_pattern, density_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/lbe";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    lbe_a(pdk);
    lbe_b(pdk);
    lbe_b1(pdk);
    lbe_b1_merge(pdk);
    lbe_b2(pdk);
    lbe_c_space(pdk);
    lbe_c_notch(pdk);
    lbe_d(pdk);
    lbe_e(pdk);
    lbe_f(pdk);
    lbe_h(pdk);
    lbe_h_open(pdk);
    lbe_i(pdk);
}

/// LBE.b2 — min. LBE area 30000 µm².  A 200×200 (40000) region is clean; a 150×150 (22500)
/// region is too small.
fn lbe_b2(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 200.0, o + 200.0),            // 40000 µm² → clean
        rect(l, o + 300.0, o, o + 450.0, o + 150.0),    // 22500 µm² → LBE.b2
    ];
    write_gz(&format!("{DIR}/LBE.b2.gds.gz"), library("TOP", elems));
}

/// LBE.e — min. LBE space to dfpad and Passiv 50.0.  An LBE plate with a dfpad and a Passiv
/// shape each 49 µm away → one LBE.e violation per neighbour.
fn lbe_e(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let dfpad = layer(pdk, "dfpad");
    let passiv = layer(pdk, "Passiv");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 200.0, o + 200.0),
        rect(dfpad, o + 249.0, o, o + 349.0, o + 200.0),          // 49 µm gap → LBE.e
        rect(passiv, o, o + 249.0, o + 200.0, o + 349.0),         // 49 µm gap → LBE.e
    ];
    write_gz(&format!("{DIR}/LBE.e.gds.gz"), library("TOP", elems));
}

/// LBE.f — min. LBE space to Activ 30.0.  An LBE plate with an Activ shape 29 µm away.
fn lbe_f(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 200.0, o + 200.0),
        rect(activ, o + 229.0, o, o + 329.0, o + 200.0),          // 29 µm gap → LBE.f
    ];
    write_gz(&format!("{DIR}/LBE.f.gds.gz"), library("TOP", elems));
}

fn lbe_a(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = min_width_pattern(l, 100.0, 100.0, 200.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/LBE.a.gds.gz"), library("TOP", elems));
}

fn lbe_b(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = max_width_pattern(l, 1500.0, 1500.0, 1610.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/LBE.b.gds.gz"), library("TOP", elems));
}

fn lbe_b1(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = vec![
        // clean: exactly at the limit
        rect(l, 0.0, 0.0, 500.00, 500.00),
        // too small
        rect(l, 600.0, 0.0, 1100.00, 500.005),
        rect(l, 1200.0, 0.0, 1700.005, 500.00),
    ];

    write_gz(&format!("{DIR}/LBE.b1.gds.gz"), library("TOP", elems));
}

/// Edge case: two abutting rectangles, each below the 250000 µm² ceiling (500×300 =
/// 150000), merge into one 500×600 region of 300000 µm² → a single `max_area`
/// violation.  Confirms `max_area` is measured per merged region, not per shape.
fn lbe_b1_merge(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = vec![
        rect(l, 0.0, 0.0, 500.0, 300.0),
        rect(l, 0.0, 300.0, 500.0, 600.0), // abuts the first along y = 300
    ];

    write_gz(&format!("{DIR}/LBE.b1.merge.gds.gz"), library("TOP", elems));
}

fn lbe_c_space(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = space_pattern(l, l, 100.0, 100.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/LBE.c.space.gds.gz"), library("TOP", elems));
}

fn lbe_c_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = notch_pattern(l, 100.0, 100.0, 100.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/LBE.c.notch.gds.gz"), library("TOP", elems));
}

fn lbe_d(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let edgeseal = layer(pdk, "EdgeSeal");
    let elems = space_pattern(l, edgeseal, 100.0, 150.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/LBE.d.space.gds.gz"), library("TOP", elems));
}

fn lbe_h(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    // Closed ring → encloses a hole → no_ring violation.
    let elems = vec![
        rect(l, 0.0, 0.0, 500.00, 100.00),
        rect(l, 0.0, 100.0, 100.00, 500.00),
        rect(l, 400.0, 100.0, 500.00, 500.00),
        rect(l, 0.0, 400.0, 500.00, 500.00),
    ];

    write_gz(&format!("{DIR}/LBE.h.gds.gz"), library("TOP", elems));
}

/// Edge case: an open U (the ring's top side removed).  The interior connects to
/// the exterior, so there is no enclosed hole and `no_ring` must stay clean.
fn lbe_h_open(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let elems = vec![
        rect(l, 0.0, 0.0, 500.00, 100.00),   // bottom
        rect(l, 0.0, 100.0, 100.00, 500.00), // left arm
        rect(l, 400.0, 100.0, 500.00, 500.00), // right arm
        // no top piece → open
    ];

    write_gz(&format!("{DIR}/LBE.h.open.gds.gz"), library("TOP", elems));
}

fn lbe_i(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let boundary = layer(pdk, "EdgeSeal");
    // max_density: the single LBE stripe rises above the 20 % ceiling when too tall.
    let elems = density_pattern(boundary, 1000.0, &[(l, 0.0, 200.0)]);
    write_gz(&format!("{DIR}/LBE.i.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &[(l, 0.0, 200.01)]);
    write_gz(&format!("{DIR}/LBE.i.fail.gds.gz"), library("TOP", elems_fail));
}
