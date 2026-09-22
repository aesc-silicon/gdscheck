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
use crate::helpers::{layer, library, poly, rect, write_gz};
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
    hardening(pdk);
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

// Hardening patterns (hardening/SPEC.md, the GF180MCU section): layouts drawn from the
// manual's section 7.2 by someone who has not seen the engine.  Each is a
// `tests/data/gf180mcuD/generated/dnwell/DN.<rule>.h<n>.gds.gz` with a case in the
// `hardening_dnwell` table of `tests/gf180mcuD.rs`; the findings are in
// hardening/reports/gf180mcuD/dnwell.md.
//
// The deck's own conditions here are nets - which diffusion ties a deep well to Metal1
// and which does not - and the P+ guard ring DN.3 asks for: what counts as a ring, and
// what it may hold besides the well.

/// Layers the hardening patterns draw on.
struct L {
    dn: (i16, i16),
    nw: (i16, i16),
    pw: (i16, i16),
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    co: (i16, i16),
    m1: (i16, i16),
}

impl L {
    fn new(pdk: &PdkConfig) -> Self {
        L {
            dn: layer(pdk, "dnwell"),
            nw: layer(pdk, "nwell"),
            pw: layer(pdk, "lvpwell"),
            comp: layer(pdk, "comp"),
            nplus: layer(pdk, "nplus"),
            pplus: layer(pdk, "pplus"),
            co: layer(pdk, "contact"),
            m1: layer(pdk, "metal1_drawn"),
        }
    }

    /// A contacted diffusion `size` on a side centred on `(cx, cy)`, N+ or P+; the
    /// contact's centre comes back for the strap.
    fn diff(
        &self,
        cx: f64,
        cy: f64,
        size: f64,
        implant: (i16, i16),
    ) -> (Vec<GdsElement>, (f64, f64)) {
        let h = size * 0.5;
        (
            vec![
                rect(self.comp, cx - h, cy - h, cx + h, cy + h),
                rect(
                    implant,
                    cx - h - 0.1,
                    cy - h - 0.1,
                    cx + h + 0.1,
                    cy + h + 0.1,
                ),
                rect(self.co, cx - 0.11, cy - 0.11, cx + 0.11, cy + 0.11),
            ],
            (cx, cy),
        )
    }

    /// A deep well `(x0, y0)-(x1, y1)` with a 1 µm N+ tap at its centre.
    fn well(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (Vec<GdsElement>, (f64, f64)) {
        let (mut v, p) = self.diff((x0 + x1) * 0.5, (y0 + y1) * 0.5, 1.0, self.nplus);
        v.push(rect(self.dn, x0, y0, x1, y1));
        (v, p)
    }

    /// A P+ ring whose outer box is `(x0, y0)-(x1, y1)`, `w` wide, as four bars.
    fn ring(&self, x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<GdsElement> {
        let mut v = Vec::new();
        for l in [self.comp, self.pplus] {
            v.push(rect(l, x0, y0, x1, y0 + w));
            v.push(rect(l, x0, y1 - w, x1, y1));
            v.push(rect(l, x0, y0 + w, x0 + w, y1 - w));
            v.push(rect(l, x1 - w, y0 + w, x1, y1 - w));
        }
        v
    }

    /// The ring the pair fixtures use: 0.5 out from the box, 0.5 wide.
    fn ring_around(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GdsElement> {
        self.ring(x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0, 0.5)
    }

    /// One Metal1 plate over the contact centres: one net.
    fn strap(&self, pts: &[(f64, f64)]) -> GdsElement {
        let m = 0.11 + 0.2;
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        rect(self.m1, x0 - m, y0 - m, x1 + m, y1 + m)
    }

    /// Two 4 × 4 wells `gap` apart, the left one at `(x, y)`, each in its own ring, on
    /// one net or two.
    fn pair(&self, x: f64, y: f64, gap: f64, same_net: bool) -> Vec<GdsElement> {
        let (mut v, a) = self.well(x, y, x + 4.0, y + 4.0);
        let bx = x + 4.0 + gap;
        let (w, b) = self.well(bx, y, bx + 4.0, y + 4.0);
        v.extend(w);
        v.extend(self.ring_around(x, y, x + 4.0, y + 4.0));
        v.extend(self.ring_around(bx, y, bx + 4.0, y + 4.0));
        if same_net {
            v.push(self.strap(&[a, b]));
        } else {
            v.push(self.strap(&[a]));
            v.push(self.strap(&[b]));
        }
        v
    }
}

fn write(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

fn hardening(pdk: &PdkConfig) {
    let l = L::new(pdk);
    dn_1_h(&l);
    dn_2a_h(&l);
    dn_2b_h(&l);
    dn_3_h(&l);
}

// --- DN.1: min. DNWELL width 1.7 ---

fn dn_1_h(l: &L) {
    // h1 - the bound, in a ring so DN.3 is quiet: a 1.695 × 6 well, DN.1; a 1.7 × 6
    // well, clean.
    write("DN.1.h1", {
        let (mut v, a) = l.well(2.0, 2.0, 3.695, 8.0);
        v.extend(l.ring_around(2.0, 2.0, 3.695, 8.0));
        v.push(l.strap(&[a]));
        let (w, b) = l.well(12.0, 2.0, 13.7, 8.0);
        v.extend(w);
        v.extend(l.ring_around(12.0, 2.0, 13.7, 8.0));
        v.push(l.strap(&[b]));
        v
    });
}

// --- DN.2a: min. DNWELL space, equi-potential, 2.5 ---

fn dn_2a_h(l: &L) {
    // An 8 × 6 well with a slot `d` wide and 3 deep cut into its top, tapped in its
    // left arm, in a ring.
    let notched = |x: f64, y: f64, d: f64| -> Vec<GdsElement> {
        let (mut v, p) = l.diff(x + 1.5, y + 1.5, 1.0, l.nplus);
        v.push(l.strap(&[p]));
        v.push(poly(
            l.dn,
            &[
                (x, y),
                (x + 8.0, y),
                (x + 8.0, y + 6.0),
                (x + 4.0 + d * 0.5, y + 6.0),
                (x + 4.0 + d * 0.5, y + 3.0),
                (x + 4.0 - d * 0.5, y + 3.0),
                (x + 4.0 - d * 0.5, y + 6.0),
                (x, y + 6.0),
            ],
        ));
        v.extend(l.ring_around(x, y, x + 8.0, y + 6.0));
        v
    };
    // h1 - the bound.  Two wells on one net (a tap each, one Metal1 plate) at 2.495,
    // DN.2a; at 2.5, clean; a slot 2.495 wide into one well, DN.2a - a notch is a
    // space at the same potential; at 2.5, clean.
    write(
        "DN.2a.h1",
        [
            l.pair(2.0, 2.0, 2.495, true), // DN.2a
            l.pair(24.0, 2.0, 2.5, true),  // clean
            notched(2.0, 16.0, 2.495),     // DN.2a
            notched(24.0, 16.0, 2.5),      // clean
        ]
        .concat(),
    );

    // h2 - one potential through an N-well.  Two 4 × 4 wells 2.495 apart, each holding
    // a 3 × 3 N-well (the deep well 0.5 around it) whose tap is the only diffusion, and
    // one Metal1 plate over both taps: the N-wells short the deep wells, DN.2a.
    write("DN.2a.h2", {
        let mut v = Vec::new();
        let mut pts = Vec::new();
        for x in [2.0, 8.495] {
            let (t, p) = l.diff(x + 2.0, 4.0, 1.0, l.nplus);
            v.extend(t);
            pts.push(p);
            v.push(rect(l.nw, x + 0.5, 2.5, x + 3.5, 5.5));
            v.push(rect(l.dn, x, 2.0, x + 4.0, 6.0));
            v.extend(l.ring_around(x, 2.0, x + 4.0, 6.0));
        }
        v.push(l.strap(&pts));
        v
    });

    // h3 - the tile lines.  One-net pairs at 2.495 with the gap straddling x = 20 and
    // x = 42 and the strap crossing it, DN.2a twice; two-net pairs at 5.415 straddling
    // x = 40 and x = 21, DN.2b twice.
    write(
        "DN.2a.h3",
        [
            l.pair(14.5, 2.0, 2.495, true),   // gap 18.5-20.995: DN.2a
            l.pair(36.5, 2.0, 2.495, true),   // gap 40.5-42.995: DN.2a
            l.pair(33.5, 16.0, 5.415, false), // gap 37.5-42.915: DN.2b
            l.pair(14.5, 16.0, 5.415, false), // gap 18.5-23.915: DN.2b
        ]
        .concat(),
    );
}

// --- DN.2b: min. DNWELL space, different potential, 5.42 ---

fn dn_2b_h(l: &L) {
    // h1 - the bound.  Two taps, two plates: at 5.415, DN.2b; at 5.42, clean; at
    // 2.495, DN.2b - two potentials, so the equi-potential rule is not the one.
    write(
        "DN.2b.h1",
        [
            l.pair(2.0, 2.0, 5.415, false),  // DN.2b
            l.pair(26.0, 2.0, 5.42, false),  // clean
            l.pair(2.0, 16.0, 2.495, false), // DN.2b
        ]
        .concat(),
    );

    // h2 - what does not tie a deep well.  Three pairs of 7 × 7 wells 5.415 apart, the
    // left one tapped in its N region: (a) the right one holds a 2 × 2 P-well (2.5
    // inside the deep well) with a contacted N+ diffusion in it - an NMOS terminal in
    // the isolated P-well, not a deep-well tap - strapped to the left tap: two
    // potentials, DN.2b; (b) the left plate runs over the right well without touching
    // its tap, which has a plate of its own: DN.2b; (c) the right diffusion is P+ in
    // the N region - a PMOS terminal - strapped to the left tap: DN.2b.
    write("DN.2b.h2", {
        let mut v = Vec::new();
        for (i, y) in [2.0, 18.0, 34.0].into_iter().enumerate() {
            let (w, a) = l.well(2.0, y, 9.0, y + 7.0);
            v.extend(w);
            v.extend(l.ring_around(2.0, y, 9.0, y + 7.0));
            let bx = 14.415;
            v.push(rect(l.dn, bx, y, bx + 7.0, y + 7.0));
            v.extend(l.ring_around(bx, y, bx + 7.0, y + 7.0));
            let c = (bx + 3.5, y + 3.5);
            match i {
                0 => {
                    v.push(rect(l.pw, c.0 - 1.0, c.1 - 1.0, c.0 + 1.0, c.1 + 1.0));
                    let (d, p) = l.diff(c.0, c.1, 0.6, l.nplus);
                    v.extend(d);
                    v.push(l.strap(&[a, p]));
                }
                1 => {
                    let (d, p) = l.diff(c.0, c.1, 1.0, l.nplus);
                    v.extend(d);
                    v.push(l.strap(&[p]));
                    v.push(rect(l.m1, a.0 - 0.31, y + 0.3, bx + 6.5, y + 0.8));
                    v.push(rect(l.m1, a.0 - 0.31, y + 0.3, a.0 + 0.31, a.1 + 0.31));
                }
                _ => {
                    let (d, p) = l.diff(c.0, c.1, 1.0, l.pplus);
                    v.extend(d);
                    v.push(l.strap(&[a, p]));
                }
            }
        }
        v
    });

    // h3 - two wells with no tap at all, 2.495 apart, each in its ring: nothing ties
    // them to one potential, so they may sit at two, DN.2b.
    write("DN.2b.h3", {
        let mut v = vec![
            rect(l.dn, 2.0, 2.0, 6.0, 6.0),
            rect(l.dn, 8.495, 2.0, 12.495, 6.0),
        ];
        v.extend(l.ring_around(2.0, 2.0, 6.0, 6.0));
        v.extend(l.ring_around(8.495, 2.0, 12.495, 6.0));
        v
    });
}

// --- DN.3: each DNWELL directly surrounded by a PCOMP guard ring ---

fn dn_3_h(l: &L) {
    // A 4 × 4 tapped well at (x, y) with its own plate, no ring.
    let bare = |x: f64, y: f64| -> Vec<GdsElement> {
        let (mut v, p) = l.well(x, y, x + 4.0, y + 4.0);
        v.push(l.strap(&[p]));
        v
    };
    // h1 - rings that do not surround.  (a) the ring's right wall has a 0.5 gap - a C,
    // not a ring: DN.3; (b) three walls P+, the fourth N+: not a PCOMP ring, DN.3;
    // (c) one ring around two wells 2.5 apart on one net: neither is *directly*
    // surrounded, DN.3 twice; (d) a ring holding an N-well beside the deep well, 3.1
    // from it: the ring is shared with foreign N-type, DN.3; (e) a ring holding an N+
    // diffusion beside the deep well: DN.3.
    let mut v = bare(2.0, 2.0); // (a)
    for lay in [l.comp, l.pplus] {
        v.push(rect(lay, 1.0, 1.0, 7.0, 1.5));
        v.push(rect(lay, 1.0, 6.5, 7.0, 7.0));
        v.push(rect(lay, 1.0, 1.5, 1.5, 6.5));
        v.push(rect(lay, 6.5, 1.5, 7.0, 3.75));
        v.push(rect(lay, 6.5, 4.25, 7.0, 6.5));
    }
    v.extend(bare(14.0, 2.0)); // (b)
    v.push(rect(l.comp, 13.0, 1.0, 19.0, 1.5));
    v.push(rect(l.comp, 13.0, 6.5, 19.0, 7.0));
    v.push(rect(l.comp, 13.0, 1.5, 13.5, 6.5));
    v.push(rect(l.comp, 18.5, 1.5, 19.0, 6.5));
    v.push(rect(l.pplus, 12.9, 0.9, 19.1, 1.6));
    v.push(rect(l.pplus, 12.9, 6.4, 19.1, 7.1));
    v.push(rect(l.pplus, 12.9, 1.4, 13.6, 6.6));
    v.push(rect(l.nplus, 18.4, 1.6, 19.1, 6.4));
    // (c)
    {
        let (w, a) = l.well(26.0, 2.0, 30.0, 6.0);
        v.extend(w);
        let (w, b) = l.well(32.5, 2.0, 36.5, 6.0);
        v.extend(w);
        v.push(l.strap(&[a, b]));
        v.extend(l.ring(25.0, 1.0, 37.5, 7.0, 0.5));
    }
    // (d)
    v.extend(bare(2.0, 16.0));
    v.push(rect(l.nw, 9.1, 16.0, 12.1, 19.0));
    v.extend(l.ring(1.0, 15.0, 13.1, 21.0, 0.5));
    // (e)
    v.extend(bare(22.0, 16.0));
    v.extend(l.diff(28.5, 18.0, 1.0, l.nplus).0);
    v.extend(l.ring(21.0, 15.0, 30.0, 21.0, 0.5));
    write("DN.3.h1", v);

    // h2 - rings that do.  (f) a ring holding a P+ diffusion beside the deep well and
    // an N-well inside it; (g) a ring abutting the well; (h) a ring with 45° corners;
    // (i) a ring drawn as sixteen boxes; (j) a ring inside a ring.  All clean.
    let mut v = bare(2.0, 2.0); // (f)
    v.extend(l.diff(8.5, 4.0, 1.0, l.pplus).0);
    v.push(rect(l.nw, 2.5, 2.5, 5.5, 5.5));
    v.extend(l.ring(1.0, 1.0, 10.0, 7.0, 0.5));
    v.extend(bare(16.0, 2.0)); // (g)
    v.extend(l.ring(15.5, 1.5, 20.5, 6.5, 0.5));
    v.extend(bare(28.0, 2.0)); // (h): four bars and four 45° corner strips
    for lay in [l.comp, l.pplus] {
        v.push(rect(lay, 28.0, 0.5, 32.0, 1.0));
        v.push(rect(lay, 28.0, 7.0, 32.0, 7.5));
        v.push(rect(lay, 25.5, 3.0, 26.0, 5.0));
        v.push(rect(lay, 34.0, 3.0, 34.5, 5.0));
        v.push(poly(
            lay,
            &[(25.5, 3.0), (28.0, 0.5), (28.0, 1.0), (26.0, 3.0)],
        ));
        v.push(poly(
            lay,
            &[(32.0, 0.5), (34.5, 3.0), (34.0, 3.0), (32.0, 1.0)],
        ));
        v.push(poly(
            lay,
            &[(34.5, 5.0), (32.0, 7.5), (32.0, 7.0), (34.0, 5.0)],
        ));
        v.push(poly(
            lay,
            &[(28.0, 7.5), (25.5, 5.0), (26.0, 5.0), (28.0, 7.0)],
        ));
    }
    v.extend(bare(2.0, 16.0)); // (i)
    for lay in [l.comp, l.pplus] {
        for k in 0..4 {
            let s = 1.0 + k as f64 * 1.5;
            v.push(rect(lay, s, 15.0, s + 1.5, 15.5));
            v.push(rect(lay, s, 20.5, s + 1.5, 21.0));
            v.push(rect(lay, 1.0, s + 14.0, 1.5, s + 15.5));
            v.push(rect(lay, 6.5, s + 14.0, 7.0, s + 15.5));
        }
    }
    v.extend(bare(16.0, 16.0)); // (j)
    v.extend(l.ring(15.0, 15.0, 21.0, 21.0, 0.5));
    v.extend(l.ring(14.0, 14.0, 22.0, 22.0, 0.5));
    write("DN.3.h2", v);

    // h3 - the tile lines.  A 20 × 20 well from (10, 10) in a ring whose walls stand on
    // x = 7 and x = 33, y likewise, across the 20 µm line and several 7 µm ones: clean.
    // The same at (60, 10) with a 0.5 gap in the ring's top wall at x = 70: DN.3.
    write("DN.3.h3", {
        let (mut v, a) = l.well(10.0, 10.0, 30.0, 30.0);
        v.push(l.strap(&[a]));
        v.extend(l.ring(5.0, 5.0, 35.0, 35.0, 2.0));
        let (w, b) = l.well(60.0, 10.0, 80.0, 30.0);
        v.extend(w);
        v.push(l.strap(&[b]));
        for lay in [l.comp, l.pplus] {
            v.push(rect(lay, 55.0, 5.0, 85.0, 7.0));
            v.push(rect(lay, 55.0, 33.0, 69.75, 35.0));
            v.push(rect(lay, 70.25, 33.0, 85.0, 35.0));
            v.push(rect(lay, 55.0, 7.0, 57.0, 33.0));
            v.push(rect(lay, 83.0, 7.0, 85.0, 33.0));
        }
        v
    });
}
