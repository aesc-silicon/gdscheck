// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Space: the gap between two merged regions, on one layer or across two.
//!
//! One measurement, the closest approach of a region pair over the tiles in
//! [`helper::run_gated`](super::helper::run_gated), and a rule adds the bound and the
//! pairs it is about.  A plain rule is about every pair.  The gates narrow it: `angle:
//! bent` to pairs with a 45° wall at the gap, `net: same` or `net: different` to pairs
//! on one net or on two, `width` and `length` to pairs facing each other along a run
//! longer than so much with a line wider than so much on at least one side, `facing`
//! to pairs whose walls face each other across one axis - or, as `none`, nowhere, the
//! corner-to-corner reading.  The gates combine, and every one of them asks about the gap itself, not the shapes at large: a
//! long net bent somewhere else, or a wide rail that dips to a narrow tooth, does not
//! lend the condition to a gap it is not at.
//!
//! The same facing scan run inward reads how deeply two regions that share area
//! penetrate each other - `min_overlap`, KLayout's `overlap`: a salicide block that
//! must cover the COMP it blocks by 0.22 µm has to reach that far past the COMP's edge,
//! not merely touch it.  Not a separate engine, only the other direction through one.
//!
//! A rule with `rows` and `cols` is about the vias packed into an array, which want a
//! larger space than a lone pair; [`mod@array`] finds the arrays and reads their gaps.
//!
//! A gap between two walls of *one* region is a notch, read by [`notch`] with the width
//! scan turned the other way round; it takes `angle: bent` and `length` the same way.
//! And the other bound, that nothing of a layer lies *farther* than so much from
//! another, is [`max`]: a reach rule, read per part, per polygon or per edge.
//!
//! Two wires on adjacent routing tracks are read along the tracks by [`track`]: the run
//! they share, and the stagger of their tips.
//!
//! Declared on two *edge* layers, a plain rule measures between segments instead - see
//! [`edge_distance`](super::edge_distance).  Which one runs is the deck's choice of layer,
//! not a different rule; the gates read regions and have no meaning there.

pub mod array;
pub mod max;
pub mod notch;
pub mod track;

use super::helper::RunCtx;
use super::params::{Facing, NotAWord, bent_only, facing, mode};
use crate::connectivity::{Connectivity, LayerKey};
use crate::geom::{
    Limit, Marker, Outline, RunRead, faces_within, has_diagonal_within, on_grid, parallel_run,
    parallel_run_applies,
};
use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Which nets a rule's pairs are on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Net {
    /// Both regions on one net: the small minimum of a rule pair written as two
    /// potentials.
    Same,
    /// The regions on two nets, or not known to be on one: the large minimum.
    Different,
}

/// What a rule asks of a pair beyond its gap.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Gates {
    /// A 45° wall of one region within the gap of the other.
    pub bent: bool,
    /// The pair's nets, resolved at each region's marker.
    pub net: Option<Net>,
    /// Facing edges, one from each region, running alongside for more than `length` µm
    /// with a line deeper than `width` µm behind at least one of them.  Either alone
    /// is the other at zero.
    pub run: Option<(f64, f64)>,
    /// Walls of the two facing each other under the value across one axis, or - as
    /// [`Facing::Neither`] - facing nowhere under it, so that only a corner is closer.
    pub facing: Option<Facing>,
    /// With `facing: none`, how many of the two nearest corners lie in the
    /// reference region (boundary included). The deck can derive that region
    /// from metal with sufficient end-cap, without classifying a whole via.
    pub corner_cover: Option<(LayerKey, u8)>,
}

impl Gates {
    /// The gates a rule's params spell out; `None` if a param is malformed, which the
    /// rule has said and refuses to run on.
    pub fn from_rule(rule: &RuleDefinition, name: &str) -> Option<Gates> {
        let bent = bent_only(rule, name)?;
        let net = match mode(rule, name, "net") {
            Ok(None) => None,
            Ok(Some("same")) => Some(Net::Same),
            Ok(Some("different")) => Some(Net::Different),
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] {name}: net can only be `same` or `different`, not `{other}`",
                    rule.id
                );
                return None;
            }
            Err(NotAWord) => return None,
        };
        let (width, length) = (rule.num("width"), rule.num("length"));
        let run = (width.is_some() || length.is_some())
            .then(|| (width.unwrap_or(0.0), length.unwrap_or(0.0)));
        let facing = facing(rule, name, true)?;
        let corner_cover = if rule.params.contains_key("corner_cover")
            || rule.params.contains_key("covered_corners")
        {
            let (Some(layer), Some(dt), Some(count)) = (
                rule.num("corner_cover"),
                rule.num("corner_cover_dt"),
                rule.num("covered_corners"),
            ) else {
                eprintln!(
                    "[{}] {name}: corner_cover needs a layer_params entry and covered_corners",
                    rule.id
                );
                return None;
            };
            if facing != Some(Facing::Neither) || ![0.0, 1.0, 2.0].contains(&count) {
                eprintln!(
                    "[{}] {name}: corner_cover needs facing: none and covered_corners: 0, 1 or 2",
                    rule.id
                );
                return None;
            }
            Some(((layer as i16, dt as i16), count as u8))
        } else {
            None
        };
        Some(Gates {
            bent,
            net,
            run,
            facing,
            corner_cover,
        })
    }

    fn any(self) -> bool {
        self.bent
            || self.net.is_some()
            || self.run.is_some()
            || self.facing.is_some()
            || self.corner_cover.is_some()
    }
}

/// `min_space`: no pair closer than the value, among the pairs the params name.
pub fn run_min(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    // `rows` or `cols` gates the rule on membership in a via array, a different scan.
    if rule.params.contains_key("rows") || rule.params.contains_key("cols") {
        return array::run(rule, layout, dbu_to_um, merged);
    }
    let Some(gates) = Gates::from_rule(rule, "min_space") else {
        return vec![];
    };
    run_min_gated(rule, layout, dbu_to_um, merged, conn, gates)
}

/// `min_space` under `gates`, whether they came from the params or from a check name
/// that stands for them.
pub fn run_min_gated(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
    conn: Option<&Connectivity>,
    gates: Gates,
) -> Vec<Violation> {
    if super::edge_distance::on_edge_layers(rule, merged, "min_space") {
        if gates.any() {
            eprintln!(
                "[{}] min_space: the gates read regions, and an edge layer has none - \
                 name the regions, or drop the params",
                rule.id
            );
            return vec![];
        }
        return super::edge_distance::run_space(rule, layout, dbu_to_um, merged);
    }
    // A net gate needs the nets.  Without extraction the rule is skipped upstream; a
    // deck that reaches here with none has asked for it and not got it.
    let nets = match gates.net {
        None => None,
        Some(which) => {
            let Some(conn) = conn else {
                eprintln!("[{}] min_space: `net` needs connectivity", rule.id);
                return vec![];
            };
            Some((which, conn, net_keys(rule, conn, which)))
        }
    };
    // The gates apply at the violating gap.  The engine reports a pair under the value
    // on the grid, and the same bound picks the facing pair that is the violation: a
    // wide rail running alongside a wire at the clean gap must not lend its width, nor
    // a bend elsewhere on the net its angle, to a narrow tooth that dips below it.
    let limit = Limit::at_least(rule.value, dbu_to_um).dbu();
    let cover = gates.corner_cover.map(|(key, count)| {
        merged.ensure(layout, key.0, key.1);
        (merged.tiles(key.0, key.1), count)
    });
    let tile_dbu = merged.tile_dbu() as f64;
    let run = gates.run.map(|(w, l)| {
        (
            on_grid(w / dbu_to_um, f64::round),
            on_grid(l / dbu_to_um, f64::round),
        )
    });
    super::helper::run_gated(
        rule,
        layout,
        dbu_to_um,
        merged,
        move |a: &Outline, b: &Outline, ma: Marker, mb: Marker, ctx: &RunCtx| {
            if gates.bent && !(has_diagonal_within(a, b, limit) || has_diagonal_within(b, a, limit))
            {
                return false;
            }
            // Facing is symmetric - a wall of one facing a wall of the other - so one
            // direction reads it.
            match gates.facing {
                Some(Facing::Along(axis)) if !faces_within(a, b, limit, Some(axis)) => {
                    return false;
                }
                Some(Facing::Neither) if faces_within(a, b, limit, None) => return false,
                _ => {}
            }
            if let Some((tiles, count)) = &cover {
                let Some((_, _, pa, pb)) = crate::geom::closest_approach(a, b, limit) else {
                    return false;
                };
                let covered = |(x, y): (f64, f64)| {
                    // The reference may end exactly on a core boundary, leaving
                    // no area in the point's owning tile. Read both incident
                    // tiles on each such axis: closed-set membership includes
                    // the boundary even when its reference has no right/top tile.
                    let (tx, ty) = ((x / tile_dbu).floor() as i32, (y / tile_dbu).floor() as i32);
                    let x0 = tx - i32::from(x == tx as f64 * tile_dbu);
                    let y0 = ty - i32::from(y == ty as f64 * tile_dbu);
                    (x0..=tx).any(|cx| {
                        (y0..=ty).any(|cy| {
                            tiles.get(&(cx, cy)).is_some_and(|polys| {
                                polys
                                    .iter()
                                    .any(|p| crate::merge::point_on_or_in_merged(x, y, p))
                            })
                        })
                    })
                };
                if u8::from(covered(pa)) + u8::from(covered(pb)) != *count {
                    return false;
                }
            }
            if let Some((wide, min_run)) = run
                && let read = parallel_run(a, b, limit, wide, min_run, Some(ctx.zone))
                && read != RunRead::Applies
            {
                // Read within the zone the copies are exact in, the run may have been
                // cut short: a line 60 µm long in a 20 µm tile.  Where a facing stretch
                // reaches the zone's edge, the pair's regions are assembled out to the
                // rule's reach and read again, exact in that box.  A copy at the edge
                // with no core piece to name its region - a shape touching the core at
                // a tile line - is left to the tile on the other side, which has the
                // piece.
                if read == RunRead::Clean || !(ctx.cut(a) || ctx.cut(b)) {
                    return false;
                }
                // The box is the zone grown by the reach, the same for every pair in
                // the tile, so a rail's assembly serves every wire beside it.
                let reach = limit + min_run + wide + 1;
                let (zx0, zy0, zx1, zy1) = ctx.zone;
                let box_ = (zx0 - reach, zy0 - reach, zx1 + reach, zy1 + reach);
                let (wa, wb) = ctx.within(box_);
                fn assembled<'p>(
                    cut: bool,
                    w: &'p super::helper::Within,
                ) -> Option<Vec<Outline<'p>>> {
                    match (cut, w) {
                        (false, _) => Some(vec![]),
                        (true, Some(v)) => Some(v.iter().map(Outline::new).collect()),
                        (true, None) => None,
                    }
                }
                let (Some(oa), Some(ob)) = (assembled(ctx.cut(a), &wa), assembled(ctx.cut(b), &wb))
                else {
                    return false;
                };
                let oa: Vec<&Outline> = if oa.is_empty() {
                    vec![a]
                } else {
                    oa.iter().collect()
                };
                let ob: Vec<&Outline> = if ob.is_empty() {
                    vec![b]
                } else {
                    ob.iter().collect()
                };
                if !oa.iter().any(|x| {
                    ob.iter()
                        .any(|y| parallel_run_applies(x, y, limit, wide, min_run, Some(box_)))
                }) {
                    return false;
                }
            }
            if let Some((which, conn, (key_a, key_b))) = nets {
                let na = conn.net_at(key_a, ma.0, ma.1);
                let nb = conn.net_at(key_b, mb.0, mb.1);
                let one_net = matches!((na, nb), (Some(a), Some(b)) if a == b);
                // A pair not known to be one net is not shown to be one, so `same` goes
                // quiet on it and `different` reports it: an unresolved net can cost a
                // false positive on the large minimum, never a false clean, and the
                // small minimum's companion rule owns the pair either way.
                if (which == Net::Same) != one_net {
                    return false;
                }
            }
            true
        },
    )
}

/// The layer keys a net gate resolves on, said aloud when one is outside the connect
/// graph: `same` then finds no pair at all, `different` reports every pair.
fn net_keys(rule: &RuleDefinition, conn: &Connectivity, which: Net) -> (LayerKey, LayerKey) {
    let key = |i: usize| -> LayerKey {
        let l = rule.layers.get(i).unwrap_or(&rule.layers[0]);
        (l.gds_layer as i16, l.gds_datatype as i16)
    };
    let keys = (key(0), key(1));
    for (k, i) in [(keys.0, 0usize), (keys.1, 1)] {
        if conn.node_base(k).is_none() {
            let name = &rule.layers.get(i).unwrap_or(&rule.layers[0]).name;
            match which {
                Net::Same => eprintln!(
                    "[{}] layer '{name}' is not in the connect graph - no pair can resolve \
                     as same-net, so this rule will report nothing",
                    rule.id
                ),
                Net::Different => eprintln!(
                    "[{}] layer '{name}' is not in the connect graph - reporting every pair",
                    rule.id
                ),
            }
        }
    }
    keys
}

/// `min_overlap`: no two regions of the two layers overlapping by less than the value,
/// read where they face each other across material of both.
pub fn run_min_overlap(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    super::helper::run_overlap(rule, layout, dbu_to_um, merged)
}
