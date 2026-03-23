// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flate2::{write::GzEncoder, Compression};
use gds21::{
    GdsBoundary, GdsDateTime, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsTextElem, GdsUnits,
};
use gdscheck::pdk::PdkConfig;

// 1 DBU = 1 nm = 0.001 µm  →  1 µm = 1000 DBU
const DBU_PER_UM: f64 = 1000.0;

pub fn um(v: f64) -> i32 {
    (v * DBU_PER_UM).round() as i32
}

pub fn layer(pdk: &PdkConfig, name: &str) -> (i16, i16) {
    let l = pdk
        .layer(name)
        .unwrap_or_else(|| panic!("layer '{name}' not found in PDK"));
    (l.gds_layer as i16, l.gds_datatype as i16)
}

/// Build a polygon from a list of (x, y) vertices in µm.
/// The closing point (first vertex repeated) is added automatically.
pub fn poly(layer: (i16, i16), pts: &[(f64, f64)]) -> GdsElement {
    let mut xy: Vec<(i32, i32)> = pts.iter().map(|&(x, y)| (um(x), um(y))).collect();
    xy.push(xy[0]); // close the polygon
    GdsElement::GdsBoundary(GdsBoundary {
        layer: layer.0,
        datatype: layer.1,
        xy: GdsPoint::vec(&xy),
        ..Default::default()
    })
}

/// A text label at (x, y) µm on the given layer/texttype.
pub fn text(layer: (i16, i16), string: &str, x: f64, y: f64) -> GdsElement {
    GdsElement::GdsTextElem(GdsTextElem {
        string: string.to_string(),
        layer: layer.0,
        texttype: layer.1,
        xy: GdsPoint::new(um(x), um(y)),
        ..Default::default()
    })
}

pub fn rect(layer: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    GdsElement::GdsBoundary(GdsBoundary {
        layer: layer.0,
        datatype: layer.1,
        xy: GdsPoint::vec(&[
            (um(x0), um(y0)),
            (um(x1), um(y0)),
            (um(x1), um(y1)),
            (um(x0), um(y1)),
            (um(x0), um(y0)),
        ]),
        ..Default::default()
    })
}

/// Generate a 3-shape min-width test pattern.
///
/// Layout (shapes placed along x, starting at `offset`):
/// - Shape 1 at `offset`:            `width × height`          → clean
/// - Shape 2 at `offset + distance`: `(width+delta) × height`  → violation (too narrow in X)
/// - Shape 3 at `offset + 2*distance`: `width × (height+delta)` → violation (too narrow in Y)
///
/// Pass `delta = -0.005` for the default ½-grid undershoot.
pub fn min_width_pattern(
    layer: (i16, i16),
    width: f64,
    height: f64,
    distance: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    vec![
        // clean: exactly at the limit
        rect(layer, offset, 0.0, offset + width, height),
        // too narrow in X
        rect(layer, offset + distance, 0.0, offset + distance + width + delta, height),
        // too narrow in Y
        rect(layer, offset + 2.0 * distance, 0.0, offset + 2.0 * distance + width, height + delta),
    ]
}

/// Generate a 3-shape max-width test pattern.
///
/// Layout (shapes placed along x, starting at `offset`):
/// - Shape 1 at `offset`:              `width × height`          → clean
/// - Shape 2 at `offset + distance`:   `(width-delta) × height`  → violation (too wide in X)
/// - Shape 3 at `offset + 2*distance`: `width × (height-delta)`  → violation (too wide in Y)
///
/// Pass `delta = -0.005` for the default ½-grid overshoot.
pub fn max_width_pattern(
    layer: (i16, i16),
    width: f64,
    height: f64,
    distance: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    vec![
        // clean: exactly at the limit
        rect(layer, offset, 0.0, offset + width, height),
        // too wide in X
        rect(layer, offset + distance, 0.0, offset + distance + width - delta, height),
        // too wide in Y
        rect(layer, offset + 2.0 * distance, 0.0, offset + 2.0 * distance + width, height - delta),
    ]
}

/// Generate a 5-shape exact-width test pattern.
///
/// `exact_width` flags any perpendicular span that is not exactly `width`, so both
/// undersized and oversized features must violate.  Shapes along x from `offset`:
/// - Shape 1: `width × height`            → clean (exactly at the limit)
/// - Shape 2: `(width+delta) × height`    → too narrow in X
/// - Shape 3: `width × (height+delta)`    → too narrow in Y
/// - Shape 4: `(width-delta) × height`    → too wide in X
/// - Shape 5: `width × (height-delta)`    → too wide in Y
///
/// Pass `delta = -0.005` for the default ½-grid under/overshoot.
pub fn exact_width_pattern(
    layer: (i16, i16),
    width: f64,
    height: f64,
    distance: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    let step = width.max(height) + distance;
    vec![
        rect(layer, offset,               0.0, offset + width,               height),
        rect(layer, offset + step,        0.0, offset + step + width + delta, height),
        rect(layer, offset + 2.0 * step,  0.0, offset + 2.0 * step + width,  height + delta),
        rect(layer, offset + 3.0 * step,  0.0, offset + 3.0 * step + width - delta, height),
        rect(layer, offset + 4.0 * step,  0.0, offset + 4.0 * step + width,  height - delta),
    ]
}

/// Exact-width fixture that also exercises a `…NoSealring` virtual layer (`layer NOT
/// EdgeSeal`): a square `exact_width_pattern` out in the open, plus an identical copy
/// fully covered by an `edgeseal` rectangle.  The covered copy is subtracted away, so
/// the violation count equals a single pattern's (8) — proving features outside the
/// seal are still flagged while the seal copy is excluded.
pub fn exact_width_sealring_pattern(
    layer: (i16, i16),
    edgeseal: (i16, i16),
    width: f64,
    distance: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    // x-extent of the 5-shape square pattern (step = width + distance, 5 shapes).
    let span = 5.0 * width + 4.0 * distance;

    // 1. Open structure — its violations must be reported.
    let mut elems = exact_width_pattern(layer, width, width, distance, offset, delta);

    // 2. Identical structure 10 µm clear of the first, fully under EdgeSeal.
    let seal_off = offset + span + 10.0;
    elems.append(&mut exact_width_pattern(layer, width, width, distance, seal_off, delta));
    elems.push(rect(edgeseal, seal_off - 1.0, -1.0, seal_off + span + 1.0, width + 1.0));
    elems
}

/// Generate an offgrid test pattern: a clean on-grid rectangle plus one whose right
/// edge is shifted `off` µm off the manufacturing grid, so its two right-hand
/// vertices are off-grid (→ 2 offgrid vertices).
///
/// `size`, `distance` and `offset` must be on-grid; `off` must not be a multiple of
/// the grid (e.g. `0.003` for a 0.005 µm grid).
pub fn offgrid_pattern(
    layer: (i16, i16),
    size: f64,
    distance: f64,
    offset: f64,
    off: f64,
) -> Vec<GdsElement> {
    let x1 = offset + size + distance;
    vec![
        // fully on grid → clean
        rect(layer, offset, 0.0, offset + size, size),
        // right edge off the grid → its two vertices are offgrid
        rect(layer, x1, 0.0, x1 + size + off, size),
    ]
}

/// Generate a 5-shape min-space test pattern centred on (`offset`, `offset`).
///
/// Layout:
/// - Centre square (`shape × shape`) on `center_layer`
/// - Left  neighbour: gap == `distance`          → clean
/// - Bottom neighbour: gap == `distance`          → clean
/// - Right neighbour: gap == `distance + violation_delta` → violation (delta < 0)
/// - Top  neighbour: gap == `distance + violation_delta` → violation (delta < 0)
///
/// Pass `violation_delta = -0.005` for the default ½-grid overshoot.
/// Returns the elements so the caller can mix them into a larger layout.
pub fn space_pattern(
    center_layer: (i16, i16),
    other_layer: (i16, i16),
    shape: f64,
    distance: f64,
    offset: f64,
    violation_delta: f64,
) -> Vec<GdsElement> {
    let x0 = offset;
    let x1 = offset + shape;
    let y0 = offset;
    let y1 = offset + shape;
    let vd = distance + violation_delta; // gap used for the two violating neighbours

    vec![
        rect(center_layer, x0, y0, x1, y1),
        // left: clean (exactly at distance)
        rect(other_layer, x0 - distance - shape, y0, x0 - distance, y1),
        // bottom: clean (exactly at distance)
        rect(other_layer, x0, y0 - distance - shape, x1, y0 - distance),
        // right: violation
        rect(other_layer, x1 + vd, y0, x1 + vd + shape, y1),
        // top: violation
        rect(other_layer, x0, y1 + vd, x1, y1 + vd + shape),
    ]
}

/// Generate a 4-shape notch test pattern.
///
/// Produces two orientations (horizontal and vertical) × two cases (violation and clean):
/// 1. Horizontal notch, violation  (gap = `notch_dist + delta`)
/// 2. Horizontal notch, clean      (gap = `notch_dist`, exactly at the limit)
/// 3. Vertical notch, violation
/// 4. Vertical notch, clean
///
/// The outer size is derived as `max(notch_dist + 2 × thickness, 1.0)` so the shape
/// always fits the notch with one arm on each side. The notch is cut halfway along the
/// shape and halfway deep.
/// Pass `delta = -0.005` for the default ½-grid undershoot.
pub fn notch_pattern(
    layer: (i16, i16),
    thickness: f64,
    notch_dist: f64,
    shape_dist: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    let size = (notch_dist + 2.0 * thickness).max(1.0);
    let step = size + shape_dist;

    // Notch cut from the right side; arms run horizontally.
    let h_shape = |nd: f64, ox: f64| -> GdsElement {
        poly(layer, &[
            (ox,              0.0),
            (ox + size,       0.0),
            (ox + size,       thickness),
            (ox + size / 2.0, thickness),
            (ox + size / 2.0, thickness + nd),
            (ox + size,       thickness + nd),
            (ox + size,       size),
            (ox,              size),
        ])
    };

    // Notch cut from the top side; arms run vertically (90° rotation of h_shape).
    let v_shape = |nd: f64, ox: f64| -> GdsElement {
        poly(layer, &[
            (ox,                  0.0),
            (ox + size,           0.0),
            (ox + size,           size),
            (ox + thickness + nd, size),
            (ox + thickness + nd, size / 2.0),
            (ox + thickness,      size / 2.0),
            (ox + thickness,      size),
            (ox,                  size),
        ])
    };

    vec![
        h_shape(notch_dist + delta, offset),
        h_shape(notch_dist,         offset + step),
        v_shape(notch_dist + delta, offset + 2.0 * step),
        v_shape(notch_dist,         offset + 3.0 * step),
    ]
}

/// Generate a 4-shape mixed-orientation notch test pattern: a rectilinear wall facing
/// a *diagonal* wall of the same region, instead of the parallel walls `notch_pattern`
/// exercises.  This is the case a facing-wall scan restricted to same-orientation pairs
/// (two verticals, two horizontals, or two anti-parallel diagonals) cannot see at all —
/// e.g. a diagonal font-glyph stroke closing in on a straight stem.
///
/// Each shape is a "foot" sticking out from a taller body: a straight wall runs out to
/// `size`, steps up by the tested gap (`nd`), then a diagonal wall of slope
/// `diag_rise / (size / 2)` runs back up and away.  The straight wall and the diagonal
/// wall are connected only by that short vertical (resp. horizontal) step, so their
/// closest approach is exactly `nd` — independent of the diagonal's own run or angle.
///
/// Produces two orientations (a horizontal wall facing a diagonal, and its 90° rotation
/// — a vertical wall facing a diagonal) × two cases (violation and clean):
/// 1. Horizontal-vs-diagonal, violation  (gap = `notch_dist + delta`)
/// 2. Horizontal-vs-diagonal, clean      (gap = `notch_dist`, exactly at the limit)
/// 3. Vertical-vs-diagonal, violation
/// 4. Vertical-vs-diagonal, clean
///
/// The outer size is derived as `max(notch_dist + diag_rise + 2 × thickness, 1.0)` so
/// each arm stays at least `thickness` wide (clear of any `min_width` floor).
/// Pass `delta = -0.005` for the default ½-grid undershoot.
pub fn mixed_notch_pattern(
    layer: (i16, i16),
    thickness: f64,
    notch_dist: f64,
    diag_rise: f64,
    shape_dist: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    let size = (notch_dist + diag_rise + 2.0 * thickness).max(1.0);
    let step = size + shape_dist;

    // Straight wall runs along y = thickness (leftward from the outer edge); the
    // diagonal wall runs from the top of the `nd`-tall step back out to the right.
    let h_shape = |nd: f64, ox: f64| -> GdsElement {
        poly(layer, &[
            (ox,              0.0),
            (ox + size,       0.0),
            (ox + size,       thickness),
            (ox + size / 2.0, thickness),
            (ox + size / 2.0, thickness + nd),
            (ox + size,       thickness + nd + diag_rise),
            (ox + size,       size),
            (ox,              size),
        ])
    };

    // 90° rotation of h_shape (same [ox, ox+size] × [0, size] footprint and x-offset
    // convention): the straight wall is now vertical, facing a diagonal wall.
    let v_shape = |nd: f64, ox: f64| -> GdsElement {
        poly(layer, &[
            (ox,                              0.0),
            (ox + thickness,                  0.0),
            (ox + thickness,                  size / 2.0),
            (ox + thickness + nd,              size / 2.0),
            (ox + thickness + nd + diag_rise,  0.0),
            (ox + size,                       0.0),
            (ox + size,                       size),
            (ox,                              size),
        ])
    };

    vec![
        h_shape(notch_dist + delta, offset),
        h_shape(notch_dist,         offset + step),
        v_shape(notch_dist + delta, offset + 2.0 * step),
        v_shape(notch_dist,         offset + 3.0 * step),
    ]
}

/// Generate a 5-shape min-enclosure test pattern.
///
/// Each shape is an outer rect on `enclosing_layer` containing an inner rect on `enclosed_layer`.
/// The five cases, placed in a row starting at `offset`:
/// 1. Clean: exactly `enclosure` margin on all four sides
/// 2. Left violation:   left margin   = `enclosure + delta`
/// 3. Right violation:  right margin  = `enclosure + delta`
/// 4. Bottom violation: bottom margin = `enclosure + delta`
/// 5. Top violation:    top margin    = `enclosure + delta`
///
/// Pass `delta = -0.005` for the default ½-grid undershoot.
pub fn enclosure_pattern(
    enclosing_layer: (i16, i16),
    enclosed_layer: (i16, i16),
    enclosure: f64,
    width: f64,
    distance: f64,
    offset: f64,
    delta: f64,
) -> Vec<GdsElement> {
    let outer = width + 2.0 * enclosure;
    let step = outer.max(1.0) + distance;

    // Produce one (outer, width) rect pair. enc_* are the four enclosure margins.
    let pair = |ox: f64, enc_l: f64, enc_r: f64, enc_b: f64, enc_t: f64| {
        vec![
            rect(enclosing_layer, ox,           0.0,       ox + outer,           outer      ),
            rect(enclosed_layer,  ox + enc_l,   enc_b,     ox + outer - enc_r,   outer - enc_t),
        ]
    };

    let e = enclosure;
    let v = enclosure + delta;

    let mut elems = vec![];
    elems.extend(pair(offset,                e, e, e, e)); // clean
    elems.extend(pair(offset +       step,   v, e, e, e)); // left violation
    elems.extend(pair(offset + 2.0 * step,   e, v, e, e)); // right violation
    elems.extend(pair(offset + 3.0 * step,   e, e, v, e)); // bottom violation
    elems.extend(pair(offset + 4.0 * step,   e, e, e, v)); // top violation
    elems
}

/// Build a density test layout: a `size`×`size` µm boundary box (typically
/// `EdgeSeal`) plus a set of full-width horizontal stripes.  Each stripe is
/// `(layer, y0, y1)` in µm and spans the full width, so the measured density of a
/// layer group is simply the sum of its stripe heights over `size`.
///
/// Pass an empty `stripes` slice to get just the boundary box (then `extend` with
/// custom shapes, e.g. for windowed-density or merge tests).
pub fn density_pattern(
    boundary: (i16, i16),
    size: f64,
    stripes: &[((i16, i16), f64, f64)],
) -> Vec<GdsElement> {
    let mut elems = vec![rect(boundary, 0.0, 0.0, size, size)];
    for &(l, y0, y1) in stripes {
        elems.push(rect(l, 0.0, y0, size, y1));
    }
    elems
}

pub fn write_gz(path: &str, lib: GdsLibrary) {
    let out = std::fs::File::create(path).expect("failed to create file");
    let mut encoder = GzEncoder::new(out, Compression::best());
    lib.write(&mut encoder).expect("failed to write GDS");
    encoder.finish().expect("failed to finish gzip");
    println!("wrote {path}");
}

pub fn library(topcell: &str, elems: Vec<GdsElement>) -> GdsLibrary {
    let mut cell = GdsStruct::new(topcell);
    cell.elems = elems;

    let mut lib = GdsLibrary::new("LIB");
    lib.units = GdsUnits(1e-6, 1e-9); // 1 µm user unit, 1 nm DBU
    lib.structs = vec![cell];

    // Pin all GDS timestamps to a fixed epoch (1900-01-01 00:00:00) so regenerating
    // a fixture is byte-for-byte reproducible — otherwise `BGNLIB`/`BGNSTR` record
    // the current time and every regen churns every committed fixture.
    lib.set_all_dates(GdsDateTime::from(&[0i16, 1, 1, 0, 0, 0]));
    lib
}
