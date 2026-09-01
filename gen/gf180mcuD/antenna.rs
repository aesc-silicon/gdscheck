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
}
