// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Select and threshold-adjust patterns (DRM 3.7): NSELECT, PSELECT, SLVT, LVT and
//! SRAMVT, one rule set stated for NSELECT and read on all five. A select is drawn in
//! rows like the well, 108 nm across and 54 nm along; it holds an ordinary ACTIVE by
//! 46 nm across and 27 nm along, an SRAM ACTIVE by 13.5 nm either way, and runs 7 nm
//! past a gate either way. NSELECT and PSELECT may not overlap, nor may two VT layers.
//! Each pattern leads with the shape exactly at the limit, then the one a DBU past it.

use super::patterns::{ROOM, boxes, bx};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// An ACTIVE bar, a gate's width, and a select's extent where nothing reads it.
const ACTIVE: (f64, f64) = (100.0, 27.0);
const GATE: f64 = 20.0;
const LONG: f64 = 200.0;
const TALL: f64 = 100.0;

pub(super) fn generate(c: &Corpus<'_>) {
    for name in ["NSELECT", "PSELECT", "SLVT", "LVT", "SRAMVT"] {
        layer(c, name);
    }
    overlaps(c);
}

/// The eight rules of one layer.
fn layer(c: &Corpus<'_>, name: &str) {
    let l = c.layer(name);
    let active = c.layer("ACTIVE");
    let gate = c.layer("GATE");
    let marker = c.layer("SRAMDRC");
    let id = |r: &str| format!("{name}.{r}");
    let row = |k: usize| OFFSET + k as f64 * (TALL + 60.0 + ROOM);
    let (aw, ah) = ACTIVE;

    let w = c.drm(&id("W.1"), "min_width", 108.0);
    let h = c.drm(&id("W.2"), "min_width", 54.0);
    let narrow = [(w, TALL), (w - DBU, TALL)];
    c.write(
        "select",
        &id("W.1.narrow"),
        true,
        boxes(l, &narrow, OFFSET, OFFSET),
    );
    let short = [(LONG, h), (LONG, h - DBU)];
    c.write(
        "select",
        &id("W.2.short"),
        true,
        boxes(l, &short, OFFSET, OFFSET),
    );

    // An ACTIVE held by `left` across and `below` along, the other margins ample; in
    // SRAM, under the marker.
    let held = |y: f64, left: f64, below: f64, sram: bool| -> Vec<GdsElement> {
        let mut e = vec![
            bx(active, OFFSET + left, y + below, aw, ah),
            bx(l, OFFSET, y, left + aw + 60.0, below + ah + 40.0),
        ];
        if sram {
            e.push(bx(
                marker,
                OFFSET - 50.0,
                y - 50.0,
                left + aw + 160.0,
                below + ah + 140.0,
            ));
        }
        e
    };
    for (r, value, sram) in [("ACTIVE.EN.1", 46.0, false), ("ACTIVE.EN.3", 13.5, true)] {
        let rid = if sram {
            format!("SRAM.{}", id(r))
        } else {
            id(r)
        };
        let en = c.drm(&rid, "min_enclosure", value);
        let mut elems = held(row(0), en, 40.0, sram);
        elems.extend(held(row(1), en - DBU, 40.0, sram));
        c.write("select", &format!("{rid}.short"), true, elems);
    }
    for (r, value, sram) in [("ACTIVE.EN.2", 27.0, false), ("ACTIVE.EN.4", 13.5, true)] {
        let rid = if sram {
            format!("SRAM.{}", id(r))
        } else {
            id(r)
        };
        let en = c.drm(&rid, "min_enclosure", value);
        let mut elems = held(row(0), 60.0, en, sram);
        elems.extend(held(row(1), 60.0, en - DBU, sram));
        c.write("select", &format!("{rid}.short"), true, elems);
    }

    // A gate crossing the select top to bottom, 7 nm in from its left wall, then a DBU
    // less: the extension across. Then a gate ending inside it 7 nm under its top wall,
    // running out of its bottom: the extension along.
    let ex1 = c.drm(&id("GATE.EX.1"), "min_enclosure", 7.0);
    let crossing = |y: f64, left: f64| {
        vec![
            bx(l, OFFSET, y, LONG, TALL),
            bx(gate, OFFSET + left, y - 30.0, GATE, TALL + 60.0),
        ]
    };
    let mut elems = crossing(row(0), ex1);
    elems.extend(crossing(row(1), ex1 - DBU));
    c.write("select", &id("GATE.EX.1.short"), true, elems);
    let ex2 = c.drm(&id("GATE.EX.2"), "min_enclosure", 7.0);
    let ending = |y: f64, top: f64| {
        vec![
            bx(l, OFFSET, y, LONG, TALL),
            bx(gate, OFFSET + 90.0, y - 30.0, GATE, 30.0 + TALL - top),
        ]
    };
    let mut elems = ending(row(0), ex2);
    elems.extend(ending(row(1), ex2 - DBU));
    c.write("select", &id("GATE.EX.2.short"), true, elems);
}

/// NSELECT against PSELECT, and each VT layer against the other two: abutting is
/// allowed, sharing area is not.
fn overlaps(c: &Corpus<'_>) {
    let pair = |a: (i16, i16), b: (i16, i16), y: f64, overlap: f64| {
        vec![
            bx(a, OFFSET, y, 108.0, 54.0),
            bx(b, OFFSET + 108.0 - overlap, y, 108.0, 54.0),
        ]
    };
    let row = |k: usize| OFFSET + k as f64 * (54.0 + ROOM);
    let (n, p) = (c.layer("NSELECT"), c.layer("PSELECT"));
    let mut elems = pair(n, p, row(0), 0.0);
    elems.extend(pair(n, p, row(1), 10.0));
    c.write("select", "NSELECT.PSELECT.AUX.1.overlap", true, elems);

    let (lvt, slvt, sramvt) = (c.layer("LVT"), c.layer("SLVT"), c.layer("SRAMVT"));
    let mut elems = pair(lvt, slvt, row(0), 0.0);
    for (k, (a, b)) in [(lvt, slvt), (lvt, sramvt), (slvt, sramvt)]
        .into_iter()
        .enumerate()
    {
        elems.extend(pair(a, b, row(k + 1), 10.0));
    }
    c.write("select", "VT.AUX.2.overlap", true, elems);
}
