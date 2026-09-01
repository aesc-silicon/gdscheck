// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Guard ring: a good and a bad pattern for every rule in the `guard_ring` deck.
//!
//! Four rules, but three of them are named once per layer - GR.2 keeps eight different
//! layers ten microns clear of the marker, GR.4 bounds the width of metal inside it at
//! five levels - so a fixture covers a whole family at once and the id it is named after
//! is the only one that can fire.
//!
//! Every fixture draws a PAD on the marker, because GR.11 says a guard ring without one
//! is itself a violation.  Like `DPF.1` in the dummy decks, it is a statement about the
//! shape rather than about a distance, so it fires on every other fixture in the deck
//! until each one satisfies it.

use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/guard_ring";
const O: f64 = 10.0;
const D: f64 = 0.005;
/// The marker, big enough to hold the widest shape any rule here asks for.
const RING: f64 = 40.0;

/// The eight layers GR.2 keeps clear of the marker.
const CLEARED: &[&str] = &[
    "comp",
    "nwell",
    "poly2_drawn",
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let gr = layer(pdk, "guard_ring_mk");
    let pad = layer(pdk, "pad");

    // The marker with its pad, which every fixture needs to keep GR.11 quiet.
    let ring = |elems: &mut Vec<GdsElement>| {
        elems.push(rect(gr, O, O, O + RING, O + RING));
        elems.push(rect(
            pad,
            O + RING - 2.0,
            O + RING - 2.0,
            O + RING + 3.0,
            O + RING + 3.0,
        ));
    };

    // GR.2, ten microns from the marker to each of eight layers.  They sit in a row below
    // it, so each is clear of the others and none of them overlaps the marker - an
    // overlap would put the shape *inside* the ring, which is what GR.4 and GR.6 measure.
    for (name, g) in [("good", 10.0), ("bad", 10.0 - D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        for (i, lname) in CLEARED.iter().enumerate() {
            let x = O + i as f64 * 4.0;
            elems.push(rect(layer(pdk, lname), x, O - g - 2.0, x + 2.0, O - g));
        }
        write(&format!("GR.2.{name}"), elems);
    }

    // GR.4, metal inside the ring is at least 12 µm wide, at each of five levels.  The
    // metal overlaps the marker, which is what puts it inside; an overlapping pair is not
    // a gap, so GR.2 has nothing to say about it.
    for (name, w) in [("good", 12.0), ("bad", 12.0 - D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        for (i, lname) in [
            "metal1_drawn",
            "metal2_drawn",
            "metal3_drawn",
            "metal4_drawn",
            "metal5_drawn",
        ]
        .iter()
        .enumerate()
        {
            let y = O + 2.0 + i as f64 * 6.0;
            elems.push(rect(layer(pdk, lname), O + 2.0, y, O + 2.0 + 20.0, y + w));
        }
        write(&format!("GR.4.{name}"), elems);
    }

    // GR.6, the same for COMP, at 16 µm.
    for (name, w) in [("good", 16.0), ("bad", 16.0 - D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        elems.push(rect(
            layer(pdk, "comp"),
            O + 2.0,
            O + 2.0,
            O + 22.0,
            O + 2.0 + w,
        ));
        write(&format!("GR.6.{name}"), elems);
    }

    // GR.11: the ring needs a pad.  The bad half is the one fixture here without one.
    let mut good = Vec::new();
    ring(&mut good);
    write("GR.11.good", good);
    write("GR.11.bad", vec![rect(gr, O, O, O + RING, O + RING)]);
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
