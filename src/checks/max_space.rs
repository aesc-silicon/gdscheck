// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maximum space (proximity coverage) — latch-up LU.a–d.  Every part of `layers[0]` must
//! lie within `value` µm of `layers[1]`; any part that does not is flagged.  Used for
//! "every device source/drain must be within 20 µm of a well tie" (LU.a/b) and "every well
//! tie's Activ must be within 6 µm of a contact" (LU.c/c1/d/d1).  Runs on the tiled merge,
//! so the dense `Cont` layer is never globally unioned.

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!("[{}] max_space needs two layers (target, reference)", rule.id);
        return vec![];
    }
    let target = &rule.layers[0];
    let reference = &rule.layers[1];

    println!(
        "[{}] Checking max_space <= {:.2} µm from {} to {}",
        rule.id, rule.value, target.name, reference.name
    );

    let gaps = merged.max_space_gaps(
        layout,
        (target.gds_layer as i16, target.gds_datatype as i16),
        (reference.gds_layer as i16, reference.gds_datatype as i16),
        rule.value / dbu_to_um,
    );

    gaps.into_iter()
        .map(|(cx, cy)| {
            let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
            Violation::point(
                &rule.id,
                "Maximum space violation",
                format!(
                    "{} more than {:.2} µm from {} at ({:.4}, {:.4}) µm",
                    target.name, rule.value, reference.name, x, y
                ),
                x, y,
            )
        })
        .collect()
}
