// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GATE patterns (DRM 3.4). A gate runs along y, exactly 20 nm across, on a 54 nm
//! pitch. GATE.S.3 asks every gate for a neighbour within that pitch, so every gate
//! here comes with a partner 34 nm beside it. Each pattern leads with the shape exactly
//! at the limit, then the ones one DBU past it.
//!
//! Not drawn here: GATE.S.2's notch. A notch needs a gate that bends, which GATE.AUX.1
//! forbids, and the bend's joint is wider than 20 nm, which GATE.W.1 forbids.

use super::patterns::{ROOM, bx, pg};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// A gate's height where nothing reads it, past GATE.W.2's 40 nm.
const TALL: f64 = 100.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let gate = c.layer("GATE");
    let active = c.layer("ACTIVE");
    let w = c.drm("GATE.W.1", "exact_width", 20.0);
    let s = c.drm("GATE.S.2", "min_space", 34.0);
    let pitch = w + s;
    // A `gw` x `gh` gate at (x, y), with its partner one pitch to the right.
    let pair = |x: f64, y: f64, gw: f64, gh: f64| {
        vec![bx(gate, x, y, gw, gh), bx(gate, x + gw + s, y, w, gh)]
    };
    let row = |k: usize| OFFSET + k as f64 * (TALL + ROOM);

    // Exactly 20 nm across: a DBU under and a DBU over both fail.
    let mut elems = vec![];
    for (k, gw) in [w, w - DBU, w + DBU].into_iter().enumerate() {
        elems.extend(pair(OFFSET, row(k), gw, TALL));
    }
    c.write("gate", "GATE.W.1.narrow", true, elems);

    let h = c.drm("GATE.W.2", "min_width", 40.0);
    let mut elems = pair(OFFSET, row(0), w, h);
    elems.extend(pair(OFFSET, row(1), w, h - DBU));
    c.write("gate", "GATE.W.2.short", true, elems);

    // The pitch less a DBU: the pair is still a pair under S.3, but too close.
    let mut elems = vec![];
    for (k, gap) in [s, s - DBU].into_iter().enumerate() {
        elems.push(bx(gate, OFFSET, row(k), w, TALL));
        elems.push(bx(gate, OFFSET + w + gap, row(k), w, TALL));
    }
    c.write("gate", "GATE.S.2.close", true, elems);

    // GATE.S.3: a pair one pitch apart, a gate alone, and a pair a DBU too far apart
    // for either to count as the other's neighbour.
    let mut elems = pair(OFFSET, row(0), w, TALL);
    elems.push(bx(gate, OFFSET, row(1), w, TALL));
    elems.push(bx(gate, OFFSET, row(2), w, TALL));
    elems.push(bx(gate, OFFSET + w + s + DBU, row(2), w, TALL));
    c.write("gate", "GATE.S.3.solitary", true, elems);

    // A straight gate, then one that jogs a DBU half way up; each half is 20 nm across
    // and 50 nm tall, so only the bend is wrong. Its partner clears the jog by the pitch.
    let mut elems = pair(OFFSET, row(0), w, TALL);
    let (x, y) = (OFFSET, row(1));
    elems.push(pg(
        gate,
        &[
            (x, y),
            (x + w, y),
            (x + w, y + TALL / 2.0),
            (x + w + DBU, y + TALL / 2.0),
            (x + w + DBU, y + TALL),
            (x + DBU, y + TALL),
            (x + DBU, y + TALL / 2.0),
            (x, y + TALL / 2.0),
        ],
    ));
    elems.push(bx(gate, x + w + DBU + s, y, w, TALL));
    c.write("gate", "GATE.AUX.1.bent", true, elems);

    // Against ACTIVE: a pair crossing one ACTIVE `ah` tall, the gates running `ex` past
    // it below and above, and the ACTIVE `ax` past the outer gate walls.
    let ex1 = c.drm("GATE.ACTIVE.EX.1", "min_enclosure", 4.0);
    let ex2 = c.drm("GATE.ACTIVE.EX.2", "min_enclosure", 25.0);
    let ah = 54.0;
    let device = |y: f64, below: f64, above: f64, left: f64, right: f64| -> Vec<GdsElement> {
        let x = OFFSET + left;
        let mut e = pair(x, y - below, w, below + ah + above);
        e.push(bx(active, OFFSET, y, left + pitch + w + right, ah));
        e
    };
    let mut elems = device(row(0), ex1, ex1, ex2, ex2);
    elems.extend(device(row(1), ex1 - DBU, ex1, ex2, ex2));
    c.write("gate", "GATE.ACTIVE.EX.1.short", true, elems);

    let mut elems = device(row(0), ex1, ex1, ex2, ex2);
    elems.extend(device(row(1), ex1, ex1, ex2 - DBU, ex2));
    c.write("gate", "GATE.ACTIVE.EX.2.short", true, elems);

    // ACTIVE's side inside a gate, then flush with it: its end then runs no way past
    // the gate, which GATE.ACTIVE.EX.2 also reads.
    let mut elems = device(row(0), ex1, ex1, ex2, ex2);
    elems.extend(device(row(1), ex1, ex1, w / 2.0 - w, ex2));
    elems.extend(device(row(2), ex1, ex1, 0.0, ex2));
    c.write("gate", "GATE.ACTIVE.AUX.3.inside", true, elems);

    // A pair off ACTIVE, the nearer gate 9 nm from the ACTIVE's side, then a DBU less.
    let g = c.drm("GATE.ACTIVE.S.4", "min_space", 9.0);
    let mut elems = vec![];
    for (k, gap) in [g, g - DBU].into_iter().enumerate() {
        let y = row(k);
        elems.push(bx(active, OFFSET, y, 100.0, ah));
        elems.extend(pair(OFFSET + 100.0 + gap, y, w, TALL));
    }
    c.write("gate", "GATE.ACTIVE.S.4.close", true, elems);
}
