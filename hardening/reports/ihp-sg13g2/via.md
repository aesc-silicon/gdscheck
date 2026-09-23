<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / via1-via4: hardening report

Decks `via1`, `via2`, `via3` and `via4` against SG13G2 Layout Rules Rev. 0.4, section
5.19 (V1.a-V1.c1) and section 5.20 (Vn.a-Vn.c1, one rule set for Via2-Via4), with
section 6.10's sentence on the sealring.  41 layouts per layer, 164 in all,
`tests/data/ihp-sg13g2/via<n>/V<n>.<rule>.h<k>.gds.gz`, drawn by
`gen/ihp_sg13g2/via.rs` once per index (Metal(n) below the via; the V(n).c/c1
margins take the deck's value `c`, 0.01 on Via1 and 0.005 on Via2-4, so "a step short" is
0.005 on Via1 and 0.000 on the others), each with a `#[case]` in `test_via_hardening` of
`tests/ihp-sg13g2.rs`, a table whose second axis is the layer index 1-4.  Every Via1 and
Via2 layout ran through gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`hardening/oracle-ihp.sh`); the Via3 and Via4 copies ran through gdscheck at the
three tiles and through the test table.  `show-deck` lists every rule of both sections
for each deck (V(n).a, V(n).b, V(n).b1 with rows/cols 3, V(n).c, V(n).c1 with `sides:
adjacent`, `trigger: 0.05`), and the four decks are identical but for the index and the
V(n).c value.

Reading the numbers below: gdscheck reports one marker per wall off the value for
`exact_width` (two per off direction, four for an off square, four for an L of 0.19
arms, two for a 0.19 × 300 bar, eight for a 0.19-wide via ring, two for a via cut to
0.095 by the seal's edge), one per violating pair for V(n).b, one per array for V(n).b1
(a via ring four vias thick is one 32 × 4 array), one per via for V(n).c and V(n).c1.
KLayout's column in the oracle adds the driver's tables and the maximal deck: V1.a, V1.b
and V1.c run in the driver only (once; V1.a by bounding box, V1.c with the euclidian
metric - note A), V1.b1 and V1.c1 in the maximal deck (twice); its V1.b1 marker is the
gap region of an array plus every via touching it (a 4 × 4 gives 17 per run), its
hierarchical run counts an array cell once, and its driver checks V1.b on the unmerged
input and cuts a corner pair into two edge pairs.  No count moved with the tile size in
any layout on any layer; every flat/array pair agreed; and the four layers answered
every layout alike - no difference between Via1, Via2, Via3 and Via4 anywhere.

Test status on the engine as of this report: 3 of the 41 cases fail, on every layer (12
of 164), all on the findings below; the other 38 (152) pass.

## Resolution (2026-09-20)

Fixed, deck: 1 (every rule of the via decks runs on `Via<n>NoSealring`; the topvia
report found the same split, one commit covers both).

Open, case carries the engine's answer: 2 - the chamfer through the via's corner is the
metaln report's finding 4 (a corner touch pairs no wall under the projection reading),
the gdscheck owner's call together with the angled-wall projection of the gatpoly report's
finding 11.

## Decided (2026-09-21, gdscheck owner)

2: the chamfer through the via's corner fires - enclosure is the closest approach (`metric: euclidian`), decided 2026-09-21; the diamond and the 0.0035 chamfer of `V(n).c.h3` and the chamfers of `V(n).c1.h4` with it.

## Findings

### 1. V(n).b and V(n).b1 are checked within EdgeSeal (false positive)

Manual, section 6.10: "Please be aware that corresponding standard metal and via rules
are not checked within EdgeSeal regions."  The decks read V(n).a, V(n).c and V(n).c1 on
`Via<n>NoSealring` but V(n).b and V(n).b1 on `Via<n>` as drawn.

Layout `V1.b.h6`: a 0.215 pair (2, 2)/(2.405, 2) under an EdgeSeal (1, 1)-(4, 4); the same
pair at x = 8 outside it; an EdgeSeal (11, 1)-(12, 4) with a via (11.7, 2) under it and a
via (12.105, 2) outside, 0.215 apart.  Layout `V1.b1.h9`: a 4 × 4 at 0.22 from (2, 2)
under an EdgeSeal (1, 1)-(5, 5), the same at x = 8 outside.

- gdscheck @20/7/100: V1.b 3 ("space 0.2150 µm ... at (2.19, 2)-(2.405, 2)", "(8.19,
  2)-(8.405, 2)", "(11.89, 2)-(12.105, 2)"); V1.b1 2 ("4×4 Via1 array (1.42×1.42 µm) ...
  at (2.71, 2.71)" and "at (8.71, 2.71)").
- KLayout: V1.b 1, the pair at x = 8 (its driver runs `via1_drw.not(edgeseal_drw)`);
  V1.b1 on the array at x = 8 only (`Via1.ext_not(EdgeSeal)`).
- Verdict: the pairs and the array under the seal are exempt, and so is the pair with
  one via under it - the seal's via is not checked, so it is no partner.  Expected V(n).b
  × 1 (`vn_b_h6`) and V(n).b1 × 1 (`vn_b1_h9`).  The same on Via2-4.

### 2. V(n).c misses a chamfer through the via's corner (false negative; the metaln report's open finding 4 on the via decks)

Manual: "V1.c  Min. Metal1 enclosure of Via1  0.01", "Vn.c  Min. Metal(n) enclosure of
Via(n)  0.005".

Layout `V1.c.h3`: three 0.6 pads with a via `c` from the top and right edges, the pad's
corner chamfered along x + y = k.  At x = 5 the chamfer passes through the via's corner
(5.59, 2.59) on Via1 (k = 8.18; on Via2-4 the corner is (5.595, 2.595), k = 8.19): the
via touches the metal boundary, enclosure 0.000.  Beside it the chamfer at x = 8 cuts the
corner off (fires everywhere) and the one at x = 2 passes 0.0035 from the corner with
both walls `c` away (the settled projection reading: clean).

- gdscheck @20/7/100: V1.c on the cut corner only ("not enclosed ... at (8.495, 2.495)").
- KLayout: V1.c on the cut, on the touch ("edge-pair: (5.59,2.59;5.59,2.576)/(5.59,2.59;
  5.6,2.58)") and, euclidian, on the x = 2 pad too (note A).
- Verdict: a via corner on the metal boundary is enclosed by nothing there.  Expected 2
  (`vn_c_h3`).  The same touch on an axis-aligned edge (`V1.c.h1`, the via in the pad's
  corner at (17, 2)) is reported fine.

## Notes that are not findings

- A. KLayout's V1.c is euclidian.  The driver's `5_19_via1.drc` runs
  `via1_nseal.enclosed(metal1_drw, 0.01, euclidian)`, unlike its Mn.c, and the maximal
  deck's V1.c1 term is `if_any(enclosed(Metal1) < 0.01, enclosed(..., projection, ...,
  one_side_allowed, two_opposite_sides_allowed) < 0.05)`, so every V1.c violation is a
  V1.c1 there as well.  Hence its V1.c on the chamfer 0.0035 from the corner (`V1.c.h3`
  at x = 2, "edge-pair: (2.581,2.59;2.59,2.59)/(2.585,2.6;2.595,2.59)"), on the diamond
  of metal whose walls pass 0.0071 (Via1) / 0.0035 (Via2-4) from a centred via's corners
  (`V1.c.h3` at x = 15, four edge pairs), and on the chamfered line ends of `V1.c1.h4`
  (0.007 from the via's corners).  The settled reading measures enclosure by projection
  between walls; gdscheck reports none of these under V(n).c, and the cases expect none.
  The diamond at x = 12 (0.0106 / 0.0071) is clean in both tools.
- B. Diamonds under V(n).c1.  A via centred in a diamond of metal has no direction in
  which the metal runs 0.05 past it; gdscheck reports V(n).c1 on both diamonds of
  `V1.c.h3` ("side enclosed 0.0150 µm < 0.05 µm and a bordering side only 0.0150 µm"),
  KLayout too.  Right by the manual; the case sets V(n).c1 aside, as it does for the
  pads with `c` on two adjacent sides.
- C. KLayout's V1.c does not report an uncovered via: the bare via, the via half out of
  a line's end, the via beside a line, the via in a metal ring's hole and the via on the
  hole's edge of `V1.c.h2`, and the via sticking 0.005 out of its line in `V1.c.h1`, are
  V1.c1 only there (the metaln report's note C).  gdscheck reports V(n).c (and V(n).c1)
  on them; the cases expect V(n).c and set V(n).c1 aside.
- D. Two vias touching at a corner (`V1.a.h2` at (17, 2)/(17.19, 2.19)): gdscheck V(n).b
  ("space 0.0000 µm") and no V(n).a; KLayout V1.b (two edge pairs, on the unmerged
  input) and V1.a as well (the merged shape's bounding box is 0.38).  The case expects
  V(n).b: two 0.19 vias with no space between them are a space violation, and each is
  0.19 wide between its walls.  Two vias overlapping by 0.005 at a corner are one
  non-rectangle and V(n).a to both tools (four walls in gdscheck, "width 0.3750").
- E. A staggered 4 × 4 (`V1.b1.h2` at (16, 2): four rows of four at 0.22, the odd rows
  shifted by half a pitch, 0.2205 corner to corner between rows) is one array to
  gdscheck ("4×4 Via1 array (1.62×1.42 µm)") and nothing to KLayout: its detector
  shrinks the merged blob by 0.685 and the stagger's 0.205 indentations leave only 1.215
  between opposite notches, under its 1.37 core.  The cont report saw a staggered 5 × 5
  in both tools (its fifth column made the core), and its resolution reads a staggered
  array as one, corner to corner counting; the case follows that (fires).
- F. A 4 × 4 at 0.22 missing a corner via (`V1.b1.h2` at (10, 6)) is silent in both
  tools; the case expects nothing, as the cont report's finding 3 was resolved (a row of
  three is no row of the array).
- G. A 4 × 4 whose row gaps are 0.22/0.29/0.22 (`V1.b1.h2` at (12, 2)) is one array in
  gdscheck ("1.42×1.49 µm") and two polygons in KLayout (rows 1-2 and rows 3-4), the
  cont report's finding 2 as fixed; a 4 × 4 with a fifth via on its bottom row is one
  array ("1.83×1.42 µm"); two 2-column blocks 0.22 apart are one.
- H. The via ring four vias thick of `V1.b1.h8` (20 per side at 0.22) is one array in
  gdscheck ("32×4 Via1 array (7.98×7.98 µm)") and one polygon in KLayout; the rings one
  and two thick are clean in both.  The 20 × 20 at 0.22 across x = 20/21 and y = 40/42
  and the 4 × 50 across y = 7/14/20/21 of `V1.b1.h4` are one each at every tile.
- I. The 0.19 × 300 via bar of `V1.a.h6` and `V1.b.h5` is V(n).a in both tools (two
  walls in gdscheck, "width 300.0000"); the 0.005 × 0.19 sliver too ("width 0.0050").
- J. The seal fixtures (`V1.a.h7`, `V1.b.h6`, `V1.b1.h9`, `V1.c.h9`, `V1.c1.h9`) trip the
  sealring deck in the oracle's `core` suite (Seal.a, Seal.c1, Seal.l; KLayout Seal.m,
  Seal.n) since they draw no Activ or Passiv ring; not this deck's.  The via across the
  seal's edge (`V1.a.h7` at (21.905, 2), `V1.c.h9` at (11.905, 2)) is cut at the edge in
  both tools and its 0.095 × 0.19 outside part reported (V1.a; V1.c on the line's edge),
  as the metal1 report has it for Metal1NoSealring.
- K. KLayout's Vn.c runs on the drawn via.  Its `5_20_vian.drc` derives `vian_nseal`
  for Vn.a and Vn.b but runs Vn.c on `via_lay` itself, unlike V1.c; so on `V2.c.h9` it
  reports the exempt via under the seal too ("edge-pair: (2.59,2;2.4,2)/(2.595,2;2.395,
  2)", V2.c 3 where V1.c is 2).  The only place the Via2 oracle column differs from
  Via1's; gdscheck answers both alike (2).
- L. KLayout's driver excludes `npn13g2l` vias from V1.a and the maximal deck excludes
  SRAM from V1.c1; neither marker was drawn.

## Tested and found clean or correct (no need to redo)

- V(n).a: 0.19 vs 0.19 × 0.195, 0.195 × 0.19, 0.185 and 0.195 squares, 0.19 × 0.185; a
  bar and an L; unions (0.29 bar, halves, quadrants, a sliver, clockwise, corner touch,
  corner overlap); 0.185 squares on, across and beside x = 20/21/40/42 and y = 20 with
  0.19 controls; fifty flat/array; the sliver, the 300 µm bar, (1000, 1000); exempt
  under EdgeSeal (a square and a Seal.c1-style 0.19 ring), checked outside and across
  the seal's edge.
- V(n).b: 0.22 vs 0.215 in x and y; corner to corner 0.219 vs 0.226, (0.1, 0.195) vs
  (0.1, 0.2), the 0.2/0.2 diagonal (0.283, clean); corner-on; a row of three, an L of
  three, a 2 × 2 block; gaps across x = 20/21/40/42/14 and y = 20, beginning and ending on
  x = 20; fifty flat/array; a via 0.215 vs 0.22 from a 300 µm bar; (1000, 1000).
- V(n).b1: 4 × 4 at 0.22, 0.25 and 0.285 vs 0.29 and 0.30; one direction relaxed either
  way (0.29/0.22, 0.22/0.29, 0.25/0.30, 0.30/0.22); 4 × 3, 3 × 4, 3 × 3; 5 × 5, 4 × 10,
  10 × 10, 20 × 20, 4 × 50; mixed row gaps; staggered; two blocks; a fifth via; a missing
  corner; with V(n).b at 0.215 (24 and 12 pairs); arrays across every tile line, a
  column gap and a via starting on x = 20, cornered on (20, 20), (1000, 1000); fifty
  flat/array; a `GdsArrayRef` of one via; via rings one, two and four thick.
- V(n).c: `c` vs a step short vs 0.005 out; either side of a vertical line; a pad's
  corner at 0.000 and a step short on two sides; bare, half out, beside, in a ring's
  hole, on the hole's edge (note C); the parallel chamfer and the diamonds (settled,
  note A); unions of boxes and slices; vias on tile lines incl. an edge on x = 20 and a
  10 µm line; fifty flat/array; 300 µm; (1000, 1000); exempt under EdgeSeal, checked
  outside and across its edge.
- V(n).c1: 0.05 vs 0.045 vs 0.000 endcaps right, top, left, the outer of two; mid-line;
  the corner variants of note 2 ((0.05, c), (0.05, 0.05), (c, 0.05) clean; (0.045, c),
  (c, c) fire); pads with one short side (clean), two opposite short (clean), two
  adjacent short (fire), three and four short (fire), three and four good (clean); a
  wide line's end with the sides as the opposite pair; a T; a cross; chamfered line
  ends on a narrow line (fire) and a wide one (clean); a via centred in a 45° strip;
  endcaps on and across x = 20/21/40/42; fifty flat/array; 300 µm; (1000, 1000); exempt
  under EdgeSeal; a 4 × 4 in a pad with 0.05 (clean) and 0.045 (the four corners) all
  round; a row of four with 0.045 ends (both ends).
- Not tested: vias in a 45° metal line closer than 0.05 to its walls (no parallel wall
  to project onto; the manual's figure is axis-aligned); the npn13g2l and SRAM markers.
