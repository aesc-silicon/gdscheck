// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Full-precision geometric merge of a layer's shapes.
//!
//! Real GDS layouts draw a single physical region as many overlapping or
//! edge-abutting polygons (wires built from segments, vias stacked on metal,
//! filler abutting drawn metal, …).  Width / spacing / area / density checks
//! must operate on the *merged* geometry, otherwise they see artificial inner
//! edges and report violations that do not exist on the manufactured layer.
//!
//! Merging is done with `i_overlay`'s sweep-line `NonZero` union.  Coordinates
//! are the integer DBU values carried as `f64`; DBU magnitudes (≤ ~1e6) are far
//! below 2^53, so this is exact for the manufacturing grid and the result is
//! rounded back to integer DBU.

use crate::layout::FlatLayout;
use gds21::GdsBoundary;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::simplify::SimplifyShape;
use i_overlay::float::single::SingleFloatOverlay;
use i_overlay::i_float::int::point::IntPoint;
use i_overlay::mesh::outline::offset::OutlineOffset;
use i_overlay::mesh::style::OutlineStyle;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Tile size for the tiled merge (µm).
pub const TILE_UM: f64 = 20.0;
/// Minimum merge halo (µm); a run uses the larger of this and the biggest
/// geometric rule distance in the deck.
pub const MIN_HALO_UM: f64 = 1.0;

/// Canonical key for exact-duplicate detection: the integer vertex sequence
/// rotated to start at its lexicographically smallest vertex.  Inputs are
/// already winding-normalised, so identical shapes collapse regardless of which
/// vertex the GDS polygon happened to start on.
fn canonical_key(pts: &[[f64; 2]]) -> Vec<(i32, i32)> {
    let ints: Vec<(i32, i32)> = pts.iter().map(|p| (p[0] as i32, p[1] as i32)).collect();
    let n = ints.len();
    if n == 0 {
        return ints;
    }
    let mut start = 0;
    for i in 1..n {
        if ints[i] < ints[start] {
            start = i;
        }
    }
    (0..n).map(|k| ints[(start + k) % n]).collect()
}

/// A merged region in integer DBU.
///
/// `outer` is counter-clockwise and each contour in `holes` is clockwise, so the
/// metal (filled area) lies on the **left** of every directed edge — the
/// convention the geometric checks rely on.
#[derive(Clone)]
pub struct MergedPoly {
    pub outer: Vec<IntPoint>,
    pub holes: Vec<Vec<IntPoint>>,
}

/// Twice the signed area of a closed contour (shoelace), in DBU².
/// Positive ⇒ counter-clockwise.  Uses i64 to avoid overflow on large chips.
fn signed_area2_xy(pts: &[gds21::GdsPoint]) -> i64 {
    let n = pts.len();
    let mut acc: i64 = 0;
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        acc += pts[i].x as i64 * pts[j].y as i64;
        acc -= pts[j].x as i64 * pts[i].y as i64;
    }
    acc
}

/// Union the given boundaries into non-overlapping regions at full integer
/// precision.  Touching and overlapping shapes are dissolved into single
/// regions; enclosed empty space becomes a hole.
pub fn merge_boundaries(boundaries: &[GdsBoundary]) -> Vec<MergedPoly> {
    merge_iter(boundaries.iter())
}

/// Like [`merge_boundaries`] but over a slice of references — used by the tiled
/// path, where each tile owns references into the shared layer.
pub fn merge_refs(boundaries: &[&GdsBoundary]) -> Vec<MergedPoly> {
    merge_iter(boundaries.iter().copied())
}

/// Convert GDS boundaries into i_overlay float single-contour shapes.
///
/// Each GDS boundary becomes a single-contour shape.  GDS rings repeat the first
/// vertex as the last; drop it (i_overlay contours are open).  Every contour is
/// normalised to CCW (positive area) so NonZero fill treats them all as solid.
///
/// Coordinates are carried as f64 of the integer DBU values: DBU magnitudes
/// (≤ ~1e6) are far below 2^53, so this is exact for the grid.  Identical
/// polygons are deduplicated — coincident edges make the sweep-line degenerate
/// (quadratic memory); removing them is correct and the difference between a few
/// MB and tens of GB.  The key is the CCW vertex sequence rotated to its
/// lexicographically smallest vertex, so duplicates that differ only in start
/// vertex or winding also collapse.
fn boundaries_to_shapes<'a>(it: impl Iterator<Item = &'a GdsBoundary>) -> Vec<Vec<Vec<[f64; 2]>>> {
    let mut seen: HashSet<Vec<(i32, i32)>> = HashSet::new();
    it.filter_map(|b| {
        let n = b.xy.len().saturating_sub(1);
        if n < 3 {
            return None;
        }
        let area2 = signed_area2_xy(&b.xy[..n]);
        let mut pts: Vec<[f64; 2]> = b.xy[..n].iter().map(|p| [p.x as f64, p.y as f64]).collect();
        if area2 < 0 {
            pts.reverse();
        }
        if !seen.insert(canonical_key(&pts)) {
            return None;
        }
        Some(vec![pts])
    })
    .collect()
}

/// Convert i_overlay float shapes back to integer-DBU `MergedPoly`s.
fn shapes_to_merged(shapes: Vec<Vec<Vec<[f64; 2]>>>) -> Vec<MergedPoly> {
    let to_int = |c: Vec<[f64; 2]>| -> Vec<IntPoint> {
        c.into_iter()
            .map(|p| IntPoint::new(p[0].round() as i32, p[1].round() as i32))
            .collect()
    };
    shapes
        .into_iter()
        .filter_map(|mut shape| {
            if shape.is_empty() {
                return None;
            }
            let outer = to_int(shape.remove(0));
            let holes = shape.into_iter().map(to_int).collect();
            Some(MergedPoly { outer, holes })
        })
        .collect()
}

fn merge_iter<'a>(it: impl Iterator<Item = &'a GdsBoundary>) -> Vec<MergedPoly> {
    let shapes = boundaries_to_shapes(it);
    if shapes.is_empty() {
        return Vec::new();
    }
    shapes_to_merged(shapes.simplify_shape(FillRule::NonZero))
}

/// Geometric intersection (AND) of several layers' boundaries: the region covered
/// by *every* layer.  Each layer is unioned first (so internal overlaps resolve),
/// then the layers are intersected pairwise.  Used for device-recognition virtual
/// layers (e.g. `CuPillarPad = Passiv.pillar AND dfpad`): if any input layer is
/// empty, the result is empty.
pub fn intersect_layers(layers: &[&[GdsBoundary]]) -> Vec<MergedPoly> {
    let Some((first, rest)) = layers.split_first() else {
        return Vec::new();
    };
    let mut acc = boundaries_to_shapes(first.iter()).simplify_shape(FillRule::NonZero);
    for lyr in rest {
        if acc.is_empty() {
            return Vec::new();
        }
        let clip = boundaries_to_shapes(lyr.iter()).simplify_shape(FillRule::NonZero);
        acc = acc.overlay(&clip, OverlayRule::Intersect, FillRule::NonZero);
    }
    shapes_to_merged(acc)
}

/// Geometric difference (NOT): the region of `base` not covered by any of `clips`.
/// `base` and each clip are unioned first, then the clips are subtracted in turn.
/// Used for subtraction virtual layers (e.g. `ContNoSealring = Cont NOT EdgeSeal`).
/// Returns merged regions with holes preserved.
pub fn difference_layers(base: &[GdsBoundary], clips: &[&[GdsBoundary]]) -> Vec<MergedPoly> {
    let mut acc = boundaries_to_shapes(base.iter()).simplify_shape(FillRule::NonZero);
    for clip in clips {
        if acc.is_empty() {
            return Vec::new();
        }
        let c = boundaries_to_shapes(clip.iter()).simplify_shape(FillRule::NonZero);
        if c.is_empty() {
            continue; // nothing to subtract
        }
        acc = acc.overlay(&c, OverlayRule::Difference, FillRule::NonZero);
    }
    shapes_to_merged(acc)
}

/// Morphological opening of merged polygons by `radius` (in the polygons' own DBU units):
/// erode by `radius` then dilate by `radius`, which deletes any feature narrower than
/// `2 * radius`.  Used to extract the parts of a layer that are genuinely wide in every
/// direction — e.g. Slt.c's "metal wider than 30 µm" (radius = 15 µm).
pub fn opening(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    if polys.is_empty() || radius <= 0.0 {
        return polys.to_vec();
    }
    let eroded = tile_shapes(polys).outline(&OutlineStyle::new(-radius));
    if eroded.is_empty() {
        return Vec::new();
    }
    shapes_to_merged(eroded.outline(&OutlineStyle::new(radius)))
}

/// Morphological *closing*: dilate by `radius` (DBU) then erode by `radius`.  Merges gaps
/// narrower than `2 * radius` into one region (and fills concavities), while leaving the
/// outer edges of regions that stay separate unchanged.  Used for "same-net merge" spacing
/// rules (e.g. NW.b/b1: NWells closer than 0.62 µm are treated as one net).
pub fn closing(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    if polys.is_empty() || radius <= 0.0 {
        return polys.to_vec();
    }
    let dilated = tile_shapes(polys).outline(&OutlineStyle::new(radius));
    if dilated.is_empty() {
        return Vec::new();
    }
    shapes_to_merged(dilated.outline(&OutlineStyle::new(-radius)))
}

/// Morphological dilate (grow) by `radius` DBU, one-directional — no erode back, unlike
/// [`closing`].  Used to build a "within distance X" halo region for a subsequent
/// `not_interacting` selection (e.g. Rppd.c/Rhi.d's "Cont must be near SalBlock": grow
/// SalBlock by the rule value, then flag Cont that still doesn't touch it).
///
/// Adds a small (5 DBU = 5 nm) extra margin beyond `radius`.  Without it, a target polygon
/// drawn *exactly* at the boundary distance would have its edge exactly coincide with the
/// grown region's edge — a zero-area touch — and i_overlay's `Intersect` (which
/// `polys_overlap`/`not_interacting` are built on) treats that as *no* overlap, so a
/// legitimately-compliant polygon would misclassify as "too far".  The extra margin turns
/// an exact touch into a hairline genuine overlap, which a whole-region boolean test
/// (unlike an area-difference test) correctly resolves with an amount this small.  5 nm
/// (not the usual 0.5 DBU tolerance used elsewhere) is deliberate: empirically, `outline`'s
/// offset on a shape with 45°-chamfered corners applied a sub-nm-scale epsilon
/// inconsistently between the two facing edges — 5 nm reliably clears it on both, and is
/// still far below any real DRC-scale feature size.
pub fn grow(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    if polys.is_empty() {
        return Vec::new();
    }
    shapes_to_merged(tile_shapes(polys).outline(&OutlineStyle::new(radius + 5.0)))
}

/// Maximum-space / proximity-coverage gaps (latch-up LU.a–d): every part of `a` must lie
/// within `value` (DBU) of `b`.  Returns a point inside each part of `a` that is *not* —
/// i.e. the residue of `a − dilate(b, value)`.  Tiled: `b` is read and dilated only within
/// each tile's `value`-neighbourhood, so a dense layer (e.g. Cont) is never globally
/// unioned (which OOMs).
pub fn max_space_gaps(a: &TileMap, b: &TileMap, value: f64, tile_dbu: i32) -> Vec<(f64, f64)> {
    let t = tile_dbu as i64;
    let b_count: usize = b.values().map(|ps| ps.len()).sum();

    // Dense reference (e.g. the contacts on ties, thousands of them): a single global dilate
    // would hit i_overlay's super-linear `simplify`, so query `b` local to each `a` polygon
    // and subtract the coverage in small early-exiting batches.  Each `a` is clipped to the
    // tile core first, so even a large tie's query stays tile-sized.
    if b_count > 4000 {
        return a
            .par_iter()
            .flat_map_iter(move |(&(tx, ty), a_polys)| {
                let cx0 = (tx as i64 * t) as f64;
                let cy0 = (ty as i64 * t) as f64;
                let cx1 = ((tx as i64 + 1) * t) as f64;
                let cy1 = ((ty as i64 + 1) * t) as f64;
                let core_box = vec![vec![[cx0, cy0], [cx1, cy0], [cx1, cy1], [cx0, cy1]]];
                let mut out = Vec::new();
                for p in a_polys {
                    if clipped_area_dbu(p, cx0, cy0, cx1, cy1) <= 0.5 {
                        continue;
                    }
                    let a_core = merged_to_shape(p)
                        .overlay(&core_box, OverlayRule::Intersect, FillRule::NonZero);
                    if a_core.is_empty() {
                        continue;
                    }
                    let (mut qx0, mut qy0, mut qx1, mut qy1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                    for s in &a_core {
                        for c in s {
                            for pt in c {
                                qx0 = qx0.min(pt[0]);
                                qy0 = qy0.min(pt[1]);
                                qx1 = qx1.max(pt[0]);
                                qy1 = qy1.max(pt[1]);
                            }
                        }
                    }
                    let (qx0, qy0, qx1, qy1) = (qx0 - value, qy0 - value, qx1 + value, qy1 + value);
                    // Coverage = each reference shape dilated by `value` with a *square*
                    // structuring element (its bbox grown by `value`), matching KLayout's
                    // `sized`.  We deliberately do NOT use i_overlay's `outline` offset here:
                    // it is unreliable when the offset (e.g. 6 µm) dwarfs the feature size
                    // (0.16 µm contacts), under-covering and producing false gaps.
                    let grown: Vec<Vec<Vec<[f64; 2]>>> = (((qy0 / t as f64).floor() as i32)..=((qy1 / t as f64).floor() as i32))
                        .flat_map(|qy| (((qx0 / t as f64).floor() as i32)..=((qx1 / t as f64).floor() as i32)).map(move |qx| (qx, qy)))
                        .filter_map(|(qx, qy)| b.get(&(qx, qy)))
                        .flat_map(|ps| ps.iter())
                        .filter_map(|bp| {
                            let (bx0, by0, bx1, by1) = bp.outer.iter().fold(
                                (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
                                |(x0, y0, x1, y1), p| (x0.min(p.x as f64), y0.min(p.y as f64), x1.max(p.x as f64), y1.max(p.y as f64)),
                            );
                            if bx1 >= qx0 && bx0 <= qx1 && by1 >= qy0 && by0 <= qy1 {
                                let (gx0, gy0, gx1, gy1) = (bx0 - value, by0 - value, bx1 + value, by1 + value);
                                Some(vec![vec![[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]]])
                            } else {
                                None
                            }
                        })
                        .collect();
                    // Subtract the coverage from `a_core` in small batches, stopping as soon
                    // as nothing is left.  Unioning thousands of heavily-overlapping grown
                    // rectangles at once makes `simplify` blow up (super-linear, OOM); a
                    // dense contact field instead covers `a_core` after a handful of batches,
                    // so we keep each `simplify` tiny and bail early.
                    let mut remaining = a_core;
                    for batch in grown.chunks(256) {
                        if remaining.is_empty() {
                            break;
                        }
                        let cov = batch.to_vec().simplify_shape(FillRule::NonZero);
                        remaining = remaining.overlay(&cov, OverlayRule::Difference, FillRule::NonZero);
                    }
                    let gaps = remaining;
                    for g in shapes_to_merged(gaps) {
                        if clipped_area_dbu(&g, cx0, cy0, cx1, cy1) > 0.5 {
                            out.push(merged_centroid_dbu(&g));
                        }
                    }
                }
                out.into_iter()
            })
            .collect();
    }

    // Sparse reference (ties — a few hundred polygons even if large): dilate it ONCE globally
    // (cheap for i_overlay), bucket the result, and difference `a` against it per tile.
    let all_b: Vec<Vec<Vec<[f64; 2]>>> =
        b.values().flat_map(|ps| ps.iter().map(merged_to_shape)).collect();
    let covered_polys: Vec<MergedPoly> = if all_b.is_empty() {
        Vec::new()
    } else {
        shapes_to_merged(all_b.simplify_shape(FillRule::NonZero).outline(&OutlineStyle::new(value)))
    };
    // Bucket each dilated-`b` polygon into the tiles its bbox covers.
    let mut covered_tiles: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, cp) in covered_polys.iter().enumerate() {
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for p in &cp.outer {
            x0 = x0.min(p.x as i64);
            y0 = y0.min(p.y as i64);
            x1 = x1.max(p.x as i64);
            y1 = y1.max(p.y as i64);
        }
        for ty in y0.div_euclid(t)..=y1.div_euclid(t) {
            for tx in x0.div_euclid(t)..=x1.div_euclid(t) {
                covered_tiles.entry((tx as i32, ty as i32)).or_default().push(i);
            }
        }
    }

    a.par_iter()
        .flat_map_iter(move |(&(tx, ty), a_polys)| {
            let cx0 = (tx as i64 * t) as f64;
            let cy0 = (ty as i64 * t) as f64;
            let cx1 = ((tx as i64 + 1) * t) as f64;
            let cy1 = ((ty as i64 + 1) * t) as f64;
            let mut out = Vec::new();
            // The `a` tiles are already merged, so take them as-is (no per-poly clip — that
            // boolean op per polygon is what was slow on dense device layers).  Gaps are
            // de-duplicated by reporting only those whose centroid lies in this tile's core.
            let a_shapes: Vec<Vec<Vec<[f64; 2]>>> = a_polys
                .iter()
                .filter(|p| clipped_area_dbu(p, cx0, cy0, cx1, cy1) > 0.5)
                .map(merged_to_shape)
                .collect();
            if a_shapes.is_empty() {
                return out.into_iter();
            }
            let gaps = match covered_tiles.get(&(tx, ty)) {
                Some(idxs) => {
                    let cov: Vec<Vec<Vec<[f64; 2]>>> =
                        idxs.iter().map(|&i| merged_to_shape(&covered_polys[i])).collect();
                    a_shapes.overlay(&cov, OverlayRule::Difference, FillRule::NonZero)
                }
                None => a_shapes.simplify_shape(FillRule::NonZero),
            };
            for g in shapes_to_merged(gaps) {
                let (mx, my) = merged_centroid_dbu(&g);
                if mx >= cx0 && mx < cx1 && my >= cy0 && my < cy1 && clipped_area_dbu(&g, cx0, cy0, cx1, cy1) > 0.5 {
                    out.push((mx, my));
                }
            }
            out.into_iter()
        })
        .collect()
}

/// Operator for a tiled (lazy) virtual layer.  Most are boolean ops between source
/// layers; `Square`/`NotSquare` are unary *shape filters* that keep regions of the
/// single source whose outline is / isn't an axis-aligned square (e.g. splitting Cont
/// into square contacts vs. `ContBar`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VirtualOp {
    Union,
    Intersection,
    Difference,
    Square,
    NotSquare,
    /// Morphological close by `radius` DBU (size-merge); single source.
    Close(i32),
    /// Morphological open by `radius` DBU (erode-then-dilate); single source.  Removes
    /// every part of a region narrower than `2·radius` — a region that vanishes entirely
    /// has *no* position wider than that, which implements "min. X at one position" rules
    /// (e.g. pSD.e) via a subsequent `not_interacting` against the opened layer.
    Open(i32),
    /// Morphological grow (one-directional dilate) by `radius` DBU; single source.
    Grow(i32),
    /// Region-level selection: keep whole regions of source[0] that touch source[1]
    /// (KLayout `interacting` / `not_outside`).
    Interacting,
    /// Keep whole regions of source[0] that touch *no* region of source[1]
    /// (KLayout `ext_interacting(..., inverted: true)`).
    NotInteracting,
    /// Keep whole regions of source[0] that fully contain a region of source[1]
    /// (KLayout `ext_covering`).  Our uses always have source[1] ⊆ source[0], so
    /// "covers" coincides with "interacts"; computed the same way.
    Covering,
    /// Unary shape filter: keep regions that are neither a circle nor a regular octagon
    /// (KLayout `.not(get_circle).not(get_octagon)`, e.g. Padb.f's disallowed shapes).
    NotCircleOrOctagon,
    /// Unary shape filter: keep regions that are not a circle (KLayout `.not(get_circle)`,
    /// e.g. Padc.f's disallowed shapes).
    NotCircle,
    /// Unary: the *hole* areas of each source region, as filled polygons (KLayout
    /// `.holes` — e.g. the interior of a pSD substrate-tie ring).  Only valid for
    /// device-scale rings that assemble whole within one tile bucket; a chip-perimeter
    /// ring's hole is not tile-local (the `with_holes` limitation).
    Holes,
    /// Unary shape filter: keep source regions that contain at least one hole (KLayout
    /// `.with_holes` — e.g. the ring-shaped NWell encircling an iso-PWell).  Same
    /// tile-locality limitation as [`VirtualOp::Holes`]: the ring must assemble whole
    /// within one tile bucket (declare the max ring extent via the def's `radius`).
    WithHoles,
    /// Region-level selection: keep whole regions of source[0] containing a text label
    /// on source[1] (a text layer) matching the def's pattern (KLayout
    /// `ext_interacting_with_text`).  Routed specially in `ensure` (needs the layout's
    /// texts, not polygon tiles).
    WithText,
}

impl VirtualOp {
    /// Region-level selectors are evaluated on stitched whole regions in
    /// [`MergedCache::ensure`], not composed per tile like the boolean ops.
    fn is_selection(self) -> bool {
        matches!(self, VirtualOp::Interacting | VirtualOp::NotInteracting | VirtualOp::Covering)
    }
}

/// True if a merged region is a filled axis-aligned square: no holes, and a rectangle
/// (area == bounding box) with equal width and height.  Coordinates are integer DBU,
/// so the test is exact (the 0.5 slack only absorbs the f64 area arithmetic).
fn is_square(m: &MergedPoly) -> bool {
    if !m.holes.is_empty() || m.outer.is_empty() {
        return false;
    }
    let (mut xmin, mut ymin) = (i32::MAX, i32::MAX);
    let (mut xmax, mut ymax) = (i32::MIN, i32::MIN);
    for p in &m.outer {
        xmin = xmin.min(p.x);
        ymin = ymin.min(p.y);
        xmax = xmax.max(p.x);
        ymax = ymax.max(p.y);
    }
    let (w, h) = ((xmax - xmin) as i64, (ymax - ymin) as i64);
    if w <= 0 || h <= 0 || w != h {
        return false;
    }
    // Filled rectangle ⇒ shoelace area (2×) equals 2·w·h.
    (ring_area2(&m.outer) - 2.0 * (w * h) as f64).abs() < 0.5
}

/// Bounding box of a region's outer contour, DBU.
fn outer_bbox(outer: &[IntPoint]) -> (i64, i64, i64, i64) {
    let (mut xmin, mut ymin) = (i32::MAX, i32::MAX);
    let (mut xmax, mut ymax) = (i32::MIN, i32::MIN);
    for p in outer {
        xmin = xmin.min(p.x);
        ymin = ymin.min(p.y);
        xmax = xmax.max(p.x);
        ymax = ymax.max(p.y);
    }
    (xmin as i64, ymin as i64, xmax as i64, ymax as i64)
}

/// True if a region approximates a circle inscribed in a square bounding box — mirrors
/// IHP's `get_circle` (Padb.f/Padc.f pad-opening shape check): no holes, square bbox,
/// ≥16 vertices (a faceted circle approximation), and a bbox-area/polygon-area ratio in
/// [1.270, 1.276] (≈ 4/π, the ratio for a true circle inscribed in its bounding square).
fn is_circle(m: &MergedPoly) -> bool {
    if !m.holes.is_empty() || m.outer.len() < 16 {
        return false;
    }
    let (x0, y0, x1, y1) = outer_bbox(&m.outer);
    let (w, h) = (x1 - x0, y1 - y0);
    if w <= 0 || w != h {
        return false;
    }
    let area = ring_area2(&m.outer) / 2.0;
    if area <= 0.0 {
        return false;
    }
    let ratio = (w * h) as f64 / area;
    (1.270..=1.276).contains(&ratio)
}

/// True if a region is a regular octagon inscribed in a square bounding box — mirrors
/// IHP's `get_octagon`: no holes, square bbox, exactly 8 vertices, and its 8 edges split
/// into exactly 2 horizontal + 2 vertical + 4 diagonal (~45°, chamfered-corner) edges.
fn is_octagon(m: &MergedPoly) -> bool {
    if !m.holes.is_empty() || m.outer.len() != 8 {
        return false;
    }
    let (x0, y0, x1, y1) = outer_bbox(&m.outer);
    if x1 - x0 <= 0 || x1 - x0 != y1 - y0 {
        return false;
    }
    let n = m.outer.len();
    let (mut horiz, mut vert, mut diag) = (0, 0, 0);
    for i in 0..n {
        let a = m.outer[i];
        let b = m.outer[if i + 1 == n { 0 } else { i + 1 }];
        let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
        if dy == 0.0 && dx != 0.0 {
            horiz += 1;
        } else if dx == 0.0 && dy != 0.0 {
            vert += 1;
        } else if dx != 0.0 && dy != 0.0 && (dx.abs() / dy.abs() - 1.0).abs() < 0.02 {
            // within ±~1° of 45°, matching KLayout's [44.5°, 45.5°] tolerance
            diag += 1;
        } else {
            return false;
        }
    }
    horiz == 2 && vert == 2 && diag == 4
}

/// A lazy virtual layer evaluated per tile from its source layers' tiles, rather
/// than materialised globally up front.  `sources[0]` is the base for `Difference`.
#[derive(Clone)]
pub struct TiledVirtual {
    pub op: VirtualOp,
    pub sources: Vec<(i16, i16)>,
    /// Text pattern for [`VirtualOp::WithText`] (exact, or prefix if it ends in `*`).
    pub text: Option<String>,
}

/// One merged region as an `i_overlay` shape (outer contour first, then holes).
fn merged_to_shape(m: &MergedPoly) -> Vec<Vec<[f64; 2]>> {
    let mut s = Vec::with_capacity(1 + m.holes.len());
    s.push(m.outer.iter().map(|p| [p.x as f64, p.y as f64]).collect());
    for h in &m.holes {
        s.push(h.iter().map(|p| [p.x as f64, p.y as f64]).collect());
    }
    s
}

fn tile_shapes(polys: &[MergedPoly]) -> Vec<Vec<Vec<[f64; 2]>>> {
    polys.iter().map(merged_to_shape).collect()
}

/// Apply a boolean op to one tile's already-merged source geometry.  Pointwise, so
/// computing it per tile (on the haloed source polys) and reading the core later is
/// exact.  Holes are preserved.
pub fn compose_tile(op: VirtualOp, sources: &[&[MergedPoly]]) -> Vec<MergedPoly> {
    match op {
        VirtualOp::Union => {
            let shapes: Vec<Vec<Vec<[f64; 2]>>> =
                sources.iter().flat_map(|s| tile_shapes(s)).collect();
            if shapes.is_empty() {
                return Vec::new();
            }
            shapes_to_merged(shapes.simplify_shape(FillRule::NonZero))
        }
        VirtualOp::Intersection => {
            let mut acc = tile_shapes(sources[0]).simplify_shape(FillRule::NonZero);
            for s in &sources[1..] {
                if acc.is_empty() {
                    return Vec::new();
                }
                let clip = tile_shapes(s).simplify_shape(FillRule::NonZero);
                acc = acc.overlay(&clip, OverlayRule::Intersect, FillRule::NonZero);
            }
            shapes_to_merged(acc)
        }
        VirtualOp::Difference => {
            let mut acc = tile_shapes(sources[0]).simplify_shape(FillRule::NonZero);
            for s in &sources[1..] {
                if acc.is_empty() {
                    return Vec::new();
                }
                let clip = tile_shapes(s).simplify_shape(FillRule::NonZero);
                if clip.is_empty() {
                    continue;
                }
                acc = acc.overlay(&clip, OverlayRule::Difference, FillRule::NonZero);
            }
            shapes_to_merged(acc)
        }
        // Unary shape filters: keep the source regions that are / aren't squares.
        VirtualOp::Square => sources[0].iter().filter(|m| is_square(m)).cloned().collect(),
        VirtualOp::NotSquare => sources[0].iter().filter(|m| !is_square(m)).cloned().collect(),
        VirtualOp::NotCircleOrOctagon => sources[0]
            .iter()
            .filter(|m| !is_circle(m) && !is_octagon(m))
            .cloned()
            .collect(),
        VirtualOp::NotCircle => sources[0].iter().filter(|m| !is_circle(m)).cloned().collect(),
        // The hole areas of each region, as filled polygons.  Hole contours are stored
        // clockwise (see MergedPoly); reverse to CCW so downstream ops see solid regions.
        VirtualOp::Holes => sources[0]
            .iter()
            .flat_map(|m| {
                m.holes.iter().map(|h| {
                    let mut outer = h.clone();
                    outer.reverse();
                    MergedPoly { outer, holes: Vec::new() }
                })
            })
            .collect(),
        VirtualOp::WithHoles => {
            sources[0].iter().filter(|m| !m.holes.is_empty()).cloned().collect()
        }
        // Text selection is routed to `build_text_selection_tiles` in `ensure`.
        VirtualOp::WithText => Vec::new(),
        // Morphological close of the single source by `r` DBU.  The source tile carries a
        // halo ≥ 2·r (set in run_drc) so dilate-then-erode is exact in the core.
        VirtualOp::Close(r) => closing(sources[0], r as f64),
        VirtualOp::Open(r) => opening(sources[0], r as f64),
        VirtualOp::Grow(r) => grow(sources[0], r as f64),
        // Region selectors are not composable per tile — `ensure` routes them to
        // `build_selection_tiles` before this point.
        VirtualOp::Interacting | VirtualOp::NotInteracting | VirtualOp::Covering => Vec::new(),
    }
}

/// Build a lazy virtual layer's tiles from its (already tiled) source layers — one
/// boolean op per tile, run across tiles in parallel.
fn build_virtual_tiles(op: VirtualOp, sources: &[&TileMap]) -> TileMap {
    let Some((first, rest)) = sources.split_first() else {
        return TileMap::new();
    };
    // Which tile keys can produce output: union of all keys, except Difference is
    // bounded by the base and Intersection by the keys common to every source.
    let keys: HashSet<(i32, i32)> = match op {
        VirtualOp::Difference => first.keys().copied().collect(),
        VirtualOp::Intersection => {
            let mut acc: HashSet<(i32, i32)> = first.keys().copied().collect();
            for m in rest {
                acc.retain(|k| m.contains_key(k));
            }
            acc
        }
        VirtualOp::Union => sources.iter().flat_map(|m| m.keys().copied()).collect(),
        // Unary ops (filters, close) output only where the single source has geometry; the
        // source's halo already covers the tiles a close can bridge into.
        VirtualOp::Square
        | VirtualOp::NotSquare
        | VirtualOp::Close(_)
        | VirtualOp::Open(_)
        | VirtualOp::Grow(_)
        | VirtualOp::NotCircleOrOctagon
        | VirtualOp::NotCircle
        | VirtualOp::Holes
        | VirtualOp::WithHoles => first.keys().copied().collect(),
        // Selection ops never reach here (handled in `ensure`).
        VirtualOp::Interacting
        | VirtualOp::NotInteracting
        | VirtualOp::Covering
        | VirtualOp::WithText => first.keys().copied().collect(),
    };

    keys.into_par_iter()
        .filter_map(|key| {
            let per_src: Vec<&[MergedPoly]> = sources
                .iter()
                .map(|m| m.get(&key).map(Vec::as_slice).unwrap_or(&[]))
                .collect();
            let polys = compose_tile(op, &per_src);
            (!polys.is_empty()).then_some((key, polys))
        })
        .collect()
}

/// A tile's core rectangle in DBU.  Checks keep only violations whose location
/// lies in `[x0, x1) × [y0, y1)`, so a feature seen in several overlapping tiles
/// is counted exactly once.
#[derive(Clone, Copy)]
pub struct Core {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Core {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x0 as f64 && x < self.x1 as f64 && y >= self.y0 as f64 && y < self.y1 as f64
    }
}

/// Merged geometry of one layer, indexed by global tile `(tx, ty)`.
pub type TileMap = HashMap<(i32, i32), Vec<MergedPoly>>;

/// Shoelace area of a simple polygon (absolute value).
fn shoelace(p: &[(f64, f64)]) -> f64 {
    let n = p.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        a += p[i].0 * p[j].1 - p[j].0 * p[i].1;
    }
    (a / 2.0).abs()
}

/// One Sutherland–Hodgman pass: keep the part of `poly` inside a half-plane.
/// `isect` is only called for crossing edges, so its denominator is never zero.
fn clip_halfplane(
    poly: &[(f64, f64)],
    keep: impl Fn(f64, f64) -> bool,
    isect: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Vec<(f64, f64)> {
    let n = poly.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n + 4);
    for i in 0..n {
        let cur = poly[i];
        let prev = poly[if i == 0 { n - 1 } else { i - 1 }];
        let cin = keep(cur.0, cur.1);
        let pin = keep(prev.0, prev.1);
        if cin {
            if !pin {
                out.push(isect(prev, cur));
            }
            out.push(cur);
        } else if pin {
            out.push(isect(prev, cur));
        }
    }
    out
}

/// Clip a DBU contour to the axis-aligned rect `[x0,x1] × [y0,y1]`.
fn clip_rect(pts: &[IntPoint], x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    let mut p: Vec<(f64, f64)> = pts.iter().map(|q| (q.x as f64, q.y as f64)).collect();
    p = clip_halfplane(&p, |x, _| x >= x0, |a, b| {
        let t = (x0 - a.0) / (b.0 - a.0);
        (x0, a.1 + t * (b.1 - a.1))
    });
    if p.is_empty() { return p; }
    p = clip_halfplane(&p, |x, _| x <= x1, |a, b| {
        let t = (x1 - a.0) / (b.0 - a.0);
        (x1, a.1 + t * (b.1 - a.1))
    });
    if p.is_empty() { return p; }
    p = clip_halfplane(&p, |_, y| y >= y0, |a, b| {
        let t = (y0 - a.1) / (b.1 - a.1);
        (a.0 + t * (b.0 - a.0), y0)
    });
    if p.is_empty() { return p; }
    clip_halfplane(&p, |_, y| y <= y1, |a, b| {
        let t = (y1 - a.1) / (b.1 - a.1);
        (a.0 + t * (b.0 - a.0), y1)
    })
}

/// Area (DBU²) of a merged region clipped to the rect `[x0,x1] × [y0,y1]`.
/// Used by density checks to sum coverage within a tile or window without
/// double-counting (holes subtracted, regions clipped to disjoint rects).
pub fn clipped_area_dbu(m: &MergedPoly, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let mut a = shoelace(&clip_rect(&m.outer, x0, y0, x1, y1));
    for h in &m.holes {
        a -= shoelace(&clip_rect(h, x0, y0, x1, y1));
    }
    a.max(0.0)
}

/// Twice the absolute area of an integer ring (shoelace), in DBU².
fn ring_area2(c: &[IntPoint]) -> f64 {
    let n = c.len();
    let mut a = 0.0_f64;
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        a += c[i].x as f64 * c[j].y as f64 - c[j].x as f64 * c[i].y as f64;
    }
    a.abs()
}

/// Area (DBU²) of a whole merged region: outer contour minus its holes.
pub fn merged_area_dbu(m: &MergedPoly) -> f64 {
    let mut a2 = ring_area2(&m.outer);
    for h in &m.holes {
        a2 -= ring_area2(h);
    }
    (a2 / 2.0).max(0.0)
}

/// Centroid (DBU) of a merged region's outer ring (vertex average) — a marker
/// point for per-region violations.
pub fn merged_centroid_dbu(m: &MergedPoly) -> (f64, f64) {
    let n = m.outer.len().max(1) as f64;
    let (sx, sy) = m.outer.iter().fold((0.0, 0.0), |(sx, sy), p| (sx + p.x as f64, sy + p.y as f64));
    (sx / n, sy / n)
}

/// A connected region of a whole layer: total area (DBU²) and a marker point (DBU).
#[derive(Clone, Copy)]
pub struct Region {
    pub area_dbu: f64,
    pub marker: (f64, f64),
}

/// Metal `y`-intervals where the region covers the vertical line `x = xs`,
/// clipped to `[ylo, yhi]`.  Even-odd over horizontal-edge crossings (`xs` is
/// sampled half-a-DBU off the grid, so it never lies on a vertical edge).
fn coverage_y(m: &MergedPoly, xs: f64, ylo: f64, yhi: f64) -> Vec<(f64, f64)> {
    // Scanline crossings of the vertical line x = xs: every edge that straddles xs
    // contributes its y-intercept (interpolated, so 45°/diagonal edges count too — a
    // chamfered corner astride a tile boundary would otherwise be missed and split the
    // region).  Half-open straddle test (`<= xs` xor) avoids double-counting vertices.
    let mut ys: Vec<f64> = Vec::new();
    let mut scan = |c: &[IntPoint]| {
        let n = c.len();
        for i in 0..n {
            let a = c[i];
            let b = c[if i + 1 == n { 0 } else { i + 1 }];
            let (ax, bx) = (a.x as f64, b.x as f64);
            if (ax <= xs) != (bx <= xs) {
                let t = (xs - ax) / (bx - ax);
                ys.push(a.y as f64 + (b.y as f64 - a.y as f64) * t);
            }
        }
    };
    scan(&m.outer);
    for h in &m.holes { scan(h); }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < ys.len() {
        let (lo, hi) = (ys[i].max(ylo), ys[i + 1].min(yhi));
        if hi > lo { out.push((lo, hi)); }
        i += 2;
    }
    out
}

/// Metal `x`-intervals where the region covers the horizontal line `y = ys`.
fn coverage_x(m: &MergedPoly, ys: f64, xlo: f64, xhi: f64) -> Vec<(f64, f64)> {
    // See [`coverage_y`]: scanline crossings of the horizontal line y = ys, interpolating
    // x for every straddling edge (diagonals included).
    let mut xs: Vec<f64> = Vec::new();
    let mut scan = |c: &[IntPoint]| {
        let n = c.len();
        for i in 0..n {
            let a = c[i];
            let b = c[if i + 1 == n { 0 } else { i + 1 }];
            let (ay, by) = (a.y as f64, b.y as f64);
            if (ay <= ys) != (by <= ys) {
                let t = (ys - ay) / (by - ay);
                xs.push(a.x as f64 + (b.x as f64 - a.x as f64) * t);
            }
        }
    };
    scan(&m.outer);
    for h in &m.holes { scan(h); }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < xs.len() {
        let (lo, hi) = (xs[i].max(xlo), xs[i + 1].min(xhi));
        if hi > lo { out.push((lo, hi)); }
        i += 2;
    }
    out
}

fn intervals_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.iter().any(|&(a0, a1)| b.iter().any(|&(b0, b1)| a0 < b1 && b0 < a1))
}

/// Minimal union-find with path compression (shared by the region stitching here,
/// net extraction and the array-space clustering).
pub(crate) struct UnionFind { parent: Vec<usize> }
impl UnionFind {
    pub(crate) fn new(n: usize) -> Self { Self { parent: (0..n).collect() } }
    pub(crate) fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r { r = self.parent[r]; }
        let mut c = x;
        while self.parent[c] != r {
            let next = self.parent[c];
            self.parent[c] = r;
            c = next;
        }
        r
    }
    pub(crate) fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb { self.parent[ra] = rb; }
    }
}

/// Metal coverage just inside each of a tile core's four edges — the interface a
/// piece exposes for cross-tile stitching (pieces in adjacent tiles whose facing
/// intervals overlap belong to one connected region).
struct Sides {
    right: Vec<(f64, f64)>,
    left: Vec<(f64, f64)>,
    top: Vec<(f64, f64)>,
    bottom: Vec<(f64, f64)>,
}

impl Sides {
    fn of(poly: &MergedPoly, cx0: f64, cy0: f64, cx1: f64, cy1: f64) -> Self {
        Sides {
            right: coverage_y(poly, cx1 - 0.5, cy0, cy1),
            left: coverage_y(poly, cx0 + 0.5, cy0, cy1),
            top: coverage_x(poly, cy1 - 0.5, cx0, cx1),
            bottom: coverage_x(poly, cy0 + 0.5, cx0, cx1),
        }
    }
}

/// One tile piece: its core-clipped area, a marker, and its stitching [`Sides`].
struct Piece {
    area: f64,
    marker: (f64, f64),
    sides: Sides,
}

impl Piece {
    /// A piece for `poly` in the tile core `[cx0,cx1) × [cy0,cy1)`, or `None` if the
    /// region only reaches this tile's halo, not its core.
    fn of(poly: &MergedPoly, cx0: f64, cy0: f64, cx1: f64, cy1: f64) -> Option<Piece> {
        let area = clipped_area_dbu(poly, cx0, cy0, cx1, cy1);
        (area > 0.0).then(|| Piece {
            area,
            marker: merged_centroid_dbu(poly),
            sides: Sides::of(poly, cx0, cy0, cx1, cy1),
        })
    }
}

/// Union-find pieces that are continuous across shared tile-core edges (their
/// right↔left / top↔bottom coverage intervals overlap).  Shared by both stitchers
/// and [`analyze_regions`].
fn link_adjacent_pieces(
    uf: &mut UnionFind,
    tile_pieces: &HashMap<(i32, i32), Vec<usize>>,
    sides: &[&Sides],
) {
    for (&(tx, ty), ids) in tile_pieces {
        if let Some(neigh) = tile_pieces.get(&(tx + 1, ty)) {
            for &a in ids {
                for &b in neigh {
                    if intervals_overlap(&sides[a].right, &sides[b].left) {
                        uf.union(a, b);
                    }
                }
            }
        }
        if let Some(neigh) = tile_pieces.get(&(tx, ty + 1)) {
            for &a in ids {
                for &b in neigh {
                    if intervals_overlap(&sides[a].top, &sides[b].bottom) {
                        uf.union(a, b);
                    }
                }
            }
        }
    }
}

/// Stage 2: reconstruct whole-layer connected regions from the per-tile merge.
///
/// Each tile contributes one piece per merged polygon (area clipped to the tile
/// core, so cores tile the plane and a region spanning tiles isn't double-counted).
/// Pieces in adjacent tiles whose metal is continuous across the shared core edge
/// (their edge-coverage intervals overlap) are union-found into one region; the
/// region's area is the sum of its pieces' areas.  Only border pieces enter the
/// union-find, so this stays cheap even when the per-tile merge is dense.
pub fn stitch_regions(tiles: &TileMap, tile_dbu: i32) -> Vec<Region> {
    stitch_impl(tiles, tile_dbu, false).regions
}

/// Connected regions of a layer *with a point-lookup index*: like [`stitch_regions`]
/// but each region keeps an id (its index in `regions`) and every core piece's polygon
/// is recorded under its tile, tagged with its region id.  The connectivity engine uses
/// this to resolve "which region of layer L contains point p" when a via/contact bridges
/// two layers.
pub struct LabeledRegions {
    pub regions: Vec<Region>,
    /// Per tile: the merged polygons whose core lies in that tile, each with its region id.
    pub by_tile: HashMap<(i32, i32), Vec<(MergedPoly, usize)>>,
}

pub fn stitch_labeled(tiles: &TileMap, tile_dbu: i32) -> LabeledRegions {
    stitch_impl(tiles, tile_dbu, true)
}

/// Shared stitcher behind [`stitch_regions`] and [`stitch_labeled`].  `record_polys`
/// selects whether each core piece's polygon is cloned into `by_tile` (the point-lookup
/// index) — skipped for plain region stitching so dense layers aren't copied.
fn stitch_impl(tiles: &TileMap, tile_dbu: i32, record_polys: bool) -> LabeledRegions {
    let t = tile_dbu as i64;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut piece_loc: Vec<((i32, i32), MergedPoly)> = Vec::new();
    let mut tile_pieces: HashMap<(i32, i32), Vec<usize>> = HashMap::new();

    for (&(tx, ty), polys) in tiles {
        let cx0 = (tx as i64 * t) as f64;
        let cy0 = (ty as i64 * t) as f64;
        let cx1 = ((tx as i64 + 1) * t) as f64;
        let cy1 = ((ty as i64 + 1) * t) as f64;
        for poly in polys {
            let Some(piece) = Piece::of(poly, cx0, cy0, cx1, cy1) else { continue };
            let id = pieces.len();
            pieces.push(piece);
            if record_polys {
                piece_loc.push(((tx, ty), poly.clone()));
            }
            tile_pieces.entry((tx, ty)).or_default().push(id);
        }
    }

    let mut uf = UnionFind::new(pieces.len());
    let sides: Vec<&Sides> = pieces.iter().map(|p| &p.sides).collect();
    link_adjacent_pieces(&mut uf, &tile_pieces, &sides);

    // Compact each union-find root to a dense region id and aggregate area/marker
    // (marker from the largest piece).
    let mut root_to_region: HashMap<usize, usize> = HashMap::new();
    let mut regions: Vec<Region> = Vec::new();
    let mut largest: Vec<f64> = Vec::new();
    for (id, p) in pieces.iter().enumerate() {
        let root = uf.find(id);
        let region = *root_to_region.entry(root).or_insert_with(|| {
            regions.push(Region { area_dbu: 0.0, marker: p.marker });
            largest.push(0.0);
            regions.len() - 1
        });
        regions[region].area_dbu += p.area;
        if p.area > largest[region] {
            largest[region] = p.area;
            regions[region].marker = p.marker;
        }
    }

    let mut by_tile: HashMap<(i32, i32), Vec<(MergedPoly, usize)>> = HashMap::new();
    for (id, (tile, poly)) in piece_loc.into_iter().enumerate() {
        let region = root_to_region[&uf.find(id)];
        by_tile.entry(tile).or_default().push((poly, region));
    }

    LabeledRegions { regions, by_tile }
}

/// Per connected region of a base layer: true filled area (DBU²), the enclosed `feature`
/// area (DBU²), bounding box, a marker, and whether the region is *wide* — i.e. it contains
/// a spot at least `2 * erode_radius` across in every direction.  Everything is derived
/// from the per-tile merge, so a dense layer is never globally unioned (which OOMs).
pub struct PlateInfo {
    pub metal_area: f64,
    pub feature_area: f64,
    pub marker: (f64, f64),
    /// A point on the wide spot itself (set when `is_wide`) — unlike `marker` (the whole
    /// region's centroid), this always lands on the flagged metal.
    pub wide_at: (f64, f64),
    pub bbox: (i32, i32, i32, i32),
    pub is_wide: bool,
}

/// Even-odd ray cast over a ring (point sampled as given; callers offset off-grid).
fn point_in_ring_i(px: f64, py: f64, ring: &[IntPoint]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (ring[i].x as f64, ring[i].y as f64);
        let (xj, yj) = (ring[j].x as f64, ring[j].y as f64);
        if (yi > py) != (yj > py) {
            let xc = (xj - xi) * (py - yi) / (yj - yi) + xi;
            if px < xc {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Axis-aligned bbox of a merged polygon (DBU).
fn poly_bbox(m: &MergedPoly) -> (i32, i32, i32, i32) {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in &m.outer {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x0, y0, x1, y1)
}

/// Whether two merged polygons share positive overlap area.
fn polys_overlap(a: &MergedPoly, b: &MergedPoly) -> bool {
    let (ax0, ay0, ax1, ay1) = poly_bbox(a);
    let (bx0, by0, bx1, by1) = poly_bbox(b);
    if ax1 < bx0 || bx1 < ax0 || ay1 < by0 || by1 < ay0 {
        return false;
    }
    let av = vec![merged_to_shape(a)];
    let bv = vec![merged_to_shape(b)];
    !av.overlay(&bv, OverlayRule::Intersect, FillRule::NonZero).is_empty()
}

/// Build a selection virtual's tiles: keep whole *regions* of the candidate layer
/// (`cand`) by whether they interact with the filter layer (`filt`), preserving the
/// candidate's original tiling.  Region membership is recovered with [`stitch_labeled`],
/// so a region spanning tiles is kept or dropped as a unit (a per-tile test would split
/// a region that only touches the filter in one of its tiles).  `keep` is true for
/// `Interacting`/`Covering`, false for `NotInteracting`.
fn build_selection_tiles(cand: &TileMap, filt: &TileMap, keep: bool, tile_dbu: i32) -> TileMap {
    // An empty filter means nothing interacts: `Interacting`/`Covering` keep nothing,
    // `NotInteracting` keeps everything.  Short-circuiting here avoids stitching a dense
    // candidate (e.g. `covering [GatPolyRes, Rsil]` on a chip with no resistors).
    if filt.values().all(|v| v.is_empty()) {
        return if keep { TileMap::new() } else { cand.clone() };
    }

    let labeled = stitch_labeled(cand, tile_dbu);
    let mut interacts = vec![false; labeled.regions.len()];

    // A candidate piece overlaps the filter only where they share a tile, so testing
    // against the same tile's filter polys is both sufficient and cheap.
    for (tile, polys) in &labeled.by_tile {
        let fpolys = filt.get(tile).map(Vec::as_slice).unwrap_or(&[]);
        for (poly, rid) in polys {
            if !interacts[*rid] && fpolys.iter().any(|fp| polys_overlap(poly, fp)) {
                interacts[*rid] = true;
            }
        }
    }

    let mut out: TileMap = HashMap::new();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            if interacts[rid] == keep {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

/// KLayout-style text pattern match: a trailing `*` makes it a case-insensitive
/// prefix; otherwise a case-insensitive exact match (mirrors `texts(pattern)`).
fn text_matches(s: &str, pattern: &str) -> bool {
    if let Some(stem) = pattern.strip_suffix('*') {
        s.len() >= stem.len() && s[..stem.len()].eq_ignore_ascii_case(stem)
    } else {
        s.eq_ignore_ascii_case(pattern)
    }
}

/// Build a `with_text` selection's tiles: keep whole *regions* of the candidate layer
/// containing any of the (already pattern-filtered) text points, preserving the
/// candidate's tiling — the region-level analogue of [`select_with_point`], stitched so
/// a labelled region spanning tiles is kept as a unit.
fn build_text_selection_tiles(cand: &TileMap, pts: &[(f64, f64)], tile_dbu: i32) -> TileMap {
    if pts.is_empty() {
        return TileMap::new();
    }
    let labeled = stitch_labeled(cand, tile_dbu);
    let mut hit = vec![false; labeled.regions.len()];
    for polys in labeled.by_tile.values() {
        for (poly, rid) in polys {
            if !hit[*rid] && pts.iter().any(|&(x, y)| point_in_merged(x, y, poly)) {
                hit[*rid] = true;
            }
        }
    }
    let mut out: TileMap = HashMap::new();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            if hit[rid] {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

/// Select whole polygons of `a` by whether they overlap any polygon of `b`
/// (`keep == true` keeps the interacting ones, `false` the non-interacting ones).
/// This is the region-mode `interacting` / `not_interacting` / `not_outside` selector.
pub fn select_interacting(a: &[MergedPoly], b: &[MergedPoly], keep: bool) -> Vec<MergedPoly> {
    a.iter()
        .filter(|ap| b.iter().any(|bp| polys_overlap(ap, bp)) == keep)
        .cloned()
        .collect()
}

/// Select whole polygons of `a` by whether they contain any of `points` (DBU).
pub fn select_with_point(a: &[MergedPoly], points: &[(f64, f64)], keep: bool) -> Vec<MergedPoly> {
    a.iter()
        .filter(|ap| points.iter().any(|&(x, y)| point_in_merged(x, y, ap)) == keep)
        .cloned()
        .collect()
}

pub fn point_in_merged(px: f64, py: f64, m: &MergedPoly) -> bool {
    point_in_ring_i(px, py, &m.outer) && !m.holes.iter().any(|h| point_in_ring_i(px, py, h))
}

/// A `radius` disk at `(cx, cy)` as a 24-gon (one shape), for an exact solidity test.
fn disk_poly(cx: f64, cy: f64, radius: f64) -> Vec<Vec<[f64; 2]>> {
    let n = 24;
    let ring: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            [cx + radius * a.cos(), cy + radius * a.sin()]
        })
        .collect();
    vec![ring]
}

struct PlatePiece {
    area: f64,
    feature: f64,
    wide: bool,
    wide_at: (f64, f64),
    marker: (f64, f64),
    bbox: (i32, i32, i32, i32),
    tile: (i32, i32),
    sides: Sides,
}

pub fn analyze_regions(
    metal: &TileMap,
    feature: &TileMap,
    erode_radius: f64,
    tile_dbu: i32,
) -> Vec<PlateInfo> {
    let t = tile_dbu as i64;
    let empty: Vec<MergedPoly> = Vec::new();
    // The erosion reach is supplied by reading neighbouring tiles (no wide halo needed):
    // `ring` tiles in every direction cover the core ± `erode_radius`.
    let ring = (erode_radius / tile_dbu as f64).ceil().max(1.0) as i32;

    // Stage 1 (parallel): one piece per metal poly with core area, carrying the enclosed
    // feature area and a wide flag.  The wide flag erodes the union of the tile's `ring`
    // neighbourhood (a few hundred polys) — bounded and never a global union.
    let mut pieces: Vec<PlatePiece> = metal
        .par_iter()
        .flat_map_iter(|(&(tx, ty), polys)| {
            let cx0 = (tx as i64 * t) as f64;
            let cy0 = (ty as i64 * t) as f64;
            let cx1 = ((tx as i64 + 1) * t) as f64;
            let cy1 = ((ty as i64 + 1) * t) as f64;
            let feats = feature.get(&(tx, ty)).unwrap_or(&empty);
            let mut local: Vec<PlatePiece> = Vec::new();
            let mut local_polys: Vec<&MergedPoly> = Vec::new();
            for poly in polys {
                let area = clipped_area_dbu(poly, cx0, cy0, cx1, cy1);
                if area <= 0.0 {
                    continue; // only reaches this tile's halo, not its core
                }
                let (mut bx0, mut by0, mut bx1, mut by1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                for p in &poly.outer {
                    bx0 = bx0.min(p.x);
                    by0 = by0.min(p.y);
                    bx1 = bx1.max(p.x);
                    by1 = by1.max(p.y);
                }
                // Features are enclosed in metal, so each one's centroid lands in exactly
                // one metal poly; attribute its core-clipped area there.
                let mut feature = 0.0;
                for f in feats {
                    let (fcx, fcy) = merged_centroid_dbu(f);
                    if point_in_merged(fcx, fcy, poly) {
                        feature += clipped_area_dbu(f, cx0, cy0, cx1, cy1);
                    }
                }
                local.push(PlatePiece {
                    area,
                    feature,
                    wide: false,
                    wide_at: (0.0, 0.0),
                    marker: merged_centroid_dbu(poly),
                    bbox: (bx0, by0, bx1, by1),
                    tile: (tx, ty),
                    sides: Sides::of(poly, cx0, cy0, cx1, cy1),
                });
                local_polys.push(poly);
            }
            if local.is_empty() {
                return local.into_iter();
            }
            // A ≥ 2*radius-wide plate fully covers a tile core, so a sparsely-filled tile
            // (thin routing) can't host a wide spot — skip its erosion.  This keeps the
            // neighbourhood union to the few plate-dense tiles, not every routing tile.
            let core_area = (t * t) as f64;
            let core_metal: f64 = local.iter().map(|p| p.area).sum();
            if core_metal < 0.5 * core_area {
                return local.into_iter();
            }
            // Erode the neighbourhood; any eroded metal left in this core marks the local
            // piece that contains it as wide (it sits in a ≥ 2*radius-wide plate).  Each
            // neighbour fragment is clipped to its own tile core first, so the per-tile
            // halo overlaps don't seam into erosion slivers.
            let mut neigh: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
            for dx in -ring..=ring {
                for dy in -ring..=ring {
                    let (nx, ny) = (tx + dx, ty + dy);
                    let Some(ps) = metal.get(&(nx, ny)) else { continue };
                    let bx0 = (nx as i64 * t) as f64;
                    let by0 = (ny as i64 * t) as f64;
                    let bx1 = ((nx as i64 + 1) * t) as f64;
                    let by1 = ((ny as i64 + 1) * t) as f64;
                    let core_box = vec![vec![[bx0, by0], [bx1, by0], [bx1, by1], [bx0, by1]]];
                    for p in ps {
                        neigh.extend(merged_to_shape(p).overlay(
                            &core_box,
                            OverlayRule::Intersect,
                            FillRule::NonZero,
                        ));
                    }
                }
            }
            let metal_shapes = neigh.simplify_shape(FillRule::NonZero);
            let eroded = metal_shapes.outline(&OutlineStyle::new(-erode_radius));
            for e in shapes_to_merged(eroded) {
                if clipped_area_dbu(&e, cx0, cy0, cx1, cy1) <= 0.5 {
                    continue;
                }
                let (ecx, ecy) = merged_centroid_dbu(&e);
                // The `outline` erosion can fill narrow slots/notches (closing a self-slotted
                // plate into a false wide spot).  Verify exactly against the slot-preserving
                // local metal: the radius disk must be fully covered (disk − metal empty).
                let leftover = disk_poly(ecx, ecy, erode_radius).overlay(
                    &metal_shapes,
                    OverlayRule::Difference,
                    FillRule::NonZero,
                );
                if !leftover.is_empty() {
                    continue;
                }
                if let Some(i) = local_polys.iter().position(|p| point_in_merged(ecx, ecy, p)) {
                    local[i].wide = true;
                    local[i].wide_at = (ecx, ecy);
                }
            }
            local.into_iter()
        })
        .collect();

    // Stage 2: union pieces that are continuous across shared tile-core edges.
    let mut tile_pieces: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (id, p) in pieces.iter().enumerate() {
        tile_pieces.entry(p.tile).or_default().push(id);
    }
    let mut uf = UnionFind::new(pieces.len());
    let sides: Vec<&Sides> = pieces.iter().map(|p| &p.sides).collect();
    link_adjacent_pieces(&mut uf, &tile_pieces, &sides);
    drop(sides);

    // Stage 3: aggregate per region.
    struct Acc {
        area: f64,
        feature: f64,
        marker: (f64, f64),
        largest: f64,
        bbox: (i32, i32, i32, i32),
        wide: bool,
        wide_at: (f64, f64),
    }
    let mut acc: HashMap<usize, Acc> = HashMap::new();
    for (id, p) in pieces.drain(..).enumerate() {
        let root = uf.find(id);
        let e = acc.entry(root).or_insert(Acc {
            area: 0.0,
            feature: 0.0,
            marker: p.marker,
            largest: 0.0,
            bbox: p.bbox,
            wide: false,
            wide_at: (0.0, 0.0),
        });
        e.area += p.area;
        e.feature += p.feature;
        if p.area > e.largest {
            e.largest = p.area;
            e.marker = p.marker;
        }
        e.bbox.0 = e.bbox.0.min(p.bbox.0);
        e.bbox.1 = e.bbox.1.min(p.bbox.1);
        e.bbox.2 = e.bbox.2.max(p.bbox.2);
        e.bbox.3 = e.bbox.3.max(p.bbox.3);
        if p.wide {
            e.wide = true;
            e.wide_at = p.wide_at;
        }
    }
    acc.into_values()
        .map(|a| PlateInfo {
            metal_area: a.area,
            feature_area: a.feature,
            marker: a.marker,
            wide_at: a.wide_at,
            bbox: a.bbox,
            is_wide: a.wide,
        })
        .collect()
}

fn bbox_of(b: &GdsBoundary) -> Option<(i32, i32, i32, i32)> {
    let mut x0 = i32::MAX;
    let mut y0 = i32::MAX;
    let mut x1 = i32::MIN;
    let mut y1 = i32::MIN;
    for p in &b.xy {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    if x0 == i32::MAX { None } else { Some((x0, y0, x1, y1)) }
}

/// Tiled merge of one layer on the global grid (origin (0,0), `tile_dbu`).
///
/// A boundary is placed, in full, into every tile whose core lies within
/// `halo_dbu` of its bounding box, then each tile's bucket is unioned
/// independently (in parallel).  This bounds peak memory — the global sweep that
/// would process the whole layer at once is replaced by many small sweeps — and
/// for a *local* rule (width/space/notch ≤ halo) each tile's geometry is
/// identical to a global merge within its core, because both walls of a thin
/// feature and any shape that would merge with near-core geometry lie in the halo.
fn build_tiled_merge(
    boundaries: &[GdsBoundary],
    tile_dbu: i32,
    halo_dbu: i32,
) -> TileMap {
    let tile = tile_dbu.max(1) as i64;
    let halo = halo_dbu.max(0) as i64;

    let mut buckets: HashMap<(i32, i32), Vec<&GdsBoundary>> = HashMap::new();
    for b in boundaries {
        let Some((x0, y0, x1, y1)) = bbox_of(b) else { continue };
        let tx0 = (x0 as i64 - halo).div_euclid(tile) as i32;
        let tx1 = (x1 as i64 + halo).div_euclid(tile) as i32;
        let ty0 = (y0 as i64 - halo).div_euclid(tile) as i32;
        let ty1 = (y1 as i64 + halo).div_euclid(tile) as i32;
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                buckets.entry((tx, ty)).or_default().push(b);
            }
        }
    }

    buckets
        .into_par_iter()
        .map(|(key, shapes)| (key, merge_refs(&shapes)))
        .collect()
}

/// Lazily-built, per-layer tiled merge shared across all checks in a run.
///
/// Every layer tiles on the same global grid, so tile `(tx, ty)` covers the same
/// region on every layer — which lets inter-layer checks (e.g. spacing between a
/// layer and its filler) line up tile-for-tile.  The first check that needs a
/// layer pays for its merge; the rest reuse it.
pub struct MergedCache {
    tile_dbu: i32,
    halo_dbu: i32,
    halo_by_layer: HashMap<(i16, i16), i32>,
    layers: HashMap<(i16, i16), TileMap>,
    regions: HashMap<(i16, i16), Vec<Region>>,
    /// Lazy virtual layers, keyed by their synthetic (layer, datatype); built on
    /// first `ensure` from their source layers' tiles instead of the layout.
    virtual_defs: HashMap<(i16, i16), TiledVirtual>,
}

impl MergedCache {
    /// `halo_dbu` is the default/minimum halo; `halo_by_layer` overrides it for
    /// layers that need a larger one (e.g. a coarse layer with a big `max_width`).
    /// Keeping the halo per layer stops one large rule from inflating the merge of
    /// every other (fine) layer in the deck.
    pub fn new(tile_dbu: i32, halo_dbu: i32, halo_by_layer: HashMap<(i16, i16), i32>) -> Self {
        Self {
            tile_dbu,
            halo_dbu,
            halo_by_layer,
            layers: HashMap::new(),
            regions: HashMap::new(),
            virtual_defs: HashMap::new(),
        }
    }

    /// Register a lazy virtual layer: a synthetic `key` built per tile by applying
    /// `op` to the tiles of `sources`.  Its tiles are computed on first `ensure`.
    pub fn register_virtual(
        &mut self,
        key: (i16, i16),
        op: VirtualOp,
        sources: Vec<(i16, i16)>,
        text: Option<String>,
    ) {
        self.virtual_defs.insert(key, TiledVirtual { op, sources, text });
    }

    /// Whole-layer connected regions (areas + markers), built by stitching the
    /// per-tile merge across tile borders.  Cached per layer.
    pub fn regions(&mut self, layout: &FlatLayout, layer: i16, datatype: i16) -> &[Region] {
        self.ensure(layout, layer, datatype);
        if !self.regions.contains_key(&(layer, datatype)) {
            let r = stitch_regions(&self.layers[&(layer, datatype)], self.tile_dbu);
            self.regions.insert((layer, datatype), r);
        }
        &self.regions[&(layer, datatype)]
    }

    /// Connected regions of `metal` with their enclosed `feature` area and a wide-spot
    /// flag (see [`analyze_regions`]).  Both layers are tiled (and `metal` must carry a
    /// halo ≥ `erode_radius`), so no dense layer is globally unioned.
    pub fn plate_regions(
        &mut self,
        layout: &FlatLayout,
        metal: (i16, i16),
        feature: (i16, i16),
        erode_radius: f64,
    ) -> Vec<PlateInfo> {
        self.ensure(layout, metal.0, metal.1);
        self.ensure(layout, feature.0, feature.1);
        analyze_regions(
            &self.layers[&metal],
            &self.layers[&feature],
            erode_radius,
            self.tile_dbu,
        )
    }

    /// Proximity-coverage gaps (see [`max_space_gaps`]): a point in each part of `a` that
    /// lies more than `value` from `b`.  Both layers are tiled; nothing is globally unioned.
    pub fn max_space_gaps(
        &mut self,
        layout: &FlatLayout,
        a: (i16, i16),
        b: (i16, i16),
        value: f64,
    ) -> Vec<(f64, f64)> {
        self.ensure(layout, a.0, a.1);
        self.ensure(layout, b.0, b.1);
        max_space_gaps(&self.layers[&a], &self.layers[&b], value, self.tile_dbu)
    }

    pub fn tile_dbu(&self) -> i32 {
        self.tile_dbu
    }

    /// Core rectangle of tile `(tx, ty)` on the global grid.
    pub fn core(&self, tx: i32, ty: i32) -> Core {
        let t = self.tile_dbu as i64;
        Core { x0: tx as i64 * t, y0: ty as i64 * t, x1: (tx as i64 + 1) * t, y1: (ty as i64 + 1) * t }
    }

    /// Merge `(layer, datatype)` and cache it if not already done.  A registered
    /// lazy virtual layer is composed per tile from its (recursively ensured)
    /// sources; any other layer is tiled directly from the layout.
    pub fn ensure(&mut self, layout: &FlatLayout, layer: i16, datatype: i16) {
        let key = (layer, datatype);
        if self.layers.contains_key(&key) {
            return;
        }
        if let Some(def) = self.virtual_defs.get(&key).cloned() {
            if matches!(def.op, VirtualOp::WithText) {
                // candidate = source[0]; source[1] is a TEXT layer, read from the
                // layout's labels rather than polygon tiles.
                self.ensure(layout, def.sources[0].0, def.sources[0].1);
                let pattern = def.text.as_deref().unwrap_or("");
                let pts: Vec<(f64, f64)> = def
                    .sources
                    .get(1)
                    .map(|&(tl, td)| {
                        layout
                            .texts(tl, td)
                            .iter()
                            .filter(|t| text_matches(&t.string, pattern))
                            .map(|t| (t.x as f64, t.y as f64))
                            .collect()
                    })
                    .unwrap_or_default();
                let cand = &self.layers[&def.sources[0]];
                let tiles = build_text_selection_tiles(cand, &pts, self.tile_dbu);
                self.layers.insert(key, tiles);
                return;
            }
            for &(sg, sd) in &def.sources {
                self.ensure(layout, sg, sd);
            }
            if def.op.is_selection() {
                // candidate = source[0], filter = source[1] (empty if absent).
                let empty = TileMap::new();
                let cand = &self.layers[&def.sources[0]];
                let filt = def.sources.get(1).map(|s| &self.layers[s]).unwrap_or(&empty);
                let keep = !matches!(def.op, VirtualOp::NotInteracting);
                let tiles = build_selection_tiles(cand, filt, keep, self.tile_dbu);
                self.layers.insert(key, tiles);
                return;
            }
            let src_maps: Vec<&TileMap> =
                def.sources.iter().map(|s| &self.layers[s]).collect();
            let tiles = build_virtual_tiles(def.op, &src_maps);
            self.layers.insert(key, tiles);
            return;
        }
        let tile = self.tile_dbu;
        let halo = self.halo_by_layer.get(&key).copied().unwrap_or(self.halo_dbu);
        let tiles = build_tiled_merge(layout.get(layer, datatype), tile, halo);
        self.layers.insert(key, tiles);
    }

    /// Drop a layer's cached tiles and stitched regions.  Used by the deck
    /// runner to free a layer's merged geometry once no remaining rule needs it,
    /// bounding peak memory when a deck touches many layers.
    pub fn evict(&mut self, layer: i16, datatype: i16) {
        self.layers.remove(&(layer, datatype));
        self.regions.remove(&(layer, datatype));
    }

    /// Per-tile merged geometry of a layer.  Must be `ensure`d first; a layer
    /// with no shapes yields an empty map.
    pub fn tiles(&self, layer: i16, datatype: i16) -> &TileMap {
        self.layers
            .get(&(layer, datatype))
            .expect("MergedCache::tiles called before ensure")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        MergedPoly {
            outer: vec![IntPoint::new(x0, y0), IntPoint::new(x1, y0), IntPoint::new(x1, y1), IntPoint::new(x0, y1)],
            holes: vec![],
        }
    }

    /// A 60 000 × 20 000 DBU region straddling the x = 50 000 tile border appears
    /// (full) in both tiles' caches; stitching must yield ONE region of the full
    /// area (1.2e9 DBU²), not two fragments.
    #[test]
    fn stitch_joins_region_across_tile_border() {
        let mut tiles: TileMap = HashMap::new();
        tiles.insert((0, 0), vec![rect(0, 0, 60_000, 20_000)]);
        tiles.insert((1, 0), vec![rect(0, 0, 60_000, 20_000)]);
        let regions = stitch_regions(&tiles, 50_000);
        assert_eq!(regions.len(), 1, "spanning region should be one");
        assert!((regions[0].area_dbu - 1.2e9).abs() < 1.0, "area = {}", regions[0].area_dbu);
    }

    /// Two regions separated by a gap across the border stay distinct.
    #[test]
    fn stitch_keeps_separate_regions_apart() {
        let mut tiles: TileMap = HashMap::new();
        // Region A entirely in tile 0; region B entirely in tile 1; 10 000 DBU gap.
        tiles.insert((0, 0), vec![rect(0, 0, 40_000, 20_000)]);
        tiles.insert((1, 0), vec![rect(60_000, 0, 90_000, 20_000)]);
        let regions = stitch_regions(&tiles, 50_000);
        assert_eq!(regions.len(), 2, "separate regions must stay apart");
    }

    /// L-shape: 100×100 bounding box (square box) but only three quarters filled.
    fn lshape() -> MergedPoly {
        MergedPoly {
            outer: vec![
                IntPoint::new(0, 0), IntPoint::new(100, 0), IntPoint::new(100, 50),
                IntPoint::new(50, 50), IntPoint::new(50, 100), IntPoint::new(0, 100),
            ],
            holes: vec![],
        }
    }

    #[test]
    fn is_square_classifies_shapes() {
        assert!(is_square(&rect(0, 0, 160, 160)), "equal-sided rectangle is a square");
        assert!(!is_square(&rect(0, 0, 160, 400)), "unequal-sided rectangle is a bar");
        assert!(!is_square(&lshape()), "L-shape with a square bbox is not a square");
        // A square outline with a hole is not a (filled) square.
        let mut holed = rect(0, 0, 200, 200);
        holed.holes.push(vec![
            IntPoint::new(50, 50), IntPoint::new(50, 150),
            IntPoint::new(150, 150), IntPoint::new(150, 50),
        ]);
        assert!(!is_square(&holed), "holed square is not a filled square");
    }

    #[test]
    fn square_filter_splits_squares_and_bars() {
        let src = vec![rect(0, 0, 160, 160), rect(500, 0, 660, 400), lshape()];
        let squares = compose_tile(VirtualOp::Square, &[&src]);
        let bars = compose_tile(VirtualOp::NotSquare, &[&src]);
        assert_eq!(squares.len(), 1, "one square contact");
        assert_eq!(bars.len(), 2, "the rectangle and the L-shape are bars");
    }
}
