// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Antenna ratios: a good and a bad pattern for every rule in the `antenna` deck.
//!
//! Every rule here is the same check at a different height - the conductor hanging off a
//! sensitive node during processing, over that node's own area - so every fixture is the
//! same stack with one storey enlarged.  A gate is contacted up through metal1, via1,
//! metal2 and so on, each storey a bare landing pad, until the level under test, which
//! becomes either a long bar (a metal rule, scored on the perimeter times the metal's
//! thickness) or a field of cuts (a via or contact rule, scored on plan area).  Because
//! each rule names one conductor layer and no other, the storeys below stay far under
//! their own limits and the fixture isolates.
//!
//! Three kinds of node share the stack.  A gate Dualgate does not touch is the nominal
//! one and a gate it covers is the thick one, so the marker alone moves a fixture from
//! the `_i_` rules to the `_ii_` ones.  The `_iii_` rules score against a MIM-B top plate
//! instead, and those fixtures carry no gate at all - a plate and a gate on one net would
//! answer for the same metal twice.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/antenna";

/// The stack's axis, and the pitch and size of a field of cuts.
const PITCH: f64 = 0.6;
const ROW: usize = 10;
/// How far a metal or plate must reach past a cut.
const ENC: f64 = 0.2;

/// What is enlarged at the level under test.
enum Big {
    /// A long poly run off the gate.
    Poly(f64),
    /// `n` cuts at via level `i` - 0 is contact, 1 is via1, up to via4.
    Cut(usize, usize),
    /// A long bar at metal level `k` - 0 is metal1, up to metal5.
    Metal(usize, f64),
}

/// What the conductor is measured against.
enum Node {
    /// A transistor gate, thick if Dualgate covers it.
    Gate(bool),
    /// A MIM-B top plate, which joins the stack at via3.
    Fuse,
}

struct Ctx {
    comp: (i16, i16),
    poly2: (i16, i16),
    dualgate: (i16, i16),
    fusetop: (i16, i16),
    cuts: [(i16, i16); 5],
    metals: [(i16, i16); 5],
}

/// A field of `n` cuts in rows of ten, and the box that holds them.
fn field(l: (i16, i16), n: usize, x: f64, y: f64, size: f64) -> (Vec<GdsElement>, [f64; 4]) {
    let mut v = Vec::new();
    let (mut x1, mut y1) = (x + size, y + size);
    for i in 0..n {
        let (cx, cy) = (x + (i % ROW) as f64 * PITCH, y + (i / ROW) as f64 * PITCH);
        v.push(rect(l, cx, cy, cx + size, cy + size));
        x1 = x1.max(cx + size);
        y1 = y1.max(cy + size);
    }
    (v, [x, y, x1, y1])
}

/// The box `b` grown by the enclosure a metal owes a cut.
fn over(l: (i16, i16), b: [f64; 4]) -> GdsElement {
    rect(l, b[0] - ENC, b[1] - ENC, b[2] + ENC, b[3] + ENC)
}

fn build(c: &Ctx, node: &Node, big: &Big) -> Vec<GdsElement> {
    let o = OFFSET;
    // Where the stack rises.  A gate contacts at the bottom; a plate joins at via3.
    let (cx, cy) = (o + 1.0, o + 2.3);
    let mut v = Vec::new();
    let (bottom, poly_len) = match (node, big) {
        (Node::Gate(_), Big::Poly(len)) => (0, *len),
        (Node::Gate(_), _) => (0, 1.0),
        (Node::Fuse, _) => (3, 0.0),
    };
    let top = match big {
        Big::Poly(_) => 0,
        Big::Cut(i, _) => *i,
        Big::Metal(k, _) => *k,
    };

    // The node itself.
    match node {
        Node::Gate(thick) => {
            // A 0.22 by 0.3 µm channel, and the poly that runs off it to the contact.
            v.push(rect(c.comp, o, o, o + 2.0, o + 0.3));
            v.push(rect(c.poly2, o + 0.89, o - 0.4, o + 1.11, o + 2.0));
            if *thick {
                v.push(rect(c.dualgate, o - 0.5, o - 0.5, o + 2.5, o + 1.0));
            }
        }
        // Small on purpose: the plate is the denominator, so a wide one would need a
        // conductor too long to be worth drawing.
        Node::Fuse => v.push(rect(c.fusetop, cx - 0.15, cy - 0.15, cx + 0.15, cy + 0.15)),
    }

    // Each storey: the cuts under a metal, then the metal over them.
    let mut boxes: Vec<[f64; 4]> = Vec::new();
    for k in bottom..=top {
        let size = if k == 0 { 0.22 } else { 0.26 };
        let n = match big {
            Big::Cut(i, n) if *i == k => *n,
            _ => 1,
        };
        // A field on the plate would enlarge the plate; the plate keeps one cut and the
        // field goes beside it, under the same metal, over a pad of the metal below.
        let (x, y) = if matches!(node, Node::Fuse) && k == bottom && n > 1 {
            (cx + 1.0, cy - size * 0.5)
        } else {
            (cx - size * 0.5, cy - size * 0.5)
        };
        let (cut, mut b) = field(c.cuts[k], n, x, y, size);
        v.extend(cut);
        if matches!(node, Node::Fuse) && k == bottom {
            let plate = [b[0].min(cx - 0.15), b[1].min(cy - 0.15), b[2], b[3]];
            if n > 1 {
                // The plate needs a cut of its own, and the metal below has to run from
                // under that cut to under the field: the rule resolves the net one step
                // short of the metal above, so the two only meet underneath.
                let h = size * 0.5;
                v.push(rect(c.cuts[k], cx - h, cy - h, cx + h, cy + h));
                v.push(over(c.metals[k - 1], plate));
            }
            b = plate;
        }
        boxes.push(b);
    }
    for (idx, k) in (bottom..=top).enumerate() {
        let mut b = boxes[idx];
        if let Some(next) = boxes.get(idx + 1) {
            b = [
                b[0].min(next[0]),
                b[1].min(next[1]),
                b[2].max(next[2]),
                b[3].max(next[3]),
            ];
        }
        if k == bottom && matches!(node, Node::Gate(_)) {
            // The poly reaches from the channel over every contact, plus whatever
            // extra run the rule under test asks for.
            v.push(rect(
                c.poly2,
                cx - 0.3,
                o + 2.0,
                (b[2] + 0.2).max(cx + 0.3 + poly_len),
                (b[3] + 0.2).max(cy + 0.3),
            ));
        }
        v.push(over(c.metals[k], b));
        if let Big::Metal(mk, len) = big
            && *mk == k
        {
            v.push(rect(c.metals[k], cx, cy - 0.25, cx + len, cy + 0.25));
        }
    }
    v
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        comp: layer(pdk, "comp"),
        poly2: layer(pdk, "poly2_drawn"),
        dualgate: layer(pdk, "dualgate"),
        fusetop: layer(pdk, "fusetop"),
        cuts: [
            layer(pdk, "contact"),
            layer(pdk, "via1"),
            layer(pdk, "via2"),
            layer(pdk, "via3"),
            layer(pdk, "via4"),
        ],
        metals: [
            layer(pdk, "metal1_drawn"),
            layer(pdk, "metal2_drawn"),
            layer(pdk, "metal3_drawn"),
            layer(pdk, "metal4_drawn"),
            layer(pdk, "metal5_drawn"),
        ],
    };
    let write = |id: &str, polarity: &str, node: Node, big: Big| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", build(&c, &node, &big)),
        );
    };

    // ANT.1 and ANT.8 measure any gate, thick or not, so they use the nominal one.
    write("ANT.1", "good", Node::Gate(false), Big::Poly(8.0));
    write("ANT.1", "bad", Node::Gate(false), Big::Poly(45.0));
    write("ANT.8", "good", Node::Gate(false), Big::Cut(0, 1));
    write("ANT.8", "bad", Node::Gate(false), Big::Cut(0, 20));

    // The metal and via rules, once against a nominal gate and once against a thick one.
    let metals = [
        ("ANT.2", 0),
        ("ANT.3", 1),
        ("ANT.4", 2),
        ("ANT.5", 3),
        ("ANT.6", 4),
    ];
    let vias = [("ANT.9", 1), ("ANT.10", 2), ("ANT.11", 3), ("ANT.12", 4)];
    for (flavour, thick) in [("i", false), ("ii", true)] {
        for (name, k) in metals {
            let id = format!("ANT.16_{flavour}_{name}");
            write(&id, "good", Node::Gate(thick), Big::Metal(k, 8.0));
            write(&id, "bad", Node::Gate(thick), Big::Metal(k, 30.0));
        }
        for (name, i) in vias {
            let id = format!("ANT.16_{flavour}_{name}");
            write(&id, "good", Node::Gate(thick), Big::Cut(i, 1));
            write(&id, "bad", Node::Gate(thick), Big::Cut(i, 30));
        }
    }

    // The MIM-B rules, scored against a top plate rather than a gate.
    for (id, big_good, big_bad) in [
        ("ANT.16_iii_ANT.15_V3_MIMB", Big::Cut(3, 1), Big::Cut(3, 30)),
        (
            "ANT.16_iii_ANT.14_M4_MIMB",
            Big::Metal(3, 8.0),
            Big::Metal(3, 40.0),
        ),
        ("ANT.16_iii_ANT.15_V4_MIMB", Big::Cut(4, 1), Big::Cut(4, 30)),
        (
            "ANT.16_iii_ANT.14_M5_MIMB",
            Big::Metal(4, 8.0),
            Big::Metal(4, 40.0),
        ),
    ] {
        write(id, "good", Node::Fuse, big_good);
        write(id, "bad", Node::Fuse, big_bad);
    }

    hardening(pdk);
}

// --- Hardening (hardening/SPEC.md) ------------------------------------------------
//
// The deck's own conditions, not the check's: which level each rule is read at, what
// counts as the gate and what as the conductor, which diffusions relieve the ratio and
// by how much, and the cut between one level and the next.  Every fixture is scored by
// hand from section 8.0 and the numbers are in the comments.
//
// The gate throughout is 0.21 µm of Poly2 across 0.315 µm of COMP: 0.06615 µm² of oxide,
// chosen so the metal bounds land on the 0.005 µm grid.  A metal antenna is one 0.66 µm
// wide rectangle `len` long, so its perimeter is 2(len + 0.66) and the manual's
// "perimeter area" is that times the layer's thickness (0.2 µm Poly2, 0.54 µm Metal1 to
// Metal4, 1.19 µm Metal5, which is this stack's MetalTop).

/// A GDS layer.
type L = (i16, i16);

/// A contact, and a via.
const CONT: f64 = 0.22;
const VIA: f64 = 0.26;
/// Half the width of a riser pad or an antenna bar.
const HW: f64 = 0.33;

struct Hw {
    comp: L,
    nplus: L,
    pplus: L,
    nwell: L,
    poly2: L,
    dualgate: L,
    res_mk: L,
    fusetop: L,
    contact: L,
    vias: [L; 4],
    metals: [L; 5],
}

impl Hw {
    /// A transistor: 2 µm of COMP, a 0.21 µm Poly2 stripe across it, and a Poly2 head
    /// `head` µm wide above the COMP to land contacts on.  Returns the head's centre.
    /// The whole Poly2 shape has perimeter 2·head + 3.6.
    fn transistor(&self, v: &mut Vec<GdsElement>, x: f64, y: f64, head: f64) -> (f64, f64) {
        v.push(rect(self.comp, x, y, x + 2.0, y + 0.315));
        v.push(rect(self.poly2, x + 0.9, y - 0.4, x + 1.11, y + 0.6));
        v.push(rect(self.poly2, x + 0.7, y + 0.6, x + 0.7 + head, y + 1.4));
        (x + 1.0, y + 1.0)
    }

    /// One cut of `size` centred on a point.
    fn cut(&self, l: L, cx: f64, cy: f64, size: f64) -> GdsElement {
        rect(
            l,
            cx - size / 2.0,
            cy - size / 2.0,
            cx + size / 2.0,
            cy + size / 2.0,
        )
    }

    /// `n` cuts in rows of `cols` at `pitch`, the first centred at `at`.
    fn cuts(
        &self,
        v: &mut Vec<GdsElement>,
        l: L,
        at: (f64, f64),
        grid: (usize, usize),
        pitch: f64,
        size: f64,
    ) {
        let (n, cols) = grid;
        for i in 0..n {
            let x = at.0 + (i % cols) as f64 * pitch;
            let y = at.1 + (i / cols) as f64 * pitch;
            v.push(self.cut(l, x, y, size));
        }
    }

    /// A conductor bar 0.66 µm wide and `len` µm long, its left end at `cx`.
    fn bar(&self, l: L, cx: f64, cy: f64, len: f64) -> GdsElement {
        rect(l, cx - HW, cy - HW, cx - HW + len, cy + HW)
    }

    /// The stack from Metal1 up to `top` (0 is Metal1), a bare landing pad at every
    /// level except where `big` names one, which becomes the antenna.
    fn riser(
        &self,
        v: &mut Vec<GdsElement>,
        cx: f64,
        cy: f64,
        top: usize,
        big: Option<(usize, f64)>,
    ) {
        for k in 0..=top {
            let len = match big {
                Some((bk, l)) if bk == k => l,
                _ => 2.0 * HW,
            };
            v.push(self.bar(self.metals[k], cx, cy, len));
            if k > 0 {
                v.push(self.cut(self.vias[k - 1], cx, cy, VIA));
            }
        }
    }

    /// An N+ diode: 0.36 µm square of N+ COMP with a contact in it, 0.1296 µm² of the
    /// COMP the manual's ANT.16 counts.  The metal above it is the caller's antenna.
    fn ndiode(&self, v: &mut Vec<GdsElement>, cx: f64, cy: f64) {
        v.push(self.cut(self.comp, cx, cy, 0.36));
        v.push(self.cut(self.nplus, cx, cy, 0.5));
        v.push(self.cut(self.contact, cx, cy, CONT));
    }

    /// A P+ diode in its own 1 µm N-well, the well left untied: the diffusion is on the
    /// node, the well is not.
    fn pdiode(&self, v: &mut Vec<GdsElement>, cx: f64, cy: f64) {
        v.push(self.cut(self.nwell, cx, cy, 1.0));
        v.push(self.cut(self.comp, cx, cy, 0.36));
        v.push(self.cut(self.pplus, cx, cy, 0.5));
        v.push(self.cut(self.contact, cx, cy, CONT));
    }

    /// A 1 µm N-well tied to the node by an N+ tap: 1 µm² of well on the node.  The tap's
    /// own COMP lies inside the well, so it is not an N+ diode.
    fn welltap(&self, v: &mut Vec<GdsElement>, cx: f64, cy: f64) {
        v.push(self.cut(self.nwell, cx, cy, 1.0));
        v.push(self.cut(self.comp, cx, cy, 0.36));
        v.push(self.cut(self.nplus, cx, cy, 0.5));
        v.push(self.cut(self.contact, cx, cy, CONT));
    }

    /// Dualgate over a transistor at (x, y), out to `to` in x - the whole gate when that
    /// is past x + 1.11, part of it in between, none of it at x + 0.9.
    fn dg(&self, x: f64, y: f64, to: f64) -> GdsElement {
        rect(self.dualgate, x - 0.5, y - 0.5, to, y + 0.5)
    }
}

fn hardening(pdk: &PdkConfig) {
    let h = Hw {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        nwell: layer(pdk, "nwell"),
        poly2: layer(pdk, "poly2_drawn"),
        dualgate: layer(pdk, "dualgate"),
        res_mk: layer(pdk, "res_mk"),
        fusetop: layer(pdk, "fusetop"),
        contact: layer(pdk, "contact"),
        vias: [
            layer(pdk, "via1"),
            layer(pdk, "via2"),
            layer(pdk, "via3"),
            layer(pdk, "via4"),
        ],
        metals: [
            layer(pdk, "metal1_drawn"),
            layer(pdk, "metal2_drawn"),
            layer(pdk, "metal3_drawn"),
            layer(pdk, "metal4_drawn"),
            layer(pdk, "metal5_drawn"),
        ],
    };
    let out = |name: &str, v: Vec<GdsElement>| {
        write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", v));
    };
    let o = OFFSET;

    // ANT.1.h1: three Poly2 antennas, no metal anywhere.  A head 31.4 µm wide gives the
    // Poly2 shape a perimeter of 66.4 µm, and 66.4 × 0.2 / 0.06615 = 200.7, over ANT.1's
    // 200; a head of 31.0 gives 198.3, under it.  The third has Dualgate over its gate:
    // ANT.1 is read on every gate, thick or thin, so it fires like the first.
    {
        let mut v = Vec::new();
        h.transistor(&mut v, o, o, 31.4);
        h.transistor(&mut v, o, o + 5.0, 31.0);
        h.transistor(&mut v, o, o + 10.0, 31.4);
        v.push(h.dg(o, o + 10.0, o + 2.5));
        out("ANT.1.h1", v);
    }

    // ANT.1.h2: a gate with a 10 µm head (perimeter 23.6, ratio 71) strapped through a
    // contact and Metal1 to a gateless 40 µm Poly2 pad 12 µm away.  Poly2's ratio is read
    // before the contact level exists - "structures ... that connect to an active area
    // only via layers below it" - so the far pad is not on the gate's node and the two
    // Poly2 shapes are not added.  Added, the ratio would be 316.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 10.0);
        v.push(h.cut(h.contact, cx, cy, CONT));
        v.push(rect(h.poly2, o + 22.0, o + 0.6, o + 62.0, o + 1.1));
        v.push(h.cut(h.contact, o + 23.0, cy - 0.15, CONT));
        v.push(rect(h.metals[0], cx - 0.4, cy - 0.4, o + 23.5, cy + 0.4));
        out("ANT.1.h2", v);
    }

    // ANT.1.h3: the 200.7 antenna of h1 with an N+ diode contacted onto the Metal1 that
    // straps its gate.  ANT.16's diode relief is written for "Metaln or Vian"; Poly2 has
    // none, and at the Poly2 level the diode is not on the node anyway.  Still fires.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 31.4);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.ndiode(&mut v, cx + 3.0, cy + 1.5);
        v.push(rect(h.metals[0], cx - 0.4, cy - 0.4, cx + 3.4, cy + 1.9));
        out("ANT.1.h3", v);
    }

    // ANT.8.h1: fourteen contacts on the gate's Poly2 head, 14 × 0.0484 = 0.6776 µm² of
    // contact over 0.06615 µm² of gate: 10.24, over ANT.8's 10.  (Thirteen would be 9.5.)
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 4.5);
        h.cuts(&mut v, h.contact, (cx, cy - 0.25), (14, 7), 0.5, CONT);
        v.push(rect(h.metals[0], cx - 0.3, cy - 0.5, cx + 3.3, cy + 0.3));
        out("ANT.8.h1", v);
    }

    // ANT.8.h2: thirteen contacts on the gate's head (9.51, clean), twenty more on a
    // gateless Poly2 pad 10 µm away that the same Metal1 strap reaches, and twenty on the
    // transistor's own source/drain COMP.  Contact is read before Metal1 joins anything,
    // and the diffusion is a different node from the gate: neither set is added to the
    // gate's thirteen.  The far twenty added would be 24.2, the four source/drain ones
    // 12.4.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 4.5);
        h.cuts(&mut v, h.contact, (cx, cy - 0.25), (13, 7), 0.5, CONT);
        v.push(rect(h.poly2, o + 11.0, o + 0.6, o + 16.5, o + 1.4));
        h.cuts(
            &mut v,
            h.contact,
            (o + 11.3, cy - 0.25),
            (20, 10),
            0.5,
            CONT,
        );
        v.push(rect(h.metals[0], cx - 0.3, cy - 0.5, o + 16.4, cy + 0.3));
        for x in [0.3, 0.7, 1.35, 1.75] {
            v.push(h.cut(h.contact, o + x, o + 0.1575, CONT));
        }
        out("ANT.8.h2", v);
    }

    // ANT.16_i_ANT.2.h1: the manual's own repair - "break the metal close to the gate and
    // jog the metal to an upper metal level".  The gate's Metal1 pad rises through Via1
    // to a 6 µm Metal2 bridge and back down through a second Via1 to a 24.34 µm Metal1
    // bar (perimeter 50, 50 × 0.54 / 0.06615 = 408, over 400).  Metal1's ratio is read
    // with only Metal1 connected, so the bar is not on the gate's node: clean.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        v.push(h.bar(h.metals[0], cx, cy, 2.0 * HW));
        v.push(h.cut(h.vias[0], cx, cy, VIA));
        v.push(h.bar(h.metals[1], cx, cy, 6.0));
        v.push(h.cut(h.vias[0], cx + 5.0, cy, VIA));
        v.push(h.bar(h.metals[0], cx + 5.0, cy, 24.34));
        out("ANT.16_i_ANT.2.h1", v);
    }

    // ANT.16_i_ANT.2.h2: the same 24.34 µm bar hung straight off the gate's contact.
    // 408 over 400: fires.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 0, Some((0, 24.34)));
        out("ANT.16_i_ANT.2.h2", v);
    }

    // ANT.16_i_ANT.2.h3: a 91.84 µm bar, perimeter 185, 185 × 0.54 / 0.06615 = 1510.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 0, Some((0, 91.84)));
        out("ANT.16_i_ANT.2.h3", v);
    }

    // ANT.16_i_ANT.2.h4: the 1510 bar of h3 with an N+ diode under it.  ANT.16 case (b)
    // for a thin gate: gate area becomes 0.06615 + 2 × 0.1296 = 0.32535 and the ratio
    // 99.9 / 0.32535 = 307, under 400.  Counted once instead of twice it would be 510.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.ndiode(&mut v, cx + 3.0, cy);
        h.riser(&mut v, cx, cy, 0, Some((0, 91.84)));
        out("ANT.16_i_ANT.2.h4", v);
    }

    // ANT.16_i_ANT.2.h5: the same bar with a 1 µm² N-well tied to the node by an N+ tap
    // instead of a diode.  The well is the PMOS side's protection diode: 0.06615 + 2 × 1
    // = 2.066 and the ratio 48.  Without the well it is 1510.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.welltap(&mut v, cx + 3.0, cy);
        h.riser(&mut v, cx, cy, 0, Some((0, 91.84)));
        out("ANT.16_i_ANT.2.h5", v);
    }

    // ANT.16_i_ANT.2.h6: a 184.34 µm bar (perimeter 370, 199.8 µm² of perimeter area)
    // with a P+ diode in a 1 µm N-well that nothing ties.  Only the 0.1296 µm² of P+
    // diffusion is on the node: 199.8 / 0.32535 = 614, over 400.  With the untied well
    // counted as well it would be 86 and this would be clean.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.pdiode(&mut v, cx + 3.0, cy);
        h.riser(&mut v, cx, cy, 0, Some((0, 184.34)));
        out("ANT.16_i_ANT.2.h6", v);
    }

    // ANT.16_ii_ANT.2.h1: the 199.8 µm² bar of h6 on a gate Dualgate covers, with the N+
    // diode of h4.  ANT.16 case (b) 2 for a thick gate multiplies the diffusion by 15:
    // 0.06615 + 15 × 0.1296 = 2.010 and the ratio 99, clean.  At the thin gate's factor
    // of 2 it would be 614 and fire.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.dg(o, o, o + 2.5));
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.ndiode(&mut v, cx + 3.0, cy);
        h.riser(&mut v, cx, cy, 0, Some((0, 184.34)));
        out("ANT.16_ii_ANT.2.h1", v);
    }

    // ANT.16_ii_ANT.2.h2: two gates under a 17.86 µm bar (perimeter 37, 19.98 µm² of
    // perimeter area, 302 against a whole gate and 604 against half of one).  Dualgate
    // covers the left half of the first gate and stops at the left edge of the second's
    // Poly2.  The marker names an area, so the first gate is half thin and half thick and
    // both halves are over 400; the second is thin and whole, and clean.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.dg(o, o, o + 1.005));
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 0, Some((0, 17.86)));
        let (dx, dy) = h.transistor(&mut v, o, o + 6.0, 1.5);
        v.push(h.dg(o, o + 6.0, o + 0.9));
        v.push(h.cut(h.contact, dx, dy, CONT));
        h.riser(&mut v, dx, dy, 0, Some((0, 17.86)));
        out("ANT.16_ii_ANT.2.h2", v);
    }

    // ANT.1.h4: the 200.7 Poly2 antenna of h1 and the 408 Metal1 bar of h2 on a gate
    // RES_MK covers.  A marked gate is a resistor body, not a transistor - the deck's
    // gate layer takes RES_MK out - so there is no gate oxide to be an antenna to and
    // neither rule has a denominator.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 31.4);
        v.push(rect(h.res_mk, o + 0.5, o - 0.5, o + 1.5, o + 0.5));
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 0, Some((0, 24.34)));
        out("ANT.1.h4", v);
    }

    // ANT.16_i_ANT.6.h1: a 10.84 µm Metal5 bar (perimeter 23) at the top of a full riser.
    // Metal5 is this stack's MetalTop and 1.19 µm thick, so 23 × 1.19 / 0.06615 = 413.8,
    // over 400.  At an intermediate metal's 0.54 µm it would be 187.8 and clean.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 4, Some((4, 10.84)));
        out("ANT.16_i_ANT.6.h1", v);
    }

    // ANT.16_i_ANT.9.h1: twenty Via1 on the gate's Metal1, 20 × 0.0676 / 0.06615 = 20.4,
    // over ANT.9's 20.  (Nineteen would be 19.4.)
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        v.push(rect(h.metals[0], cx - 0.5, cy - 0.8, cx + 5.3, cy + 0.8));
        h.cuts(&mut v, h.vias[0], (cx, cy - 0.25), (20, 10), 0.5, VIA);
        v.push(rect(h.metals[1], cx - 0.5, cy - 0.8, cx + 5.3, cy + 0.8));
        out("ANT.16_i_ANT.9.h1", v);
    }

    // ANT.16_i_ANT.9.h2: the gate's Metal1 carries one Via1 up to a Metal2 bridge, and
    // the bridge comes back down to a Metal1 island carrying twenty more.  Via1's ratio
    // is read with Metal2 not yet there, so the island's vias are not on the gate's node:
    // the gate's node has one via, 1.02.  All twenty-one added would be 21.5.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, o, o, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        v.push(h.bar(h.metals[0], cx, cy, 2.0 * HW));
        v.push(h.cut(h.vias[0], cx, cy, VIA));
        v.push(h.bar(h.metals[1], cx, cy, 12.0));
        v.push(rect(h.metals[0], cx + 5.0, cy - 0.8, cx + 10.8, cy + 0.8));
        h.cuts(&mut v, h.vias[0], (cx + 5.5, cy - 0.25), (20, 10), 0.5, VIA);
        out("ANT.16_i_ANT.9.h2", v);
    }

    // ANT.16_iii_ANT.14_M5_MIMB.h1: a MIM-B cap drawn as section 10.4.2 describes it -
    // Metal4 the bottom plate, FuseTop the top plate over it, and the top plate taken up
    // through Via4 ("Vian-1", n = 5) to Metal5.  The plate is 0.5 µm square, 0.25 µm² of
    // MIM, and the Metal5 bar is 41.84 µm long: perimeter 85, 85 × 1.19 / 0.25 = 404.6,
    // over ANT.14's 400.
    {
        let mut v = Vec::new();
        let (cx, cy) = (o + 2.0, o + 2.0);
        v.push(rect(h.metals[3], cx - 1.5, cy - 1.5, cx + 1.5, cy + 1.5));
        v.push(h.cut(h.fusetop, cx, cy, 0.5));
        v.push(h.cut(h.vias[3], cx, cy, VIA));
        v.push(h.bar(h.metals[4], cx, cy, 41.84));
        out("ANT.16_iii_ANT.14_M5_MIMB.h1", v);
    }

    // ANT.16_iii_ANT.15_V4_MIMB.h1: the same cap with a sea of Via4 on the bottom plate's
    // arm beside it - seventy-five cuts beside the plate's own, 76 × 0.0676 / 0.25 = 20.6,
    // over ANT.15's 20.  At the Via4 level the bottom plate carries the cuts onto the
    // cap's node.
    {
        let mut v = Vec::new();
        let (cx, cy) = (o + 2.0, o + 2.0);
        v.push(rect(h.metals[3], cx - 1.5, cy - 1.5, cx + 11.0, cy + 1.5));
        v.push(h.cut(h.fusetop, cx, cy, 0.5));
        v.push(h.cut(h.vias[3], cx, cy, VIA));
        h.cuts(&mut v, h.vias[3], (cx + 2.0, cy - 1.0), (75, 15), 0.5, VIA);
        v.push(h.bar(h.metals[4], cx, cy, 2.0 * HW));
        out("ANT.16_iii_ANT.15_V4_MIMB.h1", v);
    }

    // ANT.16_i_ANT.2.h7: the 408 antenna of h2 again, with the gate on the tile line at
    // x = 20 and the bar running across the lines at 21, 40 and 42.  One violation,
    // whatever the tiles.
    {
        let mut v = Vec::new();
        let (cx, cy) = h.transistor(&mut v, 19.0, 20.0, 1.5);
        v.push(h.cut(h.contact, cx, cy, CONT));
        h.riser(&mut v, cx, cy, 0, Some((0, 24.34)));
        out("ANT.16_i_ANT.2.h7", v);
    }
}
