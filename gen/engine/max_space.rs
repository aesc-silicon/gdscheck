// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`max_space`](gdscheck::checks::max_space): every part of one layer
//! within a distance of another, asked where the answer depends on how the reference is
//! grown and on where the tile lines fall.
//!
//! The reference grown by the value is a square structuring element - KLayout's `sized`
//! on rectilinear geometry - so a target diagonally off a corner is covered out to the
//! value in each axis, further than a circle reaches.  A tie ring is the shape that
//! tells a grown polygon from a grown bounding box: the ring's box grown by the value
//! covers its whole interior, the ring itself does not, and a device in the middle of a
//! large ring is exactly what the latch-up rule is for.  The tile lines are where a
//! target or a reference cut in two has to be read as one shape: a gap across a line is
//! one gap, and a reference across one covers as far as it would whole.  The deck's
//! value is 20 µm, and so is the tile, with lines at its multiples.
//!
//! The three scopes meet the bound differently.  Every *part* within the value: a
//! target whose far wall is exactly the value from the reference is clean, and one
//! five nanometres further has a strip beyond reach.  Every *polygon*: one whose near
//! wall is exactly the value away is in reach, five nanometres further it is not.  And
//! the reach confined to a layer goes round a slot in it, so a target on the far leg
//! of a U is out of reach across the slot at 16 µm and in reach along the leg at 13.

use crate::helpers::{
    diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/max_space";
pub fn generate(pdk: &PdkConfig) {
    let target = layer(pdk, "Outer");
    let reference = layer(pdk, "Inner");
    let well = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: &str, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };

    // A square tie ring 100 µm across with a 2 µm band, drawn as four bars, spanning
    // several tiles.  A 4 µm target at its centre is 46 µm from the band and lies in a
    // gap; one 8 µm from the band is covered.
    let ring = |o: f64| {
        vec![
            rect(reference, o, o, o + 100.0, o + 2.0),
            rect(reference, o, o + 98.0, o + 100.0, o + 100.0),
            rect(reference, o, o + 2.0, o + 2.0, o + 98.0),
            rect(reference, o + 98.0, o + 2.0, o + 100.0, o + 98.0),
        ]
    };
    let mut v = ring(10.0);
    v.push(rect(target, 58.0, 58.0, 62.0, 62.0));
    write("ring_far", v);
    let mut v = ring(10.0);
    v.push(rect(target, 20.0, 58.0, 24.0, 62.0));
    write("ring_near", v);

    // A target across the tile line at x = 40, the reference 28 µm off: one gap, not
    // one per tile the target has a piece in.  At 5 µm off it is covered.
    for (name, rx) in [("straddle_far", 70.0), ("straddle_near", 47.0)] {
        write(
            name,
            vec![
                rect(target, 38.0, 30.0, 42.0, 34.0),
                rect(reference, rx, 30.0, rx + 2.0, 34.0),
            ],
        );
    }

    // A reference across the tile line at x = 40 covers two tiles further, as far as the
    // value: a target 18 µm off is covered, one 22 µm off is not.
    for (name, tx) in [("reach_far", 64.0), ("reach_near", 60.0)] {
        write(
            name,
            vec![
                rect(reference, 38.0, 30.0, 42.0, 34.0),
                rect(target, tx, 30.0, tx + 2.0, 34.0),
            ],
        );
    }

    // A target diagonally off the reference's corner, 18 µm away in each axis: within
    // the square's reach, and 25 µm as the crow flies.  At 22 µm in each axis it is out.
    for (name, off) in [("corner_far", 24.0), ("corner_near", 20.0)] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, 30.0 + off, 30.0 + off, 32.0 + off, 32.0 + off),
            ],
        );
    }

    // Every part: a 2 µm reference at 30..32 and a target from 34 to 52, whose far wall
    // is 20 from the reference's; then to 52.005.  SPC.max: 0 and 1.
    for (name, far) in [("part_exact", 52.0), ("part_over", 52.005)] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, 34.0, 30.0, far, 32.0),
            ],
        );
    }

    // Every polygon: a 2 µm target whose near wall is 20 from the reference, and one
    // 20.005.  M.poly: 0 and 1.  A block from 34 to 60 has its near part in reach and
    // its far part not: M.poly 0, SPC.max 1.
    for (name, x0, x1) in [
        ("poly_exact", 52.0, 54.0),
        ("poly_over", 52.005, 54.005),
        ("poly_block", 34.0, 60.0),
    ] {
        write(
            name,
            vec![
                rect(reference, 30.0, 30.0, 32.0, 32.0),
                rect(target, x0, 30.0, x1, 32.0),
            ],
        );
    }

    // The reach confined to Via, drawn as a 30 µm U with a 6 µm slot from the bottom
    // up to y = 26, across the tile lines at 20.  The reference sits on the left leg
    // at 5..7; a target on the right leg at 23..25 is 16 µm away as the crow flies and
    // 48 round the slot: M.within 1, M.poly 0.  One on the same leg at y = 20..22 is
    // 13 up the leg: M.within 0.
    let u = || {
        vec![
            poly(
                well,
                &[
                    (0.0, 0.0),
                    (12.0, 0.0),
                    (12.0, 26.0),
                    (18.0, 26.0),
                    (18.0, 0.0),
                    (30.0, 0.0),
                    (30.0, 30.0),
                    (0.0, 30.0),
                ],
            ),
            rect(reference, 5.0, 5.0, 7.0, 7.0),
        ]
    };
    let mut v = u();
    v.push(rect(target, 23.0, 5.0, 25.0, 7.0));
    write("within_slot", v);
    let mut v = u();
    v.push(rect(target, 5.0, 20.0, 7.0, 22.0));
    write("within_leg", v);

    // Every edge: a 0.2 µm bar of Outer 2 µm tall, and the reference 20 from its right
    // wall: the left wall is 20.2 away and the other three reach.  M.edge: 1.  At
    // 20.005 none of the four reaches.  M.edge: 4.
    for (name, x) in [("edge_exact", 50.2), ("edge_over", 50.205)] {
        write(
            name,
            vec![
                rect(target, 30.0, 30.0, 30.2, 32.0),
                rect(reference, x, 30.0, x + 2.0, 32.0),
            ],
        );
    }

    // --- The hardening patterns: what a rule manual's reach asks of any layer, drawn
    // once here for every deck of every PDK (hardening/SPEC.md).  The value is 20 µm
    // and the tile 20 µm.  A part-scope violation is one gap region; a polygon-scope
    // one, one region out of reach.

    // The bound, both ways and at the corners.  A 2 µm reference at 30..32 and targets
    // whose near wall is 20 off: right, left, above and below at exactly 20 are in
    // reach, at 20.005 not; diagonally off the corner by 20 in each axis a target is in
    // the square's reach, by 20.005 not; and one 14.15 off in each axis - 20.01 as the
    // crow flies - is covered, the reach being the square and not the circle.  M.poly:
    // 5.  SPC.max reads the far walls too: every 2 µm target whose near wall is at 20
    // has its far half beyond reach, so 10 - only the diagonal one is covered whole.
    write(
        "bound",
        vec![
            rect(reference, 30.0, 30.0, 32.0, 32.0),
            rect(target, 52.0, 30.0, 54.0, 32.0),     // reach
            rect(target, 52.005, 34.0, 54.005, 36.0), // gap
            rect(target, 8.0, 30.0, 10.0, 32.0),      // reach
            rect(target, 7.995, 34.0, 9.995, 36.0),   // gap
            rect(target, 30.0, 52.0, 32.0, 54.0),     // reach
            rect(target, 34.0, 52.005, 36.0, 54.005), // gap
            rect(target, 30.0, 8.0, 32.0, 10.0),      // reach
            rect(target, 34.0, 7.995, 36.0, 9.995),   // gap
            rect(target, 52.0, 52.0, 54.0, 54.0),     // reach, the corner
            rect(target, 52.005, 56.0, 54.005, 58.0), // gap: 20.005 in x
            rect(target, 46.15, 46.15, 48.15, 48.15), // reach, 20.01 as the crow flies
        ],
    );

    // 45° geometry.  A diamond reference 2 µm to the tip at (30, 30): its reach is the
    // diamond grown by 20 along every wall, a diamond 30.28 to the tip, and not its
    // box grown - a target at (51.5, 51.5), 43 from the centre by the diamond's own
    // metric, is a gap though the box's corner grown would cover it; one at (42.5,
    // 42.5), 29 by that metric, is covered.  Against a 2 µm square reference at (30,
    // 80), a diamond target with its near tip 20 off is in reach as a polygon and has
    // its far half beyond reach as parts; with the tip at 20.005 it is out as a
    // polygon.  Against a 45° strip reference from (30, 120) to (42, 132), a box whose
    // corner is 19.997 from the strip's lower wall, perpendicular, is in reach as a
    // polygon and has parts beyond; one at 20.004 is out (the grid has nothing between
    // 19.997 and 20.0006 on a 45° wall, and the reach of a polygon is a DBU more than
    // the value, so a touch at it counts).  M.poly: 3; SPC.max: 5.
    write(
        "bound_45",
        vec![
            diamond(reference, 30.0, 30.0, 2.0),
            rect(target, 51.5, 51.5, 53.5, 53.5), // gap, M.poly
            rect(target, 42.5, 42.5, 44.5, 44.5), // reach
            rect(reference, 29.0, 79.0, 31.0, 81.0),
            diamond(target, 53.0, 80.0, 2.0), // gap (the far half), in reach as a polygon
            diamond(target, 53.005, 86.0, 2.0), // gap, M.poly
            strip45(reference, 30.0, 120.0, 12.0, 1.0),
            rect(target, 46.14, 105.86, 48.14, 107.86), // gap (the far part), in reach as a polygon
            rect(target, 52.14, 111.85, 54.14, 113.85), // gap, M.poly
        ],
    );

    // Shapes that merge.  A reference drawn as two overlapping boxes reaches as their
    // union; a target from 34 to 60 drawn as thirteen 2 µm slices is one polygon with
    // one gap past 52; a 2 µm ring target 20..80 round a 2 µm reference at its centre
    // has its band 26 off and is one gap and one polygon out of reach; a 2 µm
    // reference ring 30..40 round a target at its centre covers it.  SPC.max: 2;
    // M.poly: 1.
    let mut e = vec![
        rect(reference, 30.0, 30.0, 31.5, 32.0),
        rect(reference, 30.5, 30.0, 32.0, 32.0),
    ];
    for i in 0..13 {
        let x = 34.0 + 2.0 * i as f64;
        e.push(rect(target, x, 30.0, x + 2.0, 32.0)); // one gap, 52..60
    }
    e.extend([
        rect(reference, 149.0, 149.0, 151.0, 151.0),
        rect(target, 120.0, 120.0, 180.0, 122.0),
        rect(target, 120.0, 178.0, 180.0, 180.0),
        rect(target, 120.0, 122.0, 122.0, 178.0),
        rect(target, 178.0, 122.0, 180.0, 178.0), // gap, M.poly
        rect(reference, 230.0, 230.0, 240.0, 232.0),
        rect(reference, 230.0, 238.0, 240.0, 240.0),
        rect(reference, 230.0, 232.0, 232.0, 238.0),
        rect(reference, 238.0, 232.0, 240.0, 238.0),
        rect(target, 234.0, 234.0, 236.0, 236.0), // reach
    ]);
    write("merge", e);

    // Tile lines.  A reference ending on x = 20 and a target starting on 40, exactly
    // the value apart across a tile, is in reach; starting at 40.005 not; a target
    // from 39 to 60 is covered to 40 and a gap past it, one gap; a reference across
    // x = 20 and a target across 40, 19 apart; a reference ending on 21 with a target
    // starting on 41 and one on 41.005; a reference across y = 20 with a target
    // across y = 42, 20.5 apart; a pair at (1000, 1000) 21 apart.  Each row 30 µm up
    // from the last, out of the others' reach; the 2 µm targets at exactly 20 have
    // their far half beyond reach.  SPC.max: 7; M.poly: 4.
    write(
        "tile_lines",
        vec![
            rect(reference, 18.0, 2.0, 20.0, 4.0),
            rect(target, 40.0, 2.0, 42.0, 4.0), // reach as a polygon, far half a gap
            rect(reference, 18.0, 32.0, 20.0, 34.0),
            rect(target, 40.005, 32.0, 42.005, 34.0), // gap, M.poly
            rect(reference, 18.0, 62.0, 20.0, 64.0),
            rect(target, 39.0, 62.0, 60.0, 64.0), // one gap 40..60
            rect(reference, 19.0, 92.0, 21.0, 94.0),
            rect(target, 39.0, 92.0, 41.0, 94.0), // reach, 18 apart
            rect(reference, 19.0, 122.0, 21.0, 124.0),
            rect(target, 41.0, 122.0, 43.0, 124.0), // reach as a polygon, far half a gap
            rect(reference, 19.0, 152.0, 21.0, 154.0),
            rect(target, 41.005, 152.0, 43.005, 154.0), // gap, M.poly
            rect(reference, 100.0, 19.0, 102.0, 21.0),
            rect(target, 100.0, 41.5, 102.0, 43.5), // gap, M.poly, 20.5 apart across y = 42
            rect(reference, 1000.0, 1000.0, 1002.0, 1002.0),
            rect(target, 1023.0, 1000.0, 1025.0, 1002.0), // gap, M.poly
        ],
    );

    // Fifty reference-target pairs 21 apart, at a pitch of 44 so no other cell's
    // reference comes within 20, flat and as an array reference.  SPC.max: 50; M.poly:
    // 50.
    let cell = vec![
        rect(reference, 0.0, 0.0, 1.0, 1.0),
        rect(target, 22.0, 0.0, 23.0, 1.0),
    ];
    write("array_flat", flat_array(&cell, 10, 5, 44.0));
    write_gz(
        &format!("{DIR}/array_ref.gds.gz"),
        ref_array(cell, 10, 5, 44.0),
    );

    // Small and long.  A 0.005 sliver 21 off the reference is a gap; a 300 µm bar with
    // a reference at its middle is covered 20 either way and a gap at each end; a
    // target at (1000, 1000) with nothing near it.  SPC.max: 4; M.poly: 2.
    write(
        "extremes",
        vec![
            rect(reference, 30.0, 30.0, 32.0, 32.0),
            rect(target, 53.0, 30.0, 53.005, 32.0), // gap, M.poly
            rect(reference, 179.0, 60.0, 181.0, 62.0),
            rect(target, 30.0, 62.0, 330.0, 64.0), // two gaps
            rect(target, 1000.0, 1000.0, 1002.0, 1002.0), // gap, M.poly
        ],
    );

    // Every edge, across the tile line: a 2 µm square of Outer across x = 20 and the
    // reference 20 from its right wall - the right, top and bottom walls reach, the
    // left wall is 22 off.  M.edge: 1.
    write(
        "edge_tile_lines",
        vec![
            rect(target, 19.0, 30.0, 21.0, 32.0),
            rect(reference, 41.0, 30.0, 43.0, 32.0),
        ],
    );
}
