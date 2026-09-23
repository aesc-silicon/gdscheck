// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nmosi";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nmosi_b(pdk);
    nmosi_c(pdk);
    nmosi_d(pdk);
    nmosi_f(pdk);
    nmosi_g(pdk);

    hardening(pdk);
}

fn lr(pdk: &PdkConfig, name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    rect(layer(pdk, name), x0, y0, x1, y1)
}

/// An NWell ring as four rects: the box `(x0, y0)-(x1, y1)` less the hole `(hx0, hy0)-
/// (hx1, hy1)`.  Section 6.5's rules are tested inside a closed ring of NWell AND
/// nBuLay only, so every pattern below sits in one.
#[allow(clippy::too_many_arguments)]
fn nwell_ring(
    pdk: &PdkConfig,
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
        lr(pdk, "NWell", x0, y0, x1, hy0),
        lr(pdk, "NWell", x0, hy1, x1, y1),
        lr(pdk, "NWell", x0, hy0, hx0, hy1),
        lr(pdk, "NWell", hx1, hy0, x1, hy1),
    ]
}

/// nmosi.b — an iso-PWell Activ enclosed by nBuLay by only 1.235 µm (< 1.24), in a
/// ring 0.4 from the Activ that reaches the nBuLay's edge (0.835 wide).
fn nmosi_b(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 10.0, o + 10.0),
        lr(pdk, "Activ", o + 1.235, o + 1.235, o + 8.765, o + 8.765), // inset 1.235 < 1.24
    ];
    e.extend(nwell_ring(
        pdk,
        o,
        o,
        o + 10.0,
        o + 10.0,
        o + 0.835,
        o + 0.835,
        o + 9.165,
        o + 9.165,
    ));
    write_gz(&format!("{DIR}/nmosi.b.gds.gz"), library("TOP", e));
}

/// One nmosi.c instance: an NWell ring (outer half-extent 5.0, width 1.0) on nBuLay,
/// with the iso-PWell Activ sitting `gap` µm inside the ring's hole (hole half 4.0).
fn nmosi_c_ring(pdk: &PdkConfig, cx: f64, cy: f64, gap: f64) -> Vec<GdsElement> {
    let ah = 4.0 - gap;
    let mut e = vec![
        lr(pdk, "nBuLay", cx - 6.5, cy - 6.5, cx + 6.5, cy + 6.5),
        lr(pdk, "Activ", cx - ah, cy - ah, cx + ah, cy + ah),
    ];
    // NWell ring as four rects (outer 5.0, inner 4.0).
    e.push(lr(pdk, "NWell", cx - 5.0, cy - 5.0, cx + 5.0, cy - 4.0)); // bottom
    e.push(lr(pdk, "NWell", cx - 5.0, cy + 4.0, cx + 5.0, cy + 5.0)); // top
    e.push(lr(pdk, "NWell", cx - 5.0, cy - 4.0, cx - 4.0, cy + 4.0)); // left
    e.push(lr(pdk, "NWell", cx + 4.0, cy - 4.0, cx + 5.0, cy + 4.0)); // right
    e
}

/// nmosi.c — min. NWell space to iso-PWell Activ (0.39), rings only:
/// - ring with gap 0.34 → fires;
/// - ring with gap 0.50 → clean;
/// - canary: a plain (hole-free) NWell 0.35 from an iso-PWell Activ → silent, because
///   only `NWell.with_holes` (the isolation ring) anchors this rule (0.35 also clears
///   KLayout's NW.d "NWell space to external N+Activ = 0.31" so the container
///   cross-check stays collateral-free).
fn nmosi_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nmosi_c_ring(pdk, o, o, 0.34);
    e.extend(nmosi_c_ring(pdk, o + 30.0, o, 0.50));
    let cx = o + 60.0;
    e.push(lr(pdk, "nBuLay", cx - 4.0, o - 4.0, cx + 6.0, o + 4.0));
    e.push(lr(pdk, "Activ", cx - 1.5, o - 1.5, cx + 1.5, o + 1.5));
    e.push(lr(pdk, "NWell", cx + 1.85, o - 1.5, cx + 3.85, o + 1.5)); // 0.35 away, no hole
    write_gz(&format!("{DIR}/nmosi.c.gds.gz"), library("TOP", e));
}

/// nmosi.d — the NWell∩nBuLay ring round an iso-PWell Activ only 0.50 µm wide (< 0.62)
/// on its right leg: the ring 0.75 from the Activ, 0.8 wide on three sides, and the
/// nBuLay ending 0.5 into the right leg (1.25 past the Activ, so nmosi.b holds).
fn nmosi_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 8.25, o + 9.0),
        lr(pdk, "Activ", o + 2.0, o + 2.0, o + 7.0, o + 7.0),
    ];
    e.extend(nwell_ring(
        pdk,
        o + 0.45,
        o + 0.45,
        o + 9.0,
        o + 8.55,
        o + 1.25,
        o + 1.25,
        o + 7.75,
        o + 7.75,
    ));
    write_gz(&format!("{DIR}/nmosi.d.gds.gz"), library("TOP", e));
}

/// nmosi.f — an nSD:block strip 0.50 µm wide (< 0.62) touching the iso-PWell Activ.
fn nmosi_f(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![
        lr(pdk, "nBuLay", o, o, o + 12.0, o + 10.0),
        lr(pdk, "Activ", o + 2.0, o + 2.0, o + 10.0, o + 8.0), // inset 2.0 > 1.24 → nmosi.b clean
        // nSD:block 0.50 wide strip overlapping the Activ.
        lr(pdk, "nSD.block", o + 4.0, o + 2.0, o + 4.5, o + 8.0),
    ];
    // The ring: 0.5 from the Activ, 0.8 wide.
    e.extend(nwell_ring(
        pdk,
        o + 0.7,
        o + 0.7,
        o + 11.3,
        o + 9.3,
        o + 1.5,
        o + 1.5,
        o + 10.5,
        o + 8.5,
    ));
    write_gz(&format!("{DIR}/nmosi.f.gds.gz"), library("TOP", e));
}

/// One nmosi.g instance: an iso-PWell Activ wider than its nSD:block on the right (the
/// ptap area), with SalBlock extending `ext` past the nSD:block edge toward the ptap.
/// nSD:block and SalBlock end flush with Activ on the other three sides (bands there fall
/// outside Activ and are correctly out of scope).
fn nmosi_g_instance(pdk: &PdkConfig, x: f64, y: f64, ext: f64) -> Vec<GdsElement> {
    let mut e = vec![
        lr(pdk, "nBuLay", x - 2.0, y - 2.0, x + 8.0, y + 5.0),
        lr(pdk, "Activ", x, y, x + 5.0, y + 3.0), // x+3..x+5 is the adjacent ptap area
        lr(pdk, "nSD.block", x, y, x + 3.0, y + 3.0),
        lr(pdk, "SalBlock", x, y, x + 3.0 + ext, y + 3.0),
    ];
    // The ring: 0.5 from the Activ, 0.8 wide.
    e.extend(nwell_ring(
        pdk,
        x - 1.3,
        y - 1.3,
        x + 6.3,
        y + 4.3,
        x - 0.5,
        y - 0.5,
        x + 5.5,
        y + 3.5,
    ));
    e
}

/// nmosi.g — SalBlock overlap of nSD:block over Activ ≥ 0.15:
/// - ext 0.05 (< 0.15) → fires (matches KLayout).
/// - ext 0.20 → clean (matches KLayout).
/// - ext 0.00, flush → fires HERE only: overlap 0 violates the PDF text, but KLayout's
///   coincident-pair marker has zero area and vanishes under its `.and(Activ)` — a
///   degenerate-marker artifact we deliberately don't reproduce.
fn nmosi_g(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nmosi_g_instance(pdk, o, o, 0.05);
    e.extend(nmosi_g_instance(pdk, o + 15.0, o, 0.20));
    e.extend(nmosi_g_instance(pdk, o + 30.0, o, 0.00));
    write_gz(&format!("{DIR}/nmosi.g.gds.gz"), library("TOP", e));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the last three decks of the SG13G2 set: section 6.5 (nmosi and
// nmosiHV, nmosi.b-nmosi.g, with section 4.2's Iso-PWell-Activ), section 7.4 (Pin.a-Pin.h)
// and section 7.1 (Ant.a-Ant.i, the net-aware antenna ratios).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//
// The nmosi layouts share one structure, `iso`: an Activ in the hole of a closed NWell
// ring, the whole on nBuLay, so that the Activ is Iso-PWell-Activ (Activ AND nBuLay AND
// PWell) and the ring is what the manual tests the rules inside of.  The antenna
// layouts share one gate, `gate`: a 0.2 µm GatPoly strip over a 1.0 × 0.5 Activ (gate
// area 0.1 µm², 0.12 µm² of poly over field) with a 0.1 × 0.1 Cont on the poly; the
// areas of the antennas are what the boxes draw, so every ratio is computed in the
// comment next to it.

/// A box as `(x0, y0, x1, y1)`.
pub(super) type R = (f64, f64, f64, f64);

/// The ring between `outer` and `hole` as four boxes: bottom and top run the full
/// width, left and right fill the sides between them.
pub(super) fn ring4(l: (i16, i16), outer: R, hole: R) -> Vec<GdsElement> {
    vec![
        rect(l, outer.0, outer.1, outer.2, hole.1),
        rect(l, outer.0, hole.3, outer.2, outer.3),
        rect(l, outer.0, hole.1, hole.0, hole.3),
        rect(l, hole.2, hole.1, outer.2, hole.3),
    ]
}

pub(super) struct P {
    pub(super) activ: (i16, i16),
    pub(super) gp: (i16, i16),
    pub(super) cont: (i16, i16),
    pub(super) m1: (i16, i16),
    pub(super) via1: (i16, i16),
    pub(super) m2: (i16, i16),
    pub(super) via2: (i16, i16),
    pub(super) m3: (i16, i16),
    pub(super) via3: (i16, i16),
    pub(super) m4: (i16, i16),
    pub(super) via4: (i16, i16),
    pub(super) m5: (i16, i16),
    pub(super) tv1: (i16, i16),
    pub(super) tm1: (i16, i16),
    pub(super) tv2: (i16, i16),
    pub(super) tm2: (i16, i16),
    pub(super) psd: (i16, i16),
    pub(super) nsdb: (i16, i16),
    pub(super) nw: (i16, i16),
    pub(super) pw: (i16, i16),
    pub(super) pwb: (i16, i16),
    pub(super) nbl: (i16, i16),
    pub(super) sal: (i16, i16),
    pub(super) tgo: (i16, i16),
    pub(super) diode: (i16, i16),
    pub(super) esd: (i16, i16),
    pub(super) txt: (i16, i16),
    pub(super) m1_label: (i16, i16),
}

impl P {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        P {
            activ: layer(pdk, "Activ"),
            gp: layer(pdk, "GatPoly"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            via1: layer(pdk, "Via1"),
            m2: layer(pdk, "Metal2"),
            via2: layer(pdk, "Via2"),
            m3: layer(pdk, "Metal3"),
            via3: layer(pdk, "Via3"),
            m4: layer(pdk, "Metal4"),
            via4: layer(pdk, "Via4"),
            m5: layer(pdk, "Metal5"),
            tv1: layer(pdk, "TopVia1"),
            tm1: layer(pdk, "TopMetal1"),
            tv2: layer(pdk, "TopVia2"),
            tm2: layer(pdk, "TopMetal2"),
            psd: layer(pdk, "pSD"),
            nsdb: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            pw: layer(pdk, "PWell"),
            pwb: layer(pdk, "PWell.block"),
            nbl: layer(pdk, "nBuLay"),
            sal: layer(pdk, "SalBlock"),
            tgo: layer(pdk, "ThickGateOx"),
            diode: layer(pdk, "Recog.diode"),
            esd: layer(pdk, "Recog.esd"),
            txt: layer(pdk, "TEXT"),
            m1_label: layer(pdk, "Metal1.label"),
        }
    }
}

const NMOSI: &str = "tests/data/ihp-sg13g2/nmosi";

fn grown(r: R, d: f64) -> R {
    (r.0 - d, r.1 - d, r.2 + d, r.3 + d)
}

fn rb(l: (i16, i16), r: R) -> GdsElement {
    rect(l, r.0, r.1, r.2, r.3)
}

/// The same ring as one boundary: the outer contour, a slit at the bottom-left, the
/// hole's contour backwards, and out again.
fn ring_keyhole(l: (i16, i16), outer: R, hole: R) -> GdsElement {
    let (ox0, oy0, ox1, oy1) = outer;
    let (hx0, hy0, hx1, hy1) = hole;
    poly(
        l,
        &[
            (ox0, oy0),
            (ox1, oy0),
            (ox1, oy1),
            (ox0, oy1),
            (ox0, hy0),
            (hx0, hy0),
            (hx0, hy1),
            (hx1, hy1),
            (hx1, hy0),
            (ox0, hy0),
        ],
    )
}

/// The isolated structure: `act` on nBuLay, in the hole of a closed NWell ring.  The
/// hole is `act` grown by `gap` (nmosi.c reads it, 0.39), the ring `ring` wide
/// (nmosi.d, 0.62), the nBuLay the ring's outer box grown by `ext` - so the nBuLay
/// encloses the Activ by `gap + ring + ext` (nmosi.b, 1.24).
fn iso(p: &P, act: R, gap: f64, ring: f64, ext: f64) -> Vec<GdsElement> {
    let hole = grown(act, gap);
    let outer = grown(hole, ring);
    let mut e = vec![rb(p.activ, act), rb(p.nbl, grown(outer, ext))];
    e.extend(ring4(p.nw, outer, hole));
    e
}

/// The structure with every box given: the Activ, the ring's hole and outer box, the
/// nBuLay.
fn iso_r(p: &P, act: R, hole: R, outer: R, nbl: R) -> Vec<GdsElement> {
    let mut e = vec![rb(p.activ, act), rb(p.nbl, nbl)];
    e.extend(ring4(p.nw, outer, hole));
    e
}

/// The standard 2.0 × 1.0 Activ at `(x, y)`.
fn act_at(x: f64, y: f64) -> R {
    (x, y, x + 2.0, y + 1.0)
}

/// The standard structure (gap 0.4, ring 0.7) with the nBuLay `enc` past the Activ on
/// every side but the one given in `side` (0 right, 1 top, 2 left, 3 bottom), which
/// gets `enc_side`.
fn iso_enc(p: &P, act: R, enc: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let mut nbl = grown(act, enc);
    if let Some((s, v)) = side {
        match s {
            0 => nbl.2 = act.2 + v,
            1 => nbl.3 = act.3 + v,
            2 => nbl.0 = act.0 - v,
            _ => nbl.1 = act.1 - v,
        }
    }
    iso_r(p, act, grown(act, 0.4), grown(act, 1.1), nbl)
}

/// nmosi.b - "Min. nBuLay enclosure of Iso-PWell-Activ  1.24".
fn nmosi_b_h(p: &P) {
    // h1, the bound: the nBuLay 1.24 past the Activ on every side (clean); 1.235 on the
    // right (fires); 1.235 on top (fires); 1.235 all round (fires four times).
    let mut e = iso_enc(p, act_at(2.0, 4.0), 1.24, None);
    e.extend(iso_enc(p, act_at(9.0, 4.0), 1.4, Some((0, 1.235))));
    e.extend(iso_enc(p, act_at(16.0, 4.0), 1.4, Some((1, 1.235))));
    e.extend(iso_enc(p, act_at(2.0, 12.0), 1.235, None));
    write_gz(&format!("{NMOSI}/nmosi.b.h1.gds.gz"), library("TOP", e));

    // h2, the 45° walls.  The nBuLay walls 1.3 from the Activ and its top-right corner
    // chamfered along x + y = k: the chamfer passes 0.875·√2 = 1.2374 from the Activ's
    // corner at x = 2 (fires, closest approach), 0.88·√2 = 1.2445 at x = 9 (clean).  At
    // x = 16 the nBuLay is a diamond whose walls pass 1.2374 from all four Activ corners
    // (fires four times); at x = 24 the same diamond 1.2445 away (clean).  The NWell
    // ring's outer corners poke out of the chamfered nBuLay; the NWell AND nBuLay ring
    // stays 0.67 wide there.
    let mut e = vec![];
    for (x, a) in [(2.0, 0.875), (9.0, 0.88)] {
        let act = act_at(x, 4.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        let n = grown(act, 1.3);
        e.push(chamfered_tr(
            p.nbl,
            n.0,
            n.1,
            n.2,
            n.3,
            act.2 + act.3 + 2.0 * a,
        ));
    }
    for (x, a) in [(16.0, 0.875), (24.0, 0.88)] {
        let act = act_at(x, 4.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        // Corner (1.0, 0.5) off the centre to the wall x + y = A: (A − 1.5)/√2 = a·√2.
        e.push(diamond(p.nbl, x + 1.0, 4.5, 1.5 + 2.0 * a));
    }
    write_gz(&format!("{NMOSI}/nmosi.b.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines: 1.235 on the right with the nBuLay's edge on x = 20; the
    // Activ's edge on x = 40 (the gap straddles the line); the structure across x = 42;
    // the nBuLay's edge on x = 21; one at (1000, 1000); a 300 µm long Activ 1.235 short on
    // top across every line.  Beside them 1.24 exactly with the nBuLay's edge on x = 60.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.235 - 2.0, 4.0),
        (38.0, 4.0),
        (39.9, 12.0),
        (21.0 - 1.235 - 2.0, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_enc(p, act_at(x, y), 1.4, Some((0, 1.235))));
    }
    e.extend(iso_enc(p, act_at(60.0 - 1.24 - 2.0, 4.0), 1.24, None));
    e.extend(iso_enc(p, (70.0, 4.0, 370.0, 5.0), 1.4, Some((1, 1.235))));
    write_gz(&format!("{NMOSI}/nmosi.b.h3.gds.gz"), library("TOP", e));

    // h6, the drawing: the nBuLay as two overlapping boxes whose union is the 1.24 box
    // (clean); as two boxes whose union ends 1.235 right of the Activ (fires); as a 4 × 3
    // grid of abutting tiles at 1.24 (clean); the Activ as three abutting slices, 1.235
    // on the right (fires); the nBuLay as a cross of two boxes whose re-entrant corners
    // lie 0.87·√2 = 1.2304 from the Activ's corners (fires four times); the same cross
    // with 0.88·√2 = 1.2445 (clean).
    let mut e = vec![];
    // The Activ and its ring alone; the nBuLay is drawn below.
    let bare = |act: R| {
        let mut e = vec![rb(p.activ, act)];
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        e
    };
    let act = act_at(2.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    e.push(rect(p.nbl, n.0, n.1, n.0 + 3.0, n.3));
    e.push(rect(p.nbl, n.0 + 2.0, n.1, n.2, n.3));
    let act = act_at(9.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    e.push(rect(p.nbl, n.0, n.1, n.0 + 3.0, n.3));
    e.push(rect(p.nbl, n.0 + 2.0, n.1, act.2 + 1.235, n.3));
    let act = act_at(16.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    let (tw, th) = ((n.2 - n.0) / 4.0, (n.3 - n.1) / 3.0);
    for i in 0..4 {
        for j in 0..3 {
            let (x, y) = (n.0 + i as f64 * tw, n.1 + j as f64 * th);
            e.push(rect(p.nbl, x, y, x + tw, y + th));
        }
    }
    let act = act_at(2.0, 12.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.0 + 0.7, act.3));
    e.push(rect(p.activ, act.0 + 0.7, act.1, act.0 + 1.4, act.3));
    e.push(rect(p.activ, act.0 + 1.4, act.1, act.2, act.3));
    for (x, a) in [(9.0, 0.87), (16.0, 0.88)] {
        let act = act_at(x, 12.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        e.push(rect(p.nbl, act.0 - 1.3, act.1 - a, act.2 + 1.3, act.3 + a));
        e.push(rect(p.nbl, act.0 - a, act.1 - 1.3, act.2 + a, act.3 + 1.3));
    }
    write_gz(&format!("{NMOSI}/nmosi.b.h6.gds.gz"), library("TOP", e));

    // h7, the conditions inside a closed ring: the Activ under PWell:block, 1.235 short
    // on the right (not in PWell, not isolated; clean); a ptap - Activ AND pSD - 1.235
    // short (fires: Iso-PWell-Activ is Activ of either doping); an Activ under
    // ThickGateOx (nmosiHV) 1.235 short (fires); an Activ that crosses 0.3 into the NWell
    // ring on the left, 1.235 short on the right: the Iso-PWell-Activ is the part outside
    // the NWell, 1.235 short on the right and, its left edge being the ring's inner wall,
    // 1.0 short on the left (fires twice).
    let mut e = iso_enc(p, act_at(2.0, 4.0), 1.4, Some((0, 1.235)));
    e.push(rb(p.pwb, grown(act_at(2.0, 4.0), 0.1)));
    let act = act_at(9.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.push(rb(p.psd, grown(act, 0.1)));
    let act = act_at(16.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.push(rb(p.tgo, grown(act, 0.3)));
    let act = act_at(2.0, 12.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0 - 0.7, act.1, act.2, act.3));
    write_gz(&format!("{NMOSI}/nmosi.b.h7.gds.gz"), library("TOP", e));
}

/// The standard structure with the hole `gap` from the Activ on every side but `side`,
/// which gets `gap_side`; the ring 0.7 wide, the nBuLay 0.3 past it.
fn iso_gap(p: &P, act: R, gap: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let mut hole = grown(act, gap);
    if let Some((s, v)) = side {
        match s {
            0 => hole.2 = act.2 + v,
            1 => hole.3 = act.3 + v,
            2 => hole.0 = act.0 - v,
            _ => hole.1 = act.1 - v,
        }
    }
    let outer = grown(hole, 0.7);
    iso_r(p, act, hole, outer, grown(outer, 0.3))
}

/// nmosi.c - "Min. NWell space to Iso-PWell-Activ  0.39".
fn nmosi_c_h(p: &P) {
    // h1, the bound: the ring 0.39 from the Activ on every side (clean); 0.385 on the
    // right (fires); 0.385 on top (fires); 0.385 all round (fires four times).
    let mut e = iso_gap(p, act_at(2.0, 4.0), 0.39, None);
    e.extend(iso_gap(p, act_at(9.0, 4.0), 0.6, Some((0, 0.385))));
    e.extend(iso_gap(p, act_at(16.0, 4.0), 0.6, Some((1, 0.385))));
    e.extend(iso_gap(p, act_at(2.0, 12.0), 0.385, None));
    write_gz(&format!("{NMOSI}/nmosi.c.h1.gds.gz"), library("TOP", e));

    // h2, corners and 45° walls, the hole 0.6 from the Activ.  At x = 2 an NWell tab in
    // the hole's top-right corner whose corner lies (0.27, 0.27) off the Activ's corner:
    // 0.3818 corner to corner, no wall faces the Activ (fires under the euclidian
    // reading); at x = 9 the tab (0.28, 0.28) off, 0.396 (clean).  At x = 16 the hole's
    // top-right corner filled by an NWell triangle whose hypotenuse passes 0.275·√2 =
    // 0.3889 from the Activ's corner (fires); at x = 24 the same at 0.28·√2 = 0.396
    // (clean).  At x = 2, y = 12 a 1.0 square Activ in a diamond hole whose walls pass
    // 0.3889 from its four corners (fires four times).
    let mut e = vec![];
    for (x, d) in [(2.0, 0.27), (9.0, 0.28)] {
        let act = act_at(x, 4.0);
        e.extend(iso_gap(p, act, 0.6, None));
        let hole = grown(act, 0.6);
        e.push(rect(p.nw, act.2 + d, act.3 + d, hole.2, hole.3));
    }
    for (x, a) in [(16.0, 0.275), (24.0, 0.28)] {
        let act = act_at(x, 4.0);
        e.extend(iso_gap(p, act, 0.6, None));
        let hole = grown(act, 0.6);
        // Hypotenuse x + y = hx1 + hy1 − s at (2·0.6 − s)/√2 = a·√2 from the corner.
        let s = 1.2 - 2.0 * a;
        e.push(poly(
            p.nw,
            &[(hole.2, hole.3), (hole.2 - s, hole.3), (hole.2, hole.3 - s)],
        ));
    }
    let (cx, cy) = (3.0, 13.0);
    let act = (cx - 0.5, cy - 0.5, cx + 0.5, cy + 0.5);
    e.push(rb(p.activ, act));
    // Corner (0.5, 0.5) to the wall x + y = D: (D − 1.0)/√2 = 0.275·√2 → D = 1.55.
    let d = 1.55;
    let (ox0, oy0, ox1, oy1) = (cx - 3.0, cy - 3.0, cx + 3.0, cy + 3.0);
    e.push(poly(
        p.nw,
        &[
            (ox0, oy0),
            (ox1, oy0),
            (ox1, oy1),
            (ox0, oy1),
            (ox0, cy),
            (cx - d, cy),
            (cx, cy + d),
            (cx + d, cy),
            (cx, cy - d),
            (cx - d, cy),
            (ox0, cy),
        ],
    ));
    e.push(rect(p.nbl, ox0 - 0.3, oy0 - 0.3, ox1 + 0.3, oy1 + 0.3));
    write_gz(&format!("{NMOSI}/nmosi.c.h2.gds.gz"), library("TOP", e));

    // h3, the drawing of the ring, the hole 0.385 from the Activ on the right: the ring
    // as one keyhole boundary; as eight overlapping boxes; the ring 0.6 away with a 0.5
    // wide NWell tab on its right wall reaching 0.385 from the Activ; two Activs in one
    // hole, one 0.385 from the right wall, one from the left (fires twice); the Activ as
    // two abutting boxes (fires once); the right wall stepped, 0.385 over the upper half
    // of the Activ and 0.6 over the lower.
    let mut e = vec![];
    let act = act_at(2.0, 4.0);
    let hole = (act.0 - 0.6, act.1 - 0.6, act.2 + 0.385, act.3 + 0.6);
    e.push(rb(p.activ, act));
    e.push(ring_keyhole(p.nw, grown(hole, 0.7), hole));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    let act = act_at(9.0, 4.0);
    let hole = (act.0 - 0.6, act.1 - 0.6, act.2 + 0.385, act.3 + 0.6);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    e.extend(ring4(p.nw, grown(hole, 0.7), hole));
    e.extend(ring4(p.nw, grown(hole, 0.5), grown(hole, -0.0)));
    let act = act_at(16.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.push(rect(
        p.nw,
        act.2 + 0.385,
        act.1 + 0.25,
        act.2 + 0.6,
        act.3 - 0.25,
    ));
    let (a1, a2) = ((2.0, 12.0, 3.0, 13.0), (3.5, 12.0, 4.5, 13.0));
    let hole = (a1.0 - 0.385, a1.1 - 0.6, a2.2 + 0.385, a1.3 + 0.6);
    e.push(rb(p.activ, a1));
    e.push(rb(p.activ, a2));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    e.extend(ring4(p.nw, grown(hole, 0.7), hole));
    let act = act_at(9.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.0 + 1.0, act.3));
    e.push(rect(p.activ, act.0 + 1.0, act.1, act.2, act.3));
    let act = act_at(16.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.push(rect(
        p.nw,
        act.2 + 0.385,
        act.1 + 0.5,
        act.2 + 0.6,
        act.3 + 0.6,
    ));
    write_gz(&format!("{NMOSI}/nmosi.c.h3.gds.gz"), library("TOP", e));

    // h4, the tile lines, 0.385 on the right: the hole's wall on x = 20; the gap across
    // x = 40; across x = 42 and x = 21; at (1000, 1000); a 300 µm Activ 0.385 under the
    // top wall.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 0.385 - 2.0, 4.0),
        (38.0, 4.0),
        (39.9, 12.0),
        (18.9, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_gap(p, act_at(x, y), 0.6, Some((0, 0.385))));
    }
    e.extend(iso_gap(p, (70.0, 4.0, 370.0, 5.0), 0.6, Some((1, 0.385))));
    write_gz(&format!("{NMOSI}/nmosi.c.h4.gds.gz"), library("TOP", e));

    // h7, the conditions: an NWell island (no hole of its own) inside the ring's hole,
    // 0.35 from the Activ (the manual's NWell space; fires); the Activ crossing into the
    // ring on the right (space 0 between the Iso-PWell-Activ and the NWell; fires); a
    // plain NWell 0.35 from an Activ on nBuLay with no ring (not inside a closed ring;
    // clean); a ring 0.385 from an Activ with no nBuLay (not isolated; clean); the Activ
    // under PWell:block 0.385 from the ring (clean); a ptap 0.385 from the ring (fires).
    let mut e = vec![];
    let act = act_at(2.0, 4.0);
    e.extend(iso_gap(p, act, 2.0, None));
    e.push(rect(
        p.nw,
        act.2 + 0.35,
        act.1 - 0.5,
        act.2 + 1.35,
        act.3 + 0.5,
    ));
    let act = act_at(9.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.2 + 0.9, act.3));
    let act = act_at(16.0, 4.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 2.0)));
    e.push(rect(p.nw, act.2 + 0.35, act.1, act.2 + 1.35, act.3));
    let act = act_at(2.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.remove(e.len() - 5);
    let act = act_at(9.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.push(rb(p.pwb, grown(act, 0.1)));
    let act = act_at(16.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.push(rb(p.psd, grown(act, 0.1)));
    write_gz(&format!("{NMOSI}/nmosi.c.h7.gds.gz"), library("TOP", e));
}

/// The structure with the hole 0.4 from the Activ, the ring `ring` wide on every side
/// but `side`, which gets `ring_side`; the nBuLay 0.3 past the ring.
fn iso_ring(p: &P, act: R, ring: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let hole = grown(act, 0.4);
    let mut outer = grown(hole, ring);
    if let Some((s, v)) = side {
        match s {
            0 => outer.2 = hole.2 + v,
            1 => outer.3 = hole.3 + v,
            2 => outer.0 = hole.0 - v,
            _ => outer.1 = hole.1 - v,
        }
    }
    iso_r(p, act, hole, outer, grown(outer, 0.3))
}

/// nmosi.d - "Min. NWell-nBuLay width forming an unbroken ring around any
/// Iso-PWell-Activ  0.62" (NWell-nBuLay = NWell AND nBuLay).
fn nmosi_d_h(p: &P) {
    // h1, the bound: the ring 0.62 wide (clean); the right leg 0.615 (fires); the top leg
    // 0.615 (fires); the NWell ring 1.0 wide, 0.7 from the Activ, but the nBuLay ending
    // 0.615 into its right leg, so NWell AND nBuLay is 0.615 there (fires); every leg
    // 0.615 (fires).
    let mut e = iso_ring(p, act_at(2.0, 4.0), 0.62, None);
    e.extend(iso_ring(p, act_at(9.0, 4.0), 0.8, Some((0, 0.615))));
    e.extend(iso_ring(p, act_at(16.0, 4.0), 0.8, Some((1, 0.615))));
    let act = act_at(2.0, 12.0);
    let hole = grown(act, 0.7);
    let outer = grown(hole, 1.0);
    e.extend(iso_r(
        p,
        act,
        hole,
        outer,
        (outer.0 - 0.3, outer.1 - 0.3, hole.2 + 0.615, outer.3 + 0.3),
    ));
    e.extend(iso_ring(p, act_at(9.0, 12.0), 0.615, None));
    write_gz(&format!("{NMOSI}/nmosi.d.h1.gds.gz"), library("TOP", e));

    // h2, 45° and the drawing.  An octagonal ring 0.7 from the Activ: the legs 0.8 wide,
    // the top-right corner a piece whose outer wall is chamfered and whose inner corner
    // is filled diagonally (0.707 from the Activ's corner), the two 45° walls 0.435·√2 =
    // 0.6152 apart at x = 2 (fires) and 0.44·√2 = 0.6223 at x = 9 (clean).  The right leg as two overlapping boxes 0.62 wide in
    // union at x = 16 (clean) and 0.615 at x = 24 (fires).  At y = 12: a bite 0.185 deep
    // into the outer wall of a 0.8 leg (0.615 left; fires); a 0.2 square hole in a 1.5
    // leg, 0.615 from the inner wall and 0.685 from the outer (fires).
    let mut e = vec![];
    for (x, a) in [(2.0, 0.435), (9.0, 0.44)] {
        let act = act_at(x, 4.0);
        let hole = grown(act, 0.7);
        let outer = grown(hole, 0.8);
        let (hx0, hy0, hx1, hy1) = hole;
        let (ox0, oy0, ox1, oy1) = outer;
        let ci = 0.4;
        let co = 1.6 + ci - 2.0 * a;
        e.push(rb(p.activ, act));
        e.push(rb(p.nbl, grown(outer, 0.3)));
        e.push(rect(p.nw, ox0, oy0, ox1, hy0));
        e.push(rect(p.nw, ox0, hy0, hx0, oy1));
        e.push(rect(p.nw, hx0, hy1, hx1 - ci, oy1));
        e.push(rect(p.nw, hx1, hy0, ox1, hy1 - ci));
        e.push(poly(
            p.nw,
            &[
                (hx1 - ci, hy1),
                (hx1, hy1 - ci),
                (ox1, hy1 - ci),
                (ox1, oy1 - co),
                (ox1 - co, oy1),
                (hx1 - ci, oy1),
            ],
        ));
    }
    for (x, w) in [(16.0, 0.62), (24.0, 0.615)] {
        let act = act_at(x, 4.0);
        let hole = grown(act, 0.4);
        let outer = grown(hole, 0.8);
        e.push(rb(p.activ, act));
        e.push(rb(p.nbl, grown(outer, 0.3)));
        e.push(rect(p.nw, outer.0, outer.1, outer.2, hole.1));
        e.push(rect(p.nw, outer.0, hole.3, outer.2, outer.3));
        e.push(rect(p.nw, outer.0, hole.1, hole.0, hole.3));
        e.push(rect(p.nw, hole.2, hole.1, hole.2 + w - 0.2, hole.3));
        e.push(rect(p.nw, hole.2 + 0.2, hole.1, hole.2 + w, hole.3));
    }
    let act = act_at(2.0, 12.0);
    e.extend(iso_ring(p, act, 0.8, None));
    let hole = grown(act, 0.4);
    let outer = grown(hole, 0.8);
    e.pop();
    e.push(rect(p.nw, hole.2, hole.1, hole.2 + 0.615, hole.3));
    e.push(rect(p.nw, hole.2, hole.1, outer.2, act.1));
    e.push(rect(p.nw, hole.2, act.3, outer.2, hole.3));
    let act = act_at(9.0, 12.0);
    let hole = grown(act, 0.4);
    let outer = grown(hole, 1.5);
    e.extend(iso_r(p, act, hole, outer, grown(outer, 0.3)));
    e.pop();
    let (sx0, sy0) = (hole.2 + 0.615, act.1 + 0.4);
    e.push(rect(p.nw, hole.2, hole.1, sx0, hole.3));
    e.push(rect(p.nw, sx0 + 0.2, hole.1, outer.2, hole.3));
    e.push(rect(p.nw, sx0, hole.1, sx0 + 0.2, sy0));
    e.push(rect(p.nw, sx0, sy0 + 0.2, sx0 + 0.2, hole.3));
    write_gz(&format!("{NMOSI}/nmosi.d.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines, the right leg 0.615: its outer wall on x = 20; the leg across
    // x = 40; across x = 42; the inner wall on x = 21; at (1000, 1000); a 300 µm Activ
    // with a 0.615 top leg.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 0.615 - 0.4 - 2.0, 4.0),
        (37.3, 4.0),
        (39.3, 12.0),
        (21.0 - 0.4 - 2.0, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_ring(p, act_at(x, y), 0.8, Some((0, 0.615))));
    }
    e.extend(iso_ring(p, (70.0, 4.0, 370.0, 5.0), 0.8, Some((1, 0.615))));
    write_gz(&format!("{NMOSI}/nmosi.d.h3.gds.gz"), library("TOP", e));

    // h6, the conditions: an NWell finger 0.5 wide crossing onto an nBuLay with no Activ
    // anywhere (no ring, nothing to ring; clean); a C - the ring without its right leg -
    // 0.615 wide round an Activ (not unbroken; clean); a closed 0.615 ring round nothing
    // (round no Iso-PWell-Activ; clean); a proper 0.8 ring round an Activ and, on the
    // same nBuLay, a second NWell finger 0.5 wide crossing the nBuLay's edge (the finger
    // is not the ring; clean).
    let mut e = vec![
        rect(p.nbl, 2.0, 4.0, 6.0, 8.0),
        rect(p.nw, 3.0, 3.0, 3.5, 6.0),
    ];
    let act = act_at(9.0, 4.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.pop();
    let act = act_at(16.0, 4.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.remove(e.len() - 6);
    let act = act_at(2.0, 12.0);
    let hole = grown(act, 0.4);
    let outer = grown(hole, 0.8);
    e.extend(iso_r(
        p,
        act,
        hole,
        outer,
        (outer.0 - 0.3, outer.1 - 0.3, outer.2 + 3.0, outer.3 + 0.3),
    ));
    e.push(rect(
        p.nw,
        outer.2 + 1.0,
        outer.1 - 1.0,
        outer.2 + 1.5,
        act.3,
    ));
    write_gz(&format!("{NMOSI}/nmosi.d.h6.gds.gz"), library("TOP", e));
}

/// The 3.0 × 2.0 Activ at `(x, y)` in the standard ring, for the nSD:block and SalBlock
/// rules.
fn iso_big(p: &P, x: f64, y: f64) -> (R, Vec<GdsElement>) {
    let act = (x, y, x + 3.0, y + 2.0);
    (act, iso(p, act, 0.4, 0.7, 0.3))
}

/// nmosi.f - "Min. nSD:block width to separate ptap in nmosi  0.62".
fn nmosi_f_h(p: &P) {
    // h1, the bound: a vertical nSD:block strip through the Activ 0.62 wide (clean);
    // 0.615 (fires); a horizontal 0.615 strip (fires); a 0.615 stub ending inside the
    // Activ (fires); an L of 0.615 arms (fires on both arms).
    let mut e = vec![];
    for (x, y, w) in [(2.0, 4.0, 0.62), (9.0, 4.0, 0.615)] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.0 + w,
            act.3 + 0.2,
        ));
    }
    let (act, s) = iso_big(p, 16.0, 4.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 - 0.2,
        act.1 + 0.8,
        act.2 + 0.2,
        act.1 + 1.415,
    ));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.1 + 1.2,
    ));
    let (act, s) = iso_big(p, 9.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.1 + 1.4,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 + 0.785,
        act.2 + 0.2,
        act.1 + 1.4,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h1.gds.gz"), library("TOP", e));

    // h2, the drawing and 45°: the strip as two overlapping boxes 0.62 in union (clean)
    // and 0.615 (fires); a 45° strip 0.435·√2 = 0.6152 wide (fires) and 0.44·√2 = 0.6223
    // (clean); a 0.8 strip with a 0.185 bite (0.615 left; fires).
    let mut e = vec![];
    for (x, w) in [(2.0, 0.62), (9.0, 0.615)] {
        let (act, s) = iso_big(p, x, 4.0);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.0 + w - 0.2,
            act.3 + 0.2,
        ));
        e.push(rect(
            p.nsdb,
            act.0 + 1.2,
            act.1 - 0.2,
            act.0 + 1.0 + w,
            act.3 + 0.2,
        ));
    }
    for (x, d) in [(16.0, 0.435), (24.0, 0.44)] {
        let (act, s) = iso_big(p, x, 4.0);
        e.extend(s);
        let (x0, y0) = (act.0 + 0.3, act.1 - 0.2);
        e.push(poly(
            p.nsdb,
            &[
                (x0, y0),
                (x0 + 2.4, y0 + 2.4),
                (x0 + 2.4 - d, y0 + 2.4 + d),
                (x0 - d, y0 + d),
            ],
        ));
    }
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.8,
        act.1 + 0.6,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 + 1.4,
        act.0 + 1.8,
        act.3 + 0.2,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines, a 0.615 vertical strip: its right wall on x = 20; across
    // x = 40; across x = 42; its left wall on x = 21; at (1000, 1000); a 300 µm Activ with
    // a 0.615 horizontal strip.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.615, 4.0),
        (38.8, 4.0),
        (40.8, 12.0),
        (20.0, 12.0),
        (1000.0, 1000.0),
    ] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.615,
            act.3 + 0.2,
        ));
    }
    let act = (70.0, 4.0, 370.0, 6.0);
    e.extend(iso(p, act, 0.4, 0.7, 0.3));
    e.push(rect(
        p.nsdb,
        act.0 - 0.2,
        act.1 + 0.8,
        act.2 + 0.2,
        act.1 + 1.415,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h3.gds.gz"), library("TOP", e));

    // h6, the conditions: a 0.615 strip over an Activ in a ring but with no nBuLay (not
    // isolated; clean); a 0.615 strip over the Activ under PWell:block (clean); a 0.615
    // strip abutting the Activ's right edge, not over it (clean); one over it by 0.005
    // (fires); a 0.8 strip through the Activ with a 0.5 wide tab outside the Activ on the
    // nBuLay (the block is 0.8 wide where it separates; clean by the rule's words).
    let mut e = vec![];
    let (act, s) = iso_big(p, 2.0, 4.0);
    e.extend(s);
    e.remove(e.len() - 5);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 9.0, 4.0);
    e.extend(s);
    e.push(rb(p.pwb, grown(act, 0.1)));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 16.0, 4.0);
    e.extend(s);
    e.push(rect(p.nsdb, act.2, act.1 - 0.2, act.2 + 0.615, act.3 + 0.2));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.2 - 0.005,
        act.1 - 0.2,
        act.2 + 0.61,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 9.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.8,
        act.3 + 0.2,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.3 + 0.2,
        act.0 + 1.5,
        act.3 + 1.0,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h6.gds.gz"), library("TOP", e));
}

/// The nmosi.g instance: the 3.0 × 2.0 Activ in the standard ring, an nSD:block over
/// its left 1.5 (flush with the Activ on the other three sides), SalBlock over the
/// nSD:block and `ext` past its right edge.
fn gi(p: &P, x: f64, y: f64, ext: f64) -> (R, R, Vec<GdsElement>) {
    let (act, mut e) = iso_big(p, x, y);
    let blk = (act.0, act.1, act.0 + 1.5, act.3);
    e.push(rb(p.nsdb, blk));
    e.push(rect(p.sal, blk.0, blk.1, blk.2 + ext, blk.3));
    (act, blk, e)
}

/// nmosi.g - "Min. SalBlock overlap of nSD:block over Activ  0.15".
fn nmosi_g_h(p: &P) {
    // h1, the bound: SalBlock 0.15 past the nSD:block (clean); 0.145 (fires); no
    // SalBlock at all (fires); a 1.0 square nSD:block in the middle of the Activ with
    // SalBlock 0.15 round it (clean) and 0.145 on top only (fires); the SalBlock's
    // corner chamfered so it passes 0.1·√2 = 0.1414 from the block's corner (fires,
    // closest approach) and 0.11·√2 = 0.1556 (clean); SalBlock as two overlapping boxes
    // 0.15 past the block in union (clean) and 0.145 (fires).
    let mut e = vec![];
    e.extend(gi(p, 2.0, 4.0, 0.15).2);
    e.extend(gi(p, 9.0, 4.0, 0.145).2);
    let (_, _, mut s) = gi(p, 16.0, 4.0, 0.0);
    s.pop();
    e.extend(s);
    for (x, y, top, k) in [
        (2.0, 12.0, 0.15, None),
        (9.0, 12.0, 0.145, None),
        (16.0, 12.0, 0.15, Some(0.1)),
        (24.0, 12.0, 0.15, Some(0.11)),
    ] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        let b = (act.0 + 1.0, act.1 + 0.5, act.0 + 2.0, act.1 + 1.5);
        e.push(rb(p.nsdb, b));
        match k {
            None => e.push(rect(p.sal, b.0 - 0.15, b.1 - 0.15, b.2 + 0.15, b.3 + top)),
            Some(a) => e.push(chamfered_tr(
                p.sal,
                b.0 - 0.15,
                b.1 - 0.15,
                b.2 + 0.15,
                b.3 + 0.15,
                b.2 + b.3 + 2.0 * a,
            )),
        }
    }
    for (x, ext) in [(2.0, 0.15), (9.0, 0.145)] {
        let (_, blk, mut s) = gi(p, x, 20.0, 0.0);
        s.pop();
        e.extend(s);
        e.push(rect(p.sal, blk.0, blk.1, blk.2 + 0.05, blk.3));
        e.push(rect(p.sal, blk.0 + 1.0, blk.1, blk.2 + ext, blk.3));
    }
    write_gz(&format!("{NMOSI}/nmosi.g.h1.gds.gz"), library("TOP", e));

    // h2, the tile lines, 0.145: the SalBlock's edge on x = 20; the band across x = 40;
    // across x = 42; the block's edge on x = 21; at (1000, 1000); a 300 µm Activ with the
    // block 300 long and the SalBlock 0.145 past it on top.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.645, 4.0),
        (38.4, 4.0),
        (40.4, 12.0),
        (19.5, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(gi(p, x, y, 0.145).2);
    }
    let act = (70.0, 4.0, 370.0, 6.0);
    e.extend(iso(p, act, 0.4, 0.7, 0.3));
    e.push(rect(p.nsdb, act.0, act.1, act.2, act.1 + 1.0));
    e.push(rect(p.sal, act.0, act.1, act.2, act.1 + 1.145));
    write_gz(&format!("{NMOSI}/nmosi.g.h2.gds.gz"), library("TOP", e));

    // h5, the conditions: the nSD:block and SalBlock flush with the Activ on every side
    // (nothing over Activ to cover; clean); the 0.145 instance with no nBuLay (clean); the
    // 0.145 instance with the Activ under PWell:block (clean); the SalBlock 0.145 past the
    // block but only outside the Activ - the block ends flush with the Activ's right edge
    // and the Activ continues below it (fires on the band under the block).
    let mut e = vec![];
    let (act, s) = iso_big(p, 2.0, 4.0);
    e.extend(s);
    e.push(rb(p.nsdb, act));
    e.push(rb(p.sal, act));
    let (_, _, s) = gi(p, 9.0, 4.0, 0.145);
    let mut s = s;
    s.remove(1);
    e.extend(s);
    let (act, _, s) = gi(p, 16.0, 4.0, 0.145);
    e.extend(s);
    e.push(rb(p.pwb, grown(act, 0.1)));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(p.nsdb, act.0, act.1 + 1.0, act.2, act.3));
    e.push(rect(p.sal, act.0, act.1 + 0.855, act.2 + 0.145, act.3));
    write_gz(&format!("{NMOSI}/nmosi.g.h5.gds.gz"), library("TOP", e));
}

/// The section's condition, "These rules will only be tested inside a closed ring of
/// NWell AND nBuLay": every rule's violating pattern with no NWell ring at all (the
/// Activ on a bare nBuLay), and with a C - the ring missing its right leg.
fn nmosi_ring(p: &P) {
    let mut e = vec![];
    // nmosi.b: nBuLay 1.0 past the Activ, no ring; a C.
    let act = act_at(2.0, 4.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.0)));
    let act = act_at(9.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.pop();
    // nmosi.c: a C 0.385 from the Activ on the left.
    let act = act_at(16.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, Some((2, 0.385))));
    e.pop();
    // nmosi.d: a C 0.615 wide.
    let act = act_at(2.0, 12.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.pop();
    // nmosi.f: a 0.615 strip through an Activ on bare nBuLay; in a C.
    let act = (9.0, 12.0, 12.0, 14.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.4)));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 16.0, 12.0);
    e.extend(s);
    e.pop();
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    // nmosi.g: SalBlock 0.145 past the block, on bare nBuLay; in a C.
    let act = (2.0, 20.0, 5.0, 22.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.4)));
    e.push(rect(p.nsdb, act.0, act.1, act.0 + 1.5, act.3));
    e.push(rect(p.sal, act.0, act.1, act.0 + 1.645, act.3));
    let (_, _, mut s) = gi(p, 9.0, 20.0, 0.145);
    s.remove(s.len() - 3);
    e.extend(s);
    write_gz(&format!("{NMOSI}/nmosi.ring.h1.gds.gz"), library("TOP", e));
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/nmosi")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    nmosi_b_h(&p);
    nmosi_c_h(&p);
    nmosi_d_h(&p);
    nmosi_f_h(&p);
    nmosi_g_h(&p);
    nmosi_ring(&p);
}
