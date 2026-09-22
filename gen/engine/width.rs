// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the width family: what the drivers add to the measurement, which the
//! unit tests in `geom` and `checks/width` cannot see - a pair across a tile line, a
//! parameter arriving from a deck, a shape a merge hands back in a form of its own.
//!
//! The tile is 20 µm with lines at its multiples.  Every expected count is read off the
//! drawing: a rectangle has two walls per dimension, a violation is both walls of one
//! span, a pinch is one point.

use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/width";

/// A line of width `w` running right from `(x, y)` for 2 µm, jogging up-right at 45°
/// by `h`, then on for 2 µm: the jog's walls are `wt/√2` apart and each `h·√2` long.
fn zroute(l: (i16, i16), x: f64, y: f64, w: f64, wt: f64, h: f64) -> gds21::GdsElement {
    poly(
        l,
        &[
            (x, y),
            (x + 2.0, y),
            (x + 2.0 + h, y + h),
            (x + 4.0 + h, y + h),
            (x + 4.0 + h, y + h + w),
            (x + 2.0 - wt + w + h, y + h + w),
            (x + 2.0 - wt + w, y + w),
            (x, y + w),
        ],
    )
}

/// An L of lines `w` wide, arms `len` long from the outer corner `(x, y)`, that corner
/// cut along `X + Y = x + y + k` and the inner corner along `X + Y = x + y + kin`.  The
/// two 45° walls are `(kin − k)/√2` apart; the outer one is `k·√2` long, the inner
/// `(kin − 2w)·√2`.
fn chamfered_l(
    l: (i16, i16),
    x: f64,
    y: f64,
    w: f64,
    len: f64,
    k: f64,
    kin: f64,
) -> gds21::GdsElement {
    poly(
        l,
        &[
            (x + k, y),
            (x + len, y),
            (x + len, y + w),
            (x + kin - w, y + w),
            (x + w, y + kin - w),
            (x + w, y + len),
            (x, y + len),
            (x, y + k),
        ],
    )
}

/// A 45° bar of width `w` and run `len`, starting at `(x, y)` and rising to the right,
/// its ends square to the trace.
fn diagonal(l: (i16, i16), x: f64, y: f64, w: f64, len: f64) -> gds21::GdsElement {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let (dx, dy) = (h * len, h * len);
    let (nx, ny) = (h * w, -h * w);
    poly(
        l,
        &[
            (x, y),
            (x + nx, y + ny),
            (x + nx + dx, y + ny + dy),
            (x + dx, y + dy),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A 0.4 µm bar from x = 35 to 44 across the tile line at 40, its midpoint off the
    // line: the tile owning the midpoint reports its two walls, the other does not.
    // W.min: 2.
    write("straddle_min", vec![rect(outer, 35.0, 30.0, 44.0, 30.4)]);

    // A block 12 µm across from 34 to 46, over the line at 40 and over W.max's 10 µm,
    // and 8 µm tall, which is not: the two side walls, once.  W.max: 2.
    write("straddle_max", vec![rect(outer, 34.0, 30.0, 46.0, 38.0)]);

    // A 45° bar 0.6 across and 5 µm long: under W.bent's 0.7, over W.min's 0.5.
    // W.bent: 2, W.min: 0.
    write("bent_trace", vec![diagonal(outer, 30.0, 30.0, 0.6, 5.0)]);

    // The same bar 0.8 µm long, shorter than W.bent's 1 µm run: a chamfer, not a trace.
    // W.bent: 0.
    write("bent_short", vec![diagonal(outer, 30.0, 30.0, 0.6, 0.8)]);

    // A 0.4 µm bar of Inner 6 µm long, and one 4 µm long, under W.len's 5 µm.
    // W.len: 2 and 0.
    write("length_long", vec![rect(inner, 30.0, 30.0, 36.0, 30.4)]);
    write("length_short", vec![rect(inner, 30.0, 30.0, 34.0, 30.4)]);

    // A via drawn 0.3 square, and one 0.3 by 0.32: W.exact reports the dimension that
    // is off, both walls.  W.exact: 0 and 2.
    write("exact_ok", vec![rect(via, 30.0, 30.0, 30.3, 30.3)]);
    write("exact_off", vec![rect(via, 30.0, 30.0, 30.3, 30.32)]);

    // Two 1 µm squares corner to corner: a width of zero at the shared vertex, which a
    // minimum reports as one point and a maximum has no span for.  Each square is 1 µm
    // wide, under nothing else.  W.min: 1, W.max: 0.
    write(
        "pinch",
        vec![
            rect(outer, 30.0, 30.0, 31.0, 31.0),
            rect(outer, 31.0, 31.0, 32.0, 32.0),
        ],
    );

    // A right isosceles triangle 3 µm on a side: two 45° tips, each a width of zero.
    // W.min: 2.
    write(
        "acute",
        vec![poly(outer, &[(30.0, 30.0), (33.0, 30.0), (30.0, 33.0)])],
    );

    // Two 1 µm squares overlapping by 0.2 µm diagonally: one shape with a neck between
    // the upper square's left wall and the lower square's right wall, 0.28 µm at its
    // narrowest, corner to corner with no stretch in common.  The neck has a vertical
    // and a horizontal wall at each end; it is one span.  W.min: 1.
    write(
        "corner",
        vec![
            rect(outer, 30.0, 30.0, 31.0, 31.0),
            rect(outer, 30.8, 30.8, 31.8, 31.8),
        ],
    );

    // --- The hardening patterns: what a rule manual's width asks of any layer, drawn
    // once here for every deck of every PDK (hardening/SPEC.md, "Where a pattern
    // belongs").  W.min is 0.5; a violation is both walls of one span.

    // The bound, long and far.  0.5 is legal, 0.495 (one grid step under) fires in x and
    // in y; a 300 µm bar 0.495 tall counts once; a 0.005 sliver; a bar at (1000, 1000).
    // W.min: 10.
    write(
        "bound",
        vec![
            rect(outer, 2.0, 2.0, 2.5, 4.0),               // clean
            rect(outer, 4.0, 2.0, 4.495, 4.0),             // W.min
            rect(outer, 6.0, 2.0, 8.0, 2.495),             // W.min
            rect(outer, 2.0, 6.0, 302.0, 6.5),             // clean, 300 µm
            rect(outer, 2.0, 8.0, 302.0, 8.495),           // W.min, 300 µm
            rect(outer, 2.0, 10.0, 2.005, 12.0),           // W.min, sliver
            rect(outer, 1000.0, 1000.0, 1000.495, 1002.0), // W.min
        ],
    );

    // 45° geometry.  A diamond's width is a·√2: a = 0.355 → 0.502 clean, a = 0.35 → 0.495
    // fires (four walls); a 45° strip d·√2 the same (two walls); a chamfered box and an L
    // with a chamfered inner corner are wide everywhere and stay clean.  W.min: 6.
    write(
        "bound_45",
        vec![
            diamond(outer, 3.0, 3.0, 0.355),               // clean
            diamond(outer, 6.0, 3.0, 0.35),                // W.min
            strip45(outer, 8.0, 2.0, 3.0, 0.355),          // clean
            strip45(outer, 13.0, 2.0, 3.0, 0.35),          // W.min
            chamfered_tr(outer, 2.0, 6.0, 4.0, 8.0, 11.5), // clean
            poly(
                outer,
                &[
                    (6.0, 6.0),
                    (9.0, 6.0),
                    (9.0, 7.0),
                    (7.5, 7.0),
                    (7.0, 7.5),
                    (7.0, 9.0),
                    (6.0, 9.0),
                ],
            ), // clean
        ],
    );

    // Shapes that merge.  Two overlapping 0.3 boxes whose union is 0.5 wide are clean,
    // 0.495 fires once; five abutting 0.1 slices make 0.5 (clean), 0.1 × 4 + 0.095 make
    // 0.495 (fires); a 0.5 bar drawn as a 5 × 10 grid is clean; a ring with one 0.495 wall
    // fires once; an island 0.495 wide in a ring's hole fires once.  W.min: 8.
    let ring = |x0: f64, y0: f64, x1: f64, y1: f64, hx0: f64, hy0: f64, hx1: f64, hy1: f64| {
        vec![
            rect(outer, x0, y0, x1, hy0),
            rect(outer, x0, hy1, x1, y1),
            rect(outer, x0, hy0, hx0, hy1),
            rect(outer, hx1, hy0, x1, hy1),
        ]
    };
    let mut e = vec![
        rect(outer, 2.0, 2.0, 2.3, 4.0),
        rect(outer, 2.2, 2.0, 2.5, 4.0), // union 0.5 → clean
        rect(outer, 4.0, 2.0, 4.3, 4.0),
        rect(outer, 4.195, 2.0, 4.495, 4.0), // union 0.495 → W.min
    ];
    for i in 0..5 {
        let x = 6.0 + 0.1 * i as f64;
        e.push(rect(outer, x, 2.0, x + 0.1, 4.0)); // 0.5 → clean
    }
    for i in 0..4 {
        let x = 8.0 + 0.1 * i as f64;
        e.push(rect(outer, x, 2.0, x + 0.1, 4.0));
    }
    e.push(rect(outer, 8.4, 2.0, 8.495, 4.0)); // 0.495 → W.min
    for i in 0..5 {
        for j in 0..10 {
            let (x, y) = (10.0 + 0.1 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(outer, x, y, x + 0.1, y + 0.2)); // grid 0.5 → clean
        }
    }
    e.extend(ring(2.0, 6.0, 6.0, 10.0, 2.495, 7.0, 5.0, 9.0)); // left wall 0.495 → W.min
    e.extend(ring(7.0, 6.0, 11.0, 10.0, 8.0, 7.0, 10.0, 9.0));
    e.push(rect(outer, 8.5, 7.5, 9.5, 8.5)); // island 1.0 wide → clean
    e.extend(ring(12.0, 6.0, 16.0, 10.0, 13.0, 7.0, 15.0, 9.0));
    e.push(rect(outer, 13.5, 7.5, 13.995, 8.5)); // island 0.495 → W.min
    write("merge", e);

    // Tile lines.  0.495 bars ending on x = 20, straddling 20, starting on 20, straddling
    // 21, ending on 40, straddling 42, inside a tile at 10; 0.495-tall bars across 20/21
    // and 40/42; an L cornered on x = 20 with the vertical arm narrow.  Ten whatever the
    // tile; the 0.5 controls are clean.  W.min: 20.
    write(
        "tile_lines",
        vec![
            rect(outer, 9.505, 2.0, 10.0, 4.0),
            rect(outer, 19.505, 2.0, 20.0, 4.0),
            rect(outer, 19.75, 6.0, 20.245, 8.0),
            rect(outer, 20.0, 10.0, 20.495, 12.0),
            rect(outer, 20.75, 14.0, 21.245, 16.0),
            rect(outer, 39.505, 2.0, 40.0, 4.0),
            rect(outer, 41.75, 6.0, 42.245, 8.0),
            rect(outer, 15.0, 18.0, 25.0, 18.495),
            rect(outer, 35.0, 18.0, 45.0, 18.495),
            poly(
                outer,
                &[
                    (20.0, 22.0),
                    (23.0, 22.0),
                    (23.0, 23.0),
                    (20.495, 23.0),
                    (20.495, 26.0),
                    (20.0, 26.0),
                ],
            ),
            rect(outer, 19.75, 28.0, 20.25, 30.0), // 0.5 straddling 20 → clean
            rect(outer, 15.0, 32.0, 25.0, 32.5),   // 0.5 across 20 → clean
        ],
    );

    // Fifty 0.495 × 1 bars, flat and as an array reference: hierarchy must not change
    // the answer.  W.min: 100 each.
    let cell = vec![rect(outer, 0.2, 0.2, 0.695, 1.2)];
    write("array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // A comb with three 0.495 teeth (three violations of one polygon) and a U with 0.5
    // arms (clean).  W.min: 6.
    write(
        "comb",
        vec![
            poly(
                outer,
                &[
                    (2.0, 2.0),
                    (6.485, 2.0),
                    (6.485, 5.0),
                    (5.99, 5.0),
                    (5.99, 3.0),
                    (4.495, 3.0),
                    (4.495, 5.0),
                    (4.0, 5.0),
                    (4.0, 3.0),
                    (2.495, 3.0),
                    (2.495, 5.0),
                    (2.0, 5.0),
                ],
            ),
            poly(
                outer,
                &[
                    (8.0, 2.0),
                    (11.0, 2.0),
                    (11.0, 5.0),
                    (10.5, 5.0),
                    (10.5, 3.0),
                    (8.5, 3.0),
                    (8.5, 5.0),
                    (8.0, 5.0),
                ],
            ),
        ],
    );

    // --- The hardening patterns of the bent rule (W.bent: 45° runs longer than 1.0
    // narrower than 0.7; W.min is 0.5).

    // The bound.  45° strips (`strip45`: width d·√2, walls len·√2): 0.707 wide with 4.24
    // walls is clean; 0.693 wide with 4.24 walls fires (two walls); 0.693 with 1.018
    // walls fires, with 0.99 walls (not over 1.0) is clean; a 0.495 strip 4.24 long is
    // W.min and W.bent (two each); a 0.693 diamond has 0.693 edges and is clean.
    // W.bent: 6, W.min: 2.
    write(
        "bent_bound",
        vec![
            strip45(outer, 2.0, 2.0, 3.0, 0.5),    // clean
            strip45(outer, 6.0, 2.0, 3.0, 0.49),   // W.bent
            strip45(outer, 10.0, 2.0, 0.72, 0.49), // W.bent, walls 1.018
            strip45(outer, 12.0, 2.0, 0.7, 0.49),  // clean, walls 0.99
            strip45(outer, 14.0, 2.0, 3.0, 0.35),  // W.min + W.bent
            diamond(outer, 19.0, 3.0, 0.49),       // clean
        ],
    );

    // Real routes.  A 0.6 Z route whose 45° jog is 0.693 wide with 1.018 walls fires;
    // the same jog with 0.99 walls is clean; a 0.707 jog is clean.  An L with a
    // chamfered corner: the 45° walls 0.693 apart, the outer 1.33 and the inner 1.018
    // long, fires; with the inner 0.99 long it is clean (a bend is as long as each of
    // its walls); 0.707 apart it is clean.  W.bent: 4.
    write(
        "bent_routes",
        vec![
            zroute(outer, 2.0, 2.0, 0.6, 0.98, 0.72), // W.bent
            zroute(outer, 2.0, 5.0, 0.6, 0.98, 0.7),  // clean
            zroute(outer, 2.0, 8.0, 0.6, 1.0, 0.72),  // clean
            chamfered_l(outer, 10.0, 2.0, 0.6, 4.0, 0.94, 1.92), // W.bent
            chamfered_l(outer, 15.0, 2.0, 0.6, 4.0, 0.92, 1.9), // clean, inner 0.99
            chamfered_l(outer, 20.0, 2.0, 0.6, 4.0, 0.92, 1.92), // clean, 0.707
        ],
    );

    // Tile lines and far.  The firing Z route with its jog straddling x = 20 (19.8..21),
    // starting on 20, straddling 40 and 42, and at (1000, 1000).  W.bent: 10.
    write(
        "bent_tile_lines",
        vec![
            zroute(outer, 17.8, 2.0, 0.6, 0.98, 0.72),
            zroute(outer, 18.0, 5.0, 0.6, 0.98, 0.72),
            zroute(outer, 37.8, 2.0, 0.6, 0.98, 0.72),
            zroute(outer, 39.8, 5.0, 0.6, 0.98, 0.72),
            zroute(outer, 1000.0, 1000.0, 0.6, 0.98, 0.72),
        ],
    );

    // Fifty firing Z routes, flat and as an array reference.  W.bent: 100 each.
    let cell = vec![zroute(outer, 0.2, 0.2, 0.6, 0.98, 0.72)];
    write("bent_array_flat", flat_array(&cell, 10, 5, 6.0));
    write_gz(
        &format!("{DIR}/bent_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 6.0),
    );

    // Long and small.  A 300 µm 45° strip 0.693 wide fires (two walls); a 45° sliver
    // 0.007 wide fires (and is W.min's).  W.bent: 4.
    write(
        "bent_extremes",
        vec![
            strip45(outer, 2.0, 2.0, 212.0, 0.49),
            strip45(outer, 2.0, 220.0, 2.0, 0.005),
        ],
    );
}
