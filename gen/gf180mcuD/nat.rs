// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native-Vt device: a good and a bad pattern for every rule in the `nat` deck.
//!
//! The device is an NMOS whose channel is left unimplanted, so a base cell is a NAT
//! marker over an N+ active with a poly gate across it, clear of any N-well.  Four of the
//! rules are about the marker's own geometry and four say the marker may not sit over the
//! wrong thing, so most fixtures move the marker rather than the transistor.
//!
//! NAT.4 and NAT.5 are the same rule at the two voltages, and what they bound is the
//! channel *length* - the gate's own width where it crosses the active.  The two are told
//! apart by Dualgate, so the 6 V fixture wraps the marker in it and the 3.3 V one does
//! not, which also keeps each quiet on the other's pattern.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/nat";

/// Active area: long enough for a gate and two heads, and 2 µm tall.
const COMP_L: f64 = 8.0;
const COMP_W: f64 = 2.0;
/// The N+ implant's reach past the active.
const NP: f64 = 0.4;
/// How far the NAT marker reaches past the active - NAT.1 asks 2.0.
const NAT_M: f64 = 2.2;
/// Gate length, over NAT.4's 1.8 µm, and how far the gate overhangs the active.
const GATE: f64 = 2.0;
const GATE_EXT: f64 = 0.5;

struct Ctx {
    nat: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    poly: (i16, i16),
    nwell: (i16, i16),
    dualgate: (i16, i16),
}

/// One native transistor, with the margins each rule needs to move.
struct Cell {
    x: f64,
    y: f64,
    /// NAT marker reach past the active.
    nat_m: f64,
    /// Gate length.
    gate: f64,
    /// Whether a gate is drawn at all - NAT.11 wants an active without one.
    gated: bool,
    /// Whether the marker is wrapped in Dualgate, which makes the device 6 V.
    mv: bool,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            nat_m: NAT_M,
            gate: GATE,
            gated: true,
            mv: false,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let mut v = vec![
            rect(c.comp, x, y, x + COMP_L, y + COMP_W),
            rect(c.nplus, x - NP, y - NP, x + COMP_L + NP, y + COMP_W + NP),
            rect(
                c.nat,
                x - self.nat_m,
                y - self.nat_m,
                x + COMP_L + self.nat_m,
                y + COMP_W + self.nat_m,
            ),
        ];
        if self.gated {
            let gx = x + (COMP_L - self.gate) * 0.5;
            v.push(rect(
                c.poly,
                gx,
                y - GATE_EXT,
                gx + self.gate,
                y + COMP_W + GATE_EXT,
            ));
        }
        if self.mv {
            // Covering the marker entirely, which is what NAT.8 asks of any Dualgate that
            // touches it at all.
            let m = self.nat_m + 0.5;
            v.push(rect(
                c.dualgate,
                x - m,
                y - m,
                x + COMP_L + m,
                y + COMP_W + m,
            ));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        nat: layer(pdk, "nat"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        poly: layer(pdk, "poly2_drawn"),
        nwell: layer(pdk, "nwell"),
        dualgate: layer(pdk, "dualgate"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);
    let mv_clean = || {
        let mut cell = Cell::at(o, o);
        cell.mv = true;
        cell.draw(&c)
    };

    for id in [
        "NAT.1", "NAT.2", "NAT.3", "NAT.4", "NAT.7", "NAT.8", "NAT.9", "NAT.10", "NAT.11", "NAT.12",
    ] {
        write(id, "good", clean());
    }
    // NAT.6: a second active under the same marker with nothing tying it to the first, so
    // the marker covers two potentials.  It sits in the margin the marker already keeps
    // past the active, which is where a second one would be.
    // Its marker is drawn wide enough to hold both actives with NAT.1's two microns to
    // spare, which the default one has no room for.
    let two_actives = |second: bool| {
        let mut cell = Cell::at(o, o);
        cell.nat_m = 7.0;
        let mut v = cell.draw(&c);
        if second {
            // A second device, not merely a second active: NAT.11 asks that every active
            // under the marker have a gate on it.
            let (x0, y0, x1, y1) = (o + 1.0, o + COMP_W + 2.4, o + 7.0, o + COMP_W + 3.6);
            v.push(rect(c.comp, x0, y0, x1, y1));
            v.push(rect(c.nplus, x0 - NP, y0 - NP, x1 + NP, y1 + NP));
            let gx = x0 + (x1 - x0 - GATE) * 0.5;
            v.push(rect(c.poly, gx, y0 - GATE_EXT, gx + GATE, y1 + GATE_EXT));
        }
        v
    };
    write("NAT.6", "good", two_actives(false));
    write("NAT.6", "bad", two_actives(true));

    // NAT.9: poly that belongs to no gate of the marker, closer than 0.3 µm to it.  The
    // rule's other half is a poly interconnect *under* the marker, which needs two gates
    // to run between and so cannot be drawn on a one-gate device.
    let stray_poly = |gap: f64| {
        let mut v = Cell::at(o, o).draw(&c);
        let x = o + COMP_L + NAT_M + gap;
        v.push(rect(c.poly, x, o, x + 1.0, o + COMP_W));
        v
    };
    write("NAT.9", "bad", stray_poly(0.295));

    // NAT.5 is the 6 V rule, so both its halves carry Dualgate.
    write("NAT.5", "good", mv_clean());

    // NAT.1: the marker reaching only 1.995 µm past the active.
    write("NAT.1", "bad", {
        let mut cell = Cell::at(o, o);
        cell.nat_m = 1.995;
        cell.draw(&c)
    });

    // NAT.2: an unrelated active 0.295 µm from the marker.  It carries no N+ and no gate,
    // so it is not a second device.
    write("NAT.2", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.295;
        v.push(rect(c.comp, x, o, x + 2.0, o + COMP_W));
        v
    });

    // NAT.3: an N-well 0.495 µm from the marker.  Clear of it, so NAT.10 - which is about
    // a well *over* the marker - has nothing to say.
    write("NAT.3", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.495;
        v.push(rect(c.nwell, x, o, x + 3.0, o + COMP_W));
        v
    });

    // NAT.4: a 3.3 V channel shorter than 1.8 µm.
    write("NAT.4", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gate = 1.795;
        cell.draw(&c)
    });

    // NAT.5: the same at 6 V, which is the Dualgate one.
    write("NAT.5", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gate = 1.795;
        cell.mv = true;
        cell.draw(&c)
    });

    // NAT.7: a second marker 0.735 µm away.  Bare, so it is only a marker: nothing in it
    // to enclose, no active to want a gate.
    write("NAT.7", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.735;
        v.push(rect(c.nat, x, o, x + 3.0, o + COMP_W));
        v
    });

    // NAT.8: Dualgate over part of the marker but not all of it.  A marker it touches at
    // all it has to cover.
    write("NAT.8", "bad", {
        let mut v = clean();
        v.push(rect(
            c.dualgate,
            o + COMP_L * 0.5,
            o - NAT_M - 1.0,
            o + COMP_L + NAT_M + 1.0,
            o + COMP_W + NAT_M + 1.0,
        ));
        v
    });

    // NAT.10: an N-well over a corner of the marker, away from the active - a well *under*
    // the active would leave nothing for NAT.1 to measure.
    write("NAT.10", "bad", {
        let mut v = clean();
        v.push(rect(
            c.nwell,
            o - NAT_M - 1.0,
            o - NAT_M - 1.0,
            o - NAT_M + 1.0,
            o - NAT_M + 1.0,
        ));
        v
    });

    // NAT.11: the active in the marker with no gate across it.
    write("NAT.11", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gated = false;
        cell.draw(&c)
    });

    // NAT.12: poly over the marker that reaches no active at all.
    write("NAT.12", "bad", {
        let mut v = clean();
        v.push(rect(
            c.poly,
            o - NAT_M + 0.2,
            o - NAT_M + 0.2,
            o - NAT_M + 1.2,
            o - 0.6,
        ));
        v
    });
}
