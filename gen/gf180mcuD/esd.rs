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

    hardening(pdk);
}

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.11 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/esd/ESD.<rule>.h<n>.gds.gz` with a case in the
// `hardening_esd` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/esd.md.
//
// What these draw is the deck's own conditions - the implant's enclosure of the N+
// active it protects, the butted P+ active the section allows against it, the gate poly
// the extension is measured from, the Dualgate the implant may only live under and the
// LVS_IO that has to cover the active - at the bound and one step past it.  The generic
// classes (the bound on a bare layer, 45 degrees, unions, arrays) are the engine
// family's.

/// Layers the hardening patterns draw on.
struct H {
    esd: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    dg: (i16, i16),
    lvsio: (i16, i16),
    res: (i16, i16),
}

/// How the base cell is put together: a 6 x 3 N+ active at `(x, y)` with a poly gate
/// across it, the ESD implant reaching `m` past the active on every side, and LVS_IO
/// over the active.  The implant is flush with the active so that ncomp is the active
/// itself and nothing of the implant leaves the marker.
struct C {
    x: f64,
    y: f64,
    /// The marker's reach past the active.
    m: f64,
    /// Gate length and where its left edge sits.
    gate: f64,
    gate_x: f64,
    /// Whether LVS_IO is drawn, and how far it reaches over the active.
    io: Option<f64>,
}

impl C {
    fn at(x: f64, y: f64) -> Self {
        C {
            x,
            y,
            m: 0.8,
            gate: 1.0,
            gate_x: 2.5,
            io: Some(6.0),
        }
    }

    /// The marker's box.
    fn esd_box(&self) -> (f64, f64, f64, f64) {
        (
            self.x - self.m,
            self.y - self.m,
            self.x + 6.0 + self.m,
            self.y + 3.0 + self.m,
        )
    }

    fn draw(&self, h: &H) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let (ex0, ey0, ex1, ey1) = self.esd_box();
        let gx = x + self.gate_x;
        let mut v = vec![
            rect(h.comp, x, y, x + 6.0, y + 3.0),
            rect(h.nplus, x, y, x + 6.0, y + 3.0),
            rect(h.esd, ex0, ey0, ex1, ey1),
            rect(h.poly, gx, y - 0.6, gx + self.gate, y + 3.6),
        ];
        if let Some(w) = self.io {
            v.push(rect(h.lvsio, x - 0.2, y - 0.2, x + w + 0.2, y + 3.2));
        }
        v
    }
}

/// Dualgate over everything the fixture draws: the marker must lie on it (ESD.9) and the
/// gate poly must be a 5 V one (ESD.pl).
fn dualgate(h: &H, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    rect(h.dg, x0, y0, x1, y1)
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let h = H {
        esd: layer(pdk, "esd"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
        dg: layer(pdk, "dualgate"),
        lvsio: layer(pdk, "lvs_io"),
        res: layer(pdk, "res_mk"),
    };
    esd_3a_h(&h);
    esd_3b_h(&h);
    esd_4a_h(&h);
    esd_6_h(&h);
    esd_8_h(&h);
    esd_9_h(&h);
    esd_10_h(&h);
    esd_pl_h(&h);
}

// --- ESD.3a: minimum space to NCOMP 0.6 ---

fn esd_3a_h(h: &H) {
    // h1 - the active the implant does not protect.  Four ESD devices, each with a bare
    // N+ active off its right: (a) 0.595 from the marker, one step under the 0.6: ESD.3a;
    // (b) 0.6: clean; (c) butted against the marker's edge, a space of nothing - ESD.3a,
    // and ESD.8 with it, the 0.3 to an implant being the stricter of the two; (d) an N+
    // active crossing the marker's right edge, half in and half out, which is an active
    // the implant lies on rather than one it stands off.
    let ncomp = |x: f64, y: f64, w: f64| {
        vec![
            rect(h.comp, x, y, x + w, y + 2.0),
            rect(h.nplus, x, y, x + w, y + 2.0),
        ]
    };
    let mut v = C::at(2.0, 2.0).draw(h); // (a) esd right edge 8.8
    v.extend(ncomp(9.395, 2.0, 1.6));
    v.extend(C::at(16.0, 2.0).draw(h)); // (b) esd right edge 22.8
    v.extend(ncomp(23.4, 2.0, 1.6));
    v.extend(C::at(2.0, 10.0).draw(h)); // (c) esd right edge 8.8
    v.extend(ncomp(8.8, 10.0, 2.0));
    let mut d = C::at(16.0, 10.0); // (d) esd right edge 22.8
    d.io = Some(9.0); // LVS_IO over the crossing active too, or ESD.10 speaks as well
    v.extend(d.draw(h));
    v.extend(ncomp(22.0, 10.0, 3.0));
    v.push(dualgate(h, 0.0, 0.0, 27.0, 16.0));
    hwrite("ESD.3a.h1", v);
}

// --- ESD.3b / ESD.7: the butted P+ active ---

fn esd_3b_h(h: &H) {
    // h1 - what "min/max space to a butted PCOMP = 0" allows.  (a) a P+ active butted
    // against the marker's right edge: the space is exactly the 0 the rule asks for, so
    // the butted active is the legal drawing and neither ESD.3b nor ESD.7 - "no ESD
    // implant inside PCOMP" - has anything to say about it.  ESD.8's 0.3 to an implant
    // does, which is the section stating two things about one edge.  (b) the same active
    // moved 0.005 under the marker: the implant is now inside a P+ active, ESD.3b and
    // ESD.7.  (c) the same active 0.005 clear of the marker: a space that is neither 0
    // nor anything the rule names.
    let pcomp = |x: f64, y: f64| {
        vec![
            rect(h.comp, x, y, x + 2.0, y + 2.0),
            rect(h.pplus, x, y, x + 2.0, y + 2.0),
        ]
    };
    let mut v = C::at(2.0, 2.0).draw(h); // (a) butted at 8.8
    v.extend(pcomp(8.8, 2.0));
    v.extend(C::at(16.0, 2.0).draw(h)); // (b) 0.005 under the marker's edge 22.8
    v.extend(pcomp(22.795, 2.0));
    v.extend(C::at(2.0, 10.0).draw(h)); // (c) 0.005 clear of 8.8
    v.extend(pcomp(8.805, 10.0));
    v.push(dualgate(h, 0.0, 0.0, 26.0, 16.0));
    hwrite("ESD.3b.h1", v);
}

// --- ESD.4a: extension beyond NCOMP 0.24 ---

fn esd_4a_h(h: &H) {
    // h1 - the edge a butted P+ active lets off.  Both devices have the marker 0.8 past
    // the active on three sides and 0.235 past it on the right, one step under the 0.24.
    // (a) nothing beside it: ESD.4a on that wall.  (b) a P+ active butted against that
    // same right edge - the edge the implant shares with a P+ active is the one ESD.3b
    // sets to zero space, and an extension is not asked of it, so the wall is clean.
    // ESD.8 speaks for the butted implant in (b), as it does in ESD.3b.h1.
    let narrow = |x: f64, y: f64| {
        let mut c = C::at(x, y);
        c.m = 0.8;
        let mut v = c.draw(h);
        // redraw the marker with a 0.235 right margin
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == h.esd));
        v.push(rect(h.esd, x - 0.8, y - 0.8, x + 6.235, y + 3.8));
        v
    };
    let mut v = narrow(2.0, 2.0); // (a) ESD.4a
    v.extend(narrow(16.0, 2.0)); // (b) the butted P+ active at 22.235
    v.push(rect(h.comp, 22.235, 2.0, 24.235, 4.0));
    v.push(rect(h.pplus, 22.235, 2.0, 24.235, 4.0));
    v.push(dualgate(h, 0.0, 0.0, 26.0, 8.0));
    hwrite("ESD.4a.h1", v);
}

// --- ESD.6: extension perpendicular to the Poly2 gate 0.45 ---

fn esd_6_h(h: &H) {
    // h1 - which poly the extension is measured from.  Four devices, each with a second
    // poly at the active's right end so that its right wall sits on the active's right
    // edge, and the marker reaching `d` past that wall: (a) the second poly lifted off
    // the active into the marker's top margin, so it touches no active and is not a gate
    // - clean at d = 0.445; (b) the second poly crossing the active under RES_MK, which
    // takes it out of the transistor layer - clean at 0.445; (c) the second poly
    // crossing the active bare, a gate: ESD.6 at 0.445; (d) the same gate at 0.45: clean
    // at the bound.  Every device's own gate stands 2.945 or more from the marker.
    let cell = |x: f64, kind: u8| {
        let d = if kind == 3 { 0.45 } else { 0.445 };
        let mut v = vec![
            rect(h.comp, x, 2.0, x + 6.0, 5.0),
            rect(h.nplus, x, 2.0, x + 6.0, 5.0),
            rect(h.esd, x - 0.8, 1.2, x + 6.0 + d, 5.8),
            rect(h.poly, x + 2.5, 1.4, x + 3.5, 5.6), // the device's own gate
            rect(h.lvsio, x - 0.2, 1.8, x + 6.2, 5.2),
        ];
        match kind {
            // in the marker's top margin, clear of the active: not a gate
            0 => v.push(rect(h.poly, x + 5.5, 5.05, x + 6.0, 5.35)),
            // across the active's end under RES_MK: not a transistor
            1 => {
                v.push(rect(h.poly, x + 5.0, 1.4, x + 6.0, 5.6));
                v.push(rect(h.res, x + 4.8, 1.2, x + 6.2, 5.8));
            }
            // across the active's end, bare: a gate
            _ => v.push(rect(h.poly, x + 5.0, 1.4, x + 6.0, 5.6)),
        }
        v
    };
    let mut v = cell(2.0, 0); // (a) clean
    v.extend(cell(14.0, 1)); // (b) clean
    v.extend(cell(26.0, 2)); // (c) ESD.6
    v.extend(cell(38.0, 3)); // (d) clean
    v.push(dualgate(h, 0.0, 0.0, 47.0, 8.0));
    hwrite("ESD.6.h1", v);

    // h2 - the step in the marker.  The gate sits at the active's right end and stops on
    // the active's top edge; the marker's right side steps back from 6.85 to 6.2 above
    // it.  The step's inner corner stands 0.2 in x and 0.05 in y from the gate's upper
    // right corner, so the marker's boundary comes within 0.206 of the gate while no two
    // walls face each other across the step.  The manual asks 0.45 of extension from the
    // gate and this corner gives 0.206, wherever one stands.
    let x = 2.0;
    hwrite(
        "ESD.6.h2",
        vec![
            rect(h.comp, x, 2.0, x + 6.0, 5.0),
            rect(h.nplus, x, 2.0, x + 6.0, 5.0),
            // the marker as an L: the full box up to y = 5.05, then only to x + 6.2
            rect(h.esd, x - 0.8, 1.2, x + 6.85, 5.05),
            rect(h.esd, x - 0.8, 5.05, x + 6.2, 5.8),
            rect(h.poly, x + 2.5, 1.4, x + 3.5, 5.6), // the device's own gate
            rect(h.poly, x + 5.0, 1.4, x + 6.0, 5.0), // the gate at the active's end
            rect(h.lvsio, x - 0.2, 1.8, x + 6.2, 5.2),
            dualgate(h, 0.0, 0.0, 10.0, 7.0),
        ],
    );
}

// --- ESD.8: minimum space to Nplus/Pplus 0.3 ---

fn esd_8_h(h: &H) {
    // h1 - the implant the marker is on, and the one beside it.  (a) the device's own N+
    // drawn 0.1 inside the marker's edge: an implant the marker covers is not an implant
    // it stands off, clean.  (b) an unrelated P+ 0.295 from the marker: ESD.8.  (c) the
    // same at 0.3: clean.  (d) an unrelated N+ crossing the marker's right edge.
    let mut c = C::at(2.0, 2.0); // (a) marker 0.8 past the active
    let mut v = c.draw(h);
    v.push(rect(h.nplus, 1.3, 1.3, 8.7, 5.7)); // 0.1 inside the marker
    c = C::at(16.0, 2.0);
    v.extend(c.draw(h)); // (b) esd right edge 22.8
    v.push(rect(h.pplus, 23.095, 2.0, 25.0, 4.0));
    c = C::at(2.0, 10.0);
    v.extend(c.draw(h)); // (c) esd right edge 8.8
    v.push(rect(h.pplus, 9.1, 10.0, 11.0, 12.0));
    c = C::at(16.0, 10.0);
    v.extend(c.draw(h)); // (d) crossing 22.8
    v.push(rect(h.nplus, 22.0, 10.0, 25.0, 12.0));
    v.push(dualgate(h, 0.0, 0.0, 27.0, 16.0));
    hwrite("ESD.8.h1", v);
}

// --- ESD.9: the implant must be overlapped by Dualgate ---

fn esd_9_h(h: &H) {
    // h1 - how much Dualgate is enough.  (a) Dualgate over the whole marker: clean.
    // (b) Dualgate over the marker's left half - the manual says the implant "must be
    // overlapped by Dualgate", the implant option existing only for the 5 V devices, and
    // the right half of this one is a 3.3 V implant; both decks are content with any
    // overlap at all.  (c) Dualgate abutting the marker's left edge and over none of it:
    // ESD.9.  The gate in (c) is not a 5 V gate, so ESD.pl has nothing to measure.
    let mut v = C::at(2.0, 2.0).draw(h); // (a) esd 1.2..8.8, 1.2..5.8
    v.push(dualgate(h, 0.7, 0.7, 9.3, 6.3));
    v.extend(C::at(16.0, 2.0).draw(h)); // (b) esd 15.2..22.8
    v.push(dualgate(h, 14.7, 0.7, 19.0, 6.3));
    v.extend(C::at(2.0, 10.0).draw(h)); // (c) esd 1.2..8.8, 9.2..13.8
    v.push(dualgate(h, 8.8, 8.7, 12.0, 14.3));
    hwrite("ESD.9.h1", v);
}

// --- ESD.10: LVS_IO shall cover the I/O MOS active area ---

fn esd_10_h(h: &H) {
    // h1 - the marker that has to cover what it touches.  (a) no LVS_IO at all: the rule
    // is about a marker that covers part of an ESD active, not about its absence, clean.
    // (b) LVS_IO over the active's left half: the uncovered half is ESD.10.  (c) LVS_IO
    // abutting the active's right edge and over none of it: nothing of the active is
    // left uncovered by a marker that covers nothing, clean.  (d) LVS_IO over the whole
    // active: clean.
    let mut c = C::at(2.0, 2.0);
    c.io = None;
    let mut v = c.draw(h); // (a)
    c = C::at(16.0, 2.0);
    c.io = Some(3.0);
    v.extend(c.draw(h)); // (b) ESD.10
    c = C::at(2.0, 10.0);
    c.io = None;
    v.extend(c.draw(h)); // (c)
    v.push(rect(h.lvsio, 8.0, 10.0, 10.0, 13.0)); // abutting the active's right edge
    c = C::at(16.0, 10.0);
    v.extend(c.draw(h)); // (d) clean
    v.push(dualgate(h, 0.0, 0.0, 26.0, 16.0));
    hwrite("ESD.10.h1", v);
}

// --- ESD.pl: minimum gate length of a 5 V/6 V NMOS 0.8 ---

fn esd_pl_h(h: &H) {
    // h1 - which gate the rule reaches.  Four 0.795 gates, one step under the 0.8:
    // (a) on an ESD device, under Dualgate: ESD.pl.  (b) on a transistor 2 um clear of
    // the marker: the rule is stated of the 5 V NMOS the implant is on, clean.  (c) on a
    // transistor whose poly touches the marker's edge: the implant reaches it, ESD.pl.
    // (d) on an ESD device with no Dualgate over it: not a 5 V gate, so ESD.pl is
    // silent, and the marker off Dualgate is ESD.9.
    let mut c = C::at(2.0, 2.0);
    c.gate = 0.795;
    let mut v = c.draw(h); // (a) ESD.pl
    v.push(dualgate(h, 0.7, 0.7, 9.3, 6.3));
    // (b) a bare transistor 2 um right of a clean ESD device
    v.extend(C::at(16.0, 2.0).draw(h)); // esd 15.2..22.8
    v.push(rect(h.comp, 24.8, 2.0, 30.8, 5.0));
    v.push(rect(h.nplus, 24.8, 2.0, 30.8, 5.0));
    v.push(rect(h.poly, 27.0, 1.4, 27.795, 5.6));
    v.push(dualgate(h, 14.7, 0.7, 31.5, 6.3));
    // (c) the gate's poly touching the marker's right edge at x = 14.6
    v.push(rect(h.comp, 12.0, 10.0, 18.0, 13.0));
    v.push(rect(h.nplus, 12.0, 10.0, 18.0, 13.0));
    v.push(rect(h.esd, 8.0, 9.0, 14.6, 14.0));
    v.push(rect(h.poly, 14.6, 9.4, 15.395, 13.6));
    v.push(rect(h.lvsio, 11.8, 9.8, 18.2, 13.2));
    v.push(dualgate(h, 7.5, 8.5, 18.5, 14.5));
    // (d) an ESD device with no Dualgate: ESD.9, and no ESD.pl
    let mut d = C::at(24.0, 10.0);
    d.gate = 0.795;
    v.extend(d.draw(h));
    hwrite("ESD.pl.h1", v);
}
