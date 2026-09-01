// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Gate poly: a good and a bad pattern for every rule in the `poly2` deck.
//!
//! The deck splits nearly every rule by voltage, and reads the split off the markers: low
//! voltage is poly no Dualgate touches, medium voltage is poly Dualgate overlaps, and a
//! V5_XTOR marker on top of that separates 5 V from 6 V.  So a fixture picks its column
//! by what it draws over the poly, and one drawn for one column is silent in the other.
//!
//! Two of the width rules read the gate's *edges* rather than the poly: PL.2 measures the
//! stretch of gate edge that lies on the active, which is the channel, and PL.7 measures
//! only the stretches running at 45°.  PL.7's fixtures are therefore the only diagonal
//! ones here, and its medium-voltage device is a P-channel: at 6 V the deck asks 0.7 µm
//! of an N channel and 0.55 of a P one, and a fixture bending PL.7's 0.7 µm limit would
//! break the N-channel rule on the way past.
//!
//! `PL.6`, "90 degree bends on the COMP are not allowed", is a rule about a *vertex*, not an edge,
//! and its good pattern has to say so from several directions at once: a bend is fine off
//! the active area, a 45° turn is fine on it, and a right angle is fine anywhere inside a
//! YMTP marker. All three are drawn here, so a check that reduced to "any bend" or "any
//! bend on COMP" would fail rather than pass.

use super::OFFSET;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/poly2";

/// An L, as six vertices: five convex right angles and one concave. The arms run well
/// past the COMP they cross, so the shape has a proper end cap and trips no rule but the
/// one under test.
fn ell(l: (i16, i16), x: f64, y: f64) -> gds21::GdsElement {
    poly(
        l,
        &[
            (x - 4.0, y),
            (x + 2.0, y),
            (x + 2.0, y + 5.0),
            (x + 1.5, y + 5.0),
            (x + 1.5, y + 0.5),
            (x - 4.0, y + 0.5),
        ],
    )
}

/// The configuration `PL.5a`/`PL.5b` lose nine markers each to, reduced from the site at
/// (-3830.478, 881.684) in the foundry case.
///
/// A COMP's corner sits *exactly* on the gate poly's boundary. The two walls are
/// collinear and run away from that point in opposite directions, so they meet at one
/// point and nowhere else. KLayout calls that a separation of zero and reports it; this
/// engine calls a boundary touch no spacing at all and reports nothing, which is a
/// deliberate convention (`check_tile`: "Touching: shapes share a boundary -> no
/// spacing") and not specific to this rule or this deck.
///
/// Drawn so the two are otherwise clear of each other: every other part of the poly is
/// more than the rule's limit from the COMP, so the point of contact is the only thing
/// either engine could be reporting.
fn kissing_corner(comp: (i16, i16), poly2: (i16, i16), x: f64, y: f64) -> Vec<gds21::GdsElement> {
    vec![
        // The COMP sits up and to the right of (x, y), with that corner on the wall.
        rect(comp, x, y, x + 1.2, y + 1.2),
        // The poly runs down and to the left, its right wall collinear with the COMP's
        // left wall below the shared point.
        rect(poly2, x - 1.2, y - 1.2, x, y),
    ]
}

/// The same two shapes pulled a comfortable distance apart, which both engines call clean.
fn split_corner(comp: (i16, i16), poly2: (i16, i16), x: f64, y: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(comp, x, y, x + 1.2, y + 1.2),
        rect(poly2, x - 1.2 - 0.5, y - 1.2 - 0.5, x - 0.5, y - 0.5),
    ]
}

/// A 45° bar `w` µm wide across, running from `(x, y)` for `len` µm.
fn diagonal(l: (i16, i16), x: f64, y: f64, w: f64, len: f64) -> GdsElement {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let (dx, dy) = (h * len, h * len);
    let (nx, ny) = (h * w, -h * w);
    poly(
        l,
        &[
            (x, y),
            (x + nx, y + ny),
            (x + nx + dx, y + ny + dy),
            (x + dx, y + dy),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let comp = layer(pdk, "comp");
    let poly2 = layer(pdk, "poly2_drawn");
    let ymtp = layer(pdk, "ymtp_mk");
    let nplus = layer(pdk, "nplus");
    let pplus = layer(pdk, "pplus");
    let nwell = layer(pdk, "nwell");
    let dualgate = layer(pdk, "dualgate");
    let plfuse = layer(pdk, "plfuse");
    let v5 = layer(pdk, "v5_xtor");
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    // Over the whole scene: what moves a fixture from the low-voltage column to the
    // medium-voltage one.
    let mv = |v: &mut Vec<GdsElement>| v.push(rect(dualgate, o - 3.0, o - 3.0, o + 9.0, o + 9.0));

    // PL.1 and PL.1a: the poly's own width, off a fuse marker and then on one.
    for (id, w, thick, fuse) in [
        ("PL.1_LV", 0.175, false, false),
        ("PL.1_MV", 0.195, true, false),
        ("PL.1a_LV", 0.175, false, true),
        ("PL.1a_MV", 0.175, true, true),
    ] {
        for (polarity, w) in [("good", 0.4), ("bad", w)] {
            let mut v = vec![rect(poly2, o, o, o + w, o + 3.0)];
            if fuse {
                v.push(rect(plfuse, o - 0.4, o - 0.4, o + w + 0.4, o + 3.4));
            }
            if thick {
                mv(&mut v);
            }
            write(id, polarity, v);
        }
    }

    // PL.2: the channel, which is the stretch of gate edge lying on the active.  The
    // medium-voltage device carries the implant that makes it an N channel; the
    // low-voltage one needs none, since that rule reads any gate.
    for (id, len, thick) in [("PL.2_LV", 0.275, false), ("PL.2_MV", 0.695, true)] {
        for (polarity, len) in [("good", 1.0), ("bad", len)] {
            let mut v = vec![
                rect(comp, o, o, o + 3.0, o + 1.0),
                rect(poly2, o + 1.0, o - 0.4, o + 1.0 + len, o + 1.4),
            ];
            if thick {
                v.push(rect(nplus, o - 0.2, o - 0.2, o + 3.2, o + 1.2));
                mv(&mut v);
            }
            write(id, polarity, v);
        }
    }

    // PL.3a: poly to poly, on the field.
    for (polarity, gap) in [("good", 0.5), ("bad", 0.235)] {
        write(
            "PL.3a",
            polarity,
            vec![
                rect(poly2, o, o, o + 0.4, o + 3.0),
                rect(poly2, o + 0.4 + gap, o, o + 0.8 + gap, o + 3.0),
            ],
        );
    }

    // PL.4: how far the gate reaches past the active.  No implant, so the channel rules
    // have no N or P gate to measure and only this one is left.
    for (id, thick) in [("PL.4_LV", false), ("PL.4_MV", true)] {
        for (polarity, cap) in [("good", 0.4), ("bad", 0.215)] {
            let mut v = vec![
                rect(comp, o, o, o + 3.0, o + 1.0),
                rect(poly2, o + 1.0, o - cap, o + 1.4, o + 1.0 + cap),
            ];
            if thick {
                mv(&mut v);
            }
            write(id, polarity, v);
        }
    }

    // PL.5a and PL.5b: poly beside an active it does not cross.  The deck states the two
    // as one measurement, so either fixture answers for both.
    for (id, gap, thick) in [
        ("PL.5a_LV", 0.095, false),
        ("PL.5b_LV", 0.095, false),
        ("PL.5a_MV", 0.295, true),
        ("PL.5b_MV", 0.295, true),
    ] {
        for (polarity, gap) in [("good", 0.6), ("bad", gap)] {
            let mut v = vec![
                rect(comp, o, o, o + 2.0, o + 2.0),
                rect(poly2, o + 2.0 + gap, o, o + 2.4 + gap, o + 2.0),
            ];
            if thick {
                mv(&mut v);
            }
            write(id, polarity, v);
        }
    }

    // PL.7: the same channel measured only where the gate runs at 45°.
    //
    // The widths are picked from a sweep rather than set to the limit minus 5 nm like
    // everything else here.  A diagonal gate is not measured at every width: between
    // roughly 0.20 and 0.26 µm this engine finds no channel at all, and PL.2 - which
    // reads the same edges without an angle filter - goes quiet over the same band, so
    // whatever is lost goes before the filter rather than in it.  Above and below that
    // band both rules read the gate correctly.
    for (id, w, thick) in [("PL.7_LV", 0.29, false), ("PL.7_MV", 0.695, true)] {
        for (polarity, w) in [("good", 1.2), ("bad", w)] {
            let mut v = vec![rect(comp, o, o, o + 3.0, o + 3.0)];
            v.push(diagonal(poly2, o + 0.5, o - 1.0, w, 6.0));
            if thick {
                // A P channel: at 6 V the deck asks 0.55 µm of one and 0.7 of an N, so
                // this fixture can bend 0.7 without breaking the channel rule as well.
                v.push(rect(nwell, o - 0.5, o - 0.5, o + 3.5, o + 3.5));
                v.push(rect(pplus, o - 0.2, o - 0.2, o + 3.2, o + 3.2));
                mv(&mut v);
            }
            write(id, polarity, v);
        }
    }

    // PL.9: one poly both sides of a Dualgate edge, which makes it low and medium voltage
    // at once.
    for (polarity, top) in [("good", o + 3.5), ("bad", o + 1.5)] {
        let v = vec![
            rect(poly2, o, o, o + 0.4, o + 3.0),
            rect(dualgate, o - 1.0, o - 1.0, o + 1.4, top),
        ];
        write("PL.9", polarity, v);
    }

    // PL.11: a 5 V marker on neither a Dualgate nor an OTP marker.
    for (polarity, covered) in [("good", true), ("bad", false)] {
        let mut v = vec![rect(v5, o, o, o + 3.0, o + 3.0)];
        if covered {
            v.push(rect(dualgate, o - 0.5, o - 0.5, o + 3.5, o + 3.5));
        }
        write("PL.11", polarity, v);
    }

    // PL.12: an active under a gate reaching out of the 5 V marker that claims it.
    for (polarity, out) in [("good", 0.5), ("bad", -0.5)] {
        let v = vec![
            rect(comp, o, o, o + 3.0, o + 1.0),
            rect(poly2, o + 1.0, o - 0.4, o + 1.8, o + 1.4),
            rect(v5, o - out, o - 0.5, o + 3.0 + out, o + 1.5),
            rect(dualgate, o - 1.0, o - 1.0, o + 4.0, o + 2.0),
        ];
        write("PL.12", polarity, v);
    }

    // PL.5's touching case, as its own pair.
    write_gz(
        &format!("{DIR}/PL.5.good.gds.gz"),
        library("TOP", split_corner(comp, poly2, o, o)),
    );
    write_gz(
        &format!("{DIR}/PL.5.bad.gds.gz"),
        library("TOP", kissing_corner(comp, poly2, o, o)),
    );

    // Bad: an L whose elbow sits well inside a COMP island. Two of its six right angles
    // have their probe square inside the COMP - the outer corner of the elbow and the
    // inner one - and the remaining four are out beyond the COMP's edges.
    let bad = vec![
        rect(comp, o, o, o + 4.0, o + 4.0),
        ell(poly2, o + 1.0, o + 1.0),
    ];
    write_gz(&format!("{DIR}/PL.6.bad.gds.gz"), library("TOP", bad));

    // Good, in three parts.
    let mut good = vec![
        // 1. A right-angle bend clear of any COMP.
        ell(poly2, o + 20.0, o),
        // 2. A poly crossing COMP with 45° turns only, its right-angled ends outside.
        rect(comp, o, o, o + 4.0, o + 4.0),
        poly(
            poly2,
            &[
                (o - 2.0, o + 1.0),
                (o + 1.0, o + 1.0),
                (o + 3.0, o + 3.0),
                (o + 6.0, o + 3.0),
                (o + 6.0, o + 3.5),
                (o + 2.8, o + 3.5),
                (o + 0.8, o + 1.5),
                (o - 2.0, o + 1.5),
            ],
        ),
    ];
    // 3. A right-angle bend on COMP, but inside a YMTP marker, which the rule exempts.
    good.push(rect(comp, o + 10.0, o, o + 14.0, o + 4.0));
    good.push(rect(ymtp, o + 9.0, o - 1.0, o + 15.0, o + 5.0));
    good.push(ell(poly2, o + 12.0, o + 1.0));
    write_gz(&format!("{DIR}/PL.6.good.gds.gz"), library("TOP", good));
}
