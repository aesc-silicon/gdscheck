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

    let key = (layer.gds_layer as i16, layer.gds_datatype as i16);
    // An edge layer says the same thing about a boundary: GF180's PP.11 forbids a butting
    // Pplus/NCOMP edge within 0.43 um of a well edge, and what it forbids is the segment,
    // which no region carries.
    if merged.is_edge_layer(key) {
        println!(
            "[{}] Checking forbidden edges on edge layer {}",
            rule.id, layer.name
        );
        merged.ensure_edges(layout, key);
        let tile = merged.tile_dbu() as i64;
        let mut out = Vec::new();
        for (&(tx, ty), edges) in merged.edges(key) {
            let core = crate::merge::Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            for e in edges {
                let (mx, my) = e.midpoint();
                if !core.contains(mx, my) {
                    continue; // owned by the tile the segment's middle falls in
                }
                let (ax, ay) = (e.a.x as f64 * dbu_to_um, e.a.y as f64 * dbu_to_um);
                let (bx, by) = (e.b.x as f64 * dbu_to_um, e.b.y as f64 * dbu_to_um);
                out.push(Violation::edge(
                    &rule.id,
                    "Forbidden edge",
                    format!(
                        "{} present at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                        layer.name, ax, ay, bx, by
                    ),
                    ax,
                    ay,
                    bx,
                    by,
                ));
            }
        }
        return out;
    }

    println!(
        "[{}] Checking forbidden region on layer {} ({}/{})",
        rule.id, layer.name, layer.gds_layer, layer.gds_datatype
    );

    let regions = merged.regions(layout, key.0, key.1);
    regions
        .iter()
        .map(|region| {
            let (cx, cy) = region.marker;
            let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
            Violation::point(
                &rule.id,
                "Forbidden region",
                format!("{} present at ({:.4}, {:.4}) µm", layer.name, x, y),
                x,
                y,
            )
        })
        .collect()
}
