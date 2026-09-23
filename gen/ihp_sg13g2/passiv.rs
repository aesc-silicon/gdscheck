// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pad::{L, write};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, diamond, enclosure_pattern, layer, library, min_width_pattern, notch_pattern,
    poly, rect, space_pattern, strip45, write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/passiv";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pas_a(pdk);
    pas_b_space(pdk);
    pas_b_notch(pdk);
    pas_c(pdk);

    hardening(pdk);
}

fn pas_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = min_width_pattern(l, 2.10, 2.10, 10.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.a.gds.gz"), library("TOP", elems));
}

fn pas_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = space_pattern(l, l, 2.1, 3.5, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.b.space.gds.gz"), library("TOP", elems));
}

fn pas_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = notch_pattern(l, 2.1, 3.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.b.notch.gds.gz"), library("TOP", elems));
}

/// `Pas.c` — min TopMetal2 enclosure of Passiv (2.10 µm), checked only inside the
/// seal (`PassivInSeal`).  Two identical enclosure patterns: the first is covered by
/// an EdgeSeal box (so it is checked and its 4 under-enclosed cases violate); the
/// second has no EdgeSeal, so it is outside the seal and must produce no violations.
fn pas_c(pdk: &PdkConfig) {
    let tm2 = layer(pdk, "TopMetal2");
    let passiv = layer(pdk, "Passiv");
    let edgeseal = layer(pdk, "EdgeSeal");

    let (enc, width, dist) = (2.10, 10.0, 5.0);
    let outer = width + 2.0 * enc; // outer square side of each enclosure pair
    let step = outer + dist;
    let span = 4.0 * step + outer; // x-extent of the 5-pair pattern from its offset

    let mut elems = Vec::new();
    // Inside the seal: fully covered by an EdgeSeal box → checked.
    elems.append(&mut enclosure_pattern(
        tm2,
        passiv,
        enc,
        width,
        dist,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.push(rect(
        edgeseal,
        OFFSET - 1.0,
        -1.0,
        OFFSET + span + 1.0,
        outer + 1.0,
    ));
    // Outside the seal: identical pattern, no EdgeSeal → exempt, no violations.
    let off2 = OFFSET + span + 50.0;
    elems.append(&mut enclosure_pattern(
        tm2,
        passiv,
        enc,
        width,
        dist,
        off2,
        SPACE_DELTA,
    ));

    write_gz(&format!("{DIR}/Pas.c.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------

const PAS: &str = "tests/data/ihp-sg13g2/passiv";

/// Pas.a — "Min. Passiv width 2.10".  On a 15 pitch: (a) 2.1 × 10 bar, clean; (b)
/// 2.095 × 10, fires; (c) 10 × 2.095, fires; (d) a 45° strip of width 2.1001 (d =
/// 1.485), clean; (e) of 2.093 (d = 1.48), fires; (f) a diamond of width 2.1001,
/// clean; (g) of 2.093, fires; (h) an L of 2.095 arms, fires; (i) two 10 × 10 plates
/// joined by a 2.095 × 4 neck, fires; (j) a 0.005 × 5 sliver, fires; (k) a 300 × 2.095
/// bar, fires; (l) 2.095 × 10 at (1000, 1000), fires.
fn pas_a_h1(l: &L) {
    let p = l.passiv;
    let e = vec![
        rect(p, 0.0, 0.0, 2.1, 10.0),
        rect(p, 15.0, 0.0, 17.095, 10.0),
        rect(p, 30.0, 0.0, 40.0, 2.095),
        strip45(p, 45.0, 0.0, 10.0, 1.485),
        strip45(p, 60.0, 0.0, 10.0, 1.48),
        diamond(p, 80.0, 5.0, 1.485),
        diamond(p, 95.0, 5.0, 1.48),
        poly(
            p,
            &[
                (105.0, 0.0),
                (115.0, 0.0),
                (115.0, 2.095),
                (107.095, 2.095),
                (107.095, 10.0),
                (105.0, 10.0),
            ],
        ),
        poly(
            p,
            &[
                (120.0, 0.0),
                (130.0, 0.0),
                (130.0, 10.0),
                (126.045, 10.0),
                (126.045, 14.0),
                (130.0, 14.0),
                (130.0, 24.0),
                (120.0, 24.0),
                (120.0, 14.0),
                (123.95, 14.0),
                (123.95, 10.0),
                (120.0, 10.0),
            ],
        ),
        rect(p, 135.0, 0.0, 135.005, 5.0),
        rect(p, 0.0, 30.0, 300.0, 32.095),
        rect(p, 1000.0, 1000.0, 1002.095, 1010.0),
    ];
    write(PAS, "Pas.a.h1", e);
}

/// Pas.a — 10 × 5 bars of 2.095 × 10 at a 15 pitch, flat (`h3`) and as an array
/// reference (`h4`).  Fires 50 bars.
fn pas_a_h34(_l: &L) {}

/// Pas.b — 10 × 5 pairs of 5 squares 3.495 apart on a 20 pitch, flat (`h3`) and as an
/// array reference (`h4`).  Fires 50.
fn pas_b_h34(_l: &L) {}

/// Pas.c — "Min. TopMetal2 enclosure of Passiv 2.10", "not checked outside of sealring
/// (edge-seal-passive)".  An EdgeSeal frame (0, 0)-(120, 120) 10 wide; in its hole, 10
/// µm openings on a 25 pitch: (a) TopMetal2 2.1 beyond, clean; (b) 2.095 on the right,
/// fires; (c) TopMetal2's top-right corner cut passing 2.093 (euclidian) from the
/// opening's corner, fires; (d) passing 2.104, clean; (e) no TopMetal2, fires; (f)
/// TopMetal2 as two abutting boxes, clean.  Outside: (g) an opening 2.095 short of
/// TopMetal2 beyond the frame at (140, 20), clean (not checked); (h) an opening 2.095
/// short on the frame itself at (2, 60): on the seal ring is not "outside of
/// sealring", so it is checked and fires (IHP's deck reads the frame's holes alone and
/// stays silent; IHP's own seal ring pcell keeps its Passiv ring 3 µm outside the
/// EdgeSeal ring, so the case never arises there).  Fires 4.
fn pas_c_h1(l: &L) {
    let frame = |x0: f64, y0: f64, x1: f64, y1: f64, w: f64| {
        poly(
            l.seal,
            &[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, y0 + w),
                (x0 + w, y0 + w),
                (x0 + w, y1 - w),
                (x1 - w, y1 - w),
                (x1 - w, y0 + w),
                (x0, y0 + w),
            ],
        )
    };
    let mut e = vec![frame(0.0, 0.0, 120.0, 120.0, 10.0)];
    let at = |i: usize| ((i % 4) as f64 * 25.0 + 15.0, (i / 4) as f64 * 25.0 + 15.0);
    let op = |x: f64, y: f64| rect(l.passiv, x, y, x + 10.0, y + 10.0);
    let (x, y) = at(0);
    e.push(op(x, y));
    e.push(rect(l.tm2, x - 2.1, y - 2.1, x + 12.1, y + 12.1));
    let (x, y) = at(1);
    e.push(op(x, y));
    e.push(rect(l.tm2, x - 2.1, y - 2.1, x + 12.095, y + 12.1));
    // The corner sits at x + y + 20; a cut c beyond it passes c/√2: 2.96 → 2.093, 2.975 → 2.104.
    let (x, y) = at(2);
    e.push(op(x, y));
    e.push(chamfered_tr(
        l.tm2,
        x - 2.1,
        y - 2.1,
        x + 12.1,
        y + 12.1,
        x + y + 20.0 + 2.96,
    ));
    let (x, y) = at(3);
    e.push(op(x, y));
    e.push(chamfered_tr(
        l.tm2,
        x - 2.1,
        y - 2.1,
        x + 12.1,
        y + 12.1,
        x + y + 20.0 + 2.975,
    ));
    let (x, y) = at(4);
    e.push(op(x, y));
    let (x, y) = at(5);
    e.push(op(x, y));
    e.push(rect(l.tm2, x - 2.1, y - 2.1, x + 5.0, y + 12.1));
    e.push(rect(l.tm2, x + 5.0, y - 2.1, x + 12.1, y + 12.1));
    e.push(op(140.0, 20.0));
    e.push(rect(l.tm2, 137.9, 17.9, 152.095, 32.1));
    e.push(rect(l.passiv, 2.0, 60.0, 8.0, 70.0));
    e.push(rect(l.tm2, -0.1, 57.9, 10.095, 72.1));
    write(PAS, "Pas.c.h1", e);
}

/// Pas.c — the tile lines: an EdgeSeal frame (10, 10)-(210, 210) 10 wide (hole from
/// 20).  Openings inside with TopMetal2 2.095 short: (a) an opening from x = 22.095
/// with TopMetal2 from x = 20 (the short margin on the line), fires; (b) an opening
/// (30, 60)-(40, 70) with TopMetal2 to x = 42.095 (across 40 and 42), fires; (c) an
/// opening (95, 95)-(105, 105) with TopMetal2 2.095 beyond on every side (across 100),
/// fires; (d) an opening (90, 130)-(100, 140) with TopMetal2 to 102.1, clean; (e) at
/// (1000, 1000) inside its own frame, 2.095 short, fires.  Fires 4.
fn pas_c_h2(l: &L) {
    let frame = |x0: f64, y0: f64, x1: f64, y1: f64, w: f64| {
        poly(
            l.seal,
            &[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, y0 + w),
                (x0 + w, y0 + w),
                (x0 + w, y1 - w),
                (x1 - w, y1 - w),
                (x1 - w, y0 + w),
                (x0, y0 + w),
            ],
        )
    };
    let mut e = vec![frame(10.0, 10.0, 210.0, 210.0, 10.0)];
    e.push(rect(l.passiv, 22.095, 30.0, 32.095, 40.0));
    e.push(rect(l.tm2, 20.0, 27.9, 34.195, 42.1));
    e.push(rect(l.passiv, 30.0, 60.0, 40.0, 70.0));
    e.push(rect(l.tm2, 27.9, 57.9, 42.095, 72.1));
    e.push(rect(l.passiv, 95.0, 95.0, 105.0, 105.0));
    e.push(rect(l.tm2, 92.905, 92.905, 107.095, 107.095));
    e.push(rect(l.passiv, 90.0, 130.0, 100.0, 140.0));
    e.push(rect(l.tm2, 87.9, 127.9, 102.1, 142.1));
    e.push(frame(980.0, 980.0, 1040.0, 1040.0, 10.0));
    e.push(rect(l.passiv, 1000.0, 1000.0, 1010.0, 1010.0));
    e.push(rect(l.tm2, 997.9, 997.9, 1012.095, 1012.1));
    write(PAS, "Pas.c.h2", e);
}

/// Pas.c — 10 × 5 openings 2.095 short of TopMetal2 on a 25 pitch inside one EdgeSeal
/// frame drawn in TOP, flat (`h3`) and as an array reference (`h4`).  Fires 50.
fn pas_c_h34(_l: &L) {}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/passiv")
        .expect("failed to create output directory");
    let l = L::new(pdk);
    pas_a_h1(&l);
    pas_a_h34(&l);
    pas_b_h34(&l);
    pas_c_h1(&l);
    pas_c_h2(&l);
    pas_c_h34(&l);
}
