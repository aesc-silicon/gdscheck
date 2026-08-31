// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum width: the perpendicular span of the metal must be at least the rule
//! value.  Runs the shared facing-edge width scan over the merged tiles (see
//! [`helper`](super::helper)).

use super::helper::run_width;
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if super::edge_distance::on_edge_layers(rule, merged, "min_width") {
        return super::edge_distance::run_width(rule, layout, dbu_to_um, merged);
    }
    let min_w_dbu = rule.value / dbu_to_um;
    run_width(
        rule,
        layout,
        dbu_to_um,
        merged,
        "min_width",
        ">=",
        "Minimum width violation",
        move |w| w < min_w_dbu - 0.5,
        false,
        true,
        0.5,
    )
}
