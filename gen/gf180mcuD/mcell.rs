// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Memory-cell marker: a good and a bad pattern for every rule in the `mcell` deck.
//!
//! Five rules over one marker layer, and the only deck here that bounds an *enclosed*
//! area - MC.4 is about the hole in a ring rather than the ring.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/mcell";
const O: f64 = 10.0;
const D: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    let m = layer(pdk, "mcell_feol_mk");
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // MC.1, min width 0.4.  Two microns long, so the area rule has nothing to say.
    for (name, w) in [("good", 0.4), ("bad", 0.4 - D)] {
        write(&format!("MC.1.{name}"), vec![rect(m, O, O, O + w, O + 2.0)]);
    }

    // MC.2, min space 0.4, and the notch under the same id.
    for (name, g) in [("good", 0.4), ("bad", 0.4 - D)] {
        write(
            &format!("MC.2.{name}"),
            vec![
                rect(m, O, O, O + 2.0, O + 2.0),
                rect(m, O + 2.0 + g, O, O + 4.0 + g, O + 2.0),
                rect(m, O, O + 5.0, O + 2.0, O + 5.0 + g + 2.0),
                rect(m, O + 2.0, O + 5.0, O + 4.0, O + 6.0),
                rect(m, O + 2.0, O + 5.0 + g + 1.0, O + 4.0, O + 5.0 + g + 2.0),
            ],
        );
    }

    // MC.3, min area 0.35.  Square and comfortably over the 0.4 width either way.
    for (name, s) in [("good", 0.65), ("bad", 0.55)] {
        write(&format!("MC.3.{name}"), vec![rect(m, O, O, O + s, O + s)]);
    }

    // MC.4, min *enclosed* area 0.35: the hole in the ring, not the ring.  Arms stay a
    // micron thick so nothing else has anything to say about them.
    for (name, h) in [("good", 0.65), ("bad", 0.55)] {
        let a = 1.0; // arm thickness
        write(
            &format!("MC.4.{name}"),
            vec![poly(
                m,
                &[
                    (O, O),
                    (O + h + 2.0 * a, O),
                    (O + h + 2.0 * a, O + h + 2.0 * a),
                    (O, O + h + 2.0 * a),
                    (O, O + a),
                    (O + a, O + a),
                    (O + a, O + a + h),
                    (O + a + h, O + a + h),
                    (O + a + h, O + a),
                    (O, O + a),
                ],
            )],
        );
    }

    hardening(m);
}

// --- Hardening (hardening/SPEC.md) ---
//
// Section 7.17 of the manual is four rules over one marker layer, MTP cell implant, and
// the layer is the whole of the deck's own condition: 11/17 and nothing beside it.  What
// is left to ask is each bound (0.4 µm of width, 0.4 of space, 0.35 µm² of area and 0.35
// of enclosed area), which of the two area rules reads which region, and where a gap that
// looks like a hole is not one.

/// The four bounds and the two area rules, drawn on `mcell_feol_mk`.
fn hardening(m: (i16, i16)) {
    areas(m);
    width_space(m);
    holes(m);
    layers(m);
}

/// MC.3's bound read both ways round the 0.4 µm width: 0.5 by 0.7 is exactly 0.35 µm² and
/// 0.5 by 0.695 is under it; 0.4 by 0.875 is exactly 0.35 at the width bound and 0.4 by
/// 0.87 is under.  Then the same bound on a region drawn as two abutting boxes: 0.4 by 0.4
/// twice is one 0.4 by 0.8 region of 0.32 µm² and one violation, not two, and 0.4 by 0.45
/// twice is 0.36 µm² and clean.
fn areas(m: (i16, i16)) {
    write(
        "MC.area.h1",
        vec![
            rect(m, O, O, O + 0.5, O + 0.7),
            rect(m, O + 2.0, O, O + 2.5, O + 0.695),
            rect(m, O + 4.0, O, O + 4.4, O + 0.875),
            rect(m, O + 6.0, O, O + 6.4, O + 0.87),
            rect(m, O + 8.0, O, O + 8.4, O + 0.4),
            rect(m, O + 8.0, O + 0.4, O + 8.4, O + 0.8),
            rect(m, O + 10.0, O, O + 10.4, O + 0.45),
            rect(m, O + 10.0, O + 0.45, O + 10.4, O + 0.9),
        ],
    );
}

/// MC.1 and MC.2 at the bound: a 0.4 µm bar is legal and 0.395 is not; two markers 0.4
/// apart are legal and 0.395 are not; a U with a 0.4 notch is legal and one with a 0.395
/// notch is not.  Every piece is over 0.35 µm² so the area rules stay out of it.
fn width_space(m: (i16, i16)) {
    let u = |x: f64, g: f64| {
        vec![
            rect(m, x, O, x + 2.0, O + 0.5),
            rect(m, x, O, x + 0.5, O + 2.0),
            rect(m, x + 0.5 + g, O, x + 1.0 + g, O + 2.0),
        ]
    };
    let mut v = vec![
        rect(m, O, O, O + 0.4, O + 2.0),
        rect(m, O + 2.0, O, O + 2.395, O + 2.0),
        rect(m, O + 4.0, O, O + 5.0, O + 1.0),
        rect(m, O + 5.4, O, O + 6.4, O + 1.0),
        rect(m, O + 8.0, O, O + 9.0, O + 1.0),
        rect(m, O + 9.395, O, O + 10.395, O + 1.0),
    ];
    v.extend(u(O + 12.0, 0.4));
    v.extend(u(O + 16.0, 0.395));
    write("MC.width.h1", v);
}

/// MC.4 reads the hole and MC.3 the marker, so a hole under 0.35 µm² is MC.4's and the
/// ring round it is nobody's.  A 0.5 by 0.7 hole is exactly at the bound and a 0.5 by
/// 0.695 one is under it.  Then the shape that looks like a hole and is not: a C, whose
/// inside reaches the outside through a 0.4 µm channel (legal, and no hole) and through a
/// 0.395 one (MC.2's space between the two ends, and still no hole).
fn holes(m: (i16, i16)) {
    // A ring with a `w` by `h` hole and 0.5 µm arms, corner at (x, y).
    let ring = |x: f64, y: f64, w: f64, h: f64| {
        let a = 0.5;
        poly(
            m,
            &[
                (x, y),
                (x + w + 2.0 * a, y),
                (x + w + 2.0 * a, y + h + 2.0 * a),
                (x, y + h + 2.0 * a),
                (x, y + a),
                (x + a, y + a),
                (x + a, y + a + h),
                (x + a + w, y + a + h),
                (x + a + w, y + a),
                (x, y + a),
            ],
        )
    };
    // A C: the ring above with a `g` µm gap cut out of the middle of its right arm.
    let c = |x: f64, y: f64, g: f64| {
        let (a, w, h) = (0.5, 0.5, 0.5);
        let (r0, r1) = (x + a + w, x + 2.0 * a + w);
        let mid = y + a + h / 2.0;
        vec![
            rect(m, x, y, r1, y + a),
            rect(m, x, y + a + h, r1, y + 2.0 * a + h),
            rect(m, x, y + a, x + a, y + a + h),
            rect(m, r0, y + a, r1, mid - g / 2.0),
            rect(m, r0, mid + g / 2.0, r1, y + a + h),
        ]
    };
    let mut v = vec![ring(O, O, 0.5, 0.7), ring(O + 4.0, O, 0.5, 0.695)];
    v.extend(c(O + 8.0, O, 0.4));
    v.extend(c(O + 12.0, O, 0.395));
    write("MC.hole.h1", v);
}

/// The marker is 11/17 and nothing else on layer 11 is this deck's: the same 0.395 µm bar
/// on 11/17 (MC.1), on 11/39 (MVSD, a layer of the section's own chapter but not of this
/// deck) and on the unassigned 11/16 and 11/18.
fn layers(m: (i16, i16)) {
    write(
        "MC.layer.h1",
        vec![
            rect(m, O, O, O + 0.395, O + 2.0),
            rect((11, 39), O + 2.0, O, O + 2.395, O + 2.0),
            rect((11, 16), O + 4.0, O, O + 4.395, O + 2.0),
            rect((11, 18), O + 6.0, O, O + 6.395, O + 2.0),
        ],
    );
}

fn write(name: &str, elems: Vec<gds21::GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}
