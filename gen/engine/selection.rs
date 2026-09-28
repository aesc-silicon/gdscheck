// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for a selection handed on whole: `inside [Inner - Diode, Outer]` under an
//! enclosure, with Inner built far past the Diode that cuts it.
//!
//! A tile's copy of a boolean is exact out to the thinner of its sources' reaches; past
//! it, an Inner reaching further than the Diode keeps what the Diode covers.  A copy
//! handed on whole carried that into every reader.

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
}
