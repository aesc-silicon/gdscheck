// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::boundaries_on;
use super::helper::point_in_polygon;
use crate::layout::FlatLayout;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::VecDeque;

pub fn run(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    let layer = &rule.layers[0];

    println!(
        "[{}] Checking no LBE ring on layer {} ({}/{})",
        rule.id, layer.name, layer.gds_layer, layer.gds_datatype
    );

    let polys: Vec<Vec<(f64, f64)>> = boundaries_on(layout, layer)
        .iter()
        .map(|b| {
            let n = b.xy.len().saturating_sub(1);
            b.xy[..n]
                .iter()
                .map(|p| (p.x as f64 * dbu_to_um, p.y as f64 * dbu_to_um))
                .collect()
        })
        .collect();

    if polys.is_empty() {
        return vec![];
    }

    // Build sorted, deduplicated lists of all x and y coordinates from polygon vertices.
    let mut xs: Vec<f64> = polys.iter().flat_map(|p| p.iter().map(|&(x, _)| x)).collect();
    let mut ys: Vec<f64> = polys.iter().flat_map(|p| p.iter().map(|&(_, y)| y)).collect();

    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    // Add sentinel strips outside the bounding box so the exterior is reachable.
    let margin = dbu_to_um; // one DBU of margin
    xs.insert(0, xs[0] - margin);
    xs.push(*xs.last().unwrap() + margin);
    ys.insert(0, ys[0] - margin);
    ys.push(*ys.last().unwrap() + margin);

    let nx = xs.len() - 1;
    let ny = ys.len() - 1;

    // For each cell, determine whether its centre is covered by any LBE polygon.
    let covered: Vec<Vec<bool>> = (0..nx)
        .map(|i| {
            (0..ny)
                .map(|j| {
                    let cx = (xs[i] + xs[i + 1]) / 2.0;
                    let cy = (ys[j] + ys[j + 1]) / 2.0;
                    polys.iter().any(|poly| point_in_polygon(cx, cy, poly))
                })
                .collect()
        })
        .collect();

    // Flood-fill from cell (0, 0) — which is always in the exterior sentinel strip —
    // through uncovered cells to find all empty space reachable from outside.
    let mut reached = vec![vec![false; ny]; nx];
    let mut queue = VecDeque::new();
    reached[0][0] = true;
    queue.push_back((0usize, 0usize));

    while let Some((i, j)) = queue.pop_front() {
        for (di, dj) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let ni = i as i32 + di;
            let nj = j as i32 + dj;
            if ni < 0 || nj < 0 || ni >= nx as i32 || nj >= ny as i32 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if !covered[ni][nj] && !reached[ni][nj] {
                reached[ni][nj] = true;
                queue.push_back((ni, nj));
            }
        }
    }

    // Any uncovered cell not reached from outside is inside a hole.
    // Find connected components of such cells and report one violation per hole.
    let mut visited = vec![vec![false; ny]; nx];
    let mut violations = vec![];

    for si in 0..nx {
        for sj in 0..ny {
            if covered[si][sj] || reached[si][sj] || visited[si][sj] {
                continue;
            }

            // BFS to collect all cells in this hole region.
            let mut component: Vec<(usize, usize)> = vec![];
            let mut queue = VecDeque::new();
            visited[si][sj] = true;
            queue.push_back((si, sj));

            while let Some((i, j)) = queue.pop_front() {
                component.push((i, j));
                for (di, dj) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
                    let ni = i as i32 + di;
                    let nj = j as i32 + dj;
                    if ni < 0 || nj < 0 || ni >= nx as i32 || nj >= ny as i32 {
                        continue;
                    }
                    let (ni, nj) = (ni as usize, nj as usize);
                    if !covered[ni][nj] && !reached[ni][nj] && !visited[ni][nj] {
                        visited[ni][nj] = true;
                        queue.push_back((ni, nj));
                    }
                }
            }

            // Report at the centroid of the hole region.
            let n = component.len() as f64;
            let cx = component.iter().map(|&(i, _)| (xs[i] + xs[i + 1]) / 2.0).sum::<f64>() / n;
            let cy = component.iter().map(|&(_, j)| (ys[j] + ys[j + 1]) / 2.0).sum::<f64>() / n;

            violations.push(Violation::point(
                &rule.id,
                "LBE ring violation",
                format!(
                    "{} forms a closed ring (enclosed empty region) at ({:.4}, {:.4}) µm",
                    layer.name, cx, cy
                ),
                cx, cy,
            ));
        }
    }

    violations
}
