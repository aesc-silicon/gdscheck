// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dual gate oxide: a good and a bad pattern for every rule in the `dualgate` deck.
//!
//! The marker says which devices run at the higher voltage, and most of the deck is about
//! what it has to cover: an active it touches at all, the poly on that active, and the
//! deep well under it.  So the base cell is one 6 V transistor - a gate on an active in an
//! N-well over a deep well - with the marker generously over the lot, and each fixture
//! pulls the marker in on one of them.
//!
//! The substrate tap is the exception the deck makes: DV.6 measures the marker against
//! COMP *less* the P+ active outside any N-well, so a tap needs no marker of its own.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/dualgate";

/// Active area, and the gate across it.
const COMP_L: f64 = 6.0;
const COMP_W: f64 = 3.0;
const GATE: f64 = 1.0;
/// How far the marker reaches past each thing it must cover - DV.1 asks 0.5 of the deep
/// well, DV.6 asks 0.24 of the active and DV.8 asks 0.4 of the poly.
const DV: f64 = 1.2;
/// The deep well's reach past the active, inside the marker.
const DN: f64 = 0.3;
/// The gate's overhang past the active.
const GATE_EXT: f64 = 0.5;

struct Ctx {
    dualgate: (i16, i16),
    dnwell: (i16, i16),
    nwell: (i16, i16),
    comp: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
}

struct Cell {
    x: f64,
    y: f64,
    /// How far past the marker's own edge each covered thing reaches out.  The marker is
    /// fixed and the covered layer grows toward it, not the other way round: the marker
    /// has to clear *all three*, so pulling its edge in on one account leaves it where the
    /// others put it and nothing changes.
    dn_out: f64,
    comp_out: f64,
    poly_out: f64,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            dn_out: 0.0,
            comp_out: 0.0,
            poly_out: 0.0,
        }
    }

    /// The marker, fixed at `DV` past the nominal device.
    fn marker(&self) -> (f64, f64, f64, f64) {
        let (x, y) = (self.x, self.y);
        (
            x - DV,
            y - DV - GATE_EXT,
            x + COMP_L + DV,
            y + COMP_W + DV + GATE_EXT,
        )
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let (cx1, cy1) = (x + COMP_L, y + COMP_W);
        let gx = x + (COMP_L - GATE) * 0.5;
        let (mx0, my0, mx1, my1) = self.marker();
        // Each covered thing reaches to its own margin inside the marker, or further out
        // when a fixture asks.
        let dn_x1 = if self.dn_out > 0.0 {
            mx1 - self.dn_out
        } else {
            cx1 + DN
        };
        let comp_x1 = if self.comp_out > 0.0 {
            mx1 - self.comp_out
        } else {
            cx1
        };
        let poly_y1 = if self.poly_out > 0.0 {
            my1 - self.poly_out
        } else {
            cy1 + GATE_EXT
        };
        vec![
            rect(c.dualgate, mx0, my0, mx1, my1),
            rect(c.dnwell, x - DN, y - DN, dn_x1, cy1 + DN),
            // An N-well over the active, so the P+ in it is a device and not a tap.
            rect(c.nwell, x - 0.2, y - 0.2, comp_x1 + 0.2, cy1 + 0.2),
            rect(c.comp, x, y, comp_x1, cy1),
            rect(c.pplus, x - 0.1, y - 0.1, comp_x1 + 0.1, cy1 + 0.1),
            rect(c.poly, gx, y - GATE_EXT, gx + GATE, poly_y1),
        ]
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        dualgate: layer(pdk, "dualgate"),
        dnwell: layer(pdk, "dnwell"),
        nwell: layer(pdk, "nwell"),
        comp: layer(pdk, "comp"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let base = Cell::at(o, o);
    for id in [
        "DV.1", "DV.2", "DV.3", "DV.5", "DV.6", "DV.7", "DV.8", "DV.9",
    ] {
        write(id, "good", base.draw(&c));
    }
    let dim = |f: &dyn Fn(&mut Cell)| {
        let mut cell = Cell::at(o, o);
        f(&mut cell);
        cell.draw(&c)
    };

    // DV.1 / DV.6 / DV.8: the deep well, the active and the poly each reaching to within
    // less than the marker owes them.
    write("DV.1", "bad", dim(&|k| k.dn_out = 0.495));
    write("DV.6", "bad", dim(&|k| k.comp_out = 0.235));
    write("DV.8", "bad", dim(&|k| k.poly_out = 0.395));

    // DV.2: a second marker 0.44 µm away, bare - nothing under it to cover.
    write("DV.2", "bad", {
        let mut v = base.draw(&c);
        let (_, my0, mx1, my1) = base.marker();
        v.push(rect(c.dualgate, mx1 + 0.435, my0, mx1 + 2.435, my1));
        v
    });

    // DV.3: an active outside the marker, within 0.24 µm of it.  It is a substrate tap -
    // P+ with no N-well under it - which is the one active the deck excuses the marker
    // from covering, so DV.6 has nothing to say about it lying outside.
    write("DV.3", "bad", {
        let mut v = base.draw(&c);
        let (_, my0, mx1, _) = base.marker();
        let x = mx1 + 0.235;
        v.push(rect(c.comp, x, my0 + 1.0, x + 2.0, my0 + 3.0));
        v.push(rect(c.pplus, x - 0.1, my0 + 0.9, x + 2.1, my0 + 3.1));
        v
    });

    // DV.5: a marker narrower than 0.7 µm, bare beside the cell.
    write("DV.5", "bad", {
        let mut v = base.draw(&c);
        let (_, my0, mx1, _) = base.marker();
        let x = mx1 + 2.0;
        v.push(rect(c.dualgate, x, my0, x + 0.695, my0 + 3.0));
        v
    });

    // DV.7: a marker that touches an active without covering it.  It has to be a marker
    // of its own: the cell's covers its device, and a marker that covers *any* active is
    // excused, so the straddling one would never be seen.  Falling short of covering also
    // falls short of DV.6's margin, which no drawing can avoid.
    write("DV.7", "bad", {
        let mut v = base.draw(&c);
        let (_, my0, mx1, _) = base.marker();
        let (sx, sy) = (mx1 + 2.0, my0 + 1.0);
        v.push(rect(c.dualgate, sx, sy, sx + 3.0, sy + 3.0));
        // An active straddling that marker's right edge, in a well so it is a device and
        // not the substrate tap the deck excuses.
        v.push(rect(c.comp, sx + 2.0, sy + 1.0, sx + 4.0, sy + 2.0));
        v.push(rect(c.nwell, sx + 1.8, sy + 0.8, sx + 4.2, sy + 2.2));
        v.push(rect(c.pplus, sx + 1.9, sy + 0.9, sx + 4.1, sy + 2.1));
        v
    });

    // DV.9: one N-well carrying a 6 V P-channel gate and a 3.3 V one at once - the marker
    // covers the first and not the second, which is what the rule forbids.
    write("DV.9", "bad", {
        let mut v = base.draw(&c);
        let (_, _, mx1, _) = base.marker();
        let (gx, gy) = (mx1 + 2.0, o);
        v.push(rect(c.nwell, o - 0.2, o - 0.2, gx + 3.2, o + COMP_W + 0.2));
        v.push(rect(c.comp, gx, gy, gx + 3.0, gy + COMP_W));
        v.push(rect(
            c.pplus,
            gx - 0.1,
            gy - 0.1,
            gx + 3.1,
            gy + COMP_W + 0.1,
        ));
        v.push(rect(
            c.poly,
            gx + 1.0,
            gy - 0.5,
            gx + 2.0,
            gy + COMP_W + 0.5,
        ));
        v
    });
}
