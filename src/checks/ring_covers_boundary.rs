// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::layout::FlatLayout;
use crate::merge::{merge_boundaries, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use i_overlay::i_float::int::point::IntPoint;

/// Even-odd point-in-contour test in DBU coordinates.
fn point_in_contour(px: i64, py: i64, c: &[IntPoint]) -> bool {
    let n = c.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (c[i].x as i64, c[i].y as i64);
        let (xj, yj) = (c[j].x as i64, c[j].y as i64);
        if (yi > py) != (yj > py) {
            // x of the edge at height py (cross-multiplied to stay exact-ish)
            let t = (px - xi) * (yj - yi) - (xj - xi) * (py - yi);
            if (yj > yi) == (t < 0) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// A point is enclosed by the ring if it lies inside the hole of some merged ring
/// region — i.e. surrounded by ring material on all sides.  A break in the ring
/// merges that hole into the exterior, so the point is no longer enclosed.
fn enclosed_by_ring(px: i64, py: i64, ring: &[MergedPoly]) -> bool {
    ring.iter().any(|r| {
        point_in_contour(px, py, &r.outer) && r.holes.iter().any(|h| point_in_contour(px, py, h))
    })
}

/// `layers[0]` = ring layer (e.g. Passiv)
/// `layers[1]` = boundary layer to enclose (e.g. EdgeSeal)
///
/// The sealring must be enclosed by an **unbroken** ring of the ring layer.  Both
/// layers are merged; every vertex of the merged boundary must sit inside a hole
/// of the merged ring.  If the ring has a gap, the enclosing hole opens to the
/// exterior and the exposed boundary vertices are flagged.
pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!("[{}] ring_covers_boundary requires 2 layers (ring, boundary)", rule.id);
        return vec![];
    }
    let ring_layer = &rule.layers[0];
    let boundary_layer = &rule.layers[1];

    let ring = merge_boundaries(layout.get(
        ring_layer.gds_layer as i16,
        ring_layer.gds_datatype as i16,
    ));
    let seal = merge_boundaries(layout.get(
        boundary_layer.gds_layer as i16,
        boundary_layer.gds_datatype as i16,
    ));
    if ring.is_empty() || seal.is_empty() {
        return vec![];
    }

    let mut violations = vec![];
    for s in &seal {
        for p in &s.outer {
            if !enclosed_by_ring(p.x as i64, p.y as i64, &ring) {
                violations.push(Violation::point(
                    &rule.id,
                    "Unbroken ring required",
                    format!(
                        "{} not enclosed by unbroken {} ring at ({:.4}, {:.4}) µm",
                        boundary_layer.name,
                        ring_layer.name,
                        p.x as f64 * dbu_to_um,
                        p.y as f64 * dbu_to_um,
                    ),
                    p.x as f64 * dbu_to_um,
                    p.y as f64 * dbu_to_um,
                ));
                break; // one violation per merged seal region
            }
        }
    }

    violations
}
