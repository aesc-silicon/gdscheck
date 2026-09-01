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
}
