// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::psd::{L, ring, space_suite, sq, width_suite, write};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, diamond, layer, library, min_width_pattern, poly, rect, space_pattern, strip45,
    write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/tgo";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    tgo_a(pdk);
    tgo_b(pdk);
    tgo_c(pdk);
    tgo_d(pdk);
    tgo_e(pdk);
    tgo_f(pdk);

    hardening(pdk);
}

/// TGO.e — min. space between ThickGateOx regions 0.86 µm.
fn tgo_e(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let elems = space_pattern(tgo, tgo, 2.0, 0.86, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.e.gds.gz"), library("TOP", elems));
}

/// TGO.a — ThickGateOx must extend ≥ 0.27 µm over Activ.  A TGO covering an Activ with
/// 0.27 µm all round (clean) and one extending only 0.26 µm on the left (violation).
fn tgo_a(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o + 0.3, o + 0.3, o + 1.3, o + 1.3),
        rect(tgo, o + 0.03, o + 0.03, o + 1.57, o + 1.57), // 0.27 all round → clean
        rect(activ, o + 4.0, o + 4.0, o + 5.0, o + 5.0),
        rect(
            tgo,
            o + 4.0 - 0.26,
            o + 4.0 - 0.27,
            o + 5.0 + 0.27,
            o + 5.0 + 0.27,
        ), // 0.26 left → fail
    ];
    write_gz(&format!("{DIR}/TGO.a.gds.gz"), library("TOP", elems));
}

/// TGO.b — min. ThickGateOx space to Activ outside the TGO region 0.27 µm.
fn tgo_b(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let elems = space_pattern(tgo, activ, 1.0, 0.27, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.b.gds.gz"), library("TOP", elems));
}

/// A transistor (Activ + a gate crossing it) at `(x, y)`, Activ `aw`×1 µm, gate 0.4 µm
/// wide centred, extending 0.5 µm past the Activ top/bottom.
fn transistor(
    activ: (i16, i16),
    gp: (i16, i16),
    x: f64,
    y: f64,
    aw: f64,
) -> Vec<gds21::GdsElement> {
    let gx = x + aw / 2.0 - 0.2;
    vec![
        rect(activ, x, y, x + aw, y + 1.0),
        rect(gp, gx, y - 0.5, gx + 0.4, y + 1.5),
    ]
}

/// TGO.c — the ThickGateOx must extend ≥ 0.34 µm past the GatPoly sides where its edge
/// crosses the Activ (figure 5.7); an Activ ending inside the oxide has no oxide edge
/// over it, and the oxide past the Activ's end is TGO.a's.  Clean device: a centred gate
/// with wide source/drain under an oxide reaching ≥ 0.34 past each side.  Fail device:
/// the oxide's left edge crossing the Activ 0.20 µm from the gate's left side.
fn tgo_c(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let mut elems = transistor(activ, gp, o, o, 2.0);
    elems.push(rect(tgo, o - 0.34, o - 0.34, o + 2.34, o + 1.34)); // 0.8 S/D each side → clean

    // Fail device: Activ o+6.0..o+8.0, gate at o+6.8..o+7.2; the oxide starts at o+6.6,
    // 0.20 µm from the gate's left side, its edge crossing the Activ → TGO.c on that side.
    let (ax0, ax1) = (o + 6.0, o + 8.0);
    elems.push(rect(activ, ax0, o, ax1, o + 1.0));
    elems.push(rect(gp, ax0 + 0.8, o - 0.5, ax0 + 1.2, o + 1.5));
    elems.push(rect(tgo, ax0 + 0.6, o - 0.34, ax1 + 0.34, o + 1.34)); // left edge 0.20 from the gate
    write_gz(&format!("{DIR}/TGO.c.gds.gz"), library("TOP", elems));
}

/// TGO.d — min. ThickGateOx space to gate-over-channel outside the TGO region 0.34 µm.
/// A TGO and a separate transistor whose channel sits 0.34 µm away (clean) / 0.33 (fail).
fn tgo_d(pdk: &PdkConfig) {
    let tgo = layer(pdk, "ThickGateOx");
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    // Gate at the left edge of its Activ, so channel-to-TGO == Activ-to-TGO.
    let dev = |x: f64| {
        vec![
            rect(activ, x, o, x + 1.0, o + 0.5),
            rect(gp, x, o - 0.3, x + 0.16, o + 0.8), // channel left edge = x
        ]
    };
    let mut elems = vec![rect(tgo, o, o, o + 1.0, o + 1.0)];
    elems.extend(dev(o + 1.0 + 0.34)); // channel 0.34 from TGO → clean
    elems.push(rect(tgo, o + 5.0, o, o + 6.0, o + 1.0));
    elems.extend(dev(o + 6.0 + 0.33)); // channel 0.33 from TGO → violation
    write_gz(&format!("{DIR}/TGO.d.gds.gz"), library("TOP", elems));
}

fn tgo_f(pdk: &PdkConfig) {
    let l = layer(pdk, "ThickGateOx");
    let elems = min_width_pattern(l, 0.86, 0.86, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/TGO.f.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the implant decks: section 5.7 ThickGateOxide (TGO.a-TGO.f),
// section 5.10 pSD (pSD.a-pSD.n) and section 5.11 nSD:block (nSDB.a-nSDB.e) of the
// SG13G2 layout rules, with section 4.2's derivations (N+/P+ Activ by drawn nSD/pSD or
// by default, NFET/PFET, the ties).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.

impl L {
    /// A 1 × 1 Activ at `(x, y)` under a ThickGateOx with the margins `ml, mb, mr, mt`.
    fn tgo_box(&self, x: f64, y: f64, ml: f64, mb: f64, mr: f64, mt: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.0, y + 1.0),
            rect(self.tgo, x - ml, y - mb, x + 1.0 + mr, y + 1.0 + mt),
        ]
    }

    /// A 3.3 V transistor at `(x, y)`: Activ `sdl + 0.45 + sdr` × 1 with a 0.45 gate
    /// crossing it (caps 0.3), under a ThickGateOx 0.27 past the Activ all round.  TGO.c
    /// reads `sdl` and `sdr`, the Activ past the gate's sides.
    fn hv(&self, x: f64, y: f64, sdl: f64, sdr: f64) -> Vec<GdsElement> {
        let x1 = x + sdl + 0.45 + sdr;
        vec![
            rect(self.act, x, y, x1, y + 1.0),
            rect(self.gp, x + sdl, y - 0.3, x + sdl + 0.45, y + 1.3),
            rect(self.tgo, x - 0.27, y - 0.27, x1 + 0.27, y + 1.27),
        ]
    }

    /// A 3.3 V transistor whose Activ crosses both oxide edges, as figure 5.7 draws it: a
    /// 0.45 gate at `(x, y)` (caps 0.3) under a ThickGateOx `sdl` past its left side and
    /// `sdr` past its right, the Activ running 1.0 past either oxide edge.
    fn hv2(&self, x: f64, y: f64, sdl: f64, sdr: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x - sdl - 1.0, y, x + 0.45 + sdr + 1.0, y + 1.0),
            rect(self.gp, x, y - 0.3, x + 0.45, y + 1.3),
            rect(self.tgo, x - sdl, y - 0.27, x + 0.45 + sdr, y + 1.27),
        ]
    }

    /// A 1.2 V transistor at `(x, y)`: Activ 1 × 0.5 with a 0.16 gate on its left end, so
    /// the gate's left edge is the Activ's.
    fn lv(&self, x: f64, y: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.0, y + 0.5),
            rect(self.gp, x, y - 0.2, x + 0.16, y + 0.7),
        ]
    }
}

/// TGO.a, "Min. ThickGateOx extension over Activ 0.27".
fn tgo_a_h(l: &L) {
    // h1: 0.27 all round clean; 0.265 left, 0.265 top, 0.265 all round (four walls), a
    // right margin of 0 fire; an Activ crossing the oxide's edge (figure 5.7 draws one) and
    // an Activ well outside are nothing; of two Activs under one oxide the 0.265 one fires.
    let mut e = l.tgo_box(2.0, 2.0, 0.27, 0.27, 0.27, 0.27);
    e.extend(l.tgo_box(5.0, 2.0, 0.265, 0.27, 0.27, 0.27));
    e.extend(l.tgo_box(8.0, 2.0, 0.27, 0.27, 0.27, 0.265));
    e.extend(l.tgo_box(11.0, 2.0, 0.265, 0.265, 0.265, 0.265));
    e.extend(l.tgo_box(14.0, 2.0, 0.27, 0.27, 0.0, 0.27));
    e.push(rect(l.tgo, 16.0, 1.73, 17.5, 3.27));
    e.push(rect(l.act, 17.0, 2.0, 18.0, 3.0));
    e.push(rect(l.tgo, 2.0, 6.0, 3.0, 7.0));
    e.push(rect(l.act, 3.5, 6.0, 4.5, 7.0));
    e.push(rect(l.tgo, 6.0, 5.73, 9.27, 7.27));
    e.push(rect(l.act, 6.265, 6.0, 7.265, 7.0));
    e.push(rect(l.act, 8.0, 6.0, 9.0, 7.0));
    write("tgo", "TGO.a.h1", e);

    // h3: an oxide drawn as two overlapping boxes whose union leaves 0.265 fires; an Activ
    // drawn as four quadrants with 0.265 fires; an Activ in the hole of an oxide ring and
    // one crossing the ring's inner edge are nothing.
    let mut e = vec![
        rect(l.tgo, 2.0, 1.73, 3.5, 3.27),
        rect(l.tgo, 3.0, 1.73, 4.265, 3.27),
        rect(l.act, 2.27, 2.0, 4.0, 3.0),
        rect(l.act, 6.0, 2.0, 6.5, 2.5),
        rect(l.act, 6.5, 2.0, 7.0, 2.5),
        rect(l.act, 6.0, 2.5, 6.5, 3.0),
        rect(l.act, 6.5, 2.5, 7.0, 3.0),
        rect(l.tgo, 5.73, 1.73, 7.265, 3.27),
    ];
    e.extend(ring(l.tgo, 9.0, 1.0, 15.0, 7.0, 10.5, 2.5, 13.5, 5.5));
    e.push(rect(l.act, 11.5, 3.5, 12.5, 4.5));
    e.push(rect(l.act, 13.0, 3.0, 14.0, 4.0));
    write("tgo", "TGO.a.h3", e);

    // h7: a 300 µm Activ 0.265 from its oxide's top edge, and a 0.265 at (1000, 1000).
    write(
        "tgo",
        "TGO.a.h7",
        vec![
            rect(l.act, 2.0, 2.0, 302.0, 3.0),
            rect(l.tgo, 1.73, 1.73, 302.27, 3.265),
            rect(l.act, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(l.tgo, 999.735, 999.73, 1001.27, 1001.27),
        ],
    );
}

/// TGO.b, "Min. space between ThickGateOx and Activ outside thick gate oxide region 0.27".
fn tgo_b_h(l: &L) {
    // h1: 0.27 clean; 0.265 in x and y and 0.2687 corner to corner fire, 0.2758 corner to
    // corner is clean; an Activ abutting the oxide is a space of zero; an Activ in an oxide
    // ring's hole 0.265 from the inner wall, one 0.265 from a 300 µm oxide and a pair at
    // (1000, 1000) fire.
    let mut e = vec![
        sq(l.tgo, 2.0, 2.0, 2.0),
        sq(l.act, 4.27, 2.0, 1.0),
        sq(l.tgo, 7.0, 2.0, 2.0),
        sq(l.act, 9.265, 2.0, 1.0),
        sq(l.tgo, 12.0, 2.0, 2.0),
        sq(l.act, 12.0, 4.265, 1.0),
        sq(l.tgo, 16.0, 2.0, 2.0),
        sq(l.act, 18.19, 4.19, 1.0),
        sq(l.tgo, 2.0, 8.0, 2.0),
        sq(l.act, 4.195, 10.195, 1.0),
        sq(l.tgo, 7.0, 8.0, 2.0),
        sq(l.act, 9.0, 8.0, 1.0),
    ];
    e.extend(ring(l.tgo, 12.0, 8.0, 18.0, 14.0, 13.5, 9.5, 16.5, 12.5));
    e.push(sq(l.act, 13.765, 10.5, 1.0));
    e.push(rect(l.tgo, 30.0, -100.0, 31.0, 200.0));
    e.push(sq(l.act, 31.265, 2.0, 1.0));
    e.push(sq(l.tgo, 1000.0, 1000.0, 2.0));
    e.push(sq(l.act, 1002.265, 1000.0, 1.0));
    write("tgo", "TGO.b.h1", e);

    // h2: an oxide's chamfered corner passing 0.269 from an Activ's corner (the walls
    // farther) fires, at 0.276 clean; a diamond oxide's corner 0.265 from an Activ's wall
    // and a diamond Activ's corner 0.265 from an oxide's wall fire; a 45° oxide wall 0.265
    // from an Activ's corner fires.
    write(
        "tgo",
        "TGO.b.h2",
        vec![
            chamfered_tr(l.tgo, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.act, 4.14, 3.74, 1.0),
            chamfered_tr(l.tgo, 6.0, 2.0, 8.0, 4.0, 11.5),
            sq(l.act, 7.945, 3.945, 1.0),
            diamond(l.tgo, 12.0, 3.0, 1.5),
            sq(l.act, 13.765, 2.5, 1.0),
            diamond(l.act, 17.0, 3.0, 0.5),
            rect(l.tgo, 17.765, 2.0, 19.765, 4.0),
            strip45(l.tgo, 2.0, 8.0, 3.0, 1.0),
            sq(l.act, 4.0, 8.625, 1.0),
        ],
    );

    // h3: 0.265 gaps straddling x = 20, ending on it, starting on it, straddling 21, 40, 42
    // and y = 20.
    write(
        "tgo",
        "TGO.b.h3",
        vec![
            sq(l.tgo, 17.8, 2.0, 2.0),
            sq(l.act, 20.065, 2.0, 1.0),
            sq(l.tgo, 17.735, 6.0, 2.0),
            sq(l.act, 20.0, 6.0, 1.0),
            sq(l.tgo, 18.0, 10.0, 2.0),
            sq(l.act, 20.265, 10.0, 1.0),
            sq(l.tgo, 18.8, 14.0, 2.0),
            sq(l.act, 21.065, 14.0, 1.0),
            sq(l.tgo, 37.8, 2.0, 2.0),
            sq(l.act, 40.065, 2.0, 1.0),
            sq(l.tgo, 39.8, 6.0, 2.0),
            sq(l.act, 42.065, 6.0, 1.0),
            sq(l.tgo, 10.0, 17.8, 2.0),
            sq(l.act, 10.0, 20.065, 1.0),
        ],
    );
}

/// TGO.c, "Min. ThickGateOx extension over GatPoly over Activ 0.34".  Figure 5.7 draws
/// `c` from the gate's side to the oxide's edge, where that edge crosses the Activ; an
/// Activ ending inside the oxide has no oxide edge over it, only TGO.a's 0.27.
fn tgo_c_h(l: &L) {
    // h1: an Activ crossing both oxide edges with the gate 0.34 from either is clean;
    // 0.335 left, right and both fire; an Activ ending 0.335 past the gate inside an oxide
    // 0.27 past it (the oxide's edge 0.605 from the gate) and a gate ending 0.335 inside
    // such an Activ are clean; a gate the oxide's edge cuts through fires (TGO.c on its
    // inside part, TGO.d on its outside part abutting the oxide); a poly over field under
    // the oxide is nothing.
    let mut e = l.hv2(2.34, 2.0, 0.34, 0.34);
    e.extend(l.hv2(6.34, 2.0, 0.335, 0.34));
    e.extend(l.hv2(10.34, 2.0, 0.34, 0.335));
    e.extend(l.hv2(14.34, 2.0, 0.335, 0.335));
    e.extend(l.hv(2.0, 6.0, 0.335, 0.335));
    e.push(rect(l.act, 6.0, 6.0, 7.13, 7.0));
    e.push(rect(l.gp, 6.34, 5.7, 6.79, 6.665));
    e.push(rect(l.tgo, 5.73, 5.73, 7.4, 7.27));
    e.push(rect(l.act, 10.0, 6.0, 12.0, 7.0));
    e.push(rect(l.gp, 10.775, 5.7, 11.225, 7.3));
    e.push(rect(l.tgo, 9.73, 5.73, 11.0, 7.27));
    e.push(rect(l.tgo, 14.0, 5.5, 16.0, 7.5));
    e.push(rect(l.gp, 14.5, 5.7, 14.95, 7.3));
    write("tgo", "TGO.c.h1", e);

    // h2: a 45° oxide edge crossing the Activ, 0.336 from the gate's corner where the
    // perpendicular lands below the Activ (0.475 along the Activ's edge), fires under the
    // closest-approach reading; an oxide and an Activ each drawn as two boxes, the union's
    // oxide edge 0.335 from the gate, fire.
    let mut e = vec![
        rect(l.act, 1.0, 2.0, 5.0, 3.0),
        rect(l.gp, 2.0, 1.7, 2.45, 3.3),
        poly(
            l.tgo,
            &[
                (1.5, 1.73),
                (2.655, 1.73),
                (4.195, 3.27),
                (6.0, 3.27),
                (6.0, 4.5),
                (1.5, 4.5),
            ],
        ),
    ];
    e.push(rect(l.act, 7.0, 2.0, 9.0, 3.0));
    e.push(rect(l.act, 8.5, 2.0, 11.0, 3.0));
    e.push(rect(l.gp, 8.0, 1.7, 8.45, 3.3));
    e.push(rect(l.tgo, 7.5, 1.73, 8.6, 3.27));
    e.push(rect(l.tgo, 8.4, 1.73, 8.785, 3.27));
    write("tgo", "TGO.c.h2", e);
}

/// TGO.d, "Min. space between ThickGateOx and GatPoly over Activ outside thick gate oxide
/// region 0.34".
fn tgo_d_h(l: &L) {
    // h1: 0.34 clean; 0.335 in x, 0.335 with the Activ's S/D 0.1 from the oxide (a TGO.b,
    // set aside), 0.335 from the gate's width edge (the poly's cap nearer), 0.339 corner
    // to corner, a gate abutting the oxide (a TGO.b too, set aside) and 0.335 at
    // (1000, 1000) fire; 0.3465 corner to corner is clean.
    let mut e = vec![sq(l.tgo, 2.0, 2.0, 1.0)];
    e.extend(l.lv(3.34, 2.0));
    e.push(sq(l.tgo, 6.0, 2.0, 1.0));
    e.extend(l.lv(7.335, 2.0));
    e.push(sq(l.tgo, 10.0, 2.0, 1.0));
    e.push(rect(l.act, 11.1, 2.0, 12.5, 2.5));
    e.push(rect(l.gp, 11.335, 1.8, 11.495, 2.7));
    e.push(sq(l.tgo, 14.0, 2.0, 1.0));
    e.push(rect(l.act, 14.0, 3.335, 15.0, 3.835));
    e.push(rect(l.gp, 14.4, 3.135, 14.56, 4.035));
    e.push(sq(l.tgo, 2.0, 6.0, 1.0));
    e.extend(l.lv(3.24, 7.24));
    e.push(sq(l.tgo, 6.0, 6.0, 1.0));
    e.extend(l.lv(7.245, 7.245));
    e.push(sq(l.tgo, 10.0, 6.0, 1.0));
    e.extend(l.lv(11.0, 6.0));
    e.push(sq(l.tgo, 1000.0, 1000.0, 1.0));
    e.extend(l.lv(1001.335, 1000.0));
    write("tgo", "TGO.d.h1", e);

    // h2: a diamond oxide's corner 0.335 from a gate's edge fires; an oxide's chamfered
    // corner 0.335 from a gate's corner fires.
    let mut e = vec![diamond(l.tgo, 3.0, 3.0, 1.0)];
    e.extend(l.lv(4.335, 2.75));
    e.push(chamfered_tr(l.tgo, 7.0, 2.0, 9.0, 4.0, 12.5));
    e.extend(l.lv(8.985, 3.985));
    write("tgo", "TGO.d.h2", e);

    // h3: 0.335 gaps straddling x = 20, ending on it, straddling 21, 40 and 42.
    let mut e = vec![sq(l.tgo, 18.8, 2.0, 1.0)];
    e.extend(l.lv(20.135, 2.0));
    e.push(sq(l.tgo, 18.665, 6.0, 1.0));
    e.extend(l.lv(20.0, 6.0));
    e.push(sq(l.tgo, 19.8, 10.0, 1.0));
    e.extend(l.lv(21.135, 10.0));
    e.push(sq(l.tgo, 38.8, 2.0, 1.0));
    e.extend(l.lv(40.135, 2.0));
    e.push(sq(l.tgo, 40.8, 6.0, 1.0));
    e.extend(l.lv(42.135, 6.0));
    write("tgo", "TGO.d.h3", e);

    // h4/h5: fifty 0.335 gaps, flat and as an array.
    let mut cell = vec![sq(l.tgo, 0.0, 0.0, 1.0)];
    cell.extend(l.lv(1.335, 0.25));
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/tgo")
        .expect("failed to create output directory");
    let l = L::new(pdk);
    width_suite("tgo", "TGO.f", l.tgo, 0.86);
    space_suite("tgo", "TGO.e", l.tgo, 0.86, 2.0);
    tgo_a_h(&l);
    tgo_b_h(&l);
    tgo_c_h(&l);
    tgo_d_h(&l);
}
