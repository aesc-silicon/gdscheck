// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{layer, library, min_width_pattern, rect, space_pattern, strap, tap, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nbulay";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    nbl_a(pdk);
    nbl_b(pdk);
    nbl_c(pdk);
    nbl_c_same_net(pdk);
    nbl_d(pdk);
    nbl_e(pdk);
    nbl_f(pdk);
    nbl_d_same_net(pdk);
}

/// NBL.b — min. nBuLay space or notch 1.50: a pair at 1.20 (< 1.50 → fires) and at
/// 1.50 exactly (clean).  Neither pair draws NBL.c: both gaps close in nBuLayMerged
/// (the close radius 0.75 bridges gaps up to and including exactly 1.50).
fn nbl_b(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let o = OFFSET;
    let elems = vec![
        rect(nb, o, o, o + 2.0, o + 2.0),
        rect(nb, o + 3.2, o, o + 5.2, o + 2.0), // gap 1.20 → NBL.b
        rect(nb, o + 10.0, o, o + 12.0, o + 2.0),
        rect(nb, o + 13.5, o, o + 15.5, o + 2.0), // gap 1.50 → clean for NBL.b
    ];
    write_gz(&format!("{DIR}/NBL.b.gds.gz"), library("TOP", elems));
}

/// NBL.c — same-net merge (gaps < 1.50 µm) then different-net space 3.20 µm.  Three nBuLay
/// pairs: gap 1.00 (merged → clean), gap 2.00 (in [1.50, 3.20) → NBL.c), gap 4.00 (clean).
fn nbl_c(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let o = OFFSET;
    let pair = |y: f64, gap: f64| {
        vec![
            rect(nb, o, y, o + 2.0, y + 2.0),
            rect(nb, o + 2.0 + gap, y, o + 4.0 + gap, y + 2.0),
        ]
    };
    let mut elems = pair(o, 1.00); // merged → clean
    elems.extend(pair(o + 6.0, 2.00)); // NBL.c
    elems.extend(pair(o + 12.0, 4.00)); // clean
    write_gz(&format!("{DIR}/NBL.c.gds.gz"), library("TOP", elems));
}

/// NBL.c same-net regression — the nBuLay twin of `NW.b1.same_net`.  Two nBuLay pairs,
/// both with a 2.00 µm gap (inside the 1.50–3.20 µm band that nBuLayMerged's close,
/// radius 0.75, cannot bridge):
///
/// - left pair: bare buried layers, nothing tying them → different net → NBL.c is correct;
/// - right pair: each buried layer carries an NWell sinker on top (the NWell ∩ nBuLay
///   overlap that nmosi.d sizes), each sinker holds an N+Activ tap with a Cont, and one
///   Metal1 strap spans both — so the two buried layers are electrically one net.
///
/// Kept clear of the neighbouring rules on purpose: the sinkers are 3.60 µm apart (NW.b1
/// needs 1.80), each is fully inside its own nBuLay and 2.80 µm from the other (NBL.d
/// needs 2.20), and each tap sits inside a sinker so it never becomes IsoPWellAct.
fn nbl_c_same_net(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let o = OFFSET;

    // One 2.00 µm-gap nBuLay pair at x-origin `x`.  `tied` adds the sinker/tap/Cont stack
    // in each buried layer plus the Metal1 strap that shorts the two.
    let pair = |x: f64, tied: bool| {
        let mut e = vec![
            rect(nb, x, o, x + 3.0, o + 3.0),
            rect(nb, x, o + 5.0, x + 3.0, o + 8.0), // gap 2.00 µm
        ];
        if tied {
            let centres = [(x + 1.5, o + 1.5), (x + 1.5, o + 6.5)];
            for &(cx, cy) in &centres {
                // Sinker 1.40 across (nmosi.d needs 0.62), tap enclosed by it by 0.40.
                e.push(rect(nw, cx - 0.7, cy - 0.7, cx + 0.7, cy + 0.7));
                e.extend(tap(activ, cont, cx, cy, 0.60));
            }
            e.push(strap(metal1, &centres));
        }
        e
    };

    let mut elems = pair(o, false);
    elems.extend(pair(o + 8.0, true)); // 5.00 µm clear of the left pair (NBL.c needs 3.20)
    write_gz(&format!("{DIR}/NBL.c.same_net.gds.gz"), library("TOP", elems));
}

/// NBL.d same-net regression — the two-layer twin of `NBL.c.same_net`.  A bare row (y+0)
/// and a strapped row (y+8), each an nBuLay with an NWell 2.00 µm away (NBL.d is 2.20).
///
/// In the strapped row the buried layer carries an NWell sinker, a tap and a Cont, the
/// partner NWell carries its own tap and Cont, and one Metal1 plate joins the two — so
/// partner and buried layer are one net and the rule must not fire.  The bare row is the
/// same geometry with nothing tying it, and must fire once.
///
/// Kept clear of its neighbours on purpose: the two nBuLay boxes are 5.00 µm apart (NBL.c
/// needs 3.20), sinker to partner NWell is 2.80 µm (NW.b1 needs 1.80), and each tap sits
/// inside a well so it never becomes IsoPWellAct.
fn nbl_d_same_net(pdk: &PdkConfig) {
    let nb = layer(pdk, "nBuLay");
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let o = OFFSET;

    let mut elems = Vec::new();
    for tied in [false, true] {
        let y = if tied { o + 8.0 } else { o };
        elems.push(rect(nb, o, y, o + 3.0, y + 3.0));
        elems.push(rect(nw, o + 5.0, y + 0.5, o + 6.0, y + 1.5)); // gap 2.00 µm
        if tied {
            // Sinker 1.40 across (nmosi.d needs 0.62); its tap is enclosed by 0.40, the
            // partner well's by 0.30 (NW.e needs 0.24).
            elems.push(rect(nw, o + 0.8, y + 0.8, o + 2.2, y + 2.2));
            elems.extend(tap(activ, cont, o + 1.5, y + 1.5, 0.60));
            elems.extend(tap(activ, cont, o + 5.5, y + 1.0, 0.40));
            elems.push(strap(metal1, &[(o + 1.5, y + 1.5), (o + 5.5, y + 1.0)]));
        }
    }
    write_gz(&format!("{DIR}/NBL.d.same_net.gds.gz"), library("TOP", elems));
}

/// NBL.a — min. nBuLay width 1.00 µm.
fn nbl_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay");
    let elems = min_width_pattern(l, 1.0, 1.0, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.a.gds.gz"), library("TOP", elems));
}

/// NBL.d — min. nBuLay to NWell space 2.20 µm.
fn nbl_d(pdk: &PdkConfig) {
    let elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "NWell"), 3.0, 2.20, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.d.gds.gz"), library("TOP", elems));
}

/// NBL.e — min. nBuLay to N+Activ (Activ without pSD → NActiv) space 1.00 µm.
fn nbl_e(pdk: &PdkConfig) {
    let elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "Activ"), 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.e.gds.gz"), library("TOP", elems));
}

/// NBL.f — min. nBuLay to P+Activ (Activ ∩ pSD → PsdActiv) space 0.50 µm.  pSD covers the
/// whole pattern so every Activ neighbour reads as P+Activ rather than N+Activ.
fn nbl_f(pdk: &PdkConfig) {
    let mut elems = space_pattern(layer(pdk, "nBuLay"), layer(pdk, "Activ"), 2.0, 0.50, OFFSET, SPACE_DELTA);
    let o = OFFSET;
    elems.push(rect(layer(pdk, "pSD"), o - 5.0, o - 5.0, o + 10.0, o + 10.0));
    write_gz(&format!("{DIR}/NBL.f.gds.gz"), library("TOP", elems));
}
