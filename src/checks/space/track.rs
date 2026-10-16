// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Two wires on adjacent tracks, read along the tracks: how far they run alongside each
//! other, and how far apart their tips on one side are.  ASAP7's M4-M7 route on tracks
//! and may not bend (AUX.3), so a wire is a rectangle and is read whole by its extent,
//! across tiles; a region that is not a rectangle is the bend's business and is left
//! out here.
//!
//! - `min_track_run`: two wires that run alongside each other at all run alongside for
//!   at least the value - ASAP7's M4.S.5, 44 nm.
//! - `min_tip_stagger`: of two wires that run alongside each other, the tips on one side
//!   are aligned or at least the value apart along the track - ASAP7's M4.S.4, 40 nm.
//!   Aligned tips are the manual's own drawing of two wires on adjacent tracks (its
//!   M4.S.1), so a stagger of nothing is clean.
//!
//! `facing` names the axis across which the two wires' long walls face each other - `y`
//! for a layer that routes along x - and `within` the gap under which they are on
//! adjacent tracks: three widths, the least gap a wire fits into with its space on
//! either side.  Two minimum-width wires on neighbouring tracks are a width apart and
//! two tracks apart three widths; a wire three widths wide and the nearest wire beside
//! it are two widths apart, with no track free between them, and so are adjacent too.
//! Two wires that do not run alongside at all are tip to tip on adjacent tracks, which a
//! corner spacing reads.

use super::super::params::facing_axis;
use crate::geom::{Axis, Limit, on_grid};
use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

/// Which reading a rule asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The run two wires share.
    Run,
    /// The offset between their tips on one side.
    Stagger,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Run => "min_track_run",
            Kind::Stagger => "min_tip_stagger",
        }
    }
}

/// A wire in track coordinates, DBU: `lo`..`hi` along the track, `bot`..`top` across.
#[derive(Clone, Copy, Debug)]
struct Wire {
    lo: i64,
    hi: i64,
    bot: i64,
    top: i64,
}

/// The side of a bucket in the pair index, in µm: a few tracks' worth of wires.
const BUCKET_UM: f64 = 2.0;

pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let name = kind.name();
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] {name} needs a layer", rule.id);
        return vec![];
    };
    let axis = match facing_axis(rule, name) {
        None => return vec![],
        Some(None) => {
            eprintln!(
                "[{}] {name} needs `facing: x` or `facing: y`, the axis across which \
                 the wires on adjacent tracks face each other",
                rule.id
            );
            return vec![];
        }
        Some(Some(axis)) => axis,
    };
    let Some(within_um) = rule.num("within") else {
        eprintln!(
            "[{}] {name} needs `within`, the gap under which two wires are on adjacent \
             tracks",
            rule.id
        );
        return vec![];
    };
    let within = on_grid(within_um / dbu_to_um, f64::round);
    let limit = Limit::at_least(rule.value, dbu_to_um);
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);
    let word = match axis {
        Axis::X => "x",
        Axis::Y => "y",
    };
    println!(
        "[{}] Checking {name} >= {:.4} µm between wires facing across {word} under \
         {within_um:.4} µm on layer {}",
        rule.id, rule.value, layer.name
    );

    // Along the track and across it.
    let wires: Vec<Wire> = super::super::shape::region_rects(merged, gl, gd)
        .into_iter()
        .map(|(_, (x0, y0, x1, y1))| {
            let (x0, y0, x1, y1) = (x0 as i64, y0 as i64, x1 as i64, y1 as i64);
            match axis {
                Axis::Y => Wire {
                    lo: x0,
                    hi: x1,
                    bot: y0,
                    top: y1,
                },
                Axis::X => Wire {
                    lo: y0,
                    hi: y1,
                    bot: x0,
                    top: x1,
                },
            }
        })
        .collect();
    // A point in layout µm from one along the track and one across it.
    let point = |along: f64, across: f64| {
        let (x, y) = match axis {
            Axis::Y => (along, across),
            Axis::X => (across, along),
        };
        (x * dbu_to_um, y * dbu_to_um)
    };

    // Every wire filed under each bucket its box touches.  A pair is read from its lower
    // wire, in the one bucket that holds the point where the run starts on the upper
    // wire's lower wall - a point of both the upper wire and the lower wire's query.
    let bucket = on_grid(BUCKET_UM / dbu_to_um, f64::round).max(1);
    let cell = |v: i64| v.div_euclid(bucket);
    let mut index: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
    for (i, w) in wires.iter().enumerate() {
        for a in cell(w.lo)..=cell(w.hi) {
            for c in cell(w.bot)..=cell(w.top) {
                index.entry((a, c)).or_default().push(i as u32);
            }
        }
    }

    wires
        .par_iter()
        .flat_map_iter(|a| {
            // The upper wire's lower wall lies in (top, top + within): above the lower
            // wire, and under the pitch.
            let (q0, q1) = (a.top + 1, a.top + within - 1);
            let mut out = Vec::new();
            if q1 < q0 {
                return out.into_iter();
            }
            for ca in cell(a.lo)..=cell(a.hi) {
                for cc in cell(q0)..=cell(q1) {
                    let Some(ids) = index.get(&(ca, cc)) else {
                        continue;
                    };
                    for &j in ids {
                        let b = &wires[j as usize];
                        if b.bot < q0 || b.bot > q1 {
                            continue;
                        }
                        let (r0, r1) = (a.lo.max(b.lo), a.hi.min(b.hi));
                        if r1 <= r0 || (cell(r0), cell(b.bot)) != (ca, cc) {
                            continue;
                        }
                        let mid = (a.top + b.bot) as f64 / 2.0;
                        match kind {
                            Kind::Run => {
                                let run = r1 - r0;
                                if !limit.broken_by(run) {
                                    continue;
                                }
                                let (x1, y1) = point(r0 as f64, mid);
                                let (x2, y2) = point(r1 as f64, mid);
                                out.push(Violation::edge(
                                    &rule.id,
                                    "Minimum parallel run on adjacent tracks",
                                    format!(
                                        "{}: wires on adjacent tracks run alongside for \
                                         {:.4} µm < {:.4} µm",
                                        layer.name,
                                        run as f64 * dbu_to_um,
                                        rule.value
                                    ),
                                    x1,
                                    y1,
                                    x2,
                                    y2,
                                ));
                            }
                            Kind::Stagger => {
                                for (ta, tb) in [(a.lo, b.lo), (a.hi, b.hi)] {
                                    let off = (ta - tb).abs();
                                    if off == 0 || !limit.broken_by(off) {
                                        continue;
                                    }
                                    let (x1, y1) = point(ta as f64, mid);
                                    let (x2, y2) = point(tb as f64, mid);
                                    out.push(Violation::edge(
                                        &rule.id,
                                        "Minimum tip stagger on adjacent tracks",
                                        format!(
                                            "{}: tips of wires on adjacent tracks that run \
                                             alongside are {:.4} µm apart < {:.4} µm",
                                            layer.name,
                                            off as f64 * dbu_to_um,
                                            rule.value
                                        ),
                                        x1,
                                        y1,
                                        x2,
                                        y2,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            out.into_iter()
        })
        .collect()
}
