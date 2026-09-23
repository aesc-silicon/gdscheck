// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening patterns for the `via_recommended` deck (hardening/SPEC.md, the GF180MCU
//! section): layouts drawn from section 7.14's third item of V*n*.3 and V*n*.4 -
//! "Minimum Metal[n] / Metal[n+1] overlap of Vian on all sides for minimum Vian
//! resistance variation (guideline): 0.12" - by someone who has not seen the engine.
//!
//! The deck's good/bad pairs live in `via.rs` beside the mandatory via rules, because
//! that is where the level template is; these go to their own directory, because they
//! are read with their own deck (`via_recommended`, the `recommended` suite) and the
//! manual puts the rules in Appendix B, "rules not coded".  The foundry's own runset
//! carries no V*n*.3.3 or V*n*.4.3 at all, so on these layouts the oracle only confirms
//! what the *mandatory* rules make of the geometry; the guideline's answer is the
//! manual's alone.
//!
//! What is drawn here is the guideline's own conditions: 0.12 on every side of a
//! 0.26 µm via and the grid step past it, the diagonal approach of a chamfered metal
//! corner, a via that runs out of its metal or has none, a metal plate that is only one
//! plate after merging, a hole in the metal eating into the margin, a via sitting in a
//! metal ring's hole, and the tile lines.  The generic classes (the bound on a bare
//! layer, both metrics, unions, arrays, a shape far off) are the engine family's and are
//! not redrawn.
//!
//! The findings are in hardening/reports/gf180mcuD/via_recommended.md.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/via_recommended";

/// The via side: V*n*.1 fixes it at 0.26 µm on every level.
const VIA: f64 = 0.26;
/// The guideline's value.
const E: f64 = 0.12;
/// A margin no rule on either metal can object to.
const CLEAR: f64 = 0.3;
/// The manufacturing grid.
const D: f64 = 0.005;

/// One via level: the via, the metal below it and the metal above it.
type Level = ((i16, i16), (i16, i16), (i16, i16));

/// The four levels of variant D.
fn levels(pdk: &PdkConfig) -> [Level; 4] {
    let m = |n: usize| layer(pdk, &format!("metal{n}_drawn"));
    let v = |n: usize| layer(pdk, &format!("via{n}"));
    [
        (v(1), m(1), m(2)),
        (v(2), m(2), m(3)),
        (v(3), m(3), m(4)),
        (v(4), m(4), m(5)),
    ]
}

/// A box around the via at `(x, y)` with the margins `[left, right, bottom, top]`.
fn around(l: (i16, i16), x: f64, y: f64, d: [f64; 4]) -> GdsElement {
    rect(l, x - d[0], y - d[2], x + VIA + d[1], y + VIA + d[3])
}

fn sq(l: (i16, i16), x: f64, y: f64) -> GdsElement {
    rect(l, x, y, x + VIA, y + VIA)
}

/// One via at `(x, y)` with the metal below at margins `lo` and the metal above at `up`.
fn cell((via, below, above): Level, x: f64, y: f64, lo: [f64; 4], up: [f64; 4]) -> Vec<GdsElement> {
    vec![
        around(below, x, y, lo),
        sq(via, x, y),
        around(above, x, y, up),
    ]
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let l = levels(pdk);
    bound(l[0]);
    diagonal(l[0]);
    running_out(l[0]);
    unions_and_holes(l[0]);
    tile_lines(l[0]);
    two_vias_one_plate(l[0]);
    for (i, lvl) in l.iter().enumerate().skip(1) {
        upper_levels(i + 1, *lvl);
    }
}

// --- V1.3.3 / V1.4.3: the bound, on each metal in turn ---

/// `V1.3.3.h1` and `V1.4.3.h1`: 0.12 exactly; 0.115 on one side; 0.115 on two adjacent
/// sides; 0.115 on all four; a margin far past the value; and a side flush with the via.
/// The manual asks 0.12 "on all sides", so each side that falls under it is a side the
/// guideline names - two short sides are two of them and four are four.  The other metal
/// is drawn at 0.3 throughout, so each fixture reads one rule.
fn bound(lvl: Level) {
    for (name, lower) in [("V1.3.3.h1", true), ("V1.4.3.h1", false)] {
        let mut elems = Vec::new();
        for (i, m) in [
            [E; 4],
            [E - D, E, E, E],
            [E - D, E, E - D, E],
            [E - D; 4],
            [E, E, E, 1.0],
            // Three sides generous, one side flush with the via: no overlap at all
            // there, which is as far under 0.12 as a side can go.
            [0.0, E, E, E],
        ]
        .into_iter()
        .enumerate()
        {
            let x = 10.0 + i as f64 * 3.0;
            let (lo, up) = if lower {
                (m, [CLEAR; 4])
            } else {
                ([CLEAR; 4], m)
            };
            elems.extend(cell(lvl, x, 10.0, lo, up));
        }
        write(name, elems);
    }
}

// --- The diagonal approach ---

/// `V1.3.3.h2`: a metal square `m` clear of the via on every axis, with its top-right
/// corner chamfered along `x + y = const` so that `k` is cut off each side.  The
/// chamfer's perpendicular distance to the via's top-right corner is `(2m - k)/sqrt(2)`,
/// while every axis-aligned margin stays at `m`.  An enclosure is read euclidian (the
/// settled reading of 2026-09-21: "a chamfer or a 45° wall passing under the value from
/// a corner ... fires"), so the chamfer is the whole measurement:
///
/// - `m = 0.20`, `k = 0.230`: 0.1202 - clears the value, clean either way.
/// - `m = 0.20`, `k = 0.235`: 0.1167 - fires euclidian; the nearest projection is the
///   0.165 from the via's right wall to the chamfer's lower end, so projection is clean.
/// - `m = 0.15`, `k = 0.150`: 0.1061 - fires euclidian, and the chamfer ends level with
///   the via's top and right edges, so no wall of the via faces it at all and every
///   projection is the full 0.15.  This one cannot be explained away by an edge pair.
///
/// The fourth structure is the last of those against the metal above.
fn diagonal((via, below, above): Level) {
    let chamfer = |l: (i16, i16), x: f64, y: f64, m: f64, k: f64| {
        let (x0, y0) = (x - m, y - m);
        let (x1, y1) = (x + VIA + m, y + VIA + m);
        poly(
            l,
            &[(x0, y0), (x1, y0), (x1, y1 - k), (x1 - k, y1), (x0, y1)],
        )
    };
    let mut elems = Vec::new();
    for (i, (m, k)) in [(0.20, 0.23), (0.20, 0.235), (0.15, 0.15)]
        .into_iter()
        .enumerate()
    {
        let x = 10.0 + i as f64 * 3.0;
        elems.push(chamfer(below, x, 10.0, m, k));
        elems.push(sq(via, x, 10.0));
        elems.push(around(above, x, 10.0, [CLEAR; 4]));
    }
    elems.push(around(below, 19.0, 10.0, [CLEAR; 4]));
    elems.push(sq(via, 19.0, 10.0));
    elems.push(chamfer(above, 19.0, 10.0, 0.15, 0.15));
    write("V1.3.3.h2", elems);
}

// --- A via that runs out of its metal ---

/// `V1.3.3.h3`: the three ways a via can have no overlap at all on a side - the metal
/// stopping half way across it, the metal ending flush with its edge, and no metal under
/// it whatsoever.  "Overlap on all sides" of 0.12 µm is broken hardest by an overlap of
/// nothing, so the manual's answer is that every one of them fires.  A rule that only
/// measures from the via's boundary to the boundary of a metal that *covers* it will
/// miss all three.
fn running_out((via, below, above): Level) {
    // Half covered, flush, and no metal at all - each of them once against the metal
    // below and once against the metal above.
    let half = |l: (i16, i16), x: f64, y: f64| rect(l, x - E, y - E, x + VIA / 2.0, y + VIA + E);
    let away = |l: (i16, i16), x: f64, y: f64| rect(l, x + 1.0, y, x + 2.0, y + 1.0);
    let elems = vec![
        // (a) Metal1 stops half way across the via: its right edge cuts the via in two.
        half(below, 10.0, 10.0),
        sq(via, 10.0, 10.0),
        around(above, 10.0, 10.0, [CLEAR; 4]),
        // (b) Metal1 ends flush with the via's right edge: 0.12 on three sides, 0 on one.
        around(below, 13.0, 10.0, [E, 0.0, E, E]),
        sq(via, 13.0, 10.0),
        around(above, 13.0, 10.0, [CLEAR; 4]),
        // (c) No Metal1 within reach: the nearest plate is 1 µm away.
        away(below, 16.0, 10.0),
        sq(via, 16.0, 10.0),
        around(above, 16.0, 10.0, [CLEAR; 4]),
        // (d) to (f): the same three against the metal above.
        around(below, 10.0, 14.0, [CLEAR; 4]),
        sq(via, 10.0, 14.0),
        half(above, 10.0, 14.0),
        around(below, 13.0, 14.0, [CLEAR; 4]),
        sq(via, 13.0, 14.0),
        around(above, 13.0, 14.0, [E, 0.0, E, E]),
        around(below, 16.0, 14.0, [CLEAR; 4]),
        sq(via, 16.0, 14.0),
        away(above, 16.0, 14.0),
    ];
    write("V1.3.3.h3", elems);
}

// --- Unions, holes and a ring ---

/// `V1.3.3.h4`: what the metal is, once it is merged.
///
/// (a) A plate built from four overlapping boxes that is one plate with 0.12 all round
/// after merging - clean, if the rule reads the union and not the boxes.  Drawn so that
/// two of the seams run straight across the via.
/// (b) A plate 0.30 µm clear of the via with a square hole cut in it whose nearest wall
/// is 0.115 from the via - the metal within the margin is not solid, so the overlap on
/// that side is 0.115 and the guideline is broken.
/// (c) The same hole 0.12 away - clean.
/// (d) A metal ring whose hole holds the whole via, the ring's inner wall 0.12 clear of
/// it on every side: the boundary is 0.12 away but the overlap is nothing at all, so
/// the manual's "Metal[n] overlap of Vian ... on all sides" fires.
fn unions_and_holes((via, below, above): Level) {
    let mut elems = Vec::new();

    // (a) one plate, drawn as four overlapping boxes crossing over the via.
    let (x, y) = (10.0, 10.0);
    let (x0, y0, x1, y1) = (x - E, y - E, x + VIA + E, y + VIA + E);
    let mid = (x0 + x1) / 2.0;
    let midy = (y0 + y1) / 2.0;
    elems.push(rect(below, x0, y0, mid + 0.02, midy + 0.02));
    elems.push(rect(below, mid - 0.02, y0, x1, midy + 0.02));
    elems.push(rect(below, x0, midy - 0.02, mid + 0.02, y1));
    elems.push(rect(below, mid - 0.02, midy - 0.02, x1, y1));
    elems.push(sq(via, x, y));
    elems.push(around(above, x, y, [CLEAR; 4]));

    // (b) and (c): a hole in a generous plate, 0.115 and 0.12 from the via's right edge.
    for (i, g) in [E - D, E].into_iter().enumerate() {
        let (x, y) = (13.0 + i as f64 * 3.0, 10.0);
        let hx = x + VIA + g;
        elems.push(poly(
            below,
            &[
                (x - CLEAR, y - CLEAR),
                (x + VIA + CLEAR, y - CLEAR),
                (x + VIA + CLEAR, y + VIA + CLEAR),
                (x - CLEAR, y + VIA + CLEAR),
                (x - CLEAR, y - CLEAR),
                // the hole, walked back along the same seam
                (hx, y - 0.1),
                (hx, y + VIA + 0.1),
                (hx + 0.1, y + VIA + 0.1),
                (hx + 0.1, y - 0.1),
                (hx, y - 0.1),
                (x - CLEAR, y - CLEAR),
            ],
        ));
        elems.push(sq(via, x, y));
        elems.push(around(above, x, y, [CLEAR; 4]));
    }

    // (d) the via in the hole of a metal ring, 0.12 from its inner wall all round.
    let (x, y) = (19.0, 10.0);
    let (ix0, iy0) = (x - E, y - E);
    let (ix1, iy1) = (x + VIA + E, y + VIA + E);
    let (ox0, oy0) = (ix0 - 0.5, iy0 - 0.5);
    let (ox1, oy1) = (ix1 + 0.5, iy1 + 0.5);
    elems.push(poly(
        below,
        &[
            (ox0, oy0),
            (ox1, oy0),
            (ox1, oy1),
            (ox0, oy1),
            (ox0, oy0),
            (ix0, iy0),
            (ix0, iy1),
            (ix1, iy1),
            (ix1, iy0),
            (ix0, iy0),
            (ox0, oy0),
        ],
    ));
    elems.push(sq(via, x, y));
    elems.push(around(above, x, y, [CLEAR; 4]));

    write("V1.3.3.h4", elems);
}

// --- The tile lines ---

/// `V1.3.3.h5`: the same 0.115 margin three times, with the measured gap laid across
/// x = 20, x = 42 and y = 20.  Nothing here may depend on the tile size.
fn tile_lines(lvl: Level) {
    let mut elems = Vec::new();
    // The via's left edge on the line, its short margin reaching back across it.
    elems.extend(cell(lvl, 20.0, 10.0, [E - D, E, E, E], [CLEAR; 4]));
    elems.extend(cell(lvl, 42.0, 10.0, [E - D, E, E, E], [CLEAR; 4]));
    // The bottom margin across y = 20.
    elems.extend(cell(lvl, 30.0, 20.0, [E, E, E - D, E], [CLEAR; 4]));
    write("V1.3.3.h5", elems);
}

// --- Two vias under one plate ---

/// `V1.3.3.h6`: one Metal1 region carrying two vias, each short on a different side -
/// the left one 0.115 from the plate's left wall, the right one 0.115 under a notch cut
/// into the plate's top.  Every other margin is 0.2 or more, and one Metal2 rectangle
/// covers both at 0.3.
///
/// A real ring or bus has many vias under one plate, and the guideline is about a via,
/// not about a plate: two vias short of the overlap are two vias short of it.  The
/// fixture says whether a second via under the same metal is measured at all.
fn two_vias_one_plate((via, below, above): Level) {
    let (ax, bx, y) = (10.0, 11.0, 10.0);
    // The plate: left wall 0.115 from via A, a notch in its top 0.115 over via B, and
    // 0.2 or more everywhere else.
    let elems = vec![
        poly(
            below,
            &[
                (ax - E + D, y - 0.3),
                (bx + VIA + 0.5, y - 0.3),
                (bx + VIA + 0.5, y + VIA + 0.3),
                (bx + VIA + 0.2, y + VIA + 0.3),
                (bx + VIA + 0.2, y + VIA + E - D),
                (bx - 0.2, y + VIA + E - D),
                (bx - 0.2, y + VIA + 0.3),
                (ax - E + D, y + VIA + 0.3),
            ],
        ),
        sq(via, ax, y),
        sq(via, bx, y),
        rect(above, ax - 0.3, y - 0.3, bx + VIA + 0.5, y + VIA + 0.3),
    ];
    write("V1.3.3.h6", elems);
}

// --- The other three levels ---

/// `V<n>.3.3.h1` for *n* = 2, 3 and 4: one via of the level with the metal below 0.115
/// on its left and the metal above 0.115 on its right, so a level wired to the wrong
/// metal or the wrong via shows up as a missing or a doubled id.  Via4's metal above is
/// Metal5, which is variant D's top metal.
fn upper_levels(n: usize, lvl: Level) {
    let mut elems = Vec::new();
    elems.extend(cell(lvl, 10.0, 10.0, [E - D, E, E, E], [E, E - D, E, E]));
    // A second via of the same level, generous on both metals: it must stay clean.
    elems.extend(cell(lvl, 13.0, 10.0, [E; 4], [E; 4]));
    write(&format!("V{n}.3.3.h1"), elems);
}
