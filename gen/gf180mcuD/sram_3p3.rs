// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SRAM at 3.3 V: a good and a bad pattern for every rule in the `sram_3p3` deck.
//!
//! These are the ordinary well and contact rules again, tightened for the memory core and
//! applying only inside the SRAMCORE marker - so the cell is that marker with the smallest
//! arrangement each rule needs under it, and nothing outside.  The deck is the low-voltage
//! half of the pair, told from the 5 V one by the absence of Dualgate, so no fixture here
//! draws any.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/sram_3p3";

/// The N-well, and how far it holds the P+ active inside it - S.DF.4c asks 0.4.
const WELL: f64 = 6.0;
const WELL_ENC: f64 = 0.6;
/// How far the N+ active outside the well stays clear of it - S.DF.16 asks 0.4.
const NW_GAP: f64 = 0.6;
/// Contact side, and the margins around it: poly 0.04, active 0.03, Metal1 0.04.
const CO: f64 = 0.22;
const POLY_ENC: f64 = 0.2;
const COMP_ENC: f64 = 0.2;
const M1_ENC: f64 = 0.2;

struct Ctx {
    sramcore: (i16, i16),
    nwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
}

struct Cell {
    x: f64,
    y: f64,
    well_enc: f64,
    nw_gap: f64,
    poly_enc: f64,
    comp_enc: f64,
    m1_enc: f64,
    /// Metal1's width, which S.M1.1 bounds at 0.22.
    m1_w: f64,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            well_enc: WELL_ENC,
            nw_gap: NW_GAP,
            poly_enc: POLY_ENC,
            comp_enc: COMP_ENC,
            m1_enc: M1_ENC,
            m1_w: 0.6,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        // The core marker over everything the deck is allowed to look at.
        let mut v = vec![rect(c.sramcore, x - 1.0, y - 1.0, x + 14.0, y + 12.0)];
        // An N-well holding a P+ active, and an N+ active outside it.
        v.push(rect(c.nwell, x, y, x + WELL, y + WELL));
        let e = self.well_enc;
        v.push(rect(c.comp, x + e, y + e, x + WELL - e, y + WELL - e));
        v.push(rect(c.pplus, x, y, x + WELL, y + WELL));
        let nx = x + WELL + self.nw_gap;
        v.push(rect(c.comp, nx, y, nx + 2.0, y + 2.0));
        v.push(rect(c.nplus, nx - 0.2, y - 0.2, nx + 2.2, y + 2.2));
        // A contact on poly, and one on active, each with its metal.
        let (px, py) = (x, y + 8.0);
        v.push(rect(
            c.poly,
            px - self.poly_enc,
            py - self.poly_enc,
            px + CO + self.poly_enc,
            py + CO + self.poly_enc,
        ));
        let (cx, cy) = (x + 4.0, y + 8.0);
        v.push(rect(
            c.comp,
            cx - self.comp_enc,
            cy - self.comp_enc,
            cx + CO + self.comp_enc,
            cy + CO + self.comp_enc,
        ));
        v.push(rect(
            c.nplus,
            cx - self.comp_enc - 0.2,
            cy - self.comp_enc - 0.2,
            cx + CO + self.comp_enc + 0.2,
            cy + CO + self.comp_enc + 0.2,
        ));
        for (bx, by) in [(px, py), (cx, cy)] {
            v.push(rect(c.contact, bx, by, bx + CO, by + CO));
            let m = self.m1_enc;
            let extra = (self.m1_w - (CO + 2.0 * m)).max(0.0);
            v.push(rect(
                c.metal1,
                bx - m,
                by - m,
                bx + CO + m + extra,
                by + CO + m,
            ));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        sramcore: layer(pdk, "sramcore"),
        nwell: layer(pdk, "nwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
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
        "S.DF.4c_LV",
        "S.DF.16_LV",
        "S.CO.3_LV",
        "S.CO.4_LV",
        "S.CO.6_ii_LV",
        "S.M1.1_LV",
    ] {
        write(id, "good", base.draw(&c));
    }
    let dim = |f: &dyn Fn(&mut Cell)| {
        let mut cell = Cell::at(o, o);
        f(&mut cell);
        cell.draw(&c)
    };
    write("S.DF.4c_LV", "bad", dim(&|k| k.well_enc = 0.395));
    write("S.DF.16_LV", "bad", dim(&|k| k.nw_gap = 0.395));
    write("S.CO.3_LV", "bad", dim(&|k| k.poly_enc = 0.035));
    write("S.CO.4_LV", "bad", dim(&|k| k.comp_enc = 0.025));
    // S.CO.6_ii is the adjacent-edge form, not a plain margin: where Metal1 encloses the
    // contact by under 0.02 µm on one side, the sides beside it must reach 0.04.  So one
    // side is drawn under the trigger and its neighbour under the margin, and the other
    // two stay generous - a uniformly thin metal would be a width violation instead.
    write("S.CO.6_ii_LV", "bad", {
        let mut v = base.draw(&c);
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.metal1));
        let (px, py) = (o, o + 8.0);
        v.push(rect(
            c.metal1,
            px - 0.015,
            py - 0.035,
            px + CO + 0.5,
            py + CO + 0.5,
        ));
        let (cx, cy) = (o + 4.0, o + 8.0);
        v.push(rect(
            c.metal1,
            cx - M1_ENC,
            cy - M1_ENC,
            cx + CO + M1_ENC,
            cy + CO + M1_ENC,
        ));
        v
    });
    // S.M1.1: Metal1 narrower than 0.22 µm.  The contact margins stay legal, so only the
    // width is in question - the metal is drawn as a track past the contact instead.
    write("S.M1.1_LV", "bad", {
        let mut v = base.draw(&c);
        v.push(rect(c.metal1, o + 8.0, o + 8.0, o + 8.0 + 0.215, o + 10.0));
        v
    });
}
