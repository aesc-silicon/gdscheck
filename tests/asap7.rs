// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ASAP7 tests.
//!
//! The Calibre deck the manual was written against is not public, so there are no
//! foundry test cases to run the decks on.  What there is, is the standard-cell
//! library the PDK ships, which was verified with that deck: every cell of it, placed
//! once, is the fixture.  A rule that reads the manual wrongly shows up here as a
//! violation on a cell that has none.
//!
//! The baseline includes isolated-cell context effects. The half-width
//! filler FILLERxp5 and the TAPCELL_WITH_FILLER are 54 nm wide, under the 108 nm the
//! manual asks of a well, an implant and a fin across, which only abutting cells make
//! up; and DFFASRHQNx1 draws one LIG 3 nm from an SDT.  The KLayout port of the manual
//! reports these too.  FILLERxp5's gate also has no neighbour until a cell is placed
//! beside it, which the port's GATE.S.3 - a gate grown and tested against itself -
//! cannot see.
//! The external comparison also records additional KLayout findings which need
//! individual review; these counts alone do not establish checker parity.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "asap7";
const LIB: &str = "tests/data/asap7/static/asap7sc7p5t_28_L.gds.gz";

fn counts(gds: &str, suite: &str) -> Vec<(String, usize)> {
    let violations = run_drc(gds, PDK, &[], Some(suite), "ALLCELLS", true).expect("DRC run failed");
    let mut by_rule: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for v in violations {
        *by_rule.entry(v.rule_id).or_default() += 1;
    }
    by_rule.into_iter().collect()
}

/// Freeze the reviewed library baseline. A width is two markers, one per wall;
/// the independent comparison records disagreements beyond this regression test.
#[test]
fn the_standard_cell_library_matches_the_reviewed_baseline() {
    let want: Vec<(String, usize)> = [
        ("FIN.W.2", 20),
        ("GATE.S.3", 2),
        ("LIG.SDT.S.8", 1),
        ("LVT.W.1", 2),
        ("NSELECT.W.1", 4),
        ("PSELECT.W.1", 4),
        ("WELL.W.1", 2),
    ]
    .iter()
    .map(|(id, n)| ((*id).to_string(), *n))
    .collect();
    assert_eq!(counts(LIB, "main"), want);
}

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
