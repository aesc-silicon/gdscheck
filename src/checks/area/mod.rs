// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Area: how much of a layer there is, summed over what `scope` names - a connected
//! region (the default), a hole through one, everything of a second layer inside each
//! region of the first, or the whole chip.  Three rules read it, at least, at most and
//! exactly, and `net: connected` narrows a region rule to the regions on a net that
//! also carries some region of another layer - the antenna diode that must be big
//! enough to protect the gate it is tied to.
//!
//! Regions are reconstructed from the shared [`MergedCache`] by stitching the per-tile
//! merge across tile borders, so a layer of any density is measured whole in bounded
//! memory.  An area on the grid is a whole number of half square DBU, and so is the
//! value once rounded the way the bound wants - a minimum up, a maximum down - so the
//! comparison is exact wherever the tiling cuts a shape at grid points, which is every
//! axis-aligned and 45° wall.

use super::params::{NotAWord, mode};
use crate::connectivity::{Connectivity, LayerKey};
use crate::geom::on_grid;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, VirtualOp, clipped_area_dbu, compose_tile, stitch_labeled};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashSet;

/// Which bound a rule puts on the area.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
    Exact,
}

impl Kind {
    /// The check's name, as the deck spells it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Min => "min_area",
            Kind::Max => "max_area",
            Kind::Exact => "exact_area",
        }
    }

    /// The requirement, as the log prints it.
    pub fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
            Kind::Exact => "==",
        }
    }

    /// The failing comparison, as the message prints it.
    pub fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
            Kind::Exact => "≠",
        }
    }

    /// The word a violation's title starts with.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Min => "Minimum",
            Kind::Max => "Maximum",
            Kind::Exact => "Exact",
        }
    }

    /// The bound in DBU², on the half grid an area lives on: a minimum rounds up, a
    /// maximum down, an exact value to the nearest.
    fn limit(self, value_um2: f64, dbu_to_um: f64) -> f64 {
        let twice = 2.0 * value_um2 / (dbu_to_um * dbu_to_um);
        let round = match self {
            Kind::Min => f64::ceil,
            Kind::Max => f64::floor,
            Kind::Exact => f64::round,
        };
        on_grid(twice, round) as f64 * 0.5
    }

    /// Whether an area of `a` DBU² breaks the bound `limit`.
    fn broken_by(self, a: f64, limit: f64) -> bool {
        match self {
            Kind::Min => a < limit,
            Kind::Max => a > limit,
            Kind::Exact => a != limit,
        }
    }
}

/// What the area is summed over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// Each connected region of the layer.  The default.
    Region,
    /// Each hole through a region: an empty area the layer surrounds entirely.
    Hole,
    /// Everything of `layers[1]` inside each region of `layers[0]`, summed per region.
    Contained,
    /// Everything of the layer on the chip, summed.
    Chip,
}

impl Scope {
    fn of(rule: &RuleDefinition, name: &str) -> Option<Self> {
        match mode(rule, name, "scope") {
            Ok(None) | Ok(Some("region")) => Some(Scope::Region),
            Ok(Some("hole")) => Some(Scope::Hole),
            Ok(Some("contained")) => Some(Scope::Contained),
            Ok(Some("chip")) => Some(Scope::Chip),
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] {name}: scope can only be `region`, `hole`, `contained` or `chip`, \
                     not `{other}`",
                    rule.id
                );
                None
            }
            Err(NotAWord) => None,
        }
    }
}

/// The regions a `net: connected` rule is about: those on a net that also carries a
/// region of `connected`.  Nets are looked up on conductors of the connect graph, which
/// a marker layer is not: `net_of` names the conductor under the measured layer (a
/// diode sits on Activ), `connected_net_of` the one under the connected layer (a gate
/// on GatPoly); each defaults to the layer itself.
pub struct Connected {
    pub layer: LayerKey,
    pub layer_net_of: LayerKey,
    pub net_of: Option<LayerKey>,
}

impl Connected {
    /// The gate a rule's params name; `Ok(None)` when it names none, `Err` when it is
    /// malformed, which the rule has said.
    fn of(rule: &RuleDefinition, name: &str) -> Result<Option<Connected>, ()> {
        match mode(rule, name, "net") {
            Ok(None) => Ok(None),
            Ok(Some("connected")) => {
                let key = |k: &str| {
                    rule.num(k)
                        .map(|l| (l as i16, rule.num(&format!("{k}_dt")).unwrap_or(0.0) as i16))
                };
                let Some(layer) = key("connected") else {
                    eprintln!(
                        "[{}] {name}: `net: connected` needs the layer param `connected`",
                        rule.id
                    );
                    return Err(());
                };
                Ok(Some(Connected {
                    layer,
                    layer_net_of: key("connected_net_of").unwrap_or(layer),
                    net_of: key("net_of"),
                }))
            }
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] {name}: net can only be `connected`, not `{other}`",
                    rule.id
                );
                Err(())
            }
            Err(NotAWord) => Err(()),
        }
    }
}

/// `min_area` / `max_area` / `exact_area`: the area over the scope the params name.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let name = kind.name();
    let Some(scope) = Scope::of(rule, name) else {
        return vec![];
    };
    let Ok(connected) = Connected::of(rule, name) else {
        return vec![];
    };
    run_scoped(
        kind,
        scope,
        connected,
        &rule.layers,
        rule,
        layout,
        dbu_to_um,
        merged,
        conn,
    )
}

/// The area under `kind`, `scope` and `connected`, whether they came from the params or
/// from a check name that stands for them; `measured` is the layers a region rule reads,
/// which is every layer the rule names unless a name says otherwise.
#[allow(clippy::too_many_arguments)]
pub fn run_scoped(
    kind: Kind,
    scope: Scope,
    connected: Option<Connected>,
    measured: &[crate::pdk::Layer],
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let name = kind.name();
    if rule.layers.is_empty() {
        eprintln!("[{}] {name} needs a layer", rule.id);
        return vec![];
    }
    if connected.is_some() && scope != Scope::Region {
        eprintln!(
            "[{}] {name}: `net: connected` reads regions - a hole, a container or the \
             chip is on no net",
            rule.id
        );
        return vec![];
    }
    let limit = kind.limit(rule.value, dbu_to_um);
    let d2 = dbu_to_um * dbu_to_um;
    let (title, cmp) = (format!("{} area violation", kind.word()), kind.cmp());
    let um2 = |a: f64| a * d2;
    match scope {
        Scope::Region => {
            // The nets that carry the connected layer, and the conductor the measured
            // layer's net is looked up on.
            let gate: Option<(HashSet<usize>, Option<LayerKey>)> = match connected {
                None => None,
                Some(c) => {
                    let Some(conn) = conn else {
                        eprintln!("[{}] {name}: `net: connected` needs connectivity", rule.id);
                        return vec![];
                    };
                    let mut nets = HashSet::new();
                    for r in merged.regions(layout, c.layer.0, c.layer.1).to_vec() {
                        if let Some(net) = conn.net_at(c.layer_net_of, r.marker.0, r.marker.1) {
                            nets.insert(net);
                        }
                    }
                    Some((nets, c.net_of))
                }
            };
            let mut out = Vec::new();
            for layer in measured {
                let key = (layer.gds_layer as i16, layer.gds_datatype as i16);
                println!(
                    "[{}] Checking {name} {} {:.4} µm² on layer {}{}",
                    rule.id,
                    kind.op(),
                    rule.value,
                    layer.name,
                    if gate.is_some() {
                        ", on a net with the connected layer"
                    } else {
                        ""
                    }
                );
                for r in merged.regions(layout, key.0, key.1).to_vec() {
                    if !kind.broken_by(r.area_dbu, limit) {
                        continue;
                    }
                    if let Some((nets, net_of)) = &gate {
                        let conn = conn.expect("checked above");
                        let lookup = net_of.unwrap_or(key);
                        let Some(net) = conn.net_at(lookup, r.marker.0, r.marker.1) else {
                            continue; // on no net: not connected to anything
                        };
                        if !nets.contains(&net) {
                            continue;
                        }
                    }
                    let (x, y) = (r.marker.0 * dbu_to_um, r.marker.1 * dbu_to_um);
                    let how = if gate.is_some() {
                        " on a connected net"
                    } else {
                        ""
                    };
                    out.push(Violation::point(
                        &rule.id,
                        &title,
                        format!(
                            "region area {:.4} µm² {cmp} {:.4} µm² on layer {}{how} at ({x:.4}, {y:.4}) µm",
                            um2(r.area_dbu),
                            rule.value,
                            layer.name
                        ),
                        x,
                        y,
                    ));
                }
            }
            out
        }
        Scope::Hole => {
            // Holes live in each merged region's `holes`, whole in every tile's copy of
            // the region; each is owned by the tile whose core holds its centroid.
            let tile = merged.tile_dbu() as i64;
            let mut out = Vec::new();
            for layer in &rule.layers {
                let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
                merged.ensure(layout, gl, gd);
                println!(
                    "[{}] Checking {name} {} {:.4} µm² of every hole through {}",
                    rule.id,
                    kind.op(),
                    rule.value,
                    layer.name
                );
                let (rid, ln, title) = (rule.id.as_str(), layer.name.as_str(), title.as_str());
                let mut v: Vec<Violation> = merged
                    .tiles(gl, gd)
                    .par_iter()
                    .flat_map_iter(move |(&(tx, ty), polys)| {
                        let core = Core {
                            x0: tx as i64 * tile,
                            y0: ty as i64 * tile,
                            x1: (tx as i64 + 1) * tile,
                            y1: (ty as i64 + 1) * tile,
                        };
                        let mut out = Vec::new();
                        for m in polys {
                            for hole in &m.holes {
                                let (area, cx, cy) = crate::geom::ring_area_centroid(hole);
                                if !kind.broken_by(area, limit) || !core.owns_region(cx, cy) {
                                    continue;
                                }
                                let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
                                out.push(Violation::point(
                                    rid,
                                    title,
                                    format!(
                                        "hole area {:.4} µm² {cmp} {:.4} µm² through {ln} at ({x:.4}, {y:.4}) µm",
                                        um2(area),
                                        rule.value
                                    ),
                                    x,
                                    y,
                                ));
                            }
                        }
                        out.into_iter()
                    })
                    .collect();
                out.append(&mut v);
            }
            out
        }
        Scope::Contained => {
            // Every region of the container on its own, the area of the contained layer
            // within it summed: "these may share a plate, as long as the total on that
            // one plate stays under the cap" (GF180 MIM.11).
            let (Some(outer), Some(inner)) = (rule.layers.first(), rule.layers.get(1)) else {
                eprintln!(
                    "[{}] {name} with `scope: contained` needs two layers (container, contained)",
                    rule.id
                );
                return vec![];
            };
            let (ok, od) = (outer.gds_layer as i16, outer.gds_datatype as i16);
            let (ik, id) = (inner.gds_layer as i16, inner.gds_datatype as i16);
            println!(
                "[{}] Checking {name} {} {:.4} µm² of {} inside each {} region",
                rule.id,
                kind.op(),
                rule.value,
                inner.name,
                outer.name
            );
            merged.ensure(layout, ok, od);
            merged.ensure(layout, ik, id);
            let tile = merged.tile_dbu();
            let labeled = stitch_labeled(merged.tiles(ok, od), tile);
            let mut total = vec![0.0_f64; labeled.regions.len()];
            // The violation is about the whole container, so it is marked at the middle
            // of it rather than at whichever piece the stitcher named it after.
            let mut bbox = vec![(f64::MAX, f64::MAX, f64::MIN, f64::MIN); labeled.regions.len()];
            for (&(tx, ty), polys) in &labeled.by_tile {
                let t = tile as i64;
                let (x0, y0) = ((tx as i64 * t) as f64, (ty as i64 * t) as f64);
                let (x1, y1) = (((tx as i64 + 1) * t) as f64, ((ty as i64 + 1) * t) as f64);
                let ins = merged.tiles(ik, id).get(&(tx, ty));
                for (poly, rid) in polys {
                    let b = &mut bbox[*rid];
                    for p in &poly.outer {
                        b.0 = b.0.min(p.x as f64);
                        b.1 = b.1.min(p.y as f64);
                        b.2 = b.2.max(p.x as f64);
                        b.3 = b.3.max(p.y as f64);
                    }
                    let Some(ins) = ins else {
                        continue;
                    };
                    // Clip to the tile's own core so a container spanning tiles is
                    // counted once.
                    let hit =
                        compose_tile(VirtualOp::Intersection, &[std::slice::from_ref(poly), ins]);
                    for h in &hit {
                        total[*rid] += clipped_area_dbu(h, x0, y0, x1, y1);
                    }
                }
            }
            let mut out = Vec::new();
            for rid in 0..labeled.regions.len() {
                if !kind.broken_by(total[rid], limit) {
                    continue;
                }
                let b = bbox[rid];
                let (cx, cy) = ((b.0 + b.2) * 0.5 * dbu_to_um, (b.1 + b.3) * 0.5 * dbu_to_um);
                out.push(Violation::point(
                    &rule.id,
                    &title,
                    format!(
                        "{} inside this {} totals {:.4} µm² {cmp} {:.4} µm² at ({cx:.4}, {cy:.4}) µm",
                        inner.name,
                        outer.name,
                        um2(total[rid]),
                        rule.value
                    ),
                    cx,
                    cy,
                ));
            }
            out
        }
        Scope::Chip => {
            let mut out = Vec::new();
            for layer in &rule.layers {
                println!(
                    "[{}] Checking {name} {} {:.4} µm² of {} on the chip",
                    rule.id,
                    kind.op(),
                    rule.value,
                    layer.name
                );
                let regions =
                    merged.regions(layout, layer.gds_layer as i16, layer.gds_datatype as i16);
                let total: f64 = regions.iter().map(|r| r.area_dbu).sum();
                if !kind.broken_by(total, limit) {
                    continue;
                }
                // One chip-level violation, anchored at the first region's marker.
                let (cx, cy) = regions.first().map(|r| r.marker).unwrap_or((0.0, 0.0));
                out.push(Violation::point(
                    &rule.id,
                    &title,
                    format!(
                        "total {} area {:.4} µm² {cmp} {:.4} µm²",
                        layer.name,
                        um2(total),
                        rule.value
                    ),
                    cx * dbu_to_um,
                    cy * dbu_to_um,
                ));
            }
            out
        }
    }
}

/// The check names that stood for one scope each before the scope was a param.
pub fn run_named(
    name: &str,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    match name {
        "min_enclosed_area" => run_scoped(
            Kind::Min,
            Scope::Hole,
            None,
            &rule.layers,
            rule,
            layout,
            dbu_to_um,
            merged,
            conn,
        ),
        "max_contained_area" => run_scoped(
            Kind::Max,
            Scope::Contained,
            None,
            &rule.layers,
            rule,
            layout,
            dbu_to_um,
            merged,
            conn,
        ),
        "max_total_area" => run_scoped(
            Kind::Max,
            Scope::Chip,
            None,
            &rule.layers,
            rule,
            layout,
            dbu_to_um,
            merged,
            conn,
        ),
        "gate_connected_min_area" => {
            // layers = [gate, markers...], with `gate_net_layer` and `marker_net_layer`.
            if rule.layers.len() < 2 {
                eprintln!(
                    "[{}] gate_connected_min_area needs a gate layer and a marker layer",
                    rule.id
                );
                return vec![];
            }
            let gate = &rule.layers[0];
            let gate_key = (gate.gds_layer as i16, gate.gds_datatype as i16);
            let key = |k: &str| {
                rule.num(k)
                    .map(|l| (l as i16, rule.num(&format!("{k}_dt")).unwrap_or(0.0) as i16))
            };
            let connected = Connected {
                layer: gate_key,
                layer_net_of: key("gate_net_layer").unwrap_or(gate_key),
                net_of: key("marker_net_layer"),
            };
            run_scoped(
                Kind::Min,
                Scope::Region,
                Some(connected),
                &rule.layers[1..],
                rule,
                layout,
                dbu_to_um,
                merged,
                conn,
            )
        }
        _ => unreachable!("not an area name"),
    }
}
