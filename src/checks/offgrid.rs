// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let grid_dbu = (rule.value / dbu_to_um).round() as i32;
    if grid_dbu < 1 {
        eprintln!(
            "[{}] Grid size {:.4} µm is smaller than 1 DBU",
            rule.id, rule.value
        );
        return vec![];
    }
    let mut violations = vec![];
    for layer in &rule.layers {
        println!(
            "[{}] Checking offgrid (grid = {:.4} µm) on layer {} ({}/{})",
            rule.id, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );
        // The merged layer, not the drawn shapes: a vertex inside another shape of the
        // same layer is nobody's corner once the layer is one region, and that is what
        // the foundry's check reads.  A ring drawn both as an on-grid polygon and as a
        // path has the path's mitered corner three nanometres inside the polygon, and
        // reporting it named a point the layout never shows.  Each vertex is counted in
        // the tile whose core owns it, so a copy in a neighbour's halo is not a second.
        let key = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, key.0, key.1);
        let t = merged.tile_dbu() as i64;
        let mut hits: Vec<(i32, i32)> = merged
            .tiles(key.0, key.1)
            .par_iter()
            .flat_map_iter(|(&(tx, ty), polys)| {
                let (x0, y0) = (tx as i64 * t, ty as i64 * t);
                let (x1, y1) = ((tx as i64 + 1) * t, (ty as i64 + 1) * t);
                let mut v = Vec::new();
                for poly in polys {
                    for p in poly.outer.iter().chain(poly.holes.iter().flatten()) {
                        let (x, y) = (p.x as i64, p.y as i64);
                        if x >= x0
                            && x < x1
                            && y >= y0
                            && y < y1
                            && (p.x % grid_dbu != 0 || p.y % grid_dbu != 0)
                        {
                            v.push((p.x, p.y));
                        }
                    }
                }
                v.into_iter()
            })
            .collect();
        hits.sort_unstable();
        hits.dedup();
        for (px, py) in hits {
            let x = px as f64 * dbu_to_um;
            let y = py as f64 * dbu_to_um;
            violations.push(Violation::point(
                &rule.id,
                "Off-grid vertex",
                format!(
                    "{}: off-grid vertex (grid = {:.4} µm) at ({:.4}, {:.4}) µm",
                    layer.name, rule.value, x, y
                ),
                x,
                y,
            ));
        }
    }

    violations
}
