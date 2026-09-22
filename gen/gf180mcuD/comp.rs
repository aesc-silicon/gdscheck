// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Active area: a good and a bad pattern for every rule in the `comp` deck.
//!
//! Two rules apply to almost every fixture here rather than to one of them.  DF.12 fires
//! on any active with no implant over it, so every comp drawn below carries one.  DF.13
//! and DF.14 then ask that no source/drain sit further than 20 µm (15 at medium voltage)
//! from a well tie of the opposite type, so every fixture that draws a source/drain also
//! draws the tie that answers for it - an N-tie for a P source/drain, a P-tie for an N
//! one, and only the one the fixture needs.  Where the fixture has a well of its own the
//! tie goes inside it, as it would in a real cell: DF.18 keeps a substrate P-tie 2.5 µm
//! off any deep well, but a P-tie in the P-well inside that deep well is not a substrate
//! tie at all and the rule does not reach it.
//!
//! The voltage split costs nothing: the deck reads medium voltage as "comp overlapping
//! Dualgate" and low voltage as "comp that Dualgate does not touch", so a marker over the
//! whole scene moves every rule in it from one column to the other, and a fixture for one
//! voltage is silent on the other's rule.

use super::OFFSET;
use crate::helpers::{diamond, layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/comp";

struct Ctx {
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly2: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    dualgate: (i16, i16),
    mos_cap: (i16, i16),
    drc_bjt: (i16, i16),
}

/// An active with an implant over it: `n` picks N+ or P+.
fn implanted(c: &Ctx, n: bool, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    let l = if n { c.nplus } else { c.pplus };
    vec![
        rect(c.comp, x0, y0, x1, y1),
        rect(l, x0 - 0.2, y0 - 0.2, x1 + 0.2, y1 + 0.2),
    ]
}

/// A P-tie in the substrate, which answers DF.14 for an N source/drain.  Not a
/// source/drain itself, so it needs no tie in turn.
fn ptie(c: &Ctx, x: f64, y: f64) -> Vec<GdsElement> {
    implanted(c, false, x, y, x + 2.0, y + 2.0)
}

/// An N-tie in its own N-well, which answers DF.13 for a P source/drain.
fn ntie(c: &Ctx, x: f64, y: f64) -> Vec<GdsElement> {
    let mut v = vec![rect(c.nwell, x, y, x + 3.0, y + 3.0)];
    v.extend(implanted(c, true, x + 0.5, y + 0.5, x + 2.5, y + 2.5));
    v
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly2: layer(pdk, "poly2_drawn"),
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        dualgate: layer(pdk, "dualgate"),
        mos_cap: layer(pdk, "mos_cap_mk"),
        drc_bjt: layer(pdk, "drc_bjt"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    // Wrap a scene: add the tie it needs, then the Dualgate that decides its voltage.
    let scene = |mut v: Vec<GdsElement>, mv: bool, tie: Option<(f64, f64)>| {
        if let Some((x, y)) = tie {
            v.extend(ptie(&c, x, y));
        }
        if mv {
            v.push(rect(c.dualgate, o - 2.0, o - 2.0, o + 20.0, o + 16.0));
        }
        v
    };
    // The P-tie most fixtures use, off to the right of a scene no wider than 5 µm.
    let near = Some((o + 6.0, o));
    // Most rules come as a low- and a medium-voltage pair over one drawing.
    let pair =
        |id: &str, f: &dyn Fn(f64, bool) -> Vec<GdsElement>, good: (f64, f64), bad: (f64, f64)| {
            write(&format!("{id}_LV"), "good", f(good.0, false));
            write(&format!("{id}_LV"), "bad", f(bad.0, false));
            write(&format!("{id}_MV"), "good", f(good.1, true));
            write(&format!("{id}_MV"), "bad", f(bad.1, true));
        };

    // DF.1a: the active's own width.
    let df1a = |w: f64, mv: bool| scene(implanted(&c, true, o, o, o + w, o + 2.0), mv, near);
    pair("DF.1a", &df1a, (0.5, 0.5), (0.215, 0.295));

    // DF.1c: the width of an active under a MOS-capacitor marker, which is far wider than
    // a plain active has to be.
    let df1c = |w: f64| {
        let mut v = implanted(&c, true, o, o, o + w, o + 3.0);
        v.push(rect(c.mos_cap, o - 0.3, o - 0.3, o + w + 0.3, o + 3.3));
        scene(v, false, near)
    };
    write("DF.1c", "good", df1c(1.5));
    write("DF.1c", "bad", df1c(0.995));

    // DF.2a: the channel's width, which is the length of the gate edge inside the active.
    // A poly island on the active gives a short one without narrowing the active itself.
    let df2a = |h: f64, mv: bool| {
        let mut v = implanted(&c, true, o, o, o + 3.0, o + 3.0);
        v.push(rect(
            c.poly2,
            o + 1.0,
            o + 1.5 - h * 0.5,
            o + 2.0,
            o + 1.5 + h * 0.5,
        ));
        scene(v, mv, near)
    };
    pair("DF.2a", &df2a, (0.6, 0.6), (0.215, 0.295));

    // DF.2b: an active over 100 µm across.  Drawn as a P-tie, so it is not a source/drain
    // and DF.14 does not ask for a tie 50 µm away inside it.
    let df2b = |w: f64| scene(implanted(&c, false, o, o, o + w, o + w), false, None);
    write("DF.2b", "good", df2b(99.0));
    write("DF.2b", "bad", df2b(101.0));

    // DF.3a: active to active.
    let df3a = |gap: f64, mv: bool| {
        let mut v = implanted(&c, true, o, o, o + 1.0, o + 1.0);
        v.extend(implanted(
            &c,
            true,
            o + 1.0 + gap,
            o,
            o + 2.0 + gap,
            o + 1.0,
        ));
        scene(v, mv, near)
    };
    pair("DF.3a", &df3a, (1.0, 1.0), (0.275, 0.355));

    // DF.3b: an N-tie over a P source/drain - one implant over the other inside a well.
    let df3b = |overlap: f64| {
        let mut v = vec![rect(c.nwell, o, o, o + 4.0, o + 4.0)];
        v.push(rect(c.comp, o + 1.0, o + 1.0, o + 3.0, o + 3.0));
        v.push(rect(c.nplus, o + 0.8, o + 0.8, o + 2.0 + overlap, o + 3.2));
        v.push(rect(c.pplus, o + 2.0, o + 0.8, o + 3.2, o + 3.2));
        scene(v, false, None)
    };
    write("DF.3b", "good", df3b(0.0));
    write("DF.3b", "bad", df3b(0.5));

    // DF.3c: two actives under one bipolar marker - spaced at low voltage, forbidden
    // outright at medium.
    let df3c = |gap: f64, mv: bool| {
        let mut v = implanted(&c, true, o, o, o + 1.0, o + 1.0);
        v.extend(implanted(
            &c,
            true,
            o + 1.0 + gap,
            o,
            o + 2.0 + gap,
            o + 1.0,
        ));
        v.push(rect(c.drc_bjt, o - 0.5, o - 0.5, o + 2.5 + gap, o + 1.5));
        scene(v, mv, near)
    };
    write("DF.3c_LV", "good", df3c(0.4, false));
    write("DF.3c_LV", "bad", df3c(0.315, false));
    // At medium voltage the second active is what breaks it, so the clean half has the
    // marker over one active only.
    write("DF.3c_MV", "good", {
        let mut v = implanted(&c, true, o, o, o + 1.0, o + 1.0);
        v.push(rect(c.drc_bjt, o - 0.5, o - 0.5, o + 1.5, o + 1.5));
        scene(v, true, near)
    });
    write("DF.3c_MV", "bad", df3c(1.0, true));

    // DF.4a: an N-tie in a deep well, to the P-well beside it.
    let df4a = |gap: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 10.0, o + 8.0),
            rect(c.lvpwell, o + 1.0, o + 1.0, o + 5.0, o + 7.0),
        ];
        v.extend(implanted(
            &c,
            true,
            o + 5.0 + gap,
            o + 3.0,
            o + 7.0 + gap,
            o + 5.0,
        ));
        scene(v, mv, None)
    };
    pair("DF.4a", &df4a, (0.5, 0.5), (0.115, 0.155));

    // DF.4b: how far the deep well holds an N-tie.  No P-well, so DF.4a has nothing to
    // measure against.
    let df4b = |enc: f64, mv: bool| {
        let mut v = vec![rect(c.dnwell, o, o, o + 6.0, o + 6.0)];
        v.extend(implanted(&c, true, o + enc, o + 2.0, o + 4.0, o + 4.0));
        scene(v, mv, None)
    };
    pair("DF.4b", &df4b, (1.5, 1.5), (0.615, 0.655));

    // DF.4c: how far an N-well holds a P source/drain.  The tie that answers DF.13 for
    // it sits in the same well - a tie in a well of its own, however near, is no tie for
    // this one - so its well abuts this one and the two merge.
    let df4c = |enc: f64, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 6.0, o + 6.0)];
        v.extend(ntie(&c, o + 6.0, o));
        v.extend(implanted(&c, false, o + enc, o + 2.0, o + 4.0, o + 4.0));
        scene(v, mv, None)
    };
    pair("DF.4c", &df4c, (1.5, 1.5), (0.425, 0.595));

    // DF.4d: how far an N-well holds an N-tie.
    let df4d = |enc: f64, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 6.0, o + 6.0)];
        v.extend(implanted(&c, true, o + enc, o + 2.0, o + 4.0, o + 4.0));
        scene(v, mv, None)
    };
    pair("DF.4d", &df4d, (1.5, 1.5), (0.115, 0.155));

    // DF.4e: how far a deep well holds a P source/drain.  The tie that answers DF.13 is
    // in a drawn well with the source/drain, inside the deep well: the tap's reach runs
    // in the drawn well alone, and a tie in a well of its own is no tie for this one.
    let df4e = |enc: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 8.0, o + 8.0),
            rect(c.nwell, o + 0.3, o + 0.3, o + 7.0, o + 7.0),
        ];
        v.extend(implanted(&c, true, o + 5.5, o + 5.5, o + 6.5, o + 6.5));
        v.extend(implanted(&c, false, o + enc, o + 3.0, o + 5.0, o + 5.0));
        scene(v, mv, None)
    };
    pair("DF.4e", &df4e, (2.0, 2.0), (0.925, 1.095));

    // DF.5: how far the P-well holds a P-tie.  The deep well keeps its own 0.93 µm from
    // that tie, which is DF.4e's business, so the P-well sits well inside it.
    let df5 = |enc: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 10.0, o + 10.0),
            rect(c.lvpwell, o + 2.0, o + 2.0, o + 8.0, o + 8.0),
        ];
        v.extend(implanted(
            &c,
            false,
            o + 2.0 + enc,
            o + 4.0,
            o + 6.0,
            o + 6.0,
        ));
        scene(v, mv, None)
    };
    pair("DF.5", &df5, (1.5, 1.5), (0.115, 0.155));

    // DF.6: how far the active reaches past the gate.
    let df6 = |ext: f64, mv: bool| {
        let mut v = implanted(&c, true, o, o, o + 1.18 + ext, o + 1.0);
        v.push(rect(c.poly2, o + 0.9, o - 0.3, o + 1.18, o + 1.3));
        scene(v, mv, near)
    };
    pair("DF.6", &df6, (0.8, 0.8), (0.235, 0.395));

    // DF.7: a P source/drain in the deep well, to the P-well beside it.  A drawn well
    // over the source/drain and its tie, since the tap's reach for DF.13 runs in the
    // drawn well alone; it reaches under the P-well's edge so the well holds the
    // source/drain by DF.4c's margin at the narrow end of the sweep.
    let df7 = |gap: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 12.0, o + 8.0),
            rect(c.lvpwell, o + 1.0, o + 1.0, o + 5.0, o + 7.0),
            rect(c.nwell, o + 4.9, o + 1.0, o + 11.8, o + 7.0),
        ];
        v.extend(implanted(
            &c,
            false,
            o + 5.0 + gap,
            o + 3.0,
            o + 7.0 + gap,
            o + 5.0,
        ));
        // Fixed, not offset by `gap`: at the wide end of the sweep a tie that moved with
        // it would come inside DF.4b's 0.62 µm of the deep well's own edge.
        v.extend(implanted(&c, true, o + 9.0, o + 3.0, o + 11.0, o + 5.0));
        scene(v, mv, None)
    };
    pair("DF.7", &df7, (1.5, 1.5), (0.425, 0.595));

    // DF.8: how far the P-well holds an N source/drain.
    let df8 = |enc: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 12.0, o + 12.0),
            rect(c.lvpwell, o + 2.0, o + 2.0, o + 10.0, o + 10.0),
        ];
        v.extend(implanted(
            &c,
            true,
            o + 2.0 + enc,
            o + 5.0,
            o + 7.0,
            o + 7.0,
        ));
        v.extend(ptie(&c, o + 7.5, o + 5.0));
        scene(v, mv, None)
    };
    pair("DF.8", &df8, (1.5, 1.5), (0.425, 0.595));

    // DF.9: the active's area, at a legal width.
    let df9 = |h: f64| scene(implanted(&c, true, o, o, o + 0.45, o + h), false, near);
    write("DF.9", "good", df9(0.6));
    write("DF.9", "bad", df9(0.449));

    // DF.10: the area of a hole in the active, at a legal notch.
    let df10 = |hh: f64| {
        let (hw, w, h) = (0.5, 2.5, hh + 2.0);
        let mut v = vec![
            rect(c.comp, o, o, o + w, o + 1.0),
            rect(c.comp, o, o + 1.0 + hh, o + w, o + h),
            rect(c.comp, o, o + 1.0, o + 1.0, o + 1.0 + hh),
            rect(c.comp, o + 1.0 + hw, o + 1.0, o + w, o + 1.0 + hh),
            rect(c.nplus, o - 0.2, o - 0.2, o + w + 0.2, o + h + 0.2),
        ];
        v.extend(ptie(&c, o + 4.0, o));
        v
    };
    write("DF.10", "good", df10(0.52));
    write("DF.10", "bad", df10(0.519));

    // DF.11: the width of an active carrying a butted N+/P+ tie.
    // The P+ half is its own substrate tie, so DF.14 is answered inside the fixture.
    let df11 = |h: f64| {
        vec![
            rect(c.comp, o, o, o + 2.0, o + h),
            rect(c.nplus, o - 0.2, o - 0.2, o + 1.0, o + h + 0.2),
            rect(c.pplus, o + 1.0, o - 0.2, o + 2.2, o + h + 0.2),
        ]
    };
    write("DF.11", "good", df11(0.5));
    write("DF.11", "bad", df11(0.295));

    // DF.12: an active with no implant at all.
    write(
        "DF.12",
        "good",
        scene(implanted(&c, true, o, o, o + 1.0, o + 1.0), false, near),
    );
    write(
        "DF.12",
        "bad",
        scene(vec![rect(c.comp, o, o, o + 1.0, o + 1.0)], false, None),
    );

    // DF.13: a P source/drain with no N-tie in reach.  The clean half puts one in the
    // same well; the broken half has none anywhere.
    let df13 = |tie: bool, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 6.0, o + 6.0)];
        v.extend(implanted(&c, false, o + 1.0, o + 1.0, o + 3.0, o + 3.0));
        if tie {
            v.extend(implanted(&c, true, o + 4.0, o + 4.0, o + 5.0, o + 5.0));
        }
        scene(v, mv, None)
    };
    for (id, mv) in [("DF.13_LV", false), ("DF.13_MV", true)] {
        write(id, "good", df13(true, mv));
        write(id, "bad", df13(false, mv));
    }

    // DF.14: an N source/drain with no P-tie in reach.
    let df14 = |tie: bool, mv: bool| {
        let mut v = implanted(&c, true, o, o, o + 2.0, o + 2.0);
        if tie {
            v.extend(implanted(&c, false, o + 3.0, o, o + 4.0, o + 1.0));
        }
        scene(v, mv, None)
    };
    for (id, mv) in [("DF.14_LV", false), ("DF.14_MV", true)] {
        write(id, "good", df14(true, mv));
        write(id, "bad", df14(false, mv));
    }

    // DF.16: an N source/drain outside the wells, to an N-well.
    let df16 = |gap: f64, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 3.0, o + 3.0)];
        v.extend(implanted(
            &c,
            true,
            o + 3.0 + gap,
            o + 0.5,
            o + 5.0 + gap,
            o + 2.5,
        ));
        v.extend(ptie(&c, o + 8.0, o));
        scene(v, mv, None)
    };
    pair("DF.16", &df16, (1.5, 1.5), (0.425, 0.595));

    // DF.17: a P-tie outside the wells, to an N-well.
    let df17 = |gap: f64, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 3.0, o + 3.0)];
        v.extend(implanted(
            &c,
            false,
            o + 3.0 + gap,
            o + 0.5,
            o + 5.0 + gap,
            o + 2.5,
        ));
        scene(v, mv, None)
    };
    pair("DF.17", &df17, (1.5, 1.5), (0.115, 0.155));

    // DF.18: a P-tie outside the wells, to a deep well.
    let df18 = |gap: f64| {
        let mut v = vec![rect(c.dnwell, o, o, o + 5.0, o + 5.0)];
        v.extend(implanted(
            &c,
            false,
            o + 5.0 + gap,
            o + 1.5,
            o + 7.0 + gap,
            o + 3.5,
        ));
        scene(v, false, None)
    };
    write("DF.18", "good", df18(3.5));
    write("DF.18", "bad", df18(2.495));

    // DF.19: an N source/drain outside the wells, to a deep well.  Its P-tie goes on the
    // far side, since DF.18 keeps one 2.5 µm off the deep well too.
    let df19 = |gap: f64, mv: bool| {
        let mut v = vec![rect(c.dnwell, o, o, o + 5.0, o + 5.0)];
        v.extend(implanted(
            &c,
            true,
            o + 5.0 + gap,
            o + 1.5,
            o + 7.0 + gap,
            o + 3.5,
        ));
        v.extend(implanted(
            &c,
            false,
            o + 6.0 + gap,
            o + 6.0,
            o + 8.0 + gap,
            o + 8.0,
        ));
        scene(v, mv, None)
    };
    pair("DF.19", &df19, (4.0, 4.0), (3.195, 3.275));

    hardening(pdk);
}

// ---------------------------------------------------------------------------------------
// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.5 alone, one fixture per theme, `<rule>.h<n>`.  The engine's own
// classes - the bound, 45°, unions, notches, tile lines, arrays - are the engine family's;
// what is drawn here is what only this deck has: the markers that pick a rule's layer
// (Dualgate, V5_XTOR, MOS_CAP_MK, DRC_BJT, OTP_MK, RES_MK, YMTP_MK, SRAMCORE, MVSD,
// SCHOTTKY_DIODE), the wells that classify a COMP, the implants that make it N or P, the
// gate that makes it source/drain, the butted tap, and the low/medium voltage split -
// each at the bound and one grid step past it, and where a marker far along a shape or
// a tile line could change the deck's reading.  Each function's comment states the
// geometry and what the manual says; the expected answers are in the `hardening_comp`
// table of tests/gf180mcuD.rs and the reasoning in hardening/reports/gf180mcuD/comp.md.
//
// Nearly every COMP here is a P+ substrate tap: DF.12 wants an implant on every active,
// and a P+ active outside the wells is nobody's source/drain, so it needs no tie for
// DF.13/DF.14 and no other rule of the deck reaches it.  An N+ active outside the wells
// is a source/drain and gets a P+ tap within 20 µm; a P+ one inside an N-well gets an N+
// tap in the same well.
// ---------------------------------------------------------------------------------------

/// Layers the hardening patterns draw on.
struct L {
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly2: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    dualgate: (i16, i16),
    v5_xtor: (i16, i16),
    mos_cap: (i16, i16),
    drc_bjt: (i16, i16),
    otp_mk: (i16, i16),
    res_mk: (i16, i16),
    ymtp_mk: (i16, i16),
    sramcore: (i16, i16),
    mvsd: (i16, i16),
    schottky: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            comp: layer(pdk, "comp"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            poly2: layer(pdk, "poly2_drawn"),
            nwell: layer(pdk, "nwell"),
            dnwell: layer(pdk, "dnwell"),
            lvpwell: layer(pdk, "lvpwell"),
            dualgate: layer(pdk, "dualgate"),
            v5_xtor: layer(pdk, "v5_xtor"),
            mos_cap: layer(pdk, "mos_cap_mk"),
            drc_bjt: layer(pdk, "drc_bjt"),
            otp_mk: layer(pdk, "otp_mk"),
            res_mk: layer(pdk, "res_mk"),
            ymtp_mk: layer(pdk, "ymtp_mk"),
            sramcore: layer(pdk, "sramcore"),
            mvsd: layer(pdk, "mvsd"),
            schottky: layer(pdk, "schottky_diode"),
        }
    }

    /// A P+ active: the substrate tap most patterns are made of.
    fn p(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        self.imp(false, x0, y0, x1, y1)
    }

    /// An N+ active.
    fn n(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        self.imp(true, x0, y0, x1, y1)
    }

    /// An active with its implant 0.2 wider all round.
    fn imp(&self, n: bool, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        let l = if n { self.nplus } else { self.pplus };
        vec![
            rect(self.comp, x0, y0, x1, y1),
            rect(l, x0 - 0.2, y0 - 0.2, x1 + 0.2, y1 + 0.2),
        ]
    }

    /// A butted active `(x0, y0)-(x1, y1)`: N+ up to `xb`, P+ from there on.
    fn butted(&self, x0: f64, y0: f64, x1: f64, y1: f64, xb: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x0, y0, x1, y1),
            rect(self.nplus, x0 - 0.2, y0 - 0.2, xb, y1 + 0.2),
            rect(self.pplus, xb, y0 - 0.2, x1 + 0.2, y1 + 0.2),
        ]
    }

    /// A transistor: N+ active `(x0, y0)-(x1, y1)` with a vertical gate `gx0..gx1`
    /// reaching 0.3 past the active top and bottom.
    fn fet(&self, x0: f64, y0: f64, x1: f64, y1: f64, gx0: f64, gx1: f64) -> Vec<GdsElement> {
        let mut v = self.n(x0, y0, x1, y1);
        v.push(rect(self.poly2, gx0, y0 - 0.3, gx1, y1 + 0.3));
        v
    }

    /// A square ring of P+ active: outer `(x0, y0)-(x1, y1)`, hole `(hx0, hy0)-(hx1, hy1)`,
    /// four overlapping walls that merge into one.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        hx0: f64,
        hy0: f64,
        hx1: f64,
        hy1: f64,
    ) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x0, y0, hx0, y1),
            rect(self.comp, hx1, y0, x1, y1),
            rect(self.comp, x0, y0, x1, hy0),
            rect(self.comp, x0, hy1, x1, y1),
            rect(self.pplus, x0 - 0.2, y0 - 0.2, x1 + 0.2, y1 + 0.2),
        ]
    }
}

fn write_h(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    df1a_h(&l);
    df1c_h(&l);
    df2a_h(&l);
    df2b_h(&l);
    df3a_h(&l);
    df3b_h(&l);
    df3c_h(&l);
    df4a_h(&l);
    df4b_h(&l);
    df4c_h(&l);
    df4d_h(&l);
    df4e_h(&l);
    df5_h(&l);
    df6_h(&l);
    df7_h(&l);
    df8_h(&l);
    df9_h(&l);
    df10_h(&l);
    df11_h(&l);
    df12_h(&l);
    df13_h(&l);
    df14_h(&l);
    df16_h(&l);
    df17_h(&l);
    df18_h(&l);
    df19_h(&l);
}

// --- DF.1a: min. COMP width 0.22 (3.3 V) / 0.3 (5 V/6 V) ---

fn df1a_h(l: &L) {
    // h1 - the bound in both columns and what decides the column.  Bars 2 µm tall: 0.22
    // is legal at low voltage, 0.215 is not; under Dualgate 0.3 is legal, 0.295 and 0.25
    // are not.  A 0.25 bar that Dualgate covers only the bottom half of is a 5 V active
    // (DV.7 forbids the partial cover, the width rule still reads the 5 V column).  A
    // 0.215 bar whose left wall lies on a Dualgate edge, touching without overlap, is
    // not under the marker and reads the 3.3 V column; so does one under V5_XTOR alone,
    // since the manual knows no 5 V area without Dualgate.  A 0.25 bar under Dualgate
    // with MVSD over its top: the LDMOS drift diffusion has rules of its own, the rest
    // of the bar is a 0.25 active at 5 V.
    let mut v = vec![];
    v.extend(l.p(2.0, 2.0, 2.22, 4.0)); // clean
    v.extend(l.p(4.0, 2.0, 4.215, 4.0)); // DF.1a_LV
    v.extend(l.p(6.0, 2.0, 6.25, 4.0)); // clean
    v.push(rect(l.dualgate, 9.0, 1.0, 15.5, 5.0));
    v.extend(l.p(10.0, 2.0, 10.3, 4.0)); // clean
    v.extend(l.p(12.0, 2.0, 12.295, 4.0)); // DF.1a_MV
    v.extend(l.p(14.0, 2.0, 14.25, 4.0)); // DF.1a_MV
    v.push(rect(l.dualgate, 17.0, 1.0, 19.0, 3.0));
    v.extend(l.p(18.0, 2.0, 18.25, 4.0)); // DF.1a_MV: partly covered
    v.push(rect(l.dualgate, 2.0, 7.0, 5.0, 9.0));
    v.extend(l.p(5.0, 7.0, 5.215, 9.0)); // DF.1a_LV: touches the marker's edge
    v.push(rect(l.v5_xtor, 7.0, 7.0, 9.0, 9.0));
    v.extend(l.p(7.5, 7.2, 7.715, 8.8)); // DF.1a_LV: V5_XTOR without Dualgate
    v.push(rect(l.dualgate, 11.0, 6.5, 15.0, 9.5));
    v.extend(l.p(12.0, 7.0, 12.25, 9.0)); // DF.1a_MV: the part MVSD leaves
    v.push(rect(l.mvsd, 11.8, 8.0, 12.5, 9.3));
    write_h("DF.1a.h1", v);

    // h2 - the marker far along the shape.  A 0.25 bar 300 µm long with Dualgate over
    // its last 12 µm is a 5 V active from end to end (the manual's column is the
    // device's, DV.7 aside), so it is 0.05 too narrow along every tile it crosses; the
    // same bar without the marker is legal; a 0.215 bar of that length fails at 3.3 V.
    let mut v = vec![];
    v.extend(l.p(2.0, 12.0, 302.0, 12.25)); // DF.1a_MV, the marker at the far end
    v.push(rect(l.dualgate, 290.0, 11.0, 303.0, 13.0));
    v.extend(l.p(2.0, 14.0, 302.0, 14.25)); // clean
    v.extend(l.p(2.0, 16.0, 302.0, 16.215)); // DF.1a_LV
    write_h("DF.1a.h2", v);
}

// --- DF.1c: min. COMP width for MOSCAP 1.0 ---

fn df1c_h(l: &L) {
    // h1 - actives under MOS_CAP_MK: 1.0 wide is legal, 0.995 is not.  A 2.0 wide
    // active the marker covers only 0.995 of reads as a 0.995 capacitor plate (the
    // marker says what is capacitor).  A 0.5 active outside the marker is a plain active.
    let mut v = vec![];
    v.extend(l.p(2.0, 2.0, 3.0, 4.0));
    v.push(rect(l.mos_cap, 1.5, 1.5, 3.5, 4.5)); // clean
    v.extend(l.p(5.0, 2.0, 5.995, 4.0));
    v.push(rect(l.mos_cap, 4.5, 1.5, 6.5, 4.5)); // DF.1c
    v.extend(l.p(8.0, 2.0, 10.0, 4.0));
    v.push(rect(l.mos_cap, 7.5, 1.5, 8.995, 4.5)); // DF.1c: the marker cuts the active
    v.extend(l.p(12.0, 2.0, 12.5, 4.0)); // clean: no marker
    write_h("DF.1c.h1", v);
}

// --- DF.2a: min. channel width 0.22 (3.3 V) / 0.3 (5 V/6 V) ---

fn df2a_h(l: &L) {
    // h1 - the bound, read on the gate's edge inside the active.  A transistor whose
    // active is 0.22 wide has a 0.22 channel.  A poly island inside a wide active makes a
    // channel edge without narrowing the active: 0.215 tall fails, 0.22 does not; under
    // Dualgate 0.295 and 0.25 fail, 0.3 does not.  The P+ tap at the bottom answers
    // DF.14 for the N+ actives.
    let mut v = vec![];
    v.extend(l.fet(2.0, 2.0, 4.0, 2.22, 2.9, 3.2)); // clean
    v.extend(l.n(2.0, 6.0, 4.0, 7.2));
    v.push(rect(l.poly2, 2.5, 6.5, 3.5, 6.715)); // DF.2a_LV (two edges)
    v.extend(l.n(2.0, 8.0, 4.0, 9.2));
    v.push(rect(l.poly2, 2.5, 8.5, 3.5, 8.72)); // clean
    v.push(rect(l.dualgate, 6.0, 1.0, 10.0, 10.0));
    v.extend(l.n(6.5, 2.0, 8.5, 3.3));
    v.push(rect(l.poly2, 7.0, 2.5, 8.0, 2.795)); // DF.2a_MV
    v.extend(l.n(6.5, 5.0, 8.5, 6.3));
    v.push(rect(l.poly2, 7.0, 5.5, 8.0, 5.8)); // clean
    v.extend(l.n(6.5, 7.5, 8.5, 8.8));
    v.push(rect(l.poly2, 7.0, 8.0, 8.0, 8.25)); // DF.2a_MV: legal at 3.3 V only
    v.extend(l.p(2.0, 11.0, 4.0, 12.0));
    write_h("DF.2a.h1", v);

    // h2 - the deck's derivation.  An active drawn as two 0.11 boxes stacked is one
    // 0.22 active under its gate (clean).  A 0.5 gate across a 2 µm active with a 0.1
    // deep, 0.4 wide notch in its side: the channel is 2 µm wide, the notch shortens
    // the gate, not the channel (clean).  Islands across the tile line at x = 20: a
    // 0.6 x 0.5 island straddling it is clean however the line cuts its edges; a
    // 0.2 x 0.5 island straddling it fails on its two 0.2 edges; a 0.5 x 0.215 island
    // with an edge on the line fails on its two 0.215 edges.
    let mut v = vec![rect(l.comp, 12.0, 2.0, 14.0, 2.11)];
    v.push(rect(l.comp, 12.0, 2.11, 14.0, 2.22));
    v.push(rect(l.nplus, 11.8, 1.8, 14.2, 2.42));
    v.push(rect(l.poly2, 12.9, 1.7, 13.2, 2.72)); // clean: the union is 0.22
    v.extend(l.n(16.0, 2.0, 18.0, 4.0));
    v.push(poly(
        l.poly2,
        &[
            (16.9, 1.7),
            (17.4, 1.7),
            (17.4, 2.8),
            (17.3, 2.8),
            (17.3, 3.2),
            (17.4, 3.2),
            (17.4, 4.3),
            (16.9, 4.3),
        ],
    )); // clean: a notch in the gate's side
    v.extend(l.n(19.0, 5.5, 21.5, 11.0));
    v.push(rect(l.poly2, 19.9, 6.5, 20.5, 7.0)); // clean, straddling x = 20
    v.push(rect(l.poly2, 19.9, 8.0, 20.1, 8.5)); // DF.2a_LV x2, straddling x = 20
    v.push(rect(l.poly2, 20.0, 10.0, 20.5, 10.215)); // DF.2a_LV x2, an edge on x = 20
    v.extend(l.p(23.0, 6.0, 25.0, 7.0));
    write_h("DF.2a.h2", v);
}

// --- DF.2b: max. COMP width 100 except under MOS_CAP_MK ---

fn df2b_h(l: &L) {
    // h1 - 100 x 100 is legal, 100.005 square is not.  Two 120 plates joined by a 30
    // wide bar are two places 100 wide.  Under MOS_CAP_MK a 150 square is a capacitor;
    // with the marker over 75 of its 150 the rest is 75 wide (clean), over 40 of it the
    // rest is 110 (fails).  A diamond 120 between its flats is 120 wide (fails); one 99
    // between its flats is not.
    let mut v = vec![];
    v.extend(l.p(2.0, 2.0, 102.0, 102.0)); // clean
    v.extend(l.p(110.0, 2.0, 210.005, 102.005)); // DF.2b
    v.extend(l.p(220.0, 2.0, 340.0, 122.0)); // DF.2b
    v.extend(l.p(360.0, 2.0, 480.0, 122.0)); // DF.2b
    v.extend(l.p(340.0, 50.0, 360.0, 80.0));
    v.extend(l.p(2.0, 130.0, 152.0, 280.0));
    v.push(rect(l.mos_cap, 1.0, 129.0, 153.0, 281.0)); // clean
    v.extend(l.p(160.0, 130.0, 310.0, 280.0));
    v.push(rect(l.mos_cap, 159.0, 129.0, 235.0, 281.0)); // clean: 75 left
    v.extend(l.p(320.0, 130.0, 470.0, 280.0));
    v.push(rect(l.mos_cap, 319.0, 129.0, 360.0, 281.0)); // DF.2b: 110 left
    v.push(diamond(l.comp, 80.0, 400.0, 70.0));
    v.push(rect(l.pplus, 9.0, 329.0, 151.0, 471.0)); // clean: 99 between flats
    v.push(diamond(l.comp, 300.0, 420.0, 84.85));
    v.push(rect(l.pplus, 214.0, 334.0, 386.0, 506.0)); // DF.2b: 120 between flats
    write_h("DF.2b.h1", v);
}

// --- DF.3a: min. COMP space 0.28 (3.3 V) / 0.36 (5 V/6 V) ---

fn df3a_h(l: &L) {
    // h1 - the bound in both columns, the notch, the mixed pair, the butted tap and the
    // OTP marker.  0.28 is legal at 3.3 V, 0.275 is not, nor is a 0.275 notch; under
    // Dualgate 0.36 is legal, 0.355 and 0.30 are not.  A 3.3 V active 0.275 from a 5 V
    // one is a pair the manual bounds at 0.28 at least (DV.3 keeps the two 0.48 apart
    // in a legal layout, the space rule still reads 0.275).  An N+ active butted to a
    // P+ tap is one active, and no space; a P+ tap 0.275 from an N+ active is a space.
    // Two actives 0.275 apart under OTP_MK are the OTP cell's business; with only one
    // of them under the marker, the pair is still that (upstream's reading).
    let mut v = vec![];
    v.extend(l.p(2.0, 2.0, 3.0, 3.0));
    v.extend(l.p(3.28, 2.0, 4.28, 3.0)); // clean
    v.extend(l.p(2.0, 4.0, 3.0, 5.0));
    v.extend(l.p(3.275, 4.0, 4.275, 5.0)); // DF.3a_LV
    v.push(poly(
        l.comp,
        &[
            (6.0, 2.0),
            (8.0, 2.0),
            (8.0, 3.0),
            (7.275, 3.0),
            (7.275, 2.5),
            (7.0, 2.5),
            (7.0, 3.0),
            (6.0, 3.0),
        ],
    ));
    v.push(rect(l.pplus, 5.8, 1.8, 8.2, 3.2)); // DF.3a_LV: a 0.275 notch
    v.push(rect(l.dualgate, 10.0, 1.0, 16.5, 6.0));
    v.extend(l.p(10.5, 2.0, 11.5, 3.0));
    v.extend(l.p(11.86, 2.0, 12.86, 3.0)); // clean
    v.extend(l.p(10.5, 4.0, 11.5, 5.0));
    v.extend(l.p(11.855, 4.0, 12.855, 5.0)); // DF.3a_MV
    v.extend(l.p(13.5, 2.0, 14.5, 3.0));
    v.extend(l.p(14.8, 2.0, 15.8, 3.0)); // DF.3a_MV: 0.30, legal at 3.3 V only
    v.extend(l.p(2.0, 7.0, 3.0, 8.0));
    v.push(rect(l.dualgate, 3.275, 6.5, 6.0, 9.0));
    v.extend(l.p(3.275, 7.0, 4.275, 8.0)); // DF.3a_LV: 3.3 V to 5 V at 0.275
    v.extend(l.butted(6.5, 7.0, 8.5, 8.0, 7.5)); // clean: butted
    v.extend(l.n(10.0, 10.0, 11.0, 11.0));
    v.extend(l.p(11.275, 10.0, 12.275, 11.0)); // DF.3a_LV: tap to N+ active
    v.extend(l.p(2.0, 10.0, 3.0, 11.0));
    v.extend(l.p(3.275, 10.0, 4.275, 11.0));
    v.push(rect(l.otp_mk, 1.5, 9.5, 5.0, 11.5)); // clean: both under OTP_MK
    v.extend(l.p(6.0, 10.0, 7.0, 11.0));
    v.push(rect(l.otp_mk, 5.5, 9.5, 7.2, 11.5));
    v.extend(l.p(7.275, 10.0, 8.275, 11.0)); // clean: one under OTP_MK
    write_h("DF.3a.h1", v);

    // h2 - the marker far along the shapes.  Two 300 µm bars 0.30 apart with Dualgate
    // over their last 12 µm are 5 V actives from end to end, 0.06 too close all along;
    // the same two without the marker are legal at 3.3 V; two 300 µm bars 0.275 apart
    // fail at 3.3 V.
    let mut v = vec![];
    v.extend(l.p(2.0, 12.0, 302.0, 12.5));
    v.extend(l.p(2.0, 12.8, 302.0, 13.3)); // DF.3a_MV
    v.push(rect(l.dualgate, 290.0, 11.0, 303.0, 14.0));
    v.extend(l.p(2.0, 16.0, 302.0, 16.5));
    v.extend(l.p(2.0, 16.8, 302.0, 17.3)); // clean
    v.extend(l.p(2.0, 20.0, 302.0, 20.5));
    v.extend(l.p(2.0, 20.775, 302.0, 21.275)); // DF.3a_LV
    write_h("DF.3a.h2", v);
}

// --- DF.3b: N+/P+ butting in one well is exact; no MOSCAP butting ---

fn df3b_h(l: &L) {
    // h1 - butted actives in an N-well: the implants meet exactly (clean), overlap by
    // 0.005 (DF.3b: the N+ tap over the P+ source/drain), or leave 0.005 uncovered
    // (DF.12, no DF.3b).  Outside the wells a P+ tap overlapping an N+ source/drain by
    // 0.01 across x = 20 is the same fault in the substrate.  A butted active under
    // MOS_CAP_MK is a butted capacitor (DF.3b); with the marker over its P+ half only
    // it is still one, the manual forbids MOSCAP butting whichever half is the
    // capacitor; over its N+ half only, likewise.  In a deep well without a P-well the
    // N+ tap over the P+ source/drain is DF.3b too.
    let mut v = vec![
        rect(l.nwell, 2.0, 2.0, 8.0, 8.0),
        rect(l.comp, 3.0, 3.0, 7.0, 5.0),
    ];
    v.push(rect(l.nplus, 2.8, 2.8, 5.005, 5.2));
    v.push(rect(l.pplus, 5.0, 2.8, 7.2, 5.2)); // DF.3b: 0.005 overlap
    v.push(rect(l.nwell, 10.0, 2.0, 16.0, 8.0));
    v.extend(l.butted(11.0, 3.0, 15.0, 5.0, 13.0)); // clean: exact butt
    v.push(rect(l.nwell, 2.0, 10.0, 8.0, 16.0));
    v.push(rect(l.comp, 3.0, 11.0, 7.0, 13.0));
    v.push(rect(l.nplus, 2.8, 10.8, 5.0, 13.2));
    v.push(rect(l.pplus, 5.005, 10.8, 7.2, 13.2)); // DF.12: a 0.005 gap
    v.push(rect(l.comp, 18.0, 14.0, 22.0, 16.0));
    v.push(rect(l.nplus, 17.8, 13.8, 20.005, 16.2));
    v.push(rect(l.pplus, 19.995, 13.8, 22.2, 16.2)); // DF.3b: the tap over the S/D
    v.extend(l.butted(18.0, 2.0, 22.0, 4.0, 20.0));
    v.push(rect(l.mos_cap, 17.5, 1.5, 22.5, 4.5)); // DF.3b: butted MOSCAP
    v.extend(l.butted(18.0, 6.0, 22.0, 8.0, 20.0));
    v.push(rect(l.mos_cap, 20.5, 5.5, 22.5, 8.5)); // DF.3b: the P+ half is the MOSCAP
    v.extend(l.butted(18.0, 10.0, 22.0, 12.0, 20.0));
    v.push(rect(l.mos_cap, 17.5, 9.5, 19.5, 12.5)); // DF.3b: the N+ half is the MOSCAP
    v.push(rect(l.dnwell, 26.0, 2.0, 34.0, 8.0));
    v.push(rect(l.comp, 28.0, 4.0, 32.0, 6.0));
    v.push(rect(l.nplus, 27.8, 3.8, 30.1, 6.2));
    v.push(rect(l.pplus, 30.0, 3.8, 32.2, 6.2)); // DF.3b: in the deep well
    write_h("DF.3b.h1", v);
}

// --- DF.3c: min. COMP space in a BJT area 0.32 (3.3 V), none at 5 V/6 V ---

fn df3c_h(l: &L) {
    // h1 - under DRC_BJT two actives 0.32 apart are legal, 0.315 are not, nor a 0.315
    // notch.  An active in the marker 0.315 from one outside it: the outside one is not
    // in the BJT area (clean; DF.3a's 0.28 is met).  Under Dualgate a marker with two
    // actives is forbidden whatever their space, one with a single active is not, and
    // one whose edge an outside active touches still holds a single active.
    let mut v = vec![];
    v.push(rect(l.drc_bjt, 1.5, 1.5, 6.0, 4.0));
    v.extend(l.p(2.0, 2.0, 3.0, 3.0));
    v.extend(l.p(3.32, 2.0, 4.32, 3.0)); // clean
    v.push(rect(l.drc_bjt, 1.5, 5.5, 6.0, 8.0));
    v.extend(l.p(2.0, 6.0, 3.0, 7.0));
    v.extend(l.p(3.315, 6.0, 4.315, 7.0)); // DF.3c_LV
    v.push(rect(l.drc_bjt, 7.5, 1.5, 12.0, 4.0));
    v.push(poly(
        l.comp,
        &[
            (8.0, 2.0),
            (10.0, 2.0),
            (10.0, 3.0),
            (9.315, 3.0),
            (9.315, 2.5),
            (9.0, 2.5),
            (9.0, 3.0),
            (8.0, 3.0),
        ],
    ));
    v.push(rect(l.pplus, 7.8, 1.8, 10.2, 3.2)); // DF.3c_LV: a 0.315 notch
    v.push(rect(l.drc_bjt, 7.5, 5.5, 9.5, 8.0));
    v.extend(l.p(8.0, 6.0, 9.0, 7.0));
    v.extend(l.p(9.815, 6.0, 10.815, 7.0)); // clean: the second is outside the area
    v.push(rect(l.dualgate, 1.0, 9.0, 12.0, 16.5));
    v.push(rect(l.drc_bjt, 1.5, 9.5, 6.0, 12.0));
    v.extend(l.p(2.0, 10.0, 3.0, 11.0));
    v.extend(l.p(4.0, 10.0, 5.0, 11.0)); // DF.3c_MV x2
    v.push(rect(l.drc_bjt, 7.0, 9.5, 11.0, 12.0));
    v.extend(l.p(8.0, 10.0, 9.0, 11.0)); // clean: one active
    v.push(rect(l.drc_bjt, 1.5, 13.0, 4.0, 15.5));
    v.extend(l.p(2.0, 13.5, 3.0, 14.5));
    v.extend(l.p(4.0, 13.5, 5.0, 14.5)); // clean: touches the marker from outside
    write_h("DF.3c.h1", v);
}

// --- DF.4a: min. LVPWELL space to N+ well tap inside DNWELL 0.12 / 0.16 ---

fn df4a_h(l: &L) {
    // h1 - an N+ tap in a deep well 0.12 from the P-well is legal, 0.115 is not; with
    // Dualgate over the deep well 0.16 is legal, 0.155 is not.  A deep well touched by
    // Dualgate at one corner only is a 5 V deep well (DV.1 wants the whole of it
    // covered; the column is the well's), so a tap 0.13 from its P-well fails at 5 V.
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, gap: f64| {
        v.push(rect(l.dnwell, x, y, x + 10.0, y + 8.0));
        v.push(rect(l.lvpwell, x + 1.0, y + 1.0, x + 4.0, y + 7.0));
        v.extend(l.n(x + 4.0 + gap, y + 3.0, x + 6.0 + gap, y + 5.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.12); // clean
    dn(&mut v, 14.0, 2.0, 0.115); // DF.4a_LV
    v.push(rect(l.dualgate, 1.5, 11.5, 24.5, 20.5));
    dn(&mut v, 2.0, 12.0, 0.16); // clean
    dn(&mut v, 14.0, 12.0, 0.155); // DF.4a_MV
    dn(&mut v, 26.0, 2.0, 0.13); // DF.4a_MV: Dualgate on the well's corner only
    v.push(rect(l.dualgate, 35.0, 9.0, 37.0, 11.0));
    write_h("DF.4a.h1", v);

    // h2 - the marker far along the well.  A 300 µm deep well with Dualgate over its
    // last 12 µm is a 5 V deep well; its tap 0.13 from the P-well at x = 6 fails at 5 V
    // whatever tile the marker lies in.
    let mut v = vec![];
    v.push(rect(l.dnwell, 2.0, 2.0, 302.0, 8.0));
    v.push(rect(l.dualgate, 290.0, 1.0, 303.0, 9.0));
    v.push(rect(l.lvpwell, 3.0, 3.0, 6.0, 7.0));
    v.extend(l.n(6.13, 4.0, 8.13, 6.0)); // DF.4a_MV
    write_h("DF.4a.h2", v);
}

// --- DF.4b: min. DNWELL overlap of N+ well tap 0.62 / 0.66 ---

fn df4b_h(l: &L) {
    // h1 - a deep well holding an N+ tap by 0.62 is legal, by 0.615 is not; under
    // Dualgate 0.66 is, 0.655 is not.  A deep well whose corner is cut off at 45° so
    // the cut passes 0.17 from the tap's corner holds it by 0.17 there.
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.dnwell, x, y, x + 6.0, y + 6.0));
        v.extend(l.n(x + enc, y + 2.0, x + 4.0, y + 4.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.62); // clean
    dn(&mut v, 10.0, 2.0, 0.615); // DF.4b_LV
    v.push(rect(l.dualgate, 1.5, 9.5, 16.5, 16.5));
    dn(&mut v, 2.0, 10.0, 0.66); // clean
    dn(&mut v, 10.0, 10.0, 0.655); // DF.4b_MV
    v.push(poly(
        l.dnwell,
        &[
            (19.0, 2.0),
            (24.0, 2.0),
            (24.0, 8.0),
            (18.0, 8.0),
            (18.0, 3.0),
        ],
    ));
    v.extend(l.n(18.62, 2.62, 20.62, 4.62)); // DF.4b_LV: 0.17 to the chamfer
    write_h("DF.4b.h1", v);
}

// --- DF.4c: min. NWELL overlap of P+ active outside DNWELL 0.43 / 0.6 ---

fn df4c_h(l: &L) {
    // h1 - an N-well holding a P+ source/drain by 0.43 is legal, by 0.425 is not; under
    // Dualgate 0.6 is, 0.595 is not.  Each well has its N+ tap for DF.13.  A well under
    // SRAMCORE is the SRAM cell's (0.3 is clean).  A well with Dualgate over its far
    // corner only is a 5 V well (DV.9 allows one voltage per well), so 0.5 fails.
    let nw = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.nwell, x, y, x + 7.0, y + 6.0));
        v.extend(l.n(x + 5.0, y + 4.0, x + 6.0, y + 5.0));
        v.extend(l.p(x + enc, y + 1.0, x + 2.0 + enc, y + 3.0));
    };
    let mut v = vec![];
    nw(&mut v, 2.0, 2.0, 0.43); // clean
    nw(&mut v, 11.0, 2.0, 0.425); // DF.4c_LV
    v.push(rect(l.dualgate, 1.5, 9.5, 18.5, 16.5));
    nw(&mut v, 2.0, 10.0, 0.6); // clean
    nw(&mut v, 11.0, 10.0, 0.595); // DF.4c_MV
    nw(&mut v, 20.0, 2.0, 0.3);
    v.push(rect(l.sramcore, 19.5, 1.5, 27.5, 8.5)); // clean: SRAM
    nw(&mut v, 20.0, 10.0, 0.5); // DF.4c_MV: Dualgate on the well's corner only
    v.push(rect(l.dualgate, 26.0, 15.0, 28.0, 17.0));
    write_h("DF.4c.h1", v);
}

// --- DF.4d: min. NWELL overlap of N+ active outside DNWELL 0.12 / 0.16 ---

fn df4d_h(l: &L) {
    // h1 - an N-well holding an N+ tap by 0.12 is legal, by 0.115 is not; under
    // Dualgate 0.16 is, 0.155 is not.  A 3.3 V tap (no Dualgate on it) held by 0.14 in a
    // well Dualgate touches elsewhere: the tap is a 3.3 V active (clean).  A well under
    // YMTP_MK is the MTP cell's (0.1 is clean).
    let nw = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.nwell, x, y, x + 6.0, y + 6.0));
        v.extend(l.n(x + enc, y + 2.0, x + 2.0 + enc, y + 4.0));
    };
    let mut v = vec![];
    nw(&mut v, 2.0, 2.0, 0.12); // clean
    nw(&mut v, 10.0, 2.0, 0.115); // DF.4d_LV
    v.push(rect(l.dualgate, 17.5, 1.5, 32.5, 8.5));
    nw(&mut v, 18.0, 2.0, 0.16); // clean
    nw(&mut v, 26.0, 2.0, 0.155); // DF.4d_MV
    nw(&mut v, 2.0, 10.0, 0.14); // clean: the tap is 3.3 V
    v.push(rect(l.dualgate, 7.0, 15.0, 9.0, 17.0));
    nw(&mut v, 10.0, 10.0, 0.1);
    v.push(rect(l.ymtp_mk, 9.5, 9.5, 16.5, 16.5)); // clean: MTP
    write_h("DF.4d.h1", v);
}

// --- DF.4e: min. DNWELL overlap of P+ active 0.93 / 1.1 ---

fn df4e_h(l: &L) {
    // h1 - a deep well holding a P+ source/drain by 0.93 is legal, by 0.925 is not;
    // under Dualgate 1.1 is, 1.095 is not.  The source/drain sits in a drawn N-well with
    // its N+ tap, as the existing fixtures do.  A P+ tap in the P-well of a deep well is
    // a P+ active in the deep well too: held by 0.925 it fails.
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.dnwell, x, y, x + 8.0, y + 8.0));
        v.push(rect(l.nwell, x + 0.3, y + 0.3, x + 7.7, y + 7.7));
        v.extend(l.n(x + 5.0, y + 5.0, x + 6.0, y + 6.0));
        v.extend(l.p(x + enc, y + 2.0, x + 2.0 + enc, y + 4.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.93); // clean
    dn(&mut v, 12.0, 2.0, 0.925); // DF.4e_LV
    v.push(rect(l.dualgate, 1.5, 11.5, 20.5, 20.5));
    dn(&mut v, 2.0, 12.0, 1.1); // clean
    dn(&mut v, 12.0, 12.0, 1.095); // DF.4e_MV
    v.push(rect(l.dnwell, 22.0, 2.0, 30.0, 10.0));
    v.push(rect(l.lvpwell, 22.5, 2.5, 27.0, 9.5));
    v.extend(l.p(22.925, 4.0, 24.925, 6.0)); // DF.4e_LV: the P-well's tap
    write_h("DF.4e.h1", v);
}

// --- DF.5: min. LVPWELL overlap of P+ well tap inside DNWELL 0.12 / 0.16 ---

fn df5_h(l: &L) {
    // h1 - a P-well in a deep well holding its P+ tap by 0.12 is legal, by 0.115 is
    // not; under Dualgate 0.16 is, 0.155 is not.  A P-well outside any deep well holding
    // its tap by 0.1 is not this rule's ("inside DNWELL").
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.dnwell, x, y, x + 10.0, y + 10.0));
        v.push(rect(l.lvpwell, x + 2.0, y + 2.0, x + 8.0, y + 8.0));
        v.extend(l.p(x + 2.0 + enc, y + 4.0, x + 4.0 + enc, y + 6.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.12); // clean
    dn(&mut v, 14.0, 2.0, 0.115); // DF.5_LV
    v.push(rect(l.dualgate, 1.5, 13.5, 24.5, 24.5));
    dn(&mut v, 2.0, 14.0, 0.16); // clean
    dn(&mut v, 14.0, 14.0, 0.155); // DF.5_MV
    v.push(rect(l.lvpwell, 28.0, 2.0, 34.0, 8.0));
    v.extend(l.p(28.1, 4.0, 30.1, 6.0)); // clean: no deep well
    write_h("DF.5.h1", v);
}

// --- DF.6: min. COMP extension beyond gate 0.24 / 0.4 ---

fn df6_h(l: &L) {
    // h1 - a source/drain 0.24 beyond the gate is legal, 0.235 is not; under Dualgate
    // 0.4 is, 0.395 and 0.3 are not.  A 0.235 overhang under MVSD is the LDMOS's
    // business (clean).  A poly across an active under RES_MK is a resistor, not a
    // gate: 0.2 beyond it is no overhang (clean).  The P+ tap answers DF.14.
    let mut v = vec![];
    v.extend(l.fet(2.0, 2.0, 3.44, 3.0, 2.9, 3.2)); // clean
    v.extend(l.fet(5.0, 2.0, 6.435, 3.0, 5.9, 6.2)); // DF.6_LV
    v.push(rect(l.dualgate, 8.0, 1.0, 14.0, 7.0));
    v.extend(l.fet(9.0, 2.0, 10.6, 3.0, 9.9, 10.2)); // clean
    v.extend(l.fet(11.0, 2.0, 12.595, 3.0, 11.9, 12.2)); // DF.6_MV
    v.extend(l.fet(9.0, 4.0, 10.5, 5.0, 9.9, 10.2)); // DF.6_MV: 0.3, legal at 3.3 V only
    v.extend(l.fet(2.0, 5.0, 3.435, 6.0, 2.9, 3.2));
    v.push(rect(l.mvsd, 1.8, 4.8, 3.6, 6.2)); // clean: MVSD
    v.extend(l.fet(5.0, 5.0, 6.4, 6.0, 5.9, 6.2));
    v.push(rect(l.res_mk, 5.8, 4.6, 6.3, 6.4)); // clean: a resistor, no gate
    v.extend(l.p(2.0, 8.0, 4.0, 9.0));
    write_h("DF.6.h1", v);

    // h2 - the gate over the active's end, the shared source/drain, the shared gate,
    // and the tile line.  A gate whose poly runs out over the active's right end leaves
    // no source/drain there: an overhang of 0 (fails).  Two gates 1.4 apart share the
    // active between them, which is beyond neither (clean).  One poly across two
    // actives: the upper overhangs it by 0.235 (fails), the lower by 0.24.  On the tile
    // line: an overhang of 0.24 ending at x = 21 is clean; a gate edge on x = 20 with a
    // 0.235 overhang fails; a 0.235 overhang straddling x = 20 fails; the same at
    // x = 41 and x = 42.
    let mut v = vec![];
    v.extend(l.fet(2.0, 2.0, 3.2, 3.0, 2.9, 3.5)); // DF.6_LV: no S/D on the right
    v.extend(l.n(5.0, 2.0, 8.0, 3.0));
    v.push(rect(l.poly2, 5.5, 1.7, 5.8, 3.3));
    v.push(rect(l.poly2, 7.2, 1.7, 7.5, 3.3)); // clean: two fingers
    v.extend(l.n(10.0, 2.0, 11.04, 3.0)); // clean
    v.extend(l.n(10.0, 4.0, 11.035, 5.0)); // DF.6_LV
    v.push(rect(l.poly2, 10.5, 1.7, 10.8, 5.3));
    v.extend(l.fet(19.5, 2.0, 21.0, 3.0, 20.46, 20.76)); // clean: ends on x = 21
    v.extend(l.fet(19.0, 4.0, 20.235, 5.0, 19.7, 20.0)); // DF.6_LV: gate edge on x = 20
    v.extend(l.fet(19.0, 6.0, 20.2, 7.0, 19.665, 19.965)); // DF.6_LV: straddles x = 20
    v.extend(l.fet(39.5, 2.0, 41.0, 3.0, 40.46, 40.76)); // clean: ends on x = 41
    v.extend(l.fet(41.0, 4.0, 42.235, 5.0, 41.7, 42.0)); // DF.6_LV: gate edge on x = 42
    v.extend(l.p(12.0, 8.0, 14.0, 9.0));
    v.extend(l.p(30.0, 8.0, 32.0, 9.0));
    write_h("DF.6.h2", v);
}

// --- DF.7: min. LVPWELL space to P+ active inside DNWELL 0.43 / 0.6 ---

fn df7_h(l: &L) {
    // h1 - a P+ source/drain in the deep well 0.43 from the P-well is legal, 0.425 is
    // not; under Dualgate 0.6 is, 0.595 is not.  As the existing fixture: a drawn N-well
    // under the source/drain and its N+ tap, reaching under the P-well's edge.  A P+ tap
    // inside the P-well, 0.5 from its edge, is the well's own tap and no space (clean).
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, gap: f64| {
        v.push(rect(l.dnwell, x, y, x + 12.0, y + 8.0));
        v.push(rect(l.lvpwell, x + 1.0, y + 1.0, x + 4.0, y + 7.0));
        v.push(rect(l.nwell, x + 3.9, y + 1.0, x + 11.8, y + 7.0));
        v.extend(l.p(x + 4.0 + gap, y + 3.0, x + 6.0 + gap, y + 5.0));
        v.extend(l.n(x + 8.0, y + 3.0, x + 10.0, y + 5.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.43); // clean
    v.extend(l.p(3.5, 4.0, 4.5, 5.0)); // clean: the P-well's own tap
    dn(&mut v, 16.0, 2.0, 0.425); // DF.7_LV
    v.push(rect(l.dualgate, 1.5, 11.5, 28.5, 20.5));
    dn(&mut v, 2.0, 12.0, 0.6); // clean
    dn(&mut v, 16.0, 12.0, 0.595); // DF.7_MV
    write_h("DF.7.h1", v);
}

// --- DF.8: min. LVPWELL overlap of N+ active inside DNWELL 0.43 / 0.6 ---

fn df8_h(l: &L) {
    // h1 - a P-well in a deep well holding an N+ source/drain by 0.43 is legal, by
    // 0.425 is not; under Dualgate 0.6 is, 0.595 is not.  Each P-well has its P+ tap.
    // Under SRAMCORE the 5 V column is the SRAM cell's (0.3 is clean); the manual makes
    // no such exception at 3.3 V (0.3 fails).
    let dn = |v: &mut Vec<GdsElement>, x: f64, y: f64, enc: f64| {
        v.push(rect(l.dnwell, x, y, x + 12.0, y + 12.0));
        v.push(rect(l.lvpwell, x + 1.0, y + 1.0, x + 9.0, y + 9.0));
        v.extend(l.n(x + 1.0 + enc, y + 4.0, x + 4.0 + enc, y + 6.0));
        v.extend(l.p(x + 5.5, y + 4.0, x + 7.5, y + 6.0));
    };
    let mut v = vec![];
    dn(&mut v, 2.0, 2.0, 0.43); // clean
    dn(&mut v, 16.0, 2.0, 0.425); // DF.8_LV
    v.push(rect(l.dualgate, 1.5, 15.5, 42.5, 28.5));
    dn(&mut v, 2.0, 16.0, 0.6); // clean
    dn(&mut v, 16.0, 16.0, 0.595); // DF.8_MV
    dn(&mut v, 30.0, 16.0, 0.3);
    v.push(rect(l.sramcore, 29.5, 15.5, 42.5, 28.5)); // clean: SRAM at 5 V
    dn(&mut v, 30.0, 2.0, 0.3);
    v.push(rect(l.sramcore, 29.5, 1.5, 42.5, 14.5)); // DF.8_LV: SRAM at 3.3 V
    write_h("DF.8.h1", v);
}

// --- DF.9: min. COMP area 0.2025 ---

fn df9_h(l: &L) {
    // h1 - 0.45 x 0.45 is 0.2025 and legal, 0.45 x 0.445 is not.  Two boxes that make
    // 0.45 x 0.45 together are one active (clean); an L of 0.27 is clean.  A 0.16 active
    // under OTP_MK is the OTP cell's.  0.45 x 0.445 straddling x = 20 fails; 0.45 x 0.45
    // straddling x = 40 does not.
    let mut v = vec![];
    v.extend(l.p(2.0, 2.0, 2.45, 2.45)); // clean
    v.extend(l.p(4.0, 2.0, 4.45, 2.445)); // DF.9
    v.push(rect(l.comp, 6.0, 2.0, 6.3, 2.45));
    v.push(rect(l.comp, 6.3, 2.0, 6.45, 2.45));
    v.push(rect(l.pplus, 5.8, 1.8, 6.65, 2.65)); // clean: the union
    v.push(poly(
        l.comp,
        &[
            (8.0, 2.0),
            (8.6, 2.0),
            (8.6, 2.3),
            (8.3, 2.3),
            (8.3, 2.6),
            (8.0, 2.6),
        ],
    ));
    v.push(rect(l.pplus, 7.8, 1.8, 8.8, 2.8)); // clean: an L of 0.27
    v.extend(l.p(10.0, 2.0, 10.4, 2.4));
    v.push(rect(l.otp_mk, 9.8, 1.8, 10.6, 2.6)); // clean: OTP
    v.extend(l.p(19.8, 2.0, 20.25, 2.445)); // DF.9, straddling x = 20
    v.extend(l.p(39.8, 2.0, 40.25, 2.45)); // clean, straddling x = 40
    write_h("DF.9.h1", v);
}

// --- DF.10: min. field area 0.26 ---

fn df10_h(l: &L) {
    // h1 - a hole of 0.51 x 0.51 (0.2601) is legal, 0.5 x 0.5 is not, drawn as four
    // walls or as one keyhole polygon alike; 0.505 x 0.515 (0.260075) is legal.  A 0.5
    // wide notch open to the outside is no hole (clean).  A 0.7 hole with a 0.5 island
    // in it leaves 0.24 of field (fails; the 0.1 gaps to the island are DF.3a's).  A 0.5
    // hole straddling x = 20 fails, one with a wall on x = 20 fails, a 0.51 hole
    // straddling x = 40 does not.
    let mut v = vec![];
    v.extend(l.ring(2.0, 2.0, 4.0, 4.0, 2.745, 2.745, 3.255, 3.255)); // clean
    v.extend(l.ring(6.0, 2.0, 8.0, 4.0, 6.75, 2.75, 7.25, 3.25)); // DF.10
    v.push(poly(
        l.comp,
        &[
            (10.0, 2.0),
            (12.0, 2.0),
            (12.0, 4.0),
            (10.0, 4.0),
            (10.0, 2.75),
            (10.75, 2.75),
            (10.75, 3.25),
            (11.25, 3.25),
            (11.25, 2.75),
            (10.75, 2.75),
            (10.0, 2.75),
        ],
    ));
    v.push(rect(l.pplus, 9.8, 1.8, 12.2, 4.2)); // DF.10: the keyhole
    v.extend(l.ring(14.0, 2.0, 16.0, 4.0, 14.75, 2.75, 15.255, 3.265)); // clean
    v.push(poly(
        l.comp,
        &[
            (18.0, 2.0),
            (20.0, 2.0),
            (20.0, 4.0),
            (19.25, 4.0),
            (19.25, 3.5),
            (18.75, 3.5),
            (18.75, 4.0),
            (18.0, 4.0),
        ],
    ));
    v.push(rect(l.pplus, 17.8, 1.8, 20.2, 4.2)); // clean: a notch is no hole
    v.extend(l.ring(2.0, 6.0, 4.0, 8.0, 2.65, 6.65, 3.35, 7.35));
    v.extend(l.p(2.75, 6.75, 3.25, 7.25)); // DF.10: 0.24 of field round the island
    v.extend(l.ring(19.0, 6.0, 21.0, 8.0, 19.75, 6.75, 20.25, 7.25)); // DF.10, on x = 20
    v.extend(l.ring(19.0, 10.0, 21.0, 12.0, 20.0, 10.75, 20.5, 11.25)); // DF.10, wall on x = 20
    v.extend(l.ring(39.0, 6.0, 41.0, 8.0, 39.745, 6.745, 40.255, 7.255)); // clean, on x = 40
    write_h("DF.10.h1", v);
}

// --- DF.11: min. length of butting COMP edge 0.3 ---

fn df11_h(l: &L) {
    // h1 - an N+/P+ boundary across a 0.3 wide active is a 0.3 butting edge (clean),
    // across 0.295 it is not.  A 2 x 0.25 active with the boundary along its length has
    // a 2.0 butting edge (clean; the active is 0.25 wide, which DF.1a allows).  A 0.25
    // wide N+ finger off a 1 µm plate whose boundary is 1.0 long: the butting edge is
    // 1.0 (clean).  A 0.25 N+ active without a P+ half is not butted at all (clean).
    // The 0.295 butt at x = 20 fails.
    let mut v = vec![];
    v.extend(l.butted(2.0, 2.0, 4.0, 2.3, 3.0)); // clean
    v.extend(l.butted(2.0, 4.0, 4.0, 4.295, 3.0)); // DF.11
    v.push(rect(l.comp, 6.0, 2.0, 8.0, 2.25));
    v.push(rect(l.nplus, 5.8, 1.8, 8.2, 2.125));
    v.push(rect(l.pplus, 5.8, 2.125, 8.2, 2.45)); // clean: the butting edge is 2.0
    v.push(rect(l.comp, 10.0, 2.0, 12.0, 3.0));
    v.push(rect(l.comp, 12.0, 2.375, 14.0, 2.625));
    v.push(rect(l.nplus, 9.8, 1.8, 11.0, 3.2));
    v.push(rect(l.pplus, 11.0, 1.8, 14.2, 3.2)); // clean: the butting edge is 1.0
    v.extend(l.n(2.0, 6.0, 4.0, 6.25)); // clean: not butted
    v.extend(l.butted(19.0, 2.0, 21.0, 2.295, 20.0)); // DF.11, the butt on x = 20
    write_h("DF.11.h1", v);
}

// --- DF.12: COMP not covered by N+ or P+ ---

fn df12_h(l: &L) {
    // h1 - an active with N+ over it is covered; one whose N+ stops 0.005 short is
    // not.  An active under SCHOTTKY_DIODE with no implant is the marked exception;
    // with the marker over half of it the other half is uncovered and unmarked (fails).
    // An N+ drawn exactly on the active covers it.  N+ and P+ overlapping on the
    // active cover it twice (no DF.12; DF.3b's P+ tap over the N+ source/drain).  An
    // active whose N+ ends on x = 20 is uncovered from there.
    let mut v = vec![];
    v.extend(l.n(2.0, 2.0, 3.0, 3.0)); // clean
    v.push(rect(l.comp, 5.0, 2.0, 6.0, 3.0));
    v.push(rect(l.nplus, 4.8, 1.8, 5.995, 3.2)); // DF.12
    v.push(rect(l.comp, 8.0, 2.0, 9.0, 3.0));
    v.push(rect(l.schottky, 7.8, 1.8, 9.2, 3.2)); // clean: marked
    v.push(rect(l.comp, 11.0, 2.0, 13.0, 3.0));
    v.push(rect(l.schottky, 10.8, 1.8, 12.0, 3.2)); // DF.12: half of it unmarked
    v.push(rect(l.comp, 15.0, 2.0, 16.0, 3.0));
    v.push(rect(l.nplus, 15.0, 2.0, 16.0, 3.0)); // clean: coincident
    v.push(rect(l.comp, 18.0, 2.0, 19.0, 3.0));
    v.push(rect(l.nplus, 17.8, 1.8, 19.2, 3.2));
    v.push(rect(l.pplus, 18.5, 1.8, 19.2, 3.2)); // DF.3b, no DF.12
    v.push(rect(l.comp, 19.5, 6.0, 20.5, 7.0));
    v.push(rect(l.nplus, 19.3, 5.8, 20.0, 7.2)); // DF.12, from x = 20
    v.extend(l.p(2.0, 5.0, 4.0, 6.0));
    write_h("DF.12.h1", v);
}

// --- DF.13: max. distance of N-well tap from P+ active in the well 20 / 15 ---

fn df13_h(l: &L) {
    // h1 - a P+ source/drain 20.0 from the N+ tap in its well is legal, 20.005 is not;
    // under Dualgate 15.0 is, 15.005 is not.
    let nw = |v: &mut Vec<GdsElement>, y: f64, m: f64, gap: f64| {
        v.push(rect(l.nwell, 2.0, y, 42.0, y + 4.0));
        v.extend(l.p(2.0 + m, y + 1.0, 3.0 + m, y + 3.0));
        v.extend(l.n(3.0 + m + gap, y + 1.0, 4.0 + m + gap, y + 3.0));
    };
    let mut v = vec![];
    nw(&mut v, 2.0, 0.5, 20.0); // clean
    nw(&mut v, 8.0, 0.5, 20.005); // DF.13_LV
    v.push(rect(l.dualgate, 1.5, 13.5, 42.5, 24.5));
    nw(&mut v, 14.0, 0.7, 15.0); // clean
    nw(&mut v, 20.0, 0.7, 15.005); // DF.13_MV
    write_h("DF.13.h1", v);

    // h2 - the reach is through the well.  A tap in the next well over, 4 µm away, is
    // no tap for this well (fails).  An L-shaped well with the source/drain at one end
    // and the tap at the other: 18.6 across the corner, 26 through the well (fails: the
    // distance the rule bounds is the well's resistance path).  A source/drain and a tap
    // 3 µm apart in a deep well with no drawn N-well are in one N-region (clean).  A
    // 20.0 gap across x = 20 is clean; in one well a source/drain 20.005 from the tap
    // across x = 20 and x = 40, and another 36 away, fail once each.  In a wide well a
    // source/drain 19 each way from the tap (26.9) fails.
    let mut v = vec![];
    v.push(rect(l.nwell, 2.0, 2.0, 6.0, 6.0));
    v.extend(l.p(2.5, 3.0, 3.5, 5.0)); // DF.13_LV: no tap in this well
    v.push(rect(l.nwell, 7.0, 2.0, 11.0, 6.0));
    v.extend(l.n(7.5, 3.0, 8.5, 5.0));
    v.push(rect(l.nwell, 2.0, 8.0, 16.0, 10.0));
    v.push(rect(l.nwell, 14.0, 8.0, 16.0, 26.0));
    v.extend(l.p(2.5, 8.5, 3.5, 9.5)); // DF.13_LV: 26 through the L
    v.extend(l.n(14.5, 24.5, 15.5, 25.5));
    v.push(rect(l.dnwell, 18.0, 2.0, 30.0, 8.0));
    v.extend(l.p(20.0, 4.0, 22.0, 6.0)); // clean: the tap is 3 µm away
    v.extend(l.n(25.0, 4.0, 27.0, 6.0));
    v.push(rect(l.nwell, 2.0, 28.0, 40.0, 32.0));
    v.extend(l.p(2.5, 29.0, 3.5, 31.0)); // clean: 20.0 across x = 20
    v.extend(l.n(23.5, 29.0, 24.5, 31.0));
    v.push(rect(l.nwell, 2.0, 34.0, 42.0, 38.0));
    v.extend(l.p(18.5, 35.0, 19.5, 37.0)); // DF.13_LV: 20.005 across x = 20 and 40
    v.extend(l.p(2.5, 35.0, 3.5, 37.0)); // DF.13_LV: 36
    v.extend(l.n(39.505, 35.0, 40.505, 37.0));
    v.push(rect(l.nwell, 44.0, 2.0, 70.0, 28.0));
    v.extend(l.p(45.0, 3.0, 46.0, 4.0)); // DF.13_LV: 26.9 diagonal
    v.extend(l.n(65.0, 23.0, 66.0, 24.0));
    write_h("DF.13.h2", v);
}

// --- DF.14: max. distance of substrate tap from N+ active outside the wells 20 / 15 ---

fn df14_h(l: &L) {
    // h1 - an N+ source/drain 20.0 from a P+ tap is legal, 20.005 is not; diagonally
    // 14.145 each way (20.004) fails and 14.14 (19.997) does not, and 19 each way
    // (26.9) fails outright; under Dualgate 15.0 is legal, 15.005 is not.  The pairs
    // are 40 µm apart so that no pair's tap is within any reading of reach of another.
    let mut v = vec![];
    v.extend(l.n(2.0, 2.0, 3.0, 4.0));
    v.extend(l.p(23.0, 2.0, 24.0, 4.0)); // clean
    v.extend(l.n(2.0, 42.0, 3.0, 44.0));
    v.extend(l.p(23.005, 42.0, 24.005, 44.0)); // DF.14_LV
    v.extend(l.n(2.0, 82.0, 3.0, 83.0));
    v.extend(l.p(17.145, 97.145, 18.145, 98.145)); // DF.14_LV: 20.004 diagonal
    v.extend(l.n(2.0, 122.0, 3.0, 123.0));
    v.extend(l.p(17.14, 137.14, 18.14, 138.14)); // clean: 19.997 diagonal
    v.extend(l.n(2.0, 162.0, 3.0, 163.0));
    v.extend(l.p(22.0, 182.0, 23.0, 183.0)); // DF.14_LV: 26.9 diagonal
    v.push(rect(l.dualgate, 1.5, 201.5, 30.5, 206.5));
    v.extend(l.n(2.0, 202.0, 3.0, 204.0));
    v.extend(l.p(18.0, 202.0, 19.0, 204.0)); // clean
    v.push(rect(l.dualgate, 1.5, 241.5, 30.5, 246.5));
    v.extend(l.n(2.0, 242.0, 3.0, 244.0));
    v.extend(l.p(18.005, 242.0, 19.005, 244.0)); // DF.14_MV
    write_h("DF.14.h1", v);

    // h2 - the tile line and the isolated well.  20.0 across x = 20 is clean; 20.005
    // across x = 20 and x = 40 fails.  An N+ source/drain in the P-well of a deep well
    // with the nearest P+ tap outside the deep well, 9.5 away: the manual's letter
    // (P+ active outside N-well, N+ active outside N-well) is met (clean).
    let mut v = vec![];
    v.extend(l.n(2.0, 2.0, 3.0, 4.0));
    v.extend(l.p(23.0, 2.0, 24.0, 4.0)); // clean
    v.extend(l.n(18.0, 30.0, 19.0, 32.0));
    v.extend(l.p(39.005, 30.0, 40.005, 32.0)); // DF.14_LV
    v.push(rect(l.dnwell, 44.0, 2.0, 56.0, 12.0));
    v.push(rect(l.lvpwell, 45.0, 3.0, 55.0, 11.0));
    v.extend(l.n(47.0, 6.0, 49.0, 8.0)); // clean: the tap outside the deep well
    v.extend(l.p(58.5, 6.0, 60.5, 8.0));
    write_h("DF.14.h2", v);
}

// --- DF.16: min. NWELL space to N+ active outside the wells 0.43 / 0.6 ---

fn df16_h(l: &L) {
    // h1 - an N+ source/drain 0.43 from an N-well is legal, 0.425 is not; under
    // Dualgate (over both) 0.6 is, 0.595 is not.  A 3.3 V source/drain 0.425 from a
    // well Dualgate lies on elsewhere is 0.425 from an N-well (fails; the manual bounds
    // the pair at 0.43 at least).  A 5 V source/drain under its own Dualgate, 0.425
    // from a 3.3 V well the marker stops 0.185 short of, likewise.  Under YMTP_MK or
    // SRAMCORE the pair is the cell's (clean).  P+ taps answer DF.14.
    let mut v = vec![];
    v.push(rect(l.nwell, 2.0, 2.0, 5.0, 5.0));
    v.extend(l.n(5.43, 2.5, 7.43, 4.5)); // clean
    v.push(rect(l.nwell, 2.0, 7.0, 5.0, 10.0));
    v.extend(l.n(5.425, 7.5, 7.425, 9.5)); // DF.16_LV
    v.push(rect(l.dualgate, 1.5, 11.5, 12.5, 21.5));
    v.push(rect(l.nwell, 2.0, 12.0, 5.0, 15.0));
    v.extend(l.n(5.6, 12.5, 7.6, 14.5)); // clean
    v.push(rect(l.nwell, 2.0, 17.0, 5.0, 20.0));
    v.extend(l.n(5.595, 17.5, 7.595, 19.5)); // DF.16_MV
    v.push(rect(l.nwell, 14.0, 2.0, 20.0, 5.0));
    v.push(rect(l.dualgate, 18.5, 3.5, 20.5, 5.5));
    v.extend(l.n(14.0, 5.425, 16.0, 7.425)); // DF.16_LV: 3.3 V active, 5 V well
    v.push(rect(l.nwell, 14.0, 9.0, 20.0, 12.0));
    v.push(rect(l.dualgate, 13.76, 12.185, 16.24, 14.665));
    v.extend(l.n(14.0, 12.425, 16.0, 14.425)); // DF.16_MV: 5 V active, 3.3 V well
    v.push(rect(l.nwell, 24.0, 2.0, 27.0, 5.0));
    v.extend(l.n(27.425, 2.5, 29.425, 4.5));
    v.push(rect(l.ymtp_mk, 23.5, 1.5, 30.0, 5.0)); // clean: MTP
    v.push(rect(l.nwell, 24.0, 7.0, 27.0, 10.0));
    v.extend(l.n(27.425, 7.5, 29.425, 9.5));
    v.push(rect(l.sramcore, 27.0, 7.0, 30.0, 10.0)); // clean: SRAM
    v.extend(l.p(9.0, 2.0, 11.0, 4.0));
    v.extend(l.p(9.0, 12.0, 11.0, 14.0));
    write_h("DF.16.h1", v);

    // h2 - the marker far along the well.  A 300 µm N-well with Dualgate over its last
    // 12 µm is a 5 V well; a 5 V source/drain under its own marker 0.5 from it at x = 10
    // fails the 5 V column whatever tile the well's marker lies in.  A 3.3 V well of that
    // length 0.425 from a 3.3 V source/drain fails at 3.3 V.
    let mut v = vec![];
    v.push(rect(l.nwell, 2.0, 2.0, 302.0, 4.0));
    v.push(rect(l.dualgate, 290.0, 1.5, 302.5, 4.5));
    v.push(rect(l.dualgate, 9.76, 4.26, 12.24, 6.74));
    v.extend(l.n(10.0, 4.5, 12.0, 6.5)); // DF.16_MV
    v.push(rect(l.nwell, 2.0, 8.0, 302.0, 10.0));
    v.extend(l.n(10.0, 10.425, 12.0, 12.425)); // DF.16_LV
    v.extend(l.p(14.0, 5.0, 16.0, 7.0));
    write_h("DF.16.h2", v);
}

// --- DF.17: min. NWELL space to P+ active outside the wells 0.12 / 0.16 ---

fn df17_h(l: &L) {
    // h1 - a P+ tap 0.12 from an N-well is legal, 0.115 is not; under Dualgate 0.16
    // is, 0.155 is not.  A tap butted against the well's edge is 0 from it (fails).  A
    // tap Dualgate covers half of (DV.7 allows that of a substrate tap) is a 5 V active,
    // 0.115 from a 3.3 V well (fails).  A 3.3 V tap 0.115 from a well Dualgate lies on
    // elsewhere likewise.
    let mut v = vec![];
    v.push(rect(l.nwell, 2.0, 2.0, 5.0, 5.0));
    v.extend(l.p(5.12, 2.5, 7.12, 4.5)); // clean
    v.push(rect(l.nwell, 2.0, 7.0, 5.0, 10.0));
    v.extend(l.p(5.115, 7.5, 7.115, 9.5)); // DF.17_LV
    v.push(rect(l.dualgate, 1.5, 11.5, 9.5, 21.5));
    v.push(rect(l.nwell, 2.0, 12.0, 5.0, 15.0));
    v.extend(l.p(5.16, 12.5, 7.16, 14.5)); // clean
    v.push(rect(l.nwell, 2.0, 17.0, 5.0, 20.0));
    v.extend(l.p(5.155, 17.5, 7.155, 19.5)); // DF.17_MV
    v.push(rect(l.nwell, 12.0, 2.0, 15.0, 5.0));
    v.extend(l.p(15.0, 2.5, 17.0, 4.5)); // DF.17_LV: butted to the well
    v.push(rect(l.nwell, 12.0, 7.0, 15.0, 10.0));
    v.extend(l.p(15.115, 7.5, 17.115, 9.5)); // DF.17_MV: 5 V tap, 3.3 V well
    v.push(rect(l.dualgate, 16.0, 7.0, 18.0, 10.0));
    v.push(rect(l.nwell, 12.0, 12.0, 15.0, 15.0));
    v.push(rect(l.dualgate, 12.0, 14.9, 14.0, 16.0));
    v.extend(l.p(15.115, 12.5, 17.115, 14.5)); // DF.17_LV: 3.3 V tap, 5 V well
    write_h("DF.17.h1", v);
}

// --- DF.18: min. DNWELL space to P+ active outside the wells 2.5 ---

fn df18_h(l: &L) {
    // h1 - a P+ tap 2.5 from a deep well is legal, 2.495 is not; diagonally 1.765 each
    // way (2.496) fails, 1.77 (2.503) does not.  A P+ source/drain in an N-well 1.5
    // from the deep well is inside an N-well and not this rule's (clean).
    let mut v = vec![];
    v.push(rect(l.dnwell, 2.0, 2.0, 8.0, 8.0));
    v.extend(l.p(10.5, 4.0, 12.5, 6.0)); // clean
    v.push(rect(l.dnwell, 16.0, 2.0, 22.0, 8.0));
    v.extend(l.p(24.495, 4.0, 26.495, 6.0)); // DF.18
    v.push(rect(l.dnwell, 2.0, 12.0, 8.0, 18.0));
    v.extend(l.p(9.765, 19.765, 11.765, 21.765)); // DF.18: 2.496 diagonal
    v.push(rect(l.dnwell, 16.0, 12.0, 22.0, 18.0));
    v.extend(l.p(23.77, 19.77, 25.77, 21.77)); // clean: 2.503 diagonal
    v.push(rect(l.dnwell, 30.0, 2.0, 36.0, 8.0));
    v.push(rect(l.nwell, 37.5, 2.0, 42.0, 8.0));
    v.extend(l.p(38.0, 4.0, 40.0, 6.0)); // clean: inside an N-well
    v.extend(l.n(40.5, 4.0, 41.5, 6.0));
    write_h("DF.18.h1", v);
}

// --- DF.19: min. DNWELL space to N+ active outside the wells 3.2 / 3.28 ---

fn df19_h(l: &L) {
    // h1 - an N+ source/drain 3.2 from a deep well is legal, 3.195 is not; under
    // Dualgate 3.28 is, 3.275 is not; diagonally 2.262 each way (3.199) fails.  An N+
    // tap in an N-well 1.0 from the deep well is inside an N-well and not this rule's
    // (clean).  P+ taps, 2.5 or more from the deep wells, answer DF.14.
    let mut v = vec![];
    v.push(rect(l.dnwell, 2.0, 2.0, 8.0, 8.0));
    v.extend(l.n(11.2, 4.0, 13.2, 6.0)); // clean
    v.extend(l.p(15.0, 4.0, 17.0, 6.0));
    v.push(rect(l.dnwell, 20.0, 2.0, 26.0, 8.0));
    v.extend(l.n(29.195, 4.0, 31.195, 6.0)); // DF.19_LV
    v.extend(l.p(33.0, 4.0, 35.0, 6.0));
    v.push(rect(l.dualgate, 1.5, 11.5, 36.5, 21.5));
    v.push(rect(l.dnwell, 2.0, 12.0, 8.0, 18.0));
    v.extend(l.n(11.28, 14.0, 13.28, 16.0)); // clean
    v.extend(l.p(15.0, 14.0, 17.0, 16.0));
    v.push(rect(l.dnwell, 20.0, 12.0, 26.0, 18.0));
    v.extend(l.n(29.275, 14.0, 31.275, 16.0)); // DF.19_MV
    v.extend(l.p(33.0, 14.0, 35.0, 16.0));
    v.push(rect(l.dnwell, 2.0, 24.0, 8.0, 30.0));
    v.extend(l.n(10.262, 32.262, 12.262, 34.262)); // DF.19_LV: 3.199 diagonal
    v.extend(l.p(14.0, 32.0, 16.0, 34.0));
    v.push(rect(l.dnwell, 20.0, 24.0, 26.0, 30.0));
    v.push(rect(l.nwell, 27.0, 24.0, 31.0, 30.0));
    v.extend(l.n(27.5, 26.0, 29.5, 28.0)); // clean: inside an N-well
    write_h("DF.19.h1", v);
}
