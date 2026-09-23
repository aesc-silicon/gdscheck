// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `min_notch`: no gap narrower than the value between two walls of one region - a slot
//! cut into a shape, or a thin hole through it.  The same walls a width is read between,
//! facing the other way; [`notch_pairs`] is the width scan run so, and this is what a
//! rule adds: its bound, `angle: bent` for the 45° gaps alone, `length` for gaps whose
//! walls share more than so much run.

use super::super::helper::piece_notches;
use super::super::params::{bent_only, min_run};
use crate::geom::{Limit, notch_pairs};
use crate::layout::FlatLayout;
use crate::merge::{Core, SharedCache};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let limit = Limit::at_least(rule.value, dbu_to_um);
    let Some(bent) = bent_only(rule, "min_notch") else {
        return vec![];
    };
    let min_run_dbu = min_run(rule, dbu_to_um);
    // A chamfer facing a straight wall across a gap is a notch too; a bent-only rule
    // leaves it to the plain one.
    let mixed = !bent;
    let tile = merged.tile_dbu() as i64;
    let mut violations = Vec::new();

    for layer in &rule.layers {
        let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, gl, gd);
        println!(
            "[{}] Checking min_notch >= {:.2} µm on layer {} ({}/{})",
            rule.id, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );
        let (rid, lname, limit_um) = (rule.id.as_str(), layer.name.as_str(), rule.value);
        let mut layer_violations: Vec<Violation> = merged
            .tiles(gl, gd)
            .par_iter()
            .flat_map_iter(|(&(tx, ty), polys)| {
                let core = Core {
                    x0: tx as i64 * tile,
                    y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile,
                    y1: (ty as i64 + 1) * tile,
                };
                polys
                    .iter()
                    .flat_map(move |poly| {
                        notch_pairs(poly, core, limit, bent, mixed, min_run_dbu)
                            .into_iter()
                            .map(move |(x1, y1, x2, y2, gap_dbu)| {
                                let g = gap_dbu * dbu_to_um;
                                let (x1, y1, x2, y2) = (
                                    x1 * dbu_to_um,
                                    y1 * dbu_to_um,
                                    x2 * dbu_to_um,
                                    y2 * dbu_to_um,
                                );
                                Violation::edge(
                                    rid,
                                    "Minimum notch violation",
                                    format!(
                                        "{lname}: notch {g:.4} µm < {limit_um:.2} µm at \
                                         ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                                    ),
                                    x1,
                                    y1,
                                    x2,
                                    y2,
                                )
                            })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            })
            .collect();
        violations.append(&mut layer_violations);
        // A notch between two pieces of one region, where the tile holds them apart.
        violations.extend(piece_notches(
            rule,
            layout,
            dbu_to_um,
            merged,
            (gl, gd),
            lname,
            limit,
            bent,
            min_run_dbu,
        ));
    }
    violations
}
