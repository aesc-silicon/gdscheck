// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Guard ring: a good and a bad pattern for every rule in the `guard_ring` deck.
//!
//! The ring is drawn the way a real seal ring is, scaled down: a band `B` wide with a
//! 45° chamfer `C` across each corner, its active laid out as four
//! corner pieces and four bars between them, the marker one annulus over the same
//! outline, and P+ over the active.  Three rules are statements about that shape rather
//! than about a distance and so fire on every other fixture until it is right - GR.1 wants
//! the marker and the active to share an outline, GR.3 wants the P+ over the active, and
//! GR.11 wants a pad on the marker - which is why every fixture starts from the same ring
//! and perturbs one thing.
//!
//! The real ring's own numbers are the good halves here: its active band is 16 µm, which
//! is exactly GR.6's minimum, and its metal 15 µm against GR.4's 12.
//!
//! `seal` is that ring at something nearer chip scale, with the die it guards: metal on
//! every level standing 8.8 µm off the marker where GR.2 asks ten, the way the ring this
//! was modelled on stood, and its contacts and vias filling a stretch of the band at
//! exactly the 0.7 µm GR.7 and GR.8 allow.  It spans twenty tiles each way, so a wall
//! that a tile reads from its neighbour's copy has to be reported by someone.

use crate::helpers::{layer, library, poly, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/guard_ring";
/// Lower left of the ring's outline.
const O: f64 = 10.0;
const W: f64 = 160.0;
const H: f64 = 120.0;
/// The band, and the chamfer across each corner - both the real ring's.
const B: f64 = 16.0;
const C: f64 = 9.0;
/// The `seal` ring's outline, and how far its die's metal stands off the marker.
const SEAL_W: f64 = 400.0;
const SEAL_H: f64 = 300.0;
const DIE_GAP: f64 = 8.8;
/// The die's metal blocks: `BLOCK` long along the wall, `BLOCK_DEPTH` into the die, at
/// `BLOCK_PITCH`.
const BLOCK: f64 = 10.0;
const BLOCK_DEPTH: f64 = 6.0;
const BLOCK_PITCH: f64 = 15.0;
const D: f64 = 0.005;

/// The eight layers GR.2 keeps clear of the marker.
const CLEARED: &[&str] = &[
    "comp",
    "nwell",
    "poly2_drawn",
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

const METALS: &[&str] = &[
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

/// A ring band `b` wide on layer `l`, as the eight pieces the real ring is drawn as:
/// four corners carrying the 45° chamfer, four bars between them.
fn band(l: (i16, i16), b: f64) -> Vec<GdsElement> {
    band_at(l, b, (O, W, H))
}

/// [`band`] round an outline with lower left `o`, `w` wide and `h` high.
fn band_at(l: (i16, i16), b: f64, (o, w, h): (f64, f64, f64)) -> Vec<GdsElement> {
    let (x1, y1) = (o + w, o + h);
    vec![
        poly(
            l,
            &[
                (o + C, o),
                (o, o + C),
                (o, o + b),
                (o + b, o + b),
                (o + b, o),
            ],
        ),
        poly(
            l,
            &[
                (x1 - C, o),
                (x1 - b, o),
                (x1 - b, o + b),
                (x1, o + b),
                (x1, o + C),
            ],
        ),
        poly(
            l,
            &[
                (x1, y1 - C),
                (x1, y1 - b),
                (x1 - b, y1 - b),
                (x1 - b, y1),
                (x1 - C, y1),
            ],
        ),
        poly(
            l,
            &[
                (o + C, y1),
                (o + b, y1),
                (o + b, y1 - b),
                (o, y1 - b),
                (o, y1 - C),
            ],
        ),
        rect(l, o + b, o, x1 - b, o + b),
        rect(l, o + b, y1 - b, x1 - b, y1),
        rect(l, o, o + b, o + b, y1 - b),
        rect(l, x1 - b, o + b, x1, y1 - b),
    ]
}

/// The marker: one annulus over a band `b` wide, written as the keyhole polygon the real
/// ring uses - the outer boundary with its chamfers, then the inner rectangle.
fn marker(l: (i16, i16), b: f64) -> GdsElement {
    marker_at(l, b, (O, W, H))
}

/// [`marker`] round an outline with lower left `o`, `w` wide and `h` high.
fn marker_at(l: (i16, i16), b: f64, (o, w, h): (f64, f64, f64)) -> GdsElement {
    let (x1, y1) = (o + w, o + h);
    poly(
        l,
        &[
            (o + C, o),
            (o, o + C),
            (o, y1 - b),
            (o + b, y1 - b),
            (o + b, o + b),
            (x1 - b, o + b),
            (x1 - b, y1 - b),
            (o, y1 - b),
            (o, y1 - C),
            (o + C, y1),
            (x1 - C, y1),
            (x1, y1 - C),
            (x1, o + C),
            (x1 - C, o),
        ],
    )
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let gr = layer(pdk, "guard_ring_mk");
    let pad = layer(pdk, "pad");
    let comp = layer(pdk, "comp");
    let pplus = layer(pdk, "pplus");

    // The ring every fixture starts from: the marker, the active under it, P+ over that,
    // metal on each level, and the pad GR.11 wants.
    let ring = |elems: &mut Vec<GdsElement>| {
        elems.push(marker(gr, B));
        elems.extend(band(comp, B));
        elems.extend(band(pplus, B));
        for m in METALS {
            elems.extend(band(layer(pdk, m), B - 1.0));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
    };

    // GR.2, ten microns from the marker to each of nine layers.  They sit inside the
    // ring's hole, which is where the die they belong to is.
    for (name, g) in [("good", 10.0), ("bad", 10.0 - D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        for (i, lname) in CLEARED.iter().enumerate() {
            let x = O + B + g + i as f64 * 6.0;
            elems.push(rect(
                layer(pdk, lname),
                x,
                O + B + g,
                x + 4.0,
                O + B + g + 4.0,
            ));
        }
        write(&format!("GR.2.{name}"), elems);
    }

    // GR.4, the ring's metal is at least 12 µm wide at each of six levels.  The real ring
    // draws it one micron inside the active, at 15.
    for (name, w) in [("good", 15.0), ("bad", 12.0 - D)] {
        let mut elems = vec![marker(gr, B)];
        elems.extend(band(comp, B));
        elems.extend(band(pplus, B));
        for m in METALS {
            elems.extend(band(layer(pdk, m), w));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
        write(&format!("GR.4.{name}"), elems);
    }

    // GR.6, the ring's own P+ active is at least 16 µm wide - which is exactly what the
    // real one is.  The marker follows the active, so that GR.1 stays satisfied.
    for (name, w) in [("good", 0.0), ("bad", D)] {
        let mut elems = vec![marker(gr, B - w)];
        elems.extend(band(comp, B - w));
        elems.extend(band(pplus, B - w));
        for m in METALS {
            elems.extend(band(layer(pdk, m), B - 1.0 - w));
        }
        elems.push(rect(
            pad,
            O + W - 12.0,
            O + H - 12.0,
            O + W - 2.0,
            O + H - 2.0,
        ));
        write(&format!("GR.6.{name}"), elems);
    }

    // GR.1: the marker reaching past the ring's active.
    for (name, over) in [("good", 0.0), ("bad", D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        if over > 0.0 {
            elems.push(rect(gr, O + W, O + B, O + W + over, O + H - B));
        }
        write(&format!("GR.1.{name}"), elems);
    }

    // GR.3: the ring's active with the implant stopping short of it.
    for (name, short) in [("good", 0.0), ("bad", D)] {
        let mut elems = Vec::new();
        ring(&mut elems);
        if short > 0.0 {
            elems.retain(|e| {
                !matches!(e, GdsElement::GdsBoundary(b)
                if (b.layer, b.datatype) == pplus)
            });
            // The implant slides off one end of the band rather than narrowing it, so the
            // active it leaves bare is GR.3's business and stays GR.6's right width.
            let mut pp = band(pplus, B);
            pp[4] = rect(pplus, O + B + short, O, O + W - B, O + B);
            elems.extend(pp);
        }
        write(&format!("GR.3.{name}"), elems);
    }

    // GR.7 and GR.8, the ring's own contacts and vias no closer than 0.7 µm.  They sit in
    // the band, which is what puts them in the ring; the real ring is built to exactly
    // this spacing on every level it uses.
    for (id, names) in [
        ("GR.7", &["contact"][..]),
        ("GR.8", &["via1", "via2", "via3", "via4"][..]),
    ] {
        for (half, s) in [("good", 0.7), ("bad", 0.7 - D)] {
            let mut elems = Vec::new();
            ring(&mut elems);
            for (i, lname) in names.iter().enumerate() {
                let l = layer(pdk, lname);
                let y = O + 2.0 + i as f64 * 2.0;
                elems.push(rect(l, O + B + 2.0, y, O + B + 2.22, y + 0.22));
                elems.push(rect(l, O + B + 2.22 + s, y, O + B + 2.44 + s, y + 0.22));
            }
            write(&format!("{id}.{half}"), elems);
        }
    }

    // GR.11: the ring needs a pad.  The bad half is the one fixture here without one.
    let mut good = Vec::new();
    ring(&mut good);
    write("GR.11.good", good);
    let mut bad = vec![marker(gr, B)];
    bad.extend(band(comp, B));
    bad.extend(band(pplus, B));
    for m in METALS {
        bad.extend(band(layer(pdk, m), B - 1.0));
    }
    write("GR.11.bad", bad);

    seal(pdk);
    hardening(pdk);
}

/// The seal ring with its die: 400 x 300 µm, the die's metal a row of blocks along each
/// wall 8.8 µm off the marker on every level, a stretch of the band across two tile
/// lines filled with contacts and vias at 0.7 µm.
fn seal(pdk: &PdkConfig) {
    let ring = (O, SEAL_W, SEAL_H);
    let (x1, y1) = (O + SEAL_W, O + SEAL_H);
    let mut elems = vec![marker_at(layer(pdk, "guard_ring_mk"), B, ring)];
    elems.extend(band_at(layer(pdk, "comp"), B, ring));
    elems.extend(band_at(layer(pdk, "pplus"), B, ring));
    for m in METALS {
        elems.extend(band_at(layer(pdk, m), B - 1.0, ring));
    }
    elems.push(rect(
        layer(pdk, "pad"),
        x1 - 12.0,
        y1 - 12.0,
        x1 - 2.0,
        y1 - 2.0,
    ));
    // The die's metal along each wall as a row of blocks, each its own region and so its
    // own GR.2 marker: a spacing rule reports one pair once, and a die drawn as one plate
    // would be one marker per level however many tiles its walls cross.
    let (dx0, dy0, dx1, dy1) = (
        O + B + DIE_GAP,
        O + B + DIE_GAP,
        x1 - B - DIE_GAP,
        y1 - B - DIE_GAP,
    );
    for m in METALS {
        let l = layer(pdk, m);
        let mut x = dx0;
        while x + BLOCK <= dx1 {
            elems.push(rect(l, x, dy0, x + BLOCK, dy0 + BLOCK_DEPTH));
            elems.push(rect(l, x, dy1 - BLOCK_DEPTH, x + BLOCK, dy1));
            x += BLOCK_PITCH;
        }
        let mut y = dy0 + BLOCK_DEPTH + 2.0;
        while y + BLOCK <= dy1 - BLOCK_DEPTH - 2.0 {
            elems.push(rect(l, dx0, y, dx0 + BLOCK_DEPTH, y + BLOCK));
            elems.push(rect(l, dx1 - BLOCK_DEPTH, y, dx1, y + BLOCK));
            y += BLOCK_PITCH;
        }
    }
    for (i, lname) in ["contact", "via1", "via2", "via3", "via4"]
        .iter()
        .enumerate()
    {
        let l = layer(pdk, lname);
        let y = O + 2.0 + i as f64 * 2.0;
        let mut x = O + 30.0;
        while x < O + 70.0 {
            elems.push(rect(l, x, y, x + 0.22, y + 0.22));
            x += 0.22 + 0.7;
        }
    }
    write("seal", elems);
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 12.4 (GR.1 to GR.11, `gf180mcu_drm/drm_12_4.txt`) and from the scribe
// line and die seal chapter those rules sit in (12.1 to 12.3), by someone who has not
// seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/guard_ring/GR.<n>.h<n>.gds.gz` with a case in the
// `hardening_guard_ring` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/guard_ring.md.
//
// A guard ring is a ring, so every fixture is one: a rectangular annulus written as a
// keyhole polygon, with the marker, the active and the implant on one outline, the five
// metals inside it, and a pad on the band.  The ring's own numbers are the rules' own
// bounds met exactly - the active band is 16 µm against GR.6's minimum and the metal
// 12 µm against GR.4's - so any fixture that does not say otherwise is a ring every rule
// of the section is content with, and the perturbation is the whole layout.
//
// The outline starts at 19, which puts the band across the tile lines at 20, 21 and 28
// and the hole's walls at 35, so that a die shape 9.995 µm off one of them has its gap
// across x = 35, x = 40 and x = 42 at once.
//
// What these draw is what only this section has: the coincidence GR.1 asks of the marker
// and the ring's active in both directions, GR.2's ten microns from the marker to each
// member of its long layer list and the metrics it is read with, GR.3's implant, GR.4's
// and GR.6's widths on a shape whose hole must not be measured, the marker's reach over a
// die trace that butts against it, GR.7's and GR.8's staggered rows, and GR.11's pad.
// The generic engine classes (metrics on a bare layer, 45° walls, unions, arrays,
// extremes) are the engine family's and are not redrawn.

/// The hardening ring's outline: lower left, and the two sizes used.
const HO: f64 = 19.0;
const HW: f64 = 160.0;
const HH: f64 = 120.0;
/// The ring's active band - GR.6's minimum, met exactly.
const HB: f64 = 16.0;
/// The ring's metal band - GR.4's minimum, met exactly.
const HM: f64 = 12.0;
/// Contact and via sides, from the table in section 12.2.
const HCO: f64 = 0.22;
const HVIA: f64 = 0.26;

/// The layers the hardening patterns draw on.
struct G {
    gr: (i16, i16),
    pad: (i16, i16),
    comp: (i16, i16),
    pplus: (i16, i16),
    nwell: (i16, i16),
    poly2: (i16, i16),
    metal: [(i16, i16); 5],
    contact: (i16, i16),
    via: [(i16, i16); 4],
}

impl G {
    fn new(pdk: &PdkConfig) -> Self {
        G {
            gr: layer(pdk, "guard_ring_mk"),
            pad: layer(pdk, "pad"),
            comp: layer(pdk, "comp"),
            pplus: layer(pdk, "pplus"),
            nwell: layer(pdk, "nwell"),
            poly2: layer(pdk, "poly2_drawn"),
            metal: [
                layer(pdk, "metal1_drawn"),
                layer(pdk, "metal2_drawn"),
                layer(pdk, "metal3_drawn"),
                layer(pdk, "metal4_drawn"),
                layer(pdk, "metal5_drawn"),
            ],
            contact: layer(pdk, "contact"),
            via: [
                layer(pdk, "via1"),
                layer(pdk, "via2"),
                layer(pdk, "via3"),
                layer(pdk, "via4"),
            ],
        }
    }
}

/// A rectangular annulus on `l`: the outline `(x0, y0)-(x1, y1)` with a band `b` wide on
/// the left, right, bottom and top, written as one keyhole polygon - the ring as a ring,
/// with a hole, rather than as four bars that happen to meet.
fn annulus(l: (i16, i16), x0: f64, y0: f64, x1: f64, y1: f64, b: [f64; 4]) -> GdsElement {
    let (ix0, ix1) = (x0 + b[0], x1 - b[1]);
    let (iy0, iy1) = (y0 + b[2], y1 - b[3]);
    poly(
        l,
        &[
            (x0, y0),
            (x1, y0),
            (x1, y1),
            (x0, y1),
            (x0, y0),
            (ix0, iy0),
            (ix0, iy1),
            (ix1, iy1),
            (ix1, iy0),
            (ix0, iy0),
        ],
    )
}

/// A pad inside the ring's top band, which is what GR.11 asks for.
fn hpad(g: &G, x1: f64, y1: f64, b: f64) -> GdsElement {
    rect(g.pad, x1 - 20.0, y1 - b + 2.0, x1 - 10.0, y1 - 2.0)
}

/// The ring every hardening fixture starts from: the marker, the active and the implant
/// on one outline with the band `b`, the five metals `m` wide flush with the outer edge,
/// and a pad on the top band.  Passing different bands for the marker and the active is
/// how the GR.1 fixtures break their coincidence.
#[allow(clippy::too_many_arguments)]
fn hring(
    g: &G,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    mk: [f64; 4],
    act: [f64; 4],
    pp: [f64; 4],
    m: [f64; 4],
) -> Vec<GdsElement> {
    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, mk),
        annulus(g.comp, x0, y0, x1, y1, act),
        annulus(g.pplus, x0, y0, x1, y1, pp),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, m));
    }
    e.push(hpad(g, x1, y1, mk[3]));
    e
}

/// The ring with everything coincident and every bound met exactly.
fn plain(g: &G, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    hring(g, x0, y0, x1, y1, [HB; 4], [HB; 4], [HB; 4], [HM; 4])
}

/// The standard ring: 160 x 120 µm at (19, 19), hole (35, 35)-(163, 123).
fn big(g: &G) -> Vec<GdsElement> {
    plain(g, HO, HO, HO + HW, HO + HH)
}

pub fn hardening(pdk: &PdkConfig) {
    let g = G::new(pdk);
    gr1(&g);
    gr2(&g);
    gr3(&g);
    gr4(&g);
    gr5(&g);
    gr6(&g);
    gr7(&g);
    gr11(&g);
}

// --- GR.1: "Min/Max GUARD_RING_MK overlap of guard ring comp: 0" ---

/// `GR.1.h1`: the rule's two bounds, one ring each, 40 µm apart.  A minimum and a maximum
/// that are both zero is a statement that the marker and the ring's active have to be the
/// same shape, so it is broken from either side: the left ring's marker stops 0.005 µm
/// short of the active on the inside, leaving active outside the marker (the maximum
/// broken), and the right ring's marker reaches 0.005 past it (the minimum broken).
/// Each is one violation of GR.1.
fn gr1(g: &G) {
    let mut e = hring(
        g,
        HO,
        HO,
        HO + HW,
        HO + HH,
        [HB, HB, HB, HB - 0.005],
        [HB; 4],
        [HB; 4],
        [HM; 4],
    );
    let x = HO + HW + 40.0;
    e.extend(hring(
        g,
        x,
        HO,
        x + HW,
        HO + HH,
        [HB, HB, HB, HB + 0.005],
        [HB; 4],
        [HB; 4],
        [HM; 4],
    ));
    write("GR.1.h1", e);

    // `GR.1.h2`: the same outline drawn two different ways.  The marker is one keyhole
    // annulus; the active is the eight pieces a real ring is drawn as - four corners and
    // four bars - which merge into that same annulus, and the implant is nine overlapping
    // boxes over it.  One shape drawn as many is still one shape, so the coincidence
    // holds and nothing fires.
    let (x0, y0, x1, y1) = (HO, HO, HO + HW, HO + HH);
    let (ix0, iy0, ix1, iy1) = (x0 + HB, y0 + HB, x1 - HB, y1 - HB);
    let mut e = vec![annulus(g.gr, x0, y0, x1, y1, [HB; 4])];
    for (bx0, by0, bx1, by1) in [
        (x0, y0, ix0, iy0),
        (ix1, y0, x1, iy0),
        (ix1, iy1, x1, y1),
        (x0, iy1, ix0, y1),
        (ix0, y0, ix1, iy0),
        (ix0, iy1, ix1, y1),
        (x0, iy0, ix0, iy1),
        (ix1, iy0, x1, iy1),
    ] {
        e.push(rect(g.comp, bx0, by0, bx1, by1));
    }
    // The implant as overlapping boxes: the four bars stretched over the corners.
    e.push(rect(g.pplus, x0, y0, x1, iy0));
    e.push(rect(g.pplus, x0, iy1, x1, y1));
    e.push(rect(g.pplus, x0, y0, ix0, y1));
    e.push(rect(g.pplus, ix1, y0, x1, y1));
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.1.h2", e);
}

// --- GR.2: "Min GUARD_RING_MK space to prime die COMP, NWELL, Poly2, Metal 1, 2, 3, 4,
// 5 and metal Top: 10" ---

/// `GR.2.h1`: the two metrics, measured to a corner the marker turns outwards.  A die
/// shape inside the hole always faces a wall of the marker, so the corner-to-corner case
/// needs a corner of the ring pointing into the die: two bumps 8 µm deep on the bottom
/// band, drawn on the marker, the active and the implant together so the rest of the
/// section stays satisfied.
///
/// The bumps are 16 µm along the wall, so the band stays 16 µm wide across them.
///
/// - A: a block whose corner is 7.0 µm right and 7.0 µm above the first bump's corner -
///   9.8995 µm as the crow flies and no facing wall at all.  Under 10: fires.
/// - B: the same at 7.075 - 10.0056: clean.
/// - C: a block with its wall 10.000 from the hole's top wall: clean.
/// - D: the same at 9.995: fires.
fn gr2(g: &G) {
    let y1 = HO + HH;
    let (ix0, iy0, iy1) = (HO + HB, HO + HB, y1 - HB);
    let mut e = big(g);
    for (bx0, bx1) in [(50.0, 66.0), (95.0, 111.0)] {
        for l in [g.gr, g.comp, g.pplus] {
            e.push(rect(l, bx0, iy0, bx1, iy0 + 8.0));
        }
    }
    let m1 = g.metal[0];
    // A: corner to corner at 9.8995 from the first bump's top right corner (66, 43).
    e.push(rect(m1, 73.0, 50.0, 81.0, 58.0));
    // B: the same from the second bump's corner (111, 43), at 10.0056.
    e.push(rect(m1, 118.075, 50.075, 126.075, 58.075));
    // C: wall to wall, 10.000 below the hole's top wall.
    e.push(rect(m1, 50.0, iy1 - 20.0, 60.0, iy1 - 10.0));
    // D: 9.995 below it.
    e.push(rect(m1, 80.0, iy1 - 20.0, 90.0, iy1 - 9.995));
    write("GR.2.h1", e);

    // `GR.2.h2`: no gap at all.  (a) a die Metal1 block whose left wall is the marker's
    // hole wall - a shared edge, a space of nothing, and nothing is under ten; (b) a
    // block crossing that wall by 2 µm, 14 µm square so that it clears GR.4 wherever the
    // marker is read to reach; (c) a block 9.995 µm outside the ring's outer wall, on the
    // scribe-line side, which is not the prime die the rule names.
    let mut e = big(g);
    e.push(rect(m1, ix0, 50.0, ix0 + 14.0, 64.0));
    e.push(rect(m1, ix0 - 2.0, 80.0, ix0 + 18.0, 100.0));
    e.push(rect(m1, 0.005, 50.0, HO - 9.995, 64.0));
    write("GR.2.h2", e);

    // `GR.2.h3`: the tile lines and the top of the stack.  The hole's walls are at 35, so
    // a 9.995 µm gap off one of them runs across x = 35, x = 40 and x = 42 - a 20 µm tile
    // line, a 7 µm one, and one of each again.  P is Metal1 off the left wall, Q is
    // Metal5 - which is variant D's MetalTop, the last member of the rule's layer list -
    // off the bottom wall, and R is Metal1 at exactly 10.000 off the left wall.
    let mut e = big(g);
    e.push(rect(m1, ix0 + 9.995, 50.0, ix0 + 21.995, 62.0));
    e.push(rect(g.metal[4], 80.0, iy0 + 9.995, 92.0, iy0 + 21.995));
    e.push(rect(m1, ix0 + 10.0, 80.0, ix0 + 22.0, 92.0));
    write("GR.2.h3", e);

    // `GR.2.h4`: the rest of the rule's layer list, each member once at 9.995 µm from the
    // hole's bottom wall - COMP, NWELL, Poly2 and Metal2, 3 and 4.  Metal1 and Metal5 are
    // in `GR.2.h3`; with these six that is every layer the rule names.
    let mut e = big(g);
    for (i, l) in [g.comp, g.nwell, g.poly2, g.metal[1], g.metal[2], g.metal[3]]
        .into_iter()
        .enumerate()
    {
        let x = 45.0 + i as f64 * 18.0;
        e.push(rect(l, x, iy0 + 9.995, x + 12.0, iy0 + 21.995));
    }
    write("GR.2.h4", e);
}

// --- GR.3: "Minimum Pplus overlap of PCOMP inside guard ring: 0" ---

/// `GR.3.h1`: the implant drawn 1 µm larger than the active on both edges of the band.
/// The rule asks for an overlap of at least zero, so more of it is not less legal, and
/// the ring stays clean.
///
/// `GR.3.h2`: the implant 0.005 µm short of the active on the outer edge of the whole
/// band.  That is active inside the ring with no implant over it, which is what GR.3
/// forbids; it is also what narrows the P+ active GR.6 measures, and no drawing separates
/// the two (the pair is recorded in `tests/gf180mcuD.rs`).
fn gr3(g: &G) {
    let (x0, y0, x1, y1) = (HO, HO, HO + HW, HO + HH);
    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, [HB; 4]),
        annulus(g.comp, x0, y0, x1, y1, [HB; 4]),
        annulus(
            g.pplus,
            x0 - 1.0,
            y0 - 1.0,
            x1 + 1.0,
            y1 + 1.0,
            [HB + 2.0; 4],
        ),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.3.h1", e);

    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, [HB; 4]),
        annulus(g.comp, x0, y0, x1, y1, [HB; 4]),
        annulus(
            g.pplus,
            x0 + 0.005,
            y0 + 0.005,
            x1 - 0.005,
            y1 - 0.005,
            [HB - 0.005; 4],
        ),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.3.h2", e);
}

// --- GR.4: "Minimum metal-n width (n = 1 to 6): 12" ---

/// `GR.4.h1`: every level's band 0.005 µm under the value on the bottom bar and exactly
/// 12 elsewhere.  Variant D's stack is five metals - Metal5 is its MetalTop - so the
/// manual's "n = 1 to 6" is five rules here and the fixture is short on all five.
///
/// `GR.4.h2`: what the marker reaches.  The rules are to be checked "for the real ring
/// area recognized by GUARD_RING_MK", so a 2 µm die trace that ends on the marker's wall
/// without crossing it is not ring metal and its width is not GR.4's business, while one
/// that starts on the ring's own metal under the marker and runs out into the die is part
/// of the ring's metal and drags GR.4 onto its 2 µm.  (a) is the first, on Metal1; (b) is
/// the second, on Metal2; 12.3.2 permits (b) as a Vss bus butted to the ring, which is
/// the note in the report.
fn gr4(g: &G) {
    let (x0, y0, x1, y1) = (HO, HO, HO + HW, HO + HH);
    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, [HB; 4]),
        annulus(g.comp, x0, y0, x1, y1, [HB; 4]),
        annulus(g.pplus, x0, y0, x1, y1, [HB; 4]),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM, HM, HM - 0.005, HM]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.4.h1", e);

    let mut e = big(g);
    // (a) a Metal1 trace whose left edge is the marker's hole wall, running into the die.
    e.push(rect(g.metal[0], x0 + HB, 50.0, x0 + HB + 20.0, 52.0));
    // (b) a Metal2 trace starting on the ring's own metal, under the marker, and running
    // out through the hole's wall into the die.
    e.push(rect(g.metal[1], x0 + HM - 2.0, 80.0, x0 + HB + 20.0, 82.0));
    write("GR.4.h2", e);
}

// --- GR.5: "Minimum metal-n outer edge space to die edge": Solder Bump 3, Other Cases -
// ---

/// `GR.5.h1`: the ring's metal set 2.995 µm in from the outer edge of the marker, which
/// is the die edge the rule names.  Under the manual's Solder Bump column that is GR.5
/// broken on all five levels; under Other Cases the column reads "-" and there is no
/// rule to break.  gdscheck implements the Other Cases column throughout - its GR.11
/// requires a pad rather than forbidding one - so the layout is clean, and the fixture is
/// here to say which column the deck is.
fn gr5(g: &G) {
    let (x0, y0, x1, y1) = (HO, HO, HO + HW, HO + HH);
    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, [HB; 4]),
        annulus(g.comp, x0, y0, x1, y1, [HB; 4]),
        annulus(g.pplus, x0, y0, x1, y1, [HB; 4]),
    ];
    for l in g.metal {
        e.push(annulus(
            l,
            x0 + 2.995,
            y0 + 2.995,
            x1 - 2.995,
            y1 - 2.995,
            [HM; 4],
        ));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.5.h1", e);
}

// --- GR.6: "Min PCOMP width: 16" ---

/// `GR.6.h1`: the active, the implant and the marker together 0.005 µm under the value on
/// the bottom bar.  One wall of the ring is too narrow; the rest is exactly 16.
///
/// `GR.6.h2`: the active 16 µm wide with the implant covering only 15.995 of it, so the
/// P+ active - which is what the rule names - is 15.995 and the COMP is not.  GR.3 fires
/// with it, because the strip of active the implant leaves bare is what GR.3 forbids.
///
/// `GR.6.h3`: a ring 46 µm square with a 16 µm band, so its hole is 14 µm across - under
/// the value, and empty.  A width is a width of material; nothing here may fire.
fn gr6(g: &G) {
    let (x0, y0, x1, y1) = (HO, HO, HO + HW, HO + HH);
    let narrow = [HB, HB, HB - 0.005, HB];
    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, narrow),
        annulus(g.comp, x0, y0, x1, y1, narrow),
        annulus(g.pplus, x0, y0, x1, y1, narrow),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.6.h1", e);

    let mut e = vec![
        annulus(g.gr, x0, y0, x1, y1, [HB; 4]),
        annulus(g.comp, x0, y0, x1, y1, [HB; 4]),
        annulus(g.pplus, x0, y0 + 0.005, x1, y1, [HB, HB, HB - 0.005, HB]),
    ];
    for l in g.metal {
        e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
    }
    e.push(hpad(g, x1, y1, HB));
    write("GR.6.h2", e);

    write("GR.6.h3", plain(g, HO, HO, HO + 46.0, HO + 46.0));
}

// --- GR.7 and GR.8: contact to contact and via to via, 0.7 ---

/// `GR.7.h1`: the ring's own contacts, in the band, where section 12.2 puts them - "the
/// minimum size contacts/vias placed in staggered formation with separation of 0.7µm".
///
/// - P1 0.700 apart along x: clean.  P2 0.695: fires.
/// - P3 staggered - 0.42 along x and 0.55 along y, so the two squares face each other
///   over nothing at all and the only distance between them is the 0.6920 µm diagonal.
///   A space is euclidian, so it fires.  P4 is the same staggered pair at 0.50 and 0.50,
///   0.7071 apart: clean.
/// - P5 0.695 apart with the gap across the tile line at x = 21.
/// - P6 0.3 apart out in the die, where the ring's rule does not reach: clean.
///
/// `GR.8.h1`: one pair 0.695 apart on each of Via1 to Via4 - variant D has no Via5 - and
/// one Via1 pair out in the die.
fn gr7(g: &G) {
    let mut e = big(g);
    let sq = |l: (i16, i16), x: f64, y: f64, s: f64| rect(l, x, y, x + s, y + s);
    let pair = |e: &mut Vec<GdsElement>, l, x: f64, y: f64, dx: f64, dy: f64, s: f64| {
        e.push(sq(l, x, y, s));
        e.push(sq(l, x + s + dx, y + dy, s));
    };
    // In the band: y = 24 is inside the bottom bar, which runs from 19 to 35.
    pair(&mut e, g.contact, 40.0, 24.0, 0.7, 0.0, HCO);
    pair(&mut e, g.contact, 45.0, 24.0, 0.695, 0.0, HCO);
    pair(&mut e, g.contact, 50.0, 24.0, 0.42, 0.55 + HCO, HCO);
    pair(&mut e, g.contact, 55.0, 24.0, 0.5, 0.5 + HCO, HCO);
    // The gap from 20.8 to 21.495 runs across the tile line at 21.
    pair(&mut e, g.contact, 20.58, 24.0, 0.695, 0.0, HCO);
    // Out in the die, 0.3 apart: not the ring's contacts.
    pair(&mut e, g.contact, 60.0, 60.0, 0.3, 0.0, HCO);
    write("GR.7.h1", e);

    let mut e = big(g);
    for (i, l) in g.via.into_iter().enumerate() {
        pair(&mut e, l, 40.0 + i as f64 * 5.0, 24.0, 0.695, 0.0, HVIA);
    }
    pair(&mut e, g.via[0], 60.0, 60.0, 0.3, 0.0, HVIA);
    write("GR.8.h1", e);
}

// --- GR.11: "Pad opening on top of GUARD_RING_MK layer: Required" ---

/// `GR.11.h1`: three small rings 30 µm apart.  The first has its pad on the band, the
/// second has none at all, and the third has one just outside the marker sharing its
/// outer edge.  "On top of" is a pad over the marker; a pad that only touches it is not
/// on top of anything, so the second and the third both want a pad opening and have not
/// got one.
fn gr11(g: &G) {
    let (w, h) = (60.0, 50.0);
    let mut e = Vec::new();
    for (i, pad) in [0u8, 1, 2].into_iter().enumerate() {
        let x0 = HO + i as f64 * (w + 30.0);
        let (y0, x1, y1) = (HO, x0 + w, HO + h);
        e.push(annulus(g.gr, x0, y0, x1, y1, [HB; 4]));
        e.push(annulus(g.comp, x0, y0, x1, y1, [HB; 4]));
        e.push(annulus(g.pplus, x0, y0, x1, y1, [HB; 4]));
        for l in g.metal {
            e.push(annulus(l, x0, y0, x1, y1, [HM; 4]));
        }
        match pad {
            0 => e.push(hpad(g, x1, y1, HB)),
            2 => e.push(rect(g.pad, x0 + 10.0, y1, x0 + 20.0, y1 + 8.0)),
            _ => {}
        }
    }
    write("GR.11.h1", e);
}
