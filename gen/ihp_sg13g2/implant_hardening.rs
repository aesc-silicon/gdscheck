// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the implant decks: section 5.7 ThickGateOxide (TGO.a-TGO.f),
//! section 5.10 pSD (pSD.a-pSD.n) and section 5.11 nSD:block (nSDB.a-nSDB.e) of the
//! SG13G2 layout rules, with section 4.2's derivations (N+/P+ Activ by drawn nSD/pSD or
//! by default, NFET/PFET, the ties).  Every layout is
//! `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.

use crate::helpers::{
    chamfered_bl, chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array,
    strip45, um, write_gz,
};
use gds21::{GdsArrayRef, GdsDateTime, GdsElement, GdsLibrary, GdsPoint, GdsStruct};
use gdscheck::pdk::PdkConfig;

/// One grid step.
const S: f64 = 0.005;
const SQRT2: f64 = std::f64::consts::SQRT_2;

/// The layers the three decks draw on.
struct L {
    tgo: (i16, i16),
    act: (i16, i16),
    gp: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    nsdb: (i16, i16),
    nw: (i16, i16),
    pwb: (i16, i16),
    cont: (i16, i16),
    sal: (i16, i16),
    res: (i16, i16),
    ext: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
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

fn path(deck: &str, name: &str) -> String {
    format!("tests/data/ihp-sg13g2/{deck}/{name}.gds.gz")
}

fn write(deck: &str, name: &str, elems: Vec<GdsElement>) {
    write_gz(&path(deck, name), library("TOP", elems));
}

/// `<rule>.h<k>` flat and `<rule>.h<k+1>` as a `GdsArrayRef`: 10 × 5 copies of `cell` at
/// `pitch`, with `extra` drawn flat in TOP beside either (a well under the whole array,
/// say).  Hierarchy must not change the answer: fifty violations either way.
fn arrays(
    deck: &str,
    rule: &str,
    k: u32,
    cell: Vec<GdsElement>,
    pitch: f64,
    extra: Vec<GdsElement>,
) {
    let mut flat = extra.clone();
    flat.extend(flat_array(&cell, 10, 5, pitch));
    write(deck, &format!("{rule}.h{k}"), flat);
    let lib = if extra.is_empty() {
        ref_array(cell, 10, 5, pitch)
    } else {
        ref_array_with(cell, 10, 5, pitch, extra)
    };
    write_gz(&path(deck, &format!("{rule}.h{}", k + 1)), lib);
}

/// A `GdsArrayRef` of `cell` (`cols × rows` at `pitch`) placed in TOP beside `extra`.
fn ref_array_with(
    cell: Vec<GdsElement>,
    cols: i16,
    rows: i16,
    pitch: f64,
    extra: Vec<GdsElement>,
) -> GdsLibrary {
    let aref = GdsElement::GdsArrayRef(GdsArrayRef {
        name: "CELL".into(),
        xy: [
            GdsPoint::new(0, 0),
            GdsPoint::new(um(pitch * cols as f64), 0),
            GdsPoint::new(0, um(pitch * rows as f64)),
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

/// A frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, four abutting boxes
/// that merge into one ring.
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

/// A square box of side `b` with its lower-left corner at `(x, y)`.
fn sq(l: (i16, i16), x: f64, y: f64, b: f64) -> GdsElement {
    rect(l, x, y, x + b, y + b)
}

/// The largest half-diagonal on the 0.005 grid whose diamond is narrower than `v`, and
/// the smallest whose diamond is wider.
fn diag(v: f64) -> (f64, f64) {
    let fire = ((v - S) / SQRT2 / S).floor() * S;
    let clean = ((v + S) / SQRT2 / S).ceil() * S;
    (fire, clean)
}

// ---------------------------------------------------------------------------------------
// The width and space suites, shared by TGO.f/e, pSD.a/b and nSDB.a/b.
// ---------------------------------------------------------------------------------------

/// The `min_width` suite for `l` at value `v` (bars 3 µm long):
/// h1 the bound (a `v` bar clean; `v − 0.005` bars in x and y, a 0.005 sliver and a 300 µm
///    bar fire, two walls each);
/// h2 45° (a diamond and a 45° strip under `v` fire, four and two walls; the ones over `v`
///    and a chamfered box are clean);
/// h3 unions (two overlapping boxes, two abutting slices and one side of a ring at
///    `v − 0.005` fire; a plate drawn as a 10 × 10 grid is clean);
/// h4 tile lines (bars ending on, starting on and straddling x = 20/21/40/42, a horizontal
///    bar across x = 20);
/// h5/h6 fifty bars flat and as an array.
fn width_suite(deck: &str, rule: &str, l: (i16, i16), v: f64) {
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

    let (a_fire, a_clean) = diag(v);
    write(
        deck,
        &format!("{rule}.h2"),
        vec![
            diamond(l, 3.0, 3.0, a_fire),
            diamond(l, 6.0, 3.0, a_clean),
            strip45(l, 2.0, 6.0, 2.0, a_fire),
            strip45(l, 6.0, 6.0, 2.0, a_clean),
            chamfered_tr(l, 9.0, 2.0, 10.0 + v, 3.0 + v, 13.0 + 2.0 * v - 0.3),
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

    let h = n / 2.0;
    write(
        deck,
        &format!("{rule}.h4"),
        vec![
            rect(l, 20.0 - n, 2.0, 20.0, 4.0),
            rect(l, 20.0, 6.0, 20.0 + n, 8.0),
            rect(l, 21.0 - h, 10.0, 21.0 + h, 12.0),
            rect(l, 40.0 - h, 2.0, 40.0 + h, 4.0),
            rect(l, 42.0 - h, 2.0, 42.0 + h, 4.0),
            rect(l, 18.0, 14.0, 23.0, 14.0 + n),
        ],
    );

    arrays(deck, rule, 5, vec![rect(l, 0.0, 0.0, n, 1.5)], 3.0, vec![]);
}

/// The `min_space` suite for `l` at value `v` between boxes of side `b`:
/// h1 the bound (a gap of `v` clean; `v − 0.005` in x and y, a corner-to-corner gap under
///    `v` on the diagonal (its axis-aligned parts well under), a box beside a 300 µm bar
///    and a pair at (1000, 1000) fire; a diagonal gap over `v` and a pair whose x-gap is
///    under `v` but whose corners are over it are clean);
/// h2 notches (a U and a slot into a plate at `v − 0.005`, a comb with two such slots
///    fire; a U at `v` is clean);
/// h3 45° (two 45° strips and a diamond's corner against a wall at `v − 0.005` fire, at
///    `v + 0.005` clean);
/// h4 tile lines (gaps straddling, ending on and starting on x = 20, straddling 21, 40, 42
///    and y = 20);
/// h5/h6 fifty pairs flat and as an array.
fn space_suite(deck: &str, rule: &str, l: (i16, i16), v: f64, b: f64) {
    let n = v - S;
    let (d_fire, d_clean) = diag(v);
    let pair =
        |x: f64, y: f64, gx: f64, gy: f64| vec![sq(l, x, y, b), sq(l, x + b + gx, y + b + gy, b)];
    let cs = 2.0 * b + v + 1.0;
    let mut e = vec![sq(l, 2.0, 2.0, b), sq(l, 2.0 + b + v, 2.0, b)];
    e.extend(pair(2.0 + cs, 2.0, n, -b));
    e.extend(pair(2.0 + 2.0 * cs, 2.0, -b, n));
    e.extend(pair(2.0 + 3.0 * cs, 2.0, d_fire, d_fire));
    e.extend(pair(2.0 + 4.0 * cs, 2.0, d_clean, d_clean));
    e.extend(pair(2.0 + 5.0 * cs, 2.0, n, 0.3));
    e.push(rect(l, 60.0, -100.0, 61.0, 200.0));
    e.push(sq(l, 61.0 + n, 2.0, b));
    e.extend(pair(1000.0, 1000.0, n, -b));
    write(deck, &format!("{rule}.h1"), e);

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

    let d = b;
    let dy_fire = (n * SQRT2 / S).floor() * S + 2.0 * d;
    let dy_clean = ((v + S) * SQRT2 / S).ceil() * S + 2.0 * d;
    let mut e = vec![
        strip45(l, 2.0, 2.0, 3.0, d),
        strip45(l, 2.0, 2.0 + dy_fire, 3.0, d),
        strip45(l, 10.0, 2.0, 3.0, d),
        strip45(l, 10.0, 2.0 + dy_clean, 3.0, d),
    ];
    let a = b;
    e.push(rect(l, 2.0, 16.0, 6.0, 16.0 + b));
    e.push(diamond(l, 3.0, 16.0 + b + n + a, a));
    e.push(rect(l, 10.0, 16.0, 14.0, 16.0 + b));
    e.push(diamond(l, 11.0, 16.0 + b + v + S + a, a));
    write(deck, &format!("{rule}.h3"), e);

    let across = |xl: f64, y: f64| {
        vec![
            rect(l, xl - 1.5 - n / 2.0, y, xl - n / 2.0, y + b),
            rect(l, xl + n / 2.0, y, xl + 1.5 + n / 2.0, y + b),
        ]
    };
    let mut e = across(20.0, 2.0);
    e.push(rect(
        l,
        20.0 - n - 1.5,
        2.0 + b + 1.0,
        20.0 - n,
        2.0 + 2.0 * b + 1.0,
    ));
    e.push(rect(l, 20.0, 2.0 + b + 1.0, 21.5, 2.0 + 2.0 * b + 1.0));
    e.push(rect(
        l,
        18.5,
        2.0 + 2.0 * b + 2.0,
        20.0,
        2.0 + 3.0 * b + 2.0,
    ));
    e.push(rect(
        l,
        20.0 + n,
        2.0 + 2.0 * b + 2.0,
        21.5 + n,
        2.0 + 3.0 * b + 2.0,
    ));
    e.extend(across(21.0, 2.0 + 3.0 * b + 3.0));
    e.extend(across(40.0, 2.0));
    e.extend(across(42.0, 2.0 + b + 1.0));
    e.push(rect(
        l,
        10.0,
        20.0 - 1.5 - n / 2.0,
        10.0 + b,
        20.0 - n / 2.0,
    ));
    e.push(rect(
        l,
        10.0,
        20.0 + n / 2.0,
        10.0 + b,
        20.0 + 1.5 + n / 2.0,
    ));
    write(deck, &format!("{rule}.h4"), e);

    arrays(
        deck,
        rule,
        5,
        vec![sq(l, 0.0, 0.0, b), sq(l, b + n, 0.0, b)],
        2.0 * b + v + 1.0,
        vec![],
    );
}

pub fn generate(pdk: &PdkConfig) {
    for d in ["tgo", "psd", "nsdblock"] {
        std::fs::create_dir_all(format!("tests/data/ihp-sg13g2/{d}"))
            .expect("failed to create output directory");
    }
    let l = L::new(pdk);

    width_suite("tgo", "TGO.f", l.tgo, 0.86);
    space_suite("tgo", "TGO.e", l.tgo, 0.86, 2.0);
    tgo_a(&l);
    tgo_b(&l);
    tgo_c(&l);
    tgo_d(&l);

    width_suite("psd", "pSD.a", l.psd, 0.31);
    space_suite("psd", "pSD.b", l.psd, 0.31, 1.0);
    psd_c(&l);
    psd_c1(&l);
    psd_d(&l);
    psd_d1(&l);
    psd_e(&l);
    psd_f(&l);
    psd_g(&l);
    psd_i(&l);
    psd_j(&l);
    psd_k(&l);
    psd_l(&l);
    psd_mn(&l);

    width_suite("nsdblock", "nSDB.a", l.nsdb, 0.31);
    space_suite("nsdblock", "nSDB.b", l.nsdb, 0.31, 1.0);
    nsdb_c(&l);
    nsdb_e(&l);
}

// ---------------------------------------------------------------------------------------
// 5.7 ThickGateOxide
// ---------------------------------------------------------------------------------------

impl L {
    /// A 1 × 1 Activ at `(x, y)` under a ThickGateOx with the margins `ml, mb, mr, mt`.
    fn tgo_box(&self, x: f64, y: f64, ml: f64, mb: f64, mr: f64, mt: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.0, y + 1.0),
            rect(self.tgo, x - ml, y - mb, x + 1.0 + mr, y + 1.0 + mt),
        ]
    }

    /// A 3.3 V transistor at `(x, y)`: Activ `sdl + 0.45 + sdr` × 1 with a 0.45 gate
    /// crossing it (caps 0.3), under a ThickGateOx 0.27 past the Activ all round.  TGO.c
    /// reads `sdl` and `sdr`, the Activ past the gate's sides.
    fn hv(&self, x: f64, y: f64, sdl: f64, sdr: f64) -> Vec<GdsElement> {
        let x1 = x + sdl + 0.45 + sdr;
        vec![
            rect(self.act, x, y, x1, y + 1.0),
            rect(self.gp, x + sdl, y - 0.3, x + sdl + 0.45, y + 1.3),
            rect(self.tgo, x - 0.27, y - 0.27, x1 + 0.27, y + 1.27),
        ]
    }

    /// A 3.3 V transistor whose Activ crosses both oxide edges, as figure 5.7 draws it: a
    /// 0.45 gate at `(x, y)` (caps 0.3) under a ThickGateOx `sdl` past its left side and
    /// `sdr` past its right, the Activ running 1.0 past either oxide edge.
    fn hv2(&self, x: f64, y: f64, sdl: f64, sdr: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x - sdl - 1.0, y, x + 0.45 + sdr + 1.0, y + 1.0),
            rect(self.gp, x, y - 0.3, x + 0.45, y + 1.3),
            rect(self.tgo, x - sdl, y - 0.27, x + 0.45 + sdr, y + 1.27),
        ]
    }

    /// A 1.2 V transistor at `(x, y)`: Activ 1 × 0.5 with a 0.16 gate on its left end, so
    /// the gate's left edge is the Activ's.
    fn lv(&self, x: f64, y: f64) -> Vec<GdsElement> {
        vec![
            rect(self.act, x, y, x + 1.0, y + 0.5),
            rect(self.gp, x, y - 0.2, x + 0.16, y + 0.7),
        ]
    }
}

/// TGO.a, "Min. ThickGateOx extension over Activ 0.27".
fn tgo_a(l: &L) {
    // h1: 0.27 all round clean; 0.265 left, 0.265 top, 0.265 all round (four walls), a
    // right margin of 0 fire; an Activ crossing the oxide's edge (figure 5.7 draws one) and
    // an Activ well outside are nothing; of two Activs under one oxide the 0.265 one fires.
    let mut e = l.tgo_box(2.0, 2.0, 0.27, 0.27, 0.27, 0.27);
    e.extend(l.tgo_box(5.0, 2.0, 0.265, 0.27, 0.27, 0.27));
    e.extend(l.tgo_box(8.0, 2.0, 0.27, 0.27, 0.27, 0.265));
    e.extend(l.tgo_box(11.0, 2.0, 0.265, 0.265, 0.265, 0.265));
    e.extend(l.tgo_box(14.0, 2.0, 0.27, 0.27, 0.0, 0.27));
    e.push(rect(l.tgo, 16.0, 1.73, 17.5, 3.27));
    e.push(rect(l.act, 17.0, 2.0, 18.0, 3.0));
    e.push(rect(l.tgo, 2.0, 6.0, 3.0, 7.0));
    e.push(rect(l.act, 3.5, 6.0, 4.5, 7.0));
    e.push(rect(l.tgo, 6.0, 5.73, 9.27, 7.27));
    e.push(rect(l.act, 6.265, 6.0, 7.265, 7.0));
    e.push(rect(l.act, 8.0, 6.0, 9.0, 7.0));
    write("tgo", "TGO.a.h1", e);

    // h2: a chamfer 0.265 from the Activ's corner (walls 0.27) fires, one at 0.275 is
    // clean; a diamond Activ in a square oxide 0.265 from its corners fires four times,
    // at 0.27 clean; a square Activ in a diamond oxide whose walls pass 0.265 from its
    // corners fires four times, at 0.27 clean.
    write(
        "tgo",
        "TGO.a.h2",
        vec![
            chamfered_tr(l.tgo, 2.0, 2.0, 4.0, 4.0, 7.835),
            rect(l.act, 2.27, 2.27, 3.73, 3.73),
            chamfered_tr(l.tgo, 6.0, 2.0, 8.0, 4.0, 11.85),
            rect(l.act, 6.27, 2.27, 7.73, 3.73),
            rect(l.tgo, 10.235, 2.235, 11.765, 3.765),
            diamond(l.act, 11.0, 3.0, 0.5),
            rect(l.tgo, 13.23, 2.23, 14.77, 3.77),
            diamond(l.act, 14.0, 3.0, 0.5),
            diamond(l.tgo, 17.0, 3.0, 0.875),
            rect(l.act, 16.75, 2.75, 17.25, 3.25),
            diamond(l.tgo, 17.0, 7.0, 0.885),
            rect(l.act, 16.75, 6.75, 17.25, 7.25),
        ],
    );

    // h3: an oxide drawn as two overlapping boxes whose union leaves 0.265 fires; an Activ
    // drawn as four quadrants with 0.265 fires; an Activ in the hole of an oxide ring and
    // one crossing the ring's inner edge are nothing.
    let mut e = vec![
        rect(l.tgo, 2.0, 1.73, 3.5, 3.27),
        rect(l.tgo, 3.0, 1.73, 4.265, 3.27),
        rect(l.act, 2.27, 2.0, 4.0, 3.0),
        rect(l.act, 6.0, 2.0, 6.5, 2.5),
        rect(l.act, 6.5, 2.0, 7.0, 2.5),
        rect(l.act, 6.0, 2.5, 6.5, 3.0),
        rect(l.act, 6.5, 2.5, 7.0, 3.0),
        rect(l.tgo, 5.73, 1.73, 7.265, 3.27),
    ];
    e.extend(ring(l.tgo, 9.0, 1.0, 15.0, 7.0, 10.5, 2.5, 13.5, 5.5));
    e.push(rect(l.act, 11.5, 3.5, 12.5, 4.5));
    e.push(rect(l.act, 13.0, 3.0, 14.0, 4.0));
    write("tgo", "TGO.a.h3", e);

    // h4: 0.265 with the oxide's edge on x = 20, the Activ's edge on x = 20, straddling 21,
    // on 40 and straddling 42 fire; a 0.27 one straddling x = 20 is clean.
    let mut e = l.tgo_box(18.735, 2.0, 0.27, 0.27, 0.265, 0.27);
    e.extend(l.tgo_box(19.0, 6.0, 0.27, 0.27, 0.265, 0.27));
    e.extend(l.tgo_box(20.5, 10.0, 0.27, 0.27, 0.265, 0.27));
    e.extend(l.tgo_box(39.0, 2.0, 0.27, 0.27, 0.265, 0.27));
    e.extend(l.tgo_box(41.5, 2.0, 0.27, 0.27, 0.265, 0.27));
    e.extend(l.tgo_box(19.5, 14.0, 0.27, 0.27, 0.27, 0.27));
    write("tgo", "TGO.a.h4", e);

    // h5/h6: fifty Activs 0.265 from their oxide's right edge, flat and as an array.
    let cell = vec![
        rect(l.act, 0.27, 0.27, 0.77, 0.77),
        rect(l.tgo, 0.0, 0.0, 1.035, 1.04),
    ];
    arrays("tgo", "TGO.a", 5, cell, 3.0, vec![]);

    // h7: a 300 µm Activ 0.265 from its oxide's top edge, and a 0.265 at (1000, 1000).
    write(
        "tgo",
        "TGO.a.h7",
        vec![
            rect(l.act, 2.0, 2.0, 302.0, 3.0),
            rect(l.tgo, 1.73, 1.73, 302.27, 3.265),
            rect(l.act, 1000.0, 1000.0, 1001.0, 1001.0),
            rect(l.tgo, 999.735, 999.73, 1001.27, 1001.27),
        ],
    );
}

/// TGO.b, "Min. space between ThickGateOx and Activ outside thick gate oxide region 0.27".
fn tgo_b(l: &L) {
    // h1: 0.27 clean; 0.265 in x and y and 0.2687 corner to corner fire, 0.2758 corner to
    // corner is clean; an Activ abutting the oxide is a space of zero; an Activ in an oxide
    // ring's hole 0.265 from the inner wall, one 0.265 from a 300 µm oxide and a pair at
    // (1000, 1000) fire.
    let mut e = vec![
        sq(l.tgo, 2.0, 2.0, 2.0),
        sq(l.act, 4.27, 2.0, 1.0),
        sq(l.tgo, 7.0, 2.0, 2.0),
        sq(l.act, 9.265, 2.0, 1.0),
        sq(l.tgo, 12.0, 2.0, 2.0),
        sq(l.act, 12.0, 4.265, 1.0),
        sq(l.tgo, 16.0, 2.0, 2.0),
        sq(l.act, 18.19, 4.19, 1.0),
        sq(l.tgo, 2.0, 8.0, 2.0),
        sq(l.act, 4.195, 10.195, 1.0),
        sq(l.tgo, 7.0, 8.0, 2.0),
        sq(l.act, 9.0, 8.0, 1.0),
    ];
    e.extend(ring(l.tgo, 12.0, 8.0, 18.0, 14.0, 13.5, 9.5, 16.5, 12.5));
    e.push(sq(l.act, 13.765, 10.5, 1.0));
    e.push(rect(l.tgo, 30.0, -100.0, 31.0, 200.0));
    e.push(sq(l.act, 31.265, 2.0, 1.0));
    e.push(sq(l.tgo, 1000.0, 1000.0, 2.0));
    e.push(sq(l.act, 1002.265, 1000.0, 1.0));
    write("tgo", "TGO.b.h1", e);

    // h2: an oxide's chamfered corner passing 0.269 from an Activ's corner (the walls
    // farther) fires, at 0.276 clean; a diamond oxide's corner 0.265 from an Activ's wall
    // and a diamond Activ's corner 0.265 from an oxide's wall fire; a 45° oxide wall 0.265
    // from an Activ's corner fires.
    write(
        "tgo",
        "TGO.b.h2",
        vec![
            chamfered_tr(l.tgo, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.act, 4.14, 3.74, 1.0),
            chamfered_tr(l.tgo, 6.0, 2.0, 8.0, 4.0, 11.5),
            sq(l.act, 7.945, 3.945, 1.0),
            diamond(l.tgo, 12.0, 3.0, 1.5),
            sq(l.act, 13.765, 2.5, 1.0),
            diamond(l.act, 17.0, 3.0, 0.5),
            rect(l.tgo, 17.765, 2.0, 19.765, 4.0),
            strip45(l.tgo, 2.0, 8.0, 3.0, 1.0),
            sq(l.act, 4.0, 8.625, 1.0),
        ],
    );

    // h3: 0.265 gaps straddling x = 20, ending on it, starting on it, straddling 21, 40, 42
    // and y = 20.
    write(
        "tgo",
        "TGO.b.h3",
        vec![
            sq(l.tgo, 17.8, 2.0, 2.0),
            sq(l.act, 20.065, 2.0, 1.0),
            sq(l.tgo, 17.735, 6.0, 2.0),
            sq(l.act, 20.0, 6.0, 1.0),
            sq(l.tgo, 18.0, 10.0, 2.0),
            sq(l.act, 20.265, 10.0, 1.0),
            sq(l.tgo, 18.8, 14.0, 2.0),
            sq(l.act, 21.065, 14.0, 1.0),
            sq(l.tgo, 37.8, 2.0, 2.0),
            sq(l.act, 40.065, 2.0, 1.0),
            sq(l.tgo, 39.8, 6.0, 2.0),
            sq(l.act, 42.065, 6.0, 1.0),
            sq(l.tgo, 10.0, 17.8, 2.0),
            sq(l.act, 10.0, 20.065, 1.0),
        ],
    );

    // h4/h5: fifty 0.265 gaps, flat and as an array.
    arrays(
        "tgo",
        "TGO.b",
        4,
        vec![
            sq(l.tgo, 0.0, 0.0, 1.0),
            rect(l.act, 1.265, 0.25, 1.765, 0.75),
        ],
        3.0,
        vec![],
    );
}

/// TGO.c, "Min. ThickGateOx extension over GatPoly over Activ 0.34".  Figure 5.7 draws
/// `c` from the gate's side to the oxide's edge, where that edge crosses the Activ; an
/// Activ ending inside the oxide has no oxide edge over it, only TGO.a's 0.27.
fn tgo_c(l: &L) {
    // h1: an Activ crossing both oxide edges with the gate 0.34 from either is clean;
    // 0.335 left, right and both fire; an Activ ending 0.335 past the gate inside an oxide
    // 0.27 past it (the oxide's edge 0.605 from the gate) and a gate ending 0.335 inside
    // such an Activ are clean; a gate the oxide's edge cuts through fires (TGO.c on its
    // inside part, TGO.d on its outside part abutting the oxide); a poly over field under
    // the oxide is nothing.
    let mut e = l.hv2(2.34, 2.0, 0.34, 0.34);
    e.extend(l.hv2(6.34, 2.0, 0.335, 0.34));
    e.extend(l.hv2(10.34, 2.0, 0.34, 0.335));
    e.extend(l.hv2(14.34, 2.0, 0.335, 0.335));
    e.extend(l.hv(2.0, 6.0, 0.335, 0.335));
    e.push(rect(l.act, 6.0, 6.0, 7.13, 7.0));
    e.push(rect(l.gp, 6.34, 5.7, 6.79, 6.665));
    e.push(rect(l.tgo, 5.73, 5.73, 7.4, 7.27));
    e.push(rect(l.act, 10.0, 6.0, 12.0, 7.0));
    e.push(rect(l.gp, 10.775, 5.7, 11.225, 7.3));
    e.push(rect(l.tgo, 9.73, 5.73, 11.0, 7.27));
    e.push(rect(l.tgo, 14.0, 5.5, 16.0, 7.5));
    e.push(rect(l.gp, 14.5, 5.7, 14.95, 7.3));
    write("tgo", "TGO.c.h1", e);

    // h2: a 45° oxide edge crossing the Activ, 0.336 from the gate's corner where the
    // perpendicular lands below the Activ (0.475 along the Activ's edge), fires under the
    // closest-approach reading; an oxide and an Activ each drawn as two boxes, the union's
    // oxide edge 0.335 from the gate, fire.
    let mut e = vec![
        rect(l.act, 1.0, 2.0, 5.0, 3.0),
        rect(l.gp, 2.0, 1.7, 2.45, 3.3),
        poly(
            l.tgo,
            &[
                (1.5, 1.73),
                (2.655, 1.73),
                (4.195, 3.27),
                (6.0, 3.27),
                (6.0, 4.5),
                (1.5, 4.5),
            ],
        ),
    ];
    e.push(rect(l.act, 7.0, 2.0, 9.0, 3.0));
    e.push(rect(l.act, 8.5, 2.0, 11.0, 3.0));
    e.push(rect(l.gp, 8.0, 1.7, 8.45, 3.3));
    e.push(rect(l.tgo, 7.5, 1.73, 8.6, 3.27));
    e.push(rect(l.tgo, 8.4, 1.73, 8.785, 3.27));
    write("tgo", "TGO.c.h2", e);

    // h3: 0.335 with the oxide's edge on x = 20, straddling 21, on 40 and straddling 42.
    let mut e = l.hv2(19.215, 2.0, 0.34, 0.335);
    e.extend(l.hv2(20.5, 6.0, 0.34, 0.335));
    e.extend(l.hv2(39.215, 2.0, 0.34, 0.335));
    e.extend(l.hv2(41.5, 6.0, 0.34, 0.335));
    write("tgo", "TGO.c.h3", e);

    // h4/h5: fifty 0.335 transistors, flat and as an array.
    arrays(
        "tgo",
        "TGO.c",
        4,
        l.hv2(1.34, 0.27, 0.34, 0.335),
        4.0,
        vec![],
    );
}

/// TGO.d, "Min. space between ThickGateOx and GatPoly over Activ outside thick gate oxide
/// region 0.34".
fn tgo_d(l: &L) {
    // h1: 0.34 clean; 0.335 in x, 0.335 with the Activ's S/D 0.1 from the oxide (a TGO.b,
    // set aside), 0.335 from the gate's width edge (the poly's cap nearer), 0.339 corner
    // to corner, a gate abutting the oxide (a TGO.b too, set aside) and 0.335 at
    // (1000, 1000) fire; 0.3465 corner to corner is clean.
    let mut e = vec![sq(l.tgo, 2.0, 2.0, 1.0)];
    e.extend(l.lv(3.34, 2.0));
    e.push(sq(l.tgo, 6.0, 2.0, 1.0));
    e.extend(l.lv(7.335, 2.0));
    e.push(sq(l.tgo, 10.0, 2.0, 1.0));
    e.push(rect(l.act, 11.1, 2.0, 12.5, 2.5));
    e.push(rect(l.gp, 11.335, 1.8, 11.495, 2.7));
    e.push(sq(l.tgo, 14.0, 2.0, 1.0));
    e.push(rect(l.act, 14.0, 3.335, 15.0, 3.835));
    e.push(rect(l.gp, 14.4, 3.135, 14.56, 4.035));
    e.push(sq(l.tgo, 2.0, 6.0, 1.0));
    e.extend(l.lv(3.24, 7.24));
    e.push(sq(l.tgo, 6.0, 6.0, 1.0));
    e.extend(l.lv(7.245, 7.245));
    e.push(sq(l.tgo, 10.0, 6.0, 1.0));
    e.extend(l.lv(11.0, 6.0));
    e.push(sq(l.tgo, 1000.0, 1000.0, 1.0));
    e.extend(l.lv(1001.335, 1000.0));
    write("tgo", "TGO.d.h1", e);

    // h2: a diamond oxide's corner 0.335 from a gate's edge fires; an oxide's chamfered
    // corner 0.335 from a gate's corner fires.
    let mut e = vec![diamond(l.tgo, 3.0, 3.0, 1.0)];
    e.extend(l.lv(4.335, 2.75));
    e.push(chamfered_tr(l.tgo, 7.0, 2.0, 9.0, 4.0, 12.5));
    e.extend(l.lv(8.985, 3.985));
    write("tgo", "TGO.d.h2", e);

    // h3: 0.335 gaps straddling x = 20, ending on it, straddling 21, 40 and 42.
    let mut e = vec![sq(l.tgo, 18.8, 2.0, 1.0)];
    e.extend(l.lv(20.135, 2.0));
    e.push(sq(l.tgo, 18.665, 6.0, 1.0));
    e.extend(l.lv(20.0, 6.0));
    e.push(sq(l.tgo, 19.8, 10.0, 1.0));
    e.extend(l.lv(21.135, 10.0));
    e.push(sq(l.tgo, 38.8, 2.0, 1.0));
    e.extend(l.lv(40.135, 2.0));
    e.push(sq(l.tgo, 40.8, 6.0, 1.0));
    e.extend(l.lv(42.135, 6.0));
    write("tgo", "TGO.d.h3", e);

    // h4/h5: fifty 0.335 gaps, flat and as an array.
    let mut cell = vec![sq(l.tgo, 0.0, 0.0, 1.0)];
    cell.extend(l.lv(1.335, 0.25));
    arrays("tgo", "TGO.d", 4, cell, 3.5, vec![]);
}

// ---------------------------------------------------------------------------------------
// 5.10 pSD
// ---------------------------------------------------------------------------------------

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

    // h5/h6: fifty 0.175 in one well, flat and as an array.
    let cell = vec![
        rect(l.act, 0.18, 0.18, 0.68, 0.68),
        rect(l.psd, 0.0, 0.0, 0.855, 0.86),
    ];
    arrays(
        "psd",
        "pSD.c",
        5,
        cell,
        2.5,
        vec![rect(l.nw, -1.0, -1.0, 26.0, 13.0)],
    );

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
fn psd_c1(l: &L) {
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

    // h2/h3: fifty 0.025, flat and as an array.
    let cell = vec![
        rect(l.act, 0.03, 0.03, 0.53, 0.53),
        rect(l.psd, 0.0, 0.0, 0.555, 0.56),
    ];
    arrays("psd", "pSD.c1", 2, cell, 2.0, vec![]);
}

/// pSD.d, "Min. pSD space to unrelated N+Activ in PWell 0.18"; N+Activ is Activ AND nSD,
/// nSD what is under neither pSD nor nSD:block or under drawn nSD (section 4.2), and
/// unrelated regions "do not touch each other" (section 4.1).
fn psd_d(l: &L) {
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

    // h5/h6: fifty 0.175 gaps, flat and as an array.
    arrays(
        "psd",
        "pSD.d",
        5,
        vec![sq(l.psd, 0.0, 0.0, 1.0), sq(l.act, 1.175, 0.0, 1.0)],
        3.0,
        vec![],
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

    // h3/h4: fifty 0.025 gaps in one well, flat and as an array.
    let cell = vec![sq(l.psd, 0.0, 0.0, 1.0), sq(l.act, 1.025, 0.0, 1.0)];
    arrays(
        "psd",
        "pSD.d1",
        3,
        cell,
        3.0,
        vec![rect(l.nw, -1.0, -1.0, 31.0, 16.0)],
    );
}

/// pSD.e, "Min. pSD overlap of Activ at one position when forming abutted substrate tie
/// 0.30"; figure 5.10 draws `e` across the P+ part from the abutment line.
fn psd_e(l: &L) {
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

    // h2/h3: fifty 0.295 ties, flat and as an array.
    arrays("psd", "pSD.e", 2, l.stie(0.0, 0.0, 0.295), 4.0, vec![]);
}

/// pSD.f, "Min. Activ extension over pSD at one position when forming abutted NWell tie
/// 0.30".
fn psd_f(l: &L) {
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

    // h2/h3: fifty 0.295 tabs in one well, flat and as an array.
    let cell = l.ntie_up(0.2, 0.2, 0.295, 0.5);
    arrays(
        "psd",
        "pSD.f",
        2,
        cell,
        3.0,
        vec![rect(l.nw, -1.0, -1.0, 31.0, 16.0)],
    );
}

/// pSD.g, "Min. N+Activ or P+Activ area (µm²) when forming abutted tie 0.09".
fn psd_g(l: &L) {
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

    // h2/h3: fifty 0.0885 N+ tabs (0.295 × 0.30) in one well, flat and as an array.
    let cell = l.ntie_up(0.2, 0.2, 0.30, 0.295);
    arrays(
        "psd",
        "pSD.g",
        2,
        cell,
        3.0,
        vec![rect(l.nw, -1.0, -1.0, 31.0, 16.0)],
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

    // h3/h4: fifty 0.295 PFETs in one well, flat and as an array.
    let cell = l.pfet(0.18, 0.3, 0.12, 0.115, 0.2, 0.5, 0.18, 0.18, 0.30);
    arrays(
        "psd",
        "pSD.i",
        3,
        cell,
        2.5,
        vec![rect(l.nw, -1.0, -1.0, 26.0, 13.0)],
    );
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
    arrays("psd", "pSD.j", 3, cell, 3.5, vec![]);
}

/// pSD.k, "Min. pSD area (µm²) 0.25".
fn psd_k(l: &L) {
    // h1: 0.5 × 0.5 clean, 0.5 × 0.495 fires; a diamond of 0.245 fires, of 0.2592 clean;
    // two overlapping 0.4 boxes whose union is 0.24 fire once; two abutting boxes adding to
    // 0.25 are clean; 0.2475 straddling x = 20, 21 and 40 fire once each; two 0.16 boxes
    // touching at a corner fire twice (their pSD.b set aside); 0.16 at (1000, 1000) fires; a
    // 0.005 × 40 sliver (a pSD.a, set aside) fires.
    write(
        "psd",
        "pSD.k.h1",
        vec![
            rect(l.psd, 2.0, 2.0, 2.5, 2.5),
            rect(l.psd, 4.0, 2.0, 4.5, 2.495),
            diamond(l.psd, 7.0, 2.5, 0.35),
            diamond(l.psd, 9.0, 2.5, 0.36),
            rect(l.psd, 11.0, 2.0, 11.4, 2.4),
            rect(l.psd, 11.2, 2.0, 11.6, 2.4),
            rect(l.psd, 13.0, 2.0, 13.25, 2.5),
            rect(l.psd, 13.25, 2.0, 13.5, 2.5),
            rect(l.psd, 19.75, 2.0, 20.25, 2.495),
            rect(l.psd, 20.75, 6.0, 21.25, 6.495),
            rect(l.psd, 39.75, 2.0, 40.25, 2.495),
            rect(l.psd, 2.0, 6.0, 2.4, 6.4),
            rect(l.psd, 2.4, 6.4, 2.8, 6.8),
            rect(l.psd, 1000.0, 1000.0, 1000.4, 1000.4),
            rect(l.psd, 6.0, 6.0, 6.005, 46.0),
        ],
    );

    // h2/h3: fifty 0.16 boxes, flat and as an array.
    arrays(
        "psd",
        "pSD.k",
        2,
        vec![rect(l.psd, 0.0, 0.0, 0.4, 0.4)],
        2.0,
        vec![],
    );
}

/// pSD.l, "Min. pSD enclosed area (µm²) 0.25"; figure 5.10 draws `l` in a hole holding an
/// island as the empty area between.
fn psd_l(l: &L) {
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

    // h2/h3: fifty 0.2475 holes, flat and as an array.
    let cell = ring(l.psd, 0.0, 0.0, 1.5, 1.5, 0.5, 0.5, 1.0, 0.995);
    arrays("psd", "pSD.l", 2, cell, 2.5, vec![]);
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

// ---------------------------------------------------------------------------------------
// 5.11 nSD:block
// ---------------------------------------------------------------------------------------

/// nSDB.c, "Min. nSD:block space to pSD 0.31", with nSDB.d, "Overlap of nSD:block and pSD
/// is allowed".
fn nsdb_c(l: &L) {
    // h1: 0.31 clean; 0.305 in x and y and 0.304 corner to corner fire; a pSD abutting the
    // block and one overlapping it are allowed; a pSD overlapping one arm of a U-shaped
    // block and 0.2 from its other arm fires; a pSD in a block ring's hole 0.305 from the
    // inner wall fires; 0.305 at (1000, 1000) fires.
    let mut e = vec![
        sq(l.nsdb, 2.0, 2.0, 1.0),
        sq(l.psd, 3.31, 2.0, 1.0),
        sq(l.nsdb, 6.0, 2.0, 1.0),
        sq(l.psd, 7.305, 2.0, 1.0),
        sq(l.nsdb, 10.0, 2.0, 1.0),
        sq(l.psd, 10.0, 3.305, 1.0),
        sq(l.nsdb, 14.0, 2.0, 1.0),
        sq(l.psd, 15.215, 3.215, 1.0),
        sq(l.nsdb, 2.0, 6.0, 1.0),
        sq(l.psd, 3.0, 6.0, 1.0),
        sq(l.nsdb, 6.0, 6.0, 1.0),
        sq(l.psd, 6.5, 6.0, 1.0),
        rect(l.nsdb, 10.0, 6.0, 10.5, 8.0),
        rect(l.nsdb, 11.0, 6.0, 11.5, 8.0),
        rect(l.nsdb, 10.0, 6.0, 11.5, 6.5),
        rect(l.psd, 10.2, 7.0, 10.8, 7.8),
    ];
    e.extend(ring(l.nsdb, 14.0, 5.0, 18.0, 9.0, 15.0, 6.0, 17.0, 8.0));
    e.push(sq(l.psd, 15.305, 6.5, 1.0));
    e.push(sq(l.nsdb, 1000.0, 1000.0, 1.0));
    e.push(sq(l.psd, 1001.305, 1000.0, 1.0));
    write("nsdblock", "nSDB.c.h1", e);

    // h2: a block's chamfered corner 0.304 from a pSD's corner (the walls farther) and a
    // diamond pSD's corner 0.305 from a block's wall fire.
    write(
        "nsdblock",
        "nSDB.c.h2",
        vec![
            chamfered_tr(l.nsdb, 2.0, 2.0, 4.0, 4.0, 7.5),
            sq(l.psd, 4.165, 3.765, 1.0),
            rect(l.nsdb, 7.0, 2.0, 8.195, 4.0),
            diamond(l.psd, 9.0, 3.0, 0.5),
        ],
    );

    // h3: 0.305 gaps straddling x = 20, ending on it, starting on it, straddling 21, 40 and
    // 42.
    write(
        "nsdblock",
        "nSDB.c.h3",
        vec![
            sq(l.nsdb, 18.85, 2.0, 1.0),
            sq(l.psd, 20.155, 2.0, 1.0),
            sq(l.nsdb, 18.695, 6.0, 1.0),
            sq(l.psd, 20.0, 6.0, 1.0),
            sq(l.nsdb, 19.0, 10.0, 1.0),
            sq(l.psd, 20.305, 10.0, 1.0),
            sq(l.nsdb, 19.85, 14.0, 1.0),
            sq(l.psd, 21.155, 14.0, 1.0),
            sq(l.nsdb, 38.85, 2.0, 1.0),
            sq(l.psd, 40.155, 2.0, 1.0),
            sq(l.nsdb, 40.85, 6.0, 1.0),
            sq(l.psd, 42.155, 6.0, 1.0),
        ],
    );

    // h4/h5: fifty 0.305 gaps, flat and as an array.
    arrays(
        "nsdblock",
        "nSDB.c",
        4,
        vec![sq(l.nsdb, 0.0, 0.0, 1.0), sq(l.psd, 1.305, 0.0, 1.0)],
        3.5,
        vec![],
    );
}

/// nSDB.e, "Min. nSD:block space to Cont 0.00 (nSD:block and Cont do not overlap)".
fn nsdb_e(l: &L) {
    // h1: a Cont inside a block, one half over its edge and one 0.005 over it fire; one
    // abutting the edge, one touching at a corner and one in a block ring's hole are no
    // overlap; a Cont half over a block's edge on x = 20, one in a block straddling 21, one
    // in a block at (1000, 1000) and a 0.16 × 0.5 bar in a block fire.
    let mut e = vec![
        sq(l.nsdb, 2.0, 2.0, 1.0),
        sq(l.cont, 2.4, 2.4, 0.16),
        sq(l.nsdb, 5.0, 2.0, 1.0),
        sq(l.cont, 5.92, 2.4, 0.16),
        sq(l.nsdb, 8.0, 2.0, 1.0),
        sq(l.cont, 9.0, 2.4, 0.16),
        sq(l.nsdb, 11.0, 2.0, 1.0),
        sq(l.cont, 12.0, 3.0, 0.16),
    ];
    e.extend(ring(l.nsdb, 14.0, 1.0, 17.0, 4.0, 15.0, 2.0, 16.0, 3.0));
    e.push(sq(l.cont, 15.42, 2.42, 0.16));
    e.push(sq(l.nsdb, 2.0, 6.0, 1.0));
    e.push(sq(l.cont, 2.995, 6.4, 0.16));
    e.push(sq(l.nsdb, 19.0, 6.0, 1.0));
    e.push(sq(l.cont, 19.92, 6.4, 0.16));
    e.push(sq(l.nsdb, 20.5, 10.0, 1.0));
    e.push(sq(l.cont, 20.92, 10.4, 0.16));
    e.push(sq(l.nsdb, 1000.0, 1000.0, 1.0));
    e.push(sq(l.cont, 1000.4, 1000.4, 0.16));
    e.push(sq(l.nsdb, 5.0, 6.0, 1.0));
    e.push(rect(l.cont, 5.4, 6.2, 5.56, 6.7));
    write("nsdblock", "nSDB.e.h1", e);

    // h2/h3: fifty Conts in blocks, flat and as an array.
    arrays(
        "nsdblock",
        "nSDB.e",
        2,
        vec![sq(l.nsdb, 0.0, 0.0, 1.0), sq(l.cont, 0.42, 0.42, 0.16)],
        2.0,
        vec![],
    );
}
