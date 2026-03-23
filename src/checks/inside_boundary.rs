// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::layout::FlatLayout;
use crate::merge::merge_boundaries;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::HashSet;

/// `layers[0]` = boundary layer (e.g. EdgeSeal)
///
/// Checks that every shape on every *other* layer lies inside the **outer edge**
/// of the (merged) boundary ring.  The boundary layer is usually drawn as many
/// segments — merging them yields the real ring, and its outermost contour is the
/// reference; anything sticking out beyond it is flagged once (at the first
/// offending vertex).  Layers listed in the rule's `ignore` are skipped — IHP, for
/// instance, does not check the edge-seal passivation ring outside the seal.
pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    if rule.layers.is_empty() {
        eprintln!("[{}] inside_boundary requires 1 layer (boundary)", rule.id);
        return vec![];
    }
    let boundary_layer = &rule.layers[0];
    let bnd_gds_layer = boundary_layer.gds_layer as i16;
    let bnd_gds_dt = boundary_layer.gds_datatype as i16;

    // Outer edge of the merged boundary ring: take the largest merged region and
    // use its outer contour.  For an axis-aligned/chamfered seal the bounding box
    // of that contour is the outside edge we measure against.
    let merged = merge_boundaries(layout.get(bnd_gds_layer, bnd_gds_dt));
    let Some(outer) = merged
        .iter()
        .max_by(|a, b| outer_area(a).partial_cmp(&outer_area(b)).unwrap())
        .map(|m| &m.outer)
    else {
        return vec![];
    };

    let tol = 0.5 * dbu_to_um;
    let mut bnd_x_min = f64::INFINITY;
    let mut bnd_x_max = f64::NEG_INFINITY;
    let mut bnd_y_min = f64::INFINITY;
    let mut bnd_y_max = f64::NEG_INFINITY;
    for p in outer {
        let x = p.x as f64 * dbu_to_um;
        let y = p.y as f64 * dbu_to_um;
        bnd_x_min = bnd_x_min.min(x);
        bnd_x_max = bnd_x_max.max(x);
        bnd_y_min = bnd_y_min.min(y);
        bnd_y_max = bnd_y_max.max(y);
    }

    let ignore: HashSet<(i16, i16)> = rule
        .ignore
        .iter()
        .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        .collect();

    let mut violations = vec![];

    for b in layout.all_except(bnd_gds_layer, bnd_gds_dt) {
        if ignore.contains(&(b.layer, b.datatype)) {
            continue;
        }
        let outside = b.xy.iter().find(|p| {
            let x = p.x as f64 * dbu_to_um;
            let y = p.y as f64 * dbu_to_um;
            x < bnd_x_min - tol
                || x > bnd_x_max + tol
                || y < bnd_y_min - tol
                || y > bnd_y_max + tol
        });

        if let Some(op) = outside {
            let vx = op.x as f64 * dbu_to_um;
            let vy = op.y as f64 * dbu_to_um;
            violations.push(Violation::point(
                &rule.id,
                "Structure outside boundary",
                format!(
                    "shape on GDS layer {}/{} outside boundary at ({:.4}, {:.4}) µm",
                    b.layer, b.datatype, vx, vy
                ),
                vx, vy,
            ));
        }
    }

    violations
}

/// Absolute area of a merged region's outer contour (shoelace), used to pick the
/// main ring among any spurious fragments.
fn outer_area(m: &crate::merge::MergedPoly) -> f64 {
    let pts = &m.outer;
    let n = pts.len();
    let mut s = 0i128;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        s += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    (s.abs() as f64) * 0.5
}
