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

use gdscheck::pdk::{Param, PdkConfig, RuleDefinition};
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

/// The SG13G2 derived layers CMOS5L defines itself.  `nBuLayDerived`: SG13G2 generates an
/// nBuLay under every wide NWell (section 4.2); CMOS5L's section 4 has no such layer and
/// forbids the drawn one, so the rules reading it see none (issue #46).
const CMOS5L_OWN: &[&str] = &["nBuLayDerived"];

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
/// rule of every shared deck (its drawn layers with their GDS numbers, its derived ones
/// by name, its value, its params with the layer params resolved), every SG13G2 virtual
/// layer's definition and the GDS numbers of its drawn sources (CMOS5L may add its own,
/// and a same-name child entry would replace the base one, which is what this would
/// catch), the waivers, and the connect graph: SG13G2's through Metal4, CMOS5L's own
/// stack top, and the well step as SG13G2 has it, so every net a shared rule can read is
/// the same net.  A derived layer's synthetic number is its place in the list, which a
/// replaced entry moves; its definition is what is compared.  The one replaced entry is
/// [`CMOS5L_OWN`]'s, pinned by its own tests below.  Structural,
/// so it costs nothing; the DRC walk it replaces ran 700 fixtures twice and took 40 s
/// of the 7 µm tile suite to say what this says.
#[test]
fn shared_decks_are_sg13g2s_as_loaded() {
    let g2 = PdkConfig::for_process(G2).unwrap();
    let c5l = PdkConfig::for_process(C5L).unwrap();
    let derived = |name: &str| {
        g2.virtual_layers.iter().any(|v| v.name == name)
            || g2.edge_layers.iter().any(|e| e.name == name)
    };
    // A drawn layer with its GDS numbers, a derived one by name.
    let show = |layers: &[gdscheck::pdk::Layer]| -> String {
        layers
            .iter()
            .map(|l| match derived(&l.name) {
                true => l.name.clone(),
                false => format!("{l:?}"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    // One line per rule, its params in key order (they live in a `HashMap`), a layer
    // param (`key` and `key_dt`, resolved to numbers) naming a derived layer by name.
    let canon = |pdk: &PdkConfig, rules: Vec<RuleDefinition>| -> String {
        let named = |n: f64, dt: f64| {
            pdk.layers()
                .find(|(name, l)| {
                    derived(name) && l.gds_layer as f64 == n && l.gds_datatype as f64 == dt
                })
                .map(|(name, _)| name.to_string())
        };
        rules
            .iter()
            .map(|r| {
                let mut params: Vec<String> = r
                    .params
                    .iter()
                    .filter(|(k, _)| {
                        !k.strip_suffix("_dt")
                            .is_some_and(|base| r.params.contains_key(base))
                    })
                    .map(|(k, v)| {
                        let dt = r.params.get(&format!("{k}_dt"));
                        match (v, dt) {
                            (Param::Num(n), Some(Param::Num(dt))) => match named(*n, *dt) {
                                Some(name) => format!("{k}={name}"),
                                None => format!("{k}={v:?}/{dt}"),
                            },
                            _ => format!("{k}={v:?}"),
                        }
                    })
                    .collect();
                params.sort();
                format!(
                    "{} {} [{}] {} [{}] ignore=[{}] text={:?}\n",
                    r.id,
                    r.check,
                    show(&r.layers),
                    r.value,
                    params.join(" "),
                    show(&r.ignore),
                    r.text
                )
            })
            .collect()
    };
    for deck in shared_decks() {
        let a = canon(&g2, g2.load_deck(&deck).unwrap());
        let b = canon(&c5l, c5l.load_deck(&deck).unwrap());
        assert_eq!(a, b, "deck '{deck}' loads differently under CMOS5L");
    }
    for vl in &g2.virtual_layers {
        if CMOS5L_OWN.contains(&vl.name.as_str()) {
            continue;
        }
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
        for name in vl.layers.iter().filter(|n| !derived(n)) {
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
    // A step with its drawn layers by GDS number and its derived ones by name.
    let label = |pdk: &PdkConfig, (n, dt): (i16, i16)| {
        pdk.layers()
            .find(|(name, l)| {
                derived(name) && l.gds_layer as i16 == n && l.gds_datatype as i16 == dt
            })
            .map_or(format!("({n}, {dt})"), |(name, _)| name.to_string())
    };
    let step = |pdk: &PdkConfig, s: &gdscheck::connectivity::ConnectSpec| {
        let layers: Vec<String> = s.layers.iter().map(|&k| label(pdk, k)).collect();
        format!("{} -> [{}]", label(pdk, s.connector), layers.join(", "))
    };
    let through_m4 = g2
        .connectivity
        .iter()
        .rposition(|s| s.connector == key("Via3"))
        .expect("SG13G2 connects Via3")
        + 1;
    let g2_steps: Vec<String> = g2.connectivity.iter().map(|s| step(&g2, s)).collect();
    let c5l_steps: Vec<String> = c5l.connectivity.iter().map(|s| step(&c5l, s)).collect();
    assert_eq!(
        c5l_steps[..through_m4],
        g2_steps[..through_m4],
        "the stack through Metal4"
    );
    let top = |connector: &str, layer: &str| {
        format!(
            "{} -> [{}]",
            label(&g2, key(connector)),
            label(&g2, key(layer))
        )
    };
    assert_eq!(
        c5l_steps[through_m4..],
        [
            top("TopVia1", "Metal4"),
            top("TopVia1", "TopMetal1"),
            g2_steps
                .iter()
                .find(|s| s.starts_with(&label(&g2, key("NActivInNWell"))))
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
        ("angle", "angle"),
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

/// What SG13G2 reads through its generated nBuLay, CMOS5L does not: AFil.d and GFil.e hold
/// a filler off the NWell alone, and a ContBar in a SalBlock / nSD:block / PWell:block
/// stack is no Schottky diode, so CntB.a and PWB.f1 apply to it.  Each fixture is run
/// under SG13G2 too, whose answer differs, so it keeps exercising the generated nBuLay.
/// CMOS5L's answers are those of IHP's CMOS5L deck (the driver for AFil.d, GFil.e and
/// PWB.f1, the maximal deck for CntB.a, which the driver lacks).
#[rstest]
// A 10 µm NWell: a filler 1.72 inside (SG13G2: 0.72 from its nBuLay), one 0.99 inside,
// one 0.99 and one 1.00 outside.
#[case::afil_d("activ/AFil.d.gds.gz", "activ", &["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"],
    vec!["AFil.d"; 2], vec!["AFil.d"; 3])]
// A 10 µm NWell: a filler 0.15 inside (SG13G2: 0.05 from its nBuLay), one 1.09 and one
// 1.10 outside.
#[case::gfil_e("gatpoly/GFil.e.gds.gz", "gatpoly", &["GFil.g"], vec!["GFil.e"], vec!["GFil.e"; 2])]
// The Schottky stack in a solid NWell that SG13G2 grows an nBuLay in: the 0.3 bar and
// the PWell:block ring 0.5 from the P+ tie are exempt there only.
#[case::schottky_cntb_a("contbar/schottky.gds.gz", "contbar", &[], vec!["CntB.a"], vec![])]
#[case::schottky_pwb_f1("contbar/schottky.gds.gz", "pwellblock", &[], vec!["PWB.f1"], vec![])]
fn no_generated_nbulay(
    #[case] gds: &str,
    #[case] deck: &str,
    #[case] ignore: &[&str],
    #[case] c5l: Vec<&str>,
    #[case] g2: Vec<&str>,
) {
    let path = format!("{C5L_DATA}/{gds}");
    assert_eq!(drc(C5L, &path, deck, ignore), c5l, "CMOS5L");
    assert_eq!(drc(G2, &path, deck, ignore), g2, "SG13G2");
}

/// A layer CMOS5L forbids is empty in a layout it accepts, so a shared rule reading it
/// reads nothing - unless a derived layer unites it with one CMOS5L does have.  That
/// union is SG13G2's way of saying "the layer, or what the process generates in its
/// place" (nBuLay OR the NWell inset, section 4.2), and in CMOS5L it puts the generated
/// part back without the layer (issue #46).  Every rule's layers, its layer params and
/// everything they derive from are walked; a union of a forbidden layer with any other
/// source fails, and CMOS5L redefines the layer above it (see [`CMOS5L_OWN`]).
#[test]
fn no_rule_reads_a_forbidden_layer_united_with_another() {
    let c5l = PdkConfig::for_process(C5L).unwrap();
    let forbidden: Vec<String> = c5l
        .load_deck("forbidden")
        .unwrap()
        .into_iter()
        .filter(|r| r.check == "forbidden")
        .flat_map(|r| r.layers.into_iter().map(|l| l.name))
        .collect();
    assert!(forbidden.iter().any(|l| l == "nBuLay"), "{forbidden:?}");
    let derived = |name: &str| -> Option<(&str, &[String])> {
        let v = c5l.virtual_layers.iter().find(|v| v.name == name);
        let e = c5l.edge_layers.iter().find(|e| e.name == name);
        v.map(|v| (v.op.as_str(), v.layers.as_slice()))
            .or(e.map(|e| (e.op.as_str(), e.layers.as_slice())))
    };
    let mut bad = Vec::new();
    for deck in &c5l.decks {
        for rule in c5l.load_deck(&deck.name).unwrap() {
            let words = rule.params.values().filter_map(|p| match p {
                Param::Word(w) if c5l.layer(w).is_some() => Some(w.clone()),
                _ => None,
            });
            let mut todo: Vec<String> = rule.layers.iter().map(|l| l.name.clone()).collect();
            todo.extend(words);
            let mut seen = std::collections::HashSet::new();
            while let Some(name) = todo.pop() {
                if !seen.insert(name.clone()) {
                    continue;
                }
                let Some((op, sources)) = derived(&name) else {
                    continue;
                };
                let union = matches!(op, "union" | "or");
                if union
                    && sources.len() > 1
                    && sources.iter().any(|s| forbidden.contains(s))
                    && sources.iter().any(|s| !forbidden.contains(s))
                {
                    bad.push(format!("{} ({}): {name} = {sources:?}", rule.id, deck.name));
                }
                todo.extend(sources.iter().cloned());
            }
        }
    }
    bad.sort();
    bad.dedup();
    assert!(
        bad.is_empty(),
        "rules reading a forbidden layer united with another:\n{}",
        bad.join("\n")
    );
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
