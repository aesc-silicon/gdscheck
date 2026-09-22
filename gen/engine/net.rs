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

use crate::helpers::{flat_array, layer, library, poly, rect, ref_array, write_gz};
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

    // --- The hardening patterns: what a rule manual's antenna ratio asks of any
    // layer, drawn once here for every deck of every PDK (hardening/SPEC.md).  The
    // gate is 1 µm², so the plate's area in µm² is the ratio, and a violation is one
    // gate on one net.

    // 45° geometry.  A 5 × 2 plate with a 0.1 × 0.1 triangle on one corner is 10.005,
    // over; with a triangle cut off a corner, 9.995.  N.ant: 1.
    let antenna45 = |x: f64, y: f64, over: bool| {
        let plate = if over {
            poly(
                outer,
                &[
                    (x, y),
                    (x + 5.1, y),
                    (x + 5.0, y + 0.1),
                    (x + 5.0, y + 2.0),
                    (x, y + 2.0),
                ],
            )
        } else {
            poly(
                outer,
                &[
                    (x, y),
                    (x + 5.0, y),
                    (x + 5.0, y + 1.9),
                    (x + 4.9, y + 2.0),
                    (x, y + 2.0),
                ],
            )
        };
        vec![
            rect(inner, x, y, x + 1.0, y + 1.0),
            rect(via, x + 0.4, y + 0.4, x + 0.6, y + 0.6),
            plate,
        ]
    };
    let mut e = antenna45(2.0, 2.0, true);
    e.extend(antenna45(12.0, 2.0, false));
    write("bound_45", e);

    // Shapes that merge.  A plate drawn as two 3 × 2 boxes overlapping by 1 is 10 as a
    // union and 12 as a sum: not over.  A gate drawn as two abutting halves under a
    // 10.005 plate is one gate, one violation.  N.ant: 1.
    write(
        "merge",
        vec![
            rect(inner, 2.0, 2.0, 3.0, 3.0),
            rect(via, 2.4, 2.4, 2.6, 2.6),
            rect(outer, 2.0, 2.0, 5.0, 4.0),
            rect(outer, 4.0, 2.0, 7.0, 4.0), // union 10: clean
            rect(inner, 12.0, 2.0, 12.5, 3.0),
            rect(inner, 12.5, 2.0, 13.0, 3.0),
            rect(via, 12.4, 2.4, 12.6, 2.6),
            rect(outer, 12.0, 2.0, 17.0025, 4.0), // 10.005: N.ant, once
        ],
    );

    // Tile lines.  A 10.005 plate across x = 20 with its gate left of it, one whose
    // gate is across the line, one across y = 20, a 10 plate across 40 (clean), a
    // 10.005 plate ending on 42, one at (1000, 1000).  N.ant: 5.
    let antenna_at = |x: f64, y: f64, w: f64| {
        vec![
            rect(inner, x, y, x + 1.0, y + 1.0),
            rect(via, x + 0.4, y + 0.4, x + 0.6, y + 0.6),
            rect(outer, x, y, x + w, y + 2.0),
        ]
    };
    let mut e = antenna_at(17.0, 2.0, 5.0025); // plate 17..22.0025 across 20
    e.extend(antenna_at(19.5, 6.0, 5.0025)); // gate 19.5..20.5 across 20
    e.extend(antenna_at(2.0, 19.0, 5.0025)); // plate 19..21 across y = 20
    e.extend(antenna_at(37.0, 2.0, 5.0)); // 10 across 40: clean
    e.extend(antenna_at(36.9975, 6.0, 5.0025)); // plate ending on 42
    e.extend(antenna_at(1000.0, 1000.0, 5.0025));
    write("tile_lines", e);

    // Fifty antennas at 10.005, flat and as an array reference.  N.ant: 50 each.
    let cell = antenna_at(0.2, 0.2, 5.0025);
    write("array_flat", flat_array(&cell, 10, 5, 8.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 8.0),
    );

    // Small and long.  A 300 × 2 plate over fifteen tiles is 600, over; a 0.005 sliver
    // of a plate is 0.01.  N.ant: 1.
    write(
        "extremes",
        vec![
            rect(inner, 2.0, 2.0, 3.0, 3.0),
            rect(via, 2.4, 2.4, 2.6, 2.6),
            rect(outer, 2.0, 2.0, 302.0, 4.0), // N.ant
            rect(inner, 2.0, 6.0, 3.0, 7.0),
            rect(via, 2.4, 6.4, 2.6, 6.6),
            rect(outer, 2.0, 6.4, 2.005, 8.4), // clean
        ],
    );

    // Nets under a marker, across the tile lines: an Outer marker across x = 20 over
    // two Inner squares, one each side, tied through Vias is one net; untied, two.
    // Fifty untied markers flat and as an array reference.  N.nets: 0, 1, 50, 50.
    let marker = |x: f64, y: f64, tied: bool| {
        let mut v = vec![
            rect(outer, x, y, x + 6.0, y + 2.0),
            rect(inner, x + 1.0, y + 0.5, x + 2.0, y + 1.5),
            rect(inner, x + 4.0, y + 0.5, x + 5.0, y + 1.5),
        ];
        if tied {
            v.push(rect(via, x + 1.4, y + 0.9, x + 1.6, y + 1.1));
            v.push(rect(via, x + 4.4, y + 0.9, x + 4.6, y + 1.1));
        }
        v
    };
    write("nets_tile_line_one", marker(17.0, 30.0, true));
    write("nets_tile_line_two", marker(17.0, 30.0, false));
    let cell = marker(0.2, 0.2, false);
    write("nets_array_flat", flat_array(&cell, 10, 5, 8.0));
    write_gz(
        &format!("{DIR}/nets_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 8.0),
    );
}
