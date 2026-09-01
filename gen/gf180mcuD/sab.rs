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
}
