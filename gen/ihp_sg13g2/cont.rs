// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{cont_at, layer, library, poly, rect, space_pattern, um, write_gz};
use gds21::{GdsArrayRef, GdsDateTime, GdsElement, GdsLibrary, GdsPoint, GdsStruct};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/cont";

/// A 0.16 µm contact (exact Cont width) with lower-left corner at `(x, y)`.
fn cont16(l: (i16, i16), x: f64, y: f64) -> GdsElement {
    rect(l, x, y, x + 0.16, y + 0.16)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    cnt_a(pdk);
    cnt_b(pdk);
    cnt_e(pdk);
    cnt_f(pdk);
    cnt_g(pdk);
    cnt_g1(pdk);
    cnt_g2(pdk);
    cnt_h(pdk);
    cnt_j(pdk);
    hardening(pdk);
}

/// Cnt.e — min. space of a gate contact (Cont on GatPoly) to Activ (0.14 µm).  Both
/// contacts sit on a shared GatPoly; one Activ is 0.14 µm away (clean), the other
/// 0.13 µm (violation).
fn cnt_e(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(gp, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(activ, 0.80, 0.0, 1.2, 1.0), // 0.80 - 0.66 = 0.14 → clean
        cont16(cont, 1.5, 0.42),
        rect(activ, 1.79, 0.0, 2.2, 1.0), // 1.79 - 1.66 = 0.13 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.e.gds.gz"), library("TOP", elems));
}

/// Cnt.f — min. space of a diffusion contact (Cont on Activ) to GatPoly (0.11 µm).
fn cnt_f(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(gp, 0.77, 0.0, 1.2, 1.0), // 0.77 - 0.66 = 0.11 → clean
        cont16(cont, 1.5, 0.42),
        rect(gp, 1.76, 0.0, 2.2, 1.0), // 1.76 - 1.66 = 0.10 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.f.gds.gz"), library("TOP", elems));
}

/// Cnt.g — Cont must be within Activ or GatPoly.  One contact on Activ, one on
/// GatPoly (both clean); one over neither → coverage violation.
fn cnt_g(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(activ, 0.0, 0.0, 1.0, 1.0),
        cont16(cont, 0.4, 0.4), // inside Activ
        rect(gp, 2.0, 0.0, 3.0, 1.0),
        cont16(cont, 2.4, 0.4), // inside GatPoly
        cont16(cont, 5.0, 0.4), // outside both → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g.gds.gz"), library("TOP", elems));
}

/// Cnt.g1 — min. pSD space to a contact on nSD-Activ (0.09 µm).  Contacts sit on an
/// n+ active (Activ ∩ nSD); a pSD region is 0.09 µm away (clean) / 0.08 µm (violation).
fn cnt_g1(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let activ = layer(pdk, "Activ");
    let nsd = layer(pdk, "nSD");
    let psd = layer(pdk, "pSD");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        rect(nsd, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(psd, 0.75, 0.0, 1.2, 1.0), // 0.75 - 0.66 = 0.09 → clean
        cont16(cont, 1.5, 0.42),
        rect(psd, 1.74, 0.0, 2.2, 1.0), // 1.74 - 1.66 = 0.08 → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g1.gds.gz"), library("TOP", elems));
}

/// Cnt.g2 — min. pSD overlap (enclosure) of a contact on pSD-Activ (0.09 µm).  Both
/// contacts sit on a p+ active (Activ ∩ pSD); one pSD encloses by 0.09 (clean), the
/// other by 0.08 (violation).
fn cnt_g2(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let elems = vec![
        rect(activ, 0.0, 0.0, 3.0, 1.0),
        cont16(cont, 0.5, 0.42),
        rect(psd, 0.41, 0.33, 0.75, 0.67), // 0.09 margin on the contact at 0.5..0.66 / 0.42..0.58
        cont16(cont, 1.5, 0.42),
        rect(psd, 1.42, 0.34, 1.74, 0.66), // 0.08 margin → violation
    ];
    write_gz(&format!("{DIR}/Cnt.g2.gds.gz"), library("TOP", elems));
}

/// Cnt.h — Cont must be covered with Metal1.  One contact inside Metal1 (clean), one
/// uncovered → coverage violation.
fn cnt_h(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let m1 = layer(pdk, "Metal1");
    let elems = vec![
        rect(m1, 0.0, 0.0, 1.0, 1.0),
        cont16(cont, 0.4, 0.4), // covered
        cont16(cont, 3.0, 0.4), // uncovered → violation
    ];
    write_gz(&format!("{DIR}/Cnt.h.gds.gz"), library("TOP", elems));
}

/// Cnt.j — a contact on GatPoly that is also over Activ is not allowed.  GatPoly and
/// Activ overlap; one contact sits in the overlap (violation), one on GatPoly only
/// (clean).
fn cnt_j(pdk: &PdkConfig) {
    let cont = layer(pdk, "Cont");
    let gp = layer(pdk, "GatPoly");
    let activ = layer(pdk, "Activ");
    let elems = vec![
        rect(gp, 0.0, 0.0, 2.0, 1.0),
        rect(activ, 0.8, 0.0, 2.0, 1.0), // overlaps GatPoly for x ∈ [0.8, 2.0]
        cont16(cont, 0.3, 0.42),         // on GatPoly, not over Activ → clean
        cont16(cont, 1.2, 0.42),         // on GatPoly and over Activ → violation
    ];
    write_gz(&format!("{DIR}/Cnt.j.gds.gz"), library("TOP", elems));
}

/// `Cnt.a` runs `exact_width` on `ContSquare` (square contacts only; bars are checked
/// by the CntBar width rules).  Square contacts must be exactly 0.16 µm: a 0.155 and a
/// 0.165 µm square each fail on all four walls (8 total).  A seal-covered off-size
/// square confirms `ContSquare` (built on `ContNoSealring`) drops the seal ring.
fn cnt_a(pdk: &PdkConfig) {
    let l = layer(pdk, "Cont");
    let edgeseal = layer(pdk, "EdgeSeal");
    let o = OFFSET;
    let sq = |x: f64, side: f64| rect(l, x, o, x + side, o + side);
    let elems = vec![
        sq(o, 0.16),        // exact → clean
        sq(o + 1.0, 0.155), // too small → 4 walls
        sq(o + 2.0, 0.165), // too large → 4 walls
        // seal-covered off-size square: removed by ContNoSealring → not in ContSquare
        sq(o + 10.0, 0.155),
        rect(edgeseal, o + 9.0, o - 1.0, o + 12.0, o + 2.0),
    ];
    write_gz(&format!("{DIR}/Cnt.a.gds.gz"), library("TOP", elems));
}

fn cnt_b(pdk: &PdkConfig) {
    let l = layer(pdk, "Cont");
    let elems = space_pattern(l, l, 0.16, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Cnt.b.gds.gz"), library("TOP", elems));
}

// ---------------------------------------------------------------------------------------
// Hardening patterns (hardening/SPEC.md): layouts drawn from the manual's section 5.14
// and 8.1.2 alone, one fixture per theme, `Cnt.<rule>.h<n>`.  Each function's comment
// states the geometry and what the manual says about it; the expected answers are in
// the `cont` table of tests/ihp-sg13g2.rs and the reasoning in
// hardening/reports/ihp-sg13g2/cont.md.
// ---------------------------------------------------------------------------------------

/// Layers the hardening patterns draw on.
struct L {
    cont: (i16, i16),
    activ: (i16, i16),
    gp: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    nsd_block: (i16, i16),
    m1: (i16, i16),
    digi: (i16, i16),
    nw: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            cont: layer(pdk, "Cont"),
            activ: layer(pdk, "Activ"),
            gp: layer(pdk, "GatPoly"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            nsd_block: layer(pdk, "nSD.block"),
            m1: layer(pdk, "Metal1"),
            digi: layer(pdk, "DigiBnd"),
            nw: layer(pdk, "NWell"),
        }
    }

    /// A 0.16 Cont with its lower-left corner at `(x, y)`.
    fn sq(&self, x: f64, y: f64) -> GdsElement {
        cont16(self.cont, x, y)
    }

    /// A 0.16 Cont centred on `(cx, cy)`.
    fn c(&self, cx: f64, cy: f64) -> GdsElement {
        cont_at(self.cont, cx, cy)
    }

    /// A Cont centred on `(cx, cy)` in a box of `layer` with margins `l, r, b, t`.
    #[allow(clippy::too_many_arguments)]
    fn c_in(
        &self,
        layer: (i16, i16),
        cx: f64,
        cy: f64,
        l: f64,
        r: f64,
        b: f64,
        t: f64,
    ) -> Vec<GdsElement> {
        vec![
            rect(
                layer,
                cx - 0.08 - l,
                cy - 0.08 - b,
                cx + 0.08 + r,
                cy + 0.08 + t,
            ),
            self.c(cx, cy),
        ]
    }

    /// A Cont centred on `(cx, cy)` with the same margin `m` of `layer` on every side.
    fn c_in_m(&self, layer: (i16, i16), cx: f64, cy: f64, m: f64) -> Vec<GdsElement> {
        self.c_in(layer, cx, cy, m, m, m, m)
    }

    /// `cols × rows` Conts, lower-left at `(x, y)`, with gaps `gx` and `gy` between them.
    fn grid(&self, x: f64, y: f64, cols: usize, rows: usize, gx: f64, gy: f64) -> Vec<GdsElement> {
        let mut out = vec![];
        for r in 0..rows {
            for c in 0..cols {
                out.push(self.sq(x + c as f64 * (0.16 + gx), y + r as f64 * (0.16 + gy)));
            }
        }
        out
    }

    /// Plates of `layers` covering `(x0, y0)-(x1, y1)`: the Activ and Metal1 under a
    /// pure-Cont pattern that keep Cnt.c, Cnt.g and Cnt.h quiet.
    fn plates(&self, layers: &[(i16, i16)], x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        layers.iter().map(|&l| rect(l, x0, y0, x1, y1)).collect()
    }

    /// Box `(x0, y0)-(x1, y1)` of `layer` whose top-right corner is cut along `x + y = k`.
    fn chamfer_tr(
        &self,
        layer: (i16, i16),
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        k: f64,
    ) -> GdsElement {
        poly(
            layer,
            &[(x0, y0), (x1, y0), (x1, k - x1), (k - y1, y1), (x0, y1)],
        )
    }

    /// Box `(x0, y0)-(x1, y1)` of `layer` whose bottom-left corner is cut along `x + y = k`.
    fn chamfer_bl(
        &self,
        layer: (i16, i16),
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        k: f64,
    ) -> GdsElement {
        poly(
            layer,
            &[(k - y0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, k - x0)],
        )
    }

    /// Frame of `layer` 1 µm wide whose hole is `(x0, y0)-(x1, y1)`, one keyhole polygon.
    fn frame(&self, layer: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
        poly(
            layer,
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
}

/// Translate every boundary in `elems` by `(dx, dy)` µm.
fn shift(elems: &[GdsElement], dx: f64, dy: f64) -> Vec<GdsElement> {
    elems
        .iter()
        .map(|e| match e {
            GdsElement::GdsBoundary(b) => {
                let mut b = b.clone();
                for p in &mut b.xy {
                    p.x += um(dx);
                    p.y += um(dy);
                }
                GdsElement::GdsBoundary(b)
            }
            other => other.clone(),
        })
        .collect()
}

/// `cols × rows` copies of `cell` at `pitch`, every copy drawn in TOP.
fn flat_array(cell: &[GdsElement], cols: usize, rows: usize, pitch: f64) -> Vec<GdsElement> {
    let mut out = vec![];
    for r in 0..rows {
        for c in 0..cols {
            out.extend(shift(cell, c as f64 * pitch, r as f64 * pitch));
        }
    }
    out
}

/// The same array as one `GdsArrayRef` of a `CELL` struct placed in TOP; `extra` is
/// drawn flat in TOP beside it.
fn ref_array(
    cell: Vec<GdsElement>,
    cols: i16,
    rows: i16,
    px: f64,
    py: f64,
    extra: Vec<GdsElement>,
) -> GdsLibrary {
    let aref = GdsElement::GdsArrayRef(GdsArrayRef {
        name: "CELL".into(),
        xy: [
            GdsPoint::new(0, 0),
            GdsPoint::new(um(px * cols as f64), 0),
            GdsPoint::new(0, um(py * rows as f64)),
        ],
        cols,
        rows,
        ..Default::default()
    });
    let mut top = extra;
    top.push(aref);
    let mut lib = library("TOP", top);
    let mut child = GdsStruct::new("CELL");
    child.elems = cell;
    lib.structs.insert(0, child);
    lib.set_all_dates(GdsDateTime::from(&[0i16, 1, 1, 0, 0, 0]));
    lib
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
        ref_array(cell, 10, 5, pitch, pitch, vec![]),
    );
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    cnt_a_h(&l);
    cnt_b_h(&l);
    cnt_b1_h(&l);
    cnt_c_h(&l);
    cnt_d_h(&l);
    cnt_e_h(&l);
    cnt_f_h(&l);
    cnt_g_h(&l);
    cnt_g1_h(&l);
    cnt_g2_h(&l);
    cnt_h_h(&l);
    cnt_j_h(&l);
}

// --- Cnt.g1: min. pSD space to Cont on nSD-Activ 0.09 ---

/// A Cont on N+Activ with drawn nSD: a 0.30 Activ square under a 0.30 nSD square.
fn ncont(l: &L, cx: f64, cy: f64) -> Vec<GdsElement> {
    let mut e = l.c_in_m(l.activ, cx, cy, 0.07);
    e.push(rect(l.nsd, cx - 0.15, cy - 0.15, cx + 0.15, cy + 0.15));
    e
}

fn cnt_g1_h(l: &L) {
    let (activ, psd, m1) = (l.activ, l.psd, l.m1);

    // h1 — the bound, both metrics, 45°, touch, and what nSD-Activ is.  pSD 0.09 right of
    // a Cont on nSD-Activ is clean, 0.085 fires; a pSD corner 0.065/0.065 from the Cont
    // corner (0.092) is clean, 0.06/0.06 (0.085) fires, 0.05/0.08 (0.094) is clean though
    // 0.08 in the axis; a pSD whose corner is cut along a 45° line 0.085 from the Cont's
    // corner fires, 0.092 is clean; pSD abutting the Cont is 0 away (fires).  Section 4.2:
    // nSD = NOT (pSD OR nSD:block) OR nSD:drawing, so a Cont on plain Activ with no nSD
    // drawn is on nSD-Activ and pSD 0.085 away fires; a Cont on Activ under nSD:block is
    // not, and is clean; a Cont on N+Activ inside an NWell (a well tie) is (fires).
    let mut e = vec![];
    e.extend(ncont(l, 2.0, 2.0));
    e.push(rect(psd, 2.17, 1.5, 2.67, 2.5));
    e.extend(ncont(l, 4.0, 2.0));
    e.push(rect(psd, 4.165, 1.5, 4.665, 2.5)); // 0.085
    e.extend(ncont(l, 6.0, 2.0));
    e.push(rect(psd, 6.145, 2.145, 6.645, 2.645)); // 0.092
    e.extend(ncont(l, 8.0, 2.0));
    e.push(rect(psd, 8.14, 2.14, 8.64, 2.64)); // 0.085
    e.extend(ncont(l, 10.0, 2.0));
    e.push(rect(psd, 10.13, 2.16, 10.63, 2.66)); // 0.094
    e.extend(ncont(l, 12.0, 2.0));
    e.push(l.chamfer_bl(psd, 12.05, 2.05, 13.0, 3.0, 14.28)); // (14.28 − 14.16)/√2 = 0.085
    e.extend(ncont(l, 14.0, 2.0));
    e.push(l.chamfer_bl(psd, 14.05, 2.05, 15.0, 3.0, 16.29)); // 0.092
    e.extend(ncont(l, 16.0, 2.0));
    e.push(rect(psd, 16.08, 1.5, 16.58, 2.5)); // touch
    e.extend(diff_cont(l, 18.0, 2.0));
    e.push(rect(psd, 18.165, 1.5, 18.665, 2.5)); // plain Activ: nSD-Activ, 0.085
    e.extend(diff_cont(l, 22.0, 2.0));
    e.push(rect(l.nsd_block, 21.8, 1.8, 22.2, 2.2));
    e.push(rect(psd, 22.165, 1.5, 22.665, 2.5)); // nSD:block: not nSD-Activ, clean
    e.push(rect(l.nw, 25.0, 1.0, 27.0, 3.0));
    e.extend(ncont(l, 26.0, 2.0));
    e.push(rect(psd, 26.165, 1.5, 26.665, 2.5)); // well tie, 0.085
    e.extend(l.plates(&[m1], 1.0, 1.0, 28.0, 4.0));
    write("Cnt.g1.h1", e);

    // h2 — tile lines, as Cnt.e.h2 with 0.085 (nine fire) and 0.09 (clean).
    let gap = |cx: f64, cy: f64, g: f64| {
        let mut e = ncont(l, cx, cy);
        e.push(rect(psd, cx + 0.08 + g, cy - 0.5, cx + 0.58 + g, cy + 0.5));
        e
    };
    let mut e = vec![];
    e.extend(gap(19.9, 2.0, 0.085));
    e.extend(gap(19.835, 3.5, 0.085));
    e.extend(gap(19.92, 5.0, 0.085));
    e.extend(gap(20.0, 6.5, 0.085));
    e.extend(gap(19.9, 8.0, 0.09));
    for (x, y) in [(20.9, 9.5), (39.9, 2.0), (41.9, 3.5), (13.9, 2.0)] {
        e.extend(gap(x, y, 0.085));
    }
    e.extend(ncont(l, 10.0, 19.9));
    e.push(rect(psd, 9.5, 20.065, 10.5, 20.565));
    e.extend(l.plates(&[m1], 1.0, 1.0, 45.0, 11.0));
    e.extend(l.plates(&[m1], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.g1.h2", e);

    // h3/h4 — fifty Conts on nSD-Activ with pSD 0.085 to the right, flat and as an array.
    let mut cell = ncont(l, 0.35, 0.5);
    cell.push(rect(psd, 0.515, 0.2, 0.9, 0.8));
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.g1", 3, cell, 1.0);

    // h5 — large and far.  A 300 µm pSD strip 0.085 from a Cont on nSD-Activ (once); one
    // at (1000, 1000); a ContBar on nSD-Activ 0.085 from pSD is CntB.g1's.
    let mut e = ncont(l, 2.0, 2.0);
    e.push(rect(psd, 2.165, 1.5, 302.165, 2.5));
    e.extend(ncont(l, 1000.0, 1000.0));
    e.push(rect(psd, 1000.165, 999.5, 1000.665, 1000.5));
    e.push(rect(activ, 1.85, 5.68, 2.15, 6.32));
    e.push(rect(l.nsd, 1.85, 5.68, 2.15, 6.32));
    e.push(rect(l.cont, 1.92, 5.75, 2.08, 6.25));
    e.push(rect(psd, 2.165, 5.5, 3.0, 6.5));
    e.extend(l.plates(&[m1], 1.0, 1.0, 303.0, 7.0));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.g1.h5", e);
}

// --- Cnt.g2: min. pSD overlap of Cont on pSD-Activ 0.09 ---

fn cnt_g2_h(l: &L) {
    let (activ, psd, m1) = (l.activ, l.psd, l.m1);

    // A Cont on a 0.50 Activ square (0.17 margins) with pSD margins `pl, pr, pb, pt`.
    let pc = |cx: f64, cy: f64, pl: f64, pr: f64, pb: f64, pt: f64| {
        let mut e = l.c_in_m(activ, cx, cy, 0.17);
        e.push(rect(
            psd,
            cx - 0.08 - pl,
            cy - 0.08 - pb,
            cx + 0.08 + pr,
            cy + 0.08 + pt,
        ));
        e
    };

    // h1 — the bound.  pSD margins of 0.09 are clean; 0.085 on the right fires once,
    // 0.085 on all four sides fires (four walls, one Cont), 0.085 right and top (a
    // corner) fires; a 0.005 margin and a pSD edge on the Cont edge (0) fire; a pSD
    // ending on the Activ edge 0.07 from the Cont fires (the usual mistake: pSD must
    // reach past Activ); the pSD edge through the Cont's middle fires Cnt.g2 - and, the
    // uncovered half being on nSD-Activ (section 4.2), Cnt.g1 at 0.
    let mut e = vec![];
    e.extend(pc(2.0, 2.0, 0.09, 0.09, 0.09, 0.09));
    e.extend(pc(4.0, 2.0, 0.09, 0.085, 0.09, 0.09));
    e.extend(pc(6.0, 2.0, 0.085, 0.085, 0.085, 0.085));
    e.extend(pc(8.0, 2.0, 0.09, 0.085, 0.09, 0.085));
    e.extend(pc(10.0, 2.0, 0.005, 0.09, 0.09, 0.09));
    e.extend(pc(12.0, 2.0, 0.09, 0.0, 0.09, 0.09));
    e.extend(l.c_in_m(activ, 14.0, 2.0, 0.07));
    e.push(rect(psd, 13.75, 1.75, 14.15, 2.25)); // ends on the Activ edge: 0.07
    e.extend(pc(16.0, 2.0, 0.09, -0.08, 0.09, 0.09));
    e.extend(l.plates(&[m1], 1.0, 1.0, 18.0, 3.0));
    write("Cnt.g2.h1", e);

    // h2 — 45° and merging.  A pSD corner cut along a 45° line 0.085 from the Cont's
    // corner, both axis margins 0.20: clean by the settled projection reading (0.092
    // too); pSD drawn as two abutting halves with the seam through the Cont (clean at
    // 0.09), as two overlapping boxes (clean), as a union with 0.085 on the right (fires
    // once), drawn twice with 0.085 (once).
    let mut e = vec![];
    e.extend(l.c_in_m(activ, 2.0, 2.0, 0.17));
    e.push(l.chamfer_tr(psd, 1.72, 1.72, 2.28, 2.28, 4.28)); // (4.28 − 4.16)/√2 = 0.085
    e.extend(l.c_in_m(activ, 4.0, 2.0, 0.17));
    e.push(l.chamfer_tr(psd, 3.72, 1.72, 4.28, 2.28, 6.29)); // 0.092
    e.extend(l.c_in_m(activ, 6.0, 2.0, 0.17));
    e.push(rect(psd, 5.83, 1.83, 6.0, 2.17));
    e.push(rect(psd, 6.0, 1.83, 6.17, 2.17));
    e.extend(l.c_in_m(activ, 8.0, 2.0, 0.17));
    e.push(rect(psd, 7.83, 1.83, 8.05, 2.17));
    e.push(rect(psd, 7.95, 1.83, 8.17, 2.17));
    e.extend(l.c_in_m(activ, 10.0, 2.0, 0.17));
    e.push(rect(psd, 9.83, 1.83, 10.0, 2.17));
    e.push(rect(psd, 10.0, 1.83, 10.165, 2.17)); // union: 0.085 right
    e.extend(l.c_in_m(activ, 12.0, 2.0, 0.17));
    e.push(rect(psd, 11.83, 1.83, 12.165, 2.17));
    e.push(rect(psd, 11.83, 1.83, 12.165, 2.17)); // twice: 0.085 right, once
    e.extend(l.plates(&[m1], 1.0, 1.0, 14.0, 3.0));
    write("Cnt.g2.h2", e);

    // h3 — tile lines.  At x = 20: a Cont straddling the line with a 0.085 right pSD
    // margin, a pSD edge on the line 0.085 from the Cont, a 0.085 margin straddling it, a
    // 0.09 margin straddling it (clean); 0.085 straddling x = 21, 40, 42, 14 and y = 20.
    // Eight fire.
    let mut e = vec![];
    e.extend(pc(20.0, 2.0, 0.30, 0.085, 0.09, 0.09));
    e.extend(pc(19.835, 3.0, 0.30, 0.085, 0.09, 0.09));
    e.extend(pc(19.875, 4.0, 0.30, 0.085, 0.09, 0.09));
    e.extend(pc(19.875, 5.0, 0.30, 0.09, 0.09, 0.09));
    for (x, y) in [(21.0, 6.0), (40.0, 2.0), (42.0, 3.0), (14.0, 2.0)] {
        e.extend(pc(x, y, 0.30, 0.085, 0.09, 0.09));
    }
    e.extend(pc(10.0, 20.0, 0.09, 0.09, 0.30, 0.085));
    e.extend(l.plates(&[m1], 1.0, 1.0, 45.0, 7.0));
    e.extend(l.plates(&[m1], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.g2.h3", e);

    // h4/h5 — fifty Conts with a 0.085 right pSD margin, flat and as an array reference.
    let mut cell = pc(0.5, 0.5, 0.09, 0.085, 0.09, 0.09);
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.g2", 4, cell, 1.0);

    // h6 — large and far.  A 300 µm pSD strip 0.33 wide over a 0.50 Activ strip, three
    // Conts with 0.085 top and bottom (three Conts, one marker each side); a Cont at
    // (1000, 1000) with 0.085 on the right.
    let mut e = vec![
        rect(activ, 2.0, 2.0, 302.0, 2.5),
        rect(psd, 2.0, 2.085, 302.0, 2.415),
    ];
    for x in [5.0, 150.0, 300.0] {
        e.push(l.c(x, 2.25));
    }
    e.extend(pc(1000.0, 1000.0, 0.09, 0.085, 0.09, 0.09));
    e.extend(l.plates(&[m1], 1.0, 1.0, 303.0, 3.0));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.g2.h6", e);
}

// --- Cnt.h: Cont must be covered with Metal1 ---

fn cnt_h_h(l: &L) {
    let (activ, m1) = (l.activ, l.m1);

    // h1 — what "covered" means.  Metal1 with margins (clean) and exactly coincident
    // with the Cont (clean); no Metal1 (fires); a 0.005 sliver uncovered (fires); half
    // covered (fires); Metal1 as two abutting boxes with the seam through the Cont
    // (clean) and as two overlapping boxes (clean); the Cont in the hole of a Metal1
    // ring (fires); Metal1 abutting the Cont from outside (fires); a bare Cont at
    // (1000, 1000) (fires).
    let mut e = vec![];
    e.extend(l.c_in_m(m1, 2.0, 2.0, 0.10));
    e.extend(l.c_in_m(m1, 4.0, 2.0, 0.0));
    e.push(l.c(6.0, 2.0));
    e.extend(l.c_in(m1, 8.0, 2.0, 0.10, -0.005, 0.10, 0.10));
    e.extend(l.c_in(m1, 10.0, 2.0, 0.10, -0.08, 0.10, 0.10));
    e.push(l.c(12.0, 2.0));
    e.push(rect(m1, 11.8, 1.8, 12.0, 2.2));
    e.push(rect(m1, 12.0, 1.8, 12.2, 2.2));
    e.push(l.c(14.0, 2.0));
    e.push(rect(m1, 13.8, 1.8, 14.05, 2.2));
    e.push(rect(m1, 13.95, 1.8, 14.2, 2.2));
    e.push(l.frame(m1, 15.5, 1.5, 16.5, 2.5));
    e.push(l.c(16.0, 2.0));
    e.push(l.c(18.0, 2.0));
    e.push(rect(m1, 17.5, 1.5, 17.92, 2.5));
    e.push(l.c(1000.0, 1000.0));
    e.extend(l.plates(&[activ], 1.0, 1.0, 20.0, 3.0));
    e.extend(l.plates(&[activ], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.h.h1", e);

    // h2 — tile lines.  Bare Conts straddling x = 20, 21, 40, 42, 14 and y = 20 (six); a
    // Cont straddling x = 20 whose Metal1 ends on the line (half covered, fires); a Cont
    // ending on x = 20 whose Metal1 ends there too (covered); a Cont straddling x = 20
    // under Metal1 drawn as two boxes meeting on the line (covered).
    let mut e = vec![];
    for (x, y) in [
        (20.0, 2.0),
        (21.0, 3.5),
        (40.0, 2.0),
        (42.0, 3.5),
        (14.0, 2.0),
        (10.0, 20.0),
    ] {
        e.push(l.c(x, y));
    }
    e.push(rect(m1, 19.5, 4.5, 20.0, 5.5));
    e.push(l.c(20.0, 5.0));
    e.push(rect(m1, 19.5, 6.0, 20.0, 7.0));
    e.push(l.c(19.92, 6.5));
    e.push(rect(m1, 19.5, 7.5, 20.0, 8.5));
    e.push(rect(m1, 20.0, 7.5, 20.5, 8.5));
    e.push(l.c(20.0, 8.0));
    e.extend(l.plates(&[activ], 1.0, 1.0, 45.0, 9.0));
    e.extend(l.plates(&[activ], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.h.h2", e);

    // h3/h4 — fifty bare Conts, flat and as an array reference.
    let mut cell = vec![l.c(0.5, 0.5)];
    cell.extend(l.plates(&[activ], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.h", 3, cell, 1.0);
}

// --- Cnt.j: Cont on GatPoly over Activ is not allowed ---

fn cnt_j_h(l: &L) {
    let (activ, gp, m1) = (l.activ, l.gp, l.m1);

    // h1 — how little overlap counts.  A Cont in a gate (poly over Activ) fires; a Cont on
    // GatPoly whose right 0.005 lies over Activ fires; one whose corner overlaps Activ by
    // 0.005 × 0.005 fires; a Cont on GatPoly abutting Activ does not overlap (Cnt.e at 0,
    // no Cnt.j); a Cont on poly over two Activ fingers 0.05 wide is one Cont over Activ
    // (once); a Cont on the gate of an Activ drawn as two abutting boxes fires once; a
    // gate contact at (1000, 1000) fires.
    let mut e = vec![rect(activ, 1.5, 1.5, 2.5, 2.5)];
    e.push(rect(gp, 1.7, 1.0, 2.3, 3.0));
    e.push(l.c(2.0, 2.0));
    e.push(rect(gp, 3.5, 1.7, 4.5, 2.3));
    e.push(rect(activ, 4.075, 1.0, 4.5, 3.0));
    e.push(l.c(4.0, 2.0));
    e.push(rect(gp, 5.5, 1.7, 6.5, 2.3));
    e.push(rect(activ, 6.075, 2.075, 6.5, 3.0));
    e.push(l.c(6.0, 2.0));
    e.push(rect(gp, 7.5, 1.7, 8.5, 2.3));
    e.push(rect(activ, 8.08, 1.0, 8.5, 3.0));
    e.push(l.c(8.0, 2.0));
    e.push(rect(gp, 9.5, 1.7, 10.5, 2.3));
    e.push(rect(activ, 9.95, 1.0, 10.0, 3.0));
    e.push(rect(activ, 10.03, 1.0, 10.08, 3.0));
    e.push(l.c(10.0, 2.0));
    e.push(rect(activ, 11.5, 1.5, 12.0, 2.5));
    e.push(rect(activ, 12.0, 1.5, 12.5, 2.5));
    e.push(rect(gp, 11.7, 1.0, 12.3, 3.0));
    e.push(l.c(12.0, 2.0));
    e.push(rect(activ, 999.5, 999.5, 1000.5, 1000.5));
    e.push(rect(gp, 999.7, 999.0, 1000.3, 1001.0));
    e.push(l.c(1000.0, 1000.0));
    e.extend(l.plates(&[m1], 1.0, 0.5, 14.0, 3.5));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.j.h1", e);

    // h2 — tile lines.  Gate contacts straddling x = 20, 21, 40, 42, 14 and y = 20; a Cont
    // on poly whose Activ begins exactly on x = 20 under the Cont's right half (fires);
    // one whose Activ begins on x = 20 where the Cont ends (abutting: no Cnt.j).
    let gate = |cx: f64, cy: f64| {
        vec![
            rect(activ, cx - 0.5, cy - 0.5, cx + 0.5, cy + 0.5),
            rect(gp, cx - 0.3, cy - 1.0, cx + 0.3, cy + 1.0),
            l.c(cx, cy),
        ]
    };
    let mut e = vec![];
    for (x, y) in [
        (20.0, 2.0),
        (21.0, 5.0),
        (40.0, 2.0),
        (42.0, 5.0),
        (14.0, 2.0),
        (10.0, 20.0),
    ] {
        e.extend(gate(x, y));
    }
    e.push(rect(gp, 19.5, 7.7, 20.5, 8.3));
    e.push(rect(activ, 20.0, 7.0, 20.5, 9.0));
    e.push(l.c(20.0, 8.0));
    e.push(rect(gp, 19.5, 10.7, 20.5, 11.3));
    e.push(rect(activ, 20.0, 10.0, 20.5, 12.0));
    e.push(l.c(19.92, 11.0));
    e.extend(l.plates(&[m1], 1.0, 0.5, 45.0, 12.5));
    e.extend(l.plates(&[m1], 9.0, 18.5, 11.0, 21.5));
    write("Cnt.j.h2", e);

    // h3/h4 — fifty gate contacts, flat and as an array reference.
    let mut cell = gate(1.0, 1.0);
    cell.extend(l.plates(&[m1], 0.0, 0.0, 2.0, 2.0));
    arrays("Cnt.j", 3, cell, 2.0);
}

// --- Cnt.e: min. Cont on GatPoly space to Activ 0.14 ---

/// A gate contact: a Cont centred on `(cx, cy)` on a 0.30 GatPoly square (0.07 margins).
fn gate_cont(l: &L, cx: f64, cy: f64) -> Vec<GdsElement> {
    l.c_in_m(l.gp, cx, cy, 0.07)
}

/// A diffusion contact: a Cont centred on `(cx, cy)` on a 0.30 Activ square (0.07 margins).
fn diff_cont(l: &L, cx: f64, cy: f64) -> Vec<GdsElement> {
    l.c_in_m(l.activ, cx, cy, 0.07)
}

fn cnt_e_h(l: &L) {
    let (activ, gp, m1) = (l.activ, l.gp, l.m1);

    // h1 — the bound, both metrics, 45°, touch, gate.  Activ 0.14 right of a gate contact
    // is clean, 0.135 fires; an Activ corner 0.10/0.10 from the Cont corner (0.141) is
    // clean, 0.095/0.095 (0.134) fires, 0.08/0.12 (0.144) is clean though 0.12 in the
    // axis; an Activ whose bottom-left corner is cut along a 45° line 0.134 from the
    // Cont's corner fires, 0.141 is clean; an Activ abutting the Cont's edge is 0 away
    // (fires; nothing overlaps, so no Cnt.j); a gate whose Activ edge is 0.22 from the
    // Cont is clean, 0.135 fires; a Cont with Activ 0.135 on both sides fires twice.
    let mut e = vec![];
    e.extend(gate_cont(l, 2.0, 2.0));
    e.push(rect(activ, 2.22, 1.5, 2.72, 2.5));
    e.extend(gate_cont(l, 4.0, 2.0));
    e.push(rect(activ, 4.215, 1.5, 4.715, 2.5)); // 0.135
    e.extend(gate_cont(l, 6.0, 2.0));
    e.push(rect(activ, 6.18, 2.18, 6.68, 2.68)); // 0.141
    e.extend(gate_cont(l, 8.0, 2.0));
    e.push(rect(activ, 8.175, 2.175, 8.675, 2.675)); // 0.134
    e.extend(gate_cont(l, 10.0, 2.0));
    e.push(rect(activ, 10.16, 2.20, 10.66, 2.70)); // 0.144
    e.extend(gate_cont(l, 12.0, 2.0));
    e.push(l.chamfer_bl(activ, 12.05, 2.05, 13.0, 3.0, 14.35)); // (14.35 − 14.16)/√2 = 0.134
    e.extend(gate_cont(l, 14.0, 2.0));
    e.push(l.chamfer_bl(activ, 14.05, 2.05, 15.0, 3.0, 16.36)); // 0.141
    e.extend(gate_cont(l, 16.0, 2.0));
    e.push(rect(activ, 16.08, 1.5, 16.58, 2.5)); // touch
    e.push(rect(gp, 17.85, 1.85, 19.2, 2.15));
    e.push(l.c(18.0, 2.0));
    e.push(rect(activ, 18.3, 1.0, 19.0, 3.0)); // gate, 0.22
    e.push(rect(gp, 21.85, 1.85, 23.2, 2.15));
    e.push(l.c(22.0, 2.0));
    e.push(rect(activ, 22.215, 1.0, 23.0, 3.0)); // gate, 0.135
    e.extend(gate_cont(l, 26.0, 2.0));
    e.push(rect(activ, 25.0, 1.5, 25.785, 2.5));
    e.push(rect(activ, 26.215, 1.5, 27.0, 2.5)); // both sides
    e.extend(l.plates(&[m1], 1.0, 1.0, 28.0, 4.0));
    write("Cnt.e.h1", e);

    // h2 — tile lines.  A 0.135 gap straddling x = 20; the Activ edge on x = 20 with the
    // Cont 0.135 left of it; the Cont's edge on x = 20 with the Activ 0.135 right; the
    // Cont straddling x = 20; 0.135 gaps straddling x = 21, 40, 42, 14 and y = 20; a 0.14
    // gap straddling x = 20 (clean).  Nine fire.
    let gap = |cx: f64, cy: f64, g: f64| {
        let mut e = gate_cont(l, cx, cy);
        e.push(rect(
            activ,
            cx + 0.08 + g,
            cy - 0.5,
            cx + 0.58 + g,
            cy + 0.5,
        ));
        e
    };
    let mut e = vec![];
    e.extend(gap(19.9, 2.0, 0.135));
    e.extend(gap(19.785, 3.5, 0.135));
    e.extend(gap(19.92, 5.0, 0.135));
    e.extend(gap(20.0, 6.5, 0.135));
    e.extend(gap(19.9, 8.0, 0.14));
    for (x, y) in [(20.9, 9.5), (39.9, 2.0), (41.9, 3.5), (13.9, 2.0)] {
        e.extend(gap(x, y, 0.135));
    }
    e.extend(gate_cont(l, 10.0, 19.9));
    e.push(rect(activ, 9.5, 20.115, 10.5, 20.615));
    e.extend(l.plates(&[m1], 1.0, 1.0, 45.0, 11.0));
    e.extend(l.plates(&[m1], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.e.h2", e);

    // h3/h4 — fifty gate contacts with Activ 0.135 to the right, flat and as an array.
    let mut cell = gate_cont(l, 0.35, 0.5);
    cell.push(rect(activ, 0.565, 0.2, 0.9, 0.8));
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.e", 3, cell, 1.0);

    // h5 — large and far.  A 300 µm Activ strip 0.135 from a gate contact (once); a gate
    // contact at (1000, 1000) with Activ 0.135 away; a 0.16 × 0.50 ContBar on GatPoly
    // 0.135 from Activ is CntB.e's, not Cnt.e's.
    let mut e = gate_cont(l, 2.0, 2.0);
    e.push(rect(activ, 2.215, 1.5, 302.215, 2.5));
    e.extend(gate_cont(l, 1000.0, 1000.0));
    e.push(rect(activ, 1000.215, 999.5, 1000.715, 1000.5));
    e.push(rect(gp, 1.85, 5.68, 2.15, 6.32));
    e.push(rect(l.cont, 1.92, 5.75, 2.08, 6.25));
    e.push(rect(activ, 2.215, 5.5, 3.0, 6.5));
    e.extend(l.plates(&[m1], 1.0, 1.0, 303.0, 7.0));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.e.h5", e);
}

// --- Cnt.f: min. Cont on Activ space to GatPoly 0.11 ---

fn cnt_f_h(l: &L) {
    let (activ, gp, m1) = (l.activ, l.gp, l.m1);

    // h1 — the bound, both metrics, 45°, touch, gate.  GatPoly 0.11 right of a diffusion
    // contact is clean, 0.105 fires; a poly corner 0.08/0.08 from the Cont corner (0.113)
    // is clean, 0.075/0.075 (0.106) fires, 0.07/0.09 (0.114) is clean though 0.09 in the
    // axis; a poly whose bottom-left corner is cut along a 45° line 0.106 from the Cont's
    // corner fires, 0.113 is clean; a poly abutting the Cont's edge is 0 away (fires, no
    // overlap so no Cnt.j); a gate crossing the Activ 0.105 from the Cont fires; a Cont
    // between two gates 0.105 away fires twice; a poly on the field 0.105 from the Cont
    // (0.035 outside the Activ) fires.
    let mut e = vec![];
    e.extend(diff_cont(l, 2.0, 2.0));
    e.push(rect(gp, 2.19, 1.5, 2.69, 2.5));
    e.extend(diff_cont(l, 4.0, 2.0));
    e.push(rect(gp, 4.185, 1.5, 4.685, 2.5)); // 0.105
    e.extend(diff_cont(l, 6.0, 2.0));
    e.push(rect(gp, 6.16, 2.16, 6.66, 2.66)); // 0.113
    e.extend(diff_cont(l, 8.0, 2.0));
    e.push(rect(gp, 8.155, 2.155, 8.655, 2.655)); // 0.106
    e.extend(diff_cont(l, 10.0, 2.0));
    e.push(rect(gp, 10.15, 2.17, 10.65, 2.67)); // 0.114
    e.extend(diff_cont(l, 12.0, 2.0));
    e.push(l.chamfer_bl(gp, 12.05, 2.05, 13.0, 3.0, 14.31)); // (14.31 − 14.16)/√2 = 0.106
    e.extend(diff_cont(l, 14.0, 2.0));
    e.push(l.chamfer_bl(gp, 14.05, 2.05, 15.0, 3.0, 16.32)); // 0.113
    e.extend(diff_cont(l, 16.0, 2.0));
    e.push(rect(gp, 16.08, 1.5, 16.58, 2.5)); // touch
    e.push(rect(activ, 17.85, 1.85, 19.2, 2.15));
    e.push(l.c(18.0, 2.0));
    e.push(rect(gp, 18.185, 1.0, 18.5, 3.0)); // gate, 0.105
    e.push(rect(activ, 21.5, 1.85, 23.2, 2.15));
    e.push(l.c(22.0, 2.0));
    e.push(rect(gp, 21.5, 1.0, 21.815, 3.0));
    e.push(rect(gp, 22.185, 1.0, 22.5, 3.0)); // two gates
    e.extend(diff_cont(l, 26.0, 2.0));
    e.push(rect(gp, 26.185, 1.5, 26.685, 2.5)); // field poly, 0.105
    e.extend(l.plates(&[m1], 1.0, 1.0, 28.0, 4.0));
    write("Cnt.f.h1", e);

    // h2 — tile lines, as Cnt.e.h2 with 0.105 (nine fire) and 0.11 (clean).
    let gap = |cx: f64, cy: f64, g: f64| {
        let mut e = diff_cont(l, cx, cy);
        e.push(rect(gp, cx + 0.08 + g, cy - 0.5, cx + 0.58 + g, cy + 0.5));
        e
    };
    let mut e = vec![];
    e.extend(gap(19.9, 2.0, 0.105));
    e.extend(gap(19.815, 3.5, 0.105));
    e.extend(gap(19.92, 5.0, 0.105));
    e.extend(gap(20.0, 6.5, 0.105));
    e.extend(gap(19.9, 8.0, 0.11));
    for (x, y) in [(20.9, 9.5), (39.9, 2.0), (41.9, 3.5), (13.9, 2.0)] {
        e.extend(gap(x, y, 0.105));
    }
    e.extend(diff_cont(l, 10.0, 19.9));
    e.push(rect(gp, 9.5, 20.085, 10.5, 20.585));
    e.extend(l.plates(&[m1], 1.0, 1.0, 45.0, 11.0));
    e.extend(l.plates(&[m1], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.f.h2", e);

    // h3/h4 — fifty diffusion contacts with GatPoly 0.105 to the right, flat and as an array.
    let mut cell = diff_cont(l, 0.35, 0.5);
    cell.push(rect(gp, 0.535, 0.2, 0.9, 0.8));
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.f", 3, cell, 1.0);

    // h5 — large and far.  A 300 µm GatPoly strip 0.105 from a diffusion contact (once);
    // one at (1000, 1000); a 0.16 × 0.50 ContBar on Activ 0.105 from GatPoly is CntB.f's.
    let mut e = diff_cont(l, 2.0, 2.0);
    e.push(rect(gp, 2.185, 1.5, 302.185, 2.5));
    e.extend(diff_cont(l, 1000.0, 1000.0));
    e.push(rect(gp, 1000.185, 999.5, 1000.685, 1000.5));
    e.push(rect(activ, 1.85, 5.68, 2.15, 6.32));
    e.push(rect(l.cont, 1.92, 5.75, 2.08, 6.25));
    e.push(rect(gp, 2.185, 5.5, 3.0, 6.5));
    e.extend(l.plates(&[m1], 1.0, 1.0, 303.0, 7.0));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.f.h5", e);
}

// --- Cnt.g: Cont must be within Activ or GatPoly ---

fn cnt_g_h(l: &L) {
    let (activ, gp, m1) = (l.activ, l.gp, l.m1);

    // h1 — what "within" means.  A bare Cont fires; a Cont 0.005 past the Activ edge
    // and one half outside fire (and are Cnt.c, enclosed by 0); a Cont in the hole of an
    // Activ ring fires; a Cont abutting the Activ from outside fires (nothing on Activ,
    // so no Cnt.c); Conts in Activ, in GatPoly, over two abutting Activ boxes and over
    // four Activ quadrants are within; a Cont on Activ under GatPoly is within (Cnt.j,
    // not Cnt.g); a bare Cont at (1000, 1000) fires.
    let mut e = vec![l.c(2.0, 2.0)];
    e.push(rect(activ, 3.5, 1.5, 4.075, 2.5));
    e.push(l.c(4.0, 2.0));
    e.push(rect(activ, 5.5, 1.5, 6.0, 2.5));
    e.push(l.c(6.0, 2.0));
    e.push(l.frame(activ, 7.5, 1.5, 8.5, 2.5));
    e.push(l.c(8.0, 2.0));
    e.push(rect(activ, 9.0, 1.5, 9.92, 2.5));
    e.push(l.c(10.0, 2.0));
    e.push(rect(activ, 11.5, 1.5, 12.5, 2.5));
    e.push(l.c(12.0, 2.0));
    e.push(rect(gp, 13.5, 1.5, 14.5, 2.5));
    e.push(l.c(14.0, 2.0));
    e.push(rect(activ, 15.5, 1.5, 16.0, 2.5));
    e.push(rect(activ, 16.0, 1.5, 16.5, 2.5));
    e.push(l.c(16.0, 2.0));
    e.push(rect(activ, 17.5, 1.5, 18.0, 2.0));
    e.push(rect(activ, 18.0, 1.5, 18.5, 2.0));
    e.push(rect(activ, 17.5, 2.0, 18.0, 2.5));
    e.push(rect(activ, 18.0, 2.0, 18.5, 2.5));
    e.push(l.c(18.0, 2.0));
    e.push(rect(activ, 21.5, 1.5, 22.5, 2.5));
    e.push(rect(gp, 21.7, 1.0, 22.3, 3.0));
    e.push(l.c(22.0, 2.0));
    e.push(l.c(1000.0, 1000.0));
    e.extend(l.plates(&[m1], 1.0, 1.0, 24.0, 4.0));
    e.extend(l.plates(&[m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.g.h1", e);

    // h2 — tile lines.  Bare Conts straddling x = 20, 21, 40, 42, 14 and y = 20 (six); a
    // Cont straddling x = 20 whose Activ ends on the line (half covered: Cnt.g and
    // Cnt.c); a Cont ending on x = 20 whose Activ ends there too (within).
    let mut e = vec![];
    for (x, y) in [
        (20.0, 2.0),
        (21.0, 3.5),
        (40.0, 2.0),
        (42.0, 3.5),
        (14.0, 2.0),
        (10.0, 20.0),
    ] {
        e.push(l.c(x, y));
    }
    e.push(rect(activ, 19.5, 4.5, 20.0, 5.5));
    e.push(l.c(20.0, 5.0));
    e.push(rect(activ, 19.5, 6.0, 20.0, 7.0));
    e.push(l.c(19.92, 6.5));
    e.extend(l.plates(&[m1], 1.0, 1.0, 45.0, 8.0));
    e.extend(l.plates(&[m1], 9.0, 19.0, 11.0, 21.0));
    write("Cnt.g.h2", e);

    // h3/h4 — fifty bare Conts, flat and as an array reference.
    let mut cell = vec![l.c(0.5, 0.5)];
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.g", 3, cell, 1.0);
}

// --- Cnt.c: min. Activ enclosure of Cont 0.07 (0.05 inside DigiBnd, section 8.1.2) ---

fn cnt_c_h(l: &L) {
    let (activ, m1) = (l.activ, l.m1);

    // h1 — the bound.  Margins of 0.07 all round are clean; 0.065 on the right only fires
    // once, 0.065 on all four sides four times, 0.065 right and top (a corner) twice; a
    // 0.005 margin fires; a Cont edge on the Activ edge is an enclosure of 0 (fires); a
    // Cont 0.05 past the Activ edge is Cnt.c and Cnt.g.
    let mut e = vec![];
    e.extend(l.c_in_m(activ, 2.0, 2.0, 0.07));
    e.extend(l.c_in(activ, 4.0, 2.0, 0.07, 0.065, 0.07, 0.07));
    e.extend(l.c_in_m(activ, 6.0, 2.0, 0.065));
    e.extend(l.c_in(activ, 8.0, 2.0, 0.07, 0.065, 0.07, 0.065));
    e.extend(l.c_in(activ, 10.0, 2.0, 0.005, 0.07, 0.07, 0.07));
    e.extend(l.c_in(activ, 12.0, 2.0, 0.07, 0.0, 0.07, 0.07));
    e.extend(l.c_in(activ, 14.0, 2.0, 0.07, -0.05, 0.07, 0.07));
    e.extend(l.plates(&[m1], 1.0, 1.0, 16.0, 3.0));
    write("Cnt.c.h1", e);

    // h2 — 45° geometry.  The enclosing box's top-right corner is cut along a 45° line
    // passing 0.064 from the Cont's corner: once as a short chamfer (box margins 0.20,
    // legs 0.31) and once as a long wall (margins 0.50, legs 0.91); controls at 0.071.
    // The axis-aligned margins are fine in every case, so by the settled projection
    // reading all four are clean.
    write("Cnt.c.h2", slant_set(l, activ));

    // h3 — shapes that merge.  The Cont across the seam of two abutting Activ boxes, over
    // two overlapping ones and in a 4 × 4 grid of 0.25 boxes is enclosed by the union
    // (clean); a union whose right margin is 0.065 fires once; an Activ drawn twice with
    // 0.065 fires once, not twice; a Cont on the wall of an Activ ring 0.065 from the
    // hole fires once.
    write("Cnt.c.h3", merge_set(l, activ));

    // h4 — tile lines.  At x = 20: a Cont straddling the line with a 0.065 right margin,
    // an Activ edge on the line 0.065 from the Cont, a 0.065 margin straddling the line,
    // and a 0.07 margin straddling it (clean); a straddling Cont at 0.065 at x = 21, 40,
    // 42, 14 and y = 20.  Eight fire.
    write("Cnt.c.h4", tile_set(l, activ));

    // h5/h6 — fifty Conts with a 0.065 right margin, flat and as an array reference.
    let mut cell = l.c_in(activ, 0.5, 0.5, 0.07, 0.065, 0.07, 0.07);
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.c", 5, cell, 1.0);

    // h7 — large and far.  A 300 µm Activ strip 0.30 wide with Conts at 0.07 (clean) and
    // one 0.29 wide with three Conts at 0.065 top and bottom (six markers); a Cont at
    // (1000, 1000) with 0.065 on the right.
    write("Cnt.c.h7", far_set(l, activ));

    // h8 — DigiBnd (section 8.1.2: 0.05 inside DigiBnd).  Under a DigiBnd plate: 0.05
    // margins (clean), 0.045 on the right (Cnt.c.dig), 0.065 (clean); outside: 0.065
    // (Cnt.c); an Activ in the hole of a DigiBnd frame at 0.065 (analog: Cnt.c); the
    // DigiBnd edge through the Activ 0.03 right of a Cont whose margins are 0.10 (clean
    // - the Activ goes on past the DigiBnd); the DigiBnd edge through the Cont itself,
    // margins 0.10 (clean); a 0.10 DigiBnd inside the Cont at 0.065 (digital: clean);
    // a digital 0.045 straddling x = 40 (Cnt.c.dig).
    let mut e = vec![rect(l.digi, 1.0, 1.0, 9.0, 3.0)];
    e.extend(l.c_in_m(activ, 2.0, 2.0, 0.05));
    e.extend(l.c_in(activ, 4.0, 2.0, 0.05, 0.045, 0.05, 0.05));
    e.extend(l.c_in_m(activ, 6.0, 2.0, 0.065));
    e.extend(l.c_in(activ, 12.0, 2.0, 0.07, 0.065, 0.07, 0.07));
    e.push(l.frame(l.digi, 14.5, 1.5, 15.5, 2.5));
    e.extend(l.c_in(activ, 15.0, 2.0, 0.07, 0.065, 0.07, 0.07));
    e.extend(l.c_in_m(activ, 18.0, 2.0, 0.10));
    e.push(rect(l.digi, 17.0, 1.0, 18.11, 3.0));
    e.extend(l.c_in_m(activ, 22.0, 2.0, 0.10));
    e.push(rect(l.digi, 21.2, 1.0, 22.0, 3.0));
    e.extend(l.c_in_m(activ, 24.0, 2.0, 0.065));
    e.push(rect(l.digi, 23.95, 1.95, 24.05, 2.05));
    e.extend(l.c_in(activ, 40.0, 2.0, 0.07, 0.045, 0.07, 0.07));
    e.push(rect(l.digi, 39.0, 1.0, 41.0, 3.0));
    e.extend(l.plates(&[m1], 1.0, 1.0, 42.0, 3.0));
    write("Cnt.c.h8", e);
}

/// Cnt.c/Cnt.d h2: 45° cuts of the enclosing `layer` near the Cont's corner.
fn slant_set(l: &L, layer: (i16, i16)) -> Vec<GdsElement> {
    // Cont corner at (cx + 0.08, 2.08); a line x + y = k passes it at (k − cx − 2.16)/√2.
    let cut = |cx: f64, m: f64, k: f64| {
        vec![
            l.chamfer_tr(
                layer,
                cx - 0.08 - m,
                2.0 - 0.08 - m,
                cx + 0.08 + m,
                2.08 + m,
                k,
            ),
            l.c(cx, 2.0),
        ]
    };
    let mut e = vec![];
    e.extend(cut(2.0, 0.20, 4.25)); // 0.0636
    e.extend(cut(4.0, 0.20, 6.26)); // 0.0707
    e.extend(cut(6.0, 0.50, 8.25)); // 0.0636
    e.extend(cut(8.0, 0.50, 10.26)); // 0.0707
    e.extend(l.plates(&[l.m1], 1.0, 1.0, 10.0, 3.0));
    e
}

/// Cnt.c/Cnt.d h3: the enclosing `layer` drawn as pieces that merge.
fn merge_set(l: &L, layer: (i16, i16)) -> Vec<GdsElement> {
    let mut e = vec![
        rect(layer, 1.5, 1.5, 2.0, 2.5),
        rect(layer, 2.0, 1.5, 2.5, 2.5),
        l.c(2.0, 2.0),
        rect(layer, 3.5, 1.5, 4.05, 2.5),
        rect(layer, 3.95, 1.5, 4.5, 2.5),
        l.c(4.0, 2.0),
    ];
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (5.5 + 0.25 * i as f64, 1.5 + 0.25 * j as f64);
            e.push(rect(layer, x, y, x + 0.25, y + 0.25));
        }
    }
    e.extend(vec![
        l.c(6.0, 2.0),
        rect(layer, 7.5, 1.5, 8.0, 2.5),
        rect(layer, 8.0, 1.5, 8.145, 2.5),
        l.c(8.0, 2.0), // union: 0.065 right
        rect(layer, 9.5, 1.5, 10.145, 2.5),
        rect(layer, 9.5, 1.5, 10.145, 2.5),
        l.c(10.0, 2.0), // duplicate: 0.065 right, once
        l.frame(layer, 12.0, 2.0, 13.0, 3.0),
        l.c(11.855, 2.5), // on the ring's left wall, 0.065 from the hole
    ]);
    e.extend(l.plates(&[l.m1], 1.0, 1.0, 14.5, 4.5));
    e
}

/// Cnt.c/Cnt.d h4: 0.065 margins of `layer` on and across the tile lines.
fn tile_set(l: &L, layer: (i16, i16)) -> Vec<GdsElement> {
    let mut e = vec![];
    // Cont straddling x = 20, 0.065 on the right.
    e.extend(l.c_in(layer, 20.0, 2.0, 0.30, 0.065, 0.07, 0.07));
    // Activ edge on x = 20, Cont 0.065 left of it.
    e.extend(l.c_in(layer, 19.855, 3.0, 0.30, 0.065, 0.07, 0.07));
    // Margin 19.965..20.03 straddles the line.
    e.extend(l.c_in(layer, 19.885, 4.0, 0.30, 0.065, 0.07, 0.07));
    // Margin 19.965..20.035: 0.07, clean.
    e.extend(l.c_in(layer, 19.885, 5.0, 0.30, 0.07, 0.07, 0.07));
    for (x, y) in [(21.0, 6.0), (40.0, 2.0), (42.0, 3.0), (14.0, 2.0)] {
        e.extend(l.c_in(layer, x, y, 0.30, 0.065, 0.07, 0.07));
    }
    e.extend(l.c_in(layer, 10.0, 20.0, 0.07, 0.07, 0.30, 0.065));
    e.extend(l.plates(&[l.m1], 1.0, 1.0, 45.0, 7.0));
    e.extend(l.plates(&[l.m1], 9.0, 19.0, 11.0, 21.0));
    e
}

/// Cnt.c/Cnt.d h7: 300 µm strips of `layer` and a Cont at (1000, 1000).
fn far_set(l: &L, layer: (i16, i16)) -> Vec<GdsElement> {
    let mut e = vec![
        rect(layer, 2.0, 2.0, 302.0, 2.30),
        rect(layer, 2.0, 4.0, 302.0, 4.29),
    ];
    for x in [5.0, 150.0, 300.0] {
        e.push(l.c(x, 2.15));
        e.push(l.c(x, 4.145));
    }
    e.extend(l.c_in(layer, 1000.0, 1000.0, 0.07, 0.065, 0.07, 0.07));
    e.extend(l.plates(&[l.m1], 1.0, 1.0, 303.0, 5.0));
    e.extend(l.plates(&[l.m1], 999.0, 999.0, 1001.0, 1001.0));
    e
}

// --- Cnt.d: min. GatPoly enclosure of Cont 0.07 ---

fn cnt_d_h(l: &L) {
    let (activ, gp, m1) = (l.activ, l.gp, l.m1);

    // h1 — the bound, as Cnt.c.h1 with GatPoly: 0.07 clean; 0.065 right (1), all sides
    // (4), corner (2), 0.005 (1), 0 (1); 0.05 past the edge is Cnt.d and Cnt.g.
    let mut e = vec![];
    e.extend(l.c_in_m(gp, 2.0, 2.0, 0.07));
    e.extend(l.c_in(gp, 4.0, 2.0, 0.07, 0.065, 0.07, 0.07));
    e.extend(l.c_in_m(gp, 6.0, 2.0, 0.065));
    e.extend(l.c_in(gp, 8.0, 2.0, 0.07, 0.065, 0.07, 0.065));
    e.extend(l.c_in(gp, 10.0, 2.0, 0.005, 0.07, 0.07, 0.07));
    e.extend(l.c_in(gp, 12.0, 2.0, 0.07, 0.0, 0.07, 0.07));
    e.extend(l.c_in(gp, 14.0, 2.0, 0.07, -0.05, 0.07, 0.07));
    e.extend(l.plates(&[m1], 1.0, 1.0, 16.0, 3.0));
    write("Cnt.d.h1", e);

    // h2 — 45° cuts (see Cnt.c.h2): clean by the settled projection reading.
    write("Cnt.d.h2", slant_set(l, gp));

    // h3 — merged GatPoly (see Cnt.c.h3): three fire.
    write("Cnt.d.h3", merge_set(l, gp));

    // h4 — tile lines (see Cnt.c.h4): eight fire.
    write("Cnt.d.h4", tile_set(l, gp));

    // h5/h6 — fifty Conts with a 0.065 right margin, flat and as an array reference.
    let mut cell = l.c_in(gp, 0.5, 0.5, 0.07, 0.065, 0.07, 0.07);
    cell.extend(l.plates(&[m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.d", 5, cell, 1.0);

    // h7 — large and far (see Cnt.c.h7): seven markers.
    write("Cnt.d.h7", far_set(l, gp));

    // h8 — the Cont on a gate and across the poly's edge.  A Cont on GatPoly over Activ
    // with the poly 0.065 past it on the right is Cnt.d and Cnt.j; a Cont straddling
    // the seam of an abutting GatPoly and Activ is enclosed by neither (Cnt.d, Cnt.c) and
    // is 0 from the other layer either way (Cnt.e, Cnt.f) - the union covers it, so no
    // Cnt.g, and nothing overlaps, so no Cnt.j; a Cont half on GatPoly and half on
    // nothing is Cnt.d and Cnt.g.
    let mut e = vec![
        rect(activ, 1.0, 1.0, 3.0, 3.0),
        rect(gp, 1.5, 1.5, 2.145, 2.5),
        l.c(2.0, 2.0),
        rect(gp, 3.5, 1.5, 4.0, 2.5),
        rect(activ, 4.0, 1.5, 4.5, 2.5),
        l.c(4.0, 2.0),
        rect(gp, 5.5, 1.5, 6.0, 2.5),
        l.c(6.0, 2.0),
    ];
    e.extend(l.plates(&[m1], 0.5, 0.5, 7.0, 3.5));
    write("Cnt.d.h8", e);
}

// --- Cnt.a: min. and max. Cont width 0.16 (square Cont) ---

fn cnt_a_h(l: &L) {
    let (cont, activ, m1) = (l.cont, l.activ, l.m1);

    // h1 — the bound and the same square drawn differently.  A 0.16 square drawn as one
    // box, as four abutting 0.08 quadrants, as two overlapping boxes, with eight vertices
    // (mid-edge points), twice on top of itself, or clockwise is one 0.16 square: clean.
    // 0.155 and 0.165 squares fail on all four walls.  A 0.16 × 0.165 rectangle and a
    // square missing a 0.005 corner are not squares: section 5.15's, not Cnt.a's.
    let mut e = vec![
        l.sq(2.0, 2.0),
        rect(cont, 4.0, 2.0, 4.08, 2.08),
        rect(cont, 4.08, 2.0, 4.16, 2.08),
        rect(cont, 4.0, 2.08, 4.08, 2.16),
        rect(cont, 4.08, 2.08, 4.16, 2.16),
        rect(cont, 6.0, 2.0, 6.10, 2.16),
        rect(cont, 6.06, 2.0, 6.16, 2.16),
        poly(
            cont,
            &[
                (8.0, 2.0),
                (8.08, 2.0),
                (8.16, 2.0),
                (8.16, 2.08),
                (8.16, 2.16),
                (8.08, 2.16),
                (8.0, 2.16),
                (8.0, 2.08),
            ],
        ),
        l.sq(10.0, 2.0),
        l.sq(10.0, 2.0),
        poly(
            cont,
            &[(12.0, 2.0), (12.0, 2.155), (12.155, 2.155), (12.155, 2.0)],
        ), // CW, 0.155: 4
        rect(cont, 14.0, 2.0, 14.155, 2.155), // 4
        rect(cont, 16.0, 2.0, 16.165, 2.165), // 4
        rect(cont, 18.0, 2.0, 18.16, 2.165),  // bar
        poly(
            cont,
            &[
                (2.0, 4.0),
                (2.16, 4.0),
                (2.16, 4.155),
                (2.155, 4.155),
                (2.155, 4.16),
                (2.0, 4.16),
            ],
        ), // bar
    ];
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 20.0, 5.0));
    write("Cnt.a.h1", e);

    // h2 — tile lines.  0.155 squares straddling x = 20, 21, 40, 42 and y = 20, one with
    // its right edge exactly on x = 20, one far away at (1000, 1000): four walls each
    // (7 × 4 = 28).  A 0.16 square with its right edge on x = 20 is clean.
    let mut e = vec![
        rect(cont, 19.925, 2.0, 20.08, 2.155),
        rect(cont, 20.925, 4.0, 21.08, 4.155),
        rect(cont, 39.925, 2.0, 40.08, 2.155),
        rect(cont, 41.925, 4.0, 42.08, 4.155),
        rect(cont, 10.0, 19.925, 10.155, 20.08),
        rect(cont, 19.845, 6.0, 20.0, 6.155),
        rect(cont, 19.84, 8.0, 20.0, 8.16),
        rect(cont, 1000.0, 1000.0, 1000.155, 1000.155),
    ];
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 45.0, 9.0));
    e.extend(l.plates(&[activ, m1], 9.0, 19.0, 11.0, 21.0));
    e.extend(l.plates(&[activ, m1], 999.0, 999.0, 1001.0, 1001.0));
    write("Cnt.a.h2", e);

    // h3/h4 — fifty 0.155 squares, flat and as an array reference: 200 walls each.
    let mut cell = vec![rect(cont, 0.5, 0.5, 0.655, 0.655)];
    cell.extend(l.plates(&[activ, m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.a", 3, cell, 1.0);
}

// --- Cnt.b: min. Cont space 0.18 ---

fn cnt_b_h(l: &L) {
    let (cont, activ, m1) = (l.cont, l.activ, l.m1);

    // h1 — the bound and both metrics.  0.18 apart is clean, 0.175 fires in x and in y.
    // Corner to corner: 0.125/0.125 (0.177) fires, 0.13/0.13 (0.184) is clean; offset
    // 0.10/0.15 is 0.180 euclidian (clean) though 0.15 in the axis.  Three squares in a
    // row 0.175 apart are two pairs.  Cont is 90° only (section 3.1), so no 45° here.
    let mut e = vec![
        l.sq(2.0, 2.0),
        l.sq(2.34, 2.0), // 0.18: clean
        l.sq(4.0, 2.0),
        l.sq(4.335, 2.0), // 0.175 x
        l.sq(6.0, 2.0),
        l.sq(6.0, 2.335), // 0.175 y
        l.sq(8.0, 2.0),
        l.sq(8.285, 2.285), // 0.177 diagonal
        l.sq(10.0, 2.0),
        l.sq(10.29, 2.29), // 0.184 diagonal: clean
        l.sq(12.0, 2.0),
        l.sq(12.26, 2.31), // 0.10/0.15 → 0.1803: clean
        l.sq(14.0, 2.0),
        l.sq(14.335, 2.0),
        l.sq(14.67, 2.0), // two pairs
    ];
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 16.0, 4.0));
    write("Cnt.b.h1", e);

    // h2 — tile lines.  0.175 gaps straddling x = 20, 21, 40, 42, 14 and y = 20; a gap
    // that begins exactly on x = 20 and one that ends on it; a pair at (1000, 1000).
    let pair = |x: f64, y: f64| vec![l.sq(x, y), l.sq(x + 0.335, y)];
    let mut e = vec![];
    e.extend(pair(19.745, 2.0));
    e.extend(pair(20.745, 4.0));
    e.extend(pair(39.745, 2.0));
    e.extend(pair(41.745, 4.0));
    e.extend(pair(13.745, 2.0));
    e.extend(vec![l.sq(10.0, 19.745), l.sq(10.0, 20.08)]);
    e.extend(pair(19.84, 6.0)); // gap 20.0..20.175
    e.extend(pair(19.665, 8.0)); // gap 19.825..20.0
    e.extend(pair(1000.0, 1000.0));
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 45.0, 9.0));
    e.extend(l.plates(&[activ, m1], 9.0, 19.0, 11.0, 21.0));
    e.extend(l.plates(&[activ, m1], 999.0, 999.0, 1002.0, 1001.0));
    write("Cnt.b.h2", e);

    // h3/h4 — fifty 0.175 pairs, flat and as an array reference.
    let mut cell = pair(0.3, 0.42);
    cell.extend(l.plates(&[activ, m1], 0.0, 0.0, 1.0, 1.0));
    arrays("Cnt.b", 3, cell, 1.0);

    // h5 — what is and is not a pair.  Two squares touching at a corner are 0 apart
    // (fires, whatever the merge makes of the touch); a square 0.175 from a 0.16 × 0.40
    // bar is CntB.b2's (0.22), not Cnt.b's; a 0.155 square 0.175 from a 0.16 one is
    // Cnt.b once (and Cnt.a four times).
    let mut e = vec![
        l.sq(2.0, 2.0),
        l.sq(2.16, 2.16),
        l.sq(4.0, 2.0),
        rect(cont, 4.335, 2.0, 4.495, 2.40),
        l.sq(6.0, 2.0),
        rect(cont, 6.335, 2.0, 6.49, 2.155),
    ];
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 8.0, 4.0));
    write("Cnt.b.h5", e);
}

// --- Cnt.b1: min. Cont space 0.20 in an array of more than 4 rows and more than 4 columns, one direction ---

fn cnt_b1_h(l: &L) {
    let (activ, m1) = (l.activ, l.m1);

    // h1 — the array's definition.  5 × 5 at 0.18/0.18 fires; 5 × 5 at 0.20 in x and 0.18
    // in y, or the reverse, is clean (note 1: one direction suffices); 4 rows × 5 cols,
    // 5 rows × 4 cols and 4 × 4 at 0.18 are not arrays the rule covers; 5 × 5 at
    // 0.195/0.195 fires; 5 × 5 at 0.20/0.20 is clean.
    let mut e = vec![];
    e.extend(l.grid(2.0, 2.0, 5, 5, 0.18, 0.18)); // fires
    e.extend(l.grid(6.0, 2.0, 5, 5, 0.20, 0.18)); // clean
    e.extend(l.grid(10.0, 2.0, 5, 5, 0.18, 0.20)); // clean
    e.extend(l.grid(14.0, 2.0, 5, 4, 0.18, 0.18)); // 4 rows: clean
    e.extend(l.grid(2.0, 6.0, 4, 5, 0.18, 0.18)); // 4 cols: clean
    e.extend(l.grid(6.0, 6.0, 5, 5, 0.195, 0.195)); // fires
    e.extend(l.grid(10.0, 6.0, 5, 5, 0.20, 0.20)); // clean
    e.extend(l.grid(14.0, 6.0, 4, 4, 0.18, 0.18)); // clean
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 17.0, 9.0));
    write("Cnt.b1.h1", e);

    // h2 — larger and irregular arrays.  6 × 6 and 5 rows × 10 cols at 0.18 fire; 5 × 5
    // at 0.18 with the centre Cont missing is still five rows of five (fires); 5 × 5 whose
    // row gaps alternate 0.18/0.20 has no direction at 0.20 throughout (fires); five rows
    // of five staggered by half a pitch, rows 0.18 apart and columns 0.18 apart in each
    // row, is an array in the rule's sense (fires); a 5 × 5 at 0.18 made of a 3-column
    // and a 2-column block drawn as separate groups is one array (fires).
    let mut e = vec![];
    e.extend(l.grid(2.0, 2.0, 6, 6, 0.18, 0.18));
    e.extend(l.grid(6.0, 2.0, 10, 5, 0.18, 0.18));
    let mut holed = l.grid(2.0, 6.0, 5, 5, 0.18, 0.18);
    holed.remove(12);
    e.extend(holed);
    for dy in [0.0, 0.34, 0.70, 1.04, 1.40] {
        e.extend(l.grid(6.0, 6.0 + dy, 5, 1, 0.18, 0.0));
    }
    for r in 0..5 {
        let x = 10.0 + if r % 2 == 1 { 0.17 } else { 0.0 };
        e.extend(l.grid(x, 6.0 + r as f64 * 0.34, 5, 1, 0.18, 0.0));
    }
    e.extend(l.grid(14.0, 6.0, 3, 5, 0.18, 0.18));
    e.extend(l.grid(15.02, 6.0, 2, 5, 0.18, 0.18));
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 17.0, 9.0));
    write("Cnt.b1.h2", e);

    // h3 — tile lines.  5 × 5 arrays at 0.18 straddling x = 20, 21, 40, 42 and 14, one
    // straddling both x = 20 and y = 20, one at (1000, 1000): each fires as the one at
    // x = 10 does.
    let mut e = vec![];
    for (x, y) in [
        (9.15, 2.0),
        (19.15, 2.0),
        (20.15, 5.0),
        (39.15, 2.0),
        (41.15, 5.0),
        (13.15, 5.0),
        (19.15, 19.15),
        (1000.0, 1000.0),
    ] {
        e.extend(l.grid(x, y, 5, 5, 0.18, 0.18));
    }
    e.extend(l.plates(&[activ, m1], 1.0, 1.0, 45.0, 8.0));
    e.extend(l.plates(&[activ, m1], 18.0, 18.0, 22.0, 22.0));
    e.extend(l.plates(&[activ, m1], 999.0, 999.0, 1003.0, 1003.0));
    write("Cnt.b1.h3", e);

    // h4/h5 — fifty 5 × 5 arrays at 0.18, flat and as an array reference of a cell
    // holding one array.
    let mut cell = l.grid(0.5, 0.5, 5, 5, 0.18, 0.18);
    cell.extend(l.plates(&[activ, m1], 0.0, 0.0, 3.0, 3.0));
    arrays("Cnt.b1", 4, cell, 3.0);

    // h6 — the array itself as hierarchy: a 5 × 5 `GdsArrayRef` of a one-Cont cell at
    // pitch 0.34 (0.18 gaps) fires like the flat array; beside it, flat, a 5 × 5 at 0.20
    // in x (clean) and a 5 × 5 whose gaps are 0.175 in both directions, which is Cnt.b
    // (40 pairs) as well as Cnt.b1.
    let mut extra = l.grid(6.0, 2.0, 5, 5, 0.20, 0.18);
    extra.extend(l.grid(10.0, 2.0, 5, 5, 0.175, 0.175));
    extra.extend(l.plates(&[activ, m1], -1.0, -1.0, 13.0, 5.0));
    write_gz(
        &format!("{DIR}/Cnt.b1.h6.gds.gz"),
        ref_array(vec![l.sq(0.0, 0.0)], 5, 5, 0.34, 0.34, extra),
    );
}
