// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, text, write_gz};
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
    elems.extend(gate_stack(o + 9.0, (o + 9.0, o + 0.6, o + 9.5, o + 1.6), o + 9.05));
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
