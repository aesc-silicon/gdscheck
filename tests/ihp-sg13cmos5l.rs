// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SG13CMOS5L tests.
//!
//! CMOS5L is SG13G2 without the HBT module and with a reduced metal stack
//! (M1-M4 + TopVia1 + TopMetal1); every shared rule is value-identical.  Instead of
//! duplicating the SG13G2 fixture tree, the parity tests below compare each shared
//! deck as loaded under both PDKs - the same rules over the same layers - and run one
//! SG13G2 fixture per deck under both, which covers the whole shared rule set.
//! Only the genuinely different rules get their own fixtures (see
//! gen/ihp_sg13cmos5l).

use gdscheck::pdk::{PdkConfig, RuleDefinition};
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

/// The decks CMOS5L reads from the SG13G2 tree, by name.
fn shared_decks() -> Vec<String> {
    let c5l = PdkConfig::for_process(C5L).unwrap();
    let shared: Vec<String> = c5l
        .decks
        .iter()
        .filter(|d| d.path.starts_with("../ihp-sg13g2/"))
        .map(|d| d.name.clone())
        .collect();
    assert!(shared.len() > 20, "shared decks: {shared:?}");
    shared
}

/// A shared deck is the same rules over the same layers under both PDKs, and the engine
/// reads nothing else, so it answers the same on any layout.  Compared as loaded: every
/// rule of every shared deck (its layers with their GDS numbers, its value, its params
/// with the layer params resolved), every SG13G2 virtual layer's definition and assigned
/// number and the numbers of its sources (CMOS5L may add its own, and a same-name child
/// entry would replace the base one, which is what this would catch), the waivers, and
/// the connect graph: SG13G2's through Metal4, CMOS5L's own stack top, and the well step
/// as SG13G2 has it, so every net a shared rule can read is the same net.  Structural,
/// so it costs nothing; the DRC walk it replaces ran 700 fixtures twice and took 40 s
/// of the 7 µm tile suite to say what this says.
#[test]
fn shared_decks_are_sg13g2s_as_loaded() {
    let g2 = PdkConfig::for_process(G2).unwrap();
    let c5l = PdkConfig::for_process(C5L).unwrap();
    // One line per rule, its params in key order (they live in a `HashMap`).
    let canon = |rules: Vec<RuleDefinition>| -> String {
        rules
            .iter()
            .map(|r| {
                let mut params: Vec<String> =
                    r.params.iter().map(|(k, v)| format!("{k}={v:?}")).collect();
                params.sort();
                format!(
                    "{} {} {:?} {} [{}] ignore={:?} text={:?}\n",
                    r.id,
                    r.check,
                    r.layers,
                    r.value,
                    params.join(" "),
                    r.ignore,
                    r.text
                )
            })
            .collect()
    };
    for deck in shared_decks() {
        let a = canon(g2.load_deck(&deck).unwrap());
        let b = canon(c5l.load_deck(&deck).unwrap());
        assert_eq!(a, b, "deck '{deck}' loads differently under CMOS5L");
    }
    for vl in &g2.virtual_layers {
        let twin = c5l
            .virtual_layers
            .iter()
            .find(|v| v.name == vl.name)
            .unwrap_or_else(|| panic!("virtual layer '{}' missing in CMOS5L", vl.name));
        assert_eq!(
            format!("{vl:#?}"),
            format!("{twin:#?}"),
            "virtual layer '{}'",
            vl.name
        );
        for name in std::iter::once(&vl.name).chain(&vl.layers) {
            assert_eq!(
                format!("{:?}", g2.layer(name)),
                format!("{:?}", c5l.layer(name)),
                "layer '{name}' resolves differently"
            );
        }
    }
    assert_eq!(
        format!("{:#?}", g2.edge_layers),
        format!("{:#?}", c5l.edge_layers)
    );
    assert_eq!(format!("{:#?}", g2.waivers), format!("{:#?}", c5l.waivers));
    // The connect graph: SG13G2's through Via3 ↔ Metal4, then CMOS5L's own stack top
    // (TopVia1 lands on Metal4, not Metal5), then the well step as SG13G2 has it; the
    // nBuLay step has no twin, the layer being forbidden.
    let key = |name: &str| {
        let l = g2
            .layer(name)
            .unwrap_or_else(|| panic!("no layer '{name}'"));
        (l.gds_layer as i16, l.gds_datatype as i16)
    };
    let step =
        |s: &gdscheck::connectivity::ConnectSpec| format!("{:?} -> {:?}", s.connector, s.layers);
    let through_m4 = g2
        .connectivity
        .iter()
        .rposition(|s| s.connector == key("Via3"))
        .expect("SG13G2 connects Via3")
        + 1;
    let g2_steps: Vec<String> = g2.connectivity.iter().map(step).collect();
    let c5l_steps: Vec<String> = c5l.connectivity.iter().map(step).collect();
    assert_eq!(
        c5l_steps[..through_m4],
        g2_steps[..through_m4],
        "the stack through Metal4"
    );
    let top =
        |connector: &str, layer: &str| format!("{:?} -> {:?}", key(connector), vec![key(layer)]);
    assert_eq!(
        c5l_steps[through_m4..],
        [
            top("TopVia1", "Metal4"),
            top("TopVia1", "TopMetal1"),
            g2_steps
                .iter()
                .find(|s| s.starts_with(&format!("{:?}", key("NActivInNWell"))))
                .expect("SG13G2 connects the well")
                .clone(),
        ]
    );
}

/// The same, run: one SG13G2 fixture per shared deck (the first of its directory; the
/// Metal2-4 and Via2-3 hardening layouts sit in `metaln` and `vian`, one file for the
/// layers) under both PDKs, for the path the structural test does not walk - the
/// embedded lookup of a `../ihp-sg13g2/` deck path and the `extends` of the layer
/// tables, end to end.
#[rstest]
fn parity_with_sg13g2_on_shared_decks(
    #[values(
        ("offgrid", "offgrid"),
        ("pin", "pin"),
        ("lbe", "lbe"),
        ("activ", "activ"),
        ("tgo", "tgo"),
        ("gatpoly", "gatpoly"),
        ("extblock", "extblock"),
        ("cont", "cont"),
        ("contbar", "contbar"),
        ("salblock", "salblock"),
        ("nsdblock", "nsdblock"),
        ("psd", "psd"),
        ("resistor", "resistor"),
        ("nwell", "nwell"),
        ("pwellblock", "pwellblock"),
        ("metal1", "metal1"),
        ("metal2", "metal2"),
        ("metal3", "metal3"),
        ("metal4", "metal4"),
        ("metaln", "metal2"),
        ("metaln", "metal3"),
        ("metaln", "metal4"),
        ("via1", "via1"),
        ("via2", "via2"),
        ("via3", "via3"),
        ("vian", "via2"),
        ("vian", "via3"),
        ("topmetal1", "topmetal1"),
        ("sealring", "sealring"),
        ("slit", "slit"),
        ("lu", "lu")
    )]
    pair: (&str, &str),
) {
    let (dir, deck) = pair;
    assert!(
        shared_decks().iter().any(|d| d == deck),
        "'{deck}' is not a shared deck"
    );
    let dir = format!("{G2_DATA}/{dir}");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("fixture dir {dir}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".gds.gz"))
        .collect();
    entries.sort();
    let path = entries
        .first()
        .unwrap_or_else(|| panic!("no fixtures in {dir}"));
    let path = path.to_string_lossy();
    let g2 = drc(G2, &path, deck, &[]);
    let c5l = drc(C5L, &path, deck, &[]);
    assert_eq!(g2, c5l, "deck '{deck}', fixture '{path}'");
}

/// Same-net NW.b1 regression, on the shared SG13G2 fixture (the parity test above only
/// requires both PDKs to agree, which a shared false positive would satisfy).
///
/// Two NWell pairs at a 1.00 µm gap; the second is shorted well-to-well through an
/// N+Activ tie → Cont → Metal1 strap, so exactly one pair is different-net.  Currently
/// fires twice under both PDKs — see the SG13G2 case for the mechanism.
#[test]
fn nw_b1_same_net() {
    let path = format!("{G2_DATA}/nwell/NW.b1.same_net.gds.gz");
    assert_eq!(drc(C5L, &path, "nwell", &[]), vec!["NW.b1"]);
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
// 0.045 fires inside as Cnt.c.dig.
#[case::cnt_digi("cont/Cnt.c.digi.gds.gz", "cont", vec!["Cnt.c", "Cnt.c.dig"], vec![])]
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

// The recommended pad rules.  Those CMOS5L shares with SG13G2 run on SG13G2's fixtures
// (the MIM under the Pad.jR pad is not a device here, only the gate is); Pad.gR and
// Pad.kR read TopVia1 on Metal4 and have their own.
#[rstest]
#[case::pad_ar(G2_DATA, "Pad.aR", vec!["Pad.aR"; 6])]
#[case::pad_br(G2_DATA, "Pad.bR", vec!["Pad.bR"; 2])]
#[case::pad_dr(G2_DATA, "Pad.dR", vec!["Pad.dR"; 2])]
#[case::pad_d1r(G2_DATA, "Pad.d1R", vec!["Pad.d1R", "Pad.d1R", "Pad.d1R", "Pad.dR"])]
#[case::pad_jr(G2_DATA, "Pad.jR", vec!["Pad.jR", "Pad.d1R"])]
#[case::pad_gr(C5L_DATA, "Pad.gR", vec!["Pad.gR"])]
#[case::pad_kr(C5L_DATA, "Pad.kR", vec!["Pad.kR"])]
fn test_pad_recommended(#[case] data: &str, #[case] name: &str, #[case] mut expected: Vec<&str>) {
    expected.sort();
    let path = format!("{data}/recommended/pad/{name}.gds.gz");
    assert_eq!(drc(C5L, &path, "pad_recommended", &[]), expected);
}
