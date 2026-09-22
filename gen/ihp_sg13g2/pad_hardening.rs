// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Hardening patterns (hardening/SPEC.md) for the `pad`, `passiv` and `mim` decks:
// layouts drawn from the manual's sections 6.9 (Pad.a1, Pad.d, Pad.i, Padb.*, Padc.*),
// 5.27 (Pas.a-Pas.c) and 6.11 (MIM.a-MIM.h) alone, one fixture per theme,
// `<RULE>.h<k>`.  The recommended `*R` pad rules are deliberately not in the deck and
// not drawn.  Each function's comment states the geometry and what the manual says
// about it; the expected answers are in the `pad`/`passiv`/`mim` tables of
// tests/ihp-sg13g2.rs and the reasoning in hardening/reports/ihp-sg13g2/pad.md.
//
// A pad here is what section 6.9 recognises: Passiv (or Passiv:sbump / Passiv:pillar)
// over dfpad, with TopMetal2 under the dfpad (Pad.i).  IHP's driver recognises a bump
// or pillar pad by the dfpad:sbump / dfpad:pillar datatypes as well, so the bump and
// pillar pads carry both dfpad datatypes for the oracle's sake.

use crate::helpers::{
    chamfered_tr, diamond, layer, library, poly, rect, ref_array, shift, strip45, write_gz,
};
use gds21::{GdsBoundary, GdsElement};
use gdscheck::pdk::PdkConfig;
use std::f64::consts::{SQRT_2, TAU};

const PAD: &str = "tests/data/ihp-sg13g2/pad";
const PAS: &str = "tests/data/ihp-sg13g2/passiv";
const MIM: &str = "tests/data/ihp-sg13g2/mim";

pub fn generate(pdk: &PdkConfig) {
    for d in [PAD, PAS, MIM] {
        std::fs::create_dir_all(d).expect("failed to create output directory");
    }
    let l = L::new(pdk);

    pad_a1_h1(&l);
    pad_a1_h2(&l);
    pad_d_h1(&l);
    pad_d_h2(&l);
    pad_d_h34(&l);
    pad_i_h1(&l);
    pad_i_h23(&l);
    padb_a_h1(&l);
    padb_a_h2(&l);
    padb_a_h34(&l);
    padb_b_h1(&l);
    padb_b_h2(&l);
    padb_b_h34(&l);
    padb_c_h1(&l);
    padb_c_h2(&l);
    padb_c_h34(&l);
    padb_d_h1(&l);
    padb_f_h1(&l);
    padc_a_h1(&l);
    padc_b_h1(&l);
    padc_c_h1(&l);
    padc_d_h1(&l);
    padc_f_h1(&l);

    pas_a_h1(&l);
    pas_a_h2(&l);
    pas_a_h34(&l);
    pas_b_h1(&l);
    pas_b_h2(&l);
    pas_b_h34(&l);
    pas_c_h1(&l);
    pas_c_h2(&l);
    pas_c_h34(&l);

    mim_a_h1(&l);
    mim_a_h2(&l);
    mim_a_h34(&l);
    mim_b_h1(&l);
    mim_b_h2(&l);
    mim_b_h34(&l);
    mim_c_h1(&l);
    mim_c_h2(&l);
    mim_d_h1(&l);
    mim_d_h2(&l);
    mim_e_h1(&l);
    mim_f_h1(&l);
    mim_g_h1(&l);
    mim_gr_h1(&l);
    mim_h_h1(&l);
}

/// The drawn layers of the three decks.
struct L {
    passiv: (i16, i16),
    sbump: (i16, i16),
    pillar: (i16, i16),
    dfpad: (i16, i16),
    dfpad_sbump: (i16, i16),
    dfpad_pillar: (i16, i16),
    tm2: (i16, i16),
    tm1: (i16, i16),
    activ: (i16, i16),
    seal: (i16, i16),
    mim: (i16, i16),
    m5: (i16, i16),
    tv1: (i16, i16),
    vmim: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            passiv: layer(pdk, "Passiv"),
            sbump: layer(pdk, "Passiv.sbump"),
            pillar: layer(pdk, "Passiv.pillar"),
            dfpad: layer(pdk, "dfpad"),
            dfpad_sbump: layer(pdk, "dfpad.sbump"),
            dfpad_pillar: layer(pdk, "dfpad.pillar"),
            tm2: layer(pdk, "TopMetal2"),
            tm1: layer(pdk, "TopMetal1"),
            activ: layer(pdk, "Activ"),
            seal: layer(pdk, "EdgeSeal"),
            mim: layer(pdk, "MIM"),
            m5: layer(pdk, "Metal5"),
            tv1: layer(pdk, "TopVia1"),
            vmim: layer(pdk, "Vmim"),
        }
    }

    /// A bond pad opening: `shape` on Passiv and dfpad, and TopMetal2 `margin` beyond
    /// its bounding box (0 for the shape itself, the bump pads want 15 for Padb.c).
    fn opening(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(&[self.passiv, self.dfpad], &shape);
        v.push(if margin == 0.0 {
            relayer(&shape, self.tm2)
        } else {
            grown_bbox(&shape, self.tm2, margin)
        });
        v
    }

    /// A pad opening as a box.
    fn open_box(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        self.opening(rect(self.passiv, x0, y0, x1, y1), 0.0)
    }

    /// A solder-bump pad: `shape` on Passiv, Passiv:sbump, dfpad and dfpad:sbump, in a
    /// TopMetal2 box `margin` beyond its bounding box.
    fn bump(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(
            &[self.passiv, self.sbump, self.dfpad, self.dfpad_sbump],
            &shape,
        );
        v.push(grown_bbox(&shape, self.tm2, margin));
        v
    }

    /// A copper-pillar pad, as `bump` with the pillar datatypes.
    fn pillar_pad(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(
            &[self.passiv, self.pillar, self.dfpad, self.dfpad_pillar],
            &shape,
        );
        v.push(grown_bbox(&shape, self.tm2, margin));
        v
    }

    /// A seal-ring fragment: the box on Activ and EdgeSeal.
    fn seal_activ(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.activ, x0, y0, x1, y1),
            rect(self.seal, x0, y0, x1, y1),
        ]
    }

    /// A seal ring: a frame `w` wide from `(x0, y0)` to `(x1, y1)` on Activ and
    /// EdgeSeal, its four corners cut at 45° over `k` (Seal.k asks 21).
    fn seal_ring(&self, x0: f64, y0: f64, x1: f64, y1: f64, w: f64, k: f64) -> Vec<GdsElement> {
        let outer = chamfered_all(x0, y0, x1, y1, k);
        let inner = chamfered_all(x0 + w, y0 + w, x1 - w, y1 - w, k - w * SQRT_2);
        let mut pts = outer.clone();
        pts.push(outer[0]);
        let mut rev: Vec<(f64, f64)> = inner.iter().rev().cloned().collect();
        rev.push(inner[inner.len() - 1]);
        pts.extend(rev);
        vec![poly(self.activ, &pts), poly(self.seal, &pts)]
    }

    /// A MIM plate at `(x, y)`, `w × h`, on Metal5 with 0.7 to spare all round; a
    /// TopVia1 with `via` margin from the plate's lower-left corner when `via > 0`.
    fn cap(&self, x: f64, y: f64, w: f64, h: f64, via: f64) -> Vec<GdsElement> {
        let mut v = vec![
            rect(self.mim, x, y, x + w, y + h),
            rect(self.m5, x - 0.7, y - 0.7, x + w + 0.7, y + h + 0.7),
        ];
        if via > 0.0 {
            v.push(rect(
                self.tv1,
                x + via,
                y + via,
                x + via + 0.42,
                y + via + 0.42,
            ));
        }
        v
    }
}

/// The eight corners of the box `(x0, y0)-(x1, y1)` with every corner cut at 45° `k`
/// along each axis, counter-clockwise from the bottom-left.
fn chamfered_all(x0: f64, y0: f64, x1: f64, y1: f64, k: f64) -> Vec<(f64, f64)> {
    vec![
        (x0 + k, y0),
        (x1 - k, y0),
        (x1, y0 + k),
        (x1, y1 - k),
        (x1 - k, y1),
        (x0 + k, y1),
        (x0, y1 - k),
        (x0, y0 + k),
    ]
}

/// The same boundary on another layer.
fn relayer(e: &GdsElement, l: (i16, i16)) -> GdsElement {
    match e {
        GdsElement::GdsBoundary(b) => GdsElement::GdsBoundary(GdsBoundary {
            layer: l.0,
            datatype: l.1,
            ..b.clone()
        }),
        other => other.clone(),
    }
}

/// The same boundary on every layer of `ls`.
fn on(ls: &[(i16, i16)], e: &GdsElement) -> Vec<GdsElement> {
    ls.iter().map(|&l| relayer(e, l)).collect()
}

/// The bounding box of `e` grown by `m` on every side, on `l`.
fn grown_bbox(e: &GdsElement, l: (i16, i16), m: f64) -> GdsElement {
    let GdsElement::GdsBoundary(b) = e else {
        panic!("not a boundary")
    };
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in &b.xy {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let f = |v: i32| v as f64 / 1000.0;
    rect(l, f(x0) - m, f(y0) - m, f(x1) + m, f(y1) + m)
}

/// A regular `n`-gon of circumradius `r` about `(cx, cy)`, a vertex on the +x axis: the
/// circle the manual allows for a bump pad (IHP's pcell draws 64 points).
fn ngon(l: (i16, i16), cx: f64, cy: f64, r: f64, n: usize) -> GdsElement {
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            (g(cx + r * a.cos()), g(cy + r * a.sin()))
        })
        .collect();
    poly(l, &pts)
}

/// A box of side `s` about `(cx, cy)` with `c` cut from each corner at 45°: the
/// octagon the manual allows for a bump pad.  `c = s·(1 − 1/√2)` makes it regular,
/// which is what IHP's bond pad pcell draws (`r·(1 − 1/(1+√2))` from a radius `r`).
fn octagon(l: (i16, i16), cx: f64, cy: f64, s: f64, c: f64) -> GdsElement {
    let (x0, y0) = (g(cx - s / 2.0), g(cy - s / 2.0));
    poly(l, &chamfered_all(x0, y0, x0 + s, y0 + s, c))
}

/// The regular octagon's corner cut for a box of side `s`, on the grid.
fn regular_cut(s: f64) -> f64 {
    g(s * (1.0 - 1.0 / SQRT_2))
}

/// Snap to the 0.005 µm grid.
fn g(v: f64) -> f64 {
    (v * 200.0).round() / 200.0
}

fn write(dir: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{dir}/{name}.gds.gz"), library("TOP", elems));
}

// --- Pad.a1: Max. Pad width 150.00 ---------------------------------------------------

/// Pad.a1 — "Max. Pad width 150.00", on the opening (Passiv AND dfpad).  A width is
/// the narrower dimension, so a long bar or an L of 100 arms is not wide.  (a) 150 square, clean; (b) 150.005 square, fires; (c) 150.005 × 100 and
/// (d) 100 × 150.005, clean; (e) 300 × 149.995 clean and (f) 300 × 150.005 fires; (g)
/// regular octagon of 150, clean, (h) of 150.005, fires; (i) diamond of width 149.9
/// clean, (j) 150.6 fires; (k) an L of 100 arms 300 long, clean; (l) a ring of 250 with
/// a 50 hole (100 walls), clean; (m) a ring of 400 with a 40 hole (180 walls), fires;
/// (n) 150.005 square as two overlapping boxes, fires once; (o) Passiv 200 under dfpad
/// 200 × 100, clean (the opening is what dfpad recognises); (p) Passiv 200 under dfpad
/// 160, fires; (q) Passiv 160 with no dfpad, clean (not a pad; IHP's maximal deck
/// reads Passiv alone here).  Pads on a 450 pitch.
fn pad_a1_h1(l: &L) {
    let p = |i: usize| (i % 6) as f64 * 450.0;
    let q = |i: usize| (i / 6) as f64 * 450.0;
    let mut e = vec![];
    let sq = |i: usize, s: f64| rect(l.passiv, p(i), q(i), p(i) + s, q(i) + s);
    e.extend(l.opening(sq(0, 150.0), 0.0));
    e.extend(l.opening(sq(1, 150.005), 0.0));
    e.extend(l.open_box(p(2), q(2), p(2) + 150.005, q(2) + 100.0));
    e.extend(l.open_box(p(3), q(3), p(3) + 100.0, q(3) + 150.005));
    e.extend(l.open_box(p(4), q(4), p(4) + 300.0, q(4) + 149.995));
    e.extend(l.open_box(p(5), q(5), p(5) + 300.0, q(5) + 150.005));
    let c = regular_cut(150.0);
    e.extend(l.opening(octagon(l.passiv, p(6) + 75.0, q(6) + 75.0, 150.0, c), 0.0));
    e.extend(l.opening(octagon(l.passiv, p(7) + 75.0, q(7) + 75.0, 150.005, c), 0.0));
    e.extend(l.opening(diamond(l.passiv, p(8) + 110.0, q(8) + 110.0, 106.0), 0.0));
    e.extend(l.opening(diamond(l.passiv, p(9) + 110.0, q(9) + 110.0, 106.5), 0.0));
    let (x, y) = (p(10), q(10));
    e.extend(l.opening(
        poly(
            l.passiv,
            &[
                (x, y),
                (x + 300.0, y),
                (x + 300.0, y + 100.0),
                (x + 100.0, y + 100.0),
                (x + 100.0, y + 300.0),
                (x, y + 300.0),
            ],
        ),
        0.0,
    ));
    let ring = |x: f64, y: f64, s: f64, h: f64| {
        let m = (s - h) / 2.0;
        poly(
            l.passiv,
            &[
                (x, y),
                (x + s, y),
                (x + s, y + s),
                (x, y + s),
                (x, y + m),
                (x + m, y + m),
                (x + m, y + m + h),
                (x + m + h, y + m + h),
                (x + m + h, y + m),
                (x, y + m),
            ],
        )
    };
    e.extend(l.opening(ring(p(11), q(11), 250.0, 50.0), 0.0));
    e.extend(l.opening(ring(p(12), q(12), 400.0, 40.0), 0.0));
    let (x, y) = (p(13), q(13));
    for r in [
        rect(l.passiv, x, y, x + 80.0, y + 150.005),
        rect(l.passiv, x + 70.0, y, x + 150.005, y + 150.005),
    ] {
        e.extend(l.opening(r, 0.0));
    }
    let (x, y) = (p(14), q(14));
    e.push(rect(l.passiv, x, y, x + 200.0, y + 200.0));
    e.push(rect(l.dfpad, x, y, x + 200.0, y + 100.0));
    e.push(rect(l.tm2, x, y, x + 200.0, y + 200.0));
    let (x, y) = (p(15), q(15));
    e.push(rect(l.passiv, x, y, x + 200.0, y + 200.0));
    e.push(rect(l.dfpad, x + 20.0, y + 20.0, x + 180.0, y + 180.0));
    e.push(rect(l.tm2, x, y, x + 200.0, y + 200.0));
    e.push(sq(16, 160.0));
    write(PAD, "Pad.a1.h1", e);
}

/// Pad.a1 — the tile lines.  150.005 squares at (10, 10) (across x = 20, 21, 40, 42,
/// 100), at (20, 200) (starting on the line), from x = −50.005 to 100 (ending on the
/// line), and at (1000, 1000); a 149.995 square at (40, 600) across the same lines,
/// clean.  Fires 4.
fn pad_a1_h2(l: &L) {
    let mut e = vec![];
    for (x, y, s) in [
        (10.0, 10.0, 150.005),
        (20.0, 200.0, 150.005),
        (-50.005, 400.0, 150.005),
        (1000.0, 1000.0, 150.005),
        (40.0, 600.0, 149.995),
    ] {
        e.extend(l.open_box(x, y, x + s, y + s));
    }
    write(PAD, "Pad.a1.h2", e);
}

// --- Pad.d: Min. Pad space to EdgeSeal 7.50 ------------------------------------------

/// Pad.d — "Min. Pad space to EdgeSeal 7.50", the opening to the seal ring's Activ.  A
/// seal bar (Activ AND EdgeSeal) (0, 0)-(4, 300); 30 µm openings (a) 7.5 to its right,
/// clean; (b) 7.495, fires; (c) beyond its top end, corner to corner dx = dy = 5.3
/// (7.495 euclidian, 5.3 on each axis), fires; (d) past its top-left corner, dx = 7.5,
/// dy = 0.5 (7.517), clean; (e) past its bottom-left corner, dx = dy = 7.0 (9.9), clean.  (f) Passiv 5.0 from the bar with dfpad from 7.5,
/// clean (the rule reads the opening); (g) the same with dfpad from 7.495, fires.  (h)
/// Activ alone (no EdgeSeal) 7.0 from an opening, clean; (i) EdgeSeal alone (no
/// Activ) 7.0 from an opening, clean by the deck's and IHP's reading of "EdgeSeal" as
/// the seal's Activ.  (j) a seal block with a 45° wall (x − y = 180, from (200, 20) to
/// (220, 40)) and an opening whose corner is 7.495 from it, the foot of the
/// perpendicular on the wall, fires; (k) the same at 7.502, clean.  Fires 4.
fn pad_d_h1(l: &L) {
    let mut e = l.seal_activ(0.0, 0.0, 4.0, 300.0);
    e.extend(l.open_box(11.5, 10.0, 41.5, 40.0));
    e.extend(l.open_box(11.495, 60.0, 41.495, 90.0));
    e.extend(l.open_box(9.3, 305.3, 39.3, 335.3));
    e.extend(l.open_box(-37.5, 300.5, -7.5, 330.5));
    e.extend(l.open_box(-37.0, -37.0, -7.0, -7.0));
    // (f), (g)
    e.push(rect(l.passiv, 9.0, 160.0, 39.0, 190.0));
    e.push(rect(l.dfpad, 11.5, 160.0, 39.0, 190.0));
    e.push(rect(l.tm2, 9.0, 160.0, 39.0, 190.0));
    e.push(rect(l.passiv, 9.0, 210.0, 39.0, 240.0));
    e.push(rect(l.dfpad, 11.495, 210.0, 39.0, 240.0));
    e.push(rect(l.tm2, 9.0, 210.0, 39.0, 240.0));
    // (h), (i)
    e.push(rect(l.activ, 100.0, 0.0, 104.0, 40.0));
    e.extend(l.open_box(111.0, 5.0, 141.0, 35.0));
    e.push(rect(l.seal, 100.0, 60.0, 104.0, 100.0));
    e.extend(l.open_box(111.0, 65.0, 141.0, 95.0));
    // (j), (k): the corner (px, py) sits on x − y = 180 − d·√2 with its foot
    // (px + d/√2, py − d/√2) on the wall: 10.6 → 7.495, 10.61 → 7.502.
    let block = |x: f64| {
        poly(
            l.passiv,
            &[
                (x, 0.0),
                (x + 40.0, 0.0),
                (x + 40.0, 40.0),
                (x + 20.0, 40.0),
                (x, 20.0),
            ],
        )
    };
    e.extend(on(&[l.activ, l.seal], &block(200.0)));
    e.extend(l.open_box(175.0, 35.6, 205.0, 65.6));
    e.extend(on(&[l.activ, l.seal], &block(300.0)));
    e.extend(l.open_box(275.0, 35.61, 305.0, 65.61));
    write(PAD, "Pad.d.h1", e);
}

/// Pad.d — a seal ring as drawn: a 3.5 wide Activ-and-EdgeSeal frame from (16.5,
/// 16.5) to (316.5, 316.5) with 21 µm 45° corners, so its inner wall is on the tile
/// line x = 20.  Inside, 60 µm openings: (a) 7.495 from the left wall (the gap holds
/// x = 21, and 28 of the 7 µm tiles), fires; (b) 7.5 above the bottom wall, clean; (c)
/// its corner 7.495 from the top-right 45° corner, fires.  (d) outside the ring, 7.495
/// below the bottom wall (the scribe side), fires.  Fires 3.
fn pad_d_h2(l: &L) {
    let mut e = l.seal_ring(16.5, 16.5, 316.5, 316.5, 3.5, 21.0);
    e.extend(l.open_box(27.495, 100.0, 87.495, 160.0));
    e.extend(l.open_box(120.0, 27.5, 180.0, 87.5));
    // Inner top-right chamfer: from (313 − k', 313) to (313, 313 − k') with
    // k' = 21 − 3.5·√2 = 16.05: x + y = 609.95.  Corner at 7.495 → x + y = 609.95 −
    // 7.495·√2 = 599.35.
    e.extend(l.open_box(239.675, 239.675, 299.675, 299.675));
    e.extend(l.open_box(120.0, -51.0, 180.0, 9.005));
    write(PAD, "Pad.d.h2", e);
}

/// Pad.d — 50 openings of 30 at 7.495 from one seal bar, flat (`h3`) and as an array
/// reference over the bar in TOP (`h4`).  Fires 50.
fn pad_d_h34(_l: &L) {}

// --- Pad.i: dfpad without TopMetal2 not allowed ---------------------------------------

/// Pad.i — "dfpad without TopMetal2 not allowed".  30 µm dfpad boxes on a 50 pitch:
/// (a) TopMetal2 the same box, clean; (b) TopMetal2 0.005 short on the right, fires;
/// (c) TopMetal2 as two abutting halves, clean; (d) TopMetal2 with a 2 × 2 hole under
/// the dfpad, fires; (e) TopMetal2 with its four corners cut 5 at 45°, fires (four
/// pieces); (f) dfpad with its corners cut, TopMetal2 square, clean; (g) no TopMetal2,
/// fires; (h) dfpad as two overlapping boxes under one TopMetal2, clean; (i) dfpad
/// (90, y)-(120, y+30) with TopMetal2 to x = 100 only, fires; (j) at (1000, 1000),
/// no TopMetal2, fires.  Fires 9 (e is four).
fn pad_i_h1(l: &L) {
    let at = |i: usize| ((i % 5) as f64 * 50.0, (i / 5) as f64 * 50.0);
    let mut e = vec![];
    let (x, y) = at(0);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    e.push(rect(l.tm2, x, y, x + 30.0, y + 30.0));
    let (x, y) = at(1);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    e.push(rect(l.tm2, x, y, x + 29.995, y + 30.0));
    let (x, y) = at(2);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    e.push(rect(l.tm2, x, y, x + 15.0, y + 30.0));
    e.push(rect(l.tm2, x + 15.0, y, x + 30.0, y + 30.0));
    let (x, y) = at(3);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    e.push(poly(
        l.tm2,
        &[
            (x, y),
            (x + 30.0, y),
            (x + 30.0, y + 30.0),
            (x, y + 30.0),
            (x, y + 14.0),
            (x + 14.0, y + 14.0),
            (x + 14.0, y + 16.0),
            (x + 16.0, y + 16.0),
            (x + 16.0, y + 14.0),
            (x, y + 14.0),
        ],
    ));
    let (x, y) = at(4);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    e.push(poly(l.tm2, &chamfered_all(x, y, x + 30.0, y + 30.0, 5.0)));
    let (x, y) = at(5);
    e.push(poly(l.dfpad, &chamfered_all(x, y, x + 30.0, y + 30.0, 5.0)));
    e.push(rect(l.tm2, x, y, x + 30.0, y + 30.0));
    let (x, y) = at(6);
    e.push(rect(l.dfpad, x, y, x + 30.0, y + 30.0));
    let (x, y) = at(7);
    e.push(rect(l.dfpad, x, y, x + 20.0, y + 30.0));
    e.push(rect(l.dfpad, x + 10.0, y, x + 30.0, y + 30.0));
    e.push(rect(l.tm2, x, y, x + 30.0, y + 30.0));
    let y = 100.0;
    e.push(rect(l.dfpad, 90.0, y, 120.0, y + 30.0));
    e.push(rect(l.tm2, 90.0, y, 100.0, y + 30.0));
    e.push(rect(l.dfpad, 1000.0, 1000.0, 1030.0, 1030.0));
    write(PAD, "Pad.i.h1", e);
}

/// Pad.i — 50 dfpad boxes whose TopMetal2 is 0.005 short, flat (`h2`) and as an array
/// reference (`h3`).  Fires 50.
fn pad_i_h23(_l: &L) {}

// --- Padb: solder bump pads -------------------------------------------------------------

/// Padb.a — "SBumpPad size 60.00", with Padb.f allowing an octagon or a circle.  The
/// size of an octagon or a circle is what it spans, 60 across; its diagonal flats are
/// the designer's.  Pads on a 140 pitch, TopMetal2 15 beyond: (a) 60 square, clean
/// (Padb.f fires, ignored); (b) 59.995 square and (c) 60.005 square, fire; (d) 60 ×
/// 59.995, fires; (e) regular octagon of 60 as IHP's pcell draws it (cut 17.575, which
/// puts its diagonal flats 59.9985 apart - no cut on the grid puts them at 60.000),
/// clean; (f) an octagon of 60 cut 10 (70.7 across the diagonals), clean by the
/// reading that the size is the span; (g) a 64-point circle of radius 30 as IHP's
/// pcell draws it (its walls 59.93 apart), clean; (h) regular octagon of 59.995 and (i) of 60.005, fire; (j)
/// circle of radius 29.995, fires; (k) 60 square as two overlapping boxes, clean; (l)
/// Passiv:sbump 60 over dfpad 60 × 50 (the pad is 60 × 50), fires.
fn padb_a_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 140.0 + 30.0, (i / 6) as f64 * 140.0 + 30.0);
    let mut e = vec![];
    let sq = |i: usize, w: f64, h: f64| {
        let (cx, cy) = at(i);
        rect(l.passiv, cx - 30.0, cy - 30.0, cx - 30.0 + w, cy - 30.0 + h)
    };
    e.extend(l.bump(sq(0, 60.0, 60.0), 15.0));
    e.extend(l.bump(sq(1, 59.995, 59.995), 15.0));
    e.extend(l.bump(sq(2, 60.005, 60.005), 15.0));
    e.extend(l.bump(sq(3, 60.0, 59.995), 15.0));
    let (cx, cy) = at(4);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 60.0, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(5);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 60.0, 10.0), 15.0));
    let (cx, cy) = at(6);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 30.0, 64), 15.0));
    let (cx, cy) = at(7);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 59.995, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(8);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 60.005, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(9);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 29.995, 64), 15.0));
    let (cx, cy) = at(10);
    for r in [
        rect(l.passiv, cx - 30.0, cy - 30.0, cx + 5.0, cy + 30.0),
        rect(l.passiv, cx - 5.0, cy - 30.0, cx + 30.0, cy + 30.0),
    ] {
        e.extend(on(&[l.passiv, l.sbump, l.dfpad, l.dfpad_sbump], &r));
    }
    e.push(rect(l.tm2, cx - 45.0, cy - 45.0, cx + 45.0, cy + 45.0));
    let (cx, cy) = at(11);
    e.extend(on(
        &[l.passiv, l.sbump],
        &rect(l.passiv, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0),
    ));
    e.extend(on(
        &[l.dfpad, l.dfpad_sbump],
        &rect(l.passiv, cx - 30.0, cy - 25.0, cx + 30.0, cy + 25.0),
    ));
    e.push(rect(l.tm2, cx - 45.0, cy - 45.0, cx + 45.0, cy + 45.0));
    write(PAD, "Padb.a.h1", e);
}

/// Padb.a — the tile lines: 59.995 squares from (5, 5) (across x = 20, 21, 40, 42),
/// from x = 40.005 to 100 (ending on the line) at y = 150, from x = 100 (starting on
/// it) at y = 300, and at (1000, 1000); a 60 square from (5, 450), clean.  Fires 4
/// pads.
fn padb_a_h2(l: &L) {
    let mut e = vec![];
    for (x, y, s) in [
        (5.0, 5.0, 59.995),
        (40.005, 150.0, 59.995),
        (100.0, 300.0, 59.995),
        (1000.0, 1000.0, 59.995),
        (5.0, 450.0, 60.0),
    ] {
        e.extend(l.bump(rect(l.passiv, x, y, x + s, y + s), 15.0));
    }
    write(PAD, "Padb.a.h2", e);
}

/// Padb.a — 10 × 5 pads of 59.995 at a 200 pitch, flat (`h3`) and as an array
/// reference (`h4`).  Fires 50 pads.
fn padb_a_h34(_l: &L) {}

/// Padb.b — "Min. SBumpPad space 70.00".  Pairs of 60 pads (squares; Padb.f ignored):
/// (a) 70 apart, clean; (b) 69.995, fires; (c) corner to corner dx = dy = 49.5 (70.004),
/// clean; (d) dx = dy = 49.49 (69.99), fires; (e) dx = 69, dy = 100 (121 euclidian, 69
/// on the x axis alone), clean; (f) two regular octagons offset (91.92, 91.92), their
/// diagonal flats 69.9945 apart, fires; (g) offset (91.925, 91.925), 70.0015, clean;
/// (h) two 64-point circles 129.995 apart on the x axis (69.995 vertex to vertex),
/// fires; (i) 130 apart, clean.  Fires 4.
fn padb_b_h1(l: &L) {
    let mut e = vec![];
    let sq = |x: f64, y: f64| rect(l.passiv, x, y, x + 60.0, y + 60.0);
    let pair = |e: &mut Vec<GdsElement>, x: f64, y: f64, dx: f64, dy: f64| {
        e.extend(l.bump(sq(x, y), 15.0));
        e.extend(l.bump(sq(x + 60.0 + dx, y + dy), 15.0));
    };
    pair(&mut e, 0.0, 0.0, 70.0, 0.0);
    pair(&mut e, 300.0, 0.0, 69.995, 0.0);
    pair(&mut e, 0.0, 300.0, 49.5, 109.5);
    pair(&mut e, 300.0, 300.0, 49.49, 109.49);
    pair(&mut e, 600.0, 300.0, 69.0, 160.0);
    let c = regular_cut(60.0);
    for (i, d) in [91.92, 91.925].iter().enumerate() {
        let (x, y) = (i as f64 * 300.0 + 30.0, 630.0);
        e.extend(l.bump(octagon(l.passiv, x, y, 60.0, c), 15.0));
        e.extend(l.bump(octagon(l.passiv, x + d, y + d, 60.0, c), 15.0));
    }
    for (i, d) in [129.995, 130.0].iter().enumerate() {
        let (x, y) = (i as f64 * 400.0 + 30.0, 900.0);
        e.extend(l.bump(ngon(l.passiv, x, y, 30.0, 64), 15.0));
        e.extend(l.bump(ngon(l.passiv, x + d, y, 30.0, 64), 15.0));
    }
    write(PAD, "Padb.b.h1", e);
}

/// Padb.b — the tile lines: 60 squares 69.995 apart with the gap (a) from 65 to
/// 134.995 (holding x = 100), (b) from 100 (starting on the line) at y = 200, (c) at
/// (1000, 1000).  Fires 3.
fn padb_b_h2(l: &L) {
    let mut e = vec![];
    for (x, y) in [(5.0, 5.0), (40.0, 200.0), (1000.0, 1000.0)] {
        e.extend(l.bump(rect(l.passiv, x, y, x + 60.0, y + 60.0), 15.0));
        let x2 = x + 60.0 + 69.995;
        e.extend(l.bump(rect(l.passiv, x2, y, x2 + 60.0, y + 60.0), 15.0));
    }
    write(PAD, "Padb.b.h2", e);
}

/// Padb.b — 10 × 5 pairs of 60 squares 69.995 apart on a 300 × 200 pitch, flat (`h3`)
/// and as an array reference (`h4`).  Fires 50.
fn padb_b_h34(l: &L) {
    let mut cell = l.bump(rect(l.passiv, 0.0, 0.0, 60.0, 60.0), 15.0);
    cell.extend(l.bump(rect(l.passiv, 129.995, 0.0, 189.995, 60.0), 15.0));
    let mut flat = vec![];
    for r in 0..5 {
        for c in 0..10 {
            flat.extend(shift(&cell, c as f64 * 300.0, r as f64 * 200.0));
        }
    }
    write(PAD, "Padb.b.h3", flat);
    let mut lib = ref_array(cell, 10, 5, 300.0);
    // ref_array uses one pitch; stretch the row vector to 200.
    if let GdsElement::GdsArrayRef(a) = &mut lib.structs[1].elems[0] {
        a.xy[2].y = 5 * 200_000;
    }
    write_gz(&format!("{PAD}/Padb.b.h4.gds.gz"), lib);
}

/// Padb.c — "Min. TopMetal2 (within dfpad) enclosure of SBumpPad 10.00".  60 pads
/// (squares unless said; Padb.f ignored) on a 200 pitch: (a) TopMetal2 10 beyond all
/// round, clean; (b) 9.995 on the right, fires; (c) 9.995 all round, fires; (d)
/// TopMetal2's top-right corner cut so it passes 9.995 (euclidian) from the pad's
/// corner, fires; (e) cut passing 10.002, clean; (f) regular octagon of 60 in a regular
/// octagon of 80 (10.002 across each diagonal on the grid), clean; (g) in an octagon of
/// 80 cut 23.44 (9.995 across the diagonals), fires; (h) 64-point circle of radius 30
/// in a circle of radius 40.02, clean; (i) the circle in a 79.99 square (9.995 from the
/// vertices on the axes), fires; (j) no TopMetal2, fires (and Pad.i); (k) TopMetal2 as
/// two abutting boxes, clean; (l) TopMetal2 plate with a 20 µm exit wire, clean.
fn padb_c_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 200.0 + 50.0, (i / 6) as f64 * 200.0 + 50.0);
    let mut e = vec![];
    let sq = |cx: f64, cy: f64| rect(l.passiv, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0);
    let pad = |e: &mut Vec<GdsElement>, s: &GdsElement| {
        e.extend(on(&[l.passiv, l.sbump, l.dfpad, l.dfpad_sbump], s));
    };
    let (cx, cy) = at(0);
    e.extend(l.bump(sq(cx, cy), 10.0));
    let (cx, cy) = at(1);
    pad(&mut e, &sq(cx, cy));
    e.push(rect(l.tm2, cx - 40.0, cy - 40.0, cx + 39.995, cy + 40.0));
    let (cx, cy) = at(2);
    e.extend(l.bump(sq(cx, cy), 9.995));
    let (cx, cy) = at(3);
    pad(&mut e, &sq(cx, cy));
    e.push(chamfered_tr(
        l.tm2,
        cx - 40.0,
        cy - 40.0,
        cx + 40.0,
        cy + 40.0,
        cx + cy + 60.0 + 14.135,
    ));
    let (cx, cy) = at(4);
    pad(&mut e, &sq(cx, cy));
    e.push(chamfered_tr(
        l.tm2,
        cx - 40.0,
        cy - 40.0,
        cx + 40.0,
        cy + 40.0,
        cx + cy + 60.0 + 14.145,
    ));
    let (cx, cy) = at(5);
    pad(&mut e, &octagon(l.passiv, cx, cy, 60.0, regular_cut(60.0)));
    e.push(octagon(l.tm2, cx, cy, 80.0, regular_cut(80.0)));
    let (cx, cy) = at(6);
    pad(&mut e, &octagon(l.passiv, cx, cy, 60.0, regular_cut(60.0)));
    e.push(octagon(l.tm2, cx, cy, 80.0, 23.44));
    let (cx, cy) = at(7);
    pad(&mut e, &ngon(l.passiv, cx, cy, 30.0, 64));
    e.push(ngon(l.tm2, cx, cy, 40.02, 64));
    let (cx, cy) = at(8);
    pad(&mut e, &ngon(l.passiv, cx, cy, 30.0, 64));
    e.push(rect(
        l.tm2,
        cx - 39.995,
        cy - 39.995,
        cx + 39.995,
        cy + 39.995,
    ));
    let (cx, cy) = at(9);
    pad(&mut e, &sq(cx, cy));
    let (cx, cy) = at(10);
    pad(&mut e, &sq(cx, cy));
    e.push(rect(l.tm2, cx - 40.0, cy - 40.0, cx, cy + 40.0));
    e.push(rect(l.tm2, cx, cy - 40.0, cx + 40.0, cy + 40.0));
    let (cx, cy) = at(11);
    pad(&mut e, &sq(cx, cy));
    e.push(poly(
        l.tm2,
        &[
            (cx - 40.0, cy - 40.0),
            (cx + 40.0, cy - 40.0),
            (cx + 40.0, cy - 10.0),
            (cx + 60.0, cy - 10.0),
            (cx + 60.0, cy + 10.0),
            (cx + 40.0, cy + 10.0),
            (cx + 40.0, cy + 40.0),
            (cx - 40.0, cy + 40.0),
        ],
    ));
    write(PAD, "Padb.c.h1", e);
}

/// Padb.c — the tile lines: (a) a pad from x = 30.005 to 90.005 with TopMetal2 to x =
/// 100 exactly (9.995), fires; (b) a pad from 30 to 90 with TopMetal2 to 100 (10.0),
/// clean; (c) as (a) at (1000, 1000).  Fires 2.
fn padb_c_h2(l: &L) {
    let mut e = vec![];
    for (x, y, short) in [
        (30.0, 30.0, true),
        (30.0, 200.0, false),
        (1000.0, 1000.0, true),
    ] {
        let x0 = if short { x + 0.005 } else { x };
        let s = rect(l.passiv, x0, y, x0 + 60.0, y + 60.0);
        e.extend(on(&[l.passiv, l.sbump, l.dfpad, l.dfpad_sbump], &s));
        e.push(rect(l.tm2, x - 10.0, y - 10.0, x + 70.0, y + 70.0));
    }
    write(PAD, "Padb.c.h2", e);
}

/// Padb.c — 10 × 5 pads whose TopMetal2 is 9.995 on the right, at a 200 pitch, flat
/// (`h3`) and as an array reference (`h4`).  Fires 50.
fn padb_c_h34(l: &L) {
    let s = rect(l.passiv, 10.0, 10.0, 70.0, 70.0);
    let mut cell = on(&[l.passiv, l.sbump, l.dfpad, l.dfpad_sbump], &s);
    cell.push(rect(l.tm2, 0.0, 0.0, 79.995, 80.0));
}

/// Padb.d — "Min. SBumpPad space to EdgeSeal 50.00", the pad to the EdgeSeal marker
/// itself.  An EdgeSeal bar (0, 0)-(4, 400); 60 squares (Padb.f ignored) (a) 50 to its
/// right, clean; (b) 49.995, fires; (c) past its top end, dx = dy = 35.35 (49.99
/// euclidian), fires; (d) past its top-left corner, dx = dy = 35.36 (50.006), clean; (e) an Activ bar with no
/// EdgeSeal 49.995 from a pad, clean; (f) an EdgeSeal frame (300, 300)-(500, 500) 4 wide
/// with a pad inside 49.995 from its left wall, fires; (g) a pad at (1000, 1000)
/// 49.995 from its own EdgeSeal bar, fires.  Fires 4.
fn padb_d_h1(l: &L) {
    let mut e = vec![rect(l.seal, 0.0, 0.0, 4.0, 400.0)];
    let sq = |x: f64, y: f64| rect(l.passiv, x, y, x + 60.0, y + 60.0);
    e.extend(l.bump(sq(54.0, 10.0), 15.0));
    e.extend(l.bump(sq(53.995, 140.0), 15.0));
    e.extend(l.bump(sq(39.35, 435.35), 15.0));
    e.extend(l.bump(sq(-95.36, 435.36), 15.0));
    e.push(rect(l.activ, 200.0, 0.0, 204.0, 100.0));
    e.extend(l.bump(sq(253.995, 20.0), 15.0));
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
    e.push(frame(300.0, 300.0, 500.0, 500.0, 4.0));
    e.extend(l.bump(sq(353.995, 370.0), 15.0));
    e.push(rect(l.seal, 1000.0, 1000.0, 1004.0, 1100.0));
    e.extend(l.bump(sq(1053.995, 1020.0), 15.0));
    write(PAD, "Padb.d.h1", e);
}

/// Padb.f — "Allowed passivation opening shape: Octagon, Circle" for a bump pad.  60
/// pads on a 130 pitch, TopMetal2 15 beyond (Padb.a and Padb.b ignored): (a) regular
/// octagon (IHP's pcell's), clean; (b) an octagon cut 10, clean; (c) an octagon cut 10
/// at two corners and 15 at the other two, clean (eight sides at 45°); (d) a
/// 64-point circle, clean; (e) a 128-point circle, clean; (f) a square, fires; (g) a
/// diamond, fires; (h) a hexagon, fires; (i) a D (the circle with a flat cut at x =
/// 20), fires; (j) an ellipse 60 × 50, fires; (k) a 16-gon, fires (too coarse for a
/// circle; IHP's deck wants 64 points); (l) a circle with a 10 hole, fires.  Fires 7.
fn padb_f_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 130.0 + 40.0, (i / 6) as f64 * 130.0 + 40.0);
    let mut e = vec![];
    let (cx, cy) = at(0);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 60.0, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(1);
    e.extend(l.bump(octagon(l.passiv, cx, cy, 60.0, 10.0), 15.0));
    let (cx, cy) = at(2);
    let (a, b) = (cx - 30.0, cy - 30.0);
    e.extend(l.bump(
        poly(
            l.passiv,
            &[
                (a + 10.0, b),
                (a + 45.0, b),
                (a + 60.0, b + 15.0),
                (a + 60.0, b + 50.0),
                (a + 50.0, b + 60.0),
                (a + 15.0, b + 60.0),
                (a, b + 45.0),
                (a, b + 10.0),
            ],
        ),
        15.0,
    ));
    let (cx, cy) = at(3);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 30.0, 64), 15.0));
    let (cx, cy) = at(4);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 30.0, 128), 15.0));
    let (cx, cy) = at(5);
    e.extend(l.bump(
        rect(l.passiv, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0),
        15.0,
    ));
    let (cx, cy) = at(6);
    e.extend(l.bump(diamond(l.passiv, cx, cy, 30.0), 15.0));
    let (cx, cy) = at(7);
    e.extend(l.bump(
        poly(
            l.passiv,
            &[
                (cx - 15.0, cy - 25.98),
                (cx + 15.0, cy - 25.98),
                (cx + 30.0, cy),
                (cx + 15.0, cy + 25.98),
                (cx - 15.0, cy + 25.98),
                (cx - 30.0, cy),
            ],
        ),
        15.0,
    ));
    let (cx, cy) = at(8);
    let d: Vec<(f64, f64)> = (0..64)
        .map(|k| {
            let a = TAU * k as f64 / 64.0;
            (g(cx + (30.0 * a.cos()).min(20.0)), g(cy + 30.0 * a.sin()))
        })
        .collect();
    e.extend(l.bump(poly(l.passiv, &d), 15.0));
    let (cx, cy) = at(9);
    let el: Vec<(f64, f64)> = (0..64)
        .map(|k| {
            let a = TAU * k as f64 / 64.0;
            (g(cx + 30.0 * a.cos()), g(cy + 25.0 * a.sin()))
        })
        .collect();
    e.extend(l.bump(poly(l.passiv, &el), 15.0));
    let (cx, cy) = at(10);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 30.0, 16), 15.0));
    let (cx, cy) = at(11);
    let mut ring: Vec<(f64, f64)> = (0..64)
        .map(|k| {
            let a = TAU * k as f64 / 64.0;
            (g(cx + 30.0 * a.cos()), g(cy + 30.0 * a.sin()))
        })
        .collect();
    ring.push(ring[0]);
    let hole: Vec<(f64, f64)> = (0..64)
        .rev()
        .map(|k| {
            let a = TAU * k as f64 / 64.0;
            (g(cx + 5.0 * a.cos()), g(cy + 5.0 * a.sin()))
        })
        .collect();
    ring.push(hole[hole.len() - 1]);
    ring.extend(hole);
    e.extend(l.bump(poly(l.passiv, &ring), 15.0));
    write(PAD, "Padb.f.h1", e);
}

// --- Padc: copper pillar pads ------------------------------------------------------------

/// Padc.a — "CuPillarPad size: Table 6.1", which lists 35, 40 and 45 µm openings (Padc.f
/// wants a circle).  Pads on a 130 pitch, TopMetal2 15 beyond (Padc.f ignored on the
/// squares): (a) 35 square, clean; (b) 34.995 and (c) 35.005 squares, fire; (d) a
/// 64-point circle of radius 17.5, clean; (e) 40 square and (f) 45 square, clean by
/// the table; (g) circle of radius 20 and (h) of 22.5, clean by the table; (i) circle
/// of radius 17.495, fires.  Fires 3.
fn padc_a_h1(l: &L) {
    let at = |i: usize| ((i % 5) as f64 * 130.0 + 30.0, (i / 5) as f64 * 130.0 + 30.0);
    let mut e = vec![];
    let sq = |i: usize, s: f64| {
        let (cx, cy) = at(i);
        rect(l.passiv, cx - 17.5, cy - 17.5, cx - 17.5 + s, cy - 17.5 + s)
    };
    e.extend(l.pillar_pad(sq(0, 35.0), 15.0));
    e.extend(l.pillar_pad(sq(1, 34.995), 15.0));
    e.extend(l.pillar_pad(sq(2, 35.005), 15.0));
    let (cx, cy) = at(3);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 64), 15.0));
    e.extend(l.pillar_pad(sq(4, 40.0), 15.0));
    e.extend(l.pillar_pad(sq(5, 45.0), 15.0));
    let (cx, cy) = at(6);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 20.0, 64), 15.0));
    let (cx, cy) = at(7);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 22.5, 64), 15.0));
    let (cx, cy) = at(8);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.495, 64), 15.0));
    write(PAD, "Padc.a.h1", e);
}

/// Padc.b — "Min. CuPillarPad space: Table 6.1": 40 between 35 or 40 µm openings, 50
/// between 45 µm ones.  Pairs of squares (Padc.f ignored): (a) 35s 40 apart, clean;
/// (b) 39.995, fires; (c) corner to corner dx = dy = 28.28 (39.99), fires; (d) dx = dy =
/// 28.29 (40.008), clean; (e) 40s 40 apart, clean; (f) 40s 39.995 apart, fires; (g) 45s
/// 50 apart, clean; (h) 45s 49.995 apart, fires by the table; (i) 45s 45 apart, fires
/// by the table.  Fires 5.
fn padc_b_h1(l: &L) {
    let mut e = vec![];
    let pair = |e: &mut Vec<GdsElement>, x: f64, y: f64, s: f64, dx: f64, dy: f64| {
        e.extend(l.pillar_pad(rect(l.passiv, x, y, x + s, y + s), 15.0));
        let x2 = x + s + dx;
        e.extend(l.pillar_pad(rect(l.passiv, x2, y + dy, x2 + s, y + dy + s), 15.0));
    };
    pair(&mut e, 0.0, 0.0, 35.0, 40.0, 0.0);
    pair(&mut e, 200.0, 0.0, 35.0, 39.995, 0.0);
    pair(&mut e, 0.0, 200.0, 35.0, 28.28, 63.28);
    pair(&mut e, 200.0, 200.0, 35.0, 28.29, 63.29);
    pair(&mut e, 0.0, 400.0, 40.0, 40.0, 0.0);
    pair(&mut e, 200.0, 400.0, 40.0, 39.995, 0.0);
    pair(&mut e, 0.0, 600.0, 45.0, 50.0, 0.0);
    pair(&mut e, 200.0, 600.0, 45.0, 49.995, 0.0);
    pair(&mut e, 400.0, 600.0, 45.0, 45.0, 0.0);
    write(PAD, "Padc.b.h1", e);
}

/// Padc.c — "Min. TopMetal2 (within dfpad) enclosure of CuPillarPad 7.5".  35 squares
/// (Padc.f ignored) on a 150 pitch: (a) TopMetal2 7.5 beyond, clean; (b) 7.495 on the
/// right, fires; (c) top-right corner cut passing 7.495 (euclidian) from the pad's
/// corner, fires; (d) cut passing 7.502, clean; (e) no TopMetal2, fires (and Pad.i);
/// (f) a 64-point circle of radius 17.5 in a circle of radius 25.02, clean.  Fires 3.
fn padc_c_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 150.0 + 40.0, (i / 6) as f64 * 150.0 + 40.0);
    let mut e = vec![];
    let sq = |cx: f64, cy: f64| rect(l.passiv, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5);
    let pad = |e: &mut Vec<GdsElement>, s: &GdsElement| {
        e.extend(on(&[l.passiv, l.pillar, l.dfpad, l.dfpad_pillar], s));
    };
    let (cx, cy) = at(0);
    e.extend(l.pillar_pad(sq(cx, cy), 7.5));
    let (cx, cy) = at(1);
    pad(&mut e, &sq(cx, cy));
    e.push(rect(l.tm2, cx - 25.0, cy - 25.0, cx + 24.995, cy + 25.0));
    // 2·7.5 − t with (2·7.5 − t)/√2 = 7.495 → 10.6; 7.502 → 10.61.
    let (cx, cy) = at(2);
    pad(&mut e, &sq(cx, cy));
    e.push(chamfered_tr(
        l.tm2,
        cx - 25.0,
        cy - 25.0,
        cx + 25.0,
        cy + 25.0,
        cx + cy + 35.0 + 10.6,
    ));
    let (cx, cy) = at(3);
    pad(&mut e, &sq(cx, cy));
    e.push(chamfered_tr(
        l.tm2,
        cx - 25.0,
        cy - 25.0,
        cx + 25.0,
        cy + 25.0,
        cx + cy + 35.0 + 10.61,
    ));
    let (cx, cy) = at(4);
    pad(&mut e, &sq(cx, cy));
    let (cx, cy) = at(5);
    pad(&mut e, &ngon(l.passiv, cx, cy, 17.5, 64));
    e.push(ngon(l.tm2, cx, cy, 25.02, 64));
    write(PAD, "Padc.c.h1", e);
}

/// Padc.d — "Min. CuPillarPad space to EdgeSeal 30.00", read like Pad.d against the
/// seal's Activ.  A seal bar (0, 0)-(4, 200); 35 squares (Padc.f ignored) (a) 30 to
/// its right, clean; (b) 29.995, fires; (c) past its top end, dx = dy = 21.21 (29.995
/// euclidian), fires; (d) past its top-left corner, dx = dy = 21.22 (30.01), clean; (e)
/// past its bottom-left corner, dx = 29, dy = 40 (49), clean; (f) an Activ bar with no
/// EdgeSeal 29.995 from a pad, clean; (g) a seal block with a 45° wall (x − y = 270,
/// from (300, 30) to (330, 60)) and a pad whose corner is 29.995 from it, the foot of
/// the perpendicular on the wall, fires.  Fires 3.
fn padc_d_h1(l: &L) {
    let mut e = l.seal_activ(0.0, 0.0, 4.0, 200.0);
    let sq = |x: f64, y: f64| rect(l.passiv, x, y, x + 35.0, y + 35.0);
    e.extend(l.pillar_pad(sq(34.0, 10.0), 15.0));
    e.extend(l.pillar_pad(sq(33.995, 100.0), 15.0));
    e.extend(l.pillar_pad(sq(25.21, 221.21), 15.0));
    e.extend(l.pillar_pad(sq(-56.22, 221.22), 15.0));
    e.extend(l.pillar_pad(sq(-64.0, -75.0), 15.0));
    e.push(rect(l.activ, 100.0, 0.0, 104.0, 100.0));
    e.extend(l.pillar_pad(sq(133.995, 20.0), 15.0));
    e.extend(on(
        &[l.activ, l.seal],
        &poly(
            l.passiv,
            &[
                (300.0, 0.0),
                (360.0, 0.0),
                (360.0, 60.0),
                (330.0, 60.0),
                (300.0, 30.0),
            ],
        ),
    ));
    // The corner (290, 62.42) sits on x − y = 270 − 29.995·√2 = 227.58; its foot
    // (311.21, 41.21) is on the wall.
    e.extend(l.pillar_pad(sq(255.0, 62.42), 15.0));
    write(PAD, "Padc.d.h1", e);
}

/// Padc.f — "Allowed passivation opening shape: Circle" for a pillar pad.  35 pads on
/// a 100 pitch (Padc.a ignored): (a) 64-point circle, clean; (b) 128-point circle,
/// clean; (c) regular octagon, fires; (d) square, fires; (e) 16-gon, fires; (f) a D
/// (the circle cut flat at x = 12), fires.  Fires 4.
fn padc_f_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 100.0 + 30.0, 30.0);
    let mut e = vec![];
    let (cx, cy) = at(0);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 64), 15.0));
    let (cx, cy) = at(1);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 128), 15.0));
    let (cx, cy) = at(2);
    e.extend(l.pillar_pad(octagon(l.passiv, cx, cy, 35.0, regular_cut(35.0)), 15.0));
    let (cx, cy) = at(3);
    e.extend(l.pillar_pad(
        rect(l.passiv, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5),
        15.0,
    ));
    let (cx, cy) = at(4);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 16), 15.0));
    let (cx, cy) = at(5);
    let d: Vec<(f64, f64)> = (0..64)
        .map(|k| {
            let a = TAU * k as f64 / 64.0;
            (g(cx + (17.5 * a.cos()).min(12.0)), g(cy + 17.5 * a.sin()))
        })
        .collect();
    e.extend(l.pillar_pad(poly(l.passiv, &d), 15.0));
    write(PAD, "Padc.f.h1", e);
}

// --- Passiv --------------------------------------------------------------------------------

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

/// Pas.a — the tile lines: 2.095 bars (a) vertical from x = 19.95 to 22.045 (across 20
/// and 21); (b) from 39.995 to 42.09 (across 40 and 42); (c) from 100 to 102.095
/// (starting on the line); (d) horizontal 2.095 tall from x = 15 to 25 at y = 20 (its
/// walls across the line); (e) ending exactly on x = 20 (17.905 to 20).  A 2.1 bar from
/// 59.95 to 62.05 across 60 and 63, clean.  Fires 5.
fn pas_a_h2(l: &L) {
    let p = l.passiv;
    let e = vec![
        rect(p, 19.95, 0.0, 22.045, 10.0),
        rect(p, 39.995, 0.0, 42.09, 10.0),
        rect(p, 100.0, 0.0, 102.095, 10.0),
        rect(p, 15.0, 20.0, 25.0, 22.095),
        rect(p, 17.905, 40.0, 20.0, 50.0),
        rect(p, 59.95, 0.0, 62.05, 10.0),
    ];
    write(PAS, "Pas.a.h2", e);
}

/// Pas.a — 10 × 5 bars of 2.095 × 10 at a 15 pitch, flat (`h3`) and as an array
/// reference (`h4`).  Fires 50 bars.
fn pas_a_h34(_l: &L) {}

/// Pas.b — "Min. Passiv space or notch 3.50".  5 µm squares: (a) 3.5 apart, clean;
/// (b) 3.495, fires; (c) corner to corner dx = dy = 2.47 (3.493), fires; (d) dx = dy =
/// 2.48 (3.507), clean; (e) dx = 3.0, dy = 6 (6.7 euclidian, 3.0 on the x axis alone),
/// clean; (f) two 45° strips of d = 2, 8.94 apart in y (3.4931 across), fires; (g) 8.95
/// apart (3.5002), clean; (h) a U whose arms are 3.49 apart (a notch), fires; (i) a
/// comb of three 3 µm fingers with 3.495 slots, fires twice; (j) a 3.495 slot 6 deep
/// into a 15 plate, fires; (k) a ring of 15 with a 3.49 hole, fires; (l) a ring of 15
/// with a 10 hole holding a 3 square (3.5 each side), clean; (m) with a 9.99 hole
/// (3.495 each side), fires; (n) a square drawn as two overlapping boxes 3.495 from a
/// square, fires once.
fn pas_b_h1(l: &L) {
    let p = l.passiv;
    let sq = |x: f64, y: f64| rect(p, x, y, x + 5.0, y + 5.0);
    let mut e = vec![
        sq(0.0, 0.0),
        sq(8.5, 0.0),
        sq(20.0, 0.0),
        sq(28.495, 0.0),
        sq(40.0, 0.0),
        sq(47.47, 7.47),
        sq(60.0, 0.0),
        sq(67.48, 7.48),
        sq(80.0, 0.0),
        sq(88.0, 11.0),
        strip45(p, 100.0, 0.0, 10.0, 2.0),
        strip45(p, 100.0, 8.94, 10.0, 2.0),
        strip45(p, 120.0, 0.0, 10.0, 2.0),
        strip45(p, 120.0, 8.95, 10.0, 2.0),
    ];
    // (h) U: 10 wide, arms 3.2525 (3.495 between), 10 tall, base 3.
    let u = |x: f64, y: f64, gap: f64| {
        let arm = (10.0 - gap) / 2.0;
        poly(
            p,
            &[
                (x, y),
                (x + 10.0, y),
                (x + 10.0, y + 10.0),
                (x + 10.0 - arm, y + 10.0),
                (x + 10.0 - arm, y + 3.0),
                (x + arm, y + 3.0),
                (x + arm, y + 10.0),
                (x, y + 10.0),
            ],
        )
    };
    e.push(u(140.0, 0.0, 3.5)); // clean
    e.push(u(155.0, 0.0, 3.49));
    // (i) comb: base 16 × 3, three fingers 3 wide, slots 3.495.
    let (x, y) = (170.0, 0.0);
    e.push(poly(
        p,
        &[
            (x, y),
            (x + 15.99, y),
            (x + 15.99, y + 13.0),
            (x + 12.99, y + 13.0),
            (x + 12.99, y + 3.0),
            (x + 9.495, y + 3.0),
            (x + 9.495, y + 13.0),
            (x + 6.495, y + 13.0),
            (x + 6.495, y + 3.0),
            (x + 3.0, y + 3.0),
            (x + 3.0, y + 13.0),
            (x, y + 13.0),
        ],
    ));
    // (j) slot 3.495 wide, 6 deep, from the top of a 15 plate.
    let (x, y) = (200.0, 0.0);
    e.push(poly(
        p,
        &[
            (x, y),
            (x + 15.0, y),
            (x + 15.0, y + 15.0),
            (x + 9.25, y + 15.0),
            (x + 9.25, y + 9.0),
            (x + 5.755, y + 9.0),
            (x + 5.755, y + 15.0),
            (x, y + 15.0),
        ],
    ));
    let ring = |x: f64, y: f64, s: f64, h: f64| {
        let m = (s - h) / 2.0;
        poly(
            p,
            &[
                (x, y),
                (x + s, y),
                (x + s, y + s),
                (x, y + s),
                (x, y + m),
                (x + m, y + m),
                (x + m, y + m + h),
                (x + m + h, y + m + h),
                (x + m + h, y + m),
                (x, y + m),
            ],
        )
    };
    e.push(ring(230.0, 0.0, 15.0, 3.49));
    e.push(ring(250.0, 0.0, 15.0, 10.0));
    e.push(rect(p, 256.0, 6.0, 259.0, 9.0));
    e.push(ring(270.0, 0.0, 15.0, 9.99));
    e.push(rect(p, 276.0, 6.0, 279.0, 9.0));
    e.push(rect(p, 300.0, 0.0, 303.0, 5.0));
    e.push(rect(p, 302.0, 0.0, 305.0, 5.0));
    e.push(sq(308.495, 0.0));
    write(PAS, "Pas.b.h1", e);
}

/// Pas.b — the tile lines: 5 squares 3.495 apart with the gap (a) from 18 to 21.495
/// (across 20 and 21), (b) from 38.5 to 41.995 (across 40), (c) from 20 exactly, (d)
/// ending on 42 exactly, (e) from 98.5 to 101.995 (across 100), (f) at (1000, 1000);
/// (g) a corner-to-corner pair dx = dy = 2.47 whose corners sit on (60, 60) and
/// (62.47, 62.47).  Fires 7.
fn pas_b_h2(l: &L) {
    let p = l.passiv;
    let sq = |x: f64, y: f64| rect(p, x, y, x + 5.0, y + 5.0);
    let e = vec![
        sq(13.0, 0.0),
        sq(21.495, 0.0),
        sq(33.5, 0.0),
        sq(41.995, 0.0),
        sq(15.0, 10.0),
        sq(23.495, 10.0),
        sq(33.505, 10.0),
        sq(42.0, 10.0),
        sq(93.5, 0.0),
        sq(101.995, 0.0),
        sq(1000.0, 1000.0),
        sq(1008.495, 1000.0),
        sq(55.0, 55.0),
        sq(62.47, 62.47),
    ];
    write(PAS, "Pas.b.h2", e);
}

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

// --- MIM -------------------------------------------------------------------------------------

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

/// MIM.a — the tile lines: 1.135 bars (a) from x = 19.95 to 21.085 (across 20 and 21);
/// (b) from 39.995 to 41.13 (across 40); (c) from 100 to 101.135; (d) horizontal 1.135
/// tall from x = 15 to 25 at y = 20; (e) ending on x = 42 (40.865 to 42).  A 1.14 bar
/// from 6.5 to 7.64 (across 7), clean.  Fires 5.
fn mim_a_h2(l: &L) {
    let m = l.mim;
    let e = vec![
        rect(m, 19.95, 0.0, 21.085, 10.0),
        rect(m, 39.995, 0.0, 41.13, 10.0),
        rect(m, 100.0, 0.0, 101.135, 10.0),
        rect(m, 15.0, 20.0, 25.0, 21.135),
        rect(m, 40.865, 30.0, 42.0, 40.0),
        rect(m, 6.5, 0.0, 7.64, 10.0),
        rect(l.m5, -5.0, -5.0, 110.0, 50.0),
    ];
    write(MIM, "MIM.a.h2", e);
}

/// MIM.a — 10 × 5 bars of 1.135 × 10 at a 15 pitch on one Metal5 plate in TOP, flat
/// (`h3`) and as an array reference (`h4`).  Fires 50 bars.
fn mim_a_h34(_l: &L) {}

/// MIM.b — "Min. MIM space 0.60".  2 µm squares, no vias (MIM.h ignored), on one
/// Metal5 plate: (a) 0.6 apart, clean; (b) 0.595, fires; (c) corner to corner dx = dy =
/// 0.42 (0.594), fires; (d) dx = dy = 0.425 (0.601), clean; (e) dx = 0.5, dy = 3 (3.04
/// euclidian), clean; (f) two 45° strips of d = 1, 2.84 apart in y (0.594 across),
/// fires; (g) 2.85 apart (0.601), clean; (h) a U whose arms are 0.59 apart, fires;
/// (i) a comb of three 2 fingers with 0.595 slots, fires twice; (j) a ring of 6 with a
/// 0.59 hole, fires; (k) a ring of 6 with a 3.2 hole holding a 2 square (0.6 each
/// side), clean; (l) with a 3.19 hole (0.595 each side), fires; (m) a square as two
/// overlapping boxes 0.595 from a square, fires once.
fn mim_b_h1(l: &L) {
    let m = l.mim;
    let sq = |x: f64, y: f64| rect(m, x, y, x + 2.0, y + 2.0);
    let mut e = vec![
        sq(0.0, 0.0),
        sq(2.6, 0.0),
        sq(10.0, 0.0),
        sq(12.595, 0.0),
        sq(20.0, 0.0),
        sq(22.42, 2.42),
        sq(30.0, 0.0),
        sq(32.425, 2.425),
        sq(40.0, 0.0),
        sq(42.5, 5.0),
        strip45(m, 50.0, 0.0, 5.0, 1.0),
        strip45(m, 50.0, 2.84, 5.0, 1.0),
        strip45(m, 60.0, 0.0, 5.0, 1.0),
        strip45(m, 60.0, 2.85, 5.0, 1.0),
    ];
    let u = |x: f64, y: f64, gap: f64| {
        let arm = (5.0 - gap) / 2.0;
        poly(
            m,
            &[
                (x, y),
                (x + 5.0, y),
                (x + 5.0, y + 5.0),
                (x + 5.0 - arm, y + 5.0),
                (x + 5.0 - arm, y + 2.0),
                (x + arm, y + 2.0),
                (x + arm, y + 5.0),
                (x, y + 5.0),
            ],
        )
    };
    e.push(u(70.0, 0.0, 0.6));
    e.push(u(80.0, 0.0, 0.59));
    let (x, y) = (90.0, 0.0);
    e.push(poly(
        m,
        &[
            (x, y),
            (x + 7.19, y),
            (x + 7.19, y + 6.0),
            (x + 5.19, y + 6.0),
            (x + 5.19, y + 2.0),
            (x + 4.595, y + 2.0),
            (x + 4.595, y + 6.0),
            (x + 2.595, y + 6.0),
            (x + 2.595, y + 2.0),
            (x + 2.0, y + 2.0),
            (x + 2.0, y + 6.0),
            (x, y + 6.0),
        ],
    ));
    let ring = |x: f64, y: f64, s: f64, h: f64| {
        let mm = (s - h) / 2.0;
        poly(
            m,
            &[
                (x, y),
                (x + s, y),
                (x + s, y + s),
                (x, y + s),
                (x, y + mm),
                (x + mm, y + mm),
                (x + mm, y + mm + h),
                (x + mm + h, y + mm + h),
                (x + mm + h, y + mm),
                (x, y + mm),
            ],
        )
    };
    e.push(ring(100.0, 0.0, 6.0, 0.59));
    e.push(ring(110.0, 0.0, 6.0, 3.2));
    e.push(sq(112.0, 2.0));
    e.push(ring(120.0, 0.0, 6.0, 3.19));
    e.push(sq(122.0, 2.0));
    e.push(rect(m, 130.0, 0.0, 131.5, 2.0));
    e.push(rect(m, 131.0, 0.0, 132.5, 2.0));
    e.push(sq(133.095, 0.0));
    e.push(rect(l.m5, -5.0, -5.0, 140.0, 15.0));
    write(MIM, "MIM.b.h1", e);
}

/// MIM.b — the tile lines: 2 squares 0.595 apart with the gap (a) from 19.7 to
/// 20.295 (across 20), (b) from 20.7 to 21.295 (across 21), (c) from 40 exactly, (d)
/// ending on 42, (e) from 99.7 to 100.295, (f) at (1000, 1000); (g) a corner-to-corner
/// pair dx = dy = 0.42 whose corners sit on (60, 60) and (60.42, 60.42).  Fires 7.
fn mim_b_h2(l: &L) {
    let m = l.mim;
    let sq = |x: f64, y: f64| rect(m, x, y, x + 2.0, y + 2.0);
    let e = vec![
        sq(17.7, 0.0),
        sq(20.295, 0.0),
        sq(18.7, 5.0),
        sq(21.295, 5.0),
        sq(38.0, 0.0),
        sq(40.595, 0.0),
        sq(39.405, 5.0),
        sq(42.0, 5.0),
        sq(97.7, 0.0),
        sq(100.295, 0.0),
        sq(1000.0, 1000.0),
        sq(1002.595, 1000.0),
        sq(58.0, 58.0),
        sq(60.42, 60.42),
        rect(l.m5, -5.0, -5.0, 110.0, 70.0),
        rect(l.m5, 995.0, 995.0, 1010.0, 1010.0),
    ];
    write(MIM, "MIM.b.h2", e);
}

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

/// MIM.c — the tile lines: 3 plates with Metal5 0.595 short on one side, the short
/// margin (a) from 19.405 to 20 (ending on the line), (b) from 20 to 20.595 (starting
/// on it), (c) from 39.7 to 40.295 (across 40), (d) from 99.7 to 100.295, (e) at
/// (1000, 1000); (f) Metal5 from 20 to 20.6 beyond a plate, clean.  Fires 5.
fn mim_c_h2(l: &L) {
    let mut e = vec![];
    let plate = |e: &mut Vec<GdsElement>, x: f64, y: f64, right: f64| {
        e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
        e.push(rect(l.tv1, x + 0.4, y + 0.4, x + 0.82, y + 0.82));
        e.push(rect(l.m5, x - 0.6, y - 0.6, x + 3.0 + right, y + 3.6));
    };
    plate(&mut e, 16.405, 0.0, 0.595);
    plate(&mut e, 17.0, 10.0, 0.595);
    plate(&mut e, 36.7, 0.0, 0.595);
    plate(&mut e, 96.7, 0.0, 0.595);
    plate(&mut e, 1000.0, 1000.0, 0.595);
    plate(&mut e, 17.0, 20.0, 0.6);
    write(MIM, "MIM.c.h2", e);
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

/// MIM.e — "Min. TopMetal1 space to MIM 0.60".  3 µm plates on Metal5 0.7 beyond with
/// a via 0.5 in, on a 12 pitch: (a) a TopMetal1 wire 0.6 to the right, clean; (b)
/// 0.595, fires; (c) a wire past the plate's corner dx = dy = 0.42 (0.594), fires; (d)
/// dx = dy = 0.425, clean; (e) the cap's top plate: TopMetal1 over the via, inside the
/// MIM, clean; (f) the top plate with a 1 µm exit wire crossing the MIM's wall, clean;
/// (g) the exit wire turning to run along the MIM's wall 0.595 outside it, fires; (h)
/// a TopMetal1 45° wall passing 0.594 from the plate's corner, fires.  Fires 4.
fn mim_e_h1(l: &L) {
    let at = |i: usize| ((i % 8) as f64 * 12.0, 0.0);
    let mut e = vec![];
    let plate = |e: &mut Vec<GdsElement>, x: f64, y: f64| {
        e.push(rect(l.mim, x, y, x + 3.0, y + 3.0));
        e.push(rect(l.m5, x - 0.7, y - 0.7, x + 3.7, y + 3.7));
        e.push(rect(l.tv1, x + 0.5, y + 0.5, x + 0.92, y + 0.92));
    };
    let (x, y) = at(0);
    plate(&mut e, x, y);
    e.push(rect(l.tm1, x + 3.6, y, x + 5.6, y + 3.0));
    let (x, y) = at(1);
    plate(&mut e, x, y);
    e.push(rect(l.tm1, x + 3.595, y, x + 5.595, y + 3.0));
    let (x, y) = at(2);
    plate(&mut e, x, y);
    e.push(rect(l.tm1, x + 3.42, y + 3.42, x + 5.42, y + 5.42));
    let (x, y) = at(3);
    plate(&mut e, x, y);
    e.push(rect(l.tm1, x + 3.425, y + 3.425, x + 5.425, y + 5.425));
    let (x, y) = at(4);
    plate(&mut e, x, y);
    e.push(rect(l.tm1, x + 0.4, y + 0.4, x + 2.6, y + 2.6));
    let (x, y) = at(5);
    plate(&mut e, x, y);
    e.push(poly(
        l.tm1,
        &[
            (x + 0.4, y + 0.4),
            (x + 2.6, y + 0.4),
            (x + 2.6, y + 1.0),
            (x + 6.0, y + 1.0),
            (x + 6.0, y + 2.0),
            (x + 2.6, y + 2.0),
            (x + 2.6, y + 2.6),
            (x + 0.4, y + 2.6),
        ],
    ));
    let (x, y) = at(6);
    plate(&mut e, x, y);
    e.push(poly(
        l.tm1,
        &[
            (x + 0.4, y + 0.4),
            (x + 2.6, y + 0.4),
            (x + 2.6, y + 1.0),
            (x + 5.0, y + 1.0),
            (x + 5.0, y + 4.595),
            (x - 1.0, y + 4.595),
            (x - 1.0, y + 3.595),
            (x + 4.0, y + 3.595),
            (x + 4.0, y + 2.0),
            (x + 2.6, y + 2.0),
            (x + 2.6, y + 2.6),
            (x + 0.4, y + 2.6),
        ],
    ));
    // (h): a triangle whose 45° wall X − Y = x + 3.84 passes 0.84/√2 = 0.594 from the
    // plate's bottom-right corner, the foot of the perpendicular on the wall.
    let (x, y) = at(7);
    plate(&mut e, x, y);
    e.push(poly(
        l.tm1,
        &[(x + 2.84, y - 1.0), (x + 8.0, y - 1.0), (x + 8.0, y + 4.16)],
    ));
    write(MIM, "MIM.e.h1", e);
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

/// MIM.gR — "Max. recommended total MIM area per chip 174800": 8 × 4 caps of 74 × 74
/// (5476 each, 175232 in all) placed by one array reference.  Fires once.
fn mim_gr_h1(l: &L) {
    let cell = l.cap(0.0, 0.0, 74.0, 74.0, 0.5);
    let lib = ref_array(cell, 8, 4, 80.0);
    write_gz(&format!("{MIM}/MIM.gR.h1.gds.gz"), lib);
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
