// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Directional extension (e.g. Sal.c "SalBlock extension over Activ or GatPoly").
//!
//! Where the cover layer (`layers[0]`) sits over a target region (`layers[1]`), it must
//! extend at least `value` past the target's two long edges (its width direction); the
//! target's short edges (ends) are exempt, as it runs out past the cover there.  See
//! [`helper::run_extension`](super::helper::run_extension).

use super::helper::run_extension;
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
        eprintln!("[{}] min_extension needs a cover layer and a target layer", rule.id);
        return vec![];
    }
    run_extension(rule, layout, dbu_to_um, merged)
}
