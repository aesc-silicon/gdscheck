// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shape: what a region or a segment is on its own - its extents, the length of its
//! edges, the angles they run at and turn by, its holes and rings, and whether its
//! vertices sit on the grid.  Nothing here measures one shape against another.
//!
//! The bounded readings - an extent, an edge length - come in three kinds, at least, at
//! most and exactly, on one driver each, the way the width family does.

pub mod edge_length;
pub mod extent;

/// Which bound a rule puts on what it reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Min,
    Max,
    Exact,
}

impl Kind {
    /// The requirement, as the log prints it.
    pub fn op(self) -> &'static str {
        match self {
            Kind::Min => ">=",
            Kind::Max => "<=",
            Kind::Exact => "==",
        }
    }

    /// The failing comparison, as the message prints it.
    pub fn cmp(self) -> &'static str {
        match self {
            Kind::Min => "<",
            Kind::Max => ">",
            Kind::Exact => "≠",
        }
    }

    /// The bound on the grid.
    pub fn limit(self, value_um: f64, dbu_to_um: f64) -> crate::geom::Limit {
        match self {
            Kind::Min => crate::geom::Limit::at_least(value_um, dbu_to_um),
            Kind::Max => crate::geom::Limit::at_most(value_um, dbu_to_um),
            Kind::Exact => crate::geom::Limit::exactly(value_um, dbu_to_um),
        }
    }

    /// The word a violation's title starts with.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Min => "Minimum",
            Kind::Max => "Maximum",
            Kind::Exact => "Exact",
        }
    }
}
