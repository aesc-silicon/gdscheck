<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / topmetal1, topmetal2: hardening report

Decks `topmetal1` and `topmetal2` against SG13G2 Layout Rules Rev. 0.4, sections 5.22
(TM1.a-TM1.d), 5.23 (TM1Fil.a-TM1Fil.d), 5.25 (TM2.a-TM2.d, TM2.bR) and 5.26
(TM2Fil.a-TM2Fil.d).  The two decks share every rule but TM2.bR, so
`gen/ihp_sg13g2/topmetal.rs` draws each theme once for both with the deck's
own values: 59 layouts per deck (`tests/data/ihp-sg13g2/topmetal<n>/TM<n>.*.h<k>.gds.gz`,
`TM<n>Fil.*.h<k>.gds.gz`) plus 11 for TM2.bR, 129 in all, each with a `#[case]` in the
`topmetal1`/`topmetal2` tables of `tests/ihp-sg13g2.rs`.  Every layout ran through
gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`hardening/oracle-ihp.sh`; the oracle runs the driver with `--no_density`, so the
density layouts have no KLayout column).

`show-deck` lists every rule of the four sections; nothing is missing from either deck.

Reading the numbers below: gdscheck reports one marker per wall for `min_width` and
`max_width` (two per narrow or oversized bar, four per diamond), one per violating pair
for a space; KLayout's column in the oracle is the driver's tables plus the maximal deck
(which carries only the filler rules, so TMFil.a/b/d count twice there), its
corner-to-corner pairs come as two or three edge pairs, its driver reads the TM.b of
gridded boxes on unmerged input (five pairs for `TM<n>.b.h4`'s grid), and its
hierarchical run counts an array cell once.  TM1 and TM2 behaved identically on every
shared layout.  Apart from finding 5's abutting boxes no count moved with the tile size,
and every flat/array pair agreed.

Test status on the engine as of this report: 14 of the 129 new cases fail (5 per
shared deck, 4 on TM2.bR), all on the findings below; the other 115 pass.

## Resolution (2026-09-20)

Fixed, engine: 4 (the global density counts the metal inside the boundary alone), 5's
tile dependence and 6 (the run is read within the zone a tile's copies are exact in -
where a difference layer such as `TopMetal2NotIND` is exact, so the IND-cut line keeps
40, not 60 - and where a copy reaches the zone's edge the pair's regions are assembled
from the neighbouring tiles' cores out to `value + length + width` around the gap and
read again; the marker of a parallel pair is the shared stretch's lowest end, the same
point from every tile that sees it, which also ends a pre-existing double report of a
pair whose shape is drawn as two abutting boxes across a tile line), 7 (the depth
behind a wall is read stretch by stretch, so a 4 line with a 6 part on its far side is
wide along the part - the metaln report's finding 2).  Fixed, deck: 1 (TMFil.a1, MnFil.a2
and LBE.b are `max_length`, the bounding box's long side, exactly KLayout's
`with_bbox_max`; one marker per shape now, and the metal1 report's diamond fires), 2
(`abutting: report` on TMFil.c and TMFil.d), 3 (an enclosure entry on
`TopMetal{n}FillerInTRANS`, as the Activ and Metal filler decks have).

Decided (2026-09-21, gdscheck owner): the run is the manual's "lines" - along a wall, the
stretches of every wall facing it under the value are joined, so the stepped and the
nicked neighbours of 5 (`TM2.bR.h3`, `h11`) run 60 and fire, as the cases first
expected; KLayout's per-edge projection stays the stricter tool's miss.

## Findings

### 1. TMFil.a1 reads wall pairs, not the filler's extent (false negative)

Manual: "TM1Fil.a1  Max. TopMetal1:filler width  10.00" (TM2Fil.a1 the same); figure 5.23
draws "a1" along the *long* side of a rectangular filler, "a" along the short one, so a1
is the filler's extent, and the activ report's resolution keeps the metal-filler maxima
on the upstream bounding-box reading.

Layout `TM<n>Fil.a1.h2`: a diamond of half-diagonal 5.005 (extent 10.01, walls 7.08
apart) and a 6-wide 45° strip 8.49 long (extent 10.245, walls 6 apart, end caps 8.49);
beside them a 5.0 diamond and a 5.66-long strip (extents 10.0 and 8.245).

- gdscheck @20/7/100: nothing on any of the four (the rectangles of `h1`, `h3`, `h4` are
  all reported, on their long walls).
- KLayout: TMFil.a1 "polygon: (24,2.995;18.995,8;24,13.005;29.005,8)" and "(54,2;
  49.755,6.245;55.755,12.245;60,8)" (`with_bbox_max(10.001)`), the two large ones only.
- Verdict: both fire; the count is the tool's cut, the case expects one per shape
  (`tm<n>fil_a1_h2`).  Rectangles, Ls and pluses come out right either way.

### 2. TMFil.c does not report a filler abutting the metal (false negative)

Manual: "TM1Fil.c  Min. TopMetal1:filler space to TopMetal1  3.00".

Layout `TM<n>Fil.c.h3`: a 6 × 6 filler 2-8 × 2-8 and a metal 8-12 × 2-8 sharing the
edge x = 8; beside them a filler across a metal's edge and a filler inside a metal plate.

- gdscheck: nothing.
- KLayout: TMFil.c "(8,2;8,8)/(8,8;8,2)" for the abutment; nothing for the two overlaps.
- Verdict: a space of zero is under 3.00: the abutment fires, the overlaps share area and
  are no pair (settled with the activ report, whose AFil.c/AFil.c1 carry `abutting:
  report`).  Expected 1 (`tm<n>fil_c_h3`).  A deck entry, as there.

### 3. TMFil.d does not look inside a TRANS (false negative, KLayout too)

Manual: "TM1Fil.d  Min. TopMetal1:filler space to TRANS  4.90".

Layout `TM<n>Fil.d.h3`: a 6 × 6 filler 4-10 × 8-14 inside the TRANS 2-22 × 2-22, 2.0 from
its left edge; a filler 7 µm inside another TRANS; a filler across a TRANS edge.

- gdscheck: nothing.
- KLayout: nothing (a plain `ext_separation`).
- Verdict: the activ report's resolution gave AFil.d/AFil.e an enclosure entry for a
  filler lying inside the marker and enclosed by less than the value, and let a filler
  well inside (more than the value) and one across the edge through.  Read the same way
  here: the 2.0 filler fires, the other two are clean.  Expected 1 (`tm<n>fil_d_h3`).
  The metal-filler decks (`metal1`-`metal5`, `topmetal1`, `topmetal2`) all carry the
  plain `min_space` and would want the same entry.

### 4. Global density counts metal outside the EdgeSeal boundary (false positive and false negative)

Manual: "TM1.c  Min. global TopMetal1 density [%]  25.00", "TM1.d  Max. ...  70.00"; the
deck measures over `EdgeSeal.boundary`.

Layout `TM<n>.c.h2`: a 1000 µm boundary with a 1000 × 250 metal plate inside (25.000 %)
and a 1000 × 1000 plate beside it at x = 1100-2100.  Layout `TM<n>.c.h4`: the boundary
with a 1000 × 500 plate from y = −250.05 to 249.95, 249.95 of it inside (24.995 %);
`TM<n>.c.h3` the same with 250 inside (25.000 %).

- gdscheck @20/100: h2 "Density: 125.00 %", TM.d fires; h4 "50.00 %", nothing; h3
  50.00 %, nothing.
- KLayout: not run (density is off in the oracle).
- Verdict: the density is the metal *inside* the boundary over the boundary's area; a
  density over 100 % is not a density.  h2 is 25.000 % and clean, h4 is 24.995 % and
  fires TM.c (`tm<n>_c_h2`, `tm<n>_c_h4`; h3 is clean either way).  The existing
  `boundary_ok`/`boundary_ring` fixtures test the denominator only.  Artificial for a
  sealed die, but real for a bare block checked with a boundary drawn smaller than its
  metal.

### 5. TM2.bR takes the parallel run per edge, and a merged edge's run depends on the tile (false negative, tile-dependent)

Manual: "TM2.bR  Min. space of TopMetal2 lines if, at least one line is wider than
5.0 µm and the parallel run is more than 50.0 µm  5.00".

Layout `TM2.bR.h3`, four 6 × 60 lines each with a neighbour at a gap of 4: (a) a
neighbour whose facing wall steps at mid-height, 4 for 30 µm and 4.5 for 30; (b) one
whose facing wall carries a 0.5 µm long, 0.005 µm deep nick at mid-height; (c) one drawn
as two abutting 30 µm boxes; (d) a 2 line beside a 6 line drawn as two abutting 3 µm
strips.

Layout `TM2.bR.h11`: (a), (b) and (c) again with the neighbour a 2 (2.5 for the nick)
line beside a straight 6 line, the pairing KLayout's check can see (note C).

- gdscheck @20/7: h3 TM2.bR once, (d) at (96, 0)-(100, 0); @100: twice, (d) and (c) at
  (66, 0)-(70, 0) - `TILE-DEPENDENT`.  h11 @20/7: nothing; @100: (c) at (66, 0)-(70, 0).
  Plus TM2.b "notch 0.5000" at the nick in both, which is right (a 0.5 µm wide notch is
  under 2.00).
- KLayout: h3 (d) only (the other three pairs are two wide lines, note C); h11 (c)
  "(70,0;70,60)/(66,60;66,0)" - the merged 60 µm edge - and nothing for (a) and (b),
  whose per-edge projection is 30 and 29.75.
- Verdict: the lines of (a), (b) and (c) run 60 µm in parallel under 5 apart; the rule
  speaks of lines, not of edges, and a wall split by a step, a nick or a merge seam is
  still one line.  All four fire in h3 (`tm2_br_h3`: 4 × TM2.bR + TM2.b) and three in
  h11 (`tm2_br_h11`).  (c) is certain and tile-dependent: the union of two abutting boxes
  is one 60 µm edge at tile 100 (and to KLayout) and two 30 µm edges at 20 and 7 - the
  stitched merge keeps the seam vertex at (70, 30) as a break in the run.  (a)
  and (b) are the manual's reading against KLayout's per-edge one; if the gdscheck owner
  takes the latter the two cases drop two markers each.

### 6. TM2.bR: a line cut by IND keeps its full run (false positive)

Manual, note 1: "Not checked within IND regions"; the deck runs the rule on
`TopMetal2NotIND` (TopMetal2 minus IND).

Layout `TM2.bR.h5`: two 6 lines at a gap of 4 over 60 with an IND covering the top 20 µm
of the run (40 outside, `IND -2-18 × 40-62`); the same with the IND over the top 5 µm
(55 outside); a 6 line inside an IND beside a 2 line outside it.

- gdscheck: TM2.bR on the 40-outside pair "(6, 0)-(10, 0)" and on the 55-outside pair;
  nothing on the third.  Probing further: with the IND over the top 30 µm (30 outside)
  the pair still fires, while two plain 40 µm lines at the same gap are clean.
- KLayout: the 55 pair only, "(36,58;36,0)/(40,0;40,55)" (its wide layer is cut by the
  IND, which also lifts the shielding of note C); nothing on the 40 pair or the third.
- Verdict: outside the IND the lines run 40 in parallel, under the 50: clean; only the
  55 pair fires.  Expected 1 (`tm2_br_h5`).  The difference layer cuts the shapes but the
  run condition is still met - apparently measured on the uncut edges.  The reading is
  the deck's own and KLayout's (its wide layer is `not(ind_drw)` and the projection is of
  the cut edge); a reading that keeps the drawn lines' run and only silences the markers
  inside IND would flag the 40 pair - if the gdscheck owner prefers it the case flips, the
  engine does not.

### 7. TM2.bR does not see a line that is wide over most of its run (false negative)

Layout `TM2.bR.h10`: a 4-wide line 60 long with a 6-wide bulge on its far side over
y = 2.5-57.5 (55 of the 60), beside a 4 line at a gap of 4; beside it the same with a
bulge 20 long, one with the bulge on the near side (gap 4 over 20, 6 over the rest), a
300 µm pair of 6 lines at 4 and a pair at (1000, 1000).

- gdscheck: the 300 µm pair and the far pair only.  Probing: a bulge over 0-57.5, 1-59
  or even 0.005-60 stays silent; a 6 line with an extra tab fires.  The width condition
  wants the facing wall's opposite wall at 5 along the whole wall.
- KLayout: the bulged pair, "(56,0;56,60)/(52,57.5;52,2.5)" (`sized(-2.5).sized(2.5)`
  keeps the part a 5 × 5 square fits in, the projection against it is 55); the two 6/6
  pairs are shielded (note C).
- Verdict: the line is wider than 5 over 55 µm of parallel run at a gap of 4: fires.
  Expected 3 (`tm2_br_h10`); the two 20 µm bulges are clean under either reading.

## Notes that are not findings

- A. A chamfered corner is a width.  `TM<n>Fil.a.h2` holds 6 × 6 fillers with one corner
  chamfered by 1, 2, 3.5 and 4.  gdscheck reports "width 4.0000", "2.5000" and "2.0000"
  on the two walls the chamfer faces (six markers); KLayout the same three boxes as
  wall/chamfer edge pairs, e.g. "(12,17;12,20)|(16,20;17,19)".  The chamfer faces the
  opposite walls at 45° across 6 − c, within the settled 90° angle limit, and the box is
  indeed 6 − c wide along its top edge before the chamfer starts.  Drawn expecting them
  clean; the case follows both tools (12 markers).  The chamfered fillers of
  `TM<n>Fil.b.h2`, `c.h2` and `d.h2` draw the same TMFil.a, set aside there.
- B. TMFil.b is "space", not "space or notch" (compare TM1.b, TM2.b, which say so).
  `TM<n>Fil.b.h3`'s U with a 2.995 notch: gdscheck quiet, KLayout's `ext_space` reports
  it ("(10.995,8;10.995,14)|(8,14;8,8)").  The case follows the wording (2: the facing Ls
  and the island), as the activ report's AFil.b does.
- C. IHP's KLayout TM2.bR never reports two wide lines.  The driver checks
  `topmetal2_drw.sep(wide, 5.0, projection_limits(50.001))` where `wide` is derived from
  `topmetal2_drw` itself; KLayout's separation check is shielded by default, and when
  both lines are wide the copy of the primary line in the second layer lies on the
  pair's own edge and shields it.  `TM2.bR.fail.gds.gz` (two 6 lines), `h1`'s 6/6 pairs,
  `h6`, `h7`/`h8` and the 300 µm and far pairs of `h10` are all silent in KLayout;
  rerunning the driver's expression with `transparent` on the old fixture gives the two
  expected markers.  A wide line beside a narrow one is reported (h1's 5.005/2 pair,
  h2, h4's figure, h9's five, h10's bulge, h11's merged pair), and so is a wide line the
  IND cuts (h5).  Read the oracle's TM2.bR column with that in mind.
- D. The 45° crossing of `TM2.bR.h4` (a 6 line 80 long and a 6-wide strip at 45°
  passing 2.755 from its wall) is clean in gdscheck.  Lines at 45° to each other have no
  parallel run; a projection-based reading would give the strip's 60 µm and fire.  The
  case expects clean.
- E. TMFil.c and `TopMetal<n>.mask`: a filler 2.995 from a mask shape (`TM<n>Fil.c.h1`)
  draws nothing in either tool; the rule names the drawing layer.  The mask layer is
  "added to TopMetal1:drawing at mask" and counts for the density; whether a filler must
  keep 3.00 from it is the gdscheck owner's question, the case expects nothing.
- F. `TM2.bR.h6` (two 6 lines 1.995 apart over 60) reports TM2.b and TM2.bR both, as the
  manual gives no exclusion; the case expects both.
- G. The nick of `TM2.bR.h3` and `h11` is a TM2.b notch: a 0.5 × 0.005 nick in a wall is
  a notch 0.5 wide, and gdscheck's "notch 0.5000 at (40.0025, 29.75)-(40.0025, 30.25)"
  is right; KLayout reports it too.
- H. Counts: `min_width` two per bar and four per diamond; `max_width` two per bar, four
  per L or plus (one per wall of each oversized pair); the cases follow that.  KLayout's
  driver reports `TM<n>.b.h4`'s gridded plate as four edge pairs and `TM<n>Fil.a1.h3`'s
  shapes once each.

## Tested and found clean or correct (no need to redo)

Each item holds for TM1 (1.64) and TM2 (2.00) alike; the filler values are the same in
both decks.

- TM.a: the value vs one step under in x and y; 300 µm bars; a diamond and a 45° strip
  one on-grid step above and below w/√2; a chamfered box and an L with a chamfered
  inner corner; unions of overlapping/abutting/gridded boxes; a ring with one narrow
  side; an island in a ring; bars on/straddling x = 20/21/40/42 and an L cornered on 20;
  fifty flat and as GdsArrayRef; a 0.005 sliver; (1000, 1000); a comb; a U.
- TM.b: one step under vs the value in x and y; corner-to-corner one grid step either
  side of s/√2; corner-on; diamond tip to wall, 45° strips, chamfer to corner, tip to
  tip; U notch, straight-vs-45° notch, comb, slot, keyhole hole, facing Ls, island in a
  ring (eight, matching KLayout); unions and a gridded plate once each, overlapping boxes
  nothing; gaps on/straddling x = 20/21/40/42 and y = 20/21 and two corner-on; fifty
  flat/array; sliver, 300 µm, (1000, 1000); a pair joined through TopVia1/Metal5 or
  TopVia2/TopMetal1 fires like a bare pair (the rule names no net).
- TM.c/TM.d: the three layers count as their union (30 %, not 90 %; 70.000 % clean,
  70.005 % fires); a GdsArrayRef of stripes at 30 % and 24.995 %.
- TMFil.a: 5.0 vs 4.995; diamond and strip 5.006 vs 4.999; unions (overlap, slices, grid,
  ring wall, island); tile lines and an L; fifty flat/array; sliver; (1000, 1000); comb
  and U; the chamfers of note A.
- TMFil.a1: 10 vs 10.005 in x and y, 6 × 10.005, 6 × 300, (1000, 1000); a 10.0 diamond and
  an 8.245 strip clean; unions 6 × 10 vs 6 × 10.005, abutting 10.005, gridded 10.005, an L
  spanning 12, a plus spanning 14; tile lines and an L; fifty flat/array; a 0.005 × 10.005
  sliver.
- TMFil.b: 3.0 vs 2.995; diagonal 3.005 vs 2.998; corner-on; tip to wall, 45° strips at
  2.995 vs 3.002, chamfer to corner at 2.995 vs 3.002, tip to tip; Ls and island; unions
  and a gridded plate; tile lines; fifty flat/array; sliver, 6 × 10 pairs, (1000, 1000).
- TMFil.c: 3.0 vs 2.995 with the metal right, left, above; diagonal; corner-on; metal tip
  to filler wall, metal strip to filler strip, metal chamfer to filler corner and the
  reverse at 2.995 vs 3.002; a filler across a metal's edge and inside a plate (nothing);
  tile lines; fifty flat/array; a 300 µm metal bar, a metal sliver, (1000, 1000).
- TMFil.d: 4.9 vs 4.895; diagonal 4.900 vs 4.893; corner-on; TRANS tip to filler wall at
  4.895 vs 4.9, TRANS chamfer to filler corner, filler chamfer to TRANS corner and
  strips at 4.893; a filler 7 µm inside a TRANS and one across its edge (nothing); tile
  lines; fifty flat/array; a 300 µm TRANS bar, (1000, 1000).
- TM2.bR: the three bounds one step past (5.005 wide, 4.995 gap, 50.005 run) vs at the
  value (5.0 wide, 50.0 run; 5.0 gap in the old fixture); the run as the overlap of
  offset lines (50.005 vs 50.0) and a 20 µm neighbour; two 6-wide 45° strips at 3.999 vs
  5.003; the figure's plate beside a 2 line; a T (run 2); a 45° crossing (note D); a
  wide line inside IND beside a narrow one outside; with TM2.b at 1.995; fifty
  flat/array; 50.005 runs from x = 0, 20, 19.995, 7 and 40 (the tile lines) vs 50.0; a 6
  line with a 20 µm bulge, far or near side; 300 µm; (1000, 1000).
- Not tested: TopMetal.nofill and the slit layers (their rules live in the slit deck);
  TM2.bR between a line inside IND and one outside where both are wide.
