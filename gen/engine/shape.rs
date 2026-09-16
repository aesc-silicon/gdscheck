// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the shape family ([`shape`](gdscheck::checks::shape)): each bound of an
//! extent and of an edge length met exactly and missed by five nanometres, a 45° edge
//! straddling the length bound by the nearest grid offsets, and the corner, angle, hole
//! and vertex rules met and not met.

use crate::engine::space::bar45;
use crate::helpers::{layer, library, poly, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/shape";

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // The short side of a bar: 0.5 and 0.495 against E.min_dim, 2.0 and 2.005 against
    // E.max_dim.  0, 1, 0, 1.
    for (name, w) in [
        ("dim_exact", 0.5),
        ("dim_under", 0.495),
        ("dim_max_exact", 2.0),
        ("dim_max_over", 2.005),
    ] {
        write(name, vec![rect(outer, 30.0, 30.0, 33.0, 30.0 + w)]);
    }

    // The long side of a 1 µm bar: 5.0 and 4.995 against E.min_len, 20 and 20.005
    // against E.max_len.  0, 1, 0, 1.
    for (name, l) in [
        ("len_exact", 5.0),
        ("len_under", 4.995),
        ("len_max_exact", 20.0),
        ("len_max_over", 20.005),
    ] {
        write(name, vec![rect(inner, 30.0, 30.0, 30.0 + l, 31.0)]);
    }

    // A via bar exactly 0.3 by 1.0, then 0.305 by 1.0, then 0.3 by 1.005.  E.exact_dim:
    // 0, 1, 0; E.exact_len: 0, 0, 1.
    for (name, w, l) in [
        ("exact_ok", 0.3, 1.0),
        ("exact_off_dim", 0.305, 1.0),
        ("exact_off_len", 0.3, 1.005),
    ] {
        write(name, vec![rect(via, 30.0, 30.0, 30.0 + w, 30.0 + l)]);
    }

    // A 3 µm square of Inner with a corner cut out 1 µm tall and 0.5 wide, then 0.495:
    // the cut's top edge is the one short edge.  L.min: 0 and 1.  The same shape on
    // Outer, read through its edge layer: L.edge: 1 for the 0.495 cut.
    let cut = |l: (i16, i16), w: f64| {
        poly(
            l,
            &[
                (30.0, 30.0),
                (33.0, 30.0),
                (33.0, 32.0),
                (33.0 - w, 32.0),
                (33.0 - w, 33.0),
                (30.0, 33.0),
            ],
        )
    };
    write("edge_exact", vec![cut(inner, 0.5)]);
    write("edge_under", vec![cut(inner, 0.495)]);
    write("edge_layer_under", vec![cut(outer, 0.495)]);

    // A chamfered corner: a 45° edge of length c·√2, 0.5006 for c = 0.354 and 0.4992
    // for c = 0.353, the nearest the grid comes either side of 0.5.  L.min: 0 and 1.
    // The chamfer is one non-orthogonal edge: A.ortho: 1 and 1.
    for (name, c) in [("edge45_over", 0.354), ("edge45_under", 0.353)] {
        write(
            name,
            vec![poly(
                inner,
                &[
                    (30.0, 30.0),
                    (33.0, 30.0),
                    (33.0, 33.0 - c),
                    (33.0 - c, 33.0),
                    (30.0, 33.0),
                ],
            )],
        );
    }

    // A square of Outer turns four right angles; an octagon turns eight at 135°, and
    // has eight vertices, exactly V.max's limit.  C.right: 4 and 0; V.max: 0 and 0;
    // H.none: 0.  A 9-gon is one vertex over: V.max: 1.
    write("corner_square", vec![rect(outer, 30.0, 30.0, 33.0, 33.0)]);
    let octagon = poly(
        outer,
        &[
            (31.0, 30.0),
            (32.0, 30.0),
            (33.0, 31.0),
            (33.0, 32.0),
            (32.0, 33.0),
            (31.0, 33.0),
            (30.0, 32.0),
            (30.0, 31.0),
        ],
    );
    write("corner_octagon", vec![octagon]);
    write(
        "vert_over",
        vec![poly(
            outer,
            &[
                (31.0, 30.0),
                (32.0, 30.0),
                (33.0, 31.0),
                (33.0, 32.0),
                (32.5, 32.5),
                (32.0, 33.0),
                (31.0, 33.0),
                (30.0, 32.0),
                (30.0, 31.0),
            ],
        )],
    );

    // A right triangle of Inner turns 90° at one vertex and 135° at the two others,
    // which are sharper than a right angle; a rectangle turns 90° four times, exactly
    // on C.sharp's lower bound and not over it.  C.sharp: 2 and 0.  A.ortho: 1 and 0,
    // the hypotenuse being the one edge off the axes.
    write(
        "sharp_triangle",
        vec![poly(inner, &[(30.0, 30.0), (33.0, 30.0), (30.0, 33.0)])],
    );
    write("sharp_rect", vec![rect(inner, 30.0, 30.0, 33.0, 31.0)]);

    // A ring of Outer drawn as four bars has a hole; the ring with a square filling the
    // hole has none, the merge closing it.  H.none: 1 and 0.
    let ring = || {
        vec![
            rect(outer, 30.0, 30.0, 34.0, 31.0),
            rect(outer, 30.0, 33.0, 34.0, 34.0),
            rect(outer, 30.0, 31.0, 31.0, 33.0),
            rect(outer, 33.0, 31.0, 34.0, 33.0),
        ]
    };
    write("hole_ring", ring());
    let mut v = ring();
    v.push(rect(outer, 31.0, 31.0, 33.0, 33.0));
    write("hole_filled", v);

    // A 45° bar of Via has two walls at 45° and two at 135°; A.bent45 forbids 45° alone.
    // 2.  An axis-aligned via: 0.
    write("via45", vec![bar45(via, 30.0, 30.0, 3.0, 0.6)]);
    write("via_rect", vec![rect(via, 30.0, 30.0, 30.3, 30.3)]);
}
