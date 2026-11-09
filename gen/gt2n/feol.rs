// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Front-end patterns: the buried power rail and its via, the gate and its cut and
//! dummy marker, the active, the selects and the threshold-voltage layers. Each pattern
//! leads with the shape exactly at the limit, then the ones a DBU past it, and draws
//! whatever else its deck reads - an active's dummy ends, a via's contact - so its
//! violations all belong to the rule it was drawn for.
//!
//! A notch needs a shape that bends, which BPR.4, GATE.4 and so on forbid, and the
//! bend's joint is no exact width; those patterns' tests ignore the other two rules.

use super::{Corpus, DBU, Rows, bx, pg};
use gds21::GdsElement;

const OFFSET: f64 = 5000.0;

pub(super) fn generate(c: &Corpus<'_>) {
    bpr(c);
    vbpr(c);
    gate(c);
    dummy(c);
    gcut(c);
    act(c);
    select(c, "npsel", &["NSEL", "PSEL"], 84.0);
    select(c, "vt", &["ELVT", "ULVT", "SVT", "HVT", "SRAMVT"], 14.0);
    vt_overlap(c);
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

/// [`jog_x`] turned a quarter: a `w`×`h` box jogged `d` along y half way across.
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

/// A C: two `w`×`arm` arms along x, `gap` apart in y, joined at the left by a `w`-wide
/// bar - a notch `gap` across y, `arm - w` deep.
fn c_y(layer: (i16, i16), x: f64, y: f64, arm: f64, w: f64, gap: f64) -> GdsElement {
    let top = y + 2.0 * w + gap;
    pg(
        layer,
        &[
            (x, y),
            (x + arm, y),
            (x + arm, y + w),
            (x + w, y + w),
            (x + w, y + w + gap),
            (x + arm, y + w + gap),
            (x + arm, top),
            (x, top),
        ],
    )
}

/// [`c_y`] turned a quarter: a U, two `w`×`leg` legs along y `gap` apart in x, joined
/// at the bottom by a `foot`-tall bar.
fn u_x(layer: (i16, i16), x: f64, y: f64, leg: f64, w: f64, gap: f64, foot: f64) -> GdsElement {
    let right = x + 2.0 * w + gap;
    pg(
        layer,
        &[
            (x, y),
            (right, y),
            (right, y + leg),
            (x + w + gap, y + leg),
            (x + w + gap, y + foot),
            (x + w, y + foot),
            (x + w, y + leg),
            (x, y + leg),
        ],
    )
}

fn bpr(c: &Corpus<'_>) {
    let bpr = c.layer("BPR");
    let h = c.src("BPR.1", "exact_width", 32.0);
    let long = 1000.0;

    // Exactly 32 nm tall: a DBU under and a DBU over both fail.
    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .map(|rh| bx(bpr, OFFSET, rows.next(rh), long, rh))
        .to_vec();
    c.write("bpr", "BPR.1.narrow", true, elems);

    let s = c.src("BPR.2", "min_space", 112.0);
    assert_eq!(c.src("BPR.2", "min_notch", 112.0), s);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(2.0 * h + gap);
        elems.push(bx(bpr, OFFSET, y, long, h));
        elems.push(bx(bpr, OFFSET, y + h + gap, long, h));
    }
    c.write("bpr", "BPR.2.close", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [s, s - DBU]
        .map(|gap| c_y(bpr, OFFSET, rows.next(2.0 * h + gap), long, h, gap))
        .to_vec();
    c.write("bpr", "BPR.2.notch", true, elems);

    // BPR.3: a rail broken by 10 µm runs on; one broken by less does not.
    let run = c.src("BPR.3", "min_space", 10000.0);
    assert_eq!(c.src("BPR.3", "min_notch", 10000.0), run);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [run, run - DBU] {
        let y = rows.next(h);
        elems.push(bx(bpr, OFFSET, y, long, h));
        elems.push(bx(bpr, OFFSET + long + gap, y, long, h));
    }
    c.write("bpr", "BPR.3.broken", true, elems);

    // A notch across x is a slot in a rail, open below: a 10 µm slot, then a DBU less.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [run, run - DBU] {
        let y = rows.next(2.0 * h);
        let x1 = OFFSET + long + gap;
        elems.push(pg(
            bpr,
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
    c.write("bpr", "BPR.3.notch", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = vec![
        bx(bpr, OFFSET, rows.next(h), long, h),
        jog_y(bpr, OFFSET, rows.next(h + DBU), long, h, DBU),
    ];
    c.write("bpr", "BPR.4.bent", true, elems);
}

fn vbpr(c: &Corpus<'_>) {
    let (vbpr, sdcon, gcut) = (c.layer("VBPR"), c.layer("SDCON"), c.layer("GCUT"));
    let h = c.src("VBPR.1", "exact_width", 11.0);
    let w = c.src("VBPR.2", "exact_width", 16.0);
    // Every via in an SDCON wide enough for the widest of them, `m` past it all round.
    let m = 10.0;
    let via = |x: f64, y: f64, vw: f64, vh: f64| {
        vec![
            bx(vbpr, x, y, vw, vh),
            bx(sdcon, x - m, y - m, vw + 2.0 * m, vh + 2.0 * m),
        ]
    };

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU, h + DBU]
        .into_iter()
        .flat_map(|vh| via(OFFSET, rows.next(vh + 2.0 * m), w, vh))
        .collect();
    c.write("vbpr", "VBPR.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .into_iter()
        .flat_map(|vw| via(OFFSET, rows.next(h + 2.0 * m), vw, h))
        .collect();
    c.write("vbpr", "VBPR.2.narrow", true, elems);

    // A via clear of a GCUT, one abutting it - which shares no area with it, and does
    // not interact under the runset's `include_touch = NONE` - and one a DBU into it.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gx in [DBU, 0.0, -DBU] {
        let y = rows.next(h + 2.0 * m);
        elems.extend(via(OFFSET, y, w, h));
        elems.push(bx(gcut, OFFSET + w + gx, y, 42.0, h));
    }
    c.write("vbpr", "VBPR.3.gcut", true, elems);

    // A via flush with its SDCON's wall, one a DBU past it, and one with no SDCON.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for past in [0.0, DBU] {
        let y = rows.next(h + 2.0 * m);
        elems.push(bx(vbpr, OFFSET, y, w, h));
        elems.push(bx(sdcon, OFFSET - m, y - m, w + m - past, h + 2.0 * m));
    }
    elems.push(bx(vbpr, OFFSET, rows.next(h), w, h));
    c.write("vbpr", "VBPR.4.outside", true, elems);
}

fn gate(c: &Corpus<'_>) {
    let gate = c.layer("GATE");
    let w = c.src("GATE.1", "exact_width", 14.0);
    let s = c.src("GATE.2", "min_space", 28.0);
    assert_eq!(c.src("GATE.2", "min_notch", 28.0), s);
    let pitch = c.src("GATE.2", "offtrack", 42.0);
    assert_eq!(pitch, w + s);
    assert_eq!(
        c.src_param("GATE.2", "offtrack", "offset", 0.0),
        0.0,
        "the gates are drawn in the deck's frame"
    );
    // A gate's left wall, its centre on a multiple of the pitch.
    let x = (OFFSET / pitch).round() * pitch - w / 2.0;
    let tall = 100.0;

    // A gate a DBU narrow or wide has its centre a quarter nm off the pitch, which
    // GATE.2 reads; the test ignores it.
    let mut rows = Rows::new(OFFSET);
    let elems = [w, w - DBU, w + DBU]
        .map(|gw| bx(gate, x, rows.next(tall), gw, tall))
        .to_vec();
    c.write("gate", "GATE.1.narrow", true, elems);

    // Two gates on the pitch; a gate a DBU right of it, and one half a pitch off.
    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems = vec![bx(gate, x, y, w, tall), bx(gate, x + pitch, y, w, tall)];
    elems.push(bx(gate, x + DBU, rows.next(tall), w, tall));
    elems.push(bx(gate, x + pitch / 2.0, rows.next(tall), w, tall));
    c.write("gate", "GATE.2.pitch", true, elems);

    // A gate missing from a row: three gates on the pitch; two with one missing between
    // them, and with three; two 24 pitches apart, 994 nm facing, within the 1 µm the
    // deck closes the gates over; two 25 pitches apart, beyond it, which read as two
    // blocks; and two a pitch apart across x but in different rows, facing nowhere.
    let reach = 2.0 * c.radius("GateRun#1");
    assert_eq!(reach, 1000.0);
    assert_eq!(c.radius("GateMissing#1"), s / 2.0);
    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems: Vec<_> = (0..3)
        .map(|k| bx(gate, x + k as f64 * pitch, y, w, tall))
        .collect();
    for apart in [2.0, 4.0, 24.0, 25.0] {
        assert_eq!(apart * pitch - w <= reach, apart < 25.0);
        let y = rows.next(tall);
        elems.push(bx(gate, x, y, w, tall));
        elems.push(bx(gate, x + apart * pitch, y, w, tall));
    }
    let y = rows.next(2.0 * tall + 10.0);
    elems.push(bx(gate, x, y, w, tall));
    elems.push(bx(gate, x + 2.0 * pitch, y + tall + 10.0, w, tall));
    c.write("gate", "GATE.2.missing", true, elems);

    // A pair 28 nm apart, then a DBU nearer: the second gate is then off the pitch too.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.push(bx(gate, x, y, w, tall));
        elems.push(bx(gate, x + w + gap, y, w, tall));
    }
    c.write("gate", "GATE.2.close", true, elems);

    // A U whose extent is centred on the pitch, then one a DBU narrower inside, whose
    // extent is not.
    let mut rows = Rows::new(OFFSET);
    let elems = [s, s - DBU]
        .map(|gap| u_x(gate, x - pitch / 2.0, rows.next(tall), tall, w, gap, w))
        .to_vec();
    c.write("gate", "GATE.2.notch", true, elems);

    // A jogged gate's extent is a DBU wider, its centre off the pitch: the test ignores
    // GATE.2.
    let mut rows = Rows::new(OFFSET);
    let elems = vec![
        bx(gate, x, rows.next(tall), w, tall),
        jog_x(gate, x, rows.next(tall), w, tall, DBU),
    ];
    c.write("gate", "GATE.4.bent", true, elems);
}

fn dummy(c: &Corpus<'_>) {
    let (dummy, gate) = (c.layer("DUMMY"), c.layer("GATE"));
    let w = c.src("DUMMY.1", "exact_width", 14.0);
    let tall = 100.0;

    // Each marker on a gate wide enough to hold the widest of them.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for dw in [w, w - DBU, w + DBU] {
        let y = rows.next(tall);
        elems.push(bx(gate, OFFSET - 5.0, y, w + 10.0, tall));
        elems.push(bx(dummy, OFFSET, y, dw, tall));
    }
    c.write("dummy", "DUMMY.1.narrow", true, elems);

    // A marker on its gate; one abutting a gate's side, which shares no area with it,
    // and one alone. Neither lies inside a gate, which DUMMY.4 also reads.
    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems = vec![bx(gate, OFFSET, y, w, tall), bx(dummy, OFFSET, y, w, tall)];
    let y = rows.next(tall);
    elems.push(bx(gate, OFFSET, y, w, tall));
    elems.push(bx(dummy, OFFSET + w, y, w, tall));
    elems.push(bx(dummy, OFFSET, rows.next(tall), w, tall));
    c.write("dummy", "DUMMY.2.alone", true, elems);

    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems = vec![bx(gate, OFFSET - 5.0, y, w + 10.0, tall)];
    elems.push(jog_x(dummy, OFFSET, y, w, tall, DBU));
    c.write("dummy", "DUMMY.3.bent", true, elems);

    // A marker flush with its gate's ends, then one running a DBU past the top.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for past in [0.0, DBU] {
        let y = rows.next(tall + past);
        elems.push(bx(gate, OFFSET, y, w, tall));
        elems.push(bx(dummy, OFFSET, y, w, tall + past));
    }
    c.write("dummy", "DUMMY.4.outside", true, elems);
}

fn gcut(c: &Corpus<'_>) {
    let (gcut, gate, dummy, act) = (
        c.layer("GCUT"),
        c.layer("GATE"),
        c.layer("DUMMY"),
        c.layer("ACT"),
    );
    let h = c.src("GCUT.1", "min_width", 10.0);
    let len = c.src("GCUT.2", "min_width", 42.0);
    // Three gates on GT2N's 42 nm pitch; a cut's walls lie in the 28 nm gaps.
    let (gw, pitch, tall) = (14.0, 42.0, 100.0);
    let gates = |y: f64| {
        (0..3)
            .map(|k| bx(gate, OFFSET + k as f64 * pitch, y, gw, tall))
            .collect::<Vec<_>>()
    };
    // A cut from the middle of the gap left of the middle gate, `cw` long and `ch` tall.
    let mid = OFFSET + pitch + gw / 2.0;
    let cut = |y: f64, cw: f64, ch: f64| bx(gcut, mid - len / 2.0, y + tall / 2.0, cw, ch);

    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for ch in [h, h - DBU] {
        let y = rows.next(tall);
        elems.extend(gates(y));
        elems.push(cut(y, len, ch));
    }
    c.write("gcut", "GCUT.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for cw in [len, len - DBU] {
        let y = rows.next(tall);
        elems.extend(gates(y));
        elems.push(cut(y, cw, h));
    }
    c.write("gcut", "GCUT.2.narrow", true, elems);

    // A cut across two gates, then one jogged a DBU in the gap between them.
    let x0 = OFFSET + gw + (pitch - gw) / 2.0;
    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems = gates(y);
    elems.push(bx(gcut, x0, y + tall / 2.0, 2.0 * pitch, h));
    let y = rows.next(tall);
    elems.extend(gates(y));
    elems.push(jog_y(gcut, x0, y + tall / 2.0, 2.0 * pitch, h, DBU));
    c.write("gcut", "GCUT.5.bent", true, elems);

    // A cut on a gate; one abutting a gate's top, which shares no area with it, and one
    // alone. The last two walls lie nowhere near a gate.
    let mut rows = Rows::new(OFFSET);
    let y = rows.next(tall);
    let mut elems = gates(y);
    elems.push(cut(y, len, h));
    let y = rows.next(tall + h);
    elems.push(bx(gate, OFFSET, y, gw, tall));
    elems.push(bx(gcut, OFFSET + gw / 2.0 - len / 2.0, y + tall, len, h));
    elems.push(bx(gcut, OFFSET, rows.next(h), len, h));
    c.write("gcut", "GCUT.6.alone", true, elems);

    // GCUT.7: a cut's right wall in the gap; inside the third gate; flush with that
    // gate's right wall, and with its left wall, from outside; and inside a gate under a
    // DUMMY, which the rule leaves alone.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    let third = OFFSET + 2.0 * pitch;
    for (right, dummied) in [
        (third - (pitch - gw) / 2.0, false),
        (third + gw / 2.0, false),
        (third + gw, false),
        (third, false),
        (third + gw / 2.0, true),
    ] {
        let y = rows.next(tall);
        elems.extend(gates(y));
        if dummied {
            elems.push(bx(dummy, third, y, gw, tall));
        }
        elems.push(bx(gcut, x0, y + tall / 2.0, right - x0, h));
    }
    c.write("gcut", "GCUT.7.edge", true, elems);

    // GCUT.8 and GCUT.ACT.1: an ACT across the gates, below a cut by `gap`. The cut 10 nm
    // off, then a DBU nearer; for GCUT.8, abutting, which interacts, and a DBU into it.
    let act_under = |y: f64, gap: f64| {
        bx(
            act,
            OFFSET - 50.0,
            y + tall / 2.0 - gap - 20.0,
            2.0 * pitch + gw + 100.0,
            20.0,
        )
    };
    let s = c.src("GCUT.ACT.1", "min_space", 10.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(tall);
        elems.extend(gates(y));
        elems.push(cut(y, len, h));
        elems.push(act_under(y, gap));
    }
    c.write("gcut", "GCUT.ACT.1.close", true, elems);

    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [0.0, -DBU] {
        let y = rows.next(tall);
        elems.extend(gates(y));
        elems.push(cut(y, len, h));
        elems.push(act_under(y, gap));
    }
    c.write("gcut", "GCUT.8.act", true, elems);
}

fn act(c: &Corpus<'_>) {
    let (act, dummy, bpr) = (c.layer("ACT"), c.layer("DUMMY"), c.layer("BPR"));
    let h = c.src("ACT.1", "min_width", 8.0);
    let len = c.src("ACT.2", "min_width", 84.0);
    let (ah, along, dw, dh) = (20.0, 200.0, 14.0, 60.0);
    // A DUMMY straddling each end wall of an active from x0 to x1, centred on y.
    let ends =
        |x0: f64, x1: f64, y: f64| [x0, x1].map(|x| bx(dummy, x - dw / 2.0, y - dh / 2.0, dw, dh));
    let device = |x: f64, y: f64, w: f64, h: f64| {
        let mut e = vec![bx(act, x, y, w, h)];
        e.extend(ends(x, x + w, y + h / 2.0));
        e
    };

    let mut rows = Rows::new(OFFSET);
    let elems = [h, h - DBU]
        .into_iter()
        .flat_map(|ah| device(OFFSET, rows.next(dh), len.max(along), ah))
        .collect();
    c.write("act", "ACT.1.narrow", true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [len, len - DBU]
        .into_iter()
        .flat_map(|aw| device(OFFSET, rows.next(dh), aw, ah))
        .collect();
    c.write("act", "ACT.2.narrow", true, elems);

    // A row as the libraries draw it: a 126 nm cell, a one-pitch filler, a 126 nm cell,
    // abutted, a DUMMY on each cell edge. The 13 nm sheets' ACT runs y 35-48, the 31 nm
    // sheets' y 26-57. Fillers matching their neighbours merge into one bar; a 13 nm
    // filler between 31 nm cells leaves a neck 13 nm tall, which nothing reads; a 31 nm
    // filler between 13 nm cells stands 9 nm proud of them above and below, two pieces
    // one pitch across, under ACT.2's 84 nm - what a w13 design filled with the example
    // flow's gt2_6t_filler_w31 draws.
    let (thin, thick) = ((35.0, 48.0), (26.0, 57.0));
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for (cells, filler) in [(thin, thin), (thick, thick), (thick, thin), (thin, thick)] {
        let y = rows.next(144.0);
        let mut x = OFFSET;
        for (wide, (y0, y1)) in [(126.0, cells), (42.0, filler), (126.0, cells)] {
            elems.push(bx(act, x, y + y0, wide, y1 - y0));
            elems.push(bx(dummy, x - dw / 2.0, y, dw, 144.0));
            x += wide;
        }
        elems.push(bx(dummy, x - dw / 2.0, y, dw, 144.0));
    }
    c.write("act", "ACT.2.sheets", true, elems);

    let s = c.src("ACT.3", "min_space", 30.0);
    assert_eq!(c.src("ACT.3", "min_notch", 30.0), s);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(2.0 * ah + gap + dh);
        elems.extend(device(OFFSET, y, along, ah));
        elems.extend(device(OFFSET, y + ah + gap, along, ah));
    }
    c.write("act", "ACT.3.close", true, elems);

    // A C, its bar as wide as ACT.2 asks, a DUMMY on each of its three end walls.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(2.0 * ah + gap + dh);
        let top = y + 2.0 * ah + gap;
        elems.push(pg(
            act,
            &[
                (OFFSET, y),
                (OFFSET + along, y),
                (OFFSET + along, y + ah),
                (OFFSET + len, y + ah),
                (OFFSET + len, top - ah),
                (OFFSET + along, top - ah),
                (OFFSET + along, top),
                (OFFSET, top),
            ],
        ));
        elems.push(bx(dummy, OFFSET - dw / 2.0, y - 10.0, dw, top - y + 20.0));
        elems.push(bx(dummy, OFFSET + len - dw / 2.0, y, dw, top - y));
        elems.push(bx(
            dummy,
            OFFSET + along - dw / 2.0,
            y - 10.0,
            dw,
            top - y + 20.0,
        ));
    }
    c.write("act", "ACT.3.notch", true, elems);

    // Both ends in a DUMMY; the right end on a DUMMY's left wall, which touches it;
    // the right end a DBU short of one; and the right end half covered, the DUMMY
    // stopping half way up it.
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    let half = (dh + ah) / 2.0 - ah / 2.0;
    for (shift, top) in [(0.0, dh), (dw / 2.0, dh), (dw / 2.0 + DBU, dh), (0.0, half)] {
        let y = rows.next(dh);
        elems.push(bx(act, OFFSET, y + (dh - ah) / 2.0, along, ah));
        elems.push(bx(dummy, OFFSET - dw / 2.0, y, dw, dh));
        elems.push(bx(dummy, OFFSET + along - dw / 2.0 + shift, y, dw, top));
    }
    c.write("act", "ACT.6.end", true, elems);

    let s = c.src("ACT.BPR.1", "min_space", 10.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let y = rows.next(dh + gap + 32.0);
        elems.extend(device(OFFSET, y + (dh - ah) / 2.0, along, ah));
        elems.push(bx(
            bpr,
            OFFSET - 50.0,
            y + (dh + ah) / 2.0 + gap,
            along + 100.0,
            32.0,
        ));
    }
    c.write("act", "ACT.BPR.1.close", true, elems);
}

/// A select or threshold-voltage layer: `.1` its height, `.2` its width (`narrow` nm),
/// `.3` its reach past an ACT across y.
fn select(c: &Corpus<'_>, deck: &str, layers: &[&str], narrow: f64) {
    let act = c.layer("ACT");
    for &name in layers {
        let l = c.layer(name);
        let h = c.src(&format!("{name}.1"), "min_width", 72.0);
        let w = c.src(&format!("{name}.2"), "min_width", narrow);
        let en = c.src(&format!("{name}.3"), "min_enclosure", 15.0);
        let wide = 200.0;

        let mut rows = Rows::new(OFFSET);
        let elems = [h, h - DBU]
            .map(|sh| bx(l, OFFSET, rows.next(sh), wide, sh))
            .to_vec();
        c.write(deck, &format!("{name}.1.narrow"), true, elems);

        let mut rows = Rows::new(OFFSET);
        let elems = [w, w - DBU]
            .map(|sw| bx(l, OFFSET, rows.next(h), sw, h))
            .to_vec();
        c.write(deck, &format!("{name}.2.narrow"), true, elems);

        // An ACT 20 nm tall in a 100 nm layer, `en` from its bottom wall then a DBU less,
        // and `en` from its top wall then a DBU less.
        let (tall, ah) = (100.0, 20.0);
        let mut rows = Rows::new(OFFSET);
        let mut elems = vec![];
        for below in [en, en - DBU, tall - ah - en, tall - ah - en + DBU] {
            let y = rows.next(tall);
            elems.push(bx(l, OFFSET, y, wide, tall));
            elems.push(bx(act, OFFSET + 20.0, y + below, wide - 40.0, ah));
        }
        c.write(deck, &format!("{name}.3.enc"), true, elems);
    }
    if deck == "npsel" {
        overlap(c, "npsel", "NSEL.PSEL.1", &[("NSEL", "PSEL")]);
    }
}

/// Each pair of `pairs` abutting, which shares no area, then overlapping by a DBU.
fn overlap(c: &Corpus<'_>, deck: &str, id: &str, pairs: &[(&str, &str)]) {
    let (w, h) = (200.0, 100.0);
    let mut rows = Rows::new(OFFSET);
    let mut elems = vec![];
    for &(a, b) in pairs {
        for into in [0.0, DBU] {
            let y = rows.next(h);
            elems.push(bx(c.layer(a), OFFSET, y, w, h));
            elems.push(bx(c.layer(b), OFFSET + w - into, y, w, h));
        }
    }
    c.write(deck, &format!("{id}.overlap"), true, elems);
}

fn vt_overlap(c: &Corpus<'_>) {
    let vts = ["ELVT", "ULVT", "SVT", "HVT", "SRAMVT"];
    let pairs: Vec<_> = vts
        .iter()
        .enumerate()
        .flat_map(|(i, &a)| vts[i + 1..].iter().map(move |&b| (a, b)))
        .collect();
    assert_eq!(pairs.len(), 10);
    overlap(c, "vt", "VT_OVERLAP.1", &pairs);
}
