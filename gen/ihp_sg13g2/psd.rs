// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_bl, chamfered_tr, diamond, layer, library, min_width_pattern, rect, space_pattern,
    write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/psd";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    psd_a(pdk);
    psd_b(pdk);
    psd_d(pdk);
    psd_k(pdk);
    psd_l(pdk);
    psd_g(pdk);
    psd_e(pdk);
    psd_f(pdk);
    psd_c1(pdk);

    hardening(pdk);
}

/// One abutted substrate tie: an Activ bar of height `h` whose right portion is covered
/// by a pSD rect overlapping it by `ov` (the P+ overlap sliver = `ov`×`h`); the uncovered
/// left portion is the N+ tap.  The pSD extends 1.5 µm past the Activ so it is a healthy
/// standalone pSD region (pSD.a/k quiet), and 0.1 vertically past it so pSD.c1's 0.03
/// enclosure stays quiet (a flush sliver is a real pSD.c1 violation — both tools agree).
fn tie(pdk: &PdkConfig, x: f64, y: f64, h: f64, ov: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "Activ"), x, y, x + 1.5, y + h),
        rect(
            layer(pdk, "pSD"),
            x + 1.5 - ov,
            y - 0.1,
            x + 3.0,
            y + h + 0.1,
        ),
    ]
}

/// pSD.e — "min. pSD overlap of Activ at one position" (0.30):
/// - A: uniform 0.30 sliver (exactly at value) → clean.
/// - B: uniform 0.20 sliver → violation (nowhere ≥0.30).
/// - C: 0.20 sliver with a 0.40-deep × 0.5-tall pocket (extra pSD rect) → clean
///   (wide enough at one position, narrow elsewhere — must NOT fire).
/// - D: 0.20 × 0.5 tab → violation (0.5 long but only 0.2 in the narrow direction;
///   the real rule fails this too — width, not length, is the metric).
fn psd_e(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = tie(pdk, o, o, 1.0, 0.30); // A
    e.extend(tie(pdk, o + 8.0, o, 1.0, 0.20)); // B
    e.extend(tie(pdk, o + 16.0, o, 1.0, 0.20)); // C base...
    // ...plus the wide pocket: pSD reaching 0.40 into the Activ over 0.5 of its height.
    e.push(rect(
        layer(pdk, "pSD"),
        o + 16.0 + 1.5 - 0.40,
        o + 0.25,
        o + 16.0 + 3.0,
        o + 0.75,
    ));
    e.extend(tie(pdk, o + 24.0, o, 0.5, 0.20)); // D (sliver area 0.10 ≥ 0.09 keeps pSD.g quiet)
    write_gz(&format!("{DIR}/pSD.e.gds.gz"), library("TOP", e));
}

/// One abutted NWell tie at `cx`: NWell 10×10, a P+ body (Activ 4×2 under pSD ending
/// flush at the abutment line) and an N+ tab of `w`×`d` on top (drawn nSD over it, as
/// KLayout's recognition wants; ours derives N+ as Activ−pSD).
fn nwell_tie(pdk: &PdkConfig, cx: f64, cy: f64, w: f64, d: f64) -> Vec<gds21::GdsElement> {
    let mut e = nwell_tie_base(pdk, cx, cy);
    let tx = cx + 5.0 - w / 2.0;
    e.push(rect(
        layer(pdk, "Activ"),
        tx,
        cy + 6.0,
        tx + w,
        cy + 6.0 + d,
    ));
    e.push(rect(layer(pdk, "nSD"), tx, cy + 6.0, tx + w, cy + 6.0 + d));
    e
}

fn nwell_tie_base(pdk: &PdkConfig, cx: f64, cy: f64) -> Vec<gds21::GdsElement> {
    vec![
        rect(layer(pdk, "NWell"), cx, cy, cx + 10.0, cy + 10.0),
        rect(layer(pdk, "Activ"), cx + 3.0, cy + 4.0, cx + 7.0, cy + 6.0),
        rect(layer(pdk, "pSD"), cx + 2.8, cy + 3.8, cx + 7.2, cy + 6.0),
    ]
}

/// pSD.f — min. Activ extension over pSD at ONE position (abutted NWell tie, 0.30).
/// Tab depth `d` is the metric, not its width:
/// - d=0.20 → fires;  d=0.30 → clean;  d=0.295 → fires (grid boundary);
/// - 0.2-wide × 0.45-deep tab → clean (dies under a min-width/opening formulation —
///   the pSD.e reformulation is provably wrong here; the depth reaches 0.45);
/// - 2.0-wide × 0.20-deep → fires;
/// - L-shaped tab (0.1-deep stem + sideways arm at ≤0.2) → fires HERE ONLY: nowhere
///   extends 0.30 past the abutment (PDF-first); the shipped KLayout body's "bad band"
///   covers only the area directly in front of the abutment edge, letting the arm
///   escape.  Tab widths ≥0.5 (or area ≥0.09) keep pSD.g quiet.
fn psd_f(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = nwell_tie(pdk, o, o, 0.5, 0.20); // fires
    e.extend(nwell_tie(pdk, o + 20.0, o, 1.0, 0.30)); // clean
    e.extend(nwell_tie(pdk, o + 40.0, o, 0.5, 0.295)); // fires
    e.extend(nwell_tie(pdk, o + 60.0, o, 0.2, 0.45)); // clean (area exactly 0.09)
    e.extend(nwell_tie(pdk, o + 80.0, o, 2.0, 0.20)); // fires
    let cx = o + 100.0;
    e.extend(nwell_tie_base(pdk, cx, o)); // L-shape: fires (ours only)
    e.push(rect(
        layer(pdk, "Activ"),
        cx + 4.9,
        o + 6.0,
        cx + 5.1,
        o + 6.1,
    )); // stem
    e.push(rect(
        layer(pdk, "Activ"),
        cx + 4.9,
        o + 6.1,
        cx + 5.9,
        o + 6.2,
    )); // arm
    e.push(rect(
        layer(pdk, "nSD"),
        cx + 4.9,
        o + 6.0,
        cx + 5.1,
        o + 6.1,
    ));
    e.push(rect(
        layer(pdk, "nSD"),
        cx + 4.9,
        o + 6.1,
        cx + 5.9,
        o + 6.2,
    ));
    write_gz(&format!("{DIR}/pSD.f.gds.gz"), library("TOP", e));
}

/// pSD.c1 — min. pSD enclosure of P+Activ in implicit PWell (0.03), all cases
/// validated 1:1 against the FEOL driver (`enclosed` euclidian semantics):
/// - margin 0.02 → fires;  0.05 → clean;  exactly 0.03 → clean;
/// - abutted substrate tie crossing the pSD edge, lateral margins 0.10 → CLEAN
///   (the protruding part is ignored — crossing edges don't pair);
/// - flush left edge → fires (0 < 0.03 — no skip_coincident here, Rppd.b-style);
/// - crossing tie with a bad 0.02 lateral margin → fires (the inside portion of the
///   crossing edge still pairs — run_enclosure's partial-overlap measurement).
fn psd_c1(pdk: &PdkConfig) {
    let o = OFFSET;
    let m = |cx: f64, margin: f64| -> Vec<gds21::GdsElement> {
        vec![
            rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0),
            rect(
                layer(pdk, "Activ"),
                cx + margin,
                o + margin,
                cx + 4.0 - margin,
                o + 4.0 - margin,
            ),
        ]
    };
    let mut e = m(o, 0.02); // fires
    e.extend(m(o + 20.0, 0.05)); // clean
    e.extend(m(o + 40.0, 0.03)); // clean (boundary)
    let cx = o + 60.0; // crossing tie, margins 0.10 → clean
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(
        layer(pdk, "Activ"),
        cx + 0.1,
        o + 0.1,
        cx + 3.9,
        o + 4.5,
    ));
    e.push(rect(
        layer(pdk, "nSD"),
        cx + 0.1,
        o + 4.0,
        cx + 3.9,
        o + 4.5,
    ));
    let cx = o + 80.0; // flush left edge → fires
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(layer(pdk, "Activ"), cx, o + 0.1, cx + 3.9, o + 3.9));
    let cx = o + 100.0; // crossing tie, bad 0.02 lateral margin → fires
    e.push(rect(layer(pdk, "pSD"), cx, o, cx + 4.0, o + 4.0));
    e.push(rect(
        layer(pdk, "Activ"),
        cx + 0.02,
        o + 0.1,
        cx + 3.9,
        o + 4.5,
    ));
    e.push(rect(
        layer(pdk, "nSD"),
        cx + 0.02,
        o + 4.0,
        cx + 3.9,
        o + 4.5,
    ));
    write_gz(&format!("{DIR}/pSD.c1.gds.gz"), library("TOP", e));
}

/// pSD.a — min width 0.31 (one clean + two under-width shapes).
fn psd_a(pdk: &PdkConfig) {
    let elems = min_width_pattern(layer(pdk, "pSD"), 0.31, 2.0, 3.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.a.gds.gz"), library("TOP", elems));
}

/// pSD.b — min space 0.31 between pSD regions (two clean, two too-close).
fn psd_b(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let elems = space_pattern(p, p, 1.0, 0.31, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.b.gds.gz"), library("TOP", elems));
}

/// pSD.d — min space 0.18 of pSD to N+Activ in PWell.  The neighbours are bare Activ
/// (no pSD, no NWell) → they derive as NActivInPWell.
fn psd_d(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let a = layer(pdk, "Activ");
    let elems = space_pattern(p, a, 1.0, 0.18, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/pSD.d.gds.gz"), library("TOP", elems));
}

/// pSD.k — min area 0.25 µm² (a 0.4×0.4 = 0.16 µm² dab fails; a 0.6×0.6 passes).
fn psd_k(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let o = OFFSET;
    let elems = vec![
        rect(p, o, o, o + 0.4, o + 0.4),       // 0.16 µm² → violation
        rect(p, o + 5.0, o, o + 5.6, o + 0.6), // 0.36 µm² → clean
    ];
    write_gz(&format!("{DIR}/pSD.k.gds.gz"), library("TOP", elems));
}

/// pSD.l — min enclosed (hole) area 0.25 µm²: a pSD frame around a 0.4×0.4 = 0.16 µm² hole.
fn psd_l(pdk: &PdkConfig) {
    let p = layer(pdk, "pSD");
    let o = OFFSET;
    // Frame around the hole (o+1, o+1)-(o+1.4, o+1.4); thickness 1.0.
    let elems = vec![
        rect(p, o, o, o + 2.4, o + 1.0),             // bottom
        rect(p, o, o + 1.4, o + 2.4, o + 2.4),       // top
        rect(p, o, o + 1.0, o + 1.0, o + 1.4),       // left
        rect(p, o + 1.4, o + 1.0, o + 2.4, o + 1.4), // right
    ];
    write_gz(&format!("{DIR}/pSD.l.gds.gz"), library("TOP", elems));
}

/// pSD.g — min. abutted-tie area 0.09 µm², checked on both the N-tap (Activ in NWell) and
/// P-tap (Activ+pSD outside NWell) flavours: a 0.06 µm² tie fails, a 0.12+ µm² one is clean.
/// The rule is "when forming abutted tie": each N-tap abuts a P+ Activ (a tap on its own
/// is Act.d's).
fn psd_g(pdk: &PdkConfig) {
    let o = OFFSET;
    let activ = layer(pdk, "Activ");
    let psd = layer(pdk, "pSD");
    let nwell = layer(pdk, "NWell");
    let elems = vec![
        // N-tap: bare Activ (no pSD) inside NWell abutting a P+ Activ on its left — the
        // whole tie area is the tiny one, so it doesn't collide with any raw-pSD rule.
        rect(nwell, o - 1.0, o - 1.0, o + 1.0, o + 1.0),
        rect(activ, o, o, o + 0.3, o + 0.2), // 0.06 µm² → violation (0.3 deep: pSD.f clear)
        rect(activ, o - 0.5, o - 0.15, o, o + 0.35),
        rect(psd, o - 0.7, o - 0.35, o, o + 0.55), // the P+ tap it abuts (0.5 wide: pSD.e clear)
        rect(nwell, o + 5.0 - 1.0, o - 1.0, o + 5.0 + 1.0, o + 1.0),
        rect(activ, o + 5.0, o, o + 5.3, o + 0.4), // 0.12 µm² → clean
        rect(activ, o + 4.5, o - 0.05, o + 5.0, o + 0.45),
        rect(psd, o + 4.3, o - 0.25, o + 5.0, o + 0.65),
        // P-tap: a 1.0×1.0 Activ square with pSD extended 0.2 µm past it on every side (so
        // pSD's own boundary never coincides with the NWell-clipped tie edge — avoiding the
        // pre-existing pSD.c coincident-edge false positive, see [[gdscheck-min-enclosure-
        // coincident-bug]]).  NWell covers all of Activ but a small corner sliver — only
        // that NWell-subtracted sliver (Activ+pSD outside NWell) is what pSD.g measures.
        rect(activ, o + 10.0, o, o + 11.0, o + 1.0),
        rect(psd, o + 9.8, o - 0.2, o + 11.2, o + 1.2),
        rect(nwell, o + 9.0, o - 1.0, o + 11.0, o + 0.7),
        rect(nwell, o + 9.0, o - 1.0, o + 10.8, o + 1.0), // sliver left: 0.2×0.3 = 0.06 → violation
        rect(activ, o + 15.0, o, o + 16.0, o + 1.0),
        rect(psd, o + 14.8, o - 0.2, o + 16.2, o + 1.2),
        rect(nwell, o + 14.0, o - 1.0, o + 16.0, o + 0.7),
        rect(nwell, o + 14.0, o - 1.0, o + 15.6, o + 1.0), // sliver left: 0.4×0.3 = 0.12 → clean
    ];
    write_gz(&format!("{DIR}/pSD.g.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the implant decks: section 5.7 ThickGateOxide (TGO.a-TGO.f),
// section 5.10 pSD (pSD.a-pSD.n) and section 5.11 nSD:block (nSDB.a-nSDB.e) of the
// SG13G2 layout rules, with section 4.2's derivations (N+/P+ Activ by drawn nSD/pSD or
// by default, NFET/PFET, the ties).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.

/// One grid step.
pub(super) const S: f64 = 0.005;

/// The layers the three decks draw on.
pub(super) struct L {
    pub(super) tgo: (i16, i16),
    pub(super) act: (i16, i16),
    pub(super) gp: (i16, i16),
    pub(super) psd: (i16, i16),
    pub(super) nsd: (i16, i16),
    pub(super) nsdb: (i16, i16),
    pub(super) nw: (i16, i16),
    pub(super) pwb: (i16, i16),
    pub(super) cont: (i16, i16),
    pub(super) sal: (i16, i16),
    pub(super) res: (i16, i16),
    pub(super) ext: (i16, i16),
}

impl L {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        L {
            tgo: layer(pdk, "ThickGateOx"),
            act: layer(pdk, "Activ"),
            gp: layer(pdk, "GatPoly"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            nsdb: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            pwb: layer(pdk, "PWell.block"),
            cont: layer(pdk, "Cont"),
            sal: layer(pdk, "SalBlock"),
            res: layer(pdk, "RES"),
            ext: layer(pdk, "EXTBlock"),
        }
    }
}

pub(super) fn path(deck: &str, name: &str) -> String {
    format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz")
}

pub(super) fn write(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(&path(deck, name), library("TOP", elems));
}

/// A frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, four abutting boxes
/// that merge into one ring.
#[allow(clippy::too_many_arguments)]
pub(super) fn ring(
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
        rect(l, x0, y0, x1, hy0),
        rect(l, x0, hy1, x1, y1),
        rect(l, x0, hy0, hx0, hy1),
        rect(l, hx1, hy0, x1, hy1),
    ]
}

/// A square box of side `b` with its lower-left corner at `(x, y)`.
pub(super) fn sq(l: (i16, i16), x: f64, y: f64, b: f64) -> GdsElement {
    rect(l, x, y, x + b, y + b)
}

/// The `min_width` suite for `l` at value `v` (bars 3 µm long):
/// h1 the bound (a `v` bar clean; `v − 0.005` bars in x and y, a 0.005 sliver and a 300 µm
///    bar fire, two walls each);
/// h3 unions (two overlapping boxes, two abutting slices and one side of a ring at
///    `v − 0.005` fire; a plate drawn as a 10 × 10 grid is clean).
/// The 45° shapes and the tile lines are the engine's (`gen/engine/width.rs`).
pub(super) fn width_suite(deck: &str, rule: &str, l: (i16, i16), v: f64) {
    let n = v - S;
    write(
        deck,
        &format!("{rule}.h1"),
        vec![
            rect(l, 2.0, 2.0, 2.0 + v, 5.0),
            rect(l, 4.0, 2.0, 4.0 + n, 5.0),
            rect(l, 2.0, 6.0, 5.0, 6.0 + n),
            rect(l, 7.0, 2.0, 7.0 + S, 5.0),
            rect(l, 9.0, -100.0, 9.0 + n, 200.0),
        ],
    );

    let mut e = vec![
        rect(l, 2.0, 2.0, 2.0 + n, 4.0),
        rect(l, 2.0, 3.0, 2.0 + n, 5.0),
        rect(l, 4.0, 2.0, 4.0 + n / 2.0, 5.0),
        rect(l, 4.0 + n / 2.0, 2.0, 4.0 + n, 5.0),
    ];
    e.extend(ring(l, 8.0, 2.0, 11.0, 5.0, 8.0 + n, 3.0, 10.0, 4.0));
    for i in 0..10 {
        for j in 0..10 {
            let (x, y) = (12.0 + 0.3 * i as f64, 2.0 + 0.3 * j as f64);
            e.push(rect(l, x, y, x + 0.3, y + 0.3));
        }
    }
    write(deck, &format!("{rule}.h3"), e);
}

/// The `min_space` suite for `l` at value `v` between boxes of side `b`:
/// h2 notches (a U and a slot into a plate at `v − 0.005`, a comb with two such slots
///    fire; a U at `v` is clean).
/// The bound, the 45° shapes and the tile lines are the engine's (`gen/engine/space.rs`).
pub(super) fn space_suite(deck: &str, rule: &str, l: (i16, i16), v: f64, b: f64) {
    let n = v - S;
    let u = |x: f64, y: f64, g: f64| {
        vec![
            rect(l, x, y, x + b, y + 3.0),
            rect(l, x + b + g, y, x + 2.0 * b + g, y + 3.0),
            rect(l, x, y, x + 2.0 * b + g, y + b),
        ]
    };
    let mut e = u(2.0, 2.0, n);
    e.extend(u(2.0 + 2.0 * b + 2.0, 2.0, v));
    let x = 2.0 + 4.0 * b + 4.0;
    e.push(rect(l, x, 2.0, x + 3.0 * b + 2.0 * n, 2.0 + b));
    for i in 0..3 {
        let tx = x + i as f64 * (b + n);
        e.push(rect(l, tx, 2.0, tx + b, 5.0));
    }
    let x = x + 3.0 * b + 2.0 * n + 2.0;
    e.push(rect(l, x, 2.0, x + 3.0, 2.0 + b));
    e.push(rect(l, x, 2.0, x + 1.5 - n / 2.0, 5.0));
    e.push(rect(l, x + 1.5 + n / 2.0, 2.0, x + 3.0, 5.0));
    write(deck, &format!("{rule}.h2"), e);
}

impl L {
    /// A 1 × 1 Activ at `(x, y)` under a pSD with the margins `ml, mb, mr, mt`.
    fn psd_box(&self, x: f64, y: f64, ml: f64, mb: f64, mr: f64, mt: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.0, y + 1.0),
            rect(self.psd, x - ml, y - mb, x + 1.0 + mr, y + 1.0 + mt),
        ]
    }

    /// An abutted NWell tie at `(x, y)`: a 1 × 1 P+ body under a pSD with the margins
    /// `ml, mb, mt`, the pSD ending on the body's right edge, where the Activ runs on 0.3
    /// as the N+ tab (pSD.f and pSD.g fine).
    fn ntie(&self, x: f64, y: f64, ml: f64, mb: f64, mt: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.3, y + 1.0),
            rect(self.psd, x - ml, y - mb, x + 1.0, y + 1.0 + mt),
        ]
    }

    /// An abutted substrate tie at `(x, y)`: Activ 1.5 × 0.5 whose right `ov` lies under a
    /// pSD reaching 1.5 further right and 0.1 past the Activ in y.
    fn stie(&self, x: f64, y: f64, ov: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.5, y + 0.5),
            rect(self.psd, x + 1.5 - ov, y - 0.1, x + 3.0, y + 0.6),
        ]
    }

    /// An abutted NWell tie at `(x, y)` with the abutment line horizontal: a 2 × 1 P+ body
    /// under a pSD 0.2 past it left, right and below and ending on the body's top edge,
    /// where an N+ tab `w` wide runs on `d` (the pSD's top edge is flush with the body's
    /// beside the tab: figure 5.10's "f not required").
    fn ntie_up(&self, x: f64, y: f64, d: f64, w: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 2.0, y + 1.0),
            rect(self.psd, x - 0.2, y - 0.2, x + 2.2, y + 1.0),
            rect(
                self.act,
                x + 1.0 - w / 2.0,
                y + 1.0,
                x + 1.0 + w / 2.0,
                y + 1.0 + d,
            ),
        ]
    }

    /// A PFET at `(x, y)`: Activ `sdl + gl + sdr` × 1, a gate `gl` long with caps `cap`,
    /// under a pSD `pl`/`pr` past the Activ's S/D ends and `pw` past its width edges.  The
    /// well is the caller's.  pSD.i reads `sdl + pl` and `sdr + pr`.
    #[allow(clippy::too_many_arguments)]
    fn pfet(
        &self,
        x: f64,
        y: f64,
        sdl: f64,
        sdr: f64,
        gl: f64,
        cap: f64,
        pl: f64,
        pr: f64,
        pw: f64,
    ) -> Vec<GdsElement> {
        let x1 = x + sdl + gl + sdr;
        vec![
            rect(self.act, x, y, x1, y + 1.0),
            rect(self.gp, x + sdl, y - cap, x + sdl + gl, y + 1.0 + cap),
            rect(self.psd, x - pl, y - pw, x1 + pr, y + 1.0 + pw),
        ]
    }

    /// An NFET at `(x, y)`: Activ `sdl + gl + sdr` × 1 and a gate `gl` long with caps
    /// `cap`; N+ by default (no pSD, no nSD:block).
    fn nfet(&self, x: f64, y: f64, sdl: f64, sdr: f64, gl: f64, cap: f64) -> Vec<GdsElement> {
        let x1 = x + sdl + gl + sdr;
        vec![
            rect(self.act, x, y, x1, y + 1.0),
            rect(self.gp, x + sdl, y - cap, x + sdl + gl, y + 1.0 + cap),
        ]
    }
}

/// pSD.c, "Min. pSD enclosure of P+Activ in NWell 0.18"; P+Activ is Activ AND pSD
/// (section 4.2), and figure 5.10 draws a P+ edge flush with the pSD's ("f not
/// required") as legal.
fn psd_c(l: &L) {
    // h1 (one well under all): 0.18 all round clean; 0.175 left, 0.175 top, 0.175 all round
    // (four walls) fire; a flush right edge is clean; an Activ in the well without pSD, one
    // a pSD merely abuts and one 0.5 from a pSD are no P+Activ and clean.
    let mut e = vec![rect(l.nw, 0.0, 0.0, 24.0, 10.0)];
    e.extend(l.psd_box(2.0, 2.0, 0.18, 0.18, 0.18, 0.18));
    e.extend(l.psd_box(5.0, 2.0, 0.175, 0.18, 0.18, 0.18));
    e.extend(l.psd_box(8.0, 2.0, 0.18, 0.18, 0.18, 0.175));
    e.extend(l.psd_box(11.0, 2.0, 0.175, 0.175, 0.175, 0.175));
    e.extend(l.psd_box(14.0, 2.0, 0.18, 0.18, 0.0, 0.18));
    e.push(sq(l.act, 17.0, 2.0, 1.0));
    e.push(sq(l.act, 2.0, 6.0, 1.0));
    e.push(rect(l.psd, 3.0, 5.5, 4.5, 7.5));
    e.push(sq(l.act, 6.0, 6.0, 1.0));
    e.push(rect(l.psd, 7.5, 5.5, 9.0, 7.5));
    write("psd", "pSD.c.h1", e);

    // h2: abutted NWell ties (the Activ crossing the pSD's edge into an N+ tab): margins
    // 0.18 clean; 0.175 on the P+ body's left, top and bottom fire.
    let mut e = vec![rect(l.nw, 0.0, 0.0, 20.0, 10.0)];
    e.extend(l.ntie(2.0, 2.0, 0.18, 0.18, 0.18));
    e.extend(l.ntie(5.0, 2.0, 0.175, 0.18, 0.18));
    e.extend(l.ntie(8.0, 2.0, 0.18, 0.18, 0.175));
    e.extend(l.ntie(11.0, 2.0, 0.18, 0.175, 0.18));
    write("psd", "pSD.c.h2", e);

    // h3: a chamfer 0.173 from the Activ's corner (walls 0.18) fires, at 0.18 clean; a
    // diamond Activ in a square pSD 0.175 from its corners fires, at 0.18 clean; a square
    // Activ in a diamond pSD whose walls pass 0.173 from its corners fires.
    write(
        "psd",
        "pSD.c.h3",
        vec![
            rect(l.nw, 0.0, 0.0, 20.0, 10.0),
            chamfered_tr(l.psd, 2.0, 2.0, 3.36, 3.36, 6.605),
            rect(l.act, 2.18, 2.18, 3.18, 3.18),
            chamfered_tr(l.psd, 5.0, 2.0, 6.36, 3.36, 9.615),
            rect(l.act, 5.18, 2.18, 6.18, 3.18),
            rect(l.psd, 8.325, 2.325, 9.675, 3.675),
            diamond(l.act, 9.0, 3.0, 0.5),
            rect(l.psd, 11.32, 2.32, 12.68, 3.68),
            diamond(l.act, 12.0, 3.0, 0.5),
            diamond(l.psd, 15.0, 3.0, 0.745),
            rect(l.act, 14.75, 2.75, 15.25, 3.25),
        ],
    );

    // h4 (a 2 × 2 well round each): a pSD drawn as two overlapping boxes with 0.175, an
    // Activ drawn as four quadrants with 0.175, a well drawn as two abutting boxes with
    // 0.175 fire; 0.175 with the pSD's edge on x = 20, the Activ's edge on 20, straddling
    // 21, on 40 and straddling 42 fire.
    let mut e = vec![];
    for (x, y) in [
        (2.0, 2.0),
        (5.0, 2.0),
        (18.825, 2.0),
        (19.0, 6.0),
        (20.5, 10.0),
        (39.0, 2.0),
        (41.5, 2.0),
    ] {
        e.push(rect(l.nw, x - 0.5, y - 0.5, x + 1.5, y + 1.5));
    }
    e.push(rect(l.nw, 7.5, 1.5, 8.5, 3.5));
    e.push(rect(l.nw, 8.5, 1.5, 9.5, 3.5));
    e.push(rect(l.psd, 1.82, 1.82, 2.7, 3.18));
    e.push(rect(l.psd, 2.5, 1.82, 3.175, 3.18));
    e.push(sq(l.act, 2.0, 2.0, 1.0));
    e.push(sq(l.act, 5.0, 2.0, 0.5));
    e.push(sq(l.act, 5.5, 2.0, 0.5));
    e.push(sq(l.act, 5.0, 2.5, 0.5));
    e.push(sq(l.act, 5.5, 2.5, 0.5));
    e.push(rect(l.psd, 4.82, 1.82, 6.175, 3.18));
    e.extend(l.psd_box(8.0, 2.0, 0.18, 0.18, 0.175, 0.18));
    e.extend(l.psd_box(18.825, 2.0, 0.18, 0.18, 0.175, 0.18));
    e.extend(l.psd_box(19.0, 6.0, 0.18, 0.18, 0.175, 0.18));
    e.extend(l.psd_box(20.5, 10.0, 0.18, 0.18, 0.175, 0.18));
    e.extend(l.psd_box(39.0, 2.0, 0.18, 0.18, 0.175, 0.18));
    e.extend(l.psd_box(41.5, 2.0, 0.18, 0.18, 0.175, 0.18));
    write("psd", "pSD.c.h4", e);

    // h7: a 300 µm P+Activ 0.175 from its pSD's top edge, and 0.175 at (1000, 1000).
    write(
        "psd",
        "pSD.c.h7",
        vec![
            rect(l.nw, 0.0, 0.0, 310.0, 10.0),
            rect(l.act, 2.0, 2.0, 302.0, 3.0),
            rect(l.psd, 1.82, 1.82, 302.18, 3.175),
            rect(l.nw, 995.0, 995.0, 1010.0, 1010.0),
            rect(l.act, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(l.psd, 999.825, 999.82, 1001.18, 1001.18),
        ],
    );
}

/// pSD.c1, "Min. pSD enclosure of P+Activ in PWell 0.03"; PWell is what is neither NWell
/// nor PWell:block (section 4.2).
fn psd_c1_h(l: &L) {
    // h1: 0.03 clean; 0.025 left fires; a chamfer 0.025 from the Activ's corner (walls
    // 0.03) fires; 0.01 under PWell:block is no PWell and clean; 0.025 with the pSD's edge
    // on x = 20, straddling 21 and on 40 fire.
    let mut e = l.psd_box(2.0, 2.0, 0.03, 0.03, 0.03, 0.03);
    e.extend(l.psd_box(5.0, 2.0, 0.025, 0.03, 0.03, 0.03));
    e.push(chamfered_tr(l.psd, 8.0, 2.0, 9.06, 3.06, 12.095));
    e.push(rect(l.act, 8.03, 2.03, 9.03, 3.03));
    e.push(rect(l.pwb, 11.0, 1.0, 14.0, 4.0));
    e.extend(l.psd_box(12.0, 2.0, 0.01, 0.01, 0.01, 0.01));
    e.extend(l.psd_box(18.975, 2.0, 0.03, 0.03, 0.025, 0.03));
    e.extend(l.psd_box(20.5, 2.0, 0.03, 0.03, 0.025, 0.03));
    e.extend(l.psd_box(39.0, 2.0, 0.03, 0.03, 0.025, 0.03));
    write("psd", "pSD.c1.h1", e);
}

/// pSD.d, "Min. pSD space to unrelated N+Activ in PWell 0.18"; N+Activ is Activ AND nSD,
/// nSD what is under neither pSD nor nSD:block or under drawn nSD (section 4.2), and
/// unrelated regions "do not touch each other" (section 4.1).
fn psd_d_h(l: &L) {
    // h1: 0.18 clean; 0.175 in x and y and 0.1768 corner to corner fire, 0.1838 corner to
    // corner is clean; an abutting Activ and one touching at a corner are related and
    // clean; an Activ under nSD:block, one under PWell:block and one in an NWell 0.175 away
    // are no N+Activ in PWell and clean; 0.175 to an Activ under drawn nSD and 0.175 at
    // (1000, 1000) fire.
    write(
        "psd",
        "pSD.d.h1",
        vec![
            sq(l.psd, 2.0, 2.0, 1.0),
            sq(l.act, 3.18, 2.0, 1.0),
            sq(l.psd, 6.0, 2.0, 1.0),
            sq(l.act, 7.175, 2.0, 1.0),
            sq(l.psd, 10.0, 2.0, 1.0),
            sq(l.act, 10.0, 3.175, 1.0),
            sq(l.psd, 14.0, 2.0, 1.0),
            sq(l.act, 15.125, 3.125, 1.0),
            sq(l.psd, 2.0, 6.0, 1.0),
            sq(l.act, 3.13, 7.13, 1.0),
            sq(l.psd, 6.0, 6.0, 1.0),
            sq(l.act, 7.0, 6.0, 1.0),
            sq(l.psd, 10.0, 6.0, 1.0),
            sq(l.act, 11.0, 7.0, 1.0),
            sq(l.psd, 14.0, 6.0, 1.0),
            rect(l.nsdb, 15.1, 5.8, 16.5, 7.2),
            sq(l.act, 15.175, 6.0, 1.0),
            sq(l.psd, 2.0, 10.0, 1.0),
            rect(l.pwb, 3.1, 9.8, 4.5, 11.2),
            sq(l.act, 3.175, 10.0, 1.0),
            rect(l.nw, 5.5, 9.5, 8.5, 11.5),
            sq(l.psd, 6.0, 10.0, 1.0),
            sq(l.act, 7.175, 10.0, 1.0),
            sq(l.psd, 10.0, 10.0, 1.0),
            sq(l.act, 11.175, 10.0, 1.0),
            sq(l.nsd, 11.175, 10.0, 1.0),
            sq(l.psd, 1000.0, 1000.0, 1.0),
            sq(l.act, 1001.175, 1000.0, 1.0),
        ],
    );

    // h2: an Activ crossing the well's edge, its PWell part 0.175 from a pSD, fires; an
    // abutted substrate tie's N+ part 0.175 from a second pSD fires; an L-shaped pSD
    // abutting an Activ along one edge and 0.175 from its other edge is related and clean.
    let e = vec![
        rect(l.nw, 2.0, 1.0, 4.0, 4.0),
        rect(l.act, 3.5, 2.0, 5.5, 3.0),
        sq(l.psd, 5.675, 2.0, 1.0),
        rect(l.act, 9.0, 2.0, 10.5, 3.0),
        rect(l.psd, 10.2, 1.9, 11.5, 3.1),
        rect(l.psd, 7.825, 1.9, 8.825, 3.1),
        sq(l.act, 14.0, 2.0, 1.0),
        rect(l.psd, 15.0, 1.5, 16.0, 3.5),
        rect(l.psd, 14.0, 3.175, 16.0, 4.175),
    ];
    write("psd", "pSD.d.h2", e);

    // h3: a pSD's chamfered corner 0.177 from an Activ's corner (the walls farther) and a
    // diamond Activ's corner 0.175 from a pSD's wall fire.
    write(
        "psd",
        "pSD.d.h3",
        vec![
            chamfered_tr(l.psd, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.act, 4.075, 3.675, 1.0),
            rect(l.psd, 7.0, 2.0, 8.325, 4.0),
            diamond(l.act, 9.0, 3.0, 0.5),
        ],
    );

    // h4: 0.175 gaps straddling x = 20, ending on it, starting on it, straddling 21, 40 and
    // 42.
    write(
        "psd",
        "pSD.d.h4",
        vec![
            sq(l.psd, 18.9, 2.0, 1.0),
            sq(l.act, 20.075, 2.0, 1.0),
            sq(l.psd, 18.825, 6.0, 1.0),
            sq(l.act, 20.0, 6.0, 1.0),
            sq(l.psd, 19.0, 10.0, 1.0),
            sq(l.act, 20.175, 10.0, 1.0),
            sq(l.psd, 19.9, 14.0, 1.0),
            sq(l.act, 21.075, 14.0, 1.0),
            sq(l.psd, 38.9, 2.0, 1.0),
            sq(l.act, 40.075, 2.0, 1.0),
            sq(l.psd, 40.9, 6.0, 1.0),
            sq(l.act, 42.075, 6.0, 1.0),
        ],
    );
}

/// pSD.d1, "Min. pSD space to N+Activ in NWell 0.03".
fn psd_d1(l: &L) {
    // h1 (the well under the first two rows): 0.03 clean; 0.025 in x and y and 0.0283
    // corner to corner fire; an abutting N+ tab is an abutted tie and clean; an Activ under
    // nSD:block is no N+Activ; an N+Activ under PWell:block outside the well 0.025 from a
    // pSD is in neither well and nothing (section 4.2's PWell).
    write(
        "psd",
        "pSD.d1.h1",
        vec![
            rect(l.nw, 0.0, 0.0, 17.0, 5.0),
            rect(l.nw, 0.0, 5.0, 9.0, 8.0),
            sq(l.psd, 2.0, 2.0, 1.0),
            sq(l.act, 3.03, 2.0, 1.0),
            sq(l.psd, 6.0, 2.0, 1.0),
            sq(l.act, 7.025, 2.0, 1.0),
            sq(l.psd, 10.0, 2.0, 1.0),
            sq(l.act, 10.0, 3.025, 1.0),
            sq(l.psd, 14.0, 2.0, 1.0),
            sq(l.act, 15.02, 3.02, 1.0),
            sq(l.psd, 2.0, 6.0, 1.0),
            sq(l.act, 3.0, 6.0, 1.0),
            sq(l.psd, 6.0, 6.0, 1.0),
            rect(l.nsdb, 7.01, 5.8, 8.5, 7.2),
            sq(l.act, 7.025, 6.0, 1.0),
            rect(l.pwb, 10.0, 5.5, 13.0, 7.5),
            sq(l.psd, 10.5, 6.0, 1.0),
            sq(l.act, 11.525, 6.0, 1.0),
        ],
    );

    // h2: 0.025 gaps straddling x = 20, 21 and 40 fire; a pSD's chamfered corner 0.028 from
    // an Activ's corner (the walls farther) fires.
    write(
        "psd",
        "pSD.d1.h2",
        vec![
            rect(l.nw, 0.0, 0.0, 46.0, 8.0),
            sq(l.psd, 18.98, 2.0, 1.0),
            sq(l.act, 20.005, 2.0, 1.0),
            sq(l.psd, 19.98, 6.0, 1.0),
            sq(l.act, 21.005, 6.0, 1.0),
            sq(l.psd, 38.98, 2.0, 1.0),
            sq(l.act, 40.005, 2.0, 1.0),
            chamfered_tr(l.psd, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.act, 3.97, 3.57, 1.0),
        ],
    );
}

/// pSD.e, "Min. pSD overlap of Activ at one position when forming abutted substrate tie
/// 0.30"; figure 5.10 draws `e` across the P+ part from the abutment line.
fn psd_e_h(l: &L) {
    // h1: an overlap of 0.295 fires, 0.30 and 0.305 are clean; an Activ drawn as two boxes
    // with 0.295 fires; 0.295 with the abutment line on x = 20 and one straddling 21 fire.
    let mut e = l.stie(2.0, 2.0, 0.295);
    e.extend(l.stie(6.0, 2.0, 0.30));
    e.extend(l.stie(10.0, 2.0, 0.305));
    e.push(rect(l.act, 2.0, 6.0, 3.0, 6.5));
    e.push(rect(l.act, 2.9, 6.0, 3.5, 6.5));
    e.push(rect(l.psd, 3.205, 5.9, 5.0, 6.6));
    e.push(rect(l.act, 18.5, 2.0, 20.0, 2.5));
    e.push(rect(l.psd, 19.705, 1.9, 21.5, 2.6));
    e.push(rect(l.act, 19.5, 6.0, 21.2, 6.5));
    e.push(rect(l.psd, 20.905, 5.9, 22.5, 6.6));
    write("psd", "pSD.e.h1", e);
}

/// pSD.f, "Min. Activ extension over pSD at one position when forming abutted NWell tie
/// 0.30".
fn psd_f_h(l: &L) {
    // h1 (one well under all): a tab 0.295 deep fires, 0.30 is clean; a tab drawn as two
    // boxes reaching 0.295 fires; a 0.295 tab under drawn nSD fires; 0.295 tabs with the
    // abutment line on x = 20, straddling 21 and on 40 fire.
    let mut e = vec![rect(l.nw, 0.0, 0.0, 46.0, 10.0)];
    e.extend(l.ntie_up(2.0, 2.0, 0.295, 0.5));
    e.extend(l.ntie_up(6.0, 2.0, 0.30, 0.5));
    e.extend(l.ntie_up(10.0, 2.0, 0.2, 0.5));
    e.push(rect(l.act, 10.75, 3.15, 11.25, 3.295));
    e.extend(l.ntie_up(14.0, 2.0, 0.295, 0.5));
    e.push(rect(l.nsd, 14.75, 3.0, 15.25, 3.295));
    e.push(rect(l.act, 18.0, 6.0, 20.0, 7.0));
    e.push(rect(l.psd, 17.8, 5.8, 20.0, 7.2));
    e.push(rect(l.act, 20.0, 6.25, 20.295, 6.75));
    e.push(rect(l.act, 19.5, 2.0, 20.9, 3.0));
    e.push(rect(l.psd, 19.3, 1.8, 20.9, 3.2));
    e.push(rect(l.act, 20.9, 2.25, 21.195, 2.75));
    e.push(rect(l.act, 38.0, 2.0, 40.0, 3.0));
    e.push(rect(l.psd, 37.8, 1.8, 40.0, 3.2));
    e.push(rect(l.act, 40.0, 2.25, 40.295, 2.75));
    write("psd", "pSD.f.h1", e);
}

/// pSD.g, "Min. N+Activ or P+Activ area (µm²) when forming abutted tie 0.09".
fn psd_g_h(l: &L) {
    // h1 (a well under the second row): a substrate tie's P+ part 0.30 × 0.30 is clean,
    // 0.30 × 0.295 fires; an NWell tie's N+ tab 0.30 × 0.30 is clean, 0.295 × 0.30 fires;
    // a standalone 0.2 × 0.3 N+ tap in the well and a standalone P+ tap form no abutted tie
    // (the letter: nothing; Act.d has them); 0.0885 parts with the abutment on x = 20 and
    // straddling 21 fire.
    write(
        "psd",
        "pSD.g.h1",
        vec![
            rect(l.nw, 0.0, 5.0, 24.0, 9.0),
            rect(l.act, 2.0, 2.0, 3.5, 2.3),
            rect(l.psd, 3.2, 1.9, 5.0, 2.4),
            rect(l.act, 6.0, 2.0, 7.5, 2.295),
            rect(l.psd, 7.2, 1.9, 9.0, 2.4),
            rect(l.act, 2.0, 6.0, 4.0, 7.0),
            rect(l.psd, 1.8, 5.8, 4.2, 7.0),
            rect(l.act, 2.85, 7.0, 3.15, 7.3),
            rect(l.act, 6.0, 6.0, 8.0, 7.0),
            rect(l.psd, 5.8, 5.8, 8.2, 7.0),
            rect(l.act, 6.85, 7.0, 7.145, 7.3),
            rect(l.act, 10.0, 6.0, 10.2, 6.3),
            rect(l.act, 10.0, 2.0, 10.2, 2.3),
            rect(l.psd, 9.8, 1.8, 10.4, 2.5),
            rect(l.act, 18.5, 2.0, 20.0, 2.295),
            rect(l.psd, 19.7, 1.9, 21.5, 2.4),
            rect(l.act, 19.5, 6.0, 21.5, 7.0),
            rect(l.psd, 19.3, 5.8, 21.7, 7.0),
            rect(l.act, 20.85, 7.0, 21.145, 7.3),
        ],
    );
}

/// pSD.i and pSD.i1, "Min. pSD enclosure of PFET gate not inside / inside ThickGateOx
/// 0.30 / 0.40", in every direction: IHP's own inverter keeps its pSD 0.315 past the
/// PFET's Activ, where the gate's width edge lies (figure 5.10's `c` is drawn at 0.18
/// there, but a figure).
fn psd_i(l: &L) {
    // h1 (one well under all): 0.30 either side and 0.30 past the Activ in the width
    // direction is clean; 0.295 left, right, both, and 0.295 in the width direction fire;
    // a 3.3 V PFET with 0.395 on the left (its pSD inside the oxide), one whose pSD reaches
    // out of the oxide with 0.395 on the right, and one with 0.395 in the width direction
    // fire pSD.i1.
    let mut e = vec![rect(l.nw, 0.0, 0.0, 24.0, 13.0)];
    e.extend(l.pfet(2.0, 2.0, 0.12, 0.12, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(6.0, 2.0, 0.115, 0.12, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(10.0, 2.0, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(14.0, 2.0, 0.115, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(2.0, 6.0, 0.12, 0.12, 0.2, 0.5, 0.18, 0.18, 0.295));
    e.extend(l.pfet(6.0, 6.0, 0.215, 0.22, 0.45, 0.5, 0.18, 0.18, 0.40));
    e.push(rect(l.tgo, 5.5, 5.4, 7.4, 7.6));
    e.extend(l.pfet(12.0, 6.0, 0.22, 0.215, 0.45, 0.5, 1.5, 0.18, 0.40));
    e.push(rect(l.tgo, 11.5, 5.4, 13.4, 7.6));
    e.extend(l.pfet(2.0, 10.0, 0.22, 0.22, 0.45, 0.5, 0.18, 0.18, 0.395));
    e.push(rect(l.tgo, 1.5, 9.4, 3.4, 11.6));
    write("psd", "pSD.i.h1", e);

    // h2 (one well under all): a pSD's chamfered corner 0.293 from the gate's corner (the
    // walls 0.30 and 0.30, the Activ's corner 0.21) fires; a pSD and an Activ each drawn as
    // two boxes with 0.295 fire; 0.295 with the pSD's edge on x = 20, straddling 21, on 40
    // and straddling 42 fire.
    let mut e = vec![
        rect(l.nw, 0.0, 0.0, 46.0, 12.0),
        rect(l.act, 2.0, 2.0, 2.44, 3.0),
        rect(l.gp, 2.12, 1.5, 2.32, 3.5),
        chamfered_tr(l.psd, 1.82, 1.7, 2.62, 3.3, 5.735),
        rect(l.psd, 5.82, 1.7, 6.4, 3.3),
        rect(l.psd, 6.3, 1.7, 6.615, 3.3),
        rect(l.act, 6.0, 2.0, 6.2, 3.0),
        rect(l.act, 6.2, 2.0, 6.435, 3.0),
        rect(l.gp, 6.12, 1.5, 6.32, 3.5),
    ];
    e.extend(l.pfet(19.385, 2.0, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(20.5, 6.0, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(39.385, 2.0, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    e.extend(l.pfet(41.5, 6.0, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30));
    write("psd", "pSD.i.h2", e);
}

/// pSD.j and pSD.j1, "Min. pSD space to NFET gate not inside / inside ThickGateOx
/// 0.30 / 0.40"; figure 5.10 draws `j` from the gate's side along the Activ and from the
/// poly's end past the Activ.
fn psd_j(l: &L) {
    // h1: a pSD abutting the Activ's end 0.295 from the gate's side fires, 0.30 is clean; a
    // pSD 0.295 below the poly's end (0.475 below the Activ) fires as the figure draws
    // `j`; a pSD 0.295 below the Activ over the poly's cap and one 0.295 below the Activ
    // with the cap 0.18 (0.115 from the poly's end) fire; an Activ under nSD:block makes no
    // NFET and is clean; an N-gate in an NWell is an NFET by section 4.2 and fires; a 3.3 V
    // NFET with 0.395 fires pSD.j1.
    let mut e = l.nfet(2.0, 2.0, 0.5, 0.295, 0.2, 0.3);
    e.push(rect(l.psd, 2.995, 1.5, 4.0, 3.5));
    e.extend(l.nfet(6.0, 2.0, 0.5, 0.30, 0.2, 0.3));
    e.push(rect(l.psd, 7.0, 1.5, 8.0, 3.5));
    e.extend(l.nfet(10.0, 2.0, 0.5, 0.5, 0.2, 0.18));
    e.push(rect(l.psd, 10.0, 0.5, 11.2, 1.525));
    e.extend(l.nfet(14.0, 2.0, 0.5, 0.5, 0.2, 0.3));
    e.push(rect(l.psd, 14.0, 0.5, 15.2, 1.705));
    e.extend(l.nfet(2.0, 6.0, 0.5, 0.5, 0.2, 0.18));
    e.push(rect(l.psd, 2.0, 4.5, 3.2, 5.705));
    e.extend(l.nfet(6.0, 6.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.nsdb, 5.9, 5.9, 7.1, 7.1));
    e.push(rect(l.psd, 6.995, 5.5, 8.0, 7.5));
    e.push(rect(l.nw, 9.5, 5.0, 12.0, 8.5));
    e.extend(l.nfet(10.0, 6.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.psd, 10.995, 5.5, 12.0, 7.5));
    e.extend(l.nfet(14.0, 6.0, 0.5, 0.395, 0.45, 0.3));
    e.push(rect(l.tgo, 13.73, 5.73, 15.615, 7.27));
    e.push(rect(l.psd, 15.345, 5.5, 16.5, 7.5));
    write("psd", "pSD.j.h1", e);

    // h2: a 45° pSD wall 0.293 from the gate's corner (0.18 from the Activ's) fires; a
    // diamond pSD's corner 0.295 above the gate's width edge fires; 0.295 with the pSD's
    // edge on x = 20, straddling 21, on 40 and straddling 42 fire.
    let mut e = l.nfet(2.0, 2.0, 0.5, 0.16, 0.2, 0.18);
    e.push(chamfered_bl(l.psd, 2.5, 3.0, 4.5, 5.0, 6.115));
    e.extend(l.nfet(6.0, 2.0, 0.5, 0.5, 0.2, 0.18));
    e.push(diamond(l.psd, 6.6, 3.895, 0.6));
    e.extend(l.nfet(19.005, 2.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.psd, 20.0, 1.5, 21.0, 3.5));
    e.extend(l.nfet(20.2, 6.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.psd, 21.195, 5.5, 22.2, 7.5));
    e.extend(l.nfet(39.005, 2.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.psd, 40.0, 1.5, 41.0, 3.5));
    e.extend(l.nfet(41.2, 6.0, 0.5, 0.295, 0.2, 0.3));
    e.push(rect(l.psd, 42.195, 5.5, 43.2, 7.5));
    write("psd", "pSD.j.h2", e);

    // h3/h4: fifty 0.295 NFETs, flat and as an array.
    let mut cell = l.nfet(0.0, 0.5, 0.5, 0.295, 0.2, 0.3);
    cell.push(rect(l.psd, 0.995, 0.0, 2.0, 2.0));
}

/// pSD.l, "Min. pSD enclosed area (µm²) 0.25"; figure 5.10 draws `l` in a hole holding an
/// island as the empty area between.
fn psd_l_h(l: &L) {
    // h1: a 0.25 hole is clean, 0.2475 fires; a 0.49 hole holding a 0.25 island (0.24
    // empty; the island's 0.1 gaps are pSD.b, set aside) fires; an L-shaped hole of 0.24
    // fires; 0.2475 holes straddling x = 20 and 21 and at (1000, 1000) fire.
    let mut e = ring(l.psd, 2.0, 2.0, 3.5, 3.5, 2.5, 2.5, 3.0, 3.0);
    e.extend(ring(l.psd, 5.0, 2.0, 6.5, 3.5, 5.5, 2.5, 6.0, 2.995));
    e.extend(ring(l.psd, 8.0, 2.0, 9.7, 3.7, 8.5, 2.5, 9.2, 3.2));
    e.push(rect(l.psd, 8.6, 2.6, 9.1, 3.1));
    e.push(rect(l.psd, 11.0, 2.0, 12.5, 2.5));
    e.push(rect(l.psd, 11.0, 3.1, 12.5, 3.5));
    e.push(rect(l.psd, 11.0, 2.5, 11.5, 3.1));
    e.push(rect(l.psd, 12.0, 2.5, 12.5, 2.8));
    e.push(rect(l.psd, 11.8, 2.8, 12.5, 3.1));
    e.extend(ring(
        l.psd, 19.25, 2.0, 20.75, 3.5, 19.75, 2.5, 20.25, 2.995,
    ));
    e.extend(ring(
        l.psd, 20.25, 6.0, 21.75, 7.5, 20.75, 6.5, 21.25, 6.995,
    ));
    e.extend(ring(
        l.psd, 1000.0, 1000.0, 1001.5, 1001.5, 1000.5, 1000.5, 1001.0, 1000.995,
    ));
    write("psd", "pSD.l.h1", e);
}

/// pSD.m, "Min. pSD space to n-type poly resistors 0.18", and pSD.n, "Min. pSD enclosure
/// of p-type poly resistors 0.18" (the resistor deck's recognition: Rsil is GatPoly under
/// RES in an EXTBlock, Rppd is GatPoly under pSD and SalBlock).
fn psd_mn(l: &L) {
    // m.h1: a pSD 0.175 from an Rsil's poly fires, 0.18 is clean.
    write(
        "psd",
        "pSD.m.h1",
        vec![
            rect(l.gp, 2.0, 2.0, 2.5, 5.0),
            rect(l.res, 2.0, 2.0, 2.5, 5.0),
            rect(l.ext, 1.82, 1.82, 2.68, 5.18),
            rect(l.psd, 2.675, 2.0, 3.675, 5.0),
            rect(l.gp, 6.0, 2.0, 6.5, 5.0),
            rect(l.res, 6.0, 2.0, 6.5, 5.0),
            rect(l.ext, 5.82, 1.82, 6.68, 5.18),
            rect(l.psd, 6.68, 2.0, 7.68, 5.0),
        ],
    );

    // n.h1: an Rppd's pSD 0.18 past the body all round is clean, 0.175 on the left fires.
    write(
        "psd",
        "pSD.n.h1",
        vec![
            rect(l.gp, 2.0, 2.0, 2.5, 5.0),
            rect(l.sal, 2.0, 2.0, 2.5, 5.0),
            rect(l.psd, 1.82, 1.82, 2.68, 5.18),
            rect(l.ext, 1.5, 1.5, 3.0, 5.5),
            rect(l.gp, 6.0, 2.0, 6.5, 5.0),
            rect(l.sal, 6.0, 2.0, 6.5, 5.0),
            rect(l.psd, 5.825, 1.82, 6.68, 5.18),
            rect(l.ext, 5.5, 1.5, 7.0, 5.5),
        ],
    );
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/psd")
        .expect("failed to create output directory");
    let l = L::new(pdk);
    width_suite("psd", "pSD.a", l.psd, 0.31);
    space_suite("psd", "pSD.b", l.psd, 0.31, 1.0);
    psd_c(&l);
    psd_c1_h(&l);
    psd_d_h(&l);
    psd_d1(&l);
    psd_e_h(&l);
    psd_f_h(&l);
    psd_g_h(&l);
    psd_i(&l);
    psd_j(&l);
    psd_l_h(&l);
    psd_mn(&l);
}
