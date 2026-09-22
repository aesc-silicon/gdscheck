// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! High-sheet poly resistor: a good and a bad pattern for every rule in the `hres` deck.
//!
//! The third resistor here, and the one with an extra layer: where the P+ and N+ bodies
//! are recognised by their implant, this one carries a RESISTOR marker as well, and half
//! its rules are about that marker rather than about the poly.  So the base cell is
//! `pres`'s with the marker added, and rules that used to move the implant move the
//! marker instead.
//!
//! Every fixture is one resistor, perturbed one rule at a time.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/hres";

/// Body length, and a width over `HRES.2`'s 1.0 µm minimum.
const LEN: f64 = 8.0;
const WIDTH: f64 = 1.2;
/// How far each marker and implant reaches past the body.  RESISTOR's 0.5 clears
/// `HRES.4`'s 0.4, SAB's 0.4 clears `HRES.9`'s 0.28, and RES_MK sits outside both so no
/// edge of it falls in the poly, which is what `HRES.12a` looks for.
const PP: f64 = 0.5;
const RES: f64 = 0.5;
const SAB_W: f64 = 0.4;
const MK: f64 = 0.7;
/// Where the SAB starts along the body, leaving the contact heads salicided.
const SAB_IN: f64 = 2.0;
/// Contact side, and its clearance from the SAB - `HRES.8` asks 0.22.
const CONT: f64 = 0.22;
const CONT_GAP: f64 = 0.5;

struct Ctx {
    poly: (i16, i16),
    pplus: (i16, i16),
    sab: (i16, i16),
    res_mk: (i16, i16),
    resistor: (i16, i16),
    contact: (i16, i16),
    comp: (i16, i16),
}

/// One resistor, every margin overridable so a rule's bad half moves exactly one of them.
struct Cell {
    /// How far the implant reaches past the body.  A fixture that pulls the block in
    /// moves this with it, so the 0.1 µm overlap HRES.10 fixes is left where it was.
    pp: f64,
    x: f64,
    y: f64,
    width: f64,
    res: f64,
    sab_w: f64,
    cont_gap: f64,
    mk: f64,
    /// How far the P+ implant's left edge sits from the left contact, for `HRES.7`.
    pp_left: Option<f64>,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            pp: PP,
            x,
            y,
            width: WIDTH,
            res: RES,
            sab_w: SAB_W,
            cont_gap: CONT_GAP,
            mk: MK,
            pp_left: None,
        }
    }

    fn contacts(&self) -> [f64; 2] {
        [
            self.x + SAB_IN - self.cont_gap - CONT,
            self.x + LEN - SAB_IN + self.cont_gap,
        ]
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y, w) = (self.x, self.y, self.width);
        let pp_x0 = match self.pp_left {
            Some(margin) => self.contacts()[0] - margin,
            None => x - self.pp,
        };
        let mut v = vec![
            rect(c.poly, x, y, x + LEN, y + w),
            rect(
                c.pplus,
                pp_x0,
                y - self.pp,
                x + LEN + self.pp,
                y + w + self.pp,
            ),
            rect(
                c.sab,
                x + SAB_IN,
                y - self.sab_w,
                x + LEN - SAB_IN,
                y + w + self.sab_w,
            ),
            rect(
                c.resistor,
                x - self.res,
                y - self.res,
                x + LEN + self.res,
                y + w + self.res,
            ),
            rect(
                c.res_mk,
                x - self.mk,
                y - self.mk,
                x + LEN + self.mk,
                y + w + self.mk,
            ),
        ];
        for cx in self.contacts() {
            let cy = y + (w - CONT) * 0.5;
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v
    }
}

/// A RES_MK big enough for HRES.12b: over 15000 µm² with both sides over 80 µm.
fn big_marker(c: &Ctx, x: f64, y: f64) -> GdsElement {
    rect(c.res_mk, x, y, x + 130.0, y + 130.0)
}

/// Its neighbour, under the area bound, so only one end of the pair qualifies and the gap
/// is one violation rather than two.
fn small_marker(c: &Ctx, x: f64, y: f64) -> GdsElement {
    rect(c.res_mk, x, y, x + 121.0, y + 123.0)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        pplus: layer(pdk, "pplus"),
        sab: layer(pdk, "sab"),
        res_mk: layer(pdk, "res_mk"),
        resistor: layer(pdk, "resistor"),
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let clean = || Cell::at(o, o).draw(&c);

    // HRES.10 fixes how far the implant reaches past the block at 0.1 µm, and the clean
    // cell already sits there - the implant runs 0.5 µm past the poly and the block 0.4.
    // Pulling the block in widens the overlap without moving the implant.
    let overlap = |sab_w: f64| {
        let mut cell = Cell::at(o, o);
        cell.sab_w = sab_w;
        cell.draw(&c)
    };

    write("HRES.10", "bad", overlap(0.395));

    for id in [
        "HRES.1", "HRES.2", "HRES.3", "HRES.4", "HRES.5", "HRES.6", "HRES.7", "HRES.8", "HRES.9",
        "HRES.10", "HRES.12a",
    ] {
        write(id, "good", clean());
    }
    write("HRES.12b", "good", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 20.1, o)]
    });

    // HRES.1: two devices whose RESISTOR markers come within 0.4 µm.  Their bodies stay a
    // marker's reach further apart than that, so the poly spacing has nothing to say.
    write("HRES.1", "bad", {
        let mut v = clean();
        v.extend(Cell::at(o, o + WIDTH + 2.0 * RES + 0.395).draw(&c));
        v
    });

    // HRES.2: a body under the 1.0 µm minimum.
    write("HRES.2", "bad", {
        let mut cell = Cell::at(o, o);
        cell.width = 0.995;
        cell.draw(&c)
    });

    // HRES.3: two bodies within 0.4 µm.  Their markers overlap and merge, so the marker
    // spacing is not in play.
    write("HRES.3", "bad", {
        let mut v = clean();
        v.extend(Cell::at(o, o + WIDTH + 0.395).draw(&c));
        v
    });

    // HRES.4: the RESISTOR marker reaching only 0.395 µm past the body.
    write("HRES.4", "bad", {
        let mut cell = Cell::at(o, o);
        cell.res = 0.395;
        cell.draw(&c)
    });

    // HRES.5: unrelated Poly2 - no SAB, no marker - within 0.3 µm of the marker.
    write("HRES.5", "bad", {
        let mut v = clean();
        let x = o + LEN + RES + 0.295;
        v.push(rect(c.poly, x, o, x + 2.0, o + WIDTH));
        v
    });

    // HRES.6: a COMP within 0.3 µm of the marker.
    write("HRES.6", "bad", {
        let mut v = clean();
        let x = o + LEN + RES + 0.295;
        v.push(rect(c.comp, x, o, x + 2.0, o + WIDTH));
        v
    });

    // HRES.7: the P+ implant enclosing a contact head by only 0.195 µm.
    write("HRES.7", "bad", {
        let mut cell = Cell::at(o, o);
        cell.pp_left = Some(0.195);
        cell.draw(&c)
    });

    // HRES.8: a contact head 0.215 µm from the SAB.
    write("HRES.8", "bad", {
        let mut cell = Cell::at(o, o);
        cell.cont_gap = 0.215;
        cell.draw(&c)
    });

    // HRES.9: the SAB overhanging the body by only 0.275 µm in the width direction.  The
    // implant comes in with it: leaving the implant where it was would widen its reach
    // past the block, which is HRES.10's business and not this rule's.
    write("HRES.9", "bad", {
        let mut cell = Cell::at(o, o);
        cell.sab_w = 0.275;
        cell.pp = 0.375;
        cell.draw(&c)
    });

    // HRES.12a: a RES_MK that stops short, so its edge runs across the body instead of
    // following the resistor's outline.
    write("HRES.12a", "bad", {
        let mut v = clean();
        v.retain(|e| !matches!(e, GdsElement::GdsBoundary(b) if (b.layer, b.datatype) == c.res_mk));
        v.push(rect(
            c.res_mk,
            o - MK,
            o - MK,
            o + LEN * 0.5,
            o + WIDTH + MK,
        ));
        v
    });

    // HRES.12b: the same two large markers, now under 20 µm apart.
    write("HRES.12b", "bad", {
        vec![big_marker(&c, o, o), small_marker(&c, o + 130.0 + 19.9, o)]
    });

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) --------------------------
//
// The pairs above are drawn as one bar with the P+ implant over the whole of it.  A real
// high-sheet resistor is not: the manual says its length is "determined by Pplus to Pplus
// space on Poly2", so the implant comes in from each end, stops 0.1 µm over the salicide
// block and leaves the body between the two heads bare.  That is the shape HRES.10 fixes,
// the shape HRES.12a's marking has to coincide with, and the shape the layouts below are
// built from.

/// The bar: 8 µm long and 1.2 µm wide, over HRES.2's 1.0 µm.
const HLEN: f64 = 8.0;
const HW: f64 = 1.2;
/// Where the block starts and ends along the bar, and how far it overhangs in the width
/// direction - over HRES.9's 0.28.
const HSAB0: f64 = 2.0;
const HSAB1: f64 = 6.0;
const HSABW: f64 = 0.4;
/// How far each P+ head reaches outward past the bar, and how far it reaches over the
/// block: HRES.10 fixes the second at 0.1 from both sides.
const HPP: f64 = 0.5;
const HOV: f64 = 0.1;
/// The RESISTOR marker's margin - over HRES.4's 0.4 - and RES_MK's, when the marking is
/// not the thing under test.
const HMARK: f64 = 0.5;
const HMK: f64 = 0.7;
const HCONT: f64 = 0.22;

/// The RES_MK marking of one device.
enum HMk {
    None,
    /// A box `d` clear of the bar on every side, so no edge of it falls in the Poly2.
    Clear(f64),
    Box([f64; 4]),
}

/// One high-sheet resistor: every layer it is recognised by moves on its own, so a
/// fixture perturbs exactly the thing its rule is about.
struct HDev {
    x: f64,
    y: f64,
    w: f64,
    /// How far each P+ head reaches over the block.  Negative pulls it clear instead.
    ov_l: f64,
    ov_r: f64,
    /// An explicit P+, for a head drawn in more than one piece.
    pp: Option<Vec<[f64; 4]>>,
    /// An explicit block, for one with a hole or a notch in it; empty for no block.
    sab: Option<Vec<[f64; 4]>>,
    sab_top: f64,
    sab_bot: f64,
    mk: HMk,
    /// The RESISTOR marker: a margin past the bar, explicit boxes, or none at all.
    res: Option<f64>,
    res_box: Option<Vec<[f64; 4]>>,
    /// Where each contact sits along the bar.
    conts: Vec<f64>,
}

impl HDev {
    fn at(x: f64, y: f64) -> Self {
        HDev {
            x,
            y,
            w: HW,
            ov_l: HOV,
            ov_r: HOV,
            pp: None,
            sab: None,
            sab_top: HSABW,
            sab_bot: HSABW,
            mk: HMk::Clear(HMK),
            res: Some(HMARK),
            res_box: None,
            conts: vec![1.0, 6.9],
        }
    }

    fn w(mut self, w: f64) -> Self {
        self.w = w;
        self
    }

    /// The body: the stretch of bar between the two heads, which is what the marking is
    /// asked to coincide with.
    fn body(&self) -> (f64, f64) {
        (self.x + HSAB0 + self.ov_l, self.x + HSAB1 - self.ov_r)
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y, w) = (self.x, self.y, self.w);
        let mut v = vec![rect(c.poly, x, y, x + HLEN, y + w)];
        match &self.pp {
            Some(bs) => {
                for b in bs {
                    v.push(rect(c.pplus, b[0], b[1], b[2], b[3]));
                }
            }
            None => {
                let (l, r) = self.body();
                v.push(rect(c.pplus, x - HPP, y - HPP, l, y + w + HPP));
                v.push(rect(c.pplus, r, y - HPP, x + HLEN + HPP, y + w + HPP));
            }
        }
        match &self.sab {
            Some(bs) => {
                for b in bs {
                    v.push(rect(c.sab, b[0], b[1], b[2], b[3]));
                }
            }
            None => v.push(rect(
                c.sab,
                x + HSAB0,
                y - self.sab_bot,
                x + HSAB1,
                y + w + self.sab_top,
            )),
        }
        match self.mk {
            HMk::None => {}
            HMk::Clear(d) => v.push(rect(c.res_mk, x - d, y - d, x + HLEN + d, y + w + d)),
            HMk::Box([x0, y0, x1, y1]) => v.push(rect(c.res_mk, x0, y0, x1, y1)),
        }
        match &self.res_box {
            Some(bs) => {
                for b in bs {
                    v.push(rect(c.resistor, b[0], b[1], b[2], b[3]));
                }
            }
            None => {
                if let Some(d) = self.res {
                    v.push(rect(c.resistor, x - d, y - d, x + HLEN + d, y + w + d));
                }
            }
        }
        let cy = y + (w - HCONT) * 0.5;
        for dx in &self.conts {
            v.push(rect(c.contact, x + dx, cy, x + dx + HCONT, cy + HCONT));
        }
        v
    }
}

/// An octagon 140 µm across in x and in y, chamfered by 41: 16238 µm², both sides over
/// 80 µm, and not one edge of it 80 µm long.
fn octagon(l: (i16, i16), x: f64, y: f64) -> GdsElement {
    let (n, k) = (140.0, 41.0);
    crate::helpers::poly(
        l,
        &[
            (x + k, y),
            (x + n - k, y),
            (x + n, y + k),
            (x + n, y + n - k),
            (x + n - k, y + n),
            (x + k, y + n),
            (x, y + n - k),
            (x, y + k),
        ],
    )
}

fn hardening(pdk: &PdkConfig) {
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        pplus: layer(pdk, "pplus"),
        sab: layer(pdk, "sab"),
        res_mk: layer(pdk, "res_mk"),
        resistor: layer(pdk, "resistor"),
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
    };
    let write = |name: &str, elems: Vec<GdsElement>| {
        write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
    };
    let o = OFFSET;

    // HRES.10.h1: the minimum half of the P+ overlap of the block.  Two devices: the left
    // head reaching 0.1 µm over the block, and one reaching 0.095.
    write("HRES.10.h1", {
        let mut v = HDev::at(o, o).draw(&c);
        let mut a = HDev::at(o, o + 5.0);
        a.ov_l = 0.095;
        v.extend(a.draw(&c));
        v
    });

    // HRES.10.h3: the maximum half of the same rule, on the same two devices - the left
    // head reaching 0.105 µm over the block instead of 0.095.  "Minimum & maximum Pplus
    // overlap of SAB is 0.1" fixes the overlap from both sides, and this one is over.
    write("HRES.10.h3", {
        let mut v = HDev::at(o, o).draw(&c);
        let mut a = HDev::at(o, o + 5.0);
        a.ov_l = 0.105;
        v.extend(a.draw(&c));
        v
    });

    // HRES.10.h2: the left head stopping 0.1 µm short of the block instead of reaching
    // 0.1 µm over it.  An overlap of nothing is under the minimum the rule names.
    write("HRES.10.h2", {
        let mut d = HDev::at(o, o);
        d.ov_l = -0.1;
        d.draw(&c)
    });

    // HRES.9.h1: a 0.5 x 0.4 µm hole in the block over the middle of the body.  The
    // salicide block overlaps the resistor by nothing there.
    write("HRES.9.h1", {
        let mut d = HDev::at(o, o);
        d.sab = Some(vec![
            [o + HSAB0, o - HSABW, o + HSAB1, o + 0.4],
            [o + HSAB0, o + 0.8, o + HSAB1, o + HW + HSABW],
            [o + HSAB0, o + 0.4, o + 3.5, o + 0.8],
            [o + 4.0, o + 0.4, o + HSAB1, o + 0.8],
        ]);
        d.draw(&c)
    });

    // HRES.9.h2: the block overhanging 0.275 µm above the bar and 0.28 below it.
    write("HRES.9.h2", {
        let mut d = HDev::at(o, o);
        d.sab_top = 0.275;
        d.sab_bot = 0.28;
        d.draw(&c)
    });

    // HRES.9.h3: a notch bitten out of the block from above, ending inside the bar: over
    // that half micron the block covers the bottom of the resistor and stops 0.4 µm short
    // of its top edge.
    write("HRES.9.h3", {
        let mut d = HDev::at(o, o);
        d.sab = Some(vec![
            [o + HSAB0, o - HSABW, o + 3.5, o + HW + HSABW],
            [o + 4.0, o - HSABW, o + HSAB1, o + HW + HSABW],
            [o + 3.5, o - HSABW, o + 4.0, o + 0.8],
        ]);
        d.draw(&c)
    });

    // HRES.4.h1: the RESISTOR marker running out across the bar - it covers the left half
    // and stops, so the rest of the resistor has no marker over it at all.
    write("HRES.4.h1", {
        let mut d = HDev::at(o, o);
        d.res_box = Some(vec![[o - HMARK, o - HMARK, o + 4.0, o + HW + HMARK]]);
        d.draw(&c)
    });

    // HRES.2.h1: what makes a bar a high-sheet resistor.  Five bars of 0.995 µm, under the
    // 1.0 minimum: the full device; without the RESISTOR marker; without RES_MK; without
    // the block; and one whose P+ only abuts the bar's left edge and covers none of it -
    // this section asks the Poly2 to *touch* the implant where 10.1 asks for Poly2 and
    // Pplus, so a bar with no implant on it at all is still one of these resistors.
    write("HRES.2.h1", {
        let mut v = HDev::at(o, o).w(0.995).draw(&c);
        let mut a = HDev::at(o, o + 5.0).w(0.995);
        a.res = None;
        v.extend(a.draw(&c));
        let mut b = HDev::at(o, o + 10.0).w(0.995);
        b.mk = HMk::None;
        v.extend(b.draw(&c));
        let mut d = HDev::at(o, o + 15.0).w(0.995);
        d.sab = Some(Vec::new());
        v.extend(d.draw(&c));
        let mut e = HDev::at(o, o + 20.0).w(0.995);
        e.pp = Some(vec![[o - 2.0, o + 20.0 - HPP, o, o + 20.0 + 0.995 + HPP]]);
        v.extend(e.draw(&c));
        v
    });

    // HRES.2.h2: the same 0.995 bar on the tile lines - across x = 20, across x = 40 and
    // 42, and across y = 20.
    write("HRES.2.h2", {
        let mut a = HDev::at(17.0, 10.0).w(0.995);
        a.conts = Vec::new();
        let mut b = HDev::at(38.0, 10.0).w(0.995);
        b.conts = Vec::new();
        let mut d = HDev::at(15.0, 19.5).w(0.995);
        d.conts = Vec::new();
        let mut v = a.draw(&c);
        v.extend(b.draw(&c));
        v.extend(d.draw(&c));
        v
    });

    // HRES.3.h1: the serpentine.  One resistor folded back on itself, its two arms 0.395
    // µm apart - the space the rule names, inside one polygon.
    write("HRES.3.h1", {
        let g = 0.395;
        let top = o + 2.0 * HW + g;
        let mut v = vec![
            rect(c.poly, o, o, o + HLEN, o + HW),
            rect(c.poly, o, o + HW + g, o + HLEN, top),
            rect(c.poly, o, o, o + 1.2, top),
            rect(c.pplus, o - HPP, o - HPP, o + HSAB0 + HOV, top + HPP),
            rect(c.pplus, o + HSAB1 - HOV, o - HPP, o + HLEN + HPP, top + HPP),
            rect(c.sab, o + HSAB0, o - HSABW, o + HSAB1, top + HSABW),
            rect(
                c.resistor,
                o - HMARK,
                o - HMARK,
                o + HLEN + HMARK,
                top + HMARK,
            ),
            rect(c.res_mk, o - HMK, o - HMK, o + HLEN + HMK, top + HMK),
        ];
        let cy = o + (HW - HCONT) * 0.5;
        v.push(rect(c.contact, o + 1.0, cy, o + 1.0 + HCONT, cy + HCONT));
        v
    });

    // HRES.1.h1: a 0.395 µm notch in the RESISTOR marker itself, 0.2 µm deep and 0.5 µm
    // clear of the bar, so the marker still covers the resistor by more than HRES.4 asks.
    write("HRES.1.h1", {
        let mut d = HDev::at(o, o);
        d.res_box = Some(vec![
            [o - HMARK, o - HMARK, o + 3.0, o + HW + 0.7],
            [o + 3.395, o - HMARK, o + HLEN + HMARK, o + HW + 0.7],
            [o - HMARK, o - HMARK, o + HLEN + HMARK, o + HW + HMARK],
        ]);
        d.draw(&c)
    });

    // HRES.5.h1: unrelated Poly2 against the RESISTOR marker.  A bare bar 0.295 µm off the
    // marker's right edge (fires), one at 0.3, and one at 0.295 whose far end touches a
    // salicide block of its own - which is all it takes for the deck to stop calling it
    // unrelated.
    write("HRES.5.h1", {
        let mut v = HDev::at(o, o).draw(&c);
        v.push(rect(c.poly, o + 8.795, o, o + 10.795, o + HW));
        v.extend(HDev::at(o, o + 6.0).draw(&c));
        v.push(rect(c.poly, o + 8.8, o + 6.0, o + 10.8, o + 6.0 + HW));
        v.extend(HDev::at(o, o + 12.0).draw(&c));
        v.push(rect(c.poly, o + 8.795, o + 12.0, o + 10.795, o + 12.0 + HW));
        v.push(rect(c.sab, o + 10.795, o + 12.3, o + 11.6, o + 12.9));
        v
    });

    // HRES.6.h1: COMP against the RESISTOR marker.  One 0.295 µm off its right edge
    // (fires), one at 0.3, and one lying under the marker but clear of the bar - a space
    // of nothing.
    write("HRES.6.h1", {
        let mut v = HDev::at(o, o).draw(&c);
        v.push(rect(c.comp, o + 8.795, o, o + 10.795, o + HW));
        v.extend(HDev::at(o, o + 6.0).draw(&c));
        v.push(rect(c.comp, o + 8.8, o + 6.0, o + 10.8, o + 6.0 + HW));
        v.extend(HDev::at(o, o + 12.0).draw(&c));
        v.push(rect(c.comp, o + 7.0, o + 13.4, o + 8.0, o + 13.6));
        v
    });

    // HRES.7.h1: a contact half on the implant.  The left head is an L - it runs the
    // whole way to its 0.1 µm over the block below y + 0.6 and stops at x + 1.1 above it -
    // so the contact's top-right corner sticks out of the implant while the head itself
    // overlaps the block exactly as HRES.10 asks.
    write("HRES.7.h1", {
        let mut d = HDev::at(o, o);
        d.pp = Some(vec![
            [o - HPP, o - HPP, o + HSAB0 + HOV, o + 0.6],
            [o - HPP, o + 0.6, o + 1.1, o + HW + HPP],
            [o + HSAB1 - HOV, o - HPP, o + HLEN + HPP, o + HW + HPP],
        ]);
        d.draw(&c)
    });

    // HRES.8.h1: a contact on the resistor's body, wholly inside the salicide block, with
    // the two head contacts where they belong.
    write("HRES.8.h1", {
        let mut d = HDev::at(o, o);
        d.conts = vec![1.0, 3.0, 6.9];
        d.draw(&c)
    });

    // HRES.8.h2: a contact on the head abutting the block's edge.  It also ends 0.1 µm
    // from the implant's edge, which is where HRES.10 puts that edge, so this layout
    // breaks two rules at once.
    write("HRES.8.h2", {
        let mut d = HDev::at(o, o);
        d.conts = vec![HSAB0 - HCONT, 6.9];
        d.draw(&c)
    });

    // HRES.12a.h1: what the marking has to coincide with - "resistor length (defined by
    // Pplus space) and width covering the width of Poly2".  Three devices: RES_MK on the
    // body exactly; 0.6 µm narrow, so its long edges run inside the Poly2; and 0.9 µm
    // short at each end, with 0.3 of width to spare.
    write("HRES.12a.h1", {
        let mut v = Vec::new();
        for (i, narrow, short) in [(0usize, 0.0, 0.0), (1, 0.3, 0.0), (2, -0.3, 0.9)] {
            let y = o + 6.0 * i as f64;
            let mut d = HDev::at(o, y);
            let (l, r) = d.body();
            d.mk = HMk::Box([l + short, y + narrow, r - short, y + HW - narrow]);
            v.extend(d.draw(&c));
        }
        v
    });

    // HRES.12b.h1: an octagon 140 µm across in x and in y - over 15000 µm², both sides
    // over 80 - with a 100 µm square marking 19.9 µm off its right side.  The square is
    // under the area bound, so one end of the pair qualifies and the gap is one violation.
    write("HRES.12b.h1", {
        vec![
            octagon(c.res_mk, 10.0, 10.0),
            rect(c.res_mk, 169.9, 10.0, 269.9, 110.0),
        ]
    });
}
