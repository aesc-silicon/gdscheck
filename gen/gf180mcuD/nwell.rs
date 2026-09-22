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
use crate::helpers::{chamfered_tr, layer, library, poly, rect, write_gz};
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
    hardening(pdk);
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

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.4 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/nwell/NW.<rule>.h<n>.gds.gz` with a case in the
// `hardening_nwell` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/nwell.md.
//
// What these draw is the deck's own conditions - the Dualgate marker that makes a well
// the 5 V kind, the RES_MK marker that makes it a resistor, the deep well it may or may
// not lie in, and the tap-and-Metal1 strap that decides whether two wells are one
// potential - at the bound and one grid step past it.  The generic classes (the bound
// on a bare layer, 45°, unions, arrays) are the engine family's.

/// Layers the hardening patterns draw on.
struct L {
    nw: (i16, i16),
    dn: (i16, i16),
    pw: (i16, i16),
    dg: (i16, i16),
    v5: (i16, i16),
    ymtp: (i16, i16),
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
            nw: layer(pdk, "nwell"),
            dn: layer(pdk, "dnwell"),
            pw: layer(pdk, "lvpwell"),
            dg: layer(pdk, "dualgate"),
            v5: layer(pdk, "v5_xtor"),
            ymtp: layer(pdk, "ymtp_mk"),
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

    /// An N+ tap with a contact, centred on `(cx, cy)`, `size` on a side; the contact's
    /// centre comes back for the strap.
    fn tap(&self, cx: f64, cy: f64, size: f64) -> (Vec<GdsElement>, (f64, f64)) {
        let h = size * 0.5;
        (
            vec![
                rect(self.comp, cx - h, cy - h, cx + h, cy + h),
                rect(
                    self.nplus,
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

    /// A well `(x0, y0)-(x1, y1)` with a 1 µm tap at its centre.
    fn well(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (Vec<GdsElement>, (f64, f64)) {
        let (mut v, p) = self.tap((x0 + x1) * 0.5, (y0 + y1) * 0.5, 1.0);
        v.push(rect(self.nw, x0, y0, x1, y1));
        (v, p)
    }

    /// One Metal1 plate over the contact centres: one net.
    fn strap(&self, pts: &[(f64, f64)]) -> GdsElement {
        strap_on(self.m1, pts)
    }

    /// Two wells `gap` apart, 3 × 3 µm, the left one at `(x, y)`, on one net or two;
    /// `dg` covers both with 0.5 to spare when set.
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
}

fn strap_on(layer: (i16, i16), pts: &[(f64, f64)]) -> GdsElement {
    let m = 0.11 + 0.2;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    rect(layer, x0 - m, y0 - m, x1 + m, y1 + m)
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    nw_1a_h(&l);
    nw_1b_h(&l);
    nw_2a_h(&l);
    nw_2b_h(&l);
    nw_3_h(&l);
    nw_4_h(&l);
    nw_5_h(&l);
    nw_6_h(&l);
}

// --- NW.1a: min. Nwell width 0.86, LV and MV ---

fn nw_1a_h(l: &L) {
    // h1 - which well is the 5 V kind.  Five bare wells 0.855 wide (one step under
    // 0.86) and 3 µm tall, 4 µm apart: (a) no marker near it, NW.1a_LV; (b) Dualgate
    // covering it with 0.5 to spare, NW.1a_MV; (c) Dualgate over its top half only,
    // NW.1a_MV - the marker lies over the shape; (d) Dualgate abutting its right edge
    // and not over it, NW.1a_LV - a marker beside a well is not over it; (e) 0.86
    // wide under Dualgate, clean at the bound.
    write(
        "NW.1a.h1",
        vec![
            rect(l.nw, 2.0, 2.0, 2.855, 5.0), // (a) NW.1a_LV
            rect(l.nw, 6.0, 2.0, 6.855, 5.0), // (b) NW.1a_MV
            rect(l.dg, 5.5, 1.5, 7.355, 5.5),
            rect(l.nw, 10.0, 2.0, 10.855, 5.0), // (c) NW.1a_MV
            rect(l.dg, 9.5, 3.5, 11.355, 5.5),
            rect(l.nw, 14.0, 2.0, 14.855, 5.0), // (d) NW.1a_LV
            rect(l.dg, 14.855, 1.5, 16.0, 5.5),
            rect(l.nw, 18.0, 2.0, 18.86, 5.0), // (e) clean
            rect(l.dg, 17.5, 1.5, 19.36, 5.5),
        ],
    );

    // h2 - the V5_XTOR marker, which the manual's section 4.1 calls the 5 V transistor
    // marking layer and section 7.4 never mentions.  A 0.855 well under V5_XTOR alone
    // has no Dualgate over it, so it is the 3.3 V kind: NW.1a_LV.  One under both
    // markers is NW.1a_MV.
    write(
        "NW.1a.h2",
        vec![
            rect(l.nw, 2.0, 2.0, 2.855, 5.0), // NW.1a_LV
            rect(l.v5, 1.5, 1.5, 3.355, 5.5),
            rect(l.nw, 6.0, 2.0, 6.855, 5.0), // NW.1a_MV
            rect(l.v5, 5.5, 1.5, 7.355, 5.5),
            rect(l.dg, 5.5, 1.5, 7.355, 5.5),
        ],
    );

    // h3 - the marker at the far end of a long well.  A 0.855 well running from x = 5
    // to x = 35, across the 20 µm line and four of the 7 µm ones, with Dualgate over its
    // last 4 µm only: the whole well is the 5 V kind, NW.1a_MV once, at every tile size.
    // A second such well with the Dualgate abutting its end at x = 35 is NW.1a_LV.
    write(
        "NW.1a.h3",
        vec![
            rect(l.nw, 5.0, 2.0, 35.0, 2.855), // NW.1a_MV
            rect(l.dg, 31.0, 1.5, 35.5, 3.355),
            rect(l.nw, 5.0, 8.0, 35.0, 8.855), // NW.1a_LV
            rect(l.dg, 35.0, 7.5, 39.0, 9.355),
        ],
    );
}

// --- NW.1b: min. Nwell width as a resistor 2.0, outside DNWELL only ---

fn nw_1b_h(l: &L) {
    // A resistor the way NW.7 describes it: an N+ COMP head at each end of the well and
    // RES_MK between the heads, covering the well's width with 0.3 to spare.  The heads
    // are 1 µm tall, the marker runs from head to head.
    let resistor = |x0: f64, y0: f64, x1: f64, y1: f64| -> Vec<GdsElement> {
        let (mut v, _) = l.tap((x0 + x1) * 0.5, y0 + 0.5, 0.6);
        v.extend(l.tap((x0 + x1) * 0.5, y1 - 0.5, 0.6).0);
        v.push(rect(l.nw, x0, y0, x1, y1));
        v.push(rect(l.res, x0 - 0.3, y0 + 1.0, x1 + 0.3, y1 - 1.0));
        v
    };
    // h1 - what makes a well a resistor.  (a) 1.995 × 6 with the marker overhanging on
    // every side, NW.1b_LV; (b) 1.995 × 6 with the marker wholly inside the well, not a
    // resistor, clean; (c) 1.995 × 6 drawn as NW.7 says, heads and a marker between
    // them, NW.1b_LV; (d) 2.0 × 6 the same way, clean at the bound; (e) 3.0 wide with a
    // marker between heads only 1.5 apart - the resistor's width is the well's, 3.0,
    // and NW.1b measures width, so clean.
    write(
        "NW.1b.h1",
        [
            vec![
                rect(l.nw, 2.0, 2.0, 3.995, 8.0), // (a) NW.1b_LV
                rect(l.res, 1.7, 1.7, 4.295, 8.3),
                rect(l.nw, 8.0, 2.0, 9.995, 8.0), // (b) clean
                rect(l.res, 8.2, 3.0, 9.795, 7.0),
            ],
            resistor(14.0, 2.0, 15.995, 8.0), // (c) NW.1b_LV
            resistor(20.0, 2.0, 22.0, 8.0),   // (d) clean
            {
                // (e) clean: heads 1.5 apart on a 3.0 wide well
                let (mut v, _) = l.tap(27.5, 2.5, 0.6);
                v.extend(l.tap(27.5, 5.5, 0.6).0);
                v.push(rect(l.nw, 26.0, 2.0, 29.0, 6.0));
                v.push(rect(l.res, 25.7, 3.0, 29.3, 4.5));
                v
            },
        ]
        .concat(),
    );

    // h2 - the 5 V resistor and the marker beside it.  (a) 1.995 wide under Dualgate,
    // NW.1b_MV; (b) 1.995 wide with the Dualgate abutting the well's right edge, not
    // over the well, NW.1b_LV; (c) 2.0 wide under Dualgate, clean.
    write(
        "NW.1b.h2",
        [
            resistor(2.0, 2.0, 3.995, 8.0), // (a) NW.1b_MV
            vec![rect(l.dg, 1.2, 1.5, 4.795, 8.5)],
            resistor(8.0, 2.0, 9.995, 8.0), // (b) NW.1b_LV
            vec![rect(l.dg, 9.995, 1.5, 12.0, 8.5)],
            resistor(14.0, 2.0, 16.0, 8.0), // (c) clean
            vec![rect(l.dg, 13.2, 1.5, 16.8, 8.5)],
        ]
        .concat(),
    );
}

// --- NW.2a: min. Nwell space, equi-potential, 0.6 LV / 0.74 MV ---

fn nw_2a_h(l: &L) {
    // A 3 × 3 well with a slot `d` wide and 1.5 deep cut into its top, tapped once.
    let notched = |x: f64, y: f64, d: f64| -> Vec<GdsElement> {
        let (mut v, p) = l.tap(x + 1.5, y + 0.75, 0.6);
        v.push(l.strap(&[p]));
        v.push(poly(
            l.nw,
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
    };
    // h1 - the LV bound.  Two wells on one net (a tap each, one Metal1 plate) at 0.595,
    // NW.2a_LV; at 0.6, clean; a slot 0.595 wide into one well, NW.2a_LV - a notch is
    // a space at the same potential; at 0.6, clean.
    write(
        "NW.2a.h1",
        [
            l.pair(2.0, 2.0, 0.595, true, false), // NW.2a_LV
            l.pair(12.0, 2.0, 0.6, true, false),  // clean
            notched(2.0, 10.0, 0.595),            // NW.2a_LV
            notched(10.0, 10.0, 0.6),             // clean
        ]
        .concat(),
    );

    // h2 - the MV bound and a pair of mixed voltage.  Under Dualgate: 0.735, NW.2a_MV;
    // 0.74, clean.  Then a same-net pair 0.7 apart with Dualgate over the left well
    // only, reaching 0.3 into the gap: the left well is the 5 V kind and its space is
    // 0.74, NW.2a_MV.
    write(
        "NW.2a.h2",
        [
            l.pair(2.0, 2.0, 0.735, true, true), // NW.2a_MV
            l.pair(12.0, 2.0, 0.74, true, true), // clean
            l.pair(2.0, 10.0, 0.7, true, false), // NW.2a_MV (mixed)
            vec![rect(l.dg, 1.5, 9.5, 5.3, 13.5)],
        ]
        .concat(),
    );

    // h3 - the YMTP marker, which NW.2a exempts.  A same-net pair at 0.595 with
    // YMTP_MK over both wells and the gap: exempt, clean.  A second pair at 0.595 with
    // the marker over the gap and 0.5 into each well but not over the wells: the space
    // is inside the marker, exempt, clean.
    write(
        "NW.2a.h3",
        [
            l.pair(2.0, 2.0, 0.595, true, false), // clean
            vec![rect(l.ymtp, 1.0, 1.0, 9.6, 6.0)],
            l.pair(2.0, 10.0, 0.595, true, false), // clean
            vec![rect(l.ymtp, 4.5, 9.5, 6.1, 13.5)],
        ]
        .concat(),
    );
}

// --- NW.2b: min. Nwell space, different potential, 1.4 LV / 1.7 MV ---

fn nw_2b_h(l: &L) {
    // h1 - what makes two wells one potential.  Two taps, two Metal1 plates: at 1.395,
    // NW.2b_LV; at 1.4, clean; at 0.595, NW.2b_LV (NW.2a's bound too, ignored in the
    // case).  Then at 1.395: (d) the two plates joined by Via1 and a Metal2 strap, one
    // net, clean; (e) the left plate running over the right well without touching its
    // tap, still two nets, NW.2b_LV; (f) one plate over both taps but the right tap
    // has no contact, two nets, NW.2b_LV; (g) one plate, one contact each, but the
    // right well's diffusion is P+ - a PMOS terminal, not a tap - two nets, NW.2b_LV.
    let mut v = [
        l.pair(2.0, 2.0, 1.395, false, false),  // (a) NW.2b_LV
        l.pair(12.0, 2.0, 1.4, false, false),   // (b) clean
        l.pair(22.0, 2.0, 0.595, false, false), // (c) NW.2b_LV
    ]
    .concat();
    // (d) clean
    v.extend(l.pair(2.0, 10.0, 1.395, false, false));
    for cx in [3.5, 7.895] {
        v.push(rect(l.via1, cx - 0.13, 11.37, cx + 0.13, 11.63));
    }
    v.push(rect(l.m2, 3.1, 11.1, 8.295, 11.9));
    // (e) NW.2b_LV
    v.extend(l.pair(12.0, 10.0, 1.395, false, false));
    v.push(rect(l.m1, 13.19, 10.0, 18.0, 10.5));
    // (f) NW.2b_LV
    {
        let (w, a) = l.well(22.0, 10.0, 25.0, 13.0);
        v.extend(w);
        let bx = 26.395;
        v.push(rect(l.nw, bx, 10.0, bx + 3.0, 13.0));
        v.push(rect(l.comp, bx + 1.0, 11.0, bx + 2.0, 12.0));
        v.push(rect(l.nplus, bx + 0.9, 10.9, bx + 2.1, 12.1));
        v.push(l.strap(&[a, (bx + 1.5, 11.5)]));
    }
    // (g) NW.2b_LV
    {
        let (w, a) = l.well(2.0, 18.0, 5.0, 21.0);
        v.extend(w);
        let bx = 6.395;
        v.push(rect(l.nw, bx, 18.0, bx + 3.0, 21.0));
        v.push(rect(l.comp, bx + 1.0, 19.0, bx + 2.0, 20.0));
        v.push(rect(l.pplus, bx + 0.9, 18.9, bx + 2.1, 20.1));
        v.push(rect(l.co, bx + 1.39, 19.39, bx + 1.61, 19.61));
        v.push(l.strap(&[a, (bx + 1.5, 19.5)]));
    }
    write("NW.2b.h1", v);

    // h2 - the MV bound.  Two nets under Dualgate at 1.695, NW.2b_MV; at 1.7, clean.
    write(
        "NW.2b.h2",
        [
            l.pair(2.0, 2.0, 1.695, false, true), // NW.2b_MV
            l.pair(12.0, 2.0, 1.7, false, true),  // clean
        ]
        .concat(),
    );

    // h3 - a pair of mixed voltage.  Two nets 1.695 apart with Dualgate over the left
    // well only, reaching 0.3 into the gap: the left well is the 5 V kind and its
    // space to any other well is 1.7, NW.2b_MV.  A second such pair at 1.7, clean.
    write(
        "NW.2b.h3",
        [
            l.pair(2.0, 2.0, 1.695, false, false), // NW.2b_MV
            vec![rect(l.dg, 1.5, 1.5, 5.3, 5.5)],
            l.pair(12.0, 2.0, 1.7, false, false), // clean
            vec![rect(l.dg, 11.5, 1.5, 15.3, 5.5)],
        ]
        .concat(),
    );

    // h4 - the marker beside the well.  Two nets at 1.395 with Dualgate abutting the
    // left well's left edge, not over either well: both are the 3.3 V kind, NW.2b_LV.
    write(
        "NW.2b.h4",
        [
            l.pair(4.0, 2.0, 1.395, false, false), // NW.2b_LV
            vec![rect(l.dg, 2.0, 1.5, 4.0, 5.5)],
        ]
        .concat(),
    );

    // h5 - inside a deep well.  Section 7.4: "all Nwell inside DNWELL will be shorted
    // together through DNWELL", and NW.2a/NW.2b say "Outside DNWELL".  Two wells 1.395
    // apart with a tap and a Metal1 plate each, in one DNWELL holding them by 1.0: one
    // potential through the deep well, clean.
    write(
        "NW.2b.h5",
        [
            l.pair(2.0, 2.0, 1.395, false, false), // clean
            vec![rect(l.dn, 1.0, 1.0, 10.395, 6.0)],
        ]
        .concat(),
    );

    // h6 - the same two wells without taps of their own, the deep well tapped once
    // beside them (an N+ contact in the DNWELL outside both wells, on its own Metal1):
    // one potential through the deep well, clean.
    write("NW.2b.h6", {
        let mut v = vec![
            rect(l.nw, 2.0, 2.0, 5.0, 5.0),
            rect(l.nw, 6.395, 2.0, 9.395, 5.0),
            rect(l.dn, 1.0, 1.0, 10.395, 8.0),
        ];
        let (t, p) = l.tap(5.7, 6.5, 0.8);
        v.extend(t);
        v.push(l.strap(&[p]));
        v
    });

    // h7 - the tile lines.  Two-net pairs at 1.395 with the gap straddling x = 20 and
    // x = 42, NW.2b_LV twice; one-net pairs at 0.595 with the strap crossing x = 40
    // and x = 21, NW.2a_LV twice.
    write(
        "NW.2b.h7",
        [
            l.pair(16.3, 2.0, 1.395, false, false), // gap 19.3-20.695: NW.2b_LV
            l.pair(38.3, 2.0, 1.395, false, false), // gap 41.3-42.695: NW.2b_LV
            l.pair(36.5, 10.0, 0.595, true, false), // gap 39.5-40.095: NW.2a_LV
            l.pair(17.5, 10.0, 0.595, true, false), // gap 20.5-21.095: NW.2a_LV
        ]
        .concat(),
    );
}

// --- NW.3: min. Nwell to DNWELL space 3.1 ---

fn nw_3_h(l: &L) {
    // h1 - (a) a well 3.1 from a deep well, clean; (b) 3.095, NW.3; (c) abutting it,
    // NW.3 - a space of nothing is under 3.1; (d) corner to corner 2.19 each way, 3.097
    // on the diagonal, NW.3; (e) 2.2 each way, 3.111, clean; (f) a well in the hole of
    // a DNWELL ring, 3.095 from the ring's inner wall, NW.3 - it is not inside the
    // deep well, so there is no enclosure to measure, only a space.
    let mut v = vec![
        rect(l.nw, 2.0, 2.0, 5.0, 5.0), // (a) clean
        rect(l.dn, 8.1, 2.0, 12.0, 5.0),
        rect(l.nw, 2.0, 12.0, 5.0, 15.0), // (b) NW.3
        rect(l.dn, 8.095, 12.0, 12.0, 15.0),
        rect(l.nw, 2.0, 22.0, 5.0, 25.0), // (c) NW.3
        rect(l.dn, 5.0, 22.0, 9.0, 25.0),
        rect(l.nw, 22.0, 2.0, 25.0, 5.0), // (d) NW.3
        rect(l.dn, 27.19, 7.19, 31.0, 11.0),
        rect(l.nw, 22.0, 14.0, 25.0, 17.0), // (e) clean
        rect(l.dn, 27.2, 19.2, 31.0, 23.0),
    ];
    // (f) NW.3: ring 2 wide, hole 9.19 square, well 3 × 3 at its centre
    let (x0, y0) = (40.0, 2.0);
    let (ix0, iy0, ix1, iy1) = (x0 + 2.0, y0 + 2.0, x0 + 11.19, y0 + 11.19);
    v.push(rect(l.dn, x0, y0, ix1 + 2.0, iy0));
    v.push(rect(l.dn, x0, iy1, ix1 + 2.0, iy1 + 2.0));
    v.push(rect(l.dn, x0, iy0, ix0, iy1));
    v.push(rect(l.dn, ix1, iy0, ix1 + 2.0, iy1));
    v.push(rect(
        l.nw,
        ix0 + 3.095,
        iy0 + 3.095,
        ix0 + 6.095,
        iy0 + 6.095,
    ));
    write("NW.3.h1", v);
}

// --- NW.4: Nwell to LVPWELL space 0 ---

fn nw_4_h(l: &L) {
    // h1 - (a) a well overlapping a P-well by 0.005, NW.4; (b) abutting it, clean -
    // the space is 0; (c) overlapping one inside a deep well (holding both by 3.0),
    // NW.4 - the rule is not about where the wells lie.
    write(
        "NW.4.h1",
        vec![
            rect(l.nw, 2.0, 2.0, 5.0, 5.0), // (a) NW.4
            rect(l.pw, 4.995, 2.0, 8.0, 5.0),
            rect(l.nw, 12.0, 2.0, 15.0, 5.0), // (b) clean
            rect(l.pw, 15.0, 2.0, 18.0, 5.0),
            rect(l.nw, 25.0, 2.0, 28.0, 5.0), // (c) NW.4
            rect(l.pw, 27.995, 2.0, 31.0, 5.0),
            rect(l.dn, 22.0, -1.0, 34.0, 8.0),
        ],
    );
}

// --- NW.5: min. DNWELL enclosure of Nwell 0.5 ---

fn nw_5_h(l: &L) {
    // A 3 × 3 well at (x, y) and a deep well around it with margins l, b, r, t.
    let held = |x: f64, y: f64, ml: f64, mb: f64, mr: f64, mt: f64| {
        vec![
            rect(l.nw, x, y, x + 3.0, y + 3.0),
            rect(l.dn, x - ml, y - mb, x + 3.0 + mr, y + 3.0 + mt),
        ]
    };
    // h1 - the LV bound and the corner.  (a) held by 0.5 all round, clean; (b) 0.495
    // on the right, NW.5_LV; (c) the well's right edge on the deep well's, NW.5_LV;
    // (d) held by 0.5 but the deep well's top-right corner chamfered 0.3 each way -
    // the well's corner is 0.495 from the chamfer, NW.5_LV (enclosure is the closest
    // approach); (e) chamfered 0.29 each way, 0.502, clean.
    let chamfered = |x: f64, y: f64, cut: f64| {
        let (x1, y1) = (x + 3.5, y + 3.5);
        vec![
            rect(l.nw, x, y, x + 3.0, y + 3.0),
            chamfered_tr(l.dn, x - 0.5, y - 0.5, x1, y1, x1 + y1 - cut),
        ]
    };
    write(
        "NW.5.h1",
        [
            held(2.0, 2.0, 0.5, 0.5, 0.5, 0.5),    // (a) clean
            held(12.0, 2.0, 0.5, 0.5, 0.495, 0.5), // (b) NW.5_LV
            held(22.0, 2.0, 0.5, 0.5, 0.0, 0.5),   // (c) NW.5_LV
            chamfered(2.0, 12.0, 0.3),             // (d) NW.5_LV
            chamfered(12.0, 12.0, 0.29),           // (e) clean
        ]
        .concat(),
    );

    // h2 - a well crossing out of the deep well.  A 3.3 V well half in, half out:
    // there is no margin to measure, and the manual's answer is NW.5_LV - the 3.3 V
    // rule, once.  It is not the 5 V kind, so NW.5_MV has nothing to say.
    write(
        "NW.5.h2",
        vec![
            rect(l.nw, 2.0, 2.0, 5.0, 5.0), // NW.5_LV
            rect(l.dn, 1.0, 1.0, 3.5, 6.0),
        ],
    );

    // h3 - the MV bound.  Under Dualgate: (a) held by 0.495 on the right, NW.5_MV; (b)
    // half in, half out, NW.5_MV; (c) held by 0.5, clean.
    write(
        "NW.5.h3",
        [
            held(2.0, 2.0, 0.5, 0.5, 0.495, 0.5), // (a) NW.5_MV
            vec![rect(l.dg, 1.0, 1.0, 6.0, 6.0)],
            vec![
                rect(l.nw, 12.0, 2.0, 15.0, 5.0), // (b) NW.5_MV
                rect(l.dn, 11.0, 1.0, 13.5, 6.0),
                rect(l.dg, 11.0, 1.0, 16.0, 6.0),
            ],
            held(22.0, 2.0, 0.5, 0.5, 0.5, 0.5), // (c) clean
            vec![rect(l.dg, 21.0, 1.0, 26.0, 6.0)],
        ]
        .concat(),
    );

    // h4 - the marker beside the well.  Held by 0.495 with Dualgate abutting the well's
    // left edge, over the deep well but not over the well: the well is the 3.3 V kind,
    // NW.5_LV.
    write(
        "NW.5.h4",
        [
            held(4.0, 2.0, 2.0, 0.5, 0.495, 0.5), // NW.5_LV
            vec![rect(l.dg, 2.5, 1.5, 4.0, 5.5)],
        ]
        .concat(),
    );
}

// --- NW.6: Nwell resistors only outside DNWELL ---

fn nw_6_h(l: &L) {
    // h1 - (a) a well in a deep well (held by 1.0) with RES_MK overhanging it on every
    // side, NW.6; (b) the same well without a marker, clean.
    write(
        "NW.6.h1",
        vec![
            rect(l.nw, 2.0, 2.0, 5.0, 10.0), // (a) NW.6
            rect(l.dn, 1.0, 1.0, 6.0, 11.0),
            rect(l.res, 1.7, 1.7, 5.3, 10.3),
            rect(l.nw, 12.0, 2.0, 15.0, 10.0), // (b) clean
            rect(l.dn, 11.0, 1.0, 16.0, 11.0),
        ],
    );

    // h2 - the resistor as NW.7 draws it, in a deep well.  A 3 × 8 well held by 1.0,
    // an N+ COMP head at each end, RES_MK from head to head covering the width with
    // 0.3 to spare - the well runs past the marker at both ends.  NW.6.
    write("NW.6.h2", {
        let (mut v, _) = l.tap(3.5, 2.5, 0.6);
        v.extend(l.tap(3.5, 9.5, 0.6).0);
        v.push(rect(l.nw, 2.0, 2.0, 5.0, 10.0)); // NW.6
        v.push(rect(l.dn, 1.0, 1.0, 6.0, 11.0));
        v.push(rect(l.res, 1.7, 3.0, 5.3, 9.0));
        v
    });
}
