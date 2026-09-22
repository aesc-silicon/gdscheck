// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening patterns for the two salicide-blocked poly resistors, `pres` (manual 10.1)
//! and `lres` (10.2).
//!
//! The two sections are the same nine rules word for word with the implant swapped, so
//! every layout is drawn once here and written into both decks' directories with the
//! implant layer and the rule prefix exchanged.  Where the two decks disagree the layout
//! says so on its own: the deck's own recognition is `Poly2 and implant, touching a
//! salicide block, touching RES_MK` - and, for P+ only, *not* touching RESISTOR.
//!
//! What these ask is not the checks (the engine family has the bound, the metrics, the
//! 45° walls and the arrays) but the deck's conditions: what makes a poly bar a resistor
//! at all, where its body ends and its heads begin, what the marking has to coincide
//! with, and which neighbour counts as unrelated.

use super::OFFSET;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

/// Which of the two resistors a layout is drawn for.
#[derive(Clone, Copy)]
pub enum Kind {
    /// 10.1, the P+ resistor: `pres`, Pplus implant, and RESISTOR disqualifies it.
    P,
    /// 10.2, the N+ resistor: `lres`, Nplus implant.
    N,
}

impl Kind {
    fn dir(self) -> &'static str {
        match self {
            Kind::P => "tests/data/gf180mcuD/generated/pres",
            Kind::N => "tests/data/gf180mcuD/generated/lres",
        }
    }

    fn rule(self) -> &'static str {
        match self {
            Kind::P => "PRES",
            Kind::N => "LRES",
        }
    }

    fn implant(self) -> &'static str {
        match self {
            Kind::P => "pplus",
            Kind::N => "nplus",
        }
    }
}

struct Ctx {
    poly: (i16, i16),
    implant: (i16, i16),
    sab: (i16, i16),
    res_mk: (i16, i16),
    resistor: (i16, i16),
    contact: (i16, i16),
    comp: (i16, i16),
}

/// The body of the drawn resistor: 6 µm long, 1 µm wide - over the 0.8 µm minimum.
const LEN: f64 = 6.0;
const W: f64 = 1.0;
/// Implant margin past the body (the rule asks 0.3) and the block's overhang in the
/// width direction (0.28).
const IMP: f64 = 0.4;
const SABW: f64 = 0.35;
/// How far the block is inset from each end of the body, leaving the heads salicided.
const SAB_IN: f64 = 1.4;
/// How far the marking clears the body when it is not the thing under test.
const MK: f64 = 0.3;
/// Contact side, and the clearance from the block the rule asks for (0.22).
const CONT: f64 = 0.22;
const CONT_GAP: f64 = 0.5;

/// The RES_MK marking of one device.
enum Mk {
    /// No marking at all: without it the bar is not a resistor.
    None,
    /// A box `d` clear of the body on every side, so no edge of it falls in the Poly2.
    Clear(f64),
    /// An explicit box, for the rule that asks the marking to coincide with the body.
    Box([f64; 4]),
}

/// The implant of one device.
enum Imp {
    /// A box `d` past the body on every side.
    Round(f64),
    /// An explicit box, for an implant that runs out across the body.
    Box([f64; 4]),
}

/// The salicide block of one device.
enum Sab {
    None,
    /// Inset `SAB_IN` from each end of the body, overhanging `top`/`bot` in the width
    /// direction.
    Std {
        top: f64,
        bot: f64,
    },
    Box([f64; 4]),
}

/// One resistor.  Every layer it is recognised by can be moved on its own, so a fixture
/// perturbs exactly the thing its rule is about.
struct Dev {
    x: f64,
    y: f64,
    len: f64,
    w: f64,
    imp: Imp,
    sab: Sab,
    mk: Mk,
    /// A RESISTOR marker `d` past the body: what makes the bar a high-sheet resistor
    /// instead, and so not this deck's.
    resistor: Option<f64>,
    /// The clearance each contact head keeps from the block; empty for no contacts.
    conts: Vec<f64>,
}

impl Dev {
    fn at(x: f64, y: f64) -> Self {
        Dev {
            x,
            y,
            len: LEN,
            w: W,
            imp: Imp::Round(IMP),
            sab: Sab::Std {
                top: SABW,
                bot: SABW,
            },
            mk: Mk::Clear(MK),
            resistor: None,
            conts: vec![CONT_GAP, CONT_GAP],
        }
    }

    fn width(mut self, w: f64) -> Self {
        self.w = w;
        self
    }

    fn no_conts(mut self) -> Self {
        self.conts = Vec::new();
        self
    }

    fn mk(mut self, mk: Mk) -> Self {
        self.mk = mk;
        self
    }

    fn sab(mut self, sab: Sab) -> Self {
        self.sab = sab;
        self
    }

    fn imp(mut self, imp: Imp) -> Self {
        self.imp = imp;
        self
    }

    /// The block's own span along the body, which is what the marking has to coincide
    /// with: the manual reads the resistor's length off the block.
    fn sab_span(&self) -> (f64, f64) {
        match self.sab {
            Sab::Box([x0, _, x1, _]) => (x0, x1),
            _ => (self.x + SAB_IN, self.x + self.len - SAB_IN),
        }
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y, w, len) = (self.x, self.y, self.w, self.len);
        let mut v = vec![rect(c.poly, x, y, x + len, y + w)];
        match self.imp {
            Imp::Round(d) => v.push(rect(c.implant, x - d, y - d, x + len + d, y + w + d)),
            Imp::Box([x0, y0, x1, y1]) => v.push(rect(c.implant, x0, y0, x1, y1)),
        }
        match self.sab {
            Sab::None => {}
            Sab::Std { top, bot } => v.push(rect(
                c.sab,
                x + SAB_IN,
                y - bot,
                x + len - SAB_IN,
                y + w + top,
            )),
            Sab::Box([x0, y0, x1, y1]) => v.push(rect(c.sab, x0, y0, x1, y1)),
        }
        match self.mk {
            Mk::None => {}
            Mk::Clear(d) => v.push(rect(c.res_mk, x - d, y - d, x + len + d, y + w + d)),
            Mk::Box([x0, y0, x1, y1]) => v.push(rect(c.res_mk, x0, y0, x1, y1)),
        }
        if let Some(d) = self.resistor {
            v.push(rect(c.resistor, x - d, y - d, x + len + d, y + w + d));
        }
        let (s0, s1) = self.sab_span();
        let cy = y + (w - CONT) * 0.5;
        for (i, gap) in self.conts.iter().enumerate() {
            let cx = if i == 0 { s0 - gap - CONT } else { s1 + gap };
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v
    }
}

/// A U opening to the right: two arms `W` wide joined on the left, with a notch `g` wide
/// and 5 µm deep between them.  A serpentine resistor is drawn exactly like this, and the
/// gap between its arms is the space the rule is about.
fn u_poly(c: &Ctx, x: f64, y: f64, g: f64) -> Vec<GdsElement> {
    vec![
        rect(c.poly, x, y, x + LEN, y + W),
        rect(c.poly, x, y + W + g, x + LEN, y + 2.0 * W + g),
        rect(c.poly, x, y, x + 1.0, y + 2.0 * W + g),
    ]
}

/// The implant, block and marking of the U above, drawn over its whole bounding box so
/// only the notch is under test.
fn u_context(c: &Ctx, x: f64, y: f64, g: f64) -> Vec<GdsElement> {
    let top = y + 2.0 * W + g;
    vec![
        rect(c.implant, x - IMP, y - IMP, x + LEN + IMP, top + IMP),
        rect(c.sab, x + SAB_IN, y - SABW, x + LEN - SAB_IN, top + SABW),
        rect(c.res_mk, x - MK, y - MK, x + LEN + MK, top + MK),
    ]
}

/// An octagon: `n` µm across in both x and y with each corner chamfered by `k`.  Its X
/// and Y sides are both `n`, which is what the marking rule asks about, but no single
/// edge of it is as long as the 80 µm the foundry's deck measures.
fn octagon(l: (i16, i16), x: f64, y: f64, n: f64, k: f64) -> GdsElement {
    poly(
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

pub fn hardening(pdk: &PdkConfig, kind: Kind) {
    let dir = kind.dir();
    std::fs::create_dir_all(dir).expect("pattern dir");
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        implant: layer(pdk, kind.implant()),
        sab: layer(pdk, "sab"),
        res_mk: layer(pdk, "res_mk"),
        resistor: layer(pdk, "resistor"),
        contact: layer(pdk, "contact"),
        comp: layer(pdk, "comp"),
    };
    let r = kind.rule();
    let write = |name: String, elems: Vec<GdsElement>| {
        write_gz(&format!("{dir}/{name}.gds.gz"), library("TOP", elems));
    };
    let o = OFFSET;

    // .1.h1: what makes a bar a resistor.  Four bars of 0.795 µm, under the 0.8 µm
    // minimum, one above the other: the full device (fires); the same without RES_MK;
    // without the salicide block; and the full device with a RESISTOR marker over it.
    // The last is the one place the two sections part - a P+ bar under RESISTOR is a
    // high-sheet resistor and 10.1 lets it go, while 10.2 says nothing of the kind.
    write(format!("{r}.1.h1"), {
        let mut v = Dev::at(o, o).width(0.795).draw(&c);
        v.extend(Dev::at(o, o + 8.0).width(0.795).mk(Mk::None).draw(&c));
        v.extend(Dev::at(o, o + 16.0).width(0.795).sab(Sab::None).draw(&c));
        let mut hi = Dev::at(o, o + 24.0).width(0.795);
        hi.resistor = Some(0.5);
        v.extend(hi.draw(&c));
        v
    });

    // .1.h2: the marking that only touches.  Top: a 0.795 bar whose RES_MK abuts its left
    // edge and covers none of it.  Bottom: the same marking pulled 0.005 clear.  The
    // manual says the resistor "shall be covered by RES_MK marking"; a marking that
    // shares one edge covers nothing.
    write(format!("{r}.1.h2"), {
        let mut v = Dev::at(o, o)
            .width(0.795)
            .mk(Mk::Box([o - 2.0, o - 0.3, o, o + 0.795 + 0.3]))
            .draw(&c);
        let y = o + 8.0;
        v.extend(
            Dev::at(o, y)
                .width(0.795)
                .mk(Mk::Box([o - 2.0, y - 0.3, o - 0.005, y + 0.795 + 0.3]))
                .draw(&c),
        );
        v
    });

    // .1.h3: the head.  The body under the block is 1.0 µm wide, but the right head - the
    // salicided stub the contact sits on, outside the block - necks down to 0.6.  The
    // manual's "minimum width of Poly2 resistor" is measured on a layer that runs the
    // whole bar, heads included.
    write(format!("{r}.1.h3"), {
        let mut v = vec![
            rect(c.poly, o, o, o + 4.6, o + W),
            rect(c.poly, o + 4.6, o + 0.2, o + LEN, o + 0.8),
            rect(c.implant, o - IMP, o - IMP, o + LEN + IMP, o + W + IMP),
            rect(c.sab, o + SAB_IN, o - SABW, o + 4.4, o + W + SABW),
            rect(c.res_mk, o - MK, o - MK, o + LEN + MK, o + W + MK),
        ];
        for (cx, cy) in [(o + 0.68, o + 0.39), (o + 5.1, o + 0.39)] {
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v
    });

    // .1.h4: the same 0.795 bar on the tile lines - one across x = 20, one across x = 40
    // and 42, one across y = 20 and ending exactly on x = 21.  The layer the width is
    // measured on is built by three `interacting` steps, which is where a tile could
    // change what the deck recognises.
    write(format!("{r}.1.h4"), {
        let mut v = Dev::at(17.0, 10.0).width(0.795).no_conts().draw(&c);
        v.extend(Dev::at(38.0, 10.0).width(0.795).no_conts().draw(&c));
        v.extend(Dev::at(15.0, 19.6).width(0.795).no_conts().draw(&c));
        v
    });

    // .2.h1: the serpentine.  Top: a U whose two arms are 0.395 µm apart, which is the
    // gap the rule names - a resistor folded back on itself is one polygon, and the space
    // between its arms is still a space between Poly2 resistors.  Bottom: the same U at
    // 0.4.
    write(format!("{r}.2.h1"), {
        let mut v = u_poly(&c, o, o, 0.395);
        v.extend(u_context(&c, o, o, 0.395));
        v.extend(u_poly(&c, o, o + 6.0, 0.4));
        v.extend(u_context(&c, o, o + 6.0, 0.4));
        v
    });

    // .2.h2: two resistors 0.395 µm apart with the gap straddling the y = 40 tile line,
    // and a clean 0.4 pair well inside a tile.
    write(format!("{r}.2.h2"), {
        let mut v = Dev::at(o, 38.805).no_conts().draw(&c);
        v.extend(Dev::at(o, 40.2).no_conts().draw(&c));
        v.extend(Dev::at(o, 10.0).no_conts().draw(&c));
        v.extend(Dev::at(o, 11.4).no_conts().draw(&c));
        v
    });

    // .3.h1: the space to COMP.  Three devices: a COMP 0.595 µm off the end of the body
    // (fires), one at 0.6, and one whose corner is 0.594 µm from the body's corner on the
    // diagonal - 0.42 in x and in y, so nothing faces it square on.
    write(format!("{r}.3.h1"), {
        let mut v = Dev::at(o, o).draw(&c);
        v.push(rect(c.comp, o + LEN + 0.595, o, o + LEN + 2.595, o + W));
        v.extend(Dev::at(o, o + 8.0).draw(&c));
        v.push(rect(
            c.comp,
            o + LEN + 0.6,
            o + 8.0,
            o + LEN + 2.6,
            o + 8.0 + W,
        ));
        v.extend(Dev::at(o, o + 16.0).draw(&c));
        v.push(rect(
            c.comp,
            o + LEN + 0.42,
            o + 16.0 + W + 0.42,
            o + LEN + 2.42,
            o + 16.0 + W + 2.42,
        ));
        v
    });

    // .3.h2: COMP that touches and COMP that overlaps.  Three devices: a COMP abutting
    // the body's end (a space of nothing), one overlapping it by 0.5 µm, and one wholly
    // inside the body.
    write(format!("{r}.3.h2"), {
        let mut v = Dev::at(o, o).draw(&c);
        v.push(rect(c.comp, o + LEN, o, o + LEN + 2.0, o + W));
        v.extend(Dev::at(o, o + 8.0).draw(&c));
        v.push(rect(
            c.comp,
            o + LEN - 0.5,
            o + 8.0,
            o + LEN + 1.5,
            o + 8.0 + W,
        ));
        v.extend(Dev::at(o, o + 16.0).draw(&c));
        v.push(rect(
            c.comp,
            o + 2.5,
            o + 16.0 + 0.25,
            o + 3.5,
            o + 16.0 + 0.75,
        ));
        v
    });

    // .4.h1: unrelated Poly2.  Three devices: a bare poly bar 0.595 µm off the end
    // (fires), one at 0.6, and one at 0.595 whose far end touches a salicide block of its
    // own.  The deck reads "unrelated" as "touching no block anywhere", so that last bar
    // stops being unrelated to *this* resistor because of a block five micron away.
    write(format!("{r}.4.h1"), {
        let mut v = Dev::at(o, o).draw(&c);
        v.push(rect(c.poly, o + LEN + 0.595, o, o + LEN + 2.595, o + W));
        v.extend(Dev::at(o, o + 8.0).draw(&c));
        v.push(rect(
            c.poly,
            o + LEN + 0.6,
            o + 8.0,
            o + LEN + 2.6,
            o + 8.0 + W,
        ));
        v.extend(Dev::at(o, o + 16.0).draw(&c));
        v.push(rect(
            c.poly,
            o + LEN + 0.595,
            o + 16.0,
            o + LEN + 2.595,
            o + 16.0 + W,
        ));
        v.push(rect(
            c.sab,
            o + LEN + 2.595,
            o + 16.0 + 0.2,
            o + LEN + 3.5,
            o + 16.0 + 0.8,
        ));
        v
    });

    // .5.h1: the implant running out across the body.  It covers the bar as far as
    // x + 3.0 and stops there, in the middle of the block, so the resistor's own right
    // wall is the implant's edge and the implant overlaps it by nothing.
    write(format!("{r}.5.h1"), {
        Dev::at(o, o)
            .imp(Imp::Box([o - IMP, o - IMP, o + 3.0, o + W + IMP]))
            .draw(&c)
    });

    // .6.h1: the block overhanging 0.275 µm above the body and 0.28 below it - one wall
    // short, one at the bound.  A second device has both at 0.28.
    write(format!("{r}.6.h1"), {
        let mut v = Dev::at(o, o)
            .sab(Sab::Std {
                top: 0.275,
                bot: 0.28,
            })
            .draw(&c);
        v.extend(
            Dev::at(o, o + 8.0)
                .sab(Sab::Std {
                    top: 0.28,
                    bot: 0.28,
                })
                .draw(&c),
        );
        v
    });

    // .6.h2: the block covering the whole bar, ends included - 0.35 µm past it in the
    // width direction and 0.15 past each end.  The rule is "salicide block overlap of
    // Poly2 resistor *in width direction*": the ends are not in the width direction and
    // the resistor has no salicided head left to measure there.
    write(format!("{r}.6.h2"), {
        Dev::at(o, o)
            .no_conts()
            .sab(Sab::Box([o - 0.15, o - SABW, o + LEN + 0.15, o + W + SABW]))
            .draw(&c)
    });

    // .6.h3: the same question on a layout somebody would draw.  The block runs 0.15 µm
    // past the *left* end of the bar - it is shared with whatever lies to the left - and
    // ends inside the bar on the right, where the head and its contact are.  The overlap
    // in the width direction is 0.35 the whole way.
    write(format!("{r}.6.h3"), {
        let mut v = Dev::at(o, o)
            .no_conts()
            .sab(Sab::Box([o - 0.15, o - SABW, o + 4.4, o + W + SABW]))
            .draw(&c);
        let cy = o + (W - CONT) * 0.5;
        v.push(rect(c.contact, o + 4.9, cy, o + 4.9 + CONT, cy + CONT));
        v
    });

    // .7.h1: contacts against the block.  One device with the left contact abutting the
    // block's edge and the right contact wholly inside the block.
    write(format!("{r}.7.h1"), {
        let mut v = Dev::at(o, o).no_conts().draw(&c);
        let cy = o + (W - CONT) * 0.5;
        v.push(rect(
            c.contact,
            o + SAB_IN - CONT,
            cy,
            o + SAB_IN,
            cy + CONT,
        ));
        v.push(rect(c.contact, o + 3.0, cy, o + 3.0 + CONT, cy + CONT));
        v
    });

    // .7.h2: a contact 0.2149 µm from a block on the diagonal.  The device's contacts sit
    // high in the body and a second salicide island lies above the bar, its corner 0.152
    // in x and 0.152 in y from the left contact's corner: under the value as a distance,
    // over it on either axis alone.
    write(format!("{r}.7.h2"), {
        let mut v = Dev::at(o, o).no_conts().draw(&c);
        let cy = o + 0.70;
        for cx in [o + 0.68, o + 4.9] {
            v.push(rect(c.contact, cx, cy, cx + CONT, cy + CONT));
        }
        v.push(rect(c.sab, o + 1.052, o + 1.072, o + 2.052, o + 2.072));
        v
    });

    // .9a.h1: what the marking has to coincide with.  Four devices, each with a different
    // RES_MK: on the resistor's outline exactly (the manual's own drawing); 0.6 µm narrow,
    // so its long edges run along inside the Poly2; 0.6 short of the block at each end;
    // and 0.9 past the block at each end but still short of the bar.
    write(format!("{r}.9a.h1"), {
        let mut v = Vec::new();
        for (i, b) in [
            [o + 1.4, o, o + 4.6, o + W],
            [o + 1.4, o + 0.2, o + 4.6, o + 0.8],
            [o + 2.0, o - 0.3, o + 4.0, o + W + 0.3],
            [o + 0.5, o - 0.3, o + 5.5, o + W + 0.3],
        ]
        .into_iter()
        .enumerate()
        {
            let y = o + 8.0 * i as f64;
            let b = [b[0], b[1] + 8.0 * i as f64, b[2], b[3] + 8.0 * i as f64];
            v.extend(Dev::at(o, y).mk(Mk::Box(b)).draw(&c));
        }
        v
    });

    // .9a.h2: a 1 µm resistor under a 6.6 µm marking.  The block runs from x + 2.5 to
    // x + 3.5, so the resistor is 1 µm long, and the RES_MK clears the whole bar on every
    // side - its length coincides with nothing.  Every edge of it falls outside the
    // Poly2, which is the only place the coded rule looks.
    write(format!("{r}.9a.h2"), {
        Dev::at(o, o)
            .sab(Sab::Box([o + 2.5, o - SABW, o + 3.5, o + W + SABW]))
            .mk(Mk::Clear(MK))
            .draw(&c)
    });

    // .9b.h1: the big marking.  An octagon 140 µm across in x and in y - 16238 µm², both
    // sides over 80 - with a 100 µm square marking 19.9 µm off its right side.  The square
    // is under the area bound, so only one end of the pair qualifies and the gap is one
    // violation.  No edge of the octagon is 80 µm long.
    write(format!("{r}.9b.h1"), {
        vec![
            octagon(c.res_mk, 10.0, 10.0, 140.0, 41.0),
            rect(c.res_mk, 169.9, 10.0, 269.9, 110.0),
        ]
    });

    // .9b.h2: one marking of 110 x 160 µm with a 19.9 µm notch cut 70 µm into it.  The
    // rule is a spacing "to adjacent RES_MK layer": a notch in one marking has no
    // adjacent marking on the other side of it.
    write(format!("{r}.9b.h2"), {
        vec![
            rect(c.res_mk, 10.0, 10.0, 120.0, 80.0),
            rect(c.res_mk, 10.0, 99.9, 120.0, 170.0),
            rect(c.res_mk, 10.0, 80.0, 50.0, 99.9),
        ]
    });
}
