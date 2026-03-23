// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum enclosure: every shape on the enclosed layer (`layers[1]`) must sit
//! inside an enclosing region (`layers[0]`) with at least the rule's margin on
//! **all** sides.  The tiled engine lives in [`helper::run_enclosure`](super::helper).

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
    super::helper::run_enclosure(rule, layout, dbu_to_um, merged, false)
}
