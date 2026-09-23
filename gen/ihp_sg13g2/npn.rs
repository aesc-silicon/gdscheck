// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{
    chamfered_bl, chamfered_tr, diamond, layer, library, poly, rect, shift, text, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/npn";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    npn_ties(pdk);
    npn_emitters(pdk);

    hardening(pdk);
}

/// A rectangular ring centred at `(cx, cy)` with outer half-extent `oh` and width `w`.
fn ring(pdk: &PdkConfig, name: &str, cx: f64, cy: f64, oh: f64, w: f64) -> Vec<GdsElement> {
    let l = layer(pdk, name);
    let ih = oh - w;
    vec![
        rect(l, cx - oh, cy - oh, cx + oh, cy - ih), // bottom
        rect(l, cx - oh, cy + ih, cx + oh, cy + oh), // top
        rect(l, cx - oh, cy - ih, cx - ih, cy + ih), // left
        rect(l, cx + ih, cy - ih, cx + oh, cy + ih), // right
    ]
}

/// The core of a recognised npn device at `(cx, cy)`: a TRANS square (half-extent `th`),
/// its flavour text label, an emitter window `ww`×`wl` and the E-labelled Metal2 pin
/// (which KLayout's flavour recognition additionally requires; ours doesn't need it but
/// the fixture carries it so the container cross-check exercises the real rules).
fn core(
    pdk: &PdkConfig,
    cx: f64,
    cy: f64,
    th: f64,
    label: &str,
    ww: f64,
    wl: f64,
) -> Vec<GdsElement> {
    vec![
        rect(layer(pdk, "TRANS"), cx - th, cy - th, cx + th, cy + th),
        text(layer(pdk, "TEXT"), label, cx - th + 0.1, cy - th + 0.1),
        rect(
            layer(pdk, "EmWind"),
            cx - ww / 2.0,
            cy - wl / 2.0,
            cx + ww / 2.0,
            cy + wl / 2.0,
        ),
        rect(
            layer(pdk, "Metal2.pin"),
            cx - 0.1,
            cy - 0.1,
            cx + 0.1,
            cy + 0.1,
        ),
        text(layer(pdk, "TEXT"), "E", cx, cy),
    ]
}

/// Substrate-tie (npnG2.*) cases:
/// - clean device: pSD/Activ ring (0.20 margins), TRANS with 1.0 hole margin, 0.9 emitter.
/// - npnG2.b: an "npn*"-labelled ring with NO TRANS inside.
/// - npnG2.d + npnG2.e: an NWell 1.0 µm and a Cont 0.20 µm from the tie'd TRANS.
/// - npnG2.c: the ring's Activ enclosed by pSD by only 0.05 (< 0.20).
fn npn_ties(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e: Vec<GdsElement> = Vec::new();

    // Clean device.
    let (cx, cy) = (o, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.3, 0.6));
    e.extend(core(pdk, cx, cy, 1.5, "npn13G2", 0.07, 0.9));

    // npnG2.b: labelled tie, no TRANS.
    let (cx, cy) = (o + 30.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.3, 0.6));
    e.push(text(layer(pdk, "TEXT"), "npnCustom", cx, cy));

    // npnG2.d + npnG2.e: bigger hole so the offenders fit inside it.
    let (cx, cy) = (o + 60.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 4.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 4.3, 0.6));
    e.extend(core(pdk, cx, cy, 1.5, "npn13G2", 0.07, 0.9));
    e.push(rect(
        layer(pdk, "NWell"),
        cx + 2.5,
        cy - 1.0,
        cx + 3.3,
        cy + 1.0,
    )); // 1.00 < 1.21
    e.push(rect(
        layer(pdk, "Cont"),
        cx - 0.08,
        cy - 1.5 - 0.2 - 0.16,
        cx + 0.08,
        cy - 1.5 - 0.2,
    )); // 0.20 < 0.27

    // npnG2.c: Activ ring margins 0.05 (< 0.20) inside the pSD ring.  TRANS exactly
    // fills the hole (flush with the pSD ring's inner edge) because KLayout anchors
    // this rule on TRANS touching the ring — otherwise the container cross-check
    // would not exercise it.
    let (cx, cy) = (o + 90.0, o);
    e.extend(ring(pdk, "pSD", cx, cy, 3.5, 1.0));
    e.extend(ring(pdk, "Activ", cx, cy, 3.45, 0.9));
    e.extend(core(pdk, cx, cy, 2.5, "npn13G2", 0.07, 0.9));

    write_gz(&format!("{DIR}/npnG2.gds.gz"), library("TOP", e));
}

/// Emitter-length (npn13G2*) cases — one minimal device (TRANS + label + pin + window)
/// per case; the 0.8-long G2L window is also the exact-text canary (it must NOT draw
/// npn13G2.a, whose (0.07, 0.9) band it would fall into if "npn13G2" glob-matched it).
fn npn_emitters(pdk: &PdkConfig) {
    let o = OFFSET;
    let y = o + 30.0;
    let mut e: Vec<GdsElement> = Vec::new();
    e.extend(core(pdk, o, y, 1.5, "npn13G2", 0.07, 0.7)); // npn13G2.a (min)
    e.extend(core(pdk, o + 30.0, y, 1.5, "npn13G2", 0.07, 1.2)); // npn13G2.a (max; ours only)
    e.extend(core(pdk, o + 60.0, y, 1.5, "npn13G2L", 0.07, 0.8)); // npn13G2L.a + canary
    e.extend(core(pdk, o + 90.0, y, 2.0, "npn13G2L", 0.07, 3.0)); // npn13G2L.b
    e.extend(core(pdk, o + 120.0, y, 1.5, "npn13G2V", 0.12, 0.8)); // npn13G2V.a
    e.extend(core(pdk, o + 150.0, y, 3.5, "npn13G2V", 0.12, 6.0)); // npn13G2V.b
    e.extend(core(pdk, o + 180.0, y, 1.5, "npn13G2", 0.07, 0.9)); // clean G2
    write_gz(&format!("{DIR}/npn13G2.gds.gz"), library("TOP", e));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the bipolar and Schottky decks: section 6.1 (npnG2.b-e, the
// substrate tie of the npn13G2/L/V transistors, and npn13G2.a, npn13G2L.a/b,
// npn13G2V.a/b, the emitter lengths) and section 6.7 (Sdiod.a-e, the schottky_nbl1
// ContBar and its PWell:block, nSD:block and SalBlock) of the SG13G2 layout rules,
// with section 4.2's derived layers (N+Activ, the generated nBuLay).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//
// The devices are drawn as IHP's reference cells have them (`sg13g2_pr.gds`): the npn
// is a TRANS box filling the hole of a pSD ring 0.9 wide whose Activ ring 0.5 wide lies
// 0.2 inside the pSD on both edges (`tie`), its flavour label and emitter window in the
// TRANS (`core`); the Schottky is a 0.30 x 1.00 ContBar under PWell:block, nSD:block and
// SalBlock at 0.25, 0.40 and 0.45, in a drawn nBuLay, ringed by an NWell whose hole is
// the PWell:block, a PWell:block ring, a P+Activ tie ring, under ThickGateOx and
// Recog:diode (`schottky`).

/// One grid step.
pub(super) const G: f64 = 0.005;

pub(super) const SQRT2: f64 = std::f64::consts::SQRT_2;

pub(super) struct P {
    pub(super) trans: (i16, i16),
    pub(super) txt: (i16, i16),
    pub(super) emw: (i16, i16),
    pub(super) m2pin: (i16, i16),
    pub(super) psd: (i16, i16),
    pub(super) activ: (i16, i16),
    pub(super) nsd: (i16, i16),
    pub(super) nsdb: (i16, i16),
    pub(super) nw: (i16, i16),
    pub(super) pwb: (i16, i16),
    pub(super) nbl: (i16, i16),
    pub(super) nblb: (i16, i16),
    pub(super) sal: (i16, i16),
    pub(super) gp: (i16, i16),
    pub(super) sram: (i16, i16),
    pub(super) cont: (i16, i16),
    pub(super) m1: (i16, i16),
    pub(super) tgo: (i16, i16),
    pub(super) recog: (i16, i16),
}

impl P {
    pub(super) fn new(pdk: &PdkConfig) -> Self {
        P {
            trans: layer(pdk, "TRANS"),
            txt: layer(pdk, "TEXT"),
            emw: layer(pdk, "EmWind"),
            m2pin: layer(pdk, "Metal2.pin"),
            psd: layer(pdk, "pSD"),
            activ: layer(pdk, "Activ"),
            nsd: layer(pdk, "nSD"),
            nsdb: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            pwb: layer(pdk, "PWell.block"),
            nbl: layer(pdk, "nBuLay"),
            nblb: layer(pdk, "nBuLay.block"),
            sal: layer(pdk, "SalBlock"),
            gp: layer(pdk, "GatPoly"),
            sram: layer(pdk, "SRAM"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            tgo: layer(pdk, "ThickGateOx"),
            recog: layer(pdk, "Recog.diode"),
        }
    }
}

pub(super) fn out(dir: &str, name: &str, e: Vec<GdsElement>) {
    write_gz(&format!("{dir}/{name}.gds.gz"), library("TOP", e));
}

/// A box `[x0, y0, x1, y1]` grown by `m` on each side (left, bottom, right, top).
pub(super) fn grown(b: [f64; 4], m: [f64; 4]) -> [f64; 4] {
    [b[0] - m[0], b[1] - m[1], b[2] + m[2], b[3] + m[3]]
}

pub(super) fn bx(l: (i16, i16), b: [f64; 4]) -> GdsElement {
    rect(l, b[0], b[1], b[2], b[3])
}

/// The ring between the boxes `outer` and `inner`, as four overlapping boxes.
pub(super) fn ring_boxes(l: (i16, i16), outer: [f64; 4], inner: [f64; 4]) -> Vec<GdsElement> {
    vec![
        rect(l, outer[0], outer[1], outer[2], inner[1]),
        rect(l, outer[0], inner[3], outer[2], outer[3]),
        rect(l, outer[0], outer[1], inner[0], outer[3]),
        rect(l, inner[2], outer[1], outer[2], outer[3]),
    ]
}

/// A `w` x `l` box centred on `(cx, cy)`, its lower left corner snapped to the grid
/// (an odd multiple of the grid step centred on a grid point would put both edges off
/// it).
pub(super) fn boxc(cx: f64, cy: f64, w: f64, l: f64) -> [f64; 4] {
    let (x0, y0) = (grid(cx - w / 2.0), grid(cy - l / 2.0));
    [x0, y0, grid(x0 + w), grid(y0 + l)]
}

pub(super) fn grid(v: f64) -> f64 {
    (v / G).round() * G
}

const NPN: &str = "tests/data/ihp-sg13g2/npn";

/// The width of the tie's pSD ring, and the Activ ring's margin inside it (figure 6.2
/// and the reference cell: Activ 0.5 wide, 0.2 inside pSD on both edges).
const TIE_W: f64 = 0.9;

const TIE_M: f64 = 0.2;

/// The same ring as one polygon with a zero-width cut from the left wall into the hole.
fn ring_cut(l: (i16, i16), o: [f64; 4], i: [f64; 4]) -> GdsElement {
    let ym = (i[1] + i[3]) / 2.0;
    poly(
        l,
        &[
            (o[0], o[1]),
            (o[2], o[1]),
            (o[2], o[3]),
            (o[0], o[3]),
            (o[0], ym),
            (i[0], ym),
            (i[0], i[3]),
            (i[2], i[3]),
            (i[2], i[1]),
            (i[0], i[1]),
            (i[0], ym),
            (o[0], ym),
        ],
    )
}

/// An octagon: the box `b` with its four corners cut by `k` along both axes.
fn octagon(l: (i16, i16), b: [f64; 4], k: f64) -> GdsElement {
    poly(
        l,
        &[
            (b[0] + k, b[1]),
            (b[2] - k, b[1]),
            (b[2], b[1] + k),
            (b[2], b[3] - k),
            (b[2] - k, b[3]),
            (b[0] + k, b[3]),
            (b[0], b[3] - k),
            (b[0], b[1] + k),
        ],
    )
}

/// An octagonal ring as one polygon with a zero-width cut, the outer octagon `o` cut
/// by `ko`, the inner `i` by `ki`.
fn oct_ring(l: (i16, i16), o: [f64; 4], ko: f64, i: [f64; 4], ki: f64) -> GdsElement {
    let ym = (i[1] + i[3]) / 2.0;
    poly(
        l,
        &[
            (o[0], ym),
            (o[0], o[1] + ko),
            (o[0] + ko, o[1]),
            (o[2] - ko, o[1]),
            (o[2], o[1] + ko),
            (o[2], o[3] - ko),
            (o[2] - ko, o[3]),
            (o[0] + ko, o[3]),
            (o[0], o[3] - ko),
            (o[0], ym),
            (i[0], ym),
            (i[0], i[3] - ki),
            (i[0] + ki, i[3]),
            (i[2] - ki, i[3]),
            (i[2], i[3] - ki),
            (i[2], i[1] + ki),
            (i[2] - ki, i[1]),
            (i[0] + ki, i[1]),
            (i[0], i[1] + ki),
            (i[0], ym),
        ],
    )
}

/// The substrate tie around the hole `h`: a pSD ring 0.9 wide hugging the hole, its
/// Activ ring from `mi[side]` outside the hole to `mo[side]` inside the pSD's outer
/// edge (0.2 both in the reference cell), as four boxes each.
fn tie(p: &P, h: [f64; 4], mi: [f64; 4], mo: [f64; 4]) -> Vec<GdsElement> {
    let o = grown(h, [TIE_W; 4]);
    let mut e = ring_boxes(p.psd, o, h);
    e.extend(ring_boxes(
        p.activ,
        [o[0] + mo[0], o[1] + mo[1], o[2] - mo[2], o[3] - mo[3]],
        grown(h, mi),
    ));
    e
}

/// The reference tie: 0.2 margins all round.
fn tie_ref(p: &P, h: [f64; 4]) -> Vec<GdsElement> {
    tie(p, h, [TIE_M; 4], [TIE_M; 4])
}

/// The core_h of an npn: the TRANS box `t`, its flavour `label` 0.1 inside its lower
/// left corner (which labels the tie's hole too), the emitter windows, and the
/// E-labelled Metal2 pin KLayout's flavour recognition wants.
fn core_h(p: &P, t: [f64; 4], label: &str, windows: &[[f64; 4]]) -> Vec<GdsElement> {
    let (cx, cy) = ((t[0] + t[2]) / 2.0, (t[1] + t[3]) / 2.0);
    let mut e = vec![
        bx(p.trans, t),
        text(p.txt, label, t[0] + 0.1, t[1] + 0.1),
        rect(p.m2pin, cx - 0.1, cy - 0.1, cx + 0.1, cy + 0.1),
        text(p.txt, "E", cx, cy),
    ];
    for w in windows {
        e.push(bx(p.emw, *w));
    }
    e
}

/// A vertical emitter window `w` x `l` centred on `(cx, cy)`.
fn window(cx: f64, cy: f64, w: f64, l: f64) -> [f64; 4] {
    boxc(cx, cy, w, l)
}

/// A TRANS `th` half-extent square at `(cx, cy)`.
fn tbox(cx: f64, cy: f64, th: f64) -> [f64; 4] {
    [cx - th, cy - th, cx + th, cy + th]
}

/// A whole npn13G2 at `(cx, cy)`: TRANS half-extent `th`, the tie's hole `hm` beyond
/// the TRANS (0 is the reference cell: the TRANS fills the hole), one 0.07 x 0.9
/// window.
fn npn(p: &P, cx: f64, cy: f64, th: f64, hm: f64) -> Vec<GdsElement> {
    let t = tbox(cx, cy, th);
    let mut e = tie_ref(p, grown(t, [hm; 4]));
    e.extend(core_h(p, t, "npn13G2", &[window(cx, cy, 0.07, 0.9)]));
    e
}

/// The reference npn: a TRANS 2.45 half-extent filling its hole.
const REF_TH: f64 = 2.45;

fn npn_g2_b(p: &P) {
    // h1: labelled ties with no TRANS: a ring of four boxes, one polygon with a cut,
    // an octagonal ring, a ring with a P+ island in its hole; an unlabelled empty ring
    // and a labelled ring with its TRANS are clean.  Four fire.
    let mut e = vec![];
    let h = [-2.45, -2.45, 2.45, 2.45];
    let mut at = |dx: f64, dy: f64, el: Vec<GdsElement>| {
        e.extend(shift(&el, dx, dy));
    };
    let mut ring = tie_ref(p, h);
    ring.push(text(p.txt, "npnCustom", 0.0, 0.0));
    at(10.0, 10.0, ring);
    let o = grown(h, [TIE_W; 4]);
    at(
        24.0,
        10.0,
        vec![
            ring_cut(p.psd, o, h),
            ring_cut(p.activ, grown(o, [-TIE_M; 4]), grown(h, [TIE_M; 4])),
            text(p.txt, "npn13G2", 0.0, 0.0),
        ],
    );
    at(
        38.0,
        10.0,
        vec![
            oct_ring(p.psd, o, 1.0, h, 0.6),
            oct_ring(
                p.activ,
                grown(o, [-TIE_M; 4]),
                grid(1.0 - 2.0 * TIE_M + TIE_M * SQRT2),
                grown(h, [TIE_M; 4]),
                grid(0.6 + 2.0 * TIE_M - TIE_M * SQRT2),
            ),
            text(p.txt, "npnX", 0.0, 0.0),
        ],
    );
    let mut island = tie_ref(p, h);
    island.push(rect(p.activ, -0.5, -0.5, 0.5, 0.5));
    island.push(rect(p.psd, -0.7, -0.7, 0.7, 0.7));
    island.push(text(p.txt, "npn13G2", -1.5, -1.5));
    at(52.0, 10.0, island);
    let mut plain = tie_ref(p, h);
    plain.push(text(p.txt, "tie", 0.0, 0.0));
    at(10.0, 24.0, plain);
    at(24.0, 24.0, npn(p, 0.0, 0.0, REF_TH, 0.0));
    out(NPN, "npnG2.b.h1", e);

    // h2: ties that do not enclose their TRANS: the TRANS crossing the ring (half in
    // the hole, half over the ring and beyond), a labelled TRANS with no tie at all,
    // a TRANS in a ring with a 0.5 gap in it, a TRANS in a pSD-only ring (no Activ:
    // no tie by npnG2.a).  The manual's "must enclose" fires four times; both tools
    // read only the labelled holes without a TRANS and are silent.
    let mut e = vec![];
    let mut at = |dx: f64, dy: f64, el: Vec<GdsElement>| {
        e.extend(shift(&el, dx, dy));
    };
    let mut crossing = tie_ref(p, h);
    crossing.extend(core_h(
        p,
        [0.0, -1.5, 4.5, 1.5],
        "npn13G2",
        &[window(2.25, 0.0, 0.07, 0.9)],
    ));
    at(10.0, 10.0, crossing);
    at(
        24.0,
        10.0,
        core_h(
            p,
            tbox(0.0, 0.0, 1.5),
            "npn13G2",
            &[window(0.0, 0.0, 0.07, 0.9)],
        ),
    );
    // The open ring: both rings' right walls with 0.5 cut out of them.
    let mut open: Vec<GdsElement> = Vec::new();
    let o = grown(h, [TIE_W; 4]);
    for (l, oo, ii) in [
        (p.psd, o, h),
        (p.activ, grown(o, [-TIE_M; 4]), grown(h, [TIE_M; 4])),
    ] {
        open.push(rect(l, oo[0], oo[1], oo[2], ii[1]));
        open.push(rect(l, oo[0], ii[3], oo[2], oo[3]));
        open.push(rect(l, oo[0], oo[1], ii[0], oo[3]));
        open.push(rect(l, ii[2], oo[1], oo[2], -0.25));
        open.push(rect(l, ii[2], 0.25, oo[2], oo[3]));
    }
    open.extend(core_h(
        p,
        tbox(0.0, 0.0, 1.5),
        "npn13G2",
        &[window(0.0, 0.0, 0.07, 0.9)],
    ));
    at(38.0, 10.0, open);
    let mut psd_only = ring_boxes(p.psd, o, h);
    psd_only.extend(core_h(
        p,
        tbox(0.0, 0.0, 1.5),
        "npn13G2",
        &[window(0.0, 0.0, 0.07, 0.9)],
    ));
    at(52.0, 10.0, psd_only);
    out(NPN, "npnG2.b.h2", e);

    // h3: empty labelled rings on the tile lines - a hole straddling x = 20, a hole's
    // wall on x = 21, the ring's outer edge on x = 40, the hole's wall on x = 42, a
    // hole straddling x = 100 - and one at (1000, 1000).  Six.
    let mut e = vec![];
    for (x0, y0) in [
        (20.0 - 2.45, 10.0),
        (21.0, 30.0),
        (40.0 - 2.45 - TIE_W, 50.0),
        (42.0 - 4.9, 70.0),
        (100.0 - 1.0, 10.0),
        (1000.0, 1000.0),
    ] {
        let hh = [x0, y0, x0 + 4.9, y0 + 4.9];
        e.extend(tie_ref(p, hh));
        e.push(text(p.txt, "npnCustom", x0 + 2.0, y0 + 2.0));
    }
    out(NPN, "npnG2.b.h3", e);
}

/// A reference-style npn (TRANS = hole) at `(cx, cy)` with the tie's margins given.
fn npn_margins(p: &P, cx: f64, cy: f64, mi: [f64; 4], mo: [f64; 4]) -> Vec<GdsElement> {
    let t = tbox(cx, cy, REF_TH);
    let mut e = tie(p, t, mi, mo);
    e.extend(core_h(p, t, "npn13G2", &[window(cx, cy, 0.07, 0.9)]));
    e
}

/// An octagonal reference npn at `(cx, cy)`: the pSD ring's outer octagon cut by
/// 1.5, its hole (and the TRANS) by 0.6, the Activ ring 0.2 inside both on the straight
/// walls and `d` inside the outer diagonal walls (0.2 inside the inner ones).  Two
/// diagonals `a` apart (perpendicular) lie `a*sqrt2` apart in x + y.
fn npn_oct(p: &P, cx: f64, cy: f64, d: f64) -> Vec<GdsElement> {
    let m = TIE_M;
    let t = tbox(cx, cy, REF_TH);
    let o = grown(t, [TIE_W; 4]);
    let (ko, ki) = (1.5, 0.6);
    let ka = grid(ko - 2.0 * m + d * SQRT2);
    let kai = grid(ki + 2.0 * m - m * SQRT2);
    vec![
        oct_ring(p.psd, o, ko, t, ki),
        oct_ring(p.activ, grown(o, [-m; 4]), ka, grown(t, [m; 4]), kai),
        octagon(p.trans, t, ki),
        text(p.txt, "npn13G2", t[0] + 0.8, t[1] + 0.1),
        rect(p.m2pin, cx - 0.1, cy - 0.1, cx + 0.1, cy + 0.1),
        text(p.txt, "E", cx, cy),
        bx(p.emw, window(cx, cy, 0.07, 0.9)),
    ]
}

fn npn_g2_c(p: &P) {
    let m = TIE_M;
    let u = TIE_M - G;
    // h1: the bound.  Outer margin 0.195 on the right wall; hole-side margin 0.195
    // on the top wall; outer margin 0.195 all round (one closed run); 0.2 all round;
    // 0.2 with the Activ ring as one cut polygon; an octagonal tie whose diagonal
    // Activ wall is 0.195 (perpendicular) from the pSD's; the same at 0.2; a pSD
    // outer corner chamfered so it passes 0.195 from the Activ's corner (the
    // euclidian reading, settled).  Four fire.
    let mut e = vec![];
    e.extend(npn_margins(p, 10.0, 10.0, [m; 4], [m, m, u, m]));
    e.extend(npn_margins(p, 24.0, 10.0, [m, m, m, u], [m; 4]));
    e.extend(npn_margins(p, 38.0, 10.0, [m; 4], [u; 4]));
    e.extend(npn_margins(p, 52.0, 10.0, [m; 4], [m; 4]));
    {
        let t = tbox(66.0, 10.0, REF_TH);
        let o = grown(t, [TIE_W; 4]);
        e.extend(ring_boxes(p.psd, o, t));
        e.push(ring_cut(p.activ, grown(o, [-m; 4]), grown(t, [m; 4])));
        e.extend(core_h(p, t, "npn13G2", &[window(66.0, 10.0, 0.07, 0.9)]));
    }
    for (i, d) in [(0, u), (1, m)] {
        let (cx, cy) = (10.0 + 14.0 * i as f64, 26.0);
        e.extend(npn_oct(p, cx, cy, d));
    }
    {
        // The pSD's outer top-right corner chamfered along x + y = k so the chamfer
        // passes 0.195 from the Activ's corner (ax, ay): (k - ax - ay)/sqrt2 = 0.195.
        // The pSD ring is its left, bottom and right walls as boxes and the top wall
        // as a chamfered box.
        let (cx, cy) = (38.0, 26.0);
        let t = tbox(cx, cy, REF_TH);
        let o = grown(t, [TIE_W; 4]);
        let (ax, ay) = (o[2] - m, o[3] - m);
        let k = grid(ax + ay + u * SQRT2);
        e.push(rect(p.psd, o[0], o[1], t[0], o[3]));
        e.push(rect(p.psd, o[0], o[1], o[2], t[1]));
        e.push(rect(p.psd, t[2], o[1], o[2], t[3]));
        e.push(chamfered_tr(p.psd, o[0], t[3], o[2], o[3], k));
        e.extend(ring_boxes(p.activ, grown(o, [-m; 4]), grown(t, [m; 4])));
        e.extend(core_h(p, t, "npn13G2", &[window(cx, cy, 0.07, 0.9)]));
    }
    out(NPN, "npnG2.c.h1", e);

    // h2: the Activ ring flush with the pSD's hole edge (enclosure 0 on the hole
    // side); the Activ ring sticking 0.1 out of the pSD's outer edge on the right
    // (the P+ part ends on the pSD edge, enclosure 0); a P+ island 0.45 from the
    // hole's wall with 0.19 pSD around it (the tie's Activ is the ring - not this,
    // and pSD.c is content at 0.19).  The manual: the first two fire.
    let mut e = vec![];
    e.extend(npn_margins(p, 10.0, 10.0, [0.0; 4], [m; 4]));
    e.extend(npn_margins(p, 24.0, 10.0, [m; 4], [m, m, -0.1, m]));
    {
        let (cx, cy) = (38.0, 10.0);
        let th = 1.5;
        let t = tbox(cx, cy, th);
        let h = grown(t, [2.5; 4]);
        e.extend(tie_ref(p, h));
        e.extend(core_h(p, t, "npn13G2", &[window(cx, cy, 0.07, 0.9)]));
        let ix = h[2] - 0.45;
        e.push(rect(p.activ, ix - 0.5, cy - 0.3, ix, cy + 0.3));
        e.push(rect(p.psd, ix - 0.69, cy - 0.49, ix + 0.19, cy + 0.49));
    }
    out(NPN, "npnG2.c.h2", e);

    // h3: the 0.195 outer margin on the tile lines: the gap straddling x = 20 (pSD
    // edge 20.1, Activ 19.905), the Activ's wall on x = 21, the gap straddling
    // x = 40, the pSD's wall on x = 42, the gap straddling x = 100, and at
    // (1000, 1000).  Six.
    let mut e = vec![];
    for (i, xr) in [20.1, 21.0 + u, 40.1, 42.0, 100.1, 1000.0]
        .iter()
        .enumerate()
    {
        let cx = xr - TIE_W - REF_TH;
        let cy = if i == 5 {
            1000.0
        } else {
            10.0 + 14.0 * i as f64
        };
        e.extend(npn_margins(p, cx, cy, [m; 4], [m, m, u, m]));
    }
    out(NPN, "npnG2.c.h3", e);
}

/// A big-hole npn at `(cx, cy)`: TRANS 1.5 half-extent, the hole `hm` beyond it.
fn npn_room(p: &P, cx: f64, cy: f64, hm: f64) -> Vec<GdsElement> {
    npn(p, cx, cy, 1.5, hm)
}

/// A box `w` x `h` whose left wall is `gap` right of the TRANS's right wall (x = tx),
/// centred on `cy`.
fn beside(l: (i16, i16), tx: f64, cy: f64, gap: f64, w: f64, h: f64) -> GdsElement {
    rect(l, tx + gap, cy - h / 2.0, tx + gap + w, cy + h / 2.0)
}

fn npn_g2_d(p: &P) {
    let s = 1.21;
    let u = s - G;
    // h1: the bound, one device per layer: N+Activ (bare Activ), NWell, PWell:block,
    // nBuLay, nSD:block at 1.205 on the right and 1.21 on the left of the TRANS.
    // Five.
    let mut e = vec![];
    for (i, l) in [p.activ, p.nw, p.pwb, p.nbl, p.nsdb].iter().enumerate() {
        let (cx, cy) = (10.0 + 14.0 * i as f64, 10.0);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(*l, cx + 1.5, cy, u, 0.5, 1.0));
        e.push(rect(
            *l,
            cx - 1.5 - s - 0.5,
            cy - 0.5,
            cx - 1.5 - s,
            cy + 0.5,
        ));
    }
    out(NPN, "npnG2.d.h1", e);

    // h2: the conditions.  P+Activ (pSD over it) at 1.0: not listed, clean.  Activ
    // under an nSD:block abutting the TRANS (related) with no nSD, at 1.0: the Activ
    // is no N+ (section 4.2), clean.  The same with nSD drawn on the Activ: N+,
    // fires.  N+Activ abutting the TRANS and NWell overlapping it: related, clean.
    // N+Activ touching the TRANS at a corner point: related, clean.  All five layers
    // at 1.0 from a TRANS whose hole has no label: no tie, clean.  An nSD:block in
    // the hole at 1.0 (KLayout exempts blocks in the hole): fires.  An nSD:block
    // outside a reference tie, 0.305 past the pSD (1.205 from the TRANS): fires.
    // Three.
    let mut e = vec![];
    let row = |i: usize| (10.0 + 14.0 * i as f64, 10.0);
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.activ, cx + 1.5, cy, 1.0, 0.5, 1.0));
        e.push(beside(p.psd, cx + 1.5, cy, 0.8, 0.9, 1.4));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.activ, cx + 1.5, cy, 1.0, 0.3, 1.0));
        e.push(beside(p.nsdb, cx + 1.5, cy, 0.0, 1.5, 1.4));
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.activ, cx + 1.5, cy, 1.0, 0.3, 1.0));
        e.push(beside(p.nsd, cx + 1.5, cy, 1.0, 0.3, 1.0));
        e.push(beside(p.nsdb, cx + 1.5, cy, 0.0, 1.5, 1.4));
    }
    {
        let (cx, cy) = row(3);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.activ, cx + 1.5, cy, 0.0, 0.5, 1.0));
        e.push(rect(p.nw, cx - 2.5, cy - 0.5, cx - 1.0, cy + 0.5));
        e.push(rect(p.activ, cx + 1.5, cy + 1.5, cx + 2.5, cy + 2.5));
    }
    {
        let (cx, cy) = row(4);
        let t = tbox(cx, cy, 1.5);
        e.extend(tie_ref(p, grown(t, [3.0; 4])));
        e.push(bx(p.trans, t));
        e.push(bx(p.emw, window(cx, cy, 0.07, 0.9)));
        for (j, l) in [p.activ, p.nw, p.pwb, p.nbl, p.nsdb].iter().enumerate() {
            let y = cy - 2.0 + j as f64 * 1.0;
            e.push(rect(*l, cx + 2.5, y, cx + 3.0, y + 0.5));
        }
    }
    {
        let (cx, cy) = row(5);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.nsdb, cx + 1.5, cy, 1.0, 0.5, 1.0));
    }
    {
        let (cx, cy) = row(6);
        e.extend(npn(p, cx, cy, REF_TH, 0.0));
        e.push(beside(p.nsdb, cx + REF_TH, cy, u, 0.5, 1.0));
    }
    out(NPN, "npnG2.d.h2", e);

    // h3: the metrics and 45°.  An N+Activ corner 0.85/0.85 from the TRANS corner
    // (euclidian 1.202, fires); 0.86/0.86 (1.216, clean); an N+Activ with a chamfered
    // corner passing 1.205 from the TRANS corner (fires); a diamond N+Activ whose tip
    // is 1.205 from the wall (fires); a 45° NWell strip 1.205 (perpendicular) from a
    // wall (fires); a PWell:block 1.205 in x but 2.0 in y past the corner (clean).
    // Four.
    let mut e = vec![];
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.5));
        let (tx, ty) = (cx + 1.5, cy + 1.5);
        e.push(rect(p.activ, tx + 0.85, ty + 0.85, tx + 1.85, ty + 1.85));
        e.push(rect(
            p.activ,
            tx + 0.86,
            cy - 1.5 - 1.86,
            tx + 1.86,
            cy - 1.5 - 0.86,
        ));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.5));
        let (tx, ty) = (cx + 1.5, cy + 1.5);
        // Chamfer along x + y = k through the box's lower-left corner region: the
        // TRANS corner (tx, ty) is (k - tx - ty)/sqrt2 from the line.
        let k = grid(tx + ty + u * SQRT2);
        e.push(chamfered_bl(
            p.activ,
            tx + 0.3,
            ty + 0.3,
            tx + 2.0,
            ty + 2.0,
            k,
        ));
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 3.5));
        e.push(diamond(p.activ, cx + 1.5 + u + 0.6, cy, 0.6));
    }
    {
        let (cx, cy) = row(3);
        e.extend(npn_room(p, cx, cy, 3.5));
        // A 45° NWell strip whose lower-left wall, x + y = tx + ty + a, passes the
        // TRANS's top-right corner (tx, ty) at the perpendicular distance a/sqrt2 = u.
        let (tx, ty) = (cx + 1.5, cy + 1.5);
        let a = grid(u * SQRT2);
        e.push(poly(
            p.nw,
            &[
                (tx + a, ty),
                (tx + a + 0.3, ty),
                (tx + a + 0.3 - 1.5, ty + 1.5),
                (tx + a - 1.5, ty + 1.5),
            ],
        ));
    }
    {
        let (cx, cy) = row(4);
        e.extend(npn_room(p, cx, cy, 3.5));
        e.push(rect(
            p.pwb,
            cx + 1.5 + u,
            cy + 1.5 + 2.0,
            cx + 1.5 + u + 0.5,
            cy + 1.5 + 2.5,
        ));
    }
    out(NPN, "npnG2.d.h3", e);

    // h4: shapes that merge and multi-piece neighbours.  An N+Activ of two
    // overlapping boxes 1.205 away (one); an N+Activ comb whose three teeth face the
    // TRANS at 1.205 (three walls); an NWell ring in the hole around the TRANS at
    // 1.205 on all four sides (four walls, one shape); two ties sharing a wall
    // (npnG2.f) with an NWell 1.205 from one TRANS and 3.0 from the other (one).
    let mut e = vec![];
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.5));
        e.push(beside(p.activ, cx + 1.5, cy, u, 0.6, 0.6));
        e.push(beside(p.activ, cx + 1.5, cy + 0.4, u, 0.6, 0.6));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.5));
        let x = cx + 1.5 + u;
        e.push(rect(p.activ, x + 0.5, cy - 1.2, x + 1.0, cy + 1.2));
        for k in 0..3 {
            let y = cy - 1.2 + k as f64 * 1.0;
            e.push(rect(p.activ, x, y, x + 0.5, y + 0.4));
        }
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 4.0));
        let t = tbox(cx, cy, 1.5);
        e.extend(ring_boxes(p.nw, grown(t, [u + 0.5; 4]), grown(t, [u; 4])));
    }
    {
        let (cx, cy) = (10.0 + 14.0 * 3.0 + 4.0, 10.0);
        let ta = tbox(cx, cy, 1.5);
        let tb = tbox(cx + 12.0, cy, 1.5);
        let ha = grown(ta, [3.5; 4]);
        let hb = grown(tb, [3.5; 4]);
        // The shared wall: hole A ends at cx + 5.0, hole B starts at cx + 7.0; the
        // pSD between them is one 2.0 wide wall (0.9 + 0.9 overlapping).
        e.extend(tie_ref(p, ha));
        e.extend(tie_ref(p, hb));
        e.extend(core_h(p, ta, "npn13G2", &[window(cx, cy, 0.07, 0.9)]));
        e.extend(core_h(
            p,
            tb,
            "npn13G2",
            &[window(cx + 12.0, cy, 0.07, 0.9)],
        ));
        e.push(beside(p.nw, cx + 1.5, cy, u, 0.5, 1.0));
    }
    out(NPN, "npnG2.d.h4", e);

    // h5: the tile lines: the 1.205 gap straddling x = 20 (TRANS wall at 19.4), the
    // TRANS wall on x = 21, the N+Activ's wall on x = 40, the gap straddling x = 42,
    // the gap straddling x = 100, and at (1000, 1000).  Six.
    let mut e = vec![];
    for (i, tx) in [19.4, 21.0, 40.0 - u, 41.4, 99.4, 1000.0]
        .iter()
        .enumerate()
    {
        let cx = tx - 1.5;
        let cy = if i == 5 {
            1000.0
        } else {
            10.0 + 14.0 * i as f64
        };
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.activ, *tx, cy, u, 0.5, 1.0));
    }
    out(NPN, "npnG2.d.h5", e);

    // h8: section 4.2's generated nBuLay.  A 4.0 wide NWell overlapping the TRANS by
    // 0.5 (the well is related; its generated nBuLay, 1.0 inside the well, lies 0.5
    // outside the TRANS: unrelated, fires); the same well under nBuLay:block (clean);
    // a 2.0 wide well overlapping the TRANS (no generated nBuLay, clean); the 4.0 well
    // with a drawn nBuLay overlapping the TRANS (one region, related, clean).  One.
    let mut e = vec![];
    for (i, kind) in [0, 1, 2, 3].iter().enumerate() {
        let (cx, cy) = (10.0 + 16.0 * i as f64, 10.0);
        e.extend(npn_room(p, cx, cy, 5.0));
        let tx = cx + 1.5;
        let w = if *kind == 2 { 2.0 } else { 4.0 };
        e.push(rect(
            p.nw,
            tx - 0.5,
            cy - w / 2.0,
            tx - 0.5 + w,
            cy + w / 2.0,
        ));
        if *kind == 1 {
            e.push(rect(p.nblb, tx - 1.0, cy - 2.5, tx + 4.0, cy + 2.5));
        }
        if *kind == 3 {
            e.push(rect(p.nbl, tx - 0.5, cy - 1.0, tx + 3.5, cy + 1.0));
        }
    }
    out(NPN, "npnG2.d.h8", e);
}

fn npn_g2_d1_d2(p: &P) {
    let s = 0.9;
    let u = s - G;
    // h1 (d1): GatPoly 0.895 (fires) and 0.9 (clean); GatPoly 0.5 away inside an SRAM
    // marker (clean); GatPoly crossing the SRAM's edge, the part outside at 0.5
    // (fires); GatPoly abutting the TRANS (related, clean); a 45° GatPoly strip
    // 0.895 perpendicular from the wall (fires); corner to corner 0.63/0.63 (0.891,
    // fires) and 0.64/0.64 (0.905, clean); GatPoly 0.895 from a TRANS whose hole has
    // no label (clean).  Four.
    let mut e = vec![];
    let row = |i: usize| (10.0 + 14.0 * i as f64, 10.0);
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.gp, cx + 1.5, cy, u, 0.5, 1.0));
        e.push(rect(
            p.gp,
            cx - 1.5 - s - 0.5,
            cy - 0.5,
            cx - 1.5 - s,
            cy + 0.5,
        ));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.gp, cx + 1.5, cy, 0.5, 0.5, 1.0));
        e.push(beside(p.sram, cx + 1.5, cy, 0.3, 1.0, 1.4));
        e.push(rect(
            p.gp,
            cx - 1.5 - 1.5,
            cy - 0.5,
            cx - 1.5 - 0.5,
            cy + 0.5,
        ));
        e.push(rect(
            p.sram,
            cx - 1.5 - 1.7,
            cy - 0.7,
            cx - 1.5 - 0.7,
            cy + 0.7,
        ));
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.gp, cx + 1.5, cy, 0.0, 0.5, 1.0));
        let (tx, ty) = (cx - 1.5, cy + 1.5);
        let a = grid(u * SQRT2);
        e.push(poly(
            p.gp,
            &[
                (tx - a, ty),
                (tx - a - 0.3, ty),
                (tx - a - 0.3 + 1.2, ty + 1.2),
                (tx - a + 1.2, ty + 1.2),
            ],
        ));
    }
    {
        let (cx, cy) = row(3);
        e.extend(npn_room(p, cx, cy, 3.0));
        let (tx, ty) = (cx + 1.5, cy + 1.5);
        e.push(rect(p.gp, tx + 0.63, ty + 0.63, tx + 1.3, ty + 1.3));
        e.push(rect(
            p.gp,
            tx + 0.64,
            cy - 1.5 - 1.3,
            tx + 1.3,
            cy - 1.5 - 0.64,
        ));
    }
    {
        let (cx, cy) = row(4);
        let t = tbox(cx, cy, 1.5);
        e.extend(tie_ref(p, grown(t, [3.0; 4])));
        e.push(bx(p.trans, t));
        e.push(beside(p.gp, cx + 1.5, cy, u, 0.5, 1.0));
    }
    out(NPN, "npnG2.d1.h1", e);

    // h2 (d2): SalBlock 0.895 (fires) and 0.9 (clean); SalBlock abutting (clean);
    // SalBlock over the TRANS (clean); a SalBlock U whose two arms face the TRANS's
    // top and bottom at 0.895 (two).  Three.
    let mut e = vec![];
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.sal, cx + 1.5, cy, u, 0.5, 1.0));
        e.push(rect(
            p.sal,
            cx - 1.5 - s - 0.5,
            cy - 0.5,
            cx - 1.5 - s,
            cy + 0.5,
        ));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.sal, cx + 1.5, cy, 0.0, 0.5, 1.0));
        e.push(rect(p.sal, cx - 0.5, cy - 0.3, cx + 0.5, cy + 0.3));
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 3.5));
        let (x0, x1) = (cx - 1.0, cx + 3.4);
        e.push(rect(p.sal, x0, cy + 1.5 + u, x1, cy + 1.5 + u + 0.4));
        e.push(rect(p.sal, x0, cy - 1.5 - u - 0.4, x1, cy - 1.5 - u));
        e.push(rect(p.sal, x1 - 0.4, cy - 1.5 - u, x1, cy + 1.5 + u));
    }
    out(NPN, "npnG2.d2.h1", e);

    // h3: the tile lines for d1: the 0.895 gap straddling x = 20 (TRANS wall at
    // 19.5), the TRANS wall on x = 21, the GatPoly's wall on x = 40, the gap straddling
    // x = 42, at x = 100 and (1000, 1000).  Six.
    let mut e = vec![];
    for (i, tx) in [19.5, 21.0, 40.0 - u, 41.5, 99.5, 1000.0]
        .iter()
        .enumerate()
    {
        let cx = tx - 1.5;
        let cy = if i == 5 {
            1000.0
        } else {
            10.0 + 14.0 * i as f64
        };
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.gp, *tx, cy, u, 0.5, 1.0));
    }
    out(NPN, "npnG2.d1.h2", e);
}

fn npn_g2_e(p: &P) {
    let s = 0.27;
    let u = s - G;
    let c = 0.16;
    // h1: a Cont 0.265 (fires) and 0.27 (clean); a Cont abutting the TRANS and one
    // inside it (related, clean); corner to corner 0.185/0.185 (0.262, fires) and
    // 0.195/0.195 (0.276, clean); a 0.16 x 0.5 ContBar 0.265 (fires); the tie's own
    // Cont in the reference tie's Activ 0.07 inside its wall (0.27 from the TRANS,
    // clean); a Cont 0.265 from an unlabelled device (clean).  Three.
    let mut e = vec![];
    let row = |i: usize| (10.0 + 14.0 * i as f64, 10.0);
    {
        let (cx, cy) = row(0);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.cont, cx + 1.5, cy, u, c, c));
        e.push(rect(
            p.cont,
            cx - 1.5 - s - c,
            cy - c / 2.0,
            cx - 1.5 - s,
            cy + c / 2.0,
        ));
    }
    {
        let (cx, cy) = row(1);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.cont, cx + 1.5, cy, 0.0, c, c));
        e.push(rect(p.cont, cx - 0.5, cy + 0.5, cx - 0.5 + c, cy + 0.5 + c));
    }
    {
        let (cx, cy) = row(2);
        e.extend(npn_room(p, cx, cy, 3.0));
        let (tx, ty) = (cx + 1.5, cy + 1.5);
        e.push(rect(
            p.cont,
            tx + 0.185,
            ty + 0.185,
            tx + 0.185 + c,
            ty + 0.185 + c,
        ));
        e.push(rect(
            p.cont,
            tx + 0.195,
            cy - 1.5 - 0.195 - c,
            tx + 0.195 + c,
            cy - 1.5 - 0.195,
        ));
    }
    {
        let (cx, cy) = row(3);
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.cont, cx + 1.5, cy, u, c, 0.5));
    }
    {
        let (cx, cy) = row(4);
        e.extend(npn(p, cx, cy, REF_TH, 0.0));
        let ax = cx + REF_TH + TIE_M + 0.07;
        e.push(rect(p.cont, ax, cy - c / 2.0, ax + c, cy + c / 2.0));
        e.push(rect(p.m1, ax - 0.05, cy - 0.15, ax + c + 0.05, cy + 0.15));
    }
    {
        let (cx, cy) = row(5);
        let t = tbox(cx, cy, 1.5);
        e.extend(tie_ref(p, grown(t, [3.0; 4])));
        e.push(bx(p.trans, t));
        e.push(beside(p.cont, cx + 1.5, cy, u, c, c));
    }
    out(NPN, "npnG2.e.h1", e);

    // h2: the tile lines: the 0.265 gap straddling x = 20 (TRANS wall at 19.9), the
    // TRANS wall on x = 21, the Cont's wall on x = 40, the gap straddling x = 42, at
    // x = 100 and (1000, 1000).  Six.
    let mut e = vec![];
    for (i, tx) in [19.9, 21.0, 40.0 - u, 41.9, 99.9, 1000.0]
        .iter()
        .enumerate()
    {
        let cx = tx - 1.5;
        let cy = if i == 5 {
            1000.0
        } else {
            10.0 + 14.0 * i as f64
        };
        e.extend(npn_room(p, cx, cy, 3.0));
        e.push(beside(p.cont, *tx, cy, u, c, c));
    }
    out(NPN, "npnG2.e.h2", e);
}

/// A flavoured npn at `(cx, cy)` in a reference tie with the given windows.
fn npn_windows(p: &P, cx: f64, cy: f64, label: &str, windows: &[[f64; 4]]) -> Vec<GdsElement> {
    npn_windows_th(p, cx, cy, REF_TH, label, windows)
}

fn npn_windows_th(
    p: &P,
    cx: f64,
    cy: f64,
    th: f64,
    label: &str,
    windows: &[[f64; 4]],
) -> Vec<GdsElement> {
    let t = tbox(cx, cy, th);
    let mut e = tie_ref(p, t);
    e.extend(core_h(p, t, label, windows));
    e
}

fn npn_emitters_h(p: &P) {
    let w = 0.07;
    // h1 (npn13G2.a, min and max 0.90): windows 0.895 (fires), 0.9 (clean), 0.905
    // (fires); a horizontal 0.9 (clean) and 0.895 (fires); a 0.9 drawn as two abutting
    // halves (one window, clean); a 0.895 as two overlapping boxes (fires); a 0.9 x 0.9
    // square (length 0.9, clean); a 0.9 window crossing the TRANS's wall (clean) and a
    // 0.895 one (fires); ten 0.9 windows in a row (clean); ten 0.895 (ten).  Fifteen.
    let mut e = vec![];
    let row = |i: usize| (10.0 + 8.0 * i as f64, 10.0);
    let hw = |cx: f64, cy: f64, l: f64| boxc(cx, cy, l, w);
    for (i, l) in [0.895, 0.9, 0.905].iter().enumerate() {
        let (cx, cy) = row(i);
        e.extend(npn_windows(p, cx, cy, "npn13G2", &[window(cx, cy, w, *l)]));
    }
    for (i, l) in [0.9, 0.895].iter().enumerate() {
        let (cx, cy) = row(3 + i);
        e.extend(npn_windows(p, cx, cy, "npn13G2", &[hw(cx, cy, *l)]));
    }
    {
        let (cx, cy) = row(5);
        e.extend(npn_windows(
            p,
            cx,
            cy,
            "npn13G2",
            &[
                [cx - w / 2.0, cy - 0.45, cx + w / 2.0, cy],
                [cx - w / 2.0, cy, cx + w / 2.0, cy + 0.45],
            ],
        ));
    }
    {
        let (cx, cy) = row(6);
        e.extend(npn_windows(
            p,
            cx,
            cy,
            "npn13G2",
            &[
                [cx - w / 2.0, cy - 0.45, cx + w / 2.0, cy + 0.1],
                [cx - w / 2.0, cy - 0.1, cx + w / 2.0, cy + 0.445],
            ],
        ));
    }
    {
        let (cx, cy) = row(7);
        e.extend(npn_windows(
            p,
            cx,
            cy,
            "npn13G2",
            &[window(cx, cy, 0.9, 0.9)],
        ));
    }
    for (i, l) in [0.9, 0.895].iter().enumerate() {
        let (cx, cy) = (10.0 + 8.0 * i as f64, 26.0);
        let tx = cx + REF_TH;
        e.extend(npn_windows(p, cx, cy, "npn13G2", &[hw(tx, cy, *l)]));
    }
    for (i, l) in [0.9, 0.895].iter().enumerate() {
        let (cx, cy) = (26.0 + 8.0 * i as f64, 26.0);
        let ws: Vec<[f64; 4]> = (0..10)
            .map(|k| window(cx - 1.35 + 0.3 * k as f64, cy, w, *l))
            .collect();
        e.extend(npn_windows(p, cx, cy, "npn13G2", &ws));
    }
    out(NPN, "npn13G2.a.h1", e);

    // h2 (npn13G2L.a/b: 1.00 to 2.50): 0.995 (L.a), 1.0, 2.5, 2.0 (clean), 2.505
    // (L.b); an "npn13G2L" device with a 0.9 window (L.a, and not npn13G2.a); an
    // "npn13G2" device with a 2.0 window (npn13G2.a, and not L).  Four.
    let mut e = vec![];
    for (i, l) in [0.995, 1.0, 2.5, 2.0, 2.505].iter().enumerate() {
        let (cx, cy) = row(i);
        e.extend(npn_windows(p, cx, cy, "npn13G2L", &[window(cx, cy, w, *l)]));
    }
    {
        let (cx, cy) = row(5);
        e.extend(npn_windows(
            p,
            cx,
            cy,
            "npn13G2L",
            &[window(cx, cy, w, 0.9)],
        ));
        let (cx, cy) = row(6);
        e.extend(npn_windows(p, cx, cy, "npn13G2", &[window(cx, cy, w, 2.0)]));
    }
    out(NPN, "npn13G2L.a.h1", e);

    // h3 (npn13G2V.a/b: 1.00 to 5.00, 0.12 wide): 0.995 (V.a), 1.0, 5.0 (clean), 5.005
    // (V.b); an "npn13G2V" device with a 0.07 x 0.9 window (V.a: the width is no
    // rule).  Three.
    let mut e = vec![];
    for (i, l) in [0.995, 1.0, 5.0, 5.005].iter().enumerate() {
        let (cx, cy) = (10.0 + 10.0 * i as f64, 10.0);
        e.extend(npn_windows_th(
            p,
            cx,
            cy,
            3.0,
            "npn13G2V",
            &[window(cx, cy, 0.12, *l)],
        ));
    }
    {
        let (cx, cy) = (50.0, 10.0);
        e.extend(npn_windows_th(
            p,
            cx,
            cy,
            3.0,
            "npn13G2V",
            &[window(cx, cy, w, 0.9)],
        ));
    }
    out(NPN, "npn13G2V.a.h1", e);

    // h4: recognition.  A 0.5 window in an unlabelled TRANS; the label 0.1 outside the
    // TRANS in the hole; the labels "npn13G2C" and "npn13G2X" (no flavour of the
    // manual); a labelled TRANS with no tie and a 0.895 window (the length rules need
    // no tie: fires).  A window at 45° is not drawn: 0.07 x 0.9 has no on-grid
    // rotation.  One.
    let mut e = vec![];
    {
        let (cx, cy) = row(0);
        let t = tbox(cx, cy, REF_TH);
        e.extend(tie_ref(p, t));
        e.push(bx(p.trans, t));
        e.push(bx(p.emw, window(cx, cy, w, 0.5)));
    }
    {
        let (cx, cy) = row(1);
        let t = tbox(cx, cy, 1.5);
        e.extend(tie_ref(p, grown(t, [1.0; 4])));
        e.push(bx(p.trans, t));
        e.push(bx(p.emw, window(cx, cy, w, 0.5)));
        e.push(text(p.txt, "npn13G2", t[2] + 0.1, cy));
    }
    for (i, label) in ["npn13G2C", "npn13G2X"].iter().enumerate() {
        let (cx, cy) = row(2 + i);
        e.extend(npn_windows(p, cx, cy, label, &[window(cx, cy, w, 0.5)]));
    }
    {
        let (cx, cy) = row(4);
        e.extend(core_h(
            p,
            tbox(cx, cy, 1.5),
            "npn13G2",
            &[window(cx, cy, w, 0.895)],
        ));
    }
    out(NPN, "npn13G2.a.h2", e);

    // h5: the tile lines: a 0.895 vertical window straddling x = 20, a horizontal one
    // crossing x = 21, a horizontal one ending on x = 40, one starting on x = 42, a
    // vertical one on x = 100, and at (1000, 1000).  Six.
    let mut e = vec![];
    let l = 0.895;
    for wdw in [
        window(20.0, 10.0, w, l),
        hw(21.0, 24.0, l),
        [40.0 - l, 38.0 - w / 2.0, 40.0, 38.0 + w / 2.0],
        [42.0, 52.0 - w / 2.0, 42.0 + l, 52.0 + w / 2.0],
        window(100.0, 10.0, w, l),
        window(1000.0, 1000.0, w, l),
    ]
    .iter()
    {
        let (cx, cy) = ((wdw[0] + wdw[2]) / 2.0, (wdw[1] + wdw[3]) / 2.0);
        e.extend(npn_windows(p, cx, cy, "npn13G2", &[*wdw]));
    }
    out(NPN, "npn13G2.a.h3", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/npn")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    npn_g2_b(&p);
    npn_g2_c(&p);
    npn_g2_d(&p);
    npn_g2_d1_d2(&p);
    npn_g2_e(&p);
    npn_emitters_h(&p);
}
