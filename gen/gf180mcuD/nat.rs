// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native-Vt device: a good and a bad pattern for every rule in the `nat` deck.
//!
//! The device is an NMOS whose channel is left unimplanted, so a base cell is a NAT
//! marker over an N+ active with a poly gate across it, clear of any N-well.  Four of the
//! rules are about the marker's own geometry and four say the marker may not sit over the
//! wrong thing, so most fixtures move the marker rather than the transistor.
//!
//! NAT.4 and NAT.5 are the same rule at the two voltages, and what they bound is the
//! channel *length* - the gate's own width where it crosses the active.  The two are told
//! apart by Dualgate, so the 6 V fixture wraps the marker in it and the 3.3 V one does
//! not, which also keeps each quiet on the other's pattern.

use super::OFFSET;
use crate::helpers::{chamfered_tr, layer, library, poly, rect, shift, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/nat";

/// Active area: long enough for a gate and two heads, and 2 µm tall.
const COMP_L: f64 = 8.0;
const COMP_W: f64 = 2.0;
/// The N+ implant's reach past the active.
const NP: f64 = 0.4;
/// How far the NAT marker reaches past the active - NAT.1 asks 2.0.
const NAT_M: f64 = 2.2;
/// Gate length, over NAT.4's 1.8 µm, and how far the gate overhangs the active.
const GATE: f64 = 2.0;
const GATE_EXT: f64 = 0.5;

struct Ctx {
    nat: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    poly: (i16, i16),
    nwell: (i16, i16),
    dualgate: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
    res_mk: (i16, i16),
    pplus: (i16, i16),
}

/// One native transistor, with the margins each rule needs to move.
struct Cell {
    x: f64,
    y: f64,
    /// NAT marker reach past the active.
    nat_m: f64,
    /// Gate length.
    gate: f64,
    /// Whether a gate is drawn at all - NAT.11 wants an active without one.
    gated: bool,
    /// Whether the marker is wrapped in Dualgate, which makes the device 6 V.
    mv: bool,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            nat_m: NAT_M,
            gate: GATE,
            gated: true,
            mv: false,
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let mut v = vec![
            rect(c.comp, x, y, x + COMP_L, y + COMP_W),
            rect(c.nplus, x - NP, y - NP, x + COMP_L + NP, y + COMP_W + NP),
            rect(
                c.nat,
                x - self.nat_m,
                y - self.nat_m,
                x + COMP_L + self.nat_m,
                y + COMP_W + self.nat_m,
            ),
        ];
        if self.gated {
            let gx = x + (COMP_L - self.gate) * 0.5;
            v.push(rect(
                c.poly,
                gx,
                y - GATE_EXT,
                gx + self.gate,
                y + COMP_W + GATE_EXT,
            ));
        }
        if self.mv {
            // Covering the marker entirely, which is what NAT.8 asks of any Dualgate that
            // touches it at all.
            let m = self.nat_m + 0.5;
            v.push(rect(
                c.dualgate,
                x - m,
                y - m,
                x + COMP_L + m,
                y + COMP_W + m,
            ));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        nat: layer(pdk, "nat"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        poly: layer(pdk, "poly2_drawn"),
        nwell: layer(pdk, "nwell"),
        dualgate: layer(pdk, "dualgate"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
        res_mk: layer(pdk, "res_mk"),
        pplus: layer(pdk, "pplus"),
    };
    hardening(&c);
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);
    let mv_clean = || {
        let mut cell = Cell::at(o, o);
        cell.mv = true;
        cell.draw(&c)
    };

    for id in [
        "NAT.1", "NAT.2", "NAT.3", "NAT.4", "NAT.7", "NAT.8", "NAT.9", "NAT.10", "NAT.11", "NAT.12",
    ] {
        write(id, "good", clean());
    }
    // NAT.6: a second active under the same marker with nothing tying it to the first, so
    // the marker covers two potentials.  It sits in the margin the marker already keeps
    // past the active, which is where a second one would be.
    // Its marker is drawn wide enough to hold both actives with NAT.1's two microns to
    // spare, which the default one has no room for.
    let two_actives = |second: bool| {
        let mut cell = Cell::at(o, o);
        cell.nat_m = 7.0;
        let mut v = cell.draw(&c);
        if second {
            // A second device, not merely a second active: NAT.11 asks that every active
            // under the marker have a gate on it.
            let (x0, y0, x1, y1) = (o + 1.0, o + COMP_W + 2.4, o + 7.0, o + COMP_W + 3.6);
            v.push(rect(c.comp, x0, y0, x1, y1));
            v.push(rect(c.nplus, x0 - NP, y0 - NP, x1 + NP, y1 + NP));
            let gx = x0 + (x1 - x0 - GATE) * 0.5;
            v.push(rect(c.poly, gx, y0 - GATE_EXT, gx + GATE, y1 + GATE_EXT));
        }
        v
    };
    write("NAT.6", "good", two_actives(false));
    write("NAT.6", "bad", two_actives(true));

    // NAT.9: poly that belongs to no gate of the marker, closer than 0.3 µm to it.  The
    // rule's other half is a poly interconnect *under* the marker, which needs two gates
    // to run between and so cannot be drawn on a one-gate device.
    let stray_poly = |gap: f64| {
        let mut v = Cell::at(o, o).draw(&c);
        let x = o + COMP_L + NAT_M + gap;
        v.push(rect(c.poly, x, o, x + 1.0, o + COMP_W));
        v
    };
    write("NAT.9", "bad", stray_poly(0.295));

    // NAT.5 is the 6 V rule, so both its halves carry Dualgate.
    write("NAT.5", "good", mv_clean());

    // NAT.1: the marker reaching only 1.995 µm past the active.
    write("NAT.1", "bad", {
        let mut cell = Cell::at(o, o);
        cell.nat_m = 1.995;
        cell.draw(&c)
    });

    // NAT.2: an unrelated active 0.295 µm from the marker.  It carries no N+ and no gate,
    // so it is not a second device.
    write("NAT.2", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.295;
        v.push(rect(c.comp, x, o, x + 2.0, o + COMP_W));
        v
    });

    // NAT.3: an N-well 0.495 µm from the marker.  Clear of it, so NAT.10 - which is about
    // a well *over* the marker - has nothing to say.
    write("NAT.3", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.495;
        v.push(rect(c.nwell, x, o, x + 3.0, o + COMP_W));
        v
    });

    // NAT.4: a 3.3 V channel shorter than 1.8 µm.
    write("NAT.4", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gate = 1.795;
        cell.draw(&c)
    });

    // NAT.5: the same at 6 V, which is the Dualgate one.
    write("NAT.5", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gate = 1.795;
        cell.mv = true;
        cell.draw(&c)
    });

    // NAT.7: a second marker 0.735 µm away.  Bare, so it is only a marker: nothing in it
    // to enclose, no active to want a gate.
    write("NAT.7", "bad", {
        let mut v = clean();
        let x = o + COMP_L + NAT_M + 0.735;
        v.push(rect(c.nat, x, o, x + 3.0, o + COMP_W));
        v
    });

    // NAT.8: Dualgate over part of the marker but not all of it.  A marker it touches at
    // all it has to cover.
    write("NAT.8", "bad", {
        let mut v = clean();
        v.push(rect(
            c.dualgate,
            o + COMP_L * 0.5,
            o - NAT_M - 1.0,
            o + COMP_L + NAT_M + 1.0,
            o + COMP_W + NAT_M + 1.0,
        ));
        v
    });

    // NAT.10: an N-well over a corner of the marker, away from the active - a well *under*
    // the active would leave nothing for NAT.1 to measure.
    write("NAT.10", "bad", {
        let mut v = clean();
        v.push(rect(
            c.nwell,
            o - NAT_M - 1.0,
            o - NAT_M - 1.0,
            o - NAT_M + 1.0,
            o - NAT_M + 1.0,
        ));
        v
    });

    // NAT.11: the active in the marker with no gate across it.
    write("NAT.11", "bad", {
        let mut cell = Cell::at(o, o);
        cell.gated = false;
        cell.draw(&c)
    });

    // NAT.12: poly over the marker that reaches no active at all.
    write("NAT.12", "bad", {
        let mut v = clean();
        v.push(rect(
            c.poly,
            o - NAT_M + 0.2,
            o - NAT_M + 0.2,
            o - NAT_M + 1.2,
            o - 0.6,
        ));
        v
    });
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from section 10.5 of the manual, at the bound each rule names and one
// 0.005 µm step past it, with a case in the `hardening_nat` table of
// `tests/gf180mcuD.rs` and the findings in hardening/reports/gf180mcuD/nat.md.
//
// A native transistor is an N+ active clear of every well, a poly gate across it, and a
// NAT marker over the pair; eight of the twelve rules read the marker against something
// else and four read what is under it.  So the fixtures are rows of whole devices, each
// bent one way, and the probes that measure against the marker are parked outside it -
// a bar of anything left under the marker answers to NAT.11 or NAT.12 rather than to the
// rule it was drawn for.
//
// The generic classes (a bound on a bare layer, unions, arrays, a shape far off) are the
// engine family's and are not redrawn; what is drawn here is the euclidian corner, the
// shared edge, the shape that crosses out of its marker, and the tile lines.

/// How far apart the devices of one hardening row stand.  The widest marker drawn here is
/// 15 µm across, so this leaves 15 µm between markers - twenty times NAT.7's 0.74.
const H_PITCH: f64 = 30.0;

/// One native transistor: an N+ active `w × h`, a poly gate across it, and a NAT marker
/// `m` past the active on every side.
#[derive(Clone, Copy)]
struct HDev {
    w: f64,
    h: f64,
    /// The marker's reach past the active, left / bottom / right / top.  NAT.1 asks 2.
    m: (f64, f64, f64, f64),
    /// Gate length - the channel, which NAT.4 and NAT.5 hold at 1.8.
    gate: f64,
    gated: bool,
    /// Whether Dualgate covers the marker, which makes the device the 6 V kind.
    mv: bool,
}

impl Default for HDev {
    fn default() -> Self {
        HDev {
            w: COMP_L,
            h: COMP_W,
            m: (2.2, 2.2, 2.2, 2.2),
            gate: 2.0,
            gated: true,
            mv: false,
        }
    }
}

impl HDev {
    fn draw(&self, c: &Ctx, x: f64, y: f64) -> Vec<GdsElement> {
        let (ml, mb, mr, mt) = self.m;
        let (x1, y1) = (x + self.w, y + self.h);
        let mut v = vec![
            rect(c.comp, x, y, x1, y1),
            rect(c.nplus, x - NP, y - NP, x1 + NP, y1 + NP),
            rect(c.nat, x - ml, y - mb, x1 + mr, y1 + mt),
        ];
        if self.gated {
            // The gate's left edge is fixed rather than centred, so that bending the
            // channel by one 0.005 µm step leaves every vertex on the grid.
            let gx = x + 3.0;
            v.push(rect(
                c.poly,
                gx,
                y - GATE_EXT,
                gx + self.gate,
                y1 + GATE_EXT,
            ));
        }
        if self.mv {
            // Covering the marker whole, which is what NAT.8 asks of a 6 V device.
            v.push(rect(
                c.dualgate,
                x - ml - 0.5,
                y - mb - 0.5,
                x1 + mr + 0.5,
                y1 + mt + 0.5,
            ));
        }
        v
    }
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A row of devices `H_PITCH` apart, each with the extra shapes its own probe needs,
/// drawn in the device's own frame and shifted with it.
fn hrow(c: &Ctx, items: Vec<(HDev, Vec<GdsElement>)>) -> Vec<GdsElement> {
    let mut v = Vec::new();
    for (i, (d, extra)) in items.into_iter().enumerate() {
        let dx = i as f64 * H_PITCH;
        v.extend(shift(&d.draw(c, OFFSET, OFFSET), dx, 0.0));
        v.extend(shift(&extra, dx, 0.0));
    }
    v
}

fn hardening(c: &Ctx) {
    let o = OFFSET;
    let good = HDev::default();
    // The device's own edges, for the probes that measure off them: the active runs
    // x = o .. o + COMP_L and the default marker 2.2 further out on every side.
    let mr = o + COMP_L + 2.2;

    // --- NAT.1: the marker's two microns of the active.  An enclosure is euclidian and
    // has to have something to say about an active that runs out through the marker's
    // edge, which is held by nothing at all.
    hwrite(
        "NAT.1.h1",
        hrow(
            c,
            vec![
                // Exactly 2.0 on every side: clean.
                (
                    HDev {
                        m: (2.0, 2.0, 2.0, 2.0),
                        ..good
                    },
                    vec![],
                ),
                // 1.995 on the right wall alone: one violation, on that wall.
                (
                    HDev {
                        m: (2.2, 2.2, 1.995, 2.2),
                        ..good
                    },
                    vec![],
                ),
                // A tab of the active running out through the marker's right edge: the
                // marker holds it by nothing at all.
                (
                    good,
                    vec![
                        rect(c.comp, o + COMP_L, o + 0.5, mr + 1.0, o + 1.5),
                        rect(c.nplus, o + COMP_L, o + 0.5 - NP, mr + 1.0, o + 1.5 + NP),
                    ],
                ),
            ],
        ),
    );
    // The same enclosure read across a chamfer: the marker's top-right corner is cut
    // along x + y = k so the active's corner stands 1.994 µm from the 45° wall while both
    // axis distances are still 2.2.  Euclidian fires, projection does not.
    hwrite("NAT.1.h2", {
        let (x1, y1) = (o + COMP_L + 2.2, o + COMP_W + 2.2);
        let gx = o + 3.0;
        vec![
            rect(c.comp, o, o, o + COMP_L, o + COMP_W),
            rect(c.nplus, o - NP, o - NP, o + COMP_L + NP, o + COMP_W + NP),
            chamfered_tr(c.nat, o - 2.2, o - 2.2, x1, y1, x1 + y1 - 1.58),
            rect(c.poly, gx, o - GATE_EXT, gx + 2.0, o + COMP_W + GATE_EXT),
        ]
    });

    // --- NAT.2: 0.3 µm from the marker to an active that is not its own.  The probe
    // carries no N+, so it is not a second native device and NAT.11 has nothing to say.
    hwrite(
        "NAT.2.h1",
        hrow(
            c,
            vec![
                // 0.3 exactly: clean.
                (good, vec![rect(c.comp, mr + 0.3, o, mr + 1.3, o + COMP_W)]),
                // 0.295.
                (
                    good,
                    vec![rect(c.comp, mr + 0.295, o, mr + 1.295, o + COMP_W)],
                ),
                // Sharing the marker's edge: a space of nothing.
                (good, vec![rect(c.comp, mr, o, mr + 1.0, o + COMP_W)]),
                // Corner to corner: 0.22 in x and 0.2 in y, so 0.2966 euclidian with both
                // axis distances over the value - euclidian fires, projection does not.
                (
                    good,
                    vec![rect(
                        c.comp,
                        mr + 0.22,
                        o + COMP_W + 2.2 + 0.2,
                        mr + 1.22,
                        o + COMP_W + 3.4,
                    )],
                ),
            ],
        ),
    );

    // --- NAT.3: 0.5 µm from the marker to an N-well edge.  A well that reaches over the
    // marker is NAT.10's, so the probes stop at its edge.
    hwrite(
        "NAT.3.h1",
        hrow(
            c,
            vec![
                (good, vec![rect(c.nwell, mr + 0.5, o, mr + 3.5, o + COMP_W)]),
                (
                    good,
                    vec![rect(c.nwell, mr + 0.495, o, mr + 3.5, o + COMP_W)],
                ),
                // Sharing the marker's edge.
                (good, vec![rect(c.nwell, mr, o, mr + 3.0, o + COMP_W)]),
            ],
        ),
    );

    // --- NAT.4 / NAT.5: the channel, which is the gate's own width where it crosses the
    // active.  Both rules ask 1.8 µm; Dualgate over the marker picks which id reads it.
    hwrite(
        "NAT.4.h1",
        hrow(
            c,
            vec![
                // 1.8 at 3.3 V: clean.
                (HDev { gate: 1.8, ..good }, vec![]),
                // 1.795 at 3.3 V: NAT.4.
                (
                    HDev {
                        gate: 1.795,
                        ..good
                    },
                    vec![],
                ),
                // 1.8 at 6 V: clean.
                (
                    HDev {
                        gate: 1.8,
                        mv: true,
                        ..good
                    },
                    vec![],
                ),
                // 1.795 at 6 V: NAT.5.
                (
                    HDev {
                        gate: 1.795,
                        mv: true,
                        ..good
                    },
                    vec![],
                ),
            ],
        ),
    );
    // A 1.795 channel under a marker Dualgate only *abuts*: no thick oxide lies over the
    // device, so it is the 3.3 V one and NAT.4 is its rule.
    hwrite("NAT.4.h2", {
        let mut v = HDev {
            gate: 1.795,
            ..good
        }
        .draw(c, o, o);
        v.push(rect(c.dualgate, mr, o - 2.2, mr + 4.0, o + COMP_W + 2.2));
        v
    });

    // --- NAT.6: two actives at two potentials under one marker.  The first pair is left
    // unconnected, the second is tied by a Metal1 strap over a contact in each.
    let two_actives = |tied: bool| {
        // The marker keeps NAT.1's two microns of both actives, so nothing but NAT.6 has
        // anything to say about the pair.
        let d = HDev {
            m: (2.2, 2.2, 2.2, 5.4),
            ..good
        };
        let mut v = d.draw(c, o, o);
        let (x0, y0, x1, y1) = (o + 1.0, o + COMP_W + 2.2, o + 7.0, o + COMP_W + 3.4);
        v.push(rect(c.comp, x0, y0, x1, y1));
        v.push(rect(c.nplus, x0 - NP, y0 - NP, x1 + NP, y1 + NP));
        // Its own gate, so NAT.11 has nothing to say about it.
        v.push(rect(
            c.poly,
            x0 + 2.0,
            y0 - GATE_EXT,
            x0 + 4.0,
            y1 + GATE_EXT,
        ));
        // A 0.22 contact in each active, and - when the two are one node - one Metal1
        // plate over both.
        let a = (o + 0.6, o + 0.6);
        let b = (x0 + 0.6, y0 + 0.6);
        for (cx, cy) in [a, b] {
            v.push(rect(c.contact, cx - 0.11, cy - 0.11, cx + 0.11, cy + 0.11));
        }
        let plate = |p: (f64, f64), q: (f64, f64)| {
            rect(c.metal1, p.0 - 0.31, p.1 - 0.31, q.0 + 0.31, q.1 + 0.31)
        };
        if tied {
            v.push(plate(a, b));
        } else {
            v.push(plate(a, a));
            v.push(plate(b, b));
        }
        v
    };
    hwrite("NAT.6.h1", two_actives(false));
    hwrite("NAT.6.h2", two_actives(true));

    // --- NAT.7: 0.74 µm between markers.  Bare markers, so nothing under them answers
    // for anything else, and every gap is centred on a tile line: 20, 21, 40, 42 and
    // y = 20.
    hwrite("NAT.7.h1", {
        let bar = |x0: f64, y0: f64, x1: f64, y1: f64| rect(c.nat, x0, y0, x1, y1);
        vec![
            // x = 20: 0.74 exactly, clean.
            bar(14.0, 5.0, 19.63, 8.0),
            bar(20.37, 5.0, 26.0, 8.0),
            // x = 21: 0.735.
            bar(14.0, 10.0, 20.635, 13.0),
            bar(21.37, 10.0, 27.0, 13.0),
            // x = 40: 0.735.
            bar(34.0, 5.0, 39.635, 8.0),
            bar(40.37, 5.0, 46.0, 8.0),
            // x = 42: a 0.735 slot cut into one marker, crossing the line at x = 42.
            poly(
                c.nat,
                &[
                    (41.635, 10.0),
                    (48.0, 10.0),
                    (48.0, 15.0),
                    (41.635, 15.0),
                    (41.635, 12.37),
                    (46.0, 12.37),
                    (46.0, 11.635),
                    (41.635, 11.635),
                ],
            ),
            // y = 20: 0.735 across the horizontal tile line.
            bar(60.0, 14.0, 66.0, 19.635),
            bar(60.0, 20.37, 66.0, 26.0),
        ]
    });

    // --- NAT.8: a 6 V native device has to lie under Dualgate whole.  A marker Dualgate
    // merely abuts is a 3.3 V device and has no Dualgate overlap to measure.
    hwrite(
        "NAT.8.h1",
        hrow(
            c,
            vec![
                // Covered whole: clean.
                (HDev { mv: true, ..good }, vec![]),
                // Covered from the middle rightwards: the left half is uncovered.
                (
                    good,
                    vec![rect(
                        c.dualgate,
                        o + COMP_L * 0.5,
                        o - 2.7,
                        mr + 0.5,
                        o + COMP_W + 2.7,
                    )],
                ),
                // Abutting the marker's right edge and covering no part of it.
                (
                    good,
                    vec![rect(c.dualgate, mr, o - 2.2, mr + 4.0, o + COMP_W + 2.2)],
                ),
            ],
        ),
    );

    // --- NAT.9: unrelated poly 0.3 µm off the marker, and a poly interconnect under it.
    hwrite(
        "NAT.9.h1",
        hrow(
            c,
            vec![
                (good, vec![rect(c.poly, mr + 0.3, o, mr + 1.3, o + COMP_W)]),
                (
                    good,
                    vec![rect(c.poly, mr + 0.295, o, mr + 1.295, o + COMP_W)],
                ),
                // Sharing the marker's edge.
                (good, vec![rect(c.poly, mr, o, mr + 1.0, o + COMP_W)]),
            ],
        ),
    );
    // Two gates on one active joined over the field by a poly bridge, all under one
    // marker: poly that runs from one gate of the marker to another is an interconnect.
    // One active, so the two gates are at one potential and NAT.6 is quiet.
    hwrite("NAT.9.h2", {
        let d = HDev {
            gated: false,
            m: (2.2, 2.2, 2.2, 3.2),
            ..good
        };
        let mut v = d.draw(c, o, o);
        let top = o + COMP_W + GATE_EXT;
        v.push(rect(c.poly, o + 1.0, o - GATE_EXT, o + 2.9, top));
        v.push(rect(c.poly, o + 5.1, o - GATE_EXT, o + 7.0, top));
        // The bridge runs clear of the active, so it is field poly joining two gates and
        // not part of either of them.
        v.push(rect(c.poly, o + 1.0, o + COMP_W + 0.2, o + 7.0, top));
        v
    });

    // --- NAT.10: an N-well over the marker.  One corner of the well inside it, one well
    // wholly inside it, and one abutting its edge - which is NAT.3's zero space and not a
    // well *inside* the marker at all.
    hwrite(
        "NAT.10.h1",
        hrow(
            c,
            vec![
                (
                    good,
                    vec![rect(
                        c.nwell,
                        mr - 1.0,
                        o + COMP_W + 1.2,
                        mr + 2.0,
                        o + COMP_W + 4.2,
                    )],
                ),
                (
                    good,
                    vec![rect(
                        c.nwell,
                        mr - 2.0,
                        o + COMP_W + 0.6,
                        mr - 0.4,
                        o + COMP_W + 2.0,
                    )],
                ),
                (
                    good,
                    vec![rect(
                        c.nwell,
                        mr,
                        o + COMP_W + 0.6,
                        mr + 3.0,
                        o + COMP_W + 2.0,
                    )],
                ),
            ],
        ),
    );

    // --- NAT.11: an N+ active under the marker with no poly on it.  The second device's
    // gate only abuts its active; upstream and the deck both read that as intersecting.
    // The third probe is a P+ active, which is not an NCOMP and not this rule's.
    hwrite(
        "NAT.11.h1",
        hrow(
            c,
            vec![
                (
                    HDev {
                        gated: false,
                        ..good
                    },
                    vec![],
                ),
                (
                    HDev {
                        gated: false,
                        ..good
                    },
                    vec![rect(c.poly, o + 3.0, o + COMP_W, o + 5.0, o + COMP_W + 1.5)],
                ),
                (
                    good,
                    vec![
                        rect(c.comp, o + 1.0, o + COMP_W + 0.6, o + 4.0, o + COMP_W + 1.6),
                        rect(
                            c.pplus,
                            o + 0.6,
                            o + COMP_W + 0.2,
                            o + 4.4,
                            o + COMP_W + 2.0,
                        ),
                    ],
                ),
            ],
        ),
    );

    // --- NAT.12: poly under the marker that reaches no active, and poly under the marker
    // that does reach one but carries RES_MK.  The second probe touches the active at a
    // single point, which both decks read as intersecting.
    hwrite(
        "NAT.12.h1",
        hrow(
            c,
            vec![
                (
                    good,
                    vec![rect(
                        c.poly,
                        o + 1.0,
                        o + COMP_W + 0.6,
                        o + 3.0,
                        o + COMP_W + 1.6,
                    )],
                ),
                (
                    good,
                    vec![rect(
                        c.poly,
                        o + COMP_L,
                        o + COMP_W,
                        o + COMP_L + 2.0,
                        o + COMP_W + 1.0,
                    )],
                ),
                (
                    good,
                    vec![
                        rect(c.res_mk, o + 1.0, o - 0.2, o + 3.0, o + COMP_W + 0.2),
                        rect(
                            c.poly,
                            o + 1.0,
                            o - GATE_EXT,
                            o + 3.0,
                            o + COMP_W + GATE_EXT,
                        ),
                    ],
                ),
            ],
        ),
    );
}
