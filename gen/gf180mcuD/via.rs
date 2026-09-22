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
use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
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
    hardening(pdk);
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
        // The lower-metal margin V#.3a/V#.3b asks for: nothing at Via1, 0.01 above it.
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

        // V#.3a / V#.3b - the metal below must cover the via, by 0.01 um above Via1.
        cases.push((
            format!("V{n}.3"),
            cell(l, OFFSET, OFFSET, [lo_min + 0.05; 4], [CLEAR; 4]),
            cell(l, OFFSET, OFFSET, [-0.05, CLEAR, CLEAR, CLEAR], [CLEAR; 4]),
        ));

        // V#.3d - a lower side under 0.04 forces the sides bordering it to reach 0.06.
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

        // V#.3c - at the end of a narrow track (under 0.34 um wide, 0.28 um long) the
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

        // V#.4a - and the metal above must cover it by 0.01 um.
        cases.push((
            format!("V{n}.4"),
            cell(l, OFFSET, OFFSET, ok, [0.06; 4]),
            // Deficient on one side only, so V#.4c - whose neighbours stay generous -
            // has nothing to say and this fixture is about V#.4a alone.
            cell(l, OFFSET, OFFSET, ok, [-0.05, CLEAR, CLEAR, CLEAR]),
        ));

        // V#.4b - the same end-of-line rule against the metal above.
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

        // V#.4c - the same adjacent-side rule against the metal above.
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
        // everything else that watches these margins (V#.3b wants 0.01, V#.3d triggers
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

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.14 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/via/V<n>.<rule>.h<n>.gds.gz` with a case in the
// `hardening_via` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/via.md.
//
// What these draw is the deck's own conditions - the fixed 0.26 µm via and what
// "min/max size" means for a via that is not a square, the 4x4 array threshold with its
// 0.36 spacing and its "projecting" condition, the zero margin Metal1 is allowed under
// Via1 against the 0.01 every level above it asks, the line-end reading with its 0.28 µm
// branch and its 0.34 µm line, the adjacent-side trigger, and the stack the manual
// permits.  The generic classes (the bound on a bare layer, both metrics, 45°, unions,
// notches, fifty at once, a shape far off) are the engine family's and are not redrawn.

/// The contact side, for the stack V1.5 permits.
const CO: f64 = 0.22;

/// Layers the hardening patterns draw on.
struct H {
    via1: (i16, i16),
    via2: (i16, i16),
    via3: (i16, i16),
    co: (i16, i16),
    comp: (i16, i16),
    m1: (i16, i16),
    m2: (i16, i16),
    m3: (i16, i16),
    m4: (i16, i16),
}

impl H {
    fn new(pdk: &PdkConfig) -> Self {
        H {
            via1: layer(pdk, "via1"),
            via2: layer(pdk, "via2"),
            via3: layer(pdk, "via3"),
            co: layer(pdk, "contact"),
            comp: layer(pdk, "comp"),
            m1: layer(pdk, "metal1_drawn"),
            m2: layer(pdk, "metal2_drawn"),
            m3: layer(pdk, "metal3_drawn"),
            m4: layer(pdk, "metal4_drawn"),
        }
    }

    /// A via box at `(x, y)`, `w` by `h`.
    fn v(&self, l: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> GdsElement {
        rect(l, x, y, x + w, y + h)
    }

    /// A square via at `(x, y)`.
    fn sq(&self, l: (i16, i16), x: f64, y: f64) -> GdsElement {
        self.v(l, x, y, VIA, VIA)
    }

    /// A box around the via at `(x, y)` with the margins `[left, right, bottom, top]`; a
    /// negative margin falls short of the via.
    fn around(&self, l: (i16, i16), x: f64, y: f64, d: [f64; 4]) -> GdsElement {
        rect(l, x - d[0], y - d[2], x + VIA + d[1], y + VIA + d[3])
    }

    /// A Via1 at `(x, y)` with the given Metal1 and Metal2 margins.
    fn cell1(&self, x: f64, y: f64, lo: [f64; 4], up: [f64; 4]) -> Vec<GdsElement> {
        vec![
            self.around(self.m1, x, y, lo),
            self.sq(self.via1, x, y),
            self.around(self.m2, x, y, up),
        ]
    }
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A Via1 array at `(x, y)` whose gaps are given one per column / row after the first,
/// with Metal1 and Metal2 over all of it, so one deficient gap can sit inside an
/// otherwise legal array.
fn via_array(h: &H, x: f64, y: f64, gaps_x: &[f64], gaps_y: &[f64]) -> Vec<GdsElement> {
    let pos = |gaps: &[f64], start: f64| -> Vec<f64> {
        let mut v = vec![start];
        for g in gaps {
            v.push(v[v.len() - 1] + VIA + g);
        }
        v
    };
    let xs = pos(gaps_x, x);
    let ys = pos(gaps_y, y);
    let (x1, y1) = (xs[xs.len() - 1] + VIA, ys[ys.len() - 1] + VIA);
    let mut v = vec![
        rect(h.m1, x - 0.3, y - 0.3, x1 + 0.3, y1 + 0.3),
        rect(h.m2, x - 0.3, y - 0.3, x1 + 0.3, y1 + 0.3),
    ];
    for &cx in &xs {
        for &cy in &ys {
            v.push(h.sq(h.via1, cx, cy));
        }
    }
    v
}

fn hardening(pdk: &PdkConfig) {
    let h = H::new(pdk);
    v1_1_h(&h);
    v1_2b_h(&h);
    v1_3a_h(&h);
    v2_3b_h(&h);
    v1_4a_h(&h);
    v1_3c_h(&h);
    v1_3d_h(&h);
    v1_4b_h(&h);
    v1_4c_h(&h);
    v1_5_h(&h);
}

// --- V1.1: min/max Via1 size 0.26 ---

fn v1_1_h(h: &H) {
    // h1 - the size in both directions.  Five vias 1 µm apart between one Metal1 and one
    // Metal2 plate: 0.26 square (clean), then one step tall, one step wide, one step
    // narrow and one step short.  "Min/max" is both bounds, so all four fire.
    let mut v = vec![
        rect(h.m1, 2.4, 2.4, 7.2, 3.1),
        rect(h.m2, 2.4, 2.4, 7.2, 3.1),
    ];
    for (i, (w, t)) in [
        (VIA, VIA),
        (VIA, VIA + 0.005),
        (VIA + 0.005, VIA),
        (VIA - 0.005, VIA),
        (VIA, VIA - 0.005),
    ]
    .into_iter()
    .enumerate()
    {
        v.push(h.v(h.via1, 2.7 + i as f64, 2.7, w, t));
    }
    hwrite("V1.1.h1", v);

    // h2 - a via that is not a square.  (a) two 0.26 squares drawn side by side with no
    // gap: one 0.52 by 0.26 shape after merging, 0.52 across, over the maximum; (b) an L
    // whose arms are both 0.26 wide - every straight run measures 0.26, but the via is
    // not a 0.26 square and its outer edges are 0.52 long; (c) one 0.26 square drawn as
    // two overlapping boxes, a legal via.
    hwrite(
        "V1.1.h2",
        vec![
            rect(h.m1, 2.4, 5.7, 7.2, 7.1),
            rect(h.m2, 2.4, 5.7, 7.2, 7.1),
            h.sq(h.via1, 2.7, 6.0),
            h.sq(h.via1, 2.96, 6.0),
            poly(
                h.via1,
                &[
                    (4.7, 6.0),
                    (5.22, 6.0),
                    (5.22, 6.26),
                    (4.96, 6.26),
                    (4.96, 6.52),
                    (4.7, 6.52),
                ],
            ),
            rect(h.via1, 6.7, 6.0, 6.96, 6.17),
            rect(h.via1, 6.7, 6.09, 6.96, 6.26),
        ],
    );
}

// --- V1.2b: space in a 4x4 or larger Via1 array 0.36 (V1.2a's 0.26 otherwise) ---

fn v1_2b_h(h: &H) {
    // A via beside an array, with the metal it needs of its own.
    let extra = |v: &mut Vec<GdsElement>, x: f64, y: f64| {
        v.push(h.around(h.m1, x, y, [0.3; 4]));
        v.push(h.sq(h.via1, x, y));
        v.push(h.around(h.m2, x, y, [0.3; 4]));
    };
    let pitch = VIA + 0.38;

    // h1 - what makes a group an array.  Every gap is 0.38 except one row gap of 0.355,
    // which is legal for a lone pair (V1.2a asks 0.26) and not inside an array.
    // (a) 4 by 4: an array, so the four pairs across that row gap fire; (b) the same
    // with three columns - twelve vias, not "4x4 or larger", nothing fires; (c) sixteen
    // vias in a single row at 0.355: sixteen is a 4x4's count, but a row is not an
    // array, nothing fires.
    let gy = [0.38, 0.38, 0.355];
    let mut v = via_array(h, 3.0, 3.0, &[0.38; 3], &gy);
    v.extend(via_array(h, 3.0, 8.0, &[0.38; 2], &gy));
    v.extend(via_array(h, 3.0, 13.0, &[0.355; 15], &[]));
    hwrite("V1.2b.h1", v);

    // h2 - how far a via has to be to fall outside the array.  (a) a legal 4x4 at 0.38
    // with a seventeenth via 0.355 to the right of one row: a via in a 4x4 array, so
    // 0.36 applies and it fires; (b) the same beside a 3x3, where 0.26 applies and it is
    // clean; (c) a legal 4x4 with a via 0.42 to the right of one row - too far to belong
    // to the array at all, and 0.42 clears 0.36 anyway, clean; (d) a 4x4 with a
    // deficient row gap of its own and a seventeenth via beside it.
    let mut v = via_array(h, 3.0, 3.0, &[0.38; 3], &[0.38; 3]);
    extra(&mut v, 3.0 + 3.0 * pitch + VIA + 0.355, 3.0 + pitch);
    v.extend(via_array(h, 10.0, 3.0, &[0.38; 2], &[0.38; 2]));
    extra(&mut v, 10.0 + 2.0 * pitch + VIA + 0.355, 3.0 + pitch);
    v.extend(via_array(h, 3.0, 10.0, &[0.38; 3], &[0.38; 3]));
    extra(&mut v, 3.0 + 3.0 * pitch + VIA + 0.42, 10.0 + pitch);
    // (d) a 4x4 whose own middle row gap is 0.355, with a seventeenth via 0.355 to the
    // right as well: the array's four deficient pairs fire whatever the extra via does.
    v.extend(via_array(h, 10.0, 10.0, &[0.38; 3], &[0.38, 0.355, 0.38]));
    extra(&mut v, 10.0 + 3.0 * pitch + VIA + 0.355, 10.0 + pitch);
    hwrite("V1.2b.h2", v);

    // h3 - the array rule's "projecting >= 0.26" condition: it reads a pair only where
    // the two vias face each other over a via's full side.  Beside a legal 4x4 at 0.38,
    // a via 0.30 to the right of a row: (a) level with it, facing over the whole 0.26,
    // fires; (b) one grid step up, facing over 0.255, which the condition drops - and
    // V1.2a's 0.26 is cleared by the 0.30 gap, so it is clean; (c) 0.15 up, facing over
    // 0.11, clean.
    let mut v = via_array(h, 3.0, 3.0, &[0.38; 3], &[0.38; 3]);
    extra(&mut v, 3.0 + 3.0 * pitch + VIA + 0.30, 3.0 + pitch);
    v.extend(via_array(h, 10.0, 3.0, &[0.38; 3], &[0.38; 3]));
    extra(&mut v, 10.0 + 3.0 * pitch + VIA + 0.30, 3.0 + pitch + 0.005);
    v.extend(via_array(h, 3.0, 10.0, &[0.38; 3], &[0.38; 3]));
    extra(&mut v, 3.0 + 3.0 * pitch + VIA + 0.30, 10.0 + pitch + 0.15);
    hwrite("V1.2b.h3", v);

    // h4 - the same array on the tile lines: the cluster that makes it an array has to
    // be found across a cut.  Three 4x4 arrays with one 0.355 row gap, straddling
    // x = 20, x = 42 and y = 21.
    let mut v = via_array(h, 19.2, 3.0, &[0.38; 3], &gy);
    v.extend(via_array(h, 41.2, 3.0, &[0.38; 3], &gy));
    v.extend(via_array(h, 3.0, 20.2, &[0.38; 3], &gy));
    hwrite("V1.2b.h4", v);
}

// --- V1.3a: Metal1 overlap of Via1 >= 0 ---

fn v1_3a_h(h: &H) {
    // h1 - the one level whose lower metal may sit flush with the via.  (a) Metal1's
    // edge exactly on the via's on the left, 0.06 above and below so V1.3d's adjacent
    // sides are satisfied: an overlap of nothing is what the rule allows, clean;
    // (b) the via 0.005 outside Metal1 on the left - an overlap of less than nothing;
    // (c) a via with no Metal1 under it at all; (d) Metal1 drawn as the via's own
    // square, flush on all four sides: V1.3a is happy, but every side overlaps by less
    // than 0.04 and no side reaches 0.06, so V1.3d is not.
    let mut v = h.cell1(3.0, 3.0, [0.0, 0.3, 0.06, 0.06], [0.3; 4]);
    v.extend(h.cell1(8.0, 3.0, [-0.005, 0.3, 0.3, 0.3], [0.3; 4]));
    v.extend(vec![
        h.sq(h.via1, 13.0, 3.0),
        h.around(h.m2, 13.0, 3.0, [0.3; 4]),
    ]);
    v.extend(h.cell1(3.0, 8.0, [0.0; 4], [0.3; 4]));
    hwrite("V1.3a.h1", v);
}

// --- V2.3b: Metal2 overlap of Via2 0.01 ---

fn v2_3b_h(h: &H) {
    // h1 - every level above Via1 asks 0.01 of the metal below.  (a) 0.01 on the left
    // with 0.06 above and below, at the bound, clean; (b) 0.005; (c) flush, which Via1
    // is allowed and Via2 is not.
    let cell = |x: f64, y: f64, lo: [f64; 4]| -> Vec<GdsElement> {
        vec![
            h.around(h.m2, x, y, lo),
            h.sq(h.via2, x, y),
            h.around(h.m3, x, y, [0.3; 4]),
        ]
    };
    let mut v = cell(3.0, 3.0, [0.01, 0.3, 0.06, 0.06]);
    v.extend(cell(8.0, 3.0, [0.005, 0.3, 0.06, 0.06]));
    v.extend(cell(13.0, 3.0, [0.0, 0.3, 0.06, 0.06]));
    hwrite("V2.3b.h1", v);
}

// --- V1.4a: Metal2 overlap of Via1 0.01 ---

fn v1_4a_h(h: &H) {
    // h1 - the metal above asks 0.01 at every level, Via1 included.  (a) 0.01 on the
    // left with 0.06 above and below, clean; (b) 0.005; (c) no Metal2 over the via at
    // all.
    let mut v = h.cell1(3.0, 3.0, [0.3; 4], [0.01, 0.3, 0.06, 0.06]);
    v.extend(h.cell1(8.0, 3.0, [0.3; 4], [0.005, 0.3, 0.06, 0.06]));
    v.extend(vec![
        h.around(h.m1, 13.0, 3.0, [0.3; 4]),
        h.sq(h.via1, 13.0, 3.0),
    ]);
    hwrite("V1.4a.h1", v);
}

// --- V1.3c: Metal1 (< 0.34 µm) end-of-line overlap of Via1 0.06 ---

fn v1_3c_h(h: &H) {
    // A via at the capped end of a Metal1 track `w` wide, the cap `tip` past it.
    let track = |x: f64, y: f64, w: f64, tip: f64| -> Vec<GdsElement> {
        let m = (w - VIA) / 2.0;
        vec![
            rect(h.m1, x - tip, y - m, x + VIA + 3.0, y + VIA + m),
            h.sq(h.via1, x, y),
            h.around(h.m2, x, y, [0.3, 3.0, 0.3, 0.3]),
        ]
    };
    // h1 - which track is a line end.  The note says "< 0.34 µm wide", so a 0.34 µm
    // track is not one - and 0.34 is the only width at which this rule can be read on
    // its own, because a via in anything narrower has margins under 0.04 and so trips
    // V1.3d as well.  (a) 0.34 wide with the cap 0.06 past the via, clean; (b) 0.34
    // wide at 0.055, still not a narrow line, clean; (c) 0.35 wide at 0.055, clean;
    // (d) 0.32 wide at 0.055, narrow, fires (with V1.3d, which the case ignores).
    let mut v = track(3.0, 3.0, 0.34, 0.06);
    v.extend(track(3.0, 6.0, 0.34, 0.055));
    v.extend(track(3.0, 9.0, 0.35, 0.055));
    v.extend(track(3.0, 12.0, 0.32, 0.055));
    hwrite("V1.3c.h1", v);

    // h2 - the note's other half.  A via's branch is short below 0.28 µm, where the
    // contact rule's is 0.24: a wide Metal1 plate with a 0.32 µm branch out of its left
    // edge, the via at the branch's tip with 0.055 to the cap.  (a) a 0.275 µm branch,
    // not a line end, clean; (b) 0.28 exactly, a line end, fires; (c) 0.35, fires.  A
    // 0.32 µm branch leaves 0.03 above and below the via, so V1.3d fires on all three
    // and the case ignores it.
    let branch = |x: f64, y: f64, len: f64| -> Vec<GdsElement> {
        let tip = x + 0.5 - len;
        vec![
            rect(h.m1, tip, y - 0.03, x + 0.5, y + VIA + 0.03),
            rect(h.m1, x + 0.5, y - 0.5, x + 3.0, y + VIA + 0.5),
            h.sq(h.via1, tip + 0.055, y),
            h.around(h.m2, tip + 0.055, y, [0.3; 4]),
        ]
    };
    let mut v = branch(3.0, 3.0, 0.275);
    v.extend(branch(3.0, 7.0, 0.28));
    v.extend(branch(3.0, 11.0, 0.35));
    hwrite("V1.3c.h2", v);
}

// --- V1.3d: if Metal1 overlaps Via1 by < 0.04 on one side, the adjacent edges 0.06 ---

fn v1_3d_h(h: &H) {
    // The metal is drawn large in the directions the case is not about, so the track is
    // never a narrow line and V1.3c has nothing to say.
    // h1 - the trigger and the value.  (a) 0.035 on the left with 0.06 below it, at the
    // bound, clean; (b) 0.035 on the left with 0.055 below it, fires; (c) 0.04 on the
    // left is not "< 0.04", so it does not trigger and 0.055 below is clean; (d) two
    // adjacent sides both under 0.04, fires; (e) the two opposite sides under 0.04 with
    // the sides beside them generous, clean.
    let mut v = h.cell1(3.0, 3.0, [0.035, 2.0, 0.06, 2.0], [0.3; 4]);
    v.extend(h.cell1(9.0, 3.0, [0.035, 2.0, 0.055, 2.0], [0.3; 4]));
    v.extend(h.cell1(15.0, 3.0, [0.04, 2.0, 0.055, 2.0], [0.3; 4]));
    v.extend(h.cell1(3.0, 9.0, [0.035, 2.0, 0.035, 2.0], [0.3; 4]));
    v.extend(h.cell1(9.0, 9.0, [0.035, 0.035, 2.0, 2.0], [0.3; 4]));
    hwrite("V1.3d.h1", v);
}

// --- V1.4b: Metal2 (< 0.34 µm) end-of-line overlap of Via1 0.06 ---

fn v1_4b_h(h: &H) {
    let track = |x: f64, y: f64, w: f64, tip: f64| -> Vec<GdsElement> {
        let m = (w - VIA) / 2.0;
        vec![
            h.around(h.m1, x, y, [0.3, 3.0, 0.3, 0.3]),
            h.sq(h.via1, x, y),
            rect(h.m2, x - tip, y - m, x + VIA + 3.0, y + VIA + m),
        ]
    };
    // h1 - the same reading against the metal above.  (a) 0.34 wide, cap 0.06, clean;
    // (b) 0.34 wide, cap 0.055, not a narrow line, clean; (c) 0.35 wide, cap 0.055,
    // clean - 0.055 clears V1.4a's 0.01; (d) 0.32 wide, cap 0.055, fires (with V1.4c,
    // which the case ignores).
    let mut v = track(3.0, 3.0, 0.34, 0.06);
    v.extend(track(3.0, 6.0, 0.34, 0.055));
    v.extend(track(3.0, 9.0, 0.35, 0.055));
    v.extend(track(3.0, 12.0, 0.32, 0.055));
    hwrite("V1.4b.h1", v);
}

// --- V1.4c: if Metal2 overlaps Via1 by < 0.04 on one side, the adjacent edges 0.06 ---

fn v1_4c_h(h: &H) {
    // h1 - (a) 0.035 on the left with 0.055 below it, fires; (b) 0.04 on the left does
    // not trigger, so 0.055 below is clean.
    let mut v = h.cell1(3.0, 3.0, [0.3; 4], [0.035, 2.0, 0.055, 2.0]);
    v.extend(h.cell1(9.0, 3.0, [0.3; 4], [0.04, 2.0, 0.055, 2.0]));
    hwrite("V1.4c.h1", v);
}

// --- V1.5: a Vian stack over a contact is permitted ---

fn v1_5_h(h: &H) {
    // h1 - the manual permits the stack, so nothing in the deck may forbid it: a
    // contact, Via1, Via2 and Via3 all on the same centre with every metal over them.
    // The contact is 0.22 and the vias 0.26, so the stack is drawn concentric.
    let (x, y) = (3.0, 3.0);
    let c = x + (VIA - CO) / 2.0;
    hwrite(
        "V1.5.h1",
        vec![
            rect(h.comp, c - 0.3, c - 0.3, c + CO + 0.3, c + CO + 0.3),
            rect(h.co, c, c, c + CO, c + CO),
            h.around(h.m1, x, y, [0.3; 4]),
            h.sq(h.via1, x, y),
            h.around(h.m2, x, y, [0.3; 4]),
            h.sq(h.via2, x, y),
            h.around(h.m3, x, y, [0.3; 4]),
            h.sq(h.via3, x, y),
            h.around(h.m4, x, y, [0.3; 4]),
        ],
    );
}
