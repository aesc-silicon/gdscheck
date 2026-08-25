// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! YMTP marker: a good and a bad pattern for every rule in the `ymtp_mk` deck.
//!
//! YMTP is a marker layer that relaxes or replaces the ordinary rules inside it, so every
//! fixture starts from a marker and puts the geometry under test inside it. Which set of
//! rules applies depends on the voltage class, and that is geometry too: an LV marker is
//! one that touches no Dualgate, an MV marker is one that overlaps Dualgate. Each case
//! below is drawn for one of the two and says which in its name.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/ymtp_mk";

/// Marker side. Generous: every rule here measures something well under a micron, and a
/// shape that reached the marker's edge would start testing the marker instead.
const MARK: f64 = 20.0;

/// Voltage class of a case, which is drawn rather than declared.
#[derive(Clone, Copy, PartialEq)]
enum Class {
    /// No Dualgate anywhere near the marker.
    Lv,
    /// Dualgate over the marker, which is what makes it the 5 V flavour.
    Mv,
}

struct Ctx {
    ymtp_mk: (i16, i16),
    dualgate: (i16, i16),
    nwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    poly: (i16, i16),
    plfuse: (i16, i16),
}

impl Ctx {
    /// The marker, plus the Dualgate that makes it MV. Everything else sits inside.
    fn base(&self, class: Class) -> Vec<gds21::GdsElement> {
        let mut v = vec![rect(
            self.ymtp_mk,
            OFFSET,
            OFFSET,
            OFFSET + MARK,
            OFFSET + MARK,
        )];
        if class == Class::Mv {
            v.push(rect(
                self.dualgate,
                OFFSET - 1.0,
                OFFSET - 1.0,
                OFFSET + MARK + 1.0,
                OFFSET + MARK + 1.0,
            ));
        }
        v
    }
}

/// A transistor: a COMP island with a POLY stripe crossing it.
///
/// `gate` is the poly width, which is the channel length the gate-length rules measure;
/// `overhang` is how far the poly runs past COMP, which is what the end-cap rule measures;
/// `sd` is the source/drain COMP either side of the gate, which is what the COMP-extend
/// rule measures. Every rule in this deck that needs a device needs a different one of
/// those three to be wrong, so they are all parameters.
fn transistor(
    c: &Ctx,
    x: f64,
    y: f64,
    gate: f64,
    sd: f64,
    overhang: f64,
    height: f64,
) -> Vec<gds21::GdsElement> {
    let comp_w = gate + 2.0 * sd;
    vec![
        rect(c.comp, x, y, x + comp_w, y + height),
        rect(
            c.poly,
            x + sd,
            y - overhang,
            x + sd + gate,
            y + height + overhang,
        ),
    ]
}

/// One generated case: a name, the class it is drawn for, and its two halves.
struct Case {
    name: &'static str,
    class: Class,
    good: Vec<gds21::GdsElement>,
    bad: Vec<gds21::GdsElement>,
    /// Cover the working area with PLFUSE.
    ///
    /// `Y.PL.1_MV` has no legal width at 5 V - upstream outputs the layer, so *any* poly
    /// in an MV marker is a violation of it, and every other MV case here would trip it
    /// as well. PLFUSE is the one thing the rule excludes (`poly.outside(plfuse)`), so
    /// shielding with it leaves each MV case measuring only its own rule. The `Y.PL.1_MV`
    /// case itself is of course not shielded.
    shield_pl1: bool,
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let c = Ctx {
        ymtp_mk: layer(pdk, "ymtp_mk"),
        dualgate: layer(pdk, "dualgate"),
        nwell: layer(pdk, "nwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        poly: layer(pdk, "poly2_drawn"),
        plfuse: layer(pdk, "plfuse"),
    };
    let o = OFFSET + 2.0; // inset from the marker edge

    // Two N-wells inside the marker. Y.NW.2b wants 1 um between them.
    let nwell_pair = |gap: f64| {
        vec![
            rect(c.nwell, o, o, o + 3.0, o + 3.0),
            rect(c.nwell, o + 3.0 + gap, o, o + 6.0 + gap, o + 3.0),
        ]
    };
    // An N-COMP inside the marker and clear of any well, with an unrelated N-well beside
    // it. Y.DF.16 wants 0.27 um (LV) / 0.23 um (MV) between them.
    let ncomp_near_nwell = |gap: f64| {
        vec![
            rect(c.comp, o, o, o + 2.0, o + 2.0),
            rect(c.nplus, o - 0.1, o - 0.1, o + 2.1, o + 2.1),
            rect(c.nwell, o + 2.0 + gap, o, o + 5.0 + gap, o + 2.0),
        ]
    };
    // Field poly, with no COMP under it. Y.PL.1 wants 0.13 um of width at 3.3 V, and no
    // poly at all inside an MV marker.
    let field_poly = |w: f64| vec![rect(c.poly, o, o, o + w, o + 3.0)];
    // Field poly beside a COMP island. Y.PL.5a/b want 0.04 um (LV) / 0.2 um (MV).
    let poly_near_comp = |gap: f64| {
        vec![
            rect(c.comp, o, o, o + 2.0, o + 2.0),
            rect(c.poly, o + 2.0 + gap, o, o + 2.5 + gap, o + 2.0),
        ]
    };

    let cases = vec![
        Case {
            name: "Y.NW.2b_LV",
            class: Class::Lv,
            good: nwell_pair(1.2),
            bad: nwell_pair(0.8),
            shield_pl1: false,
        },
        Case {
            name: "Y.NW.2b_MV",
            class: Class::Mv,
            good: nwell_pair(1.2),
            bad: nwell_pair(0.8),
            shield_pl1: false,
        },
        // Source/drain overhang: COMP must extend 0.15 um past the gate.
        Case {
            name: "Y.DF.6_MV",
            class: Class::Mv,
            good: transistor(&c, o, o, 0.6, 0.5, 0.4, 2.0),
            bad: transistor(&c, o, o, 0.6, 0.08, 0.4, 2.0),
            shield_pl1: true,
        },
        Case {
            name: "Y.DF.16_LV",
            class: Class::Lv,
            good: ncomp_near_nwell(0.4),
            bad: ncomp_near_nwell(0.2),
            shield_pl1: false,
        },
        Case {
            name: "Y.DF.16_MV",
            class: Class::Mv,
            good: ncomp_near_nwell(0.4),
            bad: ncomp_near_nwell(0.15),
            shield_pl1: false,
        },
        Case {
            name: "Y.PL.1_LV",
            class: Class::Lv,
            good: field_poly(0.2),
            bad: field_poly(0.1),
            shield_pl1: false,
        },
        // At 5 V there is no legal width: any field poly in the marker is the violation.
        Case {
            name: "Y.PL.1_MV",
            class: Class::Mv,
            good: vec![],
            bad: field_poly(0.5),
            shield_pl1: false,
        },
        // Channel length.
        Case {
            name: "Y.PL.2_LV",
            class: Class::Lv,
            good: transistor(&c, o, o, 0.2, 0.5, 0.4, 2.0),
            bad: transistor(&c, o, o, 0.1, 0.5, 0.4, 2.0),
            shield_pl1: true,
        },
        Case {
            name: "Y.PL.2_MV",
            class: Class::Mv,
            good: transistor(&c, o, o, 0.6, 0.5, 0.4, 2.0),
            bad: transistor(&c, o, o, 0.3, 0.5, 0.4, 2.0),
            shield_pl1: true,
        },
        // Poly end cap: poly must run 0.16 um past COMP.
        Case {
            name: "Y.PL.4_MV",
            class: Class::Mv,
            good: transistor(&c, o, o, 0.6, 0.5, 0.4, 2.0),
            bad: transistor(&c, o, o, 0.6, 0.5, 0.08, 2.0),
            shield_pl1: true,
        },
        // 5a (unrelated COMP) and 5b (related COMP) are the same measurement upstream, so
        // one pattern exercises both and the test expects both to fire.
        Case {
            name: "Y.PL.5_LV",
            class: Class::Lv,
            good: poly_near_comp(0.1),
            bad: poly_near_comp(0.02),
            shield_pl1: false,
        },
        Case {
            name: "Y.PL.5_MV",
            class: Class::Mv,
            good: poly_near_comp(0.4),
            bad: poly_near_comp(0.1),
            shield_pl1: true,
        },
    ];

    for case in cases {
        for (polarity, extra) in [("good", case.good), ("bad", case.bad)] {
            let mut elems = c.base(case.class);
            if case.shield_pl1 {
                elems.push(rect(c.plfuse, o - 1.0, o - 1.0, o + 9.0, o + 5.0));
            }
            elems.extend(extra);
            write_gz(
                &format!("{DIR}/{}.{polarity}.gds.gz", case.name),
                library("TOP", elems),
            );
        }
    }
}
