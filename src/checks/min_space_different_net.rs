// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimum spacing between regions *on different nets*.
//!
//! The geometric twin of [`min_space`](super::min_space): same tiled region-pair engine,
//! but a pair whose two regions resolve to the same net is not a violation.  Models the
//! "different potential" spacing rules — IHP NW.b1, where wells at the same potential are
//! allowed to sit closer than wells that are not (the same-potential minimum is the
//! separate NW.b).
//!
//! The net comes from the rule's own layers, which must therefore appear as conductors in
//! the PDK's connect graph.  For NW.b1 that layer is `NWellMergedNoSRAM`, the close-merged
//! well: the merge is what makes the lookup sound, because a merged region's marker can
//! land in a gap the close filled in, which is inside the merged layer but outside any
//! drawn NWell.
//!
//! Fails conservative in every uncertain case — an unresolved marker, a layer missing from
//! the connect graph, an untied (floating) region with no net of its own — all report,
//! exactly as the plain geometric rule would.  A wrong net can therefore only ever cost a
//! false positive, never a false clean.

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
        eprintln!("[{}] min_space_different_net needs connectivity", rule.id);
        return vec![];
    };

    let key = |i: usize| -> LayerKey {
        let l = rule.layers.get(i).unwrap_or(&rule.layers[0]);
        (l.gds_layer as i16, l.gds_datatype as i16)
    };
    let (key_a, key_b) = (key(0), key(1));

    // Without the layer in the connect graph there is no net to compare, so every pair
    // stays a violation — the rule degrades to plain min_space rather than going quiet.
    for (k, l) in [(key_a, 0usize), (key_b, 1)] {
        if conn.node_base(k).is_none() {
            eprintln!(
                "[{}] layer '{}' is not in the connect graph — reporting every pair",
                rule.id,
                rule.layers.get(l).unwrap_or(&rule.layers[0]).name
            );
        }
    }

    super::helper::run_gated(rule, layout, dbu_to_um, merged, |_, _, ma: Marker, mb: Marker| {
        let na = conn.net_at(key_a, ma.0, ma.1);
        let nb = conn.net_at(key_b, mb.0, mb.1);
        match (na, nb) {
            // Both resolved and equal: one net, so the different-net rule does not apply.
            (Some(a), Some(b)) => a != b,
            _ => true,
        }
    })
}
