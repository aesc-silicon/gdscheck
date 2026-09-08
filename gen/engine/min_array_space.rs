// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Patterns for [`min_array_space`](gdscheck::checks::min_array_space): what is inside a
//! via array, asked on the shapes an array comes in.
//!
//! The rule is about the vias packed into a block, which etch and fill differently from
//! a lone pair, so the block wants a larger space than the ordinary via rule.  The
//! question every pattern here asks is which vias the block is, and the answer the
//! foundry's own deck gives is the geometric one: a row of vias attached to a block a
//! hundredth too close is, with the block's three nearest rows, a 4x4 block of its own,
//! and it violates.  An L or a U is block all the way.  Each shape is drawn at the limit
//! and a hundredth under it, the tight pair placed where the shape says.
//!
//! One pattern records a difference from that deck, deliberately.  It exempts a whole
//! blob of vias from the rule if any part of the blob is thinner than four vias - a tail
//! of vias on a block hides a tight pair in the block's own middle.  `tail` draws that,
//! and we report the pair.

use crate::helpers::{layer, library, rect, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/engine/generated/min_array_space";
/// Via side, and the legal array space it is drawn at.
const VIA: f64 = 0.26;
const SPACE: f64 = 0.36;
/// The space under test: a hundredth under the limit.
const TIGHT: f64 = 0.35;
const O: f64 = 10.0;

fn name(space: f64) -> String {
    format!("{:04}", (space * 1000.0).round() as i64)
}

/// A `cols` x `rows` block of vias with its lower-left via at (x, y), every space `sx`
/// across and `sy` up.
fn block(
    via: (i16, i16),
    x: f64,
    y: f64,
    cols: usize,
    rows: usize,
    sx: f64,
    sy: f64,
) -> Vec<gds21::GdsElement> {
    let mut v = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let (x0, y0) = (x + c as f64 * (VIA + sx), y + r as f64 * (VIA + sy));
            v.push(rect(via, x0, y0, x0 + VIA, y0 + VIA));
        }
    }
    v
}

pub fn generate(pdk: &PdkConfig) {
    let via = layer(pdk, "Via");
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let write = |file: String, elems: Vec<gds21::GdsElement>| {
        write_gz(&format!("{DIR}/{file}.gds.gz"), library("TOP", elems));
    };
    let pitch = VIA + SPACE;

    // A 6x6 block, spaced `s` in both axes.
    for s in [SPACE, TIGHT] {
        write(format!("block_{}", name(s)), block(via, O, O, 6, 6, s, s));
    }
    // A 6x6 block tight in one axis only: legal across, `s` up.  Under "both axes" any
    // tight pair violates; under "one axis" the legal axis saves it.
    write("block_x_0350".into(), block(via, O, O, 6, 6, SPACE, TIGHT));

    // A single row of six vias attached below a legal 6x6 block, the row's own spaces
    // legal and its gap to the block's bottom row `s`.
    for s in [SPACE, TIGHT] {
        let mut v = block(via, O, O, 6, 6, SPACE, SPACE);
        v.extend(block(via, O, O - VIA - s, 6, 1, SPACE, SPACE));
        write(format!("row_{}", name(s)), v);
    }
    // The same attached row, and one tight pair inside the block as well - the
    // block's third column moved a hundredth towards the second.  Two findings' worth
    // of geometry, one marker: the check reports a block once.
    {
        let mut v = block(via, O, O, 6, 6, SPACE, SPACE);
        v.extend(block(via, O, O - VIA - TIGHT, 6, 1, SPACE, SPACE));
        for e in v.iter_mut() {
            if let gds21::GdsElement::GdsBoundary(b) = e {
                let x0 = b.xy.iter().map(|p| p.x).min().unwrap();
                let col = ((x0 as f64 / 1000.0 - O) / pitch).round() as i64;
                if col == 2 && b.xy.iter().map(|p| p.y).min().unwrap() as f64 / 1000.0 >= O {
                    for p in b.xy.iter_mut() {
                        p.x -= 10;
                    }
                }
            }
        }
        write("row_inside_0350".into(), v);
    }
    // A single column of six vias attached to the right of a legal 6x6 block, its gap to
    // the block's right column `s`: the case a design placed.  With the block's three
    // nearest columns it is a 4x4 of its own, and the foundry's deck agrees.
    for s in [SPACE, TIGHT] {
        let mut v = block(via, O, O, 6, 6, SPACE, SPACE);
        v.extend(block(
            via,
            O + 6.0 * pitch - SPACE + s,
            O,
            1,
            6,
            SPACE,
            SPACE,
        ));
        write(format!("column_{}", name(s)), v);
    }

    // A tail: a legal 6x6 block with a single row of five vias leading away from its
    // right side at legal spacing, and one tight pair in the block's middle (the third
    // column moved a hundredth).  The tail is a thin part of the blob, which the foundry
    // deck takes as reason to skip the whole blob; the pair is the block's own and we
    // report it.
    {
        let mut v = block(via, O, O, 6, 6, SPACE, SPACE);
        v.extend(block(
            via,
            O + 6.0 * pitch,
            O + 2.0 * pitch,
            5,
            1,
            SPACE,
            SPACE,
        ));
        for e in v.iter_mut() {
            if let gds21::GdsElement::GdsBoundary(b) = e {
                let x0 = b.xy.iter().map(|p| p.x).min().unwrap();
                let col = ((x0 as f64 / 1000.0 - O) / pitch).round() as i64;
                if col == 2 {
                    for p in b.xy.iter_mut() {
                        p.x -= 10;
                    }
                }
            }
        }
        write("tail_0350".into(), v);
    }

    // A finger: a legal 6x6 block with a 12-wide finger of vias at the tight space in
    // both directions leading away from its right side, in line with its bottom rows.
    // Three rows deep the finger is no part of any 4x4, and the blob the pitch groups
    // is not the array; four rows deep it is a 4x4 of its own, and violates.
    for rows in [3usize, 4] {
        let mut v = block(via, O, O, 6, 6, SPACE, SPACE);
        v.extend(block(via, O + 6.0 * pitch, O, 12, rows, TIGHT, TIGHT));
        write(format!("finger_{rows}_0350"), v);
    }

    // An L: an 8x4 block, and a 4x4 block standing on its left end.  Every via is in
    // some 4x4 of the whole, so the arm is block too; the tight pair is in the arm.
    for s in [SPACE, TIGHT] {
        let mut v = block(via, O, O, 8, 4, SPACE, SPACE);
        v.extend(block(via, O, O + 4.0 * pitch, 4, 4, SPACE, s));
        write(format!("l_{}", name(s)), v);
    }
    // A U: a 12x4 base with a 4x4 arm on each end; the tight pair is in the right arm.
    for s in [SPACE, TIGHT] {
        let mut v = block(via, O, O, 12, 4, SPACE, SPACE);
        v.extend(block(via, O, O + 4.0 * pitch, 4, 4, SPACE, SPACE));
        v.extend(block(via, O + 8.0 * pitch, O + 4.0 * pitch, 4, 4, SPACE, s));
        write(format!("u_{}", name(s)), v);
    }
}
