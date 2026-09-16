// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the enclosure family ([`enclosure`](gdscheck::checks::enclosure)): a
//! margin measured on three shape families, swept either side of the limit, under both
//! metrics; then the other bound and each `sides` word, met exactly and missed by five
//! nanometres.
//!
//! The families are chosen for where the metrics part company.  Orthogonal walls are the
//! case they must agree on.  A 45° chamfer in the enclosing corner is the case where
//! nothing is parallel, so the projection metric has no facing run to measure and only
//! the euclidian one reaches the wall.  A 45° notch is that same divergence away from a
//! corner, in the middle of a wall.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/min_enclosure";

/// Margins the enclosure families are drawn at, against a 0.5 µm limit: two under it and
/// two over, the inner pair a single nanometre either side so the comparison itself is
/// under test and not just the geometry.
const ORTHO_MARGINS: [f64; 4] = [0.4, 0.499, 0.5, 0.6];

/// The diagonal family cannot use those numbers.  Its margin is a perpendicular distance
/// to a 45° wall, so it is the corner's offset over √2 and no offset on the 1 nm grid
/// lands on 0.5 exactly.  These four corner positions give 0.566, 0.501, 0.499 and 0.354 -
/// the middle pair straddling the limit by less than a nanometre of margin.
const DIAG_CORNERS: [(f64, f64); 4] = [
    (8.100, 0.565_685),
    (8.146, 0.500_631),
    (8.147, 0.499_217),
    (8.250, 0.353_553),
];

fn name(margin: f64) -> String {
    format!("{:04}", (margin * 1000.0).round() as i64)
}

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: String, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A bar 40 µm long across three tiles, enclosed by a row of 2.5 µm pieces the way
    // abutting cells enclose a merged row of their Activ, its far end margin under test.
    // The tile owning the bar's centroid holds a copy of the enclosing layer exact
    // only 1 µm past its core, so the far end lies beyond what that copy shows.
    for m in [0.4, 0.6] {
        let mut v = vec![rect(inner, 5.0, 10.0, 45.0, 11.0)];
        let mut x = 0.0;
        while x < 45.0 + m {
            v.push(rect(outer, x, 9.0, (x + 2.5).min(45.0 + m), 12.0));
            x += 2.5;
        }
        write(format!("bar_{}", name(m)), v);
    }

    // Orthogonal: a square hole in a square, one wall brought in to the margin under
    // test.  Both metrics measure this one, and must agree on it.
    for m in ORTHO_MARGINS {
        write(
            format!("ortho_{}", name(m)),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, m, 1.0, 9.0, 9.0),
            ],
        );
    }

    // Diagonal wall: the enclosing shape's top-right corner is chamfered at 45° and the
    // enclosed square's corner points into it.  Every orthogonal margin is 1.0 or more
    // and no pair of walls here is parallel, so the projection metric has nothing to
    // measure and only the euclidian one sees the chamfer.
    for (t, m) in DIAG_CORNERS {
        write(
            format!("diag_{}", name(m)),
            vec![
                poly(
                    outer,
                    &[
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 7.0),
                        (7.0, 10.0),
                        (0.0, 10.0),
                    ],
                ),
                rect(inner, 1.0, 1.0, t, t),
            ],
        );
    }

    // A corner resting exactly on the enclosing wall measures zero, which is under any
    // limit and is not what the rule is about.  `touch` is that alone; `touch_and_diag`
    // puts a real 0.354 margin on a second chamfer beside it, so the pair says whether
    // dropping the zero also drops the violation next to it.
    let two_chamfers = vec![
        (0.0, 0.0),
        (7.0, 0.0),
        (10.0, 3.0),
        (10.0, 7.0),
        (7.0, 10.0),
        (0.0, 10.0),
    ];
    write(
        "touch".into(),
        vec![poly(outer, &two_chamfers), rect(inner, 1.0, 5.5, 8.5, 8.5)],
    );
    write(
        "touch_and_diag".into(),
        vec![
            poly(outer, &two_chamfers),
            rect(inner, 1.0, 5.5, 8.5, 8.5),
            rect(inner, 1.0, 1.0, 7.5, 3.5),
        ],
    );

    // Diagonal notch: the enclosing wall dips towards the enclosed shape in a 45° V.
    // The wall it faces is still 1.7 away, so again only the euclidian metric reaches
    // the apex - and unlike the chamfer, the violation is in the middle of a wall
    // rather than at a corner.
    for m in ORTHO_MARGINS {
        let ay = 8.3 + m;
        let half = 10.0 - ay;
        write(
            format!("notch_{}", name(m)),
            vec![
                poly(
                    outer,
                    &[
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 10.0),
                        (4.6 + half, 10.0),
                        (4.6, ay),
                        (4.6 - half, 10.0),
                        (0.0, 10.0),
                    ],
                ),
                rect(inner, 1.0, 1.0, 9.0, 8.3),
            ],
        );
    }

    // The maximum: a square with every margin exactly 0.5, and one with its left margin
    // 0.505.  ENC.max: 0 and 1; ENC.proj: 0 and 0.
    for (name, left) in [("max_exact", 0.5), ("max_over", 0.505)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, 0.5, 9.5, 9.5),
            ],
        );
    }

    // The endcap: a shape flush on three sides with its right margin 0.5 and 0.495 -
    // the best side, which is all a minimum on `any` asks about.  ENC.any: 0 and 1.  A
    // maximum on `any` asks that some side be within the value, and a flush side always
    // is: ENC.max_any: 0 and 0.
    for (name, right) in [("any_exact", 9.5), ("any_under", 9.505)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, 0.0, 0.0, right, 10.0),
            ],
        );
    }

    // A maximum on `any` fails only when every side is over: a square with every margin
    // 0.505, and one with three at 0.505 and its left at 0.5.  ENC.max_any: 1 and 0;
    // ENC.max: 1 and 1.
    for (name, left) in [("max_any_over", 0.505), ("max_any_one", 0.5)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, 0.505, 9.495, 9.495),
            ],
        );
    }

    // Bordering sides: a shape 0.05 from the left wall, under ENC.adj's 0.1 trigger,
    // whose bottom margin is 1.0 and 0.495; and one exactly 0.1 from the wall, not
    // short, whose bottom margin is 0.495 and asks nothing.  ENC.adj: 0, 1, 0.
    for (name, left, bottom) in [
        ("adj_ok", 0.05, 1.0),
        ("adj_under", 0.05, 0.495),
        ("adj_trigger", 0.1, 0.495),
    ] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, bottom, 5.0, 9.0),
            ],
        );
    }

    // A line end: a 0.3 µm track 5 long with a 0.2 via inside near its tip, the tip's
    // cap 0.3 and 0.295 from the via; and a track exactly ENC.cap's 0.34 wide, which is
    // not a narrow line.  ENC.cap: 0, 1, 0.
    for (name, w, via_right) in [
        ("cap_exact", 0.3, 4.7),
        ("cap_under", 0.3, 4.705),
        ("cap_wide", 0.34, 4.705),
    ] {
        let y0 = (w - 0.2) * 0.5;
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 5.0, w),
                rect(inner, via_right - 0.2, y0, via_right, y0 + 0.2),
            ],
        );
    }

    // An extension: a cover 2 wide crossing a 10 µm target bar 2 tall, reaching 0.5
    // past the bar's long walls, then 0.495 and 0.505 past the lower one.  The bar's
    // ends lie outside the cover and are not measured.  ENC.ext: 0, 1, 0;
    // ENC.max_ext: 0, 0, 1.
    for (name, y0) in [
        ("ext_exact", 3.5),
        ("ext_under", 3.505),
        ("ext_over", 3.495),
    ] {
        write(
            name.into(),
            vec![
                rect(inner, 0.0, 4.0, 10.0, 6.0),
                rect(outer, 2.0, y0, 4.0, 6.5),
            ],
        );
    }
}
