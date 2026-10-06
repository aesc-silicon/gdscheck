// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ACTIVE patterns (DRM 3.5). An ACTIVE is a bar along x, at least 27 nm tall and
//! 16 nm wide, 27 nm from the next row and 38 nm from the next along it, 10 nm past
//! the fins it holds, under a select clear of its edges, and at least 864 nm² - 432
//! in SRAM, where the widths do not apply. Each pattern leads with the shape exactly
//! at the limit, then the one a DBU past it.
//!
//! The deck reads AUX.3, no notch along the vertical axis, as a notch under 1 µm, so
//! any hole in an ordinary ACTIVE is one; and a bar under 27 nm or a hole under the
//! area is no multiple of 27 nm, so W.1's short bar and A.1B's hole fail W.2 too. The
//! tests name those and ignore them; every other ACTIVE here is drawn to W.2.

use super::patterns::{ROOM, boxes, bx, gap_pairs, pg};
use super::{Corpus, DBU, nm};
use crate::helpers::space_pattern;
use gds21::GdsElement;

/// ACTIVE side - two fin pitches, for W.2 - room between enclosure pairs, and where
/// each pattern starts, in nm.
const SIDE: f64 = 54.0;
const GAP: f64 = 100.0;
const OFFSET: f64 = 6990.0;
/// A bar's extent where nothing reads it; the height four fin pitches, for W.2.
const LONG: f64 = 100.0;
const TALL: f64 = 108.0;

pub(super) fn generate(c: &Corpus<'_>) {
    well(c);
    shapes(c);
    increments(c);
    latch_up(c);
}

/// DRM 3.5: the entire ACTIVE polygon gets SRAM rules when it interacts with SRAMDRC,
/// so a crossing marker must not clip it into two rule regions. Each pattern holds
/// neighbours at the limit (clean) and one DBU inside it (violations):
///
/// - `outside`: no marker over the pattern; the ordinary 27 nm rule applies.
/// - `inside`: the marker covers everything; the 13.5 nm SRAM rule applies.
/// - `crossing`: the marker covers the bottom half of each ACTIVE, so the top
///   violation faces unmarked ACTIVE. A clipped fragment would answer to 27 nm there.
fn well(c: &Corpus<'_>) {
    let active = c.layer("ACTIVE");
    let nwell = c.layer("NWELL");
    for (kind, check, normal, sram) in [
        (
            "space",
            "min_space",
            "ACTIVE.WELL.S.4",
            "SRAM.ACTIVE.WELL.S.5",
        ),
        (
            "enclosure",
            "min_enclosure",
            "ACTIVE.WELL.EN.1",
            "SRAM.ACTIVE.WELL.EN.2",
        ),
    ] {
        let normal_nm = c.drm(normal, check, 27.0);
        let sram_nm = c.drm(sram, check, 13.5);
        for (mode, rule, limit) in [
            ("outside", normal, normal_nm),
            ("inside", sram, sram_nm),
            ("crossing", sram, sram_nm),
        ] {
            // Both patterns put their first ACTIVE's bottom edge `bottom` nm above OFFSET.
            let (mut elems, bottom) = if kind == "space" {
                let e = space_pattern(active, nwell, nm(SIDE), nm(limit), nm(OFFSET), -nm(DBU));
                (e, 0.0)
            } else {
                // Five pairs: a well `limit` round a SIDE square of ACTIVE, then one with
                // each wall in turn a DBU in. The shared enclosure_pattern shortens a
                // margin by growing the enclosed shape instead, which would take the
                // ACTIVE off W.2's multiple of 27.
                let outer = SIDE + 2.0 * limit;
                let mut e = vec![];
                let sides = [
                    [0.0; 4],
                    [DBU, 0.0, 0.0, 0.0],
                    [0.0, DBU, 0.0, 0.0],
                    [0.0, 0.0, DBU, 0.0],
                    [0.0, 0.0, 0.0, DBU],
                ];
                for (k, [l, r, b, t]) in sides.into_iter().enumerate() {
                    let x = OFFSET + k as f64 * (outer + GAP);
                    e.push(bx(nwell, x + l, OFFSET + b, outer - l - r, outer - b - t));
                    e.push(bx(active, x + limit, OFFSET + limit, SIDE, SIDE));
                }
                (e, limit)
            };
            let around = [-200.0, -200.0, 2000.0, 300.0];
            let marker = match mode {
                "inside" => around,
                "crossing" => [-200.0, -200.0, 2000.0, bottom + SIDE / 2.0],
                _ => [2500.0, -200.0, 2600.0, 300.0],
            };
            elems.push(c.rect("NSELECT", around, OFFSET));
            elems.push(c.rect("SRAMDRC", marker, OFFSET));
            c.write("active", &format!("{rule}.{mode}"), true, elems);
        }
    }
}

/// The ACTIVE's own width, space, notch, area, fin and select rules.
fn shapes(c: &Corpus<'_>) {
    let active = c.layer("ACTIVE");
    let fin = c.layer("FIN");
    let select = c.layer("NSELECT");
    let marker = c.layer("SRAMDRC");
    // A select over the whole pattern, clear of every ACTIVE edge, as AUX.1 asks; and
    // the SRAM marker over the whole of it, where a pattern is drawn in SRAM.
    let over = |layer: (i16, i16)| bx(layer, OFFSET - 100.0, OFFSET - 100.0, 1600.0, 3200.0);
    let write = |name: &str, mut elems: Vec<GdsElement>| {
        elems.push(over(select));
        c.write("active", name, true, elems);
    };

    let h = c.drm("ACTIVE.W.1", "min_width", 27.0);
    let w = c.drm("ACTIVE.W.3", "min_width", 16.0);
    write(
        "ACTIVE.W.1.short",
        boxes(active, &[(LONG, h), (LONG, h - DBU)], OFFSET, OFFSET),
    );
    write(
        "ACTIVE.W.3.narrow",
        boxes(active, &[(w, TALL), (w - DBU, TALL)], OFFSET, OFFSET),
    );

    // Two rows, then two bars along one row.
    let rows = c.drm("ACTIVE.S.1", "min_space", 27.0);
    let along = c.drm("ACTIVE.S.2B", "min_space", 38.0);
    write(
        "ACTIVE.S.1.close",
        gap_pairs(
            active,
            (LONG, h),
            false,
            &[rows, rows - DBU],
            OFFSET,
            OFFSET,
        ),
    );
    write(
        "ACTIVE.S.2B.close",
        gap_pairs(
            active,
            (50.0, TALL),
            true,
            &[along, along - DBU],
            OFFSET,
            OFFSET,
        ),
    );

    // A slot open upward between two 20 nm arms on a 27 nm back: its walls face
    // across x, S.2B's notch.
    let n_along = c.drm("ACTIVE.S.2B", "min_notch", 38.0);
    let mut elems = vec![];
    let mut y = OFFSET;
    for g in [n_along, n_along - DBU] {
        let (x, right) = (OFFSET, 40.0 + g);
        elems.push(pg(
            active,
            &[
                (x, y),
                (x + right, y),
                (x + right, y + TALL),
                (x + 20.0 + g, y + TALL),
                (x + 20.0 + g, y + 27.0),
                (x + 20.0, y + 27.0),
                (x + 20.0, y + TALL),
                (x, y + TALL),
            ],
        ));
        y += TALL + ROOM;
    }
    write("ACTIVE.S.2B.notch", elems);

    // A plain bar, then one with a slot open to the right between two 27 nm arms on a
    // 20 nm back: its walls face across y, 108 nm apart, which AUX.3 forbids at any
    // distance a cell holds.
    let n_rows = c.drm("ACTIVE.AUX.3", "min_notch", 1000.0);
    assert!(108.0 < n_rows);
    let mut elems = boxes(active, &[(LONG, h)], OFFSET, OFFSET);
    let (x, y) = (OFFSET, OFFSET + h + ROOM);
    let top = 2.0 * h + 108.0;
    elems.push(pg(
        active,
        &[
            (x, y),
            (x + LONG, y),
            (x + LONG, y + h),
            (x + 20.0, y + h),
            (x + 20.0, y + h + 108.0),
            (x + LONG, y + h + 108.0),
            (x + LONG, y + top),
            (x, y + top),
        ],
    ));
    write("ACTIVE.AUX.3.notch", elems);

    // The minimum bar, 27 by 32, then one a DBU shorter: under the area with both
    // widths still legal.
    let area = c.drm_area("ACTIVE.A.1A", "min_area", 864.0);
    let len = area / h;
    assert_eq!(len, 32.0);
    write(
        "ACTIVE.A.1A.small",
        boxes(active, &[(len, h), (len - DBU, h)], OFFSET, OFFSET),
    );

    // A ring of legal arms - 20 nm sides, 30 nm caps - around a hole 48 nm along, past
    // S.2B's notch, and 18 nm tall: the minimum hole, then one a DBU shorter. Either
    // hole is an AUX.3 notch across y.
    let ring = |x: f64, y: f64, hw: f64, hh: f64| -> Vec<GdsElement> {
        let (side, cap) = (20.0, 30.0);
        vec![
            bx(active, x, y, 2.0 * side + hw, cap),
            bx(active, x, y + cap + hh, 2.0 * side + hw, cap),
            bx(active, x, y + cap, side, hh),
            bx(active, x + side + hw, y + cap, side, hh),
        ]
    };
    let hole = c.drm_area("ACTIVE.A.1B", "min_area", 864.0);
    let (hw, hh) = (48.0, hole / 48.0);
    assert!(hw >= along && hh == 18.0);
    let mut elems = ring(OFFSET, OFFSET, hw, hh);
    elems.extend(ring(OFFSET, OFFSET + 60.0 + hh + ROOM, hw, hh - DBU));
    write("ACTIVE.A.1B.hole", elems);

    // In SRAM the widths do not apply, and the minimum area is the minimum bar's 16
    // by 27; nor does any notch rule, so the minimum hole is the same 16 by 27.
    let sram_area = c.drm_area("SRAM.ACTIVE.A.2A", "min_area", 432.0);
    assert_eq!(sram_area, w * h);
    let mut elems = boxes(active, &[(w, h), (w, h - DBU)], OFFSET, OFFSET);
    elems.push(over(marker));
    write("SRAM.ACTIVE.A.2A.small", elems);
    let sram_hole = c.drm_area("SRAM.ACTIVE.A.2B", "min_area", 432.0);
    assert_eq!(sram_hole, w * h);
    let mut elems = ring(OFFSET, OFFSET, w, h);
    elems.extend(ring(OFFSET, OFFSET + 60.0 + h + ROOM, w, h - DBU));
    elems.push(over(marker));
    write("SRAM.ACTIVE.A.2B.hole", elems);

    // A 54 nm ACTIVE holding one 7 nm fin 10 nm up from its bottom wall, then a DBU
    // less. The fin runs on past the ACTIVE both ways, as fins do.
    let ex = c.drm("ACTIVE.FIN.EX.1", "min_enclosure", 10.0);
    let held = |y: f64, below: f64| {
        vec![
            bx(active, OFFSET, y, LONG, 54.0),
            bx(fin, OFFSET - 50.0, y + below, LONG + 100.0, 7.0),
        ]
    };
    let mut elems = held(OFFSET, ex);
    elems.extend(held(OFFSET + 54.0 + ROOM, ex - DBU));
    write("ACTIVE.FIN.EX.1.short", elems);

    // AUX.1: a bar under a select clear of its edges; one the select covers only half
    // of; one whose left wall lies on the select's.
    let mut elems = vec![];
    for (k, (sx, sw)) in [(-20.0, 140.0), (-20.0, 70.0), (0.0, 120.0)]
        .into_iter()
        .enumerate()
    {
        let y = OFFSET + k as f64 * (h + ROOM);
        elems.push(bx(active, OFFSET, y, LONG, h));
        elems.push(bx(select, OFFSET + sx, y - 20.0, sw, h + 40.0));
    }
    c.write("active", "ACTIVE.AUX.1.select", true, elems);

    // SRAM.AUX.2: a bar clear of the marker, one abutting it along an edge - no shared
    // area, so an ordinary ACTIVE that touches SRAM - and one over it, which is SRAM.
    let mut elems = vec![];
    for (k, x0) in [0.0, 100.0, 150.0].into_iter().enumerate() {
        elems.push(bx(
            active,
            OFFSET + x0,
            OFFSET + k as f64 * (h + ROOM),
            LONG,
            h,
        ));
    }
    elems.push(bx(
        marker,
        OFFSET + 200.0,
        OFFSET - 100.0,
        400.0,
        3.0 * (h + ROOM),
    ));
    write("SRAM.ACTIVE.AUX.2.abut", elems);
}

/// ACTIVE.W.2: every vertical edge a whole multiple of 27 nm. Bars 27 and 54 nm tall,
/// then 40.5 and 81.25, each of those two failing walls; an L whose column rises 27 nm
/// above its bar, then 13.5 nm, where the column's wall and the riser both fail; and a
/// 1100 nm bar, no multiple either, above the cap and so unread.
fn increments(c: &Corpus<'_>) {
    let active = c.layer("ACTIVE");
    let select = c.layer("NSELECT");
    let h = c.drm("ACTIVE.W.1", "min_width", 27.0);
    let mut elems = vec![];
    let mut y = OFFSET;
    for rise in [h, h / 2.0] {
        elems.push(pg(
            active,
            &[
                (OFFSET, y),
                (OFFSET + LONG, y),
                (OFFSET + LONG, y + h),
                (OFFSET + 40.0, y + h),
                (OFFSET + 40.0, y + h + rise),
                (OFFSET, y + h + rise),
            ],
        ));
        y += h + rise + ROOM;
    }
    let bars = [
        (LONG, h),
        (LONG, 2.0 * h),
        (LONG, 1.5 * h),
        (LONG, 81.25),
        (LONG, 1100.0),
    ];
    elems.extend(boxes(active, &bars, OFFSET, y));
    let top = y + bars.iter().map(|b| b.1 + ROOM).sum::<f64>();
    elems.push(bx(
        select,
        OFFSET - 100.0,
        OFFSET - 100.0,
        400.0,
        top - OFFSET + 200.0,
    ));
    c.write("active", "ACTIVE.W.2.increment", true, elems);
}

/// ACTIVE.LUP.1: a MOS device's ACTIVE within 30 µm of a tap in its own well or in the
/// substrate. A device is a 100 by 27 ACTIVE under its select with a gate across it; a
/// tap the same ACTIVE under the other select. The tap's near wall is 30 µm from the
/// device's far wall, then a DBU more; in the well, a third tap sits 20 µm off in a
/// well of its own, which is no tap for this one.
fn latch_up(c: &Corpus<'_>) {
    let (active, gate, nwell) = (c.layer("ACTIVE"), c.layer("GATE"), c.layer("NWELL"));
    let (nsel, psel) = (c.layer("NSELECT"), c.layer("PSELECT"));
    let reach = c.drm("ACTIVE.LUP.1", "max_space", 30_000.0);
    let device = |x: f64, y: f64, sel: (i16, i16)| {
        vec![
            bx(active, x, y, LONG, 27.0),
            bx(sel, x - 20.0, y - 20.0, LONG + 40.0, 67.0),
            bx(gate, x + 40.0, y - 30.0, 20.0, 87.0),
        ]
    };
    let tap = |x: f64, y: f64, sel: (i16, i16)| {
        vec![
            bx(active, x, y, LONG, 27.0),
            bx(sel, x - 20.0, y - 20.0, LONG + 40.0, 67.0),
        ]
    };
    let row = |k: usize| OFFSET + k as f64 * (200.0 + ROOM);
    let x = OFFSET + 100.0;

    let mut elems = vec![];
    for (k, d) in [reach, reach + DBU].into_iter().enumerate() {
        let y = row(k);
        elems.extend(device(x, y + 50.0, psel));
        elems.extend(tap(x + d, y + 50.0, nsel));
        elems.push(bx(nwell, OFFSET, y, d + 300.0, 200.0));
    }
    let y = row(2);
    elems.extend(device(x, y + 50.0, psel));
    elems.push(bx(nwell, OFFSET, y, 10_000.0, 200.0));
    let tx = x + 20_000.0;
    elems.extend(tap(tx, y + 50.0, nsel));
    elems.push(bx(nwell, tx - 100.0, y, LONG + 200.0, 200.0));
    c.write("active", "ACTIVE.LUP.1.well", true, elems);

    let mut elems = vec![];
    for (k, d) in [reach, reach + DBU].into_iter().enumerate() {
        let y = row(k);
        elems.extend(device(x, y + 50.0, nsel));
        elems.extend(tap(x + d, y + 50.0, psel));
    }
    c.write("active", "ACTIVE.LUP.1.substrate", true, elems);
}
