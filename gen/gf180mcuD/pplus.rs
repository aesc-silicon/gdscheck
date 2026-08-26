// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! P+ implant: a good and a bad pattern for every rule in the `pplus` deck.
//!
//! The deck that most needs drawn patterns. Its foundry case reports 222 `PP.1` markers
//! where we report 392, so no count comparison on it means anything, and half the rules
//! come in near/far pairs split by a 0.429 um band around a well — a classifier that has
//! already caused two wrong measurements in this port. Here each fixture sits deliberately
//! on one side of that band, and the clean half is a hard zero.
//!
//! Unlike the resistor deck there is no single base cell: an implant rule is about the
//! implant *against something* - a well, a COMP, a gate, the other implant - so each rule
//! gets the smallest scene that puts those together, and its good half is that same scene
//! with the one measured dimension made legal.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/pplus";

/// Comfortably clear of every limit in the deck, for the dimensions a fixture is not about.
const CLEAR: f64 = 0.6;
/// Half a grid step under a limit: the smallest real violation on a 5 nm grid.
const UNDER: f64 = 0.005;

struct Ctx {
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly: (i16, i16),
    sab: (i16, i16),
}

impl Ctx {
    /// A COMP with the P+ implant over it, the implant reaching `margin` beyond on all
    /// sides — the shape every extension rule measures.
    fn pcomp(&self, x: f64, y: f64, w: f64, h: f64, margin: f64) -> Vec<gds21::GdsElement> {
        vec![
            rect(self.comp, x, y, x + w, y + h),
            rect(
                self.pplus,
                x - margin,
                y - margin,
                x + w + margin,
                y + h + margin,
            ),
        ]
    }

    /// The same with the N+ implant: an NCOMP. The implant reach is the caller's, since
    /// a spacing rule measures to the *NCOMP* - COMP and Nplus together - and an implant
    /// drawn generously past the COMP would swallow the gap under test.
    fn ncomp(&self, x: f64, y: f64, w: f64, h: f64, reach: f64) -> Vec<gds21::GdsElement> {
        vec![
            rect(self.comp, x, y, x + w, y + h),
            rect(
                self.nplus,
                x - reach,
                y - reach,
                x + w + reach,
                y + h + reach,
            ),
        ]
    }

    /// A transistor: a COMP with a poly stripe crossing it and extending past on both
    /// ends. `implant` decides whether the gate is N- or P-channel.
    #[allow(clippy::too_many_arguments)]
    fn fet(
        &self,
        implant: (i16, i16),
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        gate_x: f64,
        gate_w: f64,
        implant_margin: f64,
    ) -> Vec<gds21::GdsElement> {
        vec![
            rect(self.comp, x, y, x + w, y + h),
            rect(
                implant,
                x - implant_margin,
                y - implant_margin,
                x + w + implant_margin,
                y + h + implant_margin,
            ),
            // The stripe runs past the COMP top and bottom: that overhang is the gate
            // extension PP.12 and PP.4b are about.
            rect(self.poly, gate_x, y - 0.8, gate_x + gate_w, y + h + 0.8),
        ]
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let c = Ctx {
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly: layer(pdk, "poly2_drawn"),
        sab: layer(pdk, "sab"),
    };
    let o = OFFSET;

    let write = |id: &str, polarity: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    // Each rule's scene, as a function of the one dimension it measures. Called twice:
    // once at the limit (clean) and once a grid step inside it (the violation).
    let scene = |id: &str, good: Vec<gds21::GdsElement>, bad: Vec<gds21::GdsElement>| {
        write(id, "good", good);
        write(id, "bad", bad);
    };

    // --- PP.1 / PP.2 / PP.8a / PP.8b: the implant on its own -----------------
    let bar = |w: f64| vec![rect(c.pplus, o, o, o + w, o + 2.0)];
    scene("PP.1", bar(0.4), bar(0.4 - UNDER));

    let pair = |gap: f64| {
        vec![
            rect(c.pplus, o, o, o + 1.0, o + 1.0),
            rect(c.pplus, o + 1.0 + gap, o, o + 2.0 + gap, o + 1.0),
        ]
    };
    scene("PP.2", pair(0.4), pair(0.4 - UNDER));

    // 0.35 um^2 exactly, against one grid step under it, both wider than PP.1's minimum.
    let island = |h: f64| vec![rect(c.pplus, o, o, o + 0.6, o + h)];
    scene("PP.8a", island(0.6), island(0.58));

    // A ring, judged on the area of its hole.
    let ring = |hole: f64| {
        vec![
            rect(c.pplus, o, o, o + hole + 2.0, o + hole + 2.0),
            rect(c.pplus, o + 1.0, o + 1.0, o + 1.0 + hole, o + 1.0 + hole),
        ]
    };
    // The hole is drawn as the gap between four bars so it is a real hole, not an island.
    let ring_hole = |hole: f64| {
        let outer = hole + 2.0;
        vec![
            rect(c.pplus, o, o, o + outer, o + 1.0),
            rect(c.pplus, o, o + 1.0 + hole, o + outer, o + outer),
            rect(c.pplus, o, o, o + 1.0, o + outer),
            rect(c.pplus, o + 1.0 + hole, o, o + outer, o + outer),
        ]
    };
    let _ = ring;
    scene("PP.8b", ring_hole(0.6), ring_hole(0.59));

    // --- PP.3*: spacing to an NCOMP, by where the two sit -------------------
    // PP.3a: both clear of every well.
    let p3a = |gap: f64| {
        let mut v = c.ncomp(o, o, 2.0, 2.0, 0.1);
        v.push(rect(c.pplus, o + 2.0 + gap, o, o + 4.0 + gap, o + 2.0));
        v
    };
    scene("PP.3a", p3a(0.16), p3a(0.16 - UNDER));

    // PP.3bi / PP.3bii: the Pplus inside a DNWELL, split by whether the NCOMP is within
    // 0.429 um of an LVPWELL.
    //
    // The NCOMP has to sit in an N-well of its own, and that well has to stay clear of the
    // DNWELL. PP.3a covers `ncomp` minus `nwell_n_dn`, so an NCOMP anywhere else is also a
    // PP.3a case and both rules fire at once — the 0.16 um general limit and the 0.08 um
    // one for this arrangement. Only a well that touches no DNWELL takes it out of PP.3a.
    let p3b = |gap: f64, lvpwell: bool| {
        let mut v = vec![
            rect(c.nwell, o, o, o + 2.0, o + 2.0),
            rect(c.dnwell, o + 2.05, o - 2.0, o + 12.0, o + 6.0),
        ];
        if lvpwell {
            // Reaches to within 0.429 um of the NCOMP without touching it.
            v.push(rect(c.lvpwell, o + 2.05, o - 1.0, o + 4.0, o + 4.0));
        }
        v.extend(c.ncomp(o, o, 2.0, 2.0, 0.0));
        v.push(rect(c.pplus, o + 2.0 + gap, o, o + 4.0 + gap, o + 2.0));
        v
    };
    scene("PP.3bi", p3b(0.08, false), p3b(0.08 - UNDER, false));
    scene("PP.3bii", p3b(0.16, true), p3b(0.16 - UNDER, true));

    // PP.3ci / PP.3cii: inside an NWELL, split by distance to its edge.
    let p3c = |gap: f64, near_edge: bool| {
        // The NCOMP sits either well inside the well or in the 0.429 um rim along it.
        let inset = if near_edge { 0.2 } else { 1.0 };
        let mut v = vec![rect(
            c.nwell,
            o - inset,
            o - inset,
            o + 12.0,
            o + 2.0 + inset,
        )];
        v.extend(c.ncomp(o, o, 2.0, 2.0, 0.1));
        v.push(rect(c.pplus, o + 2.0 + gap, o, o + 4.0 + gap, o + 2.0));
        v
    };
    scene("PP.3ci", p3c(0.08, false), p3c(0.08 - UNDER, false));
    scene("PP.3cii", p3c(0.16, true), p3c(0.16 - UNDER, true));

    // PP.3d / PP.3e: the two implants over one COMP. They are the same set - COMP and
    // Nplus and Pplus - so neither can be drawn without the other.
    let p3de = |on_comp: bool| {
        let mut v = vec![rect(c.comp, o, o, o + 3.0, o + 2.0)];
        // Pplus covers the COMP whole, so every PCOMP edge is a COMP edge with a clear
        // margin and no extension rule has anything to measure.
        v.push(rect(
            c.pplus,
            o - CLEAR,
            o - CLEAR,
            o + 3.0 + CLEAR,
            o + 2.0 + CLEAR,
        ));
        // The violation is the Nplus reaching onto that same COMP; clear of it, nothing.
        let x1 = if on_comp { o + 1.5 } else { o - 0.2 };
        v.push(rect(c.nplus, o - 2.0, o - CLEAR, x1, o + 2.0 + CLEAR));
        v
    };
    scene("PP.3d", p3de(false), p3de(true));
    scene("PP.3e", p3de(false), p3de(true));

    // --- PP.5a: the implant must cover a P-channel gate ----------------------
    let p5a = |margin: f64| {
        let mut v = vec![rect(c.nwell, o - 2.0, o - 2.0, o + 6.0, o + 5.0)];
        v.extend(c.fet(c.pplus, o, o, 3.0, 2.0, o + 1.2, 0.6, margin));
        v
    };
    scene("PP.5a", p5a(0.23), p5a(0.23 - UNDER));

    // --- PP.6: the COMP must reach past the NCOMP butted into it -------------
    let p6 = |margin: f64| {
        let mut v = vec![rect(c.comp, o, o, o + 3.0, o + 2.0)];
        // Nplus covers a strip inside the COMP, so the NCOMP is bounded by COMP on three
        // sides and by the implant boundary on the fourth; `margin` is that reach.
        v.push(rect(
            c.nplus,
            o - CLEAR,
            o - CLEAR,
            o + 3.0 - margin,
            o + 2.0 + CLEAR,
        ));
        v.push(rect(
            c.pplus,
            o + 3.0 - margin,
            o - CLEAR,
            o + 3.0 + CLEAR,
            o + 2.0 + CLEAR,
        ));
        v
    };
    scene("PP.6", p6(0.22), p6(0.22 - UNDER));

    // --- PP.7 / PP.9: unsalicided Poly2, beside and under the implant --------
    let p7 = |gap: f64| {
        vec![
            rect(c.pplus, o, o, o + 2.0, o + 2.0),
            rect(c.poly, o + 2.0 + gap, o, o + 3.0 + gap, o + 2.0),
            rect(
                c.sab,
                o + 2.0 + gap - 0.1,
                o - 0.1,
                o + 3.0 + gap + 0.1,
                o + 2.1,
            ),
        ]
    };
    scene("PP.7", p7(0.18), p7(0.18 - UNDER));

    let p9 = |margin: f64| {
        vec![
            rect(c.poly, o, o, o + 1.0, o + 1.0),
            rect(c.sab, o - 0.1, o - 0.1, o + 1.1, o + 1.1),
            rect(
                c.pplus,
                o - margin,
                o - margin,
                o + 1.0 + margin,
                o + 1.0 + margin,
            ),
        ]
    };
    scene("PP.9", p9(0.18), p9(0.18 - UNDER));

    // --- PP.10: unsalicided COMP under the implant --------------------------
    let p10 = |margin: f64| {
        vec![
            rect(c.comp, o, o, o + 1.0, o + 1.0),
            rect(c.sab, o - 0.1, o - 0.1, o + 1.1, o + 1.1),
            rect(
                c.pplus,
                o - margin,
                o - margin,
                o + 1.0 + margin,
                o + 1.0 + margin,
            ),
        ]
    };
    scene("PP.10", p10(0.18), p10(0.18 - UNDER));

    // --- PP.5b / PP.5c* / PP.5d*: extension beyond COMP, by where it sits ----
    // PP.5b: inside a DNWELL and clear of any LVPWELL, which is the branch that excludes
    // the PP.5c and PP.5d cases outright.
    let p5b = |margin: f64| {
        let mut v = vec![rect(c.dnwell, o - 2.0, o - 2.0, o + 6.0, o + 6.0)];
        v.extend(c.pcomp(o, o, 2.0, 2.0, margin));
        v
    };
    scene("PP.5b", p5b(0.16), p5b(0.16 - UNDER));

    // PP.5ci / PP.5cii: inside a DNWELL *and* an LVPWELL, split by whether the COMP edge
    // falls in the well's inner region or the 0.429 um rim along its edge.
    let p5c = |margin: f64, near_edge: bool| {
        let inset = if near_edge { 0.2 } else { 1.0 };
        vec![
            rect(c.dnwell, o - 3.0, o - 3.0, o + 7.0, o + 7.0),
            rect(
                c.lvpwell,
                o - inset,
                o - inset,
                o + 2.0 + inset,
                o + 2.0 + inset,
            ),
        ]
        .into_iter()
        .chain(c.pcomp(o, o, 2.0, 2.0, margin))
        .collect()
    };
    scene("PP.5ci", p5c(0.02, false), p5c(0.02 - UNDER, false));
    scene("PP.5cii", p5c(0.16, true), p5c(0.16 - UNDER, true));

    // PP.5di / PP.5dii: outside any DNWELL, split by whether an N-well is within 0.429 um.
    let p5d = |margin: f64, near_nwell: bool| {
        let mut v = c.pcomp(o, o, 2.0, 2.0, margin);
        if near_nwell {
            v.push(rect(c.nwell, o + 2.0 + margin + 0.2, o, o + 6.0, o + 2.0));
        }
        v
    };
    scene("PP.5di", p5d(0.02, false), p5d(0.02 - UNDER, false));
    scene("PP.5dii", p5d(0.16, true), p5d(0.16 - UNDER, true));

    // --- PP.4a / PP.11: a butted Pplus/NCOMP edge ---------------------------
    // The two implants drawn edge to edge over one COMP: they touch without overlapping,
    // which is what makes the shared edge a butting edge.
    let butted = |x: f64, y: f64, split: f64| {
        vec![
            rect(c.comp, x, y, x + 3.0, y + 2.0),
            rect(c.nplus, x - CLEAR, y - CLEAR, x + split, y + 2.0 + CLEAR),
            rect(
                c.pplus,
                x + split,
                y - CLEAR,
                x + 3.0 + CLEAR,
                y + 2.0 + CLEAR,
            ),
        ]
    };
    // PP.4a: the butting edge must stay 0.32 um from an N-channel gate.
    let p4a = |gap: f64| {
        let mut v = butted(o, o, 1.5);
        // An NMOS to the left, its gate `gap` from the butting edge at o + 1.5.
        v.push(rect(
            c.poly,
            o + 1.5 - gap - 0.3,
            o - 0.8,
            o + 1.5 - gap,
            o + 2.8,
        ));
        v
    };
    scene("PP.4a", p4a(0.32), p4a(0.32 - UNDER));

    // PP.11: a butting edge is forbidden in the band along a well edge.
    let p11 = |inside: bool| {
        // The N-well edge sits either 0.2 um from the butting edge (inside the band) or
        // 0.6 um from it (clear of it).
        // The well edge sits either 0.2 um past the butting edge - putting the edge in
        // the 0.429 um rim inside the well - or 0.6 um past it, clear of the rim.
        let d = if inside { 0.2 } else { 0.6 };
        let mut v = butted(o, o, 1.5);
        v.push(rect(c.nwell, o - 3.0, o - 3.0, o + 1.5 + d, o + 5.0));
        v
    };
    scene("PP.11", p11(false), p11(true));

    // --- PP.12: no implant on the N-channel gate's Poly2 extension ----------
    let p12 = |over: bool| {
        let mut v = c.fet(c.nplus, o, o, 3.0, 2.0, o + 1.2, 0.6, CLEAR);
        // Covering the stripe's overhang is PP.12's violation, and covering rather than
        // approaching is deliberate: a Pplus that merely comes near the overhang is
        // within PP.4b's 0.22 um of it as well, and one drawn over it leaves no facing
        // pair for that rule to measure.
        v.push(if over {
            rect(c.pplus, o + 0.9, o + 2.0 + 0.2, o + 2.1, o + 2.8)
        } else {
            rect(
                c.pplus,
                o + 1.2 + 0.6 + 0.4,
                o + 2.0 + 0.2,
                o + 4.0,
                o + 2.8,
            )
        });
        v
    };
    scene("PP.12", p12(false), p12(true));

    // --- PP.4b: clearance to that same extension, within 0.32 um of the channel -------
    // The island sits 0.2 um above the COMP, so it is inside the 0.32 um window the rule
    // is scoped to, and `gap` from the stripe's side. Near enough to measure, far enough
    // not to touch — touching would be PP.12's violation, not this one.
    let p4b = |gap: f64| {
        let mut v = c.fet(c.nplus, o, o, 3.0, 2.0, o + 1.2, 0.6, CLEAR);
        v.push(rect(
            c.pplus,
            o + 1.2 + 0.6 + gap,
            o + 2.0 + 0.2,
            o + 4.0,
            o + 2.6,
        ));
        v
    };
    scene("PP.4b", p4b(0.22), p4b(0.22 - UNDER));
}
