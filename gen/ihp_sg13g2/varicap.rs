// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! IHP's SVaricap pcell at its defaults (w 3.74, l 0.3, Nx 1), as KLayout draws it: the
//! decks leave a varicap - an Activ under a gate in NWell over nBuLay, labelled
//! "SVaricap" - out of the rules its own geometry breaks.  The label sits on the Activ's
//! edge.

use crate::helpers::{layer, library, rect, text, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/varicap";

/// The pcell's shapes: layer, then the box.
const SHAPES: &[(&str, f64, f64, f64, f64)] = &[
    ("Activ", 1.035, 4.53, 1.275, 5.29),
    ("Activ", 1.035, -0.52, 1.275, 0.24),
    ("Activ", 0.24, 0.24, 1.91, 4.53),
    ("GatPoly", 0.73, 0.64, 1.03, 4.38),
    ("GatPoly", 1.28, 0.39, 1.58, 4.13),
    ("GatPoly", 0.73, 4.38, 1.58, 4.88),
    ("GatPoly", 0.73, -0.11, 1.58, 0.39),
    ("Cont", 0.8, 0.77, 0.96, 0.93),
    ("Cont", 0.8, 1.145, 0.96, 1.305),
    ("Cont", 0.8, 1.52, 0.96, 1.68),
    ("Cont", 0.8, 1.9, 0.96, 2.06),
    ("Cont", 0.8, 2.275, 0.96, 2.435),
    ("Cont", 0.8, 2.65, 0.96, 2.81),
    ("Cont", 0.8, 3.03, 0.96, 3.19),
    ("Cont", 0.8, 3.405, 0.96, 3.565),
    ("Cont", 0.8, 3.78, 0.96, 3.94),
    ("Cont", 0.8, 4.16, 0.96, 4.32),
    ("Cont", 1.35, 0.45, 1.51, 0.61),
    ("Cont", 1.35, 0.825, 1.51, 0.985),
    ("Cont", 1.35, 1.2, 1.51, 1.36),
    ("Cont", 1.35, 1.58, 1.51, 1.74),
    ("Cont", 1.35, 1.955, 1.51, 2.115),
    ("Cont", 1.35, 2.33, 1.51, 2.49),
    ("Cont", 1.35, 2.71, 1.51, 2.87),
    ("Cont", 1.35, 3.085, 1.51, 3.245),
    ("Cont", 1.35, 3.46, 1.51, 3.62),
    ("Cont", 1.35, 3.84, 1.51, 4.0),
    ("Cont", 0.8, 0.06, 0.96, 0.22),
    ("Cont", 1.35, 0.06, 1.51, 0.22),
    ("Cont", 0.8, 4.55, 0.96, 4.71),
    ("Cont", 1.35, 4.55, 1.51, 4.71),
    ("Cont", 0.31, 1.955, 0.47, 2.115),
    ("Cont", 0.31, 2.305, 0.47, 2.465),
    ("Cont", 0.31, 2.655, 0.47, 2.815),
    ("Metal1", 0.75, 0.72, 1.01, 4.37),
    ("Metal1", 1.3, 0.4, 1.56, 4.05),
    ("Metal1", 0.75, 4.37, 1.01, 4.5),
    ("Metal1", 1.3, 0.27, 1.56, 0.4),
    ("Metal1", 0.75, 0.01, 1.56, 0.27),
    ("Metal1", 0.75, 4.5, 1.56, 4.76),
    ("Metal1", 0.29, 1.905, 0.49, 2.865),
    ("Metal1.pin", 0.75, 0.01, 1.56, 0.27),
    ("Metal1.pin", 0.75, 4.5, 1.56, 4.76),
    ("Metal1.pin", 0.29, 1.915, 0.49, 2.855),
    ("pSD", 0.935, 4.63, 1.375, 5.39),
    ("pSD", 0.935, -0.62, 1.375, 0.14),
    ("NWell", 0.0, 0.0, 2.15, 4.77),
    ("nBuLay", 0.24, 0.24, 1.91, 4.53),
    ("ThickGateOx", -0.1, -0.79, 2.25, 5.56),
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let shapes = || -> Vec<_> {
        SHAPES
            .iter()
            .map(|&(l, x0, y0, x1, y1)| rect(layer(pdk, l), x0, y0, x1, y1))
            .collect()
    };
    // SVaricap: the pcell with its label on the Activ's left edge.
    let mut e = shapes();
    e.push(text(layer(pdk, "TEXT"), "SVaricap", 0.24, 0.59));
    write_gz(&format!("{DIR}/SVaricap.gds.gz"), library("TOP", e));
    // SVaricap.nolabel: the same geometry without the label is no varicap.
    write_gz(
        &format!("{DIR}/SVaricap.nolabel.gds.gz"),
        library("TOP", shapes()),
    );
}
