// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The width scan as the checks drive it: [`width_pairs`](crate::geom::width_pairs)
//! measured tile by tile, the pinch points no pair of walls can express, and each result
//! written out as a violation.  Every rule in [`super`](super) comes through
//! [`run_width`]; the gate rules come through [`run_gate`], which is the same scan with
//! a [`WallFilter`] choosing the walls.

use super::Kind;
use crate::geom::*;
use crate::layout::FlatLayout;
use crate::merge::{Core, IntPoint, MergedCache, MergedPoly};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use rayon::prelude::*;
use std::collections::HashMap;

/// The width scan as a rule reports it: [`width_pairs`] measured, then each wall written
/// out as one violation.
#[allow(clippy::too_many_arguments)]
pub(super) fn scan_widths(
    poly: &MergedPoly,
    core: Core,
    dbu_to_um: f64,
    rule_id: &str,
    label: &str,
    // Message context: the measured layer, the rule limit (µm) and the failing
    // comparison symbol ("<", ">", "≠").
    layer: &str,
    limit_um: f64,
    cmp: &str,
    limit: Limit,
    oblique_only: bool,
    mixed: bool,
    min_run: i64,
    walls: Option<&WallFilter>,
) -> Vec<Violation> {
    width_pairs(poly, core, limit, walls, oblique_only, mixed, min_run)
        .into_iter()
        .map(|(x1, y1, x2, y2, w_dbu)| {
            let w = w_dbu * dbu_to_um;
            Violation::edge(
                rule_id,
                label,
                format!(
                    "{}: width {:.4} µm {} {:.4} µm at ({:.4}, {:.4})-({:.4}, {:.4}) µm",
                    layer,
                    w,
                    cmp,
                    limit_um,
                    x1 * dbu_to_um,
                    y1 * dbu_to_um,
                    x2 * dbu_to_um,
                    y2 * dbu_to_um
                ),
                x1 * dbu_to_um,
                y1 * dbu_to_um,
                x2 * dbu_to_um,
                y2 * dbu_to_um,
            )
        })
        .collect()
}

/// Points where a layer's own material narrows to nothing: two pieces of it meeting at
/// an isolated vertex.
///
/// Both are one shape pinched to a point — two squares corner to corner are drawn as a
/// bow-tie and merge into two polygons that touch, so the width there is zero and no pair
/// of facing edges exists to measure it between. The spacing checks deliberately send
/// this case here rather than calling it a gap of zero, so this is where it has to be
/// caught.
///
/// A shared *run* of boundary is not a pinch: two pieces drawn edge to edge are one wide
/// shape, and the width across it is whatever the scan measures.
///
/// The merge does not always hand the two pieces back as two polygons.  Two squares
/// corner to corner can come back as *one* contour that passes through the shared
/// corner twice, and a hole that reaches the outer boundary at a point does the same
/// between two rings of one polygon; either way the vertex is the same width of zero.
/// So a vertex a polygon's own rings visit twice is a
/// pinch too, and so is a vertex that sits on the inside of an edge - the tip of a notch
/// meeting a straight wall, where the merge keeps the wall as one edge and the touch has
/// no vertex of its own on that side.
pub fn pinch_points(polys: &[MergedPoly]) -> Vec<(f64, f64)> {
    // Every vertex filed once under its coordinates; a pinch is a vertex two polygons
    // share.  Trying every pair of polygons through a set intersection was quadratic in
    // the tile, and a tile of five hundred vias - none of which touch anything - paid
    // a hundred thousand intersections to learn that, on every width rule of the deck.
    let mut at: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, p) in polys.iter().enumerate() {
        for q in std::iter::once(&p.outer).chain(p.holes.iter()).flatten() {
            let owners = at.entry((q.x, q.y)).or_default();
            if owners.last() != Some(&i) {
                owners.push(i);
            }
        }
    }
    // The shared vertices of each pair, so a pair is judged once however many it shares.
    let mut shared_by: HashMap<(usize, usize), Vec<(i32, i32)>> = HashMap::new();
    for (&v, owners) in &at {
        for a in 0..owners.len() {
            for b in a + 1..owners.len() {
                shared_by.entry((owners[a], owners[b])).or_default().push(v);
            }
        }
    }
    let mut out = Vec::new();
    // Within one polygon: a vertex its own rings visit twice.
    for p in polys {
        let mut count: HashMap<(i32, i32), u32> = HashMap::new();
        for q in std::iter::once(&p.outer).chain(p.holes.iter()).flatten() {
            *count.entry((q.x, q.y)).or_default() += 1;
        }
        let mut twice: Vec<_> = count
            .into_iter()
            .filter(|&(_, n)| n >= 2)
            .map(|(v, _)| v)
            .collect();
        twice.sort_unstable();
        out.extend(twice.into_iter().map(|(x, y)| (x as f64, y as f64)));
    }
    // A vertex on the *interior* of an edge: the tip of a notch touching a straight
    // wall.  No ring visits that point twice - the wall runs straight through it - so
    // neither lookup above sees it.  Every edge is filed by the cells its box covers, and
    // a vertex asks the edges in its own cell.  A vertex whose own edge runs along the
    // edge it sits on is the end of an abutting run, not a pinch: a layer delivered as
    // core-clipped pieces meets itself that way along every tile line.
    const CELL: i64 = 4096;
    let cell = |v: i32| (v as i64).div_euclid(CELL);
    let mut edges: Vec<(IntPoint, IntPoint)> = Vec::new();
    let mut by_cell: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for p in polys {
        for ring in std::iter::once(&p.outer).chain(p.holes.iter()) {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if a == b {
                    continue;
                }
                let k = edges.len();
                edges.push((a, b));
                for cx in cell(a.x.min(b.x))..=cell(a.x.max(b.x)) {
                    for cy in cell(a.y.min(b.y))..=cell(a.y.max(b.y)) {
                        by_cell.entry((cx, cy)).or_default().push(k);
                    }
                }
            }
        }
    }
    let collinear = |u: (i128, i128), v: (i128, i128)| u.0 * v.1 - u.1 * v.0 == 0;
    let mut touched: Vec<(i32, i32)> = Vec::new();
    for p in polys {
        for ring in std::iter::once(&p.outer).chain(p.holes.iter()) {
            let n = ring.len();
            for i in 0..n {
                let v = ring[i];
                let (prev, next) = (ring[(i + n - 1) % n], ring[(i + 1) % n]);
                let own = [
                    ((v.x - prev.x) as i128, (v.y - prev.y) as i128),
                    ((next.x - v.x) as i128, (next.y - v.y) as i128),
                ];
                let Some(cands) = by_cell.get(&(cell(v.x), cell(v.y))) else {
                    continue;
                };
                for &k in cands {
                    let (a, b) = edges[k];
                    if a == v || b == v {
                        continue; // its own edge, or one that ends here
                    }
                    let d = ((b.x - a.x) as i128, (b.y - a.y) as i128);
                    let w = ((v.x - a.x) as i128, (v.y - a.y) as i128);
                    let along = w.0 * d.0 + w.1 * d.1;
                    if !collinear(d, w) || along <= 0 || along >= d.0 * d.0 + d.1 * d.1 {
                        continue; // not strictly inside this edge
                    }
                    if own.iter().any(|&o| collinear(o, d)) {
                        continue; // runs along it: abutting
                    }
                    touched.push((v.x, v.y));
                    break;
                }
            }
        }
    }
    touched.sort_unstable();
    touched.dedup();
    out.extend(touched.into_iter().map(|(x, y)| (x as f64, y as f64)));
    let mut pairs: Vec<_> = shared_by.into_iter().collect();
    pairs.sort_unstable();
    for ((i, j), mut shared) in pairs {
        let (a, b) = (
            poly_from_merged(&polys[i], 1.0),
            poly_from_merged(&polys[j], 1.0),
        );
        if let (Some(a), Some(b)) = (a, b)
            && shares_boundary_run(&a, &b, 0.5)
        {
            continue; // abutting, not pinched
        }
        shared.sort_unstable();
        out.extend(shared.into_iter().map(|(x, y)| (x as f64, y as f64)));
    }
    out
}

/// Drive a width check over the cached tiles: `limit` decides a violation,
/// `op`/`check_name`/`label` shape the log and the report.
#[allow(clippy::too_many_arguments)]
pub fn run_width(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
    check_name: &str,
    op: &str,
    label: &str,
    limit: Limit,
    oblique_only: bool,
    mixed: bool,
    min_run_dbu: i64,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let tile = merged.tile_dbu() as i64;
    // `span: narrowest`, for a maximum: a shape is too wide only where its narrowest
    // dimension exceeds the value - where a value x value square fits inside it -
    // rather than wherever two facing walls are further apart than that.  A 1 x 300 µm
    // filler stripe has a width of 1 by the first reading and of 300 by the second.
    // IHP reads AFil.a, GFil.a and Pad.a1 the first way (an opening by half the value,
    // what survives is the violation); its metal-filler and LBE maxima are the bounding
    // box's long side, `max_length`, no width at all.
    let narrowest = match rule.word("span") {
        Some("narrowest") if matches!(limit, Limit::AtMost(_)) => true,
        Some("narrowest") => {
            eprintln!(
                "[{}] {check_name}: `span: narrowest` is a maximum's reading",
                rule.id
            );
            false
        }
        Some("any") | None => false,
        Some(other) => {
            eprintln!(
                "[{}] {check_name}: span can be `any` or `narrowest`, not `{other}`",
                rule.id
            );
            false
        }
    };

    for layer in &rule.layers {
        let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
        merged.ensure(layout, gl, gd);

        println!(
            "[{}] Checking {} {} {:.2} µm on layer {} ({}/{})",
            rule.id, check_name, op, rule.value, layer.name, layer.gds_layer, layer.gds_datatype
        );
        if narrowest {
            violations.extend(narrowest_over(
                rule,
                merged,
                (gl, gd),
                layer.name.as_str(),
                dbu_to_um,
            ));
            continue;
        }

        let rid = rule.id.as_str();
        // The failing comparison is the inverse of the requirement op.
        let cmp = match op {
            ">=" => "<",
            "<=" => ">",
            _ => "≠",
        };
        let lname = layer.name.as_str();
        let limit_um = rule.value;
        let mut layer_violations: Vec<Violation> = merged
            .tiles(gl, gd)
            .par_iter()
            .flat_map_iter(|(&(tx, ty), polys)| {
                let core = Core {
                    x0: tx as i64 * tile,
                    y0: ty as i64 * tile,
                    x1: (tx as i64 + 1) * tile,
                    y1: (ty as i64 + 1) * tile,
                };
                let mut pinches: Vec<Violation> = Vec::new();
                if !oblique_only && min_run_dbu == 0 && matches!(limit, Limit::AtLeast(_)) {
                    for (px, py) in pinch_points(polys) {
                        if !core.owns(px, py) {
                            continue; // owned by the tile the point falls in
                        }
                        let (x, y) = (px * dbu_to_um, py * dbu_to_um);
                        pinches.push(Violation::point(
                            rid,
                            label,
                            format!(
                                "{lname}: width 0.0000 µm {cmp} {limit_um:.2} µm at \
                                 ({x:.4}, {y:.4}) µm — the layer pinches to a point"
                            ),
                            x,
                            y,
                        ));
                    }
                }
                let scanned: Vec<Violation> = polys
                    .iter()
                    .flat_map(|poly| {
                        scan_widths(
                            poly,
                            core,
                            dbu_to_um,
                            rid,
                            label,
                            lname,
                            limit_um,
                            cmp,
                            limit,
                            oblique_only,
                            mixed,
                            min_run_dbu,
                            None,
                        )
                    })
                    .collect();
                pinches.into_iter().chain(scanned)
            })
            .collect();

        violations.append(&mut layer_violations);
    }

    violations
}

/// The openings taken so far, by the copy's outline.
type Openings = std::sync::Mutex<HashMap<Vec<(i32, i32)>, std::sync::Arc<Vec<MergedPoly>>>>;

/// What a `span: narrowest` maximum reports: every region of the layer's opening by
/// half the value - the parts a value x value square fits inside - as one violation
/// at the region's marker.  The opening is taken per tile, where a copy is exact to
/// the halo, which is at least the value; the pieces are cut to the tile's core and
/// stitched, so a wide shape across tiles is one report whatever the tile.
fn narrowest_over(
    rule: &RuleDefinition,
    merged: &MergedCache,
    (gl, gd): (i16, i16),
    lname: &str,
    dbu_to_um: f64,
) -> Vec<Violation> {
    let tile = merged.tile_dbu() as i64;
    let radius = rule.value / dbu_to_um / 2.0;
    // The opening of a copy is the copy's alone, and a copy reaches every tile within
    // the rule's halo of it - at a 7 µm tile a 150 µm rule put a 128-point pad in four
    // hundred tiles, each opening it afresh, 14 s for one layout.  A tile opens only
    // the copies with area in its core (a shape with none there leaves nothing of its
    // opening there either), and a copy opened once is opened for every tile that
    // holds it: the openings are kept by the copy's outline.
    let cache: Openings = std::sync::Mutex::new(HashMap::new());
    let opened: crate::merge::TileMap = merged
        .tiles(gl, gd)
        .par_iter()
        .filter_map(|(&(tx, ty), polys)| {
            let (x0, y0) = (tx as i64 * tile, ty as i64 * tile);
            let (fx0, fy0, fx1, fy1) =
                (x0 as f64, y0 as f64, (x0 + tile) as f64, (y0 + tile) as f64);
            let mut wide: Vec<MergedPoly> = Vec::new();
            for poly in polys {
                if crate::merge::clipped_area_dbu(poly, fx0, fy0, fx1, fy1) <= 0.0 {
                    continue;
                }
                let key: Vec<(i32, i32)> = poly
                    .outer
                    .iter()
                    .chain(poly.holes.iter().flatten())
                    .map(|p| (p.x, p.y))
                    .collect();
                let hit = cache.lock().expect("openings").get(&key).cloned();
                let opened = match hit {
                    Some(o) => o,
                    None => {
                        let o = std::sync::Arc::new(crate::merge::opening(
                            std::slice::from_ref(poly),
                            radius,
                        ));
                        cache.lock().expect("openings").insert(key, o.clone());
                        o
                    }
                };
                wide.extend(opened.iter().cloned());
            }
            let pieces = crate::merge::clip_to_box(wide, x0, y0, x0 + tile, y0 + tile);
            (!pieces.is_empty()).then_some(((tx, ty), pieces))
        })
        .collect();
    crate::merge::stitch_regions(&opened, tile as i32)
        .into_iter()
        .map(|r| {
            let (x, y) = (r.marker.0 * dbu_to_um, r.marker.1 * dbu_to_um);
            Violation::point(
                rule.id.as_str(),
                "Maximum width violation",
                format!(
                    "{lname}: wider than {:.2} µm in every direction at ({x:.4}, {y:.4}) µm",
                    rule.value
                ),
                x,
                y,
            )
        })
        .collect()
}

/// The facing-wall width of `layers[0]` measured between the walls it shares with the
/// boundary of `layers[1]` - or, with `walls: unshared`, between the ones it does not.
/// `outside: X` (a layer param) keeps only the stretches outside `X`, `angle: bent` only
/// the 45° runs, and `length` only pairs sharing more than that run.
///
/// A gate has two kinds of wall.  The ones the poly brought with it stand across the
/// channel and the distance between them is the gate's length; the ones the active cut
/// stand at the ends and the distance between them is the transistor's width.  Both are
/// widths of one region, and which walls take part is the whole difference - so the rule
/// names the region and the reference, and the [`WallFilter`] cuts every pair the scan
/// finds to the stretch along which both walls sit on (or off) the reference boundary.
/// A poly stripe over a channel mask shares its two long walls with the mask's boundary
/// exactly where they cross the active, and the channel width, bounded by active edges
/// that are no walls of the poly, never enters; a body whose ends the active cut has its
/// channel-length walls off the active's boundary.
///
/// Measured per tile from that tile's copies of both layers, and owned by the stretch's
/// own midpoint - so a stripe that crosses many actives reports each gate from the tile
/// it falls in, whatever the stripe's own midpoint is.
pub fn run_gate(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let name = kind.gate_name();
    let (Some(body), Some(reference)) = (rule.layers.first(), rule.layers.get(1)) else {
        eprintln!("[{}] {name} needs a region and a reference layer", rule.id);
        return vec![];
    };
    let on = match crate::checks::params::mode(rule, name, "walls") {
        Ok(None) | Ok(Some("shared")) => true,
        Ok(Some("unshared")) => false,
        Ok(Some(other)) => {
            eprintln!(
                "[{}] {name}: walls must be `shared` or `unshared`, not `{other}`",
                rule.id
            );
            return vec![];
        }
        Err(crate::checks::params::NotAWord) => return vec![],
    };
    let Some(bent_only) = crate::checks::params::bent_only(rule, name) else {
        return vec![];
    };
    let min_run = crate::checks::params::min_run(rule, dbu_to_um);
    let outside = match (rule.num("outside"), rule.num("outside_dt")) {
        (Some(l), Some(d)) => Some((l as i16, d as i16)),
        _ => None,
    };
    let (bl, bd) = (body.gds_layer as i16, body.gds_datatype as i16);
    let (rl, rd) = (reference.gds_layer as i16, reference.gds_datatype as i16);
    merged.ensure(layout, bl, bd);
    merged.ensure(layout, rl, rd);
    if let Some((ol, od)) = outside {
        merged.ensure(layout, ol, od);
    }
    println!(
        "[{}] Checking {name} {} {:.2} µm of {} between its walls {} with the boundary of {}",
        rule.id,
        kind.op(),
        rule.value,
        body.name,
        if on { "shared" } else { "unshared" },
        reference.name
    );
    let limit = kind.limit(rule.value, dbu_to_um);
    let cmp = kind.cmp();
    let label = match kind {
        Kind::Min => "Minimum gate-length violation",
        Kind::Max => "Maximum gate-length violation",
        Kind::Exact => "Exact gate-length violation",
    };
    let tile = merged.tile_dbu() as i64;
    let bmap = merged.tiles(bl, bd);
    let rmap = merged.tiles(rl, rd);
    let omap = outside.map(|(ol, od)| merged.tiles(ol, od));
    let none: Vec<MergedPoly> = Vec::new();
    let rid = rule.id.as_str();
    let bname = body.name.as_str();
    let limit_um = rule.value;
    bmap.par_iter()
        .flat_map_iter(move |(&(tx, ty), polys)| {
            let core = Core {
                x0: tx as i64 * tile,
                y0: ty as i64 * tile,
                x1: (tx as i64 + 1) * tile,
                y1: (ty as i64 + 1) * tile,
            };
            let mut out = Vec::new();
            // With no reference here nothing is shared with its boundary, and everything
            // is unshared: an `unshared` rule still measures the body's plain width.
            let refs = rmap.get(&(tx, ty)).unwrap_or(&none);
            if on && refs.is_empty() {
                return out.into_iter();
            }
            let excluded = omap.map(|m| m.get(&(tx, ty)).unwrap_or(&none).as_slice());
            let walls = WallFilter::new(refs, on, excluded);
            // A pinch is a width of zero at a vertex; it counts where the filter would
            // keep that point.  A bent-only rule reads 45° runs and a vertex has none, and
            // neither has one a required run.
            if kind == Kind::Min && !bent_only && min_run == 0 {
                for (px, py) in pinch_points(polys) {
                    if !core.owns(px, py) || !walls.keeps_point(px as i32, py as i32) {
                        continue;
                    }
                    let (x, y) = (px * dbu_to_um, py * dbu_to_um);
                    out.push(Violation::point(
                        rid,
                        label,
                        format!(
                            "{bname}: width 0.0000 µm {cmp} {limit_um:.2} µm at ({x:.4}, \
                             {y:.4}) µm — the layer pinches to a point"
                        ),
                        x,
                        y,
                    ));
                }
            }
            for p in polys {
                out.extend(scan_widths(
                    p,
                    core,
                    dbu_to_um,
                    rid,
                    label,
                    bname,
                    limit_um,
                    cmp,
                    limit,
                    bent_only,
                    // The gate's width is between two walls of one kind; a chamfer
                    // facing a straight wall has no stretch to cut to a boundary.
                    false,
                    min_run,
                    Some(&walls),
                ));
            }
            out.into_iter()
        })
        .collect()
}
