// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Count: how many connected regions of a layer the chip holds - the emitters of one
//! bipolar flavour, say, which a foundry caps per chip.  Three rules read it, at least,
//! at most and exactly.  Regions are stitched across tile borders from the shared
//! [`SharedCache`], so a shape cut by a tile line counts once.

use crate::layout::FlatLayout;
use crate::merge::SharedCache;
use crate::pdk::RuleDefinition;
use crate::violation::Violation;

/// Which bound a rule puts on the count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
    Exact,
}

impl Kind {
    /// The check's name, as the deck spells it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Min => "min_count",
            Kind::Max => "max_count",
            Kind::Exact => "exact_count",
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

    /// The failing comparison, as the message prints it.
    fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
            Kind::Exact => "≠",
        }
    }

    /// The violation's title.
    fn title(self) -> &'static str {
        match self {
            Kind::Min => "Minimum count violation",
            Kind::Max => "Maximum count violation",
            Kind::Exact => "Exact count violation",
        }
    }

    fn broken_by(self, count: f64, value: f64) -> bool {
        match self {
            Kind::Min => count < value,
            Kind::Max => count > value,
            Kind::Exact => count != value,
        }
    }
}

/// `min_count` / `max_count` / `exact_count`: the number of regions of each layer on
/// the chip.  One violation per layer out of bounds, anchored at its first region, or
/// at the origin when it has none.
pub fn run(
    kind: Kind,
    rule: &RuleDefinition,
    layout: &FlatLayout,
    dbu_to_um: f64,
    merged: &SharedCache,
) -> Vec<Violation> {
    let mut out = Vec::new();
    for layer in &rule.layers {
        println!(
            "[{}] Checking {} {} {} of {} on the chip",
            rule.id,
            kind.name(),
            kind.op(),
            rule.value,
            layer.name
        );
        let regions = merged.regions(layout, layer.gds_layer as i16, layer.gds_datatype as i16);
        if !kind.broken_by(regions.len() as f64, rule.value) {
            continue;
        }
        let (cx, cy) = regions.first().map(|r| r.marker).unwrap_or((0.0, 0.0));
        out.push(Violation::point(
            &rule.id,
            kind.title(),
            format!(
                "{} {} regions on the chip {} {}",
                regions.len(),
                layer.name,
                kind.cmp(),
                rule.value
            ),
            cx * dbu_to_um,
            cy * dbu_to_um,
        ));
    }
    out
}
