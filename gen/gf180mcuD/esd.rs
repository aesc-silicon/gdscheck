// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ESD implant: a good and a bad pattern for every rule in the `esd` deck.
//!
//! The base cell is an ESD NMOS: an N+ active with a poly gate across it, the whole thing
//! under Dualgate and marked by ESD.  Three of the rules are constraints on the marker
//! itself rather than distances - it must lie on Dualgate (ESD.9), it must never lie on a
//! P+ active (ESD.3b and ESD.7, which are the same statement twice), and the active under
//! it must be marked for I/O (ESD.10) - so every fixture satisfies all three before it
//! breaks anything else.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/esd";

/// Active area under the marker.
const COMP_L: f64 = 6.0;
const COMP_W: f64 = 3.0;
/// Gate length, over ESD.pl's 0.8 µm, and its overhang past the active.
const GATE: f64 = 1.0;
const GATE_EXT: f64 = 0.6;
/// How far the ESD marker reaches past the active - ESD.4a asks 0.24 of the N+ active's
/// edge and ESD.6 asks 0.45 of the gate's.
const ESD_M: f64 = 0.8;

struct Ctx {
    esd: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    dualgate: (i16, i16),
    lvs_io: (i16, i16),
}

/// How far the I/O marker reaches over the ESD active.
#[derive(Clone, Copy)]
enum Io {
    Whole,
    Half,
}

struct Cell {
    x: f64,
    y: f64,
    /// The marker's reach past the active.
    esd_m: f64,
    /// Gate length.
    gate: f64,
    /// Where the gate's left edge sits, if not centred on the active.  ESD.6 measures the
    /// marker's edges against the gate's *side walls*, so its fixture slides the gate
    /// toward one end rather than shrinking the marker - which would break the rules that
    /// measure the marker against the active instead.
    gate_x: Option<f64>,
    /// Whether Dualgate is drawn over the marker - ESD.9 wants it.
    dg: bool,
    /// How the I/O marker covers the active.  ESD.10 is a coverage rule, not a presence
    /// one: an ESD active that the marker touches at all it has to cover, so leaving the
    /// marker out entirely is legal and covering half of it is not.
    io: Io,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            esd_m: ESD_M,
            gate: GATE,
            gate_x: None,
            dg: true,
            io: Io::Whole,
        }
    }

    /// The ESD marker's box, which several fixtures measure their neighbours from.
    fn esd_box(&self) -> (f64, f64, f64, f64) {
        (
            self.x - self.esd_m,
            self.y - self.esd_m,
            self.x + COMP_L + self.esd_m,
            self.y + COMP_W + self.esd_m,
        )
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let (ex0, ey0, ex1, ey1) = self.esd_box();
        let mut v = vec![
            rect(c.comp, x, y, x + COMP_L, y + COMP_W),
            rect(
                c.nplus,
                x - 0.4,
                y - 0.4,
                x + COMP_L + 0.4,
                y + COMP_W + 0.4,
            ),
            rect(c.esd, ex0, ey0, ex1, ey1),
        ];
        let gx = self.gate_x.unwrap_or(x + (COMP_L - self.gate) * 0.5);
        v.push(rect(
            c.poly,
            gx,
            y - GATE_EXT,
            gx + self.gate,
            y + COMP_W + GATE_EXT,
        ));
        if self.dg {
            // Wide enough to cover the second markers some fixtures add beside the cell.
            // ESD.9 asks that *every* marker lie on Dualgate, so one drawn outside it
            // would break that rule as well as the one the fixture is for.
            let m = 8.0;
            v.push(rect(c.dualgate, ex0 - m, ey0 - m, ex1 + m, ey1 + m));
        }
        match self.io {
            Io::Whole => v.push(rect(
                c.lvs_io,
                x - 0.2,
                y - 0.2,
                x + COMP_L + 0.2,
                y + COMP_W + 0.2,
            )),
            Io::Half => v.push(rect(
                c.lvs_io,
                x - 0.2,
                y - 0.2,
                x + COMP_L * 0.5,
                y + COMP_W + 0.2,
            )),
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        esd: layer(pdk, "esd"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
        dualgate: layer(pdk, "dualgate"),
        lvs_io: layer(pdk, "lvs_io"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);

    for id in [
        "ESD.1", "ESD.2", "ESD.3a", "ESD.3b", "ESD.4a", "ESD.4b", "ESD.5a", "ESD.5b", "ESD.6",
        "ESD.7", "ESD.8", "ESD.9", "ESD.10", "ESD.pl",
    ] {
        write(id, "good", clean());
    }

    // ESD.1: the marker narrower than 0.6 µm.  A bare marker, since a device under one
    // this thin could not satisfy the rules that measure from it.
    write("ESD.1", "bad", {
        let mut v = clean();
        let (_, _, ex1, _) = Cell::at(o, o).esd_box();
        v.push(rect(c.esd, ex1 + 2.0, o, ex1 + 2.0 + 0.595, o + 2.0));
        v
    });

    // ESD.2: a second marker 0.6 µm away.
    write("ESD.2", "bad", {
        let mut v = clean();
        let (_, ey0, ex1, ey1) = Cell::at(o, o).esd_box();
        v.push(rect(c.esd, ex1 + 0.595, ey0, ex1 + 0.595 + 2.0, ey1));
        v
    });

    // ESD.3a: an N+ active outside the marker, within 0.6 µm of it.
    write("ESD.3a", "bad", {
        let mut v = clean();
        let (_, ey0, ex1, _) = Cell::at(o, o).esd_box();
        let x = ex1 + 0.595;
        v.push(rect(c.comp, x, ey0, x + 2.0, ey0 + 2.0));
        v.push(rect(c.nplus, x - 0.1, ey0 - 0.1, x + 2.1, ey0 + 2.1));
        v
    });

    // ESD.3b and ESD.7 are the same statement - the marker may not lie on a P+ active -
    // so one fixture answers both and the harness reads them as the twins they are.
    let on_pcomp = || {
        let mut v = clean();
        let (ex0, ey0, _, _) = Cell::at(o, o).esd_box();
        // In the strip between the marker's edge and the I/O marker's, so this active is
        // under the ESD marker without touching the I/O one - an ESD active the I/O
        // marker covers only partly is ESD.10's violation, not this one's.
        v.push(rect(c.comp, ex0 + 0.05, ey0 + 0.05, ex0 + 0.55, ey0 + 0.55));
        v.push(rect(c.pplus, ex0, ey0, ex0 + 0.6, ey0 + 0.6));
        v
    };
    write("ESD.3b", "bad", on_pcomp());
    write("ESD.7", "bad", on_pcomp());

    // ESD.4a: the marker's edge only 0.24 µm outside the N+ active's.
    write("ESD.4a", "bad", {
        let mut cell = Cell::at(o, o);
        cell.esd_m = 0.235;
        cell.draw(&c)
    });

    // ESD.4b: the marker overlapping the active by under 0.45 µm.  It sits off the end of
    // the active rather than over it, so the overlap is a strip that narrow.
    write("ESD.4b", "bad", {
        let mut v = clean();
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.esd));
        v.push(rect(
            c.esd,
            o + COMP_L - 0.445,
            o - ESD_M,
            o + COMP_L + 3.0,
            o + COMP_W + ESD_M,
        ));
        v
    });

    // ESD.5a / ESD.5b: the marker under 0.49 µm², and a hole in it under 0.49 µm².
    write("ESD.5a", "bad", {
        let mut v = clean();
        let (_, _, ex1, _) = Cell::at(o, o).esd_box();
        // 0.69² = 0.4761 µm², under the limit; 0.7² would be exactly on it.
        v.push(rect(c.esd, ex1 + 2.0, o, ex1 + 2.69, o + 0.69));
        v
    });
    write("ESD.5b", "bad", {
        let mut v = clean();
        let (_, _, ex1, _) = Cell::at(o, o).esd_box();
        let x = ex1 + 2.0;
        // A ring whose opening is 0.69 x 0.69 - 0.4761 µm², under the 0.49 the rule asks
        // of a hole, where 0.7 x 0.7 would sit exactly on it.
        for (a, b, d, e) in [
            (0.0, 0.0, 2.69, 1.0),
            (0.0, 1.69, 2.69, 2.69),
            (0.0, 1.0, 1.0, 1.69),
            (1.69, 1.0, 2.69, 1.69),
        ] {
            v.push(rect(c.esd, x + a, o + b, x + d, o + e));
        }
        v
    });

    // ESD.6: the marker's edge only 0.45 µm outside the gate's.
    write("ESD.6", "bad", {
        // Drawn out rather than from the base cell.  The layer this rule measures against
        // is the poly walls that *touch* the gate, so the wall has to stay on the active -
        // sliding the gate off its end takes that wall out of the layer instead of moving
        // it closer.  So the gate sits at the active's end and the marker comes in to
        // 0.445 µm of it, and the implant is drawn flush to the active so that the marker
        // closing in does not break ESD.4a on the way.
        let m = 0.445;
        let gx = o + COMP_L - GATE;
        vec![
            rect(c.comp, o, o, o + COMP_L, o + COMP_W),
            rect(c.nplus, o, o, o + COMP_L, o + COMP_W),
            rect(c.esd, o - m, o - m, o + COMP_L + m, o + COMP_W + m),
            rect(c.poly, gx, o - GATE_EXT, gx + GATE, o + COMP_W + GATE_EXT),
            rect(
                c.dualgate,
                o - 9.0,
                o - 9.0,
                o + COMP_L + 9.0,
                o + COMP_W + 9.0,
            ),
            rect(
                c.lvs_io,
                o - 0.2,
                o - 0.2,
                o + COMP_L + 0.2,
                o + COMP_W + 0.2,
            ),
        ]
    });

    // ESD.8: an implant outside the marker, within 0.3 µm of it.
    write("ESD.8", "bad", {
        let mut v = clean();
        let (_, ey0, ex1, _) = Cell::at(o, o).esd_box();
        let x = ex1 + 0.295;
        v.push(rect(c.pplus, x, ey0, x + 2.0, ey0 + 2.0));
        v
    });

    // ESD.9: the marker with no Dualgate over it.
    write("ESD.9", "bad", {
        let mut cell = Cell::at(o, o);
        cell.dg = false;
        cell.draw(&c)
    });

    // ESD.10: the I/O marker over half the ESD active.  Leaving it out entirely would be
    // legal - the rule is about a marker that covers part of one, not about its absence.
    write("ESD.10", "bad", {
        let mut cell = Cell::at(o, o);
        cell.io = Io::Half;
        cell.draw(&c)
    });

    // ESD.pl: a gate shorter than 0.8 µm.
    write("ESD.pl", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gate = 0.795;
        cell.draw(&c)
    });
}
