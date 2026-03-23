// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Wide base area that lacks a required feature (Slt.c: metal wider than 30 µm must have a
//! slit).  `layers[0]` (e.g. Metal(n)NoExempt — metal with the slit-exempt regions removed)
//! is split into connected regions; a region that is *wide* (contains a spot ≥ `value` µm
//! across in every direction, found by a tiled erosion of radius `value/2`) and encloses no
//! `layers[1]` feature is flagged.  Everything runs on the tiled merge, so dense metal is
//! never globally unioned.

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
        eprintln!("[{}] wide_uncovered needs a base layer and a feature layer", rule.id);
        return vec![];
    }
    let base = &rule.layers[0];
    let feature = &rule.layers[1];
    // A region is "wide" if it survives erosion by half the width threshold.
    let radius = (rule.value / dbu_to_um) / 2.0;

    println!(
        "[{}] Checking wide_uncovered: {} wider than {:.2} µm must contain {}",
        rule.id, base.name, rule.value, feature.name
    );

    let plates = merged.plate_regions(
        layout,
        (base.gds_layer as i16, base.gds_datatype as i16),
        (feature.gds_layer as i16, feature.gds_datatype as i16),
        radius,
    );

    plates
        .iter()
        .filter(|p| p.is_wide && p.feature_area <= 0.5)
        .map(|p| {
            let (cx, cy) = p.wide_at;
            Violation::point(
                &rule.id,
                "Wide-area without feature violation",
                format!(
                    "{} wider than {:.2} µm has no {} at ({:.4}, {:.4}) µm",
                    base.name, rule.value, feature.name, cx * dbu_to_um, cy * dbu_to_um
                ),
                cx * dbu_to_um, cy * dbu_to_um,
            )
        })
        .collect()
}
