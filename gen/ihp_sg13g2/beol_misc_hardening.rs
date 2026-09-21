// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts (ci/hardening/SPEC.md) for the `sealring`, `slit`, `lbe` and `lu`
//! decks: sections 6.10 (Sealring, Seal.*), 7.3 (Metal Slits, Slt.*), 9.1 (Localized
//! Backside Etching, LBE.*) and 7.2.2 (Latch-up, LU.*) of the SG13G2 layout rules.
//! Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`; each function's
//! comment states the geometry and what the manual says about it, the expected answers
//! are in the decks' tables of tests/ihp-sg13g2.rs and the reasoning in
//! ci/hardening/reports/ihp-sg13g2/beol_misc.md.
//!
//! The seal frames are drawn as IHP's `sealring` pcell draws them: the EdgeSeal marker
//! is the 4.2 µm ring itself, the conductors coincide with it, the via rings are 4.2
//! long pieces overlapping into a ring, and the Passiv ring lies 3 µm outside.

use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
use gds21::{GdsElement, GdsLibrary};
use gdscheck::pdk::PdkConfig;

const SQRT2: f64 = std::f64::consts::SQRT_2;
/// One grid step.
const G: f64 = 0.005;

fn grid(v: f64) -> f64 {
    (v / G).round() * G
}

/// Every layer the four decks draw.
struct P {
    activ: (i16, i16),
    psd: (i16, i16),
    nwell: (i16, i16),
    pwb: (i16, i16),
    gp: (i16, i16),
    cont: (i16, i16),
    m1: (i16, i16),
    m2: (i16, i16),
    m3: (i16, i16),
    m4: (i16, i16),
    m5: (i16, i16),
    tm1: (i16, i16),
    tm2: (i16, i16),
    via1: (i16, i16),
    via2: (i16, i16),
    via3: (i16, i16),
    via4: (i16, i16),
    tv1: (i16, i16),
    tv2: (i16, i16),
    m1s: (i16, i16),
    m2s: (i16, i16),
    m3s: (i16, i16),
    m5s: (i16, i16),
    tm1s: (i16, i16),
    tm2s: (i16, i16),
    mim: (i16, i16),
    ind: (i16, i16),
    passiv: (i16, i16),
    dfpad: (i16, i16),
    seal: (i16, i16),
    bnd: (i16, i16),
    lbe: (i16, i16),
}

impl P {
    fn new(pdk: &PdkConfig) -> Self {
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
    fn conductors(&self) -> [(i16, i16); 9] {
        [
            self.activ, self.psd, self.m1, self.m2, self.m3, self.m4, self.m5, self.tm1, self.tm2,
        ]
    }
}

fn write(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(
        &format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}

fn write_lib(deck: &str, name: &str, lib: GdsLibrary) {
    write_gz(&format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz"), lib);
}

/// `<name>.h<k>` flat and `<name>.h<k+1>` as an array reference: 10 × 5 copies of
/// `cell` at `pitch`.  Hierarchy must not change the answer.
fn arrays(deck: &str, name: &str, k: u32, cell: Vec<GdsElement>, pitch: f64) {
    write(
        deck,
        &format!("{name}.h{k}"),
        flat_array(&cell, 10, 5, pitch),
    );
    write_lib(
        deck,
        &format!("{name}.h{}", k + 1),
        ref_array(cell, 10, 5, pitch),
    );
}

/// A rectangular ring on `l`: the box `(x0, y0)-(x1, y1)` less its hole `w` in, as four
/// overlapping walls.
fn frame(l: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<GdsElement> {
    vec![
        rect(l, x0, y0, x1, y0 + w),
        rect(l, x0, y1 - w, x1, y1),
        rect(l, x0, y0, x0 + w, y1),
        rect(l, x1 - w, y0, x1, y1),
    ]
}

/// The same ring as one polygon with a hole: the outline runs in along a zero-width
/// cut from the left wall's middle, round the hole and back.
fn frame_poly(l: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> GdsElement {
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
fn octagon(x0: f64, y0: f64, x1: f64, y1: f64, k: f64) -> Vec<(f64, f64)> {
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
fn oct_frame(
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
fn seal(
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

pub fn generate(pdk: &PdkConfig) {
    for d in ["sealring", "slit", "lbe", "lu"] {
        std::fs::create_dir_all(format!("tests/data/ihp-sg13g2/{d}"))
            .expect("failed to create output directory");
    }
    let p = P::new(pdk);
    seal_a(&p);
    seal_b(&p);
    seal_c(&p);
    seal_d(&p);
    seal_e(&p);
    seal_f(&p);
    seal_l(&p);
    seal_n(&p);
    slt_a(&p);
    slt_b(&p);
    slt_c(&p);
    slt_e(&p);
    slt_f(&p);
    slt_g(&p);
    slt_h(&p);
    slt_i(&p);
    lbe_a(&p);
    lbe_b(&p);
    lbe_b1(&p);
    lbe_b2(&p);
    lbe_c(&p);
    lbe_d(&p);
    lbe_e(&p);
    lbe_f(&p);
    lbe_h(&p);
    lbe_i(&p);
    lu_a(&p);
    lu_b(&p);
    lu_cd(&p, true);
    lu_cd(&p, false);
}

// --- Seal.l: no structures outside the sealring boundary ---

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

// --- Seal.n: the sealring must be enclosed by an unbroken Passiv ring ---

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

// --- Seal.a: min. EdgeSeal-<conductor> width 3.50 ---

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

    // h3/h4 - fifty 3.495 frames (10 boxes, EdgeSeal + Metal1), flat and as an array.
    arrays(
        "sealring",
        "Seal.a",
        3,
        seal(p, &[p.m1], 0.0, 0.0, 10.0, 10.0, 3.495),
        15.0,
    );
}

// --- Seal.b: min. Activ space to EdgeSeal-<conductor> 4.90 ---

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
fn seal_b(p: &P) {
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
    arrays("sealring", "Seal.b", 3, cell, 30.0);
}

// --- Seal.c/c1/c2/c3: EdgeSeal-Cont/Via/TopVia ring width 0.16/0.19/0.42/0.90 ---

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

// --- Seal.d: min. EdgeSeal-Activ enclosure of the via rings 1.30 ---

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
fn seal_d(p: &P) {
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
    arrays("sealring", "Seal.d", 3, cell, 20.0);
}

// --- Seal.e: min. Passiv ring width outside of the sealring 4.20 ---

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

// --- Seal.f: min. Passiv ring outside of the sealring space to EdgeSeal-<conductor> 1.00 ---

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

// --- Slt.a: min. Metal:slit width 2.80 ---

/// A `w` × `h` slit on `s` centred in a plate on `m` with margin `mg` all round, the
/// plate's lower-left corner at `(x0, y0)`.
fn plated(
    m: (i16, i16),
    s: (i16, i16),
    x0: f64,
    y0: f64,
    w: f64,
    h: f64,
    mg: f64,
) -> Vec<GdsElement> {
    vec![
        rect(m, x0, y0, x0 + w + 2.0 * mg, y0 + h + 2.0 * mg),
        rect(s, x0 + mg, y0 + mg, x0 + mg + w, y0 + mg + h),
    ]
}

/// h1 - the bound.  Metal1 slits in plates 3 µm wider than the slit (enclosure 3): (a)
/// 2.8 × 6, clean; (b) 2.795 × 6 fires; (c) 6 × 2.795 fires; (d) a 45° strip 2.8002
/// wide (`d` 1.98): clean; (e) 2.793 wide (`d` 1.975): fires; (f) 2.8 × 6 drawn as two
/// abutting 1.4 × 6 boxes: one slit after merging, clean; (g) 2.8 × 6 with a 0.2 × 0.2
/// nick in its right wall: 2.6 there, fires; (h) 2.795 × 6 across x = 20 (18.6 to
/// 21.395): fires; (i) the same across x = 40; (j) at (1000, 1000); (k) a TopMetal2
/// slit 2.795 × 6; (l) a Metal3 slit 2.795 × 6.  Every plate is under 30 wide.
fn slt_a(p: &P) {
    let (m, s) = (p.m1, p.m1s);
    let mut e = plated(m, s, 0.0, 0.0, 2.8, 6.0, 3.0); // (a)
    e.extend(plated(m, s, 16.0, 0.0, 2.795, 6.0, 3.0)); // (b)
    e.extend(plated(m, s, 32.0, 0.0, 6.0, 2.795, 3.0)); // (c)
    e.push(rect(m, 48.0, 0.0, 60.0, 12.0)); // (d): bbox (50.02, 2)-(58, 9.98)
    e.push(strip45(s, 52.0, 2.0, 6.0, 1.98));
    e.push(rect(m, 64.0, 0.0, 76.0, 12.0)); // (e)
    e.push(strip45(s, 68.0, 2.0, 6.0, 1.975));
    e.push(rect(m, 80.0, 0.0, 88.8, 12.0)); // (f)
    e.push(rect(s, 83.0, 3.0, 84.4, 9.0));
    e.push(rect(s, 84.4, 3.0, 85.8, 9.0));
    e.push(rect(m, 96.0, 0.0, 104.8, 12.0)); // (g)
    e.push(poly(
        s,
        &[
            (99.0, 3.0),
            (101.8, 3.0),
            (101.8, 5.9),
            (101.6, 5.9),
            (101.6, 6.1),
            (101.8, 6.1),
            (101.8, 9.0),
            (99.0, 9.0),
        ],
    ));
    e.extend(plated(m, s, 15.6, 30.0, 2.795, 6.0, 3.0)); // (h)
    e.extend(plated(m, s, 35.6, 30.0, 2.795, 6.0, 3.0)); // (i)
    e.extend(plated(m, s, 1000.0, 1000.0, 2.795, 6.0, 3.0)); // (j)
    e.extend(plated(p.tm2, p.tm2s, 60.0, 30.0, 2.795, 6.0, 3.0)); // (k)
    e.extend(plated(p.m3, p.m3s, 76.0, 30.0, 2.795, 6.0, 3.0)); // (l)
    write("slit", "Slt.a.h1", e);

    // h2/h3 - fifty 2.795 × 4 slits, each in its own 8.795 × 10 plate, flat and as an
    // array.
    arrays(
        "slit",
        "Slt.a",
        2,
        plated(m, s, 0.0, 0.0, 2.795, 4.0, 3.0),
        12.0,
    );
}

// --- Slt.b: max. Metal:slit width 20.00 ---

/// h1 - the bound and the long side.  Figure 7.5 draws `b` across the slit's long
/// side, `a` across its short one.  Metal1 slits in plates 2 wider (enclosure 2, every
/// plate under 30): (a) 20 × 6, clean; (b) 20.005 × 6 fires; (c) 6 × 20.005 fires; (d)
/// 3 × 25: 3 wide and 25 long - `b` is 25, fires; (e) 20 × 20, clean; (f) 20.005 ×
/// 20.005 fires; (g) a 45° strip 3 wide (`d` 2.12) running 15 along each axis, 21.2
/// long: fires; (h) an L-shaped slit, arms 3 wide and 15 long, in a 15 box: clean.
fn slt_b(p: &P) {
    let (m, s) = (p.m1, p.m1s);
    let mut e = plated(m, s, 0.0, 0.0, 20.0, 6.0, 2.0); // (a)
    e.extend(plated(m, s, 30.0, 0.0, 20.005, 6.0, 2.0)); // (b)
    e.extend(plated(m, s, 60.0, 0.0, 6.0, 20.005, 2.0)); // (c)
    e.extend(plated(m, s, 75.0, 0.0, 3.0, 25.0, 2.0)); // (d)
    e.extend(plated(m, s, 0.0, 30.0, 20.0, 20.0, 2.0)); // (e)
    e.extend(plated(m, s, 30.0, 30.0, 20.005, 20.005, 2.0)); // (f)
    e.push(rect(m, 60.0, 30.0, 82.0, 52.0)); // (g): bbox (62.88, 32)-(80, 49.12)
    e.push(strip45(s, 65.0, 32.0, 15.0, 2.12));
    e.push(rect(m, 90.0, 30.0, 109.0, 49.0)); // (h)
    e.push(poly(
        s,
        &[
            (92.0, 32.0),
            (107.0, 32.0),
            (107.0, 35.0),
            (95.0, 35.0),
            (95.0, 47.0),
            (92.0, 47.0),
        ],
    ));
    write("slit", "Slt.b.h1", e);
}

// --- Slt.c: max. Metal width without requiring a slit 30.00 ---

/// h1 - the bound, geometry and exemptions.  Metal1 unless said otherwise: (a) a 30 ×
/// 30 plate, clean; (b) 30.005 × 30.005 fires; (c) 30.005 × 100 fires; (d) 30 × 100
/// clean; (e) 40 × 40 with a 2.8 × 38 slit through its middle: two 18.6 strips, clean;
/// (f) 40 × 40 with a 2.8 × 10 slit at its centre: no 30 disc of metal is left
/// uncut (every centre is within 15 of the slit), clean; (g) 32 × 32 with a 2.8 × 30
/// slit 1 from its left wall: 1 and 28.2, clean; (h) two abutting 20 × 40 boxes: one
/// 40 × 40 plate, fires once; (i) a ring 60 outer with 20 walls: clean; (j) a ring
/// 70.01 outer with 30.005 walls: fires; (k) a diamond 30.01 across its walls (`a`
/// 21.22): fires; (l) a diamond 29.995 across (`a` 21.21): clean; (m) an L of 30.005
/// arms 60 long: fires; (n) a 45° strip 30.01 wide (`d` 21.22) running 40: fires; (o)
/// a 30.005 square from x = 5 across x = 20; (p) at (1000, 1000); (q) a 100 × 30.005
/// bar from x = 160 across x = 200; (r) Metal5 40 × 40 under a MIM 40 × 40: Slt.e1,
/// clean; (s) TopMetal2 40 × 40 under a Passiv opening 40 × 40 with dfpad (a pad):
/// Slt.e, clean; (t) Metal1 40 × 40 inside an IND 50 × 50: Slt.e2, clean; (u)
/// TopMetal2 60 × 60 with a pad opening 40 × 40 at its centre: the metal outside the
/// opening is a ring 10 wide, clean; (v) Metal1 40 × 40 with a Passiv 5 × 40 over its
/// left edge: 35 × 40 of metal is not under the opening, fires.
fn slt_c(p: &P) {
    let m = p.m1;
    let mut e = vec![
        rect(m, 0.0, 0.0, 30.0, 30.0),      // (a)
        rect(m, 40.0, 0.0, 70.005, 30.005), // (b)
        rect(m, 80.0, 0.0, 110.005, 100.0), // (c)
        rect(m, 120.0, 0.0, 150.0, 100.0),  // (d)
        rect(m, 160.0, 0.0, 200.0, 40.0),   // (e)
        rect(p.m1s, 178.6, 1.0, 181.4, 39.0),
        rect(m, 210.0, 0.0, 250.0, 40.0), // (f)
        rect(p.m1s, 228.6, 15.0, 231.4, 25.0),
        rect(m, 260.0, 0.0, 292.0, 32.0), // (g)
        rect(p.m1s, 261.0, 1.0, 263.8, 31.0),
        rect(m, 300.0, 0.0, 320.0, 40.0), // (h)
        rect(m, 320.0, 0.0, 340.0, 40.0),
        diamond(m, 400.0, 30.0, 21.22),              // (k)
        diamond(m, 460.0, 30.0, 21.21),              // (l)
        rect(m, 5.0, 60.0, 35.005, 90.005),          // (o)
        rect(m, 1000.0, 1000.0, 1030.005, 1030.005), // (p)
        rect(m, 160.0, 60.0, 260.0, 90.005),         // (q)
    ];
    e.extend(frame(m, 0.0, 120.0, 60.0, 180.0, 20.0)); // (i)
    e.extend(frame(m, 80.0, 120.0, 150.01, 190.01, 30.005)); // (j)
    e.push(rect(m, 170.0, 120.0, 200.005, 180.0)); // (m)
    e.push(rect(m, 170.0, 120.0, 230.0, 150.005));
    e.push(strip45(m, 270.0, 120.0, 40.0, 21.22)); // (n)
    e.push(rect(p.m5, 0.0, 220.0, 40.0, 260.0)); // (r)
    e.push(rect(p.mim, 0.0, 220.0, 40.0, 260.0));
    e.push(rect(p.tm2, 60.0, 220.0, 100.0, 260.0)); // (s)
    e.push(rect(p.passiv, 60.0, 220.0, 100.0, 260.0));
    e.push(rect(p.dfpad, 60.0, 220.0, 100.0, 260.0));
    e.push(rect(m, 125.0, 225.0, 165.0, 265.0)); // (t)
    e.push(rect(p.ind, 120.0, 220.0, 170.0, 270.0));
    e.push(rect(p.tm2, 190.0, 220.0, 250.0, 280.0)); // (u)
    e.push(rect(p.passiv, 200.0, 230.0, 240.0, 270.0));
    e.push(rect(p.dfpad, 200.0, 230.0, 240.0, 270.0));
    e.push(rect(m, 270.0, 220.0, 310.0, 260.0)); // (v)
    e.push(rect(p.passiv, 270.0, 220.0, 275.0, 260.0));
    write("slit", "Slt.c.h1", e);

    // h2/h3 - fifty 30.005 squares, flat and as an array.
    arrays(
        "slit",
        "Slt.c",
        2,
        vec![rect(m, 0.0, 0.0, 30.005, 30.005)],
        40.0,
    );
}

// --- Slt.e: no slits on pads ---

/// h1 - the pad's extent.  A pad: TopMetal2 90 × 90, dfpad 80 × 80 (5..85), Passiv
/// opening 70 × 70: (a) a TopMetal2 slit 3 × 10 inside the dfpad fires; (b) one across
/// the dfpad's right edge (83.5..86.5) fires; (c) one touching the dfpad's edge from
/// outside (85..88) is off the pad, clean; (d) a slotted TopMetal2 plate under a dfpad
/// with no Passiv (not a pad): clean.
fn slt_e(p: &P) {
    let mut e = vec![
        rect(p.tm2, 0.0, 0.0, 90.0, 90.0),
        rect(p.dfpad, 5.0, 5.0, 85.0, 85.0),
        rect(p.passiv, 10.0, 10.0, 80.0, 80.0),
        rect(p.tm2s, 40.0, 20.0, 43.0, 30.0), // (a)
        rect(p.tm2s, 83.5, 20.0, 86.5, 30.0), // (b)
        rect(p.tm2s, 85.0, 40.0, 88.0, 50.0), // (c)
    ];
    e.extend(plated(p.tm2, p.tm2s, 120.0, 0.0, 3.0, 10.0, 11.0)); // (d)
    e.push(rect(p.dfpad, 122.0, 2.0, 143.0, 30.0));
    write("slit", "Slt.e.h1", e);
}

// --- Slt.f: min. Metal enclosure of Metal:slit 1.00 ---

/// h1 - the bound.  Metal1 plates 12 × 12 with a 2.8 × 6 slit 3 from the top and
/// bottom: (a) 1.0 from the right wall, clean; (b) 0.995 from the right wall, fires;
/// (c) 0.995 from all four walls (a 4.79 × 7.99 plate): fires; (d) a slit on the
/// plate's right edge (0): fires; (e) a slit crossing the plate's right edge by 1:
/// fires; (f) a slit with no metal at all: enclosed by nothing, fires; (g) a plate
/// whose top-right corner is chamfered so its 45° wall passes 0.9935 from the slit's
/// top-right corner: fires; (h) the same at 1.0006: clean; (i) a plate ending on x =
/// 20 with the slit 0.995 from that edge; (j) a plate across x = 40 with the slit's
/// right wall 0.995 from the plate's at 44.6; (k) at (1000, 1000).
fn slt_f(p: &P) {
    let (m, s) = (p.m1, p.m1s);
    let plate = |x0: f64, y0: f64, right: f64| {
        vec![
            rect(m, x0, y0, x0 + 12.0, y0 + 12.0),
            rect(
                s,
                x0 + 12.0 - right - 2.8,
                y0 + 3.0,
                x0 + 12.0 - right,
                y0 + 9.0,
            ),
        ]
    };
    let mut e = plate(0.0, 0.0, 1.0); // (a)
    e.extend(plate(16.0, 0.0, 0.995)); // (b)
    e.push(rect(m, 32.0, 0.0, 36.79, 7.99)); // (c)
    e.push(rect(s, 32.995, 0.995, 35.795, 6.995));
    e.extend(plate(48.0, 0.0, 0.0)); // (d)
    e.extend(plate(64.0, 0.0, -1.0)); // (e)
    e.push(rect(s, 84.0, 3.0, 86.8, 9.0)); // (f)
    // (g), (h): the slit's top-right corner at (x0 + 8, 9); the chamfer x + y = k.
    for (i, gap) in [(0.0, 0.9935), (1.0, 1.0006)] {
        let x0 = 100.0 + i * 16.0;
        let k = x0 + 8.0 + 9.0 + grid(gap * SQRT2);
        e.push(chamfered_tr(m, x0, 0.0, x0 + 12.0, 12.0, k));
        e.push(rect(s, x0 + 5.2, 3.0, x0 + 8.0, 9.0));
    }
    e.extend(plate(8.0, 30.0, 0.995)); // (i): plate 8..20
    e.extend(plate(32.6, 30.0, 0.995)); // (j): plate 32.6..44.6
    e.extend(plate(1000.0, 1000.0, 0.995)); // (k)
    write("slit", "Slt.f.h1", e);
}

// --- Slt.g: min. Metal5:slit and TopMetal1:slit space to MIM 0.60 ---

/// h1 - the bound and both metrics.  A Metal5 plate 20 × 20 holds a MIM 5 × 5 (at
/// (8, 8)) and a slit 2.8 × 6: (a) the slit 0.6 left of the MIM, clean; (b) 0.595:
/// fires; (c) a TopMetal1 plate with the same MIM and a TopMetal1 slit 0.595 away:
/// fires; (d) a Metal1 plate with a Metal1 slit 0.595 from a MIM: no rule, clean; (e)
/// a Metal5 slit whose top-right corner is dx = dy = 0.42 (0.594) from the MIM's
/// bottom-left corner: fires; (f) dx = dy = 0.425 (0.601): clean.
fn slt_g(p: &P) {
    let cell = |m: (i16, i16), s: (i16, i16), x0: f64, gap: f64| {
        vec![
            rect(m, x0, 0.0, x0 + 20.0, 20.0),
            rect(p.mim, x0 + 8.0, 8.0, x0 + 13.0, 13.0),
            rect(s, x0 + 8.0 - gap - 2.8, 7.0, x0 + 8.0 - gap, 13.0),
        ]
    };
    let mut e = cell(p.m5, p.m5s, 0.0, 0.6); // (a)
    e.extend(cell(p.m5, p.m5s, 30.0, 0.595)); // (b)
    e.extend(cell(p.tm1, p.tm1s, 60.0, 0.595)); // (c)
    e.extend(cell(p.m1, p.m1s, 90.0, 0.595)); // (d)
    for (i, d) in [(0.0, 0.42), (1.0, 0.425)] {
        let x0 = 120.0 + i * 30.0;
        e.push(rect(p.m5, x0, 0.0, x0 + 20.0, 20.0));
        e.push(rect(p.mim, x0 + 8.0, 8.0, x0 + 13.0, 13.0));
        e.push(rect(
            p.m5s,
            x0 + 8.0 - d - 2.8,
            8.0 - d - 6.0,
            x0 + 8.0 - d,
            8.0 - d,
        ));
    }
    write("slit", "Slt.g.h1", e);
}

// --- Slt.h1-h4: min. Metal(n):slit space to the vias above and below ---

/// h1 - the bound, per layer.  Plates 12 × 12 with a slit 2.8 × 6 (at x0 + 2..4.8, y
/// 3..9) and a via to its right: (a) Metal1 slit, Cont 0.3 away, clean; (b) Cont
/// 0.295, fires (Slt.h1); (c) Via1 0.295, fires (Slt.h1); (d) Metal2 slit, Via1 0.295
/// (Slt.h2); (e) Metal2 slit, Via2 0.295 (Slt.h2); (f) Metal5 slit, Via4 0.295
/// (Slt.h2); (g) Metal5 slit, TopVia1 0.295 (Slt.h2); (h) TopMetal1 slit, TopVia1 1.0,
/// clean; (i) TopVia1 0.995, fires (Slt.h3); (j) TopVia2 0.995 (Slt.h3); (k)
/// TopMetal2 slit, TopVia2 0.995 (Slt.h4); (l) a Via1 lying inside a Metal1 slit: the
/// via is on no metal, 0 from the slit: fires (Slt.h1); (m) a Cont corner to corner
/// dx = dy = 0.21 (0.297) from the slit's top-right corner: fires; (n) dx = dy =
/// 0.215 (0.304): clean; (o) a Metal1 slit with a Via2 0.1 away: no rule, clean.
fn slt_h(p: &P) {
    let cell =
        |m: (i16, i16), s: (i16, i16), v: (i16, i16), vs: f64, x0: f64, y0: f64, gap: f64| {
            vec![
                rect(m, x0, y0, x0 + 12.0, y0 + 12.0),
                rect(s, x0 + 2.0, y0 + 3.0, x0 + 4.8, y0 + 9.0),
                rect(
                    v,
                    x0 + 4.8 + gap,
                    y0 + 5.0,
                    x0 + 4.8 + gap + vs,
                    y0 + 5.0 + vs,
                ),
            ]
        };
    let mut e = cell(p.m1, p.m1s, p.cont, 0.16, 0.0, 0.0, 0.3); // (a)
    e.extend(cell(p.m1, p.m1s, p.cont, 0.16, 16.0, 0.0, 0.295)); // (b)
    e.extend(cell(p.m1, p.m1s, p.via1, 0.19, 32.0, 0.0, 0.295)); // (c)
    e.extend(cell(p.m2, p.m2s, p.via1, 0.19, 48.0, 0.0, 0.295)); // (d)
    e.extend(cell(p.m2, p.m2s, p.via2, 0.19, 64.0, 0.0, 0.295)); // (e)
    e.extend(cell(p.m5, p.m5s, p.via4, 0.19, 80.0, 0.0, 0.295)); // (f)
    e.extend(cell(p.m5, p.m5s, p.tv1, 0.42, 96.0, 0.0, 0.295)); // (g)
    e.extend(cell(p.tm1, p.tm1s, p.tv1, 0.42, 0.0, 20.0, 1.0)); // (h)
    e.extend(cell(p.tm1, p.tm1s, p.tv1, 0.42, 16.0, 20.0, 0.995)); // (i)
    e.extend(cell(p.tm1, p.tm1s, p.tv2, 0.9, 32.0, 20.0, 0.995)); // (j)
    e.extend(cell(p.tm2, p.tm2s, p.tv2, 0.9, 48.0, 20.0, 0.995)); // (k)
    e.push(rect(p.m1, 64.0, 20.0, 76.0, 32.0)); // (l)
    e.push(rect(p.m1s, 66.0, 23.0, 68.8, 29.0));
    e.push(rect(p.via1, 67.3, 25.0, 67.49, 25.19));
    e.push(rect(p.m1, 80.0, 20.0, 92.0, 32.0)); // (m): the slit's corner (84.8, 29)
    e.push(rect(p.m1s, 82.0, 23.0, 84.8, 29.0));
    e.push(rect(p.cont, 85.01, 29.21, 85.17, 29.37));
    e.push(rect(p.m1, 96.0, 20.0, 108.0, 32.0)); // (n)
    e.push(rect(p.m1s, 98.0, 23.0, 100.8, 29.0));
    e.push(rect(p.cont, 101.015, 29.215, 101.175, 29.375));
    e.extend(cell(p.m1, p.m1s, p.via2, 0.19, 112.0, 20.0, 0.1)); // (o)
    write("slit", "Slt.h.h1", e);
}

// --- Slt.i: min. Metal:slit density 6 % on plates bigger than 35 × 35 ---

/// Four 3 × `h` slits in a 40 plate at `(x0, y0)`.
fn four_slits(s: (i16, i16), x0: f64, y0: f64, h: f64) -> Vec<GdsElement> {
    let mut e = vec![];
    for i in 0..2 {
        for j in 0..2 {
            let sx = x0 + 8.0 + i as f64 * 20.0;
            let sy = y0 + 6.0 + j as f64 * 20.0;
            e.push(rect(s, sx, sy, sx + 3.0, sy + h));
        }
    }
    e
}

/// h1 - the bound and what a plate is.  Metal1: (a) a 35 × 35 plate with one 2.8 × 3
/// slit (0.69 %): not bigger than 35 × 35, clean; (b) 35.005 × 35.005 with the same
/// slit: fires; (c) 40 × 40 with four 3 × 8 slits (96 = 6.0 %): clean; (d) four 3 ×
/// 7.995 (95.94 = 5.996 %): fires; (e) a 48 × 100 bus with one lengthwise 2.8 × 98
/// slit (5.72 %, two 22.6 strips - no Slt.c): fires; (f) an L of a 56 × 90 bar and a
/// 30 × 56 stub with a lengthwise slit in each (394.8 of 6720 = 5.87 %, every strip
/// under 30): fires once; (g) 40 × 40 with three 3 × 8 slits inside and one 3 × 8 slit
/// crossing the plate's right edge, 12 of it inside (84 = 5.25 % of what lies on the
/// plate): fires (and Slt.f for the crossing slit); (h) 40 × 40 with two 3 × 8 slits
/// overlapping by 3 × 4 (36 as one slit) and two more (84 = 5.25 %): fires - the
/// slits' union counts, not their sum; (i) two abutting 20 × 40 boxes with one 2.8 × 3
/// slit: one plate, fires once; (j) a 40 × 40 plate at (80, 0) across x = 100 with one
/// slit: fires; (k) at (1000, 1000): fires; (l) a TopMetal2 pad 60 × 60 with a 50 × 50
/// opening (Passiv + dfpad): the metal not under the opening is a ring 5 wide, clean.
fn slt_i(p: &P) {
    let (m, s) = (p.m1, p.m1s);
    let one = |x0: f64, y0: f64, side: f64| {
        vec![
            rect(m, x0, y0, x0 + side, y0 + side),
            rect(s, x0 + 16.0, y0 + 16.0, x0 + 18.8, y0 + 19.0),
        ]
    };
    let mut e = one(0.0, 0.0, 35.0); // (a)
    e.extend(one(40.0, 0.0, 35.005)); // (b)
    e.push(rect(m, 80.0, 0.0, 120.0, 40.0)); // (j) - also (c)'s neighbour
    e.push(rect(s, 96.0, 16.0, 98.8, 19.0));
    e.push(rect(m, 0.0, 50.0, 40.0, 90.0)); // (c)
    e.extend(four_slits(s, 0.0, 50.0, 8.0));
    e.push(rect(m, 50.0, 50.0, 90.0, 90.0)); // (d)
    e.extend(four_slits(s, 50.0, 50.0, 7.995));
    e.push(rect(m, 100.0, 50.0, 148.0, 150.0)); // (e)
    e.push(rect(s, 122.6, 51.0, 125.4, 149.0));
    e.push(rect(m, 160.0, 50.0, 216.0, 140.0)); // (f)
    e.push(rect(m, 216.0, 50.0, 246.0, 106.0));
    e.push(rect(s, 186.6, 51.0, 189.4, 139.0));
    e.push(rect(s, 192.0, 76.6, 245.0, 79.4));
    e.push(rect(m, 0.0, 100.0, 40.0, 140.0)); // (g)
    e.extend(four_slits(s, 0.0, 100.0, 8.0).into_iter().take(3));
    e.push(rect(s, 36.0, 126.0, 44.0, 129.0));
    e.push(rect(m, 50.0, 100.0, 90.0, 140.0)); // (h)
    e.push(rect(s, 58.0, 106.0, 61.0, 114.0));
    e.push(rect(s, 58.0, 110.0, 61.0, 118.0));
    e.push(rect(s, 78.0, 106.0, 81.0, 114.0));
    e.push(rect(s, 78.0, 126.0, 81.0, 134.0));
    e.push(rect(m, 260.0, 0.0, 280.0, 40.0)); // (i)
    e.push(rect(m, 280.0, 0.0, 300.0, 40.0));
    e.push(rect(s, 276.0, 16.0, 278.8, 19.0));
    e.extend(one(1000.0, 1000.0, 40.0)); // (k)
    e.push(rect(p.tm2, 260.0, 60.0, 320.0, 120.0)); // (l)
    e.push(rect(p.passiv, 265.0, 65.0, 315.0, 115.0));
    e.push(rect(p.dfpad, 265.0, 65.0, 315.0, 115.0));
    write("slit", "Slt.i.h1", e);

    // h2/h3 - fifty 35.005 plates with one 2.8 × 3 slit, flat and as an array.
    arrays("slit", "Slt.i", 2, one(0.0, 0.0, 35.005), 45.0);
}

// --- LBE.a: min. LBE width 100 ---

/// h1 - the bound and geometry.  Every shape 100 or more from the others: (a) 100 ×
/// 200, clean; (b) 99.995 × 200 fires; (c) 200 × 99.995 fires; (d) a 45° strip
/// 100.006 wide (`d` 70.715): clean; (e) 99.985 wide (`d` 70.7): fires; (f) a diamond
/// 100.006 across (`a` 70.715): clean; (g) 99.985 across (`a` 70.7): fires; (h) an L
/// of 99.995 arms 300 long: fires; (i) a 0.005 × 200 sliver: fires; (j) a 300 × 100
/// bar: clean.  The slivers and bars under 30000 µm² are LBE.b2's business.
fn lbe_a(p: &P) {
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

    // h2/h3 - fifty 99.995 × 150 bars, flat and as an array.
    arrays(
        "lbe",
        "LBE.a",
        2,
        vec![rect(l, 0.0, 0.0, 99.995, 150.0)],
        260.0,
    );
}

// --- LBE.b: max. LBE width 1500 ---

/// h1 - the bound and the long side.  (a) 1500 × 200, clean; (b) 1500.005 × 200
/// fires; (c) 200 × 1500.005 fires; (d) an L of 1000 arms 200 wide (its box 1000):
/// clean; (e) two abutting 800 × 200 boxes: one 1600 bar, fires once; (f) two 800 ×
/// 200 boxes 100 apart: two bars of 800, clean.
fn lbe_b(p: &P) {
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

// --- LBE.b1/b2: max. LBE area 250000, min. LBE area 30000 ---

/// h1 - LBE.b1.  (a) 500 × 500, clean; (b) 500 × 500.005 fires; (c) two 500 × 300
/// boxes sharing a wall: 300000, fires once; (d) two 500 × 300 boxes touching at a
/// corner only: two shapes of 150000, clean; (e) a ring 600 outer with 150 walls:
/// 270000, fires (and LBE.h); (f) an L of 300 × 500 and 200 × 300: 210000, clean.
fn lbe_b1(p: &P) {
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
fn lbe_b2(p: &P) {
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

// --- LBE.c: min. LBE space or notch 100 ---

/// h1 - the bound and both metrics.  Pairs of 200 squares: (a) 100 apart, clean; (b)
/// 99.995 apart (at (1000, 1000)): fires; (c) corner to corner dx = dy = 70.7
/// (99.99): fires; (d) dx = dy = 70.715 (100.006): clean; (e) a U 500 × 300 with a
/// 99.995 slot 200 deep: the notch fires; (f) a square whose top-right corner is
/// chamfered (legs 100) against a square whose corner is 99.99 from the 45° wall:
/// fires.
fn lbe_c(p: &P) {
    let l = p.lbe;
    let mut e = vec![
        rect(l, 0.0, 0.0, 200.0, 200.0), // (a)
        rect(l, 300.0, 0.0, 500.0, 200.0),
        rect(l, 1000.0, 1000.0, 1200.0, 1200.0), // (b)
        rect(l, 1299.995, 1000.0, 1499.995, 1200.0),
        rect(l, 600.0, 0.0, 800.0, 200.0), // (c)
        rect(l, 870.7, 270.7, 1070.7, 470.7),
        rect(l, 1200.0, 0.0, 1400.0, 200.0), // (d)
        rect(l, 1470.715, 270.715, 1670.715, 470.715),
        chamfered_tr(l, 0.0, 1000.0, 200.0, 1200.0, 1300.0), // (f)
        rect(l, 220.705, 1220.705, 420.705, 1420.705),
    ];
    e.push(poly(
        l,
        &[
            (0.0, 600.0),
            (500.0, 600.0),
            (500.0, 900.0),
            (300.0, 900.0),
            (300.0, 700.0),
            (200.005, 700.0),
            (200.005, 900.0),
            (0.0, 900.0),
        ],
    )); // (e)
    write("lbe", "LBE.c.h1", e);
}

// --- LBE.d: min. LBE space to the inner edge of EdgeSeal 150 ---

/// h1 - the bound.  EdgeSeal rings 4.2 wide (500 boxes) with an LBE inside: (a) 150
/// from the left inner wall (and further from the rest): clean; (b) 149.995: fires;
/// rings with 45° corners (600 boxes, legs 100, the inner bottom-left chamfer x + y =
/// 105.94 from the box's corner): (c) an LBE whose corner is 149.99 from the diagonal
/// (at 159.025 in both axes): fires; (d) 150.005 (159.04): clean.
fn lbe_d(p: &P) {
    let mut e = vec![];
    for (i, gap) in [(0.0, 150.0), (1.0, 149.995)] {
        let x0 = i * 600.0;
        e.extend(frame(p.seal, x0, 0.0, x0 + 500.0, 500.0, 4.2));
        e.push(rect(p.lbe, x0 + 4.2 + gap, 160.0, x0 + 340.0, 340.0));
    }
    for (i, c) in [(2.0, 159.025), (3.0, 159.04)] {
        let x0 = i * 600.0;
        e.push(oct_frame(
            p.seal,
            x0,
            0.0,
            x0 + 600.0,
            600.0,
            100.0,
            4.2,
            4.2,
        ));
        e.push(rect(p.lbe, x0 + c, c, x0 + 350.0, 350.0));
    }
    write("lbe", "LBE.d.h1", e);
}

// --- LBE.e: min. LBE space to dfpad and Passiv 50 ---

/// h1 - the bound, both metrics and 45°.  An LBE 200 square at (200, 200): (a) a
/// dfpad 50 to its right, clean; (b) a dfpad 49.995 above: fires; (c) a Passiv 49.995
/// to its left: fires; (d) a Passiv square whose corner is dx = dy = 35.35 (49.99)
/// from the LBE's top-right corner: fires; (e) a Passiv square below-left whose
/// chamfered corner (x + y = 329.3) passes 49.99 from the LBE's bottom-left corner:
/// fires; (f) a Passiv square overlapping the LBE's bottom-right corner: 0 (less)
/// apart, fires.
fn lbe_e(p: &P) {
    let e = vec![
        rect(p.lbe, 200.0, 200.0, 400.0, 400.0),
        rect(p.dfpad, 450.0, 200.0, 550.0, 400.0), // (a)
        rect(p.dfpad, 200.0, 449.995, 400.0, 549.995), // (b)
        rect(p.passiv, 50.005, 200.0, 150.005, 400.0), // (c)
        rect(p.passiv, 435.35, 435.35, 535.35, 535.35), // (d)
        chamfered_tr(p.passiv, 0.0, 0.0, 190.0, 190.0, 329.3), // (e)
        rect(p.passiv, 350.0, 150.0, 450.0, 250.0), // (f)
    ];
    write("lbe", "LBE.e.h1", e);
}

// --- LBE.f: min. LBE space to Activ 30 ---

/// h1 - the bound, both metrics and 45°.  An LBE 200 square at (200, 200): (a) an
/// Activ 30 to its right, clean; (b) 29.995 above: fires; (c) an Activ square whose
/// corner is dx = dy = 21.21 (29.995) from the LBE's top-right corner: fires; (d) dx =
/// dy = 21.22 (30.01) from the top-left corner: clean; (e) an Activ square below-left
/// whose chamfered corner (x + y = 357.59) passes 29.99 from the LBE's bottom-left
/// corner: fires.
fn lbe_f(p: &P) {
    let e = vec![
        rect(p.lbe, 200.0, 200.0, 400.0, 400.0),
        rect(p.activ, 430.0, 250.0, 450.0, 350.0), // (a)
        rect(p.activ, 200.0, 429.995, 400.0, 449.995), // (b)
        rect(p.activ, 421.21, 421.21, 441.21, 441.21), // (c)
        rect(p.activ, 158.78, 421.22, 178.78, 441.22), // (d)
        chamfered_tr(p.activ, 100.0, 100.0, 190.0, 190.0, 357.59), // (e)
    ];
    write("lbe", "LBE.f.h1", e);
}

// --- LBE.h: no LBE ring ---

/// h1 - what a ring is.  (a) a ring 500 outer with 100 walls: fires; (b) a U (no top
/// wall): clean; (c) a ring drawn as two abutting U's: one ring after merging, fires;
/// (d) a ring with a 100 square in its hole (100 from every wall): fires once; (e)
/// the ring as one polygon with a hole: fires; (f) a C - the ring with a 0.005 gap in
/// its top wall: no hole, clean (the gap is LBE.c's); (g) a ring with 45° corners:
/// fires.
fn lbe_h(p: &P) {
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

// --- LBE.i: max. global LBE density 20 % ---

/// h1-h4 - the chip.  An EdgeSeal.boundary 1000 square: h1, two LBE 200 × 500 plates
/// (20.000 %): clean; h2, 200 × 500 and 200 × 500.05 (20.001 %): fires; h3, one LBE
/// 300 × 500 reaching 100 beyond the boundary's left edge and one 200 × 500: 20 % of
/// the chip lies under LBE - what is beyond the boundary is no chip (and Seal.l's):
/// clean; h4, no boundary at all, one LBE 300 × 300: no chip to take a density of.
fn lbe_i(p: &P) {
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

// --- LU.a / LU.b: max. space from P+Activ in NWell (N+Activ in PWell) to a tie 20 ---

/// A tie: a 1 × 1 Activ at `(x, y)` with a Cont at its centre, under `impl_` if given.
fn tie(p: &P, x: f64, y: f64, impl_: Option<(i16, i16)>, cont: bool) -> Vec<GdsElement> {
    let mut e = vec![rect(p.activ, x, y, x + 1.0, y + 1.0)];
    if let Some(i) = impl_ {
        e.push(rect(i, x, y, x + 1.0, y + 1.0));
    }
    if cont {
        e.push(rect(p.cont, x + 0.42, y + 0.42, x + 0.58, y + 0.58));
    }
    e
}

/// h1 - LU.a.  "Any portion" of the P+Activ is within the value, so a shape is read
/// by its farthest point.  Separate NWells, each with an N+ tie (1 × 1 Activ, Cont)
/// and a P+Activ under pSD: (a) a 2 × 2 P+ whose far edge is 20 from the tie, clean;
/// (b) 20.005: fires; (c) a 1.5 square whose far corner is dx = dy = 14.5 (20.5) from
/// the tie's corner, under 20 on either axis alone: fires; (d) one whose far corner is
/// dx = dy = 14.14 (19.997): clean; (e) a P+ bar 52 long whose near end is 5 from the
/// tie and whose far end 57: fires; (f) the tie in its own NWell 4 away from the
/// P+'s well, 9 from the P+: no tie in the P+'s well, fires; (g) an uncontacted
/// N+Activ 10 from the P+: section 4.2's tie needs no Cont, clean; (h) a P+ alone in
/// a well at (1000, 1000): fires.
fn lu_a(p: &P) {
    let psd = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![rect(p.activ, x0, y0, x1, y1), rect(p.psd, x0, y0, x1, y1)]
    };
    let mut e = vec![];
    for (i, dx) in [(0.0, 20.0), (1.0, 20.005)] {
        let y = i * 30.0;
        e.push(rect(p.nwell, 0.0, y, 40.0, y + 20.0));
        e.extend(tie(p, 2.0, y + 9.5, None, true));
        e.extend(psd(1.0 + dx, y + 9.0, 3.0 + dx, y + 11.0));
    }
    for (i, d) in [(2.0, 14.5), (3.0, 14.14)] {
        let y = i * 30.0;
        e.push(rect(p.nwell, 0.0, y, 40.0, y + 20.0));
        e.extend(tie(p, 2.0, y + 2.0, None, true));
        e.extend(psd(1.5 + d, y + 1.5 + d, 3.0 + d, y + 3.0 + d));
    }
    e.push(rect(p.nwell, 0.0, 120.0, 80.0, 140.0)); // (e)
    e.extend(tie(p, 2.0, 129.5, None, true));
    e.extend(psd(8.0, 129.0, 60.0, 131.0));
    e.push(rect(p.nwell, 0.0, 150.0, 6.0, 170.0)); // (f)
    e.push(rect(p.nwell, 10.0, 150.0, 50.0, 170.0));
    e.extend(tie(p, 2.0, 159.5, None, true));
    e.extend(psd(12.0, 159.0, 14.0, 161.0));
    e.push(rect(p.nwell, 0.0, 180.0, 40.0, 200.0)); // (g)
    e.extend(tie(p, 2.0, 189.5, None, false));
    e.extend(psd(13.0, 189.0, 15.0, 191.0));
    e.push(rect(p.nwell, 1000.0, 1000.0, 1040.0, 1020.0)); // (h)
    e.extend(psd(1010.0, 1009.0, 1012.0, 1011.0));
    write("lu", "LU.a.h1", e);

    // h2/h3 - fifty wells with a tie and a P+ whose far edge is 20.005 from it, flat
    // and as an array.
    let mut cell = vec![rect(p.nwell, 0.0, 0.0, 30.0, 10.0)];
    cell.extend(tie(p, 2.0, 4.5, None, true));
    cell.extend(psd(21.005, 4.0, 23.005, 6.0));
    arrays("lu", "LU.a", 2, cell, 60.0);
}

/// h1 - LU.b, as LU.a.  In the substrate, a P+ tie (1 × 1 Activ under pSD, Cont)
/// and an N+Activ (Activ, no pSD): (a) a 2 × 2 N+ whose far edge is 20 from the tie,
/// clean; (b) 20.005: fires; (c) a 1.5 square whose far corner is 20.5 from the
/// tie's corner (14.5 on each axis): fires; (d) 19.997 (14.14): clean; (e) an N+ bar
/// 52 long, 5 from the tie at its near end: fires; (f) an N+ under a PWell:block 9
/// from a tie: in no well (section 4.2), not read, clean; (g) an uncontacted P+ 10
/// from the N+: clean; (h) an N+ alone at (1000, 1000): fires.
fn lu_b(p: &P) {
    let n = |x0: f64, y0: f64, x1: f64, y1: f64| rect(p.activ, x0, y0, x1, y1);
    let mut e = vec![];
    for (i, dx) in [(0.0, 20.0), (1.0, 20.005)] {
        let y = i * 30.0;
        e.extend(tie(p, 2.0, y + 9.5, Some(p.psd), true));
        e.push(n(1.0 + dx, y + 9.0, 3.0 + dx, y + 11.0));
    }
    for (i, d) in [(2.0, 14.5), (3.0, 14.14)] {
        let y = i * 30.0;
        e.extend(tie(p, 2.0, y + 2.0, Some(p.psd), true));
        e.push(n(1.5 + d, y + 1.5 + d, 3.0 + d, y + 3.0 + d));
    }
    e.extend(tie(p, 2.0, 129.5, Some(p.psd), true)); // (e)
    e.push(n(8.0, 129.0, 60.0, 131.0));
    e.extend(tie(p, 2.0, 159.5, Some(p.psd), true)); // (f)
    e.push(rect(p.pwb, 10.0, 150.0, 30.0, 170.0));
    e.push(n(12.0, 159.0, 14.0, 161.0));
    e.extend(tie(p, 2.0, 189.5, Some(p.psd), false)); // (g)
    e.push(n(13.0, 189.0, 15.0, 191.0));
    e.push(n(1010.0, 1009.0, 1012.0, 1011.0)); // (h)
    write("lu", "LU.b.h1", e);
}

// --- LU.c/c1/d/d1: max. extension of a tie beyond its Cont 6 ---

/// The tie extension layouts, in an NWell (`nwell`, N+ ties abutting a PMOS) or in
/// the substrate (P+ ties abutting an NMOS).  (a) a tie 1 × 12.16 with its Cont
/// centred: 6.0 each way, clean; (b) 12.17 with the Cont 6.005 from its left end:
/// fires (LU.d/d1 - the tie stands alone); (c) a square tie 12.16 with the Cont
/// centred: 6.0 beyond the Cont on every side, the corners 8.49 away diagonally -
/// the extension is 6, clean; (d) a tie 24.33 long with two Conts 12.01 apart: its
/// middle is 6.005 from either, fires (LU.d/d1); (e) figure 7.4's abutted tie: a
/// transistor (Activ 10 × 8 under a 1 wide gate, four Conts in its left source) with
/// an Activ 4 wide of the other type abutting below the source and reaching 5.92
/// below the lowest Cont: clean; (f) the same reaching 6.005 below: fires (LU.c/c1 -
/// abutted); (g) a 4 × 2 tie with no Cont at all: fires (LU.d/d1); (h) (b) again at
/// (1000, 1000).
fn lu_cd(p: &P, nwell: bool) {
    let (tie_impl, sd_impl) = if nwell {
        (None, Some(p.psd))
    } else {
        (Some(p.psd), None)
    };
    let mut e = vec![];
    let well = |e: &mut Vec<GdsElement>, y0: f64, y1: f64| {
        if nwell {
            e.push(rect(p.nwell, 0.0, y0, 40.0, y1));
        }
    };
    let tie_box = |e: &mut Vec<GdsElement>, x0: f64, y0: f64, x1: f64, y1: f64| {
        e.push(rect(p.activ, x0, y0, x1, y1));
        if let Some(i) = tie_impl {
            e.push(rect(i, x0, y0, x1, y1));
        }
    };
    let cont = |e: &mut Vec<GdsElement>, x: f64, y: f64| {
        e.push(rect(p.cont, x, y, x + 0.16, y + 0.16));
    };
    well(&mut e, 0.0, 20.0); // (a)
    tie_box(&mut e, 5.5, 9.0, 17.66, 10.0);
    cont(&mut e, 11.5, 9.42);
    well(&mut e, 30.0, 50.0); // (b)
    tie_box(&mut e, 5.495, 39.0, 17.665, 40.0);
    cont(&mut e, 11.5, 39.42);
    well(&mut e, 60.0, 80.0); // (c)
    tie_box(&mut e, 5.0, 63.0, 17.16, 75.16);
    cont(&mut e, 11.0, 69.0);
    well(&mut e, 90.0, 110.0); // (d)
    tie_box(&mut e, 0.0, 99.0, 24.33, 100.0);
    cont(&mut e, 6.0, 99.42);
    cont(&mut e, 18.17, 99.42);
    for (i, ext) in [(0.0, 5.92), (1.0, 6.005)] {
        // (e), (f): the transistor at y0 + 7..15, its lowest Cont at y0 + 7.92
        let y0 = 118.0 + i * 30.0;
        well(&mut e, y0, y0 + 22.0);
        e.push(rect(p.activ, 5.0, y0 + 7.0, 15.0, y0 + 15.0));
        if let Some(i) = sd_impl {
            e.push(rect(i, 4.5, y0 + 7.0, 15.5, y0 + 15.5));
        }
        e.push(rect(p.gp, 9.5, y0 + 6.0, 10.5, y0 + 16.0));
        for k in 0..4 {
            cont(&mut e, 6.92, y0 + 7.92 + k as f64 * 2.0);
        }
        tie_box(&mut e, 5.0, y0 + 7.92 - ext, 9.0, y0 + 7.0);
    }
    well(&mut e, 180.0, 200.0); // (g)
    tie_box(&mut e, 5.0, 189.0, 9.0, 191.0);
    if nwell {
        e.push(rect(p.nwell, 1000.0, 1000.0, 1040.0, 1020.0)); // (h)
    }
    tie_box(&mut e, 1005.495, 1009.0, 1017.665, 1010.0);
    cont(&mut e, 1011.5, 1009.42);
    write("lu", if nwell { "LU.c.h1" } else { "LU.c1.h1" }, e);
}
