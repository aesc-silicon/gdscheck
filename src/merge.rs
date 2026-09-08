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

use crate::geom::{
    ENCLOSURE_MAX_REACH, EnclosurePair, FacingRun, closest, enclosure_pairs, facing_runs,
    poly_from_merged, width_pairs,
};
use crate::layout::FlatLayout;
use gds21::GdsBoundary;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::simplify::SimplifyShape;
use i_overlay::float::single::SingleFloatOverlay;
// Re-exported: `MergedPoly`'s contours are made of these, so the type is already part of
// this module's surface - only its name was missing.
pub use i_overlay::i_float::int::point::IntPoint;
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
    // Unioned first, for the same reason as `shrink`.
    let whole = tile_shapes(polys).simplify_shape(FillRule::NonZero);
    let eroded = whole.outline(&OutlineStyle::new(-radius));
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
/// Grows by exactly `radius`.  A deck that wants slack beyond it asks with `slack:`, and
/// only a *selection radius* should: a shape at exactly the boundary distance then merely
/// touches the grown region, and i_overlay's `Intersect` — which `polys_overlap` and the
/// whole-region selectors are built on — reads a zero-area touch as no overlap, so a
/// compliant shape misclassifies as "too far".  A hair of slack turns the touch into a
/// hairline overlap.
///
/// It used to be the default, and that was wrong five times over: a band deciding which of
/// two limits applies, or a region a rule must not reach into, is judged wrong by any
/// slack at all, and each such rule over-reported plausibly rather than failing.  Eight
/// layers across both PDKs ask for it now, and the clearest is IHP's `PsdActivX1Touch` -
/// a grow of radius *zero*, which exists for the slack alone.
pub fn grow(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    grow_by(polys, radius, GROW_MARGIN as f64)
}

/// The slack a *selection radius* wants, in DBU.  Not a default any more - a deck asks
/// for it with `slack:` - and 5 rather than the usual half-DBU because `outline`'s offset
/// on a 45°-chamfered corner applied a sub-nanometre epsilon inconsistently between the
/// two facing edges; 5 nm clears it on both and is still far below any real feature.
pub const GROW_MARGIN: i32 = 5;

/// [`grow`] with the slack chosen explicitly.
pub fn grow_by(polys: &[MergedPoly], radius: f64, margin: f64) -> Vec<MergedPoly> {
    if polys.is_empty() {
        return Vec::new();
    }
    shapes_to_merged(tile_shapes(polys).outline(&OutlineStyle::new(radius + margin)))
}

/// Morphological erode (shrink) by `radius` DBU — the inverse of [`grow`], and KLayout's
/// `sized(-r)`.  Unlike [`opening`] there is no dilate back: a region narrower than
/// `2 * radius` disappears, and one that survives keeps its eroded outline, which is what
/// a rule comparing against "the well minus 0.43 um" wants.
pub fn shrink(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    if polys.is_empty() || radius <= 0.0 {
        return polys.to_vec();
    }
    // Unioned first: the outline erodes each input shape on its own, and two pieces of
    // one region that abut along a cut would each lose a strip along it.  Merged tile
    // geometry never abuts, but a layer delivered as core-clipped pieces does.
    let whole = tile_shapes(polys).simplify_shape(FillRule::NonZero);
    shapes_to_merged(whole.outline(&OutlineStyle::new(-radius)))
}

/// Directional dilate: the Minkowski sum of `polys` with the segment from `-d` to `+d`,
/// where `d` is `(radius, 0)` for the x axis and `(0, radius)` for the y axis (KLayout
/// `sized(r, 0)` / `sized(0, r)`).  [`grow`] offsets every edge outward by `radius` in
/// all directions; this one leaves the perpendicular extent exactly as drawn, which is
/// what the wide-metal rules (`M#.2b`) need.
///
/// Exact by construction: a point is in `P ⊕ seg` iff some translate of it along the
/// segment lands in `P`.  The set of such offsets is a union of closed intervals, so it
/// either reaches an endpoint of the segment (covered by the two translated copies) or
/// has an interior endpoint sitting on `∂P` (covered by sweeping that edge).  Sweeping
/// every contour edge — holes included, since dilating a region also shrinks its holes —
/// therefore closes the union.
fn size_directional(polys: &[MergedPoly], radius: f64, along_x: bool) -> Vec<MergedPoly> {
    if polys.is_empty() || radius <= 0.0 {
        return polys.to_vec();
    }
    let (dx, dy) = if along_x {
        (radius, 0.0)
    } else {
        (0.0, radius)
    };
    let mut shapes: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();

    for m in polys {
        // The two endpoint translates, holes preserved.
        for sign in [-1.0, 1.0] {
            let mut s = merged_to_shape(m);
            for ring in &mut s {
                for p in ring.iter_mut() {
                    p[0] += sign * dx;
                    p[1] += sign * dy;
                }
            }
            shapes.push(s);
        }
        // The swept band of every contour edge, as a parallelogram wound CCW so the
        // NonZero union adds it rather than cancelling against a clockwise hole ring.
        for (a, b) in poly_edges(m) {
            let (ax, ay) = (a.x as f64, a.y as f64);
            let (bx, by) = (b.x as f64, b.y as f64);
            let mut quad = vec![
                [ax - dx, ay - dy],
                [bx - dx, by - dy],
                [bx + dx, by + dy],
                [ax + dx, ay + dy],
            ];
            let area2: f64 = (0..4)
                .map(|i| {
                    let (p, q) = (quad[i], quad[(i + 1) % 4]);
                    p[0] * q[1] - q[0] * p[1]
                })
                .sum();
            if area2.abs() < 0.5 {
                continue; // edge parallel to the sweep: nothing swept
            }
            if area2 < 0.0 {
                quad.reverse();
            }
            shapes.push(vec![quad]);
        }
    }
    shapes_to_merged(shapes.simplify_shape(FillRule::NonZero))
}

/// Directional dilate along x by `radius` DBU.  See [`size_directional`].
pub fn grow_x(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    size_directional(polys, radius, true)
}

/// Directional dilate along y by `radius` DBU.  See [`size_directional`].
pub fn grow_y(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    size_directional(polys, radius, false)
}

/// Directional erode: the dual of [`size_directional`] — dilate the complement, then
/// complement back.  The complement is taken inside a frame comfortably larger than the
/// geometry's bounding box (`2·radius` plus a margin), so the frame's own edges can never
/// dilate into the result and every real edge is eroded exactly.
fn erode_directional(polys: &[MergedPoly], radius: f64, along_x: bool) -> Vec<MergedPoly> {
    if polys.is_empty() || radius <= 0.0 {
        return polys.to_vec();
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for m in polys {
        let (a, b, c, d) = poly_bbox(m);
        x0 = x0.min(a);
        y0 = y0.min(b);
        x1 = x1.max(c);
        y1 = y1.max(d);
    }
    let pad = 2.0 * radius + 10.0;
    let (fx0, fy0) = (x0 as f64 - pad, y0 as f64 - pad);
    let (fx1, fy1) = (x1 as f64 + pad, y1 as f64 + pad);
    let frame: Vec<Vec<Vec<[f64; 2]>>> =
        vec![vec![vec![[fx0, fy0], [fx1, fy0], [fx1, fy1], [fx0, fy1]]]];

    let solid = tile_shapes(polys).simplify_shape(FillRule::NonZero);
    let hole = frame.overlay(&solid, OverlayRule::Difference, FillRule::NonZero);
    let grown = size_directional(&shapes_to_merged(hole), radius, along_x);
    let grown_shapes = tile_shapes(&grown).simplify_shape(FillRule::NonZero);
    shapes_to_merged(frame.overlay(&grown_shapes, OverlayRule::Difference, FillRule::NonZero))
}

/// Directional erode along x by `radius` DBU (KLayout `sized(-r, 0)`).
pub fn shrink_x(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    erode_directional(polys, radius, true)
}

/// Directional erode along y by `radius` DBU (KLayout `sized(0, -r)`).
pub fn shrink_y(polys: &[MergedPoly], radius: f64) -> Vec<MergedPoly> {
    erode_directional(polys, radius, false)
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
                    let a_core = merged_to_shape(p).overlay(
                        &core_box,
                        OverlayRule::Intersect,
                        FillRule::NonZero,
                    );
                    if a_core.is_empty() {
                        continue;
                    }
                    let (mut qx0, mut qy0, mut qx1, mut qy1) =
                        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
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
                    let grown: Vec<Vec<Vec<[f64; 2]>>> = (((qy0 / t as f64).floor() as i32)
                        ..=((qy1 / t as f64).floor() as i32))
                        .flat_map(|qy| {
                            (((qx0 / t as f64).floor() as i32)..=((qx1 / t as f64).floor() as i32))
                                .map(move |qx| (qx, qy))
                        })
                        .filter_map(|(qx, qy)| b.get(&(qx, qy)))
                        .flat_map(|ps| ps.iter())
                        .filter_map(|bp| {
                            let (bx0, by0, bx1, by1) = bp.outer.iter().fold(
                                (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
                                |(x0, y0, x1, y1), p| {
                                    (
                                        x0.min(p.x as f64),
                                        y0.min(p.y as f64),
                                        x1.max(p.x as f64),
                                        y1.max(p.y as f64),
                                    )
                                },
                            );
                            if bx1 >= qx0 && bx0 <= qx1 && by1 >= qy0 && by0 <= qy1 {
                                let (gx0, gy0, gx1, gy1) =
                                    (bx0 - value, by0 - value, bx1 + value, by1 + value);
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
                        remaining =
                            remaining.overlay(&cov, OverlayRule::Difference, FillRule::NonZero);
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
    let all_b: Vec<Vec<Vec<[f64; 2]>>> = b
        .values()
        .flat_map(|ps| ps.iter().map(merged_to_shape))
        .collect();
    let covered_polys: Vec<MergedPoly> = if all_b.is_empty() {
        Vec::new()
    } else {
        shapes_to_merged(
            all_b
                .simplify_shape(FillRule::NonZero)
                .outline(&OutlineStyle::new(value)),
        )
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
                covered_tiles
                    .entry((tx as i32, ty as i32))
                    .or_default()
                    .push(i);
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
                    let cov: Vec<Vec<Vec<[f64; 2]>>> = idxs
                        .iter()
                        .map(|&i| merged_to_shape(&covered_polys[i]))
                        .collect();
                    a_shapes.overlay(&cov, OverlayRule::Difference, FillRule::NonZero)
                }
                None => a_shapes.simplify_shape(FillRule::NonZero),
            };
            for g in shapes_to_merged(gaps) {
                let (mx, my) = merged_centroid_dbu(&g);
                if mx >= cx0
                    && mx < cx1
                    && my >= cy0
                    && my < cy1
                    && clipped_area_dbu(&g, cx0, cy0, cx1, cy1) > 0.5
                {
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
    /// Where the second layer is enclosed by the first by less than `radius` DBU, as the
    /// quadrangles spanning the short margin (KLayout `inner.enclosed(outer, v).polygons`).
    /// Sources are `[outer, inner]`, the order `min_enclosure` names them in.
    EnclosureBelow(i32),
    /// The other side of [`EnclosureBelow`]: where the margin *exceeds* `radius` DBU.  A
    /// maximum has to know the real margin before it can call it too large, so each inner
    /// wall is measured to its nearest outer one and only then compared - the same reading
    /// `max_enclosure` takes, and the reason this cannot be the complement of the minimum.
    ///
    /// The search stops at [`ENCLOSURE_MAX_REACH`] times the bound: a wall that far off is
    /// not really the one opposite, and a margin swept to it would be a quadrangle long
    /// enough to cross whatever the rule uses to confine itself.
    EnclosureAbove(i32),
    /// The gaps between two layers narrower than `radius` DBU, as the quadrangles that
    /// span them (KLayout `a.drc(separation(b) < v).polygons`).  A measurement kept as
    /// geometry: a spacing *rule* asks whether two shapes are too close, this answers
    /// where they are close at all, which is what a maximum-distance rule needs - it
    /// reports the walls this leaves untouched.
    SeparationBelow(i32),
    /// Morphological open by `radius` DBU (erode-then-dilate); single source.  Removes
    /// every part of a region narrower than `2·radius` — a region that vanishes entirely
    /// has *no* position wider than that, which implements "min. X at one position" rules
    /// (e.g. pSD.e) via a subsequent `not_interacting` against the opened layer.
    Open(i32),
    /// Morphological grow (one-directional dilate) by `radius` DBU, plus `margin` DBU
    /// of slack; single source.  The slack
    /// is opt-in (a deck asks with `slack:`) and exists so a shape drawn at the boundary
    /// distance resolves as overlapping rather than merely touching; a band used to
    /// *classify* an edge wants it at zero instead, or an edge drawn exactly on the
    /// boundary is pulled to the wrong side of it.
    Grow(i32, i32),
    /// Morphological erode by `radius` DBU; single source (KLayout `sized(-r)`).  No
    /// dilate back, unlike [`Open`] — a region narrower than `2 * radius` vanishes and
    /// the rest keep their eroded outline.
    Shrink(i32),
    /// Morphological grow along **x only** by `radius` DBU: Minkowski sum with the
    /// horizontal segment `[-r, +r] × {0}` (KLayout `sized(r, 0)`).  Unlike [`Grow`],
    /// which dilates isotropically, this leaves the y extent untouched.
    GrowX(i32),
    /// Morphological grow along **y only** by `radius` DBU (KLayout `sized(0, r)`).
    GrowY(i32),
    /// Morphological shrink along **x only** by `radius` DBU (KLayout `sized(-r, 0)`).
    /// Erosion is the dual of [`GrowX`]: grow the complement, then complement back.
    ShrinkX(i32),
    /// Morphological shrink along **y only** by `radius` DBU (KLayout `sized(0, -r)`).
    ShrinkY(i32),
    /// Region-level selection: keep whole regions of source[0] that share positive
    /// *overlap area* with source[1] (KLayout `overlapping` / `not_outside`).  An
    /// edge-touching region is **not** kept — for that, use [`Interacting`].
    ///
    /// The pair is KLayout's `(min_count, max_count)` — how many *distinct* filter regions
    /// the candidate must meet, both bounds inclusive and both optional.  `(None, None)`
    /// is the uncounted form, "at least one", and takes a cheaper path in
    /// [`build_selection_tiles`]; every counted selector carries the same pair.
    Overlapping(Option<u32>, Option<u32>),
    /// Keep whole regions of source[0] that share no overlap area with source[1]
    /// (KLayout `not_overlapping` / `outside`).  An edge-touching region *is* kept.
    NotOverlapping(Option<u32>, Option<u32>),
    /// Region-level selection: keep whole regions of source[0] that overlap **or merely
    /// touch** a region of source[1] (KLayout `interacting`).  Differs from
    /// [`Overlapping`] only on zero-area contact: coincident edges and shared vertices
    /// count here and don't there.
    Interacting(Option<u32>, Option<u32>),
    /// Keep whole regions of source[0] that neither overlap nor touch source[1]
    /// (KLayout `not_interacting`).
    NotInteracting(Option<u32>, Option<u32>),
    /// Keep whole regions of source[0] that lie entirely within source[1] (KLayout
    /// `inside`) — no part of the region may fall outside the filter.  Unlike the other
    /// selectors this reduces with AND over the region's tile pieces, so *every* piece
    /// must be covered.
    Inside,
    /// Keep whole regions of source[0] that are **not** entirely within source[1]
    /// (KLayout `not_inside`).
    NotInside,
    /// Keep whole regions of source[0] that fully contain at least one whole region of
    /// source[1] (KLayout `covering`).  Containment is tested region-to-region, so a
    /// filter region straddling the candidate's edge does not count.
    Covering(Option<u32>, Option<u32>),
    /// Keep whole regions of source[0] that fully contain no region of source[1]
    /// (KLayout `not_covering`).
    NotCovering(Option<u32>, Option<u32>),
    /// Unary shape filter: keep regions that are neither a circle nor a regular octagon
    /// (KLayout `.not(get_circle).not(get_octagon)`, e.g. Padb.f's disallowed shapes).
    NotCircleOrOctagon,
    /// Unary shape filter: keep regions that are not a circle (KLayout `.not(get_circle)`,
    /// e.g. Padc.f's disallowed shapes).
    NotCircle,
    /// Unary: the *hole* areas of each source region, as filled polygons (KLayout
    /// `.holes` — e.g. the interior of a pSD substrate-tie ring).  Read on the stitched
    /// region, so a ring of any size has its hole - a guard ring round a pad frame as
    /// much as one round a device - and the source needs no halo for it.
    Holes,
    /// Unary shape filter: keep source regions that contain at least one hole (KLayout
    /// `.with_holes` — e.g. the ring-shaped NWell encircling an iso-PWell).  Read on
    /// the stitched region like [`VirtualOp::Holes`].
    WithHoles,
    /// Unary shape filter: keep regions whose outline is a filled axis-aligned rectangle
    /// (KLayout `rectangles`).  [`Square`] is the special case with equal sides.
    Rectangle,
    /// Unary shape filter: keep regions that are not filled axis-aligned rectangles
    /// (KLayout `non_rectangles`, e.g. MSLOT`#`.0 "slot is not a rectangle").
    NotRectangle,
    /// Unary region filter on total area, in DBU²: keep regions whose area is `>= min`
    /// (if set) and `< max` (if set) — KLayout `with_area(a..b)`.  Measured on the
    /// stitched region, so a shape spanning tiles is judged whole.
    WithArea(Option<i64>, Option<i64>),
    /// Unary shape filter on the **shorter** bounding-box side, in DBU: keep regions whose
    /// short side is `>= min` (if set) and `< max` (if set) — KLayout `with_bbox_min(a..b)`.
    /// Both bounds open means "keep everything".
    WithBBoxMin(Option<i32>, Option<i32>),
    /// Unary shape filter on the **longer** bounding-box side, in DBU (KLayout
    /// `with_bbox_max(a..b)`); same half-open `[min, max)` convention as [`WithBBoxMin`].
    WithBBoxMax(Option<i32>, Option<i32>),
    /// Each whole region of the source replaced by its bounding box (KLayout `.extents`).
    ///
    /// Region-level, so it is evaluated on the stitched shape: a tile piece of an L has a
    /// bounding box, but not the L's.  What the reference deck uses it for is turning a
    /// device's scattered pieces into the one rectangle they occupy, to measure against.
    Extents,
    /// Region-level selection: keep whole regions of source[0] containing a text label
    /// on source[1] (a text layer) matching the def's pattern (KLayout
    /// `ext_interacting_with_text`).  Routed specially in `ensure` (needs the layout's
    /// texts, not polygon tiles).
    WithText,
}

/// A region-level unary filter: which measured quantity, and its half-open bounds.
/// Unlike the shape predicates in [`compose_tile`], these are properties of a whole
/// stitched region — a tile piece of a 15000 um² marker has neither its area nor its
/// extent — so they are evaluated after stitching, like the selectors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegionFilter {
    /// Total area in DBU².
    Area(Option<i64>, Option<i64>),
    /// The shorter side of the region's bounding box, in DBU.
    ShortSide(Option<i32>, Option<i32>),
    /// The longer side of the region's bounding box, in DBU.
    LongSide(Option<i32>, Option<i32>),
}

/// A selector's count bounds: how many *distinct* filter regions a candidate must meet,
/// `(min, max)`, both inclusive and both optional.  `(None, None)` is KLayout's uncounted
/// form and means "at least one".
pub type Count = (Option<u32>, Option<u32>);

/// How a region-level selector decides whether to keep a candidate region.  Each maps
/// to a predicate over the region's tile pieces and the filter geometry sharing those
/// tiles; see [`build_selection_tiles`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionKind {
    /// Any piece shares positive overlap area with the filter.
    Overlaps,
    /// Any piece overlaps or touches the filter (zero-area contact counts).
    Touches,
    /// *Every* piece is fully covered by the filter — the one AND-reducing predicate.
    Inside,
    /// Any piece fully contains a whole filter region.
    Covers,
}

impl VirtualOp {
    /// Region-level selectors are evaluated on stitched whole regions in
    /// [`MergedCache::ensure`], not composed per tile like the boolean ops.  Returns the
    /// predicate and whether a matching region is kept (`true`) or dropped (`false`).
    fn selection(self) -> Option<(SelectionKind, bool, Count)> {
        use SelectionKind as K;
        let none = (None, None);
        match self {
            VirtualOp::Overlapping(a, b) => Some((K::Overlaps, true, (a, b))),
            VirtualOp::NotOverlapping(a, b) => Some((K::Overlaps, false, (a, b))),
            VirtualOp::Interacting(a, b) => Some((K::Touches, true, (a, b))),
            VirtualOp::NotInteracting(a, b) => Some((K::Touches, false, (a, b))),
            VirtualOp::Inside => Some((K::Inside, true, none)),
            VirtualOp::NotInside => Some((K::Inside, false, none)),
            VirtualOp::Covering(a, b) => Some((K::Covers, true, (a, b))),
            VirtualOp::NotCovering(a, b) => Some((K::Covers, false, (a, b))),
            _ => None,
        }
    }

    /// The region-level unary filter this op is, if it is one.  See [`RegionFilter`].
    fn region_filter(self) -> Option<RegionFilter> {
        match self {
            VirtualOp::WithArea(lo, hi) => Some(RegionFilter::Area(lo, hi)),
            VirtualOp::WithBBoxMin(lo, hi) => Some(RegionFilter::ShortSide(lo, hi)),
            VirtualOp::WithBBoxMax(lo, hi) => Some(RegionFilter::LongSide(lo, hi)),
            _ => None,
        }
    }
}

/// True if a merged region is a filled axis-aligned rectangle: no holes, and its outline
/// encloses exactly its bounding box.  Coordinates are integer DBU, so the area test is
/// exact (the 0.5 slack only absorbs the f64 arithmetic in [`ring_area2`]).
fn is_rectangle(m: &MergedPoly) -> bool {
    if !m.holes.is_empty() || m.outer.is_empty() {
        return false;
    }
    let (x0, y0, x1, y1) = outer_bbox(&m.outer);
    let (w, h) = (x1 - x0, y1 - y0);
    if w <= 0 || h <= 0 {
        return false;
    }
    (ring_area2(&m.outer) - 2.0 * (w * h) as f64).abs() < 0.5
}

/// Whether `side` falls in the half-open range `[min, max)`; an absent bound is open.
fn side_in_range(side: i64, min: Option<i32>, max: Option<i32>) -> bool {
    min.is_none_or(|v| side >= v as i64) && max.is_none_or(|v| side < v as i64)
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
        VirtualOp::Square => sources[0]
            .iter()
            .filter(|m| is_square(m))
            .cloned()
            .collect(),
        VirtualOp::NotSquare => sources[0]
            .iter()
            .filter(|m| !is_square(m))
            .cloned()
            .collect(),
        VirtualOp::NotCircleOrOctagon => sources[0]
            .iter()
            .filter(|m| !is_circle(m) && !is_octagon(m))
            .cloned()
            .collect(),
        VirtualOp::NotCircle => sources[0]
            .iter()
            .filter(|m| !is_circle(m))
            .cloned()
            .collect(),
        VirtualOp::Holes | VirtualOp::WithHoles => {
            unreachable!("holes are read on stitched regions, see build_holes_tiles")
        }
        VirtualOp::Rectangle => sources[0]
            .iter()
            .filter(|m| is_rectangle(m))
            .cloned()
            .collect(),
        VirtualOp::NotRectangle => sources[0]
            .iter()
            .filter(|m| !is_rectangle(m))
            .cloned()
            .collect(),
        // Area and bounding-box filters are routed to `build_region_filter_tiles`: a
        // region's area and extent are properties of the whole shape, and a tile piece
        // of one has neither.
        VirtualOp::WithArea(_, _) | VirtualOp::WithBBoxMin(_, _) | VirtualOp::WithBBoxMax(_, _) => {
            Vec::new()
        }
        // Text selection is routed to `build_text_selection_tiles` in `ensure`, and
        // `extents` to `build_extents_tiles` - both need whole regions, not tile pieces.
        VirtualOp::WithText | VirtualOp::Extents => Vec::new(),
        // Morphological close of the single source by `r` DBU.  The source tile carries a
        // halo ≥ 2·r (set in run_drc) so dilate-then-erode is exact in the core.
        VirtualOp::EnclosureAbove(r) => {
            let (Some(outer), Some(inner)) = (sources.first(), sources.get(1)) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for pi in inner.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                for po in outer.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                    // The nearest outer wall per inner wall is that wall's real margin.
                    let mut best: HashMap<(i64, i64, i64, i64), EnclosurePair> = HashMap::new();
                    let reach = r as f64 * ENCLOSURE_MAX_REACH;
                    for p in enclosure_pairs(&pi, &po, reach, false, false, 0.0).0 {
                        let k = (
                            p.edge.0.round() as i64,
                            p.edge.1.round() as i64,
                            p.edge.2.round() as i64,
                            p.edge.3.round() as i64,
                        );
                        if best.get(&k).is_none_or(|q| p.dist < q.dist) {
                            best.insert(k, p);
                        }
                    }
                    out.extend(
                        best.into_values()
                            .filter(|p| p.dist > r as f64)
                            .filter_map(|p| enclosure_quad(&p)),
                    );
                }
            }
            out
        }
        VirtualOp::EnclosureBelow(r) => {
            let (Some(outer), Some(inner)) = (sources.first(), sources.get(1)) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for pi in inner.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                for po in outer.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                    let (pairs, _) = enclosure_pairs(&pi, &po, r as f64, false, false, 0.0);
                    for p in pairs {
                        let (x1, y1, x2, y2) = p.edge;
                        let (mx, my) = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                        // The probe sits just past the outer wall this margin was measured
                        // to, so it points the way the margin runs.
                        let (vx, vy) = (p.probe.0 - mx, p.probe.1 - my);
                        let l = vx.hypot(vy);
                        if l <= 0.0 {
                            continue;
                        }
                        let (ox, oy) = (vx / l * p.dist, vy / l * p.dist);
                        let outer_ring: Vec<IntPoint> =
                            [(x1, y1), (x2, y2), (x2 + ox, y2 + oy), (x1 + ox, y1 + oy)]
                                .iter()
                                .map(|&(x, y)| IntPoint::new(x.round() as i32, y.round() as i32))
                                .collect();
                        if ring_area2(&outer_ring).abs() > 0.0 {
                            out.push(MergedPoly {
                                outer: outer_ring,
                                holes: Vec::new(),
                            });
                        }
                    }
                }
            }
            out
        }
        VirtualOp::SeparationBelow(r) => {
            let (Some(a), Some(b)) = (sources.first(), sources.get(1)) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for pa in a.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                for pb in b.iter().filter_map(|m| poly_from_merged(m, 1.0)) {
                    // Euclidian, as the rule asks: the facing runs, plus the closest
                    // approach of the two outlines, which is what carries a corner that
                    // nears another corner without any wall facing it.
                    let mut runs = facing_runs(&pa, &pb, r as f64, 0.0, false);
                    let (d, ca, cb) = closest(&pa, &pb, 0.0, false);
                    // Only where no wall faces another: two corners that near each other
                    // are within the euclidian distance the rule asks for, and no facing
                    // run describes them.
                    if runs.is_empty() && d < r as f64 {
                        let (vx, vy) = (cb.0 - ca.0, cb.1 - ca.1);
                        let l = vx.hypot(vy).max(1.0);
                        let (px, py) = (-vy / l, vx / l);
                        runs.push(FacingRun {
                            gap: d,
                            a: (ca.0 - px, ca.1 - py, ca.0 + px, ca.1 + py),
                            b: (cb.0 - px, cb.1 - py, cb.0 + px, cb.1 + py),
                            closest: (ca, cb),
                        });
                    }
                    for run in runs {
                        let quad = [
                            (run.a.0, run.a.1),
                            (run.a.2, run.a.3),
                            (run.b.2, run.b.3),
                            (run.b.0, run.b.1),
                        ];
                        let outer: Vec<IntPoint> = quad
                            .iter()
                            .map(|&(x, y)| IntPoint::new(x.round() as i32, y.round() as i32))
                            .collect();
                        if ring_area2(&outer).abs() > 0.0 {
                            out.push(MergedPoly {
                                outer,
                                holes: Vec::new(),
                            });
                        }
                    }
                }
            }
            out
        }
        VirtualOp::Close(r) => closing(sources[0], r as f64),
        VirtualOp::Open(r) => opening(sources[0], r as f64),
        VirtualOp::Grow(r, m) => grow_by(sources[0], r as f64, m as f64),
        VirtualOp::Shrink(r) => shrink(sources[0], r as f64),
        VirtualOp::GrowX(r) => grow_x(sources[0], r as f64),
        VirtualOp::GrowY(r) => grow_y(sources[0], r as f64),
        VirtualOp::ShrinkX(r) => shrink_x(sources[0], r as f64),
        VirtualOp::ShrinkY(r) => shrink_y(sources[0], r as f64),
        // Region selectors are not composable per tile — `ensure` routes them to
        // `build_selection_tiles` before this point.
        VirtualOp::Overlapping(_, _)
        | VirtualOp::NotOverlapping(_, _)
        | VirtualOp::Interacting(_, _)
        | VirtualOp::NotInteracting(_, _)
        | VirtualOp::Inside
        | VirtualOp::NotInside
        | VirtualOp::Covering(_, _)
        | VirtualOp::NotCovering(_, _) => Vec::new(),
    }
}

/// Clip an edge to a closed rectangle, or `None` where nothing of it lies inside.
///
/// Closed on every side on purpose: an edge lying exactly along a tile's boundary belongs
/// to that tile as much as to its neighbour, and dropping it on both sides is how an edge
/// goes missing altogether.  The duplicate that costs is rejoined afterwards.
fn clip_edge(e: Edge, core: (i64, i64, i64, i64)) -> Option<Edge> {
    let (px, py) = (e.a.x as f64, e.a.y as f64);
    let (dx, dy) = ((e.b.x - e.a.x) as f64, (e.b.y - e.a.y) as f64);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, px - core.0 as f64),
        (dx, core.2 as f64 - px),
        (-dy, py - core.1 as f64),
        (dy, core.3 as f64 - py),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None; // parallel to this side and outside it
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            if r > t1 {
                return None;
            }
            t0 = t0.max(r);
        } else {
            if r < t0 {
                return None;
            }
            t1 = t1.min(r);
        }
    }
    if t1 <= t0 {
        return None;
    }
    let pt = |t: f64| IntPoint::new((px + t * dx).round() as i32, (py + t * dy).round() as i32);
    let (a, b) = (pt(t0), pt(t1));
    (a != b).then_some(Edge { a, b })
}

/// A supporting line: a direction reduced to its smallest integer step, and how far the
/// line sits from the origin perpendicular to it.
type Line = (i64, i64, i64);

/// One piece on such a line: where it starts and ends along the direction, and the two
/// points it was cut between.
type LinePiece = (i64, i64, IntPoint, IntPoint);

/// Put an edge back together from the pieces the tiles cut it into.
///
/// Every piece of one edge shares its supporting line *and its direction* - the direction
/// carries which side the material is on, so pieces running the other way are a different
/// wall and are left alone.  Within a line the pieces are intervals: sort them and merge
/// the ones that touch or overlap.
fn rejoin_collinear(edges: Vec<Edge>) -> Vec<Edge> {
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 {
            a.abs().max(1)
        } else {
            gcd(b, a % b)
        }
    }
    let mut by_line: HashMap<Line, Vec<LinePiece>> = HashMap::new();
    for e in edges {
        let (dx, dy) = ((e.b.x - e.a.x) as i64, (e.b.y - e.a.y) as i64);
        let g = gcd(dx, dy);
        let (ux, uy) = (dx / g, dy / g);
        // The line: its direction, and where it sits perpendicular to that direction.
        let off = e.a.x as i64 * uy - e.a.y as i64 * ux;
        let t = |p: IntPoint| p.x as i64 * ux + p.y as i64 * uy;
        by_line
            .entry((ux, uy, off))
            .or_default()
            .push((t(e.a), t(e.b), e.a, e.b));
    }
    let mut out = Vec::new();
    for (_, mut v) in by_line {
        v.sort_unstable_by_key(|&(t0, _, _, _)| t0);
        let (mut lo, mut hi, mut a, mut b) = v[0];
        for &(t0, t1, pa, pb) in &v[1..] {
            if t0 <= hi {
                if t1 > hi {
                    hi = t1;
                    b = pb;
                }
            } else {
                out.push(Edge { a, b });
                (lo, hi, a, b) = (t0, t1, pa, pb);
            }
        }
        let _ = lo;
        out.push(Edge { a, b });
    }
    out
}

/// The margin a pair measures, as the quadrangle spanning it: the inner wall, swept to the
/// outer wall it was measured to.  The probe sits just past that wall, which is what says
/// which way the sweep runs.
fn enclosure_quad(p: &EnclosurePair) -> Option<MergedPoly> {
    let (x1, y1, x2, y2) = p.edge;
    let (mx, my) = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
    let (vx, vy) = (p.probe.0 - mx, p.probe.1 - my);
    let l = vx.hypot(vy);
    if l <= 0.0 {
        return None;
    }
    let (ox, oy) = (vx / l * p.dist, vy / l * p.dist);
    let outer: Vec<IntPoint> = [(x1, y1), (x2, y2), (x2 + ox, y2 + oy), (x1 + ox, y1 + oy)]
        .iter()
        .map(|&(x, y)| IntPoint::new(x.round() as i32, y.round() as i32))
        .collect();
    (ring_area2(&outer).abs() > 0.0).then_some(MergedPoly {
        outer,
        holes: Vec::new(),
    })
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
        // A gap only materialises in a tile that holds both sides of it.
        VirtualOp::SeparationBelow(_)
        | VirtualOp::EnclosureBelow(_)
        | VirtualOp::EnclosureAbove(_)
        | VirtualOp::Intersection => {
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
        | VirtualOp::Rectangle
        | VirtualOp::NotRectangle
        | VirtualOp::WithArea(_, _)
        | VirtualOp::WithBBoxMin(_, _)
        | VirtualOp::WithBBoxMax(_, _)
        | VirtualOp::Close(_)
        | VirtualOp::Open(_)
        | VirtualOp::Grow(_, _)
        | VirtualOp::Shrink(_)
        | VirtualOp::GrowX(_)
        | VirtualOp::GrowY(_)
        | VirtualOp::ShrinkX(_)
        | VirtualOp::ShrinkY(_)
        | VirtualOp::NotCircleOrOctagon
        | VirtualOp::NotCircle
        | VirtualOp::Holes
        | VirtualOp::WithHoles => first.keys().copied().collect(),
        // Selection ops never reach here (handled in `ensure`).
        VirtualOp::Overlapping(_, _)
        | VirtualOp::NotOverlapping(_, _)
        | VirtualOp::Interacting(_, _)
        | VirtualOp::NotInteracting(_, _)
        | VirtualOp::Inside
        | VirtualOp::NotInside
        | VirtualOp::Covering(_, _)
        | VirtualOp::NotCovering(_, _)
        | VirtualOp::WithText
        | VirtualOp::Extents => first.keys().copied().collect(),
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

    /// Whether this tile owns a violation reported at a *region's* centroid `(x, y)`,
    /// in DBU.  Half-open, unlike [`Core::owns`]: a region's centroid is the average of
    /// its outer vertices, so it can only land on a tile line when the region has
    /// vertices on both sides - and a region straddling the line is filed in both tiles,
    /// so either may own it and the upper one does.  Claiming it from both, as `owns`
    /// must for a point between two shapes, relied on the two tiles' copies of the
    /// region being byte-identical for the duplicate to fold, and they are not once the
    /// region outreaches the halo: IHP's npnG2.c reported one tie ring's wall twice,
    /// the ring centred on a tile line and each tile's pSD copy merged with a different
    /// neighbouring bar.
    pub fn owns_region(&self, x: f64, y: f64) -> bool {
        self.contains(x, y)
    }

    /// Whether this tile owns a violation reported at `(x, y)`, in DBU.
    ///
    /// Ownership is what stops a pair two tiles can both see from being reported twice,
    /// and a half-open core settles that for any point strictly inside one.  A point
    /// landing *on* a shared line is the exception, and the half-open rule gets it wrong:
    /// it hands the point to the tile above, which is the one whose geometry need not
    /// reach back down to it - a shape whose edge stops on the line has no area past it
    /// and so is not filed beyond it at all, and the violation is then claimed by a tile
    /// that cannot see either shape and dropped by the tile that can.
    ///
    /// Handing it to the tile below instead only mirrors the problem, since a shape can
    /// stop on the line from either side.  Neither tile is reliably the right owner, so a
    /// point on a line is claimed by *both*: the core closes on its upper edges, every
    /// tile that can see the pair reports it, and the duplicate that creates is dropped
    /// afterwards by [`run_drc`], which already sorts the violations and so has identical
    /// ones adjacent.  Only points exactly on a tile line are ever claimed twice.
    pub fn owns(&self, x: f64, y: f64) -> bool {
        x >= self.x0 as f64 && x <= self.x1 as f64 && y >= self.y0 as f64 && y <= self.y1 as f64
    }
}

/// Merged geometry of one layer, indexed by global tile `(tx, ty)`.
pub type TileMap = HashMap<(i32, i32), Vec<MergedPoly>>;

// ===========================================================================
// Edge layers
//
// A polygon layer's atom is a filled region: booleans combine areas, selectors keep or
// drop whole regions, and the checks measure between facing walls.  That leaves a class
// of rules unsayable, because they are about *one piece of a boundary* rather than about
// a region.  A transistor's channel width is the length of the Activ boundary that runs
// under the gate; no region has that length, and the source/drain region's own width is
// a different quantity entirely.
//
// So edges get their own layer type, whose elements are individual boundary segments.
// The same tiling applies — a segment belongs to the tile holding its midpoint — so edge
// layers compose with polygon layers in the merge cache and inherit its memory bound.
// ===========================================================================

/// A directed boundary segment, in DBU.  Direction carries the same convention as the
/// contour it came from: material lies to the left of `a → b`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Edge {
    pub a: IntPoint,
    pub b: IntPoint,
}

impl Edge {
    pub fn length(&self) -> f64 {
        (((self.b.x - self.a.x) as f64).powi(2) + ((self.b.y - self.a.y) as f64).powi(2)).sqrt()
    }

    pub fn midpoint(&self) -> (f64, f64) {
        (
            (self.a.x as f64 + self.b.x as f64) * 0.5,
            (self.a.y as f64 + self.b.y as f64) * 0.5,
        )
    }

    /// Angle of `a → b` in degrees, normalised to `[0, 180)` — an edge's orientation is
    /// the same whichever way it is walked, so 270° and 90° are one answer.
    pub fn angle_deg(&self) -> f64 {
        let d = ((self.b.y - self.a.y) as f64).atan2((self.b.x - self.a.x) as f64);
        let mut deg = d.to_degrees();
        while deg < 0.0 {
            deg += 180.0;
        }
        while deg >= 180.0 {
            deg -= 180.0;
        }
        deg
    }
}

/// Edge geometry of one layer, indexed by global tile `(tx, ty)`.
pub type EdgeTileMap = HashMap<(i32, i32), Vec<Edge>>;

/// Every contour segment of a merged region — outer ring and holes alike.
pub fn region_edges(m: &MergedPoly) -> Vec<Edge> {
    let mut out = Vec::new();
    for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (a, b) = (ring[i], ring[if i + 1 == n { 0 } else { i + 1 }]);
            if a != b {
                out.push(Edge { a, b });
            }
        }
    }
    out
}

/// The portion of `e` that lies on `f`, if the two are collinear and overlap in more
/// than a point.  This is what an edge-layer `and` means: not the area two layers share,
/// but the stretch of boundary they have in common.
/// The line an edge lies on: its direction reduced to lowest terms with a fixed sign,
/// and the line's offset from the origin.  Two edges are collinear exactly when their
/// keys agree, which is what lets an edge boolean look its partners up instead of
/// testing every pair.
fn line_key(e: &Edge) -> (i64, i64, i64) {
    let (mut dx, mut dy) = (e.b.x as i64 - e.a.x as i64, e.b.y as i64 - e.a.y as i64);
    let g = gcd(dx.abs(), dy.abs());
    if g > 1 {
        dx /= g;
        dy /= g;
    }
    if dx < 0 || (dx == 0 && dy < 0) {
        dx = -dx;
        dy = -dy;
    }
    (dx, dy, dx * e.a.y as i64 - dy * e.a.x as i64)
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn line_index(edges: &[Edge]) -> HashMap<(i64, i64, i64), Vec<Edge>> {
    let mut index: HashMap<(i64, i64, i64), Vec<Edge>> = HashMap::new();
    for e in edges {
        index.entry(line_key(e)).or_default().push(*e);
    }
    index
}

fn edge_overlap(e: &Edge, f: &Edge) -> Option<Edge> {
    let (ex, ey) = (e.b.x as i64 - e.a.x as i64, e.b.y as i64 - e.a.y as i64);
    let (fx, fy) = (f.b.x as i64 - f.a.x as i64, f.b.y as i64 - f.a.y as i64);
    // Parallel, and f's start on e's line: the two are collinear.
    if ex * fy - ey * fx != 0 {
        return None;
    }
    if ex * (f.a.y as i64 - e.a.y as i64) - ey * (f.a.x as i64 - e.a.x as i64) != 0 {
        return None;
    }
    // Project both onto e's direction and intersect the parameter ranges.
    let len2 = ex * ex + ey * ey;
    if len2 == 0 {
        return None;
    }
    let t = |p: IntPoint| (p.x as i64 - e.a.x as i64) * ex + (p.y as i64 - e.a.y as i64) * ey;
    let (t0, t1) = (0, len2);
    let (mut u0, mut u1) = (t(f.a), t(f.b));
    if u0 > u1 {
        std::mem::swap(&mut u0, &mut u1);
    }
    let (lo, hi) = (t0.max(u0), t1.min(u1));
    if hi <= lo {
        return None; // disjoint, or touching at a single point
    }
    let at = |s: i64| IntPoint {
        x: (e.a.x as i64 + ex * s / len2) as i32,
        y: (e.a.y as i64 + ey * s / len2) as i32,
    };
    Some(Edge {
        a: at(lo),
        b: at(hi),
    })
}

/// `a` with every stretch it shares with any edge of `b` removed.
fn subtract_edges(a: &Edge, b: &[Edge]) -> Vec<Edge> {
    let (dx, dy) = (a.b.x as i64 - a.a.x as i64, a.b.y as i64 - a.a.y as i64);
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return Vec::new();
    }
    // Collect the covered parameter ranges along `a`, then keep the gaps.
    let mut cuts: Vec<(i64, i64)> = Vec::new();
    for f in b {
        if let Some(o) = edge_overlap(a, f) {
            let t =
                |p: IntPoint| (p.x as i64 - a.a.x as i64) * dx + (p.y as i64 - a.a.y as i64) * dy;
            let (mut s, mut e) = (t(o.a), t(o.b));
            if s > e {
                std::mem::swap(&mut s, &mut e);
            }
            cuts.push((s, e));
        }
    }
    if cuts.is_empty() {
        return vec![*a];
    }
    cuts.sort_unstable();
    let at = |s: i64| IntPoint {
        x: (a.a.x as i64 + dx * s / len2) as i32,
        y: (a.a.y as i64 + dy * s / len2) as i32,
    };
    let mut out = Vec::new();
    let mut pos = 0i64;
    for (s, e) in cuts {
        if s > pos {
            out.push(Edge {
                a: at(pos),
                b: at(s),
            });
        }
        pos = pos.max(e);
    }
    if pos < len2 {
        out.push(Edge {
            a: at(pos),
            b: at(len2),
        });
    }
    out.retain(|e| e.a != e.b);
    out
}

/// Split `e` where it crosses the boundary of `polys`, keeping the parts whose midpoint
/// is inside (`keep_inside`) or outside it.  This is KLayout's `inside_part` /
/// `outside_part`: an edge is not kept or dropped whole, it is cut.
/// A uniform grid over bounding boxes, so an edge asks only the polygons or edges it
/// could meet at all rather than every one gathered for the tile.  The edge cuts and
/// selections were trying each edge against everything in reach: `inside_part` on
/// 16 million gate edges against the COMP in nine tiles was 90 seconds of contour
/// crossings that could not happen.
struct BoxGrid {
    cell: i64,
    cells: HashMap<(i64, i64), Vec<u32>>,
}

impl BoxGrid {
    fn new(boxes: &[(i32, i32, i32, i32)], cell: i64) -> Self {
        let cell = cell.max(1);
        let mut cells: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, &(x0, y0, x1, y1)) in boxes.iter().enumerate() {
            for cx in (x0 as i64).div_euclid(cell)..=(x1 as i64).div_euclid(cell) {
                for cy in (y0 as i64).div_euclid(cell)..=(y1 as i64).div_euclid(cell) {
                    cells.entry((cx, cy)).or_default().push(i as u32);
                }
            }
        }
        BoxGrid { cell, cells }
    }

    /// The indices filed in any cell the box touches, each once.
    fn hits(&self, (x0, y0, x1, y1): (i32, i32, i32, i32), out: &mut Vec<u32>) {
        out.clear();
        for cx in (x0 as i64).div_euclid(self.cell)..=(x1 as i64).div_euclid(self.cell) {
            for cy in (y0 as i64).div_euclid(self.cell)..=(y1 as i64).div_euclid(self.cell) {
                if let Some(v) = self.cells.get(&(cx, cy)) {
                    out.extend_from_slice(v);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

fn edge_bbox(e: &Edge) -> (i32, i32, i32, i32) {
    (
        e.a.x.min(e.b.x),
        e.a.y.min(e.b.y),
        e.a.x.max(e.b.x),
        e.a.y.max(e.b.y),
    )
}

fn boxes_meet(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    !(a.2 < b.0 || b.2 < a.0 || a.3 < b.1 || b.3 < a.1)
}

/// `polys` come with their contour edges, computed once per tile rather than once per
/// subject edge: a tile of COMP against sixteen million gate edges rebuilt every
/// contour for every one of them.
fn edge_vs_polygons(e: &Edge, polys: &[(&MergedPoly, &[Edge])], keep_inside: bool) -> Vec<Edge> {
    let (dx, dy) = (e.b.x as i64 - e.a.x as i64, e.b.y as i64 - e.a.y as i64);
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return Vec::new();
    }
    // Cut parameters: the ends, plus every crossing of a polygon contour.
    let mut ts: Vec<f64> = vec![0.0, 1.0];
    let eb = edge_bbox(e);
    for (_, edges) in polys {
        for f in *edges {
            if boxes_meet(eb, edge_bbox(f))
                && let Some(t) = seg_cross_param(e, f)
            {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

    let at = |t: f64| IntPoint {
        x: (e.a.x as f64 + dx as f64 * t).round() as i32,
        y: (e.a.y as f64 + dy as f64 * t).round() as i32,
    };
    let mut out = Vec::new();
    for w in ts.windows(2) {
        let mid = (w[0] + w[1]) * 0.5;
        let (mx, my) = (
            e.a.x as f64 + dx as f64 * mid,
            e.a.y as f64 + dy as f64 * mid,
        );
        let inside = polys.iter().any(|(m, _)| point_in_merged(mx, my, m));
        if inside == keep_inside {
            let (p, q) = (at(w[0]), at(w[1]));
            if p != q {
                out.push(Edge { a: p, b: q });
            }
        }
    }
    out
}

/// Parameter along `e` (in `0..=1`) where it properly crosses `f`, if it does.
fn seg_cross_param(e: &Edge, f: &Edge) -> Option<f64> {
    let (ex, ey) = (e.b.x as f64 - e.a.x as f64, e.b.y as f64 - e.a.y as f64);
    let (fx, fy) = (f.b.x as f64 - f.a.x as f64, f.b.y as f64 - f.a.y as f64);
    let den = ex * fy - ey * fx;
    if den.abs() < 1e-12 {
        return None; // parallel or collinear: no single crossing point
    }
    let (gx, gy) = (f.a.x as f64 - e.a.x as f64, f.a.y as f64 - e.a.y as f64);
    let t = (gx * fy - gy * fx) / den;
    let u = (gx * ey - gy * ex) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// Operator for a tiled edge layer.  `Edges` is the bridge from polygons; the rest
/// combine or filter edge layers, and `InsidePart`/`OutsidePart` cut an edge layer
/// against a polygon layer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeOp {
    /// Every contour segment of the source *polygon* layer (KLayout `.edges`).
    Edges,
    /// The walls of the source *polygon* layer that face another wall closer than the
    /// value, both walls of each pair (KLayout `layer.width(v).edges`).  A measurement
    /// kept as geometry rather than reported: the width rules answer "is this too
    /// narrow", this answers "which walls are", so a rule can then ask where they lie.
    WidthBelow(i32),
    /// The stretches two edge layers have in common (KLayout edge `.and`).  Collinear
    /// overlap, not shared area.
    And,
    /// The first edge layer with every stretch it shares with the second removed
    /// (KLayout edge `.not`).
    Not,
    /// Every edge of every source layer, kept as they are (KLayout edge `.join`).  Not a
    /// merge: two collinear edges stay two, because an edge layer is a bag of segments
    /// and the checks that read one measure each segment on its own.
    Or,
    /// The parts of the edge layer lying inside the *polygon* layer, cut at the
    /// boundary (KLayout `.inside_part`).
    InsidePart,
    /// The parts lying outside it (KLayout `.outside_part`).
    OutsidePart,
    /// Whole edges that touch the *polygon* layer, kept entire (KLayout edge
    /// `.interacting`).  Unlike [`InsidePart`], which cuts an edge at the boundary, this
    /// keeps or drops each segment whole — the difference between "the stretch of this
    /// wall that lies in the marker" and "the walls that reach it at all".
    /// `NotInteracting` keeps the complement.
    Interacting,
    NotInteracting,
    /// The same against another *edge* layer: whole edges of the first that touch any
    /// edge of the second.
    InteractingEdges,
    NotInteractingEdges,
    /// Keep edges whose length is in `[min, max)` DBU; an absent bound is open
    /// (KLayout `.with_length(a..b)`).  `WithoutLength` keeps the complement.
    WithLength(Option<i32>, Option<i32>),
    WithoutLength(Option<i32>, Option<i32>),
    /// Keep edges whose orientation in degrees is in `[min, max)`, normalised to
    /// `[0, 180)` (KLayout `.with_angle`).  `WithoutAngle` keeps the complement.
    WithAngle(i32, i32),
    WithoutAngle(i32, i32),
    /// The middle part of each edge (KLayout `.centers(length, fraction)`): an absolute
    /// length in DBU and a fraction in per mille, of which the *longer* is taken, centred
    /// on the edge's midpoint.
    ///
    /// What it is for is trimming the ends off.  Two edges meeting at a corner touch at
    /// that vertex, so an `interacting` between them answers yes for reasons that have
    /// nothing to do with the rule; shaving a percent off each end makes contact mean
    /// overlap.  Hence `centers(0, 0.99)` all over the reference deck.
    Centers(i32, i32),
}

impl EdgeOp {
    /// True if the op's sources are polygon layers rather than edge layers.
    fn takes_polygons(self) -> bool {
        matches!(self, EdgeOp::Edges | EdgeOp::WidthBelow(_))
    }
    /// True if source[1] is a polygon layer while source[0] is an edge layer.
    fn mixes(self) -> bool {
        matches!(
            self,
            EdgeOp::InsidePart | EdgeOp::OutsidePart | EdgeOp::Interacting | EdgeOp::NotInteracting
        )
    }
}

/// Whether a segment touches a region at all — a point of it inside, or a crossing of the
/// boundary.  The test `Interacting` selects whole edges by.
fn edge_meets_region(e: &Edge, m: &MergedPoly) -> bool {
    poly_meets_edge(m, e)
}

/// Whether two segments touch: crossing, or meeting at a point, or overlapping
/// collinearly.  Endpoint contact counts, which is what `interacting` means.
fn edges_touch(a: &Edge, b: &Edge) -> bool {
    let (p0, p1) = ((a.a.x as f64, a.a.y as f64), (a.b.x as f64, a.b.y as f64));
    let (q0, q1) = ((b.a.x as f64, b.a.y as f64), (b.b.x as f64, b.b.y as f64));
    let cross = |o: (f64, f64), u: (f64, f64), v: (f64, f64)| {
        (u.0 - o.0) * (v.1 - o.1) - (u.1 - o.1) * (v.0 - o.0)
    };
    const EPS: f64 = 1e-9;
    let (d1, d2) = (cross(q0, q1, p0), cross(q0, q1, p1));
    let (d3, d4) = (cross(p0, p1, q0), cross(p0, p1, q1));
    if ((d1 > EPS && d2 < -EPS) || (d1 < -EPS && d2 > EPS))
        && ((d3 > EPS && d4 < -EPS) || (d3 < -EPS && d4 > EPS))
    {
        return true;
    }
    let within = |o: (f64, f64), u: (f64, f64), p: (f64, f64)| {
        p.0 >= o.0.min(u.0) - EPS
            && p.0 <= o.0.max(u.0) + EPS
            && p.1 >= o.1.min(u.1) - EPS
            && p.1 <= o.1.max(u.1) + EPS
    };
    (d1.abs() <= EPS && within(q0, q1, p0))
        || (d2.abs() <= EPS && within(q0, q1, p1))
        || (d3.abs() <= EPS && within(p0, p1, q0))
        || (d4.abs() <= EPS && within(p0, p1, q1))
}

/// Apply an edge op to one tile's geometry.  `core` is the composing tile's core, in DBU.
///
/// Extraction is the one op that has to know which tile it is in.  A tile holds every
/// polygon that reaches it, whole and unmerged with the neighbours out of its reach, so a
/// shape drawn edge to edge with another can appear in one tile joined and in the next one
/// alone - and alone it still has the wall they share, which is interior and not boundary.
/// Keeping only the segments whose midpoint falls in the core makes each wall the business
/// of exactly one tile: the one that owns it, and so the one with the most of its
/// neighbourhood in reach.  The same midpoint convention `bucket_edges` uses.
fn compose_edge_tile(
    op: EdgeOp,
    edge_srcs: &[&[Edge]],
    poly_srcs: &[&[MergedPoly]],
    core: (i64, i64, i64, i64),
) -> Vec<Edge> {
    match op {
        EdgeOp::WidthBelow(v) => poly_srcs
            .first()
            .map(|ps| {
                let limit = v as f64;
                ps.iter()
                    .flat_map(|p| {
                        width_pairs(
                            p,
                            Core {
                                x0: core.0,
                                y0: core.1,
                                x1: core.2,
                                y1: core.3,
                            },
                            |w| w < limit,
                            |_, _| true,
                            false,
                            true,
                            0.0,
                        )
                    })
                    .map(|(x1, y1, x2, y2, _)| Edge {
                        a: IntPoint::new(x1.round() as i32, y1.round() as i32),
                        b: IntPoint::new(x2.round() as i32, y2.round() as i32),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        // Each tile contributes the stretch of boundary its own core covers, and no more.
        // Charging a whole edge to the tile its midpoint falls in reads that tile's merge
        // for a boundary another tile drew: the tiles disagree about where a region ends
        // when it is drawn as several abutting pieces, and the edge either comes back
        // twice at two granularities or not at all.  Clipping asks each tile only about
        // the part it is sure of, and `rejoin_collinear` puts the edge back together.
        EdgeOp::Edges => poly_srcs
            .first()
            .map(|ps| {
                ps.iter()
                    .flat_map(region_edges)
                    .filter_map(|e| clip_edge(e, core))
                    .collect()
            })
            .unwrap_or_default(),
        // Only collinear edges share a stretch, so the filter is filed by its supporting
        // line and each subject edge looks up its own line rather than trying every
        // pair: DF.2a's channel edges were 24 million COMP boundary edges against 16
        // million gate ones, 46 seconds of pairs that could not possibly overlap.
        EdgeOp::And => {
            let (Some(a), Some(b)) = (edge_srcs.first(), edge_srcs.get(1)) else {
                return Vec::new();
            };
            let index = line_index(b);
            a.iter()
                .flat_map(|e| {
                    index
                        .get(&line_key(e))
                        .into_iter()
                        .flatten()
                        .filter_map(move |f| edge_overlap(e, f))
                })
                .collect()
        }
        EdgeOp::Or => edge_srcs.iter().flat_map(|s| s.iter().copied()).collect(),
        EdgeOp::Not => {
            let (Some(a), Some(b)) = (edge_srcs.first(), edge_srcs.get(1)) else {
                return Vec::new();
            };
            let index = line_index(b);
            a.iter()
                .flat_map(|e| {
                    let on_line = index.get(&line_key(e)).map(Vec::as_slice).unwrap_or(&[]);
                    subtract_edges(e, on_line)
                })
                .collect()
        }
        EdgeOp::Interacting | EdgeOp::NotInteracting => {
            let (Some(a), Some(p)) = (edge_srcs.first(), poly_srcs.first()) else {
                return Vec::new();
            };
            let want = op == EdgeOp::Interacting;
            let boxes: Vec<_> = p.iter().map(poly_bbox).collect();
            let grid = BoxGrid::new(&boxes, (core.2 - core.0) / 20);
            let mut hits: Vec<u32> = Vec::new();
            a.iter()
                .filter(|e| {
                    let eb = edge_bbox(e);
                    grid.hits(eb, &mut hits);
                    hits.iter().any(|&i| {
                        boxes_meet(eb, boxes[i as usize]) && edge_meets_region(e, &p[i as usize])
                    }) == want
                })
                .copied()
                .collect()
        }
        EdgeOp::InteractingEdges | EdgeOp::NotInteractingEdges => {
            let (Some(a), Some(b)) = (edge_srcs.first(), edge_srcs.get(1)) else {
                return Vec::new();
            };
            let want = op == EdgeOp::InteractingEdges;
            let boxes: Vec<_> = b.iter().map(edge_bbox).collect();
            let grid = BoxGrid::new(&boxes, (core.2 - core.0) / 20);
            let mut hits: Vec<u32> = Vec::new();
            a.iter()
                .filter(|e| {
                    let eb = edge_bbox(e);
                    grid.hits(eb, &mut hits);
                    hits.iter().any(|&i| {
                        boxes_meet(eb, boxes[i as usize]) && edges_touch(e, &b[i as usize])
                    }) == want
                })
                .copied()
                .collect()
        }
        EdgeOp::InsidePart | EdgeOp::OutsidePart => {
            let (Some(a), Some(p)) = (edge_srcs.first(), poly_srcs.first()) else {
                return Vec::new();
            };
            let keep_inside = op == EdgeOp::InsidePart;
            let boxes: Vec<_> = p.iter().map(poly_bbox).collect();
            let contours: Vec<Vec<Edge>> = p.iter().map(region_edges).collect();
            let grid = BoxGrid::new(&boxes, (core.2 - core.0) / 20);
            let mut hits: Vec<u32> = Vec::new();
            let mut near: Vec<(&MergedPoly, &[Edge])> = Vec::new();
            a.iter()
                .flat_map(|e| {
                    let eb = edge_bbox(e);
                    grid.hits(eb, &mut hits);
                    near.clear();
                    near.extend(
                        hits.iter()
                            .filter(|&&i| boxes_meet(eb, boxes[i as usize]))
                            .map(|&i| (&p[i as usize], contours[i as usize].as_slice())),
                    );
                    edge_vs_polygons(e, &near, keep_inside)
                })
                .collect()
        }
        EdgeOp::Centers(abs, per_mille) => edge_srcs
            .first()
            .map(|a| {
                a.iter()
                    .filter_map(|e| {
                        let l = e.length();
                        if l <= 0.0 {
                            return None;
                        }
                        let want = (abs as f64).max(l * per_mille as f64 / 1000.0).min(l);
                        // Half the trim at each end, as a fraction of the edge's length.
                        let f = (1.0 - want / l) * 0.5;
                        let (dx, dy) = ((e.b.x - e.a.x) as f64, (e.b.y - e.a.y) as f64);
                        let pt = |t: f64| IntPoint {
                            x: (e.a.x as f64 + dx * t).round() as i32,
                            y: (e.a.y as f64 + dy * t).round() as i32,
                        };
                        let (a, b) = (pt(f), pt(1.0 - f));
                        // A short edge can round to nothing; an empty segment is not an
                        // edge and would make `interacting` meaningless.
                        (a != b).then_some(Edge { a, b })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        EdgeOp::WithLength(min, max) | EdgeOp::WithoutLength(min, max) => {
            let want = matches!(op, EdgeOp::WithLength(_, _));
            edge_srcs
                .first()
                .map(|a| {
                    a.iter()
                        .filter(|e| {
                            let l = e.length();
                            let in_range = min.is_none_or(|v| l >= v as f64 - 0.5)
                                && max.is_none_or(|v| l < v as f64 - 0.5);
                            in_range == want
                        })
                        .copied()
                        .collect()
                })
                .unwrap_or_default()
        }
        EdgeOp::WithAngle(min, max) | EdgeOp::WithoutAngle(min, max) => {
            let want = matches!(op, EdgeOp::WithAngle(_, _));
            edge_srcs
                .first()
                .map(|a| {
                    a.iter()
                        .filter(|e| {
                            let d = e.angle_deg();
                            let in_range = d >= min as f64 - 1e-9 && d <= max as f64 + 1e-9;
                            in_range == want
                        })
                        .copied()
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}

/// Bucket edges by the tile holding their midpoint, so an edge layer tiles the same way
/// a polygon layer does and a check can read one tile at a time.
/// Bucket edges by the tile their midpoint falls in.
///
/// This is a real limitation and it is worth stating, because it is invisible until a
/// rule leans on it. An edge op only ever sees what shares a tile with it, so a shape's
/// own boundary can be split: GF180's efuse has an anode whose right and bottom edges
/// meet at a corner 0.07 um from a tile boundary, their midpoints land either side of it,
/// and `not_interacting_edges` cannot see that they touch. The width/length partition
/// comes apart there, which is most of what EF.06/EF.08 over-report and EF.09 misses.
///
/// Bucketing by every tile an edge *reaches* fixes that and costs more than it buys:
/// measured, efuse finds 32 more logical violations and invents 66 more false positives,
/// because the cutting ops (`not`, `and`) then run with different context in each tile
/// and produce fragments that disagree - EF.22a and EF.22b go from near exact to
/// over-reporting by half. The real fix is for edge ops to compose against consistent
/// context rather than tile-local context, which is a larger piece of work than this
/// function.
/// Every tile the segment passes through, not only the one holding its midpoint.
///
/// A segment is *owned* by one tile so that an op reads it once and emits it once.  As
/// somebody else's filter it has to be visible wherever it reaches instead: a boundary
/// run long enough to cross a tile line is otherwise simply absent from the tiles it
/// crosses, and an `and` against it there quietly finds nothing to keep.
fn edge_tiles(e: &Edge, t: i64) -> Vec<(i32, i32)> {
    let tf = t as f64;
    let (x0, y0) = (e.a.x as f64, e.a.y as f64);
    let (x1, y1) = (e.b.x as f64, e.b.y as f64);
    let (mut cx, mut cy) = ((x0 / tf).floor() as i64, (y0 / tf).floor() as i64);
    let (ex, ey) = ((x1 / tf).floor() as i64, (y1 / tf).floor() as i64);
    let mut out = vec![(cx as i32, cy as i32)];
    let (dx, dy) = (x1 - x0, y1 - y0);
    let (sx, sy) = (
        if dx < 0.0 { -1i64 } else { 1 },
        if dy < 0.0 { -1i64 } else { 1 },
    );
    // How far along the segment the next tile line in each axis is, and how far apart
    // successive lines are - the usual grid walk, one axis stepped at a time.
    let line = |c: i64, s: i64| {
        if s > 0 {
            (c + 1) as f64 * tf
        } else {
            c as f64 * tf
        }
    };
    let mut mx = if dx != 0.0 {
        (line(cx, sx) - x0) / dx
    } else {
        f64::INFINITY
    };
    let mut my = if dy != 0.0 {
        (line(cy, sy) - y0) / dy
    } else {
        f64::INFINITY
    };
    let (px, py) = (
        if dx != 0.0 {
            tf / dx.abs()
        } else {
            f64::INFINITY
        },
        if dy != 0.0 {
            tf / dy.abs()
        } else {
            f64::INFINITY
        },
    );
    // Exactly one step per tile line crossed, so the walk cannot run away.
    for _ in 0..((ex - cx).abs() + (ey - cy).abs()) {
        if mx < my {
            cx += sx;
            mx += px;
        } else {
            cy += sy;
            my += py;
        }
        out.push((cx as i32, cy as i32));
    }
    out
}

/// The same edges filed under every tile they cross, for use as a filter.
fn spanning_index(owned: &EdgeTileMap, tile_dbu: i32) -> EdgeTileMap {
    let t = tile_dbu.max(1) as i64;
    let mut out: EdgeTileMap = HashMap::new();
    for v in owned.values() {
        for e in v {
            for k in edge_tiles(e, t) {
                out.entry(k).or_default().push(*e);
            }
        }
    }
    out
}

fn bucket_edges(edges: Vec<Edge>, tile_dbu: i32) -> EdgeTileMap {
    let t = tile_dbu.max(1) as i64;
    let mut out: EdgeTileMap = HashMap::new();
    for e in edges {
        let (mx, my) = e.midpoint();
        let key = (
            (mx as i64).div_euclid(t) as i32,
            (my as i64).div_euclid(t) as i32,
        );
        out.entry(key).or_default().push(e);
    }
    out
}

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
    p = clip_halfplane(
        &p,
        |x, _| x >= x0,
        |a, b| {
            let t = (x0 - a.0) / (b.0 - a.0);
            (x0, a.1 + t * (b.1 - a.1))
        },
    );
    if p.is_empty() {
        return p;
    }
    p = clip_halfplane(
        &p,
        |x, _| x <= x1,
        |a, b| {
            let t = (x1 - a.0) / (b.0 - a.0);
            (x1, a.1 + t * (b.1 - a.1))
        },
    );
    if p.is_empty() {
        return p;
    }
    p = clip_halfplane(
        &p,
        |_, y| y >= y0,
        |a, b| {
            let t = (y0 - a.1) / (b.1 - a.1);
            (a.0 + t * (b.0 - a.0), y0)
        },
    );
    if p.is_empty() {
        return p;
    }
    clip_halfplane(
        &p,
        |_, y| y <= y1,
        |a, b| {
            let t = (y1 - a.1) / (b.1 - a.1);
            (a.0 + t * (b.0 - a.0), y1)
        },
    )
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
    let (sx, sy) = m
        .outer
        .iter()
        .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x as f64, sy + p.y as f64));
    (sx / n, sy / n)
}

/// A connected region of a whole layer: total area (DBU²) and a marker point (DBU).
#[derive(Clone, Copy)]
pub struct Region {
    pub area_dbu: f64,
    /// Total boundary length (DBU), outer ring plus holes.  Tile-correct: only the
    /// polygon's own edges count, clipped to each tile core, so the cuts the tiling
    /// introduces along tile lines are never included and no edge is counted twice.
    pub perimeter_dbu: f64,
    pub marker: (f64, f64),
}

/// Length of the part of segment `a`-`b` lying inside the rectangle, by Liang-Barsky.
/// Used to attribute a polygon edge to the tile core it runs through.
fn clipped_seg_len(a: IntPoint, b: IntPoint, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let (px, py) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-px, a.x as f64 - x0),
        (px, x1 - a.x as f64),
        (-py, a.y as f64 - y0),
        (py, y1 - a.y as f64),
    ] {
        if p == 0.0 {
            // Parallel to this boundary and outside it: nothing survives.
            if q < 0.0 {
                return 0.0;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            if r > t1 {
                return 0.0;
            }
            t0 = t0.max(r);
        } else {
            if r < t0 {
                return 0.0;
            }
            t1 = t1.min(r);
        }
    }
    ((t1 - t0).max(0.0)) * (px * px + py * py).sqrt()
}

/// Boundary length of `m` lying inside the tile core, counting only the polygon's own
/// edges.  Summed over every core a region touches, this is the region's true perimeter:
/// the tiling's cut edges run along the core rectangle and are never polygon edges, and
/// each real edge falls in exactly one core.
pub fn poly_perimeter_in_core(m: &MergedPoly, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let mut total = 0.0;
    for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (a, b) = (ring[i], ring[if i + 1 == n { 0 } else { i + 1 }]);
            total += clipped_seg_len(a, b, x0, y0, x1, y1);
        }
    }
    total
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
    for h in &m.holes {
        scan(h);
    }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < ys.len() {
        let (lo, hi) = (ys[i].max(ylo), ys[i + 1].min(yhi));
        if hi > lo {
            out.push((lo, hi));
        }
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
    for h in &m.holes {
        scan(h);
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < xs.len() {
        let (lo, hi) = (xs[i].max(xlo), xs[i + 1].min(xhi));
        if hi > lo {
            out.push((lo, hi));
        }
        i += 2;
    }
    out
}

fn intervals_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.iter()
        .any(|&(a0, a1)| b.iter().any(|&(b0, b1)| a0 < b1 && b0 < a1))
}

/// Minimal union-find with path compression (shared by the region stitching here,
/// net extraction and the array-space clustering).
pub(crate) struct UnionFind {
    parent: Vec<usize>,
}
impl UnionFind {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }
    /// Extend to `n` elements, each new one its own set.
    pub(crate) fn grow_to(&mut self, n: usize) {
        let from = self.parent.len();
        self.parent.extend(from..n);
    }

    pub(crate) fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
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
        if ra != rb {
            self.parent[ra] = rb;
        }
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
    perimeter: f64,
    marker: (f64, f64),
    /// Sum and count of the outer-ring vertices this core owns, towards the region's
    /// vertex-average centroid.  Within its core a copy's outline is the region's, so the
    /// sum over a region's pieces is the average over the whole region's outer ring -
    /// what `representative_point` returns for a copy that holds the whole region, and
    /// which a copy holds only when the halo happens to reach that far.
    vsum: (f64, f64, usize),
    sides: Sides,
}

impl Piece {
    /// A piece for `poly` in the tile core `[cx0,cx1) × [cy0,cy1)`, or `None` if the
    /// region only reaches this tile's halo, not its core.
    fn of(poly: &MergedPoly, cx0: f64, cy0: f64, cx1: f64, cy1: f64) -> Option<Piece> {
        let area = clipped_area_dbu(poly, cx0, cy0, cx1, cy1);
        (area > 0.0).then(|| {
            // Strictly inside: a vertex on a tile line is as likely a cut as a corner -
            // a layer delivered as pieces has one at every crossing - and a cut is not
            // part of the region's outline.  A real corner on the line is lost with it,
            // which moves the average by less than the cuts would.  Hole vertices count
            // too: a hole a tile line runs through is a notch in the outer ring of
            // both pieces, so only the sum over every vertex is the same however the
            // region was cut - and a marker polygon written with cut lines, which is
            // what the references hold, averages its hole corners as well.
            let mut vsum = (0.0, 0.0, 0usize);
            for p in poly.outer.iter().chain(poly.holes.iter().flatten()) {
                let (x, y) = (p.x as f64, p.y as f64);
                if x > cx0 && x < cx1 && y > cy0 && y < cy1 {
                    vsum = (vsum.0 + x, vsum.1 + y, vsum.2 + 1);
                }
            }
            Piece {
                area,
                perimeter: poly_perimeter_in_core(poly, cx0, cy0, cx1, cy1),
                marker: representative_point(poly),
                vsum,
                sides: Sides::of(poly, cx0, cy0, cx1, cy1),
            }
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
    only: Option<&[(i32, i32)]>,
) {
    // With `only`, just the borders those tiles have - in both directions, since the
    // neighbour may have been filed in an earlier wave.  A pair linked twice is no harm.
    let mut across = |tile: (i32, i32), dx: i32, dy: i32| {
        let (Some(ids), Some(neigh)) = (
            tile_pieces.get(&tile),
            tile_pieces.get(&(tile.0 + dx, tile.1 + dy)),
        ) else {
            return;
        };
        // Only a piece that reaches the border can continue past it.  Nearly none do on
        // a via layer, and trying every pair of a tile and its neighbour regardless was
        // ten seconds of interval tests over 7.5 million vias that touch nothing.
        type Side = fn(&Sides) -> &[(f64, f64)];
        let (mine, theirs): (Side, Side) = match (dx, dy) {
            (1, 0) => (|s| &s.right, |s| &s.left),
            (-1, 0) => (|s| &s.left, |s| &s.right),
            (0, 1) => (|s| &s.top, |s| &s.bottom),
            _ => (|s| &s.bottom, |s| &s.top),
        };
        let at_border: Vec<usize> = ids
            .iter()
            .copied()
            .filter(|&a| !mine(sides[a]).is_empty())
            .collect();
        if at_border.is_empty() {
            return;
        }
        let facing: Vec<usize> = neigh
            .iter()
            .copied()
            .filter(|&b| !theirs(sides[b]).is_empty())
            .collect();
        for &a in &at_border {
            for &b in &facing {
                if intervals_overlap(mine(sides[a]), theirs(sides[b])) {
                    uf.union(a, b);
                }
            }
        }
    };
    match only {
        Some(tiles) => {
            for &tile in tiles {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    across(tile, dx, dy);
                }
            }
        }
        None => {
            for &tile in tile_pieces.keys() {
                across(tile, 1, 0);
                across(tile, 0, 1);
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
    stitch_from(tiles, tile_dbu, record_polys, None).0
}

/// The stitcher proper: regions grown outward from `seeds`, or the whole layer.
///
/// With `seeds`, only the regions that have a piece in one of those tiles are built.
/// The seed tiles' pieces come first; a piece whose metal reaches a tile border pulls
/// the tile beyond it into the next wave, and the waves stop when nothing crosses.  A
/// region is either taken whole or not at all, since every border it crosses is
/// followed, so what comes back is exactly the full stitch restricted to the regions
/// that touch a seed.  The tiles visited come back with it, so a caller can tell which
/// of the layer's other tiles it never looked at.
///
/// That is what a selection needs.  `overlapping [via4, mim_marker]` stitched all nine
/// million via4 copies to decide which regions touch a marker filed in 53 tiles, one
/// thread, fifty seconds a rule; it needs the regions in those 53 tiles and whatever
/// they connect to.
///
/// Each wave's pieces are built across tiles in parallel and then filed in tile order,
/// so the order pieces are numbered in - which piece a region takes its marker from
/// when two are the same size, and so the order violations come out - is fixed by the
/// seeds and the geometry, never by a `HashMap`'s iteration order.  Without seeds the
/// first wave is every tile, sorted, which is the order the full stitch always used.
/// One tile's pieces: the tile, and each piece with the index of the polygon it was cut
/// from in the tile's list.
type TilePieces = ((i32, i32), Vec<(usize, Piece)>);

fn stitch_from(
    tiles: &TileMap,
    tile_dbu: i32,
    record_polys: bool,
    seeds: Option<&[(i32, i32)]>,
) -> (LabeledRegions, HashSet<(i32, i32)>) {
    let t = tile_dbu as i64;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut piece_loc: Vec<((i32, i32), MergedPoly)> = Vec::new();
    let mut tile_pieces: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    // Every piece's source polygon, for the within-tile touch test below.
    let mut piece_poly: Vec<&MergedPoly> = Vec::new();
    let mut uf = UnionFind::new(0);
    let mut visited: HashSet<(i32, i32)> = HashSet::new();

    let mut wave: Vec<(i32, i32)> = match seeds {
        Some(s) => s
            .iter()
            .copied()
            .filter(|k| tiles.contains_key(k))
            .collect(),
        None => tiles.keys().copied().collect(),
    };
    wave.sort_unstable();
    wave.dedup();
    let trace = std::env::var("GDSCHECK_STITCH_TRACE").is_ok();
    let t_all = std::time::Instant::now();
    let (mut t_build, mut t_file, mut t_adj, mut t_touch) = (0.0, 0.0, 0.0, 0.0);
    while !wave.is_empty() {
        let t0 = std::time::Instant::now();
        // Build this wave's pieces in parallel, then file them in tile order.
        let built: Vec<TilePieces> = wave
            .par_iter()
            .map(|&(tx, ty)| {
                let polys = &tiles[&(tx, ty)];
                let cx0 = (tx as i64 * t) as f64;
                let cy0 = (ty as i64 * t) as f64;
                let cx1 = ((tx as i64 + 1) * t) as f64;
                let cy1 = ((ty as i64 + 1) * t) as f64;
                let ps = polys
                    .iter()
                    .enumerate()
                    .filter_map(|(i, poly)| Piece::of(poly, cx0, cy0, cx1, cy1).map(|p| (i, p)))
                    .collect();
                ((tx, ty), ps)
            })
            .collect();
        t_build += t0.elapsed().as_secs_f64();
        let t0 = std::time::Instant::now();
        let mut next: Vec<(i32, i32)> = Vec::new();
        let mut new_tiles: Vec<(i32, i32)> = Vec::new();
        for (tile, ps) in built {
            visited.insert(tile);
            let polys = &tiles[&tile];
            let ids = tile_pieces.entry(tile).or_default();
            for (i, piece) in ps {
                let id = pieces.len();
                if seeds.is_some() {
                    let (tx, ty) = tile;
                    let s = &piece.sides;
                    if !s.right.is_empty() {
                        next.push((tx + 1, ty));
                    }
                    if !s.left.is_empty() {
                        next.push((tx - 1, ty));
                    }
                    if !s.top.is_empty() {
                        next.push((tx, ty + 1));
                    }
                    if !s.bottom.is_empty() {
                        next.push((tx, ty - 1));
                    }
                }
                pieces.push(piece);
                piece_poly.push(&polys[i]);
                if record_polys {
                    piece_loc.push((tile, polys[i].clone()));
                }
                ids.push(id);
            }
            new_tiles.push(tile);
        }
        uf.grow_to(pieces.len());
        t_file += t0.elapsed().as_secs_f64();
        let t0 = std::time::Instant::now();
        let sides: Vec<&Sides> = pieces.iter().map(|p| &p.sides).collect();
        let only = seeds.map(|_| new_tiles.as_slice());
        link_adjacent_pieces(&mut uf, &tile_pieces, &sides, only);
        t_adj += t0.elapsed().as_secs_f64();
        let t0 = std::time::Instant::now();
        link_touching_pieces(&mut uf, &tile_pieces, &piece_poly, only);
        t_touch += t0.elapsed().as_secs_f64();
        next.sort_unstable();
        next.dedup();
        wave = next
            .into_iter()
            .filter(|k| !visited.contains(k) && tiles.contains_key(k))
            .collect();
    }

    // Compact each union-find root to a dense region id and aggregate area/marker
    // (marker from the largest piece).
    // Roots are piece ids, so a vector indexed by root does what a map would, for
    // millions of single-via regions without the hashing.
    let mut root_to_region: Vec<usize> = vec![usize::MAX; pieces.len()];
    let mut piece_region: Vec<usize> = vec![0; pieces.len()];
    let mut regions: Vec<Region> = Vec::new();
    let mut largest: Vec<(f64, usize)> = Vec::new();
    let mut vsum: Vec<(f64, f64, usize)> = Vec::new();
    for (id, p) in pieces.iter().enumerate() {
        let root = uf.find(id);
        if root_to_region[root] == usize::MAX {
            root_to_region[root] = regions.len();
            regions.push(Region {
                area_dbu: 0.0,
                perimeter_dbu: 0.0,
                marker: p.marker,
            });
            largest.push((0.0, id));
            vsum.push((0.0, 0.0, 0));
        }
        let region = root_to_region[root];
        piece_region[id] = region;
        regions[region].area_dbu += p.area;
        regions[region].perimeter_dbu += p.perimeter;
        let v = &mut vsum[region];
        *v = (v.0 + p.vsum.0, v.1 + p.vsum.1, v.2 + p.vsum.2);
        // Strictly greater, so the first piece of a tie keeps it - and the pieces now
        // arrive in a fixed order, so "first" means something.
        if p.area > largest[region].0 {
            largest[region] = (p.area, id);
            regions[region].marker = p.marker;
        }
    }
    // A region's marker is the vertex average of its whole outer ring where that lies
    // inside it, which is what a copy holding the whole region reports and does not
    // depend on how far any tile's copy reaches.  A ring's average sits in its hole, and
    // a region whose pieces do not agree on what is outer keeps the largest piece's
    // point, which is always inside.  The largest piece is asked first, which settles
    // every one-piece region at once; only a point outside it costs a scan of the
    // tile it fell in.
    for (region, v) in vsum.iter().enumerate() {
        if v.2 == 0 {
            continue;
        }
        let c = (v.0 / v.2 as f64, v.1 / v.2 as f64);
        let inside = point_in_merged(c.0, c.1, piece_poly[largest[region].1]) || {
            let tile = (
                (c.0 / t as f64).floor() as i32,
                (c.1 / t as f64).floor() as i32,
            );
            tile_pieces.get(&tile).is_some_and(|ids| {
                ids.iter().any(|&id| {
                    piece_region[id] == region && point_in_merged(c.0, c.1, piece_poly[id])
                })
            })
        };
        if inside {
            regions[region].marker = c;
        }
    }

    // Pieces were filed tile by tile, so each tile's run is contiguous.
    let mut by_tile: HashMap<(i32, i32), Vec<(MergedPoly, usize)>> = HashMap::new();
    let mut current: Option<(i32, i32)> = None;
    let mut bucket: Vec<(MergedPoly, usize)> = Vec::new();
    for (id, (tile, poly)) in piece_loc.into_iter().enumerate() {
        if current != Some(tile) {
            if let Some(prev) = current {
                by_tile.entry(prev).or_default().append(&mut bucket);
            }
            current = Some(tile);
        }
        bucket.push((poly, piece_region[id]));
    }
    if let Some(prev) = current {
        by_tile.entry(prev).or_default().append(&mut bucket);
    }

    if trace && pieces.len() > 100_000 {
        eprintln!(
            "stitch pieces={} regions={} tiles={} build={t_build:.1}s file={t_file:.1}s \
             adjacent={t_adj:.1}s touching={t_touch:.1}s total={:.1}s",
            pieces.len(),
            regions.len(),
            visited.len(),
            t_all.elapsed().as_secs_f64()
        );
    }
    (LabeledRegions { regions, by_tile }, visited)
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

/// A point guaranteed to lie *inside* `m`, for use as a region's marker.
///
/// The centroid is the natural choice and is wrong for a ring: a guard ring's centroid
/// sits in its hole. That is not cosmetic. A region's marker is what resolves the
/// region's net (`Partition::net_at` is a point lookup), so a ring whose centroid lands
/// on an island in its own hole takes the *island's* net — and a spacing rule between the
/// two then reads them as one net and stays silent. GF180's `DN.2b` lost four violations
/// exactly that way, a deep-well ring with a 1 µm island in its hole 3.9 µm clear.
///
/// The centroid is used when it is inside. Otherwise a vertical scanline is walked across
/// the bounding box and the midpoint of the widest covered span is taken, which always
/// lands in the interior for any non-degenerate polygon.
pub fn representative_point(m: &MergedPoly) -> (f64, f64) {
    let c = merged_centroid_dbu(m);
    if m.holes.is_empty() || point_in_merged(c.0, c.1, m) {
        return c;
    }
    let (x0, y0, x1, y1) = poly_bbox(m);
    let (x0, y0, x1, y1) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
    let mut best: Option<(f64, f64, f64)> = None; // (span, x, y)
    // Sample off the DBU grid so a scanline never runs along a vertical edge.
    const SAMPLES: usize = 17;
    for i in 1..SAMPLES {
        let xs = x0 + (x1 - x0) * i as f64 / SAMPLES as f64 + 0.5;
        for (a, b) in coverage_y(m, xs, y0, y1) {
            let span = b - a;
            if span > best.map_or(0.0, |(s, _, _)| s) {
                best = Some((span, xs, (a + b) * 0.5));
            }
        }
    }
    best.map(|(_, x, y)| (x, y)).unwrap_or(c)
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

/// Union pieces *within* one tile whose polygons touch.  The tile boolean leaves two
/// shapes meeting at a single vertex as two polygons — a corner touch is not an overlap,
/// so there is nothing for a union to dissolve — but KLayout treats them as one region,
/// and so must every region-level check here: a chamfered active touching its neighbour
/// at one corner is one shape of 0.38 µm², not two of 0.11 and 0.27 that both fail a
/// 0.2 µm² area rule.
///
/// Only same-tile pairs need this; pieces in different tiles are already linked by their
/// shared core edge.  The bbox test rejects almost every pair before the exact
/// segment-intersection test runs.
fn link_touching_pieces(
    uf: &mut UnionFind,
    tile_pieces: &HashMap<(i32, i32), Vec<usize>>,
    piece_poly: &[&MergedPoly],
    only: Option<&[(i32, i32)]>,
) {
    // The geometry is the work, and it is per tile, so the pairs are found across tiles
    // in parallel and the unions replayed after.
    let groups: Vec<&Vec<usize>> = match only {
        Some(tiles) => tiles.iter().filter_map(|k| tile_pieces.get(k)).collect(),
        None => tile_pieces.values().collect(),
    };
    let pairs: Vec<(usize, usize)> = groups
        .par_iter()
        .flat_map_iter(|ids| {
            let mut local = Vec::new();
            if ids.len() >= 2 {
                let boxes: Vec<(i32, i32, i32, i32)> =
                    ids.iter().map(|&i| poly_bbox(piece_poly[i])).collect();
                for a in 0..ids.len() {
                    for b in (a + 1)..ids.len() {
                        let (ax0, ay0, ax1, ay1) = boxes[a];
                        let (bx0, by0, bx1, by1) = boxes[b];
                        if ax1 < bx0 || bx1 < ax0 || ay1 < by0 || by1 < ay0 {
                            continue;
                        }
                        if polys_interact(piece_poly[ids[a]], piece_poly[ids[b]]) {
                            local.push((ids[a], ids[b]));
                        }
                    }
                }
            }
            local.into_iter()
        })
        .collect();
    for (a, b) in pairs {
        uf.union(a, b);
    }
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
    !av.overlay(&bv, OverlayRule::Intersect, FillRule::NonZero)
        .is_empty()
}

/// Whether two integer segments `p→p2` and `q→q2` intersect, endpoints and collinear
/// overlap included.  Exact: all arithmetic is i64 cross products on DBU coordinates.
fn segs_intersect(p: IntPoint, p2: IntPoint, q: IntPoint, q2: IntPoint) -> bool {
    let cross = |o: IntPoint, a: IntPoint, b: IntPoint| -> i64 {
        (a.x as i64 - o.x as i64) * (b.y as i64 - o.y as i64)
            - (a.y as i64 - o.y as i64) * (b.x as i64 - o.x as i64)
    };
    // A point of a degenerate/collinear segment lies on the other iff it is within the
    // other's bounding box (collinearity already established by a zero cross product).
    let on = |o: IntPoint, a: IntPoint, b: IntPoint| -> bool {
        b.x.min(o.x) <= a.x && a.x <= b.x.max(o.x) && b.y.min(o.y) <= a.y && a.y <= b.y.max(o.y)
    };
    let (d1, d2) = (cross(p, p2, q), cross(p, p2, q2));
    let (d3, d4) = (cross(q, q2, p), cross(q, q2, p2));
    if ((d1 > 0) != (d2 > 0) || (d1 < 0) != (d2 < 0))
        && ((d3 > 0) != (d4 > 0) || (d3 < 0) != (d4 < 0))
    {
        return true;
    }
    (d1 == 0 && on(p, q, p2))
        || (d2 == 0 && on(p, q2, p2))
        || (d3 == 0 && on(q, p, q2))
        || (d4 == 0 && on(q, p2, q2))
}

/// Every edge of a region's contours (outer ring and holes), as point pairs.
fn poly_edges(m: &MergedPoly) -> impl Iterator<Item = (IntPoint, IntPoint)> + '_ {
    std::iter::once(&m.outer)
        .chain(m.holes.iter())
        .flat_map(|ring| {
            ring.iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
                .map(|(a, b)| (*a, *b))
        })
}

/// Whether two merged polygons overlap **or merely touch** — the KLayout `interacting`
/// relation.  [`polys_overlap`] misses zero-area contact (coincident edges, a shared
/// vertex) because `Intersect` yields an empty region for it, so test the boundaries
/// directly as well.
fn polys_interact(a: &MergedPoly, b: &MergedPoly) -> bool {
    let (ax0, ay0, ax1, ay1) = poly_bbox(a);
    let (bx0, by0, bx1, by1) = poly_bbox(b);
    if ax1 < bx0 || bx1 < ax0 || ay1 < by0 || by1 < ay0 {
        return false;
    }
    if polys_overlap(a, b) {
        return true;
    }
    let bedges: Vec<(IntPoint, IntPoint)> = poly_edges(b).collect();
    poly_edges(a).any(|(p, p2)| bedges.iter().any(|&(q, q2)| segs_intersect(p, p2, q, q2)))
}

/// Whether `a` lies entirely within the area covered by `others` — i.e. `a` minus their
/// union is empty.  Used by the `inside` selector, which (unlike the others) must hold
/// for *every* piece of a region rather than any one of them.
fn poly_within(a: &MergedPoly, others: &[MergedPoly]) -> bool {
    if others.is_empty() {
        return false;
    }
    let av = vec![merged_to_shape(a)];
    let bv: Vec<Vec<Vec<[f64; 2]>>> = others.iter().map(merged_to_shape).collect();
    let clip = bv.simplify_shape(FillRule::NonZero);
    if clip.is_empty() {
        return false;
    }
    av.overlay(&clip, OverlayRule::Difference, FillRule::NonZero)
        .is_empty()
}

/// Build a selection virtual's tiles: keep whole *regions* of the candidate layer
/// (`cand`) by how they relate to the filter layer (`filt`), preserving the candidate's
/// original tiling.  Region membership is recovered with [`stitch_labeled`], so a region
/// spanning tiles is kept or dropped as a unit (a per-tile test would split a region that
/// only meets the filter in one of its tiles).  `keep` selects whether matching regions
/// are the ones retained or the ones discarded.
///
/// All predicates are evaluated tile-locally: a candidate piece is tested only against
/// the filter polygons stored under the *same* tile key.  That is exact for
/// [`SelectionKind::Overlaps`]/[`Touches`] (a piece can only meet the filter where they
/// share a tile) and for [`Covers`] as long as the filter region assembles within one
/// tile bucket.  [`SelectionKind::Inside`] additionally needs the covering filter
/// geometry present in each of the candidate's tiles, which the source halo provides.
/// Put a region-filtered layer back on the tiling contract the drawn layers keep: every
/// tile carries not only its own geometry but everything within `halo` of it, so a check
/// that measures *between* shapes sees both sides of a tile boundary.
///
/// Stitching deliberately drops those copies — it hands back one piece per region per
/// tile, owned by the tile its core falls in — so any layer built through it comes back
/// halo-less. That is invisible until a spacing rule runs on one: GF180's PRES.9b puts
/// two RES_MK markers 19.995 um apart across a full 20 um tile, and without this the two
/// never appear in the same tile and the rule reads clean.
///
/// Applied to the region filters only, not to the selectors, though those lose the copies
/// the same way. Restoring a selector's halo turned out to reach further than the halo
/// its *sources* were tiled with, so a boolean built on it mixes two reaches and subtracts
/// geometry that is only present on one side — measured as four extra DF.2a markers on
/// comp, where a gate that tile-locally is not there stops being subtracted from its COMP.
/// A region filter's output feeds a check directly, so it has no such downstream.
/// Cut every tile's polygons to the tile's core: what is inside stays as it is, what is
/// outside goes, and what straddles the border is clipped.  The result owns nothing
/// twice across tiles, which is what [`rebroadcast_halo`] takes.
fn clip_to_cores(tiles: TileMap, tile_dbu: i32) -> TileMap {
    let t = tile_dbu as i64;
    tiles
        .into_par_iter()
        .filter_map(|((tx, ty), polys)| {
            let (x0, y0) = (tx as i64 * t, ty as i64 * t);
            let (x1, y1) = ((tx as i64 + 1) * t, (ty as i64 + 1) * t);
            let inside = clip_to_box(polys, x0, y0, x1, y1);
            (!inside.is_empty()).then_some(((tx, ty), inside))
        })
        .collect()
}

/// The part of `polys` inside the box: what lies within stays as it is, what lies
/// outside goes, and what straddles the border is cut.  Holes survive the cut.
fn clip_to_box(polys: Vec<MergedPoly>, x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<MergedPoly> {
    let (mut inside, mut straddle): (Vec<MergedPoly>, Vec<MergedPoly>) = (Vec::new(), Vec::new());
    for p in polys {
        let (bx0, by0, bx1, by1) = poly_bbox(&p);
        let (bx0, by0, bx1, by1) = (bx0 as i64, by0 as i64, bx1 as i64, by1 as i64);
        if bx1 <= x0 || bx0 >= x1 || by1 <= y0 || by0 >= y1 {
            continue;
        }
        if bx0 >= x0 && bx1 <= x1 && by0 >= y0 && by1 <= y1 {
            inside.push(p);
        } else {
            straddle.push(p);
        }
    }
    if !straddle.is_empty() {
        let rect = vec![vec![vec![
            [x0 as f64, y0 as f64],
            [x1 as f64, y0 as f64],
            [x1 as f64, y1 as f64],
            [x0 as f64, y1 as f64],
        ]]];
        let cut = tile_shapes(&straddle)
            .simplify_shape(FillRule::NonZero)
            .overlay(&rect, OverlayRule::Intersect, FillRule::NonZero);
        inside.extend(shapes_to_merged(cut));
    }
    inside
}

/// The polygons of each tile that own area in the tile's core - what a stitch files
/// for the tile, without the stitch.
fn core_owned(tiles: &TileMap, tile_dbu: i32) -> TileMap {
    let t = tile_dbu as i64;
    tiles
        .par_iter()
        .filter_map(|(&(tx, ty), polys)| {
            let cx0 = (tx as i64 * t) as f64;
            let cy0 = (ty as i64 * t) as f64;
            let cx1 = ((tx as i64 + 1) * t) as f64;
            let cy1 = ((ty as i64 + 1) * t) as f64;
            let own: Vec<MergedPoly> = polys
                .iter()
                .filter(|p| clipped_area_dbu(p, cx0, cy0, cx1, cy1) > 0.0)
                .cloned()
                .collect();
            (!own.is_empty()).then_some(((tx, ty), own))
        })
        .collect()
}

fn rebroadcast_halo(core_owned: TileMap, tile_dbu: i32, halo_dbu: i32, clip: bool) -> TileMap {
    if halo_dbu <= 0 {
        return core_owned;
    }
    let (tile, halo) = (tile_dbu.max(1) as i64, halo_dbu as i64);
    // A tile's copy of a polygon is exact within the tile's core plus the halo the layer
    // was built at, which is at least this one, and past that it is whatever the tile
    // happened to compute - a subtrahend that stopped short leaves material that does
    // not exist.  Copying a polygon whole carried that out to tiles that read it as
    // their own: DF.14 flagged a 1.5 µm scrap of `nactive` inside an n-well, made in a
    // tile twenty microns away where the well's copy had run out.  Each owner's copy
    // is cut to the owner's exact zone first; every point another tile is owed lies in
    // the core of the tile that owns it, so nothing that matters is lost.
    let owned: Vec<Vec<MergedPoly>> = core_owned
        .into_par_iter()
        .map(|((tx, ty), polys)| {
            if !clip {
                return polys;
            }
            let (x0, y0) = (tx as i64 * tile - halo, ty as i64 * tile - halo);
            let (x1, y1) = ((tx as i64 + 1) * tile + halo, (ty as i64 + 1) * tile + halo);
            clip_to_box(polys, x0, y0, x1, y1)
        })
        .collect();
    let mut buckets: HashMap<(i32, i32), Vec<MergedPoly>> = HashMap::new();
    for polys in owned {
        for poly in polys {
            let (x0, y0, x1, y1) = outer_bbox(&poly.outer);
            let tx0 = (x0 - halo).div_euclid(tile) as i32;
            let tx1 = (x1 + halo).div_euclid(tile) as i32;
            let ty0 = (y0 - halo).div_euclid(tile) as i32;
            let ty1 = (y1 + halo).div_euclid(tile) as i32;
            for tx in tx0..=tx1 {
                for ty in ty0..=ty1 {
                    buckets.entry((tx, ty)).or_default().push(poly.clone());
                }
            }
        }
    }
    // Union per tile, not just collect: stitching hands back one piece per tile the
    // region covers, and a tile that now sees several of them must see one shape, or a
    // spacing scan pairs a region's own pieces against each other and counts a violation
    // once per piece that reaches the tile.
    buckets
        .into_par_iter()
        .map(|(key, polys)| {
            let merged = compose_tile(VirtualOp::Union, &[&polys]);
            (key, merged)
        })
        .collect()
}

/// Keep whole regions of `cand` whose measured quantity falls in the filter's range.
/// Each stitched region of `cand` as its bounding box (KLayout `.extents`).
///
/// The box goes into every tile it reaches, whole and unclipped, which is how a tiled
/// layer holds a polygon: the tile that owns a shape's core is the one that sees all of
/// it, and the copies in the neighbours are what lets a boolean there be exact.
fn build_extents_tiles(cand: &TileMap, tile_dbu: i32) -> TileMap {
    let labeled = stitch_labeled(cand, tile_dbu);
    let mut bb: Vec<Option<(i64, i64, i64, i64)>> = vec![None; labeled.regions.len()];
    for polys in labeled.by_tile.values() {
        for (poly, rid) in polys {
            let (x0, y0, x1, y1) = outer_bbox(&poly.outer);
            bb[*rid] = Some(match bb[*rid] {
                None => (x0, y0, x1, y1),
                Some(b) => (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1)),
            });
        }
    }
    let t = tile_dbu as i64;
    let mut out: TileMap = HashMap::new();
    for b in bb.into_iter().flatten() {
        let (x0, y0, x1, y1) = b;
        let poly = MergedPoly {
            outer: vec![
                IntPoint::new(x0 as i32, y0 as i32),
                IntPoint::new(x1 as i32, y0 as i32),
                IntPoint::new(x1 as i32, y1 as i32),
                IntPoint::new(x0 as i32, y1 as i32),
            ],
            holes: Vec::new(),
        };
        for tx in x0.div_euclid(t)..=x1.div_euclid(t) {
            for ty in y0.div_euclid(t)..=y1.div_euclid(t) {
                out.entry((tx as i32, ty as i32))
                    .or_default()
                    .push(poly.clone());
            }
        }
    }
    out
}

/// `holes` and `with_holes`, read on stitched regions.  A ring's hole exists only once
/// the ring is whole, and a ring can be any size - a guard ring round a pad frame spans
/// the chip - so reading it per tile meant every tile had to hold the whole ring, at a
/// halo that multiplied a dense layer by hundreds (COMP at the 200 µm a guard-ring rule
/// declared was 441 copies of 2.6 million shapes, and the run died there).  Stitched,
/// a region within one tile is read as it is and one spanning tiles is its pieces
/// unioned, the seams the cut left vanishing in the union; the source needs no halo.
///
/// `keep_regions` selects `with_holes`, the regions that have a hole, filed as their
/// pieces were.  Otherwise the holes themselves, as filled polygons, filed by core: a
/// hole goes whole into every core it spans while those copies are cheap, and one
/// spanning the chip - the pad frame's interior, ten thousand tiles by a thousand
/// vertices - is cut along the grid, because that polygon copied whole into every
/// tile it covers is what an OOM looks like.  Whole wherever affordable, since a
/// reader like `covering` wants the copy to extend over whatever it is asked about;
/// what a reader gets is assembled from the pieces by the caller, per its contract.
fn build_holes_tiles(cand: &TileMap, tile_dbu: i32, keep_regions: bool) -> TileMap {
    let labeled = stitch_labeled(cand, tile_dbu);
    let mut pieces: Vec<Vec<((i32, i32), MergedPoly)>> =
        (0..labeled.regions.len()).map(|_| Vec::new()).collect();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            pieces[rid].push((tile, poly));
        }
    }
    let per_region: Vec<(Vec<((i32, i32), MergedPoly)>, Vec<MergedPoly>)> = pieces
        .into_par_iter()
        .map(|ps| {
            let holes: Vec<MergedPoly> = if ps.len() == 1 {
                holes_of(&ps[0].1)
            } else {
                let parts: Vec<MergedPoly> = ps.iter().map(|(_, p)| p.clone()).collect();
                shapes_to_merged(tile_shapes(&parts).simplify_shape(FillRule::NonZero))
                    .iter()
                    .flat_map(holes_of)
                    .collect()
            };
            (ps, holes)
        })
        .collect();
    let mut out: TileMap = HashMap::new();
    let mut all_holes: Vec<MergedPoly> = Vec::new();
    let mut n_regions = 0usize;
    for (ps, holes) in per_region {
        if keep_regions {
            if !holes.is_empty() {
                n_regions += 1;
                for (tile, poly) in ps {
                    out.entry(tile).or_default().push(poly);
                }
            }
            continue;
        }
        all_holes.extend(holes);
    }
    let n_holes = all_holes.len();
    let mut n_large = 0;
    if !keep_regions {
        (out, n_large) = file_by_core(all_holes, tile_dbu);
    }
    if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
        let copies: usize = out.values().map(|v| v.len()).sum();
        if keep_regions {
            eprintln!("holes: {n_regions} regions with a hole kept, {copies} pieces");
        } else {
            eprintln!(
                "holes: {n_holes} holes, {n_large} too large to copy whole and cut along \
                 the grid, {copies} pieces over {} cores",
                out.len()
            );
        }
    }
    out
}

/// Vertices a polygon may cost as whole copies - its vertex count times the cores it
/// spans - before it is cut along the grid instead.  Two million is sixteen megabytes
/// of points; a device ring's hole is a few hundred, a pad frame's tens of millions.
const WHOLE_COPY_BUDGET: i64 = 2_000_000;

/// File global polygons by core: whole into every core a polygon spans while that is
/// within [`WHOLE_COPY_BUDGET`], cut along the grid otherwise.  Returns the map and
/// how many were cut.
fn file_by_core(polys: Vec<MergedPoly>, tile_dbu: i32) -> (TileMap, usize) {
    let t = tile_dbu as i64;
    let mut out: TileMap = HashMap::new();
    let mut large: Vec<MergedPoly> = Vec::new();
    for poly in polys {
        let (x0, y0, x1, y1) = outer_bbox(&poly.outer);
        let (tx0, tx1) = (x0.div_euclid(t), (x1 - 1).max(x0).div_euclid(t));
        let (ty0, ty1) = (y0.div_euclid(t), (y1 - 1).max(y0).div_euclid(t));
        let cores = (tx1 - tx0 + 1) * (ty1 - ty0 + 1);
        if cores * poly.outer.len() as i64 > WHOLE_COPY_BUDGET {
            large.push(poly);
            continue;
        }
        for tx in tx0..=tx1 {
            for ty in ty0..=ty1 {
                out.entry((tx as i32, ty as i32))
                    .or_default()
                    .push(poly.clone());
            }
        }
    }
    let n_large = large.len();
    if !large.is_empty() {
        for (tile, mut ps) in cut_along_grid(&large, tile_dbu) {
            out.entry(tile).or_default().append(&mut ps);
        }
    }
    (out, n_large)
}

/// Whether an erosion by `r` is read on stitched regions rather than per tile.  Per
/// tile, `shrink` and `open` charge their source a halo of one and two radii, and the
/// copies grow with the square of that: DF.2b's 50 µm opening of COMP on a 20 µm tile
/// is 121 copies of 2.6 million shapes.  On stitched regions the source needs no halo:
/// a region narrower than `2r` in either direction is gone entirely, and the few that
/// are not are unioned from their pieces and eroded whole.  Small radii stay per tile,
/// where a dense layer is cheaper in bulk than region by region.
pub fn erosion_on_regions(r: i32, tile_dbu: i32) -> bool {
    r * 4 >= tile_dbu
}

/// Which erosion [`build_erosion_tiles`] performs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Erosion {
    Open,
    Shrink,
    ShrinkX,
    ShrinkY,
}

/// An erosion by a radius past [`erosion_on_regions`], on stitched regions.  A region
/// narrower than `2r` across the eroded axis has no point `r` from its boundary that
/// way and is gone; that is nearly every region of a dense layer.  What survives is
/// eroded core by core: the pieces of the cores within reach of a core - `r` for an
/// erosion, `2r` for an opening, whose dilation needs eroded geometry that far - are
/// unioned, eroded, and cut back to the core.  Never the whole region at once: a
/// power mesh is one region spanning the chip with millions of vertices, and eroding
/// it in one piece is the memory the per-tile path was avoiding.
fn build_erosion_tiles(cand: &TileMap, tile_dbu: i32, r: i32, how: Erosion) -> TileMap {
    let labeled = stitch_labeled(cand, tile_dbu);
    let mut pieces: Vec<Vec<((i32, i32), MergedPoly)>> =
        (0..labeled.regions.len()).map(|_| Vec::new()).collect();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            pieces[rid].push((tile, poly));
        }
    }
    let n_regions = pieces.len();
    let t = tile_dbu as i64;
    let survivors: Vec<Vec<((i32, i32), MergedPoly)>> = pieces
        .into_iter()
        .filter(|ps| {
            let (x0, y0, x1, y1) = ps
                .iter()
                .map(|(_, p)| outer_bbox(&p.outer))
                .fold((i64::MAX, i64::MAX, i64::MIN, i64::MIN), |a, b| {
                    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
                });
            let (narrow_x, narrow_y) = (x1 - x0 < 2 * r as i64, y1 - y0 < 2 * r as i64);
            !match how {
                Erosion::Open | Erosion::Shrink => narrow_x || narrow_y,
                Erosion::ShrinkX => narrow_x,
                Erosion::ShrinkY => narrow_y,
            }
        })
        .collect();
    let n_survivors = survivors.len();
    let reach = match how {
        Erosion::Open => 2 * r as i64,
        _ => r as i64,
    };
    let k = ((reach + t - 1) / t) as i32;
    let core_box = |c: (i32, i32)| {
        (
            c.0 as i64 * t,
            c.1 as i64 * t,
            (c.0 as i64 + 1) * t,
            (c.1 as i64 + 1) * t,
        )
    };
    let parts: Vec<((i32, i32), Vec<MergedPoly>)> = survivors
        .into_par_iter()
        .flat_map_iter(|ps| {
            // A copy is exact within its own core, so each is cut to it before the
            // block is unioned; a whole copy could carry what its tile computed past
            // its zone.
            let mut by_core: HashMap<(i32, i32), Vec<MergedPoly>> = HashMap::new();
            for (core, poly) in ps {
                let (x0, y0, x1, y1) = core_box(core);
                by_core
                    .entry(core)
                    .or_default()
                    .extend(clip_to_box(vec![poly], x0, y0, x1, y1));
            }
            let cores: Vec<(i32, i32)> = by_core.keys().copied().collect();
            cores
                .into_iter()
                .filter_map(|c| {
                    let mut block: Vec<MergedPoly> = Vec::new();
                    for dx in -k..=k {
                        for dy in -k..=k {
                            if let Some(v) = by_core.get(&(c.0 + dx, c.1 + dy)) {
                                block.extend(v.iter().cloned());
                            }
                        }
                    }
                    let eroded = match how {
                        Erosion::Open => opening(&block, r as f64),
                        Erosion::Shrink => shrink(&block, r as f64),
                        Erosion::ShrinkX => shrink_x(&block, r as f64),
                        Erosion::ShrinkY => shrink_y(&block, r as f64),
                    };
                    let (x0, y0, x1, y1) = core_box(c);
                    let inside = clip_to_box(eroded, x0, y0, x1, y1);
                    (!inside.is_empty()).then_some((c, inside))
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect();
    let mut out: TileMap = HashMap::new();
    for (c, ps) in parts {
        out.entry(c).or_default().extend(ps);
    }
    if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
        let copies: usize = out.values().map(|v| v.len()).sum();
        eprintln!(
            "erosion {how:?} r={r}dbu on regions: {n_regions} regions, {n_survivors} wide enough, \
             {copies} pieces over {} cores",
            out.len()
        );
    }
    out
}

/// Cut `polys` along the tile grid: the piece of each polygon inside every core it
/// spans, keyed by core.  One overlay per grid phase rather than one per core, so a
/// polygon covering ten thousand tiles costs four sweeps and not ten thousand clips.
/// Cores of one phase - same parity of both indices - never touch, so the overlay hands
/// their pieces back apart, and each piece lies in exactly one core.  Polygons that
/// overlap each other come back merged, as a region layer reads them anyway.
fn cut_along_grid(polys: &[MergedPoly], tile_dbu: i32) -> TileMap {
    let t = tile_dbu as i64;
    let mut out: TileMap = HashMap::new();
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in polys {
        let b = outer_bbox(&p.outer);
        x0 = x0.min(b.0);
        y0 = y0.min(b.1);
        x1 = x1.max(b.2);
        y1 = y1.max(b.3);
    }
    if x1 <= x0 || y1 <= y0 {
        return out;
    }
    let (tx0, tx1) = (x0.div_euclid(t), (x1 - 1).div_euclid(t));
    let (ty0, ty1) = (y0.div_euclid(t), (y1 - 1).div_euclid(t));
    let subject = tile_shapes(polys).simplify_shape(FillRule::NonZero);
    let phases: Vec<TileMap> = (0..4i64)
        .into_par_iter()
        .map(|phase| {
            let mut rects: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
            for tx in tx0..=tx1 {
                if tx.rem_euclid(2) != (phase & 1) {
                    continue;
                }
                for ty in ty0..=ty1 {
                    if ty.rem_euclid(2) != (phase >> 1) {
                        continue;
                    }
                    let (cx0, cy0) = ((tx * t) as f64, (ty * t) as f64);
                    let (cx1, cy1) = (((tx + 1) * t) as f64, ((ty + 1) * t) as f64);
                    rects.push(vec![vec![[cx0, cy0], [cx1, cy0], [cx1, cy1], [cx0, cy1]]]);
                }
            }
            let mut m: TileMap = HashMap::new();
            if rects.is_empty() {
                return m;
            }
            let cut = subject.overlay(&rects, OverlayRule::Intersect, FillRule::NonZero);
            for piece in shapes_to_merged(cut) {
                let b = outer_bbox(&piece.outer);
                let (cx, cy) = ((b.0 + b.2) / 2, (b.1 + b.3) / 2);
                m.entry((cx.div_euclid(t) as i32, cy.div_euclid(t) as i32))
                    .or_default()
                    .push(piece);
            }
            m
        })
        .collect();
    for m in phases {
        for (tile, mut ps) in m {
            out.entry(tile).or_default().append(&mut ps);
        }
    }
    out
}

/// One copy per tile of a layer held as core pieces: the pieces of every core within
/// `halo_dbu` of the tile, and never less than the tile's neighbours, unioned into
/// whole polygons.  Exact within the tile's zone and truncated past it, which is the
/// contract a drawn layer's copies keep and what a reader of polygons is written for;
/// a hole filed whole comes out whole.
fn assemble_zone_copies(pieces: TileMap, tile_dbu: i32, halo_dbu: i32) -> TileMap {
    let (h, t) = (halo_dbu.max(0) as i64, tile_dbu as i64);
    let r = (((h + t - 1) / t) as i32).max(1);
    let mut targets: HashSet<(i32, i32)> = HashSet::new();
    for &(tx, ty) in pieces.keys() {
        for dx in -r..=r {
            for dy in -r..=r {
                targets.insert((tx + dx, ty + dy));
            }
        }
    }
    targets
        .into_par_iter()
        .filter_map(|(tx, ty)| {
            let mut block: Vec<MergedPoly> = Vec::new();
            for dx in -r..=r {
                for dy in -r..=r {
                    if let Some(ps) = pieces.get(&(tx + dx, ty + dy)) {
                        block.extend(ps.iter().cloned());
                    }
                }
            }
            if block.is_empty() {
                return None;
            }
            let copy = if block.len() == 1 {
                block
            } else {
                shapes_to_merged(tile_shapes(&block).simplify_shape(FillRule::NonZero))
            };
            Some(((tx, ty), copy))
        })
        .collect()
}

/// A region's holes as filled polygons.  Hole contours are stored clockwise (see
/// [`MergedPoly`]); reversed to CCW so downstream ops see solid regions.
fn holes_of(m: &MergedPoly) -> Vec<MergedPoly> {
    m.holes
        .iter()
        .map(|h| {
            let mut outer = h.clone();
            outer.reverse();
            MergedPoly {
                outer,
                holes: Vec::new(),
            }
        })
        .collect()
}

/// Whether a derived layer is empty as soon as its `i`-th source is: a selection, a
/// filter, a size or a measurement with nothing to work on, a difference with no
/// subject, an intersection missing any of its terms.  A union needs every source.
fn empty_when_source_empty(op: VirtualOp, i: usize) -> bool {
    match op {
        VirtualOp::Union | VirtualOp::WithText => false,
        VirtualOp::Intersection => true,
        _ => i == 0,
    }
}

fn build_region_filter_tiles(cand: &TileMap, tile_dbu: i32, f: RegionFilter) -> TileMap {
    let labeled = stitch_labeled(cand, tile_dbu);
    // A region's bounding box is the union of its pieces' — the pieces tile its core, so
    // nothing of the region falls outside them.
    let mut bb: Vec<Option<(i64, i64, i64, i64)>> = vec![None; labeled.regions.len()];
    if !matches!(f, RegionFilter::Area(_, _)) {
        for polys in labeled.by_tile.values() {
            for (poly, rid) in polys {
                let (x0, y0, x1, y1) = outer_bbox(&poly.outer);
                bb[*rid] = Some(match bb[*rid] {
                    None => (x0, y0, x1, y1),
                    Some(b) => (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1)),
                });
            }
        }
    }
    let keep: Vec<bool> = (0..labeled.regions.len())
        .map(|rid| match f {
            RegionFilter::Area(lo, hi) => {
                let a = labeled.regions[rid].area_dbu;
                lo.is_none_or(|v| a >= v as f64) && hi.is_none_or(|v| a < v as f64)
            }
            RegionFilter::ShortSide(lo, hi) | RegionFilter::LongSide(lo, hi) => {
                let Some((x0, y0, x1, y1)) = bb[rid] else {
                    return false;
                };
                let (w, h) = (x1 - x0, y1 - y0);
                let side = match f {
                    RegionFilter::LongSide(_, _) => w.max(h),
                    _ => w.min(h),
                };
                side_in_range(side, lo, hi)
            }
        })
        .collect();

    let mut out: TileMap = HashMap::new();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            if keep[rid] {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

/// True if the segment `e` meets the region `poly` — either endpoint inside or on it, or
/// the segment crossing its boundary.  A segment running clear past a region touches
/// neither, which is what makes this a selection rather than a proximity test.
fn poly_meets_edge(poly: &MergedPoly, e: &Edge) -> bool {
    let (px, py) = (e.a.x as f64, e.a.y as f64);
    let (qx, qy) = (e.b.x as f64, e.b.y as f64);
    if point_in_merged(px, py, poly) || point_in_merged(qx, qy, poly) {
        return true;
    }
    let cross = |ox: f64, oy: f64, ax: f64, ay: f64, bx: f64, by: f64| {
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    // Coordinates are integer DBU, so a cross product this small is zero.
    const EPS: f64 = 1e-9;
    let within = |ax: f64, ay: f64, bx: f64, by: f64, x: f64, y: f64| {
        x >= ax.min(bx) - EPS
            && x <= ax.max(bx) + EPS
            && y >= ay.min(by) - EPS
            && y <= ay.max(by) + EPS
    };
    let ring = |pts: &[IntPoint]| {
        for i in 0..pts.len() {
            let (ax, ay) = (pts[i].x as f64, pts[i].y as f64);
            let j = (i + 1) % pts.len();
            let (bx, by) = (pts[j].x as f64, pts[j].y as f64);
            let d1 = cross(ax, ay, bx, by, px, py);
            let d2 = cross(ax, ay, bx, by, qx, qy);
            let d3 = cross(px, py, qx, qy, ax, ay);
            let d4 = cross(px, py, qx, qy, bx, by);
            if ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0)) {
                return true;
            }
            // Zero-area contact counts, which is what makes this `interacting` rather
            // than `overlapping` — and it is the case that matters, because an edge
            // derived *from* a region lies exactly on that region's own boundary.
            if (d1.abs() <= EPS && within(ax, ay, bx, by, px, py))
                || (d2.abs() <= EPS && within(ax, ay, bx, by, qx, qy))
                || (d3.abs() <= EPS && within(px, py, qx, qy, ax, ay))
                || (d4.abs() <= EPS && within(px, py, qx, qy, bx, by))
            {
                return true;
            }
        }
        false
    };
    if ring(&poly.outer) {
        return true;
    }
    poly.holes.iter().any(|h| ring(h))
}

/// Keep whole regions of `cand` that any segment of the edge layer `filt` meets.
fn build_edge_selection_tiles(
    cand: &TileMap,
    filt: &EdgeTileMap,
    keep: bool,
    tile_dbu: i32,
) -> TileMap {
    if filt.values().all(|v| v.is_empty()) {
        return if keep { TileMap::new() } else { cand.clone() };
    }
    let labeled = stitch_labeled(cand, tile_dbu);
    let mut matches = vec![false; labeled.regions.len()];
    for (tile, polys) in &labeled.by_tile {
        let edges = filt.get(tile).map(Vec::as_slice).unwrap_or(&[]);
        for (poly, rid) in polys {
            if !matches[*rid] && edges.iter().any(|e| poly_meets_edge(poly, e)) {
                matches[*rid] = true;
            }
        }
    }
    let mut out: TileMap = HashMap::new();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            if matches[rid] == keep {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

/// Does one candidate piece meet one filter piece, under `kind`?  `Inside` is absent:
/// it reduces with AND over the candidate's pieces and is handled inline.
fn piece_meets(kind: SelectionKind, poly: &MergedPoly, fp: &MergedPoly) -> bool {
    match kind {
        SelectionKind::Overlaps => polys_overlap(poly, fp),
        SelectionKind::Touches => polys_interact(poly, fp),
        SelectionKind::Covers => poly_within(fp, std::slice::from_ref(poly)),
        SelectionKind::Inside => unreachable!("Inside reduces with AND, not per pair"),
    }
}

/// The counted form of [`build_selection_tiles`]: keep a candidate region when the number
/// of *distinct* filter regions it meets falls in `[min, max]`.
///
/// Counting is why the filter has to be stitched too.  The uncounted selectors only ever
/// ask "is there one?", which a per-tile scan answers without knowing whether two filter
/// polygons in different tiles are two shapes or one; a count gets that wrong the moment a
/// filter region straddles a tile edge, so both sides are labelled and the pairs are
/// collected by region id rather than tallied per piece.
/// `covering` / `not_covering`, on core pieces: a candidate region covers a filter
/// region when every core piece of the filter lies within that one candidate region's
/// polygons in the same core, and the candidate is the same region in every core the
/// filter touches.  Asked per core it is exact wherever the copies are, since a copy is
/// exact inside its core.  Asked of a whole copy - does the filter fit inside one
/// candidate polygon - it needed the candidate to extend over the filter in one piece,
/// which held for drawn geometry and not for a boolean over a chip-sized operand: a
/// guard ring's interior, `holes - pcomp` with the pad frame's hole among the holes,
/// carried in each tile whatever the frame's hole left where the ring was out of
/// reach, and DN.3 read a ring round one deep well as shared.  `count` bounds how many
/// filter regions a kept candidate covers.
fn build_covering_tiles(
    cand: &TileMap,
    filt: &TileMap,
    keep: bool,
    (min, max): Count,
    tile_dbu: i32,
) -> TileMap {
    let cl = stitch_labeled(cand, tile_dbu);
    let fl = stitch_labeled(filt, tile_dbu);
    let t = tile_dbu as i64;
    // Per tile: for each filter piece, the candidate regions whose polygons hold it.
    let per_tile: Vec<Vec<(usize, Vec<usize>)>> = fl
        .by_tile
        .par_iter()
        .map(|(tile, fpolys)| {
            let Some(cpolys) = cl.by_tile.get(tile) else {
                return fpolys.iter().map(|(_, frid)| (*frid, Vec::new())).collect();
            };
            let (cx0, cy0) = (tile.0 as i64 * t, tile.1 as i64 * t);
            let (cx1, cy1) = ((tile.0 as i64 + 1) * t, (tile.1 as i64 + 1) * t);
            let mut by_rid: HashMap<usize, Vec<MergedPoly>> = HashMap::new();
            for (p, rid) in cpolys {
                by_rid.entry(*rid).or_default().push(p.clone());
            }
            let rid_box: Vec<(usize, (i32, i32, i32, i32))> = by_rid
                .iter()
                .map(|(rid, ps)| {
                    let b = ps
                        .iter()
                        .map(poly_bbox)
                        .fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |a, b| {
                            (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
                        });
                    (*rid, b)
                })
                .collect();
            fpolys
                .iter()
                .map(|(fp, frid)| {
                    let mut rids: Option<Vec<usize>> = None;
                    for piece in clip_to_box(vec![fp.clone()], cx0, cy0, cx1, cy1) {
                        let (px0, py0, px1, py1) = poly_bbox(&piece);
                        let here: Vec<usize> = rid_box
                            .iter()
                            .filter(|(_, (bx0, by0, bx1, by1))| {
                                px0 >= *bx0 && py0 >= *by0 && px1 <= *bx1 && py1 <= *by1
                            })
                            .filter(|(rid, _)| poly_within(&piece, &by_rid[rid]))
                            .map(|(rid, _)| *rid)
                            .collect();
                        rids = Some(match rids {
                            None => here,
                            Some(prev) => prev.into_iter().filter(|r| here.contains(r)).collect(),
                        });
                    }
                    (*frid, rids.unwrap_or_default())
                })
                .collect()
        })
        .collect();
    // A filter region is covered by the candidate regions that hold every one of its
    // pieces, in every tile it has one.
    let mut cover: Vec<Option<Vec<usize>>> = vec![None; fl.regions.len()];
    for v in per_tile {
        for (frid, rids) in v {
            cover[frid] = Some(match cover[frid].take() {
                None => rids,
                Some(prev) => prev.into_iter().filter(|r| rids.contains(r)).collect(),
            });
        }
    }
    let mut n = vec![0u32; cl.regions.len()];
    for rids in cover.into_iter().flatten() {
        for rid in rids {
            n[rid] += 1;
        }
    }
    let lo = min.unwrap_or(1);
    let mut out: TileMap = HashMap::new();
    for (tile, polys) in cl.by_tile {
        for (poly, rid) in polys {
            let hit = n[rid] >= lo && max.is_none_or(|hi| n[rid] <= hi);
            if hit == keep {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

fn build_counted_selection_tiles(
    cand: &TileMap,
    filt: &TileMap,
    kind: SelectionKind,
    keep: bool,
    (min, max): Count,
    tile_dbu: i32,
) -> TileMap {
    let cl = stitch_labeled(cand, tile_dbu);
    let fl = stitch_labeled(filt, tile_dbu);
    let t = tile_dbu as i64;
    // Only the part of the candidate this tile owns is tested, as the uncounted path
    // does: a copy is exact within its core and no further, and the meeting point of
    // two regions is owned by some core, which tests it against copies exact there.
    let met: HashSet<(usize, usize)> = cl
        .by_tile
        .par_iter()
        .flat_map_iter(|(tile, polys)| {
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            let Some(fpolys) = fl.by_tile.get(tile) else {
                return pairs.into_iter();
            };
            let (cx0, cy0) = (tile.0 as i64 * t, tile.1 as i64 * t);
            let (cx1, cy1) = ((tile.0 as i64 + 1) * t, (tile.1 as i64 + 1) * t);
            for (poly, rid) in polys {
                for piece in clip_to_box(vec![poly.clone()], cx0, cy0, cx1, cy1) {
                    let (x0, y0, x1, y1) = poly_bbox(&piece);
                    for (fp, frid) in fpolys {
                        if pairs.contains(&(*rid, *frid)) {
                            continue;
                        }
                        let (fx0, fy0, fx1, fy1) = poly_bbox(fp);
                        if x1 < fx0 || fx1 < x0 || y1 < fy0 || fy1 < y0 {
                            continue;
                        }
                        if piece_meets(kind, &piece, fp) {
                            pairs.push((*rid, *frid));
                        }
                    }
                }
            }
            pairs.into_iter()
        })
        .collect();
    let mut n = vec![0u32; cl.regions.len()];
    for (rid, _) in met {
        n[rid] += 1;
    }
    // KLayout's bounds are inclusive, and an absent `min` still means "at least one".
    let lo = min.unwrap_or(1);
    let mut out: TileMap = HashMap::new();
    for (tile, polys) in cl.by_tile {
        for (poly, rid) in polys {
            let hit = n[rid] >= lo && max.is_none_or(|hi| n[rid] <= hi);
            if hit == keep {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    out
}

fn build_selection_tiles(
    cand: &TileMap,
    filt: &TileMap,
    kind: SelectionKind,
    keep: bool,
    count: Count,
    tile_dbu: i32,
) -> TileMap {
    // An empty filter means no region matches any predicate — `inside` included, since
    // nothing can be contained in nothing.  Short-circuiting here avoids stitching a
    // dense candidate (e.g. `covering [GatPolyRes, Rsil]` on a chip with no resistors).
    if filt.values().all(|v| v.is_empty()) {
        return if keep { TileMap::new() } else { cand.clone() };
    }
    if kind == SelectionKind::Covers {
        return build_covering_tiles(cand, filt, keep, count, tile_dbu);
    }
    if count != (None, None) {
        return build_counted_selection_tiles(cand, filt, kind, keep, count, tile_dbu);
    }

    // Only a region with a piece in a tile the filter is filed in can meet it, so only
    // those regions are stitched, outward from the filter's tiles.  What is left in the
    // tiles never visited is regions that meet nothing: dropped by `keep`, and passed
    // through as their core-owned polygons otherwise, which is what the stitch would have
    // filed for them.
    let mut seeds: Vec<(i32, i32)> = filt
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, _)| *k)
        .collect();
    seeds.sort_unstable();
    let trace = std::env::var("GDSCHECK_STITCH_TRACE").is_ok();
    let t0 = std::time::Instant::now();
    let (labeled, visited) = stitch_from(cand, tile_dbu, true, Some(&seeds));
    let t_stitch = t0.elapsed().as_secs_f64();
    let t0 = std::time::Instant::now();
    // `Inside` reduces with AND over a region's pieces, so it starts true and is cleared
    // by the first uncovered piece; the others start false and are set by the first match.
    let and_reduce = kind == SelectionKind::Inside;
    // Decided across tiles in parallel; a flag only ever moves one way, so the order the
    // tiles land in does not matter.
    let flags: Vec<std::sync::atomic::AtomicBool> = (0..labeled.regions.len())
        .map(|_| std::sync::atomic::AtomicBool::new(and_reduce))
        .collect();
    let empty_f: Vec<MergedPoly> = Vec::new();
    let t = tile_dbu as i64;
    labeled.by_tile.par_iter().for_each(|(tile, polys)| {
        use std::sync::atomic::Ordering::Relaxed;
        let fpolys = filt.get(tile).unwrap_or(&empty_f);
        let fboxes: Vec<(i32, i32, i32, i32)> = fpolys.iter().map(poly_bbox).collect();
        let (cx0, cy0) = (tile.0 as i64 * t, tile.1 as i64 * t);
        let (cx1, cy1) = ((tile.0 as i64 + 1) * t, (tile.1 as i64 + 1) * t);
        for (poly, rid) in polys {
            // Only the part of the candidate this tile owns is tested here.  A copy is
            // exact within its tile's reach and no further, and the filter is only owed
            // to be exact there too; every point of the region is owned by some tile,
            // which tests it against a filter that is exact around it.  `inside` reduces
            // with AND over the pieces, so this asks each piece exactly what it can
            // answer; the others need one piece to meet the filter, and the piece that
            // does is the one owning the meeting point.
            //
            // `covering` is the exception: it asks whether a filter polygon fits inside
            // one candidate polygon, and a filter spanning two pieces fits in neither.
            // It keeps the whole copy, and a layer it reads is never clipped.
            let pieces = if kind == SelectionKind::Covers {
                vec![poly.clone()]
            } else {
                clip_to_box(vec![poly.clone()], cx0, cy0, cx1, cy1)
            };
            for piece in &pieces {
                if kind == SelectionKind::Inside {
                    if flags[*rid].load(Relaxed) && !poly_within(piece, fpolys) {
                        flags[*rid].store(false, Relaxed);
                    }
                } else if !flags[*rid].load(Relaxed) {
                    let (x0, y0, x1, y1) = poly_bbox(piece);
                    let hit = fpolys
                        .iter()
                        .zip(&fboxes)
                        .any(|(fp, &(fx0, fy0, fx1, fy1))| {
                            !(x1 < fx0 || fx1 < x0 || y1 < fy0 || fy1 < y0)
                                && piece_meets(kind, piece, fp)
                        });
                    if hit {
                        flags[*rid].store(true, Relaxed);
                    }
                }
            }
        }
    });
    let matches: Vec<bool> = flags.into_iter().map(|f| f.into_inner()).collect();
    let t_match = t0.elapsed().as_secs_f64();
    let t0 = std::time::Instant::now();

    let mut out: TileMap = HashMap::new();
    for (tile, polys) in labeled.by_tile {
        for (poly, rid) in polys {
            if matches[rid] == keep {
                out.entry(tile).or_default().push(poly);
            }
        }
    }
    if !keep {
        let rest: TileMap = cand
            .iter()
            .filter(|(k, _)| !visited.contains(k))
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        for (tile, polys) in core_owned(&rest, tile_dbu) {
            out.entry(tile).or_default().extend(polys);
        }
    }
    if trace {
        eprintln!(
            "selection {kind:?} keep={keep} seeds={} visited={} regions={} stitch={t_stitch:.1}s \
             match={t_match:.1}s output={:.1}s",
            seeds.len(),
            visited.len(),
            labeled.regions.len(),
            t0.elapsed().as_secs_f64()
        );
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
                    let Some(ps) = metal.get(&(nx, ny)) else {
                        continue;
                    };
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
                if let Some(i) = local_polys
                    .iter()
                    .position(|p| point_in_merged(ecx, ecy, p))
                {
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
    link_adjacent_pieces(&mut uf, &tile_pieces, &sides, None);
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
    if x0 == i32::MAX {
        None
    } else {
        Some((x0, y0, x1, y1))
    }
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
fn build_tiled_merge(boundaries: &[GdsBoundary], tile_dbu: i32, halo_dbu: i32) -> TileMap {
    let tile = tile_dbu.max(1) as i64;
    let halo = halo_dbu.max(0) as i64;

    let mut buckets: HashMap<(i32, i32), Vec<&GdsBoundary>> = HashMap::new();
    for b in boundaries {
        let Some((x0, y0, x1, y1)) = bbox_of(b) else {
            continue;
        };
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
/// What one rule needs of the layers it reaches: the halo per layer the rule raised,
/// and the closure of layers it reaches at all.  See `MergedCache::rule_halos`.
pub type RuleHalos = (HashMap<(i16, i16), i32>, HashSet<(i16, i16)>);

pub struct MergedCache {
    tile_dbu: i32,
    halo_dbu: i32,
    halo_by_layer: HashMap<(i16, i16), i32>,
    layers: HashMap<(i16, i16), TileMap>,
    /// The halo each cached layer was actually built at.  A layer can be cached at less
    /// than its configured halo when only rules that measure nothing have read it so far;
    /// a later consumer that needs more rebuilds it.  See [`MergedCache::ensure_at`].
    layer_halo: HashMap<(i16, i16), i32>,
    /// The halo each layer needs for the rule currently being run - its own reach on the
    /// layers it names, and what its derivations need of their sources - and the closure
    /// of layers the rule reaches.  A layer in the closure that the table does not raise
    /// takes the minimum; a layer outside it - a violation boundary, a layer named in
    /// `params` - takes its configured halo.  `None` outside a rule.
    rule_halos: Option<RuleHalos>,
    regions: HashMap<(i16, i16), Vec<Region>>,
    /// Lazy virtual layers, keyed by their synthetic (layer, datatype); built on
    /// first `ensure` from their source layers' tiles instead of the layout.
    virtual_defs: HashMap<(i16, i16), TiledVirtual>,
    /// Edge layers, keyed the same way and built the same way, but holding boundary
    /// segments rather than regions.  Kept apart from `layers` because the element type
    /// differs — a check asks for one or the other, never both under one key.
    edge_layers: HashMap<(i16, i16), EdgeTileMap>,
    edge_defs: HashMap<(i16, i16), TiledEdge>,
    /// Every edge layer filed under every tile it crosses, for reading as a filter.
    edge_spans: HashMap<(i16, i16), EdgeTileMap>,
    /// The halo each cached edge layer was built for; see `layer_halo`.
    edge_halo: HashMap<(i16, i16), i32>,
    /// Derived layers delivered as core-clipped pieces copied out to their consumers'
    /// reach, rather than as whole tile copies.  Decided by `clippable_layers`.
    clippable: HashSet<(i16, i16)>,
    /// Layer names, for the traces and `GDSCHECK_DUMP_LAYER`.
    names: HashMap<(i16, i16), String>,
    /// Polygon copies per cached layer, kept at insert so the trace's resident count
    /// is a sum over layers and not a walk over fifty million tiles per rule.
    layer_polys: HashMap<(i16, i16), usize>,
    /// A fat copy set aside when a rule wanted the layer much thinner: the halo it
    /// was built for, its tiles, and its copies.  Handed back to the next rule that
    /// wants that reach, so comp at the guard ring's 200 um is merged once and not
    /// again for every deck that has a guard-ring rule between its own.  Dropped like
    /// the live copy when no later rule needs its reach.
    shelf: HashMap<(i16, i16), (i32, TileMap, usize)>,
    /// Derived layers read, somewhere downstream, by the enclosure engine or by
    /// `covering`, which take one tile's copy for the whole region.  Built and copied
    /// the old way; see `clippable_layers`.
    whole_chain: HashSet<(i16, i16)>,
}

/// A registered edge layer: its op and the source keys it is built from.
#[derive(Clone, Debug)]
struct TiledEdge {
    op: EdgeOp,
    sources: Vec<(i16, i16)>,
}

impl MergedCache {
    /// `halo_dbu` is the default/minimum halo; `halo_by_layer` overrides it for
    /// layers that need a larger one (e.g. a coarse layer with a big `max_width`).
    /// Keeping the halo per layer stops one large rule from inflating the merge of
    /// every other (fine) layer in the deck.
    /// How wide a halo a *stitched* layer must be put back on, or 0 for none.
    ///
    /// Only a layer some distance check measures on needs one, and those are exactly the
    /// layers a rule raised a halo for.  A layer that is only ever composed further or
    /// extracted from - the sources of the channel-width edge layers, say - is read
    /// tile-locally and gains nothing but duplicate work from the copies.
    /// The halo this layer's tiles are built with, for diagnostics.
    pub fn halo_dbu(&self, l: i16, d: i16) -> i32 {
        self.halo_for((l, d)).unwrap_or(self.halo_dbu)
    }

    fn stitch_halo(&self, key: (i16, i16)) -> i32 {
        self.halo_for(key).unwrap_or(0)
    }

    /// What the running rule needs of `key`, else what the layer is configured with.
    /// `None` is a layer nothing raised, which the callers read as their own floor.
    fn halo_for(&self, key: (i16, i16)) -> Option<i32> {
        match &self.rule_halos {
            Some((table, closure)) if closure.contains(&key) => table.get(&key).copied(),
            _ => self.halo_by_layer.get(&key).copied(),
        }
    }

    pub fn new(tile_dbu: i32, halo_dbu: i32, halo_by_layer: HashMap<(i16, i16), i32>) -> Self {
        Self {
            tile_dbu,
            halo_dbu,
            halo_by_layer,
            layers: HashMap::new(),
            layer_halo: HashMap::new(),
            rule_halos: None,
            regions: HashMap::new(),
            virtual_defs: HashMap::new(),
            edge_layers: HashMap::new(),
            edge_defs: HashMap::new(),
            edge_spans: HashMap::new(),
            edge_halo: HashMap::new(),
            clippable: HashSet::new(),
            names: HashMap::new(),
            layer_polys: HashMap::new(),
            shelf: HashMap::new(),
            whole_chain: HashSet::new(),
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
        self.virtual_defs
            .insert(key, TiledVirtual { op, sources, text });
    }

    /// Register an edge layer, built on first [`MergedCache::edges`] from its sources.
    pub fn register_edge(&mut self, key: (i16, i16), op: EdgeOp, sources: Vec<(i16, i16)>) {
        self.edge_defs.insert(key, TiledEdge { op, sources });
    }

    /// True if `key` names an edge layer rather than a polygon one.
    pub fn is_edge_layer(&self, key: (i16, i16)) -> bool {
        self.edge_defs.contains_key(&key)
    }

    /// The polygon layer an edge layer was ultimately cut from, if it is only one.
    ///
    /// Every edge op but one either drops segments or splits them, and never invents one:
    /// whatever survives still lies on the boundary the first source was cut from, so the
    /// walk follows that source back to its `edges` node and reports the region there.
    /// `or` is the exception - it is the one op that puts two boundaries in a single
    /// layer - so it agrees with itself or admits it has no single region.
    pub fn edge_base_region(&self, key: (i16, i16)) -> Option<(i16, i16)> {
        let def = self.edge_defs.get(&key)?;
        if def.op == EdgeOp::Edges || matches!(def.op, EdgeOp::WidthBelow(_)) {
            return def.sources.first().copied();
        }
        if def.op == EdgeOp::Or {
            let mut found = None;
            for &s in &def.sources {
                let base = self.edge_base_region(s)?;
                if *found.get_or_insert(base) != base {
                    return None; // two boundaries: no one material to measure through
                }
            }
            return found;
        }
        self.edge_base_region(*def.sources.first()?)
    }

    /// Build `key`'s edge tiles if not already done, resolving its sources first —
    /// polygon sources through `ensure`, edge sources recursively through here.
    pub fn ensure_edges(&mut self, layout: &FlatLayout, key: (i16, i16)) {
        let want = self.halo_dbu(key.0, key.1);
        if self.edge_layers.contains_key(&key) {
            if self.edge_halo.get(&key).copied().unwrap_or(i32::MAX) >= want {
                return;
            }
            self.edge_layers.remove(&key);
            self.edge_spans.remove(&key);
        }
        let t0 = std::time::Instant::now();
        self.ensure_edges_inner(layout, key);
        self.edge_halo.insert(key, want);
        self.dump_edges_if_asked(key);
        if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
            let n: usize = self.edge_layers[&key].values().map(|v| v.len()).sum();
            eprintln!(
                "edges {} op={:?} halo={want} edges={n} {:.1}s",
                self.name_of(key),
                self.edge_defs.get(&key).map(|d| d.op),
                t0.elapsed().as_secs_f64()
            );
        }
    }

    fn ensure_edges_inner(&mut self, layout: &FlatLayout, key: (i16, i16)) {
        let Some(def) = self.edge_defs.get(&key).cloned() else {
            self.edge_layers.insert(key, EdgeTileMap::new());
            self.edge_spans.insert(key, EdgeTileMap::new());
            return;
        };
        // An op's sources are polygons, edges, or one of each; resolve accordingly.
        // A subject with nothing in it makes the layer empty before the filter is
        // built, and the filter can be the expensive half: MDP.4b cuts the edges of a
        // deep well nothing here draws against the guard rings' interiors.
        for (i, &src) in def.sources.iter().enumerate() {
            let is_poly = def.op.takes_polygons() || (def.op.mixes() && i == 1);
            if is_poly {
                self.ensure(layout, src.0, src.1);
            } else {
                self.ensure_edges(layout, src);
            }
            if i == 0 && def.op != EdgeOp::Or {
                let empty = if is_poly {
                    self.layers[&src].is_empty()
                } else {
                    self.edge_layers[&src].is_empty()
                };
                if empty {
                    if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
                        eprintln!(
                            "edges {} op={:?} empty: source {} holds nothing",
                            self.name_of(key),
                            def.op,
                            self.name_of(src)
                        );
                    }
                    self.edge_layers.insert(key, EdgeTileMap::new());
                    self.edge_spans.insert(key, EdgeTileMap::new());
                    return;
                }
            }
        }

        let empty_p: Vec<MergedPoly> = Vec::new();
        let t = self.tile_dbu as i64;
        // `or` puts every source in the output, so all of them are subjects; every other
        // op reads the first and measures it against the rest.
        let subjects = if def.op == EdgeOp::Or {
            def.sources.len()
        } else {
            1
        };
        let is_poly = |i: usize| def.op.takes_polygons() || (def.op.mixes() && i == 1);
        // Compose per tile, over the tiles a *subject* touches.  A filter alone produces
        // nothing, and the tiles it has to be read in are the subject's, not its own.
        let mut keys: HashSet<(i32, i32)> = HashSet::new();
        for (i, &src) in def.sources.iter().enumerate() {
            if is_poly(i) {
                if def.op.takes_polygons() {
                    keys.extend(self.layers[&src].keys().copied());
                }
            } else if i < subjects {
                keys.extend(self.edge_layers[&src].keys().copied());
            }
        }
        let mut out: EdgeTileMap = HashMap::new();
        for tile in keys {
            // Every tile this tile's subjects reach.  A filter that overlaps one of them
            // shares points with it, so it crosses those tiles too and the spanning index
            // has it filed there - which is what makes a long filter visible to a subject
            // it meets three tiles away from where its own midpoint fell.
            let mut reach: HashSet<(i32, i32)> = HashSet::from([tile]);
            for (i, &src) in def.sources.iter().enumerate() {
                if is_poly(i) || i >= subjects {
                    continue;
                }
                for e in self.edge_layers[&src].get(&tile).into_iter().flatten() {
                    reach.extend(edge_tiles(e, t));
                }
            }
            let mut gathered: Vec<Vec<Edge>> = Vec::new();
            let mut ps: Vec<std::borrow::Cow<[MergedPoly]>> = Vec::new();
            for (i, &src) in def.sources.iter().enumerate() {
                if is_poly(i) && def.op.mixes() {
                    // A polygon filter is read wherever the subject edges go, like an
                    // edge filter above: an edge is filed by its midpoint and can run
                    // three tiles past it, and the polygon it meets there is in this
                    // tile only if the polygon's halo happens to reach.  It used to,
                    // by luck - a 40 µm rule elsewhere on the same layer - until halos
                    // were built per rule and MDP.9b lost its drift edge to a 1 µm one.
                    // Copies of one polygon filed in several tiles are dropped; two
                    // tiles' differing merges of one region are both kept, and read as
                    // a union by every op that takes a filter.
                    let mut seen: HashSet<(usize, i32, i32, i32, i32)> = HashSet::new();
                    let mut v: Vec<MergedPoly> = Vec::new();
                    for tt in &reach {
                        for m in self.layers[&src].get(tt).into_iter().flatten() {
                            let (a, b) = (m.outer[0], m.outer[m.outer.len() / 2]);
                            if seen.insert((m.outer.len(), a.x, a.y, b.x, b.y)) {
                                v.push(m.clone());
                            }
                        }
                    }
                    ps.push(std::borrow::Cow::Owned(v));
                } else if is_poly(i) {
                    ps.push(std::borrow::Cow::Borrowed(
                        self.layers[&src]
                            .get(&tile)
                            .map(Vec::as_slice)
                            .unwrap_or(&empty_p),
                    ));
                } else if i < subjects {
                    gathered.push(
                        self.edge_layers[&src]
                            .get(&tile)
                            .cloned()
                            .unwrap_or_default(),
                    );
                } else {
                    let spans = &self.edge_spans[&src];
                    let mut seen: HashSet<(i32, i32, i32, i32)> = HashSet::new();
                    let mut v = Vec::new();
                    for tt in &reach {
                        for e in spans.get(tt).into_iter().flatten() {
                            if seen.insert((e.a.x, e.a.y, e.b.x, e.b.y)) {
                                v.push(*e);
                            }
                        }
                    }
                    gathered.push(v);
                }
            }
            let es: Vec<&[Edge]> = gathered.iter().map(Vec::as_slice).collect();
            let ps: Vec<&[MergedPoly]> = ps.iter().map(|c| c.as_ref()).collect();
            let core = (
                tile.0 as i64 * t,
                tile.1 as i64 * t,
                (tile.0 as i64 + 1) * t,
                (tile.1 as i64 + 1) * t,
            );
            let composed = compose_edge_tile(def.op, &es, &ps, core);
            if !composed.is_empty() {
                // Re-bucket: an op can move an edge's midpoint out of the tile it was
                // composed in (a cut piece sits elsewhere than its parent).
                for (k, v) in bucket_edges(composed, self.tile_dbu) {
                    out.entry(k).or_default().extend(v);
                }
            }
        }
        if def.op == EdgeOp::Edges {
            let all: Vec<Edge> = out.into_values().flatten().collect();
            out = bucket_edges(rejoin_collinear(all), self.tile_dbu);
        }
        self.edge_spans
            .insert(key, spanning_index(&out, self.tile_dbu));
        self.edge_layers.insert(key, out);
    }

    /// Per-tile edges of an edge layer.  Must be `ensure_edges`d first.
    pub fn edges(&self, key: (i16, i16)) -> &EdgeTileMap {
        self.edge_layers
            .get(&key)
            .expect("MergedCache::edges called before ensure_edges")
    }

    /// Whole-layer connected regions (areas + markers), built by stitching the
    /// per-tile merge across tile borders.  Cached per layer.
    pub fn regions(&mut self, layout: &FlatLayout, layer: i16, datatype: i16) -> &[Region] {
        self.ensure(layout, layer, datatype);
        if !self.regions.contains_key(&(layer, datatype)) {
            let r = stitch_regions(&self.layers[&(layer, datatype)], self.tile_dbu);
            if self.dump_wanted((layer, datatype)) {
                for (i, reg) in r.iter().enumerate() {
                    eprintln!(
                        "dump {} region {i} area={:.0} marker=({:.1},{:.1})",
                        self.name_of((layer, datatype)),
                        reg.area_dbu,
                        reg.marker.0,
                        reg.marker.1
                    );
                }
            }
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
        Core {
            x0: tx as i64 * t,
            y0: ty as i64 * t,
            x1: (tx as i64 + 1) * t,
            y1: (ty as i64 + 1) * t,
        }
    }

    /// Merge `(layer, datatype)` and cache it if not already done.  A registered
    /// lazy virtual layer is composed per tile from its (recursively ensured)
    /// sources; any other layer is tiled directly from the layout.
    /// Whether `key` is a drawn layer - one with no virtual or edge derivation behind it.
    pub fn is_drawn(&self, key: (i16, i16)) -> bool {
        !self.virtual_defs.contains_key(&key) && !self.edge_defs.contains_key(&key)
    }

    /// The reach of the rule about to run and the layers it names, or `None` to fall back
    /// to each layer's configured halo.  Set by the deck runner around every rule.
    pub fn set_whole_chain(&mut self, keys: HashSet<(i16, i16)>) {
        self.whole_chain = keys;
    }

    /// Whether any other virtual or edge layer is built from `key`.
    fn feeds_another_layer(&self, key: (i16, i16)) -> bool {
        self.virtual_defs
            .iter()
            .any(|(k, d)| *k != key && d.sources.contains(&key))
            || self
                .edge_defs
                .iter()
                .any(|(k, d)| *k != key && d.sources.contains(&key))
    }

    pub fn set_names(&mut self, names: HashMap<(i16, i16), String>) {
        self.names = names;
    }

    /// A layer's name if known, else its numbers.
    pub fn name_of(&self, key: (i16, i16)) -> String {
        self.names
            .get(&key)
            .cloned()
            .unwrap_or_else(|| format!("{}/{}", key.0, key.1))
    }

    /// `GDSCHECK_NO_CLIP=1` builds every layer whole, for telling a clipping problem
    /// from any other.
    pub fn set_clippable(&mut self, keys: HashSet<(i16, i16)>) {
        self.clippable = if std::env::var("GDSCHECK_NO_CLIP").is_ok() {
            HashSet::new()
        } else {
            keys
        };
    }

    /// `GDSCHECK_DUMP_LAYER=<layer>/<datatype>[,...]`: print a layer's tiles as they are
    /// cached, polygons as bounding boxes and edges as segments.
    fn dump_wanted(&self, key: (i16, i16)) -> bool {
        std::env::var("GDSCHECK_DUMP_LAYER").is_ok_and(|w| {
            w.split(',')
                .any(|k| k == format!("{}/{}", key.0, key.1) || k == self.name_of(key))
        })
    }

    fn dump_edges_if_asked(&self, key: (i16, i16)) {
        if !self.dump_wanted(key) {
            return;
        }
        let mut keys: Vec<_> = self.edge_layers[&key].keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            for e in &self.edge_layers[&key][&k] {
                eprintln!(
                    "dump {} tile=({},{}) edge=({},{})-({},{})",
                    self.name_of(key),
                    k.0,
                    k.1,
                    e.a.x,
                    e.a.y,
                    e.b.x,
                    e.b.y
                );
            }
        }
    }

    fn dump_if_asked(&self, key: (i16, i16), tiles: &TileMap) {
        if !self.dump_wanted(key) {
            return;
        }
        let mut keys: Vec<_> = tiles.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            for p in &tiles[&k] {
                let (x0, y0, x1, y1) = poly_bbox(p);
                let verts = if std::env::var("GDSCHECK_DUMP_VERTS").is_ok() {
                    p.outer
                        .iter()
                        .map(|v| format!("({},{})", v.x, v.y))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    String::new()
                };
                eprintln!(
                    "dump {} tile=({},{}) bbox=({x0},{y0})-({x1},{y1}) n={} holes={} {verts}",
                    self.name_of(key),
                    k.0,
                    k.1,
                    p.outer.len(),
                    p.holes.len()
                );
            }
        }
    }

    /// Set (or clear) the per-layer halos of the rule about to run.  See `rule_halos`.
    pub fn set_rule_halos(&mut self, halos: Option<RuleHalos>) {
        self.rule_halos = halos;
    }

    /// Merge `layer`/`datatype` with the halo the running rule needs of it, or the
    /// layer's configured halo outside a rule.
    pub fn ensure(&mut self, layout: &FlatLayout, layer: i16, datatype: i16) {
        let want = self.halo_dbu(layer, datatype);
        self.ensure_at(layout, layer, datatype, want);
    }

    /// Merge a **drawn** layer with at least `want` of halo, reusing a cached merge that
    /// already has that much and rebuilding one that has less.
    ///
    /// A layer's configured halo is the maximum over everything that reads it, so one
    /// long-reach consumer makes every other pay its reach - and against a 20 µm tile a
    /// halo multiplies a layer's geometry by about `((tile + 2·halo)/tile)²`.  On a real
    /// design GF180's slotting chain put 120 µm on the drawn metals and a guard-ring
    /// chain 200 µm on the actives, so a `no_angle` check - which reads each polygon's
    /// own edges and needs no halo whatsoever - was merging Contact into 175 million
    /// polygon copies from 3.6 million drawn shapes.  A caller that knows it needs less
    /// asks for less.
    ///
    /// Derived layers ignore `want`: what a virtual or edge layer's sources need is set
    /// by the derivation, not by whoever reads the result.  A derived layer records the
    /// halo it was built for all the same, because its sources were built for that and
    /// a rule that needs more of it has them rebuilt - and then needs it rebuilt on top.
    pub fn ensure_at(&mut self, layout: &FlatLayout, layer: i16, datatype: i16, want: i32) {
        let key = (layer, datatype);
        let want = if self.is_drawn(key) {
            want
        } else if self.clippable.contains(&key) {
            // Delivered as pieces copied out to exactly this reach - none when nothing
            // asked - so that, not the floor a whole copy gets, is what it was built for.
            self.stitch_halo(key)
        } else {
            self.halo_dbu(layer, datatype)
        };
        if self.layers.contains_key(&key) {
            let have = self.layer_halo.get(&key).copied().unwrap_or(i32::MAX);
            // A copy built far past this consumer's reach is reused only while it is
            // cheap to keep reading: a drawn layer at a 200 µm guard-ring halo is 250
            // copies of every shape, and a 0.3 µm spacing rule reading it walks all of
            // them - NAT.2 spent 27 s on comp that way, against a 1.6 s merge to build
            // it thin (and 1.6 s again for the guard-ring rule that needs it fat later).
            // The threshold is a ratio of reaches; the copy count grows with its square.
            // A derived layer rides the same way: a selection over the implant union
            // cached at 200 um read 732k copies where 55k would do, ten times slower.
            // Its rebuild is its own build, cheap at a thin reach, so the bar is lower.
            let copies = self.layer_polys.get(&key).copied().unwrap_or(0);
            let far_too_fat = have != i32::MAX && want < have / 4 && copies > 200_000;
            if have >= want && !far_too_fat {
                return;
            }
            // Cached too thin for this consumer, or too fat to read.  Drop it and
            // everything derived from it by stitching, which inherits the tiles' reach.
            let tiles = self.layers.remove(&key).expect("checked above");
            let polys = self.layer_polys.remove(&key).unwrap_or(0);
            self.regions.remove(&key);
            self.layer_halo.remove(&key);
            if far_too_fat
                && self.is_drawn(key)
                && self.shelf.get(&key).is_none_or(|(h, _, _)| *h < have)
            {
                // Too fat to read now, and expensive to make again: set it aside.  The
                // shelf keeps the fattest, since a thinner one is cheap to make again.
                self.shelf.insert(key, (have, tiles, polys));
            }
        }
        if let Some((have, _, _)) = self.shelf.get(&key)
            && *have >= want
            && want > *have / 4
        {
            {
                let (have, tiles, polys) = self.shelf.remove(&key).expect("just seen");
                self.layers.insert(key, tiles);
                self.layer_polys.insert(key, polys);
                self.layer_halo.insert(key, have);
                if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
                    eprintln!(
                        "unshelve {} halo={have}dbu copies={polys}",
                        self.name_of(key)
                    );
                }
                return;
            }
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
                self.layer_polys
                    .insert(key, tiles.values().map(|v| v.len()).sum());
                self.layers.insert(key, tiles);
                self.layer_halo.insert(key, want);
                return;
            }
            // A layer with nothing to work on is empty, and nothing else in its chain
            // needs building to say so: a deep-well rule on a design with no deep well
            // used to merge COMP at the guard ring's reach before finding that out.
            // Drawn sources are asked first, since that costs nothing; a derived one
            // is asked as soon as it is built, before the sources after it are.
            let trace = std::env::var("GDSCHECK_MERGE_TRACE").is_ok();
            let mut empty_source = def
                .sources
                .iter()
                .enumerate()
                .find(|(i, src)| {
                    empty_when_source_empty(def.op, *i)
                        && self.is_drawn(**src)
                        && layout.get(src.0, src.1).is_empty()
                })
                .map(|(i, _)| i);
            // A selector's filter may be an *edge* layer rather than a polygon one -
            // GF180's PRES.9a keeps the RES_MK markings whose boundary does not follow
            // the resistor's - so those sources are built as edges, not merged.
            if empty_source.is_none() {
                for (i, &(sg, sd)) in def.sources.iter().enumerate() {
                    if self.is_edge_layer((sg, sd)) {
                        self.ensure_edges(layout, (sg, sd));
                        continue;
                    }
                    self.ensure(layout, sg, sd);
                    if empty_when_source_empty(def.op, i) && self.layers[&(sg, sd)].is_empty() {
                        empty_source = Some(i);
                        break;
                    }
                }
            }
            if let Some(i) = empty_source {
                if trace {
                    eprintln!(
                        "virtual {} op={:?} empty: source {} holds nothing",
                        self.name_of(key),
                        def.op,
                        self.name_of(def.sources[i])
                    );
                }
                self.insert_virtual(
                    key,
                    def.op,
                    0,
                    TileMap::new(),
                    std::time::Instant::now(),
                    want,
                );
                return;
            }
            let t0 = std::time::Instant::now();
            let src_copies: usize = def
                .sources
                .iter()
                .filter_map(|s| self.layers.get(s))
                .map(|m| m.values().map(|v| v.len()).sum::<usize>())
                .sum();
            if def.op == VirtualOp::Extents {
                let tiles = build_extents_tiles(&self.layers[&def.sources[0]], self.tile_dbu);
                self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                return;
            }
            if let VirtualOp::Open(r)
            | VirtualOp::Shrink(r)
            | VirtualOp::ShrinkX(r)
            | VirtualOp::ShrinkY(r) = def.op
                && erosion_on_regions(r, self.tile_dbu)
            {
                let how = match def.op {
                    VirtualOp::Open(_) => Erosion::Open,
                    VirtualOp::Shrink(_) => Erosion::Shrink,
                    VirtualOp::ShrinkX(_) => Erosion::ShrinkX,
                    _ => Erosion::ShrinkY,
                };
                let tiles =
                    build_erosion_tiles(&self.layers[&def.sources[0]], self.tile_dbu, r, how);
                let tiles = if self.clippable.contains(&key) {
                    tiles
                } else {
                    assemble_zone_copies(tiles, self.tile_dbu, self.stitch_halo(key))
                };
                self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                return;
            }
            if matches!(def.op, VirtualOp::Holes | VirtualOp::WithHoles) {
                let keep_regions = def.op == VirtualOp::WithHoles;
                let tiles =
                    build_holes_tiles(&self.layers[&def.sources[0]], self.tile_dbu, keep_regions);
                let tiles = if self.clippable.contains(&key) {
                    tiles
                } else if keep_regions {
                    let whole = self.whole_chain.contains(&key);
                    rebroadcast_halo(tiles, self.tile_dbu, self.stitch_halo(key), !whole)
                } else {
                    assemble_zone_copies(tiles, self.tile_dbu, self.stitch_halo(key))
                };
                self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                return;
            }
            if let Some(f) = def.op.region_filter() {
                let tiles =
                    build_region_filter_tiles(&self.layers[&def.sources[0]], self.tile_dbu, f);
                let tiles = if self.clippable.contains(&key) {
                    tiles
                } else {
                    let whole = self.whole_chain.contains(&key);
                    rebroadcast_halo(tiles, self.tile_dbu, self.stitch_halo(key), !whole)
                };
                self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                return;
            }
            if let Some((kind, keep, count)) = def.op.selection() {
                if let Some(&fkey) = def.sources.get(1).filter(|&&k| self.is_edge_layer(k)) {
                    if kind != SelectionKind::Touches {
                        eprintln!(
                            "virtual layer: only `interacting` / `not_interacting` accept \
                             an edge layer as their filter"
                        );
                    }
                    let tiles = build_edge_selection_tiles(
                        &self.layers[&def.sources[0]],
                        &self.edge_spans[&fkey],
                        keep,
                        self.tile_dbu,
                    );
                    self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                    return;
                }
                // candidate = source[0], filter = source[1] (empty if absent).
                let empty = TileMap::new();
                let cand = &self.layers[&def.sources[0]];
                let filt = def
                    .sources
                    .get(1)
                    .map(|s| &self.layers[s])
                    .unwrap_or(&empty);
                let tiles = build_selection_tiles(cand, filt, kind, keep, count, self.tile_dbu);
                // Stitching hands back one whole copy per region per tile that owns a
                // piece of it, and nothing in the tiles around - so a consumer that reads
                // across a tile edge, a rule or a boolean alike, is owed the copies back
                // out to its reach.  A selection that fed another layer used to be left
                // without them, because a whole copy carries whatever its tile computed
                // past its exact zone and a boolean on that subtracted geometry present
                // on one side only; `rebroadcast_halo` now cuts each copy to the zone it
                // is exact in, so the copies can go wherever they are needed.  A layer
                // delivered as pieces is cut and copied by `insert_virtual` instead.
                let tiles = if self.clippable.contains(&key) {
                    tiles
                } else if self.whole_chain.contains(&key) {
                    if self.feeds_another_layer(key) {
                        tiles
                    } else {
                        rebroadcast_halo(tiles, self.tile_dbu, self.stitch_halo(key), false)
                    }
                } else {
                    rebroadcast_halo(tiles, self.tile_dbu, self.stitch_halo(key), true)
                };
                self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
                return;
            }
            let src_maps: Vec<&TileMap> = def.sources.iter().map(|s| &self.layers[s]).collect();
            let tiles = build_virtual_tiles(def.op, &src_maps);
            self.insert_virtual(key, def.op, src_copies, tiles, t0, want);
            return;
        }
        let tile = self.tile_dbu;
        let halo = want;
        let raw = layout.get(layer, datatype);
        let t0 = std::time::Instant::now();
        let tiles = build_tiled_merge(raw, tile, halo);
        if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
            let copies: usize = tiles.values().map(|v| v.len()).sum();
            eprintln!(
                "merge {layer}/{datatype} raw={} halo={}dbu tile={}dbu tiles={} copies={} \
                 blowup={:.0}x {:.1}s",
                raw.len(),
                halo,
                tile,
                tiles.len(),
                copies,
                copies as f64 / (raw.len().max(1)) as f64,
                t0.elapsed().as_secs_f64()
            );
        }
        self.dump_if_asked(key, &tiles);
        self.layer_polys
            .insert(key, tiles.values().map(|v| v.len()).sum());
        self.layers.insert(key, tiles);
        self.layer_halo.insert(key, halo);
    }

    /// Store a built derived layer, and under `GDSCHECK_MERGE_TRACE` say what it cost:
    /// the copies it read, the copies it holds, and the time - a selection stitches its
    /// whole candidate layer, so on a heavily haloed source this is where a run goes.
    fn insert_virtual(
        &mut self,
        key: (i16, i16),
        op: VirtualOp,
        src_copies: usize,
        tiles: TileMap,
        t0: std::time::Instant,
        want: i32,
    ) {
        let clipped = self.clippable.contains(&key);
        let tiles = if clipped {
            let pieces = clip_to_cores(tiles, self.tile_dbu);
            rebroadcast_halo(pieces, self.tile_dbu, self.stitch_halo(key), true)
        } else {
            tiles
        };
        if std::env::var("GDSCHECK_MERGE_TRACE").is_ok() {
            let copies: usize = tiles.values().map(|v| v.len()).sum();
            eprintln!(
                "virtual {} op={:?} src_copies={} tiles={} copies={}{} {:.1}s resident={}",
                self.name_of(key),
                op,
                src_copies,
                tiles.len(),
                copies,
                if clipped { " clipped" } else { "" },
                t0.elapsed().as_secs_f64(),
                self.resident_summary()
            );
        }
        self.dump_if_asked(key, &tiles);
        self.layer_polys
            .insert(key, tiles.values().map(|v| v.len()).sum());
        self.layers.insert(key, tiles);
        self.layer_halo.insert(key, want);
    }

    /// What the cache holds right now, for the traces: layers, polygon copies, and how
    /// many of those layers no rule names (derived intermediates, which nothing evicts).
    pub fn resident_summary(&self) -> String {
        let polys: usize = self.layer_polys.values().sum();
        let derived = self.layers.keys().filter(|k| !self.is_drawn(**k)).count();
        format!("{}L/{}derived/{}polys", self.layers.len(), derived, polys)
    }

    /// Polygon copies resident across every cached layer and the shelf.
    pub fn resident_polys(&self) -> usize {
        self.layer_polys.values().sum::<usize>()
            + self.shelf.values().map(|(_, _, n)| n).sum::<usize>()
    }

    /// Every cached layer with its polygon copies, shelf included.
    pub fn resident_layers(&self) -> Vec<((i16, i16), usize)> {
        let mut v: Vec<((i16, i16), usize)> =
            self.layer_polys.iter().map(|(k, n)| (*k, *n)).collect();
        for (k, (_, _, n)) in &self.shelf {
            v.push((*k, *n));
        }
        v
    }

    /// The halo a cached layer was built for, or `None` if it is not cached.
    pub fn cached_halo(&self, key: (i16, i16)) -> Option<i32> {
        self.layer_halo
            .get(&key)
            .or_else(|| self.edge_halo.get(&key))
            .copied()
            .max(self.shelf.get(&key).map(|(h, _, _)| *h))
    }

    /// Drop a layer's cached tiles and stitched regions.  Used by the deck
    /// runner to free a layer's merged geometry once no remaining rule needs it,
    /// bounding peak memory when a deck touches many layers.
    pub fn evict(&mut self, layer: i16, datatype: i16) {
        let key = (layer, datatype);
        self.layers.remove(&key);
        self.layer_polys.remove(&key);
        self.layer_halo.remove(&key);
        self.shelf.remove(&key);
        self.regions.remove(&key);
        self.edge_layers.remove(&key);
        self.edge_spans.remove(&key);
        self.edge_halo.remove(&key);
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

    /// A violation on a tile line has to be claimed, and by a tile that can see it.
    #[test]
    fn a_point_on_a_tile_line_is_claimed_by_both_its_tiles() {
        let lower = Core {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 100,
        };
        let upper = Core {
            x0: 0,
            y0: 100,
            x1: 100,
            y1: 200,
        };
        // On the line they share: both claim it, and `run_drc` drops the copy.
        assert!(lower.owns(50.0, 100.0));
        assert!(upper.owns(50.0, 100.0));
        // Anywhere else, exactly one does.
        assert!(lower.owns(50.0, 99.5));
        assert!(!upper.owns(50.0, 99.5));
        assert!(!lower.owns(50.0, 100.5));
        assert!(upper.owns(50.0, 100.5));
        // And a tile still claims nothing outside itself.
        assert!(!lower.owns(50.0, 200.5));
        assert!(!lower.owns(-0.5, 50.0));
    }

    fn edge(ax: i32, ay: i32, bx: i32, by: i32) -> Edge {
        Edge {
            a: IntPoint { x: ax, y: ay },
            b: IntPoint { x: bx, y: by },
        }
    }

    /// A run long enough to cross tile lines belongs to every tile it passes through, or
    /// it is invisible to the ops in all but one of them.
    #[test]
    fn a_long_edge_is_filed_in_every_tile_it_crosses() {
        // Five tiles across, midpoint in the middle one.
        let e = edge(50, 50, 450, 50);
        let mut t = edge_tiles(&e, 100);
        t.sort();
        assert_eq!(t, vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        // Short enough to stay home.
        assert_eq!(edge_tiles(&edge(10, 10, 90, 10), 100), vec![(0, 0)]);
        // Backwards is the same set.
        let mut back = edge_tiles(&edge(450, 50, 50, 50), 100);
        back.sort();
        assert_eq!(back, t);
    }

    /// A diagonal steps one tile at a time and never lists the whole bounding box.
    #[test]
    fn a_diagonal_walks_the_grid_rather_than_its_bbox() {
        let t = edge_tiles(&edge(50, 50, 350, 350), 100);
        assert!(t.len() <= 7, "walked {} tiles, not a 4x4 box", t.len());
        for c in [(0, 0), (3, 3)] {
            assert!(t.contains(&c), "missing {c:?}");
        }
        assert!(!t.contains(&(3, 0)), "the far corner is not on the line");
    }

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        MergedPoly {
            outer: vec![
                IntPoint::new(x0, y0),
                IntPoint::new(x1, y0),
                IntPoint::new(x1, y1),
                IntPoint::new(x0, y1),
            ],
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
        assert!(
            (regions[0].area_dbu - 1.2e9).abs() < 1.0,
            "area = {}",
            regions[0].area_dbu
        );
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
                IntPoint::new(0, 0),
                IntPoint::new(100, 0),
                IntPoint::new(100, 50),
                IntPoint::new(50, 50),
                IntPoint::new(50, 100),
                IntPoint::new(0, 100),
            ],
            holes: vec![],
        }
    }

    #[test]
    fn is_square_classifies_shapes() {
        assert!(
            is_square(&rect(0, 0, 160, 160)),
            "equal-sided rectangle is a square"
        );
        assert!(
            !is_square(&rect(0, 0, 160, 400)),
            "unequal-sided rectangle is a bar"
        );
        assert!(
            !is_square(&lshape()),
            "L-shape with a square bbox is not a square"
        );
        // A square outline with a hole is not a (filled) square.
        let mut holed = rect(0, 0, 200, 200);
        holed.holes.push(vec![
            IntPoint::new(50, 50),
            IntPoint::new(50, 150),
            IntPoint::new(150, 150),
            IntPoint::new(150, 50),
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

#[cfg(test)]
mod pieces_are_a_region {
    use super::*;
    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> MergedPoly {
        MergedPoly {
            outer: vec![
                IntPoint::new(x0, y0),
                IntPoint::new(x1, y0),
                IntPoint::new(x1, y1),
                IntPoint::new(x0, y1),
            ],
            holes: Vec::new(),
        }
    }
    fn area(v: &[MergedPoly]) -> f64 {
        v.iter().map(merged_area_dbu).sum()
    }

    /// A layer delivered as core-clipped pieces abuts along tile lines, and every size
    /// operator must read the pieces as the one region they are - an erode that treated
    /// a cut as a wall would eat a strip along it.
    #[test]
    fn every_size_operator_unions_abutting_pieces_first() {
        let whole = vec![rect(0, 0, 2000, 1000)];
        let halves = vec![rect(0, 0, 1000, 1000), rect(1000, 0, 2000, 1000)];
        let ops: [(&str, fn(&[MergedPoly]) -> Vec<MergedPoly>); 7] = [
            ("shrink", |p| shrink(p, 100.0)),
            ("shrink_x", |p| shrink_x(p, 100.0)),
            ("shrink_y", |p| shrink_y(p, 100.0)),
            ("open", |p| opening(p, 100.0)),
            ("close", |p| closing(p, 100.0)),
            ("grow", |p| grow_by(p, 100.0, 0.0)),
            ("grow_x", |p| grow_x(p, 100.0)),
        ];
        for (name, f) in ops {
            let (a, b) = (area(&f(&whole)), area(&f(&halves)));
            assert!((a - b).abs() < 1.0, "{name}: whole {a} vs pieces {b}");
        }
    }
}
