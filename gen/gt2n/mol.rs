// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Middle-of-line patterns: the gate contact, the source/drain contact and its via, M0
//! and V0. A contact or via sits in a whole stack - the gate or SDCON under it, the M0
//! or M1 over it - each giving it what its rules ask, so a pattern's violations all
//! belong to the rule it was drawn for. Each pattern leads with the shape exactly at
//! the limit, then the ones a DBU past it.
//!
//! GT2N's contacts and vias are exact rectangles, flush with a layer on one axis: VG
//! fills its 14 nm gate across x, V0 its 12 nm M0 across y and its M1 across x. A
//! contact a DBU wide or a via a DBU off its wire also lies outside it, so those tests
//! ignore the containment rule.

use super::{Corpus, DBU, Rows, bx, corner_rise, pg};
use gds21::GdsElement;

const OFFSET: f64 = 5000.0;

pub(super) fn generate(c: &Corpus<'_>) {
    vg(c);
    sdcon(c);
    vsd(c);
    m0(c);
    v0(c);
}

/// A `w`×`h` box at (x, y) jogged `d` along x half way up, each half `w` wide.
fn jog_x(layer: (i16, i16), x: f64, y: f64, w: f64, h: f64, d: f64) -> GdsElement {
    let m = y + h / 2.0;
    pg(
        layer,
        &[
            (x, y),
            (x + w, y),
            (x + w, m),
            (x + w + d, m),
            (x + w + d, y + h),
            (x + d, y + h),
            (x + d, m),
            (x, m),
        ],
    )
}

/// A `w`×`h` box at (x, y) jogged `d` along y half way across, each half `h` tall.
fn jog_y(layer: (i16, i16), x: f64, y: f64, w: f64, h: f64, d: f64) -> GdsElement {
    let m = x + w / 2.0;
    pg(
        layer,
        &[
            (x, y),
            (m, y),
            (m, y + d),
            (x + w, y + d),
            (x + w, y + h + d),
            (m, y + h + d),
            (m, y + h),
            (x, y + h),
        ],
    )
}

fn vg(c: &Corpus<'_>) {
    let (vg, gate, m0, gcut, dummy) = (
        c.layer("VG"),
        c.layer("GATE"),
        c.layer("M0"),
        c.layer("GCUT"),
        c.layer("DUMMY"),
    );
    let w = c.src("VG.1", "exact_width", 14.0);
    let h = c.src("VG.2", "exact_width", 12.0);
    let en = c.src("VG.M0.2", "min_enclosure", 4.0);
    let tall = 100.0;
    // A gate `gw` wide at (x, y), a `cw`×`ch` contact half way up, and an M0 `l` and
    // `r` past it across x and `mh` tall, centred on where a 12 nm contact would be.
    #[allow(clippy::too_many_arguments)]
    let stack = |x: f64, y: f64, gw: f64, cw: f64, ch: f64, l: f64, r: f64, mh: f64| {
        let cy = y + tall / 2.0;
        vec![
            bx(gate, x, y, gw, tall),
            bx(vg, x, cy, cw, ch),
            bx(m0, x - l, cy + (h - mh) / 2.0, l + cw + r, mh),
        ]
    };
    let ok = |x: f64, y: f64| stack(x, y, w, w, h, en, en, h);

    // A contact a DBU wide is a DBU wider than its gate; these gates hold the widest.
    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .into_iter()
        .flat_map(|cw| stack(OFFSET, rows.next(tall), w + DBU, cw, h, en, en, h))
        .collect();
    c.write("vg", "VG.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .into_iter()
        .flat_map(|ch| stack(OFFSET, rows.next(tall), w, w, ch, en, en, h + 2.0))
        .collect();
    c.write("vg", "VG.2.narrow", true, elems);

    // A contact clear of a GCUT; one abutting a GCUT, one a DBU into one, and one on a
    // gate under a DUMMY, all three of which interact.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for into in [-DBU, 0.0, DBU] {
        let y = rows.next(tall);
        elems.extend(ok(OFFSET, y));
        elems.push(bx(
            gcut,
            OFFSET - 14.0,
            y + tall / 2.0 + h - into,
            42.0,
            10.0,
        ));
    }
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.push(bx(dummy, OFFSET, y, w, tall));
    c.write("vg", "VG.3.cut", true, elems);

    // A contact on its gate; one abutting a gate's side, and one with none. Neither of
    // the last two lies inside a gate, which VG.5 also reads.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(stack(OFFSET, y, w, w, h, en, en, h));
    elems.remove(elems.len() - 3);
    elems.push(bx(gate, OFFSET - w, y, w, tall));
    let y = rows.next(tall);
    elems.extend(stack(OFFSET, y, w, w, h, en, en, h));
    elems.remove(elems.len() - 3);
    c.write("vg", "VG.4.alone", true, elems);

    // A contact a DBU off its gate across x, and one running a DBU past its gate's top.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.push(bx(gate, OFFSET + DBU, y, w, tall));
    elems.push(bx(vg, OFFSET, y + tall / 2.0, w, h));
    elems.push(bx(m0, OFFSET - en, y + tall / 2.0, w + 2.0 * en, h));
    let y = rows.next(tall);
    elems.push(bx(gate, OFFSET, y, w, tall / 2.0 + h - DBU));
    elems.push(bx(vg, OFFSET, y + tall / 2.0, w, h));
    elems.push(bx(m0, OFFSET - en, y + tall / 2.0, w + 2.0 * en, h));
    c.write("vg", "VG.5.outside", true, elems);

    // Two contacts side by side across x, each on its own gate, `gap` apart.
    let s = c.src("VG.6.LEF", "min_space", 28.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.extend(ok(OFFSET, y));
        elems.extend(ok(OFFSET + w + gap, y));
    }
    c.write("vg", "VG.6.LEF.close", true, elems);

    // A contact jogged a DBU, in a gate and an M0 wide enough to hold it.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.push(bx(gate, OFFSET - 5.0, y, w + 10.0, tall));
    elems.push(jog_x(vg, OFFSET, y + tall / 2.0, w, h, DBU));
    elems.push(bx(
        m0,
        OFFSET - 10.0,
        y + tall / 2.0 - 5.0,
        w + 20.0,
        h + 10.0,
    ));
    c.write("vg", "VG.7.bent", true, elems);

    // A contact with no M0, and one whose M0 sits a DBU high.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    let y = rows.next(tall);
    elems.push(bx(gate, OFFSET, y, w, tall));
    elems.push(bx(vg, OFFSET, y + tall / 2.0, w, h));
    elems.push(bx(m0, OFFSET - en, y + tall / 2.0 + DBU, w + 2.0 * en, h));
    c.write("vg", "VG.M0.1.outside", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [(en, en), (en - DBU, en), (en, en - DBU)]
        .into_iter()
        .flat_map(|(l, r)| stack(OFFSET, rows.next(tall), w, w, h, l, r, h))
        .collect();
    c.write("vg", "VG.M0.2.enc", true, elems);
}

fn sdcon(c: &Corpus<'_>) {
    let (sdcon, gate, act, vsd, m0) = (
        c.layer("SDCON"),
        c.layer("GATE"),
        c.layer("ACT"),
        c.layer("VSD"),
        c.layer("M0"),
    );
    let w = c.src("SDCON.1", "exact_width", 16.0);
    let tall = 60.0;

    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .map(|sw| bx(sdcon, OFFSET, rows.next(tall), sw, tall))
        .to_vec();
    c.write("sdcon", "SDCON.1.narrow", true, elems);

    // Two contacts end to end, `gap` apart: 20 nm, then a DBU less; then a DBU less
    // again with the two joined through a VSD each and one M0, one net.
    let s = c.src("SDCON.2", "min_space", 20.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for (gap, joined) in [(s, false), (s - DBU, false), (s - DBU, true)] {
        let y = rows.next(2.0 * tall + gap);
        let top = y + tall + gap;
        elems.push(bx(sdcon, OFFSET, y, w, tall));
        elems.push(bx(sdcon, OFFSET, top, w, tall));
        if joined {
            let (vw, vh) = (13.0, 12.0);
            let vx = OFFSET + (w - vw) / 2.0;
            elems.push(bx(vsd, vx, y + tall - vh, vw, vh));
            elems.push(bx(vsd, vx, top, vw, vh));
            elems.push(bx(m0, vx - 4.0, y + tall - vh, vw + 8.0, 2.0 * vh + gap));
        }
    }
    c.write("sdcon", "SDCON.2.close", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = vec![
        bx(sdcon, OFFSET, rows.next(tall), w, tall),
        jog_x(sdcon, OFFSET, rows.next(tall), w, tall, DBU),
    ];
    c.write("sdcon", "SDCON.3.bent", true, elems);

    // A contact across a 20 nm ACT, running `below` and `above` past it.
    let en = c.src("SDCON.ACT.1", "min_enclosure", 5.0);
    let ah = 20.0;
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for (below, above) in [(en, en), (en - DBU, en), (en, en - DBU)] {
        let y = rows.next(ah + 20.0);
        elems.push(bx(act, OFFSET - 50.0, y + 10.0, w + 100.0, ah));
        elems.push(bx(sdcon, OFFSET, y + 10.0 - below, w, below + ah + above));
    }
    c.write("sdcon", "SDCON.ACT.1.enc", true, elems);

    let s = c.src("SDCON.GATE.1", "min_space", 6.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.push(bx(gate, OFFSET - gap - 14.0, y - 20.0, 14.0, tall + 40.0));
        elems.push(bx(sdcon, OFFSET, y, w, tall));
    }
    c.write("sdcon", "SDCON.GATE.1.close", true, elems);
}

fn vsd(c: &Corpus<'_>) {
    let (vsd, sdcon, m0) = (c.layer("VSD"), c.layer("SDCON"), c.layer("M0"));
    let w = c.src("VSD.1", "exact_width", 13.0);
    let h = c.src("VSD.2", "exact_width", 12.0);
    let en_sd = c.src("VSD.SDCON.1", "min_enclosure", 1.0);
    let en_m0 = c.src("VSD.M0.1", "min_enclosure", 4.0);
    let (sw, tall) = (16.0, 60.0);
    // A `vw`×`vh` via at (x, y) in a 16 nm SDCON `sl` past it on the left, and an M0
    // `ml` and `mr` past it across x and `mh` tall, both centred on where a 12 nm via
    // would be.
    #[allow(clippy::too_many_arguments)]
    let stack = |x: f64, y: f64, vw: f64, vh: f64, sl: f64, ml: f64, mr: f64, mh: f64| {
        vec![
            bx(vsd, x, y, vw, vh),
            bx(sdcon, x - sl, y + (h - tall) / 2.0, sw, tall),
            bx(m0, x - ml, y + (h - mh) / 2.0, ml + vw + mr, mh),
        ]
    };
    let mid = (sw - w) / 2.0;
    let ok = |x: f64, y: f64| stack(x, y, w, h, mid, en_m0, en_m0, h);

    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .into_iter()
        .flat_map(|vw| stack(OFFSET, rows.next(tall), vw, h, mid, en_m0, en_m0, h))
        .collect();
    c.write("vsd", "VSD.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .into_iter()
        .flat_map(|vh| stack(OFFSET, rows.next(tall), w, vh, mid, en_m0, en_m0, h + 2.0))
        .collect();
    c.write("vsd", "VSD.2.narrow", true, elems);

    // A via jogged a DBU, in an SDCON and an M0 wide enough to hold it.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.push(jog_x(vsd, OFFSET, y, w, h, DBU));
    elems.push(bx(sdcon, OFFSET - 5.0, y - 20.0, w + 10.0, tall));
    elems.push(bx(m0, OFFSET - 10.0, y - 5.0, w + 20.0, h + 10.0));
    c.write("vsd", "VSD.3.bent", true, elems);

    // A via running a DBU past its SDCON's top, and one with no SDCON.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.push(bx(vsd, OFFSET, y, w, h));
    elems.push(bx(sdcon, OFFSET - mid, y + h - DBU - tall, sw, tall));
    elems.push(bx(m0, OFFSET - en_m0, y, w + 2.0 * en_m0, h));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.remove(elems.len() - 2);
    c.write("vsd", "VSD.4.outside", true, elems);

    // A via with no M0, and one whose M0 sits a DBU high.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    elems.push(bx(m0, OFFSET - en_m0, y + DBU, w + 2.0 * en_m0, h));
    c.write("vsd", "VSD.5.outside", true, elems);

    // Two vias side by side across x, each in its own SDCON and M0, `gap` apart.
    let s = c.src("VSD.6.LEF", "min_space", 29.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.extend(ok(OFFSET, y));
        elems.extend(ok(OFFSET + w + gap, y));
    }
    c.write("vsd", "VSD.6.LEF.close", true, elems);

    // The SDCON's 3 nm to spare split 1.5 and 1.5, 1 and 2, then 0.5 and 2.5 each way.
    let mut rows = Rows::new(OFFSET);
    let elems = [mid, en_sd, en_sd - DBU, sw - w - en_sd + DBU]
        .into_iter()
        .flat_map(|sl| stack(OFFSET, rows.next(tall), w, h, sl, en_m0, en_m0, h))
        .collect();
    c.write("vsd", "VSD.SDCON.1.enc", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [(en_m0, en_m0), (en_m0 - DBU, en_m0), (en_m0, en_m0 - DBU)]
        .into_iter()
        .flat_map(|(l, r)| stack(OFFSET, rows.next(tall), w, h, mid, l, r, h))
        .collect();
    c.write("vsd", "VSD.M0.1.enc", true, elems);
}

fn m0(c: &Corpus<'_>) {
    let m0 = c.layer("M0");
    let h = c.src("M0.1", "exact_width", 12.0);
    let len = c.src("M0.2", "min_width", 24.0);
    let long = 200.0;

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .map(|mh| bx(m0, OFFSET, rows.next(mh), long, mh))
        .to_vec();
    c.write("m0", "M0.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [len, len - DBU]
        .map(|mw| bx(m0, OFFSET, rows.next(h), mw, h))
        .to_vec();
    c.write("m0", "M0.2.short", true, elems);

    let s = c.src("M0.3", "min_space", 12.0);
    assert_eq!(c.src("M0.3", "min_notch", 12.0), s);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(2.0 * h + gap);
        elems.push(bx(m0, OFFSET, y, long, h));
        elems.push(bx(m0, OFFSET, y + h + gap, long, h));
    }
    c.write("m0", "M0.3.close", true, elems);

    // A C open to the right, its bar M0.2's 24 nm across.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(2.0 * h + gap);
        let top = y + 2.0 * h + gap;
        elems.push(pg(
            m0,
            &[
                (OFFSET, y),
                (OFFSET + long, y),
                (OFFSET + long, y + h),
                (OFFSET + len, y + h),
                (OFFSET + len, top - h),
                (OFFSET + long, top - h),
                (OFFSET + long, top),
                (OFFSET, top),
            ],
        ));
    }
    c.write("m0", "M0.3.notch", true, elems);

    let t = c.src("M0.4", "min_space", 17.0);
    assert_eq!(c.src("M0.4", "min_notch", 17.0), t);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [t, t - DBU] {
        let y = rows.next(h);
        elems.push(bx(m0, OFFSET, y, long, h));
        elems.push(bx(m0, OFFSET + long + gap, y, long, h));
    }
    c.write("m0", "M0.4.close", true, elems);

    // Two wires end to end, joined by a third along their tops: a slot `gap` long.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [t, t - DBU] {
        let y = rows.next(2.0 * h);
        let x1 = OFFSET + long + gap;
        elems.push(pg(
            m0,
            &[
                (OFFSET, y),
                (OFFSET + long, y),
                (OFFSET + long, y + h),
                (x1, y + h),
                (x1, y),
                (x1 + long, y),
                (x1 + long, y + 2.0 * h),
                (OFFSET, y + 2.0 * h),
            ],
        ));
    }
    c.write("m0", "M0.4.notch", true, elems);

    let k = c.src("M0.5", "min_space", 15.0);
    let dx = 9.0;
    let dy = corner_rise(k, dx);
    // Two wires, the second past the first's tip and above it, so only corners face.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for rise in [dy, dy - DBU] {
        let y = rows.next(2.0 * h + rise);
        elems.push(bx(m0, OFFSET, y, long, h));
        elems.push(bx(m0, OFFSET + long + dx, y + h + rise, long, h));
    }
    c.write("m0", "M0.5.corner", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = vec![
        bx(m0, OFFSET, rows.next(h), long, h),
        jog_y(m0, OFFSET, rows.next(h + DBU), long, h, DBU),
    ];
    c.write("m0", "M0.6.bent", true, elems);
}

fn v0(c: &Corpus<'_>) {
    let (v0, m0, m1) = (c.layer("V0"), c.layer("M0"), c.layer("M1"));
    let w = c.src("V0.1", "exact_width", 14.0);
    let h = c.src("V0.2", "exact_width", 12.0);
    let en0 = c.src("V0.M0.1", "min_enclosure", 4.0);
    let en1 = c.src("V0.M1.1", "min_enclosure", 4.0);
    let tall = 60.0;
    // A `vw`×`vh` via at (x, y): an M0 flush with it across y, `l` and `r` past it
    // across x; an M1 flush with it across x, `b` and `t` past it across y.
    #[allow(clippy::too_many_arguments)]
    let stack = |x: f64, y: f64, vw: f64, vh: f64, l: f64, r: f64, b: f64, t: f64| {
        vec![
            bx(v0, x, y, vw, vh),
            bx(m0, x - l, y, l + vw + r, vh),
            bx(m1, x, y - b, vw, b + vh + t),
        ]
    };
    let run = 20.0;
    let ok = |x: f64, y: f64| stack(x, y, w, h, run, run, run, run);

    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .into_iter()
        .flat_map(|vw| stack(OFFSET, rows.next(tall), vw, h, run, run, run, run))
        .collect();
    c.write("v0", "V0.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .into_iter()
        .flat_map(|vh| stack(OFFSET, rows.next(tall), w, vh, run, run, run, run))
        .collect();
    c.write("v0", "V0.2.narrow", true, elems);

    // A via jogged a DBU, in an M0 and an M1 wide enough to hold it - and so off
    // their outlines, which V0.7 and V0.8 read.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.push(jog_x(v0, OFFSET, y, w, h, DBU));
    elems.push(bx(m0, OFFSET - run, y - 5.0, w + 2.0 * run, h + 10.0));
    elems.push(bx(m1, OFFSET - 5.0, y - run, w + 10.0, h + 2.0 * run));
    c.write("v0", "V0.3.bent", true, elems);

    // A via with no M0, and one whose M0 sits a DBU high: either way a horizontal wall
    // lies off M0, which V0.8 also reads.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.remove(elems.len() - 2);
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.remove(elems.len() - 2);
    elems.push(bx(m0, OFFSET - run, y + DBU, w + 2.0 * run, h));
    c.write("v0", "V0.4.outside", true, elems);

    // The same for M1, off which a vertical wall then lies, as V0.7 reads.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    elems.push(bx(m1, OFFSET + DBU, y - run, w, h + 2.0 * run));
    c.write("v0", "V0.5.outside", true, elems);

    // Two vias corner to corner, each in its own wires, run out to the enclosures.
    let k = c.src("V0.6", "min_space", 18.0);
    let dx = 10.0;
    let dy = corner_rise(k, dx);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for rise in [dy, dy - DBU] {
        let y = rows.next(2.0 * h + rise);
        elems.extend(stack(OFFSET, y, w, h, en0, en0, en1, en1));
        elems.extend(stack(
            OFFSET + w + dx,
            y + h + rise,
            w,
            h,
            en0,
            en0,
            en1,
            en1,
        ));
    }
    c.write("v0", "V0.6.corner", true, elems);

    // Two vias on one M0 track, `gap` apart across x, each in its own M1; then two on
    // adjacent tracks, one above the other at M0.3's 12 nm, which V0.7a leaves alone.
    let s = c.src("V0.7a", "min_space", 14.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.extend(ok(OFFSET, y));
        elems.extend(ok(OFFSET + w + gap, y));
    }
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.extend(ok(OFFSET, y + h + 12.0));
    c.write("v0", "V0.7a.close", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [(en0, en0), (en0 - DBU, en0), (en0, en0 - DBU)]
        .into_iter()
        .flat_map(|(l, r)| stack(OFFSET, rows.next(tall), w, h, l, r, run, run))
        .collect();
    c.write("v0", "V0.M0.1.enc", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [(en1, en1), (en1 - DBU, en1), (en1, en1 - DBU)]
        .into_iter()
        .flat_map(|(b, t)| stack(OFFSET, rows.next(tall), w, h, run, run, b, t))
        .collect();
    c.write("v0", "V0.M1.1.enc", true, elems);

    // An M1 a DBU to the right: one vertical wall lies outside it - the via then lies
    // outside it too, which V0.5 reads - and the other inside it, off its outline. Then
    // an M1 a DBU wider each side: both walls inside it, off its outline.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    elems.push(bx(m1, OFFSET + DBU, y - run, w, h + 2.0 * run));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.pop();
    elems.push(bx(m1, OFFSET - DBU, y - run, w + 2.0 * DBU, h + 2.0 * run));
    c.write("v0", "V0.7.align", true, elems);

    // The same across y for M0, which V0.4 reads.
    let mut rows = Rows::new(OFFSET);
    let mut elems = ok(OFFSET, rows.next(tall));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.remove(elems.len() - 2);
    elems.push(bx(m0, OFFSET - run, y + DBU, w + 2.0 * run, h));
    let y = rows.next(tall);
    elems.extend(ok(OFFSET, y));
    elems.remove(elems.len() - 2);
    elems.push(bx(m0, OFFSET - run, y - DBU, w + 2.0 * run, h + 2.0 * DBU));
    c.write("v0", "V0.8.align", true, elems);
}
