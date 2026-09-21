<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / metal1: hardening report

Deck `metal1` against SG13G2 Layout Rules Rev. 0.4, section 5.16 (M1.a-M1.k), section
5.18 (M1Fil.a1-M1Fil.k) and the sealring waiver of section 6.10.  96 layouts,
`tests/data/ihp-sg13g2/metal1/<rule>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/metal1_hardening.rs`, each with a `#[case]` in the `metal1` table of
`tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through IHP's KLayout decks (`hardening/oracle-ihp.sh`; the density layouts through
`gdscheck run --deck metal1`, since the oracle's suite and KLayout run leave density out).

`show-deck` lists every rule of both sections.  One of them runs on the wrong layer
(finding 6).

Reading the numbers below: gdscheck reports one marker per wall for `min_width` and
`max_width` (two per narrow bar, four per narrow diamond), one per pair for a space rule,
one per shape for M1.d, M1.c and M1.c1; KLayout's column in the oracle adds the driver's
tables and the maximal deck, so M1.c1, M1.d, M1Fil.a1/b/d count twice there, and it cuts a
corner-to-corner pair into two or three edge pairs.  No count moved with the tile size in
any layout, and every flat/array pair agreed (KLayout's hierarchical run counts an array
cell once).

Test status on the engine as of this report: 15 of the 96 new cases fail, all on the
findings below; the other 81 pass, as do the 16 older cases.

## Resolution (2026-09-20)

Fixed, engine: 1 (a shape is enclosed only if none of the enclosing layer's walls
crosses one of its own - a Cont across a slot in the metal has its corners on metal
and a strip over the slot, which the slot's walls give away).  Fixed, deck: 2 (M1.c1
reads `sides: adjacent` with `trigger: 0.05`, KLayout's `one_side_allowed,
two_opposite_sides_allowed`; the metaln report's finding 1, applied to every Metal and
Via deck, and the older `M{n}.c1`/`V{n}.c1` fixtures' clean via has its 0.05 sides
opposite now), 3 (M1.a, M1.b, M1.d, M1.e, M1.f, M1.g and M1.i run on
`Metal1NoSealring`), 4 (the density rules read `Metal1Density` = Metal1, its filler and
its mask less Metal1:slit, as IHP's density deck; the same for Metal2-5 and the
TopMetals), 5 (`abutting: report` on M1Fil.c and M1Fil.d, and on the Activ:filler marker
rules), 6 (M1Fil.d on Metal1:filler, with the enclosure entry on `Metal1FillerInTRANS`
for a filler lying inside the marker).

The bar crossing the seal's edge in `M1.seal.h1` is cut at the edge and its outside
part reported, as the case has it: `Metal1NoSealring` is the geometric difference, like
the via layers.  Dropping whole regions overlapping the seal (the DigiBnd reading, and
KLayout's `outside` for M1.c1/M1.d) was tried and costs a stitch of every region with a
piece in a border tile - the power net of a whole die - per metal rule: 100 s on the
4 mm² design.
Note G (a 0.045 endcap on a line wider than the Cont) stays as both tools read it.

## Decided (2026-09-21, PDK owner)

A crossing shape is cut at the marker's edge and its outside part read: the seal ring
is isolated from the chip, its own via rings would fail the standard rules anyway, and
the geometric difference is all the waiver needs.  Note G stays as it is: no :mask is
drawn, and an endcap on a line wider than the Cont is neither tool's reading.

## Findings

### 1. M1.c does not see a Cont lying across a hole in the Metal1 (false negative)

Manual: "M1.c  Min. Metal1 enclosure of Cont  0.00".

Layout `M1.c.h2`: a Cont (10.42, 2.42)-(10.58, 2.58) across the 0.1-wide hole
(10.45..10.55 × 2..3) of a Metal1 ring 9.5..11.5 × 1.5..3.5; beside it a Cont across a
0.005 gap between two boxes and a Cont whose corner a chamfer cuts.

- gdscheck @20/7/100: M1.c 2 - "(8.5, 2.5)" (the gap) and "(2.5, 5.0)" (the chamfer);
  nothing for the Cont over the hole.  M1.b reports the hole as a 0.1 notch.
- KLayout: M1.c 3, the third "polygon: (10.45,2.42;10.45,2.58;10.55,2.58;10.55,2.42)"
  (`Cont.not(Metal1)`).
- Verdict: a 0.1 × 0.16 strip of the Cont has no Metal1 over it.  Expected 3 (`m1_c_h2`).

### 2. M1.c1 reads "any side 0.05" and misses every endcap (false negative, the common case)

Manual: "M1.c1  Min. Metal1 endcap enclosure of Cont (Note 1)  0.05"; note 1: "For
contacts at Metal1 corners at least one side must be treated as an endcap and for the other
sides rule M1.c can be applied."  Figure 5.16 draws c1 at the end of a line whose Cont is
flush (c = 0.00) on the line's side.

The deck runs `min_enclosure 0.05` with `sides: any`: a Cont passes when *some* side has
0.05.  A Cont at the end of a 0.16 line has the whole line on its far side, so every endcap
passes whatever its length.  The reading the note and the figure give, and IHP's deck
encodes (`one_side_allowed, two_opposite_sides_allowed`): a side under 0.05 is a line
running past the Cont (M1.c's 0.00), and a line has one such side (at a corner, the other
corner side being the endcap) or two opposite ones; two *adjacent* short sides, or three, or
four, leave a corner of the Cont without an endcap.

Layouts `M1.c1.h1` (line ends), `h2` (plate corners and pads), `h3` (a seam), `h4` (tile
lines), `h5`/`h6` (fifty, flat and as an array):

- gdscheck @20/7/100: one marker in all six layouts - the 0.02-all-round pad of `h2`
  ("enclosure 0.0200 at (2.42, 5.08)-(2.42, 4.92)").  The 0.045 endcap and the 0.00 endcap
  on a 0.16 line, the Cont at the end of a 0.25 line (0.045 on three sides), the 0.05/0.045
  stub, the Cont flush on two sides of a plate corner (2.3, 2.3), the 0.045/0.045 corner,
  the pad with 0.05 on the left and 0.02 elsewhere, the seam with 0.045 on the top and
  the right, the 0.045 endcaps on and across the tile lines and the fifty are all silent.
- KLayout: M1.c1 on exactly those: 4 in `h1` (Conts at (4.5, 2.5), (6.5, 2.5), (6.5, 5),
  (8.5, 5)), 4 in `h2` ((2.38, 2.38), (2.5, 5), (4.5, 5), (6.425, 2.425)), 1 in `h3`
  ((8.5, 2.5)), 7 in `h4`, 50 in `h5`, 51 in `h6`; nothing on the 0.05 endcap, the Cont
  mid-line, the 0.05-and-flush corner, the 0.045/0.05 corner, the 0.05/0.05/0.02/0.02
  pads, the L's inner corner and the seam with 0.05 all round.
- Verdict: expected 4, 4, 1, 7, 50, 50 (`m1_c1_h1`..`h6`).  The existing `M2.c1` fixture
  (0.05 on one side, 0.02 on the other three, "clean") is the same question for the Metal2
  deck; by this reading it fires.  One sub-pattern is left clean on purpose and is
  debatable: a Cont at the end of a 0.30 (or 0.26) line, sides 0.07 (0.05), endcap 0.045 -
  one short side, clean in both tools and in the case, though the endcap is 0.045 (note G).

### 3. The Metal1 rules run inside the EdgeSeal (false positive)

Manual, section 6.10: "Please be aware that corresponding standard metal and via rules are
not checked within EdgeSeal regions."  The deck's M1.c/M1.c1 take `ContNoSealring`;
M1.a, M1.b, M1.d, M1.e, M1.f, M1.g and M1.i take `Metal1`.

Layout `M1.seal.h1`: inside an EdgeSeal 2..12 a 0.155 bar, two boxes 0.175 apart, a
0.30 × 0.295 box, a Cont in a 0.20 pad and one sticking 0.005 out, a 0.305/0.16 pair at
0.20, two 45° strips 0.2157 apart, a 0.198 45° strip; outside it a 0.155 bar at x = 15,
and a 0.155 bar 11..11.155 × 10..14 across the seal's edge.

- gdscheck @20/7/100: M1.a 6, M1.b 1, M1.d 2, M1.e 2, M1.g 2, M1.i 1 (M1.c and M1.c1
  quiet, the Cont layer excludes the seal).
- KLayout: M1.a 3, M1.b 1, M1.c 1, M1.e 1, M1.g 1, M1.i 2 - its M1.c1 and M1.d take
  `Metal1.outside(EdgeSeal)`, the rest run on the drawn layer; so it waives two rules of
  nine.
- Verdict: nothing inside the seal is checked; the bar outside and the part of the
  crossing bar outside are 0.155 lines.  Expected M1.a 4 (`m1_seal_h1`).  Whether a shape
  the seal's edge cuts is "within EdgeSeal" is the same question the nwell report's
  finding 12 settled the strict way for DigiBnd; the case follows that.

### 4. M1.j counts the metal under a Metal1:slit (false negative, a reading)

Manual: "M1.j  Min. global Metal1 density [%]  35.0"; section 7.3: slits are cut into
wide metal against mechanical stress, "Metal1:slit  Metal1 slit definition layer".  IHP's
density deck takes `Metal1 OR Metal1:filler NOT Metal1:slit`.

Layout `M1.j.h2`: ten 36 µm Metal1 stripes (36 % of a 1000 µm die) each holding eight
20 × 30 Metal1:slit boxes, 4.8 % of the die.

- gdscheck (`--deck metal1`): "Density: 36.00 %", clean.
- KLayout: not run (the oracle leaves density out); its deck reads 31.2 %.
- Verdict: the slit is where the metal is cut away at mask generation, so 31.2 % of the die
  is metal and M1.j fires (`m1_j_h2`).  A reading: the manual's density rule names only
  Metal1, and the deck's own choice to add Metal1:mask (and the filler) shows it means the
  physical metal.  The coincident-layer case `M1.j.h1` (30 % drawn on Metal1, filler and
  mask alike) is right: M1.j fires, nothing else.

### 5. M1Fil.c does not see a filler abutting Metal1 (false negative)

Manual: "MFil.c  Min. Metal(n):filler space to Metal(n)  0.42"; the glossary's "abut" is
"two edges of two different layers touching each other".

Layout `M1Fil.c.h3`: a filler 2..4 × 2..4 with Metal1 4..5 × 2.5..3.5 sharing its right
edge; a filler touching Metal1 at the corner point (10, 4); a filler crossing the Metal1
edge and one inside Metal1.

- gdscheck @20/7/100: M1Fil.c 1, "space 0.0000 at (10, 4)" - the corner point only.
- KLayout: the abutment "(4,2.5;4,3.5)/(4,3.92;4,2.08)" and the corner (two edge pairs);
  nothing for the crossing and the enclosed filler.
- Verdict: an abutment is a space of 0.00: expected 2 (`m1fil_c_h3`).  The class of the cont
  report's finding 4 (`abutting: report`), which M1Fil.c did not get; M1Fil.d needs it
  too (finding 6, `M1Fil.d.h1` has an abutting TRANS).  The crossing and enclosed fillers
  are no pair, as settled for AFil.c/AFil.e.

### 6. M1Fil.d runs on Metal5:filler (rule effectively missing; false positive on Metal5)

Manual: "MFil.d  Min. Metal(n):filler space to TRANS  1.00".  `show-deck`:
`M1Fil.d  min_space  [Metal5.filler, TRANS]  value=1` - the metal1 deck checks the
Metal5 filler (metal2-5's decks have their own layer right).

Layouts `M1Fil.d.h1` (0.995, a 0.99 diagonal, corner-on 0.995, a TRANS diamond tip at
0.995, an abutting TRANS; 1.0, 1.004, a crossing and an enclosed filler as controls),
`h2` (tile lines, (1000, 1000), 300 µm), `h3`/`h4` (fifty), `h5` (a Metal5:filler 0.995
from a TRANS, no Metal1:filler anywhere).

- gdscheck @20/7/100: nothing in `h1`-`h4`; M1Fil.d 1 in `h5` (beside the metal5 deck's
  M5Fil.d).
- KLayout: M1Fil.d 5 in `h1` ("(4,4;4,2)/(4.995,2;4.995,4)", the diagonal and the tip as
  two pairs each, "(4,11;4,10.9)/(4.995,11;4.995,11.1)", "(18,11;18,9)/(18,9.5;18,11.5)"),
  11 in `h2`, 50 in `h3`, 1 in `h4`; nothing in `h5` but M5Fil.d.
- Verdict: expected 5, 11, 50, 50 and none (`m1fil_d_h1`..`h5`).  A one-word deck fix;
  the abutting TRANS of `h1` then needs `abutting: report` as well (finding 5).

## Notes that are not findings

- A. KLayout's M1.e and M1.f miss every pair of two *wide* lines: two 0.5 lines at 0.215
  (`M1.e.h1`, x = 11), two 0.5 lines on one net at 0.20 (`M1.e.h9`), the stepped plates
  sharing 1.005 (`M1.e.h4`) and 10.005 (`M1.f.h2`), the same-net 12 × 12 plates
  (`M1.f.h6`), the 1-wide 10 µm and 300 µm bars at 0.175 (`M1.b.h5`, `M1.b.h8`), the 0.566
  45° strips (`M1.b.h2`, `M1.i.h1`-`h7`).  Its `metal1_drw.sep(wide_part, ...)` reports
  a wide line only against a narrow one.  The manual says "at least one line is wider
  than 0.3"; gdscheck reports them all and the cases expect them.
- B. Notches under the space rules that say "space" and not "space or notch" (M1.b and
  Mn.b say both; M1.e, M1.f, M1.i and MFil.b say "space"): a U of 0.5 arms with a 0.20 slot
  (`M1.e.h9`), a U of 10.005 arms with a 0.5 slot (`M1.f.h6`), a 45° U with a 0.2192 slot
  (`M1.i.h2`), the 45° notch of `M1.b.h3`, a filler U with a 0.415 slot (`M1Fil.b.h3`).
  gdscheck is quiet on all five; KLayout is quiet on the M1.e/M1.f ones and reports the
  M1.i ones (`edges.sep`) and the M1Fil.b one (`space`).  The cases follow the wording,
  as the activ report did for AFil.b; if the PDK owner reads a notch into these rules,
  `min_notch` entries and +1/+1/+1/+1 on the four cases do it.
- C. Both tools report M1.c1 as well as M1.c on a Cont that sticks out of the Metal1 or
  has none (`M1.c.h1`-`h5`); the M1.c cases ignore M1.c1.
- D. M1Fil.a2 on a diamond with a 5.2 bounding box and 3.68 between its walls
  (`M1Fil.a2.h1`): KLayout fires (`with_bbox_max`), gdscheck is quiet, and the case expects
  nothing - the manual's width is between walls.  Fillers are boxes, so nothing hangs on it.
- E. The parallel run of M1.e/M1.f on 45° lines is the length the facing walls share
  along their direction, in both tools: two 0.566 strips offset along their walls so
  the shared run is 0.987 are clean and 1.34 fires (a probe, not a fixture); the fixtures'
  aligned pairs with 1.06 walls (`M1.e.h4`) and 10.6 walls spanning 7.5 in x (`M1.f.h2`)
  fire in both tools.  Not the axis reading the gatpoly report found in Gat.g.
- F. The run is the *wide part's*: a 0.16 line with a 0.5 pad 0.8 long beside a straight
  line at 0.20 for 4 µm is clean in both tools, a 1.5 pad fires (`M1.e.h3`); a 0.5 line
  with an 8 µm 10.005 pad beside a 30 µm neighbour is clean, a 12 µm pad fires
  (`M1.f.h2`).  The cases expect that.
- G. M1.c1 at the end of a line wider than the Cont (`M1.c1.h1`, x = 2.5 and 4.5, y = 5):
  0.07 or 0.05 on the sides, 0.045 at the end.  One short side: clean in both tools, and
  in the case under the reading of finding 2, though the endcap is 0.045.  The PDK
  owner may prefer "an end is an end" here; then those two fire and the reading of
  finding 2 needs a notion of which side is the line.
- H. Marker cuts: gdscheck's M1.c1 is one per Cont ("shape not enclosed" for a Cont
  sticking out, "enclosure 0.02 at <edge>" otherwise), M1.c one per Cont, M1.d one per
  region; KLayout gives a polygon per Cont for M1.c/M1.c1 and per region for M1.d, two
  or three edge pairs for a corner-to-corner space, and its hierarchical run counts an
  array cell once.  Two boxes meeting at a corner are two M1.d regions in both tools
  (`M1.d.h2`), with the pinch reported as M1.a and M1.b as the `M1.corner` case has it.
- I. Two 45° strips 0.1768 apart are M1.b, M1.e (0.566 wide, run 2.05) and M1.i at once
  (`M1.i.h7`), and every 0.2157 strip pair in the M1.i layouts is M1.e too; the cases
  expect both.  gdscheck cuts a 45° pair as one marker, KLayout as two edge pairs.

## Tested and found clean or correct (no need to redo)

- M1.a: 0.16 vs 0.155 in x and y; 300 µm; a 0.005 sliver; (1000, 1000); diamond and 45°
  strip 0.1626/0.1556; chamfered box and L; unions of overlapping/abutting/gridded boxes,
  a ring wall, an island; bars on/straddling x = 20/21/40/42 and an L cornered on 20;
  fifty flat and as GdsArrayRef; comb and U.
- M1.b: 0.175/0.18; diagonal 0.1768/0.1838; corner-on; diamond tip, 45° strips, chamfer
  to corner, tip to tip; straight and 45° notch, comb, slot, keyhole hole, facing Ls,
  island (nine, matching KLayout); unions; tile lines incl. a corner pair across (20, 20);
  fifty flat/array; sliver, 300 µm, (1000, 1000); filler and mask neighbours are not
  M1.b; same net via Via1/Metal2 and Conts change nothing.
- M1.c: 0.005 out on each side, half out, bare (M1.c and Cnt.h in both tools); flush on
  one side, in a 0.16 line, at a plate corner; a seam and an overlap under the Cont; a
  0.005 gap; a 0.1 chamfer cutting the corner vs one through the corner; a covering grid;
  tile lines; fifty flat/array; (1000, 1000).
- M1.c1 (the sub-patterns both tools agree on): a 0.05 endcap, a Cont mid-line, 0.05 and
  flush at a corner, 0.045/0.05, 0.05/0.05/0.02/0.02 and 0.02/0.02/0.05/0.05 pads, a
  Cont in an L's inner corner (0.05 and 0.045), a seam with 0.05 all round, a chamfer
  0.078 from the corner with 0.12 walls (projection, settled), a 0.05 endcap straddling
  x = 40.
- M1.d: 0.09/0.0885; a 0.0896 vs 0.0904 line; an L, two diamonds, a chamfered square;
  a union counted once; abutting boxes summed; a grid; an island; a ring; tile lines incl.
  a 0.0896 bar across 20; fifty flat/array; 0.005 slivers of 0.0025 and 0.09; 300 µm.
- M1.e: 0.305 vs 0.30 wide; 0.215 vs 0.22; run 1.005 vs 1.0 aligned and as a shared part;
  stubs 0.8 and 0.6; a broken line (two runs); pads 1.5/1.0/0.8 (note F); an L's arm;
  45° pairs 2.83/1.06 vs 0.85 walls (note E); corner to corner; end-on; stepped plates
  1.005 vs 1.0; unions of the wide line and of the neighbour; eleven tile-line pairs;
  fifty flat/array; same net; a 300 µm pair; a sliver neighbour.
- M1.f: 10.005 vs 10.0 wide; 0.595 vs 0.60; run 10.005 vs 10.0; pads 12/8; stepped
  plates; 45° pairs 10.6 vs 9.19 walls; tile lines incl. a 32 µm horizontal pair; fifty
  flat/array; same net; an L's arm; 300 µm.
- M1.g: 0.205 vs 0.198 wide; walls 0.509 vs 0.495; a 0.155 strip (M1.a and M1.g); short
  diamonds; Z routes up and mirrored; chamfered Ls (outer 0.566/inner 0.509 fires, inner
  0.4525 clean - the settled per-wall reading); jogs straddling 20, on 20, across 40/42,
  (1000, 1000); fifty flat/array; a 300 µm strip; a 45° sliver.
- M1.i: 0.2157 vs 0.2227 parallel; chamfer to corner 0.2121 vs 0.2263; tip 0.215 vs 0.22;
  a strip's tip to a wall; a corner to a 45° wall (foot on the wall) 0.2157 vs 0.2298; the
  figure's jog beside a plate's 45° wall; same net; tile lines; fifty flat/array; 300 µm;
  a 45° sliver; M1.b/M1.e/M1.i together; a filler neighbour is M1Fil.c's.
- M1.j/M1.k: coincident layers as a union (`M1.j.h1`); the older threshold fixtures.
- M1Fil.a1: 1.0 vs 0.995 in x and y; 300 µm; sliver; (1000, 1000); diamond and strip
  1.004/0.99; chamfered box; unions, a ring wall, an island; tile lines and an L; fifty
  flat/array; comb and U.
- M1Fil.a2: 5 × 5 vs 5.005 in x, y and both; a union; an L; a ring (four markers); the
  diamond (note D); boxes across 20/40, (1000, 1000); fifty flat/array.
- M1Fil.b: 0.415/0.42; diagonal 0.410/0.424; corner-on; tip, strips, chamfer, tip to tip;
  facing Ls and an island (the U per note B); unions; tile lines; fifty flat/array;
  sliver, 300 µm, (1000, 1000); Metal1 and mask neighbours; merged fillers.
- M1Fil.c: 0.415/0.42; diagonal; corner-on; Metal1 tip, filler chamfer, parallel 45°
  strips, a strip's tip; the corner-point touch; crossing and enclosed fillers (no pair,
  settled); tile lines; fifty flat/array; sliver, 300 µm, (1000, 1000).
- Not tested: the SRAM marker (KLayout's M1.c excludes it; the manual's SRAM section is
  work in progress); Metal1:nofill; the M1Fil.h/k windows beyond the older fixtures.
