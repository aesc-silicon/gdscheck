// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Gate-connected minimum area (§7.1 Ant.g) — net-aware.
//!
//! Flags every region of a marker layer that is (a) electrically connected to a gate and
//! (b) smaller than `value` µm².  Used for the antenna protection diode: a `dantenna` /
//! `dpantenna` device tied to a gate must be at least 0.16 µm² to actually protect it.
//!
//! `layers` = [gate, marker layers…]; a region's net is resolved through `gate_net_layer`
//! (a gate sits on GatPoly) and `marker_net_layer` (a diode on Activ).

use crate::connectivity::{Connectivity, LayerKey};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::HashSet;

fn net_key(rule: &RuleDefinition, name: &str, default: LayerKey) -> LayerKey {
    match rule.params.get(name) {
        Some(l) => (*l as i16, rule.params.get(&format!("{name}_dt")).map(|v| *v as i16).unwrap_or(0)),
        None => default,
    }
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let Some(conn) = conn else {
        eprintln!("[{}] gate_connected_min_area needs connectivity", rule.id);
        return vec![];
    };
    if rule.layers.len() < 2 {
        eprintln!("[{}] gate_connected_min_area needs a gate layer and a marker layer", rule.id);
        return vec![];
    }
    let gate = &rule.layers[0];
    let markers = &rule.layers[1..];
    let gate_net = net_key(rule, "gate_net_layer", (gate.gds_layer as i16, gate.gds_datatype as i16));
    let marker_net = net_key(rule, "marker_net_layer", (0, 0));
    let d2 = dbu_to_um * dbu_to_um;

    println!(
        "[{}] Checking gate_connected_min_area >= {:.4} µm² on {}",
        rule.id,
        rule.value,
        markers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", "),
    );

    // Nets that carry a gate.
    let mut gate_nets: HashSet<usize> = HashSet::new();
    for r in merged.regions(layout, gate.gds_layer as i16, gate.gds_datatype as i16).to_vec() {
        if let Some(net) = conn.net_at(gate_net, r.marker.0, r.marker.1) {
            gate_nets.insert(net);
        }
    }

    let mut out = Vec::new();
    for ml in markers {
        for r in merged.regions(layout, ml.gds_layer as i16, ml.gds_datatype as i16).to_vec() {
            let area = r.area_dbu * d2;
            if area >= rule.value {
                continue;
            }
            let Some(net) = conn.net_at(marker_net, r.marker.0, r.marker.1) else { continue };
            if gate_nets.contains(&net) {
                let (x, y) = (r.marker.0 * dbu_to_um, r.marker.1 * dbu_to_um);
                out.push(Violation::point(
                    &rule.id,
                    "Gate-connected min area violation",
                    format!(
                        "{} region {area:.4} µm² < {:.4} µm² connected to a gate at ({x:.4}, {y:.4}) µm",
                        ml.name, rule.value
                    ),
                    x, y,
                ));
            }
        }
    }
    out
}
