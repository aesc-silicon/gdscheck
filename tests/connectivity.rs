// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Net extraction (src/connectivity.rs).  The fixture has three poly gates: A and B are
//! joined by a shared Metal2 plate (each Metal1→Via1→Metal2), gate C is an isolated
//! Metal1 island.  So A and B must be one net, C a different one.

use gdscheck::connectivity::Connectivity;
use gdscheck::{flatten, load_gds, merge, pdk::PdkConfig};
use std::collections::HashMap;

#[test]
fn gates_bridged_by_metal2_share_a_net() {
    let pdk = PdkConfig::for_process("ihp-sg13g2").expect("load pdk");
    let lib =
        load_gds("tests/data/ihp-sg13g2/connectivity/two_gates.gds.gz").expect("load fixture");
    let dbu_to_um = lib.units.1 * 1e6;

    let mut layout = flatten::flatten_to_elems("TOP", &lib, None);
    pdk.compute_virtual_layers(&mut layout, dbu_to_um);

    let tile_dbu = (merge::TILE_UM / dbu_to_um).round() as i32;
    let halo_dbu = (merge::MIN_HALO_UM / dbu_to_um).ceil() as i32;
    let mut cache = merge::MergedCache::new(tile_dbu, halo_dbu, HashMap::new());

    let con = Connectivity::build(&mut cache, &layout, &pdk.connectivity);

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
