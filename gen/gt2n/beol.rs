// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Routing patterns, M1-RDL and V1-V13, and the backside, BV0-BRDL. The odd metals
//! run along y and the even ones (and RDL) along x, on either side, and each metal's six rules read the same way on its own axes: `.1`
//! its width, `.2` its length, `.3` the space across its wires, `.4` the space between
//! their tips, `.5` the space corner to corner, `.6` a bend. So one builder draws every
//! metal, along `u` and across `v`, and turns the result onto the metal's axes.
//!
//! M1-M6 and V1-V5 are the runset's. Above them the rules are the tech LEF's, their ids
//! marked `.LEF`: M10-M13 have no `.2`, RDL only `.1` and `.3`, and a via has no rules
//! on its walls lying on its metals' outlines, only on its lying inside them, and its
//! `.5` space, read every way, in place of the runset's `.6` corner. The backside reads
//! as the LEF's frontside does, BPR being the runset's: BM1-BM4 have no `.2`, BRDL only
//! `.1` and `.3`.
//!
//! V1-V5 also take the LEF's cut-layer SPACING, which the runset numbers `.5` and leaves
//! unchecked, as `.5.LEF`, drawn as the LEF's vias' `.5` is.
//!
//! A via is flush with the metal running along y across x, and with the one running
//! along x across y, as V*.3 and V*.4 ask; each metal runs past it along its length,
//! as the two enclosure rules ask.

use super::{Corpus, DBU, ROOM, Rows, bx, corner_rise, pg};
use gds21::GdsElement;

const OFFSET: f64 = 5000.0;

pub(super) fn generate(c: &Corpus<'_>) {
    for (n, [w, len, s, t, k]) in [
        (1, [14.0, 28.0, 14.0, 20.0, 16.0]),
        (2, [12.0, 24.0, 12.0, 17.0, 15.0]),
        (3, [14.0, 28.0, 14.0, 20.0, 16.0]),
        (4, [21.0, 42.0, 21.0, 30.0, 24.0]),
        (5, [21.0, 42.0, 21.0, 30.0, 24.0]),
        (6, [38.0, 55.0, 38.0, 55.0, 45.0]),
    ] {
        metal(c, &Metal::front(n, w, Some(len), s, Some((t, k))));
    }
    // gt2_tech.lef: WIDTH, MINSIZE's length, SPACING, the EOLKEEPOUT's forward
    // extension and CORNERSPACING.
    for (n, w, len, s, t, k) in [
        (7, 38.0, Some(57.0), 38.0, 55.0, 45.0),
        (8, 38.0, Some(57.0), 38.0, 55.0, 45.0),
        (9, 38.0, Some(57.0), 38.0, 55.0, 45.0),
        (10, 56.0, None, 56.0, 80.0, 66.0),
        (11, 56.0, None, 56.0, 80.0, 66.0),
        (12, 360.0, None, 360.0, 450.0, 450.0),
        (13, 360.0, None, 360.0, 450.0, 450.0),
    ] {
        metal(c, &Metal::front(n, w, len, s, Some((t, k))));
    }
    metal(c, &Metal::front(14, 1600.0, None, 1600.0, None));
    // The backside, the same way: BM1 along y, as M1.
    for (n, w, s, t, k) in [
        (1, 56.0, 56.0, 80.0, 66.0),
        (2, 56.0, 56.0, 80.0, 66.0),
        (3, 360.0, 360.0, 450.0, 450.0),
        (4, 360.0, 360.0, 450.0, 450.0),
    ] {
        metal(
            c,
            &Metal::back(&format!("BM{n}"), n % 2 == 1, w, s, Some((t, k))),
        );
    }
    metal(c, &Metal::back("BRDL", true, 1600.0, 1600.0, None));
    for v in [
        Via::front(1, (14.0, 12.0), (4.0, 4.0), 18.0).lef_space(12.0),
        Via::front(2, (14.0, 12.0), (4.0, 4.0), 18.0).lef_space(12.0),
        Via::front(3, (14.0, 21.0), (4.0, 4.0), 18.0).lef_space(14.0),
        Via::front(4, (21.0, 21.0), (5.0, 5.0), 27.0).lef_space(21.0),
        Via::front(5, (21.0, 38.0), (6.0, 6.0), 27.0).lef_space(21.0),
        // gt2_tech.lef: the cut of each `V<n>_0` and `Via<n>Array`, the enclosures of
        // the metal along y and the one along x, and the cut layer's SPACING.
        Via::front(6, (38.0, 38.0), (8.0, 8.0), 38.0),
        Via::front(7, (38.0, 38.0), (8.0, 8.0), 38.0),
        Via::front(8, (38.0, 38.0), (8.0, 8.0), 38.0),
        Via::front(9, (38.0, 56.0), (10.0, 10.0), 38.0),
        Via::front(10, (56.0, 56.0), (12.0, 12.0), 56.0),
        Via::front(11, (56.0, 360.0), (12.0, 12.0), 56.0),
        Via::front(12, (360.0, 360.0), (20.0, 20.0), 56.0),
        Via::front(13, (360.0, 1600.0), (40.0, 200.0), 56.0),
    ] {
        via(c, &v);
    }
    // The cut of each `BV<n>_0` and `BVia<n>Array`, the enclosures - `BVia<n>Array`'s -
    // of the metal along y and the one along x, and the cut layer's SPACING.
    for v in [
        Via::back(0, ("BM1", "BPR"), false, (40.0, 32.0), (10.0, 10.0), 32.0),
        Via::back(1, ("BM1", "BM2"), true, (56.0, 56.0), (12.0, 12.0), 56.0),
        Via::back(2, ("BM3", "BM2"), false, (360.0, 56.0), (12.0, 12.0), 56.0),
        Via::back(3, ("BM3", "BM4"), true, (360.0, 360.0), (20.0, 20.0), 56.0),
        Via::back(
            4,
            ("BRDL", "BM4"),
            false,
            (1600.0, 360.0),
            (200.0, 40.0),
            56.0,
        ),
    ] {
        via(c, &v);
    }
}

/// The name of routing layer `n`: M`n`, and RDL above M13.
fn metal_name(n: u8) -> String {
    if n == 14 {
        "RDL".to_string()
    } else {
        format!("M{n}")
    }
}

/// The runset's rules stop at M6 and V5; the tech LEF's start above them.
fn from_lef(n: u8, via: bool) -> bool {
    if via { n >= 6 } else { n >= 7 }
}

/// A metal's frame: along `u`, across `v`, which for a metal running along y are y and x.
struct Frame {
    layer: (i16, i16),
    along_y: bool,
}

impl Frame {
    fn xy(&self, (u, v): (f64, f64)) -> (f64, f64) {
        if self.along_y { (v, u) } else { (u, v) }
    }

    /// A box `du` long and `dv` across, its corner at (u, v).
    fn bx(&self, u: f64, v: f64, du: f64, dv: f64) -> GdsElement {
        let (x, y) = self.xy((u, v));
        let (w, h) = self.xy((du, dv));
        bx(self.layer, x, y, w, h)
    }

    fn pg(&self, pts: &[(f64, f64)]) -> GdsElement {
        let pts: Vec<_> = pts.iter().map(|&p| self.xy(p)).collect();
        pg(self.layer, &pts)
    }
}

/// One metal level and its limits in nm.
struct Metal {
    name: String,
    along_y: bool,
    /// Whether its rules are the tech LEF's, their ids marked `.LEF`.
    lef: bool,
    w: f64,
    /// `.2`, which the LEF gives only M7-M9.
    len: Option<f64>,
    s: f64,
    /// `.4`'s tip space and `.5`'s corner space, which, with `.6`, RDL has none of.
    ends: Option<(f64, f64)>,
}

impl Metal {
    /// Frontside metal `n`, along y when odd.
    fn front(n: u8, w: f64, len: Option<f64>, s: f64, ends: Option<(f64, f64)>) -> Self {
        Metal {
            name: metal_name(n),
            along_y: n % 2 == 1,
            lef: from_lef(n, false),
            w,
            len,
            s,
            ends,
        }
    }

    /// A backside metal, all of them the LEF's, none with a length.
    fn back(name: &str, along_y: bool, w: f64, s: f64, ends: Option<(f64, f64)>) -> Self {
        Metal {
            name: name.to_string(),
            along_y,
            lef: true,
            w,
            len: None,
            s,
            ends,
        }
    }

    fn id(&self, r: u8) -> String {
        let lef = if self.lef { ".LEF" } else { "" };
        format!("{}.{r}{lef}", self.name)
    }
}

fn metal(c: &Corpus<'_>, m: &Metal) {
    let deck = m.name.to_lowercase();
    let w = c.src(&m.id(1), "min_width", m.w);
    let len = m.len.map(|len| c.src(&m.id(2), "min_width", len));
    let s = c.src(&m.id(3), "min_space", m.s);
    assert_eq!(c.src(&m.id(3), "min_notch", s), s);
    let ends = m.ends.map(|(t, k)| {
        let t = c.src(&m.id(4), "min_space", t);
        assert_eq!(c.src(&m.id(4), "min_notch", t), t);
        (t, c.src(&m.id(5), "min_space", k))
    });
    let f = Frame {
        layer: c.layer(&m.name),
        along_y: m.along_y,
    };
    // Long past the width and the tip space, and rows apart past every space.
    let long = 300.0_f64.max(5.0 * w);
    let widest = ends.map_or(s, |(t, k)| s.max(t).max(k));
    let rows = || Rows::spaced(OFFSET, ROOM.max(2.0 * widest));
    let write =
        |r: u8, case: &str, elems| c.write(&deck, &format!("{}.{case}", m.id(r)), true, elems);

    let mut rs = rows();
    let elems = [w, w - DBU]
        .map(|ww| f.bx(OFFSET, rs.next(ww), long, ww))
        .to_vec();
    write(1, "narrow", elems);

    if let Some(len) = len {
        let mut rs = rows();
        let elems = [len, len - DBU]
            .map(|l| f.bx(OFFSET, rs.next(w), l, w))
            .to_vec();
        write(2, "short", elems);
    }

    let mut rs = rows();
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let v = rs.next(2.0 * w + gap);
        elems.push(f.bx(OFFSET, v, long, w));
        elems.push(f.bx(OFFSET, v + w + gap, long, w));
    }
    write(3, "close", elems);

    // A U: two wires `gap` apart, joined at one end by a bar as long as `.2` asks, or
    // twice the width where nothing does.
    let bar = len.unwrap_or(2.0 * w);
    let mut rs = rows();
    let mut elems = vec![];
    for gap in [s, s - DBU] {
        let (u, v) = (OFFSET, rs.next(2.0 * w + gap));
        let top = v + 2.0 * w + gap;
        elems.push(f.pg(&[
            (u, v),
            (u + long, v),
            (u + long, v + w),
            (u + bar, v + w),
            (u + bar, top - w),
            (u + long, top - w),
            (u + long, top),
            (u, top),
        ]));
    }
    write(3, "notch", elems);

    let Some((t, k)) = ends else {
        return;
    };

    let mut rs = rows();
    let mut elems = vec![];
    for gap in [t, t - DBU] {
        let v = rs.next(w);
        elems.push(f.bx(OFFSET, v, long, w));
        elems.push(f.bx(OFFSET + long + gap, v, long, w));
    }
    write(4, "close", elems);

    // Two wires tip to tip, joined by a third along their sides: a slot `gap` long.
    let mut rs = rows();
    let mut elems = vec![];
    for gap in [t, t - DBU] {
        let (u, v) = (OFFSET, rs.next(2.0 * w));
        let u1 = u + long + gap;
        elems.push(f.pg(&[
            (u, v),
            (u + long, v),
            (u + long, v + w),
            (u1, v + w),
            (u1, v),
            (u1 + long, v),
            (u1 + long, v + 2.0 * w),
            (u, v + 2.0 * w),
        ]));
    }
    write(4, "notch", elems);

    // Two wires, the second past the first's tip along and beside it across, so only
    // their corners face: `du` along, then the least rise across that clears `.5`.
    let du = (k * 0.6 / DBU).round() * DBU;
    let dv = corner_rise(k, du);
    let mut rs = rows();
    let mut elems = vec![];
    for rise in [dv, dv - DBU] {
        let v = rs.next(2.0 * w + rise);
        elems.push(f.bx(OFFSET, v, long, w));
        elems.push(f.bx(OFFSET + long + du, v + w + rise, long, w));
    }
    write(5, "corner", elems);

    // A wire, then one jogged a DBU across half way along.
    let mut rs = rows();
    let mut elems = vec![f.bx(OFFSET, rs.next(w), long, w)];
    let (u, v) = (OFFSET, rs.next(w + DBU));
    let mid = u + long / 2.0;
    elems.push(f.pg(&[
        (u, v),
        (mid, v),
        (mid, v + DBU),
        (u + long, v + DBU),
        (u + long, v + w + DBU),
        (mid, v + w + DBU),
        (mid, v + w),
        (u, v + w),
    ]));
    write(6, "bent", elems);
}

/// One via level: its cut, and the metal along y and the one along x it joins.
struct Via {
    cut: String,
    my: String,
    mx: String,
    /// Whether the metal along y is the nearer the devices, `.7`'s; `.8` reads the other.
    near_y: bool,
    lef: bool,
    /// The via's least width across x and height across y.
    size: (f64, f64),
    /// What the metal along y and the metal along x run past it along their lengths.
    en: (f64, f64),
    /// The runset's `.6` corner space, or the LEF's `.5` space read every way.
    space: f64,
    /// For the runset's vias, the LEF's cut-layer SPACING, `.5.LEF`.
    lef_space: Option<f64>,
}

impl Via {
    /// Vn, from Mn to Mn+1, the odd one along y.
    fn front(n: u8, size: (f64, f64), en: (f64, f64), space: f64) -> Self {
        let (my, mx) = if n % 2 == 1 { (n, n + 1) } else { (n + 1, n) };
        Via {
            cut: format!("V{n}"),
            my: metal_name(my),
            mx: metal_name(mx),
            near_y: my == n,
            lef: from_lef(n, true),
            size,
            en,
            space,
            lef_space: None,
        }
    }

    /// BVn, joining the metals `(my, mx)` along y and x, all of them the LEF's.
    fn back(
        n: u8,
        (my, mx): (&str, &str),
        near_y: bool,
        size: (f64, f64),
        en: (f64, f64),
        space: f64,
    ) -> Self {
        Via {
            cut: format!("BV{n}"),
            my: my.to_string(),
            mx: mx.to_string(),
            near_y,
            lef: true,
            size,
            en,
            space,
            lef_space: None,
        }
    }

    /// The LEF's cut-layer SPACING, on top of the runset's rules.
    fn lef_space(self, s: f64) -> Self {
        Via {
            lef_space: Some(s),
            ..self
        }
    }

    fn id(&self, r: &str) -> String {
        let lef = if self.lef { ".LEF" } else { "" };
        format!("{}.{r}{lef}", self.cut)
    }
}

fn via(c: &Corpus<'_>, v: &Via) {
    let deck = v.cut.to_lowercase();
    let (ny, nx) = (&v.my, &v.mx);
    let wx = c.src(&v.id("1"), "min_width", v.size.0);
    let wy = c.src(&v.id("2"), "min_width", v.size.1);
    let en_y = c.src(&v.id(&format!("{ny}.EN")), "min_enclosure", v.en.0);
    let en_x = c.src(&v.id(&format!("{nx}.EN")), "min_enclosure", v.en.1);
    let space_rule = if v.lef { "5" } else { "6" };
    let k = c.src(&v.id(space_rule), "min_space", v.space);
    // The `.5` space read every way: the LEF's own, or its SPACING on a runset via.
    let close = if v.lef {
        Some((v.id("5"), k))
    } else {
        v.lef_space.map(|s| {
            let id = format!("{}.5.LEF", v.cut);
            let k = c.src(&id, "min_space", s);
            (id, k)
        })
    };
    let (cut, ly, lx) = (c.layer(&v.cut), c.layer(ny), c.layer(nx));
    // Past both enclosures, and 20 nm where they are smaller.
    let run = 20.0_f64.max(en_y).max(en_x);
    let tall = wy + 2.0 * run;
    // A `w`×`h` via at (x, y), the metal along y flush across x and `b` and `t` past it,
    // the metal along x flush across y and `l` and `r` past it; `None` leaves one out.
    let stack = |x: f64,
                 y: f64,
                 (w, h): (f64, f64),
                 along_y: Option<(f64, f64)>,
                 along_x: Option<(f64, f64)>| {
        let mut e = vec![bx(cut, x, y, w, h)];
        if let Some((b, t)) = along_y {
            e.push(bx(ly, x, y - b, w, b + h + t));
        }
        if let Some((l, r)) = along_x {
            e.push(bx(lx, x - l, y, l + w + r, h));
        }
        e
    };
    let full = Some((run, run));
    let ok = |x: f64, y: f64| stack(x, y, (wx, wy), full, full);
    let write =
        |r: &str, case: &str, elems| c.write(&deck, &format!("{}.{case}", v.id(r)), true, elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [wx, wx - DBU]
        .into_iter()
        .flat_map(|w| stack(OFFSET, rows.next(tall), (w, wy), full, full))
        .collect();
    write("1", "narrow", elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [wy, wy - DBU]
        .into_iter()
        .flat_map(|h| stack(OFFSET, rows.next(tall), (wx, h), full, full))
        .collect();
    write("2", "narrow", elems);

    if let Some((id, k)) = close {
        // Two vias side by side across x, each in its own wire along y; the wires along
        // x, run out past the gap, merge.
        let mut rows = Rows::new(OFFSET);
        let mut elems = vec![];
        for gap in [k, k - DBU] {
            let y = rows.next(tall);
            elems.extend(ok(OFFSET, y));
            elems.extend(ok(OFFSET + wx + gap, y));
        }
        c.write(&deck, &format!("{id}.close"), true, elems);
    }
    if !v.lef {
        // A via whose metal along y sits a DBU to the right: one vertical wall lies
        // outside it and the other inside it, both off its outline; then one whose metal
        // is a DBU wider each side, both walls inside it.
        let mut rows = Rows::new(OFFSET);
        let mut elems = ok(OFFSET, rows.next(tall));
        let y = rows.next(tall);
        elems.extend(stack(OFFSET, y, (wx, wy), None, full));
        elems.push(bx(ly, OFFSET + DBU, y - run, wx, tall));
        let y = rows.next(tall);
        elems.extend(stack(OFFSET, y, (wx, wy), None, full));
        elems.push(bx(ly, OFFSET - DBU, y - run, wx + 2.0 * DBU, tall));
        write("3", "align", elems);

        // The same across y for the metal along x.
        let mut rows = Rows::new(OFFSET);
        let mut elems = ok(OFFSET, rows.next(tall));
        let y = rows.next(tall);
        elems.extend(stack(OFFSET, y, (wx, wy), full, None));
        elems.push(bx(lx, OFFSET - run, y + DBU, wx + 2.0 * run, wy));
        let y = rows.next(tall);
        elems.extend(stack(OFFSET, y, (wx, wy), full, None));
        elems.push(bx(
            lx,
            OFFSET - run,
            y - DBU,
            wx + 2.0 * run,
            wy + 2.0 * DBU,
        ));
        write("4", "align", elems);

        // Two vias corner to corner, each in its own wires, run out to the enclosures.
        let dx = (k * 0.6 / DBU).round() * DBU;
        let dy = corner_rise(k, dx);
        let mut rows = Rows::new(OFFSET);
        let mut elems = vec![];
        let tight = (Some((en_y, en_y)), Some((en_x, en_x)));
        for rise in [dy, dy - DBU] {
            let y = rows.next(2.0 * wy + rise);
            elems.extend(stack(OFFSET, y, (wx, wy), tight.0, tight.1));
            elems.extend(stack(
                OFFSET + wx + dx,
                y + wy + rise,
                (wx, wy),
                tight.0,
                tight.1,
            ));
        }
        write("6", "corner", elems);
    }

    // `.7` and `.8`: a via with its lower (upper) metal missing, then with it abutting
    // the via's side - the left side for the metal along y, the bottom for the other -
    // which shares no area with it. The LEF's rules ask the via inside the metal, so
    // for them a third via has the metal a DBU aside, one wall out of it.
    for (r, near) in [("7", true), ("8", false)] {
        let mut rows = Rows::new(OFFSET);
        let mut elems = ok(OFFSET, rows.next(tall));
        let y = rows.next(tall);
        let y2 = rows.next(tall);
        if near == v.near_y {
            elems.extend(stack(OFFSET, y, (wx, wy), None, full));
            elems.extend(stack(OFFSET, y2, (wx, wy), None, full));
            elems.push(bx(ly, OFFSET - wx, y2 - run, wx, tall));
            if v.lef {
                let y3 = rows.next(tall);
                elems.extend(stack(OFFSET, y3, (wx, wy), None, full));
                elems.push(bx(ly, OFFSET + DBU, y3 - run, wx, tall));
            }
        } else {
            elems.extend(stack(OFFSET, y, (wx, wy), full, None));
            elems.extend(stack(OFFSET, y2, (wx, wy), full, None));
            elems.push(bx(lx, OFFSET - run, y2 - wy, wx + 2.0 * run, wy));
            if v.lef {
                let y3 = rows.next(tall);
                elems.extend(stack(OFFSET, y3, (wx, wy), full, None));
                elems.push(bx(lx, OFFSET - run, y3 + DBU, wx + 2.0 * run, wy));
            }
        }
        write(r, "off", elems);
    }

    let mut rows = Rows::new(OFFSET);
    let elems = [(en_y, en_y), (en_y - DBU, en_y), (en_y, en_y - DBU)]
        .into_iter()
        .flat_map(|bt| stack(OFFSET, rows.next(tall), (wx, wy), Some(bt), full))
        .collect();
    write(&format!("{ny}.EN"), "enc", elems);

    let mut rows = Rows::new(OFFSET);
    let elems = [(en_x, en_x), (en_x - DBU, en_x), (en_x, en_x - DBU)]
        .into_iter()
        .flat_map(|lr| stack(OFFSET, rows.next(tall), (wx, wy), full, Some(lr)))
        .collect();
    write(&format!("{nx}.EN"), "enc", elems);
}
