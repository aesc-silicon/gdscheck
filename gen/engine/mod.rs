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

mod density;
mod gate_length;
mod max_space;
mod min_array_space;
mod min_enclosure;
mod notch;
mod shape;
mod space;
mod width;

use gdscheck::pdk::PdkConfig;

pub fn generate(pdk: &PdkConfig) {
    density::generate(pdk);
    gate_length::generate(pdk);
    max_space::generate(pdk);
    min_array_space::generate(pdk);
    min_enclosure::generate(pdk);
    notch::generate(pdk);
    shape::generate(pdk);
    space::generate(pdk);
    width::generate(pdk);
}
