// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Gate-length rules (Gat.a1–a4): the GatPoly width where it forms a device gate of a
//! given type must be at least the rule value.  The poly width is measured directly (so
//! the channel *width* W, set by the Activ edges, is not mistaken for the gate length L);
//! `layers[1]` masks the measurement to the relevant gate region.  See
//! [`helper::run_gate_length`](super::helper::run_gate_length).

use super::helper::run_gate_length;
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
    if rule.layers.len() < 2 {
        eprintln!("[{}] gate_length needs a poly layer and a gate-region mask", rule.id);
        return vec![];
    }
    run_gate_length(rule, layout, dbu_to_um, merged)
}
