// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Residual: what a rule forbids outright.  An operation over the rule's layers has to
//! come out empty, and every region it leaves is a violation.  One check, `forbidden`,
//! read six ways by its `op` param:
//!
//! - none: the layers themselves.  A derived layer that *is* the error - Ant.i's
//!   `pactiv_con ∩ Recog.diode − Recog.esd − (NWell ∪ PWell.block)` is the set of
//!   p-diodes sitting in a PWell - or a layer the flow does not support at all.
//! - `uncovered`: the part of `layers[0]` the union of the other layers leaves bare
//!   (Cnt.g, "Cont within Activ or GatPoly").  `scope: polygon` reports the whole
//!   polygon that is not entirely inside instead of its bare part.
//! - `overlap`: the intersection of all layers (Cnt.j, "Cont on GatPoly over Activ").
//! - `apart`: regions of `layers[0]` touching none of the other layers (MIM.h, every
//!   MIM cap needs its via).  With `rows` and `cols`, regions holding no array of the
//!   partner that large (MT30.8, a 2×2 of vias on thick top metal), see [`mod@array`].
//! - `touching`: regions of `layers[0]` touching any of the other layers.
//! - `beyond`: every drawn shape past the outer edge of `layers[0]`, over the whole
//!   layout, less the rule's `ignore` list (Seal.l, nothing past the edge seal).
//!
//! A rule with a `text` and a `label` layer param exempts the regions that carry a text
//! matching it (`isolbox` on the TEXT layer): a marked structure is the designer's
//! word that the region is meant.
//!
//! Every reading but `beyond` is a derived layer registered on the [`MergedCache`] for
//! the rule's lifetime and read as stitched regions, so a region spanning tiles is
//! reported once and no layer is unioned globally - the same path a deck's own derived
//! layers take.  `beyond` reads the raw layout, since it looks at every layer there is.

pub mod array;

use super::params::{NotAWord, mode};
use crate::layout::FlatLayout;
use crate::merge::{Core, MergedPoly, SharedCache, VirtualOp, merge_boundaries};
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Bare,
    Uncovered,
    Overlap,
    Apart,
    Touching,
    Beyond,
}

impl Op {
    fn parse(rule: &RuleDefinition) -> Option<Op> {
        match mode(rule, "forbidden", "op") {
            Ok(None) => Some(Op::Bare),
            Ok(Some("uncovered")) => Some(Op::Uncovered),
            Ok(Some("overlap")) => Some(Op::Overlap),
            Ok(Some("apart")) => Some(Op::Apart),
            Ok(Some("touching")) => Some(Op::Touching),
            Ok(Some("beyond")) => Some(Op::Beyond),
            Ok(Some(other)) => {
                eprintln!(
                    "[{}] forbidden: op can be `uncovered`, `overlap`, `apart`, `touching` \
                     or `beyond`, not `{other}`",
                    rule.id
                );
                None
            }
            Err(NotAWord) => None,
        }
    }
}

/// Whether the rule reads every layer of the layout, not only the ones it names: a
/// `beyond` looks past the boundary on any layer there is, so the layout has to be
/// flattened whole for it and a derived layer built per tile is out of its sight.
pub fn whole_layout(rule: &RuleDefinition) -> bool {
    rule.check == "forbidden" && rule.word("op") == Some("beyond")
}

/// The derived layers one rule registers, keyed from the bottom of the range: a deck's
/// own derived layers live from 30000 up and the labelled chain's from -15536, and a
/// rule takes a handful.  Rules run one after another and each evicts its keys at the
/// end, so the next rule starts over at the bottom.
struct Scratch {
    next: i16,
    keys: Vec<(i16, i16)>,
}

impl Scratch {
    fn new() -> Self {
        Scratch {
            next: i16::MIN,
            keys: Vec::new(),
        }
    }

    fn key(&mut self) -> (i16, i16) {
        let k = (self.next, 0);
        self.next += 1;
        self.keys.push(k);
        k
    }
}

/// A text layer and the label pattern on it.
type Exemption = ((i16, i16), String);

/// The rule's exemption, if it has one: `text` names the label, the `label` layer param
/// where labels are drawn.  `Err` when only one of the two is given.
fn exemption(rule: &RuleDefinition) -> Result<Option<Exemption>, ()> {
    let layer = match (rule.num("label"), rule.num("label_dt")) {
        (Some(l), Some(d)) => Some((l as i16, d as i16)),
        _ => None,
    };
    match (layer, rule.text.as_deref()) {
        (Some(layer), Some(text)) if !text.is_empty() => Ok(Some((layer, text.to_string()))),
        (None, None) | (None, Some("")) => Ok(None),
        _ => {
            eprintln!(
                "[{}] forbidden: an exemption needs both `text` and a `label` layer param",
                rule.id
            );
            Err(())
        }
    }
}

/// `base` less the regions carrying the exemption label, as a derived key; `base`
/// itself without one.
fn exempt(
    merged: &SharedCache,
    base: (i16, i16),
    exemption: &Option<Exemption>,
    scratch: &mut Scratch,
) -> (i16, i16) {
    let Some((text_layer, pattern)) = exemption else {
        return base;
    };
    let tagged = scratch.key();
    merged.register_virtual(
        tagged,
        VirtualOp::WithText,
        vec![base, *text_layer],
        Some(pattern.clone()),
    );
    let kept = scratch.key();
    merged.register_virtual(
        kept,
        VirtualOp::NotInteracting(None, None),
        vec![base, tagged],
        None,
    );
    kept
}

/// One point violation per stitched region of `key`.
fn report(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
    key: (i16, i16),
    label: &str,
    descr: &str,
) -> Vec<Violation> {
    merged
        .regions(layout, key.0, key.1)
        .iter()
        .map(|r| {
            let (x, y) = (r.marker.0 * dbu_to_um, r.marker.1 * dbu_to_um);
            Violation::point(
                &rule.id,
                label,
                format!("{descr} at ({x:.4}, {y:.4}) µm"),
                x,
                y,
            )
        })
        .collect()
}

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let Some(op) = Op::parse(rule) else {
        return vec![];
    };
    let polygon = match mode(rule, "forbidden", "scope") {
        Ok(None) | Ok(Some("part")) => false,
        Ok(Some("polygon")) => true,
        Ok(Some(other)) => {
            eprintln!(
                "[{}] forbidden: scope can be `part` or `polygon`, not `{other}`",
                rule.id
            );
            return vec![];
        }
        Err(NotAWord) => return vec![],
    };
    if polygon && op != Op::Uncovered {
        eprintln!(
            "[{}] forbidden: `scope: polygon` reads with `op: uncovered` only",
            rule.id
        );
        return vec![];
    }
    let needed = match op {
        Op::Bare | Op::Beyond => 1,
        _ => 2,
    };
    if rule.layers.len() < needed {
        eprintln!(
            "[{}] forbidden: this op needs at least {needed} layer(s)",
            rule.id
        );
        return vec![];
    }
    if op == Op::Beyond {
        return beyond(rule, layout, dbu_to_um);
    }
    let array = (rule.num("rows"), rule.num("cols"));
    if array != (None, None) {
        if op != Op::Apart {
            eprintln!(
                "[{}] forbidden: `rows` and `cols` read with `op: apart` only",
                rule.id
            );
            return vec![];
        }
        let (rows, cols) = (
            array.0.unwrap_or(2.0) as usize,
            array.1.unwrap_or(2.0) as usize,
        );
        return array::run(rule, layout, dbu_to_um, merged, rows, cols);
    }
    let Ok(exemption) = exemption(rule) else {
        return vec![];
    };

    let key = |i: usize| {
        (
            rule.layers[i].gds_layer as i16,
            rule.layers[i].gds_datatype as i16,
        )
    };
    let names = |from: usize, sep: &str| {
        rule.layers[from..]
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>()
            .join(sep)
    };
    let mut scratch = Scratch::new();
    let mut out = Vec::new();

    if op == Op::Bare {
        for (i, layer) in rule.layers.iter().enumerate() {
            let k = key(i);
            if merged.is_edge_layer(k) {
                out.extend(forbidden_edges(
                    rule,
                    layout,
                    dbu_to_um,
                    merged,
                    k,
                    &layer.name,
                ));
                continue;
            }
            println!(
                "[{}] Checking forbidden region on layer {} ({}/{})",
                rule.id, layer.name, layer.gds_layer, layer.gds_datatype
            );
            let k = exempt(merged, k, &exemption, &mut scratch);
            let descr = format!("{} present", layer.name);
            out.extend(report(
                rule,
                layout,
                dbu_to_um,
                merged,
                k,
                "Forbidden region",
                &descr,
            ));
        }
    } else {
        let target = key(0);
        let rest: Vec<(i16, i16)> = (1..rule.layers.len()).map(key).collect();
        // The selections read one partner: the others joined, when there are several.
        let partner = |scratch: &mut Scratch| -> (i16, i16) {
            if rest.len() == 1 {
                return rest[0];
            }
            let u = scratch.key();
            merged.register_virtual(u, VirtualOp::Union, rest.clone(), None);
            u
        };
        let (vop, sources, label, descr) = match op {
            Op::Uncovered if !polygon => (
                VirtualOp::Difference,
                std::iter::once(target)
                    .chain(rest.iter().copied())
                    .collect(),
                "Coverage violation",
                format!(
                    "{} not covered by {}",
                    rule.layers[0].name,
                    names(1, " or ")
                ),
            ),
            Op::Uncovered => (
                VirtualOp::NotInside,
                vec![target, partner(&mut scratch)],
                "Coverage violation",
                format!("{} not inside {}", rule.layers[0].name, names(1, " or ")),
            ),
            Op::Overlap => (
                VirtualOp::Intersection,
                std::iter::once(target)
                    .chain(rest.iter().copied())
                    .collect(),
                "Forbidden overlap",
                format!("{} not allowed", names(0, " over ")),
            ),
            Op::Apart => (
                VirtualOp::NotInteracting(None, None),
                vec![target, partner(&mut scratch)],
                "Missing required overlap",
                format!("{} has no {}", rule.layers[0].name, names(1, "/")),
            ),
            Op::Touching => (
                VirtualOp::Interacting(None, None),
                vec![target, partner(&mut scratch)],
                "Forbidden contact",
                format!("{} touches {}", rule.layers[0].name, names(1, "/")),
            ),
            Op::Bare | Op::Beyond => unreachable!(),
        };
        println!("[{}] Checking forbidden: {descr}", rule.id);
        let k = scratch.key();
        merged.register_virtual(k, vop, sources, None);
        let k = exempt(merged, k, &exemption, &mut scratch);
        out = report(rule, layout, dbu_to_um, merged, k, label, &descr);
    }
    // The derived layers were this rule's; the cache has no second reader for them.
    for k in scratch.keys {
        merged.evict(k.0, k.1);
    }
    out
}

/// An edge layer says the same thing about a boundary: GF180's PP.11 forbids a butting
/// Pplus/NCOMP edge within 0.43 um of a well edge, and what it forbids is the segment,
/// which no region carries.
fn forbidden_edges(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
    key: (i16, i16),
    name: &str,
) -> Vec<Violation> {
    println!(
        "[{}] Checking forbidden edges on edge layer {}",
        rule.id, name
    );
    merged.ensure_edges(layout, key);
    let tile = merged.tile_dbu() as i64;
    let mut out = Vec::new();
    // An edge layer is a bag of segments and the ops that build one may put the same
    // segment in it more than once - a `join` of two selections that both reach it, a
    // boolean whose sources overlap. The tile a segment is owned by dedups it across
    // tiles; this dedups it within one. Reporting the same wall twice is not a second
    // violation, and it is measurable: NP.11 emitted 26 markers for 10 segments and
    // O.PL.ORT 725 for 367, in both cases exactly the reference's count once folded.
    let mut seen: HashSet<(i32, i32, i32, i32)> = HashSet::new();
    for (&(tx, ty), edges) in merged.edges(key).iter() {
        let core = Core {
            x0: tx as i64 * tile,
            y0: ty as i64 * tile,
            x1: (tx as i64 + 1) * tile,
            y1: (ty as i64 + 1) * tile,
        };
        for e in edges {
            let (mx, my) = e.midpoint();
            if !core.contains(mx, my) {
                continue; // owned by the tile the segment's middle falls in
            }
            if !seen.insert((e.a.x, e.a.y, e.b.x, e.b.y)) {
                continue; // the same segment, reached by more than one route
            }
            let (ax, ay) = (e.a.x as f64 * dbu_to_um, e.a.y as f64 * dbu_to_um);
            let (bx, by) = (e.b.x as f64 * dbu_to_um, e.b.y as f64 * dbu_to_um);
            out.push(Violation::edge(
                &rule.id,
                "Forbidden edge",
                format!("{name} present at ({ax:.4}, {ay:.4})-({bx:.4}, {by:.4}) µm"),
                ax,
                ay,
                bx,
                by,
            ));
        }
    }
    out
}

/// Twice the area of a merged region's outer contour, in DBU², to pick the ring among
/// any fragments the boundary layer carries.
fn outer_area2(m: &MergedPoly) -> i128 {
    let pts = &m.outer;
    let n = pts.len();
    let mut s = 0i128;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        s += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    s.abs()
}

/// Every shape on every other layer past the outer edge of `layers[0]`.  The boundary
/// is usually drawn as many segments; merged, its largest region is the ring and the
/// bounding box of that region's outer contour the edge nothing may cross.  A shape is
/// reported once, at the first vertex found outside.
fn beyond(rule: &RuleDefinition, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation> {
    let boundary = &rule.layers[0];
    let key = (boundary.gds_layer as i16, boundary.gds_datatype as i16);
    println!(
        "[{}] Checking every shape lies inside {}",
        rule.id, boundary.name
    );
    let merged = merge_boundaries(layout.get(key.0, key.1));
    let Some(outer) = merged
        .iter()
        .max_by_key(|m| outer_area2(m))
        .map(|m| &m.outer)
    else {
        return vec![];
    };
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in outer {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let ignore: HashSet<(i16, i16)> = rule
        .ignore
        .iter()
        .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        .collect();
    let mut out = vec![];
    for b in layout.all_except(key.0, key.1) {
        if ignore.contains(&(b.layer, b.datatype)) {
            continue;
        }
        let outside =
            b.xy.iter()
                .find(|p| p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1);
        if let Some(p) = outside {
            let (vx, vy) = (p.x as f64 * dbu_to_um, p.y as f64 * dbu_to_um);
            out.push(Violation::point(
                &rule.id,
                "Structure outside boundary",
                format!(
                    "shape on GDS layer {}/{} outside {} at ({vx:.4}, {vy:.4}) µm",
                    b.layer, b.datatype, boundary.name
                ),
                vx,
                vy,
            ));
        }
    }
    out
}
