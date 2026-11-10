// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GT3 tests.
//!
//! The decks translate GT3's IC Validator runset, which needs a licence to run, and above
//! M6, where the runset stops, its tech LEF. Every rule has a generated pattern
//! (`gen/gt`) under `tests/data/gt3/generated/<deck>/`, drawn to the source's values:
//! the shape exactly at the limit, then one DBU past it, inside whatever else its deck
//! reads. GT2N's runset is GT3's with seven values changed, and one generator draws
//! both PDKs' patterns.

use gds21::{GdsBoundary, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsUnits};
use gdscheck::pdk::PdkConfig;
use gdscheck::run_drc;
use rstest::rstest;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn the_feol_and_beol_suites_split_main() {
    let pdk = PdkConfig::for_process("gt3").expect("PDK loads");
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
    let (main, feol, beol) = (ids("main"), ids("feol"), ids("beol"));
    assert!(feol.is_disjoint(&beol), "a rule in both feol and beol");
    let all: BTreeSet<String> = feol.union(&beol).cloned().collect();
    assert_eq!(main, all);
}

// --- Generated patterns ---
// Each fixture leads with the shapes exactly at the limit, then the ones a DBU past
// it; the counts follow the drawings and the runset. Each case runs its whole deck.

/// Every rule in every deck has a generated fixture, and every fixture names a rule
/// its deck still has. A fixture is `<rule>.<case>.<polarity>.gds.gz`.
#[test]
fn every_rule_has_a_generated_fixture() {
    let pdk = PdkConfig::for_process("gt3").unwrap();
    for deck in &pdk.decks {
        let ids: BTreeSet<String> = pdk
            .load_deck(&deck.name)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        let dir = format!("tests/data/gt3/generated/{}", deck.name);
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
    let path = format!("tests/data/gt3/generated/{deck}/{rule}.{variant}.{polarity}.gds.gz");
    let mut got = BTreeMap::new();
    for v in run_drc(&path, "gt3", &[deck], None, "TOP", true).expect("DRC run failed") {
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

/// The front end. An exact width a DBU under and a DBU over gives two walls each, a
/// minimum width one DBU under gives two; a pair too close, a slot too narrow, a
/// shape that bends give one each.
///
/// GATE.2 reads the 42 nm pitch as well as the 27 nm space: a gate a DBU or half a pitch
/// off it gives one marker each, and the gate a DBU too close, or the U a DBU too
/// narrow inside, is off the pitch as well - two each. A gate a DBU narrow, wide or
/// jogged is off the pitch too, so the GATE.1 and GATE.4 tests ignore GATE.2. It reads
/// a gate missing from a row as a space too wide: one missing, three missing, and two
/// gates 993 nm apart, within the 1 µm the deck closes over, give one marker each; two
/// gates 25 pitches apart, beyond it, and two in different rows give none.
///
/// A notch needs a shape that bends, and its joint is no exact width, so the notch
/// tests ignore the shape's bend and exact-width rules. A DUMMY abutting a gate and
/// one alone lie inside none, which DUMMY.4 also reads.
///
/// A VBPR, GCUT or DUMMY abutting another shares no area with it, which the runset's
/// `include_touch = NONE` does not count; a GCUT abutting ACT is counted, under
/// `include_touch = ALL`. ACT.3 reads two actives corner to corner as it does two
/// facing across y. ACT.6 reads an end a DBU clear of its DUMMY and, stricter
/// than upstream's `not_interacting_edge`, one the DUMMY covers only half way. GCUT.7
/// reads a cut's wall inside a gate, flush with its wall on either side, but not
/// inside a gate under a DUMMY: three.
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
/// lies off both wires' outlines. SDCON.2 reads two contacts on different nets: the
/// pair joined through VSD and M0 does not count. A 13 nm VSD fills a 15 nm SDCON
/// with VSD.SDCON.1's 1 nm each side, so its enclosure pattern splits the 2 nm to
/// spare evenly, then 0.5 nm short on either side. VG.6.LEF and VSD.6.LEF, the tech
/// LEF's cut-layer SPACING, read two contacts side by side a DBU too close as one; V0.7a
/// reads two V0s on one M0 track the same way, and not two on adjacent tracks, one
/// above the other.
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

/// A GT3 library in its own 0.5 nm units, the top cell holding `elems`.
fn library(elems: Vec<GdsElement>) -> GdsLibrary {
    let mut top = GdsStruct::new("TOP");
    top.elems = elems;
    let mut lib = GdsLibrary::new("GT3");
    lib.units = GdsUnits(0.0005, 0.5e-9);
    lib.structs = vec![top];
    lib
}

/// All 41 drawing layers: negative half-nm coordinates are clean; a 0.25 nm shift
/// in x alone, y alone or both gives four markers per rectangle. These fixtures use
/// a 0.25 nm DBU, unlike the usual 0.5 nm GT3 fixtures, so the error is representable.
#[rstest]
#[case("FEOL.offgrid.LEF", 0)]
#[case("FEOL.offgrid.LEF", 168)]
#[case("BEOL.offgrid.LEF", 0)]
#[case("BEOL.offgrid.LEF", 324)]
fn manufacturing_grid_patterns(#[case] rule: &str, #[case] count: usize) {
    pattern("offgrid", rule, "grid", count, &[]);
}

/// A legal 14 x 28 nm M1 translated by 0.25 nm, and the same shift on NWELL; a pin
/// annotation is not manufacturing geometry. Only grid findings are counted.
#[rstest]
#[case("main", vec!["BEOL.offgrid.LEF", "FEOL.offgrid.LEF"])]
#[case("feol", vec!["FEOL.offgrid.LEF"])]
#[case("beol", vec!["BEOL.offgrid.LEF"])]
fn manufacturing_grid_runs_in_each_suite(#[case] suite: &str, #[case] rules: Vec<&str>) {
    let mut elems = Vec::new();
    for (layer, datatype) in [(1, 0), (25, 0), (25, 251)] {
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
    let violations =
        gdscheck::run_drc_with(&lib, "gt3", &[], Some(suite), "TOP", true).expect("DRC run failed");
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
