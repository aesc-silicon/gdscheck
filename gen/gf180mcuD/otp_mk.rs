// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! OTP marker: a good and a bad pattern for every rule in the `otp_mk` deck.
//!
//! Every layer this deck reads is some drawn layer cut against the OTP marker, so every
//! fixture is a marker with the shapes under test inside it and nothing else.
//!
//! The gate stripes run horizontally throughout.  O.PL.ORT forbids the other orientation
//! outright - it keeps the channel edges that are not horizontal - so a vertical gate
//! would trip it and no other fixture could be about only its own rule.

use super::OFFSET;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/otp_mk";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    hardening(pdk);
    let otp = layer(pdk, "otp_mk");
    let comp = layer(pdk, "comp");
    let poly = layer(pdk, "poly2_drawn");
    let sab = layer(pdk, "sab");
    let o = OFFSET;

    let write = |id: &str, polarity: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // --- O.PL.2: the gate length, measured between the poly's own two walls -----------
    //
    // Upstream pairs `poly.edges and tgate.edges` — the poly's sidewalls where it crosses
    // the active — and the rule is the distance *through the poly* between them.  Here it
    // is the poly's width masked to the gate: the same two walls, since a poly stripe has
    // walls in one direction only.  The poly region is 3 um long and the gate 0.22 wide,
    // so the width and not the length is what the mask keeps.
    //
    // The COMP is drawn wide enough that the poly's overhang clears O.DF.6's 0.22 um and
    // the COMP's own reach clears O.PL.4's 0.14, so nothing but the width is under test.
    // The stripe runs *horizontally*: O.PL.ORT in this same deck forbids the other
    // orientation outright, so a vertical gate would trip it and the width would never be
    // the only thing under test.
    let gate = |w: f64| {
        vec![
            rect(otp, o - 2.0, o - 2.0, o + 6.0, o + 6.0),
            rect(comp, o, o, o + 1.6, o + 3.0),
            rect(poly, o - 0.6, o + 1.2, o + 2.2, o + 1.2 + w),
        ]
    };
    write("O.PL.2", "good", gate(0.22));
    write("O.PL.2", "bad", gate(0.22 - 0.005));

    // --- O.SB.11: how deeply the salicide block covers the COMP it blocks -------------
    //
    // The block overlaps the COMP from one side by `depth`, which is the overlap the rule
    // measures — not a spacing and not an enclosure, since neither shape contains the
    // other. Both are large enough to clear O.SB.13's area minimum.
    let block = |depth: f64| {
        vec![
            rect(otp, o - 2.0, o - 2.0, o + 8.0, o + 8.0),
            rect(comp, o, o, o + 3.0, o + 2.0),
            rect(sab, o + 3.0 - depth, o - 0.3, o + 6.0, o + 2.3),
        ]
    };
    write("O.SB.11", "good", block(0.04));
    write("O.SB.11", "bad", block(0.04 - 0.005));

    let v5 = layer(pdk, "v5_xtor");
    let contact = layer(pdk, "contact");
    let marker = || rect(otp, o - 3.0, o - 3.0, o + 9.0, o + 9.0);

    // --- the two enclosures across a gate --------------------------------------------
    //
    // The poly reaches past the active left and right, the active reaches past the poly
    // above and below, and each rule is measured on the axis its own layer overhangs.
    let cross = |over_poly: f64, over_comp: f64| {
        vec![
            marker(),
            rect(comp, o, o + 1.2 - over_comp, o + 1.6, o + 1.42 + over_comp),
            rect(poly, o - over_poly, o + 1.2, o + 1.6 + over_poly, o + 1.42),
        ]
    };
    write("O.DF.6", "good", cross(0.6, 0.4));
    write("O.DF.6", "bad", cross(0.6, 0.215));
    write("O.PL.4", "good", cross(0.6, 0.4));
    write("O.PL.4", "bad", cross(0.135, 0.4));

    // --- the active on its own --------------------------------------------------------
    for (polarity, gap) in [("good", 0.5), ("bad", 0.235)] {
        write(
            "O.DF.3a",
            polarity,
            vec![
                marker(),
                rect(comp, o, o, o + 1.0, o + 1.0),
                rect(comp, o + 1.0 + gap, o, o + 2.0 + gap, o + 1.0),
            ],
        );
    }
    for (polarity, w) in [("good", 0.5), ("bad", 0.37)] {
        write(
            "O.DF.9",
            polarity,
            vec![marker(), rect(comp, o, o, o + w, o + w)],
        );
    }

    // --- the poly on its own ----------------------------------------------------------
    for (polarity, gap) in [("good", 0.4), ("bad", 0.175)] {
        write(
            "O.PL.3a",
            polarity,
            vec![
                marker(),
                rect(poly, o, o, o + 2.0, o + 0.3),
                rect(poly, o, o + 0.3 + gap, o + 2.0, o + 0.6 + gap),
            ],
        );
    }

    // --- the salicide block -----------------------------------------------------------
    //
    // Every block here overlaps an active by more than O.SB.11's 0.04 µm and is larger
    // than O.SB.13's area, since those two rules read every block in the marker.
    let sab_on_comp = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            rect(comp, x - 0.5, y + 0.2, x + 0.5, y + h - 0.2),
            rect(sab, x, y, x + w, y + h),
        ]
    };
    for (polarity, gap) in [("good", 0.5), ("bad", 0.275)] {
        let mut v = vec![marker()];
        v.extend(sab_on_comp(o, o, 1.5, 1.5));
        v.extend(sab_on_comp(o + 1.5 + gap, o, 1.5, 1.5));
        write("O.SB.2", polarity, v);
    }
    for (polarity, a) in [("good", 1.6), ("bad", 1.48)] {
        let mut v = vec![marker()];
        v.extend(sab_on_comp(o, o, 1.2, a / 1.2));
        write("O.SB.13_LV", polarity, v);
    }
    for (polarity, a) in [("good", 2.2), ("bad", 1.99)] {
        let mut v = vec![marker()];
        v.extend(sab_on_comp(o, o, 1.4, a / 1.4));
        v.push(rect(v5, o - 0.5, o - 0.5, o + 3.0, o + 3.0));
        write("O.SB.13_MV", polarity, v);
    }

    // O.SB.3: a block that covers no active, beside an active no block covers.
    for (polarity, gap) in [("good", 0.4), ("bad", 0.085)] {
        write(
            "O.SB.3",
            polarity,
            vec![
                marker(),
                rect(sab, o, o, o + 1.5, o + 1.5),
                rect(comp, o + 1.5 + gap, o, o + 2.5 + gap, o + 1.5),
            ],
        );
    }

    // O.SB.5b_LV: a block beside a gate it does not cover.  The gate reaches the active's
    // own edge, so the distance to the gate is the distance to the active - which keeps
    // this outside O.SB.3's 0.09 µm while still inside this rule's 0.1.
    // The block goes above the gate rather than beside it: beside it, the poly's own
    // overhang reaches the block and O.SB.9 - how far a block must cover the poly it
    // blocks - answers for the fixture instead.
    for (polarity, gap) in [("good", 0.4), ("bad", 0.095)] {
        write(
            "O.SB.5b_LV",
            polarity,
            vec![
                marker(),
                rect(comp, o, o, o + 3.0, o + 3.0),
                rect(poly, o - 0.6, o + 1.2, o + 3.6, o + 1.42),
                rect(sab, o + 0.2, o + 1.42 + gap, o + 1.8, o + 2.52 + gap),
            ],
        );
    }

    // O.SB.9: how far the block reaches past the poly it blocks.
    // The block crosses the gate stripe, so the two edges that cut the poly are
    // coincident and skipped, and only its reach past the stripe is measured.  It is
    // drawn wide because the area rule reads every block in the marker.
    for (polarity, over) in [("good", 0.3), ("bad", 0.095)] {
        write(
            "O.SB.9",
            polarity,
            vec![
                marker(),
                rect(comp, o, o, o + 3.0, o + 3.0),
                rect(poly, o - 0.6, o + 1.2, o + 3.6, o + 1.42),
                rect(sab, o - 0.4, o + 1.2 - over, o + 3.4, o + 1.42 + over),
            ],
        );
    }

    // O.SB.4: a contact beside the block, then under it.
    for (polarity, gap) in [("good", 0.3), ("bad", 0.025)] {
        let mut v = vec![marker()];
        v.extend(sab_on_comp(o, o, 1.5, 1.5));
        v.push(rect(comp, o + 1.5 + gap, o + 0.5, o + 2.5 + gap, o + 1.0));
        v.push(rect(
            contact,
            o + 1.5 + gap,
            o + 0.6,
            o + 1.72 + gap,
            o + 0.82,
        ));
        write("O.SB.4", polarity, v);
    }

    // O.CO.7: a contact on the active, closer than 0.13 µm to the gate.  The gate
    // crosses the whole active: an end stopping inside it leaves a channel edge that does
    // not run horizontally, and O.PL.ORT answers for the fixture instead.
    for (polarity, gap) in [("good", 0.4), ("bad", 0.125)] {
        write(
            "O.CO.7",
            polarity,
            vec![
                marker(),
                rect(comp, o, o + 0.8, o + 3.0, o + 2.0),
                rect(poly, o - 0.6, o + 1.2, o + 3.6, o + 1.42),
                rect(contact, o + 1.0, o + 1.42 + gap, o + 1.22, o + 1.64 + gap),
            ],
        );
    }

    // O.PL.ORT: a channel edge that does not run horizontally - the gate turned.
    for (polarity, turned) in [("good", false), ("bad", true)] {
        let mut v = vec![marker(), rect(comp, o, o, o + 1.6, o + 3.0)];
        v.push(if turned {
            rect(poly, o + 0.6, o - 0.6, o + 0.82, o + 3.6)
        } else {
            rect(poly, o - 0.6, o + 1.2, o + 2.2, o + 1.42)
        });
        write("O.PL.ORT", polarity, v);
    }
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from section 10.10 of the manual, at the bound each rule names and one
// 0.005 µm step past it, with a case in the `hardening_otp_mk` table of
// `tests/gf180mcuD.rs` and the findings in hardening/reports/gf180mcuD/otp_mk.md.
//
// The section is the ordinary 3.3 V rule set with tighter numbers inside a marker, so
// every fixture is one or more marked cells - a COMP with a horizontal poly line over it,
// and whatever the rule under test measures against.  The marker is drawn 1 µm past
// everything it holds, and the cells stand far enough apart that no marker of one reaches
// the shapes of another.
//
// The gate lines run horizontally throughout: O.PL.ORT forbids the other orientation, so
// a vertical gate anywhere would answer for the fixture instead of the rule it was drawn
// for.  Its own fixture is the only one that turns a gate.
//
// What is drawn here is what only this deck has - the marker, the salicide block, the
// two voltage classes and the orientation - plus the shared edge, the euclidian corner
// and the tile lines.  The generic classes are the engine family's.

/// The marker's reach past everything a cell holds.
const H_MK: f64 = 1.0;

fn hwrite(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// The context one hardening fixture draws in: the layers, and nothing else.
struct H {
    otp: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    sab: (i16, i16),
    contact: (i16, i16),
    v5: (i16, i16),
    dualgate: (i16, i16),
}

impl H {
    /// A marker `H_MK` past the box `(x0, y0)-(x1, y1)`.
    fn mk(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> gds21::GdsElement {
        rect(self.otp, x0 - H_MK, y0 - H_MK, x1 + H_MK, y1 + H_MK)
    }

    /// One marked transistor at `(x, y)`: a `w × h` COMP with a horizontal poly line of
    /// thickness `gate` across its middle, reaching `endcap` past it left and right.  The
    /// source/drain overhang O.DF.6 measures is `(h - gate) / 2`.
    fn dev(&self, x: f64, y: f64, w: f64, h: f64, gate: f64, endcap: f64) -> Vec<GdsElement> {
        let gy = y + (h - gate) * 0.5;
        vec![
            rect(self.comp, x, y, x + w, y + h),
            rect(self.poly, x - endcap, gy, x + w + endcap, gy + gate),
        ]
    }

    /// The same with its own marker round it.
    fn cell(&self, x: f64, y: f64, w: f64, h: f64, gate: f64, endcap: f64) -> Vec<GdsElement> {
        let mut v = vec![self.mk(x - endcap, y, x + w + endcap, y + h)];
        v.extend(self.dev(x, y, w, h, gate, endcap));
        v
    }

    /// A salicide block `(x0, y0)-(x1, y1)` with a COMP under its left edge, so that it
    /// clears O.SB.11's 0.04 µm overlap wherever the fixture puts it.
    fn block(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x0 - 0.5, y0 + 0.2, x0 + 0.3, y1 - 0.2),
            rect(self.sab, x0, y0, x1, y1),
        ]
    }
}

fn hardening(pdk: &PdkConfig) {
    let h = H {
        otp: layer(pdk, "otp_mk"),
        comp: layer(pdk, "comp"),
        poly: layer(pdk, "poly2_drawn"),
        sab: layer(pdk, "sab"),
        contact: layer(pdk, "contact"),
        v5: layer(pdk, "v5_xtor"),
        dualgate: layer(pdk, "dualgate"),
    };

    // --- O.DF.3a: 0.24 µm between actives in the marker.  Every gap is centred on a tile
    // line, and each pair sits under its own marker so no gap reaches the next pair.
    // The actives carry no poly, so nothing but the space is under test.
    hwrite("O.DF.3a.h1", {
        let pair = |xl: f64, xr: f64, y: f64| {
            vec![
                h.mk(xl - 2.0, y, xr + 2.0, y + 1.0),
                rect(h.comp, xl - 2.0, y, xl, y + 1.0),
                rect(h.comp, xr, y, xr + 2.0, y + 1.0),
            ]
        };
        let mut v = vec![];
        // x = 20: 0.24 exactly, clean.
        v.extend(pair(19.88, 20.12, 5.0));
        // x = 21: 0.235.
        v.extend(pair(20.885, 21.12, 10.0));
        // x = 40: 0.235.
        v.extend(pair(39.885, 40.12, 5.0));
        // y = 20: 0.235 across the horizontal line, drawn as a stacked pair.
        v.push(h.mk(58.0, 14.0, 62.0, 26.0));
        v.push(rect(h.comp, 58.0, 14.0, 62.0, 19.885));
        v.push(rect(h.comp, 58.0, 20.12, 62.0, 26.0));
        // A pair sharing an edge: a space of nothing.
        v.push(h.mk(70.0, 5.0, 74.0, 6.0));
        v.push(rect(h.comp, 70.0, 5.0, 72.0, 6.0));
        v.push(rect(h.comp, 72.0, 5.0, 74.0, 6.0));
        // Corner to corner: 0.18 in x and 0.15 in y, so 0.2343 euclidian with both axis
        // distances over the value.
        v.push(h.mk(80.0, 5.0, 84.18, 7.15));
        v.push(rect(h.comp, 80.0, 5.0, 82.0, 6.0));
        v.push(rect(h.comp, 82.18, 6.15, 84.18, 7.15));
        v
    });
    // One solid 3 × 3 COMP under a marker shaped like a U, its 0.2 µm slot running in
    // from the right.  The COMP has no notch; the marker's own outline cuts one into it.
    hwrite("O.DF.3a.h2", {
        vec![
            poly(
                h.otp,
                &[
                    (9.0, 9.0),
                    (14.0, 9.0),
                    (14.0, 10.9),
                    (11.0, 10.9),
                    (11.0, 11.1),
                    (14.0, 11.1),
                    (14.0, 14.0),
                    (9.0, 14.0),
                ],
            ),
            rect(h.comp, 10.0, 10.0, 13.0, 13.0),
        ]
    });

    // --- O.DF.9: 0.1444 µm² of active.  0.38 × 0.38 is the value exactly; 0.38 × 0.375
    // is one step under it.  The third active is 1 × 1 µm - seven times the minimum -
    // with the marker over a 0.3 µm corner of it only.
    hwrite("O.DF.9.h1", {
        vec![
            h.mk(10.0, 10.0, 10.38, 10.38),
            rect(h.comp, 10.0, 10.0, 10.38, 10.38),
            h.mk(20.0, 10.0, 20.38, 10.375),
            rect(h.comp, 20.0, 10.0, 20.38, 10.375),
            rect(h.otp, 29.0, 9.0, 30.3, 10.3),
            rect(h.comp, 30.0, 10.0, 31.0, 11.0),
        ]
    });

    // --- O.DF.6: the source/drain overhang, the active's reach past the gate line.  The
    // overhang is (h - gate)/2, so a 0.66 tall active with a 0.22 gate is exactly 0.22.
    hwrite("O.DF.6.h1", {
        let mut v = h.cell(10.0, 10.0, 1.0, 0.66, 0.22, 0.2);
        v.extend(h.cell(20.0, 10.0, 1.0, 0.65, 0.22, 0.2));
        v
    });
    // The same overhang over a 45° source edge.  Both devices are 1 × 1.4 µm with a 0.22
    // gate line across the middle (y = 10.59 to 10.81) reaching 0.2 past the active.
    hwrite("O.DF.6.h2", {
        let dev = |x: f64, chamfer_from: f64, chamfer_to: f64| {
            vec![
                h.mk(x - 0.2, 10.0, x + 1.2, 11.4),
                poly(
                    h.comp,
                    &[
                        (x, 10.0),
                        (x + 1.0, 10.0),
                        (x + 1.0, 11.4),
                        (x + chamfer_to, 11.4),
                        (x, chamfer_from),
                    ],
                ),
                rect(h.poly, x - 0.2, 10.59, x + 1.2, 10.81),
            ]
        };
        // The active's top-left corner cut from (x, 11.03) to (x + 0.37, 11.4).  Straight
        // up from every point of the gate's top wall there is 0.22 µm of active or more -
        // exactly the value at the wall's left end - but the gate's own top-left corner
        // stands 0.1556 µm from the 45° wall.  Euclidian fires, projection does not.
        let mut v = dev(10.0, 11.03, 0.37);
        // The same cut moved down to (x, 11.0): now the overhang straight up above the
        // wall's left end is 0.19, which is short in either metric.
        v.extend(dev(20.0, 11.0, 0.4));
        // And moved down to (x, 10.83), where the gate's top wall has 0.02 µm of active
        // over its left end - a tenth of what the rule asks, in either metric.
        v.extend(dev(30.0, 10.83, 0.57));
        v
    });

    // --- O.PL.2: the channel length, the gate line's own thickness over the active.
    hwrite("O.PL.2.h1", {
        let mut v = h.cell(10.0, 10.0, 1.0, 1.0, 0.22, 0.2);
        v.extend(h.cell(20.0, 10.0, 1.0, 1.005, 0.215, 0.2));
        v
    });

    // --- O.PL.3a: 0.18 µm of poly space in the marker.  Two gate lines over one active,
    // then a slot cut into a field poly plate, then a shared edge - each gap on a tile
    // line.
    hwrite("O.PL.3a.h1", {
        let two_gates = |x: f64, gap: f64| {
            let (y, w) = (5.0, 1.6);
            let mut v = vec![h.mk(x - 0.2, y, x + w + 0.2, y + 1.6)];
            v.push(rect(h.comp, x, y, x + w, y + 1.6));
            v.push(rect(h.poly, x - 0.2, y + 0.6, x + w + 0.2, y + 0.82));
            v.push(rect(
                h.poly,
                x - 0.2,
                y + 0.82 + gap,
                x + w + 0.2,
                y + 1.04 + gap,
            ));
            v
        };
        // x = 20: 0.18 exactly, clean.
        let mut v = two_gates(19.2, 0.18);
        // x = 40: 0.175.
        v.extend(two_gates(39.2, 0.175));
        // A field poly plate with a 0.175 slot cut in from the right, crossing x = 21.
        v.push(h.mk(20.0, 10.0, 24.0, 12.0));
        v.push(poly(
            h.poly,
            &[
                (20.0, 10.0),
                (24.0, 10.0),
                (24.0, 10.9),
                (20.6, 10.9),
                (20.6, 11.075),
                (24.0, 11.075),
                (24.0, 12.0),
                (20.0, 12.0),
            ],
        ));
        // Two field poly plates sharing an edge.
        v.push(h.mk(30.0, 10.0, 34.0, 11.0));
        v.push(rect(h.poly, 30.0, 10.0, 32.0, 11.0));
        v.push(rect(h.poly, 32.0, 10.0, 34.0, 11.0));
        v
    });

    // --- O.PL.4: the poly end cap, the gate line's reach past the active.
    hwrite("O.PL.4.h1", {
        let mut v = h.cell(10.0, 10.0, 1.0, 1.0, 0.22, 0.14);
        v.extend(h.cell(20.0, 10.0, 1.0, 1.0, 0.22, 0.135));
        // A gate line that stops 0.2 µm short of the active's right edge: the active runs
        // out through the end of the poly, which is no end cap at all.
        v.push(h.mk(29.8, 10.0, 31.0, 11.0));
        v.push(rect(h.comp, 30.0, 10.0, 31.0, 11.0));
        v.push(rect(h.poly, 29.86, 10.39, 30.8, 10.61));
        v
    });

    // --- O.SB.2: 0.28 µm between salicide blocks, on the tile lines, plus a slot and a
    // shared edge.  Each block covers an active, so O.SB.11 has nothing to say, and each
    // is over 1.488 µm², which is O.SB.13_LV's area.
    hwrite("O.SB.2.h1", {
        let pair = |xl: f64, xr: f64, y: f64| {
            let mut v = vec![h.mk(xl - 2.2, y - 0.2, xr + 2.0, y + 1.4)];
            v.extend(h.block(xl - 2.0, y, xl, y + 1.2));
            v.extend(h.block(xr, y, xr + 2.0, y + 1.2));
            v
        };
        // x = 20: 0.28 exactly, clean.
        let mut v = pair(19.86, 20.14, 5.0);
        // x = 42: 0.275.
        v.extend(pair(41.865, 42.14, 5.0));
        // A block with a 0.275 slot cut in from the right, crossing x = 21.
        v.push(h.mk(19.3, 9.8, 24.2, 13.2));
        v.push(rect(h.comp, 19.5, 10.2, 20.3, 12.8));
        v.push(poly(
            h.sab,
            &[
                (20.0, 10.0),
                (24.0, 10.0),
                (24.0, 11.2),
                (20.6, 11.2),
                (20.6, 11.475),
                (24.0, 11.475),
                (24.0, 13.0),
                (20.0, 13.0),
            ],
        ));
        // Two blocks sharing an edge.
        v.push(h.mk(29.3, 9.8, 34.2, 11.4));
        v.extend(h.block(30.0, 10.0, 32.0, 11.2));
        v.extend(h.block(32.0, 10.0, 34.0, 11.2));
        v
    });

    // --- O.SB.3: 0.09 µm from a block that covers no active to an active no block
    // covers.  Both bounds, then a shared edge.
    hwrite("O.SB.3.h1", {
        let probe = |x: f64, gap: f64| {
            vec![
                h.mk(x, 10.0, x + 3.1 + gap, 11.6),
                rect(h.sab, x, 10.0, x + 1.6, 11.6),
                rect(h.comp, x + 1.6 + gap, 10.0, x + 3.1 + gap, 11.6),
            ]
        };
        let mut v = probe(10.0, 0.09);
        v.extend(probe(20.0, 0.085));
        v.extend(probe(30.0, 0.0));
        v
    });

    // --- O.SB.4: 0.03 µm from a block to a contact, then a contact under the block.
    hwrite("O.SB.4.h1", {
        let probe = |x: f64, gap: Option<f64>| {
            let mut v = vec![h.mk(x - 0.6, 9.8, x + 2.4, 11.4)];
            v.extend(h.block(x, 10.0, x + 1.6, 11.2));
            match gap {
                Some(g) => {
                    v.push(rect(h.comp, x + 1.6 + g, 10.2, x + 2.4, 11.0));
                    v.push(rect(h.contact, x + 1.6 + g, 10.4, x + 1.82 + g, 10.62));
                }
                None => {
                    // Under the block: no space at all, which the rule forbids outright.
                    v.push(rect(h.contact, x + 0.6, 10.4, x + 0.82, 10.62));
                }
            }
            v
        };
        let mut v = probe(10.0, Some(0.03));
        v.extend(probe(20.0, Some(0.025)));
        v.extend(probe(30.0, Some(0.0)));
        v.extend(probe(40.0, None));
        v
    });

    // --- O.SB.5b: 0.1 µm from the block to a gate it does not cover, at 3.3 V.  The 5 V
    // rule is 0 and Appendix B lists O.SB.5b_MV as not coded, so the same 0.095 gap under
    // Dualgate, and again under V5_XTOR, must stay quiet.
    hwrite("O.SB.5b.h1", {
        let probe = |x: f64, gap: f64, mv: Option<bool>| {
            let mut v = vec![h.mk(x - 0.4, 9.8, x + 3.4, 12.2)];
            v.push(rect(h.comp, x, 10.0, x + 3.0, 12.0));
            v.push(rect(h.poly, x - 0.2, 10.4, x + 3.2, 10.62));
            v.extend(h.block(x + 0.4, 10.62 + gap, x + 2.4, 11.9));
            match mv {
                Some(true) => v.push(rect(h.dualgate, x - 0.8, 9.4, x + 3.8, 12.6)),
                Some(false) => v.push(rect(h.v5, x - 0.8, 9.4, x + 3.8, 12.6)),
                None => {}
            }
            v
        };
        let mut v = probe(10.0, 0.1, None);
        v.extend(probe(20.0, 0.095, None));
        v.extend(probe(30.0, 0.095, Some(true)));
        v.extend(probe(40.0, 0.095, Some(false)));
        v
    });

    // --- O.SB.9: the block's reach past the poly it blocks.  The block crosses the gate
    // line, so its two edges over the poly are coincident and only its reach past the
    // line's own walls is measured.  The last probe lets the poly run out through the
    // block's right edge, which is no extension at all.
    hwrite("O.SB.9.h1", {
        // The block is drawn 6 µm long so that its area clears O.SB.13's 1.488 µm² and
        // that rule has nothing to say about any of the three probes.
        let probe = |x: f64, over: Option<f64>| {
            let mut v = vec![h.mk(x - 0.6, 9.6, x + 7.6, 12.4)];
            v.push(rect(h.comp, x, 10.0, x + 7.0, 12.0));
            v.push(rect(h.poly, x - 0.2, 10.4, x + 7.2, 10.62));
            match over {
                Some(d) => v.push(rect(h.sab, x + 0.4, 10.4 - d, x + 6.4, 10.62 + d)),
                // The block ends inside the poly line's run, so the poly crosses out.
                None => v.push(rect(h.sab, x + 0.4, 10.2, x + 6.4, 10.82)),
            }
            v
        };
        let mut v = probe(10.0, Some(0.1));
        v.extend(probe(30.0, Some(0.095)));
        v.extend(probe(50.0, None));
        v
    });

    // --- O.SB.11: how deeply the block covers the active it blocks.
    hwrite("O.SB.11.h1", {
        let probe = |x: f64, depth: f64| {
            vec![
                h.mk(x - 0.2, 9.8, x + 3.2, 11.4),
                rect(h.comp, x, 10.0, x + 1.0, 11.2),
                rect(h.sab, x + 1.0 - depth, 9.9, x + 3.0, 11.3),
            ]
        };
        let mut v = probe(10.0, 0.04);
        v.extend(probe(20.0, 0.035));
        v
    });

    // --- O.SB.13: the block's own area, 1.488 µm² at 3.3 V and 2 µm² at 5 V.  The last
    // probe is a 1 µm² block under Dualgate and no V5_XTOR - a thick-oxide OTP cell,
    // which is the 5 V class and well under its two microns.
    hwrite("O.SB.13.h1", {
        let probe = |x: f64, w: f64, hh: f64, mark: Option<bool>| {
            let mut v = vec![h.mk(x - 0.6, 9.8, x + w + 0.2, 10.0 + hh + 0.2)];
            v.extend(h.block(x, 10.0, x + w, 10.0 + hh));
            match mark {
                Some(true) => v.push(rect(h.v5, x - 1.0, 9.4, x + w + 0.6, 10.6 + hh)),
                Some(false) => v.push(rect(h.dualgate, x - 1.0, 9.4, x + w + 0.6, 10.6 + hh)),
                None => {}
            }
            v
        };
        // 1.2 × 1.24 = 1.488 exactly, clean at 3.3 V.
        let mut v = probe(10.0, 1.2, 1.24, None);
        // 1.2 × 1.235 = 1.482.
        v.extend(probe(20.0, 1.2, 1.235, None));
        // 1.25 × 1.6 = 2.0 exactly under V5_XTOR, clean at 5 V.
        v.extend(probe(30.0, 1.25, 1.6, Some(true)));
        // 1.25 × 1.595 = 1.99375 under V5_XTOR.
        v.extend(probe(40.0, 1.25, 1.595, Some(true)));
        // 1 × 1 under Dualgate alone.
        v.extend(probe(50.0, 1.0, 1.0, Some(false)));
        v
    });

    // --- O.CO.7: 0.13 µm from a contact in the active to the gate line over it.
    hwrite("O.CO.7.h1", {
        let probe = |x: f64, gap: f64| {
            vec![
                h.mk(x - 0.4, 9.8, x + 3.4, 12.2),
                rect(h.comp, x, 10.0, x + 3.0, 12.0),
                rect(h.poly, x - 0.2, 10.4, x + 3.2, 10.62),
                rect(h.contact, x + 1.0, 10.62 + gap, x + 1.22, 10.84 + gap),
            ]
        };
        let mut v = probe(10.0, 0.13);
        v.extend(probe(20.0, 0.125));
        v.extend(probe(30.0, 0.0));
        v
    });

    // --- O.PL.ORT: the gate width has to run along x, so the channel's own edges against
    // the source and drain have to be horizontal.  A turned gate breaks it; one under
    // V5_XTOR is a 5 V cell, which the rule's own column marks NA.
    hwrite("O.PL.ORT.h1", {
        // Horizontal, clean.
        let mut v = h.cell(10.0, 10.0, 1.0, 1.0, 0.22, 0.14);
        // Turned a quarter: the channel edges run along y.
        v.push(h.mk(19.86, 9.86, 21.14, 11.14));
        v.push(rect(h.comp, 20.0, 10.0, 21.0, 11.0));
        v.push(rect(h.poly, 20.39, 9.86, 20.61, 11.14));
        // Turned, under V5_XTOR.
        v.push(h.mk(29.86, 9.86, 31.14, 11.14));
        v.push(rect(h.comp, 30.0, 10.0, 31.0, 11.0));
        v.push(rect(h.poly, 30.39, 9.86, 30.61, 11.14));
        v.push(rect(h.v5, 28.6, 8.6, 32.4, 12.4));
        v
    });
}
