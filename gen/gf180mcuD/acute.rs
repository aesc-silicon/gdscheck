// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Angles off the 45° lattice: one good and one bad fixture per rule in the `acute` deck.

use super::{OFFSET, octagon, rules_of};
use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, um, write_gz};
use gds21::{GdsDateTime, GdsElement, GdsLibrary, GdsPoint, GdsStrans, GdsStruct, GdsStructRef};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/acute";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    for (id, l) in rules_of(pdk, "acute") {
        // Good: a rectangle and an octagon. Between them they use every legal
        // orientation — 0°, 45°, 90° and 135° — so a check that allowed only right
        // angles would fail here rather than passing quietly.
        let good = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            octagon(l, OFFSET + 3.0, OFFSET, 2.0, 0.5),
        ];
        write_gz(&format!("{DIR}/{id}.good.gds.gz"), library("TOP", good));

        // Bad: a right triangle with legs 2 and 1, so its hypotenuse runs at 26.57° —
        // far enough off 0° and 45° that no tolerance excuses it, and a slope a real
        // layout could plausibly contain. Its vertices stay on the grid, so this fixture
        // is clean under the off-grid deck and the two cannot be confused.
        let (x, y) = (OFFSET + 3.0, OFFSET);
        let bad = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            poly(l, &[(x, y), (x + 2.0, y), (x, y + 1.0)]),
        ];
        write_gz(&format!("{DIR}/{id}.bad.gds.gz"), library("TOP", bad));
    }

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Layouts drawn from section 7.1 of the manual, whose whole text for this deck is one
// sentence: "All shapes must be orthogonal or on a 45° unless otherwise stated."  That
// sentence is about the *direction of an edge*, not about the angle between two of them -
// a shape all of whose edges run at 0°, 45°, 90° or 135° satisfies it even where two of
// those edges meet in an acute corner.  The foundry's own runset reads it the same way
// (`edges.without_angle(0).without_angle(45).without_angle(90).without_angle(-45)`), so
// the 45° wedge of `ACUTE.h2` is legal geometry with an acute angle in it: the rule's name
// is the industry's, not the rule's content.
//
// The findings are in hardening/reports/gf180mcuD/acute.md, the cases in the
// `hardening_acute` table of `tests/gf180mcuD.rs`.  `ACUTE.h5` is shared with the
// `offgrid` deck - a rotation takes an on-grid orthogonal box off the grid, and the two
// decks read that one file from opposite sides.

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// A right triangle at `(x0, y0)` with legs `dx` and `dy`: the bottom edge is horizontal,
/// the right edge vertical, and the hypotenuse - the only edge under test - runs back to
/// the origin at `atan(dy/dx)`.  At most one edge per wedge can be illegal, so a marker
/// count is a wedge count.
fn wedge(l: (i16, i16), x0: f64, y0: f64, dx: f64, dy: f64) -> GdsElement {
    poly(l, &[(x0, y0), (x0 + dx, y0), (x0 + dx, y0 + dy)])
}

/// A library with one `CELL` holding `cell`, placed by the references in `top`.
fn lib_with_cell(cell: Vec<GdsElement>, top: Vec<GdsElement>) -> GdsLibrary {
    let mut lib = library("TOP", top);
    let mut child = GdsStruct::new("CELL");
    child.elems = cell;
    lib.structs.insert(0, child);
    lib.set_all_dates(GdsDateTime::from(&[0i16, 1, 1, 0, 0, 0]));
    lib
}

/// `CELL` placed at `(x, y)` and turned by `angle` degrees about that point.
fn placed(x: f64, y: f64, angle: Option<f64>) -> GdsElement {
    GdsElement::GdsStructRef(GdsStructRef {
        name: "CELL".into(),
        xy: GdsPoint::new(um(x), um(y)),
        strans: angle.map(|a| GdsStrans {
            reflected: false,
            abs_mag: false,
            abs_angle: false,
            mag: None,
            angle: Some(a),
        }),
        ..Default::default()
    })
}

fn hardening(pdk: &PdkConfig) {
    let comp = layer(pdk, "comp");
    let m1_dummy = layer(pdk, "metal1_dummy");
    let m3_slot = layer(pdk, "metal3_slot");

    // Every legal orientation at once: a box (0° and 90°), an octagon and a diamond (45°
    // and 135°), a chamfer whose two endpoints are on the grid, and a box drawn with a
    // redundant vertex in the middle of its top edge - a 180° turn, which is a multiple of
    // 45 and must not be called an angle.  Nothing here may fire.
    hwrite(
        "ACUTE.h1",
        vec![
            rect(comp, 10.0, 10.0, 12.0, 12.0),
            octagon(comp, 14.0, 10.0, 2.0, 0.5),
            diamond(comp, 19.0, 11.0, 1.0),
            chamfered_tr(comp, 24.0, 10.0, 26.0, 12.0, 37.5),
            poly(
                comp,
                &[
                    (30.0, 10.0),
                    (32.0, 10.0),
                    (32.0, 12.0),
                    (31.0, 12.0),
                    (30.0, 12.0),
                ],
            ),
        ],
    );

    // Acute corners whose edges are all on the 45° lattice: a wedge with a 45° tip, an
    // arrowhead with two 45° base corners, and a plate with a 45° V-notch cut into its top
    // edge.  The manual constrains edges, not corners, so all three are legal - this is
    // the fixture that says which of the two readings the deck has.
    hwrite(
        "ACUTE.h2",
        vec![
            wedge(comp, 10.0, 10.0, 4.0, 4.0),
            poly(comp, &[(18.0, 10.0), (22.0, 10.0), (20.0, 12.0)]),
            poly(
                comp,
                &[
                    (26.0, 10.0),
                    (32.0, 10.0),
                    (32.0, 14.0),
                    (30.0, 14.0),
                    (29.0, 13.0),
                    (28.0, 14.0),
                    (26.0, 14.0),
                ],
            ),
        ],
    );

    // Five wedges whose hypotenuse misses the lattice: 26.57°, 45.14°, 44.86°, 89.9° and
    // 0.1°.  The two beside 45° and the two beside 0°/90° are the interesting ones - a
    // check with a tolerance would let them pass.  Five illegal edges, five markers.
    hwrite(
        "ACUTE.h3",
        vec![
            wedge(comp, 10.0, 10.0, 2.0, 1.0),
            wedge(comp, 16.0, 10.0, 1.0, 1.005),
            wedge(comp, 22.0, 10.0, 1.005, 1.0),
            wedge(comp, 28.0, 10.0, 0.005, 2.86),
            wedge(comp, 34.0, 10.0, 2.86, 0.005),
        ],
    );

    // The tile lines.  The same 26.57° wedge four times: well inside the first tile, across
    // x = 20 (the default tile line), across x = 42 (a tile line at 7 µm), and across
    // y = 20.  A cut splits the slanted edge into two collinear halves and gives the halves
    // new endpoints; neither may add a marker nor lose one.  The box at x = 50 straddles
    // both cuts orthogonally and must stay clean.
    hwrite(
        "ACUTE.h4",
        vec![
            wedge(comp, 2.0, 10.0, 2.0, 1.0),
            wedge(comp, 19.0, 10.0, 2.0, 1.0),
            wedge(comp, 41.0, 10.0, 2.0, 1.0),
            wedge(comp, 30.0, 19.0, 4.0, 2.0),
            rect(comp, 50.0, 19.0, 52.0, 21.0),
        ],
    );

    // Placement, not shape: one on-grid box in a cell, placed three times - as drawn, at
    // 45° and at 30°.  A 45° turn keeps every edge on the lattice (legal here, though it
    // takes the vertices off the grid - see the offgrid deck); a 30° turn puts all four
    // edges at 30° and 120°, which is four illegal edges.  This is the case the rule
    // exists for.
    write_gz(
        &format!("{DIR}/ACUTE.h5.gds.gz"),
        lib_with_cell(
            vec![rect(comp, 0.0, 0.0, 1.0, 1.0)],
            vec![
                placed(10.0, 10.0, None),
                placed(20.0, 10.0, Some(45.0)),
                placed(30.0, 10.0, Some(30.0)),
            ],
        ),
    );

    // Two drawn layers of the PDK that no rule of either deck names: dummy Metal1 and
    // slotted Metal3.  The manual says "all shapes"; these are shapes.
    hwrite(
        "ACUTE.h6",
        vec![
            wedge(m1_dummy, 10.0, 10.0, 2.0, 1.0),
            wedge(m3_slot, 20.0, 10.0, 2.0, 1.0),
        ],
    );

    // One shape for both decks: a wedge whose hypotenuse is off the lattice *and* whose two
    // far vertices are off the grid.  One ACUTE for the edge, two OFFGRID for the vertices
    // - each deck reads its own half of the same polygon.
    hwrite(
        "ACUTE.h7",
        vec![poly(comp, &[(10.0, 10.0), (12.003, 10.0), (10.0, 11.002)])],
    );

    // How far off 45° an edge has to be before it is off 45°.  Eleven wedges one micron
    // wide whose height climbs from 1.005 to 1.600, so their hypotenuse leans 45.14°,
    // 45.29°, 45.57°, 46.12°, 47.73°, 50.19°, 54.46° and 58.00° - deviations of 0.14, 0.29,
    // 0.57, 1.12, 2.73, 5.19, 9.46 and 13.00 degrees - with three more, four microns wide
    // so the grid can still hold them, at 45.85°, 45.98° and 46.02°, which brackets the
    // last of those gaps to a thirtieth of a degree.  Every one of them is a slope a layout
    // can be drawn to on this grid, every vertex is on the grid, and none of them is 45°,
    // so the manual condemns all eleven.
    hwrite(
        "ACUTE.h8",
        vec![
            wedge(comp, 10.0, 10.0, 1.0, 1.005),
            wedge(comp, 16.0, 10.0, 1.0, 1.010),
            wedge(comp, 22.0, 10.0, 1.0, 1.020),
            wedge(comp, 28.0, 10.0, 1.0, 1.040),
            wedge(comp, 34.0, 10.0, 1.0, 1.100),
            wedge(comp, 40.0, 10.0, 1.0, 1.200),
            wedge(comp, 46.0, 10.0, 1.0, 1.400),
            wedge(comp, 52.0, 10.0, 1.0, 1.600),
            wedge(comp, 58.0, 10.0, 4.0, 4.120),
            wedge(comp, 64.0, 10.0, 4.0, 4.140),
            wedge(comp, 70.0, 10.0, 4.0, 4.145),
        ],
    );
}
