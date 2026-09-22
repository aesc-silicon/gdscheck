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

use crate::helpers::{layer, library, poly, rect, write_gz};
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

/// Deck, rule, fill layer, the layer it may not lie on, and how far the good half puts
/// it from that layer - the value of the space rule the prohibition sits beside, since
/// the fill has to clear that too.
const ON: &[(&str, &str, &str, &str, f64)] = &[
    ("dummy_comp", "DCF.13", "comp_dummy", "ind_mk", 3.0),
    ("dummy_poly2", "DPF.15", "poly2_dummy", "ind_mk", 3.0),
    ("dummy_poly2", "DPF.17", "poly2_dummy", "mtpmark", 3.0),
    ("dummy_poly2", "DPF.18", "poly2_dummy", "pmndmy", 8.0),
];

/// Deck, rule and fill layer of the rules that fix the fill's own size at `FILL`.
const SIZE: &[(&str, &str, &str)] = &[
    ("dummy_metal", "DM1.1", "metal1_dummy"),
    ("dummy_metal", "DM2.1", "metal2_dummy"),
    ("dummy_metal", "DM3.1", "metal3_dummy"),
    ("dummy_metal", "DM4.1", "metal4_dummy"),
    ("dummy_metal", "DM5.1", "metal5_dummy"),
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
        // The notch probe is a C, and a C is longer than one fill shape.  Where the deck
        // fixes the fill's own size - DM.1 holds dummy metal to 2 µm each way - no shape
        // with a notch in it can be drawn at all, so the metal patterns leave it out and
        // read the space between two shapes only.
        let notched = deck != "dummy_metal";
        for (name, g) in [("good", s), ("bad", s - D)] {
            let c = if notched {
                [
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
                .concat()
            } else {
                Vec::new()
            };
            write(
                deck,
                &format!("{id}.{name}"),
                [
                    fill(pdk, f, lname, O, O, O + FILL, O + FILL),
                    fill(pdk, f, lname, O + FILL + g, O, O + 2.0 * FILL + g, O + FILL),
                    c,
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

    // The prohibitions beside those spaces: the fill may not lie on the layer at all.
    // The good half stands the fill off by the space rule's own value, the bad half puts
    // it wholly inside - which is the case the manual's wording is written for and the
    // one a marker drawn round a device produces.
    for &(deck, id, lname, oname, clear) in ON {
        let (f, o) = (layer(pdk, lname), layer(pdk, oname));
        let mark = |x: f64| rect(o, x, O - 1.0, x + FILL + 2.0, O + FILL + 1.0);
        write(
            deck,
            &format!("{id}.good"),
            [
                fill(pdk, f, lname, O, O, O + FILL, O + FILL),
                vec![mark(O + FILL + clear)],
            ]
            .concat(),
        );
        write(
            deck,
            &format!("{id}.bad"),
            [
                fill(pdk, f, lname, O, O, O + FILL, O + FILL),
                vec![mark(O - 1.0)],
            ]
            .concat(),
        );
    }

    // The fill's own size, which DM.1 fixes at 2 µm each way.
    for &(deck, id, lname) in SIZE {
        let f = layer(pdk, lname);
        write(
            deck,
            &format!("{id}.good"),
            fill(pdk, f, lname, O, O, O + FILL, O + FILL),
        );
        write(
            deck,
            &format!("{id}.bad"),
            fill(pdk, f, lname, O, O, O + FILL - D, O + FILL),
        );
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

    hardening(pdk);
}

fn write(deck: &str, name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(
        &format!("{OUT}/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from sections 13.1, 13.2 and 13.3 of the manual at each rule's bound and
// one 0.005 µm step past it, with a `#[case]` in the `hardening_dummy_comp`,
// `hardening_dummy_poly2` and `hardening_dummy_metal` tables of `tests/gf180mcuD.rs` and
// the findings in `hardening/reports/gf180mcuD/dummy_{comp,poly2,metal}.md`.
//
// Nearly every rule of these three sections is "keep this fill so far from that layer",
// so the hardening half is a table too, and each rule gets two layouts: `h1` walks the
// distance (the value, the step past it, and the same gap taken corner to corner, which
// is where a euclidian rule and a projected one part company), `h2` puts the two shapes
// on top of one another - abutting, half over, wholly inside - which is where the
// manual's own wording about fill "under" a marker, and about the *boundary* of a well,
// has to be read.  The generic classes - a bound on a bare layer, 45°, unions, arrays, a
// shape far off - belong to the engine family and are not redrawn here.

/// What the partner layer of a fill rule is, for the `h2` probes.
#[derive(Clone, Copy, PartialEq)]
enum Topo {
    /// A layer the fill has to stand clear of sideways: circuit COMP, poly2, a metal
    /// level, a pad, a marker the section only keeps fill away from.  A fill abutting it
    /// has a space of nothing and one lying over it has less than nothing, so both are
    /// the rule's own violation.
    Lateral,
    /// A marker the section keeps fill out of altogether - "Dummy COMP should not exit
    /// under RES_MK" (DCF.8a), "Dummy COMP cannot exit under NDMY" (DCF.11a), "There
    /// should not be any dummy metal pattern fill in the following areas" (DM.8).  The
    /// two lateral probes, and a fill drawn wholly inside the marker.
    Under,
    /// A well or a Dualgate, whose rule names its *boundary* and not its area.  DCF.1a
    /// calls the ground fill has to leave alone "the Region define by DCF.6a, 6b, 6c,
    /// 6d, 6e", and a region defined by a distance to a boundary is a band that
    /// straddles it: a fill inside the well and nearer its boundary than the value
    /// breaks the rule as much as one outside, while a fill deep inside is clear of the
    /// boundary and clean.
    Band,
}

/// Deck, rule, fill layer, partner, value, and how the two meet where they touch.
const H_PAIR: &[(&str, &str, &str, &str, f64, Topo)] = &[
    (
        "dummy_comp",
        "DCF.4",
        "comp_dummy",
        "comp",
        3.5,
        Topo::Lateral,
    ),
    (
        "dummy_comp",
        "DCF.5",
        "comp_dummy",
        "poly2_drawn",
        1.5,
        Topo::Lateral,
    ),
    (
        "dummy_comp",
        "DCF.6a",
        "comp_dummy",
        "nwell",
        1.3,
        Topo::Band,
    ),
    (
        "dummy_comp",
        "DCF.6b",
        "comp_dummy",
        "dnwell",
        4.0,
        Topo::Band,
    ),
    (
        "dummy_comp",
        "DCF.6c",
        "comp_dummy",
        "lvpwell",
        1.3,
        Topo::Band,
    ),
    (
        "dummy_comp",
        "DCF.6d",
        "comp_dummy",
        "dualgate",
        1.3,
        Topo::Band,
    ),
    (
        "dummy_comp",
        "DCF.8a",
        "comp_dummy",
        "res_mk",
        3.5,
        Topo::Under,
    ),
    (
        "dummy_comp",
        "DCF.11a",
        "comp_dummy",
        "ndmy",
        3.5,
        Topo::Under,
    ),
    // DCF.12 is the space to IND_MK; fill *under* IND_MK is DCF.13's own violation and
    // has its own layout below, so this one stops at the lateral probes.
    (
        "dummy_comp",
        "DCF.12",
        "comp_dummy",
        "ind_mk",
        3.0,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.4",
        "poly2_dummy",
        "comp",
        3.2,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.5",
        "poly2_dummy",
        "poly2_drawn",
        5.0,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.6a",
        "poly2_dummy",
        "nwell",
        1.0,
        Topo::Band,
    ),
    (
        "dummy_poly2",
        "DPF.6b",
        "poly2_dummy",
        "dnwell",
        2.0,
        Topo::Band,
    ),
    (
        "dummy_poly2",
        "DPF.6c",
        "poly2_dummy",
        "lvpwell",
        1.0,
        Topo::Band,
    ),
    (
        "dummy_poly2",
        "DPF.6d",
        "poly2_dummy",
        "dualgate",
        1.0,
        Topo::Band,
    ),
    (
        "dummy_poly2",
        "DPF.8",
        "poly2_dummy",
        "res_mk",
        19.7,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.9",
        "poly2_dummy",
        "pad",
        6.7,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.11",
        "poly2_dummy",
        "ndmy",
        29.7,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.12",
        "poly2_dummy",
        "metal1_drawn",
        2.0,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.13",
        "poly2_dummy",
        "metal2_drawn",
        2.0,
        Topo::Lateral,
    ),
    // DPF.14/16/19 are the spaces; fill *under* IND_MK, MTPMARK and PMNDMY is DPF.15,
    // DPF.17 and DPF.18, each with its own layout below.
    (
        "dummy_poly2",
        "DPF.14",
        "poly2_dummy",
        "ind_mk",
        3.0,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.16",
        "poly2_dummy",
        "mtpmark",
        3.0,
        Topo::Lateral,
    ),
    (
        "dummy_poly2",
        "DPF.19",
        "poly2_dummy",
        "pmndmy",
        8.0,
        Topo::Lateral,
    ),
    (
        "dummy_metal",
        "DM1.3",
        "metal1_dummy",
        "metal1_drawn",
        2.0,
        Topo::Lateral,
    ),
    (
        "dummy_metal",
        "DM1.8",
        "metal1_dummy",
        "otp_mk",
        6.0,
        Topo::Under,
    ),
];

/// The largest grid step whose corner-to-corner distance is still under `value`.
fn diag_under(value: f64) -> f64 {
    ((value / std::f64::consts::SQRT_2) / D).floor() * D
}

fn hwrite(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(
        &format!("{OUT}/{deck}/{name}.gds.gz"),
        library("TOP", elems),
    );
}

/// `h1`: the bound, the step past it, and the same two gaps taken corner to corner.
/// Every rule of these sections is euclidian in the foundry's own runset, so the
/// diagonal pair at `diag_under(value)` is a violation and the one a step wider is not.
fn h_bound(pdk: &PdkConfig, deck: &str, id: &str, lname: &str, oname: &str, value: f64) {
    let (f, o) = (layer(pdk, lname), layer(pdk, oname));
    let du = diag_under(value);
    let pitch = 2.0 * value + FILL + 10.0;
    let mut v = Vec::new();
    for (i, &(g, diagonal)) in [
        (value, false),
        (value - D, false),
        (du, true),
        (du + D, true),
    ]
    .iter()
    .enumerate()
    {
        let x = O + i as f64 * pitch;
        v.extend(fill(pdk, f, lname, x, O, x + FILL, O + FILL));
        let (ox, oy) = if diagonal {
            (x + FILL + g, O + FILL + g)
        } else {
            (x + FILL + g, O)
        };
        v.push(rect(o, ox, oy, ox + FILL, oy + FILL));
    }
    hwrite(deck, &format!("{id}.h1"), v);
}

/// `h2`: the fill and its partner on top of one another.  What that means depends on the
/// partner - see `Topo`.
fn h_topo(pdk: &PdkConfig, deck: &str, id: &str, lname: &str, oname: &str, value: f64, topo: Topo) {
    let (f, o) = (layer(pdk, lname), layer(pdk, oname));
    let mut v = Vec::new();
    if topo == Topo::Band {
        // One well, three fills in it: one a step nearer the boundary than the value,
        // one as deep inside as the box allows, one hanging over the far edge.
        let bw = 4.0 * value + 20.0;
        let bh = 2.0 * value + FILL + 8.0;
        v.push(rect(o, O, O, O + bw, O + bh));
        let fy = O + (bh - FILL) / 2.0;
        for x in [
            O + value - D,
            O + bw / 2.0 - FILL / 2.0,
            O + bw - FILL / 2.0,
        ] {
            v.extend(fill(pdk, f, lname, x, fy, x + FILL, fy + FILL));
        }
    } else {
        let pitch = value + 3.0 * FILL + 8.0;
        // Abutting: one shared edge, a space of nothing.
        v.extend(fill(pdk, f, lname, O, O, O + FILL, O + FILL));
        v.push(rect(o, O + FILL, O, O + 2.0 * FILL, O + FILL));
        // Half over: the fill and the partner share ground, which is less than nothing.
        let x = O + pitch;
        v.extend(fill(pdk, f, lname, x, O, x + FILL, O + FILL));
        v.push(rect(o, x + FILL / 2.0, O, x + 1.5 * FILL, O + FILL));
        if topo == Topo::Under {
            // Wholly inside the marker, which the manual forbids in as many words.
            let x = O + 2.0 * pitch;
            v.extend(fill(pdk, f, lname, x, O, x + FILL, O + FILL));
            v.push(rect(o, x - 3.0, O - 3.0, x + FILL + 3.0, O + FILL + 3.0));
        }
    }
    hwrite(deck, &format!("{id}.h2"), v);
}

/// The self-space rules: the bound corner to corner as well as wall to wall, and the
/// same gap laid across the tile lines the engine works in.
fn h_self(pdk: &PdkConfig, deck: &str, id: &str, lname: &str, value: f64) {
    let f = layer(pdk, lname);
    let du = diag_under(value);
    let pitch = 2.0 * value + 2.0 * FILL + 10.0;
    let mut v = Vec::new();
    for (i, &(g, diagonal)) in [
        (value, false),
        (value - D, false),
        (du, true),
        (du + D, true),
    ]
    .iter()
    .enumerate()
    {
        let x = O + i as f64 * pitch;
        v.extend(fill(pdk, f, lname, x, O, x + FILL, O + FILL));
        let (ox, oy) = if diagonal {
            (x + FILL + g, O + FILL + g)
        } else {
            (x + FILL + g, O)
        };
        v.extend(fill(pdk, f, lname, ox, oy, ox + FILL, oy + FILL));
    }
    hwrite(deck, &format!("{id}.h1"), v);

    // The tile lines.  gdscheck works in tiles of 20 µm and the suite also runs at 7 and
    // 100; a gap that straddles a line, a notch open across one and a shape that crosses
    // one must answer the same at every size.
    let g = value - D;
    let mut v = Vec::new();
    // A U whose opening is the bad gap, centred on x = 20.
    let (ux, uy) = (20.0 - FILL - g / 2.0, 10.0);
    v.extend(fill(pdk, f, lname, ux, uy, ux + 2.0 * FILL + g, uy + FILL));
    v.extend(fill(
        pdk,
        f,
        lname,
        ux,
        uy + FILL,
        ux + FILL,
        uy + FILL + 4.0,
    ));
    v.extend(fill(
        pdk,
        f,
        lname,
        ux + FILL + g,
        uy + FILL,
        ux + 2.0 * FILL + g,
        uy + FILL + 4.0,
    ));
    // A bad gap opening on x = 40, its right-hand shape crossing x = 42.
    v.extend(fill(pdk, f, lname, 39.5 - FILL, 10.0, 39.5, 10.0 + FILL));
    v.extend(fill(
        pdk,
        f,
        lname,
        39.5 + g,
        10.0,
        39.5 + g + FILL,
        10.0 + FILL,
    ));
    // A bad gap opening on y = 20.
    v.extend(fill(pdk, f, lname, 60.0, 19.5 - FILL, 60.0 + FILL, 19.5));
    v.extend(fill(
        pdk,
        f,
        lname,
        60.0,
        19.5 + g,
        60.0 + FILL,
        19.5 + g + FILL,
    ));
    // The good gap on the same lines: exactly the value, opening on x = 42.
    v.extend(fill(pdk, f, lname, 41.5 - FILL, 30.0, 41.5, 30.0 + FILL));
    v.extend(fill(
        pdk,
        f,
        lname,
        41.5 + value,
        30.0,
        41.5 + value + FILL,
        30.0 + FILL,
    ));
    hwrite(deck, &format!("{id}.h2"), v);
}

/// One fill square drawn wholly inside a marker the manual says fill may not exist
/// under - the whole of DCF.13, DPF.15, DPF.17 and DPF.18, which are not distances at
/// all.
fn h_under(pdk: &PdkConfig, deck: &str, name: &str, lname: &str, oname: &str) {
    let (f, o) = (layer(pdk, lname), layer(pdk, oname));
    let m = 6.0;
    let mut v = vec![rect(o, O - m, O - m, O + FILL + m, O + FILL + m)];
    v.extend(fill(pdk, f, lname, O, O, O + FILL, O + FILL));
    hwrite(deck, name, v);
}

fn hardening(pdk: &PdkConfig) {
    for &(deck, id, lname, oname, value, topo) in H_PAIR {
        h_bound(pdk, deck, id, lname, oname, value);
        h_topo(pdk, deck, id, lname, oname, value, topo);
    }
    for &(deck, id, lname, value) in &[
        ("dummy_comp", "DCF.2b", "comp_dummy", 1.9),
        ("dummy_poly2", "DPF.2b", "poly2_dummy", 1.1),
        ("dummy_metal", "DM1.2b", "metal1_dummy", 0.98),
    ] {
        h_self(pdk, deck, id, lname, value);
    }

    let (c, p) = (layer(pdk, "comp_dummy"), layer(pdk, "poly2_dummy"));

    // DCF.1c fixes a dummy COMP at a 5 x 5 square and DCF.10 says a truncated one shall
    // not exist: the full square, half a square, and a square with a bite out of it.
    hwrite(
        "dummy_comp",
        "DCF.10.h1",
        vec![
            rect(c, O, O, O + 5.0, O + 5.0),
            rect(c, O + 15.0, O, O + 20.0, O + 2.5),
            poly(
                c,
                &[
                    (O + 30.0, O),
                    (O + 35.0, O),
                    (O + 35.0, O + 2.0),
                    (O + 32.0, O + 2.0),
                    (O + 32.0, O + 5.0),
                    (O + 30.0, O + 5.0),
                ],
            ),
        ],
    );

    // DCF.13: "Dummy COMP should not exist under IND_MK layer".
    h_under(pdk, "dummy_comp", "DCF.13.h1", "comp_dummy", "ind_mk");

    // DCF.7a and DPF.7: the space from fill in the prime die to the scribe line, which
    // at chip level is the distance from the fill to the die's own edge - the PR_BNDRY
    // polygon the density round settled on.  One fill at the value, one a step inside it.
    for (deck, name, lname, value) in [
        ("dummy_comp", "DCF.7a.h1", "comp_dummy", 26.0),
        ("dummy_poly2", "DPF.7.h1", "poly2_dummy", 25.7),
    ] {
        let f = layer(pdk, lname);
        let (bw, bh) = (80.0, 60.0);
        let fy = O + (bh - FILL) / 2.0;
        let mut v = vec![rect(layer(pdk, "pr_bndry"), O, O, O + bw, O + bh)];
        v.extend(fill(
            pdk,
            f,
            lname,
            O + value,
            fy,
            O + value + FILL,
            fy + FILL,
        ));
        let x = O + bw - value + D - FILL;
        v.extend(fill(pdk, f, lname, x, fy, x + FILL, fy + FILL));
        hwrite(deck, name, v);
    }

    // DPF.1: dummy poly2 sits on dummy COMP, 0.3 µm larger per side.  A 5.6 square over
    // its 5.0 core; the same with the core slid half out from under it; one with no core
    // at all; one whose core is drawn to the same outline, which is poly on COMP still.
    let mut v = Vec::new();
    for (i, core) in [Some((0.3, 5.0)), Some((3.3, 5.0)), None, Some((0.0, 5.6))]
        .iter()
        .enumerate()
    {
        let x = O + i as f64 * 12.0;
        v.push(rect(p, x, O, x + 5.6, O + 5.6));
        if let Some((d, s)) = *core {
            v.push(rect(c, x + d, O + d, x + d + s, O + d + s));
        }
    }
    hwrite("dummy_poly2", "DPF.1.h1", v);

    // DPF.10, the poly twin of DCF.10: the 5.6 square DPF.1 asks for, and a truncated
    // one, each still covering a dummy COMP core so DPF.1 has nothing to say.
    hwrite(
        "dummy_poly2",
        "DPF.10.h1",
        vec![
            rect(p, O, O, O + 5.6, O + 5.6),
            rect(c, O + 0.3, O + 0.3, O + 5.3, O + 5.3),
            rect(p, O + 15.0, O, O + 20.6, O + 3.0),
            rect(c, O + 15.3, O + 0.3, O + 20.3, O + 2.7),
        ],
    );

    // DPF.15, DPF.17, DPF.18: dummy poly2 shall not exist under IND_MK, under MTPMARK,
    // or under PMNDMY.
    for (name, marker) in [
        ("DPF.15.h1", "ind_mk"),
        ("DPF.17.h1", "mtpmark"),
        ("DPF.18.h1", "pmndmy"),
    ] {
        h_under(pdk, "dummy_poly2", name, "poly2_dummy", marker);
    }

    // DM.1: "Min/Max Dummy metal line width/length 2.0 um" - the size is fixed, so a
    // narrow one, a wide one and a long one are all violations and only the 2.0 square
    // is clean.
    let m1 = layer(pdk, "metal1_dummy");
    hwrite(
        "dummy_metal",
        "DM.1.h1",
        vec![
            rect(m1, O, O, O + 2.0, O + 2.0),
            rect(m1, O + 10.0, O, O + 11.995, O + 2.0),
            rect(m1, O + 20.0, O, O + 22.005, O + 2.0),
            rect(m1, O + 30.0, O, O + 32.0, O + 4.0),
        ],
    );

    // DM.4/DM.6 (the subsequent metal level) and DM.5/DM.7 (the previous one, which for
    // Metal1 is Poly2): 1 µm of space, and no overlap at all.
    for (name, oname) in [("DM1.4.h1", "metal2_drawn"), ("DM1.5.h1", "poly2_drawn")] {
        let o = layer(pdk, oname);
        let mut v = Vec::new();
        for (i, g) in [1.0, 0.995, -1.0].iter().enumerate() {
            let x = O + i as f64 * 12.0;
            v.push(rect(m1, x, O, x + FILL, O + FILL));
            v.push(rect(o, x + FILL + g, O, x + 2.0 * FILL + g, O + FILL));
        }
        hwrite("dummy_metal", name, v);
    }

    // The five levels are one template: the same two bad gaps on each, one row per
    // level.  Nothing in this deck reaches from one level to another.
    let mut v = Vec::new();
    for n in 1..=5 {
        let (d, dr) = (
            layer(pdk, &format!("metal{n}_dummy")),
            layer(pdk, &format!("metal{n}_drawn")),
        );
        let y = O + (n - 1) as f64 * 12.0;
        v.push(rect(d, O, y, O + FILL, y + FILL));
        v.push(rect(
            d,
            O + FILL + 0.975,
            y,
            O + 2.0 * FILL + 0.975,
            y + FILL,
        ));
        let x = O + 30.0;
        v.push(rect(d, x, y, x + FILL, y + FILL));
        v.push(rect(
            dr,
            x + FILL + 1.995,
            y,
            x + 2.0 * FILL + 1.995,
            y + FILL,
        ));
    }
    hwrite("dummy_metal", "DM.levels.h1", v);

    // DM.8 names six layers and the deck reads their union; one fill at 5.995 from each.
    let mut v = Vec::new();
    for (i, marker) in ["fusetop", "polyfuse", "fusewindow_d", "pmndmy", "mtpmark"]
        .iter()
        .enumerate()
    {
        let x = O + i as f64 * 20.0;
        v.push(rect(m1, x, O, x + FILL, O + FILL));
        v.push(rect(
            layer(pdk, marker),
            x + FILL + 5.995,
            O,
            x + 2.0 * FILL + 5.995,
            O + FILL,
        ));
    }
    hwrite("dummy_metal", "DM1.8.h3", v);
}
