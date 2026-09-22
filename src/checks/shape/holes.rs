// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `no_hole`: a region of the layer must not have a hole - a MIM plate, a bond pad, a
//! fill shape with a void in it.  KLayout's `holes`, reported rather than derived.
//!
//! Holes are the stitched region's, read once each (see [`region_holes`]): a hole a
//! tile line runs through, or one wider than a tile, is the region's and no copy's.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, region_holes};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] no_hole needs a layer", rule.id);
        return vec![];
    };
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);
    println!("[{}] Checking no_hole on layer {}", rule.id, layer.name);

    let mut out = Vec::new();
    for hole in region_holes(merged.tiles(gl, gd), merged.tile_dbu()) {
        if hole.len() < 3 {
            continue;
        }
        let n = hole.len() as f64;
        let (cx, cy) = hole.iter().fold((0.0, 0.0), |(sx, sy), p| {
            (sx + p.x as f64 / n, sy + p.y as f64 / n)
        });
        let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
        out.push(Violation::point(
            &rule.id,
            "Hole violation",
            format!("{}: region has a hole at ({x:.4}, {y:.4}) µm", layer.name),
            x,
            y,
        ));
    }
    out
}
