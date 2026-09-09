// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum overlap between two layers — KLayout's `overlap` check.
//!
//! Where two layers share area, this measures how deeply they penetrate each other: a
//! salicide block that must cover the COMP it blocks by 0.22 µm has to reach that far
//! past the COMP's edge, not merely touch it.
//!
//! It is [`min_space`](super::min_space) measured the other way round. Both scan the
//! same facing edge pairs — two edges whose outward normals oppose — and differ only in
//! which side of them the rule is about: the empty ground outside, or the material of
//! both inside. So this is not a separate engine, only the other direction through one.

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
    super::helper::run_overlap(rule, layout, dbu_to_um, merged)
}
