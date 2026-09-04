// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Check-level test patterns: geometry drawn to exercise one *check*, not one rule.
//!
//! A PDK fixture answers "does this deck express this rule", and it can only ask it at
//! the one value that rule happens to fix.  These answer the other question - "does this
//! check measure" - and they sweep the margin either side of the limit, on each shape
//! family the check has to cope with.  The two failures look identical in a foundry
//! scoreboard and need different repairs, which is the reason to separate them.
//!
//! One module per check, so a check's whole story sits in one file.  Every expected count
//! lives beside the assertion in `tests/engine_checks.rs` and is derived from the
//! drawing, never from a run: a pattern whose answer came out of the engine would pass by
//! construction and guard nothing.

mod min_enclosure;

use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    min_enclosure::generate(pdk);
}
