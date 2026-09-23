// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    cont_at, density_pattern, layer, library, max_width_pattern, min_width_pattern, notch_pattern,
    poly, rect, space_pattern, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;
use std::f64::consts::SQRT_2;

const DIR: &str = "tests/data/ihp-sg13g2/gatpoly";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    gat_a(pdk);
    gat_a1(pdk);
    gat_a2(pdk);
    gat_a3(pdk);
    gat_a4(pdk);
    gat_b_space(pdk);
    gat_b_notch(pdk);
    gat_b1(pdk);
    gat_c(pdk);
    gat_d(pdk);
    gat_e(pdk);
    gat_f(pdk);
    gat_g(pdk);
    gfil_a(pdk);
    gfil_b(pdk);
    gfil_c(pdk);
    gfil_d(pdk);
    gfil_e_nwell(pdk);
    gfil_e_nbulay(pdk);
    gfil_f(pdk);
    gfil_g(pdk);
    gfil_g_boundary(pdk);
    gfil_i(pdk);
    gfil_j(pdk);
    hardening(pdk);
}

/// GFil.a — max. GatPoly:filler width 5.00 µm.
/// GFil.a — max. GatPoly:filler width 5.00, read as the narrowest dimension: a 5.005 ×
/// 5.0 and a 5.0 × 5.005 filler are 5.0 wide and clean, a 5.005 × 5.005 one is not.
fn gfil_a(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let mut elems = max_width_pattern(l, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    elems.push(rect(l, OFFSET + 60.0, 0.0, OFFSET + 65.005, 5.005));
    write_gz(&format!("{DIR}/GFil.a.gds.gz"), library("TOP", elems));
}

/// GFil.b — min. GatPoly:filler width 0.70 µm.
fn gfil_b(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let elems = min_width_pattern(l, 0.70, 0.70, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.b.gds.gz"), library("TOP", elems));
}

/// GFil.c — min. GatPoly:filler space 0.80 µm.  1 µm shapes clear the width limits.
fn gfil_c(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let elems = space_pattern(l, l, 1.0, 0.80, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.c.gds.gz"), library("TOP", elems));
}

/// GFil.e — min. GatPoly:filler space to NWell 1.10 µm.
fn gfil_e_nwell(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.e.nwell.gds.gz"), library("TOP", elems));
}

/// GFil.e — min. GatPoly:filler space to nBuLay 1.10 µm.
fn gfil_e_nbulay(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let nbl = layer(pdk, "nBuLay");
    let elems = space_pattern(l, nbl, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{DIR}/GFil.e.nbulay.gds.gz"),
        library("TOP", elems),
    );
}

/// GFil.f — min. GatPoly:filler space to TRANS 1.10 µm.
fn gfil_f(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(l, trans, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/GFil.f.gds.gz"), library("TOP", elems));
}

/// GFil.i — max. GatPoly:nofill area 160000 µm² (400×400).  A 410×410 region violates;
/// a 399×399 region is clean.
fn gfil_i(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.nofill");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 410.0, o + 410.0), // 168100 µm² → violation
        rect(l, o + 500.0, o, o + 500.0 + 399.0, o + 399.0), // 159201 µm² → clean
    ];
    write_gz(&format!("{DIR}/GFil.i.gds.gz"), library("TOP", elems));
}

/// GFil.j — GatPoly:filler endcap: must extend ≥ 0.18 µm over Activ:filler.  A 0.8 µm
/// gate-filler over an Activ:filler extending 0.18 (clean) and 0.17 (violation).  The
/// gate is 0.8 µm wide so it clears the 0.70 µm GFil.b min width.
fn gfil_j(pdk: &PdkConfig) {
    let gf = layer(pdk, "GatPoly.filler");
    let af = layer(pdk, "Activ.filler");
    let o = OFFSET;
    let elems = vec![
        rect(af, o, o, o + 1.0, o + 1.0),
        rect(gf, o + 0.1, o - 0.18, o + 0.9, o + 1.18), // extends 0.18 → clean
        rect(af, o + 3.0, o, o + 4.0, o + 1.0),
        rect(gf, o + 3.1, o - 0.18, o + 3.9, o + 1.17), // extends 0.17 on top → violation
    ];
    write_gz(&format!("{DIR}/GFil.j.gds.gz"), library("TOP", elems));
}

fn gat_a(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = min_width_pattern(l, 0.13, 0.13, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.a.gds.gz"), library("TOP", elems));
}

fn gat_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = space_pattern(l, l, 1.0, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.b.space.gds.gz"), library("TOP", elems));
}

fn gat_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let elems = notch_pattern(l, 0.15, 0.18, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.b.notch.gds.gz"), library("TOP", elems));
}

/// Gat.b1 — two 3.3 V gate fingers (GatPoly over Activ under ThickGateOx) spaced 0.2 µm
/// (< 0.25, but ≥ 0.18 so the general Gat.b stays clean) → Gat.b1.  No implant, so the NFET/
/// PFET gate-length rules don't apply.
fn gat_b1(pdk: &PdkConfig) {
    let o = OFFSET;
    let elems = vec![
        rect(layer(pdk, "Activ"), o, o, o + 3.0, o + 1.0),
        rect(
            layer(pdk, "ThickGateOx"),
            o - 0.2,
            o - 0.2,
            o + 3.2,
            o + 1.2,
        ),
        rect(layer(pdk, "GatPoly"), o + 1.0, o - 0.3, o + 1.3, o + 1.3),
        rect(layer(pdk, "GatPoly"), o + 1.5, o - 0.3, o + 1.8, o + 1.3),
    ];
    write_gz(&format!("{DIR}/Gat.b1.gds.gz"), library("TOP", elems));
}

fn gat_d(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let act = layer(pdk, "Activ");
    let elems = space_pattern(l, act, 1.0, 0.07, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Gat.d.gds.gz"), library("TOP", elems));
}

/// A transistor at `(x, OFFSET)`: a 2×1 µm Activ + implant `imp` (optionally covered by
/// ThickGateOx), crossed by a vertical GatPoly gate of length `gl` extending 0.3 µm past
/// the Activ top/bottom.
fn gate_device(pdk: &PdkConfig, imp: &str, x: f64, gl: f64, tgo: bool) -> Vec<GdsElement> {
    let o = OFFSET;
    let gx = x + 0.6;
    let mut e = vec![
        rect(layer(pdk, "Activ"), x, o, x + 2.0, o + 1.0),
        rect(layer(pdk, imp), x, o, x + 2.0, o + 1.0),
        rect(layer(pdk, "GatPoly"), gx, o - 0.3, gx + gl, o + 1.3),
    ];
    if tgo {
        e.push(rect(
            layer(pdk, "ThickGateOx"),
            x - 0.1,
            o - 0.4,
            x + 2.1,
            o + 1.4,
        ));
    }
    e
}

/// Gat.a1 — gate length of a 1.2 V NFET (nSD channel, no ThickGateOx) ≥ 0.13 µm.
fn gat_a1(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "nSD", OFFSET, 0.13, false);
    e.extend(gate_device(pdk, "nSD", OFFSET + 5.0, 0.12, false));
    write_gz(&format!("{DIR}/Gat.a1.gds.gz"), library("TOP", e));
}

/// Gat.a2 — gate length of a 1.2 V PFET (pSD channel, no ThickGateOx) ≥ 0.13 µm.
fn gat_a2(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "pSD", OFFSET, 0.13, false);
    e.extend(gate_device(pdk, "pSD", OFFSET + 5.0, 0.12, false));
    write_gz(&format!("{DIR}/Gat.a2.gds.gz"), library("TOP", e));
}

/// Gat.a3 — gate length of a 3.3 V NFET (nSD channel inside ThickGateOx) ≥ 0.45 µm.
fn gat_a3(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "nSD", OFFSET, 0.45, true);
    e.extend(gate_device(pdk, "nSD", OFFSET + 5.0, 0.44, true));
    write_gz(&format!("{DIR}/Gat.a3.gds.gz"), library("TOP", e));
}

/// Gat.a4 — gate length of a 3.3 V PFET (pSD channel inside ThickGateOx) ≥ 0.40 µm.
fn gat_a4(pdk: &PdkConfig) {
    let mut e = gate_device(pdk, "pSD", OFFSET, 0.40, true);
    e.extend(gate_device(pdk, "pSD", OFFSET + 5.0, 0.39, true));
    write_gz(&format!("{DIR}/Gat.a4.gds.gz"), library("TOP", e));
}

/// Gat.c — GatPoly endcap: must extend ≥ 0.18 µm over Activ.  A gate extending 0.18
/// (clean) and one extending only 0.17 µm on top (violation).
fn gat_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(gp, o + 0.3, o - 0.18, o + 0.7, o + 1.18),
        rect(activ, o + 3.0, o, o + 4.0, o + 1.0),
        rect(gp, o + 3.3, o - 0.18, o + 3.7, o + 1.17),
    ];
    write_gz(&format!("{DIR}/Gat.c.gds.gz"), library("TOP", elems));
}

/// Gat.e — min. GatPoly area 0.09 µm².  A 0.2×0.2 = 0.04 µm² region violates; a 0.4×0.4
/// = 0.16 µm² region is clean.  Both stay above the 0.13 µm min width.
fn gat_e(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        rect(l, o, o, o + 0.2, o + 0.2),       // 0.04 µm² → violation
        rect(l, o + 2.0, o, o + 2.4, o + 0.4), // 0.16 µm² → clean
    ];
    write_gz(&format!("{DIR}/Gat.e.gds.gz"), library("TOP", elems));
}

/// Gat.f — no 45° GatPoly over Activ.  A straight orthogonal gate crossing an Activ (only
/// 90° edges over the channel → clean) and a 45°-running gate crossing an Activ, whose two
/// diagonal sides cross the channel (2 violations).
fn gat_f(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let mut elems = vec![
        rect(activ, o, o, o + 1.0, o + 1.0),
        rect(gp, o + 0.4, o - 0.3, o + 0.6, o + 1.3), // straight → orthogonal → clean
        rect(activ, o + 3.0, o, o + 5.0, o + 1.0),
    ];
    // 45° gate fully crossing the Activ: its horizontal ends sit outside the channel, so
    // only the two diagonal sides are over the Activ → exactly two forbidden-angle edges.
    elems.push(band(gp, o + 3.6, o - 0.3, 0.2, 2.3));
    write_gz(&format!("{DIR}/Gat.f.gds.gz"), library("TOP", elems));
}

/// A 45°-bent GatPoly band: perpendicular width `w`, run `run`, snapped to the grid.
fn band(l: (i16, i16), x: f64, y: f64, w: f64, run: f64) -> GdsElement {
    const GRID: f64 = 0.005;
    let snap = |v: f64| (v / GRID).round() * GRID;
    let wt = snap(w * SQRT_2);
    let h = snap(run / SQRT_2);
    poly(
        l,
        &[(x, y), (x + wt, y), (x + wt + h, y + h), (x + h, y + h)],
    )
}

/// Gat.g — min. 45°-bent GatPoly width 0.16 µm where the bent run > 0.39 µm.  A 0.15 µm
/// band with a 0.6 µm run fails on both walls; a 0.20 µm band and a 0.30 µm-run band are
/// clean.
fn gat_g(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        band(l, o, o, 0.15, 0.6),         // narrow + long run → 2 violations
        band(l, o + 5.0, o, 0.20, 0.6),   // wide enough → clean
        band(l, o + 10.0, o, 0.15, 0.30), // short run → clean (bent-length gate)
    ];
    write_gz(&format!("{DIR}/Gat.g.gds.gz"), library("TOP", elems));
}

fn gfil_d(pdk: &PdkConfig) {
    let l = layer(pdk, "GatPoly.filler");
    let mut elems = vec![];
    let layers = vec!["Activ", "GatPoly", "Cont", "pSD", "nSD.block", "SalBlock"];
    for (i, name) in layers.into_iter().enumerate() {
        elems.append(&mut space_pattern(
            l,
            layer(pdk, name),
            1.0,
            1.10,
            i as f64 * OFFSET,
            SPACE_DELTA,
        ));
    }
    write_gz(&format!("{DIR}/GFil.d.gds.gz"), library("TOP", elems));
}

fn gfil_g(pdk: &PdkConfig) {
    let gat = layer(pdk, "GatPoly");
    let gfil = layer(pdk, "GatPoly.filler");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density: bottom GatPoly stripe drops below the 15 % floor when too short.
    let stripes = |h: f64| [(gat, 0.0, h), (gfil, 925.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(75.0));
    write_gz(&format!("{DIR}/GFil.g.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(74.99));
    write_gz(
        &format!("{DIR}/GFil.g.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

/// GFil.g boundary handling: an unrelated marker (TRANS) sits outside EdgeSeal and
/// stretches the chip's raw bounding box, so `boundary`'s *own* bbox — not the raw
/// chip bbox — must be the density denominator (`ok`, solid EdgeSeal square).  `ring`
/// draws EdgeSeal as a hollow ring instead, as a real seal ring actually is: its own
/// merged area is only the thin frame, so this proves the bbox convention (not the
/// ring's drawn area) is what `min_density`/`max_density` already use — the same class
/// of bug the windowed-density boundary fix corrected.  Both fixtures cover the same
/// 900x900 seal at 20% GatPoly density, comfortably above the 15% floor.
fn gfil_g_boundary(pdk: &PdkConfig) {
    let gat = layer(pdk, "GatPoly");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    let trans = layer(pdk, "TRANS");

    let elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
        rect(gat, 0.0, 0.0, 900.0, 180.0),
    ];
    write_gz(
        &format!("{DIR}/GFil.g.boundary_ok.gds.gz"),
        library("TOP", elems),
    );

    let frame = 20.0;
    let elems_ring = vec![
        rect(boundary, 0.0, 0.0, 900.0, frame),
        rect(boundary, 0.0, 900.0 - frame, 900.0, 900.0),
        rect(boundary, 0.0, 0.0, frame, 900.0),
        rect(boundary, 900.0 - frame, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
        rect(gat, 0.0, 0.0, 900.0, 180.0),
    ];
    write_gz(
        &format!("{DIR}/GFil.g.boundary_ring.gds.gz"),
        library("TOP", elems_ring),
    );
}

// ---------------------------------------------------------------------------------------
// Hardening patterns (hardening/SPEC.md): layouts drawn from the manual's sections 5.8
// (GatPoly) and 5.9 (GatPoly:filler) alone, one fixture per theme, `<rule>.h<n>`.  Each
// function's comment states the geometry and what the manual says about it; the expected
// answers are in the `gatpoly` table of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/gatpoly.md.
// ---------------------------------------------------------------------------------------

/// Layers the hardening patterns draw on.
struct L {
    gp: (i16, i16),
    activ: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    nsd_block: (i16, i16),
    nw: (i16, i16),
    nbl: (i16, i16),
    tgo: (i16, i16),
    cont: (i16, i16),
    salblock: (i16, i16),
    trans: (i16, i16),
    sram: (i16, i16),
    gfil: (i16, i16),
    afil: (i16, i16),
    nofill: (i16, i16),
    seal: (i16, i16),
}

/// Implant of a drawn transistor.  Section 4.2: NFET = GatPoly over (Activ AND NOT pSD),
/// so a bare Activ is an NFET; nSD:drawing is only for the rhigh case (note 1) and
/// nSD:block takes the implant away without making the device a PFET.
#[derive(Clone, Copy)]
enum Imp {
    Bare,
    Nsd,
    NsdBlock,
    /// pSD, and an NWell 0.4 around the Activ: a PFET.
    Pfet,
    /// pSD without an NWell: neither NFET nor PFET by section 4.2.
    PsdNoWell,
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            gp: layer(pdk, "GatPoly"),
            activ: layer(pdk, "Activ"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            nsd_block: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            nbl: layer(pdk, "nBuLay"),
            tgo: layer(pdk, "ThickGateOx"),
            cont: layer(pdk, "Cont"),
            salblock: layer(pdk, "SalBlock"),
            trans: layer(pdk, "TRANS"),
            sram: layer(pdk, "SRAM"),
            gfil: layer(pdk, "GatPoly.filler"),
            afil: layer(pdk, "Activ.filler"),
            nofill: layer(pdk, "GatPoly.nofill"),
            seal: layer(pdk, "EdgeSeal.boundary"),
        }
    }

    /// A transistor: Activ `(x, y)-(x+aw, y+ah)` crossed by vertical gates, one per
    /// `(dx, gl)` in `gates` - the gate runs from `x+dx` to `x+dx+gl` and `ext` past the
    /// Activ top and bottom (the end caps).  `imp` sets the implant; `tgo` puts a
    /// ThickGateOx 0.4 around the Activ and the gate ends (a 3.3 V device).
    #[allow(clippy::too_many_arguments)]
    fn fet(
        &self,
        x: f64,
        y: f64,
        aw: f64,
        ah: f64,
        gates: &[(f64, f64)],
        ext: f64,
        imp: Imp,
        tgo: bool,
    ) -> Vec<GdsElement> {
        let mut e = vec![rect(self.activ, x, y, x + aw, y + ah)];
        for &(dx, gl) in gates {
            e.push(rect(self.gp, x + dx, y - ext, x + dx + gl, y + ah + ext));
        }
        match imp {
            Imp::Bare => {}
            Imp::Nsd => e.push(rect(self.nsd, x, y, x + aw, y + ah)),
            Imp::NsdBlock => e.push(rect(self.nsd_block, x, y, x + aw, y + ah)),
            Imp::Pfet => {
                e.push(rect(self.psd, x, y, x + aw, y + ah));
                e.push(rect(self.nw, x - 0.4, y - 0.4, x + aw + 0.4, y + ah + 0.4));
            }
            Imp::PsdNoWell => e.push(rect(self.psd, x, y, x + aw, y + ah)),
        }
        if tgo {
            e.push(rect(
                self.tgo,
                x - 0.4,
                y - ext - 0.4,
                x + aw + 0.4,
                y + ah + ext + 0.4,
            ));
        }
        e
    }

    /// Ring on layer `l`: outer box minus the hole, as four overlapping walls.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
        l: (i16, i16),
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
            rect(l, x0, y0, hx0, y1),
            rect(l, hx1, y0, x1, y1),
            rect(l, x0, y0, x1, hy0),
            rect(l, x0, hy1, x1, y1),
        ]
    }
}

/// Diamond (45°-rotated square) of half-diagonal `a` centred on `(cx, cy)`; its width
/// between opposite walls is `a·√2`.
fn diamond(layer: (i16, i16), cx: f64, cy: f64, a: f64) -> GdsElement {
    poly(
        layer,
        &[(cx, cy - a), (cx + a, cy), (cx, cy + a), (cx - a, cy)],
    )
}

/// 45° strip from `(x0, y0)` running `len` up-right; its perpendicular width is `d·√2`.
/// A second strip `dy` higher sits at a perpendicular gap of `(dy − 2d)/√2`.
fn strip45(layer: (i16, i16), x0: f64, y0: f64, len: f64, d: f64) -> GdsElement {
    poly(
        layer,
        &[
            (x0, y0),
            (x0 + len, y0 + len),
            (x0 + len - d, y0 + len + d),
            (x0 - d, y0 + d),
        ],
    )
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    gat_a_h(&l);
    gat_a1_h(&l);
    gat_a2_h(&l);
    gat_a3_h(&l);
    gat_a4_h(&l);
    gat_b_h(&l);
    gat_b1_h(&l);
    gat_c_h(&l);
    gat_d_h(&l);
    gat_e_h(&l);
    gat_f_h(&l);
    gfil_a_h(&l);
    gfil_d_h(&l);
    gfil_e_h(&l);
    gfil_f_h(&l);
    gfil_g_h(&l);
    gfil_i_h(&l);
    gfil_j_h(&l);
}

// --- Gat.a: min. GatPoly width 0.13 ---

fn gat_a_h(l: &L) {
    let gp = l.gp;

    // h1 — the bound, a long bar, a sliver, a far bar.  0.13 is legal, 0.125 is not, in x
    // and in y; a 300 µm bar across every tile line counts once (two walls); a 0.005
    // sliver is a width violation (and a Gat.e area one).
    write(
        "Gat.a.h1",
        vec![
            rect(gp, 2.0, 2.0, 2.13, 4.0),              // clean
            rect(gp, 5.0, 2.0, 5.125, 4.0),             // Gat.a (x)
            rect(gp, 8.0, 2.0, 10.0, 2.125),            // Gat.a (y)
            rect(gp, 2.0, 8.0, 302.0, 8.13),            // clean, 300 µm
            rect(gp, 2.0, 12.0, 302.0, 12.125),         // Gat.a, 300 µm
            rect(gp, 2.0, 16.0, 2.005, 18.0),           // Gat.a, sliver
            rect(gp, 1000.0, 1000.0, 1000.125, 1002.0), // Gat.a, far
        ],
    );

    // h3 — shapes that merge.  The rule reads the union: overlapping boxes whose union is
    // 0.13 wide are clean, 0.125 fires once; abutting slices 0.03+0.03+0.03+0.04 make
    // 0.13 (clean), 3 × 0.03 + 0.035 make 0.125 (fires); a 0.13 bar drawn as a 2 × 10
    // grid is clean; a dogbone with a 0.125 neck fires once; a ring with one 0.125 wall
    // fires once; a 0.125 island in a ring's hole fires once.
    let mut e = vec![
        rect(gp, 2.0, 2.0, 2.08, 4.0),
        rect(gp, 2.05, 2.0, 2.13, 4.0), // union 0.13 → clean
        rect(gp, 5.0, 2.0, 5.08, 4.0),
        rect(gp, 5.045, 2.0, 5.125, 4.0), // union 0.125 → Gat.a
    ];
    for (i, w) in [0.03, 0.03, 0.03, 0.04].iter().enumerate() {
        let x = 8.0 + 0.03 * i as f64;
        e.push(rect(gp, x, 2.0, x + w, 4.0)); // 0.13 → clean
    }
    for (i, w) in [0.03, 0.03, 0.03, 0.035].iter().enumerate() {
        let x = 11.0 + 0.03 * i as f64;
        e.push(rect(gp, x, 2.0, x + w, 4.0)); // 0.125 → Gat.a
    }
    for i in 0..2 {
        for j in 0..10 {
            let (x, y) = (14.0 + 0.065 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(gp, x, y, x + 0.065, y + 0.2)); // 0.13 × 2 grid → clean
        }
    }
    e.push(poly(
        gp,
        &[
            (17.0, 2.0),
            (18.0, 2.0),
            (18.0, 2.5),
            (17.5, 2.5),
            (17.5, 3.0),
            (18.0, 3.0),
            (18.0, 3.5),
            (17.0, 3.5),
            (17.0, 3.0),
            (17.375, 3.0),
            (17.375, 2.5),
            (17.0, 2.5),
        ],
    )); // dogbone: neck 0.125 (x 17.375..17.5) → Gat.a
    e.extend(l.ring(gp, 2.0, 7.0, 4.0, 9.0, 2.125, 7.5, 3.5, 8.5)); // left wall 0.125 → Gat.a
    e.extend(l.ring(gp, 6.0, 7.0, 9.0, 10.0, 6.5, 7.5, 8.5, 9.5));
    e.push(rect(gp, 7.0, 8.0, 7.125, 9.0)); // island 0.125 → Gat.a (0.5 from the walls)
    write("Gat.a.h3", e);

    // h7 — a comb whose three teeth are 0.125 wide (three violations of one polygon); a
    // U with 0.13 arms is clean.
    write(
        "Gat.a.h7",
        vec![
            poly(
                gp,
                &[
                    (2.0, 2.0),
                    (6.0, 2.0),
                    (6.0, 4.5),
                    (5.875, 4.5),
                    (5.875, 2.5),
                    (4.125, 2.5),
                    (4.125, 4.5),
                    (4.0, 4.5),
                    (4.0, 2.5),
                    (2.125, 2.5),
                    (2.125, 4.5),
                    (2.0, 4.5),
                ],
            ),
            poly(
                gp,
                &[
                    (9.0, 2.0),
                    (12.0, 2.0),
                    (12.0, 4.5),
                    (11.87, 4.5),
                    (11.87, 2.5),
                    (9.13, 2.5),
                    (9.13, 4.5),
                    (9.0, 4.5),
                ],
            ),
        ],
    );
}

// --- Gat.a1: min. gate length of a 1.2 V NFET 0.13 ---

fn gat_a1_h(l: &L) {
    let gp = l.gp;

    // h1 — the bound and the implants.  Section 4.2: NFET = GatPoly over (Activ AND NOT
    // pSD).  A bare Activ (the way every NFET in a real design is drawn) with a 0.125 gate
    // fires; so does one with nSD drawn and one under nSD:block (no pSD, so still an
    // NFET).  A PFET's 0.125 gate is Gat.a2's; a 3.3 V NFET's is Gat.a3's.  Every 0.125
    // poly is also a Gat.a.
    let mut e = l.fet(2.0, 2.0, 2.0, 1.0, &[(0.9, 0.13)], 0.3, Imp::Bare, false); // clean
    e.extend(l.fet(6.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Bare, false)); // Gat.a1
    e.extend(l.fet(10.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Nsd, false)); // Gat.a1
    e.extend(l.fet(14.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Pfet, false)); // Gat.a2
    e.extend(l.fet(2.0, 6.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Bare, true)); // Gat.a3
    e.extend(l.fet(
        6.0,
        6.0,
        2.0,
        1.0,
        &[(0.9, 0.125)],
        0.3,
        Imp::NsdBlock,
        false,
    )); // Gat.a1
    write("Gat.a1.h1", e);

    // h2 — gate shapes.  A dumbbell (0.5 heads outside the Activ, 0.125 neck across it)
    // fires; the reverse (0.125 outside, 0.3 over the Activ) is a Gat.a only; a 0.16 gate
    // with a 0.035 notch over the channel is 0.125 long there (and a Gat.f); a 0.5 gate
    // across a 0.12-tall Activ has a 0.5 gate length, not 0.12; a four-finger device with
    // fingers 0.13/0.125/0.13/0.125 fires twice.
    let mut e = vec![
        rect(l.activ, 2.0, 2.0, 4.0, 3.0),
        rect(gp, 2.9, 1.7, 3.025, 3.3),
        rect(gp, 2.7, 1.2, 3.2, 1.7),
        rect(gp, 2.7, 3.3, 3.2, 3.8), // dumbbell → Gat.a1
        rect(l.activ, 6.0, 2.0, 8.0, 3.0),
        rect(gp, 6.9, 1.2, 7.025, 3.8),
        rect(gp, 6.8, 1.7, 7.1, 3.3), // wide over the Activ → clean (Gat.a outside)
        rect(l.activ, 10.0, 2.0, 12.0, 3.0),
        poly(
            gp,
            &[
                (10.9, 1.7),
                (11.06, 1.7),
                (11.06, 2.3),
                (11.025, 2.3),
                (11.025, 2.7),
                (11.06, 2.7),
                (11.06, 3.3),
                (10.9, 3.3),
            ],
        ), // notch → Gat.a1 (and Gat.f)
        rect(l.activ, 14.0, 2.0, 16.0, 2.12),
        rect(gp, 14.9, 1.7, 15.4, 2.42), // 0.5 gate on a 0.12 Activ → clean
    ];
    e.extend(l.fet(
        2.0,
        6.0,
        5.0,
        1.0,
        &[(0.5, 0.13), (1.5, 0.125), (2.5, 0.13), (3.5, 0.125)],
        0.3,
        Imp::Bare,
        false,
    )); // 2 × Gat.a1
    write("Gat.a1.h2", e);

    // h3 — tile lines.  0.125 gates straddling x = 20, ending on 20, starting on 20,
    // straddling 21, 40 and 42, one inside a tile at 10, a horizontal gate across x = 20
    // (its Activ 4 µm wide); a 0.13 gate straddling 20 is clean.
    let mut e = l.fet(18.5, 2.0, 3.0, 1.0, &[(1.45, 0.125)], 0.3, Imp::Bare, false); // 19.95..20.075
    e.extend(l.fet(
        18.5,
        5.0,
        3.0,
        1.0,
        &[(1.375, 0.125)],
        0.3,
        Imp::Bare,
        false,
    )); // ..20
    e.extend(l.fet(18.5, 8.0, 3.0, 1.0, &[(1.5, 0.125)], 0.3, Imp::Bare, false)); // 20..
    e.extend(l.fet(
        19.5,
        11.0,
        3.0,
        1.0,
        &[(1.45, 0.125)],
        0.3,
        Imp::Bare,
        false,
    )); // 20.95..
    e.extend(l.fet(38.5, 2.0, 3.0, 1.0, &[(1.45, 0.125)], 0.3, Imp::Bare, false)); // 39.95..
    e.extend(l.fet(40.5, 5.0, 3.0, 1.0, &[(1.45, 0.125)], 0.3, Imp::Bare, false)); // 41.95..
    e.extend(l.fet(9.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Bare, false)); // inside
    e.push(rect(l.activ, 18.0, 15.0, 22.0, 17.0));
    e.push(rect(gp, 17.7, 15.9, 22.3, 16.025)); // horizontal gate across 20
    e.extend(l.fet(
        18.5,
        19.0,
        3.0,
        1.0,
        &[(1.435, 0.13)],
        0.3,
        Imp::Bare,
        false,
    )); // clean
    write("Gat.a1.h3", e);

    // h6 — a 300 µm wide NFET with a 0.125 gate, one at (1000, 1000), and a 0.125 gate
    // crossing a 0.005 sliver of Activ (a gate all the same).
    let mut e = l.fet(
        2.0,
        2.0,
        300.0,
        1.0,
        &[(148.0, 0.125)],
        0.3,
        Imp::Bare,
        false,
    );
    e.extend(l.fet(
        1000.0,
        1000.0,
        2.0,
        1.0,
        &[(0.9, 0.125)],
        0.3,
        Imp::Bare,
        false,
    ));
    e.push(rect(l.activ, 2.0, 6.0, 4.0, 6.005));
    e.push(rect(gp, 2.9, 5.6, 3.025, 6.4));
    write("Gat.a1.h6", e);
}

// --- Gat.a2: min. gate length of a 1.2 V PFET 0.13 ---

fn gat_a2_h(l: &L) {
    // h1 — the bound and the well.  Section 4.2: PFET = GatPoly over ((Activ AND pSD)
    // inside NWell).  A PFET with a 0.125 gate fires; a pSD Activ without an NWell is no
    // PFET (Gat.a only); a pSD Activ crossing the NWell edge is not "inside NWell"; a
    // 3.3 V PFET's 0.125 gate is Gat.a4's.
    let mut e = l.fet(2.0, 2.0, 2.0, 1.0, &[(0.9, 0.13)], 0.3, Imp::Pfet, false); // clean
    e.extend(l.fet(6.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Pfet, false)); // Gat.a2
    e.extend(l.fet(
        10.0,
        2.0,
        2.0,
        1.0,
        &[(0.9, 0.125)],
        0.3,
        Imp::PsdNoWell,
        false,
    )); // no PFET
    e.extend(l.fet(
        14.0,
        2.0,
        2.0,
        1.0,
        &[(0.9, 0.125)],
        0.3,
        Imp::PsdNoWell,
        false,
    ));
    e.push(rect(l.nw, 13.0, 1.0, 15.0, 4.0)); // well over the left half → not inside
    e.extend(l.fet(2.0, 6.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Pfet, true)); // Gat.a4
    write("Gat.a2.h1", e);

    // h2 — a four-finger PFET (0.13/0.125/0.13/0.125) and a dumbbell PFET gate.
    let mut e = l.fet(
        2.0,
        2.0,
        5.0,
        1.0,
        &[(0.5, 0.13), (1.5, 0.125), (2.5, 0.13), (3.5, 0.125)],
        0.3,
        Imp::Pfet,
        false,
    );
    e.extend(l.fet(10.0, 2.0, 2.0, 1.0, &[(0.9, 0.125)], 0.3, Imp::Pfet, false));
    e.push(rect(l.gp, 10.7, 1.2, 11.2, 1.7));
    e.push(rect(l.gp, 10.7, 3.3, 11.2, 3.8));
    write("Gat.a2.h2", e);
}

// --- Gat.a3: min. gate length of a 3.3 V NFET 0.45 ---

fn gat_a3_h(l: &L) {
    // h1 — the bound and the implants under ThickGateOx.  0.45 is legal; 0.445 fires for a
    // bare Activ, with nSD drawn, and under nSD:block.  A 1.2 V NFET at 0.445 is clean; a
    // 3.3 V PFET at 0.445 is clean (Gat.a4 is 0.40).  A gate with ThickGateOx over only
    // its left half (a TGO.c elsewhere) is a 3.3 V gate as far as the oxide reaches.
    let mut e = l.fet(2.0, 2.0, 2.0, 1.0, &[(0.75, 0.45)], 0.3, Imp::Bare, true); // clean
    e.extend(l.fet(6.0, 2.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Bare, true)); // Gat.a3
    e.extend(l.fet(10.0, 2.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Nsd, true)); // Gat.a3
    e.extend(l.fet(
        14.0,
        2.0,
        2.0,
        1.0,
        &[(0.75, 0.445)],
        0.3,
        Imp::NsdBlock,
        true,
    )); // Gat.a3
    e.extend(l.fet(2.0, 6.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Bare, false)); // clean (1.2 V)
    e.extend(l.fet(6.0, 6.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Pfet, true)); // clean (PFET)
    e.extend(l.fet(10.0, 6.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Bare, false));
    e.push(rect(l.tgo, 9.6, 5.3, 11.0, 7.7)); // oxide over the left half only → Gat.a3
    write("Gat.a3.h1", e);

    // h2 — a four-finger 3.3 V NFET (0.45/0.445/0.45/0.445), a dumbbell gate, and gates
    // straddling x = 20 (19.8..20.245), ending on 20, a horizontal one across 20; a 0.45
    // gate straddling 20 is clean.
    let mut e = l.fet(
        2.0,
        2.0,
        5.0,
        1.0,
        &[(0.5, 0.45), (1.5, 0.445), (2.5, 0.45), (3.5, 0.445)],
        0.3,
        Imp::Bare,
        true,
    );
    e.extend(l.fet(10.0, 2.0, 2.0, 1.0, &[(0.75, 0.445)], 0.3, Imp::Bare, true));
    e.push(rect(l.gp, 10.5, 1.0, 11.5, 1.7));
    e.push(rect(l.gp, 10.5, 3.3, 11.5, 4.0));
    e.extend(l.fet(18.5, 6.0, 3.0, 1.0, &[(1.3, 0.445)], 0.3, Imp::Bare, true));
    e.extend(l.fet(
        18.5,
        10.0,
        3.0,
        1.0,
        &[(1.055, 0.445)],
        0.3,
        Imp::Bare,
        true,
    ));
    e.push(rect(l.activ, 18.0, 14.0, 22.0, 16.0));
    e.push(rect(l.gp, 17.7, 14.8, 22.3, 15.245));
    e.push(rect(l.tgo, 17.3, 13.5, 22.7, 16.5));
    e.extend(l.fet(18.5, 19.0, 3.0, 1.0, &[(1.28, 0.45)], 0.3, Imp::Bare, true)); // clean
    write("Gat.a3.h2", e);
}

// --- Gat.a4: min. gate length of a 3.3 V PFET 0.40 ---

fn gat_a4_h(l: &L) {
    // h1 — the bound and the well.  0.40 is legal, 0.395 fires; a pSD Activ under
    // ThickGateOx without an NWell is no PFET (section 4.2) and nothing fires; a pSD Activ
    // crossing the well edge is not "inside NWell" either; a 1.2 V PFET at 0.395 is clean;
    // a 3.3 V NFET at 0.395 is Gat.a3's.
    let mut e = l.fet(2.0, 2.0, 2.0, 1.0, &[(0.8, 0.40)], 0.3, Imp::Pfet, true); // clean
    e.extend(l.fet(6.0, 2.0, 2.0, 1.0, &[(0.8, 0.395)], 0.3, Imp::Pfet, true)); // Gat.a4
    e.extend(l.fet(
        10.0,
        2.0,
        2.0,
        1.0,
        &[(0.8, 0.395)],
        0.3,
        Imp::PsdNoWell,
        true,
    )); // no PFET
    e.extend(l.fet(
        14.0,
        2.0,
        2.0,
        1.0,
        &[(0.8, 0.395)],
        0.3,
        Imp::PsdNoWell,
        true,
    ));
    e.push(rect(l.nw, 13.0, 1.0, 15.0, 4.0)); // well over the left half → not inside
    e.extend(l.fet(2.0, 6.0, 2.0, 1.0, &[(0.8, 0.395)], 0.3, Imp::Pfet, false)); // clean (1.2 V)
    e.extend(l.fet(6.0, 6.0, 2.0, 1.0, &[(0.8, 0.395)], 0.3, Imp::Bare, true)); // Gat.a3
    write("Gat.a4.h1", e);

    // h2 — a four-finger 3.3 V PFET (0.40/0.395/0.40/0.395), a dumbbell gate, gates
    // straddling x = 20 (19.8..20.195), ending on 20, a horizontal one across 20; a 0.40
    // gate straddling 20 is clean.
    let mut e = l.fet(
        2.0,
        2.0,
        5.0,
        1.0,
        &[(0.5, 0.40), (1.5, 0.395), (2.5, 0.40), (3.5, 0.395)],
        0.3,
        Imp::Pfet,
        true,
    );
    e.extend(l.fet(10.0, 2.0, 2.0, 1.0, &[(0.8, 0.395)], 0.3, Imp::Pfet, true));
    e.push(rect(l.gp, 10.5, 1.0, 11.5, 1.7));
    e.push(rect(l.gp, 10.5, 3.3, 11.5, 4.0));
    e.extend(l.fet(18.5, 6.0, 3.0, 1.0, &[(1.3, 0.395)], 0.3, Imp::Pfet, true));
    e.extend(l.fet(
        18.5,
        10.0,
        3.0,
        1.0,
        &[(1.105, 0.395)],
        0.3,
        Imp::Pfet,
        true,
    ));
    e.push(rect(l.activ, 18.0, 14.0, 22.0, 16.0));
    e.push(rect(l.psd, 18.0, 14.0, 22.0, 16.0));
    e.push(rect(l.nw, 17.6, 13.6, 22.4, 16.4));
    e.push(rect(l.gp, 17.7, 14.8, 22.3, 15.195));
    e.push(rect(l.tgo, 17.3, 13.5, 22.7, 16.5));
    e.extend(l.fet(18.5, 19.0, 3.0, 1.0, &[(1.3, 0.40)], 0.3, Imp::Pfet, true)); // clean
    write("Gat.a4.h2", e);
}

// --- Gat.b: min. GatPoly space or notch 0.18 ---

fn gat_b_h(l: &L) {
    let gp = l.gp;

    // h3 — "space or notch".  A U with a 0.175 slot, a comb with three 0.175 slots, a
    // 0.175 slot cut into a plate, two Ls whose arms face across 0.175, a ring whose hole
    // is 0.175 wide, an island 0.175 from a ring's inner wall: eight.
    let mut e = vec![
        poly(
            gp,
            &[
                (2.0, 2.0),
                (3.175, 2.0),
                (3.175, 4.0),
                (2.675, 4.0),
                (2.675, 2.5),
                (2.5, 2.5),
                (2.5, 4.0),
                (2.0, 4.0),
            ],
        ),
        poly(
            gp,
            &[
                (5.0, 2.0),
                (8.0, 2.0),
                (8.0, 4.0),
                (7.5, 4.0),
                (7.5, 2.5),
                (7.325, 2.5),
                (7.325, 4.0),
                (6.5, 4.0),
                (6.5, 2.5),
                (6.325, 2.5),
                (6.325, 4.0),
                (5.5, 4.0),
                (5.5, 2.5),
                (5.325, 2.5),
                (5.325, 4.0),
                (5.0, 4.0),
            ],
        ),
        poly(
            gp,
            &[
                (10.0, 2.0),
                (12.0, 2.0),
                (12.0, 4.0),
                (11.175, 4.0),
                (11.175, 3.0),
                (11.0, 3.0),
                (11.0, 4.0),
                (10.0, 4.0),
            ],
        ),
        poly(
            gp,
            &[
                (14.0, 2.0),
                (16.0, 2.0),
                (16.0, 2.5),
                (14.5, 2.5),
                (14.5, 4.0),
                (14.0, 4.0),
            ],
        ),
        poly(
            gp,
            &[
                (14.675, 3.0),
                (15.175, 3.0),
                (15.175, 4.0),
                (16.5, 4.0),
                (16.5, 4.5),
                (14.675, 4.5),
            ],
        ),
    ];
    e.extend(l.ring(gp, 2.0, 6.0, 4.0, 8.0, 2.9, 6.5, 3.075, 7.5)); // hole 0.175
    e.extend(l.ring(gp, 6.0, 6.0, 9.0, 9.0, 6.5, 6.5, 8.5, 8.5));
    e.push(rect(gp, 6.675, 7.0, 7.675, 8.0)); // island 0.175 from the left wall
    write("Gat.b.h3", e);

    // h7 — in context.  Two 1.2 V gate fingers 0.175 apart on one Activ; a gate and an
    // unrelated poly line 0.175 past its end cap; a 0.005 sliver 0.175 from a bar; two
    // 300 µm bars 0.175 apart; a pair at (1000, 1000).
    write(
        "Gat.b.h7",
        vec![
            rect(l.activ, 2.0, 2.0, 4.0, 3.0),
            rect(gp, 2.8, 1.7, 3.0, 3.3),
            rect(gp, 3.175, 1.7, 3.375, 3.3),
            rect(l.activ, 6.0, 2.0, 8.0, 3.0),
            rect(gp, 6.9, 1.7, 7.1, 3.3),
            rect(gp, 6.5, 3.475, 7.5, 3.675),
            rect(gp, 10.0, 2.0, 10.005, 3.0),
            rect(gp, 10.18, 2.0, 11.18, 3.0),
            rect(gp, 2.0, 6.0, 302.0, 7.0),
            rect(gp, 2.0, 7.175, 302.0, 8.175),
            rect(gp, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(gp, 1001.175, 1000.0, 1002.175, 1001.0),
        ],
    );
}

// --- Gat.b1: min. space between unrelated 3.3 V GatPoly over Activ regions 0.25 ---

fn gat_b1_h(l: &L) {
    let gp = l.gp;
    // A 3.3 V NFET on a 3 × 1 Activ; its gates are 0.5 long so Gat.a3 (0.45) stays out.
    let hv =
        |x: f64, y: f64, gaps: &[(f64, f64)]| l.fet(x, y, 3.0, 1.0, gaps, 0.3, Imp::Bare, true);

    // h1 — the bound.  Two 3.3 V fingers (0.5 long) 0.25 apart on one Activ are clean,
    // 0.245 fires; two 1.2 V fingers 0.245 apart are clean (Gat.b is 0.18); a 3.3 V finger
    // and a finger outside the oxide 0.245 apart are clean (one 3.3 V region only); two
    // 3.3 V fingers 0.175 apart are a Gat.b1 and a Gat.b.
    let mut e = hv(2.0, 2.0, &[(0.5, 0.5), (1.25, 0.5)]); // clean
    e.extend(hv(7.0, 2.0, &[(0.5, 0.5), (1.245, 0.5)])); // Gat.b1
    e.extend(l.fet(
        12.0,
        2.0,
        3.0,
        1.0,
        &[(0.5, 0.5), (1.245, 0.5)],
        0.3,
        Imp::Bare,
        false,
    )); // clean, 1.2 V
    e.extend(l.fet(
        2.0,
        6.0,
        3.0,
        1.0,
        &[(0.5, 0.5), (1.245, 0.5)],
        0.3,
        Imp::Bare,
        false,
    ));
    e.push(rect(l.tgo, 1.6, 5.3, 3.0, 7.7)); // oxide over the first finger only → clean
    e.extend(hv(7.0, 6.0, &[(0.5, 0.5), (1.175, 0.5)])); // Gat.b1 + Gat.b
    write("Gat.b1.h1", e);

    // h2 — the figure.  Two separate 3.3 V transistors whose gate polys end 0.245 apart
    // (the figure's b1 arrow), their gate regions 0.845 apart: fires by the figure.  The
    // same at 0.25 is clean.  One poly crossing two Activs 0.245 apart makes two gate
    // regions 0.245 apart: fires.
    write(
        "Gat.b1.h2",
        vec![
            rect(l.activ, 2.0, 2.0, 3.0, 4.0),
            rect(gp, 1.7, 2.7, 3.3, 3.2),
            rect(l.activ, 3.845, 2.0, 4.845, 4.0),
            rect(gp, 3.545, 2.7, 5.145, 3.2),
            rect(l.tgo, 1.2, 1.5, 5.7, 4.5), // poly ends 0.245 apart → Gat.b1 (figure)
            rect(l.activ, 8.0, 2.0, 9.0, 4.0),
            rect(gp, 7.7, 2.7, 9.3, 3.2),
            rect(l.activ, 9.85, 2.0, 10.85, 4.0),
            rect(gp, 9.55, 2.7, 11.15, 3.2),
            rect(l.tgo, 7.2, 1.5, 11.7, 4.5), // 0.25 → clean
            rect(l.activ, 14.0, 2.0, 16.0, 3.0),
            rect(l.activ, 16.245, 2.0, 18.245, 3.0),
            rect(gp, 13.7, 2.25, 18.545, 2.75),
            rect(l.tgo, 13.3, 1.5, 18.9, 3.5), // gate regions 0.245 apart → Gat.b1
        ],
    );

    // h3 — related or not.  A U-shaped 3.3 V poly whose legs cross one Activ 0.245 apart
    // (the base outside the Activ) makes two gate regions that do not touch: fires.  A
    // U whose base lies over the Activ makes one gate region: clean for Gat.b1 (a Gat.f).
    write(
        "Gat.b1.h3",
        vec![
            rect(l.activ, 2.0, 2.0, 4.0, 3.0),
            poly(
                gp,
                &[
                    (2.5, 1.4),
                    (3.745, 1.4),
                    (3.745, 3.3),
                    (3.245, 3.3),
                    (3.245, 1.7),
                    (3.0, 1.7),
                    (3.0, 3.3),
                    (2.5, 3.3),
                ],
            ),
            rect(l.tgo, 1.6, 1.0, 4.4, 3.7), // Gat.b1
            rect(l.activ, 7.0, 2.0, 9.0, 3.0),
            poly(
                gp,
                &[
                    (7.5, 1.7),
                    (8.745, 1.7),
                    (8.745, 3.3),
                    (8.245, 3.3),
                    (8.245, 2.2),
                    (8.0, 2.2),
                    (8.0, 3.3),
                    (7.5, 3.3),
                ],
            ),
            rect(l.tgo, 6.6, 1.3, 9.4, 3.7), // one region → clean
        ],
    );

    // h4 — tile lines, far, long.  0.245 finger gaps straddling x = 20, ending on 20,
    // straddling 21, 40 and 42; a pair at (1000, 1000); two 300 µm gate regions 0.245
    // apart (polys along a 300 × 1.5 Activ).
    let mut e = hv(18.5, 2.0, &[(0.9, 0.5), (1.645, 0.5)]); // 19.9 | 20.145
    e.extend(hv(18.5, 6.0, &[(0.755, 0.5), (1.5, 0.5)])); // ..19.755 | 20..
    e.extend(hv(19.5, 10.0, &[(0.9, 0.5), (1.645, 0.5)])); // 20.9 | 21.145
    e.extend(hv(38.5, 2.0, &[(0.9, 0.5), (1.645, 0.5)]));
    e.extend(hv(40.5, 6.0, &[(0.9, 0.5), (1.645, 0.5)]));
    e.extend(hv(1000.0, 1000.0, &[(0.5, 0.5), (1.245, 0.5)]));
    e.push(rect(l.activ, 2.0, 14.0, 302.0, 15.5));
    e.push(rect(gp, 1.7, 14.2, 302.3, 14.7));
    e.push(rect(gp, 1.7, 14.945, 302.3, 15.445));
    e.push(rect(l.tgo, 1.2, 13.5, 302.8, 16.0));
    write("Gat.b1.h4", e);
}

// --- Gat.c: min. GatPoly extension over Activ (end cap) 0.18 ---

fn gat_c_h(l: &L) {
    let gp = l.gp;
    let act = l.activ;

    // h1 — the bound.  0.18 is legal; 0.175 at the bottom fires once, at both ends twice;
    // a horizontal gate 0.175 short on the left fires; a gate on a 300 µm Activ 0.175
    // short fires once; a 300 µm gate on a 300 µm-tall Activ with 0.18 caps is clean.
    write(
        "Gat.c.h1",
        vec![
            rect(act, 2.0, 2.0, 3.0, 3.0),
            rect(gp, 2.3, 1.82, 2.6, 3.18), // clean
            rect(act, 5.0, 2.0, 6.0, 3.0),
            rect(gp, 5.3, 1.825, 5.6, 3.18), // Gat.c
            rect(act, 8.0, 2.0, 9.0, 3.0),
            rect(gp, 8.3, 1.825, 8.6, 3.175), // 2 × Gat.c
            rect(act, 11.0, 2.0, 12.0, 3.0),
            rect(gp, 10.825, 2.3, 12.18, 2.6), // Gat.c
            rect(act, 2.0, 6.0, 302.0, 7.0),
            rect(gp, 150.0, 5.825, 150.3, 7.18), // Gat.c
            rect(act, 2.0, 10.0, 3.0, 310.0),
            rect(gp, 1.82, 150.0, 3.18, 150.3), // clean
        ],
    );

    // h2 — no cap at all.  A gate ending exactly on the Activ edge (0.00); a gate stub
    // ending 0.4 inside the Activ; a poly entirely inside the Activ; a poly square over an
    // Activ corner extending 0.5 (clean) and 0.175 (fires); a gate whose right side runs
    // along the Activ's right edge.
    write(
        "Gat.c.h2",
        vec![
            rect(act, 2.0, 2.0, 3.0, 3.0),
            rect(gp, 2.3, 2.0, 2.6, 3.18), // Gat.c, cap 0.00
            rect(act, 5.0, 2.0, 6.0, 3.0),
            rect(gp, 5.3, 1.82, 5.6, 2.6), // Gat.c, stub
            rect(act, 8.0, 2.0, 9.0, 3.0),
            rect(gp, 8.3, 2.3, 8.6, 2.7), // Gat.c, inside
            rect(act, 11.0, 2.0, 13.0, 4.0),
            rect(gp, 12.5, 3.5, 13.5, 4.5), // clean, corner
            rect(act, 15.0, 2.0, 17.0, 4.0),
            rect(gp, 16.5, 3.5, 17.175, 4.5), // Gat.c, corner 0.175
            rect(act, 2.0, 6.0, 4.0, 8.0),
            rect(gp, 3.7, 5.7, 4.0, 8.3), // side on the Activ edge
        ],
    );

    // h3 — shapes and tile lines.  A 0.175 neck ending in a 0.2 head (0.375 in all) is
    // clean; a cap whose corner is chamfered down to 0.10 fires; chamfered to 0.19 is
    // clean; a gate over two Activs 0.175 short at the top fires; 0.175 caps straddling
    // x = 20 (a vertical gate's bottom cap), a horizontal gate's right cap across 20, a
    // gate beside an Activ edge on 20, and at 40 and 42; a 0.18 cap across 20 is clean.
    write(
        "Gat.c.h3",
        vec![
            rect(act, 2.0, 2.0, 3.0, 3.0),
            rect(gp, 2.3, 1.82, 2.6, 3.175),
            rect(gp, 2.1, 3.175, 2.8, 3.375), // clean
            rect(act, 5.0, 2.0, 6.0, 3.0),
            poly(
                gp,
                &[
                    (5.3, 1.82),
                    (5.6, 1.82),
                    (5.6, 3.1),
                    (5.52, 3.18),
                    (5.3, 3.18),
                ],
            ), // Gat.c (0.10 at the corner)
            rect(act, 8.0, 2.0, 9.0, 3.0),
            poly(
                gp,
                &[
                    (8.3, 1.82),
                    (8.6, 1.82),
                    (8.6, 3.19),
                    (8.52, 3.27),
                    (8.3, 3.27),
                ],
            ), // clean
            rect(act, 11.0, 2.0, 12.0, 3.0),
            rect(act, 11.0, 3.5, 12.0, 4.5),
            rect(gp, 11.3, 1.82, 11.6, 4.675), // Gat.c
            rect(act, 18.5, 6.0, 21.5, 7.0),
            rect(gp, 19.85, 5.825, 20.15, 7.18), // Gat.c
            rect(act, 18.0, 9.0, 20.0, 10.0),
            rect(gp, 17.82, 9.3, 20.175, 9.6), // Gat.c
            rect(act, 20.0, 12.0, 22.0, 13.0),
            rect(gp, 20.5, 11.825, 20.8, 13.18), // Gat.c
            rect(act, 38.5, 6.0, 41.5, 7.0),
            rect(gp, 39.85, 5.825, 40.15, 7.18), // Gat.c
            rect(act, 40.5, 9.0, 43.5, 10.0),
            rect(gp, 41.85, 8.825, 42.15, 10.18), // Gat.c
            rect(act, 18.5, 15.0, 21.5, 16.0),
            rect(gp, 19.85, 14.82, 20.15, 16.18), // clean
        ],
    );

    // h6 — far and merged.  A 0.175 cap at (1000, 1000); an Activ drawn as two abutting
    // boxes with the gate across the seam; a gate drawn as two overlapping boxes; a 0.175
    // cap under an SRAM marker (the deck leaves SRAM out of Gat.c; the manual's SRAM
    // section is work in progress).
    write(
        "Gat.c.h6",
        vec![
            rect(act, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(gp, 1000.3, 999.825, 1000.6, 1001.18), // Gat.c
            rect(act, 2.0, 2.0, 2.5, 3.0),
            rect(act, 2.5, 2.0, 3.0, 3.0),
            rect(gp, 2.3, 1.82, 2.6, 3.175), // Gat.c
            rect(act, 5.0, 2.0, 6.0, 3.0),
            rect(gp, 5.3, 1.82, 5.6, 2.6),
            rect(gp, 5.3, 2.4, 5.6, 3.175), // Gat.c, once
            rect(act, 8.0, 2.0, 9.0, 3.0),
            rect(gp, 8.3, 1.825, 8.6, 3.18),
            rect(l.sram, 7.5, 1.5, 9.5, 3.5), // SRAM: not checked
        ],
    );
}

// --- Gat.d: min. GatPoly space to Activ 0.07 ---

fn gat_d_h(l: &L) {
    let gp = l.gp;
    let act = l.activ;

    // h3 — touching and in context.  A poly abutting the Activ edge (space 0.00) fires; a
    // poly touching the Activ at a corner point fires; a poly overlapping the Activ by
    // 0.05 is a gate (a Gat.c, not a Gat.d); a gate that runs 0.065 past a second Activ
    // fires; a gate stopping 0.065 short of its Activ fires; a poly in a U-shaped Activ
    // 0.065 from both arms fires twice.
    write(
        "Gat.d.h3",
        vec![
            rect(act, 2.0, 2.0, 3.0, 3.0),
            rect(gp, 3.0, 2.3, 3.3, 2.6), // Gat.d, abutting
            rect(act, 5.0, 2.0, 6.0, 3.0),
            rect(gp, 6.0, 3.0, 6.3, 3.3), // Gat.d, corner point
            rect(act, 8.0, 2.0, 9.0, 3.0),
            rect(gp, 8.95, 2.3, 9.3, 2.6), // Gat.c only
            rect(act, 11.0, 2.0, 12.0, 3.0),
            rect(gp, 11.3, 1.7, 11.6, 3.3),
            rect(act, 11.665, 3.1, 12.5, 3.6), // Gat.d, 0.065 past the gate
            rect(act, 14.0, 2.0, 15.0, 3.0),
            rect(gp, 14.3, 3.065, 14.6, 4.0), // Gat.d, short of the Activ
            poly(
                act,
                &[
                    (2.0, 6.0),
                    (4.0, 6.0),
                    (4.0, 8.0),
                    (3.5, 8.0),
                    (3.5, 6.5),
                    (2.5, 6.5),
                    (2.5, 8.0),
                    (2.0, 8.0),
                ],
            ),
            rect(gp, 2.565, 6.7, 3.435, 7.5), // 2 × Gat.d
        ],
    );
}

// --- Gat.e: min. GatPoly area 0.09 µm² ---

fn gat_e_h(l: &L) {
    let gp = l.gp;

    // h1 — the bound and merges.  0.3 × 0.3 = 0.09 is legal; 0.3 × 0.295, a 0.13 × 0.69
    // bar (0.0897) and a diamond a = 0.21 (0.0882) fire; 0.13 × 0.70 and a = 0.215 are
    // clean.  The rule reads the union: two overlapping boxes making 0.3 × 0.3 are clean,
    // making 0.3 × 0.295 fire once; two abutting halves and a 3 × 3 grid making 0.09 are
    // clean; a 0.3 × 0.3 ring with a 0.04 hole (0.0884) fires; an L of 0.1001 is clean.
    let mut e = vec![
        rect(gp, 2.0, 2.0, 2.3, 2.3),   // clean
        rect(gp, 4.0, 2.0, 4.3, 2.295), // Gat.e
        rect(gp, 6.0, 2.0, 6.13, 2.69), // Gat.e
        rect(gp, 8.0, 2.0, 8.13, 2.7),  // clean
        diamond(gp, 10.0, 2.5, 0.21),   // Gat.e
        diamond(gp, 12.0, 2.5, 0.215),  // clean
        rect(gp, 2.0, 4.0, 2.3, 4.2),
        rect(gp, 2.0, 4.1, 2.3, 4.3), // union 0.09 → clean
        rect(gp, 4.0, 4.0, 4.3, 4.2),
        rect(gp, 4.0, 4.095, 4.3, 4.295), // union 0.0885 → Gat.e
        rect(gp, 6.0, 4.0, 6.15, 4.3),
        rect(gp, 6.15, 4.0, 6.3, 4.3), // clean
    ];
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = (8.0 + 0.1 * i as f64, 4.0 + 0.1 * j as f64);
            e.push(rect(gp, x, y, x + 0.1, y + 0.1)); // clean
        }
    }
    e.extend(l.ring(gp, 10.0, 4.0, 10.3, 4.3, 10.13, 4.13, 10.17, 4.17)); // Gat.e
    e.push(poly(
        gp,
        &[
            (12.0, 4.0),
            (12.3, 4.0),
            (12.3, 4.13),
            (12.13, 4.13),
            (12.13, 4.6),
            (12.0, 4.6),
        ],
    )); // clean
    write("Gat.e.h1", e);
}

// --- Gat.f: no 45° or 90° angles for GatPoly on Activ ---

fn gat_f_h(l: &L) {
    let gp = l.gp;
    let act = l.activ;
    // An L gate: in from below at x..x+0.3, turning right at y = ty, out to the right at xr.
    let lgate = |x: f64, y0: f64, ty: f64, xr: f64| {
        poly(
            gp,
            &[
                (x, y0),
                (x + 0.3, y0),
                (x + 0.3, ty),
                (xr, ty),
                (xr, ty + 0.3),
                (x, ty + 0.3),
            ],
        )
    };

    // h1 — 90° bends over the Activ.  An L-shaped gate bending inside the Activ, a gate
    // with a stub inside the Activ (a T), a gate with a 0.035 notch over the channel, and
    // a gate that widens from 0.3 to 0.5 inside the Activ: four bent gates.
    write(
        "Gat.f.h1",
        vec![
            rect(act, 2.0, 2.0, 4.0, 4.0),
            lgate(2.8, 1.7, 3.0, 4.3), // Gat.f
            rect(act, 6.0, 2.0, 8.0, 4.0),
            rect(gp, 6.8, 1.7, 7.1, 4.3),
            rect(gp, 7.1, 2.8, 7.6, 3.1), // Gat.f, T
            rect(act, 10.0, 2.0, 12.0, 3.0),
            poly(
                gp,
                &[
                    (10.9, 1.7),
                    (11.06, 1.7),
                    (11.06, 2.3),
                    (11.025, 2.3),
                    (11.025, 2.7),
                    (11.06, 2.7),
                    (11.06, 3.3),
                    (10.9, 3.3),
                ],
            ), // Gat.f, notch (and Gat.a1)
            rect(act, 14.0, 2.0, 16.0, 4.0),
            poly(
                gp,
                &[
                    (14.8, 1.7),
                    (15.1, 1.7),
                    (15.1, 3.0),
                    (15.3, 3.0),
                    (15.3, 4.3),
                    (14.8, 4.3),
                ],
            ), // Gat.f, step
        ],
    );

    // h2 — where the angle is.  A straight gate over an L-shaped Activ whose step lies
    // under the gate: the gate region is an L but the GatPoly has no angle on the Activ -
    // clean by the wording (KLayout reads the gate region).  An L gate whose bend is 0.07
    // outside the Activ is clean; one whose bend is 0.005 inside fires.  A poly with a
    // chamfered corner outside the Activ and a 45° poly beside an Activ are clean.
    write(
        "Gat.f.h2",
        vec![
            poly(
                act,
                &[
                    (6.0, 2.0),
                    (8.0, 2.0),
                    (8.0, 3.0),
                    (7.0, 3.0),
                    (7.0, 4.0),
                    (6.0, 4.0),
                ],
            ),
            rect(gp, 6.75, 1.7, 7.25, 4.3), // straight gate, stepped Activ → clean
            rect(act, 10.0, 2.0, 12.0, 4.0),
            lgate(10.8, 1.7, 4.07, 12.3), // bend outside → clean
            rect(act, 14.0, 2.0, 16.0, 4.0),
            lgate(14.8, 1.7, 3.995, 16.3), // bend 0.005 inside → Gat.f
            rect(act, 2.0, 6.0, 4.0, 8.0),
            poly(
                gp,
                &[(2.8, 5.7), (3.1, 5.7), (3.1, 8.3), (2.9, 8.3), (2.8, 8.2)],
            ), // chamfer outside → clean
            rect(act, 6.0, 6.0, 7.0, 7.0),
            strip45(gp, 7.5, 6.0, 1.5, 0.2), // beside → clean
        ],
    );

    // h3 — 45° over the Activ.  A gate coming in straight and bending 45° inside the
    // Activ (two 45° gate edges); a gate with a 45° jog inside the Activ (two); a gate
    // bending 45° only outside the Activ (clean); a 45° band clipping an Activ corner
    // (one 45° gate edge).
    write(
        "Gat.f.h3",
        vec![
            rect(act, 2.0, 2.0, 5.0, 5.0),
            poly(
                gp,
                &[
                    (3.0, 1.7),
                    (3.3, 1.7),
                    (3.3, 3.0),
                    (5.6, 5.3),
                    (5.3, 5.6),
                    (3.0, 3.3),
                ],
            ), // 2 × Gat.f
            rect(act, 8.0, 2.0, 10.0, 4.0),
            poly(
                gp,
                &[
                    (8.8, 1.7),
                    (9.1, 1.7),
                    (9.1, 2.8),
                    (9.3, 3.0),
                    (9.3, 4.3),
                    (9.0, 4.3),
                    (9.0, 3.0),
                    (8.8, 2.8),
                ],
            ), // 2 × Gat.f, jog
            rect(act, 12.0, 2.0, 14.0, 4.0),
            poly(
                gp,
                &[
                    (12.8, 1.7),
                    (13.1, 1.7),
                    (13.1, 4.4),
                    (14.0, 5.3),
                    (13.7, 5.6),
                    (12.8, 4.7),
                ],
            ), // clean: the bend is outside
            rect(act, 16.0, 2.0, 18.0, 4.0),
            strip45(gp, 17.7, 1.2, 1.5, 0.3), // Gat.f, corner clip
        ],
    );

    // h4 — tile lines and far.  L gates cornered on x = 20 and x = 40, one straddling 42,
    // a 45° gate crossing x = 20 (two edges), an L gate at (1000, 1000); a straight gate
    // straddling 20 is clean.
    write(
        "Gat.f.h4",
        vec![
            rect(act, 18.5, 2.0, 21.5, 4.0),
            lgate(19.7, 1.7, 3.0, 21.8), // Gat.f
            rect(act, 18.5, 6.0, 21.5, 8.0),
            strip45(gp, 19.0, 5.5, 3.0, 0.3), // 2 × Gat.f
            rect(act, 38.5, 2.0, 41.5, 4.0),
            lgate(39.7, 1.7, 3.0, 41.8), // Gat.f
            rect(act, 40.5, 6.0, 43.5, 8.0),
            lgate(41.8, 5.7, 7.0, 43.8), // Gat.f
            rect(act, 1000.0, 1000.0, 1002.0, 1002.0),
            lgate(1000.8, 999.7, 1001.0, 1002.3), // Gat.f
            rect(act, 18.5, 10.0, 21.5, 12.0),
            rect(gp, 19.85, 9.7, 20.15, 12.3), // clean
        ],
    );
}

// --- GFil.a: max. GatPoly:filler width 5.00 ---

fn gfil_a_h(l: &L) {
    let gf = l.gfil;

    // Width is the distance between opposite edges of one shape (figure 4.1), so a
    // filler must have no two opposite edges more than 5.00 apart: 5 × 5 is the largest
    // legal box, and a 5 × 6 box is too wide across its 6.  (IHP's KLayout deck shrinks
    // by 2.5 instead, which only catches shapes wide in both directions.)

    // h1 — the bound and merges.  5 × 5 is legal; 5.005 × 5, 5 × 5.005 (one pair each)
    // and 5.005 × 5.005 (two) fire; a 300 × 5.005 bar fires across both its dimensions,
    // a 300 × 5 bar across its length.  The rule reads the union: two overlapping boxes
    // making 5 × 5 are clean, making 5.005 × 5 fire once; an L with 2.5 arms spanning 5
    // is clean, spanning 5.005 fires; a 4 × 4 grid of 1.25 boxes is clean; a ring 5
    // across is clean, 5.005 fires; 5.005 × 5 squares straddling x = 20 and x = 40 and one
    // at (1000, 1000) fire, a 5 × 5 straddling 20 is clean.
    let mut e = vec![
        rect(gf, 2.0, 2.0, 7.0, 7.0),       // clean
        rect(gf, 10.0, 2.0, 15.005, 7.0),   // GFil.a
        rect(gf, 18.0, 2.0, 23.0, 7.005),   // GFil.a
        rect(gf, 26.0, 2.0, 31.005, 7.005), // 2 × GFil.a
        rect(gf, 2.0, 10.0, 4.6, 15.0),
        rect(gf, 4.5, 10.0, 7.0, 15.0), // union 5 × 5 → clean
        rect(gf, 10.0, 10.0, 12.6, 15.0),
        rect(gf, 12.5, 10.0, 15.005, 15.0), // union 5.005 → GFil.a
        rect(gf, 18.0, 10.0, 20.5, 15.0),
        rect(gf, 18.0, 10.0, 23.0, 12.5), // L spanning 5 → clean
        rect(gf, 26.0, 10.0, 28.5, 15.0),
        rect(gf, 26.0, 10.0, 31.005, 12.5), // L spanning 5.005 → GFil.a
        rect(gf, 17.5, 18.0, 22.505, 23.0), // GFil.a, across x = 20
        rect(gf, 17.5, 26.0, 22.5, 31.0),   // clean, across x = 20
        rect(gf, 37.5, 18.0, 42.505, 23.0), // GFil.a, across x = 40
        rect(gf, 1000.0, 1000.0, 1005.005, 1005.0), // GFil.a
        rect(gf, 2.0, 34.0, 302.0, 39.005), // 2 × GFil.a (300 and 5.005)
        rect(gf, 2.0, 42.0, 302.0, 47.0),   // GFil.a (300)
    ];
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (2.0 + 1.25 * i as f64, 18.0 + 1.25 * j as f64);
            e.push(rect(gf, x, y, x + 1.25, y + 1.25)); // 5 × 5 grid → clean
        }
    }
    e.extend(l.ring(gf, 10.0, 18.0, 15.0, 23.0, 12.0, 20.0, 13.0, 21.0)); // clean
    e.extend(l.ring(gf, 26.0, 18.0, 31.005, 23.0, 28.0, 20.0, 29.0, 21.0)); // GFil.a
    write("GFil.a.h1", e);

    // h5 — merged shapes across tile lines.  A 300 × 5.005 bar merged with a 13 × 13 ring
    // hanging below it (the ring's hole 3 × 3), and a 300 × 5.005 bar merged with a 5 × 5.5
    // box below it: one shape each, wide across the bar and across the union's height.
    let mut e = vec![rect(gf, 2.0, 12.0, 302.0, 17.005)];
    e.extend(l.ring(gf, 46.0, 2.0, 59.0, 15.0, 51.005, 7.0, 54.0, 10.0));
    e.push(rect(gf, 2.0, 30.0, 302.0, 35.005));
    e.push(rect(gf, 46.0, 25.0, 51.0, 30.5));
    write("GFil.a.h5", e);
}

// --- GFil.d: min. GatPoly:filler space to Activ, GatPoly, Cont, pSD, nSD:block, SalBlock 1.10 ---

fn gfil_d_h(l: &L) {
    let gf = l.gfil;
    let others = [l.activ, l.gp, l.cont, l.psd, l.nsd_block, l.salblock];
    // A 1 × 1 box of `o` at (x, y), or a Cont (0.16 square) with its left/bottom edge there.
    let other = |o: (i16, i16), x: f64, y: f64| {
        if o == l.cont {
            cont_at(o, x + 0.08, y + 0.08)
        } else {
            rect(o, x, y, x + 1.0, y + 1.0)
        }
    };

    // h1 — the bound and both metrics, per layer (one row each).  1.10 is clean, 1.095
    // fires; a 0.77/0.77 diagonal (1.089) fires, 0.78/0.78 (1.103) is clean.
    let mut e = vec![];
    for (i, &o) in others.iter().enumerate() {
        let y = 2.0 + 5.0 * i as f64;
        e.push(rect(gf, 2.0, y, 3.0, y + 1.0));
        e.push(other(o, 4.1, y)); // clean
        e.push(rect(gf, 7.0, y, 8.0, y + 1.0));
        e.push(other(o, 9.095, y)); // GFil.d
        e.push(rect(gf, 12.0, y, 13.0, y + 1.0));
        e.push(other(o, 13.77, y + 1.77)); // GFil.d (1.089)
        e.push(rect(gf, 17.0, y, 18.0, y + 1.0));
        e.push(other(o, 18.78, y + 1.78)); // clean (1.103)
    }
    write("GFil.d.h1", e);

    // h2 — touching, overlapping, tile lines, far, long.  A filler abutting an Activ
    // (space 0.00) fires; a filler overlapping a pSD by 0.2, one wholly under a pSD and
    // one overlapping a GatPoly share area and are no pair; 1.095 gaps straddling x = 20 (to
    // Activ) and 40 (to GatPoly), a gap ending on 20 (to SalBlock), a 0.77/0.77 corner
    // pair to nSD:block near (20, 14); a pair at (1000, 1000); a 300 µm pair.
    write(
        "GFil.d.h2",
        vec![
            rect(gf, 2.0, 2.0, 3.0, 3.0),
            rect(l.activ, 3.0, 2.0, 4.0, 3.0), // GFil.d, abutting
            rect(gf, 6.0, 2.0, 7.0, 3.0),
            rect(l.psd, 6.8, 2.0, 8.0, 3.0), // no pair: overlap
            rect(l.psd, 10.0, 1.0, 13.0, 4.0),
            rect(gf, 11.0, 2.0, 12.0, 3.0), // no pair: inside
            rect(gf, 15.0, 2.0, 16.0, 3.0),
            rect(l.gp, 15.5, 2.2, 17.0, 2.5), // no pair: overlap
            rect(gf, 18.9, 6.0, 19.9, 7.0),
            rect(l.activ, 20.995, 6.0, 21.995, 7.0), // GFil.d
            rect(gf, 38.9, 6.0, 39.9, 7.0),
            rect(l.gp, 40.995, 6.0, 41.995, 7.0), // GFil.d
            rect(gf, 17.905, 9.0, 18.905, 10.0),
            rect(l.salblock, 20.0, 9.0, 21.0, 10.0), // GFil.d
            rect(gf, 18.0, 12.0, 19.0, 13.0),
            rect(l.nsd_block, 19.77, 13.77, 20.77, 14.77), // GFil.d (1.089)
            rect(gf, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(l.activ, 1002.095, 1000.0, 1003.095, 1001.0), // GFil.d
            rect(gf, 2.0, 20.0, 302.0, 21.0),
            rect(l.activ, 2.0, 22.095, 302.0, 23.095), // GFil.d
        ],
    );
}

// --- GFil.e: min. GatPoly:filler space to NWell, nBuLay 1.10 ---

fn gfil_e_h(l: &L) {
    let gf = l.gfil;

    // h1 — the bound and both metrics, to NWell and to nBuLay: 1.10 clean, 1.095 fires,
    // 0.77/0.77 (1.089) fires, 0.78/0.78 (1.103) clean.  Section 4.2 derives nBuLay from
    // wide NWells ("NWell ≥ 3.0 sized by 1.0/side"); IHP's deck reads that as a shrink,
    // so a filler 1.6 from a 5 × 5 NWell is clean, as is one 1.5 from a 2-wide NWell.
    let mut e = vec![];
    for (i, o) in [l.nw, l.nbl].iter().enumerate() {
        let y = 2.0 + 5.0 * i as f64;
        e.push(rect(gf, 2.0, y, 3.0, y + 1.0));
        e.push(rect(*o, 4.1, y, 5.1, y + 1.0)); // clean
        e.push(rect(gf, 7.0, y, 8.0, y + 1.0));
        e.push(rect(*o, 9.095, y, 10.095, y + 1.0)); // GFil.e
        e.push(rect(gf, 12.0, y, 13.0, y + 1.0));
        e.push(rect(*o, 13.77, y + 1.77, 14.77, y + 2.77)); // GFil.e
        e.push(rect(gf, 17.0, y, 18.0, y + 1.0));
        e.push(rect(*o, 18.78, y + 1.78, 19.78, y + 2.78)); // clean
    }
    e.push(rect(l.nw, 2.0, 14.0, 7.0, 19.0));
    e.push(rect(gf, 8.6, 16.0, 9.6, 17.0)); // clean (1.6 from a wide well)
    e.push(rect(l.nw, 12.0, 14.0, 14.0, 19.0));
    e.push(rect(gf, 15.5, 16.0, 16.5, 17.0)); // clean
    write("GFil.e.h1", e);

    // h2 — touching, overlapping, tile lines, far, long.  A filler abutting an NWell, one
    // wholly inside an NWell, one half over an nBuLay: space 0.00, all fire; 1.095 gaps
    // straddling x = 20 (NWell) and 40 (nBuLay); a pair at (1000, 1000); a 300 µm pair.
    write(
        "GFil.e.h2",
        vec![
            rect(gf, 2.0, 2.0, 3.0, 3.0),
            rect(l.nw, 3.0, 2.0, 5.0, 4.0), // GFil.e, abutting
            rect(l.nw, 7.0, 1.0, 11.0, 4.0),
            rect(gf, 8.0, 2.0, 9.0, 3.0), // GFil.e, inside
            rect(gf, 13.0, 2.0, 14.0, 3.0),
            rect(l.nbl, 13.5, 1.0, 16.0, 4.0), // GFil.e, overlap
            rect(gf, 18.9, 6.0, 19.9, 7.0),
            rect(l.nw, 20.995, 6.0, 21.995, 7.0), // GFil.e
            rect(gf, 38.9, 6.0, 39.9, 7.0),
            rect(l.nbl, 40.995, 6.0, 41.995, 7.0), // GFil.e
            rect(gf, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(l.nw, 1002.095, 1000.0, 1003.095, 1001.0), // GFil.e
            rect(gf, 2.0, 20.0, 302.0, 21.0),
            rect(l.nw, 2.0, 22.095, 302.0, 23.095), // GFil.e
        ],
    );
}

// --- GFil.f: min. GatPoly:filler space to TRANS 1.10 ---

fn gfil_f_h(l: &L) {
    let gf = l.gfil;
    let t = l.trans;

    // h1 — the bound, both metrics, touching, inside, tile lines, far, long.  1.10 is
    // clean, 1.095 fires; 0.77/0.77 (1.089) fires, 0.78/0.78 (1.103) is clean; a filler
    // abutting a TRANS and one inside it fire; 1.095 gaps straddling x = 20 and 40; a pair
    // at (1000, 1000); a 300 µm pair.
    write(
        "GFil.f.h1",
        vec![
            rect(gf, 2.0, 2.0, 3.0, 3.0),
            rect(t, 4.1, 2.0, 5.1, 3.0), // clean
            rect(gf, 7.0, 2.0, 8.0, 3.0),
            rect(t, 9.095, 2.0, 10.095, 3.0), // GFil.f
            rect(gf, 12.0, 2.0, 13.0, 3.0),
            rect(t, 13.77, 3.77, 14.77, 4.77), // GFil.f
            rect(gf, 17.0, 2.0, 18.0, 3.0),
            rect(t, 18.78, 3.78, 19.78, 4.78), // clean
            rect(gf, 2.0, 6.0, 3.0, 7.0),
            rect(t, 3.0, 6.0, 4.0, 7.0), // GFil.f, abutting
            rect(t, 6.0, 5.0, 9.0, 8.0),
            rect(gf, 7.0, 6.0, 8.0, 7.0), // GFil.f, inside
            rect(gf, 18.9, 10.0, 19.9, 11.0),
            rect(t, 20.995, 10.0, 21.995, 11.0), // GFil.f
            rect(gf, 38.9, 10.0, 39.9, 11.0),
            rect(t, 40.995, 10.0, 41.995, 11.0), // GFil.f
            rect(gf, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(t, 1002.095, 1000.0, 1003.095, 1001.0), // GFil.f
            rect(gf, 2.0, 20.0, 302.0, 21.0),
            rect(t, 2.0, 22.095, 302.0, 23.095), // GFil.f
        ],
    );
}

// --- GFil.g: min. global GatPoly density 15 % ---

fn gfil_g_h(l: &L) {
    // h1 — GatPoly and GatPoly:filler count once where they coincide: a 10 % GatPoly stripe
    // under a 10 % filler stripe over the same area is 10 %, not 20 %, and fires.  The
    // 1000-wide filler stripe is a GFil.a and its overlap with GatPoly a GFil.d.
    write(
        "GFil.g.h1",
        density_pattern(l.seal, 1000.0, &[(l.gp, 0.0, 100.0), (l.gfil, 0.0, 100.0)]),
    );
}

// --- GFil.i: max. GatPoly:nofill area 400 × 400 µm² ---

fn gfil_i_h(l: &L) {
    let nf = l.nofill;

    // h1 — the manual gives an area (160000 µm²).  A 400 × 400 is legal; 400.005 × 400
    // fires; an 800 × 199 (159200) and an L with a 500 × 500 bounding box but 90000 of
    // area are clean by the area; two overlapping 300 × 400 boxes whose union is 500 ×
    // 400 fire once.  Every shape here straddles tile lines.
    write(
        "GFil.i.h1",
        vec![
            rect(nf, 0.0, 0.0, 400.0, 400.0),     // clean
            rect(nf, 500.0, 0.0, 900.005, 400.0), // GFil.i
            rect(nf, 1000.0, 0.0, 1800.0, 199.0), // clean (area)
            rect(nf, 0.0, 500.0, 500.0, 600.0),
            rect(nf, 0.0, 500.0, 100.0, 1000.0), // L → clean (area)
            rect(nf, 1000.0, 500.0, 1300.0, 900.0),
            rect(nf, 1200.0, 500.0, 1500.0, 900.0), // union 500 × 400 → GFil.i
        ],
    );
}

// --- GFil.j: min. GatPoly:filler extension over Activ:filler (end cap) 0.18 ---

fn gfil_j_h(l: &L) {
    let gf = l.gfil;
    let af = l.afil;

    // h1 — the bound and no cap.  0.18 is legal; 0.175 at the bottom fires, at both ends
    // twice; a filler ending on the Activ:filler edge (0.00), a stub ending inside, and a
    // filler wholly inside fire; a horizontal filler 0.175 short on the left fires; a
    // 0.175 cap straddling x = 20 and one at (1000, 1000) fire.  Fillers are 0.8 wide
    // (GFil.b) and the Activ:filler 1 × 1.5.
    write(
        "GFil.j.h1",
        vec![
            rect(af, 2.0, 2.0, 3.0, 3.5),
            rect(gf, 2.1, 1.82, 2.9, 3.68), // clean
            rect(af, 5.0, 2.0, 6.0, 3.5),
            rect(gf, 5.1, 1.825, 5.9, 3.68), // GFil.j
            rect(af, 8.0, 2.0, 9.0, 3.5),
            rect(gf, 8.1, 1.825, 8.9, 3.675), // 2 × GFil.j
            rect(af, 11.0, 2.0, 12.0, 3.5),
            rect(gf, 11.1, 2.0, 11.9, 3.68), // GFil.j, 0.00
            rect(af, 14.0, 2.0, 15.0, 3.5),
            rect(gf, 14.1, 1.82, 14.9, 3.0), // GFil.j, stub
            rect(af, 17.0, 2.0, 18.0, 3.5),
            rect(gf, 17.1, 2.3, 17.9, 3.2), // GFil.j, inside
            rect(af, 2.0, 6.0, 3.5, 7.0),
            rect(gf, 1.825, 6.1, 3.68, 6.9), // GFil.j, horizontal
            rect(af, 19.5, 6.0, 20.5, 7.5),
            rect(gf, 19.6, 5.825, 20.4, 7.68), // GFil.j, across x = 20
            rect(af, 1000.0, 1000.0, 1001.0, 1001.5),
            rect(gf, 1000.1, 999.825, 1000.9, 1001.68), // GFil.j
        ],
    );
}
