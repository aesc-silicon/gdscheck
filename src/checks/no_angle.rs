// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! No forbidden edge angles on a layer (e.g. Gat.f: "45° GatPoly on Activ not allowed").
//! Run over an intersection layer such as GatPolyOverActiv so only the part crossing the
//! channel is inspected; every non-orthogonal edge (or, with the `angle` param, every edge
//! at a specific orientation) is flagged.  See
//! [`helper::run_no_angle`](super::helper::run_no_angle).

use super::helper::run_no_angle;
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
    if rule.layers.is_empty() {
        eprintln!("[{}] no_angle needs a layer", rule.id);
        return vec![];
    }
    run_no_angle(rule, layout, dbu_to_um, merged)
}
