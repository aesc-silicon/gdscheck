// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! N+ implant: a good and a bad pattern for every rule in the `nplus` deck.
//!
//! The deck asks the same two questions over and over - how far the N+ marker must stay
//! from a P+ active, and how far it must overhang one of its own - and answers them
//! differently depending on where the active sits.  NP.3a/bi/bii/ci/cii split the spacing
//! five ways by well, and NP.5b/ci/cii/di/dii split the overhang five ways more, with the
//! tighter number reserved for an active deep inside a well and the looser one for an
//! active within 0.429 µm of its edge.  So most fixtures here are the same small active
//! moved from one well to another, and the whole art is placing it far enough into the
//! region a rule cares about that no neighbouring rule claims it.
//!
//! Two pairs cannot be separated at all.  NP.3d (N+ active over P+ active) and NP.3e (N+
//! marker over P+ active) expand to the same intersection of comp, nplus and pplus.  And
//! an N+ marker touching poly connected to a P-channel gate, which NP.12 forbids, is
//! always touching a poly edge within 0.32 µm of that gate, which is exactly what NP.4b
//! measures - so NP.12's fixture fires NP.4b at distance zero.  Both are declared.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/nplus";

struct Ctx {
    nplus: (i16, i16),
    pplus: (i16, i16),
    comp: (i16, i16),
    poly2: (i16, i16),
    sab: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
}

/// An N+ active: comp with the marker overhanging it by `enc` on the left and 0.3 elsewhere.
fn active(c: &Ctx, x: f64, y: f64, w: f64, h: f64, enc: f64) -> Vec<GdsElement> {
    vec![
        rect(c.comp, x, y, x + w, y + h),
        rect(c.nplus, x - enc, y - 0.3, x + w + 0.3, y + h + 0.3),
    ]
}

/// A P+ active with a poly bar across it - a P-channel gate, which needs an N-well.
fn pmos(c: &Ctx) -> Vec<GdsElement> {
    let o = OFFSET;
    vec![
        rect(c.nwell, o, o, o + 6.0, o + 4.0),
        rect(c.comp, o + 1.0, o + 1.5, o + 4.0, o + 2.5),
        // Index 2 - NP.4a replaces this one to butt an N+ marker against it.
        rect(c.pplus, o + 0.7, o + 1.2, o + 4.3, o + 2.8),
        rect(c.poly2, o + 2.5, o + 1.2, o + 2.78, o + 2.8),
    ]
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        comp: layer(pdk, "comp"),
        poly2: layer(pdk, "poly2_drawn"),
        sab: layer(pdk, "sab"),
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // NP.1: the marker's own width.
    let np1 = |w: f64| vec![rect(c.nplus, o, o, o + w, o + 3.0)];
    write("NP.1", "good", np1(1.0));
    write("NP.1", "bad", np1(0.395));

    // NP.2: two markers.
    let np2 = |gap: f64| {
        vec![
            rect(c.nplus, o, o, o + 1.0, o + 1.0),
            rect(c.nplus, o + 1.0 + gap, o, o + 2.0 + gap, o + 1.0),
        ]
    };
    write("NP.2", "good", np2(1.0));
    write("NP.2", "bad", np2(0.395));

    // NP.3a: marker to a P+ active in a deep well.  An active in an N-well would do as
    // well by the rule's own layer, but NP.3ci reaches 0.429 µm around an N-well and so
    // claims that one too; an active in a deep well is the branch only this rule has.
    let np3a = |gap: f64| {
        vec![
            rect(c.dnwell, o, o, o + 3.0, o + 3.0),
            rect(c.comp, o + 1.5, o + 1.0, o + 2.5, o + 2.0),
            rect(c.pplus, o + 1.4, o + 0.9, o + 2.6, o + 2.1),
            rect(c.nplus, o + 2.5 + gap, o + 1.0, o + 3.5 + gap, o + 2.0),
        ]
    };
    write("NP.3a", "good", np3a(1.0));
    write("NP.3a", "bad", np3a(0.155));

    // NP.3bi/bii: marker to a P+ active inside a P-well in a deep well, split by whether
    // the active is within 0.429 µm of the P-well's edge or deeper in.
    let np3b = |gap: f64, in_band: bool| {
        // The P-well runs from o+1 to o+7; its band is the first 0.429 µm inside.
        let (px, pw) = if in_band {
            (o + 1.0, 0.3)
        } else {
            (o + 3.0, 1.0)
        };
        vec![
            rect(c.dnwell, o, o, o + 8.0, o + 8.0),
            rect(c.lvpwell, o + 1.0, o + 1.0, o + 7.0, o + 7.0),
            rect(c.comp, px, o + 3.0, px + pw, o + 5.0),
            rect(c.pplus, px - 0.1, o + 2.9, px + pw + 0.1, o + 5.1),
            rect(
                c.nplus,
                px + pw + gap,
                o + 3.0,
                px + pw + gap + 1.0,
                o + 5.0,
            ),
        ]
    };
    write("NP.3bi", "good", np3b(1.0, true));
    write("NP.3bi", "bad", np3b(0.155, true));
    write("NP.3bii", "good", np3b(1.0, false));
    write("NP.3bii", "bad", np3b(0.075, false));

    // NP.3ci/cii: marker to a P+ active outside any deep well, split by whether the active
    // is within 0.429 µm of a stand-alone N-well.  The active sits in the well's collar,
    // not in the well, so NP.3a's layer does not claim it.
    let np3ci = |gap: f64| {
        vec![
            rect(c.nwell, o, o, o + 3.0, o + 3.0),
            rect(c.comp, o + 3.1, o + 1.0, o + 3.4, o + 2.0),
            rect(c.pplus, o + 3.0, o + 0.9, o + 3.5, o + 2.1),
            rect(c.nplus, o + 3.4 + gap, o + 1.0, o + 4.4 + gap, o + 2.0),
        ]
    };
    write("NP.3ci", "good", np3ci(1.0));
    write("NP.3ci", "bad", np3ci(0.155));
    let np3cii = |gap: f64| {
        vec![
            rect(c.comp, o, o, o + 1.0, o + 1.0),
            rect(c.pplus, o - 0.1, o - 0.1, o + 1.1, o + 1.1),
            rect(c.nplus, o + 1.0 + gap, o, o + 2.0 + gap, o + 1.0),
        ]
    };
    write("NP.3cii", "good", np3cii(1.0));
    write("NP.3cii", "bad", np3cii(0.075));

    // NP.3d and NP.3e: both markers over one active.  The same intersection, so each
    // fixture fires both rules.
    let np3de = |overlap: bool| {
        let mut v = vec![
            rect(c.comp, o, o, o + 2.0, o + 1.0),
            rect(c.pplus, o - 0.1, o - 0.1, o + 1.0, o + 1.1),
        ];
        // Covering the whole active, not part of it: a marker edge that stops inside
        // the active would land exactly on the N+ active's own edge, which is NP.5b's
        // measurement at zero.
        let left = if overlap { o - 0.3 } else { o + 1.0 };
        v.push(rect(c.nplus, left, o - 0.3, o + 2.3, o + 1.3));
        v
    };
    for id in ["NP.3d", "NP.3e"] {
        write(id, "good", np3de(false));
        write(id, "bad", np3de(true));
    }

    // NP.5a: the marker's overhang of an N-channel gate.
    let np5a = |enc: f64| {
        vec![
            rect(c.comp, o, o, o + 2.0, o + 1.0),
            rect(c.poly2, o + 0.8, o - 0.3, o + 1.08, o + 1.3),
            rect(c.nplus, o - 0.3, o - enc, o + 2.3, o + 1.0 + enc),
        ]
    };
    write("NP.5a", "good", np5a(0.4));
    write("NP.5a", "bad", np5a(0.225));

    // NP.5b: the overhang of a plain N+ active, one in neither an N-well nor a deep well.
    let np5b = |enc: f64| active(&c, o, o, 1.0, 1.0, enc);
    write("NP.5b", "good", np5b(0.3));
    write("NP.5b", "bad", np5b(0.155));

    // NP.5ci/cii: the overhang inside a deep well, split by whether the active's edge is
    // within 0.429 µm of the P-well in it.  Only the left edge is short; the others clear
    // 0.3 µm, which satisfies whichever of the two rules claims them.
    let np5c = |enc: f64, near: bool| {
        let x = if near { o + 5.0 + 0.2 } else { o + 5.0 + 1.5 };
        let mut v = vec![
            rect(c.dnwell, o, o, o + 10.0, o + 10.0),
            rect(c.lvpwell, o + 1.0, o + 1.0, o + 5.0, o + 9.0),
        ];
        v.extend(active(&c, x, o + 4.5, 1.0, 1.0, enc));
        v
    };
    write("NP.5ci", "good", np5c(0.3, true));
    write("NP.5ci", "bad", np5c(0.155, true));
    write("NP.5cii", "good", np5c(0.3, false));
    write("NP.5cii", "bad", np5c(0.015, false));

    // NP.5di/dii: the overhang inside an N-well, split by whether the active's edge is in
    // the well's inner band or its core.
    let np5d = |enc: f64, near: bool| {
        let x = if near { o + 0.2 } else { o + 1.5 };
        let mut v = vec![rect(c.nwell, o, o, o + 4.0, o + 4.0)];
        v.extend(active(&c, x, o + 1.5, 1.0, 1.0, enc));
        v
    };
    write("NP.5di", "good", np5d(0.3, true));
    write("NP.5di", "bad", np5d(0.155, true));
    write("NP.5dii", "good", np5d(0.3, false));
    write("NP.5dii", "bad", np5d(0.015, false));

    // NP.6: a butted N+/P+ active - the comp has to reach 0.22 µm past the P+ part, which
    // is the same as saying the N+ part has to be that long.
    let np6 = |n_len: f64| {
        vec![
            rect(c.comp, o, o, o + 1.2 + n_len, o + 1.0),
            rect(c.pplus, o - 0.3, o - 0.3, o + 1.2, o + 1.3),
            rect(c.nplus, o + 1.2, o - 0.3, o + 1.2 + n_len + 0.3, o + 1.3),
        ]
    };
    write("NP.6", "good", np6(0.5));
    write("NP.6", "bad", np6(0.215));

    // NP.7 and NP.9: the marker beside, then over, a silicided-block poly.
    let poly_sab = |dx: f64, over: bool| {
        let mut v = vec![
            rect(c.poly2, o, o, o + 1.0, o + 1.0),
            rect(c.sab, o - 0.2, o - 0.2, o + 1.2, o + 1.2),
        ];
        v.push(if over {
            rect(c.nplus, o - dx, o - dx, o + 1.0 + dx, o + 1.0 + dx)
        } else {
            rect(c.nplus, o + 1.0 + dx, o, o + 2.0 + dx, o + 1.0)
        });
        v
    };
    write("NP.7", "good", poly_sab(0.4, false));
    write("NP.7", "bad", poly_sab(0.175, false));
    write("NP.9", "good", poly_sab(0.4, true));
    write("NP.9", "bad", poly_sab(0.175, true));

    // NP.10: the marker over a silicided-block active.
    let np10 = |enc: f64| {
        vec![
            rect(c.comp, o, o, o + 1.0, o + 1.0),
            rect(c.sab, o - 0.2, o - 0.2, o + 1.2, o + 1.2),
            rect(c.nplus, o - enc, o - enc, o + 1.0 + enc, o + 1.0 + enc),
        ]
    };
    write("NP.10", "good", np10(0.4));
    write("NP.10", "bad", np10(0.175));

    // NP.8a: the marker's area, at a legal width.
    write("NP.8a", "good", vec![rect(c.nplus, o, o, o + 0.6, o + 0.6)]);
    write(
        "NP.8a",
        "bad",
        vec![rect(c.nplus, o, o, o + 0.55, o + 0.545)],
    );

    // NP.8b: the area of a hole in the marker, at a legal notch.
    let ring = |hw: f64, hh: f64| {
        let (w, h) = (hw + 1.6, hh + 1.6);
        vec![
            rect(c.nplus, o, o, o + w, o + 0.8),
            rect(c.nplus, o, o + 0.8 + hh, o + w, o + h),
            rect(c.nplus, o, o + 0.8, o + 0.8, o + 0.8 + hh),
            rect(c.nplus, o + 0.8 + hw, o + 0.8, o + w, o + 0.8 + hh),
        ]
    };
    write("NP.8b", "good", ring(0.6, 0.6));
    write("NP.8b", "bad", ring(0.55, 0.545));

    // NP.4a: a butted N+/P+ edge beside a P-channel gate.  The gate's near edge is at
    // o+2.5, so the butted edge goes `gap` to the left of it.
    let np4a = |gap: f64| {
        // The P+ marker stops at the butted edge and the N+ marker takes over there.
        let xb = o + 2.5 - gap;
        let mut v = pmos(&c);
        v[2] = rect(c.pplus, xb, o + 1.2, o + 4.3, o + 2.8);
        v.push(rect(c.nplus, o + 0.7, o + 1.2, xb, o + 2.8));
        v
    };
    write("NP.4a", "good", np4a(0.5));
    write("NP.4a", "bad", np4a(0.315));

    // NP.4b: a marker edge beside the side of the gate extension - the 0.3 µm of poly
    // past the active - and only there, so that NP.4a, which asks the same of a butted
    // edge beside the gate itself, has nothing to say.  The rule reads the extension's
    // sides, not its end cap: an edge facing the cap is out of its reach.
    let np4b = |gap: f64| {
        let mut v = pmos(&c);
        let xb = o + 2.5 - gap;
        v.push(rect(c.nplus, o + 0.7, o + 2.5, xb, o + 3.2));
        v
    };
    write("NP.4b", "good", np4b(0.4));
    write("NP.4b", "bad", np4b(0.215));

    // NP.11: a butted N+/P+ edge in an N-well's 0.429 µm collar.
    let np11 = |xb: f64| {
        vec![
            rect(c.nwell, o, o, o + 3.0, o + 3.0),
            rect(c.comp, o + 3.05, o + 1.0, o + 4.05, o + 2.0),
            rect(c.pplus, o + 2.9, o + 0.9, xb, o + 2.1),
            rect(c.nplus, xb, o + 0.7, o + 4.35, o + 2.3),
        ]
    };
    write("NP.11", "good", np11(o + 3.8));
    write("NP.11", "bad", np11(o + 3.3));

    // NP.12: a marker touching poly that runs back to a P-channel gate.  Anything close
    // enough to touch it is also inside NP.4b's reach, so this fires both.
    let np12 = |over: bool| {
        let mut v = pmos(&c);
        let y = if over { o + 2.7 } else { o + 3.4 };
        v.push(rect(c.nplus, o + 2.0, y, o + 3.28, y + 1.0));
        v
    };
    write("NP.12", "good", np12(false));
    write("NP.12", "bad", np12(true));
}
