// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ASAP7 pattern builders, in the spirit of the `*_pattern` functions in
//! `crate::helpers` but sized per rule. The shared patterns draw squares and narrow
//! both axes at once; ASAP7's routing metals give each axis its own limit, and its
//! area and length-class rules catch squares small enough for a width or space limit.
//! Each builder here changes one dimension per shape, so a pattern's violations all
//! belong to the rule it was drawn for. Every size is in nm.

use super::nm;
use crate::helpers::{poly, rect};
use gds21::GdsElement;

/// Room between the shapes of one pattern: past every spacing rule they could answer
/// to, up to M8/M9's 120 nm for the widest wire a pattern draws.
pub const ROOM: f64 = 500.0;

/// A `w`×`h` box with its lower-left corner at (x, y).
pub fn bx(layer: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> GdsElement {
    rect(layer, nm(x), nm(y), nm(x + w), nm(y + h))
}

/// A polygon through `pts`, closed for you.
pub fn pg(layer: (i16, i16), pts: &[(f64, f64)]) -> GdsElement {
    let pts: Vec<_> = pts.iter().map(|&(x, y)| (nm(x), nm(y))).collect();
    poly(layer, &pts)
}

/// One box per `(w, h)` in `sizes`, stacked upward from (x, y) with [`ROOM`] between.
/// Lead with the box exactly at the limit, then the ones a DBU under it.
pub fn boxes(layer: (i16, i16), sizes: &[(f64, f64)], x: f64, y: f64) -> Vec<GdsElement> {
    boxes_apart(layer, sizes, ROOM, x, y)
}

/// [`boxes`] with `room` between them, for shapes wide enough to want more.
pub fn boxes_apart(
    layer: (i16, i16),
    sizes: &[(f64, f64)],
    room: f64,
    x: f64,
    y: f64,
) -> Vec<GdsElement> {
    let mut top = y;
    sizes
        .iter()
        .map(|&(w, h)| {
            let b = bx(layer, x, top, w, h);
            top += h + room;
            b
        })
        .collect()
}

/// One pair of `(w, h)` boxes per entry in `gaps`, the two split by that gap along x
/// (`along_x`) or y, the pairs stacked upward from (x, y) with [`ROOM`] between.
/// Lead with the gap exactly at the limit, then the ones a DBU under it.
pub fn gap_pairs(
    layer: (i16, i16),
    (w, h): (f64, f64),
    along_x: bool,
    gaps: &[f64],
    x: f64,
    y: f64,
) -> Vec<GdsElement> {
    let mut top = y;
    let mut out = vec![];
    for &gap in gaps {
        out.push(bx(layer, x, top, w, h));
        if along_x {
            out.push(bx(layer, x + w + gap, top, w, h));
            top += h;
        } else {
            out.push(bx(layer, x, top + h + gap, w, h));
            top += 2.0 * h + gap;
        }
        top += ROOM;
    }
    out
}

/// One `lower` box under one `upper` box per entry in `gaps`, the gap between them
/// along y and the lower box shifted `dx` right of the upper; the pairs stacked upward
/// from (x, y) with `room` between. The two boxes' sizes pick the classes of the edges
/// that face across the gap: a tip under a long side, two tips, two sides.
#[allow(clippy::too_many_arguments)]
pub fn stacked_pairs(
    layer: (i16, i16),
    lower: (f64, f64),
    upper: (f64, f64),
    dx: f64,
    gaps: &[f64],
    room: f64,
    x: f64,
    y: f64,
) -> Vec<GdsElement> {
    let mut top = y;
    let mut out = vec![];
    for &gap in gaps {
        out.push(bx(layer, x + dx, top, lower.0, lower.1));
        out.push(bx(layer, x, top + lower.1 + gap, upper.0, upper.1));
        top += lower.1 + gap + upper.1 + room;
    }
    out
}

/// One pair of `(w, h)` boxes per `(dx, dy)` in `offsets`, the second up and to the
/// right of the first by that much clear of it, so the two face each other nowhere and
/// only their corners meet across the gap; the pairs stacked upward from (x, y) with
/// [`ROOM`] between.
pub fn corner_pairs(
    layer: (i16, i16),
    (w, h): (f64, f64),
    offsets: &[(f64, f64)],
    x: f64,
    y: f64,
) -> Vec<GdsElement> {
    let mut top = y;
    let mut out = vec![];
    for &(dx, dy) in offsets {
        out.push(bx(layer, x, top, w, h));
        out.push(bx(layer, x + w + dx, top + h + dy, w, h));
        top += 2.0 * h + dy + ROOM;
    }
    out
}
