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
    &[("MDP.1", 12), ("MDP.10", 16), ("MDP.10a", 7), ("MDP.10b", 4), ("MDP.11", 36), ("MDP.12", 4), ("MDP.13a", 1), ("MDP.13b", 21), ("MDP.13c", 3), ("MDP.15", 1), ("MDP.16a", 2), ("MDP.16b", 2), ("MDP.17a", 4), ("MDP.17c", 1), ("MDP.1a", 2), ("MDP.2", 25), ("MDP.3ai", 91), ("MDP.3aii", 8), ("MDP.3b", 4), ("MDP.3d", 2), ("MDP.4", 2), ("MDP.4a", 8), ("MDP.4b", 4), ("MDP.5", 8), ("MDP.5a", 5), ("MDP.6", 3), ("MDP.6a", 21), ("MDP.7", 1), ("MDP.8", 1), ("MDP.9a", 38), ("MDP.9b", 12), ("MDP.9d", 15), ("MDP.9ei", 6), ("MDP.9eii", 4), ("MDP.9f", 1)]
)]
#[case::nat(
    "nat", "nat.gds.gz", "10_5_NAT",
    &[("NAT.1", 2), ("NAT.10", 1), ("NAT.11", 1), ("NAT.12", 2), ("NAT.2", 4), ("NAT.3", 3), ("NAT.4", 24), ("NAT.5", 10), ("NAT.6", 7), ("NAT.7", 2), ("NAT.8", 5), ("NAT.9", 4)]
)]
#[case::ldnmos(
    "ldnmos", "ldnmos.gds.gz", "10_12_1_MDN",
    &[("MDN.1", 44), ("MDN.10a", 65), ("MDN.10b", 4), ("MDN.10c", 17), ("MDN.10ei", 3), ("MDN.10eii", 2), ("MDN.10f", 6), ("MDN.11", 109), ("MDN.12", 19), ("MDN.13a", 8), ("MDN.13b", 9), ("MDN.13c", 6), ("MDN.13d", 18), ("MDN.14", 30), ("MDN.15a", 44), ("MDN.15b", 2), ("MDN.17", 74), ("MDN.2a", 24), ("MDN.2b", 27), ("MDN.3a", 9), ("MDN.3b", 4), ("MDN.4a", 29), ("MDN.4b", 16), ("MDN.5ai", 31), ("MDN.5aii", 4), ("MDN.5b", 8), ("MDN.6", 14), ("MDN.6a", 6), ("MDN.7", 81), ("MDN.7a", 274), ("MDN.8a", 10), ("MDN.8b", 14), ("MDN.9", 7)]
)]
#[case::lvpwell(
    "lvpwell", "lvpwell.gds.gz", "7_3_LVPWELL",
    &[("LPW.11", 14), ("LPW.12", 8), ("LPW.1_LV", 17), ("LPW.1_MV", 22), ("LPW.2a_LV", 13), ("LPW.2a_MV", 26), ("LPW.2b_LV", 9), ("LPW.2b_MV", 18), ("LPW.3", 49), ("LPW.5", 4)]
)]
#[case::efuse(
    "efuse", "efuse.gds.gz", "10_11_EFUSE",
    &[("EF.01", 38), ("EF.02", 179), ("EF.03", 159), ("EF.04a", 41), ("EF.04b", 75), ("EF.04c", 23), ("EF.04d", 16), ("EF.05", 38), ("EF.06", 367), ("EF.07", 37), ("EF.08", 173), ("EF.09", 30), ("EF.10", 38), ("EF.11", 5), ("EF.12", 21), ("EF.13", 5), ("EF.14", 6), ("EF.15", 12), ("EF.16a", 74), ("EF.16b", 52), ("EF.17", 6), ("EF.18", 46), ("EF.19", 21), ("EF.20", 55), ("EF.21", 122), ("EF.22a", 62), ("EF.22b", 33)]
)]
#[case::hres(
    "hres", "hres.gds.gz", "10_3_HRES",
    &[("HRES.1", 7), ("HRES.10", 24), ("HRES.12a", 37), ("HRES.12b", 1), ("HRES.2", 60), ("HRES.3", 8), ("HRES.4", 38), ("HRES.5", 7), ("HRES.6", 7), ("HRES.7", 23), ("HRES.8", 27), ("HRES.9", 5)]
)]
#[case::dualgate(
    "dualgate", "dualgate.gds.gz", "7_6_Dualgate",
    &[("DV.1", 4), ("DV.2", 3), ("DV.3", 2), ("DV.5", 13), ("DV.6", 4), ("DV.7", 1), ("DV.8", 6), ("DV.9", 1)]
)]
#[case::sram_3p3(
    "sram_3p3", "sram_3p3.gds.gz", "sram_3p3",
    &[("S.CO.3_LV", 6), ("S.CO.4_LV", 6), ("S.CO.6_ii_LV", 8), ("S.DF.16_LV", 6), ("S.DF.4c_LV", 6), ("S.M1.1_LV", 31)]
)]
#[case::drc_bjt(
    "drc_bjt", "drc_bjt.gds.gz", "DRC_BJT",
    &[("BJT.1", 1), ("BJT.2", 2), ("BJT.3", 3)]
)]
#[case::nwell(
    "nwell", "nwell.gds.gz", "7_4_NWELL",
    &[("NW.1a_LV", 81), ("NW.1a_MV", 126), ("NW.1b_LV", 2), ("NW.1b_MV", 4), ("NW.2a_LV", 9), ("NW.2a_MV", 18), ("NW.2b_LV", 13), ("NW.2b_MV", 26), ("NW.3", 14), ("NW.4", 15), ("NW.5_LV", 16), ("NW.5_MV", 23), ("NW.6", 13)]
)]
#[case::poly2(
    "poly2", "poly2.gds.gz", "7_7_Poly2",
    &[("PL.11", 6), ("PL.12", 5), ("PL.1_LV", 23), ("PL.1_MV", 22), ("PL.1a_LV", 9), ("PL.1a_MV", 9), ("PL.2_LV", 98), ("PL.2_MV", 306), ("PL.3a", 49), ("PL.4_LV", 4), ("PL.4_MV", 4), ("PL.5a_LV", 6), ("PL.5a_MV", 6), ("PL.5b_LV", 6), ("PL.5b_MV", 6), ("PL.6", 718), ("PL.7_LV", 40), ("PL.7_MV", 84), ("PL.9", 11)]
)]
#[case::dnwell(
    "dnwell", "dnwell.gds.gz", "7_2_DNWELL",
    &[("DN.1", 464), ("DN.2a", 13), ("DN.2b", 33), ("DN.3", 150)]
)]
#[case::pres(
    "pres", "pres.gds.gz", "10_1_PRES",
    &[("PRES.1", 46), ("PRES.2", 6), ("PRES.3", 9), ("PRES.4", 6), ("PRES.5", 7), ("PRES.6", 39), ("PRES.7", 8), ("PRES.9a", 5), ("PRES.9b", 1)]
)]
#[case::comp(
    "comp", "comp.gds.gz", "7_5_DF",
    &[("DF.10", 2), ("DF.11", 72), ("DF.12", 74), ("DF.13_LV", 45), ("DF.13_MV", 45), ("DF.14_LV", 43), ("DF.14_MV", 43), ("DF.16_LV", 6), ("DF.16_MV", 6), ("DF.17_LV", 6), ("DF.17_MV", 6), ("DF.18", 6), ("DF.19_LV", 6), ("DF.19_MV", 6), ("DF.1a_LV", 100), ("DF.1a_MV", 165), ("DF.1c", 9), ("DF.2a_LV", 4), ("DF.2a_MV", 4), ("DF.2b", 2), ("DF.3a_LV", 29), ("DF.3a_MV", 28), ("DF.3b", 16), ("DF.3c_LV", 7), ("DF.3c_MV", 11), ("DF.4a_LV", 12), ("DF.4a_MV", 6), ("DF.4b_LV", 6), ("DF.4b_MV", 6), ("DF.4c_LV", 8), ("DF.4c_MV", 6), ("DF.4d_LV", 6), ("DF.4d_MV", 6), ("DF.4e_LV", 6), ("DF.4e_MV", 6), ("DF.5_LV", 6), ("DF.5_MV", 6), ("DF.6_LV", 3), ("DF.6_MV", 3), ("DF.7_LV", 6), ("DF.7_MV", 6), ("DF.8_LV", 6), ("DF.8_MV", 6), ("DF.9", 215)]
)]
#[case::sab(
    "sab", "sab.gds.gz", "7_10_SB",
    &[("SB.1", 17), ("SB.10", 47), ("SB.11", 1), ("SB.12", 1), ("SB.13", 132), ("SB.14a", 6), ("SB.14b", 6), ("SB.15a", 13), ("SB.15b", 5), ("SB.16", 6), ("SB.2", 9), ("SB.3", 9), ("SB.4", 6), ("SB.5a", 9), ("SB.5b", 7), ("SB.6", 14), ("SB.7", 16), ("SB.8", 3), ("SB.9", 28)]
)]
#[case::ymtp_mk(
    "ymtp_mk", "ymtp_mk.gds.gz", "10_13_YMTP",
    &[("Y.DF.16_LV", 6), ("Y.DF.16_MV", 6), ("Y.DF.6_MV", 14), ("Y.NW.2b_LV", 14), ("Y.NW.2b_MV", 28), ("Y.PL.1_LV", 99), ("Y.PL.1_MV", 119), ("Y.PL.2_LV", 68), ("Y.PL.2_MV", 164), ("Y.PL.4_MV", 6), ("Y.PL.5a_LV", 8), ("Y.PL.5a_MV", 6), ("Y.PL.5b_LV", 8), ("Y.PL.5b_MV", 6)]
)]
#[case::contact(
    "contact", "contact.gds.gz", "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump",
    &[("CO.1", 104), ("CO.10", 2), ("CO.11", 145), ("CO.2a", 8), ("CO.2b", 6), ("CO.3", 13), ("CO.4", 8), ("CO.5a", 2), ("CO.5b", 4), ("CO.6", 98), ("CO.6a", 25), ("CO.6b", 30), ("CO.7", 2), ("CO.8", 2), ("CO.9", 6)]
)]
#[case::sram_5p0(
    "sram_5p0", "sram_5p0.gds.gz", "sram_5p0",
    &[("S.CO.4_MV", 8), ("S.DF.16_MV", 6), ("S.DF.4c_MV", 8), ("S.DF.6_MV", 15), ("S.DF.7_MV", 6), ("S.DF.8_MV", 8), ("S.PL.5a_MV", 6), ("S.PL.5b_MV", 4)]
)]
#[case::lres(
    "lres", "lres.gds.gz", "10_2_LRES",
    &[("LRES.1", 45), ("LRES.2", 6), ("LRES.3", 9), ("LRES.4", 6), ("LRES.5", 7), ("LRES.6", 39), ("LRES.7", 8), ("LRES.9a", 9), ("LRES.9b", 1)]
)]
#[case::mim_b(
    "mim_b", "mim_b.gds.gz", "10_4_2_MIM_OptionB",
    &[("MIMTM.1", 4), ("MIMTM.10", 3), ("MIMTM.11", 2), ("MIMTM.2", 9), ("MIMTM.3", 61), ("MIMTM.4", 11), ("MIMTM.5", 8), ("MIMTM.6", 6), ("MIMTM.7", 2490), ("MIMTM.8a", 60), ("MIMTM.8b", 1), ("MIMTM.9", 6)]
)]
#[case::otp_mk(
    "otp_mk", "otp_mk.gds.gz", "10_10_OTP",
    &[("O.CO.7", 2), ("O.DF.3a", 9), ("O.DF.6", 16), ("O.DF.9", 4), ("O.PL.2", 70), ("O.PL.3a", 14), ("O.PL.4", 9), ("O.PL.ORT", 367), ("O.SB.11", 1), ("O.SB.13_LV", 53), ("O.SB.13_MV", 1), ("O.SB.2", 7), ("O.SB.3", 6), ("O.SB.4", 2), ("O.SB.5b_LV", 6), ("O.SB.9", 1)]
)]
#[case::mcell(
    "mcell", "mcell.gds.gz", "7_17_Mcell",
    &[("MC.1", 36), ("MC.2", 24), ("MC.3", 13), ("MC.4", 6)]
)]
#[case::nplus(
    "nplus", "nplus.gds.gz", "7_10_Nplus",
    &[("NP.1", 392), ("NP.10", 3), ("NP.11", 10), ("NP.12", 1), ("NP.2", 86), ("NP.3a", 9), ("NP.3bi", 10), ("NP.3bii", 4), ("NP.3ci", 15), ("NP.3cii", 4), ("NP.3d", 2), ("NP.3e", 2), ("NP.4a", 4), ("NP.4b", 1), ("NP.5a", 6), ("NP.5b", 66), ("NP.5ci", 7), ("NP.5cii", 3), ("NP.5di", 7), ("NP.5dii", 9), ("NP.6", 72), ("NP.7", 6), ("NP.8a", 121), ("NP.8b", 2), ("NP.9", 4)]
)]
#[case::metaltop(
    "metaltop", "metaltop.gds.gz", "metaltop",
    &[("MT.1", 388), ("MT.2a", 16), ("MT.2b", 11), ("MT.4", 85)]
)]
#[case::pplus(
    "pplus", "pplus.gds.gz", "7_11_Pplus",
    &[("PP.1", 392), ("PP.10", 3), ("PP.11", 8), ("PP.12", 1), ("PP.2", 86), ("PP.3a", 21), ("PP.3bi", 4), ("PP.3bii", 11), ("PP.3ci", 4), ("PP.3cii", 10), ("PP.3d", 2), ("PP.3e", 2), ("PP.4a", 4), ("PP.4b", 1), ("PP.5a", 6), ("PP.5b", 71), ("PP.5ci", 6), ("PP.5cii", 9), ("PP.5di", 8), ("PP.5dii", 33), ("PP.6", 72), ("PP.7", 6), ("PP.8a", 121), ("PP.8b", 2), ("PP.9", 4)]
)]
#[case::metal1(
    "metal", "metal1.gds.gz", "metal1",
    &[("M1.1", 336), ("M1.2a", 16), ("M1.2b", 11), ("M1.3", 85)]
)]
#[case::metal2(
    "metal", "metal2.gds.gz", "metal2",
    &[("M2.1", 340), ("M2.2a", 16), ("M2.2b", 11), ("M2.3", 85)]
)]
#[case::metal3(
    "metal", "metal3.gds.gz", "metal3",
    &[("M3.1", 340), ("M3.2a", 16), ("M3.2b", 11), ("M3.3", 85)]
)]
#[case::metal4(
    "metal", "metal4.gds.gz", "metal4",
    &[("M4.1", 340), ("M4.2a", 16), ("M4.2b", 11), ("M4.3", 85)]
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
    &[("V1.1", 5), ("V1.2a", 3), ("V1.2b", 6), ("V1.3a", 3), ("V1.3c", 2), ("V1.3d", 15), ("V1.4a", 6), ("V1.4b", 2), ("V1.4c", 15)]
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
    // source/drain overhang for Y.DF.6_MV, the poly end cap for Y.PL.4_MV. These two were
    // the fixtures that found `polys_interact`'s missing edge-crossing test.
    ("Y.DF.6_MV", Some(&[("Y.DF.6_MV", 1)])),
    ("Y.PL.4_MV", Some(&[("Y.PL.4_MV", 1)])),
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
/// * `PRES.7` moves both contact heads, since the cell places them symmetrically.
fn pres_bad_expect(id: &str) -> usize {
    match id {
        "PRES.1" | "PRES.7" => 2,
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
/// * `PP.5b` and the `PP.5c`/`PP.5d` pairs measure an extension that is short on every
///   side of the COMP at once, so a square reports four — three for `PP.5dii`, whose
///   fourth side faces the N-well that puts it in the near bucket.
/// * `PP.3d` and `PP.3e` are the same set of geometry — COMP and Nplus and Pplus
///   together — so neither can be drawn without the other.
fn pplus_bad_expect(id: &str) -> Vec<(String, usize)> {
    let one = |i: &str| (i.to_string(), 1);
    match id {
        "PP.1" => vec![("PP.1".to_string(), 2)],
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
// the only user anywhere of `min_width` on an edge layer, and `O.SB.11` one of two users
// of `min_overlap`, which had no drawn pattern at all.

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
