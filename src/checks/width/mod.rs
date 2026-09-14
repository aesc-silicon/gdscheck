// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Width: the perpendicular span of material between two facing walls of one merged
//! region.
//!
//! Three rules read it - at least, at most, exactly - and the gate length reads it under
//! a mask.  All are one measurement, the facing-wall scan in
//! [`crate::geom::width_pairs`], driven over the tiles by [`scan::run_width`]; what a rule
//! adds is its bound and which passes of the scan it wants.  A plain rule wants the
//! axis-aligned sweeps and the oblique pass; one restricted to 45° runs (`angle: bent`)
//! wants the oblique pass alone, since the axis-aligned material is the plain rule's
//! business; a maximum or an exact width leaves out the mixed pass, because a chamfer
//! facing a straight wall has no single width to be too large or unequal.

pub mod scan;

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Which bound a width rule puts on the span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// No span narrower than the value (`min_width`).
    Min,
    /// No span wider than the value (`max_width`).
    Max,
    /// Every span equal to the value (`exact_width`): fixed-size features such as vias.
    Exact,
}

impl Kind {
    /// The width check's name, as the deck spells it.
    pub fn width_name(self) -> &'static str {
        match self {
            Kind::Min => "min_width",
            Kind::Max => "max_width",
            Kind::Exact => "exact_width",
        }
    }

    /// The requirement, as the log prints it.
    fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
            Kind::Exact => "==",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Min => "Minimum width violation",
            Kind::Max => "Maximum width violation",
            Kind::Exact => "Exact width violation",
        }
    }
}

/// Run one width rule.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    // An edge layer carries no region to scan: the pairing form in `edge_distance` reads
    // those, for the two bounds it knows.
    if super::edge_distance::on_edge_layers(rule, merged, kind.width_name()) {
        return match kind {
            Kind::Min => super::edge_distance::run_width(rule, layout, dbu_to_um, merged),
            Kind::Max => super::edge_distance::run_max_width(rule, layout, dbu_to_um, merged),
            Kind::Exact => {
                eprintln!(
                    "[{}] {}: not defined on an edge layer",
                    rule.id,
                    kind.width_name()
                );
                vec![]
            }
        };
    }
    let limit = rule.value / dbu_to_um;
    // Half a DBU either way: the limit is a µm value converted to the grid, and the
    // spans are integers, so anything closer than that is conversion noise.
    let viol: Box<dyn Fn(f64) -> bool + Sync> = match kind {
        Kind::Min => Box::new(move |w| w < limit - 0.5),
        Kind::Max => Box::new(move |w| w > limit + 0.5),
        Kind::Exact => Box::new(move |w| (w - limit).abs() > 0.5),
    };
    // `angle: bent` reads the 45° runs alone - the axis-aligned material is a plain
    // rule's business - and only where the run is long enough to be a trace rather than
    // a chamfer, `bent_length` µm.
    let bent = match rule.str_params.get("angle").map(String::as_str) {
        None => false,
        Some("bent") => true,
        Some(other) => {
            eprintln!(
                "[{}] {}: angle can only be `bent`, not `{other}`",
                rule.id,
                kind.width_name()
            );
            return vec![];
        }
    };
    let min_run_dbu = if bent {
        rule.params.get("bent_length").copied().unwrap_or(0.5) / dbu_to_um
    } else {
        0.5
    };
    // A chamfer facing a straight wall has no single width to be too large or unequal,
    // so only a minimum reads the mixed pass.
    let (oblique_only, mixed) = (bent, kind == Kind::Min && !bent);
    scan::run_width(
        rule,
        layout,
        dbu_to_um,
        merged,
        kind.width_name(),
        kind.op(),
        kind.label(),
        &*viol,
        oblique_only,
        mixed,
        min_run_dbu,
    )
}
