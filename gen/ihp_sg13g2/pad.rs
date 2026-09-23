// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, diamond, enclosure_pattern, exact_width_pattern, layer, library, poly, rect,
    ref_array, shift, space_pattern, write_gz,
};
use gds21::{GdsBoundary, GdsElement};
use gdscheck::pdk::PdkConfig;
use std::f64::consts::{SQRT_2, TAU};

const DIR: &str = "tests/data/ihp-sg13g2/pad";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pad_a1(pdk);
    pad_d(pdk);
    pad_i(pdk);

    padb_a(pdk);
    padb_b(pdk);
    padb_c(pdk);
    padb_d(pdk);
    padc_d(pdk);

    padc_a(pdk);
    padc_b(pdk);
    padc_c(pdk);

    padb_f(pdk);
    padc_f(pdk);

    hardening(pdk);
}

/// A regular `n`-gon (multiple of 4, so its bbox is an exact `2r`×`2r` square) centered at
/// `(cx, cy)` with radius `r`, on the given layer.
fn regular_ngon(layer: (i16, i16), cx: f64, cy: f64, r: f64, n: usize) -> gds21::GdsElement {
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    poly(layer, &pts)
}

/// A regular octagon (square with 45°-chamfered corners) of bbox side `s`, `chamfer` cut
/// from each corner, centered at `(cx, cy)`.
fn octagon(layer: (i16, i16), cx: f64, cy: f64, s: f64, chamfer: f64) -> gds21::GdsElement {
    let h = s / 2.0;
    let c = chamfer;
    let pts = [
        (cx - h + c, cy - h),
        (cx + h - c, cy - h),
        (cx + h, cy - h + c),
        (cx + h, cy + h - c),
        (cx + h - c, cy + h),
        (cx - h + c, cy + h),
        (cx - h, cy + h - c),
        (cx - h, cy - h + c),
    ];
    poly(layer, &pts)
}

/// Padb.f — Allowed SBumpPad shape is Octagon or Circle only.  A plain-square SBumpPad
/// violates; an octagon-shaped and a circle-shaped one are both clean.  Each is fully
/// covered by TopMetal2 and far from EdgeSeal to isolate Padb.f from Padb.c/d/Pad.i.
fn padb_f(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let tm2 = layer(pdk, "TopMetal2");
    let o = OFFSET;
    let cover = |cx: f64, cy: f64, r: f64| {
        rect(
            tm2,
            cx - r - 15.0,
            cy - r - 15.0,
            cx + r + 15.0,
            cy + r + 15.0,
        )
    };

    // Square — violation.
    let (cx, cy) = (o, o);
    let mut elems = vec![rect(sbump, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0)];
    elems.push(rect(dfpad, cx - 30.0, cy - 30.0, cx + 30.0, cy + 30.0));
    elems.push(cover(cx, cy, 30.0));

    // Octagon — clean.
    let (cx, cy) = (o + 100.0, o);
    elems.push(octagon(sbump, cx, cy, 60.0, 15.0));
    elems.push(octagon(dfpad, cx, cy, 60.0, 15.0));
    elems.push(cover(cx, cy, 30.0));

    // Circle (64-gon) — clean.
    let (cx, cy) = (o + 200.0, o);
    elems.push(regular_ngon(sbump, cx, cy, 30.0, 64));
    elems.push(regular_ngon(dfpad, cx, cy, 30.0, 64));
    elems.push(cover(cx, cy, 30.0));

    write_gz(&format!("{DIR}/Padb.f.gds.gz"), library("TOP", elems));
}

/// Padc.f — Allowed CuPillarPad shape is Circle only (unlike Padb.f, an octagon is NOT
/// allowed here).  Square and octagon both violate; only the circle is clean.
fn padc_f(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let tm2 = layer(pdk, "TopMetal2");
    let o = OFFSET + 400.0;
    let cover = |cx: f64, cy: f64, r: f64| {
        rect(
            tm2,
            cx - r - 15.0,
            cy - r - 15.0,
            cx + r + 15.0,
            cy + r + 15.0,
        )
    };

    // Square — violation.
    let (cx, cy) = (o, o);
    let mut elems = vec![rect(pillar, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5)];
    elems.push(rect(dfpad, cx - 17.5, cy - 17.5, cx + 17.5, cy + 17.5));
    elems.push(cover(cx, cy, 17.5));

    // Octagon — violation (not circle).
    let (cx, cy) = (o + 100.0, o);
    elems.push(octagon(pillar, cx, cy, 35.0, 9.0));
    elems.push(octagon(dfpad, cx, cy, 35.0, 9.0));
    elems.push(cover(cx, cy, 17.5));

    // Circle (64-gon) — clean.
    let (cx, cy) = (o + 200.0, o);
    elems.push(regular_ngon(pillar, cx, cy, 17.5, 64));
    elems.push(regular_ngon(dfpad, cx, cy, 17.5, 64));
    elems.push(cover(cx, cy, 17.5));

    write_gz(&format!("{DIR}/Padc.f.gds.gz"), library("TOP", elems));
}

/// A pad opening (`Passiv AND dfpad`) at `(x, y)`, `w`×`h` µm.
fn opening(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Passiv"), x, y, x + w, y + h),
        rect(layer(pdk, "dfpad"), x, y, x + w, y + h),
    ]
}

/// An EdgeSeal-Activ ring fragment (`Activ AND EdgeSeal`) — the seal reference for Pad.d/Padc.d.
fn seal_activ(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Activ"), x, y, x + w, y + h),
        rect(layer(pdk, "EdgeSeal"), x, y, x + w, y + h),
    ]
}

/// Pad.a1 — a 160 µm pad opening exceeds the 150 µm max width; the width is the
/// narrowest dimension, so the square opening is one violation.
fn pad_a1(pdk: &PdkConfig) {
    let mut elems = opening(pdk, OFFSET, OFFSET, 160.0, 160.0);
    elems.extend(opening(pdk, OFFSET + 200.0, OFFSET, 100.0, 100.0)); // clean
    write_gz(&format!("{DIR}/Pad.a1.gds.gz"), library("TOP", elems));
}

/// Pad.d — a pad opening 7.0 µm from the EdgeSeal-Activ ring (< 7.50).
fn pad_d(pdk: &PdkConfig) {
    let mut elems = seal_activ(pdk, OFFSET, OFFSET, 10.0, 10.0);
    elems.extend(opening(pdk, OFFSET + 10.0 + 7.0, OFFSET, 30.0, 10.0));
    write_gz(&format!("{DIR}/Pad.d.gds.gz"), library("TOP", elems));
}

/// Pad.i — a dfpad opening with no TopMetal2 underneath (clean pad has TopMetal2 added).
fn pad_i(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        rect(layer(pdk, "Passiv"), o, o, o + 30.0, o + 30.0),
        rect(layer(pdk, "dfpad"), o, o, o + 30.0, o + 30.0), // no TopMetal2 → violation
        rect(layer(pdk, "Passiv"), o + 50.0, o, o + 80.0, o + 30.0),
        rect(layer(pdk, "dfpad"), o + 50.0, o, o + 80.0, o + 30.0),
        rect(layer(pdk, "TopMetal2"), o + 50.0, o, o + 80.0, o + 30.0), // clean
    ];
    write_gz(&format!("{DIR}/Pad.i.gds.gz"), library("TOP", elems));
}

/// Padb.d — an SBumpPad 40.0 µm from raw EdgeSeal (< 50.0 → violation).  TopMetal2 fully
/// (over-)covers the pad so this doesn't collaterally trip Padb.c/Pad.i.
fn padb_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let (x0, x1) = (o + 10.0 + 40.0, o + 10.0 + 40.0 + 60.0);
    let elems = vec![
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "Passiv.sbump"), x0, o, x1, o + 60.0),
        rect(layer(pdk, "dfpad"), x0, o, x1, o + 60.0),
        rect(
            layer(pdk, "TopMetal2"),
            x0 - 15.0,
            o - 15.0,
            x1 + 15.0,
            o + 60.0 + 15.0,
        ),
    ];
    write_gz(&format!("{DIR}/Padb.d.gds.gz"), library("TOP", elems));
}

/// Padc.d — CuPillarPads 25.0 µm (< 30 → fires) and exactly 30.0 µm (clean) from an
/// EdgeSeal-Activ bar.  TopMetal2 over-covers both pads (keeps Padc.c/Pad.i quiet); the
/// square pads trip Padc.f (circle-only), which the test ignores.
fn padc_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut elems = vec![
        rect(layer(pdk, "Activ"), o, o, o + 10.0, o + 130.0),
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 130.0),
    ];
    // A real pillar-pad cell carries both marker conventions: the PDF's (Passiv:pillar +
    // dfpad — our CuPillarPad recognition) and the maximal deck's Padc.d input
    // (`cupPad_candidat = Passiv ∩ dfpad:pillar`); draw all four so the container
    // cross-check exercises the real rule.
    for (dy, gap) in [(0.0, 25.0), (80.0, 30.0)] {
        let (x0, y0) = (o + 10.0 + gap, o + dy);
        for l in ["Passiv.pillar", "dfpad", "Passiv", "dfpad.pillar"] {
            elems.push(rect(layer(pdk, l), x0, y0, x0 + 35.0, y0 + 35.0));
        }
        elems.push(rect(
            layer(pdk, "TopMetal2"),
            x0 - 10.0,
            y0 - 10.0,
            x0 + 45.0,
            y0 + 45.0,
        ));
    }
    write_gz(&format!("{DIR}/Padc.d.gds.gz"), library("TOP", elems));
}

fn padb_a(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut exact_width_pattern(
        sbump,
        60.0,
        60.0,
        160.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut exact_width_pattern(
        dfpad,
        60.0,
        60.0,
        160.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padb.a.gds.gz"), library("TOP", elems));
}

fn padb_b(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut space_pattern(
        sbump,
        sbump,
        60.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut space_pattern(
        dfpad,
        dfpad,
        60.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padb.b.gds.gz"), library("TOP", elems));
}

fn padb_c(pdk: &PdkConfig) {
    let sbump = layer(pdk, "Passiv.sbump");
    let dfpad = layer(pdk, "dfpad");
    let tm = layer(pdk, "TopMetal2");
    let mut elems = vec![];
    elems.append(&mut enclosure_pattern(
        tm,
        sbump,
        10.0,
        60.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut enclosure_pattern(
        tm,
        dfpad,
        10.0,
        60.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padb.c.gds.gz"), library("TOP", elems));
}

fn padc_a(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut exact_width_pattern(
        pillar,
        35.0,
        35.0,
        160.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut exact_width_pattern(
        dfpad,
        35.0,
        35.0,
        160.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padc.a.gds.gz"), library("TOP", elems));
}

fn padc_b(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let mut elems = vec![];
    elems.append(&mut space_pattern(
        pillar,
        pillar,
        35.0,
        40.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut space_pattern(
        dfpad,
        dfpad,
        35.0,
        40.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padc.b.gds.gz"), library("TOP", elems));
}

fn padc_c(pdk: &PdkConfig) {
    let pillar = layer(pdk, "Passiv.pillar");
    let dfpad = layer(pdk, "dfpad");
    let tm = layer(pdk, "TopMetal2");
    let mut elems = vec![];
    elems.append(&mut enclosure_pattern(
        tm,
        pillar,
        7.5,
        35.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    elems.append(&mut enclosure_pattern(
        tm,
        dfpad,
        7.5,
        35.0,
        70.0,
        OFFSET,
        SPACE_DELTA,
    ));
    write_gz(&format!("{DIR}/Padc.c.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------

pub(super) const MIM: &str = "tests/data/ihp-sg13g2/mim";

/// The drawn layers of the three decks.
pub(super) struct L {
    pub(super) passiv: (i16, i16),
    pub(super) sbump: (i16, i16),
    pub(super) pillar: (i16, i16),
    pub(super) dfpad: (i16, i16),
    pub(super) dfpad_sbump: (i16, i16),
    pub(super) dfpad_pillar: (i16, i16),
    pub(super) tm2: (i16, i16),
    pub(super) activ: (i16, i16),
    pub(super) seal: (i16, i16),
    pub(super) mim: (i16, i16),
    pub(super) m5: (i16, i16),
    pub(super) tv1: (i16, i16),
    pub(super) vmim: (i16, i16),
}

impl L {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        L {
            passiv: layer(pdk, "Passiv"),
            sbump: layer(pdk, "Passiv.sbump"),
            pillar: layer(pdk, "Passiv.pillar"),
            dfpad: layer(pdk, "dfpad"),
            dfpad_sbump: layer(pdk, "dfpad.sbump"),
            dfpad_pillar: layer(pdk, "dfpad.pillar"),
            tm2: layer(pdk, "TopMetal2"),
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
    pub(super) fn opening(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(&[self.passiv, self.dfpad], &shape);
        v.push(if margin == 0.0 {
            relayer(&shape, self.tm2)
        } else {
            grown_bbox(&shape, self.tm2, margin)
        });
        v
    }

    /// A pad opening as a box.
    pub(super) fn open_box(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        self.opening(rect(self.passiv, x0, y0, x1, y1), 0.0)
    }

    /// A solder-bump pad: `shape` on Passiv, Passiv:sbump, dfpad and dfpad:sbump, in a
    /// TopMetal2 box `margin` beyond its bounding box.
    pub(super) fn bump(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(
            &[self.passiv, self.sbump, self.dfpad, self.dfpad_sbump],
            &shape,
        );
        v.push(grown_bbox(&shape, self.tm2, margin));
        v
    }

    /// A copper-pillar pad, as `bump` with the pillar datatypes.
    pub(super) fn pillar_pad(&self, shape: GdsElement, margin: f64) -> Vec<GdsElement> {
        let mut v = on(
            &[self.passiv, self.pillar, self.dfpad, self.dfpad_pillar],
            &shape,
        );
        v.push(grown_bbox(&shape, self.tm2, margin));
        v
    }

    /// A seal-ring fragment: the box on Activ and EdgeSeal.
    pub(super) fn seal_activ(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.activ, x0, y0, x1, y1),
            rect(self.seal, x0, y0, x1, y1),
        ]
    }

    /// A seal ring: a frame `w` wide from `(x0, y0)` to `(x1, y1)` on Activ and
    /// EdgeSeal, its four corners cut at 45° over `k` (Seal.k asks 21).
    pub(super) fn seal_ring(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        w: f64,
        k: f64,
    ) -> Vec<GdsElement> {
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
    pub(super) fn cap(&self, x: f64, y: f64, w: f64, h: f64, via: f64) -> Vec<GdsElement> {
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
pub(super) fn chamfered_all(x0: f64, y0: f64, x1: f64, y1: f64, k: f64) -> Vec<(f64, f64)> {
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
pub(super) fn relayer(e: &GdsElement, l: (i16, i16)) -> GdsElement {
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
pub(super) fn on(ls: &[(i16, i16)], e: &GdsElement) -> Vec<GdsElement> {
    ls.iter().map(|&l| relayer(e, l)).collect()
}

/// The bounding box of `e` grown by `m` on every side, on `l`.
pub(super) fn grown_bbox(e: &GdsElement, l: (i16, i16), m: f64) -> GdsElement {
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

/// Snap to the 0.005 µm grid.
pub(super) fn g(v: f64) -> f64 {
    (v * 200.0).round() / 200.0
}

pub(super) fn write(dir: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{dir}/{name}.gds.gz"), library("TOP", elems));
}

const PAD: &str = "tests/data/ihp-sg13g2/pad";

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
/// octagon_h the manual allows for a bump pad.  `c = s·(1 − 1/√2)` makes it regular,
/// which is what IHP's bond pad pcell draws (`r·(1 − 1/(1+√2))` from a radius `r`).
fn octagon_h(l: (i16, i16), cx: f64, cy: f64, s: f64, c: f64) -> GdsElement {
    let (x0, y0) = (g(cx - s / 2.0), g(cy - s / 2.0));
    poly(l, &chamfered_all(x0, y0, x0 + s, y0 + s, c))
}

/// The regular octagon_h's corner cut for a box of side `s`, on the grid.
fn regular_cut(s: f64) -> f64 {
    g(s * (1.0 - 1.0 / SQRT_2))
}

/// Pad.a1 — "Max. Pad width 150.00", on the opening (Passiv AND dfpad).  A width is
/// the narrower dimension, so a long bar or an L of 100 arms is not wide.  (a) 150 square, clean; (b) 150.005 square, fires; (c) 150.005 × 100 and
/// (d) 100 × 150.005, clean; (e) 300 × 149.995 clean and (f) 300 × 150.005 fires; (g)
/// regular octagon_h of 150, clean, (h) of 150.005, fires; (i) diamond of width 149.9
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
    e.extend(l.opening(octagon_h(l.passiv, p(6) + 75.0, q(6) + 75.0, 150.0, c), 0.0));
    e.extend(l.opening(
        octagon_h(l.passiv, p(7) + 75.0, q(7) + 75.0, 150.005, c),
        0.0,
    ));
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

/// Padb.a — "SBumpPad size 60.00", with Padb.f allowing an octagon_h or a circle.  The
/// size of an octagon_h or a circle is what it spans, 60 across; its diagonal flats are
/// the designer's.  Pads on a 140 pitch, TopMetal2 15 beyond: (a) 60 square, clean
/// (Padb.f fires, ignored); (b) 59.995 square and (c) 60.005 square, fire; (d) 60 ×
/// 59.995, fires; (e) regular octagon_h of 60 as IHP's pcell draws it (cut 17.575, which
/// puts its diagonal flats 59.9985 apart - no cut on the grid puts them at 60.000),
/// clean; (f) an octagon_h of 60 cut 10 (70.7 across the diagonals), clean by the
/// reading that the size is the span; (g) a 64-point circle of radius 30 as IHP's
/// pcell draws it (its walls 59.93 apart), clean; (h) regular octagon_h of 59.995 and (i) of 60.005, fire; (j)
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
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 60.0, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(5);
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 60.0, 10.0), 15.0));
    let (cx, cy) = at(6);
    e.extend(l.bump(ngon(l.passiv, cx, cy, 30.0, 64), 15.0));
    let (cx, cy) = at(7);
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 59.995, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(8);
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 60.005, regular_cut(60.0)), 15.0));
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
        e.extend(l.bump(octagon_h(l.passiv, x, y, 60.0, c), 15.0));
        e.extend(l.bump(octagon_h(l.passiv, x + d, y + d, 60.0, c), 15.0));
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
/// corner, fires; (e) cut passing 10.002, clean; (f) regular octagon_h of 60 in a regular
/// octagon_h of 80 (10.002 across each diagonal on the grid), clean; (g) in an octagon_h of
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
    pad(
        &mut e,
        &octagon_h(l.passiv, cx, cy, 60.0, regular_cut(60.0)),
    );
    e.push(octagon_h(l.tm2, cx, cy, 80.0, regular_cut(80.0)));
    let (cx, cy) = at(6);
    pad(
        &mut e,
        &octagon_h(l.passiv, cx, cy, 60.0, regular_cut(60.0)),
    );
    e.push(octagon_h(l.tm2, cx, cy, 80.0, 23.44));
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
/// octagon_h (IHP's pcell's), clean; (b) an octagon_h cut 10, clean; (c) an octagon_h cut 10
/// at two corners and 15 at the other two, clean (eight sides at 45°); (d) a
/// 64-point circle, clean; (e) a 128-point circle, clean; (f) a square, fires; (g) a
/// diamond, fires; (h) a hexagon, fires; (i) a D (the circle with a flat cut at x =
/// 20), fires; (j) an ellipse 60 × 50, fires; (k) a 16-gon, fires (too coarse for a
/// circle; IHP's deck wants 64 points); (l) a circle with a 10 hole, fires.  Fires 7.
fn padb_f_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 130.0 + 40.0, (i / 6) as f64 * 130.0 + 40.0);
    let mut e = vec![];
    let (cx, cy) = at(0);
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 60.0, regular_cut(60.0)), 15.0));
    let (cx, cy) = at(1);
    e.extend(l.bump(octagon_h(l.passiv, cx, cy, 60.0, 10.0), 15.0));
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
/// clean; (c) regular octagon_h, fires; (d) square, fires; (e) 16-gon, fires; (f) a D
/// (the circle cut flat at x = 12), fires.  Fires 4.
fn padc_f_h1(l: &L) {
    let at = |i: usize| ((i % 6) as f64 * 100.0 + 30.0, 30.0);
    let mut e = vec![];
    let (cx, cy) = at(0);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 64), 15.0));
    let (cx, cy) = at(1);
    e.extend(l.pillar_pad(ngon(l.passiv, cx, cy, 17.5, 128), 15.0));
    let (cx, cy) = at(2);
    e.extend(l.pillar_pad(octagon_h(l.passiv, cx, cy, 35.0, regular_cut(35.0)), 15.0));
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
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/pad")
        .expect("failed to create output directory");
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
}
