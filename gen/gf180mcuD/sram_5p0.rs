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
use crate::helpers::{chamfered_tr, layer, library, rect, write_gz};
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

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Section 11.1 relaxes eight of chapter 7's rules for the cells SRAMCORE marks that also
// carry V5_XTOR.  Every layout here draws a Dualgate over the whole of it - a 5 V core is
// a 5 V core whether or not it is in an SRAM - and puts the SRAMCORE and V5_XTOR pair
// over one half only, so the half beside the marker answers to the base `_MV` rule at
// its larger value.  Findings: hardening/reports/gf180mcuD/sram_5p0.md; cases: the
// `hardening_sram_5p0` table of tests/gf180mcuD.rs.
//
// The generic classes (a bound on a bare layer, boxes that merge, notches, arrays, a
// shape far off) belong to the engine family and are not redrawn.

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A contact at `(x, y)` with `enc` of COMP round it; `cham` chamfers the COMP's top
/// right corner that far back, putting the 45° wall `(2·enc − cham)/√2` from the
/// contact's corner while both straight walls stay at `enc`.
fn co_cell(c: &Ctx, x: f64, y: f64, enc: f64, cham: f64) -> Vec<GdsElement> {
    let (x0, y0) = (x - enc, y - enc);
    let (x1, y1) = (x + CO + enc, y + CO + enc);
    let comp = if cham > 0.0 {
        chamfered_tr(c.comp, x0, y0, x1, y1, x1 + y1 - cham)
    } else {
        rect(c.comp, x0, y0, x1, y1)
    };
    vec![
        comp,
        rect(c.nplus, x0 - 0.2, y0 - 0.2, x1 + 0.2, y1 + 0.2),
        rect(c.contact, x, y, x + CO, y + CO),
    ]
}

/// An N-well holding a P+ active inset `enc`; `cham` chamfers the well's top right
/// corner, `over` runs the active out past the well's right edge instead.
fn well_cell(c: &Ctx, x: f64, y: f64, enc: f64, cham: f64, over: bool) -> Vec<GdsElement> {
    let (x1, y1) = (x + 4.0, y + 4.0);
    let nw = if cham > 0.0 {
        chamfered_tr(c.nwell, x, y, x1, y1, x1 + y1 - cham)
    } else {
        rect(c.nwell, x, y, x1, y1)
    };
    let px1 = if over { x1 + 0.5 } else { x1 - enc };
    vec![
        nw,
        rect(c.comp, x + enc, y + enc, px1, y1 - enc),
        rect(c.pplus, x + enc, y + enc, px1, y1 - enc),
    ]
}

/// An LVPWELL inside a deep N-well holding an N+ active inset `enc`; `cham` chamfers the
/// P-well's top right corner, `over` runs the active out past its right edge.
fn lvp_cell(c: &Ctx, x: f64, y: f64, enc: f64, cham: f64, over: bool) -> Vec<GdsElement> {
    let (x1, y1) = (x + 4.0, y + 4.0);
    let lvp = if cham > 0.0 {
        chamfered_tr(c.lvpwell, x, y, x1, y1, x1 + y1 - cham)
    } else {
        rect(c.lvpwell, x, y, x1, y1)
    };
    let nx1 = if over { x1 + 0.3 } else { x1 - enc };
    vec![
        rect(c.dnwell, x - 1.0, y - 1.0, x1 + 1.0, y1 + 1.0),
        lvp,
        rect(c.comp, x + enc, y + enc, nx1, y1 - enc),
        rect(c.nplus, x + enc, y + enc, nx1, y1 - enc),
    ]
}

/// An N+ active - COMP under Nplus, which is what `ncomp` is derived from.
fn ncomp(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![rect(c.comp, x0, y0, x1, y1), rect(c.nplus, x0, y0, x1, y1)]
}

/// A P+ active.
fn pcomp(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![rect(c.comp, x0, y0, x1, y1), rect(c.pplus, x0, y0, x1, y1)]
}

fn hardening(pdk: &PdkConfig) {
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
    let dualgate = layer(pdk, "dualgate");
    // The 5 V core: SRAMCORE and V5_XTOR together, under the Dualgate that covers the
    // whole layout.
    let core = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![rect(c.sramcore, x0, y0, x1, y1), rect(c.v5, x0, y0, x1, y1)]
    };
    let dg = |x0: f64, y0: f64, x1: f64, y1: f64| rect(dualgate, x0, y0, x1, y1);

    // --- S.CO.4_MV, "COMP overlap of contact 0.04", against chapter 7's CO.4 of 0.07:
    // the bound, the step past it, a 45° COMP corner 0.0283 from the contact's corner
    // (fires) and one 0.046 from it (clean), and the same 0.035 margin outside the core,
    // where CO.4 speaks instead.
    hwrite("S.CO.4_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 22.0, 12.0)];
        v.extend(core(9.0, 9.0, 17.5, 11.5));
        v.extend(co_cell(&c, 10.0, 10.0, 0.04, 0.0));
        v.extend(co_cell(&c, 12.0, 10.0, 0.035, 0.0));
        v.extend(co_cell(&c, 14.0, 10.0, 0.04, 0.04));
        v.extend(co_cell(&c, 16.0, 10.0, 0.04, 0.015));
        v.extend(co_cell(&c, 20.0, 10.0, 0.035, 0.0));
        v
    });

    // The crossing half: a contact the COMP's edge cuts is no margin at all, while one
    // outside the COMP with its edge on the COMP's is no enclosure either.
    hwrite("S.CO.4_MV.h2", {
        let mut v = vec![dg(8.0, 8.0, 16.0, 12.0)];
        v.extend(core(9.0, 9.0, 15.0, 11.5));
        v.push(rect(c.comp, 9.9, 9.8, 10.11, 10.42));
        v.push(rect(c.nplus, 9.7, 9.6, 10.31, 10.62));
        v.push(rect(c.comp, 12.5, 9.8, 13.0, 10.42));
        v.push(rect(c.nplus, 12.3, 9.6, 13.2, 10.62));
        for x in [10.0, 13.0] {
            v.push(rect(c.contact, x, 10.0, x + CO, 10.0 + CO));
        }
        v
    });

    // --- S.DF.4c_MV, "Min. (Nwell overlap of PCOMP) outside DNWELL 0.45", against
    // DF.4c's 0.6: the bound, the step past it, a 45° well corner 0.318 from the active's
    // corner (fires) and one 0.495 from it (clean), and the active running out of the
    // well.
    hwrite("S.DF.4c_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 40.0, 16.0)];
        v.extend(core(8.5, 8.5, 39.5, 15.5));
        v.extend(well_cell(&c, 10.0, 10.0, 0.45, 0.0, false));
        v.extend(well_cell(&c, 16.0, 10.0, 0.445, 0.0, false));
        v.extend(well_cell(&c, 22.0, 10.0, 0.45, 0.45, false));
        v.extend(well_cell(&c, 28.0, 10.0, 0.45, 0.2, false));
        v.extend(well_cell(&c, 34.0, 10.0, 0.45, 0.0, true));
        v
    });

    // --- S.DF.6_MV, "Min. COMP extend beyond gate 0.32", against DF.6's 0.4.  A gate
    // across an active, the active running past it either side: 0.32 (clean), 0.315
    // (fires), and the same overhang at 0.7 with the active's corner chamfered so that
    // the 45° wall passes 0.354 from the gate's wall (clean) and 0.212 from it (fires).
    hwrite("S.DF.6_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 40.0, 16.0)];
        v.extend(core(8.5, 8.5, 39.5, 15.5));
        for (x, over, cham) in [
            (10.0, 0.32, 0.0),
            (16.0, 0.315, 0.0),
            (22.0, 0.7, 0.2),
            (28.0, 0.7, 0.4),
        ] {
            let (cx0, cy0, cx1, cy1) = (x, 10.0, x + 3.0, 13.0);
            if cham > 0.0 {
                v.push(chamfered_tr(c.comp, cx0, cy0, cx1, cy1, cx1 + cy1 - cham));
            } else {
                v.push(rect(c.comp, cx0, cy0, cx1, cy1));
            }
            v.push(rect(c.nplus, cx0 - 0.3, cy0 - 0.3, cx1 + 0.3, cy1 + 0.3));
            v.push(rect(
                c.poly,
                cx1 - over - 0.4,
                cy0 - 0.3,
                cx1 - over,
                cy1 + 0.3,
            ));
        }
        v
    });

    // --- S.DF.7_MV, "Min. (LVPWELL Spacer to PCOMP) inside DNWELL 0.45", against DF.7's
    // 0.6: the bound, the step past it, a corner-to-corner pair at 0.4525 (clean) and one
    // at 0.4384 (fires), and a P+ active abutting the P-well, which is a space of
    // nothing.
    hwrite("S.DF.7_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 44.0, 18.0)];
        v.extend(core(8.5, 8.5, 43.5, 17.5));
        for x in [10.0, 17.0, 24.0, 31.0, 38.0] {
            v.push(rect(c.dnwell, x - 1.0, 9.0, x + 4.0, 16.0));
            v.push(rect(c.lvpwell, x, 10.0, x + 3.0, 13.0));
        }
        v.extend(pcomp(&c, 13.45, 10.0, 13.95, 11.0));
        v.extend(pcomp(&c, 20.445, 10.0, 20.945, 11.0));
        v.extend(pcomp(&c, 27.32, 13.32, 27.82, 14.32));
        v.extend(pcomp(&c, 34.31, 13.31, 34.81, 14.31));
        v.extend(pcomp(&c, 41.0, 10.0, 41.5, 11.0));
        v
    });

    // --- S.DF.8_MV, "Min. (LVPWELL overlap of NCOMP) Inside DNWELL 0.45", against DF.8's
    // 0.6: the bound, the step past it, a 45° P-well corner at 0.318 (fires) and at 0.495
    // (clean), and the active running out of the P-well.
    hwrite("S.DF.8_MV.h1", {
        let mut v = vec![dg(6.0, 6.0, 42.0, 18.0)];
        v.extend(core(6.5, 6.5, 41.5, 17.5));
        v.extend(lvp_cell(&c, 10.0, 10.0, 0.45, 0.0, false));
        v.extend(lvp_cell(&c, 17.0, 10.0, 0.445, 0.0, false));
        v.extend(lvp_cell(&c, 24.0, 10.0, 0.45, 0.45, false));
        v.extend(lvp_cell(&c, 31.0, 10.0, 0.45, 0.2, false));
        v.extend(lvp_cell(&c, 38.0, 10.0, 0.45, 0.0, true));
        v
    });

    // S.DF.8_MV in a 3.3 V core.  The deck derives its N+ active from the bare SRAMCORE,
    // not from the 5 V class, so a core with neither V5_XTOR nor Dualgate is measured by
    // the 5 V value - which is *stricter* than the 0.43 chapter 7 gives a 3.3 V SRAM.
    // Both wells hold the active by 0.44: legal at 3.3 V, short of 11.1's 0.45.
    hwrite("S.DF.8_MV.h2", {
        let mut v = vec![rect(c.sramcore, 8.5, 8.5, 22.0, 15.5)];
        v.extend(lvp_cell(&c, 10.0, 10.0, 0.44, 0.0, false));
        v.extend(lvp_cell(&c, 17.0, 10.0, 0.46, 0.0, false));
        v
    });

    // --- S.DF.16_MV, "Min. space from (Nwell outside DNWELL) to (NCOMP outside Nwell and
    // DNWELL) 0.45", against DF.16's 0.6: the bound, the step past it, a corner pair at
    // 0.4525 (clean) and one at 0.4384 (fires), and an active abutting the well.
    hwrite("S.DF.16_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 44.0, 16.0)];
        v.extend(core(8.5, 8.5, 43.5, 15.5));
        for x in [10.0, 17.0, 24.0, 31.0, 38.0] {
            v.push(rect(c.nwell, x, 10.0, x + 3.0, 13.0));
        }
        v.extend(ncomp(&c, 13.45, 10.0, 14.45, 11.0));
        v.extend(ncomp(&c, 20.445, 10.0, 21.445, 11.0));
        v.extend(ncomp(&c, 27.32, 13.32, 28.32, 14.32));
        v.extend(ncomp(&c, 34.31, 13.31, 35.31, 14.31));
        v.extend(ncomp(&c, 41.0, 10.0, 42.0, 11.0));
        v
    });

    // --- S.PL.5a_MV, "Space from field Poly2 to unrelated COMP / Spacer from field Poly2
    // to Guard-ring 0.12", against PL.5a's 0.3.  Field Poly2 is poly that lies over no
    // active: the bound, the step past it, a corner pair at 0.1216 (clean) and one at
    // 0.1131 (fires), a strip whose corner touches the active's corner, and one whose
    // wall lies on the active's - both spaces of nothing.
    hwrite("S.PL.5a_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 46.0, 16.0)];
        v.extend(core(8.5, 8.5, 45.5, 15.5));
        for (x, gx, gy) in [
            (10.0, 0.12, 0.0),
            (16.0, 0.115, 0.0),
            (22.0, 0.086, 0.086),
            (28.0, 0.08, 0.08),
            (34.0, 0.0, 0.0),
        ] {
            v.push(rect(c.poly, x, 10.0, x + 1.0, 11.0));
            v.extend(ncomp(&c, x + 1.0 + gx, 11.0 + gy, x + 3.0 + gx, 12.0 + gy));
        }
        v.push(rect(c.poly, 40.0, 10.0, 41.0, 11.0));
        v.extend(ncomp(&c, 41.0, 10.0, 43.0, 11.0));
        v
    });

    // --- S.PL.5b_MV, "Space from field Poly2 to related COMP 0.12", against PL.5b's 0.3.
    // (a) a U-shaped active with the gate across its left arm, the poly turning into the
    // slot and stopping 0.115 from the right arm - the same active the gate belongs to,
    // so the related COMP.  (b) the same at 0.12.  (c) a gate over one active whose field
    // stretch passes 0.115 from a second: the poly is not field Poly2 as a whole, so
    // S.PL.5a's layer has already let it go, and a gap under 0.12 between poly and an
    // active is what the pair of rules is for.
    hwrite("S.PL.5b_MV.h1", {
        let mut v = vec![dg(8.0, 8.0, 34.0, 18.0)];
        v.extend(core(8.5, 8.5, 33.5, 17.5));
        for (x, gap) in [(10.0, 0.115), (18.0, 0.12)] {
            v.push(rect(c.comp, x, 10.0, x + 3.4, 10.8));
            v.push(rect(c.comp, x, 10.8, x + 1.2, 13.0));
            v.push(rect(c.comp, x + 2.6, 10.8, x + 3.4, 13.0));
            v.push(rect(c.nplus, x - 0.3, 9.7, x + 3.7, 13.3));
            v.push(rect(c.poly, x + 0.4, 9.7, x + 0.8, 13.3));
            v.push(rect(c.poly, x + 0.8, 12.0, x + 2.6 - gap, 12.4));
        }
        v.push(rect(c.comp, 26.0, 10.0, 28.0, 11.0));
        v.push(rect(c.nplus, 25.7, 9.7, 28.3, 11.3));
        v.push(rect(c.poly, 26.5, 9.7, 26.9, 13.0));
        v.extend(ncomp(&c, 27.015, 12.0, 29.0, 13.0));
        v
    });

    // --- Which cells the section speaks for.  11.1 is for the cores "with marking layer
    // V5_XTOR": (a) a core wholly under V5_XTOR, (b) a core the marker covers in part,
    // (c) a core V5_XTOR only abuts, (d) a bare core, (e) no core at all.  Every one holds
    // the same 0.035 contact margin, and (e) answers to chapter 7's CO.4 of 0.07.
    hwrite("S.CO.4_MV.h3", {
        let mut v = vec![dg(8.0, 8.0, 38.0, 14.0)];
        for (i, x) in [10.0, 14.0, 24.0, 28.0, 32.0].into_iter().enumerate() {
            if i < 4 {
                v.push(rect(c.sramcore, x, 10.0, x + 3.0, 13.0));
            }
            match i {
                0 => v.push(rect(c.v5, x - 0.2, 9.8, x + 3.2, 13.2)),
                1 => v.push(rect(c.v5, x - 0.2, 9.8, x + 1.5, 13.2)),
                2 => v.push(rect(c.v5, x + 3.0, 10.0, x + 4.0, 13.0)),
                _ => {}
            }
            v.extend(co_cell(&c, x + 1.0, 11.0, 0.035, 0.0));
        }
        v
    });

    // --- The tile lines: three 0.035 contact margins straddling x = 20, x = 42 and
    // y = 20, and an S.DF.16 gap of 0.445 straddling x = 40.
    hwrite("S.CO.4_MV.h4", {
        let mut v = vec![dg(8.0, 8.0, 46.0, 24.0)];
        v.extend(core(8.5, 8.5, 45.5, 23.5));
        for (x, y) in [(19.9, 10.0), (41.9, 10.0), (10.0, 19.9)] {
            v.extend(co_cell(&c, x, y, 0.035, 0.0));
        }
        v.push(rect(c.nwell, 36.0, 14.0, 39.8, 17.0));
        v.extend(ncomp(&c, 40.245, 14.0, 41.0, 17.0));
        v
    });
}
