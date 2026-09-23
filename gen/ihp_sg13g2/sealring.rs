// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{diamond, layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/sealring";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    seal_ef(pdk);
    seal_b(pdk);
    seal_d(pdk);

    hardening(pdk);
}

/// Seal.d — the EdgeSeal-Activ ring must enclose each EdgeSeal-via ring by 1.30.  Two
/// seal frames (EdgeSeal + coincident Activ, 4.0 µm wide): the first has its Cont ring's
/// outer edge only 0.80 from the frame's outer edge (< 1.30 → fires); the second at 1.50
/// (clean).  Both Activ rings are clipped to EdgeSeal, so their coincident frame edges
/// exercise the skip_coincident path.
fn seal_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e: Vec<GdsElement> = Vec::new();
    for (dx, inset) in [(0.0, 0.80), (60.0, 1.50)] {
        let (x0, y0, x1, y1) = (o + dx, o, o + dx + 40.0, o + 40.0);
        e.extend(ring(pdk, "EdgeSeal", x0, y0, x1, y1, 4.0));
        e.extend(ring(pdk, "Activ", x0, y0, x1, y1, 4.0));
        e.extend(ring(
            pdk,
            "Cont",
            x0 + inset,
            y0 + inset,
            x1 - inset,
            y1 - inset,
            0.16,
        ));
    }
    write_gz(&format!("{DIR}/Seal.d.gds.gz"), library("TOP", e));
}

/// A rectangular ring (frame with a hole) on `layer_name`, as four overlapping strips.
fn ring(
    pdk: &PdkConfig,
    name: &str,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    w: f64,
) -> Vec<GdsElement> {
    let l = layer(pdk, name);
    vec![
        rect(l, x0, y0, x1, y0 + w), // bottom
        rect(l, x0, y1 - w, x1, y1), // top
        rect(l, x0, y0, x0 + w, y1), // left
        rect(l, x1 - w, y0, x1, y1), // right
    ]
}

/// A passivation ring outside the seal: frame 3.0 µm wide (< 4.20 → Seal.e), with its
/// left edge 0.5 µm from a seal-Activ ring (< 1.00 → Seal.f.Activ).
fn seal_ef(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = ring(pdk, "Passiv", o + 5.0, o, o + 25.0, o + 20.0, 3.0);
    // A seal-Activ bar (Activ ∩ EdgeSeal) 0.5 µm left of the ring's outer edge (o+5).
    let sx1 = o + 4.5;
    e.push(rect(layer(pdk, "Activ"), o, o, sx1, o + 20.0));
    e.push(rect(layer(pdk, "EdgeSeal"), o, o, sx1, o + 20.0));
    write_gz(&format!("{DIR}/Seal.ef.gds.gz"), library("TOP", e));
}

/// Seal.b — always "circuit Activ space to EdgeSeal-<conductor>", never <conductor> itself
/// (the KLayout body's first `.sep` argument is always `activ_drw`).  Two blocks:
/// - a seal-Activ block (Activ ∩ EdgeSeal) with a circuit Activ 2.0 µm away (< 4.90 →
///   violation) and a far one (10.0 µm, clean) — confirms the ring's own Activ doesn't
///   self-flag (SealActiv ⊆ Activ; min_space skips overlapping A/B pairs).
/// - a seal-Metal1 block (Metal1 ∩ EdgeSeal) with a circuit **Activ** (not Metal1) 2.0 µm
///   away (< 4.90 → violation), exercising one of the cross-layer Seal<X> variants.
fn seal_b(pdk: &PdkConfig) {
    let o = OFFSET + 40.0;
    let mut e = vec![
        rect(layer(pdk, "Activ"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "EdgeSeal"), o, o, o + 10.0, o + 10.0),
        rect(layer(pdk, "Activ"), o + 12.0, o, o + 14.0, o + 10.0), // gap 2.0 → violation
        rect(layer(pdk, "Activ"), o + 20.0, o, o + 22.0, o + 10.0), // gap 10.0 → clean
    ];
    let om = o + 30.0;
    e.push(rect(layer(pdk, "Metal1"), om, o, om + 10.0, o + 10.0));
    e.push(rect(layer(pdk, "EdgeSeal"), om, o, om + 10.0, o + 10.0));
    e.push(rect(layer(pdk, "Activ"), om + 12.0, o, om + 14.0, o + 10.0)); // gap 2.0 → violation
    write_gz(&format!("{DIR}/Seal.b.gds.gz"), library("TOP", e));
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

pub(super) const SQRT2: f64 = std::f64::consts::SQRT_2;

/// One grid step.
pub(super) const G: f64 = 0.005;

pub(super) fn grid(v: f64) -> f64 {
    (v / G).round() * G
}

/// Every layer the four decks draw.
pub(super) struct P {
    pub(super) activ: (i16, i16),
    pub(super) psd: (i16, i16),
    pub(super) nwell: (i16, i16),
    pub(super) pwb: (i16, i16),
    pub(super) gp: (i16, i16),
    pub(super) cont: (i16, i16),
    pub(super) m1: (i16, i16),
    pub(super) m2: (i16, i16),
    pub(super) m3: (i16, i16),
    pub(super) m4: (i16, i16),
    pub(super) m5: (i16, i16),
    pub(super) tm1: (i16, i16),
    pub(super) tm2: (i16, i16),
    pub(super) via1: (i16, i16),
    pub(super) via2: (i16, i16),
    pub(super) via3: (i16, i16),
    pub(super) via4: (i16, i16),
    pub(super) tv1: (i16, i16),
    pub(super) tv2: (i16, i16),
    pub(super) m1s: (i16, i16),
    pub(super) m2s: (i16, i16),
    pub(super) m3s: (i16, i16),
    pub(super) m5s: (i16, i16),
    pub(super) tm1s: (i16, i16),
    pub(super) tm2s: (i16, i16),
    pub(super) mim: (i16, i16),
    pub(super) ind: (i16, i16),
    pub(super) passiv: (i16, i16),
    pub(super) dfpad: (i16, i16),
    pub(super) seal: (i16, i16),
    pub(super) bnd: (i16, i16),
    pub(super) lbe: (i16, i16),
}

impl P {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        let l = |n: &str| layer(pdk, n);
        P {
            activ: l("Activ"),
            psd: l("pSD"),
            nwell: l("NWell"),
            pwb: l("PWell.block"),
            gp: l("GatPoly"),
            cont: l("Cont"),
            m1: l("Metal1"),
            m2: l("Metal2"),
            m3: l("Metal3"),
            m4: l("Metal4"),
            m5: l("Metal5"),
            tm1: l("TopMetal1"),
            tm2: l("TopMetal2"),
            via1: l("Via1"),
            via2: l("Via2"),
            via3: l("Via3"),
            via4: l("Via4"),
            tv1: l("TopVia1"),
            tv2: l("TopVia2"),
            m1s: l("Metal1.slit"),
            m2s: l("Metal2.slit"),
            m3s: l("Metal3.slit"),
            m5s: l("Metal5.slit"),
            tm1s: l("TopMetal1.slit"),
            tm2s: l("TopMetal2.slit"),
            mim: l("MIM"),
            ind: l("IND"),
            passiv: l("Passiv"),
            dfpad: l("dfpad"),
            seal: l("EdgeSeal"),
            bnd: l("EdgeSeal.boundary"),
            lbe: l("LBE"),
        }
    }

    /// The nine conductors of Seal.a, Seal.b and Seal.f.
    pub(super) fn conductors(&self) -> [(i16, i16); 9] {
        [
            self.activ, self.psd, self.m1, self.m2, self.m3, self.m4, self.m5, self.tm1, self.tm2,
        ]
    }
}

pub(super) fn write(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(
        &format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}

/// A rectangular ring on `l`: the box `(x0, y0)-(x1, y1)` less its hole `w` in, as four
/// overlapping walls.
pub(super) fn frame(l: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<GdsElement> {
    vec![
        rect(l, x0, y0, x1, y0 + w),
        rect(l, x0, y1 - w, x1, y1),
        rect(l, x0, y0, x0 + w, y1),
        rect(l, x1 - w, y0, x1, y1),
    ]
}

/// The same ring as one polygon with a hole: the outline runs in along a zero-width
/// cut from the left wall's middle, round the hole and back.
pub(super) fn frame_poly(l: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> GdsElement {
    let ym = grid((y0 + y1) / 2.0);
    poly(
        l,
        &[
            (x0, ym),
            (x0, y0),
            (x1, y0),
            (x1, y1),
            (x0, y1),
            (x0, ym),
            (x0 + w, ym),
            (x0 + w, y1 - w),
            (x1 - w, y1 - w),
            (x1 - w, y0 + w),
            (x0 + w, y0 + w),
            (x0 + w, ym),
        ],
    )
}

/// The box `(x0, y0)-(x1, y1)` with its four corners cut along 45° lines `k` from
/// the corner.
pub(super) fn octagon(x0: f64, y0: f64, x1: f64, y1: f64, k: f64) -> Vec<(f64, f64)> {
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

/// A ring with 45° corners on `l`, as one polygon with a hole: the outer octagon of
/// the box `(x0, y0)-(x1, y1)` cut `k` from the corners, the hole `w` in from the
/// straight walls and `d` in from the diagonal ones (`d = w` for a ring of one width;
/// on the grid, `w·√2` is 5.94 for 4.2 and 4.95 for 3.5).
#[allow(clippy::too_many_arguments)]
pub(super) fn oct_frame(
    l: (i16, i16),
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    k: f64,
    w: f64,
    d: f64,
) -> GdsElement {
    // The inner chamfer leg: the outer line x + y = c moves in by d·√2 along x + y.
    let ki = grid(k + d * SQRT2 - 2.0 * w);
    let outer = octagon(x0, y0, x1, y1, k);
    let inner = octagon(x0 + w, y0 + w, x1 - w, y1 - w, ki);
    let ym = grid((y0 + y1) / 2.0);
    let mut pts = vec![(x0, ym)];
    pts.extend(outer.iter().skip(7).chain(outer.iter().take(7)));
    pts.push((x0, ym));
    pts.push((x0 + w, ym));
    pts.extend(inner.iter().take(7).rev().chain(inner.iter().skip(7).rev()));
    pts.push((x0 + w, ym));
    poly(l, &pts)
}

/// A seal frame as the pcell draws it: the EdgeSeal marker and every layer in `layers`
/// on the same `w`-wide ring of the box `(x0, y0)-(x1, y1)`.
pub(super) fn seal(
    p: &P,
    layers: &[(i16, i16)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    w: f64,
) -> Vec<GdsElement> {
    let mut e = frame(p.seal, x0, y0, x1, y1, w);
    for &l in layers {
        e.extend(frame(l, x0, y0, x1, y1, w));
    }
    e
}

/// A via ring `t` wide inside the frame of the box `(x0, y0)-(x1, y1)`, its outer edge
/// `inset` in from the frame's outer edge - four overlapping strips, as the pcell.
fn via_ring(
    l: (i16, i16),
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    inset: f64,
    t: f64,
) -> Vec<GdsElement> {
    frame(l, x0 + inset, y0 + inset, x1 - inset, y1 - inset, t)
}

/// The Passiv ring of a seal frame: `gap` outside the frame's box, `w` wide.
fn passiv_ring(p: &P, x0: f64, y0: f64, x1: f64, y1: f64, gap: f64, w: f64) -> Vec<GdsElement> {
    frame(
        p.passiv,
        x0 - gap - w,
        y0 - gap - w,
        x1 + gap + w,
        y1 + gap + w,
        w,
    )
}

/// h1 - the boundary.  A seal frame (EdgeSeal + Metal1, 4.2 wide, box (10, 10)-(50, 50))
/// with its Passiv ring 3 outside and the EdgeSeal.boundary at (0, 0)-(60, 60), as the
/// pcell draws it.  Figure 6.12 draws the "Sealring boundary" outside the EdgeSeal ring,
/// 30 µm away in the text; Seal.l forbids structures outside that boundary.  (a) a
/// Metal1 square between the ring and the boundary is inside the boundary: clean; (b) a
/// Metal1 square beyond x = 60 fires; (c) a Metal2 bar crossing the boundary (its part
/// outside is outside) fires; (d) a Passiv square outside is passivation, which the rule
/// lets be; (e) an Activ square outside fires (and is 20 from any seal conductor).
///
/// h2 - no boundary drawn.  The same frame and ring with no EdgeSeal.boundary and a
/// Metal1 square 5 outside the ring: with no boundary there is nothing to be outside of
/// (IHP's deck skips the rule); a block without a boundary is no chip.
fn seal_l(p: &P) {
    let mut e = seal(p, &[p.m1], 10.0, 10.0, 50.0, 50.0, 4.2);
    e.extend(passiv_ring(p, 10.0, 10.0, 50.0, 50.0, 3.0, 4.2));
    let ring = e.clone();
    e.push(rect(p.bnd, 0.0, 0.0, 60.0, 60.0));
    e.push(rect(p.m1, 52.0, 2.0, 55.0, 5.0)); // (a) inside the boundary
    e.push(rect(p.m1, 65.0, 10.0, 68.0, 13.0)); // (b) outside
    e.push(rect(p.m2, 55.0, 30.0, 65.0, 32.0)); // (c) crossing x = 60
    e.push(rect(p.passiv, 65.0, 40.0, 70.0, 45.0)); // (d) Passiv, allowed
    e.push(rect(p.activ, 65.0, 50.0, 68.0, 53.0)); // (e) Activ outside
    write("sealring", "Seal.l.h1", e);

    let mut e = ring;
    e.push(rect(p.m1, 65.0, 10.0, 68.0, 13.0));
    write("sealring", "Seal.l.h2", e);
}

/// h1 - a seal ring as IHP's pcell places it, whole, on every layer: the 4.2 EdgeSeal
/// marker on the box (32.2, 32.2)-(117.8, 117.8) carrying Activ, pSD, Metal1-5 and both
/// TopMetals, the via rings 2.0 in (Cont 0.16, Via1-4 0.19, TopVia1 0.42, TopVia2 0.9),
/// the Passiv ring 3 outside and 4.2 wide, and the boundary 30 past the marker at
/// (0, 0)-(150, 150): the whole sealring deck is clean on it.  h2 - the same ring with
/// two Metal1 squares beyond the boundary (Seal.l, 2) and a 1.0 break in the Passiv
/// ring's right wall (Seal.n).
fn seal_pcell(p: &P) {
    let (x0, y0, x1, y1) = (32.2, 32.2, 117.8, 117.8);
    let ring = |broken: bool| -> Vec<GdsElement> {
        let mut e = vec![rect(p.bnd, 0.0, 0.0, 150.0, 150.0)];
        e.extend(seal(
            p,
            &[p.activ, p.psd, p.m1, p.m2, p.m3, p.m4, p.m5, p.tm1, p.tm2],
            x0,
            y0,
            x1,
            y1,
            4.2,
        ));
        for &(l, t) in &[
            (p.cont, 0.16),
            (p.via1, 0.19),
            (p.via2, 0.19),
            (p.via3, 0.19),
            (p.via4, 0.19),
            (p.tv1, 0.42),
            (p.tv2, 0.9),
        ] {
            e.extend(via_ring(l, x0, y0, x1, y1, 2.0, t));
        }
        if broken {
            // The ring's three whole walls and its right wall in two pieces, 1.0 apart.
            let (px0, py0, px1, py1) = (x0 - 7.2, y0 - 7.2, x1 + 7.2, y1 + 7.2);
            e.push(rect(p.passiv, px0, py0, px1, py0 + 4.2));
            e.push(rect(p.passiv, px0, py1 - 4.2, px1, py1));
            e.push(rect(p.passiv, px0, py0, px0 + 4.2, py1));
            e.push(rect(p.passiv, px1 - 4.2, py0, px1, 74.5));
            e.push(rect(p.passiv, px1 - 4.2, 75.5, px1, py1));
        } else {
            e.extend(passiv_ring(p, x0, y0, x1, y1, 3.0, 4.2));
        }
        e
    };
    write("sealring", "Seal.pcell.h1", ring(false));
    let mut e = ring(true);
    e.push(rect(p.m1, -9.8, 49.0, -7.8, 51.0));
    e.push(rect(p.m1, -9.8, 53.2, -7.8, 55.2));
    write("sealring", "Seal.pcell.h2", e);
}

/// h1 - the ring.  Seal frames (EdgeSeal + Metal1, 4.2 wide, 40 boxes) each with its
/// Passiv ring 3 outside, 4.2 wide: (a) whole, clean; (b) with a 1.0 break in its right
/// wall at y = 30: fires; (c) drawn as two abutting U's (left and right halves sharing
/// x = the box's middle), one ring after merging: clean; (d) no Passiv at all: fires;
/// (e) the Passiv ring inside the seal's hole instead of outside: fires - it does not
/// enclose the seal.  The boundary is drawn round everything, so Seal.l has nothing to
/// say.
fn seal_n(p: &P) {
    let mut e = vec![rect(p.bnd, 0.0, 0.0, 300.0, 120.0)];
    let bx = |i: f64| 10.0 + i * 60.0;
    // (a)
    e.extend(seal(p, &[p.m1], bx(0.0), 10.0, bx(0.0) + 40.0, 50.0, 4.2));
    e.extend(passiv_ring(
        p,
        bx(0.0),
        10.0,
        bx(0.0) + 40.0,
        50.0,
        3.0,
        4.2,
    ));
    // (b) a break
    let x0 = bx(1.0);
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    let (px0, py0, px1, py1) = (x0 - 7.2, 2.8, x0 + 47.2, 57.2);
    e.push(rect(p.passiv, px0, py0, px1, py0 + 4.2));
    e.push(rect(p.passiv, px0, py1 - 4.2, px1, py1));
    e.push(rect(p.passiv, px0, py0, px0 + 4.2, py1));
    e.push(rect(p.passiv, px1 - 4.2, py0, px1, 29.5));
    e.push(rect(p.passiv, px1 - 4.2, 30.5, px1, py1));
    // (c) two U's
    let x0 = bx(2.0);
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    let (px0, py0, px1, py1) = (x0 - 7.2, 2.8, x0 + 47.2, 57.2);
    let xm = x0 + 20.0;
    e.push(poly(
        p.passiv,
        &[
            (px0, py0),
            (xm, py0),
            (xm, py0 + 4.2),
            (px0 + 4.2, py0 + 4.2),
            (px0 + 4.2, py1 - 4.2),
            (xm, py1 - 4.2),
            (xm, py1),
            (px0, py1),
        ],
    ));
    e.push(poly(
        p.passiv,
        &[
            (xm, py0),
            (px1, py0),
            (px1, py1),
            (xm, py1),
            (xm, py1 - 4.2),
            (px1 - 4.2, py1 - 4.2),
            (px1 - 4.2, py0 + 4.2),
            (xm, py0 + 4.2),
        ],
    ));
    // (d) no Passiv
    let x0 = bx(3.0);
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    // (e) the Passiv ring inside the hole
    let x0 = bx(4.0);
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    e.extend(frame(p.passiv, x0 + 7.2, 17.2, x0 + 32.8, 42.8, 4.2));
    write("sealring", "Seal.n.h1", e);
}

/// h1 - the bound, per conductor.  Seal frames of 20 boxes, the EdgeSeal coincident with
/// the conductor: (a) all nine conductors 3.5 wide, clean; (b) Metal1 3.495 fires; (c)
/// Activ 3.495; (d) TopMetal2 3.495; (e) pSD 3.495; (f) Metal3 3.495; (g) TopMetal1
/// 3.495; (h) Metal1 3.5 wide under an EdgeSeal 3.495 wide - EdgeSeal-Metal1 is the
/// metal inside the marker, 3.495: fires; (i) EdgeSeal 3.5 wide over a Metal1 ring 10
/// wide: the marker cuts 3.5 of it, clean; (j) a 0.5 Metal1 wire from the hole running
/// through frame (a)'s left wall: its part inside the marker lies within the ring's
/// own metal, EdgeSeal-Metal1 is unchanged, clean; (k) a 0.5 × 3 Metal1 bar lying in
/// the marker of a frame that carries Activ only (box (275, 5)-(295, 25)): an
/// EdgeSeal-Metal1 0.5 wide, fires.  Seal.l and Seal.n are the several frames'
/// collateral.
fn seal_a(p: &P) {
    let bx = |i: f64| 5.0 + i * 30.0;
    let mut e = seal(p, &p.conductors(), bx(0.0), 5.0, bx(0.0) + 20.0, 25.0, 3.5); // (a)
    for (i, l) in [p.m1, p.activ, p.tm2, p.psd, p.m3, p.tm1]
        .into_iter()
        .enumerate()
    {
        let x0 = bx(i as f64 + 1.0);
        e.extend(seal(p, &[l], x0, 5.0, x0 + 20.0, 25.0, 3.495)); // (b)-(g)
    }
    let x0 = bx(7.0); // (h)
    e.extend(frame(p.seal, x0, 5.0, x0 + 20.0, 25.0, 3.495));
    e.extend(frame(p.m1, x0, 5.0, x0 + 20.0, 25.0, 3.5));
    let x0 = bx(8.0); // (i)
    e.extend(frame(p.seal, x0, 5.0, x0 + 20.0, 25.0, 3.5));
    e.extend(frame(p.m1, x0, 5.0, x0 + 20.0, 25.0, 10.0));
    e.push(rect(p.m1, bx(0.0) + 3.0, 14.75, bx(0.0) + 12.0, 15.25)); // (j) x 8..17
    let x0 = bx(9.0); // (k)
    e.extend(seal(p, &[p.activ], x0, 5.0, x0 + 20.0, 25.0, 3.5));
    e.push(rect(p.m1, x0 + 1.0, 12.0, x0 + 1.5, 15.0));
    write("sealring", "Seal.a.h1", e);

    // h2 - geometry, Metal1 under a coincident EdgeSeal.  (a) a ring with 45° corners
    // (legs 10), 3.5 wide across the straight walls and 3.5003 across the diagonal ones
    // (the inner chamfer 4.95 = 3.5·√2 along x + y): clean; (b) the same with the
    // diagonal walls 3.493 apart (4.94): four diagonal width violations; (c) the ring as
    // one polygon with a hole, 3.495 wide: as (b)-(g) of h1; (d) the ring as sixteen
    // boxes 3.495 wide, five per wall: one ring after merging, the same; (e) a 3.5 ring
    // with a 0.2 × 0.2 nick in its outer right wall: 3.3 there, fires once; (f) the
    // pcell's staircase corner: 3.5 × 7 boxes stepped by 3.5 - a band 3.5 wide in x and
    // y, clean; (g) a 3.495 ring across the tile lines: box (16, 40)-(44, 60), its left
    // wall across x = 20, its right wall across x = 40; (h) a 3.495 ring whose right wall
    // ends on x = 100; (i) a 3.495 ring at (1000, 1000).
    let mut e = vec![];
    for (i, d) in [(0.0, 3.5), (1.0, grid(4.94 / SQRT2))].into_iter() {
        let x0 = 5.0 + i * 40.0;
        e.push(oct_frame(p.seal, x0, 5.0, x0 + 30.0, 35.0, 10.0, 3.5, d));
        e.push(oct_frame(p.m1, x0, 5.0, x0 + 30.0, 35.0, 10.0, 3.5, d));
    }
    let x0 = 85.0; // (c)
    e.push(frame_poly(p.seal, x0, 5.0, x0 + 20.0, 25.0, 3.495));
    e.push(frame_poly(p.m1, x0, 5.0, x0 + 20.0, 25.0, 3.495));
    let x0 = 115.0; // (d)
    for l in [p.seal, p.m1] {
        for i in 0..5 {
            let s = i as f64 * 4.0;
            e.push(rect(l, x0 + s, 5.0, x0 + s + 4.0, 8.495));
            e.push(rect(l, x0 + s, 21.505, x0 + s + 4.0, 25.0));
            e.push(rect(l, x0, 5.0 + s, x0 + 3.495, 9.0 + s));
            e.push(rect(l, x0 + 16.505, 5.0 + s, x0 + 20.0, 9.0 + s));
        }
    }
    let x0 = 145.0; // (e): three walls as boxes, the right wall with its nick
    for l in [p.seal, p.m1] {
        e.push(rect(l, x0, 5.0, x0 + 20.0, 8.5));
        e.push(rect(l, x0, 21.5, x0 + 20.0, 25.0));
        e.push(rect(l, x0, 5.0, x0 + 3.5, 25.0));
        e.push(poly(
            l,
            &[
                (x0 + 16.5, 5.0),
                (x0 + 20.0, 5.0),
                (x0 + 20.0, 14.9),
                (x0 + 19.8, 14.9),
                (x0 + 19.8, 15.1),
                (x0 + 20.0, 15.1),
                (x0 + 20.0, 25.0),
                (x0 + 16.5, 25.0),
            ],
        ));
    }
    let x0 = 175.0; // (f): a staircase band from (x0, 5) up-left, five steps
    for l in [p.seal, p.m1] {
        for i in 0..5 {
            let s = i as f64 * 3.5;
            e.push(rect(
                l,
                x0 + 17.5 - s - 3.5,
                5.0 + s,
                x0 + 17.5 - s,
                12.0 + s,
            ));
        }
    }
    e.extend(seal(p, &[p.m1], 16.0, 40.0, 44.0, 60.0, 3.495)); // (g)
    e.extend(seal(p, &[p.m1], 80.0, 40.0, 100.0, 60.0, 3.495)); // (h)
    e.extend(seal(p, &[p.m1], 1000.0, 1000.0, 1020.0, 1020.0, 3.495)); // (i)
    write("sealring", "Seal.a.h2", e);
}

/// h1 - the bound and both metrics.  A seal frame (EdgeSeal + Metal1, 4.2 wide, box
/// (10, 10)-(70, 70)) with circuit Activ squares (2 × 2): (a) 4.9 from the left inner
/// wall, clean; (b) 4.895 from the bottom inner wall, fires; (c) corner to corner from
/// the top-right outer corner, dx = dy = 3.46 (4.893): fires; (d) dx = dy = 3.47 (4.907)
/// from the bottom-right outer corner: clean; (e) dx = 4.5, dy = 10 from the top-right
/// outer corner, under the value on one axis alone: clean; (f) 4.895 outside the right
/// wall: the rule does not say inside, fires.  A second frame (EdgeSeal + Activ + pSD,
/// box (100, 10)-(140, 50)): (g) an Activ 4.895 from its inner wall is 4.895 from
/// EdgeSeal-Activ and from EdgeSeal-pSD, twice; (h) an Activ bar running from the hole
/// into the frame's top wall: its part inside the marker is seal Activ, and the whole is
/// 0 from the frame's conductors - fires.
fn seal_b_h(p: &P) {
    let mut e = seal(p, &[p.m1], 10.0, 10.0, 70.0, 70.0, 4.2);
    e.push(rect(p.activ, 19.1, 30.0, 21.1, 32.0)); // (a) 4.9
    e.push(rect(p.activ, 40.0, 19.095, 42.0, 21.095)); // (b) 4.895
    e.push(rect(p.activ, 73.46, 73.46, 75.46, 75.46)); // (c) 3.46 / 3.46
    e.push(rect(p.activ, 73.47, 4.53, 75.47, 6.53)); // (d) 3.47 / 3.47
    e.push(rect(p.activ, 74.5, 80.0, 76.5, 82.0)); // (e) 4.5 / 10
    e.push(rect(p.activ, 74.895, 30.0, 76.895, 32.0)); // (f) outside
    e.extend(seal(p, &[p.activ, p.psd], 100.0, 10.0, 140.0, 50.0, 4.2));
    e.push(rect(p.activ, 109.095, 25.0, 111.095, 27.0)); // (g)
    e.push(rect(p.activ, 120.0, 40.0, 122.0, 48.0)); // (h) into the top wall (45.8..50)
    write("sealring", "Seal.b.h1", e);

    // h2 - 45° geometry and the tile lines.  A seal frame with 45° corners (EdgeSeal +
    // Metal1, box (10, 10)-(110, 110), legs 30, 4.2 wide; the inner bottom-left chamfer
    // is x + y = 55.94, the top-left one y − x = 64.06): (a) an Activ square whose
    // bottom-left corner (31.43, 31.43) is 4.893 from the bottom-left inner diagonal:
    // fires; (b) one whose top-left corner (24.5, 81.62) is 4.907 from the top-left
    // diagonal: clean; (c) a diamond of half-diagonal 2 whose right vertex is 4.895
    // from the right inner wall: fires.  Rectangular frames (EdgeSeal + Metal1, 40
    // boxes, 4.2 wide): (d) box (24, 120)-(64, 160) with an Activ 4.895 outside its
    // left wall, the gap across x = 20: fires; (e) box (60, 170)-(100, 210), its right
    // wall ending on x = 100, with an Activ 4.895 beyond the line: fires; (f) box
    // (1000, 1000)-(1040, 1040) with an Activ 4.895 inside its left wall: fires.
    let mut e = vec![
        oct_frame(p.seal, 10.0, 10.0, 110.0, 110.0, 30.0, 4.2, 4.2),
        oct_frame(p.m1, 10.0, 10.0, 110.0, 110.0, 30.0, 4.2, 4.2),
        rect(p.activ, 31.43, 31.43, 33.43, 33.43), // (a) 4.893
        rect(p.activ, 24.5, 79.62, 26.5, 81.62),   // (b) 4.907
        diamond(p.activ, 98.905, 60.0, 2.0),       // (c) 4.895
    ];
    e.extend(seal(p, &[p.m1], 24.0, 120.0, 64.0, 160.0, 4.2)); // (d)
    e.push(rect(p.activ, 17.105, 138.0, 19.105, 140.0));
    e.extend(seal(p, &[p.m1], 60.0, 170.0, 100.0, 210.0, 4.2)); // (e)
    e.push(rect(p.activ, 104.895, 188.0, 106.895, 190.0));
    e.extend(seal(p, &[p.m1], 1000.0, 1000.0, 1040.0, 1040.0, 4.2)); // (f)
    e.push(rect(p.activ, 1009.095, 1018.0, 1011.095, 1020.0));
    write("sealring", "Seal.b.h2", e);

    // h3/h4 - fifty frames (24 boxes, EdgeSeal + Metal1, 4.2 wide) each with an Activ
    // square 4.895 inside its left wall, flat and as an array.
    let mut cell = seal(p, &[p.m1], 0.0, 0.0, 24.0, 24.0, 4.2);
    cell.push(rect(p.activ, 9.095, 10.0, 11.095, 12.0));
}

/// h1 - the bound.  Seal frames (EdgeSeal + Activ + Metal1, 4.2 wide, 30 boxes) with
/// via rings 2.0 in from the outer edge (Activ enclosure 2.0 outside, over 1.3 inside):
/// (a) Cont 0.16, Via1 0.19, TopVia1 0.42 and TopVia2 0.9 wide - the pcell's rings,
/// clean; (b) Cont 0.155, Via1 0.185, TopVia1 0.415 and TopVia2 0.895: Seal.c, Seal.c1,
/// Seal.c2 and Seal.c3 fire; (c) Cont 0.2 and Via1 0.25: the manual says "ring width
/// 0.16", not "min." as every other row of the table, and the exact widths of Cnt.a
/// and V1.a are not checked inside EdgeSeal (6.10) - an over-wide ring is a width the
/// manual does not allow, so Seal.c and Seal.c1 should fire (neither tool reads it
/// so); (d) Via2, Via3 and Via4 rings 0.185 wide: Seal.c1 three times; (e) a Cont
/// ring 0.16 wide with a 1.0 break in its right wall: no numbered rule reads the
/// ring's continuity, clean.  Rings are drawn as four strips overlapping at the
/// corners, as the pcell draws them.
fn seal_c(p: &P) {
    let mut e = vec![];
    let bx = |i: f64| 5.0 + i * 40.0;
    let rings: [&[((i16, i16), f64)]; 4] = [
        &[(p.cont, 0.16), (p.via1, 0.19), (p.tv1, 0.42), (p.tv2, 0.9)],
        &[
            (p.cont, 0.155),
            (p.via1, 0.185),
            (p.tv1, 0.415),
            (p.tv2, 0.895),
        ],
        &[(p.cont, 0.2), (p.via1, 0.25)],
        &[(p.via2, 0.185), (p.via3, 0.185), (p.via4, 0.185)],
    ];
    for (i, r) in rings.into_iter().enumerate() {
        let x0 = bx(i as f64);
        e.extend(seal(p, &[p.activ, p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
        for &(l, t) in r {
            e.extend(via_ring(l, x0, 5.0, x0 + 30.0, 35.0, 2.0, t));
        }
    }
    let x0 = bx(4.0); // (e)
    e.extend(seal(p, &[p.activ, p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
    let (rx0, ry0, rx1, ry1) = (x0 + 2.0, 7.0, x0 + 28.0, 33.0);
    e.push(rect(p.cont, rx0, ry0, rx1, ry0 + 0.16));
    e.push(rect(p.cont, rx0, ry1 - 0.16, rx1, ry1));
    e.push(rect(p.cont, rx0, ry0, rx0 + 0.16, ry1));
    e.push(rect(p.cont, rx1 - 0.16, ry0, rx1, 19.5));
    e.push(rect(p.cont, rx1 - 0.16, 20.5, rx1, ry1));
    write("sealring", "Seal.c.h1", e);
}

/// h1 - the bound.  Seal frames (EdgeSeal + Activ + Metal1, 4.2 wide, 30 boxes) with
/// Cont rings 0.16 wide: (a) 1.3 in from the outer edge (2.74 to the inner one): clean;
/// (b) 1.295 in: fires - four walls 1.295 short, one run; (c) a Via1 ring 1.295 from
/// the inner edge (inset 2.715): fires; (d) a frame 3.5 wide with its Cont ring 1.3
/// from the outer edge and 2.04 from the inner: clean; (e) an EdgeSeal 4.2 wide whose
/// Activ ring is 3.5 wide, its outer edge 0.7 inside the marker's, with a Cont ring 1.7
/// in from the marker's edge - 1.0 from the Activ's own edge: fires (the Activ's edge
/// is what encloses, not the marker's); (f) a Cont ring 2.0 in a frame of EdgeSeal +
/// Metal1 with no Activ at all: no Activ encloses it, the manual's 1.3 is not met,
/// fires - both tools are silent (KLayout reads the rings that overlap Activ, gdscheck
/// the interacting ones).
fn seal_d_h(p: &P) {
    let mut e = vec![];
    let bx = |i: f64| 5.0 + i * 40.0;
    for (i, inset) in [(0.0, 1.3), (1.0, 1.295)] {
        let x0 = bx(i);
        e.extend(seal(p, &[p.activ, p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
        e.extend(via_ring(p.cont, x0, 5.0, x0 + 30.0, 35.0, inset, 0.16));
    }
    let x0 = bx(2.0); // (c) Via1 1.295 from the inner edge: 4.2 − 1.295 − 0.19 = 2.715
    e.extend(seal(p, &[p.activ, p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
    e.extend(via_ring(p.via1, x0, 5.0, x0 + 30.0, 35.0, 2.715, 0.19));
    let x0 = bx(3.0); // (d)
    e.extend(seal(p, &[p.activ, p.m1], x0, 5.0, x0 + 30.0, 35.0, 3.5));
    e.extend(via_ring(p.cont, x0, 5.0, x0 + 30.0, 35.0, 1.3, 0.16));
    let x0 = bx(4.0); // (e)
    e.extend(seal(p, &[p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
    e.extend(frame(p.activ, x0 + 0.7, 5.7, x0 + 29.3, 34.3, 3.5));
    e.extend(via_ring(p.cont, x0, 5.0, x0 + 30.0, 35.0, 1.7, 0.16));
    let x0 = bx(5.0); // (f)
    e.extend(seal(p, &[p.m1], x0, 5.0, x0 + 30.0, 35.0, 4.2));
    e.extend(via_ring(p.cont, x0, 5.0, x0 + 30.0, 35.0, 2.0, 0.16));
    write("sealring", "Seal.d.h1", e);

    // h2 - 45° corners and the tile lines.  (a) A frame with 45° corners (EdgeSeal +
    // Activ + Metal1, box (10, 10)-(70, 70), legs 20, 4.2 wide) with a Cont ring 1.3
    // in from its straight outer walls whose diagonal walls are 1.301 from the outer
    // diagonals (the frame's chamfer x + y = 40, the Cont's x + y = 41.84): clean; (b)
    // the same frame at (90, 10) with the Cont's diagonals 1.294 from the frame's
    // (x + y = 121.83 against 120): fires at the four corners, the straight walls at
    // 1.3.  Rectangular frames (EdgeSeal + Activ + Metal1, 40 boxes, 4.2 wide) with a
    // 1.295 Cont ring: (c) box (16, 100)-(56, 140), its left wall (16..20.2) across
    // x = 20; (d) box (60, 150)-(100, 190), its right wall ending on x = 100; (e) box
    // (1000, 1000)-(1040, 1040): each fires once.
    let mut e = vec![];
    for (x0, c) in [(10.0, 41.84), (90.0, 121.83)] {
        e.push(oct_frame(p.seal, x0, 10.0, x0 + 60.0, 70.0, 20.0, 4.2, 4.2));
        e.push(oct_frame(
            p.activ,
            x0,
            10.0,
            x0 + 60.0,
            70.0,
            20.0,
            4.2,
            4.2,
        ));
        e.push(oct_frame(p.m1, x0, 10.0, x0 + 60.0, 70.0, 20.0, 4.2, 4.2));
        // The Cont ring: the box 1.3 in, its outer chamfer on x + y = c, 0.16 wide.
        let k = c - (x0 + 1.3) - 11.3;
        e.push(oct_frame(
            p.cont,
            x0 + 1.3,
            11.3,
            x0 + 58.7,
            68.7,
            k,
            0.16,
            0.23 / SQRT2,
        ));
    }
    for (x0, y0) in [(16.0, 100.0), (60.0, 150.0), (1000.0, 1000.0)] {
        e.extend(seal(p, &[p.activ, p.m1], x0, y0, x0 + 40.0, y0 + 40.0, 4.2));
        e.extend(via_ring(p.cont, x0, y0, x0 + 40.0, y0 + 40.0, 1.295, 0.16));
    }
    write("sealring", "Seal.d.h2", e);

    // h3/h4 - fifty frames (EdgeSeal + Activ + Metal1, 4.2 wide, 12 boxes) with a
    // 1.295 Cont ring, flat and as an array.
    let mut cell = seal(p, &[p.activ, p.m1], 0.0, 0.0, 12.0, 12.0, 4.2);
    cell.extend(via_ring(p.cont, 0.0, 0.0, 12.0, 12.0, 1.295, 0.16));
}

/// h1 - the bound and what a ring is.  Seal frames (EdgeSeal + Metal1, 4.2 wide, 40
/// boxes) each with its Passiv ring 3 outside: (a) 4.2 wide, clean; (b) 4.195 wide,
/// fires; (c) 4.2 wide with a 1.0 × 1.0 nick in its outer right wall: 3.2 there,
/// fires once; (d) frame (a)'s hole holds a Passiv opening 3 × 3 (a small pad's
/// opening, Pas.a allows 2.1): it is no ring, and "Passiv ring width outside of
/// sealring" does not read it - clean; (e) a Passiv ring 4.2 wide with 45° corners
/// (legs 6 outside, 4.2 across the diagonals too, its hole 1.74 from the frame
/// corners): clean.
fn seal_e(p: &P) {
    let mut e = vec![];
    let bx = |i: f64| 10.0 + i * 70.0;
    for (i, w) in [(0.0, 4.2), (1.0, 4.195)] {
        let x0 = bx(i);
        e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
        e.extend(passiv_ring(p, x0, 10.0, x0 + 40.0, 50.0, 3.0, w));
    }
    let x0 = bx(2.0); // (c): the ring's outer right wall is at x0 + 47.2
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    let (px0, py0, px1, py1) = (x0 - 7.2, 2.8, x0 + 47.2, 57.2);
    e.push(rect(p.passiv, px0, py0, px1, py0 + 4.2));
    e.push(rect(p.passiv, px0, py1 - 4.2, px1, py1));
    e.push(rect(p.passiv, px0, py0, px0 + 4.2, py1));
    e.push(poly(
        p.passiv,
        &[
            (px1 - 4.2, py0),
            (px1, py0),
            (px1, 29.5),
            (px1 - 1.0, 29.5),
            (px1 - 1.0, 30.5),
            (px1, 30.5),
            (px1, py1),
            (px1 - 4.2, py1),
        ],
    ));
    e.push(rect(p.passiv, bx(0.0) + 18.0, 28.0, bx(0.0) + 21.0, 31.0)); // (d)
    let x0 = bx(3.0); // (e)
    e.extend(seal(p, &[p.m1], x0, 10.0, x0 + 40.0, 50.0, 4.2));
    e.push(oct_frame(
        p.passiv,
        x0 - 7.2,
        2.8,
        x0 + 47.2,
        57.2,
        6.0,
        4.2,
        4.2,
    ));
    write("sealring", "Seal.e.h1", e);
}

/// h1 - the bound, per conductor, and the tile lines.  Seal frames (4.2 wide, 40
/// boxes) with Passiv rings 4.2 wide outside: (a) EdgeSeal + Metal1, the ring 1.0
/// away, clean; (b) EdgeSeal + Metal1, 0.995 away, fires; (c) EdgeSeal + TopMetal2,
/// 0.995: fires (to EdgeSeal-TopMetal2); (d) EdgeSeal + Activ + Metal1 + TopMetal2,
/// 0.995: three times; (e) EdgeSeal + Metal1, the ring touching the frame (0): fires;
/// (f) an EdgeSeal 6.2 wide with Metal1 4.2 wide 1.0 inside its outer edge, the
/// Passiv ring 0.995 from the marker and 1.995 from the metal: the rule reads the
/// conductor, clean.  Frame (c)'s box is (100, 60)-(140, 100), its left gap
/// 99.005..100 ending on the line, its right gap 140..140.995 starting on x = 140;
/// frame (b)'s box (16, 60)-(56, 100) has its left wall across x = 20; frame (e) sits
/// at (1000, 1000).
fn seal_f(p: &P) {
    let mut e = seal(p, &[p.m1], 10.0, 10.0, 50.0, 50.0, 4.2); // (a)
    e.extend(passiv_ring(p, 10.0, 10.0, 50.0, 50.0, 1.0, 4.2));
    e.extend(seal(p, &[p.m1], 16.0, 60.0, 56.0, 100.0, 4.2)); // (b)
    e.extend(passiv_ring(p, 16.0, 60.0, 56.0, 100.0, 0.995, 4.2));
    e.extend(seal(p, &[p.tm2], 100.0, 60.0, 140.0, 100.0, 4.2)); // (c)
    e.extend(passiv_ring(p, 100.0, 60.0, 140.0, 100.0, 0.995, 4.2));
    e.extend(seal(
        p,
        &[p.activ, p.m1, p.tm2],
        70.0,
        10.0,
        110.0,
        50.0,
        4.2,
    )); // (d)
    e.extend(passiv_ring(p, 70.0, 10.0, 110.0, 50.0, 0.995, 4.2));
    e.extend(seal(p, &[p.m1], 1000.0, 1000.0, 1040.0, 1040.0, 4.2)); // (e)
    e.extend(passiv_ring(p, 1000.0, 1000.0, 1040.0, 1040.0, 0.0, 4.2));
    e.extend(frame(p.seal, 130.0, 10.0, 170.0, 50.0, 6.2)); // (f)
    e.extend(frame(p.m1, 131.0, 11.0, 169.0, 49.0, 4.2));
    e.extend(passiv_ring(p, 130.0, 10.0, 170.0, 50.0, 0.995, 4.2));
    write("sealring", "Seal.f.h1", e);

    // h2 - 45° geometry.  Frames with 45° corners (EdgeSeal + Metal1, 60 boxes, legs
    // 20, 4.2 wide) and Passiv rings 4.2 wide 1.0 outside their straight walls: (a)
    // box (10, 10)-(70, 70), the hole's diagonals 1.0006 from the frame's (the
    // frame's chamfer x + y = 40, the hole's x + y = 38.585): clean; (b) box (90, 10)-
    // (150, 70), the hole's diagonals 0.9935 from the frame's (x + y = 118.595 against
    // 120): fires at the four corners.  (c) A rectangular frame (box (10, 100)-(50,
    // 140)) with a Passiv ring 1.0 outside whose hole's top-right corner is chamfered
    // by a wall passing 0.9935 from the frame's outer corner (50, 140): fires once.
    let mut e = vec![];
    for (x0, gap) in [(10.0, 1.0006), (90.0, 0.9935)] {
        e.push(oct_frame(p.seal, x0, 10.0, x0 + 60.0, 70.0, 20.0, 4.2, 4.2));
        e.push(oct_frame(p.m1, x0, 10.0, x0 + 60.0, 70.0, 20.0, 4.2, 4.2));
        // The Passiv ring's box is 5.2 out (1.0 gap, 4.2 wide); its hole's chamfer
        // lies gap·√2 before the frame's along x + y, the outer chamfer 4.2·√2 further.
        let hole_c = (x0 + 30.0) - grid(gap * SQRT2);
        let ki = hole_c - (x0 - 1.0) - 9.0;
        let ko = ki + 8.4 - 5.94;
        e.push(oct_frame(
            p.passiv,
            x0 - 5.2,
            4.8,
            x0 + 65.2,
            75.2,
            ko,
            4.2,
            4.2,
        ));
    }
    e.extend(seal(p, &[p.m1], 10.0, 100.0, 50.0, 140.0, 4.2)); // (c)
    let (px0, py0, px1, py1) = (4.8, 94.8, 55.2, 145.2);
    e.push(rect(p.passiv, px0, py0, px1, py0 + 4.2));
    e.push(rect(p.passiv, px0, py0, px0 + 4.2, py1));
    e.push(rect(p.passiv, px1 - 4.2, py0, px1, py1));
    // The top wall, its inner right corner (51, 141) cut by x + y = 191.405 - 0.9935
    // from the frame's corner (50, 140).
    e.push(poly(
        p.passiv,
        &[
            (px0, py1 - 4.2),
            (50.405, py1 - 4.2),
            (px1 - 4.2, 140.405),
            (px1, 140.405),
            (px1, py1),
            (px0, py1),
        ],
    ));
    write("sealring", "Seal.f.h2", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/sealring")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    seal_a(&p);
    seal_b_h(&p);
    seal_c(&p);
    seal_d_h(&p);
    seal_e(&p);
    seal_f(&p);
    seal_l(&p);
    seal_n(&p);
    seal_pcell(&p);
}
