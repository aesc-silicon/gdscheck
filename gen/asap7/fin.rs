// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! FIN patterns (DRM 3.3). A fin runs along x, exactly 7 nm across. Each pattern
//! leads with the fin exactly at the limit, then the ones one DBU past it.
//!
//! Not drawn here: FIN.S.1's notch. A notch needs a fin that bends, which FIN.AUX.1
//! forbids, and the bend's joint is wider than 7 nm, which FIN.W.1 forbids.

use super::patterns::{boxes, gap_pairs, pg};
use super::{Corpus, DBU};

const OFFSET: f64 = 6990.0;
const RUN: f64 = 200.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let l = c.layer("FIN");
    let w = c.drm("FIN.W.1", "exact_width", 7.0);

    // Exactly 7 nm across: a DBU under and a DBU over both fail.
    let fins = [(RUN, w), (RUN, w - DBU), (RUN, w + DBU)];
    c.write(
        "fin",
        "FIN.W.1.narrow",
        true,
        boxes(l, &fins, OFFSET, OFFSET),
    );

    let len = c.drm("FIN.W.2", "min_width", 108.0);
    let fins = [(len, w), (len - DBU, w)];
    c.write(
        "fin",
        "FIN.W.2.short",
        true,
        boxes(l, &fins, OFFSET, OFFSET),
    );

    // FIN.S.1 is an exact 27 nm pitch; the deck reads the space it leaves, 27 - 7.
    let s = c.drm("FIN.S.1", "min_space", 27.0 - w);
    let pairs = gap_pairs(l, (RUN, w), false, &[s, s - DBU], OFFSET, OFFSET);
    c.write("fin", "FIN.S.1.close", true, pairs);

    // A straight fin, then one that jogs up a DBU half way along: each half is 7 nm
    // across and longer than 108 nm, so only the bend is wrong.
    let (x, y) = (OFFSET, OFFSET);
    let half = 150.0;
    let jog = pg(
        l,
        &[
            (x, y + 100.0),
            (x + half, y + 100.0),
            (x + half, y + 100.0 + DBU),
            (x + 2.0 * half, y + 100.0 + DBU),
            (x + 2.0 * half, y + 100.0 + DBU + w),
            (x + half, y + 100.0 + DBU + w),
            (x + half, y + 100.0 + w),
            (x, y + 100.0 + w),
        ],
    );
    let mut elems = boxes(l, &[(2.0 * half, w)], x, y);
    elems.push(jog);
    c.write("fin", "FIN.AUX.1.bent", true, elems);
}
