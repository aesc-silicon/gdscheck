// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum / maximum area of each **connected region**.
//!
//! Shapes that overlap or abut form one region and are measured together.  Regions
//! are reconstructed from the shared [`MergedCache`] by stitching the per-tile merge
//! across tile borders (`MergedCache::regions`) — bounded memory even for
//! large/dense layers.  A region failing the bound is flagged once at its marker.

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Region area must be at least `value` (µm²).
pub fn run_min(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, "min_area", ">=", "Minimum", "<", |a, v| a < v)
}

/// Region area must be at most `value` (µm²).
pub fn run_max(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, "max_area", "<=", "Maximum", ">", |a, v| a > v)
}

#[allow(clippy::too_many_arguments)]
fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    check_name: &str,
    op: &str,
    bound: &str,
    cmp: &str,
    viol: impl Fn(f64, f64) -> bool,
) -> Vec<Violation> {
    let mut violations = vec![];
    let d2 = dbu_to_um * dbu_to_um;

    for layer in &rule.layers {
        println!(
            "[{}] Checking {} {} {:.4} µm² on layer {} ({}/{})",
            rule.id, check_name, op, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        let regions = merged.regions(layout, layer.gds_layer as i16, layer.gds_datatype as i16);
        let mut layer_violations: Vec<Violation> = regions
            .iter()
            .filter_map(|region| {
                let area = region.area_dbu * d2;
                if viol(area, rule.value) {
                    let (cx, cy) = region.marker;
                    Some(Violation::point(
                        &rule.id,
                        &format!("{bound} area violation"),
                        format!(
                            "region area {:.4} µm² {} {:.4} µm² on layer {} at ({:.4}, {:.4}) µm",
                            area, cmp, rule.value, layer.name, cx * dbu_to_um, cy * dbu_to_um
                        ),
                        cx * dbu_to_um, cy * dbu_to_um,
                    ))
                } else {
                    None
                }
            })
            .collect();

        violations.append(&mut layer_violations);
    }

    violations
}
