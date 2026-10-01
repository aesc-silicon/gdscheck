// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use gds21::{GdsBoundary, GdsPoint};
use std::collections::HashMap;

/// One shape of a [`Shapes`] set: a rectangle kept as its box, or any other polygon
/// as its points.  Either way [`Shape::xy`] gives the ring exactly as it was drawn -
/// the same first vertex, the same winding, the closing vertex repeated.
#[derive(Clone)]
pub enum Shape<'a> {
    Rect([GdsPoint; 5]),
    Poly(&'a [GdsPoint]),
}

impl Shape<'_> {
    pub fn xy(&self) -> &[GdsPoint] {
        match self {
            Shape::Rect(r) => r,
            Shape::Poly(p) => p,
        }
    }
}

/// The shapes of one layer, compact.  A flattened layer is mostly rectangles - every
/// contact, most wires - and a `GdsBoundary` holds one in some 130 bytes: the struct,
/// a vector of five points on the heap, the allocator's share.  Here a rectangle is its
/// two opposite corners, 16 bytes, and every other polygon is a run in one shared
/// point vector.  `order` keeps the shapes in the order they came, which is what the
/// merges and every report read them in: per shape its index into `rects` or `polys`,
/// shifted by two; bit 0 says a polygon, bit 1 which way a rectangle turns.
#[derive(Default)]
pub struct Shapes {
    order: Vec<u32>,
    rects: Vec<[i32; 4]>,
    points: Vec<GdsPoint>,
    /// Where each polygon's points start in `points`; one more entry than polygons.
    starts: Vec<usize>,
}

/// The most shapes of one kind `order` can index.
const MAX_INDEX: usize = (u32::MAX >> 2) as usize;

static NO_SHAPES: Shapes = Shapes {
    order: Vec::new(),
    rects: Vec::new(),
    points: Vec::new(),
    starts: Vec::new(),
};

impl Shapes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Add a ring, as drawn.  A closed five-point ring with axis-parallel sides and
    /// area is kept as a rectangle; anything else as its points.
    pub fn push(&mut self, xy: &[GdsPoint]) {
        if let Some((corners, turn)) = as_rect(xy)
            && self.rects.len() <= MAX_INDEX
        {
            self.order
                .push(((self.rects.len() as u32) << 2) | (turn << 1));
            self.rects.push(corners);
            return;
        }
        if self.starts.is_empty() {
            self.starts.push(0);
        }
        let poly = self.starts.len() - 1;
        assert!(
            poly <= MAX_INDEX,
            "more than {MAX_INDEX} polygons on one layer"
        );
        self.order.push(((poly as u32) << 2) | 1);
        self.points.extend_from_slice(xy);
        self.starts.push(self.points.len());
    }

    pub fn get(&self, i: usize) -> Shape<'_> {
        let e = self.order[i];
        let k = (e >> 2) as usize;
        if e & 1 == 1 {
            return Shape::Poly(&self.points[self.starts[k]..self.starts[k + 1]]);
        }
        let [x0, y0, x2, y2] = self.rects[k];
        // The second vertex shares its x with the first, or its y.
        let (x1, y1, x3, y3) = if e & 2 == 0 {
            (x0, y2, x2, y0)
        } else {
            (x2, y0, x0, y2)
        };
        Shape::Rect([
            GdsPoint::new(x0, y0),
            GdsPoint::new(x1, y1),
            GdsPoint::new(x2, y2),
            GdsPoint::new(x3, y3),
            GdsPoint::new(x0, y0),
        ])
    }

    pub fn iter(&self) -> impl Iterator<Item = Shape<'_>> + Clone + '_ {
        (0..self.len()).map(|i| self.get(i))
    }

    /// Every coordinate times `k`.
    fn scale(&mut self, k: i32) {
        for r in &mut self.rects {
            for c in r.iter_mut() {
                *c *= k;
            }
        }
        for p in &mut self.points {
            p.x *= k;
            p.y *= k;
        }
    }

    fn shrink_to_fit(&mut self) {
        self.order.shrink_to_fit();
        self.rects.shrink_to_fit();
        self.points.shrink_to_fit();
        self.starts.shrink_to_fit();
    }
}

impl<'a> FromIterator<&'a [GdsPoint]> for Shapes {
    fn from_iter<I: IntoIterator<Item = &'a [GdsPoint]>>(it: I) -> Self {
        let mut s = Shapes::new();
        for xy in it {
            s.push(xy);
        }
        s
    }
}

/// A closed ring of four axis-parallel sides around some area, as its first and third
/// vertex and whether the second shares the first's y (`1`) or its x (`0`).
fn as_rect(xy: &[GdsPoint]) -> Option<([i32; 4], u32)> {
    let [a, b, c, d, e] = xy else {
        return None;
    };
    if a != e || a.x == c.x || a.y == c.y {
        return None;
    }
    let turn = if b.x == a.x && b.y == c.y && d.x == c.x && d.y == a.y {
        0
    } else if b.y == a.y && b.x == c.x && d.y == c.y && d.x == a.x {
        1
    } else {
        return None;
    };
    Some(([a.x, a.y, c.x, c.y], turn))
}

/// A text label flattened to absolute DBU coordinates.
#[derive(Clone)]
pub struct Text {
    pub string: String,
    pub x: i32,
    pub y: i32,
}

/// Flattened GDS layout with boundaries indexed by (gds_layer, gds_datatype).
#[derive(Default)]
pub struct FlatLayout {
    layers: HashMap<(i16, i16), Shapes>,
    texts: HashMap<(i16, i16), Vec<Text>>,
    waived: Vec<WaivedInstance>,
    /// The box of every shape, in DBU, once something asked; cleared by an insert.
    /// Thirty density rules each walked ten million contacts to find the chip.
    bbox: std::sync::OnceLock<Option<(i64, i64, i64, i64)>>,
}

/// A placed instance of a cell a PDK waiver names: its cell and the bounding box of
/// its geometry in the flattened layout, in DBU.  A violation whose marker falls inside
/// is reported waived if the waiver covers its rule.
#[derive(Debug, Clone)]
pub struct WaivedInstance {
    pub cell: String,
    pub waiver: usize,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl FlatLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_waived_instance(&mut self, inst: WaivedInstance) {
        self.waived.push(inst);
    }

    pub fn waived_instances(&self) -> &[WaivedInstance] {
        &self.waived
    }

    /// Every coordinate times `k`: the layout read in a DBU `k` times finer.  Exact, as
    /// a multiple of a whole number is one.
    pub fn scale(&mut self, k: i32) {
        if k == 1 {
            return;
        }
        use rayon::prelude::*;
        self.layers
            .par_iter_mut()
            .for_each(|(_, shapes)| shapes.scale(k));
        for texts in self.texts.values_mut() {
            for t in texts {
                t.x *= k;
                t.y *= k;
            }
        }
        for w in &mut self.waived {
            w.x0 *= k;
            w.y0 *= k;
            w.x1 *= k;
            w.y1 *= k;
        }
        self.bbox = std::sync::OnceLock::new();
    }

    pub fn insert(&mut self, layer: i16, datatype: i16, boundary: GdsBoundary) {
        self.insert_xy(layer, datatype, &boundary.xy);
    }

    /// Add a ring, as drawn, to `(layer, datatype)`.
    pub fn insert_xy(&mut self, layer: i16, datatype: i16, xy: &[GdsPoint]) {
        self.layers.entry((layer, datatype)).or_default().push(xy);
        self.bbox = std::sync::OnceLock::new();
    }

    /// Give back what the layers' vectors hold beyond their shapes, once all are in.
    pub fn shrink_to_fit(&mut self) {
        for shapes in self.layers.values_mut() {
            shapes.shrink_to_fit();
        }
    }

    /// The box of every shape in the layout, `(x0, y0, x1, y1)` in DBU, or `None` when
    /// it holds none.
    pub fn bbox(&self) -> Option<(i64, i64, i64, i64)> {
        *self.bbox.get_or_init(|| {
            let mut b = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for (_, s) in self.all_boundaries() {
                for p in s.xy() {
                    b.0 = b.0.min(p.x as i64);
                    b.1 = b.1.min(p.y as i64);
                    b.2 = b.2.max(p.x as i64);
                    b.3 = b.3.max(p.y as i64);
                }
            }
            (b.0 != i64::MAX).then_some(b)
        })
    }

    pub fn insert_text(&mut self, layer: i16, texttype: i16, text: Text) {
        self.texts.entry((layer, texttype)).or_default().push(text);
    }

    /// All text labels on the given layer/texttype.
    pub fn texts(&self, layer: i16, texttype: i16) -> &[Text] {
        self.texts
            .get(&(layer, texttype))
            .map_or(&[], Vec::as_slice)
    }

    /// All shapes on the given layer/datatype.
    pub fn get(&self, layer: i16, datatype: i16) -> &Shapes {
        self.layers.get(&(layer, datatype)).unwrap_or(&NO_SHAPES)
    }

    /// Iterate all shapes across all layers, with their layer/datatype.
    pub fn all_boundaries(&self) -> impl Iterator<Item = ((i16, i16), Shape<'_>)> {
        self.layers
            .iter()
            .flat_map(|(&k, v)| v.iter().map(move |s| (k, s)))
    }

    /// Iterate all shapes on all layers except the specified one, with their
    /// layer/datatype.
    pub fn all_except(
        &self,
        layer: i16,
        datatype: i16,
    ) -> impl Iterator<Item = ((i16, i16), Shape<'_>)> {
        self.all_boundaries()
            .filter(move |&((l, d), _)| l != layer || d != datatype)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(pts: &[(i32, i32)]) -> Vec<GdsPoint> {
        pts.iter().map(|&(x, y)| GdsPoint::new(x, y)).collect()
    }

    /// Every ring comes back as drawn, rectangle or not, in the order it went in:
    /// both windings from every corner, a box drawn without its closing vertex, a
    /// line, a triangle.
    #[test]
    fn shapes_come_back_as_drawn() {
        let corners = [(0, 0), (10, 0), (10, 5), (0, 5)];
        let mut rings = vec![];
        for start in 0..4 {
            for rev in [false, true] {
                let mut r: Vec<(i32, i32)> = (0..4).map(|i| corners[(start + i) % 4]).collect();
                if rev {
                    r[1..].reverse();
                }
                r.push(r[0]);
                rings.push(ring(&r));
            }
        }
        rings.push(ring(&[(0, 0), (10, 0), (10, 5), (0, 5)]));
        rings.push(ring(&[(0, 0), (10, 0), (10, 0), (0, 0), (0, 0)]));
        rings.push(ring(&[(0, 0), (10, 0), (0, 5), (0, 0)]));
        let shapes: Shapes = rings.iter().map(Vec::as_slice).collect();
        assert_eq!(shapes.rects.len(), 8);
        let back: Vec<Vec<GdsPoint>> = shapes.iter().map(|s| s.xy().to_vec()).collect();
        assert_eq!(back, rings);
    }
}
