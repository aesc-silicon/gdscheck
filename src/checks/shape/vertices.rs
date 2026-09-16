// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `max_vertices`: no polygon as drawn with more vertices than the value - the stream
//! and mask constraint every tape-out deck carries at 200, 600 or 8191.
//!
//! Read on the drawn polygons, not the merge: the limit is on what is written to the
//! stream, and a merge joins shapes into regions that were never one polygon.  A
//! boundary's closing point repeats its first and is not a vertex.

use crate::layout::FlatLayout;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] max_vertices needs a layer", rule.id);
        return vec![];
    };
    let limit = rule.value.round() as usize;
    println!(
        "[{}] Checking max_vertices <= {limit} on layer {}",
        rule.id, layer.name
    );
    let mut out = Vec::new();
    for b in super::super::boundaries_on(layout, layer) {
        let pts = b.xy.len();
        let closed = pts >= 2 && b.xy[0] == b.xy[pts - 1];
        let n = if closed { pts - 1 } else { pts };
        if n <= limit || pts == 0 {
            continue;
        }
        let (x, y) = (b.xy[0].x as f64 * dbu_to_um, b.xy[0].y as f64 * dbu_to_um);
        out.push(Violation::point(
            &rule.id,
            "Vertex count violation",
            format!(
                "{}: polygon with {n} vertices > {limit} at ({x:.4}, {y:.4}) µm",
                layer.name
            ),
            x,
            y,
        ));
    }
    out
}
