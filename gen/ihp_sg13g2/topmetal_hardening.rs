// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Hardening patterns (hardening/SPEC.md) for the `topmetal1` and `topmetal2` decks:
// layouts drawn from the manual's sections 5.22/5.23 (TM1.*, TM1Fil.*) and 5.25/5.26
// (TM2.*, TM2Fil.*) alone, one fixture per theme, `TM<n>.<rule>.h<k>` and
// `TM<n>Fil.<rule>.h<k>`.  The two decks share every rule but TM2.bR, so one function
// draws each theme for both, with the deck's own values.  Each function's comment
// states the geometry and what the manual says about it; the expected answers are in
// the `topmetal1`/`topmetal2` tables of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/topmetal.md.

use crate::helpers::{
    chamfered_tr, density_pattern, diamond, layer, library, poly, rect, ref_array, strip45,
    write_gz,
};
use gds21::{GdsElement, GdsLibrary};
use gdscheck::pdk::PdkConfig;

const SQRT2: f64 = std::f64::consts::SQRT_2;

/// Layers and values of one deck.
struct L {
    n: i32,
    dir: String,
    met: (i16, i16),
    fil: (i16, i16),
    mask: (i16, i16),
    boundary: (i16, i16),
    trans: (i16, i16),
    ind: (i16, i16),
    via: (i16, i16),
    below: (i16, i16),
    /// TM<n>.a and TM<n>.b (the same value in both decks).
    w: f64,
}

impl L {
    fn new(pdk: &PdkConfig, n: i32) -> Self {
        let (via, below) = if n == 1 {
            (layer(pdk, "TopVia1"), layer(pdk, "Metal5"))
        } else {
            (layer(pdk, "TopVia2"), layer(pdk, "TopMetal1"))
        };
        L {
            n,
            dir: format!("tests/data/ihp-sg13g2/topmetal{n}"),
            met: layer(pdk, &format!("TopMetal{n}")),
            fil: layer(pdk, &format!("TopMetal{n}.filler")),
            mask: layer(pdk, &format!("TopMetal{n}.mask")),
            boundary: layer(pdk, "EdgeSeal.boundary"),
            trans: layer(pdk, "TRANS"),
            ind: layer(pdk, "IND"),
            via,
            below,
            w: if n == 1 { 1.64 } else { 2.00 },
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
}

/// Ring on `l`: outer box minus the hole, as four overlapping walls.
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

/// A comb on `l`: base `(x0, y0)-(x0 + 3·tooth + 2·slot, y0 + base)`, three teeth
/// `tooth` wide and `height` tall, slots `slot` wide between them.
fn comb(
    l: (i16, i16),
    x0: f64,
    y0: f64,
    tooth: f64,
    slot: f64,
    base: f64,
    height: f64,
) -> GdsElement {
    let (t, s) = (tooth, slot);
    let y1 = y0 + base;
    let y2 = y1 + height;
    poly(
        l,
        &[
            (x0, y0),
            (g(x0 + 3.0 * t + 2.0 * s), y0),
            (g(x0 + 3.0 * t + 2.0 * s), y2),
            (g(x0 + 2.0 * t + 2.0 * s), y2),
            (g(x0 + 2.0 * t + 2.0 * s), y1),
            (g(x0 + 2.0 * t + s), y1),
            (g(x0 + 2.0 * t + s), y2),
            (g(x0 + t + s), y2),
            (g(x0 + t + s), y1),
            (g(x0 + t), y1),
            (g(x0 + t), y2),
            (x0, y2),
        ],
    )
}

/// A U on `l`: base `(x0, y0)-(x0 + 2·arm + gap, y0 + base)` with two arms `arm` wide
/// and `height` tall, `gap` apart.
fn u(l: (i16, i16), x0: f64, y0: f64, arm: f64, gap: f64, base: f64, height: f64) -> GdsElement {
    let x1 = g(x0 + 2.0 * arm + gap);
    let y1 = y0 + base;
    let y2 = y1 + height;
    poly(
        l,
        &[
            (x0, y0),
            (x1, y0),
            (x1, y2),
            (g(x1 - arm), y2),
            (g(x1 - arm), y1),
            (g(x0 + arm), y1),
            (g(x0 + arm), y2),
            (x0, y2),
        ],
    )
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

/// The y offset of a second 45° strip of half-width `d` so that the perpendicular gap
/// to the first is `gap`, rounded down (`grid_up == false`) or up onto the grid.
fn strip_dy(gap: f64, d: f64, grid_up: bool) -> f64 {
    let dy = SQRT2 * gap + 2.0 * d;
    if grid_up { up(dy) } else { down(dy) }
}

pub fn generate(pdk: &PdkConfig) {
    for n in 1..3 {
        let l = L::new(pdk, n);
        std::fs::create_dir_all(&l.dir).expect("failed to create output directory");
        tm_a_h(&l);
        tm_b_h(&l);
        tm_density_h(&l);
        tmfil_a_h(&l);
        tmfil_a1_h(&l);
        tmfil_b_h(&l);
        tmfil_c_h(&l);
        tmfil_d_h(&l);
    }
    tm2_br_h(&L::new(pdk, 2));
}

// --- TM<n>.a: min. TopMetal width (1.64 / 2.00) ---

fn tm_a_h(l: &L) {
    let (m, w) = (l.met, l.w);
    let wm = g(w - 0.005);
    let p = format!("TM{}.a", l.n);

    // h1 — the bound and long shapes.  `w` wide is legal, `w − 0.005` is not, in x and in
    // y; a 300 µm bar crossing every tile line is one narrow shape; a 0.005 µm sliver
    // and a narrow bar at (1000, 1000).
    l.write(
        &format!("{p}.h1"),
        vec![
            rect(m, 2.0, 2.0, g(2.0 + w), 6.0),              // clean
            rect(m, 6.0, 2.0, g(6.0 + wm), 6.0),             // TM.a (x)
            rect(m, 10.0, 2.0, 14.0, g(2.0 + wm)),           // TM.a (y)
            rect(m, 2.0, 10.0, 302.0, g(10.0 + w)),          // clean, 300 µm
            rect(m, 2.0, 15.0, 302.0, g(15.0 + wm)),         // TM.a, 300 µm
            rect(m, 2.0, 20.0, 2.005, 24.0),                 // TM.a, a sliver
            rect(m, 1000.0, 1000.0, g(1000.0 + wm), 1004.0), // TM.a, far away
        ],
    );

    // h3 — shapes that merge.  The rule reads the union: two overlapping boxes whose
    // union is `w` wide are clean, `w − 0.005` fires once; four abutting slices of w/4
    // make w (clean), three plus a short one make w − 0.005 (fires); a w bar drawn as a
    // 4 × 8 grid of boxes is clean; a ring with one narrow side fires once; an island
    // inside a ring's hole is a shape of its own (narrow → fires once).
    let q = g(w / 4.0);
    let ov = g(w * 0.6);
    let mut e = vec![
        rect(m, 2.0, 2.0, g(2.0 + ov), 6.0),
        rect(m, g(2.0 + w - ov), 2.0, g(2.0 + w), 6.0), // union w → clean
        rect(m, 7.0, 2.0, g(7.0 + ov), 6.0),
        rect(m, g(7.0 + wm - ov), 2.0, g(7.0 + wm), 6.0), // union w − 0.005 → TM.a
    ];
    for i in 0..4 {
        e.push(rect(
            m,
            g(12.0 + q * i as f64),
            2.0,
            g(12.0 + q * (i + 1) as f64),
            6.0,
        )); // clean
    }
    for i in 0..3 {
        e.push(rect(
            m,
            g(17.0 + q * i as f64),
            2.0,
            g(17.0 + q * (i + 1) as f64),
            6.0,
        ));
    }
    e.push(rect(m, g(17.0 + 3.0 * q), 2.0, g(17.0 + wm), 6.0)); // TM.a
    for i in 0..4 {
        for j in 0..8 {
            let (x, y) = (g(22.0 + q * i as f64), 2.0 + 0.5 * j as f64);
            e.push(rect(m, x, y, g(x + q), y + 0.5)); // clean grid
        }
    }
    e.extend(ring(m, 2.0, 10.0, 12.0, 20.0, g(2.0 + wm), 13.0, 9.0, 17.0)); // left side narrow → TM.a
    e.extend(ring(m, 16.0, 10.0, 28.0, 20.0, 19.0, 12.0, 25.0, 18.0));
    e.push(rect(m, 21.0, 14.0, g(21.0 + wm), 16.0)); // island narrow → TM.a
    l.write(&format!("{p}.h3"), e);

    // h7 — a comb whose three teeth are narrow (three violations of one polygon, two
    // markers each; the slots are 2.5 wide, clear of TM.b) and a U whose arms are `w`
    // (clean).
    l.write(
        &format!("{p}.h7"),
        vec![
            comb(m, 2.0, 2.0, wm, 2.5, 3.0, 4.0),
            u(m, 16.0, 2.0, w, 2.5, 3.0, 4.0),
        ],
    );
}

// --- TM<n>.b: min. TopMetal space or notch (1.64 / 2.00) ---

fn tm_b_h(l: &L) {
    let (m, s) = (l.met, l.w);
    let sm = g(s - 0.005);
    let p = format!("TM{}.b", l.n);

    // h2 — 45° geometry.  A diamond's tip `s − 0.005` from a wall (fires; `s` clean); two
    // 45° strips at a perpendicular gap one step under s (fires; at/over s clean); a
    // chamfer passing one step under s from a square's corner (fires; at/over s clean);
    // two diamond tips facing `s − 0.005` apart (fires; `s` clean).
    let d = 2.5; // strip half-width: width 3.54, clear of TM.a
    let k_f = down(SQRT2 * sm); // corner-to-chamfer offset along x + y
    let k_c = up(SQRT2 * s);
    l.write(
        &format!("{p}.h2"),
        vec![
            diamond(m, 5.0, 5.0, 3.0),
            rect(m, g(8.0 + sm), 2.0, g(12.0 + sm), 8.0), // TM.b: tip to wall
            diamond(m, 19.0, 5.0, 3.0),
            rect(m, g(22.0 + s), 2.0, g(26.0 + s), 8.0), // clean
            strip45(m, 36.0, 2.0, 6.0, d),
            strip45(m, 36.0, g(2.0 + strip_dy(sm, d, false)), 6.0, d), // TM.b: strips
            strip45(m, 52.0, 2.0, 6.0, d),
            strip45(m, 52.0, g(2.0 + strip_dy(s, d, true)), 6.0, d), // clean
            chamfered_tr(m, 2.0, 14.0, 8.0, 20.0, 24.0),
            rect(m, g(6.0 + k_f), 18.0, g(10.0 + k_f), 22.0), // TM.b: corner to the chamfer x + y = 24
            chamfered_tr(m, 18.0, 14.0, 24.0, 20.0, 40.0),
            rect(m, g(22.0 + k_c), 18.0, g(26.0 + k_c), 22.0), // clean
            diamond(m, 29.0, 30.0, 3.0),
            diamond(m, g(35.0 + sm), 30.0, 3.0), // TM.b: tip to tip
            diamond(m, 49.0, 30.0, 3.0),
            diamond(m, g(55.0 + s), 30.0, 3.0), // clean
        ],
    );

    // h4 — shapes that merge.  A union of overlapping boxes and a union of abutting
    // boxes `s − 0.005` apart fire once, not once per drawn box; a plate drawn as a 3 × 4
    // grid of boxes `s − 0.005` from a bar fires once; two boxes overlapping into one
    // L-shaped region draw nothing.
    let mut e = vec![
        rect(m, 2.0, 2.0, 6.0, 6.0),
        rect(m, 5.0, 2.0, 9.0, 6.0),
        rect(m, g(9.0 + sm), 2.0, g(11.0 + sm), 6.0),
        rect(m, g(11.0 + sm), 2.0, g(13.0 + sm), 6.0), // TM.b once
        rect(m, 20.0, 2.0, 28.0, 6.0),
    ];
    for i in 0..3 {
        for j in 0..4 {
            let (x, y) = (20.0 + 2.0 * i as f64, g(6.0 + sm) + j as f64);
            e.push(rect(m, x, y, x + 2.0, y + 1.0));
        }
    } // TM.b once (the grid's bottom row vs the bar)
    e.extend([
        rect(m, 2.0, 12.0, 6.0, 16.0),
        rect(m, 4.0, 12.0, 10.0, 14.0), // overlaps: one shape, no gap
    ]);
    l.write(&format!("{p}.h4"), e);

    // h8 — large and small.  A 0.005 µm sliver `s − 0.005` from a plate (fires TM.b; its
    // own width is TM.a's), two 300 µm bars `s − 0.005` apart (one gap crossing every tile
    // line), and a pair at (1000, 1000).
    l.write(
        &format!("{p}.h8"),
        vec![
            rect(m, 2.0, 2.0, 6.0, 6.0),
            rect(m, g(6.0 + sm), 2.0, g(6.005 + sm), 6.0), // TM.b (+ TM.a)
            rect(m, 2.0, 10.0, 302.0, 14.0),
            rect(m, 2.0, g(14.0 + sm), 302.0, g(18.0 + sm)), // TM.b
            rect(m, 1000.0, 1000.0, 1004.0, 1004.0),
            rect(m, g(1004.0 + sm), 1000.0, g(1008.0 + sm), 1004.0), // TM.b
        ],
    );

    // h9 — nets.  The rule names no net: two shapes `s − 0.005` apart fire whether they
    // are joined through a via and the metal below (left pair) or not (right pair).
    let (v, b) = (l.via, l.below);
    let vs = if l.n == 1 { 0.42 } else { 0.9 }; // the via's square
    let vc = |cx: f64, cy: f64| {
        rect(
            v,
            g(cx - vs / 2.0),
            g(cy - vs / 2.0),
            g(cx + vs / 2.0),
            g(cy + vs / 2.0),
        )
    };
    let x2 = g(6.0 + sm);
    l.write(
        &format!("{p}.h9"),
        vec![
            rect(m, 2.0, 2.0, 6.0, 6.0),
            rect(m, x2, 2.0, x2 + 4.0, 6.0), // TM.b, same net
            vc(4.0, 4.0),
            vc(x2 + 2.0, 4.0),
            rect(b, 2.0, 2.0, x2 + 4.0, 6.0),
            rect(m, 20.0, 2.0, 24.0, 6.0),
            rect(m, g(24.0 + sm), 2.0, g(28.0 + sm), 6.0), // TM.b, different nets
        ],
    );
}

// --- TM<n>.c / TM<n>.d: global density 25 % / 70 % over EdgeSeal.boundary ---

fn tm_density_h(l: &L) {
    let (m, f, k, b) = (l.met, l.fil, l.mask, l.boundary);
    let p = format!("TM{}", l.n);

    // c.h1 — the layers count once.  Ten 30 µm stripes at a 100 µm pitch drawn on the
    // metal, its filler and its mask alike: 30 % of the 1000 µm chip, between the 25 %
    // floor and the 70 % ceiling (clean).  Added instead of united they would read 90 %
    // and fire TM.d.
    let mut e = density_pattern(b, 1000.0, &[]);
    for j in 0..10 {
        let y = 100.0 * j as f64;
        for layer in [m, f, k] {
            e.push(rect(layer, 0.0, y, 1000.0, y + 30.0));
        }
    }
    l.write(&format!("{p}.c.h1"), e);

    // c.h2 — the boundary.  250 µm of metal inside the 1000 µm boundary is 25.000 %
    // (clean, the bound itself); a 1000 × 1000 plate beside the boundary is outside the
    // count.
    l.write(
        &format!("{p}.c.h2"),
        vec![
            rect(b, 0.0, 0.0, 1000.0, 1000.0),
            rect(m, 0.0, 500.0, 1000.0, 750.0),   // 25 % inside
            rect(m, 1100.0, 0.0, 2100.0, 1000.0), // outside the boundary entirely
        ],
    );

    // c.h3 / c.h4 — a plate reaching in from below with 250 of its 500 µm inside the
    // boundary: 25.000 % (clean); with 249.95 inside: 24.995 % (TM.c).
    l.write(
        &format!("{p}.c.h3"),
        vec![
            rect(b, 0.0, 0.0, 1000.0, 1000.0),
            rect(m, 0.0, -250.0, 1000.0, 250.0),
        ],
    );
    l.write(
        &format!("{p}.c.h4"),
        vec![
            rect(b, 0.0, 0.0, 1000.0, 1000.0),
            rect(m, 0.0, -250.05, 1000.0, 249.95), // TM.c
        ],
    );

    // c.h5 / c.h6 — hierarchy.  A 100 × 100 cell holding a 100 × 30 stripe, placed as a
    // 10 × 10 array reference under the 1000 µm boundary: 30 % (clean); the same with a
    // 100 × 24.995 stripe: 24.995 % (TM.c).  The boundary is drawn in TOP.
    for (h, name) in [(30.0, "c.h5"), (24.995, "c.h6")] {
        let mut lib = ref_array(vec![rect(m, 0.0, 0.0, 100.0, h)], 10, 10, 100.0);
        let top = lib
            .structs
            .iter_mut()
            .find(|s| s.name == "TOP")
            .expect("TOP");
        top.elems.push(rect(b, 0.0, 0.0, 1000.0, 1000.0));
        l.write_lib(&format!("{p}.{name}"), lib);
    }

    // d.h1 / d.h2 — the ceiling with overlapping layers.  Metal over 0..500 and mask
    // over 200..700 of the 1000 µm chip: the union is 70.000 % (clean); with the mask to
    // 700.05 it is 70.005 % (TM.d).  Added instead of united both would read 100 %.
    l.write(
        &format!("{p}.d.h1"),
        vec![
            rect(b, 0.0, 0.0, 1000.0, 1000.0),
            rect(m, 0.0, 0.0, 1000.0, 500.0),
            rect(k, 0.0, 200.0, 1000.0, 700.0),
        ],
    );
    l.write(
        &format!("{p}.d.h2"),
        vec![
            rect(b, 0.0, 0.0, 1000.0, 1000.0),
            rect(m, 0.0, 0.0, 1000.0, 500.0),
            rect(k, 0.0, 200.0, 1000.0, 700.05), // TM.d
        ],
    );
}

// --- TM<n>Fil.a: min. filler width 5.00 ---

fn tmfil_a_h(l: &L) {
    let f = l.fil;
    let p = format!("TM{}Fil.a", l.n);

    // h1 — the bound.  5.0 wide is legal, 4.995 is not, in x and in y; a 0.005 µm sliver
    // and a 4.995 bar at (1000, 1000).  (A long bar is TMFil.a1's; none here.)
    l.write(
        &format!("{p}.h1"),
        vec![
            rect(f, 2.0, 2.0, 7.0, 8.0),               // clean
            rect(f, 12.0, 2.0, 16.995, 8.0),           // TMFil.a (x)
            rect(f, 22.0, 2.0, 28.0, 6.995),           // TMFil.a (y)
            rect(f, 2.0, 14.0, 2.005, 20.0),           // TMFil.a, a sliver
            rect(f, 1000.0, 1000.0, 1004.995, 1006.0), // TMFil.a, far away
        ],
    );

    // h2 — 45°.  A diamond of half-diagonal 3.54 is 5.006 wide (clean), 3.535 is 4.999
    // (fires); a 45° strip of d = 3.54 (clean) and 3.535 (fires); 6 × 6 boxes with a
    // corner chamfered by 1, 2, 3.5 and 4: the chamfer faces the opposite walls at 45°
    // across 6 − c, so 2, 3.5 and 4 fire (twice each, one per wall) and 1 is clean.
    l.write(
        &format!("{p}.h2"),
        vec![
            diamond(f, 6.0, 6.0, 3.54),                    // clean
            diamond(f, 18.0, 6.0, 3.535),                  // TMFil.a
            strip45(f, 32.0, 2.0, 4.0, 3.54),              // clean
            strip45(f, 46.0, 2.0, 4.0, 3.535),             // TMFil.a
            chamfered_tr(f, 2.0, 14.0, 8.0, 20.0, 27.0),   // clean (chamfer 1)
            chamfered_tr(f, 12.0, 14.0, 18.0, 20.0, 36.0), // TMFil.a (chamfer 2: 4.0)
            chamfered_tr(f, 22.0, 14.0, 28.0, 20.0, 44.5), // TMFil.a (chamfer 3.5: 2.5)
            chamfered_tr(f, 32.0, 14.0, 38.0, 20.0, 54.0), // TMFil.a (chamfer 4: 2.0)
        ],
    );

    // h3 — shapes that merge.  Overlapping 3 × 6 boxes whose union is 5 (clean) or 4.995
    // (fires once); four abutting 1.25 slices (clean) and three plus 1.245 (fires); a 5 × 6
    // plate as a 4 × 6 grid (clean); a 16 × 16 ring with one 4.995 wall (fires once) and a
    // 21 × 21 ring holding a 4.995 island (fires once); the rings' spans are TMFil.a1's,
    // set aside.
    let mut e = vec![
        rect(f, 2.0, 2.0, 5.0, 8.0),
        rect(f, 4.0, 2.0, 7.0, 8.0), // union 5 → clean
        rect(f, 10.0, 2.0, 13.0, 8.0),
        rect(f, 11.995, 2.0, 14.995, 8.0), // union 4.995 → TMFil.a
    ];
    for i in 0..4 {
        e.push(rect(
            f,
            18.0 + 1.25 * i as f64,
            2.0,
            19.25 + 1.25 * i as f64,
            8.0,
        )); // clean
    }
    for i in 0..3 {
        e.push(rect(
            f,
            26.0 + 1.25 * i as f64,
            2.0,
            27.25 + 1.25 * i as f64,
            8.0,
        ));
    }
    e.push(rect(f, 29.75, 2.0, 30.995, 8.0)); // TMFil.a
    for i in 0..4 {
        for j in 0..6 {
            let (x, y) = (34.0 + 1.25 * i as f64, 2.0 + 1.0 * j as f64);
            e.push(rect(f, x, y, x + 1.25, y + 1.0)); // clean grid
        }
    }
    e.extend(ring(f, 2.0, 12.0, 18.0, 28.0, 6.995, 17.0, 13.0, 23.0)); // left wall 4.995 → TMFil.a
    e.extend(ring(f, 22.0, 12.0, 43.0, 33.0, 27.0, 17.0, 38.0, 28.0));
    e.push(rect(f, 30.0, 20.0, 34.995, 25.0)); // island 4.995 wide → TMFil.a
    l.write(&format!("{p}.h3"), e);

    // h7 — a comb whose three teeth are 4.995 wide (three violations of one polygon) and
    // a U with 5.0 arms (clean); both span more than 10 µm (TMFil.a1's, set aside).
    l.write(
        &format!("{p}.h7"),
        vec![
            comb(f, 2.0, 2.0, 4.995, 3.0, 5.0, 6.0),
            u(f, 30.0, 2.0, 5.0, 3.0, 5.0, 6.0),
        ],
    );
}

// --- TM<n>Fil.a1: max. filler width 10.00 (figure 5.23: the filler's long side) ---

fn tmfil_a1_h(l: &L) {
    let f = l.fil;
    let p = format!("TM{}Fil.a1", l.n);

    // h1 — the bound.  10 × 10 is legal, 10.005 × 10 and 10 × 10.005 are not; a 6 × 10.005
    // bar is (its long side); a 6 × 300 bar; a 6 × 10.005 bar at (1000, 1000).
    l.write(
        &format!("{p}.h1"),
        vec![
            rect(f, 2.0, 2.0, 12.0, 12.0),             // clean
            rect(f, 16.0, 2.0, 26.005, 12.0),          // TMFil.a1 (x)
            rect(f, 30.0, 2.0, 40.0, 12.005),          // TMFil.a1 (y)
            rect(f, 44.0, 2.0, 54.005, 8.0),           // TMFil.a1, 6 × 10.005
            rect(f, 2.0, 16.0, 302.0, 22.0),           // TMFil.a1, 300 µm
            rect(f, 1000.0, 1000.0, 1010.005, 1006.0), // TMFil.a1, far away
        ],
    );

    // h3 — shapes that merge.  Two overlapping 6 × 6 boxes whose union is 6 × 10 (clean)
    // or 6 × 10.005 (fires once); 5 × 6 abutting 5.005 × 6 (fires once); a 10.005 × 6 bar as
    // a 3 × 3 grid of boxes (fires once); an L with 6-wide arms spanning 12 (fires); a
    // plus with 6-wide arms spanning 14 (fires).
    let mut e = vec![
        rect(f, 2.0, 2.0, 8.0, 8.0),
        rect(f, 6.0, 2.0, 12.0, 8.0), // union 6 × 10 → clean
        rect(f, 16.0, 2.0, 22.0, 8.0),
        rect(f, 20.005, 2.0, 26.005, 8.0), // union 6 × 10.005 → TMFil.a1
        rect(f, 30.0, 2.0, 35.0, 8.0),
        rect(f, 35.0, 2.0, 40.005, 8.0), // abutting → TMFil.a1
    ];
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = (44.0 + 3.335 * i as f64, 2.0 + 2.0 * j as f64);
            e.push(rect(f, x, y, x + 3.335, y + 2.0)); // 10.005 × 6 grid → TMFil.a1
        }
    }
    e.extend([
        poly(
            f,
            &[
                (2.0, 14.0),
                (14.0, 14.0),
                (14.0, 20.0),
                (8.0, 20.0),
                (8.0, 26.0),
                (2.0, 26.0),
            ],
        ), // L → TMFil.a1
        poly(
            f,
            &[
                (22.0, 14.0),
                (28.0, 14.0),
                (28.0, 18.0),
                (32.0, 18.0),
                (32.0, 24.0),
                (28.0, 24.0),
                (28.0, 28.0),
                (22.0, 28.0),
                (22.0, 24.0),
                (18.0, 24.0),
                (18.0, 18.0),
                (22.0, 18.0),
            ],
        ), // plus → TMFil.a1
    ]);
    l.write(&format!("{p}.h3"), e);

    // h4 — tile lines.  10.005 × 6 bars well inside a tile, ending on x = 20, straddling
    // 20, starting on 20, straddling 21, ending on 40, straddling 42; 6 × 10.005 bars
    // across y = 20 and 21; an L of 5-wide arms cornered on x = 20 spanning 10.005.  Ten
    // oversized shapes; a 10 × 6 bar straddling 20 is clean.
    let mut e = vec![];
    for (i, x) in tile_xs(10.005).iter().enumerate() {
        let y = 2.0 + 10.0 * i as f64;
        e.push(rect(f, *x, y, g(x + 10.005), y + 6.0));
    }
    e.extend([
        rect(f, 60.0, 14.0, 66.0, 24.005),
        rect(f, 70.0, 15.0, 76.0, 25.005),
        poly(
            f,
            &[
                (20.0, 74.0),
                (30.005, 74.0),
                (30.005, 79.0),
                (25.0, 79.0),
                (25.0, 84.0),
                (20.0, 84.0),
            ],
        ),
        rect(f, 15.0, 88.0, 25.0, 94.0), // 10 × 6 straddling 20 → clean
    ]);
    l.write(&format!("{p}.h4"), e);

    // h7 — a 0.005 × 10.005 sliver (fires TMFil.a1 as well as TMFil.a).
    l.write(&format!("{p}.h7"), vec![rect(f, 2.0, 2.0, 2.005, 12.005)]);
}

// --- TM<n>Fil.b: min. filler space 3.00 ---

fn tmfil_b_h(l: &L) {
    let f = l.fil;
    let p = format!("TM{}Fil.b", l.n);

    // h2 — 45°.  A diamond's tip 2.995 from a filler wall (fires; 3.0 clean); two 45°
    // strips 5.006 wide (d = 3.54) and 9.05 long, overlapping 1.05 along the diagonal, at
    // a perpendicular gap of 2.995 (fires; 3.002 clean); a chamfered filler corner 2.995
    // from a square's corner (fires; 3.002 clean); two tips 2.995 apart (fires).
    let d = 3.54;
    l.write(
        &format!("{p}.h2"),
        vec![
            diamond(f, 6.0, 6.0, 4.0),
            rect(f, 12.995, 2.0, 18.995, 10.0), // TMFil.b: tip to wall
            diamond(f, 26.0, 6.0, 4.0),
            rect(f, 33.0, 2.0, 39.0, 10.0), // clean
            strip45(f, 50.0, 2.0, 6.4, d),
            strip45(f, 50.0, g(2.0 + strip_dy(2.995, d, false)), 6.4, d), // TMFil.b (2.995)
            strip45(f, 70.0, 2.0, 6.4, d),
            strip45(f, 70.0, g(2.0 + strip_dy(3.0, d, true)), 6.4, d), // clean (3.002)
            chamfered_tr(f, 2.0, 16.0, 8.0, 22.0, 26.0),
            rect(f, 8.235, 22.0, 14.235, 28.0), // TMFil.b: corner (8.235, 22) is 2.995 from x + y = 26
            chamfered_tr(f, 22.0, 16.0, 28.0, 22.0, 46.0),
            rect(f, 28.245, 22.0, 34.245, 28.0), // clean (3.002)
            diamond(f, 10.0, 36.0, 4.0),
            diamond(f, 20.995, 36.0, 4.0), // TMFil.b: tip to tip
        ],
    );

    // h3 — notches.  TMFil.b says "space", not "space or notch" (compare TM.b): a U filler
    // with a 2.995 notch draws nothing; two Ls whose arms face across 2.995 (separate
    // shapes) fire; an island 2.995 from a ring's inner wall fires.  All three span more
    // than 10 µm (TMFil.a1, set aside).
    let mut e = vec![
        u(f, 2.0, 2.0, 6.0, 2.995, 6.0, 6.0), // no TMFil.b
        poly(
            f,
            &[
                (24.0, 2.0),
                (36.0, 2.0),
                (36.0, 8.0),
                (30.0, 8.0),
                (30.0, 14.0),
                (24.0, 14.0),
            ],
        ),
        poly(
            f,
            &[
                (32.995, 11.0),
                (44.0, 11.0),
                (44.0, 23.0),
                (38.0, 23.0),
                (38.0, 17.0),
                (32.995, 17.0),
            ],
        ), // TMFil.b: facing Ls
    ];
    e.extend(ring(f, 50.0, 2.0, 74.0, 26.0, 56.0, 8.0, 68.0, 20.0));
    e.push(rect(f, 58.995, 11.0, 65.0, 17.0)); // TMFil.b: island to the ring's wall
    l.write(&format!("{p}.h3"), e);

    // h4 — shapes that merge.  Two unions 2.995 apart fire once; a plate drawn as a 2 × 3
    // grid 2.995 from a filler fires once.
    let mut e = vec![
        rect(f, 2.0, 2.0, 7.0, 8.0),
        rect(f, 5.0, 2.0, 10.0, 8.0),
        rect(f, 12.995, 2.0, 15.995, 8.0),
        rect(f, 15.995, 2.0, 18.995, 8.0), // TMFil.b once
        rect(f, 30.0, 2.0, 36.0, 8.0),
    ];
    for i in 0..2 {
        for j in 0..3 {
            let (x, y) = (30.0 + 3.0 * i as f64, 10.995 + 2.0 * j as f64);
            e.push(rect(f, x, y, x + 3.0, y + 2.0));
        }
    } // TMFil.b once
    l.write(&format!("{p}.h4"), e);

    // h8 — large and small.  A 0.005 sliver 2.995 from a filler (TMFil.b; its width is
    // TMFil.a's), two 6 × 10 fillers 2.995 apart, a pair at (1000, 1000).
    l.write(
        &format!("{p}.h8"),
        vec![
            rect(f, 2.0, 2.0, 8.0, 8.0),
            rect(f, 10.995, 2.0, 11.0, 8.0), // TMFil.b (+ TMFil.a)
            rect(f, 20.0, 2.0, 26.0, 12.0),
            rect(f, 28.995, 2.0, 34.995, 12.0), // TMFil.b
            rect(f, 1000.0, 1000.0, 1006.0, 1006.0),
            rect(f, 1008.995, 1000.0, 1014.995, 1006.0), // TMFil.b
        ],
    );
}

// --- TM<n>Fil.c: min. filler space to TopMetal 3.00 ---

fn tmfil_c_h(l: &L) {
    let (f, m) = (l.fil, l.met);
    let p = format!("TM{}Fil.c", l.n);

    // h2 — 45°.  A metal diamond's tip 2.995 from a filler wall (fires; 3.0 clean); a
    // metal 45° strip 2.995 from a filler strip 5.006 wide (d = 3.54) and 9.05 long
    // (fires); a metal chamfer 2.995 from a filler's corner (fires); a filler chamfer
    // 2.995 from a metal's corner (fires; 3.002 clean).
    let d = 3.54;
    l.write(
        &format!("{p}.h2"),
        vec![
            diamond(m, 6.0, 6.0, 4.0),
            rect(f, 12.995, 2.0, 18.995, 10.0), // TMFil.c: tip to wall
            diamond(m, 26.0, 6.0, 4.0),
            rect(f, 33.0, 2.0, 39.0, 10.0), // clean
            strip45(f, 70.0, 2.0, 6.4, d),
            strip45(m, 66.0, g(2.0 + strip_dy(2.995, d, false) - 4.0), 14.4, d), // TMFil.c (2.995)
            chamfered_tr(m, 2.0, 16.0, 8.0, 22.0, 26.0),
            rect(f, 8.235, 22.0, 14.235, 28.0), // TMFil.c: filler corner to metal chamfer
            chamfered_tr(f, 22.0, 16.0, 28.0, 22.0, 46.0),
            rect(m, 28.235, 22.0, 32.235, 28.0), // TMFil.c: metal corner to filler chamfer
            chamfered_tr(f, 40.0, 16.0, 46.0, 22.0, 64.0),
            rect(m, 46.245, 22.0, 50.245, 28.0), // clean (3.002)
        ],
    );

    // h3 — no distance.  A filler abutting a metal along one edge is a space of zero
    // (fires); a filler across a metal's edge and a filler wholly inside a metal plate
    // share area with it and are no pair (nothing).
    l.write(
        &format!("{p}.h3"),
        vec![
            rect(f, 2.0, 2.0, 8.0, 8.0),
            rect(m, 8.0, 2.0, 12.0, 8.0), // TMFil.c: abutting
            rect(f, 20.0, 2.0, 26.0, 8.0),
            rect(m, 24.0, 3.0, 30.0, 7.0), // crossing: nothing
            rect(m, 36.0, 0.0, 52.0, 16.0),
            rect(f, 41.0, 5.0, 47.0, 11.0), // inside: nothing
        ],
    );

    // h4 — tile lines.  Filler-to-metal gaps of 2.995 (the metal on the right in the
    // even rows, on the left in the odd ones) well inside a tile and ending on x = 20,
    // straddling 20, starting on 20, straddling 21, ending on 40, straddling 42; one
    // ending on y = 20; a corner-on pair with its corner on x = 20.  Nine gaps.
    let mut e = vec![];
    for (i, (x0, x1)) in tile_gaps(2.995).iter().enumerate() {
        let y = 2.0 + 10.0 * i as f64;
        if i % 2 == 0 {
            e.push(rect(f, x0 - 6.0, y, *x0, y + 6.0));
            e.push(rect(m, *x1, y, x1 + 4.0, y + 6.0));
        } else {
            e.push(rect(m, x0 - 4.0, y, *x0, y + 6.0));
            e.push(rect(f, *x1, y, x1 + 6.0, y + 6.0));
        }
    }
    e.extend([
        rect(f, 60.0, 11.005, 66.0, 17.005),
        rect(m, 60.0, 20.0, 64.0, 26.0), // gap ends on y = 20
        rect(f, 14.0, 76.0, 20.0, 82.0),
        rect(m, 22.995, 82.0, 26.995, 88.0), // corner-on, corner on x = 20
    ]);
    l.write(&format!("{p}.h4"), e);

    // h7 — large and small.  A 300 µm metal bar 2.995 from a filler (one gap), a metal
    // sliver 0.005 wide 2.995 from a filler (TMFil.c; its width is TM.a's), a pair at
    // (1000, 1000).
    l.write(
        &format!("{p}.h7"),
        vec![
            rect(f, 2.0, 2.0, 8.0, 8.0),
            rect(m, 2.0, 10.995, 302.0, 14.995), // TMFil.c
            rect(f, 2.0, 22.0, 8.0, 28.0),
            rect(m, 10.995, 22.0, 11.0, 28.0), // TMFil.c (+ TM.a)
            rect(f, 1000.0, 1000.0, 1006.0, 1006.0),
            rect(m, 1008.995, 1000.0, 1012.995, 1006.0), // TMFil.c
        ],
    );
}

// --- TM<n>Fil.d: min. filler space to TRANS 4.90 ---

fn tmfil_d_h(l: &L) {
    let (f, t) = (l.fil, l.trans);
    let p = format!("TM{}Fil.d", l.n);

    // h1 — the bound and both metrics.  A 6 × 6 filler and a 6 × 6 TRANS: 4.9 apart
    // clean, 4.895 fires in x and in y; a diagonal 3.46/3.46 (4.893) fires, 3.465/3.465
    // (4.900) is clean; a corner-on 4.895 fires.
    l.write(
        &format!("{p}.h1"),
        vec![
            rect(f, 2.0, 2.0, 8.0, 8.0),
            rect(t, 12.9, 2.0, 18.9, 8.0), // clean
            rect(f, 24.0, 2.0, 30.0, 8.0),
            rect(t, 34.895, 2.0, 40.895, 8.0), // TMFil.d (x)
            rect(f, 46.0, 2.0, 52.0, 8.0),
            rect(t, 46.0, 12.895, 52.0, 18.895), // TMFil.d (y)
            rect(f, 2.0, 24.0, 8.0, 30.0),
            rect(t, 11.465, 33.465, 17.465, 39.465), // clean diagonal 4.900
            rect(f, 24.0, 24.0, 30.0, 30.0),
            rect(t, 33.46, 33.46, 39.46, 39.46), // TMFil.d diagonal 4.893
            rect(f, 46.0, 24.0, 52.0, 30.0),
            rect(t, 56.895, 30.0, 62.895, 36.0), // TMFil.d corner-on
        ],
    );

    // h2 — 45°.  A TRANS diamond's tip 4.895 from a filler wall (fires; 4.9 clean); a
    // TRANS chamfer 4.893 from a filler's corner (fires); a filler chamfer 4.893 from a
    // TRANS corner (fires); a TRANS 45° strip 4.893 from a filler strip 5.006 wide
    // (d = 3.54) and 9.05 long (fires).
    let d = 3.54;
    l.write(
        &format!("{p}.h2"),
        vec![
            diamond(t, 6.0, 6.0, 4.0),
            rect(f, 14.895, 2.0, 20.895, 10.0), // TMFil.d: tip to wall
            diamond(t, 30.0, 6.0, 4.0),
            rect(f, 38.9, 2.0, 44.9, 10.0), // clean
            chamfered_tr(t, 2.0, 16.0, 8.0, 22.0, 26.0),
            rect(f, 10.92, 22.0, 16.92, 28.0), // TMFil.d: corner (10.92, 22) is 4.893 from x + y = 26
            chamfered_tr(f, 24.0, 16.0, 30.0, 22.0, 48.0),
            rect(t, 32.92, 22.0, 38.92, 28.0), // TMFil.d: TRANS corner to filler chamfer
            strip45(f, 56.0, 14.0, 6.4, d),
            strip45(t, 48.0, g(14.0 + strip_dy(4.895, d, false) - 8.0), 22.4, d), // TMFil.d (4.893)
        ],
    );

    // h3 — inside and across.  A filler inside a TRANS 2.0 from its edge (the marker
    // encloses it by less than 4.9: fires, as the activ deck's AFil.e reads it), a filler
    // inside a TRANS with 7 µm all round (clean), a filler across a TRANS edge (shares
    // area, no pair: nothing).
    l.write(
        &format!("{p}.h3"),
        vec![
            rect(t, 2.0, 2.0, 22.0, 22.0),
            rect(f, 4.0, 8.0, 10.0, 14.0), // TMFil.d: 2.0 inside the left edge
            rect(t, 30.0, 2.0, 50.0, 22.0),
            rect(f, 37.0, 9.0, 43.0, 15.0), // clean, 7 all round
            rect(t, 60.0, 2.0, 70.0, 12.0),
            rect(f, 67.0, 4.0, 73.0, 10.0), // crossing: nothing
        ],
    );

    // h4 — tile lines.  Filler-to-TRANS gaps of 4.895 well inside a tile and ending on
    // x = 20, straddling 20, starting on 20, straddling 21, ending on 40, straddling 42;
    // one ending on y = 20; a corner-on pair with its corner on x = 20.  Nine gaps.
    let mut e = vec![];
    for (i, (x0, x1)) in tile_gaps(4.895).iter().enumerate() {
        let y = 2.0 + 12.0 * i as f64;
        e.push(rect(f, x0 - 6.0, y, *x0, y + 6.0));
        e.push(rect(t, *x1, y, x1 + 6.0, y + 6.0));
    }
    e.extend([
        rect(f, 60.0, 9.105, 66.0, 15.105),
        rect(t, 60.0, 20.0, 66.0, 26.0), // gap ends on y = 20
        rect(f, 14.0, 96.0, 20.0, 102.0),
        rect(t, 24.895, 102.0, 30.895, 108.0), // corner-on, corner on x = 20
    ]);
    l.write(&format!("{p}.h4"), e);

    // h7 — large and small.  A 300 µm TRANS bar 4.895 from a filler (one gap), a pair at
    // (1000, 1000).
    l.write(
        &format!("{p}.h7"),
        vec![
            rect(f, 2.0, 2.0, 8.0, 8.0),
            rect(t, 2.0, 12.895, 302.0, 18.895), // TMFil.d
            rect(f, 1000.0, 1000.0, 1006.0, 1006.0),
            rect(t, 1010.895, 1000.0, 1016.895, 1006.0), // TMFil.d
        ],
    );
}

// --- TM2.bR: min. space 5.00 of TopMetal2 lines if at least one line is wider than 5.0
// and the parallel run is more than 50.0; not checked within IND ---

/// Two vertical lines `(x0, w0)` and `(x1, w1)`, from `y0` to `y0 + len`.
fn pair(m: (i16, i16), x0: f64, w0: f64, x1: f64, w1: f64, y0: f64, len: f64) -> Vec<GdsElement> {
    vec![
        rect(m, x0, y0, g(x0 + w0), g(y0 + len)),
        rect(m, x1, y0, g(x1 + w1), g(y0 + len)),
    ]
}

fn tm2_br_h(l: &L) {
    let (m, ind) = (l.met, l.ind);
    let p = "TM2.bR";

    // h1 — the three bounds, one step past each.  (a) a 5.005 line and a 2.0 line, gap 4,
    // run 60: fires; (b) two 6 lines at a gap of 4.995, run 60: fires; (c) two 6 lines at
    // a gap of 4, run 50.005: fires; (d) a 6 line and a 2 line at a gap of 4, run 50.0:
    // clean; (e) a 5.0 line and a 2 line at a gap of 4, run 60: clean.
    let mut e = vec![];
    e.extend(pair(m, 0.0, 5.005, 9.005, 2.0, 0.0, 60.0)); // TM2.bR
    e.extend(pair(m, 20.0, 6.0, 30.995, 6.0, 0.0, 60.0)); // TM2.bR
    e.extend(pair(m, 50.0, 6.0, 60.0, 6.0, 0.0, 50.005)); // TM2.bR
    e.extend(pair(m, 80.0, 6.0, 90.0, 2.0, 0.0, 50.0)); // clean
    e.extend(pair(m, 100.0, 5.0, 109.0, 2.0, 0.0, 60.0)); // clean
    l.write(&format!("{p}.h1"), e);

    // h2 — the parallel run is the overlap.  A 6 line 100 long and a 2 line 60 long at a
    // gap of 4, the short one shifted so the two overlap by 50.0 (clean) and by 50.005
    // (fires); a 6 line 60 long and a 2 line 20 long facing its middle (run 20, clean).
    l.write(
        &format!("{p}.h2"),
        vec![
            rect(m, 0.0, 0.0, 6.0, 100.0),
            rect(m, 10.0, 50.0, 12.0, 110.0), // overlap 50.0 → clean
            rect(m, 30.0, 0.0, 36.0, 100.0),
            rect(m, 40.0, 49.995, 42.0, 109.995), // overlap 50.005 → TM2.bR
            rect(m, 60.0, 0.0, 66.0, 60.0),
            rect(m, 70.0, 20.0, 72.0, 40.0), // run 20 → clean
        ],
    );

    // h3 — the run of the lines, not of an edge.  Beside a straight 6 line 60 long: (a) a
    // line whose facing wall steps at mid-height, gap 4 for 30 µm and 4.5 for the other
    // 30 (the lines run 60 in parallel, under 5 apart throughout: fires); (b) a line whose
    // facing wall carries a 0.5 µm long, 0.005 deep nick at mid-height (fires); (c) a
    // neighbour drawn as two abutting 30 µm boxes (one 60 µm shape: fires); (d) a 6 line
    // drawn as two abutting 3 µm strips beside a 2 line (one line wider than 5: fires).
    l.write(
        &format!("{p}.h3"),
        vec![
            rect(m, 0.0, 0.0, 6.0, 60.0),
            poly(
                m,
                &[
                    (10.0, 0.0),
                    (17.0, 0.0),
                    (17.0, 60.0),
                    (10.5, 60.0),
                    (10.5, 30.0),
                    (10.0, 30.0),
                ],
            ), // TM2.bR (a)
            rect(m, 30.0, 0.0, 36.0, 60.0),
            poly(
                m,
                &[
                    (40.0, 0.0),
                    (47.0, 0.0),
                    (47.0, 60.0),
                    (40.0, 60.0),
                    (40.0, 30.25),
                    (40.005, 30.25),
                    (40.005, 29.75),
                    (40.0, 29.75),
                ],
            ), // TM2.bR (b)
            rect(m, 60.0, 0.0, 66.0, 60.0),
            rect(m, 70.0, 0.0, 76.0, 30.0),
            rect(m, 70.0, 30.0, 76.0, 60.0), // TM2.bR (c)
            rect(m, 90.0, 0.0, 93.0, 60.0),
            rect(m, 93.0, 0.0, 96.0, 60.0),
            rect(m, 100.0, 0.0, 102.0, 60.0), // TM2.bR (d)
        ],
    );

    // h4 — 45° and the figure.  Two 6-wide 45° strips (d = 4.245) running 70 µm along the
    // diagonal, 60 of it side by side, at a perpendicular gap of 3.999 (fires) and of
    // 5.003 (clean); a 60 × 60 plate and a 2 line 55 long at a gap of 4 (the manual's
    // figure: fires); a 2 line 60 long ending 4 from a wide line's flank, crossing its
    // direction (run 2: clean); a 6 line 80 long and a 6-wide 45° strip 85 long passing
    // 2.755 from it at its nearest corner (no parallel run: clean).
    let d = 4.245;
    l.write(
        &format!("{p}.h4"),
        vec![
            strip45(m, 10.0, 0.0, 50.0, d),
            strip45(m, 10.0, strip_dy(4.0, d, false), 50.0, d), // TM2.bR (3.999)
            strip45(m, 90.0, 0.0, 50.0, d),
            strip45(m, 90.0, strip_dy(5.0, d, true), 50.0, d), // clean (5.003)
            rect(m, 160.0, 0.0, 220.0, 60.0),
            rect(m, 224.0, 2.0, 226.0, 57.0), // TM2.bR: the figure
            rect(m, 240.0, 0.0, 246.0, 60.0),
            rect(m, 250.0, 28.0, 310.0, 30.0), // clean: a T, run 2
            rect(m, 330.0, 0.0, 336.0, 80.0),
            strip45(m, 343.0, 10.0, 60.0, d), // clean: 45° to the line
        ],
    );

    // h5 — IND.  Two 6 lines at a gap of 4, run 60: with IND over the top 20 µm of the
    // run the 40 outside it is under the 50 (clean); with IND over the top 5 µm the 55
    // outside fires; a wide line inside IND beside a 2 line outside it, run 60 (the wide
    // line is within IND and not checked: clean).
    let mut e = vec![];
    e.extend(pair(m, 0.0, 6.0, 10.0, 6.0, 0.0, 60.0));
    e.push(rect(ind, -2.0, 40.0, 18.0, 62.0)); // clean
    e.extend(pair(m, 30.0, 6.0, 40.0, 6.0, 0.0, 60.0));
    e.push(rect(ind, 28.0, 55.0, 48.0, 62.0)); // TM2.bR
    e.extend(pair(m, 60.0, 6.0, 70.0, 2.0, 0.0, 60.0));
    e.push(rect(ind, 58.0, -2.0, 68.0, 62.0)); // clean: the wide line is within IND
    l.write(&format!("{p}.h5"), e);

    // h6 — with TM2.b.  Two 6 lines 60 long at a gap of 1.995 break TM2.b and TM2.bR both.
    l.write(&format!("{p}.h6"), pair(m, 0.0, 6.0, 7.995, 6.0, 0.0, 60.0));

    // h9 — tile lines.  Horizontal pairs (gap 4, 6 and 2 wide) whose run of 50.005 starts
    // at x = 0, at 20, at 19.995, at 7 and at 40 (ends on, across and off the 20 and 7
    // tile lines); one whose run of 50.0 starts at 20 (clean).  Five fire.
    let mut e = vec![];
    for (i, x0) in [0.0, 20.0, 19.995, 7.0, 40.0].iter().enumerate() {
        let y = 2.0 + 20.0 * i as f64;
        e.push(rect(m, *x0, y, g(x0 + 50.005), y + 6.0));
        e.push(rect(m, *x0, y + 10.0, g(x0 + 50.005), y + 12.0));
    }
    e.push(rect(m, 20.0, 102.0, 70.0, 108.0));
    e.push(rect(m, 20.0, 112.0, 70.0, 114.0)); // clean
    l.write(&format!("{p}.h9"), e);

    // h10 — where the line is wide.  A 4 line 60 long with a 6-wide, 20 µm long bulge on
    // its far side, beside a 4 line 60 long at a gap of 4 (the wide part runs 20: clean);
    // the same bulge on the near side (gap 4 over 20, 6 over the rest: clean); a 4 line
    // with a 6-wide bulge 55 long on its far side, beside a 4 line at a gap of 4 (fires);
    // a 300 µm pair of 6 lines at a gap of 4 (one violation); a pair at (1000, 1000).
    let mut e = vec![
        poly(
            m,
            &[
                (0.0, 0.0),
                (4.0, 0.0),
                (4.0, 60.0),
                (0.0, 60.0),
                (0.0, 40.0),
                (-2.0, 40.0),
                (-2.0, 20.0),
                (0.0, 20.0),
            ],
        ),
        rect(m, 8.0, 0.0, 12.0, 60.0), // clean
        poly(
            m,
            &[
                (20.0, 0.0),
                (24.0, 0.0),
                (24.0, 20.0),
                (26.0, 20.0),
                (26.0, 40.0),
                (24.0, 40.0),
                (24.0, 60.0),
                (20.0, 60.0),
            ],
        ),
        rect(m, 30.0, 0.0, 34.0, 60.0), // clean
        poly(
            m,
            &[
                (48.0, 0.0),
                (52.0, 0.0),
                (52.0, 60.0),
                (48.0, 60.0),
                (48.0, 57.5),
                (46.0, 57.5),
                (46.0, 2.5),
                (48.0, 2.5),
            ],
        ),
        rect(m, 56.0, 0.0, 60.0, 60.0), // TM2.bR: the wide part runs 55
        rect(m, 70.0, 0.0, 370.0, 6.0),
        rect(m, 70.0, 10.0, 370.0, 16.0), // TM2.bR: 300 µm
    ];
    e.extend(pair(m, 1000.0, 6.0, 1010.0, 6.0, 1000.0, 60.0)); // TM2.bR
    l.write(&format!("{p}.h10"), e);

    // h11 — h3's split walls again with a narrow neighbour, the pairing IHP's KLayout deck
    // can see (its check is shielded on two wide lines, note C).  Beside a straight 6 line
    // 60 long: (a) a 2 line whose facing wall steps at mid-height, gap 4 for 30 µm and 4.5
    // for 30; (b) a 2.5 line whose facing wall carries a 0.5 µm long, 0.005 deep nick (a
    // TM2.b notch as well); (c) a 2 line drawn as two abutting 30 µm boxes.  Three runs of
    // 60 under 5.
    l.write(
        &format!("{p}.h11"),
        vec![
            rect(m, 0.0, 0.0, 6.0, 60.0),
            poly(
                m,
                &[
                    (10.0, 0.0),
                    (12.5, 0.0),
                    (12.5, 60.0),
                    (10.5, 60.0),
                    (10.5, 30.0),
                    (10.0, 30.0),
                ],
            ), // TM2.bR (a)
            rect(m, 30.0, 0.0, 36.0, 60.0),
            poly(
                m,
                &[
                    (40.0, 0.0),
                    (42.5, 0.0),
                    (42.5, 60.0),
                    (40.0, 60.0),
                    (40.0, 30.25),
                    (40.005, 30.25),
                    (40.005, 29.75),
                    (40.0, 29.75),
                ],
            ), // TM2.bR (b)
            rect(m, 60.0, 0.0, 66.0, 60.0),
            rect(m, 70.0, 0.0, 72.0, 30.0),
            rect(m, 70.0, 30.0, 72.0, 60.0), // TM2.bR (c)
        ],
    );
}
