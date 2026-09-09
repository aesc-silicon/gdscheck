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
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/otp_mk";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
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
    // The channel edges are `poly.edges and tgate.edges` — the poly's sidewalls where it
    // crosses the active — and the rule is the distance *through the poly* between them.
    // No region carries it: the poly region here is 3 um long and the gate is 0.22 wide.
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
