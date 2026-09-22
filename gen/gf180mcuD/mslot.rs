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
use crate::helpers::{layer, library, poly, rect, write_gz};
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
    /// The plate drawn as a ring, with the mark in the hole rather than in the metal.
    InHole,
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
        // A ring of the level's metal with the mark in its hole: the mark lies on no
        // metal at all, which is rule .10.  The arms are 10 µm, so nothing here is wide.
        Bend::InHole => {
            v.clear();
            let (x0, y0, x1, y1) = (o, o, o + PLATE, o + PLATE + 18.0);
            v.push(rect(l.metal, x0, y0, x1, y0 + 10.0));
            v.push(rect(l.metal, x0, y1 - 10.0, x1, y1));
            v.push(rect(l.metal, x0, y0 + 10.0, x0 + 10.0, y1 - 10.0));
            v.push(rect(l.metal, x1 - 10.0, y0 + 10.0, x1, y1 - 10.0));
            let (mx, my) = (
                (x0 + x1) * 0.5 - SLOT_W * 0.5,
                (y0 + y1) * 0.5 - SLOT_L * 0.5,
            );
            v.push(rect(l.slot, mx, my, mx + SLOT_W, my + SLOT_L));
            return v;
        }
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
        // .10: the mark in a hole of the metal rather than in the metal.  The good half
        // is the plain plate, where the mark lies on metal.
        write("10", "good", clean());
        write("10", "bad", Bend::InHole);
    }
    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md) ---
//
// Section 14.6.3 of the manual, read against the foundry's own runset.  These layouts ask
// the deck's own conditions rather than the engine's generic classes: what shape a slot
// mark has to be for the deck to read it at all, which bbox side each size rule takes, the
// metal the mark must sit in, the per-level via map, how the regions slotting is forbidden
// in are derived, and - the rule that makes slots necessary - what "a metal line wider
// than 30 µm" is, at the bound and with each of the three things that relieve it.
//
// Every plate is either under 30 µm in one direction, so the wide-metal rule has nothing
// to say about it, or is deliberately at that rule's bound.

/// The layers the hardening layouts draw on.
struct H {
    metal: [(i16, i16); 5],
    slot: [(i16, i16); 5],
    /// via1 .. via4.
    via: [(i16, i16); 4],
    contact: (i16, i16),
    m1_dummy: (i16, i16),
    pad: (i16, i16),
    fusetop: (i16, i16),
    fusewindow: (i16, i16),
    ubm: [(i16, i16); 3],
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// One plate of the `.9` layouts: the metal level under the opening, the opening's own
/// layer, and the gap the opening keeps from the plate's slot mark.
type Unit = (Option<usize>, Option<(i16, i16)>, f64);

/// A metal plate of `w` by `ht` with its corner at `(x, y)`, on level `n`.
fn mplate(h: &H, n: usize, x: f64, y: f64, w: f64, ht: f64) -> GdsElement {
    rect(h.metal[n - 1], x, y, x + w, y + ht)
}

/// A slot mark of `w` by `ht` with its corner at `(x, y)`, on level `n`.
fn mmark(h: &H, n: usize, x: f64, y: f64, w: f64, ht: f64) -> GdsElement {
    rect(h.slot[n - 1], x, y, x + w, y + ht)
}

fn hardening(pdk: &PdkConfig) {
    let m = |n: u32| layer(pdk, &format!("metal{n}_drawn"));
    let s = |n: u32| layer(pdk, &format!("metal{n}_slot"));
    let v = |n: u32| layer(pdk, &format!("via{n}"));
    let h = H {
        metal: [m(1), m(2), m(3), m(4), m(5)],
        slot: [s(1), s(2), s(3), s(4), s(5)],
        via: [v(1), v(2), v(3), v(4)],
        contact: layer(pdk, "contact"),
        m1_dummy: layer(pdk, "metal1_dummy"),
        pad: layer(pdk, "pad"),
        fusetop: layer(pdk, "fusetop"),
        fusewindow: layer(pdk, "fusewindow_d"),
        ubm: [
            layer(pdk, "ubmpperi"),
            layer(pdk, "ubmparray"),
            layer(pdk, "ubmeplate"),
        ],
    };

    all_levels(&h);
    dim(&h);
    length(&h);
    space(&h);
    space_tiles(&h);
    space_max(&h);
    enclosure(&h);
    enclosure_edge(&h);
    metal_hole(&h);
    via_map(&h);
    via_bound(&h);
    via_abut(&h);
    dont(&h);
    dont_over(&h);
    wide_bound(&h);
    wide_slot(&h);
    wide_via(&h);
    wide_dont(&h);
    wide_tiles(&h);
    wide_levels(&h);
}

/// `.0`, `.2`, `.3` and `.4` on all five levels at once: the five slot marks live on five
/// layers, so one 22 by 150 µm plate per level at the same place asks the same four
/// questions five times over.  The plate is 22 µm wide, under the wide-metal rule's 30, so
/// `.1` has nothing to say; every mark keeps the 10 µm of metal round it that `.5` asks
/// for and the 10 µm from its neighbours that `.4` asks for, bar the one pair that does
/// not.  Bottom to top: an L (a mark that is not a rectangle, `.0`), a 1.995 µm wide mark
/// (`.2`), a 9.995 µm long one (`.3`), and a pair 9.995 µm apart (`.4`).
fn all_levels(h: &H) {
    let mut v = Vec::new();
    for n in 1..=5usize {
        v.push(mplate(h, n, 10.0, 10.0, 22.0, 149.995));
        // The L: a 2 x 20 bar with a 6 µm foot.  The foot comes within 4 µm of the plate's
        // edge, which no rule reads - only rectangles reach .2 through .9.
        v.push(mmark(h, n, 20.0, 20.0, 2.0, 20.0));
        v.push(mmark(h, n, 22.0, 20.0, 6.0, 2.0));
        v.push(mmark(h, n, 20.0, 50.0, 1.995, 20.0));
        v.push(mmark(h, n, 20.0, 80.0, 2.0, 9.995));
        v.push(mmark(h, n, 20.0, 100.0, 2.0, 20.0));
        v.push(mmark(h, n, 20.0, 129.995, 2.0, 20.0));
    }
    hwrite("MSLOT.all.h1", v);
}

/// Which bbox side each size rule reads.  `.2` is the short side of the mark and `.3` the
/// long one, whichever way round the mark lies: a 1.995 by 20 mark and a 20 by 1.995 one
/// are the same violation.  A 2 by 2 square is wide enough for `.2` and too short for `.3`.
fn dim(h: &H) {
    hwrite(
        "MSLOT.dim.h1",
        vec![
            // Upright: 2.0 (clean), then 1.995 wide.
            mplate(h, 1, 10.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 20.0),
            mplate(h, 1, 50.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 60.0, 20.0, 1.995, 20.0),
            // Lying down: 2.0 (clean), then 1.995 tall.
            mplate(h, 1, 90.0, 10.0, 42.0, 22.0),
            mmark(h, 1, 101.0, 20.0, 20.0, 2.0),
            mplate(h, 1, 140.0, 10.0, 42.0, 22.0),
            mmark(h, 1, 151.0, 20.0, 20.0, 1.995),
            // A 2 by 2 square: at .2's bound and under .3's 10 µm.
            mplate(h, 1, 190.0, 10.0, 22.0, 22.0),
            mmark(h, 1, 200.0, 20.0, 2.0, 2.0),
        ],
    );
}

/// `.3`'s two bounds: 10 µm long is legal and 9.995 is not, 250 is legal and 250.005 is
/// not.  Every plate keeps the 10 µm margin `.5` asks for.
fn length(h: &H) {
    hwrite(
        "MSLOT.len.h1",
        vec![
            mplate(h, 1, 10.0, 10.0, 22.0, 30.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 10.0),
            mplate(h, 1, 50.0, 10.0, 22.0, 29.995),
            mmark(h, 1, 60.0, 20.0, 2.0, 9.995),
            mplate(h, 1, 90.0, 10.0, 22.0, 270.0),
            mmark(h, 1, 100.0, 20.0, 2.0, 250.0),
            mplate(h, 1, 140.0, 10.0, 22.0, 270.005),
            mmark(h, 1, 150.0, 20.0, 2.0, 250.005),
        ],
    );
}

/// `.4`'s bound, and who takes part in it.  Two marks 10 µm apart are legal and 9.995 is
/// not; a mark that is not a rectangle is read by `.0` alone, so an L 5 µm from a
/// rectangle is not a `.4` violation - the runset measures `metal_slot.rectangles` and
/// nothing else.
fn space(h: &H) {
    hwrite(
        "MSLOT.space.h1",
        vec![
            mplate(h, 1, 10.0, 10.0, 34.0, 40.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 20.0),
            mmark(h, 1, 32.0, 20.0, 2.0, 20.0),
            mplate(h, 1, 60.0, 10.0, 34.0, 40.0),
            mmark(h, 1, 70.0, 20.0, 2.0, 20.0),
            mmark(h, 1, 81.995, 20.0, 2.0, 20.0),
            // A rectangle and an L, 5 µm apart.
            mplate(h, 1, 110.0, 10.0, 34.0, 40.0),
            mmark(h, 1, 120.0, 20.0, 2.0, 20.0),
            mmark(h, 1, 127.0, 20.0, 2.0, 20.0),
            mmark(h, 1, 129.0, 20.0, 4.0, 2.0),
        ],
    );
}

/// The same 9.995 µm gap three ways over the tile lines: the gap across x = 20, the gap
/// across x = 42, and a mark whose own edge sits on x = 20.
fn space_tiles(h: &H) {
    hwrite(
        "MSLOT.space.h2",
        vec![
            mplate(h, 1, 5.0, 10.0, 34.0, 40.0),
            mmark(h, 1, 15.0, 20.0, 2.0, 20.0),
            mmark(h, 1, 26.995, 20.0, 2.0, 20.0),
            mplate(h, 1, 27.0, 60.0, 34.0, 40.0),
            mmark(h, 1, 37.0, 70.0, 2.0, 20.0),
            mmark(h, 1, 48.995, 70.0, 2.0, 20.0),
            mplate(h, 1, 10.0, 110.0, 34.0, 40.0),
            mmark(h, 1, 20.0, 120.0, 2.0, 20.0),
            mmark(h, 1, 31.995, 120.0, 2.0, 20.0),
        ],
    );
}

/// `.4`'s *other* bound.  The manual's table gives slot space a minimum of 10 µm and a
/// maximum of 30; the deck and the runset both carry the minimum alone.  Two marks 30 µm
/// apart are at the bound, and two 30.005 apart are over it.  The plates are 30 µm tall,
/// so no part of them is wide metal and `.1` cannot stand in for the missing maximum.
fn space_max(h: &H) {
    hwrite(
        "MSLOT.space.h3",
        vec![
            mplate(h, 1, 10.0, 10.0, 54.0, 30.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 10.0),
            mmark(h, 1, 52.0, 20.0, 2.0, 10.0),
            mplate(h, 1, 80.0, 10.0, 54.005, 30.0),
            mmark(h, 1, 90.0, 20.0, 2.0, 10.0),
            mmark(h, 1, 122.005, 20.0, 2.0, 10.0),
        ],
    );
}

/// `.5` on all five levels: 9.995 µm of metal to the left of the mark and 10.005 to the
/// right is one short approach, and a mark with 10 µm all round is clean.
fn enclosure(h: &H) {
    let mut v = Vec::new();
    for n in 1..=5usize {
        v.push(mplate(h, n, 10.0, 10.0, 22.0, 40.0));
        v.push(mmark(h, n, 19.995, 20.0, 2.0, 20.0));
        v.push(mplate(h, n, 50.0, 10.0, 22.0, 40.0));
        v.push(mmark(h, n, 60.0, 20.0, 2.0, 20.0));
    }
    hwrite("MSLOT.enc.h1", v);
}

/// What `.5` does where there is no enclosure to measure: a mark crossing the metal's
/// edge, a mark with no metal under it at all, a mark in a plate drawn on the dummy
/// datatype (which is not `metal1_drawn`, so it is not this deck's metal), and a mark 8 µm
/// from the wall of a notch in the plate rather than from its outer edge.
fn enclosure_edge(h: &H) {
    hwrite(
        "MSLOT.enc.h2",
        vec![
            // Half in, half out.
            mplate(h, 1, 10.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 25.0, 20.0, 10.0, 20.0),
            // No metal at all.
            mmark(h, 1, 60.0, 20.0, 2.0, 20.0),
            // Dummy metal only.
            rect(h.m1_dummy, 90.0, 10.0, 112.0, 50.0),
            mmark(h, 1, 100.0, 20.0, 2.0, 20.0),
            // A U: the mark is 10 µm from the outer edge and 8 from the notch's wall.
            rect(h.metal[0], 140.0, 10.0, 160.0, 70.0),
            rect(h.metal[0], 180.0, 10.0, 200.0, 70.0),
            rect(h.metal[0], 160.0, 10.0, 180.0, 30.0),
            mmark(h, 1, 150.0, 20.0, 2.0, 20.0),
        ],
    );
}

/// MSLOT.10: "Slot mark layer on the metal hole (same metal level, e.g. Metal1_Slot on the
/// Metal1 hole) are not allowed".  A 42 by 60 µm plate with a 22 by 40 hole in it - the
/// ring's arms are 10 µm, so no part of it is wide metal - and a legal 2 by 20 mark in the
/// middle of the hole, 10 µm from every wall of it.  That margin is exactly `.5`'s bound,
/// so `.5` has nothing to say and the mark is in a hole.
fn metal_hole(h: &H) {
    hwrite(
        "MSLOT.hole.h1",
        vec![
            // The ring, cut open along y = 10 so one boundary can carry the hole.
            poly(
                h.metal[0],
                &[
                    (10.0, 10.0),
                    (52.0, 10.0),
                    (52.0, 70.0),
                    (10.0, 70.0),
                    (10.0, 20.0),
                    (20.0, 20.0),
                    (20.0, 60.0),
                    (42.0, 60.0),
                    (42.0, 20.0),
                    (10.0, 20.0),
                ],
            ),
            mmark(h, 1, 30.0, 30.0, 2.0, 20.0),
        ],
    );
}

/// The per-level via map: `.7` is the via above the metal and `.8` the one below it, and
/// below Metal1 that is the contact.  One plate per level, 45 µm apart so no via reaches
/// its neighbour's mark, each with a via 0.195 µm to the right of its mark (the layer
/// above) and one 0.195 µm to the left (the layer below).  Metal5 is this variant's top
/// metal and has no `.7`.
fn via_map(h: &H) {
    let mut v = Vec::new();
    for n in 1..=5usize {
        let x = 10.0 + (n as f64 - 1.0) * 45.0;
        v.push(mplate(h, n, x, 10.0, 22.0, 40.0));
        v.push(mmark(h, n, x + 10.0, 20.0, 2.0, 20.0));
        let below = if n == 1 { h.contact } else { h.via[n - 2] };
        v.push(rect(below, x + 9.545, 29.0, x + 9.805, 29.26));
        if n < 5 {
            v.push(rect(h.via[n - 1], x + 12.195, 29.0, x + 12.455, 29.26));
        }
    }
    hwrite("MSLOT.via.h1", v);
}

/// `.7`'s bound and the approaches that are not a gap: 0.2 µm is legal; a via sharing the
/// mark's edge is a space of nothing; a via wholly inside the mark, and one straddling its
/// edge, have no gap to the mark at all, and a slot cut where a via lands is the very
/// thing the rule is about.  A Via2 beside a Metal1 mark is another level's business.
fn via_bound(h: &H) {
    hwrite(
        "MSLOT.via.h2",
        vec![
            // 0.2 either side: clean.
            mplate(h, 1, 10.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 20.0),
            rect(h.contact, 19.54, 29.0, 19.8, 29.26),
            rect(h.via[0], 22.2, 29.0, 22.46, 29.26),
            // Abutting the mark's right edge.
            mplate(h, 1, 55.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 65.0, 20.0, 2.0, 20.0),
            rect(h.via[0], 67.0, 29.0, 67.26, 29.26),
            // Wholly inside the mark.
            mplate(h, 1, 100.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 110.0, 20.0, 2.0, 20.0),
            rect(h.via[0], 110.8, 29.0, 111.06, 29.26),
            // Straddling the mark's left edge.
            mplate(h, 1, 145.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 155.0, 20.0, 2.0, 20.0),
            rect(h.via[0], 154.87, 29.0, 155.13, 29.26),
            // A Via2 0.195 from a Metal1 mark: the wrong level.
            mplate(h, 1, 190.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 200.0, 20.0, 2.0, 20.0),
            rect(h.via[1], 202.195, 29.0, 202.455, 29.26),
        ],
    );
}

/// The abutments, read once per rule and once at a corner: a via bar sharing the whole of
/// the mark's right edge (`.7`), a contact sharing the whole of its left edge (`.8`), and
/// a via touching one corner of the mark and nothing else (`.7`).  A shared edge is a
/// space of nothing and the smallest space there is.
fn via_abut(h: &H) {
    hwrite(
        "MSLOT.via.h3",
        vec![
            mplate(h, 1, 10.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 20.0, 20.0, 2.0, 20.0),
            rect(h.via[0], 22.0, 20.0, 22.26, 40.0),
            rect(h.contact, 19.74, 20.0, 20.0, 40.0),
            mplate(h, 1, 55.0, 10.0, 22.0, 40.0),
            mmark(h, 1, 65.0, 20.0, 2.0, 20.0),
            rect(h.via[0], 67.0, 40.0, 67.26, 40.26),
        ],
    );
}

/// `.9`'s region and its bound.  Slotting is forbidden under a pad, under a fuse window
/// and under a UBM opening - each read on the *top* metal, Metal5 on this variant - and
/// under a fuse top, which is read on the MIM bottom plate, Metal4.  Each of those is
/// grown by 5 µm before the rule's own 5 µm separation, so the drawn opening has to keep
/// 10 µm from a slot mark.  Eleven plates, each with one legal mark and one opening to its
/// right: the pair that makes a keep-out at 9.995 µm, the same pair at 10, and the
/// combinations that make no keep-out at all.
fn dont(h: &H) {
    let units: [Unit; 11] = [
        (Some(5), Some(h.pad), 9.995),
        (Some(5), Some(h.pad), 10.0),
        (None, Some(h.pad), 9.995),
        (Some(5), None, 9.995),
        (Some(4), Some(h.fusetop), 9.995),
        (Some(5), Some(h.fusetop), 9.995),
        (Some(5), Some(h.fusewindow), 9.995),
        (Some(5), Some(h.ubm[0]), 9.995),
        (Some(5), Some(h.ubm[1]), 9.995),
        (Some(5), Some(h.ubm[2]), 9.995),
        (Some(4), Some(h.pad), 9.995),
    ];
    let mut v = Vec::new();
    for (i, &(lvl, mark, gap)) in units.iter().enumerate() {
        let x = 10.0 + i as f64 * 45.0;
        v.push(mplate(h, 1, x, 10.0, 22.0, 40.0));
        v.push(mmark(h, 1, x + 10.0, 20.0, 2.0, 20.0));
        let ox = x + 12.0 + gap;
        if let Some(n) = lvl {
            v.push(rect(h.metal[n - 1], ox, 27.0, ox + 6.0, 33.0));
        }
        if let Some(l) = mark {
            v.push(rect(l, ox, 27.0, ox + 6.0, 33.0));
        }
    }
    hwrite("MSLOT.dont.h1", v);
}

/// `.9` where the keep-out reaches the mark or covers it, and the region the manual lists
/// that neither deck builds.  Five plates, each with one legal mark: a pad opening over
/// the top metal lying on the mark; the same opening sharing the mark's right edge; one
/// 5 µm away, so that the grown keep-out shares the edge instead; one 10 µm away, the
/// bound; and a Metal3 island enclosed by FuseTop 9.995 µm away - the manual's "Metal3
/// area (for MIM top plate connection) directly enclosed by FuseTop", the fifth of the
/// five regions its `.9` names and the one the runset's `dont_slot` leaves out.
fn dont_over(h: &H) {
    let mut v = Vec::new();
    for (i, gap) in [-3.0f64, 0.0, 5.0, 10.0, 9.995].iter().enumerate() {
        let x = 10.0 + i as f64 * 45.0;
        v.push(mplate(h, 1, x, 10.0, 22.0, 40.0));
        v.push(mmark(h, 1, x + 10.0, 20.0, 2.0, 20.0));
        let ox = x + 12.0 + gap;
        if i == 4 {
            // Metal3 inside FuseTop, rather than the top metal under a pad.
            v.push(rect(h.metal[2], ox, 27.0, ox + 6.0, 33.0));
            v.push(rect(h.fusetop, ox - 2.0, 25.0, ox + 8.0, 35.0));
        } else {
            v.push(rect(h.metal[4], ox, 27.0, ox + 6.0, 33.0));
            v.push(rect(h.pad, ox, 27.0, ox + 6.0, 33.0));
        }
    }
    hwrite("MSLOT.dont.h2", v);
}

/// What `.1` calls a metal line wider than 30 µm.  The runset opens the metal with a 30 by
/// 30 µm square, so the rule reads as "a 30 µm square fits inside": 30 by 30 is clean,
/// 30.005 by 30.005 is not, and 30.005 by 30.0 is clean because the square still does not
/// fit.  A 40 by 20 plate and an L of two 40 by 20 arms are clean by the same reading,
/// although the manual's own words - "any metal line that is wider than 30µm" - would take
/// the 40 µm side of the first at face value.
fn wide_bound(h: &H) {
    hwrite(
        "MSLOT.wide.h1",
        vec![
            mplate(h, 1, 10.0, 10.0, 30.0, 30.0),
            mplate(h, 1, 60.0, 10.0, 30.005, 30.005),
            mplate(h, 1, 110.0, 10.0, 30.005, 30.0),
            mplate(h, 1, 160.0, 10.0, 40.0, 20.0),
            mplate(h, 1, 210.0, 10.0, 40.0, 20.0),
            mplate(h, 1, 210.0, 10.0, 20.0, 40.0),
        ],
    );
}

/// The slot that relieves `.1`, at the bound.  A 2 µm mark down the middle of a 62 by 60
/// µm plate leaves two 30 µm halves and the plate is clean; widen the plate by 0.01 µm and
/// each half is 30.005 and needs a slot of its own.  Both marks keep 10 µm of metal above
/// and below - `.5`'s bound - and 30 to either side.
fn wide_slot(h: &H) {
    hwrite(
        "MSLOT.wide.h2",
        vec![
            mplate(h, 1, 10.0, 10.0, 62.0, 60.0),
            mmark(h, 1, 40.0, 20.0, 2.0, 40.0),
            mplate(h, 1, 110.0, 10.0, 62.01, 60.0),
            mmark(h, 1, 140.005, 20.0, 2.0, 40.0),
        ],
    );
}

/// The vias break a wide plate up too, because a slot may not come within 0.2 µm of one
/// and the runset takes that strip away before it looks for wide metal.  A 1.7 µm via bar
/// down the middle of a 62.01 µm plate grows to 2.1 and leaves 29.955 µm halves: clean.  A
/// 1.5 µm bar grows to 1.9 and leaves 30.055 µm halves, which need slots.
fn wide_via(h: &H) {
    hwrite(
        "MSLOT.wide.h3",
        vec![
            mplate(h, 1, 10.0, 10.0, 62.01, 60.0),
            rect(h.via[0], 40.155, 10.0, 41.855, 70.0),
            mplate(h, 1, 110.0, 10.0, 62.01, 60.0),
            rect(h.via[0], 140.255, 10.0, 141.755, 70.0),
        ],
    );
}

/// And so does a region slotting is forbidden in: a 2 µm pad opening over the top metal
/// grows to 12 µm and leaves 25 µm halves.  The same opening with no top metal under it is
/// no keep-out at all and the plate still needs its slots.
fn wide_dont(h: &H) {
    hwrite(
        "MSLOT.wide.h4",
        vec![
            mplate(h, 1, 10.0, 10.0, 62.01, 60.0),
            rect(h.metal[4], 40.005, 10.0, 42.005, 70.0),
            rect(h.pad, 40.005, 10.0, 42.005, 70.0),
            mplate(h, 1, 110.0, 10.0, 62.01, 60.0),
            rect(h.pad, 140.005, 10.0, 142.005, 70.0),
        ],
    );
}

/// `.1` reads the metal through a 15 µm shrink and a 15 µm grow - twice a 7 µm tile and
/// most of a 20 µm one.  Three 30.005 µm squares: one across x = 7, 14, 20, 21, 28 and 35,
/// one across x = 42, and one on its own at (1000, 1000).
fn wide_tiles(h: &H) {
    hwrite(
        "MSLOT.wide.h5",
        vec![
            mplate(h, 1, 5.0, 10.0, 30.005, 30.005),
            mplate(h, 1, 27.0, 60.0, 30.005, 30.005),
            mplate(h, 1, 1000.0, 1000.0, 30.005, 30.005),
        ],
    );
}

/// One 30.005 µm square per level, all at the same place: five levels, five rules.
fn wide_levels(h: &H) {
    let mut v = Vec::new();
    for n in 1..=5usize {
        v.push(mplate(h, n, 10.0, 10.0, 30.005, 30.005));
    }
    hwrite("MSLOT.wide.h6", v);
}
