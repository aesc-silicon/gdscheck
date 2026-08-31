// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! OTP marker: patterns for the two rules that rest on thin engine surface.
//!
//! Not the whole deck. `otp_mk` has no false positives at either count and its misses are
//! all pinch- or 45°-shaped, which is a class no axis-aligned fixture reaches — drawing
//! the other fourteen would pin rules that are already exact against geometry that is not
//! where the risk is.
//!
//! The risk is these two. `O.PL.2` is the only user anywhere of `min_width` on an edge
//! layer, a relation added the same day. `O.SB.11` is one of two users of `min_overlap`,
//! which had no drawn pattern at all and reports one marker here where the reference
//! reports six.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/otp_mk";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let otp = layer(pdk, "otp_mk");
    let comp = layer(pdk, "comp");
    let poly = layer(pdk, "poly2_drawn");
    let sab = layer(pdk, "sab");
    let o = OFFSET;

    let write = |id: &str, polarity: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // --- O.PL.2: the gate length, measured between the poly's own two walls -----------
    //
    // The channel edges are `poly.edges and tgate.edges` — the poly's sidewalls where it
    // crosses the active — and the rule is the distance *through the poly* between them.
    // No region carries it: the poly region here is 3 um long and the gate is 0.22 wide.
    //
    // The COMP is drawn wide enough that the poly's overhang clears O.DF.6's 0.22 um and
    // the COMP's own reach clears O.PL.4's 0.14, so nothing but the width is under test.
    // The stripe runs *horizontally*: O.PL.ORT in this same deck forbids the other
    // orientation outright, so a vertical gate would trip it and the width would never be
    // the only thing under test.
    let gate = |w: f64| {
        vec![
            rect(otp, o - 2.0, o - 2.0, o + 6.0, o + 6.0),
            rect(comp, o, o, o + 1.6, o + 3.0),
            rect(poly, o - 0.6, o + 1.2, o + 2.2, o + 1.2 + w),
        ]
    };
    write("O.PL.2", "good", gate(0.22));
    write("O.PL.2", "bad", gate(0.22 - 0.005));

    // --- O.SB.11: how deeply the salicide block covers the COMP it blocks -------------
    //
    // The block overlaps the COMP from one side by `depth`, which is the overlap the rule
    // measures — not a spacing and not an enclosure, since neither shape contains the
    // other. Both are large enough to clear O.SB.13's area minimum.
    let block = |depth: f64| {
        vec![
            rect(otp, o - 2.0, o - 2.0, o + 8.0, o + 8.0),
            rect(comp, o, o, o + 3.0, o + 2.0),
            rect(sab, o + 3.0 - depth, o - 0.3, o + 6.0, o + 2.3),
        ]
    };
    write("O.SB.11", "good", block(0.04));
    write("O.SB.11", "bad", block(0.04 - 0.005));
}
