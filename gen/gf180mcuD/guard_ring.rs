// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Guard ring: a good and a bad pattern for every rule in the `guard_ring` deck.
//!
//! The ring is drawn the way the real one in `static/guardring.gds.gz` is, scaled down:
//! a band `B` wide with a 45° chamfer `C` across each corner, its active laid out as four
//! corner pieces and four bars between them, the marker one annulus over the same
//! outline, and P+ over the active.  Three rules are statements about that shape rather
//! than about a distance and so fire on every other fixture until it is right - GR.1 wants
//! the marker and the active to share an outline, GR.3 wants the P+ over the active, and
//! GR.11 wants a pad on the marker - which is why every fixture starts from the same ring
//! and perturbs one thing.
//!
//! The real ring's own numbers are the good halves here: its active band is 16 µm, which
//! is exactly GR.6's minimum, and its metal 15 µm against GR.4's 12.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/guard_ring";
/// Lower left of the ring's outline.
const O: f64 = 10.0;
const W: f64 = 160.0;
const H: f64 = 120.0;
/// The band, and the chamfer across each corner - both the real ring's.
const B: f64 = 16.0;
const C: f64 = 9.0;
const D: f64 = 0.005;

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

const METALS: &[&str] = &[
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

/// A ring band `b` wide on layer `l`, as the eight pieces the real ring is drawn as:
/// four corners carrying the 45° chamfer, four bars between them.
fn band(l: (i16, i16), b: f64) -> Vec<GdsElement> {
    let (x1, y1) = (O + W, O + H);
    vec![
        poly(
            l,
            &[
                (O + C, O),
                (O, O + C),
                (O, O + b),
                (O + b, O + b),
                (O + b, O),
            ],
        ),
        poly(
            l,
            &[
                (x1 - C, O),
                (x1 - b, O),
                (x1 - b, O + b),
                (x1, O + b),
                (x1, O + C),
            ],
        ),
        poly(
            l,
            &[
                (x1, y1 - C),
                (x1, y1 - b),
                (x1 - b, y1 - b),
                (x1 - b, y1),
                (x1 - C, y1),
            ],
        ),
        poly(
            l,
            &[
                (O + C, y1),
                (O + b, y1),
                (O + b, y1 - b),
                (O, y1 - b),
                (O, y1 - C),
            ],
        ),
        rect(l, O + b, O, x1 - b, O + b),
        rect(l, O + b, y1 - b, x1 - b, y1),
        rect(l, O, O + b, O + b, y1 - b),
        rect(l, x1 - b, O + b, x1, y1 - b),
    ]
}

/// The marker: one annulus over a band `b` wide, written as the keyhole polygon the real
/// ring uses - the outer boundary with its chamfers, then the inner rectangle.
fn marker(l: (i16, i16), b: f64) -> GdsElement {
    let (x1, y1) = (O + W, O + H);
    poly(
        l,
        &[
            (O + C, O),
            (O, O + C),
            (O, y1 - b),
            (O + b, y1 - b),
            (O + b, O + b),
            (x1 - b, O + b),
            (x1 - b, y1 - b),
            (O, y1 - b),
            (O, y1 - C),
            (O + C, y1),
            (x1 - C, y1),
            (x1, y1 - C),
            (x1, O + C),
            (x1 - C, O),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let gr = layer(pdk, "guard_ring_mk");
    let pad = layer(pdk, "pad");
    let comp = layer(pdk, "comp");
    let pplus = layer(pdk, "pplus");

    // The ring every fixture starts from: the marker, the active under it, P+ over that,
    // metal on each level, and the pad GR.11 wants.
    let ring = |elems: &mut Vec<GdsElement>| {
        elems.push(marker(gr, B));
        elems.extend(band(comp, B));
        elems.extend(band(pplus, B));
        for m in METALS {
            elems.extend(band(layer(pdk, m), B - 1.0));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
    };

    // GR.2, ten microns from the marker to each of nine layers.  They sit inside the
    // ring's hole, which is where the die they belong to is.
    for (name, g) in [("good", 10.0), ("bad", 10.0 - D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        for (i, lname) in CLEARED.iter().enumerate() {
            let x = O + B + g + i as f64 * 6.0;
            elems.push(rect(
                layer(pdk, lname),
                x,
                O + B + g,
                x + 4.0,
                O + B + g + 4.0,
            ));
        }
        write(&format!("GR.2.{name}"), elems);
    }

    // GR.4, the ring's metal is at least 12 µm wide at each of six levels.  The real ring
    // draws it one micron inside the active, at 15.
    for (name, w) in [("good", 15.0), ("bad", 12.0 - D)] {
        let mut elems = vec![marker(gr, B)];
        elems.extend(band(comp, B));
        elems.extend(band(pplus, B));
        for m in METALS {
            elems.extend(band(layer(pdk, m), w));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
        write(&format!("GR.4.{name}"), elems);
    }

    // GR.6, the ring's own P+ active is at least 16 µm wide - which is exactly what the
    // real one is.  The marker follows the active, so that GR.1 stays satisfied.
    for (name, w) in [("good", 0.0), ("bad", D)] {
        let mut elems = vec![marker(gr, B - w)];
        elems.extend(band(comp, B - w));
        elems.extend(band(pplus, B - w));
        for m in METALS {
            elems.extend(band(layer(pdk, m), B - 1.0 - w));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
        write(&format!("GR.6.{name}"), elems);
    }

    // GR.1: the marker reaching past the ring's active.
    for (name, over) in [("good", 0.0), ("bad", D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        if over > 0.0 {
            elems.push(rect(gr, O + W, O + B, O + W + over, O + H - B));
        }
        write(&format!("GR.1.{name}"), elems);
    }

    // GR.3: the ring's active with the implant stopping short of it.
    for (name, short) in [("good", 0.0), ("bad", D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        if short > 0.0 {
            elems.retain(|e| {
                !matches!(e, GdsElement::GdsBoundary(b)
                if (b.layer, b.datatype) == (pplus.0 as i16, pplus.1 as i16))
            });
            // The implant slides off one end of the band rather than narrowing it, so the
            // active it leaves bare is GR.3's business and stays GR.6's right width.
            let mut pp = band(pplus, B);
            pp[4] = rect(pplus, O + B + short, O, O + W - B, O + B);
            elems.extend(pp);
        }
        write(&format!("GR.3.{name}"), elems);
    }

    // GR.7 and GR.8, the ring's own contacts and vias no closer than 0.7 µm.  They sit in
    // the band, which is what puts them in the ring; the real ring is built to exactly
    // this spacing on every level it uses.
    for (id, names) in [
        ("GR.7", &["contact"][..]),
        ("GR.8", &["via1", "via2", "via3", "via4"][..]),
    ] {
        for (half, s) in [("good", 0.7), ("bad", 0.7 - D)] {
            let mut elems = Vec::new();
            ring(&mut elems);
            for (i, lname) in names.iter().enumerate() {
                let l = layer(pdk, lname);
                let y = O + 2.0 + i as f64 * 2.0;
                elems.push(rect(l, O + B + 2.0, y, O + B + 2.22, y + 0.22));
                elems.push(rect(l, O + B + 2.22 + s, y, O + B + 2.44 + s, y + 0.22));
            }
            write(&format!("{id}.{half}"), elems);
        }
    }

    // GR.11: the ring needs a pad.  The bad half is the one fixture here without one.
    let mut good = Vec::new();
    ring(&mut good);
    write("GR.11.good", good);
    let mut bad = vec![marker(gr, B)];
    bad.extend(band(comp, B));
    bad.extend(band(pplus, B));
    for m in METALS {
        bad.extend(band(layer(pdk, m), B - 1.0));
    }
    write("GR.11.bad", bad);
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
