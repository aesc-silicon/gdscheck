// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Top metal: a good and a bad pattern for every rule in the `metaltop` deck.
//!
//! Four rules over one layer, which makes this the deck to read first if you are adding
//! patterns for another.  Each bad half carries one drawn violation of its own rule and
//! nothing else, so the harness can say plainly which rule a pattern is for; the good
//! half is the same geometry with the offending dimension put back to the limit.

use crate::helpers::{chamfered_tr, layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/metaltop";
/// Off the origin, so a sign error reads as a wrong answer rather than a shape that
/// happens to straddle (0, 0).
const O: f64 = 10.0;

/// Half the manufacturing grid: enough to break a limit, too little to look like slop.
const D: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    let m = layer(pdk, "metal5_drawn");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // MT.1, min width 0.44.  Long enough that the bar clears MT.4's area on either half.
    for (name, w) in [("good", 0.44), ("bad", 0.44 - D)] {
        write(&format!("MT.1.{name}"), vec![rect(m, O, O, O + w, O + 3.0)]);
    }

    // MT.2a, min space 0.46 - and the notch, which the deck checks under the same id, so
    // both entries are exercised by the one pattern.
    for (name, g) in [("good", 0.46), ("bad", 0.46 - D)] {
        write(
            &format!("MT.2a.{name}"),
            vec![
                rect(m, O, O, O + 2.0, O + 2.0),
                rect(m, O + 2.0 + g, O, O + 4.0 + g, O + 2.0),
                // A C, its opening the same gap: the notch is a space within one shape.
                rect(m, O, O + 5.0, O + 2.0, O + 5.0 + g + 2.0),
                rect(m, O + 2.0, O + 5.0, O + 4.0, O + 6.0),
                rect(m, O + 2.0, O + 5.0 + g + 1.0, O + 4.0, O + 5.0 + g + 2.0),
            ],
        );
    }

    // MT.2b, space to *wide* top metal: 0.6 where the neighbour is over 10 µm across in
    // both directions.  The gap stays clear of MT.2a's 0.46 either way, so only the wide
    // rule can speak.
    for (name, g) in [("good", 0.6), ("bad", 0.6 - D)] {
        write(
            &format!("MT.2b.{name}"),
            vec![
                rect(m, O, O, O + 12.0, O + 12.0),
                rect(m, O + 12.0 + g, O, O + 13.0 + g, O + 1.0),
            ],
        );
    }

    // MT.4, min area 0.5625.  Both halves are square and well over the 0.44 width, so the
    // only dimension in play is the area: 0.75² clears it and 0.7² does not.
    for (name, s) in [("good", 0.75), ("bad", 0.70)] {
        write(&format!("MT.4.{name}"), vec![rect(m, O, O, O + s, O + s)]);
    }
    hardening(pdk);
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md; the report is hardening/reports/gf180mcuD/metaltop.md)
//
// Section 7.15 is four geometric rules on one layer, and variant D is the 11K stack, so
// the starred column applies: 0.44 width, 0.46 space, 0.6 to wide metal, 0.5625 µm² of
// area.  On a 5LM stack the top metal *is* Metal5, so these layouts also ask what that
// makes of Metal5's own datatypes and of section 7.13's Mn rules, which stop at Metal4.
// Section 7.16's 3 µm option (MT30.x) is a different top metal and not this variant's.

/// The gap the wide-metal layouts leave: over MT.2a's 0.46 and one grid step under
/// MT.2b's 0.6, so only the wide rule can speak.
const WIDE_GAP: f64 = 0.6 - D;

pub fn hardening(pdk: &PdkConfig) {
    let m = layer(pdk, "metal5_drawn");
    let dummy = layer(pdk, "metal5_dummy");

    // --- What the top metal is --------------------------------------------------
    // The top metal is the drawn layer and the dummy fill together - dummy metal is
    // metal, and a 0.435 dummy bar is as narrow as a drawn one.  The slot, blocked,
    // label and resistor datatypes are markers, not metal, and a 0.435 bar on any of
    // them is nothing.  Last, a 0.3 drawn bar abutting a 0.3 dummy bar is one 0.6 wide
    // conductor, not two narrow ones.
    let mut e = vec![];
    let bar = |l: (i16, i16), x: f64| rect(l, x, 1.0, x + 0.435, 4.0);
    e.push(bar(m, 1.0));
    e.push(bar(dummy, 3.0));
    for (i, suffix) in ["slot", "blk", "label", "res"].iter().enumerate() {
        e.push(bar(
            layer(pdk, &format!("metal5_{suffix}")),
            5.0 + i as f64 * 2.0,
        ));
    }
    e.push(rect(m, 13.0, 1.0, 13.3, 4.0));
    e.push(rect(dummy, 13.3, 1.0, 13.6, 4.0));
    write("MT.1.h1", e);

    // Metal5 on a 5LM stack is the top metal, so 7.13's Mn.1 (0.28 above Metal1) is not
    // its rule and MT.1's 0.44 is: a 0.3 bar is legal nowhere and a 0.45 bar everywhere.
    // The `metal` deck must say nothing about either.
    write(
        "MT.1.h2",
        vec![rect(m, 1.0, 1.0, 1.3, 4.0), rect(m, 3.0, 1.0, 3.45, 4.0)],
    );

    // --- MT.2a across the two datatypes -----------------------------------------
    // Drawn to dummy at 0.455 is a space between two pieces of top metal; drawn to a
    // blocked-fill marker at the same gap is not; drawn to dummy at 0.46 is the bound.
    write(
        "MT.2a.h1",
        vec![
            rect(m, 1.0, 1.0, 2.0, 2.0),
            rect(dummy, 2.455, 1.0, 3.455, 2.0),
            rect(m, 5.0, 1.0, 6.0, 2.0),
            rect(layer(pdk, "metal5_blk"), 6.455, 1.0, 7.455, 2.0),
            rect(m, 9.0, 1.0, 10.0, 2.0),
            rect(dummy, 10.46, 1.0, 11.46, 2.0),
        ],
    );

    // --- MT.2b: what "wide" is ---------------------------------------------------
    // "Space between wide (length & width > 10um) MetalTop".  10.0 is not over 10 and
    // 10.005 is; a 30 x 9.995 bar is long but not wide.  Each plate has a neighbour
    // 0.595 away, over MT.2a and one step under MT.2b.
    write(
        "MT.2b.h1",
        vec![
            rect(m, 1.0, 1.0, 11.0, 11.0),
            rect(m, 11.0 + WIDE_GAP, 1.0, 11.8 + WIDE_GAP, 1.8),
            rect(m, 15.0, 1.0, 25.005, 11.005),
            rect(m, 25.005 + WIDE_GAP, 1.0, 25.805 + WIDE_GAP, 1.8),
            rect(m, 1.0, 15.0, 31.0, 24.995),
            rect(m, 31.0 + WIDE_GAP, 15.0, 31.8 + WIDE_GAP, 15.8),
            rect(m, 1.0, 28.0, 31.0, 38.005),
            rect(m, 31.0 + WIDE_GAP, 28.0, 31.8 + WIDE_GAP, 28.8),
        ],
    );

    // Two plates that are plainly over 10 µm across both ways but carry a short edge:
    // a 12 x 12 plate with a 1 µm chamfer on one corner, and a 12 x 12 plate with a
    // 3 x 0.6 stub.  Both are wide metal by the manual's dimensions.  The chamfered
    // plate's neighbour is 0.595 from its straight right wall; the stubbed plate's is
    // 0.595 below the plate itself, and a third neighbour faces only the stub, which is
    // not wide and owes 0.46, not 0.6.
    write(
        "MT.2b.h2",
        vec![
            chamfered_tr(m, 1.0, 1.0, 13.0, 13.0, 25.0),
            rect(m, 13.0 + WIDE_GAP, 1.0, 13.8 + WIDE_GAP, 1.8),
            rect(m, 17.0, 1.0, 29.0, 13.0),
            rect(m, 29.0, 6.0, 32.0, 6.6),
            rect(m, 32.0 + WIDE_GAP, 6.0, 32.8 + WIDE_GAP, 6.8),
            rect(m, 1.0, 17.0, 13.0, 29.0),
            rect(m, 13.0, 22.0, 16.0, 22.6),
            rect(m, 5.0, 17.0 - WIDE_GAP - 0.8, 5.8, 17.0 - WIDE_GAP),
        ],
    );

    // A 0.595 slot cut into a 30 x 20 plate, 14.5 µm of metal one side and 14.9 the
    // other: a space between two walls of wide metal.  Below it the same slot in a 9 µm
    // plate, which is wide nowhere.
    write(
        "MT.2b.h3",
        vec![
            poly(
                m,
                &[
                    (1.0, 1.0),
                    (31.0, 1.0),
                    (31.0, 21.0),
                    (15.5 + WIDE_GAP, 21.0),
                    (15.5 + WIDE_GAP, 9.0),
                    (15.5, 9.0),
                    (15.5, 21.0),
                    (1.0, 21.0),
                ],
            ),
            poly(
                m,
                &[
                    (1.0, 25.0),
                    (10.0, 25.0),
                    (10.0, 34.0),
                    (5.3 + WIDE_GAP, 34.0),
                    (5.3 + WIDE_GAP, 29.0),
                    (5.3, 29.0),
                    (5.3, 34.0),
                    (1.0, 34.0),
                ],
            ),
        ],
    );

    // The same 12 x 12 plate and its 0.595 neighbour four times over, put where the tile
    // lines fall: the gap opening on x = 20, x = 21, x = 40 and x = 42, with the plate
    // crossing a line in two of them.
    let mut e = vec![];
    for (x, y) in [(8.0, 1.0), (9.0, 15.0), (28.0, 1.0), (30.0, 15.0)] {
        e.push(rect(m, x, y, x + 12.0, y + 12.0));
        e.push(rect(
            m,
            x + 12.0 + WIDE_GAP,
            y,
            x + 12.8 + WIDE_GAP,
            y + 0.8,
        ));
    }
    write("MT.2b.h4", e);

    // --- MT.4 --------------------------------------------------------------------
    // 0.5625 µm²: 0.75 x 0.75 is exactly it, 0.75 x 0.745 one grid step under, and both
    // are well over MT.1's 0.44 so area is the only dimension in play.
    write(
        "MT.4.h1",
        vec![
            rect(m, 1.0, 1.0, 1.75, 1.75),
            rect(m, 3.0, 1.0, 3.75, 1.745),
        ],
    );

    // A ring drawn as four bars: 0.8 square outside, a 0.4 hole inside, so the metal is
    // 0.48 µm² and the outline 0.64.  Beside it a solid 0.8 square, which is 0.64 and
    // clean.  The 0.2 arms are MT.1's business and are ignored.
    write(
        "MT.4.h2",
        vec![
            rect(m, 1.0, 1.0, 1.8, 1.2),
            rect(m, 1.0, 1.6, 1.8, 1.8),
            rect(m, 1.0, 1.2, 1.2, 1.6),
            rect(m, 1.6, 1.2, 1.8, 1.6),
            rect(m, 3.0, 1.0, 3.8, 1.8),
        ],
    );
}
