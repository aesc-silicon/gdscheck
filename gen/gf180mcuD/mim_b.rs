// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! MIM capacitor, option B: a good and a bad pattern for every rule in the `mim_b` deck.
//!
//! The capacitor is a FuseTop plate over the metal below the top one, with vias landing
//! on each, and the deck measures almost everything against a layer that is not drawn at
//! all: the *virtual* bottom plate, which is the FuseTop grown by 1.06 µm and clipped to
//! the metal under it.  So the base cell draws the metal exactly 1.06 µm past the plate,
//! which makes the virtual plate and the metal the same shape and every margin in the
//! deck readable off one number.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/mim_b";

/// Top plate side: 36 µm², between MIMTM.8a's 25 and MIMTM.8b's 10000.
const PLATE: f64 = 6.0;
/// How far the metal below runs past the plate.  Exactly the oversize the virtual plate
/// is built with, so the two coincide.
const BOT: f64 = 1.06;
/// Via side, and how far inside the plate a via sits - MIMTM.4 asks 0.4.
const VIA: f64 = 0.26;
const VIA_IN: f64 = 0.6;
/// How far the CAP_MK marker runs past the plate; MIMTM.7 only asks that it covers it.
const CAP: f64 = 0.5;

struct Ctx {
    metal: (i16, i16),
    top_via: (i16, i16),
    low_via: (i16, i16),
    fusetop: (i16, i16),
    cap_mk: (i16, i16),
}

struct Cell {
    x: f64,
    y: f64,
    plate: f64,
    /// The metal's reach past the plate, per side: left, right, bottom, top.
    bot: [f64; 4],
    via_in: f64,
    /// Whether the CAP_MK marker is drawn.
    capped: bool,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            plate: PLATE,
            bot: [BOT; 4],
            via_in: VIA_IN,
            capped: true,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y, p) = (self.x, self.y, self.plate);
        let [l, r, b, t] = self.bot;
        let mut v = vec![
            rect(c.metal, x - l, y - b, x + p + r, y + p + t),
            rect(c.fusetop, x, y, x + p, y + p),
        ];
        if self.capped {
            v.push(rect(c.cap_mk, x - CAP, y - CAP, x + p + CAP, y + p + CAP));
        }
        // One via on the plate, set in far enough for MIMTM.4.
        v.push(rect(
            c.top_via,
            x + self.via_in,
            y + self.via_in,
            x + self.via_in + VIA,
            y + self.via_in + VIA,
        ));
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        metal: layer(pdk, "metal4_drawn"),
        top_via: layer(pdk, "via4"),
        low_via: layer(pdk, "via3"),
        fusetop: layer(pdk, "fusetop"),
        cap_mk: layer(pdk, "cap_mk"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);

    for id in [
        "MIMTM.1", "MIMTM.2", "MIMTM.3", "MIMTM.4", "MIMTM.5", "MIMTM.6", "MIMTM.7", "MIMTM.8a",
        "MIMTM.8b", "MIMTM.9", "MIMTM.10",
    ] {
        write(id, "good", clean());
    }

    // MIMTM.11: two caps whose bottom metal is one plate.  Each is well inside MIMTM.8b's
    // cap on a single MIM; what this rule adds is that they add up against it, so the pair
    // is drawn either side of that sum.
    let shared = |side: f64| {
        let a = Cell {
            plate: side,
            ..Cell::at(o, o)
        };
        let b = Cell {
            x: o + side + 2.0 * BOT,
            plate: side,
            ..Cell::at(o, o)
        };
        let mut v = a.draw(&c);
        v.extend(b.draw(&c));
        v
    };
    write("MIMTM.11", "good", shared(70.0));
    write("MIMTM.11", "bad", shared(72.0));

    // MIMTM.1: other metal below, 1.2 µm from the virtual plate - which here is the cell's
    // own metal, so the gap is measured from that edge.
    write("MIMTM.1", "bad", {
        let mut v = clean();
        let x = o + PLATE + BOT + 1.195;
        v.push(rect(c.metal, x, o, x + 3.0, o + PLATE));
        v
    });

    // MIMTM.2: a via on the bottom plate that the metal encloses by only 0.4 µm.  It has
    // to sit inside the virtual plate to count, which is why the metal stops at the
    // oversize: any further out and the via would be past the plate's edge instead.
    write("MIMTM.2", "bad", {
        let mut v = clean();
        let x = o + PLATE + BOT - 0.395 - VIA;
        v.push(rect(
            c.top_via,
            x,
            o + PLATE * 0.5,
            x + VIA,
            o + PLATE * 0.5 + VIA,
        ));
        v
    });

    // MIMTM.3: the metal running only 0.6 µm past the plate on one side, so the virtual
    // plate - clipped to it - falls short there.
    write("MIMTM.3", "bad", {
        let mut cell = Cell::at(o, o);
        cell.bot = [BOT, 0.595, BOT, BOT];
        cell.draw(&c)
    });

    // MIMTM.4: a via on the top plate, only 0.4 µm inside it.
    write("MIMTM.4", "bad", {
        let mut cell = Cell::at(o, o);
        cell.via_in = 0.395;
        cell.draw(&c)
    });

    // MIMTM.5: a via on the metal below, within 0.4 µm of the top plate.
    write("MIMTM.5", "bad", {
        let mut v = clean();
        let x = o + PLATE + 0.395;
        v.push(rect(
            c.top_via,
            x,
            o + PLATE * 0.5,
            x + VIA,
            o + PLATE * 0.5 + VIA,
        ));
        v
    });

    // MIMTM.6: a second top plate 0.6 µm away, on its own metal so the first plate's
    // margins are untouched.
    write("MIMTM.6", "bad", {
        let mut v = clean();
        let mut second = Cell::at(o + PLATE + 0.595, o);
        second.bot = [0.0, BOT, BOT, BOT];
        v.extend(second.draw(&c));
        v
    });

    // MIMTM.7: a top plate the CAP_MK marker does not cover.
    write("MIMTM.7", "bad", {
        let mut cell = Cell::at(o, o);
        cell.capped = false;
        cell.draw(&c)
    });

    // MIMTM.8a / 8b: the plate under 25 µm² and over 10000 µm².
    for (id, p) in [("MIMTM.8a", 4.9), ("MIMTM.8b", 101.0)] {
        write(id, "bad", {
            let mut cell = Cell::at(o, o);
            cell.plate = p;
            cell.draw(&c)
        });
    }

    // MIMTM.9: two vias on the plate, 0.5 µm apart.
    write("MIMTM.9", "bad", {
        let mut v = clean();
        let x = o + VIA_IN + VIA + 0.495;
        v.push(rect(c.top_via, x, o + VIA_IN, x + VIA, o + VIA_IN + VIA));
        v
    });

    // MIMTM.10: a lower via reaching the bottom plate under the top one, which is the one
    // connection the option forbids - the bottom plate is reached from above.
    write("MIMTM.10", "bad", {
        let mut v = clean();
        let x = o + PLATE * 0.5;
        v.push(rect(
            c.low_via,
            x,
            o + PLATE * 0.5,
            x + VIA,
            o + PLATE * 0.5 + VIA,
        ));
        v
    });
}
