// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum enclosure: every shape on the enclosed layer (`layers[1]`) must sit
//! inside an enclosing region (`layers[0]`) with at least the rule's margin on
//! **all** sides.  The tiled engine lives in [`helper::run_enclosure`](super::helper).
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
    if super::edge_distance::on_edge_layers(rule, merged, "min_enclosure") {
        return super::edge_distance::run_enclosure(rule, layout, dbu_to_um, merged);
    }
    let sides = super::helper::Sides::of(rule);
    super::helper::run_enclosure(rule, layout, dbu_to_um, merged, sides)
}
