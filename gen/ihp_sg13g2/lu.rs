// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/lu";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    lu_a(pdk);
    lu_b(pdk);
    lu_c(pdk);
    lu_c1(pdk);
}

/// LU.a — a PMOS source/drain (P+Activ in NWell) more than 20 µm from the NWell tie.  The
/// tie itself is compact (Activ ≈ Cont + 1.5 µm), so LU.c/LU.d stay clean.
fn lu_a(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let nwell = layer(pdk, "NWell");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(nwell, o, o, o + 80.0, o + 20.0),
        // N+ NWell tie (no pSD): Activ 4×4, Cont 1×1 centred → 1.5 µm extension.
        rect(activ, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(cont, o + 3.5, o + 9.5, o + 4.5, o + 10.5),
        // PMOS S/D (P+) 64 µm from the tie → LU.a.
        rect(activ, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
        rect(psd, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
    ];
    write_gz(&format!("{DIR}/LU.a.gds.gz"), library("TOP", elems));
}

/// LU.b — an NMOS source/drain (N+Activ in the substrate) more than 20 µm from the
/// substrate tie.  The tie is compact, so LU.c1/LU.d1 stay clean.
fn lu_b(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        // P+ substrate tie (pSD), compact.
        rect(activ, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(psd, o + 2.0, o + 8.0, o + 6.0, o + 12.0),
        rect(cont, o + 3.5, o + 9.5, o + 4.5, o + 10.5),
        // NMOS S/D (N+ = Activ with no pSD) 64 µm away → LU.b.
        rect(activ, o + 70.0, o + 8.0, o + 76.0, o + 12.0),
    ];
    write_gz(&format!("{DIR}/LU.b.gds.gz"), library("TOP", elems));
}

/// LU.c / LU.d — an NWell tie whose Activ stretches ~20 µm past its only contact (> 6 µm).
fn lu_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let nwell = layer(pdk, "NWell");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(nwell, o, o, o + 40.0, o + 20.0),
        // N+ NWell tie, Activ 23 µm long, Cont at the left end.
        rect(activ, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(cont, o + 3.0, o + 6.5, o + 5.0, o + 8.5),
    ];
    write_gz(&format!("{DIR}/LU.c.gds.gz"), library("TOP", elems));
}

/// LU.c1 / LU.d1 — a substrate tie whose Activ stretches ~20 µm past its only contact.
fn lu_c1(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let cont = layer(pdk, "Cont");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(psd, o + 2.0, o + 5.0, o + 25.0, o + 10.0),
        rect(cont, o + 3.0, o + 6.5, o + 5.0, o + 8.5),
    ];
    write_gz(&format!("{DIR}/LU.c1.gds.gz"), library("TOP", elems));
}
