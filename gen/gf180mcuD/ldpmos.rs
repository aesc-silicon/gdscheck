// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! LDMOS PFET: a good and a bad pattern for every rule in the `ldpmos` deck.
//!
//! The same shape of fixture as the NFET next door (one whole device with one edge moved)
//! over a device built the other way up.  It sits in a deep N-well, its source and drain
//! are P+, and the guard ring round it is N+, which is what MDP.12 measures the deep well
//! against.  MDP.3d and MDP.4 then fire on more than 10 µm of active edge Metal1 does not
//! cover, so every fixture straps the ring the way a real cell would - following it,
//! rather than plating the field inside it, or those two rules could never see anything.
//!
//! MDP.9d wants the gate to reach the drain's own 0.16 µm surround and stop there: poly
//! that overlaps the surround breaks it and poly that never reaches it breaks it too.  So
//! the drain is placed off the gate rather than off the drift's far end, and every
//! fixture inherits that spacing.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/ldpmos";

struct Ctx {
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly2: (i16, i16),
    mvpsd: (i16, i16),
    dualgate: (i16, i16),
    ldmos: (i16, i16),
    contact: (i16, i16),
    metal1: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
}

#[derive(Clone, Copy)]
struct Dev {
    /// The channel's length: MDP.1 asks 0.6 µm of it and MDP.1a caps it at 20.
    channel: f64,
    /// How far the *active* reaches into the drift.  That overlap is the channel the drift
    /// covers, which MDP.10 fixes at 0.4 in both directions.
    comp_into_drift: f64,
    /// How far the gate reaches past the active, on into the drift - MDP.9c's extension
    /// beyond COMP towards the drain, 0.2 both ways.  The gate ends at the sum of the two,
    /// so bending either rule leaves the other's measurement alone, and the drain sits off
    /// the gate's end where MDP.9d puts it rather than inside the source.
    poly_past_comp: f64,
    /// How far the gate reaches past the active in the width direction, which MDP.9b
    /// asks 0.4 of.
    poly_past_width: f64,
    /// How much active there is to the left of the gate - the source.  MDP.13b wants each
    /// finger to meet exactly one, so a device drawn without any breaks it.
    source_len: f64,
    /// Whether to leave the N+ guard ring open on one side.  MDP.3 wants it unbroken, and
    /// a ring with a gap in it has no hole for the device to sit in.
    ring_break: bool,
    /// The device's width, which MDP.13a caps at 50 µm.
    width: f64,
    /// The drain's height; MDP.16a asks 0.22 µm.
    drain_h: f64,
    /// How far the drift reaches past the drain it holds, which MDP.11 asks 0.8 of.
    drain_hold: f64,
    /// And how far it reaches past the active in the width direction, which is the other
    /// direction of the same rule.
    drift_past_width: f64,
    /// Whether the drain is contacted, which only MDP.16b needs.
    drain_contact: bool,
    /// How far the deep well holds the guard ring - MDP.12 asks 0.66 µm.
    well_hold: f64,
    /// The ring's wall and how far its hole clears the drift.
    ring_wall: f64,
    ring_clear: Option<f64>,
    /// A narrow tab off the gate, for the rule that bounds the poly's own width.
    poly_tab: Option<f64>,
    /// Whether Dualgate stays inside the LDMOS marker, which MDP.6a asks.
    dualgate_out: f64,
    /// How far Dualgate reaches past the deep well.  MDP.5a measures a P+ active against
    /// that edge and MDP.4a measures one against the well's, and the two rules pull in
    /// opposite directions, so each fixture sets the gap it needs.
    outer_margin: f64,
}

impl Default for Dev {
    fn default() -> Self {
        Dev {
            channel: 7.0,
            comp_into_drift: 0.4,
            poly_past_comp: 0.2,
            poly_past_width: 0.5,
            source_len: 1.0,
            ring_break: false,
            width: 6.0,
            drain_h: 4.0,
            drain_hold: 1.0,
            drift_past_width: 1.0,
            drain_contact: true,
            well_hold: 1.5,
            ring_wall: 2.0,
            ring_clear: None,
            poly_tab: None,
            dualgate_out: 0.0,
            outer_margin: 1.0,
        }
    }
}

const HOLE_TOP: f64 = 22.0;
const HOLE_RIGHT: f64 = 22.0;
/// Where a fixture parks a second shape: inside the ring's hole, clear of the device.
const SPARE: f64 = 13.0;

fn device(c: &Ctx, d: Dev) -> Vec<GdsElement> {
    let o = OFFSET;
    let (x0, y0) = (o + 4.0, o + 4.0);
    let y1 = y0 + d.width;
    let gate_x = x0 + d.source_len;
    let drift_x = gate_x + d.channel;
    let comp_x1 = drift_x + d.comp_into_drift;
    let (py0, py1) = (y0 - d.poly_past_width, y1 + d.poly_past_width);
    // MDP.9d reads the drain's own 0.16 µm surround: poly that overlaps it breaks the
    // rule and poly that never reaches it breaks the rule, so the gate stops exactly
    // there and the drain is placed off the gate rather than off the drift's far end.
    let poly_x1 = comp_x1 + d.poly_past_comp;
    let (drain_x, drain_x1) = (poly_x1 + 0.16, poly_x1 + 2.16);
    let drift_x1 = drain_x1 + d.drain_hold;
    let mid = (y0 + y1) * 0.5;
    let (ky0, ky1) = (mid - d.drain_h * 0.5, mid + d.drain_h * 0.5);

    let mut v = vec![
        rect(c.comp, x0, y0, comp_x1, y1),
        rect(c.comp, drain_x, ky0, drain_x1, ky1),
        rect(c.pplus, x0 - 0.3, y0 - 0.3, drain_x1 + 0.3, y1 + 0.3),
        rect(c.poly2, gate_x, py0, poly_x1, py1),
        rect(
            c.mvpsd,
            drift_x,
            y0 - d.drift_past_width,
            drift_x1,
            y1 + d.drift_past_width,
        ),
    ];
    if d.drain_contact {
        v.push(rect(
            c.contact,
            drain_x + 0.9,
            mid - 0.11,
            drain_x + 1.12,
            mid + 0.11,
        ));
    }
    if let Some(w) = d.poly_tab {
        v.push(rect(
            c.poly2,
            gate_x + 1.0,
            py1,
            gate_x + 1.0 + w,
            py1 + 2.0,
        ));
    }
    // The N+ guard ring, four bars that merge into a ring with a hole.
    let (ix0, iy0, ix1, iy1) = match d.ring_clear {
        Some(g) => (x0 - 1.0 - g, py0 - g, drift_x1 + g, py1 + g),
        None => (
            o + 1.0,
            o + 1.0,
            (o + HOLE_RIGHT).max(drift_x1 + 2.0),
            (o + HOLE_TOP).max(py1 + 2.0),
        ),
    };
    let w = d.ring_wall;
    let (ox0, oy0, ox1, oy1) = (ix0 - w, iy0 - w, ix1 + w, iy1 + w);
    for l in [c.comp, c.nplus] {
        v.push(rect(l, ox0, oy0, ox1, iy0));
        v.push(rect(l, ox0, iy1, ox1, oy1));
        v.push(rect(l, ox0, iy0, ix0, iy1));
        if !d.ring_break {
            v.push(rect(l, ix1, iy0, ox1, iy1));
        }
    }
    // Metal1 straps the ring, the way a real cell does.  It follows the ring rather than
    // covering the field inside it: MDP.3d and MDP.4 fire on bare active edge, and a
    // plate over the whole device would hide every one of them.
    for (a, b, e, f) in [
        (ox0, oy0, ox1, iy0),
        (ox0, iy1, ox1, oy1),
        (ox0, iy0, ix0, iy1),
        (ix1, iy0, ox1, iy1),
    ] {
        v.push(rect(c.metal1, a - 0.2, b - 0.2, e + 0.2, f + 0.2));
    }
    // The deep well holds the ring, Dualgate holds the well, and the marker holds
    // Dualgate - MDP.12, MDP.5 and MDP.6a in that order.
    let h = d.well_hold;
    v.push(rect(c.dnwell, ox0 - h, oy0 - h, ox1 + h, oy1 + h));
    let (g, m) = (d.dualgate_out, d.outer_margin);
    v.push(rect(
        c.dualgate,
        ox0 - h - m - g,
        oy0 - h - m,
        ox1 + h + m + g,
        oy1 + h + m,
    ));
    v.push(rect(
        c.ldmos,
        ox0 - h - m - 0.5,
        oy0 - h - m - 0.5,
        ox1 + h + m + 0.5,
        oy1 + h + m + 0.5,
    ));
    v
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly2: layer(pdk, "poly2_drawn"),
        mvpsd: layer(pdk, "mvpsd"),
        dualgate: layer(pdk, "dualgate"),
        ldmos: layer(pdk, "ldmos_xtor"),
        contact: layer(pdk, "contact"),
        metal1: layer(pdk, "metal1_drawn"),
        nwell: layer(pdk, "nwell"),
        dnwell: layer(pdk, "dnwell"),
    };
    let o = OFFSET;
    let write = |id: &str, polarity: &str, elems: Vec<GdsElement>| {
        write_gz(
            &format!("{DIR}/{id}.{polarity}.gds.gz"),
            library("TOP", elems),
        );
    };
    let good = Dev::default();
    let with = |extra: Vec<GdsElement>| {
        let mut v = device(&c, good);
        v.extend(extra);
        v
    };
    let ncomp = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c.comp, x0, y0, x1, y1),
            rect(c.nplus, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    };
    let pcomp = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            rect(c.comp, x0, y0, x1, y1),
            rect(c.pplus, x0 - 0.1, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    };
    // Butted against one another, the two implants meet on a line rather than overlap:
    // an overlap cuts the very active the rule wants to measure out of both layers.
    let butted = |x0: f64, y0: f64, xm: f64, x1: f64, y1: f64, n_left: bool| {
        let (l, r) = if n_left {
            (c.nplus, c.pplus)
        } else {
            (c.pplus, c.nplus)
        };
        vec![
            rect(c.comp, x0, y0, x1, y1),
            rect(l, x0 - 0.1, y0 - 0.1, xm, y1 + 0.1),
            rect(r, xm, y0 - 0.1, x1 + 0.1, y1 + 0.1),
        ]
    };
    // A drift with the channel every drift is expected to have: the active reaches
    // 0.4 µm into it, which is what MDP.10 measures, the gate runs 0.2 µm past the active
    // and 2 µm further to pick up a source, which MDP.13b asks each finger for, and the
    // drain sits 0.16 µm off the gate, the surround MDP.9d reads.  `flip` puts the gate on
    // the right instead, so a mirrored pair can share one drain and so one potential.  The
    // height is the channel's width, which MDP.2 floors at 4 µm.
    let drift = |x: f64, y: f64, w: f64, h: f64, flip: bool| {
        let (cy0, cy1) = (y + 1.0, y + h - 1.0);
        let s = if flip { -1.0 } else { 1.0 };
        let e = if flip { x + w } else { x };
        let at = |a: f64, b: f64| {
            let (p, q) = (e + s * a, e + s * b);
            (p.min(q), p.max(q))
        };
        let (gx0, gx1) = at(-1.0, 0.6);
        let (sx0, sx1) = at(-2.0, 0.4);
        let (dx0, dx1) = at(0.76, 2.0);
        let (px0, px1) = at(-2.1, 2.1);
        vec![
            rect(c.mvpsd, x, y, x + w, y + h),
            rect(c.comp, sx0, cy0, sx1, cy1),
            rect(c.poly2, gx0, cy0 - 0.4, gx1, cy1 + 0.4),
            rect(c.comp, dx0, cy0, dx1, cy1),
            rect(c.pplus, px0, cy0 - 0.1, px1, cy1 + 0.1),
        ]
    };
    // Where the default device's marker and its deep well end, so the fixtures that
    // measure something against either from outside can be placed off them.
    let marker_r = o + 27.0;
    let well_r = o + 25.5;

    // The rules that bend one of the device's own dimensions.
    for (id, bad, clean) in [
        (
            "MDP.1",
            Dev {
                channel: 0.595,
                ..good
            },
            good,
        ),
        (
            "MDP.1a",
            Dev {
                channel: 20.005,
                ..good
            },
            good,
        ),
        (
            "MDP.2",
            Dev {
                width: 3.995,
                drain_h: 3.0,
                ..good
            },
            good,
        ),
        (
            "MDP.13b",
            Dev {
                source_len: 0.0,
                ..good
            },
            good,
        ),
        (
            "MDP.13a",
            Dev {
                width: 50.005,
                ..good
            },
            good,
        ),
        (
            "MDP.16a",
            Dev {
                drain_h: 0.215,
                drain_contact: false,
                ..good
            },
            Dev {
                drain_contact: false,
                ..good
            },
        ),
        (
            "MDP.12",
            Dev {
                well_hold: 0.655,
                ..good
            },
            good,
        ),
        (
            "MDP.6a",
            Dev {
                dualgate_out: 2.0,
                ..good
            },
            good,
        ),
        (
            "MDP.11",
            Dev {
                drain_hold: 0.795,
                ..good
            },
            good,
        ),
        (
            "MDP.9b",
            Dev {
                poly_past_width: 0.395,
                ..good
            },
            good,
        ),
        (
            "MDP.10",
            Dev {
                comp_into_drift: 0.405,
                ..good
            },
            good,
        ),
        (
            "MDP.9a",
            Dev {
                poly_tab: Some(1.195),
                ..good
            },
            Dev {
                poly_tab: Some(2.0),
                ..good
            },
        ),
    ] {
        write(id, "good", device(&c, clean));
        write(id, "bad", device(&c, bad));
    }

    // MDP.10b: two drifts closer than 1 µm at the same potential, and MDP.10a the
    // different-potential rule at 2 µm.
    let pair_same = |gap: f64| {
        let mut v = drift(o + 4.0, o + SPARE, 4.0, 6.5, false);
        v.extend(drift(o + 8.0 + gap, o + SPARE, 4.0, 6.5, true));
        // The pair is at one potential because one active is the drain of both.
        v.extend(pcomp(
            o + 6.0,
            o + SPARE + 1.0,
            o + 10.0 + gap,
            o + SPARE + 5.5,
        ));
        v
    };
    write("MDP.10b", "good", with(pair_same(1.5)));
    write("MDP.10b", "bad", with(pair_same(0.995)));
    let pair_diff = |gap: f64| {
        // Mirrored, so that neither drift's source active reaches over the other at the
        // sub-2 µm gaps the rule is about, and the two stay at potentials of their own.
        let mut v = drift(o + 4.0, o + SPARE, 4.0, 6.5, false);
        v.extend(drift(o + 8.0 + gap, o + SPARE, 4.0, 6.5, true));
        v
    };
    write("MDP.10a", "good", with(pair_diff(2.5)));
    write("MDP.10a", "bad", with(pair_diff(1.995)));

    // MDP.15: a deep well within 6 µm of the one that carries a drift.
    let deep = |gap: f64| vec![rect(c.dnwell, o + 30.0 + gap, o, o + 40.0 + gap, o + 10.0)];
    write("MDP.15", "good", with(deep(7.0)));
    write("MDP.15", "bad", with(deep(0.0)));

    // MDP.7 and MDP.8: an N-well, then an N+ active, too close to the LDMOS marker from
    // outside it.
    // Both of these measure something outside the marker against it.  The neighbour's
    // top edge lands exactly on a 20 µm tile line, which is where a gap midpoint on that
    // line used to be claimed by the tile above and dropped by the tile below.
    let outside = |gap: f64, well: bool| {
        let x = marker_r + gap;
        if well {
            vec![rect(c.nwell, x, o + 4.0, x + 4.0, o + 10.0)]
        } else {
            ncomp(x, o + 4.0, x + 4.0, o + 10.0)
        }
    };
    write("MDP.7", "good", with(outside(3.0, true)));
    write("MDP.7", "bad", with(outside(1.995, true)));
    write("MDP.8", "good", with(outside(3.0, false)));
    write("MDP.8", "bad", with(outside(1.495, false)));

    // MDP.3ai: an N+ active touching no P+, closer than 1 µm to a drift.
    let ntouch = |gap: f64| ncomp(o + 13.0, o + 10.5 + gap, o + 15.0, o + 12.0 + gap);
    write("MDP.3ai", "good", with(ntouch(2.0)));
    write("MDP.3ai", "bad", with(ntouch(0.995)));

    // MDP.3aii: an N+ active that does touch a P+, closer than 0.92 µm to a drift.
    let nbutt = |gap: f64| {
        let mut v = drift(o + 4.0, o + SPARE, 5.0, 6.5, false);
        v.extend(butted(
            o + 7.0,
            o + SPARE + 1.0,
            o + 9.0 + gap,
            o + 11.0 + gap,
            o + SPARE + 3.0,
            false,
        ));
        v
    };
    write("MDP.3aii", "good", with(nbutt(2.0)));
    write("MDP.3aii", "bad", with(nbutt(0.915)));

    // MDP.3b: an N+ active closer than 0.4 µm to a P+ one, both clear of gate and drift.
    let nb = |gap: f64| {
        let mut v = pcomp(o + 5.0, o + SPARE, o + 7.0, o + SPARE + 2.0);
        v.extend(ncomp(
            o + 7.0 + gap,
            o + SPARE,
            o + 9.0 + gap,
            o + SPARE + 2.0,
        ));
        v
    };
    write("MDP.3b", "good", with(nb(1.0)));
    write("MDP.3b", "bad", with(nb(0.395)));

    // MDP.4a: a P+ active closer than 2.5 µm to the deep well, from outside it.
    let pnear = |gap: f64| {
        let mut v = device(
            &c,
            Dev {
                outer_margin: 8.0,
                ..good
            },
        );
        let x = well_r + gap;
        v.extend(pcomp(x, o + 4.0, x + 3.0, o + 10.0));
        v
    };
    write("MDP.4a", "good", with(pnear(4.0)));
    write("MDP.4a", "bad", with(pnear(2.495)));

    // MDP.9ei: an N+ active touching no P+, closer than 0.4 µm to the gate.
    let npoly = |gap: f64| ncomp(o + 6.0, o + 10.5 + gap, o + 8.0, o + 12.0 + gap);
    write("MDP.9ei", "good", with(npoly(1.0)));
    write("MDP.9ei", "bad", with(npoly(0.395)));

    // MDP.9eii: an N+ active that touches a P+, closer than 0.32 µm to the gate.
    let npoly_butt = |gap: f64| {
        let y = o + 10.5 + gap;
        butted(o + 5.0, y, o + 6.5, o + 7.5, y + 1.5, true)
    };
    write("MDP.9eii", "good", with(npoly_butt(1.0)));
    write("MDP.9eii", "bad", with(npoly_butt(0.315)));

    // MDP.9f: a poly run bridging two implanted stretches.
    let bridge = |joined: bool| {
        let mut v = device(
            &c,
            Dev {
                poly_tab: Some(2.0),
                ..good
            },
        );
        if joined {
            v.push(rect(c.pplus, o + 6.5, o + 11.5, o + 7.5, o + 12.5));
        }
        v
    };
    write("MDP.9f", "good", bridge(false));
    write("MDP.9f", "bad", bridge(true));

    // MDP.5: device material the marker covers and Dualgate does not.
    let outside_dg = |out: bool| {
        let mut v = device(&c, good);
        if out {
            v.push(rect(c.ldmos, o + 30.0, o + 2.0, o + 40.0, o + 12.0));
            v.push(rect(c.dualgate, o + 31.0, o + 3.0, o + 35.0, o + 11.0));
            v.extend(pcomp(o + 36.0, o + 5.0, o + 39.0, o + 8.0));
        }
        v
    };
    write("MDP.5", "good", outside_dg(false));
    write("MDP.5", "bad", outside_dg(true));

    // MDP.6: a drift outside the LDMOS marker.
    write(
        "MDP.6",
        "good",
        with(drift(o + 4.0, o + SPARE, 4.0, 6.5, false)),
    );
    write("MDP.6", "bad", with(drift(o + 32.0, o + 4.0, 4.0, 6.5, false)));

    // MDP.16b: a contact on the drain that reaches off the drain's own active.
    let drain_contact = |dx: f64| {
        let mut v = device(&c, good);
        v.push(rect(
            c.contact,
            o + 14.54 + dx,
            o + 6.9,
            o + 14.76 + dx,
            o + 7.12,
        ));
        v
    };
    write("MDP.16b", "good", drain_contact(0.0));
    write("MDP.16b", "bad", drain_contact(0.15));

    // MDP.17c: a deep well drawn as a ring, which has to hold every N+ active in it.
    let well_ring = |hole: f64| {
        let mut v = device(&c, good);
        let (x, y) = (o + 32.0, o + 2.0);
        for (a, b) in [(0.0, 2.0), (2.0 + hole, 4.0 + hole)] {
            v.push(rect(c.dnwell, x + a, y, x + b, y + 4.0 + hole));
            v.push(rect(c.dnwell, x, y + a, x + 4.0 + hole, y + b));
        }
        v.extend(ncomp(x + 2.5, y + 2.5, x + 1.5 + hole, y + 1.5 + hole));
        v
    };
    write("MDP.17c", "good", well_ring(0.0));
    write("MDP.17c", "bad", well_ring(6.0));

    // MDP.9d: gate poly that does not reach the drain, or reaches material it should not.
    // The drain's own 0.16 µm surround is what the rule reads, so a stray poly island in
    // the device that touches neither breaks it.
    let stray_poly = |on: bool| {
        let mut v = device(&c, good);
        if on {
            v.push(rect(c.poly2, o + 6.0, o + SPARE, o + 8.0, o + SPARE + 2.0));
        }
        v
    };
    write("MDP.9d", "good", stray_poly(false));
    write("MDP.9d", "bad", stray_poly(true));

    // MDP.3d and MDP.4: more than 10 µm of active edge that Metal1 does not cover - the
    // guard ring for one, a P+ active for the other.
    let bare = |strip: bool, n: bool| {
        let mut v = device(&c, good);
        if strip {
            // A long finger reaching out from under the plate.
            let y = o + SPARE;
            let piece = if n {
                ncomp(o + 3.0, y, o + 20.0, y + 1.0)
            } else {
                pcomp(o + 3.0, y, o + 20.0, y + 1.0)
            };
            v.extend(piece);
            v.push(rect(c.metal1, o + 3.0, y + 1.0, o + 20.0, y + 1.2));
        }
        v
    };
    write("MDP.3d", "good", bare(false, true));
    write("MDP.3d", "bad", bare(true, true));
    write("MDP.4", "good", bare(false, false));
    write("MDP.4", "bad", bare(true, false));

    // MDP.5a: a P+ active within 0.5 µm of where the marker and Dualgate meet.
    let near_edge = |margin: f64| {
        let mut v = device(
            &c,
            Dev {
                outer_margin: margin,
                ..good
            },
        );
        v.extend(pcomp(well_r - 1.0, o + 5.0, well_r, o + 8.0));
        v
    };
    write("MDP.5a", "good", near_edge(1.0));
    write("MDP.5a", "bad", near_edge(0.395));

    // MDP.4b: a deep well edge more than 15 µm from any P+ active.  The rule reads only
    // the well edges that fall in a hole of the P+, so this fixture is a P+ ring with the
    // well inside it, and the question is how big the ring's hole is.
    let reach = |hole: f64| {
        let mut v = device(&c, good);
        let (x, y) = (o + 32.0, o + 2.0);
        for (a, b) in [(0.0, 2.0), (2.0 + hole, 4.0 + hole)] {
            v.push(rect(c.comp, x + a, y, x + b, y + 4.0 + hole));
            v.push(rect(c.comp, x, y + a, x + 4.0 + hole, y + b));
            v.push(rect(
                c.pplus,
                x + a - 0.1,
                y - 0.1,
                x + b + 0.1,
                y + 4.1 + hole,
            ));
            v.push(rect(
                c.pplus,
                x - 0.1,
                y + a - 0.1,
                x + 4.1 + hole,
                y + b + 0.1,
            ));
            v.push(rect(
                c.metal1,
                x + a - 0.2,
                y - 0.2,
                x + b + 0.2,
                y + 4.2 + hole,
            ));
            v.push(rect(
                c.metal1,
                x - 0.2,
                y + a - 0.2,
                x + 4.2 + hole,
                y + b + 0.2,
            ));
        }
        let (mid, half) = (x + 2.0 + hole * 0.5, 1.5);
        v.push(rect(
            c.dnwell,
            mid - half,
            y + 2.0 + hole * 0.5 - half,
            mid + half,
            y + 2.0 + hole * 0.5 + half,
        ));
        v.push(rect(
            c.dualgate,
            x - 1.0,
            y - 1.0,
            x + 5.0 + hole,
            y + 5.0 + hole,
        ));
        v.push(rect(
            c.ldmos,
            x - 1.5,
            y - 1.5,
            x + 5.5 + hole,
            y + 5.5 + hole,
        ));
        // The marker only counts as an LDPMOS one because a drift sits in it, and a
        // drift needs its channel, so it is parked in the ring's hole clear of the deep
        // well rather than on top of it - MDP.4a keeps P+ active 2.5 µm off the well.
        v.extend(drift(x + 5.0, y + 2.0 + hole * 0.5 - 3.25, 3.0, 6.5, false));
        v
    };
    // MDP.13c: a finger with source on both sides.  It brings its own ring rather than
    // sharing the device's, for the reason its N-side twin does - the drain beside a shared
    // source is a drain like any other, and a ring holding two of those answers to a
    // different rule.  The right-hand drift meets four actives so it is not one of them.
    let two_finger = |shared: bool| {
        let (dy0, dy1) = (o + 7.0, o + 14.0);
        let (gy0, gy1) = (o + 7.6, o + 13.4);
        let (ny0, ny1) = (o + 8.0, o + 13.0);
        let src_x1 = if shared { o + 13.4 } else { o + 9.0 };
        let mut v = vec![
            rect(c.mvpsd, o + 2.0, dy0, o + 6.0, dy1),
            rect(c.poly2, o + 5.6, gy0, o + 7.4, gy1),
        ];
        v.extend(pcomp(o + 5.6, ny0, src_x1, ny1));
        // MDP.9d puts the drain 0.16 past the gate, inside the drift.
        v.extend(pcomp(o + 5.14, o + 10.0, o + 5.44, o + 11.0));
        if shared {
            v.push(rect(c.mvpsd, o + 13.0, dy0, o + 18.0, dy1));
            v.push(rect(c.poly2, o + 11.6, gy0, o + 13.4, gy1));
            v.extend(pcomp(o + 13.56, o + 10.0, o + 13.86, o + 11.0));
            v.extend(pcomp(o + 16.0, o + 13.5, o + 17.0, o + 14.5));
            v.extend(pcomp(o + 14.0, o + 6.0, o + 15.0, o + 7.5));
        }
        // The N+ ring the P device sits in, and the deep well that holds it.
        let (ix0, iy0, ix1, iy1) = (o + 1.0, o + 1.0, o + 22.0, o + 22.0);
        let w = 2.0;
        for l in [c.comp, c.nplus] {
            v.push(rect(l, ix0 - w, iy0 - w, ix1 + w, iy0));
            v.push(rect(l, ix0 - w, iy1, ix1 + w, iy1 + w));
            v.push(rect(l, ix0 - w, iy0, ix0, iy1));
            v.push(rect(l, ix1, iy0, ix1 + w, iy1));
        }
        // Metal1 straps the ring, as the device's own does: MDP.3d and MDP.4 read bare
        // active edge, and a ring left unstrapped is all of it.
        for (a, b, e, f) in [
            (ix0 - w, iy0 - w, ix1 + w, iy0),
            (ix0 - w, iy1, ix1 + w, iy1 + w),
            (ix0 - w, iy0, ix0, iy1),
            (ix1, iy0, ix1 + w, iy1),
        ] {
            v.push(rect(c.metal1, a - 0.2, b - 0.2, e + 0.2, f + 0.2));
        }
        v.push(rect(
            c.dnwell,
            ix0 - w - 1.0,
            iy0 - w - 1.0,
            ix1 + w + 1.0,
            iy1 + w + 1.0,
        ));
        for l in [c.dualgate, c.ldmos] {
            v.push(rect(
                l,
                ix0 - w - 3.0,
                iy0 - w - 3.0,
                ix1 + w + 3.0,
                iy1 + w + 3.0,
            ));
        }
        v
    };
    write("MDP.13c", "good", two_finger(false));
    write("MDP.13c", "bad", two_finger(true));

    // MDP.17a: N+ active outside both wells, within 40 µm of the marked drift and with no
    // deep-well ring between them.  The device's own ring holes shield anything near it,
    // so the active sits out past the well where none of them reaches.
    // The rule is about the ring, not the distance: with the active this close, an
    // unbroken deep-well ring is what stands between them, and a ring with a gap in it has
    // no hole to stand anywhere.
    let near_active = |broken: bool| {
        let mut v = device(
            &c,
            Dev {
                ring_break: broken,
                ..good
            },
        );
        v.extend(ncomp(o + 30.0, o + 6.0, o + 34.0, o + 12.0));
        v
    };
    write("MDP.17a", "good", near_active(false));
    write("MDP.17a", "bad", near_active(true));

    write("MDP.4b", "good", reach(20.0));
    write("MDP.4b", "bad", reach(40.0));
}
