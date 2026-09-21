// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `antenna_ratio`: the conductor area on a gate's net divided by the gate's area, summed
//! level by level up the stack.
//!
//! Mirrors IHP's `antenna.drc` (Fig 7.1): at each metal or via level the net is the
//! connectivity through every connect step up to that level, so the gate-area
//! denominator grows as higher layers merge gates, and the level's own conductor area on
//! the net is the numerator.  A net carrying a protection diode takes the relaxed limit
//! (`diode: with`), a net without one the strict limit (`diode: without`).  GF180
//! measures a metal by its sidewall (`metric: sidewall`, perimeter times `thickness`)
//! and credits a diode by adding `diode_factor` times its area to the denominator.
//!
//! `layers` are the conductors, one per level being accumulated.  The gate and the
//! diodes are layer params - `gate`, `diode_1`, `diode_2`, `diode_3` - each with a
//! `_net_of` naming the conductor its net is looked up on, since a gate sits on poly and
//! a diode on diffusion and neither marker is in the connect graph; `antenna_net_of`
//! does the same for a derived conductor such as poly over field.  `level` names the
//! layer through whose first connect step the net is read, `level_before` the one it
//! stops short of; without either, each conductor is read at its own step.  The
//! cumulative ratio is monotonic up the stack, so flagging the final sum is what
//! KLayout flagging at any level comes to.

use super::super::params::{NotAWord, mode};
use crate::connectivity::{Connectivity, LayerKey};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::{Layer, RuleDefinition};
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

fn net_key(rule: &RuleDefinition, name: &str) -> Option<LayerKey> {
    let l = rule.num(name)? as i16;
    let d = rule
        .num(&format!("{name}_dt"))
        .map(|v| v as i16)
        .unwrap_or(0);
    Some((l, d))
}

fn key(l: &Layer) -> LayerKey {
    (l.gds_layer as i16, l.gds_datatype as i16)
}

/// The level the net is read at: through the first connect step of a layer, or short of
/// it.  `None` reads each conductor at its own step.
fn level(rule: &RuleDefinition, conn: &Connectivity) -> Result<Option<usize>, ()> {
    let named = |k: &str, before: bool| -> Result<Option<usize>, ()> {
        let Some(l) = net_key(rule, k) else {
            return Ok(None);
        };
        let Some(prefix) = conn.connect_prefix(l) else {
            eprintln!(
                "[{}] antenna_ratio: the `{k}` layer is in no connect step",
                rule.id
            );
            return Err(());
        };
        Ok(Some(if before { prefix - 1 } else { prefix }))
    };
    match (named("level", false)?, named("level_before", true)?) {
        (Some(_), Some(_)) => {
            eprintln!(
                "[{}] antenna_ratio: `level` and `level_before` name one level each - give one",
                rule.id
            );
            Err(())
        }
        (Some(p), None) | (None, Some(p)) => Ok(Some(p)),
        (None, None) => Ok(None),
    }
}

/// Diode area (µm²) per net at `part`, summed over every diode layer.  Each layer's net
/// is resolved through its own base layer, since a derived diode (n+ outside the well,
/// p+ inside it) is not itself in the connect graph.
fn diode_area_per_net(
    diodes: &[(LayerKey, LayerKey)],
    conn: &Connectivity,
    part: &crate::connectivity::Partition,
    layout: &FlatLayout,
    merged: &mut MergedCache,
    d2: f64,
) -> HashMap<usize, f64> {
    let mut out: HashMap<usize, f64> = HashMap::new();
    for &(dkey, dnet) in diodes {
        let regions = merged.regions(layout, dkey.0, dkey.1).to_vec();
        let per: Vec<HashMap<usize, f64>> = regions
            .par_chunks(1 << 12)
            .map(|chunk| {
                let mut per: HashMap<usize, f64> = HashMap::new();
                for r in chunk {
                    if let Some(net) = part.net_at(conn, dnet, r.marker.0, r.marker.1) {
                        *per.entry(net).or_default() += r.area_dbu * d2;
                    }
                }
                per
            })
            .collect();
        for m in per {
            for (net, v) in m {
                *out.entry(net).or_default() += v;
            }
        }
    }
    out
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
    // The gate and the conductor its net is looked up on, and the diodes and theirs.
    let Some(gate) = net_key(rule, "gate") else {
        eprintln!("[{}] antenna_ratio needs the layer param `gate`", rule.id);
        return vec![];
    };
    let gate_net = net_key(rule, "gate_net_of").unwrap_or(gate);
    let diodes: Vec<(LayerKey, LayerKey)> = ["diode_1", "diode_2", "diode_3"]
        .iter()
        .filter_map(|k| {
            net_key(rule, k).map(|d| (d, net_key(rule, &format!("{k}_net_of")).unwrap_or(d)))
        })
        .collect();
    let conductors = rule.layers.clone();
    if conductors.is_empty() {
        eprintln!(
            "[{}] antenna_ratio needs at least one conductor layer",
            rule.id
        );
        return vec![];
    }
    let require_diode = match mode(rule, "antenna_ratio", "diode") {
        Ok(None) => None,
        Ok(Some("with")) => Some(true),
        Ok(Some("without")) => Some(false),
        Ok(Some(other)) => {
            eprintln!(
                "[{}] antenna_ratio: diode can only be `with` or `without`, not `{other}`",
                rule.id
            );
            return vec![];
        }
        Err(NotAWord) => return vec![],
    };
    // GF180 measures a metal antenna by its sidewall area - perimeter times the metal's
    // thickness - rather than by its plan area, and credits a protection diode by adding
    // `diode_factor` times the diode area to the *denominator* instead of switching to a
    // relaxed limit.  Both are off unless the rule asks for them, so IHP's rules keep
    // their own model.
    let by_perimeter = match mode(rule, "antenna_ratio", "metric") {
        Ok(None) => false,
        Ok(Some("sidewall")) => true,
        Ok(Some("area")) => false,
        Ok(Some(other)) => {
            eprintln!(
                "[{}] antenna_ratio: metric can only be `area` or `sidewall`, not `{other}`",
                rule.id
            );
            return vec![];
        }
        Err(NotAWord) => return vec![],
    };
    let thickness = rule.num("thickness").unwrap_or(1.0);
    let diode_factor = rule.num("diode_factor");
    let diode_min = rule.num("diode_area").unwrap_or(0.16);
    let Ok(fixed_level) = level(rule, conn) else {
        return vec![];
    };
    // The base layer an antenna region's net is resolved through (poly-on-field sits on
    // GatPoly); defaults to the antenna layer itself (a metal/via/contact is its own net).
    let antenna_net = net_key(rule, "antenna_net_of");

    let d2 = dbu_to_um * dbu_to_um;
    let limit = rule.value;
    let rule = &RuleDefinition {
        id: rule.id.clone(),
        check: rule.check.clone(),
        layers: conductors,
        value: rule.value,
        params: rule.params.clone(),
        ignore: rule.ignore.clone(),
        text: rule.text.clone(),
    };

    println!(
        "[{}] Checking antenna_ratio: cumulative {} area / gate area ≥ {limit}{}",
        rule.id,
        rule.layers
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>()
            .join("+"),
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
        .regions(layout, gate.0, gate.1)
        .par_iter()
        .filter_map(|r| {
            conn.node_at(gate_net, r.marker.0, r.marker.1)
                .map(|n| (r.area_dbu * d2, r.marker, n))
        })
        .collect();
    if gates.is_empty() {
        return vec![];
    }

    // Summed per net over the layer's regions - ten million contacts on a full chip
    // - in parallel, a map per chunk folded into one.
    let sum_nets = |per: &mut HashMap<usize, f64>, net: usize, v: f64| {
        *per.entry(net).or_default() += v;
    };
    let fold = |maps: Vec<HashMap<usize, f64>>| {
        let mut out: HashMap<usize, f64> = HashMap::new();
        for m in maps {
            for (net, v) in m {
                sum_nets(&mut out, net, v);
            }
        }
        out
    };

    // Cumulative ratio per gate, summed level by level with each level's own partition.
    let mut cum = vec![0.0f64; gates.len()];
    for l in &rule.layers {
        let lkey = key(l);
        let Some(prefix) = fixed_level.or_else(|| conn.connect_prefix(lkey)) else {
            continue;
        };
        let part = conn.partition(prefix);

        // Gate area per net at this level (O(1) per gate via the precomputed node).
        let gate_area: HashMap<usize, f64> = fold(
            gates
                .par_chunks(1 << 14)
                .map(|chunk| {
                    let mut per: HashMap<usize, f64> = HashMap::new();
                    for (area, _, node) in chunk {
                        sum_nets(&mut per, part.net_of(*node), *area);
                    }
                    per
                })
                .collect(),
        );

        // The antenna quantity per net: plan area, or sidewall area (perimeter times
        // metal thickness) when the rule asks for it.  If the antenna layer is itself in
        // the connect graph (the common case — a metal/via/contact), walk its own regions
        // and read each net directly; only a derived antenna layer (e.g. poly-on-field)
        // needs point lookups.
        let metric = |r: &crate::merge::Region| {
            if by_perimeter {
                r.perimeter_dbu * dbu_to_um * thickness
            } else {
                r.area_dbu * d2
            }
        };
        let mut layer_area: HashMap<usize, f64> = HashMap::new();
        if antenna_net.is_none() && conn.in_graph(lkey) {
            let regions = conn.regions_of(lkey);
            layer_area = fold(
                regions
                    .par_chunks(1 << 16)
                    .enumerate()
                    .map(|(c, chunk)| {
                        let mut per: HashMap<usize, f64> = HashMap::new();
                        for (i, r) in chunk.iter().enumerate() {
                            if let Some(node) = conn.region_node(lkey, (c << 16) + i) {
                                sum_nets(&mut per, part.net_of(node), metric(r));
                            }
                        }
                        per
                    })
                    .collect(),
            );
        }
        if layer_area.is_empty() {
            let ant_net = antenna_net.unwrap_or(lkey);
            let regions = merged.regions(layout, lkey.0, lkey.1).to_vec();
            layer_area = fold(
                regions
                    .par_chunks(1 << 12)
                    .map(|chunk| {
                        let mut per: HashMap<usize, f64> = HashMap::new();
                        for r in chunk {
                            if let Some(net) = part.net_at(conn, ant_net, r.marker.0, r.marker.1) {
                                sum_nets(&mut per, net, metric(r));
                            }
                        }
                        per
                    })
                    .collect(),
            );
        }

        // The diode credit, when the rule uses one, is evaluated on the *same* net as the
        // antenna it offsets — a diode only protects a gate it is already connected to at
        // this level of the stack.
        let level_diode =
            diode_factor.map(|_| diode_area_per_net(&diodes, conn, &part, layout, merged, d2));

        for (i, (_, _, node)) in gates.iter().enumerate() {
            let net = part.net_of(*node);
            let g = gate_area.get(&net).copied().unwrap_or(0.0);
            let denom = match (diode_factor, &level_diode) {
                (Some(mf), Some(d)) => g + mf * d.get(&net).copied().unwrap_or(0.0),
                _ => g,
            };
            if denom > 0.0 {
                cum[i] += layer_area.get(&net).copied().unwrap_or(0.0) / denom;
            }
        }
    }

    // Diode presence on the full net (relaxes the limit).  The full partition is cached.
    let full = conn.partition(usize::MAX);
    let diode_area: HashMap<usize, f64> = if require_diode.is_some() {
        diode_area_per_net(&diodes, conn, &full, layout, merged, d2)
    } else {
        HashMap::new()
    };

    let mut out = Vec::new();
    for (i, (gate_a, marker, node)) in gates.iter().enumerate() {
        if *gate_a <= 0.0 {
            continue;
        }
        // A diode of the size the diode rule asks for (`diode_area`, Ant.g's 0.16)
        // protects: the diode rule's floor is met at its value, and a diode that meets
        // it is a protection diode.
        let has_diode =
            diode_area.get(&full.net_of(*node)).copied().unwrap_or(0.0) >= diode_min * (1.0 - 1e-9);
        if require_diode.is_some_and(|req| has_diode != req) {
            continue;
        }
        // A maximum is met at its value, as every rule of a manual is; a ratio of
        // exactly 200 is clean.  The sum of the levels is read a hair past the value,
        // since 20.0 / 0.1 in floating point is not always 200.
        if cum[i] > limit * (1.0 + 1e-9) {
            let (x, y) = (marker.0 * dbu_to_um, marker.1 * dbu_to_um);
            out.push(Violation::point(
                &rule.id,
                "Antenna ratio violation",
                format!(
                    "cumulative antenna ratio {:.1} > {limit} (gate {gate_a:.4} µm²{}) at ({x:.4}, {y:.4}) µm",
                    cum[i],
                    if has_diode { ", with diode" } else { "" },
                ),
                x, y,
            ));
        }
    }
    out
}
