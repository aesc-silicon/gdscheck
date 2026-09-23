// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::nmosi::P;
use crate::helpers::{layer, library, rect, text, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/antenna";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    ant_i(pdk);
    ant_b(pdk);
    ant_merge(pdk);
    ant_ac(pdk);
    ant_g(pdk);
    ant_h(pdk);

    hardening(pdk);
}

/// Ant.h — an n-diode (dantenna) in an NWell is not allowed.  Diode A is a bare dantenna in
/// an NWell → fires.  Diode B is the same but part of an `isolbox` (nBuLay over the diode,
/// tagged with an "isolbox" text), which is exempt → clean.
fn ant_h(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let diode = layer(pdk, "Recog.diode");
    let nwell = layer(pdk, "NWell");
    let nbulay = layer(pdk, "nBuLay");
    let txt = layer(pdk, "TEXT");
    let o = OFFSET;

    let mut elems = vec![
        // A: bare dantenna in NWell → Ant.h.
        rect(nwell, o - 1.0, o - 1.0, o + 3.0, o + 3.0),
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(diode, o, o, o + 1.0, o + 1.0),
    ];
    // B: dantenna inside an isolbox (nBuLay over diode + "isolbox" label) → exempt.
    let x = o + 10.0;
    elems.extend([
        rect(nwell, x - 1.0, o - 1.0, x + 3.0, o + 3.0),
        rect(activ, x, o, x + 1.0, o + 1.0),
        rect(diode, x, o, x + 1.0, o + 1.0),
        rect(nbulay, x - 0.5, o - 0.5, x + 1.5, o + 1.5),
        text(txt, "isolbox", x + 0.5, o + 0.5),
    ]);
    write_gz(&format!("{DIR}/Ant.h.gds.gz"), library("TOP", elems));
}

/// Ant.g — an undersized (0.12 µm² < 0.16) n-diode (dantenna) tied to a gate through Metal1.
/// A diode that small does not protect the gate, so it must be flagged.
fn ant_g(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gatpoly = layer(pdk, "GatPoly");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let diode = layer(pdk, "Recog.diode");
    let o = OFFSET;
    let elems = vec![
        // Gate stack with a Metal1 strip reaching the diode.
        rect(activ, o, o, o + 1.0, o + 0.5),
        rect(gatpoly, o + 0.3, o - 0.5, o + 0.5, o + 1.0),
        rect(cont, o + 0.35, o + 0.7, o + 0.45, o + 0.8),
        rect(metal1, o, o + 0.6, o + 9.0, o + 1.6),
        // Undersized n-diode (Activ ∩ Recog.diode, no pSD/NWell): 0.3 × 0.4 = 0.12 µm².
        rect(activ, o + 8.0, o + 0.6, o + 8.3, o + 1.0),
        rect(diode, o + 8.0, o + 0.6, o + 8.3, o + 1.0),
        rect(cont, o + 8.05, o + 0.7, o + 8.25, o + 0.9),
    ];
    write_gz(&format!("{DIR}/Ant.g.gds.gz"), library("TOP", elems));
}

/// Ant.a / Ant.c (pre-metal antennas).  Gate Pa has a small gate but a large GatPoly pad
/// routed over field oxide → poly/gate ratio trips Ant.a.  Gate Ca has a large contact area
/// on its gate poly → cont/gate ratio trips Ant.c.
fn ant_ac(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gatpoly = layer(pdk, "GatPoly");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;

    // Pa: gate (poly 0.2×0.5 over Activ = 0.1 µm²) + an 8×8 poly pad over field (64 µm²).
    let mut elems = vec![
        rect(activ, o, o, o + 1.0, o + 0.5),
        rect(gatpoly, o + 0.3, o - 0.5, o + 0.5, o + 2.0),
        rect(gatpoly, o + 0.3, o + 1.5, o + 8.3, o + 9.5),
    ];
    // Ca: gate (0.1 µm²) + a poly pad carrying a 1.45×1.45 = 2.1 µm² contact → ratio 21.
    let g = o + 12.0;
    elems.extend([
        rect(activ, g, o, g + 1.0, o + 0.5),
        rect(gatpoly, g + 0.3, o - 0.5, g + 0.5, o + 2.0),
        rect(gatpoly, g + 0.2, o + 1.0, g + 2.0, o + 2.8),
        rect(cont, g + 0.3, o + 1.1, g + 1.75, o + 2.55),
    ]);
    write_gz(&format!("{DIR}/Ant.ac.gds.gz"), library("TOP", elems));
}

/// Per-level vs full-net stress: gate G1 has a 25 µm² Metal1 antenna (ratio 250 at the
/// Metal1 level → trips Ant.b there).  At Metal2 it merges with gate G2 (no antenna) through
/// a shared Metal2 plate, which would dilute a *final-net* ratio below 200 — but the per-level
/// cumulative keeps G1's Metal1 term, so G1 still trips.  Only G1 should be flagged.
fn ant_merge(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gatpoly = layer(pdk, "GatPoly");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let via1 = layer(pdk, "Via1");
    let metal2 = layer(pdk, "Metal2");
    let o = OFFSET;

    let gate_stack = |gx: f64, m1: (f64, f64, f64, f64), vx: f64| {
        vec![
            rect(activ, gx, o, gx + 1.0, o + 0.5),
            rect(gatpoly, gx + 0.3, o - 0.5, gx + 0.5, o + 1.0),
            rect(cont, gx + 0.35, o + 0.7, gx + 0.45, o + 0.8),
            rect(metal1, m1.0, m1.1, m1.2, m1.3),
            rect(via1, vx, o + 0.9, vx + 0.4, o + 1.3),
        ]
    };

    // G1: 5×5 = 25 µm² Metal1 antenna, Via1 at its right edge.
    let mut elems = gate_stack(o, (o, o + 0.6, o + 5.0, o + 5.6), o + 4.4);
    // G2: thin 0.5 µm² Metal1, Via1 near it.
    elems.extend(gate_stack(
        o + 9.0,
        (o + 9.0, o + 0.6, o + 9.5, o + 1.6),
        o + 9.05,
    ));
    // Metal2 plate over both Via1s → merges the two gates at the Metal2 level.
    elems.push(rect(metal2, o + 4.2, o + 0.8, o + 9.6, o + 1.4));
    write_gz(&format!("{DIR}/Ant.merge.gds.gz"), library("TOP", elems));
}

/// Ant.b/e — a tiny poly gate tied through Cont→Metal1 to a large Metal1 antenna.  Gate A
/// has no discharge path, so its metal/gate ratio (~1000) trips Ant.b.  Gate B is identical
/// but its Metal1 also reaches a diffusion diode (NActivCon ≥ 0.16 µm²), so the relaxed
/// Ant.e limit (20000) applies and it stays clean.
fn ant_b(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gatpoly = layer(pdk, "GatPoly");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let o = OFFSET;

    // A poly gate at x-origin `gx`: Activ 1×0.5, GatPoly crossing it (gate = 0.2×0.5 µm²),
    // a poly contact above the Activ, and a `w`×`h` Metal1 antenna over the contact.
    let gate = |gx: f64, w: f64, h: f64| {
        vec![
            rect(activ, gx, o, gx + 1.0, o + 0.5),
            rect(gatpoly, gx + 0.3, o - 0.5, gx + 0.5, o + 1.0),
            rect(cont, gx + 0.35, o + 0.7, gx + 0.45, o + 0.8),
            rect(metal1, gx, o + 0.6, gx + w, o + 0.6 + h),
        ]
    };

    let mut elems = gate(o, 10.0, 10.0); // gate A: 100 µm² antenna, no diode → Ant.b
    elems.extend(gate(o + 12.0, 12.0, 12.0)); // gate B: 144 µm² antenna, plus a diode below
    // Diffusion diode (NActivCon: Activ, no GatPoly/pSD) tied to gate B's Metal1 via a Cont.
    elems.extend([
        rect(activ, o + 22.0, o + 0.6, o + 24.0, o + 2.6),
        rect(cont, o + 22.5, o + 1.0, o + 23.5, o + 2.0),
    ]);
    write_gz(&format!("{DIR}/Ant.b.gds.gz"), library("TOP", elems));
}

/// Ant.i — a p-diode (dpantenna) is only allowed inside an NWell.  Two diodes are drawn:
/// a P+ diffusion (Activ ∩ pSD, no GatPoly) under a `Recog.diode` marker.  The left one
/// sits in the PWell (no NWell) → Ant.i fires; the right one is covered by NWell → clean.
fn ant_i(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let diode = layer(pdk, "Recog.diode");
    let nwell = layer(pdk, "NWell");
    let o = OFFSET;

    // Violating dpantenna in the PWell (no NWell underneath).
    let mut elems = vec![
        rect(activ, o, o, o + 4.0, o + 4.0),
        rect(psd, o, o, o + 4.0, o + 4.0),
        rect(diode, o, o, o + 4.0, o + 4.0),
    ];
    // Correctly-placed dpantenna in an NWell → must stay clean.
    let x = o + 20.0;
    elems.extend([
        rect(nwell, x - 1.0, o - 1.0, x + 5.0, o + 5.0),
        rect(activ, x, o, x + 4.0, o + 4.0),
        rect(psd, x, o, x + 4.0, o + 4.0),
        rect(diode, x, o, x + 4.0, o + 4.0),
    ]);
    write_gz(&format!("{DIR}/Ant.i.gds.gz"), library("TOP", elems));
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

const ANT: &str = "tests/data/ihp-sg13g2/antenna";

/// The gate at `(x, y)`: Activ (x, y)-(x+1, y+0.5), GatPoly (x+0.4, y−0.3)-(x+0.6, y+0.8),
/// so the gate is 0.2 × 0.5 = 0.1 µm² and the poly over field 0.2 × 0.6 = 0.12 µm², and a
/// 0.1 × 0.1 Cont on the poly at (x+0.45, y+0.6)-(x+0.55, y+0.7).  Metal1 over the Cont
/// is the caller's.
fn gate(p: &P, x: f64, y: f64) -> Vec<GdsElement> {
    vec![
        rect(p.activ, x, y, x + 1.0, y + 0.5),
        rect(p.gp, x + 0.4, y - 0.3, x + 0.6, y + 0.8),
        rect(p.cont, x + 0.45, y + 0.6, x + 0.55, y + 0.7),
    ]
}

/// The gate at `(x, y)` with a Metal1 antenna `w` × `h` from (x, y+0.6) over its Cont.
fn gate_m1(p: &P, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    let mut e = gate(p, x, y);
    e.push(rect(p.m1, x, y + 0.6, x + w, y + 0.6 + h));
    e
}

/// An n-diode `w` × `h` at `(x, y)` - Activ with a Recog:diode marker, no pSD, no
/// NWell - with a Cont in its middle; the Metal1 over the Cont is the caller's.
fn ndiode(p: &P, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![
        rect(p.activ, x, y, x + w, y + h),
        rect(p.diode, x, y, x + w, y + h),
        rect(
            p.cont,
            x + w / 2.0 - 0.05,
            y + h / 2.0 - 0.05,
            x + w / 2.0 + 0.05,
            y + h / 2.0 + 0.05,
        ),
    ]
}

fn antenna(p: &P) {
    ant_b_h(p);
    ant_d(p);
    ant_ac_h(p);
    ant_g_h(p);
    ant_hi(p);
}

/// Ant.b/e - "Max. ratio of cumulative metal area (from Metal1 to TopMetal2) to
/// connected Gate area  200.00 (without protection diode) / 20000.00 (with)".
fn ant_b_h(p: &P) {
    // h1, the bound, all inside one tile: Metal1 4.0 × 4.995 = 19.98 µm² on a 0.1 µm²
    // gate, ratio 199.8 (clean); 4.0 × 5.0 = 20.0, ratio 200.0 exactly (a maximum of 200
    // is met; clean); 4.0 × 5.005 = 20.02, ratio 200.2 (fires).
    let mut e = gate_m1(p, 2.0, 2.0, 4.0, 4.995);
    e.extend(gate_m1(p, 8.0, 2.0, 4.0, 5.0));
    e.extend(gate_m1(p, 14.0, 2.0, 4.0, 5.005));
    write_gz(&format!("{ANT}/Ant.b.h1.gds.gz"), library("TOP", e));

    // h3, several gates and the union.  At x = 2 two gates under one 6.0 × 5.0 = 30 µm²
    // Metal1, gate area 0.2, ratio 150 (clean); at x = 10 one gate under 30 µm², ratio
    // 300 (fires).  At y = 10 the per-level sum of figure 7.1: gate G1 at x = 2 with
    // 3.9 × 5.0 = 19.5 µm² of Metal1 (195), gate G2 at x = 7 with 1.0 µm² (10), joined
    // at Metal2 by a 2.0 × 1.0 = 2.0 µm² plate over a Via1 on each (Metal2 level: 2.0 /
    // 0.2 = 10): G1 sums to 205 (fires), G2 to 20 (clean); on the final net the sum would
    // be 22.5 / 0.2 = 112.5 and nothing would fire.  At y = 18: the gate's Activ as two
    // overlapping boxes (gate still 0.1) under 20.02 (fires); Metal1 as two 4.0 × 3.0
    // boxes overlapping by 4.0 × 1.005 - 19.98 in union, 24 as a sum (clean).
    let mut e = gate(p, 2.0, 2.0);
    e.extend(gate(p, 5.0, 2.0));
    e.push(rect(p.m1, 2.0, 2.6, 8.0, 7.6));
    e.extend(gate_m1(p, 10.0, 2.0, 6.0, 5.0));
    e.extend(gate_m1(p, 2.0, 10.0, 3.9, 5.0));
    e.extend(gate_m1(p, 7.0, 10.0, 1.0, 1.0));
    e.push(rect(p.via1, 5.5, 11.1, 5.7, 11.3));
    e.push(rect(p.via1, 7.1, 11.1, 7.3, 11.3));
    e.push(rect(p.m2, 5.4, 10.9, 7.4, 11.9));
    e.extend(gate_m1(p, 2.0, 18.0, 4.0, 5.005));
    e.push(rect(p.activ, 2.5, 18.0, 3.5, 18.5));
    e.extend(gate(p, 10.0, 18.0));
    e.push(rect(p.m1, 10.0, 18.6, 14.0, 21.6));
    e.push(rect(p.m1, 10.0, 20.595, 14.0, 23.595));
    write_gz(&format!("{ANT}/Ant.b.h3.gds.gz"), library("TOP", e));

    // h4, the diode.  A gate under 30 µm² of Metal1 (300) whose Metal1 also reaches an
    // n-diode: 0.5 × 0.5 = 0.25 µm² at x = 2 (protected; clean); 0.4 × 0.4 = 0.16 exactly
    // at x = 10 (a diode of the minimum size protects; clean); 0.4 × 0.395 = 0.158 at
    // x = 18 (too small: Ant.g fires, and the net is unprotected, so Ant.b fires).  At
    // y = 20 a p-diode - Activ AND pSD AND Recog:diode in NWell - 0.25 µm² protects a 300
    // net (clean).  At x = 30, Ant.e: a 0.25 diode and 40.0 × 50.0 = 2000 µm² of Metal1,
    // ratio 20000 exactly (clean); at x = 80, 40.01 × 50.0 = 2000.5, ratio 20005 (fires).
    let mut e = vec![];
    for (x, w, h) in [(2.0, 0.5, 0.5), (10.0, 0.4, 0.4), (18.0, 0.4, 0.395)] {
        e.extend(gate_m1(p, x, 2.0, 6.0, 5.0));
        e.extend(ndiode(p, x + 4.5, 2.65, w, h));
    }
    e.extend(gate_m1(p, 2.0, 20.0, 6.0, 5.0));
    e.push(rect(p.nw, 6.0, 20.2, 7.5, 21.6));
    e.push(rect(p.activ, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.psd, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.diode, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.cont, 6.7, 20.85, 6.8, 20.95));
    for (x, w) in [(30.0, 40.0), (80.0, 40.01)] {
        e.extend(gate_m1(p, x, 2.0, w, 50.0));
        e.extend(ndiode(p, x + 4.5, 2.65, 0.5, 0.5));
    }
    write_gz(&format!("{ANT}/Ant.b.h4.gds.gz"), library("TOP", e));

    // h6, the level the diode joins at: the gate's 6.0 × 5.0 = 30 µm² Metal1 (300) goes
    // up a Via1 to a Metal2 strip that comes down a Via1 onto a 0.25 µm² n-diode's own
    // Metal1 pad.  At the Metal1 level - the Metal1 etch - the net holds no diode and its
    // ratio is 300 (fires, by the per-level reading of figure 7.1); read off the final
    // net the diode protects it (KLayout).
    let mut e = gate_m1(p, 2.0, 2.0, 6.0, 5.0);
    e.push(rect(p.via1, 7.5, 4.0, 7.7, 4.2));
    e.push(rect(p.m2, 7.4, 3.9, 10.6, 4.3));
    e.push(rect(p.via1, 10.3, 4.0, 10.5, 4.2));
    e.push(rect(p.m1, 10.0, 3.5, 11.0, 4.5));
    e.extend(ndiode(p, 10.25, 3.75, 0.5, 0.5));
    write_gz(&format!("{ANT}/Ant.b.h6.gds.gz"), library("TOP", e));

    // h5, several metal layers.  A gate with 2.0 × 2.5 = 5.0 µm² of Metal1 (50), a Via1 to
    // 2.0 × 4.0 = 8.0 µm² of Metal2 (80), a Via2 to Metal3: 8.0 µm² at x = 2, the sum 210
    // (fires at Metal3); 6.0 µm² at x = 8, 190 (clean); 7.0 µm² at x = 14, 200 exactly
    // (clean).  At x = 2, y = 12 the stack to TopMetal2: Metal1 5.0 (50), then 0.5 × 0.5
    // pads on Metal2-Metal5 and TopMetal1 (2.5 each, 12.5) under 4.0 × 4.0 = 16 µm² of
    // TopMetal2 (160): 222.5 (fires at TopMetal2).
    let mut e = vec![];
    for (x, h3) in [(2.0, 4.0), (8.0, 3.0), (14.0, 3.5)] {
        e.extend(gate_m1(p, x, 2.0, 2.0, 2.5));
        e.push(rect(p.via1, x + 0.9, 3.9, x + 1.1, 4.1));
        e.push(rect(p.m2, x, 2.6, x + 2.0, 6.6));
        e.push(rect(p.via2, x + 0.9, 5.9, x + 1.1, 6.1));
        e.push(rect(p.m3, x, 2.6, x + 2.0, 2.6 + h3));
    }
    let (x, y) = (2.0, 12.0);
    e.extend(gate_m1(p, x, y, 2.0, 2.5));
    let (cx, cy) = (x + 1.0, y + 2.0);
    let pad = |l: (i16, i16), h: f64| rect(l, cx - h, cy - h, cx + h, cy + h);
    e.push(pad(p.via1, 0.1));
    e.push(pad(p.m2, 0.25));
    e.push(pad(p.via2, 0.1));
    e.push(pad(p.m3, 0.25));
    e.push(pad(p.via3, 0.1));
    e.push(pad(p.m4, 0.25));
    e.push(pad(p.via4, 0.1));
    e.push(pad(p.m5, 0.25));
    e.push(pad(p.tv1, 0.2));
    e.push(pad(p.tm1, 0.25));
    e.push(pad(p.tv2, 0.2));
    e.push(pad(p.tm2, 2.0));
    write_gz(&format!("{ANT}/Ant.b.h5.gds.gz"), library("TOP", e));
}

/// Ant.d/f - "Max. ratio of cumulative via area (from Via1 to TopVia2) to connected
/// Gate area  20.00 (without protection diode) / 500.00 (with)".
fn ant_d(p: &P) {
    // The gate under a Metal1 pad with one Via1 of `w` × `h` on it and a Metal2 pad over.
    let via = |x: f64, y: f64, w: f64, h: f64| {
        let mut e = gate_m1(p, x, y, w + 0.2, h + 0.2);
        e.push(rect(p.via1, x + 0.1, y + 0.7, x + 0.1 + w, y + 0.7 + h));
        e.push(rect(p.m2, x, y + 0.6, x + w + 0.2, y + 0.8 + h));
        e
    };
    // h1: a Via1 of 2.0 × 1.0 = 2.0 µm² on a 0.1 gate, ratio 20.0 exactly (clean); 2.0 ×
    // 1.005 = 2.01, 20.1 (fires); 2.0 × 0.995 = 1.99, 19.9 (clean).  At y = 8 the sum over
    // levels: Via1 1.0 × 1.0 (10) and Via2 1.0 × 1.01 (10.1) on the Metal2, 20.1 (fires).
    // At y = 14 with a 0.25 diode on the Metal1: Via1 2.01 (Ant.d does not apply, Ant.f's
    // 500 is far; clean) at x = 2; Via1 10.0 × 5.005 = 50.05, 500.5 (Ant.f fires) at
    // x = 8.  At y = 24 a 40.2 × 0.05 = 2.01 Via1 from x = 10 across x = 20 and 40 (fires).
    // At y = 28 two gates on one Metal1 with a 4.0 × 1.0 Via1, 4.0 / 0.2 = 20.0 (clean),
    // and at x = 12 with 4.0 × 1.005 = 4.02, 20.1 (fires).
    let mut e = via(2.0, 2.0, 2.0, 1.0);
    e.extend(via(6.0, 2.0, 2.0, 1.005));
    e.extend(via(10.0, 2.0, 2.0, 0.995));
    e.extend(via(2.0, 8.0, 1.0, 1.0));
    e.push(rect(p.via2, 2.1, 8.7, 3.1, 9.71));
    e.push(rect(p.m3, 2.0, 8.6, 3.2, 9.8));
    e.extend(via(2.0, 14.0, 2.0, 1.005));
    e.extend(ndiode(p, 3.4, 13.9, 0.5, 0.5));
    e.push(rect(p.m1, 3.3, 13.8, 4.0, 14.7));
    e.extend(via(8.0, 14.0, 10.0, 5.005));
    e.extend(ndiode(p, 18.4, 13.9, 0.5, 0.5));
    e.push(rect(p.m1, 17.8, 13.8, 19.0, 14.7));
    e.extend(gate_m1(p, 10.0, 24.0, 40.4, 0.25));
    e.push(rect(p.via1, 10.1, 24.7, 50.3, 24.75));
    e.push(rect(p.m2, 10.0, 24.65, 50.4, 24.8));
    for (x, h) in [(2.0, 1.0), (12.0, 1.005)] {
        e.extend(gate(p, x, 28.0));
        e.extend(gate(p, x + 3.0, 28.0));
        e.push(rect(p.m1, x, 28.6, x + 4.4, 29.8 + h));
        e.push(rect(p.via1, x + 0.2, 28.9, x + 4.2, 28.9 + h));
        e.push(rect(p.m2, x, 28.8, x + 4.4, 29.0 + h));
    }
    write_gz(&format!("{ANT}/Ant.d.h1.gds.gz"), library("TOP", e));
}

/// Ant.a - "Max. ratio of GatPoly over field oxide area to connected Gate area  200.00";
/// Ant.c - "Max. ratio of Cont area to connected Gate area  20.00".
fn ant_ac_h(p: &P) {
    // A poly pad `w` × `h` abutting the top of the gate's strip.
    let pad =
        |x: f64, y: f64, w: f64, h: f64| rect(p.gp, x + 0.4, y + 0.8, x + 0.4 + w, y + 0.8 + h);
    // h1.  Ant.a: the gate's 0.12 µm² of field poly plus a 4.0 × 4.97 = 19.88 pad, 20.0
    // µm² on 0.1, ratio 200.0 exactly at x = 2 (clean); a 4.0 × 4.975 = 19.9 pad, 200.2
    // at x = 16, the pad across x = 20 (fires).  At y = 10 one poly strip crossing two Activs 1.0 apart (gates 0.2,
    // field 0.32) with a 30 µm² pad, 151.6 (clean); at x = 10, y = 10 two gates whose
    // polys are separate, G1 with a 25 µm² pad (251.2) and G2 with 3 (31.2), strapped by
    // Metal1 over their Conts: poly is connected by poly, so G1 fires and G2 is clean; on
    // the whole net 28.24 / 0.2 = 141 and nothing would.  Ant.c, at y = 20, every gate
    // counting its own 0.01 µm² poly Cont: a Cont of 1.0 × 1.99 = 1.99 µm² on a 1.2 × 2.2
    // poly pad, 2.0 in all, ratio 20.0 (clean); 1.0 × 1.995, 2.005, 20.05 (fires); two
    // gates, G1 with a 2.5 µm² Cont (25.1) and G2 with 0.5 (5.1), strapped by Metal1 (G1
    // fires; the whole net would be 15.1); a gate with a 2.0 µm² Cont on its Activ beside
    // the poly and no metal (the Cont is not on the gate's net; clean).
    let mut e = gate(p, 2.0, 2.0);
    e.push(pad(2.0, 2.0, 4.0, 4.97));
    e.extend(gate(p, 16.0, 2.0));
    e.push(pad(16.0, 2.0, 4.0, 4.975));
    e.push(rect(p.activ, 2.0, 10.0, 3.0, 10.5));
    e.push(rect(p.activ, 2.0, 11.5, 3.0, 12.0));
    e.push(rect(p.gp, 2.4, 9.7, 2.6, 12.3));
    e.push(rect(p.gp, 2.4, 12.3, 7.4, 18.3));
    e.extend(gate(p, 10.0, 10.0));
    e.push(pad(10.0, 10.0, 5.0, 5.0));
    e.extend(gate(p, 16.0, 10.0));
    e.push(pad(16.0, 10.0, 1.0, 3.0));
    e.push(rect(p.m1, 10.4, 10.55, 16.6, 10.75));
    let cpad = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            rect(p.gp, x + 0.4, y + 0.8, x + 0.6 + w, y + 1.0 + h),
            rect(p.cont, x + 0.5, y + 0.9, x + 0.5 + w, y + 0.9 + h),
        ]
    };
    e.extend(gate(p, 2.0, 20.0));
    e.extend(cpad(2.0, 20.0, 1.0, 1.99));
    e.extend(gate(p, 6.0, 20.0));
    e.extend(cpad(6.0, 20.0, 1.0, 1.995));
    e.extend(gate(p, 10.0, 20.0));
    e.extend(cpad(10.0, 20.0, 1.0, 2.5));
    e.extend(gate(p, 14.0, 20.0));
    e.extend(cpad(14.0, 20.0, 1.0, 0.5));
    e.push(rect(p.m1, 10.4, 20.8, 15.6, 21.5));
    e.push(rect(p.activ, 18.0, 20.0, 21.0, 21.0));
    e.push(rect(p.gp, 18.4, 19.7, 18.6, 21.3));
    e.push(rect(p.cont, 19.0, 20.0, 21.0, 21.0));
    write_gz(&format!("{ANT}/Ant.ac.h1.gds.gz"), library("TOP", e));
}

/// Ant.g - "Size of protection diode (µm²) (Note 4)  0.16"; note 4: "PDarea (µm²) =
/// 0.02 x (Vn_area / (GatPoly over Activ)_area)", Vn_area the cumulative Cont and via
/// area.
fn ant_g_h(p: &P) {
    // h1: a gate under a small Metal1 whose Metal1 reaches an n-diode of 0.4 × 0.4 = 0.16
    // at x = 2 (clean); 0.4 × 0.395 = 0.158 at x = 6 (fires); a 0.1 × 0.1 diode under its
    // own Metal1, on no gate's net, at x = 10 (clean); a 0.158 diode reached through
    // Via1-Metal2-Via1 at x = 14 (fires); a p-diode of 0.158 in an NWell at x = 2, y = 8
    // (fires).  At y = 16: the 0.16 diode as two
    // abutting 0.2 × 0.4 boxes (clean); a 0.4 × 0.395 Activ under a 1 × 1 marker (fires);
    // a 1 × 1 Activ under a 0.4 × 0.395 marker (the diode is Activ AND marker; fires).
    let m1 = |x: f64| rect(p.m1, x, 2.6, x + 2.5, 3.6);
    let mut e = vec![];
    for (x, w, h) in [(2.0, 0.4, 0.4), (6.0, 0.4, 0.395)] {
        e.extend(gate(p, x, 2.0));
        e.push(m1(x));
        e.extend(ndiode(p, x + 1.5, 2.9, w, h));
    }
    e.extend(ndiode(p, 10.0, 2.0, 0.1, 0.1));
    e.push(rect(p.m1, 9.9, 1.9, 10.5, 2.5));
    e.extend(gate(p, 14.0, 2.0));
    e.push(rect(p.m1, 14.0, 2.6, 15.0, 3.6));
    e.push(rect(p.via1, 14.7, 3.0, 14.9, 3.2));
    e.push(rect(p.m2, 14.6, 2.9, 16.4, 3.3));
    e.push(rect(p.via1, 16.1, 3.0, 16.3, 3.2));
    e.push(rect(p.m1, 16.0, 2.5, 17.0, 3.5));
    e.extend(ndiode(p, 16.3, 2.7, 0.4, 0.395));
    e.extend(gate(p, 2.0, 8.0));
    e.push(rect(p.m1, 2.0, 8.6, 4.5, 9.6));
    e.push(rect(p.nw, 3.0, 8.4, 4.5, 9.8));
    e.push(rect(p.activ, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.psd, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.diode, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.cont, 3.65, 9.05, 3.75, 9.15));
    e.extend(gate(p, 2.0, 16.0));
    e.push(rect(p.m1, 2.0, 16.6, 4.5, 17.6));
    e.push(rect(p.activ, 3.5, 16.9, 3.7, 17.3));
    e.push(rect(p.activ, 3.7, 16.9, 3.9, 17.3));
    e.push(rect(p.diode, 3.5, 16.9, 3.9, 17.3));
    e.push(rect(p.cont, 3.65, 17.05, 3.75, 17.15));
    e.extend(gate(p, 6.0, 16.0));
    e.push(rect(p.m1, 6.0, 16.6, 8.5, 17.6));
    e.push(rect(p.activ, 7.5, 16.9, 7.9, 17.295));
    e.push(rect(p.diode, 7.2, 16.6, 8.2, 17.6));
    e.push(rect(p.cont, 7.65, 17.05, 7.75, 17.15));
    e.extend(gate(p, 10.0, 16.0));
    e.push(rect(p.m1, 10.0, 16.6, 12.5, 17.6));
    e.push(rect(p.activ, 11.2, 16.6, 12.2, 17.6));
    e.push(rect(p.diode, 11.5, 16.9, 11.9, 17.295));
    e.push(rect(p.cont, 11.65, 17.05, 11.75, 17.15));
    write_gz(&format!("{ANT}/Ant.g.h1.gds.gz"), library("TOP", e));

    // h2, note 4: a 0.25 µm² n-diode on a net whose Via1 is 2.0 × 2.0 = 4.0 µm² on a 0.1
    // gate - Vn_area / gate area = 40.1 with the poly Cont, under Ant.f's 500 - so the
    // diode must be 0.02 × 40.1 = 0.80 µm² (fires by note 4; both tools read the 0.16
    // only).
    let mut e = gate(p, 2.0, 2.0);
    e.push(rect(p.m1, 2.0, 2.6, 5.5, 5.0));
    e.push(rect(p.via1, 3.0, 3.0, 5.0, 5.0));
    e.push(rect(p.m2, 2.9, 2.9, 5.1, 5.1));
    e.extend(ndiode(p, 2.2, 3.0, 0.5, 0.5));
    write_gz(&format!("{ANT}/Ant.g.h2.gds.gz"), library("TOP", e));
}

/// Ant.h - "dantenna in NWell not allowed"; Ant.i - "dpantenna in PWell not allowed".
fn ant_hi(p: &P) {
    // A 1 × 1 n-diode (Activ AND Recog:diode, no pSD) at (x, y).
    let nd = |x: f64, y: f64| {
        vec![
            rect(p.activ, x, y, x + 1.0, y + 1.0),
            rect(p.diode, x, y, x + 1.0, y + 1.0),
        ]
    };
    // Ant.h.h1: an n-diode in an NWell across x = 20 (fires); one with Recog:esd over it
    // (an ESD device; clean); one whose right half is in the NWell (fires on that half);
    // an isolbox - the diode under nBuLay with an "isolbox" TEXT inside the nBuLay (clean);
    // the same with "ISOLBOX" (clean); the text 0.5 outside the nBuLay (fires); the text
    // on Metal1:label (fires); the text "isolbox2" (fires); the diode's Activ under
    // nSD:block (not N+; clean); an n-diode under PWell:block with no NWell (clean).
    let mut e = vec![rect(p.nw, 18.5, 1.5, 21.5, 4.5)];
    e.extend(nd(19.5, 2.5));
    e.push(rect(p.nw, 1.0, 1.0, 4.0, 4.0));
    e.extend(nd(2.0, 2.0));
    e.push(rect(p.esd, 1.5, 1.5, 3.5, 3.5));
    e.push(rect(p.nw, 8.5, 1.0, 11.0, 4.0));
    e.extend(nd(8.0, 2.0));
    for (i, (label, lay, dx)) in [
        ("isolbox", p.txt, 0.0),
        ("ISOLBOX", p.txt, 0.0),
        ("isolbox", p.txt, 2.0),
        ("isolbox", p.m1_label, 0.0),
        ("isolbox2", p.txt, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        let (x, y) = (2.0 + 6.0 * i as f64, 8.0);
        e.push(rect(p.nw, x - 1.0, y - 1.0, x + 2.0, y + 2.0));
        e.extend(nd(x, y));
        e.push(rect(p.nbl, x - 0.5, y - 0.5, x + 1.5, y + 1.5));
        e.push(text(lay, label, x + 0.5 + dx, y + 0.5));
    }
    e.push(rect(p.nw, 1.0, 13.0, 4.0, 16.0));
    e.extend(nd(2.0, 14.0));
    e.push(rect(p.nsdb, 1.5, 13.5, 3.5, 15.5));
    e.push(rect(p.pwb, 7.0, 13.0, 10.0, 16.0));
    e.extend(nd(8.0, 14.0));
    write_gz(&format!("{ANT}/Ant.h.h1.gds.gz"), library("TOP", e));

    // A 1 × 1 p-diode (Activ AND pSD AND Recog:diode) at (x, y).
    let pd = |x: f64, y: f64| {
        vec![
            rect(p.activ, x, y, x + 1.0, y + 1.0),
            rect(p.psd, x, y, x + 1.0, y + 1.0),
            rect(p.diode, x, y, x + 1.0, y + 1.0),
        ]
    };
    // Ant.i.h1: a p-diode on bare substrate across x = 20 (fires); in an NWell (clean);
    // its right half in an NWell (fires on the left half); under PWell:block (not PWell
    // by section 4.2; clean); on bare substrate with Recog:esd (clean); a P+ Activ with
    // no Recog:diode on bare substrate (a ptap, no diode; clean).
    let mut e = pd(19.5, 2.5);
    e.push(rect(p.nw, 1.0, 1.0, 4.0, 4.0));
    e.extend(pd(2.0, 2.0));
    e.push(rect(p.nw, 8.5, 1.0, 11.0, 4.0));
    e.extend(pd(8.0, 2.0));
    e.push(rect(p.pwb, 1.0, 7.0, 4.0, 10.0));
    e.extend(pd(2.0, 8.0));
    e.extend(pd(8.0, 8.0));
    e.push(rect(p.esd, 7.5, 7.5, 9.5, 9.5));
    e.push(rect(p.activ, 2.0, 14.0, 3.0, 15.0));
    e.push(rect(p.psd, 2.0, 14.0, 3.0, 15.0));
    write_gz(&format!("{ANT}/Ant.i.h1.gds.gz"), library("TOP", e));

    // Ant.i.h2: a p-diode in an NWell under a drawn PWell - PWell:drawing is PWell by
    // section 4.2, so the diode is in PWell (fires; both tools read NWell OR PWell:block
    // only).
    let mut e = vec![
        rect(p.nw, 1.0, 1.0, 4.0, 4.0),
        rect(p.pw, 1.5, 1.5, 3.5, 3.5),
    ];
    e.extend(pd(2.0, 2.0));
    write_gz(&format!("{ANT}/Ant.i.h2.gds.gz"), library("TOP", e));
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/antenna")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    antenna(&p);
}
