// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the shape family ([`shape`](gdscheck::checks::shape)): each bound of an
//! extent and of an edge length met exactly and missed by five nanometres, a 45° edge
//! straddling the length bound by the nearest grid offsets, and the corner, angle, hole
//! and vertex rules met and not met.

use crate::engine::space::bar45;
use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
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

    // --- The hardening patterns: what a rule manual's extent, edge, corner, angle, hole
    // and vertex rules ask of any layer, drawn once here for every deck of every PDK
    // (hardening/SPEC.md).  An extent is the merged region's bounding box, so a 45°
    // bar's box is not the bar and an L's box is the L's reach: the readings below say
    // so.  A violation of an extent rule is one region; of an edge rule, one segment;
    // of a corner rule, one vertex; of a hole rule, one hole; of the vertex rule, one
    // drawn polygon.

    // The short side (Outer: E.min_dim 0.5, E.max_dim 2.0).  3 × 0.5 and 0.5 × 3 are
    // clean; 3 × 0.495 and 0.495 × 3 fire; 3 × 2.0 is clean, 3 × 2.005 over; a 45°
    // strip 0.495 across has a 3.35 box - clean by the short side, over 2.0 by it; a
    // diamond 0.48 across the box fires, 0.5 is clean; a 1 × 0.5 box with 0.1 chamfers
    // and an L of 0.3 arms reaching 1 µm are clean by their boxes.  E.min_dim: 3;
    // E.max_dim: 2.
    write(
        "dim_bound",
        vec![
            rect(outer, 2.0, 2.0, 5.0, 2.5),      // clean
            rect(outer, 2.0, 4.0, 2.5, 7.0),      // clean
            rect(outer, 6.0, 2.0, 9.0, 2.495),    // E.min_dim
            rect(outer, 6.0, 4.0, 6.495, 7.0),    // E.min_dim
            rect(outer, 10.0, 2.0, 13.0, 4.0),    // clean
            rect(outer, 10.0, 6.0, 13.0, 8.005),  // E.max_dim
            strip45(outer, 14.0, 2.0, 3.0, 0.35), // E.max_dim (box 3.35), not E.min_dim
            diamond(outer, 19.0, 3.0, 0.24),      // E.min_dim (box 0.48)
            diamond(outer, 21.0, 3.0, 0.25),      // clean (box 0.5)
            poly(
                outer,
                &[
                    (23.1, 2.0),
                    (23.9, 2.0),
                    (24.0, 2.1),
                    (24.0, 2.4),
                    (23.9, 2.5),
                    (23.1, 2.5),
                    (23.0, 2.4),
                    (23.0, 2.1),
                ],
            ), // clean (box 1 × 0.5)
            poly(
                outer,
                &[
                    (25.0, 2.0),
                    (26.0, 2.0),
                    (26.0, 2.3),
                    (25.3, 2.3),
                    (25.3, 3.0),
                    (25.0, 3.0),
                ],
            ), // clean (box 1 × 1)
        ],
    );

    // Shapes that merge.  Two abutting 0.3 × 3 boxes are a 0.6 box, clean; two 0.3 × 3
    // boxes overlapping to a union 0.5 wide are clean, to 0.495 they fire; a 0.495 × 3
    // bar drawn as six slices fires once; two 0.3 squares meeting at a corner are one
    // region with a 0.6 box, clean; a plus of two 0.3 × 2.005 bars has a 2.005 box and
    // is over E.max_dim, not under E.min_dim; a 2 × 2 ring with a 0.4 band is clean
    // and the 0.4 square in its hole fires.  E.min_dim: 3; E.max_dim: 1.
    let mut e = vec![
        rect(outer, 2.0, 2.0, 2.3, 5.0),
        rect(outer, 2.3, 2.0, 2.6, 5.0), // clean, 0.6
        rect(outer, 4.0, 2.0, 4.3, 5.0),
        rect(outer, 4.2, 2.0, 4.5, 5.0), // clean, 0.5
        rect(outer, 6.0, 2.0, 6.3, 5.0),
        rect(outer, 6.195, 2.0, 6.495, 5.0), // E.min_dim, 0.495
        rect(outer, 10.0, 2.0, 10.3, 2.3),
        rect(outer, 10.3, 2.3, 10.6, 2.6), // clean, one region 0.6
        rect(outer, 12.0, 2.0, 14.005, 2.3),
        rect(outer, 12.85, 1.15, 13.15, 3.155), // E.max_dim, plus 2.005
        rect(outer, 16.0, 2.0, 18.0, 2.4),
        rect(outer, 16.0, 3.6, 18.0, 4.0),
        rect(outer, 16.0, 2.4, 16.4, 3.6),
        rect(outer, 17.6, 2.4, 18.0, 3.6), // clean, ring 2 × 2
        rect(outer, 16.8, 2.8, 17.2, 3.2), // E.min_dim, island 0.4
    ];
    for i in 0..6 {
        let y = 2.0 + 0.5 * i as f64;
        e.push(rect(outer, 8.0, y, 8.495, y + 0.5)); // E.min_dim, one 0.495 × 3 bar
    }
    write("dim_merge", e);

    // Tile lines.  0.495 × 3 bars ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, inside a tile at 10; a 3 × 0.495 bar
    // across 20 and one across y = 20; one at (1000, 1000); a 3 × 2 box from 19.7
    // whose piece left of 20 is 0.3 wide is clean; a 2.005 × 3 bar across 20, whose
    // pieces are 1 and 1.005, is over E.max_dim once.  E.min_dim: 10; E.max_dim: 1.
    let mut e = vec![];
    for (i, x) in [9.505, 19.505, 19.75, 20.0, 20.75, 39.505, 41.75]
        .iter()
        .enumerate()
    {
        let y = 2.0 + 4.0 * i as f64;
        e.push(rect(outer, *x, y, x + 0.495, y + 3.0)); // E.min_dim
    }
    e.push(rect(outer, 18.5, 30.0, 21.5, 30.495)); // E.min_dim
    e.push(rect(outer, 30.0, 18.5, 30.495, 21.5)); // E.min_dim
    e.push(rect(outer, 1000.0, 1000.0, 1000.495, 1003.0)); // E.min_dim
    e.push(rect(outer, 19.7, 34.0, 22.7, 36.0)); // clean
    e.push(rect(outer, 19.0, 40.0, 21.005, 43.0)); // E.max_dim
    write("dim_tile_lines", e);

    // Fifty 0.495 × 1 bars, flat and as an array reference.  E.min_dim: 50 each.
    let cell = vec![rect(outer, 0.2, 0.2, 0.695, 1.2)];
    write("dim_array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/dim_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Small and long.  A 0.005 × 3 sliver is under E.min_dim; a 300 × 1 bar is clean
    // both ways; a 300 × 300 plate is over E.max_dim.  E.min_dim: 1; E.max_dim: 1.
    write(
        "dim_extremes",
        vec![
            rect(outer, 2.0, 2.0, 2.005, 5.0),
            rect(outer, 2.0, 8.0, 302.0, 9.0),
            rect(outer, 2.0, 12.0, 302.0, 312.0),
        ],
    );

    // The long side (Inner: E.min_len 5, E.max_len 20).  1 × 5 and 5 × 1 are clean,
    // 1 × 4.995 and 4.995 × 1 fire; 1 × 20 is clean, 1 × 20.005 over; a diamond 4.99
    // across the box and a 45° strip with a 3.36 box are under 5.  E.min_len: 4;
    // E.max_len: 1.
    write(
        "len_bound",
        vec![
            rect(inner, 2.0, 2.0, 3.0, 7.0),      // clean
            rect(inner, 4.0, 2.0, 9.0, 3.0),      // clean
            rect(inner, 10.0, 2.0, 11.0, 6.995),  // E.min_len
            rect(inner, 12.0, 2.0, 16.995, 3.0),  // E.min_len
            rect(inner, 18.0, 2.0, 19.0, 22.0),   // clean
            rect(inner, 20.0, 2.0, 21.0, 22.005), // E.max_len
            diamond(inner, 26.0, 5.0, 2.495),     // E.min_len (box 4.99)
            strip45(inner, 30.0, 2.0, 3.0, 0.36), // E.min_len (box 3.36)
        ],
    );

    // Shapes that merge.  A 1 × 5 bar as two abutting halves is clean; two 1 × 3 bars
    // overlapping to a 4.995 union fire once; a 1 × 20.005 bar as two overlapping
    // halves is over E.max_len once; a 6 × 6 ring with a 1 band is clean and the 1 × 3
    // island in it fires.  E.min_len: 2; E.max_len: 1.
    write(
        "len_merge",
        vec![
            rect(inner, 2.0, 2.0, 3.0, 4.5),
            rect(inner, 2.0, 4.5, 3.0, 7.0), // clean, 5
            rect(inner, 4.0, 2.0, 5.0, 5.0),
            rect(inner, 4.0, 3.995, 5.0, 6.995), // E.min_len, 4.995
            rect(inner, 6.0, 2.0, 7.0, 12.0),
            rect(inner, 6.0, 11.0, 7.0, 22.005), // E.max_len, 20.005
            rect(inner, 10.0, 2.0, 16.0, 3.0),
            rect(inner, 10.0, 7.0, 16.0, 8.0),
            rect(inner, 10.0, 3.0, 11.0, 7.0),
            rect(inner, 15.0, 3.0, 16.0, 7.0), // clean, ring 6 × 6
            rect(inner, 12.5, 3.5, 13.5, 6.5), // E.min_len, island 1 × 3
        ],
    );

    // Tile lines.  1 × 4.995 bars across x = 20 and x = 21 and y = 20, 4.995 × 1 bars
    // ending on 20, starting on 20, straddling 40 and 42, at (1000, 1000); a 1 × 5 bar
    // from 19 to 24, whose piece left of 20 is 1 long, is clean; a 1 × 20.005 bar from
    // 10 to 30.005 and a 1 × 30 bar across 20 and 40 are over E.max_len once each.
    // E.min_len: 8; E.max_len: 2.
    write(
        "len_tile_lines",
        vec![
            rect(inner, 18.0, 2.0, 22.995, 3.0), // E.min_len, across 20
            rect(inner, 19.0, 4.0, 23.995, 5.0), // E.min_len, across 21
            rect(inner, 2.0, 18.0, 3.0, 22.995), // E.min_len, across y = 20
            rect(inner, 15.005, 6.0, 20.0, 7.0), // E.min_len, ending on 20
            rect(inner, 20.0, 8.0, 24.995, 9.0), // E.min_len, starting on 20
            rect(inner, 38.0, 2.0, 42.995, 3.0), // E.min_len, across 40
            rect(inner, 40.0, 4.0, 44.995, 5.0), // E.min_len, across 42
            rect(inner, 1000.0, 1000.0, 1004.995, 1001.0), // E.min_len
            rect(inner, 19.0, 10.0, 24.0, 11.0), // clean
            rect(inner, 10.0, 12.0, 30.005, 13.0), // E.max_len
            rect(inner, 15.0, 14.0, 45.0, 15.0), // E.max_len
        ],
    );

    // Fifty 1 × 4.5 bars, flat and as an array reference.  E.min_len: 50 each.
    let cell = vec![rect(inner, 0.2, 0.2, 1.2, 4.7)];
    write("len_array_flat", flat_array(&cell, 10, 5, 6.0));
    write_gz(
        &format!("{DIR}/len_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 6.0),
    );

    // Small and long.  A 0.005 × 5 sliver is clean, a 0.005 × 4.995 one under; a 300 ×
    // 1 bar is over E.max_len.  E.min_len: 1; E.max_len: 1.
    write(
        "len_extremes",
        vec![
            rect(inner, 2.0, 2.0, 2.005, 7.0),
            rect(inner, 4.0, 2.0, 4.005, 6.995),
            rect(inner, 2.0, 10.0, 302.0, 11.0),
        ],
    );

    // Exactly so (Via: E.exact_dim 0.3, E.exact_len 1.0).  0.3 × 1 and 1 × 0.3 are
    // right; 0.295 × 1 and 0.305 × 1 are off by the short side, 0.3 × 0.995 and
    // 0.3 × 1.005 by the long one.  E.exact_dim: 2; E.exact_len: 2.
    write(
        "exact_bound",
        vec![
            rect(via, 2.0, 2.0, 2.3, 3.0),   // right
            rect(via, 3.0, 2.0, 4.0, 2.3),   // right
            rect(via, 5.0, 2.0, 5.295, 3.0), // E.exact_dim
            rect(via, 6.0, 2.0, 6.305, 3.0), // E.exact_dim
            rect(via, 7.0, 2.0, 7.3, 2.995), // E.exact_len
            rect(via, 8.0, 2.0, 8.3, 3.005), // E.exact_len
        ],
    );

    // Shapes that merge.  0.3 × 1 as two abutting halves and as two abutting 0.15
    // strips is right; two 0.2 strips overlapping to 0.305 are off by the short side,
    // two halves overlapping to 1.005 by the long one.  E.exact_dim: 1; E.exact_len: 1.
    write(
        "exact_merge",
        vec![
            rect(via, 2.0, 2.0, 2.3, 2.5),
            rect(via, 2.0, 2.5, 2.3, 3.0), // right
            rect(via, 3.0, 2.0, 3.15, 3.0),
            rect(via, 3.15, 2.0, 3.3, 3.0), // right
            rect(via, 4.0, 2.0, 4.2, 3.0),
            rect(via, 4.105, 2.0, 4.305, 3.0), // E.exact_dim
            rect(via, 5.0, 2.0, 5.3, 2.6),
            rect(via, 5.0, 2.405, 5.3, 3.005), // E.exact_len
        ],
    );

    // Tile lines.  0.3 × 1 vias across x = 20, y = 20 and (20, 20), whose pieces are
    // 0.3 × 0.5, are right; a 0.305 × 1 across 20 and a 1.005 × 0.3 across 40 are off
    // once each; one 0.305 × 1 at (1000, 1000).  E.exact_dim: 2; E.exact_len: 1.
    write(
        "exact_tile_lines",
        vec![
            rect(via, 19.85, 2.0, 20.15, 3.0),           // right
            rect(via, 2.0, 19.5, 2.3, 20.5),             // right
            rect(via, 19.5, 19.85, 20.5, 20.15),         // right
            rect(via, 19.85, 5.0, 20.155, 6.0),          // E.exact_dim
            rect(via, 39.5, 2.0, 40.505, 2.3),           // E.exact_len
            rect(via, 1000.0, 1000.0, 1000.305, 1001.0), // E.exact_dim
        ],
    );

    // Fifty 0.305 × 1 vias, flat and as an array reference.  E.exact_dim: 50 each.
    let cell = vec![rect(via, 0.2, 0.2, 0.505, 1.2)];
    write("exact_array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/exact_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Small and long.  A 0.005 × 1 sliver is off by the short side, a 0.3 × 300 bar by
    // the long one.  E.exact_dim: 1; E.exact_len: 1.
    write(
        "exact_extremes",
        vec![
            rect(via, 2.0, 2.0, 2.005, 3.0),
            rect(via, 4.0, 2.0, 4.3, 302.0),
        ],
    );

    // Edges (Inner: L.min 0.5; Outer through its edge layer: L.edge 0.5).  A 3 µm
    // square with a 0.5 corner cut is clean; with a 0.495 cut its cut's top edge is
    // short; a 0.495 notch's floor, a 0.495 step's tread and a 0.353 chamfer (0.4992)
    // are short too.  L.min: 4.
    let edge_shapes = |l: (i16, i16)| {
        vec![
            cut(l, 0.5), // clean, at 30
            poly(
                l,
                &[
                    (2.0, 2.0),
                    (5.0, 2.0),
                    (5.0, 4.0),
                    (4.505, 4.0),
                    (4.505, 5.0),
                    (2.0, 5.0),
                ],
            ), // L.min, the cut's top edge 0.495
            poly(
                l,
                &[
                    (7.0, 2.0),
                    (10.0, 2.0),
                    (10.0, 5.0),
                    (9.0, 5.0),
                    (9.0, 4.0),
                    (8.505, 4.0),
                    (8.505, 5.0),
                    (7.0, 5.0),
                ],
            ), // L.min, the notch's floor 0.495
            poly(
                l,
                &[
                    (12.0, 2.0),
                    (15.0, 2.0),
                    (15.0, 5.0),
                    (13.0, 5.0),
                    (13.0, 4.0),
                    (12.505, 4.0),
                    (12.505, 5.0),
                    (12.0, 5.0),
                ],
            ), // L.min, the step's tread 0.495 (its riser is 1)
            chamfered_tr(l, 17.0, 2.0, 20.0, 5.0, 24.647), // L.min, chamfer 0.4992
        ]
    };
    write("edge_bound", edge_shapes(inner));
    write("edge_layer_bound", edge_shapes(outer));

    // Shapes that merge.  Two abutting 3 µm squares are one 6 × 3 box with no short
    // edge; two 3 µm squares overlapping, offset 0.495 up, have two 0.495 edges on
    // their union; a 3 × 3 ring with a 1 × 1 hole has none; a 0.495 square island in
    // it has four.  L.min: 6.
    write(
        "edge_merge",
        vec![
            rect(inner, 2.0, 2.0, 5.0, 5.0),
            rect(inner, 5.0, 2.0, 8.0, 5.0), // clean
            rect(inner, 10.0, 2.0, 13.0, 5.0),
            rect(inner, 12.0, 2.495, 15.0, 5.495), // L.min ×2
            rect(inner, 17.0, 2.0, 20.0, 3.0),
            rect(inner, 17.0, 4.0, 20.0, 5.0),
            rect(inner, 17.0, 3.0, 18.0, 4.0),
            rect(inner, 19.0, 3.0, 20.0, 4.0), // clean, ring
            rect(inner, 18.2525, 3.2525, 18.7475, 3.7475), // L.min ×4, island 0.495
        ],
    );

    // Tile lines.  A 3 µm square across x = 20 has no short edge, the tile's cuts being
    // no edges of it; a 0.495 notch floor across 20, one ending on 20, one starting on
    // 20, one across 21, 40 and 42; a 0.495 riser lying on x = 20; a floor at (1000,
    // 1000).  L.min: 8.
    let step = |l: (i16, i16), x: f64, y: f64| {
        // A 3 µm square with a notch 0.495 wide and 1 deep in its top, floored from x.
        poly(
            l,
            &[
                (x - 1.0, y),
                (x + 2.0, y),
                (x + 2.0, y + 3.0),
                (x + 0.495, y + 3.0),
                (x + 0.495, y + 2.0),
                (x, y + 2.0),
                (x, y + 3.0),
                (x - 1.0, y + 3.0),
            ],
        )
    };
    let riser = |l: (i16, i16), x: f64, y: f64| {
        // A 3 µm square whose top-right corner steps in by 1 µm at x, 0.495 down.
        poly(
            l,
            &[
                (x - 2.0, y),
                (x + 1.0, y),
                (x + 1.0, y + 2.505),
                (x, y + 2.505),
                (x, y + 3.0),
                (x - 2.0, y + 3.0),
            ],
        )
    };
    let edge_lines = |l: (i16, i16)| {
        vec![
            rect(l, 18.5, 2.0, 21.5, 5.0), // clean
            step(l, 19.75, 6.0),           // L.min, floor across 20
            step(l, 19.505, 10.0),         // L.min, floor ending on 20
            step(l, 20.0, 14.0),           // L.min, floor starting on 20
            step(l, 20.75, 18.0),          // L.min, floor across 21
            step(l, 39.75, 2.0),           // L.min, floor across 40
            step(l, 41.75, 6.0),           // L.min, floor across 42
            riser(l, 20.0, 22.0),          // L.min, riser on x = 20
            step(l, 1000.0, 1000.0),       // L.min
        ]
    };
    write("edge_tile_lines", edge_lines(inner));
    write("edge_layer_tile_lines", edge_lines(outer));

    // Fifty squares with a 0.495 notch, flat and as an array reference.  L.min: 50
    // each.
    let cell = vec![step(inner, 1.2, 0.2)];
    write("edge_array_flat", flat_array(&cell, 10, 5, 4.0));
    write_gz(
        &format!("{DIR}/edge_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 4.0),
    );

    // Small and long.  A 0.005 notch floor; a 300 × 0.5 bar, whose ends are 0.5, is
    // clean.
    // L.min: 1.
    write(
        "edge_extremes",
        vec![
            poly(
                inner,
                &[
                    (2.0, 2.0),
                    (5.0, 2.0),
                    (5.0, 5.0),
                    (3.005, 5.0),
                    (3.005, 4.0),
                    (3.0, 4.0),
                    (3.0, 5.0),
                    (2.0, 5.0),
                ],
            ),
            rect(inner, 2.0, 8.0, 302.0, 8.5),
        ],
    );

    // Corners (Outer: C.right).  A square across x = 20, one with a corner on (20, 10),
    // an L whose inner corner lies on (20, 20), a square at (1000, 1000): the tile's
    // cuts are no corners.  C.right: 18.  H.none: 0; V.max: 0.
    write(
        "corner_tile_lines",
        vec![
            rect(outer, 18.5, 2.0, 21.5, 5.0),   // 4
            rect(outer, 20.0, 10.0, 23.0, 13.0), // 4
            poly(
                outer,
                &[
                    (17.0, 17.0),
                    (23.0, 17.0),
                    (23.0, 20.0),
                    (20.0, 20.0),
                    (20.0, 23.0),
                    (17.0, 23.0),
                ],
            ), // 6
            rect(outer, 1000.0, 1000.0, 1003.0, 1003.0), // 4
        ],
    );

    // Shapes that merge.  Two abutting squares are one box with four corners; two
    // overlapping squares offset both ways have eight; a 3 µm square drawn as nine
    // has four.  C.right: 16.
    let mut e = vec![
        rect(outer, 2.0, 2.0, 5.0, 5.0),
        rect(outer, 5.0, 2.0, 8.0, 5.0), // 4
        rect(outer, 10.0, 2.0, 13.0, 5.0),
        rect(outer, 12.0, 4.0, 15.0, 7.0), // 8
    ];
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = (17.0 + i as f64, 2.0 + j as f64);
            e.push(rect(outer, x, y, x + 1.0, y + 1.0)); // 4 in all
        }
    }
    write("corner_merge", e);

    // Fifty squares, flat and as an array reference.  C.right: 200 each.
    let cell = vec![rect(outer, 0.2, 0.2, 1.2, 1.2)];
    write("corner_array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/corner_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Sharp corners (Inner: C.sharp, sharper than a right angle).  Right triangles
    // across x = 20, y = 20 and at (1000, 1000): the two 45° corners of each, and not
    // the cut vertices.  C.sharp: 6.  A.ortho: 3, the hypotenuses.
    write(
        "sharp_tile_lines",
        vec![
            poly(inner, &[(18.0, 2.0), (22.0, 2.0), (18.0, 6.0)]),
            poly(inner, &[(2.0, 18.0), (6.0, 18.0), (2.0, 22.0)]),
            poly(
                inner,
                &[(1000.0, 1000.0), (1004.0, 1000.0), (1000.0, 1004.0)],
            ),
        ],
    );

    // Angles (Inner: A.ortho, every edge off the axes).  A diamond across x = 20 has
    // four; a 45° strip across (20, 20) has four; a box whose chamfer crosses 20 has
    // one; a chamfered box at (1000, 1000) one.  A.ortho: 10.
    write(
        "angle_tile_lines",
        vec![
            diamond(inner, 20.0, 5.0, 2.0),
            strip45(inner, 18.0, 18.0, 4.0, 0.5),
            chamfered_tr(inner, 17.0, 10.0, 21.0, 14.0, 34.0),
            chamfered_tr(inner, 1000.0, 1000.0, 1003.0, 1003.0, 2005.0),
        ],
    );

    // Only 45° (Via: A.bent45).  A 45° bar across x = 20 has two walls at 45° and two
    // at 135°; a bar at 26.6° across y = 20 has none at 45.  A.bent45: 2.
    write(
        "angle45_tile_lines",
        vec![
            bar45(via, 18.0, 2.0, 4.0, 0.6),
            poly(via, &[(2.0, 18.0), (6.0, 20.0), (6.0, 20.6), (2.0, 18.6)]),
        ],
    );

    // Holes (Outer: H.none).  A ring across x = 20, one whose hole ends on 20, one
    // across (20, 20), one drawn as four bars across 20, one with an island in the
    // hole across 20, a 50 µm ring over three tiles, one at (1000, 1000).  H.none: 7.
    let ring = |x0: f64, y0: f64, x1: f64, y1: f64, band: f64| {
        vec![
            rect(outer, x0, y0, x1, y0 + band),
            rect(outer, x0, y1 - band, x1, y1),
            rect(outer, x0, y0 + band, x0 + band, y1 - band),
            rect(outer, x1 - band, y0 + band, x1, y1 - band),
        ]
    };
    // One boundary with a keyhole slit to its hole: the drawn polygon has the hole.
    let hollow = |x0: f64, y0: f64, x1: f64, y1: f64, band: f64| {
        poly(
            outer,
            &[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0 + band, y1),
                (x0 + band, y1 - band),
                (x1 - band, y1 - band),
                (x1 - band, y0 + band),
                (x0 + band, y0 + band),
                (x0 + band, y1),
                (x0, y1),
            ],
        )
    };
    let mut e = vec![hollow(18.0, 2.0, 22.0, 6.0, 1.0)]; // across 20
    e.push(hollow(15.0, 8.0, 21.0, 12.0, 1.0)); // hole ending on 20
    e.push(hollow(18.0, 18.0, 22.0, 22.0, 1.0)); // across (20, 20)
    e.extend(ring(18.0, 26.0, 22.0, 30.0, 1.0)); // four bars across 20
    e.extend(ring(17.0, 34.0, 23.0, 40.0, 1.0)); // ring with an island
    e.push(rect(outer, 19.5, 36.5, 20.5, 37.5));
    e.extend(ring(5.0, 45.0, 55.0, 95.0, 2.0)); // over three tiles
    e.extend(ring(1000.0, 1000.0, 1004.0, 1004.0, 1.0));
    write("hole_tile_lines", e);

    // Fifty rings, flat and as an array reference.  H.none: 50 each.
    let cell = ring(0.2, 0.2, 1.4, 1.4, 0.3);
    write("hole_array_flat", flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/hole_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Vertices (Outer: V.max 8, on the drawn polygon).  A 9-gon across x = 20 and one
    // at (1000, 1000) are over; an octagon across 20, whose tile pieces have more
    // vertices than it, is not.  V.max: 2.
    let ngon = |x: f64, y: f64, nine: bool| {
        let mut pts = vec![
            (x + 1.0, y),
            (x + 2.0, y),
            (x + 3.0, y + 1.0),
            (x + 3.0, y + 2.0),
        ];
        if nine {
            pts.push((x + 2.5, y + 2.5));
        }
        pts.extend([
            (x + 2.0, y + 3.0),
            (x + 1.0, y + 3.0),
            (x, y + 2.0),
            (x, y + 1.0),
        ]);
        poly(outer, &pts)
    };
    write(
        "vert_tile_lines",
        vec![
            ngon(18.5, 2.0, true),
            ngon(1000.0, 1000.0, true),
            ngon(18.5, 8.0, false),
        ],
    );

    // Fifty 9-gons, flat and as an array reference.  V.max: 50 each.
    let cell = vec![ngon(0.2, 0.2, true)];
    write("vert_array_flat", flat_array(&cell, 10, 5, 4.0));
    write_gz(
        &format!("{DIR}/vert_array_ref.gds.gz"),
        ref_array(cell, 10, 5, 4.0),
    );
}
