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

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/width";

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
}
