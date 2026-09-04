// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`min_enclosure`](gdscheck::checks::min_enclosure): a margin measured on
//! three shape families, swept either side of the limit, under both metrics.
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
        vec![
            poly(outer, &two_chamfers),
            rect(inner, 1.0, 5.5, 8.5, 8.5),
        ],
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
}
