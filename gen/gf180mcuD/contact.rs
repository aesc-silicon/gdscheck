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
use crate::helpers::{layer, library, poly, rect, write_gz};
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
    hardening(pdk);
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

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.12 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/contact/CO.<rule>.h<n>.gds.gz` with a case in the
// `hardening_contact` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/contact.md.
//
// What these draw is the deck's own conditions - the fixed 0.22 µm contact and what
// "min/max size" means for a contact that is not a square, the 4x4 array threshold and
// how far a contact has to be to fall outside an array, the poly / COMP / implant /
// Metal1 enclosures at the bound, the line-end and adjacent-side readings of CO.6, the
// gate and the butting edge, and the SRAM and OTP markers that exempt.  The generic
// classes (the bound on a bare layer, both metrics, 45°, unions, notches, fifty at once,
// a shape far off) are the engine family's and are not redrawn here.

/// Layers the hardening patterns draw on.
struct H {
    co: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    np: (i16, i16),
    pp: (i16, i16),
    m1: (i16, i16),
    sram: (i16, i16),
    otp: (i16, i16),
    res: (i16, i16),
}

impl H {
    fn new(pdk: &PdkConfig) -> Self {
        H {
            co: layer(pdk, "contact"),
            comp: layer(pdk, "comp"),
            poly: layer(pdk, "poly2_drawn"),
            np: layer(pdk, "nplus"),
            pp: layer(pdk, "pplus"),
            m1: layer(pdk, "metal1_drawn"),
            sram: layer(pdk, "sramcore"),
            otp: layer(pdk, "otp_mk"),
            res: layer(pdk, "res_mk"),
        }
    }

    /// A contact box at `(x, y)`, `w` by `h`.
    fn co(&self, x: f64, y: f64, w: f64, h: f64) -> GdsElement {
        rect(self.co, x, y, x + w, y + h)
    }

    /// A square contact at `(x, y)`.
    fn c(&self, x: f64, y: f64) -> GdsElement {
        self.co(x, y, CO, CO)
    }

    /// A box around the contact at `(x, y)` with the margins `[left, right, bottom, top]`;
    /// a negative margin falls short of the contact.
    fn around(&self, l: (i16, i16), x: f64, y: f64, d: [f64; 4]) -> GdsElement {
        rect(l, x - d[0], y - d[2], x + CO + d[1], y + CO + d[3])
    }
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A contact array at `(x, y)` whose gaps are given one per column / row after the first,
/// with COMP and Metal1 over all of it, so one deficient gap can sit inside an otherwise
/// legal array.
fn co_array(h: &H, x: f64, y: f64, gaps_x: &[f64], gaps_y: &[f64]) -> Vec<GdsElement> {
    let pos = |gaps: &[f64], start: f64| -> Vec<f64> {
        let mut v = vec![start];
        for g in gaps {
            v.push(v[v.len() - 1] + CO + g);
        }
        v
    };
    let xs = pos(gaps_x, x);
    let ys = pos(gaps_y, y);
    let (x1, y1) = (xs[xs.len() - 1] + CO, ys[ys.len() - 1] + CO);
    let mut v = vec![
        rect(h.comp, x - 0.3, y - 0.3, x1 + 0.3, y1 + 0.3),
        rect(h.m1, x - 0.4, y - 0.4, x1 + 0.4, y1 + 0.4),
    ];
    for &cx in &xs {
        for &cy in &ys {
            v.push(h.c(cx, cy));
        }
    }
    v
}

fn hardening(pdk: &PdkConfig) {
    let h = H::new(pdk);
    co_1_h(&h);
    co_2b_h(&h);
    co_3_h(&h);
    co_4_h(&h);
    co_5_h(&h);
    co_6_h(&h);
    co_6a_h(&h);
    co_6b_h(&h);
    co_7_h(&h);
    co_8_h(&h);
    co_9_h(&h);
    co_10_h(&h);
    co_11_h(&h);
}

// --- CO.1: min/max contact size 0.22 ---

fn co_1_h(h: &H) {
    // h1 - the size in both directions.  Five contacts 1 µm apart on one COMP bar under
    // one Metal1 plate: 0.22 square (clean), then one step tall, one step wide, one step
    // narrow and one step short.  "Min/max" is both bounds, so all four fire.
    let mut v = vec![
        rect(h.comp, 2.3, 2.3, 7.1, 3.0),
        rect(h.m1, 2.0, 2.0, 7.4, 3.3),
    ];
    for (i, (w, t)) in [
        (CO, CO),
        (CO, CO + 0.005),
        (CO + 0.005, CO),
        (CO - 0.005, CO),
        (CO, CO - 0.005),
    ]
    .into_iter()
    .enumerate()
    {
        v.push(h.co(2.5 + i as f64, 2.4, w, t));
    }
    hwrite("CO.1.h1", v);

    // h2 - a contact that is not a square.  (a) two 0.22 squares drawn side by side with
    // no gap: one 0.44 by 0.22 shape after merging, which is 0.44 across and breaks the
    // maximum; (b) an L whose arms are both 0.22 wide - every straight run measures
    // 0.22, but the contact is not a 0.22 square and its outer edges are 0.44 long;
    // (c) one 0.22 square drawn as two overlapping boxes, which is a legal contact.
    hwrite(
        "CO.1.h2",
        vec![
            rect(h.comp, 2.3, 5.7, 7.1, 7.0),
            rect(h.m1, 2.0, 5.4, 7.4, 7.3),
            h.c(2.5, 6.0),
            h.c(2.72, 6.0),
            poly(
                h.co,
                &[
                    (4.5, 6.0),
                    (4.94, 6.0),
                    (4.94, 6.22),
                    (4.72, 6.22),
                    (4.72, 6.44),
                    (4.5, 6.44),
                ],
            ),
            rect(h.co, 6.5, 6.0, 6.72, 6.15),
            rect(h.co, 6.5, 6.07, 6.72, 6.22),
        ],
    );
}

// --- CO.2b: space in a 4x4 or larger contact array 0.28 (CO.2a's 0.25 otherwise) ---

fn co_2b_h(h: &H) {
    // A contact beside an array, with the COMP and Metal1 it needs of its own.
    let extra = |v: &mut Vec<GdsElement>, x: f64, y: f64| {
        v.push(h.around(h.comp, x, y, [0.3; 4]));
        v.push(h.c(x, y));
        v.push(h.around(h.m1, x, y, [0.4; 4]));
    };
    let pitch = CO + 0.30;

    // h1 - what makes a group an array.  Every gap is 0.30 except one row gap of 0.275,
    // which is legal for a lone pair (CO.2a asks 0.25) and not inside an array.
    // (a) 4 by 4: an array, so the four pairs across that row gap fire; (b) the same
    // with three columns - twelve contacts, not "4x4 or larger", nothing fires;
    // (c) sixteen contacts in a single row at 0.275: sixteen is a 4x4's count, but a row
    // is not an array, nothing fires.
    let gy = [0.30, 0.30, 0.275];
    let mut v = co_array(h, 3.0, 3.0, &[0.30; 3], &gy);
    v.extend(co_array(h, 3.0, 8.0, &[0.30; 2], &gy));
    v.extend(co_array(h, 3.0, 13.0, &[0.275; 15], &[]));
    hwrite("CO.2b.h1", v);

    // h2 - how far a contact has to be to fall outside the array.  (a) a legal 4x4 at
    // 0.30 with a seventeenth contact 0.275 to the right of one row: a contact in a 4x4
    // array, so 0.28 applies and it fires; (b) the same beside a 3x3, where 0.25 applies
    // and it is clean; (c) a legal 4x4 with a contact set diagonally off its top right
    // corner, 0.20 across and 0.19 up - corner to corner that is 0.2762, under 0.28 and
    // over CO.2a's 0.25, and the contact rule carries no projection condition (the via
    // rule does), so it fires; (d) a 4x4 with a deficient row gap of its own and a
    // seventeenth contact beside it.
    let mut v = co_array(h, 3.0, 3.0, &[0.30; 3], &[0.30; 3]);
    extra(&mut v, 3.0 + 3.0 * pitch + CO + 0.275, 3.0 + pitch);
    v.extend(co_array(h, 10.0, 3.0, &[0.30; 2], &[0.30; 2]));
    extra(&mut v, 10.0 + 2.0 * pitch + CO + 0.275, 3.0 + pitch);
    v.extend(co_array(h, 3.0, 10.0, &[0.30; 3], &[0.30; 3]));
    extra(
        &mut v,
        3.0 + 3.0 * pitch + CO + 0.20,
        10.0 + 3.0 * pitch + CO + 0.19,
    );
    // (d) a 4x4 whose own middle row gap is 0.275, with a seventeenth contact 0.275 to
    // the right as well: the array's four deficient pairs fire whatever the extra
    // contact does.
    v.extend(co_array(h, 10.0, 10.0, &[0.30; 3], &[0.30, 0.275, 0.30]));
    extra(&mut v, 10.0 + 3.0 * pitch + CO + 0.275, 10.0 + pitch);
    hwrite("CO.2b.h2", v);

    // h3 - the same array on the tile lines: the cluster that makes it an array has to
    // be found across a cut.  Three 4x4 arrays with one 0.275 row gap, straddling
    // x = 20, x = 42 and y = 21.
    let mut v = co_array(h, 19.1, 3.0, &[0.30; 3], &gy);
    v.extend(co_array(h, 41.1, 3.0, &[0.30; 3], &gy));
    v.extend(co_array(h, 3.0, 20.1, &[0.30; 3], &gy));
    hwrite("CO.2b.h3", v);
}

// --- CO.3: Poly2 overlap of contact 0.07 ---

/// The five enclosure readings of CO.3 / CO.4, drawn on `l`: 0.07 at the bound, 0.065,
/// a margin of nothing, and a chamfered corner either side of the value.
fn enclosure_h(h: &H, l: (i16, i16)) -> Vec<GdsElement> {
    let cell = |x: f64, y: f64, enc: [f64; 4]| -> Vec<GdsElement> {
        vec![
            h.around(l, x, y, enc),
            h.c(x, y),
            h.around(h.m1, x, y, [0.4; 4]),
        ]
    };
    let mut v = cell(3.0, 3.0, [0.07, 0.7, 0.07, 0.7]);
    v.extend(cell(8.0, 3.0, [0.065, 0.7, 0.07, 0.7]));
    v.extend(cell(13.0, 3.0, [0.0, 0.7, 0.07, 0.7]));
    // The contact sits in the chamfered corner of a 1 µm square, 0.07 from both straight
    // edges: a 0.05 chamfer passes 0.0636 from its corner, a 0.04 one 0.0707.
    for (x, c) in [(3.0, 0.05), (8.0, 0.04)] {
        let (px, py) = (x, 8.0);
        v.push(poly(
            l,
            &[
                (px, py),
                (px + 1.0, py),
                (px + 1.0, py + 1.0 - c),
                (px + 1.0 - c, py + 1.0),
                (px, py + 1.0),
            ],
        ));
        let (cx, cy) = (px + 1.0 - 0.07 - CO, py + 1.0 - 0.07 - CO);
        v.push(h.c(cx, cy));
        v.push(h.around(h.m1, cx, cy, [0.4; 4]));
    }
    v
}

/// The three conditions of CO.3 / CO.4: a contact the layer's edge cuts, a contact just
/// outside it, and a deficient margin under a SRAMCORE marker.
fn crossing_h(h: &H, l: (i16, i16)) -> Vec<GdsElement> {
    vec![
        rect(l, 3.0, 3.0, 4.0, 4.0),
        h.c(3.89, 3.4),
        h.around(h.m1, 3.89, 3.4, [0.4; 4]),
        rect(l, 8.0, 3.0, 9.0, 4.0),
        h.c(9.0, 3.4),
        h.around(h.m1, 9.0, 3.4, [0.4; 4]),
        rect(l, 12.935, 2.935, 13.7, 3.7),
        h.c(13.0, 3.0),
        h.around(h.m1, 13.0, 3.0, [0.4; 4]),
        rect(h.sram, 12.6, 2.6, 14.1, 4.1),
    ]
}

fn co_3_h(h: &H) {
    // h1 - the margin at the bound and at a corner.  (a) 0.07 all round, clean;
    // (b) 0.065 on one side; (c) the poly edge flush with the contact, a margin of
    // nothing; (d) a poly square whose top right corner is chamfered by 0.05, the
    // contact 0.07 from both straight edges - the chamfer's closest approach is 0.0636,
    // under the value; (e) the same with a 0.04 chamfer, 0.0707 away, clean.
    hwrite("CO.3.h1", enclosure_h(h, h.poly));

    // h2 - the conditions.  (a) a contact the poly edge cuts in half: it overlaps poly
    // and runs outside it, which no margin describes, and the part outside is on field
    // oxide; (b) a contact wholly outside the poly with its edge on the poly's - it is
    // not a contact to poly at all, it is a contact on field oxide; (c) h1's 0.065
    // margin under a SRAMCORE marker, which has its own rules and is exempt from this
    // one.
    hwrite("CO.3.h2", crossing_h(h, h.poly));
}

// --- CO.4: COMP overlap of contact 0.07 ---

fn co_4_h(h: &H) {
    // h1, h2 - the same five readings and three conditions against COMP.
    hwrite("CO.4.h1", enclosure_h(h, h.comp));
    hwrite("CO.4.h2", crossing_h(h, h.comp));
}

// --- CO.5a / CO.5b: Nplus / Pplus overlap of a contact on butted COMP 0.1 ---

/// A COMP bar 4 µm long whose left half is Nplus and right half Pplus, the two implants
/// meeting on a line: the butted arrangement the rule names.  `over` shifts the implant
/// boundary so the two overlap (positive) or leave a gap (negative).
fn butted(h: &H, x: f64, y: f64, over: f64) -> Vec<GdsElement> {
    vec![
        rect(h.comp, x, y, x + 4.0, y + 1.4),
        rect(h.np, x - 0.3, y - 0.3, x + 2.0 + over, y + 1.7),
        rect(h.pp, x + 2.0 - over, y - 0.3, x + 4.3, y + 1.7),
    ]
}

fn co_5_h(h: &H) {
    let with_co = |v: &mut Vec<GdsElement>, cx: f64, cy: f64| {
        v.push(h.c(cx, cy));
        v.push(h.around(h.m1, cx, cy, [0.4; 4]));
    };

    // h1 - the margin at the bound, on either side of the butting edge, and on a side
    // that is not the butting edge at all: the rule is the implant's overlap of the
    // contact, so it holds on every side of the N+ (or P+) COMP.  (a) 0.1 from the
    // butting edge on the N side, clean; (b) 0.095 there, CO.5a; (c) 0.095 on the P
    // side, CO.5b; (d) 0.5 from the butting edge but 0.095 from the bar's bottom edge -
    // COMP still encloses by more than CO.4's 0.07, and CO.5a fires.
    let mut v = butted(h, 3.0, 3.0, 0.0);
    with_co(&mut v, 5.0 - 0.1 - CO, 3.59);
    v.extend(butted(h, 3.0, 6.0, 0.0));
    with_co(&mut v, 5.0 - 0.095 - CO, 6.59);
    v.extend(butted(h, 3.0, 9.0, 0.0));
    with_co(&mut v, 5.0 + 0.095, 9.59);
    v.extend(butted(h, 3.0, 12.0, 0.0));
    with_co(&mut v, 5.0 - 0.5 - CO, 12.095);
    hwrite("CO.5.h1", v);

    // h2 - when the actives are not butted the rule does not apply at all.  (a) the two
    // implants overlapping by 0.2 over the COMP, a contact 0.095 from the N+ edge;
    // (b) a 0.2 gap between them, the contact 0.095 from the N+ edge; (c) the butted
    // arrangement at 0.095, the control that fires.
    let mut v = butted(h, 3.0, 3.0, 0.1);
    with_co(&mut v, 5.1 - 0.095 - CO, 3.59);
    v.extend(butted(h, 3.0, 6.0, -0.1));
    with_co(&mut v, 4.9 - 0.095 - CO, 6.59);
    v.extend(butted(h, 3.0, 9.0, 0.0));
    with_co(&mut v, 5.0 - 0.095 - CO, 9.59);
    hwrite("CO.5.h2", v);
}

// --- CO.6: Metal1 overlap of contact 0.005 ---

fn co_6_h(h: &H) {
    // h1 - what "Metal1 overlap of contact" asks.  (a) 0.005 on one side, at the bound,
    // clean; (b) the metal edge flush with the contact, a margin of nothing; (c) a
    // contact with no Metal1 over it at all - nothing to measure, and the contact goes
    // nowhere; (d) a Metal1 edge cutting the contact in half, so half of it is bare.
    hwrite(
        "CO.6.h1",
        vec![
            h.around(h.comp, 3.0, 3.0, [0.3; 4]),
            h.c(3.0, 3.0),
            h.around(h.m1, 3.0, 3.0, [0.005, 0.4, 0.4, 0.4]),
            h.around(h.comp, 8.0, 3.0, [0.3; 4]),
            h.c(8.0, 3.0),
            h.around(h.m1, 8.0, 3.0, [0.0, 0.4, 0.4, 0.4]),
            h.around(h.comp, 13.0, 3.0, [0.3; 4]),
            h.c(13.0, 3.0),
            h.around(h.comp, 3.0, 8.0, [0.3; 4]),
            h.c(3.0, 8.0),
            rect(h.m1, 3.11, 6.0, 5.11, 10.0),
        ],
    );
}

// --- CO.6a: Metal1 (< 0.34 µm) end-of-line overlap of contact 0.06 ---

fn co_6a_h(h: &H) {
    // A contact at the capped end of a Metal1 track `w` wide, the cap `tip` past it.
    let track = |x: f64, y: f64, w: f64, tip: f64| -> Vec<GdsElement> {
        let m = (w - CO) / 2.0;
        vec![
            h.around(h.comp, x, y, [0.3; 4]),
            h.c(x, y),
            rect(h.m1, x - tip, y - m, x + CO + 3.0, y + CO + m),
        ]
    };
    // h1 - which track is a line end.  The note says "< 0.34 µm wide", so a 0.34 µm
    // track is not one.  (a) 0.34 wide with the cap 0.06 past the contact, clean;
    // (b) 0.34 wide at 0.055, still not a narrow line, clean; (c) 0.33 wide at 0.055,
    // narrow, fires; (d) 0.30 wide at 0.055, fires; (e) 0.35 wide at 0.055, clean -
    // CO.6 asks only 0.005 of it.
    let mut v = track(3.0, 3.0, 0.34, 0.06);
    v.extend(track(3.0, 6.0, 0.34, 0.055));
    v.extend(track(3.0, 9.0, 0.30, 0.055));
    v.extend(track(3.0, 12.0, 0.35, 0.055));
    v.extend(track(3.0, 15.0, 0.33, 0.055));
    hwrite("CO.6a.h1", v);

    // h2 - the note's other half, "excluding metal branches shorter than 0.24 µm".  A
    // wide Metal1 plate with a 0.30 µm branch out of its left edge - narrow enough to be
    // a line, wide enough that its 0.04 margins do not trigger CO.6b - and the contact
    // at the branch's tip with 0.055 to the cap.  (a) a 0.22 µm branch, shorter than
    // 0.24, not a line end, clean; (b) 0.24 exactly, a line end, fires; (c) 0.30, fires.
    let branch = |x: f64, y: f64, len: f64| -> Vec<GdsElement> {
        let tip = x + 0.5 - len;
        vec![
            h.around(h.comp, tip + 0.055, y, [0.3; 4]),
            h.c(tip + 0.055, y),
            rect(h.m1, tip, y - 0.04, x + 0.5, y + CO + 0.04),
            rect(h.m1, x + 0.5, y - 0.5, x + 3.0, y + CO + 0.5),
        ]
    };
    let mut v = branch(3.0, 3.0, 0.22);
    v.extend(branch(3.0, 7.0, 0.24));
    v.extend(branch(3.0, 11.0, 0.30));
    hwrite("CO.6a.h2", v);
}

// --- CO.6b: if Metal1 overlaps a contact by < 0.04 on one side, the adjacent edges 0.06 ---

fn co_6b_h(h: &H) {
    // The metal is drawn large in the directions the case is not about, so the track is
    // never a narrow line and CO.6a has nothing to say.
    let cell = |x: f64, y: f64, m: [f64; 4]| -> Vec<GdsElement> {
        vec![
            h.around(h.comp, x, y, [0.3; 4]),
            h.c(x, y),
            h.around(h.m1, x, y, m),
        ]
    };
    // h1 - the trigger and the value.  (a) 0.035 on the left with 0.06 below it, at the
    // bound, clean; (b) 0.035 on the left with 0.055 below it, fires; (c) 0.04 on the
    // left is not "< 0.04", so it does not trigger and 0.055 below is clean; (d) two
    // adjacent sides both under 0.04 - each is the other's deficient neighbour, fires;
    // (e) the two opposite sides under 0.04 with the sides beside them generous, clean.
    let mut v = cell(3.0, 3.0, [0.035, 2.0, 0.06, 2.0]);
    v.extend(cell(9.0, 3.0, [0.035, 2.0, 0.055, 2.0]));
    v.extend(cell(15.0, 3.0, [0.04, 2.0, 0.055, 2.0]));
    v.extend(cell(3.0, 9.0, [0.035, 2.0, 0.035, 2.0]));
    v.extend(cell(9.0, 9.0, [0.035, 0.035, 2.0, 2.0]));
    hwrite("CO.6b.h1", v);
}

// --- CO.7: space from a COMP contact to Poly2 on COMP 0.15 ---

fn co_7_h(h: &H) {
    // A COMP bar with a poly finger crossing it at x + 2: the gate the rule measures to.
    // `cross` false puts the poly beside the COMP instead, where it is not a gate.
    let cell = |x: f64, y: f64, gap: f64, cross: bool, otp: Option<[f64; 4]>| {
        let px = if cross { x + 2.0 } else { x + 3.03 };
        let cx = px - gap - CO;
        let mut v = vec![
            rect(h.comp, x, y, x + 3.0, y + 1.0),
            rect(h.poly, px, y - 0.3, px + 0.3, y + 1.3),
            h.c(cx, y + 0.39),
            h.around(h.m1, cx, y + 0.39, [0.4; 4]),
        ];
        if let Some(m) = otp {
            v.push(rect(h.otp, m[0], m[1], m[2], m[3]));
        }
        v
    };
    // h1 - (a) 0.15 to the gate, at the bound, clean; (b) 0.145, fires; (c) 0.10 to a
    // poly line that runs beside the COMP and never over it, which is no gate at all -
    // the rule names Poly2 *on* COMP - clean; (d) 0.145 with an OTP_MK marker over the
    // whole cell, which the rule exempts; (e) 0.145 with the marker over the gate only,
    // which takes the gate out of the rule just the same.
    let mut v = cell(3.0, 3.0, 0.15, true, None);
    v.extend(cell(9.0, 3.0, 0.145, true, None));
    v.extend(cell(15.0, 3.0, 0.10, false, None));
    v.extend(cell(3.0, 9.0, 0.145, true, Some([2.5, 8.5, 6.5, 10.5])));
    v.extend(cell(9.0, 9.0, 0.145, true, Some([10.9, 8.5, 11.4, 10.5])));
    hwrite("CO.7.h1", v);
}

// --- CO.8: space from a Poly2 contact to COMP 0.17 ---

fn co_8_h(h: &H) {
    // h1 - (a) a contact on a poly bar with COMP 0.17 to its right, at the bound, clean;
    // (b) 0.165, fires; (c) the poly runs over the COMP - a gate - and the contact sits
    // on the poly outside the COMP, 0.165 from its edge: the rule measures to COMP,
    // whether or not the poly is a gate there, so it fires.
    let bar = |x: f64, y: f64, gap: f64| -> Vec<GdsElement> {
        vec![
            rect(h.poly, x, y, x + 0.85, y + 0.6),
            h.c(x + 0.5, y + 0.19),
            h.around(h.m1, x + 0.5, y + 0.19, [0.4; 4]),
            rect(h.comp, x + 0.72 + gap, y - 0.5, x + 2.0, y + 1.1),
        ]
    };
    let mut v = bar(3.0, 3.0, 0.17);
    v.extend(bar(9.0, 3.0, 0.165));
    v.extend(vec![
        rect(h.poly, 15.0, 3.0, 17.0, 3.6),
        rect(h.comp, 16.5, 2.5, 18.0, 4.1),
        h.c(16.5 - 0.165 - CO, 3.19),
        h.around(h.m1, 16.5 - 0.165 - CO, 3.19, [0.4; 4]),
    ]);
    hwrite("CO.8.h1", v);
}

// --- CO.9: a contact on the NCOMP/PCOMP butting edge is forbidden ---

fn co_9_h(h: &H) {
    let with_co = |v: &mut Vec<GdsElement>, cx: f64, cy: f64| {
        v.push(h.c(cx, cy));
        v.push(h.around(h.m1, cx, cy, [0.4; 4]));
    };
    // h1 - (a) a contact centred on the butting edge, straddling it; (b) a contact
    // wholly on the P side with its edge lying on the butting edge - it straddles
    // nothing, but it is 0 from the N+ implant, which CO.5b measures; (c) implants that
    // overlap by 0.2 instead of meeting: there is no butting edge, so a contact across
    // their boundary breaks neither CO.9 nor CO.5.
    let mut v = butted(h, 3.0, 3.0, 0.0);
    with_co(&mut v, 5.0 - CO / 2.0, 3.59);
    v.extend(butted(h, 3.0, 6.0, 0.0));
    with_co(&mut v, 5.0, 6.59);
    v.extend(butted(h, 3.0, 9.0, 0.1));
    with_co(&mut v, 5.0 - CO / 2.0, 9.59);
    hwrite("CO.9.h1", v);

    // h2 - the butting edge on the tile lines: it is derived from where two implants'
    // edges coincide, and a cut must not lose it.  Four straddling contacts, on butting
    // edges at x = 20, 21, 40 and 42.
    let mut v = Vec::new();
    for (i, bx) in [20.0, 21.0, 40.0, 42.0].into_iter().enumerate() {
        let y = 3.0 + 3.0 * i as f64;
        v.push(rect(h.comp, bx - 2.0, y, bx + 2.0, y + 1.4));
        v.push(rect(h.np, bx - 2.3, y - 0.3, bx, y + 1.7));
        v.push(rect(h.pp, bx, y - 0.3, bx + 2.3, y + 1.7));
        with_co(&mut v, bx - CO / 2.0, y + 0.59);
    }
    hwrite("CO.9.h2", v);
}

// --- CO.10: a contact on a Poly2 gate over COMP is forbidden ---

fn co_10_h(h: &H) {
    // A COMP bar with a poly finger over it, running past it above and below.
    let cell = |x: f64, y: f64, cy: f64, res: bool| -> Vec<GdsElement> {
        let mut v = vec![
            rect(h.comp, x, y, x + 2.0, y + 1.0),
            rect(h.poly, x + 0.8, y - 0.4, x + 1.4, y + 1.6),
            h.c(x + 0.99, cy),
            h.around(h.m1, x + 0.99, cy, [0.4; 4]),
        ];
        if res {
            v.push(rect(h.res, x + 0.7, y - 0.1, x + 1.5, y + 1.1));
        }
        v
    };
    // h1 - (a) a contact in the middle of the gate; (b) the same contact moved up the
    // poly, 0.17 clear of the COMP: on poly, not on a gate, clean; (c) its bottom edge
    // exactly on the COMP's top edge - it covers no part of the gate, so CO.10 has
    // nothing, but it is 0 from COMP and CO.8 asks 0.17; (d) a contact on the gate with
    // RES_MK over the whole poly-COMP overlap, which upstream takes out of the gate
    // layer.
    let mut v = cell(3.0, 3.0, 3.39, false);
    v.extend(cell(9.0, 3.0, 4.17, false));
    v.extend(cell(15.0, 3.0, 4.0, false));
    v.extend(cell(3.0, 9.0, 9.39, true));
    hwrite("CO.10.h1", v);
}

// --- CO.11: a contact on field oxide is forbidden ---

fn co_11_h(h: &H) {
    // h1 - (a) a contact on neither poly nor COMP; (b) one on poly alone, which is a
    // poly contact and legal; (c) one over COMP but for a 0.005 sliver on its left -
    // the sliver is on field oxide, and the contact crosses the COMP edge, which CO.4
    // has no margin for; (d) one covered half by COMP and half by an abutting poly: no
    // part of it is on field oxide, so CO.11 is silent, while it crosses both edges.
    hwrite(
        "CO.11.h1",
        vec![
            h.c(3.0, 3.0),
            h.around(h.m1, 3.0, 3.0, [0.4; 4]),
            rect(h.poly, 7.7, 2.7, 8.7, 3.7),
            h.c(8.0, 3.0),
            h.around(h.m1, 8.0, 3.0, [0.4; 4]),
            rect(h.comp, 13.0, 2.7, 14.0, 3.7),
            h.c(12.995, 3.0),
            h.around(h.m1, 12.995, 3.0, [0.4; 4]),
            rect(h.comp, 3.0, 7.7, 4.0, 8.7),
            rect(h.poly, 4.0, 7.7, 5.0, 8.7),
            h.c(4.0 - CO / 2.0, 8.0),
            h.around(h.m1, 4.0 - CO / 2.0, 8.0, [0.4; 4]),
        ],
    );
}
