// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dummy fill: a good and a bad pattern for every rule in the `dummy_comp`,
//! `dummy_poly2` and `dummy_metal` decks.
//!
//! Three decks and forty-odd rules, and nearly all of them say the same thing: keep this
//! fill so far from that layer.  So they are generated from a table of
//! `(rule, fill layer, what it must clear, how far)` rather than drawn one at a time, and
//! a fixture is one fill shape with one neighbour at the limit or a half-grid inside it.
//!
//! Isolation comes free here and is worth saying why: a pattern draws only the partner
//! its own rule names, so every *other* rule in the deck is measuring against a layer
//! that is not in the file.  The two exceptions are handled on their own - the self-space
//! rules, which need two fill shapes and a notch, and `DPF.1`, which is not a distance at
//! all.

use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const OUT: &str = "tests/data/gf180mcuD/generated";
const O: f64 = 10.0;
const D: f64 = 0.005;
/// Fill shapes are square and well clear of anything these decks measure on one shape.
const FILL: f64 = 2.0;

/// Deck, rule, the fill layer, and the space it must keep from each named layer.
const SELF_SPACE: &[(&str, &str, &str, f64)] = &[
    ("dummy_comp", "DCF.2b", "comp_dummy", 1.9),
    ("dummy_poly2", "DPF.2b", "poly2_dummy", 1.1),
    ("dummy_metal", "DM1.2b", "metal1_dummy", 0.98),
    ("dummy_metal", "DM2.2b", "metal2_dummy", 0.98),
    ("dummy_metal", "DM3.2b", "metal3_dummy", 0.98),
    ("dummy_metal", "DM4.2b", "metal4_dummy", 0.98),
    ("dummy_metal", "DM5.2b", "metal5_dummy", 0.98),
];

/// Deck, rule, fill layer, the layer it must clear, and by how much.
const PAIR: &[(&str, &str, &str, &str, f64)] = &[
    ("dummy_comp", "DCF.4", "comp_dummy", "comp", 3.5),
    ("dummy_comp", "DCF.5", "comp_dummy", "poly2_drawn", 1.5),
    ("dummy_comp", "DCF.6a", "comp_dummy", "nwell", 1.3),
    ("dummy_comp", "DCF.6b", "comp_dummy", "dnwell", 4.0),
    ("dummy_comp", "DCF.6c", "comp_dummy", "lvpwell", 1.3),
    ("dummy_comp", "DCF.6d", "comp_dummy", "dualgate", 1.3),
    ("dummy_comp", "DCF.8a", "comp_dummy", "res_mk", 3.5),
    ("dummy_comp", "DCF.11a", "comp_dummy", "ndmy", 3.5),
    ("dummy_comp", "DCF.12", "comp_dummy", "ind_mk", 3.0),
    ("dummy_poly2", "DPF.4", "poly2_dummy", "comp", 3.2),
    ("dummy_poly2", "DPF.5", "poly2_dummy", "poly2_drawn", 5.0),
    ("dummy_poly2", "DPF.6a", "poly2_dummy", "nwell", 1.0),
    ("dummy_poly2", "DPF.6b", "poly2_dummy", "dnwell", 2.0),
    ("dummy_poly2", "DPF.6c", "poly2_dummy", "lvpwell", 1.0),
    ("dummy_poly2", "DPF.6d", "poly2_dummy", "dualgate", 1.0),
    ("dummy_poly2", "DPF.8", "poly2_dummy", "res_mk", 19.7),
    ("dummy_poly2", "DPF.9", "poly2_dummy", "pad", 6.7),
    ("dummy_poly2", "DPF.11", "poly2_dummy", "ndmy", 29.7),
    // DPF.12 and DPF.13 are the space to *active circuit* metal, so the neighbour is the
    // drawn layer: the rule does not apply to dummy metal fill.
    ("dummy_poly2", "DPF.12", "poly2_dummy", "metal1_drawn", 2.0),
    ("dummy_poly2", "DPF.13", "poly2_dummy", "metal2_drawn", 2.0),
    ("dummy_poly2", "DPF.14", "poly2_dummy", "ind_mk", 3.0),
    ("dummy_poly2", "DPF.16", "poly2_dummy", "mtpmark", 3.0),
    ("dummy_poly2", "DPF.19", "poly2_dummy", "pmndmy", 8.0),
    ("dummy_metal", "DM1.3", "metal1_dummy", "metal1_drawn", 2.0),
    ("dummy_metal", "DM2.3", "metal2_dummy", "metal2_drawn", 2.0),
    ("dummy_metal", "DM3.3", "metal3_dummy", "metal3_drawn", 2.0),
    ("dummy_metal", "DM4.3", "metal4_dummy", "metal4_drawn", 2.0),
    ("dummy_metal", "DM5.3", "metal5_dummy", "metal5_drawn", 2.0),
    // DM#.8 keeps fill off anything the fuses and markers are sensitive to; the union has
    // six members and one of them stands for it.
    ("dummy_metal", "DM1.8", "metal1_dummy", "otp_mk", 6.0),
    ("dummy_metal", "DM2.8", "metal2_dummy", "otp_mk", 6.0),
    ("dummy_metal", "DM3.8", "metal3_dummy", "otp_mk", 6.0),
    ("dummy_metal", "DM4.8", "metal4_dummy", "otp_mk", 6.0),
    ("dummy_metal", "DM5.8", "metal5_dummy", "otp_mk", 6.0),
];

/// A fill shape, plus whatever else it needs to be legal on its own.
///
/// Dummy poly has to sit *on* dummy COMP - `DPF.1`, and unlike every other rule in these
/// decks it says something about each fill shape rather than about a distance - so a
/// poly2 fill drawn without a COMP core under it trips that rule in every fixture of the
/// deck.  Giving each one its core is what lets the other twenty rules be read on their
/// own.
fn fill(
    pdk: &PdkConfig,
    l: (i16, i16),
    lname: &str,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Vec<GdsElement> {
    let mut out = vec![rect(l, x0, y0, x1, y1)];
    if lname == "poly2_dummy" {
        let i = 0.4;
        out.push(rect(
            layer(pdk, "comp_dummy"),
            x0 + i,
            y0 + i,
            x1 - i,
            y1 - i,
        ));
    }
    out
}

pub fn generate(pdk: &PdkConfig) {
    for deck in ["dummy_comp", "dummy_poly2", "dummy_metal"] {
        std::fs::create_dir_all(format!("{OUT}/{deck}")).expect("pattern dir");
    }

    for &(deck, id, lname, s) in SELF_SPACE {
        let f = layer(pdk, lname);
        for (name, g) in [("good", s), ("bad", s - D)] {
            write(
                deck,
                &format!("{id}.{name}"),
                [
                    fill(pdk, f, lname, O, O, O + FILL, O + FILL),
                    fill(pdk, f, lname, O + FILL + g, O, O + 2.0 * FILL + g, O + FILL),
                    // A C whose opening is the same gap: the notch the deck checks under
                    // this id is a space within one shape.
                    fill(
                        pdk,
                        f,
                        lname,
                        O,
                        O + 3.0 * FILL,
                        O + FILL,
                        O + 5.0 * FILL + g,
                    ),
                    fill(
                        pdk,
                        f,
                        lname,
                        O + FILL,
                        O + 3.0 * FILL,
                        O + 2.0 * FILL,
                        O + 4.0 * FILL,
                    ),
                    fill(
                        pdk,
                        f,
                        lname,
                        O + FILL,
                        O + 4.0 * FILL + g,
                        O + 2.0 * FILL,
                        O + 5.0 * FILL + g,
                    ),
                ]
                .concat(),
            );
        }
    }

    for &(deck, id, lname, oname, s) in PAIR {
        let (f, o) = (layer(pdk, lname), layer(pdk, oname));
        for (name, g) in [("good", s), ("bad", s - D)] {
            write(
                deck,
                &format!("{id}.{name}"),
                [
                    fill(pdk, f, lname, O, O, O + FILL, O + FILL),
                    vec![rect(o, O + FILL + g, O, O + 2.0 * FILL + g, O + FILL)],
                ]
                .concat(),
            );
        }
    }

    // DPF.1 is the odd one: dummy poly must sit *on* dummy COMP, so the good half puts a
    // COMP fill inside the poly fill and the bad half leaves it out.
    let (p, c) = (layer(pdk, "poly2_dummy"), layer(pdk, "comp_dummy"));
    write(
        "dummy_poly2",
        "DPF.1.good",
        vec![
            rect(p, O, O, O + 4.0, O + 4.0),
            rect(c, O + 1.0, O + 1.0, O + 3.0, O + 3.0),
        ],
    );
    write(
        "dummy_poly2",
        "DPF.1.bad",
        vec![rect(p, O, O, O + 4.0, O + 4.0)],
    );
}

fn write(deck: &str, name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(
        &format!("{OUT}/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}
