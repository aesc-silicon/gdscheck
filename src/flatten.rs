// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hierarchy flattening: convert a GDS cell tree into a `FlatLayout` of
//! `GdsBoundary` elements by resolving all `GdsStructRef` and `GdsArrayRef`
//! references and applying the accumulated coordinate transformation.

use crate::layout::FlatLayout;
use gds21::{GdsBoundary, GdsElement, GdsLibrary, GdsPath, GdsPoint, GdsStrans, GdsStruct};
use std::collections::{HashMap, HashSet};

/// Layers to keep while flattening: `None` keeps everything; `Some(set)` keeps
/// only the listed `(layer, datatype)` pairs, so a deck that touches a handful of
/// layers doesn't instantiate the entire (possibly enormous) hierarchy.
type Needed<'a> = Option<&'a HashSet<(i16, i16)>>;

#[inline]
fn wanted(needed: Needed, layer: i16, datatype: i16) -> bool {
    needed.is_none_or(|n| n.contains(&(layer, datatype)))
}

/// Affine transform represented as a 2×2 linear matrix plus a translation.
/// Encodes the composition of magnification, x-axis reflection, and
/// counter-clockwise rotation that GDS transformations allow.
struct Transform {
    /// [ [a00, a01], [a10, a11] ] — linear (rotation/reflection/scale) part
    a: [[f64; 2]; 2],
    /// Translation in database units (DBU)
    tx: f64,
    ty: f64,
}

impl Transform {
    fn identity() -> Self {
        Transform { a: [[1.0, 0.0], [0.0, 1.0]], tx: 0.0, ty: 0.0 }
    }

    /// Build from an optional `GdsStrans` and a translation (tx, ty) in DBU.
    ///
    /// GDS transformation order (per spec):
    ///   1. Magnification
    ///   2. Reflection about x-axis (negate y)
    ///   3. Rotation CCW by `angle` degrees
    ///   4. Translation
    fn from_strans(strans: Option<&GdsStrans>, tx: i32, ty: i32) -> Self {
        let mag = strans.and_then(|s| s.mag).unwrap_or(1.0);
        let angle = strans.and_then(|s| s.angle).unwrap_or(0.0);
        let reflect_x = strans.map(|s| s.reflected).unwrap_or(false);

        let rad = angle.to_radians();
        let (sin_a, cos_a) = rad.sin_cos();
        let ry = if reflect_x { -1.0_f64 } else { 1.0_f64 };

        // Column vectors after applying (mag → reflect → rotate):
        //   e₁ = [1,0] → mag·[cos, sin]
        //   e₂ = [0,1] → mag·ry·[−sin, cos]  (ry flips y before rotation)
        Transform {
            a: [
                [mag * cos_a, -mag * ry * sin_a],
                [mag * sin_a,  mag * ry * cos_a],
            ],
            tx: tx as f64,
            ty: ty as f64,
        }
    }

    /// Apply this transform to a single DBU point.
    fn apply(&self, x: i32, y: i32) -> GdsPoint {
        let xf = x as f64;
        let yf = y as f64;
        GdsPoint {
            x: (self.a[0][0] * xf + self.a[0][1] * yf + self.tx).round() as i32,
            y: (self.a[1][0] * xf + self.a[1][1] * yf + self.ty).round() as i32,
        }
    }

    /// Compose: `self` is the outer (parent) transform, `inner` is the child.
    /// Returns T such that `T.apply(p) == self.apply(inner.apply(p))`.
    fn compose(&self, inner: &Transform) -> Transform {
        // T(p) = A_self · (A_inner · p + t_inner) + t_self
        //      = (A_self · A_inner) · p + (A_self · t_inner + t_self)
        let a = [
            [
                self.a[0][0] * inner.a[0][0] + self.a[0][1] * inner.a[1][0],
                self.a[0][0] * inner.a[0][1] + self.a[0][1] * inner.a[1][1],
            ],
            [
                self.a[1][0] * inner.a[0][0] + self.a[1][1] * inner.a[1][0],
                self.a[1][0] * inner.a[0][1] + self.a[1][1] * inner.a[1][1],
            ],
        ];
        let tx = self.a[0][0] * inner.tx + self.a[0][1] * inner.ty + self.tx;
        let ty = self.a[1][0] * inner.tx + self.a[1][1] * inner.ty + self.ty;
        Transform { a, tx, ty }
    }
}

fn flatten_cell(
    cell_name: &str,
    cell_map: &HashMap<&str, &GdsStruct>,
    transform: &Transform,
    depth: u32,
    needed: Needed,
    out: &mut FlatLayout,
) {
    if depth > 64 {
        eprintln!("flatten: depth limit reached inside cell '{}'", cell_name);
        return;
    }
    let Some(cell) = cell_map.get(cell_name) else {
        eprintln!("flatten: cell '{}' not found in library", cell_name);
        return;
    };

    for elem in &cell.elems {
        match elem {
            GdsElement::GdsBoundary(b) if wanted(needed, b.layer, b.datatype) => {
                let new_xy: Vec<GdsPoint> =
                    b.xy.iter().map(|p| transform.apply(p.x, p.y)).collect();
                out.insert(b.layer, b.datatype, GdsBoundary {
                    layer: b.layer,
                    datatype: b.datatype,
                    xy: new_xy,
                    ..Default::default()
                });
            }
            GdsElement::GdsPath(p) if wanted(needed, p.layer, p.datatype) => {
                add_path(p, transform, out)
            }
            GdsElement::GdsBox(b) if wanted(needed, b.layer, b.boxtype) => {
                // A BOX is a rectangle; BOXTYPE plays the role of the datatype.
                let xy: Vec<GdsPoint> = b.xy.iter().map(|p| transform.apply(p.x, p.y)).collect();
                out.insert(b.layer, b.boxtype, GdsBoundary {
                    layer: b.layer,
                    datatype: b.boxtype,
                    xy,
                    ..Default::default()
                });
            }
            GdsElement::GdsStructRef(sr) => {
                let child_tr =
                    Transform::from_strans(sr.strans.as_ref(), sr.xy.x, sr.xy.y);
                let composed = transform.compose(&child_tr);
                flatten_cell(&sr.name, cell_map, &composed, depth + 1, needed, out);
            }
            GdsElement::GdsArrayRef(ar) => {
                let cols = ar.cols as i32;
                let rows = ar.rows as i32;
                // Per GDS spec:
                //   xy[1] = origin + cols × col_step
                //   xy[2] = origin + rows × row_step
                let col_dx = (ar.xy[1].x - ar.xy[0].x) / cols;
                let col_dy = (ar.xy[1].y - ar.xy[0].y) / cols;
                let row_dx = (ar.xy[2].x - ar.xy[0].x) / rows;
                let row_dy = (ar.xy[2].y - ar.xy[0].y) / rows;
                for c in 0..cols {
                    for r in 0..rows {
                        let ix = ar.xy[0].x + c * col_dx + r * row_dx;
                        let iy = ar.xy[0].y + c * col_dy + r * row_dy;
                        let child_tr =
                            Transform::from_strans(ar.strans.as_ref(), ix, iy);
                        let composed = transform.compose(&child_tr);
                        flatten_cell(&ar.name, cell_map, &composed, depth + 1, needed, out);
                    }
                }
            }
            GdsElement::GdsTextElem(t) if wanted(needed, t.layer, t.texttype) => {
                let p = transform.apply(t.xy.x, t.xy.y);
                out.insert_text(t.layer, t.texttype, crate::layout::Text {
                    string: t.string.clone(),
                    x: p.x,
                    y: p.y,
                });
            }
            _ => {} // node, or an element on an unwanted layer
        }
    }
}

/// Convert a `GdsPath` (a centreline with a width) into filled rectangles — one
/// per segment — inserted as boundaries so the downstream merge unions them into
/// the full ribbon.  Each interior joint is extended by a half-width so abutting
/// segment rectangles overlap and fill the corner with no gap; the two true ends
/// honour `path_type` (0 = flush, 1/2 = extended by a half-width, 4 = explicit
/// `begin_extn`/`end_extn`).  Rounded ends (type 1) are approximated by their
/// bounding square, a conservative superset that never under-fills.
fn add_path(path: &GdsPath, transform: &Transform, out: &mut FlatLayout) {
    let Some(width) = path.width else { return }; // zero-width path encloses no area
    if width == 0 || path.xy.len() < 2 {
        return;
    }
    let hw = width.unsigned_abs() as f64 / 2.0;
    let pts: Vec<(f64, f64)> = path.xy.iter().map(|p| (p.x as f64, p.y as f64)).collect();
    let n = pts.len();
    let ptype = path.path_type.unwrap_or(0);
    let cap = |is_begin: bool| -> f64 {
        match ptype {
            1 | 2 => hw,
            4 => {
                let e = if is_begin { path.begin_extn } else { path.end_extn };
                e.unwrap_or(0) as f64
            }
            _ => 0.0,
        }
    };

    for i in 0..n - 1 {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[i + 1];
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy);
        if len == 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len); // along the segment
        let (nx, ny) = (-uy, ux); // perpendicular (left)
        let ext_a = if i == 0 { cap(true) } else { hw };
        let ext_b = if i + 1 == n - 1 { cap(false) } else { hw };
        let (sax, say) = (ax - ux * ext_a, ay - uy * ext_a);
        let (sbx, sby) = (bx + ux * ext_b, by + uy * ext_b);
        let corners = [
            (sax + nx * hw, say + ny * hw),
            (sbx + nx * hw, sby + ny * hw),
            (sbx - nx * hw, sby - ny * hw),
            (sax - nx * hw, say - ny * hw),
        ];
        // Repeat the first corner so the ring is closed, matching the GDS
        // boundary convention the merge expects (it drops the final vertex).
        let xy: Vec<GdsPoint> = corners
            .iter()
            .chain(std::iter::once(&corners[0]))
            .map(|&(x, y)| transform.apply(x.round() as i32, y.round() as i32))
            .collect();
        out.insert(path.layer, path.datatype, GdsBoundary {
            layer: path.layer,
            datatype: path.datatype,
            xy,
            ..Default::default()
        });
    }
}

/// Flatten the named top cell from `lib` into a `FlatLayout` of `GdsBoundary`
/// variants in top-cell coordinates.  `needed` restricts which `(layer,
/// datatype)` pairs are kept (`None` keeps all) — scoping a flatten to the layers
/// a deck actually uses keeps a large hierarchy from blowing up memory.
pub fn flatten_to_elems(topcell: &str, lib: &GdsLibrary, needed: Needed) -> FlatLayout {
    let cell_map: HashMap<&str, &GdsStruct> =
        lib.structs.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut out = FlatLayout::new();
    flatten_cell(topcell, &cell_map, &Transform::identity(), 0, needed, &mut out);
    out
}
