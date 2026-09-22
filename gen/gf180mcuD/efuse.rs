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
use crate::helpers::{layer, library, poly, rect, write_gz};
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
    nplus: (i16, i16),
    esd: (i16, i16),
    sab: (i16, i16),
    resistor: (i16, i16),
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
        nplus: layer(pdk, "nplus"),
        esd: layer(pdk, "esd"),
        sab: layer(pdk, "sab"),
        resistor: layer(pdk, "resistor"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    hardening(&c);
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

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from section 10.11 of the manual, with a case in the `hardening_efuse`
// table of `tests/gf180mcuD.rs` and the findings in hardening/reports/gf180mcuD/efuse.md.
//
// This section is unlike the others: nine of its rules read "Min. Max." and *fix* a
// dimension rather than bounding it, so the step past the value has two sides and the
// deck's own bad halves only ever take the short one.  Most of what follows takes the
// long one instead, and the rest is the classes a spacing rule has: the value, the step,
// and the neighbour drawn flush against the shape it is measured from.
//
// Every probe is a whole fuse - a cathode, a link and an anode, all inside EFUSE_MK and
// P+, the anode under LVS_SOURCE - because the deck reads those four layers together to
// tell one part of the device from another, and a bar of any one of them on its own is
// not a fuse at all.  Probes stand `H_PITCH` apart, which is more than twice EF.20's
// 2.73 µm, the longest reach in the section.

/// How far apart the fuses of one hardening row stand.
const H_PITCH: f64 = 20.0;

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A row of fuses, each with whatever extra shapes its own probe needs, drawn in the
/// fuse's own frame.
fn hrow(c: &Ctx, items: Vec<(Cell, Vec<GdsElement>)>) -> Vec<GdsElement> {
    let mut v = Vec::new();
    for (cell, extra) in items {
        v.extend(cell.draw(c));
        v.extend(extra);
    }
    v
}

fn hardening(c: &Ctx) {
    let o = OFFSET;
    // One fuse at the `i`-th place of a row, with `f` applied to it.
    let at = |i: usize, f: &dyn Fn(&mut Cell)| {
        let mut cell = Cell::at(o + i as f64 * H_PITCH, o);
        f(&mut cell);
        cell
    };
    let plain = |i: usize| at(i, &|_| {});
    let nothing = |_: &mut Cell| {};

    // --- EF.02: the fuse layer's width, which the manual fixes at 0.18 both ways.  Bare
    // bars off to the side of one good fuse, so that nothing reads them as a device: the
    // link of a real fuse cannot be widened without moving EF.22a's and EF.22b's
    // shoulders with it.  Each bar is 2 µm long, which is what the max side has to not
    // mistake for a width.
    hwrite("EF.02.h1", {
        let mut v = plain(0).draw(c);
        for (i, w) in [(0usize, 0.18), (1, 0.175), (2, 0.185)] {
            let x = o + 20.0 + i as f64 * 4.0;
            v.push(rect(c.plfuse, x, o, x + 2.0, o + w));
        }
        v
    });

    // --- EF.03: the link's length, fixed at 1.26.  One step short and one step long.
    // The whole poly runs 1.84 + 1.26 + 2.43, so moving the link moves EF.21's 5.53 with
    // it and the shoulders stay put; both ids belong to each of these fuses.
    hwrite(
        "EF.03.h1",
        hrow(
            c,
            vec![
                (plain(0), vec![]),
                (at(1, &|k| k.fuse_l = 1.255), vec![]),
                (at(2, &|k| k.fuse_l = 1.265), vec![]),
            ],
        ),
    );

    // --- EF.06 to EF.09: the two pads' four dimensions, each fixed by the manual and
    // each drawn here one step *over* the value.  The deck's own bad halves take them one
    // step under.
    hwrite(
        "EF.06.h1",
        hrow(
            c,
            vec![
                (at(0, &|k| k.cat_w = 2.265), vec![]),
                (at(1, &|k| k.cat_l = 1.845), vec![]),
                // One step over 1.06 puts the anode's own edges off the 0.005 grid,
                // since it grows either side of the centre line; two steps is the
                // smallest wrong width that stays on it.
                (at(2, &|k| k.an_w = 1.07), vec![]),
                (at(3, &|k| k.an_l = 2.435), vec![]),
            ],
        ),
    );

    // --- EF.12: the cathode's contacts and the link's end, 0.155 µm.  The value, the
    // step, and a contact drawn flush against the link's end.
    hwrite(
        "EF.12.h1",
        hrow(
            c,
            vec![
                (at(0, &|k| k.cat_gap = 0.155), vec![]),
                (at(1, &|k| k.cat_gap = 0.15), vec![]),
                (at(2, &|k| k.cat_gap = 0.0), vec![]),
            ],
        ),
    );

    // --- EF.13: the same at the anode, 0.14 µm.
    hwrite(
        "EF.13.h1",
        hrow(
            c,
            vec![
                (at(0, &|k| k.an_gap = 0.14), vec![]),
                (at(1, &|k| k.an_gap = 0.135), vec![]),
                (at(2, &|k| k.an_gap = 0.0), vec![]),
            ],
        ),
    );

    // --- EF.15: no contact may *touch* the link.  One 0.005 clear of it, one sharing its
    // end edge, and one meeting its corner at a single point.
    hwrite("EF.15.h1", {
        let fuse_of = |cell: &Cell| cell.boxes()[1];
        // Three contacts on the pad, so that the probe contact makes the four EF.16a
        // asks for and that rule has nothing to say about any of these fuses.
        let probe = |i: usize, f: &dyn Fn((f64, f64, f64, f64)) -> (f64, f64)| {
            let cell = at(i, &|k| k.cat_contacts = 3);
            let (x, y) = f(fuse_of(&cell));
            (cell, vec![rect(c.contact, x, y, x + CO, y + CO)])
        };
        hrow(
            c,
            vec![
                // 0.005 short of the link's left end, on the cathode.
                probe(0, &|b| (b.0 - 0.005 - CO, b.1)),
                // Flush against it.
                probe(1, &|b| (b.0 - CO, b.1)),
                // Meeting the link's bottom-left corner at one point.
                probe(2, &|b| (b.0 - CO, b.1 - CO)),
            ],
        )
    });

    // --- EF.16a / EF.16b: each pad must hold exactly four contacts - so five is as wrong
    // as three.  The fourth fuse has a fifth contact on the anode, stacked above the row.
    hwrite("EF.16.h1", {
        let extra_cat = {
            let cell = plain(2);
            let cat = cell.boxes()[0];
            vec![rect(
                c.contact,
                cat.0 + 0.2,
                cat.1 + 1.7,
                cat.0 + 0.2 + CO,
                cat.1 + 1.7 + CO,
            )]
        };
        let extra_an = {
            let cell = plain(3);
            let an = cell.boxes()[2];
            vec![rect(
                c.contact,
                an.0 + 0.4,
                an.1 + 0.1,
                an.0 + 0.4 + CO,
                an.1 + 0.1 + CO,
            )]
        };
        hrow(
            c,
            vec![
                (plain(0), vec![]),
                (at(1, &|k| k.cat_contacts = 3), vec![]),
                (plain(2), extra_cat),
                (plain(3), extra_an),
            ],
        )
    });

    // --- EF.17: 0.26 µm between markers, with both gaps centred on a tile line, and a
    // 0.255 slot cut into a marker of its own.  A marker with nothing inside it answers
    // to nothing else in the section.
    hwrite("EF.17.h1", {
        // Two fuses whose markers face at 0.26 across x = 20 ...
        let a = Cell::at(13.94, o);
        let b = Cell::at(20.53, o);
        // ... and two more at 0.255 across x = 40.
        let d = Cell::at(33.94, o);
        let e = Cell::at(40.525, o);
        let mut v = hrow(c, vec![(a, vec![]), (b, vec![]), (d, vec![]), (e, vec![])]);
        // A bare marker with a 0.255 slot in it, well clear of the fuses.
        v.push(poly(
            c.efuse_mk,
            &[
                (60.0, 20.0),
                (66.0, 20.0),
                (66.0, 22.0),
                (62.0, 22.0),
                (62.0, 22.255),
                (66.0, 22.255),
                (66.0, 24.0),
                (60.0, 24.0),
            ],
        ));
        v
    });

    // --- EF.19: *Min. PLFUSE space to Metal1, Metal2* is zero, so metal drawn flush
    // against the link keeps the space the rule asks for; metal *over* the link does not.
    hwrite("EF.19.h1", {
        let probe = |i: usize, over: bool| {
            let cell = plain(i);
            let b = cell.boxes()[1];
            let m = if over {
                rect(c.metal1, b.0, b.1 - 0.3, b.2, b.3 + 0.3)
            } else {
                rect(c.metal1, b.0, b.3, b.2, b.3 + 0.6)
            };
            (cell, vec![m])
        };
        hrow(c, vec![probe(0, false), probe(1, true)])
    });

    // --- EF.20: 2.73 µm from the link to an active.  The value, the step, and an active
    // drawn flush against the link's own wall - which is a space of nothing.
    hwrite("EF.20.h1", {
        let probe = |i: usize, gap: Option<f64>| {
            let cell = plain(i);
            let b = cell.boxes()[1];
            let comp = match gap {
                Some(g) => rect(c.comp, b.0, b.3 + g, b.0 + 2.0, b.3 + g + 2.0),
                None => rect(c.comp, b.0, b.3, b.0 + 2.0, b.3 + 2.0),
            };
            (cell, vec![comp])
        };
        hrow(
            c,
            vec![probe(0, Some(2.73)), probe(1, Some(2.725)), probe(2, None)],
        )
    });

    // --- EF.10 / EF.11: 0.26 µm from a pad to the poly beside it.  Both decks read the
    // rule as the pad against a pad of its own kind, so each probe is a pair of fuses,
    // the second drawn right to left so that the pad of the same kind faces the first's.
    let facing = |xa: f64, gap: f64| {
        let a = Cell::at(xa, o);
        let mut b = Cell::at(xa - (CAT_L + FUSE_L + AN_L) - gap, o);
        b.flip = true;
        let mut v = a.draw(c);
        v.extend(b.draw(c));
        v
    };
    // Cathode to cathode, at the value and one step under it.
    hwrite("EF.10.h1", {
        let mut v = facing(o + 8.0, 0.26);
        v.extend(facing(o + 28.0, 0.255));
        v
    });
    // Anode to anode: the left fuse is the one drawn right to left, so the two anodes
    // face each other instead.
    hwrite("EF.11.h1", {
        let facing_an = |xa: f64, gap: f64| {
            let a = Cell::at(xa, o);
            let mut b = Cell::at(xa + CAT_L + FUSE_L + AN_L + gap, o);
            b.flip = true;
            let mut v = a.draw(c);
            v.extend(b.draw(c));
            v
        };
        let mut v = facing_an(o, 0.26);
        v.extend(facing_an(o + 20.0, 0.255));
        v
    });

    // --- EF.14: the marker has to hold LVS_SOURCE with zero to spare, so a source flush
    // with the marker's edge is legal and one that crosses it is not.  The first fuse's
    // marker ends exactly on the anode's right edge, where the source ends too.
    hwrite(
        "EF.14.h1",
        hrow(
            c,
            vec![
                (at(0, &|k| k.mk_right = 0.0), vec![]),
                (
                    at(1, &|k| {
                        k.mk_right = 0.0;
                        k.src_out = true;
                    }),
                    vec![],
                ),
            ],
        ),
    );

    // The rest of EF.20's neighbour list: the rule names COMP, Nplus, ESD, SAB and
    // Resistor, and one of each is drawn 2.725 µm from the link.
    hwrite("EF.20.h2", {
        let probe = |i: usize, l: (i16, i16)| {
            let cell = plain(i);
            let b = cell.boxes()[1];
            let y = b.3 + 2.725;
            (cell, vec![rect(l, b.0, y, b.0 + 2.0, y + 2.0)])
        };
        hrow(
            c,
            vec![
                probe(0, c.nplus),
                probe(1, c.esd),
                probe(2, c.sab),
                probe(3, c.resistor),
            ],
        )
    });

    // A fuse with nothing wrong with it at all, for the record: every rule of the section
    // reads it and none of them may speak.
    hwrite("EF.00.h1", hrow(c, vec![(plain(0), vec![])]));
    let _ = nothing;
}
