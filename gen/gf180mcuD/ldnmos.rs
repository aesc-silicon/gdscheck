// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! LDMOS NFET: a good and a bad pattern for every rule in the `ldnmos` deck.
//!
//! Every fixture is one device with one thing wrong with it.  An LDMOS is not a shape the
//! way a via or a well is - it is a source, a gate, a drift region under the far end of
//! that gate, a drain inside the drift, and a P+ guard ring round the lot - and most of
//! this deck is about how those parts sit against one another, so leaving any of them out
//! fires half the rules at once.  The fixtures draw the whole device and move one edge.
//!
//! The guard ring's hole is drawn far larger than the device needs, and the space left
//! over is where the fixtures that need a second shape put it.  Almost every rule here
//! measures into that hole - the device's own layers have to be inside it (MDN.17), the
//! ring has to keep 1 µm from any drift (MDN.5ai), a stray active has to keep 4 µm
//! (MDN.9) - so a shape parked outside the ring answers a different rule than the one it
//! was drawn for, which is how the first draft of nearly every fixture below failed.
//!
//! The LDMOS marker and Dualgate are the same rectangle.  MDN.6 fires on device material
//! the marker covers and Dualgate does not, MDN.7 on material a Dualgate covers outside
//! the marker, and MDN.7a on any Dualgate that overlaps the marker and reaches past it -
//! so those three fixtures are the only ones that pull the two apart, and MDN.7 has to do
//! it with a Dualgate of its own that never touches the marker at all, or it fires MDN.7a
//! in passing.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/ldnmos";

struct Ctx {
    comp: (i16, i16),
    nplus: (i16, i16),
    pplus: (i16, i16),
    poly2: (i16, i16),
    mvsd: (i16, i16),
    dualgate: (i16, i16),
    ldmos: (i16, i16),
    contact: (i16, i16),
    nwell: (i16, i16),
    dnwell: (i16, i16),
}

/// The device, with one knob per rule that bends it.  The defaults draw a legal one.
#[derive(Clone, Copy)]
struct Dev {
    /// How much active there is to the left of the gate - the source.  MDN.13b wants each
    /// finger to meet exactly one, so a device drawn without any breaks it.
    source_len: f64,
    /// How far the *active* reaches into the drift.  That overlap is the channel the
    /// drift covers, which MDN.11 fixes at 0.4 in both directions.
    comp_into_drift: f64,
    /// How far the gate reaches past the active, on into the drift.  MDN.10c fixes this
    /// at 0.2 in both directions - it is the poly's extension beyond COMP on the field,
    /// towards the drain.  The gate ends at `comp_into_drift + poly_past_comp` into the
    /// drift, so bending either rule leaves the other's measurement alone.
    poly_past_comp: f64,
    /// How far the gate reaches past the active - MDN.10b asks 0.4 µm.
    endcap: f64,
    /// The drain's height; MDN.15a asks 0.22 µm.
    drain_h: f64,
    /// Whether the drain is contacted, which only MDN.15b needs.
    drain_contact: bool,
    /// How far Dualgate reaches past the guard ring - MDN.6a asks 0.5 µm of the active.
    margin: f64,
    /// The channel's length: MDN.3a asks 0.6 µm of it and MDN.3b caps it at 20.  It is
    /// how far the gate runs before the drift starts, which is what the gate holding the
    /// drift's near edge amounts to - the drift's other edges run past the gate top and
    /// bottom and out to the drain, and a wall the gate does not hold is not measured.
    channel: f64,
    /// How long the drift is.  MDN.10c only reads where the poly's overhang falls inside
    /// the drain's own bounding box, and that box is the *closed* N+ - so the drain island
    /// has to sit near enough the active for the two to close into one box, which a short
    /// drift is what arranges.
    drift_len: f64,
    /// The device's width, which MDN.4b and MDN.13a cap at 50 µm.
    width: f64,
    /// A narrow tab off the gate, for the one rule that bounds the poly's own width.
    poly_tab: Option<f64>,
    /// The guard ring's wall.  MDN.6a wants Dualgate close to the active, and the ring
    /// stands between them, so its fixture draws a thin one.
    ring_wall: f64,
    /// How far the ring's hole clears the drift - MDN.5ai asks 1 µm.
    ring_clear: Option<f64>,
}

impl Default for Dev {
    fn default() -> Self {
        Dev {
            source_len: 1.0,
            comp_into_drift: 0.4,
            poly_past_comp: 0.2,
            endcap: 0.5,
            drain_h: 4.0,
            drain_contact: true,
            margin: 3.0,
            channel: 7.0,
            drift_len: 7.0,
            width: 6.0,
            poly_tab: None,
            ring_wall: 2.0,
            ring_clear: None,
        }
    }
}

/// Where a fixture parks the extra shapes its own rule needs.  It is *outside* the
/// device's LDMOS_XTOR and Dualgate marker, which reaches `o + 27`: MDN.11 forbids a drift
/// with no channel under it and MDN.13d forbids two drains in one ring, and both read only
/// what the marker covers, so a bare bar of MVSD parked inside it would answer for them
/// rather than for the rule it was drawn for.  The rules those bars are drawn for -
/// MDN.1, MDN.2a/b, MDN.8a/b, MDN.14 - all measure the drawn MVSD and do not need it.
const SPARE: f64 = 30.0;
const HOLE_TOP: f64 = 22.0;
const HOLE_RIGHT: f64 = 22.0;

fn device(c: &Ctx, d: Dev) -> Vec<GdsElement> {
    let o = OFFSET;
    let (x0, y0) = (o + 4.0, o + 4.0);
    let y1 = y0 + d.width;
    let gate_x = x0 + d.source_len;
    let drift_x = gate_x + d.channel;
    let comp_x1 = drift_x + d.comp_into_drift;
    let drift_x1 = drift_x + d.drift_len;
    let (py0, py1) = (y0 - d.endcap, y1 + d.endcap);
    let (dy0, dy1) = (y0 - 0.5, y1 + 0.5);
    let (drain_x, drain_x1) = (drift_x1 - 3.0, drift_x1 - 1.0);
    let mid = (y0 + y1) * 0.5;
    let (ky0, ky1) = (mid - d.drain_h * 0.5, mid + d.drain_h * 0.5);

    let mut v = vec![
        // Source, channel and the near end of the drift, all one active.
        rect(c.comp, x0, y0, comp_x1, y1),
        // The drain, an active island inside the drift.
        rect(c.comp, drain_x, ky0, drain_x1, ky1),
        rect(c.nplus, x0 - 0.3, y0 - 0.3, drain_x1 + 0.3, y1 + 0.3),
        rect(c.poly2, gate_x, py0, comp_x1 + d.poly_past_comp, py1),
        rect(c.mvsd, drift_x, dy0, drift_x1, dy1),
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
        // Hangs off the gate's top, clear of the active and of the ring.
        v.push(rect(
            c.poly2,
            gate_x + 1.0,
            py1,
            gate_x + 1.0 + w,
            py1 + 2.0,
        ));
    }
    // The P+ guard ring, four bars that merge into a ring with a hole.  The hole is
    // always at least SPARE-and-more across, and grows past that with the device: a long
    // channel or a wide one would otherwise put the device's own layers outside it, and
    // MDN.17 would answer for whatever rule the fixture was drawn for.
    let (ix0, iy0, ix1, iy1) = match d.ring_clear {
        // Hugging the device: only as much room as the rules that measure into the hole
        // need, which is 1 µm off the drift.
        Some(g) => (x0 - 1.0 - g, dy0 - g, drift_x1 + g, py1 + g),
        None => (
            o + 1.0,
            o + 1.0,
            (o + HOLE_RIGHT).max(drift_x1 + 2.0),
            (o + HOLE_TOP).max(py1 + 2.0),
        ),
    };
    let w = d.ring_wall;
    let (ox0, oy0, ox1, oy1) = (ix0 - w, iy0 - w, ix1 + w, iy1 + w);
    for l in [c.comp, c.pplus] {
        v.push(rect(l, ox0, oy0, ox1, iy0));
        v.push(rect(l, ox0, iy1, ox1, oy1));
        v.push(rect(l, ox0, iy0, ix0, iy1));
        v.push(rect(l, ix1, iy0, ox1, iy1));
    }
    for l in [c.ldmos, c.dualgate] {
        v.push(rect(
            l,
            ox0 - d.margin,
            oy0 - d.margin,
            ox1 + d.margin,
            oy1 + d.margin,
        ));
    }
    v
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let c = Ctx {
        comp: layer(pdk, "comp"),
        nplus: layer(pdk, "nplus"),
        pplus: layer(pdk, "pplus"),
        poly2: layer(pdk, "poly2_drawn"),
        mvsd: layer(pdk, "mvsd"),
        dualgate: layer(pdk, "dualgate"),
        ldmos: layer(pdk, "ldmos_xtor"),
        contact: layer(pdk, "contact"),
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
    // A legal device plus whatever a rule needs beside it, in the ring's spare room.
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
            rect(c.pplus, x0 - 0.2, y0 - 0.2, x1 + 0.2, y1 + 0.2),
        ]
    };

    // The rules that bend one of the device's own dimensions.
    for (id, bad) in [
        (
            "MDN.3a",
            Dev {
                channel: 0.595,
                ..good
            },
        ),
        (
            "MDN.10b",
            Dev {
                endcap: 0.395,
                ..good
            },
        ),
        (
            "MDN.15a",
            Dev {
                drain_h: 0.215,
                drain_contact: false,
                ..good
            },
        ),
        (
            "MDN.12",
            Dev {
                drain_h: 6.01,
                ..good
            },
        ),
        (
            "MDN.4a",
            Dev {
                width: 3.995,
                // The drain has to shrink with the device or it comes within MDN.12's
                // half micron of the drift's own edge.
                drain_h: 3.0,
                ..good
            },
        ),
        (
            "MDN.11",
            Dev {
                comp_into_drift: 0.405,
                ..good
            },
        ),
        (
            "MDN.13b",
            Dev {
                source_len: 0.0,
                ..good
            },
        ),
        (
            "MDN.10c",
            Dev {
                poly_past_comp: 0.205,
                drift_len: 4.0,
                ..good
            },
        ),
        (
            "MDN.3b",
            Dev {
                channel: 20.005,
                ..good
            },
        ),
        (
            "MDN.4b",
            Dev {
                width: 50.005,
                ..good
            },
        ),
        (
            "MDN.13a",
            Dev {
                width: 50.005,
                ..good
            },
        ),
        (
            "MDN.10a",
            Dev {
                poly_tab: Some(1.195),
                ..good
            },
        ),
    ] {
        let clean = match id {
            "MDN.15a" => Dev {
                drain_contact: false,
                ..good
            },
            "MDN.10a" => Dev {
                poly_tab: Some(2.0),
                ..good
            },
            // Its good half needs the same short drift as its bad one, or the rule has
            // nothing to read on either and passes for the wrong reason.
            "MDN.10c" => Dev {
                drift_len: 4.0,
                ..good
            },
            _ => good,
        };
        write(id, "good", device(&c, clean));
        write(id, "bad", device(&c, bad));
    }

    // A drift bar in the spare room, which several rules measure something against.
    let bar = |x: f64, y: f64, w: f64, h: f64| rect(c.mvsd, x, y, x + w, y + h);

    // A drift that is a device, for the fixtures that need one where a bare bar of MVSD
    // would answer for MDN.11 instead.  `h` is the transistor's width, so it has to clear
    // MDN.4a's 4 µm with the 0.3 the active is inset by at each end: a gate crossing the near edge by exactly the 0.4
    // MDN.11 fixes, reaching 0.4 past the active for MDN.10b, the source it sits on, and a
    // bare drain island in the drift.  `right` puts the source on the far side, for a
    // fixture with something to its left; `plain` adds a third active so the drift meets
    // three and is not the two-active drain MDN.13d counts.
    let device_bar = |x: f64, y: f64, w: f64, h: f64, right: bool, plain: bool| {
        // The gate crosses one edge of the drift, reaching 0.4 in - which is the overlap
        // MDN.11 fixes - and 1.4 out; the source it sits on reaches 2.6 out.
        let (gx0, gx1, nx0, nx1) = if right {
            (x + w - 0.4, x + w + 1.4, x + w - 0.4, x + w + 2.6)
        } else {
            (x - 1.4, x + 0.4, x - 2.6, x + 0.4)
        };
        let mut v = vec![
            rect(c.mvsd, x, y, x + w, y + h),
            rect(c.poly2, gx0, y - 0.1, gx1, y + h + 0.1),
        ];
        v.extend(ncomp(nx0, y + 0.3, nx1, y + h - 0.3));
        v.extend(ncomp(
            x + w * 0.5,
            y + h * 0.35,
            x + w * 0.5 + 0.3,
            y + h * 0.65,
        ));
        if plain {
            // A third active on the drift, so it meets three and is not the two-active
            // drain MDN.13d counts.
            v.extend(ncomp(
                x + w * 0.3,
                y + h - 0.4,
                x + w * 0.3 + 0.3,
                y + h + 0.9,
            ));
        }
        v
    };

    // MDN.1: a drift narrower than 1 µm.
    write(
        "MDN.1",
        "good",
        with(vec![bar(o + 4.0, o + SPARE, 4.0, 1.0)]),
    );
    write(
        "MDN.1",
        "bad",
        with(vec![bar(o + 4.0, o + SPARE, 4.0, 0.995)]),
    );

    // MDN.13c: a finger with source on both sides.  It brings its own ring rather than
    // sharing the device's, because the drain beside a shared source is a drain like any
    // other and a ring holding two of those is what MDN.13d forbids.  The right-hand drift
    // meets a third active so it is not one of those, leaving the left as the only one.
    let two_finger = |shared: bool| {
        let (dy0, dy1) = (o + 7.0, o + 14.0);
        let (gy0, gy1) = (o + 7.6, o + 13.4);
        let (ny0, ny1) = (o + 8.0, o + 13.0);
        let src_x1 = if shared { o + 13.4 } else { o + 9.0 };
        let mut v = vec![
            rect(c.mvsd, o + 2.0, dy0, o + 6.0, dy1),
            rect(c.poly2, o + 5.6, gy0, o + 7.4, gy1),
        ];
        v.extend(ncomp(o + 5.6, ny0, src_x1, ny1));
        v.extend(ncomp(o + 3.0, o + 10.0, o + 3.3, o + 11.0));
        if shared {
            v.push(rect(c.mvsd, o + 13.0, dy0, o + 18.0, dy1));
            v.push(rect(c.poly2, o + 11.6, gy0, o + 13.4, gy1));
            v.extend(ncomp(o + 15.0, o + 10.0, o + 15.3, o + 11.0));
            // Two more actives, so the right-hand drift meets four and is neither the two
            // that make a drain nor the three that make a multi-finger one.
            v.extend(ncomp(o + 16.0, o + 13.5, o + 17.0, o + 14.5));
            v.extend(ncomp(o + 14.0, o + 6.0, o + 15.0, o + 7.5));
        }
        let (ix0, iy0, ix1, iy1) = (o + 1.0, o + 1.0, o + 22.0, o + 22.0);
        let w = 2.0;
        for l in [c.comp, c.pplus] {
            v.push(rect(l, ix0 - w, iy0 - w, ix1 + w, iy0));
            v.push(rect(l, ix0 - w, iy1, ix1 + w, iy1 + w));
            v.push(rect(l, ix0 - w, iy0, ix0, iy1));
            v.push(rect(l, ix1, iy0, ix1 + w, iy1));
        }
        for l in [c.ldmos, c.dualgate] {
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
    write("MDN.13c", "good", two_finger(false));
    write("MDN.13c", "bad", two_finger(true));

    // MDN.13d: a second drift-and-drain inside the device's own guard ring, so the ring
    // holds two where it may hold one.
    let second_drain = || {
        let y = o + 14.0;
        device_bar(o + 6.0, y, 5.0, 5.0, false, false)
    };
    write("MDN.13d", "good", with(vec![]));
    write("MDN.13d", "bad", with(second_drain()));

    // MDN.2a: two drifts closer than 1 µm at the same potential.  An active over both
    // shorts them, which is what keeps MDN.2b - the different-potential rule - quiet.
    let pair_same = |gap: f64| {
        let mut v = vec![
            bar(o + 4.0, o + SPARE, 4.0, 2.0),
            bar(o + 8.0 + gap, o + SPARE, 4.0, 2.0),
        ];
        v.extend(ncomp(
            o + 6.0,
            o + SPARE + 0.5,
            o + 10.0 + gap,
            o + SPARE + 1.5,
        ));
        v
    };
    write("MDN.2a", "good", with(pair_same(1.5)));
    write("MDN.2a", "bad", with(pair_same(0.995)));

    // MDN.2b: two drifts at different potentials, further apart than MDN.2a's limit.
    let pair_diff = |gap: f64| {
        vec![
            bar(o + 4.0, o + SPARE, 4.0, 2.0),
            bar(o + 8.0 + gap, o + SPARE, 4.0, 2.0),
        ]
    };
    write("MDN.2b", "good", with(pair_diff(2.5)));
    write("MDN.2b", "bad", with(pair_diff(1.995)));

    // MDN.8a: a drift within 1 µm of an N-well at the same potential - an active over
    // both ties them together, so MDN.8b has nothing to measure.
    let well_same = |gap: f64| {
        let mut v = vec![
            bar(o + 4.0, o + SPARE, 4.0, 2.0),
            rect(
                c.nwell,
                o + 8.0 + gap,
                o + SPARE,
                o + 13.0 + gap,
                o + SPARE + 2.0,
            ),
        ];
        v.extend(ncomp(
            o + 6.0,
            o + SPARE + 0.5,
            o + 10.0 + gap,
            o + SPARE + 1.5,
        ));
        v
    };
    write("MDN.8a", "good", with(well_same(2.0)));
    write("MDN.8a", "bad", with(well_same(0.995)));

    // MDN.8b: the same at different potentials, outside MDN.8a's 1 µm.
    let well_diff = |gap: f64| {
        vec![
            bar(o + 4.0, o + SPARE, 4.0, 2.0),
            rect(
                c.nwell,
                o + 8.0 + gap,
                o + SPARE,
                o + 13.0 + gap,
                o + SPARE + 2.0,
            ),
        ]
    };
    write("MDN.8b", "good", with(well_diff(2.5)));
    write("MDN.8b", "bad", with(well_diff(1.995)));

    // MDN.14: a drift within 6 µm of a deep well.
    let deep = |gap: f64| {
        vec![
            bar(o + 4.0, o + SPARE, 4.0, 2.0),
            rect(
                c.dnwell,
                o + 4.0,
                o + SPARE + 2.0 + gap,
                o + 12.0,
                o + SPARE + 6.0 + gap,
            ),
        ]
    };
    write("MDN.14", "good", with(deep(7.0)));
    write("MDN.14", "bad", with(deep(5.995)));

    // MDN.9: an active that does not touch a drift, closer than 4 µm to the device's.
    // The device's drift stops at o+10.5, so the distance is measured from there.
    let stray = |gap: f64| ncomp(o + 13.0, o + 10.5 + gap, o + 15.0, o + 12.5 + gap);
    write("MDN.9", "good", with(stray(5.0)));
    write("MDN.9", "bad", with(stray(3.995)));

    // MDN.5ai: a P+ active touching no N+, closer than 1 µm to a drift.
    let ptouch = |gap: f64| pcomp(o + 13.0, o + 10.5 + gap, o + 15.0, o + 12.0 + gap);
    write("MDN.5ai", "good", with(ptouch(2.0)));
    write("MDN.5ai", "bad", with(ptouch(0.995)));

    // MDN.5aii: a P+ active that *does* touch an N+, closer than 0.92 µm to a drift.  The
    // N+ it touches runs onto the drift, or MDN.9 measures that instead.
    let pbutt = |gap: f64| {
        let mut v = device_bar(o + 6.0, o + 13.0, 5.0, 5.0, false, true);
        v.extend(ncomp(o + 9.0, o + 13.5, o + 11.0 + gap, o + 14.5));
        v.extend(pcomp(o + 11.0 + gap, o + 13.5, o + 13.0 + gap, o + 14.5));
        v
    };
    write("MDN.5aii", "good", with(pbutt(2.0)));
    write("MDN.5aii", "bad", with(pbutt(0.915)));

    // MDN.5b: a P+ active closer than 0.4 µm to the source, which is the strip of active
    // the gate leaves uncovered at the near end.
    let psource = |gap: f64| pcomp(o + 2.0 - gap, o + 4.0, o + 4.0 - gap, o + 10.0);
    write("MDN.5b", "good", with(psource(1.0)));
    write("MDN.5b", "bad", with(psource(0.395)));

    // MDN.10ei: a P+ active touching no N+, closer than 0.4 µm to the gate.
    let ppoly = |gap: f64| pcomp(o + 6.0, o + 10.5 + gap, o + 8.0, o + 12.0 + gap);
    write("MDN.10ei", "good", with(ppoly(1.0)));
    write("MDN.10ei", "bad", with(ppoly(0.395)));

    // MDN.10eii: a P+ active that touches an N+, closer than 0.32 µm to the gate.  Both
    // stay 4 µm clear of the drift so MDN.9 does not claim the N+.
    let ppoly_butt = |gap: f64| {
        let y = o + 10.5 + gap;
        let mut v = pcomp(o + 5.0, y, o + 6.5, y + 1.5);
        v.extend(ncomp(o + 6.5, y, o + 7.5, y + 1.5));
        v
    };
    write("MDN.10eii", "good", with(ppoly_butt(1.0)));
    write("MDN.10eii", "bad", with(ppoly_butt(0.315)));

    // MDN.10f: a poly run that bridges two gated stretches - a tab off the gate with an
    // implant patch of its own, joined to the gate through poly the implant does not
    // cover.
    let bridge = |joined: bool| {
        let mut v = device(
            &c,
            Dev {
                poly_tab: Some(2.0),
                ..good
            },
        );
        if joined {
            v.push(rect(c.nplus, o + 6.5, o + 11.5, o + 7.5, o + 12.5));
        }
        v
    };
    write("MDN.10f", "good", bridge(false));
    write("MDN.10f", "bad", bridge(true));

    // MDN.6: device material the LDMOS marker covers and Dualgate does not.  The marker
    // grows and a drift bar goes in what it gained.
    let outside_dg = |out: bool| {
        let mut v = device(&c, good);
        if out {
            v.push(rect(c.ldmos, o + 27.0, o - 5.0, o + 34.0, o + 27.0));
            v.push(bar(o + 29.0, o + 5.0, 3.0, 3.0));
        }
        v
    };
    write("MDN.6", "good", outside_dg(false));
    write("MDN.6", "bad", outside_dg(true));

    // MDN.7: device material a Dualgate covers outside the marker.  Its Dualgate is a
    // rectangle of its own that never reaches the marker, which is what keeps MDN.7a -
    // "a Dualgate overlapping the marker and reaching past it" - out of this fixture.
    let far_device = |out: bool| {
        let mut v = device(&c, good);
        if out {
            v.push(rect(c.dualgate, o + 30.0, o + 4.0, o + 40.0, o + 14.0));
            v.push(bar(o + 32.0, o + 6.0, 4.0, 4.0));
            v.extend(ncomp(o + 33.0, o + 7.0, o + 35.0, o + 9.0));
        }
        v
    };
    write("MDN.7", "good", far_device(false));
    write("MDN.7", "bad", far_device(true));

    // MDN.7a: a Dualgate that overlaps the marker and reaches past it.
    let dg_over = |out: f64| {
        let mut v = device(&c, good);
        v.push(rect(
            c.dualgate,
            o + 20.0,
            o + 4.0,
            o + 27.0 + out,
            o + 14.0,
        ));
        v
    };
    write("MDN.7a", "good", dg_over(0.0));
    write("MDN.7a", "bad", dg_over(3.0));

    // MDN.6a: an active within 0.5 µm of Dualgate.  The ring stands between the two
    // wherever the device's own active is, and it has to - MDN.17 wants every active
    // inside a hole of it and MDN.17 again wants the ring inside Dualgate - so the ring
    // is drawn thin, hugging the device, and the active that comes close is an island
    // parked at the top of the hole.
    let hug = Dev {
        margin: 0.0,
        ring_wall: 0.2,
        ring_clear: Some(1.1),
        ..good
    };
    let near_dualgate = |top: f64| {
        let mut v = device(&c, hug);
        v.extend(ncomp(o + 5.0, o + 10.7, o + 7.0, top));
        v
    };
    write("MDN.6a", "good", near_dualgate(o + 11.0));
    write("MDN.6a", "bad", near_dualgate(o + 11.5));

    // MDN.15b: a contact on the drain that reaches off the drain's own active.
    let drain_contact = |dx: f64| {
        let mut v = device(&c, good);
        v.push(rect(
            c.contact,
            o + 16.0 + dx,
            o + 6.9,
            o + 16.22 + dx,
            o + 7.12,
        ));
        v
    };
    write("MDN.15b", "good", drain_contact(0.0));
    write("MDN.15b", "bad", drain_contact(1.9));

    // MDN.17: device material outside the guard ring's hole, in the band the marker
    // covers beyond the ring.
    // MDN.17 wants device material outside the ring but still under the marker, so its
    // device carries a wider one than the rest.
    let wide_marker = Dev {
        margin: 12.0,
        ..good
    };
    let outside_ring = |extra: Vec<GdsElement>| {
        let mut v = device(&c, wide_marker);
        v.extend(extra);
        v
    };
    write("MDN.17", "good", outside_ring(vec![]));
    // Active is device material too, and unlike a drift it answers to none of the rules
    // that ask a transistor to be whole - which is what this fixture wants outside the
    // ring, rather than a stand-in device that is not one.
    write(
        "MDN.17",
        "bad",
        outside_ring(ncomp(o + 28.0, o + 6.0, o + 32.0, o + 11.0)),
    );
}
