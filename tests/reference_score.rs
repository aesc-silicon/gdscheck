// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! How much of each foundry deck this port reproduces, held to a floor.
//!
//! The self-contained patterns under `tests/data/*/generated` are what say a rule is
//! *correct*: each is drawn here, its legal half must be silent and its illegal half must
//! fire exactly so.  This file is the other question - how much of a real foundry deck
//! the port covers - and it is asked by running the deck over the case the PDK ships and
//! scoring the result against the report that ships beside it.
//!
//! The numbers below are a floor on violations found and a ceiling on violations
//! invented, not an expectation.  That is deliberate: an engine change that finds more,
//! or invents less, passes without touching this file, and only a regression fails.  The
//! suite used to pin exact per-rule marker counts instead, which cannot tell those apart -
//! every count moves whenever the engine improves, so a diff of them says nothing.

#[path = "common/lyrdb.rs"]
mod lyrdb;

use gdscheck::run_drc;
use rstest::rstest;

/// Run the deck and return the report *as written*, so what is scored is exactly the
/// file a user gets rather than a re-rendering of it.
fn report(pdk: &str, data: &str, deck: &str, gds: &str, topcell: &str) -> String {
    let violations = run_drc(
        &format!("{data}/static/{gds}"),
        pdk,
        &[deck],
        None,
        topcell,
        true,
    )
    .expect("DRC run failed");
    let dir = std::env::temp_dir().join("gdscheck-reference-score");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join(format!("{deck}-{gds}.lyrdb"));
    let path = path.to_str().expect("utf-8 path");
    gdscheck::report::write_lyrdb(path, topcell, &violations).expect("report written");
    std::fs::read_to_string(path).expect("report read back")
}

fn check(pdk: &str, data: &str, deck: &str, gds: &str, topcell: &str, floor: usize, ceil: usize) {
    let ours = report(pdk, data, deck, gds, topcell);
    let refname = gds.trim_end_matches(".gds.gz");
    let gold = lyrdb::read_gz(&format!("{data}/reference/{refname}.lyrdb.gz"));
    let s = lyrdb::score(&gold, &ours);
    let detail = || {
        let mut d = String::new();
        for (rule, m, r, e) in &s.by_rule {
            if m < r || *e > 0 {
                d.push_str(&format!("\n    {rule:<14} matched {m}/{r}  extra {e}"));
            }
        }
        d
    };
    assert!(
        s.matched >= floor,
        "{deck} on {gds}: matched {} of {} reference violations, floor is {floor}{}",
        s.matched,
        s.reference,
        detail()
    );
    assert!(
        s.extra <= ceil,
        "{deck} on {gds}: {} false positives, ceiling is {ceil}{}",
        s.extra,
        detail()
    );
}

const D: &str = "tests/data/gf180mcuD";

#[rstest]
#[case("antenna", "antenna-1.gds.gz", "8_0_ANT", 10, 4)]
#[case("dummy_exclude", "dummy_exclude.gds.gz", "10_8_DE", 4, 1)]
#[case("ldpmos", "ldpmos.gds.gz", "10_12_2_MDP", 246, 2)]
#[case("nat", "nat.gds.gz", "10_5_NAT", 36, 5)]
#[case("ldnmos", "ldnmos.gds.gz", "10_12_1_MDN", 735, 13)]
#[case("lvpwell", "lvpwell.gds.gz", "7_3_LVPWELL", 165, 0)]
#[case("efuse", "efuse.gds.gz", "10_11_EFUSE", 809, 1)]
#[case("hres", "hres.gds.gz", "10_3_HRES", 149, 2)]
#[case("dualgate", "dualgate.gds.gz", "7_6_Dualgate", 16, 2)]
#[case("sram_3p3", "sram_3p3.gds.gz", "sram_3p3", 27, 6)]
#[case("drc_bjt", "drc_bjt.gds.gz", "DRC_BJT", 6, 0)]
#[case("nwell", "nwell.gds.gz", "7_4_NWELL", 195, 0)]
#[case("poly2", "poly2.gds.gz", "7_7_Poly2", 260, 11)]
#[case("dnwell", "dnwell.gds.gz", "7_2_DNWELL", 246, 0)]
#[case("pres", "pres.gds.gz", "10_1_PRES", 52, 1)]
#[case("comp", "comp.gds.gz", "7_5_DF", 734, 14)]
#[case("sab", "sab.gds.gz", "7_10_SB", 236, 2)]
#[case("ymtp_mk", "ymtp_mk.gds.gz", "10_13_YMTP", 145, 2)]
#[case(
    "contact",
    "contact.gds.gz",
    "7_12_CO_Rev13_1P6M_11kA_MIMA_Gold_Bump",
    166,
    8
)]
#[case("sram_5p0", "sram_5p0.gds.gz", "sram_5p0", 41, 0)]
#[case("lres", "lres.gds.gz", "10_2_LRES", 56, 1)]
#[case("mim_b", "mim_b.gds.gz", "10_4_2_MIM_OptionB", 157, 7)]
#[case("otp_mk", "otp_mk.gds.gz", "10_10_OTP", 121, 0)]
#[case("mcell", "mcell.gds.gz", "7_17_Mcell", 22, 2)]
#[case("nplus", "nplus.gds.gz", "7_10_Nplus", 164, 4)]
#[case("metaltop", "metaltop.gds.gz", "metaltop", 72, 2)]
#[case("pplus", "pplus.gds.gz", "7_11_Pplus", 178, 4)]
#[case("metal", "metal1.gds.gz", "metal1", 74, 2)]
#[case("metal", "metal2.gds.gz", "metal2", 74, 2)]
#[case("metal", "metal3.gds.gz", "metal3", 74, 2)]
#[case("metal", "metal4.gds.gz", "metal4", 74, 2)]
#[case("esd", "esd.gds.gz", "7_11_ESD", 26, 0)]
#[case("lvs_bjt", "lvs_bjt.gds.gz", "LVS_BJT", 2, 0)]
#[case("via", "via1.gds.gz", "7_14_VIA", 11, 9)]
fn deck_reproduces_the_foundry_case(
    #[case] deck: &str,
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] floor: usize,
    #[case] ceil: usize,
) {
    check("gf180mcuD", D, deck, gds, topcell, floor, ceil);
}
