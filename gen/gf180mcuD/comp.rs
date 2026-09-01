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
use crate::helpers::{layer, library, rect, write_gz};
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

    // DF.4c: how far an N-well holds a P source/drain.
    let df4c = |enc: f64, mv: bool| {
        let mut v = vec![rect(c.nwell, o, o, o + 6.0, o + 6.0)];
        v.extend(ntie(&c, o + 7.0, o));
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

    // DF.4e: how far a deep well holds a P source/drain.
    let df4e = |enc: f64, mv: bool| {
        let mut v = vec![rect(c.dnwell, o, o, o + 8.0, o + 8.0)];
        v.extend(ntie(&c, o + 9.0, o + 1.0));
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

    // DF.7: a P source/drain in the deep well, to the P-well beside it.
    let df7 = |gap: f64, mv: bool| {
        let mut v = vec![
            rect(c.dnwell, o, o, o + 12.0, o + 8.0),
            rect(c.lvpwell, o + 1.0, o + 1.0, o + 5.0, o + 7.0),
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
}
