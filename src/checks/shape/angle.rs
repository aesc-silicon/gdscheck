// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! No forbidden edge angles on a layer (e.g. Gat.f: "45° GatPoly on Activ not allowed").
//! Run over an intersection layer such as GatPolyOverActiv so only the part crossing the
//! channel is inspected; every non-orthogonal edge (or, with the `angle` param, every edge
//! at a specific orientation) is flagged.
//! [`run`].

use crate::geom::poly_from_merged;
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;

/// Flag non-orthogonal edges of `layers[0]` (e.g. Gat.f: no 45° GatPoly over Activ — run
/// over the GatPoly∩Activ intersection so only the part crossing the channel is checked).
/// By default every edge that is not axis-aligned is forbidden; an optional `angle` param
/// (degrees) restricts the check to edges at that specific orientation (and its 180°
/// complement), with an optional `tolerance` (degrees, default 1.0).
pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] no_angle needs a layer", rule.id);
        return vec![];
    };
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);

    let forbidden = rule.num("angle"); // specific forbidden orientation
    let tol = rule.num("tolerance").unwrap_or(1.0);
    // Allowed orientations are multiples of `step` degrees; 90 (the default) permits
    // only axis-aligned edges, 45 also permits the diagonals.  GF180's ACUTE rules want
    // the latter - they allow 0, 45, 90 and -45 and flag everything else.
    let step = rule.num("step").unwrap_or(90.0);

    match forbidden {
        Some(a) => println!(
            "[{}] Checking no_angle: {} edges at {:.1}°",
            rule.id, layer.name, a
        ),
        None => println!(
            "[{}] Checking no_angle: non-orthogonal {} edges",
            rule.id, layer.name
        ),
    }

    let tile = merged.tile_dbu() as i64;
    let gmap = merged.tiles(gl, gd);
    let rid = rule.id.as_str();
    let ln = layer.name.as_str();
    // Orientation of a forbidden angle, folded into [0,180).
    let target = forbidden.map(|a| a.rem_euclid(180.0));

    gmap.par_iter()
        .flat_map_iter(move |(&(tx, ty), ps)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let mut out = Vec::new();
            for pm in ps {
                let Some(p) = poly_from_merged(pm, dbu_to_um) else {
                    continue;
                };
                for &(ax, ay, bx, by) in &p.edges {
                    let (dx, dy) = (bx - ax, by - ay);
                    if dx == 0.0 && dy == 0.0 {
                        continue;
                    }
                    let ang = dy.atan2(dx).to_degrees().rem_euclid(180.0);
                    let near =
                        |a: f64, b: f64| (a - b).abs() <= tol || (a - b).abs() >= 180.0 - tol;
                    let flag = match target {
                        Some(t) => near(ang, t),
                        None => {
                            // Not on the allowed lattice: distance to the nearest
                            // multiple of `step` exceeds the tolerance.
                            let k = (ang / step).round() * step;
                            !near(ang, k)
                        }
                    };
                    if !flag {
                        continue;
                    }
                    let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                    if !core.owns(mx / dbu_to_um, my / dbu_to_um) {
                        continue;
                    }
                    out.push(Violation::edge(
                        rid,
                        "Forbidden angle violation",
                        format!(
                            "{ln}: forbidden ({ang:.1}°) edge at \
                             ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"
                        ),
                        ax,
                        ay,
                        bx,
                        by,
                    ));
                }
            }
            out.into_iter()
        })
        .collect()
}
