// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::sealring::{P, SQRT2, frame, grid, write};
use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, strip45, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/slit";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    slt_a(pdk);
    slt_b(pdk);
    slt_c(pdk);
    slt_e(pdk);
    slt_f(pdk);
    slt_h1(pdk);
    slt_i(pdk);

    hardening(pdk);
}

/// A square Metal1 plate with one slit slot inside it.  `sw`×`sh` slit centred in a
/// `plate`×`plate` plate (≥ 1 µm enclosure as long as the slit is small enough).
fn plated_slit(
    m: (i16, i16),
    s: (i16, i16),
    ox: f64,
    oy: f64,
    plate: f64,
    sw: f64,
    sh: f64,
) -> Vec<GdsElement> {
    let cx = ox + plate / 2.0;
    let cy = oy + plate / 2.0;
    vec![
        rect(m, ox, oy, ox + plate, oy + plate),
        rect(
            s,
            cx - sw / 2.0,
            cy - sh / 2.0,
            cx + sw / 2.0,
            cy + sh / 2.0,
        ),
    ]
}

/// Slt.a — min. slit width 2.80.  Clean 2.80-wide slit, plus one only 2.795 wide.
fn slt_a(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 12.0, 2.80, 6.0); // clean
    elems.extend(plated_slit(m, s, o + 20.0, o, 12.0, 2.795, 6.0)); // too narrow → Slt.a
    write_gz(&format!("{DIR}/Slt.a.gds.gz"), library("TOP", elems));
}

/// Slt.b — max. slit width 20.00.  Clean 20.0-wide slit, plus one 20.005 wide.
fn slt_b(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Plate 24 wide keeps width < 30 (no Slt.c) and < 35 (no Slt.i); 1 µm enclosure.
    let mut elems = plated_slit(m, s, o, o, 24.0, 20.0, 6.0); // clean
    elems.extend(plated_slit(m, s, o + 40.0, o, 24.0, 20.005, 6.0)); // too wide → Slt.b
    write_gz(&format!("{DIR}/Slt.b.gds.gz"), library("TOP", elems));
}

/// Slt.c — max. metal width without a slit 30.00.  A 32×32 plate with no slit is wider than
/// 30 µm everywhere → violation; an identical plate with a slit is clean; a 20-µm-wide wire
/// is never wide enough to need one.
fn slt_c(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Wide plate, no slit → Slt.c (32 < 35, so no Slt.i).
    let mut elems = vec![rect(m, o, o, o + 32.0, o + 32.0)];
    // Wide plate with a centred slit → clean.
    let bx = o + 40.0;
    elems.extend(plated_slit(m, s, bx, o, 32.0, 2.80, 6.0));
    // 20-µm-wide wire (≤ 30 wide) → opened away, never flagged.
    elems.push(rect(m, o, o + 40.0, o + 20.0, o + 100.0));
    // Self-slotted plate (as IO power buses are drawn): a 40-µm footprint cut into 6-µm
    // bars by 2-µm slots in the metal itself.  Every piece is < 30 µm, so it must stay
    // clean — the erosion would fill the slots, but the exact disk check sees them.
    let sx = o + 100.0;
    for i in 0..6 {
        let y = o + i as f64 * 8.0;
        elems.push(rect(m, sx, y, sx + 40.0, y + 6.0));
    }
    write_gz(&format!("{DIR}/Slt.c.gds.gz"), library("TOP", elems));
}

/// Slt.e — no slits on pads:
/// - a pad (TopMetal2 + dfpad + Passiv opening) with a TopMetal2 slit AND a Metal1
///   plate with a slit under it → fires twice (once per metal — the pad region is the
///   whole dfpad shape, on every layer);
/// - a plain slotted TopMetal2 plate away from any pad → clean;
/// - canary: a dfpad with NO Passiv opening (not a pad) over a slotted plate → clean.
fn slt_e(pdk: &PdkConfig) {
    let tm2 = layer(pdk, "TopMetal2");
    let ts = layer(pdk, "TopMetal2.slit");
    let o = OFFSET;
    let c = o + 45.0;
    let mut elems = vec![
        rect(tm2, o, o, o + 90.0, o + 90.0),
        rect(layer(pdk, "dfpad"), c - 40.0, c - 40.0, c + 40.0, c + 40.0),
        rect(layer(pdk, "Passiv"), c - 35.0, c - 35.0, c + 35.0, c + 35.0),
        rect(ts, c - 20.0, c - 5.0, c - 17.0, c + 5.0), // on the pad → fires
    ];
    elems.extend(plated_slit(
        layer(pdk, "Metal1"),
        layer(pdk, "Metal1.slit"),
        c + 5.0,
        c - 12.5,
        25.0,
        3.0,
        10.0,
    )); // under the same pad → fires
    elems.extend(plated_slit(tm2, ts, o + 120.0, o, 25.0, 3.0, 10.0)); // no pad → clean
    let bx = o + 160.0;
    elems.extend(plated_slit(tm2, ts, bx, o, 25.0, 3.0, 10.0));
    elems.push(rect(
        layer(pdk, "dfpad"),
        bx + 2.5,
        o + 2.5,
        bx + 22.5,
        o + 22.5,
    )); // no Passiv → clean
    write_gz(&format!("{DIR}/Slt.e.gds.gz"), library("TOP", elems));
}

/// Slt.f — min. metal enclosure of slit 1.00.  Clean 1.0-µm enclosure, plus one where the
/// slit reaches 0.995 µm from the plate's right edge.
fn slt_f(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 12.0, 2.80, 6.0); // 4.6 µm enclosure → clean
    // Plate 6 wide, slit 2.8 wide pushed right so right enclosure = 0.995 < 1.00.
    let bx = o + 20.0;
    elems.push(rect(m, bx, o, bx + 6.0, o + 8.0));
    elems.push(rect(
        s,
        bx + 6.0 - 0.995 - 2.8,
        o + 1.0,
        bx + 6.0 - 0.995,
        o + 7.0,
    ));
    write_gz(&format!("{DIR}/Slt.f.gds.gz"), library("TOP", elems));
}

/// Slt.h1 — min. Metal1:slit space to Cont and Via1 0.30.  A slit 0.295 µm from a Via1.
fn slt_h1(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let v = layer(pdk, "Via1");
    let o = OFFSET;
    let mut elems = plated_slit(m, s, o, o, 16.0, 2.80, 6.0);
    // Via1 just 0.295 µm to the right of the slit's right edge (slit right = cx + 1.4).
    let cx = o + 8.0;
    let sright = cx + 1.4;
    elems.push(rect(
        v,
        sright + 0.295,
        o + 6.0,
        sright + 0.295 + 0.19,
        o + 6.19,
    ));
    write_gz(&format!("{DIR}/Slt.h1.gds.gz"), library("TOP", elems));
}

/// Slt.i — min. slit density 6 % on metal plates > 35×35 µm.  A 40×40 plate with a single
/// small slit (≈1 %) fails; a 40×40 plate slit at ≥6 % is clean.
fn slt_i(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let s = layer(pdk, "Metal1.slit");
    let o = OFFSET;
    // Starved plate: 40×40 = 1600 µm²; one 2.8×6 = 16.8 µm² slit → ~1.05 % → Slt.i.
    let mut elems = vec![rect(m, o, o, o + 40.0, o + 40.0)];
    elems.push(rect(s, o + 18.6, o + 17.0, o + 21.4, o + 23.0));
    // Healthy plate: 40×40 with slits totalling ≥ 6 % (96 µm²).  Six 3×6 slits = 108 µm².
    let bx = o + 50.0;
    elems.push(rect(m, bx, o, bx + 40.0, o + 40.0));
    for i in 0..3 {
        for j in 0..2 {
            let sx = bx + 6.0 + i as f64 * 12.0;
            let sy = o + 8.0 + j as f64 * 18.0;
            elems.push(rect(s, sx, sy, sx + 3.0, sy + 6.0));
        }
    }
    write_gz(&format!("{DIR}/Slt.i.gds.gz"), library("TOP", elems));
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
fn slt_a_h(p: &P) {
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
}

/// h1 - the bound and the long side.  Figure 7.5 draws `b` across the slit's long
/// side, `a` across its short one.  Metal1 slits in plates 2 wider (enclosure 2, every
/// plate under 30): (a) 20 × 6, clean; (b) 20.005 × 6 fires; (c) 6 × 20.005 fires; (d)
/// 3 × 25: 3 wide and 25 long - `b` is 25, fires; (e) 20 × 20, clean; (f) 20.005 ×
/// 20.005 fires; (g) a 45° strip 3 wide (`d` 2.12) running 15 along each axis, 21.2
/// long: fires; (h) an L-shaped slit, arms 3 wide and 15 long, in a 15 box: clean.
fn slt_b_h(p: &P) {
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
fn slt_c_h(p: &P) {
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
}

/// h1 - the pad's extent.  A pad: TopMetal2 90 × 90, dfpad 80 × 80 (5..85), Passiv
/// opening 70 × 70: (a) a TopMetal2 slit 3 × 10 inside the dfpad fires; (b) one across
/// the dfpad's right edge (83.5..86.5) fires; (c) one touching the dfpad's edge from
/// outside (85..88) is off the pad, clean; (d) a slotted TopMetal2 plate under a dfpad
/// with no Passiv (not a pad): clean.
fn slt_e_h(p: &P) {
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

/// h1 - the bound.  Metal1 plates 12 × 12 with a 2.8 × 6 slit 3 from the top and
/// bottom: (a) 1.0 from the right wall, clean; (b) 0.995 from the right wall, fires;
/// (c) 0.995 from all four walls (a 4.79 × 7.99 plate): fires; (d) a slit on the
/// plate's right edge (0): fires; (e) a slit crossing the plate's right edge by 1:
/// fires; (f) a slit with no metal at all: enclosed by nothing, fires; (g) a plate
/// whose top-right corner is chamfered so its 45° wall passes 0.9935 from the slit's
/// top-right corner: fires; (h) the same at 1.0006: clean; (i) a plate ending on x =
/// 20 with the slit 0.995 from that edge; (j) a plate across x = 40 with the slit's
/// right wall 0.995 from the plate's at 44.6; (k) at (1000, 1000).
fn slt_f_h(p: &P) {
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
fn slt_i_h(p: &P) {
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
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/slit")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    slt_a_h(&p);
    slt_b_h(&p);
    slt_c_h(&p);
    slt_e_h(&p);
    slt_f_h(&p);
    slt_g(&p);
    slt_h(&p);
    slt_i_h(&p);
}
