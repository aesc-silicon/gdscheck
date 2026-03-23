// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, offgrid_pattern, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/offgrid";

/// Off-grid shift: 0.003 µm is not a multiple of the 0.005 µm grid.
const OFF: f64 = 0.003;

/// Primary drawn layer of each offgrid rule (rule id is `<layer>.offgrid`).  Each
/// fixture places one off-grid shape on this layer, so exactly that rule fires —
/// twice, since the shifted right edge has two off-grid vertices.
const LAYERS: &[&str] = &[
    // Front-end: poly / active / implants
    "Activ", "GatPoly", "PolyRes", "Cont", "nSD", "pSD", "SalBlock", "ThickGateOx",
    "NLDB", "PLDB", "NLDD", "PLDD", "NExt", "PExt", "NExtHV", "PExtHV", "EXTBlock",
    // Wells / buried layers / substrate
    "NWell", "PWell", "nBuLay", "nBuLayCut", "isoNWell", "INLDPWL", "IC", "Substrate",
    // Metal stack
    "Metal1", "Metal2", "Metal3", "Metal4", "Metal5",
    "Via1", "Via2", "Via3", "Via4", "MIM", "Vmim",
    // Top metals / passivation
    "TopVia1", "TopMetal1", "TopVia2", "TopMetal2", "Passiv", "AntMetal1",
    // Backside
    "BackMetal1", "BackPassiv", "AlCuStop", "DeepVia", "LBE",
    // Bipolar / HV devices
    "BiWind", "PEmWind", "BasPoly", "EmWind", "EmWiHV", "EmPoly", "PEmPoly",
    "PBiWind", "DeepCo", "ColOpen", "ColWind", "CtrGat", "LDMOS",
    // Flash / special process
    "FBE", "FGEtch", "FGImp", "FLM", "HafniumOx", "ThinFilmRes",
    // Photonics / SNS / MEMS
    "GraphGate", "MEMPAD", "MEMVia", "RFMEM", "SNSRing", "Sensor", "SNSArms",
    "SNSCMOSVia", "SNSBotVia", "SNSTopVia",
    // Boundaries / exchange
    "prBoundary", "Exchange0", "Exchange1", "Exchange2", "Exchange3", "Exchange4",
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    for name in LAYERS {
        let l = layer(pdk, name);
        let elems = offgrid_pattern(l, 1.0, 5.0, OFFSET, OFF);
        write_gz(&format!("{DIR}/{name}.offgrid.gds.gz"), library("TOP", elems));
    }
}
