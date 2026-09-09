// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum spacing between merged regions.
//!
//! The tiled region-pair engine lives in [`helper::run_gated`](super::helper); this
//! is the plain, always-on variant (every facing pair within `value` violates).
//!
//! Declared on two *edge* layers, the same rule measures between segments instead — see
//! [`edge_distance`](super::edge_distance). Which one runs is the deck's choice of layer,
//! not a different rule.

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
    if super::edge_distance::on_edge_layers(rule, merged, "min_space") {
        return super::edge_distance::run_space(rule, layout, dbu_to_um, merged);
    }
    super::helper::run_gated(rule, layout, dbu_to_um, merged, |_, _, _, _| true)
}
