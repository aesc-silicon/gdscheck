// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! eFuse: a good and a bad pattern for every rule in the `efuse` deck.
//!
//! The fuse is a dogbone - a wide cathode, a narrow link, a longer anode - and this deck
//! is unlike the others here in that it does not bound its dimensions, it *fixes* them.
//! Nine of the rules ask for an exact length, and the numbers agree with each other:
//! 1.84 + 1.26 + 2.43 is EF.21's 5.53 µm of poly end to end, and the shoulders left when
//! the 0.18 µm link meets the 2.26 and 1.06 µm pads are EF.22a's 1.04 and EF.22b's 0.44.
//! So the cell below is the fuse the deck describes, and a bad half is that fuse with one
//! dimension a half-grid out.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/efuse";

/// The fuse, as the deck fixes it.
const CAT_L: f64 = 1.84;
const CAT_W: f64 = 2.26;
const FUSE_L: f64 = 1.26;
const FUSE_W: f64 = 0.18;
const AN_L: f64 = 2.43;
const AN_W: f64 = 1.06;
/// Marker and implant reach past the poly.
const MK: f64 = 0.4;
/// Contact side, and how many each pad must hold - EF.16a and EF.16b ask for four.
const CO: f64 = 0.22;

struct Ctx {
    poly: (i16, i16),
    plfuse: (i16, i16),
    lvs_source: (i16, i16),
    efuse_mk: (i16, i16),
    pplus: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
    comp: (i16, i16),
}

#[derive(Clone, Copy)]
struct Cell {
    x: f64,
    y: f64,
    cat_l: f64,
    cat_w: f64,
    fuse_l: f64,
    fuse_w: f64,
    an_l: f64,
    an_w: f64,
    /// How far the cathode contacts sit from the link - EF.12 asks 0.155.
    cat_gap: f64,
    /// And the anode's - EF.13 asks 0.14.
    an_gap: f64,
    /// Contacts on each pad.  Four apiece is what EF.16a and EF.16b ask for, and they are
    /// separate so a fixture can take one from a single pad.
    cat_contacts: usize,
    an_contacts: usize,
    /// Whether the P+ implant covers the marker, which EF.01 asks.
    pp: bool,
    /// Whether LVS_SOURCE covers the anode, which EF.05 and EF.14 ask.
    src: bool,
    /// Drawn right to left, so a second fuse can put a pad of the same kind beside the
    /// first one's without its other pad landing on top.
    flip: bool,
    /// LVS_SOURCE grown past the anode, in y within the marker or in x beyond it.
    src_wide: bool,
    src_out: bool,
    /// The marker's reach past the anode, which `src_out` needs at zero.
    mk_right: f64,
}

impl Cell {
    fn at(x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            cat_l: CAT_L,
            cat_w: CAT_W,
            fuse_l: FUSE_L,
            fuse_w: FUSE_W,
            an_l: AN_L,
            an_w: AN_W,
            cat_gap: 0.8,
            an_gap: 0.4,
            cat_contacts: 4,
            an_contacts: 4,
            pp: true,
            src: true,
            flip: false,
            src_wide: false,
            src_out: false,
            mk_right: MK,
        }
    }

    /// The three boxes the fuse is made of, along x, sharing a centre line.
    fn boxes(&self) -> [(f64, f64, f64, f64); 3] {
        let (x, y) = (self.x, self.y);
        // The nominal centre, not the drawn one: a fixture that narrows the cathode must
        // not slide the link and the anode with it.
        let mid = y + CAT_W * 0.5;
        let x1 = x + self.cat_l;
        let x2 = x1 + self.fuse_l;
        [
            (x, y, x1, y + self.cat_w),
            (x1, mid - self.fuse_w * 0.5, x2, mid + self.fuse_w * 0.5),
            (
                x2,
                mid - self.an_w * 0.5,
                x2 + self.an_l,
                mid + self.an_w * 0.5,
            ),
        ]
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        if self.flip {
            // Mirror about the cell's own left edge: same fuse, drawn right to left.
            let mut plain = *self;
            plain.flip = false;
            let axis = 2.0 * self.x + self.cat_l + self.fuse_l + self.an_l;
            return plain
                .draw(c)
                .into_iter()
                .map(|e| match e {
                    GdsElement::GdsBoundary(mut b) => {
                        for p in b.xy.iter_mut() {
                            p.x = (axis * 1000.0) as i32 - p.x;
                        }
                        b.xy.reverse();
                        GdsElement::GdsBoundary(b)
                    }
                    other => other,
                })
                .collect();
        }
        let [cat, fuse, an] = self.boxes();
        let mut v = Vec::new();
        // The poly is one shape in three pieces; they abut and merge.
        for b in [cat, fuse, an] {
            v.push(rect(c.poly, b.0, b.1, b.2, b.3));
        }
        v.push(rect(c.plfuse, fuse.0, fuse.1, fuse.2, fuse.3));
        let (mx0, my0) = (cat.0 - MK, cat.1 - MK);
        let (mx1, my1) = (an.2 + self.mk_right, cat.3 + MK);
        if self.src {
            let (sx1, sy0, sy1) = if self.src_out {
                (mx1 + 1.0, an.1, an.3)
            } else if self.src_wide {
                (an.2, an.1 - 0.2, an.3 + 0.2)
            } else {
                (an.2, an.1, an.3)
            };
            v.push(rect(c.lvs_source, an.0, sy0, sx1, sy1));
        }
        v.push(rect(c.efuse_mk, mx0, my0, mx1, my1));
        if self.pp {
            v.push(rect(c.pplus, mx0 - 0.1, my0 - 0.1, mx1 + 1.5, my1 + 0.1));
        }
        // Contacts: a column on the cathode and a row on the anode, both clear of the link.
        for i in 0..self.cat_contacts {
            let (dx, dy) = (i % 2, i / 2);
            let x = cat.2 - self.cat_gap - CO - dx as f64 * (CO + 0.3);
            let y = cat.1 + 0.3 + dy as f64 * (CO + 0.4);
            v.push(rect(c.contact, x, y, x + CO, y + CO));
        }
        for i in 0..self.an_contacts {
            let x = an.0 + self.an_gap + i as f64 * (CO + 0.3);
            let y = (an.1 + an.3) * 0.5 - CO * 0.5;
            v.push(rect(c.contact, x, y, x + CO, y + CO));
        }
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        poly: layer(pdk, "poly2_drawn"),
        plfuse: layer(pdk, "plfuse"),
        lvs_source: layer(pdk, "lvs_source"),
        efuse_mk: layer(pdk, "efuse_mk"),
        pplus: layer(pdk, "pplus"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
        comp: layer(pdk, "comp"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let base = Cell::at(o, o);
    let clean = || base.draw(&c);

    for id in [
        "EF.02", "EF.01", "EF.03", "EF.04a", "EF.04b", "EF.04c", "EF.04d", "EF.05", "EF.06",
        "EF.07", "EF.08", "EF.09", "EF.10", "EF.11", "EF.12", "EF.13", "EF.14", "EF.15", "EF.16a",
        "EF.16b", "EF.17", "EF.18", "EF.19", "EF.20", "EF.21", "EF.22a", "EF.22b",
    ] {
        write(id, "good", clean());
    }

    // EF.02 fixes the fuse layer's own width wherever it is drawn, not just on a device,
    // so its fixture is a bare bar off to one side rather than a bent fuse: bending the
    // fuse's link moves the shoulders EF.22a and EF.22b pin and answers for them too.
    let bar = |w: f64| {
        let mut v = base.draw(&c);
        v.push(rect(c.plfuse, o + 20.0, o, o + 22.0, o + w));
        v
    };
    write("EF.02", "good", bar(0.18));
    write("EF.02", "bad", bar(0.175));

    // EF.04a: a fuse with no poly under it and no anode at either end, in a marker of its
    // own off to one side.  Widening the device's own fuse would say the same thing, but
    // its width is what EF.02 fixes and its shoulders are what EF.22a and EF.22b pin.
    write("EF.04a", "bad", {
        let mut v = base.draw(&c);
        let (x, y) = (o + 20.0, o);
        v.push(rect(c.plfuse, x, y, x + 1.26, y + 0.18));
        v.push(rect(c.efuse_mk, x - 0.7, y - 0.7, x + 1.96, y + 0.88));
        // EF.01 wants the implant over anything the marker covers.
        v.push(rect(c.pplus, x - 0.9, y - 0.9, x + 2.16, y + 1.08));
        v
    });

    // The dimensions the deck fixes: each bad half is the fuse with one of them a
    // half-grid out, which is what the exact-length rules are there to catch.
    let dim = |f: &dyn Fn(&mut Cell)| {
        let mut cell = base;
        f(&mut cell);
        cell.draw(&c)
    };
    write("EF.03", "bad", dim(&|k| k.fuse_l -= 0.005));
    write("EF.06", "bad", dim(&|k| k.cat_w -= 0.005));
    write("EF.07", "bad", dim(&|k| k.cat_l -= 0.005));
    write("EF.08", "bad", dim(&|k| k.an_w -= 0.005));
    write("EF.09", "bad", dim(&|k| k.an_l -= 0.005));
    // EF.21 measures the poly end to end, so it moves with any of them; the link is the
    // one dimension no other exact rule reads.
    write("EF.21", "bad", dim(&|k| k.fuse_l += 0.005));
    // EF.22a and EF.22b are the shoulders where the link meets each pad, so widening the
    // link narrows them without touching a pad's own width.
    write("EF.22a", "bad", dim(&|k| k.fuse_w += 0.01));
    write("EF.22b", "bad", dim(&|k| k.fuse_w += 0.01));

    // EF.01: poly under the marker that the P+ implant does not cover.
    write("EF.01", "bad", dim(&|k| k.pp = false));
    // EF.05 and EF.14: the anode with no LVS_SOURCE on it.
    // EF.05: LVS_SOURCE reaching past the anode *within* the marker, where there is no
    // poly under it.  Leaving the source off entirely makes the anode disappear - the
    // anode is poly and source together - and with it the rule, so the source has to stay
    // and grow instead.
    write("EF.05", "bad", dim(&|k| k.src_wide = true));
    // EF.14: the same source reaching past the *marker*.  The marker is drawn flush to
    // the anode here, so the part of the source with no poly under it lies outside the
    // marker and EF.05 has nothing to say about it.
    write(
        "EF.14",
        "bad",
        dim(&|k| {
            k.src_out = true;
            k.mk_right = 0.0;
        }),
    );

    // EF.04b / EF.04c / EF.04d: the link, the cathode and the anode must each be a
    // rectangle.  An L is the smallest thing that is not.
    // EF.04b / EF.04c / EF.04d: the link, the cathode and the anode must each be a
    // rectangle.  A bump on one edge is the smallest thing that is not - and it
    // necessarily lengthens that edge, so each of these also breaks whichever rule pins
    // it.  The deck fixes every dimension of this device, so there is no bump that
    // changes the shape without changing a number.
    let bump = |which: usize| {
        let mut v = clean();
        let b = base.boxes()[which];
        let l = if which == 1 { c.plfuse } else { c.poly };
        let (bx, by) = ((b.0 + b.2) * 0.5, b.3);
        v.push(rect(l, bx, by, bx + 0.3, by + 0.3));
        if which == 1 {
            // The link's bump has to be poly as well, or it is a marker off the shape.
            v.push(rect(c.poly, bx, by, bx + 0.3, by + 0.3));
        }
        if which == 2 {
            // And the anode's has to carry LVS_SOURCE, or the bump is poly the source
            // does not cover: that is a piece of *cathode*, and EF.05's violation, while
            // the anode itself stays the rectangle this rule is about.
            v.push(rect(c.lvs_source, bx, by, bx + 0.3, by + 0.3));
        }
        v
    };
    write("EF.04b", "bad", bump(1));
    write("EF.04c", "bad", bump(0));
    write("EF.04d", "bad", bump(2));

    // EF.10 / EF.11: a second fuse, its cathode and anode 0.26 µm from the first's.  The
    // pads are at different heights, so one gap can be broken without the other.
    // EF.10 / EF.11: a second fuse, mirrored so the pad of the same kind faces the
    // first's.  Drawn the same way round, its other pad would land on top of this one.
    write("EF.10", "bad", {
        let mut v = clean();
        let mut second = Cell::at(o - (CAT_L + FUSE_L + AN_L) - 0.255, o);
        second.flip = true;
        v.extend(second.draw(&c));
        v
    });
    write("EF.11", "bad", {
        // Drawn well left of the usual origin, so the pair stays inside one 20 µm tile.
        // Edges are filed by their midpoint, so a fuse that straddles a tile line has its
        // link in one tile and the pad it feeds in the next, and the pad's edges are then
        // classified without it - which reads as a width violation that is the tiling's
        // and not the layout's.
        let x = 2.0;
        let mut a = Cell::at(x, o);
        a.x = x;
        let mut v = a.draw(&c);
        let mut second = Cell::at(x + CAT_L + FUSE_L + AN_L + 0.255, o);
        second.flip = true;
        v.extend(second.draw(&c));
        v
    });

    // EF.12 / EF.13: a contact too near the link, on each pad in turn.
    write("EF.12", "bad", dim(&|k| k.cat_gap = 0.15));
    write("EF.13", "bad", dim(&|k| k.an_gap = 0.135));

    // EF.15: a contact on the link itself.
    write("EF.15", "bad", {
        let mut v = clean();
        let [_, fuse, _] = base.boxes();
        v.push(rect(
            c.contact,
            fuse.0 + 0.4,
            fuse.1,
            fuse.0 + 0.4 + CO,
            fuse.1 + CO,
        ));
        v
    });

    // EF.16a / EF.16b: a pad with three contacts instead of four.
    write("EF.16a", "bad", dim(&|k| k.cat_contacts = 3));
    write("EF.16b", "bad", dim(&|k| k.an_contacts = 3));

    // EF.17: a second marker 0.26 µm from the first.
    write("EF.17", "bad", {
        let mut v = clean();
        let [cat, _, an] = base.boxes();
        v.push(rect(
            c.efuse_mk,
            an.2 + MK + 0.255,
            cat.1 - MK,
            an.2 + MK + 2.255,
            cat.3 + MK,
        ));
        v
    });

    // EF.18 / EF.19: the link over something it must sit clear of - anything at all for
    // EF.18, metal for EF.19.
    write("EF.18", "bad", {
        let mut v = clean();
        let [_, fuse, _] = base.boxes();
        v.push(rect(c.comp, fuse.0, fuse.1 - 0.3, fuse.2, fuse.3 + 0.3));
        v
    });
    write("EF.19", "bad", {
        let mut v = clean();
        let [_, fuse, _] = base.boxes();
        v.push(rect(c.metal1, fuse.0, fuse.1 - 0.3, fuse.2, fuse.3 + 0.3));
        v
    });

    // EF.20: an active within 2.73 µm of the link, but clear of it - over it is EF.18.
    write("EF.20", "bad", {
        let mut v = clean();
        // Measured from the link, which is what the rule names - not from the anode.
        let [_, fuse, an] = base.boxes();
        let x = fuse.2 + 2.725;
        v.push(rect(c.comp, x, an.1, x + 2.0, an.3));
        v
    });
}
