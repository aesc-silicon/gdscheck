// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the Via1-Via4 decks: section 5.19 (V1.a-V1.c1) and section 5.20
//! (Vn.a-Vn.c1, one rule set for Via2-Via4) of the SG13G2 layout rules, with section
//! 6.10's sentence on the sealring.  Every layout is drawn with the same geometry on
//! Via1-Via4 (Metal(n) below); only the enclosure `c` differs (V1.c is 0.01, Vn.c is
//! 0.005), so the V(n).c/c1 layouts take their margins from it.  Via1's layouts are
//! `tests/data/ihp-sg13g2/via1/V1.<rule>.h<k>.gds.gz`; Via2, Via3 and Via4 share
//! `tests/data/ihp-sg13g2/vian/V.<rule>.h<k>.gds.gz`, the three layers at the same
//! place - a Via(n) deck reads its own via and metal of it and nothing else.

use crate::helpers::{chamfered_tr, diamond, layer, library, poly, rect, strip45, um, write_gz};
use gds21::{GdsArrayRef, GdsDateTime, GdsElement, GdsLibrary, GdsPoint, GdsStruct};
use gdscheck::pdk::PdkConfig;

/// Via side, V(n).a is exact.
const VIA: f64 = 0.19;

/// What the layers' patterns of one layout are gathered into: the elements of a flat
/// layout, or an array reference of a cell (`cols × rows` at `px`, `py`) beside flat
/// elements.
enum Out {
    Flat(Vec<GdsElement>),
    Array {
        cell: Vec<GdsElement>,
        cols: i16,
        rows: i16,
        px: f64,
        py: f64,
        extra: Vec<GdsElement>,
    },
}

/// The layouts of one directory, gathered across the layers that share it.
struct Group {
    dir: String,
    /// The file prefix: `V1` for Via1's own directory, `V` for the shared one.
    prefix: String,
    out: std::cell::RefCell<std::collections::BTreeMap<String, Out>>,
}

impl Group {
    fn new(dir: &str, prefix: &str) -> std::rc::Rc<Group> {
        std::rc::Rc::new(Group {
            dir: format!("tests/data/ihp-sg13g2/{dir}"),
            prefix: prefix.to_string(),
            out: Default::default(),
        })
    }

    /// Writes every gathered layout.
    fn flush(&self) {
        std::fs::create_dir_all(&self.dir).expect("failed to create output directory");
        for (name, o) in self.out.borrow_mut().iter_mut() {
            let path = format!("{}/{}{name}.gds.gz", self.dir, self.prefix);
            match o {
                Out::Flat(v) => write_gz(&path, library("TOP", std::mem::take(v))),
                Out::Array {
                    cell,
                    cols,
                    rows,
                    px,
                    py,
                    extra,
                } => write_gz(
                    &path,
                    ref_array_with(
                        std::mem::take(cell),
                        *cols,
                        *rows,
                        *px,
                        *py,
                        std::mem::take(extra),
                    ),
                ),
            }
        }
    }
}

/// The layers of one Via(n) deck.
struct L {
    group: std::rc::Rc<Group>,
    /// Via(n).
    v: (i16, i16),
    /// Metal(n), the metal V(n).c/c1 ask to enclose the via.
    m: (i16, i16),
    seal: (i16, i16),
    /// V(n).c's value: 0.01 on Via1, 0.005 on Via2-4.
    c: f64,
    /// A line just wide enough for a via with `c` either side.
    w: f64,
}

impl L {
    fn new(pdk: &PdkConfig, n: i32, group: std::rc::Rc<Group>) -> Self {
        let c = if n == 1 { 0.01 } else { 0.005 };
        L {
            group,
            v: layer(pdk, &format!("Via{n}")),
            m: layer(pdk, &format!("Metal{n}")),
            seal: layer(pdk, "EdgeSeal"),
            c,
            w: VIA + 2.0 * c,
        }
    }

    /// Gathers this layer's elements of `<rule>`, e.g. `write(".a.h1", ..)` into
    /// `V.a.h1.gds.gz` (or `V1.a.h1.gds.gz`).
    fn write(&self, rule: &str, mut elems: Vec<GdsElement>) {
        let mut out = self.group.out.borrow_mut();
        match out
            .entry(rule.to_string())
            .or_insert_with(|| Out::Flat(Vec::new()))
        {
            Out::Flat(v) => v.append(&mut elems),
            Out::Array { extra, .. } => extra.append(&mut elems),
        }
    }

    /// Gathers this layer's `cell` of an array layout `<rule>`: `cols × rows` at `px`,
    /// `py`, with `extra` drawn flat beside it.
    #[allow(clippy::too_many_arguments)]
    fn write_array(
        &self,
        rule: &str,
        mut cell: Vec<GdsElement>,
        cols: i16,
        rows: i16,
        px: f64,
        py: f64,
        mut extra: Vec<GdsElement>,
    ) {
        let mut out = self.group.out.borrow_mut();
        match out.entry(rule.to_string()).or_insert_with(|| Out::Array {
            cell: Vec::new(),
            cols,
            rows,
            px,
            py,
            extra: Vec::new(),
        }) {
            Out::Array {
                cell: c, extra: e, ..
            } => {
                c.append(&mut cell);
                e.append(&mut extra);
            }
            Out::Flat(v) => v.append(&mut extra),
        }
    }

    /// A 0.19 via with its lower-left corner at `(x, y)`.
    fn via(&self, x: f64, y: f64) -> GdsElement {
        rect(self.v, x, y, x + VIA, y + VIA)
    }

    /// `cols × rows` vias from `(x, y)` with gaps `gx` and `gy`.
    fn grid(&self, x: f64, y: f64, cols: usize, rows: usize, gx: f64, gy: f64) -> Vec<GdsElement> {
        let mut out = vec![];
        for r in 0..rows {
            for c in 0..cols {
                out.push(self.via(x + c as f64 * (VIA + gx), y + r as f64 * (VIA + gy)));
            }
        }
        out
    }

    /// A square ring of vias `k` per side, `thick` vias deep, at gap `g`, from `(x, y)`.
    fn via_ring(&self, x: f64, y: f64, k: usize, thick: usize, g: f64) -> Vec<GdsElement> {
        let mut out = vec![];
        for r in 0..k {
            for c in 0..k {
                if r < thick || r >= k - thick || c < thick || c >= k - thick {
                    out.push(self.via(x + c as f64 * (VIA + g), y + r as f64 * (VIA + g)));
                }
            }
        }
        out
    }

    /// A rectangular frame `(x0, y0)-(x1, y1)` with the hole `(hx0, hy0)-(hx1, hy1)`, drawn
    /// as four abutting boxes that merge into one ring.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &self,
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
}

/// A `GdsArrayRef` of `cell` (`cols × rows` at `px`, `py`) placed in TOP beside `extra`,
/// drawn flat.
fn ref_array_with(
    cell: Vec<GdsElement>,
    cols: i16,
    rows: i16,
    px: f64,
    py: f64,
    extra: Vec<GdsElement>,
) -> GdsLibrary {
    let aref = GdsElement::GdsArrayRef(GdsArrayRef {
        name: "CELL".into(),
        xy: [
            GdsPoint::new(0, 0),
            GdsPoint::new(um(px * cols as f64), 0),
            GdsPoint::new(0, um(py * rows as f64)),
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

pub fn generate(pdk: &PdkConfig) {
    let via1 = Group::new("via1", "V1");
    let vian = Group::new("vian", "V");
    for n in 1..5 {
        let l = L::new(pdk, n, if n == 1 { via1.clone() } else { vian.clone() });
        vn_a(&l);
        vn_b(&l);
        vn_b1(&l);
        vn_c(&l);
        vn_c1(&l);
    }
    via1.flush();
    vian.flush();
}

// --- V(n).a: min. and max. Via(n) width 0.19 ---

fn vn_a(l: &L) {
    let v = l.v;

    // h1 — the bound.  A 0.19 square is the via; 0.19 × 0.195 and 0.195 × 0.19 are off in
    // one direction, 0.185 and 0.195 squares in both; a 0.19 × 1 bar and an L of 0.19 arms
    // are 0.19 between facing walls but no 0.19 via.
    l.write(
        ".a.h1",
        vec![
            l.via(2.0, 2.0),                   // clean
            rect(v, 5.0, 2.0, 5.19, 2.195),    // tall → V(n).a
            rect(v, 8.0, 2.0, 8.195, 2.19),    // wide → V(n).a
            rect(v, 11.0, 2.0, 11.185, 2.185), // small → V(n).a
            rect(v, 14.0, 2.0, 14.195, 2.195), // big → V(n).a
            rect(v, 17.0, 2.0, 17.19, 2.185),  // short → V(n).a
            rect(v, 2.0, 5.0, 3.0, 5.19),      // bar → V(n).a
            poly(
                v,
                &[
                    (5.0, 5.0),
                    (5.5, 5.0),
                    (5.5, 5.19),
                    (5.19, 5.19),
                    (5.19, 5.5),
                    (5.0, 5.5),
                ],
            ), // L → V(n).a
        ],
    );

    // h2 — shapes that merge.  Two 0.19 squares overlapping by 0.09 are a 0.29 bar; two
    // abutting halves and four quadrants are a via; a 0.005 sliver on a via's side makes
    // it 0.195; a via drawn clockwise is a via; two vias touching at a corner merge into
    // one shape 0.19 wide everywhere - and 0.000 apart, which is V(n).b's; two overlapping
    // by 0.005 at a corner are one non-rectangle.
    l.write(
        ".a.h2",
        vec![
            l.via(2.0, 2.0),
            l.via(2.1, 2.0), // 0.29 bar → V(n).a
            rect(v, 5.0, 2.0, 5.095, 2.19),
            rect(v, 5.095, 2.0, 5.19, 2.19), // halves → clean
            rect(v, 8.0, 2.0, 8.095, 2.095),
            rect(v, 8.095, 2.0, 8.19, 2.095),
            rect(v, 8.0, 2.095, 8.095, 2.19),
            rect(v, 8.095, 2.095, 8.19, 2.19), // quadrants → clean
            l.via(11.0, 2.0),
            rect(v, 11.19, 2.0, 11.195, 2.19), // sliver → 0.195 → V(n).a
            poly(v, &[(14.0, 2.0), (14.0, 2.19), (14.19, 2.19), (14.19, 2.0)]), // clockwise → clean
            l.via(17.0, 2.0),
            l.via(17.19, 2.19), // corner touch → V(n).b (a space of nothing)
            l.via(2.0, 5.0),
            l.via(2.185, 5.185), // 0.005 corner overlap → V(n).a
        ],
    );

    // h3 — tile lines.  0.185 squares inside a tile at x = 10, straddling 20, ending on 20,
    // starting on 20, straddling 21, 40, 42 and y = 20; a 0.19 straddling 20 and one
    // ending on 20 are clean.  Eight off-size squares whatever the tile.
    let sq = |x: f64, y: f64, s: f64| rect(v, x, y, x + s, y + s);
    l.write(
        ".a.h3",
        vec![
            sq(9.9, 2.0, 0.185),
            sq(19.9, 2.0, 0.185),
            sq(19.815, 4.0, 0.185),
            sq(20.0, 6.0, 0.185),
            sq(20.9, 2.0, 0.185),
            sq(39.9, 2.0, 0.185),
            sq(41.9, 2.0, 0.185),
            sq(10.0, 19.9, 0.185),
            sq(19.9, 8.0, 0.19),   // clean
            sq(19.81, 10.0, 0.19), // clean
        ],
    );

    // h6 — a 0.005 × 0.19 sliver, a 0.19 × 300 bar and a 0.185 square at (1000, 1000).
    l.write(
        ".a.h6",
        vec![
            rect(v, 2.0, 2.0, 2.005, 2.19),
            rect(v, 2.0, 5.0, 302.0, 5.19),
            sq(1000.0, 1000.0, 0.185),
        ],
    );

    // h7 — the sealring.  Section 6.10: "standard metal and via rules are not checked
    // within EdgeSeal regions"; section 6.10's Seal.c1 makes the seal's via a 0.19-wide
    // ring.  A 0.185 square and a 4 × 4 via ring 0.19 wide under an EdgeSeal are exempt;
    // the same square and ring outside it fire; a via across the seal's edge has a
    // 0.095 × 0.19 part outside (cut at the edge, as the metal is).
    let mut e = vec![
        rect(l.seal, 1.0, 1.0, 9.0, 9.0),
        sq(2.0, 2.0, 0.185),  // exempt
        sq(12.0, 2.0, 0.185), // V(n).a
        rect(l.seal, 20.0, 1.0, 22.0, 3.0),
        sq(21.905, 2.0, 0.19), // across the edge → V(n).a on the outside part
    ];
    e.extend(l.ring(v, 4.0, 4.0, 8.0, 8.0, 4.19, 4.19, 7.81, 7.81)); // exempt
    e.extend(l.ring(v, 12.0, 4.0, 16.0, 8.0, 12.19, 4.19, 15.81, 7.81)); // V(n).a
    l.write(".a.h7", e);
}

// --- V(n).b: min. Via(n) space 0.22 ---

fn vn_b(l: &L) {
    // h1 — the bound and both metrics.  0.22 apart clean, 0.215 fires (x and y); corner to
    // corner dx = dy = 0.155 is 0.219 euclidian (fires), 0.16 is 0.226 (clean); dx = dy =
    // 0.2 is under 0.22 on either axis but 0.283 euclidian (clean); (0.1, 0.195) is 0.219
    // (fires), (0.1, 0.2) is 0.224 (clean); a corner 0.215 off a wall with its projection
    // half over it fires; three in a row at 0.215 are two pairs, an L of three two (its
    // diagonal is 0.304), a 2 × 2 block four.
    l.write(
        ".b.h1",
        vec![
            l.via(2.0, 2.0),
            l.via(2.41, 2.0), // 0.22 → clean
            l.via(4.0, 2.0),
            l.via(4.405, 2.0), // 0.215 (x) → V(n).b
            l.via(6.0, 2.0),
            l.via(6.0, 2.405), // 0.215 (y) → V(n).b
            l.via(8.0, 2.0),
            l.via(8.345, 2.345), // diagonal 0.219 → V(n).b
            l.via(10.0, 2.0),
            l.via(10.35, 2.35), // diagonal 0.226 → clean
            l.via(12.0, 2.0),
            l.via(12.39, 2.39), // 0.2/0.2 axes, 0.283 euclidian → clean
            l.via(14.0, 2.0),
            l.via(14.29, 2.385), // (0.1, 0.195) → 0.219 → V(n).b
            l.via(16.0, 2.0),
            l.via(16.29, 2.39), // (0.1, 0.2) → 0.224 → clean
            l.via(18.0, 2.0),
            l.via(18.405, 2.1), // corner-on 0.215 → V(n).b
            l.via(2.0, 5.0),
            l.via(2.405, 5.0),
            l.via(2.81, 5.0), // row of three → 2
            l.via(5.0, 5.0),
            l.via(5.405, 5.0),
            l.via(5.0, 5.405), // L of three → 2
            l.via(8.0, 5.0),
            l.via(8.405, 5.0),
            l.via(8.0, 5.405),
            l.via(8.405, 5.405), // 2 × 2 block → 4
        ],
    );

    // h2 — tile lines.  0.215 gaps straddling x = 20, 21, 40, 42, 14 and y = 20; a gap
    // that begins on x = 20 and one that ends on it; a pair at (1000, 1000).
    let pair = |x: f64, y: f64| vec![l.via(x, y), l.via(x + VIA + 0.215, y)];
    let mut e = vec![];
    e.extend(pair(19.7, 2.0));
    e.extend(pair(20.7, 4.0));
    e.extend(pair(39.7, 2.0));
    e.extend(pair(41.7, 4.0));
    e.extend(pair(13.7, 2.0));
    e.extend(vec![l.via(10.0, 19.7), l.via(10.0, 20.105)]);
    e.extend(pair(19.81, 6.0)); // gap 20.0..20.215
    e.extend(pair(19.595, 8.0)); // gap 19.785..20.0
    e.extend(pair(1000.0, 1000.0));
    l.write(".b.h2", e);

    // h5 — large.  A 0.19 × 300 via bar (V(n).a too) with a via 0.215 below its middle
    // and one 0.22 below its far end: one pair.
    l.write(
        ".b.h5",
        vec![
            rect(l.v, 2.0, 5.0, 302.0, 5.19),
            l.via(150.0, 4.595), // 0.215 → V(n).b
            l.via(301.0, 4.59),  // 0.22 → clean
        ],
    );

    // h6 — the sealring (section 6.10).  A 0.215 pair under an EdgeSeal is exempt; the
    // same outside fires; a via outside the seal 0.215 from a via inside it: the seal's
    // via is not checked, so no pair.
    l.write(
        ".b.h6",
        vec![
            rect(l.seal, 1.0, 1.0, 4.0, 4.0),
            l.via(2.0, 2.0),
            l.via(2.405, 2.0), // exempt
            l.via(8.0, 2.0),
            l.via(8.405, 2.0), // V(n).b
            rect(l.seal, 11.0, 1.0, 12.0, 4.0),
            l.via(11.7, 2.0),
            l.via(12.105, 2.0), // one in, one out → clean
        ],
    );
}

// --- V(n).b1: min. Via(n) space 0.29 in an array of more than 3 rows and more than 3 columns, one direction ---

fn vn_b1(l: &L) {
    // h1 — the array's definition.  4 × 4 at 0.22/0.22 fires; 4 × 4 at 0.29 in x and 0.22
    // in y, or the reverse, is clean (note 1: one direction suffices); 3 rows × 4 cols,
    // 4 rows × 3 cols and 3 × 3 at 0.22 are not arrays the rule covers; 4 × 4 at
    // 0.285/0.285 fires; 4 × 4 at 0.29/0.29 is clean; 4 × 4 at 0.25/0.25 fires, at
    // 0.25/0.30 and 0.30/0.22 is clean.
    let mut e = vec![];
    e.extend(l.grid(2.0, 2.0, 4, 4, 0.22, 0.22)); // fires
    e.extend(l.grid(6.0, 2.0, 4, 4, 0.29, 0.22)); // clean
    e.extend(l.grid(10.0, 2.0, 4, 4, 0.22, 0.29)); // clean
    e.extend(l.grid(14.0, 2.0, 4, 3, 0.22, 0.22)); // 3 rows: clean
    e.extend(l.grid(2.0, 6.0, 3, 4, 0.22, 0.22)); // 3 cols: clean
    e.extend(l.grid(6.0, 6.0, 4, 4, 0.285, 0.285)); // fires
    e.extend(l.grid(10.0, 6.0, 4, 4, 0.29, 0.29)); // clean
    e.extend(l.grid(14.0, 6.0, 3, 3, 0.22, 0.22)); // clean
    e.extend(l.grid(2.0, 10.0, 4, 4, 0.25, 0.25)); // fires
    e.extend(l.grid(6.0, 10.0, 4, 4, 0.25, 0.30)); // clean
    e.extend(l.grid(10.0, 10.0, 4, 4, 0.30, 0.22)); // clean
    l.write(".b1.h1", e);

    // h2 — larger and irregular arrays.  5 × 5 and 4 rows × 10 cols at 0.22 fire; a 4 × 4
    // whose row gaps are 0.22/0.29/0.22 has no direction at 0.29 throughout (fires); four
    // rows of four staggered by half a pitch, rows 0.22 apart (0.2205 corner to corner),
    // is an array in the rule's sense (fires); a 4 × 4 drawn as two 2-column blocks is
    // one array (fires); a 4 × 4 with a fifth via on one row is one array (fires); a 4 × 4
    // missing a corner via has a row of three and is no array of four (the cont report's
    // reading of a 5 × 5 missing its centre: clean).
    let mut e = vec![];
    e.extend(l.grid(2.0, 2.0, 5, 5, 0.22, 0.22));
    e.extend(l.grid(6.0, 2.0, 10, 4, 0.22, 0.22));
    for dy in [0.0, 0.41, 0.89, 1.30] {
        e.extend(l.grid(12.0, 2.0 + dy, 4, 1, 0.22, 0.0));
    }
    for r in 0..4 {
        let x = 16.0 + if r % 2 == 1 { 0.205 } else { 0.0 };
        e.extend(l.grid(x, 2.0 + r as f64 * 0.41, 4, 1, 0.22, 0.0));
    }
    e.extend(l.grid(2.0, 6.0, 2, 4, 0.22, 0.22));
    e.extend(l.grid(2.82, 6.0, 2, 4, 0.22, 0.22));
    e.extend(l.grid(6.0, 6.0, 4, 4, 0.22, 0.22));
    e.push(l.via(7.64, 6.0));
    let mut cornerless = l.grid(10.0, 6.0, 4, 4, 0.22, 0.22);
    cornerless.remove(15);
    e.extend(cornerless);
    l.write(".b1.h2", e);

    // h3 — with V(n).b.  A 4 × 4 at 0.215/0.215 is V(n).b1 and twenty-four V(n).b; at
    // 0.215 in x and 0.29 in y it is twelve V(n).b and no array violation (one direction
    // relaxed); at 0.22/0.215 it is twelve V(n).b and V(n).b1.
    let mut e = vec![];
    e.extend(l.grid(2.0, 2.0, 4, 4, 0.215, 0.215));
    e.extend(l.grid(6.0, 2.0, 4, 4, 0.215, 0.29));
    e.extend(l.grid(10.0, 2.0, 4, 4, 0.22, 0.215));
    l.write(".b1.h3", e);

    // h4 — tile lines.  4 × 4 arrays at 0.22 (1.42 wide) inside a tile at x = 9.29,
    // straddling x = 20, 21, 40, 42 and 14, one with a column gap starting on x = 20, one
    // with a via starting on x = 20, one cornered on (20, 20), one at (1000, 1000): ten;
    // then a 20 × 20 at 0.22 (8 µm, bigger than a 7 µm tile) across x = 20, 21 and y =
    // 40, 42, and a 4 × 50 (1.42 × 20.3) across y = 7, 14, 20, 21: one each.
    let mut e = vec![];
    for (x, y) in [
        (9.29, 2.0),
        (19.29, 2.0),
        (20.29, 5.0),
        (39.29, 2.0),
        (41.29, 5.0),
        (13.29, 5.0),
        (19.4, 8.0),
        (19.59, 11.0),
        (19.29, 19.29),
        (1000.0, 1000.0),
    ] {
        e.extend(l.grid(x, y, 4, 4, 0.22, 0.22));
    }
    e.extend(l.grid(16.0, 36.0, 20, 20, 0.22, 0.22));
    e.extend(l.grid(30.0, 2.0, 4, 50, 0.22, 0.22));
    l.write(".b1.h4", e);

    // h7 — the array itself as hierarchy: a 4 × 4 `GdsArrayRef` of a one-via cell at pitch
    // 0.41 (0.22 gaps) fires like the flat array; beside it, flat, a 4 × 4 at 0.29 in x
    // (clean).
    l.write_array(
        ".b1.h7",
        vec![l.via(0.0, 0.0)],
        4,
        4,
        0.41,
        0.41,
        l.grid(6.0, 0.0, 4, 4, 0.29, 0.22),
    );

    // h8 — rings and pads.  A bond-pad style via ring one via thick at 0.22 gaps (20 per
    // side) has no more than one row anywhere (clean); two thick is at most two rows
    // (clean); four thick is a 4-deep field all the way round (fires); a 10 × 10 pad
    // array at 0.22 fires.
    let mut e = l.via_ring(2.0, 2.0, 20, 1, 0.22);
    e.extend(l.via_ring(12.0, 2.0, 20, 2, 0.22));
    e.extend(l.via_ring(22.0, 2.0, 20, 4, 0.22));
    e.extend(l.grid(32.0, 2.0, 10, 10, 0.22, 0.22));
    l.write(".b1.h8", e);

    // h9 — the sealring (section 6.10).  A 4 × 4 at 0.22 under an EdgeSeal is exempt;
    // the same outside fires.
    let mut e = vec![rect(l.seal, 1.0, 1.0, 5.0, 5.0)];
    e.extend(l.grid(2.0, 2.0, 4, 4, 0.22, 0.22));
    e.extend(l.grid(8.0, 2.0, 4, 4, 0.22, 0.22));
    l.write(".b1.h9", e);
}

// --- V(n).c: min. Metal(n) enclosure of Via(n) 0.01 (Via1) / 0.005 (Via2-4) ---

fn vn_c(l: &L) {
    let (m, c, w) = (l.m, l.c, l.w);
    let s = c - 0.005; // one step short of the value

    // h1 — the bound.  A via in a `w`-wide line with `c` above and below is clean; one with
    // `c − 0.005` below fires; one sticking 0.005 out fires; `c` either side in a vertical
    // line is clean; `c` on the left of a pad is clean; a via in a pad's corner with 0.000
    // on two sides fires, and one with `c − 0.005` on two sides fires.
    l.write(
        ".c.h1",
        vec![
            rect(m, 2.0, 2.0, 3.0, 2.0 + w),
            l.via(2.4, 2.0 + c), // clean
            rect(m, 5.0, 2.0, 6.0, 2.0 + w),
            l.via(5.4, 2.0 + s), // short below → V(n).c
            rect(m, 8.0, 2.0, 9.0, 2.0 + w),
            l.via(8.4, 1.995), // sticks 0.005 out → V(n).c
            rect(m, 11.0, 2.0, 11.0 + w, 3.0),
            l.via(11.0 + c, 2.4), // clean
            rect(m, 14.0, 2.0, 14.6, 2.6),
            l.via(14.0 + c, 2.2), // clean
            rect(m, 17.0, 2.0, 17.6, 2.6),
            l.via(17.0, 2.0), // corner, 0.000 on two sides → V(n).c
            rect(m, 2.0, 5.0, 2.6, 5.6),
            l.via(2.0 + s, 5.0 + s), // corner, short on two sides → V(n).c
        ],
    );

    // h2 — no cover.  A bare via, a via half out of a line's end, a via 0.1 beside a line,
    // a via in the hole of a metal ring and a via on the hole's edge: none is enclosed.
    // (All are V(n).c1 too: no side reaches 0.05.)
    let mut e = vec![
        l.via(2.0, 2.0),
        rect(m, 5.0, 2.0, 6.0, 2.0 + w),
        l.via(5.9, 2.0 + c),
        rect(m, 8.0, 2.0, 9.0, 2.0 + w),
        l.via(9.1, 2.0 + c),
    ];
    e.extend(l.ring(m, 12.0, 2.0, 14.0, 4.0, 12.5, 2.5, 13.5, 3.5));
    e.push(l.via(12.9, 2.9));
    e.extend(l.ring(m, 16.0, 2.0, 18.0, 4.0, 16.5, 2.5, 17.5, 3.5));
    e.push(l.via(16.5, 2.9));
    l.write(".c.h2", e);

    // h3 — 45° walls at the via's corner.  A 0.6 pad with the via `c` from its top and
    // right edges, the pad's corner chamfered along x + y = k: k = touch + 0.005 passes
    // 0.0035 from the via's corner with both walls `c` away (the settled projection
    // reading: clean); k = touch passes through the corner (enclosure 0.000); k = touch −
    // 0.01 cuts it.  Then a diamond of metal around a centred via, its walls `d` from the
    // via's corners: d ≥ c and d = the step under it; no wall of the diamond is parallel to
    // the via's, so the projection metric has nothing to measure (see the report).
    let touch = |x: f64| x + 3.2 - 2.0 * c;
    let pad = |x: f64, k: f64| {
        vec![
            chamfered_tr(m, x, 2.0, x + 0.6, 2.6, k),
            l.via(x + 0.41 - c, 2.41 - c),
        ]
    };
    let a_clean = if c > 0.005 { 0.205 } else { 0.20 };
    let mut e = pad(2.0, touch(2.0) + 0.005);
    e.extend(pad(5.0, touch(5.0)));
    e.extend(pad(8.0, touch(8.0) - 0.01));
    e.push(diamond(m, 12.0, 2.3, a_clean));
    e.push(l.via(11.905, 2.205)); // d = 0.0106 (Via1) / 0.0071 → clean
    e.push(diamond(m, 15.0, 2.3, a_clean - 0.005));
    e.push(l.via(14.905, 2.205)); // d = 0.0071 (Via1) / 0.0035
    l.write(".c.h3", e);

    // h4 — unions.  Metal(n) from two overlapping boxes encloses the via by `c` though each
    // box alone clips it (clean); two boxes whose union is short below fire once; a line
    // drawn as ten abutting slices is clean.
    let mut e = vec![
        rect(m, 2.0, 2.0, 2.5, 2.0 + w),
        rect(m, 2.4, 2.0, 3.0, 2.0 + w),
        l.via(2.4, 2.0 + c),
        rect(m, 5.0, 2.0, 5.5, 2.0 + w),
        rect(m, 5.4, 2.0, 6.0, 2.0 + w),
        l.via(5.4, 2.0 + s),
    ];
    for i in 0..10 {
        let x = 8.0 + 0.1 * i as f64;
        e.push(rect(m, x, 2.0, x + 0.1, 2.0 + w));
    }
    e.push(l.via(8.4, 2.0 + c));
    l.write(".c.h4", e);

    // h5 — tile lines.  Vias short below straddling x = 20, 21, 40 and 42; a via whose left
    // edge lies on x = 20 and on the metal edge; a via ending `c` short of x = 20 and of
    // its line's end (V(n).c clean, a `c` endcap with `c` sides for V(n).c1); a via
    // straddling 20 in a 10 µm line (clean).
    l.write(
        ".c.h5",
        vec![
            rect(m, 19.0, 2.0, 22.0, 2.0 + w),
            l.via(19.905, 2.0 + s),
            l.via(20.905, 2.0 + s),
            rect(m, 39.0, 2.0, 43.0, 2.0 + w),
            l.via(39.905, 2.0 + s),
            l.via(41.905, 2.0 + s),
            rect(m, 20.0, 5.0, 20.0 + w, 8.0),
            l.via(20.0, 6.0),
            rect(m, 19.0, 10.0, 20.0, 10.0 + w),
            l.via(20.0 - c - VIA, 10.0 + c),
            rect(m, 15.0, 12.0, 25.0, 12.0 + w),
            l.via(19.905, 12.0 + c),
        ],
    );

    // h8 — a via short below at (1000, 1000); one in the middle of a 300 µm line.
    l.write(
        ".c.h8",
        vec![
            rect(m, 1000.0, 1000.0, 1001.0, 1000.0 + w),
            l.via(1000.4, 1000.0 + s),
            rect(m, 2.0, 2.0, 302.0, 2.0 + w),
            l.via(150.0, 2.0 + s),
        ],
    );

    // h9 — the sealring (section 6.10).  A via short below under an EdgeSeal is exempt;
    // the same outside fires; a via across the EdgeSeal edge has its outside part
    // enclosed by `c − 0.005` below → fires.
    l.write(
        ".c.h9",
        vec![
            rect(l.seal, 1.0, 1.0, 4.0, 4.0),
            rect(m, 2.0, 2.0, 3.0, 2.0 + w),
            l.via(2.4, 2.0 + s),
            rect(m, 8.0, 2.0, 9.0, 2.0 + w),
            l.via(8.4, 2.0 + s),
            rect(l.seal, 11.0, 1.0, 12.0, 4.0),
            rect(m, 11.0, 2.0, 13.0, 2.0 + w),
            l.via(11.905, 2.0 + s),
        ],
    );
}

// --- V(n).c1: min. Metal(n) endcap enclosure of Via(n) 0.05 ---

fn vn_c1(l: &L) {
    let (m, c, w) = (l.m, l.c, l.w);

    // h1 — line ends.  A via at the end of a `w`-wide line (`c` either side, the line
    // continuing on the other side): endcap 0.05 clean, 0.045 fires, 0.000 fires (with
    // V(n).c); a via in the middle of a line is clean; the same at the top of a vertical
    // line and at the left end; two vias at a line's end - the outer one's 0.045 fires,
    // the inner one is covered by the outer.
    l.write(
        ".c1.h1",
        vec![
            rect(m, 2.0, 2.0, 4.0, 2.0 + w),
            l.via(3.76, 2.0 + c), // endcap 0.05 → clean
            rect(m, 6.0, 2.0, 8.0, 2.0 + w),
            l.via(7.765, 2.0 + c), // endcap 0.045 → V(n).c1
            rect(m, 10.0, 2.0, 12.0, 2.0 + w),
            l.via(11.81, 2.0 + c), // endcap 0.000 → V(n).c + V(n).c1
            rect(m, 14.0, 2.0, 16.0, 2.0 + w),
            l.via(14.9, 2.0 + c), // middle → clean
            rect(m, 18.0, 2.0, 18.0 + w, 4.0),
            l.via(18.0 + c, 3.765), // top endcap 0.045 → V(n).c1
            rect(m, 2.0, 5.0, 4.0, 5.0 + w),
            l.via(2.045, 5.0 + c), // left endcap 0.045 → V(n).c1
            rect(m, 6.0, 5.0, 8.0, 5.0 + w),
            l.via(7.765, 5.0 + c), // outer of two → V(n).c1
            l.via(7.355, 5.0 + c),
        ],
    );

    // h2 — corners (note 2: "at least one side must be treated as an endcap").  An L
    // route with 0.3 arms and the via in its corner square, under the vertical arm and
    // beside the horizontal one, so its left and top sides continue into the arms; the
    // outer margins (right, bottom) are: (0.05, c) clean; (0.045, c) fires; (c, c) fires;
    // (0.05, 0.05) clean; (c, 0.05) clean.
    let corner = |x: f64, r: f64, b: f64| {
        vec![
            poly(
                m,
                &[
                    (x - 2.0, 2.0),
                    (x, 2.0),
                    (x, 4.0),
                    (x - 0.3, 4.0),
                    (x - 0.3, 2.3),
                    (x - 2.0, 2.3),
                ],
            ),
            l.via(x - r - VIA, 2.0 + b),
        ]
    };
    let mut e = corner(3.5, 0.05, c);
    e.extend(corner(8.0, 0.045, c));
    e.extend(corner(12.5, c, c));
    e.extend(corner(17.0, 0.05, 0.05));
    e.extend(corner(24.0, c, 0.05));
    l.write(".c1.h2", e);

    // h3 — isolated pads: a via with margins (left, right, bottom, top).  (0.05, c, c, c):
    // three short sides → fires; (0.05, 0.05, c, c): a line passing through, sides tight →
    // clean; (0.05, c, 0.05, c): two good sides adjacent, the other two short → fires; 0.045
    // all round fires; three good sides clean; four clean; c all round fires; (0.05, 0.05,
    // 0.045, 0.05): one short side → clean; (0.045, 0.05, 0.045, 0.05): left and bottom
    // short, two adjacent sides → fires; (0.045, 0.045, 0.05, 0.05): left and right short,
    // an opposite pair, the other pair the endcaps → clean.
    let pad = |x: f64, ml: f64, mr: f64, mb: f64, mt: f64| {
        vec![
            rect(m, x - ml, 2.0 - mb, x + VIA + mr, 2.0 + VIA + mt),
            l.via(x, 2.0),
        ]
    };
    let mut e = pad(2.0, 0.05, c, c, c);
    e.extend(pad(4.0, 0.05, 0.05, c, c));
    e.extend(pad(6.0, 0.05, c, 0.05, c));
    e.extend(pad(8.0, 0.045, 0.045, 0.045, 0.045));
    e.extend(pad(10.0, 0.05, 0.05, 0.05, c));
    e.extend(pad(12.0, 0.05, 0.05, 0.05, 0.05));
    e.extend(pad(14.0, c, c, c, c));
    e.extend(pad(16.0, 0.05, 0.05, 0.045, 0.05));
    e.extend(pad(18.0, 0.045, 0.05, 0.045, 0.05));
    e.extend(pad(20.0, 0.045, 0.045, 0.05, 0.05));
    l.write(".c1.h3", e);

    // h4 — wide lines, junctions and 45° ends.  A via at the end of a 0.5-wide line with
    // 0.155 either side and a 0.045 endcap: the 0.155 sides are an opposite pair over 0.05
    // (clean); the same with 0.05; a via at a T junction (three sides continue, `c` below);
    // a via in the middle of a cross; a `w`-wide line whose end corners are chamfered by
    // 0.05 with the via 0.05 from the straight end: the chamfers run across the span of
    // the via's sides, 0.01 from their ends, so the metal past the via's corners is not
    // 0.05 (fires), and 0.045 fires; the same chamfered end on a 0.5-wide line with the
    // via 0.155 from either side: the chamfers end before the via's sides begin, no wall
    // pairs with them (clean); a via in the middle of a 0.57-wide 45° strip (clean, 0.148
    // to the nearest wall).
    let chamfered_end = |x: f64, y: f64, wd: f64| {
        poly(
            m,
            &[
                (x, y),
                (x + 1.95, y),
                (x + 2.0, y + 0.05),
                (x + 2.0, y + wd - 0.05),
                (x + 1.95, y + wd),
                (x, y + wd),
            ],
        )
    };
    l.write(
        ".c1.h4",
        vec![
            rect(m, 2.0, 2.0, 4.0, 2.5),
            l.via(3.765, 2.155), // endcap 0.045, sides 0.155 → clean
            rect(m, 6.0, 2.0, 8.0, 2.5),
            l.via(7.76, 2.155), // endcap 0.05 → clean
            rect(m, 10.0, 2.0, 13.0, 2.0 + w),
            rect(m, 11.4, 2.0 + w, 11.6, 4.0),
            l.via(11.405, 2.0 + c), // T → clean
            rect(m, 15.0, 3.0, 18.0, 3.0 + w),
            rect(m, 16.4, 2.0, 16.6, 5.0),
            l.via(16.405, 3.0 + c), // cross → clean
            chamfered_end(2.0, 5.0, w),
            l.via(3.76, 5.0 + c), // 0.05 to the chamfered end, chamfers across the sides → V(n).c1
            chamfered_end(6.0, 5.0, w),
            l.via(7.765, 5.0 + c), // 0.045 → V(n).c1
            chamfered_end(10.0, 5.0, 0.5),
            l.via(11.76, 5.155), // wide line, 0.05 to the chamfered end → clean
            strip45(m, 14.0, 6.0, 3.0, 0.4),
            l.via(15.205, 7.605), // centred in the strip → clean
        ],
    );

    // h5 — tile lines.  Line-end vias with a 0.045 endcap: the end on x = 20, the via
    // straddling 20, the end on 21, on 40, the via straddling 42; an end on 20 with a 0.05
    // endcap (clean).
    l.write(
        ".c1.h5",
        vec![
            rect(m, 18.0, 2.0, 20.0, 2.0 + w),
            l.via(19.765, 2.0 + c),
            rect(m, 18.1, 5.0, 20.1, 5.0 + w),
            l.via(19.865, 5.0 + c),
            rect(m, 19.0, 8.0, 21.0, 8.0 + w),
            l.via(20.765, 8.0 + c),
            rect(m, 38.0, 2.0, 40.0, 2.0 + w),
            l.via(39.765, 2.0 + c),
            rect(m, 40.1, 5.0, 42.1, 5.0 + w),
            l.via(41.865, 5.0 + c),
            rect(m, 18.0, 11.0, 20.0, 11.0 + w),
            l.via(19.76, 11.0 + c),
        ],
    );

    // h8 — a 0.045 endcap at (1000, 1000) and at the far end of a 300 µm line.
    l.write(
        ".c1.h8",
        vec![
            rect(m, 1000.0, 1000.0, 1002.0, 1000.0 + w),
            l.via(1001.765, 1000.0 + c),
            rect(m, 2.0, 2.0, 302.0, 2.0 + w),
            l.via(301.765, 2.0 + c),
        ],
    );

    // h9 — the sealring (section 6.10).  A 0.045 endcap under an EdgeSeal is exempt; the
    // same outside fires.
    l.write(
        ".c1.h9",
        vec![
            rect(l.seal, 1.0, 1.0, 5.0, 5.0),
            rect(m, 2.0, 2.0, 4.0, 2.0 + w),
            l.via(3.765, 2.0 + c),
            rect(m, 8.0, 2.0, 10.0, 2.0 + w),
            l.via(9.765, 2.0 + c),
        ],
    );

    // h10 — vias in company.  A 4 × 4 array (0.29 in x, 0.22 in y: legal under V(n).b1)
    // in a pad with 0.05 all round: every via has the metal running past on every side
    // (clean); the same pad with 0.045 all round: the four corner vias have two adjacent
    // short sides (fire), the eight edge vias one (clean); a row of four in a `w`-wide
    // line with 0.045 at both ends: the two end vias fire.
    let mut e = vec![rect(m, 1.95, 1.95, 3.68, 3.47)];
    e.extend(l.grid(2.0, 2.0, 4, 4, 0.29, 0.22));
    e.push(rect(m, 5.955, 1.955, 7.675, 3.465));
    e.extend(l.grid(6.0, 2.0, 4, 4, 0.29, 0.22));
    e.push(rect(m, 9.955, 5.0 - c, 11.465, 5.0 + VIA + c));
    e.extend(l.grid(10.0, 5.0, 4, 1, 0.22, 0.0));
    l.write(".c1.h10", e);
}
