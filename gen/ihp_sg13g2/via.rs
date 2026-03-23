// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, rect, space_pattern, exact_width_sealring_pattern, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    for index in 1..5 {
        let dir = format!("tests/data/ihp-sg13g2/via{}", index);
        std::fs::create_dir_all(&dir).expect("failed to create output directory");

        via_a(pdk, index, &dir);
        via_b(pdk, index, &dir);
        via_b1(pdk, index, &dir);
        via_c(pdk, index, &dir);
        via_c1(pdk, index, &dir);
    }
}

/// V{n}.b1 — array spacing.  A 4×4 array tight in both axes (gap 0.25 < 0.29) fails;
/// the same array with the columns relaxed to a 0.31 gap (≥ 0.29) is clean, since
/// only one direction needs the wider spacing.
fn via_b1(pdk: &PdkConfig, index: i32, dir: &str) {
    let v = layer(pdk, &format!("Via{}", index));
    let w = 0.19;
    let array = |px: f64, py: f64| {
        let mut e = Vec::new();
        for r in 0..4 {
            for c in 0..4 {
                let (x, y) = (OFFSET + c as f64 * px, OFFSET + r as f64 * py);
                e.push(rect(v, x, y, x + w, y + w));
            }
        }
        e
    };
    write_gz(&format!("{dir}/V{index}.b1.fail.gds.gz"), library("TOP", array(0.44, 0.44)));
    write_gz(&format!("{dir}/V{index}.b1.gds.gz"), library("TOP", array(0.50, 0.44)));
    // A bond-pad style via RING (one via thick, tight 0.44 pitch along the ring) is
    // not a 2-D array: long tight runs exist, but never more than two stacked, so
    // V{n}.b1 must stay clean (this was a real false-positive class on bondpad cells).
    let mut ring = Vec::new();
    let k = 20; // vias per side
    for i in 0..k {
        let t = OFFSET + i as f64 * 0.44;
        let far = OFFSET + (k - 1) as f64 * 0.44;
        ring.push(rect(v, t, OFFSET, t + w, OFFSET + w)); // bottom
        ring.push(rect(v, t, far, t + w, far + w)); // top
        if i > 0 && i < k - 1 {
            ring.push(rect(v, OFFSET, t, OFFSET + w, t + w)); // left
            ring.push(rect(v, far, t, far + w, t + w)); // right
        }
    }
    write_gz(&format!("{dir}/V{index}.b1.ring.gds.gz"), library("TOP", ring));
}

/// V{n}.c — Metal{n} encloses Via{n} on all sides (`enclosure_pattern`: one clean
/// pair plus four with a short margin on each side).  Via1 needs 0.01, Via2–4 0.005.
fn via_c(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));
    let v = layer(pdk, &format!("Via{}", index));
    let enc = if index == 1 { 0.01 } else { 0.005 };
    let elems = enclosure_pattern(m, v, enc, 0.19, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/V{index}.c.gds.gz"), library("TOP", elems));
}

/// V{n}.c1 — Metal{n} endcap enclosure of Via{n} (0.05 µm on at least one side).  A
/// via with one 0.05 side (others 0.02) passes; a via with 0.02 on every side fails.
fn via_c1(pdk: &PdkConfig, index: i32, dir: &str) {
    let m = layer(pdk, &format!("Metal{}", index));
    let v = layer(pdk, &format!("Via{}", index));
    let elems = vec![
        // clean: left side is a 0.05 endcap, the other three sides are 0.02
        rect(v, 0.0, 0.0, 0.19, 0.19),
        rect(m, -0.05, -0.02, 0.21, 0.21),
        // fail: 0.02 on every side — no side reaches the 0.05 endcap
        rect(v, 10.0, 0.0, 10.19, 0.19),
        rect(m, 9.98, -0.02, 10.21, 0.21),
    ];
    write_gz(&format!("{dir}/V{index}.c1.gds.gz"), library("TOP", elems));
}

fn via_a(pdk: &PdkConfig, index: i32, dir: &str) {
    // exact_width on Via{n}NoSealring: open pattern + a seal-covered copy (still 8).
    let width = 0.19;
    let l = layer(pdk, &format!("Via{}", index));
    let edgeseal = layer(pdk, "EdgeSeal");
    let elems = exact_width_sealring_pattern(l, edgeseal, width, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/V{index}.a.gds.gz"), library("TOP", elems));
}

fn via_b(pdk: &PdkConfig, index: i32, dir: &str) {
    let width = 0.19;
    let space = 0.22;
    let l = layer(pdk, &format!("Via{}", index));
    let elems = space_pattern(l, l, width, space, OFFSET, SPACE_DELTA);
    write_gz(&format!("{dir}/V{index}.b.gds.gz"), library("TOP", elems));
}
