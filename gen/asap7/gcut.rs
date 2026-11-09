// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GCUT patterns (DRM 3.6). A GCUT is a bar along x that cuts the gates it crosses:
//! at least 17 nm tall, reaching 17 nm past each gate it cuts, its ends clear of any
//! gate. The next gate one 54 nm pitch along then sits exactly 17 nm from its end,
//! GCUT.GATE.S.2's limit. Each pattern leads with the bar exactly at the limit, then
//! the ones one DBU past it.

use super::patterns::{ROOM, bx, pg};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// Gate width and the space to its neighbour: GATE.W.1 and GATE.S.2.
const GATE: f64 = 20.0;
const SPACE: f64 = 34.0;
/// A gate's height, room for two bars and the channel below them.
const TALL: f64 = 300.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let (gate, gcut, active) = (c.layer("GATE"), c.layer("GCUT"), c.layer("ACTIVE"));
    let h = c.drm("GCUT.W.1", "min_width", 17.0);
    let ex = c.drm("GCUT.GATE.EX.1", "min_enclosure", 17.0);
    let row = |k: usize| OFFSET + k as f64 * (TALL + ROOM);
    // A gate at x from y up, TALL high.
    let g = |x: f64, y: f64| bx(gate, x, y, GATE, TALL);
    // A bar `bh` tall at height `at` over a gate at x, reaching `l` and `r` past it.
    let bar = |x: f64, at: f64, bh: f64, l: f64, r: f64| bx(gcut, x - l, at, GATE + l + r, bh);
    // The cut gate at x and a bar across it at height `at`.
    let cut = |x: f64, y: f64, at: f64, bh: f64, l: f64, r: f64| -> Vec<GdsElement> {
        vec![g(x, y), bar(x, y + at, bh, l, r)]
    };

    let mut elems = cut(OFFSET, row(0), 100.0, h, ex, ex);
    elems.extend(cut(OFFSET, row(1), 100.0, h - DBU, ex, ex));
    c.write("gcut", "GCUT.W.1.narrow", true, elems);

    let mut elems = cut(OFFSET, row(0), 100.0, h, ex, ex);
    elems.extend(cut(OFFSET, row(1), 100.0, h, ex - DBU, ex));
    c.write("gcut", "GCUT.GATE.EX.1.short", true, elems);

    // The neighbouring gate one pitch along, then the bar a DBU longer toward it.
    let s2 = c.drm("GCUT.GATE.S.2", "min_space", 17.0);
    assert_eq!(SPACE - ex, s2);
    let mut elems = vec![];
    for (k, r) in [ex, ex + DBU].into_iter().enumerate() {
        elems.extend(cut(OFFSET, row(k), 100.0, h, ex, r));
        elems.push(g(OFFSET + GATE + SPACE, row(k)));
    }
    c.write("gcut", "GCUT.GATE.S.2.close", true, elems);

    // A channel, the gate over an ACTIVE, with the bar 4 nm above it, then a DBU less.
    let s1 = c.drm("GCUT.ACTIVE.S.1", "min_space", 4.0);
    let mut elems = vec![];
    for (k, gap) in [s1, s1 - DBU].into_iter().enumerate() {
        let y = row(k);
        elems.push(bx(active, OFFSET - 30.0, y + 20.0, GATE + 60.0, 54.0));
        elems.extend(cut(OFFSET, y, 20.0 + 54.0 + gap, h, ex, ex));
    }
    c.write("gcut", "GCUT.ACTIVE.S.1.close", true, elems);

    // Two bars across one gate, 35 nm apart, then a DBU closer.
    let s3 = c.drm("GCUT.S.3", "min_space", 35.0);
    let mut elems = vec![];
    for (k, gap) in [s3, s3 - DBU].into_iter().enumerate() {
        elems.extend(cut(OFFSET, row(k), 100.0, h, ex, ex));
        elems.push(bar(OFFSET, row(k) + 100.0 + h + gap, h, ex, ex));
    }
    c.write("gcut", "GCUT.S.3.close", true, elems);

    // The same two bars joined past the gate's right side: a slot 35 nm tall, then a
    // DBU less, between two arms of one GCUT.
    let s3n = c.drm("GCUT.S.3", "min_notch", 35.0);
    let mut elems = vec![];
    for (k, gap) in [s3n, s3n - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k) + 100.0);
        let (x0, x1) = (x - ex, x + GATE + ex);
        let joint = x1 + 20.0;
        elems.push(g(x, row(k)));
        elems.push(pg(
            gcut,
            &[
                (x0, y),
                (joint, y),
                (joint, y + 2.0 * h + gap),
                (x0, y + 2.0 * h + gap),
                (x0, y + h + gap),
                (x1, y + h + gap),
                (x1, y + h),
                (x0, y + h),
            ],
        ));
    }
    c.write("gcut", "GCUT.S.3.notch", true, elems);

    // GCUT.AUX.1: a bar across a gate, then one alone.
    let mut elems = cut(OFFSET, row(0), 100.0, h, ex, ex);
    elems.push(bar(OFFSET, row(1) + 100.0, h, ex, ex));
    c.write("gcut", "GCUT.AUX.1.alone", true, elems);

    // GCUT.AUX.2: a bar ending inside a gate, then one ending flush with its side. Its
    // end then reaches no way past the gate, which GCUT.GATE.EX.1 also reads.
    let mut elems = cut(OFFSET, row(0), 100.0, h, ex, ex);
    elems.extend(cut(OFFSET, row(1), 100.0, h, ex, -GATE / 2.0));
    elems.extend(cut(OFFSET, row(2), 100.0, h, ex, 0.0));
    c.write("gcut", "GCUT.AUX.2.inside", true, elems);

    // GCUT.AUX.3: a bar clear of a channel, then one across it.
    let mut elems = vec![];
    for (k, at) in [100.0, 40.0].into_iter().enumerate() {
        let y = row(k);
        elems.push(bx(active, OFFSET - 30.0, y + 20.0, GATE + 60.0, 54.0));
        elems.extend(cut(OFFSET, y, at, h, ex, ex));
    }
    c.write("gcut", "GCUT.AUX.3.channel", true, elems);
}
