// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, space_pattern, exact_width_sealring_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    for index in 1..3 {
        let dir = format!("tests/data/ihp-sg13g2/topvia{}", index);
        std::fs::create_dir_all(&dir).expect("failed to create output directory");

        topvia_a(pdk, index, &dir);
        topvia_b(pdk, index, &dir);
        topvia_c(pdk, index, &dir);
        topvia_d(pdk, index, &dir);
    }
}

/// `TV{n}.c` — enclosure of the via by the lower metal (Metal5 for TopVia1,
/// TopMetal1 for TopVia2).  `enclosure_pattern` emits one clean pair plus four with
/// a short margin on each side.
fn topvia_c(pdk: &PdkConfig, index: i32, dir: &str) {
    let (metal, enc) = if index == 1 { ("Metal5", 0.10) } else { ("TopMetal1", 0.50) };
    let via_w = if index == 1 { 0.42 } else { 0.90 };
    let m = layer(pdk, metal);
    let v = layer(pdk, &format!("TopVia{}", index));
    let elems = enclosure_pattern(m, v, enc, via_w, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TV{index}.c.gds.gz"), library("TOP", elems));
}

/// `TV{n}.d` — enclosure of the via by the upper metal (TopMetal1 for TopVia1,
/// TopMetal2 for TopVia2).
fn topvia_d(pdk: &PdkConfig, index: i32, dir: &str) {
    let (metal, enc) = if index == 1 { ("TopMetal1", 0.42) } else { ("TopMetal2", 0.50) };
    let via_w = if index == 1 { 0.42 } else { 0.90 };
    let m = layer(pdk, metal);
    let v = layer(pdk, &format!("TopVia{}", index));
    let elems = enclosure_pattern(m, v, enc, via_w, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TV{index}.d.gds.gz"), library("TOP", elems));
}

fn topvia_a(pdk: &PdkConfig, index: i32, dir: &str) {
    // exact_width on TopVia{n}NoSealring: open pattern + a seal-covered copy (still 8).
    let width = if index == 1 { 0.42 } else { 0.90 };
    let l = layer(pdk, &format!("TopVia{}", index));
    let edgeseal = layer(pdk, "EdgeSeal");
    let elems = exact_width_sealring_pattern(l, edgeseal, width, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TV{index}.a.gds.gz"), library("TOP", elems));
}

fn topvia_b(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = if index == 1 { 0.42 } else { 0.90 };
    let space = if index == 1 { 0.42 } else { 1.06 };
    let l = layer(pdk, &format!("TopVia{}", index));
    let elems = space_pattern(l, l, width, space, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/TV{index}.b.gds.gz"), library("TOP", elems));
}
