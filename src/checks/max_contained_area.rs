// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maximum total area of one layer inside each region of another.
//!
//! `layers[0]` is the container and `layers[1]` what is counted inside it: every region
//! of the container is taken on its own and the area of the contained layer within it
//! summed, and a region whose total exceeds the value is reported.
//!
//! It is what a rule means by "these may share a plate, as long as the total on that one
//! plate stays under the cap" - GF180's MIM.11 and MIMTM.11, where several MIM capacitors
//! may share a bottom plate but their areas add up against MIM.8b's limit.  Neither
//! [`max_area`](super::area), which caps one region of one layer, nor
//! [`max_total_area`](super::max_total_area), which caps a whole chip, says that.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, VirtualOp, clipped_area_dbu, compose_tile, stitch_labeled};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let (Some(outer), Some(inner)) = (rule.layers.first(), rule.layers.get(1)) else {
        eprintln!("[{}] max_contained_area needs two layers", rule.id);
        return Vec::new();
    };
    let (ok, od) = (outer.gds_layer as i16, outer.gds_datatype as i16);
    let (ik, id) = (inner.gds_layer as i16, inner.gds_datatype as i16);
    println!(
        "[{}] Checking max_contained_area <= {:.4} µm² of {} inside each {} region",
        rule.id, rule.value, inner.name, outer.name
    );
    merged.ensure(layout, ok, od);
    merged.ensure(layout, ik, id);

    let tile = merged.tile_dbu();
    let labeled = stitch_labeled(merged.tiles(ok, od), tile);
    let mut total = vec![0.0_f64; labeled.regions.len()];
    // The violation is about the whole container, so it is marked at the middle of it
    // rather than at whichever piece the stitcher happened to name it after.
    let mut bbox = vec![(f64::MAX, f64::MAX, f64::MIN, f64::MIN); labeled.regions.len()];
    let d2 = dbu_to_um * dbu_to_um;

    for (&(tx, ty), polys) in &labeled.by_tile {
        let Some(ins) = merged.tiles(ik, id).get(&(tx, ty)) else {
            continue;
        };
        // Clip to the tile's own core so a container spanning tiles is counted once.
        let t = tile as i64;
        let (x0, y0) = ((tx as i64 * t) as f64, (ty as i64 * t) as f64);
        let (x1, y1) = (((tx as i64 + 1) * t) as f64, ((ty as i64 + 1) * t) as f64);
        for (poly, rid) in polys {
            let b = &mut bbox[*rid];
            for p in &poly.outer {
                b.0 = b.0.min(p.x as f64);
                b.1 = b.1.min(p.y as f64);
                b.2 = b.2.max(p.x as f64);
                b.3 = b.3.max(p.y as f64);
            }
            let hit = compose_tile(VirtualOp::Intersection, &[std::slice::from_ref(poly), ins]);
            for h in &hit {
                total[*rid] += clipped_area_dbu(h, x0, y0, x1, y1);
            }
        }
    }

    let mut violations = Vec::new();
    for rid in 0..labeled.regions.len() {
        let a = total[rid] * d2;
        if a > rule.value {
            let b = bbox[rid];
            let (cx, cy) = ((b.0 + b.2) * 0.5, (b.1 + b.3) * 0.5);
            violations.push(Violation::point(
                &rule.id,
                "Maximum contained area violation",
                format!(
                    "{} inside this {} totals {:.4} µm² > {:.4} µm²",
                    inner.name, outer.name, a, rule.value
                ),
                cx * dbu_to_um,
                cy * dbu_to_um,
            ));
        }
    }
    violations
}
