// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! N-well: a good and a bad pattern for every rule in the `nwell` deck.
//!
//! The well takes an N+ tap up to Metal1 for its net, and is drawn at both voltages: the
//! deck splits width and spacing between a well Dualgate does not touch and one it
//! overlaps.  NW.2a bounds the distance between any two wells and NW.2b the distance
//! between wells at different potentials, so the pair fixtures differ in whether one
//! Metal1 plate covers both taps or two cover one each, and each sits outside the other
//! rule's limit.
//!
//! Two rules need a well that is also a resistor, which takes a RES_MK marker overhanging
//! the well: NW.1b widens the minimum to 2 µm for one outside a deep well, NW.6 forbids
//! one inside a deep well entirely.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/nwell";

/// The well, over NW.1a's 0.86 µm and NW.1b's 2.0.
const NW: f64 = 3.0;
/// The N+ tap that takes the well up to Metal1, and the contact in it.
const TAP: f64 = 1.0;
const CO: f64 = 0.22;

struct Ctx {
    nwell: (i16, i16),
    dnwell: (i16, i16),
    lvpwell: (i16, i16),
    dualgate: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    res_mk: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
}

/// One well with its tap, and where the tap's contact sits.
fn well(c: &Ctx, x: f64, y: f64, w: f64) -> (Vec<GdsElement>, (f64, f64)) {
    let (cx, cy) = (x + w * 0.5, y + NW * 0.5);
    let h = (TAP * 0.5).min(w * 0.4);
    let v = vec![
        rect(c.nwell, x, y, x + w, y + NW),
        rect(c.comp, cx - h, cy - h, cx + h, cy + h),
        rect(
            c.nplus,
            cx - h - 0.1,
            cy - h - 0.1,
            cx + h + 0.1,
            cy + h + 0.1,
        ),
        rect(
            c.contact,
            cx - CO * 0.5,
            cy - CO * 0.5,
            cx + CO * 0.5,
            cy + CO * 0.5,
        ),
    ];
    (v, (cx, cy))
}

fn strap(c: &Ctx, centres: &[(f64, f64)]) -> GdsElement {
    let m = CO * 0.5 + 0.2;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in centres {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    rect(c.metal1, x0 - m, y0 - m, x1 + m, y1 + m)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
        lvpwell: layer(pdk, "lvpwell"),
        dualgate: layer(pdk, "dualgate"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        res_mk: layer(pdk, "res_mk"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // One or two wells, at either voltage, on one net or two.
    let scene = |w: f64, gap: Option<f64>, mv: bool, same_net: bool| {
        let (mut v, p1) = well(&c, o, o, w);
        let mut pts = vec![p1];
        let mut right = o + w;
        if let Some(g) = gap {
            let x2 = o + w + g;
            let (w2, p2) = well(&c, x2, o, w);
            v.extend(w2);
            pts.push(p2);
            right = x2 + w;
        }
        if mv {
            // Over the wells, which is what makes them the medium-voltage kind.
            v.push(rect(
                c.dualgate,
                o - 0.5,
                o - 0.5,
                right + 0.5,
                o + NW + 0.5,
            ));
        }
        if same_net {
            v.push(strap(&c, &pts));
        } else {
            for p in &pts {
                v.push(strap(&c, &[*p]));
            }
        }
        v
    };
    // A well marked as a resistor: the marker has to overhang the well.
    let resistor = |w: f64, mv: bool| {
        let mut v = scene(w, None, mv, true);
        v.push(rect(c.res_mk, o - 0.3, o - 0.3, o + w + 0.3, o + NW + 0.3));
        v
    };

    for (id, mv) in [
        ("NW.1a_LV", false),
        ("NW.2a_LV", false),
        ("NW.2b_LV", false),
        ("NW.3", false),
        ("NW.4", false),
        ("NW.6", false),
        ("NW.1a_MV", true),
        ("NW.2a_MV", true),
        ("NW.2b_MV", true),
    ] {
        write(id, "good", scene(NW, None, mv, true));
    }
    for (id, mv) in [("NW.1b_LV", false), ("NW.1b_MV", true)] {
        write(id, "good", resistor(NW, mv));
    }
    // NW.5's clean half is a well the deep well holds by more than 0.5 µm.
    let held = |mv: bool, enc: f64| {
        let mut v = scene(NW, None, mv, true);
        v.push(rect(c.dnwell, o - enc, o - enc, o + NW + enc, o + NW + enc));
        v
    };
    write("NW.5_LV", "good", held(false, 1.0));
    write("NW.5_MV", "good", held(true, 1.0));

    // NW.1a: a well narrower than 0.86 µm.
    write("NW.1a_LV", "bad", scene(0.855, None, false, true));
    write("NW.1a_MV", "bad", scene(0.855, None, true, true));

    // NW.1b: a resistor well narrower than 2 µm - legal for a plain well, not for one
    // under a resistor marker.
    write("NW.1b_LV", "bad", resistor(1.995, false));
    write("NW.1b_MV", "bad", resistor(1.995, true));

    // NW.2a: two wells closer than the voltage allows, on one net so the
    // different-potential rule has nothing to say.
    write("NW.2a_LV", "bad", scene(NW, Some(0.595), false, true));
    write("NW.2a_MV", "bad", scene(NW, Some(0.735), true, true));

    // NW.2b: two wells at different potentials, outside NW.2a's reach but inside this.
    write("NW.2b_LV", "bad", scene(NW, Some(1.395), false, false));
    write("NW.2b_MV", "bad", scene(NW, Some(1.695), true, false));

    // NW.3: a well 3.1 µm from a deep well.  Far enough not to interact with it, so the
    // enclosure rule does not reach.
    write("NW.3", "bad", {
        let mut v = scene(NW, None, false, true);
        let x = o + NW + 3.095;
        v.push(rect(c.dnwell, x, o, x + NW, o + NW));
        v
    });

    // NW.4: a well over a P-well.
    write("NW.4", "bad", {
        let mut v = scene(NW, None, false, true);
        v.push(rect(
            c.lvpwell,
            o + 1.0,
            o + 1.0,
            o + NW + 2.0,
            o + NW - 1.0,
        ));
        v
    });

    // NW.5: a deep well holding a well it touches by under 0.5 µm.
    write("NW.5_LV", "bad", held(false, 0.495));
    write("NW.5_MV", "bad", held(true, 0.495));

    // NW.6: a resistor well inside a deep well, which is not allowed at any width.
    write("NW.6", "bad", {
        let mut v = held(false, 1.0);
        v.push(rect(c.res_mk, o - 0.3, o - 0.3, o + NW + 0.3, o + NW + 0.3));
        v
    });
}
