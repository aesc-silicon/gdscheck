// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Via patterns for V0-V9 (DRM 3.11, 3.13, 3.15, 3.17, 3.19). Every via sits in a
//! whole stack - the layer it lands on and the one over it, each giving it the
//! enclosure its rules ask - so a pattern's violations all belong to the rule it was
//! drawn for. Each pattern leads with the via or gap exactly at the limit, then one
//! DBU past it.
//!
//! Each level also draws its spacing and enclosure defects under the SRAM marker,
//! where convention 7 exempts a via that shares area with it but keeps its landing
//! metal whole.

use super::patterns::{ROOM, bx, pg};
use super::{Corpus, DBU};
use gds21::GdsElement;

const OFFSET: f64 = 6990.0;
/// How far a landing layer runs past the via along its length, beyond the enclosure.
const RUN: f64 = 30.0;

/// What one side of a via asks of the layer around it.
#[derive(Clone, Copy)]
enum Sides {
    /// At least the value on some side.
    Any,
    /// At least the value on both sides of one pair.
    Opposite,
    /// The value on one side and `.0` on the side opposite it (V1.M1.EN.1).
    OneAndOpposite(f64),
}

/// An enclosure rule: its id, the DRM value and how it reads.
#[derive(Clone, Copy)]
struct Enc {
    id: &'static str,
    value: f64,
    sides: Sides,
}

/// One level of the stack.
struct Level {
    n: u8,
    /// The via's side; V4-V9 are exact squares, V0-V3 at least this.
    side: f64,
    below: &'static str,
    above: &'static str,
    en_below: Enc,
    en_above: Enc,
    /// The upper layer must be exactly as wide as the via across its length
    /// (V0.M1.AUX.3, V1-V7 AUX.2): flush on one pair of sides.
    flush: Option<&'static str>,
}

impl Level {
    fn deck(&self) -> String {
        format!("v{}", self.n)
    }

    fn id(&self, r: &str) -> String {
        format!("V{}.{r}", self.n)
    }

    /// The via `w`×`h` at (x, y) with the landing and upper layers around it, the
    /// lower one `below` = (left, right, bottom, top) past it and the upper one
    /// `above` past it - `None` leaving that layer out.
    fn with(
        &self,
        c: &Corpus<'_>,
        (x, y, w, h): (f64, f64, f64, f64),
        below: Option<[f64; 4]>,
        above: Option<[f64; 4]>,
    ) -> Vec<GdsElement> {
        let mut out = vec![bx(c.layer(&format!("V{}", self.n)), x, y, w, h)];
        for (layer, m) in [(self.below, below), (self.above, above)] {
            if let Some([l, r, b, t]) = m {
                out.push(bx(c.layer(layer), x - l, y - b, w + l + r, h + b + t));
            }
        }
        out
    }

    /// The margins a layer gives a via when its rule is met exactly: the lower layer
    /// all round, running on along y; the upper layer along x and, where it must be,
    /// flush across y.
    fn below_ok(&self) -> [f64; 4] {
        let e = self.en_below.value;
        [e, e, e + RUN, e + RUN]
    }

    fn above_ok(&self) -> [f64; 4] {
        let e = self.en_above.value;
        let across = if self.flush.is_some() { 0.0 } else { e };
        [e + RUN, e + RUN, across, across]
    }

    /// A whole stack around a `w`×`h` via at (x, y).
    fn stack(&self, c: &Corpus<'_>, x: f64, y: f64, w: f64, h: f64) -> Vec<GdsElement> {
        self.with(
            c,
            (x, y, w, h),
            Some(self.below_ok()),
            Some(self.above_ok()),
        )
    }
}

fn levels(c: &Corpus<'_>) -> Vec<Level> {
    // Each value read once from the DRM and checked against the deck.
    let enc = |id: &'static str, value: f64, sides: Sides| Enc {
        id,
        value: c.drm(id, "min_enclosure", value),
        sides,
    };
    let five_two = c.drm_param("V1.M1.EN.1", "min_enclosure", "opposite_value", 2.0);
    use Sides::*;
    let lv = |n, side, below, above, en_below, en_above, flush| Level {
        n,
        side,
        below,
        above,
        en_below,
        en_above,
        flush,
    };
    vec![
        lv(
            0,
            18.0,
            "LISD",
            "M1",
            enc("V0.LISD.EN.2", 3.0, Opposite),
            enc("V0.M1.EN.1", 5.0, Any),
            Some("V0.M1.AUX.3"),
        ),
        lv(
            1,
            18.0,
            "M1",
            "M2",
            enc("V1.M1.EN.1", 5.0, OneAndOpposite(five_two)),
            enc("V1.M2.EN.2", 5.0, Any),
            Some("V1.M2.AUX.2"),
        ),
        lv(
            2,
            18.0,
            "M2",
            "M3",
            enc("V2.M2.EN.1", 5.0, Opposite),
            enc("V2.M3.EN.2", 5.0, Any),
            Some("V2.M3.AUX.2"),
        ),
        lv(
            3,
            18.0,
            "M3",
            "M4",
            enc("V3.M3.EN.1", 5.0, Opposite),
            enc("V3.M4.EN.2", 11.0, Opposite),
            Some("V3.M4.AUX.2"),
        ),
        lv(
            4,
            24.0,
            "M4",
            "M5",
            enc("V4.M4.EN.1", 11.0, Opposite),
            enc("V4.M5.EN.2", 11.0, Opposite),
            Some("V4.M5.AUX.2"),
        ),
        lv(
            5,
            24.0,
            "M5",
            "M6",
            enc("V5.M5.EN.1", 11.0, Opposite),
            enc("V5.M6.EN.2", 11.0, Opposite),
            Some("V5.M6.AUX.2"),
        ),
        lv(
            6,
            32.0,
            "M6",
            "M7",
            enc("V6.M6.EN.1", 11.0, Opposite),
            enc("V6.M7.EN.2", 11.0, Opposite),
            Some("V6.M7.AUX.2"),
        ),
        lv(
            7,
            32.0,
            "M7",
            "M8",
            enc("V7.M7.EN.1", 11.0, Opposite),
            enc("V7.M8.EN.2", 11.0, Opposite),
            Some("V7.M8.AUX.2"),
        ),
        lv(
            8,
            40.0,
            "M8",
            "M9",
            enc("V8.M8.EN.1", 20.0, Opposite),
            enc("V8.M9.EN.2", 20.0, Opposite),
            None,
        ),
        lv(
            9,
            40.0,
            "M9",
            "PAD",
            enc("V9.M9.EN.1", 20.0, Opposite),
            enc("V9.PAD.EN.2", 20.0, Opposite),
            None,
        ),
    ]
}

pub(super) fn generate(c: &Corpus<'_>) {
    for lv in levels(c) {
        width(c, &lv);
        spacing(c, &lv);
        enclosure(c, &lv);
        cover(c, &lv);
        sram(c, &lv);
        if lv.n <= 3 {
            adjacent(c, &lv);
            corner(c, &lv);
        }
    }
    v0_on_lig(c);
    v0_partly_on_lisd(c);
}

/// Stacks of each `(w, h)` via in `sizes`, one above another.
fn stacks(c: &Corpus<'_>, lv: &Level, sizes: &[(f64, f64)]) -> Vec<GdsElement> {
    let mut y = OFFSET;
    let mut out = vec![];
    for &(w, h) in sizes {
        out.extend(lv.stack(c, OFFSET, y, w, h));
        y += h + 2.0 * (lv.en_below.value + RUN) + ROOM;
    }
    out
}

/// DRM W.1: V0-V3 at least 18 nm, V4-V6 exactly their side along the upper metal,
/// V7 exactly its side across its short side, V8/V9 exactly 40 nm
/// across and 40 or 120 nm long, and a rectangle. The V8/V9 L fails twice: it is not
/// a rectangle, and its box is 120 nm across.
fn width(c: &Corpus<'_>, lv: &Level) {
    let s = lv.side;
    let (sizes, check): (Vec<(f64, f64)>, _) = match lv.n {
        0..=3 => (vec![(s, s), (s - DBU, s), (s, s - DBU)], "min_width"),
        // Exact along the upper metal - y on V4 and V6, x on V5 - and free across it:
        // a legal via three tracks across, then one a DBU short and one a DBU long along.
        4..=6 => {
            let along = |a: f64, across: f64| if lv.n == 5 { (a, across) } else { (across, a) };
            let sizes = vec![
                along(s, s),
                along(s, 3.0 * s),
                along(s - DBU, s),
                along(s + DBU, s),
            ];
            (sizes, "forbidden")
        }
        // V7 reads the short side: the manual never says which way M8 runs.
        7 => (vec![(s, s), (s - DBU, s), (s, s - DBU)], "exact_dim"),
        _ => (
            vec![
                (s, s),
                (s, 3.0 * s),
                (3.0 * s, s),
                (s - DBU, s),
                (s, 2.0 * s),
            ],
            "exact_dim",
        ),
    };
    if check == "forbidden" {
        c.drm_edge_length(&format!("V{}OffWidth", lv.n), s);
    } else {
        c.drm(&lv.id("W.1"), check, s);
    }
    let mut elems = stacks(c, lv, &sizes);
    if lv.n >= 8 {
        // An L of 40 nm arms, in a stack around its box.
        let (x, y) = (OFFSET + 1000.0, OFFSET);
        let l = c.layer(&format!("V{}", lv.n));
        let a = 3.0 * s;
        elems.push(pg(
            l,
            &[
                (x, y),
                (x + a, y),
                (x + a, y + s),
                (x + s, y + s),
                (x + s, y + a),
                (x, y + a),
            ],
        ));
        let e = lv.en_below.value;
        elems.push(bx(
            c.layer(lv.below),
            x - e,
            y - e,
            a + 2.0 * e,
            a + 2.0 * e,
        ));
        let e = lv.en_above.value;
        elems.push(bx(
            c.layer(lv.above),
            x - e,
            y - e,
            a + 2.0 * e,
            a + 2.0 * e,
        ));
    }
    c.write(&lv.deck(), &lv.id("W.1.narrow"), true, elems);
}

/// DRM S.1 (V0-V3, V8/V9) or S.2 (V4-V7): two vias side by side, then corner to
/// corner. On V0-V3 the corner reading is S.2-S.4, drawn by [`corner`].
/// The rule a level's side-by-side spacing is read under, and its DRM value.
fn spacing_rule(lv: &Level) -> (&'static str, f64) {
    match lv.n {
        0..=3 => ("S.1", 18.0),
        4 | 5 => ("S.2", 33.0),
        6 | 7 => ("S.2", 45.0),
        _ => ("S.1", 57.0),
    }
}

/// (left, right, bottom, top) margins that leave `en` one DBU short, the layer flush
/// across so that only the pair along x is read.
fn one_short(en: Enc) -> [f64; 4] {
    let e = en.value;
    match en.sides {
        Sides::Any => [e - DBU, e - DBU, 0.0, 0.0],
        Sides::Opposite => [e - DBU, e, 0.0, 0.0],
        Sides::OneAndOpposite(o) => [e, o - DBU, 0.0, 0.0],
    }
}

fn spacing(c: &Corpus<'_>, lv: &Level) {
    let (r, value) = spacing_rule(lv);
    let limit = c.drm(&lv.id(r), "min_space", value);
    let elems = spaced(c, lv, limit);
    c.write(&lv.deck(), &lv.id(&format!("{r}.close")), true, elems);
}

/// The `openroad` deck's V6.S.2.LEF: [`spacing`]'s pairs at the tech LEF's 34 nm.
pub(super) fn lef_spacing(c: &Corpus<'_>) {
    let lv = levels(c).into_iter().find(|lv| lv.n == 6).unwrap();
    let limit = c.lef("V6.S.2.LEF", "min_space", 34.0);
    c.write("openroad", "V6.S.2.LEF.close", true, spaced(c, &lv, limit));
}

/// Two vias side by side `limit` apart and a DBU under it, then, on V4-V9, two
/// corner to corner likewise.
fn spaced(c: &Corpus<'_>, lv: &Level, limit: f64) -> Vec<GdsElement> {
    let s = lv.side;
    let pitch = s + 2.0 * (lv.en_below.value + RUN) + ROOM;
    let mut elems = vec![];
    let mut y = OFFSET;
    for gap in [limit, limit - DBU] {
        elems.extend(lv.stack(c, OFFSET, y, s, s));
        elems.extend(lv.stack(c, OFFSET + s + gap, y, s, s));
        y += pitch;
    }
    // V4-V9 read the corner under the same rule: the vias `s` apart vertically, so
    // the gap is first met at sqrt(limit^2 - s^2) across, rounded up to the grid.
    if lv.n >= 4 {
        let dy = s;
        let dx = ((limit * limit - dy * dy).sqrt() / DBU).ceil() * DBU;
        for dx in [dx, dx - DBU] {
            elems.extend(lv.stack(c, OFFSET, y, s, s));
            elems.extend(lv.stack(c, OFFSET + s + dx, y + s + dy, s, s));
            y += pitch + s + dy;
        }
    }
    elems
}

/// DRM 3.11/3.13 S.1 on neighbouring tracks: the upper via shifted half a via along
/// the track, so the two still overlap in projection and face each other across the
/// gap between the tracks. Out of line altogether they meet only corner to corner,
/// which is S.2-S.4's reading, drawn by [`corner`].
fn adjacent(c: &Corpus<'_>, lv: &Level) {
    let limit = c.drm(&lv.id("S.1"), "min_space", 18.0);
    let s = lv.side;
    let pitch = 2.0 * s + 2.0 * (lv.en_below.value + RUN) + ROOM;
    let mut elems = vec![];
    let mut y = OFFSET;
    for gap in [limit, limit - DBU] {
        elems.extend(lv.stack(c, OFFSET, y, s, s));
        elems.extend(lv.stack(c, OFFSET + s / 2.0, y + s + gap, s, s));
        y += pitch + gap;
    }
    c.write(&lv.deck(), &lv.id("S.1.adjacent"), true, elems);
}

/// Each enclosure one DBU short, the other layer whole; and an upper layer that must
/// be flush across overhanging there.
fn enclosure(c: &Corpus<'_>, lv: &Level) {
    let s = lv.side;
    let pitch = s + 2.0 * (lv.en_below.value.max(lv.en_above.value) + RUN) + ROOM;
    for (below, en) in [(true, lv.en_below), (false, lv.en_above)] {
        let e = en.value;
        // (left, right, bottom, top), the clean ones first. The upper layer stays
        // flush across, the lower layer is drawn flush across too so that only the
        // pair along x is read.
        let margins: Vec<[f64; 4]> = match en.sides {
            Sides::Any => vec![[e, 0.0, 0.0, 0.0], [e - DBU, e - DBU, 0.0, 0.0]],
            Sides::Opposite => vec![[e, e, 0.0, 0.0], [e - DBU, e, 0.0, 0.0]],
            // V1.M1.EN.1: 5 and 2 on one pair, either way round; then split across
            // two pairs, and each value one DBU short.
            Sides::OneAndOpposite(o) => vec![
                [e, o, 0.0, 0.0],
                [e + DBU, o + DBU, 0.0, 0.0],
                [0.0, 0.0, o, e],
                [e, 0.0, o, o],
                [e, o - DBU, 0.0, 0.0],
                [e - DBU, o, 0.0, 0.0],
            ],
        };
        let mut elems = vec![];
        let mut y = OFFSET;
        for m in margins {
            let at = (OFFSET, y, s, s);
            elems.extend(if below {
                lv.with(c, at, Some(m), Some(lv.above_ok()))
            } else {
                lv.with(c, at, Some(lv.below_ok()), Some(m))
            });
            y += pitch;
        }
        c.write(&lv.deck(), &format!("{}.short", en.id), true, elems);
    }
    if let Some(id) = lv.flush {
        let e = lv.en_above.value;
        let mut elems = lv.stack(c, OFFSET, OFFSET, s, s);
        let over = [e + RUN, e + RUN, e, e];
        elems.extend(lv.with(
            c,
            (OFFSET, OFFSET + pitch, s, s),
            Some(lv.below_ok()),
            Some(over),
        ));
        c.write(&lv.deck(), &format!("{id}.overhang"), true, elems);
    }
}

/// DRM AUX.1: a via must lie on both layers. A whole stack, then one missing its
/// lower layer, then one missing its upper.
fn cover(c: &Corpus<'_>, lv: &Level) {
    let s = lv.side;
    let pitch = s + 2.0 * (lv.en_below.value + RUN) + ROOM;
    let mut elems = lv.stack(c, OFFSET, OFFSET, s, s);
    elems.extend(lv.with(c, (OFFSET, OFFSET + pitch, s, s), None, Some(lv.above_ok())));
    elems.extend(lv.with(
        c,
        (OFFSET, OFFSET + 2.0 * pitch, s, s),
        Some(lv.below_ok()),
        None,
    ));
    c.write(&lv.deck(), &lv.id("AUX.1.uncovered"), true, elems);
}

/// DRM 3.11/3.13 S.2-S.4: the corner spacing reads the upper layer's end-caps at the
/// two corners forming the gap - 5 nm or more at both, at one, or at neither. The
/// vias sit 18 nm apart vertically, so a corner-to-corner limit L is first met at a
/// horizontal gap of sqrt(L^2 - 18^2), rounded up to the grid. Each via also has a
/// 5 nm end-cap at its far end, which must not count.
fn corner(c: &Corpus<'_>, lv: &Level) {
    const ENDCAP: f64 = 5.0;
    let s = lv.side;
    let both = c.drm(&lv.id("S.2"), "min_space", 23.0);
    let neither = c.drm(&lv.id("S.3"), "min_space", 30.0);
    let one = c.drm(&lv.id("S.4"), "min_space", 27.0);
    let gaps = |limit: f64| {
        let good = ((limit * limit - s * s).sqrt() / DBU).ceil() * DBU;
        [good, good - DBU]
    };
    for (caps, a, b, r, limit) in [
        ("both", ENDCAP, ENDCAP, "S.2", both),
        ("neither", 0.0, 0.0, "S.3", neither),
        ("one", ENDCAP, 0.0, "S.4", one),
        ("under", ENDCAP - DBU, ENDCAP - DBU, "S.3", neither),
    ] {
        let mut elems = vec![];
        let mut y = OFFSET;
        for gap in gaps(limit) {
            let x = OFFSET + s + gap;
            let lo = lv.below_ok();
            elems.extend(lv.with(c, (OFFSET, y, s, s), Some(lo), Some([ENDCAP, a, 0.0, 0.0])));
            elems.extend(lv.with(
                c,
                (x, y + 2.0 * s, s, s),
                Some(lo),
                Some([b, ENDCAP, 0.0, 0.0]),
            ));
            y += 3.0 * s + 2.0 * (lv.en_below.value + RUN) + ROOM;
        }
        c.write(&lv.deck(), &lv.id(&format!("{r}.{caps}")), true, elems);
    }
}

/// DRM 3.11: a V0 on LIG and off LISD overhangs the LIG by 1 nm on two opposite
/// sides, shares at least 288 nm² with it - an 18 nm V0 over a 16 nm LIG - and
/// crosses it, the LIG running on past it both ways.
fn v0_on_lig(c: &Corpus<'_>) {
    let en = c.drm("V0.LIG.EN.4", "min_enclosure", 1.0);
    let area = c.drm_area("V0.LIG.A.1", "min_area", 288.0);
    let (v0, lig, m1) = (c.layer("V0"), c.layer("LIG"), c.layer("M1"));
    let s = 18.0;
    assert_eq!((s - 2.0 * en) * s, area);
    let on = |y: f64, l: f64, r: f64| {
        let m = 5.0;
        vec![
            bx(v0, OFFSET, y, s, s),
            bx(lig, OFFSET + l, y - RUN, s - l - r, s + 2.0 * RUN),
            bx(m1, OFFSET - m - RUN, y, s + 2.0 * (m + RUN), s),
        ]
    };
    let pitch = s + 2.0 * RUN + ROOM;
    for (r, sides) in [
        ("V0.LIG.EN.4", [(en, en), (en - DBU, en)]),
        ("V0.LIG.A.1", [(en, en), (en, en + DBU)]),
    ] {
        let mut elems = vec![];
        for (k, (l, rr)) in sides.into_iter().enumerate() {
            elems.extend(on(OFFSET + k as f64 * pitch, l, rr));
        }
        let variant = if r.ends_with("A.1") { "small" } else { "short" };
        c.write("v0", &format!("{r}.{variant}"), true, elems);
    }

    // V0.LIG.AUX.2, figure 3.11.2(c): along one LIG running up the page, a V0 crossing
    // it, then one flush with its side, one with a side inside it, and one over its end.
    let (lo, hi) = (en, s - en); // the LIG across, under a V0 at 0..s
    let pitch = s + 2.0 * RUN;
    let shifts = [0.0, en, en + DBU, 0.0];
    let mut elems = vec![];
    for (k, dx) in shifts.into_iter().enumerate() {
        let (x, y) = (OFFSET + dx, OFFSET + k as f64 * pitch);
        elems.push(bx(v0, x, y, s, s));
        elems.push(bx(m1, x - 5.0 - RUN, y, s + 2.0 * (5.0 + RUN), s));
    }
    // The LIG ends half way up the last V0.
    let top = OFFSET + 3.0 * pitch + s / 2.0;
    elems.push(bx(
        lig,
        OFFSET + lo,
        OFFSET - RUN,
        hi - lo,
        top - OFFSET + RUN,
    ));
    c.write("v0", "V0.LIG.AUX.2.uncrossed", true, elems);
}

/// DRM 1.2.2 convention 7 under the SRAM marker: the same defect four times - a pair
/// one DBU too close, or a via one DBU short of each enclosure - with the marker over
/// the via (over both, for a pair), over one via of the pair, touching the via along
/// an edge only, and over the landing layer away from the via. A via sharing area
/// with the marker is exempt, and a pair is read only when both vias are outside it;
/// edge contact is not membership, and the landing layer stays whole.
fn sram(c: &Corpus<'_>, lv: &Level) {
    let s = lv.side;
    let marker = c.layer("SRAMDRC");
    let reach = lv.en_below.value.max(lv.en_above.value) + RUN;
    let pitch = s + 2.0 * reach + ROOM;
    // The marker for case `k`, around a via at (x, y): over it, over its half nearest
    // `x`, along its left edge, and over a landing layer clear of it - the lower one
    // where it runs on below the via, else the far end of the upper one.
    let mark = |k: usize, x: f64, y: f64, w: f64, on_upper: bool| match k {
        0 => bx(marker, x - 1.0, y - 1.0, w + 2.0, s + 2.0),
        1 => bx(marker, x - 1.0, y - 1.0, s / 2.0 + 1.0, s + 2.0),
        2 => bx(marker, x - 10.0, y, 10.0, s),
        _ if on_upper => bx(marker, x + s + lv.en_above.value + RUN - 5.0, y, 5.0, s),
        _ => bx(marker, x, y - lv.en_below.value - RUN, s, 5.0),
    };

    let (r, value) = spacing_rule(lv);
    let gap = c.drm(&lv.id(r), "min_space", value) - DBU;
    let mut elems = vec![];
    for k in 0..4 {
        let y = OFFSET + k as f64 * pitch;
        elems.extend(lv.stack(c, OFFSET, y, s, s));
        elems.extend(lv.stack(c, OFFSET + s + gap, y, s, s));
        elems.push(mark(
            k,
            OFFSET,
            y,
            if k == 0 { 2.0 * s + gap } else { s },
            false,
        ));
    }
    c.write(&lv.deck(), &lv.id(&format!("{r}.sram")), true, elems);

    for (below, en) in [(true, lv.en_below), (false, lv.en_above)] {
        let mut elems = vec![];
        for k in [0, 2, 3] {
            let y = OFFSET + k as f64 * pitch;
            let at = (OFFSET, y, s, s);
            elems.extend(if below {
                lv.with(c, at, Some(one_short(en)), Some(lv.above_ok()))
            } else {
                lv.with(c, at, Some(lv.below_ok()), Some(one_short(en)))
            });
            elems.push(mark(k, OFFSET, y, s, below));
        }
        c.write(&lv.deck(), &format!("{}.sram", en.id), true, elems);
    }
}

/// DRM 3.11 V0.LISD.EN.3, figure 3.11.1(d): a V0 over a LIG may run off the end of a
/// LISD, but the part on the LISD needs 3 nm of it on both sides across. Margins of 3
/// and 3.25 nm, then each side a DBU short.
fn v0_partly_on_lisd(c: &Corpus<'_>) {
    let en = c.drm("V0.LISD.EN.3", "min_enclosure", 3.0);
    let (v0, lisd, lig, m1) = (
        c.layer("V0"),
        c.layer("LISD"),
        c.layer("LIG"),
        c.layer("M1"),
    );
    let s = 18.0;
    let pitch = s + 2.0 * RUN + ROOM;
    let mut elems = vec![];
    for (k, (l, r)) in [
        (en, en),
        (en + DBU, en + DBU),
        (en - DBU, en),
        (en, en - DBU),
    ]
    .into_iter()
    .enumerate()
    {
        let (x, y) = (OFFSET, OFFSET + k as f64 * pitch);
        elems.push(bx(v0, x, y, s, s));
        // The LISD ends half way up the V0; the LIG crosses it, 1 nm inside it.
        elems.push(bx(lisd, x - l, y - RUN, s + l + r, RUN + s / 2.0));
        elems.push(bx(lig, x - 20.0, y + 1.0, s + 40.0, s - 2.0));
        elems.push(bx(m1, x - 5.0, y, s + 10.0, s));
    }
    c.write("v0", "V0.LISD.EN.3.short", true, elems);
}
