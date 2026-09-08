// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use gds21::GdsBoundary;
use std::collections::HashMap;

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
    layers: HashMap<(i16, i16), Vec<GdsBoundary>>,
    texts: HashMap<(i16, i16), Vec<Text>>,
    waived: Vec<WaivedInstance>,
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

    pub fn insert(&mut self, layer: i16, datatype: i16, boundary: GdsBoundary) {
        self.layers
            .entry((layer, datatype))
            .or_default()
            .push(boundary);
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

    /// All boundaries on the given layer/datatype.
    pub fn get(&self, layer: i16, datatype: i16) -> &[GdsBoundary] {
        self.layers
            .get(&(layer, datatype))
            .map_or(&[], Vec::as_slice)
    }

    /// Iterate all boundaries across all layers.
    pub fn all_boundaries(&self) -> impl Iterator<Item = &GdsBoundary> {
        self.layers.values().flatten()
    }

    /// Iterate all boundaries on all layers except the specified one.
    pub fn all_except(&self, layer: i16, datatype: i16) -> impl Iterator<Item = &GdsBoundary> {
        self.layers
            .iter()
            .filter(move |&(&(l, d), _)| l != layer || d != datatype)
            .flat_map(|(_, v)| v.iter())
    }
}
