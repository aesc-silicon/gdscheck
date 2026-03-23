// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Spacing between lines where at least one is 45-degree-bent (e.g. M5.i).
//!
//! A wider minimum space `value` is required between two regions when at least one of
//! them has a 45°-bent (angled) wall *near the gap*.  Reuses the tiled region-pair
//! engine in [`helper`](super::helper); this module only supplies the per-pair gate.
//!
//! The bend has to be close to the spacing, not merely somewhere on the same net: a
//! long net may run at 45° in one place and Manhattan elsewhere, and only the angled
//! stretch should attract the wider spacing.

use super::helper::{run_gated, Poly};
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
    let max_gap = rule.value - 0.5 * dbu_to_um;
    run_gated(rule, layout, dbu_to_um, merged, move |a: &Poly, b: &Poly| {
        a.has_diagonal_near(b, max_gap) || b.has_diagonal_near(a, max_gap)
    })
}
