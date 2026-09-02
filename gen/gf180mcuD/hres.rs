// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! High-sheet poly resistor: a good and a bad pattern for every rule in the `hres` deck.
//!
//! The third resistor here, and the one with an extra layer: where the P+ and N+ bodies
//! are recognised by their implant, this one carries a RESISTOR marker as well, and half
//! its rules are about that marker rather than about the poly.  So the base cell is
//! `pres`'s with the marker added, and rules that used to move the implant move the
//! marker instead.
//!
//! Every fixture is one resistor, perturbed one rule at a time.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/hres";

/// Body length, and a width over `HRES.2`'s 1.0 µm minimum.
const LEN: f64 = 8.0;
const WIDTH: f64 = 1.2;
/// How far each marker and implant reaches past the body.  RESISTOR's 0.5 clears
/// `HRES.4`'s 0.4, SAB's 0.4 clears `HRES.9`'s 0.28, and RES_MK sits outside both so no
/// edge of it falls in the poly, which is what `HRES.12a` looks for.
const PP: f64 = 0.5;
const RES: f64 = 0.5;
const SAB_W: f64 = 0.4;
const MK: f64 = 0.7;
/// Where the SAB starts along the body, leaving the contact heads salicided.
const SAB_IN: f64 = 2.0;
/// Contact side, and its clearance from the SAB - `HRES.8` asks 0.22.
const CONT: f64 = 0.22;
const CONT_GAP: f64 = 0.5;

struct Ctx {
    poly: (i16, i16),
    pplus: (i16, i16),
    sab: (i16, i16),
    res_mk: (i16, i16),
    resistor: (i16, i16),
    contact: (i16, i16),
    comp: (i16, i16),
}

/// One resistor, every margin overridable so a rule's bad half moves exactly one of them.
struct Cell {
    /// How far the implant reaches past the body.  A fixture that pulls the block in
    /// moves this with it, so the 0.1 µm overlap HRES.10 fixes is left where it was.
    pp: f64,
    x: f64,
    y: f64,
    width: f64,
    res: f64,
    sab_w: f64,
    cont_gap: f64,
    mk: f64,
    /// How far the P+ implant's left edge sits from the left contact, for `HRES.7`.
    pp_left: Option<f64>,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            pp: PP,
            x,
            y,
            width: WIDTH,
            res: RES,
            sab_w: SAB_W,
            cont_gap: CONT_GAP,
            mk: MK,
            pp_left: None,
        }
    }

    fn contacts(&self) -> [f64; 2] {
        [
            self.x + SAB_IN - self.cont_gap - CONT,
            self.x + LEN - SAB_IN + self.cont_gap,
        ]
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y, w) = (self.x, self.y, self.width);
        let pp_x0 = match self.pp_left {
            Some(margin) => self.contacts()[0] - margin,
            None => x - self.pp,
        };
        let mut v = vec![
            rect(c.poly, x, y, x + LEN, y + w),
            rect(
                c.pplus,
                pp_x0,
                y - self.pp,
                x + LEN + self.pp,
                y + w + self.pp,
            ),
            rect(
                c.sab,
                x + SAB_IN,
                y - self.sab_w,
                x + LEN - SAB_IN,
                y + w + self.sab_w,
            ),
            rect(
                c.resistor,
                x - self.res,
                y - self.res,
                x + LEN + self.res,
                y + w + self.res,
            ),
            rect(
                c.res_mk,
                x - self.mk,
                y - self.mk,
                x + LEN + self.mk,
                y + w + self.mk,
            ),
        ];
        for cx in self.contacts() {
            let cy = y + (w - CONT) * 0.5;
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v
    }
}

/// A RES_MK big enough for HRES.12b: over 15000 µm² with both sides over 80 µm.
fn big_marker(c: &Ctx, x: f64, y: f64) -> GdsElement {
    rect(c.res_mk, x, y, x + 130.0, y + 130.0)
}

/// Its neighbour, under the area bound, so only one end of the pair qualifies and the gap
/// is one violation rather than two.
fn small_marker(c: &Ctx, x: f64, y: f64) -> GdsElement {
    rect(c.res_mk, x, y, x + 121.0, y + 123.0)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        pplus: layer(pdk, "pplus"),
        sab: layer(pdk, "sab"),
        res_mk: layer(pdk, "res_mk"),
        resistor: layer(pdk, "resistor"),
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);

    // HRES.10 fixes how far the implant reaches past the block at 0.1 µm, and the clean
    // cell already sits there - the implant runs 0.5 µm past the poly and the block 0.4.
    // Pulling the block in widens the overlap without moving the implant.
    let overlap = |sab_w: f64| {
        let mut cell = Cell::at(o, o);
        cell.sab_w = sab_w;
        cell.draw(&c)
    };

    write("HRES.10", "bad", overlap(0.395));

    for id in [
        "HRES.1", "HRES.2", "HRES.3", "HRES.4", "HRES.5", "HRES.6", "HRES.7", "HRES.8", "HRES.9",
        "HRES.10", "HRES.12a",
    ] {
        write(id, "good", clean());
    }
    write("HRES.12b", "good", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 20.1, o)]
    });

    // HRES.1: two devices whose RESISTOR markers come within 0.4 µm.  Their bodies stay a
    // marker's reach further apart than that, so the poly spacing has nothing to say.
    write("HRES.1", "bad", {
        let mut v = clean();
        v.extend(Cell::at(o, o + WIDTH + 2.0 * RES + 0.395).draw(&c));
        v
    });

    // HRES.2: a body under the 1.0 µm minimum.
    write("HRES.2", "bad", {
        let mut cell = Cell::at(o, o);
        cell.width = 0.995;
        cell.draw(&c)
    });

    // HRES.3: two bodies within 0.4 µm.  Their markers overlap and merge, so the marker
    // spacing is not in play.
    write("HRES.3", "bad", {
        let mut v = clean();
        v.extend(Cell::at(o, o + WIDTH + 0.395).draw(&c));
        v
    });

    // HRES.4: the RESISTOR marker reaching only 0.395 µm past the body.
    write("HRES.4", "bad", {
        let mut cell = Cell::at(o, o);
        cell.res = 0.395;
        cell.draw(&c)
    });

    // HRES.5: unrelated Poly2 - no SAB, no marker - within 0.3 µm of the marker.
    write("HRES.5", "bad", {
        let mut v = clean();
        let x = o + LEN + RES + 0.295;
        v.push(rect(c.poly, x, o, x + 2.0, o + WIDTH));
        v
    });

    // HRES.6: a COMP within 0.3 µm of the marker.
    write("HRES.6", "bad", {
        let mut v = clean();
        let x = o + LEN + RES + 0.295;
        v.push(rect(c.comp, x, o, x + 2.0, o + WIDTH));
        v
    });

    // HRES.7: the P+ implant enclosing a contact head by only 0.195 µm.
    write("HRES.7", "bad", {
        let mut cell = Cell::at(o, o);
        cell.pp_left = Some(0.195);
        cell.draw(&c)
    });

    // HRES.8: a contact head 0.215 µm from the SAB.
    write("HRES.8", "bad", {
        let mut cell = Cell::at(o, o);
        cell.cont_gap = 0.215;
        cell.draw(&c)
    });

    // HRES.9: the SAB overhanging the body by only 0.275 µm in the width direction.  The
    // implant comes in with it: leaving the implant where it was would widen its reach
    // past the block, which is HRES.10's business and not this rule's.
    write("HRES.9", "bad", {
        let mut cell = Cell::at(o, o);
        cell.sab_w = 0.275;
        cell.pp = 0.375;
        cell.draw(&c)
    });

    // HRES.12a: a RES_MK that stops short, so its edge runs across the body instead of
    // following the resistor's outline.
    write("HRES.12a", "bad", {
        let mut v = clean();
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.res_mk));
        v.push(rect(
            c.res_mk,
            o - MK,
            o - MK,
            o + LEN * 0.5,
            o + WIDTH + MK,
        ));
        v
    });

    // HRES.12b: the same two large markers, now under 20 µm apart.
    write("HRES.12b", "bad", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 19.9, o)]
    });
}
