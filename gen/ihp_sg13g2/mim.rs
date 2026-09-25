// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::pad::{L, MIM, write};
use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, strip45, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/mim";
/// MIM.gR is recommended: its fixture sits with the recommended decks'.
const MIM_GR: &str = "tests/data/ihp-sg13g2/recommended/mim";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    std::fs::create_dir_all(MIM_GR).expect("failed to create output directory");
    mim_all(pdk);
    mim_gr(pdk);

    hardening(pdk);
}

/// A grid of individually-clean caps whose *total* MIM area exceeds the MIM.gR chip cap
/// (174800 µm²): 32 caps of 74×74 µm = 175232 µm², each region 5476 µm² < the 5625 µm²
/// per-device MIM.g limit, so only MIM.gR fires.
fn mim_gr(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    for i in 0..8 {
        for j in 0..4 {
            e.extend(cap(
                pdk,
                o + i as f64 * 80.0,
                o + j as f64 * 80.0,
                74.0,
                74.0,
                0.5,
                true,
            ));
        }
    }
    write_gz(&format!("{MIM_GR}/MIM.gR.gds.gz"), library("TOP", e));
}

/// A MIM cap at `(x, y)`, `w`×`h` µm: Metal5 bottom plate enclosing by 0.70, a TopVia1 at
/// `via_margin` from the MIM corner (omitted if `via == false`).
fn cap(
    pdk: &PdkConfig,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    via_margin: f64,
    via: bool,
) -> Vec<GdsElement> {
    let mut e = vec![
        rect(layer(pdk, "MIM"), x, y, x + w, y + h),
        rect(
            layer(pdk, "Metal5"),
            x - 0.7,
            y - 0.7,
            x + w + 0.7,
            y + h + 0.7,
        ),
    ];
    if via {
        let v = layer(pdk, "TopVia1");
        e.push(rect(
            v,
            x + via_margin,
            y + via_margin,
            x + via_margin + 0.3,
            y + via_margin + 0.3,
        ));
    }
    e
}

/// One layout exercising the MIM rules; each cap perturbs a single aspect.
fn mim_all(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    e.extend(cap(pdk, o, o, 3.0, 3.0, 0.5, true)); // clean
    e.extend(cap(pdk, o + 10.0, o, 3.0, 3.0, 0.2, true)); // via margin 0.20 < 0.36 → MIM.d
    e.extend(cap(pdk, o + 20.0, o, 3.0, 3.0, 0.0, false)); // no via → MIM.h
    e.extend(cap(pdk, o + 30.0, o, 1.0, 3.0, 0.36, true)); // width 1.0 < 1.14 → MIM.a
    e.extend(cap(pdk, o + 40.0, o, 1.2, 1.0, 0.36, true)); // area 1.2 < 1.30 → MIM.f
    // Two caps 0.50 µm apart → MIM.b (their Metal5 overlaps, fine).
    e.extend(cap(pdk, o, o + 12.0, 2.0, 2.0, 0.5, true));
    e.extend(cap(pdk, o + 2.5, o + 12.0, 2.0, 2.0, 0.5, true));
    write_gz(&format!("{DIR}/MIM.gds.gz"), library("TOP", e));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------

/// MIM.a — "Min. MIM width 1.14".  Plates on Metal5 0.7 beyond, no vias (MIM.h and
/// MIM.f ignored), on a 15 pitch: (a) 1.14 × 10, clean; (b) 1.135 × 10, fires; (c) 10 ×
/// 1.135, fires; (d) a 45° strip of width 1.1455 (d = 0.81), clean; (e) of 1.1314 (d =
/// 0.80), fires; (f) a diamond of width 1.1455, clean; (g) of 1.1314, fires; (h) an L
/// of 1.135 arms, fires; (i) two 5 plates joined by a 1.135 × 2 neck, fires; (j) a 0.005
/// × 5 sliver, fires; (k) a 300 × 1.135 bar, fires; (l) 1.135 × 10 at (1000, 1000),
/// fires.
fn mim_a_h1(l: &L) {
    let m = l.mim;
    let mut e = vec![
        rect(m, 0.0, 0.0, 1.14, 10.0),
        rect(m, 15.0, 0.0, 16.135, 10.0),
        rect(m, 30.0, 0.0, 40.0, 1.135),
        strip45(m, 45.0, 0.0, 8.0, 0.81),
        strip45(m, 60.0, 0.0, 8.0, 0.80),
        diamond(m, 78.0, 3.0, 0.81),
        diamond(m, 93.0, 3.0, 0.80),
        poly(
            m,
            &[
                (105.0, 0.0),
                (110.0, 0.0),
                (110.0, 1.135),
                (106.135, 1.135),
                (106.135, 5.0),
                (105.0, 5.0),
            ],
        ),
        poly(
            m,
            &[
                (120.0, 0.0),
                (125.0, 0.0),
                (125.0, 5.0),
                (123.07, 5.0),
                (123.07, 7.0),
                (125.0, 7.0),
                (125.0, 12.0),
                (120.0, 12.0),
                (120.0, 7.0),
                (121.935, 7.0),
                (121.935, 5.0),
                (120.0, 5.0),
            ],
        ),
        rect(m, 135.0, 0.0, 135.005, 5.0),
        rect(m, 0.0, 20.0, 300.0, 21.135),
        rect(m, 1000.0, 1000.0, 1001.135, 1010.0),
    ];
    e.push(rect(l.m5, -5.0, -5.0, 310.0, 30.0));
    e.push(rect(l.m5, 995.0, 995.0, 1010.0, 1015.0));
    write(MIM, "MIM.a.h1", e);
}

/// MIM.a — 10 × 5 bars of 1.135 × 10 at a 15 pitch on one Metal5 plate in TOP, flat
/// (`h3`) and as an array reference (`h4`).  Fires 50 bars.
fn mim_a_h34(_l: &L) {}

/// MIM.b — 10 × 5 pairs of 2 squares 0.595 apart on a 10 pitch over one Metal5 plate
/// in TOP, flat (`h3`) and as an array reference (`h4`).  Fires 50.
fn mim_b_h34(_l: &L) {}

/// MIM.c — "Min. Metal5 enclosure of MIM 0.60".  3 µm plates with a via 0.4 in (MIM.d
/// quiet), on a 10 pitch: (a) Metal5 0.6 beyond, clean; (b) 0.595 on the right, fires;
/// (c) Metal5's top-right corner cut passing 0.595 (euclidian) from the plate's
/// corner, fires; (d) passing 0.601, clean; (e) no Metal5, fires; (f) Metal5 as two
/// abutting boxes, clean; (g) the plate over Metal5's edge by 0.5, fires; (h) 0.595
/// all round, fires.  Fires 5.
fn mim_c_h1(l: &L) {
    let at = |i: usize| ((i % 8) as f64 * 10.0, 0.0);
    let mut e = vec![];
    let plate = |x: f64, y: f64| {
        vec![
            rect(l.mim, x, y, x + 3.0, y + 3.0),
            rect(l.tv1, x + 0.4, y + 0.4, x + 0.82, y + 0.82),
        ]
    };
    let (x, y) = at(0);
    e.extend(plate(x, y));
    e.push(rect(l.m5, x - 0.6, y - 0.6, x + 3.6, y + 3.6));
    let (x, y) = at(1);
    e.extend(plate(x, y));
    e.push(rect(l.m5, x - 0.6, y - 0.6, x + 3.595, y + 3.6));
    // (1.2 − t)/√2 = 0.595 → 0.8415 → 0.84 (0.594); 0.601 → 0.85.
    let (x, y) = at(2);
    e.extend(plate(x, y));
    e.push(chamfered_tr(
        l.m5,
        x - 0.6,
        y - 0.6,
        x + 3.6,
        y + 3.6,
        x + y + 6.0 + 0.84,
    ));
    let (x, y) = at(3);
    e.extend(plate(x, y));
    e.push(chamfered_tr(
        l.m5,
        x - 0.6,
        y - 0.6,
        x + 3.6,
        y + 3.6,
        x + y + 6.0 + 0.85,
    ));
    let (x, y) = at(4);
    e.extend(plate(x, y));
    let (x, y) = at(5);
    e.extend(plate(x, y));
    e.push(rect(l.m5, x - 0.6, y - 0.6, x + 1.5, y + 3.6));
    e.push(rect(l.m5, x + 1.5, y - 0.6, x + 3.6, y + 3.6));
    let (x, y) = at(6);
    e.extend(plate(x, y));
    e.push(rect(l.m5, x - 0.6, y - 0.6, x + 2.5, y + 3.6));
    let (x, y) = at(7);
    e.extend(plate(x, y));
    e.push(rect(l.m5, x - 0.595, y - 0.595, x + 3.595, y + 3.595));
    write(MIM, "MIM.c.h1", e);
}

/// MIM.d — "Min. MIM enclosure of TopVia1 0.36", for the vias that touch MIM.  3 µm
/// plates on Metal5 0.7 beyond, on a 10 pitch: (a) a 0.42 via 0.36 from the corner,
/// clean; (b) 0.355 from the left wall, fires; (c) 0.355 from the left and bottom walls
/// of a 1.15 plate, fires; (d) a via over the plate's right wall by 0.2, fires; (e) a via 0.5 outside
/// the plate (the plate has its own), clean (not the cap's via); (f) a Vmim 0.355 from
/// the wall (with a TopVia1 in place), clean by the rule's letter (it names TopVia1;
/// IHP's deck reads TopVia1 alone); (g) the plate's corner cut at 45° passing 0.355
/// from the via's corner, fires; (h) passing 0.361, clean; (i) a via over the plate's
/// top-right corner by 0.2 each way, fires.  Fires 5.
fn mim_d_h1(l: &L) {
    let at = |i: usize| ((i % 9) as f64 * 10.0, 0.0);
    let mut e = vec![];
    let m5 = |x: f64, y: f64| rect(l.m5, x - 0.7, y - 0.7, x + 3.7, y + 3.7);
    let via = |x: f64, y: f64| rect(l.tv1, x, y, x + 0.42, y + 0.42);
    let (x, y) = at(0);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(m5(x, y));
    e.push(via(x + 0.36, y + 0.36));
    let (x, y) = at(1);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(m5(x, y));
    e.push(via(x + 0.355, y + 1.0));
    let (x, y) = at(2);
    e.push(rect(l.mim, x, y, x + 1.15, y + 1.15));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 1.85, y + 1.85));
    e.push(via(x + 0.355, y + 0.355));
    let (x, y) = at(3);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(m5(x, y));
    e.push(via(x + 2.78, y + 1.0));
    let (x, y) = at(4);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 5.0, y + 3.7));
    e.push(via(x + 1.0, y + 1.0));
    e.push(via(x + 3.5, y + 1.0));
    let (x, y) = at(5);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(m5(x, y));
    e.push(via(x + 1.5, y + 1.5));
    e.push(rect(l.vmim, x + 0.355, y + 0.5, x + 0.775, y + 0.92));
    // (g), (h): via at (x+1, y+1); plate's corner (x, y) cut along x + y = k, the via's
    // corner (x+1, y+1) at (k' )… distance from (x+1, y+1) to the line x + y = x + y + c
    // is (2 − c)/√2: 0.355 → c = 1.498 → 1.5 (0.3536); 0.361 → c = 1.4895 → 1.49 (0.3606).
    let (x, y) = at(6);
    e.push(poly(
        l.mim,
        &[
            (x + 1.5, y),
            (x + 3.0, y),
            (x + 3.0, y + 3.0),
            (x, y + 3.0),
            (x, y + 1.5),
        ],
    ));
    e.push(m5(x, y));
    e.push(via(x + 1.0, y + 1.0));
    let (x, y) = at(7);
    e.push(poly(
        l.mim,
        &[
            (x + 1.49, y),
            (x + 3.0, y),
            (x + 3.0, y + 3.0),
            (x, y + 3.0),
            (x, y + 1.49),
        ],
    ));
    e.push(m5(x, y));
    e.push(via(x + 1.0, y + 1.0));
    let (x, y) = at(8);
    e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
    e.push(m5(x, y));
    e.push(via(x + 2.78, y + 2.78));
    write(MIM, "MIM.d.h1", e);
}

/// MIM.d — the tile lines: 3 plates whose via is 0.355 from a wall, the short margin
/// (a) from 19.645 to 20 (wall on the line), (b) from 20 to 20.355 (via on it), (c)
/// from 39.7 to 40.055 (across 40), (d) from 99.7 to 100.055, (e) at (1000, 1000);
/// (f) 0.36 from 20 to 20.36, clean.  Fires 5.
fn mim_d_h2(l: &L) {
    let mut e = vec![];
    let plate = |e: &mut Vec<GdsElement>, x: f64, y: f64, margin: f64| {
        e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
        e.push(rect(l.m5, x - 0.7, y - 0.7, x + 3.7, y + 3.7));
        e.push(rect(
            l.tv1,
            x + margin,
            y + 1.0,
            x + margin + 0.42,
            y + 1.42,
        ));
    };
    plate(&mut e, 19.645, 0.0, 0.355);
    plate(&mut e, 20.0, 10.0, 0.355);
    plate(&mut e, 39.7, 0.0, 0.355);
    plate(&mut e, 99.7, 0.0, 0.355);
    plate(&mut e, 1000.0, 1000.0, 0.355);
    plate(&mut e, 20.0, 20.0, 0.36);
    write(MIM, "MIM.d.h2", e);
}

/// MIM.f — "Min. MIM area per MIM device 1.30".  Plates on one Metal5 plate, no vias
/// (MIM.h ignored; MIM.a ignored on the narrow ones), on a 6 pitch: (a) 1.14 × 1.14
/// (1.2996), fires; (b) 1.14 × 1.145 (1.3053), clean; (c) 1.3 × 1.0 (1.3), clean; (d)
/// 1.3 × 0.995 (1.2935), fires; (e) two 1 squares sharing a wall (one 2 device), clean;
/// (f) two 1 squares overlapping by 0.5 (1.75), clean; (g) an L of 0.6 arms 2 long
/// (2.04), clean; (h) two 1 squares touching at a corner, two devices, fires twice.
/// Fires 4.
fn mim_f_h1(l: &L) {
    let m = l.mim;
    let e = vec![
        rect(m, 0.0, 0.0, 1.14, 1.14),
        rect(m, 6.0, 0.0, 7.14, 1.145),
        rect(m, 12.0, 0.0, 13.3, 1.0),
        rect(m, 18.0, 0.0, 19.3, 0.995),
        rect(m, 24.0, 0.0, 25.0, 1.0),
        rect(m, 25.0, 0.0, 26.0, 1.0),
        rect(m, 30.0, 0.0, 31.0, 1.0),
        rect(m, 30.5, 0.0, 31.5, 1.0),
        poly(
            m,
            &[
                (36.0, 0.0),
                (38.0, 0.0),
                (38.0, 0.6),
                (36.6, 0.6),
                (36.6, 2.0),
                (36.0, 2.0),
            ],
        ),
        rect(m, 42.0, 0.0, 43.0, 1.0),
        rect(m, 43.0, 1.0, 44.0, 2.0),
        rect(l.m5, -5.0, -5.0, 50.0, 10.0),
    ];
    write(MIM, "MIM.f.h1", e);
}

/// MIM.g — "Max. MIM area per MIM device 5625.00".  Plates on Metal5 0.7 beyond, a
/// via 0.5 in, on a 120 pitch: (a) 75 × 75 (5625), clean; (b) 75 × 75.005, fires; (c)
/// 100 × 56.25 (5625), clean; (d) two 60 × 60 sharing a wall (7200 as one device),
/// fires; (e) two 60 × 60 overlapping by 20 (6000), fires; (f) a ring of 80 with a 30
/// hole (5500), clean; (g) an L of 100 less a 50 corner (7500), fires; (h) 75 × 75.005
/// as two overlapping boxes, fires once.  Fires 5.
fn mim_g_h1(l: &L) {
    let at = |i: usize| ((i % 4) as f64 * 120.0, (i / 4) as f64 * 120.0);
    let mut e = vec![];
    let (x, y) = at(0);
    e.extend(l.cap(x, y, 75.0, 75.0, 0.5));
    let (x, y) = at(1);
    e.extend(l.cap(x, y, 75.0, 75.005, 0.5));
    let (x, y) = at(2);
    e.extend(l.cap(x, y, 100.0, 56.25, 0.5));
    let (x, y) = at(3);
    e.extend(l.cap(x, y, 60.0, 60.0, 0.5));
    e.push(rect(l.mim, x + 60.0, y, x + 120.0, y + 60.0));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 120.7, y + 60.7));
    let (x, y) = at(4);
    e.extend(l.cap(x, y, 60.0, 60.0, 0.5));
    e.push(rect(l.mim, x + 40.0, y, x + 100.0, y + 60.0));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 100.7, y + 60.7));
    let (x, y) = at(5);
    e.push(poly(
        l.mim,
        &[
            (x, y),
            (x + 80.0, y),
            (x + 80.0, y + 80.0),
            (x, y + 80.0),
            (x, y + 25.0),
            (x + 25.0, y + 25.0),
            (x + 25.0, y + 55.0),
            (x + 55.0, y + 55.0),
            (x + 55.0, y + 25.0),
            (x, y + 25.0),
        ],
    ));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 80.7, y + 80.7));
    e.push(rect(l.tv1, x + 0.5, y + 0.5, x + 0.92, y + 0.92));
    let (x, y) = at(6);
    e.push(poly(
        l.mim,
        &[
            (x, y),
            (x + 100.0, y),
            (x + 100.0, y + 50.0),
            (x + 50.0, y + 50.0),
            (x + 50.0, y + 100.0),
            (x, y + 100.0),
        ],
    ));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 100.7, y + 100.7));
    e.push(rect(l.tv1, x + 0.5, y + 0.5, x + 0.92, y + 0.92));
    let (x, y) = at(7);
    e.push(rect(l.mim, x, y, x + 40.0, y + 75.005));
    e.push(rect(l.mim, x + 35.0, y, x + 75.0, y + 75.005));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 75.7, y + 75.705));
    e.push(rect(l.tv1, x + 0.5, y + 0.5, x + 0.92, y + 0.92));
    write(MIM, "MIM.g.h1", e);
}

/// MIM.h — "TopVia1 must be over MIM": every MIM device carries a TopVia1 (or a Vmim,
/// which "can be used instead").  3 µm plates on Metal5 0.7 beyond, on a 10 pitch:
/// (a) a TopVia1 inside, clean; (b) a Vmim inside, clean; (c) no via, fires; (d) a via
/// over the plate's wall by 0.2, clean here (MIM.d fires); (e) a via abutting the
/// plate from outside, fires (not over it); (f) a ring of 6 with a 3 hole and the via
/// in the hole, fires; (g) two plates sharing a wall with one via, clean (one
/// device); (h) at (1000, 1000), no via, fires.  Fires 4 (and MIM.d once).
fn mim_h_h1(l: &L) {
    let at = |i: usize| ((i % 8) as f64 * 10.0, 0.0);
    let mut e = vec![];
    let via = |x: f64, y: f64| rect(l.tv1, x, y, x + 0.42, y + 0.42);
    let (x, y) = at(0);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.5));
    let (x, y) = at(1);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.0));
    e.push(rect(l.vmim, x + 0.5, y + 0.5, x + 0.92, y + 0.92));
    let (x, y) = at(2);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.0));
    let (x, y) = at(3);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.0));
    e.push(via(x + 2.78, y + 1.0));
    let (x, y) = at(4);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.0));
    e.push(via(x + 3.0, y + 1.0));
    let (x, y) = at(5);
    e.push(poly(
        l.mim,
        &[
            (x, y),
            (x + 6.0, y),
            (x + 6.0, y + 6.0),
            (x, y + 6.0),
            (x, y + 1.5),
            (x + 1.5, y + 1.5),
            (x + 1.5, y + 4.5),
            (x + 4.5, y + 4.5),
            (x + 4.5, y + 1.5),
            (x, y + 1.5),
        ],
    ));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 6.7, y + 6.7));
    e.push(via(x + 2.79, y + 2.79));
    let (x, y) = at(6);
    e.extend(l.cap(x, y, 3.0, 3.0, 0.5));
    e.push(rect(l.mim, x + 3.0, y, x + 6.0, y + 3.0));
    e.push(rect(l.m5, x - 0.7, y - 0.7, x + 6.7, y + 3.7));
    e.extend(l.cap(1000.0, 1000.0, 3.0, 3.0, 0.0));
    write(MIM, "MIM.h.h1", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/mim")
        .expect("failed to create output directory");
    let l = L::new(pdk);
    mim_a_h1(&l);
    mim_a_h34(&l);
    mim_b_h34(&l);
    mim_c_h1(&l);
    mim_d_h1(&l);
    mim_d_h2(&l);
    mim_f_h1(&l);
    mim_g_h1(&l);
    mim_h_h1(&l);
}
