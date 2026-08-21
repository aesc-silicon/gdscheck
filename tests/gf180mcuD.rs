// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GF180MCU variant D, against the test cases the PDK ships for its own DRC deck
//! (`libs.tech/klayout/tech/drc/testing/unit/`), vendored under
//! `tests/data/gf180mcuD/static/`.
//!
//! These differ in kind from the IHP fixtures next door. Those are generated one rule at
//! a time, so a case names the handful of violations it should produce and an exact list
//! is readable. A foundry case is one dense layout exercising a whole deck at once and
//! yields hundreds of markers, so the assertion here is the **per-rule count**: which
//! rules fire, and how often.
//!
//! What that does and does not buy:
//!
//! * It pins current behaviour. Any change in a rule, a derived layer or the geometry
//!   engine that moves a count shows up here, on real foundry geometry rather than on a
//!   pattern written to match our own implementation.
//! * It is **not** a correctness proof. The numbers are what gdscheck produces today,
//!   not what the foundry's engine produces — the two disagree on markers-per-violation
//!   by construction (an edge-pair spanning a gap versus one edge per violating pair),
//!   so they were never going to match. Where a port stands against the reference
//!   `.lyrdb` is recorded in the deck file itself and re-checked with
//!   `tools/compare-lyrdb.py`; that is the correctness question, and this is the
//!   regression gate.
//!
//! A count changing is therefore a prompt to look, not a failure on its own: confirm the
//! new number against the reference, then update it here.
//!
//! Only decks that have actually been ported appear below. The remaining fixtures are
//! vendored and waiting — adding a case for a deck that is still `rules: []` would
//! assert "no violations" and read as coverage while proving nothing.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "gf180mcuD";
const DATA: &str = "tests/data/gf180mcuD/static";

/// Violation count per rule id, sorted by id.
fn counts(deck: &str, gds: &str, topcell: &str) -> Vec<(String, usize)> {
    let path = format!("{DATA}/{gds}");
    let violations = run_drc(&path, PDK, &[deck], None, topcell, true).expect("DRC run failed");
    let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
    for v in violations {
        *by_rule.entry(v.rule_id).or_default() += 1;
    }
    by_rule.into_iter().collect()
}

fn assert_counts(deck: &str, gds: &str, topcell: &str, expected: &[(&str, usize)]) {
    let want: Vec<(String, usize)> = expected
        .iter()
        .map(|(r, n)| ((*r).to_string(), *n))
        .collect();
    assert_eq!(counts(deck, gds, topcell), want);
}

// --- N-well ---
//
// Every rule in the deck fires on this case, which is the first thing worth knowing: a
// rule that silently matches nothing is the failure mode a skeleton invites.

#[rstest]
#[case::nwell(
    "nwell",
    "nwell.gds.gz",
    "7_4_NWELL",
    &[
        ("NW.1a_LV", 79),
        ("NW.1a_MV", 122),
        ("NW.1b_LV", 2),
        ("NW.1b_MV", 4),
        ("NW.2a_LV", 4),
        ("NW.2a_MV", 10),
        ("NW.2b_LV", 13),
        ("NW.2b_MV", 24),
        ("NW.3", 13),
        ("NW.4", 15),
        ("NW.5_LV", 4),
        ("NW.5_MV", 6),
        ("NW.6", 13),
    ]
)]
#[case::comp(
    "comp",
    "comp.gds.gz",
    "7_5_DF",
    &[
        ("DF.10", 2),
        ("DF.12", 77),
        ("DF.13_LV", 45),
        ("DF.13_MV", 45),
        ("DF.14_LV", 43),
        ("DF.14_MV", 44),
        ("DF.16_LV", 4),
        ("DF.16_MV", 4),
        ("DF.17_LV", 4),
        ("DF.17_MV", 4),
        ("DF.18", 4),
        ("DF.19_LV", 3),
        ("DF.19_MV", 3),
        ("DF.1a_LV", 97),
        ("DF.1a_MV", 167),
        ("DF.1c", 9),
        ("DF.2b", 2),
        ("DF.3a_LV", 23),
        ("DF.3a_MV", 23),
        ("DF.3b", 16),
        ("DF.3c_LV", 5),
        ("DF.4a_LV", 8),
        ("DF.4a_MV", 4),
        ("DF.4b_LV", 6),
        ("DF.4b_MV", 6),
        ("DF.4c_LV", 8),
        ("DF.4c_MV", 6),
        ("DF.4d_LV", 6),
        ("DF.4d_MV", 6),
        ("DF.4e_LV", 6),
        ("DF.4e_MV", 6),
        ("DF.5_LV", 6),
        ("DF.5_MV", 6),
        ("DF.6_LV", 3),
        ("DF.6_MV", 3),
        ("DF.7_LV", 4),
        ("DF.7_MV", 4),
        ("DF.8_LV", 6),
        ("DF.8_MV", 6),
        ("DF.9", 225),
    ]
)]
#[case::dnwell(
    "dnwell", "dnwell.gds.gz", "7_2_DNWELL",
    &[("DN.1", 462), ("DN.2a", 13), ("DN.2b", 41)]
)]
#[case::mcell(
    "mcell", "mcell.gds.gz", "7_17_Mcell",
    &[("MC.1", 35), ("MC.2", 21), ("MC.3", 15), ("MC.4", 6)]
)]
#[case::drc_bjt(
    "drc_bjt", "drc_bjt.gds.gz", "DRC_BJT",
    &[("BJT.1", 1), ("BJT.2", 2), ("BJT.3", 3)]
)]
#[case::lvs_bjt("lvs_bjt", "lvs_bjt.gds.gz", "LVS_BJT", &[("LVS_BJT.1", 2)])]
#[case::dummy_exclude(
    "dummy_exclude", "dummy_exclude.gds.gz", "10_8_DE",
    &[("DE.2", 3), ("DE.3", 1), ("DE.4", 2)]
)]
#[case::dualgate(
    "dualgate", "dualgate.gds.gz", "7_6_Dualgate",
    &[
        ("DV.1", 3), ("DV.2", 3), ("DV.3", 1), ("DV.5", 14),
        ("DV.6", 3), ("DV.7", 1), ("DV.8", 3), ("DV.9", 1),
    ]
)]
// The metal deck carries one rule set per level, and each fixture exercises one level,
// so only the M1.* rules fire here. That is why metal is absent from
// `every_rule_in_the_deck_fires` below - the other levels are silent by design.
#[case::metal1(
    "metal", "metal1.gds.gz", "metal1",
    &[("M1.1", 335), ("M1.2a", 14), ("M1.3", 86)]
)]
#[case::nat(
    "nat", "nat.gds.gz", "10_5_NAT",
    &[
        ("NAT.1", 2), ("NAT.10", 1), ("NAT.11", 1), ("NAT.12", 2),
        ("NAT.2", 4), ("NAT.3", 3), ("NAT.4", 36), ("NAT.5", 8),
        ("NAT.7", 2), ("NAT.8", 5),
    ]
)]
#[case::metaltop(
    "metaltop", "metaltop.gds.gz", "metaltop",
    &[("MT.1", 387), ("MT.2a", 14), ("MT.4", 86)]
)]
#[case::lvpwell(
    "lvpwell", "lvpwell.gds.gz", "7_3_LVPWELL",
    &[
        ("LPW.11", 13), ("LPW.12", 8), ("LPW.1_LV", 9), ("LPW.1_MV", 18),
        ("LPW.2a_LV", 10), ("LPW.2a_MV", 20), ("LPW.2b_LV", 5), ("LPW.2b_MV", 10),
        ("LPW.3", 49), ("LPW.5", 4),
    ]
)]
#[case::esd(
    "esd", "esd.gds.gz", "7_11_ESD",
    &[
        ("ESD.1", 2), ("ESD.10", 2), ("ESD.2", 2), ("ESD.3a", 1), ("ESD.3b", 7),
        ("ESD.5a", 2), ("ESD.5b", 2), ("ESD.7", 7), ("ESD.8", 3), ("ESD.9", 1),
        ("ESD.pl", 2),
    ]
)]
#[case::contact(
    "contact", "contact.gds.gz", "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump",
    &[
        ("CO.1", 104), ("CO.10", 2), ("CO.11", 145), ("CO.2a", 8), ("CO.3", 7),
        ("CO.4", 5), ("CO.5a", 2), ("CO.5b", 4), ("CO.6", 98), ("CO.7", 2), ("CO.8", 2),
    ]
)]
#[case::sram_3p3(
    "sram_3p3", "sram_3p3.gds.gz", "sram_3p3",
    &[
        ("S.CO.3_LV", 3), ("S.CO.4_LV", 3), ("S.DF.16_LV", 4),
        ("S.DF.4c_LV", 3), ("S.M1.1_LV", 31),
    ]
)]
// Like metal, the via deck carries one rule set per level and each fixture exercises
// one, so only the V1.* rules fire here.
#[case::via1(
    "via", "via1.gds.gz", "7_14_VIA",
    &[("V1.1", 4), ("V1.2a", 2), ("V1.3a", 3), ("V1.4a", 6)]
)]
fn static_fixture(
    #[case] deck: &str,
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] expected: &[(&str, usize)],
) {
    assert_counts(deck, gds, topcell, expected);
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
