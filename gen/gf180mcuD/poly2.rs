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
//! Two of the width rules read the gate between chosen walls rather than the poly whole:
//! PL.2 measures between the gate's walls that lie inside the active, which is the
//! channel, and PL.7 only between the ones running at 45°.  PL.7's fixtures are therefore
//! the only diagonal ones here, and its medium-voltage device is a P-channel: at 6 V the
//! deck asks 0.7 µm of an N channel and 0.55 of a P one, and a fixture bending PL.7's
//! 0.7 µm limit would break the N-channel rule on the way past.
//!
//! `PL.6`, "90 degree bends on the COMP are not allowed", is a rule about a *vertex*, not an edge,
//! and its good pattern has to say so from several directions at once: a bend is fine off
//! the active area, a 45° turn is fine on it, and a right angle is fine anywhere inside a
//! YMTP marker. All three are drawn here, so a check that reduced to "any bend" or "any
//! bend on COMP" would fail rather than pass.

use super::OFFSET;
use crate::helpers::{chamfered_tr, layer, library, poly, rect, write_gz};
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

    // PL.7: the same channel measured only where the gate runs at 45°.  The bar starts
    // 0.65 in, so its right wall passes the active's lower-right corner by 0.25 - an
    // end cap PL.4 reads on the slanted wall, and 0.20 was short of it.
    for (id, w, thick) in [("PL.7_LV", 0.295, false), ("PL.7_MV", 0.695, true)] {
        for (polarity, w) in [("good", 1.2), ("bad", w)] {
            let mut v = vec![rect(comp, o, o, o + 3.0, o + 3.0)];
            v.push(diagonal(poly2, o + 0.65, o - 1.0, w, 6.0));
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

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------------------
//
// Layouts drawn from the manual's section 7.7 by someone who has not seen the engine: the
// deck's own conditions - the voltage split off the Dualgate and V5_XTOR markers, the fuse
// marker, the gate against the field poly, the derivations - at the bound and a step past
// it, and where a tile line or a 45° wall could change the deck's reading.  The generic
// classes (the bound, the metrics, unions, arrays) are the engine family's.  Expected
// values and the reasoning are in hardening/reports/gf180mcuD/poly2.md.

/// Layers the hardening patterns draw on.
struct L {
    comp: (i16, i16),
    poly: (i16, i16),
    dg: (i16, i16),
    v5: (i16, i16),
    fuse: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    nwell: (i16, i16),
    ymtp: (i16, i16),
    otp: (i16, i16),
    sram: (i16, i16),
    res: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            comp: layer(pdk, "comp"),
            poly: layer(pdk, "poly2_drawn"),
            dg: layer(pdk, "dualgate"),
            v5: layer(pdk, "v5_xtor"),
            fuse: layer(pdk, "plfuse"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            nwell: layer(pdk, "nwell"),
            ymtp: layer(pdk, "ymtp_mk"),
            otp: layer(pdk, "otp_mk"),
            sram: layer(pdk, "sramcore"),
            res: layer(pdk, "res_mk"),
        }
    }
}

fn write_h(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A vertical gate: the active `(x0, y0)-(x1, y1)` and a poly `len` wide at `gx` crossing
/// it with `cap` past both edges.
fn xtor(l: &L, comp: (f64, f64, f64, f64), gx: f64, len: f64, cap: f64) -> Vec<GdsElement> {
    let (x0, y0, x1, y1) = comp;
    vec![
        rect(l.comp, x0, y0, x1, y1),
        rect(l.poly, gx, y0 - cap, gx + len, y1 + cap),
    ]
}

/// A 45° bar with horizontal ends `ends` long, running up-right from `(x, y)` by `len`
/// in x and y (`ul`: up-left).  Its perpendicular width is `ends / √2`, and every vertex
/// is on the grid, which a bar with perpendicular ends cannot be; the acute tips are
/// PL.1 widths of their own.
fn bar45(l: (i16, i16), x: f64, y: f64, ends: f64, len: f64, ul: bool) -> GdsElement {
    let dx = if ul { -len } else { len };
    poly(
        l,
        &[
            (x, y),
            (x + ends, y),
            (x + ends + dx, y + len),
            (x + dx, y + len),
        ],
    )
}

/// A small L, 0.5 wide: the bottom arm from `x - 3.5` to `x + 1`, the vertical arm from
/// `y` to `y + 4`.  Its convex elbow is at `(x + 1, y)`, its concave one at
/// `(x + 0.5, y + 0.5)`; the four other corners lie 3.5 and 4 away, off the active.
fn ell2(l: (i16, i16), x: f64, y: f64) -> GdsElement {
    poly(
        l,
        &[
            (x - 3.5, y),
            (x + 1.0, y),
            (x + 1.0, y + 4.0),
            (x + 0.5, y + 4.0),
            (x + 0.5, y + 0.5),
            (x - 3.5, y + 0.5),
        ],
    )
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    pl1_h(&l);
    pl2_h(&l);
    pl3a_h(&l);
    pl4_h(&l);
    pl5_h(&l);
    pl6_h(&l);
    pl7_h(&l);
    pl9_h(&l);
    pl11_h(&l);
    pl12_h(&l);
}

// --- PL.1 / PL.1a: interconnect width 0.18 (3.3 V) / 0.2 (5 V, 6 V) outside PLFUSE, 0.18
// inside it.  The column is the marker's: low voltage is poly no Dualgate touches, medium
// voltage is poly Dualgate overlaps.

fn pl1_h(l: &L) {
    // h1 - the voltage split.  0.175 fires the 3.3 V rule and 0.18 is clean; under a
    // Dualgate 0.195 and the 3.3 V-legal 0.18 fire the 5 V rule and 0.2 is clean.  A
    // 0.175 poly *abutting* the Dualgate edge from outside is a 3.3 V interconnect by the
    // manual (it lies outside the marker) and must fire PL.1_LV; a 0.175 poly crossing
    // the edge is medium voltage and fires PL.1_MV (and PL.9).
    write_h(
        "PL.1.h1",
        vec![
            rect(l.poly, 2.0, 2.0, 2.175, 5.0), // PL.1_LV
            rect(l.poly, 4.0, 2.0, 4.18, 5.0),  // clean
            rect(l.dg, 7.0, 1.0, 13.0, 6.0),
            rect(l.poly, 8.0, 2.0, 8.195, 5.0),  // PL.1_MV
            rect(l.poly, 10.0, 2.0, 10.2, 5.0),  // clean
            rect(l.poly, 12.0, 2.0, 12.18, 5.0), // PL.1_MV
            rect(l.dg, 15.0, 1.0, 18.0, 6.0),
            rect(l.poly, 14.825, 2.0, 15.0, 5.0), // abutting: PL.1_LV by the manual
            rect(l.dg, 22.0, 1.0, 25.0, 6.0),
            rect(l.poly, 21.5, 2.0, 23.0, 2.175), // crossing: PL.1_MV (+ PL.9)
        ],
    );

    // h2 - the fuse marker.  Inside PLFUSE the 5 V width is 0.18, so a 0.19 medium-voltage
    // poly is clean inside and fires PL.1_MV outside.  A poly crossing the PLFUSE edge is
    // neither inside nor outside to the runset's whole-region selectors; the manual reads
    // its outside part as an interconnect, so a 0.19 medium-voltage poly and a 0.175
    // low-voltage one crossing the edge fire PL.1_MV and PL.1_LV.  A poly abutting the
    // marker from outside is outside.
    write_h(
        "PL.1.h2",
        vec![
            rect(l.fuse, 2.0, 1.0, 4.0, 6.0),
            rect(l.poly, 2.5, 2.0, 2.675, 5.0), // PL.1a_LV
            rect(l.poly, 3.5, 2.0, 3.68, 5.0),  // clean
            rect(l.dg, 6.0, 0.5, 13.0, 6.5),
            rect(l.fuse, 7.0, 1.0, 9.0, 6.0),
            rect(l.poly, 7.5, 2.0, 7.69, 5.0), // clean: 0.19 inside the fuse at 5 V
            rect(l.poly, 8.5, 2.0, 8.675, 5.0), // PL.1a_MV
            rect(l.poly, 11.0, 2.0, 11.19, 5.0), // PL.1_MV: 0.19 outside the fuse at 5 V
            rect(l.dg, 15.0, 0.5, 19.5, 6.5),
            rect(l.fuse, 15.5, 3.0, 19.0, 6.0),
            rect(l.poly, 16.0, 2.0, 16.19, 5.0), // crossing the fuse edge: PL.1_MV
            rect(l.fuse, 22.0, 3.0, 25.0, 6.0),
            rect(l.poly, 23.0, 2.0, 23.175, 5.0), // crossing the fuse edge: PL.1_LV
            rect(l.fuse, 27.0, 1.0, 29.0, 6.0),
            rect(l.poly, 26.825, 2.0, 27.0, 5.0), // abutting the fuse: PL.1_LV
        ],
    );

    // h3 - the YMTP marker is cut out of the poly before the width is read.  A 0.175 poly
    // running out of the marker fires on its outside part.  A 0.4 poly ending 0.1 past
    // the marker's edge is 0.4 wide everywhere by the manual: the 0.4 x 0.1 piece the cut
    // leaves is no interconnect and must not fire.
    write_h(
        "PL.1.h3",
        vec![
            rect(l.ymtp, 2.0, 1.0, 5.0, 6.0),
            rect(l.poly, 3.0, 2.0, 3.175, 8.0), // PL.1_LV on the part above y = 6
            rect(l.ymtp, 8.0, 1.0, 11.0, 6.0),
            rect(l.poly, 9.0, 2.0, 9.4, 6.1), // clean: 0.4 wide, cut at y = 6
        ],
    );
}

// --- PL.2: gate width (channel length) 0.28 at 3.3 V; 0.6 / 0.5 for the 5 V N / P channel
// and 0.7 / 0.55 for the 6 V one.  Read between the gate's walls lying on the active.

fn pl2_h(l: &L) {
    // h1 - the 3.3 V bound and shapes.  0.28 is clean, 0.275 fires (two walls); a
    // horizontal gate; one poly over two actives is two gates; a poly 0.5 wide that
    // narrows to 0.275 only over the active (a bite in its wall - PL.6 corners, ignored)
    // is a 0.275 channel.  Then the tile lines: 0.275 gates with a wall on x = 20 and
    // x = 21, straddling x = 20, 40 and 42, and clear of them.
    let mut v = vec![];
    v.extend(xtor(l, (2.0, 2.0, 5.0, 3.0), 3.0, 0.28, 0.5)); // clean
    v.extend(xtor(l, (6.0, 2.0, 9.0, 3.0), 7.0, 0.275, 0.5)); // PL.2_LV x2
    v.push(rect(l.comp, 10.0, 2.0, 11.0, 5.0));
    v.push(rect(l.poly, 9.5, 3.0, 11.5, 3.275)); // PL.2_LV x2, horizontal
    v.push(rect(l.comp, 12.0, 2.0, 14.0, 3.0));
    v.push(rect(l.comp, 12.0, 4.0, 14.0, 5.0));
    v.push(rect(l.poly, 13.0, 1.5, 13.275, 5.5)); // two gates: PL.2_LV x4
    v.push(rect(l.comp, 15.0, 2.0, 17.0, 3.0));
    v.push(poly(
        l.poly,
        &[
            (15.6, 1.5),
            (16.1, 1.5),
            (16.1, 2.3),
            (15.875, 2.3),
            (15.875, 2.7),
            (16.1, 2.7),
            (16.1, 3.5),
            (15.6, 3.5),
        ],
    )); // PL.2_LV x2 on the bite
    v.push(rect(l.comp, 18.0, 2.0, 23.0, 3.0));
    v.push(rect(l.poly, 18.5, 1.5, 18.775, 3.5)); // x2, inside the tile
    v.push(rect(l.poly, 19.9, 1.5, 20.175, 3.5)); // x2, straddling x = 20
    v.push(rect(l.poly, 21.0, 1.5, 21.275, 3.5)); // x2, wall on x = 21
    v.push(rect(l.poly, 22.0, 1.5, 22.28, 3.5)); // clean
    v.push(rect(l.comp, 39.0, 2.0, 44.0, 3.0));
    v.push(rect(l.poly, 39.9, 1.5, 40.175, 3.5)); // x2, straddling x = 40
    v.push(rect(l.poly, 41.865, 1.5, 42.14, 3.5)); // x2, straddling x = 42
    v.push(rect(l.poly, 43.0, 1.5, 43.275, 3.5)); // x2
    write_h("PL.2.h1", v);

    // h2 - the four medium-voltage classes under one Dualgate: the N channel (N+ on a
    // bare substrate) and the P channel (P+ in an N-well), each at 6 V and, under a
    // V5_XTOR that covers the whole active, at 5 V, each at the bound and one step under
    // it.  A gate with no implant is no device and no class; N+ in an N-well is a tap.
    let mut v = vec![rect(l.dg, 0.0, -1.0, 43.0, 6.0)];
    let mut dev = |x: f64, len: f64, kind: &str| {
        v.extend(xtor(l, (x, 2.0, x + 3.0, 3.0), x + 1.0, len, 0.5));
        match kind {
            "n" => v.push(rect(l.nplus, x - 0.2, 1.8, x + 3.2, 3.2)),
            "p" => {
                v.push(rect(l.nwell, x - 0.5, 1.5, x + 3.5, 3.5));
                v.push(rect(l.pplus, x - 0.2, 1.8, x + 3.2, 3.2));
            }
            "ntap" => {
                v.push(rect(l.nwell, x - 0.5, 1.5, x + 3.5, 3.5));
                v.push(rect(l.nplus, x - 0.2, 1.8, x + 3.2, 3.2));
            }
            _ => {}
        }
    };
    dev(2.0, 0.695, "n"); // PL.2_MV x2 (6 V N: 0.7)
    dev(6.0, 0.7, "n"); // clean
    dev(10.0, 0.545, "p"); // PL.2_MV x2 (6 V P: 0.55)
    dev(14.0, 0.55, "p"); // clean
    dev(18.0, 0.595, "n"); // PL.2_MV x2 (5 V N: 0.6), straddling x = 20
    dev(22.0, 0.6, "n"); // clean
    dev(26.0, 0.495, "p"); // PL.2_MV x2 (5 V P: 0.5)
    dev(30.0, 0.5, "p"); // clean
    dev(34.0, 0.275, ""); // no implant: no device
    dev(38.0, 0.275, "ntap"); // a tap, no device
    v.push(rect(l.v5, 17.5, 1.3, 33.5, 3.7));
    write_h("PL.2.h2", v);

    // h3 - a gate whose poly abuts the Dualgate: its end lies on the marker's edge, and a
    // second one touches the marker's corner at a point.  The poly lies outside the
    // marker, so it is a 3.3 V gate by the manual and 0.275 fires PL.2_LV; to the runset
    // it is neither low voltage (it touches the marker) nor medium (it does not overlap
    // it) and no rule reads it.
    let mut v = vec![];
    v.extend(xtor(l, (2.0, 2.0, 5.0, 3.0), 3.0, 0.275, 0.5));
    v.push(rect(l.poly, 3.0, 3.5, 3.275, 6.0)); // the same poly, on up to the marker
    v.push(rect(l.dg, 2.0, 6.0, 6.0, 9.0)); // its bottom edge on the poly's end
    v.extend(xtor(l, (8.0, 2.0, 11.0, 3.0), 9.0, 0.275, 0.5));
    v.push(rect(l.poly, 9.0, 3.5, 9.275, 6.0));
    v.push(rect(l.dg, 9.275, 6.0, 13.0, 9.0)); // its corner on the poly's corner
    write_h("PL.2.h3", v);

    // h4 - 45° gates.  The channel is read between the slanted walls too: 0.272 (ends of
    // 0.385) fires PL.2_LV (and PL.7_LV, which asks 0.3 of a bent gate and is ignored),
    // 0.286 (0.405) is clean for PL.2; the same 0.272 bar across x = 20 and x = 21, and
    // one running the other way.  The bars' acute tips are PL.1 widths (ignored).
    let v = vec![
        rect(l.comp, 2.0, 2.0, 5.0, 5.0),
        bar45(l.poly, 2.65, 1.0, 0.385, 5.0, false), // PL.2_LV x2
        rect(l.comp, 8.0, 2.0, 11.0, 5.0),
        bar45(l.poly, 8.65, 1.0, 0.405, 5.0, false), // clean (PL.7_LV ignored)
        rect(l.comp, 18.5, 2.0, 21.5, 5.0),
        bar45(l.poly, 19.15, 1.0, 0.385, 5.0, false), // PL.2_LV x2 across x = 20, 21
        rect(l.comp, 26.0, 2.0, 29.0, 5.0),
        bar45(l.poly, 27.965, 1.0, 0.385, 5.0, true), // PL.2_LV x2, up-left
    ];
    write_h("PL.2.h4", v);
}

// --- PL.3a: poly space 0.24, on the active and on the field.  The deck's layer is the
// gate pieces plus the field pieces, which must read as one poly again.

fn pl3a_h(l: &L) {
    // Two gates on one active 0.235 apart.
    let mut v = vec![
        rect(l.comp, 2.0, 2.0, 6.0, 3.0),
        rect(l.poly, 3.0, 1.5, 3.4, 3.5),
    ];
    v.push(rect(l.poly, 3.635, 1.5, 4.035, 3.5)); // PL.3a
    // A gate's on-active part against a field poly beside the active's edge.
    v.push(rect(l.comp, 8.0, 2.0, 9.5, 3.0));
    v.push(rect(l.poly, 9.0, 1.5, 9.4, 3.5));
    v.push(rect(l.poly, 9.635, 2.0, 10.035, 3.0)); // PL.3a
    // A poly crossing a 0.23 wide active under RES_MK: the runset drops the on-active
    // part of the poly (tgate less res_mk) and reads its two field pieces 0.23 apart.
    // It is one poly by the manual, clean.
    v.push(rect(l.res, 11.8, 1.8, 15.2, 2.43));
    v.push(rect(l.comp, 12.0, 2.0, 15.0, 2.23));
    v.push(rect(l.poly, 13.0, 1.5, 13.4, 3.0)); // clean
    // A slot 0.235 wide cut into a gate from below, its end 0.3 inside the active: the
    // notch spans the seam between the field piece and the gate piece.  Its two inner
    // corners are PL.6 corners (ignored).
    v.push(rect(l.comp, 17.0, 2.0, 19.0, 3.0));
    v.push(poly(
        l.poly,
        &[
            (17.5, 1.5),
            (17.9, 1.5),
            (17.9, 2.3),
            (18.135, 2.3),
            (18.135, 1.5),
            (18.5, 1.5),
            (18.5, 3.5),
            (17.5, 3.5),
        ],
    )); // PL.3a (notch)
    // The tile lines: a 0.235 gap straddling x = 20, one starting on x = 40, and a 0.24
    // gap starting on x = 42.
    v.push(rect(l.poly, 19.4, 5.0, 19.8, 7.0));
    v.push(rect(l.poly, 20.035, 5.0, 20.435, 7.0)); // PL.3a
    v.push(rect(l.poly, 39.6, 5.0, 40.0, 7.0));
    v.push(rect(l.poly, 40.235, 5.0, 40.635, 7.0)); // PL.3a
    v.push(rect(l.poly, 41.6, 5.0, 42.0, 7.0));
    v.push(rect(l.poly, 42.24, 5.0, 42.64, 7.0)); // clean
    write_h("PL.3a.h1", v);
}

// --- PL.4: the gate's extension beyond the active (the end cap), 0.22 at every voltage.

fn pl4_h(l: &L) {
    // h1 - the bound and the column.  A 0.215 cap at the bottom, at both ends (two), on a
    // horizontal gate's left; 0.22 clean.  Under a Dualgate the same cap is PL.4_MV, and
    // it is still PL.4_MV under an SRAM core marker: the manual's 7.7 exempts nothing
    // there (the runset checks it, the deck's `poly_pl_mv` drops the SRAM core).
    let v = vec![
        rect(l.comp, 2.0, 2.0, 5.0, 3.0),
        rect(l.poly, 3.0, 1.785, 3.4, 3.5), // PL.4_LV
        rect(l.comp, 6.0, 2.0, 9.0, 3.0),
        rect(l.poly, 7.0, 1.78, 7.4, 3.5), // clean
        rect(l.comp, 10.0, 2.0, 13.0, 3.0),
        rect(l.poly, 11.0, 1.785, 11.4, 3.215), // PL.4_LV x2
        rect(l.comp, 14.0, 1.0, 15.0, 4.0),
        rect(l.poly, 13.785, 2.0, 15.5, 2.4), // PL.4_LV
        rect(l.dg, 16.5, 0.0, 19.5, 5.0),
        rect(l.comp, 17.0, 2.0, 19.0, 3.0),
        rect(l.poly, 18.0, 1.785, 18.4, 3.5), // PL.4_MV
        rect(l.dg, 22.0, 0.0, 25.0, 5.0),
        rect(l.sram, 22.2, 0.2, 24.8, 4.8),
        rect(l.comp, 22.5, 2.0, 24.5, 3.0),
        rect(l.poly, 23.5, 1.785, 23.9, 3.5), // PL.4_MV, under SRAM core
    ];
    write_h("PL.4.h1", v);

    // h2 - shapes and the tile lines.  A stub ending inside the active and a poly wholly
    // inside it have no cap to read (the IHP round's Gat.c decision: clean, another rule's
    // business; their corners are PL.6's, ignored).  A gate crossing a chamfered corner of
    // the active, its end 0.2157 from the chamfer perpendicular, fires - the closest
    // approach, with no straight active edge under the poly at all; 0.4985 is clean.  A
    // gate cap chamfered at 45° from 0.25 and 0.35 up its wall is clean: the chamfer
    // only recedes from the active, so the cap is the extension at the wall.  Then
    // horizontal gates whose 0.215 cap starts on x = 20, straddles x = 20, starts on
    // x = 40, straddles x = 42; a 0.22 cap starting on x = 42 is clean.
    let v = vec![
        rect(l.comp, 2.0, 2.0, 5.0, 4.0),
        rect(l.poly, 3.0, 1.5, 3.4, 3.0), // stub: clean
        rect(l.comp, 6.0, 2.0, 9.0, 4.0),
        rect(l.poly, 7.0, 2.5, 7.4, 3.5), // wholly inside: clean
        chamfered_tr(l.comp, 10.0, 2.0, 13.0, 3.0, 15.6), // chamfer (13, 2.6)-(12.6, 3)
        rect(l.poly, 12.65, 1.5, 12.95, 3.255), // PL.4_LV: 0.2157 off the chamfer
        chamfered_tr(l.comp, 14.0, 2.0, 17.0, 3.0, 19.6),
        rect(l.poly, 16.65, 1.5, 16.95, 3.655), // clean: 0.4985
        rect(l.comp, 22.0, 2.0, 25.0, 3.0),
        chamfered_tr(l.poly, 23.0, 1.5, 23.5, 3.5, 26.75), // clean: 0.25 at the wall
        rect(l.comp, 26.0, 2.0, 29.0, 3.0),
        chamfered_tr(l.poly, 27.0, 1.5, 27.5, 3.5, 30.85), // clean: 0.35 at the wall
        rect(l.comp, 17.5, 4.0, 20.0, 5.0),
        rect(l.poly, 16.5, 4.3, 20.215, 4.7), // PL.4_LV, cap from x = 20
        rect(l.comp, 17.5, 6.0, 19.9, 7.0),
        rect(l.poly, 16.5, 6.3, 20.115, 6.7), // PL.4_LV, cap across x = 20
        rect(l.comp, 38.0, 2.0, 40.0, 3.0),
        rect(l.poly, 37.0, 2.3, 40.215, 2.7), // PL.4_LV, cap from x = 40
        rect(l.comp, 40.5, 4.0, 42.0, 5.0),
        rect(l.poly, 39.5, 4.3, 42.22, 4.7), // clean, cap from x = 42
        rect(l.comp, 40.5, 6.0, 41.9, 7.0),
        rect(l.poly, 39.5, 6.3, 42.115, 6.7), // PL.4_LV, cap across x = 42
    ];
    write_h("PL.4.h2", v);

    // h3 - a 0.215 cap on a gate whose poly abuts the Dualgate: a 3.3 V gate by the
    // manual, PL.4_LV; no rule of the runset reads it (see PL.2.h3).
    let v = vec![
        rect(l.comp, 2.0, 2.0, 5.0, 3.0),
        rect(l.poly, 3.0, 1.785, 3.4, 4.0), // PL.4_LV
        rect(l.dg, 2.0, 4.0, 6.0, 8.0),
    ];
    write_h("PL.4.h3", v);
}

// --- PL.5a / PL.5b: field poly to active 0.1 at 3.3 V, 0.3 at 5 V / 6 V; the deck reads
// both ids as one measurement, so every pair fires under both.

fn pl5_h(l: &L) {
    // h1 - the bound, the column and the related active.  0.095 fires (a and b), 0.1 is
    // clean; under a Dualgate 0.295 fires, 0.3 is clean, and so does the 3.3 V-legal 0.2.
    // A gate's own poly running 0.095 beside an arm of the active it crosses fires (the
    // related active of PL.5b), drawn as an L and as a U.  A corner-to-corner 0.07/0.07 (0.099) fires and
    // 0.075/0.075 (0.106) is clean.  Then the tile lines: a 0.095 gap starting on x = 20,
    // straddling x = 20 and x = 42; a 0.1 gap starting on x = 42 is clean.
    let mut v = vec![
        rect(l.comp, 2.0, 2.0, 4.0, 4.0),
        rect(l.poly, 4.095, 2.0, 4.495, 4.0), // PL.5a_LV, PL.5b_LV
        rect(l.comp, 6.0, 2.0, 8.0, 4.0),
        rect(l.poly, 8.1, 2.0, 8.5, 4.0), // clean
        rect(l.dg, 10.0, 1.0, 15.0, 5.0),
        rect(l.comp, 10.5, 2.0, 12.5, 4.0),
        rect(l.poly, 12.795, 2.0, 13.195, 4.0), // PL.5a_MV, PL.5b_MV
        rect(l.dg, 16.0, 1.0, 19.8, 5.0),
        rect(l.comp, 16.5, 2.0, 18.0, 4.0),
        rect(l.poly, 18.3, 2.0, 18.7, 4.0), // clean
        rect(l.dg, 22.0, 1.0, 26.0, 5.0),
        rect(l.comp, 22.5, 2.0, 24.0, 4.0),
        rect(l.poly, 24.2, 2.0, 24.6, 4.0), // PL.5a_MV, PL.5b_MV: 0.2 at 5 V
    ];
    v.push(poly(
        l.comp,
        &[
            (28.0, 2.0),
            (31.0, 2.0),
            (31.0, 2.5),
            (29.905, 2.5),
            (29.905, 4.0),
            (28.0, 4.0),
        ],
    ));
    v.push(rect(l.poly, 30.0, 1.5, 30.4, 4.5)); // PL.5a_LV, PL.5b_LV beside the arm
    v.push(poly(
        l.comp,
        &[
            (44.0, 2.0),
            (49.0, 2.0),
            (49.0, 4.0),
            (48.5, 4.0),
            (48.5, 2.5),
            (45.905, 2.5),
            (45.905, 4.0),
            (44.0, 4.0),
        ],
    ));
    v.push(rect(l.poly, 46.0, 1.5, 46.4, 4.5)); // PL.5a_LV, PL.5b_LV: a U's left arm
    v.push(rect(l.comp, 33.0, 2.0, 35.0, 4.0));
    v.push(rect(l.poly, 35.07, 4.07, 35.5, 5.0)); // PL.5a_LV, PL.5b_LV: 0.099
    v.push(rect(l.comp, 37.0, 2.0, 39.0, 4.0));
    v.push(rect(l.poly, 39.075, 4.075, 39.5, 5.0)); // clean: 0.106
    v.push(rect(l.comp, 18.0, 6.0, 20.0, 8.0));
    v.push(rect(l.poly, 20.095, 6.0, 20.5, 8.0)); // x2, gap from x = 20
    v.push(rect(l.comp, 18.0, 9.0, 19.95, 11.0));
    v.push(rect(l.poly, 20.045, 9.0, 20.5, 11.0)); // x2, gap across x = 20
    v.push(rect(l.comp, 40.0, 6.0, 42.0, 8.0));
    v.push(rect(l.poly, 42.1, 6.0, 42.5, 8.0)); // clean, gap from x = 42
    v.push(rect(l.comp, 40.0, 9.0, 41.95, 11.0));
    v.push(rect(l.poly, 42.045, 9.0, 42.5, 11.0)); // x2, gap across x = 42
    write_h("PL.5.h1", v);

    // h2 - 45°: a poly band parallel to a chamfered active corner, 0.092 away
    // perpendicular (0.13 along x + y) fires; 0.1025 (0.145) is clean.
    let mut v = vec![chamfered_tr(l.comp, 2.0, 2.0, 5.0, 5.0, 9.0)];
    v.push(poly(
        l.poly,
        &[(4.13, 5.0), (5.13, 4.0), (5.53, 4.0), (4.53, 5.0)],
    )); // PL.5a_LV, PL.5b_LV
    v.push(chamfered_tr(l.comp, 8.0, 2.0, 11.0, 5.0, 15.0));
    v.push(poly(
        l.poly,
        &[(10.145, 5.0), (11.145, 4.0), (11.545, 4.0), (10.545, 5.0)],
    )); // clean
    write_h("PL.5.h2", v);

    // h3 - a field poly 0.095 beside an active, its end on the Dualgate's edge: 3.3 V by
    // the manual, PL.5a_LV and PL.5b_LV; the runset reads it in no column (PL.2.h3).
    let v = vec![
        rect(l.comp, 2.0, 2.0, 4.0, 4.0),
        rect(l.poly, 4.095, 2.0, 4.495, 5.0), // PL.5a_LV, PL.5b_LV
        rect(l.dg, 2.0, 5.0, 6.0, 9.0),
    ];
    write_h("PL.5.h3", v);
}

// --- PL.6: 90° bends of the poly on the active are not allowed.  A rule about a vertex:
// every right-angle corner of the poly lying on the active fires, convex or concave.

fn pl6_h(l: &L) {
    // h1 - which corners are on the active.  An L's elbow inside (two corners); the elbow
    // 0.05 from the active's edge (two: the runset only sees a corner whose 0.2 square
    // lies wholly inside the active); the convex corner 0.005 outside (one, the concave
    // one); a T stub ending inside (four); a step in the poly's width (two); the elbow
    // inside a YMTP marker (none); the elbow in the hole of an active ring drawn as four
    // boxes (none); the elbow on the seam of two abutting active boxes (two); an L drawn
    // as two overlapping boxes (two - the union's corners, not the boxes').
    let mut v = vec![
        rect(l.comp, 2.0, 2.0, 6.0, 6.0),
        ell2(l.poly, 3.5, 3.0), // 2: (4.5, 3), (4, 3.5)
        rect(l.comp, 8.0, 2.0, 12.0, 6.0),
        ell2(l.poly, 10.95, 3.0), // 2: (11.95, 3), (11.45, 3.5)
        rect(l.comp, 14.0, 2.0, 18.0, 6.0),
        ell2(l.poly, 17.005, 3.0), // 1: (17.505, 3.5); (18.005, 3) is outside
        rect(l.comp, 22.0, 2.0, 26.0, 6.0),
        rect(l.poly, 23.0, 1.0, 23.5, 7.0),
        rect(l.poly, 23.5, 3.5, 25.0, 4.0), // 4: the stub's corners
        rect(l.comp, 28.0, 2.0, 32.0, 6.0),
    ];
    v.push(poly(
        l.poly,
        &[
            (29.0, 1.0),
            (29.5, 1.0),
            (29.5, 4.0),
            (30.0, 4.0),
            (30.0, 7.0),
            (29.0, 7.0),
        ],
    )); // 2: (29.5, 4), (30, 4)
    v.push(rect(l.comp, 34.0, 2.0, 38.0, 6.0));
    v.push(rect(l.ymtp, 35.0, 3.0, 38.5, 6.5));
    v.push(ell2(l.poly, 36.5, 4.0)); // 0: inside the YMTP marker
    v.push(rect(l.comp, 44.5, 2.0, 46.0, 8.0)); // a ring, four boxes
    v.push(rect(l.comp, 49.0, 2.0, 50.5, 8.0));
    v.push(rect(l.comp, 46.0, 2.0, 49.0, 3.5));
    v.push(rect(l.comp, 46.0, 6.5, 49.0, 8.0));
    v.push(ell2(l.poly, 46.5, 4.5)); // 0: (47.5, 4.5), (47, 5) in the hole
    v.push(rect(l.comp, 56.0, 2.0, 58.0, 6.0));
    v.push(rect(l.comp, 58.0, 2.0, 60.0, 6.0));
    v.push(ell2(l.poly, 57.0, 3.0)); // 2: (58, 3) on the seam, (57.5, 3.5)
    v.push(rect(l.comp, 62.0, 2.0, 66.0, 6.0));
    v.push(rect(l.poly, 61.0, 3.0, 65.0, 3.5));
    v.push(rect(l.poly, 64.5, 3.0, 65.0, 8.0)); // 2: (65, 3), (64.5, 3.5)
    write_h("PL.6.h1", v);

    // h2 - the tile lines.  The convex elbow on x = 20, 0.005 either side of it, on
    // x = 40 and on (40, 40); the concave one on x = 21 and on (21, 21) and x = 42.  Two
    // each.
    let mut v = vec![];
    let mut at = |x: f64, y: f64| {
        v.push(rect(l.comp, x - 0.5, y - 0.5, x + 1.5, y + 1.5));
        v.push(ell2(l.poly, x, y));
    };
    at(19.0, 2.5); // (20, 2.5), (19.5, 3)
    at(19.005, 8.5); // (20.005, 8.5), (19.505, 9)
    at(18.995, 14.5); // (19.995, 14.5), (19.495, 15)
    at(20.5, 20.5); // (21.5, 20.5), (21, 21)
    at(39.0, 2.5); // (40, 2.5), (39.5, 3)
    at(41.5, 8.5); // (42.5, 8.5), (42, 9)
    at(39.0, 40.0); // (40, 40), (39.5, 40.5)
    write_h("PL.6.h2", v);

    // h3 - the active's edge itself.  A convex elbow exactly on the active's edge is not on
    // the active (one: the concave corner inside); both corners on the edges at the
    // active's corner (none); the elbow 0.1 inside (two, the runset's square just fits)
    // and 0.095 inside (two).
    let v = vec![
        rect(l.comp, 2.0, 2.0, 6.0, 6.0),
        ell2(l.poly, 5.0, 3.0), // 1: (6, 3) on the edge, (5.5, 3.5) inside
        rect(l.comp, 8.0, 2.0, 12.0, 6.0),
        ell2(l.poly, 11.0, 5.5), // 0: (12, 5.5) and (11.5, 6) on the edges
        rect(l.comp, 14.0, 2.0, 18.0, 6.0),
        ell2(l.poly, 16.9, 3.0), // 2: (17.9, 3), (17.4, 3.5)
        rect(l.comp, 22.0, 2.0, 26.0, 6.0),
        ell2(l.poly, 24.905, 3.0), // 2: (25.905, 3), (25.405, 3.5)
    ];
    write_h("PL.6.h3", v);
}

// --- PL.7: the width of a gate running at 45°, 0.3 at 3.3 V and 0.7 at 5 V / 6 V.  Read
// between the gate's slanted walls on the active; the column is the Dualgate's, with no
// implant needed.

fn pl7_h(l: &L) {
    // h1 - bars with horizontal ends (`bar45`): 0.2934 wide (ends of 0.415) fires (two
    // walls), 0.3005 (0.425) is clean; the same 0.2934 across x = 20 and x = 21; under a
    // Dualgate with no implant 0.693 (0.98) fires PL.7_MV and 0.70004 (0.99) is clean; a
    // bar running the other way; an N+ 6 V gate at 0.693 fires PL.7_MV (and PL.2_MV,
    // ignored).  The bars' acute tips are PL.1 widths (ignored).
    let mut v = vec![];
    let mut bar = |x0: f64, ends: f64, mv: bool, ul: bool| {
        if mv {
            v.push(rect(l.dg, x0 - 1.0, 0.0, x0 + 7.5, 7.0));
        }
        v.push(rect(l.comp, x0, 2.0, x0 + 3.0, 5.0));
        if ul {
            v.push(bar45(l.poly, x0 + 2.35 - ends, 1.0, ends, 5.0, true));
        } else {
            v.push(bar45(l.poly, x0 + 0.65, 1.0, ends, 5.0, false));
        }
    };
    bar(2.0, 0.415, false, false); // PL.7_LV x2
    bar(8.0, 0.425, false, false); // clean
    bar(18.5, 0.415, false, false); // PL.7_LV x2 across x = 20, 21
    bar(26.0, 0.98, true, false); // PL.7_MV x2
    bar(36.0, 0.99, true, false); // clean
    bar(47.0, 0.415, false, true); // PL.7_LV x2, up-left
    bar(54.0, 0.98, true, false); // PL.7_MV x2 (+ PL.2_MV)
    v.push(rect(l.nplus, 53.5, 1.5, 57.5, 5.5));
    write_h("PL.7.h1", v);

    // h2 - a 45° gate 0.2934 wide whose top end lies on the Dualgate's edge.  PL.7's
    // layer is the gate *less* the marker, a geometric difference and not a whole-region
    // selector, so the touch does not move this gate out of the 3.3 V column: PL.7_LV
    // fires in both tools, unlike PL.2, PL.4 and PL.5 (PL.2.h3).  The tips are PL.1
    // widths (ignored), though the touch takes the poly out of PL.1's columns as well.
    let v = vec![
        rect(l.comp, 2.0, 2.0, 6.0, 5.0),
        bar45(l.poly, 2.65, 1.0, 0.415, 5.0, false),
        rect(l.dg, 7.0, 6.0, 10.0, 9.0),
    ];
    write_h("PL.7.h2", v);
}

// --- PL.9: a poly interconnect connecting the 3.3 V area and the 5 V / 6 V area (inside
// and outside the Dualgate) is not allowed.

fn pl9_h(l: &L) {
    let v = vec![
        rect(l.dg, 2.0, 2.0, 6.0, 6.0),
        rect(l.poly, 4.0, 1.0, 4.4, 4.0), // PL.9: crossing
        rect(l.poly, 3.0, 3.0, 3.4, 5.0), // clean: inside
        rect(l.poly, 6.0, 3.0, 6.4, 5.0), // clean: outside, abutting the edge
        rect(l.dg, 9.0, 2.0, 13.0, 6.0),
        rect(l.poly, 11.0, 1.0, 11.4, 2.0),
        rect(l.poly, 11.0, 2.0, 11.4, 4.0), // PL.9: two boxes meeting on the edge
        rect(l.dg, 15.0, 2.0, 17.0, 6.0),
        rect(l.dg, 18.0, 2.0, 19.5, 6.0),
        rect(l.poly, 16.0, 3.0, 18.5, 3.4), // PL.9: bridging two markers
        rect(l.dg, 22.0, 2.0, 23.5, 8.0),   // a ring, four boxes
        rect(l.dg, 26.5, 2.0, 28.0, 8.0),
        rect(l.dg, 23.5, 2.0, 26.5, 3.5),
        rect(l.dg, 23.5, 6.5, 26.5, 8.0),
        rect(l.poly, 24.0, 4.0, 24.4, 6.0), // clean: in the hole
        rect(l.poly, 25.5, 4.0, 25.9, 7.5), // PL.9: out of the hole
        rect(l.v5, 30.0, 2.0, 34.0, 6.0),   // V5_XTOR abutting a Dualgate (PL.11 clean)
        rect(l.dg, 34.0, 2.0, 37.0, 6.0),
        rect(l.poly, 31.0, 3.0, 31.4, 5.0), // clean: under V5_XTOR only
        rect(l.poly, 33.0, 3.0, 35.0, 3.4), // clean by the runset: no 3.3 V area
        rect(l.poly, 2.0, 12.0, 302.0, 12.4), // PL.9: 300 µm, into a marker at 250
        rect(l.dg, 250.0, 10.0, 310.0, 14.0),
        rect(l.dg, 20.0, 15.0, 24.0, 19.0),
        rect(l.poly, 19.0, 16.0, 21.0, 16.4), // PL.9: the edge on x = 20
        rect(l.dg, 17.0, 21.0, 21.0, 25.0),
        rect(l.poly, 20.5, 22.0, 22.0, 22.4), // PL.9: the edge on x = 21
        rect(l.dg, 40.0, 15.0, 44.0, 19.0),
        rect(l.poly, 39.0, 16.0, 41.0, 16.4), // PL.9: the edge on x = 40
        rect(l.dg, 38.0, 21.0, 42.0, 25.0),
        rect(l.poly, 41.0, 22.0, 43.0, 22.4), // PL.9: the edge on x = 42
        rect(l.dg, 2.0, 16.0, 6.0, 20.0),
        rect(l.poly, 1.995, 17.0, 4.0, 17.4), // PL.9: 0.005 outside
    ];
    write_h("PL.9.h1", v);
}

// --- PL.11: V5_XTOR must enclose a 5 V device - the runset's reading, a V5_XTOR touching
// neither a Dualgate nor an OTP marker.

fn pl11_h(l: &L) {
    let v = vec![
        rect(l.dg, 2.0, 2.0, 5.0, 5.0),
        rect(l.v5, 2.5, 5.005, 4.5, 7.0), // PL.11: 0.005 off the marker
        rect(l.dg, 7.0, 2.0, 10.0, 5.0),
        rect(l.v5, 7.5, 5.0, 9.5, 7.0), // clean: abutting
        rect(l.dg, 12.0, 2.0, 15.0, 5.0),
        rect(l.v5, 13.0, 4.0, 14.0, 7.0), // clean: overlapping
        rect(l.otp, 17.0, 2.0, 19.5, 5.0),
        rect(l.v5, 17.5, 2.5, 19.0, 4.5), // clean: in an OTP marker
        rect(l.dg, 22.0, 2.0, 25.0, 5.0),
        rect(l.v5, 22.5, 5.0, 24.0, 7.0),
        rect(l.v5, 24.0, 5.5, 26.0, 7.0), // clean: abutting the box that touches
        rect(l.v5, 15.0, 9.0, 25.0, 11.0),
        rect(l.dg, 25.0, 9.0, 30.0, 11.0), // clean: touching across x = 20 at x = 25
        rect(l.v5, 15.0, 13.0, 19.995, 15.0),
        rect(l.dg, 20.0, 13.0, 23.0, 15.0), // PL.11: 0.005 short of x = 20
        rect(l.v5, 1000.0, 1000.0, 1003.0, 1003.0), // PL.11: far
    ];
    write_h("PL.11.h1", v);
}

// --- PL.12: V5_XTOR encloses the 5 V active by 0 - the active under a gate that the
// marker touches must lie wholly inside it.

fn pl12_h(l: &L) {
    let mut v = vec![
        rect(l.dg, 1.0, 0.0, 9.0, 7.0),
        rect(l.v5, 2.0, 1.0, 5.0, 6.0),
    ];
    v.extend(xtor(l, (3.0, 2.0, 7.0, 3.0), 4.0, 0.4, 0.5)); // PL.12: half outside
    v.push(rect(l.dg, 10.0, 0.0, 16.0, 7.0));
    v.push(rect(l.v5, 11.0, 1.0, 15.0, 6.0));
    v.extend(xtor(l, (12.0, 2.0, 14.0, 3.0), 13.0, 0.4, 0.5)); // clean: inside
    v.push(rect(l.dg, 17.0, 0.0, 24.0, 7.0));
    v.push(rect(l.v5, 18.0, 1.0, 19.5, 6.0));
    v.extend(xtor(l, (19.5, 2.0, 22.0, 3.0), 20.5, 0.4, 0.5)); // PL.12: abutting from outside
    v.push(rect(l.dg, 26.0, 0.0, 32.0, 7.0));
    v.push(rect(l.v5, 27.0, 1.0, 29.0, 6.0));
    v.push(rect(l.comp, 28.0, 2.0, 31.0, 3.0)); // clean: no gate
    v.push(rect(l.dg, 34.0, 0.0, 40.0, 7.0));
    v.push(rect(l.v5, 35.0, 1.0, 39.0, 6.0));
    v.extend(xtor(l, (35.0, 2.0, 39.0, 3.0), 37.0, 0.4, 0.5)); // clean: enclosed by 0
    v.push(rect(l.dg, 16.0, 10.0, 26.0, 15.0));
    v.push(rect(l.v5, 17.0, 11.0, 20.0, 14.0));
    v.extend(xtor(l, (18.0, 12.0, 24.0, 13.0), 19.0, 0.4, 0.5)); // PL.12: out past x = 20
    write_h("PL.12.h1", v);
}
