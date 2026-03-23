// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SG13CMOS5L tests.
//!
//! CMOS5L is SG13G2 without the HBT module and with a reduced metal stack
//! (M1-M4 + TopVia1 + TopMetal1); every shared rule is value-identical.  Instead of
//! duplicating the SG13G2 fixture tree, the parity test below runs each shared deck
//! on the *SG13G2* fixtures under both PDKs and requires identical results — this
//! covers the whole shared rule set, including the CMOS5L cont/nwell forks, whose
//! DigiBnd splits must reduce to the SG13G2 behaviour on DigiBnd-free layouts.
//! Only the genuinely different rules get their own fixtures (see
//! gen/ihp_sg13cmos5l).

use gdscheck::run_drc;
use rstest::rstest;

const G2: &str = "ihp-sg13g2";
const C5L: &str = "ihp-sg13cmos5l";
const G2_DATA: &str = "tests/data/ihp-sg13g2";
const C5L_DATA: &str = "tests/data/ihp-sg13cmos5l";

fn drc(pdk: &str, path: &str, deck: &str, ignore: &[&str]) -> Vec<String> {
    let violations = run_drc(path, pdk, &[deck], None, "TOP", true).expect("DRC run failed");
    let mut ids: Vec<String> = violations
        .into_iter()
        .filter(|v| !ignore.contains(&v.rule_id.as_str()))
        .map(|v| v.rule_id)
        .collect();
    ids.sort();
    ids
}

/// Decks shared with SG13G2 (same file, or the cont/nwell forks whose digital
/// splits are inert without DigiBnd).  Excluded because they genuinely differ:
/// forbidden, pad, passiv, topvia1, antenna.
const SHARED_DECKS: &[&str] = &[
    "offgrid", "pin", "lbe", "activ", "tgo", "gatpoly", "extblock", "cont",
    "contbar", "salblock", "nsdblock", "psd", "resistor", "nwell", "pwellblock",
    "metal1", "metal2", "metal3", "metal4", "via1", "via2", "via3", "topmetal1",
    "sealring", "slit", "lu",
];

/// Run every SG13G2 fixture of every shared deck under both PDKs and require
/// identical violation lists.
#[test]
fn parity_with_sg13g2_on_shared_decks() {
    let mut checked = 0;
    for deck in SHARED_DECKS {
        let dir = format!("{G2_DATA}/{deck}");
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("fixture dir {dir}: {e}"))
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().ends_with(".gds.gz"))
            .collect();
        entries.sort();
        for path in entries {
            let path = path.to_string_lossy();
            let g2 = drc(G2, &path, deck, &[]);
            let c5l = drc(C5L, &path, deck, &[]);
            assert_eq!(g2, c5l, "deck '{deck}', fixture '{path}'");
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} fixtures checked — walker broken?");
}

// --- CMOS5L-specific rules ---

#[rstest]
// TopVia1 lands on Metal4: 0.10 enclosure, one violation per undershot side.
// TV1.a/b/d ignored exactly as in the SG13G2 TV1.c fixture (the pattern grows the
// via by the undershoot, and no TopMetal1 is drawn).
#[case::tv1_c("topvia1/TV1.c.gds.gz", "topvia1", vec!["TV1.c"; 4], vec!["TV1.a", "TV1.b", "TV1.d"])]
// TopMetal1 enclosure of Passiv inside the seal; the identical pattern outside the
// EdgeSeal is exempt.
#[case::pas_c("passiv/Pas.c.gds.gz", "passiv", vec!["Pas.c"; 4], vec![])]
// A dfpad without TopMetal1 under it; the well-formed TM1 pad is clean.
#[case::pad_i("pad/Pad.i.gds.gz", "pad", vec!["Pad.i"], vec![])]
// One shape on each of Metal5 / TRANS / nBuLay / MIM (§3.2 forbidden in CMOS5L).
#[case::forbidden("forbidden.gds.gz", "forbidden", vec!["forbidden"; 4], vec![])]
// Cnt.c relaxes from 0.07 to 0.05 inside a DigiBnd: 0.065 fires only outside,
// 0.045 fires inside as Cnt.c.Digi.
#[case::cnt_digi("cont/Cnt.c.digi.gds.gz", "cont", vec!["Cnt.c", "Cnt.c.Digi"], vec![])]
// NW.f1 relaxes from 0.62 to 0.24 inside a DigiBnd: gap 0.30 fires only outside,
// 0.20 fires inside as NW.f1.dig.
#[case::nw_f1_digi("nwell/NW.f1.digi.gds.gz", "nwell", vec!["NW.f1", "NW.f1.dig"], vec![])]
fn test_cmos5l(
    #[case] gds: &str,
    #[case] deck: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    let path = format!("{C5L_DATA}/{gds}");
    assert_eq!(drc(C5L, &path, deck, &ignore), expected);
}
