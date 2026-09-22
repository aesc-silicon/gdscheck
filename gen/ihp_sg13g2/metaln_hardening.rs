// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the Metal(n=2-5) decks: section 5.17 (Mn.a-Mn.k) and section
//! 5.18 (MnFil.*) of the SG13G2 layout rules, one rule set on four layers.  Every
//! layout is `tests/data/ihp-sg13g2/metaln/M<rule>.h<k>.gds.gz` and carries the same
//! geometry on Metal2, Metal3, Metal4 and Metal5 at the same place (Via(n-1) and
//! Metal(n-1) below): a Metal(n) deck reads its own layers of it and nothing else, so
//! one file serves the four decks, and the layers can be read against one another.

use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, mixed_notch_pattern, notch_pattern, poly,
    rect, ref_array, strap, strip45, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

/// What the four layers' patterns of one layout are gathered into: the elements of a
/// flat layout, or the cell of an array layout and its pitch.
enum Out {
    Flat(Vec<GdsElement>),
    Array(Vec<GdsElement>, f64),
}

type Gathered = std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, Out>>>;

/// The layers of one Metal(n) deck.
struct L {
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

const DIR: &str = "tests/data/ihp-sg13g2/metaln";

impl L {
    fn new(pdk: &PdkConfig, n: i32, out: Gathered) -> Self {
        L {
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
        let mut out = self.out.borrow_mut();
        match out
            .entry(format!("M{rule}"))
            .or_insert_with(|| Out::Flat(Vec::new()))
        {
            Out::Flat(v) | Out::Array(v, _) => v.append(&mut elems),
        }
    }

    /// `<rule>.h<k>` flat and `<rule>.h<k+1>` as a `GdsArrayRef`: 10 × 5 copies of `cell`
    /// at `pitch`.  Hierarchy must not change the answer: fifty violations either way.
    fn arrays(&self, rule: &str, k: u32, mut cell: Vec<GdsElement>, pitch: f64) {
        self.write(&format!("{rule}.h{k}"), flat_array(&cell, 10, 5, pitch));
        let mut out = self.out.borrow_mut();
        match out
            .entry(format!("M{rule}.h{}", k + 1))
            .or_insert_with(|| Out::Array(Vec::new(), pitch))
        {
            Out::Flat(v) | Out::Array(v, _) => v.append(&mut cell),
        }
    }

    /// Writes every gathered layout.
    fn flush(out: Gathered) {
        std::fs::create_dir_all(DIR).expect("failed to create output directory");
        for (name, o) in out.borrow_mut().iter_mut() {
            let path = format!("{DIR}/{name}.gds.gz");
            match o {
                Out::Flat(v) => write_gz(&path, library("TOP", std::mem::take(v))),
                Out::Array(v, pitch) => {
                    write_gz(&path, ref_array(std::mem::take(v), 10, 5, *pitch))
                }
            }
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

/// An L route 0.2 wide from `(x0, y0)` with its bend chamfered at 45°: the outer chamfer
/// cuts `k` off the corner, the inner one runs parallel 0.285 in (0.2015 wide).  Outer
/// wall `k·√2` long, inner `(k − 0.115)·√2`.
fn chamfered_l(l: (i16, i16), x0: f64, y0: f64, k: f64) -> GdsElement {
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

pub fn generate(pdk: &PdkConfig) {
    let out: Gathered = Default::default();
    for n in 2..6 {
        let l = L::new(pdk, n, out.clone());
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
    L::flush(out);
}

// --- Mn.a: min. Metal(n) width 0.20 ---

fn mn_a(l: &L) {
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
    // case sets Mn.g aside.  A chamfered box and an L with a chamfered inner corner are
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
            ), // clean: L, inner corner chamfered, arms 1.0 wide
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
    // running across 20/21 and 40/42; an L cornered on x = 20 with its vertical arm
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

    // h5/h6 — fifty 0.195 × 1 bars, flat and as an array; pitch 3 keeps them apart.
    l.arrays(".a", 5, vec![rect(m, 0.2, 0.2, 0.395, 1.2)], 3.0);

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

fn mn_b(l: &L) {
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

    // h6/h7 — fifty 0.205 pairs, flat and as an array.
    l.arrays(
        ".b",
        6,
        vec![rect(m, 0.2, 0.2, 1.0, 1.2), rect(m, 1.205, 0.2, 2.005, 1.2)],
        3.0,
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

fn mn_c(l: &L) {
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

    // h6/h7 — fifty vias on a line's edge, flat and as an array.
    l.arrays(
        ".c",
        6,
        vec![rect(m, 0.2, 0.2, 1.2, 0.4), l.via(0.6, 0.2)],
        3.0,
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

fn mn_c1(l: &L) {
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

    // h2 — corners (note 1: "at least one side must be treated as an endcap").  An L
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

    // h6/h7 — fifty line-end vias with 0.045 endcaps, flat and as an array.
    l.arrays(
        ".c1",
        6,
        vec![rect(m, 0.2, 0.2, 2.2, 0.4), l.via(1.965, 0.205)],
        3.0,
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

fn mn_d(l: &L) {
    let m = l.m;

    // h1 — the bound.  0.4 × 0.36 = 0.144 clean; 0.4 × 0.355 = 0.142 fires (both ways);
    // an L of 0.144 clean and of 0.142 fires; a diamond of 0.1458 clean, 0.1405 fires.
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

    // h4/h5 — fifty 0.14 bars, flat and as an array.
    l.arrays(".d", 4, vec![rect(m, 0.2, 0.2, 0.4, 0.9)], 3.0);

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

fn mn_e(l: &L) {
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

    // h4 — shapes.  An L pad with 0.5 arms and a narrow line 0.235 from its vertical arm
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

    // h9/h10 — fifty wide/narrow pairs at 0.235, flat and as an array.
    l.arrays(
        ".e",
        9,
        vec![rect(m, 0.2, 0.2, 0.7, 2.2), rect(m, 0.935, 0.2, 1.135, 2.2)],
        3.0,
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

fn mn_f(l: &L) {
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
    // two 10 × 12 plates 0.595 apart are clean; an L plate with 12 arms and a narrow line
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

    // h4/h5 — fifty plate pairs at 0.595, flat and as an array, pitch 30.
    l.arrays(
        ".f",
        4,
        vec![
            rect(m, 0.0, 0.0, 12.0, 12.0),
            rect(m, 12.595, 0.0, 24.595, 12.0),
        ],
        30.0,
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

fn mn_g(l: &L) {
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

    // h3 — an L with a 45° bend.  The 0.2 route's bend is chamfered on both corners so the
    // bend is a 0.2015-wide 45° segment: outer/inner walls 0.707/0.544 fire; 0.566/0.403
    // are clean under the settled reading; 0.424/0.262 are clean.  A 0.2333 diamond's
    // walls are 0.2333 long: clean.
    l.write(
        ".g.h3",
        vec![
            chamfered_l(m, 2.0, 2.0, 0.5),
            chamfered_l(m, 8.0, 2.0, 0.4),
            chamfered_l(m, 14.0, 2.0, 0.3),
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

    // h5/h6 — fifty 0.2333 strips, flat and as an array.
    l.arrays(".g", 5, vec![strip45(m, 0.4, 0.2, 1.0, 0.165)], 3.0);

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

fn mn_i(l: &L) {
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

    // h5/h6 — fifty strip pairs, flat and as an array.
    l.arrays(".i", 5, pair(0.5, 0.3), 3.5);

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

fn mn_density(l: &L) {
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

fn mnfil(l: &L) {
    let (f, m, t) = (l.fil, l.m, l.trans);

    // Fil.a1.h1 — min. filler width 1.0: 1.0 × 3 clean, 0.995 fires (both ways); a diamond
    // 1.004 across clean, 0.99 fires; an L with 1.0 arms clean; two overlapping 0.6 boxes
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
    // 5.005 × 5.0 filler is 5.0 wide (clean); a 3 × 20 bar, an L with 3-wide arms in an
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

    // Fil.b.h3/h4 — fifty 0.415 pairs, flat and as an array.
    l.arrays("Fil.b", 3, pair(2.2, 0.2), 6.0);

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

    // Fil.c.h3/h4 — fifty filler/metal pairs at 0.415, flat and as an array.
    l.arrays("Fil.c", 3, pair(2.2, 0.2), 6.0);

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
