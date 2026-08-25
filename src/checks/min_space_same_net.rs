// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum spacing between regions *on the same net*.
//!
//! The mirror of [`min_space_different_net`](super::min_space_different_net), and the
//! other half of the pattern foundries write as one rule pair: two shapes at the same
//! potential may sit closer together than two at different potentials, so the deck states
//! a small same-net minimum and a larger different-net one. GF180's `DN.2a`/`DN.2b` are
//! 2.5 µm and 5.42 µm on DNWELL; upstream derives both from one `conn_space` call.
//!
//! Using a plain `min_space` for the same-net half is wrong in a way that is easy to miss:
//! it also flags the different-net pairs below the *smaller* value, which the companion
//! rule already owns. That was four of `dnwell`'s false positives.
//!
//! Unlike its mirror, this one goes **quiet** when a net will not resolve rather than
//! reporting. Reporting would be the false positive here — an unresolved pair is not
//! known to be one net — and nothing is lost by it: the pair is below the same-net
//! minimum, therefore below the larger different-net one, so the companion rule reports
//! it. That reasoning depends on the pair being written as a pair, which is how these
//! rules come; a same-net rule standing alone would need the opposite default.

use crate::checks::helper::Marker;
use crate::connectivity::{Connectivity, LayerKey};
use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    conn: Option<&Connectivity>,
) -> Vec<Violation> {
    let Some(conn) = conn else {
        eprintln!("[{}] min_space_same_net needs connectivity", rule.id);
        return vec![];
    };

    let key = |i: usize| -> LayerKey {
        let l = rule.layers.get(i).unwrap_or(&rule.layers[0]);
        (l.gds_layer as i16, l.gds_datatype as i16)
    };
    let (key_a, key_b) = (key(0), key(1));

    // A layer outside the connect graph has no net to compare, so nothing can be shown to
    // be same-net and the rule reports nothing. Say so — silence here would read as clean.
    for (k, l) in [(key_a, 0usize), (key_b, 1)] {
        if conn.node_base(k).is_none() {
            eprintln!(
                "[{}] layer '{}' is not in the connect graph — no pair can resolve as \
                 same-net, so this rule will report nothing",
                rule.id,
                rule.layers.get(l).unwrap_or(&rule.layers[0]).name
            );
        }
    }

    super::helper::run_gated(
        rule,
        layout,
        dbu_to_um,
        merged,
        |_, _, ma: Marker, mb: Marker| {
            let na = conn.net_at(key_a, ma.0, ma.1);
            let nb = conn.net_at(key_b, mb.0, mb.1);
            matches!((na, nb), (Some(a), Some(b)) if a == b)
        },
    )
}
