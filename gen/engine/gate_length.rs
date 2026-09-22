// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the gate rules: the width of Outer between the walls it shares with the
//! boundary of Inner, or between the ones it does not, driven from a deck so that the
//! parameters arrive the way a PDK writes them.
//!
//! The tile is 20 µm with lines at its multiples.  Every expected count is read off the
//! drawing: a gate is both walls of one stretch, cut to it.

use crate::helpers::{flat_array, layer, library, poly, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/gate_length";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A stripe 0.8 µm tall from x = 30 to 70 and a 2 µm piece of Inner cut out of it at
    // 58..60 - the way a channel mask is the poly over the active - across the tile
    // line at 40 from the stripe's midpoint.  The stripe's walls are shared with the
    // piece's boundary there, and 0.8 is under G.min's 1 µm.  G.min: 2, G.len: 0 (the
    // shared run is 2 µm, not more than 3), G.outside: 2 (no Via to be outside of), and
    // G.max_unshared: 2 - the stripe's ends are unshared walls 40 µm apart.
    write(
        "stripe_far",
        vec![
            rect(outer, 30.0, 30.0, 70.0, 30.8),
            rect(inner, 58.0, 30.0, 60.0, 30.8),
        ],
    );

    // The same stripe with a 4 µm piece: a run over G.len's 3 µm.  G.min: 2, G.len: 2,
    // and G.outside and G.max_unshared as above.
    write(
        "stripe_long_run",
        vec![
            rect(outer, 30.0, 30.0, 70.0, 30.8),
            rect(inner, 56.0, 30.0, 60.0, 30.8),
        ],
    );

    // A body 4 µm by 1 µm whose two ends Inner cuts: the shared walls are the ends, 4
    // apart, the unshared the top and bottom, 1 apart.  G.max_shared: 2 (4 > 3),
    // G.max_unshared: 0 (1 is not over 3), G.min: 0 (4 is not under 1).
    write(
        "ends_on_reference",
        vec![
            rect(outer, 30.0, 30.0, 34.0, 31.0),
            rect(inner, 25.0, 29.0, 30.0, 32.0),
            rect(inner, 34.0, 29.0, 39.0, 32.0),
        ],
    );

    // The same body 3.5 µm tall: now the unshared pair is over 3 as well.
    // G.max_shared: 2, G.max_unshared: 2.
    write(
        "ends_on_reference_tall",
        vec![
            rect(outer, 30.0, 30.0, 34.0, 33.5),
            rect(inner, 25.0, 29.0, 30.0, 34.5),
            rect(inner, 34.0, 29.0, 39.0, 34.5),
        ],
    );

    // A 10 µm stripe with the piece at 34..36 and Via over the right half of the piece
    // from 35 on: G.outside keeps the stretch 34..35 only, G.min all of it.  G.outside:
    // 2, G.min: 2, G.max_unshared: 2 (the stripe's ends, 10 apart).  With Via over the
    // whole piece, G.outside: 0, G.min: 2.
    write(
        "outside_half",
        vec![
            rect(outer, 30.0, 30.0, 40.0, 30.8),
            rect(inner, 34.0, 30.0, 36.0, 30.8),
            rect(via, 35.0, 29.0, 50.0, 32.0),
        ],
    );
    write(
        "outside_all",
        vec![
            rect(outer, 30.0, 30.0, 40.0, 30.8),
            rect(inner, 34.0, 30.0, 36.0, 30.8),
            rect(via, 33.0, 29.0, 50.0, 32.0),
        ],
    );

    // --- The hardening patterns: what a rule manual's gate length asks of any layer,
    // drawn once here for every deck of every PDK (hardening/SPEC.md).  A gate is a
    // stripe of Outer and a piece of Inner cut out of it, the piece's walls across the
    // stripe lying on the stripe's own; a violation is both walls of one gate, two
    // markers.  Every stripe is 10 µm long with a 2 µm piece at its middle, so G.len
    // (a run over 3) is quiet and G.max_unshared (the ends, 10 apart) fires twice on
    // each, which the cases do not read.

    // A stripe `h` tall from (x, y), `len` long, with a piece `run` long at its middle.
    let gate = |x: f64, y: f64, len: f64, h: f64, run: f64| {
        vec![
            rect(outer, x, y, x + len, y + h),
            rect(
                inner,
                x + (len - run) * 0.5,
                y,
                x + (len + run) * 0.5,
                y + h,
            ),
        ]
    };
    // The same standing up.
    let gate_v = |x: f64, y: f64, len: f64, w: f64, run: f64| {
        vec![
            rect(outer, x, y, x + w, y + len),
            rect(
                inner,
                x,
                y + (len - run) * 0.5,
                x + w,
                y + (len + run) * 0.5,
            ),
        ]
    };

    // The bound.  Gates 1.0 tall and wide are not under G.min, 0.995 tall and wide are;
    // 3.0 tall is not over G.max_shared, 3.005 is; a 0.995 gate 3.0 long is no run over
    // G.len's 3, one 3.005 long is (and both are G.min's).  G.min: 8; G.max_shared: 2;
    // G.len: 2.
    let mut e = vec![];
    e.extend(gate(2.0, 2.0, 10.0, 1.0, 2.0)); // clean
    e.extend(gate_v(2.0, 4.0, 10.0, 1.0, 2.0)); // clean
    e.extend(gate(2.0, 16.0, 10.0, 0.995, 2.0)); // G.min
    e.extend(gate_v(14.0, 2.0, 10.0, 0.995, 2.0)); // G.min
    e.extend(gate(2.0, 18.0, 10.0, 3.0, 2.0)); // clean
    e.extend(gate(2.0, 23.0, 10.0, 3.005, 2.0)); // G.max_shared
    e.extend(gate(2.0, 28.0, 10.0, 0.995, 3.0)); // G.min, not G.len
    e.extend(gate(2.0, 30.0, 10.0, 0.995, 3.005)); // G.min, G.len
    write("bound", e);

    // 45° geometry.  A 45° stripe with a piece cut from it whose walls across lie on
    // the stripe's: 0.99 across (d = 0.7) is under G.min, 1.004 (d = 0.71) is not.
    // G.min: 2.
    let gate45 = |x: f64, y: f64, d: f64| {
        let len = 8.0;
        vec![
            poly(
                outer,
                &[
                    (x, y),
                    (x + len, y + len),
                    (x + len - d, y + len + d),
                    (x - d, y + d),
                ],
            ),
            poly(
                inner,
                &[
                    (x + 3.0, y + 3.0),
                    (x + 5.0, y + 5.0),
                    (x + 5.0 - d, y + 5.0 + d),
                    (x + 3.0 - d, y + 3.0 + d),
                ],
            ),
        ]
    };
    let mut e = gate45(2.0, 2.0, 0.7); // G.min
    e.extend(gate45(14.0, 2.0, 0.71)); // clean
    write("bound_45", e);

    // Shapes that merge.  A piece drawn as two abutting halves is one gate, two
    // markers, not four; a stripe drawn as two overlapping boxes with one piece; a
    // stripe with two pieces 1 apart is two gates.  G.min: 8.
    write(
        "merge",
        vec![
            rect(outer, 2.0, 2.0, 12.0, 2.995),
            rect(inner, 6.0, 2.0, 7.0, 2.995),
            rect(inner, 7.0, 2.0, 8.0, 2.995), // one gate
            rect(outer, 2.0, 6.0, 8.0, 6.995),
            rect(outer, 6.0, 6.0, 12.0, 6.995),
            rect(inner, 6.0, 6.0, 8.0, 6.995), // one gate
            rect(outer, 2.0, 10.0, 12.0, 10.995),
            rect(inner, 5.0, 10.0, 6.0, 10.995),
            rect(inner, 7.0, 10.0, 8.0, 10.995), // two gates
        ],
    );

    // Tile lines.  0.995 gates whose piece is across x = 20, ends on 20, starts on 20,
    // is across 21, 40 and 42, well inside a tile at 10; a standing gate with its piece
    // across y = 20; one at (1000, 1000).  G.min: 18.
    let mut e = vec![];
    for (i, px) in [9.0, 19.0, 18.0, 20.0, 20.0, 39.0, 41.0].iter().enumerate() {
        let y = 2.0 + 2.0 * i as f64;
        e.extend(gate(px - 4.0, y, 10.0, 0.995, 2.0)); // piece from px to px + 2
    }
    e.extend(gate_v(30.0, 15.0, 10.0, 0.995, 2.0)); // piece 19..21 across y = 20
    e.extend(gate(996.0, 1000.0, 10.0, 0.995, 2.0)); // piece 1000..1002
    write("tile_lines", e);

    // Fifty 0.995 gates, flat and as an array reference.  G.min: 100 each.
    let cell = gate(0.2, 0.2, 10.0, 0.995, 2.0);
    write("array_flat", flat_array(&cell, 5, 10, 12.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 5, 10, 12.0),
    );

    // Small and long.  A 0.005 gate; a 300 µm stripe with a 0.995 gate at its middle,
    // its ends 300 apart.  G.min: 4.
    let mut e = gate(2.0, 2.0, 10.0, 0.005, 2.0);
    e.extend(gate(2.0, 6.0, 300.0, 0.995, 2.0));
    write("extremes", e);

    // Exactly so (G.exact, 1.0).  Gates 1.0 tall and wide are right; 0.995 and 1.005
    // tall are off; a 1.0 gate with its piece across x = 20 is right, a 1.005 one is
    // off; fifty 1.005 gates flat and as an array reference.  G.exact: 4, 2, 100, 100.
    let mut e = gate(2.0, 2.0, 10.0, 1.0, 2.0);
    e.extend(gate_v(2.0, 4.0, 10.0, 1.0, 2.0));
    e.extend(gate(2.0, 16.0, 10.0, 0.995, 2.0)); // G.exact
    e.extend(gate(2.0, 18.0, 10.0, 1.005, 2.0)); // G.exact
    write("exact_bound", e);
    let mut e = gate(15.0, 2.0, 10.0, 1.0, 2.0);
    e.extend(gate(15.0, 4.0, 10.0, 1.005, 2.0)); // G.exact
    write("exact_tile_lines", e);
    let cell = gate(0.2, 0.2, 10.0, 1.005, 2.0);
    write("exact_array_flat", flat_array(&cell, 5, 10, 12.0));
    write_gz(
        &format!("{DIR}/exact_array_ref.gds.gz"),
        ref_array(cell, 5, 10, 12.0),
    );
}
