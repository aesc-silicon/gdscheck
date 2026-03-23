// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Width- and parallel-run-conditional spacing (e.g. IHP `TM2.bR`).
//!
//! A spacing rule that only applies between *wide* lines running *parallel* for a
//! long distance: the minimum space `value` is required between two regions when at
//! least one is wider than `wide_width` and their parallel run exceeds `parallel_run`.
//! Reuses the tiled region-pair engine in [`helper`](super::helper); this module only
//! supplies the per-pair gate.
//!
//! Params: `wide_width` (µm) and `parallel_run` (µm), both required.  Width and run
//! are measured from the regions' bounding boxes (exact for axis-aligned metal).

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
    let wide_width = rule.params.get("wide_width").copied();
    let parallel_run = rule.params.get("parallel_run").copied();
    let (Some(wide_width), Some(parallel_run)) = (wide_width, parallel_run) else {
        eprintln!("[{}] min_space_prl needs `wide_width` and `parallel_run` params", rule.id);
        return vec![];
    };

    let value = rule.value;
    // The conditional spacing only applies where a wide line runs parallel *at the
    // violating gap*.  Match the engine's violation threshold (`< value - half`) so the
    // wide/parallel facing pair coincides with the close spacing: a wide power rail that
    // merely runs alongside a wire at the clean `value` gap must not lend its width to a
    // narrow tooth that dips to a sub-`value` gap elsewhere on the same polygon.
    let max_gap = value - 0.5 * dbu_to_um;
    // Both conditions are evaluated from the real facing edges, not the bounding boxes:
    // a stepped IO pad's box overlaps a neighbour for tens of microns and an L-shaped
    // narrow trace's box looks wide, yet neither is a long wide parallel run.
    run_gated(rule, layout, dbu_to_um, merged, move |a: &Poly, b: &Poly| {
        a.prl_applies(b, max_gap, wide_width, parallel_run)
    })
}
