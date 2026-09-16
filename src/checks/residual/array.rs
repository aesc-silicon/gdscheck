// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `forbidden` with `op: apart` and `rows`/`cols`: a region of `layers[0]` is apart
//! from its partners unless it holds an array of them at least `rows` × `cols` large.
//!
//! GF180's MT30.8 is the rule this is for.  A via is small against three-micron top
//! metal, so one of them is not a connection and the DRM asks for a 2×2 array at one
//! location.  Upstream reaches that answer sideways - it closes the vias together and
//! asks whether the blob is a rectangle wide enough - which needs a mitred size to come
//! out right and says nothing about arrays.  Counting rows and columns is what the rule
//! says.
//!
//! "At one location" is what the grid does: the rows and columns are only an array if
//! every crossing holds a via, so four vias strung out in a line or bent into an L do
//! not pass for one, and `value` is how far apart two rows may be and still be one
//! location.  Vias are lined up into rows and columns by their markers with a tenth of
//! that as the tolerance: they are drawn on a pitch, not to the nanometre, and a pitch
//! is far coarser than the drift within a row.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, point_in_merged, stitch_labeled};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::{HashMap, HashSet};

/// Group values into bands of ones that sit within `tol` of each other, and return each
/// value's band index.
fn bands(vals: &[f64], tol: f64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..vals.len()).collect();
    order.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap());
    let mut band = vec![0usize; vals.len()];
    let (mut b, mut last) = (0usize, f64::NEG_INFINITY);
    for (n, &i) in order.iter().enumerate() {
        if n > 0 && vals[i] - last > tol {
            b += 1;
        }
        band[i] = b;
        last = vals[i];
    }
    band
}

/// Whether `rows` of the row bands, each within `reach` of the next, share `cols`
/// columns between them.  `bands` is sorted by y; the search walks it upward from every
/// start, keeping the columns the rows so far have in common.
fn has_array(bands: &[(f64, HashSet<usize>)], rows: usize, cols: usize, reach: f64) -> bool {
    fn walk(
        bands: &[(f64, HashSet<usize>)],
        at: usize,
        shared: &HashSet<usize>,
        left: usize,
        cols: usize,
        reach: f64,
    ) -> bool {
        if left == 0 {
            return true;
        }
        bands[at + 1..].iter().enumerate().any(|(k, (y, set))| {
            if y - bands[at].0 > reach {
                return false;
            }
            let common: HashSet<usize> = shared.intersection(set).copied().collect();
            common.len() >= cols && walk(bands, at + 1 + k, &common, left - 1, cols, reach)
        })
    }
    (0..bands.len())
        .any(|i| bands[i].1.len() >= cols && walk(bands, i, &bands[i].1, rows - 1, cols, reach))
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    rows: usize,
    cols: usize,
) -> Vec<Violation> {
    let (host, via) = (&rule.layers[0], &rule.layers[1]);
    if rule.layers.len() > 2 {
        eprintln!(
            "[{}] forbidden: an array is of one partner layer, not of {}",
            rule.id,
            rule.layers.len() - 1
        );
        return Vec::new();
    }
    if rows == 0 || cols == 0 {
        eprintln!("[{}] forbidden: `rows` and `cols` are at least 1", rule.id);
        return Vec::new();
    }
    let reach = rule.value / dbu_to_um;
    let tol = reach / 10.0;

    let (hk, hd) = (host.gds_layer as i16, host.gds_datatype as i16);
    let (vk, vd) = (via.gds_layer as i16, via.gds_datatype as i16);
    println!(
        "[{}] Checking forbidden: {} without a {rows}x{cols} array of {} (rows within {:.2} µm)",
        rule.id, host.name, via.name, rule.value
    );
    merged.ensure(layout, hk, hd);
    merged.ensure(layout, vk, vd);

    let tile = merged.tile_dbu();
    let hosts = stitch_labeled(merged.tiles(hk, hd), tile);
    let vias = stitch_labeled(merged.tiles(vk, vd), tile);

    let mut per_host: HashMap<usize, Vec<(f64, f64)>> = HashMap::new();
    for region in &vias.regions {
        let (x, y) = region.marker;
        let key = (
            (x as i64).div_euclid(tile as i64) as i32,
            (y as i64).div_euclid(tile as i64) as i32,
        );
        let Some(here) = hosts.by_tile.get(&key) else {
            continue;
        };
        if let Some((_, hid)) = here.iter().find(|(p, _)| point_in_merged(x, y, p)) {
            per_host.entry(*hid).or_default().push((x, y));
        }
    }

    let mut out: Vec<(usize, usize)> = Vec::new(); // (host, vias present)
    for (hid, _) in hosts.regions.iter().enumerate() {
        let pts = per_host.get(&hid).map(Vec::as_slice).unwrap_or(&[]);
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let (col, row) = (bands(&xs, tol), bands(&ys, tol));
        // The columns each row holds a via in, and the row's lowest y.
        let mut in_row: HashMap<usize, (f64, HashSet<usize>)> = HashMap::new();
        for i in 0..pts.len() {
            let e = in_row.entry(row[i]).or_insert((ys[i], HashSet::new()));
            e.1.insert(col[i]);
            e.0 = e.0.min(ys[i]);
        }
        let mut sorted: Vec<(f64, HashSet<usize>)> = in_row.into_values().collect();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if !has_array(&sorted, rows, cols, reach) {
            out.push((hid, pts.len()));
        }
    }
    out.sort_unstable();
    out.into_iter()
        .map(|(hid, present)| {
            let (cx, cy) = hosts.regions[hid].marker;
            Violation::point(
                &rule.id,
                "Missing via array",
                format!(
                    "no {rows}x{cols} array of {} at one location in this {} ({present} via(s) present) at ({:.4}, {:.4}) µm",
                    via.name,
                    host.name,
                    cx * dbu_to_um,
                    cy * dbu_to_um
                ),
                cx * dbu_to_um,
                cy * dbu_to_um,
            )
        })
        .collect()
}
