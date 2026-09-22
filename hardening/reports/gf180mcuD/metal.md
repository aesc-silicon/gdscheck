<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / metal: hardening report

Deck `metal` against the GF180MCU design manual, section 7.13 (Metaln, n = 1 to 5; on
variant D's five-level stack the deck carries Metal1 to Metal4 and the top metal's own
section 7.15 carries Metal5).  12 layouts under
`tests/data/gf180mcuD/generated/metal/M*.h<n>.gds.gz`, drawn by `gen/gf180mcuD/metal.rs`,
each with a `#[case]` in the `hardening_metal` table of `tests/gf180mcuD.rs`.  Every
layout ran through gdscheck at tiles 20, 7 and 100 and through the upstream KLayout
runset (`hardening/oracle-gf180.sh`, `DECKS=metal`).

`show-deck` lists the section's rules per level: Mn.1 (width), Mn.2a (space and notch),
Mn.2b (space to wide metal), Mn.3 (area), and Metal1's SRAM-core variant.  No count
moved with the tile size in any layout.

## Findings

### 1. The wide-metal space is blind to a slot cut into the plate (false negative)

Manual: "Mn.2b  Space to wide Metaln (length & width > 10 µm)  0.3".

The rule is written as two layers, the drawn metal against the wide selection of itself.
A wide plate therefore lies on *both*, and a pair that shares area is no pair to a
spacing rule - so a slot cut into one plate, which the wide derivation leaves as two
wide regions of one drawn shape, was measured by nobody.

Layout `M1.2b.h3`: a 30 x 20 plate with a 0.295 slot cut into it from the top, 14.5 µm
of plate one side and 15.2 the other, both over 10 µm every way; below it the same slot
in a 9 µm plate, which is wide nowhere.

- gdscheck @20/7/100: nothing, before the fix.
- KLayout: nothing either (`metal1.separation(metal1_wide, 0.3)` has the same shape).
- Verdict: `M1.2b` once.  A slot in a wide plate is a space between two walls of wide
  metal, and the manual's "space to wide metal" says nothing about the two walls having
  to belong to different shapes.  Fixed with `pairs: any`, which measures an overlapping
  pair on its facing gaps (the parameter round 2 added for GF180's PL.5b).

### 2. One gap between two wide plates was reported twice (marker granularity)

Still Mn.2b, and the same cause read the other way: where *both* plates are wide, the
pair is found twice - once as (plate A drawn, plate B wide) and once as (plate B drawn,
plate A wide) - and the two markers named the same segment with its ends the other way
round, so the run's dedup kept both.

Layout `M1.2b.h5`: two 12 x 12 plates 0.295 apart.

- gdscheck @20/7/100: `M1.2b` twice, at `(13.0000, 1.0000)-(13.2950, 1.0000)` and at
  `(13.2950, 1.0000)-(13.0000, 1.0000)`.
- KLayout: nothing on this layout (its own reading of two wide plates; see the note).
- Verdict: one gap, one violation.  Fixed in the engine: a spacing marker names its two
  ends in a fixed order, so the two readings of one gap are one violation to the run's
  own dedup.  The same double-count was in the foundry MIM cases (MIMTM.1 4 -> 3 on
  variant D, MIM.1 4 -> 3 on variant A), where the pinned counts carried it.

## Notes that are not findings

- The SRAM core's exemption reads the *cell* marker, and the round's settled reading of
  a marker that classifies (Dualgate covering, not touching) decides which cores are the
  3.3 V kind: `M1.1.h1` draws all six relations and gdscheck and the manual agree on
  every one.
- Metal1 asks 0.23 where the levels above ask 0.28, in width and in space; `M2.1.h1` and
  `M2.2a.h1` draw the same 0.25 geometry on all four levels and only Metal1 is clean.
- The slot, dummy, blocked, label and resistor datatypes are not this section's metal
  (`M1.1.h3`, clean in both tools), and a ring's area is the metal it holds and not the
  ground its outline covers (`M1.3.h2`).

## Tested and found clean or correct (no need to redo)

Mn.1 at 0.225/0.23 and 0.25/0.28 per level; the SRAM core's six marker relations and a
bar half inside a core; the datatypes the section does not read; Mn.2a's space and notch
at 0.25; Mn.2b's "length & width > 10 µm" read strictly (10.0 and 30 x 9.995 are not
wide, 10.005 and 30 x 10.005 are), a narrow stub on a wide plate and its neighbour, the
gap opening on x = 20, 21, 40 and 42 with the plate crossing a line; Mn.3 at 0.1444 µm²
per level and the ring.
