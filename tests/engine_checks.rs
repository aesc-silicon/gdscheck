// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What each *check* measures, asked directly.
//!
//! The PDK fixtures under `tests/data/*/generated` ask whether a deck expresses a rule,
//! and they can only ask it at the one value that rule fixes.  These ask whether the
//! check underneath measures at all - across the shape families it has to cope with, and
//! at margins either side of the limit.  The two failures are indistinguishable in a
//! foundry scoreboard and need different repairs: a deck that reads the rule wrongly, and
//! a check that measures wrongly.
//!
//! **Every expected count below is read off the drawing, never off a run.**  A number
//! copied out of the engine would make the test pass by construction and guard nothing.
//! The patterns are drawn by `gen/engine.rs` and are deliberately simple enough that the
//! answer is arithmetic.

use gdscheck::run_drc;
use rstest::rstest;

const PDK: &str = "tests/data/engine/pdk.yml";
const DIR: &str = "tests/data/engine/generated";

/// How many violations `rule` reports on `pattern`.
fn count(deck: &str, pattern: &str, rule: &str) -> usize {
    let path = format!("{DIR}/{deck}/{pattern}.gds.gz");
    run_drc(&path, PDK, &[deck], None, "TOP", true)
        .expect("DRC run failed")
        .iter()
        .filter(|v| v.rule_id == rule)
        .count()
}

/// Enclosure at 0.5 µm, over three shape families and both metrics.
///
/// - `ortho` insets one wall of a square hole in a square.  Both metrics measure it and
///   must agree: the margin *is* the inset.
/// - `diag` chamfers the enclosing corner at 45° and points the enclosed square's corner
///   into it.  Every orthogonal margin is 1 µm or more and no two walls are parallel, so
///   the projection metric has nothing to measure here and only the euclidian one does.
///   The four corner offsets give margins of 0.566, 0.501, 0.499 and 0.354 µm - the
///   middle pair straddling the limit by well under a nanometre of margin.
/// - `notch` dips the enclosing wall towards the enclosed shape in a 45° V.  The wall it
///   faces is still 1.7 µm away, so again only the euclidian metric reaches the apex; the
///   difference from `diag` is that the violation is mid-wall rather than at a corner.
#[rstest]
#[case("ortho_0400", 1, 1)]
#[case("ortho_0499", 1, 1)]
#[case("ortho_0500", 0, 0)]
#[case("ortho_0600", 0, 0)]
#[case("diag_0354", 0, 1)]
#[case("diag_0499", 0, 1)]
#[case("diag_0501", 0, 0)]
#[case("diag_0566", 0, 0)]
#[case("notch_0400", 0, 1)]
#[case("notch_0499", 0, 1)]
#[case("notch_0500", 0, 0)]
#[case("notch_0600", 0, 0)]
fn min_enclosure_measures_both_metrics(
    #[case] pattern: &str,
    #[case] projection: usize,
    #[case] euclidian: usize,
) {
    assert_eq!(
        count("min_enclosure", pattern, "ENC.proj"),
        projection,
        "{pattern}: projection metric"
    );
    assert_eq!(
        count("min_enclosure", pattern, "ENC.eucl"),
        euclidian,
        "{pattern}: euclidian metric"
    );
}

/// A corner resting *on* the enclosing wall measures zero, which is under any limit and
/// is not what an enclosure rule is about - KLayout drops such a pair before reporting,
/// and `skip_coincident` mirrors that.
///
/// The pair of patterns is what makes this worth asserting.  `touch` is the zero alone.
/// `touch_and_diag` puts a real 0.354 µm margin on a second chamfer beside it, so the
/// second case says the parameter drops *only* the zero and not the violation next to
/// it - which is the way a "skip" flag usually goes wrong.
#[rstest]
#[case("touch", 1, 0)]
#[case("touch_and_diag", 2, 1)]
fn skip_coincident_drops_the_zero_and_nothing_else(
    #[case] pattern: &str,
    #[case] plain: usize,
    #[case] skipping: usize,
) {
    assert_eq!(
        count("min_enclosure", pattern, "ENC.eucl"),
        plain,
        "{pattern}: euclidian"
    );
    assert_eq!(
        count("min_enclosure", pattern, "ENC.eucl_skip"),
        skipping,
        "{pattern}: euclidian with skip_coincident"
    );
}

/// Via spacing inside a 4x4-or-larger array at 0.36 µm, on the shapes an array comes in.
///
/// Which vias are the array is answered geometrically, as the foundry's own deck answers
/// it: an attached row or column a hundredth too close forms, with the block's nearest
/// rows, a 4x4 of its own and violates.  An L and a U are all block.  Counts are per
/// array: one marker for a block with a tight pair, however many pairs it has.
///
/// `both` is GF180's reading, any tight pair in the block violates; `one` is IHP's, one
/// axis at the larger space is enough, so only a block tight both ways violates.
///
/// `tail` is where we part from the foundry deck on purpose: it exempts a whole blob of
/// vias whenever any part of it is thinner than four vias, so a tail of vias on a block
/// hides a tight pair in the block's middle.  The pair is the block's own, and it counts.
#[rstest]
#[case("block_0360", 0, 0)]
#[case("block_0350", 1, 1)]
#[case("block_x_0350", 1, 0)]
#[case("row_0360", 0, 0)]
#[case("row_0350", 1, 0)]
#[case("row_inside_0350", 1, 0)]
#[case("column_0360", 0, 0)]
#[case("column_0350", 1, 0)]
#[case("tail_0350", 1, 0)]
#[case("l_0360", 0, 0)]
#[case("l_0350", 1, 0)]
#[case("u_0360", 0, 0)]
#[case("u_0350", 1, 0)]
fn min_array_space_knows_what_is_inside_the_array(
    #[case] pattern: &str,
    #[case] both: usize,
    #[case] one: usize,
) {
    assert_eq!(
        count("min_array_space", pattern, "ARR.both"),
        both,
        "{pattern}: space in both axes"
    );
    assert_eq!(
        count("min_array_space", pattern, "ARR.one"),
        one,
        "{pattern}: space in one axis"
    );
}
