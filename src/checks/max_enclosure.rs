// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maximum enclosure: every shape on the enclosed layer (`layers[1]`) that sits inside an
//! enclosing region (`layers[0]`) must have no more than the rule's margin on any side.
//!
//! The mirror image of [`min_enclosure`](super::min_enclosure) — used for the PDF's "min.
//! and max." device-shape rules (e.g. Sdiod.a/b/c), where a `min_enclosure` entry at the
//! same value pins the margin to (near) exactly that value.  The shared engine lives in
//! [`helper::run_max_enclosure`](super::helper).

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
    super::helper::run_max_enclosure(rule, layout, dbu_to_um, merged)
}
