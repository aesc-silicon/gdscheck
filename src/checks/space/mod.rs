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
//! longer than so much with a line wider than so much on at least one side.  The gates
//! combine, and every one of them asks about the gap itself, not the shapes at large: a
//! long net bent somewhere else, or a wide rail that dips to a narrow tooth, does not
//! lend the condition to a gap it is not at.
//!
//! Declared on two *edge* layers, a plain rule measures between segments instead - see
//! [`edge_distance`](super::edge_distance).  Which one runs is the deck's choice of layer,
//! not a different rule; the gates read regions and have no meaning there.

use super::params::{NotAWord, bent_only, mode};
use crate::connectivity::{Connectivity, LayerKey};
use crate::geom::{Marker, Poly};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
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
        Some(Gates { bent, net, run })
    }

    fn any(self) -> bool {
        self.bent || self.net.is_some() || self.run.is_some()
    }
}

/// `min_space`: no pair closer than the value, among the pairs the params name.
pub fn run_min(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
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
    merged: &mut MergedCache,
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
    // The gates apply at the violating gap.  The engine reports a pair under
    // `value - half`, so the same bound picks the facing pair that is the violation: a
    // wide rail running alongside a wire at the clean gap must not lend its width, nor
    // a bend elsewhere on the net its angle, to a narrow tooth that dips below it.
    let max_gap = rule.value - 0.5 * dbu_to_um;
    super::helper::run_gated(
        rule,
        layout,
        dbu_to_um,
        merged,
        move |a: &Poly, b: &Poly, ma: Marker, mb: Marker| {
            if gates.bent && !(a.has_diagonal_near(b, max_gap) || b.has_diagonal_near(a, max_gap)) {
                return false;
            }
            if let Some((width, length)) = gates.run
                && !a.prl_applies(b, max_gap, width, length)
            {
                return false;
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

/// The check names that stood for one gate each before the gates were params.
pub fn run_min_named(
    name: &str,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let gates = match name {
        "min_space_bent" => Gates {
            bent: true,
            ..Gates::default()
        },
        "min_space_same_net" => Gates {
            net: Some(Net::Same),
            ..Gates::default()
        },
        "min_space_different_net" => Gates {
            net: Some(Net::Different),
            ..Gates::default()
        },
        "min_space_prl" => {
            let (Some(width), Some(length)) = (rule.num("wide_width"), rule.num("parallel_run"))
            else {
                eprintln!(
                    "[{}] min_space_prl needs `wide_width` and `parallel_run` params",
                    rule.id
                );
                return vec![];
            };
            Gates {
                run: Some((width, length)),
                ..Gates::default()
            }
        }
        _ => unreachable!("not a spacing name"),
    };
    run_min_gated(rule, layout, dbu_to_um, merged, conn, gates)
}
