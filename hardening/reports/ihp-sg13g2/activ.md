<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / activ: hardening report

Deck `activ` against SG13G2 Layout Rules Rev. 0.4, section 5.5 (Act.a-Act.e) and section
5.6 (AFil.a-AFil.j, with the derived layers of section 4.2).  106 layouts,
`tests/data/ihp-sg13g2/activ/<rule>.h<n>.gds.gz`, drawn by `gen/ihp_sg13g2/activ.rs`
(`hardening`), each with a `#[case]` in the `activ` table of `tests/ihp-sg13g2.rs`.  Every
layout ran through gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`hardening/oracle-ihp.sh`; the oracle runs the driver with `--no_density`, so the
density layouts have no KLayout column).

`show-deck` lists every rule of both sections; nothing is missing from the deck.

Reading the numbers below: gdscheck reports one marker per wall for `min_width` and
`max_width` (two per narrow bar, four per oversized square), one per violating pair for a
space and one per edge for an enclosure; KLayout's column in the oracle adds the driver's
tables, the driver's maximal run and the standalone maximal run, so Act.c/d/e count twice
there and its corner-to-corner pairs come as two or three edge pairs.  No count moved with
the tile size in any layout, and every flat/array pair agreed (KLayout's hierarchical run
counts an array cell once).

Test status on the engine as of this report: 16 of the 106 new cases fail, all on the
findings below; the other 90 pass.

## Resolution (2026-09-20)

Fixed, engine: 2 (`touching: separate`, a new area param: two shapes meeting at one
point are two regions, set on every IHP area rule as its deck reads them), 4 (`span:
narrowest`, a new `max_width` param: a shape is too wide where a value x value square
fits inside, KLayout's opening; on AFil.a, GFil.a and Pad.a1, whose upstream is that -
the metal-filler and LBE maxima upstream are bounding boxes and keep the old reading),
9 (several density layers are read as their union), 10 (the windows slide a merge tile
at a time from the die's corner with one laid against each far edge, off a summed-area
table; overlapping violating windows are one violation).  Fixed, deck: 3 (Act.e is the
area of `ActivHoleEmpty`, the hole less the Activ in it), 5 (AFil.c and AFil.c1 report
an abutment), 6 and 8 (enclosure entries on the fillers lying inside a NWell, an
nBuLay, a TRANS and a PWell block).

Settled on KLayout's reading, cases flipped: 1 (Act.c on the Activ past a gate's end,
both tools), the overlap halves of 5, 6 and 8 (a filler or Cont crossing the other
layer's edge shares area with it and is no pair - neither tool reports it), and the
AFil.e filler well inside a TRANS (enclosed by more than the value).  Open for the PDK
owner: 7 (nBuLay as drawn or as section 4.2 derives it; the case carries the drawn
reading).  11 is KLayout's miss, nothing to do.

The AFil.a and GFil.a fixtures of the old suite expected the wall-pair reading and
now carry a 5.005 x 5.005 filler for the narrowest one; the old metal and Activ
density fixtures were laid out per 800 µm grid cell and are uniform stripes now.

## Decided (2026-09-21, gdscheck owner)

7: nBuLay for AFil.d (and GFil.e) is section 4.2's derivation, `nBuLayDerived` = the
wells 3.0 µm and wider sized by 1.0 *inward* (IHP's `sized(-1.5).sized(0.5)`; the
block report's finding 9 - grown outward, every N+Activ 0.62 from a wide well would be
an NBL.e, and the reference designs' fillers gave 70 findings a design), or the drawn
nBuLay, less nBuLay:block.  A filler inside a wide well lies on or in that nBuLay.
Also: the chamfer 0.20 from a filler corner (AFil.j.h3) fires under the euclidian
enclosure now.

## Findings

### 1. Act.c reports the Activ past a gate's end (false positive, KLayout too)

Manual: "Act.c  Min. Activ drain/source extension  0.23"; figure 5.5 measures "c" from the
gate to the Activ edge along the channel.

Layout `Act.c.h3`: a GatPoly 8.4-8.56 × 1.7-2.9 ending 0.10 inside the Activ 8-9 × 2-3 (a
Gat.c violation, no transistor), beside a gate from two overlapping poly boxes with a 0.225
S/D and an S/D holding a hole 0.16 from the gate.

- gdscheck @20/7/100: Act.c 3 - "enclosure 0.1000 at (8.56, 2.9)-(8.4, 2.9)" on the poly
  end, plus the two intended ones (17.395, 2)-(17.395, 3) and (2.74, 6.25)-(2.74, 6.75).
- KLayout: the same three, in both decks (`GatPoly.ext_enclosed(Activ, 0.23)` reads every
  poly edge inside the Activ).
- Verdict: the 0.10 of Activ beyond a poly end is no drain/source; the layout is Gat.c's
  ("GatPoly extension over Activ") and Act.c should stay quiet.  Expected 2 (`act_c_h3`).
  The least important finding here: the marker lands on an illegal layout either way.

### 2. Act.d merges two boxes that touch at one corner (false negative)

Manual: "Act.d  Min. Activ area (µm²)  0.122".

Layout `Act.d.h2`: 0.3 × 0.3 boxes at (2, 5)-(2.3, 5.3) and (2.3, 5.3)-(2.6, 5.6), sharing
the vertex (2.3, 5.3) and nothing else; 0.09 µm² each, 0.18 together.

- gdscheck: nothing there (the other five sub-patterns fire as intended).
- KLayout: Act.d twice, "polygon: (2,5;2,5.3;2.3,5.3;2.3,5)" and "(2.3,5.3;...)".
- Verdict: two regions joined by a point are two regions; each is under 0.122.  Expected 7
  (`act_d_h2`).  The manual does not say so in words; a lithographer would.

### 3. Act.e measures the hole, not the enclosed empty area (false negative)

Manual: "Act.e  Min. Activ enclosed area (µm²)  0.15"; figure 5.5 draws the right-hand
"e" in a hole that holds an island.

Layout `Act.e.h2`: a ring 17-18.1 × 2-3.1 with a 0.5 × 0.5 hole (0.25) holding a 0.34 ×
0.34 island (0.1156), so the empty area enclosed by Activ is 0.1344.  (The island's 0.08
gaps are Act.b and its area Act.d, set aside.)

- gdscheck: nothing for that ring; six of the seven sub-patterns fire.
- KLayout: Act.e "polygon: (17.3,2.3;...;17.8,2.3/17.38,2.38;...)" - the hole minus the
  island (`(Activ.holes ...).ext_not(Activ).ext_with_area(< 0.15)`).
- Verdict: the figure's island is there for a reason: the enclosed area is what is empty.
  Expected 7 (`act_e_h2`).  A legal-spacing island can never bring a hole under 0.15
  (0.42 of gaps around anything is 0.176 µm²), so this only matters together with Act.b.

### 4. AFil.a reads a shape's long span as its width (false positive)

Manual: "AFil.a  Max. Activ:filler width  5.00".  A width is the distance between facing
walls of one shape (figure 4.1), and a rectangle's width is its smaller span - the same
notion AFil.a1 uses with the opposite sign.

Layout `AFil.a.h1`: a 5.005 × 5.005 square, a 5.005 × 5.0 rectangle, a 3 × 20 bar and an L
with 3-wide arms; `AFil.a.h2`: a 5.0 × 6 union of two boxes and a 6 × 6 filler with a 0.5
hole (2.75 of material all round); `AFil.a.h6`: a 300 × 5.005 bar; the 300 µm bars of
`AFil.a1.h1`, the rings of `AFil.a1.h3`, the 10 µm bars of the tile-line layouts.

- gdscheck: AFil.a on all of them - "width 20.0000 µm > 5 at (2, 10)-(2, 13)" on the bar,
  "5.0050 at (18, 2)-(18, 7)" on the 5.005 × 5.0, "10.0000 at (26, 2)-(26, 5)" on the L,
  "6.0000" eight times round the ring's hole, "300.0000" on the long bar; 12 markers in h1,
  28 in h2, 8 in h6.
- KLayout: AFil.a on the 5.005 × 5.005 square only in h1 (`sized(-2.5).sized(2.5)`), on
  the 5.02 diamond, the 5.02 strip, the 5.005 × 6 union and the 5.005 square from four
  boxes in h2, on the 5.005 bar and square in h6.
- Verdict: a 5.005 × 5.0 filler is 5.0 wide.  Expected 4 in `afil_a_h1`, 12 in `afil_a_h2`,
  30 in `afil_a_h3`, 6 in `afil_a_h6`; every other filler layout with a long shape sets
  AFil.a aside.  The existing fixture `AFil.a.gds.gz` (5.005 × 5.0 and 5.0 × 5.005, expected
  4) is read the same way: both shapes are 5.0 wide and clean under this reading.

### 5. A Cont or an Activ at no distance from a filler (false negative)

Manual: "AFil.c  Min. Activ:filler space to Cont, GatPoly  1.10"; "AFil.c1  Min. Activ:filler
space to Activ  0.42".

Layout `AFil.c.h3`: a Cont 20.9-21.06 × 2.4-2.56 across the right edge of the filler
20-21 × 2-3.  Layout `AFil.c1.h3`: an Activ 11-12 × 2-3 abutting the filler 10-11 × 2-3
along x = 11, and an Activ 14.8-15.8 × 2.2-2.8 across the edge of the filler 14-15 × 2-3.

- gdscheck: nothing for the three (`min_space` has no pair at a touch or an overlap).
- KLayout: AFil.c1 "(11,2;11,3)/(11,3;11,2)" for the abutting Activ; nothing for the two
  overlaps.
- Verdict: a space of zero is under the value; all three fire.  Expected 3 (`afil_c_h3`)
  and 2 (`afil_c1_h3`).  The same class as NW.d's crossing N+Activ in the nwell report,
  which became a `forbidden` entry.

### 6. AFil.d and AFil.e do not look inside (false negative)

Manual: "AFil.d  Min. Activ:filler space to NWell, nBuLay  1.00"; figure 5.6 draws "d"
twice, once from a filler outside the NWell and once from a filler *inside* it to the
well's edge.  "AFil.e  Min. Activ:filler space to TRANS  1.00".

Layout `AFil.d.h2`: a filler 0.5 inside a NWell's edge (2.5-3.5 × 3-4 in 2-5 × 2-5), one
0.5 inside an nBuLay, controls at 1.0, and fillers across a NWell's edge and across an
nBuLay's edge.  Layout `AFil.e.h2`: a filler inside a TRANS marker, one across its edge, a
TRANS diamond tip 0.995 above a filler.

- gdscheck: nothing in `AFil.d.h2`; the tip only in `AFil.e.h2`.
- KLayout: AFil.d "(2,2.134;2,4.866)/(2.5,3;2.5,4)" and "(15,...)/(15.5,...)" - the two
  inside cases, from the driver's `nwell_drw.enclosing(activ_filler, 1.0)`; nothing for
  the crossings; the tip only for AFil.e.
- Verdict: the figure asks for 1.00 inside as well as outside; a filler across the edge is
  at no distance, and a filler inside a device marker is what AFil.e exists to prevent.
  Expected 4 (`afil_d_h2`) and 3 (`afil_e_h2`).

### 7. AFil.d takes nBuLay as drawn, not as section 4.2 derives it (both directions, KLayout too)

Manual, section 4.2: "nBuLay = (((NWell ≥ 3.0 µm) sized by 1.0 µm/side) OR nBuLay:drawing)
AND NOT nBuLay:block".

Layout `AFil.d.h3`: a filler 1.5 right of a 3.0 × 3.0 NWell (no nBuLay drawn), one 1.5
from a 2.995 × 3.0 well, one 2.0 from a 3.0 well; a drawn nBuLay 2-4 × 9-11 wholly under
nBuLay:block with a filler 0.5 to its right; a drawn nBuLay 10-12 × 9-11 whose left half
is under nBuLay:block, with a filler 0.5 to its right and one 0.5 to its left.

- gdscheck: AFil.d "(4.5, 10.5)-(4.0, 10.5)" (the blocked nBuLay), "(9.5, 9.5)-(10, 9.5)"
  (the blocked half), "(12.5, 10.5)-(12, 10.5)" (the open half); nothing for the 3.0 well.
- KLayout: the same three (`nbulay_drw`).
- Verdict: the 3.0 well carries an nBuLay 1.0 wide around it, so the filler 1.5 away is
  0.5 from nBuLay and fires; the blocked nBuLay and the blocked half are no nBuLay and the
  two fillers beside them are clean; the open half fires.  Expected 2 (`afil_d_h3`).
  Whether the deck should derive nBuLay from wide wells for this rule is the gdscheck owner's
  call - the `nbulay` deck may already do it for its own rules.

### 8. AFil.i does not look inside PWell:block (false negative, KLayout too)

Manual: "AFil.i  Min. Activ:filler space to edges of PWell:block  1.50"; figure 5.6 draws
"i" on both sides of the block's edge.

Layout `AFil.i.h2`: a filler 1.495 inside a block's left edge (3.495-4.495 × 4-5 in
2-8 × 2-8, nSD:block and SalBlock round it so AFil.j is quiet), one 1.50 inside, and a
filler 21.5-22.5 across the edge of the block 22-28.

- gdscheck: nothing.
- KLayout: nothing (`Activ_filler.ext_outside(PWell_block).ext_separation(...)` looks at
  fillers outside the block only).
- Verdict: "space to edges" and the figure's inner "i" put the 1.495 filler and the
  crossing one in violation; 1.50 is clean.  Expected 2 (`afil_i_h2`).

### 9. AFil.g adds the Activ layers instead of taking their union (false positive)

Manual: "AFil.g  Min. global Activ density [%]  35.00", "AFil.g1  Max. ...  55.00"; the deck
counts Activ, Activ:filler and Activ.mask.

Layout `AFil.g.h1`: a 1000 µm chip with ten 30 µm stripes at a 100 µm pitch, every stripe
drawn on all three layers: 30 % of the chip is Activ.

- gdscheck: "Density: 90.00 %" - AFil.g1 once and AFil.g3 on all four windows; no AFil.g.
- KLayout: not run (density is off in the oracle); its deck takes
  `activ_drw.join(activ_filler).join(activ_mask)` with merged semantics, a union.
- Verdict: the same silicon drawn on three layers is 30 % of Activ: AFil.g fires, nothing
  else.  Expected AFil.g (`afil_g_h1`).

### 10. AFil.g2/g3 use a fixed 800 µm grid, clipped remainders included (false negative and false positive)

Manual: "AFil.g2  Min. Activ coverage ratio for any 800 x 800 µm² chip area [%]  25.00",
"AFil.g3  Max. ...  65.00".

Layout `AFil.g2.h2`: a 1000 µm chip, Activ everywhere except a 700 × 700 hole at (150, 150):
51 % globally; the window (100, 100)-(900, 900) holds the whole hole and 23.4 % Activ;
the windows on the 800 grid hold 34 % ((0, 0)-(800, 800)) and 80-94 % on the 200 µm
remainders.  Layout `AFil.g2.h3`: a 650 hole at (175, 175) in a plate 970 tall, 54.75 %
globally, every 800 × 800 window between 32.8 and 39 %.

- gdscheck: h2 - AFil.g3 on "(0, 800)-(800, 1000)" (79.69 %), "(800, 0)-(1000, 800)" and
  "(800, 800)-(1000, 1000)" (93.75 %), no AFil.g2; h3 - AFil.g3 on the same three
  remainders (75-90 %), nothing else.
- KLayout: not run; its density deck steps 800 µm windows by 400 µm and adds "backup"
  windows flush with the chip's far edges, never a clipped one.
- Verdict: "any 800 × 800 area" is a sliding window: the (100, 100) window fires AFil.g2 in
  h2 (the count is the tool's cut of a continuum of failing windows; the case expects one),
  and a 200 × 800 strip left over on the grid is no 800 × 800 area, so the AFil.g3 markers
  are false in both layouts.  Expected AFil.g2 (`afil_g2_h2`) and nothing (`afil_g2_h3`).
  Whether the sliding step should be 400 µm as IHP's deck has it or finer is a choice; the
  clipped remainders are not.

### 11. KLayout misses a 45° transistor's S/D one step under the bound (KLayout only)

Layout `Act.c.h2`: a transistor turned by 45° (Activ strip with ends on x + y = 4 and 10,
gate band between x + y = 4.325 and 4.555) whose lower S/D is 0.325/√2 = 0.2298; a second
with 0.2333 (clean); a third with 0.099.

- gdscheck: Act.c "0.2298 at (1.6625, 2.6625)-(2.1625, 2.1625)" and "0.0990 at (13.57,
  14.57)-(14.07, 14.07)"; nothing on 0.2333.
- KLayout: the 0.099 one only, in both decks; the 0.2298 is silent.
- Verdict: gdscheck is right on all three.  Recorded so nobody reads the oracle's silence on
  this layout as a gdscheck fault.

## Notes that are not findings

- A. AFil.b is "space", not "space or notch" (compare Act.b, NW.b, PWB.b, which say so).
  `AFil.b.h3` cuts a 0.415 notch into a filler in both orientations: gdscheck is quiet
  (the deck has `min_space` only), KLayout's `ext_space` reports both notches.  The case
  follows the manual's wording (expected 2: the facing Ls and the island).  If the PDK
  owner reads a filler notch as a space, add a `min_notch` entry and expect 4.
- B. AFil.j on a filler whose edge lies on the PWell:block's edge, enclosed 0.25 by both
  layers (`AFil.j.h2`, x = 18): KLayout reports two markers because it intersects
  nSD:block and SalBlock with the block before measuring; gdscheck reports nothing.  The
  manual asks for nSD:block and SalBlock enclosure, and they are there: clean (AFil.i
  fires on the coincident edge instead).
- C. AFil.j with an nSD:block chamfer passing 0.20 from a square filler's corner while both
  walls are 0.30 away (`AFil.j.h3`, corner (12, 4)): KLayout reports it ("(12.038,4.247;
  12.247,4.038)/(11.931,4;12,4)"), gdscheck does not.  Kept clean under the settled
  projection reading; noted because IHP's `ext_enclosed` is not as silent on corners as
  the nwell report found.
- D. Marker cuts: gdscheck's `max_width` marks every wall of an oversized square (four)
  and both long walls of a bar; `min_width` two per bar; the cases follow that.  KLayout
  splits a corner-to-corner space into two or three edge pairs and reports the gridded
  boxes of `Act.b.h4` as five pairs (its driver checks Act.b on unmerged input).
- E. `AFil.d.h1`'s wells are 2 µm wide on purpose (under the 3.0 from which section 4.2
  derives an nBuLay); `AFil.d.h2`'s are wider, which changes nothing there since the
  inside cases fire against the well itself.

## Tested and found clean or correct (no need to redo)

- Act.a: 0.15 vs 0.145 in x and y; 300 µm bars; 0.156 vs 0.148 diamond and 45° strip;
  chamfered box and L; unions of overlapping/abutting/gridded boxes; a ring with one
  narrow side; islands in a ring; bars on/straddling x = 20/21/40/42 and an L cornered on
  20; fifty flat and as GdsArrayRef; a 0.005 sliver; (1000, 1000); a comb; a U.
- Act.b: 0.21 vs 0.205; diagonal 0.145 (0.205) vs 0.15 (0.212); corner-on; diamond tip,
  45° strips, chamfer to corner, tip to tip; straight and 45° notches, comb, slot,
  keyhole hole, facing Ls, island in a ring (all eleven, matching KLayout); unions; gaps
  on/straddling tile lines incl. a corner on the line; fifty flat/array; sliver, 300 µm,
  (1000, 1000); Activ vs Activ:filler is AFil.c1's, vs Activ.mask nobody's, P+ vs N+ and
  two Activ under one poly are Act.b.
- Act.c: 0.23 vs 0.225 right/left/top/both; the 45° transistor (finding 11); a chamfer
  0.17 from a gate's corner with the walls at 0.64 (projection, clean in both tools);
  GatPoly beside or abutting the Activ; a union S/D from two boxes; a union gate from two
  boxes (0.23 clean, 0.225 once); a hole in the S/D; a gate across a sliver; nine tile-line
  transistors incl. two 10 µm wide; fifty flat/array; 300 µm wide, a 300 µm gate,
  (1000, 1000); two fingers; one gate across two Activs.
- Act.d: 0.122 vs 0.1205/0.121 in rectangles, an L and a diamond; union vs sum of areas;
  a 6 × 8 grid; abutting boxes; ring material; an island; tile lines incl. a 0.15 × 0.8
  bar across 20; fifty flat/array; a 0.01 sliver, a 0.005 × 30 sliver (0.15, clean),
  (1000, 1000); chamfered squares 0.14 vs 0.1175.
- Act.e: 0.15 vs 0.1485/0.149 holes; a diamond hole 0.157 vs 0.146; a chamfered hole
  0.14; two Cs closing a hole, and 0.005 apart not; an L-shaped hole of 0.1491; a ring in
  a ring's hole; a ring of 88 boxes; a keyhole polygon either way round; holes on/across
  tile lines, a hole in a 100 µm ring, a 0.15 hole and a 5.4 µm hole across x = 20, an
  open U 60 µm long (no hole); fifty flat/array; a 0.005 × 0.5 hole; (1000, 1000); a
  300 µm ring.
- AFil.a: the 5.005 square, the 5.02 diamond and strip, the 5.005 union and grid (all
  fire in both tools); the 4.95 diamond/strip and the 5.0 square across x = 20 are clean;
  tile lines; fifty flat/array; the 300 µm bar and the far square.
- AFil.a1: 1.0 vs 0.995; 300 µm; diamond/strip 1.004 vs 0.99; chamfered box; unions,
  slices, grid, ring side, island; tile lines and an L; fifty flat/array; sliver;
  (1000, 1000); comb and U.
- AFil.b: 0.42 vs 0.415; diagonal 0.417 vs 0.424; corner-on; tip to wall, strips, chamfer
  to corner, tip to tip; Ls and island; unions; tile lines; fifty flat/array; sliver,
  300 µm, (1000, 1000); filler to Activ/Activ.mask draws no AFil.b.
- AFil.c: 1.10 vs 1.095 to Cont and GatPoly; diagonal 1.096 vs 1.103; GatPoly chamfer
  and diamond tip; a Cont bar; GatPoly:filler is not GatPoly; a Cont on an Activ at 1.095
  with the Activ at 1.0; tile lines incl. a 10 µm GatPoly; fifty flat/array; a 300 µm
  GatPoly; (1000, 1000).
- AFil.c1: 0.42 vs 0.415; diagonal; corner-on; Activ diamond tip, Activ strip vs filler
  strip, filler chamfer vs Activ corner; Activ.mask is not Activ; tile lines; fifty
  flat/array; a 0.005 Activ sliver, 300 µm bars, (1000, 1000).
- AFil.d: 1.00 vs 0.995 to NWell and nBuLay; diagonals; NWell diamond tip and chamfer;
  the half-blocked nBuLay's open half; tile lines; fifty flat/array; 300 µm; (1000, 1000).
- AFil.e: 1.00 vs 0.995; diagonal; corner-on; diamond tip; tile lines; fifty flat/array;
  300 µm; (1000, 1000).
- AFil.i: 1.50 vs 1.495 outside; diagonal 1.499 vs 1.506; corner-on; block chamfer and
  diamond tip; tile lines; fifty flat/array; 300 µm; (1000, 1000).
- AFil.j: 0.25 vs 0.245 on nSD:block, on SalBlock, on both (one marker), nSD:block ending
  on the filler's edge; a filler outside any block; crossing or on the block's edge with
  0.25 all round (AFil.i's); a two-box nSD:block union 0.25 vs 0.245; a filler half under
  the block; parallel chamfers 0.244 vs 0.251; tile lines incl. a 10 µm filler; fifty
  flat/array; a 300 µm filler; (1000, 1000).
- Not tested: the SRAM marker (KLayout's Act.c excludes it; the manual's Act.c does not
  mention it); Activ:nofill (note 1 of section 5.6 is about the filler generator).
