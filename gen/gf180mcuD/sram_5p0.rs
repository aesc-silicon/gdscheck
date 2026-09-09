// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SRAM at 5 V: a good and a bad pattern for every rule in the `sram_5p0` deck.
//!
//! The 5 V core is told from the 3.3 V one by a V5_XTOR marker over the SRAMCORE, not by
//! Dualgate, so that pair is what every fixture starts from.  Under it the deck wants
//! three separate arrangements - a transistor, a P+ active in an N-well, and an N+ active
//! in an LVPWELL inside a deep well - so the cell draws all three side by side and each
//! fixture moves one dimension of one of them.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/sram_5p0";

/// Source/drain overhang past the gate - S.DF.6 asks 0.32.
const OVERHANG: f64 = 1.5;
/// N-well's hold on the P+ active, and its clearance to the N+ active outside it - both
/// 0.45 in the deck.
const WELL_ENC: f64 = 0.7;
const WELL_GAP: f64 = 0.7;
/// The LVPWELL's hold on the N+ active inside the deep well, and its clearance to the P+
/// one - also 0.45 apiece.
const LVP_ENC: f64 = 0.7;
const LVP_GAP: f64 = 0.7;
/// Active's hold on a contact - S.CO.4 asks 0.04.
const CO: f64 = 0.22;
const CO_ENC: f64 = 0.2;
/// Field poly's clearance to an active - S.PL.5a and S.PL.5b both ask 0.12.
const POLY_GAP: f64 = 0.4;

struct Ctx {
    sramcore: (i16, i16),
    v5: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    contact: (i16, i16),
}

struct Cell {
    x: f64,
    y: f64,
    overhang: f64,
    well_enc: f64,
    well_gap: f64,
    lvp_enc: f64,
    lvp_gap: f64,
    co_enc: f64,
    poly_gap: f64,
    /// A gate over one active brought near a second one - S.PL.5b, which unlike S.PL.5a
    /// reads poly that *is* over an active.
    gate_gap: Option<f64>,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            overhang: OVERHANG,
            well_enc: WELL_ENC,
            well_gap: WELL_GAP,
            lvp_enc: LVP_ENC,
            lvp_gap: LVP_GAP,
            co_enc: CO_ENC,
            poly_gap: POLY_GAP,
            gate_gap: None,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let mut v = vec![
            rect(c.sramcore, x - 2.0, y - 2.0, x + 32.0, y + 16.0),
            rect(c.v5, x - 2.0, y - 2.0, x + 32.0, y + 16.0),
        ];

        // A transistor: a gate across an active, the active running past it either side.
        let (gx, gy) = (x + 3.0, y);
        let (gw, gh) = (1.0, 3.0);
        v.push(rect(
            c.comp,
            gx - self.overhang,
            y + 0.5,
            gx + gw + self.overhang,
            y + 0.5 + 2.0,
        ));
        v.push(rect(
            c.nplus,
            gx - self.overhang - 0.3,
            y + 0.2,
            gx + gw + self.overhang + 0.3,
            y + 2.8,
        ));
        v.push(rect(c.poly, gx, gy, gx + gw, gy + gh));

        // Field poly, clear of every active.
        v.push(rect(c.poly, x, y + 5.0, x + 1.0, y + 6.0));
        v.push(rect(
            c.comp,
            x + 1.0 + self.poly_gap,
            y + 5.0,
            x + 3.0 + self.poly_gap,
            y + 6.0,
        ));
        v.push(rect(
            c.nplus,
            x + 0.7 + self.poly_gap,
            y + 4.7,
            x + 3.3 + self.poly_gap,
            y + 6.3,
        ));

        // A P+ active held inside an N-well, with an N+ active outside it.
        let wx = x + 8.0;
        v.push(rect(c.nwell, wx, y, wx + 6.0, y + 6.0));
        let e = self.well_enc;
        v.push(rect(c.comp, wx + e, y + e, wx + 6.0 - e, y + 6.0 - e));
        v.push(rect(c.pplus, wx, y, wx + 6.0, y + 6.0));
        let nx = wx + 6.0 + self.well_gap;
        v.push(rect(c.comp, nx, y, nx + 2.0, y + 2.0));
        v.push(rect(c.nplus, nx - 0.2, y - 0.2, nx + 2.2, y + 2.2));

        // An N+ active held inside an LVPWELL in a deep well, with a P+ active beside it.
        let dx = x + 20.0;
        v.push(rect(c.dnwell, dx, y, dx + 10.0, y + 10.0));
        v.push(rect(c.lvpwell, dx + 1.0, y + 1.0, dx + 6.0, y + 6.0));
        let l = self.lvp_enc;
        v.push(rect(
            c.comp,
            dx + 1.0 + l,
            y + 1.0 + l,
            dx + 6.0 - l,
            y + 6.0 - l,
        ));
        v.push(rect(c.nplus, dx + 1.0, y + 1.0, dx + 6.0, y + 6.0));
        let px = dx + 6.0 + self.lvp_gap;
        v.push(rect(c.comp, px, y + 1.0, px + 2.0, y + 3.0));
        v.push(rect(c.pplus, px - 0.2, y + 0.8, px + 2.2, y + 3.2));

        // A contact on an active.
        let (cx, cy) = (x + 1.0, y + 9.0);
        v.push(rect(
            c.comp,
            cx - self.co_enc,
            cy - self.co_enc,
            cx + CO + self.co_enc,
            cy + CO + self.co_enc,
        ));
        v.push(rect(
            c.nplus,
            cx - self.co_enc - 0.2,
            cy - self.co_enc - 0.2,
            cx + CO + self.co_enc + 0.2,
            cy + CO + self.co_enc + 0.2,
        ));
        v.push(rect(c.contact, cx, cy, cx + CO, cy + CO));

        if let Some(g) = self.gate_gap {
            // A second active beside the transistor's gate: poly that is over an active,
            // so S.PL.5b sees it where S.PL.5a - which reads only field poly - does not.
            let sx = gx + gw + g;
            v.push(rect(c.comp, sx, gy, sx + 2.0, gy + 1.0));
            v.push(rect(c.nplus, sx - 0.2, gy - 0.2, sx + 2.2, gy + 1.2));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        sramcore: layer(pdk, "sramcore"),
        v5: layer(pdk, "v5_xtor"),
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
        contact: layer(pdk, "contact"),
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
        "S.DF.4c_MV",
        "S.DF.6_MV",
        "S.DF.7_MV",
        "S.DF.8_MV",
        "S.DF.16_MV",
        "S.PL.5a_MV",
        "S.PL.5b_MV",
        "S.CO.4_MV",
    ] {
        write(id, "good", base.draw(&c));
    }
    let dim = |f: &dyn Fn(&mut Cell)| {
        let mut cell = Cell::at(o, o);
        f(&mut cell);
        cell.draw(&c)
    };
    write("S.DF.4c_MV", "bad", dim(&|k| k.well_enc = 0.445));
    write("S.DF.6_MV", "bad", dim(&|k| k.overhang = 0.315));
    write("S.DF.7_MV", "bad", dim(&|k| k.lvp_gap = 0.445));
    write("S.DF.8_MV", "bad", dim(&|k| k.lvp_enc = 0.445));
    write("S.DF.16_MV", "bad", dim(&|k| k.well_gap = 0.445));
    write("S.PL.5a_MV", "bad", dim(&|k| k.poly_gap = 0.115));
    write("S.PL.5b_MV", "bad", dim(&|k| k.gate_gap = Some(0.115)));
    write("S.CO.4_MV", "bad", dim(&|k| k.co_enc = 0.035));
}
