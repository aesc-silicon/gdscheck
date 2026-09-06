// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deep N-well: a good and a bad pattern for every rule in the `dnwell` deck.
//!
//! The first patterns here that need *nets*.  DN.2a and DN.2b bound the same distance
//! twice - 2.5 µm between deep wells at the same potential and 5.42 between wells at
//! different ones - so a fixture has to say which two wells are which, and it says it the
//! way a layout does: an N+ tap on each well, a contact on each tap, and one Metal1 plate
//! over both contacts or two plates, one each.
//!
//! Every well also sits in the hole of a P+ guard ring, which DN.3 asks of it.  One ring
//! around both wells is enough and is what the pair fixtures draw: the rule objects to a
//! ring shared with a *foreign* N-type region, and a second deep well is not foreign.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/dnwell";

/// The deep well, over DN.1's 1.7 µm minimum.
const WELL: f64 = 4.0;
/// The N+ tap that takes a well up to Metal1, and the contact in it.
const TAP: f64 = 1.0;
const CO: f64 = 0.22;
/// The guard ring's wall, and its clearance to the wells inside it.
const RING: f64 = 2.0;
const RING_GAP: f64 = 2.0;

struct Ctx {
    dnwell: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
}

/// One deep well with its tap, and where that tap's contact sits.
fn well(c: &Ctx, x: f64, y: f64, w: f64) -> (Vec<GdsElement>, (f64, f64)) {
    let (cx, cy) = (x + w * 0.5, y + WELL * 0.5);
    let h = TAP * 0.5;
    let v = vec![
        rect(c.dnwell, x, y, x + w, y + WELL),
        rect(c.comp, cx - h, cy - h, cx + h, cy + h),
        rect(
            c.nplus,
            cx - h - 0.2,
            cy - h - 0.2,
            cx + h + 0.2,
            cy + h + 0.2,
        ),
        rect(
            c.contact,
            cx - CO * 0.5,
            cy - CO * 0.5,
            cx + CO * 0.5,
            cy + CO * 0.5,
        ),
    ];
    (v, (cx, cy))
}

/// A P+ guard ring whose hole holds everything between `x0,y0` and `x1,y1`.
fn ring(c: &Ctx, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
    ring_sized(c, x0, y0, x1, y1, RING_GAP, RING)
}

/// A ring `gap` out from the box and `width` wide.  The pair fixtures put two wells 2.5 um
/// apart, and DN.3 wants each in a ring of its own - a ring's interior that touches two
/// wells surrounds neither - so theirs are narrow and close.
fn ring_sized(
    c: &Ctx,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    gap: f64,
    width: f64,
) -> Vec<GdsElement> {
    let (a, b) = (gap, gap + width);
    let (ox0, oy0, ox1, oy1) = (x0 - b, y0 - b, x1 + b, y1 + b);
    let (ix0, iy0, ix1, iy1) = (x0 - a, y0 - a, x1 + a, y1 + a);
    let mut v = Vec::new();
    for l in [c.comp, c.pplus] {
        // Four bars, which merge into a ring with a hole.
        v.push(rect(l, ox0, oy0, ox1, iy0));
        v.push(rect(l, ox0, iy1, ox1, oy1));
        v.push(rect(l, ox0, iy0, ix0, iy1));
        v.push(rect(l, ix1, iy0, ox1, iy1));
    }
    v
}

/// Metal1 over the given contact centres: one plate is one net.
fn strap(c: &Ctx, centres: &[(f64, f64)]) -> GdsElement {
    let m = CO * 0.5 + 0.2;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in centres {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    rect(c.metal1, x0 - m, y0 - m, x1 + m, y1 + m)
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        dnwell: layer(pdk, "dnwell"),
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };

    // One well in its ring, strapped on its own.
    let single = |w: f64| {
        let (mut v, p) = well(&c, o, o, w);
        v.extend(ring(&c, o, o, o + w, o + WELL));
        v.push(strap(&c, &[p]));
        v
    };
    // Two wells `gap` apart, each in its own ring, on one net or on two.
    let pair = |gap: f64, same_net: bool| {
        let (mut v, p1) = well(&c, o, o, WELL);
        let x2 = o + WELL + gap;
        let (w2, p2) = well(&c, x2, o, WELL);
        v.extend(w2);
        v.extend(ring_sized(&c, o, o, o + WELL, o + WELL, 0.5, 0.5));
        v.extend(ring_sized(&c, x2, o, x2 + WELL, o + WELL, 0.5, 0.5));
        if same_net {
            v.push(strap(&c, &[p1, p2]));
        } else {
            v.push(strap(&c, &[p1]));
            v.push(strap(&c, &[p2]));
        }
        v
    };

    for id in ["DN.1", "DN.2a", "DN.2b", "DN.3"] {
        write(id, "good", single(WELL));
    }
    // The pair rules' clean halves are the pair at a legal distance.
    write("DN.2a", "good", pair(2.5, true));
    write("DN.2b", "good", pair(5.42, false));

    // DN.1: a deep well narrower than 1.7 µm.
    write("DN.1", "bad", single(1.695));
    // DN.2a: two wells at one potential, 2.5 µm apart.
    write("DN.2a", "bad", pair(2.495, true));
    // DN.2b: two wells at different potentials, 5.42 µm apart.
    write("DN.2b", "bad", pair(5.415, false));
    // DN.3: a well with no guard ring around it.
    write("DN.3", "bad", {
        let (mut v, p) = well(&c, o, o, WELL);
        v.push(strap(&c, &[p]));
        v
    });
}
