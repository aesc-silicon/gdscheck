// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact width: every perpendicular span of the metal must equal the rule value
//! (used for fixed-size features such as vias).  Runs the shared facing-edge width
//! scan over the merged tiles (see [`helper`](super::helper)), flagging any width
//! that differs from the value.

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
    let val_dbu = rule.value / dbu_to_um;
    run_width(
        rule, layout, dbu_to_um, merged,
        "exact_width", "==", "Exact width violation",
        move |w| (w - val_dbu).abs() > 0.5,
        false, 0.5,
    )
}
