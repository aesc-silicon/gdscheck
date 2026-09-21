<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / sealring, slit, lbe, lu: hardening report

Decks `sealring`, `slit`, `lbe` and `lu` against SG13G2 Layout Rules Rev. 0.4, sections
6.10 (Sealring), 7.3 (Metal Slits), 9.1 (Localized Backside Etching) and 7.2.2 (Latch-up
ties).  `gen/ihp_sg13g2/beol_misc_hardening.rs` draws 54 layouts
(`tests/data/ihp-sg13g2/{sealring,slit,lbe,lu}/<RULE>.h<k>.gds.gz`: 19, 14, 15 and 6),
each with a `#[case]` in the deck's table of `tests/ihp-sg13g2.rs`.  Every layout ran
through gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`ci/hardening/oracle-ihp.sh`); the density rules (Slt.i, LBE.i), which the oracle's
suite leaves out, ran through `gdscheck run --deck` at the three tile sizes, and the
latch-up fixtures also through the maximal deck with `latchUpRules=true`, which the
oracle switches off.  The seal frames are drawn as IHP's `sealring` pcell draws them: the
EdgeSeal marker is the 4.2 µm ring itself with the conductors coincident on it, the via
rings are strips overlapping at the corners, the Passiv ring lies 3 µm outside.

`show-deck` against the manual:

- `sealring`: Seal.a (nine conductors), Seal.b (nine), Seal.c/c1/c2/c3 as `min_width`,
  Seal.d (seven via rings, euclidian, `skip_coincident`, `interacting_only`), Seal.e on
  `RingPassiv` (Passiv not overlapping EdgeSeal), Seal.f (nine), Seal.l (`forbidden`,
  `op: beyond` EdgeSeal, Passiv/Pad/boundaries/dt30 ignored), Seal.n
  (`ring_covers_boundary`).  Seal.k and Seal.m carry the manual's note 1, "not checked
  during DRC" (IHP's decks run both).  Every other row of 6.10 is in the deck.
- `slit`: Slt.a, b, c (`wide_uncovered` on the metal less Passiv, MIM and IND - that is
  Slt.e, e1 and e2 as exemptions), e (`forbidden` on slit ∩ pad), f, g, h1-h4, i
  (`min_density`, `min_size: 35`, `scope: region`): every rule of 7.3.
- `lbe`: LBE.a, b (`max_length`), b1, b2 (`touching: separate`), c (`min_space` and
  `min_notch`), d, e (dfpad and Passiv), f, h (`no_ring`), i (`max_density` in
  EdgeSeal.boundary): every rule of 9.1.
- `lu`: LU.a, b, c, c1, d, d1 as `max_space`: every rule of 7.2.2.  The deck reads LU.c
  and LU.d (LU.c1 and LU.d1) with the same layers and value, so each fires with the
  other: finding 16.

Reading the numbers below: gdscheck gives one marker per wall for a width (eight for a
ring 0.005 under the value, two for a bar), one per pair for a space (a ring's four
walls against another ring's are one), one per run for an enclosure (a ring short on
all four walls is one), one per uncovered region for Slt.c, one per plate for Slt.i,
one per portion for the latch-up `max_space`.  KLayout's column is the driver's tables
plus the maximal deck; Seal.b, LBE.a-d and LU.b are the driver's, the rest of 6.10, 7.3
and LBE.b2/e/f are the maximal deck's, so a few count twice.  Every flat/array pair
agreed (Seal.a/b/d, Slt.a/c/i, LBE.a, LU.a); the counts that moved with the tile size
are findings 3 and 4.

Test status on the engine as of this report: 15 of the 54 new cases fail at tile 20
(`seal_b_h1`, `seal_c_h1`, `seal_d_h1`, `seal_e_h1`, `seal_f_h1`, `seal_l_h1`,
`seal_l_h2`, `slt_h_h1`, `lbe_e_h1`, `lbe_h_h1`, `lbe_i_h4`, `lu_a_h1`, `lu_b_h1`,
`lu_c_h1`, `lu_c1_h1`); at tile 7 `seal_a_h1` and `seal_a_h2` fail too (and `seal_d_h1`
passes by accident, finding 3); at tile 100 `slt_c_h1` fails too (finding 4).

## Resolution (2026-09-21)

Fixed, engine: 3 (a width pair is owned by the low end of the stretch its walls share,
half-open, not by the stretch's midpoint: a tile's copy of a drawn layer is the merge of
the shapes reaching its zone, so a U across a line is an L in each side's copy and the
stretch between the arms had a different midpoint in each - neither tile owned the
ring's walls at tile 20; a bar drawn as two boxes had two midpoints and was reported
twice.  Enclosure runs join on the walls of the region put together from its pieces,
so a wall cut 0.095 short of its corner by a tile line still meets the next wall there:
`Seal.d.h1` (c) is one run at every tile), 4 (the wide spot of a plate is a point inside
the eroded metal cut to the core, not its centroid: the eroded ring's centroid lay in
its hole, the L's off its arms), 7 (`regions_overlap` read a shape with every vertex on
the rim of another's hole as lying inside it, and the touching Passiv ring was set aside
as overlapping the seal; a point inside the shape decides), 10 (a slit's area on a plate
is the intersection, per core, not the whole slit charged to the plate holding its
centroid: 5.875 % and 5.25 % at every tile), 11 (a notch between two pieces of one region
in a tile - the two halves of a cut wall, joined beyond the zone - is read as the spacing
scan reads a pair, at the closest approach where the walls run alongside, owned by the
stretch's low end; the closest approach prefers a parallel pair to a corner at the same
distance, so its marker is the same in every copy), 15 (`within` confines `scope: part`
too: LU.a's reach grows inside `NWell`).

Fixed, deck: 1 (Seal.l reads beyond `EdgeSeal.boundary`, as the manual, the pcell and
IHP's deck have it; with no boundary drawn there is nothing to be outside of, and the
static `Seal.l.fail` fixture's two squares moved beyond the boundary), 2 (`RingPassiv` is
the Passiv with a hole - `PassivRings` - not overlapping the seal, not every opening on
the die), 5 (Seal.c-c3 keep the minimum and add a `max_width` with `span: narrowest`,
IHP's `sized(-w/2).sized(w/2)`: a ring wider than the value is one region wider than it
in every direction, so the 0.2 Cont ring and the 0.25 Via1 ring are one report each,
not eight - `Seal.d.h2`'s 45° rings, drawn 0.163 wide on the diagonals, fire it too),
6 (Seal.d without `interacting_only`/`skip_coincident`: a ring on no Activ is enclosed
by nothing), 7 (Seal.f `abutting: report`), 8 (Seal.b reads `ActivNoSealring`, Activ
less the marker, `abutting: report` - the bar running into the wall abuts the seal's
Activ and pSD where the marker cuts it, two reports as (g) has two), 9 and 12 (a
`forbidden` `op: overlap` entry beside every Slt.h1-h4 and LBE.e space entry: a via in
a slit, a Passiv or dfpad over the LBE), 16 (LU.c/c1 read the abutted tie with no Cont
of its own (`NWellTieAbuttedNoCont`, `SubstrateTieAbuttedNoCont` - from the Conts on the
abutting tap within 6 along `ActivNotGat`; LU.d/d1 the tie standing alone,
`NWellTieStandalone` / `SubstrateTieStandalone`, the N+ (P+) on an Activ no gate
touches, from the Conts on the ties of its well, squarely - confined to the Activ it
cost 25 s on a 4 mm² design; a tie 6.005 past its
Cont at both ends is two parts out of reach and two markers, `part` scope, as KLayout
has two polygons).

Kept: 13 (with no boundary drawn the box of everything stands in for the die, as for
every density rule: an LBE alone is 100 % of it), 14 (the reach is grown squarely, as
IHP's deck grows it - the euclidian reach would want a round grow of thousands of Cont
squares at 0.005 precision; the 20.5 corner case stays clean and is listed for the
owner).

Decided cases: `seal_c_h1` 9/33/8/8, `seal_d_h2` + Seal.c 8, `seal_b_h1` 7, `lu_a_h1`
LU.a 4 + LU.d, `lu_b_h1` LU.b 3 + LU.d1, `lu_c_h1`/`lu_c1_h1` LU.d 6 + LU.c, `lbe_i_h4`
LBE.i, the older `lu_c`/`lu_c1` LU.d alone.

## Findings

### 1. Seal.l reads "outside the EdgeSeal ring", not "outside the sealring boundary" (false positive)

Manual, 6.10: "Seal.l  No structures outside sealring boundary allowed"; the
introduction: "Figure 6.12 shows distance between EdgeSeal and the sealring boundary
(30 µm)", and the figure draws the boundary as a dashed box outside the EdgeSeal ring.
The pcell draws it on EdgeSeal.boundary (39/4), `edgeBox` outside the ring; IHP's deck
reads `layer.not(edgeseal_bound)`.

Layout `Seal.l.h1`: a frame (EdgeSeal + Metal1, box (10, 10)-(50, 50)), its Passiv ring
3 outside, the boundary at (0, 0)-(60, 60); (a) a Metal1 square at (52, 2)-(55, 5),
between the ring and the boundary; (b) Metal1 at (65, 10) beyond it; (c) a Metal2 bar
(55, 30)-(65, 32) across it; (d) a Passiv square beyond it; (e) an Activ square beyond
it.  `Seal.l.h2`: the same frame with no boundary drawn and a Metal1 square 5 outside
the ring.

- gdscheck @20/7/100: `Seal.l.h1` 4 - (a), (b), (c), (e), "shape on GDS layer 8/0
  outside EdgeSeal at (52.0000, 2.0000)" for (a); `Seal.l.h2` 1.
- KLayout: `Seal.l.h1` 4 - (b), (c), (e) and (d): its deck has no Passiv exemption;
  (a) is clean.  `Seal.l.h2`: 0, the rule is skipped without a boundary.
- Verdict: (a) is inside the sealring boundary and clean, and with no boundary there is
  nothing to be outside of; the deck's `op: beyond` reads the EdgeSeal drawing where
  the manual and the pcell put the boundary on EdgeSeal.boundary, 30 µm further out.
  Expected 3 (`seal_l_h1`) and 0 (`seal_l_h2`).  The same reading is why every
  multi-frame fixture of this deck reports Seal.l on every other frame's metal (the
  cases ignore it).  KLayout's flagging of Passiv beyond the boundary is its own
  reading; the pcell's Passiv ring lies inside the boundary, so it never arises there.

### 2. Seal.e reads any Passiv opening as the ring outside the sealring (false positive)

Manual, 6.10: "Seal.e  Min. Passiv ring width outside of sealring  4.20".  Pas.a
allows a Passiv opening of 2.1.  IHP's deck reads `Passiv.with_holes.outside(EdgeSeal
.with_holes)`: the Passiv polygons with a hole, outside the seal ring.

Layout `Seal.e.h1` (d): a 3 × 3 Passiv opening at (28, 28) in the hole of a seal frame
whose own Passiv ring is 4.2 wide; `Seal.l.h1` (d) before its square was made 5 × 5: a
3 × 3 Passiv square beyond the boundary.

- gdscheck @20/7/100: Seal.e 14 on `Seal.e.h1`: the 4.195 ring (8), the nick (2), and
  "RingPassiv: width 3.0000 µm < 4.2000 µm" four times on the 3 × 3 opening.
- KLayout: Seal.e 9 (the ring's four pairs, the nick), nothing on the opening.
- Verdict: clean.  `RingPassiv` is "Passiv not overlapping EdgeSeal", which is every
  pad opening on the die; the rule reads the ring - a Passiv shape with a hole that
  holds the seal.  A 3 µm pad opening anywhere on a real die is a Seal.e violation to
  gdscheck.  Expected 10 (`seal_e_h1`).

### 3. Ring width markers are lost on the tile lines (tile-dependent)

Manual, 6.10: Seal.a (3.50), Seal.c (0.16), Seal.c1 (0.19), Seal.c2 (0.42), Seal.c3
(0.90).

Layout `Seal.c.h1`: seal frames (EdgeSeal + Activ + Metal1, box (5 + 40i, 5)-(35 +
40i, 35)) with via rings 2.0 in: (b) Cont 0.155, Via1 0.185, TopVia1 0.415, TopVia2
0.895; (d) Via2/Via3/Via4 0.185.  `Seal.a.h1`: seven 3.495 frames of 20 boxes on a
30 pitch (8 walls each) and a 0.5 × 3 bar (4).  `Seal.a.h2`: the same across the tile
lines.

- gdscheck: `Seal.c.h1` Seal.c 0 / 8 / 8, Seal.c1 0 / 32 / 32, Seal.c2 0 / 8 / 8,
  Seal.c3 0 / 8 / 8 at tiles 20 / 7 / 100 - at tile 20 not one of the 56 markers.
  `Seal.a.h1` 60 / 24 / 60: at tile 7 frames (b), (c), (e), (f) keep four of their
  eight walls, (d), (g), (h) lose all eight.  `Seal.a.h2` 58 / 46 / 58: the 3.495 ring
  across x = 20 and x = 40 keeps four.  A probe (not kept): one frame at (5, 5)-(35,
  35) with a 0.155 Cont ring at 7..33 is 0 at tile 20 and 8 at tiles 7, 10, 13, 15, 26,
  30, 40 and 100; the same frame moved to x = 48 (the ring 50..76, cut at x = 60 off
  its middle) is 4 at tile 20.  The rings cut through the middle of their walls lose
  the halves; a straight 0.155 × 26 bar in an EdgeSeal box at the same place is
  reported at every tile.
- KLayout: Seal.c 4 pairs on (b), Seal.c1 on (b) and (d), Seal.c2 and Seal.c3 on (b);
  Seal.a 4 pairs per frame.
- Verdict: 8 per ring at every tile.  A ring cut by a tile line into pieces is one ring;
  its wall pairs that end on the line are reported at tile 7 and 100 and not at 20, or
  at 20 and 100 and not at 7.  The existing `Seal.d` case's comment records the same
  class for enclosures ("the pieces the tiling cut the ring into").  `Seal.d.h1` shows
  it on the enclosure count too: 3 at tiles 20 and 100, 4 at tile 7, where the Via1
  ring's left and right walls (1.295 from the inner edge) are two runs instead of one.
  The `metal1` deck, run alongside in the oracle's suite, has it on `Seal.a.h1`'s (h)
  frame as well: the 0.005 Metal1 sliver the EdgeSeal clip leaves outside the marker
  is M1.a 8 / 0 / 8.  Expected: Seal.c 16, Seal.c1 40, Seal.c2 8, Seal.c3 8
  (`seal_c_h1`, with finding 5), 60 and 58 (`seal_a_h1`, `seal_a_h2`).

### 4. Slt.c loses two regions at tile 100 (tile-dependent)

Manual, 7.3: "Slt.c  Max. Metal width without requiring a slit  30.00".

Layout `Slt.c.h1`: (j) a Metal1 ring (80, 120)-(150.01, 190.01) with 30.005 walls;
(m) an L of 30.005 arms, (170, 120)-(200.005, 180) and (170, 120)-(230, 150.005).

- gdscheck: Slt.c 11 / 11 / 9 at tiles 20 / 7 / 100: at 100 the ring's marker
  "(98.3350, 138.3350)" and the L's "(205.0000, 135.0025)" are gone, the other nine
  stay ((b), (c), (h), (k), (n), (o), (p), (q), (v)).
- KLayout: 12 regions (its `sized(-3).sized(-12).sized(15)`), the ring and the L among
  them.
- Verdict: 11 at every tile.  The two lost shapes cross x = 100 and x = 200; the
  30.005-wide ones that cross a 20-line ((o) across 20, (q) across 200) are kept at
  tile 20, so the loss is in the 100 grid's handling of a region wider than its halo
  reaches, not in the lines as such.  Expected 11 (`slt_c_h1`).

### 5. Seal.c/c1/c2/c3 read a minimum where the manual gives the ring's width (false negative)

Manual, 6.10: "Seal.c  EdgeSeal-Cont ring width  0.16", "Seal.c1  EdgeSeal-Via(n=1-4)
ring width  0.19", "Seal.c2  ... 0.42", "Seal.c3  ... 0.90" - the only rows of the
table without "Min."; Cnt.a and V(n).a are exact widths, and 6.10 says the standard
via rules are not checked within EdgeSeal.  The pcell draws every via ring exactly
Cnt_a, Vn_a, TV1_a, TV2_a wide.

Layout `Seal.c.h1` (c): a frame at (85, 5) with a Cont ring 0.2 wide and a Via1 ring
0.25 wide.

- gdscheck @20/7/100: nothing on (c) (at 7 and 100; at 20 nothing at all, finding 3).
- KLayout: Seal.c reports the 0.2 Cont ring as one polygon "(87,7;87,33;113,33;113,7/
  ...)" and Seal.c1.Via1 the 0.25 ring: its second check, `sized(-w/2).sized(w/2)`,
  outputs whatever survives an erosion of half the width - a ring at the width
  vanishes, a wider one is reported.
- Verdict: fires.  A 0.2 Cont ring in the seal is a width the process does not make
  (Cnt.a is 0.16, exact) and no other rule reads it there; IHP's deck reads the row as
  exact by construction.  The deck's `min_width` should be an `exact_width` (as the
  contact and via decks have for Cnt.a and V(n).a).  Expected 8 more Seal.c and 8 more
  Seal.c1 in `seal_c_h1`.

### 6. Seal.d does not read a via ring that no Activ encloses (false negative)

Manual, 6.10: "Seal.d  Min. EdgeSeal-Activ enclosure of EdgeSeal-Cont, EdgeSeal-Via
(n=1-4), EdgeSeal-TopVia1, EdgeSeal-TopVia2 ring  1.30".

Layout `Seal.d.h1` (f): a frame (205, 5)-(235, 35) of EdgeSeal + Metal1 with a Cont
ring 2.0 in and no Activ at all.

- gdscheck @20/7/100: nothing on (f) (`interacting_only`: the ring interacts with no
  Activ).
- KLayout: nothing (`ext_enclosed` on `Cont.and(EdgeSeal)` against `Activ.and
  (EdgeSeal)`, plus `ext_overlapping`: a ring on no Activ is in neither).
- Verdict: fires.  An enclosure of 1.3 that no Activ provides is an enclosure of
  nothing, as the settled reading has it for the corner case; a seal ring whose Cont
  ring has lost its Activ is broken and neither tool says so.  Expected 4
  (`seal_d_h1`).  The pcell's own vias are on Activ, so this is a hand-drawn seal's
  failure mode.

### 7. Seal.f is silent on a Passiv ring touching the seal (false negative)

Manual, 6.10: "Seal.f  Min. Passiv ring outside of sealring space to EdgeSeal-Activ,
EdgeSeal-Metal(n=1-5), EdgeSeal-TopMetal1, EdgeSeal-TopMetal2  1.00".

Layout `Seal.f.h1` (e): a frame (EdgeSeal + Metal1, box (1000, 1000)-(1040, 1040))
whose Passiv ring's hole is the frame's box - the ring touches the metal along its
outer edge.

- gdscheck @20/7/100: Seal.f 5 - (b), (c), (d) three times - nothing on (e).
- KLayout: Seal.f.Metal1 on (b), (d) and (e), the latter as the polygon of the touching
  ring (its `ext_coincident_edges(..., outside: true)` branch).
- Verdict: fires.  A space of 0 is under 1.00; the deck's `min_space` reads touching as
  it reads overlap, as nothing.  Expected 6 (`seal_f_h1`).

### 8. Seal.b is silent on an Activ running into the seal (false negative, low confidence)

Manual, 6.10: "Seal.b  Min. Activ space to EdgeSeal-Activ, EdgeSeal-pSD, EdgeSeal-Metal
(n=1-5), EdgeSeal-TopMetal1, EdgeSeal-TopMetal2  4.90".

Layout `Seal.b.h1` (h): a circuit Activ bar (120, 40)-(122, 48) running from the hole
of an EdgeSeal + Activ + pSD frame (box (100, 10)-(140, 50)) into its top wall.

- gdscheck @20/7/100: Seal.b 5 - (b), (c), (f), (g) twice; nothing on (h).
- KLayout: the same five pairs; nothing on (h) (`sep` does not report overlaps).
- Verdict: fires - a space of 0 - but both tools skip overlapping pairs by design, and
  the seal's own Activ (EdgeSeal-Activ is a part of Activ) relies on it.  If the
  overlap rule stays, the case's expectation drops to 5; the settled readings do not
  cover a partial overlap of a circuit shape with the seal.  Expected 6 (`seal_b_h1`).

### 9. Slt.h1 is silent on a via lying in a slit (false negative)

Manual, 7.3: "Slt.h1  Min. Metal1:slit space to Cont and Via1  0.30".

Layout `Slt.h.h1` (l): a Metal1 plate (64, 20)-(76, 32) with a slit (66, 23)-(68.8,
29) and a Via1 at (67.3, 25)-(67.49, 25.19) inside the slit.

- gdscheck @20/7/100: Slt.h1 3 - (b) Cont 0.295, (c) Via1 0.295, (m) the corner 0.297;
  nothing on (l).
- KLayout: the same three (the corner as two pairs); nothing on (l).
- Verdict: fires.  A via in a slit sits on no metal and is nearer than 0.3 to the slit;
  it is the same overlap-is-nothing reading as finding 7.  Expected Slt.h1 4
  (`slt_h_h1`).

### 10. Slt.i's density is read off a wrong and tile-dependent slit area (value, not verdict)

Manual, 7.3: "Slt.i  Min. Metal:slit density for any Metal plate bigger than 35 µm x
35 µm [%]  6.00".

Layout `Slt.i.h1`: (f) an L of a 56 × 90 bar (160, 50)-(216, 140) and a 30 × 56 stub
(216, 50)-(246, 106) with a slit 2.8 × 88 along the bar and one 2.8 × 53 along the
stub (394.8 of 6720 µm², 5.875 %); (g) a 40 × 40 plate with three 3 × 8 slits and a
3 × 8 slit across its right edge, 12 µm² of it on the plate (84 of 1600, 5.25 %).

- gdscheck: (f) "density 5.54% < 6.00%" at tiles 20 and 100, "5.12%" at tile 7; (g)
  "4.50%" at every tile - the crossing slit counts for nothing, not even its part on
  the plate.  The verdicts are right (both are under 6 %); (e), a 48 × 100 bus with
  one lengthwise slit, reads its 5.72 % exactly, as do the single-box plates.
- KLayout: no Slt.i (the oracle runs without density; IHP's deck has none).
- Verdict: 5.875 % and 5.25 %, the same at every tile.  A slit's area is what lies on
  the plate; a slit that crosses from one drawn box of the plate to another, or over
  the plate's edge, loses area, and how much depends on the tile.  A plate at 6.0 %
  whose slits cross its boxes' seams would fire.  No case is written for the value;
  `slt_i_h1` expects the nine plates that fire.

### 11. LBE.c misses a channel through a ring's wall (false negative)

Manual, 9.1: "LBE.c  Min. LBE space or notch  100.00".

Layout `LBE.h.h1` (f): a C - a ring 500 outer with 100 walls whose top wall is two
pieces, (1200, 1000)-(1449.995, 1100) and (1450, 1000)-(1700, 1100), a 0.005 gap.  A
probe (not kept) widened the gap to 0.05, 0.5, 5 and 50: silent every time; two
separate 200 squares 0.005 apart fire.

- gdscheck @20/7/100: LBE.h 5, no LBE.c on the C.
- KLayout: LBE.c "(1450,1000;1450,1100)|(1449.995,1100;1449.995,1000)" (`space` covers
  the same polygon's edges).
- Verdict: fires.  The gap is two walls of one merged shape facing 0.005 apart; it is
  not a notch (it opens into the hole on one side and outside on the other), and
  `min_space` reads a shape against others only.  Any slot cut right through a wall -
  a ring opened on purpose - is missed the same way.  Expected LBE.h 5 + LBE.c 1
  (`lbe_h_h1`).

### 12. LBE.e is silent on a Passiv over the LBE (false negative)

Manual, 9.1: "LBE.e  Min. LBE space to dfpad and Passiv  50.00".

Layout `LBE.e.h1` (f): a Passiv square (350, 150)-(450, 250) over the bottom-right
corner of the LBE (200, 200)-(400, 400).

- gdscheck @20/7/100: LBE.e 4 - the dfpad 49.995 above, the Passiv 49.995 left, the
  corner 49.992, the 45° wall 49.992; nothing on (f).
- KLayout: the same four (`consider_intersecting_edges: false`); nothing on (f).
- Verdict: fires - a pad opening over the etched region is nearer than 50.  The
  overlap-is-nothing reading again (findings 7, 8, 9).  Expected 5 (`lbe_e_h1`).

### 13. LBE.i with no boundary reads the layout's own extent (false positive)

Manual, 9.1: "LBE.i  Max. global LBE density [%]  20.00"; the density is the chip's,
and the deck's `boundary: EdgeSeal.boundary` says so.

Layout `LBE.i.h4`: one LBE 300 × 300 and nothing else.

- gdscheck @20/7/100: "density 100.00% > 20.00%".
- KLayout: no density run.
- Verdict: clean - with no boundary there is no chip, and a block that holds an LBE
  cannot be checked on its own.  Whether the answer is "clean" or "not checked", it is
  not 100 %.  Expected 0 (`lbe_i_h4`).  `LBE.i.h3`, an LBE reaching 100 beyond the
  boundary, reads 20 % of the boundary (clean, right): the part outside counts for
  nothing, and Seal.l has it.

### 14. LU.a/LU.b read "any portion" by a square, not a circle (false negative, low confidence)

Manual, 7.2.2: "LU.a  Max. space from any portion of P+Activ inside NWell to an
nSD-NWell tie  20.00" (LU.b the same for N+Activ in PWell to a pSD-PWell tie).

Layout `LU.a.h1` (c): an N+ tie (2, 62)-(3, 63) with its Cont and a P+ square (16,
76)-(17.5, 77.5): its far corner is 14.5 from the tie's corner on each axis, 20.5
diagonally; (d) a square whose far corner is 14.14 / 14.14 (19.997).  `LU.b.h1` (c),
(d) the same in the substrate.

- gdscheck @20/7/100: LU.a 3 - (b) far edge 20.005 ("at (23.0025, 40.0000)", the
  0.005 portion), (e) the 52 bar's portion beyond 23, (h) the P+ with no tie; nothing
  on (c).  The engine does read "any portion": a probe (not kept) with a 1 wide P+
  whose far edge is 20.5 from the tie fired on the 0.5 portion, one whose far edge is
  20.0 did not.
- KLayout (maximal, `latchUpRules=true`): LU.a on (b), (e), (h), (f) and (g); nothing on
  (c) either - its `ext_enlarge_inside(NWell, 20, 0.1)` grows the tie squarely too.
  The driver's LU.b is shape-level (`not_interacting` a 20 octagonal grow): 1 on
  `LU.b.h1`, the lone N+ (h).
- Verdict: fires.  Space is euclidian in this deck (Seal.b, LBE.c-f and every other
  space rule, the settled reading), and a point 20.5 from the nearest tie is 20.5 away
  whichever way the axes run.  Both tools grow by a square; the case follows the
  manual.  Expected LU.a 5 + LU.d 1 (`lu_a_h1`), LU.b 5 + LU.d1 1 (`lu_b_h1`).

### 15. LU.a counts a tie in another NWell (false negative)

Manual, 7.2.2, LU.a as above: the tie is the P+Activ's well's.

Layout `LU.a.h1` (f): an NWell (0, 150)-(6, 170) holding the tie (2, 159.5) and a
separate NWell (10, 150)-(50, 170) holding the P+ (12, 159)-(14, 161), 9 from the tie
across the 4 µm gap between the wells.

- gdscheck @20/7/100: nothing on (f).
- KLayout (maximal): LU.a "(12,159;12,161;14,161;14,159)" - the grow stays inside the
  tie's own well.
- Verdict: fires.  A tie biases the well it sits in; the P+'s well has none.  The deck's
  `max_space [PsdActivInNWell, NActivInNWell]` reads the layers over the whole die.
  Expected in `lu_a_h1` (finding 14's count).  The array `LU.a.h2/h3` had its pitch
  raised to 60 for this: at 35 the next column's tie satisfied 45 of the 50 cells.

### 16. LU.c/LU.d (LU.c1/LU.d1) fire together, and on figure 7.4's abutted tie (false positive)

Manual, 7.2.2: "LU.c  Max. extension of an abutted NWell tie beyond Cont  6.00",
"LU.d  Max. extension of NWell tie Activ tie beyond Cont  6.00" (c1/d1 for the
substrate).  Figure 7.4 draws `c`/`c1` as the tie of the other type abutting a
transistor's source, contacted through the source's Conts (note 2 of 5.10: the
connection "is made through the source/drain silicide"), the arrow from the source's
last Cont to the tie's far end; `d`/`d1` as a tie standing alone with its own Cont.

Layout `LU.c.h1` (in an NWell; `LU.c1.h1` the same in the substrate): (a) a tie 1 ×
12.16 with its Cont centred; (b) 12.17, 6.005 beyond the Cont; (c) a 12.16 square with
the Cont centred; (d) a tie 24.33 long with two Conts 12.01 apart; (e) a transistor
(Activ (5, 125)-(15, 133) under pSD, gate at x = 9.5..10.5, Conts at x = 6.92 and y =
125.92, 127.92, 129.92, 131.92) with an N+ Activ (5, 120)-(9, 125) abutting below its
source, 5.92 below the lowest Cont; (f) the same at (5, 149.915)-(9, 155), 6.005; (g)
an N+ 4 × 2 with no Cont; (h) (b) again at (1000, 1000).

- gdscheck @20/7/100: LU.c 8 and LU.d 8 - (b) twice (its two ends, "(5.4975, 39)" and
  "(17.6625, 39)"), (d) "(12.165, 99)", (e) "(7, 122.5)", (f) "(7, 152.4575)", (g)
  "(7, 190)", (h) twice - every one under both ids.
- KLayout (maximal, `latchUpRules=true`): nothing.  A probe (not kept) with standalone
  ties 6.1, 6.25, 6.5 and 7 beyond the Cont: LU.d on all four, the 6.1 one as a 0.01
  sliver "(5.4,9;5.4,10;5.41,10;5.41,9)" - its `ext_enlarge_inside(Act, 6, 0.21)` grows
  by 29 steps of 0.21 = 6.09, so 6.005 is inside its tolerance; a tie with no Cont at
  all (g) has no coincident edge with the grown Cont and is dropped.
- Verdict: (e) is clean - the abutted tie is 5.92 beyond the Cont that contacts it, the
  source's; (b), (d), (g), (h) are LU.d alone (ties standing alone); (f) is LU.c alone.
  The deck's `ContOnNWellTie` is the Cont on the N+ itself, so a tie contacted through
  the abutting P+ (the figure's own picture) has no Cont within reach and fires at
  any length, and LU.c and LU.d being the same check, each fires with the other.  IHP's
  deck splits them as the manual does: LU.c reads `Abut_NWell_Tie` (N+ sharing an edge
  with P+ in the well) against the Conts in the abutting P+, LU.d the N+ whose Activ
  touches no gate.  Expected LU.d 4 + LU.c 1 (`lu_c_h1`), LU.d1 4 + LU.c1 1
  (`lu_c1_h1`).  The existing `lu_c`/`lu_c1` cases expect both ids on one tie for the
  same reason.  KLayout's 0.09 tolerance and its silence on an uncontacted tie are its
  own; (b)'s 6.005 is what the manual's 6.00 forbids.

## Notes (not findings)

- A. Seal.a reads the conductor inside the marker: a Metal1 ring 10 wide under a 3.5
  EdgeSeal is clean, a 3.5 ring under a 3.495 marker fires, and a 0.5 × 3 Metal1 bar
  lying in a marker that carries Activ only fires (`Seal.a.h1` (h), (i), (k)); IHP's
  deck (`Metal1.and(EdgeSeal)`, projection width) agrees.  A 0.5 wire running through
  a wall changes nothing inside the marker ((j), clean in both).
- B. Seal.b measures euclidian: 4.893 corner to corner fires, 4.907 does not; a corner
  4.893 from a 45° inner wall and a diamond's vertex 4.895 from a wall fire; an Activ
  outside the ring fires too (the rule does not say inside).  KLayout agrees on all.
- C. Seal.d: the enclosure is read to the Activ's own edge, not the marker's - a Cont
  ring 1.0 from an Activ 0.7 inside a 4.2 EdgeSeal fires (`Seal.d.h1` (e)); a Cont ring
  whose 45° walls are 1.294 from the frame's fires four times (`Seal.d.h2` (b)).
  KLayout agrees (its 45° Cont walls also trip its own angle rule and Seal.c).
- D. Seal.e/Seal.f count: a ring 0.005 under the width is eight walls to gdscheck and
  four pairs to KLayout; a Passiv ring's four 45° walls 0.9935 from a frame's are one
  pair to gdscheck ("(89.2975, 50.7025)-(90, 50)") and four to KLayout
  (`Seal.f.h2`).  A Passiv ring 0.995 from an EdgeSeal marker wider than its metal is
  clean in both: the rule reads the conductor.
- E. Seal.n: a break of 1.0, no Passiv, and a Passiv ring inside the seal's hole fire;
  a ring drawn as two abutting U's is unbroken.  KLayout agrees.  The engine prints
  "Virtual layer 'PassivInSeal': result polygon has 1 hole(s); holes are not
  represented" for the ring inside the hole - Pas.c's layer, built for this deck too.
- F. Slt.b reads figure 7.5's `b`: a 3 × 25 slit is "width 25.0000 µm > 20.0000 µm",
  a 45° slit 21.2 long fires, an L-shaped slit in a 15 box is clean; KLayout's `edges
  longer than 20` agrees on the bars.  It follows that the 38 and 98 long slits with
  which `Slt.c.h1` and `Slt.i.h1` cut their plates are Slt.b violations in both tools
  (the cases expect them): a plate is slotted with slits of 20 or less.
- G. Slt.c reads the exemptions as the manual lists them: Metal5 under MIM, TopMetal2
  under a pad opening, Metal1 inside IND are clean, a Passiv over 5 of a 40 plate leaves
  35 that fires.  KLayout exempts Recog and dfpad only: it fires on the Metal1 in IND
  ("(125,225;125,265;165,265;165,225)"), the manual's Slt.e2 says not to.  A 40 plate
  with a short central slit is clean in both (no 30 disc of metal is left uncut), as is
  a self-slotted bus of 6 bars.
- H. Slt.e reads the dfpad's extent: a slit across the dfpad's edge fires, one touching
  it from outside does not; KLayout agrees.
- I. Slt.f: a slit on the plate's edge, one across it and one with no metal at all fire
  in both; 0.9935 from a 45° corner fires, 1.0006 does not.
- J. Slt.i: 6.0 % exactly is clean (four 3 × 8 slits in 40 × 40), 5.996 % fires; a
  35 × 35 plate is not "bigger than 35 × 35" and a 35.005 one is; two overlapping slits
  count once; two abutting boxes are one plate; a 48 × 100 bus with one lengthwise
  slit (5.72 %) fires while its two 22.6 strips are clean of Slt.c; a pad's TopMetal2
  outside its opening is a ring 5 wide and no plate.
- K. LBE.a/b/b1/b2: a 45° strip and a diamond 99.985 across fire and 100.006 are clean
  (70.71·√2 is 99.999, one step short - the case uses 70.715); LBE.b reads the box's
  long side (a 200 × 1500.005 bar fires, an L of 1000 arms is clean, two abutting 800
  boxes are 1600); two boxes touching at a corner are two areas (LBE.b1 clean at
  150000 each, LBE.b2 twice at 15000), and their touching point is a 0 space (LBE.c)
  and a pinch ("width 0.0000 µm ... the layer pinches to a point", LBE.a) - both fair.
  KLayout merges the touching corner into one polygon (LBE.b1 on the 300000) and its
  LBE.b2 compares against 250000 (`ext_with_area([["<", 250000.0.um2]])`), so it fires
  on every LBE under that.
- L. LBE.c-f measure euclidian and read 45° walls: a corner 99.99 from a chamfer, 49.99
  and 29.99 from Passiv and Activ chamfers fire; 100.006 corner to corner is clean.
  KLayout agrees, its `ignore_non_axis_aligned_edges` notwithstanding.  LBE.d reads to
  the inner edge in both; a 45° EdgeSeal corner 149.99 from an LBE's corner fires.
- M. LBE.h: a ring, two abutting U's, a ring with an island, a keyhole polygon and an
  octagonal ring fire once each; a U and a C are open.  KLayout also flags the island
  ("(200,800;200,900;300,900;300,800)" - `interacting(holes)`), its own error.
- N. LBE.i: 20.000 % of the boundary is clean, 20.001 % fires, at every tile.
- O. LU.a/LU.b: the tie of section 4.2 needs no Cont - an uncontacted N+ (P+) 10 from
  the P+ (N+) satisfies LU.a (LU.b) in gdscheck; KLayout's maximal deck fires LU.a
  there (its `n_tie` wants drawn nSD).  An N+ under a PWell:block is in no well and not
  read (both).  The 52 bar's portion beyond 20 is reported as one marker at its
  middle, KLayout's polygon "(23,129;23,131;60,131;60,129)" is the same portion.
- P. Every rule of the four decks that the oracle could compare agreed on which shapes
  fire except where the findings say; the counts differ by the granularities above.
  The KLayout driver's LU.b, the maximal deck's LU.c/d and IHP's LBE.b2 are the places
  where IHP's deck is the weaker reading of the manual.

## Tested and found clean or correct (no need to redo)

- Seal.a: 3.5 vs 3.495 on Activ, pSD, Metal1, Metal3, TopMetal1, TopMetal2 (a frame
  with all nine at 3.5 clean), a 3.495 marker over 3.5 metal, a 3.5 marker over 10
  metal clean, a 0.5 bar in the marker, a wire through the wall clean; 45° corners
  3.5003 across clean vs 3.493, a keyhole ring, sixteen boxes, a 0.2 nick, the pcell's
  staircase corner clean; rings across x = 20/40, ending on 100, at (1000, 1000); fifty
  flat and as an array.
- Seal.b: 4.9 vs 4.895 inside and outside, corner to corner 4.893 vs 4.907, dx = 4.5
  with dy = 10 clean, against Activ + pSD (twice), a corner 4.893 from a 45° inner wall
  vs 4.907, a diamond's vertex 4.895; gaps across x = 20, from x = 100, at (1000,
  1000); fifty flat and as an array.
- Seal.c-c3: 0.16/0.19/0.42/0.9 clean vs 0.005 under on every via layer; a Cont ring
  with a 1.0 break is nobody's rule.
- Seal.d: 1.3 vs 1.295 from the outer edge, 1.295 from the inner edge (Via1), 1.3 both
  ways in a 3.5 frame clean, the Activ's edge inside the marker, 45° walls 1.301 clean
  vs 1.294; rings across x = 20, ending on 100, at (1000, 1000); fifty flat and as an
  array.
- Seal.e: 4.2 vs 4.195, a 1.0 nick, an octagonal Passiv ring 4.2 across the diagonals
  clean.
- Seal.f: 1.0 vs 0.995 to Metal1, to TopMetal2, to Activ + Metal1 + TopMetal2 (three),
  to a marker wider than its metal clean, 45° walls 1.0006 clean vs 0.9935, a chamfered
  hole corner 0.9935; gaps ending on x = 100, across 20, at (1000, 1000).
- Seal.l: beyond the boundary on Metal1, Metal2 (crossing) and Activ; Passiv beyond it
  allowed.  Seal.n: whole, broken, absent, inside, two U's.
- Slt.a: 2.8 vs 2.795 in x and y, 45° strips 2.8002 vs 2.793, two abutting halves
  clean, a 0.2 nick, across x = 20/40, at (1000, 1000), TopMetal2 and Metal3; fifty
  flat and as an array.  Slt.b: 20 vs 20.005 in x and y, 3 × 25, 20 vs 20.005 square, a
  45° slit 21.2 long, an L in a 15 box clean.
- Slt.c: 30 vs 30.005 square and bar, a through slit (18.6 strips) clean, a short
  central slit clean, a slit 1 from the wall clean, two abutting boxes once, rings of 20
  vs 30.005 walls, diamonds 29.995 vs 30.01, an L, a 45° strip, across x = 20 and 200,
  at (1000, 1000), MIM/pad/IND exemptions, a pad's 10 ring clean, a 5 Passiv strip
  leaving 35; fifty flat and as an array.
- Slt.e: inside, across the edge, touching from outside clean, under a dfpad with no
  Passiv clean.  Slt.f: 1.0 vs 0.995 one side and all round, on the edge, across it,
  no metal, 45° 1.0006 vs 0.9935, ending on x = 20, across 40, at (1000, 1000).
  Slt.g: 0.6 vs 0.595 on Metal5 and TopMetal1, Metal1 no rule, corner 0.601 vs 0.594.
  Slt.h1-h4: 0.3 vs 0.295 to Cont, Via1, Via1/Via2 (Metal2), Via4/TopVia1 (Metal5),
  1.0 vs 0.995 to TopVia1/TopVia2 (TopMetal1), TopVia2 (TopMetal2), corner 0.297 vs
  0.304, a Metal1 slit 0.1 from a Via2 no rule.
- Slt.i: 35 vs 35.005 with one slit, 6.0 vs 5.996 %, the bus, the L, the crossing slit,
  the overlapping slits, two abutting boxes, across x = 100, at (1000, 1000), a pad's
  ring clean; fifty flat and as an array.
- LBE.a: 100 vs 99.995 in x and y, 45° strips and diamonds 100.006 vs 99.985, an L, a
  0.005 sliver, a 300 × 100 bar clean; fifty flat and as an array.  LBE.b: 1500 vs
  1500.005 in x and y, an L of 1000, two abutting 800, two 800 apart clean.  LBE.b1:
  250000 vs 250002.5, two boxes sharing a wall, a ring of 270000, an L of 210000 clean,
  corner-touching separate.  LBE.b2: 30000 vs 29999, two 15000 sharing a wall clean,
  corner-touching twice, 100 × 300 clean.
- LBE.c: 100 vs 99.995, corner 99.985 vs 100.006, a 99.995 notch, a corner 99.99 from a
  chamfer, across lines, at (1000, 1000).  LBE.d: 150 vs 149.995, a 45° corner 149.99
  vs 150.005.  LBE.e: dfpad and Passiv 50 vs 49.995, corner 49.99, 45° 49.99.  LBE.f:
  30 vs 29.995, corner 29.995 vs 30.01, 45° 29.99.  LBE.h: ring, U, two U's, island,
  keyhole, C, octagon.  LBE.i: 20.000 vs 20.001 %, beyond the boundary.
- LU.a/LU.b: far edge 20 vs 20.005, far corner 19.997, a 52 bar, no tie at (1000,
  1000), an uncontacted tie counts, PWell:block is no well; fifty flat and as an
  array.  LU.c-d1: 6.0 vs 6.005, a 12.16 square clean, two Conts 12.01 apart, the
  abutted tie at 5.92 and 6.005, no Cont, at (1000, 1000).
