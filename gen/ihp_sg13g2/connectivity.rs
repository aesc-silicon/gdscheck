// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/connectivity";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    two_gates_bridged(pdk);
}

/// Three poly gates, each tied up through Cont→Metal1.  Gates A and B are then joined by a
/// shared Metal2 plate (Metal1→Via1→Metal2), so they are one net; gate C stays on its own
/// Metal1 island and must be a different net.  Used to validate net extraction.
fn two_gates_bridged(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gatpoly = layer(pdk, "GatPoly");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let via1 = layer(pdk, "Via1");
    let metal2 = layer(pdk, "Metal2");
    let o = OFFSET;

    // One poly gate stack at x-origin `gx`; returns its elements.  `with_via` adds a Via1
    // landing inside the shared Metal2 plate.
    let stack = |gx: f64, with_via: bool| {
        let mut e = vec![
            rect(activ, gx, o, gx + 2.0, o + 4.0),
            rect(gatpoly, gx + 0.5, o - 1.0, gx + 1.5, o + 5.0),
            rect(cont, gx + 0.7, o + 1.5, gx + 1.3, o + 2.1),
            rect(metal1, gx - 1.0, o + 1.0, gx + 4.0, o + 3.0),
        ];
        if with_via {
            e.push(rect(via1, gx + 2.5, o + 1.6, gx + 3.3, o + 2.4));
        }
        e
    };

    let mut elems = stack(o, true); // gate A
    elems.extend(stack(o + 8.0, true)); // gate B (8 µm right)
    elems.extend(stack(o + 24.0, false)); // gate C (isolated, no via)
    // Metal2 plate spanning A's and B's Via1 (but not C, 24 µm away).
    elems.push(rect(metal2, o + 2.0, o + 1.0, o + 12.0, o + 3.0));

    write_gz(&format!("{DIR}/two_gates.gds.gz"), library("TOP", elems));
}
