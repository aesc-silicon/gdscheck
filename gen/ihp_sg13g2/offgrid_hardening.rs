// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the `offgrid` and `forbidden` decks (hardening/SPEC.md):
//! vertices off the 0.005 µm grid by every amount, on tile lines, on 45° walls, far
//! away, in arrays; forbidden shapes across tile lines, overlapping, in arrays.

use crate::helpers::{flat_array, layer, library, poly, rect, ref_array, write_gz};
use gdscheck::pdk::PdkConfig;

const OFFGRID: &str = "tests/data/ihp-sg13g2/offgrid";
const FORBIDDEN: &str = "tests/data/ihp-sg13g2/forbidden";

pub fn generate(pdk: &PdkConfig) {
    let m = layer(pdk, "Metal1");
    let write = |name: &str, elems| {
        write_gz(
            &format!("{OFFGRID}/Metal1.offgrid.{name}.gds.gz"),
            library("TOP", elems),
        );
    };

    // h1 — how far off.  A box whose right edge is 0.001 off, one 0.0025 off (half a
    // step), one 0.004 off, one 0.006 off (a step and a fifth): two vertices each;
    // a box with all four vertices off (4); one on grid at -0.005 and one at -0.003
    // (2); a 0.005-exact box at 0.995 (clean).  Eighteen.
    write(
        "h1",
        vec![
            rect(m, 2.0, 2.0, 3.001, 3.0),
            rect(m, 5.0, 2.0, 6.0025, 3.0),
            rect(m, 8.0, 2.0, 9.004, 3.0),
            rect(m, 11.0, 2.0, 12.006, 3.0),
            rect(m, 14.001, 2.001, 15.001, 3.001),
            rect(m, -1.005, 2.0, -0.005, 3.0),
            rect(m, -4.003, 2.0, -3.003, 3.0),
            rect(m, 0.995, 5.0, 1.995, 6.0),
        ],
    );

    // h2 — 45° walls.  A diamond on grid (clean); a diamond whose top vertex is 0.002
    // off (1); a 45° strip whose four vertices are on grid (clean) and one whose far
    // end is 0.003 off in y (2, both vertices of that end); a chamfered box with the
    // chamfer's vertices on grid (clean).  Three.
    write(
        "h2",
        vec![
            poly(m, &[(3.0, 2.0), (4.0, 3.0), (3.0, 4.0), (2.0, 3.0)]),
            poly(m, &[(7.0, 2.0), (8.0, 3.0), (7.0, 4.002), (6.0, 3.0)]),
            poly(m, &[(10.0, 2.0), (10.5, 2.0), (13.0, 4.5), (12.5, 4.5)]),
            poly(m, &[(15.0, 2.0), (15.5, 2.0), (18.0, 4.503), (17.5, 4.503)]),
            poly(
                m,
                &[
                    (20.0, 2.0),
                    (21.0, 2.0),
                    (21.0, 3.0),
                    (20.5, 3.5),
                    (20.0, 3.5),
                ],
            ),
        ],
    );

    // h3 — tile lines and far.  A vertex 0.002 past x = 20, one 0.002 short of 40,
    // one on 21 exactly (clean), a box straddling 100 with its far edge 0.003 off, a
    // box at (1000.003, 1000) and one at (-1000, -1000.002): two each.  Ten.
    write(
        "h3",
        vec![
            rect(m, 19.0, 2.0, 20.002, 3.0),
            rect(m, 39.0, 2.0, 39.998, 3.0),
            rect(m, 20.0, 5.0, 21.0, 6.0),
            rect(m, 99.0, 2.0, 101.003, 3.0),
            rect(m, 1000.003, 1000.0, 1001.003, 1001.0),
            rect(m, -1001.0, -1001.002, -1000.0, -1000.002),
        ],
    );

    // h4/h5 — fifty boxes with an off-grid right edge, flat and as a GdsArrayRef of
    // an on-grid pitch (100 vertices); h6 — fifty on-grid boxes as a GdsArrayRef
    // whose pitch is 2.003, so every placed copy but the first is off (196).
    let cell = vec![rect(m, 0.2, 0.2, 1.201, 1.2)];
    write_gz(
        &format!("{OFFGRID}/Metal1.offgrid.h4.gds.gz"),
        library("TOP", flat_array(&cell, 10, 5, 2.0)),
    );
    write_gz(
        &format!("{OFFGRID}/Metal1.offgrid.h5.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );
    let on = vec![rect(m, 0.2, 0.2, 1.2, 1.2)];
    write_gz(
        &format!("{OFFGRID}/Metal1.offgrid.h6.gds.gz"),
        ref_array(on, 10, 5, 2.003),
    );

    // forbidden — h1: a BiWind across x = 20 and 40 (one shape, one report), one at
    // (1000, 1000), a BiWind overlapping a PEmWind (one each), a NoDRC 0.005 square;
    // h2/h3: fifty LDMOS boxes, flat and as a GdsArrayRef.
    let bi = layer(pdk, "BiWind");
    let pem = layer(pdk, "PEmWind");
    let nodrc = layer(pdk, "NoDRC");
    let ldmos = layer(pdk, "LDMOS");
    write_gz(
        &format!("{FORBIDDEN}/forbidden.h1.gds.gz"),
        library(
            "TOP",
            vec![
                rect(bi, 15.0, 2.0, 45.0, 3.0),
                rect(bi, 1000.0, 1000.0, 1001.0, 1001.0),
                rect(bi, 2.0, 6.0, 4.0, 8.0),
                rect(pem, 3.0, 7.0, 5.0, 9.0),
                rect(nodrc, 8.0, 6.0, 8.005, 6.005),
            ],
        ),
    );
    let cell = vec![rect(ldmos, 0.2, 0.2, 1.2, 1.2)];
    write_gz(
        &format!("{FORBIDDEN}/forbidden.h2.gds.gz"),
        library("TOP", flat_array(&cell, 10, 5, 2.0)),
    );
    write_gz(
        &format!("{FORBIDDEN}/forbidden.h3.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );
}
