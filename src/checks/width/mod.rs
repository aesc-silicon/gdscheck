// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Width: the perpendicular span of material between two facing walls of one merged
//! region.
//!
//! Three rules read it - at least, at most, exactly - and the gate rules read it between
//! chosen walls.  All are one measurement, the facing-wall scan in
//! [`crate::geom::width_pairs`], driven over the tiles by [`scan::run_width`]; what a rule
//! adds is its bound and which passes of the scan it wants.  A plain rule wants the
//! axis-aligned sweeps and the oblique pass; one restricted to 45° runs (`angle: bent`)
//! wants the oblique pass alone, since the axis-aligned material is the plain rule's
//! business; a maximum or an exact width leaves out the mixed pass, because a chamfer
//! facing a straight wall has no single width to be too large or unequal.

pub mod scan;

use crate::geom::{Limit, on_grid};
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

    /// The gate check's name, the same bound between chosen walls.
    pub fn gate_name(self) -> &'static str {
        match self {
            Kind::Min => "min_gate_length",
            Kind::Max => "max_gate_length",
            Kind::Exact => "exact_gate_length",
        }
    }

    /// The bound on the grid.
    pub fn limit(self, value_um: f64, dbu_to_um: f64) -> Limit {
        match self {
            Kind::Min => Limit::at_least(value_um, dbu_to_um),
            Kind::Max => Limit::at_most(value_um, dbu_to_um),
            Kind::Exact => Limit::exactly(value_um, dbu_to_um),
        }
    }

    /// The failing comparison, the inverse of [`Self::op`].
    pub(super) fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
            Kind::Exact => "≠",
        }
    }

    /// The requirement, as the log prints it.
    pub(super) fn op(self) -> &'static str {
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
    // A width is the thickness of a region, and an edge layer carries no region: it used
    // to be read by pairing its segments, which could not tell a gap from a thickness
    // without the region anyway.  A rule that wants the width between chosen walls names
    // the region and the reference with a gate rule instead.  Saying so beats passing
    // in silence on a layer with no polygons, which is the failure this engine works
    // hardest to avoid.
    if let Some(l) = rule
        .layers
        .iter()
        .find(|l| merged.is_edge_layer((l.gds_layer as i16, l.gds_datatype as i16)))
    {
        eprintln!(
            "[{}] {}: '{}' is an edge layer - a width is measured on a region; to measure \
             between chosen walls, name the region and the reference with {}",
            rule.id,
            kind.width_name(),
            l.name,
            kind.gate_name()
        );
        return vec![];
    }
    let limit = kind.limit(rule.value, dbu_to_um);
    // `angle: bent` reads the 45° runs alone - the axis-aligned material is a plain
    // rule's business - and only where the run is long enough to be a trace rather than
    // a chamfer, `bent_length` µm; the others drop nothing.
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
        let bent_um = rule.params.get("bent_length").copied().unwrap_or(0.5);
        on_grid(bent_um / dbu_to_um, f64::ceil)
    } else {
        0
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
        limit,
        oblique_only,
        mixed,
        min_run_dbu,
    )
}

/// The width of `layers[0]` between the walls it shares with the boundary of
/// `layers[1]`, under the bound `kind` puts on it.  See [`scan::run_gate`].
pub fn run_gate(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    scan::run_gate(kind, rule, layout, dbu_to_um, merged)
}
