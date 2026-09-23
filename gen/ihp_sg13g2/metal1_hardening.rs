// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Hardening layouts (hardening/SPEC.md) for the `metal1` deck, drawn from sections 5.16
// (M1.a-M1.k) and 5.18 (M1Fil.*) of the SG13G2 rule manual alone, one fixture per theme,
// `<rule>.h<n>`.  Each function's comment states the geometry and what the manual says
// about it; the expected answers are in the `metal1` table of tests/ihp-sg13g2.rs and the
// reasoning in hardening/reports/ihp-sg13g2/metal1.md.

use crate::helpers::{
    chamfered_tr, cont_at, diamond, layer, library, poly, rect, strip45, stripes, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

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

pub fn generate(pdk: &PdkConfig) {
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
