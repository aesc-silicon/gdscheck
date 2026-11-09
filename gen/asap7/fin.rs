// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! FIN patterns (DRM 3.3). A fin runs along x, exactly 7 nm across, centred on a 27 nm
//! pitch - 13.5 nm past a multiple of 27, as the standard cells have them. Each pattern
//! leads with the fin exactly at the limit, then the ones one DBU past it.
//!
//! A fin a DBU narrow or wide, or jogged by a DBU, has its centre half a DBU off the
//! pitch, so W.1's and AUX.1's defects also fail S.1; the tests name and ignore it.

use super::patterns::{boxes_apart, bx, pg};
use super::{Corpus, DBU};

const OFFSET: f64 = 6990.0;
/// The fins' lower walls: 10 nm past a multiple of 27, the pitch's frame.
const Y: f64 = 6976.0;
const RUN: f64 = 200.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let l = c.layer("FIN");
    let w = c.drm("FIN.W.1", "exact_width", 7.0);
    let pitch = c.drm("FIN.S.1", "offtrack", 27.0);
    assert_eq!(
        c.drm_param("FIN.S.1", "offtrack", "offset", 13.5),
        Y % pitch + w / 2.0,
        "the fins are drawn in the deck's frame"
    );
    // Between stacked fins: nineteen pitches less a fin, so the next fin is on the pitch.
    let apart = 19.0 * pitch - w;

    // Exactly 7 nm across: a DBU under and a DBU over both fail.
    let fins = [(RUN, w), (RUN, w - DBU), (RUN, w + DBU)];
    c.write(
        "fin",
        "FIN.W.1.narrow",
        true,
        boxes_apart(l, &fins, apart, OFFSET, Y),
    );

    let len = c.drm("FIN.W.2", "min_width", 108.0);
    let fins = [(len, w), (len - DBU, w)];
    c.write(
        "fin",
        "FIN.W.2.short",
        true,
        boxes_apart(l, &fins, apart, OFFSET, Y),
    );

    // FIN.S.1: a pair a pitch apart, then a DBU under and a DBU over it - the far fin
    // off the pitch either way, which the space the deck once read could not see.
    let mut elems = vec![];
    for (k, p) in [pitch, pitch - DBU, pitch + DBU].into_iter().enumerate() {
        let y = Y + k as f64 * 20.0 * pitch;
        elems.push(bx(l, OFFSET, y, RUN, w));
        elems.push(bx(l, OFFSET, y + p, RUN, w));
    }
    c.write("fin", "FIN.S.1.pitch", true, elems);

    // A straight fin, then one four pitches up that jogs up a DBU half way along: each
    // half is 7 nm across and longer than 108 nm, so only the bend is wrong - and the
    // jogged fin's centre, half a DBU up, is off the pitch.
    let (x, y) = (OFFSET, Y + 4.0 * pitch);
    let half = 150.0;
    let jog = pg(
        l,
        &[
            (x, y),
            (x + half, y),
            (x + half, y + DBU),
            (x + 2.0 * half, y + DBU),
            (x + 2.0 * half, y + DBU + w),
            (x + half, y + DBU + w),
            (x + half, y + w),
            (x, y + w),
        ],
    );
    let elems = vec![bx(l, x, Y, 2.0 * half, w), jog];
    c.write("fin", "FIN.AUX.1.bent", true, elems);
}
