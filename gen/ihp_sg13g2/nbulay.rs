// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::pwellblock::{P, ring, space2_kit, width_kit};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    layer, library, min_width_pattern, poly, rect, space_pattern, strap, tap, write_gz,
};
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

    hardening(pdk);
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
    write_gz(
        &format!("{DIR}/NBL.c.same_net.gds.gz"),
        library("TOP", elems),
    );
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
    write_gz(
        &format!("{DIR}/NBL.d.same_net.gds.gz"),
        library("TOP", elems),
    );
}

/// NBL.a — min. nBuLay width 1.00 µm.
fn nbl_a(pdk: &PdkConfig) {
    let l = layer(pdk, "nBuLay");
    let elems = min_width_pattern(l, 1.0, 1.0, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NBL.a.gds.gz"), library("TOP", elems));
}

/// NBL.d — min. nBuLay to NWell space 2.20 µm.
fn nbl_d(pdk: &PdkConfig) {
    let elems = space_pattern(
        layer(pdk, "nBuLay"),
        layer(pdk, "NWell"),
        3.0,
        2.20,
        OFFSET,
        SPACE_DELTA,
    );
    write_gz(&format!("{DIR}/NBL.d.gds.gz"), library("TOP", elems));
}

/// NBL.e — min. nBuLay to N+Activ (Activ without pSD → NActiv) space 1.00 µm.
fn nbl_e(pdk: &PdkConfig) {
    let elems = space_pattern(
        layer(pdk, "nBuLay"),
        layer(pdk, "Activ"),
        2.0,
        1.00,
        OFFSET,
        SPACE_DELTA,
    );
    write_gz(&format!("{DIR}/NBL.e.gds.gz"), library("TOP", elems));
}

/// NBL.f — min. nBuLay to P+Activ (Activ ∩ pSD → PsdActiv) space 0.50 µm.  pSD covers the
/// whole pattern so every Activ neighbour reads as P+Activ rather than N+Activ.
fn nbl_f(pdk: &PdkConfig) {
    let mut elems = space_pattern(
        layer(pdk, "nBuLay"),
        layer(pdk, "Activ"),
        2.0,
        0.50,
        OFFSET,
        SPACE_DELTA,
    );
    let o = OFFSET;
    elems.push(rect(
        layer(pdk, "pSD"),
        o - 5.0,
        o - 5.0,
        o + 10.0,
        o + 10.0,
    ));
    write_gz(&format!("{DIR}/NBL.f.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the block decks: section 5.2 (PWell:block, PWB.a-PWB.f1), 5.3
// (nBuLay, NBL.a-NBL.f), 5.4 (nBuLay:block, NBLB.a-NBLB.d), 5.12 (EXTBlock, EXTB.a-c),
// 5.13 (SalBlock, Sal.a-e) and 5.15 (ContBar, CntB.a-CntB.j) of the SG13G2 layout rules,
// with section 4.2's derived layers (N+Activ, P+Activ, PWell, the generated nBuLay).
// Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//
// The width rules of the five block layers share a kit (`width_kit`), the two-layer
// space rules another (`space2_kit`); their plain space rules are read on the engine's
// patterns (`gen/engine/space.rs`), and the conditions of each rule - which Activ is N+,
// what "in PWell" or "unrelated" means, the generated nBuLay - are drawn rule by rule
// below.

fn nbulay(p: &P) {
    let deck = "nbulay";
    let n = p.nbl;
    width_kit(p, deck, "NBL.a", n, 1.0, 3.2);
    // NBL.b - space or notch (same net) 1.50: the notch half is in h2.
    // NBL.c - PWell width between nBuLay regions (different net) 3.20.  Bare regions are
    // different nets; no notch layout: a notch is one region.

    // NBL.b.h6 - a 3 × 3 square in a ring's hole 1.495 from the hole's right and top
    // walls (1.5 from the others): two walls under the value, two markers.
    let mut e = ring(n, 2.0, 2.0, 10.195, 10.195, 3.1, 3.1, 9.095, 9.095);
    e.push(rect(n, 4.6, 4.6, 7.6, 7.6));
    p.write(deck, "NBL.b.h6", e);

    // NBL.c.h8 - the boundary between NBL.b and NBL.c for bare regions.  Pairs 3 × 3 at
    // 1.495 (NBL.b), at 1.5 and 1.505 (NBL.c: unconnected regions 1.5 apart are on
    // different nets and the PWell between them is under 3.2), at 3.195 (NBL.c) and 3.2
    // (clean).
    let mut e = vec![];
    for (k, g) in [1.495, 1.5, 1.505, 3.195, 3.2].iter().enumerate() {
        let y = 2.0 + k as f64 * 7.0;
        e.push(rect(n, 2.0, y, 5.0, y + 3.0));
        e.push(rect(n, 5.0 + g, y, 8.0 + g, y + 3.0));
    }
    p.write(deck, "NBL.c.h8", e);

    // NBL.c.h6 - the PWell between.  Section 4.2: PWell = NOT (NWell OR PWell:block).
    // Two nBuLay 2.0 apart with PWell:block over the whole gap (no PWell between them:
    // clean); two 2.0 apart with a 0.7 block strip in the middle (0.65 of PWell either
    // side: fires).
    let pair = |x: f64, y: f64, gap: f64| {
        vec![
            rect(n, x, y, x + 3.0, y + 3.0),
            rect(n, x + 3.0 + gap, y, x + 6.0 + gap, y + 3.0),
        ]
    };
    let mut e = pair(2.0, 2.0, 2.0);
    e.push(rect(p.pwb, 5.0, 1.5, 7.0, 5.5));
    e.extend(pair(14.0, 2.0, 2.0));
    e.push(rect(p.pwb, 17.65, 1.5, 18.35, 5.5));
    p.write(deck, "NBL.c.h6", e);

    // NBL.c.h9 - figure 5.3's c: a block overlapping the left nBuLay and reaching to
    // 3.195 from the right one, which is 5.0 away (the PWell between the regions is
    // 3.195: fires); the same reaching to 3.2 (clean); a block reaching to 3.195 from an
    // nBuLay but touching no nBuLay (not "between nBuLay regions": clean).
    let mut e = pair(2.0, 2.0, 5.0);
    e.push(rect(p.pwb, 4.0, 1.5, 10.0 - 3.195, 5.5));
    e.extend(pair(20.0, 2.0, 5.0));
    e.push(rect(p.pwb, 22.0, 1.5, 28.0 - 3.2, 5.5));
    e.push(rect(n, 2.0, 12.0, 5.0, 15.0));
    e.push(rect(p.pwb, 5.0 + 3.195, 11.5, 10.0, 15.5));
    p.write(deck, "NBL.c.h9", e);

    // NBL.c.h7 - generated nBuLay (note 1, section 4.2): a well 3.0 µm and wider carries
    // nBuLay sized by 1.0 µm a side, which IHP's decks read as the well inset by 1.0.
    // A 6 × 6 well and a drawn nBuLay 2.195 apart (the generated nBuLay is 3.195 from the
    // drawn one: NBL.c, and the well itself 2.195: NBL.d); two 6 × 6 wells 1.15 apart
    // (generated regions 3.15 apart: NBL.c); two 6 × 6 wells 1.2 apart (3.2: clean); a
    // 2.5 × 6 well (no generated nBuLay) 2.195 from a drawn nBuLay (NBL.d only).
    let e = vec![
        rect(p.nw, 2.0, 2.0, 8.0, 8.0),
        rect(n, 10.195, 2.0, 13.195, 5.0),
        rect(p.nw, 2.0, 12.0, 8.0, 18.0),
        rect(p.nw, 9.15, 12.0, 15.15, 18.0),
        rect(p.nw, 20.0, 12.0, 26.0, 18.0),
        rect(p.nw, 27.2, 12.0, 33.2, 18.0),
        rect(p.nw, 20.0, 2.0, 22.5, 8.0),
        rect(n, 24.695, 2.0, 27.695, 5.0),
    ];
    p.write(deck, "NBL.c.h7", e);

    // NBL.d - PWell width between nBuLay and NWell (different net) 2.20.  The kit with a
    // 2 × 2 bare well as the fixed box.
    let nwell = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.nw, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "NBL.d",
        2.2,
        &|pts| vec![poly(n, pts)],
        3.0,
        &nwell,
        2.0,
        false,
    );

    // NBL.d.h5 - the relation and the PWell between.  A well overlapping the nBuLay (a
    // sinker: one net, clean) and one abutting it (connected, clean); a well 2.0 from an
    // nBuLay with PWell:block over the whole gap (clean) and with a 0.7 strip in the
    // middle (fires).
    let e = vec![
        rect(n, 2.0, 2.0, 5.0, 5.0),
        rect(p.nw, 4.0, 3.0, 6.0, 4.0),
        rect(n, 9.0, 2.0, 12.0, 5.0),
        rect(p.nw, 12.0, 3.0, 14.0, 4.0),
        rect(n, 2.0, 9.0, 5.0, 12.0),
        rect(p.nw, 7.0, 10.0, 9.0, 11.0),
        rect(p.pwb, 5.0, 8.5, 7.0, 12.5),
        rect(n, 13.0, 9.0, 16.0, 12.0),
        rect(p.nw, 18.0, 10.0, 20.0, 11.0),
        rect(p.pwb, 16.65, 8.5, 17.35, 12.5),
    ];
    p.write(deck, "NBL.d.h5", e);

    // NBL.d.h7 - figure 5.3's d: a block adjoining the well, its far edge 2.195 from the
    // nBuLay which is 4.0 from the well (fires), and a block adjoining the nBuLay
    // reaching to 2.195 from the well (fires).
    let e = vec![
        rect(n, 2.0, 2.0, 5.0, 5.0),
        rect(p.nw, 9.0, 3.0, 11.0, 4.0),
        rect(p.pwb, 5.0 + 2.195, 1.5, 9.5, 5.5),
        rect(n, 15.0, 2.0, 18.0, 5.0),
        rect(p.nw, 22.0, 3.0, 24.0, 4.0),
        rect(p.pwb, 17.5, 1.5, 22.0 - 2.195, 5.5),
    ];
    p.write(deck, "NBL.d.h7", e);

    // NBL.d.h6 - generated nBuLay.  A 6 × 6 well and a bare 1 × 1 well 1.15 apart (the
    // generated nBuLay, the well inset by 1.0, is 2.15 from the small well: NBL.d); the
    // same 1.5 apart (2.5: clean under the inset, 0.5 under the outset that grows the
    // well); a 2.5-wide well (no generated nBuLay) 1.15 from a 1 × 1 well (clean).
    let e = vec![
        rect(p.nw, 2.0, 2.0, 8.0, 8.0),
        rect(p.nw, 9.15, 4.5, 10.15, 5.5),
        rect(p.nw, 14.0, 2.0, 20.0, 8.0),
        rect(p.nw, 21.5, 4.5, 22.5, 5.5),
        rect(p.nw, 26.0, 2.0, 28.5, 8.0),
        rect(p.nw, 29.65, 4.5, 30.65, 5.5),
    ];
    p.write(deck, "NBL.d.h6", e);

    // NBL.e - nBuLay space to unrelated N+Activ 1.00.  The kit with a bare 0.5 Activ
    // square (N+ by default) as the fixed box; the abutting pair is a space of nothing.
    let act = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.activ, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "NBL.e",
        1.0,
        &|pts| vec![poly(n, pts)],
        3.0,
        &act,
        0.5,
        true,
    );

    // NBL.e.h5 - what is unrelated N+Activ.  0.5 Activ squares 0.995 right of a 4 × 4
    // nBuLay at y = 2: plain (fires), under drawn nSD (fires), under nSD:block (neither
    // N+ nor P+: clean), under pSD (P+, NBL.f's, and 0.995 clears its 0.5: clean).  At
    // y = 10: a well sinker overlapping the nBuLay's right edge with an N+ tap in it 0.5
    // outside the nBuLay (the tap is the nBuLay's own net: clean); an N+ tap in PWell
    // 0.5 from the nBuLay, strapped by Metal1 to a tap in the nBuLay's sinker (one net:
    // clean); an Activ crossing the nBuLay edge, its outside part 0 away (fires).
    let mut e = vec![];
    let blk = |x: f64, y: f64| rect(n, x, y, x + 4.0, y + 4.0);
    let a = |x: f64, y: f64| rect(p.activ, x + 4.995, y + 1.75, x + 5.495, y + 2.25);
    for (k, imp) in [
        (0, None),
        (1, Some(p.nsd)),
        (2, Some(p.nsdb)),
        (3, Some(p.psd)),
    ] {
        let x = 2.0 + k as f64 * 8.0;
        e.push(blk(x, 2.0));
        e.push(a(x, 2.0));
        if let Some(l) = imp {
            e.push(rect(l, x + 4.9, 3.65, x + 5.6, 4.35));
        }
    }
    e.push(blk(2.0, 10.0));
    e.push(rect(p.nw, 5.0, 11.0, 8.0, 13.0));
    e.extend(tap(p.activ, p.cont, 6.75, 12.0, 0.5));
    e.push(strap(p.m1, &[(6.75, 12.0)]));
    e.push(blk(11.0, 10.0));
    e.push(rect(p.nw, 12.0, 11.0, 14.0, 13.0));
    e.extend(tap(p.activ, p.cont, 13.0, 12.0, 0.5));
    e.extend(tap(p.activ, p.cont, 15.75, 12.0, 0.5));
    e.push(strap(p.m1, &[(13.0, 12.0), (15.75, 12.0)]));
    e.push(blk(19.0, 10.0));
    e.push(rect(p.activ, 22.75, 11.75, 23.25, 12.25));
    p.write(deck, "NBL.e.h5", e);

    // NBL.f - nBuLay space to unrelated P+Activ 0.50.  The kit with a 0.5 P+Activ
    // square as the fixed box.
    let pact = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(p.activ, x0, y0, x1, y1),
            rect(p.psd, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    };
    space2_kit(
        p,
        deck,
        "NBL.f",
        0.5,
        &|pts| vec![poly(n, pts)],
        3.0,
        &pact,
        0.5,
        true,
    );

    // NBL.f.h5 - what is unrelated P+Activ.  At y = 2: a P+Activ in a well sinker that
    // overlaps the nBuLay, 0.3 outside the nBuLay's edge (a PMOS's diffusion in the
    // nBuLay's own well: related, clean); a P+ tie in PWell 0.3 from the nBuLay, strapped
    // by Metal1 to an N+ tap in the nBuLay's sinker (one net: clean); a 0.5 × 1.0 Activ
    // 0.495 from the nBuLay with pSD over its near half (the P+ half fires f, the bare
    // far half is N+ 0.995 away: NBL.e); a P+Activ crossing the nBuLay edge (fires).
    let mut e = vec![
        blk(2.0, 2.0),
        rect(p.nw, 5.0, 3.0, 8.0, 5.0),
        rect(p.activ, 6.3, 3.75, 6.8, 4.25),
        rect(p.psd, 6.2, 3.65, 6.9, 4.35),
        blk(11.0, 2.0),
        rect(p.nw, 12.0, 3.0, 14.0, 5.0),
    ];
    e.extend(tap(p.activ, p.cont, 13.0, 4.0, 0.5));
    e.extend(tap(p.activ, p.cont, 15.55, 4.0, 0.5));
    e.push(rect(p.psd, 15.2, 3.65, 15.9, 4.35));
    e.push(strap(p.m1, &[(13.0, 4.0), (15.55, 4.0)]));
    e.push(blk(19.0, 2.0));
    e.push(rect(p.activ, 23.495, 3.75, 24.495, 4.25));
    e.push(rect(p.psd, 23.4, 3.65, 23.995, 4.35));
    e.push(blk(27.0, 2.0));
    e.push(rect(p.activ, 30.75, 3.75, 31.25, 4.25));
    e.push(rect(p.psd, 30.65, 3.65, 31.35, 4.35));
    p.write(deck, "NBL.f.h5", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/nbulay")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    nbulay(&p);
}
