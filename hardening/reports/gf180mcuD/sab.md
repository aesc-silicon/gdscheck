<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / sab: hardening report

Deck `sab` against the GF180MCU design manual, section 7.10 (SB.1 to SB.16,
`gf180mcu_drm/drm_07_11.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  20
layouts, `tests/data/gf180mcuD/generated/sab/SB.<rule>.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/sab.rs`, each with a `#[case]` in the `hardening_sab`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through GlobalFoundries' own KLayout runset (`hardening/oracle-gf180.sh <layout> TOP
20 7 100`); the KLayout column below counts only the section's own ids, the runset's
density markers being noise on layouts this small.

`show-deck` lists 21 checks for the 16 rules of the section: SB.2 and SB.4 each carry two
(a space and a notch, a space and a forbidden region), and every id the manual states is
there.  Nothing in section 7.10 is missing from the deck.

Reading the numbers: gdscheck reports one marker per wall for `min_width` (two for a
narrow block), one per pair for a space, a notch, an enclosure or an overlap, and one per
region for a `forbidden`; KLayout cuts a width or a space into one edge pair and the
square metric of SB.14 into two.  **No count moved with the tile size anywhere in this
deck** - 20 layouts x 3 tiles, every rule identical, including the three layouts whose
geometry straddles x = 20.

Test status on the engine as of this report: 8 of the 20 new cases fail, each on a
finding below; the other 12 pass, and the deck's older good/bad pairs
(`every_rule_in_the_deck_fires`, `static_fixture`) are untouched and still green.

## Findings

### 1. A shared edge is not a space

*Rules SB.3, SB.4, SB.5a, SB.5b - every two-layer `min_space` of the section.*  The
settled GF180 reading is that a shared edge is a space of nothing and is reported.  Where
the block and its neighbour lie on **different layers** and share an edge, gdscheck reports
nothing; KLayout reports the pair.  Four such edges, in three layouts:

| layout | geometry | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- | --- |
| `SB.3.h2` | an unrelated COMP abutting a 3 x 3 block's right edge at x = 5 | 1 (only the 0.005 twin at x = 15) | 2, the second `(5,4.72;5,2.28)/(5,2.5;5,4.5)` | `SB.3` twice |
| `SB.4.h1` | a contact whose lower edge lies on a block's top edge at (17, 5) | 2 | 3, the extra at `(16.999,4.999;17.371,5.001)` | `SB.4` three times |
| `SB.5a.h2` | a field poly abutting a block's right edge at x = 5 | 1 (only the 0.005 twin) | 2, the second `(5,3.9;5,2.7)/(5,3;5,3.6)` | `SB.5a` twice |
| `SB.16.h2` | a block sharing the gate's right edge at x = 6: its field poly and its poly-on-COMP are a space of nothing away | `SB.5a` x2, `SB.5b` x1 - all from the 0.005 cell at x = 18.005 | `SB.5a` x4, `SB.5b` x2 | `SB.5a` x4, `SB.5b` x2 |

Verdict: a false negative of the engine.  The same layouts a grid step wider (0.005) fire
in both tools, so the check reaches the geometry and stops one step short of zero.  Note
that this is specific to a **two-layer** space: two blocks of `sab` sharing an edge merge
into one region, so the same-layer rule (SB.2) has no zero case to answer for, and SB.2's
own 0.415 pair and 0.415 notch both fire correctly (`SB.2.h1`).

### 2. "Unrelated" is read of the shape and not of the pair

*Rule SB.3, "Space from salicide block to unrelated COMP, 0.22".*  A block's 0.22 to an
active is a mask-alignment margin between that block's edge and silicide that must print:
whether the active happens to carry *another* block somewhere else does not change it.
The manual's word is "unrelated", which reads of the pair - this block and this COMP.

`SB.3.h1` draws it: one 10 µm COMP bar at y = 12-14, block A across it at x = 3.5-6.5, and
block B at (8.0, 14.215)-(10.0, 16.215), which is 0.215 above the bar and touches no part
of it.  Both decks build the layer as "COMP not overlapping *any* block"
(`comp_unrelated_sab`, upstream `comp.not_overlapping(sab_out_otp)`), so the bar is
related to A and therefore related to B as well, and the 0.215 goes unreported.  gdscheck
1 marker (the 0.215 pair at the top of the layout), KLayout 1, at every tile size.

Verdict: a false negative shared with the runset.  Its shape - an active that runs under
one block and past another - is ordinary in an I/O row, where several unsalicided devices
sit on one diffusion.

### 3. A COMP or a poly wholly inside the block has no extension, and no rule says so

*Rules SB.7 ("COMP extension beyond related salicide block, 0.22") and SB.10 ("Poly2
extension beyond related salicide block, 0.22").*  Both are stated of the shape the block
sits on: it has to run 0.22 past the block.  A shape that lies wholly **inside** the block
runs past it by nothing at all, which is the worst case the rule can have.

| layout | geometry | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- | --- |
| `SB.7.h1` (a) | a 2 x 2 COMP island at (4,4)-(6,6) inside a 6 x 6 block at (2,2)-(8,8) | 0 | 0 | `SB.7` |
| `SB.10.h1` (a) | a 2 x 2 poly island in the same block | 0 | 0 | `SB.10` |

Both decks read the rule as an enclosure of the block by the shape, and a shape that never
crosses the block's boundary offers no wall to measure, so the pair is skipped outright.
The same block is clean on SB.6 and SB.9 (it extends 2.0 past the island, well over the
0.22), so the layout leaves with no marker at all - a block drawn entirely over an active
with no diffusion left to make contact to passes the section.

Verdict: a false negative shared with the runset, and the one case of the four extension
rules where nothing catches the geometry.

### 4. A coincident wall is the block's fault on COMP and the shape's fault on poly

*Rules SB.6/SB.7 and SB.9/SB.10, the two mirrored pairs.*  Where a bar crosses a block and
one of its walls is exactly on the block's wall, the extension is nothing in **both**
directions: the block does not run 0.22 past the bar, and the bar does not run 0.22 past
the block.  The two decks each report one of the two, and not the same one:

| layout | geometry | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- | --- |
| `SB.7.h1` (b) | a COMP (12,4)-(20,6) crossing a block (12,2)-(18,8), left walls both on x = 12 | `SB.6` at `(12,6)-(12,4)` | `SB.6`, the same pair | `SB.6` and `SB.7` |
| `SB.10.h1` (b) | the same picture on poly | `SB.10` at `(12,6)-(12,4)` | `SB.10`, the same pair | `SB.9` and `SB.10` |

The asymmetry is in the deck: SB.7 and SB.9 carry `skip_coincident` and SB.6 and SB.10 do
not, and upstream's `enclosed`/`.polygons` pairs come out the same way.  So identical
geometry is called the block's mistake when the bar is COMP and the bar's mistake when it
is poly.

Verdict: left as it stands in the case, since which rule id a coincident wall earns is a
labelling question and the engine's `skip_coincident` is a settled decision of its own;
recorded here because the two halves of a mirrored pair should not carry it differently.
The `SB.7.h1` case does expect `SB.7` - for sub-pattern (a), finding 3.

### 5. A marker that merely touches the block exempts the whole block

*Rules SB.12 ("Overlap with Poly2 **outside ESD_MK**") and SB.16 (the block "can only
exist on CMOS transistors marked by LVS_IO, OTP_MK, ESD_MK").*  Both decks select whole
blocks - `sab_out_esd` is `sab_out_otp not_overlapping esd_mk`, `sab_core` is the same
against `lvs_io or esd_mk` - so a marker overlapping any part of a block takes the block
out of the rule.

SB.12's wording is the clearer of the two: it names the overlap that lies outside the
marker.  `SB.12.h1` (c) draws a poly bar with a block reaching only 0.215 onto its right
end, and ESD_MK at (9.285,13)-(11.285,17) covering the block's far half - the 0.215
overlap itself, at x = 7.785-8.0, is nowhere near the marker.  Both tools are silent;
gdscheck 2 markers on the layout (the bare copy and the copy with ESD_MK abutting),
KLayout 2, at every tile size, where the manual asks 3.

SB.16's is a reading rather than a finding, and the case leaves it clean: `SB.16.h1` (f)
has LVS_IO over the block's left half and over part of the gate, so whether the transistor
counts as "marked" is genuinely open.  Recorded so nobody redraws it.

Verdict for SB.12: a false negative shared with the runset.

## Tested and clean

Everything below was drawn, run at three tile sizes and behaves as the manual says, in
both tools; it does not need drawing again.

- **The OTP marker**, which the manual never mentions and the deck uses as the section's
  general exemption.  `SB.1.h1`: a 0.415 block under OTP_MK still fires SB.1 (the width
  rule reads the drawn layer), and a 0.42 one is clean.  `SB.2.h1`: a 0.415 pair under one
  OTP_MK is exempt, the same pair with OTP_MK *abutting* one block is not (a marker beside
  a block does not mark it), 0.42 is clean.  `SB.4.h1`: a contact on a block under OTP_MK
  is SB.8 and not SB.4; on a bare block it is both.  `SB.5a.h1` and `SB.13.h1` carry the
  same exemption for a space and for the area.
- **SB.4 and SB.8 apart**: 0.145 fires, 0.15 is clean, a contact touching the block's edge
  is SB.4 and not SB.8 (nothing of it lies on the block - modulo finding 1).
- **SB.5a against SB.5b**: `SB.5.h1` draws a poly crossing a COMP's edge, so its lower
  strip is field poly and its upper strip is poly-on-COMP; the block reads 0.29 to the
  first (SB.5a fires) and 0.59 to the second (SB.5b silent), and the same at 0.30/0.60 is
  clean.  `SB.5.h2` draws one poly line cut in two by a COMP bar with a block on the left
  piece only: the right piece is bare poly and owes the 0.3, which both tools report -
  the section's "unrelated" read of the piece, which is the physical reading, salicide
  blocking being local.
- **SB.11 and SB.12 are not the same rule**: `SB.12.h1` (b) shows ESD_MK exempting SB.12
  where SB.11 (which keys on OTP_MK alone) would still apply.
- **SB.13 counts the block's own area**: a 1.5 x 1.5 ring with a 0.515 hole is 1.985 µm²
  and fires, although its outline is 2.25; 1.96 fires, 2.002 is clean.
- **SB.14a exempts a butted pair**: `SB.14a.h1` (a) is one poly bar under one block, N+ on
  the left half, P+ on the right, butted on a shared edge - a poly diode, which has no
  corner to keep clear.  Neither tool reports it, upstream by an explicit
  `not_interacting` on the butting edge and gdscheck by reading the merged region.  The
  corner case (b), two unsalicided regions 0.5 in x and 0.5 in y apart, fires in both: the
  rule is stated as a 0.56 square and reads as one, the euclidian distance there being
  0.707.
- **SB.14b's square** the same way: 0.5 by 0.5 from a P-channel gate's corner fires,
  0.565 by 0.565 does not.
- **SB.15a against SB.15b**: `SB.15a.h1` puts a P+ plate under the block but never on the
  poly, so it is unrelated - 0.175 fires, 0.18 is clean.  `SB.15b.h1` is the poly
  resistor's picture: two N+ heads for the contacts, 0.315 from the block (two markers,
  the rule's own purpose) and 0.32 (clean).  Its (c) is the case the deck's header
  worried about - one N+ covering the whole bar with a 0.2 margin, containing the
  block-on-poly region rather than facing it - and neither tool calls that a space.
- **SB.16's exemptions**: bare fires; LVS_IO, ESD_MK or OTP_MK over the block exempts;
  RES_MK over the poly-on-COMP takes the transistor away and exempts; a block whose left
  edge lies on the gate's right edge fires (a shared edge counts as being on the gate,
  which is the same reading finding 1 asks for in the space rules) and the same block
  0.005 clear does not.
- **The tile lines**: `SB.15a.h1`, `SB.15b.h1` and `SB.16.h1` each put a pattern across
  x = 20; at tiles 20, 7 and 100 every count is the same, in this deck's rules and in the
  runset's.

## Resolution (2026-09-22)

- **1 (a shared edge)**: fixed in the deck; SB.3, SB.4, SB.5a and SB.5b report an
  abutting pair.
- **2 ("unrelated")**: fixed in the deck.  SB.3 reads the plain COMP layer, so
  relatedness is decided per pair: a pair that shares area is the block's own and is
  skipped, every other pair is measured - and a COMP drawn edge to edge with a block
  shares no area with it, which is finding 1's case.
- **3 (a shape wholly inside the block)**: fixed in the deck; SB.7 and SB.10 get a
  `forbidden` half on the contained COMP and the contained poly, which is the worst
  case each rule has.  A block over a whole gate is SB.16 and SB.10 by one geometry,
  which the isolation test now allows as twins.
- **4 (`skip_coincident` on opposite members)**: left as it stands, as the report
  proposes - which id a coincident wall earns is a labelling question.
- **5 (the ESD marker)**: fixed in the deck; SB.12's `sab_out_esd` takes the marker
  away geometrically, since the rule names the overlap *outside* it.

Foundry case `sab.gds.gz`: SB.3 9 -> 11, SB.4 6 -> 7, SB.5a 9 -> 12, SB.5b 7 -> 8 (the
touches), SB.7 17 -> 26 and SB.10 61 -> 97 (the contained shapes).  The first four move
toward KLayout's counts; the last two are the half neither tool had.
