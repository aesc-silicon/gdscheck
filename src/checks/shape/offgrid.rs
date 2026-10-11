// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Vertices on a grid: every vertex of the merged layer has both coordinates - or, under
//! `facing`, the one across that axis - a whole number of `value` from `offset`.  A
//! manufacturing grid reads both coordinates from the origin; a routing layer's grid
//! reads one, the coordinate its edges run at, from wherever the design put its first
//! track: ASAP7's M4 has its horizontal edges every 24 nm from an offset set per design.

use crate::checks::params;
use crate::geom::Axis;
use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let grid_dbu = (rule.value / dbu_to_um).round() as i32;
    if grid_dbu < 1 {
        eprintln!(
            "[{}] Grid size {:.4} µm is smaller than 1 DBU",
            rule.id, rule.value
        );
        return vec![];
    }
    // `facing: y` reads the horizontal walls, so the y of every vertex; nothing, both.
    let Some(axis) = params::facing_axis(rule, "offgrid") else {
        return vec![];
    };
    let Some(offset_um) = params::offset(rule) else {
        return vec![];
    };
    let offset = offset_um / dbu_to_um;
    if (offset - offset.round()).abs() > 1e-6 {
        eprintln!(
            "[{}] Offset {:.4} µm is not on the DBU grid",
            rule.id, offset_um
        );
        return vec![];
    }
    let offset = offset.round() as i32;
    let off = |c: i32| (c - offset).rem_euclid(grid_dbu) != 0;
    let grid = format!(
        "grid = {:.4} µm{}{}",
        rule.value,
        if offset == 0 {
            String::new()
        } else {
            format!(" from {offset_um:.4} µm")
        },
        match axis {
            None => "",
            Some(Axis::X) => ", x",
            Some(Axis::Y) => ", y",
        }
    );

    let mut violations = vec![];
    for layer in &rule.layers {
        println!(
            "[{}] Checking offgrid ({grid}) on layer {} ({}/{})",
            rule.id, layer.name, layer.gds_layer, layer.gds_datatype
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
                for poly in polys.iter() {
                    for p in poly.outer.iter().chain(poly.holes.iter().flatten()) {
                        let (x, y) = (p.x as i64, p.y as i64);
                        let owned = x >= x0 && x < x1 && y >= y0 && y < y1;
                        let bad = match axis {
                            None => off(p.x) || off(p.y),
                            Some(Axis::X) => off(p.x),
                            Some(Axis::Y) => off(p.y),
                        };
                        if owned && bad {
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
                    "{}: off-grid vertex ({grid}) at ({:.4}, {:.4}) µm",
                    layer.name, x, y
                ),
                x,
                y,
            ));
        }
    }

    violations
}
