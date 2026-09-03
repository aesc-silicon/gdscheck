// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! How many different nets one marker may cover.
//!
//! `layers[0]` is the marker and `layers[1]` what lies beneath it; the regions of the
//! latter under each marker region are resolved to nets, and a marker covering more than
//! `value` of them is reported.
//!
//! `layers[2]`, if given, is where the net is looked up.  It is wanted whenever the
//! geometry worth counting is a derived layer - NAT.6 counts the active *under the
//! marker*, which is an intersection, and an intersection is in no connect graph.  The
//! shapes come from the derived layer and the net from the drawn one.
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
    let net_key = rule
        .layers
        .get(2)
        .map_or((ck, cd), |l| (l.gds_layer as i16, l.gds_datatype as i16));
    println!(
        "[{}] Checking max_nets_under <= {} of {} beneath each {} region",
        rule.id, rule.value as i64, cond.name, mark.name
    );
    merged.ensure(layout, mk, md);
    merged.ensure(layout, ck, cd);

    let tile = merged.tile_dbu();
    let marks = stitch_labeled(merged.tiles(mk, md), tile);
    let conds = stitch_labeled(merged.tiles(ck, cd), tile);

    // Each conductor region is placed under whichever marker region holds its own marker
    // point, and contributes its net there.  An unresolved net is its own, which is what
    // keeps the check from going quiet on geometry it cannot follow.
    let mut nets: HashMap<usize, HashSet<i64>> = HashMap::new();
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
        // An unresolved region counts as a net of its own: the check would rather report
        // geometry it cannot follow than go quiet on it.
        let net = conn
            .net_at(net_key, x * dbu_to_um, y * dbu_to_um)
            .map_or(-(rid as i64) - 1, |n| n as i64);
        nets.entry(*mrid).or_default().insert(net);
    }

    let limit = rule.value as usize;
    let mut violations = Vec::new();
    for (mrid, seen) in nets {
        if seen.len() > limit {
            // One violation per marker: the regions under it are all part of the same
            // fault, and reporting each separately reads as several.
            let (cx, cy) = marks.regions[mrid].marker;
            violations.push(Violation::point(
                &rule.id,
                "Too many nets under one marker",
                format!(
                    "{} regions on {} different nets lie under this {}, at most {} allowed",
                    cond.name,
                    seen.len(),
                    mark.name,
                    limit
                ),
                cx * dbu_to_um,
                cy * dbu_to_um,
            ));
        }
    }
    violations
}
