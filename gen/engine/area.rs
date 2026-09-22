// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the area family ([`area`](gdscheck::checks::area)): each bound met
//! exactly and missed by five nanometres of one side, on each scope - a region, a hole,
//! a container, the chip - and the connected-net gate met and not met.  A right
//! triangle with legs of odd nanometres has an area of a half square DBU under the
//! bound, which is what the comparison on the half grid is for.

use crate::helpers::{diamond, flat_array, layer, library, poly, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/area";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A 1 by 0.5 µm rectangle is exactly A.min's 0.5 µm²; 1 by 0.495 is under it.
    // A.min: 0 and 1.  A 2 by 2 is exactly A.max's 4; 2 by 2.005 is over.  A.max: 0 and 1.
    write("region_exact", vec![rect(outer, 30.0, 30.0, 31.0, 30.5)]);
    write("region_under", vec![rect(outer, 30.0, 30.0, 31.0, 30.495)]);
    write(
        "region_max_exact",
        vec![rect(outer, 30.0, 30.0, 32.0, 32.0)],
    );
    write(
        "region_max_over",
        vec![rect(outer, 30.0, 30.0, 32.0, 32.005)],
    );

    // A right triangle with legs of 1 µm has 0.5 µm² exactly; with legs of 0.999 and
    // 1.001 it has 499999.5 DBU², half a square DBU under.  A.min: 0 and 1.
    write(
        "tri_exact",
        vec![poly(outer, &[(30.0, 30.0), (31.0, 30.0), (30.0, 31.0)])],
    );
    write(
        "tri_half",
        vec![poly(outer, &[(30.0, 30.0), (30.999, 30.0), (30.0, 31.001)])],
    );

    // A via 0.3 square is exactly A.exact's 0.09 µm²; 0.3 by 0.305 is not.  A.exact:
    // 0 and 1.
    write("exact_ok", vec![rect(via, 30.0, 30.0, 30.3, 30.3)]);
    write("exact_off", vec![rect(via, 30.0, 30.0, 30.3, 30.305)]);

    // A ring with a hole 1 by 0.5, exactly A.hole's 0.5 µm², then 1 by 0.495; a ring
    // with a hole 2 by 2, exactly A.hole_max's 4, then 2 by 2.005.  A.hole: 0 and 1;
    // A.hole_max: 0 and 1.
    let ring = |w: f64, h: f64| {
        vec![
            rect(outer, 30.0, 30.0, 33.0 + w, 31.0),
            rect(outer, 30.0, 31.0 + h, 33.0 + w, 32.0 + h),
            rect(outer, 30.0, 31.0, 31.0, 31.0 + h),
            rect(outer, 31.0 + w, 31.0, 33.0 + w, 31.0 + h),
        ]
    };
    write("hole_exact", ring(1.0, 0.5));
    write("hole_under", ring(1.0, 0.495));
    write("hole_max_exact", ring(2.0, 2.0));
    write("hole_max_over", ring(2.0, 2.005));

    // A 5 µm container of Outer with two 1 µm squares of Inner inside, exactly A.cont's
    // 2 µm² together, then one of them 1 by 1.005.  A.cont: 0 and 1.
    for (name, h) in [("contained_exact", 1.0), ("contained_over", 1.005)] {
        write(
            name,
            vec![
                rect(outer, 30.0, 30.0, 35.0, 35.0),
                rect(inner, 31.0, 31.0, 32.0, 32.0),
                rect(inner, 33.0, 31.0, 34.0, 31.0 + h),
            ],
        );
    }

    // Two 2 by 2.5 rectangles of Outer far apart, exactly A.chip's 10 µm² together,
    // then one of them 2 by 2.505.  A.chip: 0 and 1.
    for (name, h) in [("chip_exact", 2.5), ("chip_over", 2.505)] {
        write(
            name,
            vec![
                rect(outer, 30.0, 30.0, 32.0, 32.5),
                rect(outer, 50.0, 50.0, 52.0, 50.0 + h),
            ],
        );
    }

    // A 0.3 by 1 µm region of Inner, under A.net's 0.5 µm², tied to Outer through a
    // Via: on a net that carries Outer.  A.net: 1.  The same region with the Via left
    // out is on no such net: 0.  A 1 by 1 region tied the same way is big enough: 0.
    let tied = |w: f64, via_on: bool| {
        let mut v = vec![
            rect(inner, 30.0, 30.0, 30.0 + w, 31.0),
            rect(outer, 30.0, 30.4, 32.0, 30.6),
        ];
        if via_on {
            v.push(rect(via, 30.05, 30.45, 30.15, 30.55));
        }
        v
    };
    write("net_small_tied", tied(0.3, true));
    write("net_small_loose", tied(0.3, false));
    write("net_big_tied", tied(1.0, true));

    // --- The hardening patterns: what a rule manual's area asks of any layer, drawn
    // once here for every deck of every PDK (hardening/SPEC.md).  A.min is 0.5 µm²; a
    // violation is one region.

    // The bound and the shapes.  1.0 × 0.5 (0.5) is clean; 1.0 × 0.495 (0.495), a 0.5 ×
    // 0.99 bar, an L of 0.5 arms 0.75 long (0.4375 + 0.0625 = 0.5, clean at 0.75 - the
    // arms 0.745 long give 0.4975), a diamond a = 0.495 (0.4901) and a 1.0 × 0.5 box with
    // 0.1 chamfers (0.48) fire; 0.5 × 1.005 (0.5025) and a diamond a = 0.505 (0.51) are
    // clean.  A.min: 5.
    write(
        "bound",
        vec![
            rect(outer, 2.0, 2.0, 3.0, 2.5),   // clean
            rect(outer, 4.0, 2.0, 5.0, 2.495), // A.min
            rect(outer, 6.0, 2.0, 6.5, 2.99),  // A.min
            rect(outer, 7.0, 2.0, 7.5, 3.005), // clean
            poly(
                outer,
                &[
                    (8.0, 2.0),
                    (8.745, 2.0),
                    (8.745, 2.5),
                    (8.5, 2.5),
                    (8.5, 2.745),
                    (8.0, 2.745),
                ],
            ), // A.min (0.4975)
            diamond(outer, 10.5, 2.5, 0.495),  // A.min (0.4901)
            diamond(outer, 12.0, 2.5, 0.505),  // clean (0.51)
            poly(
                outer,
                &[
                    (13.1, 2.0),
                    (13.9, 2.0),
                    (14.0, 2.1),
                    (14.0, 2.4),
                    (13.9, 2.5),
                    (13.1, 2.5),
                    (13.0, 2.4),
                    (13.0, 2.1),
                ],
            ), // A.min (0.48)
        ],
    );

    // Shapes that merge.  Two overlapping 0.5 × 0.6 boxes (0.3 each) whose union is 0.5
    // × 0.7 (0.35) fire once; two 0.5 × 0.6 boxes meeting at one corner are one region
    // of 0.6 (clean - `touching: separate` reads them apart); two abutting 0.5 × 0.6
    // boxes (0.6) are clean; a 1.0 × 0.5 box drawn as a 5 × 5 grid is clean; an island
    // 0.4 × 0.4 in a ring's hole fires, the ring does not (it is over A.max).  A.min: 2.
    let mut e = vec![
        rect(outer, 2.0, 2.0, 2.5, 2.6),
        rect(outer, 2.0, 2.1, 2.5, 2.7), // A.min, union 0.35
        rect(outer, 3.0, 2.0, 3.5, 2.6),
        rect(outer, 3.5, 2.6, 4.0, 3.2), // clean, one region at the corner
        rect(outer, 5.0, 2.0, 5.5, 2.6),
        rect(outer, 5.5, 2.0, 6.0, 2.6), // clean, 0.6
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (7.0 + 0.2 * i as f64, 2.0 + 0.1 * j as f64);
            e.push(rect(outer, x, y, x + 0.2, y + 0.1)); // clean, 0.5
        }
    }
    e.extend([
        rect(outer, 9.0, 2.0, 12.0, 3.0),
        rect(outer, 9.0, 4.0, 12.0, 5.0),
        rect(outer, 9.0, 3.0, 10.0, 4.0),
        rect(outer, 11.0, 3.0, 12.0, 4.0),
    ]);
    e.push(rect(outer, 10.3, 3.3, 10.7, 3.7)); // A.min, island 0.16
    write("merge", e);

    // Tile lines.  1.0 × 0.495 boxes ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, inside a tile at 10; a 0.5 × 0.99 bar
    // across x = 20; one at (1000, 1000); a 1.0 × 0.5 box straddling 20 is clean.
    // A.min: 9.
    let mut e = vec![];
    for (i, x) in [9.0, 19.0, 19.5, 20.0, 20.5, 39.0, 41.5].iter().enumerate() {
        let y = 2.0 + i as f64;
        e.push(rect(outer, *x, y, x + 1.0, y + 0.495));
    }
    e.push(rect(outer, 19.75, 10.0, 20.25, 10.99));
    e.push(rect(outer, 1000.0, 1000.0, 1001.0, 1000.495));
    e.push(rect(outer, 19.5, 12.0, 20.5, 12.5)); // clean
    write("tile_lines", e);

    // Fifty 0.5 × 0.8 boxes (0.4), flat and as an array reference.  A.min: 50 each.
    let cell = vec![rect(outer, 0.2, 0.2, 0.7, 1.0)];
    write("array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Small and long.  A 0.005 × 2 sliver (0.01) fires; a 0.005 × 100 sliver is 0.5
    // and clean by area; a 300 µm bar is clean.  A.min: 1; the bar is over A.max.
    write(
        "extremes",
        vec![
            rect(outer, 2.0, 2.0, 2.005, 4.0),
            rect(outer, 4.0, 2.0, 4.005, 102.0),
            rect(outer, 2.0, 124.0, 302.0, 124.5),
        ],
    );
}
