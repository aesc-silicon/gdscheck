// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the grid readings of the shape family
//! ([`offgrid`](gdscheck::checks::shape::offgrid) and
//! [`offtrack`](gdscheck::checks::shape::offtrack)): vertices on and off a grid, from
//! the origin on both coordinates and from an offset on one; and regions whose
//! centreline across an axis is on and off a pitch.  A grid has no margin to sweep: a
//! coordinate is on it or it is not, so each pattern puts one on and one a known
//! distance off, and the count is the vertices or the regions off.

use crate::helpers::{chamfered_tr, layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/grid";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // G.both, Outer's vertices on a 0.5 µm grid both ways.  A square on it: 0.  One
    // whose x is 0.2 off: 4.  One on it with its top-right corner cut 0.2 in, the two
    // new vertices one coordinate off each: 2.  One at negative coordinates, x 0.2 off:
    // 4.
    write("grid_on", vec![rect(outer, 30.0, 30.0, 31.0, 31.0)]);
    write("grid_x_off", vec![rect(outer, 35.2, 30.0, 36.2, 31.0)]);
    write(
        "grid_corner",
        vec![chamfered_tr(outer, 30.0, 30.0, 31.0, 31.0, 0.2)],
    );
    write("grid_negative", vec![rect(outer, -10.3, -10.0, -9.3, -9.0)]);
    // A small square 0.3 off the grid inside a larger one on it: merged, its corners
    // are nobody's: 0.  An off-grid square across the tile line at 20 µm: 4, each
    // vertex counted once.
    write(
        "grid_inner",
        vec![
            rect(outer, 40.0, 30.0, 41.0, 31.0),
            rect(outer, 40.3, 30.3, 40.7, 30.7),
        ],
    );
    write("grid_tile_line", vec![rect(outer, 19.3, 35.0, 20.7, 36.0)]);

    // G.y, Inner's y on the 0.5 µm grid from 0.2, its x free.  y from 30.2 to 30.7 with
    // x ending at 31.3: 0.  y from 30.0 to 30.5, the grid without its offset: 4.  At
    // negative y, -0.3 to 0.2 is on and -0.1 to 0.4 off: 0 and 4.  Across the tile line
    // at y = 20, from 19.7 to 20.7: 0, the cut at the line making no vertex of its own.
    write("offset_on", vec![rect(inner, 30.0, 30.2, 31.3, 30.7)]);
    write("offset_off", vec![rect(inner, 30.0, 30.0, 31.0, 30.5)]);
    write(
        "offset_negative_on",
        vec![rect(inner, -5.0, -0.3, -4.0, 0.2)],
    );
    write(
        "offset_negative_off",
        vec![rect(inner, -5.0, -0.1, -4.0, 0.4)],
    );
    write(
        "offset_tile_line",
        vec![rect(inner, 30.0, 19.7, 31.0, 20.7)],
    );

    // T.y, Via's centre across y on tracks every micron from 0.3; T.x reads the same
    // shapes across x from the origin, so every shape here runs from x 29.5 to 30.5,
    // centre 30, unless it is T.x's.  Centre y 30.3: T.y 0.  Centre 30.5: T.y 1.
    write("track_on", vec![rect(via, 29.5, 29.8, 30.5, 30.8)]);
    write("track_off", vec![rect(via, 29.5, 30.0, 30.5, 31.0)]);
    // x from 29.5 to 30.501, centre 30.0005: half a DBU off its track, and read as such
    // rather than rounded onto it: T.x 1.  y on: T.y 0.
    write("track_half", vec![rect(via, 29.5, 29.8, 30.501, 30.8)]);
    // Across the tile line at y = 20: one from 19.8 to 20.8, centre 20.3, read whole
    // across the line: T.y 0.  One from 19.5 to 20.5, centre 20: T.y 1, not one per
    // piece.
    write(
        "track_tile_line",
        vec![
            rect(via, 29.5, 19.8, 30.5, 20.8),
            rect(via, 34.5, 19.5, 35.5, 20.5),
        ],
    );
    // An L whose box runs across y from `y0` to `y0 + 1`: the centre of the box is
    // what is read.  From 29.8, centre 30.3: T.y 0.  From 30.0, centre 30.5: T.y 1.
    let ell = |y0: f64| {
        poly(
            via,
            &[
                (29.5, y0),
                (30.5, y0),
                (30.5, y0 + 0.4),
                (30.0, y0 + 0.4),
                (30.0, y0 + 1.0),
                (29.5, y0 + 1.0),
            ],
        )
    };
    write("track_bent_on", vec![ell(29.8)]);
    write("track_bent_off", vec![ell(30.0)]);
    // At negative y: a centre of -0.7 is on a track, -0.2 is not: T.y 1 in all.
    write(
        "track_negative",
        vec![
            rect(via, 29.5, -1.2, 30.5, -0.2),
            rect(via, 34.5, -0.7, 35.5, 0.3),
        ],
    );
}
