// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SDT patterns (DRM 3.8). A source/drain trench is 24 nm wide and as tall as the
//! 27 nm ACTIVE it sits on, its horizontal edges on the ACTIVE's, inside a LISD, 30 nm
//! from the next trench and 5 nm from a gate; in SRAM it is 17 nm tall at least and
//! overlaps its ACTIVE and LISD by 17 nm. Each pattern leads with the trench exactly
//! at the limit, then the one a DBU past it.
//!
//! An SRAM trench under 17 nm tall overlaps nothing by 17 nm, so SRAM.SDT.W.4's short
//! trench fails the two overlaps too; the test names and ignores them.

use super::patterns::{ROOM, bx, pg};
use super::{Corpus, DBU, around};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// A gate's width and how far the ACTIVE runs past a trench either side.
const GATE: f64 = 20.0;
const RUN: f64 = 20.0;

pub(super) fn generate(c: &Corpus<'_>) {
    sram_overlaps(c);
    shapes(c);
}

/// DRM 3.8: the two explicit SRAM overlaps at 17 nm, including a reference that is
/// absent or only touches, where there is no intersection to measure a width on.
fn sram_overlaps(c: &Corpus<'_>) {
    let sdt_active = c.drm("SRAM.SDT.ACTIVE.OV.3", "min_width", 17.0);
    for overlap in around(sdt_active) {
        let name = format!("SRAM.SDT.ACTIVE.OV.3.overlap_{}", (overlap * 4.0) as i32);
        c.layout(
            "sdt",
            &name,
            overlap < sdt_active,
            &[
                ("SDT", [0.0, 0.0, 24.0, 27.0]),
                ("ACTIVE", [-10.0, 27.0 - overlap, 40.0, 54.0 - overlap]),
                ("LISD", [-5.0, -5.0, 30.0, 60.0]),
                ("SRAMDRC", [-20.0, -20.0, 70.0, 70.0]),
            ],
            19990.0,
        );
    }

    for (reference, other, rule) in [
        ("LISD", "ACTIVE", "SRAM.SDT.LISD.OV.4"),
        ("ACTIVE", "LISD", "SRAM.SDT.ACTIVE.OV.3"),
    ] {
        let ov = c.drm(rule, "min_width", 17.0);
        for (label, overlap) in [
            ("absent", None),
            ("touch", Some(0.0)),
            ("under", Some(ov - DBU)),
            ("exact", Some(ov)),
            ("over", Some(ov + DBU)),
        ] {
            let name = format!("{rule}.{label}");
            let mut shapes = vec![
                ("SDT", [0.0, 0.0, 24.0, 27.0]),
                (other, [-10.0, -10.0, 40.0, 40.0]),
                ("SRAMDRC", [23.75, 26.75, 40.0, 40.0]),
            ];
            if let Some(h) = overlap {
                shapes.push((reference, [0.0, h - 50.0, 24.0, h]));
            }
            c.layout(
                "sdt",
                &name,
                overlap.is_none_or(|h| h < ov),
                &shapes,
                19990.0,
            );
        }
    }
}

/// The trench's own rules.
fn shapes(c: &Corpus<'_>) {
    let sdt = c.layer("SDT");
    let active = c.layer("ACTIVE");
    let lisd = c.layer("LISD");
    let gate = c.layer("GATE");
    let marker = c.layer("SRAMDRC");
    let w = c.drm("SDT.W.1", "min_width", 24.0);
    let h = c.drm("SDT.W.2", "min_width", 27.0);
    // A `tw` by `th` trench at (x, y) on an ACTIVE as tall as it, running RUN past it
    // either side, under a LISD 3 nm wider and 5 nm taller.
    let trench = |x: f64, y: f64, tw: f64, th: f64| -> Vec<GdsElement> {
        vec![
            bx(sdt, x, y, tw, th),
            bx(active, x - RUN, y, tw + 2.0 * RUN, th),
            bx(lisd, x - 3.0, y - 5.0, tw + 6.0, th + 10.0),
        ]
    };
    let row = |k: usize| OFFSET + k as f64 * (100.0 + ROOM);
    let write = |name: &str, elems: Vec<GdsElement>| c.write("sdt", name, true, elems);

    let mut elems = trench(OFFSET, row(0), w, h);
    elems.extend(trench(OFFSET, row(1), w - DBU, h));
    write("SDT.W.1.narrow", elems);

    // The ACTIVE is as short as the trench, so only the height is wrong.
    let mut elems = trench(OFFSET, row(0), w, h);
    elems.extend(trench(OFFSET, row(1), w, h - DBU));
    write("SDT.W.2.short", elems);

    // In SRAM, 17 nm tall on a taller ACTIVE and LISD: the short one overlaps neither
    // by 17 nm either.
    let sram_h = c.drm("SRAM.SDT.W.4", "min_width", 17.0);
    let mut elems = vec![];
    for (k, th) in [sram_h, sram_h - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(sdt, x, y, w, th));
        elems.push(bx(active, x - RUN, y - 10.0, w + 2.0 * RUN, 40.0));
        elems.push(bx(lisd, x - 3.0, y - 10.0, w + 6.0, 40.0));
    }
    elems.push(bx(marker, OFFSET - 100.0, OFFSET - 100.0, 400.0, 1200.0));
    write("SRAM.SDT.W.4.short", elems);

    // Two trenches on one ACTIVE, under one LISD.
    let s = c.drm("SDT.S.1", "min_space", 30.0);
    let mut elems = vec![];
    for (k, g) in [s, s - DBU].into_iter().enumerate() {
        let (x, y) = (OFFSET, row(k));
        elems.push(bx(sdt, x, y, w, h));
        elems.push(bx(sdt, x + w + g, y, w, h));
        elems.push(bx(active, x - RUN, y, 2.0 * w + g + 2.0 * RUN, h));
        elems.push(bx(lisd, x - 3.0, y - 5.0, 2.0 * w + g + 6.0, h + 10.0));
    }
    write("SDT.S.1.close", elems);

    // A U of two 24 nm arms, 40 nm tall on a 30 nm back, the slot between them facing
    // across x. The ACTIVE is the same U, so every horizontal edge of the trench lies
    // on one of its own.
    let n = c.drm("SDT.S.1", "min_notch", 30.0);
    let mut elems = vec![];
    for (k, g) in [n, n - DBU].into_iter().enumerate() {
        let (x, y, back, top) = (OFFSET, row(k), 30.0, 70.0);
        let u = [
            (x, y),
            (x + 2.0 * w + g, y),
            (x + 2.0 * w + g, y + top),
            (x + w + g, y + top),
            (x + w + g, y + back),
            (x + w, y + back),
            (x + w, y + top),
            (x, y + top),
        ];
        elems.push(pg(sdt, &u));
        elems.push(pg(active, &u));
        elems.push(bx(lisd, x - 3.0, y - 5.0, 2.0 * w + g + 6.0, top + 10.0));
    }
    write("SDT.S.1.notch", elems);

    // A gate crossing the ACTIVE, the trench 5 nm to its right, then a DBU less; then
    // a trench touching the gate and one over it, which AUX.1 forbids either way.
    let s2 = c.drm("SDT.GATE.S.2", "min_space", 5.0);
    let beside = |y: f64, gap: f64| -> Vec<GdsElement> {
        let mut e = trench(OFFSET + GATE + gap, y, w, h);
        e.push(bx(gate, OFFSET, y - 20.0, GATE, h + 40.0));
        e
    };
    let mut elems = beside(row(0), s2);
    elems.extend(beside(row(1), s2 - DBU));
    write("SDT.GATE.S.2.close", elems);
    let mut elems = beside(row(0), s2);
    elems.extend(beside(row(1), 0.0));
    elems.extend(beside(row(2), -5.0));
    write("SDT.GATE.AUX.1.touch", elems);

    // A trench as tall as a 40 nm ACTIVE, then one 30 nm tall on it with their bottom
    // edges together: its top edge lies inside the ACTIVE.
    let mut elems = trench(OFFSET, row(0), w, 40.0);
    let y = row(1);
    elems.push(bx(sdt, OFFSET, y, w, 30.0));
    elems.push(bx(active, OFFSET - RUN, y, w + 2.0 * RUN, 40.0));
    elems.push(bx(lisd, OFFSET - 3.0, y - 5.0, w + 6.0, 50.0));
    write("SDT.ACTIVE.AUX.2.inside", elems);

    // A trench on its ACTIVE, then one whose ACTIVE lies 50 nm to its right.
    let mut elems = trench(OFFSET, row(0), w, h);
    let y = row(1);
    elems.push(bx(sdt, OFFSET, y, w, h));
    elems.push(bx(active, OFFSET + w + 50.0, y, 100.0, h));
    elems.push(bx(lisd, OFFSET - 3.0, y - 5.0, w + 6.0, h + 10.0));
    write("SDT.ACTIVE.AUX.3.off", elems);

    // A trench inside its LISD, then one whose LISD covers only its left half.
    let mut elems = trench(OFFSET, row(0), w, h);
    let y = row(1);
    elems.push(bx(sdt, OFFSET, y, w, h));
    elems.push(bx(active, OFFSET - RUN, y, w + 2.0 * RUN, h));
    elems.push(bx(lisd, OFFSET - 3.0, y - 5.0, 3.0 + w / 2.0, h + 10.0));
    write("SDT.LISD.AUX.4.uncovered", elems);
}
