// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum spacing between merged regions.
//!
//! The tiled region-pair engine lives in [`helper::run_gated`](super::helper); this
//! is the plain, always-on variant (every facing pair within `value` violates).

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
    super::helper::run_gated(rule, layout, dbu_to_um, merged, |_, _| true)
}
