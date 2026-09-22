// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the block decks: section 5.2 (PWell:block, PWB.a-PWB.f1), 5.3
//! (nBuLay, NBL.a-NBL.f), 5.4 (nBuLay:block, NBLB.a-NBLB.d), 5.12 (EXTBlock, EXTB.a-c),
//! 5.13 (SalBlock, Sal.a-e) and 5.15 (ContBar, CntB.a-CntB.j) of the SG13G2 layout rules,
//! with section 4.2's derived layers (N+Activ, P+Activ, PWell, the generated nBuLay).
//! Every layout is `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//!
//! The width and space rules of the five block layers share two kits (`width_kit`,
//! `space_kit`), the two-layer space rules a third (`space2_kit`); the conditions of each
//! rule - which Activ is N+, what "in PWell" or "unrelated" means, the generated nBuLay -
//! are drawn rule by rule below.

use crate::helpers::{
    chamfered_bl, chamfered_tr, diamond, layer, library, mixed_notch_pattern, notch_pattern, poly,
    rect, strap, strip45, tap, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

/// One grid step.
const G: f64 = 0.005;
/// A layer drawn as any polygon.
type Free<'a> = dyn Fn(&[(f64, f64)]) -> Vec<GdsElement> + 'a;
/// A layer drawn as boxes, with whatever must go with them.
type Boxes<'a> = dyn Fn(f64, f64, f64, f64) -> Vec<GdsElement> + 'a;
const SQRT2: f64 = std::f64::consts::SQRT_2;

fn grid(v: f64) -> f64 {
    (v / G).round() * G
}

/// The largest grid multiple `a` with `a·√2` under `s`: a diagonal gap or a diamond's
/// half-diagonal one step under the value.
fn diag_under(s: f64) -> f64 {
    let mut a = grid((s / SQRT2 / G).floor() * G);
    while a * SQRT2 >= s - 1e-9 {
        a -= G;
    }
    a
}

/// The smallest grid multiple `a` with `a·√2` at or over `s`.
fn diag_over(s: f64) -> f64 {
    diag_under(s) + G
}

/// Every layer the six decks draw.
struct P {
    activ: (i16, i16),
    gp: (i16, i16),
    cont: (i16, i16),
    m1: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    nsdb: (i16, i16),
    nw: (i16, i16),
    pwb: (i16, i16),
    nbl: (i16, i16),
    nblb: (i16, i16),
    extb: (i16, i16),
    sal: (i16, i16),
    tgo: (i16, i16),
}

impl P {
    fn new(pdk: &PdkConfig) -> Self {
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

    fn write(&self, deck: &str, name: &str, elems: Vec<GdsElement>) {
        write_gz(
            &format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz"),
            library("TOP", elems),
        );
    }
}

/// A rectangular frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, drawn as
/// four abutting boxes that merge into one ring.
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
        rect(l, x0, y0, x1, hy0),
        rect(l, x0, hy1, x1, y1),
        rect(l, x0, hy0, hx0, hy1),
        rect(l, hx1, hy0, x1, hy1),
    ]
}

/// A comb: a plate `(x, y)-(x + width, y + height)` with `n` slots of width `slot` and
/// depth `depth` cut into its top edge, teeth `tooth` wide between them.
#[allow(clippy::too_many_arguments)]
fn comb(
    l: (i16, i16),
    x: f64,
    y: f64,
    height: f64,
    n: usize,
    slot: f64,
    depth: f64,
    tooth: f64,
) -> GdsElement {
    let mut pts = vec![(x, y)];
    let width = tooth * (n + 1) as f64 + slot * n as f64;
    pts.push((x + width, y));
    pts.push((x + width, y + height));
    let mut cx = x + width;
    for _ in 0..n {
        cx -= tooth;
        pts.push((cx, y + height));
        pts.push((cx, y + height - depth));
        cx -= slot;
        pts.push((cx, y + height - depth));
        pts.push((cx, y + height));
    }
    pts.push((x, y + height));
    poly(l, &pts)
}

// --- The kits ---

/// Min. width `w` of layer `l` (`s`, the largest space the deck asks of the layer, keeps
/// the sub-patterns apart): `<rule>.h1` the bound and the 45° shapes, `.h2` shapes that
/// merge, `.h3` the tile lines, a 300 µm bar and (1000, 1000), `.h4`/`.h5` fifty flat and
/// as an array.
fn width_kit(p: &P, deck: &str, rule: &str, l: (i16, i16), w: f64, s: f64) {
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

    // h3 - tile lines.  d bars (two walls each) inside a tile at x = 10, straddling
    // x = 20, ending on 20, starting on 20, straddling 21, 40 and 42, one straddling
    // y = 20, one at (1000, 1000); a d × 300 bar across every line at y = 30; a w bar
    // straddling x = 40 is clean.
    let row = len + gap;
    let e = vec![
        rect(l, 9.9, 2.0, 9.9 + d, 2.0 + len),
        rect(l, 19.9, 2.0, 19.9 + d, 2.0 + len),
        rect(l, 20.0 - d, 2.0 + row, 20.0, 2.0 + row + len),
        rect(l, 20.0, 2.0 + 2.0 * row, 20.0 + d, 2.0 + 2.0 * row + len),
        rect(l, 20.9, 2.0 + 3.0 * row, 20.9 + d, 2.0 + 3.0 * row + len),
        rect(l, 39.9, 2.0, 39.9 + d, 2.0 + len),
        rect(l, 41.9, 2.0 + 2.0 * row, 41.9 + d, 2.0 + 2.0 * row + len),
        rect(l, 10.0, 19.9, 10.0 + len, 19.9 + d),
        rect(l, 1000.0, 1000.0, 1000.0 + d, 1000.0 + len),
        rect(l, 2.0, 30.0, 302.0, 30.0 + d),
        rect(l, 39.9, 2.0 + 3.0 * row, 39.9 + w, 2.0 + 3.0 * row + len),
    ];
    p.write(deck, &format!("{rule}.h3"), e);
}

/// Min. space (and notch) `s` of layer `l`, whose min. width is `w` (`clear`, the largest
/// space the deck asks of the layer, keeps the sub-patterns apart): `<rule>.h1` the bound
/// and both metrics, `.h2` notches and unions (when `notch`), `.h3` the tile lines, a
/// 300 µm pair and (1000, 1000), `.h4`/`.h5` fifty flat and as an array.
#[allow(clippy::too_many_arguments)]
fn space_kit(
    p: &P,
    deck: &str,
    rule: &str,
    l: (i16, i16),
    s: f64,
    w: f64,
    clear: f64,
    notch: bool,
) {
    let d = s - G;
    let q = 2.0 * s.max(w);
    let gap = clear.max(s) + 1.0;
    let av = diag_under(s);
    let ac = diag_over(s);
    let bx = |x: f64, y: f64| rect(l, x, y, x + q, y + q);

    // h1 - the bound and both metrics.  Pairs at s (clean) and d (fires); corner to
    // corner at av/av (av·√2 under s, fires) and ac/ac (clean); 0.1 in x with s in y
    // (clean: the euclidian gap is over s); a diamond tip d from a wall (fires) and s
    // (clean); a corner facing a chamfer av·√2 away (fires) and ac·√2 (clean); two
    // parallel chamfers av·√2 apart (fires) and ac·√2 (clean).
    let y = 2.0;
    let mut x = 2.0;
    let mut e = vec![bx(x, y), bx(x + q + s, y)];
    x += 2.0 * q + s + gap;
    e.push(bx(x, y));
    e.push(bx(x + q + d, y));
    x += 2.0 * q + d + gap;
    for a in [av, ac] {
        e.push(bx(x, y));
        e.push(bx(x + q + a, y + q + a));
        x += 2.0 * q + a + gap;
    }
    e.push(bx(x, y));
    e.push(bx(x + q + 0.1, y + q + s));
    x += 2.0 * q + gap;
    for g in [d, s] {
        e.push(bx(x, y));
        e.push(diamond(l, x + q + g + q / 2.0, y + q / 2.0, q / 2.0));
        x += 2.0 * q + g + gap;
    }
    for a in [av, ac] {
        let c = 2.0 * a + 0.1;
        let qa = q + c;
        e.push(chamfered_tr(l, x, y, x + qa, y + qa, x + qa + y + qa - c));
        e.push(bx(x + qa - 0.05, y + qa - 0.05));
        x += qa + q + gap;
    }
    for a in [av, ac] {
        let qa = q + 2.0 * a;
        let k1 = x + qa + y + qa - 2.0 * a;
        e.push(chamfered_tr(l, x, y, x + qa, y + qa, k1));
        e.push(chamfered_bl(
            l,
            x + qa - a,
            y + qa - a,
            x + 2.0 * qa - a,
            y + 2.0 * qa - a,
            k1 + 2.0 * a,
        ));
        x += 2.0 * qa + gap;
    }
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - notches and unions.  A U notch and a straight-vs-45° notch of d (the helpers'
    // patterns, s controls beside them); a comb with three d slots; a ring whose hole is d
    // wide; a square in a ring's hole d from one inner wall (s from the others); two
    // unions (each two overlapping boxes) d apart.
    if notch {
        let t = w + 0.1;
        let mut e = notch_pattern(l, t, s, gap, 2.0, -G);
        let size = (s + 2.0 * t).max(1.0);
        let ny = size + gap;
        e.extend(
            mixed_notch_pattern(l, t, s, 0.5, gap, 2.0, -G)
                .into_iter()
                .map(|el| shift1(&el, 0.0, ny)),
        );
        let ny2 = ny + (d + 0.5 + 2.0 * t).max(1.0) + gap;
        let mut x = 2.0;
        e.push(comb(l, x, ny2, 2.0 * s + t, 3, d, 2.0 * s, t));
        x += 4.0 * t + 3.0 * d + gap;
        let o = grid(3.0 * s + 2.0 * t);
        e.extend(ring(
            l,
            x,
            ny2,
            x + o,
            ny2 + o,
            x + t,
            ny2 + t,
            x + t + d,
            ny2 + o - t,
        ));
        x += o + gap;
        let (ox, oy) = (q + s + d + 2.0 * t, q + 2.0 * s + 2.0 * t);
        e.extend(ring(
            l,
            x,
            ny2,
            x + ox,
            ny2 + oy,
            x + t,
            ny2 + t,
            x + ox - t,
            ny2 + oy - t,
        ));
        e.push(rect(
            l,
            x + t + s,
            ny2 + t + s,
            x + t + s + q,
            ny2 + t + s + q,
        ));
        x += ox + gap;
        e.push(rect(l, x, ny2, x + q - 0.1, ny2 + q));
        e.push(rect(l, x + 0.1, ny2, x + q, ny2 + q));
        e.push(rect(l, x + q + d, ny2, x + 2.0 * q + d - 0.1, ny2 + q));
        e.push(rect(l, x + q + d + 0.1, ny2, x + 2.0 * q + d, ny2 + q));
        p.write(deck, &format!("{rule}.h2"), e);
    }

    // h3 - tile lines.  d gaps straddling x = 20 (three rows: the gap over the line, the
    // gap starting on it, the gap ending on it), straddling 21, 40 and 42, one straddling
    // y = 20 at x = xy (30, or further right of the x = 20 column for a large value), a
    // corner-to-corner pair whose corner is on (xc, 20) (60, or the next multiple of 20
    // clear of the x = 42 column), a 300 µm pair at y = 60, one at (1000, 1000).
    let row = q + gap;
    let xy = 30f64.max(grid(20.0 + d + q + gap));
    let xc = 60f64.max(((42.0 + d + 2.0 * q + gap) / 20.0).ceil() * 20.0);
    let e = vec![
        bx(19.9 - q, 2.0),
        bx(19.9 + d, 2.0),
        bx(20.0 - q, 2.0 + row),
        bx(20.0 + d, 2.0 + row),
        bx(20.0 - d - q, 2.0 + 2.0 * row),
        bx(20.0, 2.0 + 2.0 * row),
        bx(20.9 - q, 2.0 + 3.0 * row),
        bx(20.9 + d, 2.0 + 3.0 * row),
        bx(39.9 - q, 2.0),
        bx(39.9 + d, 2.0),
        bx(41.9 - q, 2.0 + row),
        bx(41.9 + d, 2.0 + row),
        bx(xy, 19.9 - q),
        bx(xy, 19.9 + d),
        bx(xc - q, 20.0 - q),
        bx(xc + av, 20.0 + av),
        rect(l, 2.0, 60.0, 302.0, 60.0 + w + 0.1),
        rect(
            l,
            2.0,
            60.0 + w + 0.1 + d,
            302.0,
            60.0 + 2.0 * (w + 0.1) + d,
        ),
        bx(1000.0, 1000.0),
        bx(1000.0 + q + d, 1000.0),
    ];
    p.write(deck, &format!("{rule}.h3"), e);
}

/// Translate one element.
fn shift1(e: &GdsElement, dx: f64, dy: f64) -> GdsElement {
    crate::helpers::shift(std::slice::from_ref(e), dx, dy).remove(0)
}

/// A two-layer space rule of value `s` between a `free` layer (drawn as any polygon: the
/// block or well) and a `fixed` one (drawn as boxes `qf` across, with whatever must go
/// with them): `<rule>.h1` the bound, both metrics, the 45° shapes of the free layer and
/// (when `touch`) the abutting pair, `.h2` the tile lines, a 300 µm run and (1000, 1000),
/// `.h3`/`.h4` fifty flat and as an array.
#[allow(clippy::too_many_arguments)]
fn space2_kit(
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
    // on it), straddling 21, 40, 42, y = 20 at x = xy, a corner on (xc, 20) (as in
    // `space_kit`), a 300 µm free strip d from a fixed box at y = 60, one pair at (1000,
    // 1000).
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

pub fn generate(pdk: &PdkConfig) {
    let p = P::new(pdk);
    for deck in [
        "pwellblock",
        "nbulay",
        "nbulayblock",
        "extblock",
        "salblock",
        "contbar",
    ] {
        std::fs::create_dir_all(format!("tests/data/ihp-sg13g2/{deck}"))
            .expect("failed to create output directory");
    }
    pwellblock(&p);
    nbulay(&p);
    nbulayblock(&p);
    extblock(&p);
    salblock(&p);
    contbar(&p);
}

// --- 5.2 PWell:block ---

fn pwellblock(p: &P) {
    let deck = "pwellblock";
    let b = p.pwb;
    width_kit(p, deck, "PWB.a", b, 0.62, 0.62);
    space_kit(p, deck, "PWB.b", b, 0.62, 0.62, 0.62, true);

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

/// A min. enclosure `v` of the `inner` layer (drawn as boxes `qi` across, with whatever
/// must go with them) by the `outer` one (any polygon; `clear` keeps its shapes apart):
/// `<rule>.h1` the bound on each
/// side, the chamfer, the crossing and the coincident edge, `.h2` the tile lines, a 300 µm
/// strip and (1000, 1000), `.h3`/`.h4` fifty flat and as an array.
#[allow(clippy::too_many_arguments)]
fn enclosure_kit(
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

// --- 5.3 nBuLay ---

fn nbulay(p: &P) {
    let deck = "nbulay";
    let n = p.nbl;
    width_kit(p, deck, "NBL.a", n, 1.0, 3.2);
    // NBL.b - space or notch (same net) 1.50: the notch half is in h2.
    space_kit(p, deck, "NBL.b", n, 1.5, 1.0, 3.2, true);
    // NBL.c - PWell width between nBuLay regions (different net) 3.20.  Bare regions are
    // different nets; no notch layout: a notch is one region.
    space_kit(p, deck, "NBL.c", n, 3.2, 1.0, 3.2, false);

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

// --- 5.4 nBuLay:block ---

fn nbulayblock(p: &P) {
    let deck = "nbulayblock";
    let b = p.nblb;
    width_kit(p, deck, "NBLB.a", b, 1.5, 1.0);
    space_kit(p, deck, "NBLB.b", b, 1.0, 1.5, 1.0, true);

    // NBLB.c - nBuLay enclosure of nBuLay:block 1.00.  The kit with a 2 × 2 block.
    let blk = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(b, x0, y0, x1, y1)];
    enclosure_kit(
        p,
        deck,
        "NBLB.c",
        1.0,
        &|pts| vec![poly(p.nbl, pts)],
        &blk,
        2.0,
        3.2,
    );

    // NBLB.c.h5 - a block with no drawn nBuLay.  A bare 2 × 2 block at (2, 2) (nothing
    // encloses it and nothing is near it: clean); a block 2.5 inside every edge of a
    // 10 × 10 well (the well's generated nBuLay, inset by 1.0, encloses it by 1.5:
    // clean); a block 1.5 inside a 10 × 10 well (enclosed by 0.5 of generated nBuLay:
    // fires); a block over a whole 6 × 6 well and 1.0 beyond it (no nBuLay is left to
    // enclose anything: clean).
    let e = vec![
        rect(b, 2.0, 2.0, 4.0, 4.0),
        rect(p.nw, 8.0, 2.0, 18.0, 12.0),
        rect(b, 10.5, 4.5, 15.5, 9.5),
        rect(p.nw, 22.0, 2.0, 32.0, 12.0),
        rect(b, 23.5, 3.5, 30.5, 10.5),
        rect(p.nw, 36.0, 2.0, 42.0, 8.0),
        rect(b, 35.0, 1.0, 43.0, 9.0),
    ];
    p.write(deck, "NBLB.c.h5", e);

    // NBLB.d - nBuLay:block space to unrelated nBuLay 1.50.  The kit with a 2 × 2
    // nBuLay as the fixed box and the block as the free shape; the abutting pair is in h5.
    let nbl = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.nbl, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "NBLB.d",
        1.5,
        &|pts| vec![poly(b, pts)],
        2.5,
        &nbl,
        2.0,
        false,
    );

    // NBLB.d.h5 - the relation.  A block inside an nBuLay with 1.0 margins (NBLB.c's,
    // clean for d); a block crossing the nBuLay edge (NBLB.c, not d); a block abutting the
    // nBuLay from outside (unrelated, 0 away: fires); a block 0.4 outside a 6 × 6 well (the
    // generated nBuLay, inset by 1.0, is 1.4 away: fires) and one 0.5 outside (1.5: clean).
    let e = vec![
        rect(p.nbl, 2.0, 2.0, 6.0, 6.0),
        rect(b, 3.0, 3.0, 5.0, 5.0),
        rect(p.nbl, 9.0, 2.0, 13.0, 6.0),
        rect(b, 12.0, 3.0, 14.0, 5.0),
        rect(p.nbl, 17.0, 2.0, 21.0, 6.0),
        rect(b, 21.0, 3.0, 23.0, 5.0),
        rect(p.nw, 2.0, 10.0, 8.0, 16.0),
        rect(b, 8.4, 12.0, 10.4, 14.0),
        rect(p.nw, 14.0, 10.0, 20.0, 16.0),
        rect(b, 20.5, 12.0, 22.5, 14.0),
    ];
    p.write(deck, "NBLB.d.h5", e);
}

// --- 5.12 EXTBlock ---

fn extblock(p: &P) {
    let deck = "extblock";
    let l = p.extb;
    width_kit(p, deck, "EXTB.a", l, 0.31, 0.31);
    space_kit(p, deck, "EXTB.b", l, 0.31, 0.31, 0.31, true);
    // EXTB.c - EXTBlock space to pSD 0.31; pSD alone is pSD.
    let psd = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.psd, x0, y0, x1, y1)];
    space2_kit(
        p,
        deck,
        "EXTB.c",
        0.31,
        &|pts| vec![poly(l, pts)],
        1.0,
        &psd,
        0.5,
        true,
    );
}

// --- 5.13 SalBlock ---

fn salblock(p: &P) {
    let deck = "salblock";
    let sb = p.sal;
    width_kit(p, deck, "Sal.a", sb, 0.42, 0.42);
    space_kit(p, deck, "Sal.b", sb, 0.42, 0.42, 0.42, true);

    // Sal.c - SalBlock extension over Activ or GatPoly 0.20.  The kit with a 0.5 Activ.
    let act = |x0: f64, y0: f64, x1: f64, y1: f64| vec![rect(p.activ, x0, y0, x1, y1)];
    enclosure_kit(
        p,
        deck,
        "Sal.c",
        0.2,
        &|pts| vec![poly(sb, pts)],
        &act,
        0.5,
        0.42,
    );

    // Sal.c.h5 - the extension and the union.  An Activ strip crossing a block that
    // extends 0.195 past its top edge (fires) and 0.2 (clean); a GatPoly strip crossing a
    // block with 0.195 (fires); a 0.5 Activ 0.195 from the block's bottom edge wholly
    // under a GatPoly that runs out of the block (the union's boundary there is the
    // poly's crossing, not the Activ's edge: clean); the same Activ with the poly inside
    // the block at 0.2 (the Activ's edge is the union's: fires).
    let e = vec![
        rect(p.activ, 2.0, 3.0, 6.0, 3.5),
        rect(sb, 3.0, 2.5, 4.0, 3.695),
        rect(p.activ, 8.0, 3.0, 12.0, 3.5),
        rect(sb, 9.0, 2.5, 10.0, 3.7),
        rect(p.gp, 14.0, 3.0, 18.0, 3.5),
        rect(sb, 15.0, 2.5, 16.0, 3.695),
        rect(sb, 19.8, 2.0, 21.2, 4.0),
        rect(p.activ, 20.2, 2.195, 20.8, 2.6),
        rect(p.gp, 20.1, 1.5, 20.9, 2.8),
        rect(sb, 22.8, 2.0, 24.2, 4.0),
        rect(p.activ, 23.2, 2.195, 23.8, 2.6),
        rect(p.gp, 23.2, 2.4, 23.8, 2.8),
    ];
    p.write(deck, "Sal.c.h5", e);

    // Sal.d - SalBlock space to unrelated Activ or GatPoly 0.20.  The kit with a 0.5
    // Activ; the abutting pair is a space of nothing.
    space2_kit(
        p,
        deck,
        "Sal.d",
        0.2,
        &|pts| vec![poly(sb, pts)],
        1.0,
        &act,
        0.5,
        true,
    );

    // Sal.d.h5 - GatPoly 0.195 from a block (fires); a U-shaped Activ whose left arm is
    // under the block (extended by 0.5) and whose right arm is 0.195 from the block's
    // right edge (the arm is not covered, so the space is measured: fires); a 0.5 Activ
    // 0.195 from a block that covers another Activ (fires).
    let e = vec![
        rect(sb, 2.0, 2.0, 3.0, 3.0),
        rect(p.gp, 3.195, 2.25, 3.695, 2.75),
        rect(sb, 6.0, 2.0, 7.0, 4.0),
        poly(
            p.activ,
            &[
                (6.5, 2.5),
                (6.5, 3.5),
                (7.195, 3.5),
                (7.195, 3.0),
                (7.7, 3.0),
                (7.7, 3.5),
                (8.2, 3.5),
                (8.2, 2.5),
            ],
        ),
        rect(sb, 11.0, 2.0, 12.0, 4.0),
        rect(p.activ, 11.2, 2.5, 11.8, 3.5),
        rect(p.activ, 12.195, 2.75, 12.695, 3.25),
    ];
    p.write(deck, "Sal.d.h5", e);

    // Sal.e - SalBlock space to Cont 0.20.  Every Cont sits at the right end of an Activ
    // strip that runs under the block (so the Activ is related and Sal.d stays quiet)
    // with Metal1 over it.
    // h1: 0.2 (clean), 0.195 (fires), corner to corner 0.14/0.14 (0.198: fires) and
    // 0.145/0.145 (0.205: clean), a Cont abutting the block (fires), a Cont inside the
    // block (a Schottky's bar: not a space, clean), a Cont crossing the block edge (its
    // outside part 0 away: fires), a 0.16 × 0.5 bar 0.195 away (fires).
    let cont = |x: f64, y: f64| {
        vec![
            rect(p.cont, x, y, x + 0.16, y + 0.16),
            rect(p.m1, x - 0.05, y - 0.05, x + 0.21, y + 0.21),
        ]
    };
    let mut e = vec![];
    for (k, (dx, dy)) in [
        (0.2, 0.0),
        (0.195, 0.0),
        (0.14, 0.14),
        (0.145, 0.145),
        (0.0, 0.0),
    ]
    .iter()
    .enumerate()
    {
        let x = 2.0 + k as f64 * 3.0;
        e.push(rect(sb, x, 2.0, x + 1.0, 3.0));
        if *dy > 0.0 {
            e.push(rect(p.activ, x + 0.5, 2.3, x + 1.5, 3.3 + dy));
            e.extend(cont(x + 1.0 + dx, 3.0 + dy));
        } else {
            e.push(rect(p.activ, x + 0.5, 2.3, x + 1.5, 2.7));
            e.extend(cont(x + 1.0 + dx, 2.42));
        }
    }
    e.push(rect(sb, 17.0, 2.0, 18.0, 3.0));
    e.push(rect(p.activ, 17.2, 2.2, 17.8, 2.8));
    e.extend(cont(17.42, 2.42));
    e.push(rect(sb, 20.0, 2.0, 21.0, 3.0));
    e.push(rect(p.activ, 20.5, 2.3, 21.5, 2.7));
    e.extend(cont(20.92, 2.42));
    e.push(rect(sb, 23.0, 2.0, 24.0, 3.0));
    e.push(rect(p.activ, 23.5, 2.3, 24.9, 2.7));
    e.push(rect(p.cont, 24.195, 2.42, 24.695, 2.58));
    e.push(rect(p.m1, 24.145, 2.37, 24.745, 2.63));
    p.write(deck, "Sal.e.h1", e);

    // Sal.e.h2 - tile lines: 0.195 gaps straddling x = 20 (three ways), 21, 40, 42, y = 20
    // at x = 30, a corner 0.14/0.14 on (60, 20), a 300 µm block at y = 60, (1000, 1000).
    let pair = |x: f64, y: f64, g: f64| {
        let mut e = vec![
            rect(sb, x - 1.0, y - 0.5, x, y + 0.5),
            rect(p.activ, x - 0.5, y - 0.2, x + g + 0.5, y + 0.2),
        ];
        e.extend(cont(x + g, y - 0.08));
        e
    };
    let mut e = vec![];
    e.extend(pair(19.9, 2.0, 0.195));
    e.extend(pair(20.0, 4.0, 0.195));
    e.extend(pair(20.0 - 0.195, 6.0, 0.195));
    e.extend(pair(20.9, 8.0, 0.195));
    e.extend(pair(39.9, 2.0, 0.195));
    e.extend(pair(41.9, 4.0, 0.195));
    e.push(rect(sb, 30.0, 18.9, 31.0, 19.9));
    e.push(rect(p.activ, 30.3, 19.4, 30.7, 20.5));
    e.extend(cont(30.42, 19.9 + 0.195));
    e.push(rect(sb, 59.0, 19.0, 60.0, 20.0));
    e.push(rect(p.activ, 59.5, 19.5, 60.5, 20.5));
    e.extend(cont(60.14, 20.14));
    e.push(rect(sb, 2.0, 60.0, 302.0, 61.0));
    e.push(rect(p.activ, 150.0, 60.5, 150.4, 61.6));
    e.extend(cont(150.12, 61.195));
    e.extend(pair(1000.0, 1000.0, 0.195));
    p.write(deck, "Sal.e.h2", e);
}

// --- 5.15 ContBar ---

fn contbar(p: &P) {
    let deck = "contbar";
    let c = p.cont;
    // A Cont bar `(x, y)-(x + w, y + h)` on Activ with 0.07 of it around, and Metal1
    // with 0.05.
    let bar = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            rect(c, x, y, x + w, y + h),
            rect(p.m1, x - 0.05, y - 0.05, x + w + 0.05, y + h + 0.05),
            rect(p.activ, x - 0.07, y - 0.07, x + w + 0.07, y + h + 0.07),
        ]
    };
    // A Cont polygon with Activ and Metal1 around its bounding box.
    let cpoly = |pts: &[(f64, f64)]| {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        vec![
            poly(c, pts),
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };

    // CntB.a/CntB.a1.h1 - the bound.  0.16 × 0.34 and 0.34 × 0.16 bars are clean; 0.155
    // and 0.165 wide bars (both orientations) are CntB.a; 0.16 × 0.335 and 0.16 × 0.165
    // (both orientations) are CntB.a1; a 0.16 × 0.5 bar with a 0.005 nick in its side is
    // 0.155 wide there (CntB.a); a 0.17 square is a square, Cont's, not a bar (clean
    // here); an L and a T of 0.16 arms 0.5 long are 0.16 wide everywhere (clean); a bar
    // drawn as two 0.08 halves, as two overlapping 0.16 × 0.3 boxes and as a 0.16 square
    // abutting a 0.16 × 0.5 bar end to end are one clean bar each; two 0.16 × 0.5 bars
    // overlapping sideways by 0.06 are 0.26 wide (CntB.a); a 45° bar of width
    // 0.115·√2 = 0.163 is over 0.16 (CntB.a).
    let mut e = vec![];
    let mut x = 2.0;
    for (w, h) in [
        (0.16, 0.34),
        (0.34, 0.16),
        (0.155, 0.5),
        (0.5, 0.155),
        (0.165, 0.5),
        (0.5, 0.165),
        (0.16, 0.335),
        (0.335, 0.16),
        (0.16, 0.165),
        (0.165, 0.16),
    ] {
        e.extend(bar(x, 2.0, w, h));
        x += 1.0;
    }
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.16, 2.0),
        (x + 0.16, 2.2),
        (x + 0.155, 2.2),
        (x + 0.155, 2.3),
        (x + 0.16, 2.3),
        (x + 0.16, 2.5),
        (x, 2.5),
    ]));
    x += 1.0;
    e.extend(bar(x, 2.0, 0.17, 0.17));
    x += 1.0;
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.5, 2.0),
        (x + 0.5, 2.16),
        (x + 0.16, 2.16),
        (x + 0.16, 2.5),
        (x, 2.5),
    ]));
    x += 1.0;
    e.extend(cpoly(&[
        (x, 2.0),
        (x + 0.66, 2.0),
        (x + 0.66, 2.16),
        (x + 0.41, 2.16),
        (x + 0.41, 2.66),
        (x + 0.25, 2.66),
        (x + 0.25, 2.16),
        (x, 2.16),
    ]));
    x += 1.0;
    let cover = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };
    e.extend(cover(x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x, 2.0, x + 0.08, 2.5));
    e.push(rect(c, x + 0.08, 2.0, x + 0.16, 2.5));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x, 2.0, x + 0.16, 2.3));
    e.push(rect(c, x, 2.2, x + 0.16, 2.5));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.16, 2.66));
    e.push(rect(c, x, 2.0, x + 0.16, 2.16));
    e.push(rect(c, x, 2.16, x + 0.16, 2.66));
    x += 1.0;
    e.extend(cover(x, 2.0, x + 0.26, 2.5));
    e.push(rect(c, x, 2.0, x + 0.16, 2.5));
    e.push(rect(c, x + 0.1, 2.0, x + 0.26, 2.5));
    x += 1.0;
    e.extend(cpoly(&[
        (x + 0.115, 2.0),
        (x + 0.615, 2.5),
        (x + 0.5, 2.615),
        (x, 2.115),
    ]));
    p.write(deck, "CntB.a.h1", e);

    // CntB.a.h2 - tile lines.  0.155 × 0.5 bars (CntB.a) straddling x = 20, ending on
    // 20, starting on 20, straddling 21, 40, 42, one straddling y = 20, one at (1000,
    // 1000) and a 0.155 × 300 bar at y = 30; 0.16 × 0.335 bars (CntB.a1) the same eight
    // ways; a 0.16 × 5 bar straddling x = 20 is clean.
    let mut e = vec![];
    for (k, (w, h)) in [(0.5, 0.155), (0.335, 0.16)].iter().enumerate() {
        let y = 2.0 + k as f64 * 6.0;
        e.extend(bar(19.9, y, *w, *h));
        e.extend(bar(20.0 - w, y + 1.0, *w, *h));
        e.extend(bar(20.0, y + 2.0, *w, *h));
        e.extend(bar(20.9, y + 3.0, *w, *h));
        e.extend(bar(39.9, y, *w, *h));
        e.extend(bar(41.9, y, *w, *h));
        e.extend(bar(10.0 + k as f64 * 3.0, 19.9, *h, *w));
        e.extend(bar(1000.0, 1000.0 + k as f64 * 3.0, *w, *h));
    }
    e.extend(bar(2.0, 30.0, 300.0, 0.155));
    e.extend(bar(17.5, 40.0, 5.0, 0.16));
    p.write(deck, "CntB.a.h2", e);

    // CntB.a.h3/h4 - fifty cells of a 0.155 × 0.5 bar (CntB.a) and a 0.16 × 0.335 bar
    // (CntB.a1), flat and as an array.
    let mut cell = bar(0.2, 0.2, 0.5, 0.155);
    cell.extend(bar(0.2, 1.2, 0.335, 0.16));

    // CntB.b.h1 - space 0.28.  Side by side at 0.28 (clean) and 0.275 (fires); end to
    // end at 0.275 (fires); a bar's end 0.275 from another's side (fires); corner to
    // corner 0.195/0.195 (0.276: fires) and 0.2/0.2 (0.283: clean); 0.1 in x with 0.28
    // in y (clean); two 0.16 × 5.5 bars 0.275 apart (CntB.b and, the run being over 5,
    // CntB.b1).
    let mut e = vec![];
    let mut x = 2.0;
    for g in [0.28, 0.275] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(bar(x + 0.16 + g, 2.0, 0.16, 0.5));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(bar(x, 2.5 + 0.275, 0.16, 0.5));
    x += 1.5;
    e.extend(bar(x, 2.0, 0.5, 0.16));
    e.extend(bar(x + 0.17, 2.16 + 0.275, 0.16, 0.5));
    x += 1.5;
    for d in [0.195, 0.2] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(bar(x + 0.16 + d, 2.5 + d, 0.16, 0.5));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(bar(x + 0.16 + 0.1, 2.5 + 0.28, 0.16, 0.5));
    x += 1.5;
    e.extend(bar(x, 2.0, 0.16, 5.5));
    e.extend(bar(x + 0.16 + 0.275, 2.0, 0.16, 5.5));
    p.write(deck, "CntB.b.h1", e);

    // CntB.b.h2 - tile lines.  0.275 gaps between 0.16 × 0.5 bars: end to end across
    // x = 20 (gap over the line, starting on it, ending on it), across 21, 40, 42; side by
    // side across y = 20 at x = 30; a corner pair 0.195/0.195 on (60, 20); two 300 µm bars
    // 0.275 apart at y = 60 (CntB.b and CntB.b1); a pair at (1000, 1000).
    let ee = |x: f64, y: f64| {
        let mut e = bar(x - 0.5, y, 0.5, 0.16);
        e.extend(bar(x + 0.275, y, 0.5, 0.16));
        e
    };
    let mut e = vec![];
    e.extend(ee(19.9, 2.0));
    e.extend(ee(20.0, 3.0));
    e.extend(ee(20.0 - 0.275, 4.0));
    e.extend(ee(20.9, 5.0));
    e.extend(ee(39.9, 2.0));
    e.extend(ee(41.9, 3.0));
    e.extend(bar(30.0, 19.9 - 0.16, 0.5, 0.16));
    e.extend(bar(30.0, 19.9 + 0.275, 0.5, 0.16));
    e.extend(bar(60.0 - 0.16, 20.0 - 0.5, 0.16, 0.5));
    e.extend(bar(60.0 + 0.195, 20.0 + 0.195, 0.16, 0.5));
    e.extend(bar(2.0, 60.0, 300.0, 0.16));
    e.extend(bar(2.0, 60.0 + 0.16 + 0.275, 300.0, 0.16));
    e.extend(ee(1000.0, 1000.0));
    p.write(deck, "CntB.b.h2", e);

    // CntB.b.h3/h4 - fifty 0.275 pairs, flat and as an array.
    let mut cell = bar(0.2, 0.2, 0.16, 0.5);
    cell.extend(bar(0.2 + 0.16 + 0.275, 0.2, 0.16, 0.5));

    // CntB.b1.h1 - space 0.36 with a common run over 5 µm.  Two 0.16 × 6 bars at 0.355
    // (fires) and 0.36 (clean); 0.16 × 5.0 bars at 0.355 (a run of exactly 5: clean) and
    // 0.16 × 5.005 (fires); 6 bars offset so the run is 4.5 (clean) and 5.005 (fires); a
    // 6.5 bar facing three collinear 1.9 bars 0.28 apart (each pair's common run is
    // 1.9: clean); a 6.5 bar facing a bar 0.355 away whose upper half jogs 0.05 closer
    // (one stepped wall running alongside for 6.0 under the value: fires; the 0.11
    // chord across the jog is a CntB.a width); five 6 bars at 0.355 (four pairs); a 6
    // bar whose end is 0.355 from a 6 bar's side (a run of 0.16: clean).
    let mut e = vec![];
    let mut x = 2.0;
    for (len, g) in [(6.0, 0.355), (6.0, 0.36), (5.0, 0.355), (5.005, 0.355)] {
        e.extend(bar(x, 2.0, 0.16, len));
        e.extend(bar(x + 0.16 + g, 2.0, 0.16, len));
        x += 1.5;
    }
    for off in [1.5, 0.995] {
        e.extend(bar(x, 2.0, 0.16, 6.0));
        e.extend(bar(x + 0.16 + 0.355, 2.0 + off, 0.16, 6.0));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 6.5));
    for k in 0..3 {
        e.extend(bar(x + 0.16 + 0.355, 2.2 + k as f64 * 2.18, 0.16, 1.9));
    }
    x += 1.5;
    e.extend(bar(x, 2.0, 0.16, 6.5));
    let jx = x + 0.16 + 0.355;
    e.extend(cpoly(&[
        (jx, 2.2),
        (jx + 0.16, 2.2),
        (jx + 0.16, 5.2),
        (jx + 0.11, 5.2),
        (jx + 0.11, 8.2),
        (jx - 0.05, 8.2),
        (jx - 0.05, 5.2),
        (jx, 5.2),
    ]));
    x += 1.5;
    for k in 0..5 {
        e.extend(bar(x + k as f64 * (0.16 + 0.355), 2.0, 0.16, 6.0));
    }
    x += 4.0;
    e.extend(bar(x, 2.0, 0.16, 6.0));
    e.extend(bar(x + 0.16 + 0.355, 4.0, 6.0, 0.16));
    p.write(deck, "CntB.b1.h1", e);

    // CntB.b1.h2 - tile lines.  Pairs of 0.16 × 6 bars 0.355 apart, horizontal, running
    // from x = 17 to 23 (the run cut by x = 20 and 21), from 14 to 20 (ending on the
    // line), from 20 to 26 (starting on it), from 37 to 43 (across 40 and 42), from 4 to
    // 10 (across 7); vertical from y = 17 to 23 at x = 30; a 5.005 pair from 17.5 to
    // 22.505; two 300 µm bars 0.355 apart at y = 60; a pair at (1000, 1000).
    let hp = |x: f64, y: f64, len: f64| {
        let mut e = bar(x, y, len, 0.16);
        e.extend(bar(x, y + 0.16 + 0.355, len, 0.16));
        e
    };
    let mut e = vec![];
    e.extend(hp(17.0, 2.0, 6.0));
    e.extend(hp(14.0, 3.0, 6.0));
    e.extend(hp(20.0, 4.0, 6.0));
    e.extend(hp(37.0, 2.0, 6.0));
    e.extend(hp(4.0, 3.0, 6.0));
    e.extend(bar(30.0, 17.0, 0.16, 6.0));
    e.extend(bar(30.0 + 0.16 + 0.355, 17.0, 0.16, 6.0));
    e.extend(hp(17.5, 5.0, 5.005));
    e.extend(hp(2.0, 60.0, 300.0));
    e.extend(hp(1000.0, 1000.0, 6.0));
    p.write(deck, "CntB.b1.h2", e);

    // CntB.b2.h1 - space to Cont 0.22.  A 0.16 square beside a 0.16 × 0.5 bar at 0.22
    // (clean) and 0.215 (fires); at the bar's end 0.215 (fires); corner to corner
    // 0.15/0.15 (0.212: fires) and 0.16/0.16 (0.226: clean); 0.1 in x and 0.22 in y
    // (clean).
    let sq = |x: f64, y: f64| bar(x, y, 0.16, 0.16);
    let mut e = vec![];
    let mut x = 2.0;
    for g in [0.22, 0.215] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(sq(x + 0.16 + g, 2.1));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(sq(x, 2.5 + 0.215));
    x += 1.5;
    for d in [0.15, 0.16] {
        e.extend(bar(x, 2.0, 0.16, 0.5));
        e.extend(sq(x + 0.16 + d, 2.5 + d));
        x += 1.5;
    }
    e.extend(bar(x, 2.0, 0.16, 0.5));
    e.extend(sq(x + 0.16 + 0.1, 2.5 + 0.22));
    p.write(deck, "CntB.b2.h1", e);

    // CntB.b2.h2 - tile lines: 0.215 gaps (a square right of a bar) straddling x = 20
    // three ways, 21, 40, 42; a square above a bar across y = 20 at x = 30; a corner
    // 0.15/0.15 on (60, 20); a square 0.215 from a 300 µm bar at y = 60; (1000, 1000).
    let bs = |x: f64, y: f64| {
        let mut e = bar(x - 0.5, y, 0.5, 0.16);
        e.extend(sq(x + 0.215, y));
        e
    };
    let mut e = vec![];
    e.extend(bs(19.9, 2.0));
    e.extend(bs(20.0, 3.0));
    e.extend(bs(20.0 - 0.215, 4.0));
    e.extend(bs(20.9, 5.0));
    e.extend(bs(39.9, 2.0));
    e.extend(bs(41.9, 3.0));
    e.extend(bar(30.0, 19.9 - 0.16, 0.5, 0.16));
    e.extend(sq(30.1, 19.9 + 0.215));
    e.extend(bar(60.0 - 0.16, 20.0 - 0.5, 0.16, 0.5));
    e.extend(sq(60.0 + 0.15, 20.0 + 0.15));
    e.extend(bar(2.0, 60.0, 300.0, 0.16));
    e.extend(sq(150.0, 60.0 + 0.16 + 0.215));
    e.extend(bs(1000.0, 1000.0));
    p.write(deck, "CntB.b2.h2", e);

    // CntB.c/CntB.d - Activ / GatPoly enclosure of a 0.16 × 0.5 bar 0.07: the kit, the
    // inner box being the bar with Metal1 over it.
    let bare = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c, x0, y0, x1, y1),
            rect(p.m1, x0 - 0.05, y0 - 0.05, x1 + 0.05, y1 + 0.05),
        ]
    };
    let in_activ = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let mut e = bare(x0, y0, x1, y1);
        e.push(rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07));
        e
    };
    let in_poly = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let mut e = bare(x0, y0, x1, y1);
        e.push(rect(p.gp, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07));
        e
    };
    bar_enclosure_kit(
        p,
        deck,
        "CntB.c",
        0.07,
        &|pts| vec![poly(p.activ, pts)],
        &bare,
    );
    bar_enclosure_kit(p, deck, "CntB.d", 0.07, &|pts| vec![poly(p.gp, pts)], &bare);
    // CntB.g2 - pSD overlap of a bar on P+Activ 0.09: the kit, the bar on Activ.
    bar_enclosure_kit(
        p,
        deck,
        "CntB.g2",
        0.09,
        &|pts| vec![poly(p.psd, pts)],
        &in_activ,
    );
    // CntB.h1 - Metal1 enclosure of a bar 0.05: the kit, the bar on Activ with no Metal1
    // of its own.
    let on_activ_bare = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c, x0, y0, x1, y1),
            rect(p.activ, x0 - 0.07, y0 - 0.07, x1 + 0.07, y1 + 0.07),
        ]
    };
    bar_enclosure_kit(
        p,
        deck,
        "CntB.h1",
        0.05,
        &|pts| vec![poly(p.m1, pts)],
        &on_activ_bare,
    );

    // CntB.g2.h5 - the pSD edge through the middle of a bar on Activ: the covered half is
    // on P+Activ enclosed by 0 (CntB.g2), the bare half is on N+Activ 0 from pSD (CntB.g1).
    let mut e = in_activ(2.0, 2.0, 2.16, 2.5);
    e.push(rect(p.psd, 1.5, 1.5, 2.08, 3.0));
    p.write(deck, "CntB.g2.h5", e);

    // CntB.e - a bar on GatPoly to Activ 0.14: the kit, the bar on poly as the fixed box
    // (0.16 × 0.5), Activ the free shape; the abutting pair is a space of nothing.
    let gbar = |x0: f64, y0: f64, _x1: f64, _y1: f64| in_poly(x0, y0, x0 + 0.16, y0 + 0.5);
    space2_kit(
        p,
        deck,
        "CntB.e",
        0.14,
        &|pts| vec![poly(p.activ, pts)],
        1.0,
        &gbar,
        0.16,
        true,
    );
    // CntB.f - a bar on Activ to GatPoly 0.11.
    let abar = |x0: f64, y0: f64, _x1: f64, _y1: f64| in_activ(x0, y0, x0 + 0.16, y0 + 0.5);
    space2_kit(
        p,
        deck,
        "CntB.f",
        0.11,
        &|pts| vec![poly(p.gp, pts)],
        1.0,
        &abar,
        0.16,
        true,
    );
    // CntB.g1 - pSD to a bar on nSD-Activ 0.09: the bar on plain Activ (N+ by default).
    space2_kit(
        p,
        deck,
        "CntB.g1",
        0.09,
        &|pts| vec![poly(p.psd, pts)],
        1.0,
        &abar,
        0.16,
        true,
    );

    // CntB.g1.h5 - what is nSD-Activ.  pSD 0.085 from a bar on plain Activ (fires), on
    // Activ under drawn nSD (fires), on Activ under nSD:block (clean), on P+Activ (CntB.g2's
    // at 0.09, and the bar is enclosed by 0.2: clean).
    let mut e = vec![];
    for (k, imp) in [(0, None), (1, Some(p.nsd)), (2, Some(p.nsdb))] {
        let x = 2.0 + k as f64 * 2.0;
        e.extend(in_activ(x, 2.0, x + 0.16, 2.5));
        e.push(rect(p.psd, x + 0.16 + 0.085, 1.9, x + 1.0, 2.6));
        if let Some(l) = imp {
            e.push(rect(l, x - 0.1, 1.9, x + 0.2, 2.6));
        }
    }
    e.extend(in_activ(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.psd, 7.8, 1.8, 9.0, 2.7));
    p.write(deck, "CntB.g1.h5", e);

    // CntB.g.h1 - a bar must be within Activ or GatPoly.  A bare bar (fires); a bar 0.05
    // past the Activ edge (CntB.g and CntB.c); a bar half out of its GatPoly (CntB.g and
    // CntB.d); a bar straddling the seam of an abutting Activ and GatPoly (covered by
    // their union: no CntB.g; enclosed by neither: CntB.c and CntB.d; its Activ half 0 from
    // poly: CntB.f; its poly half 0 from Activ: CntB.e); a bar in an Activ ring's hole
    // (fires); a bar abutting Activ from outside (fires); a bar on Activ under GatPoly
    // (CntB.j, not g); one at (1000, 1000) (fires).
    let mut e = bare(2.0, 2.0, 2.16, 2.5);
    e.extend(bare(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.activ, 3.93, 1.93, 4.11, 2.57));
    e.extend(bare(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.gp, 5.93, 1.93, 6.08, 2.57));
    e.extend(bare(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.activ, 7.8, 1.8, 8.08, 2.7));
    e.push(rect(p.gp, 8.08, 1.8, 8.4, 2.7));
    e.extend(bare(10.0, 2.0, 10.16, 2.5));
    e.extend(ring(p.activ, 9.5, 1.5, 10.66, 3.0, 9.8, 1.8, 10.36, 2.7));
    e.extend(bare(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.activ, 12.16, 1.8, 12.7, 2.7));
    e.extend(in_poly(14.0, 2.0, 14.16, 2.5));
    e.push(rect(p.activ, 13.8, 1.8, 14.4, 2.7));
    e.extend(bare(1000.0, 1000.0, 1000.16, 1000.5));
    p.write(deck, "CntB.g.h1", e);

    // CntB.g.h2 - bare bars straddling x = 20, ending on 20, starting on 20, straddling
    // 21, 40, 42 and y = 20 (fires each); a bar straddling x = 20 whose Activ ends on the
    // line (CntB.g and CntB.c); a bar ending on x = 20 with its Activ ending there (within,
    // enclosed by 0: CntB.c).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        e.extend(bare(*x, 2.0 + k as f64, x + 0.5, 2.16 + k as f64));
    }
    e.extend(bare(30.0, 19.9, 30.16, 20.4));
    e.extend(bare(19.9, 10.0, 20.4, 10.16));
    e.push(rect(p.activ, 19.5, 9.5, 20.0, 10.7));
    e.extend(bare(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.activ, 19.2, 11.5, 20.0, 12.7));
    p.write(deck, "CntB.g.h2", e);

    // CntB.h.h1 - a bar must be covered with Metal1.  No Metal1 (fires); a 0.005 strip
    // uncovered (fires); half covered (CntB.h, and CntB.h1 for the covered half enclosed by
    // 0); in a Metal1 ring's hole (fires); Metal1 abutting from outside (fires); Metal1
    // coincident with the bar (covers; CntB.h1 at 0); Metal1 with 0.05 all round (clean);
    // Metal1 as two abutting halves and as two overlapping boxes (clean); a bare bar at
    // (1000, 1000) (fires).
    let mut e = on_activ_bare(2.0, 2.0, 2.16, 2.5);
    e.extend(on_activ_bare(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.m1, 3.95, 1.95, 4.155, 2.55));
    e.extend(on_activ_bare(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.m1, 5.95, 1.95, 6.08, 2.55));
    e.extend(on_activ_bare(8.0, 2.0, 8.16, 2.5));
    e.extend(ring(p.m1, 7.5, 1.5, 8.66, 3.0, 7.9, 1.9, 8.26, 2.6));
    e.extend(on_activ_bare(10.0, 2.0, 10.16, 2.5));
    e.push(rect(p.m1, 10.16, 1.9, 10.6, 2.6));
    e.extend(on_activ_bare(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.m1, 12.0, 2.0, 12.16, 2.5));
    e.extend(on_activ_bare(14.0, 2.0, 14.16, 2.5));
    e.push(rect(p.m1, 13.95, 1.95, 14.21, 2.55));
    e.extend(on_activ_bare(16.0, 2.0, 16.16, 2.5));
    e.push(rect(p.m1, 15.95, 1.95, 16.08, 2.55));
    e.push(rect(p.m1, 16.08, 1.95, 16.21, 2.55));
    e.extend(on_activ_bare(18.0, 2.0, 18.16, 2.5));
    e.push(rect(p.m1, 17.95, 1.95, 18.12, 2.55));
    e.push(rect(p.m1, 18.04, 1.95, 18.21, 2.55));
    e.extend(on_activ_bare(1000.0, 1000.0, 1000.16, 1000.5));
    p.write(deck, "CntB.h.h1", e);

    // CntB.h.h2 - bare bars on Activ straddling x = 20, ending on 20, starting on 20,
    // straddling 21, 40, 42 and y = 20 (fires each); a bar straddling x = 20 whose Metal1
    // ends on the line (CntB.h, and CntB.h1 for the covered part); Metal1 ending where the
    // bar ends on x = 20 covers (CntB.h1 at 0); two Metal1 boxes meeting on x = 20 under a
    // bar cover it (clean).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        e.extend(on_activ_bare(*x, 2.0 + k as f64, x + 0.5, 2.16 + k as f64));
    }
    e.extend(on_activ_bare(30.0, 19.9, 30.16, 20.4));
    e.extend(on_activ_bare(19.9, 10.0, 20.4, 10.16));
    e.push(rect(p.m1, 19.5, 9.9, 20.0, 10.26));
    e.extend(on_activ_bare(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.m1, 19.2, 11.9, 20.0, 12.26));
    e.extend(on_activ_bare(19.7, 14.0, 20.2, 14.16));
    e.push(rect(p.m1, 19.5, 13.9, 20.0, 14.26));
    e.push(rect(p.m1, 20.0, 13.9, 20.5, 14.26));
    p.write(deck, "CntB.h.h2", e);

    // CntB.j.h1 - a bar on GatPoly over Activ is not allowed.  A bar in a gate (fires); a
    // bar on poly overlapping Activ by a 0.005 strip (fires; the strip is a bar on Activ
    // enclosed by 0: CntB.c) and by a 0.005 × 0.005 corner (the same); a bar on poly over
    // two 0.05 Activ fingers (two overlap pieces: two CntB.j, and CntB.c for each); a bar
    // on poly abutting Activ (not over it: no CntB.j, but 0 from it: CntB.e); a bar on
    // poly 0.14 from Activ (clean); one in a gate at (1000, 1000) (fires).
    let mut e = in_poly(2.0, 2.0, 2.16, 2.5);
    e.push(rect(p.activ, 1.8, 1.8, 2.4, 2.7));
    e.extend(in_poly(4.0, 2.0, 4.16, 2.5));
    e.push(rect(p.activ, 4.155, 1.8, 4.6, 2.7));
    e.extend(in_poly(6.0, 2.0, 6.16, 2.5));
    e.push(rect(p.activ, 6.155, 2.495, 6.6, 2.9));
    e.extend(in_poly(8.0, 2.0, 8.16, 2.5));
    e.push(rect(p.activ, 7.8, 2.1, 8.4, 2.15));
    e.push(rect(p.activ, 7.8, 2.3, 8.4, 2.35));
    e.extend(in_poly(10.0, 2.0, 10.16, 2.5));
    e.push(rect(p.activ, 10.16, 1.8, 10.6, 2.7));
    e.extend(in_poly(12.0, 2.0, 12.16, 2.5));
    e.push(rect(p.activ, 12.3, 1.8, 12.7, 2.7));
    e.extend(in_poly(1000.0, 1000.0, 1000.16, 1000.5));
    e.push(rect(p.activ, 999.8, 999.8, 1000.4, 1000.7));
    p.write(deck, "CntB.j.h1", e);

    // CntB.j.h2 - gates straddling x = 20, ending on 20, starting on 20, straddling 21,
    // 40, 42 and y = 20 (fires each); a bar on poly whose Activ begins on x = 20 under its
    // right half (CntB.j, and CntB.c for the half on Activ); one whose Activ begins where
    // the bar ends on x = 20 abuts it (no CntB.j, CntB.e at 0).
    let mut e = vec![];
    for (k, x) in [19.9, 20.0 - 0.5, 20.0, 20.9, 39.9, 41.9]
        .iter()
        .enumerate()
    {
        let y = 2.0 + k as f64;
        e.extend(in_poly(*x, y, x + 0.5, y + 0.16));
        e.push(rect(p.activ, x - 0.2, y - 0.2, x + 0.7, y + 0.36));
    }
    e.extend(in_poly(30.0, 19.9, 30.16, 20.4));
    e.push(rect(p.activ, 29.8, 19.7, 30.36, 20.6));
    e.extend(in_poly(19.75, 10.0, 20.25, 10.16));
    e.push(rect(p.activ, 20.0, 9.8, 20.5, 10.36));
    e.extend(in_poly(19.5, 12.0, 20.0, 12.16));
    e.push(rect(p.activ, 20.0, 11.8, 20.5, 12.36));
    p.write(deck, "CntB.j.h2", e);

    // CntB.j.h3/h4 - fifty gate bars, flat and as an array.
    let mut cell = in_poly(0.2, 0.2, 0.7, 0.36);
    cell.push(rect(p.activ, 0.0, 0.0, 0.9, 0.56));
}

/// The enclosure kit for a 0.16 × 0.5 bar: `enclosure_kit` with the inner box a bar.
fn bar_enclosure_kit(p: &P, deck: &str, rule: &str, v: f64, outer: &Free<'_>, inner: &Boxes<'_>) {
    let d = v - G;
    let m = v + 0.5;
    let gap = 1.0;
    let (bw, bh) = (0.16, 0.5);
    let av = diag_under(v);
    let ac = diag_over(v);
    let obox =
        |x0: f64, y0: f64, x1: f64, y1: f64| outer(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
    let pair = |x: f64, y: f64, l: f64, r: f64, b: f64, t: f64| {
        let mut e = obox(x, y, x + l + bw + r, y + b + bh + t);
        e.extend(inner(x + l, y + b, x + l + bw, y + b + bh));
        e
    };

    // h1.  Margins v all round (clean); d on the left, right, bottom, top (one each); d
    // all round (one); the outer's corner chamfered av·√2 from the bar's corner with both
    // axis margins m (fires) and ac·√2 (clean); the bar half out of the outer (fires);
    // the bar's right edge on the outer's (fires).
    let y = 2.0;
    let mut x = 2.0;
    let mut e = pair(x, y, v, v, v, v);
    x += bw + 2.0 * v + gap;
    for (l, r, b, t) in [
        (d, v, v, v),
        (v, d, v, v),
        (v, v, d, v),
        (v, v, v, d),
        (d, d, d, d),
    ] {
        e.extend(pair(x, y, l, r, b, t));
        x += bw + l + r + gap;
    }
    for a in [av, ac] {
        let (ox, oy) = (bw + 2.0 * m, bh + 2.0 * m);
        let c = 2.0 * m - 2.0 * a;
        e.extend(outer(&[
            (x, y),
            (x + ox, y),
            (x + ox, y + oy - c),
            (x + ox - c, y + oy),
            (x, y + oy),
        ]));
        e.extend(inner(x + m, y + m, x + m + bw, y + m + bh));
        x += ox + gap;
    }
    let ox = bw + 2.0 * m;
    e.extend(obox(x, y, x + ox, y + bh + 2.0 * m));
    e.extend(inner(
        x + ox - bw / 2.0,
        y + m,
        x + ox + bw / 2.0,
        y + m + bh,
    ));
    x += ox + bw + gap;
    e.extend(pair(x, y, v, 0.0, v, v));
    p.write(deck, &format!("{rule}.h1"), e);

    // h2 - tile lines, as `enclosure_kit`: d right margins straddling x = 20 (three ways),
    // 21, 40, 42; a d top margin across y = 20 at x = 30; a corner on (60, 20); a bar d
    // from the top of a 300 µm strip at y = 60; (1000, 1000); a v margin across x = 20
    // is clean.
    let row = bh + 2.0 * v + gap;
    let mut e = vec![];
    e.extend(pair(19.9 - v - bw, 2.0, v, d, v, v));
    e.extend(pair(20.0 - d - v - bw, 2.0 + row, v, d, v, v));
    e.extend(pair(20.0 - v - bw, 2.0 + 2.0 * row, v, d, v, v));
    e.extend(pair(20.9 - v - bw, 2.0 + 3.0 * row, v, d, v, v));
    e.extend(pair(39.9 - v - bw, 2.0, v, d, v, v));
    e.extend(pair(41.9 - v - bw, 2.0 + row, v, d, v, v));
    e.extend(pair(30.0, 19.9 - v - bh, v, v, v, d));
    e.extend(pair(60.0 - v - bw, 20.0 - v - bh, v, d, v, d));
    e.extend(obox(2.0, 60.0, 302.0, 60.0 + bh + 2.0 * v));
    e.extend(inner(150.0, 60.0 + v + G, 150.0 + bw, 60.0 + v + bh + G));
    e.extend(pair(1000.0, 1000.0, v, d, v, v));
    e.extend(pair(19.9 - v - bw, 2.0 + 4.0 * row, v, v, v, v));
    p.write(deck, &format!("{rule}.h2"), e);
}
