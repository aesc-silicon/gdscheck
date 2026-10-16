// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GEOMETRY.NONORTHOGONAL (DRM 3.1): no edge off the horizontal or vertical, read on
//! the ordinary shapes of every drawn layer and on the SRAM marker itself. One square
//! and one 45° chamfer per layer, the chamfer one edge marker each; then the same
//! chamfers under a square marker, which exempts them all, and reads nothing.

use super::Corpus;
use super::patterns::{ROOM, bx, pg};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;

pub(super) fn generate(c: &Corpus<'_>) {
    // The layers the deck reads, each resolved to the drawn layer under it: a rule
    // names `M1NoSram`, defined as `M1 not_overlapping SRAMDRC`, or the marker itself.
    let layers: Vec<String> = c
        .rules
        .iter()
        .filter(|r| r.id == "GEOMETRY.NONORTHOGONAL" && r.check == "no_angle")
        .map(|r| {
            let name = &r.layers[0].name;
            c.pdk
                .virtual_layers
                .iter()
                .find(|v| &v.name == name)
                .map_or_else(|| name.clone(), |v| v.layers[0].clone())
        })
        .collect();
    assert!(
        layers.iter().any(|l| l == "SRAMDRC"),
        "the marker is always read"
    );

    // A 100 by 54 square at (x, y), and the same with its top-right corner cut at 45°.
    let square = |l: (i16, i16), x: f64, y: f64| bx(l, x, y, 100.0, 54.0);
    let chamfered = |l: (i16, i16), x: f64, y: f64| {
        pg(
            l,
            &[
                (x, y),
                (x + 100.0, y),
                (x + 100.0, y + 34.0),
                (x + 80.0, y + 54.0),
                (x, y + 54.0),
            ],
        )
    };

    let mut elems: Vec<GdsElement> = vec![];
    let mut y = OFFSET;
    for name in &layers {
        let l = c.layer(name);
        elems.push(square(l, OFFSET, y));
        elems.push(chamfered(l, OFFSET + 200.0, y));
        y += 54.0 + ROOM;
    }
    c.write("geometry", "GEOMETRY.NONORTHOGONAL.chamfer", true, elems);

    let mut elems: Vec<GdsElement> = vec![];
    let mut y = OFFSET;
    for name in layers.iter().filter(|n| *n != "SRAMDRC") {
        elems.push(chamfered(c.layer(name), OFFSET, y));
        y += 54.0 + ROOM;
    }
    let marker = c.layer("SRAMDRC");
    elems.push(bx(
        marker,
        OFFSET - 100.0,
        OFFSET - 100.0,
        300.0,
        y - OFFSET + 100.0,
    ));
    c.write("geometry", "GEOMETRY.NONORTHOGONAL.sram", false, elems);
}
