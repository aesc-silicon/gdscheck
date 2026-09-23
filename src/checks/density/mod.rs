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
//! the layer that *is* the die - GF180's PR_BNDRY, IHP's `EdgeSeal.boundary` - and the
//! measurement is read on its own polygons: the coverage counted is what lies inside
//! them, and the area divided by is theirs.  A die is not always one rectangle (an
//! L-shaped one, or two dies with a street between them are drawn as one boundary layer),
//! and its bounding box then holds ground that is not die: counting that ground into the
//! denominator drops every percentage, and counting geometry that sits on it into the
//! numerator raises them.  A boundary drawn as a ring - a seal ring is a frame - is
//! read as what it rings: its holes are filled first, so the die is the area inside the
//! frame and not the frame's own material.  Without a boundary the box of
//! every shape in the design serves.  For the windowed density the grid still runs over
//! the boundary's box, so the windows are laid the same way whatever the die's shape;
//! what each window is measured against is the die area inside it, so an edge or corner
//! window that hangs over the die's edge (chip dimensions are rarely a multiple of the
//! window) is read against the area that is actually there, and one with no die in it at
//! all is skipped rather than failed.

pub mod region;
#[cfg(test)]
mod tests;

use super::params::{NotAWord, mode};
use crate::layout::FlatLayout;
use crate::merge::{SharedCache, TileMap, clipped_area_dbu};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

/// Which bound a density rule puts on the percentage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
}

impl Kind {
    pub fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
        }
    }

    pub fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
        }
    }

    pub fn bound(self) -> &'static str {
        match self {
            Kind::Min => "Minimum",
            Kind::Max => "Maximum",
        }
    }

    pub fn broken_by(self, density: f64, value: f64) -> bool {
        match self {
            Kind::Min => density < value,
            Kind::Max => density > value,
        }
    }
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Min => "min_density",
            Kind::Max => "max_density",
        }
    }
}

/// Over what a density is read: the rule's `scope` param.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// The whole chip, as one percentage (the default).
    Chip,
    /// Every `window × window` tile of a grid over the chip.
    Window,
    /// Each large connected region of a base layer, see [`region`].
    Region,
}

impl Scope {
    fn parse(rule: &RuleDefinition, name: &str) -> Option<Scope> {
        match mode(rule, name, "scope") {
            Ok(None) | Ok(Some("chip")) => Some(Scope::Chip),
            Ok(Some("window")) => Some(Scope::Window),
            Ok(Some("region")) => Some(Scope::Region),
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] {name}: scope can be `chip`, `window` or `region`, not `{other}`",
                    rule.id
                );
                None
            }
            Err(NotAWord) => None,
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
    layout.bbox()
}

/// The box of the rule's `boundary` layer, if the rule names one and it has shapes.
pub fn boundary_bbox(rule: &RuleDefinition, layout: &FlatLayout) -> Option<Box> {
    let l = rule.num("boundary")?;
    let dt = rule.num("boundary_dt").unwrap_or(0.0);
    bbox_of(layout.get(l as i16, dt as i16).iter())
}

/// The die a boundary layer means: its own polygons together with whatever they ring.
/// A seal ring is drawn as a frame, and the die is the area it encloses, not the frame's
/// own material; a PR_BNDRY drawn as the die itself has no holes and is returned as it
/// is.  The hole contours come from the stitched layer, so a ring split across tiles is
/// one ring, and each is filed into the tiles it covers, clipped to their cores - which
/// is how [`coverage_dbu2`] counts.
fn filled_die(die: &TileMap, tile: i64) -> Option<TileMap> {
    let holes = crate::merge::region_holes(die, tile as i32);
    if holes.is_empty() {
        return None;
    }
    let mut out = die.clone();
    for hole in holes {
        let poly = crate::merge::MergedPoly {
            outer: hole,
            holes: vec![],
        };
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for p in &poly.outer {
            x0 = x0.min(p.x as i64);
            y0 = y0.min(p.y as i64);
            x1 = x1.max(p.x as i64);
            y1 = y1.max(p.y as i64);
        }
        for ty in y0.div_euclid(tile)..=(y1 - 1).div_euclid(tile) {
            for tx in x0.div_euclid(tile)..=(x1 - 1).div_euclid(tile) {
                let (cx0, cy0) = (tx * tile, ty * tile);
                let part =
                    crate::merge::clip_to_box(vec![poly.clone()], cx0, cy0, cx0 + tile, cy0 + tile);
                if !part.is_empty() {
                    out.entry((tx as i32, ty as i32)).or_default().extend(part);
                }
            }
        }
    }
    Some(out)
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
    // Summed per tile row in parallel: a whole-chip window over a 4 mm² design is ten
    // thousand tiles of clipping, which walked on one core while the others waited.
    // A window of the windowed reading is one of many run in parallel already and
    // spans a few rows, so the split costs it nothing.
    (ty0..=ty1)
        .into_par_iter()
        .map(|ty| {
            let mut covered = 0.0;
            for map in maps {
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
            covered
        })
        .sum()
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
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let name = kind.name();
    let Some(scope) = Scope::parse(rule, name) else {
        return vec![];
    };
    if scope == Scope::Region {
        return region::run(kind, rule, layout, dbu_to_um, merged);
    }
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
                eprintln!(
                    "[{}] {name}: `scope: window` needs a `window` in µm",
                    rule.id
                );
                return vec![];
            }
        },
        Scope::Region => unreachable!(),
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
    // The boundary is read as a region, not as its box, so it is merged like any layer.
    let bl = boundary.and(rule.num("boundary").map(|l| {
        let dt = rule.num("boundary_dt").unwrap_or(0.0);
        (l as i16, dt as i16)
    }));
    if let Some((l, dt)) = bl {
        merged.ensure(layout, l, dt);
    }
    let tile = merged.tile_dbu() as i64;
    let layer_arcs: Vec<_> = rule
        .layers
        .iter()
        .map(|l| merged.tiles(l.gds_layer as i16, l.gds_datatype as i16))
        .collect();
    let layer_maps: Vec<&TileMap> = layer_arcs.iter().map(|a| &**a).collect();
    let drawn_die_arc = bl.map(|(l, dt)| merged.tiles(l, dt));
    let drawn_die: Option<&TileMap> = drawn_die_arc.as_deref();
    let filled: Option<TileMap> = drawn_die.and_then(|d| filled_die(d, tile));
    let die_map: Option<&TileMap> = filled.as_ref().or(drawn_die);
    // Several layers are read as their union: the drawing, its filler and its mask are
    // meant to be disjoint, but a mask drawn over the active - or a stripe on all three
    // layers at once - counted three times and read 90 % where 30 % was covered.
    let union: TileMap;
    let maps: Vec<&TileMap> = if layer_maps.len() > 1 {
        let keys: std::collections::HashSet<(i32, i32)> =
            layer_maps.iter().flat_map(|m| m.keys().copied()).collect();
        union = keys
            .into_par_iter()
            .map(|k| {
                let parts: Vec<&[crate::merge::MergedPoly]> = layer_maps
                    .iter()
                    .filter_map(|m| m.get(&k).map(Vec::as_slice))
                    .collect();
                let polys = if parts.len() == 1 {
                    parts[0].to_vec()
                } else {
                    crate::merge::compose_tile(crate::merge::VirtualOp::Union, &parts)
                };
                (k, polys)
            })
            .collect();
        vec![&union]
    } else {
        layer_maps
    };
    // Only what lies inside the die counts: fill sitting on the street between two dies,
    // or in the corner of the box an L-shaped die leaves, is no part of its coverage.
    let clipped: TileMap;
    let maps: Vec<&TileMap> = match die_map {
        None => maps,
        Some(die) => {
            clipped = maps[0]
                .iter()
                .filter_map(|(k, polys)| {
                    let d = die.get(k)?;
                    Some((
                        *k,
                        crate::merge::compose_tile(
                            crate::merge::VirtualOp::Intersection,
                            &[polys.as_slice(), d.as_slice()],
                        ),
                    ))
                })
                .collect();
            vec![&clipped]
        }
    };
    let um2 = dbu_to_um * dbu_to_um;

    let Some(window_um) = window_um else {
        // The whole coverage against the die: the boundary's own polygons if the rule
        // names a layer and it is drawn, else the box of everything.  What lies outside
        // the boundary is no part of the die and counts for nothing - a block checked
        // with its boundary drawn smaller than its metal read 125 % otherwise.
        let die = boundary.unwrap_or(chip);
        let denominator = match die_map {
            Some(m) => coverage_dbu2(&[m], tile, die) * um2,
            None => area(die) * um2,
        };
        if denominator == 0.0 {
            eprintln!("[{}] Could not compute density", rule.id);
            return vec![];
        }
        let covered = coverage_dbu2(&maps, tile, die) * um2;
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

    // "Any window x window area": the windows slide over the die a merge tile at a
    // time, from the die's corner, and one more is laid against each far edge, so
    // every part of the die is in some whole window and no window is a clipped
    // remainder - a 200 µm strip read as a window of its own reported the density of
    // what happened to lie there, and a hole a step off the grid was in no window at
    // all.  The coverage is summed once per tile core and the windows on the tile grid
    // are read off a summed-area table; the edge-anchored ones are clipped exactly.
    // Overlapping violating windows are one violation, reported at the worst of them.
    let win = (window_um / dbu_to_um).round() as i64;
    let (bx0, by0, bx1, by1) = boundary.unwrap_or(chip);
    let per_tile: HashMap<(i32, i32), f64> = maps
        .iter()
        .flat_map(|m| m.keys().copied())
        .collect::<std::collections::HashSet<_>>()
        .into_par_iter()
        .map(|(tx, ty)| {
            let core = (
                tx as i64 * tile,
                ty as i64 * tile,
                (tx as i64 + 1) * tile,
                (ty as i64 + 1) * tile,
            );
            ((tx, ty), coverage_dbu2(&maps, tile, core))
        })
        .collect();
    let (tx0, ty0) = (bx0.div_euclid(tile), by0.div_euclid(tile));
    let (tx1, ty1) = (
        (bx1 - 1).div_euclid(tile) + 1,
        (by1 - 1).div_euclid(tile) + 1,
    );
    let (nx, ny) = ((tx1 - tx0) as usize, (ty1 - ty0) as usize);
    // Summed-area table over the tile grid of the die, one cell wider each way.
    let mut sat = vec![0.0f64; (nx + 1) * (ny + 1)];
    for iy in 0..ny {
        for ix in 0..nx {
            let c = per_tile
                .get(&((tx0 + ix as i64) as i32, (ty0 + iy as i64) as i32))
                .copied()
                .unwrap_or(0.0);
            sat[(iy + 1) * (nx + 1) + ix + 1] =
                c + sat[iy * (nx + 1) + ix + 1] + sat[(iy + 1) * (nx + 1) + ix]
                    - sat[iy * (nx + 1) + ix];
        }
    }
    let sum = |ix0: usize, iy0: usize, ix1: usize, iy1: usize| -> f64 {
        sat[iy1 * (nx + 1) + ix1] - sat[iy0 * (nx + 1) + ix1] - sat[iy1 * (nx + 1) + ix0]
            + sat[iy0 * (nx + 1) + ix0]
    };
    // Window origins: every tile line from the die's first, as long as the window
    // stays in the die, then the die's far edge less a window.
    let origins = |lo: i64, hi: i64, t0: i64| -> Vec<(i64, bool)> {
        if hi - lo <= win {
            return vec![(lo, false)];
        }
        let mut v: Vec<(i64, bool)> = (0..)
            .map(|k| (t0 + k) * tile)
            .skip_while(|&o| o < lo)
            .take_while(|&o| o + win <= hi)
            .map(|o| (o, true))
            .collect();
        if v.last().is_none_or(|&(o, _)| o + win < hi) {
            v.push((hi - win, false));
        }
        v
    };
    let xs = origins(bx0, bx1, tx0);
    let ys = origins(by0, by1, ty0);
    let cells: Vec<(usize, usize)> = (0..ys.len())
        .flat_map(|j| (0..xs.len()).map(move |i| (i, j)))
        .collect();
    let readings: Vec<Option<(Box, f64)>> = cells
        .par_iter()
        .map(|&(i, j)| {
            let ((wx0, ax), (wy0, ay)) = (xs[i], ys[j]);
            let window = (wx0, wy0, (wx0 + win).min(bx1), (wy0 + win).min(by1));
            let clip = intersect(window, (bx0, by0, bx1, by1));
            let denominator = match die_map {
                Some(m) => coverage_dbu2(&[m], tile, clip),
                None => area(clip),
            };
            if denominator <= 0.0 {
                return None;
            }
            let covered = if ax && ay && win % tile == 0 {
                let (ix, iy) = (((wx0 / tile) - tx0) as usize, ((wy0 / tile) - ty0) as usize);
                let n = (win / tile) as usize;
                sum(ix, iy, (ix + n).min(nx), (iy + n).min(ny))
            } else {
                coverage_dbu2(&maps, tile, window)
            };
            let density = covered / denominator * 100.0;
            kind.broken_by(density, rule.value)
                .then_some((window, density))
        })
        .collect();
    // Cluster the violating windows: neighbours on the origin grid that both violate
    // are one violation.
    let mut uf = crate::merge::UnionFind::new(cells.len());
    for (k, &(i, j)) in cells.iter().enumerate() {
        if readings[k].is_none() {
            continue;
        }
        if i + 1 < xs.len() && readings[k + 1].is_some() {
            uf.union(k, k + 1);
        }
        if j + 1 < ys.len() && readings[k + xs.len()].is_some() {
            uf.union(k, k + xs.len());
        }
    }
    let mut worst: HashMap<usize, (Box, f64)> = HashMap::new();
    for (k, r) in readings.iter().enumerate() {
        let Some((window, density)) = *r else {
            continue;
        };
        let root = uf.find(k);
        let e = worst.entry(root).or_insert((window, density));
        let further = match kind {
            Kind::Min => density < e.1,
            Kind::Max => density > e.1,
        };
        if further {
            *e = (window, density);
        }
    }
    let rid = rule.id.as_str();
    let mut found: Vec<(Box, f64)> = worst.into_values().collect();
    found.sort_by_key(|a| a.0);
    found
        .into_iter()
        .map(|(window, density)| {
            let (ux0, uy0, ux1, uy1) = (
                window.0 as f64 * dbu_to_um,
                window.1 as f64 * dbu_to_um,
                window.2 as f64 * dbu_to_um,
                window.3 as f64 * dbu_to_um,
            );
            Violation::edge(
                rid,
                &format!("{} windowed density violation", kind.bound()),
                format!(
                    "windowed density {density:.2}% {} {:.2}% in window ({ux0:.2}, {uy0:.2})-({ux1:.2}, {uy1:.2}) µm",
                    kind.cmp(),
                    rule.value
                ),
                ux0,
                uy0,
                ux1,
                uy1,
            )
        })
        .collect()
}
