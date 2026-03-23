// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Antenna area-ratio checks (§7.1, Ant.b/e and Ant.d/f) — the first net-aware checks.
//!
//! Mirrors IHP's `antenna.drc`: the conductor area connected to a gate, divided by the
//! gate area, is accumulated layer by layer up the stack (Fig 7.1).  At each metal (or via)
//! level the net is the connectivity through *all layers up to that level*, so the gate-area
//! denominator grows as higher layers merge gates.  A net carrying a protection diode
//! (≥ 0.16 µm² of diffusion) takes the relaxed limit (Ant.e/f); otherwise the strict one
//! (Ant.b/d).
//!
//! Each metal/via level's net comes from [`Connectivity::partition`] over the matching
//! prefix of the ordered connect steps; the diode flag uses the final (full) net.
//!
//! `layers` = [gate, antenna conductors…, diode]; `antenna_layers` counts the conductors;
//! `gate_net_layer` / `diode_net_layer` are the base layers a region's net is resolved
//! through (a gate sits on `GatPoly`, a diode on `Activ`).  `require_diode` selects nets
//! with (1) or without (0) a diode.  Because the cumulative ratio is monotonic up the
//! stack, flagging the final cumulative is equivalent to KLayout flagging at any level.

use crate::connectivity::{Connectivity, LayerKey};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::{Layer, RuleDefinition};
use crate::violation::Violation;
use std::collections::HashMap;

fn net_key(rule: &RuleDefinition, name: &str) -> Option<LayerKey> {
    let l = *rule.params.get(name)? as i16;
    let d = rule.params.get(&format!("{name}_dt")).map(|v| *v as i16).unwrap_or(0);
    Some((l, d))
}

fn key(l: &Layer) -> LayerKey {
    (l.gds_layer as i16, l.gds_datatype as i16)
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let Some(conn) = conn else {
        eprintln!("[{}] antenna_ratio needs connectivity", rule.id);
        return vec![];
    };
    if rule.layers.len() < 2 {
        eprintln!("[{}] antenna_ratio needs a gate layer and at least one antenna layer", rule.id);
        return vec![];
    }

    let n_ant = rule.params.get("antenna_layers").map(|v| *v as usize).unwrap_or(rule.layers.len() - 1);
    let gate = &rule.layers[0];
    let antenna = &rule.layers[1..1 + n_ant];
    let diode = rule.layers.get(1 + n_ant);

    let gate_net = net_key(rule, "gate_net_layer").unwrap_or(key(gate));
    let diode_net = net_key(rule, "diode_net_layer");
    let require_diode = rule.params.get("require_diode").map(|v| *v != 0.0);

    let d2 = dbu_to_um * dbu_to_um;
    let limit = rule.value;

    println!(
        "[{}] Checking antenna_ratio: cumulative {} area / {} gate area ≥ {limit}{}",
        rule.id,
        antenna.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join("+"),
        gate.name,
        match require_diode {
            Some(true) => " (nets with a protection diode)",
            Some(false) => " (nets without a protection diode)",
            None => "",
        },
    );

    // Gates: area (µm²), marker, and — once — the global node of the GatPoly region each
    // sits on.  The node is partition-independent, so the per-level net is then O(1)
    // (`part.net_of(node)`) with no repeated point lookup.
    let gates: Vec<(f64, (f64, f64), usize)> = merged
        .regions(layout, gate.gds_layer as i16, gate.gds_datatype as i16)
        .iter()
        .filter_map(|r| {
            conn.node_at(gate_net, r.marker.0, r.marker.1).map(|n| (r.area_dbu * d2, r.marker, n))
        })
        .collect();
    if gates.is_empty() {
        return vec![];
    }

    // A fixed connectivity level (Ant.a/c, the "initial" pre-metal net) or, by default,
    // each antenna layer's own connect level (the cumulative metal/via stack).
    let fixed_level = rule.params.get("level").map(|v| *v as usize);
    // The base layer an antenna region's net is resolved through (poly-on-field sits on
    // GatPoly); defaults to the antenna layer itself (a metal/via/contact is its own net).
    let antenna_net = net_key(rule, "antenna_net_layer");

    // Cumulative ratio per gate, summed level by level with each level's own partition.
    let mut cum = vec![0.0f64; gates.len()];
    for l in antenna {
        let lkey = key(l);
        let Some(prefix) = fixed_level.or_else(|| conn.connect_prefix(lkey)) else { continue };
        let part = conn.partition(prefix);

        // Gate area per net at this level (O(1) per gate via the precomputed node).
        let mut gate_area: HashMap<usize, f64> = HashMap::new();
        for (area, _, node) in &gates {
            *gate_area.entry(part.net_of(*node)).or_default() += area;
        }

        // This-layer area per net.  If the antenna layer is itself in the connect graph
        // (the common case — a metal/via/contact), walk its own regions and read each net
        // directly; only a derived antenna layer (e.g. poly-on-field) needs point lookups.
        let mut layer_area: HashMap<usize, f64> = HashMap::new();
        if let Some(base) = conn.node_base(lkey).filter(|_| antenna_net.is_none()) {
            for (idx, r) in conn.regions_of(lkey).iter().enumerate() {
                *layer_area.entry(part.net_of(base + idx)).or_default() += r.area_dbu * d2;
            }
        }
        if layer_area.is_empty() {
            let ant_net = antenna_net.unwrap_or(lkey);
            for r in merged.regions(layout, lkey.0, lkey.1).iter() {
                if let Some(net) = part.net_at(conn, ant_net, r.marker.0, r.marker.1) {
                    *layer_area.entry(net).or_default() += r.area_dbu * d2;
                }
            }
        }

        for (i, (_, _, node)) in gates.iter().enumerate() {
            let net = part.net_of(*node);
            let g = gate_area.get(&net).copied().unwrap_or(0.0);
            if g > 0.0 {
                cum[i] += layer_area.get(&net).copied().unwrap_or(0.0) / g;
            }
        }
    }

    // Diode presence on the full net (relaxes the limit).  The full partition is cached.
    let full = conn.partition(usize::MAX);
    let mut diode_area: HashMap<usize, f64> = HashMap::new();
    if let (Some(diode), Some(dnet)) = (diode, diode_net) {
        for r in merged.regions(layout, diode.gds_layer as i16, diode.gds_datatype as i16).to_vec() {
            if let Some(node) = conn.node_at(dnet, r.marker.0, r.marker.1) {
                *diode_area.entry(full.net_of(node)).or_default() += r.area_dbu * d2;
            }
        }
    }

    let mut out = Vec::new();
    for (i, (gate_a, marker, node)) in gates.iter().enumerate() {
        if *gate_a <= 0.0 {
            continue;
        }
        let has_diode = diode_area.get(&full.net_of(*node)).copied().unwrap_or(0.0) > 0.16;
        if require_diode.is_some_and(|req| has_diode != req) {
            continue;
        }
        if cum[i] >= limit {
            let (x, y) = (marker.0 * dbu_to_um, marker.1 * dbu_to_um);
            out.push(Violation::point(
                &rule.id,
                "Antenna ratio violation",
                format!(
                    "cumulative antenna ratio {:.1} ≥ {limit} (gate {gate_a:.4} µm²{}) at ({x:.4}, {y:.4}) µm",
                    cum[i],
                    if has_diode { ", with diode" } else { "" },
                ),
                x, y,
            ));
        }
    }
    out
}
