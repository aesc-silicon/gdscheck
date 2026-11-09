// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shape: what a region or a segment is on its own - its extents, the length of its
//! edges, the angles they run at and turn by, its holes and rings, whether its
//! vertices sit on the grid and its centreline on a track.  Nothing here measures one
//! shape against another.
//!
//! The bounded readings - an extent, an edge length - come in three kinds, at least, at
//! most and exactly, on one driver each, the way the width family does.

pub mod angle;
pub mod corner;
pub mod edge_length;
pub mod extent;
pub mod holes;
pub mod offgrid;
pub mod offtrack;
pub mod ring;
pub mod ring_covers_boundary;
pub mod vertices;
pub mod wide_uncovered;

/// Which bound a rule puts on what it reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
    Exact,
}

impl Kind {
    /// The requirement, as the log prints it.
    pub fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
            Kind::Exact => "==",
        }
    }

    /// The failing comparison, as the message prints it.
    pub fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
            Kind::Exact => "≠",
        }
    }

    /// The bound on the grid.
    pub fn limit(self, value_um: f64, dbu_to_um: f64) -> crate::geom::Limit {
        match self {
            Kind::Min => crate::geom::Limit::at_least(value_um, dbu_to_um),
            Kind::Max => crate::geom::Limit::at_most(value_um, dbu_to_um),
            Kind::Exact => crate::geom::Limit::exactly(value_um, dbu_to_um),
        }
    }

    /// The word a violation's title starts with.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Min => "Minimum",
            Kind::Max => "Maximum",
            Kind::Exact => "Exact",
        }
    }
}

/// Each stitched region of a layer, as its marker and its bounding box.  The box is the
/// union of the region's pieces' boxes, each piece cut to its tile core, so a region of
/// any size is measured whole without any tile holding a whole copy of it - which is
/// what the extent checks used to need, a halo the size of their value on the drawn
/// layers under the region: MDP.13a's 50 µm on a dense COMP was 21 copies of every
/// shape.  The caller has ensured the layer in the cache.
pub fn region_boxes(
    merged: &crate::merge::SharedCache,
    gl: i16,
    gd: i16,
) -> Vec<((f64, f64), crate::merge::BBoxDbu)> {
    region_boxes_areas(merged, gl, gd)
        .into_iter()
        .map(|(marker, b, _)| (marker, b))
        .collect()
}

/// The regions of [`region_boxes`] that are rectangles - whose area fills their box - as
/// their marker and box: a wire on a routing layer that may not bend, read by its extent.
pub fn region_rects(
    merged: &crate::merge::SharedCache,
    gl: i16,
    gd: i16,
) -> Vec<((f64, f64), crate::merge::BBoxDbu)> {
    region_boxes_areas(merged, gl, gd)
        .into_iter()
        .filter(|&(_, (x0, y0, x1, y1), area)| {
            let full = (x1 as f64 - x0 as f64) * (y1 as f64 - y0 as f64);
            (full - area).abs() < 0.5
        })
        .map(|(marker, b, _)| (marker, b))
        .collect()
}

/// [`region_boxes`] with each region's area in DBU².
fn region_boxes_areas(
    merged: &crate::merge::SharedCache,
    gl: i16,
    gd: i16,
) -> Vec<((f64, f64), crate::merge::BBoxDbu, f64)> {
    use rayon::prelude::*;
    let tile = merged.tile_dbu() as i64;
    let labeled = crate::merge::stitch_labeled(&merged.tiles(gl, gd), merged.tile_dbu());
    let per_tile: Vec<Vec<(usize, crate::merge::BBoxDbu)>> = labeled
        .by_tile
        .par_iter()
        .map(|(&(tx, ty), polys)| {
            let core = (
                tx as i64 * tile,
                ty as i64 * tile,
                (tx as i64 + 1) * tile,
                (ty as i64 + 1) * tile,
            );
            polys
                .iter()
                .filter_map(|(m, r)| crate::merge::core_clipped_bbox(m, core).map(|b| (*r, b)))
                .collect()
        })
        .collect();
    let mut bbox: Vec<Option<crate::merge::BBoxDbu>> = vec![None; labeled.regions.len()];
    for v in per_tile {
        for (r, b) in v {
            bbox[r] = Some(match bbox[r] {
                None => b,
                Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
            });
        }
    }
    labeled
        .regions
        .iter()
        .zip(bbox)
        .filter_map(|(region, b)| Some((region.marker, b?, region.area_dbu)))
        .collect()
}
