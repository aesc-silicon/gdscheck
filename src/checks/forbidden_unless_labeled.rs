// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forbidden-region check with a text-labelled exemption — a *candidate* device that lands
//! in a *forbidden* region is an error, unless it belongs to an isolation structure tagged
//! by a text label.  Every layer role is supplied positionally and the exemption label is
//! the rule's `text`, so another PDK can reuse it.  IHP antenna Ant.h ("dantenna in NWell
//! not allowed", exempting text-tagged `isolbox` structures) is its first user; it mirrors
//! `antenna.drc`:
//!
//! ```text
//! nactiv_con   = Activ − GatPoly − (pSD ∪ nSD.block)
//! cand         = nactiv_con ∩ Recog.diode − Recog.esd
//! nact_nwell   = nactiv_con ∩ (NWell ∪ PWell.block)
//! schottky     = nBuLay ∩ Recog.diode ∩ NWell ∩ nSD.block
//! isolbox_1    = (nBuLay  interacting Recog.diode interacting nact_nwell)
//!              ∪ (Recog.diode interacting nBuLay  interacting nact_nwell)
//! isolbox      = isolbox_1 not_interacting schottky, then interacting text <rule.text>
//! error        = (cand not_interacting isolbox) ∩ NWell
//! ```
//!
//! `layers` (positional) =
//! [Activ, GatPoly, pSD, nSD.block, Recog.diode, Recog.esd, NWell, PWell.block, nBuLay, TEXT];
//! `text` = the exemption label (e.g. "isolbox").
//!
//! Every step above is registered as a lazy tiled virtual layer ([`MergedCache`]) instead
//! of computed as one whole-chip boolean: the plain union/intersection/difference steps
//! are already tile-local (a point's membership only depends on nearby geometry), and the
//! "interacting"/"with text" steps use the *same* whole-region stitching
//! ([`stitch_labeled`](crate::merge::stitch_labeled)) that [`MergedCache::regions`] already
//! relies on for e.g. metal-plate density — a region's membership is decided by OR-ing a
//! per-tile test across all of its pieces, not by reconstructing or globally unioning the
//! source geometry.  This is what makes the check safe on a dense, chip-wide layer like
//! `Activ` or `GatPoly`: the old implementation unioned those globally (via
//! `merge_boundaries`/`compose_tile` over an anchor-clipped copy), which is fine for a
//! small block but OOMs once real antenna-diode markers are as numerous and widespread as
//! they are on a full SoC top cell.

use crate::layout::FlatLayout;
use crate::merge::{MergedCache, VirtualOp};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Reserved GDS-layer range for this check's synthetic intermediate layers.  Comfortably
/// clear of `VIRTUAL_LAYER_BASE` (30000, used by PDK-declared virtuals — see `pdk.rs`).
/// Only one `forbidden_unless_labeled` rule is expected per PDK today (IHP's Ant.h); if a
/// second one is ever added to the *same* run, give it its own offset here to avoid
/// colliding synthetic keys.
const SYN_BASE: u16 = 50000;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if rule.layers.len() < 10 {
        eprintln!(
            "[{}] forbidden_unless_labeled needs 10 layers [Activ, GatPoly, pSD, nSD.block, \
             Recog.diode, Recog.esd, NWell, PWell.block, nBuLay, TEXT]",
            rule.id
        );
        return vec![];
    }
    let pattern = rule.text.as_deref().unwrap_or("");
    println!("[{}] Checking candidate in forbidden region unless labelled '{pattern}'", rule.id);

    let key = |i: usize| (rule.layers[i].gds_layer as i16, rule.layers[i].gds_datatype as i16);
    let (activ, gatpoly, psd, nsd_block, recog_diode, recog_esd, nwell, pwell_block, nbulay, text) =
        (key(0), key(1), key(2), key(3), key(4), key(5), key(6), key(7), key(8), key(9));

    // Synthetic keys for the 15 intermediate/derived layers, in dependency order.
    let syn = |i: u16| (SYN_BASE as i16 + i as i16, 0i16);
    let nactiv_con = syn(0);
    let dn_diode = syn(1);
    let cand = syn(2);
    let well = syn(3);
    let nact_nwell = syn(4);
    let schottky = syn(5);
    let x1 = syn(6);
    let a1 = syn(7);
    let x2 = syn(8);
    let b1 = syn(9);
    let isolbox_1 = syn(10);
    let isolbox_2 = syn(11);
    let isolbox = syn(12);
    let kept = syn(13);
    let err = syn(14);

    merged.register_virtual(nactiv_con, VirtualOp::Difference, vec![activ, gatpoly, psd, nsd_block], None);
    merged.register_virtual(dn_diode, VirtualOp::Intersection, vec![nactiv_con, recog_diode], None);
    merged.register_virtual(cand, VirtualOp::Difference, vec![dn_diode, recog_esd], None);
    merged.register_virtual(well, VirtualOp::Union, vec![nwell, pwell_block], None);
    merged.register_virtual(nact_nwell, VirtualOp::Intersection, vec![nactiv_con, well], None);
    merged.register_virtual(schottky, VirtualOp::Intersection, vec![nbulay, recog_diode, nwell, nsd_block], None);
    merged.register_virtual(x1, VirtualOp::Interacting, vec![nbulay, recog_diode], None);
    merged.register_virtual(a1, VirtualOp::Interacting, vec![x1, nact_nwell], None);
    merged.register_virtual(x2, VirtualOp::Interacting, vec![recog_diode, nbulay], None);
    merged.register_virtual(b1, VirtualOp::Interacting, vec![x2, nact_nwell], None);
    merged.register_virtual(isolbox_1, VirtualOp::Union, vec![a1, b1], None);
    merged.register_virtual(isolbox_2, VirtualOp::NotInteracting, vec![isolbox_1, schottky], None);
    merged.register_virtual(isolbox, VirtualOp::WithText, vec![isolbox_2, text], Some(pattern.to_string()));
    merged.register_virtual(kept, VirtualOp::NotInteracting, vec![cand, isolbox], None);
    merged.register_virtual(err, VirtualOp::Intersection, vec![kept, nwell], None);

    // One violation per whole connected region (stitched across tile borders), matching
    // the original whole-chip-merge semantics — a candidate spanning several tiles must
    // not be reported once per tile.
    merged
        .regions(layout, err.0, err.1)
        .iter()
        .map(|r| {
            let (x, y) = (r.marker.0 * dbu_to_um, r.marker.1 * dbu_to_um);
            Violation::point(
                &rule.id,
                "Forbidden region (unlabelled)",
                format!("forbidden marker (not labelled '{pattern}') at ({x:.4}, {y:.4}) µm"),
                x, y,
            )
        })
        .collect()
}
