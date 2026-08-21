// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Length bounds on an *edge* layer (e.g. GF180 `DF.2a`, minimum channel width).
//!
//! The rule's single layer must be an edge layer — boundary segments, not regions — and
//! every segment's length is compared against the rule value.  This is the check that
//! makes the channel-width family sayable: a transistor's W is the length of the Activ
//! boundary running under the gate, which is a property of that segment and of no
//! region.  Build the segment with `edge_layers:` (`edges`, then `and` against the gate
//! outline), then bound it here.
//!
//! Reported per segment, positioned on the segment itself, so a marker lands on the
//! boundary the rule is about rather than on the region behind it.

use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Every edge must be at least `value` long.
pub fn run_min(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(
        rule,
        layout,
        dbu_to_um,
        merged,
        "min_edge_length",
        "<",
        |l, v| l < v - 0.5,
    )
}

/// No edge may be longer than `value`.
pub fn run_max(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    run(
        rule,
        layout,
        dbu_to_um,
        merged,
        "max_edge_length",
        ">",
        |l, v| l > v + 0.5,
    )
}

fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    check_name: &str,
    cmp: &str,
    viol: impl Fn(f64, f64) -> bool,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] {check_name} needs a layer", rule.id);
        return vec![];
    };
    let key = (layer.gds_layer as i16, layer.gds_datatype as i16);

    // A polygon layer here is a config error, not an empty result: the rule would
    // silently pass, which is the failure mode this engine works hardest to avoid.
    if !merged.is_edge_layer(key) {
        eprintln!(
            "[{}] {check_name}: layer '{}' is not an edge layer — declare it under \
             `edge_layers:`",
            rule.id, layer.name
        );
        return vec![];
    }
    merged.ensure_edges(layout, key);

    println!(
        "[{}] Checking {} on edge layer {} ({} µm)",
        rule.id, check_name, layer.name, rule.value
    );

    let limit_dbu = rule.value / dbu_to_um;
    let tile = merged.tile_dbu() as i64;
    let mut out = Vec::new();

    for (&(tx, ty), edges) in merged.edges(key) {
        let core = Core {
            x0: tx as i64 * tile,
            y0: ty as i64 * tile,
            x1: (tx as i64 + 1) * tile,
            y1: (ty as i64 + 1) * tile,
        };
        for e in edges {
            let len = e.length();
            if !viol(len, limit_dbu) {
                continue;
            }
            // The midpoint decides ownership, so an edge seen from two tiles is
            // reported once — the same rule the polygon checks use.
            let (mx, my) = e.midpoint();
            if !core.contains(mx, my) {
                continue;
            }
            out.push(Violation::edge(
                &rule.id,
                "Edge length violation",
                format!(
                    "{}: edge length {:.4} µm {} {:.4} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                    layer.name,
                    len * dbu_to_um,
                    cmp,
                    rule.value,
                    e.a.x as f64 * dbu_to_um,
                    e.a.y as f64 * dbu_to_um,
                    e.b.x as f64 * dbu_to_um,
                    e.b.y as f64 * dbu_to_um,
                ),
                e.a.x as f64 * dbu_to_um,
                e.a.y as f64 * dbu_to_um,
                e.b.x as f64 * dbu_to_um,
                e.b.y as f64 * dbu_to_um,
            ));
        }
    }
    out
}
