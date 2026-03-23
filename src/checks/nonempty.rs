// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forbidden region: every connected region of the layer is a violation.  Used for
//! marker rules whose error condition is "this geometry must not exist" — the layer
//! (typically a derived virtual) *is* the error.  For example antenna Ant.i, where
//! `AntIError = pactiv_con ∩ Recog.diode − Recog.esd − (NWell ∪ PWell.block)` is the
//! set of p-diodes sitting in the PWell.
//!
//! Regions are reconstructed from the shared [`MergedCache`] (stitched across tile
//! borders), so a region spanning several tiles is reported once.

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
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] nonempty needs a layer", rule.id);
        return vec![];
    };

    println!(
        "[{}] Checking forbidden region on layer {} ({}/{})",
        rule.id, layer.name, layer.gds_layer, layer.gds_datatype
    );

    let regions = merged.regions(layout, layer.gds_layer as i16, layer.gds_datatype as i16);
    regions
        .iter()
        .map(|region| {
            let (cx, cy) = region.marker;
            let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
            Violation::point(
                &rule.id,
                "Forbidden region",
                format!("{} present at ({:.4}, {:.4}) µm", layer.name, x, y),
                x, y,
            )
        })
        .collect()
}
