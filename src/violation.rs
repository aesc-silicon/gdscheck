// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Violation records and the message convention.
//!
//! `message` is the human-readable body WITHOUT the rule id — the console printer
//! prefixes `[<rule-id>]` and the lyrdb report carries the id as the item category.
//! Message format convention:
//!
//! * name the layer(s) involved;
//! * state the failing comparison as `<measured> <cmp> <limit>` with units
//!   (µm / µm² / %) where a measurement exists;
//! * end with the location: `at (x, y) µm` for points,
//!   `at (x1, y1)-(x2, y2) µm` for edges — 4 decimals, ASCII hyphen;
//! * metric words are lowercase (width, space, enclosure, density, …); layer
//!   names keep their PDK spelling.

pub enum ViolationGeometry {
    /// A single point, coordinates in µm
    Point { x: f64, y: f64 },
    /// A single edge, coordinates in µm
    Edge { x1: f64, y1: f64, x2: f64, y2: f64 },
    /// No specific geometry (e.g. global density)
    None,
}

pub struct Violation {
    pub rule_id: String,
    pub description: String,
    pub message: String,
    pub geometry: ViolationGeometry,
}

impl Violation {
    pub fn edge(
        rule_id: &str,
        description: &str,
        message: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            description: description.to_string(),
            message,
            geometry: ViolationGeometry::Edge { x1, y1, x2, y2 },
        }
    }

    pub fn point(rule_id: &str, description: &str, message: String, x: f64, y: f64) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            description: description.to_string(),
            message,
            geometry: ViolationGeometry::Point { x, y },
        }
    }

    pub fn global(rule_id: &str, description: &str, message: String) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            description: description.to_string(),
            message,
            geometry: ViolationGeometry::None,
        }
    }
}
