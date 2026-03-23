// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod area;
pub mod density;
pub mod helper;
pub mod coverage;
pub mod extent;
pub mod forbidden_overlap;
pub mod min_array_space;
pub mod exact_width;
pub mod inside_boundary;
pub mod no_angle;
pub mod gate_length;
pub mod min_region_density;
pub mod wide_uncovered;
pub mod no_ring;
pub mod ring_covers_boundary;
pub mod forbidden;
pub mod max_total_area;
pub mod antenna_ratio;
pub mod forbidden_unless_labeled;
pub mod gate_connected_min_area;
pub mod must_interact;
pub mod nonempty;
pub mod min_enclosed_area;
pub mod windowed_density;
pub mod max_width;
pub mod min_45_width;
pub mod min_enclosure;
pub mod max_enclosure;
pub mod min_endcap_enclosure;
pub mod min_extension;
pub mod min_notch;
pub mod min_space;
pub mod max_space;
pub mod min_space_bent;
pub mod min_space_prl;
pub mod min_width;
pub mod offgrid;

use crate::cache::Cache;
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
    cache: &mut Cache,
    merged: &mut MergedCache,
    conn: Option<&crate::connectivity::Connectivity>,
) -> Vec<Violation> {
    match rule.check.as_str() {
        "antenna_ratio"         => antenna_ratio::run(rule, layout, dbu_to_um, merged, conn),
        "gate_connected_min_area" => gate_connected_min_area::run(rule, layout, dbu_to_um, merged, conn),
        "forbidden_unless_labeled" => forbidden_unless_labeled::run(rule, layout, dbu_to_um, merged),
        "exact_width"           => exact_width::run(rule, layout, dbu_to_um, merged),
        "forbidden"             => forbidden::run(rule, layout, dbu_to_um),
        "forbidden_overlap"     => forbidden_overlap::run(rule, layout, dbu_to_um, merged),
        "coverage"              => coverage::run(rule, layout, dbu_to_um, merged),
        "min_dim"               => extent::run_min_width(rule, layout, dbu_to_um, merged),
        "max_dim"               => extent::run_max_width(rule, layout, dbu_to_um, merged),
        "min_length"            => extent::run_min_length(rule, layout, dbu_to_um, merged),
        "max_length"            => extent::run_max_length(rule, layout, dbu_to_um, merged),
        "inside_boundary"       => inside_boundary::run(rule, layout, dbu_to_um),
        "ring_covers_boundary"  => ring_covers_boundary::run(rule, layout, dbu_to_um),
        "min_density"           => density::run_min(rule, layout, dbu_to_um, cache, merged),
        "max_density"           => density::run_max(rule, layout, dbu_to_um, cache, merged),
        "min_area"              => area::run_min(rule, layout, dbu_to_um, merged),
        "must_interact"         => must_interact::run(rule, layout, dbu_to_um),
        "nonempty"              => nonempty::run(rule, layout, dbu_to_um, merged),
        "min_enclosed_area"     => min_enclosed_area::run(rule, layout, dbu_to_um, merged),
        "max_area"              => area::run_max(rule, layout, dbu_to_um, merged),
        "max_total_area"        => max_total_area::run(rule, layout, dbu_to_um, merged),
        "no_angle"              => no_angle::run(rule, layout, dbu_to_um, merged),
        "gate_length"           => gate_length::run(rule, layout, dbu_to_um, merged),
        "min_region_density"    => min_region_density::run(rule, layout, dbu_to_um, merged),
        "wide_uncovered"        => wide_uncovered::run(rule, layout, dbu_to_um, merged),
        "no_ring"               => no_ring::run(rule, layout, dbu_to_um),
        "min_enclosure"         => min_enclosure::run(rule, layout, dbu_to_um, merged),
        "max_enclosure"         => max_enclosure::run(rule, layout, dbu_to_um, merged),
        "min_endcap_enclosure"  => min_endcap_enclosure::run(rule, layout, dbu_to_um, merged),
        "min_extension"         => min_extension::run(rule, layout, dbu_to_um, merged),
        "min_notch"             => min_notch::run(rule, layout, dbu_to_um, merged),
        "min_space"             => min_space::run(rule, layout, dbu_to_um, merged),
        "max_space"             => max_space::run(rule, layout, dbu_to_um, merged),
        "min_space_bent"        => min_space_bent::run(rule, layout, dbu_to_um, merged),
        "min_array_space"       => min_array_space::run(rule, layout, dbu_to_um, merged),
        "min_space_prl"         => min_space_prl::run(rule, layout, dbu_to_um, merged),
        "min_45_width"          => min_45_width::run(rule, layout, dbu_to_um, merged),
        "min_width"             => min_width::run(rule, layout, dbu_to_um, merged),
        "max_width"             => max_width::run(rule, layout, dbu_to_um, merged),
        "min_windowed_density"  => windowed_density::run_min(rule, layout, dbu_to_um, merged),
        "max_windowed_density"  => windowed_density::run_max(rule, layout, dbu_to_um, merged),
        "offgrid"               => offgrid::run(rule, layout, dbu_to_um),
        other => {
            eprintln!("[{}] Unknown check function: '{}'", rule.id, other);
            vec![]
        }
    }
}
