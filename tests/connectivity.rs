// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Net extraction (src/connectivity.rs).  The fixture has three poly gates: A and B are
//! joined by a shared Metal2 plate (each Metal1→Via1→Metal2), gate C is an isolated
//! Metal1 island.  So A and B must be one net, C a different one.

use gdscheck::connectivity::Connectivity;
use gdscheck::{flatten, load_gds, merge, parse_virtual_op, pdk::PdkConfig};
use std::collections::HashMap;

/// Flatten `fixture` under `process` and extract nets, mirroring `run_drc`'s setup:
/// eager virtuals materialised into the layout, lazy ones registered on the merge cache
/// (the NWell tie connector is one, so skipping that step would silently drop the well
/// from the connect graph).  Returns the connectivity and the layout's µm-per-DBU.
fn extract(process: &str, fixture: &str) -> (Connectivity, f64) {
    let pdk = PdkConfig::for_process(process).expect("load pdk");
    let lib = load_gds(fixture).expect("load fixture");
    let dbu_to_um = lib.units.1 * 1e6;

    let mut layout = flatten::flatten_to_elems("TOP", &lib, None);
    pdk.compute_virtual_layers(&mut layout, dbu_to_um);

    let tile_dbu = (merge::TILE_UM / dbu_to_um).round() as i32;
    let halo_dbu = (merge::MIN_HALO_UM / dbu_to_um).ceil() as i32;
    let mut cache = merge::MergedCache::new(tile_dbu, halo_dbu, HashMap::new());
    for spec in pdk.tiled_virtual_layers() {
        let op = parse_virtual_op(&spec.op, spec.radius, dbu_to_um).expect("virtual op");
        cache.register_virtual(spec.key, op, spec.sources, spec.text);
    }

    (Connectivity::build(&mut cache, &layout, &pdk.connectivity), dbu_to_um)
}

#[test]
fn gates_bridged_by_metal2_share_a_net() {
    let (con, dbu_to_um) =
        extract("ihp-sg13g2", "tests/data/ihp-sg13g2/connectivity/two_gates.gds.gz");

    // A point inside each gate's GatPoly (layer 5/0).  OFFSET = 20 µm; gates at x-origin
    // 20, 28, 44; each poly spans origin+0.5 .. origin+1.5 in x, ~19..25 in y.
    let gat = (5i16, 0i16);
    let at = |x: f64, y: f64| con.net_at(gat, x / dbu_to_um, y / dbu_to_um);
    let a = at(21.0, 22.0).expect("gate A on a net");
    let b = at(29.0, 22.0).expect("gate B on a net");
    let c = at(45.0, 22.0).expect("gate C on a net");

    assert_eq!(a, b, "gates A and B are bridged by Metal2 → same net");
    assert_ne!(a, c, "gate C is isolated → different net");
    assert!(con.net_count() >= 2);
}

/// The NWell tie connect step: `NActivInNWell` (the tap) shorts its well to the Activ
/// region it sits in, and Cont carries that on to Metal1.  On the NW.b1 same-net fixture
/// the right-hand pair is strapped well→tap→Cont→Metal1→Cont→tap→well and must resolve to
/// one net, while the bare left-hand pair must stay two.  This is what lets NW.b1 tell a
/// real different-net gap from a same-net one.
#[rstest::rstest]
#[case("ihp-sg13g2")]
#[case("ihp-sg13cmos5l")]
fn nwell_tie_shorts_strapped_wells(#[case] process: &str) {
    let (con, dbu_to_um) =
        extract(process, "tests/data/ihp-sg13g2/nwell/NW.b1.same_net.gds.gz");

    // OFFSET = 20 µm.  Bare pair of 1×1 µm wells at x 20..21, tied pair at x 24..25;
    // in each pair the wells sit at y 20..21 and y 22..23 (a 1.00 µm gap).  The tied
    // pair's Metal1 strap runs x 24.36..24.64 across the gap.
    let nwell = (31i16, 0i16);
    let metal1 = (8i16, 0i16);
    let at = |l, x: f64, y: f64| con.net_at(l, x / dbu_to_um, y / dbu_to_um);

    let bare_lo = at(nwell, 20.5, 20.5).expect("bare lower well on a net");
    let bare_hi = at(nwell, 20.5, 22.5).expect("bare upper well on a net");
    let tied_lo = at(nwell, 24.5, 20.5).expect("tied lower well on a net");
    let tied_hi = at(nwell, 24.5, 22.5).expect("tied upper well on a net");
    let strap = at(metal1, 24.5, 21.5).expect("strap on a net");

    assert_eq!(tied_lo, tied_hi, "wells strapped through their taps → one net");
    assert_eq!(tied_lo, strap, "the well net is the strap's net, i.e. tied via the tap");
    assert_ne!(bare_lo, bare_hi, "untied wells → separate nets");
    assert_ne!(bare_lo, tied_lo);
    assert_ne!(bare_hi, tied_lo);
}
