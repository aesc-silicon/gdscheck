// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GF180MCU variant D.
//!
//! Two kinds of case.  The foundry's own DRC unit-test layouts
//! (`libs.tech/klayout/tech/drc/testing/unit/`, vendored under
//! `tests/data/gf180mcuD/static/`) are dense layouts exercising a whole deck at once, and
//! what is pinned for each is the per-rule count this engine reports and that every rule
//! the deck declares fires somewhere on it.  The counts are this engine's own output, so
//! a count moving is a prompt to look at what moved it, not a verdict on its own; what
//! they guard is the geometry - real foundry cases rather than patterns drawn to match
//! the implementation - and a rule that resolves to an empty derived layer, which
//! produces no violations and no error and would otherwise read as a clean design.
//!
//! Whether a rule is *right* is asked of the generated patterns under
//! `tests/data/gf180mcuD/generated/`, one good and one bad half per rule drawn by
//! `gen/gf180mcuD/`: the good half is a hard zero, and the bad half fires that rule and
//! only that rule.  That is the direction new work goes; a per-rule count on a foundry
//! case says "about as many markers as before", and a rule that over-reports can hide
//! inside that.
//!
//! The `nplus` and `pplus` fixtures are the only two vendored here that are not
//! byte-identical to upstream: each carried four TEXT elements with a zero-length
//! STRING (an empty label), which panics gds21's `read_str` - it indexes
//! `data[len - 1]` without guarding `len == 0`.  The four elements were removed, since
//! an empty label carries no DRC information.  The reader bug is still a bug: a real
//! design with an empty text label would crash the same way.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "gf180mcuD";
const DATA: &str = "tests/data/gf180mcuD/static";
const GENERATED: &str = "tests/data/gf180mcuD/generated";

/// Violation count per rule id, sorted by id, for a fixture under `static/`.
fn counts(deck: &str, gds: &str, topcell: &str) -> Vec<(String, usize)> {
    counts_at(deck, &format!("{DATA}/{gds}"), topcell)
}

/// The same, for a fixture anywhere.
fn counts_at(deck: &str, path: &str, topcell: &str) -> Vec<(String, usize)> {
    let violations = run_drc(path, PDK, &[deck], None, topcell, true).expect("DRC run failed");
    let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
    for v in violations {
        *by_rule.entry(v.rule_id).or_default() += 1;
    }
    by_rule.into_iter().collect()
}

// --- N-well ---
//
// --- The foundry's cases ---
//
// The per-rule counts are this engine's own output, false positives and all, so a
// change in a count is a prompt to look at what moved it - the generated patterns
// further down say whether a rule is *right*.  What the counts guard is that a change
// in a rule, a derived layer or the engine shows up on real foundry geometry, and the
// coverage test after them that a rule which silently matches nothing - the failure
// mode a skeleton invites - cannot pass for a clean design.

/// A seal ring at 400 x 300 µm with the die it guards: metal on every level standing
/// 8.8 µm off the marker, where GR.2 asks ten, and everything else the ring satisfies,
/// two of them exactly - its active is 16 µm against GR.6's minimum of 16, and its
/// contacts and vias sit at 0.7000 µm against GR.7 and GR.8's 0.7.  Upstream implements
/// no GR rule at all, so there is no reference to score this against; what the drawing
/// says is that nothing but GR.2 fires, and that it fires once for every block of the
/// die's metal - 10 µm blocks at a 15 µm pitch along each wall, 23 along the 350.4 µm
/// bottom and top, 15 along the 234.4 µm the left and right columns have between the
/// rows, on five levels - and that is what is asked.  The ring spans twenty tiles each
/// way, which is the point: a block a tile reads from its neighbour's copy has to be
/// reported by someone, and five such walls once went unreported.
#[test]
fn guard_ring_on_a_seal_ring_reports_every_wall_on_every_level() {
    let violations = run_drc(
        &format!("{GENERATED}/guard_ring/seal.gds.gz"),
        PDK,
        &["guard_ring"],
        None,
        "TOP",
        true,
    )
    .expect("DRC run failed");
    let others: Vec<_> = violations.iter().filter(|v| v.rule_id != "GR.2").collect();
    assert!(others.is_empty(), "only GR.2 may fire: {others:?}");
    // The die's metal starts 24.8 µm in from the ring's outline at 10; a marker's gap
    // lies on the side it is nearest.
    let (x0, y0, x1, y1) = (10.0 + 24.8, 10.0 + 24.8, 410.0 - 24.8, 310.0 - 24.8);
    let mut seen: std::collections::BTreeSet<(&str, &str)> = Default::default();
    for v in &violations {
        let level = ["metal1", "metal2", "metal3", "metal4", "metal5"]
            .into_iter()
            .find(|m| v.message.contains(&format!("between {m} and")))
            .expect("GR.2 names its metal");
        let (mx, my) = match v.geometry {
            gdscheck::violation::ViolationGeometry::Edge { x1, y1, x2, y2 } => {
                ((x1 + x2) * 0.5, (y1 + y2) * 0.5)
            }
            gdscheck::violation::ViolationGeometry::Point { x, y } => (x, y),
            gdscheck::violation::ViolationGeometry::None => continue,
        };
        let side = if mx <= x0 {
            "left"
        } else if mx >= x1 {
            "right"
        } else if my <= y0 {
            "bottom"
        } else if my >= y1 {
            "top"
        } else {
            panic!("marker inside the die at ({mx}, {my})");
        };
        seen.insert((level, side));
    }
    assert_eq!(seen.len(), 20, "every level on every side: {seen:?}");
    let along = |len: f64| ((len - 10.0) / 15.0).floor() as usize + 1;
    let blocks = 2 * along(x1 - x0) + 2 * along(y1 - y0 - 2.0 * 8.0);
    assert_eq!(
        violations.len(),
        5 * blocks,
        "one marker per block on every level"
    );
}

/// The foundry's own cases, with the per-rule counts this port produces on them.  Rule
/// ids in the foundry's naming, so a row reads against the DRM.
#[rstest]
#[case::antenna_1(
    "antenna", "antenna-1.gds.gz", "8_0_ANT",
    &[("ANT.1", 1), ("ANT.16_i_ANT.10", 1), ("ANT.16_i_ANT.11", 1), ("ANT.16_i_ANT.12", 1), ("ANT.16_i_ANT.2", 1), ("ANT.16_i_ANT.3", 1), ("ANT.16_i_ANT.4", 1), ("ANT.16_i_ANT.5", 1), ("ANT.16_i_ANT.6", 4), ("ANT.16_i_ANT.9", 1), ("ANT.8", 1)]
)]
#[case::dummy_exclude(
    "dummy_exclude", "dummy_exclude.gds.gz", "10_8_DE",
    &[("DE.2", 3), ("DE.3", 1), ("DE.4", 2)]
)]
#[case::ldpmos(
    "ldpmos", "ldpmos.gds.gz", "10_12_2_MDP",
    &[("MDP.1", 16), ("MDP.10", 16), ("MDP.10a", 7), ("MDP.10b", 4), ("MDP.11", 36), ("MDP.12", 4), ("MDP.13a", 1), ("MDP.13b", 21), ("MDP.13c", 3), ("MDP.15", 1), ("MDP.16a", 2), ("MDP.16b", 2), ("MDP.17a", 4), ("MDP.17c", 1), ("MDP.1a", 2), ("MDP.2", 25), ("MDP.3ai", 91), ("MDP.3aii", 8), ("MDP.3b", 4), ("MDP.3d", 2), ("MDP.4", 2), ("MDP.4a", 8), ("MDP.4b", 4), ("MDP.5", 8), ("MDP.5a", 5), ("MDP.6", 3), ("MDP.6a", 21), ("MDP.7", 1), ("MDP.8", 1), ("MDP.9a", 38), ("MDP.9b", 13), ("MDP.9d", 15), ("MDP.9ei", 6), ("MDP.9eii", 4), ("MDP.9f", 1)]
)]
#[case::nat(
    "nat", "nat.gds.gz", "10_5_NAT",
    &[("NAT.1", 2), ("NAT.10", 1), ("NAT.11", 1), ("NAT.12", 2), ("NAT.2", 4), ("NAT.3", 3), ("NAT.4", 24), ("NAT.5", 12), ("NAT.6", 7), ("NAT.7", 2), ("NAT.8", 5), ("NAT.9", 4)]
)]
#[case::ldnmos(
    "ldnmos", "ldnmos.gds.gz", "10_12_1_MDN",
    &[("MDN.1", 49), ("MDN.10a", 64), ("MDN.10b", 4), ("MDN.10c", 17), ("MDN.10ei", 3), ("MDN.10eii", 2), ("MDN.10f", 6), ("MDN.11", 112), ("MDN.12", 19), ("MDN.13a", 8), ("MDN.13b", 9), ("MDN.13c", 6), ("MDN.13d", 18), ("MDN.14", 30), ("MDN.15a", 44), ("MDN.15b", 2), ("MDN.17", 74), ("MDN.2a", 24), ("MDN.2b", 27), ("MDN.3a", 10), ("MDN.3b", 6), ("MDN.4a", 29), ("MDN.4b", 16), ("MDN.5ai", 31), ("MDN.5aii", 4), ("MDN.5b", 8), ("MDN.6", 14), ("MDN.6a", 6), ("MDN.7", 81), ("MDN.7a", 274), ("MDN.8a", 10), ("MDN.8b", 14), ("MDN.9", 7)]
)]
#[case::lvpwell(
    "lvpwell", "lvpwell.gds.gz", "7_3_LVPWELL",
    &[("LPW.11", 15), ("LPW.12", 8), ("LPW.1_LV", 13), ("LPW.1_MV", 26), ("LPW.2a_LV", 13), ("LPW.2a_MV", 26), ("LPW.2b_LV", 9), ("LPW.2b_MV", 18), ("LPW.3", 90), ("LPW.5", 4)]
)]
#[case::efuse(
    "efuse", "efuse.gds.gz", "10_11_EFUSE",
    &[("EF.01", 38), ("EF.02", 180), ("EF.03", 159), ("EF.04a", 41), ("EF.04b", 75), ("EF.04c", 23), ("EF.04d", 16), ("EF.05", 38), ("EF.06", 366), ("EF.07", 37), ("EF.08", 172), ("EF.09", 30), ("EF.10", 33), ("EF.11", 5), ("EF.12", 21), ("EF.13", 5), ("EF.14", 6), ("EF.15", 12), ("EF.16a", 74), ("EF.16b", 52), ("EF.17", 6), ("EF.18", 46), ("EF.19", 21), ("EF.20", 55), ("EF.21", 122), ("EF.22a", 62), ("EF.22b", 33)]
)]
#[case::hres(
    "hres", "hres.gds.gz", "10_3_HRES",
    &[("HRES.1", 7), ("HRES.10", 24), ("HRES.12a", 37), ("HRES.12b", 1), ("HRES.2", 62), ("HRES.3", 8), ("HRES.4", 38), ("HRES.5", 7), ("HRES.6", 7), ("HRES.7", 39), ("HRES.8", 27), ("HRES.9", 9)]
)]
#[case::dualgate(
    "dualgate", "dualgate.gds.gz", "7_6_Dualgate",
    &[("DV.1", 5), ("DV.2", 3), ("DV.3", 4), ("DV.5", 13), ("DV.6", 5), ("DV.7", 1), ("DV.8", 9), ("DV.9", 1)]
)]
#[case::sram_3p3(
    "sram_3p3", "sram_3p3.gds.gz", "sram_3p3",
    &[("S.CO.3_LV", 7), ("S.CO.4_LV", 7), ("S.CO.6_ii_LV", 8), ("S.DF.16_LV", 6), ("S.DF.4c_LV", 7), ("S.M1.1_LV", 32)]
)]
#[case::drc_bjt(
    "drc_bjt", "drc_bjt.gds.gz", "DRC_BJT",
    &[("BJT.1", 1), ("BJT.2", 2), ("BJT.3", 3)]
)]
#[case::nwell(
    "nwell", "nwell.gds.gz", "7_4_NWELL",
    &[("NW.1a_LV", 77), ("NW.1a_MV", 114), ("NW.1b_LV", 2), ("NW.1b_MV", 4), ("NW.2a_LV", 9), ("NW.2a_MV", 18), ("NW.2b_LV", 13), ("NW.2b_MV", 26), ("NW.3", 15), ("NW.4", 15), ("NW.5_LV", 15), ("NW.5_MV", 18), ("NW.6", 13)]
)]
#[case::poly2(
    "poly2", "poly2.gds.gz", "7_7_Poly2",
    &[("PL.11", 6), ("PL.12", 5), ("PL.1_LV", 28), ("PL.1_MV", 27), ("PL.1a_LV", 10), ("PL.1a_MV", 10), ("PL.2_LV", 99), ("PL.2_MV", 387), ("PL.3a", 49), ("PL.4_LV", 4), ("PL.4_MV", 4), ("PL.5a_LV", 10), ("PL.5a_MV", 10), ("PL.5b_LV", 10), ("PL.5b_MV", 10), ("PL.6", 718), ("PL.7_LV", 40), ("PL.7_MV", 88), ("PL.9", 11)]
)]
#[case::dnwell(
    "dnwell", "dnwell.gds.gz", "7_2_DNWELL",
    &[("DN.1", 466), ("DN.2a", 13), ("DN.2b", 33), ("DN.3", 150)]
)]
#[case::pres(
    "pres", "pres.gds.gz", "10_1_PRES",
    &[("PRES.1", 51), ("PRES.2", 6), ("PRES.3", 9), ("PRES.4", 6), ("PRES.5", 9), ("PRES.6", 41), ("PRES.7", 8), ("PRES.9a", 5), ("PRES.9b", 1)]
)]
#[case::comp(
    "comp", "comp.gds.gz", "7_5_DF",
    &[("DF.10", 5), ("DF.11", 9), ("DF.12", 74), ("DF.13_LV", 45), ("DF.13_MV", 45), ("DF.14_LV", 41), ("DF.14_MV", 41), ("DF.16_LV", 6), ("DF.16_MV", 6), ("DF.17_LV", 10), ("DF.17_MV", 10), ("DF.18", 6), ("DF.19_LV", 6), ("DF.19_MV", 6), ("DF.1a_LV", 105), ("DF.1a_MV", 170), ("DF.1c", 10), ("DF.2a_LV", 4), ("DF.2a_MV", 4), ("DF.2b", 2), ("DF.3a_LV", 29), ("DF.3a_MV", 28), ("DF.3b", 16), ("DF.3c_LV", 7), ("DF.3c_MV", 11), ("DF.4a_LV", 12), ("DF.4a_MV", 6), ("DF.4b_LV", 9), ("DF.4b_MV", 9), ("DF.4c_LV", 9), ("DF.4c_MV", 7), ("DF.4d_LV", 7), ("DF.4d_MV", 7), ("DF.4e_LV", 7), ("DF.4e_MV", 7), ("DF.5_LV", 7), ("DF.5_MV", 7), ("DF.6_LV", 4), ("DF.6_MV", 4), ("DF.7_LV", 6), ("DF.7_MV", 6), ("DF.8_LV", 7), ("DF.8_MV", 7), ("DF.9", 215)]
)]
#[case::sab(
    "sab", "sab.gds.gz", "7_10_SB",
    &[("SB.1", 20), ("SB.10", 61), ("SB.11", 1), ("SB.12", 1), ("SB.13", 132), ("SB.14a", 6), ("SB.14b", 6), ("SB.15a", 13), ("SB.15b", 5), ("SB.16", 6), ("SB.2", 9), ("SB.3", 9), ("SB.4", 6), ("SB.5a", 9), ("SB.5b", 7), ("SB.6", 27), ("SB.7", 17), ("SB.8", 3), ("SB.9", 36)]
)]
#[case::ymtp_mk(
    "ymtp_mk", "ymtp_mk.gds.gz", "10_13_YMTP",
    &[("Y.DF.16_LV", 6), ("Y.DF.16_MV", 6), ("Y.DF.6_MV", 16), ("Y.NW.2b_LV", 14), ("Y.NW.2b_MV", 28), ("Y.PL.1_LV", 100), ("Y.PL.1_MV", 119), ("Y.PL.2_LV", 68), ("Y.PL.2_MV", 164), ("Y.PL.4_MV", 7), ("Y.PL.5a_LV", 8), ("Y.PL.5a_MV", 6), ("Y.PL.5b_LV", 8), ("Y.PL.5b_MV", 6)]
)]
#[case::contact(
    "contact", "contact.gds.gz", "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump",
    &[("CO.1", 104), ("CO.10", 2), ("CO.11", 145), ("CO.2a", 8), ("CO.2b", 6), ("CO.3", 14), ("CO.4", 9), ("CO.5a", 2), ("CO.5b", 4), ("CO.6", 98), ("CO.6a", 25), ("CO.6b", 30), ("CO.7", 2), ("CO.8", 2), ("CO.9", 6)]
)]
#[case::sram_5p0(
    "sram_5p0", "sram_5p0.gds.gz", "sram_5p0",
    &[("S.CO.4_MV", 9), ("S.DF.16_MV", 6), ("S.DF.4c_MV", 9), ("S.DF.6_MV", 16), ("S.DF.7_MV", 6), ("S.DF.8_MV", 9), ("S.PL.5a_MV", 6), ("S.PL.5b_MV", 4)]
)]
#[case::lres(
    "lres", "lres.gds.gz", "10_2_LRES",
    &[("LRES.1", 50), ("LRES.2", 6), ("LRES.3", 9), ("LRES.4", 6), ("LRES.5", 9), ("LRES.6", 41), ("LRES.7", 8), ("LRES.9a", 9), ("LRES.9b", 1)]
)]
#[case::mim_b(
    "mim_b", "mim_b.gds.gz", "10_4_2_MIM_OptionB",
    &[("MIMTM.1", 4), ("MIMTM.10", 3), ("MIMTM.11", 2), ("MIMTM.2", 9), ("MIMTM.3", 62), ("MIMTM.4", 12), ("MIMTM.5", 8), ("MIMTM.6", 6), ("MIMTM.7", 2490), ("MIMTM.8a", 60), ("MIMTM.8b", 1), ("MIMTM.9", 6)]
)]
#[case::otp_mk(
    "otp_mk", "otp_mk.gds.gz", "10_10_OTP",
    &[("O.CO.7", 2), ("O.DF.3a", 9), ("O.DF.6", 24), ("O.DF.9", 4), ("O.PL.2", 74), ("O.PL.3a", 14), ("O.PL.4", 14), ("O.PL.ORT", 367), ("O.SB.11", 1), ("O.SB.13_LV", 53), ("O.SB.13_MV", 1), ("O.SB.2", 7), ("O.SB.3", 6), ("O.SB.4", 3), ("O.SB.5b_LV", 6), ("O.SB.9", 2)]
)]
#[case::mcell(
    "mcell", "mcell.gds.gz", "7_17_Mcell",
    &[("MC.1", 38), ("MC.2", 16), ("MC.3", 13), ("MC.4", 6)]
)]
#[case::nplus(
    "nplus", "nplus.gds.gz", "7_10_Nplus",
    &[("NP.1", 395), ("NP.10", 5), ("NP.11", 10), ("NP.12", 1), ("NP.2", 86), ("NP.3a", 9), ("NP.3bi", 10), ("NP.3bii", 4), ("NP.3ci", 15), ("NP.3cii", 4), ("NP.3d", 2), ("NP.3e", 2), ("NP.4a", 4), ("NP.4b", 1), ("NP.5a", 8), ("NP.5b", 66), ("NP.5ci", 8), ("NP.5cii", 3), ("NP.5di", 7), ("NP.5dii", 9), ("NP.6", 72), ("NP.7", 6), ("NP.8a", 121), ("NP.8b", 2), ("NP.9", 6)]
)]
#[case::metaltop(
    "metaltop", "metaltop.gds.gz", "metaltop",
    &[("MT.1", 386), ("MT.2a", 16), ("MT.2b", 11), ("MT.4", 85)]
)]
#[case::pplus(
    "pplus", "pplus.gds.gz", "7_11_Pplus",
    &[("PP.1", 395), ("PP.10", 5), ("PP.11", 8), ("PP.12", 1), ("PP.2", 86), ("PP.3a", 21), ("PP.3bi", 4), ("PP.3bii", 11), ("PP.3ci", 4), ("PP.3cii", 10), ("PP.3d", 2), ("PP.3e", 2), ("PP.4a", 4), ("PP.4b", 1), ("PP.5a", 8), ("PP.5b", 71), ("PP.5ci", 6), ("PP.5cii", 9), ("PP.5di", 8), ("PP.5dii", 33), ("PP.6", 72), ("PP.7", 6), ("PP.8a", 121), ("PP.8b", 2), ("PP.9", 6)]
)]
#[case::metal1(
    "metal", "metal1.gds.gz", "metal1",
    &[("M1.1", 333), ("M1.2a", 16), ("M1.2b", 11), ("M1.3", 85)]
)]
#[case::metal2(
    "metal", "metal2.gds.gz", "metal2",
    &[("M2.1", 338), ("M2.2a", 16), ("M2.2b", 11), ("M2.3", 85)]
)]
#[case::metal3(
    "metal", "metal3.gds.gz", "metal3",
    &[("M3.1", 338), ("M3.2a", 16), ("M3.2b", 11), ("M3.3", 85)]
)]
#[case::metal4(
    "metal", "metal4.gds.gz", "metal4",
    &[("M4.1", 338), ("M4.2a", 16), ("M4.2b", 11), ("M4.3", 85)]
)]
#[case::esd(
    "esd", "esd.gds.gz", "7_11_ESD",
    &[("ESD.1", 2), ("ESD.10", 2), ("ESD.2", 2), ("ESD.3a", 1), ("ESD.3b", 7), ("ESD.4a", 2), ("ESD.4b", 2), ("ESD.5a", 2), ("ESD.5b", 2), ("ESD.6", 2), ("ESD.7", 7), ("ESD.8", 3), ("ESD.9", 1), ("ESD.pl", 2)]
)]
#[case::lvs_bjt(
    "lvs_bjt", "lvs_bjt.gds.gz", "LVS_BJT",
    &[("LVS_BJT.1", 2)]
)]
#[case::via1(
    "via", "via1.gds.gz", "7_14_VIA",
    &[("V1.1", 4), ("V1.2a", 3), ("V1.2b", 6), ("V1.3a", 3), ("V1.3c", 2), ("V1.3d", 15), ("V1.4a", 6), ("V1.4b", 2), ("V1.4c", 15)]
)]
fn static_fixture(
    #[case] deck: &str,
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: &[(&str, usize)],
) {
    let want: Vec<(String, usize)> = expected
        .iter()
        .map(|(id, n)| ((*id).to_string(), *n))
        .collect();
    assert_eq!(counts(deck, gds, topcell), want, "{deck} on {gds}");
}

/// Every rule the deck declares must appear in the run above. A rule that resolves to an
/// empty derived layer produces no violations and no error, so without this a typo in a
/// layer name looks exactly like a clean design.
#[rstest]
#[case::nwell("nwell", "nwell.gds.gz", "7_4_NWELL")]
#[case::comp("comp", "comp.gds.gz", "7_5_DF")]
#[case::mcell("mcell", "mcell.gds.gz", "7_17_Mcell")]
#[case::drc_bjt("drc_bjt", "drc_bjt.gds.gz", "DRC_BJT")]
#[case::lvs_bjt("lvs_bjt", "lvs_bjt.gds.gz", "LVS_BJT")]
#[case::dummy_exclude("dummy_exclude", "dummy_exclude.gds.gz", "10_8_DE")]
#[case::dualgate("dualgate", "dualgate.gds.gz", "7_6_Dualgate")]
#[case::nat("nat", "nat.gds.gz", "10_5_NAT")]
#[case::metaltop("metaltop", "metaltop.gds.gz", "metaltop")]
#[case::lvpwell("lvpwell", "lvpwell.gds.gz", "7_3_LVPWELL")]
#[case::esd("esd", "esd.gds.gz", "7_11_ESD")]
#[case::contact("contact", "contact.gds.gz", "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump")]
#[case::sram_3p3("sram_3p3", "sram_3p3.gds.gz", "sram_3p3")]
#[case::poly2("poly2", "poly2.gds.gz", "7_7_Poly2")]
#[case::sab("sab", "sab.gds.gz", "7_10_SB")]
#[case::nplus("nplus", "nplus.gds.gz", "7_10_Nplus")]
#[case::pplus("pplus", "pplus.gds.gz", "7_11_Pplus")]
#[case::ldnmos("ldnmos", "ldnmos.gds.gz", "10_12_1_MDN")]
#[case::ldpmos("ldpmos", "ldpmos.gds.gz", "10_12_2_MDP")]
#[case::lres("lres", "lres.gds.gz", "10_2_LRES")]
#[case::pres("pres", "pres.gds.gz", "10_1_PRES")]
#[case::hres("hres", "hres.gds.gz", "10_3_HRES")]
#[case::otp_mk("otp_mk", "otp_mk.gds.gz", "10_10_OTP")]
#[case::ymtp_mk("ymtp_mk", "ymtp_mk.gds.gz", "10_13_YMTP")]
#[case::sram_5p0("sram_5p0", "sram_5p0.gds.gz", "sram_5p0")]
#[case::efuse("efuse", "efuse.gds.gz", "10_11_EFUSE")]
#[case::mim_b("mim_b", "mim_b.gds.gz", "10_4_2_MIM_OptionB")]
fn every_rule_in_the_deck_fires(#[case] deck: &str, #[case] gds: &str, #[case] topcell: &str) {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).unwrap();
    let declared: std::collections::BTreeSet<String> = pdk
        .load_deck(deck)
        .unwrap()
        .iter()
        .map(|r| r.id.clone())
        .collect();
    let fired: std::collections::BTreeSet<String> = counts(deck, gds, topcell)
        .into_iter()
        .map(|(r, _)| r)
        .collect();
    let silent: Vec<&String> = declared.difference(&fired).collect();
    assert!(
        silent.is_empty(),
        "deck '{deck}' declares rules that never fire on {gds}: {silent:?}"
    );
}

// --- Generated patterns ---
//
// The `offgrid` and `acute` decks are checked against patterns we draw ourselves rather
// than against a foundry case, and each rule gets two: a **good** one that must produce
// nothing and a **bad** one that must produce exactly the violations it was drawn to
// contain. That is a stronger statement than a count table can make. A foundry case only
// ever says "we found about as many markers as they did", and the two engines disagree on
// how many markers one violation is worth — so a rule that quietly over-reports can sit
// inside that noise indefinitely. Against a pattern of our own the good half is a hard
// zero, and a false positive has nowhere to hide.
//
// Sources: `gen/gf180mcuD/offgrid.rs` and `gen/gf180mcuD/acute.rs`.

/// Deck, fixture directory, and how many violations that deck's bad pattern contains.
/// Off-grid reports per vertex and the shifted edge has two; the acute pattern's triangle
/// has exactly one edge off the 45° lattice.
const GENERATED_DECKS: &[(&str, usize)] = &[("offgrid", 2), ("acute", 1)];

/// Every rule id in a deck, read from the deck itself so a rule without a fixture is a
/// failure here rather than a silent gap.
fn rule_ids(deck: &str) -> Vec<String> {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).expect("PDK loads");
    let rules = pdk.load_deck(deck).expect("deck loads");
    assert!(!rules.is_empty(), "deck '{deck}' is empty");
    rules.iter().map(|r| r.id.clone()).collect()
}

fn run_generated(deck: &str, id: &str, polarity: &str) -> Vec<(String, usize)> {
    counts_at(
        deck,
        &format!("{GENERATED}/{deck}/{id}.{polarity}.gds.gz"),
        "TOP",
    )
}

/// Decks where every rule has a drawn good/bad pair, and the pair means the same thing
/// for all of them: the good half is silent, and the bad half fires that rule and only
/// that rule.
///
/// This is the contract to write new patterns against.  The older decks above predate it
/// and pin exact counts through tests of their own, which says more but has to be written
/// per deck; this one costs a generator and a line here.
const PATTERN_DECKS: &[&str] = &[
    "comp",
    "contact",
    "drc_bjt",
    "cup",
    "density",
    "dnwell",
    "dualgate",
    "dummy_comp",
    "dummy_exclude",
    "dummy_metal",
    "dummy_poly2",
    "efuse",
    "esd",
    "guard_ring",
    "hres",
    "ldnmos",
    "ldpmos",
    "lres",
    "lvs_bjt",
    "lvpwell",
    "mcell",
    "nat",
    "nplus",
    "nwell",
    "otp_mk",
    "poly2",
    "sab",
    "sram_3p3",
    "sram_5p0",
    "metal",
    "metaltop",
    "mim_b",
    "mslot",
];

#[test]
fn pattern_good_halves_are_silent() {
    for deck in PATTERN_DECKS {
        for id in dedup(rule_ids(deck)) {
            let got = run_generated(deck, &id, "good");
            assert!(
                got.is_empty(),
                "{deck}: {id} good pattern is not clean: {got:?}"
            );
        }
    }
}

#[test]
fn pattern_bad_halves_fire_their_own_rule() {
    for deck in PATTERN_DECKS {
        for id in dedup(rule_ids(deck)) {
            let got = run_generated(deck, &id, "bad");
            let fired: Vec<&String> = got.iter().map(|(r, _)| r).collect();
            assert!(
                fired.contains(&&id),
                "{deck}: {id} bad pattern did not fire it: {got:?}"
            );
            // Two deck entries with the same check on the same layers at the same value
            // are one measurement under two names - GF180 floors the top metal's density
            // as both M5.4 and MT.3 - and no pattern can separate them.  Read from the
            // deck rather than listed here, so the allowance cannot outlive its reason.
            let twins = same_measurement(deck, &id);
            let others: Vec<&&String> = fired
                .iter()
                .filter(|r| **r != &id && !twins.contains(**r))
                .filter(|r| {
                    !COINCIDENT
                        .iter()
                        .any(|&(d, of, also)| d == *deck && of == id && also == r.as_str())
                })
                .collect();
            assert!(
                others.is_empty(),
                "{deck}: {id} bad pattern also fired {others:?} - a pattern should isolate \
                 its own rule"
            );
        }
    }
}

/// Rule ids in `deck` that measure exactly what `id` measures - same check, same layers,
/// same value - and so must fire wherever it does.
/// Rules a fixture may fire besides its own, because the deck makes the two coincide and
/// no geometry can tell them apart.  Twins that are literally the same check on the same
/// layers are found from the deck by [`same_measurement`]; these are the ones only the
/// rule text explains, so each carries its reason.
const COINCIDENT: &[(&str, &str, &str)] = &[
    // Upstream writes SB.4 as `separation(contact, 0.15).or(sab.and(contact))` and SB.8
    // as `contact.and(sab)`.  A contact *on* the block is the second half of one and the
    // whole of the other, so it breaks both by construction.
    ("sab", "SB.8", "SB.4"),
    // MIMTM.8b caps one MIM's area and MIMTM.11 the total sharing a bottom plate.  A
    // single MIM over the cap is over it both ways, since it is its own plate's whole
    // total - the rule only adds anything where several share one.
    ("mim_b", "MIMTM.8b", "MIMTM.11"),
    // The LDNMOS gate's width is the channel plus the drift it overlaps, and MDN.11 fixes
    // that overlap at 0.4.  So a channel under MDN.3a's 0.6 leaves a gate under MDN.10a's
    // 1.2 by arithmetic, and no geometry breaks the one without the other.
    ("ldnmos", "MDN.3a", "MDN.10a"),
    // The P side of MDN.3a and MDN.10a, for the same arithmetic: MDP.10 fixes the drift's
    // overlap onto the channel at 0.4, so a channel under MDP.1's 0.6 leaves a gate under
    // MDP.9a's 1.2.
    ("ldpmos", "MDP.1", "MDP.9a"),
    // A source shared between two fingers is what MDN.13c reports, and it is also the
    // butted-tap arrangement MDN.13d joins into its own output, so the one geometry is
    // both by construction.
    ("ldnmos", "MDN.13c", "MDN.13d"),
    // MDN.13b wants the fingers to alternate source and drain.  A finger with source on
    // both sides of it, which is what MDN.13c reports, is the arrangement that is not
    // alternating - so the one geometry answers to both.
    ("ldnmos", "MDN.13c", "MDN.13b"),
    // The P side, for the same reason.
    ("ldpmos", "MDP.13c", "MDP.13b"),
    // GR.3 wants the implant over the ring's active and GR.6 wants that active 16 µm
    // wide.  Uncovering any of it is what GR.3 forbids and what narrows what GR.6
    // measures, so no geometry breaks one alone.
    ("guard_ring", "GR.3", "GR.6"),
    // MDN.4b caps the transistor's width and MDN.13a the length of the body that carries
    // it, and on this device they are the same measurement seen from two sides: the
    // foundry's own report draws MDN.13a on a body's long walls and MDN.4b on the two
    // ends between them, the same sixteen markers under both ids.
    ("ldnmos", "MDN.4b", "MDN.13a"),
    ("ldnmos", "MDN.13a", "MDN.4b"),
    // MDN.7 forbids device material under a Dualgate outside the LDMOS marker, and every
    // layer that counts as device material there is defined against the drift.  MDN.7a's
    // second half forbids drift under a Dualgate outside the marker outright, so nothing
    // can break the first without breaking the second.
    ("ldnmos", "MDN.7", "MDN.7a"),
    // NP.3d forbids an N+ active over a P+ active and NP.3e an N+ marker over a P+
    // active.  An N+ active *is* comp under the marker, so both expand to the same
    // intersection of comp, nplus and pplus and no drawing separates them.
    ("nplus", "NP.3d", "NP.3e"),
    ("nplus", "NP.3e", "NP.3d"),
    // The eFuse deck does not bound its device's dimensions, it fixes them, and the
    // numbers are not independent: 1.84 + 1.26 + 2.43 is the 5.53 µm EF.21 asks of the
    // poly end to end, and the shoulders EF.22a and EF.22b pin are what is left when the
    // link's width is taken from each pad's.  So no drawing changes one of them alone.
    ("efuse", "EF.03", "EF.21"),
    ("efuse", "EF.07", "EF.21"),
    ("efuse", "EF.09", "EF.21"),
    ("efuse", "EF.21", "EF.03"),
    ("efuse", "EF.06", "EF.22a"),
    ("efuse", "EF.08", "EF.22b"),
    ("efuse", "EF.22a", "EF.22b"),
    ("efuse", "EF.22b", "EF.22a"),
    // The shoulders are the only pair a drawing can reach through the link's own width,
    // and EF.02 fixes that width, so narrowing a shoulder is widening the link past it.
    ("efuse", "EF.22a", "EF.02"),
    ("efuse", "EF.22b", "EF.02"),
    // A shape that is no longer a rectangle has an edge that is no longer its old length,
    // and every edge of this device is pinned by something.
    ("efuse", "EF.04b", "EF.03"),
    // And a shape that is no longer a rectangle has no one width, so EF.02 - which fixes
    // the link's - reads it as the wrong one wherever the bend put it.
    ("efuse", "EF.04b", "EF.02"),
    ("efuse", "EF.04c", "EF.06"),
    ("efuse", "EF.04c", "EF.07"),
    ("efuse", "EF.04d", "EF.08"),
    ("efuse", "EF.04d", "EF.09"),
    // A contact *on* the link is also nearer to it than the 0.155 µm EF.12 allows.
    ("efuse", "EF.15", "EF.12"),
    // Metal is in the list of things EF.18 keeps off the link as well as EF.19's.
    ("efuse", "EF.19", "EF.18"),
    // A marker that overlaps an active without covering it has, by saying so, failed to
    // enclose it by the 0.24 µm DV.6 asks.
    ("dualgate", "DV.7", "DV.6"),
];

fn same_measurement(deck: &str, id: &str) -> Vec<String> {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).expect("PDK loads");
    let rules = pdk.load_deck(deck).expect("deck loads");
    let key = |r: &gdscheck::pdk::RuleDefinition| {
        (
            r.check.clone(),
            r.layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>(),
            r.value.to_bits(),
        )
    };
    let want: Vec<_> = rules.iter().filter(|r| r.id == id).map(key).collect();
    rules
        .iter()
        .filter(|r| r.id != id && want.contains(&key(r)))
        .map(|r| r.id.clone())
        .collect()
}

fn dedup(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup(); // a rule split over a space and a notch entry shares one id
    ids
}

/// The half that matters most: legal geometry, including a 45° chamfer, produces nothing.
#[test]
fn generated_good_patterns_are_clean() {
    for (deck, _) in GENERATED_DECKS {
        for id in rule_ids(deck) {
            let got = run_generated(deck, &id, "good");
            assert!(
                got.is_empty(),
                "{deck}: {id} fired on its good pattern: {got:?}"
            );
        }
    }
}

/// And the other half: a real violation is still caught, on every layer the deck covers.
#[test]
fn generated_bad_patterns_fire() {
    for (deck, expected) in GENERATED_DECKS {
        for id in rule_ids(deck) {
            let got = run_generated(deck, &id, "bad");
            assert_eq!(
                got,
                vec![(id.clone(), *expected)],
                "{deck}: {id} should fire {expected} time(s) on its bad pattern"
            );
        }
    }
}

// --- YMTP marker, generated ---
//
// One case per rule id, drawn inside a YMTP marker, with Dualgate over it where the rule
// is the 5 V flavour, since that is what makes a marker MV rather than LV. Y.PL.5a and
// Y.PL.5b are the same measurement upstream, so one drawing is written under both ids and
// each expects both to fire.

/// Case name, and what its bad half must produce. `None` would mark a case whose
/// violation this engine does not find; there are none at present.
type YmtpCase = (&'static str, Option<&'static [(&'static str, usize)]>);

const YMTP_CASES: &[YmtpCase] = &[
    ("Y.NW.2b_LV", Some(&[("Y.NW.2b_LV", 1)])),
    ("Y.NW.2b_MV", Some(&[("Y.NW.2b_MV", 1)])),
    ("Y.DF.16_LV", Some(&[("Y.DF.16_LV", 1)])),
    ("Y.DF.16_MV", Some(&[("Y.DF.16_MV", 1)])),
    ("Y.PL.1_LV", Some(&[("Y.PL.1_LV", 2)])),
    ("Y.PL.1_MV", Some(&[("Y.PL.1_MV", 1)])),
    ("Y.PL.2_LV", Some(&[("Y.PL.2_LV", 2)])),
    ("Y.PL.2_MV", Some(&[("Y.PL.2_MV", 2)])),
    ("Y.PL.5a_LV", Some(&[("Y.PL.5a_LV", 1), ("Y.PL.5b_LV", 1)])),
    ("Y.PL.5b_LV", Some(&[("Y.PL.5a_LV", 1), ("Y.PL.5b_LV", 1)])),
    ("Y.PL.5a_MV", Some(&[("Y.PL.5a_MV", 1), ("Y.PL.5b_MV", 1)])),
    ("Y.PL.5b_MV", Some(&[("Y.PL.5a_MV", 1), ("Y.PL.5b_MV", 1)])),
    // A gate poly crossing its COMP, measured at right angles to the crossing: the
    // source/drain overhang for Y.DF.6_MV, the poly end cap for Y.PL.4_MV, each one wall
    // at either end of the crossing. These two were the fixtures that found
    // `polys_interact`'s missing edge-crossing test.
    ("Y.DF.6_MV", Some(&[("Y.DF.6_MV", 2)])),
    ("Y.PL.4_MV", Some(&[("Y.PL.4_MV", 2)])),
];

#[test]
fn ymtp_good_patterns_are_clean() {
    for (name, _) in YMTP_CASES {
        let got = counts_at(
            "ymtp_mk",
            &format!("{GENERATED}/ymtp_mk/{name}.good.gds.gz"),
            "TOP",
        );
        assert!(got.is_empty(), "{name} fired on its good pattern: {got:?}");
    }
}

#[test]
fn ymtp_bad_patterns_fire() {
    for (name, expected) in YMTP_CASES {
        let got = counts_at(
            "ymtp_mk",
            &format!("{GENERATED}/ymtp_mk/{name}.bad.gds.gz"),
            "TOP",
        );
        match expected {
            Some(want) => {
                let want: Vec<(String, usize)> =
                    want.iter().map(|(r, n)| ((*r).to_string(), *n)).collect();
                assert_eq!(got, want, "{name} bad pattern");
            }
            None => assert!(
                got.is_empty(),
                "{name} now fires ({got:?}) — the gap this case recorded appears to be \
                 fixed, so give it its real expectation"
            ),
        }
    }
}

// --- Gate poly, generated ---

/// `PL.6` is the `no_corner` rule: poly may not turn a right angle over active area.
///
/// The good pattern says that from three sides at once — a right-angle bend clear of any
/// COMP, a 45° turn on COMP, and a right-angle bend on COMP but inside a YMTP marker,
/// which the rule exempts. A check that had collapsed to "any bend", or to "any bend on
/// COMP", fails here rather than passing.
#[test]
fn pl6_corner_patterns() {
    let good = counts_at(
        "poly2",
        &format!("{GENERATED}/poly2/PL.6.good.gds.gz"),
        "TOP",
    );
    assert!(good.is_empty(), "PL.6 fired on its good pattern: {good:?}");

    let bad = counts_at(
        "poly2",
        &format!("{GENERATED}/poly2/PL.6.bad.gds.gz"),
        "TOP",
    );
    assert_eq!(
        bad,
        vec![("PL.6".to_string(), 2)],
        "the L's two elbow corners are inside the COMP; its other four are not"
    );
}

// --- P+ poly resistor, generated ---
//
// The deck whose foundry case most needs a drawn counterpart: it reports far more markers
// per violation than we do — 219 against 39 on PRES.6 alone — so a count that is merely
// close reads as agreement and a real miss hides inside the difference. Here the clean
// half is a hard zero and the bad half has a number we chose.

/// How many markers each bad pattern is drawn to produce. One each, except where the
/// check reports per wall or the perturbation necessarily moves two things:
///
/// * `PRES.1` reports a narrow body at both its walls.
/// * `PRES.6` pulls the SAB in on both long sides of the body, two walls apart.
/// * `PRES.7` moves both contact heads, since the cell places them symmetrically.
fn pres_bad_expect(id: &str) -> usize {
    match id {
        "PRES.1" | "PRES.6" | "PRES.7" => 2,
        _ => 1,
    }
}

#[test]
fn pres_patterns_cover_every_rule() {
    let mut ids: Vec<String> = rule_ids("pres");
    ids.sort();
    ids.dedup(); // PRES.3 and PRES.7 are each a spacing and a must-not-touch rule
    assert_eq!(ids.len(), 9, "every rule in the section");

    for id in ids {
        let good = run_generated("pres", &id, "good");
        assert!(
            good.is_empty(),
            "pres: {id} fired on its good pattern: {good:?}"
        );
        let bad = run_generated("pres", &id, "bad");
        assert_eq!(
            bad,
            vec![(id.clone(), pres_bad_expect(&id))],
            "pres: {id}'s bad pattern should trip {id} and nothing else"
        );
    }
}

// --- P+ implant, generated ---
//
// The deck that most needed drawn patterns. Its foundry case reports 222 PP.1 markers
// where we report 392, so no count comparison on it means anything, and half the rules
// come in near/far pairs split by a 0.429 um band around a well — a classifier that has
// already caused two wrong measurements in this port. Each fixture sits deliberately on
// one side of that band and its clean half is a hard zero.

/// What each bad pattern is drawn to contain. One marker each, except:
///
/// * `PP.1` reports a narrow shape at both its walls.
/// * `PP.5a` pulls the implant in on both sides of the gate, two walls apart.
/// * `PP.5b` and the `PP.5c`/`PP.5d` pairs measure an extension that is short on every
///   side of the COMP at once, so a square reports four — three for `PP.5dii`, whose
///   fourth side faces the N-well that puts it in the near bucket.
/// * `PP.3d` and `PP.3e` are the same set of geometry — COMP and Nplus and Pplus
///   together — so neither can be drawn without the other.
fn pplus_bad_expect(id: &str) -> Vec<(String, usize)> {
    let one = |i: &str| (i.to_string(), 1);
    match id {
        "PP.1" | "PP.5a" => vec![(id.to_string(), 2)],
        "PP.3d" | "PP.3e" => vec![one("PP.3d"), one("PP.3e")],
        "PP.5b" | "PP.5ci" | "PP.5cii" | "PP.5di" => vec![(id.to_string(), 4)],
        "PP.5dii" => vec![("PP.5dii".to_string(), 3)],
        _ => vec![one(id)],
    }
}

#[test]
fn pplus_patterns_cover_every_rule() {
    let mut ids: Vec<String> = rule_ids("pplus");
    ids.sort();
    ids.dedup(); // PP.2 is a space and a notch rule under one id
    assert_eq!(ids.len(), 25, "every rule in the section");

    for id in ids {
        let good = run_generated("pplus", &id, "good");
        assert!(
            good.is_empty(),
            "pplus: {id} fired on its good pattern: {good:?}"
        );
        let mut bad = run_generated("pplus", &id, "bad");
        bad.sort();
        let mut want = pplus_bad_expect(&id);
        want.sort();
        assert_eq!(bad, want, "pplus: {id}'s bad pattern");
    }
}

/// The touching case `PL.5a`/`PL.5b` lose nine markers each to, reduced from the site at
/// (-3830.478, 881.684) in the foundry case: a COMP corner exactly on the gate poly's
/// boundary, the two walls collinear and meeting at that one point.
///
/// KLayout reports two separations of zero there, and so do we: one under each of the two
/// ids that share the rule. It took two changes — reading a point contact between two
/// layers as a zero gap, and making `overlapping` mean positive shared *area*, since a
/// boundary point counted as area kept this very pair out of the spacing scan that is
/// about it.
///
/// It is worth being exact about what that bought on the deck it came from, because it is
/// less than the fixture suggests. `PL.5a` went from 4 of 10 logical violations to 6 with
/// the first change and stayed at 6 through the second; the four still missing are
/// 45°-shaped, not this shape. The second change paid elsewhere — ten decks, 34 logical
/// violations — so the reduced case is a faithful minimal reproduction of *a* cause and
/// not of everything `PL.5` misses.
#[test]
fn pl5_touching_corner_is_a_zero_separation() {
    let good = counts_at(
        "poly2",
        &format!("{GENERATED}/poly2/PL.5.good.gds.gz"),
        "TOP",
    );
    assert!(good.is_empty(), "PL.5 good pattern: {good:?}");
    let bad = counts_at(
        "poly2",
        &format!("{GENERATED}/poly2/PL.5.bad.gds.gz"),
        "TOP",
    );
    assert_eq!(
        bad,
        vec![("PL.5a_LV".to_string(), 1), ("PL.5b_LV".to_string(), 1)],
        "the corner is a separation of zero, under both ids that share the rule"
    );
}

// --- OTP marker, generated ---
//
// Two rules, not the deck's sixteen. `otp_mk` has no false positives at either count and
// its misses are all pinch- or 45°-shaped, which no axis-aligned fixture reaches — the
// other fourteen would pin rules already exact against geometry that is not where the
// risk is. These two carry engine surface with almost nothing else behind it: `O.PL.2` is
// a `min_gate_length` whose reference is the channel itself - the poly's width between the
// walls it shares with the gate outline - and `O.SB.11` one of two users of
// `min_overlap`, which had no drawn pattern at all.

#[test]
fn otp_mk_patterns_cover_the_new_relations() {
    for (id, bad_count) in [("O.PL.2", 2), ("O.SB.11", 1)] {
        let good = run_generated("otp_mk", id, "good");
        assert!(
            good.is_empty(),
            "otp_mk: {id} fired on its good pattern: {good:?}"
        );
        let bad = run_generated("otp_mk", id, "bad");
        assert_eq!(
            bad,
            vec![(id.to_string(), bad_count)],
            "otp_mk: {id}'s bad pattern should trip {id} and nothing else"
        );
    }
}

// --- Via, generated ---
//
// Every rule in the deck, at every level. The levels were written by expanding a
// template, which is exactly where a wrong layer slips into one of them — Via3 reaching
// for Metal3 above instead of Metal4 — so generating all four turns that from a silent
// wrong answer into a failing test.

/// The two rules the foundry documents but does not check.
fn is_guidance(suffix: &str) -> bool {
    suffix == "3.3" || suffix == "4.3"
}

/// What the bad half of each pattern is drawn to contain, by rule suffix.
///
/// Mostly the rule itself, once. Two exceptions, both structural rather than sloppy
/// drawing:
///
/// * `V#.1` reports per wall pair, so an oversized via is four.
/// * `V#.3c`/`V#.4b` cannot avoid also tripping `V#.3d`/`V#.4c`. A 0.26 µm via in a
///   track narrow enough to have a line end — under 0.34 µm — has sidewalls under 0.04,
///   so a thin tip always leaves a short side whose neighbours are short too. The two
///   rules genuinely overlap on that geometry; there is no way to draw one without the
///   other.
fn via_bad_expect(level: usize, suffix: &str) -> Vec<(String, usize)> {
    let id = |sfx: &str| (format!("V{level}.{sfx}"), 1);
    match suffix {
        "1" => vec![(format!("V{level}.1"), 4)],
        "3c" => vec![id("3c"), id("3d")],
        "4b" => vec![id("4b"), id("4c")],
        _ => vec![id(suffix)],
    }
}

#[test]
fn via_patterns_cover_every_rule() {
    // The guidance rules live in a deck of their own, and each fixture runs against the
    // deck its rule is in.  V#.3.3 and V#.4.3 ask for 0.12 µm where the rules require
    // 0.01, so they fire on any via not drawn generously - including the *legal* half of
    // patterns whose point is a small margin - which is the same property that keeps
    // them out of `main`, and the reason they are not in the `via` deck at all.
    let mut ids: Vec<String> = rule_ids("via");
    ids.extend(rule_ids("via_recommended"));
    ids.sort();
    ids.dedup(); // V#.2a is a space and a notch rule under one id
    assert_eq!(ids.len(), 44, "eleven rules at each of four levels");

    for id in ids {
        let (level, suffix) = id.split_once('.').expect("V<n>.<rule>");
        let level: usize = level[1..].parse().expect("V<n>");
        let deck = if is_guidance(suffix) {
            "via_recommended"
        } else {
            "via"
        };

        let good = counts_at(deck, &format!("{GENERATED}/via/{id}.good.gds.gz"), "TOP");
        assert!(good.is_empty(), "{id} fired on its good pattern: {good:?}");

        let bad = counts_at(deck, &format!("{GENERATED}/via/{id}.bad.gds.gz"), "TOP");
        let mut want = via_bad_expect(level, suffix);
        want.sort();
        assert_eq!(bad, want, "{id} bad pattern");
    }
}

// --- Hardening (hardening/SPEC.md, the GF180MCU section) ---
//
// Layouts drawn from the manual's edges by someone who has not seen the engine, run
// through gdscheck and the foundry's runset; the expected values are the manual's
// answer, not the engine's, and the reasoning is in hardening/reports/gf180mcuD/.

/// The rule ids one deck reports on a generated fixture, sorted, less the ones to
/// ignore.
fn hardening(deck: &str, gds: &str, topcell: &str, ignore: &[&str]) -> Vec<String> {
    let path = format!("{GENERATED}/{gds}");
    let violations = run_drc(&path, PDK, &[deck], None, topcell, true).expect("DRC run failed");
    let mut ids: Vec<String> = violations
        .into_iter()
        .filter(|v| !ignore.contains(&v.rule_id.as_str()))
        .map(|v| v.rule_id)
        .collect();
    ids.sort();
    ids
}

// --- N-well (hardening/reports/gf180mcuD/nwell.md).  A well is the 5 V kind where
// Dualgate lies over it, and the 3.3 V kind otherwise - a marker abutting a well is not
// over it.  `min_width` reports one marker per wall, two per narrow bar.
#[rstest]
// Five 0.855 wells: bare (LV), under Dualgate (MV), half under it (MV), Dualgate abutting
// its edge (LV), 0.86 under Dualgate (clean).
#[case::nw_1a_h1("nwell/NW.1a.h1.gds.gz", "TOP", vec!["NW.1a_LV", "NW.1a_LV", "NW.1a_LV", "NW.1a_LV", "NW.1a_MV", "NW.1a_MV", "NW.1a_MV", "NW.1a_MV"], vec![])]
// A 0.855 well under V5_XTOR alone has no Dualgate over it: LV.  Under both: MV.
#[case::nw_1a_h2("nwell/NW.1a.h2.gds.gz", "TOP", vec!["NW.1a_LV", "NW.1a_LV", "NW.1a_MV", "NW.1a_MV"], vec![])]
// A 30 µm 0.855 well with Dualgate over its last 4 µm is MV whole; one with Dualgate
// abutting its end is LV.
#[case::nw_1a_h3("nwell/NW.1a.h3.gds.gz", "TOP", vec!["NW.1a_LV", "NW.1a_LV", "NW.1a_MV", "NW.1a_MV"], vec![])]
// 1.995 resistor wells: marker overhanging (fires), marker inside the well (not a
// resistor), NW.7's head-to-head marker (fires), 2.0 (clean), a 3.0 well with a 1.5
// long marker - the marked region is measured as it is drawn, so its 1.5 side fires
// (kept: both tools read it that way, and a 1.5 um well resistor is not a device).
#[case::nw_1b_h1("nwell/NW.1b.h1.gds.gz", "TOP", vec!["NW.1b_LV", "NW.1b_LV", "NW.1b_LV", "NW.1b_LV", "NW.1b_LV", "NW.1b_LV"], vec![])]
// 1.995 resistor wells: under Dualgate (MV), Dualgate abutting the well (LV), 2.0 under
// Dualgate (clean).
#[case::nw_1b_h2("nwell/NW.1b.h2.gds.gz", "TOP", vec!["NW.1b_LV", "NW.1b_LV", "NW.1b_MV", "NW.1b_MV"], vec![])]
// One-net pairs at 0.595 (fires) and 0.6; a 0.595 slot (fires) and a 0.6 one.
#[case::nw_2a_h1("nwell/NW.2a.h1.gds.gz", "TOP", vec!["NW.2a_LV", "NW.2a_LV"], vec![])]
// Under Dualgate 0.735 (fires) and 0.74; a 0.7 pair with Dualgate over the left well
// only, reaching into the gap: the left well is MV, its space 0.74, fires.
#[case::nw_2a_h2("nwell/NW.2a.h2.gds.gz", "TOP", vec!["NW.2a_MV", "NW.2a_MV"], vec![])]
// 0.595 pairs under YMTP_MK: the marker over both wells is exempt; over the gap only,
// the wells are not inside the marker and the base rule keeps them (kept: the Y rule
// measures the same gap, and a cell marker that covers a gap but not the cell's wells
// is not how YMTP_MK is drawn).
#[case::nw_2a_h3("nwell/NW.2a.h3.gds.gz", "TOP", vec!["NW.2a_LV"], vec!["Y.NW.2b_LV"])]
// Two-net pairs at 1.395, 1.4 (clean), 0.595; and at 1.395: joined through Metal2
// (clean), the left plate over the right well (fires), a tap without a contact (fires),
// a P+ diffusion for a tap (fires).  NW.2a at 0.595 is the same gap under its other name.
#[case::nw_2b_h1("nwell/NW.2b.h1.gds.gz", "TOP", vec!["NW.2b_LV", "NW.2b_LV", "NW.2b_LV", "NW.2b_LV", "NW.2b_LV"], vec!["NW.2a_LV"])]
// Two nets under Dualgate at 1.695 (fires) and 1.7.
#[case::nw_2b_h2("nwell/NW.2b.h2.gds.gz", "TOP", vec!["NW.2b_MV"], vec![])]
// Two nets 1.695 apart with Dualgate over the left well only: the left well is MV, its
// space 1.7, fires; the same at 1.7 is clean.
#[case::nw_2b_h3("nwell/NW.2b.h3.gds.gz", "TOP", vec!["NW.2b_MV"], vec![])]
// Two nets at 1.395 with Dualgate abutting the left well's outer edge: both LV, fires.
#[case::nw_2b_h4("nwell/NW.2b.h4.gds.gz", "TOP", vec!["NW.2b_LV"], vec![])]
// Two tapped wells 1.395 apart in one DNWELL: shorted through it, clean.
#[case::nw_2b_h5("nwell/NW.2b.h5.gds.gz", "TOP", vec![], vec![])]
// Two untapped wells 1.395 apart in one tapped DNWELL: shorted through it, clean.
#[case::nw_2b_h6("nwell/NW.2b.h6.gds.gz", "TOP", vec![], vec![])]
// Two-net 1.395 gaps straddling x = 20 and 42; one-net 0.595 gaps straddling 40 and 21.
#[case::nw_2b_h7("nwell/NW.2b.h7.gds.gz", "TOP", vec!["NW.2a_LV", "NW.2a_LV", "NW.2b_LV", "NW.2b_LV"], vec![])]
// Well to DNWELL: 3.1 (clean), 3.095, abutting, 3.097 corner to corner, 3.111 corner to
// corner (clean), 3.095 inside a DNWELL ring's hole.
#[case::nw_3_h1("nwell/NW.3.h1.gds.gz", "TOP", vec!["NW.3", "NW.3", "NW.3", "NW.3"], vec![])]
// Well over P-well by 0.005 (fires), abutting (clean), by 0.005 inside a DNWELL (fires).
#[case::nw_4_h1("nwell/NW.4.h1.gds.gz", "TOP", vec!["NW.4", "NW.4"], vec![])]
// Held by 0.5 (clean), 0.495, 0 (edge on edge), 0.495 to a chamfered corner, 0.502 to a
// chamfered corner (clean).
#[case::nw_5_h1("nwell/NW.5.h1.gds.gz", "TOP", vec!["NW.5_LV", "NW.5_LV", "NW.5_LV"], vec![])]
// A 3.3 V well half out of the DNWELL: NW.5_LV, and not the 5 V rule.
#[case::nw_5_h2("nwell/NW.5.h2.gds.gz", "TOP", vec!["NW.5_LV"], vec![])]
// Under Dualgate: held by 0.495, half out, held by 0.5 (clean).
#[case::nw_5_h3("nwell/NW.5.h3.gds.gz", "TOP", vec!["NW.5_MV", "NW.5_MV"], vec![])]
// Held by 0.495 with Dualgate abutting the well's edge: a 3.3 V well, NW.5_LV.
#[case::nw_5_h4("nwell/NW.5.h4.gds.gz", "TOP", vec!["NW.5_LV"], vec![])]
// A well in a DNWELL with RES_MK all round it (fires); the same without a marker.
#[case::nw_6_h1("nwell/NW.6.h1.gds.gz", "TOP", vec!["NW.6"], vec![])]
// A well in a DNWELL with the marker between its COMP heads, NW.7's drawing: a resistor
// in a DNWELL, fires.
#[case::nw_6_h2("nwell/NW.6.h2.gds.gz", "TOP", vec!["NW.6"], vec![])]
fn hardening_nwell(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(hardening("nwell", gds, topcell, &ignore), expected, "{gds}");
}

// --- Deep N-well (hardening/reports/gf180mcuD/dnwell.md).  Two wells are one potential
// when an N+ tap in each reaches one Metal1 plate; a diffusion in a P-well inside the
// deep well, or a P+ one, is not a tap.
#[rstest]
// 1.695 × 6 (fires, one marker per wall) and 1.7 × 6, each in a ring.
#[case::dn_1_h1("dnwell/DN.1.h1.gds.gz", "TOP", vec!["DN.1", "DN.1"], vec![])]
// One-net pairs at 2.495 (fires) and 2.5; a 2.495 slot (fires) and a 2.5 one.
#[case::dn_2a_h1("dnwell/DN.2a.h1.gds.gz", "TOP", vec!["DN.2a", "DN.2a"], vec![])]
// Two wells 2.495 apart tapped only through the N-wells they hold, one plate: one net.
#[case::dn_2a_h2("dnwell/DN.2a.h2.gds.gz", "TOP", vec!["DN.2a"], vec![])]
// One-net 2.495 gaps straddling x = 20 and 42; two-net 5.415 gaps straddling 40 and 21.
#[case::dn_2a_h3("dnwell/DN.2a.h3.gds.gz", "TOP", vec!["DN.2a", "DN.2a", "DN.2b", "DN.2b"], vec![])]
// Two-net pairs at 5.415 (fires), 5.42, 2.495 (fires - two potentials, DN.2b's).
#[case::dn_2b_h1("dnwell/DN.2b.h1.gds.gz", "TOP", vec!["DN.2b", "DN.2b"], vec![])]
// 5.415 pairs strapped through an N+ in the right well's P-well, over the right well
// without a contact, and through a P+ diffusion: two potentials each.
#[case::dn_2b_h2("dnwell/DN.2b.h2.gds.gz", "TOP", vec!["DN.2b", "DN.2b", "DN.2b"], vec![])]
// Two untapped wells 2.495 apart: nothing ties them, DN.2b.
#[case::dn_2b_h3("dnwell/DN.2b.h3.gds.gz", "TOP", vec!["DN.2b"], vec![])]
// A C for a ring, an N+ wall, one ring round two wells (twice), an N-well and an N+
// diffusion beside the well inside its ring.
#[case::dn_3_h1("dnwell/DN.3.h1.gds.gz", "TOP", vec!["DN.3", "DN.3", "DN.3", "DN.3", "DN.3", "DN.3"], vec![])]
// A P+ diffusion and an N-well inside the well in the ring, a ring abutting the well,
// an octagonal ring, a ring of sixteen boxes, a ring in a ring.
#[case::dn_3_h2("dnwell/DN.3.h2.gds.gz", "TOP", vec![], vec![])]
// A 30 µm ring round a 20 µm well across the tile lines (clean); the same with a gap.
#[case::dn_3_h3("dnwell/DN.3.h3.gds.gz", "TOP", vec!["DN.3"], vec![])]
fn hardening_dnwell(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("dnwell", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

// --- Low-voltage P-well (hardening/reports/gf180mcuD/lvpwell.md).  Table A (inside
// DNWELL) or table B (outside), and the 5 V kind where Dualgate lies over the well.
#[rstest]
// In one DNWELL: 0.595 bare, 0.735 under Dualgate, 0.74 under it (clean), 0.6 bare
// (clean), 0.735 half under it, 0.7 with Dualgate abutting (LV, clean).
#[case::lpw_1_h1("lvpwell/LPW.1.h1.gds.gz", "TOP", vec!["LPW.1_LV", "LPW.1_LV", "LPW.1_MV", "LPW.1_MV", "LPW.1_MV", "LPW.1_MV"], vec![])]
// 0.595 wells with no deep well near them: table B has no width rule.
#[case::lpw_1_h2("lvpwell/LPW.1.h2.gds.gz", "TOP", vec![], vec![])]
// A 0.595 well under Dualgate in a DNWELL: the 5 V rule, and only that.
#[case::lpw_1_h3("lvpwell/LPW.1.h3.gds.gz", "TOP", vec!["LPW.1_MV", "LPW.1_MV"], vec![])]
// A 30 µm 0.595 well whose last 5 µm lie in a DNWELL: narrow (two walls) and crossing.
#[case::lpw_1_h4("lvpwell/LPW.1.h4.gds.gz", "TOP", vec!["LPW.1_LV", "LPW.1_LV", "LPW.3"], vec![])]
// Two-net pairs in one DNWELL at 1.395, 1.4 (clean), 1.395 joined through Metal2
// (clean), the left plate over the right well, an N+ diffusion for a tap.
#[case::lpw_2a_h1("lvpwell/LPW.2a.h1.gds.gz", "TOP", vec!["LPW.2a_LV", "LPW.2a_LV", "LPW.2a_LV"], vec![])]
// Two nets under Dualgate at 1.695 (fires) and 1.7.
#[case::lpw_2a_h2("lvpwell/LPW.2a.h2.gds.gz", "TOP", vec!["LPW.2a_MV"], vec![])]
// Two nets 1.695 apart with Dualgate over the left well only: the left is MV, fires.
#[case::lpw_2a_h3("lvpwell/LPW.2a.h3.gds.gz", "TOP", vec!["LPW.2a_MV"], vec![])]
// Two nets at 1.395 with Dualgate abutting the left well's outer edge: both LV, fires.
#[case::lpw_2a_h4("lvpwell/LPW.2a.h4.gds.gz", "TOP", vec!["LPW.2a_LV"], vec![])]
// One-net pairs at 0.855 (fires) and 0.86; a 0.855 slot (fires) and a 0.86 one.
#[case::lpw_2b_h1("lvpwell/LPW.2b.h1.gds.gz", "TOP", vec!["LPW.2b_LV", "LPW.2b_LV"], vec![])]
// The same under Dualgate: 0.855 pair, 0.86 pair (clean), 0.855 slot.
#[case::lpw_2b_h2("lvpwell/LPW.2b.h2.gds.gz", "TOP", vec!["LPW.2b_MV", "LPW.2b_MV"], vec![])]
// No deep well anywhere: 0.855 two-net pair, 0.855 slot, 1.395 two-net pair - table B
// has no space rule.
#[case::lpw_2_h5("lvpwell/LPW.2.h5.gds.gz", "TOP", vec![], vec![])]
// Two-net 1.395 gaps straddling x = 20 and 42; one-net 0.855 gaps straddling 40 and 21.
#[case::lpw_2_h6("lvpwell/LPW.2.h6.gds.gz", "TOP", vec!["LPW.2a_LV", "LPW.2a_LV", "LPW.2b_LV", "LPW.2b_LV"], vec![])]
// Two untapped wells 1.395 apart in a DNWELL: nothing ties them, LPW.2a.
#[case::lpw_2_h7("lvpwell/LPW.2.h7.gds.gz", "TOP", vec!["LPW.2a_LV"], vec![])]
// Held by 2.5 (clean), 2.495, 0 (edge on edge), 2.475 to a chamfered corner, 2.546 to a
// chamfered corner (clean).
#[case::lpw_3_h1("lvpwell/LPW.3.h1.gds.gz", "TOP", vec!["LPW.3", "LPW.3", "LPW.3"], vec![])]
// A well half out of the DNWELL: not enclosed.
#[case::lpw_3_h2("lvpwell/LPW.3.h2.gds.gz", "TOP", vec!["LPW.3"], vec![])]
// A resistor with no deep well (fires), one in a deep well, a marker with no COMP.
#[case::lpw_5_h1("lvpwell/LPW.5.h1.gds.gz", "TOP", vec!["LPW.5"], vec![])]
// A resistor under Dualgate with no deep well: the rule has no voltage column.
#[case::lpw_5_h2("lvpwell/LPW.5.h2.gds.gz", "TOP", vec!["LPW.5"], vec![])]
// Well to DNWELL: 1.5 (clean), 1.495, abutting, 1.499 corner to corner, 1.506 corner to
// corner (clean), 1.495 inside a DNWELL ring's hole.
#[case::lpw_11_h1("lvpwell/LPW.11.h1.gds.gz", "TOP", vec!["LPW.11", "LPW.11", "LPW.11", "LPW.11"], vec![])]
// A 0.595 well abutting a DNWELL from outside: table B's LPW.11 at a space of nothing,
// and not table A's width rule.  Whether it is also LPW.3 is a labelling question.
#[case::lpw_11_h2("lvpwell/LPW.11.h2.gds.gz", "TOP", vec!["LPW.11"], vec!["LPW.3"])]
// Well over N-well by 0.005 with no deep well (fires), abutting (clean), by 0.005 in a
// DNWELL (NW.4's, clean here).
#[case::lpw_12_h1("lvpwell/LPW.12.h1.gds.gz", "TOP", vec!["LPW.12"], vec![])]
fn hardening_lvpwell(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("lvpwell", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

/// `expected` concatenated: `n` of each id in `ids`.
fn each(ids: &[(&'static str, usize)]) -> Vec<&'static str> {
    ids.iter()
        .flat_map(|&(id, n)| std::iter::repeat_n(id, n))
        .collect()
}

// --- Gate poly (poly2): hardening/reports/gf180mcuD/poly2.md.  Counts follow the engine's
// marker cuts where it is right: two per narrow wall pair (min_width and the gate-length
// rules), one per space or enclosure pair, one per corner, one per forbidden region.  The
// 45° bars are drawn with horizontal ends, whose acute tips are PL.1 widths of their own
// and are ignored where a bar serves another rule.

#[rstest]
// 0.175 fires the 3.3 V width (2) and 0.18 is clean; under a Dualgate 0.195, 0.18 and a
// 0.175 poly crossing the marker's edge fire the 5 V width (6, plus PL.9); a 0.175 poly
// abutting the marker's edge from outside is a 3.3 V interconnect by the manual (2) and is
// read in no column by the runset (finding 1).
#[case::pl1_h1("poly2/PL.1.h1.gds.gz", "TOP", each(&[("PL.1_LV", 4), ("PL.1_MV", 6)]), vec!["PL.9"])]
// Inside PLFUSE: 0.175 fires PL.1a at both voltages (2 + 2), a 0.19 5 V poly is clean
// inside and PL.1_MV outside (2); a poly abutting the fuse is outside (PL.1_LV, 2); a poly
// crossing the fuse's edge is neither inside nor outside to the runset but is an
// interconnect by the manual: 0.19 at 5 V (PL.1_MV, 2) and 0.175 at 3.3 V (PL.1_LV, 2)
// (finding 2).
#[case::pl1_h2("poly2/PL.1.h2.gds.gz", "TOP", each(&[("PL.1_LV", 4), ("PL.1_MV", 4), ("PL.1a_LV", 2), ("PL.1a_MV", 2)]), vec![])]
// A 0.175 poly running out of a YMTP marker fires on its outside part (2); a 0.4 poly
// ending 0.1 past the marker's edge is 0.4 wide by the manual - the 0.4 x 0.1 piece the
// marker's cut leaves fires in both tools (finding 3).
#[case::pl1_h3("poly2/PL.1.h3.gds.gz", "TOP", vec!["PL.1_LV"; 2], vec![])]
// 0.275 channels (two walls each): a gate, a horizontal gate, one poly over two actives
// (two gates), a 0.5 poly bitten to 0.275 over the active (its corners PL.6, ignored), and
// gates with a wall on x = 20 and 21, straddling x = 20, 40 and 42, and clear of them;
// 0.28 is clean.
#[case::pl2_h1("poly2/PL.2.h1.gds.gz", "TOP", vec!["PL.2_LV"; 22], vec!["PL.6"])]
// The four medium-voltage classes: 6 V N 0.695, 6 V P 0.545, 5 V N 0.595 (across x = 20)
// and 5 V P 0.495 fire (two walls each); 0.7, 0.55, 0.6 and 0.5 are clean; a gate with no
// implant and an N+ gate in an N-well are no device.
#[case::pl2_h2("poly2/PL.2.h2.gds.gz", "TOP", vec!["PL.2_MV"; 8], vec![])]
// Two 0.275 gates whose poly abuts the Dualgate - one along its edge, one at its corner:
// 3.3 V gates by the manual (2 + 2); the runset reads them in no column (finding 1).
#[case::pl2_h3("poly2/PL.2.h3.gds.gz", "TOP", vec!["PL.2_LV"; 4], vec![])]
// 45° gates 0.272 wide fire PL.2 (two walls each): one, one across x = 20 and 21, one
// running up-left; 0.286 is clean for PL.2 (PL.7 ignored).
#[case::pl2_h4("poly2/PL.2.h4.gds.gz", "TOP", vec!["PL.2_LV"; 6], vec!["PL.7_LV", "PL.1_LV"])]
// 0.235 between two gates on one active, between a gate's on-active part and a field
// poly, across a 0.235 slot cut into a gate over the active's edge (its corners PL.6,
// ignored), straddling x = 20 and from x = 40; 0.24 from x = 42 is clean.  A poly crossing
// a 0.23 wide active under RES_MK is one poly and clean; the runset drops its on-active
// part and reads its field pieces 0.23 apart (finding 4).
#[case::pl3a_h1("poly2/PL.3a.h1.gds.gz", "TOP", vec!["PL.3a"; 5], vec!["PL.6"])]
// 0.215 caps: at the bottom, at both ends (2), on a horizontal gate's left, under a
// Dualgate (PL.4_MV) and under a Dualgate and an SRAM core marker (PL.4_MV - section 7.7
// exempts nothing there and the runset checks it; the deck's poly_pl_mv drops the SRAM
// core, finding 5); 0.22 is clean.
#[case::pl4_h1("poly2/PL.4.h1.gds.gz", "TOP", each(&[("PL.4_LV", 4), ("PL.4_MV", 2)]), vec![])]
// A gate over a chamfered active corner extending 0.215 past the chamfer fires (finding
// 6), 0.22 is clean; caps chamfered at 45° from 0.25 and 0.35 up the wall are clean; a
// stub and a poly inside the active have no cap to read (their corners PL.6, ignored);
// 0.215 caps starting on x = 20 and x = 40, straddling x = 20 and x = 42 (4); 0.22 from
// x = 42 is clean.
// The chamfered cap at 0.2157 is not read: the extension rules on angled walls are an
// open class (report, finding 6, and the IHP gatpoly report's finding 11).  The poly's
// own side wall stands 0.05 from where the chamfer leaves the active, which is a
// field-poly-to-COMP space and fires PL.5a/PL.5b twice each.
#[case::pl4_h2("poly2/PL.4.h2.gds.gz", "TOP", each(&[("PL.4_LV", 4), ("PL.5a_LV", 2), ("PL.5b_LV", 2)]), vec!["PL.6"])]
// A 0.215 cap on a gate whose poly abuts the Dualgate: PL.4_LV by the manual, read in no
// column by the runset (finding 1).
#[case::pl4_h3("poly2/PL.4.h3.gds.gz", "TOP", vec!["PL.4_LV"], vec![])]
// Field poly to active (each pair under a and b): 0.095 beside an active, corner to
// corner at 0.099, from x = 20, across x = 20 and across x = 42 (5); under a Dualgate
// 0.295 and the 3.3 V-legal 0.2 (2, MV); a gate's own poly 0.095 beside an arm of the
// L- and the U-shaped active it crosses - PL.5b's related active - fires (2) and the
// engine skips the pair whose shapes overlap (finding 7).  0.1, 0.106, 0.3 and 0.1 from
// x = 42 are clean.
#[case::pl5_h1("poly2/PL.5.h1.gds.gz", "TOP", each(&[("PL.5a_LV", 7), ("PL.5b_LV", 7), ("PL.5a_MV", 2), ("PL.5b_MV", 2)]), vec![])]
// A poly band parallel to a chamfered active corner 0.092 away fires under a and b;
// 0.1025 is clean.  The bands' acute tips are PL.1 widths (ignored).
#[case::pl5_h2("poly2/PL.5.h2.gds.gz", "TOP", vec!["PL.5a_LV", "PL.5b_LV"], vec!["PL.1_LV"])]
// A field poly 0.095 beside an active, its end on the Dualgate's edge: 3.3 V by the
// manual (a and b), read in no column by the runset (finding 1).
#[case::pl5_h3("poly2/PL.5.h3.gds.gz", "TOP", vec!["PL.5a_LV", "PL.5b_LV"], vec![])]
// Right-angle corners on the active: an L's two elbows (2); the elbow 0.05 inside the
// active's edge (2 - both tools skip a corner within 0.1 of the edge, finding 8); the
// convex elbow 0.005 outside (1); a T stub ending inside (4); a step in the width (2);
// the elbow in a YMTP marker (0); in the hole of an active ring of four boxes (0); on
// the seam of two abutting active boxes (2); an L of two overlapping boxes (2, the
// union's).  Two polys end on or 0.005 past an active edge (PL.4, ignored).
// The 0.1 µm corner probe is upstream's and is kept (report, finding 8: open): an
// elbow 0.05 inside the active's edge is not counted.
#[case::pl6_h1("poly2/PL.6.h1.gds.gz", "TOP", vec!["PL.6"; 14], vec!["PL.4_LV"])]
// Elbows on the tile lines: the convex corner on x = 20, 0.005 either side of it, on
// x = 40 and on (40, 40); the concave one on (21, 21) and on x = 42.  Two each.
#[case::pl6_h2("poly2/PL.6.h2.gds.gz", "TOP", vec!["PL.6"; 14], vec![])]
// A convex elbow exactly on the active's edge is not on the active (1, the concave one);
// both elbows on the edges at the active's corner (0); the elbow 0.1 inside (2) and
// 0.095 inside (2): both tools read only the concave one of each (finding 8).
// As h1, the elbows at 0.1 and 0.095 from the edge are not counted (report, finding 8).
#[case::pl6_h3("poly2/PL.6.h3.gds.gz", "TOP", vec!["PL.6"; 3], vec!["PL.4_LV"])]
// 45° gates: 0.2934 fires (two walls), once more across x = 20 and 21, once running
// up-left; 0.3005 is clean; under a Dualgate with no implant 0.693 fires PL.7_MV and
// 0.70004 is clean; an N+ 6 V gate at 0.693 fires PL.7_MV (and PL.2_MV, ignored).  The
// bars' tips are PL.1 widths (ignored).
#[case::pl7_h1("poly2/PL.7.h1.gds.gz", "TOP", each(&[("PL.7_LV", 6), ("PL.7_MV", 4)]), vec!["PL.1_LV", "PL.1_MV", "PL.2_MV"])]
// A 0.2934 45° gate whose end lies on the Dualgate's edge: PL.7's layer is the gate less
// the marker, no whole-region selector, so it stays 3.3 V in both tools (2).
#[case::pl7_h2("poly2/PL.7.h2.gds.gz", "TOP", vec!["PL.7_LV"; 2], vec!["PL.1_LV"])]
// A poly across the Dualgate's edge, one of two boxes meeting on the edge, one bridging
// two markers, one leaving a ring's hole, a 300 µm one into a marker at x = 250, the
// edge on x = 20, 21, 40 and 42, and a poly 0.005 outside (10); a poly inside, one
// abutting the edge from outside, one in the hole and one under V5_XTOR alone are clean.
#[case::pl9_h1("poly2/PL.9.h1.gds.gz", "TOP", vec!["PL.9"; 10], vec![])]
// A V5_XTOR 0.005 off a Dualgate, one 0.005 short of a Dualgate on x = 20, one at
// (1000, 1000) (3); abutting, overlapping, in an OTP marker, two boxes of which one
// touches, and touching across a tile line are clean.
#[case::pl11_h1("poly2/PL.11.h1.gds.gz", "TOP", vec!["PL.11"; 3], vec![])]
// An active under a gate half outside its V5_XTOR, one abutting the marker from outside
// (the runset's reading of "enclose by 0"), and one running out of it past x = 20 (3);
// inside, enclosed by 0, and without a gate are clean.
#[case::pl12_h1("poly2/PL.12.h1.gds.gz", "TOP", vec!["PL.12"; 3], vec![])]
fn hardening_poly2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(hardening("poly2", gds, topcell, &ignore), expected);
}

// --- Dual gate oxide (dualgate): hardening/reports/gf180mcuD/dualgate.md.  Counts as
// above: two per narrow wall pair, one per space or enclosure pair, one per forbidden
// region.  The runset's dnwell deck aborts on a layout with no connectivity and takes the
// dnwell layer with it, so its DV.1 reading comes from a run of the dualgate deck alone.

#[rstest]
// 0.495 fires, 0.5 is clean; a deep well crossing the marker's edge (the part outside);
// one sharing the marker's edge, enclosed by 0; one 0.495 from a chamfered marker corner
// (the closest approach - the deck's enclosure rules carry no euclidian metric, finding
// 1); the well's edge on x = 20 and the marker's edge on x = 20, 0.495 either way; a
// second well in one marker at 0.495 (7).  A well abutting the marker from outside and
// 0.5 at x = 42 are clean.
#[case::dv1_h1("dualgate/DV.1.h1.gds.gz", "TOP", vec!["DV.1"; 7], vec![])]
// 0.435 between markers, a 0.435 notch, corner to corner at 0.438, 0.435 across x = 20
// and from x = 42 (5); 0.44, 0.4455 and two overlapping boxes are clean.
#[case::dv2_h1("dualgate/DV.2.h1.gds.gz", "TOP", vec!["DV.2"; 5], vec![])]
// 0.235 from an outside active, from a substrate tap, from a chamfered marker corner
// (0.233), the active's edge on x = 20 and the marker's edge on x = 20 (5); an active
// abutting the marker's edge is 0 away and fires (6; the engine calls a shared edge no
// space, finding 2).  0.24, 0.24 at x = 42 and an active partly inside (DV.6 and DV.7,
// ignored) are clean.
#[case::dv3_h1("dualgate/DV.3.h1.gds.gz", "TOP", vec!["DV.3"; 6], vec!["DV.6", "DV.7"])]
// 0.695 in x and in y, a 45° strip at 0.693, a 0.695 bar across x = 20 (two walls each);
// 0.7, 0.700 and 0.7 across x = 42 are clean.
#[case::dv5_h1("dualgate/DV.5.h1.gds.gz", "TOP", vec!["DV.5"; 8], vec![])]
// 0.235 for an N+ active, an N+ tap in an N-well, a P+ active crossing the N-well's edge,
// the active's edge on x = 20 and the marker's edge on x = 20; an active sharing the
// marker's edge (0); one crossing the edge (and DV.7, ignored); an active corner 0.233
// from a chamfered marker corner (finding 1) (8).  0.24, 0.24 at x = 42, a substrate tap
// 0.1 inside, one crossing the edge and one abutting the N-well are clean.
#[case::dv6_h1("dualgate/DV.6.h1.gds.gz", "TOP", vec!["DV.6"; 8], vec!["DV.7"])]
// A marker straddling an active, one across x = 20 and one across x = 42 (3); a marker
// that covers one active and straddles a second still straddles the second (4; the
// deck's `covering` excuses the marker, the runset does not, finding 3).  A straddled
// substrate tap, an active sharing the marker's edge, one abutting from outside and a
// marker of two abutting boxes covering an active are clean (DV.3, DV.6 ignored).
#[case::dv7_h1("dualgate/DV.7.h1.gds.gz", "TOP", vec!["DV.7"; 4], vec!["DV.3", "DV.6"])]
// 0.395 on the left; a poly crossing the marker's edge; one sharing the edge (0); one
// 0.389 from a chamfered marker corner (finding 1); one under a marker ring 0.395 from
// the hole; the poly's edge on x = 20 and the marker's edge on x = 20; a 300 µm poly into
// a marker at x = 250 (8).  0.4, 0.4 at x = 42, a poly abutting the marker from outside
// and one in the ring's hole are clean.
#[case::dv8_h1("dualgate/DV.8.h1.gds.gz", "TOP", vec!["DV.8"; 8], vec![])]
// One N-well with a 6 V and a 3.3 V PMOS, one of two abutting boxes, one whose 3.3 V gate
// a second marker straddles (DV.6, DV.7, DV.8 ignored), one with two 3.3 V gates (one
// report), and a 44 µm well with the gates at x = 2 and x = 42 (5); two wells 1.4 apart
// and a 3.3 V gate under V5_XTOR alone are clean.
#[case::dv9_h1("dualgate/DV.9.h1.gds.gz", "TOP", vec!["DV.9"; 5], vec!["DV.6", "DV.7", "DV.8"])]
fn hardening_dualgate(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(hardening("dualgate", gds, topcell, &ignore), expected);
}

// --- comp: the hardening layouts ---
//
// Drawn in `gen/gf180mcuD/comp.rs` from section 7.5 of the manual; the reasoning, the
// oracle's answers and the verdicts are in hardening/reports/gf180mcuD/comp.md.  The
// expected ids are the manual's answer, so the cases that fail are the findings.
//
// Marker counts follow gdscheck's cut: `min_width` gives one marker per narrow wall (two
// per bar), `min_space`, `min_notch` and `max_space` one per violating pair, `min_area`
// and `forbidden` one per region, `min_enclosure` one per short edge.
#[rstest]
// The bound in both voltage columns: 0.215 and 0.25 bars at 3.3 V and 5 V, a bar Dualgate
// covers half of, a bar touching a Dualgate edge, one under V5_XTOR alone, one whose top
// MVSD takes.  The touching bar and the V5_XTOR bar are 0.215 wide and in no column of
// the deck at all (report, finding 1).
#[case::df1a_h1("comp/DF.1a.h1.gds.gz", "TOP", vec!["DF.1a_LV"; 6], vec!["DF.1a_MV"; 8])]
// A 300 µm bar with Dualgate over its last 12 µm is a 5 V active from end to end.
#[case::df1a_h2("comp/DF.1a.h2.gds.gz", "TOP", vec!["DF.1a_LV"; 2], vec!["DF.1a_MV"; 2])]
// MOSCAP width: 0.995 under the marker, and a 2 µm active the marker covers 0.995 of.
#[case::df1c_h1("comp/DF.1c.h1.gds.gz", "TOP", vec!["DF.1c"; 4], vec![])]
// Channel width read on the gate's edge: a 0.215 poly island at 3.3 V, 0.295 and 0.25 at
// 5 V, each edge of the island counting once.
#[case::df2a_h1("comp/DF.2a.h1.gds.gz", "TOP", vec!["DF.2a_LV"; 2], vec!["DF.2a_MV"; 4])]
// The channel of an active drawn as two boxes, a notch in the gate's side, and islands on
// and across x = 20.
#[case::df2a_h2("comp/DF.2a.h2.gds.gz", "TOP", vec!["DF.2a_LV"; 6], vec![])]
// Over 100 µm: a 100.005 square, two 120 plates, a 150 plate with 110 left beside the
// MOSCAP marker, a 120 diamond; 100, a 99 diamond and a 75 remainder are clean.
#[case::df2b_h1("comp/DF.2b.h1.gds.gz", "TOP", vec!["DF.2b"; 5], vec![])]
// Space at 0.275 and 0.355, a 0.275 notch, a tap 0.275 from an N+ active, and a 3.3 V
// active 0.275 from a 5 V one - a pair of unlike voltage, which takes the stricter of
// the two values and is reported under the 5 V id (report, finding 1).
#[case::df3a_h1("comp/DF.3a.h1.gds.gz", "TOP", vec!["DF.3a_LV"; 3], vec!["DF.3a_MV"; 3])]
// Two 300 µm bars 0.30 apart with Dualgate over their last 12 µm, and a 0.275 pair.
#[case::df3a_h2("comp/DF.3a.h2.gds.gz", "TOP", vec!["DF.3a_LV"], vec!["DF.3a_MV"])]
// Butting: a 0.005 implant overlap in an N-well, in the substrate and in a deep well, and
// three butted MOSCAPs - the marker over the whole, over the P+ half and over the N+ half
// (the P+ one goes unseen, report, finding 6).  The 0.005 gap is DF.12.
#[case::df3b_h1("comp/DF.3b.h1.gds.gz", "TOP", vec!["DF.3b"; 6], vec!["DF.12"])]
// BJT area: 0.315 space and notch at 3.3 V; at 5 V a marker over two actives is forbidden,
// one over a single active is not, and an active touching the marker's edge from outside
// does not put a second active in the area (report, finding 7).
#[case::df3c_h1("comp/DF.3c.h1.gds.gz", "TOP", vec!["DF.3c_LV"; 2], vec!["DF.3c_MV"; 2])]
// An N+ tap 0.115 / 0.155 from the P-well in a deep well, and a 0.13 tap in a deep well
// Dualgate only touches at one corner.
#[case::df4a_h1("comp/DF.4a.h1.gds.gz", "TOP", vec!["DF.4a_LV"], vec!["DF.4a_MV"; 2])]
// A 300 µm deep well with Dualgate over its last 12 µm; its tap is 0.13 from the P-well.
#[case::df4a_h2("comp/DF.4a.h2.gds.gz", "TOP", vec!["DF.4a_MV"], vec![])]
// Deep-well overlap of an N+ tap at 0.615 / 0.655, and a 45° corner cut passing 0.17 from
// the tap's corner (report, finding 3).
#[case::df4b_h1("comp/DF.4b.h1.gds.gz", "TOP", vec!["DF.4b_LV"; 2], vec!["DF.4b_MV"])]
// N-well overlap of a P+ source/drain at 0.425 / 0.595, 0.3 under SRAMCORE (the SRAM
// cell's rule), and 0.5 in a well Dualgate covers a corner of.
#[case::df4c_h1("comp/DF.4c.h1.gds.gz", "TOP", vec!["DF.4c_LV"], vec!["DF.4c_MV"; 2])]
// N-well overlap of an N+ tap at 0.115 / 0.155; a 3.3 V tap at 0.14 and a well under
// YMTP_MK are clean.
#[case::df4d_h1("comp/DF.4d.h1.gds.gz", "TOP", vec!["DF.4d_LV"], vec!["DF.4d_MV"])]
// Deep-well overlap of a P+ active at 0.925 / 1.095, the second one a P-well's own tap.
#[case::df4e_h1("comp/DF.4e.h1.gds.gz", "TOP", vec!["DF.4e_LV"; 2], vec!["DF.4e_MV"])]
// P-well overlap of its P+ tap at 0.115 / 0.155; a P-well outside any deep well is clean.
#[case::df5_h1("comp/DF.5.h1.gds.gz", "TOP", vec!["DF.5_LV"], vec!["DF.5_MV"])]
// Source/drain overhang 0.235 / 0.395 / 0.3; the overhangs under MVSD and under RES_MK
// belong to the LDMOS and to a resistor.
#[case::df6_h1("comp/DF.6.h1.gds.gz", "TOP", vec!["DF.6_LV"], vec!["DF.6_MV"; 2])]
// A gate running out over the active's end (no source/drain at all) is reported by
// neither tool and is left as it stands, an enclosure having no margin to read where
// the cover runs past the shape (report, finding 5: open); a 0.235 overhang on a shared
// gate, and 0.235 overhangs on and across x = 20 and x = 42.
#[case::df6_h2("comp/DF.6.h2.gds.gz", "TOP", vec!["DF.6_LV"; 4], vec![])]
// A P+ source/drain in a deep well 0.425 / 0.595 from the P-well; the P-well's own tap,
// 0.5 inside it, is no space.
#[case::df7_h1("comp/DF.7.h1.gds.gz", "TOP", vec!["DF.7_LV"], vec!["DF.7_MV"])]
// P-well overlap of an N+ source/drain at 0.425 / 0.595, and 0.3 under SRAMCORE - the
// SRAM cell has a 5 V rule of its own and no 3.3 V one.
#[case::df8_h1("comp/DF.8.h1.gds.gz", "TOP", vec!["DF.8_LV"; 2], vec!["DF.8_MV"])]
// Area 0.2002 on its own and across x = 20; a union, an L and an OTP active are clean.
#[case::df9_h1("comp/DF.9.h1.gds.gz", "TOP", vec!["DF.9"; 2], vec![])]
// Field area: 0.25 holes as walls, as a keyhole, on and across the tile line, and a 0.7
// hole with a 0.5 island leaving 0.24 of field (report, finding 4).  The island sits 0.1
// from the ring, which is DF.3a.
#[case::df10_h1("comp/DF.10.h1.gds.gz", "TOP", vec!["DF.10"; 5], vec!["DF.3a_LV"])]
// Butting edge: 0.295 across a bar and across x = 20, one marker per edge.  A 2.0 and a
// 1.0 long butting edge on a 0.25 wide active are legal by the manual (report,
// finding 8), and the rule measures the edge now, not the active.
#[case::df11_h1("comp/DF.11.h1.gds.gz", "TOP", vec!["DF.11"; 2], vec![])]
// Implant cover: N+ 0.005 short, N+ ending on x = 20, and an active SCHOTTKY_DIODE covers
// half of (report, finding 9).  The N+/P+ overlap is DF.3b.
#[case::df12_h1("comp/DF.12.h1.gds.gz", "TOP", vec!["DF.12"; 3], vec!["DF.3b"])]
// The N-well tap's reach: 20.005 at 3.3 V (report, finding 2) and 15.005 at 5 V.
#[case::df13_h1("comp/DF.13.h1.gds.gz", "TOP", vec!["DF.13_LV"], vec!["DF.13_MV"])]
// The reach through the well: no tap in this well, 26 round an L, 20.005 across two tile
// lines, 36, and 26.9 diagonally (report, finding 2).  A P+ source/drain and its N+ tap
// 3 µm apart in a deep well have no N-well between them, and the rule names NWELL
// (report, finding 10).
#[case::df13_h2("comp/DF.13.h2.gds.gz", "TOP", vec!["DF.13_LV"; 5], vec![])]
// The substrate tap's reach: 20.005, 20.004 and 26.9 diagonally (report, finding 2), and
// 15.005 at 5 V.
#[case::df14_h1("comp/DF.14.h1.gds.gz", "TOP", vec!["DF.14_LV"; 3], vec!["DF.14_MV"])]
// 20.005 across two tile lines; an N+ source/drain in a deep well's P-well with its tap
// 9.5 away outside the deep well is within reach.
#[case::df14_h2("comp/DF.14.h2.gds.gz", "TOP", vec!["DF.14_LV"], vec![])]
// N-well to N+ active at 0.425 / 0.595, and two mixed-voltage pairs at 0.425, which
// take the stricter value and the 5 V id (report, finding 1).  YMTP_MK and SRAMCORE pairs are the cells'.
#[case::df16_h1("comp/DF.16.h1.gds.gz", "TOP", vec!["DF.16_LV"], vec!["DF.16_MV"; 3])]
// A 300 µm N-well with Dualgate over its last 12 µm, 0.5 from a 5 V active; a 3.3 V well
// of the same length 0.425 from a 3.3 V active.
#[case::df16_h2("comp/DF.16.h2.gds.gz", "TOP", vec!["DF.16_LV"], vec!["DF.16_MV"])]
// N-well to P+ tap at 0.115 / 0.155, a tap butted against the well's edge (report,
// finding 5) and two mixed-voltage pairs at 0.115, which take the stricter value and
// the 5 V id (report, finding 1).
#[case::df17_h1("comp/DF.17.h1.gds.gz", "TOP", vec!["DF.17_LV"; 2], vec!["DF.17_MV"; 3])]
// Deep well to P+ tap at 2.495 and at 2.496 diagonally; 2.5, 2.503 diagonally and a P+
// source/drain inside an N-well are clean.
#[case::df18_h1("comp/DF.18.h1.gds.gz", "TOP", vec!["DF.18"; 2], vec![])]
// Deep well to N+ active at 3.195, 3.199 diagonally and 3.275 at 5 V; an N+ tap inside an
// N-well 1.0 from the deep well is clean.
#[case::df19_h1("comp/DF.19.h1.gds.gz", "TOP", vec!["DF.19_LV"; 2], vec!["DF.19_MV"])]
fn hardening_comp(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] first: Vec<&str>,
    #[case] second: Vec<&str>,
) {
    let mut want: Vec<String> = first
        .into_iter()
        .chain(second)
        .map(ToString::to_string)
        .collect();
    want.sort();
    assert_eq!(hardening("comp", gds, topcell, &[]), want, "{gds}");
}

// --- Contact (hardening/reports/gf180mcuD/contact.md).  Section 7.12 has no 3.3 V / 5 V
// split, so no rule here is `_LV` / `_MV`.  `exact_width` reports one marker per wall,
// two per contact that is off in one direction; `min_enclosure` one per deficient side.
#[rstest]
// Five contacts: 0.22 square (clean), 0.225 tall, 0.225 wide, 0.215 wide, 0.215 tall.
#[case::co_1_h1("contact/CO.1.h1.gds.gz", "TOP", vec!["CO.1"; 8], vec![])]
// A contact that is not a square: two abutting squares making a 0.44 x 0.22 bar, an L
// with 0.22 arms, and one square drawn as two overlapping boxes (clean).
#[case::co_1_h2("contact/CO.1.h2.gds.gz", "TOP", vec!["CO.1"; 6], vec![])]
// A 4x4 array at 0.30 with one 0.275 row gap (four pairs); the same with three columns
// and sixteen contacts in one row, neither an array.
#[case::co_2b_h1("contact/CO.2b.h1.gds.gz", "TOP", vec!["CO.2b"; 4], vec![])]
// A seventeenth contact 0.275 beside a legal 4x4 (1); the same beside a 3x3 (clean); a
// contact 0.2762 corner to corner off a legal 4x4 (1); a 4x4 with its own 0.275 row gap
// and a seventeenth contact 0.275 beside it (4 + 1).
#[case::co_2b_h2("contact/CO.2b.h2.gds.gz", "TOP", vec!["CO.2b"; 7], vec![])]
// The same array with a 0.275 row gap straddling x = 20, x = 42 and y = 21.
#[case::co_2b_h3("contact/CO.2b.h3.gds.gz", "TOP", vec!["CO.2b"; 12], vec![])]
// Poly over a contact by 0.07 (clean), 0.065, nothing, and a 0.05 chamfer passing 0.0636
// from the contact's corner; a 0.04 chamfer at 0.0707 is clean.
#[case::co_3_h1("contact/CO.3.h1.gds.gz", "TOP", vec!["CO.3"; 3], vec![])]
// A contact the poly edge cuts (CO.3 and CO.11 on the part outside), a contact abutting
// the poly from outside (CO.11 alone), and a 0.065 margin under SRAMCORE (exempt).
#[case::co_3_h2("contact/CO.3.h2.gds.gz", "TOP", vec!["CO.11", "CO.11", "CO.3"], vec![])]
// The same five readings against COMP.
#[case::co_4_h1("contact/CO.4.h1.gds.gz", "TOP", vec!["CO.4"; 3], vec![])]
// The same three conditions against COMP.
#[case::co_4_h2("contact/CO.4.h2.gds.gz", "TOP", vec!["CO.11", "CO.11", "CO.4"], vec![])]
// On butted N+/P+ COMP: 0.1 from the butting edge (clean), 0.095 on the N side, 0.095 on
// the P side, and 0.095 from the N+ COMP's own bottom edge.
#[case::co_5_h1("contact/CO.5.h1.gds.gz", "TOP", vec!["CO.5a", "CO.5a", "CO.5b"], vec![])]
// Implants that overlap by 0.2 and implants 0.2 apart are not butted, so 0.095 is clean
// there; the butted control at 0.095 fires.
#[case::co_5_h2("contact/CO.5.h2.gds.gz", "TOP", vec!["CO.5a"], vec![])]
// Metal1 over a contact by 0.005 (clean), flush, absent, and covering half of it.
#[case::co_6_h1("contact/CO.6.h1.gds.gz", "TOP", vec!["CO.6"; 3], vec![])]
// Line-end caps 0.055 past the contact on tracks 0.34 (not a narrow line, clean), 0.30,
// 0.35 (clean) and 0.33 wide; 0.34 wide with a 0.06 cap is clean.
#[case::co_6a_h1("contact/CO.6a.h1.gds.gz", "TOP", vec!["CO.6a"; 2], vec![])]
// The same cap on a branch off a wide plate: 0.22 long is not a line end (clean), 0.24
// and 0.30 are.
#[case::co_6a_h2("contact/CO.6a.h2.gds.gz", "TOP", vec!["CO.6a"; 2], vec![])]
// A 0.035 side with 0.06 beside it (clean), with 0.055 beside it, a 0.04 side that does
// not trigger, two adjacent short sides, two opposite short sides (clean).
#[case::co_6b_h1("contact/CO.6b.h1.gds.gz", "TOP", vec!["CO.6b"; 2], vec![])]
// A COMP contact 0.15 from a gate (clean) and 0.145; 0.10 from a poly line that is not
// over COMP (clean); 0.145 under OTP_MK, whole and over the gate only (both exempt).
#[case::co_7_h1("contact/CO.7.h1.gds.gz", "TOP", vec!["CO.7"], vec![])]
// A poly contact 0.17 from COMP (clean), 0.165, and 0.165 from a COMP the same poly
// crosses elsewhere.
#[case::co_8_h1("contact/CO.8.h1.gds.gz", "TOP", vec!["CO.8"; 2], vec![])]
// A contact straddling the butting edge (CO.9, and no N+ or P+ overlap either side); one
// abutting it from the P side (CO.9 by the `interacting` reading, and CO.5b at 0); a
// contact across implants that overlap by 0.2, where there is no butting edge.
#[case::co_9_h1("contact/CO.9.h1.gds.gz", "TOP", vec!["CO.5a", "CO.5b", "CO.5b", "CO.9", "CO.9"], vec![])]
// Straddling contacts on butting edges at x = 20, 21, 40 and 42.
#[case::co_9_h2("contact/CO.9.h2.gds.gz", "TOP", vec!["CO.9"; 4], vec![])]
// A contact on the gate; one on the poly 0.17 above the COMP (clean); one whose bottom
// edge lies on the COMP's top edge, a space of nothing, which CO.8 measures; one on the
// gate with RES_MK over the whole overlap, which is no gate (clean).
#[case::co_10_h1("contact/CO.10.h1.gds.gz", "TOP", vec!["CO.10", "CO.8"], vec![])]
// A contact on field oxide; one on poly alone (clean); one 0.005 off the COMP's edge
// (CO.11 on the sliver, CO.4 for crossing); one shared by an abutting COMP and poly - no
// field oxide under it, but it crosses both edges and sits 0 from COMP.
#[case::co_11_h1("contact/CO.11.h1.gds.gz", "TOP", vec!["CO.11", "CO.11", "CO.3", "CO.4", "CO.4", "CO.8"], vec![])]
fn hardening_contact(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("contact", gds, topcell, &ignore), want, "{gds}");
}
