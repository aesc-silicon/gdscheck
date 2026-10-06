// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{Corpus, DBU, nm};
use crate::helpers::{enclosure_pattern, shift, space_pattern};

/// ACTIVE side, room between enclosure pairs, and where each pattern starts, in nm.
const SIDE: f64 = 32.0;
const GAP: f64 = 100.0;
const OFFSET: f64 = 6990.0;

/// DRM 3.5: the entire ACTIVE polygon gets SRAM rules when it interacts with SRAMDRC,
/// so a crossing marker must not clip it into two rule regions. Each pattern holds
/// neighbours at the limit (clean) and one DBU inside it (violations):
///
/// - `outside`: no marker over the pattern; the ordinary 27 nm rule applies.
/// - `inside`: the marker covers everything; the 13.5 nm SRAM rule applies.
/// - `crossing`: the marker covers the bottom half of each ACTIVE, so the top
///   violation faces unmarked ACTIVE. A clipped fragment would answer to 27 nm there.
pub(super) fn generate(c: &Corpus<'_>) {
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
                let e = enclosure_pattern(
                    nwell,
                    active,
                    nm(limit),
                    nm(SIDE),
                    nm(GAP),
                    nm(OFFSET),
                    -nm(DBU),
                );
                // enclosure_pattern draws from y = 0; lift it level with OFFSET.
                (shift(&e, 0.0, nm(OFFSET)), limit)
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
