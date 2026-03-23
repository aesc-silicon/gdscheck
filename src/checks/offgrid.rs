// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::boundaries_on;
use crate::layout::FlatLayout;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    let grid_dbu = (rule.value / dbu_to_um).round() as i32;
    if grid_dbu < 1 {
        eprintln!("[{}] Grid size {:.4} µm is smaller than 1 DBU", rule.id, rule.value);
        return vec![];
    }

    let mut violations = vec![];

    for layer in &rule.layers {
        println!(
            "[{}] Checking offgrid (grid = {:.4} µm) on layer {} ({}/{})",
            rule.id, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );

        for b in boundaries_on(layout, layer) {
            let n = b.xy.len().saturating_sub(1);
            for point in &b.xy[..n] {
                if point.x % grid_dbu != 0 || point.y % grid_dbu != 0 {
                    let x = point.x as f64 * dbu_to_um;
                    let y = point.y as f64 * dbu_to_um;
                    violations.push(Violation::point(
                        &rule.id,
                        "Offgrid vertex",
                        format!(
                            "{}: off-grid vertex (grid = {:.4} µm) at ({:.4}, {:.4}) µm",
                            layer.name, rule.value, x, y
                        ),
                        x, y,
                    ));
                }
            }
        }
    }

    violations
}
