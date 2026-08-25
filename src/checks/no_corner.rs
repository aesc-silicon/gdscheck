// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Forbidden corners: vertices where the boundary turns by a given angle.
//!
//! The companion to [`no_angle`](super::no_angle), and not a substitute for it — the two
//! ask different questions. `no_angle` looks at one edge and asks whether its *direction*
//! is allowed; this looks at a vertex and asks whether the *turn between two edges* is.
//! A right-angle bend is built from a 0° edge and a 90° edge, both perfectly legal
//! orientations, so no setting of `no_angle` can see it without also flagging every
//! straight orthogonal run on the die.
//!
//! GF180's `PL.6` is the rule that needs it: "90 degree bends on the COMP are not
//! allowed", i.e. poly may not turn a right angle while it is over active area.
//!
//! `value` is the forbidden turn in degrees, matched on its absolute size so that a
//! convex and a concave bend of the same sharpness both count (the reference unions
//! `corners(90)` with `corners(-90)`). `layers[1]`, if given, confines the rule to
//! corners lying inside that layer. `layer_params.outside_layer`, if given, exempts
//! corners inside that one.
//!
//! Corners are taken from the *merged* layer, so a bend that only exists because two
//! drawn shapes were butted together counts, and one that two shapes merge away does not.

use crate::layout::FlatLayout;
use crate::merge::{Core, MergedCache, TileMap, point_in_merged};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Half-side of the square sampled around a corner for the containment tests.
///
/// The reference seeds each corner with a dot and grows it by 0.1 µm — in *each*
/// direction, so the square's half-side is 0.1 — then asks whether that square is inside
/// the confining layer. A bend within 0.1 µm of the layer's own edge is therefore not
/// flagged, and sampling the square's corners rather than the vertex alone keeps that.
/// Halving this to 0.05 costs 28 markers the reference does not have, all of them bends
/// sitting just inside a COMP edge.
const PROBE_UM: f64 = 0.1;

fn key(l: &crate::pdk::Layer) -> (i16, i16) {
    (l.gds_layer as i16, l.gds_datatype as i16)
}

/// Whether every corner of the probe square around `(x, y)` lies in `map`.
fn square_inside(map: &TileMap, tile_dbu: i64, dbu_to_um: f64, x: f64, y: f64) -> bool {
    let r = PROBE_UM / dbu_to_um;
    [(-r, -r), (r, -r), (r, r), (-r, r)]
        .iter()
        .all(|(dx, dy)| point_in(map, tile_dbu, x + dx, y + dy))
}

/// Point-in-layer, tested against the bucket of the tile containing the point — where
/// that bucket's union is complete by construction.
fn point_in(map: &TileMap, tile_dbu: i64, px: f64, py: f64) -> bool {
    let t = tile_dbu as f64;
    let tile = ((px / t).floor() as i32, (py / t).floor() as i32);
    map.get(&tile)
        .is_some_and(|polys| polys.iter().any(|m| point_in_merged(px, py, m)))
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] no_corner needs a layer", rule.id);
        return vec![];
    };
    let target = rule.value.abs();
    let tol = rule.params.get("tolerance").copied().unwrap_or(1.0);

    let (gl, gd) = key(layer);
    merged.ensure(layout, gl, gd);
    let inside = rule.layers.get(1).map(key);
    if let Some(k) = inside {
        merged.ensure(layout, k.0, k.1);
    }
    let outside = rule
        .params
        .get("outside_layer")
        .map(|v| {
            (
                *v as i16,
                rule.params
                    .get("outside_layer_dt")
                    .map(|d| *d as i16)
                    .unwrap_or(0),
            )
        })
        .inspect(|k| merged.ensure(layout, k.0, k.1));

    println!(
        "[{}] Checking no_corner: {} corners at {:.1}°{}",
        rule.id,
        layer.name,
        target,
        match rule.layers.get(1) {
            Some(l) => format!(" inside {}", l.name),
            None => String::new(),
        },
    );

    let tile = merged.tile_dbu() as i64;
    let gmap = merged.tiles(gl, gd);
    let imap = inside.map(|k| merged.tiles(k.0, k.1));
    let omap = outside.map(|k| merged.tiles(k.0, k.1));
    let rid = rule.id.as_str();
    let ln = layer.name.as_str();

    let mut out = Vec::new();
    for (&(tx, ty), polys) in gmap {
        let core = Core {
            x0: tx as i64 * tile,
            y0: ty as i64 * tile,
            x1: (tx as i64 + 1) * tile,
            y1: (ty as i64 + 1) * tile,
        };
        for m in polys {
            for ring in std::iter::once(&m.outer).chain(m.holes.iter()) {
                let n = ring.len();
                if n < 3 {
                    continue;
                }
                for i in 0..n {
                    let p = ring[(i + n - 1) % n];
                    let c = ring[i];
                    let q = ring[(i + 1) % n];
                    let (ix, iy) = ((c.x - p.x) as f64, (c.y - p.y) as f64);
                    let (ox, oy) = ((q.x - c.x) as f64, (q.y - c.y) as f64);
                    if (ix == 0.0 && iy == 0.0) || (ox == 0.0 && oy == 0.0) {
                        continue; // duplicated vertex: no turn to measure
                    }
                    // Signed turn between the incoming and outgoing edge.
                    let turn = (ix * oy - iy * ox)
                        .atan2(ix * ox + iy * oy)
                        .to_degrees()
                        .abs();
                    if (turn - target).abs() > tol {
                        continue;
                    }
                    let (cx, cy) = (c.x as f64, c.y as f64);
                    if !core.contains(cx, cy) {
                        continue;
                    }
                    if imap.is_some_and(|map| !square_inside(map, tile, dbu_to_um, cx, cy)) {
                        continue;
                    }
                    if omap.is_some_and(|map| point_in(map, tile, cx, cy)) {
                        continue;
                    }
                    let (x, y) = (cx * dbu_to_um, cy * dbu_to_um);
                    out.push(Violation::point(
                        rid,
                        "Forbidden corner",
                        format!("{ln}: {turn:.1}° corner at ({x:.4}, {y:.4}) µm"),
                        x,
                        y,
                    ));
                }
            }
        }
    }
    out
}
