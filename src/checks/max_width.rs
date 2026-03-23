// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maximum width: the perpendicular span of the metal must not exceed the rule
//! value anywhere.  Runs the shared facing-edge width scan over the merged tiles
//! (see [`helper`](super::helper)), flagging widths that are too large.

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
    let max_w_dbu = rule.value / dbu_to_um;
    run_width(
        rule, layout, dbu_to_um, merged,
        "max_width", "<=", "Maximum width violation",
        move |w| w > max_w_dbu + 0.5,
        false, 0.5,
    )
}
