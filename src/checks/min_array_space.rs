// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Via-array spacing: a large array must be spaced more generously than a lone via.
//!
//! Foundries write this rule two ways, and `axes` picks between them.
//!
//! * `axes: 1` (default) — the spacing must reach `value` in **at least one** axis; the
//!   other only needs the ordinary spacing rule.  A violation is an array tight in
//!   *both* directions.  IHP's `V1.b1` is this.
//! * `axes: 2` — the spacing must reach `value` in **both** axes, so *any* tight pair
//!   inside a large enough array is a violation.  GF180's `V#.2b` is this: 0.36 µm
//!   inside a 4×4-or-larger array against an ordinary 0.26 µm.
//!
//! The two are not interchangeable. An array tight in x and relaxed in y is clean under
//! the first and a violation under the second.
//!
//! Detection requires genuine two-dimensional density, mirroring the reference
//! deck's morphological test (close, then erode by half an array-block extent):
//! a **run** is a maximal chain of horizontally tight vias (edge gap < `value`,
//! rows overlapping in y); a run *qualifies* when it is longer than `cols`; and a
//! violation needs more than `rows` qualifying runs stacked vertically tight
//! (x-overlapping, vertical edge gap < `value`).  This is exactly "tight in both
//! directions": a via **ring** (e.g. around a bond pad) has long runs but never
//! more than two stacked, a single row/column has no stack, and an array that
//! relaxes either axis to ≥ `value` loses its runs or its stacking — all clean.
//!
//! Under `axes: 2` the array has to be recognised *before* its spacing is judged, since
//! a legal array is one nothing is tight in.  `pitch` is the gap up to which two vias
//! count as belonging to the same array; it defaults to `value`, and a deck whose array
//! rule allows spacings above the limit to still form an array sets it higher (GF180
//! closes the layer by 0.2 µm, which bridges 0.4).  Under `axes: 1` grouping and
//! violation are the same question, so `pitch` does not apply.
//!
//! Params: `rows` and `cols` (array-size thresholds, "more than N"; default 3), `axes`
//! (1 or 2; default 1), `pitch` (µm; default `value`), `count` (smallest array in vias;
//! default 0).
//! Operates on the whole (global) via layer, since an array can span merge tiles.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, UnionFind};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

/// A via region in µm: centroid and bounding box.
struct Via {
    cx: f64,
    cy: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    _merged: &mut MergedCache,
) -> Vec<Violation> {
    let layer = &rule.layers[0];
    let value = rule.value;
    let rows_thr = rule.params.get("rows").copied().unwrap_or(3.0) as usize;
    let cols_thr = rule.params.get("cols").copied().unwrap_or(3.0) as usize;
    let axes = rule.params.get("axes").copied().unwrap_or(1.0) as usize;
    // Smallest array, in vias, the rule applies to. The row/column thresholds already
    // bound the shape; this bounds the population, which is how the reference words it
    // ("interacting with 16 or more vias") and what keeps a ragged cluster that happens
    // to span four rows from counting as a 4×4 array.
    let min_count = rule.params.get("count").copied().unwrap_or(0.0) as usize;
    let pitch = rule.params.get("pitch").copied().unwrap_or(value);
    // What counts as "next to" for the purpose of finding the array. With `axes: 1` that
    // is the same question as the violation, so the two thresholds coincide.
    let link = if axes >= 2 { pitch.max(value) } else { value };
    let half = 0.5 * dbu_to_um;

    println!(
        "[{}] Checking min_array_space >= {:.2} µm in {} of 2 axes, arrays over {}×{}, on layer {}",
        rule.id, value, axes, rows_thr, cols_thr, layer.name
    );

    // One via per boundary.  Vias are single, non-touching rectangles, so we skip the
    // global boolean merge (a sweep-line union that is pure overhead — and the dominant
    // cost — when shapes never overlap) and read each via's bounding box directly, in
    // parallel.  Identical rectangles are de-duplicated by their integer-DBU extent.
    let boundaries = layout.get(layer.gds_layer as i16, layer.gds_datatype as i16);
    let vias: Vec<Via> = boundaries
        .par_iter()
        .filter_map(|b| {
            if b.xy.len() < 3 {
                return None;
            }
            let (mut x0, mut y0) = (i32::MAX, i32::MAX);
            let (mut x1, mut y1) = (i32::MIN, i32::MIN);
            for p in &b.xy {
                x0 = x0.min(p.x);
                y0 = y0.min(p.y);
                x1 = x1.max(p.x);
                y1 = y1.max(p.y);
            }
            if x1 <= x0 || y1 <= y0 {
                return None;
            }
            let via = Via {
                cx: (x0 as f64 + x1 as f64) * 0.5 * dbu_to_um,
                cy: (y0 as f64 + y1 as f64) * 0.5 * dbu_to_um,
                x0: x0 as f64 * dbu_to_um,
                y0: y0 as f64 * dbu_to_um,
                x1: x1 as f64 * dbu_to_um,
                y1: y1 as f64 * dbu_to_um,
            };
            Some(((x0, y0, x1, y1), via))
        })
        .collect::<HashMap<(i32, i32, i32, i32), Via>>()
        .into_values()
        .collect();

    let n = vias.len();
    if n == 0 {
        return vec![];
    }

    // Hash grid: a connectible neighbour (edge gap < value, axis-aligned) has its
    // centroid within `value + via extent` in one axis and overlaps in the other, so
    // a cell of that size puts every neighbour in the 3×3 block around a via.
    let max_extent = vias
        .iter()
        .fold(0.0_f64, |a, v| a.max(v.x1 - v.x0).max(v.y1 - v.y0));
    let cell = (link + max_extent).max(dbu_to_um);
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, v) in vias.iter().enumerate() {
        grid.entry(((v.cx / cell).floor() as i64, (v.cy / cell).floor() as i64))
            .or_default()
            .push(i);
    }

    // Discover the horizontally tight via pairs in parallel (the grid and via list
    // are read only here), then replay the unions sequentially — union-find is cheap
    // and the per-pair geometry is the work worth spreading across cores.
    let h_edges: Vec<(usize, usize)> = (0..n)
        .into_par_iter()
        .flat_map_iter(|i| {
            let a = &vias[i];
            let (gx, gy) = ((a.cx / cell).floor() as i64, (a.cy / cell).floor() as i64);
            let mut local = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some(bucket) = grid.get(&(gx + dx, gy + dy)) else {
                        continue;
                    };
                    for &j in bucket {
                        if j <= i {
                            continue;
                        }
                        let b = &vias[j];
                        // Same row (y ranges overlap) and a tight horizontal gap.
                        if a.y1.min(b.y1) - a.y0.max(b.y0) > 0.0 {
                            let xgap = (b.x0 - a.x1).max(a.x0 - b.x1);
                            if xgap >= 0.0 && xgap + half < link {
                                local.push((i, j));
                            }
                        }
                    }
                }
            }
            local.into_iter()
        })
        .collect();

    let mut uf = UnionFind::new(n);
    for (i, j) in h_edges {
        uf.union(i, j);
    }

    // Horizontal runs, keeping only those longer than the column threshold.
    let mut runs: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        runs.entry(uf.find(i)).or_default().push(i);
    }
    struct Run {
        members: Vec<usize>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    }
    let runs: Vec<Run> = runs
        .into_values()
        .filter(|m| m.len() > cols_thr)
        .map(|members| {
            let (mut x0, mut y0) = (f64::MAX, f64::MAX);
            let (mut x1, mut y1) = (f64::MIN, f64::MIN);
            for &i in &members {
                x0 = x0.min(vias[i].x0);
                y0 = y0.min(vias[i].y0);
                x1 = x1.max(vias[i].x1);
                y1 = y1.max(vias[i].y1);
            }
            Run {
                members,
                x0,
                y0,
                x1,
                y1,
            }
        })
        .collect();

    // Stack qualifying runs that face each other: x ranges overlap and the vertical
    // edge gap is tight.  A stack deeper than the row threshold is the violation.
    let mut ruf = UnionFind::new(runs.len());
    for i in 0..runs.len() {
        for j in i + 1..runs.len() {
            let (a, b) = (&runs[i], &runs[j]);
            if a.x1.min(b.x1) - a.x0.max(b.x0) <= 0.0 {
                continue; // no horizontal overlap: side-by-side arrays, not a stack
            }
            let ygap = (b.y0 - a.y1).max(a.y0 - b.y1);
            if ygap + half < link {
                ruf.union(i, j);
            }
        }
    }
    let mut stacks: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..runs.len() {
        stacks.entry(ruf.find(i)).or_default().push(i);
    }

    // Under `axes: 2` the array is only in breach if something inside it is actually
    // tight; grouping used the looser `pitch`, so that has to be asked separately.
    let tight_pair_in = |members: &[usize]| -> bool {
        let set: std::collections::HashSet<usize> = members.iter().copied().collect();
        members.iter().any(|&i| {
            let a = &vias[i];
            let (gx, gy) = ((a.cx / cell).floor() as i64, (a.cy / cell).floor() as i64);
            (-1..=1).any(|dx| {
                (-1..=1).any(|dy| {
                    grid.get(&(gx + dx, gy + dy)).is_some_and(|bucket| {
                        bucket.iter().any(|&j| {
                            if j == i || !set.contains(&j) {
                                return false;
                            }
                            let b = &vias[j];
                            // Neighbours in a row, or in a column, closer than the limit.
                            let row = a.y1.min(b.y1) - a.y0.max(b.y0) > 0.0
                                && (b.x0 - a.x1).max(a.x0 - b.x1) + half < value;
                            let col = a.x1.min(b.x1) - a.x0.max(b.x0) > 0.0
                                && (b.y0 - a.y1).max(a.y0 - b.y1) + half < value;
                            row || col
                        })
                    })
                })
            })
        })
    };

    let mut out = Vec::new();
    for stack in stacks.values() {
        if stack.len() <= rows_thr {
            continue;
        }
        let members: Vec<usize> = stack
            .iter()
            .flat_map(|&r| runs[r].members.iter().copied())
            .collect();
        if members.len() < min_count {
            continue;
        }
        if axes >= 2 && !tight_pair_in(&members) {
            continue;
        }
        let min_cols = stack
            .iter()
            .map(|&r| runs[r].members.len())
            .min()
            .unwrap_or(0);
        let (mut sx, mut sy, mut cnt) = (0.0, 0.0, 0.0);
        for &r in stack {
            for &i in &runs[r].members {
                sx += vias[i].cx;
                sy += vias[i].cy;
                cnt += 1.0;
            }
        }
        let (cx, cy) = (sx / cnt, sy / cnt);
        out.push(Violation::point(
            rule.id.as_str(),
            "Via array spacing violation",
            format!(
                "{}×{} {} array below {:.2} µm at ({:.4}, {:.4}) µm",
                stack.len(),
                min_cols,
                layer.name,
                value,
                cx,
                cy
            ),
            cx,
            cy,
        ));
    }
    out
}
