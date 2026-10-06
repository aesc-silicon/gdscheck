// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ASAP7 rule and suite regression tests.

use gdscheck::run_drc;
use rstest::rstest;
use std::collections::BTreeMap;

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

// --- Generated good/bad patterns ---
// Expected counts follow the drawings and the DRM. Each case checks its whole deck,
// with explicit exceptions for intentionally omitted via landing layers.

/// A good fixture is silent; a bad fixture reports only its target rule, with the
/// expected count, after excluding documented incidental rules.
fn pattern(deck: &str, rule: &str, variant: &str, count: usize, ignore: &[String]) {
    pattern_on(deck, rule, variant, count, ignore, false);
}

/// [`pattern`] with the nets extracted, for a deck whose rules read them.
fn pattern_nets(deck: &str, rule: &str, variant: &str, count: usize, ignore: &[String]) {
    pattern_on(deck, rule, variant, count, ignore, true);
}

fn pattern_on(deck: &str, rule: &str, variant: &str, count: usize, ignore: &[String], nets: bool) {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    assert!(
        pdk.load_deck(deck).unwrap().iter().any(|r| r.id == rule),
        "unknown rule {rule}"
    );
    let polarity = if count == 0 { "good" } else { "bad" };
    let path = format!("tests/data/asap7/generated/{deck}/{rule}.{variant}.{polarity}.gds.gz");
    let mut got = BTreeMap::new();
    for v in run_drc(&path, PDK, &[deck], None, "TOP", nets).expect("DRC run failed") {
        if !ignore.contains(&v.rule_id) {
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

/// DRM 3.5. The well rules: ordinary ACTIVE uses 27 nm and SRAM ACTIVE 13.5 nm, and
/// a crossing marker selects the whole polygon, even where it leaves ACTIVE unmarked;
/// the space patterns hold two violating neighbours, the enclosure patterns four.
///
/// The ACTIVE's own rules, each pattern a shape exactly at the limit and one a DBU
/// past it: a short or narrow bar has two failing walls; a pair too close, a slot too
/// narrow, an under-area bar or hole, a fin too near the bottom wall, and a bar
/// abutting the SRAM marker give one each; a bar half under its select and one with
/// a wall on the select's edge give two AUX.1 between them. The deck reads AUX.3 as
/// a notch under 1 µm across y, which any hole in an ordinary ACTIVE is, so the A.1B
/// test ignores it; SRAM ACTIVE has no notch rule, and its hole reads clean.
#[rstest]
#[case("ACTIVE.WELL.S.4", "outside", 2)]
#[case("ACTIVE.WELL.EN.1", "outside", 4)]
#[case("SRAM.ACTIVE.WELL.S.5", "inside", 2)]
#[case("SRAM.ACTIVE.WELL.EN.2", "inside", 4)]
#[case("SRAM.ACTIVE.WELL.S.5", "crossing", 2)]
#[case("SRAM.ACTIVE.WELL.EN.2", "crossing", 4)]
#[case("ACTIVE.W.1", "short", 2)]
#[case("ACTIVE.W.3", "narrow", 2)]
#[case("ACTIVE.S.1", "close", 1)]
#[case("ACTIVE.S.2B", "close", 1)]
#[case("ACTIVE.S.2B", "notch", 1)]
#[case("ACTIVE.AUX.3", "notch", 1)]
#[case("ACTIVE.A.1A", "small", 1)]
#[case("ACTIVE.A.1B", "hole", 1)]
#[case("SRAM.ACTIVE.A.2A", "small", 1)]
#[case("SRAM.ACTIVE.A.2B", "hole", 1)]
#[case("ACTIVE.FIN.EX.1", "short", 1)]
#[case("ACTIVE.AUX.1", "select", 2)]
#[case("SRAM.ACTIVE.AUX.2", "abut", 1)]
fn active(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore = match rule {
        "ACTIVE.A.1B" => vec!["ACTIVE.AUX.3".to_string()],
        _ => vec![],
    };
    pattern("active", rule, variant, count, &ignore);
}

/// DRM 3.8. The SRAM overlaps: 17 nm is legal, one grid step under has two narrow
/// walls, and a missing or touching-only reference gives one uncovered-region marker.
///
/// The trench's own rules, each pattern a trench exactly at the limit and one a DBU
/// past it: a narrow or short trench has two failing walls; a pair too close, a slot
/// too narrow, a trench too near a gate, one with a horizontal edge inside its ACTIVE,
/// one off its ACTIVE and one half out of its LISD give one each; a trench touching a
/// gate and one over it give two AUX.1, and nothing under S.2, which reads a gap and a
/// touching pair has none. An SRAM trench under 17 nm overlaps nothing by 17 nm, so the
/// W.4 test ignores the two overlaps.
#[rstest]
#[case("SRAM.SDT.ACTIVE.OV.3", "overlap_67", 2)]
#[case("SRAM.SDT.ACTIVE.OV.3", "overlap_68", 0)]
#[case("SRAM.SDT.ACTIVE.OV.3", "overlap_69", 0)]
#[case("SRAM.SDT.LISD.OV.4", "absent", 1)]
#[case("SRAM.SDT.LISD.OV.4", "touch", 1)]
#[case("SRAM.SDT.LISD.OV.4", "under", 2)]
#[case("SRAM.SDT.LISD.OV.4", "exact", 0)]
#[case("SRAM.SDT.LISD.OV.4", "over", 0)]
#[case("SRAM.SDT.ACTIVE.OV.3", "absent", 1)]
#[case("SRAM.SDT.ACTIVE.OV.3", "touch", 1)]
#[case("SRAM.SDT.ACTIVE.OV.3", "under", 2)]
#[case("SRAM.SDT.ACTIVE.OV.3", "exact", 0)]
#[case("SRAM.SDT.ACTIVE.OV.3", "over", 0)]
#[case("SDT.W.1", "narrow", 2)]
#[case("SDT.W.2", "short", 2)]
#[case("SRAM.SDT.W.4", "short", 2)]
#[case("SDT.S.1", "close", 1)]
#[case("SDT.S.1", "notch", 1)]
#[case("SDT.GATE.S.2", "close", 1)]
#[case("SDT.GATE.AUX.1", "touch", 2)]
#[case("SDT.ACTIVE.AUX.2", "inside", 1)]
#[case("SDT.ACTIVE.AUX.3", "off", 1)]
#[case("SDT.LISD.AUX.4", "uncovered", 1)]
fn sdt(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore: Vec<String> = match rule {
        "SRAM.SDT.W.4" => vec!["SRAM.SDT.ACTIVE.OV.3".into(), "SRAM.SDT.LISD.OV.4".into()],
        _ => vec![],
    };
    pattern("sdt", rule, variant, count, &ignore);
}

/// DRM 3.2: the well is drawn in rows, 108 nm across and 54 nm along, 108 nm between
/// rows and 54 nm along one, and 7 nm past a gate either way. A narrow or short well
/// has two failing walls; a pair too close, a slot too narrow, an under-area well or
/// hole and a short extension give one each.
///
/// The manual's own note says a well at both minimum widths is at the minimum area,
/// so an under-area well is also under W.1 and an under-area hole is also a notch
/// under S.1; those tests ignore the other rule.
#[rstest]
#[case("WELL.W.1", "narrow", 2)]
#[case("WELL.W.2", "short", 2)]
#[case("WELL.S.1", "close", 1)]
#[case("WELL.S.1", "notch", 1)]
#[case("WELL.S.2", "close", 1)]
#[case("WELL.S.2", "notch", 1)]
#[case("WELL.A.1A", "small", 1)]
#[case("WELL.A.1B", "hole", 1)]
#[case("WELL.GATE.EX.1", "short", 1)]
#[case("WELL.GATE.EX.2", "short", 1)]
fn well(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore = match rule {
        "WELL.A.1A" => vec!["WELL.W.1".to_string()],
        "WELL.A.1B" => vec!["WELL.S.1".to_string()],
        _ => vec![],
    };
    pattern("well", rule, variant, count, &ignore);
}

/// DRM 3.3: a fin runs along x, exactly 7 nm across and at least 108 nm long, on a
/// 27 nm pitch, and does not bend. A fin a DBU narrow or wide has two failing walls,
/// as does one a DBU short; a pair too close and a jogged fin give one each.
#[rstest]
#[case("FIN.W.1", "narrow", 4)]
#[case("FIN.W.2", "short", 2)]
#[case("FIN.S.1", "close", 1)]
#[case("FIN.AUX.1", "bent", 1)]
fn fin(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    pattern("fin", rule, variant, count, &[]);
}

/// DRM 3.4: a gate runs along y, exactly 20 nm across, at least 40 nm tall, on a 54 nm
/// pitch, and every gate drawn with a partner one pitch along for GATE.S.3. Narrow,
/// wide or short gates have two failing walls each; a pair too close, a jogged gate and
/// a short ACTIVE end give one; a gate a DBU short of ACTIVE fails on both gates of its
/// pair; a lone gate and a pair a DBU too far apart give three GATE.S.3.
///
/// An ACTIVE side inside a gate or flush with it also runs no way past the gate, which
/// GATE.ACTIVE.EX.2 reads, so that rule is ignored. The flush side, on the gate's left
/// wall, is reported twice: `inside_part` counts an edge on a polygon's left or bottom
/// wall as inside it but not one on its right or top wall, so both of AUX.3's entries
/// (inside, and coincident) see it. A side flush with a gate's right wall is reported
/// once. When that asymmetry is fixed, this count drops to 2.
#[rstest]
#[case("GATE.W.1", "narrow", 4)]
#[case("GATE.W.2", "short", 4)]
#[case("GATE.S.2", "close", 1)]
#[case("GATE.S.3", "solitary", 3)]
#[case("GATE.AUX.1", "bent", 1)]
#[case("GATE.ACTIVE.EX.1", "short", 2)]
#[case("GATE.ACTIVE.EX.2", "short", 1)]
#[case("GATE.ACTIVE.AUX.3", "inside", 3)]
#[case("GATE.ACTIVE.S.4", "close", 1)]
fn gate(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore = match rule {
        "GATE.ACTIVE.AUX.3" => vec!["GATE.ACTIVE.EX.2".to_string()],
        _ => vec![],
    };
    pattern("gate", rule, variant, count, &ignore);
}

/// DRM 3.6: a GCUT is a bar along x across the gates it cuts, at least 17 nm tall,
/// reaching 17 nm past each, its ends clear of gates and of the next gate by 17 nm, 4 nm
/// clear of any channel and 35 nm from the next bar. A narrow bar has two failing walls;
/// every other defect gives one. A bar ending inside a gate or flush with its side also
/// reaches no way past it, which GCUT.GATE.EX.1 reads, so that rule is ignored.
#[rstest]
#[case("GCUT.W.1", "narrow", 2)]
#[case("GCUT.GATE.EX.1", "short", 1)]
#[case("GCUT.GATE.S.2", "close", 1)]
#[case("GCUT.ACTIVE.S.1", "close", 1)]
#[case("GCUT.S.3", "close", 1)]
#[case("GCUT.S.3", "notch", 1)]
#[case("GCUT.AUX.1", "alone", 1)]
#[case("GCUT.AUX.2", "inside", 2)]
#[case("GCUT.AUX.3", "channel", 1)]
fn gcut(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore = match rule {
        "GCUT.AUX.2" => vec!["GCUT.GATE.EX.1".to_string()],
        _ => vec![],
    };
    pattern("gcut", rule, variant, count, &ignore);
}

/// DRM 3.9: a LISD line is 24 nm wide and 648 nm² - 24 by 27, so an under-area line
/// keeps its width - 18 nm from the next side to side, 25 nm tip to side and 27 nm
/// tip to tip, in SRAM too, and an ordinary line does not touch the SRAM marker. A
/// narrow line has two failing walls, and the pattern narrows one each way, so four;
/// everything else gives one.
#[rstest]
#[case("LISD.W.1", "narrow", 4)]
#[case("LISD.A.1", "small", 1)]
#[case("LISD.S.1", "close", 1)]
#[case("LISD.S.2", "close", 1)]
#[case("LISD.S.3", "close", 1)]
#[case("SRAM.LISD.S.4", "close", 1)]
#[case("SRAM.LISD.AUX.1", "abut", 1)]
fn lisd(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    pattern("lisd", rule, variant, count, &[]);
}

/// DRM 3.10, with the nets extracted: three of the rules read them. Each pattern a
/// shape exactly at the limit and one a DBU past it. A narrow line has two failing
/// walls, narrowed each way, so four; a pair or corner too close, a short overlap or
/// extension, an under-area line or overlap, and a line abutting the marker give one
/// each. LISD.S.6's third pair, as close as its second, is joined through V0 and M1
/// and so one net, which the rule leaves alone.
///
/// A 16 nm LIG reaching under 8 nm into a LISD shares under 128 nm² with it, so A.2's
/// test ignores OV.1. A LIG ending inside a gate shares 160 nm² with it, so AUX.1's
/// test ignores A.3 - EX.1 reads only a gate wall the LIG covers, and says nothing of
/// a LIG ending in or flush with one. The flush end on the gate's left wall is reported
/// twice, as GATE.ACTIVE.AUX.3's is and for the same reason, so three until that is
/// fixed, then two.
#[rstest]
#[case("LIG.W.1", "narrow", 4)]
#[case("LIG.A.1", "small", 1)]
#[case("LIG.S.1", "close", 1)]
#[case("LIG.S.2", "close", 1)]
#[case("LIG.S.3", "close", 1)]
#[case("LIG.S.4", "close", 1)]
#[case("LIG.S.5", "close", 1)]
#[case("LIG.LISD.S.6", "close", 1)]
#[case("LIG.LISD.S.7", "corner", 1)]
#[case("LIG.SDT.S.8", "close", 1)]
#[case("LIG.GATE.S.9A", "close", 1)]
#[case("LIG.GATE.S.9B", "close", 1)]
#[case("LIG.GATE.S.10", "corner", 1)]
#[case("LIG.GCUT.S.11", "close", 1)]
#[case("LIG.GATE.EX.1", "short", 1)]
#[case("LIG.GATE.A.3", "small", 1)]
#[case("LIG.GATE.AUX.1", "inside", 3)]
#[case("LIG.LISD.OV.1", "short", 1)]
#[case("LIG.LISD.A.2", "small", 1)]
#[case("SRAM.LIG.GATE.OV.2", "short", 1)]
#[case("SRAM.LIG.GATE.A.4", "small", 1)]
#[case("SRAM.LIG.AUX.2", "abut", 1)]
fn lig(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let ignore: Vec<String> = match rule {
        "LIG.LISD.A.2" => vec!["LIG.LISD.OV.1".into()],
        "LIG.GATE.AUX.1" => vec!["LIG.GATE.A.3".into()],
        _ => vec![],
    };
    pattern_nets("lig", rule, variant, count, &ignore);
}

/// DRM 3.7: one rule set on NSELECT, PSELECT, SLVT, LVT and SRAMVT. A select is drawn
/// in rows like the well, 108 nm across and 54 nm along: a narrow or short one has two
/// failing walls. It holds an ordinary ACTIVE by 46 nm across and 27 nm along, an SRAM
/// ACTIVE by 13.5 nm either way, and runs 7 nm past a gate either way: a short margin
/// gives one. NSELECT over PSELECT gives one; each VT layer over each other gives one,
/// three in all under the one rule.
#[rstest]
#[case("NSELECT.W.1", "narrow", 2)]
#[case("NSELECT.W.2", "short", 2)]
#[case("NSELECT.ACTIVE.EN.1", "short", 1)]
#[case("NSELECT.ACTIVE.EN.2", "short", 1)]
#[case("SRAM.NSELECT.ACTIVE.EN.3", "short", 1)]
#[case("SRAM.NSELECT.ACTIVE.EN.4", "short", 1)]
#[case("NSELECT.GATE.EX.1", "short", 1)]
#[case("NSELECT.GATE.EX.2", "short", 1)]
#[case("PSELECT.W.1", "narrow", 2)]
#[case("PSELECT.W.2", "short", 2)]
#[case("PSELECT.ACTIVE.EN.1", "short", 1)]
#[case("PSELECT.ACTIVE.EN.2", "short", 1)]
#[case("SRAM.PSELECT.ACTIVE.EN.3", "short", 1)]
#[case("SRAM.PSELECT.ACTIVE.EN.4", "short", 1)]
#[case("PSELECT.GATE.EX.1", "short", 1)]
#[case("PSELECT.GATE.EX.2", "short", 1)]
#[case("SLVT.W.1", "narrow", 2)]
#[case("SLVT.W.2", "short", 2)]
#[case("SLVT.ACTIVE.EN.1", "short", 1)]
#[case("SLVT.ACTIVE.EN.2", "short", 1)]
#[case("SRAM.SLVT.ACTIVE.EN.3", "short", 1)]
#[case("SRAM.SLVT.ACTIVE.EN.4", "short", 1)]
#[case("SLVT.GATE.EX.1", "short", 1)]
#[case("SLVT.GATE.EX.2", "short", 1)]
#[case("LVT.W.1", "narrow", 2)]
#[case("LVT.W.2", "short", 2)]
#[case("LVT.ACTIVE.EN.1", "short", 1)]
#[case("LVT.ACTIVE.EN.2", "short", 1)]
#[case("SRAM.LVT.ACTIVE.EN.3", "short", 1)]
#[case("SRAM.LVT.ACTIVE.EN.4", "short", 1)]
#[case("LVT.GATE.EX.1", "short", 1)]
#[case("LVT.GATE.EX.2", "short", 1)]
#[case("SRAMVT.W.1", "narrow", 2)]
#[case("SRAMVT.W.2", "short", 2)]
#[case("SRAMVT.ACTIVE.EN.1", "short", 1)]
#[case("SRAMVT.ACTIVE.EN.2", "short", 1)]
#[case("SRAM.SRAMVT.ACTIVE.EN.3", "short", 1)]
#[case("SRAM.SRAMVT.ACTIVE.EN.4", "short", 1)]
#[case("SRAMVT.GATE.EX.1", "short", 1)]
#[case("SRAMVT.GATE.EX.2", "short", 1)]
#[case("NSELECT.PSELECT.AUX.1", "overlap", 1)]
#[case("VT.AUX.2", "overlap", 3)]
fn select(#[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    pattern("select", rule, variant, count, &[]);
}

/// DRM 3.1: no edge off the horizontal or vertical, read on the ordinary shapes of
/// every drawn layer and on the SRAM marker itself. One 45° chamfer per layer the deck
/// reads gives one edge marker each; the same chamfers under a square marker give none.
#[test]
fn geometry() {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    let layers = pdk
        .load_deck("geometry")
        .unwrap()
        .iter()
        .filter(|r| r.id == "GEOMETRY.NONORTHOGONAL" && r.check == "no_angle")
        .count();
    assert!(layers >= 35, "{layers} layers");
    pattern("geometry", "GEOMETRY.NONORTHOGONAL", "chamfer", layers, &[]);
    pattern("geometry", "GEOMETRY.NONORTHOGONAL", "sram", 0, &[]);
}

/// The two enclosure rules of each via level, the lower layer's first.
const VIA_ENCLOSURES: [(&str, &str); 10] = [
    ("V0.LISD.EN.2", "V0.M1.EN.1"),
    ("V1.M1.EN.1", "V1.M2.EN.2"),
    ("V2.M2.EN.1", "V2.M3.EN.2"),
    ("V3.M3.EN.1", "V3.M4.EN.2"),
    ("V4.M4.EN.1", "V4.M5.EN.2"),
    ("V5.M5.EN.1", "V5.M6.EN.2"),
    ("V6.M6.EN.1", "V6.M7.EN.2"),
    ("V7.M7.EN.1", "V7.M8.EN.2"),
    ("V8.M8.EN.1", "V8.M9.EN.2"),
    ("V9.M9.EN.1", "V9.PAD.EN.2"),
];

/// DRM 3.11-3.19: every via in a whole stack, each pattern a via or gap exactly at the
/// limit and one DBU past it:
/// - a narrow V0-V3 has two failing walls, and the patterns narrow it each way, so
///   four; a V4-V6 a DBU short or long along its upper metal has two failing walls,
///   so four, while V7, read on its short side, gives one per via; the V8/V9 pattern
///   holds a narrow
///   via, a 40 x 80 one and an L, which is both not a rectangle and 120 nm across;
/// - each pair too close gives one, side by side on one track or, on V0-V3, partly
///   aligned on neighbouring tracks; V4-V9 add a corner pair under the same rule;
/// - V0-V3 corner spacing reads the upper layer's 5 nm end-caps at the two corners
///   forming the gap, never the far ends;
/// - V1.M1.EN.1's 5 and 2 must be on one pair: split across two, or either one
///   short, gives three;
/// - a via missing its lower and one missing its upper layer give one AUX.1 each.
/// - a V0 must cross its LIG: flush with its side, a side inside it, or over its end
///   gives one V0.LIG.AUX.2 each.
/// - a V0 partly on LISD needs 3 nm of it on both sides across: each side a DBU short
///   gives one V0.LISD.EN.3;
/// - under the SRAM marker (DRM 1.2.2, convention 7) each spacing and enclosure defect
///   is drawn four times: with the marker over the via, over one via of a pair, along
///   its edge only, and over its landing layer clear of it. The first two are exempt,
///   the last two not - edge contact is not membership and the landing layer stays
///   whole - so two.
///
/// Some rules overlap by their nature, and the patterns ignore the other: a missing
/// layer encloses nothing, a V0 that doesn't cross the whole 16 nm LIG shares less
/// than V0.LIG.A.1's 288 nm² with it, and V3's corner patterns give M4 a 5 nm end-cap, under its
/// 11 nm V3.M4.EN.2 - which, with V3.M4.AUX.2 holding M4 flush across, leaves the
/// short end-caps of V3.S.3 and V3.S.4 unreachable in a legal layout.
#[rstest]
#[case("v0", "V0.W.1", "narrow", 4)]
#[case("v0", "V0.S.1", "close", 1)]
#[case("v0", "V0.S.1", "adjacent", 1)]
#[case("v0", "V0.S.2", "both", 1)]
#[case("v0", "V0.S.3", "neither", 1)]
#[case("v0", "V0.S.3", "under", 1)]
#[case("v0", "V0.S.4", "one", 1)]
#[case("v0", "V0.LISD.EN.2", "short", 1)]
#[case("v0", "V0.M1.EN.1", "short", 1)]
#[case("v0", "V0.M1.AUX.3", "overhang", 1)]
#[case("v0", "V0.AUX.1", "uncovered", 2)]
#[case("v0", "V0.S.1", "sram", 2)]
#[case("v0", "V0.LISD.EN.2", "sram", 2)]
#[case("v0", "V0.M1.EN.1", "sram", 2)]
#[case("v0", "V0.LISD.EN.3", "short", 2)]
#[case("v0", "V0.LIG.EN.4", "short", 1)]
#[case("v0", "V0.LIG.A.1", "small", 1)]
#[case("v0", "V0.LIG.AUX.2", "uncrossed", 3)]
#[case("v1", "V1.W.1", "narrow", 4)]
#[case("v1", "V1.S.1", "close", 1)]
#[case("v1", "V1.S.1", "adjacent", 1)]
#[case("v1", "V1.S.2", "both", 1)]
#[case("v1", "V1.S.3", "neither", 1)]
#[case("v1", "V1.S.3", "under", 1)]
#[case("v1", "V1.S.4", "one", 1)]
#[case("v1", "V1.M1.EN.1", "short", 3)]
#[case("v1", "V1.M2.EN.2", "short", 1)]
#[case("v1", "V1.M2.AUX.2", "overhang", 1)]
#[case("v1", "V1.AUX.1", "uncovered", 2)]
#[case("v1", "V1.S.1", "sram", 2)]
#[case("v1", "V1.M1.EN.1", "sram", 2)]
#[case("v1", "V1.M2.EN.2", "sram", 2)]
#[case("v2", "V2.W.1", "narrow", 4)]
#[case("v2", "V2.S.1", "close", 1)]
#[case("v2", "V2.S.1", "adjacent", 1)]
#[case("v2", "V2.S.2", "both", 1)]
#[case("v2", "V2.S.3", "neither", 1)]
#[case("v2", "V2.S.3", "under", 1)]
#[case("v2", "V2.S.4", "one", 1)]
#[case("v2", "V2.M2.EN.1", "short", 1)]
#[case("v2", "V2.M3.EN.2", "short", 1)]
#[case("v2", "V2.M3.AUX.2", "overhang", 1)]
#[case("v2", "V2.AUX.1", "uncovered", 2)]
#[case("v2", "V2.S.1", "sram", 2)]
#[case("v2", "V2.M2.EN.1", "sram", 2)]
#[case("v2", "V2.M3.EN.2", "sram", 2)]
#[case("v3", "V3.W.1", "narrow", 4)]
#[case("v3", "V3.S.1", "close", 1)]
#[case("v3", "V3.S.1", "adjacent", 1)]
#[case("v3", "V3.S.2", "both", 1)]
#[case("v3", "V3.S.3", "neither", 1)]
#[case("v3", "V3.S.3", "under", 1)]
#[case("v3", "V3.S.4", "one", 1)]
#[case("v3", "V3.M3.EN.1", "short", 1)]
#[case("v3", "V3.M4.EN.2", "short", 1)]
#[case("v3", "V3.M4.AUX.2", "overhang", 1)]
#[case("v3", "V3.AUX.1", "uncovered", 2)]
#[case("v3", "V3.S.1", "sram", 2)]
#[case("v3", "V3.M3.EN.1", "sram", 2)]
#[case("v3", "V3.M4.EN.2", "sram", 2)]
#[case("v4", "V4.W.1", "narrow", 4)]
#[case("v4", "V4.S.2", "close", 2)]
#[case("v4", "V4.M4.EN.1", "short", 1)]
#[case("v4", "V4.M5.EN.2", "short", 1)]
#[case("v4", "V4.M5.AUX.2", "overhang", 1)]
#[case("v4", "V4.AUX.1", "uncovered", 2)]
#[case("v4", "V4.S.2", "sram", 2)]
#[case("v4", "V4.M4.EN.1", "sram", 2)]
#[case("v4", "V4.M5.EN.2", "sram", 2)]
#[case("v5", "V5.W.1", "narrow", 4)]
#[case("v5", "V5.S.2", "close", 2)]
#[case("v5", "V5.M5.EN.1", "short", 1)]
#[case("v5", "V5.M6.EN.2", "short", 1)]
#[case("v5", "V5.M6.AUX.2", "overhang", 1)]
#[case("v5", "V5.AUX.1", "uncovered", 2)]
#[case("v5", "V5.S.2", "sram", 2)]
#[case("v5", "V5.M5.EN.1", "sram", 2)]
#[case("v5", "V5.M6.EN.2", "sram", 2)]
#[case("v6", "V6.W.1", "narrow", 4)]
#[case("v6", "V6.S.2", "close", 2)]
#[case("v6", "V6.M6.EN.1", "short", 1)]
#[case("v6", "V6.M7.EN.2", "short", 1)]
#[case("v6", "V6.M7.AUX.2", "overhang", 1)]
#[case("v6", "V6.AUX.1", "uncovered", 2)]
#[case("v6", "V6.S.2", "sram", 2)]
#[case("v6", "V6.M6.EN.1", "sram", 2)]
#[case("v6", "V6.M7.EN.2", "sram", 2)]
#[case("v7", "V7.W.1", "narrow", 2)]
#[case("v7", "V7.S.2", "close", 2)]
#[case("v7", "V7.M7.EN.1", "short", 1)]
#[case("v7", "V7.M8.EN.2", "short", 1)]
#[case("v7", "V7.M8.AUX.2", "overhang", 1)]
#[case("v7", "V7.AUX.1", "uncovered", 2)]
#[case("v7", "V7.S.2", "sram", 2)]
#[case("v7", "V7.M7.EN.1", "sram", 2)]
#[case("v7", "V7.M8.EN.2", "sram", 2)]
#[case("v8", "V8.W.1", "narrow", 4)]
#[case("v8", "V8.S.1", "close", 2)]
#[case("v8", "V8.M8.EN.1", "short", 1)]
#[case("v8", "V8.M9.EN.2", "short", 1)]
#[case("v8", "V8.AUX.1", "uncovered", 2)]
#[case("v8", "V8.S.1", "sram", 2)]
#[case("v8", "V8.M8.EN.1", "sram", 2)]
#[case("v8", "V8.M9.EN.2", "sram", 2)]
#[case("v9", "V9.W.1", "narrow", 4)]
#[case("v9", "V9.S.1", "close", 2)]
#[case("v9", "V9.M9.EN.1", "short", 1)]
#[case("v9", "V9.PAD.EN.2", "short", 1)]
#[case("v9", "V9.AUX.1", "uncovered", 2)]
#[case("v9", "V9.S.1", "sram", 2)]
#[case("v9", "V9.M9.EN.1", "sram", 2)]
#[case("v9", "V9.PAD.EN.2", "sram", 2)]
fn via(#[case] deck: &str, #[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let level: usize = deck[1..].parse().unwrap();
    let (lower, upper) = VIA_ENCLOSURES[level];
    let ignore: Vec<String> = match variant {
        "uncovered" => vec![lower.into(), upper.into()],
        "uncrossed" => vec!["V0.LIG.A.1".into()],
        "both" | "neither" | "under" | "one" if level == 3 => vec![upper.into()],
        _ => vec![],
    };
    pattern(deck, rule, variant, count, &ignore);
}

/// DRM 3.12-3.18: width, space, area and edge patterns on every metal. Each pattern
/// holds shapes exactly at the limit and shapes one DBU past it:
/// - a narrow or wide rectangle has two failing walls, so the M1-M3 and M8/M9 W.1
///   patterns, narrow once in x and once in y, give four, as do the two even widths;
/// - a pair too close, a notch too narrow, a rectangle under the area floor and an
///   edge too short each give one;
/// - some rules overlap by their nature, and the patterns ignore the other one: every
///   M8/M9 rectangle narrower than 40 nm has edges under L.1's 40 nm; every M4-M7
///   notch makes a non-rectangle, which AUX.3 forbids; and the widest M4-M7 wire is
///   twenty tracks, an even width W.3 forbids.
#[rstest]
#[case("m1", "M1.W.1", "narrow", 4)]
#[case("m1", "M1.S.1", "close", 1)]
#[case("m1", "M1.S.2", "close", 1)]
#[case("m1", "M1.S.3", "close", 1)]
#[case("m1", "M1.S.4", "close", 1)]
#[case("m1", "M1.S.5", "close", 1)]
#[case("m1", "M1.S.6", "corner", 1)]
#[case("m1", "M1.A.1", "small", 1)]
#[case("m2", "M2.W.1", "narrow", 4)]
#[case("m2", "M2.S.1", "close", 1)]
#[case("m2", "M2.S.2", "close", 1)]
#[case("m2", "M2.S.3", "close", 1)]
#[case("m2", "M2.S.4", "close", 1)]
#[case("m2", "M2.S.5", "close", 1)]
#[case("m2", "M2.S.6", "corner", 1)]
#[case("m2", "M2.A.1", "small", 1)]
#[case("m3", "M3.W.1", "narrow", 4)]
#[case("m3", "M3.S.1", "close", 1)]
#[case("m3", "M3.S.2", "close", 1)]
#[case("m3", "M3.S.3", "close", 1)]
#[case("m3", "M3.S.4", "close", 1)]
#[case("m3", "M3.S.5", "close", 1)]
#[case("m3", "M3.S.6", "corner", 1)]
#[case("m3", "M3.A.1", "small", 1)]
#[case("m4", "M4.W.1", "narrow", 2)]
#[case("m4", "M4.W.2", "wide", 2)]
#[case("m4", "M4.W.3", "even", 4)]
#[case("m4", "M4.W.5", "short", 2)]
#[case("m4", "M4.S.1", "close", 1)]
#[case("m4", "M4.S.1", "notch", 1)]
#[case("m4", "M4.S.2", "close", 1)]
#[case("m4", "M4.S.2", "notch", 1)]
#[case("m4", "M4.S.3", "corner", 1)]
#[case("m5", "M5.W.1", "narrow", 2)]
#[case("m5", "M5.W.2", "wide", 2)]
#[case("m5", "M5.W.3", "even", 4)]
#[case("m5", "M5.W.5", "short", 2)]
#[case("m5", "M5.S.1", "close", 1)]
#[case("m5", "M5.S.1", "notch", 1)]
#[case("m5", "M5.S.2", "close", 1)]
#[case("m5", "M5.S.2", "notch", 1)]
#[case("m5", "M5.S.3", "corner", 1)]
#[case("m6", "M6.W.1", "narrow", 2)]
#[case("m6", "M6.W.2", "wide", 2)]
#[case("m6", "M6.W.3", "even", 4)]
#[case("m6", "M6.W.5", "short", 2)]
#[case("m6", "M6.S.1", "close", 1)]
#[case("m6", "M6.S.1", "notch", 1)]
#[case("m6", "M6.S.2", "close", 1)]
#[case("m6", "M6.S.2", "notch", 1)]
#[case("m6", "M6.S.3", "corner", 1)]
#[case("m7", "M7.W.1", "narrow", 2)]
#[case("m7", "M7.W.2", "wide", 2)]
#[case("m7", "M7.W.3", "even", 4)]
#[case("m7", "M7.W.5", "short", 2)]
#[case("m7", "M7.S.1", "close", 1)]
#[case("m7", "M7.S.1", "notch", 1)]
#[case("m7", "M7.S.2", "close", 1)]
#[case("m7", "M7.S.2", "notch", 1)]
#[case("m7", "M7.S.3", "corner", 1)]
#[case("m8", "M8.W.1", "narrow", 4)]
#[case("m8", "M8.W.2", "narrow", 2)]
#[case("m8", "M8.W.3", "narrow", 2)]
#[case("m8", "M8.W.4", "narrow", 2)]
#[case("m8", "M8.W.5", "wide", 1)]
#[case("m8", "M8.S.1", "close", 1)]
#[case("m8", "M8.S.2", "close", 1)]
#[case("m8", "M8.S.3", "close", 1)]
#[case("m8", "M8.S.4", "close", 1)]
#[case("m8", "M8.S.5", "close", 1)]
#[case("m8", "M8.S.6", "close", 1)]
#[case("m8", "M8.S.7", "close", 1)]
#[case("m8", "M8.S.8", "close", 1)]
#[case("m8", "M8.A.1", "small", 1)]
#[case("m8", "M8.L.1", "step", 1)]
#[case("m9", "M9.W.1", "narrow", 4)]
#[case("m9", "M9.W.2", "narrow", 2)]
#[case("m9", "M9.W.3", "narrow", 2)]
#[case("m9", "M9.W.4", "narrow", 2)]
#[case("m9", "M9.W.5", "wide", 1)]
#[case("m9", "M9.S.1", "close", 1)]
#[case("m9", "M9.S.2", "close", 1)]
#[case("m9", "M9.S.3", "close", 1)]
#[case("m9", "M9.S.4", "close", 1)]
#[case("m9", "M9.S.5", "close", 1)]
#[case("m9", "M9.S.6", "close", 1)]
#[case("m9", "M9.S.7", "close", 1)]
#[case("m9", "M9.S.8", "close", 1)]
#[case("m9", "M9.A.1", "small", 1)]
#[case("m9", "M9.L.1", "step", 1)]
fn metal(#[case] deck: &str, #[case] rule: &str, #[case] variant: &str, #[case] count: usize) {
    let layer = &rule[..2];
    let ignore = match &rule[3..] {
        "W.1" if matches!(layer, "M8" | "M9") => vec![format!("{layer}.L.1")],
        "S.1" | "S.2" if variant == "notch" => vec![format!("{layer}.AUX.3")],
        "W.2" if variant == "wide" => vec![format!("{layer}.W.3")],
        _ => vec![],
    };
    pattern(deck, rule, variant, count, &ignore);
}

/// DRM 3.19: three clean upper-metal nets; shortening M9's end-cap from exactly
/// 20 nm to 19.75 nm introduces one V8 enclosure violation in the whole main suite.
#[rstest]
#[case("routed", 0)]
#[case("routed_exact", 0)]
#[case("routed", 1)]
fn routed_enclosure(#[case] variant: &str, #[case] count: usize) {
    let polarity = if count == 0 { "good" } else { "bad" };
    let path = format!("tests/data/asap7/generated/v8/V8.M9.EN.2.{variant}.{polarity}.gds.gz");
    let violations = run_drc(&path, PDK, &[], Some("main"), "TOP", false).expect("DRC run failed");
    let ids: Vec<_> = violations.iter().map(|v| v.rule_id.as_str()).collect();
    assert_eq!(ids, vec!["V8.M9.EN.2"; count], "{path}");
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
        true,
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
                v.iter()
                    .filter(|v| v.rule_id == format!("{layer}.W.2"))
                    .count(),
                if bad { 2 } else { 0 },
                "{layer}: {width} nm across, {length} nm along, tile {tile}: {v:?}"
            );
            assert!(
                v.iter()
                    .all(|v| [format!("{layer}.W.2"), format!("{layer}.W.3")].contains(&v.rule_id)),
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
#[case::v4("v4", "V4", 24.0, 23.75, "V4.W.1")]
#[case::v5("v5", "V5", 23.75, 24.0, "V5.W.1")]
#[case::v6("v6", "V6", 32.0, 31.75, "V6.W.1")]
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

/// Neither an empty intersection nor ordinary containment can stand in for the
/// explicit 17 nm SRAM overlap. Select the whole SDT, retaining full references.
#[rstest]
#[case::active("ACTIVE", "LISD", "SRAM.SDT.ACTIVE.OV.3")]
#[case::lisd("LISD", "ACTIVE", "SRAM.SDT.LISD.OV.4")]
fn sram_sdt_requires_overlap_even_when_reference_is_absent(
    #[case] reference: &str,
    #[case] other: &str,
    #[case] rule: &str,
) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for overlap in [
            None,
            Some(-0.25),
            Some(0.0),
            Some(0.25),
            Some(16.75),
            Some(17.0),
            Some(17.25),
        ] {
            for marker in [[-10.0, -10.0, 40.0, 40.0], [23.75, 26.75, 40.0, 40.0]] {
                let mut shapes = vec![
                    ("SDT", [0.0, 0.0, 24.0, 27.0]),
                    (other, [-10.0, -10.0, 40.0, 40.0]),
                    ("SRAMDRC", marker),
                ];
                if let Some(h) = overlap {
                    shapes.push((reference, [0.0, h - 50.0, 24.0, h]));
                }
                let v = rectangles("sdt", &shapes, origin, tile);
                assert_eq!(
                    v.iter().any(|v| v.rule_id == rule),
                    overlap.is_none_or(|h| h < 17.0),
                    "{reference}, overlap {overlap:?}, marker {marker:?}, tile {tile}: {v:?}"
                );
                assert!(v.iter().all(|v| v.rule_id == rule), "{v:?}");
            }
        }
    }
}

/// DRM figures 3.11.1(b,c) and 3.13.1: only end-caps at the gap's
/// corners relax its spacing. Both vias always have a remote 5 nm end-cap.
#[rstest]
#[case::v0("v0", "V0", "M1")]
#[case::v1("v1", "V1", "M2")]
#[case::v2("v2", "V2", "M3")]
#[case::v3("v3", "V3", "M4")]
fn via_corner_spacing_reads_the_local_endcaps(
    #[case] deck: &str,
    #[case] via: &str,
    #[case] metal: &str,
) {
    for (origin, tile) in [
        (100.0, 20.0),
        (6_990.0, 7.0),
        (19_990.0, 20.0),
        (-10.0, 7.0),
        (6_982.0, 7.0),
    ] {
        for (cap_a, cap_b, suffix, gap_bad, gap_good) in [
            (5.0, 5.0, "S.2", 14.25, 14.5),
            (5.25, 5.0, "S.2", 14.25, 14.5),
            (0.0, 0.0, "S.3", 23.75, 24.0),
            (4.75, 4.75, "S.3", 23.75, 24.0),
            (5.0, 0.0, "S.4", 20.0, 20.25),
            (0.0, 5.0, "S.4", 20.0, 20.25),
            (4.75, 5.0, "S.4", 20.0, 20.25),
        ] {
            for (gap, bad) in [(gap_bad, true), (gap_good, false)] {
                for rotate in [false, true] {
                    for sram in [false, true] {
                        let x = 18.0 + gap;
                        let mut shapes = vec![
                            (via, [0.0, 0.0, 18.0, 18.0]),
                            (via, [x, 36.0, x + 18.0, 54.0]),
                            (metal, [-5.0, 0.0, 18.0 + cap_a, 18.0]),
                            (metal, [x - cap_b, 36.0, x + 23.0, 54.0]),
                        ];
                        if sram {
                            shapes.push(("SRAMDRC", [-10.0, -10.0, 100.0, 100.0]));
                        }
                        if rotate {
                            for (_, r) in &mut shapes {
                                *r = [r[1], r[0], r[3], r[2]];
                            }
                        }
                        let v = rectangles(deck, &shapes, origin, tile);
                        let ids: Vec<_> = v
                            .iter()
                            .filter(|v| {
                                ["S.2", "S.3", "S.4"]
                                    .iter()
                                    .any(|s| v.rule_id == format!("{via}.{s}"))
                            })
                            .map(|v| v.rule_id.as_str())
                            .collect();
                        let expected = format!("{via}.{suffix}");
                        assert_eq!(
                            ids,
                            if bad && !sram {
                                vec![expected.as_str()]
                            } else {
                                vec![]
                            },
                            "{via}, caps {cap_a}/{cap_b}, gap {gap}, rotate {rotate}, SRAM {sram}, origin {origin}, tile {tile}: {v:?}"
                        );
                    }
                }
            }
        }
    }
}

/// DRM 3.11 figure 3.11.1(d): a via may protrude past LISD onto LIG,
/// but the overlap still needs 3 nm on the same opposite pair of lateral sides.
#[test]
fn partly_landed_v0_keeps_its_lateral_lisd_enclosure() {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for (left, right, bad) in [
            (3.0, 3.0, false),
            (3.25, 3.25, false),
            (2.75, 3.0, true),
            (3.0, 2.75, true),
            (0.0, 6.0, true),
        ] {
            for rotate in [false, true] {
                let mut shapes = vec![
                    ("V0", [0.0, 0.0, 18.0, 18.0]),
                    ("LISD", [-left, -30.0, 18.0 + right, 9.0]),
                    ("LIG", [-20.0, 1.0, 40.0, 17.0]),
                    ("M1", [-5.0, 0.0, 23.0, 18.0]),
                ];
                if rotate {
                    for (_, r) in &mut shapes {
                        *r = [r[1], r[0], r[3], r[2]];
                    }
                }
                let v = rectangles("v0", &shapes, origin, tile);
                assert_eq!(
                    v.iter().any(|v| v.rule_id == "V0.LISD.EN.3"),
                    bad,
                    "margins {left}/{right}, rotate {rotate}, tile {tile}: {v:?}"
                );
                assert!(v.iter().all(|v| v.rule_id == "V0.LISD.EN.3"), "{v:?}");
            }
        }
    }
}

/// Same-net qualification must include remote metal routes, including portions
/// inside SRAM. Cutting either landing via restores the different-net violation.
#[rstest]
#[case::lisd("LISD", "LIG.LISD.S.6")]
#[case::sdt("SDT", "LIG.SDT.S.8")]
fn lig_spacing_resolves_remote_connections(#[case] other: &str, #[case] rule: &str) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for connected in [false, true] {
            let mut shapes = vec![
                ("LIG", [0.0, 0.0, 16.0, 100.0]),
                (other, [29.75, 0.0, 53.75, 100.0]),
                ("LISD", [29.75, 80.0, 53.75, 100.0]),
                ("V0", [0.0, 80.0, 16.0, 98.0]),
                ("M1", [-5.0, 80.0, 60.0, 98.0]),
                // Mark only the connecting metal, not the measured subjects.
                ("SRAMDRC", [-5.0, 85.0, -1.0, 95.0]),
            ];
            if connected {
                shapes.push(("V0", [32.0, 80.0, 50.0, 98.0]));
            }
            let v = rectangles("lig", &shapes, origin, tile);
            assert_eq!(
                v.iter().any(|v| v.rule_id == rule),
                !connected,
                "{other}, connected {connected}, tile {tile}: {v:?}"
            );
        }
    }
}

#[rstest]
#[case::m4("m4", "M4", 24.0, true)]
#[case::m5("m5", "M5", 24.0, false)]
#[case::m6("m6", "M6", 32.0, true)]
#[case::m7("m7", "M7", 32.0, false)]
fn routing_width_cannot_be_an_even_multiple(
    #[case] deck: &str,
    #[case] layer: &str,
    #[case] width: f64,
    #[case] horizontal: bool,
) {
    for (origin, tile) in [(100.0, 20.0), (6_990.0, 7.0), (19_990.0, 20.0)] {
        for multiple in 1..=20 {
            for delta in [-0.25, 0.0, 0.25] {
                let w = width * multiple as f64 + delta;
                let r = if horizontal {
                    [0.0, 0.0, 1000.0, w]
                } else {
                    [0.0, 0.0, w, 1000.0]
                };
                for sram in [false, true] {
                    let mut shapes = vec![(layer, r)];
                    if sram {
                        shapes.push(("SRAMDRC", [-1.0, -1.0, 1.0, 1.0]));
                    }
                    let v = rectangles(deck, &shapes, origin, tile);
                    assert_eq!(
                        v.iter().any(|v| v.rule_id == format!("{layer}.W.3")),
                        multiple % 2 == 0 && delta == 0.0 && !sram,
                        "{layer}, width {w}, SRAM {sram}: {v:?}"
                    );
                }
            }
        }
    }
}

/// Marker locations, not just verdicts, must survive exact tile-corner contact.
#[test]
fn region_marker_is_stable_on_a_tile_corner() {
    for (deck, layer, rect) in [
        ("v0", "V0", [0.0, 0.0, 18.0, 18.0]),
        // This under-area metal crosses the vertical tile line while its top
        // lies on the horizontal line: multiple pieces, all top corners lost
        // by a strictly-interior-only vertex average.
        ("m1", "M1", [-5.0, 0.0, 22.75, 18.0]),
    ] {
        let signature = |tile| {
            let v = rectangles(deck, &[(layer, rect)], 6_982.0, tile);
            let mut s: Vec<_> = v
                .iter()
                .map(|v| format!("{} {:?}", v.rule_id, v.geometry))
                .collect();
            s.sort();
            s
        };
        assert_eq!(signature(7.0), signature(20.0), "{deck}");
    }
}

/// Separate ACTIVEs extending the same long FIN wall must contribute the same
/// marker extent whether both references fit in one tile or occupy two tiles.
#[test]
fn fin_extension_marker_keeps_every_partial_covering_stretch() {
    let shapes = [
        ("FIN", [0.0, 0.0, 9_400.0, 7.0]),
        ("ACTIVE", [0.0, -5.0, 16.0, 12.0]),
        ("ACTIVE", [9_384.0, -5.0, 9_400.0, 12.0]),
    ];
    let signature = |tile| {
        let v = rectangles("active", &shapes, 100.0, tile);
        let mut s: Vec<_> = v
            .iter()
            .filter(|v| v.rule_id == "ACTIVE.FIN.EX.1")
            .map(|v| match v.geometry {
                // Orthogonal fixture coordinates are exact DBU; ignore only
                // floating-point arithmetic noise in the final µm conversion.
                gdscheck::violation::ViolationGeometry::Edge { x1, y1, x2, y2 } => {
                    [x1, y1, x2, y2].map(|x| (x / 0.00025).round() as i64)
                }
                ref other => panic!("expected an edge: {other:?}"),
            })
            .collect();
        s.sort();
        s
    };
    assert_eq!(signature(7.0).len(), 2);
    assert_eq!(signature(7.0), signature(20.0));
}
