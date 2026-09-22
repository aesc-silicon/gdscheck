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
use crate::helpers::{chamfered_tr, layer, library, rect, write_gz};
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

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Section 11.2 is a *relaxation*: six of chapter 7's rules take a smaller value inside
// the SRAMCORE marker, and the cells it marks are the ones "without marking layer
// V5_XTOR".  So the layouts here carry both halves - the same geometry under the marker
// and beside it - and the half outside has to answer to the base rule instead.  The
// findings are in hardening/reports/gf180mcuD/sram_3p3.md and the cases in the
// `hardening_sram_3p3` table of tests/gf180mcuD.rs.
//
// The generic classes (a bound on a bare layer, boxes that merge, notches, arrays, a
// shape far off) belong to the engine family and are not redrawn: what is drawn here is
// what only this deck has - the marker, the voltage class it selects, and the six values.

/// Metal1's margin round a contact wherever the fixture is not asking about it: wide
/// enough that S.CO.6_ii never triggers and the track is never under S.M1.1's 0.22.
const H_M1: f64 = 0.25;

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A contact at `(x, y)` with `enc` of `cover` round it and Metal1 clear of both bounds.
/// `cham` chamfers the cover's top right corner that far back, so the 45° wall passes
/// `(2·enc − cham)/√2` from the contact's corner while both straight walls stay at `enc`.
fn stack(c: &Ctx, cover: (i16, i16), x: f64, y: f64, enc: f64, cham: f64) -> Vec<GdsElement> {
    let (x0, y0) = (x - enc, y - enc);
    let (x1, y1) = (x + CO + enc, y + CO + enc);
    let cov = if cham > 0.0 {
        chamfered_tr(cover, x0, y0, x1, y1, x1 + y1 - cham)
    } else {
        rect(cover, x0, y0, x1, y1)
    };
    vec![
        cov,
        rect(c.contact, x, y, x + CO, y + CO),
        rect(c.metal1, x - H_M1, y - H_M1, x + CO + H_M1, y + CO + H_M1),
    ]
}

/// An N-well holding a P+ active inset `enc` on every side; `cham` chamfers the well's
/// top right corner, `over` runs the active out past the well's right edge instead.
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

/// An N+ active - COMP under Nplus, which is what `ncomp` is derived from.
fn ncomp(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![rect(c.comp, x0, y0, x1, y1), rect(c.nplus, x0, y0, x1, y1)]
}

fn hardening(pdk: &PdkConfig) {
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
    let dualgate = layer(pdk, "dualgate");
    let v5 = layer(pdk, "v5_xtor");
    let mark = |x0: f64, y0: f64, x1: f64, y1: f64| rect(c.sramcore, x0, y0, x1, y1);

    // --- S.CO.3_LV, "Poly2 overlap of contact 0.04", against chapter 7's CO.3 of 0.07.
    // Two contacts under the marker at the bound and one step past it, and the same two
    // beside it, where only the base rule may speak.
    hwrite("S.CO.3_LV.h1", {
        let mut v = vec![mark(9.0, 9.0, 13.5, 11.5)];
        v.extend(stack(&c, c.poly, 10.0, 10.0, 0.04, 0.0));
        v.extend(stack(&c, c.poly, 12.0, 10.0, 0.035, 0.0));
        v.extend(stack(&c, c.poly, 16.0, 10.0, 0.04, 0.0));
        v.extend(stack(&c, c.poly, 18.0, 10.0, 0.075, 0.0));
        v
    });

    // The same margin carried by a 45° wall: both straight walls at 0.04, the chamfer
    // 0.0212 from the contact's corner (fires) and 0.0424 from it (clean).
    hwrite("S.CO.3_LV.h2", {
        let mut v = vec![mark(9.0, 9.0, 15.5, 11.5)];
        v.extend(stack(&c, c.poly, 10.0, 10.0, 0.04, 0.0));
        v.extend(stack(&c, c.poly, 12.0, 10.0, 0.04, 0.05));
        v.extend(stack(&c, c.poly, 14.0, 10.0, 0.04, 0.02));
        v
    });

    // --- S.CO.4_LV, "COMP overlap of contact 0.03", against CO.4's 0.07: the bound, the
    // step past it, and the same two chamfers at 0.0212 and 0.0318.
    hwrite("S.CO.4_LV.h1", {
        let mut v = vec![mark(9.0, 9.0, 17.5, 11.5)];
        v.extend(stack(&c, c.comp, 10.0, 10.0, 0.03, 0.0));
        v.extend(stack(&c, c.comp, 12.0, 10.0, 0.025, 0.0));
        v.extend(stack(&c, c.comp, 14.0, 10.0, 0.03, 0.03));
        v.extend(stack(&c, c.comp, 16.0, 10.0, 0.03, 0.015));
        v
    });

    // The crossing half both rules carry: a contact the covering layer's edge cuts is no
    // margin at all, and a contact outside that layer with its edge on the layer's is no
    // enclosure either.
    hwrite("S.CO.3_LV.h3", {
        let mut v = vec![mark(9.0, 9.0, 21.0, 11.5)];
        for (x, cover) in [(10.0, c.poly), (16.0, c.comp)] {
            v.push(rect(cover, x - 0.1, 9.8, x + 0.11, 10.42));
        }
        for (x, cover) in [(13.0, c.poly), (19.0, c.comp)] {
            v.push(rect(cover, x - 0.5, 9.8, x, 10.42));
        }
        for x in [10.0, 13.0, 16.0, 19.0] {
            v.push(rect(c.contact, x, 10.0, x + CO, 10.0 + CO));
            v.push(rect(
                c.metal1,
                x - H_M1,
                10.0 - H_M1,
                x + CO + H_M1,
                10.0 + CO + H_M1,
            ));
        }
        v
    });

    // --- S.CO.6_ii_LV.  The manual: "If Metal1 overlaps contact by < 0.04 µm on one
    // side, adjacent metal1 edges Overlap - 0.02".  The trigger is the same 0.04 as
    // chapter 7's CO.6 II and it is the *adjacent* margin that drops from 0.06 to 0.02.
    // (a) 0.035 triggers, 0.02 beside it: clean.  (b) 0.035 triggers, 0.015 beside it:
    // fires.  (c) 0.04 does not trigger, so 0.015 beside it is clean.  (d) 0.015 on one
    // side with 0.03 beside it: triggered, and 0.03 is over the value - clean.
    hwrite("S.CO.6_ii_LV.h1", {
        let mut v = vec![mark(9.0, 9.0, 17.5, 11.5)];
        for (x, left, bot) in [
            (10.0, 0.035, 0.02),
            (12.0, 0.035, 0.015),
            (14.0, 0.04, 0.015),
            (16.0, 0.015, 0.03),
        ] {
            v.push(rect(c.contact, x, 10.0, x + CO, 10.0 + CO));
            v.push(rect(c.metal1, x - left, 10.0 - bot, x + CO + 0.2, 10.42));
        }
        v
    });

    // --- S.M1.1_LV, "Width 0.22", against M1.1's 0.23: a track at the bound, one step
    // under it and one step over it inside the marker, and the step under it outside,
    // where M1.1 speaks.
    hwrite("S.M1.1_LV.h1", {
        vec![
            mark(9.0, 9.0, 13.5, 12.5),
            rect(c.metal1, 10.0, 10.0, 10.22, 12.0),
            rect(c.metal1, 11.0, 10.0, 11.215, 12.0),
            rect(c.metal1, 12.0, 10.0, 12.225, 12.0),
            rect(c.metal1, 16.0, 10.0, 16.215, 12.0),
        ]
    });

    // --- S.DF.4c_LV, "Min. (Nwell overlap of PCOMP) outside DNWELL 0.4", against DF.4c's
    // 0.43: the bound, the step past it, a 45° well corner 0.2828 from the active's
    // corner (fires) and one 0.4243 from it (clean), and the active running out of the
    // well, which is no enclosure at all.
    hwrite("S.DF.4c_LV.h1", {
        let mut v = vec![mark(8.0, 8.0, 40.0, 16.0)];
        v.extend(well_cell(&c, 10.0, 10.0, 0.4, 0.0, false));
        v.extend(well_cell(&c, 16.0, 10.0, 0.395, 0.0, false));
        v.extend(well_cell(&c, 22.0, 10.0, 0.4, 0.4, false));
        v.extend(well_cell(&c, 28.0, 10.0, 0.4, 0.2, false));
        v.extend(well_cell(&c, 34.0, 10.0, 0.4, 0.0, true));
        v
    });

    // --- S.DF.16_LV, "Min. space from (Nwell outside DNWELL) to (NCOMP outside Nwell and
    // DNWELL) 0.4", against DF.16's 0.43: the bound, the step past it, a corner-to-corner
    // pair at 0.4243 (clean) and one at 0.396 (fires), and an active that abuts the well,
    // which is a space of nothing.
    hwrite("S.DF.16_LV.h1", {
        let mut v = vec![mark(8.0, 8.0, 44.0, 16.0)];
        for x in [10.0, 17.0, 24.0, 31.0, 38.0] {
            v.push(rect(c.nwell, x, 10.0, x + 3.0, 13.0));
        }
        v.extend(ncomp(&c, 13.4, 10.0, 14.4, 11.0));
        v.extend(ncomp(&c, 20.395, 10.0, 21.395, 11.0));
        v.extend(ncomp(&c, 27.3, 13.3, 28.3, 14.3));
        v.extend(ncomp(&c, 34.28, 13.28, 35.28, 14.28));
        v.extend(ncomp(&c, 41.0, 10.0, 42.0, 11.0));
        v
    });

    // --- Which cells the section speaks for.  11.2 is for "3.3V SRAM cells without
    // marking layer V5_XTOR" and 11.1 for the 5 V ones with it, so the marker alone, the
    // marker under Dualgate and the marker with Dualgate merely abutting are all this
    // deck's; the two that carry V5_XTOR are 11.1's and this deck must be silent on them.
    // Every core holds the same 0.035 poly margin.
    hwrite("S.CO.3_LV.h4", {
        let mut v = vec![];
        for (i, x) in [10.0, 14.0, 24.0, 28.0, 32.0].into_iter().enumerate() {
            v.push(mark(x, 10.0, x + 3.0, 13.0));
            match i {
                1 => v.push(rect(dualgate, x - 0.2, 9.8, x + 3.2, 13.2)),
                2 => {
                    v.push(rect(dualgate, x - 0.2, 9.8, x + 3.2, 13.2));
                    v.push(rect(v5, x - 0.2, 9.8, x + 3.2, 13.2));
                }
                3 => v.push(rect(v5, x - 0.2, 9.8, x + 3.2, 13.2)),
                4 => v.push(rect(dualgate, x + 3.0, 10.0, x + 4.0, 13.0)),
                _ => {}
            }
            v.extend(stack(&c, c.poly, x + 1.0, 11.0, 0.035, 0.0));
        }
        v
    });

    // The same five cores measured by a rule whose layers the deck derives from
    // `sram_lv` rather than from the bare marker.  Only the cores that carry Dualgate
    // hold their active at 0.395; a core under Dualgate but without V5_XTOR is still a
    // cell "without marking layer V5_XTOR", so 11.2's 0.4 is its rule - and the one with
    // Dualgate only abutting is plainly this deck's.
    hwrite("S.DF.4c_LV.h3", {
        let mut v = vec![];
        for (i, x) in [10.0, 16.0, 26.0, 32.0, 38.0].into_iter().enumerate() {
            v.push(mark(x - 0.5, 9.5, x + 4.5, 14.5));
            match i {
                1 => v.push(rect(dualgate, x - 0.7, 9.3, x + 4.7, 14.7)),
                2 => {
                    v.push(rect(dualgate, x - 0.7, 9.3, x + 4.7, 14.7));
                    v.push(rect(v5, x - 0.7, 9.3, x + 4.7, 14.7));
                }
                3 => v.push(rect(v5, x - 0.7, 9.3, x + 4.7, 14.7)),
                4 => v.push(rect(dualgate, x + 4.5, 9.5, x + 5.0, 14.5)),
                _ => {}
            }
            let enc = if matches!(i, 1 | 2 | 4) { 0.395 } else { 0.4 };
            v.extend(well_cell(&c, x, 10.0, enc, 0.0, false));
        }
        v
    });

    // --- The marker cuts the well.  SramCore marks SRAM *cells*, so it selects the
    // shapes the section speaks for; cutting a region at the marker's edge invents a wall
    // that is not in the layout.  (a) a 6 µm well holding its active by 1.0 on every
    // side, with the marker ending 0.2 to the right of the active - the well encloses the
    // active by 1.0 and the layout is clean.  (b) the same well wholly under the marker.
    hwrite("S.DF.4c_LV.h2", {
        let mut v = vec![mark(9.0, 9.0, 15.2, 17.0), mark(23.0, 9.0, 31.0, 17.0)];
        for x in [10.0, 24.0] {
            v.push(rect(c.nwell, x, 10.0, x + 6.0, 16.0));
            v.push(rect(c.comp, x + 1.0, 11.0, x + 5.0, 15.0));
            v.push(rect(c.pplus, x + 1.0, 11.0, x + 5.0, 15.0));
        }
        v
    });

    // --- The tile lines.  Four 0.035 poly margins with the contact straddling x = 20,
    // x = 40, x = 42 and y = 20, and an S.DF.16 gap of 0.395 straddling x = 20.
    hwrite("S.CO.3_LV.h5", {
        let mut v = vec![mark(8.0, 8.0, 44.0, 22.0)];
        for (x, y) in [(19.9, 10.0), (39.9, 10.0), (41.9, 10.0), (10.0, 19.9)] {
            v.extend(stack(&c, c.poly, x, y, 0.035, 0.0));
        }
        v.push(rect(c.nwell, 16.0, 14.0, 19.8, 17.0));
        v.extend(ncomp(&c, 20.195, 14.0, 21.0, 17.0));
        v
    });
}
