// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod area;
pub mod coverage;
pub mod density;
pub mod edge_distance;
pub mod enclosure;
pub mod forbidden;
pub mod forbidden_overlap;
pub mod forbidden_unless_labeled;
pub mod helper;
pub mod inside_boundary;
pub mod min_array_space;
pub mod min_via_array;
pub mod must_interact;
pub mod net;
pub mod nonempty;
pub mod params;
pub mod shape;
pub mod space;
pub mod width;

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::{Layer, RuleDefinition};
use crate::violation::Violation;
use gds21::GdsBoundary;

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
        "antenna_ratio" => net::antenna::run(rule, layout, dbu_to_um, merged, conn),
        "forbidden_unless_labeled" => {
            forbidden_unless_labeled::run(rule, layout, dbu_to_um, merged)
        }
        "exact_width" => width::run(width::Kind::Exact, rule, layout, dbu_to_um, merged),
        "forbidden" => forbidden::run(rule, layout, dbu_to_um),
        "forbidden_overlap" => forbidden_overlap::run(rule, layout, dbu_to_um, merged),
        "coverage" => coverage::run(rule, layout, dbu_to_um, merged),
        "min_edge_length" => {
            shape::edge_length::run(shape::Kind::Min, rule, layout, dbu_to_um, merged)
        }
        "max_edge_length" => {
            shape::edge_length::run(shape::Kind::Max, rule, layout, dbu_to_um, merged)
        }
        "exact_edge_length" => {
            shape::edge_length::run(shape::Kind::Exact, rule, layout, dbu_to_um, merged)
        }
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
        "inside_boundary" => inside_boundary::run(rule, layout, dbu_to_um),
        "ring_covers_boundary" => shape::ring_covers_boundary::run(rule, layout, dbu_to_um),
        "min_density" => density::run(
            density::Kind::Min,
            density::Scope::Chip,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "max_density" => density::run(
            density::Kind::Max,
            density::Scope::Chip,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "min_area" => area::run(area::Kind::Min, rule, layout, dbu_to_um, merged, conn),
        "exact_area" => area::run(area::Kind::Exact, rule, layout, dbu_to_um, merged, conn),
        "must_interact" => must_interact::run(rule, layout, dbu_to_um),
        "nonempty" => nonempty::run(rule, layout, dbu_to_um, merged),
        "max_area" => area::run(area::Kind::Max, rule, layout, dbu_to_um, merged, conn),
        "max_nets_under" => net::nets_under::run(rule, layout, dbu_to_um, merged, conn),
        "no_angle" => shape::angle::run(rule, layout, dbu_to_um, merged),
        "no_corner" => shape::corner::run(rule, layout, dbu_to_um, merged),
        "no_hole" => shape::holes::run(rule, layout, dbu_to_um, merged),
        "max_vertices" => shape::vertices::run(rule, layout, dbu_to_um),
        "min_gate_length" => width::run_gate(width::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_gate_length" => width::run_gate(width::Kind::Max, rule, layout, dbu_to_um, merged),
        "exact_gate_length" => width::run_gate(width::Kind::Exact, rule, layout, dbu_to_um, merged),
        "min_region_density" => density::region::run(rule, layout, dbu_to_um, merged),
        "wide_uncovered" => shape::wide_uncovered::run(rule, layout, dbu_to_um, merged),
        "no_ring" => shape::ring::run(rule, layout, dbu_to_um),
        "min_enclosure" => enclosure::run(enclosure::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_enclosure" => enclosure::run(enclosure::Kind::Max, rule, layout, dbu_to_um, merged),
        "min_notch" => space::notch::run(rule, layout, dbu_to_um, merged),
        "min_overlap" => space::run_min_overlap(rule, layout, dbu_to_um, merged),
        "min_space" => space::run_min(rule, layout, dbu_to_um, merged, conn),
        "max_space" => space::max::run(rule, layout, dbu_to_um, merged),
        "min_via_array" => min_via_array::run(rule, layout, dbu_to_um, merged),
        "min_array_space" => min_array_space::run(rule, layout, dbu_to_um, merged),
        "min_width" => width::run(width::Kind::Min, rule, layout, dbu_to_um, merged),
        "max_width" => width::run(width::Kind::Max, rule, layout, dbu_to_um, merged),
        "min_windowed_density" => density::run(
            density::Kind::Min,
            density::Scope::Window,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "max_windowed_density" => density::run(
            density::Kind::Max,
            density::Scope::Window,
            rule,
            layout,
            dbu_to_um,
            merged,
        ),
        "offgrid" => shape::offgrid::run(rule, layout, dbu_to_um, merged),
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
