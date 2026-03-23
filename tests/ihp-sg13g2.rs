// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use gdscheck::run_drc;
use rstest::rstest;

const DATA: &str = "tests/data/ihp-sg13g2";

fn drc(pdk: &str, deck: &str, gds: &str, topcell: &str, ignore: &[&str]) -> Vec<String> {
    let path = format!("{DATA}/{gds}");
    let violations = run_drc(&path, pdk, &[deck], None, topcell, true).expect("DRC run failed");
    let mut ids: Vec<String> = violations
        .into_iter()
        .filter(|v| !ignore.contains(&v.rule_id.as_str()))
        .map(|v| v.rule_id)
        .collect();
    ids.sort();
    ids
}

const PDK_IHP: &str = "ihp-sg13g2";

// --- Activ ---

const DECK_ACTIV: &str = "activ";

#[rstest]
#[case("activ/Act.a.gds.gz", "TOP", vec!["Act.a", "Act.a", "Act.a", "Act.a"], vec!["Act.d", "AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.b.space.gds.gz", "TOP", vec!["Act.b", "Act.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.b.notch.gds.gz", "TOP", vec!["Act.b", "Act.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.c.gds.gz", "TOP", vec!["Act.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.e.gds.gz", "TOP", vec!["Act.e"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.d.gds.gz", "TOP", vec!["Act.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.d.merge.gds.gz", "TOP", vec![], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.a.gds.gz", "TOP", vec!["AFil.a", "AFil.a", "AFil.a", "AFil.a"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.a1.gds.gz", "TOP", vec!["AFil.a1", "AFil.a1", "AFil.a1", "AFil.a1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.b.gds.gz", "TOP", vec!["AFil.b", "AFil.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.cont.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.gatpoly.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c1.gds.gz", "TOP", vec!["AFil.c1", "AFil.c1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nwell.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nbulay.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.e.gds.gz", "TOP", vec!["AFil.e", "AFil.e"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.i.gds.gz", "TOP", vec!["AFil.i", "AFil.i"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.j.gds.gz", "TOP", vec!["AFil.j"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.g.gds.gz", "TOP", vec![], vec!["AFil.a"])]
#[case("activ/AFil.g.fail.gds.gz", "TOP", vec!["AFil.g"], vec!["AFil.a"])]
#[case("activ/AFil.g1.gds.gz", "TOP", vec![], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g1.fail.gds.gz", "TOP", vec!["AFil.g1"], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g2.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g2.fail.gds.gz", "TOP", vec!["AFil.g2"; 4], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g3.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g1"])]
#[case("activ/AFil.g3.fail.gds.gz", "TOP", vec!["AFil.g3"; 4], vec!["Act.b", "AFil.g1"])]
#[case::afil_g2_boundary_ok("activ/AFil.g2.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::afil_g2_boundary_ring("activ/AFil.g2.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_activ(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_ACTIV, gds, topcell, &ignore), expected);
}

// --- ThickGateOxide ---

const DECK_TGO: &str = "tgo";

#[rstest]
#[case("tgo/TGO.a.gds.gz", "TOP", vec!["TGO.a"], vec![])]
#[case("tgo/TGO.b.gds.gz", "TOP", vec!["TGO.b", "TGO.b"], vec![])]
#[case("tgo/TGO.c.gds.gz", "TOP", vec!["TGO.c"], vec![])]
#[case("tgo/TGO.d.gds.gz", "TOP", vec!["TGO.d"], vec![])]
#[case("tgo/TGO.e.gds.gz", "TOP", vec!["TGO.e", "TGO.e"], vec![])]
#[case("tgo/TGO.f.gds.gz", "TOP", vec!["TGO.f"; 4], vec![])]
fn test_tgo(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TGO, gds, topcell, &ignore), expected);
}

// --- GatPoly ---

const DECK_GAT: &str = "gatpoly";

#[rstest]
#[case("gatpoly/Gat.a.gds.gz", "TOP", vec!["Gat.a", "Gat.a", "Gat.a", "Gat.a"], vec!["Gat.e", "GFil.g"])]
#[case("gatpoly/Gat.a1.gds.gz", "TOP", vec!["Gat.a1", "Gat.a1"], vec!["Gat.a", "GFil.g"])]
#[case("gatpoly/Gat.a2.gds.gz", "TOP", vec!["Gat.a2", "Gat.a2"], vec!["Gat.a", "GFil.g"])]
#[case("gatpoly/Gat.a3.gds.gz", "TOP", vec!["Gat.a3", "Gat.a3"], vec!["GFil.g"])]
#[case("gatpoly/Gat.a4.gds.gz", "TOP", vec!["Gat.a4", "Gat.a4"], vec!["GFil.g"])]
#[case("gatpoly/Gat.b.space.gds.gz", "TOP", vec!["Gat.b", "Gat.b"], vec!["GFil.g"])]
#[case("gatpoly/Gat.b.notch.gds.gz", "TOP", vec!["Gat.b", "Gat.b"], vec!["GFil.g"])]
#[case("gatpoly/Gat.b1.gds.gz", "TOP", vec!["Gat.b1"], vec!["GFil.g"])]
#[case("gatpoly/Gat.c.gds.gz", "TOP", vec!["Gat.c"], vec!["GFil.g"])]
#[case("gatpoly/Gat.d.gds.gz", "TOP", vec!["Gat.d", "Gat.d"], vec!["GFil.g"])]
#[case("gatpoly/Gat.e.gds.gz", "TOP", vec!["Gat.e"], vec!["GFil.g"])]
#[case("gatpoly/Gat.f.gds.gz", "TOP", vec!["Gat.f", "Gat.f"], vec!["GFil.g", "Gat.c", "Gat.e"])]
#[case("gatpoly/Gat.g.gds.gz", "TOP", vec!["Gat.g", "Gat.g"], vec!["Gat.e", "GFil.g"])]
#[case("gatpoly/GFil.a.gds.gz", "TOP", vec!["GFil.a"; 4], vec!["GFil.g"])]
#[case("gatpoly/GFil.b.gds.gz", "TOP", vec!["GFil.b"; 4], vec!["GFil.g"])]
#[case("gatpoly/GFil.c.gds.gz", "TOP", vec!["GFil.c", "GFil.c"], vec!["GFil.g"])]
#[case("gatpoly/GFil.d.gds.gz", "TOP", vec!["GFil.d"; 12], vec!["GFil.g"])]
#[case("gatpoly/GFil.e.nwell.gds.gz", "TOP", vec!["GFil.e", "GFil.e"], vec!["GFil.g"])]
#[case("gatpoly/GFil.e.nbulay.gds.gz", "TOP", vec!["GFil.e", "GFil.e"], vec!["GFil.g"])]
#[case("gatpoly/GFil.f.gds.gz", "TOP", vec!["GFil.f", "GFil.f"], vec!["GFil.g"])]
#[case("gatpoly/GFil.i.gds.gz", "TOP", vec!["GFil.i"], vec!["GFil.g"])]
#[case("gatpoly/GFil.j.gds.gz", "TOP", vec!["GFil.j"], vec!["GFil.g"])]
#[case("gatpoly/GFil.g.gds.gz", "TOP", vec![], vec!["GFil.a"])]
#[case("gatpoly/GFil.g.fail.gds.gz", "TOP", vec!["GFil.g"], vec!["GFil.a"])]
#[case::gfil_g_boundary_ok("gatpoly/GFil.g.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::gfil_g_boundary_ring("gatpoly/GFil.g.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_gatpoly(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_GAT, gds, topcell, &ignore), expected);
}

// --- Cont ---

const DECK_CNT: &str = "cont";

#[rstest]
#[case::cnt_a("cont/Cnt.a.gds.gz", "TOP", vec!["Cnt.a"; 8], vec!["Cnt.c", "Cnt.d", "Cnt.g", "Cnt.h"])]
#[case::cnt_b("cont/Cnt.b.gds.gz", "TOP", vec!["Cnt.b", "Cnt.b"], vec!["Cnt.c", "Cnt.d", "Cnt.g", "Cnt.h"])]
#[case::cnt_e("cont/Cnt.e.gds.gz", "TOP", vec!["Cnt.e"], vec!["Cnt.c", "Cnt.h"])]
#[case::cnt_f("cont/Cnt.f.gds.gz", "TOP", vec!["Cnt.f"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_g("cont/Cnt.g.gds.gz", "TOP", vec!["Cnt.g"], vec!["Cnt.c", "Cnt.d", "Cnt.h"])]
#[case::cnt_g1("cont/Cnt.g1.gds.gz", "TOP", vec!["Cnt.g1"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_g2("cont/Cnt.g2.gds.gz", "TOP", vec!["Cnt.g2"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_h("cont/Cnt.h.gds.gz", "TOP", vec!["Cnt.h"], vec!["Cnt.c", "Cnt.d", "Cnt.g"])]
#[case::cnt_j("cont/Cnt.j.gds.gz", "TOP", vec!["Cnt.j"], vec!["Cnt.c", "Cnt.h"])]
fn test_cont(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_CNT, gds, topcell, &ignore), expected);
}

// --- ContBar ---

const DECK_CNTB: &str = "contbar";

#[rstest]
#[case::cntb_a("contbar/CntB.a.gds.gz", "TOP", vec!["CntB.a"; 2], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_a1("contbar/CntB.a1.gds.gz", "TOP", vec!["CntB.a1"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_b("contbar/CntB.b.gds.gz", "TOP", vec!["CntB.b"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_b1("contbar/CntB.b1.gds.gz", "TOP", vec!["CntB.b1"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_c("contbar/CntB.c.gds.gz", "TOP", vec!["CntB.c"], vec!["CntB.d", "CntB.h", "CntB.h1"])]
#[case::cntb_h1("contbar/CntB.h1.gds.gz", "TOP", vec!["CntB.h1"], vec!["CntB.c", "CntB.d", "CntB.g"])]
#[case::cntb_g("contbar/CntB.g.gds.gz", "TOP", vec!["CntB.g"], vec!["CntB.c", "CntB.d", "CntB.h", "CntB.h1"])]
#[case::cntb_j("contbar/CntB.j.gds.gz", "TOP", vec!["CntB.j"], vec!["CntB.c", "CntB.h", "CntB.h1"])]
fn test_contbar(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_CNTB, gds, topcell, &ignore), expected);
}

// --- SalBlock ---

const DECK_SAL: &str = "salblock";

#[rstest]
#[case::sal_a("salblock/Sal.a.gds.gz", "TOP", vec!["Sal.a"; 4], vec![])]
#[case::sal_b_space("salblock/Sal.b.space.gds.gz", "TOP", vec!["Sal.b", "Sal.b"], vec![])]
#[case::sal_b_notch("salblock/Sal.b.notch.gds.gz", "TOP", vec!["Sal.b", "Sal.b"], vec![])]
#[case::sal_c("salblock/Sal.c.gds.gz", "TOP", vec!["Sal.c"], vec![])]
#[case::sal_d("salblock/Sal.d.gds.gz", "TOP", vec!["Sal.d", "Sal.d"], vec![])]
#[case::sal_e("salblock/Sal.e.gds.gz", "TOP", vec!["Sal.e", "Sal.e"], vec![])]
fn test_salblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SAL, gds, topcell, &ignore), expected);
}

// --- nSD:block ---

const DECK_NSDB: &str = "nsdblock";

#[rstest]
#[case::nsdb_a("nsdblock/nSDB.a.gds.gz", "TOP", vec!["nSDB.a"; 4], vec![])]
#[case::nsdb_b_space("nsdblock/nSDB.b.space.gds.gz", "TOP", vec!["nSDB.b", "nSDB.b"], vec![])]
#[case::nsdb_b_notch("nsdblock/nSDB.b.notch.gds.gz", "TOP", vec!["nSDB.b", "nSDB.b"], vec![])]
#[case::nsdb_c("nsdblock/nSDB.c.gds.gz", "TOP", vec!["nSDB.c", "nSDB.c"], vec![])]
#[case::nsdb_e("nsdblock/nSDB.e.gds.gz", "TOP", vec!["nSDB.e"], vec![])]
fn test_nsdblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NSDB, gds, topcell, &ignore), expected);
}

// --- nBuLay:block ---

const DECK_NBLB: &str = "nbulayblock";

#[rstest]
#[case::nblb_a("nbulayblock/NBLB.a.gds.gz", "TOP", vec!["NBLB.a"; 4], vec!["NBLB.c"])]
#[case::nblb_b_space("nbulayblock/NBLB.b.space.gds.gz", "TOP", vec!["NBLB.b", "NBLB.b"], vec!["NBLB.c"])]
#[case::nblb_b_notch("nbulayblock/NBLB.b.notch.gds.gz", "TOP", vec!["NBLB.b", "NBLB.b"], vec!["NBLB.c"])]
#[case::nblb_c("nbulayblock/NBLB.c.gds.gz", "TOP", vec!["NBLB.c"; 4], vec![])]
#[case::nblb_d("nbulayblock/NBLB.d.gds.gz", "TOP", vec!["NBLB.d", "NBLB.d"], vec!["NBLB.c"])]
fn test_nbulayblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NBLB, gds, topcell, &ignore), expected);
}

// --- nBuLay ---

const DECK_NBL: &str = "nbulay";

#[rstest]
#[case::nbl_a("nbulay/NBL.a.gds.gz", "TOP", vec!["NBL.a"; 4], vec![])]
// 1.20 gap fires; the exactly-1.50 gap is clean (and closes in nBuLayMerged, so no NBL.c).
#[case::nbl_b("nbulay/NBL.b.gds.gz", "TOP", vec!["NBL.b"], vec![])]
// The 1.00 "merged" pair now also legitimately draws the new NBL.b — ignored here.
#[case::nbl_c("nbulay/NBL.c.gds.gz", "TOP", vec!["NBL.c"], vec!["NBL.b"])]
#[case::nbl_d("nbulay/NBL.d.gds.gz", "TOP", vec!["NBL.d", "NBL.d"], vec![])]
#[case::nbl_e("nbulay/NBL.e.gds.gz", "TOP", vec!["NBL.e", "NBL.e"], vec![])]
#[case::nbl_f("nbulay/NBL.f.gds.gz", "TOP", vec!["NBL.f", "NBL.f"], vec![])]
fn test_nbulay(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NBL, gds, topcell, &ignore), expected);
}

// --- PWell:block ---

const DECK_PWB: &str = "pwellblock";

#[rstest]
#[case::pwb_a("pwellblock/PWB.a.gds.gz", "TOP", vec!["PWB.a"; 4], vec![])]
#[case::pwb_b_space("pwellblock/PWB.b.space.gds.gz", "TOP", vec!["PWB.b", "PWB.b"], vec![])]
#[case::pwb_b_notch("pwellblock/PWB.b.notch.gds.gz", "TOP", vec!["PWB.b", "PWB.b"], vec![])]
#[case::pwb_c("pwellblock/PWB.c.gds.gz", "TOP", vec!["PWB.c", "PWB.c"], vec![])]
#[case::pwb_e("pwellblock/PWB.e.gds.gz", "TOP", vec!["PWB.e"], vec![])]
#[case::pwb_e1("pwellblock/PWB.e1.gds.gz", "TOP", vec!["PWB.e1"], vec![])]
#[case::pwb_f("pwellblock/PWB.f.gds.gz", "TOP", vec!["PWB.f"], vec![])]
#[case::pwb_f1("pwellblock/PWB.f1.gds.gz", "TOP", vec!["PWB.f1"], vec![])]
fn test_pwellblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PWB, gds, topcell, &ignore), expected);
}

// --- NWell ---

const DECK_NW: &str = "nwell";

#[rstest]
#[case::nw_a("nwell/NW.a.gds.gz", "TOP", vec!["NW.a"; 4], vec![])]
// 0.50 gap fires; the exactly-0.62 gap is clean (and closes in NWellMerged, so no NW.b1).
#[case::nw_b("nwell/NW.b.gds.gz", "TOP", vec!["NW.b"], vec![])]
// The 0.50 "merged" pair now also legitimately draws the new NW.b — ignored here.
#[case::nw_b1("nwell/NW.b1.gds.gz", "TOP", vec!["NW.b1"], vec!["NW.b"])]
// NW.e is back (projection-metric enclosure + skip_clipped): the 0.20-margin tie fires
// (exact same edge pair as KLayout), the 0.30 tie is clean, and the NWell-crossing tie is
// skipped — not "surrounded entirely by NWell" per the PDF (KLayout additionally flags the
// crossing as overlap errors under NW.e/NW.d; our NW.d models only drawn-nSD N+, a known gap).
#[case::nw_e("nwell/NW.e.gds.gz", "TOP", vec!["NW.e"], vec![])]
#[case::nw_e1("nwell/NW.e1.gds.gz", "TOP", vec!["NW.e1"], vec![])]
#[case::nw_f("nwell/NW.f.gds.gz", "TOP", vec!["NW.f"], vec![])]
#[case::nw_f1("nwell/NW.f1.gds.gz", "TOP", vec!["NW.f1"], vec![])]
#[case::nw_c("nwell/NW.c.gds.gz", "TOP", vec!["NW.c"], vec![])]
#[case::nw_c1("nwell/NW.c1.gds.gz", "TOP", vec!["NW.c1"], vec![])]
// Drawn-nSD and plain undoped Activ (N+ by default) both fire at 0.30; the 0.31 gap and
// the Activ-under-nSD:block canary stay clean.  KLayout marks the same two instances.
#[case::nw_d("nwell/NW.d.gds.gz", "TOP", vec!["NW.d"; 2], vec![])]
#[case::nw_d1("nwell/NW.d1.gds.gz", "TOP", vec!["NW.d1"], vec![])]
// Chapter 8: inside a DigiBnd the HV rules relax to 0.31/0.31/0.24 (.dig variants); the
// relaxed-clean instances prove the strict values no longer apply there, and the strict
// NW.d1 still fires outside.  All four match KLayout 1:1.
#[case::nw_dig("nwell/NW.dig.gds.gz", "TOP",
    vec!["NW.c1.dig", "NW.d1", "NW.d1.dig", "NW.e1.dig"], vec![])]
fn test_nwell(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NW, gds, topcell, &ignore), expected);
}

// --- Metal1 ---

const DECK_M1: &str = "metal1";

#[rstest]
#[case::m1_a("metal1/M1.a.gds.gz", "TOP", vec!["M1.a", "M1.a", "M1.a", "M1.a"], vec!["M1.d", "M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_space("metal1/M1.b.space.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_notch("metal1/M1.b.notch.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_j_ok("metal1/M1.j.gds.gz", "TOP", vec![], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_j_fail("metal1/M1.j.fail.gds.gz", "TOP", vec!["M1.j"], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_k_ok("metal1/M1.k.gds.gz", "TOP", vec![], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_k_fail("metal1/M1.k.fail.gds.gz", "TOP", vec!["M1.k"], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1fil_c("metal1/M1Fil.c.gds.gz", "TOP", vec!["M1Fil.c", "M1Fil.c"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1fil_h_ok("metal1/M1Fil.h.gds.gz", "TOP", vec![], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_fail("metal1/M1Fil.h.fail.gds.gz", "TOP", vec!["M1Fil.h"; 4], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_boundary_ok("metal1/M1Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_h_boundary_fail("metal1/M1Fil.h.boundary_fail.gds.gz", "TOP", vec!["M1Fil.h"], vec![])]
#[case::m1fil_h_boundary_ring("metal1/M1Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_k_ok("metal1/M1Fil.k.gds.gz", "TOP", vec![], vec!["M1.b", "M1.k"])]
#[case::m1fil_k_fail("metal1/M1Fil.k.fail.gds.gz", "TOP", vec!["M1Fil.k"; 4], vec!["M1.b", "M1.k"])]
fn test_metal1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M1, gds, topcell, &ignore), expected);
}

// --- Metal2 ---

const DECK_M2: &str = "metal2";

#[rstest]
#[case::m2_a("metal2/M2.a.gds.gz", "TOP", vec!["M2.a", "M2.a", "M2.a", "M2.a"], vec!["M2.d", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_b_space("metal2/M2.b.space.gds.gz", "TOP", vec!["M2.b", "M2.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_b_notch("metal2/M2.b.notch.gds.gz", "TOP", vec!["M2.b", "M2.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_c("metal2/M2.c.gds.gz", "TOP", vec!["M2.c"; 4], vec!["M2.c1", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_c1("metal2/M2.c1.gds.gz", "TOP", vec!["M2.c1"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_d("metal2/M2.d.gds.gz", "TOP", vec!["M2.d"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_e_fail("metal2/M2.e.fail.gds.gz", "TOP", vec!["M2.e"; 2], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_e_ok("metal2/M2.e.gds.gz", "TOP", vec![], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_f_fail("metal2/M2.f.fail.gds.gz", "TOP", vec!["M2.f"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_f_ok("metal2/M2.f.gds.gz", "TOP", vec![], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_g("metal2/M2.g.gds.gz", "TOP", vec!["M2.g", "M2.g"], vec!["M2.d", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_i("metal2/M2.i.gds.gz", "TOP", vec!["M2.i"], vec!["M2.b", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_j_ok("metal2/M2.j.gds.gz", "TOP", vec![], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_j_fail("metal2/M2.j.fail.gds.gz", "TOP", vec!["M2.j"], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_k_ok("metal2/M2.k.gds.gz", "TOP", vec![], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_k_fail("metal2/M2.k.fail.gds.gz", "TOP", vec!["M2.k"], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2fil_c("metal2/M2Fil.c.gds.gz", "TOP", vec!["M2Fil.c", "M2Fil.c"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a1("metal2/M2Fil.a1.gds.gz", "TOP", vec!["M2Fil.a1"; 4], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a2("metal2/M2Fil.a2.gds.gz", "TOP", vec!["M2Fil.a2"; 4], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_b("metal2/M2Fil.b.gds.gz", "TOP", vec!["M2Fil.b", "M2Fil.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_d("metal2/M2Fil.d.gds.gz", "TOP", vec!["M2Fil.d", "M2Fil.d"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_h_ok("metal2/M2Fil.h.gds.gz", "TOP", vec![], vec!["M2.b", "M2.j"])]
#[case::m2fil_h_fail("metal2/M2Fil.h.fail.gds.gz", "TOP", vec!["M2Fil.h"; 4], vec!["M2.b", "M2.j"])]
#[case::m2fil_k_ok("metal2/M2Fil.k.gds.gz", "TOP", vec![], vec!["M2.b", "M2.k"])]
#[case::m2fil_k_fail("metal2/M2Fil.k.fail.gds.gz", "TOP", vec!["M2Fil.k"; 4], vec!["M2.b", "M2.k"])]
#[case::m2fil_h_boundary_ok("metal2/M2Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m2fil_h_boundary_fail("metal2/M2Fil.h.boundary_fail.gds.gz", "TOP", vec!["M2Fil.h"], vec![])]
#[case::m2fil_h_boundary_ring("metal2/M2Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M2, gds, topcell, &ignore), expected);
}

// --- Metal3 ---

const DECK_M3: &str = "metal3";

#[rstest]
#[case::m3_a("metal3/M3.a.gds.gz", "TOP", vec!["M3.a", "M3.a", "M3.a", "M3.a"], vec!["M3.d", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_b_space("metal3/M3.b.space.gds.gz", "TOP", vec!["M3.b", "M3.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_b_notch("metal3/M3.b.notch.gds.gz", "TOP", vec!["M3.b", "M3.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_c("metal3/M3.c.gds.gz", "TOP", vec!["M3.c"; 4], vec!["M3.c1", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_c1("metal3/M3.c1.gds.gz", "TOP", vec!["M3.c1"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_d("metal3/M3.d.gds.gz", "TOP", vec!["M3.d"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_e_fail("metal3/M3.e.fail.gds.gz", "TOP", vec!["M3.e"; 2], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_e_ok("metal3/M3.e.gds.gz", "TOP", vec![], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_f_fail("metal3/M3.f.fail.gds.gz", "TOP", vec!["M3.f"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_f_ok("metal3/M3.f.gds.gz", "TOP", vec![], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_g("metal3/M3.g.gds.gz", "TOP", vec!["M3.g", "M3.g"], vec!["M3.d", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_i("metal3/M3.i.gds.gz", "TOP", vec!["M3.i"], vec!["M3.b", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_j_ok("metal3/M3.j.gds.gz", "TOP", vec![], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_j_fail("metal3/M3.j.fail.gds.gz", "TOP", vec!["M3.j"], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_k_ok("metal3/M3.k.gds.gz", "TOP", vec![], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_k_fail("metal3/M3.k.fail.gds.gz", "TOP", vec!["M3.k"], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3fil_c("metal3/M3Fil.c.gds.gz", "TOP", vec!["M3Fil.c", "M3Fil.c"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a1("metal3/M3Fil.a1.gds.gz", "TOP", vec!["M3Fil.a1"; 4], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a2("metal3/M3Fil.a2.gds.gz", "TOP", vec!["M3Fil.a2"; 4], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_b("metal3/M3Fil.b.gds.gz", "TOP", vec!["M3Fil.b", "M3Fil.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_d("metal3/M3Fil.d.gds.gz", "TOP", vec!["M3Fil.d", "M3Fil.d"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_h_ok("metal3/M3Fil.h.gds.gz", "TOP", vec![], vec!["M3.b", "M3.j"])]
#[case::m3fil_h_fail("metal3/M3Fil.h.fail.gds.gz", "TOP", vec!["M3Fil.h"; 4], vec!["M3.b", "M3.j"])]
#[case::m3fil_k_ok("metal3/M3Fil.k.gds.gz", "TOP", vec![], vec!["M3.b", "M3.k"])]
#[case::m3fil_k_fail("metal3/M3Fil.k.fail.gds.gz", "TOP", vec!["M3Fil.k"; 4], vec!["M3.b", "M3.k"])]
#[case::m3fil_h_boundary_ok("metal3/M3Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m3fil_h_boundary_fail("metal3/M3Fil.h.boundary_fail.gds.gz", "TOP", vec!["M3Fil.h"], vec![])]
#[case::m3fil_h_boundary_ring("metal3/M3Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal3(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M3, gds, topcell, &ignore), expected);
}

// --- Metal4 ---

const DECK_M4: &str = "metal4";

#[rstest]
#[case::m4_a("metal4/M4.a.gds.gz", "TOP", vec!["M4.a", "M4.a", "M4.a", "M4.a"], vec!["M4.d", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_b_space("metal4/M4.b.space.gds.gz", "TOP", vec!["M4.b", "M4.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_b_notch("metal4/M4.b.notch.gds.gz", "TOP", vec!["M4.b", "M4.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_c("metal4/M4.c.gds.gz", "TOP", vec!["M4.c"; 4], vec!["M4.c1", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_c1("metal4/M4.c1.gds.gz", "TOP", vec!["M4.c1"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_d("metal4/M4.d.gds.gz", "TOP", vec!["M4.d"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_e_fail("metal4/M4.e.fail.gds.gz", "TOP", vec!["M4.e"; 2], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_e_ok("metal4/M4.e.gds.gz", "TOP", vec![], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_f_fail("metal4/M4.f.fail.gds.gz", "TOP", vec!["M4.f"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_f_ok("metal4/M4.f.gds.gz", "TOP", vec![], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_g("metal4/M4.g.gds.gz", "TOP", vec!["M4.g", "M4.g"], vec!["M4.d", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_i("metal4/M4.i.gds.gz", "TOP", vec!["M4.i"], vec!["M4.b", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_j_ok("metal4/M4.j.gds.gz", "TOP", vec![], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_j_fail("metal4/M4.j.fail.gds.gz", "TOP", vec!["M4.j"], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_k_ok("metal4/M4.k.gds.gz", "TOP", vec![], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_k_fail("metal4/M4.k.fail.gds.gz", "TOP", vec!["M4.k"], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4fil_c("metal4/M4Fil.c.gds.gz", "TOP", vec!["M4Fil.c", "M4Fil.c"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a1("metal4/M4Fil.a1.gds.gz", "TOP", vec!["M4Fil.a1"; 4], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a2("metal4/M4Fil.a2.gds.gz", "TOP", vec!["M4Fil.a2"; 4], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_b("metal4/M4Fil.b.gds.gz", "TOP", vec!["M4Fil.b", "M4Fil.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_d("metal4/M4Fil.d.gds.gz", "TOP", vec!["M4Fil.d", "M4Fil.d"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_h_ok("metal4/M4Fil.h.gds.gz", "TOP", vec![], vec!["M4.b", "M4.j"])]
#[case::m4fil_h_fail("metal4/M4Fil.h.fail.gds.gz", "TOP", vec!["M4Fil.h"; 4], vec!["M4.b", "M4.j"])]
#[case::m4fil_k_ok("metal4/M4Fil.k.gds.gz", "TOP", vec![], vec!["M4.b", "M4.k"])]
#[case::m4fil_k_fail("metal4/M4Fil.k.fail.gds.gz", "TOP", vec!["M4Fil.k"; 4], vec!["M4.b", "M4.k"])]
#[case::m4fil_h_boundary_ok("metal4/M4Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m4fil_h_boundary_fail("metal4/M4Fil.h.boundary_fail.gds.gz", "TOP", vec!["M4Fil.h"], vec![])]
#[case::m4fil_h_boundary_ring("metal4/M4Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal4(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M4, gds, topcell, &ignore), expected);
}

// --- Metal5 ---

const DECK_M5: &str = "metal5";

#[rstest]
#[case::m5_a("metal5/M5.a.gds.gz", "TOP", vec!["M5.a", "M5.a", "M5.a", "M5.a"], vec!["M5.d", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_b_space("metal5/M5.b.space.gds.gz", "TOP", vec!["M5.b", "M5.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_b_notch("metal5/M5.b.notch.gds.gz", "TOP", vec!["M5.b", "M5.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_c("metal5/M5.c.gds.gz", "TOP", vec!["M5.c"; 4], vec!["M5.c1", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_c1("metal5/M5.c1.gds.gz", "TOP", vec!["M5.c1"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_d("metal5/M5.d.gds.gz", "TOP", vec!["M5.d"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_e_fail("metal5/M5.e.fail.gds.gz", "TOP", vec!["M5.e"; 2], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_e_ok("metal5/M5.e.gds.gz", "TOP", vec![], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_f_fail("metal5/M5.f.fail.gds.gz", "TOP", vec!["M5.f"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_f_ok("metal5/M5.f.gds.gz", "TOP", vec![], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_g("metal5/M5.g.gds.gz", "TOP", vec!["M5.g", "M5.g"], vec!["M5.d", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_i("metal5/M5.i.gds.gz", "TOP", vec!["M5.i"], vec!["M5.b", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_j_ok("metal5/M5.j.gds.gz", "TOP", vec![], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_j_fail("metal5/M5.j.fail.gds.gz", "TOP", vec!["M5.j"], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_k_ok("metal5/M5.k.gds.gz", "TOP", vec![], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_k_fail("metal5/M5.k.fail.gds.gz", "TOP", vec!["M5.k"], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5fil_c("metal5/M5Fil.c.gds.gz", "TOP", vec!["M5Fil.c", "M5Fil.c"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a1("metal5/M5Fil.a1.gds.gz", "TOP", vec!["M5Fil.a1"; 4], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a2("metal5/M5Fil.a2.gds.gz", "TOP", vec!["M5Fil.a2"; 4], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_b("metal5/M5Fil.b.gds.gz", "TOP", vec!["M5Fil.b", "M5Fil.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_d("metal5/M5Fil.d.gds.gz", "TOP", vec!["M5Fil.d", "M5Fil.d"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_h_ok("metal5/M5Fil.h.gds.gz", "TOP", vec![], vec!["M5.b", "M5.j"])]
#[case::m5fil_h_fail("metal5/M5Fil.h.fail.gds.gz", "TOP", vec!["M5Fil.h"; 4], vec!["M5.b", "M5.j"])]
#[case::m5fil_k_ok("metal5/M5Fil.k.gds.gz", "TOP", vec![], vec!["M5.b", "M5.k"])]
#[case::m5fil_k_fail("metal5/M5Fil.k.fail.gds.gz", "TOP", vec!["M5Fil.k"; 4], vec!["M5.b", "M5.k"])]
#[case::m5fil_h_boundary_ok("metal5/M5Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m5fil_h_boundary_fail("metal5/M5Fil.h.boundary_fail.gds.gz", "TOP", vec!["M5Fil.h"], vec![])]
#[case::m5fil_h_boundary_ring("metal5/M5Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal5(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M5, gds, topcell, &ignore), expected);
}

// --- Via1 ---

const DECK_V1: &str = "via1";

#[rstest]
#[case::v1_a("via1/V1.a.gds.gz", "TOP", vec!["V1.a"; 8], vec!["V1.c", "V1.c1"])]
#[case::v1_b("via1/V1.b.gds.gz", "TOP", vec!["V1.b", "V1.b"], vec!["V1.c", "V1.c1"])]
#[case::v1_c("via1/V1.c.gds.gz", "TOP", vec!["V1.c"; 4], vec!["V1.a", "V1.b", "V1.c1"])]
#[case::v1_c1("via1/V1.c1.gds.gz", "TOP", vec!["V1.c1"], vec!["V1.a", "V1.b", "V1.c"])]
#[case::v1_b1_fail("via1/V1.b1.fail.gds.gz", "TOP", vec!["V1.b1"], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
#[case::v1_b1_ok("via1/V1.b1.gds.gz", "TOP", vec![], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
#[case::v1_b1_ring("via1/V1.b1.ring.gds.gz", "TOP", vec![], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
fn test_via1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V1, gds, topcell, &ignore), expected);
}

// --- Via2 ---

const DECK_V2: &str = "via2";

#[rstest]
#[case::v2_a("via2/V2.a.gds.gz", "TOP", vec!["V2.a"; 8], vec!["V2.c", "V2.c1"])]
#[case::v2_b("via2/V2.b.gds.gz", "TOP", vec!["V2.b", "V2.b"], vec!["V2.c", "V2.c1"])]
#[case::v2_c("via2/V2.c.gds.gz", "TOP", vec!["V2.c"; 4], vec!["V2.a", "V2.b", "V2.c1"])]
#[case::v2_c1("via2/V2.c1.gds.gz", "TOP", vec!["V2.c1"], vec!["V2.a", "V2.b", "V2.c"])]
#[case::v2_b1_fail("via2/V2.b1.fail.gds.gz", "TOP", vec!["V2.b1"], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
#[case::v2_b1_ok("via2/V2.b1.gds.gz", "TOP", vec![], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
#[case::v2_b1_ring("via2/V2.b1.ring.gds.gz", "TOP", vec![], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
fn test_via2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V2, gds, topcell, &ignore), expected);
}

// --- Via3 ---

const DECK_V3: &str = "via3";

#[rstest]
#[case::v3_a("via3/V3.a.gds.gz", "TOP", vec!["V3.a"; 8], vec!["V3.c", "V3.c1"])]
#[case::v3_b("via3/V3.b.gds.gz", "TOP", vec!["V3.b", "V3.b"], vec!["V3.c", "V3.c1"])]
#[case::v3_c("via3/V3.c.gds.gz", "TOP", vec!["V3.c"; 4], vec!["V3.a", "V3.b", "V3.c1"])]
#[case::v3_c1("via3/V3.c1.gds.gz", "TOP", vec!["V3.c1"], vec!["V3.a", "V3.b", "V3.c"])]
#[case::v3_b1_fail("via3/V3.b1.fail.gds.gz", "TOP", vec!["V3.b1"], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
#[case::v3_b1_ok("via3/V3.b1.gds.gz", "TOP", vec![], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
#[case::v3_b1_ring("via3/V3.b1.ring.gds.gz", "TOP", vec![], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
fn test_via3(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V3, gds, topcell, &ignore), expected);
}

// --- Via4 ---

const DECK_V4: &str = "via4";

#[rstest]
#[case::v4_a("via4/V4.a.gds.gz", "TOP", vec!["V4.a"; 8], vec!["V4.c", "V4.c1"])]
#[case::v4_b("via4/V4.b.gds.gz", "TOP", vec!["V4.b", "V4.b"], vec!["V4.c", "V4.c1"])]
#[case::v4_c("via4/V4.c.gds.gz", "TOP", vec!["V4.c"; 4], vec!["V4.a", "V4.b", "V4.c1"])]
#[case::v4_c1("via4/V4.c1.gds.gz", "TOP", vec!["V4.c1"], vec!["V4.a", "V4.b", "V4.c"])]
#[case::v4_b1_fail("via4/V4.b1.fail.gds.gz", "TOP", vec!["V4.b1"], vec!["V4.a", "V4.b", "V4.c", "V4.c1"])]
#[case::v4_b1_ok("via4/V4.b1.gds.gz", "TOP", vec![], vec!["V4.a", "V4.b", "V4.c", "V4.c1"])]
fn test_via4(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V4, gds, topcell, &ignore), expected);
}

// --- TopVia1 ---

const DECK_TV1: &str = "topvia1";

#[rstest]
#[case::tv1_a("topvia1/TV1.a.gds.gz", "TOP", vec!["TV1.a"; 8], vec!["TV1.c", "TV1.d"])]
#[case::tv1_b("topvia1/TV1.b.gds.gz", "TOP", vec!["TV1.b", "TV1.b"], vec!["TV1.c", "TV1.d"])]
#[case::tv1_c("topvia1/TV1.c.gds.gz", "TOP", vec!["TV1.c"; 4], vec!["TV1.a", "TV1.b", "TV1.d"])]
#[case::tv1_d("topvia1/TV1.d.gds.gz", "TOP", vec!["TV1.d"; 4], vec!["TV1.a", "TV1.b", "TV1.c"])]
fn test_topvia1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TV1, gds, topcell, &ignore), expected);
}

// --- TopVia2 ---

const DECK_TV2: &str = "topvia2";

#[rstest]
#[case::tv2_a("topvia2/TV2.a.gds.gz", "TOP", vec!["TV2.a"; 8], vec!["TV2.c", "TV2.d"])]
#[case::tv2_b("topvia2/TV2.b.gds.gz", "TOP", vec!["TV2.b", "TV2.b"], vec!["TV2.c", "TV2.d"])]
#[case::tv2_c("topvia2/TV2.c.gds.gz", "TOP", vec!["TV2.c"; 4], vec!["TV2.a", "TV2.b", "TV2.d"])]
#[case::tv2_d("topvia2/TV2.d.gds.gz", "TOP", vec!["TV2.d"; 4], vec!["TV2.a", "TV2.b", "TV2.c"])]
fn test_topvia2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TV2, gds, topcell, &ignore), expected);
}

// --- TopMetal1 ---

const DECK_TM1: &str = "topmetal1";

#[rstest]
#[case::tm1_a("topmetal1/TM1.a.gds.gz", "TOP", vec!["TM1.a", "TM1.a", "TM1.a", "TM1.a"], vec!["TM1.c", "TM1.d"])]
#[case::tm1_b_space("topmetal1/TM1.b.space.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.c", "TM1.d"])]
#[case::tm1_b_notch("topmetal1/TM1.b.notch.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.a", "TM1.c", "TM1.d"])]
#[case::tm1_b_mixed_notch("topmetal1/TM1.b.mixed_notch.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.a", "TM1.c", "TM1.d"])]
#[case::tm1_c_ok("topmetal1/TM1.c.gds.gz", "TOP", vec![], vec!["TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_c_fail("topmetal1/TM1.c.fail.gds.gz", "TOP", vec!["TM1.c"], vec!["TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_d_ok("topmetal1/TM1.d.gds.gz", "TOP", vec![], vec!["TM1.b", "TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_d_fail("topmetal1/TM1.d.fail.gds.gz", "TOP", vec!["TM1.d"], vec!["TM1.b", "TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1fil_c("topmetal1/TM1Fil.c.gds.gz", "TOP", vec!["TM1Fil.c", "TM1Fil.c"], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a("topmetal1/TM1Fil.a.gds.gz", "TOP", vec!["TM1Fil.a"; 4], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a1("topmetal1/TM1Fil.a1.gds.gz", "TOP", vec!["TM1Fil.a1"; 4], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_b("topmetal1/TM1Fil.b.gds.gz", "TOP", vec!["TM1Fil.b"; 2], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_d("topmetal1/TM1Fil.d.gds.gz", "TOP", vec!["TM1Fil.d"; 2], vec!["TM1.c", "TM1.d"])]
fn test_topmetal1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TM1, gds, topcell, &ignore), expected);
}

// --- TopMetal2 ---

const DECK_TM2: &str = "topmetal2";

#[rstest]
#[case::tm2_a("topmetal2/TM2.a.gds.gz", "TOP", vec!["TM2.a", "TM2.a", "TM2.a", "TM2.a"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_b_space("topmetal2/TM2.b.space.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_b_notch("topmetal2/TM2.b.notch.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.a", "TM2.c", "TM2.d"])]
#[case::tm2_b_mixed_notch("topmetal2/TM2.b.mixed_notch.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.a", "TM2.c", "TM2.d"])]
#[case::tm2_c_ok("topmetal2/TM2.c.gds.gz", "TOP", vec![], vec!["TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_c_fail("topmetal2/TM2.c.fail.gds.gz", "TOP", vec!["TM2.c"], vec!["TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_d_ok("topmetal2/TM2.d.gds.gz", "TOP", vec![], vec!["TM2.b", "TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_d_fail("topmetal2/TM2.d.fail.gds.gz", "TOP", vec!["TM2.d"], vec!["TM2.b", "TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2fil_c("topmetal2/TM2Fil.c.gds.gz", "TOP", vec!["TM2Fil.c", "TM2Fil.c"], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a("topmetal2/TM2Fil.a.gds.gz", "TOP", vec!["TM2Fil.a"; 4], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a1("topmetal2/TM2Fil.a1.gds.gz", "TOP", vec!["TM2Fil.a1"; 4], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_b("topmetal2/TM2Fil.b.gds.gz", "TOP", vec!["TM2Fil.b"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_d("topmetal2/TM2Fil.d.gds.gz", "TOP", vec!["TM2Fil.d"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_fail("topmetal2/TM2.bR.fail.gds.gz", "TOP", vec!["TM2.bR"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ok("topmetal2/TM2.bR.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ind("topmetal2/TM2.bR.ind.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
fn test_topmetal2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TM2, gds, topcell, &ignore), expected);
}

// --- Passiv ---

const DECK_PAS: &str = "passiv";

#[rstest]
#[case::pas_a("passiv/Pas.a.gds.gz", "TOP", vec!["Pas.a", "Pas.a", "Pas.a", "Pas.a"], vec![])]
#[case::pas_b_space("passiv/Pas.b.space.gds.gz", "TOP", vec!["Pas.b", "Pas.b"], vec![])]
#[case::pas_b_notch("passiv/Pas.b.notch.gds.gz", "TOP", vec!["Pas.b", "Pas.b"], vec![])]
#[case::pas_c("passiv/Pas.c.gds.gz", "TOP", vec!["Pas.c"; 4], vec![])]
fn test_passiv(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PAS, gds, topcell, &ignore), expected);
}



// --- Pin ---

const DECK_PIN: &str = "pin";

#[rstest]
#[case::pin_a("pin/Pin.a.gds.gz", "TOP", vec!["Pin.a"; 4], vec![])]
#[case::pin_b("pin/Pin.b.gds.gz", "TOP", vec!["Pin.b"; 4], vec![])]
#[case::pin_e("pin/Pin.e.gds.gz", "TOP", vec!["Pin.e"; 4], vec![])]
#[case::pin_f_m2("pin/Pin.f.m2.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m3("pin/Pin.f.m3.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m4("pin/Pin.f.m4.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m5("pin/Pin.f.m5.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_g("pin/Pin.g.gds.gz", "TOP", vec!["Pin.g"; 4], vec![])]
#[case::pin_h("pin/Pin.h.gds.gz", "TOP", vec!["Pin.h"; 4], vec![])]
fn test_pin(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PIN, gds, topcell, &ignore), expected);
}

// --- LBE ---

const DECK_LBE: &str = "lbe";

#[rstest]
#[case::lbe_a("lbe/LBE.a.gds.gz", "TOP", vec!["LBE.a"; 4], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_b("lbe/LBE.b.gds.gz", "TOP", vec!["LBE.b"; 4], vec!["LBE.b1", "LBE.i"])]
#[case::lbe_b1("lbe/LBE.b1.gds.gz", "TOP", vec!["LBE.b1"; 2], vec!["LBE.i"])]
#[case::lbe_b1_merge("lbe/LBE.b1.merge.gds.gz", "TOP", vec!["LBE.b1"], vec!["LBE.i"])]
#[case::lbe_b2("lbe/LBE.b2.gds.gz", "TOP", vec!["LBE.b2"], vec!["LBE.i"])]
#[case::lbe_c_space("lbe/LBE.c.space.gds.gz", "TOP", vec!["LBE.c"; 2], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_c_notch("lbe/LBE.c.notch.gds.gz", "TOP", vec!["LBE.c"; 2], vec!["LBE.a", "LBE.i"])]
#[case::lbe_d_space("lbe/LBE.d.space.gds.gz", "TOP", vec!["LBE.d"; 2], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_e("lbe/LBE.e.gds.gz", "TOP", vec!["LBE.e"; 2], vec!["LBE.i"])]
#[case::lbe_f("lbe/LBE.f.gds.gz", "TOP", vec!["LBE.f"], vec!["LBE.i"])]
#[case::lbe_h("lbe/LBE.h.gds.gz", "TOP", vec!["LBE.h"], vec!["LBE.i", "LBE.c"])]
#[case::lbe_h_open("lbe/LBE.h.open.gds.gz", "TOP", vec![], vec!["LBE.i", "LBE.c"])]
#[case::lbe_i("lbe/LBE.i.gds.gz", "TOP", vec![], vec![])]
#[case::lbe_i_fail("lbe/LBE.i.fail.gds.gz", "TOP", vec!["LBE.i"], vec![])]
fn test_lbe(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_LBE, gds, topcell, &ignore), expected);
}

// --- EXTBlock ---

const DECK_EXTB: &str = "extblock";

#[rstest]
#[case::extb_a("extblock/EXTB.a.gds.gz", "TOP", vec!["EXTB.a"; 4], vec![])]
#[case::extb_b_space("extblock/EXTB.b.space.gds.gz", "TOP", vec!["EXTB.b"; 2], vec![])]
#[case::extb_b_notch("extblock/EXTB.b.notch.gds.gz", "TOP", vec!["EXTB.b"; 2], vec![])]
#[case::extb_c("extblock/EXTB.c.gds.gz", "TOP", vec!["EXTB.c"; 2], vec![])]
fn test_extblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_EXTB, gds, topcell, &ignore), expected);
}

// --- Pad ---

const DECK_PAD: &str = "pad";

#[rstest]
#[case::pad_a1("pad/Pad.a1.gds.gz", "TOP", vec!["Pad.a1"; 4], vec!["Pad.d", "Padb.a", "Padc.a", "Pad.i"])]
#[case::pad_d("pad/Pad.d.gds.gz", "TOP", vec!["Pad.d"; 1], vec!["Pad.a1", "Padb.a", "Padc.a", "Pad.i"])]
#[case::pad_i("pad/Pad.i.gds.gz", "TOP", vec!["Pad.i"; 1], vec![])]
#[case::padb_a("pad/Padb.a.gds.gz", "TOP", vec!["Padb.a"; 8], vec!["Padb.c", "Padc.a", "Padc.b", "Padc.c", "Pad.i", "Padb.f"])]
#[case::padb_b("pad/Padb.b.gds.gz", "TOP", vec!["Padb.b"; 2], vec!["Padb.c", "Padc.a", "Padc.b", "Padc.c", "Pad.i", "Padb.f"])]
#[case::padb_c("pad/Padb.c.gds.gz", "TOP", vec!["Padb.c"; 4], vec!["Padb.a", "Padc.a", "Padc.b", "Padc.c", "Padb.f"])]
#[case::padb_d("pad/Padb.d.gds.gz", "TOP", vec!["Padb.d"; 1], vec!["Padb.f"])]
// Pad at 25 µm from the seal-Activ fires (1/1 exact match vs KLayout); at exactly 30.0 µm
// clean in both.  The square pads trip Padc.f (circle-only, BEOL rule) — ignored here.
#[case::padc_d("pad/Padc.d.gds.gz", "TOP", vec!["Padc.d"; 1], vec!["Padc.f"])]
#[case::padc_a("pad/Padc.a.gds.gz", "TOP", vec!["Padc.a"; 8], vec!["Padc.c", "Padb.a", "Padb.b", "Padb.c", "Pad.i", "Padc.f"])]
#[case::padc_b("pad/Padc.b.gds.gz", "TOP", vec!["Padc.b"; 2], vec!["Padc.c", "Padb.a", "Padb.b", "Padb.c", "Pad.i", "Padc.f"])]
#[case::padc_c("pad/Padc.c.gds.gz", "TOP", vec!["Padc.c"; 4], vec!["Padc.a", "Padb.a", "Padb.b", "Padb.c", "Padc.f"])]
// A square SBumpPad violates (not circle/octagon); an octagon and a circle both pass.
// Padb.a/b collateral is the pre-existing facing-edge width/space scan applied to the
// non-rectangular octagon/circle shapes — orthogonal to what this case tests.
#[case::padb_f("pad/Padb.f.gds.gz", "TOP", vec!["Padb.f"; 1], vec!["Padb.a", "Padb.b"])]
// A square AND an octagon CuPillarPad both violate (only circle is allowed); a circle passes.
#[case::padc_f("pad/Padc.f.gds.gz", "TOP", vec!["Padc.f"; 2], vec!["Padc.a"])]
fn test_pad(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PAD, gds, topcell, &ignore), expected);
}

// --- pSD (p+ S/D implant) ---

const DECK_PSD: &str = "psd";

#[rstest]
#[case::psd_a("psd/pSD.a.gds.gz", "TOP", vec!["pSD.a"; 2], vec!["pSD.b", "pSD.d", "pSD.k"])]
#[case::psd_b("psd/pSD.b.gds.gz", "TOP", vec!["pSD.b"; 2], vec!["pSD.a", "pSD.d", "pSD.k"])]
#[case::psd_d("psd/pSD.d.gds.gz", "TOP", vec!["pSD.d"; 2], vec!["pSD.a", "pSD.b", "pSD.k"])]
#[case::psd_k("psd/pSD.k.gds.gz", "TOP", vec!["pSD.k"; 1], vec!["pSD.a", "pSD.b", "pSD.l"])]
#[case::psd_l("psd/pSD.l.gds.gz", "TOP", vec!["pSD.l"; 1], vec!["pSD.a", "pSD.b", "pSD.k"])]
// pSD.m/n reuse the resistor-recognition fixtures (pSD too close to / not enclosing a resistor).
#[case::psd_m("resistor/RsilBody.gds.gz", "TOP", vec!["pSD.m"; 1], vec![])]
#[case::psd_n("resistor/RppdBody.gds.gz", "TOP", vec!["pSD.n"; 1], vec![])]
#[case::psd_g("psd/pSD.g.gds.gz", "TOP", vec!["pSD.g"; 2], vec![])]
// Four abutted ties: exact-0.30 sliver (clean), uniform 0.20 (fires), 0.20 with a wide
// pocket (clean — ≥0.30 at one position suffices), 0.2×0.5 tab (fires — width, not
// length, is the metric).  Validated 2/2 exact-location match vs KLayout.
#[case::psd_e("psd/pSD.e.gds.gz", "TOP", vec!["pSD.e"; 2], vec![])]
// Six abutted NWell ties, tab depth is the metric: 0.20/0.295/wide-0.20 fire, 0.30 and
// the 0.2-wide × 0.45-deep tab are clean (all five match KLayout exactly); the L-shaped
// tab fires HERE ONLY (PDF-first — see the deck comment).
#[case::psd_f("psd/pSD.f.gds.gz", "TOP", vec!["pSD.f"; 4], vec![])]
// Margins 0.02 and flush-0 fire, 0.05 and exactly-0.03 clean, the crossing abutted tie
// is clean (protruding part ignored) while a crossing tie with a 0.02 lateral margin
// fires — all validated 1:1 against the FEOL driver (rule is not in the maximal deck).
#[case::psd_c1("psd/pSD.c1.gds.gz", "TOP", vec!["pSD.c1"; 3], vec![])]
fn test_psd(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PSD, gds, topcell, &ignore), expected);
}

// --- poly resistors (Rsil/Rppd/Rhigh) ---

const DECK_RESISTOR: &str = "resistor";

#[rstest]
#[case::rsil("resistor/Rsil.gds.gz", "TOP", vec!["Rsil.b", "Rsil.f", "Rsil.f"], vec!["Rsil.c"])]
#[case::rsil_body("resistor/RsilBody.gds.gz", "TOP",
    vec!["Rsil.a", "Rsil.a", "Rsil.d", "Rsil.e", "Rsil.f", "Rsil.f"], vec![])]
#[case::rsil_c("resistor/Rsil.c.gds.gz", "TOP", vec!["Rsil.c"; 1], vec![])]
#[case::rppd_body("resistor/RppdBody.gds.gz", "TOP",
    vec!["Rppd.a", "Rppd.a", "Rppd.b", "Rppd.d", "Rppd.e", "Rppd.e"], vec![])]
// gap = 0.20 (boundary-exact, clean), 0.10 (< 0.20, min_space), 0.50 (> 0.20, too far).
// Rppd.b collateral: pSD drawn flush with SalBlock (coincident edge, same class as
// RppdBody above) — not the point of this case.
#[case::rppd_c("resistor/Rppd.c.gds.gz", "TOP", vec!["Rppd.c"; 2], vec!["Rppd.b"])]
#[case::rhigh("resistor/Rhigh.gds.gz", "TOP",
    vec!["Rhi.a", "Rhi.a", "Rhi.c", "Rhi.c", "Rhi.f", "Rhi.f"], vec![])]
// Same three-gap pattern as Rppd.c.  Rhi.c collateral (GatPoly extends past pSDnSD)
// fires legitimately since run_enclosure measures partially-overlapping shapes — as
// KLayout always did on this geometry — but it is not this case's concern.
#[case::rhi_d("resistor/Rhi.d.gds.gz", "TOP", vec!["Rhi.d"; 2], vec!["Rhi.c"])]
// nSD overhang at a realistic resistor (fires — stricter than shipped KLayout, see the
// fixture doc) and at a poly-inside-stack body (fires in both tools, exact-location
// validated); identical-implant body and isolated nSD blob stay clean.  Rhi.c ×2: the
// resistor GatPoly sticks out of the implant stack with flush edges — KLayout marks the
// same two shapes (4 per-edge markers at the identical coordinates); measured since
// run_enclosure's partial-overlap branch landed.
#[case::rhi_b("resistor/Rhi.b.gds.gz", "TOP", vec!["Rhi.b", "Rhi.b", "Rhi.c", "Rhi.c"], vec![])]
fn test_resistor(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_RESISTOR, gds, topcell, &ignore), expected);
}

// --- isolated NMOS (nmosi) ---

const DECK_NMOSI: &str = "nmosi";

#[rstest]
#[case::nmosi_b("nmosi/nmosi.b.gds.gz", "TOP", vec!["nmosi.b"], vec![])]
// Ring gap 0.34 fires, ring gap 0.50 clean, and the plain (hole-free) NWell 0.20 from
// an iso-PWell Activ stays silent — only `NWell.with_holes` anchors this rule.
#[case::nmosi_c("nmosi/nmosi.c.gds.gz", "TOP", vec!["nmosi.c"], vec![])]
#[case::nmosi_d("nmosi/nmosi.d.gds.gz", "TOP", vec!["nmosi.d", "nmosi.d"], vec![])]
#[case::nmosi_f("nmosi/nmosi.f.gds.gz", "TOP", vec!["nmosi.f", "nmosi.f"], vec!["nmosi.g"])]
// SalBlock extension past nSD:block over the ptap: 0.05 fires (matches KLayout's marker
// exactly), 0.20 clean, flush-0.00 fires here only (PDF-first; KLayout's coincident-pair
// marker is zero-area and vanishes under its .and(Activ) — see the deck comment).
#[case::nmosi_g("nmosi/nmosi.g.gds.gz", "TOP", vec!["nmosi.g"; 2], vec![])]
fn test_nmosi(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NMOSI, gds, topcell, &ignore), expected);
}

// --- NPN bipolar (npnG2 substrate ties, npn13G2* emitters) ---

const DECK_NPN: &str = "npn";

#[rstest]
// npnG2.c fires twice on one ring: the check reports the worst pair per enclosed
// polygon per owning tile bucket, and the tie ring straddles a tile boundary
// (KLayout reports the same violation as 8 edge pairs — same device, same rule).
#[case::npn_g2("npn/npnG2.gds.gz", "TOP",
    vec!["npnG2.b", "npnG2.c", "npnG2.c", "npnG2.d", "npnG2.e"], vec![])]
// One marker per emitter case: min/max for G2 (=0.90), L (1.00..2.50), V (1.00..5.00).
// KLayout's shipped emitter rules are dead code (µm/dbu bug in ext_with_length's
// ">" branch) — limits here follow the PDF; see the deck comment.
#[case::npn_13g2("npn/npn13G2.gds.gz", "TOP",
    vec!["npn13G2.a", "npn13G2.a", "npn13G2L.a", "npn13G2L.b", "npn13G2V.a", "npn13G2V.b"],
    vec![])]
fn test_npn(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NPN, gds, topcell, &ignore), expected);
}

// --- Schottky diode (Sdiod) ---

const DECK_SDIOD: &str = "sdiod";

#[rstest]
#[case::sdiod("sdiod/Sdiod.gds.gz", "TOP",
    vec!["Sdiod.a", "Sdiod.b", "Sdiod.c", "Sdiod.d", "Sdiod.e"], vec![])]
fn test_sdiod(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SDIOD, gds, topcell, &ignore), expected);
}

// --- metal slits ---

const DECK_SLIT: &str = "slit";

#[rstest]
#[case("slit/Slt.a.gds.gz", "TOP", vec!["Slt.a"; 2], vec![])]
#[case("slit/Slt.b.gds.gz", "TOP", vec!["Slt.b"; 2], vec![])]
#[case("slit/Slt.c.gds.gz", "TOP", vec!["Slt.c"], vec![])]
// A TopMetal2 slit on the pad and a Metal1 slit under it fire (the pad region is the
// whole dfpad shape, on every layer); a slotted plate away from any pad and one under
// a dfpad without a Passiv opening (not a pad) stay clean.
#[case("slit/Slt.e.gds.gz", "TOP", vec!["Slt.e"; 2], vec![])]
#[case("slit/Slt.f.gds.gz", "TOP", vec!["Slt.f"], vec![])]
#[case("slit/Slt.h1.gds.gz", "TOP", vec!["Slt.h1"], vec![])]
#[case("slit/Slt.i.gds.gz", "TOP", vec!["Slt.i"], vec![])]
fn test_slit(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SLIT, gds, topcell, &ignore), expected);
}

// --- latch-up ---
//
// LU.c/LU.d (and LU.c1/LU.d1) enforce the identical "tie Activ within 6 µm of its Cont"
// constraint, so a tie-extension fixture trips both ids of the pair.

const DECK_LU: &str = "lu";

#[rstest]
#[case("lu/LU.a.gds.gz", "TOP", vec!["LU.a"], vec![])]
#[case("lu/LU.b.gds.gz", "TOP", vec!["LU.b"], vec![])]
#[case("lu/LU.c.gds.gz", "TOP", vec!["LU.c", "LU.d"], vec![])]
#[case("lu/LU.c1.gds.gz", "TOP", vec!["LU.c1", "LU.d1"], vec![])]
fn test_lu(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_LU, gds, topcell, &ignore), expected);
}

// --- antenna ---
//
// Ant.i is purely geometric (p-diode in the PWell fires; one correctly placed in an NWell
// stays clean).  Ant.b/e and Ant.d/f are net-aware (antenna_ratio over the connectivity
// engine): in the Ant.b fixture gate A's bare Metal1 antenna trips Ant.b, while gate B's
// identical antenna also reaches a diffusion diode, so the relaxed Ant.e limit keeps it
// clean.  (Ant.a/c/g/h still pending.)

const DECK_ANTENNA: &str = "antenna";

#[rstest]
#[case("antenna/Ant.i.gds.gz", "TOP", vec!["Ant.i"], vec![])]
#[case("antenna/Ant.b.gds.gz", "TOP", vec!["Ant.b"], vec![])]
// Per-level cumulative: G1's Metal1 antenna trips Ant.b even though it merges with G2 at
// Metal2 (a final-net ratio would dilute below the limit). Validated against KLayout.
#[case("antenna/Ant.merge.gds.gz", "TOP", vec!["Ant.b"], vec![])]
// Pre-metal antennas: a poly-over-field gate trips Ant.a, a large-contact gate trips Ant.c.
#[case("antenna/Ant.ac.gds.gz", "TOP", vec!["Ant.a", "Ant.c"], vec![])]
// An undersized gate-connected protection diode trips Ant.g.
#[case("antenna/Ant.g.gds.gz", "TOP", vec!["Ant.g"], vec![])]
// A bare n-diode in an NWell trips Ant.h; an identical one inside a text-tagged `isolbox`
// is exempt. Exercises text-label matching + region-interaction selectors.
#[case("antenna/Ant.h.gds.gz", "TOP", vec!["Ant.h"], vec![])]
fn test_antenna(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_ANTENNA, gds, topcell, &ignore), expected);
}

// --- sealring ---

const DECK_SEALRING: &str = "sealring";

#[rstest]
#[case::seal_l_clean("static/sealring/Seal.l.gds.gz", "TOP", vec![], vec![])]
#[case::seal_l_fail("static/sealring/Seal.l.fail.gds.gz", "TOP", vec!["Seal.l"; 2], vec![])]
#[case::seal_n_clean("static/sealring/Seal.n.gds.gz", "TOP", vec![], vec![])]
#[case::seal_n_fail("static/sealring/Seal.n.fail.gds.gz", "TOP", vec!["Seal.n"], vec!["Seal.e"])]
// A thin passiv ring 0.5 µm from a seal-Activ: Seal.e (frame < 4.20) + Seal.f (space < 1.00).
// Seal.n (ring-covering) is incidental to the synthetic ring and ignored here.
#[case::seal_ef("sealring/Seal.ef.gds.gz", "TOP", { let mut v = vec!["Seal.e"; 8]; v.push("Seal.f"); v }, vec!["Seal.n"])]
#[case::seal_b("sealring/Seal.b.gds.gz", "TOP", vec!["Seal.b"; 2], vec!["Seal.l"])]
// Two seal frames: Cont ring at 0.80 from the frame edge (< 1.30, fires ×4 — one per ring
// side, exact match with KLayout's 4 markers) and at 1.50 (clean).  Seal.l is synthetic
// two-frame collateral.
#[case::seal_d("sealring/Seal.d.gds.gz", "TOP", vec!["Seal.d"; 4], vec!["Seal.l"])]
fn test_sealring(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SEALRING, gds, topcell, &ignore), expected);
}

// --- MIM ---
//
// One layout exercising the MIM rules; MIM.b/f/h match KLayout exactly (MIM.c/d live in the
// BEOL sub-deck, MIM.a differs only in min_width edge granularity).

const DECK_MIM: &str = "mim";

#[rstest]
#[case("mim/MIM.gds.gz", "TOP",
    vec!["MIM.a", "MIM.a", "MIM.a", "MIM.a", "MIM.b", "MIM.d", "MIM.d", "MIM.d", "MIM.f", "MIM.h"],
    vec![])]
#[case("mim/MIM.gR.gds.gz", "TOP", vec!["MIM.gR"], vec![])]
fn test_mim(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_MIM, gds, topcell, &ignore), expected);
}

// --- forbidden ---

const DECK_FORBIDDEN: &str = "forbidden";

#[rstest]
#[case("forbidden.gds.gz", "TOP", vec!["forbidden"; 11], vec![])]
fn test_forbidden(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_FORBIDDEN, gds, topcell, &ignore), expected);
}

// --- offgrid ---
//
// Each fixture places one off-grid shape on a single layer; its shifted right edge
// has two off-grid vertices, so exactly that rule fires twice.  The layer set mirrors
// `gen/ihp_sg13g2/offgrid.rs` (and the IHP reference deck).

const DECK_OFFGRID: &str = "offgrid";

/// Primary layer of every offgrid rule; the rule id is `<layer>.offgrid`.
const OFFGRID_LAYERS: &[&str] = &[
    "Activ", "GatPoly", "PolyRes", "Cont", "nSD", "pSD", "SalBlock", "ThickGateOx",
    "NLDB", "PLDB", "NLDD", "PLDD", "NExt", "PExt", "NExtHV", "PExtHV", "EXTBlock",
    "NWell", "PWell", "nBuLay", "nBuLayCut", "isoNWell", "INLDPWL", "IC", "Substrate",
    "Metal1", "Metal2", "Metal3", "Metal4", "Metal5",
    "Via1", "Via2", "Via3", "Via4", "MIM", "Vmim",
    "TopVia1", "TopMetal1", "TopVia2", "TopMetal2", "Passiv", "AntMetal1",
    "BackMetal1", "BackPassiv", "AlCuStop", "DeepVia", "LBE",
    "BiWind", "PEmWind", "BasPoly", "EmWind", "EmWiHV", "EmPoly", "PEmPoly",
    "PBiWind", "DeepCo", "ColOpen", "ColWind", "CtrGat", "LDMOS",
    "FBE", "FGEtch", "FGImp", "FLM", "HafniumOx", "ThinFilmRes",
    "GraphGate", "MEMPAD", "MEMVia", "RFMEM", "SNSRing", "Sensor", "SNSArms",
    "SNSCMOSVia", "SNSBotVia", "SNSTopVia",
    "prBoundary", "Exchange0", "Exchange1", "Exchange2", "Exchange3", "Exchange4",
];

#[test]
fn test_offgrid() {
    for name in OFFGRID_LAYERS {
        let id = format!("{name}.offgrid");
        let gds = format!("offgrid/{id}.gds.gz");
        assert_eq!(
            drc(PDK_IHP, DECK_OFFGRID, &gds, "TOP", &[]),
            vec![id.as_str(); 2],
            "offgrid rule {id} should fire exactly twice on its own fixture",
        );
    }
}
