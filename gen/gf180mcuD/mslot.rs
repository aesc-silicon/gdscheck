// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Metal slotting: a good and a bad pattern for every rule in the `mslot` deck.
//!
//! The deck is the same nine rules once per metal level, so this is one plate drawn five
//! times over.  Every fixture is a 42 µm square of metal - wide enough that the deck's
//! "still needs a slot" derivation, which shrinks by 15 µm in each direction and grows
//! back, keeps it - carrying one legal 2 by 20 µm slot.  The bad half moves one edge of
//! that slot, or adds something beside it.
//!
//! Level five has no rule .7: the deck reads .7 as the via above the metal and .8 as the
//! one below, and above metal5 there is only the top metal.
//!
//! Rule .9 needs a region the foundry has already forbidden a slot in, and there are only
//! two ways to make one: a pad over the top metal, or a MIM bottom plate under a fuse
//! top.  Which one a fixture uses depends on the level it is testing, since the first
//! draws metal5 and the second metal4 - drawing either over the plate under test would
//! forbid slotting the plate itself rather than the ground beside it.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/mslot";

/// The plate, and the legal slot in it.
const PLATE: f64 = 42.0;
const SLOT_W: f64 = 2.0;
const SLOT_L: f64 = 20.0;

/// One metal level: what it is called in a rule id, its metal and slot layers, and the
/// vias above and below it.
struct Level {
    n: u32,
    metal: (i16, i16),
    slot: (i16, i16),
    above: Option<(i16, i16)>,
    below: (i16, i16),
}

/// What a fixture changes about the plate.
enum Bend {
    /// The slot's own box: origin and size.
    Slot(f64, f64, f64, f64),
    /// No slot at all.
    None,
    /// An L, which is a slot but not a rectangle.
    Ell,
    /// A second slot this far from the first, the pair centred so that both keep the
    /// 10 µm of metal round them that rule .5 asks for.
    Second(f64),
    /// A cut on the layer above or below, this far from the slot.
    Cut(bool, f64),
    /// A region slotting is forbidden in, this far from the slot once grown.
    Forbidden(f64),
}

fn plate(c: &Ctx, l: &Level, bend: &Bend) -> Vec<GdsElement> {
    let o = OFFSET;
    let mut v = vec![rect(l.metal, o, o, o + PLATE, o + PLATE)];
    // The legal slot, centred, 20 µm from either side and 11 from the top and bottom.
    let (sx, sy) = (o + (PLATE - SLOT_W) * 0.5, o + (PLATE - SLOT_L) * 0.5);
    let (mut w, mut h) = (SLOT_W, SLOT_L);
    let (mut sx, mut sy) = (sx, sy);
    if let Bend::Second(gap) = *bend {
        sx -= (gap + SLOT_W) * 0.5;
    }
    match *bend {
        Bend::Slot(x, y, bw, bh) => {
            sx = o + x;
            sy = o + y;
            w = bw;
            h = bh;
        }
        Bend::None => return v,
        _ => {}
    }
    v.push(rect(l.slot, sx, sy, sx + w, sy + h));
    match *bend {
        // A second mark off the first's foot, so the two merge into one L.
        Bend::Ell => v.push(rect(l.slot, sx + w, sy, sx + w + 8.0, sy + 2.0)),
        Bend::Second(gap) => v.push(rect(l.slot, sx + w + gap, sy, sx + w + gap + w, sy + h)),
        // (the pair's origin is set below, before the first is drawn)
        Bend::Cut(above, gap) => {
            let k = if above {
                l.above.unwrap_or(l.below)
            } else {
                l.below
            };
            let x = if above { sx + w + gap } else { sx - gap - 0.26 };
            v.push(rect(k, x, sy + h * 0.5, x + 0.26, sy + h * 0.5 + 0.26));
        }
        Bend::Forbidden(gap) => {
            // Grown by 5 µm before the rule reads it, so the drawing starts 5 further out.
            let x = sx + w + gap + 5.0;
            let (y0, y1) = (sy + h * 0.5 - 3.0, sy + h * 0.5 + 3.0);
            if l.n == 4 {
                // A pad over the top metal, since this level *is* the MIM bottom.
                v.push(rect(c.metal5, x, y0, x + 6.0, y1));
                v.push(rect(c.pad, x, y0, x + 6.0, y1));
            } else {
                v.push(rect(c.metal4, x, y0, x + 6.0, y1));
                v.push(rect(c.fusetop, x, y0, x + 6.0, y1));
            }
        }
        _ => {}
    }
    v
}

struct Ctx {
    metal4: (i16, i16),
    metal5: (i16, i16),
    fusetop: (i16, i16),
    pad: (i16, i16),
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        metal4: layer(pdk, "metal4_drawn"),
        metal5: layer(pdk, "metal5_drawn"),
        fusetop: layer(pdk, "fusetop"),
        pad: layer(pdk, "pad"),
    };
    let m = |n: u32| layer(pdk, &format!("metal{n}_drawn"));
    let s = |n: u32| layer(pdk, &format!("metal{n}_slot"));
    let via = |n: u32| layer(pdk, &format!("via{n}"));
    let levels = [
        Level {
            n: 1,
            metal: m(1),
            slot: s(1),
            above: Some(via(1)),
            below: layer(pdk, "contact"),
        },
        Level {
            n: 2,
            metal: m(2),
            slot: s(2),
            above: Some(via(2)),
            below: via(1),
        },
        Level {
            n: 3,
            metal: m(3),
            slot: s(3),
            above: Some(via(3)),
            below: via(2),
        },
        Level {
            n: 4,
            metal: m(4),
            slot: s(4),
            above: Some(via(4)),
            below: via(3),
        },
        Level {
            n: 5,
            metal: m(5),
            slot: s(5),
            above: None,
            below: via(4),
        },
    ];

    // The legal slot's own box, for the rules that move one of its edges.
    let (gx, gy) = ((PLATE - SLOT_W) * 0.5, (PLATE - SLOT_L) * 0.5);
    for l in &levels {
        let write = |suffix: &str, polarity: &str, bend: Bend| {
            write_gz(
                &format!("{DIR}/MSLOT{}.{suffix}.{polarity}.gds.gz", l.n),
                library("TOP", plate(&c, l, &bend)),
            );
        };
        let clean = || Bend::Slot(gx, gy, SLOT_W, SLOT_L);
        for suffix in ["0", "1", "2", "3", "4", "5", "7", "8", "9"] {
            if suffix == "7" && l.above.is_none() {
                continue;
            }
            write(suffix, "good", clean());
        }
        // .0: a slot that is not a rectangle.
        write("0", "bad", Bend::Ell);
        // .1: wide metal with no slot in it at all.
        write("1", "bad", Bend::None);
        // .2: a slot narrower than 2 µm, still long enough for .3.
        write("2", "bad", Bend::Slot(gx, gy, 1.995, SLOT_L));
        // .3: a slot shorter than 10 µm, still wide enough for .2.
        write(
            "3",
            "bad",
            Bend::Slot(gx, gy + (SLOT_L - 9.995) * 0.5, SLOT_W, 9.995),
        );
        // .4: two slots closer than 10 µm, both held 10 µm inside the metal.
        write("4", "bad", Bend::Second(9.995));
        // .5: a slot the metal holds by less than 10 µm.  It is lengthened rather than
        // slid over: sliding it leaves 30 µm of unslotted metal on the far side, which is
        // rule .1's business and not this one's.
        write(
            "5",
            "bad",
            Bend::Slot(gx, 9.995, SLOT_W, PLATE - 2.0 * 9.995),
        );
        // .7 and .8: a cut on the layer above, then below, closer than 0.2 µm.
        if l.above.is_some() {
            write("7", "bad", Bend::Cut(true, 0.195));
        }
        write("8", "bad", Bend::Cut(false, 0.195));
        // .9: a region slotting is forbidden in, closer than 5 µm once grown.
        write("9", "bad", Bend::Forbidden(4.995));
    }
}
