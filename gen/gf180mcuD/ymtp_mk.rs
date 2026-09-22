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
use crate::helpers::{layer, library, poly, rect, write_gz};
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
    hardening(pdk);
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
        // one drawing exercises both.  It is written under each id rather than under a
        // shared name, so that every rule in the deck has a pattern the harness can find
        // by its own id.
        Case {
            name: "Y.PL.5a_LV",
            class: Class::Lv,
            good: poly_near_comp(0.1),
            bad: poly_near_comp(0.02),
            shield_pl1: false,
        },
        Case {
            name: "Y.PL.5b_LV",
            class: Class::Lv,
            good: poly_near_comp(0.1),
            bad: poly_near_comp(0.02),
            shield_pl1: false,
        },
        Case {
            name: "Y.PL.5a_MV",
            class: Class::Mv,
            good: poly_near_comp(0.4),
            bad: poly_near_comp(0.1),
            shield_pl1: true,
        },
        Case {
            name: "Y.PL.5b_MV",
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

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from section 10.13 of the manual, at the bound each rule names and one
// 0.005 µm step past it, with a case in the `hardening_ymtp_mk` table of
// `tests/gf180mcuD.rs` and the findings in hardening/reports/gf180mcuD/ymtp_mk.md.
//
// The section is a handful of the ordinary rules restated with their own numbers inside
// one marker, and half of them exist at one voltage only - Y.DF.6 and Y.PL.4 at 5 V,
// Y.PL.1's width at 3.3 V - so most fixtures carry the same geometry twice, once under a
// marker Dualgate covers and once under one it does not, and the pair says which rule is
// the one that should speak.
//
// Every MV probe that draws poly is shielded with PLFUSE: Y.PL.1_MV forbids poly in a 5 V
// marker outright and excludes only what lies over PLFUSE, so without the shield it would
// answer for every other MV fixture.
//
// What is drawn here is what only this deck has - the marker, the two classes, the OTP
// exemption and the PLFUSE one - plus the shared edge, the euclidian corner and the tile
// lines.  The generic classes are the engine family's.

/// The marker's reach past everything a probe holds.
const H_MK: f64 = 1.0;

fn hwrite(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// The layers one hardening fixture draws on.
struct H {
    ymtp: (i16, i16),
    dualgate: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    poly: (i16, i16),
    plfuse: (i16, i16),
    otp: (i16, i16),
}

/// How a probe's marker is classed.
#[derive(Clone, Copy, PartialEq)]
enum K {
    /// No Dualgate anywhere.
    Lv,
    /// Dualgate over the marker.
    Mv,
    /// Dualgate flush against the marker's left edge and over none of it.
    Abut,
}

impl H {
    /// A marker `H_MK` past the box, classed by `k`.
    fn mk(&self, k: K, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<gds21::GdsElement> {
        let (a, b, c, d) = (x0 - H_MK, y0 - H_MK, x1 + H_MK, y1 + H_MK);
        let mut v = vec![rect(self.ymtp, a, b, c, d)];
        match k {
            K::Lv => {}
            K::Mv => v.push(rect(self.dualgate, a - 0.5, b - 0.5, c + 0.5, d + 0.5)),
            K::Abut => v.push(rect(self.dualgate, a - 3.0, b, a, d)),
        }
        v
    }

    /// One transistor at `(x, y)`: a `w × 2` COMP with a vertical poly line of width
    /// `gate` across it, reaching `endcap` past it top and bottom.  `sd` is how much
    /// active stands either side of the gate, which is what Y.DF.6 measures.
    fn dev(&self, x: f64, y: f64, gate: f64, sd: f64, endcap: f64) -> Vec<gds21::GdsElement> {
        let w = gate + 2.0 * sd;
        vec![
            rect(self.comp, x, y, x + w, y + 2.0),
            rect(self.nplus, x - 0.2, y - 0.2, x + w + 0.2, y + 2.2),
            rect(
                self.poly,
                x + sd,
                y - endcap,
                x + sd + gate,
                y + 2.0 + endcap,
            ),
        ]
    }
}

fn hardening(pdk: &PdkConfig) {
    let h = H {
        ymtp: layer(pdk, "ymtp_mk"),
        dualgate: layer(pdk, "dualgate"),
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        poly: layer(pdk, "poly2_drawn"),
        plfuse: layer(pdk, "plfuse"),
        otp: layer(pdk, "otp_mk"),
    };

    // --- Y.NW.2b: 1 µm between wells outside DNWELL inside the marker, at both voltage
    // classes.  Every gap is centred on a tile line.
    hwrite("Y.NW.2b.h1", {
        let pair = |k: K, xl: f64, xr: f64, y: f64| {
            let mut v = h.mk(k, xl - 3.0, y, xr + 3.0, y + 3.0);
            v.push(rect(h.nwell, xl - 3.0, y, xl, y + 3.0));
            v.push(rect(h.nwell, xr, y, xr + 3.0, y + 3.0));
            v
        };
        // x = 20: 1.0 exactly, clean.
        let mut v = pair(K::Lv, 19.5, 20.5, 5.0);
        // x = 21: 0.995.
        v.extend(pair(K::Lv, 20.505, 21.5, 12.0));
        // x = 40: 0.995 under a marker Dualgate covers - the 5 V id.
        v.extend(pair(K::Mv, 39.505, 40.5, 5.0));
        // A 0.995 notch across y = 20, in a plate that has no other gap.
        v.extend(h.mk(K::Lv, 60.0, 14.0, 66.0, 26.0));
        v.push(poly(
            h.nwell,
            &[
                (60.0, 14.0),
                (66.0, 14.0),
                (66.0, 19.505),
                (62.0, 19.505),
                (62.0, 20.5),
                (66.0, 20.5),
                (66.0, 26.0),
                (60.0, 26.0),
            ],
        ));
        v
    });
    // Three wells the rule has nothing to say about, and one it should.
    hwrite("Y.NW.2b.h2", {
        // A pair 0.995 apart inside DNWELL: the rule is outside DNWELL only.
        let mut v = h.mk(K::Lv, 7.0, 5.0, 17.0, 8.0);
        v.push(rect(h.dnwell, 6.0, 4.0, 18.0, 9.0));
        v.push(rect(h.nwell, 7.0, 5.0, 11.0, 8.0));
        v.push(rect(h.nwell, 11.995, 5.0, 17.0, 8.0));
        // A pair 0.995 apart under a marker Dualgate only abuts: no thick oxide over it,
        // so it is the 3.3 V marker.
        v.extend(h.mk(K::Abut, 27.0, 5.0, 37.0, 8.0));
        v.push(rect(h.nwell, 27.0, 5.0, 31.0, 8.0));
        v.push(rect(h.nwell, 31.995, 5.0, 37.0, 8.0));
        // One solid 4 × 4 well with no notch in it, under a marker shaped like a U whose
        // slot is 0.5 µm.
        v.push(poly(
            h.ymtp,
            &[
                (49.0, 4.0),
                (56.0, 4.0),
                (56.0, 5.75),
                (52.0, 5.75),
                (52.0, 6.25),
                (56.0, 6.25),
                (56.0, 9.0),
                (49.0, 9.0),
            ],
        ));
        v.push(rect(h.nwell, 50.0, 5.0, 54.0, 8.0));
        v
    });

    // --- Y.DF.16: from an N+ active in the marker, clear of every well, to the edge of a
    // well outside DNWELL.  0.27 at 3.3 V and 0.23 at 5 V.
    hwrite("Y.DF.16.h1", {
        let probe = |k: K, x: f64, y: f64, gap: f64| {
            let mut v = h.mk(k, x, y, x + 2.0 + gap + 3.0, y + 2.0);
            v.push(rect(h.comp, x, y, x + 2.0, y + 2.0));
            v.push(rect(h.nplus, x - 0.2, y - 0.2, x + 2.2, y + 2.2));
            v.push(rect(h.nwell, x + 2.0 + gap, y, x + 5.0 + gap, y + 2.0));
            v
        };
        // 3.3 V: 0.27 (clean), 0.265, and a well flush against the active.
        let mut v = probe(K::Lv, 10.0, 5.0, 0.27);
        v.extend(probe(K::Lv, 20.0, 5.0, 0.265));
        v.extend(probe(K::Lv, 30.0, 5.0, 0.0));
        // 5 V: 0.23 (clean) and 0.225.
        v.extend(probe(K::Mv, 10.0, 15.0, 0.23));
        v.extend(probe(K::Mv, 20.0, 15.0, 0.225));
        v
    });

    // --- Y.PL.1: poly width, 0.13 at 3.3 V; at 5 V the manual gives no number and
    // upstream reads that as "no poly here at all".  PLFUSE is what either class excludes.
    hwrite("Y.PL.1.h1", {
        // 3.3 V, 0.13 wide: clean.
        let mut v = h.mk(K::Lv, 10.0, 5.0, 10.13, 8.0);
        v.push(rect(h.poly, 10.0, 5.0, 10.13, 8.0));
        // 3.3 V, 0.125 wide.
        v.extend(h.mk(K::Lv, 20.0, 5.0, 20.125, 8.0));
        v.push(rect(h.poly, 20.0, 5.0, 20.125, 8.0));
        // 3.3 V, 0.125 wide with PLFUSE over its lower half: the rule excludes a poly
        // that meets PLFUSE at all, so the whole line is out of it.
        v.extend(h.mk(K::Lv, 30.0, 5.0, 30.125, 8.0));
        v.push(rect(h.poly, 30.0, 5.0, 30.125, 8.0));
        v.push(rect(h.plfuse, 29.8, 4.8, 30.3, 6.5));
        // 5 V, a 0.5 wide line, which is wide enough at any voltage the manual names.
        v.extend(h.mk(K::Mv, 40.0, 5.0, 40.5, 8.0));
        v.push(rect(h.poly, 40.0, 5.0, 40.5, 8.0));
        // 5 V, the same line wholly under PLFUSE.
        v.extend(h.mk(K::Mv, 50.0, 5.0, 50.5, 8.0));
        v.push(rect(h.poly, 50.0, 5.0, 50.5, 8.0));
        v.push(rect(h.plfuse, 49.8, 4.8, 50.7, 8.2));
        v
    });

    // --- Y.PL.2: the channel length, 0.13 at 3.3 V and 0.47 at 5 V, and not read at all
    // where OTP_MK covers the gate.
    hwrite("Y.PL.2.h1", {
        let probe = |k: K, x: f64, gate: f64, otp: bool| {
            let mut v = h.mk(k, x - 0.4, 4.6, x + gate + 1.4, 7.4);
            v.extend(h.dev(x, 5.0, gate, 0.5, 0.4));
            if k == K::Mv {
                v.push(rect(h.plfuse, x + 0.2, 4.5, x + gate + 0.8, 7.5));
            }
            if otp {
                v.push(rect(h.otp, x - 0.6, 4.4, x + gate + 1.6, 7.6));
            }
            v
        };
        // 3.3 V: 0.13 (clean), 0.125, and 0.125 under OTP_MK.
        let mut v = probe(K::Lv, 10.0, 0.13, false);
        v.extend(probe(K::Lv, 20.0, 0.125, false));
        v.extend(probe(K::Lv, 30.0, 0.125, true));
        // 5 V: 0.47 (clean) and 0.465.
        v.extend(probe(K::Mv, 40.0, 0.47, false));
        v.extend(probe(K::Mv, 50.0, 0.465, false));
        v
    });

    // --- Y.PL.4: the poly end cap, 0.16 - a 5 V rule, so the same short cap at 3.3 V is
    // the ordinary PL.4 elsewhere and nothing of this deck's.
    hwrite("Y.PL.4.h1", {
        let probe = |k: K, x: f64, endcap: f64| {
            let mut v = h.mk(k, x - 0.4, 4.4, x + 2.0, 7.6);
            v.extend(h.dev(x, 5.0, 0.6, 0.5, endcap));
            if k == K::Mv {
                v.push(rect(h.plfuse, x + 0.3, 4.3, x + 1.3, 7.7));
            }
            v
        };
        let mut v = probe(K::Mv, 10.0, 0.16);
        v.extend(probe(K::Mv, 20.0, 0.155));
        v.extend(probe(K::Lv, 30.0, 0.155));
        v
    });

    // --- Y.DF.6: the source/drain overhang, 0.15 - also a 5 V rule, and not read where
    // OTP_MK covers the active.
    hwrite("Y.DF.6.h1", {
        let probe = |k: K, x: f64, sd: f64, otp: bool| {
            let mut v = h.mk(k, x - 0.4, 4.6, x + 0.6 + 2.0 * sd + 0.4, 7.4);
            v.extend(h.dev(x, 5.0, 0.6, sd, 0.4));
            if k == K::Mv {
                v.push(rect(h.plfuse, x + sd - 0.1, 4.5, x + sd + 0.7, 7.5));
            }
            if otp {
                v.push(rect(h.otp, x - 0.6, 4.4, x + 0.6 + 2.0 * sd + 0.6, 7.6));
            }
            v
        };
        let mut v = probe(K::Mv, 10.0, 0.15, false);
        v.extend(probe(K::Mv, 20.0, 0.145, false));
        v.extend(probe(K::Mv, 30.0, 0.145, true));
        v.extend(probe(K::Lv, 40.0, 0.145, false));
        v
    });

    // --- Y.PL.5a and Y.PL.5b: field poly to the active beside it, 0.04 at 3.3 V and 0.2
    // at 5 V.  The two ids are one measurement in the manual as in both decks - 5a for an
    // unrelated active, 5b for a related one, at the same number - so every violation
    // here carries both.
    hwrite("Y.PL.5.h1", {
        let probe = |k: K, x: f64, y: f64, gap: f64| {
            let mut v = h.mk(k, x, y, x + 2.5 + gap, y + 2.0);
            v.push(rect(h.comp, x, y, x + 2.0, y + 2.0));
            v.push(rect(h.poly, x + 2.0 + gap, y, x + 2.5 + gap, y + 2.0));
            if k == K::Mv {
                v.push(rect(
                    h.plfuse,
                    x + 1.9 + gap,
                    y - 0.2,
                    x + 2.6 + gap,
                    y + 2.2,
                ));
            }
            v
        };
        // 3.3 V: 0.04 (clean), 0.035, and poly flush against the active.
        let mut v = probe(K::Lv, 10.0, 5.0, 0.04);
        v.extend(probe(K::Lv, 20.0, 5.0, 0.035));
        v.extend(probe(K::Lv, 30.0, 5.0, 0.0));
        // 5 V: 0.2 (clean) and 0.195.
        v.extend(probe(K::Mv, 10.0, 15.0, 0.2));
        v.extend(probe(K::Mv, 20.0, 15.0, 0.195));
        v
    });
}
