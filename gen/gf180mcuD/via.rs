// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Via: a good and a bad pattern for every rule in the `via` deck, at every level.
//!
//! The levels are structurally identical and were written by expanding a template, which
//! is exactly the situation where a wrong layer slips into one of them — Via3 reaching
//! for Metal3 above instead of Metal4, say. Generating all four costs nothing and turns
//! that from a silent wrong answer into a failing test.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/via";

/// Via side and ordinary spacing, from `V#.1` and `V#.2a`.
const VIA: f64 = 0.26;
/// Margin that satisfies every enclosure rule in the deck, used wherever a fixture is
/// about something else.
const CLEAR: f64 = 0.2;

struct Level {
    n: usize,
    via: (i16, i16),
    below: (i16, i16),
    above: (i16, i16),
}

/// One via with the given lower/upper margins, in order left, right, bottom, top.
#[allow(clippy::too_many_arguments)]
fn cell(l: &Level, x: f64, y: f64, lo: [f64; 4], up: [f64; 4]) -> Vec<gds21::GdsElement> {
    let box_of =
        |m: (i16, i16), d: [f64; 4]| rect(m, x - d[0], y - d[2], x + VIA + d[1], y + VIA + d[3]);
    vec![
        rect(l.via, x, y, x + VIA, y + VIA),
        box_of(l.below, lo),
        box_of(l.above, up),
    ]
}

/// A `cols`×`rows` via array on `pitch`, fully covered above and below.
fn array(
    l: &Level,
    x: f64,
    y: f64,
    cols: usize,
    rows: usize,
    pitch: f64,
) -> Vec<gds21::GdsElement> {
    let (w, h) = (
        (cols - 1) as f64 * pitch + VIA,
        (rows - 1) as f64 * pitch + VIA,
    );
    let mut v = vec![
        rect(l.below, x - CLEAR, y - CLEAR, x + w + CLEAR, y + h + CLEAR),
        rect(l.above, x - CLEAR, y - CLEAR, x + w + CLEAR, y + h + CLEAR),
    ];
    for i in 0..cols {
        for j in 0..rows {
            let (cx, cy) = (x + i as f64 * pitch, y + j as f64 * pitch);
            v.push(rect(l.via, cx, cy, cx + VIA, cy + VIA));
        }
    }
    v
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    // The drawn layers: the deck's `metalN` are derived unions carrying synthetic layer
    // numbers, which nothing can be drawn on.
    let levels: Vec<Level> = (1..=4)
        .map(|n| Level {
            n,
            via: layer(pdk, &format!("via{n}")),
            below: layer(pdk, &format!("metal{n}_drawn")),
            above: layer(pdk, &format!("metal{}_drawn", n + 1)),
        })
        .collect();

    for l in &levels {
        let n = l.n;
        // The lower-metal margin V#.3 asks for: nothing at Via1, 0.01 above it.
        let lo_min = if n == 1 { 0.0 } else { 0.01 };
        let ok = [CLEAR; 4];
        let mut cases: Vec<(String, Vec<gds21::GdsElement>, Vec<gds21::GdsElement>)> = Vec::new();

        // V#.1 - the via is an exact 0.26 um square.
        cases.push((
            format!("V{n}.1"),
            cell(l, OFFSET, OFFSET, ok, [CLEAR; 4]),
            {
                // Oversized: 0.30 across, so both the width and the max check fire.
                let mut v = cell(l, OFFSET, OFFSET, ok, [CLEAR; 4]);
                v[0] = rect(l.via, OFFSET, OFFSET, OFFSET + 0.30, OFFSET + 0.30);
                v
            },
        ));

        // V#.2a - ordinary spacing between two vias, 0.26 um.
        let pair = |gap: f64| {
            let mut v = cell(l, OFFSET, OFFSET, ok, [CLEAR; 4]);
            v.extend(cell(l, OFFSET + VIA + gap, OFFSET, ok, [CLEAR; 4]));
            v
        };
        cases.push((format!("V{n}.2a"), pair(0.30), pair(0.20)));

        // V#.2b - inside a 4x4 array the spacing rises to 0.36, so a 0.30 gap that is
        // legal for a lone pair is not legal here. The good pattern is a 3x3 at that same
        // 0.30 gap: too small to be an array, so the ordinary rule is all that applies.
        cases.push((
            format!("V{n}.2b"),
            array(l, OFFSET, OFFSET, 3, 3, VIA + 0.30),
            array(l, OFFSET, OFFSET, 4, 4, VIA + 0.30),
        ));

        // V#.3 - the metal below must cover the via, by 0.01 um above Via1.
        cases.push((
            format!("V{n}.3"),
            cell(l, OFFSET, OFFSET, [lo_min + 0.05; 4], [CLEAR; 4]),
            cell(l, OFFSET, OFFSET, [-0.05, CLEAR, CLEAR, CLEAR], [CLEAR; 4]),
        ));

        // V#.3.2 - a lower side under 0.04 forces the sides bordering it to reach 0.06.
        // Good twice over and for different reasons: nothing deficient, then a deficient
        // side whose neighbours are generous.
        let mut good_32 = cell(l, OFFSET, OFFSET, [0.1; 4], [CLEAR; 4]);
        good_32.extend(cell(
            l,
            OFFSET + 5.0,
            OFFSET,
            [0.02, 0.2, 0.2, 0.2],
            [CLEAR; 4],
        ));
        cases.push((
            format!("V{n}.3.2"),
            good_32,
            cell(l, OFFSET, OFFSET, [0.02, 0.2, 0.05, 0.2], [CLEAR; 4]),
        ));

        // V#.3.1 - at the end of a narrow track (under 0.34 um wide, 0.28 um long) the
        // metal below must reach 0.06 past the via, six times the ordinary margin,
        // because a narrow line's tip pulls back during processing. The good pattern
        // makes the track *wide* at the same 0.02 um margin: no narrow line, no line end,
        // nothing for the rule to measure - which is what says it is an EOL rule and not
        // just enclosure with a bigger number.
        let track = |w: f64, tip: f64| {
            let (x, y) = (OFFSET, OFFSET);
            vec![
                rect(l.via, x, y, x + VIA, y + VIA),
                // A track running right from the via, capped `tip` past it on the left.
                rect(
                    l.below,
                    x - tip,
                    y - (w - VIA) / 2.0,
                    x + VIA + 3.0,
                    y + VIA + (w - VIA) / 2.0,
                ),
                rect(
                    l.above,
                    x - CLEAR,
                    y - CLEAR,
                    x + VIA + 3.0,
                    y + VIA + CLEAR,
                ),
            ]
        };
        cases.push((format!("V{n}.3.1"), track(0.5, 0.02), track(0.3, 0.02)));

        // V#.4 - and the metal above must cover it by 0.01 um.
        cases.push((
            format!("V{n}.4"),
            cell(l, OFFSET, OFFSET, ok, [0.06; 4]),
            // Deficient on one side only, so V#.4.2 - whose neighbours stay generous -
            // has nothing to say and this fixture is about V#.4 alone.
            cell(l, OFFSET, OFFSET, ok, [-0.05, CLEAR, CLEAR, CLEAR]),
        ));

        // V#.4.1 - the same end-of-line rule against the metal above.
        let track_up = |w: f64, tip: f64| {
            let (x, y) = (OFFSET, OFFSET);
            vec![
                rect(l.via, x, y, x + VIA, y + VIA),
                rect(
                    l.below,
                    x - CLEAR,
                    y - CLEAR,
                    x + VIA + 3.0,
                    y + VIA + CLEAR,
                ),
                rect(
                    l.above,
                    x - tip,
                    y - (w - VIA) / 2.0,
                    x + VIA + 3.0,
                    y + VIA + (w - VIA) / 2.0,
                ),
            ]
        };
        cases.push((
            format!("V{n}.4.1"),
            track_up(0.5, 0.02),
            track_up(0.3, 0.02),
        ));

        // V#.4.2 - the same adjacent-side rule against the metal above.
        let mut good_42 = cell(l, OFFSET, OFFSET, ok, [0.1; 4]);
        good_42.extend(cell(l, OFFSET + 5.0, OFFSET, ok, [0.02, 0.2, 0.2, 0.2]));
        let mut bad_42 = cell(l, OFFSET, OFFSET, ok, [CLEAR; 4]);
        bad_42[2] = rect(
            l.above,
            OFFSET - 0.02,
            OFFSET - 0.05,
            OFFSET + VIA + 0.2,
            OFFSET + VIA + 0.2,
        );
        cases.push((format!("V{n}.4.2"), good_42, bad_42));

        // V#.3.3 / V#.4.3 - guidance: 0.12 um on every side, for resistance matching.
        // The bad half is short on one side only, and at 0.10 um it is comfortably above
        // everything else that watches these margins (V#.3 wants 0.01, V#.3.2 triggers
        // under 0.04), so the fixture isolates the guidance rule.
        cases.push((
            format!("V{n}.3.3"),
            cell(l, OFFSET, OFFSET, [0.15; 4], [CLEAR; 4]),
            cell(l, OFFSET, OFFSET, [0.10, 0.15, 0.15, 0.15], [CLEAR; 4]),
        ));
        cases.push((
            format!("V{n}.4.3"),
            cell(l, OFFSET, OFFSET, ok, [0.15; 4]),
            cell(l, OFFSET, OFFSET, ok, [0.10, 0.15, 0.15, 0.15]),
        ));

        for (name, good, bad) in cases {
            write_gz(&format!("{DIR}/{name}.good.gds.gz"), library("TOP", good));
            write_gz(&format!("{DIR}/{name}.bad.gds.gz"), library("TOP", bad));
        }
    }
}
