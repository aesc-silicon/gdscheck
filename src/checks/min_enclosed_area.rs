// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum enclosed area (e.g. Act.e): a hole — an empty region fully surrounded by the
//! layer — must be at least `value` µm².  Tiny holes are reported.  See
//! [`helper::run_enclosed_area`](super::helper::run_enclosed_area).

use super::helper::run_enclosed_area;
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
    run_enclosed_area(rule, layout, dbu_to_um, merged)
}
