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
    &[("MDP.1", 16), ("MDP.10", 30), ("MDP.10a", 7), ("MDP.10b", 2), ("MDP.11", 36), ("MDP.12", 4), ("MDP.13a", 1), ("MDP.13b", 21), ("MDP.13c", 3), ("MDP.15", 1), ("MDP.16a", 2), ("MDP.16b", 2), ("MDP.17a", 4), ("MDP.17c", 1), ("MDP.1a", 2), ("MDP.2", 25), ("MDP.3ai", 91), ("MDP.3aii", 8), ("MDP.3b", 4), ("MDP.3d", 2), ("MDP.4", 2), ("MDP.4a", 8), ("MDP.4b", 4), ("MDP.5", 8), ("MDP.5a", 5), ("MDP.6", 3), ("MDP.6a", 21), ("MDP.7", 1), ("MDP.8", 1), ("MDP.9a", 38), ("MDP.9b", 13), ("MDP.9d", 15), ("MDP.9ei", 6), ("MDP.9eii", 4), ("MDP.9f", 1)]
)]
#[case::nat(
    "nat", "nat.gds.gz", "10_5_NAT",
    &[("NAT.1", 2), ("NAT.10", 1), ("NAT.11", 1), ("NAT.12", 2), ("NAT.2", 4), ("NAT.3", 3), ("NAT.4", 24), ("NAT.5", 12), ("NAT.6", 7), ("NAT.7", 2), ("NAT.8", 5), ("NAT.9", 4)]
)]
#[case::ldnmos(
    "ldnmos", "ldnmos.gds.gz", "10_12_1_MDN",
    &[("MDN.1", 49), ("MDN.10a", 64), ("MDN.10b", 4), ("MDN.10c", 17), ("MDN.10ei", 3), ("MDN.10eii", 2), ("MDN.10f", 6), ("MDN.11", 112), ("MDN.12", 18), ("MDN.13a", 8), ("MDN.13b", 9), ("MDN.13c", 6), ("MDN.13d", 18), ("MDN.14", 30), ("MDN.15a", 44), ("MDN.15b", 2), ("MDN.17", 74), ("MDN.2a", 23), ("MDN.2b", 27), ("MDN.3a", 10), ("MDN.3b", 6), ("MDN.4a", 29), ("MDN.4b", 16), ("MDN.5ai", 31), ("MDN.5aii", 4), ("MDN.5b", 8), ("MDN.6", 14), ("MDN.6a", 6), ("MDN.7", 81), ("MDN.7a", 274), ("MDN.8a", 10), ("MDN.8b", 14), ("MDN.9", 7)]
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
    &[("HRES.1", 7), ("HRES.10", 63), ("HRES.12a", 37), ("HRES.12b", 1), ("HRES.2", 62), ("HRES.3", 8), ("HRES.4", 38), ("HRES.5", 7), ("HRES.6", 7), ("HRES.7", 39), ("HRES.8", 27), ("HRES.9", 10)]
)]
#[case::dualgate(
    "dualgate", "dualgate.gds.gz", "7_6_Dualgate",
    &[("DV.1", 5), ("DV.2", 3), ("DV.3", 4), ("DV.5", 13), ("DV.6", 5), ("DV.7", 1), ("DV.8", 9), ("DV.9", 1)]
)]
#[case::sram_3p3(
    "sram_3p3", "sram_3p3.gds.gz", "sram_3p3",
    &[("S.CO.3_LV", 9), ("S.CO.4_LV", 9), ("S.CO.6_ii_LV", 8), ("S.DF.16_LV", 7), ("S.DF.4c_LV", 9), ("S.M1.1_LV", 32)]
)]
#[case::drc_bjt(
    "drc_bjt", "drc_bjt.gds.gz", "DRC_BJT",
    &[("BJT.1", 1), ("BJT.2", 3), ("BJT.3", 3)]
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
    &[("PRES.1", 51), ("PRES.2", 6), ("PRES.3", 10), ("PRES.4", 6), ("PRES.5", 9), ("PRES.6", 41), ("PRES.7", 8), ("PRES.9a", 7), ("PRES.9b", 1)]
)]
#[case::comp(
    "comp", "comp.gds.gz", "7_5_DF",
    &[("DF.10", 5), ("DF.11", 9), ("DF.12", 74), ("DF.13_LV", 45), ("DF.13_MV", 45), ("DF.14_LV", 41), ("DF.14_MV", 41), ("DF.16_LV", 6), ("DF.16_MV", 6), ("DF.17_LV", 10), ("DF.17_MV", 10), ("DF.18", 6), ("DF.19_LV", 6), ("DF.19_MV", 6), ("DF.1a_LV", 105), ("DF.1a_MV", 170), ("DF.1c", 10), ("DF.2a_LV", 4), ("DF.2a_MV", 4), ("DF.2b", 2), ("DF.3a_LV", 29), ("DF.3a_MV", 28), ("DF.3b", 16), ("DF.3c_LV", 7), ("DF.3c_MV", 11), ("DF.4a_LV", 12), ("DF.4a_MV", 6), ("DF.4b_LV", 9), ("DF.4b_MV", 9), ("DF.4c_LV", 9), ("DF.4c_MV", 7), ("DF.4d_LV", 7), ("DF.4d_MV", 7), ("DF.4e_LV", 7), ("DF.4e_MV", 7), ("DF.5_LV", 7), ("DF.5_MV", 7), ("DF.6_LV", 4), ("DF.6_MV", 4), ("DF.7_LV", 6), ("DF.7_MV", 6), ("DF.8_LV", 7), ("DF.8_MV", 7), ("DF.9", 215)]
)]
#[case::sab(
    "sab", "sab.gds.gz", "7_10_SB",
    &[("SB.1", 20), ("SB.10", 97), ("SB.11", 1), ("SB.12", 1), ("SB.13", 132), ("SB.14a", 6), ("SB.14b", 6), ("SB.15a", 13), ("SB.15b", 5), ("SB.16", 6), ("SB.2", 9), ("SB.3", 11), ("SB.4", 7), ("SB.5a", 12), ("SB.5b", 8), ("SB.6", 27), ("SB.7", 26), ("SB.8", 3), ("SB.9", 36)]
)]
#[case::ymtp_mk(
    "ymtp_mk", "ymtp_mk.gds.gz", "10_13_YMTP",
    &[("Y.DF.16_LV", 6), ("Y.DF.16_MV", 6), ("Y.DF.6_MV", 16), ("Y.NW.2b_LV", 14), ("Y.NW.2b_MV", 28), ("Y.PL.1_LV", 100), ("Y.PL.1_MV", 119), ("Y.PL.2_LV", 68), ("Y.PL.2_MV", 164), ("Y.PL.4_MV", 7), ("Y.PL.5a_LV", 8), ("Y.PL.5a_MV", 6), ("Y.PL.5b_LV", 8), ("Y.PL.5b_MV", 6)]
)]
#[case::contact(
    "contact", "contact.gds.gz", "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump",
    &[("CO.1", 104), ("CO.10", 2), ("CO.11", 145), ("CO.2a", 8), ("CO.2b", 6), ("CO.3", 20), ("CO.4", 13), ("CO.5a", 5), ("CO.5b", 7), ("CO.6", 98), ("CO.6a", 25), ("CO.6b", 30), ("CO.7", 2), ("CO.8", 2), ("CO.9", 6)]
)]
#[case::sram_5p0(
    "sram_5p0", "sram_5p0.gds.gz", "sram_5p0",
    &[("S.CO.4_MV", 9), ("S.DF.16_MV", 7), ("S.DF.4c_MV", 9), ("S.DF.6_MV", 16), ("S.DF.7_MV", 7), ("S.DF.8_MV", 9), ("S.PL.5a_MV", 10), ("S.PL.5b_MV", 10)]
)]
#[case::lres(
    "lres", "lres.gds.gz", "10_2_LRES",
    &[("LRES.1", 50), ("LRES.2", 6), ("LRES.3", 10), ("LRES.4", 6), ("LRES.5", 9), ("LRES.6", 41), ("LRES.7", 8), ("LRES.9a", 11), ("LRES.9b", 1)]
)]
#[case::mim_b(
    "mim_b", "mim_b.gds.gz", "10_4_2_MIM_OptionB",
    &[("MIMTM.1", 3), ("MIMTM.10", 3), ("MIMTM.11", 2), ("MIMTM.2", 7), ("MIMTM.3", 62), ("MIMTM.4", 12), ("MIMTM.5", 11), ("MIMTM.6", 6), ("MIMTM.7", 2490), ("MIMTM.8a", 60), ("MIMTM.8b", 1), ("MIMTM.9", 6)]
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
    &[("NP.1", 395), ("NP.10", 7), ("NP.11", 10), ("NP.12", 1), ("NP.2", 86), ("NP.3a", 9), ("NP.3bi", 10), ("NP.3bii", 4), ("NP.3ci", 15), ("NP.3cii", 4), ("NP.3d", 2), ("NP.3e", 2), ("NP.4a", 4), ("NP.4b", 1), ("NP.5a", 8), ("NP.5b", 66), ("NP.5ci", 8), ("NP.5cii", 3), ("NP.5di", 7), ("NP.5dii", 9), ("NP.6", 72), ("NP.7", 7), ("NP.8a", 121), ("NP.8b", 2), ("NP.9", 8)]
)]
#[case::metaltop(
    "metaltop", "metaltop.gds.gz", "metaltop",
    &[("MT.1", 386), ("MT.2a", 16), ("MT.2b", 11), ("MT.4", 85)]
)]
#[case::pplus(
    "pplus", "pplus.gds.gz", "7_11_Pplus",
    &[("PP.1", 395), ("PP.10", 5), ("PP.11", 8), ("PP.12", 1), ("PP.2", 86), ("PP.3a", 21), ("PP.3bi", 4), ("PP.3bii", 11), ("PP.3ci", 4), ("PP.3cii", 10), ("PP.3d", 2), ("PP.3e", 2), ("PP.4a", 4), ("PP.4b", 1), ("PP.5a", 8), ("PP.5b", 71), ("PP.5ci", 6), ("PP.5cii", 9), ("PP.5di", 8), ("PP.5dii", 33), ("PP.6", 72), ("PP.7", 7), ("PP.8a", 121), ("PP.8b", 2), ("PP.9", 6)]
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
    // A contact straddling the N+/P+ butting edge is CO.9 by definition, and by the same
    // geometry it runs out of both implants - which is what CO.5a's and CO.5b's own
    // "crossing" halves report.  One shape, three rules, nothing to separate.
    ("contact", "CO.9", "CO.5a"),
    ("contact", "CO.9", "CO.5b"),
    // A slot mark in a hole of the metal is `.10` by the manual's own id, and the same
    // geometry is a mark no metal encloses, which is `.5`.  One drawing, two rules.
    ("mslot", "MSLOT1.10", "MSLOT1.5"),
    ("mslot", "MSLOT2.10", "MSLOT2.5"),
    ("mslot", "MSLOT3.10", "MSLOT3.5"),
    ("mslot", "MSLOT4.10", "MSLOT4.5"),
    ("mslot", "MSLOT5.10", "MSLOT5.5"),
    // A block drawn over a whole gate is SB.16, and the poly under it then runs past
    // the block by nothing, which is SB.10's own contained-shape half.  One shape, two
    // rules.
    ("sab", "SB.16", "SB.10"),
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
// The array rules report one marker per array, however many of its pairs are short:
// an array's pitch is one structure, and the reading is the IHP round's (kept; the
// argument for one marker per pair is in the report, finding 2).
#[case::co_2b_h1("contact/CO.2b.h1.gds.gz", "TOP", vec!["CO.2b"], vec![])]
// A seventeenth contact 0.275 beside a legal 4x4 (1); the same beside a 3x3 (clean); a
// contact 0.2762 corner to corner off a legal 4x4 (1); a 4x4 with its own 0.275 row gap
// and a seventeenth contact 0.275 beside it (4 + 1).
// A contact in the array's cluster but off its grid: the pair is not spanned by the
// array's rows, and the deliberate "finger" reading exempts it (kept; report,
// findings 3 and 4).
#[case::co_2b_h2("contact/CO.2b.h2.gds.gz", "TOP", vec!["CO.2b"], vec![])]
// The same array with a 0.275 row gap straddling x = 20, x = 42 and y = 21.
// The array rules report one marker per array, however many of its pairs are short:
// an array's pitch is one structure, and the reading is the IHP round's (kept; the
// argument for one marker per pair is in the report, finding 2).
#[case::co_2b_h3("contact/CO.2b.h3.gds.gz", "TOP", vec!["CO.2b"; 3], vec![])]
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
// Straddling contacts on butting edges at x = 20, 21, 40 and 42; each of the four also
// runs out of both implants, which is CO.5a and CO.5b (report, finding 6).
#[case::co_9_h2("contact/CO.9.h2.gds.gz", "TOP", vec!["CO.9"; 4], vec!["CO.5a", "CO.5a", "CO.5a", "CO.5a", "CO.5b", "CO.5b", "CO.5b", "CO.5b"])]
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
// --- Via (hardening/reports/gf180mcuD/via.md).  Section 7.14 has no 3.3 V / 5 V split
// either.  A via in a track narrower than 0.34 always has margins under V#.3d's 0.04
// trigger, so the line-end cases carry V#.3d / V#.4c in their ignore list.
#[rstest]
// Five vias: 0.26 square (clean), 0.265 tall, 0.265 wide, 0.255 wide, 0.255 tall.
#[case::v1_1_h1("via/V1.1.h1.gds.gz", "TOP", vec!["V1.1"; 8], vec![])]
// A via that is not a square: two abutting squares making a 0.52 x 0.26 bar, an L with
// 0.26 arms, and one square drawn as two overlapping boxes (clean).
#[case::v1_1_h2("via/V1.1.h2.gds.gz", "TOP", vec!["V1.1"; 6], vec![])]
// A 4x4 array at 0.38 with one 0.355 row gap (four pairs); the same with three columns
// and sixteen vias in one row, neither an array.
// The array rules report one marker per array, however many of its pairs are short:
// an array's pitch is one structure, and the reading is the IHP round's (kept; the
// argument for one marker per pair is in the report, finding 2).
#[case::v1_2b_h1("via/V1.2b.h1.gds.gz", "TOP", vec!["V1.2b"], vec![])]
// A seventeenth via 0.355 beside a legal 4x4 (1); the same beside a 3x3 (clean); one
// 0.42 away, outside the array (clean); a 4x4 with its own 0.355 row gap and a
// seventeenth via 0.355 beside it (4 + 1).
// As the contact deck's h2: a via in the cluster but off the grid is the "finger"
// reading's (kept; report, findings 2 and 3).
#[case::v1_2b_h2("via/V1.2b.h2.gds.gz", "TOP", vec!["V1.2b"], vec![])]
// The array rule's "projecting >= 0.26": a via 0.30 beside a legal 4x4, level with a row
// (fires), a grid step up (0.255 of facing, exempt) and 0.15 up (0.11 of facing, exempt).
// The projecting condition reads over the array's own grid; the off-grid pairs it was
// drawn for are the finger reading's (kept; report, finding 3).
#[case::v1_2b_h3("via/V1.2b.h3.gds.gz", "TOP", vec![], vec![])]
// The same array with a 0.355 row gap straddling x = 20, x = 42 and y = 21.
// The array rules report one marker per array, however many of its pairs are short:
// an array's pitch is one structure, and the reading is the IHP round's (kept; the
// argument for one marker per pair is in the report, finding 2).
#[case::v1_2b_h4("via/V1.2b.h4.gds.gz", "TOP", vec!["V1.2b"; 3], vec![])]
// Metal1 flush with Via1 on one side, 0.06 above and below (clean, the rule asks 0); the
// via 0.005 outside it; no Metal1 at all; Metal1 drawn as the via's own square, which
// V1.3a allows and V1.3d does not.
#[case::v1_3a_h1("via/V1.3a.h1.gds.gz", "TOP", vec!["V1.3a", "V1.3a", "V1.3d"], vec![])]
// Line-end caps 0.055 past the via on tracks 0.34 (not a narrow line, clean), 0.35
// (clean) and 0.32 wide; 0.34 wide with a 0.06 cap is clean.
#[case::v1_3c_h1("via/V1.3c.h1.gds.gz", "TOP", vec!["V1.3c"], vec!["V1.3d"])]
// The same cap on a branch off a wide plate: 0.275 long is not a line end (clean), 0.28
// and 0.35 are - a via's branch is short below 0.28, a contact's below 0.24.
#[case::v1_3c_h2("via/V1.3c.h2.gds.gz", "TOP", vec!["V1.3c"; 2], vec!["V1.3d"])]
// A 0.035 side with 0.06 beside it (clean), with 0.055 beside it, a 0.04 side that does
// not trigger, two adjacent short sides, two opposite short sides (clean).
#[case::v1_3d_h1("via/V1.3d.h1.gds.gz", "TOP", vec!["V1.3d"; 2], vec![])]
// Metal2 over Via1 by 0.01 (clean), 0.005, and absent.
#[case::v1_4a_h1("via/V1.4a.h1.gds.gz", "TOP", vec!["V1.4a"; 2], vec![])]
// The same line-end reading against the metal above.
#[case::v1_4b_h1("via/V1.4b.h1.gds.gz", "TOP", vec!["V1.4b"], vec!["V1.4c"])]
// The same adjacent-side reading against the metal above: 0.035 triggers, 0.04 does not.
#[case::v1_4c_h1("via/V1.4c.h1.gds.gz", "TOP", vec!["V1.4c"], vec![])]
// A contact, Via1, Via2 and Via3 on one centre: the manual permits the stack.
#[case::v1_5_h1("via/V1.5.h1.gds.gz", "TOP", vec![], vec![])]
// Metal2 under Via2 by 0.01 (clean), 0.005, and flush - which Via1 is allowed and Via2
// is not.
#[case::v2_3b_h1("via/V2.3b.h1.gds.gz", "TOP", vec!["V2.3b"; 2], vec![])]
fn hardening_via(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("via", gds, topcell, &ignore), want, "{gds}");
}

// --- N+ implant (hardening/reports/gf180mcuD/nplus.md).  The manual's section 7.8.  A
// spacing rule's value is chosen by which well the P+ active sits in and how far it is
// from that well's edge; an extension rule's value the same way round the N+ one.  The
// classifier is a 0.429 um band, read per region, so an active straddling it is measured
// by both halves.
#[rstest]
// A 0.395 notch across the tile line x = 20, a 0.4 notch across x = 40 (clean) and a
// 0.395 gap across x = 21: one violation each for the notch and the gap, not two.
#[case::np_2_h1("nplus/NP.2.h1.gds.gz", "TOP", vec!["NP.2", "NP.2"], vec![])]
// One marker whose foot carries a legal butted N+/P+ pair and whose arm ends 0.155 from
// an unrelated P+ active in a deep well 8 um away.  The manual exempts the butting pair's
// own edge (NP.3d/3e), not every other active the marker comes near.
// The butted-pair exemption drops the whole marker, so the unrelated P+ diffusion 8 µm
// along the same island is not measured (kept and open; report, finding 3 - cutting the
// pair out of the marker instead invents walls the section's other rules then read).
#[case::np_3a_h1("nplus/NP.3a.h1.gds.gz", "TOP", vec![], vec![])]
// The same scene with the butted pair taken out: the control.
#[case::np_3a_h2("nplus/NP.3a.h2.gds.gz", "TOP", vec!["NP.3a"], vec![])]
// P+ actives in the 0.429 band along a P-well's wall (0.155 away, NP.3bi) and in its core
// (0.075 away, NP.3bii); the same two at 0.16 and 0.08 are clean.
#[case::np_3bi_h1("nplus/NP.3bi.h1.gds.gz", "TOP", vec!["NP.3bi", "NP.3bii"], vec![])]
// P+ actives wholly inside an N-well's 0.429 collar (0.155 away, NP.3ci) and wholly
// outside it (0.075 away, NP.3cii); the same two at 0.16 and 0.08 are clean.
#[case::np_3ci_h1("nplus/NP.3ci.h1.gds.gz", "TOP", vec!["NP.3ci", "NP.3cii"], vec![])]
// One 2 um P+ active straddling the collar's edge at x = 20.429, the marker 0.075 above
// it all along: the near half is NP.3ci's and the far half NP.3cii's.
#[case::np_3ci_h2("nplus/NP.3ci.h2.gds.gz", "TOP", vec!["NP.3ci", "NP.3cii"], vec![])]
// A butted N+/P+ edge round the corner from a P-channel gate, 0.297 corner to corner and
// facing none of the gate's walls: the manual's "parallel to gate" has nothing to measure.
#[case::np_4a_h1("nplus/NP.4a.h1.gds.gz", "TOP", vec![], vec![])]
// The same edge facing the gate's wall at 0.315 (fires) and at 0.32 (clean).
#[case::np_4a_h2("nplus/NP.4a.h2.gds.gz", "TOP", vec!["NP.4a"], vec![])]
// A gate the marker holds by 0.225 (two walls) and a gate whose poly bar the marker's
// right wall cuts: the overlap there is nothing, and so is the extension past the COMP.
#[case::np_5a_h1("nplus/NP.5a.h1.gds.gz", "TOP", vec!["NP.5a"; 3], vec!["NP.5b"])]
// Two identical taps with the marker 0.1 past the COMP: in the field NP.5b wants 0.16 and
// fires; 2 um inside an N-well NP.5dii wants 0.02 and is clean.
#[case::np_5b_h1("nplus/NP.5b.h1.gds.gz", "TOP", vec!["NP.5b"], vec![])]
// Four taps in one N-well: 0.015 past the COMP in the core (NP.5dii), 0.02 there (clean),
// 0.155 past a COMP in the 0.429 band (NP.5di), 0.16 there (clean).
#[case::np_5b_h2("nplus/NP.5b.h2.gds.gz", "TOP", vec!["NP.5di", "NP.5dii"], vec![])]
// A butted pair whose N+ half is 0.215 long (fires) and one 0.22 long; the three walls the
// P+ active shares with the COMP are an overlap of nothing and are not the rule's.
#[case::np_6_h1("nplus/NP.6.h1.gds.gz", "TOP", vec!["NP.6"], vec![])]
// Holes of 0.35 um2 (clean), 0.3465 and 0.348 across the tile line x = 20, the last drawn
// as four boxes that merge.
#[case::np_8b_h1("nplus/NP.8b.h1.gds.gz", "TOP", vec!["NP.8b", "NP.8b"], vec![])]
// Two unsalicided poly bars with the marker 0.175 past each.  The bare one is NP.9; the
// one marked as a resistor is a device whose body is meant to be bare, and PP.9 exempts
// its twin.
#[case::np_9_h1("nplus/NP.9.h1.gds.gz", "TOP", vec!["NP.9"], vec![])]
// (a) an unsalicided COMP the marker holds by 0.175: NP.10; (b) one the marker's wall cuts
// in half - an overlap of nothing, and the COMP's own edge is NP.5b's; (c) an unsalicided
// poly whose wall the marker's wall touches - a space of nothing, NP.7.
#[case::np_10_h1("nplus/NP.10.h1.gds.gz", "TOP", vec!["NP.10", "NP.10"], vec!["NP.5b", "NP.7"])]
// Butted N+/P+ edges 0.2 outside an N-well's wall (at the tile line x = 20), 0.2 inside it
// and 0.5 outside it.  NP.11 takes the collar outside the well, PP.11 the band inside it;
// between them the manual's "within 0.43 of the Nwell edge" is covered, and only the deck
// under test reports here.
#[case::np_11_h1("nplus/NP.11.h1.gds.gz", "TOP", vec!["NP.11"], vec![])]
// (a) a U of poly whose right leg the marker covers: 0.22 of air from the gate but 5 um
// along the poly, so the reach does not arrive; (b) a straight bar with the marker 0.315
// up it, one step inside the 0.319 reach.
#[case::np_12_h1("nplus/NP.12.h1.gds.gz", "TOP", vec!["NP.12"], vec![])]
fn hardening_nplus(
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
    assert_eq!(hardening("nplus", gds, topcell, &[]), want, "{gds}");
}

// --- Salicide block (hardening/reports/gf180mcuD/sab.md).  OTP_MK exempts a block from
// every rule of the section but SB.1, SB.6, SB.7, SB.8, SB.10 and SB.14; ESD_MK exempts
// SB.12; LVS_IO and ESD_MK exempt SB.16.  `min_width` reports one marker per wall.
#[rstest]
// 0.415 and 0.42 wide, both under OTP_MK: the width rule reads the drawn block, so the
// marker does not save the narrow one.  Two walls, two markers.
#[case::sb_1_h1("sab/SB.1.h1.gds.gz", "TOP", vec!["SB.1"; 2], vec![])]
// 0.415 pairs with OTP_MK over both (exempt), with it abutting one of them (fires), at
// 0.42 (clean), and a 0.415 notch in a U (fires).
#[case::sb_2_h1("sab/SB.2.h1.gds.gz", "TOP", vec!["SB.2"; 2], vec![])]
// An unrelated COMP 0.215 above a block (fires) and at 0.22; then one COMP bar with a
// block across it and a second block 0.215 above the bar, which that second block is not
// on - a space it owes by the manual, which neither deck measures because the bar
// overlaps the first block (report, finding 2).
#[case::sb_3_h1("sab/SB.3.h1.gds.gz", "TOP", vec!["SB.3"; 2], vec![])]
// A COMP abutting a block's edge and one 0.005 off it: two spaces under 0.22, the first
// of them a space of nothing (report, finding 1).
#[case::sb_3_h2("sab/SB.3.h2.gds.gz", "TOP", vec!["SB.3"; 2], vec![])]
// Contacts 0.145 from a block (fires), 0.15 (clean), touching its edge (a space of
// nothing, report, finding 1), on a block under OTP_MK (SB.8 alone) and on a bare block
// (SB.4 and SB.8).
#[case::sb_4_h1("sab/SB.4.h1.gds.gz", "TOP", vec!["SB.4"; 3], vec!["SB.8"; 2])]
// Field poly 0.295 from a block (fires), 0.3 (clean) and 0.295 from a block under
// OTP_MK (exempt).
#[case::sb_5a_h1("sab/SB.5a.h1.gds.gz", "TOP", vec!["SB.5a"], vec![])]
// Field poly abutting a block's edge and 0.005 off it: the first is a space of nothing
// (report, finding 1).
#[case::sb_5a_h2("sab/SB.5a.h2.gds.gz", "TOP", vec!["SB.5a"; 2], vec![])]
// A poly bar wholly on a COMP plate, 0.275 from a block (fires) and 0.28 (clean).
#[case::sb_5b_h1("sab/SB.5b.h1.gds.gz", "TOP", vec!["SB.5b"], vec![])]
// A poly crossing a COMP's edge: its field strip is 0.29 from the block (SB.5a) and its
// COMP strip 0.59 (no SB.5b); the same at 0.30 / 0.60 is clean.
#[case::sb_5_h1("sab/SB.5.h1.gds.gz", "TOP", vec!["SB.5a"], vec![])]
// One poly line cut in two by a COMP bar: a block on the left piece, a second block
// 0.295 from the right piece, which no block is on - SB.5a, the section's "unrelated"
// being read of the piece.
#[case::sb_5_h2("sab/SB.5.h2.gds.gz", "TOP", vec!["SB.5a"], vec![])]
// A COMP island wholly inside a block, which never runs past it (SB.7 by the manual,
// silent in both decks - report, finding 3), and a COMP crossing a block on a coincident
// wall, where the block's extension is nothing (SB.6).
#[case::sb_7_h1("sab/SB.7.h1.gds.gz", "TOP", vec!["SB.6"], vec!["SB.7"])]
// The same two on poly: the island (SB.10, silent - report, finding 3) and the
// coincident wall, which here is read as the poly's fault and not the block's (report,
// finding 4).
#[case::sb_10_h1("sab/SB.10.h1.gds.gz", "TOP", vec!["SB.10"; 2], vec![])]
// A 0.215 overlap with poly: bare (fires), under ESD_MK (exempt), with ESD_MK over the
// block's far half so the overlap itself lies outside the marker (fires by the manual,
// report, finding 5), and with ESD_MK abutting the block (fires).
#[case::sb_12_h1("sab/SB.12.h1.gds.gz", "TOP", vec!["SB.12"; 3], vec![])]
// 1.96 µm² (fires), 2.002 (clean), 1.96 under OTP_MK (exempt) and a ring of 1.985 µm²
// whose outline is 2.25 - the area is the block's own.
#[case::sb_13_h1("sab/SB.13.h1.gds.gz", "TOP", vec!["SB.13"; 2], vec![])]
// A butted N+/P+ unsalicided poly - a poly diode - which has no corner to keep clear and
// is exempt in both decks; and two unsalicided regions 0.5 by 0.5 apart, inside the
// 0.56 square the rule names although 0.707 away.
#[case::sb_14a_h1("sab/SB.14a.h1.gds.gz", "TOP", vec!["SB.14a"], vec![])]
// Unsalicided N+ poly 0.5 by 0.5 from a P-channel gate's corner (inside the square) and
// 0.565 by 0.565 (outside it).
#[case::sb_14b_h1("sab/SB.14b.h1.gds.gz", "TOP", vec!["SB.14b"], vec![])]
// An implant that lies on no unsalicided poly, 0.175 from one (fires) and 0.18 (clean).
#[case::sb_15a_h1("sab/SB.15a.h1.gds.gz", "TOP", vec!["SB.15a"], vec![])]
// A poly resistor's two N+ heads 0.315 from the block (fires twice) and at 0.32; and one
// N+ covering the whole bar, which contains the block-on-poly region rather than facing
// it and is no space.
#[case::sb_15b_h1("sab/SB.15b.h1.gds.gz", "TOP", vec!["SB.15b"; 2], vec![])]
// A block on a gate: bare (fires), under LVS_IO, ESD_MK or OTP_MK (exempt), with RES_MK
// taking the transistor away (exempt), with LVS_IO over half the block (exempt - report,
// finding 5) and bare again across the 20 µm tile line (fires).
#[case::sb_16_h1("sab/SB.16.h1.gds.gz", "TOP", vec!["SB.16"; 2], vec![])]
// A block beside the gate, sharing its edge (SB.16) and 0.005 clear of it (no SB.16).
// Both blocks stand on the COMP beside the gate, whose field and COMP poly are a space
// of nothing and 0.005 from them: SB.5a twice each and SB.5b once each (report,
// finding 1).
#[case::sb_16_h2("sab/SB.16.h2.gds.gz", "TOP", vec!["SB.16"], vec!["SB.5a", "SB.5a", "SB.5a", "SB.5a", "SB.5b", "SB.5b"])]
fn hardening_sab(
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
    assert_eq!(hardening("sab", gds, topcell, &[]), want, "{gds}");
}

// --- P+ implant (hardening/reports/gf180mcuD/pplus.md).  The manual's section 7.9, the
// mirror of 7.8 with the wells the other way round: the P+ rules read the N-well's inner
// band where the N+ rules read its outer collar, and the P-well's outer collar where they
// read its inner band.
#[rstest]
// One marker whose foot carries a legal butted P+/N+ pair and whose arm ends 0.155 from an
// unrelated N+ active 8 um away.
// As the nplus deck's NP.3a.h1: the butted-pair exemption is whole-marker (kept and
// open; report, finding 6).
#[case::pp_3a_h1("pplus/PP.3a.h1.gds.gz", "TOP", vec![], vec![])]
// The same scene with the butted pair taken out: the control.
#[case::pp_3a_h2("pplus/PP.3a.h2.gds.gz", "TOP", vec!["PP.3a"], vec![])]
// N+ actives in an N-well's 0.429 rim (0.155 away, PP.3cii) and in its core (0.075 away,
// PP.3ci); the same two at 0.16 and 0.08 are clean.
#[case::pp_3ci_h1("pplus/PP.3ci.h1.gds.gz", "TOP", vec!["PP.3ci", "PP.3cii"], vec![])]
// A butted P+/N+ edge round the corner from an N-channel gate, 0.297 corner to corner and
// facing none of the gate's walls.
#[case::pp_4a_h1("pplus/PP.4a.h1.gds.gz", "TOP", vec![], vec![])]
// The same edge facing the gate's wall at 0.315 (fires) and at 0.32 (clean).
#[case::pp_4a_h2("pplus/PP.4a.h2.gds.gz", "TOP", vec!["PP.4a"], vec![])]
// A gate the marker holds by 0.225 (two walls) and a gate whose poly bar the marker's
// right wall cuts: the overlap is nothing, and so is the extension past the COMP, which
// an N-well tap owes to both PP.5b and PP.5dii.
#[case::pp_5a_h1("pplus/PP.5a.h1.gds.gz", "TOP", vec!["PP.5a"; 3], vec!["PP.5b", "PP.5dii"])]
// A P-tap just outside an N-well whose marker wall is drawn on the well wall: the COMP is
// outside the well, so this is PP.5d's case and the marker is 0 from the well - PP.5dii.
#[case::pp_5b_h1("pplus/PP.5b.h1.gds.gz", "TOP", vec!["PP.5dii"], vec![])]
// Four taps in one P-well in a deep well: 0.015 past the COMP in the core (PP.5ci), 0.02
// there (clean), 0.155 past a COMP in the 0.429 band (PP.5cii), 0.16 there (clean).
#[case::pp_5ci_h1("pplus/PP.5ci.h1.gds.gz", "TOP", vec!["PP.5ci", "PP.5cii"], vec![])]
// Two identical P-taps in the field, the marker 0.015 past the COMP.  Section 7.9 knows
// nothing of guard rings, and the second one is drawn under GUARD_RING_MK.
// The guard-ring exemption is upstream's and drops the marker whole; the manual has no
// such exemption, and section 12's own rules cover the ring (kept and open; report,
// finding 1).
#[case::pp_5di_h1("pplus/PP.5di.h1.gds.gz", "TOP", vec!["PP.5di"], vec![])]
// A butted pair whose P+ half is 0.215 long (fires) and one 0.22 long.
#[case::pp_6_h1("pplus/PP.6.h1.gds.gz", "TOP", vec!["PP.6"], vec![])]
// Two unsalicided poly bars with the marker 0.175 past each; the one marked as a resistor
// is exempt, the bare one is PP.9.
#[case::pp_9_h1("pplus/PP.9.h1.gds.gz", "TOP", vec!["PP.9"], vec![])]
// (a) an unsalicided COMP the marker holds by 0.175: PP.10; (b) one the marker's wall cuts
// in half - an overlap of nothing, and the COMP's own edge is PP.5di's; (c) an unsalicided
// poly whose wall the marker's wall touches - a space of nothing, PP.7.
#[case::pp_10_h1("pplus/PP.10.h1.gds.gz", "TOP", vec!["PP.10", "PP.10"], vec!["PP.5di", "PP.7"])]
// Butted P+/N+ edges 0.2 inside an N-well's wall, 0.2 outside it (at the tile line x = 20)
// and 0.5 inside it.  PP.11 takes the band inside the well, NP.11 the collar outside it.
#[case::pp_11_h1("pplus/PP.11.h1.gds.gz", "TOP", vec!["PP.11"], vec![])]
// (a) a U of poly whose right leg the marker covers: 0.22 of air from the gate but 5 um
// along the poly; (b) a straight bar with the marker 0.315 up it.
#[case::pp_12_h1("pplus/PP.12.h1.gds.gz", "TOP", vec!["PP.12"], vec![])]
fn hardening_pplus(
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
    assert_eq!(hardening("pplus", gds, topcell, &[]), want, "{gds}");
}

// --- ESD implant (hardening/reports/gf180mcuD/esd.md).  The implant is a 5 V/6 V option:
// it lives on Dualgate, encloses the N+ active it protects, butts against a P+ one and
// never lies on it.  `min_gate_length` reports one marker per wall.
#[rstest]
// An unrelated N+ active 0.595 from the implant (fires), 0.6 (clean), butted against it
// (a space of nothing, report, finding 1) and crossing its edge (an active the implant
// lies on, not one it stands off).
#[case::esd_3a_h1("esd/ESD.3a.h1.gds.gz", "TOP", vec!["ESD.3a"; 2], vec![])]
// A P+ active butted against the implant (the zero space ESD.3b asks for, clean), one
// 0.005 under it (ESD.3b, ESD.7, and ESD.4b for an overlap of 0.005) and one 0.005 clear
// of it (ESD.8).
#[case::esd_3b_h1("esd/ESD.3b.h1.gds.gz", "TOP", vec!["ESD.3b", "ESD.7"], vec!["ESD.4b", "ESD.8"])]
// The implant 0.235 past the N+ active's right edge (fires), and the same wall shared
// with a butted P+ active, which is the edge ESD.3b sets to zero space and owes no
// extension (clean).
#[case::esd_4a_h1("esd/ESD.4a.h1.gds.gz", "TOP", vec!["ESD.4a"], vec![])]
// A second poly 0.445 from the implant's edge: lifted off the active (not a gate, clean),
// across the active under RES_MK (not a transistor, clean), across it bare (fires) and
// bare at 0.45 (clean).
#[case::esd_6_h1("esd/ESD.6.h1.gds.gz", "TOP", vec!["ESD.6"], vec![])]
// A step in the implant's right side whose inner corner stands 0.206 from the gate's and
// the N+ active's shared upper right corner, with no two walls facing across it: the
// extension is under both values there, and gdscheck reads neither (report, finding 3).
// A step in the implant's boundary 0.206 from the corner the gate and the active share:
// the edge-layer extensions read projection, and no two walls face across a step (kept
// and open; report, finding 3 - KLayout catches ESD.4a there and misses ESD.6 itself).
#[case::esd_6_h2("esd/ESD.6.h2.gds.gz", "TOP", vec![], vec![])]
// The device's own N+ 0.1 inside the implant (contained, clean), an unrelated P+ 0.295
// outside it (fires), one at 0.3 and one crossing its edge.
#[case::esd_8_h1("esd/ESD.8.h1.gds.gz", "TOP", vec!["ESD.8"], vec![])]
// Dualgate over the whole implant (clean), over its left half (a 3.3 V half of a 5 V
// implant, which the manual forbids and both decks allow - report, finding 2) and
// abutting it (fires).
#[case::esd_9_h1("esd/ESD.9.h1.gds.gz", "TOP", vec!["ESD.9"; 2], vec![])]
// LVS_IO absent (clean - the rule is about a marker that covers part of an ESD active),
// over the active's left half (fires), abutting its edge (covering nothing, clean) and
// over the whole active.
#[case::esd_10_h1("esd/ESD.10.h1.gds.gz", "TOP", vec!["ESD.10"], vec![])]
// 0.795 gates: on an ESD device under Dualgate (fires, one marker per wall), on a
// transistor 2 µm clear of the implant (clean), on one whose poly touches the implant's
// edge (fires) and on an ESD device with no Dualgate (no ESD.pl, and ESD.9).
#[case::esd_pl_h1("esd/ESD.pl.h1.gds.gz", "TOP", vec!["ESD.pl"; 4], vec!["ESD.9"])]
fn hardening_esd(
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
    assert_eq!(hardening("esd", gds, topcell, &[]), want, "{gds}");
}

// --- Metal 1 to 4 (hardening/reports/gf180mcuD/metal.md).  Section 7.13's rules are per
// level and read the drawn datatype alone; `min_width` reports one marker per wall, a
// space or a notch one per pair.
#[rstest]
// Six 0.225 bars, each in a different relation to SRAMCORE and the marker that decides
// whether the core is the 3.3 V kind: bare, in a bare core, in a core Dualgate covers,
// in one Dualgate only abuts, in one under V5_XTOR alone, in one Dualgate covers half
// of.  Section 11.2 is for "3.3V SRAM cells without marking layer V5_XTOR" and gives
// them M1.1 = 0.22, so every core but the V5_XTOR one is its cell and 0.225 clears it;
// the V5_XTOR core is 11.1's, which has no M1.1, so chapter 7's 0.23 is its own.  Two
// bars fire, one marker per wall.
#[case::m1_1_h1("metal/M1.1.h1.gds.gz", "TOP", vec!["M1.1"; 4], vec![])]
// A 0.225 bar half inside a bare core - the half outside is still 0.225 wide - and one
// the core abuts without covering.
#[case::m1_1_h2("metal/M1.1.h2.gds.gz", "TOP", vec!["M1.1"; 4], vec![])]
// 0.1 squares on every datatype the section does not read (slot, dummy, blocked, label,
// resistor) and a dummy shape 0.2 from a drawn one, on all four levels: nothing here is
// this deck's.
#[case::m1_1_h3("metal/M1.1.h3.gds.gz", "TOP", vec![], vec![])]
// What "wide" is, on all four levels: a 10.0 square and a 30 x 9.995 bar are not over 10
// either way; a 10.005 square and a 30 x 10.005 bar are.  Each has a neighbour 0.295 off.
#[case::m1_2b_h1("metal/M1.2b.h1.gds.gz", "TOP", each(&[("M1.2b", 2), ("M2.2b", 2), ("M3.2b", 2), ("M4.2b", 2)]), vec![])]
// A narrow stub on a wide plate: the stub is not wide, and a neighbour facing it owes
// M1.2a's 0.23, not M1.2b's 0.3.  Only the plate whose own wall faces a neighbour fires.
#[case::m1_2b_h2("metal/M1.2b.h2.gds.gz", "TOP", vec!["M1.2b"], vec![])]
// A 0.295 slot cut into a wide plate - a space between two wide regions of one drawn
// shape (report, finding 1) - and the same slot in a 9 µm plate, which is wide nowhere.
#[case::m1_2b_h3("metal/M1.2b.h3.gds.gz", "TOP", vec!["M1.2b"], vec![])]
// The same plate and neighbour four times, the gap opening on x = 20, 21, 40 and 42.
#[case::m1_2b_h4("metal/M1.2b.h4.gds.gz", "TOP", vec!["M1.2b"; 4], vec![])]
// Two wide plates 0.295 apart: one gap, one violation, whichever plate it is read from
// (report, finding 2).
#[case::m1_2b_h5("metal/M1.2b.h5.gds.gz", "TOP", vec!["M1.2b"], vec![])]
// 0.38 x 0.38 is exactly the 0.1444 µm² and 0.38 x 0.375 one grid step under, per level.
#[case::m1_3_h1("metal/M1.3.h1.gds.gz", "TOP", each(&[("M1.3", 1), ("M2.3", 1), ("M3.3", 1), ("M4.3", 1)]), vec![])]
// A ring drawn as four bars holds 0.12 µm² of metal where its outline covers 0.16: the
// area is the metal, not the ground.  The solid 0.4 square beside it is clean.
#[case::m1_3_h2("metal/M1.3.h2.gds.gz", "TOP", vec!["M1.3"], each(&[("M1.1", 8), ("M1.2a", 2)]))]
// A 0.25 bar: legal on Metal1 (0.23) and too narrow on every level above it (0.28).
#[case::m2_1_h1("metal/M2.1.h1.gds.gz", "TOP", each(&[("M2.1", 2), ("M3.1", 2), ("M4.1", 2)]), vec![])]
// The same split in space: a 0.25 gap and a 0.25 notch, legal on Metal1 only.
#[case::m2_2a_h1("metal/M2.2a.h1.gds.gz", "TOP", each(&[("M2.2a", 2), ("M3.2a", 2), ("M4.2a", 2)]), vec![])]
fn hardening_metal(
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
    assert_eq!(hardening("metal", gds, topcell, &[]), want, "{gds}");
}

// --- MetalTop (hardening/reports/gf180mcuD/metaltop.md).  Variant D is the 11K stack and
// its top metal is Metal5, so section 7.13 stops at Metal4 and these four rules take over.
#[rstest]
// 0.435 bars on the drawn and the dummy datatype (both metal), on the slot, blocked,
// label and resistor datatypes (none of them metal), and a 0.3 drawn bar abutting a 0.3
// dummy one, which is one 0.6 conductor.
#[case::mt_1_h1("metaltop/MT.1.h1.gds.gz", "TOP", vec!["MT.1"; 4], vec![])]
// A 0.3 and a 0.45 bar: the first is under MT.1's 0.44, the second over it.  The `metal`
// deck says nothing about either - Metal5 is not one of its levels.
#[case::mt_1_h2("metaltop/MT.1.h2.gds.gz", "TOP", vec!["MT.1"; 2], vec![])]
// Drawn to dummy at 0.455 is a space; drawn to a blocked-fill marker at the same gap is
// not; drawn to dummy at 0.46 is the bound.
#[case::mt_2a_h1("metaltop/MT.2a.h1.gds.gz", "TOP", vec!["MT.2a"], vec![])]
// What "wide" is: 10.0 and 30 x 9.995 are not, 10.005 and 30 x 10.005 are, each with a
// neighbour 0.595 off.
#[case::mt_2b_h1("metaltop/MT.2b.h1.gds.gz", "TOP", vec!["MT.2b"; 2], vec![])]
// A chamfered plate and a stubbed one, both wide by their dimensions: the neighbour
// facing the stub owes MT.2a's 0.46, not MT.2b's 0.6.
#[case::mt_2b_h2("metaltop/MT.2b.h2.gds.gz", "TOP", vec!["MT.2b"; 2], vec![])]
// A 0.595 slot in a wide plate (report, finding 1), and the same slot in a 9 µm plate.
#[case::mt_2b_h3("metaltop/MT.2b.h3.gds.gz", "TOP", vec!["MT.2b"], vec![])]
// The same plate and neighbour four times, the gap opening on x = 20, 21, 40 and 42.
#[case::mt_2b_h4("metaltop/MT.2b.h4.gds.gz", "TOP", vec!["MT.2b"; 4], vec![])]
// 0.75² clears the 0.5625 µm² and 0.7² does not.
#[case::mt_4_h1("metaltop/MT.4.h1.gds.gz", "TOP", vec!["MT.4"], vec![])]
// A ring drawn as four bars: the area is the metal it holds, not the ground its outline
// covers.
#[case::mt_4_h2("metaltop/MT.4.h2.gds.gz", "TOP", vec!["MT.4"], each(&[("MT.1", 8), ("MT.2a", 2)]))]
fn hardening_metaltop(
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
    assert_eq!(hardening("metaltop", gds, topcell, &[]), want, "{gds}");
}

/// The rule ids a case expects, as owned strings.
fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

/// Every rule of one level's slotting table, for all five levels.
fn per_level(suffixes: &[&str]) -> Vec<String> {
    (1..=5)
        .flat_map(|n| suffixes.iter().map(move |s| format!("MSLOT{n}.{s}")))
        .collect()
}

// --- Metal slotting (hardening/reports/gf180mcuD/mslot.md).  Section 14.6.3 of the
// manual, whose own table has three things neither this deck nor the runset carries: the
// 30 µm *maximum* on slot space in `.4`, the `.10` that forbids a slot mark on a hole in
// the metal, and the Metal3-inside-FuseTop region in `.9`'s list.  `.6` is in the manual's
// own Appendix B of rules not coded.  Counts: `forbidden` gives one marker per region,
// `min_dim` / `min_length` / `max_length` one per mark, `min_space` one per pair, and
// `min_enclosure` one per short approach.
#[rstest]
// One 22 x 150 µm plate per level carrying an L (`.0`), a 1.995 µm wide mark (`.2`), a
// 9.995 µm long one (`.3`) and a pair 9.995 µm apart (`.4`).
#[case::mslot_all_h1("mslot/MSLOT.all.h1.gds.gz", "TOP", per_level(&["0", "2", "3", "4"]))]
// `.2` is the mark's short bbox side and `.3` its long one, whichever way round it lies:
// 1.995 x 20 and 20 x 1.995 both fire `.2`, and a 2 x 2 square fires `.3` alone.
#[case::mslot_dim_h1("mslot/MSLOT.dim.h1.gds.gz", "TOP", ids(&["MSLOT1.2", "MSLOT1.2", "MSLOT1.3"]))]
// `.3`'s two bounds: 10 and 250 µm are legal, 9.995 and 250.005 are not.
#[case::mslot_len_h1("mslot/MSLOT.len.h1.gds.gz", "TOP", ids(&["MSLOT1.3", "MSLOT1.3"]))]
// `.4` at 10 µm (clean) and 9.995; and an L 5 µm from a rectangle, which is `.0`'s alone -
// only rectangles reach the rules after `.0`.
#[case::mslot_space_h1("mslot/MSLOT.space.h1.gds.gz", "TOP", ids(&["MSLOT1.0", "MSLOT1.4"]))]
// The same 9.995 µm gap across x = 20, across x = 42, and with a mark's edge on x = 20.
#[case::mslot_space_h2("mslot/MSLOT.space.h2.gds.gz", "TOP", ids(&["MSLOT1.4"; 3]))]
// `.4`'s maximum: marks 30 µm apart are at the bound and 30.005 apart are over it.  Both
// tools are silent - the deck and the runset carry the minimum alone (report, finding 1).
// The 30 um *maximum* between marks is in neither deck nor runset, and the engine has no
// reading for "every region of a layer within reach of another of the same layer" (kept
// and open; report, finding 1).
#[case::mslot_space_h3("mslot/MSLOT.space.h3.gds.gz", "TOP", ids(&[]))]
// `.5` at 9.995 µm on one side, on all five levels, and 10 µm all round for the control.
#[case::mslot_enc_h1("mslot/MSLOT.enc.h1.gds.gz", "TOP", per_level(&["5"]))]
// A mark crossing the metal's edge, one with no metal at all, one on the dummy datatype,
// and one 8 µm from the wall of a notch.  The runset reports only the last (report,
// finding 5: gdscheck is right).
#[case::mslot_enc_h2("mslot/MSLOT.enc.h2.gds.gz", "TOP", ids(&["MSLOT1.5"; 4]))]
// A legal mark in the middle of a hole in the metal, 10 µm from every wall of it: the
// manual's `.10`, which neither deck carries.  gdscheck reports it as `.5` instead, which
// is the right answer under the wrong id (report, finding 2).
#[case::mslot_hole_h1("mslot/MSLOT.hole.h1.gds.gz", "TOP", ids(&["MSLOT1.10", "MSLOT1.5"]))]
// The per-level via map: `.7` is the via above the metal, `.8` the one below, and Metal5
// has no `.7`.
#[case::mslot_via_h1("mslot/MSLOT.via.h1.gds.gz", "TOP", ids(&["MSLOT1.7", "MSLOT1.8", "MSLOT2.7", "MSLOT2.8", "MSLOT3.7", "MSLOT3.8", "MSLOT4.7", "MSLOT4.8", "MSLOT5.8"]))]
// A via 0.2 µm away (clean), one sharing part of the mark's edge, one wholly inside the
// mark, one straddling its edge, and a Via2 beside a Metal1 mark (the wrong level).  Three
// violations, none of which gdscheck reports (report, findings 3 and 4).
#[case::mslot_via_h2("mslot/MSLOT.via.h2.gds.gz", "TOP", ids(&["MSLOT1.7"; 3]))]
// A via bar sharing the whole of the mark's right edge, a contact sharing the whole of its
// left edge, and a via touching one corner.  gdscheck reports the corner and neither edge
// (report, finding 3).
#[case::mslot_via_h3("mslot/MSLOT.via.h3.gds.gz", "TOP", ids(&["MSLOT1.7", "MSLOT1.7", "MSLOT1.8"]))]
// `.9`'s six keep-out regions at 9.995 µm from a mark, the same pair at 10 (clean), and the
// five layer combinations that build no keep-out at all.
#[case::mslot_dont_h1("mslot/MSLOT.dont.h1.gds.gz", "TOP", ids(&["MSLOT1.9"; 6]))]
// A pad opening lying on the mark, one sharing its edge, one whose grown keep-out shares
// its edge, one at the 10 µm bound (clean), and a Metal3 island inside FuseTop at 9.995 -
// the fifth region the manual's `.9` names (report, findings 3, 4 and 6).
#[case::mslot_dont_h2("mslot/MSLOT.dont.h2.gds.gz", "TOP", ids(&["MSLOT1.9"; 4]))]
// What "wider than 30 µm" is: 30 x 30 (clean), 30.005 x 30.005, 30.005 x 30 (clean), a
// 40 x 20 plate and an L of two 40 x 20 arms (clean - no 30 µm square fits).
#[case::mslot_wide_h1("mslot/MSLOT.wide.h1.gds.gz", "TOP", ids(&["MSLOT1.1"]))]
// The slot that relieves it: a 2 µm mark leaves two 30 µm halves in a 62 µm plate (clean)
// and two 30.005 halves in a 62.01 one.
#[case::mslot_wide_h2("mslot/MSLOT.wide.h2.gds.gz", "TOP", ids(&["MSLOT1.1"; 2]))]
// The via keep-out relieves it too: a 1.7 µm via bar grows to 2.1 and leaves 29.955 µm
// halves (clean); a 1.5 µm bar grows to 1.9 and leaves 30.055.
#[case::mslot_wide_h3("mslot/MSLOT.wide.h3.gds.gz", "TOP", ids(&["MSLOT1.1"; 2]))]
// And so does a keep-out: a 2 µm pad opening over the top metal grows to 12 (clean); the
// same opening with no top metal under it is no keep-out, and the plate is one region.
#[case::mslot_wide_h4("mslot/MSLOT.wide.h4.gds.gz", "TOP", ids(&["MSLOT1.1"]))]
// The 15 µm shrink and grow across the tile lines: a 30.005 µm square over x = 7 to 35,
// one over x = 42, and one at (1000, 1000).
#[case::mslot_wide_h5("mslot/MSLOT.wide.h5.gds.gz", "TOP", ids(&["MSLOT1.1"; 3]))]
// One 30.005 µm square per level, all at the same place.
#[case::mslot_wide_h6("mslot/MSLOT.wide.h6.gds.gz", "TOP", per_level(&["1"]))]
fn hardening_mslot(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<String>) {
    let mut want = expected;
    want.sort();
    assert_eq!(hardening("mslot", gds, topcell, &[]), want, "{gds}");
}

// --- Memory cell (hardening/reports/gf180mcuD/mcell.md).  Section 7.17 is four rules over
// one marker layer and the deck carries all four.  `min_width` reports one marker per wall
// where the runset reports one per edge pair; everything else agrees marker for marker.
#[rstest]
// MC.3's bound read twice: 0.5 x 0.7 and 0.4 x 0.875 are exactly 0.35 µm², 0.5 x 0.695 and
// 0.4 x 0.87 are under it.  Then two 0.4 µm boxes abutting into one 0.32 µm² region (one
// violation, not two) and two abutting into 0.36 (clean).
#[case::mc_area_h1("mcell/MC.area.h1.gds.gz", "TOP", vec!["MC.3"; 3])]
// MC.1 at 0.4 µm (clean) and 0.395 - two walls, two markers; MC.2's space and notch at 0.4
// (clean) and 0.395.
#[case::mc_width_h1("mcell/MC.width.h1.gds.gz", "TOP", vec!["MC.1", "MC.1", "MC.2", "MC.2"])]
// MC.4 on a 0.5 x 0.7 hole (exactly 0.35 µm², clean) and a 0.5 x 0.695 one; then a C whose
// inside reaches the outside through a 0.4 µm channel and through a 0.395 one - neither is
// a hole, and the narrow one is MC.2's space between the two ends.
#[case::mc_hole_h1("mcell/MC.hole.h1.gds.gz", "TOP", vec!["MC.2", "MC.4"])]
// The marker is 11/17: the same 0.395 µm bar on 11/39 (MVSD), 11/16 and 11/18 is silent.
#[case::mc_layer_h1("mcell/MC.layer.h1.gds.gz", "TOP", vec!["MC.1", "MC.1"])]
fn hardening_mcell(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("mcell", gds, topcell, &[]), want, "{gds}");
}

// --- MIM capacitor, option B (hardening/reports/gf180mcuD/mim_b.md).  Everything the
// deck measures the bottom plate with is a layer nobody draws: FuseTop grown by 1.06 µm
// and clipped to Metal4.  The expected values are the manual's 1.06, not the engine's.
#[rstest]
// A capacitor whose metal runs 2.0 past the plate, alone on the layout: the metal outside
// the virtual plate is the plate's own, and there is nothing to measure against it.
#[case::mimtm_1_h1("mim_b/MIMTM.1.h1.gds.gz", "TOP", vec![])]
// The same overhang with routing metal 1.2 (clean) and 1.195 (fires) from the virtual
// plate's edge.  Report finding 1: the virtual plate is grown by 1.065, so the 1.2 gap is
// read as 1.195 and reported too.
#[case::mimtm_1_h2("mim_b/MIMTM.1.h2.gds.gz", "TOP", vec!["MIMTM.1"])]
// 1.195 gaps to the virtual plate straddling x = 20 and x = 42.
#[case::mimtm_1_h3("mim_b/MIMTM.1.h3.gds.gz", "TOP", vec!["MIMTM.1"; 2])]
// Bottom-plate vias held by 0.395 (fires) and 0.4; a via outside the metal sharing its
// edge, and a via held by 0.395 in routing metal far from any plate - neither is within
// the 1.06 oversize, so neither is this rule's.
#[case::mimtm_2_h1("mim_b/MIMTM.2.h1.gds.gz", "TOP", vec!["MIMTM.2"])]
// A via in the metal but past the 1.06 oversize (not this rule's, though the metal holds
// it by 0.24), and a via crossing the metal's outer edge (the crossing half, fires).
#[case::mimtm_2_h2("mim_b/MIMTM.2.h2.gds.gz", "TOP", vec!["MIMTM.2"])]
// A via whose inner edge sits exactly on the 1.06 oversize - touching it is not being
// within it - and one 0.01 inside it, both held by 0.395.  Only the second is a bottom-
// plate via.  Report finding 1: the 1.065 grow makes the first one fire as well.
#[case::mimtm_2_h3("mim_b/MIMTM.2.h3.gds.gz", "TOP", vec!["MIMTM.2"])]
// A plate with no metal under it at all: no bottom plate, so no 0.6 overlap.
#[case::mimtm_3_h1("mim_b/MIMTM.3.h1.gds.gz", "TOP", vec!["MIMTM.3"])]
// A plate the metal covers only half of (the crossing half), and one overlapped by 0.595.
#[case::mimtm_3_h2("mim_b/MIMTM.3.h2.gds.gz", "TOP", vec!["MIMTM.3"; 2])]
// The metal 2.0 past the plate: the overlap is the oversize's 1.06, well over 0.6.
#[case::mimtm_3_h3("mim_b/MIMTM.3.h3.gds.gz", "TOP", vec![])]
// A via straddling the top plate's edge: the plate overlaps it nowhere by 0.4.
#[case::mimtm_4_h1("mim_b/MIMTM.4.h1.gds.gz", "TOP", vec!["MIMTM.4"])]
// A via abutting the top plate's edge from outside - a spacing to the plate of nothing.
// Report finding 2: gdscheck reads a shared edge as no measurement and stays silent.
#[case::mimtm_4_h2("mim_b/MIMTM.4.h2.gds.gz", "TOP", vec!["MIMTM.5"])]
// A via 0.395 from the plate on the bottom metal (fires), and one 0.395 from a plate
// whose metal stops flush with it, landing on nothing - not a bottom-plate via.  The
// flush metal is an overlap of nothing, so MIMTM.3 fires there.
#[case::mimtm_5_h1("mim_b/MIMTM.5.h1.gds.gz", "TOP", vec!["MIMTM.3", "MIMTM.5"])]
// A via on the bottom metal whose corner sits on the plate's corner: a spacing of nothing.
#[case::mimtm_5_h2("mim_b/MIMTM.5.h2.gds.gz", "TOP", vec!["MIMTM.5"])]
// A U-shaped top plate with a 0.595 opening (fires) and one with a 0.6 opening.
#[case::mimtm_6_h1("mim_b/MIMTM.6.h1.gds.gz", "TOP", vec!["MIMTM.6"])]
// CAP_MK exactly on the plate (clean), 0.5 short of its edge (fires), as a frame with a
// hole over the plate (fires), and as two abutting boxes whose union covers it (clean).
#[case::mimtm_7_h1("mim_b/MIMTM.7.h1.gds.gz", "TOP", vec!["MIMTM.7"; 2])]
// 25.0 µm² exactly and 24.975, each as a rectangle and as an L drawn from two boxes.
#[case::mimtm_8a_h1("mim_b/MIMTM.8a.h1.gds.gz", "TOP", vec!["MIMTM.8a"; 2])]
// 100 × 100 = 10000 exactly (clean) and 100.005 × 100 = 10000.5, which is over MIMTM.8b
// and, being the whole of its bottom plate's total, over MIMTM.11 as well.
#[case::mimtm_8b_h1("mim_b/MIMTM.8b.h1.gds.gz", "TOP", vec!["MIMTM.11", "MIMTM.8b"])]
// Two vias on the plate 0.495 apart (fires) and 0.5.
#[case::mimtm_9_h1("mim_b/MIMTM.9.h1.gds.gz", "TOP", vec!["MIMTM.9"])]
// A via on the plate and one straddling its edge, 0.495 apart: the straddler is not a via
// on the plate, so the pitch rule has only one - it is MIMTM.4's crossing instead.
#[case::mimtm_9_h2("mim_b/MIMTM.9.h2.gds.gz", "TOP", vec!["MIMTM.4"])]
// A Via3 in the bottom plate's 1.06 ring (report finding 4: both tools read the rule as
// the metal under FuseTop only), and one straddling the plate's edge, which fires.
#[case::mimtm_10_h1("mim_b/MIMTM.10.h1.gds.gz", "TOP", vec!["MIMTM.10"])]
// Two 50 × 100 capacitors on one bottom plate: 10000 µm² exactly.
#[case::mimtm_11_h1("mim_b/MIMTM.11.h1.gds.gz", "TOP", vec![])]
// The same pair at 10000.25 µm², over the cap.  Neither plate is near it on its own.
#[case::mimtm_11_h2("mim_b/MIMTM.11.h2.gds.gz", "TOP", vec!["MIMTM.11"])]
// Two 6000 µm² capacitors on separate bottom plates: 12000 between them, but the cap is
// per plate.
#[case::mimtm_11_h3("mim_b/MIMTM.11.h3.gds.gz", "TOP", vec![])]
fn hardening_mim_b(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("mim_b", gds, topcell, &[]), expected, "{gds}");
}

// --- Cu pillar (hardening/reports/gf180mcuD/cup.md).  The deck reads the drawn metal a
// PAD marker lands on and no guard ring claims, and keeps the measurements that reach the
// pad.  `min_width` reports one marker per wall, so one narrow line is two.
#[rstest]
// Four 0.995 bars under pads: GUARD_RING_MK abutting the bar (exempt), 0.505 clear of it
// (fires), touching the pad but not the metal (fires), and abutting a Metal5 bar (exempt).
#[case::cup_2_h1("cup/CUP.2.h1.gds.gz", "TOP", vec!["CUP.2"; 4])]
// An L: 3 µm under the pad and a 0.995 arm running 32 µm away from it.  The narrow
// stretch is not under the pad, so it is not a bond pad's metal line.
#[case::cup_2_h2("cup/CUP.2.h2.gds.gz", "TOP", vec![])]
// A 35 µm 0.995 line with the pad over x = 12..18 of it.  Report finding 3: gdscheck
// drops the two walls because the line runs more than ~20 µm past the pad.
#[case::cup_2_h3("cup/CUP.2.h3.gds.gz", "TOP", vec!["CUP.2"; 2])]
// The same bar drawn as dummy fill on Metal1 and Metal5: the rule reads the drawn layer.
#[case::cup_2_h4("cup/CUP.2.h4.gds.gz", "TOP", vec![])]
// The pad abuts the bar's left edge without covering any of it.  Report finding 4: the
// metal is selected, but no wall of it meets the pad by area and gdscheck drops both.
// The pad reaches neither wall, so no marker of the measurement meets it: the filter
// keeps a violation by its *marker*, where upstream keeps it by the measurement between
// the two walls (kept and open; report, finding 2).
#[case::cup_2_h5("cup/CUP.2.h5.gds.gz", "TOP", vec!["CUP.2"])]
// The 35 µm line of h3 with the pad over the whole of it, tile lines included.
#[case::cup_2_h6("cup/CUP.2.h6.gds.gz", "TOP", vec!["CUP.2"; 2])]
// An 8 µm line lying on its side with the pad across its middle.
#[case::cup_2_h7("cup/CUP.2.h7.gds.gz", "TOP", vec!["CUP.2"; 2])]
// An upright 8 µm line with the pad inside its width, reaching neither wall.  Report
// finding 4: the measurement is under the pad even though its walls are not.
// The pad reaches neither wall, so no marker of the measurement meets it: the filter
// keeps a violation by its *marker*, where upstream keeps it by the measurement between
// the two walls (kept and open; report, finding 2).
#[case::cup_2_h8("cup/CUP.2.h8.gds.gz", "TOP", vec![])]
// A 20 µm line across one tile line with a 6 µm pad on its middle.
#[case::cup_2_h9("cup/CUP.2.h9.gds.gz", "TOP", vec!["CUP.2"; 2])]
// The 35 µm line of h3 with a 2 µm pad instead of a 6 µm one.  Report finding 3.
#[case::cup_2_h10("cup/CUP.2.h10.gds.gz", "TOP", vec!["CUP.2"; 2])]
// The 8 µm line of h7 with the 6 µm pad of h3 on it.
#[case::cup_2_h11("cup/CUP.2.h11.gds.gz", "TOP", vec!["CUP.2"; 2])]
// Five 0.995 lines of 16, 20, 24, 28 and 32 µm with the same pad near the left end of
// each.  Report finding 3: the 32 µm line is the one gdscheck drops.
#[case::cup_2_h12("cup/CUP.2.h12.gds.gz", "TOP", vec!["CUP.2"; 10])]
// The 35 µm line of h3 with the pad moved to its middle - the same violation, reported.
#[case::cup_2_h13("cup/CUP.2.h13.gds.gz", "TOP", vec!["CUP.2"; 2])]
// Three Cs: a 0.995 slot under the pad (fires), a 0.995 slot the pad stops 0.5 short of,
// and a 1.0 slot under the pad.
#[case::cup_3_h1("cup/CUP.3.h1.gds.gz", "TOP", vec!["CUP.3"])]
// A 0.995 slot under a pad with GUARD_RING_MK abutting the C (exempt), and the same with
// the marker 0.5 clear of it (fires).
#[case::cup_3_h2("cup/CUP.3.h2.gds.gz", "TOP", vec!["CUP.3"])]
fn hardening_cup(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("cup", gds, topcell, &[]), expected, "{gds}");
}

// --- LDMOS NFET (hardening/reports/gf180mcuD/ldnmos.md).  Every fixture is a row of
// whole devices 60 µm apart, one per reading of a rule: the value the manual names, the
// 0.005 µm step past it, and - where the manual fixes a dimension rather than bounding it
// - the step short of it as well.  The device is the unit because this section has no
// opinion about a bare layer: the channel is the gate before the drift, the drain is an
// active island inside the drift, and the body tap is read against the source it is or is
// not butted to.
#[rstest]
// The gate's end cap at 0.4 (clean) and 0.395, which is both its ends.
#[case::mdn_10b_h1("ldnmos/MDN.10b.h1.gds.gz", "TOP", vec!["MDN.10b"; 2])]
// The field poly's overhang towards the drain, which the manual fixes at 0.2: 0.2
// (clean), 0.195 and 0.205 - short and long are both violations.
#[case::mdn_10c_h1("ldnmos/MDN.10c.h1.gds.gz", "TOP", vec!["MDN.10c"; 2])]
// The gate against the substrate tap, the same butted/unbutted split MDN.5a makes: a tap
// touching nothing at 0.4 (clean) and 0.395, one butted to an N+ at 0.32 (clean) and
// 0.315.
#[case::mdn_10ei_h1("ldnmos/MDN.10ei.h1.gds.gz", "TOP", vec!["MDN.10ei", "MDN.10eii"])]
// The drift's overlap of the channel, fixed at 0.4: 0.4 (clean), 0.395 - one marker per
// wall of the 0.395 strip - and 0.405.
#[case::mdn_11_h1("ldnmos/MDN.11.h1.gds.gz", "TOP", vec!["MDN.11"; 3])]
// The drift's hold on the drain: a 6 µm drain in a 7 µm drift (0.5 either side, clean), a
// 6.01 one (0.495, one marker per wall), a drain that runs out through the drift's top
// edge - which leaves the drift with no drain in it, MDN.11's reading as well as MDN.12's
// - and the same with a second island the drift does enclose, where the tab that runs out
// is an active the drift holds by nothing.  Report finding 1: gdscheck reports nothing on
// the last, because its enclosure has no half for a shape that crosses out.
#[case::mdn_12_h1("ldnmos/MDN.12.h1.gds.gz", "TOP", vec!["MDN.11", "MDN.12", "MDN.12", "MDN.12", "MDN.12"])]
// A drift 6.0 from a deep well (clean), one 5.995 from it, and one lying on it.
#[case::mdn_14_h1("ldnmos/MDN.14.h1.gds.gz", "TOP", vec!["MDN.14"; 2])]
// The drain active at 0.22 (clean) and 0.215, then a contact flush with the drain's edge
// - enclosed by the 0 µm MDN.15b asks - and one 0.005 past it.
#[case::mdn_15a_h1("ldnmos/MDN.15a.h1.gds.gz", "TOP", vec!["MDN.15a", "MDN.15a", "MDN.15b"])]
// One gap between two drifts under two readings: tied to one potential at 1.0 (clean) and
// 0.995 (MDN.2a), untied at 2.0 (clean), 1.995 (MDN.2b) and 0.995.  Report finding 2: the
// last is MDN.2b's gap alone, and gdscheck reports MDN.2a - the equal-potential rule - on
// it as well.  Every gap is centred on a tile line (10, 20, 21, 40, 42).
#[case::mdn_2b_h1("ldnmos/MDN.2b.h1.gds.gz", "TOP", vec!["MDN.2a", "MDN.2b", "MDN.2b"])]
// The channel at 0.6 (clean), 0.595 - which leaves the gate 1.195 wide, so MDN.10a comes
// with it - 20.0 (clean) and 20.005.
#[case::mdn_3a_h1("ldnmos/MDN.3a.h1.gds.gz", "TOP", vec!["MDN.10a", "MDN.10a", "MDN.3a", "MDN.3b", "MDN.3b"])]
// The transistor's width at 4.0 (clean), 3.995, 50.0 (clean) and 50.005 - the wide one
// being MDN.13a's finger as well as MDN.4b's channel.
#[case::mdn_4a_h1("ldnmos/MDN.4a.h1.gds.gz", "TOP", vec!["MDN.13a", "MDN.4a", "MDN.4a", "MDN.4b", "MDN.4b"])]
// The body tap against the drift, one gap under two numbers: unbutted at 1.0 (clean) and
// 0.995, butted to an N+ at 0.92 (clean) and 0.915, and a butted one at 0.995 - over the
// butted number, under the unbutted one, which is what the split is for.
#[case::mdn_5ai_h1("ldnmos/MDN.5ai.h1.gds.gz", "TOP", vec!["MDN.5ai", "MDN.5aii"])]
// A tap whose corner meets the source N+'s corner at one point, 0.95 off the drift.  It
// shares no edge with the source, so it is not a butted source and body tap and the 1 µm
// number is its own.  Report finding 3: both tools read the point touch as butted and
// stay silent.
// A P+ tap meeting the source at a single *point* is read as butted, so the pair takes
// MDN.5aii's 0.92 rather than MDN.5ai's 1.0 (kept; both tools read `interacting` that
// way, and the manual's butted pair is a shared edge - report, finding 3).
#[case::mdn_5aii_h2("ldnmos/MDN.5aii.h2.gds.gz", "TOP", vec![])]
// An active island with its top 0.5 below Dualgate's edge (clean) and one 0.495 below it.
#[case::mdn_6a_h1("ldnmos/MDN.6a.h1.gds.gz", "TOP", vec!["MDN.6a"])]
// A drift against an N-well: tied at 1.0 (clean) and 0.995, untied at 2.0 (clean) and
// 1.995, and a drift lying on a well - which is one node, so MDN.8a alone.
#[case::mdn_8a_h1("ldnmos/MDN.8a.h1.gds.gz", "TOP", vec!["MDN.8a", "MDN.8a", "MDN.8b"])]
// A stray active 4.0 from the drift (clean) and one 3.995 from it.
#[case::mdn_9_h1("ldnmos/MDN.9.h1.gds.gz", "TOP", vec!["MDN.9"])]
fn hardening_ldnmos(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("ldnmos", gds, topcell, &[]), expected, "{gds}");
}

// --- LDMOS PFET (hardening/reports/gf180mcuD/ldpmos.md).  The same rows of whole devices
// as the N side, over the device built the other way up: a deep well, an N+ guard ring
// inside it, two markers over both, and a drift that holds its drain and overlaps its
// channel by a fixed amount.
#[rstest]
// The channel at 0.6 (clean), 0.595 - which leaves the gate 1.195 wide, so MDP.9a comes
// with it - 20.0 (clean) and 20.005.
#[case::mdp_1_h1("ldpmos/MDP.1.h1.gds.gz", "TOP", vec!["MDP.1", "MDP.1a", "MDP.1a", "MDP.9a", "MDP.9a"])]
// The drift's overlap of the channel, which the manual fixes at 0.4: 0.4 (clean), 0.395
// (two walls, as the N side's MDN.11 reports them) and 0.405.
#[case::mdp_10_h1("ldpmos/MDP.10.h1.gds.gz", "TOP", vec!["MDP.10"; 3])]
// One gap between two drifts under two readings: sharing a drain, so one potential, at
// 1.0 (clean) and 0.995 (MDP.10b); mirrored and untied at 2.0 (clean), 1.995 (MDP.10a)
// and 0.995.  Report finding 2: the last is MDP.10a's gap alone, and gdscheck reports
// MDP.10b - the equal-potential rule - on it as well.
#[case::mdp_10b_h1("ldpmos/MDP.10b.h1.gds.gz", "TOP", vec!["MDP.10a", "MDP.10a", "MDP.10b"])]
// The drift's hold on the drain, 0.8 in each direction: 0.8 past the drain (clean), 0.795,
// 0.8 past the active in the width direction (clean) and 0.795, which is both walls.
#[case::mdp_11_h1("ldpmos/MDP.11.h1.gds.gz", "TOP", vec!["MDP.11"; 3])]
// The deep well's hold on the N+ guard ring at 0.66 (clean) and 0.655.
#[case::mdp_12_h1("ldpmos/MDP.12.h1.gds.gz", "TOP", vec!["MDP.12"])]
// A second deep well 6.0 from the one that carries the drift (clean) and one 5.995 from
// it.
#[case::mdp_15_h1("ldpmos/MDP.15.h1.gds.gz", "TOP", vec!["MDP.15"])]
// The drain active at 0.22 (clean) and 0.215, then a contact flush with the drain's edge
// - enclosed by the 0 µm MDP.16b asks - and one 0.005 past it.
#[case::mdp_16a_h1("ldpmos/MDP.16a.h1.gds.gz", "TOP", vec!["MDP.16a", "MDP.16a", "MDP.16b"])]
// The transistor's width at 4.0 (clean), 3.995, 50.0 (clean) and 50.005.  The section
// caps the finger (MDP.13a) and floors the width (MDP.2), and names no maximum width.
#[case::mdp_2_h1("ldpmos/MDP.2.h1.gds.gz", "TOP", vec!["MDP.13a", "MDP.2", "MDP.2"])]
// The guard ring's tap against the drift, one gap under two numbers: unbutted at 1.0
// (clean) and 0.995, butted to a P+ at 0.92 (clean) and 0.915.
#[case::mdp_3ai_h1("ldpmos/MDP.3ai.h1.gds.gz", "TOP", vec!["MDP.3ai", "MDP.3aii"])]
// An N+ and a P+ active in the deep well, 0.4 apart (clean) and 0.395 apart.
#[case::mdp_3b_h1("ldpmos/MDP.3b.h1.gds.gz", "TOP", vec!["MDP.3b"])]
// A P+ active outside the deep well, 2.5 from it (clean) and 2.495 from it.
#[case::mdp_4a_h1("ldpmos/MDP.4a.h1.gds.gz", "TOP", vec!["MDP.4a"])]
// A P+ tab against the deep well's edge with Dualgate 0.5 past it (clean) and 0.495.
#[case::mdp_5a_h1("ldpmos/MDP.5a.h1.gds.gz", "TOP", vec!["MDP.5a"])]
// What the LDMOS marker keeps clear of outside itself: an N-well at 2.0 (clean) and 1.995,
// an N+ active at 1.5 (clean) and 1.495.
#[case::mdp_7_h1("ldpmos/MDP.7.h1.gds.gz", "TOP", vec!["MDP.7", "MDP.8"])]
// The gate's end cap at 0.4 (clean) and 0.395, which is both its ends.
#[case::mdp_9b_h1("ldpmos/MDP.9b.h1.gds.gz", "TOP", vec!["MDP.9b"; 2])]
// The gate against the guard ring's tap, the butted/unbutted split again: 0.4 (clean) and
// 0.395 unbutted, 0.32 (clean) and 0.315 butted.
#[case::mdp_9ei_h1("ldpmos/MDP.9ei.h1.gds.gz", "TOP", vec!["MDP.9ei", "MDP.9eii"])]
fn hardening_ldpmos(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("ldpmos", gds, topcell, &[]), expected, "{gds}");
}

// --- P+ poly resistor (hardening/reports/gf180mcuD/pres.md).  A bar is a resistor when
// Poly2 and Pplus meet under a salicide block with RES_MK touching them and no RESISTOR
// marker over them; `min_width` reports one marker per wall.
#[rstest]
// Four 0.795 bars: the full device (fires), the same without RES_MK, without the block,
// and with a RESISTOR marker over it - the last is a high-sheet resistor, not this one.
#[case::pres_1_h1("pres/PRES.1.h1.gds.gz", "TOP", vec!["PRES.1"; 2], vec![])]
// A RES_MK abutting the bar's left edge and covering none of it still makes the bar a
// resistor; pulled 0.005 clear it does not.  The marking covers no part of the resistor,
// which is what PRES.9a asks of it.
#[case::pres_1_h2("pres/PRES.1.h2.gds.gz", "TOP", vec!["PRES.1", "PRES.1", "PRES.9a"], vec![])]
// A 1.0 body with a 0.6 head outside the block: the width is measured on the whole bar.
#[case::pres_1_h3("pres/PRES.1.h3.gds.gz", "TOP", vec!["PRES.1"; 2], vec![])]
// The same 0.795 bar across x = 20, across x = 40 and 42, and across y = 20.
#[case::pres_1_h4("pres/PRES.1.h4.gds.gz", "TOP", vec!["PRES.1"; 6], vec![])]
// A serpentine whose two arms are 0.395 apart, and one at 0.4.  Report finding 1: the gap
// inside one folded resistor is a space between Poly2 resistors and is not reported.
#[case::pres_2_h1("pres/PRES.2.h1.gds.gz", "TOP", vec!["PRES.2"], vec![])]
// Two resistors 0.395 apart with the gap straddling y = 40, and a clean 0.4 pair.
#[case::pres_2_h2("pres/PRES.2.h2.gds.gz", "TOP", vec!["PRES.2"], vec![])]
// COMP 0.595 off the body's end, at 0.6, and 0.594 away on the diagonal.
#[case::pres_3_h1("pres/PRES.3.h1.gds.gz", "TOP", vec!["PRES.3"; 2], vec![])]
// COMP abutting the body, overlapping it by 0.5, and wholly inside it.  Report finding 2:
// the abutting COMP is a space of nothing and is not reported.
#[case::pres_3_h2("pres/PRES.3.h2.gds.gz", "TOP", vec!["PRES.3"; 3], vec![])]
// Unrelated Poly2 at 0.595, at 0.6, and at 0.595 with a salicide block of its own five
// micron away.  Report finding 3: that block makes the bar related to this resistor.
#[case::pres_4_h1("pres/PRES.4.h1.gds.gz", "TOP", vec!["PRES.4"; 2], vec![])]
// The implant stopping in the middle of the block, so the resistor's own wall is the
// implant's edge and the implant overlaps it by nothing.
#[case::pres_5_h1("pres/PRES.5.h1.gds.gz", "TOP", vec!["PRES.5"], vec![])]
// The block overhanging 0.275 above the body and 0.28 below it.
#[case::pres_6_h1("pres/PRES.6.h1.gds.gz", "TOP", vec!["PRES.6"], vec![])]
// The block covering the whole bar, 0.35 in the width direction and 0.15 past each end,
// and the same 0.15 past the left end only.  Both read as 0.15 short of the 0.28 (kept;
// report finding 4).  The rule is the overlap in the width direction and the ends are not
// in it, but which of a bar's walls lie in that direction is the device's orientation,
// which neither deck knows: the block's own end and the resistor's are the same wall to
// an enclosure.  Both tools report them, and a block that runs past the bar's end leaves
// no salicided head there, which is not a device either.
#[case::pres_6_h2("pres/PRES.6.h2.gds.gz", "TOP", vec!["PRES.6"; 2], vec![])]
#[case::pres_6_h3("pres/PRES.6.h3.gds.gz", "TOP", vec!["PRES.6"], vec![])]
// A contact abutting the block's edge and one wholly inside the block.
#[case::pres_7_h1("pres/PRES.7.h1.gds.gz", "TOP", vec!["PRES.7"; 2], vec![])]
// A contact 0.2149 from a salicide island on the diagonal: 0.152 in x and in y.
#[case::pres_7_h2("pres/PRES.7.h2.gds.gz", "TOP", vec!["PRES.7"], vec![])]
// Four markings: on the resistor's outline exactly (clean), 0.6 narrow, 0.6 short at each
// end, and 0.9 past the block at each end but still short of the bar.
#[case::pres_9a_h1("pres/PRES.9a.h1.gds.gz", "TOP", vec!["PRES.9a"; 3], vec![])]
// A 1 long resistor under a 6.6 long marking that clears the whole bar.  Report finding 5:
// the manual asks the marking's length to coincide with the block's.
#[case::pres_9a_h2("pres/PRES.9a.h2.gds.gz", "TOP", vec!["PRES.9a"], vec![])]
// A 140 x 140 octagon marking - 16238 µm², both sides over 80 - 19.9 from a 100 square.
#[case::pres_9b_h1("pres/PRES.9b.h1.gds.gz", "TOP", vec!["PRES.9b"], vec![])]
// One 110 x 160 marking with a 19.9 notch cut 70 into it: no adjacent marking.
#[case::pres_9b_h2("pres/PRES.9b.h2.gds.gz", "TOP", vec![], vec![])]
fn hardening_pres(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(hardening("pres", gds, topcell, &ignore), expected, "{gds}");
}

// --- N+ poly resistor (hardening/reports/gf180mcuD/lres.md).  Section 10.2 is 10.1 with
// the implant swapped, so these are the same layouts on Nplus; the one place the two part
// is the bar under a RESISTOR marker, which 10.2 still calls its own.
#[rstest]
// Four 0.795 bars: the full device, the same without RES_MK, without the block, and with a
// RESISTOR marker over it.  10.2 has no clause taking that last bar away and 10.3 does not
// claim it either (it recognises its device through Pplus), so it stays an N+ resistor.
#[case::lres_1_h1("lres/LRES.1.h1.gds.gz", "TOP", vec!["LRES.1"; 4], vec![])]
// A RES_MK abutting the bar's left edge, and the same pulled 0.005 clear.
#[case::lres_1_h2("lres/LRES.1.h2.gds.gz", "TOP", vec!["LRES.1", "LRES.1", "LRES.9a"], vec![])]
// A 1.0 body with a 0.6 head outside the block.
#[case::lres_1_h3("lres/LRES.1.h3.gds.gz", "TOP", vec!["LRES.1"; 2], vec![])]
// The same 0.795 bar across x = 20, across x = 40 and 42, and across y = 20.
#[case::lres_1_h4("lres/LRES.1.h4.gds.gz", "TOP", vec!["LRES.1"; 6], vec![])]
// A serpentine whose two arms are 0.395 apart, and one at 0.4.  Report finding 1.
#[case::lres_2_h1("lres/LRES.2.h1.gds.gz", "TOP", vec!["LRES.2"], vec![])]
// Two resistors 0.395 apart with the gap straddling y = 40, and a clean 0.4 pair.
#[case::lres_2_h2("lres/LRES.2.h2.gds.gz", "TOP", vec!["LRES.2"], vec![])]
// COMP 0.595 off the body's end, at 0.6, and 0.594 away on the diagonal.
#[case::lres_3_h1("lres/LRES.3.h1.gds.gz", "TOP", vec!["LRES.3"; 2], vec![])]
// COMP abutting the body, overlapping it, and wholly inside it.  Report finding 2.
#[case::lres_3_h2("lres/LRES.3.h2.gds.gz", "TOP", vec!["LRES.3"; 3], vec![])]
// Unrelated Poly2 at 0.595, at 0.6, and at 0.595 with a block of its own.  Report 3.
#[case::lres_4_h1("lres/LRES.4.h1.gds.gz", "TOP", vec!["LRES.4"; 2], vec![])]
// The implant stopping in the middle of the block.
#[case::lres_5_h1("lres/LRES.5.h1.gds.gz", "TOP", vec!["LRES.5"], vec![])]
// The block overhanging 0.275 above the body and 0.28 below it.
#[case::lres_6_h1("lres/LRES.6.h1.gds.gz", "TOP", vec!["LRES.6"], vec![])]
// The block covering the whole bar, 0.15 past each end, and the same past the left end
// only - both kept as both tools read them, for the reason on `pres_6_h2` (finding 4).
#[case::lres_6_h2("lres/LRES.6.h2.gds.gz", "TOP", vec!["LRES.6"; 2], vec![])]
#[case::lres_6_h3("lres/LRES.6.h3.gds.gz", "TOP", vec!["LRES.6"], vec![])]
// A contact abutting the block's edge and one wholly inside the block.
#[case::lres_7_h1("lres/LRES.7.h1.gds.gz", "TOP", vec!["LRES.7"; 2], vec![])]
// A contact 0.2149 from a salicide island on the diagonal.
#[case::lres_7_h2("lres/LRES.7.h2.gds.gz", "TOP", vec!["LRES.7"], vec![])]
// Four markings: on the outline exactly, 0.6 narrow, 0.6 short, 0.9 long.
#[case::lres_9a_h1("lres/LRES.9a.h1.gds.gz", "TOP", vec!["LRES.9a"; 3], vec![])]
// A 1 long resistor under a 6.6 long marking.  Report finding 5.
#[case::lres_9a_h2("lres/LRES.9a.h2.gds.gz", "TOP", vec!["LRES.9a"], vec![])]
// A 140 x 140 octagon marking 19.9 from a 100 square.
#[case::lres_9b_h1("lres/LRES.9b.h1.gds.gz", "TOP", vec!["LRES.9b"], vec![])]
// One 110 x 160 marking with a 19.9 notch cut into it.
#[case::lres_9b_h2("lres/LRES.9b.h2.gds.gz", "TOP", vec![], vec![])]
fn hardening_lres(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(hardening("lres", gds, topcell, &ignore), expected, "{gds}");
}

// --- High-sheet poly resistor (hardening/reports/gf180mcuD/hres.md).  Drawn as the manual
// describes it: the implant comes in from each end, stops 0.1 over the salicide block, and
// the resistor is the bare Poly2 between the two heads.
#[rstest]
// A 0.395 notch in the RESISTOR marker, 0.5 clear of the bar.
#[case::hres_1_h1("hres/HRES.1.h1.gds.gz", "TOP", vec!["HRES.1"], vec![])]
// Five 0.995 bars: the full device; without the RESISTOR marker; without RES_MK; without
// the block; and one whose implant only abuts the bar - enough for this section, which
// asks the Poly2 to touch the implant where 10.1 asks for Poly2 and Pplus.  That abutting
// implant is also a head that reaches the block by nothing, which is under HRES.10's 0.1
// (finding 8), and the runset reports it there too.
#[case::hres_2_h1("hres/HRES.2.h1.gds.gz", "TOP", vec!["HRES.10", "HRES.2", "HRES.2", "HRES.2", "HRES.2"], vec![])]
// The same 0.995 bar across x = 20, across x = 40 and 42, and across y = 20.
#[case::hres_2_h2("hres/HRES.2.h2.gds.gz", "TOP", vec!["HRES.2"; 6], vec![])]
// A serpentine whose two arms are 0.395 apart.  The same geometry under 10.1 and 10.2 is
// reported by neither tool; here the deck has the notch and reports it (report finding 1).
#[case::hres_3_h1("hres/HRES.3.h1.gds.gz", "TOP", vec!["HRES.3"], vec![])]
// The RESISTOR marker covering the left half of the bar and stopping.
#[case::hres_4_h1("hres/HRES.4.h1.gds.gz", "TOP", vec!["HRES.4"], vec![])]
// Unrelated Poly2 0.295 from the marker, at 0.3, and at 0.295 with a salicide block of
// its own.  Report finding 3: that block makes the bar related to this resistor.
#[case::hres_5_h1("hres/HRES.5.h1.gds.gz", "TOP", vec!["HRES.5"; 2], vec![])]
// COMP 0.295 from the marker, at 0.3, and under the marker but clear of the bar.
#[case::hres_6_h1("hres/HRES.6.h1.gds.gz", "TOP", vec!["HRES.6"; 2], vec![])]
// A contact on the head whose corner sticks out of the L-shaped implant.
#[case::hres_7_h1("hres/HRES.7.h1.gds.gz", "TOP", vec!["HRES.7"], vec![])]
// A contact on the body, wholly inside the salicide block.
#[case::hres_8_h1("hres/HRES.8.h1.gds.gz", "TOP", vec!["HRES.8"], vec![])]
// A contact abutting the block's edge, which leaves it 0.1 from the implant's edge too.
#[case::hres_8_h2("hres/HRES.8.h2.gds.gz", "TOP", vec!["HRES.7", "HRES.8"], vec![])]
// A 0.5 x 0.4 hole in the block over the middle of the body.  Report finding 6: the block
// overlaps the resistor by nothing there.
#[case::hres_9_h1("hres/HRES.9.h1.gds.gz", "TOP", vec!["HRES.9"], vec![])]
// The block overhanging 0.275 above the bar and 0.28 below it.
#[case::hres_9_h2("hres/HRES.9.h2.gds.gz", "TOP", vec!["HRES.9"], vec![])]
// A notch bitten out of the block from above, ending 0.4 inside the bar.  Report finding
// 7: neither tool reads the block's edge where it stops inside the resistor's width.
#[case::hres_9_h3("hres/HRES.9.h3.gds.gz", "TOP", vec!["HRES.9"], vec![])]
// The left head reaching 0.1 over the block, and one reaching 0.095.
#[case::hres_10_h1("hres/HRES.10.h1.gds.gz", "TOP", vec!["HRES.10"; 2], vec![])]
// The left head stopping 0.1 short of the block.  Report finding 8: an overlap of nothing
// is under the minimum the rule names.
#[case::hres_10_h2("hres/HRES.10.h2.gds.gz", "TOP", vec!["HRES.10"], vec![])]
// The left head reaching 0.105 over the block.  Report finding 9: the rule's maximum half
// is not read on the overlap the manual means.
#[case::hres_10_h3("hres/HRES.10.h3.gds.gz", "TOP", vec!["HRES.10"], vec![])]
// Three markings: on the body exactly (clean), 0.6 narrow, and 0.9 short at each end.
#[case::hres_12a_h1("hres/HRES.12a.h1.gds.gz", "TOP", vec!["HRES.12a"; 2], vec![])]
// A 140 x 140 octagon marking 19.9 from a 100 square.
#[case::hres_12b_h1("hres/HRES.12b.h1.gds.gz", "TOP", vec!["HRES.12b"], vec![])]
fn hardening_hres(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(hardening("hres", gds, topcell, &ignore), expected, "{gds}");
}

// --- Density (hardening/reports/gf180mcuD/density.md).  Every rule is a coverage
// fraction of the whole die: the drawn layer joined with its dummy fill over the area
// inside PR_BNDRY.  Every fixture carries all seven measured layers, so a fixture about
// one of them leaves the other eight rules clear.
#[rstest]
// Every floor exactly met: COMP 25%, Poly2 14%, each metal 30%.
#[case::dcf_1b_h1("density/DCF.1b.h1.gds.gz", "TOP", vec![])]
// The same die with every band 0.005 µm shorter: all eight floors missed.
#[case::dcf_1b_h2("density/DCF.1b.h2.gds.gz", "TOP", vec!["DCF.1b", "M1.4", "M2.4", "M3.4", "M4.4", "M5.4", "MT.3", "PL.8"])]
// The drawn layer and its dummy fill covering the same 40%: coverage is area covered,
// so the overlap counts once and COMP reads 40%, not 80%.
#[case::dcf_1b_h3("density/DCF.1b.h3.gds.gz", "TOP", vec![])]
// COMP, Poly2 and Metal1 drawn nowhere and their dummy fill carrying 40% of the die.
#[case::dcf_1b_h4("density/DCF.1b.h4.gds.gz", "TOP", vec![])]
// No PR_BNDRY drawn at all: the die is what the layout covers, and it is 40% full.
#[case::dcf_1b_h5("density/DCF.1b.h5.gds.gz", "TOP", vec![])]
// An L-shaped die, 7500 µm² in a 10 000 µm² box, every layer covering 2600 µm² of it:
// 34.67% of the die.  Report finding 1 - gdscheck measures 26%, over the box, and fires
// the six metal floors.
#[case::dcf_1b_h6("density/DCF.1b.h6.gds.gz", "TOP", vec![])]
// Two dies with a 10 µm street between them, every layer at 30% of the 10 000 µm² of
// boundary.  Report finding 1 - measured over the 11 000 µm² box it is 27.3%.
#[case::dcf_1b_h7("density/DCF.1b.h7.gds.gz", "TOP", vec![])]
// The exact bounds again as upright stripes across the tile lines at 20, 21, 40 and 42,
// on a die whose edges sit at 3 and 103.
#[case::dcf_1b_h8("density/DCF.1b.h8.gds.gz", "TOP", vec![])]
// A 400 µm die 40% full, with a 200 µm square of it empty: the rules are die-wide, and
// the 200 µm window of section 13.3 is a dummy-generation recipe, not a rule.
#[case::dcf_1b_h9("density/DCF.1b.h9.gds.gz", "TOP", vec![])]
// COMP exactly on DCF.1d's 70% ceiling.
#[case::dcf_1d_h1("density/DCF.1d.h1.gds.gz", "TOP", vec![])]
// COMP one grid step over the ceiling.
#[case::dcf_1d_h2("density/DCF.1d.h2.gds.gz", "TOP", vec!["DCF.1d"])]
// A 5000 µm² COMP block 100 µm outside the die, and one straddling its edge: what is
// outside the die is not counted in it.
#[case::dcf_1d_h3("density/DCF.1d.h3.gds.gz", "TOP", vec![])]
// The L-shaped die two thirds full of COMP, with 2400 µm² more COMP in the notch - the
// part of the bounding box that is not die.  Report finding 1: 66.67% of the die, but
// gdscheck reads 74% of the box and fires the ceiling.
#[case::dcf_1d_h4("density/DCF.1d.h4.gds.gz", "TOP", vec![])]
fn hardening_density(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("density", gds, topcell, &[]), expected, "{gds}");
}

// --- Antenna (hardening/reports/gf180mcuD/antenna.md).  The gate throughout is
// 0.06615 µm² (0.21 µm of Poly2 across 0.315 µm of COMP) and a metal antenna is one
// 0.66 µm wide bar, so its perimeter is 2(len + 0.66) and the manual's perimeter area is
// that times the layer's thickness.
#[rstest]
// Three Poly2 antennas: 200.7 over a thin gate (fires), 198.3 (clean), and 200.7 over a
// gate Dualgate covers - ANT.1 is read on every gate, thick or thin.
#[case::ant_1_h1("antenna/ANT.1.h1.gds.gz", "TOP", vec!["ANT.1", "ANT.1"])]
// A gate whose Poly2 is at 71 strapped through Metal1 to a gateless 40 µm Poly2 pad.
// Poly2's ratio is read before the contact level: the two shapes are not added (316).
#[case::ant_1_h2("antenna/ANT.1.h2.gds.gz", "TOP", vec![])]
// The 200.7 antenna with an N+ diode on the Metal1 that straps its gate.  ANT.16's
// relief is written for Metaln and Vian; Poly2 has none.
#[case::ant_1_h3("antenna/ANT.1.h3.gds.gz", "TOP", vec!["ANT.1"])]
// A 200.7 Poly2 antenna and a 408 Metal1 bar on a gate RES_MK covers: a resistor body is
// not a gate, so neither ratio has a denominator.
#[case::ant_1_h4("antenna/ANT.1.h4.gds.gz", "TOP", vec![])]
// Fourteen contacts on the gate's Poly2 head: 10.24 over ANT.8's 10.
#[case::ant_8_h1("antenna/ANT.8.h1.gds.gz", "TOP", vec!["ANT.8"])]
// Thirteen on the gate (9.51), twenty on a gateless pad the same Metal1 strap reaches,
// four on the transistor's own source/drain.  Contact is read before Metal1 joins
// anything, and the diffusion is another node: 24.2 and 12.4 if either were added.
#[case::ant_8_h2("antenna/ANT.8.h2.gds.gz", "TOP", vec![])]
// The manual's own repair: a 408 Metal1 bar reached only by jogging up to Metal2 and
// back.  Metal1's ratio is read with Metal2 absent, so the bar is off the gate's node.
#[case::ant_16_i_ant_2_h1("antenna/ANT.16_i_ANT.2.h1.gds.gz", "TOP", vec![])]
// The same 24.34 µm bar hung straight off the gate's contact: 408 over 400.
#[case::ant_16_i_ant_2_h2("antenna/ANT.16_i_ANT.2.h2.gds.gz", "TOP", vec!["ANT.16_i_ANT.2"])]
// A 91.84 µm bar, 1510.
#[case::ant_16_i_ant_2_h3("antenna/ANT.16_i_ANT.2.h3.gds.gz", "TOP", vec!["ANT.16_i_ANT.2"])]
// The same bar with an N+ diode on the node: gate area becomes 0.06615 + 2 × 0.1296 and
// the ratio 307.  At a factor of one it would be 510.
#[case::ant_16_i_ant_2_h4("antenna/ANT.16_i_ANT.2.h4.gds.gz", "TOP", vec![])]
// The same bar with a 1 µm² N-well tied to the node by an N+ tap: 48.
#[case::ant_16_i_ant_2_h5("antenna/ANT.16_i_ANT.2.h5.gds.gz", "TOP", vec![])]
// A 184.34 µm bar with a P+ diode in an N-well that nothing ties.  Only the 0.1296 µm²
// of P+ is on the node: 614.  With the untied well counted it would be 86.
#[case::ant_16_i_ant_2_h6("antenna/ANT.16_i_ANT.2.h6.gds.gz", "TOP", vec!["ANT.16_i_ANT.2"])]
// The 408 antenna with the gate on the tile line at x = 20 and the bar across 21, 40
// and 42.
#[case::ant_16_i_ant_2_h7("antenna/ANT.16_i_ANT.2.h7.gds.gz", "TOP", vec!["ANT.16_i_ANT.2"])]
// The 184.34 µm bar on a gate Dualgate covers, with the same N+ diode: the thick gate's
// factor of 15 gives 99.  At the thin gate's factor of 2 it would be 614.
#[case::ant_16_ii_ant_2_h1("antenna/ANT.16_ii_ANT.2.h1.gds.gz", "TOP", vec![])]
// A 17.86 µm bar (302 against a whole gate, 604 against half of one) over two gates:
// Dualgate covers half of the first and abuts the second.  The marker names an area, so
// the first gate is half thin and half thick and both halves fire; the second is clean.
#[case::ant_16_ii_ant_2_h2("antenna/ANT.16_ii_ANT.2.h2.gds.gz", "TOP", vec!["ANT.16_i_ANT.2", "ANT.16_ii_ANT.2"])]
// A 10.84 µm Metal5 bar at the top of a full riser: Metal5 is this stack's MetalTop and
// 1.19 µm thick, so 413.8.  At an intermediate metal's 0.54 µm it would be 187.8.
#[case::ant_16_i_ant_6_h1("antenna/ANT.16_i_ANT.6.h1.gds.gz", "TOP", vec!["ANT.16_i_ANT.6"])]
// Twenty Via1 on the gate's Metal1: 20.4 over ANT.9's 20.
#[case::ant_16_i_ant_9_h1("antenna/ANT.16_i_ANT.9.h1.gds.gz", "TOP", vec!["ANT.16_i_ANT.9"])]
// One Via1 on the gate's Metal1 and twenty on an island the gate reaches only through
// Metal2.  Via1's ratio is read with Metal2 absent: 1.02, not 21.5.
#[case::ant_16_i_ant_9_h2("antenna/ANT.16_i_ANT.9.h2.gds.gz", "TOP", vec![])]
// A MIM-B cap as section 10.4.2 draws it - Metal4 the bottom plate, FuseTop over it,
// the top plate taken up through Via4 - with a 41.84 µm Metal5 bar: 404.6 over ANT.14's
// 400.  Report finding 1: gdscheck ties the plate in at Via3 and sees nothing.
#[case::ant_16_iii_ant_14_m5_mimb_h1("antenna/ANT.16_iii_ANT.14_M5_MIMB.h1.gds.gz", "TOP", vec!["ANT.16_iii_ANT.14_M5_MIMB"])]
// The same cap with seventy-five Via4 on the bottom plate's arm: 20.6 over ANT.15's 20.
// Report finding 1.
#[case::ant_16_iii_ant_15_v4_mimb_h1("antenna/ANT.16_iii_ANT.15_V4_MIMB.h1.gds.gz", "TOP", vec!["ANT.16_iii_ANT.15_V4_MIMB"])]
fn hardening_antenna(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    assert_eq!(hardening("antenna", gds, topcell, &[]), expected, "{gds}");
}

// --- Dummy COMP (hardening/reports/gf180mcuD/dummy_comp.md).  Section 13.1.  Every
// `h1` is four probes: the value (clean), a step past it, the same gap corner to corner
// just under the value, and one step wider (clean) - so two violations.  Every `h2` puts
// the fill on its partner: abutting, half over, and, where the manual keeps fill out of
// the layer altogether, wholly inside it.  A shared edge is a space of nothing and an
// overlap is less than nothing, so both are the rule's own violation.
#[rstest]
// Fill to fill at 1.9 / 1.895, and corner to corner at 1.9021 / 1.8950.
#[case::dcf_2b_h1("dummy_comp/DCF.2b.h1.gds.gz", "TOP", vec!["DCF.2b", "DCF.2b"], vec![])]
// The same 1.895 gap on the tile lines: a notch open across x = 20, a gap opening on
// x = 40 whose right-hand square crosses x = 42, a gap opening on y = 20, and a 1.9 gap
// on x = 42 that must stay clean.
#[case::dcf_2b_h2("dummy_comp/DCF.2b.h2.gds.gz", "TOP", vec!["DCF.2b", "DCF.2b", "DCF.2b"], vec![])]
// Fill to circuit COMP at 3.5 / 3.495 / 3.5002 / 3.4931.
#[case::dcf_4_h1("dummy_comp/DCF.4.h1.gds.gz", "TOP", vec!["DCF.4", "DCF.4"], vec![])]
// Fill abutting circuit COMP, and fill half over it.  Report finding 1.
#[case::dcf_4_h2("dummy_comp/DCF.4.h2.gds.gz", "TOP", vec!["DCF.4", "DCF.4"], vec![])]
#[case::dcf_5_h1("dummy_comp/DCF.5.h1.gds.gz", "TOP", vec!["DCF.5", "DCF.5"], vec![])]
// Fill abutting circuit Poly2, and fill under it.  Report finding 1.
#[case::dcf_5_h2("dummy_comp/DCF.5.h2.gds.gz", "TOP", vec!["DCF.5", "DCF.5"], vec![])]
#[case::dcf_6a_h1("dummy_comp/DCF.6a.h1.gds.gz", "TOP", vec!["DCF.6a", "DCF.6a"], vec![])]
// Three fills in one N-well: 1.295 inside the boundary, deep inside (clean), and
// hanging over the far boundary.  The rule names the *boundary*, and DCF.1a calls what
// it defines a region; a region set by a distance to a boundary is a band that straddles
// it.  Report finding 2.
#[case::dcf_6a_h2("dummy_comp/DCF.6a.h2.gds.gz", "TOP", vec!["DCF.6a", "DCF.6a"], vec![])]
#[case::dcf_6b_h1("dummy_comp/DCF.6b.h1.gds.gz", "TOP", vec!["DCF.6b", "DCF.6b"], vec![])]
// The same band in a DNWELL, at 3.995.  Report finding 2.
#[case::dcf_6b_h2("dummy_comp/DCF.6b.h2.gds.gz", "TOP", vec!["DCF.6b", "DCF.6b"], vec![])]
#[case::dcf_6c_h1("dummy_comp/DCF.6c.h1.gds.gz", "TOP", vec!["DCF.6c", "DCF.6c"], vec![])]
// The same band in an LVPWELL, at 1.295.  Report finding 2.
#[case::dcf_6c_h2("dummy_comp/DCF.6c.h2.gds.gz", "TOP", vec!["DCF.6c", "DCF.6c"], vec![])]
#[case::dcf_6d_h1("dummy_comp/DCF.6d.h1.gds.gz", "TOP", vec!["DCF.6d", "DCF.6d"], vec![])]
// The same band under a Dualgate, at 1.295.  Report finding 2.
#[case::dcf_6d_h2("dummy_comp/DCF.6d.h2.gds.gz", "TOP", vec!["DCF.6d", "DCF.6d"], vec![])]
#[case::dcf_8a_h1("dummy_comp/DCF.8a.h1.gds.gz", "TOP", vec!["DCF.8a", "DCF.8a"], vec![])]
// Abutting RES_MK, half over it, and wholly under it - "Dummy COMP should not exit
// under RES_MK".  Report findings 1 and 3.
#[case::dcf_8a_h2("dummy_comp/DCF.8a.h2.gds.gz", "TOP", vec!["DCF.8a", "DCF.8a", "DCF.8a"], vec![])]
#[case::dcf_11a_h1("dummy_comp/DCF.11a.h1.gds.gz", "TOP", vec!["DCF.11a", "DCF.11a"], vec![])]
// The same three against NDMY - "Dummy COMP cannot exit under NDMY".  Findings 1 and 3.
#[case::dcf_11a_h2("dummy_comp/DCF.11a.h2.gds.gz", "TOP", vec!["DCF.11a", "DCF.11a", "DCF.11a"], vec![])]
#[case::dcf_12_h1("dummy_comp/DCF.12.h1.gds.gz", "TOP", vec!["DCF.12", "DCF.12"], vec![])]
// Abutting IND_MK and half over it.  The shared edge is DCF.12's space of nothing; the
// fill lying *on* the marker is what DCF.13 forbids in as many words, which is where the
// overlap half of this pair belongs.
#[case::dcf_12_h2("dummy_comp/DCF.12.h2.gds.gz", "TOP", vec!["DCF.12", "DCF.13"], vec![])]
// A 5 x 5 dummy COMP, half a square, and a square with a bite out of it.  DCF.10 is
// Appendix B's - the manual stars it as a rule not coded - so nothing here reports the
// truncated squares, and this layout pins that (report finding 5, kept).
#[case::dcf_10_h1("dummy_comp/DCF.10.h1.gds.gz", "TOP", vec![], vec![])]
// A dummy COMP wholly inside IND_MK: "Dummy COMP should not exist under IND_MK layer".
// Report finding 4.
#[case::dcf_13_h1("dummy_comp/DCF.13.h1.gds.gz", "TOP", vec!["DCF.13"], vec![])]
// Fill 26 µm inside the PR_BNDRY polygon and 25.995 inside it.  DCF.7a is Appendix B's
// too, with the rest of the scribe-line family (report finding 6, kept).
#[case::dcf_7a_h1("dummy_comp/DCF.7a.h1.gds.gz", "TOP", vec![], vec![])]
fn hardening_dummy_comp(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("dummy_comp", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

// --- Dummy Poly2 (hardening/reports/gf180mcuD/dummy_poly2.md).  Section 13.2, read the
// same way as 13.1: `h1` walks the distance, `h2` puts the two shapes together.  Every
// dummy poly here carries the dummy COMP core DPF.1 asks for.
#[rstest]
// A 5.6 square over its 5.0 core (clean), the core slid half out, no core at all, and a
// core drawn to the same outline (poly on COMP still, clean).
#[case::dpf_1_h1("dummy_poly2/DPF.1.h1.gds.gz", "TOP", vec!["DPF.1", "DPF.1"], vec![])]
#[case::dpf_2b_h1("dummy_poly2/DPF.2b.h1.gds.gz", "TOP", vec!["DPF.2b", "DPF.2b"], vec![])]
// The 1.095 gap on the tile lines, and a 1.1 one that must stay clean.
#[case::dpf_2b_h2("dummy_poly2/DPF.2b.h2.gds.gz", "TOP", vec!["DPF.2b", "DPF.2b", "DPF.2b"], vec![])]
#[case::dpf_4_h1("dummy_poly2/DPF.4.h1.gds.gz", "TOP", vec!["DPF.4", "DPF.4"], vec![])]
// Abutting COMP and half over it.  Report finding 1.
#[case::dpf_4_h2("dummy_poly2/DPF.4.h2.gds.gz", "TOP", vec!["DPF.4", "DPF.4"], vec![])]
#[case::dpf_5_h1("dummy_poly2/DPF.5.h1.gds.gz", "TOP", vec!["DPF.5", "DPF.5"], vec![])]
#[case::dpf_5_h2("dummy_poly2/DPF.5.h2.gds.gz", "TOP", vec!["DPF.5", "DPF.5"], vec![])]
#[case::dpf_6a_h1("dummy_poly2/DPF.6a.h1.gds.gz", "TOP", vec!["DPF.6a", "DPF.6a"], vec![])]
// The band round an N-well boundary, at 0.995.  Report finding 2.
#[case::dpf_6a_h2("dummy_poly2/DPF.6a.h2.gds.gz", "TOP", vec!["DPF.6a", "DPF.6a"], vec![])]
#[case::dpf_6b_h1("dummy_poly2/DPF.6b.h1.gds.gz", "TOP", vec!["DPF.6b", "DPF.6b"], vec![])]
#[case::dpf_6b_h2("dummy_poly2/DPF.6b.h2.gds.gz", "TOP", vec!["DPF.6b", "DPF.6b"], vec![])]
#[case::dpf_6c_h1("dummy_poly2/DPF.6c.h1.gds.gz", "TOP", vec!["DPF.6c", "DPF.6c"], vec![])]
#[case::dpf_6c_h2("dummy_poly2/DPF.6c.h2.gds.gz", "TOP", vec!["DPF.6c", "DPF.6c"], vec![])]
#[case::dpf_6d_h1("dummy_poly2/DPF.6d.h1.gds.gz", "TOP", vec!["DPF.6d", "DPF.6d"], vec![])]
#[case::dpf_6d_h2("dummy_poly2/DPF.6d.h2.gds.gz", "TOP", vec!["DPF.6d", "DPF.6d"], vec![])]
#[case::dpf_8_h1("dummy_poly2/DPF.8.h1.gds.gz", "TOP", vec!["DPF.8", "DPF.8"], vec![])]
#[case::dpf_8_h2("dummy_poly2/DPF.8.h2.gds.gz", "TOP", vec!["DPF.8", "DPF.8"], vec![])]
#[case::dpf_9_h1("dummy_poly2/DPF.9.h1.gds.gz", "TOP", vec!["DPF.9", "DPF.9"], vec![])]
#[case::dpf_9_h2("dummy_poly2/DPF.9.h2.gds.gz", "TOP", vec!["DPF.9", "DPF.9"], vec![])]
#[case::dpf_11_h1("dummy_poly2/DPF.11.h1.gds.gz", "TOP", vec!["DPF.11", "DPF.11"], vec![])]
#[case::dpf_11_h2("dummy_poly2/DPF.11.h2.gds.gz", "TOP", vec!["DPF.11", "DPF.11"], vec![])]
#[case::dpf_12_h1("dummy_poly2/DPF.12.h1.gds.gz", "TOP", vec!["DPF.12", "DPF.12"], vec![])]
// Dummy poly abutting circuit Metal1 and running under it: the rule is a lateral space,
// and a fill with metal over it has none of it.  Report finding 1.
#[case::dpf_12_h2("dummy_poly2/DPF.12.h2.gds.gz", "TOP", vec!["DPF.12", "DPF.12"], vec![])]
#[case::dpf_13_h1("dummy_poly2/DPF.13.h1.gds.gz", "TOP", vec!["DPF.13", "DPF.13"], vec![])]
#[case::dpf_13_h2("dummy_poly2/DPF.13.h2.gds.gz", "TOP", vec!["DPF.13", "DPF.13"], vec![])]
#[case::dpf_14_h1("dummy_poly2/DPF.14.h1.gds.gz", "TOP", vec!["DPF.14", "DPF.14"], vec![])]
// Abutting IND_MK and half over it: the shared edge is DPF.14's space of nothing, the
// fill lying on the marker is DPF.15 ("Dummy poly2 should not exist under IND_MK").
#[case::dpf_14_h2("dummy_poly2/DPF.14.h2.gds.gz", "TOP", vec!["DPF.14", "DPF.15"], vec![])]
#[case::dpf_16_h1("dummy_poly2/DPF.16.h1.gds.gz", "TOP", vec!["DPF.16", "DPF.16"], vec![])]
// The same against MTPMARK, where the overlap half is DPF.17.
#[case::dpf_16_h2("dummy_poly2/DPF.16.h2.gds.gz", "TOP", vec!["DPF.16", "DPF.17"], vec![])]
#[case::dpf_19_h1("dummy_poly2/DPF.19.h1.gds.gz", "TOP", vec!["DPF.19", "DPF.19"], vec![])]
// And against PMNDMY, where it is DPF.18 ("cannot exit under the marking layer").
#[case::dpf_19_h2("dummy_poly2/DPF.19.h2.gds.gz", "TOP", vec!["DPF.18", "DPF.19"], vec![])]
// The 5.6 square DPF.1 asks for, and a truncated one.  DPF.10 is Appendix B's, as
// DCF.10 is (report finding 4, kept).
#[case::dpf_10_h1("dummy_poly2/DPF.10.h1.gds.gz", "TOP", vec![], vec![])]
// Dummy poly2 wholly under IND_MK (DPF.15), MTPMARK (DPF.17) and PMNDMY (DPF.18), each
// of which the manual states as its own rule.  Report finding 3.
// Fill 25.7 µm inside the PR_BNDRY polygon and 25.695 inside it.  DPF.7 is Appendix B's
// too (report finding 5, kept).
#[case::dpf_7_h1("dummy_poly2/DPF.7.h1.gds.gz", "TOP", vec![], vec![])]
#[case::dpf_15_h1("dummy_poly2/DPF.15.h1.gds.gz", "TOP", vec!["DPF.15"], vec![])]
#[case::dpf_17_h1("dummy_poly2/DPF.17.h1.gds.gz", "TOP", vec!["DPF.17"], vec![])]
#[case::dpf_18_h1("dummy_poly2/DPF.18.h1.gds.gz", "TOP", vec!["DPF.18"], vec![])]
fn hardening_dummy_poly2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("dummy_poly2", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

// --- Dummy metal (hardening/reports/gf180mcuD/dummy_metal.md).  Section 13.3.  The five
// levels are one template, so the battery is drawn on Metal1 and `DM.levels.h1` carries
// the same two gaps on all five.
#[rstest]
#[case::dm1_2b_h1("dummy_metal/DM1.2b.h1.gds.gz", "TOP", vec!["DM1.2b", "DM1.2b"], vec![])]
// The 0.975 gap on the tile lines, and a 0.98 one that must stay clean.  The notch
// probe is a 6 µm U, and a dummy metal shape is 2 µm both ways: DM.1 reports its length.
#[case::dm1_2b_h2("dummy_metal/DM1.2b.h2.gds.gz", "TOP", vec!["DM1.1", "DM1.2b", "DM1.2b", "DM1.2b"], vec![])]
#[case::dm1_3_h1("dummy_metal/DM1.3.h1.gds.gz", "TOP", vec!["DM1.3", "DM1.3"], vec![])]
// Fill abutting circuit Metal1 and half over it.  Report finding 1.
#[case::dm1_3_h2("dummy_metal/DM1.3.h2.gds.gz", "TOP", vec!["DM1.3", "DM1.3"], vec![])]
#[case::dm1_8_h1("dummy_metal/DM1.8.h1.gds.gz", "TOP", vec!["DM1.8", "DM1.8"], vec![])]
// Abutting OTP_MK, half over it, and wholly under it - "There should not be any dummy
// metal pattern fill in the following areas".  Report findings 1 and 2.
#[case::dm1_8_h2("dummy_metal/DM1.8.h2.gds.gz", "TOP", vec!["DM1.8", "DM1.8", "DM1.8"], vec![])]
// The other five layers DM.8 names, each 5.995 from a fill.
#[case::dm1_8_h3("dummy_metal/DM1.8.h3.gds.gz", "TOP", vec!["DM1.8", "DM1.8", "DM1.8", "DM1.8", "DM1.8"], vec![])]
// A 2.0 square (clean), a 1.995 one, a 2.005 one and a 2 x 4 bar: DM.1 fixes the dummy
// metal's width and length at 2.0 both ways.  The narrow square is under the width on
// both of its walls, which `min_width` names one at a time.
#[case::dm_1_h1("dummy_metal/DM.1.h1.gds.gz", "TOP", vec!["DM1.1", "DM1.1", "DM1.1", "DM1.1"], vec![])]
// Dummy Metal1 against the level above it and the level below it, at 1.0 (clean), 0.995
// and overlapping - DM.4/DM.6 and DM.5/DM.7.  The deck says nothing: read as a distance
// to the *circuit* metal of the neighbouring level those four rules cannot hold beside
// the 30 % density floor, and read as a distance to its *dummy* metal they cannot hold
// beside DM.9's 0.5 µm offset, which has the two patterns overlapping by design.  Both
// readings report a real design by the hundred thousand.  Left uncoded with the foundry's
// own runset, and flagged (report finding 4).
#[case::dm1_4_h1("dummy_metal/DM1.4.h1.gds.gz", "TOP", vec![], vec![])]
#[case::dm1_5_h1("dummy_metal/DM1.5.h1.gds.gz", "TOP", vec![], vec![])]
// A 0.975 dummy-to-dummy gap and a 1.995 dummy-to-drawn gap on each of the five levels.
#[case::dm_levels_h1("dummy_metal/DM.levels.h1.gds.gz", "TOP", vec!["DM1.2b", "DM1.3", "DM2.2b", "DM2.3", "DM3.2b", "DM3.3", "DM4.2b", "DM4.3", "DM5.2b", "DM5.3"], vec![])]
fn hardening_dummy_metal(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("dummy_metal", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

// --- Dummy exclude (hardening/reports/gf180mcuD/dummy_exclude.md).  Section 10.8, three
// rules on two marker layers.  `min_width` reports one marker per wall, so a narrow bar
// counts twice.
#[rstest]
// 0.8 and 0.795 bars on NDMY and on PMNDMY.
#[case::de_2_h1("dummy_exclude/DE.2.h1.gds.gz", "TOP", vec!["DE.2", "DE.2", "DE.2", "DE.2"], vec![])]
// A 0.795 NDMY strip lying on a 20 µm PMNDMY plate, and a 0.795 PMNDMY strip on a 20 µm
// NDMY plate: the rule is the size of an NDMY *or* of a PMNDMY, and each answers for
// itself.  Report finding 1.
#[case::de_2_h2("dummy_exclude/DE.2.h2.gds.gz", "TOP", vec!["DE.2", "DE.2", "DE.2", "DE.2"], vec![])]
// 75 x 200 (exactly the 15000 µm² cap), 80 x 200 (over the cap with one side over 80 -
// which is what the rule's second half allows), 100 x 200 and 80.005 x 200 (two sides
// over 80).  Report finding 2.
#[case::de_3_h1("dummy_exclude/DE.3.h1.gds.gz", "TOP", vec!["DE.3", "DE.3"], vec![])]
// NDMY to NDMY at 20 / 19.995, and corner to corner at 20.004 / 19.997.
#[case::de_4_h1("dummy_exclude/DE.4.h1.gds.gz", "TOP", vec!["DE.4", "DE.4"], vec![])]
// The 19.995 gap on the tile lines: opening on x = 40, a notch open across x = 140, a
// gap opening on y = 20, and a 20 µm gap on x = 40 that must stay clean.
#[case::de_4_h2("dummy_exclude/DE.4.h2.gds.gz", "TOP", vec!["DE.4", "DE.4", "DE.4"], vec![])]
fn hardening_dummy_exclude(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    assert_eq!(
        hardening("dummy_exclude", gds, topcell, &ignore),
        expected,
        "{gds}"
    );
}

// --- SRAM at 3.3 V (hardening/reports/gf180mcuD/sram_3p3.md).  Section 11.2 relaxes six
// of chapter 7's rules inside SRAMCORE, for the cells "without marking layer V5_XTOR".
// One deficient margin is one violating structure and carries one id, however many of its
// sides or walls are short; `min_width` keeps its two markers per narrow bar.
#[rstest]
// Poly over a contact by 0.04 (clean) and 0.035 inside the marker, and the same two
// outside it, where chapter 7's CO.3 of 0.07 speaks instead.
#[case::s_co_3_lv_h1("sram_3p3/S.CO.3_LV.h1.gds.gz", "TOP", vec!["S.CO.3_LV"], vec![])]
// Both straight margins at 0.04 with the poly's corner chamfered: 0.0212 from the
// contact's corner (fires) and 0.0424 (clean).  Report finding 1.
#[case::s_co_3_lv_h2("sram_3p3/S.CO.3_LV.h2.gds.gz", "TOP", vec!["S.CO.3_LV"], vec![])]
// COMP over a contact by 0.03 (clean), 0.025, and the same two chamfers at 0.0212 and
// 0.0318.  Report finding 1.
#[case::s_co_4_lv_h1("sram_3p3/S.CO.4_LV.h1.gds.gz", "TOP", vec!["S.CO.4_LV"; 2], vec![])]
// A contact the poly's edge cuts and one the COMP's edge cuts, each with a twin outside
// the layer whose edge it abuts - which is no enclosure at all.
#[case::s_co_3_lv_h3("sram_3p3/S.CO.3_LV.h3.gds.gz", "TOP", vec!["S.CO.3_LV", "S.CO.4_LV"], vec![])]
// Adjacent Metal1 margins: 0.035/0.02 (clean), 0.035/0.015, 0.04/0.015 (no trigger,
// clean), 0.015/0.03.
#[case::s_co_6_ii_lv_h1("sram_3p3/S.CO.6_ii_LV.h1.gds.gz", "TOP", vec!["S.CO.6_ii_LV"; 2], vec![])]
// Metal1 tracks 0.22 (clean), 0.215, 0.225 (clean) inside the marker and 0.215 outside.
#[case::s_m1_1_lv_h1("sram_3p3/S.M1.1_LV.h1.gds.gz", "TOP", vec!["S.M1.1_LV"; 2], vec![])]
// N-well over P+ active by 0.4 (clean), 0.395, a 45° well corner at 0.2828 and at 0.4243
// (clean), and the active running out of the well.  Report finding 1.
#[case::s_df_4c_lv_h1("sram_3p3/S.DF.4c_LV.h1.gds.gz", "TOP", vec!["S.DF.4c_LV"; 3], vec![])]
// N+ active to N-well at 0.4 (clean), 0.395, corner to corner at 0.4243 (clean) and
// 0.396, and an active whose wall lies on the well's.  Report finding 2.
#[case::s_df_16_lv_h1("sram_3p3/S.DF.16_LV.h1.gds.gz", "TOP", vec!["S.DF.16_LV"; 3], vec![])]
// The same 0.035 poly margin in five cores: bare, under Dualgate, under Dualgate and
// V5_XTOR, under V5_XTOR, and with Dualgate abutting.  The two that carry V5_XTOR are
// section 11.1's.  Report finding 3.
#[case::s_co_3_lv_h4("sram_3p3/S.CO.3_LV.h4.gds.gz", "TOP", vec!["S.CO.3_LV"; 3], vec![])]
// The same five cores under a rule the deck derives from `sram_lv`: the Dualgate ones
// hold their active at 0.395, the others at 0.4.  Report finding 4.
#[case::s_df_4c_lv_h3("sram_3p3/S.DF.4c_LV.h3.gds.gz", "TOP", vec!["S.DF.4c_LV"; 2], vec![])]
// A 6 µm well holding its active by 1.0 with the marker ending 0.2 past the active, and
// the same well wholly under the marker.  Report finding 5.
#[case::s_df_4c_lv_h2("sram_3p3/S.DF.4c_LV.h2.gds.gz", "TOP", vec![], vec![])]
// Four 0.035 poly margins straddling x = 20, x = 40, x = 42 and y = 20, and a 0.395
// N-well gap straddling x = 20.
#[case::s_co_3_lv_h5("sram_3p3/S.CO.3_LV.h5.gds.gz", "TOP", vec!["S.CO.3_LV", "S.CO.3_LV", "S.CO.3_LV", "S.CO.3_LV", "S.DF.16_LV"], vec![])]
fn hardening_sram_3p3(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("sram_3p3", gds, topcell, &ignore), want, "{gds}");
}

// --- SRAM at 5 V (hardening/reports/gf180mcuD/sram_5p0.md).  Section 11.1 relaxes eight
// rules for the cores that carry V5_XTOR.  Every fixture lies under a Dualgate, so the
// half outside the core answers to the base `_MV` rule.
#[rstest]
// COMP over a contact by 0.04 (clean), 0.035, a 45° COMP corner at 0.0283 and at 0.046
// (clean), and 0.035 outside the core, where CO.4's 0.07 speaks.
#[case::s_co_4_mv_h1("sram_5p0/S.CO.4_MV.h1.gds.gz", "TOP", vec!["S.CO.4_MV"; 2], vec![])]
// A contact the COMP's edge cuts, and one outside the COMP whose edge lies on the COMP's.
#[case::s_co_4_mv_h2("sram_5p0/S.CO.4_MV.h2.gds.gz", "TOP", vec!["S.CO.4_MV"], vec![])]
// N-well over P+ active by 0.45 (clean), 0.445, a 45° well corner at 0.318 and at 0.495
// (clean), and the active running out of the well.
#[case::s_df_4c_mv_h1("sram_5p0/S.DF.4c_MV.h1.gds.gz", "TOP", vec!["S.DF.4c_MV"; 3], vec![])]
// Source/drain overhang 0.32 (clean) and 0.315, and 0.7 with the active's corner
// chamfered to pass 0.354 (clean) and 0.212 from the gate's wall.
#[case::s_df_6_mv_h1("sram_5p0/S.DF.6_MV.h1.gds.gz", "TOP", vec!["S.DF.6_MV"; 2], vec![])]
// P+ active to LVPWELL inside the deep well at 0.45 (clean), 0.445, corner to corner at
// 0.4525 (clean) and 0.4384, and an active whose wall lies on the well's.  Report
// finding 1.
#[case::s_df_7_mv_h1("sram_5p0/S.DF.7_MV.h1.gds.gz", "TOP", vec!["S.DF.7_MV"; 3], vec![])]
// LVPWELL over N+ active by 0.45 (clean), 0.445, a 45° P-well corner at 0.318 and at
// 0.495 (clean), and the active running out of the P-well.
#[case::s_df_8_mv_h1("sram_5p0/S.DF.8_MV.h1.gds.gz", "TOP", vec!["S.DF.8_MV"; 3], vec![])]
// The same arrangement at 0.44 and 0.46 in a core with neither V5_XTOR nor Dualgate: a
// 3.3 V SRAM, whose DF.8 is chapter 7's 0.43.
#[case::s_df_8_mv_h2("sram_5p0/S.DF.8_MV.h2.gds.gz", "TOP", vec![], vec![])]
// N+ active to N-well at 0.45 (clean), 0.445, corner to corner at 0.4525 (clean) and
// 0.4384, and an active whose wall lies on the well's.  Report finding 1.
#[case::s_df_16_mv_h1("sram_5p0/S.DF.16_MV.h1.gds.gz", "TOP", vec!["S.DF.16_MV"; 3], vec![])]
// Field poly to an active at 0.12 (clean), 0.115, corner to corner at 0.1216 (clean) and
// 0.1131, a corner touch, and a wall lying on the active's.  Report finding 1, and
// finding 2 beside it: 11.1's two poly rules carry one value between them, so each gap
// is both of them, as chapter 7's PL.5a_MV and PL.5b_MV are at 0.3.
#[case::s_pl_5a_mv_h1("sram_5p0/S.PL.5a_MV.h1.gds.gz", "TOP", vec!["S.PL.5a_MV"; 4].into_iter().chain(vec!["S.PL.5b_MV"; 4]).collect::<Vec<_>>(), vec![])]
// A gate whose field stretch runs into its own active's slot at 0.115 (fires) and 0.12
// (clean), and a gate over one active passing 0.115 from a second.  Report finding 2 -
// and both ids on each gap, as on `s_pl_5a_mv_h1`.
#[case::s_pl_5b_mv_h1("sram_5p0/S.PL.5b_MV.h1.gds.gz", "TOP", vec!["S.PL.5a_MV", "S.PL.5a_MV", "S.PL.5b_MV", "S.PL.5b_MV"], vec![])]
// The same 0.035 contact margin in a core wholly under V5_XTOR, one half under it, one
// V5_XTOR abuts, a bare core, and no core at all.
#[case::s_co_4_mv_h3("sram_5p0/S.CO.4_MV.h3.gds.gz", "TOP", vec!["S.CO.4_MV"; 2], vec![])]
// Three 0.035 contact margins straddling x = 20, x = 42 and y = 20, and a 0.445 N-well
// gap straddling x = 40.
#[case::s_co_4_mv_h4("sram_5p0/S.CO.4_MV.h4.gds.gz", "TOP", vec!["S.CO.4_MV", "S.CO.4_MV", "S.CO.4_MV", "S.DF.16_MV"], vec![])]
fn hardening_sram_5p0(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("sram_5p0", gds, topcell, &ignore), want, "{gds}");
}

// --- The base rules under the SRAM marker.  Section 11 names the rules that are
// "different from 3.3V/(5V)6V rules"; every other rule of chapter 7 still applies inside
// an SRAM core.  The contact deck drops the whole of SRAMCORE, so the two class layouts
// above are run through it as well.  Report sram_3p3 finding 3 and sram_5p0 finding 3.
#[rstest]
// Five 3.3 V-class cores: the two that carry V5_XTOR have no S.CO.3_MV to answer to, so
// chapter 7's CO.3 of 0.07 is theirs and 0.035 fires.
#[case::s_co_3_lv_h4_base("sram_3p3/S.CO.3_LV.h4.gds.gz", "TOP", vec!["CO.3"; 2], vec![])]
// Five 5 V-class cores.  CO.4 is the one rule section 11 replaces on both sides - 11.1
// for the cells V5_XTOR marks, 11.2 for the cells it does not - so every core has a
// CO.4 of its own and only the contact with no core at all owes chapter 7's 0.07.
#[case::s_co_4_mv_h3_base("sram_5p0/S.CO.4_MV.h3.gds.gz", "TOP", vec!["CO.4"], vec!["CO.6"])]
fn hardening_sram_base(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("contact", gds, topcell, &ignore), want, "{gds}");
}

// --- DRC_BJT (hardening/reports/gf180mcuD/drc_bjt.md).  Section 10.7 is three rules over
// a marker that has to recognise a vertical NPN from its three terminals.
#[rstest]
// A complete NPN whose deep well runs 1 µm past the marker, then the same less the
// LVS_BJT, less the base and less the collector - none of them a transistor - and a
// complete NPN with its well inside.
#[case::bjt_1_h1("drc_bjt/BJT.1.h1.gds.gz", "TOP", vec!["BJT.1"], vec![])]
// The deep well flush with the marker on every side (an overlap of exactly 0, clean),
// 0.005 past it, and wholly inside.
#[case::bjt_1_h2("drc_bjt/BJT.1.h2.gds.gz", "TOP", vec!["BJT.1"], vec![])]
// Two deep wells under one marker: the NPN's, inside, and a second that runs out of it.
// Report finding 1.
#[case::bjt_1_h3("drc_bjt/BJT.1.h3.gds.gz", "TOP", vec!["BJT.1"], vec![])]
// A substrate tap inside the marker (clean), one the marker's edge cuts, a marker whose
// only P+ is in an N-well, one tap inside with a second crossing out, and a tap whose
// edge lies on the marker's (clean).  Report finding 1.
#[case::bjt_2_h1("drc_bjt/BJT.2.h1.gds.gz", "TOP", vec!["BJT.2"; 3], vec![])]
// An unrelated active 0.1 from the marker (clean), 0.095, corner to corner at 0.1018
// (clean) and 0.0990, one whose wall lies on the marker's, and one that overlaps the
// marker (related, exempt).  Report finding 2.
#[case::bjt_3_h1("drc_bjt/BJT.3.h1.gds.gz", "TOP", vec!["BJT.3"; 3], vec![])]
// The same 0.095 gap straddling x = 20, x = 42 and y = 20.
#[case::bjt_3_h2("drc_bjt/BJT.3.h2.gds.gz", "TOP", vec!["BJT.3"; 3], vec![])]
fn hardening_drc_bjt(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("drc_bjt", gds, topcell, &ignore), want, "{gds}");
}

// --- LVS_BJT (hardening/reports/gf180mcuD/lvs_bjt.md).  One rule, and all of its work is
// deciding which COMP is an emitter.
#[rstest]
// An N+ active in the deep well with the marker over all of it (clean), over half of it,
// only abutting it, an active the well's edge cuts (no emitter), one outside every well,
// and one the marker leaves a 0.005 µm sliver of.
#[case::lvs_bjt_1_h1("lvs_bjt/LVS_BJT.1.h1.gds.gz", "TOP", vec!["LVS_BJT.1"; 3], vec![])]
// A P+ active in an N-well with the marker over all of it (clean) and over half; the same
// active in a deep well instead, an N+ active in the N-well, and a P+ active the N-well's
// edge cuts - none of them a PNP emitter.
#[case::lvs_bjt_1_h2("lvs_bjt/LVS_BJT.1.h2.gds.gz", "TOP", vec!["LVS_BJT.1"], vec![])]
// The same half-covered emitter straddling x = 20, x = 42 and y = 20.
#[case::lvs_bjt_1_h3("lvs_bjt/LVS_BJT.1.h3.gds.gz", "TOP", vec!["LVS_BJT.1"; 3], vec![])]
fn hardening_lvs_bjt(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    let mut want: Vec<String> = expected.into_iter().map(ToString::to_string).collect();
    want.sort();
    assert_eq!(hardening("lvs_bjt", gds, topcell, &ignore), want, "{gds}");
}

// --- Native-VT device (hardening/reports/gf180mcuD/nat.md).  A native transistor is an
// N+ active clear of every well with a poly gate across it, under a NAT marker; Dualgate
// over the marker makes it the 6 V kind.  `min_gate_length` reports one marker per wall.
#[rstest]
// The marker holding the active by 2.0 (clean), by 1.995 on one wall, and a third device
// whose active runs out through the marker's edge - held by nothing, which is less than
// the two microns NAT.1 asks.  Report finding 1: neither tool reads the crossing shape.
#[case::nat_1_h1("nat/NAT.1.h1.gds.gz", "TOP", vec!["NAT.1", "NAT.1"])]
// The marker's top-right corner chamfered so its 45° wall stands 1.994 µm from the
// active's corner while both axis distances are 2.2.  An enclosure is euclidian (settled
// 2026-09-21), so it fires.  Report finding 2: gdscheck reads the projection and is
// silent; KLayout fires.
#[case::nat_1_h2("nat/NAT.1.h2.gds.gz", "TOP", vec!["NAT.1"])]
// An unrelated active 0.3 from the marker (clean), 0.295, sharing the marker's edge, and
// one 0.22 × 0.2 off its corner - 0.2966 euclidian.  Report finding 3: the shared edge is
// a space of nothing and gdscheck does not report it.
#[case::nat_2_h1("nat/NAT.2.h1.gds.gz", "TOP", vec!["NAT.2", "NAT.2", "NAT.2"])]
// An N-well 0.5 from the marker (clean), 0.495, and sharing its edge.  Report finding 3.
#[case::nat_3_h1("nat/NAT.3.h1.gds.gz", "TOP", vec!["NAT.3", "NAT.3"])]
// The channel at 1.8 and 1.795, at 3.3 V and again under a Dualgate that covers the
// marker whole - one id each, two markers per gate, one per wall.
#[case::nat_4_h1("nat/NAT.4.h1.gds.gz", "TOP", vec!["NAT.4", "NAT.4", "NAT.5", "NAT.5"])]
// A 1.795 channel whose marker Dualgate only abuts: no thick oxide over the device, so it
// is the 3.3 V one and NAT.4 is its rule.
#[case::nat_4_h2("nat/NAT.4.h2.gds.gz", "TOP", vec!["NAT.4", "NAT.4"])]
// Two actives under one marker, each with its own contact and its own Metal1 plate: two
// potentials.
#[case::nat_6_h1("nat/NAT.6.h1.gds.gz", "TOP", vec!["NAT.6"])]
// The same pair with one Metal1 plate over both contacts - one potential, which is the
// only thing the rule forbids two of.  Report finding 4: both tools report it anyway.
#[case::nat_6_h2("nat/NAT.6.h2.gds.gz", "TOP", vec![])]
// Bare markers 0.74 apart (clean) and 0.735 apart three times over, plus a 0.735 slot;
// every gap centred on a tile line at x = 20, 21, 40, 42 and y = 20.
#[case::nat_7_h1("nat/NAT.7.h1.gds.gz", "TOP", vec!["NAT.7"; 4])]
// A marker Dualgate covers whole (clean), one it covers from the middle rightwards, and
// one it abuts without covering - which is a 3.3 V device and has no overlap to measure.
#[case::nat_8_h1("nat/NAT.8.h1.gds.gz", "TOP", vec!["NAT.8"])]
// Unrelated poly 0.3 from the marker (clean), 0.295, and sharing its edge.  Report
// finding 3.
#[case::nat_9_h1("nat/NAT.9.h1.gds.gz", "TOP", vec!["NAT.9", "NAT.9"])]
// Two gates on one active joined over the field by a poly bridge under the marker: poly
// running from one gate of the marker to another is the interconnect the rule forbids.
#[case::nat_9_h2("nat/NAT.9.h2.gds.gz", "TOP", vec!["NAT.9"])]
// An N-well with one corner over the marker, one wholly inside it, and one abutting its
// edge - the last is not a well inside the marker but NAT.3's zero space.  Report
// finding 3.
#[case::nat_10_h1("nat/NAT.10.h1.gds.gz", "TOP", vec!["NAT.10", "NAT.10", "NAT.3"])]
// An N+ active under the marker with no poly on it, one whose gate only abuts it (both
// tools read that as intersecting), and a P+ active, which is not an NCOMP.  The P+ and
// the N+ under that third marker are two potentials, so NAT.6 comes with it.
#[case::nat_11_h1("nat/NAT.11.h1.gds.gz", "TOP", vec!["NAT.11", "NAT.6"])]
// Poly under the marker reaching no active, poly touching an active at a single point
// (intersecting in both tools), and poly over an active carrying RES_MK.
#[case::nat_12_h1("nat/NAT.12.h1.gds.gz", "TOP", vec!["NAT.12", "NAT.12"])]
fn hardening_nat(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    let mut expected = expected;
    expected.sort();
    assert_eq!(hardening("nat", gds, topcell, &[]), expected, "{gds}");
}

// --- OTP marker (hardening/reports/gf180mcuD/otp_mk.md).  The 3.3 V rule set with
// tighter numbers inside OTP_MK; the gate lines run horizontally because O.PL.ORT
// forbids the other orientation.  `min_gate_length` and `min_enclosure` report one
// marker per wall.
#[rstest]
// A contact 0.13 from the gate line (clean), 0.125, and one flush against it.  Report
// finding 3: the shared edge is a space of nothing and gdscheck does not report it.
#[case::o_co_7_h1("otp_mk/O.CO.7.h1.gds.gz", "TOP", vec!["O.CO.7", "O.CO.7"])]
// Actives 0.24 apart (clean) and 0.235 apart three times over, every gap on a tile line
// at x = 20, 21, 40 and y = 20, plus a pair 0.18 × 0.15 off a corner - 0.2343 euclidian.
// The pair drawn edge to edge is one region after merging and is no space at all.
#[case::o_df_3a_h1("otp_mk/O.DF.3a.h1.gds.gz", "TOP", vec!["O.DF.3a"; 4])]
// One solid 3 × 3 COMP with no notch in it, under a marker shaped like a U whose slot is
// 0.2 µm.  Report finding 2: both tools cut the COMP to the marker and read the marker's
// own slot as a notch in the active.
#[case::o_df_3a_h2("otp_mk/O.DF.3a.h2.gds.gz", "TOP", vec![])]
// The source/drain overhang at 0.22 (clean) and 0.215, on both walls of the second gate.
#[case::o_df_6_h1("otp_mk/O.DF.6.h1.gds.gz", "TOP", vec!["O.DF.6", "O.DF.6"])]
// The same overhang over a 45° source edge: straight up from the gate's top wall there is
// exactly 0.22 but only 0.1556 to the chamfer; then 0.19 straight up; then 0.02.  Report
// finding 1: gdscheck reads none of the three, KLayout reads the last.
#[case::o_df_6_h2("otp_mk/O.DF.6.h2.gds.gz", "TOP", vec!["O.DF.6"; 3])]
// An active of 0.1444 µm² (clean) and one of 0.1425, then a 1 µm² active with the marker
// over a 0.3 µm corner of it.  Report finding 2: the cut leaves 0.09 µm² and both tools
// report a COMP seven times the minimum.
#[case::o_df_9_h1("otp_mk/O.DF.9.h1.gds.gz", "TOP", vec!["O.DF.9"])]
// The channel length at 0.22 (clean) and 0.215.
#[case::o_pl_2_h1("otp_mk/O.PL.2.h1.gds.gz", "TOP", vec!["O.PL.2", "O.PL.2"])]
// Two gate lines over one active 0.18 apart (clean) and 0.175 apart, a 0.175 slot in a
// field poly plate crossing x = 21, and two plates drawn edge to edge, which merge.
#[case::o_pl_3a_h1("otp_mk/O.PL.3a.h1.gds.gz", "TOP", vec!["O.PL.3a", "O.PL.3a"])]
// The poly end cap at 0.14 (clean) and 0.135 on both ends, then a gate line stopping
// 0.2 inside the active - no end cap at all, and a channel edge that runs along y.
#[case::o_pl_4_h1("otp_mk/O.PL.4.h1.gds.gz", "TOP", vec!["O.DF.6", "O.PL.4", "O.PL.4", "O.PL.ORT"])]
// A horizontal gate (clean), one turned a quarter, and a turned one under V5_XTOR, which
// is the 5 V cell the rule's own column marks NA.
#[case::o_pl_ort_h1("otp_mk/O.PL.ORT.h1.gds.gz", "TOP", vec!["O.PL.ORT", "O.PL.ORT"])]
// The block covering the active by 0.04 (clean) and by 0.035.
#[case::o_sb_11_h1("otp_mk/O.SB.11.h1.gds.gz", "TOP", vec!["O.SB.11"])]
// Blocks of 1.488 µm² (clean) and 1.482 at 3.3 V, 2.0 (clean) and 1.99375 under V5_XTOR,
// and a 1 µm² block under Dualgate alone.  Report finding 4: a Dualgate OTP cell is in
// neither class, so its area is never checked.
#[case::o_sb_13_h1("otp_mk/O.SB.13.h1.gds.gz", "TOP", vec!["O.SB.13_LV", "O.SB.13_MV", "O.SB.13_MV"])]
// Blocks 0.28 apart (clean) and 0.275 apart across x = 42, a 0.275 slot crossing x = 21,
// and two blocks drawn edge to edge, which merge.
#[case::o_sb_2_h1("otp_mk/O.SB.2.h1.gds.gz", "TOP", vec!["O.SB.2", "O.SB.2"])]
// A block over no active, 0.09 from an active no block covers (clean), 0.085, and flush
// against one.  Report finding 3.
#[case::o_sb_3_h1("otp_mk/O.SB.3.h1.gds.gz", "TOP", vec!["O.SB.3", "O.SB.3"])]
// A contact 0.03 from the block (clean), 0.025, flush against it, and under it.  Report
// finding 3.
#[case::o_sb_4_h1("otp_mk/O.SB.4.h1.gds.gz", "TOP", vec!["O.SB.4"; 3])]
// A block 0.1 above a gate line it does not cover (clean), 0.095, the same 0.095 under
// Dualgate - the 5 V rule, which Appendix B lists as not coded - and again under V5_XTOR
// alone, which carries no thick oxide and is the 3.3 V cell.
#[case::o_sb_5b_h1("otp_mk/O.SB.5b.h1.gds.gz", "TOP", vec!["O.SB.5b_LV", "O.SB.5b_LV"])]
// The block reaching 0.1 past the gate line it blocks (clean), 0.095 on both walls, and a
// block whose ends the poly line runs out of - which is how every blocked line is drawn.
#[case::o_sb_9_h1("otp_mk/O.SB.9.h1.gds.gz", "TOP", vec!["O.SB.9", "O.SB.9"])]
fn hardening_otp_mk(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    let mut expected = expected;
    expected.sort();
    assert_eq!(hardening("otp_mk", gds, topcell, &[]), expected, "{gds}");
}

// --- YMTP marker (hardening/reports/gf180mcuD/ymtp_mk.md).  A handful of the ordinary
// rules restated with their own numbers inside YMTP_MK, half of them at one voltage
// only; a marker Dualgate covers is the 5 V one and every other marker is the 3.3 V one.
// `min_width`, `min_gate_length` and `min_enclosure` report one marker per wall.
#[rstest]
// An N+ active 0.27 from an N-well (clean), 0.265, and one flush against it, then the
// 5 V pair at 0.23 and 0.225.  Report finding 2: the shared edge is a space of nothing
// and gdscheck does not report it.
#[case::y_df_16_h1("ymtp_mk/Y.DF.16.h1.gds.gz", "TOP", vec!["Y.DF.16_LV", "Y.DF.16_LV", "Y.DF.16_MV"])]
// The source/drain overhang at 0.15 (clean) and 0.145 under Dualgate, the same 0.145 with
// OTP_MK over the active, and the same 0.145 at 3.3 V - where the manual's column is NA
// and this deck has nothing to say.
#[case::y_df_6_h1("ymtp_mk/Y.DF.6.h1.gds.gz", "TOP", vec!["Y.DF.6_MV", "Y.DF.6_MV"])]
// Wells 1.0 apart (clean) and 0.995 apart across x = 21 at 3.3 V, 0.995 across x = 40
// under Dualgate, and a 0.995 notch across y = 20.
#[case::y_nw_2b_h1("ymtp_mk/Y.NW.2b.h1.gds.gz", "TOP", vec!["Y.NW.2b_LV", "Y.NW.2b_LV", "Y.NW.2b_MV"])]
// A 0.995 pair inside DNWELL, which the rule is outside of; a 0.995 pair under a marker
// Dualgate only abuts, which is the 3.3 V marker; and one solid 4 × 3 well with no notch
// in it under a marker shaped like a U.  Report finding 3: both tools cut the well to the
// marker and read the marker's own slot as a notch in the well.
#[case::y_nw_2b_h2("ymtp_mk/Y.NW.2b.h2.gds.gz", "TOP", vec!["Y.NW.2b_LV"])]
// A poly line 0.13 wide (clean), 0.125 wide, and 0.125 wide with PLFUSE over part of it -
// which the rule excludes whole - then a 0.5 wide line in a 5 V marker, where the manual
// gives no width at all, and the same line wholly under PLFUSE.
#[case::y_pl_1_h1("ymtp_mk/Y.PL.1.h1.gds.gz", "TOP", vec!["Y.PL.1_LV", "Y.PL.1_LV", "Y.PL.1_MV"])]
// The channel at 0.13 (clean), 0.125, and 0.125 under OTP_MK, then 0.47 (clean) and 0.465
// under Dualgate.  A gate line narrower than 0.13 is also an interconnect narrower than
// 0.13, so Y.PL.1_LV reads the two uncovered 3.3 V gates as well.
#[case::y_pl_2_h1("ymtp_mk/Y.PL.2.h1.gds.gz", "TOP", vec!["Y.PL.1_LV", "Y.PL.1_LV", "Y.PL.1_LV", "Y.PL.1_LV", "Y.PL.2_LV", "Y.PL.2_LV", "Y.PL.2_MV", "Y.PL.2_MV"])]
// The poly end cap at 0.16 (clean) and 0.155 under Dualgate, and the same 0.155 at 3.3 V,
// where the manual's column is NA.
#[case::y_pl_4_h1("ymtp_mk/Y.PL.4.h1.gds.gz", "TOP", vec!["Y.PL.4_MV", "Y.PL.4_MV"])]
// Field poly 0.04 from an active (clean), 0.035, and flush against it, then the 5 V pair
// at 0.2 and 0.195.  5a and 5b are one measurement under two ids, so every violation
// carries both.  Report finding 2.
#[case::y_pl_5_h1("ymtp_mk/Y.PL.5.h1.gds.gz", "TOP", vec!["Y.PL.5a_LV", "Y.PL.5a_LV", "Y.PL.5a_MV", "Y.PL.5b_LV", "Y.PL.5b_LV", "Y.PL.5b_MV"])]
fn hardening_ymtp_mk(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    let mut expected = expected;
    expected.sort();
    assert_eq!(hardening("ymtp_mk", gds, topcell, &[]), expected, "{gds}");
}

// --- eFuse (hardening/reports/gf180mcuD/efuse.md).  Nine of this section's rules fix a
// dimension rather than bounding it, so the step past the value has two sides; these
// fixtures take the long one, which the deck's own bad halves never do.  Every probe is
// a whole fuse - cathode, link and anode, inside EFUSE_MK and P+, LVS_SOURCE on the
// anode - because the deck tells the parts apart by those four layers together.
#[rstest]
// The fuse the manual describes, with nothing wrong with it: no rule of the section may
// speak.
#[case::ef_00_h1("efuse/EF.00.h1.gds.gz", "TOP", vec![])]
// Bare PLFUSE bars 2 µm long and 0.18 (clean), 0.175 and 0.185 wide.  The manual fixes
// the width both ways, so the wide bar fires as surely as the narrow one - and neither
// tool mistakes the bar's 2 µm length for a width.
#[case::ef_02_h1("efuse/EF.02.h1.gds.gz", "TOP", vec!["EF.02"; 3])]
// The link 1.26 long (clean), 1.255 and 1.265.  The whole poly runs 1.84 + 1.26 + 2.43,
// so moving the link moves EF.21's 5.53 with it.
#[case::ef_03_h1("efuse/EF.03.h1.gds.gz", "TOP", vec!["EF.03", "EF.03", "EF.03", "EF.03", "EF.21", "EF.21", "EF.21", "EF.21"])]
// The two pads' four fixed dimensions, each one step *over* the value: cathode 2.265
// wide, cathode 1.845 long, anode 1.07 wide, anode 2.435 long.  A longer pad lengthens
// the poly (EF.21) and a wider one narrows the shoulder where the link meets it
// (EF.22a, EF.22b).
#[case::ef_06_h1("efuse/EF.06.h1.gds.gz", "TOP", vec!["EF.06", "EF.06", "EF.07", "EF.07", "EF.08", "EF.08", "EF.09", "EF.09", "EF.21", "EF.21", "EF.21", "EF.21", "EF.22a", "EF.22b", "EF.22b"])]
// Two fuses cathode to cathode at 0.26 (clean) and at 0.255.
#[case::ef_10_h1("efuse/EF.10.h1.gds.gz", "TOP", vec!["EF.10"])]
// Two fuses anode to anode at 0.26 (clean) and at 0.255.
#[case::ef_11_h1("efuse/EF.11.h1.gds.gz", "TOP", vec!["EF.11"])]
// The cathode's contacts 0.155 from the link's end (clean), 0.15, and flush against it -
// which is a space of nothing and a contact touching the link, so EF.15 comes with it.
// Report finding 1: gdscheck does not read the shared edge as a space.
#[case::ef_12_h1("efuse/EF.12.h1.gds.gz", "TOP", vec!["EF.12", "EF.12", "EF.15"])]
// The same at the anode, 0.14 / 0.135 / flush.  Report finding 1.
#[case::ef_13_h1("efuse/EF.13.h1.gds.gz", "TOP", vec!["EF.13", "EF.13", "EF.15"])]
// LVS_SOURCE ending exactly on the marker's edge - the zero enclosure the rule asks for -
// and the same source running 1 µm past it.
#[case::ef_14_h1("efuse/EF.14.h1.gds.gz", "TOP", vec!["EF.14"])]
// A contact 0.005 short of the link (clean of EF.15, inside EF.12's 0.155), one sharing
// the link's end edge, and one meeting its corner at a single point.  Report finding 1:
// the shared edge is the one of the three gdscheck does not read as a space.
#[case::ef_15_h1("efuse/EF.15.h1.gds.gz", "TOP", vec!["EF.12", "EF.12", "EF.12", "EF.15", "EF.15"])]
// Four contacts on each pad (clean), three on a cathode, five on a cathode, five on an
// anode - the rule fixes the number, so too many is as wrong as too few.
#[case::ef_16_h1("efuse/EF.16.h1.gds.gz", "TOP", vec!["EF.16a", "EF.16a", "EF.16b"])]
// Markers 0.26 apart across x = 20 (clean) and 0.255 apart across x = 40, and a bare
// marker with a 0.255 slot cut into it.
#[case::ef_17_h1("efuse/EF.17.h1.gds.gz", "TOP", vec!["EF.17", "EF.17"])]
// Metal1 drawn flush against the link's long wall - the zero space EF.19 allows and no
// crossing for EF.18 - and Metal1 over the link.
#[case::ef_19_h1("efuse/EF.19.h1.gds.gz", "TOP", vec!["EF.18", "EF.19"])]
// An active 2.73 from the link (clean), 2.725, and one flush against its wall.  Report
// finding 1.
#[case::ef_20_h1("efuse/EF.20.h1.gds.gz", "TOP", vec!["EF.20", "EF.20"])]
// The rest of the rule's neighbour list at 2.725: Nplus, ESD, SAB and Resistor.
#[case::ef_20_h2("efuse/EF.20.h2.gds.gz", "TOP", vec!["EF.20"; 4])]
fn hardening_efuse(#[case] gds: &str, #[case] topcell: &str, #[case] expected: Vec<&str>) {
    let mut expected = expected;
    expected.sort();
    assert_eq!(hardening("efuse", gds, topcell, &[]), expected, "{gds}");
}
