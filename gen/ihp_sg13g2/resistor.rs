// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OFFSET;
use crate::helpers::{
    chamfered_tr, diamond, flat_array, layer, library, poly, rect, ref_array, strip45, write_gz,
};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/resistor";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    rsil(pdk);
    rsil_body(pdk);
    rsil_c(pdk);
    rppd_body(pdk);
    rppd_c(pdk);
    rhigh(pdk);
    rhi_d(pdk);
    rhi_b(pdk);

    hardening(pdk);
}

fn lr(pdk: &PdkConfig, name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> GdsElement {
    rect(layer(pdk, name), x0, y0, x1, y1)
}

/// Rsil: a too-narrow RES box (Rsil.f) and a RES box too close to a Cont (Rsil.b).
fn rsil(pdk: &PdkConfig) {
    let o = OFFSET;
    let e: Vec<GdsElement> = vec![
        lr(pdk, "RES", o, o, o + 0.40, o + 3.0), // Rsil.f (narrow RES)
        lr(pdk, "RES", o + 5.0, o, o + 5.60, o + 3.0), // clean RES, Cont 0.10 away
        lr(pdk, "Cont", o + 5.70, o, o + 5.86, o + 0.16), // → Rsil.b
    ];
    write_gz(&format!("{DIR}/Rsil.gds.gz"), library("TOP", e));
}

/// A full (narrow) Rsil resistor: GatPoly+RES inside an EXTBlock, no SalBlock/NWell/nBuLay.
/// Body 0.40 wide → Rsil.a (and Rsil.f on the RES); EXTBlock margin 0.10 < 0.18 → Rsil.e.
fn rsil_body(pdk: &PdkConfig) {
    let o = OFFSET;
    let e = vec![
        lr(pdk, "GatPoly", o, o, o + 0.40, o + 3.0),
        lr(pdk, "RES", o, o, o + 0.40, o + 3.0),
        lr(pdk, "EXTBlock", o - 0.10, o - 0.10, o + 0.50, o + 3.10),
        // pSD 0.10 µm from the resistor GatPoly (< 0.18) → Rsil.d / pSD.m.
        lr(pdk, "pSD", o + 0.50, o, o + 1.0, o + 3.0),
    ];
    write_gz(&format!("{DIR}/RsilBody.gds.gz"), library("TOP", e));
}

/// Rsil.c — "Min. RES extension over GatPoly 0.00": the RES reaches the poly's long
/// edges.  A resistor (GatPoly 1.0 x 3.0, the RES over its middle 2.0, the heads out
/// at both ends) whose RES stops 0.05 inside the left long edge fires; one whose RES
/// runs 0.3 past it does not, nor does one flush with it.
fn rsil_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = vec![];
    for (i, dx) in [(0.0, 0.05), (10.0, -0.3), (20.0, 0.0)] {
        let x = o + i;
        e.extend(vec![
            lr(pdk, "GatPoly", x, o, x + 1.0, o + 3.0),
            lr(pdk, "RES", x + dx, o + 0.5, x + 1.0, o + 2.5),
            lr(pdk, "EXTBlock", x - 0.6, o - 0.3, x + 1.3, o + 3.3),
            lr(pdk, "Cont", x + 0.42, o + 0.17, x + 0.58, o + 0.33),
            lr(pdk, "Cont", x + 0.42, o + 2.67, x + 0.58, o + 2.83),
        ]);
    }
    write_gz(&format!("{DIR}/Rsil.c.gds.gz"), library("TOP", e));
}

/// A full (narrow) Rppd resistor: GatPoly+pSD+SalBlock inside an EXTBlock, away from
/// Activ/nSD.  Body 0.40 wide → Rppd.a / Rppd.e; pSD coincident → Rppd.b; EXTBlock
/// margin 0.10 → Rppd.d.
fn rppd_body(pdk: &PdkConfig) {
    let o = OFFSET;
    let e = vec![
        lr(pdk, "GatPoly", o, o, o + 0.40, o + 3.0),
        lr(pdk, "pSD", o, o, o + 0.40, o + 3.0),
        lr(pdk, "SalBlock", o, o, o + 0.40, o + 3.0),
        lr(pdk, "EXTBlock", o - 0.10, o - 0.10, o + 0.50, o + 3.10),
    ];
    write_gz(&format!("{DIR}/RppdBody.gds.gz"), library("TOP", e));
}

/// One Rppd resistor instance: SalBlock/pSD body `w`×3.0 at `(x, y)`, with GatPoly
/// extending 1.0 µm past each end (to host the end-cap Cont), and a Cont at `gap` from the
/// body's right edge.
fn rppd_c_instance(pdk: &PdkConfig, x: f64, y: f64, w: f64, gap: f64) -> Vec<GdsElement> {
    vec![
        lr(pdk, "SalBlock", x, y, x + w, y + 3.0),
        lr(pdk, "pSD", x, y, x + w, y + 3.0),
        lr(pdk, "GatPoly", x - 1.0, y, x + w + 1.0, y + 3.0),
        lr(pdk, "EXTBlock", x - 1.3, y - 0.3, x + w + 1.3, y + 3.3),
        lr(
            pdk,
            "Cont",
            x + w + gap,
            y + 1.0,
            x + w + gap + 0.16,
            y + 1.16,
        ),
    ]
}

/// Rppd.c — three instances: boundary-exact 0.20 gap (clean, both bounds satisfied
/// simultaneously), 0.10 gap (< 0.20 → min_space violation), 0.50 gap (> 0.20 → the
/// Cont no longer touches grown SalBlock → RppdContTooFar violation).
fn rppd_c(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rppd_c_instance(pdk, o, o, 1.0, 0.20);
    e.extend(rppd_c_instance(pdk, o + 10.0, o, 1.0, 0.10));
    e.extend(rppd_c_instance(pdk, o + 20.0, o, 1.0, 0.50));
    write_gz(&format!("{DIR}/Rppd.c.gds.gz"), library("TOP", e));
}

/// A stack forming an Rhigh body (GatPoly ∩ pSD ∩ nSD ∩ SalBlock).
fn rhigh_body(pdk: &PdkConfig, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
    ["GatPoly", "pSD", "nSD", "SalBlock"]
        .iter()
        .map(|l| rect(layer(pdk, l), x, y, x + w, y + h))
        .collect()
}

/// Rhigh: a too-narrow body (Rhi.a width, Rhi.f SalBlock width; coincident pSD_nSD → Rhi.c)
/// plus a clean wide one.
fn rhigh(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rhigh_body(pdk, o, o, 0.40, 3.0);
    e.extend(rhigh_body(pdk, o + 5.0, o, 1.0, 3.0)); // clean width, but coincident pSD_nSD
    write_gz(&format!("{DIR}/Rhigh.gds.gz"), library("TOP", e));
}

/// One Rhigh resistor instance, same end-cap/Cont pattern as `rppd_c_instance` but with
/// the Rhigh recognition body (pSD+nSD+SalBlock, not just pSD+SalBlock).
fn rhi_d_instance(pdk: &PdkConfig, x: f64, y: f64, w: f64, gap: f64) -> Vec<GdsElement> {
    vec![
        lr(pdk, "SalBlock", x, y, x + w, y + 3.0),
        lr(pdk, "pSD", x, y, x + w, y + 3.0),
        lr(pdk, "nSD", x, y, x + w, y + 3.0),
        lr(pdk, "GatPoly", x - 1.0, y, x + w + 1.0, y + 3.0),
        lr(pdk, "EXTBlock", x - 1.3, y - 0.3, x + w + 1.3, y + 3.3),
        lr(
            pdk,
            "Cont",
            x + w + gap,
            y + 1.0,
            x + w + gap + 0.16,
            y + 1.16,
        ),
    ]
}

/// Rhi.b — drawn nSD must be identical to pSD inside an Rhigh.  Four cases:
/// - a realistic Rhigh (GatPoly running THROUGH the pSD==nSD+SalBlock stack, out to its
///   contacts) with nSD extending 0.2 µm past pSD → fires HERE but not in KLayout: its
///   recognition (`stack.ext_covering(GatPoly)` = strict containment) is empty whenever
///   the poly extends beyond the stack — i.e. for every realistic resistor — so the
///   shipped check has a hole.  The PDF rule text is unambiguous ("nSD:drawing is only
///   permitted within Rhigh resistors"), so we deliberately keep the stricter behaviour.
/// - the same body with nSD == pSD exactly → clean.
/// - an isolated nSD blob far from any Rhigh → clean per the KLayout formula (only
///   mismatches touching a recognition region are flagged).
/// - GatPoly fully INSIDE the stack + the same overhang → fires in BOTH tools (validated
///   exact-location match), proving the mismatch/abutment machinery itself is faithful.
fn rhi_b(pdk: &PdkConfig) {
    let o = OFFSET;
    let body = |x: f64, nsd_over: f64| -> Vec<GdsElement> {
        vec![
            lr(pdk, "GatPoly", x - 1.0, o, x + 2.0, o + 3.0),
            lr(pdk, "pSD", x, o, x + 1.0, o + 3.0),
            lr(pdk, "nSD", x, o, x + 1.0 + nsd_over, o + 3.0),
            lr(pdk, "SalBlock", x, o, x + 1.0, o + 3.0),
            lr(pdk, "EXTBlock", x - 1.3, o - 0.3, x + 2.3, o + 3.3),
        ]
    };
    let mut e = body(o, 0.2); // mismatch → violation
    e.extend(body(o + 10.0, 0.0)); // identical → clean
    e.push(lr(pdk, "nSD", o + 20.0, o, o + 21.0, o + 3.0)); // isolated blob → clean
    // Case 4: GatPoly fully INSIDE the implant stack (the only shape KLayout's strict
    // `ext_covering` recognition accepts) with the same nSD overhang → fires in both tools.
    let x4 = o + 30.0;
    e.extend(vec![
        lr(pdk, "GatPoly", x4 + 0.25, o + 0.5, x4 + 0.75, o + 2.5),
        lr(pdk, "pSD", x4, o, x4 + 1.0, o + 3.0),
        lr(pdk, "nSD", x4, o, x4 + 1.2, o + 3.0),
        lr(pdk, "SalBlock", x4, o, x4 + 1.0, o + 3.0),
        lr(pdk, "EXTBlock", x4 - 0.3, o - 0.3, x4 + 2.3, o + 3.3),
    ]);
    write_gz(&format!("{DIR}/Rhi.b.gds.gz"), library("TOP", e));
}

/// Rhi.d — same three-instance pattern as Rppd.c (boundary-exact clean, too-close, too-far).
fn rhi_d(pdk: &PdkConfig) {
    let o = OFFSET;
    let mut e = rhi_d_instance(pdk, o, o, 1.0, 0.20);
    e.extend(rhi_d_instance(pdk, o + 10.0, o, 1.0, 0.10));
    e.extend(rhi_d_instance(pdk, o + 20.0, o, 1.0, 0.50));
    write_gz(&format!("{DIR}/Rhi.d.gds.gz"), library("TOP", e));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the resistor deck: sections 6.2 (Rsil, Rsil.a-f), 6.3 (Rppd,
// Rppd.a-e) and 6.4 (Rhigh, Rhi.a-f) of the SG13G2 layout rules.  Every layout is
// `tests/data/ihp-sg13g2/resistor/<RULE>.h<k>.gds.gz`.
//
// A resistor is drawn as the manual's figures 6.3-6.5 draw it: a GatPoly stripe, the
// block over its body (RES for Rsil, SalBlock crossing the stripe for Rppd and Rhigh),
// a head of poly beyond the block at each end with a Cont on it, EXTBlock round the
// whole poly by 0.18, and for Rppd and Rhigh pSD (and nSD) round it by 0.18.  The
// rules that the three sections share - the body width, the EXTBlock enclosure, the
// block length, the Cont gap - use one kit each, run for every kind.

/// Cont side (Cnt.a).
const CONT: f64 = 0.16;
/// Metal1 margin round a Cont, so the layouts are quiet on the Metal1 deck.
const M1_OVER: f64 = 0.05;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Rsil,
    Rppd,
    Rhigh,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Kind::Rsil => "Rsil",
            Kind::Rppd => "Rppd",
            Kind::Rhigh => "Rhi",
        }
    }
    /// The manual's Cont gap: RES to Cont 0.12 (Rsil.b), SalBlock to Cont 0.20 (Rppd.c, Rhi.d).
    fn gap(self) -> f64 {
        match self {
            Kind::Rsil => 0.12,
            _ => 0.20,
        }
    }
}

/// Every layer the deck reads.
struct P {
    gp: (i16, i16),
    polyres: (i16, i16),
    res: (i16, i16),
    sal: (i16, i16),
    psd: (i16, i16),
    nsd: (i16, i16),
    extb: (i16, i16),
    cont: (i16, i16),
    m1: (i16, i16),
    activ: (i16, i16),
}

impl P {
    fn new(pdk: &PdkConfig) -> Self {
        P {
            gp: layer(pdk, "GatPoly"),
            polyres: layer(pdk, "PolyRes"),
            res: layer(pdk, "RES"),
            sal: layer(pdk, "SalBlock"),
            psd: layer(pdk, "pSD"),
            nsd: layer(pdk, "nSD"),
            extb: layer(pdk, "EXTBlock"),
            cont: layer(pdk, "Cont"),
            m1: layer(pdk, "Metal1"),
            activ: layer(pdk, "Activ"),
        }
    }

    /// A Cont centred on `(cx, cy)` with its Metal1.
    fn cont(&self, cx: f64, cy: f64) -> Vec<GdsElement> {
        let h = CONT / 2.0;
        let m = h + M1_OVER;
        vec![
            rect(self.cont, cx - h, cy - h, cx + h, cy + h),
            rect(self.m1, cx - m, cy - m, cx + m, cy + m),
        ]
    }
}

/// Margins `[left, bottom, right, top]`.
type M4 = [f64; 4];

fn m4(v: f64) -> M4 {
    [v; 4]
}

/// `v` on the 0.005 grid.
fn grid(v: f64) -> f64 {
    (v / 0.005).round() * 0.005
}

fn grown(b: (f64, f64, f64, f64), m: M4) -> (f64, f64, f64, f64) {
    (b.0 - m[0], b.1 - m[1], b.2 + m[2], b.3 + m[3])
}

/// One resistor, horizontal, its body's lower-left corner at `(x0, y0)`: the body is
/// `len` long and `w` wide under the block (RES or SalBlock), the heads run `head`
/// beyond the block at each end, `head_w` wide (centred on the body), each with a Cont
/// `gap` from the block.  `ext` and `imp` are the EXTBlock and pSD margins round the
/// whole poly (`None`: not drawn), `nsd` the nSD margins (`None`: the pSD's), `block`
/// the block's outset over the body box.
#[derive(Clone)]
struct R {
    kind: Kind,
    x0: f64,
    y0: f64,
    w: f64,
    len: f64,
    head: f64,
    head_w: f64,
    gap: (f64, f64),
    conts: (bool, bool),
    ext: Option<M4>,
    imp: Option<M4>,
    nsd: Option<M4>,
    block: M4,
    /// The body drawn as these polygons instead of one box (the heads stay).
    body: Option<Vec<Vec<(f64, f64)>>>,
    /// The body on PolyRes, as IHP's pcells draw it, with the heads on GatPoly.
    polyres: bool,
    /// Swap x and y: a vertical resistor.
    vertical: bool,
}

impl R {
    fn new(kind: Kind, x0: f64, y0: f64, w: f64, len: f64) -> Self {
        let sides = match kind {
            Kind::Rsil => 0.0,
            _ => 0.20, // Sal.c: the block extends 0.20 past the stripe it crosses
        };
        R {
            kind,
            x0,
            y0,
            w,
            len,
            head: 0.5,
            head_w: w,
            gap: (kind.gap(), kind.gap()),
            conts: (true, true),
            ext: Some(m4(0.18)),
            imp: match kind {
                Kind::Rsil => None,
                _ => Some(m4(0.18)),
            },
            nsd: None,
            block: [0.0, sides, 0.0, sides],
            body: None,
            polyres: false,
            vertical: false,
        }
    }

    /// The body box.
    fn body_box(&self) -> (f64, f64, f64, f64) {
        (self.x0, self.y0, self.x0 + self.len, self.y0 + self.w)
    }

    /// The block (RES or SalBlock) box.
    fn block_box(&self) -> (f64, f64, f64, f64) {
        grown(self.body_box(), self.block)
    }

    /// The bounding box of the whole poly.
    fn poly_box(&self) -> (f64, f64, f64, f64) {
        let (hy0, hy1) = self.head_y();
        (
            self.x0 - self.head,
            self.y0.min(hy0),
            self.x0 + self.len + self.head,
            (self.y0 + self.w).max(hy1),
        )
    }

    /// The body's centre line, on grid (a 0.495 body's is not).
    fn cy(&self) -> f64 {
        grid(self.y0 + self.w / 2.0)
    }

    fn head_y(&self) -> (f64, f64) {
        if (self.head_w - self.w).abs() < 1e-9 {
            return (self.y0, self.y0 + self.w);
        }
        let cy = self.cy();
        (grid(cy - self.head_w / 2.0), grid(cy + self.head_w / 2.0))
    }

    /// The centre of the Cont on the left (`0`) or right (`1`) head.
    fn cont_centre(&self, side: usize) -> (f64, f64) {
        let b = self.block_box();
        let cy = self.cy();
        match side {
            0 => (b.0 - self.gap.0 - CONT / 2.0, cy),
            _ => (b.2 + self.gap.1 + CONT / 2.0, cy),
        }
    }

    fn build(&self, p: &P) -> Vec<GdsElement> {
        let mut e = vec![];
        let bb = self.body_box();
        let (hy0, hy1) = self.head_y();
        let body_layer = if self.polyres { p.polyres } else { p.gp };
        match &self.body {
            Some(polys) => {
                for pts in polys {
                    e.push(poly(body_layer, pts));
                }
            }
            None => e.push(rect(body_layer, bb.0, bb.1, bb.2, bb.3)),
        }
        if self.head > 0.0 {
            e.push(rect(p.gp, bb.0 - self.head, hy0, bb.0, hy1));
            e.push(rect(p.gp, bb.2, hy0, bb.2 + self.head, hy1));
        }
        let k = self.block_box();
        let block = match self.kind {
            Kind::Rsil => p.res,
            _ => p.sal,
        };
        e.push(rect(block, k.0, k.1, k.2, k.3));
        let pb = self.poly_box();
        if let Some(m) = self.ext {
            let b = grown(pb, m);
            e.push(rect(p.extb, b.0, b.1, b.2, b.3));
        }
        if let Some(m) = self.imp {
            let b = grown(pb, m);
            e.push(rect(p.psd, b.0, b.1, b.2, b.3));
            if self.kind == Kind::Rhigh {
                let b = grown(pb, self.nsd.unwrap_or(m));
                e.push(rect(p.nsd, b.0, b.1, b.2, b.3));
            }
        }
        if self.conts.0 {
            let (cx, cy) = self.cont_centre(0);
            e.extend(p.cont(cx, cy));
        }
        if self.conts.1 {
            let (cx, cy) = self.cont_centre(1);
            e.extend(p.cont(cx, cy));
        }
        if self.vertical { transpose(&e) } else { e }
    }
}

/// Swap x and y of every boundary: the same layout turned by 90°.
fn transpose(elems: &[GdsElement]) -> Vec<GdsElement> {
    elems
        .iter()
        .map(|e| match e {
            GdsElement::GdsBoundary(b) => {
                let mut b = b.clone();
                for p in &mut b.xy {
                    std::mem::swap(&mut p.x, &mut p.y);
                }
                GdsElement::GdsBoundary(b)
            }
            other => other.clone(),
        })
        .collect()
}

fn write(name: &str, e: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", e));
}

fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");
    let p = P::new(pdk);
    for kind in [Kind::Rsil, Kind::Rppd, Kind::Rhigh] {
        width_kit(&p, kind);
        ext_kit(&p, kind);
        length_kit(&p, kind);
        if kind != Kind::Rsil {
            imp_kit(&p, kind);
            cont_kit(&p, kind);
        }
    }
    rsil_c_h(&p);
    rsil_d(&p);
    rhi_b_h(&p);
}

// ---------------------------------------------------------------------------
// Rsil.a, Rppd.a, Rhi.a: "Min. GatPoly width 0.50" - the body under the block.

fn width_kit(p: &P, kind: Kind) {
    let rule = format!("{}.a", kind.tag());
    let at = |x, y, w| R::new(kind, x, y, w, 2.0);

    // h1: the bound (0.50 clean, 0.495 fires), a 0.55 body with a 0.06 slot into its
    // top (0.49 under the slot), a body of two abutting boxes 0.25 + 0.25 (clean) and
    // 0.25 + 0.245 (fires), the body on PolyRes with GatPoly heads at 0.495 (IHP's
    // pcell), a 0.495 head neck outside the block (clean: not the body).
    let mut e = at(2.0, 2.0, 0.50).build(p);
    e.extend(at(2.0, 5.0, 0.495).build(p));
    let mut slot = at(2.0, 8.0, 0.55);
    let (x0, y0) = (slot.x0, slot.y0);
    slot.body = Some(vec![vec![
        (x0, y0),
        (x0 + 2.0, y0),
        (x0 + 2.0, y0 + 0.55),
        (x0 + 1.15, y0 + 0.55),
        (x0 + 1.15, y0 + 0.49),
        (x0 + 0.85, y0 + 0.49),
        (x0 + 0.85, y0 + 0.55),
        (x0, y0 + 0.55),
    ]]);
    e.extend(slot.build(p));
    let mut two = at(2.0, 11.0, 0.50);
    two.body = Some(vec![
        vec![(2.0, 11.0), (4.0, 11.0), (4.0, 11.25), (2.0, 11.25)],
        vec![(2.0, 11.25), (4.0, 11.25), (4.0, 11.5), (2.0, 11.5)],
    ]);
    e.extend(two.build(p));
    let mut two = at(2.0, 14.0, 0.495);
    two.body = Some(vec![
        vec![(2.0, 14.0), (4.0, 14.0), (4.0, 14.25), (2.0, 14.25)],
        vec![(2.0, 14.25), (4.0, 14.25), (4.0, 14.495), (2.0, 14.495)],
    ]);
    e.extend(two.build(p));
    let mut pr = at(10.0, 2.0, 0.495);
    pr.polyres = true;
    e.extend(pr.build(p));
    let mut neck = at(10.0, 5.0, 0.50);
    neck.head_w = 0.495;
    e.extend(neck.build(p));
    write(&format!("{rule}.h1"), e);

    // h2: 45° bodies - a strip 0.495 wide across its walls (fires) and 0.502 (clean),
    // a diamond of 0.495 (fires four times) and 0.502; the block is the same shape, the
    // EXTBlock and implants a box well round it.
    let mut e = vec![];
    for (i, d) in [0.35, 0.355].iter().enumerate() {
        let x = 2.0 + 6.0 * i as f64;
        e.extend(shape45(
            p,
            kind,
            strip45(p.gp, x, 2.0, 3.0, *d),
            (x - 1.0, 1.0, x + 4.0, 6.0),
        ));
        e.extend(shape45(
            p,
            kind,
            diamond(p.gp, x + 1.5, 9.0, *d),
            (x - 1.0, 7.0, x + 4.0, 11.0),
        ));
    }
    write(&format!("{rule}.h2"), e);

    // h3: 0.495 bodies across x = 20, ending on x = 40, starting on x = 42, a vertical
    // one across y = 20, 300 µm long, and at (1000, 1000).
    let mut e = at(17.0, 2.0, 0.495).build(p);
    e.extend(R::new(kind, 36.0, 2.0, 0.495, 4.0).build(p));
    e.extend(at(42.0, 2.0, 0.495).build(p));
    let mut v = R::new(kind, 17.0, 30.0, 0.495, 6.0);
    v.vertical = true;
    e.extend(v.build(p));
    e.extend(R::new(kind, 5.0, 10.0, 0.495, 300.0).build(p));
    e.extend(at(1000.0, 1000.0, 0.495).build(p));
    write(&format!("{rule}.h3"), e);
}

/// A 45° body `shape` (poly) with the block the same shape and the rest a box `bx`.
fn shape45(p: &P, kind: Kind, shape: GdsElement, bx: (f64, f64, f64, f64)) -> Vec<GdsElement> {
    let mut block = shape.clone();
    let l = match kind {
        Kind::Rsil => p.res,
        _ => p.sal,
    };
    if let GdsElement::GdsBoundary(b) = &mut block {
        b.layer = l.0;
        b.datatype = l.1;
    }
    let mut e = vec![shape, block, rect(p.extb, bx.0, bx.1, bx.2, bx.3)];
    if kind != Kind::Rsil {
        e.push(rect(p.psd, bx.0, bx.1, bx.2, bx.3));
    }
    if kind == Kind::Rhigh {
        e.push(rect(p.nsd, bx.0, bx.1, bx.2, bx.3));
    }
    e
}

// ---------------------------------------------------------------------------
// Rsil.e, Rppd.d, Rhi.e: "Min. EXTBlock enclosure of GatPoly 0.18".

fn ext_kit(p: &P, kind: Kind) {
    let rule = format!(
        "{}.{}",
        kind.tag(),
        if kind == Kind::Rppd { "d" } else { "e" }
    );
    enclosure_kit(p, kind, &rule, Layer::Ext);
}

// Rppd.b: "Min. pSD enclosure of GatPoly 0.18"; Rhi.c: "Min. pSD and nSD enclosure of
// GatPoly 0.18".

fn imp_kit(p: &P, kind: Kind) {
    let rule = format!(
        "{}.{}",
        kind.tag(),
        if kind == Kind::Rppd { "b" } else { "c" }
    );
    enclosure_kit(p, kind, &rule, Layer::Imp);
}

#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Ext,
    Imp,
}

/// The enclosure kit: the enclosing layer's margins round the resistor's poly.
fn enclosure_kit(p: &P, kind: Kind, rule: &str, which: Layer) {
    let with = |x, y, m: Option<M4>| {
        let mut r = R::new(kind, x, y, 0.5, 2.0);
        match which {
            Layer::Ext => r.ext = m,
            Layer::Imp => r.imp = m,
        }
        r
    };
    let top = |v| Some([0.18, 0.18, 0.18, v]);

    // h1: 0.18 all round (clean), 0.175 on top, 0.175 all round, a chamfer passing
    // 0.177 from the poly's corner (fires) and 0.184 (clean), the right head running out
    // of the layer by 0.1, the layer over the body only (both heads out), the layer
    // absent (EXTBlock only: a resistor without one is still one), 0.175 at the right
    // head's end, the layer as two overlapping boxes whose union encloses by 0.18 (clean).
    let mut e = with(2.0, 2.0, Some(m4(0.18))).build(p);
    e.extend(with(2.0, 5.0, top(0.175)).build(p));
    e.extend(with(2.0, 8.0, Some(m4(0.175))).build(p));
    let layers: Vec<(i16, i16)> = match (which, kind) {
        (Layer::Ext, _) => vec![p.extb],
        (Layer::Imp, Kind::Rhigh) => vec![p.psd, p.nsd],
        (Layer::Imp, _) => vec![p.psd],
    };
    for (i, k) in [0.25, 0.26].iter().enumerate() {
        let r = with(2.0, 11.0 + 3.0 * i as f64, None);
        let pb = r.poly_box();
        let b = grown(pb, m4(0.18));
        e.extend(r.build(p));
        for l in &layers {
            e.push(chamfered_tr(*l, b.0, b.1, b.2, b.3, pb.2 + pb.3 + k));
        }
    }
    e.extend(with(10.0, 2.0, Some([0.18, 0.18, -0.1, 0.18])).build(p));
    e.extend(with(10.0, 5.0, Some([-0.5, 0.18, -0.5, 0.18])).build(p));
    if which == Layer::Ext {
        e.extend(with(10.0, 8.0, None).build(p));
    }
    e.extend(with(10.0, 14.0, Some([0.18, 0.18, 0.175, 0.18])).build(p));
    let r = with(10.0, 11.0, None);
    let b = grown(r.poly_box(), m4(0.18));
    let mid = (b.0 + b.2) / 2.0;
    e.extend(r.build(p));
    for l in &layers {
        e.push(rect(*l, b.0, b.1, mid + 0.3, b.3));
        e.push(rect(*l, mid - 0.3, b.1, b.2, b.3));
    }
    write(&format!("{rule}.h1"), e);
    tile_and_arrays(p, kind, rule, which);
}

fn tile_and_arrays(p: &P, kind: Kind, rule: &str, which: Layer) {
    let with = |x, y, len, m: M4| {
        let mut r = R::new(kind, x, y, 0.5, len);
        match which {
            Layer::Ext => r.ext = Some(m),
            Layer::Imp => r.imp = Some(m),
        }
        r
    };
    // h2: 0.175 on the right with the poly ending on x = 20, 0.18 across x = 40
    // (clean), 0.175 on top across x = 42, 0.175 on top of a 300 µm body, and at
    // (1000, 1000).
    let mut e = with(17.5, 2.0, 2.0, [0.18, 0.18, 0.175, 0.18]).build(p);
    e.extend(with(39.0, 5.0, 2.0, m4(0.18)).build(p));
    e.extend(with(41.0, 2.0, 2.0, [0.18, 0.18, 0.18, 0.175]).build(p));
    e.extend(with(5.0, 10.0, 300.0, [0.18, 0.18, 0.18, 0.175]).build(p));
    e.extend(with(1000.0, 1000.0, 2.0, [0.18, 0.18, 0.18, 0.175]).build(p));
    write(&format!("{rule}.h2"), e);
}

// ---------------------------------------------------------------------------
// Rsil.f: "Min. RES length 0.50"; Rppd.e, Rhi.f: "Min. SalBlock length 0.50".

fn length_kit(p: &P, kind: Kind) {
    let rule = format!(
        "{}.{}",
        kind.tag(),
        if kind == Kind::Rppd { "e" } else { "f" }
    );
    let at = |x, y, w, len| R::new(kind, x, y, w, len);

    // h1: length 0.50 (clean), 0.495 (fires), 0.495 on a 0.6 body, and a 0.495 wide
    // body 3.0 long (the width rule's, not the length's).
    let mut e = at(2.0, 2.0, 0.5, 0.5).build(p);
    e.extend(at(2.0, 5.0, 0.5, 0.495).build(p));
    e.extend(at(2.0, 8.0, 0.6, 0.495).build(p));
    e.extend(at(2.0, 11.0, 0.495, 3.0).build(p));
    write(&format!("{rule}.h1"), e);

    // h2: 0.495 blocks ending on x = 20, across x = 40, starting on x = 42, at (1000, 1000).
    let mut e = at(19.505, 2.0, 0.5, 0.495).build(p);
    e.extend(at(39.75, 2.0, 0.5, 0.495).build(p));
    e.extend(at(42.0, 2.0, 0.5, 0.495).build(p));
    e.extend(at(1000.0, 1000.0, 0.5, 0.495).build(p));
    write(&format!("{rule}.h2"), e);
}

// ---------------------------------------------------------------------------
// Rppd.c, Rhi.d: "Min. and max. SalBlock space to Cont 0.20".

fn cont_kit(p: &P, kind: Kind) {
    let rule = format!(
        "{}.{}",
        kind.tag(),
        if kind == Kind::Rppd { "c" } else { "d" }
    );
    let gaps = |x, y, l, r| {
        let mut q = R::new(kind, x, y, 0.5, 2.0);
        q.gap = (l, r);
        q
    };

    // h1: 0.20 both (clean); 0.195 left (min); 0.205 left (max); a second Cont row at
    // 0.54 on a 1.0 head (max); a 1.5 wide head with a Cont on the body's line and one
    // 0.5 beside it, both 0.20 from the drawn block (clean); a Cont abutting the
    // block (min); a Cont at 1.0 on a 1.5 head (max); 0.195 both.
    let mut e = gaps(2.0, 2.0, 0.2, 0.2).build(p);
    e.extend(gaps(2.0, 5.0, 0.195, 0.2).build(p));
    e.extend(gaps(2.0, 8.0, 0.205, 0.2).build(p));
    let mut row = gaps(2.0, 11.0, 0.2, 0.2);
    row.head = 1.0;
    let (cx, cy) = row.cont_centre(0);
    e.extend(row.build(p));
    e.extend(p.cont(cx - CONT - 0.18, cy));
    let mut wide = gaps(2.0, 14.0, 0.2, 0.2);
    wide.head_w = 1.5;
    let (cx, cy) = wide.cont_centre(0);
    e.extend(wide.build(p));
    e.extend(p.cont(cx, cy + 0.5));
    e.extend(gaps(10.0, 2.0, 0.0, 0.2).build(p));
    let mut far = gaps(10.0, 5.0, 1.0, 0.2);
    far.head = 1.5;
    e.extend(far.build(p));
    e.extend(gaps(10.0, 8.0, 0.195, 0.195).build(p));
    write(&format!("{rule}.h1"), e);

    // h2: a 0.195 gap across x = 20 (Cont ending at 19.9, block starting at 20.095), a
    // Cont ending on x = 40 with the block at 40.2 (clean), the block starting on x = 42
    // with 0.195, a Cont ending on x = 60 with the block at 60.195, a 300 µm body with
    // 0.195 on the right, and at (1000, 1000).
    let mut e = gaps(20.095, 2.0, 0.195, 0.2).build(p);
    e.extend(gaps(40.2, 5.0, 0.2, 0.2).build(p));
    e.extend(gaps(42.0, 2.0, 0.195, 0.2).build(p));
    e.extend(gaps(60.195, 2.0, 0.195, 0.2).build(p));
    let mut long = gaps(5.0, 10.0, 0.2, 0.195);
    long.len = 300.0;
    e.extend(long.build(p));
    e.extend(gaps(1000.0, 1000.0, 0.195, 0.2).build(p));
    write(&format!("{rule}.h2"), e);
}

// ---------------------------------------------------------------------------
// Rsil.c: "Min. RES extension over GatPoly 0.00" - the RES reaches the poly's long
// edges, as figure 6.3 draws them coincident.

fn rsil_c_h(p: &P) {
    let res = |x, y, m: M4| {
        let mut q = R::new(Kind::Rsil, x, y, 0.6, 2.0);
        q.block = m;
        q
    };

    // h1: RES coincident with a 0.6 body (clean); RES ending 0.05 inside the body on top
    // (extension -0.05: fires); RES 0.05 past the top (0.05: clean); RES 1.5 past the top
    // (clean); RES 0.05 inside on both sides (fires twice).
    let mut e = res(2.0, 2.0, m4(0.0)).build(p);
    e.extend(res(2.0, 5.0, [0.0, 0.0, 0.0, -0.05]).build(p));
    e.extend(res(2.0, 8.0, [0.0, 0.0, 0.0, 0.05]).build(p));
    e.extend(res(2.0, 11.0, [0.0, 0.0, 0.0, 1.5]).build(p));
    e.extend(res(2.0, 14.0, [0.0, -0.05, 0.0, -0.05]).build(p));
    write("Rsil.c.h1", e);

    // h2: the inset across x = 20, the outset across x = 40 (clean), the inset at (1000, 1000).
    let mut e = res(19.0, 2.0, [0.0, 0.0, 0.0, -0.05]).build(p);
    e.extend(res(39.0, 2.0, [0.0, 0.0, 0.0, 0.05]).build(p));
    e.extend(res(1000.0, 1000.0, [0.0, 0.0, 0.0, -0.05]).build(p));
    write("Rsil.c.h2", e);
}

// ---------------------------------------------------------------------------
// Rsil.d: "Min. pSD space to GatPoly 0.18" - any pSD to the resistor's poly.

fn rsil_d(p: &P) {
    let r = |x, y| R::new(Kind::Rsil, x, y, 0.5, 2.0);
    let psd = |x0, y0| rect(p.psd, x0, y0, x0 + 1.0, y0 + 1.0);

    // h1: pSD 0.18 below the body (clean); 0.175; 0.12/0.12 corner to corner from the
    // head's corner (0.170); 0.13/0.13 (0.184, clean); pSD abutting the head's end; pSD
    // 0.175 from a plain poly with no RES (clean); an Rppd's pSD 0.175 from the head; pSD
    // 0.175 below a resistor with no EXTBlock.
    let mut e = r(2.0, 2.0).build(p);
    e.push(psd(2.5, 2.0 - 0.18 - 1.0));
    e.extend(r(2.0, 5.0).build(p));
    e.push(psd(2.5, 5.0 - 0.175 - 1.0));
    for (i, d) in [0.12, 0.13].iter().enumerate() {
        let q = r(2.0, 8.0 + 3.0 * i as f64);
        let pb = q.poly_box();
        e.extend(q.build(p));
        e.push(psd(pb.2 + d, pb.3 + d));
    }
    let q = r(2.0, 14.0);
    let pb = q.poly_box();
    e.extend(q.build(p));
    e.push(psd(pb.2, pb.1));
    e.push(rect(p.gp, 10.0, 2.0, 12.0, 2.5));
    e.push(psd(10.5, 2.0 - 0.175 - 1.0));
    let q = r(10.0, 5.0);
    let pb = q.poly_box();
    e.extend(q.build(p));
    let mut n = R::new(Kind::Rppd, pb.2 + 0.175 + 0.18 + 0.5, 5.0, 0.5, 2.0);
    n.ext = None;
    e.extend(n.build(p));
    let mut q = r(10.0, 8.0);
    q.ext = None;
    e.extend(q.build(p));
    e.push(psd(10.5, 8.0 - 0.175 - 1.0));
    write("Rsil.d.h1", e);

    // h2: 0.175 across x = 20 (head ending at 19.9, pSD from 20.075), pSD ending on
    // x = 40 with the head at 40.18 (clean), the head ending on x = 42 with pSD at
    // 42.175, the head ending on x = 60 with pSD at 60.175, the head starting on x = 80
    // with pSD ending at 79.825, a 300 µm body with pSD 0.175 below its far end, at
    // (1000, 1000) with the body's bottom edge on y = 1000.
    let mut e = r(17.4, 2.0).build(p);
    e.push(psd(20.075, 2.0));
    e.extend(r(40.68, 2.0).build(p));
    e.push(psd(39.0, 2.0));
    e.extend(r(39.5, 5.0).build(p));
    e.push(psd(42.175, 5.0));
    e.extend(r(57.5, 2.0).build(p));
    e.push(psd(60.175, 2.0));
    e.extend(r(80.5, 2.0).build(p));
    e.push(psd(78.825, 2.0));
    let mut long = r(5.0, 10.0);
    long.len = 300.0;
    e.extend(long.build(p));
    e.push(psd(300.0, 10.0 - 0.175 - 1.0));
    e.extend(r(1000.0, 1000.0).build(p));
    e.push(psd(1000.5, 1000.0 - 0.175 - 1.0));
    write("Rsil.d.h2", e);

    // h3/h4: fifty with pSD 0.175 below.
    let cell = {
        let mut c = r(1.0, 1.5).build(p);
        c.push(psd(1.5, 1.5 - 0.175 - 1.0));
        c
    };
    write("Rsil.d.h3", flat_array(&cell, 10, 5, 4.0));
    write_gz(
        &format!("{DIR}/Rsil.d.h4.gds.gz"),
        ref_array(cell, 10, 5, 4.0),
    );
}

// ---------------------------------------------------------------------------
// Rhi.b: "pSD and nSD are identical", note 1: "nSD:drawing is only permitted within
// Rhigh resistors".

fn rhi_b_h(p: &P) {
    let nsd = |x, y, imp: f64, m: Option<M4>| {
        let mut q = R::new(Kind::Rhigh, x, y, 0.5, 2.0);
        q.imp = Some(m4(imp));
        q.nsd = m;
        q
    };

    // h1: identical (clean); nSD 0.005 past pSD on top; nSD 0.005 short of pSD on top
    // (pSD at 0.30, so the enclosure holds); nSD 0.1 past pSD all round; nSD ending in
    // the body's middle (the right half has pSD only; Rhi.c too); a bare nSD:drawing on
    // an Activ far from any resistor (note 1).
    let mut e = nsd(2.0, 2.0, 0.18, None).build(p);
    e.extend(nsd(2.0, 5.0, 0.18, Some([0.18, 0.18, 0.18, 0.185])).build(p));
    e.extend(nsd(2.0, 8.0, 0.30, Some([0.30, 0.30, 0.30, 0.295])).build(p));
    e.extend(nsd(2.0, 11.0, 0.18, Some(m4(0.28))).build(p));
    e.extend(nsd(2.0, 14.0, 0.18, Some([0.18, 0.18, -1.5, 0.18])).build(p));
    e.push(rect(p.activ, 10.0, 2.0, 11.0, 3.0));
    e.push(rect(p.nsd, 9.8, 1.8, 11.2, 3.2));
    write("Rhi.b.h1", e);

    // h2: nSD 0.005 past a pSD ending on x = 20, identical across x = 40 (clean), nSD
    // 0.005 short of a pSD ending on x = 42, and the 0.005 overhang at (1000, 1000).
    let mut e = nsd(17.32, 2.0, 0.18, Some([0.18, 0.18, 0.185, 0.18])).build(p);
    e.extend(nsd(39.0, 2.0, 0.18, None).build(p));
    e.extend(nsd(39.32, 5.0, 0.18, Some([0.18, 0.18, 0.175, 0.18])).build(p));
    e.extend(nsd(1000.0, 1000.0, 0.18, Some([0.18, 0.18, 0.18, 0.185])).build(p));
    write("Rhi.b.h2", e);
}
