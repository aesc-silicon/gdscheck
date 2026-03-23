// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Coverage / containment (e.g. Cnt.g "Cont within Activ or GatPoly", Cnt.h "Cont
//! covered with Metal1").  Every region on the target layer (`layers[0]`) must lie
//! inside the union of the remaining layers; any part sticking out is reported.
//!
//! Implemented as the `Difference` residual `target − (cover ∪ …)` over the cached
//! tiles (see [`helper::run_boolean_residual`](super::helper::run_boolean_residual)).
//! Unlike `min_enclosure`, this handles "covered by A *or* B" and flags contacts that
//! miss the cover entirely, not just those short on an enclosure margin.

use super::helper::run_boolean_residual;
use crate::layout::FlatLayout;
use crate::merge::{MergedCache, VirtualOp};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!("[{}] coverage needs a target layer plus at least one cover layer", rule.id);
        return vec![];
    }
    let target = rule.layers[0].name.as_str();
    let covers = rule.layers[1..]
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join(" or ");
    let descr = format!("{target} not covered by {covers}");
    run_boolean_residual(
        rule, layout, dbu_to_um, merged,
        VirtualOp::Difference,
        "Coverage violation",
        &descr,
    )
}
