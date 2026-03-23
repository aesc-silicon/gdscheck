// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum 45-degree-bent width (e.g. M5.g): the perpendicular span of a metal
//! trace that runs at 45° must be at least the rule value, but only where the bent
//! run is longer than `bent_length` µm.  Reuses the oblique branch of the shared
//! width scan (see [`helper`](super::helper)); the rectilinear passes are skipped so
//! axis-aligned metal — covered by the ordinary `min_width` rule — is left alone.

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
    let min_w_dbu = rule.value / dbu_to_um;
    let bent_um = rule.params.get("bent_length").copied().unwrap_or(0.5);
    let min_run_dbu = bent_um / dbu_to_um;
    run_width(
        rule, layout, dbu_to_um, merged,
        "min_45_width", ">=", "Minimum 45° width violation",
        move |w| w < min_w_dbu - 0.5,
        true, min_run_dbu,
    )
}
