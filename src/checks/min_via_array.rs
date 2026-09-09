// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every region of a layer must hold a via array of at least a given size.
//!
//! `layers[0]` is the region that needs the array and `layers[1]` the vias: a region with
//! no `rows` × `cols` array of them anywhere in it is reported.
//!
//! GF180's MT30.8 is the rule this is for.  A via is small against three-micron top metal,
//! so one of them is not a connection and the DRM asks for a 2×2 array at one location.
//! Upstream reaches that answer sideways - it closes the vias together and asks whether
//! the blob is a rectangle wide enough - which needs a mitred size to come out right and
//! says nothing about arrays.  Counting rows and columns is what the rule says.
//!
//! "At one location" is what the grid does: two rows and two columns are only an array if
//! all four of their crossings hold a via, so four vias strung out in a line or bent into
//! an L do not pass for one.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, point_in_merged, stitch_labeled};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::{HashMap, HashSet};

/// Group values into bands of ones that sit within `tol` of each other, and return each
/// value's band index.  Vias in a row are drawn on a pitch, not to the nanometre.
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

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let (Some(host), Some(via)) = (rule.layers.first(), rule.layers.get(1)) else {
        eprintln!("[{}] min_via_array needs two layers", rule.id);
        return Vec::new();
    };
    let rows = rule.params.get("rows").copied().unwrap_or(2.0) as usize;
    let cols = rule.params.get("cols").copied().unwrap_or(2.0) as usize;
    if rows != 2 {
        eprintln!("[{}] min_via_array only implements `rows: 2`", rule.id);
        return Vec::new();
    }
    // `value` is how far apart two rows may be and still be one location.  Lining vias up
    // into rows and columns needs a much smaller tolerance than that - they are drawn on a
    // pitch, not to the nanometre, but a pitch is far coarser than the drift within a row.
    let reach = rule.value / dbu_to_um;
    let tol = reach / 10.0;

    let (hk, hd) = (host.gds_layer as i16, host.gds_datatype as i16);
    let (vk, vd) = (via.gds_layer as i16, via.gds_datatype as i16);
    println!(
        "[{}] Checking min_via_array {rows}x{cols} of {} in each {} region (within {:.2} µm)",
        rule.id, via.name, host.name, rule.value
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

    let mut violations = Vec::new();
    for (hid, pts) in per_host {
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let (col, row) = (bands(&xs, tol), bands(&ys, tol));
        // Which columns each row holds a via in.  Two rows are an array where they share
        // `cols` of them.
        let mut in_row: HashMap<usize, (HashSet<usize>, f64)> = HashMap::new();
        for i in 0..pts.len() {
            let e = in_row.entry(row[i]).or_insert((HashSet::new(), ys[i]));
            e.0.insert(col[i]);
            e.1 = e.1.min(ys[i]);
        }
        let sets: Vec<&(HashSet<usize>, f64)> = in_row.values().collect();
        // Two rows are an array where they share `cols` columns *and* are near enough each
        // other to be one location rather than two.
        let found = sets.iter().enumerate().any(|(i, a)| {
            sets[i + 1..]
                .iter()
                .any(|b| (a.1 - b.1).abs() <= reach && a.0.intersection(&b.0).count() >= cols)
        });
        if !found {
            let (cx, cy) = hosts.regions[hid].marker;
            violations.push(Violation::point(
                &rule.id,
                "Missing via array",
                format!(
                    "no {rows}x{cols} array of {} at one location in this {} ({} via(s) present)",
                    via.name,
                    host.name,
                    pts.len()
                ),
                cx * dbu_to_um,
                cy * dbu_to_um,
            ));
        }
    }
    violations
}
