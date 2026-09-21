// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the net family ([`net`](gdscheck::checks::net)): an antenna ratio met
//! exactly and missed, by plan area and by sidewall, with and without a protection
//! diode, and read at a level below the conductor; and the nets under one marker.
//!
//! The gate is a 1 µm square of Inner tied through a Via to an Outer plate, so the
//! plate's area is the ratio.  The connect graph joins Inner, then Diode, then Outer,
//! so a diode is on the net at the plate's level, as a diffusion diode is at Metal1.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/net";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    let diode = layer(pdk, "Diode");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A gate of Inner 1 µm square at (30, 30), a Via on it, and an Outer plate `w` by
    // `h` starting on the via and running right.
    let antenna = |w: f64, h: f64, with_diode: bool| {
        let mut v = vec![
            rect(inner, 30.0, 30.0, 31.0, 31.0),
            rect(via, 30.4, 30.4, 30.6, 30.6),
            rect(outer, 30.0, 30.0, 30.0 + w, 30.0 + h),
        ];
        if with_diode {
            // A 1 µm² diode on the plate's far end, tied through a Via of its own.
            v.push(rect(diode, 30.0 + w - 1.0, 30.0, 30.0 + w, 31.0));
            v.push(rect(via, 30.0 + w - 0.6, 30.4, 30.0 + w - 0.4, 30.6));
        }
        v
    };

    // A plate of exactly 10 µm² is a ratio of 10, which meets N.ant's limit; one of
    // 10.005 is over it, one of 9.995 under.  N.ant: 0, 1 and 0.  Read through Inner's
    // step alone, the plate is not on the gate's net yet: N.level: 0.  N.bare: 0, 1 and
    // 0, no diode about.
    write("ratio_exact", antenna(5.0, 2.0, false));
    write("ratio_over", antenna(5.0025, 2.0, false));
    write("ratio_under", antenna(4.9975, 2.0, false));

    // The same 10 µm² plate with a diode on the net: N.bare leaves it alone and
    // N.protected's 100 is far off, 0 and 0; a 100.005 µm² plate with a diode is over
    // it: N.protected 1, N.bare 0.  Without the diode the 100 µm² plate is N.bare's: 1.
    write("diode_small", antenna(5.0, 2.0, true));
    write("diode_big", antenna(20.001, 5.0, true));
    write("bare_big", antenna(20.0, 5.0, false));

    // By sidewall at 0.5 µm thickness: a 4 by 6 plate has a perimeter of 20, a sidewall
    // of 10, exactly N.side's limit; 4 by 6.005 is over, 4 by 5.995 under.  N.side: 0,
    // 1 and 0.
    write("side_exact", antenna(6.0, 4.0, false));
    write("side_over", antenna(6.005, 4.0, false));
    write("side_under", antenna(5.995, 4.0, false));

    // An Outer marker over two Inner squares.  Tied to the marker through Vias, both
    // are one net: N.nets 0.  Untied, two: N.nets 1.
    let under = |tied: bool| {
        let mut v = vec![
            rect(outer, 30.0, 30.0, 36.0, 32.0),
            rect(inner, 31.0, 30.5, 32.0, 31.5),
            rect(inner, 34.0, 30.5, 35.0, 31.5),
        ];
        if tied {
            v.push(rect(via, 31.4, 30.9, 31.6, 31.1));
            v.push(rect(via, 34.4, 30.9, 34.6, 31.1));
        }
        v
    };
    write("nets_one", under(true));
    write("nets_two", under(false));
}
