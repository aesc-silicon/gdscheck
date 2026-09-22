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
use crate::helpers::{chamfered_tr, layer, library, poly, rect, strip45, write_gz};
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

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------------------
//
// Layouts drawn from the manual's section 7.6 by someone who has not seen the engine: the
// deck's own conditions - what the marker must cover and by how much, the substrate tap
// it need not cover, the partial overlap, the N-well shared by two voltages - at the
// bound and a step past it, and where a tile line or a 45° wall could change the deck's
// reading.  Expected values and the reasoning are in hardening/reports/gf180mcuD/dualgate.md.

/// Layers the hardening patterns draw on.
struct L {
    dg: (i16, i16),
    dnwell: (i16, i16),
    nwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    v5: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            dg: layer(pdk, "dualgate"),
            dnwell: layer(pdk, "dnwell"),
            nwell: layer(pdk, "nwell"),
            comp: layer(pdk, "comp"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            poly: layer(pdk, "poly2_drawn"),
            v5: layer(pdk, "v5_xtor"),
        }
    }

    /// An N+ active - a device's, never a substrate tap.
    fn nact(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x0, y0, x1, y1),
            rect(self.nplus, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    }

    /// A P+ active with no N-well under it: the substrate tap DV.6 and DV.7 excuse.
    fn ptap(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x0, y0, x1, y1),
            rect(self.pplus, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    }

    /// A PMOS: a 3 x 1 P+ active at `(x, y)` with a gate 0.6 wide across it; its N-well
    /// is the caller's.
    fn pmos(&self, x: f64, y: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x, y, x + 3.0, y + 1.0),
            rect(self.pplus, x - 0.1, y - 0.1, x + 3.1, y + 1.1),
            rect(self.poly, x + 1.2, y - 0.5, x + 1.8, y + 1.5),
        ]
    }
}

fn write_h(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    dv1_h(&l);
    dv2_h(&l);
    dv3_h(&l);
    dv5_h(&l);
    dv6_h(&l);
    dv7_h(&l);
    dv8_h(&l);
    dv9_h(&l);
}

// --- DV.1: Dualgate encloses DNWELL by 0.5.

fn dv1_h(l: &L) {
    // 0.495 on the left fires, 0.5 is clean; a deep well crossing the marker's edge fires
    // (the part outside); one abutting the marker from outside is a 3.3 V well, clean;
    // one sharing the marker's left edge is enclosed by 0 there and fires; a chamfered
    // marker corner 0.495 from the well's corner fires (the closest approach).  Then the
    // tile lines: the well's edge on x = 20 with the marker's 0.495 past it, the marker's
    // edge on x = 20 with the well's 0.495 inside, and 0.5 at x = 42 clean.  Two wells in
    // one marker, the second at 0.495.
    let v = vec![
        rect(l.dg, 2.0, 2.0, 8.0, 8.0),
        rect(l.dnwell, 2.495, 3.0, 6.0, 7.0), // DV.1
        rect(l.dg, 10.0, 2.0, 16.0, 8.0),
        rect(l.dnwell, 10.5, 3.0, 14.0, 7.0), // clean
        rect(l.dg, 18.0, 2.0, 22.0, 8.0),
        rect(l.dnwell, 17.0, 3.0, 19.0, 7.0), // DV.1: crossing
        rect(l.dg, 26.0, 2.0, 30.0, 8.0),
        rect(l.dnwell, 23.5, 3.0, 26.0, 7.0), // clean: abutting from outside
        rect(l.dg, 32.0, 2.0, 38.0, 8.0),
        rect(l.dnwell, 32.0, 3.0, 35.0, 7.0), // DV.1: enclosed by 0 on the left
        chamfered_tr(l.dg, 40.5, 2.0, 46.0, 8.0, 52.7), // chamfer (46, 6.7)-(44.7, 8)
        rect(l.dnwell, 42.0, 3.0, 45.0, 7.0), // DV.1: 0.495 from (45, 7)
        rect(l.dg, 16.0, 10.0, 20.495, 16.0),
        rect(l.dnwell, 17.0, 11.0, 20.0, 15.0), // DV.1: the well's edge on x = 20
        rect(l.dg, 14.0, 18.0, 20.0, 24.0),
        rect(l.dnwell, 15.0, 19.0, 19.505, 23.0), // DV.1: the marker's edge on x = 20
        rect(l.dg, 38.0, 10.0, 42.0, 16.0),
        rect(l.dnwell, 39.0, 11.0, 41.5, 15.0), // clean: 0.5 at x = 42
        rect(l.dg, 48.0, 2.0, 56.0, 8.0),
        rect(l.dnwell, 49.0, 3.0, 51.0, 7.0),   // clean
        rect(l.dnwell, 52.0, 3.0, 55.505, 7.0), // DV.1
    ];
    write_h("DV.1.h1", v);
}

// --- DV.2: Dualgate space 0.44, merged when closer.

fn dv2_h(l: &L) {
    // 0.435 fires and 0.44 is clean; a U with a 0.435 notch; two overlapping boxes are one
    // marker; corner to corner 0.31/0.31 (0.438) fires, 0.315/0.315 (0.4455) is clean; a
    // 0.435 gap straddling x = 20 and one starting on x = 42.
    let mut v = vec![
        rect(l.dg, 2.0, 2.0, 4.0, 6.0),
        rect(l.dg, 4.435, 2.0, 6.435, 6.0), // DV.2
        rect(l.dg, 8.0, 2.0, 10.0, 6.0),
        rect(l.dg, 10.44, 2.0, 12.0, 6.0), // clean
    ];
    v.push(poly(
        l.dg,
        &[
            (14.0, 2.0),
            (18.0, 2.0),
            (18.0, 6.0),
            (16.435, 6.0),
            (16.435, 3.0),
            (16.0, 3.0),
            (16.0, 6.0),
            (14.0, 6.0),
        ],
    )); // DV.2: notch
    v.push(rect(l.dg, 23.0, 2.0, 25.0, 6.0));
    v.push(rect(l.dg, 24.0, 3.0, 26.0, 5.0)); // clean: one marker
    v.push(rect(l.dg, 28.0, 2.0, 30.0, 4.0));
    v.push(rect(l.dg, 30.31, 4.31, 32.0, 6.0)); // DV.2: 0.438
    v.push(rect(l.dg, 34.0, 2.0, 36.0, 4.0));
    v.push(rect(l.dg, 36.315, 4.315, 38.0, 6.0)); // clean: 0.4455
    v.push(rect(l.dg, 17.5, 8.0, 19.8, 12.0));
    v.push(rect(l.dg, 20.235, 8.0, 22.0, 12.0)); // DV.2: across x = 20
    v.push(rect(l.dg, 40.0, 8.0, 42.0, 12.0));
    v.push(rect(l.dg, 42.435, 8.0, 44.0, 12.0)); // DV.2: from x = 42
    write_h("DV.2.h1", v);
}

// --- DV.3: Dualgate to an unrelated (outside) COMP 0.24.

fn dv3_h(l: &L) {
    // 0.235 fires, 0.24 is clean; an active abutting the marker from outside is 0 away
    // and fires; one partly inside is not outside and is DV.6's and DV.7's, not this
    // rule's; a chamfered marker corner 0.233 from the active's corner fires; a substrate
    // tap gets no exemption here.  Then the tile lines: the active's edge on x = 20 with
    // the marker 0.235 before it, the marker's edge on x = 20 with the active 0.235 past
    // it, and 0.24 at x = 42 clean.
    let mut v = vec![
        rect(l.dg, 2.0, 2.0, 5.0, 6.0),
        rect(l.comp, 5.235, 3.0, 7.0, 5.0), // DV.3
        rect(l.dg, 9.0, 2.0, 12.0, 6.0),
        rect(l.comp, 12.24, 3.0, 14.0, 5.0), // clean
        rect(l.dg, 16.0, 2.0, 19.0, 6.0),
        rect(l.comp, 19.0, 3.0, 21.0, 5.0), // DV.3: abutting, 0 apart
        rect(l.dg, 23.0, 2.0, 26.0, 6.0),
        rect(l.comp, 25.0, 3.0, 28.0, 5.0), // clean here: DV.6 / DV.7 (ignored)
        chamfered_tr(l.dg, 30.0, 2.0, 34.0, 6.0, 39.67), // chamfer (34, 5.67)-(33.67, 6)
        rect(l.comp, 34.0, 6.0, 36.0, 8.0), // DV.3: 0.233 from (34, 6)
        rect(l.dg, 38.0, 2.0, 41.0, 6.0),
    ];
    v.extend(l.ptap(41.235, 3.0, 43.0, 5.0)); // DV.3: a tap
    v.push(rect(l.dg, 16.5, 8.0, 19.765, 12.0));
    v.push(rect(l.comp, 20.0, 9.0, 22.0, 11.0)); // DV.3: the active's edge on x = 20
    v.push(rect(l.dg, 14.0, 14.0, 20.0, 18.0));
    v.push(rect(l.comp, 20.235, 15.0, 22.0, 17.0)); // DV.3: the marker's edge on x = 20
    v.push(rect(l.dg, 38.0, 8.0, 41.76, 12.0));
    v.push(rect(l.comp, 42.0, 9.0, 44.0, 11.0)); // clean: 0.24 at x = 42
    write_h("DV.3.h1", v);
}

// --- DV.5: Dualgate width 0.7.

fn dv5_h(l: &L) {
    // 0.695 fires (two walls) in x and in y, 0.7 is clean; a 45° strip 0.693 wide fires
    // and 0.700 is clean; a 0.695 bar across x = 20 fires and a 0.7 one across x = 42 is
    // clean.
    let v = vec![
        rect(l.dg, 2.0, 2.0, 2.695, 6.0),     // DV.5 x2
        rect(l.dg, 4.0, 2.0, 4.7, 6.0),       // clean
        rect(l.dg, 6.0, 2.0, 10.0, 2.695),    // DV.5 x2
        strip45(l.dg, 12.5, 2.0, 3.0, 0.49),  // DV.5 x2: 0.693
        strip45(l.dg, 12.5, 6.0, 3.0, 0.495), // clean: 0.700
        rect(l.dg, 18.0, 2.0, 22.0, 2.695),   // DV.5 x2 across x = 20
        rect(l.dg, 39.0, 2.0, 43.0, 2.7),     // clean across x = 42
    ];
    write_h("DV.5.h1", v);
}

// --- DV.6: Dualgate encloses COMP by 0.24, except the substrate tap.

fn dv6_h(l: &L) {
    // 0.235 fires and 0.24 is clean; an N+ tap in an N-well is no substrate tap and fires
    // at 0.235; a substrate tap 0.1 inside is clean, and so is one crossing the marker's
    // edge; a P+ active abutting the N-well is still a tap (clean at 0.1), one crossing
    // the N-well's edge is not (fires at 0.235); an active sharing the marker's edge is
    // enclosed by 0 and fires; one crossing the edge fires (and DV.7, ignored); a
    // chamfered marker corner 0.233 from the active's corner fires.  Then the tile lines:
    // the active's edge on x = 20 with the marker 0.235 past it, the marker's edge on
    // x = 20 with the active 0.235 inside, and 0.24 at x = 42 clean.
    let mut v = vec![rect(l.dg, 2.0, 2.0, 6.0, 6.0)];
    v.extend(l.nact(2.235, 3.0, 5.0, 5.0)); // DV.6
    v.push(rect(l.dg, 8.0, 2.0, 12.0, 6.0));
    v.extend(l.nact(8.24, 3.0, 11.0, 5.0)); // clean
    v.push(rect(l.dg, 14.0, 2.0, 18.0, 6.0));
    v.push(rect(l.nwell, 14.1, 2.5, 17.5, 5.5));
    v.extend(l.nact(14.235, 3.0, 17.0, 5.0)); // DV.6: an N-well tap
    v.push(rect(l.dg, 21.0, 2.0, 25.0, 6.0));
    v.extend(l.ptap(21.1, 3.0, 24.0, 5.0)); // clean: a substrate tap
    v.push(rect(l.dg, 27.0, 2.0, 31.0, 6.0));
    v.extend(l.ptap(26.0, 3.0, 29.0, 5.0)); // clean: a tap crossing the edge
    v.push(rect(l.dg, 33.0, 2.0, 37.0, 6.0));
    v.push(rect(l.nwell, 33.5, 2.5, 34.5, 5.5));
    v.extend(l.ptap(34.5, 3.0, 36.9, 5.0)); // clean: a tap abutting the N-well
    v.push(rect(l.dg, 39.0, 2.0, 43.0, 6.0));
    v.push(rect(l.nwell, 39.5, 2.5, 41.0, 5.5));
    v.extend(l.ptap(40.0, 3.0, 42.765, 5.0)); // DV.6: P+ crossing the N-well's edge
    v.push(rect(l.dg, 45.0, 2.0, 49.0, 6.0));
    v.extend(l.nact(45.0, 3.0, 48.0, 5.0)); // DV.6: enclosed by 0 on the left
    v.push(rect(l.dg, 51.0, 2.0, 55.0, 6.0));
    v.extend(l.nact(50.0, 3.0, 53.0, 5.0)); // DV.6: crossing (+ DV.7)
    v.push(chamfered_tr(l.dg, 57.0, 2.0, 61.0, 6.0, 65.33)); // chamfer (61, 4.33)-(59.33, 6)
    v.extend(l.nact(58.0, 3.0, 60.0, 5.0)); // DV.6: 0.233 from (60, 5)
    v.push(rect(l.dg, 14.0, 8.0, 20.235, 12.0));
    v.extend(l.nact(15.0, 9.0, 20.0, 11.0)); // DV.6: the active's edge on x = 20
    v.push(rect(l.dg, 14.0, 14.0, 20.0, 18.0));
    v.extend(l.nact(15.0, 15.0, 19.765, 17.0)); // DV.6: the marker's edge on x = 20
    v.push(rect(l.dg, 38.0, 8.0, 42.0, 12.0));
    v.extend(l.nact(39.0, 9.0, 41.76, 11.0)); // clean: 0.24 at x = 42
    write_h("DV.6.h1", v);
}

// --- DV.7: COMP (except the substrate tap) cannot be partially overlapped by Dualgate.

fn dv7_h(l: &L) {
    // A marker straddling an active fires (and DV.6, ignored); a marker that covers one
    // active whole and straddles a second still straddles the second - the manual says
    // nothing about the marker's other actives - and fires; a straddled substrate tap
    // is excused; an active sharing the marker's edge is covered; one abutting the
    // marker from outside is not overlapped (DV.3's, ignored); a marker drawn as two
    // abutting boxes covering one active is one marker and covers it.  Then a straddled
    // active crossing x = 20 and one crossing x = 42.
    let mut v = vec![rect(l.dg, 2.0, 2.0, 6.0, 6.0)];
    v.extend(l.nact(5.0, 3.0, 8.0, 5.0)); // DV.7
    v.push(rect(l.dg, 10.0, 2.0, 16.0, 6.0));
    v.extend(l.nact(11.0, 3.0, 13.0, 5.0)); // covered
    v.extend(l.nact(15.0, 3.0, 18.0, 5.0)); // DV.7: straddled by the same marker
    v.push(rect(l.dg, 20.5, 2.0, 24.0, 6.0));
    v.extend(l.ptap(23.0, 3.0, 26.0, 5.0)); // clean: a tap
    v.push(rect(l.dg, 28.0, 2.0, 32.0, 6.0));
    v.extend(l.nact(28.0, 3.0, 30.0, 5.0)); // clean: covered, on the edge (DV.6 ignored)
    v.push(rect(l.dg, 34.0, 2.0, 38.0, 6.0));
    v.extend(l.nact(38.0, 3.0, 40.0, 5.0)); // clean: abutting from outside
    v.push(rect(l.dg, 42.0, 2.0, 44.0, 6.0));
    v.push(rect(l.dg, 44.0, 2.0, 46.0, 6.0));
    v.extend(l.nact(43.0, 3.0, 45.0, 5.0)); // clean: one marker of two boxes
    v.push(rect(l.dg, 16.0, 8.0, 20.0, 12.0));
    v.extend(l.nact(19.0, 9.0, 22.0, 11.0)); // DV.7: across x = 20
    v.push(rect(l.dg, 38.0, 8.0, 43.0, 12.0));
    v.extend(l.nact(41.0, 9.0, 44.0, 11.0)); // DV.7: across x = 42
    write_h("DV.7.h1", v);
}

// --- DV.8: Dualgate encloses Poly2 by 0.4.

fn dv8_h(l: &L) {
    // 0.395 fires and 0.4 is clean; a poly crossing the marker's edge fires; one abutting
    // the marker from outside is not enclosed at all and is clean; one sharing the
    // marker's edge is enclosed by 0 and fires; a chamfered marker corner 0.389 from the
    // poly's corner fires; a poly in the hole of a marker ring (four boxes) is outside
    // and clean, and one under the ring 0.395 from the hole's edge fires.  Then the tile
    // lines: the poly's edge on x = 20 with the marker 0.395 past it, the marker's edge
    // on x = 20 with the poly 0.395 inside, 0.4 at x = 42 clean; and a 300 µm poly
    // running into a marker at x = 250.
    let v = vec![
        rect(l.dg, 2.0, 2.0, 6.0, 6.0),
        rect(l.poly, 2.395, 3.0, 4.0, 5.0), // DV.8
        rect(l.dg, 8.0, 2.0, 12.0, 6.0),
        rect(l.poly, 8.4, 3.0, 10.0, 5.0), // clean
        rect(l.dg, 14.0, 2.0, 18.0, 6.0),
        rect(l.poly, 13.0, 3.0, 15.0, 5.0), // DV.8: crossing
        rect(l.dg, 21.0, 2.0, 25.0, 6.0),
        rect(l.poly, 19.5, 3.0, 21.0, 5.0), // clean: abutting from outside
        rect(l.dg, 27.0, 2.0, 31.0, 6.0),
        rect(l.poly, 27.0, 3.0, 29.0, 5.0), // DV.8: enclosed by 0 on the left
        chamfered_tr(l.dg, 33.0, 2.0, 37.0, 6.0, 41.55), // chamfer (37, 4.55)-(35.55, 6)
        rect(l.poly, 34.0, 3.0, 36.0, 5.0), // DV.8: 0.389 from (36, 5)
        rect(l.dg, 39.0, 2.0, 41.0, 10.0),  // a ring, four boxes
        rect(l.dg, 45.0, 2.0, 47.0, 10.0),
        rect(l.dg, 41.0, 2.0, 45.0, 4.0),
        rect(l.dg, 41.0, 8.0, 45.0, 10.0),
        rect(l.poly, 42.0, 5.0, 44.0, 7.0),   // clean: in the hole
        rect(l.poly, 43.0, 8.395, 44.0, 9.5), // DV.8: under the ring
        rect(l.dg, 14.0, 12.0, 20.395, 16.0),
        rect(l.poly, 15.0, 13.0, 20.0, 15.0), // DV.8: the poly's edge on x = 20
        rect(l.dg, 14.0, 18.0, 20.0, 22.0),
        rect(l.poly, 15.0, 19.0, 19.605, 21.0), // DV.8: the marker's edge on x = 20
        rect(l.dg, 38.0, 12.0, 42.0, 16.0),
        rect(l.poly, 39.0, 13.0, 41.6, 15.0), // clean: 0.4 at x = 42
        rect(l.poly, 2.0, 25.0, 302.0, 25.4), // DV.8: 300 µm, into a marker at 250
        rect(l.dg, 250.0, 23.0, 310.0, 27.0),
    ];
    write_h("DV.8.h1", v);
}

// --- DV.9: a 3.3 V and a 5 V / 6 V PMOS cannot sit in the same N-well.

fn dv9_h(l: &L) {
    // Each row is one N-well: a 6 V PMOS (under a marker with 0.5 to spare on every
    // side) and a 3.3 V one 1.5 past the marker.  One well holding both fires (one
    // report per well); two wells 1.4 apart are clean; two abutting boxes are one well
    // and fire; a 3.3 V gate a second marker straddles is still 3.3 V (DV.6, DV.7 and
    // DV.8 ignored) and fires; a 3.3 V gate under a V5_XTOR with no marker is excused
    // (PL.11's business); two 3.3 V gates beside one 6 V are one report.  Then a 44 µm
    // well with the 6 V gate at x = 2 and the 3.3 V one at x = 42.
    let row = |x: f64, y: f64| {
        let mut r = vec![rect(l.dg, x - 0.5, y - 1.0, x + 3.5, y + 2.0)];
        r.extend(l.pmos(x, y));
        r.extend(l.pmos(x + 5.0, y));
        r
    };
    let mut v = vec![rect(l.nwell, 1.7, 1.7, 10.3, 3.3)];
    v.extend(row(2.0, 2.0)); // DV.9
    v.push(rect(l.nwell, 12.7, 1.7, 16.3, 3.3));
    v.push(rect(l.nwell, 17.7, 1.7, 21.3, 3.3));
    v.extend(row(13.0, 2.0)); // clean: two wells
    v.push(rect(l.nwell, 23.7, 1.7, 27.3, 3.3));
    v.push(rect(l.nwell, 27.3, 1.7, 32.3, 3.3));
    v.extend(row(24.0, 2.0)); // DV.9: two boxes, one well
    v.push(rect(l.nwell, 34.7, 1.7, 43.3, 3.3));
    v.extend(row(35.0, 2.0));
    v.push(rect(l.dg, 39.0, 1.0, 41.5, 4.0)); // DV.9: the 3.3 V gate straddled
    v.push(rect(l.nwell, 45.7, 1.7, 54.3, 3.3));
    v.extend(row(46.0, 2.0));
    v.push(rect(l.v5, 50.5, 1.0, 54.5, 4.0)); // clean: the 3.3 V gate under V5_XTOR
    v.push(rect(l.nwell, 56.7, 1.7, 70.3, 3.3));
    v.extend(row(57.0, 2.0));
    v.extend(l.pmos(67.0, 2.0)); // DV.9: one report for the well
    v.push(rect(l.nwell, 1.7, 7.7, 45.3, 9.3));
    v.push(rect(l.dg, 1.5, 7.0, 5.5, 10.0));
    v.extend(l.pmos(2.0, 8.0));
    v.extend(l.pmos(42.0, 8.0)); // DV.9: across every tile line
    write_h("DV.9.h1", v);
}
