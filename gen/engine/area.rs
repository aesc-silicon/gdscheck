// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the area family ([`area`](gdscheck::checks::area)): each bound met
//! exactly and missed by five nanometres of one side, on each scope - a region, a hole,
//! a container, the chip - and the connected-net gate met and not met.  A right
//! triangle with legs of odd nanometres has an area of a half square DBU under the
//! bound, which is what the comparison on the half grid is for.

use crate::helpers::{layer, library, poly, rect, write_gz};
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
}
