// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, min_width_pattern, notch_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/passiv";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pas_a(pdk);
    pas_b_space(pdk);
    pas_b_notch(pdk);
    pas_c(pdk);
}

fn pas_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = min_width_pattern(l, 2.10, 2.10, 10.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.a.gds.gz"), library("TOP", elems));
}

fn pas_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = space_pattern(l, l, 2.1, 3.5, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.b.space.gds.gz"), library("TOP", elems));
}

fn pas_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "Passiv");
    let elems = notch_pattern(l, 2.1, 3.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pas.b.notch.gds.gz"), library("TOP", elems));
}

/// `Pas.c` — min TopMetal2 enclosure of Passiv (2.10 µm), checked only inside the
/// seal (`PassivInSeal`).  Two identical enclosure patterns: the first is covered by
/// an EdgeSeal box (so it is checked and its 4 under-enclosed cases violate); the
/// second has no EdgeSeal, so it is outside the seal and must produce no violations.
fn pas_c(pdk: &PdkConfig) {
    let tm2 = layer(pdk, "TopMetal2");
    let passiv = layer(pdk, "Passiv");
    let edgeseal = layer(pdk, "EdgeSeal");

    let (enc, width, dist) = (2.10, 10.0, 5.0);
    let outer = width + 2.0 * enc; // outer square side of each enclosure pair
    let step = outer + dist;
    let span = 4.0 * step + outer; // x-extent of the 5-pair pattern from its offset

    let mut elems = Vec::new();
    // Inside the seal: fully covered by an EdgeSeal box → checked.
    elems.append(&mut enclosure_pattern(tm2, passiv, enc, width, dist, OFFSET, SPACE_DELTA));
    elems.push(rect(edgeseal, OFFSET - 1.0, -1.0, OFFSET + span + 1.0, outer + 1.0));
    // Outside the seal: identical pattern, no EdgeSeal → exempt, no violations.
    let off2 = OFFSET + span + 50.0;
    elems.append(&mut enclosure_pattern(tm2, passiv, enc, width, dist, off2, SPACE_DELTA));

    write_gz(&format!("{DIR}/Pas.c.gds.gz"), library("TOP", elems));
}
