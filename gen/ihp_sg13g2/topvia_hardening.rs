// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Hardening patterns (ci/hardening/SPEC.md) for the `topvia1` and `topvia2` decks:
// layouts drawn from the manual's sections 5.21 (TV1.a-TV1.d) and 5.24 (TV2.a-TV2.d)
// alone, one fixture per theme, `TV<n>.<rule>.h<k>`.  The two decks share every rule
// with different values, so one function draws each theme for both.  Section 6.10
// applies to the via rules: they are not checked within EdgeSeal.  TopVia is 90° only
// (section 3.1), so the 45° geometry is the enclosing metal's.  Each function's
// comment states the geometry and what the manual says about it; the expected answers
// are in the `topvia1`/`topvia2` tables of tests/ihp-sg13g2.rs and the reasoning in
// ci/hardening/reports/ihp-sg13g2/topvia.md.

use crate::helpers::{chamfered_tr, flat_array, layer, library, poly, rect, ref_array, write_gz};
use gds21::{GdsElement, GdsLibrary};
use gdscheck::pdk::PdkConfig;

const SQRT2: f64 = std::f64::consts::SQRT_2;

/// Layers and values of one deck.
struct L {
    n: i32,
    dir: String,
    via: (i16, i16),
    /// The metal under the via (Metal5 / TopMetal1), TV<n>.c's.
    below: (i16, i16),
    /// The metal over the via (TopMetal1 / TopMetal2), TV<n>.d's.
    above: (i16, i16),
    seal: (i16, i16),
    /// TV<n>.a, the via's one width.
    w: f64,
    /// TV<n>.b.
    s: f64,
    /// TV<n>.c.
    c: f64,
    /// TV<n>.d.
    d: f64,
}

impl L {
    fn new(pdk: &PdkConfig, n: i32) -> Self {
        let (below, above, w, s, c, d) = if n == 1 {
            ("Metal5", "TopMetal1", 0.42, 0.42, 0.10, 0.42)
        } else {
            ("TopMetal1", "TopMetal2", 0.90, 1.06, 0.50, 0.50)
        };
        L {
            n,
            dir: format!("tests/data/ihp-sg13g2/topvia{n}"),
            via: layer(pdk, &format!("TopVia{n}")),
            below: layer(pdk, below),
            above: layer(pdk, above),
            seal: layer(pdk, "EdgeSeal"),
            w,
            s,
            c,
            d,
        }
    }

    fn write(&self, name: &str, elems: Vec<GdsElement>) {
        write_gz(
            &format!("{}/{name}.gds.gz", self.dir),
            library("TOP", elems),
        );
    }

    fn write_lib(&self, name: &str, lib: GdsLibrary) {
        write_gz(&format!("{}/{name}.gds.gz", self.dir), lib);
    }

    /// `<name>.h<k>` flat and `<name>.h<k+1>` as an array reference: 10 × 5 copies of
    /// `cell` at `pitch`.  Hierarchy must not change the answer.
    fn arrays(&self, name: &str, k: u32, cell: Vec<GdsElement>, pitch: f64) {
        self.write(&format!("{name}.h{k}"), flat_array(&cell, 10, 5, pitch));
        self.write_lib(&format!("{name}.h{}", k + 1), ref_array(cell, 10, 5, pitch));
    }

    /// A via of the one legal size with its lower-left corner at `(x, y)`.
    fn sq(&self, x: f64, y: f64) -> GdsElement {
        rect(self.via, x, y, g(x + self.w), g(y + self.w))
    }

    /// A via at `(x, y)` in a box of `m` with the four margins.
    #[allow(clippy::too_many_arguments)]
    fn enc(
        &self,
        m: (i16, i16),
        x: f64,
        y: f64,
        ml: f64,
        mr: f64,
        mb: f64,
        mt: f64,
    ) -> Vec<GdsElement> {
        let (x1, y1) = (g(x + self.w), g(y + self.w));
        vec![
            self.sq(x, y),
            rect(m, g(x - ml), g(y - mb), g(x1 + mr), g(y1 + mt)),
        ]
    }

    /// A via at `(x, y)` in a box of `m` with margin `mm` all round.
    fn enc_m(&self, m: (i16, i16), x: f64, y: f64, mm: f64) -> Vec<GdsElement> {
        self.enc(m, x, y, mm, mm, mm, mm)
    }

    /// The other metal of an enclosure rule (the one not under test), as a plate that
    /// keeps its own rule quiet.
    fn other(&self, m: (i16, i16)) -> (i16, i16) {
        if m == self.below {
            self.above
        } else {
            self.below
        }
    }
}

/// A frame on `l`: the box less the hole, as four overlapping walls.
#[allow(clippy::too_many_arguments)]
fn ring(
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
        rect(l, x0, y0, hx0, y1),
        rect(l, hx1, y0, x1, y1),
        rect(l, x0, y0, x1, hy0),
        rect(l, x0, hy1, x1, y1),
    ]
}

/// `v` rounded up to the 0.005 µm grid.
fn up(v: f64) -> f64 {
    ((v - 1e-9) / 0.005).ceil() * 0.005
}

/// `v` rounded down to the 0.005 µm grid.
fn down(v: f64) -> f64 {
    ((v + 1e-9) / 0.005).floor() * 0.005
}

/// A computed coordinate snapped to the grid.
fn g(v: f64) -> f64 {
    (v / 0.005).round() * 0.005
}

/// x positions of a shape `w` wide: well inside a tile (ending at 10), ending on x = 20,
/// straddling 20, starting on 20, straddling 21, ending on 40 and straddling 42.
fn tile_xs(w: f64) -> [f64; 7] {
    let half = down(w / 2.0);
    [
        g(10.0 - w),
        g(20.0 - w),
        g(20.0 - half),
        20.0,
        g(21.0 - half),
        g(40.0 - w),
        g(42.0 - half),
    ]
}

/// (left end, right start) of a gap `gap` wide: well inside a tile (ending at 9), ending
/// on x = 20, straddling 20, starting on 20, straddling 21, ending on 40, straddling 42.
fn tile_gaps(gap: f64) -> [(f64, f64); 7] {
    let half = down(gap / 2.0);
    [
        (g(9.0 - gap), 9.0),
        (g(20.0 - gap), 20.0),
        (g(20.0 - half), g(20.0 - half + gap)),
        (20.0, g(20.0 + gap)),
        (g(21.0 - half), g(21.0 - half + gap)),
        (g(40.0 - gap), 40.0),
        (g(42.0 - half), g(42.0 - half + gap)),
    ]
}

pub fn generate(pdk: &PdkConfig) {
    for n in 1..3 {
        let l = L::new(pdk, n);
        std::fs::create_dir_all(&l.dir).expect("failed to create output directory");
        tv_a_h(&l);
        tv_b_h(&l);
        tv_enc_h(&l, 'c');
        tv_enc_h(&l, 'd');
    }
}

// --- TV<n>.a: min. and max. TopVia width (0.42 / 0.90) ---

fn tv_a_h(l: &L) {
    let (v, w) = (l.via, l.w);
    let wm = g(w - 0.005);
    let wp = g(w + 0.005);
    let p = format!("TV{}.a", l.n);
    let x = |i: i32| 2.0 + 2.5 * i as f64;

    // h1 — the bound.  A `w` square is the via; one step short or long in x or in y
    // fires on that pair of walls; a `w − 0.005` square on both pairs; a `w × 2w` bar
    // is over the maximum along its length; a 0.005 sliver, a 300 µm bar crossing every
    // tile line, and a short square at (1000, 1000).
    l.write(
        &format!("{p}.h1"),
        vec![
            l.sq(x(0), 2.0),                                         // clean
            rect(v, x(1), 2.0, g(x(1) + wm), g(2.0 + w)),            // TV.a (x short)
            rect(v, x(2), 2.0, g(x(2) + w), g(2.0 + wm)),            // TV.a (y short)
            rect(v, x(3), 2.0, g(x(3) + wp), g(2.0 + w)),            // TV.a (x long)
            rect(v, x(4), 2.0, g(x(4) + w), g(2.0 + wp)),            // TV.a (y long)
            rect(v, x(5), 2.0, g(x(5) + wm), g(2.0 + wm)),           // TV.a (both)
            rect(v, x(6), 2.0, g(x(6) + w), g(2.0 + 2.0 * w)),       // TV.a: a bar
            rect(v, x(7), 2.0, x(7) + 0.005, g(2.0 + w)),            // TV.a: a sliver
            rect(v, 2.0, 6.0, 302.0, g(6.0 + w)),                    // TV.a: 300 µm
            rect(v, 1000.0, 1000.0, g(1000.0 + wm), g(1000.0 + wm)), // TV.a: far
        ],
    );

    // h2 — shapes that merge.  The rule reads the union: a `w` square drawn as two
    // abutting halves, two overlapping boxes, twice on top of itself, as a 2 × 2 grid
    // of quarters or clockwise is one legal via.  Two `w` squares abutting are one
    // `w × 2w` bar (over the maximum, and no pair for TV.b); two sharing a corner are
    // one shape whose extent is 2w; an L of three `w` quarters of a 2w square, a `w`
    // square with a 0.005 notch in its top wall and one with a 0.005 bump are not `w`
    // squares.
    let h = g(w / 2.0);
    let x = |i: i32| 2.5 + 3.0 * i as f64;
    let (x0, x1, x2, x3, x4, x5, x6, x7, x8, x9) =
        (x(0), x(1), x(2), x(3), x(4), x(5), x(6), x(7), x(8), x(9));
    l.write(
        &format!("{p}.h2"),
        vec![
            rect(v, x0, 2.0, g(x0 + h), g(2.0 + w)),
            rect(v, g(x0 + h), 2.0, g(x0 + w), g(2.0 + w)), // clean: halves
            rect(v, x1, 2.0, g(x1 + 0.6 * w), g(2.0 + w)),
            rect(v, g(x1 + 0.4 * w), 2.0, g(x1 + w), g(2.0 + w)), // clean: overlap
            l.sq(x2, 2.0),
            l.sq(x2, 2.0), // clean: twice
            rect(v, x3, 2.0, g(x3 + h), g(2.0 + h)),
            rect(v, g(x3 + h), 2.0, g(x3 + w), g(2.0 + h)),
            rect(v, x3, g(2.0 + h), g(x3 + h), g(2.0 + w)),
            rect(v, g(x3 + h), g(2.0 + h), g(x3 + w), g(2.0 + w)), // clean: quarters
            poly(
                v,
                &[
                    (x4, 2.0),
                    (x4, g(2.0 + w)),
                    (g(x4 + w), g(2.0 + w)),
                    (g(x4 + w), 2.0),
                ],
            ), // clean: clockwise
            l.sq(x5, 2.0),
            l.sq(g(x5 + w), 2.0), // TV.a: a bar of two
            l.sq(x6, 2.0),
            l.sq(g(x6 + w), g(2.0 + w)), // TV.a: corner to corner
            poly(
                v,
                &[
                    (x7, 2.0),
                    (g(x7 + 2.0 * w), 2.0),
                    (g(x7 + 2.0 * w), g(2.0 + w)),
                    (g(x7 + w), g(2.0 + w)),
                    (g(x7 + w), g(2.0 + 2.0 * w)),
                    (x7, g(2.0 + 2.0 * w)),
                ],
            ), // TV.a: an L
            poly(
                v,
                &[
                    (x8, 2.0),
                    (g(x8 + w), 2.0),
                    (g(x8 + w), g(2.0 + w)),
                    (g(x8 + 0.3), g(2.0 + w)),
                    (g(x8 + 0.3), g(2.0 + w - 0.005)),
                    (g(x8 + 0.1), g(2.0 + w - 0.005)),
                    (g(x8 + 0.1), g(2.0 + w)),
                    (x8, g(2.0 + w)),
                ],
            ), // TV.a: a notch
            poly(
                v,
                &[
                    (x9, 2.0),
                    (g(x9 + w), 2.0),
                    (g(x9 + w), g(2.0 + w)),
                    (g(x9 + 0.3), g(2.0 + w)),
                    (g(x9 + 0.3), g(2.0 + w + 0.005)),
                    (g(x9 + 0.1), g(2.0 + w + 0.005)),
                    (g(x9 + 0.1), g(2.0 + w)),
                    (x9, g(2.0 + w)),
                ],
            ), // TV.a: a bump
        ],
    );

    // h3 — tile lines.  `w − 0.005` squares well inside a tile, ending on x = 20,
    // straddling 20, starting on 20, straddling 21, ending on 40, straddling 42, and one
    // straddling y = 20; a `w` square straddling x = 20 is clean.
    let mut e = vec![];
    for (i, xs) in tile_xs(wm).iter().enumerate() {
        let y = 2.0 + 3.0 * i as f64;
        e.push(rect(v, *xs, y, g(xs + wm), g(y + wm)));
    }
    let half = down(wm / 2.0);
    e.push(rect(
        v,
        10.0,
        g(20.0 - half),
        g(10.0 + wm),
        g(20.0 - half + wm),
    ));
    e.push(l.sq(g(20.0 - down(w / 2.0)), 25.0)); // clean
    l.write(&format!("{p}.h3"), e);

    // h4/h5 — fifty `w − 0.005` squares, flat and as an array reference.
    l.arrays(
        &p,
        4,
        vec![rect(v, 0.2, 0.2, g(0.2 + wm), g(0.2 + wm))],
        2.0,
    );

    // h6 — the seal (section 6.10).  Under an EdgeSeal plate (1, 1)-(9, 9) a short
    // square is not checked; a `w` square inside the seal with its right wall on the
    // seal's edge x = 9 is wholly within it; a `w` square straddling that edge leaves a
    // `w/2 × w` piece outside, which is not a `w` square; a short square outside with
    // its left wall on the edge fires; a short square in the hole of an EdgeSeal frame
    // (12, 1)-(18, 7) is the design's and fires.
    l.write(
        &format!("{p}.h6"),
        vec![
            rect(l.seal, 1.0, 1.0, 9.0, 9.0),
            rect(v, 2.0, 2.0, g(2.0 + wm), g(2.0 + wm)), // clean: in the seal
            l.sq(g(9.0 - w), 7.0),                       // clean: within, on the edge
            l.sq(g(9.0 - h), 5.0),                       // TV.a: cut by the edge
            rect(v, 9.0, 2.0, g(9.0 + wm), g(2.0 + wm)), // TV.a: outside, on the edge
            rect(l.seal, 12.0, 1.0, 18.0, 2.0),
            rect(l.seal, 12.0, 6.0, 18.0, 7.0),
            rect(l.seal, 12.0, 1.0, 13.0, 7.0),
            rect(l.seal, 17.0, 1.0, 18.0, 7.0),
            rect(v, 15.0, 4.0, g(15.0 + wm), g(4.0 + wm)), // TV.a: in the frame's hole
        ],
    );
}

// --- TV<n>.b: min. TopVia space (0.42 / 1.06) ---

fn tv_b_h(l: &L) {
    let (v, w, s) = (l.via, l.w, l.s);
    let sm = g(s - 0.005);
    let p = format!("TV{}.b", l.n);
    // Corner-to-corner offsets: the smallest on-grid d with d·√2 ≥ s is clean, one step
    // less fires.
    let dc = up(s / SQRT2);
    let df = g(dc - 0.005);
    let x = |i: i32| 2.0 + 5.0 * i as f64;

    // h1 — the bound and both metrics.  Two `w` squares `s` apart are clean, `s − 0.005`
    // fires in x and in y; a diagonal pair dc/dc apart (euclidian ≥ s, both axis
    // offsets under s) is clean, df/df fires; a corner-on pair (the second starts at
    // the first's top, corners `s − 0.005` apart along x) fires, at `s` it is clean;
    // three in a row are two pairs; a 3 × 3 at `s` is clean, at `s − 0.005` twelve pairs
    // (the diagonals are (s − 0.005)·√2 apart); a pair at (1000, 1000).
    let mut e = vec![
        l.sq(x(0), 2.0),
        l.sq(g(x(0) + w + s), 2.0), // clean
        l.sq(x(1), 2.0),
        l.sq(g(x(1) + w + sm), 2.0), // TV.b (x)
        l.sq(x(2), 2.0),
        l.sq(x(2), g(2.0 + w + sm)), // TV.b (y)
        l.sq(x(3), 2.0),
        l.sq(g(x(3) + w + dc), g(2.0 + w + dc)), // clean diagonal
        l.sq(x(4), 2.0),
        l.sq(g(x(4) + w + df), g(2.0 + w + df)), // TV.b diagonal
        l.sq(x(5), 2.0),
        l.sq(g(x(5) + w + sm), g(2.0 + w)), // TV.b corner-on
        l.sq(x(6), 2.0),
        l.sq(g(x(6) + w + s), g(2.0 + w)), // clean corner-on
        l.sq(x(7), 2.0),
        l.sq(g(x(7) + w + sm), 2.0),
        l.sq(g(x(7) + 2.0 * (w + sm)), 2.0), // TV.b × 2: a row of three
        l.sq(1000.0, 1000.0),
        l.sq(g(1000.0 + w + sm), 1000.0), // TV.b: far
    ];
    for i in 0..3 {
        for j in 0..3 {
            e.push(l.sq(g(2.0 + (w + s) * i as f64), g(8.0 + (w + s) * j as f64))); // clean 3 × 3
            e.push(l.sq(g(12.0 + (w + sm) * i as f64), g(8.0 + (w + sm) * j as f64))); // TV.b × 12
        }
    }
    l.write(&format!("{p}.h1"), e);

    // h2 — shapes.  A `w × 2w` bar (TV.a's) `s − 0.005` from a square is still a TopVia
    // pair; two abutting squares are one shape, no pair; a square in the inner corner
    // of an L-shaped via `s − 0.005` from both arms; two `w` bars 300 µm long `s − 0.005`
    // apart are one pair.
    l.write(
        &format!("{p}.h2"),
        vec![
            rect(v, x(0), 2.0, g(x(0) + w), g(2.0 + 2.0 * w)),
            l.sq(g(x(0) + w + sm), 2.0), // TV.b: bar to square
            l.sq(x(1), 2.0),
            l.sq(g(x(1) + w), 2.0), // no pair: abutting
            poly(
                v,
                &[
                    (x(2), 2.0),
                    (g(x(2) + 3.0 * w), 2.0),
                    (g(x(2) + 3.0 * w), g(2.0 + w)),
                    (g(x(2) + w), g(2.0 + w)),
                    (g(x(2) + w), g(2.0 + 3.0 * w)),
                    (x(2), g(2.0 + 3.0 * w)),
                ],
            ),
            l.sq(g(x(2) + w + sm), g(2.0 + w + sm)), // TV.b: in the L's corner
            rect(v, 2.0, 8.0, 302.0, g(8.0 + w)),
            rect(v, 2.0, g(8.0 + w + sm), 302.0, g(8.0 + 2.0 * w + sm)), // TV.b: 300 µm
        ],
    );

    // h3 — tile lines.  `s − 0.005` gaps well inside a tile, ending on x = 20, straddling
    // 20, starting on 20, straddling 21, ending on 40, straddling 42, and one straddling
    // y = 20; a gap of `s` straddling x = 20 is clean.
    let mut e = vec![];
    for (i, (a, b)) in tile_gaps(sm).iter().enumerate() {
        let y = 2.0 + 4.0 * i as f64;
        e.push(l.sq(g(a - w), y));
        e.push(l.sq(*b, y));
    }
    let half = down(sm / 2.0);
    e.push(l.sq(10.0, g(20.0 - half - w)));
    e.push(l.sq(10.0, g(20.0 - half + sm)));
    let half = down(s / 2.0);
    e.push(l.sq(g(20.0 - half - w), 34.0));
    e.push(l.sq(g(20.0 - half + s), 34.0)); // clean
    l.write(&format!("{p}.h3"), e);

    // h4/h5 — fifty `s − 0.005` pairs, flat and as an array reference.
    l.arrays(&p, 4, vec![l.sq(0.2, 0.2), l.sq(g(0.2 + w + sm), 0.2)], 5.0);

    // h6 — the seal (section 6.10).  Under an EdgeSeal plate (1, 1)-(9, 9) a pair
    // `s − 0.005` apart is not checked; a via inside the seal against its edge x = 9 and
    // one `s − 0.005` outside it; a pair outside at (12, 2) fires.
    l.write(
        &format!("{p}.h6"),
        vec![
            rect(l.seal, 1.0, 1.0, 9.0, 9.0),
            l.sq(2.0, 2.0),
            l.sq(g(2.0 + w + sm), 2.0), // in the seal
            l.sq(g(9.0 - w), 5.0),
            l.sq(g(9.0 + sm), 5.0), // across the seal's edge
            l.sq(12.0, 2.0),
            l.sq(g(12.0 + w + sm), 2.0), // TV.b
        ],
    );
}

// --- TV<n>.c / TV<n>.d: min. metal enclosure of TopVia (0.10, 0.42 / 0.50, 0.50) ---

/// `rule` is 'c' (the metal below) or 'd' (the metal above).
fn tv_enc_h(l: &L, rule: char) {
    let (m, e) = if rule == 'c' {
        (l.below, l.c)
    } else {
        (l.above, l.d)
    };
    let (v, w, o) = (l.via, l.w, l.other(m));
    let em = g(e - 0.005);
    let p = format!("TV{}.{rule}", l.n);
    let x = |i: i32| 2.0 + 4.0 * i as f64;

    // h1 — the bound.  Margins of `e` all round are clean; `e − 0.005` on the right only
    // fires, on all four sides (one via), right and top (a corner), left and right (two
    // walls); a 0.005 margin fires; a via edge on the metal's edge is an enclosure of 0;
    // a via 0.05 past the metal's edge has less than none; a via with no metal at all
    // has no enclosure; a via at (1000, 1000) with `e − 0.005` on the right.
    let mut el = vec![];
    el.extend(l.enc_m(m, x(0), 2.0, e)); // clean
    el.extend(l.enc(m, x(1), 2.0, e, em, e, e)); // TV: right
    el.extend(l.enc_m(m, x(2), 2.0, em)); // TV: all round
    el.extend(l.enc(m, x(3), 2.0, e, em, e, em)); // TV: right and top
    el.extend(l.enc(m, x(4), 2.0, em, em, e, e)); // TV × 2: left and right
    el.extend(l.enc(m, x(5), 2.0, e, 0.005, e, e)); // TV: 0.005
    el.extend(l.enc(m, x(6), 2.0, e, 0.0, e, e)); // TV: 0
    el.extend(l.enc(m, x(7), 2.0, e, -0.05, e, e)); // TV: past the edge
    el.push(l.sq(x(8), 2.0)); // TV: no metal
    el.extend(l.enc(m, 1000.0, 1000.0, e, em, e, e)); // TV: far
    el.push(rect(o, 0.0, 0.0, 40.0, 5.0));
    el.push(rect(o, 998.0, 998.0, 1003.0, 1003.0));
    l.write(&format!("{p}.h1"), el);

    // h2 — 45° geometry of the metal.  The metal box's top-right corner is cut along
    // x + y = k.  (a) The cut passes `e − 0.005` (euclidian) from the via's corner: the
    // metal above the via's top wall is (e − 0.005)·√2 ≥ e everywhere, clean by the
    // settled projection reading.  (b) The cut passes through the point e/2 above the
    // via's top-right corner: the metal above the top wall's right end is e/2, fires.
    // (c) The same as a long 45° wall (box margins 3), fires.  (d) A cut exactly `e`
    // above the corner: the projection reading says clean (the euclidian distance is
    // e/√2).  Controls: (e) and (f) are (b) and (c) mirrored to the bottom-left corner.
    let mut el = vec![];
    let cut = |el: &mut Vec<GdsElement>, xv: f64, mm: f64, vert: f64| {
        let (x1, y1) = (g(xv + w), g(2.0 + w)); // the via's top-right corner
        el.push(chamfered_tr(
            m,
            g(xv - mm),
            g(2.0 - mm),
            g(x1 + mm),
            g(y1 + mm),
            g(x1 + y1 + vert),
        ));
        el.push(l.sq(xv, 2.0));
    };
    cut(&mut el, x(0), e + 0.5, down(em * SQRT2)); // clean (a)
    cut(&mut el, x(1), e + 0.5, g(e / 2.0)); // TV (b)
    cut(&mut el, x(3), 3.0, g(e / 2.0)); // TV (c)
    cut(&mut el, x(5), e + 0.5, e); // clean (d)
    // (e), (f): the bottom-left corner cut along x + y = k, k below the corner's sum.
    let bl = |el: &mut Vec<GdsElement>, xv: f64, mm: f64, vert: f64| {
        let (x0, y0) = (g(xv - mm), g(12.0 - mm));
        let (x1, y1) = (g(xv + w + mm), g(12.0 + w + mm));
        let k = g(xv + 12.0 - vert);
        el.push(poly(
            m,
            &[
                (g(k - y0), y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, g(k - x0)),
            ],
        ));
        el.push(l.sq(xv, 12.0));
    };
    bl(&mut el, x(0), e + 0.5, g(e / 2.0)); // TV (e)
    bl(&mut el, x(3), 3.0, g(e / 2.0)); // TV (f)
    el.push(rect(o, -2.0, -2.0, 25.0, 17.0));
    l.write(&format!("{p}.h2"), el);

    // h3 — shapes that merge.  The metal as two abutting boxes with the seam under the
    // via, as a 4 × 4 grid of boxes, and the via as two abutting halves or two
    // overlapping boxes in a box of `e`: clean.  The metal as two overlapping boxes
    // whose union has `e − 0.005` on the right, drawn twice with `e − 0.005`, a metal
    // frame whose left wall holds the via `e − 0.005` from the hole, and the via as two
    // halves with `e − 0.005` on the right fire once each; a via in a frame's hole has
    // no metal.
    let h = g(w / 2.0);
    let mut el = vec![];
    let mid = g(x(0) + h);
    el.extend([
        rect(m, g(x(0) - e), g(2.0 - e), mid, g(2.0 + w + e)),
        rect(m, mid, g(2.0 - e), g(x(0) + w + e), g(2.0 + w + e)),
        l.sq(x(0), 2.0), // clean: abutting metal
        rect(
            m,
            g(x(1) - e),
            g(2.0 - e),
            g(x(1) + 0.6 * w),
            g(2.0 + w + e),
        ),
        rect(
            m,
            g(x(1) + 0.4 * w),
            g(2.0 - e),
            g(x(1) + w + em),
            g(2.0 + w + e),
        ),
        l.sq(x(1), 2.0), // TV: overlapping metal, union short
    ]);
    el.extend(l.enc(m, x(2), 2.0, e, em, e, e));
    el.push(rect(
        m,
        g(x(2) - e),
        g(2.0 - e),
        g(x(2) + w + em),
        g(2.0 + w + e),
    )); // TV: twice
    let q = g((w + 2.0 * e) / 4.0);
    for i in 0..4 {
        for j in 0..4 {
            let (gx, gy) = (g(x(3) - e + q * i as f64), g(2.0 - e + q * j as f64));
            el.push(rect(m, gx, gy, g(gx + q), g(gy + q)));
        }
    }
    el.push(l.sq(x(3), 2.0)); // clean: a grid
    el.extend(ring(
        m,
        g(x(4) - 1.0),
        1.0,
        g(x(4) + w + 1.0),
        g(3.0 + w),
        g(x(4) + w + em),
        1.5,
        g(x(4) + w + 0.6),
        g(2.5 + w),
    ));
    el.push(l.sq(x(4), 2.0)); // TV: on the frame's wall
    el.extend(ring(
        m,
        g(x(5) - 1.0),
        1.0,
        g(x(5) + w + 1.0),
        g(3.0 + w),
        g(x(5) - 0.5),
        1.5,
        g(x(5) + w + 0.5),
        g(2.5 + w),
    ));
    el.push(l.sq(x(5), 2.0)); // TV: in the hole
    el.extend([
        rect(v, x(6), 2.0, g(x(6) + h), g(2.0 + w)),
        rect(v, g(x(6) + h), 2.0, g(x(6) + w), g(2.0 + w)),
        rect(m, g(x(6) - e), g(2.0 - e), g(x(6) + w + e), g(2.0 + w + e)), // clean: via halves
        rect(v, x(7), 2.0, g(x(7) + h), g(2.0 + w)),
        rect(v, g(x(7) + h), 2.0, g(x(7) + w), g(2.0 + w)),
        rect(m, g(x(7) - e), g(2.0 - e), g(x(7) + w + em), g(2.0 + w + e)), // TV: via halves, short
        rect(v, x(8), 2.0, g(x(8) + 0.6 * w), g(2.0 + w)),
        rect(v, g(x(8) + 0.4 * w), 2.0, g(x(8) + w), g(2.0 + w)),
        rect(m, g(x(8) - e), g(2.0 - e), g(x(8) + w + e), g(2.0 + w + e)), // clean: via overlap
        rect(o, 0.0, 0.0, 40.0, 5.0),
    ]);
    l.write(&format!("{p}.h3"), el);

    // h4 — tile lines.  At x = 20: a via straddling the line with `e − 0.005` on the
    // right, the metal's edge on the line `e − 0.005` from the via, an `e − 0.005` margin
    // straddling the line, and an `e` margin straddling it (clean); a straddling via
    // with `e − 0.005` on the right at x = 21, 40, 42, and one straddling y = 20 with
    // `e − 0.005` on top.  Seven fire.
    let hv = down(w / 2.0);
    let hm = down(em / 2.0);
    let he = down(e / 2.0);
    let mut el = vec![];
    el.extend(l.enc(m, g(20.0 - hv), 2.0, e, em, e, e));
    el.extend(l.enc(m, g(20.0 - em - w), 5.0, e, em, e, e));
    el.extend(l.enc(m, g(20.0 - hm - w), 8.0, e, em, e, e));
    el.extend(l.enc(m, g(20.0 - he - w), 11.0, e, e, e, e)); // clean
    el.extend(l.enc(m, g(21.0 - hv), 14.0, e, em, e, e));
    el.extend(l.enc(m, g(40.0 - hv), 2.0, e, em, e, e));
    el.extend(l.enc(m, g(42.0 - hv), 5.0, e, em, e, e));
    el.extend(l.enc(m, 10.0, g(20.0 - hv), e, e, e, em));
    el.push(rect(o, 15.0, 0.0, 45.0, 17.0));
    el.push(rect(o, 8.0, 18.0, 12.0, 22.0));
    l.write(&format!("{p}.h4"), el);

    // h5/h6 — fifty vias with `e − 0.005` on the right, flat and as an array reference.
    let mut cell = l.enc(m, 1.0, 1.0, e, em, e, e);
    cell.push(rect(o, 0.0, 0.0, 3.0, 3.0));
    l.arrays(&p, 5, cell, 3.0);

    // h7 — large and far.  A 300 µm metal strip `w + 2e` wide with three vias at `e`
    // (clean) and one `w + 2(e − 0.005)` wide with three vias short top and bottom (two
    // walls each); a via at (1000, 1000) with `e − 0.005` on the right.
    let mut el = vec![
        rect(m, 2.0, 2.0, 302.0, g(2.0 + w + 2.0 * e)),
        rect(m, 2.0, 6.0, 302.0, g(6.0 + w + 2.0 * em)),
        rect(o, 0.0, 0.0, 305.0, 10.0),
        rect(o, 998.0, 998.0, 1003.0, 1003.0),
    ];
    for xv in [10.0, 150.0, 290.0] {
        el.push(l.sq(xv, g(2.0 + e))); // clean
        el.push(l.sq(xv, g(6.0 + em))); // TV × 2
    }
    el.extend(l.enc(m, 1000.0, 1000.0, e, em, e, e)); // TV: far
    l.write(&format!("{p}.h7"), el);

    // h8 — the seal (section 6.10).  Under an EdgeSeal plate (1, 1)-(9, 9) a via with
    // `e − 0.005` all round and a bare via are not checked; a via straddling the seal's
    // edge x = 9 whose outer piece has `e − 0.005` on the right fires (the piece is not
    // a `w` square either); a via outside with its left wall on the edge and
    // `e − 0.005` on the right fires; a via at (12, 2) with `e − 0.005` on the right.
    let mut el = vec![rect(l.seal, 1.0, 1.0, 9.0, 9.0)];
    el.extend(l.enc_m(m, 2.0, 2.0, em)); // clean: in the seal
    el.push(l.sq(2.0, 6.0)); // clean: bare, in the seal
    el.extend(l.enc(m, g(9.0 - hv), 5.0, e, em, e, e)); // TV: cut by the edge
    el.extend(l.enc(m, 9.0, 2.0, e, em, e, e)); // TV: outside, on the edge
    el.extend(l.enc(m, 12.0, 2.0, e, em, e, e)); // TV
    el.push(rect(o, 0.0, 0.0, 16.0, 10.0));
    l.write(&format!("{p}.h8"), el);
}
