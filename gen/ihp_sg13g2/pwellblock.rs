// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{
    chamfered_tr, diamond, layer, library, min_width_pattern, notch_pattern, poly, rect,
    space_pattern, strip45, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/pwellblock";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pwb_a(pdk);
    pwb_b_space(pdk);
    pwb_b_notch(pdk);
    pwb_c(pdk);
    pwb_e(pdk);
    pwb_e1(pdk);
    pwb_f(pdk);
    pwb_f1(pdk);

    hardening(pdk);
}

/// An implanted Activ footprint (Activ ∩ `imp`) at `(x, y)` — N+Activ with nSD or
/// P+Activ with pSD.
fn impl_activ(
    activ: (i16, i16),
    imp: (i16, i16),
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Vec<GdsElement> {
    vec![
        rect(activ, x, y, x + w, y + h),
        rect(imp, x, y, x + w, y + h),
    ]
}

/// PWell:block-to-(implanted Activ "in PWell") spacing fixture: a clean pair at exactly
/// `value` and a violating pair at `value - 0.01`.  `tgo` (if set) covers the Activ.
fn pwb_space_to_activ(pdk: &PdkConfig, name: &str, imp_name: &str, value: f64, tgo: bool) {
    let blk = layer(pdk, "PWell.block");
    let activ = layer(pdk, "Activ");
    let imp = layer(pdk, imp_name);
    let o = OFFSET;
    let mut elems = vec![rect(blk, o, o, o + 1.0, o + 1.0)];
    let x1 = o + 1.0 + value;
    elems.extend(impl_activ(activ, imp, x1, o, 0.5, 0.5)); // gap = value → clean
    elems.push(rect(blk, o + 4.0, o, o + 5.0, o + 1.0));
    let x2 = o + 5.0 + value - 0.01;
    elems.extend(impl_activ(activ, imp, x2, o, 0.5, 0.5)); // gap = value-0.01 → violation
    if tgo {
        let t = layer(pdk, "ThickGateOx");
        elems.push(rect(t, x1 - 0.1, o - 0.1, x1 + 0.6, o + 0.6));
        elems.push(rect(t, x2 - 0.1, o - 0.1, x2 + 0.6, o + 0.6));
    }
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// PWB.e — min. PWell:block space to N+Activ (in PWell), not in ThickGateOx, 0.31 µm.
fn pwb_e(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.e", "nSD", 0.31, false);
}

/// PWB.e1 — same, inside ThickGateOx, 0.62 µm.
fn pwb_e1(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.e1", "nSD", 0.62, true);
}

/// PWB.f — min. PWell:block space to P+Activ (in PWell), not in ThickGateOx, 0.24 µm.
fn pwb_f(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.f", "pSD", 0.24, false);
}

/// PWB.f1 — same, inside ThickGateOx, 0.62 µm.
fn pwb_f1(pdk: &PdkConfig) {
    pwb_space_to_activ(pdk, "PWB.f1", "pSD", 0.62, true);
}

/// PWB.a — min. PWell:block width 0.62 µm.
fn pwb_a(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = min_width_pattern(l, 0.62, 0.62, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.a.gds.gz"), library("TOP", elems));
}

/// PWB.b — min. PWell:block space 0.62 µm.  1 µm shapes clear the 0.62 µm min width.
fn pwb_b_space(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = space_pattern(l, l, 1.0, 0.62, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.b.space.gds.gz"), library("TOP", elems));
}

/// PWB.b — min. PWell:block notch 0.62 µm.
fn pwb_b_notch(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let elems = notch_pattern(l, 1.0, 0.62, 2.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.b.notch.gds.gz"), library("TOP", elems));
}

/// PWB.c — min. PWell:block space to NWell 0.62 µm.  (Overlap with NWell is allowed —
/// PWB.d — and min_space skips overlapping pairs, so only true gaps are measured.)
fn pwb_c(pdk: &PdkConfig) {
    let l = layer(pdk, "PWell.block");
    let nw = layer(pdk, "NWell");
    let elems = space_pattern(l, nw, 1.0, 0.62, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/PWB.c.gds.gz"), library("TOP", elems));
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

/// One grid step.
pub(super) const G: f64 = 0.005;

/// A layer drawn as any polygon.
pub(super) type Free<'a> = dyn Fn(&[(f64, f64)]) -> Vec<GdsElement> + 'a;

/// A layer drawn as boxes, with whatever must go with them.
pub(super) type Boxes<'a> = dyn Fn(f64, f64, f64, f64) -> Vec<GdsElement> + 'a;

pub(super) const SQRT2: f64 = std::f64::consts::SQRT_2;

pub(super) fn grid(v: f64) -> f64 {
    (v / G).round() * G
}

/// The largest grid multiple `a` with `a·√2` under `s`: a diagonal gap or a diamond's
/// half-diagonal one step under the value.
pub(super) fn diag_under(s: f64) -> f64 {
    let mut a = grid((s / SQRT2 / G).floor() * G);
    while a * SQRT2 >= s - 1e-9 {
        a -= G;
    }
    a
}

/// The smallest grid multiple `a` with `a·√2` at or over `s`.
pub(super) fn diag_over(s: f64) -> f64 {
    diag_under(s) + G
}

/// Every layer the six decks draw.
pub(super) struct P {
    pub(super) activ: (i16, i16),
    pub(super) gp: (i16, i16),
    pub(super) cont: (i16, i16),
    pub(super) m1: (i16, i16),
    pub(super) psd: (i16, i16),
    pub(super) nsd: (i16, i16),
    pub(super) nsdb: (i16, i16),
    pub(super) nw: (i16, i16),
    pub(super) pwb: (i16, i16),
    pub(super) nbl: (i16, i16),
    pub(super) nblb: (i16, i16),
    pub(super) extb: (i16, i16),
    pub(super) sal: (i16, i16),
    pub(super) tgo: (i16, i16),
}

impl P {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        P {
            activ: layer(pdk, "Activ"),
            gp: layer(pdk, "GatPoly"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            nsdb: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            pwb: layer(pdk, "PWell.block"),
            nbl: layer(pdk, "nBuLay"),
            nblb: layer(pdk, "nBuLay.block"),
            extb: layer(pdk, "EXTBlock"),
            sal: layer(pdk, "SalBlock"),
            tgo: layer(pdk, "ThickGateOx"),
        }
    }

    pub(super) fn write(&self, deck: &str, name: &str, elems: Vec<GdsElement>) {
        write_gz(
            &format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz"),
            library("TOP", elems),
        );
    }
}

/// A rectangular frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, drawn as
/// four abutting boxes that merge into one ring.
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

/// Min. width `w` of layer `l` (`s`, the largest space the deck asks of the layer, keeps
/// the sub-patterns apart): `<rule>.h1` the bound and the 45° shapes, `.h2` shapes that
/// merge.  The tile lines are the engine's (`gen/engine/width.rs`).
pub(super) fn width_kit(p: &P, deck: &str, rule: &str, l: (i16, i16), w: f64, s: f64) {
    let d = w - G;
    let len = grid(3.0 * w).max(1.0);
    let gap = s.max(w) + 0.5;
    let av = diag_under(w);
    let ac = diag_over(w);

    // h1 - the bound.  A w square is clean; a bar d wide (in x, in y) fires on its two
    // long walls; a diamond and a 45° strip of width av·√2 (under w) fire, of ac·√2 (over)
    // are clean; an L of d arms fires; a box w + 0.2 wide whose corner is chamfered by
    // 0.2 is w from wall to chamfer end and clean.
    let y = 2.0;
    let mut x = 2.0;
    let mut e = vec![rect(l, x, y, x + w, y + w)];
    x += w + gap;
    e.push(rect(l, x, y, x + d, y + len));
    x += d + gap;
    e.push(rect(l, x, y, x + len, y + d));
    x += len + gap;
    e.push(diamond(l, x + ac, y + ac, av));
    x += 2.0 * ac + gap;
    e.push(diamond(l, x + ac, y + ac, ac));
    x += 2.0 * ac + gap;
    e.push(strip45(l, x + av, y, len, av));
    x += len + av + gap;
    e.push(strip45(l, x + ac, y, len, ac));
    x += len + ac + gap;
    e.push(poly(
        l,
        &[
            (x, y),
            (x + len, y),
            (x + len, y + d),
            (x + d, y + d),
            (x + d, y + len),
            (x, y + len),
        ],
    ));
    x += len + gap;
    e.push(chamfered_tr(
        l,
        x,
        y,
        x + w + 0.2,
        y + len,
        x + w + 0.2 + y + len - 0.2,
    ));
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - shapes that merge.  Two boxes w - 0.1 wide overlapping by 0.1 are a w bar
    // (clean); overlapping by 0.095 they are d wide (fires); two abutting halves and four
    // quadrants are a w square; a d bar drawn as three boxes along its length fires once
    // (two walls); a frame whose left side is d thick fires on that side; a w square drawn
    // clockwise is clean; a w square in a w-thick ring's hole is clean; a 0.005 sliver
    // fires.
    let mut x = 2.0;
    let mut e = vec![
        rect(l, x, y, x + w - 0.1, y + len),
        rect(l, x + 0.1, y, x + w, y + len),
    ];
    x += w + gap;
    e.push(rect(l, x, y, x + w - 0.1, y + len));
    e.push(rect(l, x + 0.095, y, x + w - G, y + len));
    x += w + gap;
    let h = grid(w / 2.0);
    e.push(rect(l, x, y, x + h, y + w));
    e.push(rect(l, x + h, y, x + w, y + w));
    x += w + gap;
    e.push(rect(l, x, y, x + h, y + h));
    e.push(rect(l, x + h, y, x + w, y + h));
    e.push(rect(l, x, y + h, x + h, y + w));
    e.push(rect(l, x + h, y + h, x + w, y + w));
    x += w + gap;
    for k in 0..3 {
        e.push(rect(l, x, y + k as f64 * w, x + d, y + (k + 1) as f64 * w));
    }
    x += d + gap;
    let t = w + 0.1;
    let o = grid(2.0 * t + s.max(w) + 0.5);
    e.extend(ring(
        l,
        x,
        y,
        x + o,
        y + o,
        x + d,
        y + t,
        x + o - t,
        y + o - t,
    ));
    x += o + gap;
    e.push(poly(l, &[(x, y), (x, y + w), (x + w, y + w), (x + w, y)]));
    x += w + gap;
    let hole = w + 2.0 * (s.max(w) + 0.1);
    let o = hole + 2.0 * w;
    e.extend(ring(
        l,
        x,
        y,
        x + o,
        y + o,
        x + w,
        y + w,
        x + w + hole,
        y + w + hole,
    ));
    let c = x + o / 2.0;
    e.push(rect(
        l,
        c - w / 2.0,
        y + o / 2.0 - w / 2.0,
        c + w / 2.0,
        y + o / 2.0 + w / 2.0,
    ));
    x += o + gap;
    e.push(rect(l, x, y, x + G, y + len));
    p.write(deck, &format!("{rule}.h2"), e);
}

/// A two-layer space rule of value `s` between a `free` layer (drawn as any polygon: the
/// block or well) and a `fixed` one (drawn as boxes `qf` across, with whatever must go
/// with them): `<rule>.h1` the bound, both metrics, the 45° shapes of the free layer and
/// (when `touch`) the abutting pair, `.h2` the tile lines, a 300 µm run and (1000, 1000),
/// `.h3`/`.h4` fifty flat and as an array.
#[allow(clippy::too_many_arguments)]
pub(super) fn space2_kit(
    p: &P,
    deck: &str,
    rule: &str,
    s: f64,
    free: &Free<'_>,
    q: f64,
    fixed: &Boxes<'_>,
    qf: f64,
    touch: bool,
) {
    let d = s - G;
    let gap = s + q.max(qf) + 1.0;
    let av = diag_under(s);
    let ac = diag_over(s);
    let fbox = |x: f64, y: f64| free(&[(x, y), (x + q, y), (x + q, y + q), (x, y + q)]);
    let xbox = |x: f64, y: f64| fixed(x, y, x + qf, y + qf);

    // h1.  Fixed box right of the free box at s (clean) and d (fires); corner to corner
    // at av/av (fires) and ac/ac (clean); 0.1 in x and s in y (clean); the free layer as a
    // diamond whose tip is d (fires) and s (clean) from the fixed box; a chamfer of the
    // free box av·√2 (fires) and ac·√2 (clean) from the fixed box's corner; the fixed box
    // abutting the free box (a space of nothing).
    let y = 2.0;
    let mut x = 2.0;
    let mut e = vec![];
    for g in [s, d] {
        e.extend(fbox(x, y));
        e.extend(xbox(x + q + g, y));
        x += q + g + qf + gap;
    }
    for a in [av, ac] {
        e.extend(fbox(x, y));
        e.extend(xbox(x + q + a, y + q + a));
        x += q + a + qf + gap;
    }
    e.extend(fbox(x, y));
    e.extend(xbox(x + q + 0.1, y + q + s));
    x += q + qf + gap;
    for g in [d, s] {
        let a = q / 2.0;
        e.extend(free(&[
            (x, y + a),
            (x + a, y),
            (x + 2.0 * a, y + a),
            (x + a, y + 2.0 * a),
        ]));
        e.extend(xbox(x + 2.0 * a + g, y + a - qf / 2.0));
        x += q + g + qf + gap;
    }
    for a in [av, ac] {
        let c = 2.0 * a + 0.1;
        let qa = q + c;
        e.extend(free(&[
            (x, y),
            (x + qa, y),
            (x + qa, y + qa - c),
            (x + qa - c, y + qa),
            (x, y + qa),
        ]));
        e.extend(xbox(x + qa - 0.05, y + qa - 0.05));
        x += qa + qf + gap;
    }
    if touch {
        e.extend(fbox(x, y));
        e.extend(xbox(x + q, y));
    }
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - tile lines.  d gaps straddling x = 20 (over the line, starting on it, ending
    // on it), straddling 21, 40, 42, y = 20 at x = xy, a corner on (xc, 20), a 300 µm
    // free strip d from a fixed box at y = 60, one pair at (1000, 1000).
    let row = q.max(qf) + gap;
    let xy = 30f64.max(grid(20.0 + d + q.max(qf) + gap));
    let xc = 60f64.max(((42.0 + d + 2.0 * q.max(qf) + gap) / 20.0).ceil() * 20.0);
    let mut e = vec![];
    e.extend(fbox(19.9 - q, 2.0));
    e.extend(xbox(19.9 + d, 2.0));
    e.extend(fbox(20.0 - q, 2.0 + row));
    e.extend(xbox(20.0 + d, 2.0 + row));
    e.extend(fbox(20.0 - d - q, 2.0 + 2.0 * row));
    e.extend(xbox(20.0, 2.0 + 2.0 * row));
    e.extend(fbox(20.9 - q, 2.0 + 3.0 * row));
    e.extend(xbox(20.9 + d, 2.0 + 3.0 * row));
    e.extend(fbox(39.9 - q, 2.0));
    e.extend(xbox(39.9 + d, 2.0));
    e.extend(fbox(41.9 - q, 2.0 + row));
    e.extend(xbox(41.9 + d, 2.0 + row));
    e.extend(fbox(xy, 19.9 - q));
    e.extend(xbox(xy, 19.9 + d));
    e.extend(fbox(xc - q, 20.0 - q));
    e.extend(xbox(xc + av, 20.0 + av));
    e.extend(free(&[
        (2.0, 60.0),
        (302.0, 60.0),
        (302.0, 60.0 + q),
        (2.0, 60.0 + q),
    ]));
    e.extend(xbox(150.0, 60.0 + q + d));
    e.extend(fbox(1000.0, 1000.0));
    e.extend(xbox(1000.0 + q + d, 1000.0));
    p.write(deck, &format!("{rule}.h2"), e);

    // h3/h4 - fifty d pairs, flat and as an array.
    let mut cell = fbox(0.2, 0.2);
    cell.extend(xbox(0.2 + q + d, 0.2));
}

/// A min. enclosure `v` of the `inner` layer (drawn as boxes `qi` across, with whatever
/// must go with them) by the `outer` one (any polygon; `clear` keeps its shapes apart):
/// `<rule>.h1` the bound on each
/// side, the chamfer, the crossing and the coincident edge, `.h2` the tile lines, a 300 µm
/// strip and (1000, 1000), `.h3`/`.h4` fifty flat and as an array.
#[allow(clippy::too_many_arguments)]
pub(super) fn enclosure_kit(
    p: &P,
    deck: &str,
    rule: &str,
    v: f64,
    outer: &Free<'_>,
    inner: &Boxes<'_>,
    qi: f64,
    clear: f64,
) {
    let d = v - G;
    let m = v + 0.5;
    let gap = clear + 0.5;
    let av = diag_under(v);
    let ac = diag_over(v);
    let obox =
        |x0: f64, y0: f64, x1: f64, y1: f64| outer(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
    // The inner box with margins (l, r, b, t) inside an outer box at (x, y).
    let pair = |x: f64, y: f64, l: f64, r: f64, b: f64, t: f64| {
        let mut e = obox(x, y, x + l + qi + r, y + b + qi + t);
        e.extend(inner(x + l, y + b, x + l + qi, y + b + qi));
        e
    };

    // h1.  Margins v all round (clean); d on the left, right, bottom, top (one each); d
    // all round (one: the walls are one run); the outer's corner chamfered av·√2 from the
    // inner's corner with both axis margins m (fires) and ac·√2 (clean); the inner box
    // half out of the outer (enclosed by nothing on that side: fires); the inner's right
    // edge on the outer's right edge (0: fires).
    let y = 2.0;
    let mut x = 2.0;
    let mut e = pair(x, y, v, v, v, v);
    x += qi + 2.0 * v + gap;
    for (l, r, b, t) in [
        (d, v, v, v),
        (v, d, v, v),
        (v, v, d, v),
        (v, v, v, d),
        (d, d, d, d),
    ] {
        e.extend(pair(x, y, l, r, b, t));
        x += qi + l + r + gap;
    }
    for a in [av, ac] {
        let o = qi + 2.0 * m;
        let c = 2.0 * m - 2.0 * a;
        e.extend(outer(&[
            (x, y),
            (x + o, y),
            (x + o, y + o - c),
            (x + o - c, y + o),
            (x, y + o),
        ]));
        e.extend(inner(x + m, y + m, x + m + qi, y + m + qi));
        x += o + gap;
    }
    let ox = qi + 2.0 * m;
    e.extend(obox(x, y, x + ox, y + ox));
    e.extend(inner(
        x + ox - qi / 2.0,
        y + m,
        x + ox + qi / 2.0,
        y + m + qi,
    ));
    x += ox + qi + gap;
    e.extend(pair(x, y, v, 0.0, v, v));
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - tile lines.  A d right margin straddling x = 20 (inner edge at 19.9), ending
    // on 20 (outer edge on the line), starting on 20 (inner edge on the line), straddling
    // 21, 40, 42; a d top margin straddling y = 20 at x = 30; a corner d/d on (60, 20); an
    // inner d from the top of a 300 µm strip at y = 60; one at (1000, 1000); a v margin
    // straddling x = 20 is clean.
    let row = qi + 2.0 * v + gap;
    let mut e = vec![];
    e.extend(pair(19.9 - v - qi, 2.0, v, d, v, v));
    e.extend(pair(20.0 - d - v - qi, 2.0 + row, v, d, v, v));
    e.extend(pair(20.0 - v - qi, 2.0 + 2.0 * row, v, d, v, v));
    e.extend(pair(20.9 - v - qi, 2.0 + 3.0 * row, v, d, v, v));
    e.extend(pair(39.9 - v - qi, 2.0, v, d, v, v));
    e.extend(pair(41.9 - v - qi, 2.0 + row, v, d, v, v));
    e.extend(pair(30.0, 19.9 - v - qi, v, v, v, d));
    e.extend(pair(60.0 - v - qi, 20.0 - v - qi, v, d, v, d));
    e.extend(obox(2.0, 60.0, 302.0, 60.0 + qi + 2.0 * v));
    e.extend(inner(150.0, 60.0 + v + G, 150.0 + qi, 60.0 + v + qi + G));
    e.extend(pair(1000.0, 1000.0, v, d, v, v));
    e.extend(pair(19.9 - v - qi, 2.0 + 4.0 * row, v, v, v, v));
    p.write(deck, &format!("{rule}.h2"), e);
}

fn pwellblock(p: &P) {
    let deck = "pwellblock";
    let b = p.pwb;
    width_kit(p, deck, "PWB.a", b, 0.62, 0.62);

    // PWB.c - min. PWell:block space to NWell 0.62; PWB.d - overlap is allowed.  The kit
    // with the well as the fixed box; its abutting pair is legal (PWB.d).
    let nwell = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.nw, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "PWB.c",
        0.62,
        &|pts| vec![poly(b, pts)],
        1.24,
        &nwell,
        1.24,
        true,
    );

    // PWB.c.h5 - the relation.  A well overlapping the block (clean, PWB.d), a well
    // inside the block and a block inside a well (clean); figure 5.2's well: a U whose
    // lower arm and connector overlap the block and whose upper arm is 0.615 above the
    // block's top edge (fires; the same at 0.62 is clean); a well ring around the block
    // 0.615 from its right inner wall and 1.0 from the others (fires once).
    let mut e = vec![
        rect(b, 2.0, 2.0, 4.0, 4.0),
        rect(p.nw, 3.0, 3.0, 5.0, 5.0),
        rect(b, 8.0, 2.0, 11.0, 5.0),
        rect(p.nw, 9.0, 3.0, 10.0, 4.0),
        rect(p.nw, 14.0, 2.0, 17.0, 5.0),
        rect(b, 15.0, 3.0, 16.0, 4.0),
    ];
    for (x, g) in [(2.0, 0.615), (10.0, 0.62)] {
        e.push(rect(b, x, 8.0, x + 3.0, 10.0));
        e.push(poly(
            p.nw,
            &[
                (x + 0.5, 8.5),
                (x + 2.7, 8.5),
                (x + 2.7, 11.0 + g),
                (x + 0.5, 11.0 + g),
                (x + 0.5, 10.0 + g),
                (x + 2.0, 10.0 + g),
                (x + 2.0, 9.2),
                (x + 0.5, 9.2),
            ],
        ));
    }
    e.push(rect(b, 28.0, 8.0, 30.0, 10.0));
    e.extend(ring(p.nw, 26.0, 6.0, 31.615, 12.0, 27.0, 7.0, 30.615, 11.0));
    p.write(deck, "PWB.c.h5", e);

    // PWB.e - min. PWell:block space to N+Activ (not in ThickGateOx) in PWell 0.31.  The
    // kit with a 0.5 Activ square as the fixed box: plain Activ is N+ by section 4.2's
    // default; the abutting pair is a space of nothing.
    let act = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.activ, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "PWB.e",
        0.31,
        &|pts| vec![poly(b, pts)],
        1.0,
        &act,
        0.5,
        true,
    );

    // PWB.e.h5 - what is N+Activ in PWell.  0.305 from a 1 × 1 block, in a row at y = 2 and
    // x = 2, 6, 10, ...: plain Activ (N+ by default, fires); Activ under drawn nSD (fires);
    // Activ under nSD:block with no implant drawn (neither N+ nor P+: clean); Activ under
    // pSD (P+: PWB.f's, and 0.305 clears its 0.24); Activ in a well that overlaps the
    // block (N+ in NWell, not in PWell: clean); Activ crossing a well edge, the part in
    // PWell 0.305 from the block (fires); Activ under ThickGateOx (PWB.e1 at 0.62, fires
    // e1 not e); ThickGateOx abutting the Activ without covering it (fires e); ThickGateOx
    // over the near half of a 0.5 × 1.0 Activ 0.5 from the block (the covered half fires
    // e1, the bare half at 0.5 clears e); the Activ half under the block (the outside half
    // is N+ in PWell 0 away: fires e); Activ wholly under the block (not in PWell: clean).
    let mut e = vec![];
    let blk = |x: f64| rect(b, x, 2.0, x + 1.0, 3.0);
    let a = |x: f64, g: f64| rect(p.activ, x + 1.0 + g, 2.25, x + 1.0 + g + 0.5, 2.75);
    let g = 0.305;
    e.push(blk(2.0));
    e.push(a(2.0, g));
    e.push(blk(6.0));
    e.push(a(6.0, g));
    e.push(rect(p.nsd, 7.2, 2.1, 8.0, 2.9));
    e.push(blk(10.0));
    e.push(a(10.0, g));
    e.push(rect(p.nsdb, 11.2, 2.1, 12.0, 2.9));
    e.push(blk(14.0));
    e.push(a(14.0, g));
    e.push(rect(p.psd, 15.2, 2.1, 16.0, 2.9));
    e.push(blk(18.0));
    e.push(a(18.0, g));
    e.push(rect(p.nw, 18.5, 1.5, 20.5, 3.5));
    e.push(blk(22.0));
    e.push(rect(p.activ, 23.305, 2.25, 24.5, 2.75));
    e.push(rect(p.nw, 24.0, 1.5, 26.0, 3.5));
    e.push(blk(27.0));
    e.push(a(27.0, g));
    e.push(rect(p.tgo, 28.2, 2.1, 29.0, 2.9));
    e.push(blk(31.0));
    e.push(a(31.0, g));
    e.push(rect(p.tgo, 32.805, 2.1, 33.5, 2.9));
    e.push(blk(35.0));
    e.push(rect(p.activ, 36.5, 2.25, 37.5, 2.75));
    e.push(rect(p.tgo, 36.4, 2.1, 37.0, 2.9));
    e.push(blk(39.0));
    e.push(rect(p.activ, 39.75, 2.25, 40.25, 2.75));
    e.push(blk(43.0));
    e.push(rect(p.activ, 43.25, 2.25, 43.75, 2.75));
    p.write(deck, "PWB.e.h5", e);

    // PWB.e1.h1 - N+Activ inside ThickGateOx in PWell: 0.615 fires, 0.62 is clean; the
    // same Activ 0.615 away with the ThickGateOx over its far half only is e1 for that
    // half (0.865, clean) and e for the bare half (0.615, clean): nothing.
    let mut e = vec![];
    for (x, g, tgo) in [(2.0, 0.615, true), (6.0, 0.62, true), (10.0, 0.615, false)] {
        e.push(blk(x));
        e.push(rect(p.activ, x + 1.0 + g, 2.25, x + 1.0 + g + 0.5, 2.75));
        let t0 = if tgo {
            x + 1.0 + g - 0.1
        } else {
            x + 1.0 + g + 0.25
        };
        e.push(rect(p.tgo, t0, 2.1, x + 1.0 + g + 0.6, 2.9));
    }
    p.write(deck, "PWB.e1.h1", e);

    // PWB.f - min. PWell:block space to P+Activ (not in ThickGateOx) in PWell 0.24.  The
    // kit with a 0.5 P+Activ square (Activ under pSD) as the fixed box.
    let pact = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(p.activ, x0, y0, x1, y1),
            rect(p.psd, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    };
    space2_kit(
        p,
        deck,
        "PWB.f",
        0.24,
        &|pts| vec![poly(b, pts)],
        1.0,
        &pact,
        0.5,
        true,
    );

    // PWB.f.h5 - pSD over the near half of a 0.5 × 1.0 Activ 0.235 from the block: the
    // P+ half fires f, the bare far half is N+ 0.735 away (clean for e); pSD over the far
    // half: the near half is N+ at 0.235 (fires e), the P+ half at 0.735 is clean; a
    // P+Activ in a well overlapping the block at 0.235 (not in PWell: clean); P+Activ
    // wholly under the block (clean).
    let e = vec![
        blk(2.0),
        rect(p.activ, 3.235, 2.25, 4.235, 2.75),
        rect(p.psd, 3.2, 2.1, 3.735, 2.9),
        blk(6.0),
        rect(p.activ, 7.235, 2.25, 8.235, 2.75),
        rect(p.psd, 7.735, 2.1, 8.3, 2.9),
        blk(10.0),
        rect(p.activ, 11.235, 2.25, 11.735, 2.75),
        rect(p.psd, 11.2, 2.1, 11.8, 2.9),
        rect(p.nw, 10.5, 1.5, 12.5, 3.5),
        blk(14.0),
        rect(p.activ, 14.25, 2.25, 14.75, 2.75),
        rect(p.psd, 14.2, 2.1, 14.8, 2.9),
    ];
    p.write(deck, "PWB.f.h5", e);

    // PWB.f1.h1 - P+Activ inside ThickGateOx in PWell: 0.615 fires, 0.62 is clean.
    let mut e = vec![];
    for (x, g) in [(2.0, 0.615), (6.0, 0.62)] {
        e.push(blk(x));
        e.push(rect(p.activ, x + 1.0 + g, 2.25, x + 1.0 + g + 0.5, 2.75));
        e.push(rect(p.psd, x + 1.0 + g - 0.1, 2.1, x + 1.0 + g + 0.6, 2.9));
        e.push(rect(p.tgo, x + 1.0 + g - 0.1, 2.1, x + 1.0 + g + 0.6, 2.9));
    }
    p.write(deck, "PWB.f1.h1", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/pwellblock")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    pwellblock(&p);
}
