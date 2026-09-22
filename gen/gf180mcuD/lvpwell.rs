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
use crate::helpers::{chamfered_tr, layer, library, poly, rect, write_gz};
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
    hardening(pdk);
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

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.3 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/lvpwell/LPW.<rule>.h<n>.gds.gz` with a case in the
// `hardening_lvpwell` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/lvpwell.md.
//
// The deck's own conditions: whether the well lies in a deep well (table A) or not
// (table B), whether Dualgate lies over it, which diffusion ties it to Metal1, and the
// RES_MK marker that makes it a resistor.

/// Layers the hardening patterns draw on.
struct L {
    pw: (i16, i16),
    dn: (i16, i16),
    nw: (i16, i16),
    dg: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    res: (i16, i16),
    co: (i16, i16),
    m1: (i16, i16),
    via1: (i16, i16),
    m2: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            pw: layer(pdk, "lvpwell"),
            dn: layer(pdk, "dnwell"),
            nw: layer(pdk, "nwell"),
            dg: layer(pdk, "dualgate"),
            comp: layer(pdk, "comp"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            res: layer(pdk, "res_mk"),
            co: layer(pdk, "contact"),
            m1: layer(pdk, "metal1_drawn"),
            via1: layer(pdk, "via1"),
            m2: layer(pdk, "metal2_drawn"),
        }
    }

    /// A contacted diffusion `size` on a side centred on `(cx, cy)`, N+ or P+; the
    /// contact's centre comes back for the strap.
    fn diff(
        &self,
        cx: f64,
        cy: f64,
        size: f64,
        implant: (i16, i16),
    ) -> (Vec<GdsElement>, (f64, f64)) {
        let h = size * 0.5;
        (
            vec![
                rect(self.comp, cx - h, cy - h, cx + h, cy + h),
                rect(
                    implant,
                    cx - h - 0.1,
                    cy - h - 0.1,
                    cx + h + 0.1,
                    cy + h + 0.1,
                ),
                rect(self.co, cx - 0.11, cy - 0.11, cx + 0.11, cy + 0.11),
            ],
            (cx, cy),
        )
    }

    /// A P-well `(x0, y0)-(x1, y1)` with a 1 µm P+ tap at its centre.
    fn well(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (Vec<GdsElement>, (f64, f64)) {
        let (mut v, p) = self.diff((x0 + x1) * 0.5, (y0 + y1) * 0.5, 1.0, self.pplus);
        v.push(rect(self.pw, x0, y0, x1, y1));
        (v, p)
    }

    /// One Metal1 plate over the contact centres: one net.
    fn strap(&self, pts: &[(f64, f64)]) -> GdsElement {
        let m = 0.11 + 0.2;
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        rect(self.m1, x0 - m, y0 - m, x1 + m, y1 + m)
    }

    /// Two 3 × 3 wells `gap` apart, the left one at `(x, y)`, on one net or two; `dg`
    /// covers both with 0.5 to spare when set.
    fn pair(&self, x: f64, y: f64, gap: f64, same_net: bool, dg: bool) -> Vec<GdsElement> {
        let (mut v, a) = self.well(x, y, x + 3.0, y + 3.0);
        let (w, b) = self.well(x + 3.0 + gap, y, x + 6.0 + gap, y + 3.0);
        v.extend(w);
        if same_net {
            v.push(self.strap(&[a, b]));
        } else {
            v.push(self.strap(&[a]));
            v.push(self.strap(&[b]));
        }
        if dg {
            v.push(rect(self.dg, x - 0.5, y - 0.5, x + 6.5 + gap, y + 3.5));
        }
        v
    }

    /// A 3 × 3 well at `(x, y)` with a slot `d` wide and 1.5 deep cut into its top,
    /// tapped once in its left arm.
    fn notched(&self, x: f64, y: f64, d: f64) -> Vec<GdsElement> {
        let (mut v, p) = self.diff(x + 0.6, y + 0.6, 0.6, self.pplus);
        v.push(self.strap(&[p]));
        v.push(poly(
            self.pw,
            &[
                (x, y),
                (x + 3.0, y),
                (x + 3.0, y + 3.0),
                (x + 1.5 + d * 0.5, y + 3.0),
                (x + 1.5 + d * 0.5, y + 1.5),
                (x + 1.5 - d * 0.5, y + 1.5),
                (x + 1.5 - d * 0.5, y + 3.0),
                (x, y + 3.0),
            ],
        ));
        v
    }

    /// A P-well resistor as LPW.4 describes it: a P+ COMP head at each end of the
    /// well `(x0, y0)-(x1, y1)` and RES_MK from head to head, covering the width with
    /// 0.3 to spare.
    fn resistor(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        let (mut v, _) = self.diff((x0 + x1) * 0.5, y0 + 0.5, 0.6, self.pplus);
        v.extend(self.diff((x0 + x1) * 0.5, y1 - 0.5, 0.6, self.pplus).0);
        v.push(rect(self.pw, x0, y0, x1, y1));
        v.push(rect(self.res, x0 - 0.3, y0 + 1.0, x1 + 0.3, y1 - 1.0));
        v
    }
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    lpw_1_h(&l);
    lpw_2a_h(&l);
    lpw_2b_h(&l);
    lpw_2_h(&l);
    lpw_3_h(&l);
    lpw_5_h(&l);
    lpw_11_h(&l);
    lpw_12_h(&l);
}

// --- LPW.1: min. LVPWELL width 0.6 LV / 0.74 MV, inside DNWELL ---

fn lpw_1_h(l: &L) {
    // h1 - which well is the 5 V kind.  Six bare wells 3 µm tall in one deep well
    // (holding everything by 3), 4 µm apart: (a) 0.595 with no marker near it,
    // LPW.1_LV; (b) 0.735 under Dualgate with 0.5 to spare, LPW.1_MV; (c) 0.74 under
    // Dualgate, clean; (d) 0.6 bare, clean; (e) 0.735 with Dualgate over its top half
    // only, LPW.1_MV; (f) 0.7 with Dualgate abutting its right edge and not over it -
    // a 3.3 V well, whose width is over 0.6, clean.
    write(
        "LPW.1.h1",
        vec![
            rect(l.dn, -1.0, -1.0, 27.0, 8.0),
            rect(l.pw, 2.0, 2.0, 2.595, 5.0), // (a) LPW.1_LV
            rect(l.pw, 6.0, 2.0, 6.735, 5.0), // (b) LPW.1_MV
            rect(l.dg, 5.5, 1.5, 7.235, 5.5),
            rect(l.pw, 10.0, 2.0, 10.74, 5.0), // (c) clean
            rect(l.dg, 9.5, 1.5, 11.24, 5.5),
            rect(l.pw, 14.0, 2.0, 14.6, 5.0),   // (d) clean
            rect(l.pw, 18.0, 2.0, 18.735, 5.0), // (e) LPW.1_MV
            rect(l.dg, 17.5, 3.5, 19.235, 5.5),
            rect(l.pw, 22.0, 2.0, 22.7, 5.0), // (f) clean
            rect(l.dg, 22.7, 1.5, 24.0, 5.5),
        ],
    );

    // h2 - outside any deep well.  Table B has no width rule: a 0.595 × 3 well with no
    // deep well anywhere, and one 5 µm from a deep well, are clean.
    write(
        "LPW.1.h2",
        vec![
            rect(l.pw, 2.0, 2.0, 2.595, 5.0),
            rect(l.pw, 12.0, 2.0, 12.595, 5.0),
            rect(l.dn, 17.595, 2.0, 22.0, 5.0),
        ],
    );

    // h3 - a 5 V well under both bounds.  0.595 wide under Dualgate in a deep well: it
    // is the 5 V kind, so LPW.1_MV, and LPW.1_LV has nothing to say about it.
    write(
        "LPW.1.h3",
        vec![
            rect(l.dn, -1.0, -1.0, 7.0, 8.0),
            rect(l.pw, 2.0, 2.0, 2.595, 5.0), // LPW.1_MV
            rect(l.dg, 1.5, 1.5, 3.095, 5.5),
        ],
    );

    // h4 - the deep well at the far end of a long well.  A 0.595 well from x = 5 to
    // x = 35, its last 5 µm inside a deep well from x = 30 to x = 40 (which holds it by
    // 3 above and below): the well is inside a deep well and crosses its edge, so
    // LPW.1_LV once along the well (two walls) and LPW.3 for the crossing, at every
    // tile size.
    write(
        "LPW.1.h4",
        vec![
            rect(l.pw, 5.0, 2.0, 35.0, 2.595), // LPW.1_LV, LPW.3
            rect(l.dn, 30.0, -1.0, 40.0, 5.595),
        ],
    );
}

// --- LPW.2a: min. LVPWELL space, different potential, 1.4 LV / 1.7 MV ---

fn lpw_2a_h(l: &L) {
    // h1 - what makes two wells one potential, in one deep well.  Two taps, two Metal1
    // plates: (a) at 1.395, LPW.2a_LV; (b) at 1.4, clean; (c) at 1.395 with the
    // plates joined by Via1 and a Metal2 strap, one net, clean; (d) at 1.395 with the
    // left plate running over the right well without touching its tap, two nets,
    // LPW.2a_LV; (e) at 1.395 with one plate over both contacts but the right well's
    // diffusion N+ - an NMOS terminal, not a tap - two nets, LPW.2a_LV.
    let mut v = vec![rect(l.dn, -1.0, -1.0, 37.0, 16.0)];
    v.extend(l.pair(2.0, 2.0, 1.395, false, false)); // (a) LPW.2a_LV
    v.extend(l.pair(14.0, 2.0, 1.4, false, false)); // (b) clean
    v.extend(l.pair(26.0, 2.0, 1.395, false, false)); // (c) clean
    for cx in [27.5, 31.895] {
        v.push(rect(l.via1, cx - 0.13, 3.37, cx + 0.13, 3.63));
    }
    v.push(rect(l.m2, 27.1, 3.1, 32.295, 3.9));
    v.extend(l.pair(2.0, 10.0, 1.395, false, false)); // (d) LPW.2a_LV
    v.push(rect(l.m1, 3.19, 10.0, 8.0, 10.5));
    {
        // (e) LPW.2a_LV
        let (w, a) = l.well(14.0, 10.0, 17.0, 13.0);
        v.extend(w);
        let bx = 18.395;
        v.push(rect(l.pw, bx, 10.0, bx + 3.0, 13.0));
        let (d, b) = l.diff(bx + 1.5, 11.5, 1.0, l.nplus);
        v.extend(d);
        v.push(l.strap(&[a, b]));
    }
    write("LPW.2a.h1", v);

    // h2 - the MV bound.  Two nets under Dualgate at 1.695, LPW.2a_MV; at 1.7, clean.
    write(
        "LPW.2a.h2",
        [
            vec![rect(l.dn, -1.0, -1.0, 25.0, 8.0)],
            l.pair(2.0, 2.0, 1.695, false, true), // LPW.2a_MV
            l.pair(14.0, 2.0, 1.7, false, true),  // clean
        ]
        .concat(),
    );

    // h3 - a pair of mixed voltage.  Two nets 1.695 apart with Dualgate over the left
    // well only, reaching 0.3 into the gap: the left well is the 5 V kind and its
    // space to any other well is 1.7, LPW.2a_MV.  A second such pair at 1.7, clean.
    write(
        "LPW.2a.h3",
        [
            vec![rect(l.dn, -1.0, -1.0, 25.0, 8.0)],
            l.pair(2.0, 2.0, 1.695, false, false), // LPW.2a_MV
            vec![rect(l.dg, 1.5, 1.5, 5.3, 5.5)],
            l.pair(14.0, 2.0, 1.7, false, false), // clean
            vec![rect(l.dg, 13.5, 1.5, 17.3, 5.5)],
        ]
        .concat(),
    );

    // h4 - the marker beside the well.  Two nets at 1.395 with Dualgate abutting the
    // left well's left edge, not over either well: both are the 3.3 V kind, LPW.2a_LV.
    write(
        "LPW.2a.h4",
        [
            vec![rect(l.dn, -1.0, -1.0, 15.0, 8.0)],
            l.pair(4.0, 2.0, 1.395, false, false), // LPW.2a_LV
            vec![rect(l.dg, 2.0, 1.5, 4.0, 5.5)],
        ]
        .concat(),
    );
}

// --- LPW.2b: min. LVPWELL space, equi-potential, 0.86 ---

fn lpw_2b_h(l: &L) {
    // h1 - the LV bound, in one deep well.  One net (a tap each, one plate) at 0.855,
    // LPW.2b_LV; at 0.86, clean; a slot 0.855 wide into one well, LPW.2b_LV; at
    // 0.86, clean.
    write(
        "LPW.2b.h1",
        [
            vec![rect(l.dn, -1.0, -1.0, 25.0, 16.0)],
            l.pair(2.0, 2.0, 0.855, true, false), // LPW.2b_LV
            l.pair(14.0, 2.0, 0.86, true, false), // clean
            l.notched(2.0, 10.0, 0.855),          // LPW.2b_LV
            l.notched(14.0, 10.0, 0.86),          // clean
        ]
        .concat(),
    );

    // h2 - the MV bound: the same under Dualgate.  0.855, LPW.2b_MV; 0.86, clean; a
    // 0.855 slot, LPW.2b_MV.
    write(
        "LPW.2b.h2",
        [
            vec![rect(l.dn, -1.0, -1.0, 25.0, 16.0)],
            l.pair(2.0, 2.0, 0.855, true, true), // LPW.2b_MV
            l.pair(14.0, 2.0, 0.86, true, true), // clean
            l.notched(2.0, 10.0, 0.855),         // LPW.2b_MV
            vec![rect(l.dg, 1.5, 9.5, 5.5, 13.5)],
        ]
        .concat(),
    );
}

// --- LPW.2a/LPW.2b: where the space rules do not reach, and the tile lines ---

fn lpw_2_h(l: &L) {
    // h5 - outside any deep well, where table B has no space rule and every well's
    // body is the substrate: two nets at 0.855, a 0.855 slot, two nets at 1.395 - all
    // clean.
    write(
        "LPW.2.h5",
        [
            l.pair(2.0, 2.0, 0.855, false, false),
            l.notched(14.0, 2.0, 0.855),
            l.pair(2.0, 10.0, 1.395, false, false),
        ]
        .concat(),
    );

    // h6 - the tile lines, in one deep well.  Two-net pairs at 1.395 with the gap
    // straddling x = 20 and x = 42, LPW.2a_LV twice; one-net pairs at 0.855 with the
    // strap crossing x = 40 and x = 21, LPW.2b_LV twice.
    write(
        "LPW.2.h6",
        [
            vec![rect(l.dn, 10.0, -1.0, 52.0, 16.0)],
            l.pair(16.3, 2.0, 1.395, false, false), // gap 19.3-20.695: LPW.2a_LV
            l.pair(38.3, 2.0, 1.395, false, false), // gap 41.3-42.695: LPW.2a_LV
            l.pair(36.5, 10.0, 0.855, true, false), // gap 39.5-40.355: LPW.2b_LV
            l.pair(17.5, 10.0, 0.855, true, false), // gap 20.5-21.355: LPW.2b_LV
        ]
        .concat(),
    );

    // h7 - two wells with no tap at all, 1.395 apart in one deep well: nothing ties
    // them to one potential, so they may sit at two, LPW.2a_LV.
    write(
        "LPW.2.h7",
        vec![
            rect(l.dn, -1.0, -1.0, 12.5, 8.0),
            rect(l.pw, 2.0, 2.0, 5.0, 5.0),
            rect(l.pw, 6.395, 2.0, 9.395, 5.0),
        ],
    );
}

// --- LPW.3: min. DNWELL enclosure of LVPWELL 2.5 ---

fn lpw_3_h(l: &L) {
    // A 2 × 2 well at (x, y) and a deep well around it with margins l, b, r, t.
    let held = |x: f64, y: f64, ml: f64, mb: f64, mr: f64, mt: f64| {
        vec![
            rect(l.pw, x, y, x + 2.0, y + 2.0),
            rect(l.dn, x - ml, y - mb, x + 2.0 + mr, y + 2.0 + mt),
        ]
    };
    // Held by 2.5 with the deep well's top-right corner chamfered `cut` each way.
    let chamfered = |x: f64, y: f64, cut: f64| {
        let (x1, y1) = (x + 4.5, y + 4.5);
        vec![
            rect(l.pw, x, y, x + 2.0, y + 2.0),
            chamfered_tr(l.dn, x - 2.5, y - 2.5, x1, y1, x1 + y1 - cut),
        ]
    };
    // h1 - the bound and the corner.  (a) held by 2.5 all round, clean; (b) 2.495 on
    // the right, LPW.3; (c) the well's right edge on the deep well's, LPW.3; (d)
    // chamfered 1.5 each way, the well's corner 2.475 from the chamfer, LPW.3 -
    // enclosure is the closest approach; (e) chamfered 1.4 each way, 2.546, clean.
    write(
        "LPW.3.h1",
        [
            held(3.0, 3.0, 2.5, 2.5, 2.5, 2.5),    // (a) clean
            held(17.0, 3.0, 2.5, 2.5, 2.495, 2.5), // (b) LPW.3
            held(31.0, 3.0, 2.5, 2.5, 0.0, 2.5),   // (c) LPW.3
            chamfered(3.0, 17.0, 1.5),             // (d) LPW.3
            chamfered(17.0, 17.0, 1.4),            // (e) clean
        ]
        .concat(),
    );

    // h2 - a well crossing out of the deep well.  A 2 × 2 well half in, half out of a
    // deep well that holds it by 2.5 everywhere else: there is no margin to measure on
    // the crossing side and the well is not enclosed, LPW.3.
    write(
        "LPW.3.h2",
        vec![
            rect(l.pw, 3.0, 3.0, 5.0, 5.0), // LPW.3
            rect(l.dn, 0.5, 0.5, 4.0, 7.5),
        ],
    );
}

// --- LPW.5: LVPWELL resistors must be enclosed by DNWELL ---

fn lpw_5_h(l: &L) {
    // h1 - (a) a 2 × 8 resistor (P+ heads, RES_MK between them) with no deep well,
    // LPW.5; (b) the same in a deep well holding it by 3, clean; (c) a 2 × 8 well
    // under RES_MK with no COMP at all - not a resistor - clean.
    write(
        "LPW.5.h1",
        [
            l.resistor(2.0, 2.0, 4.0, 10.0),   // (a) LPW.5
            l.resistor(14.0, 2.0, 16.0, 10.0), // (b) clean
            vec![rect(l.dn, 11.0, -1.0, 19.0, 13.0)],
            vec![
                rect(l.pw, 30.0, 2.0, 32.0, 10.0), // (c) clean
                rect(l.res, 29.7, 3.0, 32.3, 9.0),
            ],
        ]
        .concat(),
    );

    // h2 - a 5 V resistor with no deep well: the same 2 × 8 resistor under Dualgate.
    // The rule has no voltage column, so LPW.5.
    write(
        "LPW.5.h2",
        [
            l.resistor(2.0, 2.0, 4.0, 10.0), // LPW.5
            vec![rect(l.dg, 1.0, 1.0, 5.0, 11.0)],
        ]
        .concat(),
    );
}

// --- LPW.11: min. (LVPWELL outside DNWELL) space to DNWELL 1.5 ---

fn lpw_11_h(l: &L) {
    // h1 - (a) a 2 × 3 well 1.5 from a deep well, clean; (b) 1.495, LPW.11; (c)
    // abutting it, LPW.11; (d) corner to corner 1.06 each way, 1.499 on the diagonal,
    // LPW.11; (e) 1.065 each way, 1.506, clean; (f) a well in the hole of a DNWELL
    // ring, 1.495 from the ring's inner wall, LPW.11.
    let mut v = vec![
        rect(l.pw, 2.0, 2.0, 4.0, 5.0), // (a) clean
        rect(l.dn, 5.5, 2.0, 9.0, 5.0),
        rect(l.pw, 2.0, 12.0, 4.0, 15.0), // (b) LPW.11
        rect(l.dn, 5.495, 12.0, 9.0, 15.0),
        rect(l.pw, 2.0, 22.0, 4.0, 25.0), // (c) LPW.11
        rect(l.dn, 4.0, 22.0, 8.0, 25.0),
        rect(l.pw, 22.0, 2.0, 24.0, 5.0), // (d) LPW.11
        rect(l.dn, 25.06, 6.06, 29.0, 10.0),
        rect(l.pw, 22.0, 14.0, 24.0, 17.0), // (e) clean
        rect(l.dn, 25.065, 18.065, 29.0, 22.0),
    ];
    // (f) LPW.11: ring 2 wide, hole 5.99 square, well 3 × 3 at its centre
    let (x0, y0) = (40.0, 2.0);
    let (ix0, iy0, ix1, iy1) = (x0 + 2.0, y0 + 2.0, x0 + 7.99, y0 + 7.99);
    v.push(rect(l.dn, x0, y0, ix1 + 2.0, iy0));
    v.push(rect(l.dn, x0, iy1, ix1 + 2.0, iy1 + 2.0));
    v.push(rect(l.dn, x0, iy0, ix0, iy1));
    v.push(rect(l.dn, ix1, iy0, ix1 + 2.0, iy1));
    v.push(rect(
        l.pw,
        ix0 + 1.495,
        iy0 + 1.495,
        ix0 + 4.495,
        iy0 + 4.495,
    ));
    write("LPW.11.h1", v);

    // h2 - a narrow well abutting a deep well from outside.  A 0.595 × 3 well whose
    // right edge lies on the deep well's left edge: it is outside the deep well, so
    // table B applies - LPW.11 at a space of nothing - and table A's width rule does
    // not.
    write(
        "LPW.11.h2",
        vec![
            rect(l.pw, 2.0, 2.0, 2.595, 5.0), // LPW.11
            rect(l.dn, 2.595, 2.0, 7.0, 5.0),
        ],
    );
}

// --- LPW.12: LVPWELL cannot overlap Nwell ---

fn lpw_12_h(l: &L) {
    // h1 - (a) a well with no deep well overlapping an N-well by 0.005, LPW.12; (b)
    // abutting one, clean; (c) a well in a deep well (held by 3) overlapping an N-well
    // - table A's business (NW.4), not table B's, clean here.
    write(
        "LPW.12.h1",
        vec![
            rect(l.pw, 2.0, 2.0, 5.0, 5.0), // (a) LPW.12
            rect(l.nw, 4.995, 2.0, 8.0, 5.0),
            rect(l.pw, 14.0, 2.0, 17.0, 5.0), // (b) clean
            rect(l.nw, 17.0, 2.0, 20.0, 5.0),
            rect(l.pw, 29.0, 2.0, 32.0, 5.0), // (c) clean
            rect(l.nw, 31.995, 2.0, 35.0, 5.0),
            rect(l.dn, 26.0, -1.0, 38.0, 8.0),
        ],
    );
}
