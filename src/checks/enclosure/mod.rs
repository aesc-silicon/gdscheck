// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Enclosure: the margin by which one layer's region surrounds another's, side by side.
//!
//! Two rules read it, at least and at most, and both take `sides` for which sides have
//! to make the margin: every one (`all`, the default), at least one (`any`, a wire's
//! endcap round a via), the sides bordering a short one (`adjacent`), or the side facing
//! a narrow track's tip (`line_end`).  The measurement is the facing-edge scan in
//! [`scan`], under the `projection` or the `euclidian` metric; what a rule adds is its
//! bound, its sides, and the three flags for a region cut to its enclosing layer.
//!
//! An extension is the same question asked the other way round and only where the
//! cover crosses the target - see [`extension`].

pub mod extension;
pub mod scan;

use crate::layout::FlatLayout;
use crate::merge::MergedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;
pub use scan::Sides;

/// Which bound a rule puts on the margin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// No side shorter than the value (`min_enclosure`).
    Min,
    /// No side longer than the value (`max_enclosure`).
    Max,
}

impl Kind {
    /// The enclosure check's name, as the deck spells it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Min => "min_enclosure",
            Kind::Max => "max_enclosure",
        }
    }

    /// The extension check's name.
    pub fn extension_name(self) -> &'static str {
        match self {
            Kind::Min => "min_extension",
            Kind::Max => "max_extension",
        }
    }

    /// The requirement, as the log prints it.
    pub fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
        }
    }

    /// The failing comparison, as the message prints it.
    pub fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
        }
    }

    /// The violation's title.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Min => "Minimum enclosure violation",
            Kind::Max => "Maximum enclosure violation",
        }
    }
}

/// `min_enclosure` / `max_enclosure`: the margin of `layers[1]` within `layers[0]`,
/// on the sides the params name.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    let Some(sides) = Sides::of(rule, kind.name()) else {
        return vec![];
    };
    run_sides(kind, sides, rule, layout, dbu_to_um, merged)
}

/// The enclosure under `sides`, whether they came from the params or from a check
/// name that stands for them.
pub fn run_sides(
    kind: Kind,
    sides: Sides,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &mut MergedCache,
) -> Vec<Violation> {
    if rule.layers.len() < 2 {
        eprintln!(
            "[{}] {} needs two layers (enclosing, enclosed)",
            rule.id,
            kind.name()
        );
        return vec![];
    }
    // Declared on two *edge* layers, a minimum measures between segments instead - see
    // `edge_distance`.  Which one runs is the deck's choice of layer, not a different
    // rule; the sides and the maximum read regions and have no meaning there.
    if super::edge_distance::on_edge_layers(rule, merged, kind.name()) {
        if kind == Kind::Max || sides != Sides::All {
            eprintln!(
                "[{}] {}: an edge layer has no sides and no maximum - name the regions",
                rule.id,
                kind.name()
            );
            return vec![];
        }
        return super::edge_distance::run_enclosure(rule, layout, dbu_to_um, merged);
    }
    // A short side asking something of its neighbours, or a track's tip: both are
    // about a margin being small, and have no reading as a maximum.
    if kind == Kind::Max && matches!(sides, Sides::Adjacent | Sides::LineEnd) {
        eprintln!(
            "[{}] max_enclosure: sides can be `all` or `any` - `adjacent` and `line_end` \
             are about a margin falling short",
            rule.id
        );
        return vec![];
    }
    scan::run(kind, rule, layout, dbu_to_um, merged, sides)
}
