// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compare a run against the reference report the foundry ships beside its test case.
//!
//! The two engines report at different granularity and matching KLayout's marker *count*
//! is explicitly not the goal — matching the set of violating *structures* is.  gdscheck's
//! spacing engine walks region pairs and emits one violation at their closest approach
//! where KLayout emits an edge pair per facing side, so two interlocking combs are one
//! violation here and four there.  Both point at the same place.
//!
//! So markers are clustered by proximity into logical violations, and a reference
//! violation counts as found when one of ours lands near it.  What the tests then assert
//! is a floor on how many are found and a ceiling on how many we invent, which is the
//! shape that makes an improvement pass and a regression fail.  Pinning exact marker
//! counts instead - which this suite used to do - cannot tell those apart: every number
//! moves whenever the engine gets better, so nobody can read the diff.

use std::collections::HashMap;
use std::io::Read;

/// A device-scale span.  The markers KLayout scatters around one structure sit well
/// inside it; two genuinely separate violations sit well outside.
const RADIUS: f64 = 8.0;

/// How much slack two clusters get before they are held to be the same violation.
const NEAR: f64 = 8.0;

/// One report, as the marker centres of each rule.
pub type Markers = HashMap<String, Vec<(f64, f64)>>;

fn parse(text: &str) -> Markers {
    let mut out: Markers = HashMap::new();
    for item in text.split("<item>").skip(1) {
        let Some(rule) = between(item, "<category>", "</category>") else {
            continue;
        };
        let rule = rule.trim().trim_matches('\'').to_string();
        let Some(values) = between(item, "<values>", "</values>") else {
            continue;
        };
        // Geometry only: our own markers carry a human-readable `text:` value whose
        // numbers would otherwise read as coordinates.
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for tag in ["edge-pair:", "edge:", "polygon:", "box:"] {
            for chunk in values.split(tag).skip(1) {
                let geo = chunk.split("</value>").next().unwrap_or("");
                let nums: Vec<f64> = geo
                    .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                    .filter_map(|t| t.parse::<f64>().ok())
                    .collect();
                for (i, n) in nums.iter().enumerate() {
                    if i % 2 == 0 { xs.push(*n) } else { ys.push(*n) }
                }
            }
        }
        if xs.len() < 2 || ys.is_empty() {
            continue;
        }
        let cx = (xs.iter().cloned().fold(f64::MAX, f64::min)
            + xs.iter().cloned().fold(f64::MIN, f64::max))
            / 2.0;
        let cy = (ys.iter().cloned().fold(f64::MAX, f64::min)
            + ys.iter().cloned().fold(f64::MIN, f64::max))
            / 2.0;
        out.entry(rule).or_default().push((cx, cy));
    }
    out
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = s.find(open)? + open.len();
    let rest = &s[start..];
    Some(&rest[..rest.find(close)?])
}

/// Single-link proximity clustering: everything belonging to one violating structure
/// collapses to one entry, on both sides of the comparison.
fn cluster(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut seen = vec![false; pts.len()];
    let mut out = Vec::new();
    for i in 0..pts.len() {
        if seen[i] {
            continue;
        }
        let (mut stack, mut group) = (vec![i], Vec::new());
        seen[i] = true;
        while let Some(k) = stack.pop() {
            group.push(pts[k]);
            for j in 0..pts.len() {
                if !seen[j]
                    && (pts[k].0 - pts[j].0).abs() <= RADIUS
                    && (pts[k].1 - pts[j].1).abs() <= RADIUS
                {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        let n = group.len() as f64;
        out.push((
            group.iter().map(|p| p.0).sum::<f64>() / n,
            group.iter().map(|p| p.1).sum::<f64>() / n,
        ));
    }
    out
}

/// How a run compares to the reference: violations found, and violations invented.
pub struct Score {
    pub matched: usize,
    pub reference: usize,
    pub extra: usize,
    /// Per rule, for the failure message: (rule, matched, reference, extra).
    pub by_rule: Vec<(String, usize, usize, usize)>,
}

pub fn read_gz(path: &str) -> String {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap_or_else(|e| panic!("{path}: {e}"));
    s
}

/// Compare `ours` against `reference`, both as lyrdb text.
pub fn score(reference: &str, ours: &str) -> Score {
    let (gold, mine) = (parse(reference), parse(ours));
    let mut by_rule = Vec::new();
    let (mut tm, mut tr, mut te) = (0, 0, 0);
    let mut rules: Vec<&String> = gold.keys().chain(mine.keys()).collect();
    rules.sort();
    rules.dedup();
    for rule in rules {
        let g = gold.get(rule).map(|v| cluster(v)).unwrap_or_default();
        let o = mine.get(rule).map(|v| cluster(v)).unwrap_or_default();
        let near =
            |a: &(f64, f64), b: &(f64, f64)| (a.0 - b.0).abs() <= NEAR && (a.1 - b.1).abs() <= NEAR;
        let matched = g.iter().filter(|c| o.iter().any(|d| near(c, d))).count();
        let extra = o.iter().filter(|d| !g.iter().any(|c| near(c, d))).count();
        tm += matched;
        tr += g.len();
        te += extra;
        by_rule.push((rule.clone(), matched, g.len(), extra));
    }
    Score {
        matched: tm,
        reference: tr,
        extra: te,
        by_rule,
    }
}
