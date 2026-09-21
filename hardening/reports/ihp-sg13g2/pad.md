<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / pad, passiv, mim: hardening report

Decks `pad`, `passiv` and `mim` against SG13G2 Layout Rules Rev. 0.4, sections 6.9
(Pad Dimensions, with 6.9.1 Solder Bump and 6.9.2 Copper Pillar), 5.27 (Passiv) and
6.11 (MIM).  `gen/ihp_sg13g2/pad_hardening.rs` draws 59 layouts
(`tests/data/ihp-sg13g2/{pad,passiv,mim}/<RULE>.h<k>.gds.gz`: 30, 12 and 17), each with
a `#[case]` in the deck's table of `tests/ihp-sg13g2.rs`.  Every layout ran through
gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`hardening/oracle-ihp.sh`).  A bump or pillar pad in these layouts carries Passiv,
Passiv:sbump (or :pillar), dfpad and dfpad:sbump (or :pillar), because the manual
recognises it by Passiv:sbump + dfpad and IHP's driver by dfpad:sbump ∩ Passiv:sbump
∩ TopMetal2; the octagons and circles are drawn as IHP's bond pad pcell draws them
(`bondpad_code.py`: the regular octagon of cut `r·(1 − 1/(1+√2))`, the 64-point
circle).

`show-deck` against the manual:

- `pad`: Pad.a1 (`max_width` on Passiv ∩ dfpad, narrowest span), Pad.d (`min_space` to
  Activ ∩ EdgeSeal), Pad.i (dfpad − TopMetal2); Padb.a/b/c/d/f and Padc.a/b/c/d/f.
  Not in the deck, deliberately: the recommended Pad.aR, bR, dR, d1R, eR, fR, gR, jR,
  kR.  Padb.e and Padc.e (the pitch) carry the manual's note "not checked during
  DRC".  Padc.a/b hold one value each (35, 40) where table 6.1 lists three openings:
  finding 4.  IHP's driver also runs a "Pad.m" (bump and pillar pads in one layout)
  that the manual does not have.
- `passiv`: Pas.a, Pas.b as `min_space` and `min_notch`, Pas.c on `PassivInSeal`
  (`inside_ring` of EdgeSeal): every rule of 5.27.
- `mim`: MIM.a-MIM.h and MIM.gR: every rule of 6.11.  MIM.b is a `min_space` alone
  (note E), MIM.d reads the vias that touch MIM (`interacting_only`), MIM.h reads
  TopVia1 or Vmim.

Reading the numbers below: gdscheck gives one marker per off-size wall for
`exact_width` (four for an off-size square, two for a bar, 64 for a circle), one per
pair for a space (a square in a ring's hole 3.495 from all four walls is one pair), one
per under-enclosed shape for an enclosure when its short walls are adjacent (one run) and
one per wall when they are opposite, one per shape or piece for Pad.a1, Pad.i, the
shape rules and the area rules.  KLayout's column is the driver's tables plus the
maximal deck, so Pad.a1, Pad.d, Padc.d, MIM.a/b/e/f/g/h count twice there; its Padb.a
and Padc.a (`width(v − 0.005, projection, projection_limits(nil, 0.05))` joined with
`with_bbox_width(nil, v)`) see only pads whose bounding box is under the value and
short edges under it, so they miss an oversize pad, a 60 × 59.995 and a 60 × 50 pad
outright (note H); its Padb.c has a 0.01 tolerance and is silent at 9.995 (note B).
Every flat/array pair agreed, and no count moved with the tile size except finding 3.

Test status on the engine as of this report: 7 of the 59 new cases fail at tile 20
(`padb_a_h1`, `padb_f_h1`, `padc_a_h1`, `padc_b_h1`, `mim_d_h1`, `mim_e_h1`,
`mim_h_h1`), and `padb_c_h1` passes at 20 and 7 and fails at 100; the other 51 pass.

## Resolution (2026-09-21)

Fixed, deck: 1 and 2 (Padb.a is the pad's extent - `exact_dim` and `exact_length` on
the bounding box - figure 6.9's `a`; the pcell's regular octagon, its 64-gon circle and
a 60 octagon with small chamfers are 60 pads, one marker per off-size side now; the
same for Padc.a), 4 (table 6.1: `CuPillarPadOffSize` = the pads whose box is none of
35, 40 or 45 square, a `forbidden`; Padc.b = 40 between any pads and 50 where a 45 µm
pad is involved, `CuPillarPad45` against itself and against the rest), 5 (MIM.d reads
`TopVia1OnMIM`, the vias lying on a plate, whole - a via over the plate's edge or
corner is enclosed by nothing there; MIM.h is a `forbidden` on `MIMNoVia`, the plates no
via lies over, so the abutting via counts for nothing), 6 (a second MIM.e entry with
`pairs: overlapping` reads the cap's own exit wire against the plate it also covers),
7 (`SBumpPad` and `CuPillarPad` are lazy virtuals: an eager one keeps its result's
outer ring only, and the ring pad had lost its hole before Padb.f's circle test).

Fixed, engine: 3 (a closest approach read at an angle is filed under its corner, not
the wall it was read on, and joins the run of walls at that corner alone: the octagon
in an octagon is four runs at every tile, where the corner reads had chained all
eight walls into one when no tile line cut them; a via in a diamond is four corners
now, `V(n).c.h3`).

## Findings

### 1. The regular octagon and the circle the bump and pillar rules allow are off-size (false positive)

Manual, 6.9.1: "Padb.a  SBumpPad size  60.00" and "Padb.f  Allowed passivation opening
shape  Octagon, Circle"; 6.9.2: "Padc.a  CuPillarPad size  Table 6.1" (35, 40, 45) and
"Padc.f  Allowed passivation opening shape  Circle".  Figure 6.9 draws `a` as the
pad's extent.  IHP's pcell draws the octagon regular (cut 17.575 from a 60 box) and
the circle as a 64-gon of the pad's radius.

Layout `Padb.a.h1` (pads on a 140 pitch, TopMetal2 15 beyond): (e) at (590, 30) the
regular octagon of 60 - its diagonal flats are 59.9985 apart, and no cut on the 0.005
grid puts them at 60.000 (17.57 gives 60.0056); (f) at (730, 30) an octagon of 60 cut
10 (70.71 across the diagonals); (g) at (30, 170) the 64-point circle of radius 30
(walls 59.93 apart).  `Padc.a.h1`: (d) at (420, 30) the 64-point circle of radius 17.5
(walls 34.96 apart).

- gdscheck @20/7/100: Padb.a on (e) four times, "width 59.9980 ≠ 60.0000" on each
  diagonal wall ((560, 17.575)-(577.575, 0) and so on); on (f) four times, "70.7107 ≠
  60"; on (g) 64 times, "59.9272 ≠ 60" and so on, one per wall; Padc.a on (d) 64 times
  ("34.9531 ≠ 35" and so on).  In all 164 Padb.a markers against the 92 the off-size
  pads (b, c, d, h, i, j, l) earn, and 272 Padc.a against 72.
- KLayout: (e) and (f) clean (its projection width has the 0.005 tolerance and its
  bounding-box test reads 60); (g) 16 polygon markers and (d) 8 - it, too, reads the
  64-gon's walls as under 59.995 / 34.995.
- Verdict: clean, all four.  The size of a bump pad is what it spans: an octagon's or a
  circle's "size 60" can only mean 60 across, since no polygon on the grid has every
  wall pair at 60.000 and the rule's own shapes (Padb.f) are the ones that fail.  On (e)
  gdscheck rejects the pcell's own bump pad; on (g) and (d) it rejects the pcell's
  circle, and IHP's deck does the same - if the gdscheck owner wants the 64-gon's walls
  read (59.93), the pcell's circle is a Padb.a violation in both tools and the cases
  `padb_a_h1` and `padc_a_h1` gain 64 markers each.  (f) is the weaker reading: a
  60-bbox octagon with small chamfers is a 60 pad by figure 6.9, and its diagonals are
  the designer's; IHP's deck agrees.  Expected 92 (`padb_a_h1`) and 72 (`padc_a_h1`).
  The existing `padb_f`/`padc_f` cases ignore Padb.a/Padc.a on their octagon and circle
  for this reason.

### 2. Padb.a's exact width is a `≠` on the grid's rounding

Part of finding 1, kept apart because it is the mechanism: an `exact_width` on a shape
with diagonal walls compares a diagonal distance (59.9985) with the value and reports
the 0.0015.  The grid is 0.005; a difference under half a step is no difference.  This
is what turns every regular octagon into four markers; KLayout's `drc_tole` absorbs
it.

### 3. An octagon in an octagon loses three of its four diagonals at tile 100 (tile-dependent)

Manual: "Padb.c  Min. TopMetal2 (within dfpad) enclosure of SBumpPad  10.00".

Layout `Padb.c.h1` (g) at (50, 250): a regular octagon of 60 (cut 17.575, spanning
20-80) in an octagon of 80 cut 23.44 (spanning 10-90), whose diagonal flats are 9.995
from the pad's on all four sides.

- gdscheck @20 and @7: Padb.c four times, (20, 237.575)-(37.575, 220), (37.575,
  280)-(20, 262.425), (62.425, 220)-(80, 237.575), (80, 262.425)-(62.425, 280) - one
  per diagonal.  @100 (and @50, @200): one, the bottom-left diagonal (20,
  237.575)-(37.575, 220) alone.  12 Padb.c in the layout at 20 and 7, 9 at 100.
- KLayout: silent on (g) (its Padb.c reads 9.99, note B); its Padc.c, with no
  tolerance, reports the same geometry's straight case.
- Verdict: four, at every tile size.  At tile 20 the shape is cut by x = 20, 40, 60, 80
  and y = 220-280 and every diagonal is found; when the whole shape sits in one tile,
  three of the four diagonal-wall pairs are not measured.  Expected 12 (`padb_c_h1`);
  the case passes at 20 and 7 and fails at 100.

### 4. Padc.a and Padc.b hold one of table 6.1's three columns (deck)

Manual, 6.9.2: "Padc.a  CuPillarPad size  Table 6.1", "Padc.b  Min. CuPillarPad space
Table 6.1", and table 6.1: "Passiv opening 35 / 40 / 45 - Opening spacing 40 / 40 / 50
- Opening enclosure 7.5 / 7.5 / 7.5"; "The given pad rules are valid for a number of
different geometries offered by our partner PacTech given in table 6.1."  The deck has
`exact_width 35` and `min_space 40`.

Layout `Padc.a.h1`: (e) at (550, 30) a 40 square, (f) at (30, 160) a 45 square, (g)
at (160, 160) a circle of radius 20, (h) at (290, 160) of radius 22.5.  `Padc.b.h1`:
(g) 45 squares 50 apart at (0, 600); (h) 49.995 apart at (200, 600); (i) 45 apart at
(400, 600).

- gdscheck @20/7/100: Padc.a on (e) and (f) four walls each ("40.0000 ≠ 35", "45.0000 ≠
  35"), on (g) and (h) 64 walls each; Padc.b silent on (h) and (i).
- KLayout: Padc.a silent on all four (it only reads undersize, note H); Padc.b silent
  on (h) and (i) - its driver takes 40 from its tech json for every pad.
- Verdict: 40 and 45 are legal openings and (e)-(h) are clean; 45 µm openings need 50
  and (h) and (i) fire.  The deck cannot say this with one `exact_width` value: the
  size wants a set (35, 40, 45), and the space wants to depend on the pad's size (40 for
  35 and 40 µm pads, 50 for 45 µm ones).  Expected 72 (`padc_a_h1`) and 5
  (`padc_b_h1`).  Both tools agree with each other here; the manual's table says
  otherwise.

### 5. A via over the MIM's edge is neither MIM.d nor MIM.h (false negative)

Manual: "MIM.d  Min. MIM enclosure of TopVia1  0.36", "MIM.h  TopVia1 must be over
MIM".

Layout `MIM.d.h1`: (d) at (30, 0) a 0.42 via from x = 32.78 to 33.2 over the plate's
right wall x = 33 by 0.2; (i) at (80, 0) a via from (82.78, 2.78) over the plate's
top-right corner (83, 3) by 0.2 each way.  `MIM.h.h1`: (d) at (30, 0) the same via over
the wall, the plate's only via; (e) at (40, 0) a via from x = 43 to 43.42 abutting the
plate's right wall from outside, the plate's only via.

- gdscheck @20/7/100: nothing on any of the four - no MIM.d (`MIM.d.h1` has the three
  markers of (b), (c) and (g) and not (d) or (i)), and no MIM.h on `MIM.h.h1` (d) and
  (e) (the layout's three MIM.h are (c), (f) and (h)).
- KLayout: MIM.d silent on (d) and (i) as well (its `enclosed` does not read the part
  outside, and unlike its MIM.c it adds no `.not()` term); MIM.h on `MIM.d.h1` (d) and
  on `MIM.h.h1` (d) and (e): "(30,0;30,3;33,3;33,0)", "(40,0;40,3;43,3;43,0)" - a via
  not covered is a plate without a via.
- Verdict: MIM.d fires on the via over the wall and the via over the corner: the MIM's
  enclosure of that via is negative, under 0.36 by any reading, and it is the cap's
  via (it touches the MIM).  gdscheck's MIM.c reads the same geometry the right way
  ("shape on MIM not enclosed by Metal5" for a plate 0.5 over Metal5's edge,
  `MIM.c.h1` (g)); MIM.d's `interacting_only` path does not.  MIM.h fires on the
  abutting via - a via touching the plate's wall from outside is not over it - and not
  on the via over the wall, which is over the plate and MIM.d's business.  Expected 5
  (`mim_d_h1`) and MIM.h × 4 + MIM.d (`mim_h_h1`).  A via that has slid off its MIM
  plate is silent in gdscheck today; KLayout reports it as MIM.h.

### 6. The cap's own exit wire 0.595 from its MIM is not MIM.e (false negative)

Manual: "MIM.e  Min. TopMetal1 space to MIM  0.60"; figure 6.13 draws `e` between the
MIM and a TopMetal1 outside it.

Layout `MIM.e.h1` (g) at (72, 0): the cap's TopMetal1 top plate (72.4-74.6) with an
exit wire leaving over the MIM's right wall, rising at x = 76-77 and coming back along
the MIM's top wall (y = 3) from x = 71 to 77 at y = 3.595-4.595, 0.595 above it.
Controls: (e) the top plate alone and (f) the top plate with the exit wire crossing
the wall, both clean; (b) at (12, 0) a separate wire 0.595 from the plate, fires.

- gdscheck @20/7/100: MIM.e on (b), (c) and (h), nothing on (g).
- KLayout: MIM.e on (g), "(75.077,3.595;71.923,3.595)/(72,3;75,3)", besides (b) and (c).
- Verdict: fires.  The rule has no same-net or same-device exemption, and the wire's
  wall is 0.595 from the MIM's; that the same TopMetal1 polygon also overlaps the MIM
  (as every cap's top plate does) is no reason to drop the pair.  gdscheck's
  two-layer space seems to skip a pair of shapes that overlap.  Expected 4
  (`mim_e_h1`).

### 7. A circle with a hole passes Padb.f (false negative)

Manual: "Padb.f  Allowed passivation opening shape  Octagon, Circle".

Layout `Padb.f.h1` (l) at (690, 170): a 64-point circle of radius 30 with a 64-point
hole of radius 5.

- gdscheck @20/7/100: Padb.f on the square, diamond, hexagon, D, ellipse and 16-gon
  (six), not on the ring.
- KLayout: `padb.f` on all seven (its `get_circle` has `without_holes`).
- Verdict: fires; a ring is not a circle.  Expected 7 (`padb_f_h1`).

## Notes that are not findings

- A. Pad.a1 reads the opening, Passiv ∩ dfpad, as the section's "Pad rules are tested
  only within dfpad recognition layer" says.  `Pad.a1.h1` (o) Passiv 200 under dfpad
  200 × 100 and (q) Passiv 160 with no dfpad are clean in gdscheck; IHP's maximal deck
  reads Passiv alone (`Passiv.sized(-75).sized(75)`) and reports both (18 markers to
  gdscheck's 7).  Its erosion reading and gdscheck's narrowest span agree on every
  other shape: the 300 × 150.005 bar, the octagon, the diamond, the ring with 180
  walls fire; the L of 100 arms and the ring with 100 walls do not.
- B. IHP's driver runs Padb.c at `10 − 2·0.005` and is silent on every 9.995 layout
  (`Padb.c.h1`-`h4`: gdscheck 12/2/50/50, KLayout 1/0/0/0, the 1 the pad with no
  TopMetal2).  Its Padc.c has no such tolerance and agrees at 7.495.  Its projection
  metric misses the chamfer cases (the settled reading).
- C. Pas.c "not checked outside of sealring".  gdscheck's `PassivInSeal` is the Passiv
  within the EdgeSeal ring's outer boundary, hole and frame; IHP's is the Passiv in
  the frame's holes.  `Pas.c.h1` (h), an opening on the frame itself 2.095 short of
  TopMetal2, fires in gdscheck and not in KLayout.  On the seal ring is not "outside
  of sealring", so the case expects it; IHP's seal ring pcell keeps its Passiv ring
  3 µm outside the EdgeSeal ring's outer edge, so the case never arises in a real
  die.  An opening beyond the frame (g) is clean in both.
- D. Pad.d and Padc.d read "EdgeSeal" as the seal's Activ (`SealActiv`), as IHP's
  `Act_EdgeSeal` does: `Pad.d.h1` (i), an EdgeSeal marker with no Activ 7.0 from an
  opening, is clean in both; (h), Activ with no EdgeSeal, too.  Padb.d reads the
  EdgeSeal marker itself in both.  Padc.d's grow-and-intersect is round at the
  corners: 29.995 corner to corner fires and 30.01 does not, in both.
- E. MIM.b is "Min. MIM space", where Pas.b is "Min. Passiv space or notch"; the deck
  has `min_notch` for Pas.b and not for MIM.b, and the cases follow the wording (as the
  topvia report's note C did for TV.b).  `MIM.b.h1`: a U of 0.59, a comb with 0.595
  slots and a ring with a 0.59 hole are silent in gdscheck and MIM.b to KLayout
  (`space` covers notches there): 5 to 14.  A 0.59 notch in a MIM plate is the same
  distance between the same layer's walls as a 0.59 space; if the gdscheck owner reads
  MIM.b like Pas.b, `mim_b_h1` gains 5 (U, two slots, ring both ways).
- F. Counts.  A square 3.495 (0.595) from all four walls of a ring's hole is one pair
  to gdscheck and four edge pairs to KLayout (`Pas.b.h1` (m), `MIM.b.h1` (l)); an
  enclosure short all round is one run to gdscheck (`Padb.c.h1` (c), `MIM.c.h1` (h),
  `Pas.c.h2` (c)) and four to KLayout; a corner-to-corner space is one to gdscheck and
  two to KLayout; a 59.995 square is four walls to gdscheck and one polygon to
  KLayout; the hierarchical KLayout run counts an array cell once.
- G. A Vmim 0.355 from the MIM's wall (`MIM.d.h1` (f)) is silent in both; the rule
  names TopVia1, though 6.11 says "Vmim can be used instead of TopVia1".  MIM.h
  accepts a Vmim in both.
- H. KLayout's own gaps, for the record: Padb.a/Padc.a miss a 60.005 square, a 60 ×
  59.995 and a 60 × 50 pad (its `projection_limits(nil, 0.05)` keeps only edge pairs
  with under 0.05 of projection - the circle's short walls - and its bbox test reads
  the x extent alone); Pas.c misses TopMetal2 coincident with the opening (`Pad.d.h2`,
  whose openings have TopMetal2 equal to them: gdscheck Pas.c 3 in the ring's hole,
  KLayout 0); Padb.b reports a ring-shaped pad's own 10 µm hole 992 times
  (`Padb.f.h1`).
- I. A via with no MIM under it is nothing to either tool, and a MIM with no Metal5
  is MIM.c "not enclosed" to gdscheck and MIM.c (`.not(metal5)`) to KLayout.
- J. MIM.gR reads the chip total through an array reference (`MIM.gR.h1`, 32 caps
  placed by one GdsArrayRef: 175232 in both).

## Tested and found clean or correct (no need to redo)

- Pad.a1: 150 vs 150.005 (square), 150.005 × 100 and 100 × 150.005 clean, 300 ×
  149.995 clean vs 300 × 150.005, regular octagons of 150 vs 150.005, diamonds of
  149.9 vs 150.6, an L of 100 arms 300 long clean, rings with 100 walls clean vs 180
  walls, two overlapping boxes once, dfpad 200 × 100 under Passiv 200 clean vs dfpad
  160, Passiv without dfpad clean; squares across x = 20/21/40/42/100, starting on 20,
  ending on 100, at (1000, 1000); fifty flat and as an array.
- Pad.d: 7.5 vs 7.495, corner to corner 7.495 (5.3 each axis) vs 7.517 (7.5, 0.5) vs
  9.9 (7, 7), the opening at 7.5 with Passiv reaching 5.0 vs the opening at 7.495, a
  corner 7.495 vs 7.502 from a 45° seal wall; a seal ring with 21 µm 45° corners whose
  inner wall lies on x = 20: 7.495 from the wall (gap across 21 and 28), 7.5 clean,
  the corner 7.495 from the ring's 45° corner, 7.495 outside the ring; fifty at 7.495
  from one bar, flat and as an array over the bar in TOP.
- Pad.i: 0.005 short, a hole, four cut corners (four pieces), no TopMetal2, TopMetal2
  ending on x = 100, at (1000, 1000); the same box, two halves, a chamfered dfpad, two
  overlapping dfpad boxes clean; fifty flat and as an array.
- Padb.a: 59.995, 60.005, 60 × 59.995, octagons of 59.995 and 60.005, a circle of
  radius 29.995, a 60 × 50 pad (dfpad 60 × 50 under Passiv:sbump 60); 60 as two boxes
  clean; squares across x = 20/21/40/42, ending on 100, starting on 100, at (1000,
  1000); fifty flat and as an array.  Padb.b: 70 vs 69.995, corner to corner 70.004
  vs 69.99, dx = 69 with dy = 100 clean, two regular octagons 70.0015 vs 69.9965
  across their diagonals, two circles 70 vs 69.995 vertex to vertex; gaps across 100,
  starting on 100, at (1000, 1000); fifty pairs flat and as an array (a 300 × 200
  pitch).  Padb.c: 10 vs 9.995 on one side and all round, a cut passing 10.002 vs
  9.995, a regular octagon in a regular octagon (10.002 on the grid) clean, a circle in
  a circle of radius 40.02 clean vs in a 79.99 square (four vertices), no TopMetal2
  (and Pad.i), two abutting boxes and a plate with an exit wire clean; TopMetal2
  ending on x = 100 at 9.995 vs 10, at (1000, 1000); fifty flat and as an array.
  Padb.d: 50 vs 49.995, corner to corner 50.006 vs 49.992, inside an EdgeSeal frame,
  at (1000, 1000), Activ without EdgeSeal clean.  Padb.f: a regular octagon, an
  octagon cut 10, one cut 10 and 15, 64- and 128-point circles pass; a square, a
  diamond, a hexagon, a D, an ellipse, a 16-gon fail.
- Padc.a: 35 vs 34.995 and 35.005, a circle of radius 17.495.  Padc.b: 40 vs 39.995
  (35 and 40 µm pads), corner to corner 40.008 vs 39.99, 45 µm pads at 50 clean.
  Padc.c: 7.5 vs 7.495, a cut passing 7.502 vs 7.495, no TopMetal2 (and Pad.i), a
  circle in a circle of radius 25.02 clean.  Padc.d: 30 vs 29.995, corner to corner
  30.01 vs 29.995, dx = 29 with dy = 40 clean, Activ without EdgeSeal clean, a corner
  29.995 from a 45° seal wall.  Padc.f: 64- and 128-point circles pass; a regular
  octagon, a square, a 16-gon, a D fail.
- Pas.a: 2.1 vs 2.095 in x and y, 45° strips of 2.1001 vs 2.093, diamonds of 2.1001
  vs 2.093, an L of 2.095 arms, a 2.095 × 4 neck, a 0.005 sliver, a 300 µm bar, a bar
  at (1000, 1000); bars across x = 20/21, 40/42, starting on 100, across y = 20,
  ending on x = 20, 2.1 across 60 clean; fifty flat and as an array.
- Pas.b: 3.5 vs 3.495, corner to corner 3.507 vs 3.493, dx = 3 with dy = 6 clean,
  45° strips 3.5002 vs 3.4931 apart, a U of 3.5 vs 3.49, a comb with two 3.495
  slots, a 3.495 slot, a ring with a 3.49 hole (both ways), a square 3.5 vs 3.495 from
  a hole's walls, two overlapping boxes 3.495 from a square (once); gaps across 20 and
  21, across 40, starting on 20, ending on 42, across 100, at (1000, 1000), corner
  to corner on (60, 60); fifty pairs flat and as an array.
- Pas.c: in an EdgeSeal frame's hole, 2.1 vs 2.095, a cut passing 2.104 vs 2.093, no
  TopMetal2, two abutting boxes clean, an opening outside the frame not checked; a
  frame whose hole starts on x = 20 with margins ending on 20, across 40 and 42, all
  round across 100, at (1000, 1000) in its own frame, 2.1 ending on 102.1 clean; fifty
  in one frame drawn in TOP, flat and as an array.
- MIM.a: 1.14 vs 1.135 in x and y, 45° strips of 1.1455 vs 1.1314, diamonds of
  1.1455 vs 1.1314, an L, a neck, a 0.005 sliver, a 300 µm bar, (1000, 1000); bars
  across 20/21, 40, starting on 100, across y = 20, ending on 42, 1.14 across 7
  clean; fifty flat and as an array over one Metal5 plate in TOP.
- MIM.b: 0.6 vs 0.595, corner to corner 0.601 vs 0.594, dx = 0.5 with dy = 3 clean,
  45° strips 0.601 vs 0.594 apart, a square 0.6 vs 0.595 from a hole's walls, two
  overlapping boxes 0.595 from a square; gaps across 20, 21, starting on 40, ending
  on 42, across 100, at (1000, 1000), corner to corner on (60, 60); fifty pairs flat
  and as an array.
- MIM.c: 0.6 vs 0.595 on one side and all round, a cut passing 0.601 vs 0.594, no
  Metal5, the plate 0.5 over Metal5's edge, two abutting boxes clean; margins ending
  on 20, starting on 20, across 40, across 100, at (1000, 1000), 0.6 starting on 20
  clean.
- MIM.d: 0.36 vs 0.355 on one and two walls, a cut passing 0.3606 vs 0.3536 from the
  via's corner, a via outside the plate (with the plate's own via) clean; margins
  ending on 20, starting on 20, across 40, across 100, at (1000, 1000), 0.36 starting
  on 20 clean.
- MIM.e: 0.6 vs 0.595, corner to corner 0.601 vs 0.594, the top plate over the MIM
  and its exit wire crossing the MIM's wall clean, a 45° TopMetal1 wall passing 0.594
  from the plate's corner.
- MIM.f: 1.2996 and 1.2935 fire, 1.3053 and 1.3 clean, two squares sharing a wall are
  one device (2.0, clean), overlapping (1.75) clean, an L of 2.04 clean, two squares
  touching at a corner are two devices (twice).  MIM.g: 5625 (75 × 75, 100 × 56.25)
  clean vs 75 × 75.005, two 60s sharing a wall (7200), two 60s overlapping (6000), an L
  of 7500, 75 × 75.005 as two boxes (once), a ring of 5500 clean.  MIM.gR: 32 caps in
  an array reference.  MIM.h: a TopVia1, a Vmim, two plates sharing a wall with one
  via clean; no via, a ring with the via in its hole, no via at (1000, 1000) fire.
