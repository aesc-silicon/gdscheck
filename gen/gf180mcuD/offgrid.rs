// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Off-grid vertices: one good and one bad fixture per rule in the `offgrid` deck.

use super::{GRID, OFFSET, octagon, rules_of};
use crate::helpers::{diamond, layer, library, poly, rect, text, um, write_gz};
use gds21::{GdsArrayRef, GdsDateTime, GdsElement, GdsPoint, GdsStruct, GdsStructRef};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/offgrid";

/// Off-grid shift. 3 nm is not a multiple of the 5 nm grid, and is small enough that
/// nothing about the shape except its grid alignment changes.
const OFF: f64 = 0.003;

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    for (id, l) in rules_of(pdk, "offgrid") {
        // Good: a plain rectangle and an octagon, every vertex a multiple of the grid.
        // The octagon is the load-bearing half — a 45° edge lands its vertices on the
        // grid just as an orthogonal one does, and the check must not care.
        let good = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            octagon(l, OFFSET + 3.0, OFFSET, 2.0, 0.5),
        ];
        write_gz(&format!("{DIR}/{id}.good.gds.gz"), library("TOP", good));

        // Bad: the same rectangle with its right edge pushed 3 nm off the grid.
        let x1 = OFFSET + 3.0;
        let bad = vec![
            rect(l, OFFSET, OFFSET, OFFSET + 1.0, OFFSET + 1.0),
            rect(l, x1, OFFSET, x1 + 1.0 + OFF, OFFSET + 1.0),
        ];
        write_gz(&format!("{DIR}/{id}.bad.gds.gz"), library("TOP", bad));
    }

    // A grid that is not the rule's grid would make every fixture here meaningless.
    assert_eq!(GRID, 0.005, "fixtures assume the 5 nm grid");

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) -------------------
//
// Section 7.1 of the manual is three sentences.  Two of them are this deck: GRID, "the
// design grid must be an integer multiple of 0.005 µm", and OFFGRID, "all edge boundaries
// must be snapped to the grid defined above".  Nothing else - no exemption, no layer left
// out, no tolerance.  Section 3.4 repeats the advice and adds the database unit, 0.001 µm,
// which is the foundry runset's separate `DBU` rule and not a rule of 7.1.
//
// The database unit matters for what can be drawn: at 1 nm the finest off-grid offset is
// 0.001 µm, so "half a grid step", 0.0025, is not a representable vertex.  `OFFGRID.h2`
// walks all four representable offsets instead, and the half-step only appears where the
// engine itself makes one - on a tile line cutting a slanted edge, `OFFGRID.h4`.
//
// The findings are in hardening/reports/gf180mcuD/offgrid.md, the cases in the
// `hardening_offgrid` table of `tests/gf180mcuD.rs`.  The rotation fixture is
// `acute/ACUTE.h5.gds.gz`, drawn by the acute generator and read here for its other half.

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let comp = layer(pdk, "comp");
    let comp_label = layer(pdk, "comp_label");
    let m1_dummy = layer(pdk, "metal1_dummy");
    let m3_slot = layer(pdk, "metal3_slot");

    // On the grid in every way the layer can be: an orthogonal box, an octagon's and a
    // diamond's 45° vertices, a box one grid step wide, a box at (1000, 1000) where a
    // float coordinate has lost its last digits, and a box in the negative quadrant, where
    // a remainder taken without care comes out negative.  Nothing may fire.
    hwrite(
        "OFFGRID.h1",
        vec![
            rect(comp, 10.0, 10.0, 12.0, 12.0),
            octagon(comp, 14.0, 10.0, 2.0, 0.5),
            diamond(comp, 19.0, 11.0, 1.0),
            rect(comp, 22.0, 10.0, 22.005, 10.005),
            rect(comp, 1000.0, 1000.0, 1002.0, 1002.0),
            rect(comp, -20.0, -20.0, -18.0, -18.0),
        ],
    );

    // Every offset the database unit can express: the right edge of a box pushed 1, 2, 3
    // and 4 nm off the grid, and a fifth pushed the whole 5 nm, which lands back on it.
    // Four boxes off, two vertices each: eight markers.
    hwrite(
        "OFFGRID.h2",
        vec![
            rect(comp, 10.0, 10.0, 12.001, 12.0),
            rect(comp, 16.0, 10.0, 18.002, 12.0),
            rect(comp, 22.0, 10.0, 24.003, 12.0),
            rect(comp, 28.0, 10.0, 30.004, 12.0),
            rect(comp, 34.0, 10.0, 36.005, 12.0),
        ],
    );

    // The same 3 nm offset where arithmetic goes wrong: in the negative quadrant, and a
    // thousand microns out where 1002.003 has fewer significant digits left than 12.003.
    // Two vertices each.
    hwrite(
        "OFFGRID.h3",
        vec![
            rect(comp, -12.003, -10.0, -10.0, -8.0),
            rect(comp, 1000.0, 1000.0, 1002.003, 1002.0),
        ],
    );

    // The tile lines, which is where this deck can be made to lie.  (a) is a wedge whose
    // three vertices are all on the grid but whose hypotenuse crosses x = 20 at y =
    // 10.0025: a tile cut there invents a vertex the shape does not have, and half of a
    // grid step is not even representable, so a marker on it would be the engine's own.
    // (b), (c) and (d) are boxes with a genuine 3 nm offset, one straddling x = 20, one
    // straddling x = 42 (a line at 7 µm and not at 20), one ending exactly on x = 20 with
    // its off-grid corner sitting on the line.  Two markers each, six in all, at any tile.
    hwrite(
        "OFFGRID.h4",
        vec![
            poly(comp, &[(19.0, 10.0), (21.0, 10.0), (21.0, 10.005)]),
            rect(comp, 19.0, 12.0, 21.0, 12.003),
            rect(comp, 41.0, 14.0, 43.0, 14.003),
            rect(comp, 18.0, 16.0, 20.0, 16.003),
        ],
    );

    // The placement off the grid, not the shape.  One on-grid 0.5 µm box in a cell, placed
    // three ways: at an on-grid point (clean), at (20.003, 10) - four vertices off - and as
    // a three-column array on an on-grid origin with a 1.002 µm pitch, where the first copy
    // is on the grid and the other two are not.  Twelve markers.
    let cell = vec![rect(comp, 0.0, 0.0, 0.5, 0.5)];
    let top = vec![
        GdsElement::GdsStructRef(GdsStructRef {
            name: "CELL".into(),
            xy: GdsPoint::new(um(10.0), um(10.0)),
            ..Default::default()
        }),
        GdsElement::GdsStructRef(GdsStructRef {
            name: "CELL".into(),
            xy: GdsPoint::new(um(20.003), um(10.0)),
            ..Default::default()
        }),
        GdsElement::GdsArrayRef(GdsArrayRef {
            name: "CELL".into(),
            xy: [
                GdsPoint::new(um(30.0), um(10.0)),
                GdsPoint::new(um(30.0 + 3.0 * 1.002), um(10.0)),
                GdsPoint::new(um(30.0), um(10.0 + 2.0)),
            ],
            cols: 3,
            rows: 1,
            ..Default::default()
        }),
    ];
    let mut lib = library("TOP", top);
    let mut child = GdsStruct::new("CELL");
    child.elems = cell;
    lib.structs.insert(0, child);
    lib.set_all_dates(GdsDateTime::from(&[0i16, 1, 1, 0, 0, 0]));
    write_gz(&format!("{DIR}/OFFGRID.h5.gds.gz"), lib);

    // A label layer carrying a text at an off-grid point, and a boundary on the same layer
    // on the grid.  A text has no edge boundary, so the manual's sentence has nothing to
    // bite on: clean.
    hwrite(
        "OFFGRID.h6",
        vec![
            text(comp_label, "A", 10.003, 10.003),
            rect(comp_label, 12.0, 10.0, 14.0, 12.0),
        ],
    );

    // Two drawn layers of the PDK that no rule of either deck names, each with a 3 nm
    // offset: dummy Metal1 and slotted Metal3.  Two vertices each.
    hwrite(
        "OFFGRID.h7",
        vec![
            rect(m1_dummy, 10.0, 10.0, 12.003, 12.0),
            rect(m3_slot, 20.0, 10.0, 22.003, 12.0),
        ],
    );

    // What merging does to an off-grid vertex.  The first pair is a box with an off-grid
    // box laid across it so that the off-grid edge sticks out: the union still has the two
    // off-grid vertices.  The second is an off-grid box wholly inside an on-grid one - the
    // union is the on-grid box, and nothing of the off-grid one reaches a boundary that
    // gets manufactured.  Two markers, not four.
    hwrite(
        "OFFGRID.h8",
        vec![
            rect(comp, 10.0, 10.0, 12.0, 12.0),
            rect(comp, 11.0, 10.5, 13.003, 11.5),
            rect(comp, 20.0, 10.0, 22.0, 12.0),
            rect(comp, 20.5, 10.5, 21.003, 11.0),
        ],
    );
}
