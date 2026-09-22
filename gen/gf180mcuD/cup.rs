// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cu pillar: a good and a bad pattern for every rule in the `cup` deck.
//!
//! Two rules at each of five levels, and the first here whose layer has to be *made*
//! before it can be violated: the deck measures metal that a PAD marker lands on and no
//! guard ring claims, so every fixture draws the pad over the metal.  Without it the
//! layer is empty and both halves pass for the wrong reason - which is the failure a
//! drawn pattern exists to rule out.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/cup";
const O: f64 = 10.0;
const D: f64 = 0.005;
/// Both rules ask the same 1 µm of a bond-pad metal.
const LIMIT: f64 = 1.0;

const LEVELS: &[&str] = &[
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

pub fn generate(pdk: &PdkConfig) {
    let pad = layer(pdk, "pad");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // The deck names both rules once per level, so one fixture per rule covers all five:
    // each carries the pad over metal at every level, and the level under test is the one
    // drawn short.  A rule id is shared across levels, so a per-level fixture would need
    // a name the deck cannot give it.
    for (id, narrow) in [("CUP.2", true), ("CUP.3", false)] {
        for (name, v) in [("good", LIMIT), ("bad", LIMIT - D)] {
            let mut elems = Vec::new();
            for (i, lname) in LEVELS.iter().enumerate() {
                let m = layer(pdk, lname);
                let y = O + i as f64 * 20.0;
                if narrow {
                    // CUP.2: a bar one micron wide, or a whisker under it.
                    elems.push(rect(m, O, y, O + v, y + 8.0));
                    elems.push(rect(pad, O - 0.5, y + 1.0, O + v + 0.5, y + 3.0));
                } else {
                    // CUP.3: a C whose opening is the notch under test.
                    elems.push(rect(m, O, y, O + 2.0, y + 4.0 + v));
                    elems.push(rect(m, O + 2.0, y, O + 6.0, y + 2.0));
                    elems.push(rect(m, O + 2.0, y + 2.0 + v, O + 6.0, y + 4.0 + v));
                    // The pad sits on the notch: the rule keeps only the measurements
                    // that meet the pad, so a notch the pad does not reach is nobody's.
                    elems.push(rect(pad, O + 1.5, y + 1.5, O + 6.5, y + 2.5 + v));
                }
            }
            write(&format!("{id}.{name}"), elems);
        }
    }

    hardening(pdk);
    pad_reach(pdk);
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) ------------------------------------------------
//
// Neither rule is about the metal on its own: the deck first picks out the metal a PAD
// marker lands on and no guard ring claims, and then keeps only the measurements that
// reach the pad.  So the patterns below are about that selection - a marker touching the
// metal rather than covering it, a guard ring that takes a whole polygon away, a narrow
// stretch too far from the pad to be a bond pad's, drawn metal against dummy - and about
// what happens to it when the metal crosses a tile line and the pad does not.

const NARROW: f64 = 1.0 - D;

/// A C whose opening faces right: the notch is `g` wide and 4 µm deep, at (x + 2, y + 2).
fn c_shape(m: (i16, i16), x: f64, y: f64, g: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(m, x, y, x + 2.0, y + 4.0 + g),
        rect(m, x + 2.0, y, x + 6.0, y + 2.0),
        rect(m, x + 2.0, y + 2.0 + g, x + 6.0, y + 4.0 + g),
    ]
}

fn hardening(pdk: &PdkConfig) {
    let pad = layer(pdk, "pad");
    let gr = layer(pdk, "guard_ring_mk");
    let m1 = layer(pdk, "metal1_drawn");
    let m5 = layer(pdk, "metal5_drawn");

    // CUP.2.h1: the guard ring takes the whole polygon away, and it has to be the metal it
    // touches.  Row 1: a 0.995 bar with GUARD_RING_MK abutting it - a guard ring's own
    // metal is not a bond pad's, clean.  Row 2: the marker 0.505 clear of the bar, fires.
    // Row 3: the marker touching the *pad* but not the metal - the bar is still a bond
    // pad's, fires.  Row 4: row 1 again on Metal5, the top metal of this variant.
    write("CUP.2.h1", {
        let mut v = Vec::new();
        for (m, y, mk) in [
            (m1, 10.0, Some((10.995, 13.0, 10.0, 18.0))),
            (m1, 30.0, Some((11.5, 13.0, 30.0, 38.0))),
            (m1, 50.0, Some((11.5, 13.0, 51.0, 53.0))),
            (m5, 70.0, Some((10.995, 13.0, 70.0, 78.0))),
        ] {
            v.push(rect(m, 10.0, y, 10.0 + NARROW, y + 8.0));
            v.push(rect(pad, 9.5, y + 1.0, 11.5, y + 3.0));
            if let Some((x0, x1, y0, y1)) = mk {
                v.push(rect(gr, x0, y0, x1, y1));
            }
        }
        v
    });

    // CUP.2.h2: an L - a 3 µm block under the pad and a 0.995 arm running 32 µm away from
    // it, across the tile lines at 20 and 42.  The polygon is a bond pad's, but the narrow
    // stretch is not under the pad, and the rule is about "the metal line used for bond
    // pads".  Clean.
    write("CUP.2.h2", {
        vec![
            rect(m1, 10.0, 10.0, 13.0, 18.0),
            rect(m1, 13.0, 10.0, 45.0, 10.0 + NARROW),
            rect(pad, 10.0, 14.0, 13.0, 17.0),
        ]
    });

    // CUP.2.h3: the same 0.995 bar with the pad over its middle, so the narrow stretch is
    // the pad's.  The bar crosses the tile lines at 20 and 42 and the pad crosses none:
    // the answer is the bar's two walls whatever the tile size.
    write("CUP.2.h3", {
        vec![
            rect(m1, 10.0, 10.0, 45.0, 10.0 + NARROW),
            rect(pad, 12.0, 9.5, 18.0, 11.5),
        ]
    });

    // CUP.2.h4: the same 0.995 bar drawn as *dummy* metal under a pad, on Metal1 and on
    // Metal5.  The rule reads the drawn layer; fill is not a bond pad's metal line, and a
    // pad would not be placed on it.  Clean.
    write("CUP.2.h4", {
        let mut v = Vec::new();
        for (name, y) in [("metal1_dummy", 10.0), ("metal5_dummy", 30.0)] {
            let m = layer(pdk, name);
            v.push(rect(m, 10.0, y, 10.0 + NARROW, y + 8.0));
            v.push(rect(pad, 9.5, y + 1.0, 11.5, y + 3.0));
        }
        v
    });

    // CUP.2.h5: the pad abuts the bar's left edge without covering any of it.  The metal
    // the marker lands on is the metal it touches, and the narrow wall it shares an edge
    // with is under the pad's own boundary.  Fires.
    write("CUP.2.h5", {
        vec![
            rect(m1, 10.0, 10.0, 10.0 + NARROW, 18.0),
            rect(pad, 8.0, 10.0, 10.0, 18.0),
        ]
    });

    // CUP.3.h1: the slot, and how far the pad has to reach for it to be one.  Top: a
    // 0.995 opening with the pad over it, fires.  Middle: the same opening with the pad on
    // the far side of the C, 0.5 short of the slot - the polygon is a bond pad's but this
    // slot is not under the pad, clean.  Bottom: a 1.0 opening under the pad, clean.
    write("CUP.3.h1", {
        let mut v = Vec::new();
        v.extend(c_shape(m1, 10.0, 10.0, NARROW));
        v.push(rect(pad, 11.5, 11.5, 16.5, 12.5 + NARROW));
        v.extend(c_shape(m1, 10.0, 30.0, NARROW));
        v.push(rect(pad, 9.5, 30.2, 11.5, 31.8));
        v.extend(c_shape(m1, 10.0, 50.0, 1.0));
        v.push(rect(pad, 11.5, 51.5, 16.5, 53.5));
        v
    });

    // CUP.3.h2: the guard ring takes the slot away with the polygon.  Top: GUARD_RING_MK
    // abutting the C's spine, clean.  Bottom: the same marker 0.5 clear of it, fires.
    write("CUP.3.h2", {
        let mut v = Vec::new();
        v.extend(c_shape(m1, 10.0, 10.0, NARROW));
        v.push(rect(pad, 11.5, 11.5, 16.5, 12.5 + NARROW));
        v.push(rect(gr, 9.0, 10.0, 10.0, 14.0 + NARROW));
        v.extend(c_shape(m1, 10.0, 30.0, NARROW));
        v.push(rect(pad, 11.5, 31.5, 16.5, 32.5 + NARROW));
        v.push(rect(gr, 8.5, 30.0, 9.5, 34.0 + NARROW));
        v
    });
}

/// One 0.995 µm line and one pad on it, ten times over.  Nothing varies but the line's
/// length and where along it the pad sits, and none of that changes the violation: the
/// line is under the minimum width where the pad is, every time.
fn pad_reach(pdk: &PdkConfig) {
    let pad = layer(pdk, "pad");
    let m1 = layer(pdk, "metal1_drawn");

    // A line lying on its side from x = 10 to `end`, with a pad from `px0` to `px1` that
    // covers its whole width there.
    let line = |end: f64, px0: f64, px1: f64| {
        vec![
            rect(m1, 10.0, 10.0, end, 10.0 + NARROW),
            rect(pad, px0, 9.5, px1, 11.5),
        ]
    };

    // CUP.2.h6: the 35 µm line of h3 with the pad over the whole of it, the tile lines at
    // 20 and 42 included.
    write("CUP.2.h6", line(45.0, 9.5, 45.5));
    // CUP.2.h7: an 8 µm line well inside one tile, with a 2 µm pad across its middle -
    // the deck's own CUP.2 fixture turned 90°.
    write("CUP.2.h7", line(18.0, 12.0, 14.0));
    // CUP.2.h9: a 20 µm line across one tile line, with a 6 µm pad near its left end.
    write("CUP.2.h9", line(30.0, 12.0, 18.0));
    // CUP.2.h10: the 35 µm line of h3 with a 2 µm pad instead of a 6 µm one.
    write("CUP.2.h10", line(45.0, 12.0, 14.0));
    // CUP.2.h11: the 8 µm line of h7 with the 6 µm pad of h3 on it.
    write("CUP.2.h11", line(18.0, 12.0, 18.0));
    // CUP.2.h13: the 35 µm line of h3 with its pad moved to the middle instead of near
    // the left end.  The same line, the same pad, the same violation.
    write("CUP.2.h13", line(45.0, 24.5, 30.5));

    // CUP.2.h8: an upright 8 µm line with the pad *inside* its width, reaching neither
    // wall.  The 0.995 µm the pad sits on is still the width of a bond pad's metal line.
    write("CUP.2.h8", {
        vec![
            rect(m1, 10.0, 10.0, 10.0 + NARROW, 18.0),
            rect(pad, 10.2, 12.0, 10.8, 14.0),
        ]
    });

    // CUP.2.h12: five lines of 16, 20, 24, 28 and 32 µm with the same pad in the same
    // place on each.  Ten walls under the minimum width, however far each line runs on.
    write("CUP.2.h12", {
        let mut v = Vec::new();
        for (i, len) in [16.0, 20.0, 24.0, 28.0, 32.0].iter().enumerate() {
            let y = 10.0 + i as f64 * 5.0;
            v.push(rect(m1, 10.0, y, 10.0 + len, y + NARROW));
            v.push(rect(pad, 12.0, y - 0.5, 18.0, y + NARROW + 0.5));
        }
        v
    });
}
