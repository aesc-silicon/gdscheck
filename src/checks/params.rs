// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The params more than one family reads: a mode word, `angle: bent`, `length`.

use crate::geom::on_grid;
use crate::pdk::{Param, RuleDefinition};

/// A mode param written as a number: the rule refuses to run, rather than running as if
/// the param were absent.
pub struct NotAWord;

/// A mode param: the word the deck wrote, `None` if it wrote nothing, and [`NotAWord`] -
/// said aloud - if it wrote a number.
pub fn mode<'a>(
    rule: &'a RuleDefinition,
    name: &str,
    key: &str,
) -> Result<Option<&'a str>, NotAWord> {
    match rule.params.get(key) {
        None => Ok(None),
        Some(Param::Word(w)) => Ok(Some(w)),
        Some(Param::Num(v)) => {
            eprintln!("[{}] {name}: `{key}` must be a word, not `{v}`", rule.id);
            Err(NotAWord)
        }
    }
}

/// Whether `angle: bent` restricts the rule to 45° runs; `None` if the rule is malformed.
pub fn bent_only(rule: &RuleDefinition, name: &str) -> Option<bool> {
    match mode(rule, name, "angle") {
        Ok(None) => Some(false),
        Ok(Some("bent")) => Some(true),
        Ok(Some(other)) => {
            eprintln!(
                "[{}] {name}: angle can only be `bent`, not `{other}`",
                rule.id
            );
            None
        }
        Err(NotAWord) => None,
    }
}

/// The `length` param as DBU of run.
pub fn min_run(rule: &RuleDefinition, dbu_to_um: f64) -> i64 {
    let um = rule.num("length").unwrap_or(0.0);
    on_grid(um / dbu_to_um, f64::ceil)
}
