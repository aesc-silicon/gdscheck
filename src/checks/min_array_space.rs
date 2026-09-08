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
    // Smallest side of the array's bounding box, in µm, for it to count.  GF180 words
    // "4x4 or larger" as a box at least three vias and three spaces across in every
    // direction, so a stack four rows deep at a tighter pitch is not yet an array.
    let min_extent = rule.params.get("min_extent").copied().unwrap_or(0.0);
    // How far two vias must overlap, across the gap, for the gap to be a space between
    // them at all: KLayout's `projecting >= x`.  Staggered rows overlapping by less are
    // not neighbours in the array sense.
    let projection = rule.params.get("projection").copied().unwrap_or(0.0);
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
    let vias: Vec<((i32, i32, i32, i32), Via)> = boundaries
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
        .collect::<Vec<_>>();
    // Identical rectangles are one via.  Sorted and deduplicated in parallel rather
    // than hashed one by one: seven million vias are a second and a half of hashing
    // on one core, and every step of this check used to do it that way.
    let mut vias = vias;
    vias.par_sort_unstable_by_key(|(k, _)| *k);
    vias.dedup_by_key(|(k, _)| *k);
    let vias: Vec<Via> = vias.into_iter().map(|(_, v)| v).collect();

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
    let grid = Grid::build(
        (0..n)
            .into_par_iter()
            .map(|i| {
                let v = &vias[i];
                (
                    ((v.cx / cell).floor() as i64, (v.cy / cell).floor() as i64),
                    i,
                )
            })
            .collect(),
    );

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
                    for &j in grid.get((gx + dx, gy + dy)) {
                        if j <= i {
                            continue;
                        }
                        let b = &vias[j];
                        // Same row (y ranges overlap) and a tight horizontal gap.  Two
                        // vias that overlap are one via - two cells placing the same
                        // array a little apart - and there is no gap between them to
                        // measure, only a merge that never happened.
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
    // Grouped by root through a sort, like the grid: the roots are read once in order
    // (path compression wants the borrow), then the vias are sorted by root in parallel
    // and each run is a range.
    let roots: Vec<usize> = (0..n).map(|i| uf.find(i)).collect();
    let mut by_root: Vec<(usize, usize)> = (0..n).into_par_iter().map(|i| (roots[i], i)).collect();
    by_root.par_sort_unstable();
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut at = 0;
    while at < n {
        let root = by_root[at].0;
        let end = at + by_root[at..].partition_point(|&(r, _)| r == root);
        runs.push(by_root[at..end].iter().map(|&(_, i)| i).collect());
        at = end;
    }
    struct Run {
        members: Vec<usize>,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    }
    let runs: Vec<Run> = runs
        .into_par_iter()
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
    //
    // Runs are filed in the same kind of grid as the vias, under every cell they span,
    // so a run meets only the runs filed near it.  Every pair used to be tried: a design
    // with 7.5 million vias has a few hundred thousand qualifying runs, and that was
    // sixteen seconds a rule on nothing.
    let mut rgrid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let cells = |r: &Run| {
        let (gx0, gx1) = ((r.x0 / cell).floor() as i64, (r.x1 / cell).floor() as i64);
        let (gy0, gy1) = ((r.y0 / cell).floor() as i64, (r.y1 / cell).floor() as i64);
        (gx0, gx1, gy0, gy1)
    };
    for (i, r) in runs.iter().enumerate() {
        let (gx0, gx1, gy0, gy1) = cells(r);
        for gx in gx0..=gx1 {
            for gy in gy0..=gy1 {
                rgrid.entry((gx, gy)).or_default().push(i);
            }
        }
    }
    let r_edges: Vec<(usize, usize)> = (0..runs.len())
        .into_par_iter()
        .flat_map_iter(|i| {
            let a = &runs[i];
            let (gx0, gx1, gy0, gy1) = cells(a);
            let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
            let mut local = Vec::new();
            for gx in gx0 - 1..=gx1 + 1 {
                for gy in gy0 - 1..=gy1 + 1 {
                    let Some(bucket) = rgrid.get(&(gx, gy)) else {
                        continue;
                    };
                    for &j in bucket {
                        if j <= i || !seen.insert(j) {
                            continue;
                        }
                        let b = &runs[j];
                        if a.x1.min(b.x1) - a.x0.max(b.x0) <= 0.0 {
                            continue; // no horizontal overlap: side-by-side arrays, not a stack
                        }
                        let ygap = (b.y0 - a.y1).max(a.y0 - b.y1);
                        if ygap + half < link {
                            local.push((i, j));
                        }
                    }
                }
            }
            local.into_iter()
        })
        .collect();
    let mut ruf = UnionFind::new(runs.len());
    for (i, j) in r_edges {
        ruf.union(i, j);
    }
    let mut stacks: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..runs.len() {
        stacks.entry(ruf.find(i)).or_default().push(i);
    }

    // Under `axes: 2` the array is only in breach if something inside it is actually
    // tight; grouping used the looser `pitch`, so that has to be asked separately.
    // The first tight pair in the array, as (via, neighbour, across a row), so the
    // marker can sit on the gap that is short rather than on the array's centroid - a
    // 68-row stack's centroid lands between vias, on nothing anyone can look at.
    //
    // A pair counts only where the array is at least `rows` deep *over the pair*: more
    // than `rows` of the stack's runs have to span the pair's own x-range.  The blob
    // the pitch groups can carry more than the array - a 12x3 finger of vias at 0.28
    // hanging off a legal 4x28 block was reported as the block's, where the finger is
    // three rows and no part of any 4x4.  The foundry deck answers the same case by
    // dropping the whole blob when any part of it is thinner than four vias, which
    // also drops a tight pair in the block's own middle (`tail`); asking per pair keeps
    // that one and exempts the finger.
    let covered = |stack: &[usize], sx0: f64, sx1: f64| -> bool {
        stack
            .iter()
            .filter(|&&r| runs[r].x0 <= sx0 + half && runs[r].x1 >= sx1 - half)
            .count()
            > rows_thr
    };
    let tight_pair_in = |stack: &[usize], members: &[usize]| -> Option<(usize, usize, bool)> {
        let set: std::collections::HashSet<usize> = members.iter().copied().collect();
        members.iter().find_map(|&i| {
            let a = &vias[i];
            let (gx, gy) = ((a.cx / cell).floor() as i64, (a.cy / cell).floor() as i64);
            (-1..=1).find_map(|dx| {
                (-1..=1).find_map(|dy| {
                    {
                        grid.get((gx + dx, gy + dy)).find_map(|&j| {
                            if j == i || !set.contains(&j) {
                                return None;
                            }
                            let b = &vias[j];
                            // Neighbours in a row, or in a column, closer than the limit.
                            let xgap = (b.x0 - a.x1).max(a.x0 - b.x1);
                            let ygap = (b.y0 - a.y1).max(a.y0 - b.y1);
                            // Overlapping vias are one via; see the row linking above.
                            let row = a.y1.min(b.y1) - a.y0.max(b.y0) + half > projection.max(half)
                                && xgap >= 0.0
                                && xgap + half < value;
                            let col = a.x1.min(b.x1) - a.x0.max(b.x0) + half > projection.max(half)
                                && ygap >= 0.0
                                && ygap + half < value;
                            let (sx0, sx1) = if row {
                                (a.x0.min(b.x0), a.x1.max(b.x1))
                            } else {
                                (a.x0.max(b.x0), a.x1.min(b.x1))
                            };
                            ((row || col) && covered(stack, sx0, sx1)).then_some((i, j, row))
                        })
                    }
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
        let pair = if axes >= 2 {
            tight_pair_in(stack, &members)
        } else {
            None
        };
        if axes >= 2 && pair.is_none() {
            continue;
        }
        if min_extent > 0.0 {
            let (mut bx0, mut by0, mut bx1, mut by1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &i in &members {
                bx0 = bx0.min(vias[i].x0);
                by0 = by0.min(vias[i].y0);
                bx1 = bx1.max(vias[i].x1);
                by1 = by1.max(vias[i].y1);
            }
            if (bx1 - bx0).min(by1 - by0) + half < min_extent {
                continue;
            }
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
        let (mut bx0, mut by0, mut bx1, mut by1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &i in &members {
            bx0 = bx0.min(vias[i].x0);
            by0 = by0.min(vias[i].y0);
            bx1 = bx1.max(vias[i].x1);
            by1 = by1.max(vias[i].y1);
        }
        let what = format!(
            "{}×{} {} array ({:.2}×{:.2} µm)",
            stack.len(),
            min_cols,
            layer.name,
            bx1 - bx0,
            by1 - by0
        );
        if let Some((i, j, row)) = pair {
            // The segment across the gap, between the two facing walls.
            let (a, b) = (&vias[i], &vias[j]);
            let (x1, y1, x2, y2, gap) = if row {
                let y = (a.y0.max(b.y0) + a.y1.min(b.y1)) * 0.5;
                if a.x1 <= b.x0 {
                    (a.x1, y, b.x0, y, b.x0 - a.x1)
                } else {
                    (b.x1, y, a.x0, y, a.x0 - b.x1)
                }
            } else {
                let x = (a.x0.max(b.x0) + a.x1.min(b.x1)) * 0.5;
                if a.y1 <= b.y0 {
                    (x, a.y1, x, b.y0, b.y0 - a.y1)
                } else {
                    (x, b.y1, x, a.y0, a.y0 - b.y1)
                }
            };
            out.push(Violation::edge(
                rule.id.as_str(),
                "Via array spacing violation",
                format!(
                    "{what}: space {gap:.4} µm < {value:.2} µm at ({x1:.4}, {y1:.4})-({x2:.4}, {y2:.4}) µm"
                ),
                x1,
                y1,
                x2,
                y2,
            ));
        } else {
            out.push(Violation::point(
                rule.id.as_str(),
                "Via array spacing violation",
                format!("{what} below {value:.2} µm at ({cx:.4}, {cy:.4}) µm"),
                cx,
                cy,
            ));
        }
    }
    out
}

/// A hash grid as a sorted index: every (cell, item) pair in cell order, looked up by
/// binary search.  Built in parallel, which a map of vectors is not, and read from every
/// thread without sharing anything but a slice.
struct Grid {
    entries: Vec<((i64, i64), usize)>,
}

impl Grid {
    fn build(mut entries: Vec<((i64, i64), usize)>) -> Self {
        entries.par_sort_unstable();
        Self { entries }
    }

    /// The items filed under `cell`, as indices.
    fn get(&self, cell: (i64, i64)) -> impl Iterator<Item = &usize> + '_ {
        let start = self.entries.partition_point(|(k, _)| *k < cell);
        self.entries[start..]
            .iter()
            .take_while(move |(k, _)| *k == cell)
            .map(|(_, i)| i)
    }
}
