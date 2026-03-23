// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Whole-chip layer density (e.g. the metal-fill density rules): the merged coverage
//! of the rule's layers over the chip bounding box must meet a minimum
//! (`min_density`) or maximum (`max_density`) percentage.

use crate::cache::Cache;
use crate::layout::FlatLayout;
use crate::merge::{clipped_area_dbu, MergedCache};
use crate::pdk::{Layer, RuleDefinition};
use crate::violation::Violation;
use rayon::prelude::*;

/// Combined density must be at least `value` (%).
pub fn run_min(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    cache: &mut Cache,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, cache, merged, "min_density", ">=", "Minimum", "<", |d, v| d < v)
}

/// Combined density must be at most `value` (%).
pub fn run_max(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    cache: &mut Cache,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, cache, merged, "max_density", "<=", "Maximum", ">", |d, v| d > v)
}

#[allow(clippy::too_many_arguments)]
fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    cache: &mut Cache,
    merged: &mut MergedCache,
    check_name: &str,
    op: &str,
    bound: &str,
    cmp: &str,
    viol: impl Fn(f64, f64) -> bool,
) -> Vec<Violation> {
    let layer_names: Vec<&str> = rule.layers.iter().map(|l| l.name.as_str()).collect();
    println!(
        "[{}] Checking {} {} {:.2}% on layer(s) [{}]",
        rule.id, check_name, op, rule.value, layer_names.join(", ")
    );

    let boundary_layer = rule.params.get("boundary_layer").map(|&l| {
        let dt = rule.params.get("boundary_datatype").copied().unwrap_or(0.0);
        (l as i16, dt as i16)
    });

    let density = match compute_density(&rule.layers, layout, dbu_to_um, cache, merged, boundary_layer) {
        Some(d) => d,
        None => {
            eprintln!("[{}] Could not compute density", rule.id);
            return vec![];
        }
    };

    println!("[{}] Density: {:.2}%", rule.id, density);

    if viol(density, rule.value) {
        vec![Violation::global(
            &rule.id,
            &format!("{bound} density violation"),
            format!(
                "density {:.2}% {} {:.2}% on layer(s) [{}]",
                density, cmp, rule.value, layer_names.join(", ")
            ),
        )]
    } else {
        vec![]
    }
}

/// Chip bounding box in DBU.  Uses the boundary layer if given and present,
/// otherwise the bounding box of all shapes.
fn chip_bbox(layout: &FlatLayout, boundary_layer: Option<(i16, i16)>) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    let specific = boundary_layer.map(|(l, dt)| layout.get(l, dt));
    let boundaries: Box<dyn Iterator<Item = _>> = match specific {
        Some(s) if !s.is_empty() => Box::new(s.iter()),
        _ => Box::new(layout.all_boundaries()),
    };

    for b in boundaries {
        for pt in &b.xy {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
            max_x = max_x.max(pt.x);
            max_y = max_y.max(pt.y);
        }
    }

    if min_x == i32::MAX { None } else { Some((min_x, min_y, max_x, max_y)) }
}

/// Merged area (µm²) of one layer, summed over its cached tiles.  Each merged
/// region is clipped to the tile core so regions spanning tiles (present in
/// several halo-overlapping tiles) are counted exactly once.  Cached per layer
/// so min/max density rules on the same layer don't recompute it.
fn layer_merged_area_um2(
    layer: &Layer,
    dbu_to_um: f64,
    area_cache: &mut Cache,
    merged: &MergedCache,
) -> f64 {
    let key = format!("marea:{}/{}", layer.gds_layer, layer.gds_datatype);
    if let Some(c) = area_cache.get(&key) {
        return c;
    }
    let tile = merged.tile_dbu() as i64;
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    let area_dbu: f64 = merged
        .tiles(gl, gd)
        .par_iter()
        .map(|(&(tx, ty), polys)| {
            let x0 = (tx as i64 * tile) as f64;
            let y0 = (ty as i64 * tile) as f64;
            let x1 = ((tx as i64 + 1) * tile) as f64;
            let y1 = ((ty as i64 + 1) * tile) as f64;
            polys.iter().map(|p| clipped_area_dbu(p, x0, y0, x1, y1)).sum::<f64>()
        })
        .sum();
    let area = area_dbu * dbu_to_um * dbu_to_um;
    area_cache.set(&key, area);
    area
}

/// Combined density (%) of `layers` over the chip area, computed on **merged**
/// geometry so overlapping/nested shapes are not double-counted.  `boundary_layer`
/// selects the area denominator (its bounding box); otherwise all shapes are used.
///
/// The per-layer areas are summed; for the standard density rules the layers are
/// disjoint (drawing / filler / mask occupy different datatypes), so the sum is
/// the union area.
pub fn compute_density(
    layers: &[Layer],
    layout: &FlatLayout,
    dbu_to_um: f64,
    area_cache: &mut Cache,
    merged: &mut MergedCache,
    boundary_layer: Option<(i16, i16)>,
) -> Option<f64> {
    let bbox = chip_bbox(layout, boundary_layer)?;
    let chip_area = (bbox.2 - bbox.0) as f64 * dbu_to_um * (bbox.3 - bbox.1) as f64 * dbu_to_um;
    if chip_area == 0.0 {
        return None;
    }

    let mut total = 0.0;
    for l in layers {
        merged.ensure(layout, l.gds_layer as i16, l.gds_datatype as i16);
        total += layer_merged_area_um2(l, dbu_to_um, area_cache, merged);
    }

    Some((total / chip_area * 100.0 * 1000.0).round() / 1000.0)
}
