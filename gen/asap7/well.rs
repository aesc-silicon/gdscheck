// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! N-well patterns (DRM 3.2). The well is drawn in rows: 108 nm across (x) and 54 nm
//! along (y), 108 nm between rows and 54 nm along one, and 7 nm past a gate either
//! way. Each pattern leads with the shape exactly at the limit, then the one a DBU
//! past it.
//!
//! A well at both minimum widths is at the minimum area, as the manual's note says,
//! so an under-area well is also under a width and an under-area hole is also a
//! notch; the tests name those and ignore them.

use super::patterns::{ROOM, boxes, bx, gap_pairs, pg};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// A well's extent where nothing reads it, and a gate's width.
const LONG: f64 = 200.0;
const TALL: f64 = 100.0;
const GATE: f64 = 20.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let well = c.layer("NWELL");
    let gate = c.layer("GATE");
    let w = c.drm("WELL.W.1", "min_width", 108.0);
    let h = c.drm("WELL.W.2", "min_width", 54.0);
    let rows = c.drm("WELL.S.1", "min_space", 108.0);
    let along = c.drm("WELL.S.2", "min_space", 54.0);

    let narrow = [(w, TALL), (w - DBU, TALL)];
    c.write(
        "well",
        "WELL.W.1.narrow",
        true,
        boxes(well, &narrow, OFFSET, OFFSET),
    );
    let short = [(LONG, h), (LONG, h - DBU)];
    c.write(
        "well",
        "WELL.W.2.short",
        true,
        boxes(well, &short, OFFSET, OFFSET),
    );

    // Two rows, then two wells along one row.
    let pairs = gap_pairs(
        well,
        (LONG, TALL),
        false,
        &[rows, rows - DBU],
        OFFSET,
        OFFSET,
    );
    c.write("well", "WELL.S.1.close", true, pairs);
    let pairs = gap_pairs(
        well,
        (LONG, TALL),
        true,
        &[along, along - DBU],
        OFFSET,
        OFFSET,
    );
    c.write("well", "WELL.S.2.close", true, pairs);

    // A slot between two arms of one well. Open to the right, the arms `h` tall on a
    // back `w` wide, the slot's walls face across y; open upward, the arms `w` wide
    // on a back `h` tall, they face across x. Every arm is at its minimum width.
    let n_rows = c.drm("WELL.S.1", "min_notch", 108.0);
    let mut elems = vec![];
    let mut y = OFFSET;
    for g in [n_rows, n_rows - DBU] {
        let (x, top) = (OFFSET, 2.0 * h + g);
        elems.push(pg(
            well,
            &[
                (x, y),
                (x + 300.0, y),
                (x + 300.0, y + h),
                (x + w, y + h),
                (x + w, y + h + g),
                (x + 300.0, y + h + g),
                (x + 300.0, y + top),
                (x, y + top),
            ],
        ));
        y += top + ROOM;
    }
    c.write("well", "WELL.S.1.notch", true, elems);

    let n_along = c.drm("WELL.S.2", "min_notch", 54.0);
    let mut elems = vec![];
    let mut y = OFFSET;
    for g in [n_along, n_along - DBU] {
        let (x, right) = (OFFSET, 2.0 * w + g);
        elems.push(pg(
            well,
            &[
                (x, y),
                (x + right, y),
                (x + right, y + 300.0),
                (x + w + g, y + 300.0),
                (x + w + g, y + h),
                (x + w, y + h),
                (x + w, y + 300.0),
                (x, y + 300.0),
            ],
        ));
        y += 300.0 + ROOM;
    }
    c.write("well", "WELL.S.2.notch", true, elems);

    // The minimum well, then one a DBU narrower: under the area, and under W.1 too.
    let area = c.drm_area("WELL.A.1A", "min_area", 5832.0);
    assert_eq!(
        w * h,
        area,
        "DRM 3.2 note 1: the minimum well is the minimum area"
    );
    c.write(
        "well",
        "WELL.A.1A.small",
        true,
        boxes(well, &[(w, h), (w - DBU, h)], OFFSET, OFFSET),
    );

    // A ring of minimum arms around the minimum hole - 54 nm along, 108 nm between
    // rows - then around one a DBU shorter: under the area, and a notch under S.1 too.
    let hole = c.drm_area("WELL.A.1B", "min_area", 5832.0);
    assert_eq!(
        along * rows,
        hole,
        "the minimum hole is the minimum notch both ways"
    );
    let ring = |x: f64, y: f64, hw: f64, hh: f64| -> Vec<GdsElement> {
        vec![
            bx(well, x, y, 2.0 * w + hw, h),
            bx(well, x, y + h + hh, 2.0 * w + hw, h),
            bx(well, x, y + h, w, hh),
            bx(well, x + w + hw, y + h, w, hh),
        ]
    };
    let mut elems = ring(OFFSET, OFFSET, along, rows);
    elems.extend(ring(
        OFFSET,
        OFFSET + 2.0 * h + rows + ROOM,
        along,
        rows - DBU,
    ));
    c.write("well", "WELL.A.1B.hole", true, elems);

    // A gate crossing the well top to bottom, 7 nm in from its left wall, then a DBU
    // less: the extension across. The ends that run out of the well are not read.
    let ex1 = c.drm("WELL.GATE.EX.1", "min_enclosure", 7.0);
    let crossing = |y: f64, l: f64| {
        vec![
            bx(well, OFFSET, y, LONG, TALL),
            bx(gate, OFFSET + l, y - 30.0, GATE, TALL + 60.0),
        ]
    };
    let mut elems = crossing(OFFSET, ex1);
    elems.extend(crossing(OFFSET + TALL + ROOM, ex1 - DBU));
    c.write("well", "WELL.GATE.EX.1.short", true, elems);

    // A gate ending inside the well 7 nm under its top wall, then a DBU less, running
    // out of its bottom: the extension along.
    let ex2 = c.drm("WELL.GATE.EX.2", "min_enclosure", 7.0);
    let ending = |y: f64, t: f64| {
        vec![
            bx(well, OFFSET, y, LONG, TALL),
            bx(gate, OFFSET + 90.0, y - 30.0, GATE, 30.0 + TALL - t),
        ]
    };
    let mut elems = ending(OFFSET, ex2);
    elems.extend(ending(OFFSET + TALL + ROOM, ex2 - DBU));
    c.write("well", "WELL.GATE.EX.2.short", true, elems);
}
