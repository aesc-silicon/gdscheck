// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`min_space`](gdscheck::checks::space): the bound met exactly and missed
//! by five nanometres, on each way two shapes can face - across an axis-aligned gap,
//! corner to corner, along 45° walls - and each gate once, met and not met.
//!
//! The comparison is exact on the grid, so a gap of exactly the value is clean and one
//! five nanometres under it is not.  An axis-aligned gap is a coordinate difference and
//! a corner-to-corner one a 3-4-5 triangle, both on the grid; a gap between 45° walls
//! is `k/√2` for the walls' offset `k` in `x - y`, never on the grid, so the two
//! patterns straddle the value by the two nearest offsets, 0.708 and 0.706 µm.  Every
//! expected count is read off the drawing: a violation is one pair.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/space";

/// A 45° bar from `(x, y)` rising to the right for `len` in each axis, its walls on
/// the lines `x - y = x - y` of the start and `w2` further: `w2 / √2` across.
pub fn bar45(l: (i16, i16), x: f64, y: f64, len: f64, w2: f64) -> gds21::GdsElement {
    let h = w2 * 0.5;
    poly(
        l,
        &[
            (x, y),
            (x + len, y + len),
            (x + len + h, y + len - h),
            (x + h, y - h),
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

    // Two 1 µm squares side by side, 0.5 µm apart and 0.495.  S.min: 0 and 1.
    for (name, x) in [("axis_exact", 31.5), ("axis_under", 31.495)] {
        write(
            name,
            vec![
                rect(outer, 30.0, 30.0, 31.0, 31.0),
                rect(outer, x, 30.0, x + 1.0, 31.0),
            ],
        );
    }

    // Two squares corner to corner, 0.3 across and 0.4 up: 0.5 as the crow flies.  Then
    // 0.297 and 0.396: 0.495.  S.min: 0 and 1.
    for (name, dx, dy) in [("corner_exact", 0.3, 0.4), ("corner_under", 0.297, 0.396)] {
        write(
            name,
            vec![
                rect(outer, 30.0, 30.0, 31.0, 31.0),
                rect(outer, 31.0 + dx, 31.0 + dy, 32.0 + dx, 32.0 + dy),
            ],
        );
    }

    // Two 45° bars side by side, 3 µm along each axis, walls 1.2 apart in `x - y`.
    // The second starts `s` right and `s` down of the first, so its near wall is
    // `2s - 1.2` beyond the first's far wall: 0.708 for 0.5006 µm across, 0.706 for
    // 0.4992.  S.min: 0 and 1.  Both gaps are between 45° walls under S.bent's 0.8, so
    // S.bent: 1 and 1.
    for (name, s) in [("diag_exact", 0.954), ("diag_under", 0.953)] {
        write(
            name,
            vec![
                bar45(outer, 30.0, 30.0, 3.0, 1.2),
                bar45(outer, 30.0 + s, 30.0 - s, 3.0, 1.2),
            ],
        );
    }

    // The same pair 0.85 apart in `x - y`, 0.601 µm across: over S.min's 0.5, under
    // S.bent's 0.8.  S.bent: 1, S.min: 0.
    write(
        "bent_pair",
        vec![
            bar45(outer, 30.0, 30.0, 3.0, 1.2),
            bar45(outer, 31.025, 28.975, 3.0, 1.2),
        ],
    );

    // A 20 µm bar with its far corner chamfered at 45°, and a 5 µm bar 0.6 above its
    // near end: the gap is between axis-aligned walls, and the only bent wall is 15 µm
    // from it.  S.bent: 0, S.min: 0.
    write(
        "bent_elsewhere",
        vec![
            poly(
                outer,
                &[
                    (30.0, 30.0),
                    (50.0, 30.0),
                    (50.0, 30.5),
                    (49.5, 31.0),
                    (30.0, 31.0),
                ],
            ),
            rect(outer, 30.0, 31.6, 35.0, 32.6),
        ],
    );

    // Two 0.2 µm bars of Inner 0.55 apart running alongside for exactly S.len's 2 µm,
    // and for 2.005.  S.len: 0 and 1; S.wide: 0, the lines being under 0.3.
    for (name, run) in [("run_exact", 2.0), ("run_over", 2.005)] {
        write(
            name,
            vec![
                rect(inner, 30.0, 30.0, 30.0 + run, 30.2),
                rect(inner, 30.0, 30.75, 30.0 + run, 30.95),
            ],
        );
    }

    // Two bars of Inner 0.55 apart running alongside for 1 µm, exactly S.wide's 0.3
    // deep, and one of them 0.305.  S.wide: 0 and 1; S.len: 0, the run being under 2.
    for (name, w) in [("wide_exact", 0.3), ("wide_over", 0.305)] {
        write(
            name,
            vec![
                rect(inner, 30.0, 30.0, 31.0, 30.0 + w),
                rect(inner, 30.0, 30.55 + w, 31.0, 30.85 + w),
            ],
        );
    }

    // Two bars of Via 0.55 apart: 0.5 deep along 3 µm, wide and long; 0.5 deep along
    // 1.5; 0.2 deep along 3.  S.both: 1, 0, 0.
    for (name, w, run) in [
        ("both_wide_long", 0.5, 3.0),
        ("both_wide_short", 0.5, 1.5),
        ("both_narrow_long", 0.2, 3.0),
    ] {
        write(
            name,
            vec![
                rect(via, 30.0, 30.0, 30.0 + run, 30.0 + w),
                rect(via, 30.0, 30.55 + w, 30.0 + run, 30.55 + 2.0 * w),
            ],
        );
    }

    // Two squares of Outer 0.25 apart, under S.same's 0.3 and S.diff's 0.5 alike.
    // Bridged by a Via on each and a bar of Inner over both they are one net: S.same
    // 1, S.diff 0.  Apart, S.same 0 and S.diff 1.  S.min: 1 either way.
    let squares = || {
        vec![
            rect(outer, 30.0, 30.0, 31.0, 31.0),
            rect(outer, 31.25, 30.0, 32.25, 31.0),
        ]
    };
    let mut v = squares();
    v.push(rect(via, 30.3, 30.3, 30.7, 30.7));
    v.push(rect(via, 31.55, 30.3, 31.95, 30.7));
    v.push(rect(inner, 30.2, 30.2, 32.05, 30.8));
    write("net_bridged", v);
    write("net_apart", squares());
}
