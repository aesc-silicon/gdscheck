// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the last three decks of the SG13G2 set: section 6.5 (nmosi and
//! nmosiHV, nmosi.b-nmosi.g, with section 4.2's Iso-PWell-Activ), section 7.4 (Pin.a-Pin.h)
//! and section 7.1 (Ant.a-Ant.i, the net-aware antenna ratios).  Every layout is
//! `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//!
//! The nmosi layouts share one structure, `iso`: an Activ in the hole of a closed NWell
//! ring, the whole on nBuLay, so that the Activ is Iso-PWell-Activ (Activ AND nBuLay AND
//! PWell) and the ring is what the manual tests the rules inside of.  The antenna
//! layouts share one gate, `gate`: a 0.2 µm GatPoly strip over a 1.0 × 0.5 Activ (gate
//! area 0.1 µm², 0.12 µm² of poly over field) with a 0.1 × 0.1 Cont on the poly; the
//! areas of the antennas are what the boxes draw, so every ratio is computed in the
//! comment next to it.

use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, text, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const NMOSI: &str = "tests/data/ihp-sg13g2/nmosi";
const PIN: &str = "tests/data/ihp-sg13g2/pin";
const ANT: &str = "tests/data/ihp-sg13g2/antenna";

/// A box as `(x0, y0, x1, y1)`.
type R = (f64, f64, f64, f64);

fn grown(r: R, d: f64) -> R {
    (r.0 - d, r.1 - d, r.2 + d, r.3 + d)
}

fn rb(l: (i16, i16), r: R) -> GdsElement {
    rect(l, r.0, r.1, r.2, r.3)
}

/// The ring between `outer` and `hole` as four boxes: bottom and top run the full
/// width, left and right fill the sides between them.
fn ring4(l: (i16, i16), outer: R, hole: R) -> Vec<GdsElement> {
    vec![
        rect(l, outer.0, outer.1, outer.2, hole.1),
        rect(l, outer.0, hole.3, outer.2, outer.3),
        rect(l, outer.0, hole.1, hole.0, hole.3),
        rect(l, hole.2, hole.1, outer.2, hole.3),
    ]
}

/// The same ring as one boundary: the outer contour, a slit at the bottom-left, the
/// hole's contour backwards, and out again.
fn ring_keyhole(l: (i16, i16), outer: R, hole: R) -> GdsElement {
    let (ox0, oy0, ox1, oy1) = outer;
    let (hx0, hy0, hx1, hy1) = hole;
    poly(
        l,
        &[
            (ox0, oy0),
            (ox1, oy0),
            (ox1, oy1),
            (ox0, oy1),
            (ox0, hy0),
            (hx0, hy0),
            (hx0, hy1),
            (hx1, hy1),
            (hx1, hy0),
            (ox0, hy0),
        ],
    )
}

struct P {
    activ: (i16, i16),
    gp: (i16, i16),
    cont: (i16, i16),
    m1: (i16, i16),
    via1: (i16, i16),
    m2: (i16, i16),
    via2: (i16, i16),
    m3: (i16, i16),
    via3: (i16, i16),
    m4: (i16, i16),
    via4: (i16, i16),
    m5: (i16, i16),
    tv1: (i16, i16),
    tm1: (i16, i16),
    tv2: (i16, i16),
    tm2: (i16, i16),
    psd: (i16, i16),
    nsdb: (i16, i16),
    nw: (i16, i16),
    pw: (i16, i16),
    pwb: (i16, i16),
    nbl: (i16, i16),
    sal: (i16, i16),
    tgo: (i16, i16),
    diode: (i16, i16),
    esd: (i16, i16),
    txt: (i16, i16),
    m1_label: (i16, i16),
}

impl P {
    fn new(pdk: &PdkConfig) -> Self {
        P {
            activ: layer(pdk, "Activ"),
            gp: layer(pdk, "GatPoly"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            via1: layer(pdk, "Via1"),
            m2: layer(pdk, "Metal2"),
            via2: layer(pdk, "Via2"),
            m3: layer(pdk, "Metal3"),
            via3: layer(pdk, "Via3"),
            m4: layer(pdk, "Metal4"),
            via4: layer(pdk, "Via4"),
            m5: layer(pdk, "Metal5"),
            tv1: layer(pdk, "TopVia1"),
            tm1: layer(pdk, "TopMetal1"),
            tv2: layer(pdk, "TopVia2"),
            tm2: layer(pdk, "TopMetal2"),
            psd: layer(pdk, "pSD"),
            nsdb: layer(pdk, "nSD.block"),
            nw: layer(pdk, "NWell"),
            pw: layer(pdk, "PWell"),
            pwb: layer(pdk, "PWell.block"),
            nbl: layer(pdk, "nBuLay"),
            sal: layer(pdk, "SalBlock"),
            tgo: layer(pdk, "ThickGateOx"),
            diode: layer(pdk, "Recog.diode"),
            esd: layer(pdk, "Recog.esd"),
            txt: layer(pdk, "TEXT"),
            m1_label: layer(pdk, "Metal1.label"),
        }
    }
}

pub fn generate(pdk: &PdkConfig) {
    for d in [NMOSI, PIN, ANT] {
        std::fs::create_dir_all(d).expect("failed to create output directory");
    }
    let p = P::new(pdk);
    nmosi_b(&p);
    nmosi_c(&p);
    nmosi_d(&p);
    nmosi_f(&p);
    nmosi_g(&p);
    nmosi_ring(&p);
    pin(&p);
    antenna(&p);
}

// ------------------------------------------------------------------------------------
// 6.5 nmosi
// ------------------------------------------------------------------------------------

/// The isolated structure: `act` on nBuLay, in the hole of a closed NWell ring.  The
/// hole is `act` grown by `gap` (nmosi.c reads it, 0.39), the ring `ring` wide
/// (nmosi.d, 0.62), the nBuLay the ring's outer box grown by `ext` - so the nBuLay
/// encloses the Activ by `gap + ring + ext` (nmosi.b, 1.24).
fn iso(p: &P, act: R, gap: f64, ring: f64, ext: f64) -> Vec<GdsElement> {
    let hole = grown(act, gap);
    let outer = grown(hole, ring);
    let mut e = vec![rb(p.activ, act), rb(p.nbl, grown(outer, ext))];
    e.extend(ring4(p.nw, outer, hole));
    e
}

/// The structure with every box given: the Activ, the ring's hole and outer box, the
/// nBuLay.
fn iso_r(p: &P, act: R, hole: R, outer: R, nbl: R) -> Vec<GdsElement> {
    let mut e = vec![rb(p.activ, act), rb(p.nbl, nbl)];
    e.extend(ring4(p.nw, outer, hole));
    e
}

/// The standard 2.0 × 1.0 Activ at `(x, y)`.
fn act_at(x: f64, y: f64) -> R {
    (x, y, x + 2.0, y + 1.0)
}

/// The standard structure (gap 0.4, ring 0.7) with the nBuLay `enc` past the Activ on
/// every side but the one given in `side` (0 right, 1 top, 2 left, 3 bottom), which
/// gets `enc_side`.
fn iso_enc(p: &P, act: R, enc: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let mut nbl = grown(act, enc);
    if let Some((s, v)) = side {
        match s {
            0 => nbl.2 = act.2 + v,
            1 => nbl.3 = act.3 + v,
            2 => nbl.0 = act.0 - v,
            _ => nbl.1 = act.1 - v,
        }
    }
    iso_r(p, act, grown(act, 0.4), grown(act, 1.1), nbl)
}

/// nmosi.b - "Min. nBuLay enclosure of Iso-PWell-Activ  1.24".
fn nmosi_b(p: &P) {
    // h1, the bound: the nBuLay 1.24 past the Activ on every side (clean); 1.235 on the
    // right (fires); 1.235 on top (fires); 1.235 all round (fires four times).
    let mut e = iso_enc(p, act_at(2.0, 4.0), 1.24, None);
    e.extend(iso_enc(p, act_at(9.0, 4.0), 1.4, Some((0, 1.235))));
    e.extend(iso_enc(p, act_at(16.0, 4.0), 1.4, Some((1, 1.235))));
    e.extend(iso_enc(p, act_at(2.0, 12.0), 1.235, None));
    write_gz(&format!("{NMOSI}/nmosi.b.h1.gds.gz"), library("TOP", e));

    // h2, the 45° walls.  The nBuLay walls 1.3 from the Activ and its top-right corner
    // chamfered along x + y = k: the chamfer passes 0.875·√2 = 1.2374 from the Activ's
    // corner at x = 2 (fires, closest approach), 0.88·√2 = 1.2445 at x = 9 (clean).  At
    // x = 16 the nBuLay is a diamond whose walls pass 1.2374 from all four Activ corners
    // (fires four times); at x = 24 the same diamond 1.2445 away (clean).  The NWell
    // ring's outer corners poke out of the chamfered nBuLay; the NWell AND nBuLay ring
    // stays 0.67 wide there.
    let mut e = vec![];
    for (x, a) in [(2.0, 0.875), (9.0, 0.88)] {
        let act = act_at(x, 4.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        let n = grown(act, 1.3);
        e.push(chamfered_tr(
            p.nbl,
            n.0,
            n.1,
            n.2,
            n.3,
            act.2 + act.3 + 2.0 * a,
        ));
    }
    for (x, a) in [(16.0, 0.875), (24.0, 0.88)] {
        let act = act_at(x, 4.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        // Corner (1.0, 0.5) off the centre to the wall x + y = A: (A − 1.5)/√2 = a·√2.
        e.push(diamond(p.nbl, x + 1.0, 4.5, 1.5 + 2.0 * a));
    }
    write_gz(&format!("{NMOSI}/nmosi.b.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines: 1.235 on the right with the nBuLay's edge on x = 20; the
    // Activ's edge on x = 40 (the gap straddles the line); the structure across x = 42;
    // the nBuLay's edge on x = 21; one at (1000, 1000); a 300 µm long Activ 1.235 short on
    // top across every line.  Beside them 1.24 exactly with the nBuLay's edge on x = 60.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.235 - 2.0, 4.0),
        (38.0, 4.0),
        (39.9, 12.0),
        (21.0 - 1.235 - 2.0, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_enc(p, act_at(x, y), 1.4, Some((0, 1.235))));
    }
    e.extend(iso_enc(p, act_at(60.0 - 1.24 - 2.0, 4.0), 1.24, None));
    e.extend(iso_enc(p, (70.0, 4.0, 370.0, 5.0), 1.4, Some((1, 1.235))));
    write_gz(&format!("{NMOSI}/nmosi.b.h3.gds.gz"), library("TOP", e));

    // h6, the drawing: the nBuLay as two overlapping boxes whose union is the 1.24 box
    // (clean); as two boxes whose union ends 1.235 right of the Activ (fires); as a 4 × 3
    // grid of abutting tiles at 1.24 (clean); the Activ as three abutting slices, 1.235
    // on the right (fires); the nBuLay as a cross of two boxes whose re-entrant corners
    // lie 0.87·√2 = 1.2304 from the Activ's corners (fires four times); the same cross
    // with 0.88·√2 = 1.2445 (clean).
    let mut e = vec![];
    // The Activ and its ring alone; the nBuLay is drawn below.
    let bare = |act: R| {
        let mut e = vec![rb(p.activ, act)];
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        e
    };
    let act = act_at(2.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    e.push(rect(p.nbl, n.0, n.1, n.0 + 3.0, n.3));
    e.push(rect(p.nbl, n.0 + 2.0, n.1, n.2, n.3));
    let act = act_at(9.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    e.push(rect(p.nbl, n.0, n.1, n.0 + 3.0, n.3));
    e.push(rect(p.nbl, n.0 + 2.0, n.1, act.2 + 1.235, n.3));
    let act = act_at(16.0, 4.0);
    e.extend(bare(act));
    let n = grown(act, 1.24);
    let (tw, th) = ((n.2 - n.0) / 4.0, (n.3 - n.1) / 3.0);
    for i in 0..4 {
        for j in 0..3 {
            let (x, y) = (n.0 + i as f64 * tw, n.1 + j as f64 * th);
            e.push(rect(p.nbl, x, y, x + tw, y + th));
        }
    }
    let act = act_at(2.0, 12.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.0 + 0.7, act.3));
    e.push(rect(p.activ, act.0 + 0.7, act.1, act.0 + 1.4, act.3));
    e.push(rect(p.activ, act.0 + 1.4, act.1, act.2, act.3));
    for (x, a) in [(9.0, 0.87), (16.0, 0.88)] {
        let act = act_at(x, 12.0);
        e.push(rb(p.activ, act));
        e.extend(ring4(p.nw, grown(act, 1.1), grown(act, 0.4)));
        e.push(rect(p.nbl, act.0 - 1.3, act.1 - a, act.2 + 1.3, act.3 + a));
        e.push(rect(p.nbl, act.0 - a, act.1 - 1.3, act.2 + a, act.3 + 1.3));
    }
    write_gz(&format!("{NMOSI}/nmosi.b.h6.gds.gz"), library("TOP", e));

    // h7, the conditions inside a closed ring: the Activ under PWell:block, 1.235 short
    // on the right (not in PWell, not isolated; clean); a ptap - Activ AND pSD - 1.235
    // short (fires: Iso-PWell-Activ is Activ of either doping); an Activ under
    // ThickGateOx (nmosiHV) 1.235 short (fires); an Activ that crosses 0.3 into the NWell
    // ring on the left, 1.235 short on the right: the Iso-PWell-Activ is the part outside
    // the NWell, 1.235 short on the right and, its left edge being the ring's inner wall,
    // 1.0 short on the left (fires twice).
    let mut e = iso_enc(p, act_at(2.0, 4.0), 1.4, Some((0, 1.235)));
    e.push(rb(p.pwb, grown(act_at(2.0, 4.0), 0.1)));
    let act = act_at(9.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.push(rb(p.psd, grown(act, 0.1)));
    let act = act_at(16.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.push(rb(p.tgo, grown(act, 0.3)));
    let act = act_at(2.0, 12.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0 - 0.7, act.1, act.2, act.3));
    write_gz(&format!("{NMOSI}/nmosi.b.h7.gds.gz"), library("TOP", e));
}

/// The standard structure with the hole `gap` from the Activ on every side but `side`,
/// which gets `gap_side`; the ring 0.7 wide, the nBuLay 0.3 past it.
fn iso_gap(p: &P, act: R, gap: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let mut hole = grown(act, gap);
    if let Some((s, v)) = side {
        match s {
            0 => hole.2 = act.2 + v,
            1 => hole.3 = act.3 + v,
            2 => hole.0 = act.0 - v,
            _ => hole.1 = act.1 - v,
        }
    }
    let outer = grown(hole, 0.7);
    iso_r(p, act, hole, outer, grown(outer, 0.3))
}

/// nmosi.c - "Min. NWell space to Iso-PWell-Activ  0.39".
fn nmosi_c(p: &P) {
    // h1, the bound: the ring 0.39 from the Activ on every side (clean); 0.385 on the
    // right (fires); 0.385 on top (fires); 0.385 all round (fires four times).
    let mut e = iso_gap(p, act_at(2.0, 4.0), 0.39, None);
    e.extend(iso_gap(p, act_at(9.0, 4.0), 0.6, Some((0, 0.385))));
    e.extend(iso_gap(p, act_at(16.0, 4.0), 0.6, Some((1, 0.385))));
    e.extend(iso_gap(p, act_at(2.0, 12.0), 0.385, None));
    write_gz(&format!("{NMOSI}/nmosi.c.h1.gds.gz"), library("TOP", e));

    // h2, corners and 45° walls, the hole 0.6 from the Activ.  At x = 2 an NWell tab in
    // the hole's top-right corner whose corner lies (0.27, 0.27) off the Activ's corner:
    // 0.3818 corner to corner, no wall faces the Activ (fires under the euclidian
    // reading); at x = 9 the tab (0.28, 0.28) off, 0.396 (clean).  At x = 16 the hole's
    // top-right corner filled by an NWell triangle whose hypotenuse passes 0.275·√2 =
    // 0.3889 from the Activ's corner (fires); at x = 24 the same at 0.28·√2 = 0.396
    // (clean).  At x = 2, y = 12 a 1.0 square Activ in a diamond hole whose walls pass
    // 0.3889 from its four corners (fires four times).
    let mut e = vec![];
    for (x, d) in [(2.0, 0.27), (9.0, 0.28)] {
        let act = act_at(x, 4.0);
        e.extend(iso_gap(p, act, 0.6, None));
        let hole = grown(act, 0.6);
        e.push(rect(p.nw, act.2 + d, act.3 + d, hole.2, hole.3));
    }
    for (x, a) in [(16.0, 0.275), (24.0, 0.28)] {
        let act = act_at(x, 4.0);
        e.extend(iso_gap(p, act, 0.6, None));
        let hole = grown(act, 0.6);
        // Hypotenuse x + y = hx1 + hy1 − s at (2·0.6 − s)/√2 = a·√2 from the corner.
        let s = 1.2 - 2.0 * a;
        e.push(poly(
            p.nw,
            &[(hole.2, hole.3), (hole.2 - s, hole.3), (hole.2, hole.3 - s)],
        ));
    }
    let (cx, cy) = (3.0, 13.0);
    let act = (cx - 0.5, cy - 0.5, cx + 0.5, cy + 0.5);
    e.push(rb(p.activ, act));
    // Corner (0.5, 0.5) to the wall x + y = D: (D − 1.0)/√2 = 0.275·√2 → D = 1.55.
    let d = 1.55;
    let (ox0, oy0, ox1, oy1) = (cx - 3.0, cy - 3.0, cx + 3.0, cy + 3.0);
    e.push(poly(
        p.nw,
        &[
            (ox0, oy0),
            (ox1, oy0),
            (ox1, oy1),
            (ox0, oy1),
            (ox0, cy),
            (cx - d, cy),
            (cx, cy + d),
            (cx + d, cy),
            (cx, cy - d),
            (cx - d, cy),
            (ox0, cy),
        ],
    ));
    e.push(rect(p.nbl, ox0 - 0.3, oy0 - 0.3, ox1 + 0.3, oy1 + 0.3));
    write_gz(&format!("{NMOSI}/nmosi.c.h2.gds.gz"), library("TOP", e));

    // h3, the drawing of the ring, the hole 0.385 from the Activ on the right: the ring
    // as one keyhole boundary; as eight overlapping boxes; the ring 0.6 away with a 0.5
    // wide NWell tab on its right wall reaching 0.385 from the Activ; two Activs in one
    // hole, one 0.385 from the right wall, one from the left (fires twice); the Activ as
    // two abutting boxes (fires once); the right wall stepped, 0.385 over the upper half
    // of the Activ and 0.6 over the lower.
    let mut e = vec![];
    let act = act_at(2.0, 4.0);
    let hole = (act.0 - 0.6, act.1 - 0.6, act.2 + 0.385, act.3 + 0.6);
    e.push(rb(p.activ, act));
    e.push(ring_keyhole(p.nw, grown(hole, 0.7), hole));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    let act = act_at(9.0, 4.0);
    let hole = (act.0 - 0.6, act.1 - 0.6, act.2 + 0.385, act.3 + 0.6);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    e.extend(ring4(p.nw, grown(hole, 0.7), hole));
    e.extend(ring4(p.nw, grown(hole, 0.5), grown(hole, -0.0)));
    let act = act_at(16.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.push(rect(
        p.nw,
        act.2 + 0.385,
        act.1 + 0.25,
        act.2 + 0.6,
        act.3 - 0.25,
    ));
    let (a1, a2) = ((2.0, 12.0, 3.0, 13.0), (3.5, 12.0, 4.5, 13.0));
    let hole = (a1.0 - 0.385, a1.1 - 0.6, a2.2 + 0.385, a1.3 + 0.6);
    e.push(rb(p.activ, a1));
    e.push(rb(p.activ, a2));
    e.push(rb(p.nbl, grown(hole, 1.0)));
    e.extend(ring4(p.nw, grown(hole, 0.7), hole));
    let act = act_at(9.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.0 + 1.0, act.3));
    e.push(rect(p.activ, act.0 + 1.0, act.1, act.2, act.3));
    let act = act_at(16.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.push(rect(
        p.nw,
        act.2 + 0.385,
        act.1 + 0.5,
        act.2 + 0.6,
        act.3 + 0.6,
    ));
    write_gz(&format!("{NMOSI}/nmosi.c.h3.gds.gz"), library("TOP", e));

    // h4, the tile lines, 0.385 on the right: the hole's wall on x = 20; the gap across
    // x = 40; across x = 42 and x = 21; at (1000, 1000); a 300 µm Activ 0.385 under the
    // top wall.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 0.385 - 2.0, 4.0),
        (38.0, 4.0),
        (39.9, 12.0),
        (18.9, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_gap(p, act_at(x, y), 0.6, Some((0, 0.385))));
    }
    e.extend(iso_gap(p, (70.0, 4.0, 370.0, 5.0), 0.6, Some((1, 0.385))));
    write_gz(&format!("{NMOSI}/nmosi.c.h4.gds.gz"), library("TOP", e));

    // h7, the conditions: an NWell island (no hole of its own) inside the ring's hole,
    // 0.35 from the Activ (the manual's NWell space; fires); the Activ crossing into the
    // ring on the right (space 0 between the Iso-PWell-Activ and the NWell; fires); a
    // plain NWell 0.35 from an Activ on nBuLay with no ring (not inside a closed ring;
    // clean); a ring 0.385 from an Activ with no nBuLay (not isolated; clean); the Activ
    // under PWell:block 0.385 from the ring (clean); a ptap 0.385 from the ring (fires).
    let mut e = vec![];
    let act = act_at(2.0, 4.0);
    e.extend(iso_gap(p, act, 2.0, None));
    e.push(rect(
        p.nw,
        act.2 + 0.35,
        act.1 - 0.5,
        act.2 + 1.35,
        act.3 + 0.5,
    ));
    let act = act_at(9.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, None));
    e.remove(e.len() - 6);
    e.push(rect(p.activ, act.0, act.1, act.2 + 0.9, act.3));
    let act = act_at(16.0, 4.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 2.0)));
    e.push(rect(p.nw, act.2 + 0.35, act.1, act.2 + 1.35, act.3));
    let act = act_at(2.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.remove(e.len() - 5);
    let act = act_at(9.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.push(rb(p.pwb, grown(act, 0.1)));
    let act = act_at(16.0, 12.0);
    e.extend(iso_gap(p, act, 0.6, Some((0, 0.385))));
    e.push(rb(p.psd, grown(act, 0.1)));
    write_gz(&format!("{NMOSI}/nmosi.c.h7.gds.gz"), library("TOP", e));
}

/// The structure with the hole 0.4 from the Activ, the ring `ring` wide on every side
/// but `side`, which gets `ring_side`; the nBuLay 0.3 past the ring.
fn iso_ring(p: &P, act: R, ring: f64, side: Option<(usize, f64)>) -> Vec<GdsElement> {
    let hole = grown(act, 0.4);
    let mut outer = grown(hole, ring);
    if let Some((s, v)) = side {
        match s {
            0 => outer.2 = hole.2 + v,
            1 => outer.3 = hole.3 + v,
            2 => outer.0 = hole.0 - v,
            _ => outer.1 = hole.1 - v,
        }
    }
    iso_r(p, act, hole, outer, grown(outer, 0.3))
}

/// nmosi.d - "Min. NWell-nBuLay width forming an unbroken ring around any
/// Iso-PWell-Activ  0.62" (NWell-nBuLay = NWell AND nBuLay).
fn nmosi_d(p: &P) {
    // h1, the bound: the ring 0.62 wide (clean); the right leg 0.615 (fires); the top leg
    // 0.615 (fires); the NWell ring 1.0 wide, 0.7 from the Activ, but the nBuLay ending
    // 0.615 into its right leg, so NWell AND nBuLay is 0.615 there (fires); every leg
    // 0.615 (fires).
    let mut e = iso_ring(p, act_at(2.0, 4.0), 0.62, None);
    e.extend(iso_ring(p, act_at(9.0, 4.0), 0.8, Some((0, 0.615))));
    e.extend(iso_ring(p, act_at(16.0, 4.0), 0.8, Some((1, 0.615))));
    let act = act_at(2.0, 12.0);
    let hole = grown(act, 0.7);
    let outer = grown(hole, 1.0);
    e.extend(iso_r(
        p,
        act,
        hole,
        outer,
        (outer.0 - 0.3, outer.1 - 0.3, hole.2 + 0.615, outer.3 + 0.3),
    ));
    e.extend(iso_ring(p, act_at(9.0, 12.0), 0.615, None));
    write_gz(&format!("{NMOSI}/nmosi.d.h1.gds.gz"), library("TOP", e));

    // h2, 45° and the drawing.  An octagonal ring 0.7 from the Activ: the legs 0.8 wide,
    // the top-right corner a piece whose outer wall is chamfered and whose inner corner
    // is filled diagonally (0.707 from the Activ's corner), the two 45° walls 0.435·√2 =
    // 0.6152 apart at x = 2 (fires) and 0.44·√2 = 0.6223 at x = 9 (clean).  The right leg as two overlapping boxes 0.62 wide in
    // union at x = 16 (clean) and 0.615 at x = 24 (fires).  At y = 12: a bite 0.185 deep
    // into the outer wall of a 0.8 leg (0.615 left; fires); a 0.2 square hole in a 1.5
    // leg, 0.615 from the inner wall and 0.685 from the outer (fires).
    let mut e = vec![];
    for (x, a) in [(2.0, 0.435), (9.0, 0.44)] {
        let act = act_at(x, 4.0);
        let hole = grown(act, 0.7);
        let outer = grown(hole, 0.8);
        let (hx0, hy0, hx1, hy1) = hole;
        let (ox0, oy0, ox1, oy1) = outer;
        let ci = 0.4;
        let co = 1.6 + ci - 2.0 * a;
        e.push(rb(p.activ, act));
        e.push(rb(p.nbl, grown(outer, 0.3)));
        e.push(rect(p.nw, ox0, oy0, ox1, hy0));
        e.push(rect(p.nw, ox0, hy0, hx0, oy1));
        e.push(rect(p.nw, hx0, hy1, hx1 - ci, oy1));
        e.push(rect(p.nw, hx1, hy0, ox1, hy1 - ci));
        e.push(poly(
            p.nw,
            &[
                (hx1 - ci, hy1),
                (hx1, hy1 - ci),
                (ox1, hy1 - ci),
                (ox1, oy1 - co),
                (ox1 - co, oy1),
                (hx1 - ci, oy1),
            ],
        ));
    }
    for (x, w) in [(16.0, 0.62), (24.0, 0.615)] {
        let act = act_at(x, 4.0);
        let hole = grown(act, 0.4);
        let outer = grown(hole, 0.8);
        e.push(rb(p.activ, act));
        e.push(rb(p.nbl, grown(outer, 0.3)));
        e.push(rect(p.nw, outer.0, outer.1, outer.2, hole.1));
        e.push(rect(p.nw, outer.0, hole.3, outer.2, outer.3));
        e.push(rect(p.nw, outer.0, hole.1, hole.0, hole.3));
        e.push(rect(p.nw, hole.2, hole.1, hole.2 + w - 0.2, hole.3));
        e.push(rect(p.nw, hole.2 + 0.2, hole.1, hole.2 + w, hole.3));
    }
    let act = act_at(2.0, 12.0);
    e.extend(iso_ring(p, act, 0.8, None));
    let hole = grown(act, 0.4);
    let outer = grown(hole, 0.8);
    e.pop();
    e.push(rect(p.nw, hole.2, hole.1, hole.2 + 0.615, hole.3));
    e.push(rect(p.nw, hole.2, hole.1, outer.2, act.1));
    e.push(rect(p.nw, hole.2, act.3, outer.2, hole.3));
    let act = act_at(9.0, 12.0);
    let hole = grown(act, 0.4);
    let outer = grown(hole, 1.5);
    e.extend(iso_r(p, act, hole, outer, grown(outer, 0.3)));
    e.pop();
    let (sx0, sy0) = (hole.2 + 0.615, act.1 + 0.4);
    e.push(rect(p.nw, hole.2, hole.1, sx0, hole.3));
    e.push(rect(p.nw, sx0 + 0.2, hole.1, outer.2, hole.3));
    e.push(rect(p.nw, sx0, hole.1, sx0 + 0.2, sy0));
    e.push(rect(p.nw, sx0, sy0 + 0.2, sx0 + 0.2, hole.3));
    write_gz(&format!("{NMOSI}/nmosi.d.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines, the right leg 0.615: its outer wall on x = 20; the leg across
    // x = 40; across x = 42; the inner wall on x = 21; at (1000, 1000); a 300 µm Activ
    // with a 0.615 top leg.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 0.615 - 0.4 - 2.0, 4.0),
        (37.3, 4.0),
        (39.3, 12.0),
        (21.0 - 0.4 - 2.0, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(iso_ring(p, act_at(x, y), 0.8, Some((0, 0.615))));
    }
    e.extend(iso_ring(p, (70.0, 4.0, 370.0, 5.0), 0.8, Some((1, 0.615))));
    write_gz(&format!("{NMOSI}/nmosi.d.h3.gds.gz"), library("TOP", e));

    // h6, the conditions: an NWell finger 0.5 wide crossing onto an nBuLay with no Activ
    // anywhere (no ring, nothing to ring; clean); a C - the ring without its right leg -
    // 0.615 wide round an Activ (not unbroken; clean); a closed 0.615 ring round nothing
    // (round no Iso-PWell-Activ; clean); a proper 0.8 ring round an Activ and, on the
    // same nBuLay, a second NWell finger 0.5 wide crossing the nBuLay's edge (the finger
    // is not the ring; clean).
    let mut e = vec![
        rect(p.nbl, 2.0, 4.0, 6.0, 8.0),
        rect(p.nw, 3.0, 3.0, 3.5, 6.0),
    ];
    let act = act_at(9.0, 4.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.pop();
    let act = act_at(16.0, 4.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.remove(e.len() - 6);
    let act = act_at(2.0, 12.0);
    let hole = grown(act, 0.4);
    let outer = grown(hole, 0.8);
    e.extend(iso_r(
        p,
        act,
        hole,
        outer,
        (outer.0 - 0.3, outer.1 - 0.3, outer.2 + 3.0, outer.3 + 0.3),
    ));
    e.push(rect(
        p.nw,
        outer.2 + 1.0,
        outer.1 - 1.0,
        outer.2 + 1.5,
        act.3,
    ));
    write_gz(&format!("{NMOSI}/nmosi.d.h6.gds.gz"), library("TOP", e));
}

/// The 3.0 × 2.0 Activ at `(x, y)` in the standard ring, for the nSD:block and SalBlock
/// rules.
fn iso_big(p: &P, x: f64, y: f64) -> (R, Vec<GdsElement>) {
    let act = (x, y, x + 3.0, y + 2.0);
    (act, iso(p, act, 0.4, 0.7, 0.3))
}

/// nmosi.f - "Min. nSD:block width to separate ptap in nmosi  0.62".
fn nmosi_f(p: &P) {
    // h1, the bound: a vertical nSD:block strip through the Activ 0.62 wide (clean);
    // 0.615 (fires); a horizontal 0.615 strip (fires); a 0.615 stub ending inside the
    // Activ (fires); an L of 0.615 arms (fires on both arms).
    let mut e = vec![];
    for (x, y, w) in [(2.0, 4.0, 0.62), (9.0, 4.0, 0.615)] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.0 + w,
            act.3 + 0.2,
        ));
    }
    let (act, s) = iso_big(p, 16.0, 4.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 - 0.2,
        act.1 + 0.8,
        act.2 + 0.2,
        act.1 + 1.415,
    ));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.1 + 1.2,
    ));
    let (act, s) = iso_big(p, 9.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.1 + 1.4,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 + 0.785,
        act.2 + 0.2,
        act.1 + 1.4,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h1.gds.gz"), library("TOP", e));

    // h2, the drawing and 45°: the strip as two overlapping boxes 0.62 in union (clean)
    // and 0.615 (fires); a 45° strip 0.435·√2 = 0.6152 wide (fires) and 0.44·√2 = 0.6223
    // (clean); a 0.8 strip with a 0.185 bite (0.615 left; fires).
    let mut e = vec![];
    for (x, w) in [(2.0, 0.62), (9.0, 0.615)] {
        let (act, s) = iso_big(p, x, 4.0);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.0 + w - 0.2,
            act.3 + 0.2,
        ));
        e.push(rect(
            p.nsdb,
            act.0 + 1.2,
            act.1 - 0.2,
            act.0 + 1.0 + w,
            act.3 + 0.2,
        ));
    }
    for (x, d) in [(16.0, 0.435), (24.0, 0.44)] {
        let (act, s) = iso_big(p, x, 4.0);
        e.extend(s);
        let (x0, y0) = (act.0 + 0.3, act.1 - 0.2);
        e.push(poly(
            p.nsdb,
            &[
                (x0, y0),
                (x0 + 2.4, y0 + 2.4),
                (x0 + 2.4 - d, y0 + 2.4 + d),
                (x0 - d, y0 + d),
            ],
        ));
    }
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.8,
        act.1 + 0.6,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 + 1.4,
        act.0 + 1.8,
        act.3 + 0.2,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines, a 0.615 vertical strip: its right wall on x = 20; across
    // x = 40; across x = 42; its left wall on x = 21; at (1000, 1000); a 300 µm Activ with
    // a 0.615 horizontal strip.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.615, 4.0),
        (38.8, 4.0),
        (40.8, 12.0),
        (20.0, 12.0),
        (1000.0, 1000.0),
    ] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        e.push(rect(
            p.nsdb,
            act.0 + 1.0,
            act.1 - 0.2,
            act.0 + 1.615,
            act.3 + 0.2,
        ));
    }
    let act = (70.0, 4.0, 370.0, 6.0);
    e.extend(iso(p, act, 0.4, 0.7, 0.3));
    e.push(rect(
        p.nsdb,
        act.0 - 0.2,
        act.1 + 0.8,
        act.2 + 0.2,
        act.1 + 1.415,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h3.gds.gz"), library("TOP", e));

    // h6, the conditions: a 0.615 strip over an Activ in a ring but with no nBuLay (not
    // isolated; clean); a 0.615 strip over the Activ under PWell:block (clean); a 0.615
    // strip abutting the Activ's right edge, not over it (clean); one over it by 0.005
    // (fires); a 0.8 strip through the Activ with a 0.5 wide tab outside the Activ on the
    // nBuLay (the block is 0.8 wide where it separates; clean by the rule's words).
    let mut e = vec![];
    let (act, s) = iso_big(p, 2.0, 4.0);
    e.extend(s);
    e.remove(e.len() - 5);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 9.0, 4.0);
    e.extend(s);
    e.push(rb(p.pwb, grown(act, 0.1)));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 16.0, 4.0);
    e.extend(s);
    e.push(rect(p.nsdb, act.2, act.1 - 0.2, act.2 + 0.615, act.3 + 0.2));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.2 - 0.005,
        act.1 - 0.2,
        act.2 + 0.61,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 9.0, 12.0);
    e.extend(s);
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.8,
        act.3 + 0.2,
    ));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.3 + 0.2,
        act.0 + 1.5,
        act.3 + 1.0,
    ));
    write_gz(&format!("{NMOSI}/nmosi.f.h6.gds.gz"), library("TOP", e));
}

/// The nmosi.g instance: the 3.0 × 2.0 Activ in the standard ring, an nSD:block over
/// its left 1.5 (flush with the Activ on the other three sides), SalBlock over the
/// nSD:block and `ext` past its right edge.
fn gi(p: &P, x: f64, y: f64, ext: f64) -> (R, R, Vec<GdsElement>) {
    let (act, mut e) = iso_big(p, x, y);
    let blk = (act.0, act.1, act.0 + 1.5, act.3);
    e.push(rb(p.nsdb, blk));
    e.push(rect(p.sal, blk.0, blk.1, blk.2 + ext, blk.3));
    (act, blk, e)
}

/// nmosi.g - "Min. SalBlock overlap of nSD:block over Activ  0.15".
fn nmosi_g(p: &P) {
    // h1, the bound: SalBlock 0.15 past the nSD:block (clean); 0.145 (fires); no
    // SalBlock at all (fires); a 1.0 square nSD:block in the middle of the Activ with
    // SalBlock 0.15 round it (clean) and 0.145 on top only (fires); the SalBlock's
    // corner chamfered so it passes 0.1·√2 = 0.1414 from the block's corner (fires,
    // closest approach) and 0.11·√2 = 0.1556 (clean); SalBlock as two overlapping boxes
    // 0.15 past the block in union (clean) and 0.145 (fires).
    let mut e = vec![];
    e.extend(gi(p, 2.0, 4.0, 0.15).2);
    e.extend(gi(p, 9.0, 4.0, 0.145).2);
    let (_, _, mut s) = gi(p, 16.0, 4.0, 0.0);
    s.pop();
    e.extend(s);
    for (x, y, top, k) in [
        (2.0, 12.0, 0.15, None),
        (9.0, 12.0, 0.145, None),
        (16.0, 12.0, 0.15, Some(0.1)),
        (24.0, 12.0, 0.15, Some(0.11)),
    ] {
        let (act, s) = iso_big(p, x, y);
        e.extend(s);
        let b = (act.0 + 1.0, act.1 + 0.5, act.0 + 2.0, act.1 + 1.5);
        e.push(rb(p.nsdb, b));
        match k {
            None => e.push(rect(p.sal, b.0 - 0.15, b.1 - 0.15, b.2 + 0.15, b.3 + top)),
            Some(a) => e.push(chamfered_tr(
                p.sal,
                b.0 - 0.15,
                b.1 - 0.15,
                b.2 + 0.15,
                b.3 + 0.15,
                b.2 + b.3 + 2.0 * a,
            )),
        }
    }
    for (x, ext) in [(2.0, 0.15), (9.0, 0.145)] {
        let (_, blk, mut s) = gi(p, x, 20.0, 0.0);
        s.pop();
        e.extend(s);
        e.push(rect(p.sal, blk.0, blk.1, blk.2 + 0.05, blk.3));
        e.push(rect(p.sal, blk.0 + 1.0, blk.1, blk.2 + ext, blk.3));
    }
    write_gz(&format!("{NMOSI}/nmosi.g.h1.gds.gz"), library("TOP", e));

    // h2, the tile lines, 0.145: the SalBlock's edge on x = 20; the band across x = 40;
    // across x = 42; the block's edge on x = 21; at (1000, 1000); a 300 µm Activ with the
    // block 300 long and the SalBlock 0.145 past it on top.
    let mut e = vec![];
    for (x, y) in [
        (20.0 - 1.645, 4.0),
        (38.4, 4.0),
        (40.4, 12.0),
        (19.5, 12.0),
        (1000.0, 1000.0),
    ] {
        e.extend(gi(p, x, y, 0.145).2);
    }
    let act = (70.0, 4.0, 370.0, 6.0);
    e.extend(iso(p, act, 0.4, 0.7, 0.3));
    e.push(rect(p.nsdb, act.0, act.1, act.2, act.1 + 1.0));
    e.push(rect(p.sal, act.0, act.1, act.2, act.1 + 1.145));
    write_gz(&format!("{NMOSI}/nmosi.g.h2.gds.gz"), library("TOP", e));

    // h5, the conditions: the nSD:block and SalBlock flush with the Activ on every side
    // (nothing over Activ to cover; clean); the 0.145 instance with no nBuLay (clean); the
    // 0.145 instance with the Activ under PWell:block (clean); the SalBlock 0.145 past the
    // block but only outside the Activ - the block ends flush with the Activ's right edge
    // and the Activ continues below it (fires on the band under the block).
    let mut e = vec![];
    let (act, s) = iso_big(p, 2.0, 4.0);
    e.extend(s);
    e.push(rb(p.nsdb, act));
    e.push(rb(p.sal, act));
    let (_, _, s) = gi(p, 9.0, 4.0, 0.145);
    let mut s = s;
    s.remove(1);
    e.extend(s);
    let (act, _, s) = gi(p, 16.0, 4.0, 0.145);
    e.extend(s);
    e.push(rb(p.pwb, grown(act, 0.1)));
    let (act, s) = iso_big(p, 2.0, 12.0);
    e.extend(s);
    e.push(rect(p.nsdb, act.0, act.1 + 1.0, act.2, act.3));
    e.push(rect(p.sal, act.0, act.1 + 0.855, act.2 + 0.145, act.3));
    write_gz(&format!("{NMOSI}/nmosi.g.h5.gds.gz"), library("TOP", e));
}

/// The section's condition, "These rules will only be tested inside a closed ring of
/// NWell AND nBuLay": every rule's violating pattern with no NWell ring at all (the
/// Activ on a bare nBuLay), and with a C - the ring missing its right leg.
fn nmosi_ring(p: &P) {
    let mut e = vec![];
    // nmosi.b: nBuLay 1.0 past the Activ, no ring; a C.
    let act = act_at(2.0, 4.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.0)));
    let act = act_at(9.0, 4.0);
    e.extend(iso_enc(p, act, 1.4, Some((0, 1.235))));
    e.pop();
    // nmosi.c: a C 0.385 from the Activ on the left.
    let act = act_at(16.0, 4.0);
    e.extend(iso_gap(p, act, 0.6, Some((2, 0.385))));
    e.pop();
    // nmosi.d: a C 0.615 wide.
    let act = act_at(2.0, 12.0);
    e.extend(iso_ring(p, act, 0.615, None));
    e.pop();
    // nmosi.f: a 0.615 strip through an Activ on bare nBuLay; in a C.
    let act = (9.0, 12.0, 12.0, 14.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.4)));
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    let (act, s) = iso_big(p, 16.0, 12.0);
    e.extend(s);
    e.pop();
    e.push(rect(
        p.nsdb,
        act.0 + 1.0,
        act.1 - 0.2,
        act.0 + 1.615,
        act.3 + 0.2,
    ));
    // nmosi.g: SalBlock 0.145 past the block, on bare nBuLay; in a C.
    let act = (2.0, 20.0, 5.0, 22.0);
    e.push(rb(p.activ, act));
    e.push(rb(p.nbl, grown(act, 1.4)));
    e.push(rect(p.nsdb, act.0, act.1, act.0 + 1.5, act.3));
    e.push(rect(p.sal, act.0, act.1, act.0 + 1.645, act.3));
    let (_, _, mut s) = gi(p, 9.0, 20.0, 0.145);
    s.remove(s.len() - 3);
    e.extend(s);
    write_gz(&format!("{NMOSI}/nmosi.ring.h1.gds.gz"), library("TOP", e));
}

// ------------------------------------------------------------------------------------
// 7.4 Pin
// ------------------------------------------------------------------------------------

/// Pin.a-Pin.h - "Min. <layer> enclosure of <layer>:pin  0.00": "Pin areas must be
/// fully covered by drawing."
fn pin(p: &P) {
    let pin1 = (p.m1.0, 2);
    // h1, the bound on Metal1: the pin coincident with the metal on all four sides
    // (clean); the pin 0.005 past the metal on the right (fires); on top (fires); the pin
    // 0.5 clear of the metal (fires); the pin 0.1 inside the metal (clean); a 0.005 ×
    // 0.005 pin outside the metal (fires).
    let e = vec![
        rect(p.m1, 2.0, 2.0, 4.0, 3.0),
        rect(pin1, 2.0, 2.0, 4.0, 3.0),
        rect(p.m1, 6.0, 2.0, 8.0, 3.0),
        rect(pin1, 6.0, 2.0, 8.005, 3.0),
        rect(p.m1, 10.0, 2.0, 12.0, 3.0),
        rect(pin1, 10.0, 2.0, 12.0, 3.005),
        rect(p.m1, 14.0, 2.0, 16.0, 3.0),
        rect(pin1, 16.5, 2.0, 18.0, 3.0),
        rect(p.m1, 2.0, 6.0, 4.0, 7.0),
        rect(pin1, 2.1, 6.1, 3.9, 6.9),
        rect(p.m1, 6.0, 6.0, 8.0, 7.0),
        rect(pin1, 9.0, 6.0, 9.005, 6.005),
    ];
    write_gz(&format!("{PIN}/Pin.e.h1.gds.gz"), library("TOP", e));

    // h2, the drawing: a pin over two abutting metal boxes (clean); over two overlapping
    // ones (clean); the pin as four tiles over one metal (clean); a pin across a metal
    // ring's hole (fires on the hole); a pin over the ring's leg reaching 0.2 into the
    // hole (fires); a diamond pin inscribed in a square metal (clean); a square pin whose
    // corners poke 0.005 out of a diamond metal (fires four times); a diamond metal with a
    // diamond pin 0.005 larger (fires four times).
    let mut e = vec![
        rect(p.m1, 2.0, 2.0, 3.0, 3.0),
        rect(p.m1, 3.0, 2.0, 4.0, 3.0),
        rect(pin1, 2.2, 2.2, 3.8, 2.8),
        rect(p.m1, 6.0, 2.0, 7.2, 3.0),
        rect(p.m1, 6.8, 2.0, 8.0, 3.0),
        rect(pin1, 6.2, 2.2, 7.8, 2.8),
        rect(p.m1, 10.0, 2.0, 12.0, 3.0),
        rect(pin1, 10.0, 2.0, 11.0, 2.5),
        rect(pin1, 11.0, 2.0, 12.0, 2.5),
        rect(pin1, 10.0, 2.5, 11.0, 3.0),
        rect(pin1, 11.0, 2.5, 12.0, 3.0),
    ];
    e.extend(ring4(p.m1, (14.0, 2.0, 18.0, 6.0), (15.0, 3.0, 17.0, 5.0)));
    e.push(rect(pin1, 14.0, 3.5, 18.0, 4.5));
    e.extend(ring4(p.m1, (2.0, 6.0, 6.0, 10.0), (3.0, 7.0, 5.0, 9.0)));
    e.push(rect(pin1, 5.0, 7.5, 6.0, 8.5));
    e.push(rect(pin1, 4.8, 7.5, 5.2, 8.5));
    e.push(rect(p.m1, 8.0, 6.0, 12.0, 10.0));
    e.push(diamond(pin1, 10.0, 8.0, 2.0));
    e.push(diamond(p.m1, 16.0, 8.0, 2.0));
    e.push(rect(pin1, 15.0, 7.0, 17.005, 9.005));
    e.push(diamond(p.m1, 4.0, 14.0, 2.0));
    e.push(diamond(pin1, 4.0, 14.0, 2.005));
    write_gz(&format!("{PIN}/Pin.e.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines: pin and metal both ending on x = 20 (clean); the metal ending
    // at 19.995 under a pin ending on x = 20 (fires); a pin across x = 40 under a metal
    // across it (clean); a pin from 41 to 43 over a metal ending at 41.995 (fires); a
    // 300 µm pin on a 300 µm metal across every line (clean); an uncovered pin at
    // (1000, 1000) (fires); a text on Metal1:pin and one on Metal1:label with no metal
    // (texts are not areas; clean).
    let e = vec![
        rect(p.m1, 18.0, 2.0, 20.0, 3.0),
        rect(pin1, 18.0, 2.0, 20.0, 3.0),
        rect(p.m1, 18.0, 6.0, 19.995, 7.0),
        rect(pin1, 18.0, 6.0, 20.0, 7.0),
        rect(p.m1, 38.0, 2.0, 42.0, 3.0),
        rect(pin1, 39.0, 2.0, 41.0, 3.0),
        rect(p.m1, 40.0, 6.0, 41.995, 7.0),
        rect(pin1, 41.0, 6.0, 43.0, 7.0),
        rect(p.m1, 5.0, 12.0, 305.0, 13.0),
        rect(pin1, 5.0, 12.0, 305.0, 13.0),
        rect(pin1, 1000.0, 1000.0, 1001.0, 1001.0),
        text(pin1, "A", 10.0, 16.0),
        text(p.m1_label, "B", 12.0, 16.0),
    ];
    write_gz(&format!("{PIN}/Pin.e.h3.gds.gz"), library("TOP", e));

    // all.h1, every layer of the table: a pin 0.005 past its drawing (fires on Pin.a,
    // Pin.b, Pin.e, Pin.f four times, Pin.g, Pin.h) and a covered pin (clean) on each; a
    // Metal1:pin over Metal2 only (fires, Pin.e); pins on NWell:pin and Passiv:pin with
    // no drawing (no rule; clean).
    let mut e = vec![];
    let layers = [p.activ, p.gp, p.m1, p.m2, p.m3, p.m4, p.m5, p.tm1, p.tm2];
    for (i, l) in layers.iter().enumerate() {
        let y = 2.0 + 3.0 * i as f64;
        e.push(rect(*l, 2.0, y, 4.0, y + 1.0));
        e.push(rect((l.0, 2), 2.0, y, 4.005, y + 1.0));
        e.push(rect(*l, 6.0, y, 8.0, y + 1.0));
        e.push(rect((l.0, 2), 6.1, y + 0.1, 7.9, y + 0.9));
    }
    e.push(rect(p.m2, 10.0, 2.0, 12.0, 3.0));
    e.push(rect(pin1, 10.0, 2.0, 12.0, 3.0));
    e.push(rect((p.nw.0, 2), 10.0, 5.0, 12.0, 6.0));
    e.push(rect((9, 2), 10.0, 8.0, 12.0, 9.0));
    write_gz(&format!("{PIN}/Pin.all.h1.gds.gz"), library("TOP", e));
}

// ------------------------------------------------------------------------------------
// 7.1 Antenna
// ------------------------------------------------------------------------------------

/// The gate at `(x, y)`: Activ (x, y)-(x+1, y+0.5), GatPoly (x+0.4, y−0.3)-(x+0.6, y+0.8),
/// so the gate is 0.2 × 0.5 = 0.1 µm² and the poly over field 0.2 × 0.6 = 0.12 µm², and a
/// 0.1 × 0.1 Cont on the poly at (x+0.45, y+0.6)-(x+0.55, y+0.7).  Metal1 over the Cont
/// is the caller's.
fn gate(p: &P, x: f64, y: f64) -> Vec<GdsElement> {
    vec![
        rect(p.activ, x, y, x + 1.0, y + 0.5),
        rect(p.gp, x + 0.4, y - 0.3, x + 0.6, y + 0.8),
        rect(p.cont, x + 0.45, y + 0.6, x + 0.55, y + 0.7),
    ]
}

/// The gate at `(x, y)` with a Metal1 antenna `w` × `h` from (x, y+0.6) over its Cont.
fn gate_m1(p: &P, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    let mut e = gate(p, x, y);
    e.push(rect(p.m1, x, y + 0.6, x + w, y + 0.6 + h));
    e
}

/// An n-diode `w` × `h` at `(x, y)` - Activ with a Recog:diode marker, no pSD, no
/// NWell - with a Cont in its middle; the Metal1 over the Cont is the caller's.
fn ndiode(p: &P, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    vec![
        rect(p.activ, x, y, x + w, y + h),
        rect(p.diode, x, y, x + w, y + h),
        rect(
            p.cont,
            x + w / 2.0 - 0.05,
            y + h / 2.0 - 0.05,
            x + w / 2.0 + 0.05,
            y + h / 2.0 + 0.05,
        ),
    ]
}

fn antenna(p: &P) {
    ant_b(p);
    ant_d(p);
    ant_ac(p);
    ant_g(p);
    ant_hi(p);
}

/// Ant.b/e - "Max. ratio of cumulative metal area (from Metal1 to TopMetal2) to
/// connected Gate area  200.00 (without protection diode) / 20000.00 (with)".
fn ant_b(p: &P) {
    // h1, the bound, all inside one tile: Metal1 4.0 × 4.995 = 19.98 µm² on a 0.1 µm²
    // gate, ratio 199.8 (clean); 4.0 × 5.0 = 20.0, ratio 200.0 exactly (a maximum of 200
    // is met; clean); 4.0 × 5.005 = 20.02, ratio 200.2 (fires).
    let mut e = gate_m1(p, 2.0, 2.0, 4.0, 4.995);
    e.extend(gate_m1(p, 8.0, 2.0, 4.0, 5.0));
    e.extend(gate_m1(p, 14.0, 2.0, 4.0, 5.005));
    write_gz(&format!("{ANT}/Ant.b.h1.gds.gz"), library("TOP", e));

    // h2, the tile lines: a 0.5 wide Metal1 wire from the gate at x = 10 to x = 50.04
    // across x = 20 and 40, 40.04 × 0.5 = 20.02, ratio 200.2 (fires); the same wire to
    // x = 49.96, 19.98, ratio 199.8 (clean); a 0.7 wire from x = 10 ending on x = 40,
    // 30 × 0.7 = 21.0, ratio 210 (fires); an L: 20 × 0.5 = 10.0 along y = 22.6 from x = 10
    // across x = 20, then 0.5 × 20.04 = 10.02 up x = 29.5 across y = 40, abutting - 20.02
    // in union, ratio 200.2 (fires).
    let mut e = gate_m1(p, 10.0, 2.0, 40.04, 0.5);
    e.extend(gate_m1(p, 10.0, 6.0, 39.96, 0.5));
    e.extend(gate_m1(p, 10.0, 12.0, 30.0, 0.7));
    e.extend(gate_m1(p, 10.0, 22.0, 20.0, 0.5));
    e.push(rect(p.m1, 29.5, 23.1, 30.0, 43.14));
    write_gz(&format!("{ANT}/Ant.b.h2.gds.gz"), library("TOP", e));

    // h3, several gates and the union.  At x = 2 two gates under one 6.0 × 5.0 = 30 µm²
    // Metal1, gate area 0.2, ratio 150 (clean); at x = 10 one gate under 30 µm², ratio
    // 300 (fires).  At y = 10 the per-level sum of figure 7.1: gate G1 at x = 2 with
    // 3.9 × 5.0 = 19.5 µm² of Metal1 (195), gate G2 at x = 7 with 1.0 µm² (10), joined
    // at Metal2 by a 2.0 × 1.0 = 2.0 µm² plate over a Via1 on each (Metal2 level: 2.0 /
    // 0.2 = 10): G1 sums to 205 (fires), G2 to 20 (clean); on the final net the sum would
    // be 22.5 / 0.2 = 112.5 and nothing would fire.  At y = 18: the gate's Activ as two
    // overlapping boxes (gate still 0.1) under 20.02 (fires); Metal1 as two 4.0 × 3.0
    // boxes overlapping by 4.0 × 1.005 - 19.98 in union, 24 as a sum (clean).
    let mut e = gate(p, 2.0, 2.0);
    e.extend(gate(p, 5.0, 2.0));
    e.push(rect(p.m1, 2.0, 2.6, 8.0, 7.6));
    e.extend(gate_m1(p, 10.0, 2.0, 6.0, 5.0));
    e.extend(gate_m1(p, 2.0, 10.0, 3.9, 5.0));
    e.extend(gate_m1(p, 7.0, 10.0, 1.0, 1.0));
    e.push(rect(p.via1, 5.5, 11.1, 5.7, 11.3));
    e.push(rect(p.via1, 7.1, 11.1, 7.3, 11.3));
    e.push(rect(p.m2, 5.4, 10.9, 7.4, 11.9));
    e.extend(gate_m1(p, 2.0, 18.0, 4.0, 5.005));
    e.push(rect(p.activ, 2.5, 18.0, 3.5, 18.5));
    e.extend(gate(p, 10.0, 18.0));
    e.push(rect(p.m1, 10.0, 18.6, 14.0, 21.6));
    e.push(rect(p.m1, 10.0, 20.595, 14.0, 23.595));
    write_gz(&format!("{ANT}/Ant.b.h3.gds.gz"), library("TOP", e));

    // h4, the diode.  A gate under 30 µm² of Metal1 (300) whose Metal1 also reaches an
    // n-diode: 0.5 × 0.5 = 0.25 µm² at x = 2 (protected; clean); 0.4 × 0.4 = 0.16 exactly
    // at x = 10 (a diode of the minimum size protects; clean); 0.4 × 0.395 = 0.158 at
    // x = 18 (too small: Ant.g fires, and the net is unprotected, so Ant.b fires).  At
    // y = 20 a p-diode - Activ AND pSD AND Recog:diode in NWell - 0.25 µm² protects a 300
    // net (clean).  At x = 30, Ant.e: a 0.25 diode and 40.0 × 50.0 = 2000 µm² of Metal1,
    // ratio 20000 exactly (clean); at x = 80, 40.01 × 50.0 = 2000.5, ratio 20005 (fires).
    let mut e = vec![];
    for (x, w, h) in [(2.0, 0.5, 0.5), (10.0, 0.4, 0.4), (18.0, 0.4, 0.395)] {
        e.extend(gate_m1(p, x, 2.0, 6.0, 5.0));
        e.extend(ndiode(p, x + 4.5, 2.65, w, h));
    }
    e.extend(gate_m1(p, 2.0, 20.0, 6.0, 5.0));
    e.push(rect(p.nw, 6.0, 20.2, 7.5, 21.6));
    e.push(rect(p.activ, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.psd, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.diode, 6.5, 20.65, 7.0, 21.15));
    e.push(rect(p.cont, 6.7, 20.85, 6.8, 20.95));
    for (x, w) in [(30.0, 40.0), (80.0, 40.01)] {
        e.extend(gate_m1(p, x, 2.0, w, 50.0));
        e.extend(ndiode(p, x + 4.5, 2.65, 0.5, 0.5));
    }
    write_gz(&format!("{ANT}/Ant.b.h4.gds.gz"), library("TOP", e));

    // h6, the level the diode joins at: the gate's 6.0 × 5.0 = 30 µm² Metal1 (300) goes
    // up a Via1 to a Metal2 strip that comes down a Via1 onto a 0.25 µm² n-diode's own
    // Metal1 pad.  At the Metal1 level - the Metal1 etch - the net holds no diode and its
    // ratio is 300 (fires, by the per-level reading of figure 7.1); read off the final
    // net the diode protects it (KLayout).
    let mut e = gate_m1(p, 2.0, 2.0, 6.0, 5.0);
    e.push(rect(p.via1, 7.5, 4.0, 7.7, 4.2));
    e.push(rect(p.m2, 7.4, 3.9, 10.6, 4.3));
    e.push(rect(p.via1, 10.3, 4.0, 10.5, 4.2));
    e.push(rect(p.m1, 10.0, 3.5, 11.0, 4.5));
    e.extend(ndiode(p, 10.25, 3.75, 0.5, 0.5));
    write_gz(&format!("{ANT}/Ant.b.h6.gds.gz"), library("TOP", e));

    // h5, several metal layers.  A gate with 2.0 × 2.5 = 5.0 µm² of Metal1 (50), a Via1 to
    // 2.0 × 4.0 = 8.0 µm² of Metal2 (80), a Via2 to Metal3: 8.0 µm² at x = 2, the sum 210
    // (fires at Metal3); 6.0 µm² at x = 8, 190 (clean); 7.0 µm² at x = 14, 200 exactly
    // (clean).  At x = 2, y = 12 the stack to TopMetal2: Metal1 5.0 (50), then 0.5 × 0.5
    // pads on Metal2-Metal5 and TopMetal1 (2.5 each, 12.5) under 4.0 × 4.0 = 16 µm² of
    // TopMetal2 (160): 222.5 (fires at TopMetal2).
    let mut e = vec![];
    for (x, h3) in [(2.0, 4.0), (8.0, 3.0), (14.0, 3.5)] {
        e.extend(gate_m1(p, x, 2.0, 2.0, 2.5));
        e.push(rect(p.via1, x + 0.9, 3.9, x + 1.1, 4.1));
        e.push(rect(p.m2, x, 2.6, x + 2.0, 6.6));
        e.push(rect(p.via2, x + 0.9, 5.9, x + 1.1, 6.1));
        e.push(rect(p.m3, x, 2.6, x + 2.0, 2.6 + h3));
    }
    let (x, y) = (2.0, 12.0);
    e.extend(gate_m1(p, x, y, 2.0, 2.5));
    let (cx, cy) = (x + 1.0, y + 2.0);
    let pad = |l: (i16, i16), h: f64| rect(l, cx - h, cy - h, cx + h, cy + h);
    e.push(pad(p.via1, 0.1));
    e.push(pad(p.m2, 0.25));
    e.push(pad(p.via2, 0.1));
    e.push(pad(p.m3, 0.25));
    e.push(pad(p.via3, 0.1));
    e.push(pad(p.m4, 0.25));
    e.push(pad(p.via4, 0.1));
    e.push(pad(p.m5, 0.25));
    e.push(pad(p.tv1, 0.2));
    e.push(pad(p.tm1, 0.25));
    e.push(pad(p.tv2, 0.2));
    e.push(pad(p.tm2, 2.0));
    write_gz(&format!("{ANT}/Ant.b.h5.gds.gz"), library("TOP", e));
}

/// Ant.d/f - "Max. ratio of cumulative via area (from Via1 to TopVia2) to connected
/// Gate area  20.00 (without protection diode) / 500.00 (with)".
fn ant_d(p: &P) {
    // The gate under a Metal1 pad with one Via1 of `w` × `h` on it and a Metal2 pad over.
    let via = |x: f64, y: f64, w: f64, h: f64| {
        let mut e = gate_m1(p, x, y, w + 0.2, h + 0.2);
        e.push(rect(p.via1, x + 0.1, y + 0.7, x + 0.1 + w, y + 0.7 + h));
        e.push(rect(p.m2, x, y + 0.6, x + w + 0.2, y + 0.8 + h));
        e
    };
    // h1: a Via1 of 2.0 × 1.0 = 2.0 µm² on a 0.1 gate, ratio 20.0 exactly (clean); 2.0 ×
    // 1.005 = 2.01, 20.1 (fires); 2.0 × 0.995 = 1.99, 19.9 (clean).  At y = 8 the sum over
    // levels: Via1 1.0 × 1.0 (10) and Via2 1.0 × 1.01 (10.1) on the Metal2, 20.1 (fires).
    // At y = 14 with a 0.25 diode on the Metal1: Via1 2.01 (Ant.d does not apply, Ant.f's
    // 500 is far; clean) at x = 2; Via1 10.0 × 5.005 = 50.05, 500.5 (Ant.f fires) at
    // x = 8.  At y = 24 a 40.2 × 0.05 = 2.01 Via1 from x = 10 across x = 20 and 40 (fires).
    // At y = 28 two gates on one Metal1 with a 4.0 × 1.0 Via1, 4.0 / 0.2 = 20.0 (clean),
    // and at x = 12 with 4.0 × 1.005 = 4.02, 20.1 (fires).
    let mut e = via(2.0, 2.0, 2.0, 1.0);
    e.extend(via(6.0, 2.0, 2.0, 1.005));
    e.extend(via(10.0, 2.0, 2.0, 0.995));
    e.extend(via(2.0, 8.0, 1.0, 1.0));
    e.push(rect(p.via2, 2.1, 8.7, 3.1, 9.71));
    e.push(rect(p.m3, 2.0, 8.6, 3.2, 9.8));
    e.extend(via(2.0, 14.0, 2.0, 1.005));
    e.extend(ndiode(p, 3.4, 13.9, 0.5, 0.5));
    e.push(rect(p.m1, 3.3, 13.8, 4.0, 14.7));
    e.extend(via(8.0, 14.0, 10.0, 5.005));
    e.extend(ndiode(p, 18.4, 13.9, 0.5, 0.5));
    e.push(rect(p.m1, 17.8, 13.8, 19.0, 14.7));
    e.extend(gate_m1(p, 10.0, 24.0, 40.4, 0.25));
    e.push(rect(p.via1, 10.1, 24.7, 50.3, 24.75));
    e.push(rect(p.m2, 10.0, 24.65, 50.4, 24.8));
    for (x, h) in [(2.0, 1.0), (12.0, 1.005)] {
        e.extend(gate(p, x, 28.0));
        e.extend(gate(p, x + 3.0, 28.0));
        e.push(rect(p.m1, x, 28.6, x + 4.4, 29.8 + h));
        e.push(rect(p.via1, x + 0.2, 28.9, x + 4.2, 28.9 + h));
        e.push(rect(p.m2, x, 28.8, x + 4.4, 29.0 + h));
    }
    write_gz(&format!("{ANT}/Ant.d.h1.gds.gz"), library("TOP", e));
}

/// Ant.a - "Max. ratio of GatPoly over field oxide area to connected Gate area  200.00";
/// Ant.c - "Max. ratio of Cont area to connected Gate area  20.00".
fn ant_ac(p: &P) {
    // A poly pad `w` × `h` abutting the top of the gate's strip.
    let pad =
        |x: f64, y: f64, w: f64, h: f64| rect(p.gp, x + 0.4, y + 0.8, x + 0.4 + w, y + 0.8 + h);
    // h1.  Ant.a: the gate's 0.12 µm² of field poly plus a 4.0 × 4.97 = 19.88 pad, 20.0
    // µm² on 0.1, ratio 200.0 exactly at x = 2 (clean); a 4.0 × 4.975 = 19.9 pad, 200.2
    // at x = 16, the pad across x = 20 (fires).  At y = 10 one poly strip crossing two Activs 1.0 apart (gates 0.2,
    // field 0.32) with a 30 µm² pad, 151.6 (clean); at x = 10, y = 10 two gates whose
    // polys are separate, G1 with a 25 µm² pad (251.2) and G2 with 3 (31.2), strapped by
    // Metal1 over their Conts: poly is connected by poly, so G1 fires and G2 is clean; on
    // the whole net 28.24 / 0.2 = 141 and nothing would.  Ant.c, at y = 20, every gate
    // counting its own 0.01 µm² poly Cont: a Cont of 1.0 × 1.99 = 1.99 µm² on a 1.2 × 2.2
    // poly pad, 2.0 in all, ratio 20.0 (clean); 1.0 × 1.995, 2.005, 20.05 (fires); two
    // gates, G1 with a 2.5 µm² Cont (25.1) and G2 with 0.5 (5.1), strapped by Metal1 (G1
    // fires; the whole net would be 15.1); a gate with a 2.0 µm² Cont on its Activ beside
    // the poly and no metal (the Cont is not on the gate's net; clean).
    let mut e = gate(p, 2.0, 2.0);
    e.push(pad(2.0, 2.0, 4.0, 4.97));
    e.extend(gate(p, 16.0, 2.0));
    e.push(pad(16.0, 2.0, 4.0, 4.975));
    e.push(rect(p.activ, 2.0, 10.0, 3.0, 10.5));
    e.push(rect(p.activ, 2.0, 11.5, 3.0, 12.0));
    e.push(rect(p.gp, 2.4, 9.7, 2.6, 12.3));
    e.push(rect(p.gp, 2.4, 12.3, 7.4, 18.3));
    e.extend(gate(p, 10.0, 10.0));
    e.push(pad(10.0, 10.0, 5.0, 5.0));
    e.extend(gate(p, 16.0, 10.0));
    e.push(pad(16.0, 10.0, 1.0, 3.0));
    e.push(rect(p.m1, 10.4, 10.55, 16.6, 10.75));
    let cpad = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            rect(p.gp, x + 0.4, y + 0.8, x + 0.6 + w, y + 1.0 + h),
            rect(p.cont, x + 0.5, y + 0.9, x + 0.5 + w, y + 0.9 + h),
        ]
    };
    e.extend(gate(p, 2.0, 20.0));
    e.extend(cpad(2.0, 20.0, 1.0, 1.99));
    e.extend(gate(p, 6.0, 20.0));
    e.extend(cpad(6.0, 20.0, 1.0, 1.995));
    e.extend(gate(p, 10.0, 20.0));
    e.extend(cpad(10.0, 20.0, 1.0, 2.5));
    e.extend(gate(p, 14.0, 20.0));
    e.extend(cpad(14.0, 20.0, 1.0, 0.5));
    e.push(rect(p.m1, 10.4, 20.8, 15.6, 21.5));
    e.push(rect(p.activ, 18.0, 20.0, 21.0, 21.0));
    e.push(rect(p.gp, 18.4, 19.7, 18.6, 21.3));
    e.push(rect(p.cont, 19.0, 20.0, 21.0, 21.0));
    write_gz(&format!("{ANT}/Ant.ac.h1.gds.gz"), library("TOP", e));
}

/// Ant.g - "Size of protection diode (µm²) (Note 4)  0.16"; note 4: "PDarea (µm²) =
/// 0.02 x (Vn_area / (GatPoly over Activ)_area)", Vn_area the cumulative Cont and via
/// area.
fn ant_g(p: &P) {
    // h1: a gate under a small Metal1 whose Metal1 reaches an n-diode of 0.4 × 0.4 = 0.16
    // at x = 2 (clean); 0.4 × 0.395 = 0.158 at x = 6 (fires); a 0.1 × 0.1 diode under its
    // own Metal1, on no gate's net, at x = 10 (clean); a 0.158 diode reached through
    // Via1-Metal2-Via1 at x = 14 (fires); a p-diode of 0.158 in an NWell at x = 2, y = 8
    // (fires).  At y = 16: the 0.16 diode as two
    // abutting 0.2 × 0.4 boxes (clean); a 0.4 × 0.395 Activ under a 1 × 1 marker (fires);
    // a 1 × 1 Activ under a 0.4 × 0.395 marker (the diode is Activ AND marker; fires).
    let m1 = |x: f64| rect(p.m1, x, 2.6, x + 2.5, 3.6);
    let mut e = vec![];
    for (x, w, h) in [(2.0, 0.4, 0.4), (6.0, 0.4, 0.395)] {
        e.extend(gate(p, x, 2.0));
        e.push(m1(x));
        e.extend(ndiode(p, x + 1.5, 2.9, w, h));
    }
    e.extend(ndiode(p, 10.0, 2.0, 0.1, 0.1));
    e.push(rect(p.m1, 9.9, 1.9, 10.5, 2.5));
    e.extend(gate(p, 14.0, 2.0));
    e.push(rect(p.m1, 14.0, 2.6, 15.0, 3.6));
    e.push(rect(p.via1, 14.7, 3.0, 14.9, 3.2));
    e.push(rect(p.m2, 14.6, 2.9, 16.4, 3.3));
    e.push(rect(p.via1, 16.1, 3.0, 16.3, 3.2));
    e.push(rect(p.m1, 16.0, 2.5, 17.0, 3.5));
    e.extend(ndiode(p, 16.3, 2.7, 0.4, 0.395));
    e.extend(gate(p, 2.0, 8.0));
    e.push(rect(p.m1, 2.0, 8.6, 4.5, 9.6));
    e.push(rect(p.nw, 3.0, 8.4, 4.5, 9.8));
    e.push(rect(p.activ, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.psd, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.diode, 3.5, 8.9, 3.9, 9.295));
    e.push(rect(p.cont, 3.65, 9.05, 3.75, 9.15));
    e.extend(gate(p, 2.0, 16.0));
    e.push(rect(p.m1, 2.0, 16.6, 4.5, 17.6));
    e.push(rect(p.activ, 3.5, 16.9, 3.7, 17.3));
    e.push(rect(p.activ, 3.7, 16.9, 3.9, 17.3));
    e.push(rect(p.diode, 3.5, 16.9, 3.9, 17.3));
    e.push(rect(p.cont, 3.65, 17.05, 3.75, 17.15));
    e.extend(gate(p, 6.0, 16.0));
    e.push(rect(p.m1, 6.0, 16.6, 8.5, 17.6));
    e.push(rect(p.activ, 7.5, 16.9, 7.9, 17.295));
    e.push(rect(p.diode, 7.2, 16.6, 8.2, 17.6));
    e.push(rect(p.cont, 7.65, 17.05, 7.75, 17.15));
    e.extend(gate(p, 10.0, 16.0));
    e.push(rect(p.m1, 10.0, 16.6, 12.5, 17.6));
    e.push(rect(p.activ, 11.2, 16.6, 12.2, 17.6));
    e.push(rect(p.diode, 11.5, 16.9, 11.9, 17.295));
    e.push(rect(p.cont, 11.65, 17.05, 11.75, 17.15));
    write_gz(&format!("{ANT}/Ant.g.h1.gds.gz"), library("TOP", e));

    // h2, note 4: a 0.25 µm² n-diode on a net whose Via1 is 2.0 × 2.0 = 4.0 µm² on a 0.1
    // gate - Vn_area / gate area = 40.1 with the poly Cont, under Ant.f's 500 - so the
    // diode must be 0.02 × 40.1 = 0.80 µm² (fires by note 4; both tools read the 0.16
    // only).
    let mut e = gate(p, 2.0, 2.0);
    e.push(rect(p.m1, 2.0, 2.6, 5.5, 5.0));
    e.push(rect(p.via1, 3.0, 3.0, 5.0, 5.0));
    e.push(rect(p.m2, 2.9, 2.9, 5.1, 5.1));
    e.extend(ndiode(p, 2.2, 3.0, 0.5, 0.5));
    write_gz(&format!("{ANT}/Ant.g.h2.gds.gz"), library("TOP", e));
}

/// Ant.h - "dantenna in NWell not allowed"; Ant.i - "dpantenna in PWell not allowed".
fn ant_hi(p: &P) {
    // A 1 × 1 n-diode (Activ AND Recog:diode, no pSD) at (x, y).
    let nd = |x: f64, y: f64| {
        vec![
            rect(p.activ, x, y, x + 1.0, y + 1.0),
            rect(p.diode, x, y, x + 1.0, y + 1.0),
        ]
    };
    // Ant.h.h1: an n-diode in an NWell across x = 20 (fires); one with Recog:esd over it
    // (an ESD device; clean); one whose right half is in the NWell (fires on that half);
    // an isolbox - the diode under nBuLay with an "isolbox" TEXT inside the nBuLay (clean);
    // the same with "ISOLBOX" (clean); the text 0.5 outside the nBuLay (fires); the text
    // on Metal1:label (fires); the text "isolbox2" (fires); the diode's Activ under
    // nSD:block (not N+; clean); an n-diode under PWell:block with no NWell (clean).
    let mut e = vec![rect(p.nw, 18.5, 1.5, 21.5, 4.5)];
    e.extend(nd(19.5, 2.5));
    e.push(rect(p.nw, 1.0, 1.0, 4.0, 4.0));
    e.extend(nd(2.0, 2.0));
    e.push(rect(p.esd, 1.5, 1.5, 3.5, 3.5));
    e.push(rect(p.nw, 8.5, 1.0, 11.0, 4.0));
    e.extend(nd(8.0, 2.0));
    for (i, (label, lay, dx)) in [
        ("isolbox", p.txt, 0.0),
        ("ISOLBOX", p.txt, 0.0),
        ("isolbox", p.txt, 2.0),
        ("isolbox", p.m1_label, 0.0),
        ("isolbox2", p.txt, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        let (x, y) = (2.0 + 6.0 * i as f64, 8.0);
        e.push(rect(p.nw, x - 1.0, y - 1.0, x + 2.0, y + 2.0));
        e.extend(nd(x, y));
        e.push(rect(p.nbl, x - 0.5, y - 0.5, x + 1.5, y + 1.5));
        e.push(text(lay, label, x + 0.5 + dx, y + 0.5));
    }
    e.push(rect(p.nw, 1.0, 13.0, 4.0, 16.0));
    e.extend(nd(2.0, 14.0));
    e.push(rect(p.nsdb, 1.5, 13.5, 3.5, 15.5));
    e.push(rect(p.pwb, 7.0, 13.0, 10.0, 16.0));
    e.extend(nd(8.0, 14.0));
    write_gz(&format!("{ANT}/Ant.h.h1.gds.gz"), library("TOP", e));

    // A 1 × 1 p-diode (Activ AND pSD AND Recog:diode) at (x, y).
    let pd = |x: f64, y: f64| {
        vec![
            rect(p.activ, x, y, x + 1.0, y + 1.0),
            rect(p.psd, x, y, x + 1.0, y + 1.0),
            rect(p.diode, x, y, x + 1.0, y + 1.0),
        ]
    };
    // Ant.i.h1: a p-diode on bare substrate across x = 20 (fires); in an NWell (clean);
    // its right half in an NWell (fires on the left half); under PWell:block (not PWell
    // by section 4.2; clean); on bare substrate with Recog:esd (clean); a P+ Activ with
    // no Recog:diode on bare substrate (a ptap, no diode; clean).
    let mut e = pd(19.5, 2.5);
    e.push(rect(p.nw, 1.0, 1.0, 4.0, 4.0));
    e.extend(pd(2.0, 2.0));
    e.push(rect(p.nw, 8.5, 1.0, 11.0, 4.0));
    e.extend(pd(8.0, 2.0));
    e.push(rect(p.pwb, 1.0, 7.0, 4.0, 10.0));
    e.extend(pd(2.0, 8.0));
    e.extend(pd(8.0, 8.0));
    e.push(rect(p.esd, 7.5, 7.5, 9.5, 9.5));
    e.push(rect(p.activ, 2.0, 14.0, 3.0, 15.0));
    e.push(rect(p.psd, 2.0, 14.0, 3.0, 15.0));
    write_gz(&format!("{ANT}/Ant.i.h1.gds.gz"), library("TOP", e));

    // Ant.i.h2: a p-diode in an NWell under a drawn PWell - PWell:drawing is PWell by
    // section 4.2, so the diode is in PWell (fires; both tools read NWell OR PWell:block
    // only).
    let mut e = vec![
        rect(p.nw, 1.0, 1.0, 4.0, 4.0),
        rect(p.pw, 1.5, 1.5, 3.5, 3.5),
    ];
    e.extend(pd(2.0, 2.0));
    write_gz(&format!("{ANT}/Ant.i.h2.gds.gz"), library("TOP", e));
}
