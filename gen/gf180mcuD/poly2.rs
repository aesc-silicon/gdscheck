// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Gate poly: patterns for `PL.6`, the `no_corner` rule.
//!
//! "90 degree bends on the COMP are not allowed" is a rule about a *vertex*, not an edge,
//! and its good pattern has to say so from several directions at once: a bend is fine off
//! the active area, a 45° turn is fine on it, and a right angle is fine anywhere inside a
//! YMTP marker. All three are drawn here, so a check that reduced to "any bend" or "any
//! bend on COMP" would fail rather than pass.

use super::OFFSET;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/poly2";

/// An L, as six vertices: five convex right angles and one concave. The arms run well
/// past the COMP they cross, so the shape has a proper end cap and trips no rule but the
/// one under test.
fn ell(l: (i16, i16), x: f64, y: f64) -> gds21::GdsElement {
    poly(
        l,
        &[
            (x - 4.0, y),
            (x + 2.0, y),
            (x + 2.0, y + 5.0),
            (x + 1.5, y + 5.0),
            (x + 1.5, y + 0.5),
            (x - 4.0, y + 0.5),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let comp = layer(pdk, "comp");
    let poly2 = layer(pdk, "poly2_drawn");
    let ymtp = layer(pdk, "ymtp_mk");
    let o = OFFSET;

    // Bad: an L whose elbow sits well inside a COMP island. Two of its six right angles
    // have their probe square inside the COMP - the outer corner of the elbow and the
    // inner one - and the remaining four are out beyond the COMP's edges.
    let bad = vec![
        rect(comp, o, o, o + 4.0, o + 4.0),
        ell(poly2, o + 1.0, o + 1.0),
    ];
    write_gz(&format!("{DIR}/PL.6.bad.gds.gz"), library("TOP", bad));

    // Good, in three parts.
    let mut good = vec![
        // 1. A right-angle bend clear of any COMP.
        ell(poly2, o + 20.0, o),
        // 2. A poly crossing COMP with 45° turns only, its right-angled ends outside.
        rect(comp, o, o, o + 4.0, o + 4.0),
        poly(
            poly2,
            &[
                (o - 2.0, o + 1.0),
                (o + 1.0, o + 1.0),
                (o + 3.0, o + 3.0),
                (o + 6.0, o + 3.0),
                (o + 6.0, o + 3.5),
                (o + 2.8, o + 3.5),
                (o + 0.8, o + 1.5),
                (o - 2.0, o + 1.5),
            ],
        ),
    ];
    // 3. A right-angle bend on COMP, but inside a YMTP marker, which the rule exempts.
    good.push(rect(comp, o + 10.0, o, o + 14.0, o + 4.0));
    good.push(rect(ymtp, o + 9.0, o - 1.0, o + 15.0, o + 5.0));
    good.push(ell(poly2, o + 12.0, o + 1.0));
    write_gz(&format!("{DIR}/PL.6.good.gds.gz"), library("TOP", good));
}
