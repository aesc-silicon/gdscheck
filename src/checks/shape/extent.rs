// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounding-box extents of a feature such as a contact bar or a slot.
//!
//! A feature has two principal sizes: its **width** (the short side of its box) and its
//! **length** (the long side).  These checks bound one of them, so a contact bar can
//! require an exact width *and* a minimum length without the width rule tripping on the
//! length - which the facing-wall scan of the width family cannot distinguish.  Exact for
//! axis-aligned features; a 45° bar's box is not the bar.

use super::Kind;
use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Which side of a feature's box a rule bounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    /// The short side: `min_dim`, `max_dim`, `exact_dim`.
    Short,
    /// The long side: `min_length`, `max_length`, `exact_length`.
    Long,
}

impl Axis {
    /// The check's name under `kind`, as the deck spells it.
    pub fn name(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Axis::Short, Kind::Min) => "min_dim",
            (Axis::Short, Kind::Max) => "max_dim",
            (Axis::Short, Kind::Exact) => "exact_dim",
            (Axis::Long, Kind::Min) => "min_length",
            (Axis::Long, Kind::Max) => "max_length",
            (Axis::Long, Kind::Exact) => "exact_length",
        }
    }
}

/// Drive a bounding-box extent check over the layer's stitched regions: one point
/// violation per offending region, at the region's marker.  The box is read whole
/// across tiles by [`super::region_boxes`]; an extent is a difference of two
/// coordinates, and the bound on the grid reads it exactly.
pub fn run(
    kind: Kind,
    axis: Axis,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] {} needs a layer", rule.id, axis.name(kind));
        return vec![];
    };
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);

    println!(
        "[{}] Checking {} {} {:.2} µm on layer {}",
        rule.id,
        axis.name(kind),
        kind.op(),
        rule.value,
        layer.name
    );

    let rid = rule.id.as_str();
    let lname = layer.name.as_str();
    let limit_um = rule.value;
    let limit = kind.limit(rule.value, dbu_to_um);
    let long = axis == Axis::Long;
    let word = if long { "length" } else { "width" };
    let cmp = kind.cmp();
    let label = match axis {
        Axis::Short => format!("{} width violation", kind.word()),
        Axis::Long => format!("{} length violation", kind.word()),
    };

    super::region_boxes(merged, gl, gd)
        .into_iter()
        .filter_map(|((cx, cy), (x0, y0, x1, y1))| {
            let (w, h) = ((x1 - x0) as i64, (y1 - y0) as i64);
            let extent = if long { w.max(h) } else { w.min(h) };
            if !limit.broken_by(extent) {
                return None;
            }
            let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
            Some(Violation::point(
                rid,
                &label,
                format!(
                    "{}: {} {:.4} µm {} {:.4} µm at ({:.4}, {:.4}) µm",
                    lname,
                    word,
                    extent as f64 * dbu_to_um,
                    cmp,
                    limit_um,
                    ux,
                    uy
                ),
                ux,
                uy,
            ))
        })
        .collect()
}
