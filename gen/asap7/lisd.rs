// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! LISD patterns (DRM 3.9). A source/drain local interconnect line is 24 nm wide and
//! at least 648 nm², 18 nm from the next side to side, 25 nm tip to side and 27 nm
//! tip to tip - in SRAM too, where only the tip-to-tip space is read - and an ordinary
//! line does not touch the SRAM marker. The classes are the edge lengths: a side over
//! 36.25 nm is long, a tip under it short, and a tip of 24 to 36.25 nm mid. Each
//! pattern leads with the shape exactly at the limit, then the one a DBU past it.

use super::patterns::{ROOM, boxes, bx, gap_pairs, stacked_pairs};
use super::{Corpus, DBU};

const OFFSET: f64 = 6990.0;
/// A line's run: past the long class, with room to spare.
const RUN: f64 = 100.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let l = c.layer("LISD");
    let marker = c.layer("SRAMDRC");
    let w = c.drm("LISD.W.1", "min_width", 24.0);

    let narrow = [(w, RUN), (w - DBU, RUN), (RUN, w - DBU)];
    c.write(
        "lisd",
        "LISD.W.1.narrow",
        true,
        boxes(l, &narrow, OFFSET, OFFSET),
    );

    // 648 nm² is 24 by 27, so the short line keeps its width.
    let area = c.drm_area("LISD.A.1", "min_area", 648.0);
    let small = [(w, area / w), (w, area / w - DBU)];
    c.write(
        "lisd",
        "LISD.A.1.small",
        true,
        boxes(l, &small, OFFSET, OFFSET),
    );

    // Two sides; a tip under the middle of a side; two mid tips, 30 nm wide.
    let s1 = c.drm("LISD.S.1", "min_space", 18.0);
    let pairs = gap_pairs(l, (w, RUN), true, &[s1, s1 - DBU], OFFSET, OFFSET);
    c.write("lisd", "LISD.S.1.close", true, pairs);
    let (tip, side) = ((w, RUN), (RUN, w));
    let s2 = c.drm("LISD.S.2", "min_space", 25.0);
    let gaps = [s2, s2 - DBU];
    let pairs = stacked_pairs(l, tip, side, (RUN - w) / 2.0, &gaps, ROOM, OFFSET, OFFSET);
    c.write("lisd", "LISD.S.2.close", true, pairs);
    let s3 = c.drm("LISD.S.3", "min_space", 27.0);
    let mid = (30.0, RUN);
    let gaps = [s3, s3 - DBU];
    let pairs = stacked_pairs(l, mid, mid, 0.0, &gaps, ROOM, OFFSET, OFFSET);
    c.write("lisd", "LISD.S.3.close", true, pairs);

    // In SRAM, two 24 nm tips, under the marker.
    let s4 = c.drm("SRAM.LISD.S.4", "min_space", 27.0);
    let gaps = [s4, s4 - DBU];
    let mut elems = stacked_pairs(l, tip, tip, 0.0, &gaps, ROOM, OFFSET, OFFSET);
    elems.push(bx(marker, OFFSET - 100.0, OFFSET - 100.0, 400.0, 1200.0));
    c.write("lisd", "SRAM.LISD.S.4.close", true, elems);

    // A line clear of the marker, one abutting it along an edge - no shared area, so
    // an ordinary line that touches SRAM - and one over it, which is SRAM.
    let mut elems = vec![];
    for (k, x0) in [0.0, 100.0, 150.0].into_iter().enumerate() {
        elems.push(bx(l, OFFSET + x0, OFFSET + k as f64 * (w + ROOM), RUN, w));
    }
    elems.push(bx(
        marker,
        OFFSET + 200.0,
        OFFSET - 100.0,
        400.0,
        3.0 * (w + ROOM),
    ));
    c.write("lisd", "SRAM.LISD.AUX.1.abut", true, elems);
}
