// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forbidden overlap (e.g. Cnt.j "Cont on GatPoly over Activ is not allowed").  The
//! geometric intersection of all the rule's layers must be empty; any overlapping
//! region is reported.  The inverse predicate of [`coverage`](super::coverage): that
//! flags the part of a layer *outside* a cover, this flags the part *inside* a layer
//! it must avoid.
//!
//! Implemented as the `Intersection` residual over the cached tiles (see
//! [`helper::run_boolean_residual`](super::helper::run_boolean_residual)).

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
        eprintln!("[{}] forbidden_overlap needs at least two layers", rule.id);
        return vec![];
    }
    let descr = rule
        .layers
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join(" over ")
        + " not allowed";
    run_boolean_residual(
        rule, layout, dbu_to_um, merged,
        VirtualOp::Intersection,
        "Forbidden overlap",
        &descr,
    )
}
