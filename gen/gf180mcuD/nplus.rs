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

    hardening(pdk);
}

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.8 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/nplus/NP.<rule>.h<n>.gds.gz` with a case in the
// `hardening_nplus` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/nplus.md.
//
// What these draw is the deck's own conditions - which PCOMP a spacing rule measures to
// and which well decides its value, the butted N+/P+ pair that exempts a marker, the
// band either side of a well edge, the gate the implant must cover, the poly the implant
// may not touch - at the bound and one grid step past it.  The generic classes (the
// bound on a bare layer, 45 degrees, unions, notches, arrays) are the engine family's.

/// Layers the hardening patterns draw on.
struct H {
    np: (i16, i16),
    pp: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    sab: (i16, i16),
    nw: (i16, i16),
    dn: (i16, i16),
    res: (i16, i16),
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
        res: layer(pdk, "resistor"),
    };
    let write = |id: &str, elems: Vec<GdsElement>| {
        write_gz(&format!("{DIR}/{id}.gds.gz"), library("TOP", elems));
    };

    // --- NP.2: "Space 0.4".  One rule for a gap and for a notch; the deck runs
    // `min_space` and `min_notch` on the same layer, so a notch must be counted once.
    // h1 - (a) a U whose 0.395 notch straddles the tile line x = 20; (b) a U whose notch
    // is 0.4, clean, straddling x = 40; (c) a 0.395 gap between two markers straddling
    // x = 21.  Two violations.
    write(
        "NP.2.h1",
        vec![
            rect(h.np, 18.0, 5.0, 19.8025, 8.0),
            rect(h.np, 20.1975, 5.0, 22.0, 8.0),
            rect(h.np, 18.0, 5.0, 22.0, 6.0),
            rect(h.np, 38.0, 5.0, 39.8, 8.0),
            rect(h.np, 40.2, 5.0, 42.0, 8.0),
            rect(h.np, 38.0, 5.0, 42.0, 6.0),
            rect(h.np, 18.0, 12.0, 20.8025, 14.0),
            rect(h.np, 21.1975, 12.0, 24.0, 14.0),
        ],
    );

    // --- NP.3a: "Space to PCOMP for PCOMP: (1) Inside Nwell (2) Outside LVPWELL but
    // inside DNWELL - 0.16".  The deck drops the rule for any marker that touches an
    // NCOMP butted to a PCOMP anywhere in the layout, not only the related one.
    //
    // h1 - one L-shaped marker.  Its foot carries an NCOMP butted to a PCOMP at x = 12
    // (a legal butted pair); its arm ends at x = 20, 0.155 from an unrelated PCOMP in a
    // deep well 8 um away.  The manual's exemption is the butting pair's own edge, so
    // the unrelated PCOMP is still 0.155 from the marker: NP.3a.
    let np3a_scene = |with_pair: bool| {
        let mut v = vec![
            // The marker: foot x 10..12 y 10..11, arm x 10..20 y 11..12.
            rect(h.np, 10.0, 10.0, 12.0, 11.0),
            rect(h.np, 10.0, 11.0, 20.0, 12.0),
            // The unrelated PCOMP, in a deep well and outside every P-well.
            rect(h.dn, 20.155, 9.0, 24.0, 13.0),
            rect(h.comp, 20.155, 10.5, 21.155, 11.5),
            rect(h.pp, 20.155, 10.3, 21.4, 11.7),
        ];
        if with_pair {
            v.push(rect(h.comp, 10.3, 10.2, 13.5, 10.8));
            v.push(rect(h.pp, 12.0, 9.8, 13.8, 10.95));
        }
        v
    };
    write("NP.3a.h1", np3a_scene(true));
    // h2 - the same scene without the butted pair: the control, NP.3a fires.
    write("NP.3a.h2", np3a_scene(false));

    // --- NP.3ci/cii: "Space to PCOMP: For Outside DNWELL: (i) For PCOMP space to Nwell
    // < 0.43 - 0.16; (ii) >= 0.43 - 0.08".  The deck classifies by where the PCOMP lies
    // in a 0.429 collar grown from the well.
    //
    // h1 - four scenes against a 2 um well.  (a) a PCOMP wholly inside the collar with
    // the marker 0.155 away: NP.3ci; (b) the same at 0.16: clean; (c) a PCOMP wholly
    // outside it (its near edge 0.43 from the well) with the marker 0.075 away:
    // NP.3cii; (d) the same at 0.08: clean.
    let np3c_near = |y: f64, gap: f64| {
        vec![
            rect(h.nw, 10.0, y - 0.5, 12.0, y + 1.5),
            rect(h.comp, 12.02, y, 12.42, y + 1.0),
            rect(h.pp, 11.95, y - 0.1, 12.5, y + 1.1),
            rect(h.np, 12.42 + gap, y, 13.42 + gap, y + 1.0),
        ]
    };
    let np3c_far = |y: f64, gap: f64| {
        vec![
            rect(h.nw, 10.0, y - 0.5, 12.0, y + 1.5),
            rect(h.comp, 12.43, y, 13.43, y + 1.0),
            rect(h.pp, 12.43, y - 0.1, 13.5, y + 1.1),
            rect(h.np, 13.43 + gap, y, 14.43 + gap, y + 1.0),
        ]
    };
    let mut v = np3c_near(10.0, 0.155);
    v.extend(np3c_near(14.0, 0.16));
    v.extend(np3c_far(18.0, 0.075));
    v.extend(np3c_far(22.0, 0.08));
    write("NP.3ci.h1", v);

    // h2 - one PCOMP straddling the collar's edge, which the deck puts at x = 20.429,
    // just past the tile line x = 20: a 2 um PCOMP running out of a well that ends at
    // x = 20, with the marker 0.075 above it along its whole length.  The near part is
    // inside the collar (0.16) and the far part outside it (0.08): both rules.
    write(
        "NP.3ci.h2",
        vec![
            rect(h.nw, 18.0, 9.0, 20.0, 13.0),
            rect(h.comp, 20.1, 10.0, 22.1, 11.0),
            rect(h.pp, 20.0, 9.9, 22.2, 11.0),
            rect(h.np, 18.0, 11.075, 22.1, 12.075),
        ],
    );

    // --- NP.3bi/bii: "Space to PCOMP: For Inside DNWELL, inside LVPWELL: (i) For PCOMP
    // overlap by LVPWELL < 0.43 - 0.16; (ii) >= 0.43 - 0.08".  The mirror of NP.3c, read
    // on a band 0.429 wide inside the P-well instead of a collar outside the N-well.
    //
    // h1 - one P-well in a deep well holding four scenes.  (a) a PCOMP wholly in the band
    // along the well's right wall with the marker 0.155 away: NP.3bi; (b) the same at
    // 0.16: clean; (c) a PCOMP in the well's core with the marker 0.075 away: NP.3bii;
    // (d) the same at 0.08: clean.
    let np3b_band = |y: f64, gap: f64| {
        vec![
            rect(h.comp, 27.58, y, 27.98, y + 1.0),
            rect(h.pp, 27.5, y - 0.1, 28.05, y + 1.1),
            rect(h.np, 26.2, y, 27.58 - gap, y + 1.0),
        ]
    };
    let np3b_core = |y: f64, gap: f64| {
        vec![
            rect(h.comp, 20.0, y, 21.0, y + 1.0),
            rect(h.pp, 19.9, y - 0.1, 21.1, y + 1.1),
            rect(h.np, 18.9, y, 20.0 - gap, y + 1.0),
        ]
    };
    let mut v = vec![
        rect(h.dn, 10.0, 8.0, 30.0, 30.0),
        rect(layer(pdk, "lvpwell"), 11.0, 9.0, 28.0, 29.0),
    ];
    v.extend(np3b_band(11.0, 0.155));
    v.extend(np3b_band(15.0, 0.16));
    v.extend(np3b_core(19.0, 0.075));
    v.extend(np3b_core(23.0, 0.08));
    write("NP.3bi.h1", v);

    // --- NP.8b: "Minimum area enclosed by Nplus - 0.35 um2".  The hole, not the shape.
    // h1 - (a) a ring round a 0.7 x 0.5 hole, exactly 0.35: clean; (b) the same hole
    // 0.495 tall, 0.3465: NP.8b; (c) a 0.6 x 0.58 hole (0.348) straddling the tile
    // line x = 20, drawn as four boxes that merge into the ring: NP.8b.
    let ring = |x: f64, y: f64, hw: f64, hh: f64| {
        vec![
            rect(h.np, x, y, x + hw + 1.6, y + 0.8),
            rect(h.np, x, y + 0.8 + hh, x + hw + 1.6, y + hh + 1.6),
            rect(h.np, x, y + 0.8, x + 0.8, y + 0.8 + hh),
            rect(h.np, x + 0.8 + hw, y + 0.8, x + hw + 1.6, y + 0.8 + hh),
        ]
    };
    let mut v = ring(10.0, 10.0, 0.7, 0.5);
    v.extend(ring(14.0, 10.0, 0.7, 0.495));
    v.extend(ring(19.1, 10.0, 0.6, 0.58));
    write("NP.8b.h1", v);

    // --- NP.7 and NP.10: the salicide block's neighbours.  NP.7 is a space of 0.18 to an
    // unsalicided poly, NP.10 an overlap of 0.18 of an unsalicided COMP.
    //
    // h1 - (a) a COMP under a block, the marker 0.175 past its left wall and 0.2 past the
    // rest: NP.10; (b) a COMP under a block that the marker's right wall cuts in half -
    // an overlap of nothing on that side; (c) a poly bar under a block whose left wall
    // the marker's right wall touches: a space of nothing, NP.7.
    write(
        "NP.10.h1",
        vec![
            rect(h.comp, 10.0, 10.0, 11.0, 11.0),
            rect(h.sab, 9.9, 9.9, 11.1, 11.1),
            rect(h.np, 9.825, 9.8, 11.2, 11.2),
            rect(h.comp, 15.0, 10.0, 16.0, 11.0),
            rect(h.sab, 14.9, 9.9, 16.1, 11.1),
            rect(h.np, 14.6, 9.8, 15.5, 11.2),
            rect(h.poly, 20.0, 10.0, 21.0, 11.0),
            rect(h.sab, 19.9, 9.9, 21.1, 11.1),
            rect(h.np, 18.6, 9.8, 20.0, 11.2),
        ],
    );

    // --- NP.4a: "Space to related P-channel gate at a butting edge parallel to gate -
    // 0.32".  The manual measures a butting edge that faces the gate; a butting edge
    // round the corner from it has no facing gate edge at all.
    //
    // h1 - a PMOS whose COMP has an arm going up on the far side of the gate.  The
    // butted N+/P+ edge is horizontal, at y = 12.7, x 11.2..12.28; the nearest gate edge
    // is the gate's left wall, x = 12.5, y 11.5..12.5.  The two do not face each other
    // (nothing projects), and the corner-to-corner distance is 0.297.
    write(
        "NP.4a.h1",
        vec![
            rect(h.nw, 10.7, 11.0, 15.5, 14.0),
            rect(h.comp, 11.0, 11.5, 15.0, 12.5),
            rect(h.comp, 11.2, 12.5, 12.28, 13.5),
            rect(h.poly, 12.5, 11.5, 12.78, 12.5),
            rect(h.pp, 10.7, 11.2, 15.3, 12.7),
            rect(h.np, 11.0, 12.7, 12.32, 13.8),
        ],
    );

    // h2 - the butting edge facing the gate, the bound.  (a) a vertical butted edge
    // 0.315 from the gate's right wall, the two fully overlapping: NP.4a; (b) the same
    // at 0.32: clean.
    let np4a_face = |y: f64, gap: f64| {
        let xb = 12.28 + gap;
        vec![
            rect(h.nw, 10.7, y, 16.5, y + 2.5),
            rect(h.comp, 11.0, y + 0.5, 16.0, y + 1.5),
            rect(h.poly, 12.0, y + 0.5, 12.28, y + 1.5),
            rect(h.pp, 10.7, y + 0.2, xb, y + 1.8),
            rect(h.np, xb, y + 0.2, 16.3, y + 1.8),
        ]
    };
    let mut v = np4a_face(11.0, 0.315);
    v.extend(np4a_face(16.0, 0.32));
    write("NP.4a.h2", v);

    // --- NP.5a: "Overlap of N-channel gate - 0.23".  An N-channel gate is derived from
    // the NCOMP, which is COMP under the marker, so a gate lies inside the marker by
    // construction: a marker edge that cuts a poly bar re-cuts the gate with it.
    //
    // h1 - (a) a gate the marker holds by 0.225 above and below: NP.5a; (b) a gate whose
    // poly bar the marker's right edge cuts in half - the overlap there is nothing, and
    // the COMP is not covered either.
    write(
        "NP.5a.h1",
        vec![
            rect(h.comp, 10.0, 10.0, 13.0, 11.0),
            rect(h.poly, 11.0, 9.7, 11.28, 11.3),
            rect(h.np, 9.7, 9.775, 13.3, 11.225),
            rect(h.comp, 20.0, 10.0, 23.0, 11.0),
            rect(h.poly, 21.0, 9.7, 21.28, 11.3),
            rect(h.np, 19.7, 9.7, 21.14, 11.3),
        ],
    );

    // --- NP.6: "Overlap with NCOMP butted to PCOMP - 0.22".  The COMP has to reach
    // 0.22 past the butting edge, which is the N+ half's length; the three walls the
    // PCOMP shares with the COMP are an overlap of nothing and are not the rule's.
    //
    // h1 - (a) an N+ half 0.215 long: NP.6; (b) 0.22: clean.
    let np6 = |x: f64, n_len: f64| {
        vec![
            rect(h.comp, x, 10.0, x + 1.2 + n_len, 11.0),
            rect(h.pp, x - 0.3, 9.7, x + 1.2, 11.3),
            rect(h.np, x + 1.2, 9.7, x + 1.5 + n_len, 11.3),
        ]
    };
    let mut v = np6(10.0, 0.215);
    v.extend(np6(15.0, 0.22));
    write("NP.6.h1", v);

    // --- NP.11: "Butting Nplus and PCOMP is forbidden within 0.43um of Nwell edge (for
    // outside DNWELL)".  The manual says within 0.43 of the edge; the deck bands only
    // the collar outside the well.
    //
    // h1 - three butted N+/P+ edges against a well, all vertical and all at the well's
    // right wall.  (a) 0.2 outside it, at the tile line x = 20; (b) 0.2 inside it;
    // (c) 0.5 outside it, clear of the 0.43 band, clean.
    write(
        "NP.11.h1",
        vec![
            rect(h.nw, 17.0, 10.0, 20.0, 14.0),
            rect(h.comp, 19.5, 11.0, 22.2, 12.0),
            rect(h.pp, 19.2, 10.8, 20.2, 12.2),
            rect(h.np, 20.2, 10.8, 22.5, 12.2),
            rect(h.nw, 25.0, 10.0, 30.0, 14.0),
            rect(h.comp, 27.0, 11.0, 31.0, 12.0),
            rect(h.pp, 26.0, 10.8, 29.8, 12.2),
            rect(h.np, 29.8, 10.8, 31.3, 12.2),
            rect(h.nw, 35.0, 10.0, 38.0, 14.0),
            rect(h.comp, 38.2, 11.0, 40.8, 12.0),
            rect(h.pp, 37.9, 10.8, 38.5, 12.2),
            rect(h.np, 38.5, 10.8, 41.1, 12.2),
        ],
    );

    // --- NP.12: "Overlap with P-channel poly2 gate extension is forbidden within 0.32um
    // of P-channel gate".  The reach runs along the poly, not across the air: a marker
    // on a poly leg 0.22 away as the crow flies but 4 um away along the poly is clear.
    //
    // h1 - (a) a U of poly whose left leg carries a P-channel gate and whose right leg is
    // 0.22 of air away from that gate but 5 um away along the poly; the marker covers the
    // right leg and reaches into the plain 0.32 disc around the gate.  A reach measured
    // across the air fires; one measured along the poly does not, and the manual's
    // "gate extension" is poly.  (b) a straight poly bar above a gate with the marker
    // starting 0.315 up it, one step inside the reach: NP.12.
    write(
        "NP.12.h1",
        vec![
            // (a) the U.
            rect(h.nw, 10.0, 10.0, 15.0, 14.5),
            rect(h.comp, 11.0, 11.2, 12.35, 11.8),
            rect(h.pp, 10.7, 10.9, 12.39, 12.1),
            rect(h.poly, 12.0, 11.0, 12.28, 14.0),
            rect(h.poly, 12.0, 13.72, 12.78, 14.0),
            rect(h.poly, 12.5, 11.0, 12.78, 14.0),
            rect(h.np, 12.55, 10.6, 14.0, 11.3),
            // (b) the straight bar: the gate's top is y = 11.8, the reach ends at 12.119.
            rect(h.nw, 24.0, 10.5, 28.0, 12.5),
            rect(h.comp, 25.0, 11.2, 27.0, 11.8),
            rect(h.pp, 24.7, 10.9, 27.3, 12.1),
            rect(h.poly, 26.0, 11.0, 26.28, 14.0),
            rect(h.np, 25.5, 12.115, 27.0, 13.0),
        ],
    );

    // --- NP.5b/NP.5d: "Extension beyond COMP" - 0.16 for a COMP outside the wells,
    // 0.02 for one deep inside an N-well.  The value is chosen by where the COMP and the
    // marker sit, and nothing else about them changes.
    //
    // h1 - two identical taps with the marker 0.1 past the COMP's left wall.  (a) in the
    // field: NP.5b wants 0.16, fires; (b) 2 um inside an N-well: NP.5dii wants 0.02,
    // clean.
    write(
        "NP.5b.h1",
        vec![
            rect(h.comp, 10.0, 10.0, 11.0, 11.0),
            rect(h.np, 9.9, 9.7, 11.3, 11.3),
            rect(h.nw, 15.0, 9.0, 20.0, 13.0),
            rect(h.comp, 17.0, 10.0, 18.0, 11.0),
            rect(h.np, 16.9, 9.7, 18.3, 11.3),
        ],
    );

    // h2 - the 0.429 band inside an N-well, which splits NP.5di (0.16) from NP.5dii
    // (0.02).  One 10 um well holding four taps: (a) 3 um in from every wall with the
    // marker 0.015 past the COMP: NP.5dii; (b) the same at 0.02: clean; (c) a tap whose
    // left wall is 0.2 inside the well - in the band - with the marker 0.155 past it:
    // NP.5di; (d) the same at 0.16: clean.
    write(
        "NP.5b.h2",
        vec![
            rect(h.nw, 10.0, 10.0, 20.0, 20.0),
            rect(h.comp, 13.0, 13.0, 14.0, 14.0),
            rect(h.np, 12.985, 12.7, 14.3, 14.3),
            rect(h.comp, 16.0, 13.0, 17.0, 14.0),
            rect(h.np, 15.98, 12.7, 17.3, 14.3),
            rect(h.comp, 10.2, 16.0, 11.2, 17.0),
            rect(h.np, 10.045, 15.7, 11.5, 17.3),
            rect(h.comp, 10.2, 18.0, 11.2, 19.0),
            rect(h.np, 10.04, 17.7, 11.5, 19.3),
        ],
    );

    // --- NP.9: "Overlap of unsalicided Poly2 - 0.18".  The deck exempts a marked poly
    // resistor from PP.9 and not from NP.9; the manual's two sections carry the same
    // sentence.
    //
    // h1 - two poly bars under a salicide block with the marker 0.175 past the left wall
    // of each.  (a) marked as a resistor - a resistor's body is meant to be bare, so the
    // implant's edges there are the device's, not the rule's; (b) bare: NP.9.
    write(
        "NP.9.h1",
        vec![
            rect(h.poly, 10.0, 10.0, 12.0, 10.5),
            rect(h.sab, 9.8, 9.8, 12.2, 10.7),
            rect(h.res, 9.9, 9.9, 12.1, 10.6),
            rect(h.np, 9.825, 9.8, 12.2, 10.7),
            rect(h.poly, 20.0, 10.0, 22.0, 10.5),
            rect(h.sab, 19.8, 9.8, 22.2, 10.7),
            rect(h.np, 19.825, 9.8, 22.2, 10.7),
        ],
    );
}
