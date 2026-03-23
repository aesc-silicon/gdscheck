// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum density of a feature layer measured *per connected region* of a base layer.
//! For each connected region of `layers[0]` that is *big* — at least `min_size` µm across
//! in every direction (found by a tiled erosion of radius `min_size/2`) — the fraction of
//! its true filled area covered by the enclosed `layers[1]` feature must be at least
//! `value` %.  Measuring per region means a single starved region cannot be averaged out
//! by well-covered neighbours, and the tiled analysis never globally unions dense metal.
//!
//! Used by Slt.i (metal-slit density on large metal plates: base = Metal(n)NoExempt with
//! pads/MIM/IND removed, feature = slit), but the check is layer-agnostic.

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
        eprintln!("[{}] min_region_density needs a base layer and a feature layer", rule.id);
        return vec![];
    }
    let base = &rule.layers[0];
    let feature = &rule.layers[1];
    let min_size = rule.params.get("min_size").copied().unwrap_or(35.0);
    let radius = (min_size / dbu_to_um) / 2.0;
    let min_pct = rule.value;

    println!(
        "[{}] Checking min_region_density >= {:.2}% of {} in {} regions > {:.0} µm",
        rule.id, min_pct, feature.name, base.name, min_size
    );

    let plates = merged.plate_regions(
        layout,
        (base.gds_layer as i16, base.gds_datatype as i16),
        (feature.gds_layer as i16, feature.gds_datatype as i16),
        radius,
    );

    plates
        .iter()
        .filter(|p| p.is_wide && p.metal_area > 0.0)
        .filter_map(|p| {
            let pct = 100.0 * p.feature_area / p.metal_area;
            if pct >= min_pct {
                return None;
            }
            let (cx, cy) = p.wide_at;
            Some(Violation::point(
                &rule.id,
                "Minimum region-density violation",
                format!(
                    "{} density {:.2}% < {:.2}% in a {} region at ({:.4}, {:.4}) µm",
                    feature.name, pct, min_pct, base.name, cx * dbu_to_um, cy * dbu_to_um
                ),
                cx * dbu_to_um, cy * dbu_to_um,
            ))
        })
        .collect()
}
