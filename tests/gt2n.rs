// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GT2N tests.
//!
//! GT2N extends GT3, and its runset is GT3's with seven values changed and a deck for
//! the threshold-voltage layers.
//!
//! Every rule has a generated pattern (`gen/gt`, shared with GT3) under
//! `tests/data/gt2n/generated/<deck>/`, drawn to the runset's values, and above M6,
//! where the runset stops, to the tech LEF's, plus the technology file's well and
//! area constraints.

use gds21::{GdsBoundary, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsUnits};
use gdscheck::run_drc;
use gdscheck::{connectivity::Connectivity, flatten, merge, parse_virtual_op, pdk::PdkConfig};
use rstest::rstest;
use std::collections::{BTreeMap, BTreeSet, HashMap};

// Physical GDS layer numbers from GT2N's gt2_main.drc.rs. Drawing the source numbers
// independently of PdkConfig also catches a wrong layer mapping in the port.
const BPR: i16 = 8;
const VBPR: i16 = 9;
const SDCON: i16 = 10;
const BACK_METALS: [i16; 4] = [920, 940, 960, 980];
const BACK_VIAS: [i16; 5] = [910, 930, 950, 970, 990];
const BRDL: i16 = 1000;

fn rect(layer: i16, [x0, y0, x1, y1]: [i32; 4]) -> GdsElement {
    GdsElement::GdsBoundary(GdsBoundary {
        layer,
        datatype: 0,
        xy: [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]
            .map(|(x, y)| GdsPoint::new(x * 2, y * 2))
            .to_vec(),
        ..Default::default()
    })
}

fn library(elems: Vec<GdsElement>) -> GdsLibrary {
    let mut top = GdsStruct::new("TOP");
    top.elems = elems;
    let mut lib = GdsLibrary::new("BACKSIDE");
    lib.units = GdsUnits(0.0005, 0.5e-9);
    lib.structs = vec![top];
    lib
}

fn extract(lib: &GdsLibrary, tile_um: f64) -> Connectivity {
    let pdk = PdkConfig::for_process("gt2n").unwrap();
    let dbu_to_um = lib.units.1 * 1e6;
    let mut layout = flatten::flatten_to_elems("TOP", lib, None, &[]);
    let eager = pdk.eager_layers(&[]).unwrap();
    pdk.compute_virtual_layers(&mut layout, dbu_to_um, &eager);
    let mut cache = merge::MergedCache::new(
        (tile_um / dbu_to_um).round() as i32,
        (merge::MIN_HALO_UM / dbu_to_um).ceil() as i32,
        HashMap::new(),
    );
    for spec in pdk.tiled_virtual_layers(&eager) {
        let op = parse_virtual_op(
            &spec.op,
            spec.radius,
            spec.min,
            spec.max,
            spec.slack,
            dbu_to_um,
        )
        .unwrap();
        cache.register_virtual(spec.key, op, spec.sources, spec.text);
    }
    Connectivity::build(&mut cache, &layout, &pdk.connectivity, dbu_to_um, &[])
}

fn net(conn: &Connectivity, layer: i16, x_nm: i32, y_nm: i32) -> usize {
    conn.net_at((layer, 0), f64::from(x_nm * 2), f64::from(y_nm * 2))
        .expect("conductor must have a net even when disconnected")
}

/// Two 16 nm SDCONs, 19 nm apart: SDCON.2 (20 nm, different nets) must report
/// unless their two otherwise disjoint stacks meet at BRDL. Each via is wholly
/// inside its landing pads. These deliberately small pads exercise connectivity,
/// not LEF backside geometry rules, which are not implemented by the ICV deck.
fn backside_pair(origin: i32, missing: Option<(i16, usize)>, join_rdl: bool) -> GdsLibrary {
    let mut elems = Vec::new();
    for arm in 0..2 {
        let y = origin + arm as i32 * 35;
        for layer in [SDCON, BPR].into_iter().chain(BACK_METALS) {
            elems.push(rect(layer, [origin, y, origin + 16, y + 16]));
        }
        for layer in [VBPR].into_iter().chain(BACK_VIAS) {
            if missing != Some((layer, arm)) {
                elems.push(rect(layer, [origin + 2, y + 2, origin + 14, y + 14]));
            }
        }
        if !join_rdl {
            elems.push(rect(BRDL, [origin, y, origin + 16, y + 16]));
        }
    }
    if join_rdl {
        elems.push(rect(BRDL, [origin, origin, origin + 16, origin + 51]));
    }
    library(elems)
}

#[rstest]
#[case::intact(None)]
#[case::vbpr(Some(VBPR))]
#[case::bv0(Some(910))]
#[case::bv1(Some(930))]
#[case::bv2(Some(950))]
#[case::bv3(Some(970))]
#[case::bv4(Some(990))]
fn backside_stack_controls_sdcon_different_net_spacing(#[case] missing: Option<i16>) {
    for (origin, tile_um) in [(100, 20.0), (6_990, 7.0), (19_990, 20.0)] {
        for arm in 0..2 {
            let lib = backside_pair(origin, missing.map(|layer| (layer, arm)), true);
            let conn = extract(&lib, tile_um);
            let a = net(&conn, SDCON, origin + 8, origin + 8);
            let b = net(&conn, SDCON, origin + 8, origin + 43);
            assert_eq!(
                a == b,
                missing.is_none(),
                "missing {missing:?} on arm {arm}"
            );
            let violations = gdscheck::run_drc_with_options(
                &lib,
                "gt2n",
                &["sdcon"],
                None,
                "TOP",
                true,
                &gdscheck::RunOptions {
                    tile_um,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                violations.len(),
                usize::from(missing.is_some()),
                "{violations:?}"
            );
            assert!(violations.iter().all(|v| v.rule_id == "SDCON.2"));
        }
    }
}

#[test]
fn separate_backside_power_rails_stay_separate() {
    let lib = backside_pair(100, None, false);
    let conn = extract(&lib, 20.0);
    let supply_a = net(&conn, SDCON, 108, 108);
    let supply_b = net(&conn, SDCON, 108, 143);
    assert_ne!(supply_a, supply_b);
    for layer in [BPR].into_iter().chain(BACK_METALS).chain([BRDL]) {
        assert_eq!(net(&conn, layer, 108, 108), supply_a);
        assert_eq!(net(&conn, layer, 108, 143), supply_b);
    }
    let v = gdscheck::run_drc_with(&lib, "gt2n", &["sdcon"], None, "TOP", true).unwrap();
    assert_eq!(
        v.iter().map(|v| v.rule_id.as_str()).collect::<Vec<_>>(),
        ["SDCON.2"]
    );
}

#[rstest]
#[case::no_vias(0)]
#[case::one_via(1)]
#[case::both_vias(2)]
fn crossing_backside_metals_only_connect_where_a_via_is_present(#[case] vias: usize) {
    // Two parallel BM1 supply rails crossed by one BM2 strap. A via at the first
    // crossing must not also attach the other rail just because BM2 overlaps it.
    let mut elems = vec![
        rect(920, [100, 100, 116, 160]),
        rect(920, [135, 100, 151, 160]),
        rect(940, [100, 120, 151, 136]),
    ];
    for i in 0..vias {
        let x = 100 + i as i32 * 35;
        elems.push(rect(930, [x + 2, 122, x + 14, 134]));
    }
    let conn = extract(&library(elems), 20.0);
    let a = net(&conn, 920, 108, 128);
    let b = net(&conn, 920, 143, 128);
    let strap = net(&conn, 940, 125, 128);
    assert_eq!(a == strap, vias >= 1);
    assert_eq!(b == strap, vias == 2);
    assert_eq!(a == b, vias == 2);
}

#[test]
fn the_feol_beol_and_backside_suites_split_main() {
    let pdk = PdkConfig::for_process("gt2n").expect("PDK loads");
    let ids = |suite: &str| -> BTreeSet<String> {
        pdk.load_suite(suite)
            .expect("suite loads")
            .iter()
            .map(|r| {
                let layers: Vec<&str> = r.layers.iter().map(|l| l.name.as_str()).collect();
                format!("{} {} {:?} {}", r.id, r.check, layers, r.value)
            })
            .collect()
    };
    let (main, feol, beol, back) = (ids("main"), ids("feol"), ids("beol"), ids("backside"));
    assert!(feol.is_disjoint(&beol), "a rule in both feol and beol");
    assert!(feol.is_disjoint(&back), "a rule in both feol and backside");
    assert!(beol.is_disjoint(&back), "a rule in both beol and backside");
    let all: BTreeSet<String> = feol.union(&beol).chain(&back).cloned().collect();
    assert_eq!(main, all);
}

#[rstest]
#[case("main", vec!["ACT.area.TF", "BM1.area.TF", "BPR.area.TF", "M10.area.TF", "NWELL.area.TF"])]
#[case("feol", vec!["ACT.area.TF", "BPR.area.TF", "NWELL.area.TF"])]
#[case("beol", vec!["M10.area.TF"])]
#[case("backside", vec!["BM1.area.TF"])]
fn technology_file_constraints_run_in_each_suite(#[case] suite: &str, #[case] rules: Vec<&str>) {
    // Independent source layer numbers. The well and metals meet their widths but
    // lack area; BPR's 3 × 32 nm is the last grid step below 100 nm². The ACT piece
    // also fails its ICV length, so isolate the new technology-file findings.
    let lib = library(vec![
        rect(1, [0, 0, 42, 42]),
        rect(2, [0, 0, 40, 8]),
        rect(8, [0, 0, 3, 32]),
        rect(70, [0, 0, 56, 56]),
        rect(920, [0, 0, 56, 56]),
    ]);
    let violations = gdscheck::run_drc_with(&lib, "gt2n", &[], Some(suite), "TOP", true)
        .expect("technology-file suite run");
    let mut got: Vec<_> = violations
        .iter()
        .filter(|v| v.rule_id.ends_with(".TF"))
        .map(|v| v.rule_id.as_str())
        .collect();
    got.sort_unstable();
    assert_eq!(got, rules);
}

// --- Generated patterns ---
// Each fixture leads with the shapes exactly at the limit, then the ones a DBU past
// it; the counts follow the drawings and the runset. Each case runs its whole deck.

/// Every rule in every deck has a generated fixture, and every fixture names a rule
/// its deck still has. A fixture is `<rule>.<case>.<polarity>.gds.gz`.
#[test]
fn every_rule_has_a_generated_fixture() {
    let pdk = PdkConfig::for_process("gt2n").unwrap();
    for deck in &pdk.decks {
        let ids: BTreeSet<String> = pdk
            .load_deck(&deck.name)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        let dir = format!("tests/data/gt2n/generated/{}", deck.name);
        let mut drawn = BTreeSet::new();
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{dir}: {e}")) {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let stem = name
                .strip_suffix(".gds.gz")
                .unwrap_or_else(|| panic!("{dir}/{name} is not a fixture"));
            let rule = stem.rsplitn(3, '.').last().unwrap();
            assert!(
                ids.contains(rule),
                "{dir}/{name}: {} has no rule {rule}",
                deck.name
            );
            drawn.insert(rule.to_string());
        }
        let missing: Vec<&String> = ids.difference(&drawn).collect();
        assert!(
            missing.is_empty(),
            "{}: no fixture for {missing:?}",
            deck.name
        );
    }
}

/// The fixture `<deck>/<rule>.<variant>` reports `count` of `rule` and nothing else
/// its deck checks, but for the rules in `ignore`.
fn pattern(deck: &str, rule: &str, variant: &str, count: usize, ignore: &[&str]) {
    let polarity = if count == 0 { "good" } else { "bad" };
    let path = format!("tests/data/gt2n/generated/{deck}/{rule}.{variant}.{polarity}.gds.gz");
    let mut got = BTreeMap::new();
    for v in run_drc(&path, "gt2n", &[deck], None, "TOP", true).expect("DRC run failed") {
        if !ignore.contains(&v.rule_id.as_str()) {
            *got.entry(v.rule_id).or_insert(0usize) += 1;
        }
    }
    let expected = if count == 0 {
        BTreeMap::new()
    } else {
        BTreeMap::from([(rule.to_string(), count)])
    };
    assert_eq!(got, expected, "{path}");
}

#[rstest]
#[case("NWELL.width.TF", "width", 0)]
// The width check marks both facing walls of each of the two narrow wells.
#[case("NWELL.width.TF", "width", 4)]
#[case("NWELL.space.TF", "space", 0)]
#[case("NWELL.space.TF", "space", 3)]
#[case("NWELL.space.TF", "notch", 0)]
#[case("NWELL.space.TF", "notch", 2)]
#[case("RDL.area.TF", "hole", 0)]
#[case("RDL.area.TF", "hole", 1)]
#[case("BRDL.area.TF", "hole", 0)]
#[case("BRDL.area.TF", "hole", 1)]
fn technology_file_geometry(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    pattern("techfile", rule, variant, count, &[]);
}

#[rstest]
#[case("area", 0)]
#[case("area", 3)]
#[case("merged", 0)]
#[case("separate", 2)]
fn technology_file_minimum_area(
    #[values(
        "NWELL", "ACT", "BPR", "M10", "M11", "M12", "M13", "RDL", "BM1", "BM2", "BM3", "BM4",
        "BRDL"
    )]
    layer: &str,
    #[case] variant: &str,
    #[case] count: usize,
) {
    pattern("techfile", &format!("{layer}.area.TF"), variant, count, &[]);
}

/// All 56 drawing layers: negative half-nm coordinates are clean; a 0.25 nm shift
/// in x alone, y alone or both gives four markers per rectangle. These fixtures use
/// a 0.25 nm DBU, unlike the usual 0.5 nm GT2N fixtures, so the error is representable.
#[rstest]
#[case("FEOL.offgrid.LEF", 0)]
#[case("FEOL.offgrid.LEF", 228)]
#[case("BEOL.offgrid.LEF", 0)]
#[case("BEOL.offgrid.LEF", 324)]
#[case("BACKSIDE.offgrid.LEF", 0)]
#[case("BACKSIDE.offgrid.LEF", 120)]
fn manufacturing_grid_patterns(#[case] rule: &str, #[case] count: usize) {
    pattern("offgrid", rule, "grid", count, &[]);
}

/// The original missed M1 is a legal 14 x 28 nm rectangle translated by 0.25 nm.
/// The same half-grid shift on NWELL and BV0 exercises the other suite boundaries;
/// a pin annotation is not manufacturing geometry. Only grid findings are counted
/// here, since the isolated via also lacks its landing metals.
#[rstest]
#[case("main", vec!["BACKSIDE.offgrid.LEF", "BEOL.offgrid.LEF", "FEOL.offgrid.LEF"])]
#[case("feol", vec!["FEOL.offgrid.LEF"])]
#[case("beol", vec!["BEOL.offgrid.LEF"])]
#[case("backside", vec!["BACKSIDE.offgrid.LEF"])]
fn manufacturing_grid_runs_in_each_suite(#[case] suite: &str, #[case] rules: Vec<&str>) {
    let mut elems = Vec::new();
    for (layer, datatype) in [(1, 0), (25, 0), (910, 0), (25, 251)] {
        elems.push(GdsElement::GdsBoundary(GdsBoundary {
            layer,
            datatype,
            xy: [(1, 1), (57, 1), (57, 113), (1, 113), (1, 1)]
                .map(|(x, y)| GdsPoint::new(x, y))
                .to_vec(),
            ..Default::default()
        }));
    }
    let mut lib = library(elems);
    lib.units = GdsUnits(0.00025, 0.25e-9);
    let violations = gdscheck::run_drc_with(&lib, "gt2n", &[], Some(suite), "TOP", true)
        .expect("DRC run failed");
    let mut got = BTreeMap::new();
    for v in violations
        .iter()
        .filter(|v| v.rule_id.ends_with(".offgrid.LEF"))
    {
        *got.entry(v.rule_id.as_str()).or_insert(0usize) += 1;
    }
    let expected: BTreeMap<_, _> = rules.into_iter().map(|rule| (rule, 4)).collect();
    assert_eq!(got, expected);
}

/// The front end. An exact width a DBU under and a DBU over gives two walls each, a
/// minimum width one DBU under gives two; a pair too close, a slot too narrow, a
/// shape that bends give one each.
///
/// GATE.2 reads the 42 nm pitch as well as the 28 nm space: a gate a DBU or half a pitch
/// off it gives one marker each, and the gate a DBU too close, or the U a DBU too
/// narrow inside, is off the pitch as well - two each. A gate a DBU narrow, wide or
/// jogged is off the pitch too, so the GATE.1 and GATE.4 tests ignore GATE.2. It reads
/// a gate missing from a row as a space too wide: one missing, three missing, and two
/// gates 994 nm apart, within the 1 µm the deck closes over, give one marker each; two
/// gates 25 pitches apart, beyond it, and two in different rows give none.
///
/// A notch needs a shape that bends, and its joint is no exact width, so the notch
/// tests ignore the shape's bend and exact-width rules. A DUMMY abutting a gate and
/// one alone lie inside none, which DUMMY.4 also reads.
///
/// A VBPR, GCUT, DUMMY or VT layer abutting another shares no area with it, which
/// the runset's `include_touch = NONE` does not count; a GCUT abutting ACT is counted,
/// under `include_touch = ALL`. A 31 nm-sheet filler between 13 nm-sheet cells stands
/// proud of them by two pieces one pitch across, two walls each under ACT.2; fillers
/// matching their neighbours, and a 13 nm filler between 31 nm cells, read clean.
/// ACT.3 reads two actives corner to corner as it does two facing across y.
/// ACT.6 reads an end a DBU clear of its DUMMY and one
/// the DUMMY covers half way, stricter there than upstream. GCUT.7 reads a cut's wall inside a gate, flush with
/// its wall on either side, but not inside a gate under a DUMMY: three.
#[rstest]
#[case("bpr", "BPR.1", "narrow", 4)]
#[case("bpr", "BPR.2", "close", 1)]
#[case("bpr", "BPR.2", "notch", 1)]
#[case("bpr", "BPR.3", "broken", 1)]
#[case("bpr", "BPR.3", "notch", 1)]
#[case("bpr", "BPR.4", "bent", 1)]
#[case("vbpr", "VBPR.1", "narrow", 4)]
#[case("vbpr", "VBPR.2", "narrow", 4)]
#[case("vbpr", "VBPR.3", "gcut", 1)]
#[case("vbpr", "VBPR.4", "outside", 2)]
#[case("gate", "GATE.1", "narrow", 4)]
#[case("gate", "GATE.2", "pitch", 2)]
#[case("gate", "GATE.2", "missing", 3)]
#[case("gate", "GATE.2", "close", 2)]
#[case("gate", "GATE.2", "notch", 2)]
#[case("gate", "GATE.4", "bent", 1)]
#[case("dummy", "DUMMY.1", "narrow", 4)]
#[case("dummy", "DUMMY.2", "alone", 2)]
#[case("dummy", "DUMMY.3", "bent", 1)]
#[case("dummy", "DUMMY.4", "outside", 1)]
#[case("gcut", "GCUT.1", "narrow", 2)]
#[case("gcut", "GCUT.2", "narrow", 2)]
#[case("gcut", "GCUT.5", "bent", 1)]
#[case("gcut", "GCUT.6", "alone", 2)]
#[case("gcut", "GCUT.7", "edge", 3)]
#[case("gcut", "GCUT.8", "act", 2)]
#[case("gcut", "GCUT.ACT.1", "close", 1)]
#[case("act", "ACT.1", "narrow", 2)]
#[case("act", "ACT.2", "narrow", 2)]
#[case("act", "ACT.2", "sheets", 4)]
#[case("act", "ACT.3", "close", 1)]
#[case("act", "ACT.3", "notch", 1)]
#[case("act", "ACT.3", "corner", 1)]
#[case("act", "ACT.6", "end", 2)]
#[case("act", "ACT.BPR.1", "close", 1)]
#[case("npsel", "NSEL.1", "narrow", 2)]
#[case("npsel", "NSEL.2", "narrow", 2)]
#[case("npsel", "NSEL.3", "enc", 2)]
#[case("npsel", "PSEL.1", "narrow", 2)]
#[case("npsel", "PSEL.2", "narrow", 2)]
#[case("npsel", "PSEL.3", "enc", 2)]
#[case("npsel", "NSEL.PSEL.1", "overlap", 1)]
#[case("vt", "ELVT.1", "narrow", 2)]
#[case("vt", "ELVT.2", "narrow", 2)]
#[case("vt", "ELVT.3", "enc", 2)]
#[case("vt", "ULVT.1", "narrow", 2)]
#[case("vt", "ULVT.2", "narrow", 2)]
#[case("vt", "ULVT.3", "enc", 2)]
#[case("vt", "SVT.1", "narrow", 2)]
#[case("vt", "SVT.2", "narrow", 2)]
#[case("vt", "SVT.3", "enc", 2)]
#[case("vt", "HVT.1", "narrow", 2)]
#[case("vt", "HVT.2", "narrow", 2)]
#[case("vt", "HVT.3", "enc", 2)]
#[case("vt", "SRAMVT.1", "narrow", 2)]
#[case("vt", "SRAMVT.2", "narrow", 2)]
#[case("vt", "SRAMVT.3", "enc", 2)]
#[case("vt", "VT_OVERLAP.1", "overlap", 10)]
fn feol(#[case] deck: &str, #[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore: &[&str] = match (rule, variant) {
        ("BPR.2" | "BPR.3", "notch") => &["BPR.1", "BPR.4"],
        ("GATE.2", "notch") => &["GATE.1", "GATE.4"],
        ("GATE.1" | "GATE.4", _) => &["GATE.2"],
        ("DUMMY.2", _) => &["DUMMY.4"],
        _ => &[],
    };
    pattern(deck, rule, variant, count, ignore);
}

/// The middle of line. A contact or via jogged a DBU is a DBU off its exact height
/// on four walls, so the bend tests ignore that rule. A VG off any gate lies inside
/// none, which VG.5 also reads; a V0 off M0 (M1) has a horizontal (vertical) wall off
/// it, which V0.8 (V0.7) also reads, and the other way about. A V0 wall must lie on
/// its wire's outline, as upstream's `not_inside_touching_edge` reads it: a wire a DBU
/// off the via, or a DBU wider each side, leaves two walls off it, and a jogged via
/// lies off both wires' outlines. SDCON.2 reads two
/// contacts on different nets: the pair joined through VSD and M0 does not count.
/// VG.6.LEF and VSD.6.LEF, the tech LEF's cut-layer SPACING, read two contacts side by
/// side a DBU too close as one; V0.7a reads two V0s on one M0 track the same way, and
/// not two on adjacent tracks, one above the other.
#[rstest]
#[case("vg", "VG.1", "narrow", 4)]
#[case("vg", "VG.2", "narrow", 4)]
#[case("vg", "VG.3", "cut", 3)]
#[case("vg", "VG.4", "alone", 2)]
#[case("vg", "VG.5", "outside", 2)]
#[case("vg", "VG.6.LEF", "close", 1)]
#[case("vg", "VG.7", "bent", 1)]
#[case("vg", "VG.M0.1", "outside", 2)]
#[case("vg", "VG.M0.2", "enc", 2)]
#[case("sdcon", "SDCON.1", "narrow", 4)]
#[case("sdcon", "SDCON.2", "close", 1)]
#[case("sdcon", "SDCON.3", "bent", 1)]
#[case("sdcon", "SDCON.ACT.1", "enc", 2)]
#[case("sdcon", "SDCON.GATE.1", "close", 1)]
#[case("vsd", "VSD.1", "narrow", 4)]
#[case("vsd", "VSD.2", "narrow", 4)]
#[case("vsd", "VSD.3", "bent", 1)]
#[case("vsd", "VSD.4", "outside", 2)]
#[case("vsd", "VSD.5", "outside", 2)]
#[case("vsd", "VSD.6.LEF", "close", 1)]
#[case("vsd", "VSD.SDCON.1", "enc", 2)]
#[case("vsd", "VSD.M0.1", "enc", 2)]
#[case("m0", "M0.1", "narrow", 4)]
#[case("m0", "M0.2", "short", 2)]
#[case("m0", "M0.3", "close", 1)]
#[case("m0", "M0.3", "notch", 1)]
#[case("m0", "M0.4", "close", 1)]
#[case("m0", "M0.4", "notch", 1)]
#[case("m0", "M0.5", "corner", 1)]
#[case("m0", "M0.6", "bent", 1)]
#[case("v0", "V0.1", "narrow", 4)]
#[case("v0", "V0.2", "narrow", 4)]
#[case("v0", "V0.3", "bent", 1)]
#[case("v0", "V0.4", "outside", 2)]
#[case("v0", "V0.5", "outside", 2)]
#[case("v0", "V0.6", "corner", 1)]
#[case("v0", "V0.7", "align", 4)]
#[case("v0", "V0.7a", "close", 1)]
#[case("v0", "V0.8", "align", 4)]
#[case("v0", "V0.M0.1", "enc", 2)]
#[case("v0", "V0.M1.1", "enc", 2)]
fn mol(#[case] deck: &str, #[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore: &[&str] = match (rule, variant) {
        ("VG.7", _) => &["VG.2"],
        ("VSD.3", _) => &["VSD.2"],
        ("V0.3", _) => &["V0.2", "V0.7", "V0.8"],
        ("VG.4", _) => &["VG.5"],
        ("M0.3" | "M0.4", "notch") => &["M0.1", "M0.6"],
        ("V0.4", _) => &["V0.8"],
        ("V0.8", _) => &["V0.4"],
        ("V0.5", _) => &["V0.7"],
        ("V0.7", _) => &["V0.5"],
        _ => &[],
    };
    pattern(deck, rule, variant, count, ignore);
}

/// The routing stack: every metal and via level drawn by one builder. A notch needs a
/// wire that bends, which `.6` also reads. A via off its metal has a wall off it too,
/// which `.3` (the metal along y) or `.4` (along x) also reads. A metal a DBU off
/// its via, or a DBU wider each side, leaves two of its walls off the outline. Each via
/// also has the tech LEF's cut-layer SPACING, `.5.LEF`, read every way: two vias side by
/// side a DBU too close give one.
#[rstest]
fn beol(#[values(1, 2, 3, 4, 5, 6)] n: u8) {
    let deck = format!("m{n}");
    for (r, variant, count) in [
        (1, "narrow", 2),
        (2, "short", 2),
        (3, "close", 1),
        (3, "notch", 1),
        (4, "close", 1),
        (4, "notch", 1),
        (5, "corner", 1),
        (6, "bent", 1),
    ] {
        let bend = format!("M{n}.6");
        let ignore: &[&str] = if variant == "notch" { &[&bend] } else { &[] };
        pattern(&deck, &format!("M{n}.{r}"), variant, count, ignore);
    }
    if n == 6 {
        return;
    }
    // The metal along y is the odd one of the two a via joins.
    let (my, mx) = if n % 2 == 1 { (n, n + 1) } else { (n + 1, n) };
    let deck = format!("v{n}");
    let id = |r: &str| format!("V{n}.{r}");
    let edge_rule = |m: u8| if m == my { id("3") } else { id("4") };
    for (r, variant, count, ignore) in [
        (id("1"), "narrow", 2, None),
        (id("2"), "narrow", 2, None),
        (id("3"), "align", 4, None),
        (id("4"), "align", 4, None),
        (format!("V{n}.5.LEF"), "close", 1, None),
        (id("6"), "corner", 1, None),
        (id("7"), "off", 2, Some(edge_rule(n))),
        (id("8"), "off", 2, Some(edge_rule(n + 1))),
        (id(&format!("M{my}.EN")), "enc", 2, None),
        (id(&format!("M{mx}.EN")), "enc", 2, None),
    ] {
        let ignore: Vec<&str> = ignore.iter().map(String::as_str).collect();
        pattern(&deck, &r, variant, count, &ignore);
    }
}

/// The routing stack above M6, from the tech LEF: M6's rules on each metal, without the
/// `.2` the LEF gives M10-M13 no MINSIZE for, and RDL with only a width and a space read
/// every way. A via has its `.5` space, read every way, two vias side by side a DBU too
/// close giving one; and `.7` and `.8` ask it inside each metal, so a metal missing,
/// abutting it, or a DBU aside give one each.
#[rstest]
fn beol_lef(#[values(7, 8, 9, 10, 11, 12, 13)] n: u8) {
    let deck = format!("m{n}");
    let id = |r: u8| format!("M{n}.{r}.LEF");
    for (r, variant, count) in [
        (1, "narrow", 2),
        (2, "short", 2),
        (3, "close", 1),
        (3, "notch", 1),
        (4, "close", 1),
        (4, "notch", 1),
        (5, "corner", 1),
        (6, "bent", 1),
    ] {
        if r == 2 && n >= 10 {
            continue;
        }
        let bend = id(6);
        let ignore: &[&str] = if variant == "notch" { &[&bend] } else { &[] };
        pattern(&deck, &id(r), variant, count, ignore);
    }
}

#[test]
fn rdl_lef() {
    pattern("rdl", "RDL.1.LEF", "narrow", 2, &[]);
    pattern("rdl", "RDL.3.LEF", "close", 1, &[]);
    pattern("rdl", "RDL.3.LEF", "notch", 1, &[]);
}

#[rstest]
fn via_lef(#[values(6, 7, 8, 9, 10, 11, 12, 13)] n: u8) {
    let name = |m: u8| {
        if m == 14 {
            "RDL".to_string()
        } else {
            format!("M{m}")
        }
    };
    let deck = format!("v{n}");
    let id = |r: &str| format!("V{n}.{r}.LEF");
    for (r, variant, count) in [
        (id("1"), "narrow", 2),
        (id("2"), "narrow", 2),
        (id("5"), "close", 1),
        (id("7"), "off", 3),
        (id("8"), "off", 3),
        (id(&format!("{}.EN", name(n))), "enc", 2),
        (id(&format!("{}.EN", name(n + 1))), "enc", 2),
    ] {
        pattern(&deck, &r, variant, count, &[]);
    }
}

/// The backside, from the tech LEF, as the frontside above M6 reads: BM1-BM4 without a
/// `.2`, BRDL with a width and a space read every way, and each via as V6-V13.
#[rstest]
fn backside_lef(#[values(1, 2, 3, 4)] n: u8) {
    let deck = format!("bm{n}");
    let id = |r: u8| format!("BM{n}.{r}.LEF");
    for (r, variant, count) in [
        (1, "narrow", 2),
        (3, "close", 1),
        (3, "notch", 1),
        (4, "close", 1),
        (4, "notch", 1),
        (5, "corner", 1),
        (6, "bent", 1),
    ] {
        let bend = id(6);
        let ignore: &[&str] = if variant == "notch" { &[&bend] } else { &[] };
        pattern(&deck, &id(r), variant, count, ignore);
    }
}

#[test]
fn brdl_lef() {
    pattern("brdl", "BRDL.1.LEF", "narrow", 2, &[]);
    pattern("brdl", "BRDL.3.LEF", "close", 1, &[]);
    pattern("brdl", "BRDL.3.LEF", "notch", 1, &[]);
}

#[rstest]
#[case(0, "BPR", "BM1")]
#[case(1, "BM1", "BM2")]
#[case(2, "BM2", "BM3")]
#[case(3, "BM3", "BM4")]
#[case(4, "BM4", "BRDL")]
fn backside_via_lef(#[case] n: u8, #[case] near: &str, #[case] far: &str) {
    let deck = format!("bv{n}");
    let id = |r: &str| format!("BV{n}.{r}.LEF");
    for (r, variant, count) in [
        (id("1"), "narrow", 2),
        (id("2"), "narrow", 2),
        (id("5"), "close", 1),
        (id("7"), "off", 3),
        (id("8"), "off", 3),
        (id(&format!("{near}.EN")), "enc", 2),
        (id(&format!("{far}.EN")), "enc", 2),
    ] {
        pattern(&deck, &r, variant, count, &[]);
    }
}
