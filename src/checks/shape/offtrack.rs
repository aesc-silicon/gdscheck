// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A region's centreline on a pitch.  A routing layer lays its wires on tracks, every
//! `value` from `offset`, and a wire is on its track when the midpoint of its extent
//! across the routing axis lies on one - whatever its width: ASAP7's M4 tracks are
//! 48 nm apart, and a wire 24 or 72 nm wide is on a track when its centre is.  A gate
//! or fin pitch is the same reading, the pitch of the gates' or fins' centres.  The
//! extent is the region's bounding box across `facing`, read whole across tiles; a
//! centre between two DBU is read exactly, in doubled DBU, and not rounded onto one.

use crate::checks::params;
use crate::geom::{Axis, on_grid};
use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

pub fn run(
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let Some(layer) = rule.layers.first() else {
        eprintln!("[{}] offtrack needs a layer", rule.id);
        return vec![];
    };
    let axis = match params::facing_axis(rule, "offtrack") {
        None => return vec![],
        Some(None) => {
            eprintln!(
                "[{}] offtrack needs `facing: x` or `facing: y`, the axis across which \
                 the tracks lie",
                rule.id
            );
            return vec![];
        }
        Some(Some(axis)) => axis,
    };
    let pitch = on_grid(rule.value / dbu_to_um, f64::round);
    if pitch < 1 {
        eprintln!(
            "[{}] Pitch {:.4} µm is smaller than 1 DBU",
            rule.id, rule.value
        );
        return vec![];
    }
    let Some(offset_um) = params::offset(rule) else {
        return vec![];
    };
    // Tracks and centres in doubled DBU: a centre between two DBU is exact, and an
    // offset may name one.
    let offset2 = 2.0 * offset_um / dbu_to_um;
    if (offset2 - offset2.round()).abs() > 1e-6 {
        eprintln!(
            "[{}] Offset {:.4} µm is not on the half-DBU grid",
            rule.id, offset_um
        );
        return vec![];
    }
    let (offset2, pitch2) = (offset2.round() as i64, 2 * pitch);
    let (gl, gd) = (layer.gds_layer as i16, layer.gds_datatype as i16);
    merged.ensure(layout, gl, gd);
    let word = match axis {
        Axis::X => "x",
        Axis::Y => "y",
    };
    println!(
        "[{}] Checking offtrack: centre {word} on tracks every {:.4} µm from {:.4} µm on layer {}",
        rule.id, rule.value, offset_um, layer.name
    );

    super::region_boxes(merged, gl, gd)
        .into_iter()
        .filter_map(|((cx, cy), (x0, y0, x1, y1))| {
            let centre2 = match axis {
                Axis::X => x0 as i64 + x1 as i64,
                Axis::Y => y0 as i64 + y1 as i64,
            };
            let rem = (centre2 - offset2).rem_euclid(pitch2);
            if rem == 0 {
                return None;
            }
            let off = rem.min(pitch2 - rem) as f64 / 2.0 * dbu_to_um;
            let centre = centre2 as f64 / 2.0 * dbu_to_um;
            let (ux, uy) = (cx * dbu_to_um, cy * dbu_to_um);
            Some(Violation::point(
                &rule.id,
                "Off-track centreline",
                format!(
                    "{}: centre {word} = {centre:.4} µm is {off:.4} µm off the tracks every \
                     {:.4} µm from {offset_um:.4} µm, at ({ux:.4}, {uy:.4}) µm",
                    layer.name, rule.value
                ),
                ux,
                uy,
            ))
        })
        .collect()
}
