// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Contact: a good and a bad pattern for every rule in the `contact` deck.
//!
//! Two things every fixture has to respect before it can break anything.  A contact must
//! land on poly or on active - CO.11 forbids one that lands on neither - and it must
//! carry Metal1 over it, since three rules bound how far that metal encloses it.  So the
//! base cell is a contact on an active with its metal, and the fixtures that need a poly
//! contact draw a second one beside it, far enough that neither rule reaches across.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/contact";

/// Contact side - CO.1 wants exactly this.
const CO: f64 = 0.22;
/// The bar a contact lands on, and how far it runs past the contact.  CO.3 and CO.4 ask
/// 0.07, CO.5a and CO.5b ask 0.1 of a butted active.
const BAR: f64 = 3.0;
const ENC: f64 = 0.5;
/// Metal1's reach past the contact: over CO.6's 0.005 and CO.6a/CO.6b's 0.06, and wide
/// enough that the track is not a line end.
const M1: f64 = 0.4;

struct Ctx {
    contact: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    metal1: (i16, i16),
}

/// A contact on a bar, with its metal.
struct Cell {
    x: f64,
    y: f64,
    bar: (i16, i16),
    implant: Option<(i16, i16)>,
    /// Contact side, for CO.1.
    co: f64,
    /// How far the bar runs past the contact, for CO.3 / CO.4 / CO.5.
    enc: f64,
    /// How far Metal1 runs past it, for CO.6.
    m1: f64,
}

impl Cell {
    fn on(bar: (i16, i16), implant: Option<(i16, i16)>, x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            bar,
            implant,
            co: CO,
            enc: ENC,
            m1: M1,
        }
    }

    /// The contact's own box.
    fn co_box(&self) -> (f64, f64, f64, f64) {
        (self.x, self.y, self.x + self.co, self.y + self.co)
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x0, y0, x1, y1) = self.co_box();
        let e = self.enc;
        let mut v = vec![
            rect(self.bar, x0 - e, y0 - e, x0 - e + BAR, y1 + e),
            rect(c.contact, x0, y0, x1, y1),
            rect(
                c.metal1,
                x0 - self.m1,
                y0 - self.m1,
                x1 + self.m1,
                y1 + self.m1,
            ),
        ];
        if let Some(i) = self.implant {
            v.push(rect(
                i,
                x0 - e - 0.3,
                y0 - e - 0.3,
                x0 - e + BAR + 0.3,
                y1 + e + 0.3,
            ));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
        poly: layer(pdk, "poly2_drawn"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        metal1: layer(pdk, "metal1_drawn"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let on_comp = |x: f64, y: f64| Cell::on(c.comp, Some(c.nplus), x, y);
    let on_poly = |x: f64, y: f64| Cell::on(c.poly, None, x, y);
    let clean = || on_comp(o, o).draw(&c);

    for id in [
        "CO.1", "CO.2a", "CO.4", "CO.6", "CO.6a", "CO.6b", "CO.7", "CO.9", "CO.10", "CO.11",
    ] {
        write(id, "good", clean());
    }
    // The rules about a contact on poly get a poly cell as their clean half.
    for id in ["CO.3", "CO.8"] {
        write(id, "good", on_poly(o, o).draw(&c));
    }
    // The butted-active rules get their own arrangement: an N+ active meeting a P+ one,
    // with a contact on *one* side only.  Two contacts, one each side, would be 0.19 µm
    // apart and trip the spacing rule, and would fire both CO.5a and CO.5b at once.
    let butted = |n_side: bool, gap_from_edge: f64| -> Vec<GdsElement> {
        let mid = o + 4.0;
        let (y0, y1) = (o, o + 2.0);
        let mut v = vec![
            rect(c.comp, o, y0, mid, y1),
            rect(c.nplus, o - 0.3, y0 - 0.3, mid, y1 + 0.3),
            rect(c.comp, mid, y0, mid + 4.0, y1),
            rect(c.pplus, mid, y0 - 0.3, mid + 4.3, y1 + 0.3),
        ];
        let x = if n_side {
            mid - gap_from_edge - CO
        } else {
            mid + gap_from_edge
        };
        let y = y0 + (2.0 - CO) * 0.5;
        v.push(rect(c.contact, x, y, x + CO, y + CO));
        v.push(rect(c.metal1, x - M1, y - M1, x + CO + M1, y + CO + M1));
        v
    };
    write("CO.5a", "good", butted(true, 0.5));
    write("CO.5b", "good", butted(false, 0.5));

    // CO.1: a contact that is not exactly 0.22 µm.
    write("CO.1", "bad", {
        let mut cell = on_comp(o, o);
        cell.co = 0.215;
        cell.draw(&c)
    });

    // CO.2a: two contacts 0.25 µm apart, on one active with one metal over both.
    write("CO.2a", "bad", {
        let mut v = clean();
        let x = o + CO + 0.245;
        let y = o;
        v.push(rect(c.contact, x, y, x + CO, y + CO));
        v.push(rect(c.metal1, x - M1, y - M1, x + CO + M1, y + CO + M1));
        v
    });

    // CO.2b: inside a 4x4 array the spacing rises from CO.2a's 0.25 to 0.28, so a 0.26 µm
    // gap is legal for a lone pair and not for an array.  The clean half is the same gap
    // in a 3x3, which is too small to be an array at all.
    let array = |n: usize, gap: f64| {
        let pitch = CO + gap;
        let side = (n - 1) as f64 * pitch + CO;
        let mut v = vec![
            rect(c.comp, o - 0.2, o - 0.2, o + side + 0.2, o + side + 0.2),
            rect(c.nplus, o - 0.4, o - 0.4, o + side + 0.4, o + side + 0.4),
            rect(c.metal1, o - M1, o - M1, o + side + M1, o + side + M1),
        ];
        for i in 0..n {
            for j in 0..n {
                let (x, y) = (o + i as f64 * pitch, o + j as f64 * pitch);
                v.push(rect(c.contact, x, y, x + CO, y + CO));
            }
        }
        v
    };
    write("CO.2b", "good", array(3, 0.26));
    write("CO.2b", "bad", array(4, 0.26));

    // CO.3 / CO.4: the poly, and the active, running only 0.07 µm past the contact.
    write("CO.3", "bad", {
        let mut cell = on_poly(o, o);
        cell.enc = 0.065;
        cell.draw(&c)
    });
    write("CO.4", "bad", {
        let mut cell = on_comp(o, o);
        cell.enc = 0.065;
        cell.draw(&c)
    });

    // CO.5a / CO.5b: a contact on a butted active, only 0.1 µm from the far implant.
    write("CO.5a", "bad", butted(true, 0.095));
    write("CO.5b", "bad", butted(false, 0.095));

    // CO.6: Metal1 short of the contact on one side only.  Zero margin all round would
    // break this rule and the two beside it at once - a side under 0.04 µm is what CO.6b
    // triggers on - so the other sides stay generous, and the metal runs long enough that
    // its line ends are nowhere near the contact for CO.6a to measure.
    write("CO.6", "bad", {
        let mut v = on_comp(o, o).draw(&c);
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.metal1));
        v.push(rect(
            c.metal1,
            o - 0.003,
            o - 2.0,
            o + CO + 0.06,
            o + CO + 2.0,
        ));
        v
    });

    // CO.6a: the contact at the end of a Metal1 track under 0.34 µm wide, where the metal
    // must reach 0.06 µm past it.
    write("CO.6a", "bad", {
        let mut v = on_comp(o, o).draw(&c);
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.metal1));
        // A 0.3 µm track running away from the contact, ending 0.055 µm past it.
        v.push(rect(
            c.metal1,
            o - 0.055,
            o - 0.04,
            o + CO + 2.0,
            o + CO + 0.04,
        ));
        v
    });

    // CO.6b: where one side encloses by under 0.04 µm, the sides beside it must reach
    // 0.06.  The metal is drawn wide in both directions - over the 0.34 µm that makes a
    // track a line end - so CO.6a, which is about the cap on such a track, stays out of
    // it; only the two margins are short.
    write("CO.6b", "bad", {
        let mut v = on_comp(o, o).draw(&c);
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.metal1));
        v.push(rect(
            c.metal1,
            o - 0.035,
            o - 0.055,
            o + CO + 0.5,
            o + CO + 2.0,
        ));
        v
    });

    // CO.7: a contact on active within 0.15 µm of a gate.
    write("CO.7", "bad", {
        let mut v = clean();
        let x = o + CO + 0.145;
        v.push(rect(c.poly, x, o - ENC - 0.5, x + 0.5, o + CO + ENC + 0.5));
        v
    });

    // CO.8: a contact on poly within 0.17 µm of an active.
    write("CO.8", "bad", {
        let mut v = on_poly(o, o).draw(&c);
        let x = o + CO + 0.165;
        v.push(rect(c.comp, x, o, x + 2.0, o + CO));
        v
    });

    // CO.9: a contact on the boundary two butted actives share.
    write("CO.9", "bad", {
        let mut v = butted(true, 0.5);
        let mid = o + 4.0;
        let y = o + (2.0 - CO) * 0.5;
        v.push(rect(c.contact, mid - CO * 0.5, y, mid + CO * 0.5, y + CO));
        v.push(rect(
            c.metal1,
            mid - CO * 0.5 - M1,
            y - M1,
            mid + CO * 0.5 + M1,
            y + CO + M1,
        ));
        v
    });

    // CO.10: a contact on a gate - poly over active - which is never allowed.
    write("CO.10", "bad", {
        let v = vec![
            rect(c.comp, o - ENC, o - ENC, o - ENC + BAR, o + CO + ENC),
            rect(
                c.nplus,
                o - ENC - 0.3,
                o - ENC - 0.3,
                o - ENC + BAR + 0.3,
                o + CO + ENC + 0.3,
            ),
            rect(c.poly, o - ENC, o - ENC, o + CO + ENC, o + CO + ENC),
            rect(c.contact, o, o, o + CO, o + CO),
            rect(c.metal1, o - M1, o - M1, o + CO + M1, o + CO + M1),
        ];
        v
    });

    // CO.11: a contact on neither poly nor active.
    write("CO.11", "bad", {
        vec![
            rect(c.contact, o, o, o + CO, o + CO),
            rect(c.metal1, o - M1, o - M1, o + CO + M1, o + CO + M1),
        ]
    });
}
