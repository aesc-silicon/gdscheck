// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! How many different nets one marker may cover.
//!
//! `layers[0]` is the marker and `layers[1]` what lies beneath it; the regions of the
//! latter under each marker region are resolved to nets, and a marker covering more than
//! `value` of them is reported.
//!
//! The `net_of` layer param, if given, is where the net is looked up.  It is wanted
//! whenever the geometry worth counting is a derived layer - NAT.6 counts the active
//! *under the marker*, which is an intersection, and an intersection is in no connect
//! graph.  The shapes come from the derived layer and the net from the drawn one.
//!
//! GF180's NAT.6 is the rule this exists for - "two or more COMPs at different potential
//! are not allowed under the same NAT layer" - and it is worth saying that upstream does
//! not write it this way.  It builds the different-potential pairs with a net-aware
//! *spacing* helper and then asks which markers touch them, which is the same answer
//! reached through a measurement the rule never mentions.  Counting the nets under the
//! marker is what the rule says.
//!
//! Fails conservative, like the other net-aware checks: a conductor region whose net will
//! not resolve counts as a net of its own, so an unresolved lookup can only ever cost a
//! false positive.

use crate::connectivity::Connectivity;
use crate::layout::FlatLayout;
use crate::merge::{MergedCache, point_in_merged, stitch_labeled};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::{HashMap, HashSet};

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let (Some(mark), Some(cond)) = (rule.layers.first(), rule.layers.get(1)) else {
        eprintln!("[{}] max_nets_under needs two layers", rule.id);
        return Vec::new();
    };
    let Some(conn) = conn else {
        eprintln!("[{}] max_nets_under needs connectivity", rule.id);
        return Vec::new();
    };
    let (mk, md) = (mark.gds_layer as i16, mark.gds_datatype as i16);
    let (ck, cd) = (cond.gds_layer as i16, cond.gds_datatype as i16);
    // The conductor the nets are looked up on, when what is counted is a derived layer:
    // the `net_of` layer param, or the old third layer until the deck has moved.
    let net_key = rule
        .num("net_of")
        .map(|l| (l as i16, rule.num("net_of_dt").unwrap_or(0.0) as i16))
        .or_else(|| {
            rule.layers
                .get(2)
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        })
        .unwrap_or((ck, cd));
    println!(
        "[{}] Checking max_nets_under <= {} of {} beneath each {} region",
        rule.id, rule.value as i64, cond.name, mark.name
    );
    merged.ensure(layout, mk, md);
    merged.ensure(layout, ck, cd);
    merged.ensure(layout, net_key.0, net_key.1);

    let tile = merged.tile_dbu();
    let marks = stitch_labeled(merged.tiles(mk, md), tile);
    let conds = stitch_labeled(merged.tiles(ck, cd), tile);
    let net_tiles = merged.tiles(net_key.0, net_key.1);
    // The pieces of each counted region, tile by tile, for the net lookup below.
    let mut pieces: Vec<Vec<((i32, i32), &crate::merge::MergedPoly)>> =
        vec![Vec::new(); conds.regions.len()];
    for (tkey, polys) in &conds.by_tile {
        for (poly, rid) in polys {
            pieces[*rid].push((*tkey, poly));
        }
    }

    // Each conductor region is placed under whichever marker region holds its own marker
    // point, and carries the nets of every conductor inside it - a COMP holds its source
    // and its drain, which are two nets on one shape, and reading only the first found
    // made the answer depend on which tile the lookup walked into first.  Two regions
    // that share a net are at one potential; the rule counts potentials, not nets, so
    // the regions under a marker are grouped by the nets they have in common.
    let mut by_mark: HashMap<usize, Vec<HashSet<i64>>> = HashMap::new();
    for (rid, region) in conds.regions.iter().enumerate() {
        let (x, y) = region.marker;
        let tkey = (
            (x as i64).div_euclid(tile as i64) as i32,
            (y as i64).div_euclid(tile as i64) as i32,
        );
        let Some(under) = marks.by_tile.get(&tkey) else {
            continue;
        };
        let Some((_, mrid)) = under.iter().find(|(p, _)| point_in_merged(x, y, p)) else {
            continue;
        };
        let mut nets: HashSet<i64> = HashSet::new();
        if let Some(n) = conn.net_at(net_key, x, y) {
            nets.insert(n as i64);
        }
        // The conductor the nets are read on need not reach the region's marker point -
        // NAT.6 counts whole actives and reads their net on the contacted active, which
        // the gate cuts a hole in - so every conductor region inside this one is asked.
        for (ptile, piece) in &pieces[rid] {
            let Some(polys) = net_tiles.get(ptile) else {
                continue;
            };
            for np in polys {
                let (nx, ny) = crate::merge::inside_point(np);
                if point_in_merged(nx, ny, piece)
                    && let Some(n) = conn.net_at(net_key, nx, ny)
                {
                    nets.insert(n as i64);
                }
            }
        }
        // An unresolved region counts as a potential of its own: the check would rather
        // report geometry it cannot follow than go quiet on it.
        if nets.is_empty() {
            nets.insert(-(rid as i64) - 1);
        }
        by_mark.entry(*mrid).or_default().push(nets);
    }

    let limit = rule.value as usize;
    let mut violations = Vec::new();
    let mut found: Vec<(usize, usize)> = Vec::new();
    for (mrid, regions) in by_mark {
        let mut uf = crate::merge::UnionFind::new(regions.len());
        let mut first: HashMap<i64, usize> = HashMap::new();
        for (i, nets) in regions.iter().enumerate() {
            for &n in nets {
                match first.entry(n) {
                    std::collections::hash_map::Entry::Occupied(e) => uf.union(i, *e.get()),
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(i);
                    }
                }
            }
        }
        let groups: HashSet<usize> = (0..regions.len()).map(|i| uf.find(i)).collect();
        if groups.len() > limit {
            found.push((mrid, groups.len()));
        }
    }
    // One violation per marker, in a fixed order: the regions under it are all part of
    // the same fault, and reporting each separately reads as several.
    found.sort_unstable();
    for (mrid, n) in found {
        let (cx, cy) = marks.regions[mrid].marker;
        violations.push(Violation::point(
            &rule.id,
            "Too many nets under one marker",
            format!(
                "{} regions at {} different potentials lie under this {}, at most {} allowed",
                cond.name, n, mark.name, limit
            ),
            cx * dbu_to_um,
            cy * dbu_to_um,
        ));
    }
    violations
}
