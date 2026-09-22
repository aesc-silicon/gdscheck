// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Salicide block: a good and a bad pattern for every rule in the `sab` deck.
//!
//! Two base cells, because the deck says the same things twice - once about SAB over an
//! active and once about SAB over poly.  Each is a bar with a SAB stripe across it, and
//! the arrangement is what makes the rules readable: the stripe overhangs the bar in one
//! direction and the bar runs past the stripe in the other, so SB.6 and SB.7 (and SB.9
//! and SB.10 for poly) are a pair of margins on one shape rather than two rules fighting
//! over it.
//!
//! SB.16 is the constraint every fixture has to respect rather than a shape to draw: SAB
//! may not sit on a CMOS transistor, so wherever a pattern needs poly over an active -
//! SB.5b and SB.14b both do - the SAB has to stay clear of it.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/sab";

/// The bar the stripe crosses, and how far it runs past the stripe - SB.7 and SB.10 ask
/// 0.22 either side.
const BAR_L: f64 = 8.0;
const BAR_W: f64 = 2.0;
/// The stripe's width along the bar, and its overhang past the bar - SB.6 and SB.9 ask
/// 0.22.  The overhang is generous so the stripe clears SB.13's 2 µm² on its own.
const STRIPE: f64 = 3.0;
const OVER: f64 = 0.5;

struct Ctx {
    sab: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    contact: (i16, i16),
    nwell: (i16, i16),
}

/// A bar with a SAB stripe across it: an active one, or a poly one with an implant.
struct Cell {
    x: f64,
    y: f64,
    bar: (i16, i16),
    implant: Option<(i16, i16)>,
    /// Stripe width along the bar, and how far it overhangs across it.
    stripe: f64,
    over: f64,
    /// How far the stripe's left edge sits from the bar's, for the overlap rules.
    inset: f64,
    /// How far the implant reaches past the poly.  Zero where a second cell has to come
    /// close: SB.14a measures between the two unsalicided polys, and an implant reaching
    /// past its own poly would come within SB.15a's 0.18 µm of the other one first.
    implant_margin: f64,
}

impl Cell {
    fn comp(c: &Ctx, x: f64, y: f64) -> Self {
        Cell {
            x,
            y,
            bar: c.comp,
            implant: None,
            stripe: STRIPE,
            over: OVER,
            inset: 2.0,
            implant_margin: 0.4,
        }
    }

    fn poly(c: &Ctx, x: f64, y: f64, implant: Option<(i16, i16)>) -> Self {
        Cell {
            x,
            y,
            bar: c.poly,
            implant,
            stripe: STRIPE,
            over: OVER,
            inset: 2.0,
            implant_margin: 0.4,
        }
    }

    /// The SAB stripe's extent, which several fixtures measure their neighbours from.
    fn stripe_box(&self) -> (f64, f64, f64, f64) {
        (
            self.x + self.inset,
            self.y - self.over,
            self.x + self.inset + self.stripe,
            self.y + BAR_W + self.over,
        )
    }

    fn draw(&self, c: &Ctx) -> Vec<GdsElement> {
        let (x, y) = (self.x, self.y);
        let mut v = vec![rect(self.bar, x, y, x + BAR_L, y + BAR_W)];
        if let Some(i) = self.implant {
            let m = self.implant_margin;
            v.push(rect(i, x - m, y - m, x + BAR_L + m, y + BAR_W + m));
        }
        let (sx0, sy0, sx1, sy1) = self.stripe_box();
        v.push(rect(c.sab, sx0, sy0, sx1, sy1));
        v
    }
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        sab: layer(pdk, "sab"),
        comp: layer(pdk, "comp"),
        poly: layer(pdk, "poly2_drawn"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        contact: layer(pdk, "contact"),
        nwell: layer(pdk, "nwell"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let comp_cell = || Cell::comp(&c, o, o).draw(&c);
    let poly_cell = || Cell::poly(&c, o, o, Some(c.nplus)).draw(&c);

    // Which base cell each rule's clean half is.
    for id in [
        "SB.1", "SB.2", "SB.3", "SB.4", "SB.6", "SB.7", "SB.8", "SB.11", "SB.13", "SB.16",
    ] {
        write(id, "good", comp_cell());
    }
    for id in ["SB.9", "SB.10", "SB.12", "SB.15a", "SB.15b"] {
        write(id, "good", poly_cell());
    }
    for id in ["SB.5a", "SB.5b", "SB.14a", "SB.14b"] {
        write(id, "good", comp_cell());
    }

    // SB.1: a stripe under 0.42 µm wide, drawn tall so its area still clears SB.13.
    write("SB.1", "bad", {
        let mut cell = Cell::comp(&c, o, o);
        cell.stripe = 0.415;
        cell.over = 2.5;
        cell.draw(&c)
    });

    // SB.2: two stripes on one bar, 0.42 µm apart.
    write("SB.2", "bad", {
        let mut v = comp_cell();
        let (_, sy0, sx1, sy1) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.sab, sx1 + 0.415, sy0, sx1 + 0.415 + STRIPE, sy1));
        v
    });

    // SB.3: an active the stripe does not overlap, 0.22 µm from it.  Directly above the
    // stripe, so the whole gap is on one axis, and clear of the cell's own active - were
    // the two to touch they would merge into a region that *does* overlap the stripe, and
    // the rule only looks at unrelated ones.
    write("SB.3", "bad", {
        let mut v = comp_cell();
        let (sx0, _, sx1, sy1) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.comp, sx0, sy1 + 0.215, sx1, sy1 + 2.215));
        v
    });

    // SB.4: a contact 0.15 µm from the stripe - beside it, not on it, which is SB.8.
    write("SB.4", "bad", {
        let mut v = comp_cell();
        let (sx0, _, _, sy1) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.contact, sx0, sy1 + 0.145, sx0 + 0.22, sy1 + 0.365));
        v
    });

    // SB.5a: unrelated poly on the field - no active under it, no SAB on it - 0.3 µm
    // above the stripe.
    write("SB.5a", "bad", {
        let mut v = comp_cell();
        let (sx0, _, sx1, sy1) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.poly, sx0, sy1 + 0.295, sx1, sy1 + 1.295));
        v
    });

    // SB.5b: unrelated poly on an active, 0.28 µm above the stripe.  Both sit clear of
    // the cell's own active, and the stripe stays off the gate they make, which SB.16
    // forbids outright.
    write("SB.5b", "bad", {
        let mut v = comp_cell();
        let (sx0, _, sx1, sy1) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.comp, sx0 - 0.5, sy1 + 0.275, sx1 + 0.5, sy1 + 1.275));
        v.push(rect(c.poly, sx0, sy1 + 0.275, sx1, sy1 + 1.775));
        v
    });

    // SB.6: the stripe overhanging the active by only 0.215 µm.
    write("SB.6", "bad", {
        let mut cell = Cell::comp(&c, o, o);
        cell.over = 0.215;
        cell.draw(&c)
    });

    // SB.7: the active running past the stripe by only 0.215 µm.
    write("SB.7", "bad", {
        let mut cell = Cell::comp(&c, o, o);
        cell.stripe = BAR_L - 2.0 * 0.215;
        cell.inset = 0.215;
        cell.draw(&c)
    });

    // SB.8: a contact on the stripe.
    write("SB.8", "bad", {
        let mut v = comp_cell();
        let (sx0, _, _, _) = Cell::comp(&c, o, o).stripe_box();
        v.push(rect(c.contact, sx0 + 1.0, o + 0.5, sx0 + 1.22, o + 0.72));
        v
    });

    // SB.9 / SB.10: the poly pair of SB.6 and SB.7.
    write("SB.9", "bad", {
        let mut cell = Cell::poly(&c, o, o, Some(c.nplus));
        cell.over = 0.215;
        cell.draw(&c)
    });
    write("SB.10", "bad", {
        let mut cell = Cell::poly(&c, o, o, Some(c.nplus));
        cell.stripe = BAR_L - 2.0 * 0.215;
        cell.inset = 0.215;
        cell.draw(&c)
    });

    // SB.11 / SB.12: the stripe reaching only 0.215 µm onto the bar, over its end, so it
    // still runs past the bar one way and the bar past it the other.
    write("SB.11", "bad", {
        let mut cell = Cell::comp(&c, o, o);
        cell.inset = BAR_L - 0.215;
        cell.draw(&c)
    });
    write("SB.12", "bad", {
        let mut cell = Cell::poly(&c, o, o, Some(c.nplus));
        cell.inset = BAR_L - 0.215;
        cell.draw(&c)
    });

    // SB.13: a stripe under 2 µm², still over the 0.42 width and the 0.22 overlap.
    write("SB.13", "bad", {
        let mut cell = Cell::comp(&c, o, o);
        cell.stripe = 0.6;
        cell.over = 0.6;
        cell.draw(&c)
    });

    // SB.14a: unsalicided N+ poly within 0.56 µm of unsalicided P+ poly.
    write("SB.14a", "bad", {
        let mut a = Cell::poly(&c, o, o, Some(c.nplus));
        a.implant_margin = 0.0;
        let mut b = Cell::poly(&c, o, o + BAR_W + 0.555, Some(c.pplus));
        b.implant_margin = 0.0;
        [a.draw(&c), b.draw(&c)].concat()
    });

    // SB.14b: unsalicided N+ poly within 0.56 µm of a P-channel gate, which is poly over
    // a P+ active.  The gate sits directly below the cell so the whole 0.555 µm is on one
    // axis, and the cell's stripe is drawn at its own minimum overhang so it stays the
    // 0.3 µm clear of that poly which SB.5a and SB.5b want.
    write("SB.14b", "bad", {
        let mut a = Cell::poly(&c, o, o, Some(c.nplus));
        a.implant_margin = 0.0;
        a.over = 0.22;
        let mut v = a.draw(&c);
        // The gate: poly crossing a P+ active, its top 0.555 µm below the cell's bar.
        // A P-channel needs its N-well, or the active is not a P-channel one and the
        // layer this rule measures against never comes into being.
        let top = o - 0.555;
        v.push(rect(c.nwell, o + 0.5, top - 2.5, o + 4.5, top + 0.2));
        v.push(rect(c.comp, o + 1.0, top - 2.0, o + 4.0, top));
        v.push(rect(c.pplus, o + 1.0, top - 2.0, o + 4.0, top));
        v.push(rect(c.poly, o + 0.5, top - 1.5, o + 4.5, top));
        v
    });

    // SB.15a: an implant that is not the cell's own, within 0.18 µm of the unsalicided
    // poly.  It sits below the bar, so it comes near `sab ∩ poly` - which is what the
    // rule measures - without touching the poly and becoming related to it.  The cell's
    // own implant is drawn flush to the bar, or the two would merge into one region and
    // that region would overlap the poly, making it related after all.
    write("SB.15a", "bad", {
        let mut cell = Cell::poly(&c, o, o, Some(c.nplus));
        cell.implant_margin = 0.0;
        cell.over = 0.22;
        let mut v = cell.draw(&c);
        let (sx0, _, sx1, _) = cell.stripe_box();
        v.push(rect(c.pplus, sx0, o - 0.175 - 2.0, sx1, o - 0.175));
        v
    });

    // SB.15b measures the same distance at 0.32 µm, but only against an implant that
    // *touches* poly the SAB is on - so this one reaches the bar, beside the stripe
    // rather than under it, and the cell carries no implant of its own to merge with.
    write("SB.15b", "bad", {
        let cell = Cell::poly(&c, o, o, None);
        let mut v = cell.draw(&c);
        let (_, _, sx1, _) = cell.stripe_box();
        v.push(rect(c.pplus, sx1 + 0.315, o, sx1 + 2.315, o + BAR_W));
        v
    });

    // SB.16: the stripe on a transistor - poly over an active - which it may never be.
    write("SB.16", "bad", {
        let mut v = vec![
            rect(c.comp, o, o, o + BAR_L, o + BAR_W),
            rect(c.poly, o + 3.0, o - 0.5, o + 5.0, o + BAR_W + 0.5),
        ];
        v.push(rect(c.sab, o + 2.0, o - 1.0, o + 6.0, o + BAR_W + 1.0));
        v
    });

    hardening(pdk);
}

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.10 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/sab/SB.<rule>.h<n>.gds.gz` with a case in the
// `hardening_sab` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/sab.md.
//
// What these draw is the deck's own conditions - the three markers that exempt a block
// (OTP_MK for most of the section, ESD_MK for SB.12, LVS_IO and ESD_MK for SB.16), what
// makes a COMP, a Poly2 or an implant "related" to a block, the unsalicided N+/P+ poly
// pair SB.14a measures with a square, and the transistor SB.16 forbids the block to sit
// on.  The generic classes (the bound on a bare layer, 45 degrees, unions, arrays, tile
// lines on a plain shape) are the engine family's.

/// Layers the hardening patterns draw on.
struct H {
    sab: (i16, i16),
    comp: (i16, i16),
    poly: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    co: (i16, i16),
    nwell: (i16, i16),
    otp: (i16, i16),
    lvsio: (i16, i16),
    esdmk: (i16, i16),
    res: (i16, i16),
}

impl H {
    fn new(pdk: &PdkConfig) -> Self {
        H {
            sab: layer(pdk, "sab"),
            comp: layer(pdk, "comp"),
            poly: layer(pdk, "poly2_drawn"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            co: layer(pdk, "contact"),
            nwell: layer(pdk, "nwell"),
            otp: layer(pdk, "otp_mk"),
            lvsio: layer(pdk, "lvs_io"),
            esdmk: layer(pdk, "esd_mk"),
            res: layer(pdk, "res_mk"),
        }
    }

    /// The clean unit the section is drawn around: a bar of `bar` with a block across it,
    /// the block overhanging the bar by 0.6 and the bar running 1.5 past the block either
    /// way - clear of SB.6, SB.7, SB.9, SB.10, SB.11, SB.12 and SB.13 at once.
    fn crossed(&self, bar: (i16, i16), x: f64, y: f64) -> Vec<GdsElement> {
        vec![
            rect(bar, x, y, x + 6.0, y + 2.0),
            rect(self.sab, x + 1.5, y - 0.6, x + 4.5, y + 2.6),
        ]
    }

    /// A transistor with a block over its gate: COMP 6 x 3 at `(x, y)`, a 2 um poly
    /// across it, and a block over the gate that satisfies every other rule of the
    /// section - so only SB.16 can speak.
    fn gate_cell(&self, x: f64, y: f64) -> Vec<GdsElement> {
        vec![
            rect(self.comp, x, y, x + 6.0, y + 3.0),
            rect(self.poly, x + 2.0, y - 1.1, x + 4.0, y + 4.1),
            rect(self.sab, x + 1.5, y - 0.8, x + 4.5, y + 3.8),
        ]
    }
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let h = H::new(pdk);
    sb_1_h(&h);
    sb_2_h(&h);
    sb_3_h(&h);
    sb_4_h(&h);
    sb_5a_h(&h);
    sb_5b_h(&h);
    sb_5_h(&h);
    sb_7_h(&h);
    sb_10_h(&h);
    sb_12_h(&h);
    sb_13_h(&h);
    sb_14a_h(&h);
    sb_14b_h(&h);
    sb_15a_h(&h);
    sb_15b_h(&h);
    sb_16_h(&h);
}

// --- SB.1: min. sab width 0.42 ---

fn sb_1_h(h: &H) {
    // h1 - the marker does not reach the width rule.  SB.1 is the one rule of the
    // section the manual states about the SAB layer itself rather than about a block and
    // its neighbour, and OTP_MK does not appear in the manual at all: a block too narrow
    // to print is too narrow whatever marks it.  (a) 0.415 x 6 under OTP_MK, one step
    // under the 0.42; (b) 0.42 under OTP_MK, clean at the bound.  Both under the marker,
    // so SB.2 and SB.13 stay out of the answer.
    hwrite(
        "SB.1.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 2.415, 8.0), // (a) SB.1
            rect(h.otp, 1.5, 1.5, 2.915, 8.5),
            rect(h.sab, 6.0, 2.0, 6.42, 8.0), // (b) clean
            rect(h.otp, 5.5, 1.5, 6.92, 8.5),
        ],
    );
}

// --- SB.2: min. sab spacing 0.42 ---

fn sb_2_h(h: &H) {
    // h1 - who the OTP marker lets off.  Four pairs of 1.5 x 2 blocks (3 um2 each, clear
    // of SB.13): (a) 0.415 apart with OTP_MK over both, exempt; (b) 0.415 apart with
    // OTP_MK abutting the left block's far edge and over neither, SB.2 - a marker beside
    // a block does not mark it; (c) 0.42 apart, clean at the bound; (d) a 0.415 notch in
    // a U, which the rule reads as a space of the same kind.
    hwrite(
        "SB.2.h1",
        vec![
            // (a) exempt
            rect(h.sab, 2.0, 2.0, 3.5, 4.0),
            rect(h.sab, 3.915, 2.0, 5.415, 4.0),
            rect(h.otp, 1.5, 1.5, 5.915, 4.5),
            // (b) SB.2
            rect(h.sab, 9.0, 2.0, 10.5, 4.0),
            rect(h.sab, 10.915, 2.0, 12.415, 4.0),
            rect(h.otp, 8.0, 1.5, 9.0, 4.5),
            // (c) clean
            rect(h.sab, 16.0, 2.0, 17.5, 4.0),
            rect(h.sab, 17.92, 2.0, 19.42, 4.0),
            // (d) a U with a 0.415 notch: SB.2
            rect(h.sab, 2.0, 8.0, 3.5, 11.0),
            rect(h.sab, 3.915, 8.0, 6.0, 11.0),
            rect(h.sab, 2.0, 8.0, 6.0, 8.7),
        ],
    );
}

// --- SB.3: space from salicide block to unrelated COMP 0.22 ---

fn sb_3_h(h: &H) {
    // h1 - which COMP is "unrelated".  (a) a COMP the block does not touch, 0.215 above
    // it: SB.3.  (b) the same at 0.22: clean.  (c) one bar of COMP with two blocks -
    // block A across it (so the bar is related to A) and block B 0.215 above the bar,
    // which B does not touch.  The manual reads "unrelated" of the pair: B has no
    // salicide block relationship with that COMP and owes it the 0.22.  Both decks read
    // it of the COMP alone - a COMP that overlaps any block at all is related to every
    // block - and let the gap through.
    let mut v = h.crossed(h.comp, 2.0, 2.0);
    v.push(rect(h.comp, 3.5, 4.815, 6.5, 6.815)); // (a) SB.3
    v.extend(h.crossed(h.comp, 12.0, 2.0));
    v.push(rect(h.comp, 13.5, 4.82, 16.5, 6.82)); // (b) clean
    // (c) the cross-relation
    v.push(rect(h.comp, 2.0, 12.0, 12.0, 14.0));
    v.push(rect(h.sab, 3.5, 11.4, 6.5, 14.6)); // block A, across the bar
    v.push(rect(h.sab, 8.0, 14.215, 10.0, 16.215)); // block B, 0.215 above it
    hwrite("SB.3.h1", v);

    // h2 - the space of nothing.  (a) a 3 x 3 block with an unrelated COMP abutting its
    // right edge: no gap at all, which the section reads as a space under the value.  It
    // is also the deck's two readings of "related" meeting - a COMP that touches a block
    // is unrelated for SB.3 (no overlap) and related for SB.6, SB.7 and SB.11 (it
    // interacts), so the same edge answers to both.  (b) the same 0.005 away, which only
    // SB.3 can see.
    hwrite(
        "SB.3.h2",
        vec![
            rect(h.sab, 2.0, 2.0, 5.0, 5.0), // (a)
            rect(h.comp, 5.0, 2.5, 8.0, 4.5),
            rect(h.sab, 12.0, 2.0, 15.0, 5.0), // (b) SB.3
            rect(h.comp, 15.005, 2.5, 18.0, 4.5),
        ],
    );
}

// --- SB.4 / SB.8: the block and a contact ---

fn sb_4_h(h: &H) {
    // h1 - beside the block, on it, and under the marker.  (a) a contact 0.145 above a
    // 3 x 3 block: SB.4.  (b) the same at 0.15: clean.  (c) a contact touching the
    // block's edge: a space of nothing, SB.4, and no SB.8 - nothing of the contact lies
    // on the block.  (d) a contact on a block under OTP_MK: SB.8 alone, the manual's
    // "non-salicided contacts are forbidden" being about the contact and not about the
    // marker, while SB.4's spacing is one of the rules the deck lets OTP_MK off.  (e) a
    // contact on a bare block: SB.8, and SB.4 with it - upstream folds the on-block case
    // into SB.4 as well.
    hwrite(
        "SB.4.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 5.0, 5.0), // (a) SB.4
            rect(h.co, 3.0, 5.145, 3.22, 5.365),
            rect(h.sab, 9.0, 2.0, 12.0, 5.0), // (b) clean
            rect(h.co, 10.0, 5.15, 10.22, 5.37),
            rect(h.sab, 16.0, 2.0, 19.0, 5.0), // (c) SB.4, no SB.8
            rect(h.co, 17.0, 5.0, 17.22, 5.22),
            rect(h.sab, 2.0, 9.0, 5.0, 12.0), // (d) SB.8 only
            rect(h.otp, 1.5, 8.5, 5.5, 12.5),
            rect(h.co, 3.0, 10.0, 3.22, 10.22),
            rect(h.sab, 9.0, 9.0, 12.0, 12.0), // (e) SB.4 and SB.8
            rect(h.co, 10.0, 10.0, 10.22, 10.22),
        ],
    );
}

// --- SB.5a: space from the block to unrelated Poly2 on field 0.3 ---

fn sb_5a_h(h: &H) {
    // h1 - the bound and the marker.  (a) a field poly (no COMP under it, no block on
    // it) 0.295 above a 3 x 3 block: SB.5a.  (b) the same at 0.3: clean.  (c) 0.295 from
    // a block under OTP_MK: exempt.
    hwrite(
        "SB.5a.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 5.0, 5.0), // (a) SB.5a
            rect(h.poly, 2.0, 5.295, 5.0, 6.295),
            rect(h.sab, 9.0, 2.0, 12.0, 5.0), // (b) clean
            rect(h.poly, 9.0, 5.3, 12.0, 6.3),
            rect(h.sab, 16.0, 2.0, 19.0, 5.0), // (c) exempt
            rect(h.otp, 15.5, 1.5, 19.5, 5.5),
            rect(h.poly, 16.0, 5.295, 19.0, 6.295),
        ],
    );

    // h2 - the poly that touches the block.  (a) a field poly abutting a 3 x 3 block's
    // right edge with no overlap: the manual asks 0.3 of poly the block is not on, and
    // nothing of this block is on the poly, so the space of nothing is SB.5a.  (b) the
    // same 0.005 away, which is the same violation a step wider.  The abutting poly is
    // "interacting" the block and so lands in the unsalicided-poly layer of SB.9, SB.10
    // and SB.12, but with no overlap at all none of the three has a margin to measure.
    hwrite(
        "SB.5a.h2",
        vec![
            rect(h.sab, 2.0, 2.0, 5.0, 5.0), // (a)
            rect(h.poly, 5.0, 3.0, 8.0, 3.6),
            rect(h.sab, 12.0, 2.0, 15.0, 5.0), // (b) SB.5a
            rect(h.poly, 15.005, 3.0, 18.0, 3.6),
        ],
    );
}

// --- SB.5b: space from the block to unrelated Poly2 on COMP 0.28 ---

fn sb_5b_h(h: &H) {
    // h1 - the bound.  A poly bar lying wholly inside a COMP plate, so the whole bar is
    // Poly2-on-COMP and none of it is Poly2 on field; the block sits below the plate.
    // (a) 0.275 from the bar: SB.5b.  (b) 0.28: clean.  The plate itself stands 0.275
    // and 0.28 off the block, both over SB.3's 0.22.
    hwrite(
        "SB.5b.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 6.0, 5.0), // (a) SB.5b
            rect(h.comp, 1.0, 5.275, 9.0, 7.5),
            rect(h.poly, 2.0, 5.275, 8.0, 5.875),
            rect(h.sab, 12.0, 2.0, 16.0, 5.0), // (b) clean
            rect(h.comp, 11.0, 5.28, 19.0, 7.5),
            rect(h.poly, 12.0, 5.28, 18.0, 5.88),
        ],
    );
}

// --- SB.5a and SB.5b together: one poly, two layers ---

fn sb_5_h(h: &H) {
    // h1 - a poly bar crossing a COMP's edge, so its lower strip is Poly2 on field and
    // its upper strip is Poly2 on COMP.  The block below stands 0.29 from the field
    // strip and 0.59 from the COMP strip: SB.5a fires (0.29 under 0.3) and SB.5b does
    // not (0.59 over 0.28).  The same geometry at 0.30 / 0.60 is clean.
    hwrite(
        "SB.5.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 6.0, 5.0), // SB.5a only
            rect(h.comp, 1.0, 5.59, 9.0, 8.0),
            rect(h.poly, 2.0, 5.29, 8.0, 5.89),
            rect(h.sab, 12.0, 2.0, 16.0, 5.0), // clean
            rect(h.comp, 11.0, 5.9, 19.0, 8.0),
            rect(h.poly, 12.0, 5.3, 18.0, 5.9),
        ],
    );

    // h2 - one poly line crossing a COMP bar, which cuts the field poly into two pieces.
    // Block A lies on the left piece; block B stands 0.295 from the right piece, which
    // no block is on.  Salicide blocking is local, so the right piece is bare poly and
    // owes B the 0.3: SB.5a.  Drawn to record that the section's "unrelated" is read of
    // the piece and not of the poly line.
    hwrite(
        "SB.5.h2",
        vec![
            rect(h.comp, 10.0, 2.0, 12.0, 8.0),
            rect(h.poly, 2.0, 4.0, 20.0, 4.6),
            rect(h.sab, 4.0, 3.5, 7.0, 5.1), // block A, on the left piece
            rect(h.sab, 14.0, 4.895, 17.0, 6.5), // block B, 0.295 above the right piece
        ],
    );
}

// --- SB.7: COMP extension beyond the related block 0.22 ---

fn sb_7_h(h: &H) {
    // h1 - the extension that is not there.  (a) a 2 x 2 COMP island wholly inside a
    // 6 x 6 block: the COMP does not run past the block anywhere, which is SB.7 read as
    // the manual states it - the active must stand 0.22 clear of the block all round.
    // (b) a COMP crossing the block with its left edge exactly on the block's left edge:
    // an extension of nothing on that wall and 2.0 on the other, the coincident-edge
    // case the enclosure engine skips.
    hwrite(
        "SB.7.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 8.0, 8.0), // (a)
            rect(h.comp, 4.0, 4.0, 6.0, 6.0),
            rect(h.sab, 12.0, 2.0, 18.0, 8.0), // (b)
            rect(h.comp, 12.0, 4.0, 20.0, 6.0),
        ],
    );
}

// --- SB.10: Poly2 extension beyond the related block 0.22 ---

fn sb_10_h(h: &H) {
    // h1 - SB.7's twin on poly.  (a) a 2 x 2 poly island wholly inside a 6 x 6 block:
    // the poly never runs past the block, SB.10.  (b) a poly crossing the block with its
    // left edge on the block's left edge: the coincident wall.
    hwrite(
        "SB.10.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 8.0, 8.0), // (a)
            rect(h.poly, 4.0, 4.0, 6.0, 6.0),
            rect(h.sab, 12.0, 2.0, 18.0, 8.0), // (b)
            rect(h.poly, 12.0, 4.0, 20.0, 6.0),
        ],
    );
}

// --- SB.12: overlap with Poly2 outside ESD_MK 0.22 ---

fn sb_12_h(h: &H) {
    // h1 - the ESD marker, which the manual names in this rule and in no other.  Four
    // copies of a poly bar with a block reaching only 0.215 onto its right end: (a) no
    // marker, SB.12; (b) ESD_MK over the whole block, exempt; (c) ESD_MK over the
    // block's right half - the manual exempts the overlap that lies inside the marker
    // and this one does not, but both decks drop a block the marker touches at all;
    // (d) ESD_MK abutting the block's right edge and over none of it, SB.12.
    let cell = |y: f64| {
        vec![
            rect(h.poly, 2.0, y, 8.0, y + 2.0),
            rect(h.sab, 7.785, y - 0.5, 10.785, y + 2.5),
        ]
    };
    let mut v = cell(2.0); // (a) SB.12
    v.extend(cell(8.0)); // (b) exempt
    v.push(rect(h.esdmk, 7.285, 7.0, 11.285, 11.0));
    v.extend(cell(14.0)); // (c) half marked
    v.push(rect(h.esdmk, 9.285, 13.0, 11.285, 17.0));
    v.extend(cell(20.0)); // (d) SB.12
    v.push(rect(h.esdmk, 10.785, 19.0, 13.0, 23.0));
    hwrite("SB.12.h1", v);
}

// --- SB.13: min. area 2 um2 ---

fn sb_13_h(h: &H) {
    // h1 - what counts as the block's area.  (a) 1.4 x 1.4 = 1.96 um2: SB.13.  (b)
    // 1.415 x 1.415 = 2.002 um2: clean one step over.  (c) 1.96 um2 under OTP_MK:
    // exempt.  (d) a ring 1.5 x 1.5 with a 0.515 x 0.515 hole - 2.25 less 0.265, so
    // 1.985 um2 of block: the area is the block's own and not the outline's, SB.13.  Its
    // walls are 0.4925 and its hole 0.515, both over SB.1's and SB.2's values.
    hwrite(
        "SB.13.h1",
        vec![
            rect(h.sab, 2.0, 2.0, 3.4, 3.4),     // (a) SB.13
            rect(h.sab, 6.0, 2.0, 7.415, 3.415), // (b) clean
            rect(h.sab, 10.0, 2.0, 11.4, 3.4),   // (c) exempt
            rect(h.otp, 9.5, 1.5, 11.9, 3.9),
            // (d) the ring: SB.13
            rect(h.sab, 14.0, 2.0, 15.5, 2.4925),
            rect(h.sab, 14.0, 3.0075, 15.5, 3.5),
            rect(h.sab, 14.0, 2.4925, 14.4925, 3.0075),
            rect(h.sab, 15.0075, 2.4925, 15.5, 3.0075),
        ],
    );
}

// --- SB.14a: unsalicided N+ poly to unsalicided P+ poly, 0.56 in a square ---

fn sb_14a_h(h: &H) {
    // h1 - the butted pair and the corner.  (a) one poly bar under one block with N+ on
    // its left half and P+ on its right, butted on a shared edge: the two unsalicided
    // regions share that edge, which is a space of nothing.  The manual states the rule
    // as a square at the P+ region's corners, and a butted N+/P+ poly - a poly diode -
    // has no corner to keep clear; upstream says so outright, dropping every marker that
    // touches the butting edge.  (b) two unsalicided regions whose corners lie 0.5 apart
    // in x and 0.5 in y: 0.5 in the square the rule names (0.707 euclidian), so the
    // corner falls inside it and the rule fires.
    hwrite(
        "SB.14a.h1",
        vec![
            // (a) the butted pair
            rect(h.poly, 2.0, 2.0, 8.0, 3.0),
            rect(h.sab, 3.0, 1.5, 7.0, 3.5),
            rect(h.nplus, 2.0, 2.0, 5.0, 3.0),
            rect(h.pplus, 5.0, 2.0, 8.0, 3.0),
            // (b) corner to corner, 0.5 by 0.5
            rect(h.poly, 2.0, 8.0, 8.0, 9.0),
            rect(h.nplus, 2.0, 8.0, 8.0, 9.0),
            rect(h.sab, 3.0, 7.5, 5.0, 9.5),
            rect(h.poly, 5.0, 9.5, 11.0, 10.5),
            rect(h.pplus, 5.0, 9.5, 11.0, 10.5),
            rect(h.sab, 5.5, 9.1, 8.0, 10.9),
        ],
    );
}

// --- SB.14b: unsalicided N+ poly to a P-channel gate, 0.56 in a square ---

fn sb_14b_h(h: &H) {
    // h1 - the same square against a P-channel gate.  A PMOS - N-well, P+ COMP, poly
    // across it - with an unsalicided N+ poly bar above it whose block-covered region
    // has its lower right corner `d` in x and `d` in y from the gate's upper left
    // corner.  (a) d = 0.5, inside the 0.56 square: SB.14b.  (b) d = 0.565, outside it:
    // clean.
    let cell = |x: f64, d: f64| {
        let (gx, gy) = (x + 10.0, 7.0); // the gate's upper left corner
        vec![
            rect(h.nwell, x + 8.0, 2.0, x + 14.0, 8.0),
            rect(h.comp, x + 9.0, 3.0, x + 13.0, gy),
            rect(h.pplus, x + 9.0, 3.0, x + 13.0, gy),
            rect(h.poly, gx, 2.5, x + 11.0, gy + 0.5),
            // the unsalicided N+ bar; its block-covered region ends at gx - d
            rect(h.poly, x + 3.0, gy + d, gx - d + 6.0, gy + d + 1.0),
            rect(h.nplus, x + 3.0, gy + d, gx - d + 6.0, gy + d + 1.0),
            rect(h.sab, x + 4.0, gy + d - 0.22, gx - d, gy + d + 1.22),
        ]
    };
    let mut v = cell(0.0, 0.5); // (a) SB.14b
    v.extend(cell(20.0, 0.565)); // (b) clean
    hwrite("SB.14b.h1", v);
}

// --- SB.15a: unsalicided Poly2 to unrelated Nplus/Pplus 0.18 ---

fn sb_15a_h(h: &H) {
    // h1 - an implant that is on no unsalicided poly.  A poly bar with a block across
    // its middle, and a P+ plate below the bar which reaches under the block but never
    // onto the poly, so it is unrelated to the unsalicided region.  (a) 0.175 from the
    // block-on-poly region: SB.15a.  (b) 0.18: clean.  The second copy runs from x = 14
    // to 22, across the 20 um tile line.
    hwrite(
        "SB.15a.h1",
        vec![
            rect(h.poly, 2.0, 4.0, 10.0, 5.0), // (a) SB.15a
            rect(h.sab, 4.0, 3.5, 8.0, 5.5),
            rect(h.pplus, 4.0, 1.5, 8.0, 3.825),
            rect(h.poly, 14.0, 4.0, 22.0, 5.0), // (b) clean
            rect(h.sab, 16.0, 3.5, 20.0, 5.5),
            rect(h.pplus, 16.0, 1.5, 20.0, 3.82),
        ],
    );
}

// --- SB.15b: unsalicided Poly2 to unrelated Nplus/Pplus along the poly line 0.32 ---

fn sb_15b_h(h: &H) {
    // h1 - the poly resistor's heads.  A poly bar with a block across its middle and an
    // N+ head at either end for the contacts: the heads sit on the same poly line as the
    // unsalicided region, which is what the rule's "along Poly2 line" names, and the
    // implant edge has to stay 0.32 from the block.  (a) heads 0.315 away, twice: two
    // SB.15b.  (b) the same at 0.32: clean.  (c) one N+ covering the whole bar with a
    // 0.2 margin all round - the implant the unsalicided poly belongs to, containing the
    // block-on-poly region rather than facing it, which is no space at all.
    hwrite(
        "SB.15b.h1",
        vec![
            rect(h.poly, 2.0, 2.0, 10.0, 3.0), // (a) SB.15b x2
            rect(h.sab, 4.0, 1.5, 8.0, 3.5),
            rect(h.nplus, 1.8, 1.8, 3.685, 3.2),
            rect(h.nplus, 8.315, 1.8, 10.2, 3.2),
            rect(h.poly, 14.0, 2.0, 22.0, 3.0), // (b) clean
            rect(h.sab, 16.0, 1.5, 20.0, 3.5),
            rect(h.nplus, 13.8, 1.8, 15.68, 3.2),
            rect(h.nplus, 20.32, 1.8, 22.2, 3.2),
            rect(h.poly, 2.0, 8.0, 10.0, 9.0), // (c) clean
            rect(h.sab, 4.0, 7.5, 8.0, 9.5),
            rect(h.nplus, 1.8, 7.8, 10.2, 9.2),
        ],
    );
}

// --- SB.16: no block on a transistor ---

fn sb_16_h(h: &H) {
    // h1 - the three markers the manual names, and what marks what.  Seven transistors
    // with a block over the gate, every other rule of the section satisfied: (a) bare,
    // SB.16; (b) LVS_IO over the block, exempt; (c) ESD_MK over it, exempt; (d) OTP_MK
    // over it, exempt; (e) RES_MK over the poly-on-COMP, which stops it being a
    // transistor at all, exempt; (f) LVS_IO over the block's left half only - the
    // manual's marker marks the transistor and the right half of this block still sits
    // on an unmarked one, but both decks drop a block the marker touches at all; (g) the
    // bare case again, its block crossing the 20 um tile line.
    let mut v = h.gate_cell(2.0, 2.0); // (a) SB.16
    v.extend(h.gate_cell(12.0, 2.0)); // (b) exempt
    v.push(rect(h.lvsio, 11.0, 0.5, 19.0, 6.5));
    v.extend(h.gate_cell(22.0, 2.0)); // (c) exempt
    v.push(rect(h.esdmk, 21.0, 0.5, 29.0, 6.5));
    v.extend(h.gate_cell(2.0, 10.0)); // (d) exempt
    v.push(rect(h.otp, 1.0, 8.5, 9.0, 14.5));
    v.extend(h.gate_cell(12.0, 10.0)); // (e) exempt
    v.push(rect(h.res, 13.8, 9.8, 16.2, 13.2));
    v.extend(h.gate_cell(22.0, 10.0)); // (f) half marked
    v.push(rect(h.lvsio, 22.5, 9.2, 25.0, 14.0));
    v.extend(h.gate_cell(17.0, 18.0)); // (g) SB.16, across x = 20
    hwrite("SB.16.h1", v);

    // h2 - the block beside the gate.  (a) a block whose left edge lies exactly on the
    // gate's right edge: the manual forbids a block that exists *on* the transistor's
    // poly and COMP area, and nothing of this one does, but the deck's "interacting"
    // takes a shared edge for a block on the gate.  (b) the same block 0.005 clear of
    // the gate, which nothing reads as on it.  Both blocks stand on the COMP beside the
    // gate, whose field poly and poly-on-COMP are a space of nothing from (a) and 0.005
    // from (b), so SB.5a and SB.5b speak here too.
    let cell = |x: f64, d: f64| {
        vec![
            rect(h.comp, x, 2.0, x + 6.0, 5.0),
            rect(h.poly, x + 2.0, 0.9, x + 4.0, 6.1),
            rect(h.sab, x + 4.0 + d, 1.2, x + 7.0, 5.8),
        ]
    };
    let mut v = cell(2.0, 0.0); // (a)
    v.extend(cell(14.0, 0.005)); // (b)
    hwrite("SB.16.h2", v);
}
