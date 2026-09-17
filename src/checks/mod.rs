// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The checks, one module per family of measurement.  A rule names its check and the
//! family reads the bound, the scope and the gates from its params; `run_rule` is the
//! table from check name to family.  In the order a rule deck tends to be read:
//!
//! - [`width`]: the material between two walls of one region.
//! - [`space`]: the gap between two regions, or between two walls of one (a notch), and
//!   the reach nothing may lie beyond.
//! - [`enclosure`]: the margin of one region inside another.
//! - [`shape`]: a region's own extents, edges, corners, holes, vertices and grid.
//! - [`area`]: a region's area.
//! - [`density`]: the coverage of layers over an area.
//! - [`net`]: what only the connect graph can answer.
//! - [`residual`]: what a rule forbids outright.
//!
//! [`edge_distance`] is the spacing and enclosure read between *edge* layers, which
//! `space` and `enclosure` hand a rule to when its layers are edges; [`helper`] holds
//! the tile drivers the families share and [`params`] the readers of a rule's params.

pub mod area;
pub mod density;
pub mod edge_distance;
pub mod enclosure;
pub mod helper;
pub mod net;
pub mod params;
pub mod residual;
pub mod shape;
pub mod space;
pub mod width;

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::{Layer, RuleDefinition};
use crate::violation::Violation;
use gds21::GdsBoundary;

/// Whether a rule reads its layers from the flat layout rather than the tiled cache,
/// and so sees a virtual layer only if it was materialised there first: the ring and
/// vertex readings of a shape, and a `forbidden` past a boundary.
pub fn reads_layout(rule: &RuleDefinition) -> bool {
    matches!(
        rule.check.as_str(),
        "no_ring" | "ring_covers_boundary" | "max_vertices"
    ) || residual::whole_layout(rule)
}

/// Returns a slice of all boundaries on the given layer.
pub fn boundaries_on<'a>(layout: &'a FlatLayout, layer: &Layer) -> &'a [GdsBoundary] {
    layout.get(layer.gds_layer as i16, layer.gds_datatype as i16)
}

pub fn run_rule(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&crate::connectivity::Connectivity>,
) -> Vec<Violation> {
    let mut out = match rule.check.as_str() {
        // Width: the material between two walls of one region.
        "min_width" => width::run(width::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_width" => width::run(width::Kind::Max, rule, layout, dbu_to_um, merged),
        "exact_width" => width::run(width::Kind::Exact, rule, layout, dbu_to_um, merged),
        "min_gate_length" => width::run_gate(width::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_gate_length" => width::run_gate(width::Kind::Max, rule, layout, dbu_to_um, merged),
        "exact_gate_length" => width::run_gate(width::Kind::Exact, rule, layout, dbu_to_um, merged),
        // Space: the gap between two regions, or two walls of one.
        "min_space" => space::run_min(rule, layout, dbu_to_um, merged, conn),
        "min_notch" => space::notch::run(rule, layout, dbu_to_um, merged),
        "min_overlap" => space::run_min_overlap(rule, layout, dbu_to_um, merged),
        "max_space" => space::max::run(rule, layout, dbu_to_um, merged),
        // Enclosure: the margin of one region inside another.
        "min_enclosure" => enclosure::run(enclosure::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_enclosure" => enclosure::run(enclosure::Kind::Max, rule, layout, dbu_to_um, merged),
        // Shape: a region's own extents, edges, corners, holes and vertices.
        "min_dim" => shape::extent::run(
            shape::Kind::Min,
            shape::extent::Axis::Short,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "max_dim" => shape::extent::run(
            shape::Kind::Max,
            shape::extent::Axis::Short,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "exact_dim" => shape::extent::run(
            shape::Kind::Exact,
            shape::extent::Axis::Short,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "min_length" => shape::extent::run(
            shape::Kind::Min,
            shape::extent::Axis::Long,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "max_length" => shape::extent::run(
            shape::Kind::Max,
            shape::extent::Axis::Long,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "exact_length" => shape::extent::run(
            shape::Kind::Exact,
            shape::extent::Axis::Long,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "min_edge_length" => {
            shape::edge_length::run(shape::Kind::Min, rule, layout, dbu_to_um, merged)
        }
        "max_edge_length" => {
            shape::edge_length::run(shape::Kind::Max, rule, layout, dbu_to_um, merged)
        }
        "exact_edge_length" => {
            shape::edge_length::run(shape::Kind::Exact, rule, layout, dbu_to_um, merged)
        }
        "no_angle" => shape::angle::run(rule, layout, dbu_to_um, merged),
        "no_corner" => shape::corner::run(rule, layout, dbu_to_um, merged),
        "no_hole" => shape::holes::run(rule, layout, dbu_to_um, merged),
        "no_ring" => shape::ring::run(rule, layout, dbu_to_um),
        "ring_covers_boundary" => shape::ring_covers_boundary::run(rule, layout, dbu_to_um),
        "max_vertices" => shape::vertices::run(rule, layout, dbu_to_um),
        "offgrid" => shape::offgrid::run(rule, layout, dbu_to_um, merged),
        "wide_uncovered" => shape::wide_uncovered::run(rule, layout, dbu_to_um, merged),
        // Area: a region's area, per region, hole, containment or chip.
        "min_area" => area::run(area::Kind::Min, rule, layout, dbu_to_um, merged, conn),
        "max_area" => area::run(area::Kind::Max, rule, layout, dbu_to_um, merged, conn),
        "exact_area" => area::run(area::Kind::Exact, rule, layout, dbu_to_um, merged, conn),
        // Density: the coverage of layers over the chip, a window or a region.
        "min_density" => density::run(density::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_density" => density::run(density::Kind::Max, rule, layout, dbu_to_um, merged),
        // Net: what only the connect graph can answer.
        "antenna_ratio" => net::antenna::run(rule, layout, dbu_to_um, merged, conn),
        "max_nets_under" => net::nets_under::run(rule, layout, dbu_to_um, merged, conn),
        // Residual: what a rule forbids outright.
        "forbidden" => residual::run(rule, layout, dbu_to_um, merged),
        other => {
            eprintln!("[{}] Unknown check function: '{}'", rule.id, other);
            vec![]
        }
    };
    // `layer_params: {interacting: X}` keeps only the violations that touch X - KLayout's
    // trailing `.interacting(layer)` on a check's result.  GF180's CUP.2 measures the
    // width of the metal connected to a bond pad, but only where the measurement meets
    // the pad: a millimetre of 0.3 µm line that happens to be on the pad's net is the
    // line's own business.
    if let (Some(l), Some(d)) = (rule.num("interacting"), rule.num("interacting_dt")) {
        let key = (l as i16, d as i16);
        merged.ensure(layout, key.0, key.1);
        let tiles = merged.tiles(key.0, key.1);
        let t = merged.tile_dbu() as f64 * dbu_to_um;
        let touches = |x: f64, y: f64| {
            let tile = ((x / t).floor() as i32, (y / t).floor() as i32);
            let (xd, yd) = (x / dbu_to_um, y / dbu_to_um);
            tiles
                .get(&tile)
                .is_some_and(|ps| ps.iter().any(|p| crate::merge::point_in_merged(xd, yd, p)))
        };
        let trace = std::env::var("GDSCHECK_RULE_TRACE").is_ok();
        out.retain(|v| {
            let keep = match v.geometry {
                crate::violation::ViolationGeometry::Point { x, y } => touches(x, y),
                crate::violation::ViolationGeometry::Edge { x1, y1, x2, y2 } => (0..=4).any(|i| {
                    let f = i as f64 / 4.0;
                    touches(x1 + (x2 - x1) * f, y1 + (y2 - y1) * f)
                }),
                crate::violation::ViolationGeometry::None => true,
            };
            if !keep && trace {
                eprintln!(
                    "dropped (not interacting {}/{}): {}",
                    key.0, key.1, v.message
                );
            }
            keep
        });
    }
    out
}
