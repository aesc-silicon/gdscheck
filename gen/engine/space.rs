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

    // --- The hardening patterns of the bent rule (S.bent: a pair with a 45° wall closer
    // than 0.8; S.min is 0.5, so a gap between 0.5 and 0.8 is S.bent's alone).

    // The bound, in every direction the value can be taken.  Two parallel 45° strips
    // (0.849 wide, d = 0.6) dy higher are (dy − 1.2)/√2 apart: dy = 2.32 → 0.792 fires,
    // dy = 2.34 → 0.806 is clean; a box corner 0.792 from a chamfer (x + y = k, the
    // corner (cx, cy): (k − cx − cy)/√2) fires, 0.806 is clean; a diamond tip 0.795 above
    // a wall fires, 0.8 is clean; a 45° strip's tip 0.795 from a vertical wall fires; a
    // box corner 0.792 from a strip's 45° wall (its foot on the wall) fires, 0.806 is
    // clean.  S.bent: 5.
    write(
        "bent_bound",
        vec![
            strip45(outer, 2.0, 2.0, 2.0, 0.6),
            strip45(outer, 2.0, 4.32, 2.0, 0.6), // S.bent: 0.792
            strip45(outer, 6.0, 2.0, 2.0, 0.6),
            strip45(outer, 6.0, 4.34, 2.0, 0.6), // clean: 0.806
            chamfered_tr(outer, 10.0, 2.0, 12.0, 4.0, 15.0),
            rect(outer, 12.12, 4.0, 13.5, 5.0), // S.bent: corner (12.12, 4.0) 0.792 off x + y = 15
            chamfered_tr(outer, 14.0, 2.0, 16.0, 4.0, 19.0),
            rect(outer, 16.14, 4.0, 17.5, 5.0), // clean: 0.806
            rect(outer, 18.0, 2.0, 21.0, 3.0),
            diamond(outer, 19.5, 4.295, 0.5), // S.bent: tip 0.795 above the wall
            rect(outer, 22.0, 2.0, 25.0, 3.0),
            diamond(outer, 23.5, 4.3, 0.5), // clean: 0.8
            strip45(outer, 27.0, 2.0, 1.0, 0.6),
            rect(outer, 28.795, 2.0, 29.5, 4.0), // S.bent: tip (28, 3) 0.795 from the wall
            strip45(outer, 2.0, 8.0, 2.0, 0.6),
            rect(outer, 1.0, 10.32, 2.0, 11.32), // S.bent: corner (2, 10.32) 0.792 from y = x + 7.2
            strip45(outer, 6.0, 8.0, 2.0, 0.6),
            rect(outer, 5.0, 10.34, 6.0, 11.34), // clean: 0.806
        ],
    );

    // Tile lines and far.  The 0.792 strip pair with its gap straddling x = 20, a pair
    // whose tips end on x = 20, pairs across 40 and 42, one at (1000, 1000).  S.bent: 5.
    let pair = |x: f64, y: f64| {
        vec![
            strip45(outer, x, y, 2.0, 0.6),
            strip45(outer, x, y + 2.32, 2.0, 0.6),
        ]
    };
    let mut e = vec![];
    e.extend(pair(19.0, 2.0));
    e.extend(pair(18.0, 8.0));
    e.extend(pair(39.0, 2.0));
    e.extend(pair(41.0, 8.0));
    e.extend(pair(1000.0, 1000.0));
    write("bent_tile_lines", e);

    // Fifty 0.792 strip pairs, flat and as an array reference.  S.bent: 50 each.
    let cell = vec![
        strip45(outer, 0.6, 0.2, 1.5, 0.6),
        strip45(outer, 0.6, 2.52, 1.5, 0.6),
    ];
    write("bent_array_flat", flat_array(&cell, 10, 5, 5.0));
    write_gz(
        &format!("{DIR}/bent_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 5.0),
    );

    // Long and small.  Two 300 µm 45° strips 0.792 apart fire once; a 0.007 45° sliver
    // 0.792 from a strip fires.  S.bent: 2.
    write(
        "bent_extremes",
        vec![
            strip45(outer, 2.0, 2.0, 212.0, 0.6),
            strip45(outer, 2.0, 4.32, 212.0, 0.6),
            strip45(outer, 2.0, 230.0, 2.0, 0.6),
            strip45(outer, 2.0, 232.32, 2.0, 0.005),
        ],
    );

    // --- The hardening patterns of the gated rule (S.both on Via: 0.6 between lines
    // where one is wider than 0.3 and the two run alongside for more than 2.0).

    // The width bound.  A 0.305 line 3 µm long beside a 0.2 line at 0.55 fires; a 0.30
    // line (not wider than 0.3) beside one at 0.55 is clean; two 0.2 lines at 0.55 are
    // clean; two 0.5 lines at 0.595 fire, at 0.6 they are clean; a 3 × 3 plate with a 0.2
    // line along it at 0.55 fires.  S.both: 3.
    write(
        "gated_bound",
        vec![
            rect(via, 2.0, 2.0, 2.305, 5.0),
            rect(via, 2.855, 2.0, 3.055, 5.0), // S.both
            rect(via, 5.0, 2.0, 5.3, 5.0),
            rect(via, 5.85, 2.0, 6.05, 5.0), // clean
            rect(via, 8.0, 2.0, 8.2, 5.0),
            rect(via, 8.75, 2.0, 8.95, 5.0), // clean
            rect(via, 11.0, 2.0, 11.5, 5.0),
            rect(via, 12.095, 2.0, 12.595, 5.0), // S.both
            rect(via, 14.0, 2.0, 14.5, 5.0),
            rect(via, 15.1, 2.0, 15.6, 5.0), // clean
            rect(via, 17.0, 2.0, 20.0, 5.0),
            rect(via, 20.55, 2.0, 20.75, 5.0), // S.both
        ],
    );

    // The run bound, at 0.55 with a 0.5 line.  Aligned lines 2.0 long are clean, 2.005
    // long fire; a 5 µm pair offset so the facing walls share 2.0 is clean, 2.005 fires;
    // a 1.5 stub beside a 7 µm wide line is clean, and so are two 1.2 stubs 0.5 apart
    // (their stretches are not joined across the gap); a 7 µm 0.2 line broken by a 0.3
    // gap into two 3.35 pieces fires twice.  S.both: 4.
    write(
        "gated_run",
        vec![
            rect(via, 2.0, 2.0, 2.5, 4.0),
            rect(via, 3.05, 2.0, 3.25, 4.0), // clean, run 2.0
            rect(via, 5.0, 2.0, 5.5, 4.005),
            rect(via, 6.05, 2.0, 6.25, 4.005), // S.both, run 2.005
            rect(via, 8.0, 2.0, 8.5, 7.0),
            rect(via, 9.05, 5.0, 9.25, 10.0), // clean, shared 2.0
            rect(via, 11.0, 2.0, 11.5, 7.0),
            rect(via, 12.05, 4.995, 12.25, 10.0), // S.both, shared 2.005
            rect(via, 14.0, 2.0, 14.5, 9.0),
            rect(via, 15.05, 3.0, 15.25, 4.5), // clean, stub 1.5
            rect(via, 17.0, 2.0, 17.5, 9.0),
            rect(via, 18.05, 3.0, 18.25, 4.2),
            rect(via, 18.05, 4.7, 18.25, 5.9), // clean, stubs 1.2
            rect(via, 20.0, 2.0, 20.5, 9.0),
            rect(via, 21.05, 2.0, 21.25, 5.35),
            rect(via, 21.05, 5.65, 21.25, 9.0), // S.both × 2, pieces 3.35
        ],
    );

    // Where the line is wide.  A 0.2 line 6 µm long carrying a 0.5 pad 2.5 long, with a
    // straight 0.2 line 0.55 from the pad: the wide part runs 2.5 beside the neighbour,
    // fires; the same pad 1.5 long is clean (the lines run parallel for 6 µm, the wide
    // part for 1.5), and a pad exactly 2.0 long is clean; an L of 0.5 arms with a 0.2
    // line along the outside of one arm for 2.5 fires.  S.both: 2.
    let padded = |x: f64, plen: f64| {
        poly(
            via,
            &[
                (x, 2.0),
                (x + 0.2, 2.0),
                (x + 0.2, 3.0),
                (x + 0.5, 3.0),
                (x + 0.5, 3.0 + plen),
                (x + 0.2, 3.0 + plen),
                (x + 0.2, 8.0),
                (x, 8.0),
            ],
        )
    };
    write(
        "gated_wide",
        vec![
            padded(2.0, 2.5),
            rect(via, 3.05, 2.0, 3.25, 8.0), // S.both
            padded(5.0, 1.5),
            rect(via, 6.05, 2.0, 6.25, 8.0), // clean
            padded(8.0, 2.0),
            rect(via, 9.05, 2.0, 9.25, 8.0), // clean
            poly(
                via,
                &[
                    (11.0, 2.0),
                    (14.0, 2.0),
                    (14.0, 2.5),
                    (11.5, 2.5),
                    (11.5, 5.0),
                    (11.0, 5.0),
                ],
            ),
            rect(via, 11.0, 1.25, 13.5, 1.45), // S.both: along the arm's outside for 2.5
        ],
    );

    // Both metrics and the ends.  Two 3 × 3 plates corner to corner at 0.4/0.4 (0.566, no
    // parallel run) are clean; a 0.2 line ending 0.55 short of a plate, end-on, is clean;
    // two plates stepped so their facing walls share 2.0 are clean, sharing 2.005 they
    // fire.  S.both: 1.
    write(
        "gated_ends",
        vec![
            rect(via, 2.0, 2.0, 5.0, 5.0),
            rect(via, 5.4, 5.4, 8.4, 8.4), // clean
            rect(via, 10.0, 2.0, 13.0, 5.0),
            rect(via, 11.4, 5.55, 11.6, 8.0), // clean, end-on
            rect(via, 15.0, 2.0, 18.0, 5.0),
            rect(via, 18.55, 3.0, 21.55, 6.0), // clean, shared 2.0
            rect(via, 23.0, 2.0, 26.0, 5.0),
            rect(via, 26.55, 2.995, 29.55, 6.0), // S.both, shared 2.005
        ],
    );

    // Unions.  A wide line drawn as two overlapping 0.2 boxes (0.305) beside a 0.2 line
    // at 0.55 fires; as two abutting slices 0.2 + 0.105 it fires; 0.2 + 0.1 (0.30) is
    // clean; a 0.5 line beside a 0.2 neighbour drawn as two abutting 0.1 halves fires
    // once; beside a neighbour drawn as three collinear 1 µm boxes it fires once.
    // S.both: 4.
    write(
        "gated_merge",
        vec![
            rect(via, 2.0, 2.0, 2.2, 5.0),
            rect(via, 2.105, 2.0, 2.305, 5.0),
            rect(via, 2.855, 2.0, 3.055, 5.0), // S.both
            rect(via, 5.0, 2.0, 5.2, 5.0),
            rect(via, 5.2, 2.0, 5.305, 5.0),
            rect(via, 5.855, 2.0, 6.055, 5.0), // S.both
            rect(via, 8.0, 2.0, 8.2, 5.0),
            rect(via, 8.2, 2.0, 8.3, 5.0),
            rect(via, 8.85, 2.0, 9.05, 5.0), // clean
            rect(via, 11.0, 2.0, 11.5, 5.0),
            rect(via, 12.05, 2.0, 12.15, 5.0),
            rect(via, 12.15, 2.0, 12.25, 5.0), // S.both, once
            rect(via, 14.0, 2.0, 14.5, 5.0),
            rect(via, 15.05, 2.0, 15.25, 3.0),
            rect(via, 15.05, 3.0, 15.25, 4.0),
            rect(via, 15.05, 4.0, 15.25, 5.0), // S.both, once
        ],
    );

    // Tile lines.  The 0.305/0.2 pair at 0.55, 3 µm long, with the gap straddling x = 20,
    // ending on 20, starting on 20, straddling 21, on 40, straddling 42, inside a tile at
    // 10; horizontal pairs running across x = 20 and x = 40; a horizontal pair 2.005 long
    // ending on x = 20; a pair at (1000, 1000).  S.both: 11.
    let pair = |x: f64, y: f64| {
        vec![
            rect(via, x - 0.305, y, x, y + 3.0),
            rect(via, x + 0.55, y, x + 0.75, y + 3.0),
        ]
    };
    let mut e = vec![];
    e.extend(pair(19.7, 2.0));
    e.extend(pair(19.45, 6.0));
    e.extend(pair(20.0, 10.0));
    e.extend(pair(20.7, 14.0));
    e.extend(pair(39.45, 2.0));
    e.extend(pair(41.7, 6.0));
    e.extend(pair(10.0, 2.0));
    e.extend(pair(1000.0, 1000.0));
    e.push(rect(via, 15.0, 18.0, 25.0, 18.305));
    e.push(rect(via, 15.0, 18.855, 25.0, 19.055));
    e.push(rect(via, 35.0, 18.0, 45.0, 18.305));
    e.push(rect(via, 35.0, 18.855, 45.0, 19.055));
    e.push(rect(via, 17.995, 22.0, 20.0, 22.305));
    e.push(rect(via, 17.995, 22.855, 20.0, 23.055));
    write("gated_tile_lines", e);

    // Fifty 0.5/0.2 pairs at 0.55, flat and as an array reference.  S.both: 50 each.
    let cell = vec![
        rect(via, 0.2, 0.2, 0.7, 3.2),
        rect(via, 1.25, 0.2, 1.45, 3.2),
    ];
    write("gated_array_flat", flat_array(&cell, 10, 5, 4.0));
    write_gz(
        &format!("{DIR}/gated_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 4.0),
    );

    // Notches, long and small.  A U of 0.5 arms with a 0.55 slot 3 µm deep is a notch,
    // not a space of lines: clean; a 0.5 × 300 line beside a 0.2 × 300 line at 0.55 fires
    // once; a 0.005 sliver 3 µm long 0.55 from a 0.5 line fires.  S.both: 2.
    write(
        "gated_extremes",
        vec![
            poly(
                via,
                &[
                    (2.0, 2.0),
                    (3.55, 2.0),
                    (3.55, 5.5),
                    (3.05, 5.5),
                    (3.05, 2.5),
                    (2.5, 2.5),
                    (2.5, 5.5),
                    (2.0, 5.5),
                ],
            ), // notch: clean
            rect(via, 2.0, 8.0, 302.0, 8.5),
            rect(via, 2.0, 9.05, 302.0, 9.25), // S.both, 300 µm
            rect(via, 6.0, 2.0, 6.5, 5.0),
            rect(via, 7.05, 2.0, 7.055, 5.0), // S.both, sliver
        ],
    );
}
