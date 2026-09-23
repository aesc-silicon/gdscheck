// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Length bounds on the segments of a layer.
//!
//! On an *edge* layer - boundary segments, not regions - every segment's length is
//! bounded.  This is the check that makes the channel-width family sayable (GF180
//! `DF.2a`): a transistor's W is the length of the Activ boundary running under the
//! gate, which is a property of that segment and of no region.  Build the segment with
//! `edge_layers:` (`edges`, then `and` against the gate outline), then bound it here.
//!
//! On a *polygon* layer every segment of every region's boundary is bounded, holes
//! included: "no edge shorter than so much" is a rule of its own in finer processes, and
//! a merged region's segments are the ones a mask sees.
//!
//! A length is the root of a sum of two squares of integers, and the bound reads it
//! squared, exactly.  Reported per segment, positioned on the segment itself, so a
//! marker lands on the boundary the rule is about rather than on the region behind it.

use super::Kind;
use crate::layout::FlatLayout;
use crate::merge::{Core, SharedCache};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// The check's name under `kind`, as the deck spells it.
pub fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Min => "min_edge_length",
        Kind::Max => "max_edge_length",
        Kind::Exact => "exact_edge_length",
    }
}

pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let check_name = name(kind);
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] {check_name} needs a layer", rule.id);
        return vec![];
    };
    let key = (layer.gds_layer as i16, layer.gds_datatype as i16);
    let limit = kind.limit(rule.value, dbu_to_um);
    let cmp = kind.cmp();
    let tile = merged.tile_dbu() as i64;
    let core_of = |tx: i32, ty: i32| Core {
        x0: tx as i64 * tile,
        y0: ty as i64 * tile,
        x1: (tx as i64 + 1) * tile,
        y1: (ty as i64 + 1) * tile,
    };
    // Every segment as its ends in DBU, with the tile that owns its midpoint - so a
    // segment seen from two tiles is reported once, the rule the polygon checks use.
    let mut segs: Vec<((i64, i64), (i64, i64))> = Vec::new();
    if merged.is_edge_layer(key) {
        merged.ensure_edges(layout, key);
        println!(
            "[{}] Checking {} {} {:.4} µm on edge layer {}",
            rule.id,
            check_name,
            kind.op(),
            rule.value,
            layer.name
        );
        for (&(tx, ty), edges) in merged.edges(key).iter() {
            let core = core_of(tx, ty);
            for e in edges {
                let (mx, my) = e.midpoint();
                if core.contains(mx, my) {
                    segs.push(((e.a.x as i64, e.a.y as i64), (e.b.x as i64, e.b.y as i64)));
                }
            }
        }
    } else {
        merged.ensure(layout, key.0, key.1);
        println!(
            "[{}] Checking {} {} {:.4} µm on every boundary segment of {}",
            rule.id,
            check_name,
            kind.op(),
            rule.value,
            layer.name
        );
        for (&(tx, ty), polys) in merged.tiles(key.0, key.1).iter() {
            let core = core_of(tx, ty);
            for m in polys {
                for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
                    let n = ring.len();
                    for i in 0..n {
                        let (a, b) = (ring[i], ring[(i + 1) % n]);
                        if a == b {
                            continue;
                        }
                        let (mx, my) = (
                            (a.x as f64 + b.x as f64) * 0.5,
                            (a.y as f64 + b.y as f64) * 0.5,
                        );
                        if core.contains(mx, my) {
                            segs.push(((a.x as i64, a.y as i64), (b.x as i64, b.y as i64)));
                        }
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for ((ax, ay), (bx, by)) in segs {
        let (dx, dy) = ((bx - ax) as i128, (by - ay) as i128);
        let len2 = dx * dx + dy * dy;
        if !limit.broken_by_sq(len2, 1) {
            continue;
        }
        let len = (len2 as f64).sqrt();
        let (ax, ay, bx, by) = (
            ax as f64 * dbu_to_um,
            ay as f64 * dbu_to_um,
            bx as f64 * dbu_to_um,
            by as f64 * dbu_to_um,
        );
        out.push(Violation::edge(
            &rule.id,
            "Edge length violation",
            format!(
                "{}: edge length {:.4} µm {} {:.4} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                layer.name,
                len * dbu_to_um,
                cmp,
                rule.value,
                ax,
                ay,
                bx,
                by,
            ),
            ax,
            ay,
            bx,
            by,
        ));
    }
    out
}
