// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ASAP7 rule and suite regression tests.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "asap7";

/// The front-end and back-end suites split the full run between them: every deck is
/// in one or the other, and none in both.
#[test]
fn the_feol_and_beol_suites_split_main() {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).expect("PDK loads");
    let ids = |suite: &str| -> std::collections::BTreeSet<String> {
        pdk.load_suite(suite)
            .expect("suite loads")
            .iter()
            .map(|r| {
                let layers: Vec<&str> = r.layers.iter().map(|l| l.name.as_str()).collect();
                format!("{} {} {:?} {}", r.id, r.check, layers, r.value)
            })
            .collect()
    };
    let (main, feol, beol) = (ids("main"), ids("feol"), ids("beol"));
    assert!(feol.is_disjoint(&beol), "a rule in both feol and beol");
    let both: std::collections::BTreeSet<String> = feol.union(&beol).cloned().collect();
    let missing: Vec<&String> = main.difference(&both).collect();
    assert!(missing.is_empty(), "in main, in neither half: {missing:?}");
}

/// Portable GDS cases shared with the independent KLayout comparison. Each row
/// names a manual-derived target; incidental rules remain visible in the oracle
/// report but do not change that target's expectation.
#[test]
fn generated_oracle_cases_follow_the_manual() {
    use std::collections::BTreeMap;
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    let rules = pdk.load_suite("main").unwrap();
    let mut cases: BTreeMap<&str, Vec<(&str, bool)>> = BTreeMap::new();
    for line in include_str!("data/asap7/generated/cases.tsv").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 4);
        assert!(matches!(columns[2], "pass" | "fail"));
        cases
            .entry(columns[0])
            .or_default()
            .push((columns[1], columns[2] == "fail"));
    }
    assert!(!cases.is_empty());
    for (name, expectations) in cases {
        let path = format!("tests/data/asap7/generated/{name}.gds.gz");
        let violations = run_drc(&path, PDK, &[], Some("main"), "TOP", false).unwrap();
        for (rule, bad) in expectations {
            assert!(
                rules.iter().any(|r| r.id == rule),
                "unknown target rule {rule}"
            );
            assert_eq!(
                violations.iter().any(|v| v.rule_id == rule),
                bad,
                "{name}: {rule}: {violations:?}"
            );
        }
        if name == "routed_clean" {
            assert!(violations.is_empty(), "{violations:?}");
        }
        if name == "routed_bad" {
            assert_eq!(violations.len(), 1, "{violations:?}");
            assert_eq!(violations[0].rule_id, "V8.M9.EN.2");
        }
    }
}

/// Rectangles in nm, on ASAP7's 0.25 nm grid. Moving the same drawing across tile
/// boundaries checks that a rule reads whole regions, including both via sides.
fn rectangles(
    deck: &str,
    shapes: &[(&str, [f64; 4])],
    origin: f64,
    tile_um: f64,
) -> Vec<gdscheck::Violation> {
    use gds21::{GdsBoundary, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsUnits};
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    let mut top = GdsStruct::new("TOP");
    for &(name, [x0, y0, x1, y1]) in shapes {
        let layer = pdk.layer(name).unwrap();
        let point = |x: f64, y: f64| {
            GdsPoint::new(
                ((origin + x) * 4.0).round() as i32,
                ((origin + y) * 4.0).round() as i32,
            )
        };
        top.elems.push(GdsElement::GdsBoundary(GdsBoundary {
            layer: layer.gds_layer as i16,
            datatype: layer.gds_datatype as i16,
            xy: vec![
                point(x0, y0),
                point(x1, y0),
                point(x1, y1),
                point(x0, y1),
                point(x0, y0),
            ],
            ..Default::default()
        }));
    }
    let mut lib = GdsLibrary::new("ASAP7_REGRESSION");
    lib.units = GdsUnits(0.00025, 0.25e-9);
    lib.structs = vec![top];
    gdscheck::run_drc_with_options(
        &lib,
        PDK,
        &[deck],
        None,
        "TOP",
        false,
        &gdscheck::RunOptions {
            tile_um,
            ..Default::default()
        },
    )
    .unwrap()
}

/// DRM 3.14/3.16 bound vertical M4/M6 width; M5/M7 rotate that bound.
/// A short, over-wide rectangle must fail even when its other dimension is
/// narrow. Long legal wires must pass: the limit is across the specified axis.
#[rstest]
#[case::m4("m4", "M4", 480.0, true)]
#[case::m5("m5", "M5", 480.0, false)]
#[case::m6("m6", "M6", 640.0, true)]
#[case::m7("m7", "M7", 640.0, false)]
fn directional_max_width_uses_the_routing_axis(
    #[case] deck: &str,
    #[case] layer: &str,
    #[case] limit: f64,
    #[case] horizontal: bool,
) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for (width, length, bad) in [
            (limit - 0.25, 100.0, false),
            (limit, 100.0, false),
            (limit + 0.25, 100.0, true),
            (100.0, 5_000.0, false),
            (limit + 0.25, 5_000.0, true),
        ] {
            let (x, y) = if horizontal {
                (length, width)
            } else {
                (width, length)
            };
            let v = rectangles(deck, &[(layer, [0.0, 0.0, x, y])], origin, tile);
            assert_eq!(
                v.len(),
                if bad { 2 } else { 0 },
                "{layer}: {width} nm across, {length} nm along, tile {tile}: {v:?}"
            );
            assert!(
                v.iter().all(|v| v.rule_id == format!("{layer}.W.2")),
                "{v:?}"
            );
        }
    }
}

/// Two 18 nm line ends 24.5 nm apart break the 31 nm tip-to-tip space. The SRAM
/// exemption requires positive-area overlap with either wire; edge-only and
/// point-only contact leave the pair checked, including across tile boundaries.
#[rstest]
#[case::m1("m1", "M1")]
#[case::m2("m2", "M2")]
#[case::m3("m3", "M3")]
fn m1_to_m3_spacing_skips_shapes_overlapping_sramdrc(#[case] deck: &str, #[case] layer: &str) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for (marker, bad) in [
            (None, true),
            (Some([500.0, 0.0, 600.0, 100.0]), true),
            (Some([-50.0, -50.0, 70.0, 300.0]), false),
            (Some([-50.0, 150.0, 70.0, 300.0]), false),
            (Some([18.0, 150.0, 70.0, 300.0]), true),
            (Some([18.0, 224.5, 70.0, 300.0]), true),
            (Some([18.25, 224.5, 70.0, 300.0]), true),
            (Some([17.75, 224.25, 70.0, 300.0]), false),
        ] {
            let mut shapes = vec![
                (layer, [0.0, 0.0, 18.0, 100.0]),
                (layer, [0.0, 124.5, 18.0, 224.5]),
            ];
            shapes.extend(marker.map(|m| ("SRAMDRC", m)));
            let v = rectangles(deck, &shapes, origin, tile);
            assert_eq!(!v.is_empty(), bad, "{layer}, marker {marker:?}: {v:?}");
            assert!(
                v.iter().all(|v| v.rule_id == format!("{layer}.S.4")),
                "{v:?}"
            );
        }
    }
}

/// Width and area exemptions have the same positive-area boundary as spacing.
#[rstest]
#[case::m1_width("m1", "M1", 17.75, 100.0, "M1.W.1")]
#[case::m2_width("m2", "M2", 17.75, 100.0, "M2.W.1")]
#[case::m3_width("m3", "M3", 17.75, 100.0, "M3.W.1")]
#[case::m1_area("m1", "M1", 18.0, 27.75, "M1.A.1")]
#[case::m2_area("m2", "M2", 18.0, 27.75, "M2.A.1")]
#[case::m3_area("m3", "M3", 18.0, 27.75, "M3.A.1")]
fn m1_to_m3_width_and_area_skip_shapes_overlapping_sramdrc(
    #[case] deck: &str,
    #[case] layer: &str,
    #[case] width: f64,
    #[case] height: f64,
    #[case] rule: &str,
) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for (marker, bad) in [
            (None, true),
            (
                Some([width + 0.25, height, width + 100.0, height + 100.0]),
                true,
            ),
            (Some([width, 0.0, width + 100.0, height]), true),
            (Some([width, height, width + 100.0, height + 100.0]), true),
            (
                Some([width - 0.25, height - 0.25, width + 100.0, height + 100.0]),
                false,
            ),
            (Some([-1.0, -1.0, width + 1.0, height + 1.0]), false),
        ] {
            let mut shapes = vec![(layer, [0.0, 0.0, width, height])];
            shapes.extend(marker.map(|m| ("SRAMDRC", m)));
            let v = rectangles(deck, &shapes, origin, tile);
            assert_eq!(!v.is_empty(), bad, "{layer}, marker {marker:?}: {v:?}");
            assert!(v.iter().all(|v| v.rule_id == rule), "{v:?}");
        }
    }
}

/// Each ordinary width family must flag its defect outside SRAM and retain contact-only
/// cases, while even a one-DBU overlap at the far corner exempts the entire polygon.
#[rstest]
#[case::well("well", "NWELL", 107.75, 54.0, "WELL.W.1")]
#[case::fin("fin", "FIN", 120.0, 6.75, "FIN.W.1")]
#[case::gate("gate", "GATE", 19.75, 100.0, "GATE.W.1")]
#[case::gcut("gcut", "GCUT", 40.0, 16.75, "GCUT.W.1")]
#[case::active("active", "ACTIVE", 15.75, 27.0, "ACTIVE.W.3")]
#[case::sdt("sdt", "SDT", 23.75, 27.0, "SDT.W.1")]
#[case::lisd("lisd", "LISD", 23.75, 100.0, "LISD.W.1")]
#[case::lig("lig", "LIG", 15.75, 100.0, "LIG.W.1")]
#[case::nselect("select", "NSELECT", 107.75, 54.0, "NSELECT.W.1")]
#[case::pselect("select", "PSELECT", 107.75, 54.0, "PSELECT.W.1")]
#[case::slvt("select", "SLVT", 107.75, 54.0, "SLVT.W.1")]
#[case::lvt("select", "LVT", 107.75, 54.0, "LVT.W.1")]
#[case::sramvt("select", "SRAMVT", 107.75, 54.0, "SRAMVT.W.1")]
#[case::m4("m4", "M4", 100.0, 23.75, "M4.W.1")]
#[case::m5("m5", "M5", 23.75, 100.0, "M5.W.1")]
#[case::m6("m6", "M6", 100.0, 31.75, "M6.W.1")]
#[case::m7("m7", "M7", 31.75, 100.0, "M7.W.1")]
#[case::m8("m8", "M8", 39.75, 100.0, "M8.W.1")]
#[case::m9("m9", "M9", 39.75, 100.0, "M9.W.1")]
#[case::v0("v0", "V0", 17.75, 18.0, "V0.W.1")]
#[case::v1("v1", "V1", 17.75, 18.0, "V1.W.1")]
#[case::v2("v2", "V2", 17.75, 18.0, "V2.W.1")]
#[case::v3("v3", "V3", 17.75, 18.0, "V3.W.1")]
#[case::v4("v4", "V4", 23.75, 24.0, "V4.W.1")]
#[case::v5("v5", "V5", 23.75, 24.0, "V5.W.1")]
#[case::v6("v6", "V6", 31.75, 32.0, "V6.W.1")]
#[case::v7("v7", "V7", 31.75, 32.0, "V7.W.1")]
#[case::v8("v8", "V8", 39.75, 40.0, "V8.W.1")]
#[case::v9("v9", "V9", 39.75, 40.0, "V9.W.1")]
fn ordinary_width_rules_share_the_sram_partition(
    #[case] deck: &str,
    #[case] layer: &str,
    #[case] width: f64,
    #[case] height: f64,
    #[case] rule: &str,
) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for (marker, bad) in [
            (None, true),
            (Some([width, 0.0, width + 100.0, height]), true),
            (Some([width, height, width + 100.0, height + 100.0]), true),
            (
                Some([width - 0.25, height - 0.25, width + 100.0, height + 100.0]),
                false,
            ),
            (Some([-1.0, -1.0, width + 1.0, height + 1.0]), false),
        ] {
            let mut shapes = vec![(layer, [0.0, 0.0, width, height])];
            shapes.extend(marker.map(|m| ("SRAMDRC", m)));
            let v = rectangles(deck, &shapes, origin, tile);
            assert_eq!(
                v.iter().any(|v| v.rule_id == rule),
                bad,
                "{rule}, marker {marker:?}, tile {tile}: {v:?}"
            );
        }
    }
}

/// A marker over all physical geometry must leave only explicit SRAM checks active.
/// This also exercises derived forbidden/coverage predicates, not just width checks.
#[test]
fn a_fully_marked_layout_keeps_only_explicit_sram_checks() {
    let raw: serde_norway::Value =
        serde_norway::from_str(include_str!("../pdks/asap7/pdk.yml")).unwrap();
    let names: Vec<_> = raw["layers"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .filter(|n| !n.contains('.') && *n != "SRAMDRC")
        .collect();
    let mut shapes: Vec<_> = names.iter().map(|&n| (n, [0.0, 0.0, 10.0, 10.0])).collect();
    shapes.push(("SRAMDRC", [-1.0, -1.0, 11.0, 11.0]));
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    let mut saw_sram = false;
    for deck in &pdk.decks {
        let v = rectangles(&deck.name, &shapes, 6_995.0, 7.0);
        assert!(
            v.iter().all(|v| v.rule_id.starts_with("SRAM.")),
            "{}: {v:?}",
            deck.name
        );
        saw_sram |= !v.is_empty();
    }
    assert!(
        saw_sram,
        "The marker must not disable the explicit SRAM rules"
    );
}

#[rstest]
#[case::lisd("lisd", "LISD", 24.0, 17.75, "LISD.S.1")]
#[case::lig("lig", "LIG", 16.0, 17.75, "LIG.S.1")]
#[case::m8("m8", "M8", 40.0, 39.75, "M8.S.1")]
#[case::m9("m9", "M9", 40.0, 39.75, "M9.S.1")]
fn ordinary_edge_spacing_requires_two_non_sram_polygons(
    #[case] deck: &str,
    #[case] layer: &str,
    #[case] width: f64,
    #[case] gap: f64,
    #[case] rule: &str,
) {
    for tile in [7.0, 20.0] {
        for (marker, bad) in [
            (None, true),
            (Some([0.0, 90.0, width, 110.0]), false),
            (Some([width, 90.0, width + 1.0, 110.0]), true),
        ] {
            let mut shapes = vec![
                (layer, [0.0, 0.0, width, 100.0]),
                (layer, [width + gap, 0.0, 2.0 * width + gap, 100.0]),
            ];
            shapes.extend(marker.map(|m| ("SRAMDRC", m)));
            let v = rectangles(deck, &shapes, 6_990.0, tile);
            assert_eq!(
                v.iter().any(|v| v.rule_id == rule),
                bad,
                "{rule}, {marker:?}: {v:?}"
            );
        }
    }
}

#[test]
fn a_remote_marker_exempts_the_whole_wire_across_tiles() {
    for tile in [7.0, 20.0] {
        let mut shapes = vec![
            ("M1", [0.0, 0.0, 18.0, 25_000.0]),
            ("M1", [0.0, 25_024.5, 18.0, 25_124.5]),
        ];
        let v = rectangles("m1", &shapes, 100.0, tile);
        assert!(v.iter().any(|v| v.rule_id == "M1.S.4"), "{v:?}");
        shapes.push(("SRAMDRC", [0.0, 0.0, 18.0, 1.0]));
        let v = rectangles("m1", &shapes, 100.0, tile);
        assert!(
            v.is_empty(),
            "A marker 25 um from the gap must select the whole wire: {v:?}"
        );
    }
}

/// An outside via must still see all its landing metal, even if that metal is SRAM
/// elsewhere. A defective enclosure must remain visible too; filtering both operands
/// would either erase the defect or create a false missing-metal finding.
#[rstest]
#[case::v1(("v1", "V1", "M1", "M2"), (18.0, 5.0, 1.0), "V1.M1.EN.1")]
#[case::v8(("v8", "V8", "M9", "M8"), (40.0, 20.0, 19.75), "V8.M9.EN.2")]
fn enclosure_keeps_full_reference_metal(
    #[case] stack: (&str, &str, &str, &str),
    #[case] dimensions: (f64, f64, f64),
    #[case] rule: &str,
) {
    let (deck, via, metal, other) = stack;
    let (size, good, bad) = dimensions;
    for tile in [7.0, 20.0] {
        for margin in [good, bad] {
            let mut shapes = vec![
                (via, [0.0, 0.0, size, size]),
                (metal, [-margin, -margin, 120.0, size + margin]),
                (other, [-good, -good, size + good, size + good]),
            ];
            let before = rectangles(deck, &shapes, 6_990.0, tile);
            shapes.push(("SRAMDRC", [100.0, 0.0, 130.0, size]));
            let after = rectangles(deck, &shapes, 6_990.0, tile);
            let count = |v: &[gdscheck::Violation]| v.iter().filter(|v| v.rule_id == rule).count();
            assert_eq!(
                count(&before),
                count(&after),
                "{rule}: {before:?} vs {after:?}"
            );
            assert_eq!(count(&after) > 0, margin == bad, "{rule}: {after:?}");
            assert_eq!(
                before.len(),
                after.len(),
                "Reference membership changed another via check"
            );
        }
    }
}

#[rstest]
#[case::one_nm(1.0, true)]
#[case::just_under(16.75, true)]
#[case::exact(17.0, false)]
#[case::over(17.25, false)]
#[case::full(27.0, false)]
fn sram_sdt_requires_seventeen_nm_of_active_overlap(#[case] overlap: f64, #[case] bad: bool) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        let shapes = [
            ("SDT", [0.0, 0.0, 24.0, 27.0]),
            ("ACTIVE", [-10.0, 27.0 - overlap, 40.0, 54.0 - overlap]),
            ("LISD", [-5.0, -5.0, 30.0, 60.0]),
            ("SRAMDRC", [-20.0, -20.0, 70.0, 70.0]),
        ];
        let v = rectangles("sdt", &shapes, origin, tile);
        assert_eq!(v.len(), if bad { 2 } else { 0 }, "{v:?}");
        assert!(
            v.iter().all(|v| v.rule_id == "SRAM.SDT.ACTIVE.OV.3"),
            "{v:?}"
        );
    }
}

#[test]
fn sram_sdt_overlap_also_checks_an_active_contained_inside_it() {
    let v = rectangles(
        "sdt",
        &[
            ("SDT", [0.0, 0.0, 24.0, 27.0]),
            ("ACTIVE", [2.0, 2.0, 22.0, 18.75]),
            ("LISD", [-5.0, -5.0, 30.0, 60.0]),
            ("SRAMDRC", [-20.0, -20.0, 70.0, 70.0]),
        ],
        100.0,
        20.0,
    );
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(v.iter().all(|v| v.rule_id == "SRAM.SDT.ACTIVE.OV.3"));
}

#[rstest]
#[case::split_axes([5.0, 0.0, 2.0, 2.0], true)]
#[case::small_side_under([5.0, 1.75, 0.0, 0.0], true)]
#[case::large_side_under([4.75, 2.0, 0.0, 0.0], true)]
#[case::both_small([2.0, 2.0, 2.0, 2.0], true)]
#[case::left([5.0, 2.0, 0.0, 0.0], false)]
#[case::right([2.0, 5.0, 0.0, 0.0], false)]
#[case::bottom([0.0, 0.0, 5.0, 2.0], false)]
#[case::top([0.0, 0.0, 2.0, 5.0], false)]
#[case::over([5.25, 2.25, 0.0, 0.0], false)]
fn v1_enclosure_requires_five_and_two_on_the_same_pair(
    #[case] margins: [f64; 4],
    #[case] bad: bool,
) {
    let [left, right, bottom, top] = margins;
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        let v = rectangles(
            "v1",
            &[
                ("V1", [0.0, 0.0, 18.0, 18.0]),
                ("M1", [-left, -bottom, 18.0 + right, 18.0 + top]),
                ("M2", [0.0, -5.0, 18.0, 23.0]),
            ],
            origin,
            tile,
        );
        assert_eq!(v.len(), usize::from(bad), "{v:?}");
        assert!(v.iter().all(|v| v.rule_id == "V1.M1.EN.1"), "{v:?}");
    }
}
