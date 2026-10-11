// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The params more than one family reads: a mode word, `angle: bent`, `length`,
//! `facing`.

use crate::geom::{Axis, on_grid};
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

/// Which walls a rule measures between: those facing each other along one axis, or -
/// for a spacing rule only - those facing along neither, the corner-to-corner reading.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Along(Axis),
    Neither,
}

/// The `facing` param: `x`, `y`, or - where `corner` allows it - `none`; `Some(None)` if
/// the rule wrote nothing, `None` if it is malformed, which the rule has said.
pub fn facing(rule: &RuleDefinition, name: &str, corner: bool) -> Option<Option<Facing>> {
    match mode(rule, name, "facing") {
        Ok(None) => Some(None),
        Ok(Some("x")) => Some(Some(Facing::Along(Axis::X))),
        Ok(Some("y")) => Some(Some(Facing::Along(Axis::Y))),
        Ok(Some("none")) if corner => Some(Some(Facing::Neither)),
        Ok(Some(other)) => {
            let allowed = if corner {
                "`x`, `y` or `none`"
            } else {
                "`x` or `y`"
            };
            eprintln!(
                "[{}] {name}: facing can only be {allowed}, not `{other}`",
                rule.id
            );
            None
        }
        Err(NotAWord) => None,
    }
}

/// The `facing` axis of a rule that measures along one: a width, a notch, an enclosure.
pub fn facing_axis(rule: &RuleDefinition, name: &str) -> Option<Option<Axis>> {
    Some(match facing(rule, name, false)? {
        Some(Facing::Along(a)) => Some(a),
        _ => None,
    })
}

/// The `offset` param in µm - where a grid or the tracks start - 0 where the deck wrote
/// nothing; `None` if it wrote a word, which the rule has said.
pub fn offset(rule: &RuleDefinition) -> Option<f64> {
    if rule.params.contains_key("offset") {
        rule.num("offset")
    } else {
        Some(0.0)
    }
}
