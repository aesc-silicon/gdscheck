// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Density: the merged coverage of one or more layers over an area, as a percentage.
//!
//! One measurement, [`coverage_dbu2`], read at three scopes.  The **chip** density is the
//! layers' whole coverage over the chip's bounding box; the **windowed** density the same
//! fraction in every `window × window` tile of a grid laid over the chip; the **region**
//! density, in [`region`], the fraction of each large connected region of a base layer
//! that a feature layer covers.  Coverage is summed on the shared [`MergedCache`], so
//! overlapping and nested shapes count once, and exactly, by clipping each cache tile's
//! regions to the tile core and to the window: cores are disjoint, so a region a halo
//! copies into several tiles is counted once.
//!
//! The denominator is an area, not a layer's material.  A `boundary` layer param names
//! the layer whose *bounding box* stands for the die: a seal ring is drawn as a hollow
//! frame, so its own merged area is the thin frame and would wildly undercount the
//! region it encloses, where its box is the die extent it is meant to mean.  Without a
//! boundary the box of every shape in the design serves.  For the windowed density the
//! grid always starts at the chip's raw box; the boundary box only restricts what part of
//! each window counts, so an edge or corner window that falls outside the seal ring
//! (chip dimensions are rarely a multiple of the window) is measured against the area
//! that is actually there, and one with no overlap at all is skipped rather than failed.

pub mod region;
#[cfg(test)]
mod tests;

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, TileMap, clipped_area_dbu};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

/// Which bound a density rule puts on the percentage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
}

impl Kind {
    fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
        }
    }

    fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
        }
    }

    fn bound(self) -> &'static str {
        match self {
            Kind::Min => "Minimum",
            Kind::Max => "Maximum",
        }
    }

    fn broken_by(self, density: f64, value: f64) -> bool {
        match self {
            Kind::Min => density < value,
            Kind::Max => density > value,
        }
    }
}

/// Over what a density is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// The whole chip, as one percentage.
    Chip,
    /// Every `window × window` tile of a grid over the chip.
    Window,
}

impl Scope {
    fn name(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Scope::Chip, Kind::Min) => "min_density",
            (Scope::Chip, Kind::Max) => "max_density",
            (Scope::Window, Kind::Min) => "min_windowed_density",
            (Scope::Window, Kind::Max) => "max_windowed_density",
        }
    }
}

/// A box in DBU: `(x0, y0, x1, y1)`.
pub type Box = (i64, i64, i64, i64);

fn bbox_of<'a>(shapes: impl Iterator<Item = &'a gds21::GdsBoundary>) -> Option<Box> {
    let mut b = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for s in shapes {
        for p in &s.xy {
            b.0 = b.0.min(p.x as i64);
            b.1 = b.1.min(p.y as i64);
            b.2 = b.2.max(p.x as i64);
            b.3 = b.3.max(p.y as i64);
        }
    }
    (b.0 != i64::MAX).then_some(b)
}

/// The box of every shape in the design.
pub fn chip_bbox(layout: &FlatLayout) -> Option<Box> {
    bbox_of(layout.all_boundaries())
}

/// The box of the rule's `boundary` layer, if the rule names one and it has shapes.
pub fn boundary_bbox(rule: &RuleDefinition, layout: &FlatLayout) -> Option<Box> {
    let l = rule.num("boundary")?;
    let dt = rule.num("boundary_dt").unwrap_or(0.0);
    bbox_of(layout.get(l as i16, dt as i16).iter())
}

/// Merged coverage (DBU²) of `maps` inside the window.  Each cache tile's regions are
/// clipped to `core ∩ window`; cores are disjoint, so a region present in several
/// halo-overlapping tiles is counted once.
pub fn coverage_dbu2(maps: &[&TileMap], tile: i64, window: Box) -> f64 {
    let (wx0, wy0, wx1, wy1) = window;
    let tx0 = wx0.div_euclid(tile);
    let tx1 = (wx1 - 1).div_euclid(tile);
    let ty0 = wy0.div_euclid(tile);
    let ty1 = (wy1 - 1).div_euclid(tile);
    let mut covered = 0.0;
    for map in maps {
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let Some(polys) = map.get(&(tx as i32, ty as i32)) else {
                    continue;
                };
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

fn area(b: Box) -> f64 {
    (b.2 - b.0).max(0) as f64 * (b.3 - b.1).max(0) as f64
}

fn intersect(a: Box, b: Box) -> Box {
    (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3))
}

/// Run one density rule.
pub fn run(
    kind: Kind,
    scope: Scope,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let name = scope.name(kind);
    let layer_names = rule
        .layers
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let window_um = match scope {
        Scope::Chip => None,
        Scope::Window => match rule.num("window") {
            Some(w) if w > 0.0 => Some(w),
            _ => {
                eprintln!("[{}] {name} needs a `window` in µm", rule.id);
                return vec![];
            }
        },
    };
    match window_um {
        None => println!(
            "[{}] Checking {name} {} {:.2}% on layer(s) [{layer_names}]",
            rule.id,
            kind.op(),
            rule.value
        ),
        Some(w) => println!(
            "[{}] Checking {name} {} {:.2}% on layer(s) [{layer_names}] (window: {w:.0}x{w:.0} µm²)",
            rule.id,
            kind.op(),
            rule.value
        ),
    }
    let Some(chip) = chip_bbox(layout) else {
        if scope == Scope::Chip {
            eprintln!("[{}] Could not compute density", rule.id);
        }
        return vec![];
    };
    let boundary = boundary_bbox(rule, layout);
    for l in &rule.layers {
        merged.ensure(layout, l.gds_layer as i16, l.gds_datatype as i16);
    }
    let tile = merged.tile_dbu() as i64;
    let maps: Vec<&TileMap> = rule
        .layers
        .iter()
        .map(|l| merged.tiles(l.gds_layer as i16, l.gds_datatype as i16))
        .collect();
    let um2 = dbu_to_um * dbu_to_um;

    let Some(window_um) = window_um else {
        // The whole coverage against the die: the boundary's box if the rule names one
        // and it is drawn, else the box of everything.
        let denominator = area(boundary.unwrap_or(chip)) * um2;
        if denominator == 0.0 {
            eprintln!("[{}] Could not compute density", rule.id);
            return vec![];
        }
        let covered = coverage_dbu2(&maps, tile, chip) * um2;
        let density = (covered / denominator * 100.0 * 1000.0).round() / 1000.0;
        println!("[{}] Density: {density:.2}%", rule.id);
        if !kind.broken_by(density, rule.value) {
            return vec![];
        }
        return vec![Violation::global(
            &rule.id,
            &format!("{} density violation", kind.bound()),
            format!(
                "density {density:.2}% {} {:.2}% on layer(s) [{layer_names}]",
                kind.cmp(),
                rule.value
            ),
        )];
    };

    let win = (window_um / dbu_to_um).round() as i64;
    let (cx0, cy0, cx1, cy1) = chip;
    let cols = ((cx1 - cx0).max(0) / win + 1) as usize;
    let rows = ((cy1 - cy0).max(0) / win + 1) as usize;
    let cells: Vec<(usize, usize)> = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .collect();
    let rid = rule.id.as_str();
    cells
        .par_iter()
        .filter_map(|&(row, col)| {
            let wx0 = cx0 + col as i64 * win;
            let wy0 = cy0 + row as i64 * win;
            let window = (wx0, wy0, (wx0 + win).min(cx1), (wy0 + win).min(cy1));
            // What part of the window is there to measure: its overlap with the
            // boundary's box, or all of it.
            let denominator = area(boundary.map_or(window, |b| intersect(window, b)));
            if denominator <= 0.0 {
                return None;
            }
            let density = coverage_dbu2(&maps, tile, window) / denominator * 100.0;
            if !kind.broken_by(density, rule.value) {
                return None;
            }
            let (ux0, uy0, ux1, uy1) = (
                window.0 as f64 * dbu_to_um,
                window.1 as f64 * dbu_to_um,
                window.2 as f64 * dbu_to_um,
                window.3 as f64 * dbu_to_um,
            );
            Some(Violation::edge(
                rid,
                &format!("{} windowed density violation", kind.bound()),
                format!(
                    "windowed density {density:.2}% {} {:.2}% in tile ({ux0:.2}, {uy0:.2})-({ux1:.2}, {uy1:.2}) µm",
                    kind.cmp(),
                    rule.value
                ),
                ux0,
                uy0,
                ux1,
                uy1,
            ))
        })
        .collect()
}
