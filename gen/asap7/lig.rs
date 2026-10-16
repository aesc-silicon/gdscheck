// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! LIG patterns (DRM 3.10). A gate local interconnect line is 16 nm wide and at least
//! 324 nm², spaced from the next by the lengths of the facing edges as LISD is, 14 nm
//! from a LISD or SDT on another net - 15 corner to corner - 14 nm above or below an
//! uncut gate and 17 nm beside one, 5 nm from a channel or a GCUT, and over a gate it
//! runs 1 nm past both walls and shares 320 nm² with it; over a LISD it reaches 8 nm in
//! and shares 128 nm². In SRAM it reaches 15 nm into a gate and shares 240 nm². Each
//! pattern leads with the shape exactly at the limit, then the one a DBU past it.
//!
//! Some rules overlap by their nature: a 16 nm LIG reaching under 8 nm into a LISD
//! shares under 128 nm² with it, and a LIG ending inside a gate shares under 320 nm²
//! with it. The tests name those and ignore them.

use super::patterns::{ROOM, boxes, bx, gap_pairs, stacked_pairs};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// A line's run, past the long edge class; a gate's width; a LISD's width.
const RUN: f64 = 100.0;
const GATE: f64 = 20.0;
const LISD: f64 = 24.0;

pub(super) fn generate(c: &Corpus<'_>) {
    let lig = c.layer("LIG");
    let lisd = c.layer("LISD");
    let sdt = c.layer("SDT");
    let gate = c.layer("GATE");
    let gcut = c.layer("GCUT");
    let active = c.layer("ACTIVE");
    let marker = c.layer("SRAMDRC");
    let (v0, m1) = (c.layer("V0"), c.layer("M1"));
    let w = c.drm("LIG.W.1", "min_width", 16.0);
    let row = |k: usize| OFFSET + k as f64 * (RUN + ROOM);
    let write = |name: &str, elems: Vec<GdsElement>| c.write("lig", name, true, elems);
    let under_marker = |mut elems: Vec<GdsElement>| {
        elems.push(bx(marker, OFFSET - 200.0, OFFSET - 200.0, 600.0, 1400.0));
        elems
    };

    let narrow = [(w, RUN), (w - DBU, RUN), (RUN, w - DBU)];
    write("LIG.W.1.narrow", boxes(lig, &narrow, OFFSET, OFFSET));
    // 324 nm² is 16 by 20.25, so the short line keeps its width.
    let area = c.drm_area("LIG.A.1", "min_area", 324.0);
    let small = [(w, area / w), (w, area / w - DBU)];
    write("LIG.A.1.small", boxes(lig, &small, OFFSET, OFFSET));

    // Two sides; a tip under the middle of a side; then tip to tip: mid and mid,
    // tiny and tiny, mid and tiny. A 16 nm tip is tiny, a 30 nm one mid.
    let (tip, side, mid) = ((w, RUN), (RUN, w), (30.0, RUN));
    let s1 = c.drm("LIG.S.1", "min_space", 18.0);
    write(
        "LIG.S.1.close",
        gap_pairs(lig, tip, true, &[s1, s1 - DBU], OFFSET, OFFSET),
    );
    for (r, limit, lower, upper, dx) in [
        ("S.2", 25.0, tip, side, (RUN - w) / 2.0),
        ("S.3", 27.0, mid, mid, 0.0),
        ("S.4", 31.0, tip, tip, 0.0),
        ("S.5", 31.0, mid, tip, 0.0),
    ] {
        let s = c.drm(&format!("LIG.{r}"), "min_space", limit);
        let gaps = [s, s - DBU];
        let pairs = stacked_pairs(lig, lower, upper, dx, &gaps, ROOM, OFFSET, OFFSET);
        write(&format!("LIG.{r}.close"), pairs);
    }

    // A LISD side 14 nm over a LIG side, then a DBU less; then the same gap between
    // a pair joined through V0 and M1, one net, which the rule leaves alone.
    let s6 = c.drm("LIG.LISD.S.6", "min_space", 14.0);
    let mut elems = vec![];
    for (k, (g, joined)) in [(s6, false), (s6 - DBU, false), (s6 - DBU, true)]
        .into_iter()
        .enumerate()
    {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(lig, x, y, RUN, w));
        elems.push(bx(lisd, x, y + w + g, RUN, LISD));
        if joined {
            let top = w + g + LISD;
            elems.push(bx(v0, x + 40.0, y, 18.0, w));
            elems.push(bx(v0, x + 40.0, y + w + g, 18.0, 16.0));
            elems.push(bx(m1, x + 40.0, y, 18.0, top));
        }
    }
    write("LIG.LISD.S.6.close", elems);

    // Corner to corner on another net: 9 and 12 nm clear is exactly 15 nm.
    let s7 = c.drm("LIG.LISD.S.7", "min_space", 15.0);
    assert_eq!(9.0_f64.hypot(12.0), s7);
    let mut elems = vec![];
    for (k, dy) in [12.0, 12.0 - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(lig, x, y, RUN, w));
        elems.push(bx(lisd, x + RUN + 9.0, y + w + dy, RUN, LISD));
    }
    write("LIG.LISD.S.7.corner", elems);

    // An SDT, its own net, 14 nm over a LIG side.
    let s8 = c.drm("LIG.SDT.S.8", "min_space", 14.0);
    let mut elems = vec![];
    for (k, g) in [s8, s8 - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(lig, x, y, RUN, w));
        elems.push(bx(sdt, x + 38.0, y + w + g, LISD, 27.0));
    }
    write("LIG.SDT.S.8.close", elems);

    // An uncut gate 100 nm tall, and a LIG 14 nm over its top end; then one ending
    // 17 nm short of its side.
    let s9a = c.drm("LIG.GATE.S.9A", "min_space", 14.0);
    let mut elems = vec![];
    for (k, g) in [s9a, s9a - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(gate, x + 40.0, y - RUN, GATE, RUN));
        elems.push(bx(lig, x, y + g, RUN, w));
    }
    write("LIG.GATE.S.9A.close", elems);
    let s9b = c.drm("LIG.GATE.S.9B", "min_space", 17.0);
    let mut elems = vec![];
    for (k, g) in [s9b, s9b - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(gate, x + RUN + g, y - 40.0, GATE, RUN));
        elems.push(bx(lig, x, y, RUN, w));
    }
    write("LIG.GATE.S.9B.close", elems);

    // A channel: a gate ending in an ACTIVE, so the channel's top corner is the gate's.
    // A LIG off that corner by 3 and 4 nm, exactly 5, then by 3 and 3.75, faces the
    // gate nowhere, so only the channel's 5 nm reads it.
    let s10 = c.drm("LIG.GATE.S.10", "min_space", 5.0);
    assert_eq!(3.0_f64.hypot(4.0), s10);
    let mut elems = vec![];
    for (k, dy) in [4.0, 4.0 - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(gate, x, y - RUN, GATE, RUN));
        elems.push(bx(active, x - 30.0, y - 20.0, GATE + 60.0, 40.0));
        elems.push(bx(lig, x + GATE + 3.0, y + dy, RUN, w));
    }
    write("LIG.GATE.S.10.corner", elems);

    // A GCUT across a gate's top end, and a LIG 5 nm over the GCUT: the uncut gate
    // below it is 22 nm away and more.
    let s11 = c.drm("LIG.GCUT.S.11", "min_space", 5.0);
    let mut elems = vec![];
    for (k, g) in [s11, s11 - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(gate, x + 40.0, y - RUN, GATE, RUN));
        elems.push(bx(gcut, x + 40.0 - 17.0, y - 10.0, GATE + 34.0, 17.0));
        elems.push(bx(lig, x, y + 7.0 + g, RUN, w));
    }
    write("LIG.GCUT.S.11.close", elems);

    // Over a gate: a LIG crossing it, 20 nm past its left wall and `r` past its right,
    // 1 nm then a DBU less; a gate 20 nm wide under a 16 nm LIG shares 320 nm² with
    // it, a DBU narrower not.
    let crossing = |y: f64, gw: f64, lw: f64, r: f64| -> Vec<GdsElement> {
        vec![
            bx(gate, OFFSET + 20.0, y - 40.0, gw, RUN),
            bx(lig, OFFSET, y, 20.0 + gw + r, lw),
        ]
    };
    let ex = c.drm("LIG.GATE.EX.1", "min_enclosure", 1.0);
    let mut elems = crossing(row(0), GATE, w, 20.0);
    elems.extend(crossing(row(1), GATE, w, ex));
    elems.extend(crossing(row(2), GATE, w, ex - DBU));
    write("LIG.GATE.EX.1.short", elems);
    let a3 = c.drm_area("LIG.GATE.A.3", "min_area", 320.0);
    assert_eq!(GATE * w, a3);
    let mut elems = crossing(row(0), GATE, w, 20.0);
    elems.extend(crossing(row(1), GATE - DBU, w, 20.0));
    write("LIG.GATE.A.3.small", elems);

    // A LIG ending inside a gate, then one flush with its left wall; the first shares
    // 160 nm² with it. EX.1 reads only a gate wall the LIG covers, so neither reads.
    let mut elems = crossing(row(0), GATE, w, 20.0);
    elems.push(bx(gate, OFFSET + 40.0, row(1) - 40.0, GATE, RUN));
    elems.push(bx(lig, OFFSET, row(1), 50.0, w));
    elems.push(bx(gate, OFFSET + 40.0, row(2) - 40.0, GATE, RUN));
    elems.push(bx(lig, OFFSET, row(2), 40.0, w));
    write("LIG.GATE.AUX.1.inside", elems);

    // Into a LISD: a 20 nm LIG reaching 8 nm in, then a DBU less; a 16 nm LIG reaching
    // 8 nm in shares 128 nm², a DBU less under it, and under the overlap too.
    let landing = |y: f64, lw: f64, reach: f64| -> Vec<GdsElement> {
        vec![
            bx(lisd, OFFSET + RUN, y - 40.0, LISD, RUN),
            bx(lig, OFFSET, y, RUN + reach, lw),
        ]
    };
    let ov1 = c.drm("LIG.LISD.OV.1", "min_overlap", 8.0);
    let mut elems = landing(row(0), 20.0, ov1);
    elems.extend(landing(row(1), 20.0, ov1 - DBU));
    write("LIG.LISD.OV.1.short", elems);
    let a2 = c.drm_area("LIG.LISD.A.2", "min_area", 128.0);
    assert_eq!(w * ov1, a2);
    let mut elems = landing(row(0), w, ov1);
    elems.extend(landing(row(1), w, ov1 - DBU));
    write("LIG.LISD.A.2.small", elems);

    // In SRAM: a 20 nm LIG reaching 15 nm into a gate, then a DBU less; a gate 15 nm
    // wide under a 16 nm LIG shares 240 nm², a DBU narrower not.
    let ov2 = c.drm("SRAM.LIG.GATE.OV.2", "min_overlap", 15.0);
    let into = |y: f64, lw: f64, reach: f64| -> Vec<GdsElement> {
        vec![
            bx(gate, OFFSET + RUN, y - 40.0, GATE, RUN),
            bx(lig, OFFSET, y, RUN + reach, lw),
        ]
    };
    let mut elems = into(row(0), 20.0, ov2);
    elems.extend(into(row(1), 20.0, ov2 - DBU));
    write("SRAM.LIG.GATE.OV.2.short", under_marker(elems));
    let a4 = c.drm_area("SRAM.LIG.GATE.A.4", "min_area", 240.0);
    assert_eq!(ov2 * w, a4);
    let mut elems = crossing(row(0), ov2, w, 20.0);
    elems.extend(crossing(row(1), ov2 - DBU, w, 20.0));
    write("SRAM.LIG.GATE.A.4.small", under_marker(elems));

    // A line clear of the marker, one abutting it along an edge - no shared area, so
    // an ordinary line that touches SRAM - and one over it, which is SRAM.
    let mut elems = vec![];
    for (k, x0) in [0.0, 100.0, 150.0].into_iter().enumerate() {
        elems.push(bx(lig, OFFSET + x0, OFFSET + k as f64 * (w + ROOM), RUN, w));
    }
    elems.push(bx(
        marker,
        OFFSET + 200.0,
        OFFSET - 100.0,
        400.0,
        3.0 * (w + ROOM),
    ));
    write("SRAM.LIG.AUX.2.abut", elems);
}
