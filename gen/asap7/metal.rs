// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Width, space, area and edge patterns for M1-M9 (DRM 3.12-3.18). Each pattern leads
//! with the shape exactly at the limit, then one DBU past it.
//!
//! Not drawn here: the rules each deck lists as not checked (M4-M7 even track counts,
//! tip-to-tip runs on adjacent tracks, the routing grid).

use super::patterns::{ROOM, boxes, boxes_apart, bx, corner_pairs, gap_pairs, pg, stacked_pairs};
use super::{Corpus, DBU};

const OFFSET: f64 = 6990.0;
/// Wire run: past both long-edge classes (36.25 nm on M1-M3, 80 nm on M8/M9) and
/// short of M8/M9's first length gate (400 nm).
const RUN: f64 = 200.0;

pub(super) fn generate(c: &Corpus<'_>) {
    for n in 1..=3 {
        lower(c, n);
    }
    // Evens route horizontally, odds vertically.
    for (n, across) in [(4, 24.0), (5, 24.0), (6, 32.0), (7, 32.0)] {
        routing(c, n, across, n % 2 == 0);
    }
    for n in 8..=9 {
        upper(c, n);
    }
}

/// M1-M3 (DRM 3.12): width in either direction, the spacings by the length class of
/// the two facing edges, the corner spacing, and area. The classes: long edges run at
/// least 36.25 nm, short ones less; of the short, mid ones run at least 24 nm and tiny
/// ones less. An 18 nm wire's tip is tiny, a 30 nm one's mid, and either's side long.
fn lower(c: &Corpus<'_>, n: u8) {
    let deck = format!("m{n}");
    let l = c.layer(&format!("M{n}"));
    let id = |r: &str| format!("M{n}.{r}");

    let w = c.drm(&id("W.1"), "min_width", 18.0);
    let narrow = [(w, RUN), (w - DBU, RUN), (RUN, w - DBU)];
    c.write(
        &deck,
        &id("W.1.narrow"),
        true,
        boxes(l, &narrow, OFFSET, OFFSET),
    );

    // Two sides.
    let s = c.drm(&id("S.1"), "min_space", 18.0);
    let pairs = gap_pairs(l, (w, RUN), true, &[s, s - DBU], OFFSET, OFFSET);
    c.write(&deck, &id("S.1.close"), true, pairs);

    // A tip under the middle of a side, then tip to tip: mid and mid, tiny and tiny,
    // mid and tiny.
    let mid = 30.0;
    let (tip, side) = ((w, RUN), (RUN, w));
    for (r, limit, lower, upper, dx) in [
        ("S.2", 25.0, tip, side, (RUN - w) / 2.0),
        ("S.3", 27.0, (mid, RUN), (mid, RUN), 0.0),
        ("S.4", 31.0, tip, tip, 0.0),
        ("S.5", 31.0, (mid, RUN), tip, 0.0),
    ] {
        let s = c.drm(&id(r), "min_space", limit);
        let pairs = stacked_pairs(l, lower, upper, dx, &[s, s - DBU], ROOM, OFFSET, OFFSET);
        c.write(&deck, &id(&format!("{r}.close")), true, pairs);
    }

    // Corner to corner: 12 and 16 nm clear is exactly 20 nm, 12 and 15.75 under it.
    let s = c.drm(&id("S.6"), "min_space", 20.0);
    assert_eq!(12.0_f64.hypot(16.0), s);
    let pairs = corner_pairs(l, tip, &[(12.0, 16.0), (12.0, 16.0 - DBU)], OFFSET, OFFSET);
    c.write(&deck, &id("S.6.corner"), true, pairs);

    let a = c.drm_area(&id("A.1"), "min_area", 504.0);
    let small = [(w, a / w), (w, a / w - DBU)];
    c.write(
        &deck,
        &id("A.1.small"),
        true,
        boxes(l, &small, OFFSET, OFFSET),
    );
}

/// M4-M7 (DRM 3.14-3.17): width and space across the track and along it, the corner
/// spacing, the notch both ways, the maximum width, the even widths and the bend.
/// `across` is the track width; `horizontal` says the layer routes along x.
fn routing(c: &Corpus<'_>, n: u8, across: f64, horizontal: bool) {
    let deck = format!("m{n}");
    let l = c.layer(&format!("M{n}"));
    let id = |r: &str| format!("M{n}.{r}");
    // A wire `run` long along the routing direction and `wide` across it.
    let wire = |run: f64, wide: f64| if horizontal { (run, wide) } else { (wide, run) };
    // A point drawn for a horizontal layer, turned for a vertical one and placed.
    let at = |(a, b): (f64, f64)| {
        let (x, y) = if horizontal { (a, b) } else { (b, a) };
        (OFFSET + x, OFFSET + y)
    };

    let w = c.drm(&id("W.1"), "min_width", across);
    let narrow = [wire(RUN, w), wire(RUN, w - DBU)];
    c.write(
        &deck,
        &id("W.1.narrow"),
        true,
        boxes(l, &narrow, OFFSET, OFFSET),
    );

    let len = c.drm(&id("W.5"), "min_width", 44.0);
    let short = [wire(len, w), wire(len - DBU, w)];
    c.write(
        &deck,
        &id("W.5.short"),
        true,
        boxes(l, &short, OFFSET, OFFSET),
    );

    // The widest wire, which is also an even multiple of the track; then the even
    // widths themselves, each after an odd one that is legal.
    let max = c.drm(&id("W.2"), "max_width", 20.0 * across);
    let wide = [wire(RUN, max), wire(RUN, max + DBU)];
    c.write(
        &deck,
        &id("W.2.wide"),
        true,
        boxes(l, &wide, OFFSET, OFFSET),
    );
    let even = [3.0, 2.0, 5.0, 4.0].map(|k| wire(RUN, k * w));
    c.write(
        &deck,
        &id("W.3.even"),
        true,
        boxes(l, &even, OFFSET, OFFSET),
    );

    // Neighbouring tracks face across the routing direction, tips along it.
    let side = c.drm(&id("S.1"), "min_space", across);
    let pairs = gap_pairs(
        l,
        wire(RUN, w),
        !horizontal,
        &[side, side - DBU],
        OFFSET,
        OFFSET,
    );
    c.write(&deck, &id("S.1.close"), true, pairs);

    let tip = c.drm(&id("S.2"), "min_space", 40.0);
    let pairs = gap_pairs(
        l,
        wire(RUN, w),
        horizontal,
        &[tip, tip - DBU],
        OFFSET,
        OFFSET,
    );
    c.write(&deck, &id("S.2.close"), true, pairs);

    // Corner to corner: 24 and 32 nm clear is exactly 40 nm, 24 and 31.75 under it.
    let corner = c.drm(&id("S.3"), "min_space", 40.0);
    assert_eq!(24.0_f64.hypot(32.0), corner);
    let offsets = [(24.0, 32.0), (24.0, 32.0 - DBU)];
    let pairs = corner_pairs(l, wire(RUN, w), &offsets, OFFSET, OFFSET);
    c.write(&deck, &id("S.3.corner"), true, pairs);

    // A slot whose walls face across the track: two `w` arms with the gap between,
    // cut from the far end half way back.
    let s = c.drm(&id("S.1"), "min_notch", across);
    let mut y = 0.0;
    let notches = [s, s - DBU].map(|g| {
        let h = 2.0 * w + g;
        let p = [
            (0.0, y),
            (RUN, y),
            (RUN, y + w),
            (RUN / 2.0, y + w),
            (RUN / 2.0, y + w + g),
            (RUN, y + w + g),
            (RUN, y + h),
            (0.0, y + h),
        ];
        y += h + ROOM;
        pg(l, &p.map(at))
    });
    c.write(&deck, &id("S.1.notch"), true, notches.to_vec());

    // A slot whose walls face along the track: two `a` long arms with the gap between,
    // cut half way into a block `h` deep. 100 nm deep keeps every wall clear of the
    // even widths.
    let s = c.drm(&id("S.2"), "min_notch", 40.0);
    let (a, h) = (RUN / 2.0, 100.0);
    let mut y = 0.0;
    let notches = [s, s - DBU].map(|g| {
        let p = [
            (0.0, y),
            (2.0 * a + g, y),
            (2.0 * a + g, y + h),
            (a + g, y + h),
            (a + g, y + h / 2.0),
            (a, y + h / 2.0),
            (a, y + h),
            (0.0, y + h),
        ];
        y += h + ROOM;
        pg(l, &p.map(at))
    });
    c.write(&deck, &id("S.2.notch"), true, notches.to_vec());

    // A straight wire, then one that jogs across the track by a DBU half way along:
    // each half is a track wide and longer than the shortest wire, so only the bend
    // is wrong.
    let half = RUN / 2.0;
    let jog = [
        (0.0, 0.0),
        (half, 0.0),
        (half, DBU),
        (2.0 * half, DBU),
        (2.0 * half, DBU + w),
        (half, DBU + w),
        (half, w),
        (0.0, w),
    ];
    let mut elems = boxes(l, &[wire(2.0 * half, w)], OFFSET, OFFSET);
    let lift = |(a, b): (f64, f64)| at((a, b + w + ROOM));
    elems.push(pg(l, &jog.map(lift)));
    c.write(&deck, &id("AUX.3.bent"), true, elems);

    // The routing grid and tracks, from the manual's default offset of 0: every wall
    // across the track on the width's grid, and a track's lower wall every two widths
    // from the origin, so a wire is on a track when its centre is half a width past
    // one. These lie at 6912 nm across the track - a multiple of every pitch here -
    // where OFFSET is on none, and nothing else here is drawn on the grid.
    assert_eq!(c.drm(&id("AUX.1"), "offgrid", across), w);
    assert_eq!(c.drm_param(&id("AUX.1"), "offgrid", "offset", 0.0), 0.0);
    let pitch = c.drm(&id("AUX.2"), "offtrack", 2.0 * w);
    assert_eq!(c.drm(&id("W.4"), "offtrack", 2.0 * w), pitch);
    for r in ["AUX.2", "W.4"] {
        c.drm_param(&id(r), "offtrack", "offset", w / 2.0);
    }
    let base = 6912.0;
    assert_eq!(base % pitch, 0.0);
    // A wire RUN long along the track and `wide` across it, its lower wall at `lo`.
    let track = |lo: f64, wide: f64| {
        let (bw, bh) = wire(RUN, wide);
        let (x, y) = if horizontal {
            (OFFSET, lo)
        } else {
            (lo, OFFSET)
        };
        bx(l, x, y, bw, bh)
    };

    // A minimum-width wire on a track and a three-width wire centred on one; then a
    // wire a nanometre over the width, centred on a track but with both walls off the
    // grid: four vertices.
    let elems = vec![
        track(base, w),
        track(base + 21.0 * w, 3.0 * w),
        track(base + 40.0 * w - 0.5, w + 1.0),
    ];
    c.write(&deck, &id("AUX.1.offgrid"), true, elems);

    // A minimum-width wire on a track, then one a width up, in the space between
    // tracks: on the grid, off the tracks.
    let elems = vec![track(base, w), track(base + 21.0 * w, w)];
    c.write(&deck, &id("AUX.2.offtrack"), true, elems);

    // A three-width wire centred on a track, then one with its lower wall on a track,
    // which spans two of them.
    let elems = vec![
        track(base + 21.0 * w, 3.0 * w),
        track(base + 40.0 * w, 3.0 * w),
    ];
    c.write(&deck, &id("W.4.span"), true, elems);
}

/// M8/M9 (DRM 3.18/3.19): the base width, the three length-gated tiers and the
/// maximum; the spacings by edge class (long edges run at least 80 nm) and by the
/// width of the wider line; area; and edge length.
fn upper(c: &Corpus<'_>, n: u8) {
    let deck = format!("m{n}");
    let l = c.layer(&format!("M{n}"));
    let id = |r: &str| format!("M{n}.{r}");

    let w = c.drm(&id("W.1"), "min_width", 40.0);
    let narrow = [(w, RUN), (w - DBU, RUN), (RUN, w - DBU)];
    c.write(
        &deck,
        &id("W.1.narrow"),
        true,
        boxes(l, &narrow, OFFSET, OFFSET),
    );

    // A wire at least `length` long needs the tier's width. The deck writes each gate
    // one DBU short; the last wire, one DBU short of the gate, falls to the tier below.
    for (r, width, length) in [
        ("W.2", 60.0, 400.0),
        ("W.3", 80.0, 1200.0),
        ("W.4", 120.0, 1800.0),
    ] {
        let width = c.drm(&id(r), "min_width", width);
        let length = c.drm_param(&id(r), "min_width", "length", length - DBU) + DBU;
        let tiers = [
            (length, width),
            (length, width - DBU),
            (length - DBU, width - DBU),
        ];
        c.write(
            &deck,
            &id(&format!("{r}.narrow")),
            true,
            boxes(l, &tiers, OFFSET, OFFSET),
        );
    }

    // Long enough to need W.4's 120 nm, which they clear, and far enough apart to clear
    // S.8's 1000 nm between two lines that wide.
    let max = c.drm(&id("W.5"), "max_width", 2000.0);
    let wide = [(2200.0, max), (2200.0, max + DBU)];
    let wide = boxes_apart(l, &wide, 1500.0, OFFSET, OFFSET);
    c.write(&deck, &id("W.5.wide"), true, wide);

    // Two sides; a tip under the middle of a side; tip to tip.
    let s = c.drm(&id("S.1"), "min_space", 40.0);
    let pairs = gap_pairs(l, (w, RUN), true, &[s, s - DBU], OFFSET, OFFSET);
    c.write(&deck, &id("S.1.close"), true, pairs);
    let (tip, side) = ((w, RUN), (RUN, w));
    for (r, limit, upper, dx) in [
        ("S.2", 43.0, side, (RUN - w) / 2.0),
        ("S.3", 46.0, tip, 0.0),
    ] {
        let s = c.drm(&id(r), "min_space", limit);
        let pairs = stacked_pairs(l, tip, upper, dx, &[s, s - DBU], ROOM, OFFSET, OFFSET);
        c.write(&deck, &id(&format!("{r}.close")), true, pairs);
    }

    // A line exactly as wide as each gate, under a minimum-width wire. The deck writes
    // each gate one DBU short, so that it reads "at least". The line runs `RUN` past
    // its width: a line is as wide as its narrower side, so a 200 nm run 1000 nm deep
    // would be a 200 nm line.
    for (r, limit) in [
        ("S.4", 60.0),
        ("S.5", 80.0),
        ("S.6", 120.0),
        ("S.7", 500.0),
        ("S.8", 1000.0),
    ] {
        let s = c.drm(&id(r), "min_space", limit);
        let width = c.drm_param(&id(r), "min_space", "width", limit - DBU) + DBU;
        let gaps = [s, s - DBU];
        let pairs = stacked_pairs(
            l,
            (width + RUN, width),
            side,
            0.0,
            &gaps,
            s + ROOM,
            OFFSET,
            OFFSET,
        );
        c.write(&deck, &id(&format!("{r}.close")), true, pairs);
    }

    let a = c.drm_area(&id("A.1"), "min_area", 7520.0);
    let small = [(w, a / w), (w, a / w - DBU)];
    c.write(
        &deck,
        &id("A.1.small"),
        true,
        boxes(l, &small, OFFSET, OFFSET),
    );

    // A step `s` high in the top of a wide block: its riser is the short edge.
    let e = c.drm(&id("L.1"), "min_edge_length", 40.0);
    let mut y = OFFSET;
    let steps = [e, e - DBU].map(|s| {
        let x = OFFSET;
        let p = [
            (x, y),
            (x + RUN, y),
            (x + RUN, y + 100.0),
            (x + 100.0, y + 100.0),
            (x + 100.0, y + 100.0 + s),
            (x, y + 100.0 + s),
        ];
        y += 100.0 + s + ROOM;
        pg(l, &p)
    });
    c.write(&deck, &id("L.1.step"), true, steps.to_vec());
}
