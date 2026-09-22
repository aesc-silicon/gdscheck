<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / mslot: hardening report

Deck `mslot` against section 14.6.3 of the GF180MCU design manual, "Metal Slotting
rules".  The section is one nine-row table applied once per metal level; variant D is a
five-level stack, so `show-deck` lists 49 rules - the table for Metal1 to Metal5, less
Metal5's `.7`, which would be the via above the top metal and does not exist.  That
omission is right: the runset registers `mslot#{lvl}` only while `metal_level_numerical
>= lvl` and sets `via_above` empty when the level *is* the top, so MSLOT5.7 and the whole
MetalTop table are correctly absent for this variant.

The manual's table is not in the local checkout (`gf180mcu_drm/drm_14_6_3.rst` points at
a CSV that was never downloaded), so it was read from the published manual at
`gf180mcu-pdk.readthedocs.io/.../drm_14_6_3.html`:

| Rule | Description | Value |
| --- | --- | --- |
| MSLOT.1 | Maximum metal width without slotting | 30 |
| MSLOT.2 | Minimum slot width (slot mark layers) | 2 |
| MSLOT.3 | Slot length (slot mark layers) | min 10, max 250 |
| MSLOT.4 | Slot space (slot mark layers) | **min 10, max 30** |
| MSLOT.5 | Minimum slot (slot mark layers) to metal edge spacing | 10 |
| MSLOT.6 | For multiple slots that spans the metal width, slots should be staggered | — |
| MSLOT.7 | Minimum space from via-n to metal-n slot | 0.2 |
| MSLOT.8 | Minimum space from via-(n-1) / contact to metal-n slot | 0.2 |
| MSLOT.9 | Minimum distance to: MetalTop under Pad; MIM bottom plate under FuseTop; MetalTop under FuseWindow_D; **Metal3 area (for MIM top plate connection) directly enclosed by FuseTop**; MetalTop under UBM layers | 5 |
| MSLOT.10 | **Slot mark layer on the metal hole (same metal level) are not allowed** | — |

The bold entries are in the manual and in neither deck.  `.6` is exempt: Appendix B,
"Rules not coded", lists it by name ("14.7.3.6 For multiple slots that spans the metal
width, slots should be staggered"), so its absence is sanctioned.  `.4`'s maximum, `.10`
and `.9`'s Metal3 region are not exempt.

20 layouts under `tests/data/gf180mcuD/generated/mslot/MSLOT.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/mslot.rs`, each with a `#[case]` in `hardening_mslot`.  Every layout ran
through gdscheck at tiles 20, 7 and 100 and through the upstream runset (`DECKS=mslot,
mcell`).  **No count moved with the tile size** - which is worth saying for this deck in
particular, because `.1` reads the metal through a 15 µm shrink and a 15 µm grow, twice
the width of a 7 µm tile (`MSLOT.wide.h5`, three plates across x = 7 … 35, across x = 42
and at (1000, 1000): 3 markers at every tile size, and the runset agrees).

## Findings

### 1. `.4`'s 30 µm maximum is missing (false negative, missing rule)

The manual gives slot space a minimum of 10 µm **and a maximum of 30**; the deck carries
`min_space: 10.0` alone, and the runset only `space < 10.um`.

`MSLOT.space.h3` is two 54 x 30 µm plates, each with two legal 2 x 10 µm marks: 30.0 µm
apart in the first and 30.005 in the second, both keeping `.5`'s 10 µm of metal all
round.  The plates are 30 µm tall, so no part of them is wide metal and `.1` cannot stand
in for the missing maximum.

- gdscheck at 20 / 7 / 100: silent.
- KLayout: silent.

**Verdict: a gap in both decks.**  The maximum is the point of the rule - a wide plate
with slots 40 µm apart has 40 µm of unrelieved metal between them, which is the stress
the section exists to prevent, and `.1` does not catch it because the metal between two
slots is under 30 µm *wide* only if it is under 30 in both directions.  The case expects
one `MSLOT1.4`.

### 2. `.10` is missing: a slot mark on a hole in the metal (missing rule)

"Slot mark layer on the metal hole (same metal level, e.g. Metal1_Slot on the Metal1
hole) are not allowed."

`MSLOT.hole.h1` is a 42 x 60 µm Metal1 ring with a 22 x 40 hole - the arms are 10 µm, so
nothing here is wide metal - and one legal 2 x 20 mark in the middle of the hole, 10 µm
from every wall of it.  That margin is exactly `.5`'s bound, so by the letter of `.5` the
mark is clean, and by `.10` it is forbidden.

- gdscheck at 20 / 7 / 100: `MSLOT1.5` x 1 - "shape on metal1_slot_rect not enclosed by
  metal1_drawn at (31.0, 40.0) µm".
- KLayout: silent.  `metal_slot_rectangles.drc(enclosed(metal_drawn) < 10.um)` measures
  nothing where the mark and the metal do not overlap.

**Verdict: gdscheck reaches the right answer under the wrong id, the runset misses it
entirely.**  gdscheck's `min_enclosure` reports a shape with no enclosing layer under it,
and a mark in a hole is exactly that - so the layout does not pass silently today.  But
the manual gives the case its own id, and under `.5` alone the reading is arguable: a
mark in a hole *is* 10 µm from every metal edge.  The case expects `MSLOT1.10`; whoever
adds it should decide whether `.5` keeps firing alongside it.

### 3. A shared edge is not a space (false negative)

The round's settled reading is that a shared edge is a space of nothing and is reported.
This deck's `min_space` rules between two layers do not report it - except at a corner.

`MSLOT.via.h3`: a Metal1 plate with a legal 2 x 20 mark at x = 20 … 22, a Via1 bar at
x = 22 … 22.26 sharing the whole of the mark's right edge, a contact at x = 19.74 … 20
sharing the whole of its left edge, and on a second plate a Via1 touching one corner of
the mark at (67, 40) and nothing else.

- gdscheck at 20 / 7 / 100: `MSLOT1.7` x 1 - the *corner*, "space 0.0000 µm < 0.20 µm …
  at (67.0000, 40.0000)-(67.0000, 40.0000)".  Neither shared edge is reported.
- KLayout: `MSLOT1.7` x 3 and `MSLOT1.8` x 1 - the shared edge at x = 22
  (`(22,40;22,20)/(22,20;22,40)`), the corner twice (one marker per edge pair), and the
  shared edge at x = 20 for `.8`.

The same class appears twice more:

- `MSLOT.via.h2`, a Via1 abutting part of the mark's right edge at x = 67: gdscheck
  silent, KLayout `MSLOT1.7` x 1 (`(67,29.46;67,28.8)/(67,29;67,29.26)`).
- `MSLOT.dont.h2`, the third plate, where the keep-out is drawn 5 µm from the mark so
  that the *grown* keep-out shares the mark's edge: gdscheck silent, KLayout `MSLOT1.9`
  x 1 (`(112,40;112,20)/(112,22;112,38)`).

**Verdict: gdscheck is wrong.**  The runset, the settled reading and the manual all agree
that zero is under 0.2.  That gdscheck reports the *corner* touch and not the shared edge
makes it a cut in the space code rather than a deliberate reading.

### 4. A via inside a slot mark, or straddling its edge, is reported by neither

`MSLOT.via.h2` also carries a Via1 wholly inside the mark (x = 110.8 … 111.06, mark
110 … 112) and one straddling its left edge (x = 154.87 … 155.13, mark 155 … 157).

- gdscheck at 20 / 7 / 100: silent.
- KLayout: silent.  `separation()` needs a gap to measure.

**Verdict: both decks are wrong, and this is the worst case the rule has.**  The space
from the via to the slot is not 0.195 µm, it is negative: the slot is cut out of the
metal at mask generation, so a via that lands on one has no metal under it at all.
Nothing else in the deck sees it either - the slot mark does not remove metal in the
database, so the via's own enclosure rules are satisfied.  `MSLOT.dont.h2`'s first two
plates are the same class for `.9`: a pad opening lying on the mark, and one whose grown
keep-out covers it.  The case expects three `MSLOT1.7` on `via.h2` (the abutting one of
finding 3 and these two) and four `MSLOT1.9` on `dont.h2`.

### 5. `.5` on a mark with no metal under it: gdscheck reports, the runset does not

`MSLOT.enc.h2` puts one mark half in and half out of the plate, one with no metal at all,
one inside a plate drawn on the *dummy* datatype (which is `metal1_dummy`, not
`metal1_drawn`, so it is not this deck's metal on either side), and one 8 µm from the
wall of a notch in the plate.

- gdscheck at 20 / 7 / 100: `MSLOT1.5` x 4 - three "not enclosed" and one "enclosure
  8.0000 µm < 10.00 µm … at (152.0000, 30.0000)-(152.0000, 40.0000)".
- KLayout: `MSLOT1.5` x 1, the notch alone (`(152,40;152,24)/(160,46;160,30)`).

**Verdict: gdscheck is right**, and this is the settled enclosure reading of the round
applied to a mark that never enters the enclosing layer at all.  Recorded so nobody reads
the runset's silence as a specification.

### 6. `.9`'s fifth keep-out region is missing (false negative)

The manual lists five regions under `.9`; the runset's `dont_slot` unions four of them
and leaves out "Metal3 area (for MIM top plate connection) directly enclosed by FuseTop".
The deck copies the runset's four.

`MSLOT.dont.h2`, fifth plate: a 6 x 6 µm Metal3 island wholly inside a 10 x 10 FuseTop
region, 9.995 µm from a legal Metal1 mark - which is 4.995 µm once the region is grown by
the 5 µm the other four terms are grown by.

- gdscheck at 20 / 7 / 100: silent.
- KLayout: silent.

**Verdict: a gap in both decks**, inherited from the runset.  Whoever adds it should
check the level: the rule is written in the manual's MetalTop naming, and on this 5LM
stack the MIM bottom plate is Metal4 (as the runset's `mim_bottom_under_fusetop` has it),
so "Metal3" may well be a level relative to the top rather than the literal layer.

## Notes that are not findings

- `.1` is "a 30 µm square fits inside the metal", not "some side is over 30 µm".  The
  runset opens the metal with `sized(0,-15).sized(-15,0).sized(0,15).sized(15,0)`, and
  the deck copies it as four directional virtual layers.  `MSLOT.wide.h1`: 30 x 30 clean,
  30.005 x 30.005 fires, 30.005 x 30.0 clean, a 40 x 20 plate clean, an L of two 40 x 20
  arms clean.  The manual's words - "any metal line that is wider than 30µm" - would take
  the 40 µm side of the fourth at face value; both tools read the square, and a 40 x 20
  plate is a 20 µm-wide line by any normal use of the word.  Kept as the runset has it.
- Three things relieve `.1`, all at the bound and all agreeing with the runset: a slot
  mark (`wide.h2`, 62.00 µm plate clean, 62.01 fires twice - the two halves are two
  regions), the vias with their 0.2 µm keep-out (`wide.h3`, a 1.7 µm via bar leaves
  29.955 µm halves and is clean, a 1.5 µm bar leaves 30.055 and is not), and a `.9`
  keep-out (`wide.h4`, a 2 µm pad opening over the top metal grows to 12 µm and leaves
  25 µm halves; the same opening with no top metal under it is no keep-out, and the
  unbroken plate is one marker, not two).
- Only rectangles reach the rules after `.0`.  An L-shaped mark is `.0`'s and nobody
  else's: it does not take part in `.4` from 5 µm away (`space.h1`), and its foot 4 µm
  from the plate's edge is not a `.5` violation (`all.h1`).  Two abutting or overlapping
  marks that merge into one rectangle are clean.
- `.2` is the mark's short bbox side and `.3` its long one, whichever way round the mark
  lies, and a 2 x 2 square is at `.2`'s bound and under `.3`'s 10 µm (`dim.h1`).
- The per-level via map is right on all five levels (`via.h1`, 9 markers): `.7` is the
  via above the metal, `.8` the one below, the one below Metal1 is the contact, and
  Metal5 has no `.7`.  A Via2 0.195 µm from a Metal1 mark is read by nothing (`via.h2`).
- `.9`'s four implemented regions are each built from the right pair of layers
  (`dont.h1`, 11 plates, 6 markers): Metal5+Pad, Metal4+FuseTop, Metal5+FuseWindow_D and
  Metal5+each of the three UBM layers fire; Pad alone, Metal5 alone, Metal5+FuseTop (the
  wrong metal for that term) and Metal4+Pad (likewise) do not.  The 5 µm growth is real:
  a drawn opening 10.0 µm from a mark is clean and 9.995 is not.
- `top_metal` is Metal5 for this variant, drawn plus dummy, matching the runset's
  `METAL_STACK_MAP[5]`.

## Tested and found clean or correct (no need to redo)

`.0`'s rectangle split on all five levels; `.2` at 2.0/1.995 in both orientations; `.3`
at 10/9.995 and 250/250.005, and on a 2 µm square; `.4` at 10/9.995, across x = 20 and
x = 42 and with a mark's edge on x = 20, and against a non-rectangular mark; `.5` at
10/9.995 on all five levels, against a notch wall, against dummy metal and with no metal
at all; `.7` and `.8` at 0.2/0.195 on every level and in both directions; `.9`'s six
region recipes and its five negative controls, at 10/9.995; `.1` at 30/30.005 in both
directions, on all five levels, relieved by a mark, by vias and by a keep-out, and across
the tile lines.  Nothing in this deck is tile-dependent at 20, 7 or 100.
