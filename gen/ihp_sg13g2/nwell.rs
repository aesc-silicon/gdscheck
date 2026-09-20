// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_bl, chamfered_tr, diamond, flat_array, layer, library, min_width_pattern,
    mixed_notch_pattern, notch_pattern, poly, rect, ref_array, strap, strip45, tap, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/nwell";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    nw_a(pdk);
    nw_b(pdk);
    nw_b1(pdk);
    nw_b1_same_net(pdk);
    nw_c(pdk);
    nw_c1(pdk);
    nw_d(pdk);
    nw_d1(pdk);
    nw_e(pdk);
    nw_e1(pdk);
    nw_f(pdk);
    nw_f1(pdk);
    nw_dig(pdk);
    hardening(pdk);
}

/// P+Activ footprint (Activ ∩ pSD) at `(x, y)`.
fn pact(activ: (i16, i16), psd: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![
        rect(activ, x, y, x + w, y + h),
        rect(psd, x, y, x + w, y + h),
    ]
}

/// N+Activ footprint (Activ ∩ nSD) at `(x, y)`.
fn nact(activ: (i16, i16), nsd: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![
        rect(activ, x, y, x + w, y + h),
        rect(nsd, x, y, x + w, y + h),
    ]
}

fn nw_a(pdk: &PdkConfig) {
    let l = layer(pdk, "NWell");
    let elems = min_width_pattern(l, 0.62, 0.62, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/NW.a.gds.gz"), library("TOP", elems));
}

/// NW.b/b1 — same-net merge then different-net space.  Three NWell pairs: gap 0.50 µm
/// (< 0.62) merges (same net) → clean; gap 1.00 µm (in [0.62, 1.80)) → NW.b1; gap 2.00 µm
/// (≥ 1.80) → clean.  Exercises the `close` (size-merge) op behind NWellMerged.
/// NW.b — min. NWell space or notch 0.62: a pair at 0.50 (< 0.62 → fires) and at
/// 0.62 exactly (clean).  Neither pair draws NW.b1: both gaps close in NWellMerged
/// (the close radius 0.31 bridges gaps up to and including exactly 0.62).
fn nw_b(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let elems = vec![
        rect(nw, o, o, o + 2.0, o + 2.0),
        rect(nw, o + 2.5, o, o + 4.5, o + 2.0), // gap 0.50 → NW.b
        rect(nw, o + 10.0, o, o + 12.0, o + 2.0),
        rect(nw, o + 12.62, o, o + 14.62, o + 2.0), // gap 0.62 → clean for NW.b
    ];
    write_gz(&format!("{DIR}/NW.b.gds.gz"), library("TOP", elems));
}

fn nw_b1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let o = OFFSET;
    let pair = |y: f64, gap: f64| {
        vec![
            rect(nw, o, y, o + 2.0, y + 2.0),
            rect(nw, o + 2.0 + gap, y, o + 4.0 + gap, y + 2.0),
        ]
    };
    let mut elems = pair(o, 0.50); // merged → clean
    elems.extend(pair(o + 5.0, 1.00)); // NW.b1
    elems.extend(pair(o + 10.0, 2.00)); // clean
    write_gz(&format!("{DIR}/NW.b1.gds.gz"), library("TOP", elems));
}

/// NW.b1 same-net regression — the GitHub report behind the documented net-blindness of
/// this rule.  Two identical NWell pairs, both with a 1.00 µm gap (inside the 0.62–1.80 µm
/// band the `close` merge cannot bridge):
///
/// - left pair: bare wells, nothing tying them → genuinely different net → NW.b1 is correct;
/// - right pair: each well carries an N+Activ tie with a Cont, and one Metal1 strap spans
///   both conts, so the two wells are electrically *one* net → NW.b1 must not fire.
///
/// gdscheck currently reports both, because `min_space` is geometric and NWell is absent
/// from the connectivity model — see `docs/source/pdks/ihp-sg13g2.rst`.
fn nw_b1_same_net(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let cont = layer(pdk, "Cont");
    let metal1 = layer(pdk, "Metal1");
    let o = OFFSET;

    // One 1.00 µm-gap NWell pair at x-origin `x`.  `tied` adds the well ties (plain Activ,
    // no pSD → N+) and the Metal1 strap that shorts the two wells.
    let pair = |x: f64, tied: bool| {
        let mut e = vec![
            rect(nw, x, o, x + 1.0, o + 1.0),
            rect(nw, x, o + 2.0, x + 1.0, o + 3.0), // gap 1.00 µm
        ];
        if tied {
            // Tie 0.40 across, so the NWell encloses it by 0.30 (NW.e needs 0.24).
            let centres = [(x + 0.5, o + 0.5), (x + 0.5, o + 2.5)];
            for &(cx, cy) in &centres {
                e.extend(tap(activ, cont, cx, cy, 0.40));
            }
            e.push(strap(metal1, &centres));
        }
        e
    };

    let mut elems = pair(o, false);
    elems.extend(pair(o + 4.0, true)); // 3.00 µm clear of the left pair
    write_gz(
        &format!("{DIR}/NW.b1.same_net.gds.gz"),
        library("TOP", elems),
    );
}

/// NW.c — min. NWell enclosure of P+Activ (PMOS S/D) not in ThickGateOx, 0.31 µm.  One
/// P+Activ enclosed by 0.31 (clean); one with a 0.30 left margin (violation).
fn nw_c(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o - 0.31, o - 0.31, o + 0.81, o + 0.81)];
    elems.extend(pact(activ, psd, o, o, 0.5, 0.5)); // 0.31 all round → clean
    elems.push(rect(nw, o + 3.0 - 0.30, o - 0.31, o + 3.81, o + 0.81));
    elems.extend(pact(activ, psd, o + 3.0, o, 0.5, 0.5)); // 0.30 left → violation
    write_gz(&format!("{DIR}/NW.c.gds.gz"), library("TOP", elems));
}

/// NW.c1 — min. NWell enclosure of P+Activ inside ThickGateOx, 0.62 µm.
fn nw_c1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![
        rect(nw, o - 0.62, o - 0.62, o + 1.12, o + 1.12),
        rect(tgo, o - 0.1, o - 0.1, o + 0.6, o + 0.6),
    ];
    elems.extend(pact(activ, psd, o, o, 0.5, 0.5)); // 0.62 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.61, o - 0.62, o + 4.0 + 1.12, o + 1.12));
    elems.push(rect(tgo, o + 4.0 - 0.1, o - 0.1, o + 4.6, o + 0.6));
    elems.extend(pact(activ, psd, o + 4.0, o, 0.5, 0.5)); // 0.61 left → violation
    write_gz(&format!("{DIR}/NW.c1.gds.gz"), library("TOP", elems));
}

/// NW.d — min. NWell space to external N+Activ not in ThickGateOx, 0.31 µm.  N+ is the
/// full derived implant: drawn nSD fires, but so does plain undoped Activ (N+ by
/// default); Activ under nSD:block (without drawn nSD) is not N+ and stays clean.
fn nw_d(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    elems.extend(nact(activ, nsd, o + 1.0 + 0.31, o, 0.5, 0.5)); // gap 0.31 → clean
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    elems.extend(nact(activ, nsd, o + 5.0 + 0.30, o, 0.5, 0.5)); // gap 0.30 → violation
    // Plain Activ (no implant drawn — N+ by default) at 0.30 → violation.
    elems.push(rect(nw, o + 8.0, o, o + 9.0, o + 1.0));
    elems.push(rect(activ, o + 9.0 + 0.30, o, o + 9.8, o + 0.5));
    // Activ under nSD:block, no drawn nSD (default implant suppressed) at 0.30 → clean.
    elems.push(rect(nw, o + 12.0, o, o + 13.0, o + 1.0));
    elems.push(rect(activ, o + 13.0 + 0.30, o, o + 13.8, o + 0.5));
    elems.push(rect(
        layer(pdk, "nSD.block"),
        o + 13.0 + 0.30,
        o,
        o + 13.8,
        o + 0.5,
    ));
    write_gz(&format!("{DIR}/NW.d.gds.gz"), library("TOP", elems));
}

/// NW.d1 — min. NWell space to external N+Activ inside ThickGateOx, 0.62 µm.
fn nw_d1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + 0.62;
    elems.extend(nact(activ, nsd, x1, o, 0.5, 0.5)); // gap 0.62 → clean
    elems.push(rect(tgo, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + 0.61;
    elems.extend(nact(activ, nsd, x2, o, 0.5, 0.5)); // gap 0.61 → violation
    elems.push(rect(tgo, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    write_gz(&format!("{DIR}/NW.d1.gds.gz"), library("TOP", elems));
}

/// NW.e — min. NWell enclosure of the NWell tie (N+Activ in NWell), not in TGO, 0.24 µm.  The
/// tie is plain Activ (no pSD → N+).  One enclosed by 0.30 (clean); one with 0.20 left → NW.e.
fn nw_e(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o - 0.30, o - 0.30, o + 0.80, o + 0.80)];
    elems.push(rect(activ, o, o, o + 0.5, o + 0.5)); // 0.30 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.20, o - 0.30, o + 4.0 + 0.80, o + 0.80));
    elems.push(rect(activ, o + 4.0, o, o + 4.5, o + 0.5)); // 0.20 left → NW.e
    // A tie CROSSING the NWell boundary: not "surrounded entirely by NWell", so NW.e
    // does not apply (skip_clipped) even though its clipped-in piece has small margins.
    elems.push(rect(nw, o + 8.0, o - 0.30, o + 9.1, o + 0.80));
    elems.push(rect(activ, o + 8.9, o, o + 10.0, o + 0.5)); // extends 0.9 past NWell
    write_gz(&format!("{DIR}/NW.e.gds.gz"), library("TOP", elems));
}

/// NW.e1 — NWell enclosure of the NWell tie inside ThickGateOx, 0.62 µm.
fn nw_e1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![
        rect(nw, o - 0.70, o - 0.70, o + 1.20, o + 1.20),
        rect(tgo, o - 0.1, o - 0.1, o + 0.6, o + 0.6),
    ];
    elems.push(rect(activ, o, o, o + 0.5, o + 0.5)); // 0.70 all round → clean
    elems.push(rect(nw, o + 4.0 - 0.50, o - 0.70, o + 4.0 + 1.20, o + 1.20));
    elems.push(rect(tgo, o + 4.0 - 0.1, o - 0.1, o + 4.6, o + 0.6));
    elems.push(rect(activ, o + 4.0, o, o + 4.5, o + 0.5)); // 0.50 left → NW.e1
    write_gz(&format!("{DIR}/NW.e1.gds.gz"), library("TOP", elems));
}

/// Chapter 8 DigiBnd relaxation of the HV rules.  Inside one DigiBnd region:
/// - TGO N+Activ at 0.30 from NWell → NW.d1.dig (< 0.31); at 0.45 → clean (the strict
///   0.62 does not apply in the digital split);
/// - TGO P+Activ in NWell with 0.25 worst margin → NW.c1.dig; 0.45 margins → clean;
/// - TGO NWell tie with 0.20 worst margin → NW.e1.dig; 0.30 margins → clean.
///
/// Outside the DigiBnd, a TGO N+Activ at 0.45 still draws the strict NW.d1.
fn nw_dig(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(
        layer(pdk, "DigiBnd"),
        o - 2.0,
        o - 2.0,
        o + 25.0,
        o + 3.0,
    )];
    // d1.dig fires (gap 0.30) / relaxed-clean (gap 0.45).
    let d1 = |x: f64, gap: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + 1.0, o + 1.0));
        e.extend(nact(activ, nsd, x + 1.0 + gap, o, 0.5, 0.5));
        e.push(rect(
            tgo,
            x + 1.0 + gap - 0.1,
            o - 0.1,
            x + 1.0 + gap + 0.6,
            o + 0.6,
        ));
    };
    d1(o, 0.30, &mut elems);
    d1(o + 4.0, 0.45, &mut elems);
    // c1.dig fires (left margin 0.25, rest 0.45) / relaxed-clean (0.45 all round).
    let c1 = |x: f64, left: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + left + 0.5 + 0.45, o + 1.4));
        e.extend(pact(activ, psd, x + left, o + 0.45, 0.5, 0.5));
        e.push(rect(
            tgo,
            x + left - 0.1,
            o + 0.35,
            x + left + 0.6,
            o + 1.05,
        ));
    };
    c1(o + 8.0, 0.25, &mut elems);
    c1(o + 12.0, 0.45, &mut elems);
    // e1.dig fires (left margin 0.20, rest 0.30) / relaxed-clean (0.30 all round).
    let e1 = |x: f64, left: f64, e: &mut Vec<GdsElement>| {
        e.push(rect(nw, x, o, x + left + 0.5 + 0.30, o + 1.1));
        e.push(rect(activ, x + left, o + 0.30, x + left + 0.5, o + 0.80));
        e.push(rect(
            tgo,
            x + left - 0.1,
            o + 0.20,
            x + left + 0.6,
            o + 0.90,
        ));
    };
    e1(o + 16.0, 0.20, &mut elems);
    e1(o + 20.0, 0.30, &mut elems);
    // Outside the DigiBnd: strict NW.d1 still fires at 0.45.
    d1(o + 30.0, 0.45, &mut elems);
    write_gz(&format!("{DIR}/NW.dig.gds.gz"), library("TOP", elems));
}

/// NW.f — min. NWell space to the substrate tie (P+Activ in PWell), not in TGO, 0.24 µm.
fn nw_f(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    elems.extend(pact(activ, psd, o + 1.0 + 0.30, o, 0.5, 0.5)); // gap 0.30 → clean
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    elems.extend(pact(activ, psd, o + 5.0 + 0.20, o, 0.5, 0.5)); // gap 0.20 → NW.f
    write_gz(&format!("{DIR}/NW.f.gds.gz"), library("TOP", elems));
}

/// NW.f1 — NWell space to the substrate tie inside ThickGateOx, 0.62 µm.
fn nw_f1(pdk: &PdkConfig) {
    let nw = layer(pdk, "NWell");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let tgo = layer(pdk, "ThickGateOx");
    let o = OFFSET;
    let mut elems = vec![rect(nw, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + 0.62;
    elems.extend(pact(activ, psd, x1, o, 0.5, 0.5)); // gap 0.62 → clean
    elems.push(rect(tgo, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
    elems.push(rect(nw, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + 0.50;
    elems.extend(pact(activ, psd, x2, o, 0.5, 0.5)); // gap 0.50 → NW.f1
    elems.push(rect(tgo, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    write_gz(&format!("{DIR}/NW.f1.gds.gz"), library("TOP", elems));
}

// ---------------------------------------------------------------------------------------
// Hardening patterns (ci/hardening/SPEC.md): layouts drawn from the manual's section 5.1
// and 8.1.1 alone, one fixture per theme, `NW.<rule>.h<n>`.  Each function's comment
// states the geometry and what the manual says about it; the expected answers are in
// the `nwell` table of tests/ihp-sg13g2.rs and the reasoning in
// ci/hardening/reports/ihp-sg13g2/nwell.md.
// ---------------------------------------------------------------------------------------

/// Layers the hardening patterns draw on.
struct L {
    nw: (i16, i16),
    activ: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    nsd_block: (i16, i16),
    tgo: (i16, i16),
    digi: (i16, i16),
    cont: (i16, i16),
    m1: (i16, i16),
    via1: (i16, i16),
    m2: (i16, i16),
    pwb: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            nw: layer(pdk, "NWell"),
            activ: layer(pdk, "Activ"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            nsd_block: layer(pdk, "nSD.block"),
            tgo: layer(pdk, "ThickGateOx"),
            digi: layer(pdk, "DigiBnd"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            via1: layer(pdk, "Via1"),
            m2: layer(pdk, "Metal2"),
            pwb: layer(pdk, "PWell.block"),
        }
    }

    /// P+Activ box (Activ under pSD).
    fn pact(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.activ, x0, y0, x1, y1),
            rect(self.psd, x0, y0, x1, y1),
        ]
    }

    /// N+Activ box with drawn nSD (plain Activ is N+ as well; this one is explicit).
    fn nact(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        vec![
            rect(self.activ, x0, y0, x1, y1),
            rect(self.nsd, x0, y0, x1, y1),
        ]
    }

    /// NWell box with margins `l, r, b, t` around the inner box `(x0, y0)-(x1, y1)`.
    #[allow(clippy::too_many_arguments)]
    fn nw_around(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        l: f64,
        r: f64,
        b: f64,
        t: f64,
    ) -> GdsElement {
        rect(self.nw, x0 - l, y0 - b, x1 + r, y1 + t)
    }

    /// Square NWell ring: outer box `(x0, y0)-(x1, y1)` minus the hole `(hx0, hy0)-(hx1, hy1)`,
    /// drawn as four overlapping wall rectangles that merge into one ring.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        hx0: f64,
        hy0: f64,
        hx1: f64,
        hy1: f64,
    ) -> Vec<GdsElement> {
        vec![
            rect(self.nw, x0, y0, hx0, y1),
            rect(self.nw, hx1, y0, x1, y1),
            rect(self.nw, x0, y0, x1, hy0),
            rect(self.nw, x0, hy1, x1, y1),
        ]
    }

    /// The same ring as one keyhole polygon (GDS has no holes: the outline runs in along
    /// a zero-width cut, round the hole and back out).
    #[allow(clippy::too_many_arguments)]
    fn keyhole_ring(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        hx0: f64,
        hy0: f64,
        hx1: f64,
        hy1: f64,
    ) -> GdsElement {
        poly(
            self.nw,
            &[
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0, hy0),
                (hx0, hy0),
                (hx0, hy1),
                (hx1, hy1),
                (hx1, hy0),
                (x0, hy0),
            ],
        )
    }

    /// DigiBnd frame 1 µm wide whose hole is `(x0, y0)-(x1, y1)`.
    fn digi_ring(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
        poly(
            self.digi,
            &[
                (x0 - 1.0, y0 - 1.0),
                (x1 + 1.0, y0 - 1.0),
                (x1 + 1.0, y1 + 1.0),
                (x0 - 1.0, y1 + 1.0),
                (x0 - 1.0, y0),
                (x0, y0),
                (x0, y1),
                (x1, y1),
                (x1, y0),
                (x0 - 1.0, y0),
            ],
        )
    }

    /// A 0.40 µm N+Activ tap with a Cont, centred on `(cx, cy)`, plus a Metal1 pad over it.
    fn tap_pad(&self, cx: f64, cy: f64) -> Vec<GdsElement> {
        let mut e = tap(self.activ, self.cont, cx, cy, 0.40);
        e.push(strap(self.m1, &[(cx, cy)]));
        e
    }
}

/// NWell 1.7 × 1.9 trapezoid at `(x, y)` whose right side is a 45° wall from
/// `(x+1.7, y+0.9)` up-left to `(x+0.7, y+1.9)`, i.e. along `X + Y = x + y + 2.6`.  No
/// corner is acute (the manual forbids angles below 87°), the wedge between the bottom
/// edge and the wall is 0.636 wide and the top edge 0.7, both clear of NW.a.
fn slant_nw(nw: (i16, i16), x: f64, y: f64) -> GdsElement {
    poly(
        nw,
        &[
            (x, y),
            (x + 1.7, y),
            (x + 1.7, y + 0.9),
            (x + 0.7, y + 1.9),
            (x, y + 1.9),
        ],
    )
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// Writes `<name>.h<n>` flat and `<name>.h<n+1>` as an array reference: 10 × 5 copies of
/// `cell` at `pitch`.  The manual knows nothing about hierarchy, so both must report the
/// violating pattern fifty times.
fn arrays(name: &str, n: u32, cell: Vec<GdsElement>, pitch: f64) {
    write(&format!("{name}.h{n}"), flat_array(&cell, 10, 5, pitch));
    write_gz(
        &format!("{DIR}/{name}.h{}.gds.gz", n + 1),
        ref_array(cell, 10, 5, pitch),
    );
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    nw_a_h(&l);
    nw_b_h(&l);
    nw_b1_h(&l);
    nw_c_h(&l);
    nw_c1_h(&l);
    nw_d_h(&l);
    nw_d1_h(&l);
    nw_e_h(&l);
    nw_e1_h(&l);
    nw_f_h(&l);
    nw_f1_h(&l);
}

// --- NW.a: min. NWell width 0.62 ---

fn nw_a_h(l: &L) {
    let nw = l.nw;

    // h1 — the bound and a long shape.  0.62 wide is legal, 0.615 (one grid step under)
    // is not, in x and in y; a 300 µm bar crossing every tile line counts once.
    write(
        "NW.a.h1",
        vec![
            rect(nw, 2.0, 2.0, 2.62, 4.0),      // clean
            rect(nw, 5.0, 2.0, 5.615, 4.0),     // NW.a (x)
            rect(nw, 8.0, 2.0, 10.0, 2.615),    // NW.a (y)
            rect(nw, 2.0, 8.0, 302.0, 8.62),    // clean, 300 µm long
            rect(nw, 2.0, 12.0, 302.0, 12.615), // NW.a, 300 µm long → one violation
        ],
    );

    // h2 — 45° geometry.  A diamond's width is the distance between opposite walls
    // (a·√2): a = 0.44 → 0.622 clean, a = 0.435 → 0.615 fires.  Same for a 45° strip
    // (d·√2) and an octagon whose diagonal faces are k·√2 apart.  A chamfered box and an
    // L with a chamfered inner corner are wide everywhere and stay clean.
    let oct = |cx: f64, cy: f64, h: f64, k: f64| {
        let s = k - h;
        poly(
            nw,
            &[
                (cx + h, cy - s),
                (cx + h, cy + s),
                (cx + s, cy + h),
                (cx - s, cy + h),
                (cx - h, cy + s),
                (cx - h, cy - s),
                (cx - s, cy - h),
                (cx + s, cy - h),
            ],
        )
    };
    write(
        "NW.a.h2",
        vec![
            diamond(nw, 3.0, 3.0, 0.44),                   // clean (0.622)
            diamond(nw, 7.0, 3.0, 0.435),                  // NW.a (0.615)
            strip45(nw, 10.0, 2.0, 3.0, 0.44),             // clean
            strip45(nw, 15.0, 2.0, 3.0, 0.435),            // NW.a
            oct(3.0, 9.0, 0.31, 0.44),                     // clean (diagonal 0.622)
            oct(7.0, 9.0, 0.31, 0.435),                    // NW.a (diagonal 0.615)
            chamfered_tr(nw, 10.0, 8.0, 12.0, 10.0, 21.5), // clean
            poly(
                nw,
                &[
                    (14.0, 8.0),
                    (17.0, 8.0),
                    (17.0, 9.0),
                    (15.5, 9.0),
                    (15.0, 9.5),
                    (15.0, 11.0),
                    (14.0, 11.0),
                ],
            ), // clean: L, inner corner chamfered, arms 1.0 wide
        ],
    );

    // h3 — shapes that merge.  The rule reads the union: two overlapping 0.40 boxes whose
    // union is 0.62 wide are clean, 0.615 fires once (not twice); four abutting 0.155
    // slices make 0.62 (clean) and 3 × 0.155 + 0.15 make 0.615 (fires); a 0.62 bar drawn
    // as a 4 × 10 grid of tiny boxes is clean; a ring with one 0.615 side fires once; an
    // island inside a ring's hole is a shape of its own (0.615 wide → fires once).
    let mut e = vec![
        rect(nw, 2.0, 2.0, 2.4, 4.0),
        rect(nw, 2.22, 2.0, 2.62, 4.0), // union 0.62 → clean
        rect(nw, 5.0, 2.0, 5.4, 4.0),
        rect(nw, 5.215, 2.0, 5.615, 4.0), // union 0.615 → NW.a
    ];
    for i in 0..4 {
        e.push(rect(
            nw,
            8.0 + 0.155 * i as f64,
            2.0,
            8.155 + 0.155 * i as f64,
            4.0,
        )); // 0.62 clean
    }
    for i in 0..3 {
        e.push(rect(
            nw,
            11.0 + 0.155 * i as f64,
            2.0,
            11.155 + 0.155 * i as f64,
            4.0,
        ));
    }
    e.push(rect(nw, 11.465, 2.0, 11.615, 4.0)); // 0.615 → NW.a
    for i in 0..4 {
        for j in 0..10 {
            let (x, y) = (14.0 + 0.155 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(nw, x, y, x + 0.155, y + 0.2)); // 0.62 × 2 grid → clean
        }
    }
    e.extend(l.ring(2.0, 7.0, 6.0, 11.0, 2.615, 8.0, 5.0, 10.0)); // left side 0.615 → NW.a
    e.extend(l.ring(8.0, 7.0, 12.0, 11.0, 8.8, 7.8, 11.2, 10.2));
    e.push(rect(nw, 9.6, 8.6, 10.4, 9.4)); // island 0.8 wide, 0.8 from the ring → clean
    e.extend(l.ring(14.0, 7.0, 18.0, 11.0, 14.8, 7.8, 17.2, 10.2));
    e.push(rect(nw, 15.6, 8.6, 16.215, 9.4)); // island 0.615 wide → NW.a
    write("NW.a.h3", e);

    // h4 — tile lines.  0.615-wide bars ending on x = 20, straddling 20, starting on 20,
    // straddling 21, ending on 40, straddling 42, well inside a tile at 10; 0.615-tall
    // bars running across 20/21 and 40/42; an L with its corner on x = 20 and only the
    // vertical arm narrow.  Ten violations whatever the tile; the 0.62 controls are clean.
    write(
        "NW.a.h4",
        vec![
            rect(nw, 9.385, 2.0, 10.0, 4.0),
            rect(nw, 19.385, 2.0, 20.0, 4.0),
            rect(nw, 19.7, 6.0, 20.315, 8.0),
            rect(nw, 20.0, 10.0, 20.615, 12.0),
            rect(nw, 20.7, 14.0, 21.315, 16.0),
            rect(nw, 39.385, 2.0, 40.0, 4.0),
            rect(nw, 41.7, 6.0, 42.315, 8.0),
            rect(nw, 15.0, 18.0, 25.0, 18.615),
            rect(nw, 35.0, 18.0, 45.0, 18.615),
            poly(
                nw,
                &[
                    (20.0, 22.0),
                    (23.0, 22.0),
                    (23.0, 23.0),
                    (20.615, 23.0),
                    (20.615, 26.0),
                    (20.0, 26.0),
                ],
            ),
            rect(nw, 19.7, 28.0, 20.32, 30.0), // 0.62 straddling 20 → clean
            rect(nw, 15.0, 32.0, 25.0, 32.62), // 0.62 tall across 20 → clean
        ],
    );

    // h5/h6 — fifty 0.615 × 1 bars, flat and as an array reference; pitch 3 keeps the bars
    // 2.385 apart so no NWell space rule joins in.
    arrays("NW.a", 5, vec![rect(nw, 0.2, 0.2, 0.815, 1.2)], 3.0);

    // h7 — a 0.005 sliver (one grid step) and a bar far from everything at (1000, 1000).
    write(
        "NW.a.h7",
        vec![
            rect(nw, 2.0, 2.0, 2.005, 4.0),
            rect(nw, 1000.0, 1000.0, 1000.615, 1002.0),
        ],
    );

    // h8 — a comb whose three teeth are 0.615 wide (three violations of one polygon) and
    // a U whose 0.62 arms are clean.
    write(
        "NW.a.h8",
        vec![
            poly(
                nw,
                &[
                    (2.0, 2.0),
                    (6.0, 2.0),
                    (6.0, 3.0),
                    (5.615, 3.0),
                    (5.615, 5.0),
                    (5.0, 5.0),
                    (5.0, 3.0),
                    (4.115, 3.0),
                    (4.115, 5.0),
                    (3.5, 5.0),
                    (3.5, 3.0),
                    (2.615, 3.0),
                    (2.615, 5.0),
                    (2.0, 5.0),
                ],
            ),
            poly(
                nw,
                &[
                    (9.0, 2.0),
                    (12.0, 2.0),
                    (12.0, 5.0),
                    (11.38, 5.0),
                    (11.38, 3.0),
                    (9.62, 3.0),
                    (9.62, 5.0),
                    (9.0, 5.0),
                ],
            ),
        ],
    );
}

// --- NW.b: min. NWell space or notch 0.62 (same net; closer regions merge) ---

fn nw_b_h(l: &L) {
    let nw = l.nw;

    // h1 — the bound and both metrics.  Gap 0.615 fires; a diagonal offset of 0.44/0.44
    // is 0.622 corner to corner (clean, though each axis alone reads 0.44), 0.43/0.43 is
    // 0.608 (fires); an x-gap of 0.615 between boxes that only meet corner-on in
    // projection fires; a 0.62 gap is clean.
    write(
        "NW.b.h1",
        vec![
            rect(nw, 2.0, 2.0, 4.0, 4.0),
            rect(nw, 4.615, 2.0, 6.615, 4.0), // NW.b
            rect(nw, 9.0, 2.0, 11.0, 4.0),
            rect(nw, 11.44, 4.44, 13.44, 6.44), // clean (0.622)
            rect(nw, 15.0, 2.0, 17.0, 4.0),
            rect(nw, 17.43, 4.43, 19.43, 6.43), // NW.b (0.608)
            rect(nw, 2.0, 8.0, 4.0, 10.0),
            rect(nw, 4.615, 10.0, 6.615, 12.0), // NW.b (corner to corner 0.615)
            rect(nw, 9.0, 8.0, 11.0, 10.0),
            rect(nw, 11.62, 9.0, 13.62, 11.0), // clean
        ],
    );

    // h2 — 45° geometry.  A diamond tip 0.615 from a straight wall fires, 0.62 is clean;
    // two parallel 45° strips (d = 1, shifted 2.87 in y → perpendicular gap 0.615) fire,
    // shifted 2.88 (0.622) clean; a box corner facing a chamfer whose edge passes 0.601
    // from it fires (c = 0.25), 0.622 (c = 0.28) clean; two diamond tips 0.615 apart fire.
    write(
        "NW.b.h2",
        vec![
            rect(nw, 2.0, 2.0, 4.0, 6.0),
            diamond(nw, 5.615, 4.0, 1.0), // NW.b (tip at 4.615)
            rect(nw, 9.0, 2.0, 11.0, 6.0),
            diamond(nw, 12.62, 4.0, 1.0), // clean (tip at 11.62)
            strip45(nw, 2.0, 8.0, 3.0, 1.0),
            strip45(nw, 2.0, 10.87, 3.0, 1.0), // NW.b (0.615)
            strip45(nw, 9.0, 8.0, 3.0, 1.0),
            strip45(nw, 9.0, 10.88, 3.0, 1.0), // clean (0.622)
            rect(nw, 15.0, 2.0, 17.0, 4.0),
            chamfered_bl(nw, 17.3, 4.3, 19.3, 6.3, 21.85), // NW.b (0.601 to the corner)
            rect(nw, 15.0, 8.0, 17.0, 10.0),
            chamfered_bl(nw, 17.3, 10.3, 19.3, 12.3, 27.88), // clean (0.622)
            diamond(nw, 15.0, 16.0, 1.0),
            diamond(nw, 17.615, 16.0, 1.0), // NW.b (tips 0.615 apart)
        ],
    );

    // h3 — notches and slots.  The helpers give a straight U notch and a straight-vs-45°
    // notch, each at 0.615 (fires) and 0.62 (clean); a comb with three 0.615 slots fires
    // three times; a 0.615 slot cut into a plate fires; two Ls whose vertical arms face
    // across 0.615 fire once; a ring whose hole is 0.615 wide is a notch (fires), 0.62 is
    // clean; an island 0.615 from the inner wall of a ring fires.
    let mut e = notch_pattern(nw, 1.0, 0.62, 3.0, 2.0, SPACE_DELTA);
    e.extend(mixed_notch_pattern(
        nw,
        1.0,
        0.62,
        1.0,
        3.0,
        26.0,
        SPACE_DELTA,
    ));
    e.push(poly(
        nw,
        &[
            (2.0, 10.0),
            (8.0, 10.0),
            (8.0, 11.0),
            (7.845, 11.0),
            (7.845, 13.0),
            (6.845, 13.0),
            (6.845, 11.0),
            (6.23, 11.0),
            (6.23, 13.0),
            (5.23, 13.0),
            (5.23, 11.0),
            (4.615, 11.0),
            (4.615, 13.0),
            (3.615, 13.0),
            (3.615, 11.0),
            (3.0, 11.0),
            (3.0, 13.0),
            (2.0, 13.0),
        ],
    )); // comb: three 0.615 slots → 3 × NW.b
    e.push(poly(
        nw,
        &[
            (11.0, 10.0),
            (15.0, 10.0),
            (15.0, 13.0),
            (13.615, 13.0),
            (13.615, 11.5),
            (13.0, 11.5),
            (13.0, 13.0),
            (11.0, 13.0),
        ],
    )); // slot 0.615 wide → NW.b
    e.push(poly(
        nw,
        &[
            (2.0, 15.0),
            (5.0, 15.0),
            (5.0, 16.0),
            (3.0, 16.0),
            (3.0, 18.0),
            (2.0, 18.0),
        ],
    ));
    e.push(poly(
        nw,
        &[
            (3.615, 17.0),
            (6.615, 17.0),
            (6.615, 18.0),
            (4.615, 18.0),
            (4.615, 20.0),
            (3.615, 20.0),
        ],
    )); // arms face across 0.615 (y 17..18) → NW.b
    e.push(l.keyhole_ring(9.0, 15.0, 12.0, 18.0, 10.2, 16.0, 10.815, 17.0)); // hole 0.615 → NW.b
    e.extend(l.ring(14.0, 15.0, 17.0, 18.0, 15.2, 16.0, 15.82, 17.0)); // hole 0.62 → clean
    e.extend(l.ring(2.0, 22.0, 7.0, 27.0, 3.0, 23.0, 6.0, 26.0));
    e.push(rect(nw, 3.615, 24.0, 4.615, 25.0)); // island 0.615 from the left wall → NW.b
    write("NW.b.h3", e);

    // h4 — shapes that merge.  Two overlapping boxes, two abutting boxes and a 4 × 4 grid
    // of small boxes each face a third box across 0.615: one violation each, measured from
    // the union; the 0.62 control is clean.
    let mut e = vec![
        rect(nw, 2.0, 2.0, 3.0, 3.0),
        rect(nw, 2.8, 2.0, 4.0, 3.0),
        rect(nw, 4.615, 2.0, 6.0, 3.0), // NW.b
        rect(nw, 8.0, 2.0, 9.0, 3.0),
        rect(nw, 9.0, 2.0, 10.0, 3.0),
        rect(nw, 10.615, 2.0, 12.0, 3.0), // NW.b
        rect(nw, 15.615, 2.0, 17.0, 3.0), // NW.b (against the grid below)
        rect(nw, 2.0, 6.0, 3.0, 7.0),
        rect(nw, 2.8, 6.0, 4.0, 7.0),
        rect(nw, 4.62, 6.0, 6.0, 7.0), // clean
    ];
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (14.0 + 0.25 * i as f64, 2.0 + 0.25 * j as f64);
            e.push(rect(nw, x, y, x + 0.25, y + 0.25));
        }
    }
    write("NW.b.h4", e);

    // h5 — tile lines.  0.615 gaps well inside a tile, starting on x = 20, ending on 20,
    // straddling 20, straddling 21, ending on 40, straddling 42; a horizontal 0.615 gap
    // running across 20/21 and across 40/42; a diagonal 0.43/0.43 pair whose corner is
    // on x = 20.  Ten violations; the 0.62 control straddling 20 is clean.
    write(
        "NW.b.h5",
        vec![
            rect(nw, 8.7, 2.0, 9.7, 4.0),
            rect(nw, 10.315, 2.0, 11.315, 4.0),
            rect(nw, 19.0, 2.0, 20.0, 4.0),
            rect(nw, 20.615, 2.0, 21.615, 4.0),
            rect(nw, 18.385, 6.0, 19.385, 8.0),
            rect(nw, 20.0, 6.0, 21.0, 8.0),
            rect(nw, 18.7, 10.0, 19.7, 12.0),
            rect(nw, 20.315, 10.0, 21.315, 12.0),
            rect(nw, 19.7, 14.0, 20.7, 16.0),
            rect(nw, 21.315, 14.0, 22.315, 16.0),
            rect(nw, 38.385, 2.0, 39.385, 4.0),
            rect(nw, 40.0, 2.0, 41.0, 4.0),
            rect(nw, 40.7, 6.0, 41.7, 8.0),
            rect(nw, 42.315, 6.0, 43.315, 8.0),
            rect(nw, 15.0, 18.0, 25.0, 19.0),
            rect(nw, 15.0, 19.615, 25.0, 20.615),
            rect(nw, 35.0, 18.0, 45.0, 19.0),
            rect(nw, 35.0, 19.615, 45.0, 20.615),
            rect(nw, 18.0, 22.0, 20.0, 24.0),
            rect(nw, 20.43, 24.43, 22.43, 26.43),
            rect(nw, 18.7, 28.0, 19.7, 30.0),
            rect(nw, 20.32, 28.0, 21.32, 30.0), // 0.62 → clean
        ],
    );

    // h6/h7 — fifty pairs at 0.615, flat and as an array reference (pitch 5: cells 2.385
    // apart, more than NW.b1's 1.80).
    arrays(
        "NW.b",
        6,
        vec![
            rect(nw, 0.2, 0.2, 1.2, 1.2),
            rect(nw, 1.815, 0.2, 2.815, 1.2),
        ],
        5.0,
    );

    // h8 — a 0.005 sliver 0.615 from a box (NW.b, and NW.a for the sliver); two 300 µm bars
    // 0.615 apart (one violation); a pair far away at (1000, 1000).
    write(
        "NW.b.h8",
        vec![
            rect(nw, 2.0, 2.0, 2.005, 4.0),
            rect(nw, 2.62, 2.0, 4.0, 4.0),
            rect(nw, 2.0, 8.0, 302.0, 9.0),
            rect(nw, 2.0, 9.615, 302.0, 10.615),
            rect(nw, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(nw, 1001.615, 1000.0, 1002.615, 1001.0),
        ],
    );

    // h9 — nets.  Two wells 0.50 apart tied to different Metal1 nets and two 0.50 apart
    // tied to one net: the manual says regions closer than 0.62 will be merged, so both
    // pairs violate NW.b regardless of net (KLayout's driver calls the first NW.b1).
    let mut e = vec![
        rect(nw, 2.0, 2.0, 3.0, 3.0),
        rect(nw, 3.5, 2.0, 4.5, 3.0),
        rect(nw, 8.0, 2.0, 9.0, 3.0),
        rect(nw, 9.5, 2.0, 10.5, 3.0),
    ];
    e.extend(l.tap_pad(2.5, 2.5));
    e.extend(l.tap_pad(4.0, 2.5));
    e.extend(tap(l.activ, l.cont, 8.5, 2.5, 0.40));
    e.extend(tap(l.activ, l.cont, 10.0, 2.5, 0.40));
    e.push(strap(l.m1, &[(8.5, 2.5), (10.0, 2.5)]));
    write("NW.b.h9", e);
}

// --- NW.b1: min. PWell width between NWell regions on different nets 1.80 ---

fn nw_b1_h(l: &L) {
    let nw = l.nw;

    // h1 — the bound, bare wells (nothing ties them, so every pair is different-net).
    // Gaps 0.62 (NW.b's own limit: not merged, so NW.b1 applies), 0.625 and 1.795 fire;
    // 1.80 is clean.  Diagonal 1.28/1.28 is 1.810 corner to corner (clean), 1.27/1.27 is
    // 1.796 (fires), 1.20/1.20 is 1.697 and 0.90/0.90 is 1.273 (both fire).
    write(
        "NW.b1.h1",
        vec![
            rect(nw, 2.0, 14.0, 3.0, 16.0),
            rect(nw, 4.2, 17.2, 5.2, 19.2), // NW.b1 (1.697)
            rect(nw, 15.0, 14.0, 16.0, 16.0),
            rect(nw, 16.9, 16.9, 17.9, 18.9), // NW.b1 (0.9/0.9 → 1.273)
            rect(nw, 2.0, 2.0, 3.0, 4.0),
            rect(nw, 3.62, 2.0, 4.62, 4.0), // NW.b1 (0.62)
            rect(nw, 8.0, 2.0, 9.0, 4.0),
            rect(nw, 9.625, 2.0, 10.625, 4.0), // NW.b1 (0.625)
            rect(nw, 14.0, 2.0, 15.0, 4.0),
            rect(nw, 16.795, 2.0, 17.795, 4.0), // NW.b1 (1.795)
            rect(nw, 2.0, 8.0, 3.0, 10.0),
            rect(nw, 4.8, 8.0, 5.8, 10.0), // clean (1.80)
            rect(nw, 10.0, 8.0, 11.0, 10.0),
            rect(nw, 12.28, 11.28, 13.28, 13.28), // clean (1.810)
            rect(nw, 16.0, 8.0, 17.0, 10.0),
            rect(nw, 18.27, 11.27, 19.27, 13.27), // NW.b1 (1.796)
        ],
    );

    // h2 — nets.  Four 1.2 µm well pairs 1.00 apart, each well with a 0.40 N+Activ tap
    // (NWell tie) where it says so:
    //  - ties on two separate Metal1 pads: different nets → NW.b1;
    //  - ties joined through Metal1 → Via1 → Metal2 → Via1 → Metal1: one net → clean;
    //  - P+Activ (pSD) "taps" with Cont and one Metal1 strap: a PMOS S/D short, not a
    //    well connection → NW.b1;
    //  - N+Activ taps under one Metal1 strap but without Cont → not connected → NW.b1.
    let pair = |y: f64| {
        vec![
            rect(nw, 2.0, y, 3.2, y + 1.2),
            rect(nw, 4.2, y, 5.4, y + 1.2),
        ]
    };
    let (ax, bx) = (2.6, 4.8);
    let mut e = pair(2.0);
    e.extend(l.tap_pad(ax, 2.6));
    e.extend(l.tap_pad(bx, 2.6));
    e.extend(pair(6.0));
    e.extend(l.tap_pad(ax, 6.6));
    e.extend(l.tap_pad(bx, 6.6));
    e.push(rect(l.via1, ax - 0.095, 6.505, ax + 0.095, 6.695));
    e.push(rect(l.via1, bx - 0.095, 6.505, bx + 0.095, 6.695));
    e.push(rect(l.m2, ax - 0.2, 6.4, bx + 0.2, 6.8));
    e.extend(pair(10.0));
    e.extend(l.pact(ax - 0.2, 10.4, ax + 0.2, 10.8));
    e.extend(l.pact(bx - 0.2, 10.4, bx + 0.2, 10.8));
    e.push(rect(l.cont, ax - 0.08, 10.52, ax + 0.08, 10.68));
    e.push(rect(l.cont, bx - 0.08, 10.52, bx + 0.08, 10.68));
    e.push(strap(l.m1, &[(ax, 10.6), (bx, 10.6)]));
    e.extend(pair(14.0));
    e.push(rect(l.activ, ax - 0.2, 14.4, ax + 0.2, 14.8));
    e.push(rect(l.activ, bx - 0.2, 14.4, bx + 0.2, 14.8));
    e.push(strap(l.m1, &[(ax, 14.6), (bx, 14.6)]));
    write("NW.b1.h2", e);

    // h3 — one net through the well itself.  A U whose arms face across 1.00 (one shape,
    // same net: NW.b applies at 0.62, NW.b1 does not) → clean; two bars joined by a 0.62
    // bridge → clean; a ring with an island 1.00 from its inner wall, island tied to the
    // ring by Cont/Metal1 → clean; the same island untied → NW.b1 (its other gaps are
    // 1.80 or more).
    let mut e = vec![
        poly(
            nw,
            &[
                (2.0, 2.0),
                (5.0, 2.0),
                (5.0, 5.0),
                (4.0, 5.0),
                (4.0, 3.0),
                (3.0, 3.0),
                (3.0, 5.0),
                (2.0, 5.0),
            ],
        ),
        rect(nw, 9.0, 2.0, 10.0, 5.0),
        rect(nw, 11.0, 2.0, 12.0, 5.0),
        rect(nw, 9.0, 5.0, 12.0, 5.62),
    ];
    e.extend(l.ring(2.0, 8.0, 8.6, 14.6, 3.0, 9.0, 7.6, 13.6));
    e.push(rect(nw, 4.0, 10.8, 5.0, 11.8));
    e.extend(tap(l.activ, l.cont, 4.5, 11.3, 0.40));
    e.extend(tap(l.activ, l.cont, 5.3, 14.1, 0.40));
    e.push(strap(l.m1, &[(4.5, 11.3), (5.3, 14.1)]));
    e.extend(l.ring(11.0, 8.0, 17.6, 14.6, 12.0, 9.0, 16.6, 13.6));
    e.push(rect(nw, 13.0, 10.8, 14.0, 11.8)); // NW.b1
    write("NW.b1.h3", e);

    // h4 — 45° geometry.  A diamond tip 1.795 from a wall fires, 1.80 is clean; two 45°
    // strips (d = 1) shifted 4.54 in y sit 1.796 apart (fires), 4.55 → 1.803 (clean).
    write(
        "NW.b1.h4",
        vec![
            rect(nw, 2.0, 2.0, 4.0, 6.0),
            diamond(nw, 6.795, 4.0, 1.0), // NW.b1
            rect(nw, 11.0, 2.0, 13.0, 6.0),
            diamond(nw, 15.8, 4.0, 1.0), // clean
            strip45(nw, 2.0, 8.0, 3.0, 1.0),
            strip45(nw, 2.0, 12.54, 3.0, 1.0), // NW.b1 (1.796)
            strip45(nw, 11.0, 8.0, 3.0, 1.0),
            strip45(nw, 11.0, 12.55, 3.0, 1.0), // clean (1.803)
        ],
    );

    // h5 — tile lines, gap 1.795: inside a tile, starting on x = 20, ending on 20,
    // straddling 20, straddling 21, ending on 40, straddling 42, and a horizontal gap
    // running across 20/21.  Eight violations; the 1.80 control straddling 20 is clean.
    write(
        "NW.b1.h5",
        vec![
            rect(nw, 8.0, 2.0, 9.0, 4.0),
            rect(nw, 10.795, 2.0, 11.795, 4.0),
            rect(nw, 19.0, 2.0, 20.0, 4.0),
            rect(nw, 21.795, 2.0, 22.795, 4.0),
            rect(nw, 17.205, 6.0, 18.205, 8.0),
            rect(nw, 20.0, 6.0, 21.0, 8.0),
            rect(nw, 18.1, 10.0, 19.1, 12.0),
            rect(nw, 20.895, 10.0, 21.895, 12.0),
            rect(nw, 19.5, 14.0, 20.5, 16.0),
            rect(nw, 22.295, 14.0, 23.295, 16.0),
            rect(nw, 37.205, 2.0, 38.205, 4.0),
            rect(nw, 40.0, 2.0, 41.0, 4.0),
            rect(nw, 40.1, 6.0, 41.1, 8.0),
            rect(nw, 42.895, 6.0, 43.895, 8.0),
            rect(nw, 15.0, 18.0, 25.0, 19.0),
            rect(nw, 15.0, 20.795, 25.0, 21.795),
            rect(nw, 18.1, 24.0, 19.1, 26.0),
            rect(nw, 20.9, 24.0, 21.9, 26.0), // 1.80 → clean
        ],
    );

    // h6/h7 — fifty bare pairs 1.00 apart, flat and as an array reference (pitch 6:
    // cells 2.80 apart or more).
    arrays(
        "NW.b1",
        6,
        vec![rect(nw, 0.2, 0.2, 1.2, 1.2), rect(nw, 2.2, 0.2, 3.2, 1.2)],
        6.0,
    );

    // h8 — PWell:block in the gap.  PWell is NOT (NWell OR PWell:block), so with the
    // whole 1.00 gap under PWell:block there is no PWell between the wells and NW.b1 has
    // nothing to measure (PWB.c covers the block-to-well space); with a 0.50 block strip
    // in the middle, 0.25 of PWell remains on either side and NW.b1 fires.
    write(
        "NW.b1.h8",
        vec![
            rect(nw, 2.0, 2.0, 3.0, 4.0),
            rect(nw, 4.0, 2.0, 5.0, 4.0),
            rect(l.pwb, 3.0, 2.0, 4.0, 4.0),
            rect(nw, 8.0, 2.0, 9.0, 4.0),
            rect(nw, 10.0, 2.0, 11.0, 4.0),
            rect(l.pwb, 9.25, 2.0, 9.75, 4.0), // NW.b1
        ],
    );
}

// --- NW.c: min. NWell enclosure of P+Activ not inside ThickGateOx 0.31 ---

fn nw_c_h(l: &L) {
    let nw = l.nw;

    // One 0.5 × 0.5 P+Activ at `(x, y)` with the NWell margins given.
    let inst = |x: f64, y: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        let mut e = l.pact(x, y, x + 0.5, y + 0.5);
        e.push(l.nw_around(x, y, x + 0.5, y + 0.5, ml, mr, mb, mt));
        e
    };

    // h1 — the bound and both metrics.  A 0.305 right margin fires.  With 0.31 on every
    // side but the NWell's top-right corner chamfered along x + y = k (Activ-relative),
    // the Activ corner is (k − 1)/√2 from the chamfer: k = 1.38 → 0.269 (fires, although
    // both axis margins are 0.5), k = 1.44 → 0.311 (clean).  The same k = 1.38 as a full
    // 45° wall fires.  An Activ corner chamfered along x + y = 1.7 facing a parallel NWell
    // chamfer: k = 2.13 → 0.304 fires, 2.14 → 0.311 clean.
    let cham = |x: f64, y: f64, k: f64| {
        let mut e = l.pact(x, y, x + 0.5, y + 0.5);
        e.push(chamfered_tr(
            nw,
            x - 0.31,
            y - 0.31,
            x + 1.0,
            y + 1.0,
            x + y + k,
        ));
        e
    };
    let par = |x: f64, y: f64, k: f64| {
        let mut e = vec![
            poly(
                l.activ,
                &[
                    (x, y),
                    (x + 1.0, y),
                    (x + 1.0, y + 0.7),
                    (x + 0.7, y + 1.0),
                    (x, y + 1.0),
                ],
            ),
            poly(
                l.psd,
                &[
                    (x, y),
                    (x + 1.0, y),
                    (x + 1.0, y + 0.7),
                    (x + 0.7, y + 1.0),
                    (x, y + 1.0),
                ],
            ),
        ];
        e.push(chamfered_tr(
            nw,
            x - 0.31,
            y - 0.31,
            x + 1.31,
            y + 1.31,
            x + y + k,
        ));
        e
    };
    let mut e = inst(2.0, 2.0, 0.31, 0.305, 0.31, 0.31); // NW.c
    e.extend(cham(6.0, 2.0, 1.38)); // NW.c (0.269 to the chamfer)
    e.extend(cham(10.0, 2.0, 1.44)); // clean (0.311)
    e.extend(l.pact(14.0, 2.0, 14.5, 2.5));
    e.push(poly(
        nw,
        &[
            (13.69, 1.69),
            (14.81, 1.69),
            (14.81, 2.57),
            (14.38, 3.0),
            (13.69, 3.0),
        ],
    )); // NW.c (45° wall, 0.269)
    e.extend(par(2.0, 7.0, 2.13)); // NW.c (0.304 between parallel chamfers)
    e.extend(par(6.0, 7.0, 2.14)); // clean (0.311)
    write("NW.c.h1", e);

    // h2 — shapes that merge and the rule's conditions.  P+Activ drawn as two overlapping
    // boxes with 0.31 around the union → clean, 0.305 → fires once; NWell drawn as two
    // overlapping boxes (each alone would clip the Activ) whose union encloses by 0.31 →
    // clean; two abutting NWell boxes → clean; plain Activ (N+, no pSD) at 0.305 is a well
    // tie, not P+Activ → clean; pSD over the left half only: the P+ half's 0.305 left
    // margin fires, the N+ half's 0.31 right margin is clean; a P+Activ in a ring's body
    // 0.305 from the hole fires.
    let mut e = l.pact(2.0, 2.0, 2.6, 2.5);
    e.extend(l.pact(2.4, 2.0, 3.0, 2.5));
    e.push(l.nw_around(2.0, 2.0, 3.0, 2.5, 0.31, 0.31, 0.31, 0.31)); // clean
    e.extend(l.pact(6.0, 2.0, 6.6, 2.5));
    e.extend(l.pact(6.4, 2.0, 7.0, 2.5));
    e.push(l.nw_around(6.0, 2.0, 7.0, 2.5, 0.31, 0.305, 0.31, 0.31)); // NW.c
    e.extend(l.pact(10.0, 2.0, 11.0, 2.5));
    e.push(rect(nw, 9.69, 1.69, 10.6, 2.81));
    e.push(rect(nw, 10.3, 1.69, 11.31, 2.81)); // clean (union)
    e.extend(l.pact(14.0, 2.0, 15.0, 2.5));
    e.push(rect(nw, 13.69, 1.69, 14.5, 2.81));
    e.push(rect(nw, 14.5, 1.69, 15.31, 2.81)); // clean (abutting)
    e.push(rect(l.activ, 2.0, 6.0, 2.5, 6.5));
    e.push(l.nw_around(2.0, 6.0, 2.5, 6.5, 0.31, 0.305, 0.31, 0.31)); // clean (N+)
    e.push(rect(l.activ, 6.0, 6.0, 7.0, 6.5));
    e.push(rect(l.psd, 6.0, 5.9, 6.5, 6.6));
    e.push(l.nw_around(6.0, 6.0, 7.0, 6.5, 0.305, 0.31, 0.31, 0.31)); // NW.c (P+ half)
    e.extend(l.ring(9.5, 5.0, 14.0, 9.0, 11.0, 6.0, 13.0, 8.0));
    e.extend(l.pact(9.9, 6.5, 10.695, 7.0)); // NW.c (0.305 to the hole)
    write("NW.c.h2", e);

    // h3 — Activ crossing the well boundary.  A P+Activ half in, half out of the NWell is
    // not enclosed: NW.c (note 1 allows this only for some ESD layouts).  An N+Activ half
    // in, half out is external N+Activ at zero distance: NW.d; it is not "surrounded
    // entirely by NWell", so NW.e is not the rule for it.
    let mut e = vec![rect(nw, 2.0, 2.0, 3.0, 4.0)];
    e.extend(l.pact(2.5, 2.5, 3.5, 3.0)); // NW.c
    e.push(rect(nw, 6.0, 2.0, 7.0, 4.0));
    e.push(rect(l.activ, 6.5, 2.5, 7.5, 3.0)); // NW.d
    write("NW.c.h3", e);

    // h4 — tile lines, one margin 0.305: NWell left edge on x = 20, the 0.305 gap
    // straddling 20, Activ edge on 20, NWell right edge on 20, at 21, 40, 42, inside at
    // 10; a 10 µm long Activ with a 0.305 bottom margin across 20.  Nine violations; the
    // 0.31 control straddling 20 is clean.
    let mut e = inst(20.305, 2.31, 0.305, 0.31, 0.31, 0.31);
    e.extend(inst(20.005, 6.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(20.0, 10.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(19.195, 14.31, 0.31, 0.305, 0.31, 0.31));
    e.extend(inst(21.005, 18.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(40.005, 2.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(42.005, 6.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(10.005, 2.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(l.pact(15.0, 22.31, 25.0, 22.81));
    e.push(l.nw_around(15.0, 22.31, 25.0, 22.81, 0.31, 0.31, 0.305, 0.31));
    e.extend(inst(20.01, 26.31, 0.31, 0.31, 0.31, 0.31)); // clean
    write("NW.c.h4", e);

    // h5/h6 — fifty P+Activ with a 0.305 left margin, flat and as an array reference.
    arrays("NW.c", 5, inst(0.305, 0.31, 0.305, 0.31, 0.31, 0.31), 3.5);

    // h7 — ThickGateOx.  TGO over the left half of a 1.0 × 0.5 P+Activ: that half's 0.305
    // left margin is NW.c1's business (< 0.62) and the non-TGO half's 0.31 is clean; TGO
    // over the right half with the right margin 0.62: the non-TGO left half at 0.305 →
    // NW.c; TGO abutting the Activ's right edge without overlap: NW.c only; TGO overlapping
    // the Activ by a 0.005 sliver: the sliver is P+Activ inside TGO with a 0.31 right
    // margin → NW.c1, plus NW.c for the rest at 0.305.
    let mut e = l.pact(2.0, 2.0, 3.0, 2.5);
    e.push(rect(l.tgo, 1.8, 1.8, 2.5, 2.7));
    e.push(l.nw_around(2.0, 2.0, 3.0, 2.5, 0.305, 0.31, 0.62, 0.62)); // NW.c1
    e.extend(l.pact(6.0, 2.0, 7.0, 2.5));
    e.push(rect(l.tgo, 6.5, 1.8, 7.2, 2.7));
    e.push(l.nw_around(6.0, 2.0, 7.0, 2.5, 0.305, 0.62, 0.62, 0.62)); // NW.c
    e.extend(l.pact(10.0, 2.0, 11.0, 2.5));
    e.push(rect(l.tgo, 11.0, 1.8, 11.5, 2.7));
    e.push(l.nw_around(10.0, 2.0, 11.0, 2.5, 0.305, 0.31, 0.31, 0.31)); // NW.c
    e.extend(l.pact(14.0, 2.0, 15.0, 2.5));
    e.push(rect(l.tgo, 14.995, 1.8, 15.5, 2.7));
    e.push(l.nw_around(14.0, 2.0, 15.0, 2.5, 0.305, 0.31, 0.62, 0.62)); // NW.c + NW.c1
    write("NW.c.h7", e);

    // h8 — a 0.005 µm P+Activ sliver with a 0.305 left margin; a 300 µm P+Activ with a
    // 0.305 bottom margin (one violation); one at (1000, 1000).
    let mut e = l.pact(2.0, 2.0, 2.005, 2.5);
    e.push(l.nw_around(2.0, 2.0, 2.005, 2.5, 0.305, 0.31, 0.31, 0.31));
    e.extend(l.pact(2.0, 6.0, 302.0, 6.5));
    e.push(l.nw_around(2.0, 6.0, 302.0, 6.5, 0.31, 0.31, 0.305, 0.31));
    e.extend(inst(1000.0, 1000.0, 0.305, 0.31, 0.31, 0.31));
    write("NW.c.h8", e);
}

// --- NW.c1: NWell enclosure of P+Activ inside ThickGateOx 0.62 (0.31 inside DigiBnd) ---

fn nw_c1_h(l: &L) {
    let nw = l.nw;

    // One 0.5 × 0.5 P+Activ under a ThickGateOx patch at `(x, y)` with the margins given.
    let inst = |x: f64, y: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        let mut e = l.pact(x, y, x + 0.5, y + 0.5);
        e.push(rect(l.tgo, x - 0.1, y - 0.1, x + 0.6, y + 0.6));
        e.push(l.nw_around(x, y, x + 0.5, y + 0.5, ml, mr, mb, mt));
        e
    };

    // h1 — the bound, the corner and chapter 8.  0.62 is clean, 0.615 fires; a chamfer
    // along x + y = 1.87 passes 0.615 from the Activ corner (fires, both axis margins are
    // 1.0), 1.88 → 0.622 clean.  Inside a DigiBnd the value is 0.31: 0.31 clean, 0.305 →
    // NW.c1.dig.  "Inside DigiBnd" just failing: a DigiBnd edge cutting through the Activ,
    // and a DigiBnd frame with the device in its hole — neither device is inside DigiBnd,
    // so the 0.62 of section 5.1 applies and a 0.45 margin fires NW.c1.
    let cham = |x: f64, y: f64, k: f64| {
        let mut e = l.pact(x, y, x + 0.5, y + 0.5);
        e.push(rect(l.tgo, x - 0.1, y - 0.1, x + 0.6, y + 0.6));
        e.push(chamfered_tr(
            nw,
            x - 0.62,
            y - 0.62,
            x + 1.5,
            y + 1.5,
            x + y + k,
        ));
        e
    };
    let mut e = inst(2.0, 2.0, 0.62, 0.62, 0.62, 0.62); // clean
    e.extend(inst(6.0, 2.0, 0.62, 0.615, 0.62, 0.62)); // NW.c1
    e.extend(cham(10.0, 2.0, 1.87)); // NW.c1 (0.615)
    e.extend(cham(14.0, 2.0, 1.88)); // clean
    e.push(rect(l.digi, 1.0, 7.0, 9.0, 11.0));
    e.extend(inst(2.0, 8.0, 0.31, 0.31, 0.31, 0.31)); // clean (digital)
    e.extend(inst(6.0, 8.0, 0.31, 0.305, 0.31, 0.31)); // NW.c1.dig
    e.push(rect(l.digi, 12.0, 7.0, 14.25, 11.0));
    e.extend(inst(14.0, 8.0, 0.45, 0.45, 0.45, 0.45)); // NW.c1 (straddles the DigiBnd edge)
    e.push(l.digi_ring(18.5, 6.5, 21.5, 10.5));
    e.extend(inst(19.75, 8.0, 0.45, 0.45, 0.45, 0.45)); // NW.c1 (in the frame's hole)
    write("NW.c1.h1", e);

    // h2 — tile lines under one DigiBnd: 0.305 margins with the NWell left edge on x =
    // 20, the gap straddling 20, the Activ edge on 20, at 21, 40, 42 and inside at 10 →
    // seven NW.c1.dig; outside the DigiBnd a 0.615 margin straddling 20 → NW.c1.
    e = vec![rect(l.digi, 0.0, 0.0, 50.0, 20.0)];
    e.extend(inst(10.005, 10.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(20.305, 2.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(20.005, 6.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(20.0, 10.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(21.005, 14.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(40.005, 2.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(42.005, 6.31, 0.305, 0.31, 0.31, 0.31));
    e.extend(inst(20.005, 24.62, 0.615, 0.62, 0.62, 0.62));
    write("NW.c1.h2", e);

    // h3/h4 — fifty TGO P+Activ with a 0.615 left margin, flat and as an array reference.
    arrays("NW.c1", 3, inst(0.615, 0.62, 0.615, 0.62, 0.62, 0.62), 4.0);
}

// --- NW.d: NWell space to external N+Activ not inside ThickGateOx 0.31 ---

fn nw_d_h(l: &L) {
    let nw = l.nw;
    let activ = l.activ;

    // A 1 × 1 NWell at `(x, y)` and a 0.5 × 0.5 plain Activ (N+ by default) `gap` to its
    // right.
    let inst = |x: f64, y: f64, gap: f64| {
        vec![
            rect(nw, x, y, x + 1.0, y + 1.0),
            rect(activ, x + 1.0 + gap, y, x + 1.5 + gap, y + 0.5),
        ]
    };

    // h1 — the bound, both metrics and 45° geometry.  Gap 0.305 fires; a diagonal offset
    // 0.22/0.22 is 0.311 corner to corner (clean), 0.215/0.215 is 0.304 (fires); an Activ
    // diamond tip 0.305 from the well fires, 0.31 is clean; an Activ corner 0.304 from a
    // 45° well wall fires, 0.311 is clean; an Activ chamfer parallel to the well's 45°
    // wall at 0.304 fires.
    let mut e = inst(2.0, 2.0, 0.305); // NW.d
    e.push(rect(nw, 6.0, 2.0, 7.0, 3.0));
    e.push(rect(activ, 7.22, 3.22, 7.72, 3.72)); // clean (0.311)
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.push(rect(activ, 11.215, 3.215, 11.715, 3.715)); // NW.d (0.304)
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.push(diamond(activ, 15.805, 2.5, 0.5)); // NW.d (tip 0.305)
    e.push(rect(nw, 2.0, 6.0, 3.0, 7.0));
    e.push(diamond(activ, 3.81, 6.5, 0.5)); // clean (tip 0.31)
    e.push(slant_nw(nw, 6.0, 6.0));
    e.push(rect(activ, 7.5, 7.53, 8.0, 8.03)); // NW.d (corner 0.304 from the wall)
    e.push(slant_nw(nw, 10.0, 6.0));
    e.push(rect(activ, 11.5, 7.54, 12.0, 8.04)); // clean (0.311)
    e.push(slant_nw(nw, 14.0, 6.0));
    e.push(poly(
        activ,
        &[
            (15.78, 7.25),
            (16.0, 7.25),
            (16.0, 7.95),
            (15.3, 7.95),
            (15.3, 7.73),
        ],
    )); // NW.d (parallel, 0.304)
    write("NW.d.h1", e);

    // h2 — the rule's conditions and shapes that merge.  Plain Activ inside the well 0.30
    // from its edge is a tie, not external (NW.e is content with 0.24) → clean; P+Activ at
    // 0.305 is not N+ (NW.f is content with 0.24) → clean; Activ under nSD:block but with
    // drawn nSD is N+ → fires; N+Activ in the hole of a NWell ring 0.305 from the inner
    // wall fires, 0.31 is clean; N+Activ drawn as two abutting boxes at 0.305 fires once;
    // NWell drawn as two overlapping boxes, Activ 0.305 from the union's edge, fires; an
    // Activ in a U-notch 0.305 from one arm (0.395 from the other) fires once.
    let mut e = vec![
        rect(nw, 2.0, 2.0, 3.0, 3.0),
        rect(activ, 2.3, 2.3, 2.7, 2.7),
    ]; // clean
    e.push(rect(nw, 6.0, 2.0, 7.0, 3.0));
    e.extend(l.pact(7.305, 2.0, 7.805, 2.5)); // clean (P+)
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.extend(l.nact(11.305, 2.0, 11.805, 2.5));
    e.push(rect(l.nsd_block, 11.305, 2.0, 11.805, 2.5)); // NW.d (drawn nSD wins)
    e.extend(l.ring(14.0, 1.0, 18.0, 5.0, 15.0, 2.0, 17.0, 4.0));
    e.push(rect(activ, 15.305, 2.75, 15.805, 3.25)); // NW.d (0.305 in the hole)
    e.extend(l.ring(2.0, 7.0, 6.0, 11.0, 3.0, 8.0, 5.0, 10.0));
    e.push(rect(activ, 3.31, 8.75, 3.81, 9.25)); // clean (0.31 in the hole)
    e.push(rect(nw, 8.0, 7.0, 9.0, 8.0));
    e.push(rect(activ, 9.305, 7.0, 9.6, 7.5));
    e.push(rect(activ, 9.6, 7.0, 9.805, 7.5)); // NW.d (abutting boxes)
    e.push(rect(nw, 12.0, 7.0, 13.0, 8.0));
    e.push(rect(nw, 12.8, 7.0, 13.5, 8.0));
    e.push(rect(activ, 13.805, 7.0, 14.305, 7.5)); // NW.d (union edge)
    e.push(poly(
        nw,
        &[
            (2.0, 13.0),
            (5.0, 13.0),
            (5.0, 16.0),
            (4.2, 16.0),
            (4.2, 14.0),
            (3.0, 14.0),
            (3.0, 16.0),
            (2.0, 16.0),
        ],
    ));
    e.push(rect(activ, 3.305, 14.5, 3.805, 15.0)); // NW.d (0.305 to the left arm)
    write("NW.d.h2", e);

    // h3 — tile lines, gap 0.305: straddling 20, NWell edge on 20, Activ edge on 20,
    // straddling 21, 40, 42, inside at 10, and a 10 µm long gap running across 20/21.
    // Eight violations; the 0.31 control straddling 20 is clean.
    let mut e = inst(18.8, 2.0, 0.305);
    e.extend(inst(19.0, 6.0, 0.305));
    e.extend(inst(18.695, 10.0, 0.305));
    e.extend(inst(19.8, 14.0, 0.305));
    e.extend(inst(39.0, 2.0, 0.305));
    e.extend(inst(40.8, 6.0, 0.305));
    e.extend(inst(9.0, 2.0, 0.305));
    e.push(rect(nw, 15.0, 18.0, 25.0, 19.0));
    e.push(rect(activ, 15.0, 19.305, 25.0, 19.805));
    e.extend(inst(18.8, 22.0, 0.31)); // clean
    write("NW.d.h3", e);

    // h4/h5 — fifty wells with an N+Activ 0.305 away, flat and as an array reference.
    arrays("NW.d", 4, inst(0.2, 0.2, 0.305), 4.0);

    // h6 — a 0.005 µm Activ sliver 0.305 from a well; a 300 µm Activ 0.305 below a 300 µm
    // well (one violation); a pair at (1000, 1000).
    let mut e = vec![
        rect(nw, 2.0, 2.0, 3.0, 3.0),
        rect(activ, 3.305, 2.0, 3.31, 2.5),
        rect(nw, 2.0, 6.0, 302.0, 7.0),
        rect(activ, 2.0, 5.195, 302.0, 5.695),
    ];
    e.extend(inst(1000.0, 1000.0, 0.305));
    write("NW.d.h6", e);

    // h7 — ThickGateOx and chapter 8.  TGO over the top half of a 0.5 × 1.0 Activ 0.45
    // from the well: the TGO half is under NW.d1 (fires), the other half is clean at
    // 0.31; TGO abutting the Activ without overlap at 0.305 → NW.d only; TGO overlapping
    // by a 0.005 sliver at 0.305 → NW.d and NW.d1.  Inside a DigiBnd the TGO value is
    // 0.31: 0.31 clean, 0.305 → NW.d1.dig.  A DigiBnd edge through the Activ and a
    // DigiBnd frame with the Activ in its hole are not "inside DigiBnd": 0.45 fires
    // NW.d1.  A DigiBnd covering the Activ but not the well is inside: 0.45 is clean.
    let hv = |x: f64, y: f64, gap: f64| {
        vec![
            rect(nw, x, y, x + 1.0, y + 1.0),
            rect(activ, x + 1.0 + gap, y, x + 1.5 + gap, y + 0.5),
            rect(l.tgo, x + 0.9 + gap, y - 0.1, x + 1.6 + gap, y + 0.6),
        ]
    };
    let mut e = vec![
        rect(nw, 2.0, 2.0, 3.0, 3.0),
        rect(activ, 3.45, 2.0, 3.95, 3.0),
        rect(l.tgo, 3.35, 2.5, 4.05, 3.1), // NW.d1 (top half)
        rect(nw, 6.0, 2.0, 7.0, 3.0),
        rect(activ, 7.305, 2.0, 7.805, 2.5),
        rect(l.tgo, 7.805, 1.9, 8.3, 2.6), // NW.d (TGO abuts)
        rect(nw, 10.0, 2.0, 11.0, 3.0),
        rect(activ, 11.305, 2.0, 11.805, 2.5),
        rect(l.tgo, 11.2, 2.495, 11.9, 2.7), // NW.d + NW.d1 (top 0.005 of the Activ in TGO)
        rect(l.digi, 1.0, 5.5, 9.0, 8.5),
    ];
    e.extend(hv(2.0, 6.0, 0.31)); // clean (digital)
    e.extend(hv(6.0, 6.0, 0.305)); // NW.d1.dig
    e.push(rect(l.digi, 12.0, 5.5, 13.7, 8.5));
    e.extend(hv(12.0, 6.0, 0.45)); // NW.d1 (DigiBnd edge through the Activ)
    e.push(l.digi_ring(17.5, 5.5, 20.5, 8.5));
    e.extend(hv(18.0, 6.0, 0.45)); // NW.d1 (in the frame's hole)
    e.push(rect(l.digi, 3.3, 10.5, 5.0, 12.5));
    e.extend(hv(2.0, 11.0, 0.45)); // clean (Activ inside DigiBnd, well outside)
    write("NW.d.h7", e);
}

// --- NW.d1: NWell space to external N+Activ inside ThickGateOx 0.62 ---

fn nw_d1_h(l: &L) {
    let nw = l.nw;
    let activ = l.activ;
    let inst = |x: f64, y: f64, gap: f64| {
        vec![
            rect(nw, x, y, x + 1.0, y + 1.0),
            rect(activ, x + 1.0 + gap, y, x + 1.5 + gap, y + 0.5),
            rect(l.tgo, x + 0.9 + gap, y - 0.1, x + 1.6 + gap, y + 0.6),
        ]
    };

    // h1 — the bound and both metrics.  0.62 clean, 0.615 fires; diagonal 0.44/0.44 is
    // 0.622 (clean), 0.435/0.435 is 0.615 (fires); an Activ diamond tip at 0.615 fires.
    let mut e = inst(2.0, 2.0, 0.62); // clean
    e.extend(inst(6.0, 2.0, 0.615)); // NW.d1
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.push(rect(activ, 11.44, 3.44, 11.94, 3.94));
    e.push(rect(l.tgo, 11.34, 3.34, 12.04, 4.04)); // clean (0.622)
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.push(rect(activ, 15.435, 3.435, 15.935, 3.935));
    e.push(rect(l.tgo, 15.335, 3.335, 16.035, 4.035)); // NW.d1 (0.615)
    e.push(rect(nw, 2.0, 6.0, 3.0, 7.0));
    e.push(diamond(activ, 4.115, 6.5, 0.5));
    e.push(rect(l.tgo, 3.5, 5.9, 4.7, 7.1)); // NW.d1 (tip 0.615)
    write("NW.d1.h1", e);

    // h2/h3 — fifty wells with a TGO N+Activ 0.615 away, flat and as an array reference.
    arrays("NW.d1", 2, inst(0.2, 0.2, 0.615), 4.0);
}

// --- NW.e: NWell enclosure of the NWell tie (N+Activ surrounded entirely by NWell) 0.24 ---

fn nw_e_h(l: &L) {
    let nw = l.nw;
    let activ = l.activ;

    // One 0.5 × 0.5 plain-Activ tie at `(x, y)` with the NWell margins given.
    let inst = |x: f64, y: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        vec![
            rect(activ, x, y, x + 0.5, y + 0.5),
            l.nw_around(x, y, x + 0.5, y + 0.5, ml, mr, mb, mt),
        ]
    };

    // h1 — the bound and both metrics.  0.24 clean, 0.235 fires; a NWell chamfer along
    // x + y = 1.33 (Activ-relative) passes 0.233 from the tie's corner (fires, both axis
    // margins are 0.3), 1.34 → 0.240 clean; the same 1.33 as a full 45° wall fires; an
    // Activ chamfer along x + y = 1.7 facing a parallel NWell chamfer at 2.03 → 0.233
    // fires, 2.04 → 0.240 clean.
    let cham = |x: f64, y: f64, k: f64| {
        vec![
            rect(activ, x, y, x + 0.5, y + 0.5),
            chamfered_tr(nw, x - 0.24, y - 0.24, x + 0.8, y + 0.8, x + y + k),
        ]
    };
    let par = |x: f64, y: f64, k: f64| {
        vec![
            poly(
                activ,
                &[
                    (x, y),
                    (x + 1.0, y),
                    (x + 1.0, y + 0.7),
                    (x + 0.7, y + 1.0),
                    (x, y + 1.0),
                ],
            ),
            chamfered_tr(nw, x - 0.24, y - 0.24, x + 1.24, y + 1.24, x + y + k),
        ]
    };
    let mut e = inst(2.0, 2.0, 0.24, 0.24, 0.24, 0.24); // clean
    e.extend(inst(6.0, 2.0, 0.24, 0.235, 0.24, 0.24)); // NW.e
    e.extend(cham(10.0, 2.0, 1.33)); // NW.e (0.233)
    e.extend(cham(14.0, 2.0, 1.34)); // clean (0.240)
    e.push(rect(activ, 2.0, 6.0, 2.5, 6.5));
    e.push(poly(
        nw,
        &[
            (1.76, 5.5),
            (2.74, 5.5),
            (2.74, 6.59),
            (2.53, 6.8),
            (1.76, 6.8),
        ],
    )); // NW.e (45° wall, 0.233)
    e.extend(par(6.0, 6.0, 2.03)); // NW.e (0.233 between parallel chamfers)
    e.extend(par(10.0, 6.0, 2.04)); // clean (0.240)
    write("NW.e.h1", e);

    // h2 — the rule's conditions and shapes that merge.  P+Activ at 0.20 is NW.c's
    // (fires NW.c, not NW.e); a tie with drawn nSD at 0.235 fires; a tie in a ring's body
    // 0.235 from the hole fires, at 0.24 it is clean; a tie drawn as two overlapping boxes
    // with a 0.235 union margin fires once; a NWell drawn as two overlapping boxes, each
    // of which alone would cut the tie, encloses it by 0.24 (clean) or 0.235 (fires).
    let mut e = vec![rect(nw, 10.0, 2.0, 11.0, 3.0)];
    e.extend(l.pact(10.2, 2.3, 10.7, 2.7)); // NW.c, not NW.e
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.extend(l.nact(14.235, 2.3, 14.735, 2.7)); // NW.e
    e.extend(l.ring(2.0, 6.0, 6.0, 10.0, 3.0, 7.0, 5.0, 9.0));
    e.push(rect(activ, 2.3, 7.5, 2.765, 8.0)); // NW.e (0.235 to the hole)
    e.extend(l.ring(8.0, 6.0, 12.0, 10.0, 8.98, 7.0, 11.0, 9.0));
    e.push(rect(activ, 8.24, 7.5, 8.74, 8.0)); // clean (0.24 both ways)
    e.push(rect(nw, 14.0, 6.0, 15.235, 7.0));
    e.push(rect(activ, 14.3, 6.3, 14.7, 6.7));
    e.push(rect(activ, 14.6, 6.3, 15.0, 6.7)); // NW.e (union margin 0.235)
    e.push(rect(nw, 2.0, 12.0, 2.9, 13.0));
    e.push(rect(nw, 2.7, 12.0, 3.5, 13.0));
    e.push(rect(activ, 2.4, 12.3, 3.26, 12.7)); // clean (union margin 0.24)
    e.push(rect(nw, 6.0, 12.0, 6.9, 13.0));
    e.push(rect(nw, 6.7, 12.0, 7.495, 13.0));
    e.push(rect(activ, 6.4, 12.3, 7.26, 12.7)); // NW.e (union margin 0.235)
    write("NW.e.h2", e);

    // h3 — tile lines, one margin 0.235: NWell right edge on x = 20, the margin straddling
    // 20, the tie's edge on 20, at 21, 40, 42, inside at 10; a 10 µm tie with a 0.235
    // bottom margin across 20/21.  Eight violations; the 0.24 control is clean.
    let mut e = inst(19.265, 2.24, 0.24, 0.235, 0.24, 0.24);
    e.extend(inst(19.4, 6.24, 0.24, 0.235, 0.24, 0.24));
    e.extend(inst(19.5, 10.24, 0.24, 0.235, 0.24, 0.24));
    e.extend(inst(20.4, 14.24, 0.24, 0.235, 0.24, 0.24));
    e.extend(inst(39.265, 2.24, 0.24, 0.235, 0.24, 0.24));
    e.extend(inst(41.4, 6.24, 0.24, 0.235, 0.24, 0.24));
    e.extend(inst(9.5, 2.24, 0.24, 0.235, 0.24, 0.24));
    e.push(rect(activ, 15.24, 18.24, 24.76, 18.74));
    e.push(l.nw_around(15.24, 18.24, 24.76, 18.74, 0.24, 0.24, 0.235, 0.24));
    e.extend(inst(19.4, 22.24, 0.24, 0.24, 0.24, 0.24)); // clean
    write("NW.e.h3", e);

    // h4/h5 — fifty ties with a 0.235 right margin, flat and as an array reference.
    arrays("NW.e", 4, inst(0.24, 0.24, 0.24, 0.235, 0.24, 0.24), 3.0);

    // h6 — a 0.005 µm tie with a 0.235 left margin; a 300 µm tie with a 0.235 bottom
    // margin (one violation); one at (1000, 1000).
    let mut e = vec![
        rect(activ, 2.0, 2.0, 2.005, 2.5),
        l.nw_around(2.0, 2.0, 2.005, 2.5, 0.235, 0.24, 0.24, 0.24),
        rect(activ, 2.0, 6.0, 302.0, 6.5),
        l.nw_around(2.0, 6.0, 302.0, 6.5, 0.24, 0.24, 0.235, 0.24),
    ];
    e.extend(inst(1000.0, 1000.0, 0.235, 0.24, 0.24, 0.24));
    write("NW.e.h6", e);

    // h7 — ThickGateOx and chapter 8.  TGO over the top half of a 0.5 × 1.0 tie with 0.45
    // above it: the TGO half is under NW.e1 (fires), the bottom half is clean at 0.24; TGO
    // abutting the tie's top edge, top margin 0.235 → NW.e only; TGO over the top 0.005 of
    // the tie, NWell 0.23 above it (0.235 above the untouched part) → NW.e and NW.e1.
    // Inside a DigiBnd the TGO value is
    // 0.24: 0.24 clean, 0.235 → NW.e1.dig.  A DigiBnd edge through the tie and a DigiBnd
    // frame with the tie in its hole are not "inside DigiBnd": 0.45 fires NW.e1.
    let hv = |x: f64, y: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        let mut e = inst(x, y, ml, mr, mb, mt);
        e.push(rect(l.tgo, x - 0.1, y - 0.1, x + 0.6, y + 0.6));
        e
    };
    let mut e = vec![
        rect(activ, 2.0, 2.0, 2.5, 3.0),
        rect(l.tgo, 1.9, 2.5, 2.6, 3.1),
        l.nw_around(2.0, 2.0, 2.5, 3.0, 0.62, 0.62, 0.24, 0.45), // NW.e1 (top half)
        rect(activ, 6.0, 2.0, 6.5, 2.5),
        rect(l.tgo, 5.9, 2.5, 6.6, 2.7),
        l.nw_around(6.0, 2.0, 6.5, 2.5, 0.24, 0.24, 0.24, 0.235), // NW.e (TGO abuts)
        rect(activ, 10.0, 2.0, 10.5, 2.5),
        rect(l.tgo, 9.9, 2.495, 10.6, 2.7),
        l.nw_around(10.0, 2.0, 10.5, 2.5, 0.62, 0.62, 0.24, 0.23), // NW.e + NW.e1
        rect(l.digi, 1.0, 5.5, 9.0, 8.5),
    ];
    e.extend(hv(2.0, 6.0, 0.24, 0.24, 0.24, 0.24)); // clean (digital)
    e.extend(hv(6.0, 6.0, 0.24, 0.235, 0.24, 0.24)); // NW.e1.dig
    e.push(rect(l.digi, 12.0, 5.5, 14.25, 8.5));
    e.extend(hv(14.0, 6.0, 0.45, 0.45, 0.45, 0.45)); // NW.e1 (DigiBnd edge through the tie)
    e.push(l.digi_ring(18.5, 5.0, 21.5, 9.0));
    e.extend(hv(19.75, 6.0, 0.45, 0.45, 0.45, 0.45)); // NW.e1 (in the frame's hole)
    write("NW.e.h7", e);

    // h8 — a tie whose right edge lies on the NWell edge: it is inside the well (surrounded
    // entirely, nothing crosses out) with zero enclosure on that side → NW.e.
    write(
        "NW.e.h8",
        vec![
            rect(nw, 2.0, 2.0, 3.0, 3.0),
            rect(activ, 2.3, 2.3, 3.0, 2.7),
        ],
    );

    // h9 — Activ under nSD:block with neither nSD nor pSD drawn, 0.20 from the well edge.
    // Section 4.2: N+Activ is Activ AND nSD with nSD = NOT (pSD OR nSD:block) OR
    // nSD:drawing, and P+Activ is Activ AND pSD - this Activ is neither, so it is no NWell
    // tie for NW.e and no P+Activ for NW.c → clean.
    write(
        "NW.e.h9",
        vec![
            rect(nw, 6.0, 2.0, 7.0, 3.0),
            rect(activ, 6.2, 2.3, 6.7, 2.7),
            rect(l.nsd_block, 6.2, 2.3, 6.7, 2.7),
        ],
    );
}

// --- NW.e1: NWell enclosure of the NWell tie inside ThickGateOx 0.62 ---

fn nw_e1_h(l: &L) {
    let nw = l.nw;
    let activ = l.activ;
    let inst = |x: f64, y: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        vec![
            rect(activ, x, y, x + 0.5, y + 0.5),
            rect(l.tgo, x - 0.1, y - 0.1, x + 0.6, y + 0.6),
            l.nw_around(x, y, x + 0.5, y + 0.5, ml, mr, mb, mt),
        ]
    };

    // h1 — the bound and the corner.  0.62 clean, 0.615 fires; a chamfer along x + y =
    // 1.87 passes 0.615 from the tie's corner (fires), 1.88 → 0.622 clean.
    let cham = |x: f64, y: f64, k: f64| {
        vec![
            rect(activ, x, y, x + 0.5, y + 0.5),
            rect(l.tgo, x - 0.1, y - 0.1, x + 0.6, y + 0.6),
            chamfered_tr(nw, x - 0.62, y - 0.62, x + 1.5, y + 1.5, x + y + k),
        ]
    };
    let mut e = inst(2.0, 2.0, 0.62, 0.62, 0.62, 0.62); // clean
    e.extend(inst(6.0, 2.0, 0.62, 0.615, 0.62, 0.62)); // NW.e1
    e.extend(cham(10.0, 2.0, 1.87)); // NW.e1
    e.extend(cham(14.0, 2.0, 1.88)); // clean
    write("NW.e1.h1", e);

    // h2/h3 — fifty TGO ties with a 0.615 right margin, flat and as an array reference.
    arrays("NW.e1", 2, inst(0.62, 0.62, 0.62, 0.615, 0.62, 0.62), 4.0);
}

// --- NW.f: NWell space to the substrate tie (P+Activ in PWell) not inside ThickGateOx 0.24 ---

fn nw_f_h(l: &L) {
    let nw = l.nw;

    // A 1 × 1 NWell at `(x, y)` and a 0.5 × 0.5 P+Activ `gap` to its right.
    let inst = |x: f64, y: f64, gap: f64| {
        let mut e = vec![rect(nw, x, y, x + 1.0, y + 1.0)];
        e.extend(l.pact(x + 1.0 + gap, y, x + 1.5 + gap, y + 0.5));
        e
    };

    // h1 — the bound, both metrics and 45° geometry.  0.24 clean, 0.235 fires; diagonal
    // 0.17/0.17 is 0.240 corner to corner (clean), 0.165/0.165 is 0.233 (fires); a
    // P+Activ diamond tip at 0.235 fires, 0.24 is clean; a P+Activ corner 0.233 from a
    // 45° well wall fires, 0.240 is clean.
    let mut e = inst(2.0, 2.0, 0.24); // clean
    e.extend(inst(6.0, 2.0, 0.235)); // NW.f
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.extend(l.pact(11.17, 3.17, 11.67, 3.67)); // clean (0.240)
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.extend(l.pact(15.165, 3.165, 15.665, 3.665)); // NW.f (0.233)
    e.push(rect(nw, 2.0, 6.0, 3.0, 7.0));
    e.push(diamond(l.activ, 3.735, 6.5, 0.5));
    e.push(diamond(l.psd, 3.735, 6.5, 0.5)); // NW.f (tip 0.235)
    e.push(rect(nw, 6.0, 6.0, 7.0, 7.0));
    e.push(diamond(l.activ, 7.74, 6.5, 0.5));
    e.push(diamond(l.psd, 7.74, 6.5, 0.5)); // clean (tip 0.24)
    e.push(slant_nw(nw, 10.0, 6.0));
    e.extend(l.pact(11.5, 7.43, 12.0, 7.93)); // NW.f (corner 0.233 from the wall)
    e.push(slant_nw(nw, 14.0, 6.0));
    e.extend(l.pact(15.5, 7.44, 16.0, 7.94)); // clean (0.240)
    write("NW.f.h1", e);

    // h2 — the rule's conditions and shapes that merge.  P+Activ inside the well 0.235
    // from its edge is NW.c's (fires NW.c, not NW.f); plain Activ at 0.235 is N+ and
    // NW.d's (fires NW.d, not NW.f); P+Activ under PWell:block is not in PWell (section
    // 4.2: PWell is NOT (NWell OR PWell:block)), so it is no substrate tie → clean; with
    // PWell:block over its right half only, the left half is a tie at 0.235 → fires;
    // P+Activ in a ring's hole at 0.235 fires, at 0.24 clean; in a U-notch 0.235 from one
    // arm fires once; drawn as two abutting boxes against a well drawn as two overlapping
    // boxes, 0.235 from the union, fires once.
    let mut e = vec![rect(nw, 2.0, 2.0, 3.0, 3.0)];
    e.extend(l.pact(2.235, 2.3, 2.735, 2.7)); // NW.c, not NW.f
    e.push(rect(nw, 6.0, 2.0, 7.0, 3.0));
    e.push(rect(l.activ, 7.235, 2.0, 7.735, 2.5)); // NW.d, not NW.f
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.push(rect(l.pwb, 11.1, 1.8, 12.5, 3.2));
    e.extend(l.pact(11.235, 2.0, 11.735, 2.5)); // clean (in PWell:block)
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.push(rect(l.pwb, 15.5, 1.8, 16.5, 3.2));
    e.extend(l.pact(15.235, 2.0, 15.735, 2.5)); // NW.f (left half in PWell)
    e.extend(l.ring(2.0, 6.0, 6.0, 10.0, 3.0, 7.0, 5.0, 9.0));
    e.extend(l.pact(3.235, 7.75, 3.735, 8.25)); // NW.f (0.235 in the hole)
    e.extend(l.ring(8.0, 6.0, 12.0, 10.0, 9.0, 7.0, 11.0, 9.0));
    e.extend(l.pact(9.24, 7.75, 9.74, 8.25)); // clean (0.24 in the hole)
    e.push(poly(
        nw,
        &[
            (14.0, 6.0),
            (17.0, 6.0),
            (17.0, 9.0),
            (16.2, 9.0),
            (16.2, 7.0),
            (15.0, 7.0),
            (15.0, 9.0),
            (14.0, 9.0),
        ],
    ));
    e.extend(l.pact(15.235, 7.5, 15.735, 8.0)); // NW.f (0.235 to the left arm)
    e.push(rect(nw, 2.0, 12.0, 3.0, 13.0));
    e.push(rect(nw, 2.8, 12.0, 3.5, 13.0));
    e.extend(l.pact(3.735, 12.0, 4.0, 12.5));
    e.extend(l.pact(4.0, 12.0, 4.235, 12.5)); // NW.f (union to union)
    write("NW.f.h2", e);

    // h3 — tile lines, gap 0.235: straddling 20, NWell edge on 20, Activ edge on 20,
    // straddling 21, 40, 42, inside at 10, and a 10 µm gap running across 20/21.  Eight
    // violations; the 0.24 control straddling 20 is clean.
    let mut e = inst(18.9, 2.0, 0.235);
    e.extend(inst(19.0, 6.0, 0.235));
    e.extend(inst(18.765, 10.0, 0.235));
    e.extend(inst(19.9, 14.0, 0.235));
    e.extend(inst(39.0, 2.0, 0.235));
    e.extend(inst(40.9, 6.0, 0.235));
    e.extend(inst(9.0, 2.0, 0.235));
    e.push(rect(nw, 15.0, 18.0, 25.0, 19.0));
    e.extend(l.pact(15.0, 19.235, 25.0, 19.735));
    e.extend(inst(18.9, 22.0, 0.24)); // clean
    write("NW.f.h3", e);

    // h4/h5 — fifty wells with a substrate tie 0.235 away, flat and as an array reference.
    arrays("NW.f", 4, inst(0.2, 0.2, 0.235), 4.0);

    // h6 — a 0.005 µm P+Activ sliver 0.235 from a well; a 300 µm tie 0.235 below a 300 µm
    // well (one violation); a pair at (1000, 1000).
    let mut e = vec![rect(nw, 2.0, 2.0, 3.0, 3.0)];
    e.extend(l.pact(3.235, 2.0, 3.24, 2.5));
    e.push(rect(nw, 2.0, 6.0, 302.0, 7.0));
    e.extend(l.pact(2.0, 5.265, 302.0, 5.765));
    e.extend(inst(1000.0, 1000.0, 0.235));
    write("NW.f.h6", e);

    // h7 — ThickGateOx and chapter 8.  TGO over the top half of a 0.5 × 1.0 tie 0.45 from
    // the well: the TGO half is under NW.f1 (fires), the other half is clean at 0.24;
    // TGO abutting the tie at 0.235 → NW.f only; TGO over the top 0.005 of the tie at
    // 0.235 → NW.f and NW.f1.  Inside a DigiBnd section 8.1.1 relaxes NW.f1 to 0.24: a
    // TGO tie at 0.30 is clean and at 0.235 fires the digital variant (NW.f1.dig by the
    // deck's naming).  A DigiBnd edge through the tie and a DigiBnd frame with the tie in
    // its hole are not "inside DigiBnd": 0.45 fires NW.f1.
    let hv = |x: f64, y: f64, gap: f64| {
        let mut e = inst(x, y, gap);
        e.push(rect(l.tgo, x + 0.9 + gap, y - 0.1, x + 1.6 + gap, y + 0.6));
        e
    };
    let mut e = vec![rect(nw, 2.0, 2.0, 3.0, 3.0)];
    e.extend(l.pact(3.45, 2.0, 3.95, 3.0));
    e.push(rect(l.tgo, 3.35, 2.5, 4.05, 3.1)); // NW.f1 (top half)
    e.push(rect(nw, 6.0, 2.0, 7.0, 3.0));
    e.extend(l.pact(7.235, 2.0, 7.735, 2.5));
    e.push(rect(l.tgo, 7.735, 1.9, 8.3, 2.6)); // NW.f (TGO abuts)
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.extend(l.pact(11.235, 2.0, 11.735, 2.5));
    e.push(rect(l.tgo, 11.1, 2.495, 11.9, 2.7)); // NW.f + NW.f1
    e.push(rect(l.digi, 1.0, 5.5, 9.0, 8.5));
    e.extend(hv(2.0, 6.0, 0.30)); // clean (digital, 0.24 applies)
    e.extend(hv(6.0, 6.0, 0.235)); // NW.f1.dig
    e.push(rect(l.digi, 12.0, 5.5, 13.7, 8.5));
    e.extend(hv(12.0, 6.0, 0.45)); // NW.f1 (DigiBnd edge through the tie)
    e.push(l.digi_ring(17.5, 5.5, 20.5, 8.5));
    e.extend(hv(18.0, 6.0, 0.45)); // NW.f1 (in the frame's hole)
    write("NW.f.h7", e);
}

// --- NW.f1: NWell space to the substrate tie inside ThickGateOx 0.62 ---

fn nw_f1_h(l: &L) {
    let nw = l.nw;
    let inst = |x: f64, y: f64, gap: f64| {
        let mut e = vec![rect(nw, x, y, x + 1.0, y + 1.0)];
        e.extend(l.pact(x + 1.0 + gap, y, x + 1.5 + gap, y + 0.5));
        e.push(rect(l.tgo, x + 0.9 + gap, y - 0.1, x + 1.6 + gap, y + 0.6));
        e
    };

    // h1 — the bound and both metrics.  0.62 clean, 0.615 fires; diagonal 0.44/0.44 is
    // 0.622 (clean), 0.435/0.435 is 0.615 (fires); a tie diamond tip at 0.615 fires.
    let mut e = inst(2.0, 2.0, 0.62); // clean
    e.extend(inst(6.0, 2.0, 0.615)); // NW.f1
    e.push(rect(nw, 10.0, 2.0, 11.0, 3.0));
    e.extend(l.pact(11.44, 3.44, 11.94, 3.94));
    e.push(rect(l.tgo, 11.34, 3.34, 12.04, 4.04)); // clean (0.622)
    e.push(rect(nw, 14.0, 2.0, 15.0, 3.0));
    e.extend(l.pact(15.435, 3.435, 15.935, 3.935));
    e.push(rect(l.tgo, 15.335, 3.335, 16.035, 4.035)); // NW.f1 (0.615)
    e.push(rect(nw, 2.0, 6.0, 3.0, 7.0));
    e.push(diamond(l.activ, 4.115, 6.5, 0.5));
    e.push(diamond(l.psd, 4.115, 6.5, 0.5));
    e.push(rect(l.tgo, 3.5, 5.9, 4.7, 7.1)); // NW.f1 (tip 0.615)
    write("NW.f1.h1", e);

    // h2/h3 — fifty wells with a TGO substrate tie 0.615 away, flat and as an array
    // reference.
    arrays("NW.f1", 2, inst(0.2, 0.2, 0.615), 4.0);
}
