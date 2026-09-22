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

use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
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

    // --- The hardening patterns: what a rule manual's space asks of any layer, drawn
    // once here for every deck of every PDK (hardening/SPEC.md, "Where a pattern
    // belongs").  S.min is 0.5; a violation is one pair.

    // The bound and both metrics.  Gap 0.495 fires; a diagonal offset of 0.355/0.355 is
    // 0.502 corner to corner (clean), 0.35/0.35 is 0.495 (fires); an x-gap of 0.495
    // between boxes that meet corner-on in projection fires; 0.5 is clean.  S.min: 3.
    write(
        "bound",
        vec![
            rect(outer, 2.0, 2.0, 3.0, 3.0),
            rect(outer, 3.495, 2.0, 4.495, 3.0), // S.min
            rect(outer, 6.0, 2.0, 7.0, 3.0),
            rect(outer, 7.355, 3.355, 8.355, 4.355), // clean (0.502)
            rect(outer, 10.0, 2.0, 11.0, 3.0),
            rect(outer, 11.35, 3.35, 12.35, 4.35), // S.min (0.495)
            rect(outer, 2.0, 6.0, 3.0, 7.0),
            rect(outer, 3.495, 7.0, 4.495, 8.0), // S.min (corner-on 0.495)
            rect(outer, 6.0, 6.0, 7.0, 7.0),
            rect(outer, 7.5, 6.5, 8.5, 7.5), // clean
        ],
    );

    // 45° geometry at 0.495: a diamond tip above a wall, two parallel 45° strips, a box
    // corner facing a chamfer, tip to tip.  The strips: d = 0.6 → 0.849 wide; a second
    // strip dy higher is (dy − 1.2)/√2 away, dy = 1.9 → 0.495.  The chamfer along
    // x + y = 16.5 is 0.495 from the box corner (12.85, 4.0), whose foot (12.6, 3.75)
    // lies on the chamfer.  S.min: 4; the strip pair is under S.bent's 0.8 too (1).
    write(
        "bound_45",
        vec![
            rect(outer, 2.0, 2.0, 5.0, 3.0),
            diamond(outer, 3.5, 3.995, 0.5), // S.min: tip 0.495 above the wall
            strip45(outer, 7.0, 2.0, 2.0, 0.6),
            strip45(outer, 7.0, 3.9, 2.0, 0.6), // S.min: strips 0.495 apart (S.bent)
            chamfered_tr(outer, 11.0, 2.0, 13.0, 4.0, 16.5),
            rect(outer, 12.85, 4.0, 14.5, 5.5), // S.min: corner 0.495 from the chamfer
            diamond(outer, 17.0, 3.0, 0.5),
            diamond(outer, 18.495, 3.0, 0.5), // S.min: tips 0.495 apart
        ],
    );

    // Unions.  Overlapping, abutting and gridded boxes each 0.495 from a third box: one
    // pair each, the gap read against the merged shape; an island 0.495 from a ring's
    // inner wall.  S.min: 4.
    let mut e = vec![
        rect(outer, 2.0, 2.0, 2.6, 3.0),
        rect(outer, 2.4, 2.0, 3.0, 3.0),
        rect(outer, 3.495, 2.0, 4.5, 3.0), // S.min
        rect(outer, 6.0, 2.0, 6.5, 3.0),
        rect(outer, 6.5, 2.0, 7.0, 3.0),
        rect(outer, 7.495, 2.0, 8.5, 3.0), // S.min
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (10.0 + 0.2 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(outer, x, y, x + 0.2, y + 0.2));
        }
    }
    e.push(rect(outer, 11.495, 2.0, 12.5, 3.0)); // S.min
    e.extend([
        rect(outer, 14.0, 6.0, 18.0, 7.0),
        rect(outer, 14.0, 9.0, 18.0, 10.0),
        rect(outer, 14.0, 7.0, 15.0, 9.0),
        rect(outer, 17.0, 7.0, 18.0, 9.0),
    ]);
    e.push(rect(outer, 15.5, 7.495, 16.5, 8.495)); // S.min: island 0.495 above the wall
    write("merge", e);

    // Tile lines.  0.495 gaps ending on x = 20, straddling 20, starting on 20, straddling
    // 21, on 40, straddling 42, inside a tile at 10; two in y running across 20/21 and
    // 40/42; a corner-to-corner pair (0.495) across (20, 20).  S.min: 10.
    let pair = |x: f64, y: f64| {
        vec![
            rect(outer, x - 1.0, y, x, y + 1.0),
            rect(outer, x + 0.495, y, x + 1.495, y + 1.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(9.505, 2.0));
    e.extend(pair(19.505, 2.0));
    e.extend(pair(19.75, 4.0));
    e.extend(pair(20.0, 6.0));
    e.extend(pair(20.75, 8.0));
    e.extend(pair(39.505, 2.0));
    e.extend(pair(41.75, 4.0));
    e.push(rect(outer, 15.0, 12.0, 25.0, 13.0));
    e.push(rect(outer, 15.0, 13.495, 25.0, 14.0));
    e.push(rect(outer, 35.0, 12.0, 45.0, 13.0));
    e.push(rect(outer, 35.0, 13.495, 45.0, 14.0));
    e.push(rect(outer, 19.0, 19.0, 20.0, 20.0));
    e.push(rect(outer, 20.35, 20.35, 21.0, 21.0));
    write("tile_lines", e);

    // Fifty 0.495 pairs, flat and as an array reference.  S.min: 50 each.
    let cell = vec![
        rect(outer, 0.2, 0.2, 0.7, 0.7),
        rect(outer, 1.195, 0.2, 1.695, 0.7),
    ];
    write("array_flat", flat_array(&cell, 10, 5, 2.5));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.5),
    );

    // Small, long, far.  A 0.005 sliver 0.495 from a box, two 300 µm bars 0.495 apart
    // (one pair), a pair at (1000, 1000).  S.min: 3.
    write(
        "extremes",
        vec![
            rect(outer, 2.0, 2.0, 3.0, 3.0),
            rect(outer, 3.495, 2.0, 3.5, 3.0),
            rect(outer, 2.0, 6.0, 302.0, 7.0),
            rect(outer, 2.0, 7.495, 302.0, 8.0),
            rect(outer, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(outer, 1001.495, 1000.0, 1002.0, 1001.0),
        ],
    );
}
