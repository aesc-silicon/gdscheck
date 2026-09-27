// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Section 3.1's angle rules: metal, active and poly at 0, 45 and 90 degrees only, the
//! cuts at 0 and 90 only, and no corner sharper than 87 degrees on any mask layer.

use super::offgrid::LAYERS as ACUTE_LAYERS;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/angle";

/// The layers held to 0, 45 and 90 degrees.
pub(super) const ANGLE45: &[&str] = &[
    "GatPoly",
    "Activ",
    "Metal1",
    "Metal2",
    "Metal3",
    "Metal4",
    "Metal5",
    "TopMetal1",
    "TopMetal2",
];

/// The layers held to 0 and 90 degrees.
pub(super) const ANGLE90: &[&str] = &[
    "Cont", "Via1", "Via2", "Via3", "Via4", "Vmim", "TopVia1", "TopVia2",
];

/// A regular `n`-gon of radius `r` round `(cx, cy)`, on the 5 nm grid.
fn round(l: (i16, i16), cx: f64, cy: f64, r: f64, n: usize) -> gds21::GdsElement {
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / n as f64;
            let snap = |v: f64| (v * 200.0).round() / 200.0;
            (snap(cx + r * t.cos()), snap(cy + r * t.sin()))
        })
        .collect();
    poly(l, &pts)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let write =
        |name: String, elems| write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));

    // angle45: a box whose top-right corner is cut at 26.6° (one edge; its corners are
    // 117° and 153°); a box chamfered at 45° and a 64-gon of radius 2 are clean.  A
    // circle is IHP's: 16 vertices or more and 1.270 to 1.276 of its box - a 48-gon is
    // 1.277 and no circle.
    for name in ANGLE45 {
        let l = layer(pdk, name);
        write(
            format!("{name}.angle45"),
            vec![
                poly(
                    l,
                    &[(2.0, 2.0), (5.0, 2.0), (5.0, 3.0), (4.0, 3.5), (2.0, 3.5)],
                ),
                poly(
                    l,
                    &[
                        (8.0, 2.0),
                        (11.0, 2.0),
                        (11.0, 3.0),
                        (10.5, 3.5),
                        (8.0, 3.5),
                    ],
                ),
                round(l, 16.0, 4.0, 2.0, 64),
            ],
        );
    }

    // angle90: a 0.19 square with one corner cut at 45° (one edge); a plain one is clean.
    for name in ANGLE90 {
        let l = layer(pdk, name);
        write(
            format!("{name}.angle90"),
            vec![
                poly(
                    l,
                    &[
                        (2.0, 2.0),
                        (2.19, 2.0),
                        (2.19, 2.14),
                        (2.14, 2.19),
                        (2.0, 2.19),
                    ],
                ),
                rect(l, 3.0, 2.0, 3.19, 2.19),
            ],
        );
    }

    // acute: a right triangle (two 45° corners); a quadrilateral with an 86° corner - its
    // right wall leaning 0.28 over 4 - fires, one leaning 0.14 (88°) is clean.  Three.
    for name in ACUTE_LAYERS {
        let l = layer(pdk, name);
        write(
            format!("{name}.acute"),
            vec![
                poly(l, &[(2.0, 2.0), (6.0, 2.0), (6.0, 6.0)]),
                poly(l, &[(8.0, 2.0), (12.0, 2.0), (12.28, 6.0), (8.0, 6.0)]),
                poly(l, &[(14.0, 2.0), (18.0, 2.0), (18.14, 6.0), (14.0, 6.0)]),
            ],
        );
    }

    hardening(pdk);
}

/// Metal2.angle45.h1: a bar whose top wall rises 0.5 over 40 across the tile lines at
/// 20 and 40 (one edge, one marker at every tile size); a round 70 µm pad drawn as a
/// 64-gon (clean), a regular octagon (clean, its walls at 0/45/90) and a diamond
/// (clean) at (1000, 1000).  One.
fn hardening(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal2");
    write_gz(
        &format!("{DIR}/Metal2.angle45.h1.gds.gz"),
        library(
            "TOP",
            vec![
                poly(m, &[(5.0, 2.0), (45.0, 2.0), (45.0, 4.5), (5.0, 4.0)]),
                round(m, 100.0, 50.0, 35.0, 64),
                poly(
                    m,
                    &[
                        (160.0, 40.0),
                        (170.0, 40.0),
                        (175.0, 45.0),
                        (175.0, 55.0),
                        (170.0, 60.0),
                        (160.0, 60.0),
                        (155.0, 55.0),
                        (155.0, 45.0),
                    ],
                ),
                poly(
                    m,
                    &[
                        (1000.0, 998.0),
                        (1002.0, 1000.0),
                        (1000.0, 1002.0),
                        (998.0, 1000.0),
                    ],
                ),
            ],
        ),
    );
}
