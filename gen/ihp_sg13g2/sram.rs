// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The SRAM deck: rules the main decks skip under the SRAM marker, at the values of
//! IHP's own bit cells.  Every fixture draws its shapes under the marker, one case at
//! the SRAM value (clean) and one 5 nm under it (violation).

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/sram";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    gat_c(pdk);
    nw_c(pdk);
    nw_d(pdk);
    cnt_c(pdk);
    tgo(pdk);
}

/// The SRAM marker over everything a fixture draws.
fn marker(pdk: &PdkConfig, x1: f64) -> GdsElement {
    let o = OFFSET;
    rect(layer(pdk, "SRAM"), o - 2.0, o - 2.0, o + x1, o + 4.0)
}

/// SRAM.Gat.c — GatPoly endcap over Activ 0.13 (public Gat.c 0.18): a gate ending
/// 0.13 past the Activ (clean), one ending 0.125 past it on top (violation).
fn gat_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        marker(pdk, 6.0),
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(gp, o + 0.3, o - 0.13, o + 0.43, o + 1.13),
        rect(activ, o + 3.0, o, o + 4.0, o + 1.0),
        rect(gp, o + 3.3, o - 0.13, o + 3.43, o + 1.125),
    ];
    write_gz(&format!("{DIR}/SRAM.Gat.c.gds.gz"), library("TOP", elems));
}

/// SRAM.NW.c — NWell enclosure of P+Activ 0.27 (public NW.c 0.31): 0.27 all round
/// (clean), 0.265 on one side (violation).
fn nw_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let mut elems = vec![marker(pdk, 8.0)];
    for (x, right) in [(0.0, 0.27), (4.0, 0.265)] {
        elems.push(rect(
            nw,
            o + x - 0.27,
            o - 0.27,
            o + x + 1.0 + right,
            o + 1.27,
        ));
        elems.push(rect(psd, o + x - 0.1, o - 0.1, o + x + 1.1, o + 1.1));
        elems.push(rect(activ, o + x, o, o + x + 1.0, o + 1.0));
    }
    write_gz(&format!("{DIR}/SRAM.NW.c.gds.gz"), library("TOP", elems));
}

/// SRAM.NW.d — NWell space to N+Activ outside it 0.27 (public NW.d 0.31): 0.27 (clean)
/// and 0.265 (violation).
fn nw_d(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let mut elems = vec![marker(pdk, 9.0)];
    for (x, gap) in [(0.0, 0.27), (4.5, 0.265)] {
        elems.push(rect(nw, o + x, o, o + x + 1.0, o + 1.0));
        elems.push(rect(
            activ,
            o + x + 1.0 + gap,
            o,
            o + x + 2.0 + gap,
            o + 1.0,
        ));
    }
    write_gz(&format!("{DIR}/SRAM.NW.d.gds.gz"), library("TOP", elems));
}

/// SRAM.Cnt.c — Activ enclosure of Cont 0.006 (Cnt.c.SRAM; public Cnt.c 0.07): 0.01
/// all round (clean), 0.005 on one side (violation).
fn cnt_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let cont = layer(pdk, "Cont");
    let m1 = layer(pdk, "Metal1");
    let o = OFFSET;
    let mut elems = vec![marker(pdk, 6.0)];
    for (x, right) in [(0.0, 0.01), (3.0, 0.005)] {
        elems.push(rect(
            activ,
            o + x - 0.01,
            o - 0.01,
            o + x + 0.16 + right,
            o + 0.17,
        ));
        elems.push(rect(cont, o + x, o, o + x + 0.16, o + 0.16));
        elems.push(rect(m1, o + x - 0.05, o - 0.05, o + x + 0.21, o + 0.21));
    }
    write_gz(&format!("{DIR}/SRAM.Cnt.c.gds.gz"), library("TOP", elems));
}

/// SRAM.TGO — no ThickGateOx under the marker: the thick-oxide rules have no SRAM
/// values.
fn tgo(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        marker(pdk, 4.0),
        rect(layer(pdk, "ThickGateOx"), o, o, o + 1.0, o + 1.0),
    ];
    write_gz(&format!("{DIR}/SRAM.TGO.gds.gz"), library("TOP", elems));
}
