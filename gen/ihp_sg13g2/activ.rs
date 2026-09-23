// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_bl, chamfered_tr, density_pattern, diamond, layer, library, max_width_pattern,
    min_width_pattern, notch_pattern, poly, rect, space_pattern, strip45, stripes, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/activ";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    act_a(pdk);
    act_b_space(pdk);
    act_b_notch(pdk);
    act_c(pdk);
    act_e(pdk);
    act_d(pdk);
    act_d_merge(pdk);
    afil_a(pdk);
    afil_a1(pdk);
    afil_b(pdk);
    afil_c_cont(pdk);
    afil_c_gatpoly(pdk);
    afil_c1(pdk);
    afil_d_nwell(pdk);
    afil_d_nbulay(pdk);
    afil_e(pdk);
    afil_i(pdk);
    afil_j(pdk);
    afil_g(pdk);
    afil_g1(pdk);
    afil_g2(pdk);
    afil_g3(pdk);
    afil_g2_boundary(pdk);
    afil_g2_boundary_ring(pdk);
    hardening(pdk);
}

fn act_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = min_width_pattern(l, 0.15, 0.15, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.a.gds.gz"), library("TOP", elems));
}

fn act_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = space_pattern(l, l, 1.0, 0.21, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.b.space.gds.gz"), library("TOP", elems));
}

fn act_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = notch_pattern(l, 0.15, 0.21, 1.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Act.b.notch.gds.gz"), library("TOP", elems));
}

/// Act.c — min. Activ drain/source extension 0.23 µm.  A gate crossing an Activ where
/// the S/D extends 0.30/1.34 µm (clean) and one where the right S/D extends only 0.14 µm
/// (violation); the gate-poly ends running out past the Activ are exempt.
fn act_c(pdk: &PdkConfig) {
    let activ = layer(pdk, "Activ");
    let gp = layer(pdk, "GatPoly");
    let o = OFFSET;
    let elems = vec![
        // clean transistor: gate at x∈[o+0.5, o+0.66] across Activ x∈[o, o+2]
        rect(activ, o, o, o + 2.0, o + 1.0),
        rect(gp, o + 0.5, o - 0.3, o + 0.66, o + 1.3),
        // violation: right S/D extension only 0.14 µm (Activ ends at o+4.6, gate at o+4.46)
        rect(activ, o + 4.0, o, o + 4.6, o + 1.0),
        rect(gp, o + 4.3, o - 0.3, o + 4.46, o + 1.3),
    ];
    write_gz(&format!("{DIR}/Act.c.gds.gz"), library("TOP", elems));
}

/// Act.e — min. Activ enclosed area 0.15 µm².  An Activ ring around a 0.3×0.3 = 0.09 µm²
/// hole violates; a ring around a 0.5×0.5 = 0.25 µm² hole is clean.  Holes are 0.3/0.5 µm
/// (≥ 0.21) so they don't read as notches.
fn act_e(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    // A square Activ ring of arm width `w` around an empty `hs`×`hs` hole at (x, y).
    let ring = |x: f64, y: f64, hs: f64, w: f64| {
        vec![
            rect(l, x - w, y - w, x + hs + w, y),           // bottom
            rect(l, x - w, y + hs, x + hs + w, y + hs + w), // top
            rect(l, x - w, y, x, y + hs),                   // left
            rect(l, x + hs, y, x + hs + w, y + hs),         // right
        ]
    };
    let o = OFFSET;
    let mut elems = ring(o + 0.3, o + 0.3, 0.30, 0.3); // hole 0.09 µm² → violation
    elems.extend(ring(o + 4.0, o + 0.3, 0.50, 0.3)); // hole 0.25 µm² → clean
    write_gz(&format!("{DIR}/Act.e.gds.gz"), library("TOP", elems));
}

fn act_d(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = vec![
        // clean: exactly at the limit
        rect(l, 0.0, 0.0, 0.35, 0.35),
        // too small
        rect(l, 0.0, 5.0, 0.345, 5.35),
    ];

    write_gz(&format!("{DIR}/Act.d.gds.gz"), library("TOP", elems));
}

/// Edge case: two abutting rectangles, each below the 0.122 µm² floor (0.2×0.35 =
/// 0.07), merge into one 0.4×0.35 = 0.14 µm² region → clean.  Confirms `min_area`
/// is measured per merged region, not per shape (per shape, both would fail).
fn act_d_merge(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ");
    let elems = vec![
        rect(l, 0.0, 0.0, 0.20, 0.35),
        rect(l, 0.20, 0.0, 0.40, 0.35), // abuts the first along x = 0.20
    ];

    write_gz(&format!("{DIR}/Act.d.merge.gds.gz"), library("TOP", elems));
}

/// AFil.a — max. Activ:filler width 5.00, read as the narrowest dimension: a 5.005 × 5.0
/// and a 5.0 × 5.005 filler are 5.0 wide and clean, a 5.005 × 5.005 one is not.
fn afil_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let mut elems = max_width_pattern(l, 5.0, 5.0, 20.0, OFFSET, SPACE_DELTA);
    elems.push(rect(l, OFFSET + 60.0, 0.0, OFFSET + 65.005, 5.005));
    write_gz(&format!("{DIR}/AFil.a.gds.gz"), library("TOP", elems));
}

fn afil_a1(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let elems = min_width_pattern(l, 1.0, 1.0, 20.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.a1.gds.gz"), library("TOP", elems));
}

fn afil_b(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let elems = space_pattern(l, l, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.b.gds.gz"), library("TOP", elems));
}

/// AFil.c — min. Activ:filler space to Cont (1.10 µm).
fn afil_c_cont(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let cont = layer(pdk, "Cont");
    let elems = space_pattern(l, cont, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.c.cont.gds.gz"), library("TOP", elems));
}

/// AFil.c — min. Activ:filler space to GatPoly (1.10 µm).
fn afil_c_gatpoly(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let gp = layer(pdk, "GatPoly");
    let elems = space_pattern(l, gp, 2.0, 1.10, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{DIR}/AFil.c.gatpoly.gds.gz"),
        library("TOP", elems),
    );
}

fn afil_c1(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let act = layer(pdk, "Activ");
    let elems = space_pattern(l, act, 1.0, 0.42, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.c1.gds.gz"), library("TOP", elems));
}

/// AFil.d — min. Activ:filler space to NWell (1.00 µm).
fn afil_d_nwell(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.d.nwell.gds.gz"), library("TOP", elems));
}

/// AFil.d — min. Activ:filler space to nBuLay (1.00 µm).
fn afil_d_nbulay(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let nbl = layer(pdk, "nBuLay");
    let elems = space_pattern(l, nbl, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{DIR}/AFil.d.nbulay.gds.gz"),
        library("TOP", elems),
    );
}

/// AFil.e — min. Activ:filler space to TRANS (1.00 µm).
fn afil_e(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let trans = layer(pdk, "TRANS");
    let elems = space_pattern(l, trans, 2.0, 1.00, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.e.gds.gz"), library("TOP", elems));
}

/// AFil.i — min. Activ:filler space to edges of PWell:block (1.50 µm).
fn afil_i(pdk: &PdkConfig) {
    let l = layer(pdk, "Activ.filler");
    let blk = layer(pdk, "PWell.block");
    let elems = space_pattern(l, blk, 2.0, 1.50, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/AFil.i.gds.gz"), library("TOP", elems));
}

/// AFil.j — (nSD:block ∩ SalBlock) must enclose Activ:filler-inside-PWell:block by
/// 0.25 µm.  One filler enclosed by 0.25 (clean); one with a 0.24 left margin (fail).
fn afil_j(pdk: &PdkConfig) {
    let fill = layer(pdk, "Activ.filler");
    let pwb = layer(pdk, "PWell.block");
    let nsdb = layer(pdk, "nSD.block");
    let sal = layer(pdk, "SalBlock");
    let o = OFFSET;
    // A filler inside PWell:block, enclosed by both nSD:block and SalBlock by `margin`.
    let cell = |x: f64, margin: f64| {
        vec![
            rect(fill, x, o, x + 1.0, o + 1.0),
            rect(pwb, x - 0.1, o - 0.1, x + 1.1, o + 1.1), // filler is inside the block
            rect(nsdb, x - margin, o - 0.25, x + 1.25, o + 1.25),
            rect(sal, x - margin, o - 0.25, x + 1.25, o + 1.25),
        ]
    };
    let mut elems = cell(o, 0.25); // 0.25 all round → clean
    elems.extend(cell(o + 4.0, 0.24)); // 0.24 left → violation
    write_gz(&format!("{DIR}/AFil.j.gds.gz"), library("TOP", elems));
}

fn afil_g(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let afil = layer(pdk, "Activ.filler");
    let amask = layer(pdk, "Activ.mask");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density: bottom Activ stripe drops below the 35 % floor when too short.
    let stripes = |h: f64| [(act, 0.0, h), (afil, 500.0, 600.0), (amask, 900.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{DIR}/AFil.g.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(149.99));
    write_gz(
        &format!("{DIR}/AFil.g.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

fn afil_g1(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let afil = layer(pdk, "Activ.filler");
    let amask = layer(pdk, "Activ.mask");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // max_density: bottom Activ stripe rises above the 55 % ceiling when too tall.
    let stripes = |h: f64| [(act, 0.0, h), (afil, 400.0, 600.0), (amask, 800.0, 1000.0)];

    let elems = density_pattern(boundary, 1000.0, &stripes(150.0));
    write_gz(&format!("{DIR}/AFil.g1.gds.gz"), library("TOP", elems));

    let elems_fail = density_pattern(boundary, 1000.0, &stripes(150.01));
    write_gz(
        &format!("{DIR}/AFil.g1.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

fn afil_g2(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // min_density in any 800 µm window: uniform stripes read the same in every window,
    // 26 % is clean, 24 % fails everywhere - one violation, the windows overlap.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend(stripes(act, 1000.0, 26.0));
    write_gz(&format!("{DIR}/AFil.g2.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend(stripes(act, 1000.0, 24.0));
    write_gz(
        &format!("{DIR}/AFil.g2.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

fn afil_g3(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    // max_density in any 800 µm window: 64 % is clean, 66 % fails everywhere, one
    // violation.
    let mut elems = density_pattern(boundary, 1000.0, &[]);
    elems.extend(stripes(act, 1000.0, 64.0));
    write_gz(&format!("{DIR}/AFil.g3.gds.gz"), library("TOP", elems));

    let mut elems_fail = density_pattern(boundary, 1000.0, &[]);
    elems_fail.extend(stripes(act, 1000.0, 66.0));
    write_gz(
        &format!("{DIR}/AFil.g3.fail.gds.gz"),
        library("TOP", elems_fail),
    );
}

/// AFil.g2/g3 boundary handling: the chip's raw bounding box (from *all* shapes)
/// extends past the true EdgeSeal — a small unrelated marker on TRANS sits outside the
/// seal ring, at (950, 950)-(1000, 1000), stretching the overall bbox from the sealed
/// 900x900 die out to 1000x1000.  With an 800 µm window this makes the last row/column
/// of tiles straddle the seal boundary, so their `boundary`-clipped area (only the
/// part actually inside EdgeSeal) must be used as the density denominator — not the
/// nominal (and here doubled) window footprint.  Uniform 40% fill (period-100, height-40
/// stripes) sits comfortably inside [25%, 65%] everywhere, including the
/// boundary-straddling tiles, so a clean DRC here proves the fix; without it, the last
/// row/column would be measured against a doubled denominator and register a false ~20%
/// underfill.
fn afil_g2_boundary(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    let trans = layer(pdk, "TRANS");

    let mut elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
    ];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(act, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(
        &format!("{DIR}/AFil.g2.boundary_ok.gds.gz"),
        library("TOP", elems),
    );
}

/// Same as [`afil_g2_boundary`], but EdgeSeal is drawn as a hollow ring (4 strips)
/// around the die instead of a solid square — a real seal ring is a frame, not a filled
/// shape, so its own merged *area* is only the thin frame material, far smaller than the
/// 900x900 it encloses.  `boundary` must fall back on the ring's bounding box (its
/// die extent), not its drawn area, or the density denominator collapses to almost
/// nothing and every window reads a wildly inflated (1000%+) density.  Same uniform 40%
/// fill and out-of-seal TRANS marker; expect a clean DRC exactly as with a solid
/// boundary square.
fn afil_g2_boundary_ring(pdk: &PdkConfig) {
    let act = layer(pdk, "Activ");
    let boundary = layer(pdk, "EdgeSeal.boundary");
    let trans = layer(pdk, "TRANS");
    let frame = 20.0;

    let mut elems = vec![
        rect(boundary, 0.0, 0.0, 900.0, frame),
        rect(boundary, 0.0, 900.0 - frame, 900.0, 900.0),
        rect(boundary, 0.0, 0.0, frame, 900.0),
        rect(boundary, 900.0 - frame, 0.0, 900.0, 900.0),
        rect(trans, 950.0, 950.0, 1000.0, 1000.0),
    ];
    for k in 0..=8 {
        let y0 = k as f64 * 100.0;
        elems.push(rect(act, 0.0, y0, 900.0, y0 + 40.0));
    }
    write_gz(
        &format!("{DIR}/AFil.g2.boundary_ring.gds.gz"),
        library("TOP", elems),
    );
}

// ---------------------------------------------------------------------------------------
// Hardening patterns (hardening/SPEC.md): layouts drawn from the manual's sections 5.5
// (Activ) and 5.6 (Activ:filler) alone, one fixture per theme, `<rule>.h<n>`.  Each
// function's comment states the geometry and what the manual says about it; the expected
// answers are in the `activ` table of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/activ.md.
// ---------------------------------------------------------------------------------------

/// Layers the hardening patterns draw on.
struct L {
    activ: (i16, i16),
    amask: (i16, i16),
    afil: (i16, i16),
    gp: (i16, i16),
    gpfil: (i16, i16),
    cont: (i16, i16),
    nw: (i16, i16),
    nbl: (i16, i16),
    nblb: (i16, i16),
    trans: (i16, i16),
    pwb: (i16, i16),
    nsdb: (i16, i16),
    sal: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    boundary: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            activ: layer(pdk, "Activ"),
            amask: layer(pdk, "Activ.mask"),
            afil: layer(pdk, "Activ.filler"),
            gp: layer(pdk, "GatPoly"),
            gpfil: layer(pdk, "GatPoly.filler"),
            cont: layer(pdk, "Cont"),
            nw: layer(pdk, "NWell"),
            nbl: layer(pdk, "nBuLay"),
            nblb: layer(pdk, "nBuLay.block"),
            trans: layer(pdk, "TRANS"),
            pwb: layer(pdk, "PWell.block"),
            nsdb: layer(pdk, "nSD.block"),
            sal: layer(pdk, "SalBlock"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            boundary: layer(pdk, "EdgeSeal.boundary"),
        }
    }
}

/// Square ring on `l`: outer box `(x0, y0)-(x1, y1)` minus the hole `(hx0, hy0)-(hx1, hy1)`,
/// drawn as four overlapping wall rectangles that merge into one ring.
#[allow(clippy::too_many_arguments)]
fn ring(
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

/// The same ring as one keyhole polygon (GDS has no holes: the outline runs in along a
/// zero-width cut, round the hole and back out).
#[allow(clippy::too_many_arguments)]
fn keyhole(
    l: (i16, i16),
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    hx0: f64,
    hy0: f64,
    hx1: f64,
    hy1: f64,
) -> GdsElement {
    keyhole_poly(
        l,
        x0,
        y0,
        x1,
        y1,
        &[(hx0, hy0), (hx0, hy1), (hx1, hy1), (hx1, hy0)],
    )
}

/// A box `(x0, y0)-(x1, y1)` holding a hole given by its vertex list; the keyhole cut runs
/// from the box's left wall to the hole's first vertex, which must be its leftmost point.
fn keyhole_poly(
    l: (i16, i16),
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    hole: &[(f64, f64)],
) -> GdsElement {
    let (hx, hy) = hole[0];
    let mut pts = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, hy)];
    pts.extend(hole.iter().copied());
    pts.push((hx, hy));
    pts.push((x0, hy));
    poly(l, &pts)
}

/// A transistor: Activ `(x0, y0)-(x1, y1)` crossed by a vertical gate `gx0..gx1` running
/// 0.3 past the Activ's top and bottom.
fn fet_v(l: &L, x0: f64, y0: f64, x1: f64, y1: f64, gx0: f64, gx1: f64) -> Vec<GdsElement> {
    vec![
        rect(l.activ, x0, y0, x1, y1),
        rect(l.gp, gx0, y0 - 0.3, gx1, y1 + 0.3),
    ]
}

/// A transistor with a horizontal gate `gy0..gy1` running 0.3 past the Activ's sides.
fn fet_h(l: &L, x0: f64, y0: f64, x1: f64, y1: f64, gy0: f64, gy1: f64) -> Vec<GdsElement> {
    vec![
        rect(l.activ, x0, y0, x1, y1),
        rect(l.gp, x0 - 0.3, gy0, x1 + 0.3, gy1),
    ]
}

/// A 1 × 1 Activ:filler at `(x, y)`.
fn fil(l: &L, x: f64, y: f64) -> GdsElement {
    rect(l.afil, x, y, x + 1.0, y + 1.0)
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// Where the tile-line layouts put a gap or an edge of width `g`: well inside a tile
/// (ending at 10), ending on 20, straddling 20, starting on 20, straddling 21, ending on
/// 40, straddling 42.  Returns the x the feature starts at, one row per entry.
fn tile_xs(g: f64) -> [f64; 7] {
    let half = (g / 2.0 * 200.0).round() / 200.0; // on the 0.005 grid
    [
        10.0 - g,
        20.0 - g,
        20.0 - half,
        20.0,
        21.0 - half,
        40.0 - g,
        42.0 - half,
    ]
}

/// A space pattern for the two-layer tile-line layouts: a 1 × 1 box on `a` whose right
/// edge is at `x`, a box on `b` (width `w`) starting `gap` further right, in row `row`.
fn gap_pair(a: (i16, i16), b: (i16, i16), x: f64, gap: f64, w: f64, row: f64) -> Vec<GdsElement> {
    vec![
        rect(a, x - 1.0, row, x, row + 1.0),
        rect(b, x + gap, row, x + gap + w, row + 1.0),
    ]
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    act_a_h(&l);
    act_b_h(&l);
    act_c_h(&l);
    act_d_h(&l);
    act_e_h(&l);
    afil_a_h(&l);
    afil_a1_h(&l);
    afil_b_h(&l);
    afil_c_h(&l);
    afil_c1_h(&l);
    afil_d_h(&l);
    afil_e_h(&l);
    afil_i_h(&l);
    afil_j_h(&l);
    afil_g_h(&l);
}

// --- Act.a: min. Activ width 0.15 ---

fn act_a_h(l: &L) {
    let a = l.activ;

    // h1 — the bound and a long shape.  0.15 wide is legal, 0.145 (one grid step under) is
    // not, in x and in y; a 300 µm bar crossing every tile line counts once.
    write(
        "Act.a.h1",
        vec![
            rect(a, 2.0, 2.0, 2.15, 4.0),      // clean
            rect(a, 5.0, 2.0, 5.145, 4.0),     // Act.a (x)
            rect(a, 8.0, 2.0, 10.0, 2.145),    // Act.a (y)
            rect(a, 2.0, 8.0, 302.0, 8.15),    // clean, 300 µm long
            rect(a, 2.0, 12.0, 302.0, 12.145), // Act.a, 300 µm long → one violation
        ],
    );

    // h7 — a 0.005 sliver (one grid step; also under Act.d's area) and a bar far from
    // everything at (1000, 1000).
    write(
        "Act.a.h7",
        vec![
            rect(a, 2.0, 2.0, 2.005, 4.0),
            rect(a, 1000.0, 1000.0, 1000.145, 1002.0),
        ],
    );
}

// --- Act.b: min. Activ space or notch 0.21 ---

fn act_b_h(l: &L) {
    let a = l.activ;

    // h4 — shapes that merge: overlapping, abutting and gridded boxes, each union 0.205
    // from a third box; one violation each.
    let mut e = vec![
        rect(a, 2.0, 2.0, 2.6, 3.0),
        rect(a, 2.4, 2.0, 3.0, 3.0),
        rect(a, 3.205, 2.0, 4.205, 3.0), // Act.b
        rect(a, 6.0, 2.0, 6.5, 3.0),
        rect(a, 6.5, 2.0, 7.0, 3.0),
        rect(a, 7.205, 2.0, 8.205, 3.0),   // Act.b
        rect(a, 11.205, 2.0, 12.205, 3.0), // Act.b
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (10.0 + 0.2 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(a, x, y, x + 0.2, y + 0.2));
        }
    }
    write("Act.b.h4", e);

    // h8 — a 0.005 sliver 0.205 from a box (the sliver is also Act.a and Act.d), two
    // 300 µm bars 0.205 apart (one violation), and a pair at (1000, 1000).
    write(
        "Act.b.h8",
        vec![
            rect(a, 2.0, 2.0, 2.005, 4.0),
            rect(a, 2.21, 2.0, 3.21, 4.0), // Act.b
            rect(a, 2.0, 6.0, 302.0, 7.0),
            rect(a, 2.0, 7.205, 302.0, 8.205), // Act.b
            rect(a, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(a, 1001.205, 1000.0, 1002.205, 1001.0), // Act.b
        ],
    );

    // h9 — what the rule applies to.  Activ 0.205 from an Activ:filler is AFil.c1's, not
    // Act.b; Activ 0.205 from Activ.mask is nobody's; a P+Activ 0.205 from an N+Activ is
    // Act.b (one layer, whatever the implant); two Activ 0.205 apart under one GatPoly
    // are Act.b (the poly does not join them).
    write(
        "Act.b.h9",
        vec![
            rect(a, 2.0, 2.0, 3.0, 3.0),
            fil(l, 3.205, 2.0), // AFil.c1 only
            rect(a, 6.0, 2.0, 7.0, 3.0),
            rect(l.psd, 5.9, 1.9, 7.1, 3.1),
            rect(a, 7.205, 2.0, 8.205, 3.0),
            rect(l.nsd, 7.105, 1.9, 8.305, 3.1), // Act.b
            rect(a, 10.0, 2.0, 11.0, 3.0),
            rect(l.amask, 11.205, 2.0, 12.205, 3.0), // nothing
            rect(a, 14.0, 2.0, 15.0, 3.0),
            rect(a, 15.205, 2.0, 16.205, 3.0),
            rect(l.gp, 14.5, 1.7, 15.7, 3.3), // Act.b
        ],
    );
}

// --- Act.c: min. Activ drain/source extension 0.23 ---

fn act_c_h(l: &L) {
    let a = l.activ;
    let gp = l.gp;

    // h1 — the bound.  A gate 0.16 wide across a 0.62 Activ leaves 0.23 either side
    // (clean); a 0.615 Activ leaves 0.225 on the right (fires), a gate shifted by one step
    // leaves 0.225 on the left (fires); the same with a horizontal gate (top 0.225 fires);
    // a 0.61 Activ leaves 0.225 on both sides (two violations).
    let mut e = fet_v(l, 2.0, 2.0, 2.62, 3.0, 2.23, 2.39); // clean
    e.extend(fet_v(l, 5.0, 2.0, 5.615, 3.0, 5.23, 5.39)); // Act.c (right 0.225)
    e.extend(fet_v(l, 8.0, 2.0, 8.62, 3.0, 8.225, 8.385)); // Act.c (left 0.225)
    e.extend(fet_h(l, 11.0, 2.0, 12.0, 2.62, 2.23, 2.39)); // clean
    e.extend(fet_h(l, 14.0, 2.0, 15.0, 2.615, 2.23, 2.39)); // Act.c (top 0.225)
    e.extend(fet_v(l, 17.0, 2.0, 17.61, 3.0, 17.225, 17.385)); // 2 × Act.c
    write("Act.c.h1", e);

    // h2 — 45° geometry.  A transistor turned by 45°: the Activ is a 45° strip (ends on
    // x + y = 4 and x + y = 10), the gate a band between x + y = 4.325 and 4.555, so the
    // lower S/D is 0.325/√2 = 0.2298 long (fires); a second strip (ends on 16 and 22)
    // with the band at 16.33 leaves 0.2333 (clean); a third (ends on 28 and 34) with the
    // band at 28.14 leaves 0.099 (fires, grossly).  A straight gate on a chamfered Activ whose chamfer passes 0.17 from
    // the gate's corner while the walls are 0.64 apart is clean under the projection
    // reading (settled).
    write(
        "Act.c.h2",
        vec![
            strip45(a, 2.0, 2.0, 3.0, 0.5),
            poly(
                gp,
                &[(2.415, 1.91), (2.53, 2.025), (1.53, 3.025), (1.415, 2.91)],
            ), // Act.c (0.2298)
            strip45(a, 8.0, 8.0, 3.0, 0.5),
            poly(
                gp,
                &[(8.415, 7.915), (8.53, 8.03), (7.53, 9.03), (7.415, 8.915)],
            ), // clean (0.2333)
            strip45(a, 14.0, 14.0, 3.0, 0.5),
            poly(
                gp,
                &[
                    (14.32, 13.82),
                    (14.435, 13.935),
                    (13.435, 14.935),
                    (13.32, 14.82),
                ],
            ), // Act.c (0.099, a gross one)
            chamfered_tr(a, 12.0, 2.0, 14.0, 3.0, 16.6),
            rect(gp, 13.2, 1.7, 13.36, 3.3), // clean by projection (0.64 to the wall)
        ],
    );

    // h3 — what the rule applies to, and shapes that merge.  GatPoly 0.1 beside an Activ
    // and GatPoly abutting an Activ's edge from outside have no S/D (nothing); a gate
    // ending 0.1 inside the Activ is Gat.c's business, the Activ past a gate end is no
    // drain/source (nothing); an Activ from two abutting boxes where the S/D is the
    // second box is read as the union (0.61 → clean); a gate from two overlapping poly
    // boxes reads the union (0.23/0.23 clean, and one shifted to 0.225 fires once, not
    // twice); a hole in the S/D 0.16 from the gate is a 0.16 S/D (fires); a gate across
    // a 0.005 Activ sliver has long S/Ds (nothing for Act.c).
    let mut e = vec![
        rect(a, 2.0, 2.0, 3.0, 3.0),
        rect(gp, 3.1, 1.7, 3.26, 3.3), // beside: nothing
        rect(a, 5.0, 2.0, 6.0, 3.0),
        rect(gp, 6.0, 1.7, 6.16, 3.3), // abutting: nothing
        rect(a, 8.0, 2.0, 9.0, 3.0),
        rect(gp, 8.4, 1.7, 8.56, 2.9), // gate ends inside: nothing (Gat.c)
        rect(a, 11.0, 2.0, 11.5, 3.0),
        rect(a, 11.5, 2.0, 12.0, 3.0),
        rect(gp, 11.23, 1.7, 11.39, 3.3), // union S/D 0.61: clean
        rect(a, 14.0, 2.0, 14.62, 3.0),
        rect(gp, 14.23, 1.7, 14.33, 3.3),
        rect(gp, 14.29, 1.7, 14.39, 3.3), // union gate 14.23..14.39: clean
        rect(a, 17.0, 2.0, 17.62, 3.0),
        rect(gp, 17.23, 1.7, 17.33, 3.3),
        rect(gp, 17.295, 1.7, 17.395, 3.3), // union gate to 17.395: Act.c once
        keyhole(a, 2.0, 6.0, 4.0, 7.0, 2.9, 6.25, 3.2, 6.75),
        rect(gp, 2.58, 5.7, 2.74, 7.3), // hole 0.16 right of the gate: Act.c
        rect(a, 6.0, 6.0, 6.005, 7.0),
        rect(gp, 5.7, 6.3, 6.3, 6.46), // sliver: nothing for Act.c
    ];
    e.push(rect(a, 9.0, 6.0, 10.0, 7.0)); // a plain Activ, nothing
    write("Act.c.h3", e);

    // h7 — a 300 µm wide transistor with a 0.225 top S/D, a 300 µm long gate across a
    // small Activ with a 0.225 right S/D, and one at (1000, 1000).
    let mut e = fet_h(l, 2.0, 2.0, 302.0, 2.615, 2.23, 2.39);
    e.extend([
        rect(a, 2.0, 6.0, 2.615, 7.0),
        rect(gp, 2.23, 5.0, 2.39, 305.0),
    ]);
    e.extend(fet_v(l, 1000.0, 1000.0, 1000.615, 1001.0, 1000.23, 1000.39));
    write("Act.c.h7", e);

    // h8 — more than one gate.  Two fingers on one Activ with 0.23/0.35 outer S/Ds are
    // clean; with the right one at 0.225 they fire once; one gate across two Activs with
    // 0.23 S/Ds is clean, and fires once when the upper Activ is 0.615 wide.
    write(
        "Act.c.h8",
        vec![
            rect(a, 2.0, 2.0, 3.2, 3.0),
            rect(gp, 2.23, 1.7, 2.39, 3.3),
            rect(gp, 2.69, 1.7, 2.85, 3.3), // clean
            rect(a, 5.0, 2.0, 6.075, 3.0),
            rect(gp, 5.23, 1.7, 5.39, 3.3),
            rect(gp, 5.69, 1.7, 5.85, 3.3), // Act.c (right 0.225)
            rect(a, 8.0, 2.0, 8.62, 3.0),
            rect(a, 8.0, 3.5, 8.62, 4.5),
            rect(gp, 8.23, 1.7, 8.39, 4.8), // clean
            rect(a, 11.0, 2.0, 11.62, 3.0),
            rect(a, 11.0, 3.5, 11.615, 4.5),
            rect(gp, 11.23, 1.7, 11.39, 4.8), // Act.c (upper right 0.225)
        ],
    );
}

// --- Act.d: min. Activ area 0.122 µm² ---

fn act_d_h(l: &L) {
    let a = l.activ;

    // h1 — the bound, in more than one shape.  0.305 × 0.40 = 0.122 is legal, 0.305 ×
    // 0.395 = 0.1205 is not; 0.20 × 0.61 = 0.122 and 0.20 × 0.605 = 0.121; an L of
    // 0.4 × 0.2 + 0.2 × 0.21 = 0.122 and one of 0.4 × 0.2 + 0.2 × 0.205 = 0.121; a diamond
    // of half-diagonal 0.25 (0.125) and of 0.245 (0.120).
    write(
        "Act.d.h1",
        vec![
            rect(a, 2.0, 2.0, 2.305, 2.4),   // clean
            rect(a, 4.0, 2.0, 4.305, 2.395), // Act.d
            rect(a, 6.0, 2.0, 6.2, 2.61),    // clean
            rect(a, 8.0, 2.0, 8.2, 2.605),   // Act.d
            poly(
                a,
                &[
                    (10.0, 2.0),
                    (10.4, 2.0),
                    (10.4, 2.2),
                    (10.2, 2.2),
                    (10.2, 2.41),
                    (10.0, 2.41),
                ],
            ), // clean
            poly(
                a,
                &[
                    (12.0, 2.0),
                    (12.4, 2.0),
                    (12.4, 2.2),
                    (12.2, 2.2),
                    (12.2, 2.405),
                    (12.0, 2.405),
                ],
            ), // Act.d
            diamond(a, 14.5, 2.5, 0.25),     // clean
            diamond(a, 16.5, 2.5, 0.245),    // Act.d
        ],
    );

    // h2 — shapes that merge.  Two 0.3 × 0.3 boxes (0.09 each, 0.18 together) overlapping
    // into a 0.4 × 0.3 = 0.12 union fire once: the area is the union's, not the sum; a
    // 0.30 × 0.40 region drawn as a 6 × 8 grid of 0.05 boxes fires once, the same grid
    // with a 0.01 row on top (0.123) is clean; two abutting 0.2 × 0.3 boxes (0.12) fire
    // once; a ring whose walls hold 0.12 of material around a 0.2 hole fires (its box
    // would be 0.16; the ring is also Act.a, Act.b and Act.e), a ring of 0.16 material is
    // clean (Act.e only); an island of 0.09 in a ring's hole fires; two 0.3 boxes touching
    // at one corner only are two regions of 0.09: two violations.
    let mut e = vec![
        rect(a, 2.0, 2.0, 2.3, 2.3),
        rect(a, 2.2, 2.0, 2.5, 2.3), // union 0.15 → clean
        rect(a, 4.0, 2.0, 4.3, 2.3),
        rect(a, 4.1, 2.0, 4.4, 2.3), // union 0.12 → Act.d
        rect(a, 10.0, 2.0, 10.2, 2.3),
        rect(a, 10.2, 2.0, 10.4, 2.3), // abutting, 0.12 → Act.d
    ];
    for i in 0..6 {
        for j in 0..8 {
            let (x, y) = (6.0 + 0.05 * i as f64, 2.0 + 0.05 * j as f64);
            e.push(rect(a, x, y, x + 0.05, y + 0.05)); // 0.30 × 0.40 grid → Act.d
            let (x, y) = (8.0 + 0.05 * i as f64, 2.0 + 0.05 * j as f64);
            e.push(rect(a, x, y, x + 0.05, y + 0.05));
        }
    }
    e.push(rect(a, 8.0, 2.4, 8.3, 2.41)); // grid + 0.01 row = 0.123 → clean
    e.extend(ring(a, 12.0, 2.0, 12.4, 2.4, 12.1, 2.1, 12.3, 2.3)); // 0.12 → Act.d
    e.extend(ring(a, 14.0, 2.0, 14.5, 2.5, 14.1, 2.1, 14.4, 2.4)); // 0.16 → clean
    e.extend(ring(a, 16.0, 2.0, 17.2, 3.2, 16.2, 2.2, 17.0, 3.0));
    e.push(rect(a, 16.45, 2.45, 16.75, 2.75)); // island 0.09 → Act.d
    e.extend([
        rect(a, 2.0, 5.0, 2.3, 5.3),
        rect(a, 2.3, 5.3, 2.6, 5.6), // corner-touching: 2 × Act.d
    ]);
    write("Act.d.h2", e);

    // h6 — a 0.005 × 2 sliver (0.01; also Act.a), a 0.005 × 30 sliver whose area is 0.15
    // (Act.a only), and a 0.12 box at (1000, 1000).
    write(
        "Act.d.h6",
        vec![
            rect(a, 2.0, 2.0, 2.005, 4.0),           // Act.d
            rect(a, 2.0, 6.0, 32.0, 6.005),          // clean for Act.d
            rect(a, 1000.0, 1000.0, 1000.3, 1000.4), // Act.d
        ],
    );
}

// --- Act.e: min. Activ enclosed area 0.15 µm² ---

fn act_e_h(l: &L) {
    let a = l.activ;

    // h1 — the bound.  A ring around a 0.30 × 0.50 hole (0.15) is clean, around 0.30 ×
    // 0.495 (0.1485) fires; 0.25 × 0.60 clean, 0.25 × 0.595 fires; a diamond hole of
    // half-diagonal 0.28 (0.157) is clean, of 0.27 (0.146) fires; a 0.40 hole with four
    // 0.10 chamfers (0.14) fires, the plain 0.40 hole (0.16) is clean.
    let dia = |cx: f64, cy: f64, h: f64| [(cx - h, cy), (cx, cy + h), (cx + h, cy), (cx, cy - h)];
    let cham = |x: f64, y: f64, s: f64, c: f64| {
        [
            (x, y + c),
            (x, y + s - c),
            (x + c, y + s),
            (x + s - c, y + s),
            (x + s, y + s - c),
            (x + s, y + c),
            (x + s - c, y),
            (x + c, y),
        ]
    };
    let mut e = ring(a, 2.0, 2.0, 2.9, 3.1, 2.3, 2.3, 2.6, 2.8); // clean
    e.extend(ring(a, 4.0, 2.0, 4.9, 3.095, 4.3, 2.3, 4.6, 2.795)); // Act.e
    e.extend(ring(a, 6.0, 2.0, 6.85, 3.2, 6.3, 2.3, 6.55, 2.9)); // clean
    e.extend(ring(a, 8.0, 2.0, 8.85, 3.195, 8.3, 2.3, 8.55, 2.895)); // Act.e
    e.extend([
        keyhole_poly(a, 10.0, 2.0, 11.2, 3.2, &dia(10.6, 2.6, 0.28)), // clean
        keyhole_poly(a, 12.0, 2.0, 13.2, 3.2, &dia(12.6, 2.6, 0.27)), // Act.e
        keyhole_poly(a, 14.0, 2.0, 15.0, 3.0, &cham(14.3, 2.3, 0.4, 0.1)), // Act.e
        keyhole(a, 16.0, 2.0, 17.0, 3.0, 16.3, 2.3, 16.7, 2.7),       // clean
    ]);
    write("Act.e.h1", e);

    // h2 — shapes that merge, islands, nesting.  Two C shapes abutting close a 0.3 × 0.4
    // hole (fires); the same Cs 0.005 apart leave no hole (Act.b, not Act.e); a box
    // abutting two walls of a 0.46 hole leaves an L of 0.1491 (fires); a 0.34 island in a
    // 0.5 hole leaves 0.1344 of enclosed area (fires - the enclosed area is what is empty;
    // the island is Act.d and its gaps Act.b); a 0.4 island in a 0.9 hole leaves 0.65
    // (clean); a ring inside a ring's hole: the inner hole (0.078) fires, the annulus
    // (0.66) is clean; a ring drawn as 88 boxes of 0.1 around a 0.3 × 0.4 hole fires; the
    // keyhole polygon fires drawn counter-clockwise and drawn clockwise.
    let c_pair = |x: f64, dx: f64| {
        vec![
            poly(
                a,
                &[
                    (x, 2.0),
                    (x + 0.55, 2.0),
                    (x + 0.55, 2.3),
                    (x + 0.3, 2.3),
                    (x + 0.3, 2.7),
                    (x + 0.55, 2.7),
                    (x + 0.55, 3.0),
                    (x, 3.0),
                ],
            ),
            poly(
                a,
                &[
                    (x + 0.55 + dx, 2.0),
                    (x + 0.9 + dx, 2.0),
                    (x + 0.9 + dx, 3.0),
                    (x + 0.55 + dx, 3.0),
                    (x + 0.55 + dx, 2.7),
                    (x + 0.6 + dx, 2.7),
                    (x + 0.6 + dx, 2.3),
                    (x + 0.55 + dx, 2.3),
                ],
            ),
        ]
    };
    let mut e = c_pair(2.0, 0.0); // Act.e
    e.extend(c_pair(5.0, 0.005)); // no hole
    e.extend(ring(a, 14.0, 2.0, 15.06, 3.06, 14.3, 2.3, 14.76, 2.76));
    e.push(rect(a, 14.51, 2.51, 14.76, 2.76)); // L-shaped hole 0.1491: Act.e
    e.extend(ring(a, 17.0, 2.0, 18.1, 3.1, 17.3, 2.3, 17.8, 2.8));
    e.push(rect(a, 17.38, 2.38, 17.72, 2.72)); // enclosed 0.1344: Act.e
    e.extend(ring(a, 2.0, 6.0, 3.5, 7.5, 2.3, 6.3, 3.2, 7.2));
    e.push(rect(a, 2.55, 6.55, 2.95, 6.95)); // enclosed 0.65: clean
    e.extend(ring(a, 5.0, 6.0, 6.6, 7.6, 5.3, 6.3, 6.3, 7.3));
    e.extend(ring(a, 5.51, 6.51, 6.09, 7.09, 5.66, 6.66, 5.94, 6.94)); // inner hole: Act.e
    for i in 0..10 {
        for j in 0..10 {
            if (3..6).contains(&i) && (3..7).contains(&j) {
                continue; // the 0.3 × 0.4 hole
            }
            let (x, y) = (8.0 + 0.1 * i as f64, 6.0 + 0.1 * j as f64);
            e.push(rect(a, x, y, x + 0.1, y + 0.1));
        }
    }
    e.extend([
        keyhole(a, 11.0, 6.0, 12.0, 7.0, 11.3, 6.3, 11.6, 6.7), // Act.e
        poly(
            a,
            &[
                (14.0, 7.0),
                (15.0, 7.0),
                (15.0, 6.0),
                (14.0, 6.0),
                (14.0, 6.3),
                (14.3, 6.3),
                (14.6, 6.3),
                (14.6, 6.7),
                (14.3, 6.7),
                (14.3, 6.3),
                (14.0, 6.3),
            ],
        ), // the same, clockwise: Act.e
    ]);
    write("Act.e.h2", e);

    // h3 — tile lines.  1 × 1.1 rings with a 0.3 × 0.4 hole well inside a tile and with
    // the hole ending on, straddling and starting on x = 20, straddling 21, ending on 40,
    // straddling 42 (seven violations); a 100 µm ring with a 0.3 × 0.4 hole at x = 50
    // (one); a 0.30 × 0.50 hole straddling 20 (clean), a 0.25 × 5.4 hole across 20 (clean),
    // a U whose 0.25 slot runs from x = 0.8 to 60 and is open at 60 (no hole, and no
    // Act.b at 0.25).
    let mut e = vec![];
    for (i, x) in tile_xs(0.30).iter().enumerate() {
        let y = 2.0 + 1.5 * i as f64;
        e.extend(ring(
            a,
            x - 0.3,
            y,
            x + 0.6,
            y + 1.0,
            *x,
            y + 0.3,
            x + 0.3,
            y + 0.7,
        ));
    }
    e.extend(ring(a, 2.0, 13.0, 102.0, 14.0, 49.85, 13.3, 50.15, 13.7)); // Act.e
    e.extend(ring(a, 19.5, 15.0, 20.5, 16.1, 19.85, 15.3, 20.15, 15.8)); // clean
    e.extend(ring(a, 17.0, 17.0, 23.0, 17.85, 17.3, 17.3, 22.7, 17.55)); // clean
    e.push(poly(
        a,
        &[
            (0.5, 19.0),
            (60.0, 19.0),
            (60.0, 19.3),
            (0.8, 19.3),
            (0.8, 19.55),
            (60.0, 19.55),
            (60.0, 19.85),
            (0.5, 19.85),
        ],
    ));
    write("Act.e.h3", e);

    // h6 — a 0.005 × 0.5 hole (0.0025; also an Act.b notch), a 0.12 hole at (1000, 1000),
    // and a 300 µm ring whose hole is huge (clean).
    let mut e = ring(a, 2.0, 2.0, 2.6, 3.0, 2.3, 2.25, 2.305, 2.75); // Act.e
    e.extend(ring(
        a, 1000.0, 1000.0, 1001.0, 1001.1, 1000.3, 1000.3, 1000.6, 1000.7,
    )); // Act.e
    e.extend(ring(a, 2.0, 6.0, 302.0, 10.0, 2.3, 6.3, 301.7, 9.7)); // clean
    write("Act.e.h6", e);
}

// --- AFil.a: max. Activ:filler width 5.00 ---

fn afil_a_h(l: &L) {
    let f = l.afil;

    // h1 — the bound.  A 5 × 5 filler is legal, 5.005 × 5.005 is not.  A 5.005 × 5.0
    // filler is 5.0 wide (its width is the smaller span, as for a minimum width), so it
    // is legal too - as is a 3 × 20 bar and an L with 3-wide arms.
    write(
        "AFil.a.h1",
        vec![
            rect(f, 2.0, 2.0, 7.0, 7.0),       // clean
            rect(f, 10.0, 2.0, 15.005, 7.005), // AFil.a
            rect(f, 18.0, 2.0, 23.005, 7.0),   // clean (width 5.0)
            rect(f, 2.0, 10.0, 22.0, 13.0),    // clean
            poly(
                f,
                &[
                    (26.0, 2.0),
                    (36.0, 2.0),
                    (36.0, 5.0),
                    (29.0, 5.0),
                    (29.0, 12.0),
                    (26.0, 12.0),
                ],
            ), // clean
        ],
    );

    // h2 — 45° geometry and shapes that merge.  A diamond of half-diagonal 3.55 is 5.02
    // between its walls (fires), 3.5 is 4.95 (clean); a 45° strip of d = 3.55 (5.02) fires,
    // 3.5 is clean; two 3 × 6 boxes overlapping into a 5.005 × 6 union fire, into 5.0 × 6
    // are clean; a 6 × 6 filler with a 0.5 hole in its middle is 2.75 wide everywhere
    // (clean); a 5.005 square drawn as four boxes fires.
    write(
        "AFil.a.h2",
        vec![
            diamond(f, 6.0, 6.0, 3.55),       // AFil.a
            diamond(f, 16.0, 6.0, 3.5),       // clean
            strip45(f, 26.0, 2.0, 8.0, 3.55), // AFil.a
            strip45(f, 44.0, 2.0, 8.0, 3.5),  // clean
            rect(f, 2.0, 20.0, 5.0, 26.0),
            rect(f, 4.005, 20.0, 7.005, 26.0), // union 5.005: AFil.a
            rect(f, 10.0, 20.0, 13.0, 26.0),
            rect(f, 12.0, 20.0, 15.0, 26.0), // union 5.0: clean
            keyhole(f, 20.0, 20.0, 26.0, 26.0, 22.75, 22.75, 23.25, 23.25), // clean
            rect(f, 30.0, 20.0, 32.5, 22.5),
            rect(f, 32.5, 20.0, 35.005, 22.5),
            rect(f, 30.0, 22.5, 32.5, 25.005),
            rect(f, 32.5, 22.5, 35.005, 25.005), // union 5.005 square: AFil.a
        ],
    );

    // h6 — a 300 × 5.005 bar (one violation) and a 5.005 square at (1000, 1000).
    write(
        "AFil.a.h6",
        vec![
            rect(f, 2.0, 2.0, 302.0, 7.005),
            rect(f, 1000.0, 1000.0, 1005.005, 1005.005),
        ],
    );
}

// --- AFil.a1: min. Activ:filler width 1.00 ---

fn afil_a1_h(l: &L) {
    let f = l.afil;

    // h1 — the bound: 1.0 wide is legal, 0.995 is not, in x and in y; a 300 µm bar 0.995
    // tall counts once.
    write(
        "AFil.a1.h1",
        vec![
            rect(f, 2.0, 2.0, 3.0, 5.0),       // clean
            rect(f, 5.0, 2.0, 5.995, 5.0),     // AFil.a1
            rect(f, 8.0, 2.0, 11.0, 2.995),    // AFil.a1
            rect(f, 2.0, 8.0, 302.0, 9.0),     // clean
            rect(f, 2.0, 12.0, 302.0, 12.995), // AFil.a1
        ],
    );

    // h3 — shapes that merge: overlapping 0.6 boxes whose union is 1.0 (clean) or 0.995
    // (fires once); four abutting 0.25 slices (clean) and 3 × 0.25 + 0.245 (fires); a
    // 1.0 bar as a 4 × 10 grid (clean); a ring with one 0.995 side; a 0.995 island in a
    // ring's hole.
    let mut e = vec![
        rect(f, 2.0, 2.0, 2.6, 5.0),
        rect(f, 2.4, 2.0, 3.0, 5.0), // clean
        rect(f, 5.0, 2.0, 5.6, 5.0),
        rect(f, 5.395, 2.0, 5.995, 5.0), // AFil.a1
    ];
    for i in 0..4 {
        e.push(rect(
            f,
            8.0 + 0.25 * i as f64,
            2.0,
            8.25 + 0.25 * i as f64,
            5.0,
        )); // clean
    }
    for i in 0..3 {
        e.push(rect(
            f,
            11.0 + 0.25 * i as f64,
            2.0,
            11.25 + 0.25 * i as f64,
            5.0,
        ));
    }
    e.push(rect(f, 11.75, 2.0, 11.995, 5.0)); // AFil.a1
    for i in 0..4 {
        for j in 0..10 {
            let (x, y) = (14.0 + 0.25 * i as f64, 2.0 + 0.3 * j as f64);
            e.push(rect(f, x, y, x + 0.25, y + 0.3)); // clean
        }
    }
    e.extend(ring(f, 2.0, 8.0, 8.0, 14.0, 2.995, 9.5, 6.5, 12.5)); // AFil.a1 (left side)
    e.extend(ring(f, 10.0, 8.0, 16.0, 14.0, 11.5, 9.5, 14.5, 12.5));
    e.push(rect(f, 12.5, 10.0, 13.495, 12.0)); // island 0.995: AFil.a1
    write("AFil.a1.h3", e);

    // h7 — a 0.005 sliver and a 0.995 bar at (1000, 1000).
    write(
        "AFil.a1.h7",
        vec![
            rect(f, 2.0, 2.0, 2.005, 5.0),
            rect(f, 1000.0, 1000.0, 1000.995, 1003.0),
        ],
    );

    // h8 — a comb with three 0.995 teeth (three violations) and a U with 1.0 arms (clean).
    write(
        "AFil.a1.h8",
        vec![
            poly(
                f,
                &[
                    (2.0, 2.0),
                    (10.0, 2.0),
                    (10.0, 4.0),
                    (8.995, 4.0),
                    (8.995, 7.0),
                    (8.0, 7.0),
                    (8.0, 4.0),
                    (5.995, 4.0),
                    (5.995, 7.0),
                    (5.0, 7.0),
                    (5.0, 4.0),
                    (2.995, 4.0),
                    (2.995, 7.0),
                    (2.0, 7.0),
                ],
            ),
            poly(
                f,
                &[
                    (14.0, 2.0),
                    (18.0, 2.0),
                    (18.0, 7.0),
                    (17.0, 7.0),
                    (17.0, 4.0),
                    (15.0, 4.0),
                    (15.0, 7.0),
                    (14.0, 7.0),
                ],
            ),
        ],
    );
}

// --- AFil.b: min. Activ:filler space 0.42 ---

fn afil_b_h(l: &L) {
    let f = l.afil;

    // h3 — notches and near-notches.  The manual says "space", not "space or notch"
    // (compare Act.b), so a 0.415 notch into a filler (straight, both orientations) is
    // not AFil.b's; two facing Ls 0.415 apart and a 0.415 island in a filler ring are
    // separate shapes and fire.
    let mut e = notch_pattern(f, 1.0, 0.42, 2.0, 2.0, SPACE_DELTA);
    e.extend([
        poly(
            f,
            &[
                (2.0, 6.0),
                (6.0, 6.0),
                (6.0, 7.0),
                (3.0, 7.0),
                (3.0, 10.0),
                (2.0, 10.0),
            ],
        ),
        poly(
            f,
            &[
                (3.415, 7.415),
                (6.0, 7.415),
                (6.0, 11.0),
                (5.0, 11.0),
                (5.0, 8.415),
                (3.415, 8.415),
            ],
        ), // AFil.b
    ]);
    e.extend(ring(f, 8.0, 6.0, 13.0, 11.0, 9.0, 7.0, 12.0, 10.0));
    e.push(rect(f, 9.415, 7.5, 11.0, 9.5)); // AFil.b
    write("AFil.b.h3", e);

    // h4 — shapes that merge: overlapping, abutting and gridded boxes, each union 0.415
    // from a third filler; one violation each.
    let mut e = vec![
        rect(f, 2.0, 2.0, 2.6, 3.0),
        rect(f, 2.4, 2.0, 3.0, 3.0),
        fil(l, 3.415, 2.0), // AFil.b
        rect(f, 6.0, 2.0, 6.5, 3.0),
        rect(f, 6.5, 2.0, 7.0, 3.0),
        fil(l, 7.415, 2.0),  // AFil.b
        fil(l, 11.415, 2.0), // AFil.b
    ];
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (10.0 + 0.2 * i as f64, 2.0 + 0.2 * j as f64);
            e.push(rect(f, x, y, x + 0.2, y + 0.2));
        }
    }
    write("AFil.b.h4", e);

    // h8 — a 0.005 sliver 0.415 from a filler (the sliver is AFil.a1's too), two 300 µm
    // bars 0.415 apart, a pair at (1000, 1000).
    write(
        "AFil.b.h8",
        vec![
            rect(f, 2.0, 2.0, 2.005, 4.0),
            rect(f, 2.42, 2.0, 3.42, 4.0), // AFil.b
            rect(f, 2.0, 6.0, 302.0, 7.0),
            rect(f, 2.0, 7.415, 302.0, 8.415), // AFil.b
            fil(l, 1000.0, 1000.0),
            fil(l, 1001.415, 1000.0), // AFil.b
        ],
    );

    // h9 — what the rule applies to: a filler 0.415 from an Activ is AFil.c1's, from an
    // Activ.mask nobody's; no AFil.b either way.
    write(
        "AFil.b.h9",
        vec![
            fil(l, 2.0, 2.0),
            rect(l.activ, 3.415, 2.0, 4.415, 3.0),
            fil(l, 6.0, 2.0),
            rect(l.amask, 7.415, 2.0, 8.415, 3.0),
        ],
    );
}

// --- AFil.c: min. Activ:filler space to Cont, GatPoly 1.10 ---

fn afil_c_h(l: &L) {
    let f = l.afil;
    let (cont, gp) = (l.cont, l.gp);
    // A 0.16 Cont with its bottom-left corner at (x, y).
    let ct = |x: f64, y: f64| rect(cont, x, y, x + 0.16, y + 0.16);

    // h2 — 45°: a GatPoly chamfer passing 1.096 from a filler's corner fires, 1.103 is
    // clean; a GatPoly diamond tip 1.095 above a filler fires.
    write(
        "AFil.c.h2",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            chamfered_bl(gp, 4.5, 4.5, 7.0, 7.0, 9.55), // AFil.c (1.096)
            rect(f, 10.0, 2.0, 12.0, 4.0),
            chamfered_bl(gp, 12.5, 4.5, 15.0, 7.0, 17.56), // clean (1.103)
            rect(f, 18.0, 2.0, 20.0, 4.0),
            diamond(gp, 19.0, 6.095, 1.0), // AFil.c (tip 1.095 above the wall)
        ],
    );

    // h3 — what the rule applies to.  A 0.16 × 1.0 Cont bar 1.095 away is Cont (fires);
    // a GatPoly:filler 1.095 away is not GatPoly (nothing); a Cont on an Activ whose
    // Activ is 1.0 from the filler but whose Cont is 1.095 fires (AFil.c1 is clean at
    // 1.0); a Cont overlapping the filler's edge is at no distance at all (fires).
    write(
        "AFil.c.h3",
        vec![
            fil(l, 2.0, 2.0),
            rect(cont, 4.095, 2.0, 4.255, 3.0), // AFil.c
            fil(l, 8.0, 2.0),
            rect(l.gpfil, 10.095, 2.0, 11.095, 3.0), // nothing
            fil(l, 14.0, 2.0),
            rect(l.activ, 16.0, 1.9, 17.0, 3.1),
            ct(16.095, 2.4), // AFil.c
            fil(l, 20.0, 2.0),
            ct(20.9, 2.4), // AFil.c (overlap)
        ],
    );

    // h7 — a 300 µm GatPoly 1.095 above a 300 µm filler (one violation) and a pair at
    // (1000, 1000).
    write(
        "AFil.c.h7",
        vec![
            rect(f, 2.0, 2.0, 302.0, 3.0),
            rect(gp, 2.0, 4.095, 302.0, 4.5),
            fil(l, 1000.0, 1000.0),
            ct(1002.095, 1000.0),
        ],
    );
}

// --- AFil.c1: min. Activ:filler space to Activ 0.42 ---

fn afil_c1_h(l: &L) {
    let f = l.afil;
    let a = l.activ;

    // h2 — 45°: an Activ diamond tip 0.415 above a filler fires; an Activ 45° strip 0.415
    // from a filler strip fires; a filler chamfer 0.417 from an Activ corner fires.
    write(
        "AFil.c1.h2",
        vec![
            rect(f, 2.0, 2.0, 5.0, 3.0),
            diamond(a, 3.5, 3.915, 0.5), // AFil.c1
            strip45(f, 8.0, 2.0, 4.0, 0.8),
            strip45(a, 8.0, 4.187, 4.0, 0.3), // AFil.c1 (0.4151)
            chamfered_tr(f, 2.0, 7.0, 5.0, 10.0, 14.0),
            rect(a, 4.795, 9.795, 6.0, 11.0), // AFil.c1 (0.417)
        ],
    );

    // h3 — what the rule applies to.  Activ.mask 0.415 from a filler is nothing; a
    // filler 0.415 from a filler is AFil.b's; an Activ abutting a filler's edge is at no
    // distance (fires); an Activ overlapping a filler's edge likewise (fires).
    write(
        "AFil.c1.h3",
        vec![
            fil(l, 2.0, 2.0),
            rect(l.amask, 3.415, 2.0, 4.415, 3.0), // nothing
            fil(l, 6.0, 2.0),
            fil(l, 7.415, 2.0), // AFil.b only
            fil(l, 10.0, 2.0),
            rect(a, 11.0, 2.0, 12.0, 3.0), // AFil.c1 (abutting)
            fil(l, 14.0, 2.0),
            rect(a, 14.8, 2.2, 15.8, 2.8), // AFil.c1 (overlapping)
        ],
    );

    // h4 — tile lines: 0.415 gaps to an Activ well inside a tile and ending on, straddling
    // and starting on x = 20, straddling 21, ending on 40, straddling 42; an Activ 0.415
    // above a 10 µm filler across 20/21.  Eight violations.
    let mut e = vec![];
    for (i, x) in tile_xs(0.415).iter().enumerate() {
        e.extend(gap_pair(f, a, *x, 0.415, 1.0, 2.0 + 2.0 * i as f64));
    }
    e.extend([
        rect(f, 15.0, 17.0, 25.0, 18.0),
        rect(a, 15.0, 18.415, 25.0, 19.0), // AFil.c1
    ]);
    write("AFil.c1.h4", e);

    // h7 — a 0.005 Activ sliver 0.415 from a filler (the sliver is Act.a and Act.d), two
    // 300 µm bars 0.415 apart, a pair at (1000, 1000).
    write(
        "AFil.c1.h7",
        vec![
            fil(l, 2.0, 2.0),
            rect(a, 3.415, 2.0, 3.42, 3.0), // AFil.c1
            rect(f, 2.0, 6.0, 302.0, 7.0),
            rect(a, 2.0, 7.415, 302.0, 8.0), // AFil.c1
            fil(l, 1000.0, 1000.0),
            rect(a, 1001.415, 1000.0, 1002.415, 1001.0), // AFil.c1
        ],
    );
}

// --- AFil.d: min. Activ:filler space to NWell, nBuLay 1.00 ---

fn afil_d_h(l: &L) {
    let f = l.afil;
    let (nw, nbl) = (l.nw, l.nbl);

    // h1 — the bound and both metrics, for NWell and for a drawn nBuLay: 0.995 fires,
    // 1.00 is clean; a 0.705/0.705 diagonal (0.997) fires, 0.71/0.71 (1.004) is clean.
    // The wells are 2 wide, under the 3.0 from which section 4.2 derives an nBuLay.
    write(
        "AFil.d.h1",
        vec![
            fil(l, 2.0, 2.0),
            rect(nw, 3.995, 2.0, 5.995, 4.0), // AFil.d
            fil(l, 8.0, 2.0),
            rect(nw, 10.0, 2.0, 12.0, 4.0), // clean
            fil(l, 14.0, 2.0),
            rect(nbl, 15.995, 2.0, 17.995, 4.0), // AFil.d
            fil(l, 20.0, 2.0),
            rect(nbl, 22.0, 2.0, 24.0, 4.0), // clean
            fil(l, 2.0, 8.0),
            rect(nw, 3.705, 9.705, 5.705, 11.705), // AFil.d (0.997)
            fil(l, 8.0, 8.0),
            rect(nw, 9.71, 9.71, 11.71, 11.71), // clean (1.004)
            fil(l, 14.0, 8.0),
            rect(nbl, 15.705, 9.705, 17.705, 11.705), // AFil.d (0.997)
        ],
    );

    // h2 — inside.  Figure 5.6 draws "d" from a filler inside a NWell to the well's edge
    // as well as from one outside: a filler 0.5 inside a NWell's edge fires, 1.0 inside is
    // clean; the same for nBuLay; a filler crossing a NWell's edge and one crossing an
    // nBuLay's edge share area and are no pair (the test's comment says how the
    // section 4.2 nBuLay moves the readings).
    write(
        "AFil.d.h2",
        vec![
            rect(nw, 2.0, 2.0, 5.0, 5.0),
            fil(l, 2.5, 3.0), // AFil.d (0.5 inside)
            rect(nw, 8.0, 2.0, 12.0, 6.0),
            fil(l, 9.0, 3.0), // clean (1.0 inside)
            rect(nbl, 15.0, 2.0, 18.0, 5.0),
            fil(l, 15.5, 3.0), // AFil.d (0.5 inside)
            rect(nbl, 21.0, 2.0, 25.0, 6.0),
            fil(l, 22.0, 3.0), // clean
            rect(nw, 2.0, 9.0, 5.0, 12.0),
            fil(l, 4.5, 10.0), // AFil.d (crossing)
            rect(nbl, 15.0, 9.0, 18.0, 12.0),
            fil(l, 17.5, 10.0), // AFil.d (crossing)
        ],
    );

    // h3 — the derived nBuLay.  Section 4.2: nBuLay = ((NWell ≥ 3.0 µm) sized by 1.0/side
    // OR nBuLay:drawing) AND NOT nBuLay:block.  A filler 1.5 from a 3.0 × 3.0 NWell is
    // 0.5 from that well's nBuLay (fires); 1.5 from a 2.995 × 3.0 well there is no nBuLay
    // (clean); 2.0 from a 3.0 well is clean.  A drawn nBuLay wholly under nBuLay:block is
    // no nBuLay (0.5 away: clean); one whose left half is blocked fires for a filler 0.5
    // right of the unblocked half and not for one 0.5 left of the blocked half.
    write(
        "AFil.d.h3",
        vec![
            rect(nw, 2.0, 2.0, 5.0, 5.0),
            fil(l, 6.5, 3.0), // AFil.d (0.5 from the derived nBuLay)
            rect(nw, 10.0, 2.0, 12.995, 5.0),
            fil(l, 14.495, 3.0), // clean
            rect(nw, 18.0, 2.0, 21.0, 5.0),
            fil(l, 23.0, 3.0), // clean (2.0)
            rect(nbl, 2.0, 9.0, 4.0, 11.0),
            rect(l.nblb, 1.0, 8.0, 5.0, 12.0),
            fil(l, 4.5, 9.5), // clean (blocked)
            rect(nbl, 10.0, 9.0, 12.0, 11.0),
            rect(l.nblb, 9.0, 8.0, 11.0, 12.0),
            fil(l, 12.5, 9.5), // AFil.d (0.5 from the unblocked half)
            fil(l, 8.5, 9.5),  // clean (0.5 from the blocked half, 2.5 from the rest)
        ],
    );

    // h4 — 45°: a NWell diamond tip 0.995 above a filler fires; a NWell chamfer 0.997 from
    // a filler's corner fires, 1.004 is clean.
    write(
        "AFil.d.h4",
        vec![
            rect(f, 2.0, 2.0, 5.0, 3.0),
            diamond(nw, 3.5, 4.995, 1.0), // AFil.d
            rect(f, 8.0, 2.0, 10.0, 4.0),
            chamfered_bl(nw, 10.5, 4.5, 13.0, 7.0, 15.41), // AFil.d (0.997)
            rect(f, 16.0, 2.0, 18.0, 4.0),
            chamfered_bl(nw, 18.5, 4.5, 21.0, 7.0, 23.42), // clean (1.004)
        ],
    );

    // h5 — tile lines: 0.995 gaps to a NWell well inside a tile and ending on, straddling
    // and starting on x = 20, straddling 21, ending on 40, straddling 42; a NWell 0.995
    // above a 10 µm filler across 20/21.  Eight violations.
    let mut e = vec![];
    for (i, x) in tile_xs(0.995).iter().enumerate() {
        e.extend(gap_pair(f, nw, *x, 0.995, 1.0, 2.0 + 3.0 * i as f64));
    }
    e.extend([
        rect(f, 15.0, 23.0, 25.0, 24.0),
        rect(nw, 15.0, 24.995, 25.0, 26.0), // AFil.d
    ]);
    write("AFil.d.h5", e);

    // h8 — a 300 µm NWell 0.995 above a 300 µm filler (one violation) and a pair at
    // (1000, 1000).
    write(
        "AFil.d.h8",
        vec![
            rect(f, 2.0, 2.0, 302.0, 3.0),
            rect(nw, 2.0, 3.995, 302.0, 5.0),
            fil(l, 1000.0, 1000.0),
            rect(nw, 1001.995, 1000.0, 1003.0, 1001.0),
        ],
    );
}

// --- AFil.e: min. Activ:filler space to TRANS 1.00 ---

fn afil_e_h(l: &L) {
    let f = l.afil;
    let t = l.trans;

    // h1 — the bound and both metrics: 0.995 fires, 1.00 is clean; a 0.705/0.705 diagonal
    // (0.997) fires, 0.71/0.71 (1.004) is clean; a corner-on 0.995 fires.
    write(
        "AFil.e.h1",
        vec![
            fil(l, 2.0, 2.0),
            rect(t, 3.995, 2.0, 5.0, 3.0), // AFil.e
            fil(l, 8.0, 2.0),
            rect(t, 10.0, 2.0, 11.0, 3.0), // clean
            fil(l, 14.0, 2.0),
            rect(t, 15.705, 3.705, 16.705, 4.705), // AFil.e (0.997)
            fil(l, 20.0, 2.0),
            rect(t, 21.71, 3.71, 22.71, 4.71), // clean (1.004)
            fil(l, 2.0, 8.0),
            rect(t, 3.995, 9.0, 5.0, 10.0), // AFil.e, corner-on
        ],
    );

    // h2 — inside and across.  A filler inside a TRANS marker and one crossing its edge
    // are at no distance (fire); a TRANS diamond tip 0.995 above a filler fires.
    write(
        "AFil.e.h2",
        vec![
            rect(t, 2.0, 2.0, 6.0, 6.0),
            fil(l, 3.5, 3.5), // AFil.e (inside)
            rect(t, 10.0, 2.0, 14.0, 6.0),
            fil(l, 13.5, 3.5), // AFil.e (crossing)
            rect(f, 18.0, 2.0, 21.0, 3.0),
            diamond(t, 19.5, 4.995, 1.0), // AFil.e (tip)
        ],
    );

    // h3 — tile lines: 0.995 gaps to a TRANS well inside a tile and ending on, straddling
    // and starting on x = 20, straddling 21, ending on 40, straddling 42; a TRANS 0.995
    // above a 10 µm filler across 20/21.  Eight violations.
    let mut e = vec![];
    for (i, x) in tile_xs(0.995).iter().enumerate() {
        e.extend(gap_pair(f, t, *x, 0.995, 1.0, 2.0 + 2.0 * i as f64));
    }
    e.extend([
        rect(f, 15.0, 17.0, 25.0, 18.0),
        rect(t, 15.0, 18.995, 25.0, 20.0), // AFil.e
    ]);
    write("AFil.e.h3", e);

    // h6 — a 300 µm TRANS 0.995 above a 300 µm filler (one violation) and a pair at
    // (1000, 1000).
    write(
        "AFil.e.h6",
        vec![
            rect(f, 2.0, 2.0, 302.0, 3.0),
            rect(t, 2.0, 3.995, 302.0, 5.0),
            fil(l, 1000.0, 1000.0),
            rect(t, 1001.995, 1000.0, 1003.0, 1001.0),
        ],
    );
}

// --- AFil.i: min. Activ:filler space to edges of PWell:block 1.50 ---

fn afil_i_h(l: &L) {
    let f = l.afil;
    let b = l.pwb;

    // h1 — the bound and both metrics, from outside: 1.495 fires, 1.50 is clean; a
    // 1.06/1.06 diagonal (1.499) fires, 1.065/1.065 (1.506) is clean; a corner-on 1.495
    // fires.
    write(
        "AFil.i.h1",
        vec![
            fil(l, 2.0, 2.0),
            rect(b, 4.495, 2.0, 6.0, 3.0), // AFil.i
            fil(l, 8.0, 2.0),
            rect(b, 10.5, 2.0, 12.0, 3.0), // clean
            fil(l, 14.0, 2.0),
            rect(b, 16.06, 4.06, 17.5, 5.5), // AFil.i (1.499)
            fil(l, 20.0, 2.0),
            rect(b, 22.065, 4.065, 23.5, 5.5), // clean (1.506)
            fil(l, 2.0, 8.0),
            rect(b, 4.495, 9.0, 6.0, 10.0), // AFil.i, corner-on
        ],
    );

    // h2 — inside.  Figure 5.6 draws "i" on both sides of the PWell:block edge: a filler
    // inside a block 1.495 from its edge fires (nSD:block and SalBlock enclose it so
    // AFil.j stays quiet), 1.50 inside is clean; a filler crossing the block's edge is at
    // no distance (fires).
    let inside = |x: f64, margin: f64| {
        vec![
            rect(b, x, 2.0, x + 6.0, 8.0),
            fil(l, x + margin, 4.0),
            rect(l.nsdb, x + margin - 0.3, 3.7, x + margin + 1.3, 5.3),
            rect(l.sal, x + margin - 0.3, 3.7, x + margin + 1.3, 5.3),
        ]
    };
    let mut e = inside(2.0, 1.495); // AFil.i
    e.extend(inside(12.0, 1.5)); // clean
    e.extend([
        rect(b, 22.0, 2.0, 28.0, 8.0),
        fil(l, 21.5, 4.0), // AFil.i (crossing)
        rect(l.nsdb, 21.2, 3.7, 22.8, 5.3),
        rect(l.sal, 21.2, 3.7, 22.8, 5.3),
    ]);
    write("AFil.i.h2", e);

    // h3 — 45°: a PWell:block chamfer 1.499 from a filler's corner fires; a block diamond
    // tip 1.495 above a filler fires.
    write(
        "AFil.i.h3",
        vec![
            rect(f, 2.0, 2.0, 4.0, 4.0),
            chamfered_bl(b, 4.5, 4.5, 8.0, 8.0, 10.12), // AFil.i (1.499)
            rect(f, 12.0, 2.0, 15.0, 3.0),
            diamond(b, 13.5, 5.995, 1.5), // AFil.i
        ],
    );

    // h4 — tile lines: 1.495 gaps to a block well inside a tile and ending on, straddling
    // and starting on x = 20, straddling 21, ending on 40, straddling 42; a block 1.495
    // above a 10 µm filler across 20/21.  Eight violations.
    let mut e = vec![];
    for (i, x) in tile_xs(1.495).iter().enumerate() {
        e.extend(gap_pair(f, b, *x, 1.495, 1.0, 2.0 + 3.0 * i as f64));
    }
    e.extend([
        rect(f, 15.0, 23.0, 25.0, 24.0),
        rect(b, 15.0, 25.495, 25.0, 26.5), // AFil.i
    ]);
    write("AFil.i.h4", e);

    // h7 — a 300 µm block 1.495 above a 300 µm filler (one violation) and a pair at
    // (1000, 1000).
    write(
        "AFil.i.h7",
        vec![
            rect(f, 2.0, 2.0, 302.0, 3.0),
            rect(b, 2.0, 4.495, 302.0, 5.5),
            fil(l, 1000.0, 1000.0),
            rect(b, 1002.495, 1000.0, 1003.5, 1001.0),
        ],
    );
}

// --- AFil.j: min. nSD:block and SalBlock enclosure of Activ:filler inside PWell:block 0.25 ---

fn afil_j_h(l: &L) {
    let f = l.afil;

    // A 1 × 1 filler at (x, y) inside a PWell:block with 2.0 margins, nSD:block and
    // SalBlock around it with the given left margins and 0.30 elsewhere.
    let cell = |x: f64, y: f64, nsd_l: f64, sal_l: f64| {
        vec![
            fil(l, x, y),
            rect(l.pwb, x - 2.0, y - 2.0, x + 3.0, y + 3.0),
            rect(l.nsdb, x - nsd_l, y - 0.3, x + 1.3, y + 1.3),
            rect(l.sal, x - sal_l, y - 0.3, x + 1.3, y + 1.3),
        ]
    };

    // h1 — the bound.  Both blocks at 0.25: clean.  nSD:block at 0.245 (SalBlock 0.25)
    // fires; SalBlock at 0.245 fires; both at 0.245 fire (one filler edge, one
    // violation); nSD:block ending on the filler's edge (0.00) fires.
    let mut e = cell(2.0, 2.0, 0.25, 0.25); // clean
    e.extend(cell(10.0, 2.0, 0.245, 0.25)); // AFil.j
    e.extend(cell(18.0, 2.0, 0.25, 0.245)); // AFil.j
    e.extend(cell(26.0, 2.0, 0.245, 0.245)); // AFil.j
    e.extend(cell(34.0, 2.0, 0.0, 0.25)); // AFil.j
    write("AFil.j.h1", e);

    // h2 — what the rule applies to, and unions.  A filler outside any PWell:block with
    // no nSD:block or SalBlock is nothing; a filler crossing a block's edge, enclosed by
    // 0.25 on both layers, is clean for AFil.j (AFil.i's business); a filler inside a
    // block with one edge on the block's edge, enclosed by 0.25, is clean for AFil.j;
    // nSD:block from two boxes whose union encloses by 0.25 is clean, by 0.245 fires; a
    // block covering the filler's right half only, with both layers enclosing the whole
    // filler by 0.25, is clean.
    let mut e = vec![fil(l, 2.0, 2.0)]; // nothing
    e.extend([
        rect(l.pwb, 10.5, 0.0, 15.0, 5.0),
        fil(l, 10.0, 2.0), // crossing: AFil.i only
        rect(l.nsdb, 9.75, 1.75, 11.25, 3.25),
        rect(l.sal, 9.75, 1.75, 11.25, 3.25),
        rect(l.pwb, 18.0, 0.0, 23.0, 5.0),
        fil(l, 18.0, 2.0), // edge on the block's edge: AFil.i only
        rect(l.nsdb, 17.75, 1.75, 19.25, 3.25),
        rect(l.sal, 17.75, 1.75, 19.25, 3.25),
        rect(l.pwb, 24.0, 0.0, 29.0, 5.0),
        fil(l, 26.0, 2.0),
        rect(l.nsdb, 25.75, 1.75, 26.5, 3.25),
        rect(l.nsdb, 26.4, 1.75, 27.25, 3.25), // union 0.25: clean
        rect(l.sal, 25.75, 1.75, 27.25, 3.25),
        rect(l.pwb, 32.0, 0.0, 37.0, 5.0),
        fil(l, 34.0, 2.0),
        rect(l.nsdb, 33.755, 1.75, 34.5, 3.25),
        rect(l.nsdb, 34.4, 1.75, 35.25, 3.25), // union 0.245: AFil.j
        rect(l.sal, 33.75, 1.75, 35.25, 3.25),
        rect(l.pwb, 42.5, 0.0, 47.0, 5.0),
        fil(l, 42.0, 2.0), // half in the block, enclosed 0.25: AFil.i only
        rect(l.nsdb, 41.75, 1.75, 43.25, 3.25),
        rect(l.sal, 41.75, 1.75, 43.25, 3.25),
    ]);
    write("AFil.j.h2", e);

    // h3 — 45°.  A 2 × 2 filler with a 0.4 chamfered corner under nSD:block and SalBlock
    // chamfered parallel to it: the nSD:block chamfer 0.244 away fires, the SalBlock's at
    // 0.251 is clean; an nSD:block chamfer passing 0.20 from a square filler's corner
    // while both walls are 0.30 away is clean under the projection reading (settled).
    write(
        "AFil.j.h3",
        vec![
            rect(l.pwb, 0.0, 0.0, 6.0, 6.0),
            chamfered_tr(f, 2.0, 2.0, 4.0, 4.0, 7.6),
            chamfered_tr(l.nsdb, 1.7, 1.7, 4.3, 4.3, 7.945), // AFil.j (0.2439)
            chamfered_tr(l.sal, 1.7, 1.7, 4.3, 4.3, 7.955),  // clean (0.2510)
            rect(l.pwb, 8.0, 0.0, 14.0, 6.0),
            rect(f, 10.0, 2.0, 12.0, 4.0),
            chamfered_tr(l.nsdb, 9.7, 1.7, 12.3, 4.3, 16.285), // clean (projection)
            rect(l.sal, 9.7, 1.7, 12.3, 4.3),
        ],
    );

    // h4 — tile lines: fillers with a 0.245 nSD:block margin on the left whose left edge
    // is well inside a tile, on x = 20 (the margin ends on the line), straddled by the
    // margin, starting 0.245 right of 20, straddling 21, on 40, straddling 42; one whose
    // margin runs along y across 20/21.  Eight violations.
    let mut e = vec![];
    for (i, x) in tile_xs(0.245).iter().enumerate() {
        e.extend(cell(x + 0.245, 2.0 + 6.0 * i as f64, 0.245, 0.25));
    }
    e.extend([
        rect(l.pwb, 13.0, 42.0, 27.0, 47.0),
        rect(f, 15.0, 44.0, 25.0, 45.0),
        rect(l.nsdb, 14.7, 43.755, 25.3, 45.3), // AFil.j
        rect(l.sal, 14.7, 43.7, 25.3, 45.3),
    ]);
    write("AFil.j.h4", e);

    // h7 — a 300 × 4 filler inside a block with a 0.245 SalBlock margin below (one
    // violation), and a cell at (1000, 1000).
    let mut e = vec![
        rect(l.pwb, 0.0, 0.0, 304.0, 8.0),
        rect(f, 2.0, 2.0, 302.0, 6.0),
        rect(l.nsdb, 1.7, 1.7, 302.3, 6.3),
        rect(l.sal, 1.7, 1.755, 302.3, 6.3),
    ];
    e.extend(cell(1000.0, 1000.0, 0.245, 0.25));
    write("AFil.j.h7", e);
}

// --- AFil.g-g3: Activ density ---

fn afil_g_h(l: &L) {
    // h1 — the layers count once.  Ten 30 µm stripes at a 100 µm pitch drawn on Activ,
    // Activ:filler and Activ.mask alike: 30 % of the 1000 µm chip, under AFil.g's 35 %
    // (fires); every 800 × 800 window holds eight stripes (30 %, above AFil.g2's 25 %).
    // Adding the three layers instead of taking their union would read 90 % and fire
    // AFil.g1 and AFil.g3 instead.
    let mut e = density_pattern(l.boundary, 1000.0, &[]);
    for k in 0..10 {
        let y = 100.0 * k as f64;
        for layer in [l.activ, l.afil, l.amask] {
            e.push(rect(layer, 0.0, y, 1000.0, y + 30.0));
        }
    }
    write("AFil.g.h1", e);

    // h2 — "any 800 × 800 µm² chip area".  Activ over the whole 1000 µm chip except a
    // 700 × 700 hole at (150, 150): the global density is 51 %; the window at (100, 100)
    // holds the whole hole and 23.4 % Activ (fires AFil.g2), while the windows on the
    // 800 µm grid - (0, 0) at 34 %, the clipped ones above 80 % - are all fine.
    let mut e = density_pattern(l.boundary, 1000.0, &[]);
    e.push(keyhole(
        l.activ, 0.0, 0.0, 1000.0, 1000.0, 150.0, 150.0, 850.0, 850.0,
    ));
    write("AFil.g2.h2", e);

    // h3 — the same with a 650 × 650 hole at (175, 175) in a plate 970 tall: 54.75 %
    // globally, the worst window (200, 200)-(1000, 1000) holds 32.8 %, the fullest 39 %:
    // clean everywhere.
    let mut e = density_pattern(l.boundary, 1000.0, &[]);
    e.push(keyhole(
        l.activ, 0.0, 0.0, 1000.0, 970.0, 175.0, 175.0, 825.0, 825.0,
    ));
    write("AFil.g2.h3", e);
}
