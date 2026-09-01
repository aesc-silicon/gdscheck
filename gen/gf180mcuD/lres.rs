// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! N+ poly resistor: a good and a bad pattern for every rule in the `lres` deck.
//!
//! The N+ resistor is the P+ one with the other implant, and its deck says the same nine
//! things about it, so this file is `pres.rs` with Nplus in place of Pplus.  Keeping them
//! apart rather than sharing a generator is deliberate: the two decks are free to drift -
//! they already differ on which layer counts as unrelated poly - and a shared generator
//! would have to grow a flag for every such difference.
//!
//! Every fixture is built from one resistor, perturbed one rule at a time.  An N+ poly
//! resistor is Poly2 crossed with Nplus, marked by SAB and by RES_MK, so the base cell
//! carries all four and each rule moves one of them.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/lres";

/// Resistor body: `LEN` long and `WIDTH` wide, at least `LRES.1`'s 0.8 µm.
const LEN: f64 = 6.0;
const WIDTH: f64 = 1.0;
/// Nplus margin beyond the body (`LRES.5` asks 0.3) and SAB margin (`LRES.6` asks 0.28).
const PP: f64 = 0.4;
const SAB_W: f64 = 0.35;
/// Where SAB starts and ends along the body, leaving the contact heads salicided.
const SAB_IN: f64 = 1.4;
/// Contact side, and its clearance from SAB (`LRES.7` asks 0.22).
const CONT: f64 = 0.22;
const CONT_GAP: f64 = 0.5;

struct Ctx {
    poly: (i16, i16),
    nplus: (i16, i16),
    sab: (i16, i16),
    res_mk: (i16, i16),
    contact: (i16, i16),
    comp: (i16, i16),
}

/// One resistor with its origin at `(x, y)`, every margin overridable so a rule's bad
/// half can move exactly one of them and leave the rest legal.
struct Cell {
    x: f64,
    y: f64,
    width: f64,
    pp: f64,
    sab_w: f64,
    cont_gap: f64,
    /// How far the RES_MK reaches beyond the body. Negative cuts across it.
    mk: f64,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            width: WIDTH,
            pp: PP,
            sab_w: SAB_W,
            cont_gap: CONT_GAP,
            // Kept under LRES.4's 0.6 um so an unrelated Poly2 drawn close enough to
            // violate that rule still falls clear of the marking. Otherwise the marking's
            // own edge lands inside that Poly2 and LRES.9a fires alongside.
            mk: 0.3,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<gds21::GdsElement> {
        let (x, y, w) = (self.x, self.y, self.width);
        let mut v = vec![
            rect(c.poly, x, y, x + LEN, y + w),
            rect(
                c.nplus,
                x - self.pp,
                y - self.pp,
                x + LEN + self.pp,
                y + w + self.pp,
            ),
            // SAB covers the middle and overhangs in the width direction.
            rect(
                c.sab,
                x + SAB_IN,
                y - self.sab_w,
                x + LEN - SAB_IN,
                y + w + self.sab_w,
            ),
        ];
        // The RES_MK marking. A positive reach clears the body on every side, so no edge
        // of it falls inside the Poly2 and LRES.9a has nothing to measure.
        v.push(rect(
            c.res_mk,
            x - self.mk,
            y - self.mk,
            x + LEN + self.mk,
            y + w + self.mk,
        ));
        // A contact head at each end of the body, clear of the SAB.
        for cx in [
            x + SAB_IN - self.cont_gap - CONT,
            x + LEN - SAB_IN + self.cont_gap,
        ] {
            let cy = y + (w - CONT) * 0.5;
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v
    }
}

/// A RES_MK big enough for LRES.9b: over 15000 µm² with both sides over 80 µm.
fn big_marker(c: &Ctx, x: f64, y: f64) -> gds21::GdsElement {
    rect(c.res_mk, x, y, x + 130.0, y + 130.0)
}

/// Its neighbour, one grid step under the 15000 µm² bound so only one end of the pair
/// qualifies — as in the foundry's own case, and so the gap is one violation not two.
fn small_marker(c: &Ctx, x: f64, y: f64) -> gds21::GdsElement {
    rect(c.res_mk, x, y, x + 121.0, y + 123.0)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        nplus: layer(pdk, "nplus"),
        sab: layer(pdk, "sab"),
        res_mk: layer(pdk, "res_mk"),
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
    };
    let o = OFFSET;

    let write = |id: &str, polarity: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // Every rule's good half is the plain resistor; only LRES.9b needs its own, since a
    // marker that large is not part of one.
    let clean = || Cell::at(o, o).draw(&c);

    for id in [
        "LRES.1", "LRES.2", "LRES.3", "LRES.4", "LRES.5", "LRES.6", "LRES.7", "LRES.9a",
    ] {
        write(id, "good", clean());
    }
    write("LRES.9b", "good", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 20.1, o)]
    });

    // LRES.1: the body narrower than the 0.8 um minimum.
    write("LRES.1", "bad", {
        let mut cell = Cell::at(o, o);
        cell.width = 0.795;
        cell.draw(&c)
    });

    // LRES.2: two resistors closer than 0.4 um.
    write("LRES.2", "bad", {
        let mut v = Cell::at(o, o).draw(&c);
        v.extend(Cell::at(o, o + WIDTH + 0.395).draw(&c));
        v
    });

    // LRES.3: a COMP nearer the body than 0.6 um.
    write("LRES.3", "bad", {
        let mut v = clean();
        v.push(rect(c.comp, o + LEN + 0.595, o, o + LEN + 2.595, o + WIDTH));
        v
    });

    // LRES.4: unrelated Poly2 - no SAB on it - nearer than 0.6 um.
    write("LRES.4", "bad", {
        let mut v = clean();
        v.push(rect(c.poly, o + LEN + 0.595, o, o + LEN + 2.595, o + WIDTH));
        v
    });

    // LRES.5: Nplus reaching only 0.295 um beyond the body.
    write("LRES.5", "bad", {
        let mut cell = Cell::at(o, o);
        cell.pp = 0.295;
        cell.draw(&c)
    });

    // LRES.6: SAB overhanging the body by only 0.275 um in the width direction.
    write("LRES.6", "bad", {
        let mut cell = Cell::at(o, o);
        cell.sab_w = 0.275;
        cell.draw(&c)
    });

    // LRES.7: a contact head 0.215 um from the SAB.
    write("LRES.7", "bad", {
        let mut cell = Cell::at(o, o);
        cell.cont_gap = 0.215;
        cell.draw(&c)
    });

    // LRES.9a: a RES_MK that stops short, so its edge runs across the body instead of
    // following the resistor's own outline.
    write("LRES.9a", "bad", {
        let mut v = Cell::at(o, o).draw(&c);
        v.retain(|e| !matches!(e, gds21::GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.res_mk));
        v.push(rect(
            c.res_mk,
            o - 0.6,
            o - 0.6,
            o + LEN * 0.5,
            o + WIDTH + 0.6,
        ));
        v
    });

    // LRES.9b: the same two large markers, now 19.9 um apart.
    write("LRES.9b", "bad", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 19.9, o)]
    });
}
