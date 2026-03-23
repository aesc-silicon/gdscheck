// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum endcap enclosure (e.g. IHP `V1.c1`).
//!
//! Like [`min_enclosure`](super::min_enclosure), but the margin is required on at
//! least **one** side rather than all of them — the wire *endcap*.  A via at a metal
//! corner must be treated as an endcap on one side (this rule), with the ordinary
//! enclosure (`V1.c`) covering the rest.  The shared engine lives in
//! [`helper::run_enclosure`](super::helper); `endcap = true` selects the max-side
//! (best side) reduction, so a region violates only if *every* side is short.

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
    super::helper::run_enclosure(rule, layout, dbu_to_um, merged, true)
}
