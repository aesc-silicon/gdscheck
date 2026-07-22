// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later
use gdscheck::pdk::PdkConfig;
use std::collections::HashSet;

fn ids(pdk: &PdkConfig, suite: &str) -> Vec<String> {
    let mut v: Vec<String> = pdk.load_suite(suite).unwrap().iter().map(|r| format!("{}::{}", r.id, r.check)).collect();
    v.sort();
    v
}
fn deck_ids(pdk: &PdkConfig, deck: &str) -> Vec<String> {
    pdk.load_deck(deck).unwrap().iter().map(|r| format!("{}::{}", r.id, r.check)).collect()
}

#[test]
fn core_equals_main_minus_antenna_minus_density() {
    let mut antenna_by_proc: Vec<HashSet<String>> = Vec::new();
    for proc in ["ihp-sg13g2", "ihp-sg13cmos5l"] {
        let pdk = PdkConfig::for_process(proc).unwrap();
        let main: HashSet<String> = ids(&pdk, "main").into_iter().collect();
        let core: HashSet<String> = ids(&pdk, "core").into_iter().collect();
        let density: HashSet<String> = ids(&pdk, "density").into_iter().collect();
        let antenna: HashSet<String> = deck_ids(&pdk, "antenna").into_iter().collect();

        // core must be exactly main minus (antenna deck rules) minus (density suite rules)
        let expected: HashSet<String> = main.difference(&antenna).cloned().collect::<HashSet<_>>()
            .difference(&density).cloned().collect();
        assert_eq!(core, expected, "[{proc}] core != main - antenna - density\n only_in_core: {:?}\n missing_from_core: {:?}",
            core.difference(&expected).collect::<Vec<_>>(),
            expected.difference(&core).collect::<Vec<_>>());
        // sanity: density and antenna were actually removed
        assert!(core.is_disjoint(&density), "[{proc}] core still has density rules");
        assert!(core.is_disjoint(&antenna), "[{proc}] core still has antenna rules");
        assert!(!core.is_empty());

        // the `antenna` suite is exactly the antenna deck
        let antenna_suite: HashSet<String> = ids(&pdk, "antenna").into_iter().collect();
        assert_eq!(antenna_suite, antenna, "[{proc}] antenna suite != antenna deck");

        println!("[{proc}] main={} core={} density={} antenna={}", main.len(), core.len(), density.len(), antenna.len());
        antenna_by_proc.push(antenna);
    }
    // Antenna rule ids are identical between SG13G2 and SG13CMOS5L.
    assert_eq!(antenna_by_proc[0], antenna_by_proc[1], "antenna ids differ between processes");
}
