// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for a selection handed on whole: `inside [Inner - Diode, Outer]` under an
//! enclosure, with Inner built far past the Diode that cuts it.
//!
//! A tile's copy of a boolean is exact out to the thinner of its sources' reaches; past
//! it, an Inner reaching further than the Diode keeps what the Diode covers.  A copy
//! handed on whole carried that into every reader.
//!
//! And `walls`: the edges of Inner cut at Outer's boundary, with one Inner wall lying
//! on each of Outer's four walls.  A wall edge is inside on every side; it used to be
//! inside on Outer's left and bottom walls and outside on its right and top ones.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/selection";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let diode = layer(pdk, "Diode");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    // An Inner bar across the tile line at 20, 3 µm inside an Outer, with a tab at
    // x 30 running out of the Outer's top; a Diode covers the tab from 16 up, so what
    // is left lies whole in the Outer.  The tile at x 0..20 owns part of the bar and
    // copies the whole Inner, tab and all, but not the Diode 10 µm out.
    let bar = |x0: f64| {
        poly(
            inner,
            &[
                (x0, 5.0),
                (35.0, 5.0),
                (35.0, 15.0),
                (30.24, 15.0),
                (30.24, 21.0),
                (30.0, 21.0),
                (30.0, 15.0),
                (x0, 15.0),
            ],
        )
    };
    let tab_cover = rect(diode, 29.9, 16.0, 30.34, 21.2);
    let frame = rect(outer, 2.0, 2.0, 38.0, 19.0);
    // tab: enclosed by 3 µm and more: SEL.enc 0.
    write_gz(
        &format!("{DIR}/tab.gds.gz"),
        library("TOP", vec![frame.clone(), bar(5.0), tab_cover.clone()]),
    );
    // tab_short: the bar starts 0.2 inside the Outer's left wall: SEL.enc 1.
    write_gz(
        &format!("{DIR}/tab_short.gds.gz"),
        library("TOP", vec![frame, bar(2.2), tab_cover]),
    );
    // walls: an Outer square at 5..25, four Inner bars outside it each with one wall on
    // one of its walls, an Inner square well inside it and one well outside.  Every
    // Inner is 5 µm or more from the next, past SEL.reach, and the inner square 8 µm
    // inside the Outer, past SEL.enc.  Of the 24 Inner edges, the four on Outer's walls
    // and the inner square's four are inside - SEL.in 8 - and the bars' other twelve
    // and the outer square's four are outside - SEL.out 16.  Read by which wall they
    // lay on, the four flush edges split two and two: 6 and 18.
    write_gz(
        &format!("{DIR}/walls.gds.gz"),
        library(
            "TOP",
            vec![
                rect(outer, 5.0, 5.0, 25.0, 25.0),
                rect(inner, 3.0, 12.0, 5.0, 18.0), // on the left wall
                rect(inner, 25.0, 12.0, 27.0, 18.0), // on the right wall
                rect(inner, 12.0, 3.0, 18.0, 5.0), // on the bottom wall
                rect(inner, 12.0, 25.0, 18.0, 27.0), // on the top wall
                rect(inner, 13.0, 13.0, 17.0, 17.0), // inside
                rect(inner, 35.0, 12.0, 39.0, 18.0), // outside
            ],
        ),
    );
}
