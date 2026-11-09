// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Additional limits from `techlib/gt2_techfile.tf`, foundry/spacings.
//! These fixtures isolate that deck; dimensions need not meet the ICV/LEF decks.

use super::{Corpus, DBU, bx, corner_rise, pg};

pub(super) fn generate(c: &Corpus<'_>) {
    well(c);
    // Layer, rectangle width in nm, source area in nm². Orient the routing metals
    // along their routing direction. BPR's exact 100 nm² boundary cannot be drawn
    // with a 32 nm width on this grid, so its area fixture uses a 4 × 25 nm box.
    for (layer, width, area) in [
        ("NWELL", 42.0, 3024.0),
        ("ACT", 42.0, 420.0),
        ("BPR", 4.0, 100.0),
        ("M10", 80.0, 4480.0),
        ("M11", 56.0, 4480.0),
        ("M12", 540.0, 194400.0),
        ("M13", 360.0, 194400.0),
        ("RDL", 2400.0, 3840000.0),
        ("BM1", 56.0, 4480.0),
        ("BM2", 80.0, 4480.0),
        ("BM3", 360.0, 194400.0),
        ("BM4", 540.0, 194400.0),
        ("BRDL", 1600.0, 3840000.0),
    ] {
        minimum_area(c, layer, width, area);
    }
}

fn well(c: &Corpus<'_>) {
    let layer = c.layer("NWELL");
    let width = c.src("NWELL.width.TF", "min_width", 42.0);
    let space = c.src("NWELL.space.TF", "min_space", 42.0);
    c.src("NWELL.space.TF", "min_notch", 42.0);
    let diagonal = corner_rise(space, 25.0);
    for bad in [false, true] {
        let delta = if bad { DBU } else { 0.0 };
        c.write(
            "techfile",
            "NWELL.width.TF.width",
            bad,
            vec![
                bx(layer, -10.0, -10.0, width - delta, 100.0),
                bx(layer, 6990.0, 6990.0, 100.0, width - delta),
            ],
        );
        let gap = space - delta;
        c.write(
            "techfile",
            "NWELL.space.TF.space",
            bad,
            vec![
                bx(layer, -300.0, -300.0, 100.0, 100.0),
                bx(layer, -200.0 + gap, -300.0, 100.0, 100.0),
                bx(layer, 1000.0, 1000.0, 100.0, 100.0),
                bx(layer, 1000.0, 1100.0 + gap, 100.0, 100.0),
                bx(layer, 3000.0, 3000.0, 100.0, 100.0),
                bx(layer, 3125.0, 3100.0 + diagonal - delta, 100.0, 100.0),
            ],
        );
        let arm = 84.0;
        let points = [
            (0.0, 0.0),
            (2.0 * arm + gap, 0.0),
            (2.0 * arm + gap, 2.0 * arm),
            (arm + gap, 2.0 * arm),
            (arm + gap, arm),
            (arm, arm),
            (arm, 2.0 * arm),
            (0.0, 2.0 * arm),
        ];
        c.write(
            "techfile",
            "NWELL.space.TF.notch",
            bad,
            vec![
                pg(layer, &points.map(|(x, y)| (6990.0 + x, 6990.0 + y))),
                pg(layer, &points.map(|(x, y)| (19990.0 + y, 19990.0 + x))),
            ],
        );
    }
}

fn minimum_area(c: &Corpus<'_>, name: &str, width: f64, source_area: f64) {
    let id = format!("{name}.area.TF");
    let area = c.src_area(&id, source_area);
    let height = area / width;
    assert!((height / DBU - (height / DBU).round()).abs() < 1e-6);
    let layer = c.layer(name);
    // Negative coordinates and the tile boundaries at both sizes used by CI,
    // including regions drawn as overlapping pieces.
    let origins = [-10.0, 6990.0, 19990.0];
    for bad in [false, true] {
        c.write(
            "techfile",
            &format!("{id}.area"),
            bad,
            origins
                .map(|o| bx(layer, o, o, width, height - if bad { DBU } else { 0.0 }))
                .to_vec(),
        );
    }
    c.write(
        "techfile",
        &format!("{id}.merged"),
        false,
        origins
            .into_iter()
            .flat_map(|o| {
                [
                    bx(layer, o, o, width / 2.0 + DBU, height),
                    bx(layer, o + width / 2.0, o, width / 2.0, height),
                ]
            })
            .collect(),
    );
    // Each region is too small; summing their areas would incorrectly pass them.
    c.write(
        "techfile",
        &format!("{id}.separate"),
        true,
        vec![
            bx(layer, 6000.0, 6000.0, width, height - DBU),
            bx(layer, 11000.0 + width, 6000.0, width, height - DBU),
        ],
    );
    if matches!(name, "RDL" | "BRDL") {
        // The bounding box meets the area, but a 10 × 10 nm hole must be
        // subtracted. Adding one DBU to the height makes the material area pass.
        for bad in [false, true] {
            let h = height + if bad { 0.0 } else { DBU };
            let o = 6990.0;
            let mid_x = width / 2.0;
            let mid_y = height / 2.0;
            c.write(
                "techfile",
                &format!("{id}.hole"),
                bad,
                vec![
                    bx(layer, o, o, width, mid_y),
                    bx(layer, o, o + mid_y + 10.0, width, h - mid_y - 10.0),
                    bx(layer, o, o + mid_y, mid_x, 10.0),
                    bx(
                        layer,
                        o + mid_x + 10.0,
                        o + mid_y,
                        width - mid_x - 10.0,
                        10.0,
                    ),
                ],
            );
        }
    }
}
