// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GF180MCU variant A tests.
//!
//! Variant A is the same rule set as variant D read against a different stack: three
//! metal levels instead of five, a 30 kA top metal, and MIM option A.  The PDK says so
//! by extending gf180mcuD and re-pointing four alias layers, so almost every deck here
//! is literally D's file.
//!
//! Only the two decks that exist *only* under this variant get fixtures, because they
//! are the only ones whose behaviour is not already covered by the D suite: `mim_a` and
//! `metaltop_30k`.  Both are checked against the foundry's own variant-A test cases.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "gf180mcuA";
const DATA: &str = "tests/data/gf180mcuA/static";

fn counts(deck: &str, gds: &str, topcell: &str) -> Vec<(String, usize)> {
    let violations = run_drc(&format!("{DATA}/{gds}"), PDK, &[deck], None, topcell, true)
        .expect("DRC run failed");
    let mut by_rule: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for v in violations {
        *by_rule.entry(v.rule_id).or_default() += 1;
    }
    by_rule.into_iter().collect()
}

/// The foundry's own variant-A cases, with the counts this port produces on them.
#[rstest]
#[case::mim_a(
    "mim_a", "mim_a.gds.gz", "10_4_1_MIM_OptionA",
    &[
        ("MIM.1", 4), ("MIM.10", 4), ("MIM.2", 9), ("MIM.3", 59), ("MIM.4", 11), ("MIM.5", 8),
        ("MIM.6", 4), ("MIM.7", 2488), ("MIM.8a", 55), ("MIM.8b", 1), ("MIM.9", 831),
    ]
)]
#[case::metaltop_30k(
    "metaltop_30k", "metaltop_30k.gds.gz", "7_16_METAL_30KA_6LM",
    &[
        ("MT30.1a", 162), ("MT30.1b", 2), ("MT30.2", 7), ("MT30.3", 7), ("MT30.4", 7),
        ("MT30.5", 6), ("MT30.6", 4),
    ]
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
    assert_eq!(counts(deck, gds, topcell), want);
}

/// Every rule the deck declares must fire on its case: a rule resolving to an empty
/// derived layer produces no violations and no error, so without this a layer name that
/// did not survive the stack re-pointing looks exactly like a clean design.
#[rstest]
#[case::mim_a("mim_a", "mim_a.gds.gz", "10_4_1_MIM_OptionA")]
#[case::metaltop_30k("metaltop_30k", "metaltop_30k.gds.gz", "7_16_METAL_30KA_6LM")]
fn every_rule_in_the_deck_fires(#[case] deck: &str, #[case] gds: &str, #[case] topcell: &str) {
    let pdk = gdscheck::pdk::PdkConfig::for_process(PDK).expect("PDK loads");
    let declared: std::collections::BTreeSet<String> = pdk
        .load_deck(deck)
        .expect("deck loads")
        .iter()
        .map(|r| r.id.clone())
        .collect();
    let fired: std::collections::BTreeSet<String> = counts(deck, gds, topcell)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let silent: Vec<&String> = declared.difference(&fired).collect();
    assert!(
        silent.is_empty(),
        "deck '{deck}' declares rules that never fire on {gds}: {silent:?}"
    );
}
