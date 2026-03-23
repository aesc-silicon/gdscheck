// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PDK-agnostic tests for the gdscheck engine itself: hierarchy flattening, deck/
//! suite resolution, and virtual-layer operators.  These assert engine behaviour
//! independent of any foundry, so they use a tiny synthetic PDK (tests/data/
//! synthetic) or programmatically-built geometry rather than the IHP fixtures.

use gds21::{GdsBoundary, GdsElement, GdsLibrary, GdsPoint, GdsStrans, GdsStruct};
use gds21::{GdsArrayRef, GdsStructRef};
use gdscheck::flatten::flatten_to_elems;
use gdscheck::layout::FlatLayout;
use gdscheck::merge::{difference_layers, merged_area_dbu, MergedCache, VirtualOp};
use gdscheck::pdk::PdkConfig;
use gdscheck::run_drc;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Hierarchy flattening (src/flatten.rs)
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box of a boundary, in DBU.
fn bbox(b: &GdsBoundary) -> (i32, i32, i32, i32) {
    let xs = b.xy.iter().map(|p| p.x);
    let ys = b.xy.iter().map(|p| p.y);
    (
        xs.clone().min().unwrap(),
        ys.clone().min().unwrap(),
        xs.max().unwrap(),
        ys.max().unwrap(),
    )
}

/// Build a library whose `CHILD` cell holds a single 100×20 rect on layer 10/0,
/// instanced into `TOP` three ways: a plain placement, a 90° rotation, and a 3×2
/// array.
fn hierarchy_lib() -> GdsLibrary {
    let mut child = GdsStruct::new("CHILD");
    child.elems.push(GdsElement::GdsBoundary(GdsBoundary {
        layer: 10,
        datatype: 0,
        xy: GdsPoint::vec(&[(0, 0), (100, 0), (100, 20), (0, 20), (0, 0)]),
        ..Default::default()
    }));

    let mut top = GdsStruct::new("TOP");
    // 1. Plain placement, translated to (1000, 0).
    top.elems.push(GdsElement::GdsStructRef(GdsStructRef {
        name: "CHILD".into(),
        xy: GdsPoint::new(1000, 0),
        ..Default::default()
    }));
    // 2. Rotated 90° CCW, then translated to (0, 1000): width/height swap.
    top.elems.push(GdsElement::GdsStructRef(GdsStructRef {
        name: "CHILD".into(),
        xy: GdsPoint::new(0, 1000),
        strans: Some(GdsStrans {
            reflected: false,
            abs_mag: false,
            abs_angle: false,
            mag: None,
            angle: Some(90.0),
        }),
        ..Default::default()
    }));
    // 3. 3 cols × 2 rows array: 200 DBU column pitch, 50 DBU row pitch.
    top.elems.push(GdsElement::GdsArrayRef(GdsArrayRef {
        name: "CHILD".into(),
        xy: [GdsPoint::new(0, 0), GdsPoint::new(600, 0), GdsPoint::new(0, 100)],
        cols: 3,
        rows: 2,
        ..Default::default()
    }));

    let mut lib = GdsLibrary::new("HIER");
    lib.structs = vec![child, top];
    lib
}

#[test]
fn flatten_resolves_refs_arrays_and_rotation() {
    let lib = hierarchy_lib();
    let layout = flatten_to_elems("TOP", &lib, None);

    let boxes: Vec<(i32, i32, i32, i32)> = layout.get(10, 0).iter().map(bbox).collect();

    // 1 plain + 1 rotated + (3×2) array = 8 instances of the child rect.
    assert_eq!(boxes.len(), 8, "expected 8 flattened shapes, got {boxes:?}");

    // Plain placement: child translated by (1000, 0).
    assert!(boxes.contains(&(1000, 0, 1100, 20)), "missing plain ref: {boxes:?}");

    // 90° rotation swaps the 100×20 footprint to 20×100, here at (0,1000).
    assert!(boxes.contains(&(-20, 1000, 0, 1100)), "missing rotated ref: {boxes:?}");

    // Array corner instance (col 2, row 1): origin + (2·200, 1·50) = (400, 50).
    assert!(boxes.contains(&(400, 50, 500, 70)), "missing array instance: {boxes:?}");
}

// ---------------------------------------------------------------------------
// Deck / suite resolution (src/pdk.rs: load_deck / load_suite)
// ---------------------------------------------------------------------------

const SYNTH: &str = "tests/data/synthetic/pdk.yml";

/// Sorted rule ids of the deck `name` in the synthetic PDK.
fn deck_ids(name: &str) -> Vec<String> {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    let mut ids: Vec<String> = pdk
        .load_deck(name)
        .unwrap_or_else(|e| panic!("load_deck({name}) failed: {e}"))
        .into_iter()
        .map(|r| r.id)
        .collect();
    ids.sort();
    ids
}

/// Sorted rule ids of the suite `name` in the synthetic PDK.
fn suite_ids(name: &str) -> Vec<String> {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    let mut ids: Vec<String> = pdk
        .load_suite(name)
        .unwrap_or_else(|e| panic!("load_suite({name}) failed: {e}"))
        .into_iter()
        .map(|r| r.id)
        .collect();
    ids.sort();
    ids
}

#[test]
fn resolves_plain_deck() {
    // deckA has A.1 plus two A.2 entries (min_space + min_notch).
    assert_eq!(deck_ids("deckA"), ["A.1", "A.2", "A.2"]);
}

#[test]
fn suite_imports_whole_decks() {
    assert_eq!(suite_ids("full"), ["A.1", "A.2", "A.2", "B.1"]);
}

#[test]
fn suite_whitelist_keeps_all_entries_of_an_id() {
    // `subset` whitelists only A.2 — both A.2 entries must come through, no A.1.
    assert_eq!(suite_ids("subset"), ["A.2", "A.2"]);
}

#[test]
fn unknown_deck_is_an_error() {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    assert!(pdk.load_deck("does-not-exist").is_err());
    // A suite name is not a deck, and vice versa — the two namespaces are separate.
    assert!(pdk.load_deck("full").is_err());
    assert!(pdk.load_suite("deckA").is_err());
}

#[test]
fn suite_with_unknown_rule_id_errors() {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    let err = pdk.load_suite("badrule").expect_err("unknown rule id should error");
    assert!(
        err.to_string().contains("A.9"),
        "error should name the offending id: {err}"
    );
}

#[test]
fn suite_with_unknown_deck_errors() {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    assert!(pdk.load_suite("baddeck").is_err());
}

// ---------------------------------------------------------------------------
// Virtual-layer operators (src/pdk.rs: compute_virtual_layers)
//
// Uses the IHP PDK's Pad/SBumpPad/CuPillarPad definitions to exercise the engine's
// union and intersection operators.
// ---------------------------------------------------------------------------

/// A 10×10 marker rectangle at `x`, tagging which source layer it came from once
/// copied into a virtual layer.
fn marker(layer: i16, datatype: i16, x: i32) -> GdsBoundary {
    GdsBoundary {
        layer,
        datatype,
        xy: GdsPoint::vec(&[(x, 0), (x + 10, 0), (x + 10, 10), (x, 10), (x, 0)]),
        ..Default::default()
    }
}

/// Verifies the `virtual_layers` operators in the IHP PDK:
///   * `Pad` is a **union** of its sources.
///   * `SBumpPad` / `CuPillarPad` are device-recognition **intersections**
///     (`Passiv.<type>` AND `dfpad`) — a `dfpad` with no matching passivation
///     marker must NOT be recognised as a pad.
#[test]
fn virtual_layers_ops() {
    let pdk = PdkConfig::for_process("ihp-sg13g2").expect("load pdk");
    let put = |layout: &mut FlatLayout, name: &str, x: i32| {
        let l = pdk.layer(name).unwrap_or_else(|| panic!("missing layer {name}"));
        let (gl, gd) = (l.gds_layer as i16, l.gds_datatype as i16);
        layout.insert(gl, gd, marker(gl, gd, x));
    };

    let mut layout = FlatLayout::new();
    // dfpad at three places: with pillar, with sbump, and generic (no marker).
    put(&mut layout, "dfpad", 0);
    put(&mut layout, "dfpad", 100);
    put(&mut layout, "dfpad", 200);
    put(&mut layout, "Passiv.pillar", 0); // coincides with dfpad@0  -> CuPillarPad
    put(&mut layout, "Passiv.sbump", 100); // coincides with dfpad@100 -> SBumpPad
    put(&mut layout, "Passiv", 300); // for the Pad union only

    pdk.compute_virtual_layers(&mut layout, 0.001);

    // Sorted min-x of each shape on a (virtual) layer.
    let minxs = |name: &str| -> Vec<i32> {
        let l = pdk.layer(name).unwrap();
        let mut v: Vec<i32> = layout
            .get(l.gds_layer as i16, l.gds_datatype as i16)
            .iter()
            .map(|b| b.xy.iter().map(|p| p.x).min().unwrap())
            .collect();
        v.sort_unstable();
        v
    };

    // Pad = union of dfpad(0,100,200) and Passiv(300).
    assert_eq!(minxs("Pad"), vec![0, 100, 200, 300]);
    // CuPillarPad = Passiv.pillar AND dfpad -> only at x=0.
    assert_eq!(minxs("CuPillarPad"), vec![0]);
    // SBumpPad = Passiv.sbump AND dfpad -> only at x=100.
    assert_eq!(minxs("SBumpPad"), vec![100]);
}

// ---------------------------------------------------------------------------
// Virtual-layer boolean ops: lazy (tiled, src/merge.rs) and eager
// (src/pdk.rs::compute_virtual_layers), plus the lazy + inside_boundary guard.
// ---------------------------------------------------------------------------

/// An axis-aligned rectangle as a `GdsBoundary`, in DBU.
fn dbu_rect(layer: i16, datatype: i16, x0: i32, y0: i32, x1: i32, y1: i32) -> GdsBoundary {
    GdsBoundary {
        layer,
        datatype,
        xy: GdsPoint::vec(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]),
        ..Default::default()
    }
}

// A = (0,0)-(100k,100k); B = (50k,50k)-(150k,150k); they overlap in a 50k² square.
// Areas are exact for axis-aligned integer rectangles.
const A_AREA: f64 = 100_000.0 * 100_000.0; // 1e10
const OVERLAP: f64 = 50_000.0 * 50_000.0; // 2.5e9

fn layout_a_b() -> FlatLayout {
    let mut layout = FlatLayout::new();
    layout.insert(1, 0, dbu_rect(1, 0, 0, 0, 100_000, 100_000));
    layout.insert(2, 0, dbu_rect(2, 0, 50_000, 50_000, 150_000, 150_000));
    layout
}

/// #1 — the lazy/tiled compose path (`build_virtual_tiles`/`compose_tile`) for all
/// three ops, built on demand in the merge cache from the source layers' tiles.
#[test]
fn lazy_tiled_union_intersection_difference() {
    let layout = layout_a_b();
    // One giant tile and zero halo so per-tile areas don't double-count across halos.
    let mut cache = MergedCache::new(10_000_000, 0, HashMap::new());
    cache.register_virtual((30000, 0), VirtualOp::Union, vec![(1, 0), (2, 0)], None);
    cache.register_virtual((30001, 0), VirtualOp::Intersection, vec![(1, 0), (2, 0)], None);
    cache.register_virtual((30002, 0), VirtualOp::Difference, vec![(1, 0), (2, 0)], None);

    cache.ensure(&layout, 30000, 0);
    cache.ensure(&layout, 30001, 0);
    cache.ensure(&layout, 30002, 0);

    let area = |c: &MergedCache, g: i16, d: i16| -> f64 {
        c.tiles(g, d).values().flatten().map(merged_area_dbu).sum()
    };
    assert_eq!(area(&cache, 30000, 0), 2.0 * A_AREA - OVERLAP); // union
    assert_eq!(area(&cache, 30001, 0), OVERLAP); // intersection
    assert_eq!(area(&cache, 30002, 0), A_AREA - OVERLAP); // difference (A − B)
}

/// #3 — the eager (`mode: global`) difference arm in `compute_virtual_layers`, which
/// materialises `GlobalDiff = LayerA NOT LayerB` into the layout; the lazy `LazyDiff`
/// must NOT be materialised.
#[test]
fn eager_global_difference_materialises_lazy_does_not() {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    let mut layout = layout_a_b();
    pdk.compute_virtual_layers(&mut layout, 0.001);

    let gd = pdk.layer("GlobalDiff").expect("GlobalDiff registered");
    let shapes = layout.get(gd.gds_layer as i16, gd.gds_datatype as i16);
    let area: f64 = difference_layers(shapes, &[]).iter().map(merged_area_dbu).sum();
    assert_eq!(area, A_AREA - OVERLAP, "GlobalDiff should be LayerA minus LayerB");

    let ld = pdk.layer("LazyDiff").expect("LazyDiff registered");
    assert!(
        layout.get(ld.gds_layer as i16, ld.gds_datatype as i16).is_empty(),
        "lazy layer must not be materialised into the layout"
    );
}

/// The `inside` op fills a ring (drops its hole) and intersects the target with it,
/// so a target shape inside the ring is kept and one outside is dropped — there is no
/// drawn layer for "the area a seal ring encloses", so the op derives it.
#[test]
fn inside_op_fills_ring_and_keeps_only_enclosed() {
    let pdk = PdkConfig::load(SYNTH).expect("load synthetic pdk");
    let mut layout = FlatLayout::new();
    // LayerB (2/0) drawn as a ring frame; filled it is the solid 0..100 square.
    layout.insert(2, 0, dbu_rect(2, 0, 0, 0, 100, 20));
    layout.insert(2, 0, dbu_rect(2, 0, 0, 80, 100, 100));
    layout.insert(2, 0, dbu_rect(2, 0, 0, 0, 20, 100));
    layout.insert(2, 0, dbu_rect(2, 0, 80, 0, 100, 100));
    // LayerA (1/0): one shape inside the ring, one outside it.
    layout.insert(1, 0, dbu_rect(1, 0, 40, 40, 50, 50)); // inside  -> kept
    layout.insert(1, 0, dbu_rect(1, 0, 110, 40, 120, 50)); // outside -> dropped

    pdk.compute_virtual_layers(&mut layout, 0.001);

    let r = pdk.layer("InsideRing").expect("InsideRing registered");
    let shapes = layout.get(r.gds_layer as i16, r.gds_datatype as i16);
    let area: f64 = difference_layers(shapes, &[]).iter().map(merged_area_dbu).sum();
    assert_eq!(area, 10.0 * 10.0, "only the 10×10 shape inside the ring should remain");
}

/// #2 — a lazy virtual layer under a whole-layout check (inside_boundary) is rejected
/// up front (before any GDS is read).
#[test]
fn lazy_layer_in_inside_boundary_is_rejected() {
    let err = run_drc("unused.gds", SYNTH, &["badlazy"], None, "TOP", true)
        .err()
        .expect("lazy layer under inside_boundary must error");
    assert!(err.contains("lazy virtual layer"), "unexpected error: {err}");
}
