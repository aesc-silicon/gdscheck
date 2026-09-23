// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::sealring::{P, SQRT2, frame, frame_poly, oct_frame, write};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    density_pattern, diamond, layer, library, max_width_pattern, min_width_pattern, notch_pattern,
    rect, space_pattern, strip45, write_gz,
};
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

    hardening(pdk);
}

/// LBE.b2 — min. LBE area 30000 µm².  A 200×200 (40000) region is clean; a 150×150 (22500)
/// region is too small.
fn lbe_b2(pdk: &PdkConfig) {
    let l = layer(pdk, "LBE");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 200.0, o + 200.0),         // 40000 µm² → clean
        rect(l, o + 300.0, o, o + 450.0, o + 150.0), // 22500 µm² → LBE.b2
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
        rect(dfpad, o + 249.0, o, o + 349.0, o + 200.0), // 49 µm gap → LBE.e
        rect(passiv, o, o + 249.0, o + 200.0, o + 349.0), // 49 µm gap → LBE.e
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
        rect(activ, o + 229.0, o, o + 329.0, o + 200.0), // 29 µm gap → LBE.f
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
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // max_density: the single LBE stripe rises above the 20 % ceiling when too tall.
    let elems = density_pattern(boundary, 1000.0, &[(l, 0.0, 200.0)]);
    write_gz(&format!("{DIR}/LBE.i.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &[(l, 0.0, 200.01)]);
    write_gz(
        &format!("{DIR}/LBE.i.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts (hardening/SPEC.md) for the `sealring`, `slit`, `lbe` and `lu`
// decks: sections 6.10 (Sealring, Seal.*), 7.3 (Metal Slits, Slt.*), 9.1 (Localized
// Backside Etching, LBE.*) and 7.2.2 (Latch-up, LU.*) of the SG13G2 layout rules.
// Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`; each function's
// comment states the geometry and what the manual says about it, the expected answers
// are in the decks' tables of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/beol_misc.md.
//
// The seal frames are drawn as IHP's `sealring` pcell draws them: the EdgeSeal marker
// is the 4.2 µm ring itself, the conductors coincide with it, the via rings are 4.2
// long pieces overlapping into a ring, and the Passiv ring lies 3 µm outside.

/// h1 - the bound and geometry.  Every shape 100 or more from the others: (a) 100 ×
/// 200, clean; (b) 99.995 × 200 fires; (c) 200 × 99.995 fires; (d) a 45° strip
/// 100.006 wide (`d` 70.715): clean; (e) 99.985 wide (`d` 70.7): fires; (f) a diamond
/// 100.006 across (`a` 70.715): clean; (g) 99.985 across (`a` 70.7): fires; (h) an L
/// of 99.995 arms 300 long: fires; (i) a 0.005 × 200 sliver: fires; (j) a 300 × 100
/// bar: clean.  The slivers and bars under 30000 µm² are LBE.b2's business.
fn lbe_a_h(p: &P) {
    let l = p.lbe;
    let e = vec![
        rect(l, 0.0, 0.0, 100.0, 200.0),       // (a)
        rect(l, 200.0, 0.0, 299.995, 200.0),   // (b)
        rect(l, 400.0, 0.0, 600.0, 99.995),    // (c)
        strip45(l, 800.0, 0.0, 150.0, 70.715), // (d): bbox (729.285, 0)-(950, 220.715)
        strip45(l, 1150.0, 0.0, 150.0, 70.7),  // (e)
        diamond(l, 1500.0, 150.0, 70.715),     // (f)
        diamond(l, 1750.0, 150.0, 70.7),       // (g)
        rect(l, 0.0, 400.0, 99.995, 700.0),    // (h)
        rect(l, 0.0, 400.0, 300.0, 499.995),
        rect(l, 500.0, 400.0, 500.005, 600.0), // (i)
        rect(l, 700.0, 400.0, 1000.0, 500.0),  // (j)
    ];
    write("lbe", "LBE.a.h1", e);
}

/// h1 - the bound and the long side.  (a) 1500 × 200, clean; (b) 1500.005 × 200
/// fires; (c) 200 × 1500.005 fires; (d) an L of 1000 arms 200 wide (its box 1000):
/// clean; (e) two abutting 800 × 200 boxes: one 1600 bar, fires once; (f) two 800 ×
/// 200 boxes 100 apart: two bars of 800, clean.
fn lbe_b_h(p: &P) {
    let l = p.lbe;
    let e = vec![
        rect(l, 0.0, 0.0, 1500.0, 200.0),       // (a)
        rect(l, 0.0, 300.0, 1500.005, 500.0),   // (b)
        rect(l, 2100.0, 0.0, 2300.0, 1500.005), // (c)
        rect(l, 0.0, 600.0, 1000.0, 800.0),     // (d)
        rect(l, 0.0, 600.0, 200.0, 1600.0),
        rect(l, 300.0, 900.0, 1100.0, 1100.0), // (e)
        rect(l, 1100.0, 900.0, 1900.0, 1100.0),
        rect(l, 300.0, 1200.0, 1100.0, 1400.0), // (f)
        rect(l, 1200.0, 1200.0, 2000.0, 1400.0),
    ];
    write("lbe", "LBE.b.h1", e);
}

/// h1 - LBE.b1.  (a) 500 × 500, clean; (b) 500 × 500.005 fires; (c) two 500 × 300
/// boxes sharing a wall: 300000, fires once; (d) two 500 × 300 boxes touching at a
/// corner only: two shapes of 150000, clean; (e) a ring 600 outer with 150 walls:
/// 270000, fires (and LBE.h); (f) an L of 300 × 500 and 200 × 300: 210000, clean.
fn lbe_b1_h(p: &P) {
    let l = p.lbe;
    let mut e = vec![
        rect(l, 0.0, 0.0, 500.0, 500.0),      // (a)
        rect(l, 600.0, 0.0, 1100.0, 500.005), // (b)
        rect(l, 1200.0, 0.0, 1700.0, 300.0),  // (c)
        rect(l, 1200.0, 300.0, 1700.0, 600.0),
        rect(l, 0.0, 700.0, 500.0, 1000.0), // (d)
        rect(l, 500.0, 1000.0, 1000.0, 1300.0),
        rect(l, 2000.0, 700.0, 2300.0, 1200.0), // (f)
        rect(l, 2300.0, 700.0, 2500.0, 1000.0),
    ];
    e.extend(frame(l, 1200.0, 700.0, 1800.0, 1300.0, 150.0)); // (e)
    write("lbe", "LBE.b1.h1", e);
}

/// h1 - LBE.b2.  (a) 200 × 150 (30000), clean; (b) 200 × 149.995 fires; (c) two 150 ×
/// 100 boxes sharing a wall: 30000, clean; (d) two 150 × 100 boxes touching at a
/// corner: 15000 each, fires twice; (e) 100 × 300, clean.
fn lbe_b2_h(p: &P) {
    let l = p.lbe;
    let e = vec![
        rect(l, 0.0, 0.0, 200.0, 150.0),     // (a)
        rect(l, 300.0, 0.0, 500.0, 149.995), // (b)
        rect(l, 600.0, 0.0, 750.0, 100.0),   // (c)
        rect(l, 600.0, 100.0, 750.0, 200.0),
        rect(l, 850.0, 0.0, 1000.0, 100.0), // (d)
        rect(l, 1000.0, 100.0, 1150.0, 200.0),
        rect(l, 1250.0, 0.0, 1350.0, 300.0), // (e)
    ];
    write("lbe", "LBE.b2.h1", e);
}

/// h1 - what a ring is.  (a) a ring 500 outer with 100 walls: fires; (b) a U (no top
/// wall): clean; (c) a ring drawn as two abutting U's: one ring after merging, fires;
/// (d) a ring with a 100 square in its hole (100 from every wall): fires once; (e)
/// the ring as one polygon with a hole: fires; (f) a C - the ring with a 0.005 gap in
/// its top wall: no hole, clean (the gap is LBE.c's); (g) a ring with 45° corners:
/// fires.
fn lbe_h_h(p: &P) {
    let l = p.lbe;
    let mut e = frame(l, 0.0, 0.0, 500.0, 500.0, 100.0); // (a)
    e.push(rect(l, 600.0, 0.0, 1100.0, 100.0)); // (b)
    e.push(rect(l, 600.0, 0.0, 700.0, 500.0));
    e.push(rect(l, 1000.0, 0.0, 1100.0, 500.0));
    for (x0, x1) in [(1200.0, 1450.0), (1450.0, 1700.0)] {
        // (c): each U is the ring's half
        e.push(rect(l, x0, 0.0, x1, 100.0));
        e.push(rect(l, x0, 400.0, x1, 500.0));
        let wall = if x0 == 1200.0 {
            (x0, x0 + 100.0)
        } else {
            (x1 - 100.0, x1)
        };
        e.push(rect(l, wall.0, 0.0, wall.1, 500.0));
    }
    e.extend(frame(l, 0.0, 600.0, 500.0, 1100.0, 100.0)); // (d)
    e.push(rect(l, 200.0, 800.0, 300.0, 900.0));
    e.push(frame_poly(l, 600.0, 600.0, 1100.0, 1100.0, 100.0)); // (e)
    e.push(rect(l, 1200.0, 600.0, 1700.0, 700.0)); // (f)
    e.push(rect(l, 1200.0, 600.0, 1300.0, 1100.0));
    e.push(rect(l, 1600.0, 600.0, 1700.0, 1100.0));
    e.push(rect(l, 1200.0, 1000.0, 1449.995, 1100.0));
    e.push(rect(l, 1450.0, 1000.0, 1700.0, 1100.0));
    e.push(oct_frame(
        l,
        1800.0,
        0.0,
        2300.0,
        500.0,
        150.0,
        100.0,
        141.43 / SQRT2,
    )); // (g)
    write("lbe", "LBE.h.h1", e);
}

/// h1-h4 - the chip.  An EdgeSeal.boundary 1000 square: h1, two LBE 200 × 500 plates
/// (20.000 %): clean; h2, 200 × 500 and 200 × 500.05 (20.001 %): fires; h3, one LBE
/// 300 × 500 reaching 100 beyond the boundary's left edge and one 200 × 500: 20 % of
/// the chip lies under LBE - what is beyond the boundary is no chip (and Seal.l's):
/// clean; h4, no boundary at all, one LBE 300 × 300: no chip to take a density of.
fn lbe_i_h(p: &P) {
    let l = p.lbe;
    let b = rect(p.bnd, 0.0, 0.0, 1000.0, 1000.0);
    write(
        "lbe",
        "LBE.i.h1",
        vec![
            b.clone(),
            rect(l, 100.0, 100.0, 300.0, 600.0),
            rect(l, 500.0, 100.0, 700.0, 600.0),
        ],
    );
    write(
        "lbe",
        "LBE.i.h2",
        vec![
            b.clone(),
            rect(l, 100.0, 100.0, 300.0, 600.0),
            rect(l, 500.0, 100.0, 700.0, 600.05),
        ],
    );
    write(
        "lbe",
        "LBE.i.h3",
        vec![
            b,
            rect(l, -100.0, 100.0, 200.0, 600.0),
            rect(l, 500.0, 100.0, 700.0, 600.0),
        ],
    );
    write("lbe", "LBE.i.h4", vec![rect(l, 100.0, 100.0, 400.0, 400.0)]);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/lbe")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    lbe_a_h(&p);
    lbe_b_h(&p);
    lbe_b1_h(&p);
    lbe_b2_h(&p);
    lbe_h_h(&p);
    lbe_i_h(&p);
}
