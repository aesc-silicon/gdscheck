// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{Corpus, DBU, around};

pub(super) fn generate(c: &Corpus<'_>) {
    // DRM 3.8: minimum vertical SDT overlap with ACTIVE in SRAM is 17 nm.
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

    // DRM 3.8: both explicit SRAM overlaps, including zero-area/absent references.
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
