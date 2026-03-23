// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, enclosure_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/pin";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pin_a(pdk);
    pin_b(pdk);
    pin_e(pdk);
    for index in 2..6 {
        pin_f(pdk, index);
    }
    pin_g(pdk);
    pin_h(pdk);
}

fn pin_a(pdk: &PdkConfig) {
    let main = layer(pdk, "Activ");
    let pin = layer(pdk, "Activ.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.a.gds.gz"), library("TOP", elems));
}

fn pin_b(pdk: &PdkConfig) {
    let main = layer(pdk, "GatPoly");
    let pin = layer(pdk, "GatPoly.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.b.gds.gz"), library("TOP", elems));
}

fn pin_e(pdk: &PdkConfig) {
    let main = layer(pdk, "Metal1");
    let pin = layer(pdk, "Metal1.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.e.gds.gz"), library("TOP", elems));
}

fn pin_f(pdk: &PdkConfig, index: i32) {
    let main = layer(pdk, &format!("Metal{index}"));
    let pin = layer(pdk, &format!("Metal{index}.pin"));
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.f.m{index}.gds.gz"), library("TOP", elems));
}

fn pin_g(pdk: &PdkConfig) {
    let main = layer(pdk, "TopMetal1");
    let pin = layer(pdk, "TopMetal1.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.g.gds.gz"), library("TOP", elems));
}

fn pin_h(pdk: &PdkConfig) {
    let main = layer(pdk, "TopMetal2");
    let pin = layer(pdk, "TopMetal2.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.h.gds.gz"), library("TOP", elems));
}
