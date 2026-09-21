<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / metal2-metal5: hardening report

Decks `metal2`, `metal3`, `metal4` and `metal5` against SG13G2 Layout Rules Rev. 0.4,
section 5.17 (Mn.a-Mn.k, one rule set for Metal2-Metal5) and section 5.18 (MnFil.*), with
section 6.10's sentence on the sealring.  89 layouts per layer, 356 in all,
`tests/data/ihp-sg13g2/metal<n>/M<n>.<rule>.h<k>.gds.gz`, drawn by
`gen/ihp_sg13g2/metaln_hardening.rs` once per index (Via(n-1) and Metal(n-1) below the
metal), each with a `#[case]` in `test_metaln_hardening` of `tests/ihp-sg13g2.rs`, a table
whose second axis is the layer index 2-5.  Every Metal2 layout ran through gdscheck at
tiles 20, 7 and 100 and through IHP's KLayout decks (`hardening/oracle-ihp.sh`); the
Metal3-5 copies ran through gdscheck via the test table.  `show-deck` lists every rule of
both sections for each deck, and the four decks are identical but for the index.

Reading the numbers below: gdscheck reports one marker per wall for `min_width` (two per
narrow bar, four per narrow diamond, two per 45° strip under Mn.g), one per violating pair
for a space, one per via for the enclosures, one per region for the area.  KLayout's column
in the oracle adds the driver's tables, the driver's maximal run and the standalone
maximal run: Mn.a, Mn.b, Mn.e, Mn.f, Mn.g, Mn.i, MnFil.a2 and MnFil.c run in the driver's
tables (once), MnFil.a1, MnFil.b and MnFil.d in the driver's maximal run (once), Mn.c,
Mn.c1 and Mn.d in both maximal runs (twice); its hierarchical run counts an array cell
once, it cuts a corner pair into two or three edge pairs, and its driver checks Mn.b on
the unmerged input (seven edge pairs for the gridded box of `M2.b.h4`).  No count moved
with the tile size in any layout; every flat/array pair agreed; and the four layers
answered every layout alike - no difference between Metal2, Metal3, Metal4 and Metal5
anywhere.

Test status on the engine as of this report: 16 of the 89 cases fail, on every layer (64
of 356), all on the findings below; the other 73 (292) pass.

## Resolution (2026-09-20)

Fixed, engine: 2 (a wall's depth is read stretch by stretch along it - the nearest
anti-parallel wall of the same region behind each stretch - and Mn.e asks for a stretch
deeper than the width running longer than the length within the part the two walls
share; a 0.2 line with a 0.5 part on its far side is wide along that part, as KLayout's
`sized(-w/2).sized(w/2)` survivor).  Fixed, deck: 1 (Mn.c1 reads `sides: adjacent` with
`trigger: 0.05`, KLayout's `one_side_allowed, two_opposite_sides_allowed`: a via passes
with one short side or two opposite ones, and the older `M{n}.c1` fixtures' clean via has
its two 0.05 sides opposite now - Via1-4's V{n}.c1 likewise), 3 (Mn.a, Mn.b, Mn.d, Mn.e,
Mn.f, Mn.g and Mn.i run on `Metal{n}NoSealring`, the metal less the EdgeSeal - cut at
its edge like the vias; see the metal1 report on why not whole regions),
5 (`abutting: report` on MnFil.c and MnFil.d; a filler lying inside a TRANS is read by
an enclosure entry on `Metal{n}FillerInTRANS`, as the activ report settled AFil.d/e/i),
and the M1Fil.d layer bug the metal1 report found on the way.

Settled on KLayout's reading, cases flipped: the filler well inside the TRANS of finding
5 (enclosed by more than the value: clean, as AFil.e's), and 6 (the metal-filler maximum
reads the bounding box as upstream's `with_bbox_max`; the case carries all five shapes).

Open: 4 (the chamfer through the via's corner - the parallel-walls reading pairs no wall
with a corner touch; the gatpoly report's finding 11 is the same question), and 6 stays
the PDK owner's call.

## Decided (2026-09-21, PDK owner)

4: the chamfer through the via's corner, and the one passing 0.0035 from it, fire - enclosure is the closest approach (`metric: euclidian`), decided 2026-09-21.

## Findings

### 1. Mn.c1 accepts any one side over 0.05; the manual's endcap is a pair of opposite sides (false negative)

Manual: "Mn.c1  Min. Metal(n) endcap enclosure of Via(n-1) (Note 1)  0.05", note 1: "For
vias at Metal(n) corners at least one side must be treated as an endcap and for the other
sides rule Mn.c can be applied."  Figure 5.17 draws c1 along the line at a line's end.  A
via in a line has an endcap in the line's direction: the metal runs past it at both ends.
At a corner the two inner sides run on, the note lets one of the two outer sides be the
endcap - the other end of that direction - and the other one Mn.c.  So the rule is: some
direction, x or y, in which the via is enclosed by 0.05 at both ends.  IHP's KLayout deck
says the same (`enclosed(..., one_side_allowed, two_opposite_sides_allowed)`: a via passes
with at most one short side or two opposite short sides, i.e. with one opposite pair over
0.05).  The deck runs `min_enclosure` with `sides: any`: one side over 0.05 anywhere, and
a via at a line's end always has one - the side where the line continues.

Layouts (0.19 vias, 0.2-wide lines with 0.005 either side): `M2.c1.h1`, vias at a line's
end with a 0.045 endcap on the right (7.765, 2.005), the top (18.005, 3.765), the left
(2.045, 5.005), the outer of two (7.765, 5.005), and one with 0.000 (11.81, 2.005; Mn.c
too); `M2.c1.h2`, vias in an L's corner with outer margins (right, bottom) of (0.045,
0.005) and (0.005, 0.005); `M2.c1.h3`, pads with margins (left, right, bottom, top) of
(0.05, 0.005, 0.005, 0.005) at (2, 2) and (0.05, 0.005, 0.05, 0.005) at (6, 2);
`M2.c1.h5`, 0.045 endcaps on and across x = 20, 21, 40 and 42; `M2.c1.h6`/`h7`, fifty
flat and as an array; `M2.c1.h8`, at (1000, 1000) and at the far end of a 300 µm line;
`M2.c.h5`, a via 0.005 short of its line's end at (19.805, 10.005).

- gdscheck @20/7/100: nothing for any of them except the 0.000 endcap's Mn.c; in
  `M2.c1.h3` only the 0.045-all-round and 0.005-all-round pads (2 of 4).
- KLayout: Mn.c1 on every one of them (5, 2, 4, 5, 100/51, 2 and 1 in the driver plus
  the maximal deck: 10, 4, 8, 10, ...), e.g. "polygon: (7.765,2.005;7.765,2.195;7.955,
  2.195;7.955,2.005)".
- Verdict: all fire.  Expected Mn.c1 × 5 + Mn.c (`mn_c1_h1`), 2 (`mn_c1_h2`), 4
  (`mn_c1_h3`), 5 (`mn_c1_h5`), 50 (`mn_c1_h6`, `mn_c1_h7`), 2 (`mn_c1_h8`), Mn.c × 5 +
  Mn.c1 (`mn_c_h5`).  The corner and pad variants that the note allows are clean in
  both tools and in the cases: (0.05, 0.005), (0.05, 0.05), (0.005, 0.05) at the corner;
  two opposite 0.05 sides, three, four on a pad; a via mid-line, at a T, in a cross; and
  a 0.045 endcap at the end of a 0.5-wide line whose 0.155 sides are the opposite pair
  (`mn_c1_h4`, all clean).  The existing fixture `M2.c1.gds.gz` calls its first via
  clean - a 0.05 left side with 0.02 on the other three - and KLayout reports it too;
  by this reading that case expects two.

### 2. Mn.e reads a line's width at its facing wall only (false negative)

Manual: "Mn.e  Min. space of Metal(n) lines if, at least one line is wider than 0.39 µm
and the parallel run is more than 1.0 µm  0.24".  A line 0.5 wide for 2 µm of its length
is wider than 0.39 along those 2 µm, whichever side the extra width is on; IHP's deck
takes the wide part as what survives `sized(-0.195).sized(0.195)` and measures from it.

Layout `M2.e.h3`: a 0.2 line (14, 2)-(14.2, 5) with a 0.3 strip on its far side (13.7,
2.5)-(14, 4.5), so it is 0.5 wide for 2 µm and its facing wall x = 14.2 is straight; a
0.2 line 0.235 to the right.  Layout `M2.e.h4`: a pad (6, 2)-(6.2, 5) stepping wider on its
far side to 0.5 and 0.8 (5.7-6 × 2.5-4.5, 5.4-5.7 × 3.5-4.5), a 0.2 line 0.235 to the
right of its straight wall.

- gdscheck @20/7/100: nothing for either; the same layouts' bumps towards the neighbour
  (0.5 wide for 1.005, `M2.e.h3` at x = 5) and the 2 × 2 plate fire as they should.
- KLayout: Mn.e on both, "edge-pair: (14.435,2.451;14.435,4.549)/(14.2,4.5;14.2,2.5)"
  and "(6.435,2.451;6.435,4.549)/(6.2,4.5;6.2,2.5)" - the wide part's 2 µm, measured at
  the straight wall (its Mn.e does see a wide line against a narrow one, note B).
- Verdict: both fire; the wide part runs 2 µm beside the line at 0.235.  Expected 3
  (`mn_e_h3`) and 4 (`mn_e_h4`).  The pattern with the step towards the neighbour
  (`M2.e.h4` at x = 9: a 0.6-long step at 0.235, the rest 0.735 away) is rightly clean
  in gdscheck, and the ones with the wide part 0.8 long (`M2.e.h3` at x = 2) and the
  runs staggered to overlap 1.0 exactly are rightly clean too.

### 3. Metal rules are checked within EdgeSeal (false positive)

Manual, section 6.10: "Please be aware that corresponding standard metal and via rules
are not checked within EdgeSeal regions."  The decks exempt the vias (`Via1NoSealring`
under Mn.c/c1) but run Mn.a, Mn.b and Mn.d on Metal(n) as drawn.

Layouts `M2.a.h9`: a 0.195 bar (2, 2)-(2.195, 3) and a 0.205 pair (2-3 / 3.205-4 × 3.5-4)
under an EdgeSeal (1, 1)-(5, 5), the same at x = 8 outside it; `M2.d.h7`: a 0.14 bar
(2, 2)-(2.2, 2.7) under the seal, one at x = 8 outside.  (`M2.c.h9` has the via
version: exempt under the seal, checked outside and, for a via across the seal's edge,
on its outside part.)

- gdscheck @20/7/100: Mn.a on both bars, Mn.b on both pairs, Mn.d on both bars.
- KLayout: Mn.a and Mn.b on both (its driver runs them on `metal2_drw` as drawn); Mn.d on
  the outside bar only (`Metal2_outside_EdgeSeal`); Mn.c on the outside via only and
  nothing for the via across the seal's edge (`Via1.ext_outside(EdgeSeal)` drops whole
  vias).
- Verdict: the shapes under the seal are exempt.  Expected Mn.a × 2 + Mn.b (`mn_a_h9`),
  Mn.d × 1 (`mn_d_h7`); `mn_c_h9` expects Mn.c × 2, the crossing via's outside part
  included, as gdscheck has it.

### 4. Mn.c misses a chamfer through the via's corner (false negative)

Manual: "Mn.c  Min. Metal(n) enclosure of Via(n-1)  0.005".

Layout `M2.c.h3`: three 0.6 pads with a via 0.005 from the top and right edges, the pad's
corner chamfered along x + y = k.  At x = 5, k = 8.19 passes through the via's corner
(5.595, 2.595): the via touches the metal boundary, enclosure 0.000.  Beside it k = 11.18
at x = 8 cuts the corner off (fires everywhere) and k = 5.195 at x = 2 passes 0.0035 from
the corner with both walls 0.005 away (the settled projection reading: clean).

- gdscheck @20/7/100: Mn.c on the cut corner only ("shape ... not enclosed at (8.5, 2.5)").
- KLayout: Mn.c on the cut and on the touch ("edge-pair: (5.595,2.595;5.595,2.588)/
  (5.595,2.595;5.6,2.59)"), nothing at x = 2.
- Verdict: a via corner on the metal boundary is enclosed by nothing there.  Expected 2
  (`mn_c_h3`).  The same touch on an axis-aligned edge (a via's edge on the line's edge,
  `M2.c.h1`) is reported fine.

### 5. MnFil.c misses an abutting metal, MnFil.d a filler inside the marker (false negative)

Manual: "MFil.c  Min. Metal(n):filler space to Metal(n)  0.42", "MFil.d  Min.
Metal(n):filler space to TRANS  1.00".  The activ and cont decks got `abutting: report`
and inside-enclosure entries for the same classes (their reports' findings 5 and 6, and
Cnt.e); the metal filler rules did not.

Layout `M2Fil.c.h1`: a metal (16, 2.5)-(17, 3.5) abutting the right edge of the filler
(14, 2)-(16, 4).  Layout `M2Fil.d.h1`: a filler (4, 12)-(6, 14) inside a TRANS (2, 10)-(8,
16).  (Beside them a metal overlapping a filler by 0.2 and a filler across the marker's
edge, which share area and are no pair - the settled reading, clean.)

- gdscheck @20/7/100: nothing for the abutment or the inside filler; the 0.415/0.995 pairs
  and the corner pairs fire.
- KLayout: MnFil.c on the abutment ("edge-pair: (16,2.5;16,3.5)/(16,3.92;16,2.08)");
  nothing for the inside filler (its `ext_separation`, like its AFil.e).
- Verdict: a space of zero is under 0.42, and a filler inside a TRANS is what MFil.d keeps
  out.  Expected 4 (`mnfil_c_h1`) and 4 (`mnfil_d_h1`).

### 6. MnFil.a2 reads the bounding box as the width (false positive; kept upstream's reading on purpose)

Manual: "MFil.a2  Max. Metal(n):filler width  5.00"; a width is the distance between
facing walls (figure 4.1), the smaller span of a rectangle.

Layout `M2Fil.a2.h1`: a 5.005 × 5.005 square; a 5.005 × 5.0 filler; a 3 × 20 bar; an L
with 3-wide arms in an 8 × 8 box; a 6 × 6 frame with 2.25 walls.

- gdscheck @20/7/100: MnFil.a2 on all five, 20 markers ("width 20.0000 > 5" on the bar,
  "8.0000" on the L, "6.0000" on the frame, "5.0050 at (16, 2)-(16, 7)" on the 5.005 × 5.0).
- KLayout: the same five (`with_bbox_max`).
- Verdict: only the 5.005 square is over 5 wide.  Expected 4 (`mnfil_a2_h1`, four walls).
  The activ report's resolution says the metal-filler maximum keeps the bounding-box
  reading because upstream has it; the case carries the manual's answer and it is the
  PDK owner's call which one the deck follows.

## Notes that are not findings

- A. "Space", not "space or notch".  Mn.b says "space or notch"; Mn.e, Mn.i and MFil.b say
  "space".  A wide U with its arms 0.235 apart for 2 µm (`M2.e.h7`), a hairpin of two 45°
  strips 0.2333 apart joined at the end (`M2.i.h3`), a filler U with a 0.415 notch
  (`M2Fil.b.h1`) and the 45°-walled notches of `M2.b.h3`: gdscheck reports none of them
  under those rules.  KLayout's edge-based Mn.i reports the four `M2.b.h3` notches
  ("edge-pair: (15.13,0.25;15.246,0.25)/(15.16,0.49;15.13,0.46)", ...) and its `ext_space`
  MFil.b the filler U ("(3.415,10;3.415,12)|(3,12;3,10)"); its Mn.e cannot tell (note B).
  The cases follow the wording (clean), as the activ report's AFil.b does; if the PDK
  owner reads a notch as a space for these three rules, `mn_e_h7`, `mn_i_h3`, `mn_b_h3`
  and `mnfil_b_h1` gain one, one, four and one.
- B. KLayout's Mn.e and Mn.f see a wide line against a narrow one and never two wide
  lines.  IHP's deck derives the wide metal as `met.sized(-w/2).sized(w/2)` and runs
  `met.sep(wide, value, projection_limits(...))`; when both lines are wide each has a copy
  in the wide layer lying on its own edge, and the two-layer separation reports nothing
  for the pair (`M2.e.h1`: the 0.395/0.2 pair only, 1 of 4; `M2.e.h2`: 4 of 5, the
  wide/wide pair missing; `M2.e.h12`, `M2.f.h1`, `M2.f.h3` and the existing
  `M2.e.fail`/`M2.f.fail` fixtures: silent; a scratch script on `M2.e.fail` confirms
  `met.sep(w, 0.24)` finds no pair while `w.space(0.24)` finds two).  Where the oracle's
  Mn.e/Mn.f column is 0 below, that is why; the cases are read from the manual.
- C. KLayout's Mn.c does not report an uncovered via: the bare via, the via half out of
  a line's end and the via beside a line of `M2.c.h2`, and the via sticking 0.005 out of
  its line in `M2.c.h1`, are Mn.c1 only there (its `primary-secondary` term).  gdscheck
  reports Mn.c (and Mn.c1) on them; the cases expect Mn.c and set Mn.c1 aside.
- D. Mn.c1 under the opposite-pair reading also fires on the pads of `M2.c.h3` (0.005 on
  two adjacent sides) and on any via not fully covered; those cases set Mn.c1 aside.
- E. Marker cuts.  gdscheck: two markers per narrow bar and per 45° strip (Mn.a, Mn.g,
  MnFil.a1), four per narrow diamond and per oversized square, one per pair otherwise; a
  corner-to-corner pair is one marker (KLayout: two or three edge pairs); two facing Ls
  are one; a via in a pad's corner on two edges is one Mn.c (KLayout: two).
- F. Mn.e and Mn.i share the value 0.24, so every 45° Mn.e pattern fires Mn.i as well
  (`M2.e.h5`); Mn.b at 0.205 on 45° shapes fires Mn.i too (`M2.b.h2`, `M2.i.h8`); the
  wide 1.0 × 1.0 boxes of the Mn.b layouts fire Mn.e wherever their run is over 1.0
  (`M2.b.h3` Ls, `M2.b.h5` 10 µm bars, `M2.b.h8`, `M2.b.h9`).  All intended and expected.
- G. The Mn.g strips are cut square to their run (`helpers::strip45`), so no acute tip
  and no Mn.a on them, unlike the older `M2.g`/`M2.i` fixtures.  Two layouts were first
  drawn with a 315° vertex - a 45° arm leaving a pad's side, a hairpin closed by a box
  square to the axes - which section 3.1 forbids on metal; KLayout's Mn.b and Mn.i read
  the empty 45° wedge as a space of nothing, gdscheck reads nothing there.  Both were
  redrawn (`M2.g.h2`: the arm leaves the end of a bar; `M2.i.h3`: the join is a strip
  square to the hairpin's arms), and the angle deck is where such a vertex belongs.
- H. KLayout's Mn.e reports the 2 × 1 box of `M2.i.h4` against the 45° wall 0.2333 from
  its corner ("edge-pair: (20.205,11.125;20.125,11.205)/(19.991,11;20,11)"): a corner
  against a slanted wall has no parallel run, and its `projection_limits` lets the
  0.009 through.  gdscheck is right to stay quiet; the case expects Mn.i only.

## Tested and found clean or correct (no need to redo)

- Mn.a: 0.20 vs 0.195 in x and y; 300 µm bars; 0.205 vs 0.198 diamond and 45° strip;
  chamfered box and L; unions of overlapping/abutting/gridded boxes; a ring's side; an
  island in a ring; bars and an L on/straddling x = 20/21/40/42; fifty flat/array; a 0.005
  sliver; (1000, 1000); a comb; a U.
- Mn.b: 0.21 vs 0.205; corner to corner 0.2051 vs 0.2121 and the 0.2/0.2 diagonal
  (0.283, clean); corner-on; diamond tip, strips, chamfer to corner, tip to tip; straight
  and 45° notches, comb, slot, keyhole hole, facing Ls, island in a ring (all eleven);
  unions and a gridded box; gaps on/straddling tile lines incl. a corner pair on 20;
  fifty flat/array; sliver, 300 µm, (1000, 1000); strapped and unstrapped pairs alike.
- Mn.c: 0.005 vs 0.000 vs 0.005 out; a via 0.005 either side of a vertical line; a pad
  corner; bare/half-out/beside (note C); parallel chamfer (settled); unions of boxes and
  slices; vias on tile lines incl. an edge on x = 20 and a 10 µm line; fifty flat/array;
  300 µm; (1000, 1000); exempt under EdgeSeal, checked outside and across its edge.
- Mn.c1: 0.05 endcaps, mid-line, T, cross, wide-line end with wide sides, the allowed
  corner and pad variants (all clean, both tools).
- Mn.d: 0.144 vs 0.142 as boxes, an L, a diamond; a cross union (0.12), abutting boxes
  (0.16), corner-touching boxes (two regions), a grid, a ring, an island, 0.14 vs 0.144
  bars; bars on/across tile lines in x and y; fifty flat/array; 0.01 and 0.144 slivers;
  300 µm; (1000, 1000).
- Mn.e: 0.24 vs 0.235 (x, y); 0.39 vs 0.395; 1.0 vs 1.005 runs; wide/narrow either way,
  narrow/narrow, wide/wide, a wide line between two; bumps of 0.8 and 1.005 towards the
  neighbour; staggered 1.0 vs 1.005; an L pad; a near-side step; a plate beside a line;
  a 0.5 line's end (run 0.5) vs a plate's end (run 2); 45° strips (with Mn.i); a 0.395
  union vs a 0.39 union; a run of two abutting pieces; a neighbour split into two lines;
  runs split by tile lines, gaps straddling them, a 10 µm run, a plate cornered on
  (20, 20); fifty flat/array; 300 µm; (1000, 1000); with Mn.b at 0.205.
- Mn.f: 0.60 vs 0.595; 10.0 vs 10.005 wide; 10.0 vs 10.005 runs; a narrow line beside a
  plate; two 10-wide plates; an L plate; a 10.005 × 10.005 vs 10.005 × 9 pad on a line;
  plates bigger than a tile across every line, runs split 5/5.005; fifty flat/array at
  pitch 30; 300 µm; (1000, 1000); 12-wide 45° strips 0.594 vs 0.601.
- Mn.g: 0.2404 vs 0.2333; walls 0.495 vs 0.502; a 0.2 line's jog vs a 0.24 line's; an arm
  with one long and one short wall (settled: clean) vs both long; an L's chamfered bend at
  three sizes; a small diamond; 300 µm; strips with 0.502 walls split by tile lines (all
  fire), 0.495 (clean); fifty flat/array; (1000, 1000); a 0.198 strip under Mn.a too.
- Mn.i: 0.2333 vs 0.2404 strips; a corner to a 45° wall; a diamond tip; a chamfer to a
  corner (Mn.b quiet); a straight corner pair (no 45° edge, clean); a strip's square end
  above a wall; pairs across tile lines and a corner on 20; fifty flat/array; 300 µm;
  (1000, 1000); with Mn.b at 0.2051.
- Mn.j: stripes on all three density layers are their union (30 %).  MnFil.h: a 700 µm
  hole in a 51 % die reads 23.4 % in the window holding it, one report.
- MnFil.a1: 1.0 vs 0.995 (x, y, union); diamond 1.004 vs 0.99; an L.  MnFil.a2: 5.0 vs
  5.005 squares (finding 6 for the rest).
- MnFil.b: 0.42 vs 0.415; corner to corner 0.424 vs 0.41; diamond tip; a U (note A); tile
  lines; fifty flat/array.  MnFil.c: 0.42 vs 0.415; corner to corner; overlap (settled);
  tile lines; fifty flat/array.  MnFil.d: 1.0 vs 0.995; corner to corner 1.004 vs 0.99; a
  filler across the marker's edge (settled).
- Not tested: Metal(n):nofill / NoMetFiller (note 2 of section 5.18 is about the filler
  generator); the SRAM marker; Mn.k and MnFil.k beyond the existing fixtures.
