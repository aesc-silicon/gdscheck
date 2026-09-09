// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Low-voltage P-well: a good and a bad pattern for every rule in the `lvpwell` deck.
//!
//! The well lives inside a deep well and carries a P+ tap up to Metal1, which is what
//! gives it a net - LPW.2a bounds the distance between wells at *different* potentials
//! and LPW.2b the distance between any two, so the pair fixtures differ only in whether
//! one Metal1 plate covers both taps or two plates cover one each.
//!
//! Everything is drawn twice, once at each voltage: the deck splits every width and
//! spacing rule between a well Dualgate does not touch and one it overlaps.  A fixture
//! for one voltage is silent on the other's rule because the marker decides which layer
//! the well lands in.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/lvpwell";

/// The well, over LPW.1's 0.6 µm at low voltage and 0.74 at medium.
const PW: f64 = 2.0;
const PW_H: f64 = 3.0;
/// How far the deep well holds it - LPW.3 asks 2.5.
const DN_ENC: f64 = 3.0;
/// The P+ tap and its contact.
const TAP: f64 = 1.0;
const CO: f64 = 0.22;

struct Ctx {
    lvpwell: (i16, i16),
    dnwell: (i16, i16),
    nwell: (i16, i16),
    dualgate: (i16, i16),
    comp: (i16, i16),
    pplus: (i16, i16),
    res_mk: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
}

/// One well with its tap, and where the tap's contact sits.
fn well(c: &Ctx, x: f64, y: f64, w: f64) -> (Vec<GdsElement>, (f64, f64)) {
    let (cx, cy) = (x + w * 0.5, y + PW_H * 0.5);
    let h = (TAP * 0.5).min(w * 0.4);
    let v = vec![
        rect(c.lvpwell, x, y, x + w, y + PW_H),
        rect(c.comp, cx - h, cy - h, cx + h, cy + h),
        rect(
            c.pplus,
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
        lvpwell: layer(pdk, "lvpwell"),
        dnwell: layer(pdk, "dnwell"),
        nwell: layer(pdk, "nwell"),
        dualgate: layer(pdk, "dualgate"),
        comp: layer(pdk, "comp"),
        pplus: layer(pdk, "pplus"),
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

    // One or two wells in a deep well, at either voltage, on one net or two.
    let scene = |w: f64, gap: Option<f64>, mv: bool, same_net: bool, dn_enc: f64| {
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
        v.push(rect(
            c.dnwell,
            o - dn_enc,
            o - dn_enc,
            right + dn_enc,
            o + PW_H + dn_enc,
        ));
        if mv {
            // Over the wells, which is what makes them the medium-voltage kind.
            v.push(rect(
                c.dualgate,
                o - 0.5,
                o - 0.5,
                right + 0.5,
                o + PW_H + 0.5,
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

    for (id, mv) in [
        ("LPW.1_LV", false),
        ("LPW.2a_LV", false),
        ("LPW.2b_LV", false),
        ("LPW.3", false),
        ("LPW.5", false),
        ("LPW.11", false),
        ("LPW.12", false),
        ("LPW.1_MV", true),
        ("LPW.2a_MV", true),
        ("LPW.2b_MV", true),
    ] {
        write(id, "good", scene(PW, None, mv, true, DN_ENC));
    }

    // LPW.1: a well narrower than the voltage allows.
    write("LPW.1_LV", "bad", scene(0.595, None, false, true, DN_ENC));
    write("LPW.1_MV", "bad", scene(0.735, None, true, true, DN_ENC));

    // LPW.2a: two wells at different potentials, closer than the limit.  LPW.2b measures
    // any two at 0.86, so these stay well outside that.
    write(
        "LPW.2a_LV",
        "bad",
        scene(PW, Some(1.395), false, false, DN_ENC),
    );
    write(
        "LPW.2a_MV",
        "bad",
        scene(PW, Some(1.695), true, false, DN_ENC),
    );

    // LPW.2b: two wells at one potential, closer than 0.86.  On one net, so the
    // different-potential rule has nothing to say.
    write(
        "LPW.2b_LV",
        "bad",
        scene(PW, Some(0.855), false, true, DN_ENC),
    );
    write(
        "LPW.2b_MV",
        "bad",
        scene(PW, Some(0.855), true, true, DN_ENC),
    );

    // LPW.3: the deep well holding the well by under 2.5 µm.
    write("LPW.3", "bad", scene(PW, None, false, true, 2.495));

    // LPW.5: a well marked as a resistor outside any deep well.  RES_MK over a well with
    // a P+ active on it is what makes it one.
    write("LPW.5", "bad", {
        let mut v = scene(PW, None, false, true, DN_ENC);
        let x = o + PW + DN_ENC + 4.0;
        v.push(rect(c.lvpwell, x, o, x + PW, o + PW_H));
        v.push(rect(c.comp, x + 0.5, o + 0.5, x + PW - 0.5, o + PW_H - 0.5));
        v.push(rect(
            c.pplus,
            x + 0.4,
            o + 0.4,
            x + PW - 0.4,
            o + PW_H - 0.4,
        ));
        v.push(rect(
            c.res_mk,
            x - 0.2,
            o - 0.2,
            x + PW + 0.2,
            o + PW_H + 0.2,
        ));
        v
    });

    // LPW.11: a well outside the deep well, within 1.5 µm of it.
    write("LPW.11", "bad", {
        let mut v = scene(PW, None, false, true, DN_ENC);
        let x = o + PW + DN_ENC + 1.495;
        v.push(rect(c.lvpwell, x, o, x + PW, o + PW_H));
        v
    });

    // LPW.12: a well outside the deep well, over an N-well.
    write("LPW.12", "bad", {
        let mut v = scene(PW, None, false, true, DN_ENC);
        let x = o + PW + DN_ENC + 4.0;
        v.push(rect(c.lvpwell, x, o, x + PW, o + PW_H));
        v.push(rect(
            c.nwell,
            x + 0.5,
            o + 0.5,
            x + PW + 0.5,
            o + PW_H - 0.5,
        ));
        v
    });
}
