// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for the enclosure family ([`enclosure`](gdscheck::checks::enclosure)): a
//! margin measured on three shape families, swept either side of the limit, under both
//! metrics; then the other bound and each `sides` word, met exactly and missed by five
//! nanometres.
//!
//! The families are chosen for where the metrics part company.  Orthogonal walls are the
//! case they must agree on.  A 45° chamfer in the enclosing corner is the case where
//! nothing is parallel, so the projection metric has no facing run to measure and only
//! the euclidian one reaches the wall.  A 45° notch is that same divergence away from a
//! corner, in the middle of a wall.

use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/min_enclosure";

/// Margins the enclosure families are drawn at, against a 0.5 µm limit: two under it and
/// two over, the inner pair a single nanometre either side so the comparison itself is
/// under test and not just the geometry.
const ORTHO_MARGINS: [f64; 4] = [0.4, 0.499, 0.5, 0.6];

/// The diagonal family cannot use those numbers.  Its margin is a perpendicular distance
/// to a 45° wall, so it is the corner's offset over √2 and no offset on the 1 nm grid
/// lands on 0.5 exactly.  These four corner positions give 0.566, 0.501, 0.499 and 0.354 -
/// the middle pair straddling the limit by less than a nanometre of margin.
const DIAG_CORNERS: [(f64, f64); 4] = [
    (8.100, 0.565_685),
    (8.146, 0.500_631),
    (8.147, 0.499_217),
    (8.250, 0.353_553),
];

fn name(margin: f64) -> String {
    format!("{:04}", (margin * 1000.0).round() as i64)
}

pub fn generate(pdk: &PdkConfig) {
    let outer = layer(pdk, "Outer");
    let inner = layer(pdk, "Inner");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: String, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A bar 40 µm long across three tiles, enclosed by a row of 2.5 µm pieces the way
    // abutting cells enclose a merged row of their Activ, its far end margin under test.
    // The tile owning the bar's centroid holds a copy of the enclosing layer exact
    // only 1 µm past its core, so the far end lies beyond what that copy shows.
    for m in [0.4, 0.6] {
        let mut v = vec![rect(inner, 5.0, 10.0, 45.0, 11.0)];
        let mut x = 0.0;
        while x < 45.0 + m {
            v.push(rect(outer, x, 9.0, (x + 2.5).min(45.0 + m), 12.0));
            x += 2.5;
        }
        write(format!("bar_{}", name(m)), v);
    }

    // Orthogonal: a square hole in a square, one wall brought in to the margin under
    // test.  Both metrics measure this one, and must agree on it.
    for m in ORTHO_MARGINS {
        write(
            format!("ortho_{}", name(m)),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, m, 1.0, 9.0, 9.0),
            ],
        );
    }

    // Diagonal wall: the enclosing shape's top-right corner is chamfered at 45° and the
    // enclosed square's corner points into it.  Every orthogonal margin is 1.0 or more
    // and no pair of walls here is parallel, so the projection metric has nothing to
    // measure and only the euclidian one sees the chamfer.
    for (t, m) in DIAG_CORNERS {
        write(
            format!("diag_{}", name(m)),
            vec![
                poly(
                    outer,
                    &[
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 7.0),
                        (7.0, 10.0),
                        (0.0, 10.0),
                    ],
                ),
                rect(inner, 1.0, 1.0, t, t),
            ],
        );
    }

    // A corner resting exactly on the enclosing wall measures zero, which is under any
    // limit and is not what the rule is about.  `touch` is that alone; `touch_and_diag`
    // puts a real 0.354 margin on a second chamfer beside it, so the pair says whether
    // dropping the zero also drops the violation next to it.
    let two_chamfers = vec![
        (0.0, 0.0),
        (7.0, 0.0),
        (10.0, 3.0),
        (10.0, 7.0),
        (7.0, 10.0),
        (0.0, 10.0),
    ];
    write(
        "touch".into(),
        vec![poly(outer, &two_chamfers), rect(inner, 1.0, 5.5, 8.5, 8.5)],
    );
    write(
        "touch_and_diag".into(),
        vec![
            poly(outer, &two_chamfers),
            rect(inner, 1.0, 5.5, 8.5, 8.5),
            rect(inner, 1.0, 1.0, 7.5, 3.5),
        ],
    );

    // Diagonal notch: the enclosing wall dips towards the enclosed shape in a 45° V.
    // The wall it faces is still 1.7 away, so again only the euclidian metric reaches
    // the apex - and unlike the chamfer, the violation is in the middle of a wall
    // rather than at a corner.
    for m in ORTHO_MARGINS {
        let ay = 8.3 + m;
        let half = 10.0 - ay;
        write(
            format!("notch_{}", name(m)),
            vec![
                poly(
                    outer,
                    &[
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 10.0),
                        (4.6 + half, 10.0),
                        (4.6, ay),
                        (4.6 - half, 10.0),
                        (0.0, 10.0),
                    ],
                ),
                rect(inner, 1.0, 1.0, 9.0, 8.3),
            ],
        );
    }

    // The maximum: a square with every margin exactly 0.5, and one with its left margin
    // 0.505.  ENC.max: 0 and 1; ENC.proj: 0 and 0.
    for (name, left) in [("max_exact", 0.5), ("max_over", 0.505)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, 0.5, 9.5, 9.5),
            ],
        );
    }

    // The endcap: a shape flush on three sides with its right margin 0.5 and 0.495 -
    // the best side, which is all a minimum on `any` asks about.  ENC.any: 0 and 1.  A
    // maximum on `any` asks that some side be within the value, and a flush side always
    // is: ENC.max_any: 0 and 0.
    for (name, right) in [("any_exact", 9.5), ("any_under", 9.505)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, 0.0, 0.0, right, 10.0),
            ],
        );
    }

    // A maximum on `any` fails only when every side is over: a square with every margin
    // 0.505, and one with three at 0.505 and its left at 0.5.  ENC.max_any: 1 and 0;
    // ENC.max: 1 and 1.
    for (name, left) in [("max_any_over", 0.505), ("max_any_one", 0.5)] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, 0.505, 9.495, 9.495),
            ],
        );
    }

    // Bordering sides: a shape 0.05 from the left wall, under ENC.adj's 0.1 trigger,
    // whose bottom margin is 1.0 and 0.495; and one exactly 0.1 from the wall, not
    // short, whose bottom margin is 0.495 and asks nothing.  ENC.adj: 0, 1, 0.
    for (name, left, bottom) in [
        ("adj_ok", 0.05, 1.0),
        ("adj_under", 0.05, 0.495),
        ("adj_trigger", 0.1, 0.495),
    ] {
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 10.0, 10.0),
                rect(inner, left, bottom, 5.0, 9.0),
            ],
        );
    }

    // A line end: a 0.3 µm track 5 long with a 0.2 via inside near its tip, the tip's
    // cap 0.3 and 0.295 from the via; and a track exactly ENC.cap's 0.34 wide, which is
    // not a narrow line.  ENC.cap: 0, 1, 0.
    for (name, w, via_right) in [
        ("cap_exact", 0.3, 4.7),
        ("cap_under", 0.3, 4.705),
        ("cap_wide", 0.34, 4.705),
    ] {
        let y0 = (w - 0.2) * 0.5;
        write(
            name.into(),
            vec![
                rect(outer, 0.0, 0.0, 5.0, w),
                rect(inner, via_right - 0.2, y0, via_right, y0 + 0.2),
            ],
        );
    }

    // An extension: a cover 2 wide crossing a 10 µm target bar 2 tall, reaching 0.5
    // past the bar's long walls, then 0.495 and 0.505 past the lower one.  The bar's
    // ends lie outside the cover and are not measured.  ENC.ext: 0, 1, 0;
    // ENC.max_ext: 0, 0, 1.
    for (name, y0) in [
        ("ext_exact", 3.5),
        ("ext_under", 3.505),
        ("ext_over", 3.495),
    ] {
        write(
            name.into(),
            vec![
                rect(inner, 0.0, 4.0, 10.0, 6.0),
                rect(outer, 2.0, y0, 4.0, 6.5),
            ],
        );
    }

    // --- The hardening patterns: what a rule manual's enclosure asks of any pair of
    // layers, drawn once here for every deck of every PDK (hardening/SPEC.md).  A 0.4
    // square of Inner in Outer under ENC.proj / ENC.eucl (0.5) and ENC.adj (`sides:
    // adjacent`, trigger 0.1); a violation is one wall's margin, the runs of a shape
    // joined at their corners.
    let sq = |cx: f64, cy: f64| rect(inner, cx - 0.2, cy - 0.2, cx + 0.2, cy + 0.2);
    // The Outer round a square with the given margins on the left, bottom, right, top.
    let enc = |cx: f64, cy: f64, l: f64, b: f64, r: f64, t: f64| {
        rect(
            outer,
            cx - 0.2 - l,
            cy - 0.2 - b,
            cx + 0.2 + r,
            cy + 0.2 + t,
        )
    };

    // The bound.  A square with 0.5 all round is clean; 0.495 on the left, right, bottom
    // or top fires once each; a square half out fires; a square with no Outer at all is
    // enclosed by nothing.  ENC.proj: 6, ENC.eucl: 6.
    write(
        "bound".into(),
        vec![
            sq(2.5, 2.5),
            enc(2.5, 2.5, 0.5, 0.5, 0.5, 0.5), // clean
            sq(5.0, 2.5),
            enc(5.0, 2.5, 0.495, 0.5, 0.5, 0.5), // 0.495 left
            sq(7.5, 2.5),
            enc(7.5, 2.5, 0.5, 0.5, 0.495, 0.5), // 0.495 right
            sq(10.0, 2.5),
            enc(10.0, 2.5, 0.5, 0.495, 0.5, 0.5), // 0.495 bottom
            sq(12.5, 2.5),
            enc(12.5, 2.5, 0.5, 0.5, 0.5, 0.495), // 0.495 top
            sq(2.5, 5.0),
            rect(outer, 2.5, 4.3, 3.9, 5.7), // half out
            sq(5.0, 5.0),                    // no Outer
        ],
    );

    // Shapes and corners.  A square over the seam of two abutting boxes and one over two
    // overlapping boxes are enclosed by the union (clean); a square across a 0.005 gap
    // between two boxes has a strip uncovered (fires, either metric); a square under a
    // grid of boxes that cover it is clean; a chamfer 0.495 from the square's corner
    // (both walls 0.5 off) is the closest approach's (ENC.eucl) and no parallel pair
    // (ENC.proj); a chamfer 0.502 off is clean; a square in a diamond whose four walls
    // pass 0.495 from its corners is four corners under ENC.eucl.  ENC.proj: 1,
    // ENC.eucl: 6.
    let mut e = vec![
        sq(2.5, 2.5),
        rect(outer, 1.8, 1.8, 2.5, 3.2),
        rect(outer, 2.5, 1.8, 3.2, 3.2), // seam, clean
        sq(5.0, 2.5),
        rect(outer, 4.3, 1.8, 5.1, 3.2),
        rect(outer, 4.9, 1.8, 5.7, 3.2), // overlap, clean
        sq(7.5, 2.5),
        rect(outer, 6.8, 1.8, 7.5, 3.2),
        rect(outer, 7.505, 1.8, 8.2, 3.2), // gap: a 0.005 strip uncovered
    ];
    for i in 0..7 {
        for j in 0..7 {
            let (x, y) = (9.3 + 0.2 * i as f64, 1.8 + 0.2 * j as f64);
            e.push(rect(outer, x, y, x + 0.2, y + 0.2)); // grid covers 9.3..10.7, clean
        }
    }
    e.push(sq(10.0, 2.5));
    // The chamfer x + y = k passes (k − 7.9)/√2 from the corner (2.7, 5.2): k = 8.6 →
    // 0.495, k = 8.61 → 0.502.
    e.push(sq(2.5, 5.0));
    e.push(chamfered_tr(outer, 1.8, 4.3, 3.2, 5.7, 8.6)); // 0.495 from the corner: ENC.eucl
    e.push(sq(5.0, 5.0));
    e.push(chamfered_tr(outer, 4.3, 4.3, 5.7, 5.7, 13.61)); // 0.502: clean
    // A diamond of half-diagonal a round the square's centre: its walls are
    // (a − 0.4)/√2 from the corners; a = 1.1 → 0.495.
    e.push(sq(8.0, 5.0));
    e.push(diamond(outer, 8.0, 5.0, 1.1)); // four corners: ENC.eucl
    write("shapes".into(), e);

    // Tile lines.  Squares sticking 0.005 out to the right of an Outer ending on x = 20,
    // 21, 40, 42 and at 10, one at (1000, 1000); squares straddling x = 20 and 40 with
    // 0.5 all round are clean.  ENC.proj: 6, ENC.eucl: 6.
    let mut e = vec![];
    for x in [10.0, 20.0, 21.0, 40.0, 42.0, 1000.0] {
        e.push(sq(x - 0.195, 2.5)); // x − 0.395 .. x + 0.005
        e.push(rect(outer, x - 0.9, 1.8, x, 3.2)); // 0.005 out
    }
    e.push(sq(20.0, 5.0));
    e.push(enc(20.0, 5.0, 0.5, 0.5, 0.5, 0.5)); // clean
    e.push(sq(40.0, 5.0));
    e.push(enc(40.0, 5.0, 0.5, 0.5, 0.5, 0.5)); // clean
    write("tile_lines".into(), e);

    // Fifty squares sticking 0.005 out, flat and as an array reference.  ENC.proj: 50.
    let cell = vec![sq(0.7, 0.7), rect(outer, 0.0, 0.0, 0.895, 1.4)];
    write("array_flat".into(), flat_array(&cell, 10, 5, 2.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 2.0),
    );

    // Bordering sides (ENC.adj: a side under the 0.1 trigger is a line running past, and
    // a line has one such side or two opposite ones; two adjacent short sides, or
    // three, or four, leave a corner with no cap).  Line ends: a square in a 0.4 line
    // with a 0.5 cap is clean, 0.495 fires, 0.0 fires, mid-line is clean.  Corners of a
    // plate: flush on two sides fires, 0.5 left and flush bottom is clean, 0.05/0.05
    // fires, 0.05 left and 0.5 bottom is clean.  Pads: 0.05 all round fires; 0.5 left
    // and 0.05 elsewhere fires; 0.5 left and right with 0.05 top and bottom is clean, and
    // so is 0.05 left and right with 0.5 top and bottom.  ENC.adj: 6.
    write(
        "adjacent".into(),
        vec![
            sq(2.5, 2.5),
            enc(2.5, 2.5, 1.5, 0.0, 0.5, 0.0), // clean: cap 0.5
            sq(5.0, 2.5),
            enc(5.0, 2.5, 1.5, 0.0, 0.495, 0.0), // ENC.adj: cap 0.495
            sq(7.5, 2.5),
            enc(7.5, 2.5, 1.5, 0.0, 0.0, 0.0), // ENC.adj: cap 0.0
            sq(10.0, 2.5),
            enc(10.0, 2.5, 1.5, 0.0, 1.5, 0.0), // clean: mid-line
            sq(2.5, 5.0),
            enc(2.5, 5.0, 0.0, 0.0, 1.5, 1.5), // ENC.adj: flush left and bottom
            sq(5.0, 5.0),
            enc(5.0, 5.0, 0.5, 0.0, 1.5, 1.5), // clean: 0.5 left, flush bottom
            sq(7.5, 5.0),
            enc(7.5, 5.0, 0.05, 0.05, 1.5, 1.5), // ENC.adj: 0.05/0.05
            sq(10.0, 5.0),
            enc(10.0, 5.0, 0.05, 0.5, 1.5, 1.5), // clean: 0.05 left, 0.5 bottom
            sq(2.5, 9.0),
            enc(2.5, 9.0, 0.05, 0.05, 0.05, 0.05), // ENC.adj: 0.05 all round
            sq(5.0, 9.0),
            enc(5.0, 9.0, 0.5, 0.05, 0.05, 0.05), // ENC.adj: 0.5 left, 0.05 elsewhere
            sq(7.5, 9.0),
            enc(7.5, 9.0, 0.5, 0.05, 0.5, 0.05), // clean: 0.5 left/right, 0.05 top/bottom
            sq(10.0, 9.0),
            enc(10.0, 9.0, 0.05, 0.5, 0.05, 0.5), // clean: 0.05 left/right, 0.5 top/bottom
        ],
    );

    // The maximum across the tile lines (ENC.max, 0.5, one violation per shape read
    // whole): squares whose Outer's left margin is 0.505 with the Outer across x = 20,
    // the square across 20, the Outer ending on 20, the square's wall on 21, across y
    // = 40, at (1000, 1000); a square in the middle of a 45 µm plate over three tiles,
    // whose margin is 22 and read once, not at every line it crosses; a square with
    // 0.5 all round straddling 20 is clean.  ENC.max: 7.
    write(
        "max_tile_lines".into(),
        vec![
            sq(20.8, 2.5),
            enc(20.8, 2.5, 1.105, 0.5, 0.5, 0.5), // Outer from 19.495 across 20
            sq(20.0, 5.0),
            enc(20.0, 5.0, 0.505, 0.5, 0.5, 0.5), // square 19.8..20.2 across 20
            sq(19.3, 7.5),
            enc(19.3, 7.5, 0.505, 0.5, 0.5, 0.5), // Outer ending on 20
            sq(21.2, 10.0),
            enc(21.2, 10.0, 0.505, 0.5, 0.5, 0.5), // square's left wall on 21
            sq(5.0, 40.0),
            enc(5.0, 40.0, 0.505, 0.5, 0.5, 0.5), // across y = 40
            sq(1000.2, 1000.2),
            enc(1000.2, 1000.2, 0.505, 0.5, 0.5, 0.5),
            sq(30.0, 60.0),
            enc(30.0, 60.0, 22.3, 22.3, 22.3, 22.3), // a 45 µm plate, 7.5..52.5
            sq(20.0, 15.0),
            enc(20.0, 15.0, 0.5, 0.5, 0.5, 0.5), // clean
        ],
    );

    // The extension across the tile lines (ENC.ext / ENC.max_ext, 0.5): a 10 µm bar
    // of Inner across x = 20 with a 2 µm cover across the line reaching 0.495 past
    // the bar's lower wall, and one reaching 0.505; the same across y = 40; a bar at
    // (1000, 1000) with a 0.495 cover.  ENC.ext: 3; ENC.max_ext: 2.
    let ext = |x: f64, y: f64, past: f64| {
        vec![
            rect(inner, x - 5.0, y, x + 5.0, y + 2.0),
            rect(outer, x - 1.0, y - past, x + 1.0, y + 2.5),
        ]
    };
    let ext_v = |x: f64, y: f64, past: f64| {
        vec![
            rect(inner, x, y - 5.0, x + 2.0, y + 5.0),
            rect(outer, x - past, y - 1.0, x + 2.5, y + 1.0),
        ]
    };
    let mut e = ext(20.0, 2.0, 0.495);
    e.extend(ext(20.0, 8.0, 0.505));
    e.extend(ext_v(2.0, 40.0, 0.495));
    e.extend(ext_v(12.0, 40.0, 0.505));
    e.extend(ext(1000.0, 1000.0, 0.495));
    write("ext_tile_lines".into(), e);

    // Small and long.  A 0.005 × 0.4 sliver of Inner with 0.495 on its left; a 300 µm
    // bar of Inner in an Outer with 0.5 all round but 0.495 at its far end; one with
    // 0.5 all round is clean.  ENC.proj: 2; ENC.eucl: 2.
    write(
        "extremes".into(),
        vec![
            rect(inner, 2.0, 2.0, 2.005, 2.4),
            rect(outer, 1.505, 1.5, 2.505, 2.9),
            rect(inner, 2.0, 6.0, 302.0, 6.4),
            rect(outer, 1.5, 5.5, 302.495, 6.9),
            rect(inner, 2.0, 10.0, 302.0, 10.4),
            rect(outer, 1.5, 9.5, 302.5, 10.9), // clean
        ],
    );
}
