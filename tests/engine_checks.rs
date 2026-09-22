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
/// - `bar` is a 40 µm bar across three tiles, enclosed by a row of 2.5 µm pieces the
///   way abutting cells enclose a merged row of their Activ.  The tile owning the bar
///   holds the enclosing layer exact only 1 µm past its core; the far end's margin has
///   to be read from the tiles that own it, and was once reported as no enclosure at
///   all whatever the margin.
#[rstest]
#[case("bar_0400", 1, 1)]
#[case("bar_0600", 0, 0)]
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
/// `finger` is the other side of that: a 12x3 finger at the tight space hanging off a
/// legal block is in the blob and no part of any 4x4, so its pairs do not count - which
/// the foundry deck also gets right, by dropping the blob.  Four rows deep it is a 4x4.
///
/// The hardening patterns (hardening/SPEC.md), read off the drawings in
/// `gen/engine/array.rs`: `bound` is a block at 0.36 and one at 0.355, a grid step
/// under; `tile_lines` puts 0.355 blocks across, ending on and starting on the tile
/// lines; `array_flat` / `array_ref` are fifty blocks; `extremes` a block at (1000,
/// 1000) and a 300 µm one.
#[rstest]
#[case("bound", 1, 1)]
#[case("tile_lines", 7, 7)]
#[case("array_flat", 50, 50)]
#[case("array_ref", 50, 50)]
#[case("extremes", 2, 2)]
#[case("block_0360", 0, 0)]
#[case("block_0350", 1, 1)]
#[case("block_x_0350", 1, 0)]
#[case("row_0360", 0, 0)]
#[case("row_0350", 1, 0)]
#[case("row_inside_0350", 1, 0)]
#[case("column_0360", 0, 0)]
#[case("column_0350", 1, 0)]
#[case("tail_0350", 1, 0)]
#[case("finger_3_0350", 0, 0)]
#[case("finger_4_0350", 1, 1)]
#[case("l_0360", 0, 0)]
#[case("l_0350", 1, 0)]
#[case("u_0360", 0, 0)]
#[case("u_0350", 1, 0)]
fn array_space_knows_what_is_inside_the_array(
    #[case] pattern: &str,
    #[case] both: usize,
    #[case] one: usize,
) {
    assert_eq!(
        count("array", pattern, "ARR.both"),
        both,
        "{pattern}: space in both axes"
    );
    assert_eq!(
        count("array", pattern, "ARR.one"),
        one,
        "{pattern}: space in one axis"
    );
}

/// Every part of Outer within 20 µm of Inner, the reference grown as KLayout's `sized`
/// grows rectilinear geometry: a square structuring element, so `corner_near` at 18 µm
/// in each axis - 25 µm as the crow flies - is covered.  `ring` is a tie ring 100 µm
/// across with a target in its middle: the ring's bounding box grown by 20 µm would
/// cover the whole interior, the ring grown does not, and that device is what the
/// latch-up rule exists for.  `straddle` puts the target across a tile line, and asks
/// for one gap; `reach` puts the reference across one, and asks that it cover the full
/// 20 µm into the next tile but one.
#[rstest]
#[case("ring_far", 1)]
#[case("ring_near", 0)]
#[case("straddle_far", 1)]
#[case("straddle_near", 0)]
#[case("reach_far", 1)]
#[case("reach_near", 0)]
#[case("corner_far", 1)]
#[case("corner_near", 0)]
fn max_space_grows_the_reference_as_a_square_and_reads_across_tiles(
    #[case] pattern: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("max_space", pattern, "SPC.max"),
        expected,
        "{pattern}"
    );
}

/// The width family from a deck: what the drivers add to the measurement.  A pair across
/// a tile line is reported once; the parameters arrive; a pinch is a point, an acute tip
/// a point each, a diagonal neck one span; an exact width has no span at a pinch.
///
/// - `straddle_min` is a 0.4 µm bar across the line at x = 40 with its midpoint off the
///   line; `straddle_max` a block 12 µm across it and 8 tall, over W.max's 10 in one
///   dimension only.
/// - `bent_trace` is a 45° bar 0.6 across and 5 long, under W.bent's 0.7 and over
///   W.min's 0.5; `bent_short` the same bar 0.8 long, shorter than W.bent's 1 µm run.
/// - `length_long` and `length_short` are 0.4 µm bars of Inner 6 and 4 µm long either
///   side of W.len's 5 µm.
/// - `exact_ok` is a 0.3 µm via, `exact_off` one 0.3 by 0.32.
/// - `pinch` is two squares corner to corner, `acute` a right isosceles triangle, and
///   `corner` two squares overlapping 0.2 µm diagonally, whose neck is 0.28 µm from the
///   upper square's left wall to the lower square's right wall.
///
/// The hardening patterns, what a rule manual's width asks of any layer, drawn once for
/// every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/width.rs`:
///
/// - `bound`: 0.5 clean, 0.495 in x and y, a 300 µm bar once, a 0.005 sliver, a bar at
///   (1000, 1000) - five spans; the two 300 µm bars are over W.max as well.
/// - `bound_45`: a diamond 0.495 across (four walls) and a 45° strip (two); the 0.502
///   ones, a chamfered box and an L with a chamfered inner corner are clean; the strips
///   are under W.bent's 0.7.
/// - `merge`: unions reading 0.495 - two overlapping boxes, five slices, a ring's wall,
///   an island in a ring - one span each; the 0.5 unions and a bar drawn as a grid clean.
/// - `tile_lines`: 0.495 bars ending on, straddling and starting on the tile lines at
///   20, 21, 40 and 42, tall bars across them, an L cornered on a line: ten spans
///   whatever the tile.
/// - `array_flat` / `array_ref`: fifty 0.495 bars flat and as an array reference.
/// - `comb`: three 0.495 teeth of one polygon; a U with 0.5 arms is clean.
/// - `bent_*`: the 45° rule (W.bent, 0.7 on runs over 1.0) - strips 0.693 wide with
///   long and with 1.018 walls, a Z route's jog and a chamfered L's bend 0.693 across,
///   the same across the tile lines, fifty at once, a 300 µm strip and a sliver; 0.707,
///   walls of 0.99 and a 0.693 diamond are clean.
#[rstest]
#[case("bent_bound", "W.bent", 6)]
#[case("bent_bound", "W.min", 2)]
#[case("bent_routes", "W.bent", 4)]
#[case("bent_tile_lines", "W.bent", 10)]
#[case("bent_array_flat", "W.bent", 100)]
#[case("bent_array_ref", "W.bent", 100)]
#[case("bent_extremes", "W.bent", 4)]
#[case("bound", "W.min", 10)]
#[case("bound", "W.max", 4)]
#[case("bound_45", "W.min", 6)]
#[case("bound_45", "W.bent", 4)]
#[case("merge", "W.min", 8)]
#[case("tile_lines", "W.min", 20)]
#[case("array_flat", "W.min", 100)]
#[case("array_ref", "W.min", 100)]
#[case("comb", "W.min", 6)]
#[case("straddle_min", "W.min", 2)]
#[case("straddle_min", "W.max", 0)]
#[case("straddle_max", "W.max", 2)]
#[case("straddle_max", "W.min", 0)]
#[case("bent_trace", "W.bent", 2)]
#[case("bent_trace", "W.min", 0)]
#[case("bent_short", "W.bent", 0)]
#[case("length_long", "W.len", 2)]
#[case("length_short", "W.len", 0)]
#[case("exact_ok", "W.exact", 0)]
#[case("exact_off", "W.exact", 2)]
#[case("pinch", "W.min", 1)]
#[case("pinch", "W.max", 0)]
#[case("acute", "W.min", 2)]
#[case("corner", "W.min", 1)]
fn width_rules_read_tiles_parameters_and_the_shapes_without_a_run(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("width", pattern, rule), expected, "{pattern}: {rule}");
}

/// The gate rules from a deck: the walls a rule chooses, and the parameters that cut
/// them further.  Outer is the body, Inner the reference whose boundary chooses.
///
/// - `stripe_far` is a 0.8 µm stripe from x = 30 to 70 with a 2 µm piece of Inner cut
///   out of it at 58..60, across the tile line from the stripe's midpoint: the shared
///   walls are the stripe's long walls over the piece, 0.8 apart; the unshared ones
///   include the stripe's ends, 40 apart.  `stripe_long_run` has a 4 µm piece.
/// - `ends_on_reference` is a body 4 by 1 µm whose two ends Inner cuts: the shared walls
///   are 4 apart, the unshared 1; `ends_on_reference_tall` is 4 by 3.5.
/// - `outside_half` is a 10 µm stripe with the piece at 34..36 and Via from 35 on, so
///   only the stretch 34..35 lies outside Via; `outside_all` has Via over the whole
///   piece.
///
/// The hardening patterns, what a rule manual's gate length asks of any layer, drawn
/// once for every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/gate_length.rs`, two markers per gate:
///
/// - `bound`: gates 1.0 and 0.995 tall and wide, 3.0 and 3.005 tall against the
///   maximum, 0.995 gates 3.0 and 3.005 long against the run.
/// - `bound_45`: a 45° stripe with a piece cut from it, 0.99 and 1.004 across.
/// - `merge`: a piece as two abutting halves (one gate), a stripe as two overlapping
///   boxes, two pieces on one stripe (two gates).
/// - `tile_lines`: gates whose piece is across, ends on and starts on the tile lines,
///   a standing gate across y = 20, one at (1000, 1000).
/// - `array_flat` / `array_ref`: fifty gates.
/// - `extremes`: a 0.005 gate, a 300 µm stripe with a gate at its middle.
#[rstest]
#[case("stripe_far", "G.min", 2)]
#[case("stripe_far", "G.len", 0)]
#[case("stripe_far", "G.max_unshared", 2)]
#[case("stripe_far", "G.max_shared", 0)]
#[case("stripe_long_run", "G.min", 2)]
#[case("stripe_long_run", "G.len", 2)]
#[case("ends_on_reference", "G.max_shared", 2)]
#[case("ends_on_reference", "G.max_unshared", 0)]
#[case("bound", "G.min", 8)]
#[case("bound", "G.max_shared", 2)]
#[case("bound", "G.len", 2)]
#[case("bound_45", "G.min", 2)]
#[case("merge", "G.min", 8)]
#[case("tile_lines", "G.min", 18)]
#[case("array_flat", "G.min", 100)]
#[case("array_ref", "G.min", 100)]
#[case("extremes", "G.min", 4)]
#[case("ends_on_reference", "G.min", 0)]
#[case("ends_on_reference_tall", "G.max_shared", 2)]
#[case("ends_on_reference_tall", "G.max_unshared", 2)]
#[case("outside_half", "G.outside", 2)]
#[case("outside_half", "G.min", 2)]
#[case("outside_all", "G.outside", 0)]
#[case("outside_all", "G.min", 2)]
fn gate_rules_choose_their_walls_from_a_deck(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("gate_length", pattern, rule),
        expected,
        "{pattern}: {rule}"
    );
}

/// The density family from a deck: the coverage of Outer over the box of Inner.
///
/// - `chip_40` and `chip_70` put Outer over the lower 40 and 70 µm of a 100 µm box.
/// - `ring_frame` draws Inner as a hollow 1 µm frame: its box is the die, and Outer
///   over the lower 25 µm is 25 % of it.
/// - `window_quarters` fills the lower-left of four 50 µm windows and half of the one
///   above it; `window_partial` is a 130 by 50 die whose third column of windows is 30
///   wide, with Outer at 40 % of every window's own area.
/// - `region_05` and `region_10` are a 100 µm plate with Via over 5 and 10 % of it;
///   `region_small` a 20 µm plate, too small to count.
///
/// The hardening patterns, what a rule manual's density asks of any layer, drawn once
/// for every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/density.rs`.  A density is one number per layout, so the bound is a
/// layout apiece:
///
/// - `bound_*`: Outer over exactly 50 and 60 % of the die, and 0.005 % either side;
///   `bound_45*`: a right triangle over half the die, and 49.995 %.
/// - `merge*`: two overlapping bands summing to 80 % and uniting to 60, a square past
///   the die that the boundary cuts; the union at 60.005.
/// - `tile_lines*`: a die off the lines with bars across every line summing to 50 %,
///   and 49.995.
/// - `array_*`: fifty boxes over half a die, flat and as an array reference, and
///   49.995 %.
/// - `window_hole`: an 80 µm hole in the Outer that a 50 µm window lies in from any
///   tile's anchors.
/// - `region_*`: a plate across a tile line with Via at 6 and 5.995 %; a plate drawn
///   as four overlapping boxes; a plate 35.005 wide, which counts, and one exactly 35,
///   which does not (the size is a strict bound); fifty plates flat and as an array
///   reference.
#[rstest]
#[case("bound_50", "D.min", 0)]
#[case("bound_50", "D.max", 0)]
#[case("bound_49995", "D.min", 1)]
#[case("bound_60", "D.max", 0)]
#[case("bound_60", "D.min", 0)]
#[case("bound_60005", "D.max", 1)]
#[case("bound_45", "D.min", 0)]
#[case("bound_45_under", "D.min", 1)]
#[case("merge", "D.min", 0)]
#[case("merge", "D.max", 0)]
#[case("merge_over", "D.max", 1)]
#[case("tile_lines", "D.min", 0)]
#[case("tile_lines", "D.max", 0)]
#[case("tile_lines_under", "D.min", 1)]
#[case("array_flat", "D.min", 0)]
#[case("array_flat", "D.max", 0)]
#[case("array_ref", "D.min", 0)]
#[case("array_ref", "D.max", 0)]
#[case("array_under_flat", "D.min", 1)]
#[case("array_under_ref", "D.min", 1)]
#[case("window_hole", "D.wmin", 1)]
#[case("region_tile_line", "D.region", 0)]
#[case("region_tile_line_under", "D.region", 1)]
#[case("region_merge", "D.region", 1)]
#[case("region_size_over", "D.region", 1)]
#[case("region_size_exact", "D.region", 0)]
#[case("region_array_flat", "D.region", 50)]
#[case("region_array_ref", "D.region", 50)]
#[case("chip_40", "D.min", 1)]
#[case("chip_40", "D.max", 0)]
#[case("chip_70", "D.min", 0)]
#[case("chip_70", "D.max", 1)]
#[case("ring_frame", "D.min", 1)]
#[case("window_quarters", "D.wmin", 1)]
#[case("window_quarters", "D.wmax", 1)]
#[case("window_partial", "D.wmin", 1)]
#[case("window_partial", "D.wmax", 0)]
#[case("region_05", "D.region", 1)]
#[case("region_10", "D.region", 0)]
#[case("region_small", "D.region", 0)]
fn density_rules_read_the_chip_its_windows_and_its_regions(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("density", pattern, rule),
        expected,
        "{pattern}: {rule}"
    );
}

/// The spacing check from a deck: the bound met exactly and missed by five nanometres
/// on each way two shapes face, and each gate met and not met.
///
/// - `axis_*` are two squares side by side 0.5 and 0.495 apart; `corner_*` corner to
///   corner by 0.3/0.4 and 0.297/0.396, 0.5 and 0.495 as the crow flies; `diag_*` two
///   45° bars 0.5006 and 0.4992 across, the nearest gaps the grid has either side of
///   0.5, and both bent under S.bent's 0.8.
/// - `bent_pair` is the 45° pair 0.601 across; `bent_elsewhere` two axis-aligned bars
///   0.6 apart with the only 45° wall 15 µm from the gap.
/// - `run_*` are 0.2 µm bars running alongside for 2 and 2.005 µm; `wide_*` bars
///   running for 1 µm with a line 0.3 and 0.305 deep; `both_*` bars of Via wide and
///   long, wide and short, narrow and long.
/// - `net_bridged` is two squares 0.25 apart joined through Via and Inner, `net_apart`
///   the same two alone.
///
/// The hardening patterns, what a rule manual's space asks of any layer, drawn once for
/// every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/space.rs`:
///
/// - `bound`: a 0.495 gap, a 0.495 corner-to-corner offset, a 0.495 gap between boxes
///   meeting corner-on; 0.5 and 0.502 corner to corner are clean.
/// - `bound_45`: a diamond's tip 0.495 above a wall, two 45° strips 0.495 apart, a box
///   corner 0.495 from a chamfer, two tips 0.495 apart - every one has a 45° wall, so
///   S.bent (0.8) reads them all.
/// - `merge`: a third box 0.495 from two overlapping boxes, two abutting ones and a grid,
///   and an island 0.495 from a ring's inner wall - one pair each.
/// - `tile_lines`: 0.495 gaps ending on, straddling and starting on the tile lines at 20,
///   21, 40 and 42, two gaps running across them, a corner-to-corner pair across (20,
///   20): ten whatever the tile.
/// - `array_flat` / `array_ref`: fifty 0.495 pairs flat and as an array reference.
/// - `extremes`: a 0.005 sliver 0.495 from a box, two 300 µm bars 0.495 apart (one
///   pair), a pair at (1000, 1000).
/// - `bent_*`: the 45° rule (S.bent, 0.8) - two strips 0.792 apart, a corner 0.792 from
///   a chamfer, a tip 0.795 above a wall and one 0.795 from a wall, a corner 0.792 from
///   a strip's wall, across the tile lines, fifty at once, 300 µm strips and a sliver;
///   0.806 and 0.8 are clean.
/// - `gated_*`: the gated rule (S.both on Via: 0.6 where a line is wider than 0.3 and
///   the run over 2.0) - the width bound, the run bound (aligned, offset, stubs, a line
///   broken in two), the wide part of a padded line and an L's arm, both metrics and the
///   ends, unions, the tile lines, fifty at once, a 300 µm pair and a sliver; a U's slot
///   is a notch, not a space of lines.
#[rstest]
#[case("gated_bound", "S.both", 3)]
#[case("gated_run", "S.both", 4)]
#[case("gated_wide", "S.both", 2)]
#[case("gated_ends", "S.both", 1)]
#[case("gated_merge", "S.both", 4)]
#[case("gated_tile_lines", "S.both", 11)]
#[case("gated_array_flat", "S.both", 50)]
#[case("gated_array_ref", "S.both", 50)]
#[case("gated_extremes", "S.both", 2)]
#[case("bent_bound", "S.bent", 5)]
#[case("bent_tile_lines", "S.bent", 5)]
#[case("bent_array_flat", "S.bent", 50)]
#[case("bent_array_ref", "S.bent", 50)]
#[case("bent_extremes", "S.bent", 2)]
#[case("bound", "S.min", 3)]
#[case("bound_45", "S.min", 4)]
#[case("bound_45", "S.bent", 4)]
#[case("merge", "S.min", 4)]
#[case("tile_lines", "S.min", 10)]
#[case("array_flat", "S.min", 50)]
#[case("array_ref", "S.min", 50)]
#[case("extremes", "S.min", 3)]
#[case("axis_exact", "S.min", 0)]
#[case("axis_under", "S.min", 1)]
#[case("corner_exact", "S.min", 0)]
#[case("corner_under", "S.min", 1)]
#[case("diag_exact", "S.min", 0)]
#[case("diag_exact", "S.bent", 1)]
#[case("diag_under", "S.min", 1)]
#[case("diag_under", "S.bent", 1)]
#[case("bent_pair", "S.bent", 1)]
#[case("bent_pair", "S.min", 0)]
#[case("bent_elsewhere", "S.bent", 0)]
#[case("bent_elsewhere", "S.min", 0)]
#[case("run_exact", "S.len", 0)]
#[case("run_over", "S.len", 1)]
#[case("run_over", "S.wide", 0)]
#[case("wide_exact", "S.wide", 0)]
#[case("wide_over", "S.wide", 1)]
#[case("wide_over", "S.len", 0)]
#[case("both_wide_long", "S.both", 1)]
#[case("both_wide_short", "S.both", 0)]
#[case("both_narrow_long", "S.both", 0)]
#[case("net_bridged", "S.same", 1)]
#[case("net_bridged", "S.diff", 0)]
#[case("net_bridged", "S.min", 1)]
#[case("net_apart", "S.same", 0)]
#[case("net_apart", "S.diff", 1)]
#[case("net_apart", "S.min", 1)]
fn space_rules_meet_the_bound_exactly_and_read_their_gates(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("space", pattern, rule), expected, "{pattern}: {rule}");
}

/// The notch check from a deck: a slot exactly the value wide and five nanometres
/// narrower, axis-aligned and at 45°, a thin hole, and the gates.
///
/// - `slot_*` are slots 0.5 and 0.495 wide in a block; `hole_thin` the 0.495 slot
///   closed into a hole; `diag_*` the block turned 45° with slots 0.5006 and 0.4992
///   across, both bent under N.bent's 0.8.
/// - `bent_slot` is a 45° slot 0.601 across running 2.83 µm, `bent_short` the same
///   0.71 µm deep, short of N.bent's 1 µm.
/// - `len_*` are 0.4 µm slots in Inner 2 and 2.005 µm deep.
///
/// The hardening patterns, what a rule manual's "space or notch" asks of one shape,
/// drawn once for every deck (hardening/SPEC.md); the counts are read off the drawings
/// in `gen/engine/notch.rs`:
///
/// - `shapes`: a U notch, a straight-vs-45° notch, a comb's three slots, a slot into a
///   plate, a keyhole's hole, a U's arms, a channel through a ring's wall - all 0.495;
///   the 0.5 controls are clean.
/// - `tile_lines`: 0.495 slots ending on, straddling and starting on the tile lines at
///   20, 21, 40 and 42, and ring channels straddling 20 and 40: nine whatever the tile.
/// - `array_flat` / `array_ref`: fifty 0.495 slots flat and as an array reference.
/// - `extremes`: a 300 µm slot (one notch) and a slot at (1000, 1000).
#[rstest]
#[case("shapes", "N.min", 8)]
#[case("tile_lines", "N.min", 9)]
#[case("array_flat", "N.min", 50)]
#[case("array_ref", "N.min", 50)]
#[case("extremes", "N.min", 2)]
#[case("slot_exact", "N.min", 0)]
#[case("slot_under", "N.min", 1)]
#[case("hole_thin", "N.min", 1)]
#[case("diag_exact", "N.min", 0)]
#[case("diag_exact", "N.bent", 1)]
#[case("diag_under", "N.min", 1)]
#[case("diag_under", "N.bent", 1)]
#[case("bent_slot", "N.bent", 1)]
#[case("bent_slot", "N.min", 0)]
#[case("bent_short", "N.bent", 0)]
#[case("len_exact", "N.len", 0)]
#[case("len_over", "N.len", 1)]
fn notch_rules_meet_the_bound_exactly_and_read_their_gates(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("notch", pattern, rule), expected, "{pattern}: {rule}");
}

/// The three scopes of a reach rule at the bound, and the reach confined to a layer.
///
/// - `part_*`: a target whose far wall is 20 and 20.005 from the reference.
/// - `poly_*`: a target whose near wall is 20 and 20.005 away, and a block whose near
///   part is in reach and far part not.
/// - `within_*`: a U of Via with the reference on one leg and the target on the other
///   leg across the slot, or on the same leg.
/// - `edge_*`: a 0.2 µm bar with the reference 20 and 20.005 from its right wall.
///
/// The hardening patterns, what a rule manual's reach asks of any layer, drawn once
/// for every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/max_space.rs`:
///
/// - `bound`: targets 20 and 20.005 off on every side and off the corner, one 20.01
///   off as the crow flies but 14.15 in each axis, covered - the reach is the square;
///   as parts, every 2 µm target at 20 has its far half beyond reach.
/// - `bound_45`: a diamond reference's reach is the diamond grown along its walls,
///   not its box grown; a diamond target with its tip at 20 and at 20.005; a 45° strip
///   reference with a box 19.997 and 20.004 from its wall.
/// - `merge`: a reference as two overlapping boxes, a target as thirteen slices with one
///   gap, a ring target round a reference out of reach, a reference ring covering.
/// - `tile_lines`: pairs exactly 20 and 20.005 apart across a tile line, a target
///   reaching from the reach into a gap, pairs across 21, 40 and 42, one at (1000,
///   1000).
/// - `array_flat` / `array_ref`: fifty pairs 21 apart.
/// - `extremes`: a 0.005 sliver, a 300 µm bar with the reference at its middle (two
///   gaps), a target with nothing near it.
/// - `edge_tile_lines`: an Outer square across a line with one wall out of reach.
#[rstest]
#[case("bound", "SPC.max", 10)]
#[case("bound", "M.poly", 5)]
#[case("bound_45", "SPC.max", 5)]
#[case("bound_45", "M.poly", 3)]
#[case("merge", "SPC.max", 2)]
#[case("merge", "M.poly", 1)]
#[case("tile_lines", "SPC.max", 7)]
#[case("tile_lines", "M.poly", 4)]
#[case("array_flat", "SPC.max", 50)]
#[case("array_flat", "M.poly", 50)]
#[case("array_ref", "SPC.max", 50)]
#[case("array_ref", "M.poly", 50)]
#[case("extremes", "SPC.max", 4)]
#[case("extremes", "M.poly", 2)]
#[case("edge_tile_lines", "M.edge", 1)]
#[case("part_exact", "SPC.max", 0)]
#[case("part_over", "SPC.max", 1)]
#[case("poly_exact", "M.poly", 0)]
#[case("poly_over", "M.poly", 1)]
#[case("poly_block", "M.poly", 0)]
#[case("poly_block", "SPC.max", 1)]
#[case("within_slot", "M.within", 1)]
#[case("within_slot", "M.poly", 0)]
#[case("within_leg", "M.within", 0)]
#[case("edge_exact", "M.edge", 1)]
#[case("edge_over", "M.edge", 4)]
fn max_space_scopes_meet_the_bound_and_a_confined_reach_goes_round(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("max_space", pattern, rule),
        expected,
        "{pattern}: {rule}"
    );
}

/// The enclosure family beyond the plain minimum: the other bound, and each `sides`
/// word met exactly and missed by five nanometres.
///
/// - `max_*`: a square with every margin 0.5, then its left margin 0.505.
/// - `any_*`: a shape flush on three sides with its right margin 0.5 and 0.495;
///   `max_any_*`: a square with every margin 0.505, and one with its left at 0.5.
/// - `adj_*`: a shape 0.05 from the left wall (under the 0.1 trigger) with its bottom
///   margin 1.0 and 0.495, and one exactly 0.1 from the wall with 0.495 below.
/// - `cap_*`: a 0.3 µm track with a via 0.3 and 0.295 from its tip, and a 0.34 track,
///   which is not narrow.
/// - `ext_*`: a cover crossing a bar, reaching 0.5, 0.495 and 0.505 past its long walls.
///
/// The hardening patterns, what a rule manual's enclosure asks of any pair of layers,
/// drawn once for every deck (hardening/SPEC.md); the counts are read off the drawings
/// in `gen/engine/min_enclosure.rs`, a 0.4 square in the Outer:
///
/// - `bound`: 0.495 on the left, right, bottom and top of a square (one each), a square
///   half out and one with no Outer at all; 0.5 all round is clean.
/// - `shapes`: a square across a 0.005 gap between two boxes (both metrics), a chamfer
///   0.495 from its corner and a diamond's four walls 0.495 from its corners (the
///   closest approach's, ENC.eucl); over a seam, over an overlap, under a grid, and a
///   chamfer 0.502 off are clean.
/// - `tile_lines`: squares sticking 0.005 out of an Outer ending on the tile lines at
///   20, 21, 40 and 42, at 10 and at (1000, 1000); squares straddling 20 and 40 with 0.5
///   all round are clean.
/// - `array_flat` / `array_ref`: fifty squares sticking 0.005 out, flat and as an array
///   reference.
/// - `adjacent`: a line's cap 0.495 and 0.0, a plate corner flush on two sides, 0.05 on
///   two adjacent sides, on all four, and on three (ENC.adj); a 0.5 cap, mid-line, 0.5
///   beside a flush or a 0.05 side, and 0.05 on two opposite sides are clean.
/// - `max_tile_lines`: the maximum read across the lines, one violation per shape - a
///   square in the middle of a 45 µm plate over three tiles reads its margin once.
/// - `ext_tile_lines`: covers across the lines reaching 0.495 and 0.505 past a bar.
/// - `extremes`: a 0.005 sliver with 0.495 on one side, a 300 µm bar with 0.495 at its
///   far end; one with 0.5 all round is clean.
#[rstest]
#[case("max_tile_lines", "ENC.max", 7)]
#[case("max_tile_lines", "ENC.proj", 0)]
#[case("ext_tile_lines", "ENC.ext", 3)]
#[case("ext_tile_lines", "ENC.max_ext", 2)]
#[case("extremes", "ENC.proj", 2)]
#[case("extremes", "ENC.eucl", 2)]
#[case("bound", "ENC.proj", 6)]
#[case("bound", "ENC.eucl", 6)]
#[case("shapes", "ENC.proj", 1)]
#[case("shapes", "ENC.eucl", 6)]
#[case("tile_lines", "ENC.proj", 6)]
#[case("tile_lines", "ENC.eucl", 6)]
#[case("array_flat", "ENC.proj", 50)]
#[case("array_ref", "ENC.proj", 50)]
#[case("adjacent", "ENC.adj", 6)]
#[case("max_exact", "ENC.max", 0)]
#[case("max_exact", "ENC.proj", 0)]
#[case("max_over", "ENC.max", 1)]
#[case("any_exact", "ENC.any", 0)]
#[case("any_exact", "ENC.max_any", 0)]
#[case("any_under", "ENC.any", 1)]
#[case("any_under", "ENC.max_any", 0)]
#[case("max_any_over", "ENC.max_any", 1)]
#[case("max_any_over", "ENC.max", 1)]
#[case("max_any_one", "ENC.max_any", 0)]
#[case("max_any_one", "ENC.max", 1)]
#[case("adj_ok", "ENC.adj", 0)]
#[case("adj_under", "ENC.adj", 1)]
#[case("adj_trigger", "ENC.adj", 0)]
#[case("cap_exact", "ENC.cap", 0)]
#[case("cap_under", "ENC.cap", 1)]
#[case("cap_wide", "ENC.cap", 0)]
#[case("ext_exact", "ENC.ext", 0)]
#[case("ext_exact", "ENC.max_ext", 0)]
#[case("ext_under", "ENC.ext", 1)]
#[case("ext_over", "ENC.ext", 0)]
#[case("ext_over", "ENC.max_ext", 1)]
fn enclosure_bounds_and_sides_meet_the_bound_exactly(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("min_enclosure", pattern, rule),
        expected,
        "{pattern}: {rule}"
    );
}

/// The shape family from a deck: each bound of an extent and of an edge length met
/// exactly and missed by five nanometres, and the corner, angle, hole and vertex rules.
///
/// - `dim_*` and `len_*`: bars whose short and long sides sit on and past the bounds;
///   `exact_*`: a via bar exactly 0.3 by 1.0 and off in one dimension.
/// - `edge_*`: a square with a corner cut 0.5 and 0.495 wide; `edge45_*`: a chamfer
///   0.5006 and 0.4992 long.
/// - `corner_square` turns four right angles, `corner_octagon` none; `sharp_triangle`
///   has two corners sharper than a right angle; `hole_ring` has a hole and
///   `hole_filled` not; `vert_over` has nine vertices; `via45` two 45° walls.
///
/// The hardening patterns, what a rule manual's extent, edge, corner, angle, hole and
/// vertex rules ask of any layer, drawn once for every deck (hardening/SPEC.md); the
/// counts are read off the drawings in `gen/engine/shape.rs`.  An extent is the merged
/// region's bounding box: a 45° bar's box is not the bar, an L's box is its reach.
///
/// - `dim_*` (E.min_dim 0.5 / E.max_dim 2.0 on Outer): 0.495 bars both ways, a diamond
///   0.48 across the box, 2.005 and a 45° strip with a 3.35 box; unions read at 0.495
///   and 2.005, a sliced bar, a corner-touching pair as one region, an island; the tile
///   lines, with a box whose piece left of a line is 0.3 wide staying clean and a 2.005
///   bar whose pieces are under 2 firing once; fifty at once; a sliver and a plate.
/// - `len_*` (E.min_len 5 / E.max_len 20 on Inner): the same for the long side, a 1 ×
///   20.005 bar across a line whose pieces are 10 and 10.005 firing once.
/// - `exact_*` (E.exact_dim 0.3 / E.exact_len 1.0 on Via): 0.295, 0.305, 0.995 and
///   1.005; unions; vias across the lines whose pieces are 0.3 × 0.5 staying right.
/// - `edge_*` (L.min 0.5 on Inner, L.edge through Outer's edge layer): a 0.495 cut,
///   notch floor, step tread and a 0.4992 chamfer; a union's 0.495 edges and an island's
///   four; floors across, ending on and starting on the lines and a riser lying on x =
///   20, a square across a line having no short edge; fifty; a 0.005 floor.
/// - `corner_*` (C.right on Outer): squares and an L across the lines, the cuts being
///   no corners; unions' corners; fifty squares.  `sharp_tile_lines` (C.sharp): right
///   triangles across the lines.  `angle_tile_lines` (A.ortho): a diamond, a strip and
///   chamfers across the lines, one marker per edge; `angle45_tile_lines` (A.bent45):
///   a 45° bar across a line, and a 26.6° one that is not 45.
/// - `hole_tile_lines` (H.none on Outer): rings across the lines, one whose hole ends on
///   a line, keyhole-drawn and bar-drawn, one holding an island, a 50 µm ring over
///   three tiles - a hole wider than the halo is the region's and no tile copy's;
///   `hole_array_*`: fifty rings.
/// - `vert_*` (V.max 8 on Outer, on the drawn polygon): 9-gons across a line and far
///   off, an octagon across a line whose pieces have more vertices than it; fifty.
#[rstest]
#[case("dim_bound", "E.min_dim", 3)]
#[case("dim_bound", "E.max_dim", 2)]
#[case("dim_merge", "E.min_dim", 3)]
#[case("dim_merge", "E.max_dim", 1)]
#[case("dim_tile_lines", "E.min_dim", 10)]
#[case("dim_tile_lines", "E.max_dim", 1)]
#[case("dim_array_flat", "E.min_dim", 50)]
#[case("dim_array_ref", "E.min_dim", 50)]
#[case("dim_extremes", "E.min_dim", 1)]
#[case("dim_extremes", "E.max_dim", 1)]
#[case("len_bound", "E.min_len", 4)]
#[case("len_bound", "E.max_len", 1)]
#[case("len_merge", "E.min_len", 2)]
#[case("len_merge", "E.max_len", 1)]
#[case("len_tile_lines", "E.min_len", 8)]
#[case("len_tile_lines", "E.max_len", 2)]
#[case("len_array_flat", "E.min_len", 50)]
#[case("len_array_ref", "E.min_len", 50)]
#[case("len_extremes", "E.min_len", 1)]
#[case("len_extremes", "E.max_len", 1)]
#[case("exact_bound", "E.exact_dim", 2)]
#[case("exact_bound", "E.exact_len", 2)]
#[case("exact_merge", "E.exact_dim", 1)]
#[case("exact_merge", "E.exact_len", 1)]
#[case("exact_tile_lines", "E.exact_dim", 2)]
#[case("exact_tile_lines", "E.exact_len", 1)]
#[case("exact_array_flat", "E.exact_dim", 50)]
#[case("exact_array_ref", "E.exact_dim", 50)]
#[case("exact_extremes", "E.exact_dim", 1)]
#[case("exact_extremes", "E.exact_len", 1)]
#[case("edge_bound", "L.min", 4)]
#[case("edge_layer_bound", "L.edge", 4)]
#[case("edge_merge", "L.min", 6)]
#[case("edge_tile_lines", "L.min", 8)]
#[case("edge_layer_tile_lines", "L.edge", 8)]
#[case("edge_array_flat", "L.min", 50)]
#[case("edge_array_ref", "L.min", 50)]
#[case("edge_extremes", "L.min", 1)]
#[case("corner_tile_lines", "C.right", 18)]
#[case("corner_tile_lines", "H.none", 0)]
#[case("corner_tile_lines", "V.max", 0)]
#[case("corner_merge", "C.right", 16)]
#[case("corner_array_flat", "C.right", 200)]
#[case("corner_array_ref", "C.right", 200)]
#[case("sharp_tile_lines", "C.sharp", 6)]
#[case("sharp_tile_lines", "A.ortho", 3)]
#[case("angle_tile_lines", "A.ortho", 10)]
#[case("angle45_tile_lines", "A.bent45", 2)]
#[case("hole_tile_lines", "H.none", 7)]
#[case("hole_array_flat", "H.none", 50)]
#[case("hole_array_ref", "H.none", 50)]
#[case("vert_tile_lines", "V.max", 2)]
#[case("vert_array_flat", "V.max", 50)]
#[case("vert_array_ref", "V.max", 50)]
#[case("dim_exact", "E.min_dim", 0)]
#[case("dim_under", "E.min_dim", 1)]
#[case("dim_max_exact", "E.max_dim", 0)]
#[case("dim_max_over", "E.max_dim", 1)]
#[case("len_exact", "E.min_len", 0)]
#[case("len_under", "E.min_len", 1)]
#[case("len_max_exact", "E.max_len", 0)]
#[case("len_max_over", "E.max_len", 1)]
#[case("exact_ok", "E.exact_dim", 0)]
#[case("exact_ok", "E.exact_len", 0)]
#[case("exact_off_dim", "E.exact_dim", 1)]
#[case("exact_off_dim", "E.exact_len", 0)]
#[case("exact_off_len", "E.exact_len", 1)]
#[case("exact_off_len", "E.exact_dim", 0)]
#[case("edge_exact", "L.min", 0)]
#[case("edge_under", "L.min", 1)]
#[case("edge_layer_under", "L.edge", 1)]
#[case("edge45_over", "L.min", 0)]
#[case("edge45_under", "L.min", 1)]
#[case("edge45_under", "A.ortho", 1)]
#[case("corner_square", "C.right", 4)]
#[case("corner_square", "H.none", 0)]
#[case("corner_octagon", "C.right", 0)]
#[case("corner_octagon", "V.max", 0)]
#[case("vert_over", "V.max", 1)]
#[case("sharp_triangle", "C.sharp", 2)]
#[case("sharp_triangle", "A.ortho", 1)]
#[case("sharp_rect", "C.sharp", 0)]
#[case("sharp_rect", "A.ortho", 0)]
#[case("hole_ring", "H.none", 1)]
#[case("hole_filled", "H.none", 0)]
#[case("via45", "A.bent45", 2)]
#[case("via_rect", "A.bent45", 0)]
fn shape_rules_meet_the_bound_exactly_and_read_the_shape(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("shape", pattern, rule), expected, "{pattern}: {rule}");
}

/// The area family from a deck: each bound met exactly and missed on each scope, the
/// half-DBU² grid, and the connected-net gate.
///
/// - `region_*`: rectangles of exactly 0.5 and 4 µm² and five nanometres off;
///   `tri_half` is a right triangle with legs of odd nanometres, half a square DBU
///   under 0.5 µm² where `tri_exact` is on it.
/// - `exact_*`: a via 0.3 square and 0.3 by 0.305.
/// - `hole_*`: rings with holes exactly on the bounds and five nanometres off;
///   `contained_*`: two squares in a container summing to exactly 2 µm² and over;
///   `chip_*`: two rectangles summing to exactly 10 µm² and over.
/// - `net_*`: a 0.3 µm² region of Inner tied to Outer through a Via, the same left
///   loose, and a 1 µm² one tied.
///
/// The hardening patterns, what a rule manual's area asks of any layer, drawn once for
/// every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/area.rs`:
///
/// - `bound`: 1.0 × 0.495, a 0.5 × 0.99 bar, an L, a diamond and a chamfered box under
///   0.5 µm²; 1.0 × 0.5, 0.5 × 1.005 and a diamond over it are clean.
/// - `merge`: an overlapping pair's union at 0.35 and an island in a ring; a pair
///   meeting at a corner is one region, abutting boxes and a grid are clean.
/// - `tile_lines`: 1.0 × 0.495 boxes ending on, straddling and starting on the tile
///   lines, a bar across 20, one at (1000, 1000); a 1.0 × 0.5 box straddling 20 is clean.
/// - `array_flat` / `array_ref`: fifty 0.4 µm² boxes flat and as an array reference.
/// - `extremes`: a 0.005 × 2 sliver; a 0.005 × 100 one is 0.5 and clean.
#[rstest]
#[case("bound", "A.min", 5)]
#[case("merge", "A.min", 2)]
#[case("tile_lines", "A.min", 9)]
#[case("array_flat", "A.min", 50)]
#[case("array_ref", "A.min", 50)]
#[case("extremes", "A.min", 1)]
#[case("region_exact", "A.min", 0)]
#[case("region_under", "A.min", 1)]
#[case("region_max_exact", "A.max", 0)]
#[case("region_max_over", "A.max", 1)]
#[case("tri_exact", "A.min", 0)]
#[case("tri_half", "A.min", 1)]
#[case("exact_ok", "A.exact", 0)]
#[case("exact_off", "A.exact", 1)]
#[case("hole_exact", "A.hole", 0)]
#[case("hole_under", "A.hole", 1)]
#[case("hole_max_exact", "A.hole_max", 0)]
#[case("hole_max_over", "A.hole_max", 1)]
#[case("contained_exact", "A.cont", 0)]
#[case("contained_over", "A.cont", 1)]
#[case("chip_exact", "A.chip", 0)]
#[case("chip_over", "A.chip", 1)]
#[case("net_small_tied", "A.net", 1)]
#[case("net_small_loose", "A.net", 0)]
#[case("net_big_tied", "A.net", 0)]
fn area_rules_meet_the_bound_exactly_on_every_scope(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("area", pattern, rule), expected, "{pattern}: {rule}");
}

/// The net family from a deck: the antenna ratio met exactly and missed, by area and by
/// sidewall, by population, and read at a level below the conductor; the nets under a
/// marker.
///
/// - `ratio_*`: a 1 µm² gate under a plate of exactly 10, 10.005 and 9.995 µm²: a
///   ratio at the limit meets it, one over it fires.
/// - `diode_small`, `diode_big`, `bare_big`: the 10 µm² plate with a diode, a 100 µm²
///   plate with one, and the 100 µm² plate alone.
/// - `side_*`: a 4 by 6 plate, a sidewall of exactly 10 at 0.5 thick, 4 by 6.005 and
///   4 by 5.995.
/// - `nets_*`: two Inner squares under one Outer marker, tied to it and not.
///
/// The hardening patterns, what a rule manual's antenna ratio asks of any layer, drawn
/// once for every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/net.rs`, one violation per gate:
///
/// - `bound_45`: a plate with a triangle on a corner at 10.005 and one cut off at 9.995.
/// - `merge`: a plate as two overlapping boxes (10 as a union, 12 as a sum), a gate as
///   two halves under a 10.005 plate.
/// - `tile_lines`: plates and gates across the lines, one ending on 42, one at (1000,
///   1000); a 10 plate across 40 is clean.
/// - `array_flat` / `array_ref`: fifty antennas.
/// - `extremes`: a 300 µm plate, a 0.005 sliver.
/// - `nets_tile_line_*`, `nets_array_*`: a marker across a line over a square each
///   side, tied and not; fifty untied markers.
#[rstest]
#[case("bound_45", "N.ant", 1)]
#[case("merge", "N.ant", 1)]
#[case("tile_lines", "N.ant", 5)]
#[case("array_flat", "N.ant", 50)]
#[case("array_ref", "N.ant", 50)]
#[case("extremes", "N.ant", 1)]
#[case("nets_tile_line_one", "N.nets", 0)]
#[case("nets_tile_line_two", "N.nets", 1)]
#[case("nets_array_flat", "N.nets", 50)]
#[case("nets_array_ref", "N.nets", 50)]
#[case("ratio_exact", "N.ant", 0)]
#[case("ratio_exact", "N.level", 0)]
#[case("ratio_exact", "N.bare", 0)]
#[case("ratio_over", "N.ant", 1)]
#[case("ratio_over", "N.level", 0)]
#[case("ratio_over", "N.bare", 1)]
#[case("ratio_under", "N.ant", 0)]
#[case("ratio_under", "N.bare", 0)]
#[case("diode_small", "N.bare", 0)]
#[case("diode_small", "N.protected", 0)]
#[case("diode_big", "N.protected", 1)]
#[case("diode_big", "N.bare", 0)]
#[case("bare_big", "N.bare", 1)]
#[case("side_exact", "N.side", 0)]
#[case("side_over", "N.side", 1)]
#[case("side_under", "N.side", 0)]
#[case("nets_one", "N.nets", 0)]
#[case("nets_two", "N.nets", 1)]
fn net_rules_meet_the_bound_exactly_and_read_their_levels(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(count("net", pattern, rule), expected, "{pattern}: {rule}");
}

/// The residual family: no bound to meet, so the exact case is the coincident edge.
///
/// - `bare` has three Diode squares, one across a tile line, and an Outer square with
///   four edges.
/// - `covered_exact` / `covered_short` end the cover on the target's edge and 0.005 µm
///   short of it.
/// - `union_exact` / `union_gap` cover the target with two layers meeting on a line
///   and with a 0.005 µm gap between them.
/// - `diag_exact` / `diag_out` put a 45° diamond's tips on the cover's sides, and one
///   tip 0.005 µm past.
/// - `parts` leaves three bare parts on one polygon; `span` leaves one across a tile
///   line.
/// - `overlap_touch` / `overlap_slim` share an edge and overlap by 0.005 µm.
/// - `partners` has a partner inside, one sharing an edge, and one 0.005 µm off.
/// - `array_grid` / `array_line` / `array_bent` put four vias on a plate as a 2x2, in
///   a row and in an L; `array_reach` / `array_split` set the grid's rows exactly the
///   reach apart and 0.005 µm further.
/// - `beyond_edge` / `beyond_out` put a corner on the frame's outer edge and 0.005 µm
///   past it, with an ignored layer past the frame.
/// - `labelled` carries the exemption label on one of two squares.
///
/// The hardening patterns, what a rule manual's residual asks of any layer, drawn once
/// for every deck (hardening/SPEC.md); the counts are read off the drawings in
/// `gen/engine/residual.rs`, every shape of a layout counted since the deck's rules
/// read the same layers against each other:
///
/// - `bound_45`: a Diode cover's chamfer through the target's corner and 0.005 in; an
///   Outer diamond's tip on a wall and 0.005 in.
/// - `merge`: a target as six slices with one bare part, a cover as two overlapping
///   boxes, two Diode squares overlapping as one region, an Outer box as two halves
///   overlapping once.
/// - `tile_lines`: bare parts ending on, starting on and across the tile lines, an
///   overlap across x = 20, a Via partner across the line and a Diode partner sharing
///   the wall on it, Diode squares across (20, 20) and at (1000, 1000).
/// - `array_flat` / `array_ref`: fifty of each residual.
/// - `extremes`: a 0.005 sliver, a 300 µm bar bare at its far end, a 300 µm overlap.
/// - `array_tile_lines`: a 2×2 via grid across a tile line, and one whose rows are
///   1.005 apart.
#[rstest]
#[case("bound_45", "R.uncovered", 1)]
#[case("bound_45", "R.polygon", 1)]
#[case("bound_45", "R.overlap", 1)]
#[case("bound_45", "R.apart", 0)]
#[case("bound_45", "R.touching", 4)]
#[case("merge", "R.uncovered", 1)]
#[case("merge", "R.polygon", 1)]
#[case("merge", "R.bare", 5)]
#[case("merge", "R.overlap", 1)]
#[case("tile_lines", "R.uncovered", 8)]
#[case("tile_lines", "R.polygon", 3)]
#[case("tile_lines", "R.overlap", 3)]
#[case("tile_lines", "R.apart", 0)]
#[case("tile_lines", "R.touching", 6)]
#[case("tile_lines", "R.bare", 15)]
#[case("array_flat", "R.bare", 150)]
#[case("array_flat", "R.uncovered", 50)]
#[case("array_flat", "R.polygon", 50)]
#[case("array_flat", "R.overlap", 50)]
#[case("array_ref", "R.bare", 150)]
#[case("array_ref", "R.uncovered", 50)]
#[case("array_ref", "R.polygon", 50)]
#[case("array_ref", "R.overlap", 50)]
#[case("extremes", "R.bare", 3)]
#[case("extremes", "R.uncovered", 1)]
#[case("extremes", "R.polygon", 1)]
#[case("extremes", "R.overlap", 1)]
#[case("array_tile_lines", "R.array", 1)]
#[case("bare", "R.bare", 3)]
#[case("bare", "R.edges", 4)]
#[case("covered_exact", "R.uncovered", 0)]
#[case("covered_exact", "R.polygon", 0)]
#[case("covered_short", "R.uncovered", 1)]
#[case("covered_short", "R.polygon", 1)]
#[case("union_exact", "R.uncovered", 0)]
#[case("union_exact", "R.polygon", 0)]
#[case("union_gap", "R.uncovered", 1)]
#[case("union_gap", "R.polygon", 1)]
#[case("diag_exact", "R.uncovered", 0)]
#[case("diag_exact", "R.polygon", 0)]
#[case("diag_out", "R.uncovered", 1)]
#[case("diag_out", "R.polygon", 1)]
#[case("parts", "R.uncovered", 3)]
#[case("parts", "R.polygon", 1)]
#[case("span", "R.uncovered", 1)]
#[case("overlap_touch", "R.overlap", 0)]
#[case("overlap_slim", "R.overlap", 1)]
#[case("partners", "R.apart", 1)]
#[case("partners", "R.touching", 2)]
#[case("array_grid", "R.array", 0)]
#[case("array_line", "R.array", 1)]
#[case("array_bent", "R.array", 1)]
#[case("array_reach", "R.array", 0)]
#[case("array_split", "R.array", 1)]
#[case("beyond_edge", "R.beyond", 0)]
#[case("beyond_out", "R.beyond", 1)]
#[case("labelled", "R.bare", 2)]
#[case("labelled", "R.label", 1)]
fn residual_rules_read_the_coincident_edge_and_their_ops(
    #[case] pattern: &str,
    #[case] rule: &str,
    #[case] expected: usize,
) {
    assert_eq!(
        count("residual", pattern, rule),
        expected,
        "{pattern}: {rule}"
    );
}
