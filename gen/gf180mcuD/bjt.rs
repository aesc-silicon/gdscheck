// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Vertical NPN: a good and a bad pattern for every rule in the `drc_bjt` and `lvs_bjt`
//! decks, which measure the same device and so share a cell.
//!
//! The transistor is three actives in a deep well: an N+ emitter carrying LVS_BJT, a P+
//! base in the LVPWELL, and an N+ collector without the LVS marker - the marker is what
//! separates emitter from collector, both being N+ in the same well.  A DRC_BJT marker
//! over the three is what makes them a transistor as far as the deck is concerned, so
//! every fixture draws all four or the rules have nothing to recognise.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const OUT: &str = "tests/data/gf180mcuD/generated";

/// Each active, and the gaps between them.
const ACT: f64 = 2.0;
const GAP: f64 = 1.5;
/// How far the deep well reaches past the actives, and the marker past the well.  The
/// nesting is what BJT.1 asks for: the marker has to *contain* the well, not sit in it.
const DN: f64 = 0.5;
const MK: f64 = 1.0;

struct Ctx {
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    drc_bjt: (i16, i16),
    lvs_bjt: (i16, i16),
}

struct Cell {
    x: f64,
    y: f64,
    /// How far the deep well reaches past the actives.  Past the marker's own reach and
    /// the well is no longer inside it, which is what BJT.1 objects to.
    dn: f64,
    /// Whether LVS_BJT covers the emitter, which LVS_BJT.1 asks.
    lvs: bool,
    /// Whether the P+ base is drawn - BJT.2 wants the marker to hold a substrate tap.
    base: bool,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            dn: DN,
            lvs: true,
            base: true,
        }
    }

    /// Emitter, base and collector, in a row.
    fn actives(&self) -> [(f64, f64, f64, f64); 3] {
        let (x, y) = (self.x, self.y);
        let step = ACT + GAP;
        [0.0, 1.0, 2.0].map(|i| {
            let x0 = x + i * step;
            (x0, y, x0 + ACT, y + ACT)
        })
    }

    /// The deep well, around the actives.
    fn well(&self) -> (f64, f64, f64, f64) {
        let a = self.actives();
        (
            a[0].0 - self.dn,
            a[0].1 - self.dn,
            a[2].2 + self.dn,
            a[2].3 + self.dn,
        )
    }

    /// The DRC_BJT marker, around the well.
    fn marker(&self) -> (f64, f64, f64, f64) {
        let a = self.actives();
        (
            a[0].0 - DN - MK,
            a[0].1 - DN - MK,
            a[2].2 + DN + MK,
            a[2].3 + DN + MK,
        )
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let [e, b, col] = self.actives();
        let (wx0, wy0, wx1, wy1) = self.well();
        let (mx0, my0, mx1, my1) = self.marker();
        let mut v = vec![
            // The deep well under everything; the LVPWELL inside it holds the base.
            rect(c.dnwell, wx0, wy0, wx1, wy1),
            rect(c.lvpwell, b.0 - 0.5, b.1 - 0.5, b.2 + 0.5, b.3 + 0.5),
            rect(c.drc_bjt, mx0, my0, mx1, my1),
            // Emitter and collector: N+ actives, told apart by the LVS marker.
            rect(c.comp, e.0, e.1, e.2, e.3),
            rect(c.nplus, e.0 - 0.3, e.1 - 0.3, e.2 + 0.3, e.3 + 0.3),
            rect(c.comp, col.0, col.1, col.2, col.3),
            rect(c.nplus, col.0 - 0.3, col.1 - 0.3, col.2 + 0.3, col.3 + 0.3),
        ];
        if self.lvs {
            v.push(rect(c.lvs_bjt, e.0 - 0.2, e.1 - 0.2, e.2 + 0.2, e.3 + 0.2));
        }
        if self.base {
            v.push(rect(c.comp, b.0, b.1, b.2, b.3));
            v.push(rect(c.pplus, b.0 - 0.3, b.1 - 0.3, b.2 + 0.3, b.3 + 0.3));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    for deck in ["drc_bjt", "lvs_bjt"] {
        std::fs::create_dir_all(format!("{OUT}/{deck}")).expect("pattern dir");
    }
    let c = Ctx {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        drc_bjt: layer(pdk, "drc_bjt"),
        lvs_bjt: layer(pdk, "lvs_bjt"),
    };
    let o = OFFSET;
    let write = |deck: &str, name: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{OUT}/{deck}/{name}.gds.gz"),
            library("TOP", elems),
        );
    };
    let base = Cell::at(o, o);
    let clean = || base.draw(&c);

    for id in ["BJT.1", "BJT.2", "BJT.3"] {
        write("drc_bjt", &format!("{id}.good"), clean());
    }
    write("lvs_bjt", "LVS_BJT.1.good", clean());

    // BJT.1: the deep well reaching outside the marker that has to contain it.
    write("drc_bjt", "BJT.1.bad", {
        let mut cell = Cell::at(o, o);
        cell.dn = DN + MK + 0.5;
        cell.draw(&c)
    });

    // BJT.2: no P+ tap under the marker.  The base is that tap, so leaving it out is what
    // the rule objects to - and the transistor still reads as one, the emitter and
    // collector being what BJT.1 recognises.
    write("drc_bjt", "BJT.2.bad", {
        let mut cell = Cell::at(o, o);
        cell.base = false;
        cell.draw(&c)
    });

    // BJT.3: an active outside the marker, within 0.1 µm of it.
    write("drc_bjt", "BJT.3.bad", {
        let mut v = clean();
        let (_, my0, mx1, _) = base.marker();
        let x = mx1 + 0.095;
        v.push(rect(c.comp, x, my0, x + 2.0, my0 + 2.0));
        v
    });

    // LVS_BJT.1: an emitter the LVS marker does not cover.  Removing the marker entirely
    // would take the emitter with it - an emitter is N+ and LVS_BJT together - so the
    // marker stays and covers only part of it.
    write("lvs_bjt", "LVS_BJT.1.bad", {
        let mut v = clean();
        v.retain(
            |el| !matches!(el, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.lvs_bjt),
        );
        let [e, _, _] = base.actives();
        v.push(rect(
            c.lvs_bjt,
            e.0 - 0.2,
            e.1 - 0.2,
            e.0 + ACT * 0.5,
            e.3 + 0.2,
        ));
        v
    });

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Sections 10.7 (BJT.1 to BJT.3) and 10.9 (LVS_BJT.1) are four rules and almost all of
// the work is recognition: DRC_BJT marks "vertical NPN and PNP transistors", LVS_BJT
// "identifies Emitter, Base and Collector", and a rule fires only where the layers make
// a device.  So most of what is drawn here is each way of *not* being one.  Findings:
// hardening/reports/gf180mcuD/drc_bjt.md and lvs_bjt.md; cases: the `hardening_drc_bjt`
// and `hardening_lvs_bjt` tables of tests/gf180mcuD.rs.

fn hwrite(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(
        &format!("{OUT}/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}

/// A vertical NPN under a DRC_BJT marker `9 × 6` at `(x, y)`: the deep well, an LVPWELL
/// in it, an N+ emitter under LVS_BJT, a P+ base in the LVPWELL, an N+ collector without
/// the marker, and a P+ substrate tap wholly inside the marker so BJT.2 has its PCOM.
/// Each terminal can be left out, and the deep well can be run past the marker's edge.
struct Npn {
    x: f64,
    y: f64,
    emitter: bool,
    base: bool,
    collector: bool,
    /// How far past the marker's right edge the deep well runs; 0 keeps it inside.
    dn_out: f64,
    /// The substrate tap the marker has to cover; false leaves it out.
    tap: bool,
}

impl Npn {
    fn at(x: f64, y: f64) -> Self {
        Npn {
            x,
            y,
            emitter: true,
            base: true,
            collector: true,
            dn_out: 0.0,
            tap: true,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let dn_x1 = if self.dn_out > 0.0 {
            x + 9.0 + self.dn_out
        } else {
            x + 8.0
        };
        let mut v = vec![
            rect(c.drc_bjt, x, y, x + 9.0, y + 6.0),
            rect(c.dnwell, x + 1.0, y + 1.0, dn_x1, y + 5.0),
            rect(c.lvpwell, x + 3.5, y + 1.5, x + 5.5, y + 4.5),
        ];
        if self.emitter {
            v.push(rect(c.comp, x + 1.5, y + 2.0, x + 2.5, y + 3.0));
            v.push(rect(c.nplus, x + 1.5, y + 2.0, x + 2.5, y + 3.0));
            v.push(rect(c.lvs_bjt, x + 1.3, y + 1.8, x + 2.7, y + 3.2));
        }
        if self.base {
            v.push(rect(c.comp, x + 4.0, y + 2.0, x + 5.0, y + 3.0));
            v.push(rect(c.pplus, x + 4.0, y + 2.0, x + 5.0, y + 3.0));
        }
        if self.collector {
            v.push(rect(c.comp, x + 6.0, y + 2.0, x + 7.0, y + 3.0));
            v.push(rect(c.nplus, x + 6.0, y + 2.0, x + 7.0, y + 3.0));
        }
        if self.tap {
            v.push(rect(c.comp, x + 1.5, y + 5.2, x + 2.5, y + 5.7));
            v.push(rect(c.pplus, x + 1.5, y + 5.2, x + 2.5, y + 5.7));
        }
        v
    }
}

/// A DRC_BJT marker `3 × 3` at `(x, y)` holding a P+ substrate tap, so BJT.1 has no
/// device to recognise and BJT.2 has its PCOM covered.
fn bare_marker(c: &Ctx, x: f64, y: f64) -> Vec<GdsElement> {
    vec![
        rect(c.drc_bjt, x, y, x + 3.0, y + 3.0),
        rect(c.comp, x + 1.0, y + 1.0, x + 2.0, y + 2.0),
        rect(c.pplus, x + 1.0, y + 1.0, x + 2.0, y + 2.0),
    ]
}

/// An N+ active.
fn ncomp(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![rect(c.comp, x0, y0, x1, y1), rect(c.nplus, x0, y0, x1, y1)]
}

/// A P+ active.
fn pcomp(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    vec![rect(c.comp, x0, y0, x1, y1), rect(c.pplus, x0, y0, x1, y1)]
}

fn hardening(pdk: &PdkConfig) {
    let c = Ctx {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        drc_bjt: layer(pdk, "drc_bjt"),
        lvs_bjt: layer(pdk, "lvs_bjt"),
    };
    let nwell = layer(pdk, "nwell");

    // --- BJT.1, "Min. DRC_BJT overlap of DNWELL for NPN BJT 0".  A device is an NPN only
    // where the marker meets all three terminals, so the row is one complete NPN whose
    // deep well runs 1 µm past the marker, then the same layout less the LVS_BJT that
    // makes the emitter an emitter, less the base, less the collector - none of those is
    // a transistor and none may fire - and last a complete NPN with its well inside.
    hwrite("drc_bjt", "BJT.1.h1", {
        let mut v = Vec::new();
        for (i, x) in [10.0, 22.0, 34.0, 46.0, 58.0].into_iter().enumerate() {
            let mut d = Npn::at(x, 10.0);
            d.dn_out = if i == 4 { 0.0 } else { 1.0 };
            match i {
                1 => d.emitter = false,
                2 => d.base = false,
                3 => d.collector = false,
                _ => {}
            }
            v.extend(d.draw(&c));
        }
        v
    });

    // The bound itself, which is zero: (a) the deep well's edge lying on the marker's on
    // every side - an overlap of exactly 0, which the rule allows; (b) the well 0.005 µm
    // past the marker on one side; (c) the well wholly inside.
    hwrite("drc_bjt", "BJT.1.h2", {
        let mut v = Vec::new();
        for (i, x) in [10.0, 22.0, 34.0].into_iter().enumerate() {
            let mut d = Npn::at(x, 10.0);
            d.dn_out = match i {
                0 => 0.0,
                1 => 0.005,
                _ => 0.0,
            };
            let mut e = d.draw(&c);
            if i == 0 {
                // Redraw the deep well flush with the marker on all four sides.
                e.retain(
                    |el| !matches!(el, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.dnwell),
                );
                e.push(rect(c.dnwell, x, 10.0, x + 9.0, 16.0));
            }
            v.extend(e);
        }
        v
    });

    // Two deep wells under one marker: the NPN's, wholly inside, and a second that runs
    // out of it.  The marker overlaps a DNWELL, but not that one.
    hwrite("drc_bjt", "BJT.1.h3", {
        let mut v = Npn::at(10.0, 10.0).draw(&c);
        v.push(rect(c.dnwell, 18.3, 11.0, 21.0, 15.0));
        v
    });

    // --- BJT.2, "Min. DRC_BJT overlap of PCOM in Psub 0".  (a) the tap wholly inside the
    // marker; (b) a tap the marker's edge cuts; (c) a marker whose only P+ is inside an
    // N-well, so there is no PCOM in the substrate at all; (d) one tap inside and a
    // second crossing out; (e) a tap whose right edge lies on the marker's, an overlap of
    // exactly 0.
    hwrite("drc_bjt", "BJT.2.h1", {
        let mut v = Vec::new();
        v.extend(bare_marker(&c, 10.0, 10.0));
        v.push(rect(c.drc_bjt, 16.0, 10.0, 19.0, 13.0));
        v.extend(pcomp(&c, 18.0, 11.0, 20.0, 12.0));
        v.push(rect(c.drc_bjt, 22.0, 10.0, 25.0, 13.0));
        v.push(rect(nwell, 22.5, 10.5, 24.5, 12.5));
        v.extend(pcomp(&c, 23.0, 11.0, 24.0, 12.0));
        v.push(rect(c.drc_bjt, 28.0, 10.0, 31.0, 13.0));
        v.extend(pcomp(&c, 28.5, 11.0, 29.0, 11.5));
        v.extend(pcomp(&c, 30.0, 11.0, 32.0, 12.0));
        v.push(rect(c.drc_bjt, 34.0, 10.0, 37.0, 13.0));
        v.extend(pcomp(&c, 36.0, 11.0, 37.0, 12.0));
        v
    });

    // --- BJT.3, "Minimum space of DRC_BJT layer to unrelated COMP 0.1": the bound, the
    // step past it, a corner-to-corner pair at 0.1018 (clean) and one at 0.0990 (fires),
    // an active whose wall lies on the marker's - a space of nothing - and an active that
    // overlaps the marker, which is related and exempt.
    hwrite("drc_bjt", "BJT.3.h1", {
        let mut v = Vec::new();
        for x in [10.0, 16.0, 22.0, 28.0, 34.0, 40.0] {
            v.extend(bare_marker(&c, x, 10.0));
        }
        v.extend(ncomp(&c, 13.1, 10.0, 14.1, 11.0));
        v.extend(ncomp(&c, 19.095, 10.0, 20.095, 11.0));
        v.extend(ncomp(&c, 25.072, 13.072, 26.072, 14.072));
        v.extend(ncomp(&c, 31.07, 13.07, 32.07, 14.07));
        v.extend(ncomp(&c, 37.0, 10.0, 38.0, 11.0));
        v.extend(ncomp(&c, 42.5, 10.0, 44.0, 11.0));
        v
    });

    // The same 0.095 gap on the tile lines: straddling x = 20, x = 42 and y = 20.
    hwrite("drc_bjt", "BJT.3.h2", {
        let mut v = Vec::new();
        v.extend(bare_marker(&c, 17.0, 10.0));
        v.extend(ncomp(&c, 20.095, 10.0, 21.095, 11.0));
        v.extend(bare_marker(&c, 39.0, 10.0));
        v.extend(ncomp(&c, 42.095, 10.0, 43.095, 11.0));
        v.extend(bare_marker(&c, 10.0, 16.0));
        v.extend(ncomp(&c, 10.0, 19.095, 11.0, 20.095));
        v
    });

    // --- LVS_BJT.1, "Minimum LVS_BJT enclosure of NPN or PNP Emitter COMP layers 0".  An
    // NPN emitter is an N+ active inside the deep well that the LVS marker names: (a) the
    // marker over the whole active; (b) over half of it; (c) only abutting it, which the
    // deck still reads as naming it; (d) an active the deep well's edge cuts, which is no
    // emitter; (e) an active outside every well; (f) the marker over all but a 0.005 µm
    // sliver of the corner.
    hwrite("lvs_bjt", "LVS_BJT.1.h1", {
        let mut v = Vec::new();
        for x in [10.0, 14.0, 18.0, 22.0, 30.0] {
            v.push(rect(c.dnwell, x, 10.0, x + 3.0, 13.0));
        }
        v.extend(ncomp(&c, 11.0, 11.0, 12.0, 12.0));
        v.push(rect(c.lvs_bjt, 10.8, 10.8, 12.2, 12.2));
        v.extend(ncomp(&c, 15.0, 11.0, 16.0, 12.0));
        v.push(rect(c.lvs_bjt, 14.8, 10.8, 15.5, 12.2));
        v.extend(ncomp(&c, 19.0, 11.0, 20.0, 12.0));
        v.push(rect(c.lvs_bjt, 18.0, 11.0, 19.0, 12.0));
        v.extend(ncomp(&c, 24.0, 11.0, 26.0, 12.0));
        v.push(rect(c.lvs_bjt, 23.8, 10.8, 25.0, 12.2));
        v.extend(ncomp(&c, 35.0, 11.0, 36.0, 12.0));
        v.push(rect(c.lvs_bjt, 34.8, 10.8, 35.5, 12.2));
        v.extend(ncomp(&c, 31.0, 11.0, 32.0, 12.0));
        v.push(rect(c.lvs_bjt, 30.8, 10.8, 32.2, 11.995));
        v
    });

    // A PNP emitter is a P+ active inside an N-well: (a) the marker over the whole
    // active; (b) over half of it; (c) the same P+ active inside a deep well instead,
    // which is no PNP emitter; (d) an N+ active in the N-well, which is neither; (e) a P+
    // active the N-well's edge cuts.
    hwrite("lvs_bjt", "LVS_BJT.1.h2", {
        let mut v = Vec::new();
        for x in [10.0, 14.0, 22.0, 26.0] {
            v.push(rect(nwell, x, 10.0, x + 3.0, 13.0));
        }
        v.push(rect(c.dnwell, 18.0, 10.0, 21.0, 13.0));
        v.extend(pcomp(&c, 11.0, 11.0, 12.0, 12.0));
        v.push(rect(c.lvs_bjt, 10.8, 10.8, 12.2, 12.2));
        v.extend(pcomp(&c, 15.0, 11.0, 16.0, 12.0));
        v.push(rect(c.lvs_bjt, 14.8, 10.8, 15.5, 12.2));
        v.extend(pcomp(&c, 19.0, 11.0, 20.0, 12.0));
        v.push(rect(c.lvs_bjt, 18.8, 10.8, 19.5, 12.2));
        v.extend(ncomp(&c, 23.0, 11.0, 24.0, 12.0));
        v.push(rect(c.lvs_bjt, 22.8, 10.8, 23.5, 12.2));
        v.extend(pcomp(&c, 28.0, 11.0, 30.0, 12.0));
        v.push(rect(c.lvs_bjt, 27.8, 10.8, 29.0, 12.2));
        v
    });

    // The same half-covered emitter on the tile lines: straddling x = 20, x = 42, y = 20.
    hwrite("lvs_bjt", "LVS_BJT.1.h3", {
        let mut v = Vec::new();
        for (x, y) in [(19.5, 10.0), (41.5, 10.0), (10.0, 19.5)] {
            v.push(rect(c.dnwell, x - 1.0, y - 1.0, x + 2.0, y + 2.0));
            v.extend(ncomp(&c, x, y, x + 1.0, y + 1.0));
            v.push(rect(c.lvs_bjt, x - 0.2, y - 0.2, x + 0.5, y + 1.2));
        }
        v
    });
}
