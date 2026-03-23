// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Windowed density.
//!
//! Splits the chip into `window × window` tiles and checks the merged coverage
//! of the rule's layers in each.  Coverage is computed on the shared
//! [`MergedCache`] (so overlapping/nested shapes are not double-counted) and
//! summed exactly via Sutherland–Hodgman clipping to the window, accumulated
//! from the cache's tiles so a region spanning cache tiles is counted once.
//!
//! The tile grid itself always starts at the chip's raw bounding box (so the first
//! window is `[0, window)` in each axis); an optional `boundary_layer` /
//! `boundary_datatype` param (the same convention [`density`](super::density) uses)
//! instead restricts *which area within each window counts as checkable* to that
//! layer's bounding box — not its true merged coverage.  A seal ring is drawn as a
//! hollow frame, so its own merged *area* is just the thin frame material and would
//! wildly undercount the region it encloses; the bounding box is the die extent it's
//! meant to stand in for, exactly as the non-windowed `density` check already treats
//! `boundary_layer`.  Without this, an edge or corner window that falls outside the
//! actual seal ring (chip dimensions are rarely an exact multiple of the window size)
//! is measured against its full nominal window area and fails a minimum-density floor
//! it was never meant to meet.  A window with no overlap with the boundary box at all
//! is skipped outright (density is undefined for a zero-area denominator, not a
//! violation).

use crate::layout::FlatLayout;
use crate::merge::{clipped_area_dbu, MergedCache, TileMap};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

/// Chip bounding box in DBU from all shapes.
fn chip_bbox_dbu(layout: &FlatLayout) -> Option<(i64, i64, i64, i64)> {
    let mut x0 = i64::MAX;
    let mut y0 = i64::MAX;
    let mut x1 = i64::MIN;
    let mut y1 = i64::MIN;
    for b in layout.all_boundaries() {
        for p in &b.xy {
            x0 = x0.min(p.x as i64);
            y0 = y0.min(p.y as i64);
            x1 = x1.max(p.x as i64);
            y1 = y1.max(p.y as i64);
        }
    }
    if x0 == i64::MAX { None } else { Some((x0, y0, x1, y1)) }
}

/// Bounding box (DBU) of one layer's raw shapes — used for `boundary_layer`, where the
/// layer (e.g. a seal ring) stands in for the full area it encloses rather than its own
/// drawn material (see the module doc).
fn layer_bbox_dbu(layout: &FlatLayout, gl: i16, gd: i16) -> Option<(i64, i64, i64, i64)> {
    let mut x0 = i64::MAX;
    let mut y0 = i64::MAX;
    let mut x1 = i64::MIN;
    let mut y1 = i64::MIN;
    for b in layout.get(gl, gd) {
        for p in &b.xy {
            x0 = x0.min(p.x as i64);
            y0 = y0.min(p.y as i64);
            x1 = x1.max(p.x as i64);
            y1 = y1.max(p.y as i64);
        }
    }
    if x0 == i64::MAX { None } else { Some((x0, y0, x1, y1)) }
}

/// Merged coverage (DBU²) of `layer_maps` inside the window `[wx0,wx1] × [wy0,wy1]`.
/// Each cache tile's regions are clipped to `core ∩ window`; cores are disjoint,
/// so a region present in several halo-overlapping tiles is counted once.
fn window_coverage(
    layer_maps: &[&TileMap],
    tile: i64,
    wx0: i64, wy0: i64, wx1: i64, wy1: i64,
) -> f64 {
    let tx0 = wx0.div_euclid(tile);
    let tx1 = (wx1 - 1).div_euclid(tile);
    let ty0 = wy0.div_euclid(tile);
    let ty1 = (wy1 - 1).div_euclid(tile);

    let mut covered = 0.0;
    for map in layer_maps {
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let Some(polys) = map.get(&(tx as i32, ty as i32)) else { continue };
                // core ∩ window
                let cx0 = (tx * tile).max(wx0) as f64;
                let cy0 = (ty * tile).max(wy0) as f64;
                let cx1 = ((tx + 1) * tile).min(wx1) as f64;
                let cy1 = ((ty + 1) * tile).min(wy1) as f64;
                for p in polys {
                    covered += clipped_area_dbu(p, cx0, cy0, cx1, cy1);
                }
            }
        }
    }
    covered
}

pub fn run_min(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64, merged: &mut MergedCache) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, false)
}

pub fn run_max(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64, merged: &mut MergedCache) -> Vec<Violation> {
    run(rule, layout, dbu_to_um, merged, true)
}

fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    is_max: bool,
) -> Vec<Violation> {
    let window = match rule.params.get("window") {
        Some(&w) => w,
        None => {
            eprintln!("[{}] Missing required param 'window'", rule.id);
            return vec![];
        }
    };

    let check = if is_max { "max_windowed_density" } else { "min_windowed_density" };
    let op = if is_max { "<=" } else { ">=" };
    let layer_names: Vec<&str> = rule.layers.iter().map(|l| l.name.as_str()).collect();
    println!(
        "[{}] Checking {} {} {:.2}% on layer(s) [{}] (window: {:.0}x{:.0} µm²)",
        rule.id, check, op, rule.value, layer_names.join(", "), window, window
    );

    let (cx0, cy0, cx1, cy1) = match chip_bbox_dbu(layout) {
        Some(b) => b,
        None => return vec![],
    };
    let win = (window / dbu_to_um).round() as i64;
    if win <= 0 {
        return vec![];
    }

    let boundary_bbox = rule.params.get("boundary_layer").and_then(|&l| {
        let dt = rule.params.get("boundary_datatype").copied().unwrap_or(0.0);
        layer_bbox_dbu(layout, l as i16, dt as i16)
    });

    for l in &rule.layers {
        merged.ensure(layout, l.gds_layer as i16, l.gds_datatype as i16);
    }
    let tile = merged.tile_dbu() as i64;
    let layer_maps: Vec<&TileMap> = rule
        .layers
        .iter()
        .map(|l| merged.tiles(l.gds_layer as i16, l.gds_datatype as i16))
        .collect();

    let cols = ((cx1 - cx0).max(0) / win + 1) as usize;
    let rows = ((cy1 - cy0).max(0) / win + 1) as usize;
    let tiles: Vec<(usize, usize)> = (0..rows).flat_map(|r| (0..cols).map(move |c| (r, c))).collect();
    let value = rule.value;
    let rid = rule.id.as_str();

    tiles
        .par_iter()
        .filter_map(|&(row, col)| {
            let wx0 = cx0 + col as i64 * win;
            let wy0 = cy0 + row as i64 * win;
            let wx1 = (wx0 + win).min(cx1);
            let wy1 = (wy0 + win).min(cy1);
            let area = match boundary_bbox {
                // Denominator is the window's overlap with the boundary layer's
                // bounding box (e.g. EdgeSeal's die extent), not its nominal
                // footprint — an edge/corner window outside the seal ring, or only
                // partly inside it, is measured (or skipped) against the area that's
                // actually checkable.
                Some((bx0, by0, bx1, by1)) => {
                    let ix0 = wx0.max(bx0);
                    let iy0 = wy0.max(by0);
                    let ix1 = wx1.min(bx1);
                    let iy1 = wy1.min(by1);
                    (ix1 - ix0).max(0) as f64 * (iy1 - iy0).max(0) as f64
                }
                None => (wx1 - wx0) as f64 * (wy1 - wy0) as f64,
            };
            if area <= 0.0 {
                return None;
            }

            let covered = window_coverage(&layer_maps, tile, wx0, wy0, wx1, wy1);
            let density = covered / area * 100.0;
            let violated = if is_max { density > value } else { density < value };
            if !violated {
                return None;
            }

            let (ux0, uy0, ux1, uy1) = (
                wx0 as f64 * dbu_to_um, wy0 as f64 * dbu_to_um,
                wx1 as f64 * dbu_to_um, wy1 as f64 * dbu_to_um,
            );
            Some(Violation::edge(
                rid,
                if is_max { "Maximum windowed density violation" } else { "Minimum windowed density violation" },
                format!(
                    "windowed density {:.2}% {} {:.2}% in tile ({:.2}, {:.2})-({:.2}, {:.2}) µm",
                    density, if is_max { ">" } else { "<" }, value, ux0, uy0, ux1, uy1,
                ),
                ux0, uy0, ux1, uy1,
            ))
        })
        .collect()
}
