// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maximum *total* area of a layer across the whole chip.
//!
//! Unlike [`max_area`](super::max_area), which caps each connected region, this sums
//! the area of every region of the layer and flags once if the running total exceeds
//! the limit (KLayout `layer.area > value`).  Used by recommended density-style caps
//! such as MIM.gR (max recommended total MIM area per chip).

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
    let d2 = dbu_to_um * dbu_to_um;
    let mut violations = vec![];

    for layer in &rule.layers {
        println!(
            "[{}] Checking max_total_area <= {:.4} µm² on layer {} ({}/{})",
            rule.id, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        let regions = merged.regions(layout, layer.gds_layer as i16, layer.gds_datatype as i16);
        let total: f64 = regions.iter().map(|r| r.area_dbu * d2).sum();

        if total > rule.value {
            // One chip-level violation, anchored at the first region's marker.
            let (cx, cy) = regions.first().map(|r| r.marker).unwrap_or((0.0, 0.0));
            violations.push(Violation::point(
                &rule.id,
                "Maximum total area violation",
                format!(
                    "total {} area {:.4} µm² > {:.4} µm²",
                    layer.name, total, rule.value
                ),
                cx * dbu_to_um, cy * dbu_to_um,
            ));
        }
    }

    violations
}
