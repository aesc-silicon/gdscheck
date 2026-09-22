// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Metal 1-4: a good and a bad pattern for every rule the `metal` deck declares.
//!
//! The same five rules at each of four levels, differing only in the width and space they
//! ask for, so the fixtures are generated from a table rather than drawn one at a time.
//! Every rule reads the drawn metal directly - `metal1_no_sram` is the drawn layer minus
//! the SRAM core, and no pattern here draws one - which is what makes this deck the
//! easiest of the family to cover.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/metal";
const O: f64 = 10.0;
/// Half the manufacturing grid: enough to break a limit, too little to look like slop.
const D: f64 = 0.005;
/// Space to metal over 10 µm across in both directions, the same at every level.
const WIDE_SPACE: f64 = 0.3;

/// Level, its drawn layer, and the width and space it asks for.
const LEVELS: &[(usize, &str, f64, f64)] = &[
    (1, "metal1_drawn", 0.23, 0.23),
    (2, "metal2_drawn", 0.28, 0.28),
    (3, "metal3_drawn", 0.28, 0.28),
    (4, "metal4_drawn", 0.28, 0.28),
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    for &(n, lname, width, space) in LEVELS {
        let m = layer(pdk, lname);

        // M#.1, min width.  Long enough that the bar clears the area rule either way.
        for (name, w) in [("good", width), ("bad", width - D)] {
            write(
                &format!("M{n}.1.{name}"),
                vec![rect(m, O, O, O + w, O + 2.0)],
            );
        }

        // M#.2a, min space, and the notch the deck checks under the same id.  Both shapes
        // stay well under 10 µm, so the wide-metal rule has nothing to look at.
        for (name, g) in [("good", space), ("bad", space - D)] {
            write(
                &format!("M{n}.2a.{name}"),
                vec![
                    rect(m, O, O, O + 1.0, O + 1.0),
                    rect(m, O + 1.0 + g, O, O + 2.0 + g, O + 1.0),
                    rect(m, O, O + 3.0, O + 1.0, O + 5.0 + g),
                    rect(m, O + 1.0, O + 3.0, O + 2.0, O + 4.0),
                    rect(m, O + 1.0, O + 4.0 + g, O + 2.0, O + 5.0 + g),
                ],
            );
        }

        // M#.2b, space to *wide* metal - over 10 µm across both ways.  The gap stays over
        // the ordinary space at every level, so only the wide rule can speak.
        for (name, g) in [("good", WIDE_SPACE), ("bad", WIDE_SPACE - D)] {
            write(
                &format!("M{n}.2b.{name}"),
                vec![
                    rect(m, O, O, O + 12.0, O + 12.0),
                    rect(m, O + 12.0 + g, O, O + 13.0 + g, O + 1.0),
                ],
            );
        }

        // M#.3, min area.  Square and over the width at every level, so area is the only
        // dimension in play: 0.4² clears 0.1444 and 0.35² does not.
        for (name, s) in [("good", 0.40), ("bad", 0.35)] {
            // 0.1444 µm² is the limit at every level, and 0.35² = 0.1225 is under it
            // while staying over the widest width the deck asks for.
            debug_assert!(s > width, "the area fixture must not also be too narrow");
            write(&format!("M{n}.3.{name}"), vec![rect(m, O, O, O + s, O + s)]);
        }
    }
    hardening(pdk);
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md; the report is hardening/reports/gf180mcuD/metal.md) --
//
// Section 7.13 of the manual is five rules repeated at every level: width, space, space
// to wide metal, area and a density rule the `density` deck carries.  What is the metal
// deck's own, and what these layouts ask about, is therefore not the checks - the engine
// family has their classes - but the deck's conditions: which drawn layer each level
// reads, the SRAM exemption on M1.1 alone, the two values the levels are split over
// (0.23 at Metal1, 0.28 above it), and above all what "wide" means, since the manual
// says only "length & width > 10µm".

/// The gap the wide-metal layouts leave: over the widest Mn.2a (0.28) and one grid step
/// under Mn.2b's 0.3, so only the wide rule can speak.
const WIDE_GAP: f64 = WIDE_SPACE - D;

pub fn hardening(pdk: &PdkConfig) {
    let m1 = layer(pdk, "metal1_drawn");
    let sram = layer(pdk, "sramcore");
    let dg = layer(pdk, "dualgate");
    let v5 = layer(pdk, "v5_xtor");

    // --- M1.1 and the SRAM core -------------------------------------------------
    // Metal1 alone exempts the 3.3 V SRAM core, which has S.M1.1_LV (0.22) instead.  Six
    // 0.225 bars - under the 0.23 M1.1 asks for, over the 0.22 the SRAM rule asks for -
    // each in a different relation to SRAMCORE and the markers that decide whether the
    // core is the 3.3 V kind.  By the round's settled reading a marker classifies what it
    // covers, so a SRAMCORE that Dualgate merely abuts is still a 3.3 V core, and V5_XTOR
    // without Dualgate marks no thick oxide at all.
    let mut e = vec![];
    let bar = |x: f64| rect(m1, x, 1.0, x + 0.225, 3.0);
    // 1: bare - no core, M1.1 in full.
    e.push(bar(1.0));
    // 2: wholly inside a bare SRAMCORE - the 3.3 V core, exempt.
    e.push(bar(6.0));
    e.push(rect(sram, 5.0, 0.5, 7.0, 3.5));
    // 3: inside a SRAMCORE that Dualgate covers - a 5 V core, not exempt.
    e.push(bar(11.0));
    e.push(rect(sram, 10.0, 0.5, 12.0, 3.5));
    e.push(rect(dg, 9.5, 0.0, 12.5, 4.0));
    // 4: inside a SRAMCORE whose right edge Dualgate abuts - covered by nothing, exempt.
    e.push(bar(16.0));
    e.push(rect(sram, 15.0, 0.5, 17.0, 3.5));
    e.push(rect(dg, 17.0, 0.5, 19.0, 3.5));
    // 5: inside a SRAMCORE under V5_XTOR alone - no thick oxide, a 3.3 V core, exempt.
    e.push(bar(21.0));
    e.push(rect(sram, 20.0, 0.5, 22.0, 3.5));
    e.push(rect(v5, 20.5, 1.0, 21.5, 3.0));
    // 6: inside a SRAMCORE that Dualgate covers half of - not a 3.3 V core, not exempt.
    e.push(bar(26.0));
    e.push(rect(sram, 25.0, 0.5, 27.0, 3.5));
    e.push(rect(dg, 26.0, 0.0, 28.0, 4.0));
    write("M1.1.h1", e);

    // A 0.225 bar half inside a bare SRAMCORE: the exemption takes the half it covers and
    // the half outside is still 0.225 wide.  And a bar the core abuts without covering.
    write(
        "M1.1.h2",
        vec![
            rect(m1, 1.0, 1.0, 1.225, 5.0),
            rect(sram, 0.0, 0.0, 6.0, 3.0),
            rect(m1, 8.0, 1.0, 8.225, 3.0),
            rect(sram, 8.225, 0.0, 10.0, 4.0),
        ],
    );

    // --- The datatypes each level does *not* read -------------------------------
    // Every level is drawn on one GDS layer with the slot, dummy, blocked, label and
    // resistor variants beside it on other datatypes.  Section 7.13 is about the drawn
    // metal: a 0.1 square on any of the others is neither too narrow nor too small, and a
    // dummy shape 0.2 from a drawn one is not a space this deck measures (the dummy fill
    // rules of section 13 have their own).
    let mut e = vec![];
    for (n, base) in [(1usize, 0.0), (2, 3.0), (3, 6.0), (4, 9.0)] {
        let drawn = layer(pdk, &format!("metal{n}_drawn"));
        for (i, suffix) in ["slot", "dummy", "blk", "label", "res"].iter().enumerate() {
            let l = layer(pdk, &format!("metal{n}_{suffix}"));
            let x = 1.0 + i as f64 * 2.0;
            e.push(rect(l, x, base + 1.0, x + 0.1, base + 1.1));
        }
        let dummy = layer(pdk, &format!("metal{n}_dummy"));
        e.push(rect(drawn, 12.0, base + 1.0, 12.5, base + 1.5));
        e.push(rect(dummy, 12.7, base + 1.0, 13.2, base + 1.5));
    }
    write("M1.1.h3", e);

    // --- The two columns the levels are split over ------------------------------
    // Metal1 asks 0.23 and the levels above it 0.28: a 0.25 bar is legal on Metal1 and
    // too narrow on every other level.  All four drawn at the same place, since the decks
    // read one layer each.
    let mut e = vec![];
    for n in 1..=4 {
        let l = layer(pdk, &format!("metal{n}_drawn"));
        e.push(rect(l, 1.0, 1.0, 1.25, 3.0));
    }
    write("M2.1.h1", e);

    // The same split in space: a 0.25 gap and a 0.25 notch, legal on Metal1 only.
    let mut e = vec![];
    for n in 1..=4 {
        let l = layer(pdk, &format!("metal{n}_drawn"));
        e.push(rect(l, 1.0, 1.0, 2.0, 2.0));
        e.push(rect(l, 2.25, 1.0, 3.25, 2.0));
        // A U whose opening is the same 0.25, with 0.875 arms.
        e.push(rect(l, 1.0, 5.0, 3.0, 5.5));
        e.push(rect(l, 1.0, 5.5, 1.875, 7.0));
        e.push(rect(l, 2.125, 5.5, 3.0, 7.0));
    }
    write("M2.2a.h1", e);

    // --- Mn.2b: what "wide" is --------------------------------------------------
    // "Space to wide Metaln (length & width > 10µm)".  Read strictly, 10.0 is not over
    // 10 and 10.005 is: four plates at the bound, each with a neighbour 0.295 away.  A
    // 30 x 9.995 bar is long but not wide, so "length & width" is both dimensions and
    // not either.
    let mut e = vec![];
    for n in 1..=4 {
        let l = layer(pdk, &format!("metal{n}_drawn"));
        // 10.0 square: not over 10 either way.
        e.push(rect(l, 1.0, 1.0, 11.0, 11.0));
        e.push(rect(l, 11.0 + WIDE_GAP, 1.0, 11.6 + WIDE_GAP, 1.6));
        // 10.005 square: over 10 both ways.
        e.push(rect(l, 14.0, 1.0, 24.005, 11.005));
        e.push(rect(l, 24.005 + WIDE_GAP, 1.0, 24.605 + WIDE_GAP, 1.6));
        // 30 x 9.995: long, not wide.
        e.push(rect(l, 1.0, 15.0, 31.0, 24.995));
        e.push(rect(l, 31.0 + WIDE_GAP, 15.0, 31.6 + WIDE_GAP, 15.6));
        // 30 x 10.005: long and wide.
        e.push(rect(l, 1.0, 28.0, 31.0, 38.005));
        e.push(rect(l, 31.0 + WIDE_GAP, 28.0, 31.6 + WIDE_GAP, 28.6));
    }
    write("M1.2b.h1", e);

    // A wide plate with a narrow stub on it.  The stub is metal and it is 0.295 from its
    // neighbour, but it is not wide, and it shares its whole boundary with the wide part
    // of the same shape - a space of nothing that is not a space at all.  Four scenes: a
    // plate alone, a T alone, a T whose neighbour faces the stub, and a plate whose
    // neighbour faces the plate.  Only the last is a violation.
    write(
        "M1.2b.h2",
        vec![
            rect(m1, 1.0, 1.0, 13.0, 13.0),
            rect(m1, 16.0, 1.0, 28.0, 13.0),
            rect(m1, 28.0, 6.0, 31.0, 6.5),
            rect(m1, 1.0, 16.0, 13.0, 28.0),
            rect(m1, 13.0, 21.0, 16.0, 21.5),
            rect(m1, 16.0 + WIDE_GAP, 21.0, 16.6 + WIDE_GAP, 21.6),
            rect(m1, 20.0, 16.0, 32.0, 28.0),
            rect(m1, 32.0 + WIDE_GAP, 16.0, 32.6 + WIDE_GAP, 16.6),
        ],
    );

    // A slot cut into a wide plate: 0.295 across, with 14.5 µm of plate one side and 15.2
    // the other, both over 10 µm every way.  The manual's rule is a space, and a slot in
    // a plate is a space between two walls of wide metal.  Below it the same slot in a
    // 9 µm plate, which is wide nowhere.
    write(
        "M1.2b.h3",
        vec![
            poly(
                m1,
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
                m1,
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

    // The same 12 x 12 plate and its 0.295 neighbour four times over, put where the tile
    // lines fall: the gap opening on x = 20, on x = 21, on x = 40 and on x = 42, with the
    // plate crossing a line in two of them.  A plate is wider than a tile is, which is
    // what makes the wide-metal derivation the deck's most tile-exposed reading.
    let mut e = vec![];
    for (x, y) in [(8.0, 1.0), (9.0, 15.0), (28.0, 1.0), (30.0, 15.0)] {
        e.push(rect(m1, x, y, x + 12.0, y + 12.0));
        e.push(rect(
            m1,
            x + 12.0 + WIDE_GAP,
            y,
            x + 12.6 + WIDE_GAP,
            y + 0.6,
        ));
    }
    write("M1.2b.h4", e);

    // Two wide plates 0.295 apart.  One gap, one violation - the rule is a space, and a
    // space read from either plate is the same space.
    write(
        "M1.2b.h5",
        vec![
            rect(m1, 1.0, 1.0, 13.0, 13.0),
            rect(m1, 13.0 + WIDE_GAP, 1.0, 25.0 + WIDE_GAP, 13.0),
        ],
    );

    // --- Mn.3 -------------------------------------------------------------------
    // 0.1444 µm² at every level: 0.38 x 0.38 is exactly it and 0.38 x 0.375 is one grid
    // step under, both wider than the widest Mn.1 so area is the only dimension in play.
    let mut e = vec![];
    for n in 1..=4 {
        let l = layer(pdk, &format!("metal{n}_drawn"));
        e.push(rect(l, 1.0, 1.0, 1.38, 1.38));
        e.push(rect(l, 3.0, 1.0, 3.38, 1.375));
    }
    write("M1.3.h1", e);

    // A ring drawn as four bars: 0.4 square outside, a 0.2 hole inside, so the metal is
    // 0.12 µm² and the outline 0.16.  The area a shape has is the metal it holds, not the
    // ground its outline covers.  Beside it a solid 0.4 square, which is 0.16 and clean.
    // The 0.1 arms are Mn.1's business and are ignored.
    write(
        "M1.3.h2",
        vec![
            rect(m1, 1.0, 1.0, 1.4, 1.1),
            rect(m1, 1.0, 1.3, 1.4, 1.4),
            rect(m1, 1.0, 1.1, 1.1, 1.3),
            rect(m1, 1.3, 1.1, 1.4, 1.3),
            rect(m1, 3.0, 1.0, 3.4, 1.4),
        ],
    );
}
