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

    hardening(pdk);
}

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.9 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/pplus/PP.<rule>.h<n>.gds.gz` with a case in the
// `hardening_pplus` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/pplus.md.
//
// What these draw is the deck's own conditions - which NCOMP a spacing rule measures to
// and which well decides its value, the butted P+/N+ pair and the guard-ring marker that
// exempt, the band either side of a well edge, the resistor a salicide-block rule leaves
// alone, the poly the implant may not touch - at the bound and one grid step past it.
// The generic classes are the engine family's.

/// Layers the hardening patterns draw on.
struct H {
    np: (i16, i16),
    pp: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    sab: (i16, i16),
    nw: (i16, i16),
    dn: (i16, i16),
    pw: (i16, i16),
    res: (i16, i16),
    gr: (i16, i16),
}

fn hardening(pdk: &PdkConfig) {
    let h = H {
        np: layer(pdk, "nplus"),
        pp: layer(pdk, "pplus"),
        comp: layer(pdk, "comp"),
        poly: layer(pdk, "poly2_drawn"),
        sab: layer(pdk, "sab"),
        nw: layer(pdk, "nwell"),
        dn: layer(pdk, "dnwell"),
        pw: layer(pdk, "lvpwell"),
        res: layer(pdk, "resistor"),
        gr: layer(pdk, "guard_ring_mk"),
    };
    let write = |id: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{id}.gds.gz"), library("TOP", elems));
    };

    // --- PP.3a: "Space to NCOMP for NCOMP (1) inside LVPWELL (2) outside NWELL and
    // DNWELL - 0.16".  The deck drops the rule for any marker that touches a PCOMP
    // butted to an NCOMP anywhere in the layout, not only the related one.
    //
    // h1 - one L-shaped marker.  Its foot carries a PCOMP butted to an NCOMP at x = 12
    // (a legal butted pair); its arm ends at x = 20, 0.155 from an unrelated NCOMP in
    // the field 8 um away.  The manual's exemption is the butting pair's own edge.
    let pp3a_scene = |with_pair: bool| {
        let mut v = vec![
            rect(h.pp, 10.0, 10.0, 12.0, 11.0),
            rect(h.pp, 10.0, 11.0, 20.0, 12.0),
            rect(h.comp, 20.155, 10.5, 21.155, 11.5),
            rect(h.np, 20.155, 10.3, 21.4, 11.7),
        ];
        if with_pair {
            v.push(rect(h.comp, 10.3, 10.2, 13.5, 10.8));
            v.push(rect(h.np, 12.0, 9.8, 13.8, 10.95));
        }
        v
    };
    write("PP.3a.h1", pp3a_scene(true));
    // h2 - the same scene without the butted pair: the control, PP.3a fires.
    write("PP.3a.h2", pp3a_scene(false));

    // --- PP.3ci/cii: "Space to NCOMP: For Outside DNWELL, inside Nwell: (i) NWELL
    // overlap of NCOMP >= 0.43 - 0.08; (ii) < 0.43 - 0.16".  The deck classifies by
    // whether the NCOMP lies in the well's 0.429 rim or in its core.
    //
    // h1 - one 10 um well holding four scenes.  (a) an NCOMP in the rim along the well's
    // left wall with the marker 0.155 away: PP.3cii; (b) an NCOMP in the core with the
    // marker 0.075 away: PP.3ci; (c) and (d), the same two at 0.16 and 0.08: clean.
    let pp3c_rim = |y: f64, gap: f64| {
        vec![
            rect(h.comp, 10.1, y, 10.4, y + 1.0),
            rect(h.np, 10.0, y - 0.1, 10.5, y + 1.1),
            rect(h.pp, 10.4 + gap, y, 11.5, y + 1.0),
        ]
    };
    let pp3c_core = |y: f64, gap: f64| {
        vec![
            rect(h.comp, 14.0, y, 15.0, y + 1.0),
            rect(h.np, 13.9, y - 0.1, 15.0, y + 1.1),
            rect(h.pp, 15.0 + gap, y, 16.0, y + 1.0),
        ]
    };
    let mut v = vec![rect(h.nw, 10.0, 10.0, 20.0, 20.0)];
    v.extend(pp3c_rim(12.0, 0.155));
    v.extend(pp3c_core(12.0, 0.075));
    v.extend(pp3c_rim(16.0, 0.16));
    v.extend(pp3c_core(16.0, 0.08));
    write("PP.3ci.h1", v);

    // --- PP.5ci/cii: "Extension beyond COMP: For Inside DNWELL, inside LVPWELL: (i) For
    // LVPWELL overlap of Pplus >= 0.43 - 0.02; (ii) < 0.43 - 0.16".  The same band, read
    // on the extension instead of the spacing.
    //
    // h1 - one P-well in a deep well holding four taps.  (a) in the well's core with the
    // marker 0.015 past the COMP: PP.5ci; (b) the same at 0.02: clean; (c) a tap whose
    // left wall is 0.2 inside the well - in the band - with the marker 0.155 past it:
    // PP.5cii; (d) the same at 0.16: clean.
    let pp5c_tap = |x: f64, y: f64, margin: f64| {
        vec![
            rect(h.comp, x, y, x + 1.0, y + 1.0),
            rect(h.pp, x - margin, y - 0.3, x + 1.3, y + 1.3),
        ]
    };
    let mut v = vec![
        rect(h.dn, 10.0, 8.0, 30.0, 30.0),
        rect(h.pw, 11.0, 9.0, 28.0, 29.0),
    ];
    v.extend(pp5c_tap(20.0, 12.0, 0.015));
    v.extend(pp5c_tap(20.0, 16.0, 0.02));
    v.extend(pp5c_tap(11.2, 20.0, 0.155));
    v.extend(pp5c_tap(11.2, 24.0, 0.16));
    write("PP.5ci.h1", v);

    // --- PP.5b: "Extension beyond COMP for COMP (1) Inside NWELL (2) outside LVPWELL but
    // inside DNWELL - 0.16".  Which rule a tap falls under is decided by where it sits,
    // and a marker wall drawn exactly on a well wall sits on neither side of it.
    //
    // h1 - a P-tap just outside an N-well, its marker's left wall drawn on the well's
    // right wall and 0.1 short of the COMP.  The COMP is outside the well, so this is
    // PP.5d's case: the marker is 0 from the N-well, so PP.5dii's 0.16 applies.
    write(
        "PP.5b.h1",
        vec![
            rect(h.nw, 10.0, 10.0, 15.0, 15.0),
            rect(h.comp, 15.1, 11.0, 16.1, 12.0),
            rect(h.pp, 15.0, 10.7, 16.3, 12.3),
        ],
    );

    // --- PP.5di: "Extension beyond COMP: For Outside DNWELL (i) For Pplus to NWELL
    // space >= 0.43 - 0.02".  The deck drops every marker that touches GUARD_RING_MK;
    // the manual's section says nothing about a guard ring.
    //
    // h1 - two identical P-taps in the field with the marker 0.015 past the COMP.
    // (a) bare: PP.5di; (b) with GUARD_RING_MK over it.
    write(
        "PP.5di.h1",
        vec![
            rect(h.comp, 10.0, 10.0, 11.0, 11.0),
            rect(h.pp, 9.985, 9.7, 11.3, 11.3),
            rect(h.comp, 16.0, 10.0, 17.0, 11.0),
            rect(h.pp, 15.985, 9.7, 17.3, 11.3),
            rect(h.gr, 15.5, 9.5, 17.5, 11.5),
        ],
    );

    // --- PP.9: "Overlap of unsalicided Poly2 - 0.18".  The deck exempts a marked poly
    // resistor from PP.9 and not from NP.9; the manual's two sections carry the same
    // sentence.
    //
    // h1 - two poly bars under a salicide block with the marker 0.175 past the left wall
    // of each.  (a) marked as a resistor; (b) bare: PP.9.
    write(
        "PP.9.h1",
        vec![
            rect(h.poly, 10.0, 10.0, 12.0, 10.5),
            rect(h.sab, 9.8, 9.8, 12.2, 10.7),
            rect(h.res, 9.9, 9.9, 12.1, 10.6),
            rect(h.pp, 9.825, 9.8, 12.2, 10.7),
            rect(h.poly, 20.0, 10.0, 22.0, 10.5),
            rect(h.sab, 19.8, 9.8, 22.2, 10.7),
            rect(h.pp, 19.825, 9.8, 22.2, 10.7),
        ],
    );

    // --- PP.7 and PP.10: the salicide block's neighbours.  PP.7 is a space of 0.18 to an
    // unsalicided poly, PP.10 an overlap of 0.18 of an unsalicided COMP.
    //
    // h1 - (a) a COMP under a block, the marker 0.175 past its left wall: PP.10; (b) a
    // COMP under a block that the marker's right wall cuts in half - an overlap of
    // nothing on that side; (c) a poly bar under a block whose left wall the marker's
    // right wall touches: a space of nothing, PP.7.
    write(
        "PP.10.h1",
        vec![
            rect(h.comp, 10.0, 10.0, 11.0, 11.0),
            rect(h.sab, 9.9, 9.9, 11.1, 11.1),
            rect(h.pp, 9.825, 9.8, 11.2, 11.2),
            rect(h.comp, 15.0, 10.0, 16.0, 11.0),
            rect(h.sab, 14.9, 9.9, 16.1, 11.1),
            rect(h.pp, 14.6, 9.8, 15.5, 11.2),
            rect(h.poly, 20.0, 10.0, 21.0, 11.0),
            rect(h.sab, 19.9, 9.9, 21.1, 11.1),
            rect(h.pp, 18.6, 9.8, 20.0, 11.2),
        ],
    );

    // --- PP.4a: "Space related to N-channel gate at a butting edge parallel to gate -
    // 0.32".  The manual measures a butting edge that faces the gate; a butting edge
    // round the corner from it has no facing gate edge at all.
    //
    // h1 - an NMOS whose COMP has an arm going up on the far side of the gate.  The
    // butted P+/N+ edge is horizontal, at y = 12.7, x 11.2..12.28; the nearest gate edge
    // is the gate's left wall, x = 12.5, y 11.5..12.5.  The two do not face each other,
    // and the corner-to-corner distance is 0.297.
    write(
        "PP.4a.h1",
        vec![
            rect(h.comp, 11.0, 11.5, 15.0, 12.5),
            rect(h.comp, 11.2, 12.5, 12.28, 13.5),
            rect(h.poly, 12.5, 11.5, 12.78, 12.5),
            rect(h.np, 10.7, 11.2, 15.3, 12.7),
            rect(h.pp, 11.0, 12.7, 12.32, 13.8),
        ],
    );

    // h2 - the butting edge facing the gate, the bound.  (a) a vertical butted edge
    // 0.315 from the gate's right wall, the two fully overlapping: PP.4a; (b) the same
    // at 0.32: clean.
    let pp4a_face = |y: f64, gap: f64| {
        let xb = 12.28 + gap;
        vec![
            rect(h.comp, 11.0, y + 0.5, 16.0, y + 1.5),
            rect(h.poly, 12.0, y + 0.5, 12.28, y + 1.5),
            rect(h.np, 10.7, y + 0.2, xb, y + 1.8),
            rect(h.pp, xb, y + 0.2, 16.3, y + 1.8),
        ]
    };
    let mut v = pp4a_face(11.0, 0.315);
    v.extend(pp4a_face(16.0, 0.32));
    write("PP.4a.h2", v);

    // --- PP.5a: "Overlap of P-channel gate - 0.23".  A P-channel gate is derived from
    // the PCOMP, which is COMP under the marker, so a gate lies inside the marker by
    // construction: a marker edge that cuts a poly bar re-cuts the gate with it.
    //
    // h1 - (a) a gate the marker holds by 0.225 above and below: PP.5a; (b) a gate whose
    // poly bar the marker's right edge cuts in half - the overlap there is nothing.
    write(
        "PP.5a.h1",
        vec![
            rect(h.nw, 9.0, 9.0, 14.0, 12.0),
            rect(h.comp, 10.0, 10.0, 13.0, 11.0),
            rect(h.poly, 11.0, 9.7, 11.28, 11.3),
            rect(h.pp, 9.7, 9.775, 13.3, 11.225),
            rect(h.nw, 19.0, 9.0, 24.0, 12.0),
            rect(h.comp, 20.0, 10.0, 23.0, 11.0),
            rect(h.poly, 21.0, 9.7, 21.28, 11.3),
            rect(h.pp, 19.7, 9.7, 21.14, 11.3),
        ],
    );

    // --- PP.6: "Overlap with PCOMP butted to NCOMP - 0.22".  The COMP has to reach 0.22
    // past the butting edge, which is the P+ half's length; the three walls the NCOMP
    // shares with the COMP are an overlap of nothing and are not the rule's.
    //
    // h1 - (a) a P+ half 0.215 long: PP.6; (b) 0.22: clean.
    let pp6 = |x: f64, p_len: f64| {
        vec![
            rect(h.comp, x, 10.0, x + 1.2 + p_len, 11.0),
            rect(h.np, x - 0.3, 9.7, x + 1.2, 11.3),
            rect(h.pp, x + 1.2, 9.7, x + 1.5 + p_len, 11.3),
        ]
    };
    let mut v = pp6(10.0, 0.215);
    v.extend(pp6(15.0, 0.22));
    write("PP.6.h1", v);

    // --- PP.11: "Butting Pplus and NCOMP is forbidden within 0.43um of Nwell edge (for
    // outside DNWELL)".  The manual says within 0.43 of the edge; the deck bands only
    // the rim inside the well - NP.11 takes the collar outside it.
    //
    // h1 - three butted P+/N+ edges at a well's right wall.  (a) 0.2 inside it;
    // (b) 0.2 outside it, at the tile line x = 20; (c) 0.5 inside it, clear of the 0.43
    // band, clean.
    write(
        "PP.11.h1",
        vec![
            rect(h.nw, 25.0, 10.0, 30.0, 14.0),
            rect(h.comp, 27.0, 11.0, 31.0, 12.0),
            rect(h.np, 26.0, 10.8, 29.8, 12.2),
            rect(h.pp, 29.8, 10.8, 31.3, 12.2),
            rect(h.nw, 17.0, 10.0, 20.0, 14.0),
            rect(h.comp, 19.5, 11.0, 22.2, 12.0),
            rect(h.np, 19.2, 10.8, 20.2, 12.2),
            rect(h.pp, 20.2, 10.8, 22.5, 12.2),
            rect(h.nw, 35.0, 10.0, 40.0, 14.0),
            rect(h.comp, 37.0, 11.0, 41.0, 12.0),
            rect(h.np, 36.0, 10.8, 39.5, 12.2),
            rect(h.pp, 39.5, 10.8, 41.3, 12.2),
        ],
    );

    // --- PP.12: "Overlap with N-channel Poly2 gate extension is forbidden within 0.32um
    // of N-channel gate".  The reach runs along the poly, not across the air.
    //
    // h1 - (a) a U of poly whose left leg carries an N-channel gate and whose right leg
    // is 0.22 of air away from that gate but 5 um away along the poly; the marker covers
    // the right leg and reaches into the plain 0.32 disc around the gate.  (b) a
    // straight poly bar above a gate with the marker starting 0.315 up it: PP.12.
    write(
        "PP.12.h1",
        vec![
            rect(h.comp, 11.0, 11.2, 12.35, 11.8),
            rect(h.np, 10.7, 10.9, 12.39, 12.1),
            rect(h.poly, 12.0, 11.0, 12.28, 14.0),
            rect(h.poly, 12.0, 13.72, 12.78, 14.0),
            rect(h.poly, 12.5, 11.0, 12.78, 14.0),
            rect(h.pp, 12.55, 10.6, 14.0, 11.3),
            rect(h.comp, 25.0, 11.2, 27.0, 11.8),
            rect(h.np, 24.7, 10.9, 27.3, 12.1),
            rect(h.poly, 26.0, 11.0, 26.28, 14.0),
            rect(h.pp, 25.5, 12.115, 27.0, 13.0),
        ],
    );
}
