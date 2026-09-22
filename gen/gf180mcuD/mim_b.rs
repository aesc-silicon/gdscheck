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

    let h = |name: &str, elems: Vec<GdsElement>| {
        write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
    };
    hardening(&c, &h);
    edges(&c, &h);
}

// --- Hardening (hardening/SPEC.md) ------------------------------------------------
//
// What the deck has that no other deck has is a plate nobody draws: the *virtual* bottom
// plate, the FuseTop grown by 1.06 µm and clipped to the metal below the top one.  Three
// rules measure against it and one of them decides which vias it even applies to, so the
// patterns below are mostly about where that plate ends - which is the grow, not the
// metal - and about the selections the other rules make: a via `overlapping` the virtual
// plate, a via `inside` the top plate, the marker that has to cover the plate, the metal
// polygon an area is summed over.

/// A capacitor without a via: the metal below, the plate, and the marker over it.
/// `bot` is the metal's reach past the plate on each side - left, right, bottom, top.
fn cap(c: &Ctx, x: f64, y: f64, w: f64, h: f64, bot: [f64; 4]) -> Vec<GdsElement> {
    let [l, r, b, t] = bot;
    vec![
        rect(c.metal, x - l, y - b, x + w + r, y + h + t),
        rect(c.fusetop, x, y, x + w, y + h),
        rect(c.cap_mk, x - CAP, y - CAP, x + w + CAP, y + h + CAP),
    ]
}

/// A via of side `s` with its lower-left corner at (x, y).
fn via(l: (i16, i16), x: f64, y: f64, s: f64) -> GdsElement {
    rect(l, x, y, x + s, y + s)
}

/// Two capacitors 2 µm apart sharing one bottom-plate metal, 100 µm tall.
fn shared_plate(c: &Ctx, wa: f64, wb: f64) -> Vec<GdsElement> {
    let (x, y, h) = (5.0, 5.0, 100.0);
    let xb = x + wa + 2.0;
    vec![
        rect(c.metal, x - BOT, y - BOT, xb + wb + BOT, y + h + BOT),
        rect(c.fusetop, x, y, x + wa, y + h),
        rect(c.fusetop, xb, y, xb + wb, y + h),
        rect(c.cap_mk, x - CAP, y - CAP, xb + wb + CAP, y + h + CAP),
    ]
}

fn hardening(c: &Ctx, write: &dyn Fn(&str, Vec<GdsElement>)) {
    let n = [BOT; 4];

    // MIMTM.1.h1: one capacitor whose metal runs 2.0 µm past the plate on every side, and
    // nothing else on the layout.  The virtual plate stops at the 1.06 µm oversize, so
    // 0.94 µm of the capacitor's own metal lies outside it all the way round.  That metal
    // is not "adjacent MiM or routing metal" - it is the plate itself - and the rule has
    // nothing to measure.  Clean.
    write("MIMTM.1.h1", cap(c, 5.0, 5.0, 6.0, 6.0, [2.0; 4]));

    // MIMTM.1.h2: the same 2.0 µm overhang, with a routing metal bar beside each plate.
    // The gap the manual asks about is measured from the *virtual* plate - FuseTop plus
    // 1.06 - not from the metal's own edge.  Left plate: the bar is 1.2 µm from the
    // virtual edge (0.26 from the metal), clean.  Right plate: 1.195 µm, fires once.
    write("MIMTM.1.h2", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, [2.0; 4]);
        let x = 11.0 + BOT + 1.2;
        v.push(rect(c.metal, x, 5.0, x + 3.0, 11.0));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, [2.0; 4]));
        let x = 36.0 + BOT + 1.195;
        v.push(rect(c.metal, x, 5.0, x + 3.0, 11.0));
        v
    });

    // MIMTM.1.h3: the 1.195 µm gap of h2 put on a tile line - once straddling x = 20 and
    // once x = 42 - with the metal at its nominal 1.06 µm reach, so the virtual plate and
    // the metal coincide.  Two violations.
    write("MIMTM.1.h3", {
        let mut v = Vec::new();
        for (edge, y) in [(20.0, 5.0), (42.0, 25.0)] {
            // The gap is centred on the tile line: virtual edge at edge - 0.5975.
            let right = edge - 0.5975 - BOT;
            v.extend(cap(c, right - 6.0, y, 6.0, 6.0, n));
            v.push(rect(c.metal, edge + 0.5975, y, edge + 4.0, y + 6.0));
        }
        v
    });

    // MIMTM.2.h1: which vias the rule applies to, and the bound.  Top left: a via in the
    // ring between the plate and the metal's edge, held by 0.395, fires.  Top right: the
    // same held by exactly 0.4, clean.  Bottom left: a via outside the metal sharing its
    // edge - no area inside the virtual plate, so not a bottom-plate via at all.  Bottom
    // right: a via held by 0.395 in routing metal far from any plate - the rule is for
    // vias within the 1.06 µm oversize of FuseTop, and this one is not.
    write("MIMTM.2.h1", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, n);
        v.push(via(c.top_via, 11.0 + BOT - 0.395 - VIA, 7.5, VIA));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, n));
        v.push(via(c.top_via, 36.0 + BOT - 0.4 - VIA, 7.5, VIA));
        v.extend(cap(c, 5.0, 30.0, 6.0, 6.0, n));
        v.push(via(c.top_via, 11.0 + BOT, 32.5, VIA));
        v.push(rect(c.metal, 30.0, 30.0, 36.0, 36.0));
        v.push(via(c.top_via, 36.0 - 0.395 - VIA, 32.5, VIA));
        v
    });

    // MIMTM.2.h2: the oversize is what bounds the virtual plate, not the metal.  Left: the
    // metal runs 2.0 µm past the plate and a via sits in the strip beyond the 1.06 µm
    // oversize - outside the virtual plate, so not this rule's via, even though the metal
    // holds it by only 0.24.  Right: a via straddling the metal's outer edge with part of
    // it inside the virtual plate - the bottom plate does not cover it at all, which is
    // the crossing half of the rule.
    write("MIMTM.2.h2", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, [2.0; 4]);
        v.push(via(c.top_via, 11.0 + 1.5, 7.5, VIA));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, n));
        v.push(via(c.top_via, 36.0 + BOT - 0.16, 7.5, VIA));
        v
    });

    // MIMTM.3.h1: a plate with no metal under it at all.  There is no bottom plate, so
    // there is no 0.6 µm overlap of the top plate anywhere; the rule fires.
    write("MIMTM.3.h1", {
        vec![
            rect(c.fusetop, 5.0, 5.0, 11.0, 11.0),
            rect(c.cap_mk, 4.5, 4.5, 11.5, 11.5),
        ]
    });

    // MIMTM.3.h2: left, the metal covers only the left half of the plate, so the plate
    // runs out of the bottom plate on the right - the crossing half of the same rule.
    // Right, the plain bound: 0.595 µm of overlap on that side, and 1.06 elsewhere.
    write("MIMTM.3.h2", {
        let mut v = vec![
            rect(c.metal, 5.0 - BOT, 5.0 - BOT, 8.0, 11.0 + BOT),
            rect(c.fusetop, 5.0, 5.0, 11.0, 11.0),
            rect(c.cap_mk, 4.5, 4.5, 11.5, 11.5),
        ];
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, [BOT, 0.595, BOT, BOT]));
        v
    });

    // MIMTM.3.h3: the metal runs 2.0 µm past the plate.  The virtual plate is the grow, so
    // the overlap is 1.06 - not 2.0 - and well over the 0.6 the rule asks.  Clean.
    write("MIMTM.3.h3", cap(c, 5.0, 5.0, 6.0, 6.0, [2.0; 4]));

    // MIMTM.4.h1: a via straddling the top plate's edge.  Part of it is off the plate, so
    // the plate does not overlap it by 0.4 µm - the crossing half of the rule.  The via is
    // not `inside` the plate, so MIMTM.9 does not see it, and the metal below still holds
    // it by 0.9, so MIMTM.2 is clean.
    write("MIMTM.4.h1", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, n);
        v.push(via(c.top_via, 11.0 - 0.1, 7.5, VIA));
        v
    });

    // MIMTM.4.h2: a via abutting the top plate's edge from outside - an overlap of nothing
    // and a spacing to the plate of nothing.  Both rules read a shared edge, so MIMTM.4
    // and MIMTM.5 fire together.
    write("MIMTM.4.h2", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, n);
        v.push(via(c.top_via, 11.0, 7.5, VIA));
        v
    });

    // MIMTM.5.h1: the rule measures the plate against the vias that reach the *bottom*
    // plate, which are the ones on the metal.  Left: a via 0.395 µm from the plate on the
    // metal ring, fires.  Right: the metal stops flush with the plate's right edge, so the
    // via 0.395 µm away lands on nothing and is not a bottom-plate via - MIMTM.5 clean.
    // That flush metal is an overlap of nothing, so MIMTM.3 fires there instead.
    write("MIMTM.5.h1", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, n);
        v.push(via(c.top_via, 11.0 + 0.395, 7.5, VIA));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, [BOT, 0.0, BOT, BOT]));
        v.push(via(c.top_via, 36.0 + 0.395, 7.5, VIA));
        v
    });

    // MIMTM.6.h1: a U-shaped top plate.  The manual's 0.6 µm is between plates; the
    // opening of a single plate is the same distance and upstream's `space` reads it as
    // one.  Left opening 0.595 (fires), right 0.6 (clean).
    write("MIMTM.6.h1", {
        let mut v = Vec::new();
        for (x, gap) in [(5.0, 0.595), (30.0, 0.6)] {
            let (y, w, h) = (5.0, 8.0, 6.0);
            let m = (w - gap) * 0.5;
            v.push(rect(c.metal, x - BOT, y - BOT, x + w + BOT, y + h + BOT));
            v.push(rect(c.fusetop, x, y, x + w, y + 2.0));
            v.push(rect(c.fusetop, x, y + 2.0, x + m, y + h));
            v.push(rect(c.fusetop, x + m + gap, y + 2.0, x + w, y + h));
            v.push(rect(c.cap_mk, x - CAP, y - CAP, x + w + CAP, y + h + CAP));
        }
        v
    });

    // MIMTM.7.h1: the marker is an enclosure of 0, so it has to cover the plate and need
    // do no more.  Top left: CAP_MK exactly on the plate, clean.  Top right: CAP_MK short
    // of the plate's right edge by 0.5, fires.  Bottom left: CAP_MK drawn as a frame with
    // a hole over the middle of the plate, fires.  Bottom right: CAP_MK drawn as two
    // abutting boxes whose union covers the plate, clean - the marker is the union.
    write("MIMTM.7.h1", {
        let mut v = Vec::new();
        for (x, y) in [(5.0, 5.0), (30.0, 5.0), (5.0, 30.0), (30.0, 30.0)] {
            v.push(rect(
                c.metal,
                x - BOT,
                y - BOT,
                x + 6.0 + BOT,
                y + 6.0 + BOT,
            ));
            v.push(rect(c.fusetop, x, y, x + 6.0, y + 6.0));
        }
        v.push(rect(c.cap_mk, 5.0, 5.0, 11.0, 11.0));
        v.push(rect(c.cap_mk, 29.5, 4.5, 35.5, 11.5));
        for (x0, y0, x1, y1) in [
            (4.5, 29.5, 11.5, 32.0),
            (4.5, 34.0, 11.5, 36.5),
            (4.5, 32.0, 7.0, 34.0),
            (9.0, 32.0, 11.5, 34.0),
        ] {
            v.push(rect(c.cap_mk, x0, y0, x1, y1));
        }
        v.push(rect(c.cap_mk, 29.5, 29.5, 33.0, 36.5));
        v.push(rect(c.cap_mk, 33.0, 29.5, 36.5, 36.5));
        v
    });

    // MIMTM.8a.h1: the 25 µm² floor, on a rectangle and on an L drawn as two boxes - the
    // area is the merged shape's, not each box's.  5.0 × 5.0 = 25 exactly is clean,
    // 4.995 × 5.0 = 24.975 fires; the L is 24.975 and 25.0 the same way.
    write("MIMTM.8a.h1", {
        let mut v = Vec::new();
        for (x, y, w) in [(5.0, 5.0, 5.0), (20.0, 5.0, 4.995)] {
            v.push(rect(c.metal, x - BOT, y - BOT, x + w + BOT, y + 5.0 + BOT));
            v.push(rect(c.fusetop, x, y, x + w, y + 5.0));
            v.push(rect(c.cap_mk, x - CAP, y - CAP, x + w + CAP, y + 5.0 + CAP));
        }
        for (x, y, arm) in [(5.0, 20.0, 4.975), (20.0, 20.0, 5.0)] {
            v.push(rect(
                c.metal,
                x - BOT,
                y - BOT,
                x + 5.0 + BOT,
                y + 4.0 + arm + BOT,
            ));
            v.push(rect(c.fusetop, x, y, x + 5.0, y + 4.0));
            v.push(rect(c.fusetop, x, y + 4.0, x + 1.0, y + 4.0 + arm));
            v.push(rect(
                c.cap_mk,
                x - CAP,
                y - CAP,
                x + 5.0 + CAP,
                y + 4.0 + arm + CAP,
            ));
        }
        v
    });

    // MIMTM.8b.h1: the 10000 µm² cap on one capacitor.  100 × 100 is exactly 10000 and
    // clean; 100.005 × 100 is 10000.5 and fires.  Each is the only MIM on its bottom
    // plate, so the second is also the whole of that plate's total and MIMTM.11 fires with
    // it.  Both plates cross five tile lines at 20 µm.
    write("MIMTM.8b.h1", {
        let mut v = Vec::new();
        for (y, w) in [(5.0, 100.0), (120.0, 100.005)] {
            v.push(rect(
                c.metal,
                5.0 - BOT,
                y - BOT,
                5.0 + w + BOT,
                y + 100.0 + BOT,
            ));
            v.push(rect(c.fusetop, 5.0, y, 5.0 + w, y + 100.0));
            v.push(rect(c.cap_mk, 4.5, y - CAP, 5.0 + w + CAP, y + 100.0 + CAP));
        }
        v
    });

    // MIMTM.9.h1: the sea-of-via pitch on the plate.  Left: two vias 0.495 apart, fires.
    // Right: 0.5, clean.  Both pairs are held 0.6 inside the plate, for MIMTM.4.
    write("MIMTM.9.h1", {
        let mut v = Vec::new();
        for (x, gap) in [(5.0, 0.495), (30.0, 0.5)] {
            v.extend(cap(c, x, 5.0, 10.0, 10.0, n));
            v.push(via(c.top_via, x + 0.6, 10.0, VIA));
            v.push(via(c.top_via, x + 0.6 + VIA + gap, 10.0, VIA));
        }
        v
    });

    // MIMTM.9.h2: one via fully on the plate and one straddling its edge, 0.495 apart.
    // The pitch rule is for the vias *on* the plate and the straddling one is not one of
    // them, so MIMTM.9 is clean - the straddler is MIMTM.4's crossing instead.
    write("MIMTM.9.h2", {
        let mut v = cap(c, 5.0, 5.0, 10.0, 10.0, n);
        v.push(via(c.top_via, 15.0 - 0.1, 10.0, VIA));
        v.push(via(c.top_via, 15.0 - 0.1 - 0.495 - VIA, 10.0, VIA));
        v
    });

    // MIMTM.10.h1: left, a Via3 on the bottom plate metal but outside the top plate, in
    // the 1.06 µm ring the virtual bottom plate is made of.  The manual forbids a Via3
    // touching the bottom plate and the ring is part of it; upstream reads only the metal
    // under FuseTop.  Right, a Via3 straddling the plate's edge, which both read as a
    // violation.
    write("MIMTM.10.h1", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, n);
        v.push(via(c.low_via, 11.0 + 0.4, 7.5, VIA));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, n));
        v.push(via(c.low_via, 36.0 - 0.1, 7.5, VIA));
        v
    });

    // MIMTM.11.h1: two capacitors on one bottom plate, 50 × 100 each - 10000 µm² together,
    // exactly the cap, clean.  Neither plate is near MIMTM.8b on its own.
    write("MIMTM.11.h1", shared_plate(c, 50.0, 50.0));

    // MIMTM.11.h2: the same pair at 50 × 100 and 50.0025 × 100 - 10000.25 µm² together,
    // over the cap, fires.  Each plate on its own is half of it, so MIMTM.8b stays clean:
    // what this rule adds is the sum.
    write("MIMTM.11.h2", shared_plate(c, 50.0, 50.0025));

    // MIMTM.11.h3: two 60 × 100 capacitors - 12000 µm² between them - on *separate* bottom
    // plates.  The total is over the cap but no single plate carries it, so nothing fires:
    // the sum is per bottom plate, not per layout.
    write("MIMTM.11.h3", {
        let mut v = cap(c, 5.0, 5.0, 60.0, 100.0, [BOT; 4]);
        v.extend(cap(c, 80.0, 5.0, 60.0, 100.0, [BOT; 4]));
        v
    });
}

/// The pattern that asks exactly where the 1.06 µm oversize ends, and the one that asks
/// what a spacing of nothing is.  Apart from `hardening` only so neither runs long.
fn edges(c: &Ctx, write: &dyn Fn(&str, Vec<GdsElement>)) {
    // MIMTM.2.h3: the rule is for the vias "within 1.06 µm oversize of FuseTop".  Left: a
    // via whose inner edge sits exactly on that oversize - it touches the virtual plate
    // and no more, so it is not within it, and the 0.395 µm of metal beyond it is not this
    // rule's business.  Right: a via 0.01 µm inside the oversize, held by the same 0.395,
    // which is within it and fires.
    write("MIMTM.2.h3", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, [BOT, 1.715, BOT, BOT]);
        v.push(via(c.top_via, 11.0 + BOT, 7.5, VIA));
        v.extend(cap(c, 30.0, 5.0, 6.0, 6.0, [BOT, 1.705, BOT, BOT]));
        v.push(via(c.top_via, 36.0 + BOT - 0.01, 7.5, VIA));
        v
    });

    // MIMTM.5.h2: a via on the bottom plate metal whose corner sits on the top plate's
    // corner.  The spacing between them is nothing, which is under 0.4.
    write("MIMTM.5.h2", {
        let mut v = cap(c, 5.0, 5.0, 6.0, 6.0, [BOT; 4]);
        v.push(via(c.top_via, 11.0, 11.0, VIA));
        v
    });
}
