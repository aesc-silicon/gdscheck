// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Each region of `layers[0]` must overlap at least one shape from the other layers;
//! regions that don't are flagged.  Used for "device must contain its via/contact" rules
//! such as MIM.h ("TopVia1 must be over MIM" — every MIM cap needs a TopVia1/Vmim).
//!
//! The layers involved are sparse device/via layers, so they are merged globally.

use crate::checks::boundaries_on;
use crate::layout::FlatLayout;
use crate::merge::{merge_boundaries, merged_centroid_dbu, select_interacting};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!("[{}] must_interact needs a target layer and at least one partner", rule.id);
        return vec![];
    }
    let target = merge_boundaries(boundaries_on(layout, &rule.layers[0]));
    let partners: Vec<_> = rule.layers[1..]
        .iter()
        .flat_map(|l| merge_boundaries(boundaries_on(layout, l)))
        .collect();

    println!(
        "[{}] Checking every {} overlaps one of {}",
        rule.id,
        rule.layers[0].name,
        rule.layers[1..].iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", "),
    );

    // Keep target regions that do NOT interact any partner.
    select_interacting(&target, &partners, false)
        .iter()
        .map(|r| {
            let (cx, cy) = merged_centroid_dbu(r);
            let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
            Violation::point(
                &rule.id,
                "Missing required overlap",
                format!(
                    "{} has no {} at ({x:.4}, {y:.4}) µm",
                    rule.layers[0].name,
                    rule.layers[1..].iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join("/"),
                ),
                x, y,
            )
        })
        .collect()
}
