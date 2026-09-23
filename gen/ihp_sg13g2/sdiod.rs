// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use super::npn::{G, P, SQRT2, boxc, bx, grid, grown, out, ring_boxes};
use crate::helpers::{chamfered_tr, layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/sdiod";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    sdiod_all(pdk);

    hardening(pdk);
}

/// A Schottky-diode `schottky_nbl1` device: a bar-shaped Cont (`w`×`h`, becoming ContBar
/// since it isn't square) centered at `(cx, cy)`, enclosed by PWell:block/nSD:block/SalBlock
/// at the given margins, inside a generous nBuLay.
fn device(
    pdk: &PdkConfig,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    margins: (f64, f64, f64), // (pwb, nsd, sal)
) -> Vec<GdsElement> {
    let (pwb_margin, nsd_margin, sal_margin) = margins;
    let ring = |name: &str, margin: f64| {
        rect(
            layer(pdk, name),
            cx - w / 2.0 - margin,
            cy - h / 2.0 - margin,
            cx + w / 2.0 + margin,
            cy + h / 2.0 + margin,
        )
    };
    vec![
        rect(
            layer(pdk, "Cont"),
            cx - w / 2.0,
            cy - h / 2.0,
            cx + w / 2.0,
            cy + h / 2.0,
        ),
        ring("PWell.block", pwb_margin),
        ring("nSD.block", nsd_margin),
        ring("SalBlock", sal_margin),
        ring("nBuLay", sal_margin + 1.0), // generously covers everything
    ]
}

/// Six instances, 5 µm apart in x: a clean baseline (all margins/dims exactly on target),
/// then one violation per rule (Sdiod.a min, Sdiod.b max, Sdiod.c min, Sdiod.d min-width,
/// Sdiod.e max-length), each isolated to just that one deviation.
fn sdiod_all(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = Vec::new();
    // Clean: ContBar 0.30×1.00, margins 0.25/0.40/0.45 exactly.
    e.extend(device(pdk, o, o, 0.30, 1.00, (0.25, 0.40, 0.45)));
    // Sdiod.a (min): PWell:block margin only 0.15 (< 0.25).
    e.extend(device(pdk, o + 5.0, o, 0.30, 1.00, (0.15, 0.40, 0.45)));
    // Sdiod.b (max): nSD:block margin 0.55 (> 0.40); SalBlock stays at its own clean 0.45.
    e.extend(device(pdk, o + 10.0, o, 0.30, 1.00, (0.25, 0.55, 0.45)));
    // Sdiod.c (min): SalBlock margin only 0.30 (< 0.45).
    e.extend(device(pdk, o + 15.0, o, 0.30, 1.00, (0.25, 0.40, 0.30)));
    // Sdiod.d (min-width): ContBar 0.20 wide (< 0.30) instead of 0.30.
    e.extend(device(pdk, o + 20.0, o, 0.20, 1.00, (0.25, 0.40, 0.45)));
    // Sdiod.e (max-length): ContBar 1.50 long (> 1.00) instead of 1.00.
    e.extend(device(pdk, o + 25.0, o, 0.30, 1.50, (0.25, 0.40, 0.45)));
    write_gz(&format!("{DIR}/Sdiod.gds.gz"), library("TOP", e));
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

const SDIOD: &str = "tests/data/ihp-sg13g2/sdiod";

/// The Schottky's margins around the bar: PWell:block, nSD:block, SalBlock, each
/// per side (left, bottom, right, top).
struct Margins {
    pwb: [f64; 4],
    nsdb: [f64; 4],
    sal: [f64; 4],
}

impl Margins {
    fn exact() -> Self {
        Margins {
            pwb: [0.25; 4],
            nsdb: [0.40; 4],
            sal: [0.45; 4],
        }
    }
}

/// The schottky_nbl1 around the ContBar `bar`, as the reference cell has it: the three
/// blocks at their margins, the NWell ring from the PWell:block to 1.1 past the bar,
/// Activ 0.85 and nBuLay 1.0 past the bar, Recog:diode over the NWell, a PWell:block
/// ring 1.1 to 1.95, the P+ tie ring (Activ 2.45 to 2.75, pSD 2.35 to 2.85),
/// ThickGateOx to 3.0, Metal1 on the bar.  `nbl` false leaves the nBuLay out.
fn schottky(p: &P, bar: [f64; 4], m: &Margins, nbl: bool) -> Vec<GdsElement> {
    schottky_o(p, bar, m, nbl, 1.1)
}

/// The same with the NWell ring's outer edge `o` past the bar (1.1 in the reference);
/// everything outside it follows.
fn schottky_o(p: &P, bar: [f64; 4], m: &Margins, nbl: bool, o: f64) -> Vec<GdsElement> {
    let pwb = grown(bar, m.pwb);
    let mut e = vec![
        bx(p.cont, bar),
        bx(p.m1, grown(bar, [0.05; 4])),
        bx(p.pwb, pwb),
        bx(p.nsdb, grown(bar, m.nsdb)),
        bx(p.sal, grown(bar, m.sal)),
        bx(p.activ, grown(bar, [o - 0.25; 4])),
        bx(p.recog, grown(bar, [o; 4])),
        bx(p.tgo, grown(bar, [o + 1.9; 4])),
    ];
    if nbl {
        e.push(bx(p.nbl, grown(bar, [o - 0.1; 4])));
    }
    e.extend(ring_boxes(p.nw, grown(bar, [o; 4]), pwb));
    e.extend(ring_boxes(
        p.pwb,
        grown(bar, [o + 0.85; 4]),
        grown(bar, [o; 4]),
    ));
    e.extend(ring_boxes(
        p.activ,
        grown(bar, [o + 1.65; 4]),
        grown(bar, [o + 1.35; 4]),
    ));
    e.extend(ring_boxes(
        p.psd,
        grown(bar, [o + 1.75; 4]),
        grown(bar, [o + 1.25; 4]),
    ));
    e
}

/// A `w` x `l` bar centred on `(cx, cy)`, on the grid.
fn bar(cx: f64, cy: f64, w: f64, l: f64) -> [f64; 4] {
    boxc(cx, cy, w, l)
}

/// The reference Schottky at `(cx, cy)` with the margins `m`.
fn sd(p: &P, cx: f64, cy: f64, m: &Margins) -> Vec<GdsElement> {
    schottky(p, bar(cx, cy, 0.3, 1.0), m, true)
}

fn sdiod_enclosure(p: &P) {
    // h1 for each of Sdiod.a (PWell:block 0.25), Sdiod.b (nSD:block 0.40), Sdiod.c
    // (SalBlock 0.45): the block's right margin one step under (min fires), one step
    // over (max fires), exact (clean), left and right under (two walls), all four
    // under (one closed run), all four over (one), and the block's top-right corner
    // chamfered so it passes one step under the value from the bar's corner (fires,
    // the euclidian reading).  Seven.
    for (rule, v, which) in [
        ("Sdiod.a", 0.25, 0),
        ("Sdiod.b", 0.40, 1),
        ("Sdiod.c", 0.45, 2),
    ] {
        let set = |m: &mut Margins, sides: [f64; 4]| match which {
            0 => m.pwb = sides,
            1 => m.nsdb = sides,
            _ => m.sal = sides,
        };
        let u = v - G;
        let o = v + G;
        let mut e = vec![];
        for (i, sides) in [
            [v, v, u, v],
            [v, v, o, v],
            [v; 4],
            [u, v, u, v],
            [u; 4],
            [o; 4],
        ]
        .iter()
        .enumerate()
        {
            let mut m = Margins::exact();
            set(&mut m, *sides);
            e.extend(sd(p, 10.0 + 8.0 * i as f64, 10.0, &m));
        }
        {
            let (cx, cy) = (58.0, 10.0);
            let b = bar(cx, cy, 0.3, 1.0);
            let m = Margins::exact();
            let mut d = sd(p, cx, cy, &m);
            // Replace the block with its chamfered version.
            let l = [p.pwb, p.nsdb, p.sal][which];
            let blk = grown(b, [v; 4]);
            let idx = 2 + which;
            let k = grid(b[2] + b[3] + u * SQRT2);
            d[idx] = chamfered_tr(l, blk[0], blk[1], blk[2], blk[3], k);
            e.extend(d);
        }
        out(SDIOD, &format!("{rule}.h1"), e);
    }

    // Sdiod.a.h2: shapes.  The PWell:block drawn as four overlapping boxes at 0.25
    // (clean); a PWell:block plate 2.0 past the bar, the NWell ring beyond it (max,
    // one shape); a block 0.8 past the bar under the NWell ring whose hole is still
    // the 0.25 box (max); a block 0.25 with a second, separate PWell:block 0.5 away
    // (clean: not the bar's).  Two.
    let mut e = vec![];
    {
        let (cx, cy) = (10.0, 10.0);
        let b = bar(cx, cy, 0.3, 1.0);
        let mut d = sd(p, cx, cy, &Margins::exact());
        d.remove(2);
        let blk = grown(b, [0.25; 4]);
        let mid = (blk[1] + blk[3]) / 2.0;
        d.push(rect(p.pwb, blk[0], blk[1], blk[2], mid + 0.1));
        d.push(rect(p.pwb, blk[0], mid - 0.1, blk[2], blk[3]));
        d.push(rect(p.pwb, blk[0], blk[1], blk[0] + 0.2, blk[3]));
        d.push(rect(p.pwb, blk[2] - 0.2, blk[1], blk[2], blk[3]));
        e.extend(d);
    }
    {
        let (cx, cy) = (20.0, 10.0);
        let mut m = Margins::exact();
        m.pwb = [2.0; 4];
        e.extend(schottky_o(p, bar(cx, cy, 0.3, 1.0), &m, true, 2.6));
    }
    {
        let (cx, cy) = (30.0, 10.0);
        let b = bar(cx, cy, 0.3, 1.0);
        let mut d = sd(p, cx, cy, &Margins::exact());
        d[2] = bx(p.pwb, grown(b, [0.8; 4]));
        e.extend(d);
    }
    {
        let (cx, cy) = (40.0, 10.0);
        let mut d = sd(p, cx, cy, &Margins::exact());
        d.push(rect(
            p.pwb,
            cx + 0.15 + 0.25 + 0.5,
            cy - 0.3,
            cx + 0.15 + 0.25 + 0.8,
            cy + 0.3,
        ));
        e.extend(d);
    }
    out(SDIOD, "Sdiod.a.h2", e);

    // Sdiod.a.h3: the tile lines, the PWell:block's right margin 0.245: the block's
    // wall on x = 20, the bar straddling x = 21, the bar's wall on x = 40, the 0.245
    // gap straddling x = 42, the bar's top wall on y = 20 (at x = 60), the bar
    // straddling x = 100, at (1000, 1000).  Seven.
    let mut e = vec![];
    let mut m = Margins::exact();
    m.pwb = [0.25, 0.25, 0.245, 0.25];
    for (cx, cy) in [
        (20.0 - 0.245 - 0.15, 10.0),
        (21.0, 30.0),
        (40.0 - 0.15, 50.0),
        (42.0 - 0.1 - 0.15, 70.0),
        (60.0, 20.0 - 0.5),
        (100.0, 10.0),
        (1000.0, 1000.0),
    ] {
        e.extend(sd(p, cx, cy, &m));
    }
    out(SDIOD, "Sdiod.a.h3", e);
}

fn sdiod_dims(p: &P) {
    // Sdiod.d.h1 / Sdiod.e.h1 (width 0.30, length 1.00): bars 0.295 x 1.0 (d),
    // 0.305 x 1.0 (d), 0.3 x 0.995 (e), 0.3 x 1.005 (e), 1.0 x 0.3 lying (clean),
    // 0.3 x 0.3 (a square: a Cont, no ContBar, clean), 0.3 x 0.305 (a bar: e),
    // a 0.3 x 1.0 drawn as two abutting halves (clean), a 0.295 x 1.0 as two
    // overlapping boxes (d).  Six: d, d, e, e, e, d.
    let mut e = vec![];
    let m = Margins::exact();
    for (i, (w, l)) in [
        (0.295, 1.0),
        (0.305, 1.0),
        (0.3, 0.995),
        (0.3, 1.005),
        (1.0, 0.3),
        (0.3, 0.3),
        (0.3, 0.305),
    ]
    .iter()
    .enumerate()
    {
        let (cx, cy) = (10.0 + 8.0 * i as f64, 10.0);
        e.extend(schottky(p, bar(cx, cy, *w, *l), &m, true));
    }
    {
        let (cx, cy) = (10.0, 26.0);
        let b = bar(cx, cy, 0.3, 1.0);
        let mut d = schottky(p, b, &m, true);
        d[0] = rect(p.cont, b[0], b[1], b[2], cy);
        d.push(rect(p.cont, b[0], cy, b[2], b[3]));
        e.extend(d);
    }
    {
        let (cx, cy) = (18.0, 26.0);
        let b = bar(cx, cy, 0.295, 1.0);
        let mut d = schottky(p, b, &m, true);
        d[0] = rect(p.cont, b[0], b[1], b[2], cy + 0.1);
        d.push(rect(p.cont, b[0], cy - 0.1, b[2], b[3]));
        e.extend(d);
    }
    out(SDIOD, "Sdiod.d.h1", e);
}

fn sdiod_recognition(p: &P) {
    // Sdiod.d.h2: recognition ("ContBar enclosed by SalBlock and nSD:block and
    // PWell:block and nBuLay").  A 0.295 bar in the stack without nBuLay (no diode,
    // clean); a 0.295 bar the nBuLay's edge cuts through (not enclosed, clean); a
    // 0.3 x 1.0 bar crossing the PWell:block's wall by 0.1 (not enclosed, no Sdiod.a,
    // clean).  Clean.
    let mut e = vec![];
    let m = Margins::exact();
    e.extend(schottky(p, bar(10.0, 10.0, 0.295, 1.0), &m, false));
    {
        let (cx, cy) = (20.0, 10.0);
        let b = bar(cx, cy, 0.295, 1.0);
        let mut d = schottky(p, b, &m, false);
        d.push(rect(p.nbl, b[0] - 1.0, b[1] - 1.0, cx, b[3] + 1.0));
        e.extend(d);
    }
    {
        let (cx, cy) = (30.0, 10.0);
        let b = bar(cx, cy, 0.3, 1.0);
        let mut d = schottky(p, b, &m, true);
        d[2] = bx(p.pwb, [b[0] - 0.25, b[1] - 0.25, b[2] - 0.1, b[3] + 0.25]);
        e.extend(d);
    }
    out(SDIOD, "Sdiod.d.h2", e);

    // Sdiod.d.h3: a 0.295 bar in the stack inside a 6.0 wide NWell with no drawn
    // nBuLay: section 4.2's generated nBuLay (the well inset by 1.0) encloses it, a
    // diode, Sdiod.d.  One.
    let mut e = vec![];
    {
        let (cx, cy) = (10.0, 10.0);
        let b = bar(cx, cy, 0.295, 1.0);
        e.push(bx(p.cont, b));
        e.push(bx(p.m1, grown(b, [0.05; 4])));
        e.push(bx(p.pwb, grown(b, m.pwb)));
        e.push(bx(p.nsdb, grown(b, m.nsdb)));
        e.push(bx(p.sal, grown(b, m.sal)));
        e.push(bx(p.activ, grown(b, [0.85; 4])));
        e.push(bx(p.nw, grown(b, [3.0; 4])));
    }
    out(SDIOD, "Sdiod.d.h3", e);

    // Sdiod.a.h4: two 0.3 x 1.0 bars in one stack, 0.5 apart, the blocks at their
    // margins around the pair: each bar's inner side is enclosed by 1.05, 1.2 and
    // 1.25 - over every max (the manual's enclosure of each bar; KLayout's "block
    // outside the bars sized by the value" is content up to a 0.8 gap).  Six: a, b, c
    // for each bar.
    let mut e = vec![];
    {
        let (cx, cy) = (10.0, 10.0);
        let pair = [cx - 0.55, cy - 0.5, cx + 0.55, cy + 0.5];
        let mut d = schottky(p, pair, &m, true);
        d[0] = rect(p.cont, cx - 0.55, cy - 0.5, cx - 0.25, cy + 0.5);
        d[1] = rect(p.m1, cx - 0.6, cy - 0.55, cx - 0.2, cy + 0.55);
        d.push(rect(p.cont, cx + 0.25, cy - 0.5, cx + 0.55, cy + 0.5));
        d.push(rect(p.m1, cx + 0.2, cy - 0.55, cx + 0.6, cy + 0.55));
        e.extend(d);
    }
    out(SDIOD, "Sdiod.a.h4", e);
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/sdiod")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    sdiod_enclosure(&p);
    sdiod_dims(&p);
    sdiod_recognition(&p);
}
