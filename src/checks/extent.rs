// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounding-box extent checks for (rectangular) features such as contact bars.
//!
//! A feature has two principal sizes: its **width** (the short bounding-box side) and
//! its **length** (the long side).  These checks bound one of them, so a contact bar
//! can require an exact width *and* a minimum length without the width rule tripping on
//! the length — which the facing-wall scan (`min_width`/`max_width`) cannot distinguish.
//! All four run on the shared [`helper::run_extent`](super::helper::run_extent) driver
//! (exact for axis-aligned rectangles).

use super::helper::run_extent;
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Width (short side) must be at least `value`.
pub fn run_min_width(rule: &RuleDefinition, layout: &FlatLayout, dbu: f64, merged: &mut MergedCache) -> Vec<Violation> {
    let v = rule.value / dbu;
    run_extent(rule, layout, dbu, merged, "min_dim", ">=", "Minimum width violation", false, move |d| d < v - 0.5)
}

/// Width (short side) must be at most `value`.
pub fn run_max_width(rule: &RuleDefinition, layout: &FlatLayout, dbu: f64, merged: &mut MergedCache) -> Vec<Violation> {
    let v = rule.value / dbu;
    run_extent(rule, layout, dbu, merged, "max_dim", "<=", "Maximum width violation", false, move |d| d > v + 0.5)
}

/// Length (long side) must be at least `value`.
pub fn run_min_length(rule: &RuleDefinition, layout: &FlatLayout, dbu: f64, merged: &mut MergedCache) -> Vec<Violation> {
    let v = rule.value / dbu;
    run_extent(rule, layout, dbu, merged, "min_length", ">=", "Minimum length violation", true, move |d| d < v - 0.5)
}

/// Length (long side) must be at most `value`.
pub fn run_max_length(rule: &RuleDefinition, layout: &FlatLayout, dbu: f64, merged: &mut MergedCache) -> Vec<Violation> {
    let v = rule.value / dbu;
    run_extent(rule, layout, dbu, merged, "max_length", "<=", "Maximum length violation", true, move |d| d > v + 0.5)
}
