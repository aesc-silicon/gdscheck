// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::boundaries_on;
use crate::layout::FlatLayout;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use gds21::GdsPoint;

fn centroid(pts: &[GdsPoint], dbu_to_um: f64) -> (f64, f64) {
    let n = pts.len().saturating_sub(1);
    let (sx, sy) = pts[..n.max(1)]
        .iter()
        .fold((0.0, 0.0), |(sx, sy), p| {
            (sx + p.x as f64, sy + p.y as f64)
        });
    let count = n.max(1) as f64;
    (sx / count * dbu_to_um, sy / count * dbu_to_um)
}

pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    let mut violations = vec![];

    for layer in &rule.layers {
        println!(
            "[{}] Checking forbidden layer {} ({}/{})",
            rule.id, layer.name, layer.gds_layer, layer.gds_datatype
        );

        for b in boundaries_on(layout, layer) {
            let (cx, cy) = centroid(&b.xy, dbu_to_um);
            violations.push(Violation::point(
                &rule.id,
                "Forbidden layer",
                format!(
                    "forbidden shape on layer {} at ({:.4}, {:.4}) µm",
                    layer.name, cx, cy
                ),
                cx, cy,
            ));
        }
    }

    violations
}
