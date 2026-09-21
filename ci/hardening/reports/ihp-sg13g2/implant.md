<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / tgo, psd, nsdblock: hardening report

Decks `tgo`, `psd` and `nsdblock` against SG13G2 Layout Rules Rev. 0.4, section 5.7
(TGO.a-TGO.f), section 5.10 (pSD.a-pSD.n) and section 5.11 (nSDB.a-nSDB.e), with the
derivations of section 4.2 (N+/P+ Activ by drawn nSD/pSD or by default, NFET/PFET, the
ties) and the glossary of section 4.1 ("unrelated - two regions which do not touch each
other").  111 layouts (444 KB), `tests/data/ihp-sg13g2/<deck>/<rule>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/implant_hardening.rs`, each with a `#[case]` in the `tgo`, `psd` and
`nsdblock` tables of `tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles
20, 7 and 100 and through IHP's KLayout decks (`ci/hardening/oracle-ihp.sh`).
`show-deck` lists every rule of the three sections (nSDB.d, "overlap allowed", is no check;
nSDB.b as `min_space` and `min_notch`).  Section 8.1 has no DigiBnd variant for any rule
of these sections (only NW.c1/d1/e1/f1 and Cnt.c), so there was nothing to draw there.

Reading the numbers below: gdscheck reports one marker per wall for `min_width`, one per
pair for `min_space`, one per region for `min_area`, and for `min_enclosure` one per
enclosed shape lying whole inside the enclosing one (an Activ 0.265 short on four sides
is one marker) but one per short wall of a shape that crosses out of it (a gate 0.295
short on both sides is two).  KLayout's column adds the driver's tables and the maximal
deck, so most rules count twice there; TGO.f, pSD.a and nSDB.a run in the driver only
(once), pSD.c1 in the driver only, and its hierarchical run counts an array cell once.
KLayout cuts a corner-to-corner pair into two edge pairs and a chamfer into two.  No
count moved with the tile size except in finding 9; every flat/array pair agreed in
both tools.

Test status on the engine as of this report: 12 of the 111 cases fail, all on the
findings below (`psd_d_h2` at tile 7 only); the other 99 pass, as do the 25 older cases
of the three tables.

## Findings

### 1. TGO.c reads the oxide clipped to the Activ: an Activ ending inside the oxide fires at S/D under 0.34 (false positive)

Manual: "TGO.c  Min. ThickGateOx extension over GatPoly over Activ  0.34".  Figure 5.7
draws `c` from the gate's side to the oxide's edge where that edge crosses the Activ (the
figure's Activ runs out of the oxide); an Activ that ends inside the oxide has no oxide
edge over it, only TGO.a's 0.27 beyond its end.  The deck encloses GatPoly in
`ThickGateOxOverActiv`, so the Activ's own end counts as the oxide's edge.

Layout `TGO.c.h1`: at (2, 6) a transistor whose Activ 2-3.12 ends 0.335 past the 0.45
gate on either side, under an oxide 1.73-3.39 (0.605 past the gate); at (6, 6) a gate
ending 0.335 inside its Activ's top (Activ 6-7.13 × 6-7, gate to 6.665, oxide to 7.27).
Beside them, at y = 2, the figure's device: an Activ crossing both oxide edges with the
edge 0.335 from the gate (x = 6.34, 10.34, 14.34) and at 0.34 (2.34).

- gdscheck @20/7/100: TGO.c 7 - the two crossing-edge devices and the double one (4:
  "(6.34, 3)-(6.34, 2)", "(10.79, 2)-(10.79, 3)", "(14.34, ...)", "(14.79, ...)") plus
  "(2.335, 7)-(2.335, 6)", "(2.785, 6)-(2.785, 7)" and "(6.79, 6.665)-(6.34, 6.665)".
- KLayout: TGO.c 8 (4 per deck) - the four crossing-edge devices ("polygon:
  (6.005,2;6.005,3;6.34,3;6.34,2)", "(10.79,2;...;11.125,2)", "(14.005,...)",
  "(14.79,...)") and neither of the two at y = 6: its `Gate.ext_enclosed(ThickGateOx,
  0.34)` measures to the oxide's edge, and its `.ext_and(Activ)` keeps a pair only where
  the oxide's edge lies over the Activ (note C).
- Verdict: the oxide extends 0.605 past those gates; by the rule's words and the figure
  they are clean.  Expected TGO.c × 5 and TGO.d × 1 (`tgo_c_h1`, with finding 2).  IHP's
  IO cells satisfy both readings (their HV S/D are long), so they do not decide it.

The same clipping shows the other way in `TGO.c.h2`: a 45° oxide edge crossing the Activ
(Activ 1-5 × 2-3, gate 2-2.45, the edge from (2.655, 1.73) to (4.195, 3.27)) passes 0.336
from the gate's corner (2.45, 2), the perpendicular's foot (2.69, 1.76) below the Activ,
0.475 along the Activ's bottom edge.  gdscheck: nothing (the oxide clipped to the Activ
is 0.475 away); KLayout: nothing under TGO.c (projection, and the pair's polygon lies
below the Activ), a TGO.a instead (note J).  Under the settled closest-approach reading
it fires; expected 2 (`tgo_c_h2`), the union device beside it ("(8.45,2;...;8.785,2)")
being reported by both.

### 2. A gate the oxide's edge cuts through is neither TGO.c nor TGO.d (false negative, KLayout too)

Manual: TGO.c as above; "TGO.d  Min. space between ThickGateOx and GatPoly over Activ
outside thick gate oxide region  0.34".  A gate half under the oxide has the oxide
extending 0 over its inside part, and its outside part 0 from the oxide.

Layout `TGO.c.h1` at (10, 6): Activ 10-12 × 6-7, gate 10.775-11.225, oxide 9.73-11.0.

- gdscheck @20/7/100: nothing for it under either rule.
- KLayout: nothing; its TGO.c finds only the crossing gate's width-direction pairs, which
  its `and(Activ)` removes (note C), and its TGO.d takes `Gate.ext_outside(ThickGateOx)`,
  which the crossing gate is not.
- Verdict: fires under both.  In `tgo_c_h1`.

### 3. A gate abutting the oxide is no TGO.d, an Activ abutting it no TGO.b (false negative)

Manual: TGO.b "Min. space between ThickGateOx and Activ outside thick gate oxide region
0.27", TGO.d as above; the glossary's "abut - two edges of two different layers touching
each other".  A shape whose edge lies on the oxide's edge is 0 away.

Layouts `TGO.d.h1` at (10, 6): a transistor whose Activ and gate start on the right edge
of the oxide 10-11 × 6-7; `TGO.b.h1` at (9, 8): Activ 9-10 × 8-9 on the right edge of the
oxide 7-9 × 8-10.

- gdscheck @20/7/100: nothing for either; the 0.335/0.265 gaps beside them fire.
- KLayout: TGO.d on the gate ("polygon: (10,6;10,7;11,7;11,6)" - its `TGO_d_sep` adds
  the oxide's coincident edges on purpose); nothing on the Activ, whose `Act_out` is
  `Activ.ext_not(Activ.ext_interacting(ThickGateOx))`, and a touching Activ interacts.
- Verdict: both fire - the class of the gatpoly report's findings 4-5 and the cont
  report's finding 4 (`abutting: report`).  Expected TGO.d × 6 (`tgo_d_h1`, the Activ's
  own TGO.b set aside) and TGO.b × 7 (`tgo_b_h1`).  The Activ case is the weaker one:
  figure 5.7 draws an Activ crossing the oxide's edge as legal, and an abutting Activ is
  its limit; the letter still says outside and 0 apart.

### 4. TGO.e has no notch half (false negative)

Manual: "TGO.e  Min. ThickGateOx space (merge if less than this value)  0.86".  A slot
into an oxide is the oxide's space to itself, and "merge if less" reads on it as on two
shapes.

Layout `TGO.e.h2`: a U with a 0.855 slot (4-4.855 × 4-5), a comb with two such slots
(16-16.855 and 18.855-19.71) and a 0.855 slot into a plate (24.783-25.638); a U at 0.86.

- gdscheck @20/7/100: nothing.
- KLayout: TGO.e on all four ("polygon: (4,4;4,5;4.855,5;4.855,4)" ...), its `ext_space`
  reading notches as every KLayout space check does.
- Verdict: four fire; the U at 0.86 is clean.  Expected 4 (`tgo_e_h2`).  The nwell report's
  finding 1 (NW.b) is the same class; nSDB.b carries a `min_notch` entry and passes the
  same layout (`nsdb_b_h2`).

### 5. pSD.b has no notch half (false negative)

Manual: "pSD.b  Min. pSD space or notch (Note 1)  0.31".

Layout `pSD.b.h2`: the same U, comb and slotted plate at 0.305, a U at 0.31.

- gdscheck @20/7/100: nothing.
- KLayout: pSD.b on all four (8 over its two decks).
- Verdict: four fire.  Expected 4 (`psd_b_h2`).

### 6. pSD.d and pSD.d1 take an Activ under nSD:block for N+Activ (false positive)

Manual: "pSD.d  Min. pSD space to unrelated N+Activ in PWell  0.18", "pSD.d1  Min. pSD
space to N+Activ in NWell  0.03"; section 4.2: N+Activ = Activ AND nSD, nSD = NOT (pSD
OR nSD:block) OR nSD:drawing.  An Activ under nSD:block is undoped.  The deck's
`NActiv` is Activ less pSD, while its own `NActFull` (Cnt.g1, the NFET gate) subtracts
the block.

Layouts `pSD.d.h1` at (15.175, 6): Activ 15.175-16.175 × 6-7 under nSD:block
15.1-16.5 × 5.8-7.2, 0.175 from pSD 14-15 × 6-7; `pSD.d1.h1` at (7.025, 6): the same in a
well, 0.025 from a pSD.

- gdscheck @20/7/100: pSD.d "space 0.1750 ... at (15, 6)-(15.175, 6)"; pSD.d1 "space
  0.0250 ... at (7, 6)-(7.025, 6)".
- KLayout: nothing for either (its `NAct = Activ.and(nSD.or(Activ.not(nSD_block.or(
  pSD))))`); the block's own nSDB.c fires, as it should.
- Verdict: no N+Activ, nothing.  Expected pSD.d × 5 (`psd_d_h1`) and pSD.d1 × 3
  (`psd_d1_h1`), with findings 7 and 8.

### 7. An N+Activ under PWell:block is in PWell to gdscheck and in NWell to KLayout (both tools; the letter says neither)

Manual: pSD.d/d1 as above; section 4.2: PWell = NOT (NWell OR PWell:block) OR
PWell:drawing (a layer "reserved for internal use").  An N+Activ under a PWell:block and
no NWell is in neither well.

Layouts `pSD.d.h1` at (3.175, 10): Activ 3.175-4.175 × 10-11 under PWell:block
3.1-4.5 × 9.8-11.2, 0.175 from pSD 2-3 × 10-11; `pSD.d1.h1` at (11.525, 6): the same
0.025 from a pSD, under PWell:block 10-13 × 5.5-7.5 outside the well.

- gdscheck @20/7/100: pSD.d at both ("(3, 10)-(3.175, 10)", "(11.5, 6)-(11.525, 6)"), no
  pSD.d1 (`NActivInPWell` = N+Activ less NWell; `NActivInNWell` = N+Activ and NWell).
- KLayout: pSD.d1 at (11.525, 6) ("(11.5,7;11.5,6)/(11.525,6;11.525,7)"), no pSD.d at
  either (its `X1 = NWell.or(PWell_block)` stands in for the well in `NAct_NWell`, and
  `NAct_PWell` is the rest).
- Verdict: by section 4.2 the Activ is in no PWell, and nothing makes it NWell; the cases
  expect nothing for both (`psd_d_h1`, `psd_d1_h1`).  Debatable: KLayout's reading treats
  the blocked substrate like a well (0.03), gdscheck's like PWell (0.18); the PDK owner
  may prefer either to the letter.  Its pwellblock deck (PWB.e/f) is where the block's
  own spacing lives.

### 8. pSD.d fires on an N+Activ touching the pSD at a corner (both tools; the glossary says related)

Manual: "unrelated N+Activ"; "unrelated - two regions which do not touch each other".
Figure 5.10's "e not required" shows a pSD abutting an N+Activ's edge as legal.

Layout `pSD.d.h1` at (11, 7): Activ 11-12 × 7-8 whose corner touches the corner of pSD
10-11 × 6-7; at (7, 6) an Activ abutting a pSD along an edge.

- gdscheck @20/7/100: pSD.d "space 0.0000 ... at (11, 7)-(11, 7)"; nothing for the edge.
- KLayout: pSD.d at the corner (four edge pairs round (11, 7)); nothing for the edge
  (`consider_intersecting_edges: false`).
- Verdict: a corner touch is a touch; the two are related and exempt, like the abutting
  pair.  Pedantic - the "space 0.0000" is honest geometry and the corner touch is no tie -
  but the letter is the letter.  In `psd_d_h1`.

### 9. pSD.d and an L-shaped pSD abutting an N+Activ: the count depends on the tile (TILE-DEPENDENT)

Manual: as in finding 8.

Layout `pSD.d.h2` at (14, 2): Activ 14-15 × 2-3; a pSD in two boxes, 15-16 × 1.5-3.5
abutting the Activ's right edge and 14-16 × 3.175-4.175 lying 0.175 above its top edge,
one L after merging.  The Activ's left edge is on x = 14, a tile line at 7.

- gdscheck @20: pSD.d 2 (the well-crossing Activ at (5.5, 2) and the tie's N+ part at
  (8.825, 2)); @7: 3, the third "space 0.1750 ... at (14, 3.175)-(14, 3)"; @100: 2.
- KLayout: 3, the L's upper arm too ("(15,3.175;14,3.175)/(14,3;15,3)").
- Verdict: whatever the reading, it must not depend on the tile.  By the glossary the L
  touches the Activ and is related: expected 2 (`psd_d_h2`).  The engine seems to skip a
  pair that touches somewhere, and at tile 7 the cut at x = 14 separates the touching
  piece from the 0.175 gap.  KLayout reads pair by pair and reports the gap; if the PDK
  owner prefers that, the case becomes 3 and finding 8's corner goes with it.

### 10. pSD.g fires on a standalone tap (both tools; pedantic)

Manual: "pSD.g  Min. N+Activ or P+Activ area (µm²) when forming abutted tie  0.09"; note
2: "These rules are for abutted ties".

Layout `pSD.g.h1`: a bare 0.2 × 0.3 Activ at (10, 6) in the well (an N+ tap, 0.06) and a
0.2 × 0.3 Activ at (10, 2) under pSD 9.8-10.4 × 1.8-2.5 (a P+ tap); no tie abuts either.

- gdscheck @20/7/100: pSD.g on both ("area 0.0600 ... NTapNoGateOutSRAM at (10.1,
  6.15)", "... PTapNoGateOutSRAM at (10.1, 2.15)"), with the four abutted ones.
- KLayout: the same six (12 over its decks): `NAct_NWell_not_Gate ... with_area < 0.09`
  reads every N+Activ in a well.
- Verdict: no abutted tie, no pSD.g; Act.d (0.122) reports the taps anyway, so nothing is
  lost by either reading.  Expected 4 (`psd_g_h1`).  The old `pSD.g` fixture expects the
  tools' reading on two standalone taps.

### 11. pSD.e reads the P+ part's narrow dimension, not the overlap (both tools; debatable)

Manual: "pSD.e  Min. pSD overlap of Activ at one position when forming abutted substrate
tie  0.30"; figure 5.10 draws `e` across the P+ part from the abutment line.

Layout `pSD.g.h1` at (7.2, 2): an Activ 6-7.5 × 2-2.295 whose right 0.30 lies under a
pSD; the P+ part is 0.30 deep and 0.295 tall (0.0885, a pSD.g).  The same at (19.7, 2).

- gdscheck @20/7/100: pSD.e "TieTooNarrow present at (7.35, 2.1475)" and at (19.7,
  2.1475) - the P+ part opened by 0.149 leaves nothing.
- KLayout: pSD.e on both (its `width(0.3, projection)` on the P+ part reads the 0.295).
- Verdict: the overlap is 0.30 - the pSD reaches 0.30 into the Activ - and pSD.g is the
  rule for the part's other dimension.  Expected pSD.g only (`psd_g_h1`).  Debatable: a
  0.295-tall P+ part is a bad tie either way, and both tools read "overlap" as the part's
  width.  The old `pSD.e` fixture's cases are unaffected (their parts are 0.5 and 1.0
  tall).

### 12. pSD.l measures the hole, not the enclosed empty area (false negative)

Manual: "pSD.l  Min. pSD enclosed area (µm²)  0.25"; figure 5.10 draws the right-hand
`l` in a hole that holds an island, pointing at the empty area between.

Layout `pSD.l.h1` at (8, 2): a ring 8-9.7 × 2-3.7 with a 0.7 × 0.7 hole (0.49) holding a
0.5 × 0.5 island (0.25), so 0.24 is empty (the island's 0.1 gaps are pSD.b, set aside).

- gdscheck @20/7/100: nothing for it; the other five holes fire ("hole area 0.2475 ...").
- KLayout: pSD.l on it ("polygon: (8.5,2.5;...;9.2,2.5/8.6,2.6;...)", the hole less the
  island: `subst_tie_hole.ext_not(pSD).ext_with_area(< 0.25)`).
- Verdict: fires.  Expected 6 (`psd_l_h1`).  The activ report's finding 3 (Act.e) is the
  same class and was fixed by a derived layer, `ActivHoleEmpty`; pSD.l still runs
  `min_area` with `scope: hole` on pSD.  As there, a legal-spacing island cannot bring a
  hole under 0.25 (0.31 of gaps round a 0.31 island is 0.77 empty), so this only matters
  together with pSD.b.

### 13. nSDB.c skips a pSD that overlaps the block elsewhere (false negative, KLayout too)

Manual: "nSDB.c  Min. nSD:block space to pSD  0.31", "nSDB.d  Overlap of nSD:block and
pSD is allowed".  The final nSD is NOT (pSD OR nSD:block): a gap of 0.2 between a block
and a pSD is a 0.2 sliver of nSD, whether or not the same pSD overlaps the block further
along.

Layout `nSDB.c.h1` at (10, 6): a U-shaped block (arms 10-10.5 and 11-11.5 × 6-8, base
10-11.5 × 6-6.5) and a pSD 10.2-10.8 × 7-7.8 overlapping the left arm and 0.2 from the
right one.

- gdscheck @20/7/100: nothing for it; the five plain gaps fire ("space 0.3050 ...").
- KLayout: nothing either: its `pSD.ext_interacting(nSD_block, inverted: true)` drops
  every pSD that touches or overlaps a block before measuring.
- Verdict: fires.  Expected 6 (`nsdb_c_h1`).  An abutting pSD and one lying over the
  block, drawn beside it, are rightly clean in both tools (nSDB.d): the space rule needs a
  gap, and the overlap exemption is per gap, not per shape.

## Notes that are not findings

- A. KLayout's `ext_enclosed` is projection, so it misses every 45° enclosure case here:
  the chamfer 0.265 from an Activ's corner, the diamond Activ in a square and the square
  in a diamond of `TGO.a.h2` (gdscheck 3, KLayout 0), and the three of `pSD.c.h3`.  The
  settled closest-approach reading; the cases expect them.  Its space checks are
  euclidian and agree with gdscheck on every 45° space layout (TGO.b.h2, TGO.e.h3,
  pSD.d.h3, nSDB.c.h2, the h3 of every space suite).
- B. KLayout's pSD.c reports a P+ edge flush with the pSD's: `pSD.c.h1` at (15, 2)
  ("(15,3;15,2)/(15,3;15,2)") and every NWell tie of `pSD.f.h1`/`h2`/`h3` (28 and 200
  there, the pSD's top edge flush with the P+ body beside the tab).  Figure 5.10 draws
  that flush edge as legal ("f not required"); the deck's `skip_coincident` reads it so.
- C. KLayout's TGO.c is `Gate.ext_enclosed(ThickGateOx, 0.34, include_min_angle: false,
  polygon_output: true).ext_and(Activ)`: the pair polygon between the gate's edge and the
  oxide's is kept only where it lies over the Activ, which drops the gate's width
  direction (the oxide 0.27 past the Activ) and every device whose Activ ends inside the
  oxide, and keeps the figure's crossing-edge device.  A probe of the plain `enclosed` on
  `TGO.c.h1` lists the width-direction pairs ("(2.34,3;2.79,3)/(2.34,3.27;2.79,3.27)"
  ...) that the `and(Activ)` removes.  The driver has only TGO.f.  Its pSD.i1 encloses in
  `pSD.inside(ThickGateOx)`, so a pSD reaching out of the oxide is not checked:
  `pSD.i.h1` at (12, 6) (gdscheck pSD.i1, KLayout none).
- D. Figure 5.10's vertical `j` starts at the poly's end past the Activ, not at the
  Activ's edge.  `pSD.j.h1` at (10, 0.5) puts a pSD 0.295 below a poly's end and 0.475
  below the Activ: both tools silent, and IHP's own `sg13g2_inv_1` has its NFET poly end
  0.23 from the tie's pSD (Activ 0.41 away).  The gate region it is; the case expects
  nothing there.  The same figure's horizontal `i` runs from the gate's side, and the
  same inverter keeps its pSD 0.315 past the PFET's Activ in the width direction, where
  the figure's `c` (0.18) sits: pSD.i in every direction, as the deck reads it (note E).
- E. pSD.i/i1 in the gate's width direction: `pSD.i.h1` at (2, 6) (0.295 past the Activ)
  and (2, 10) (0.395 under an oxide) fire in both tools, two walls each; 0.30 and 0.40
  are clean.  Every PFET drawn here keeps 0.30/0.40 there, so the S/D-direction cases
  stand on their own.
- F. KLayout misses the 0.339 corner-to-corner TGO.d of `TGO.d.h1` at (3, 7) (its
  `polygon_output` of a corner pair), and cuts the chamfer of `pSD.c1.h1` into two pairs
  (6 to gdscheck's 5).  It reports the 0.2687 corner pair of `TGO.b.h1` and the 0.0283 one
  of `pSD.d1.h1` as two pieces each.
- G. A pSD.k corner touch (`pSD.k.h1` at (2.4, 6.4), two 0.4 boxes meeting at a point):
  gdscheck two regions (`touching: separate`, both 0.16), plus pSD.a "width 0.0000 ... the
  layer pinches to a point" and pSD.b "space 0.0000"; KLayout two regions and pSD.b (4
  edge pairs).  The case sets pSD.a/b aside.
- H. The deck exempts SRAM from pSD.g/i/i1/j/j1 (`pSDNoSRAM`, `...OutSRAM`), as KLayout's
  does; section 8.3 is "work in progress" and section 5.10 says nothing, so no SRAM
  layout was drawn.  KLayout's nSDB.e excludes the Schottky and bipolar devices
  (`nsdb_exlcDev`); not drawn either.
- I. Both tools take a pSD gate for a PFET's without a well (the gatpoly report's finding
  9, settled), and both take an N-gate in an NWell for an NFET's (`pSD.j.h1` at (10, 6),
  both fire): section 4.2's NFET has no well condition.
- J. An Activ crossing the oxide's edge (figure 5.7) is nothing in both tools under TGO.a
  and TGO.b (`TGO.a.h1` at (17, 2), `TGO.a.h3`'s ring, every `hv2` device); its outside
  part abutting the oxide is not the finding-3 Activ (it is inside, partly).  Crossing a
  45° oxide edge (`TGO.c.h2` at (1, 2)) KLayout reports a TGO.a ("(2.925,2;2.543,2)/
  (2.925,2;2.655,1.73)", the Activ's bottom edge against the slanted wall short of the
  crossing point); gdscheck nothing - a crossing edge has points at any distance from
  the edge it crosses, and the case expects nothing there.
- K. The oracle's LU.*, NW.*, Act.c, Gat.c, Cnt.*, M1.*, Sal.c, Rppd.b, Rsil.d and EXTB.c
  lines on these layouts are other decks' (transistors drawn without contacts, a
  0.16-long S/D, pSD rings, Conts without Metal1); not read.
- L. All coordinates are on the 0.005 grid; the 45° cases use a distance a step or two
  under the value where the grid forces it (0.269, 0.336, 0.173, 0.177, 0.0283, 0.293,
  0.304), each noted in the generator.

## Tested and found clean or correct (no need to redo)

- TGO.a: 0.27 vs 0.265 on one side, all sides, 0; two Activs under one oxide; chamfer
  0.265/0.275, diamond in square 0.265/0.27, square in diamond 0.265/0.27; oxide and
  Activ as unions; an Activ in a ring's hole and one crossing its inner edge; 0.265 on and
  across x = 20/21/40/42 with a 0.27 control; fifty flat/array; 300 µm; (1000, 1000).
- TGO.b: 0.27 vs 0.265 in x, y; corners 0.2687/0.2758 and (0.265, 0.3 offset); ring hole;
  300 µm bar; (1000, 1000); chamfer 0.269/0.276, diamond corners both ways, a 45° wall;
  gaps on and across x = 20/21/40/42 and y = 20; fifty flat/array.
- TGO.c (the figure's crossing-Activ devices): 0.34 vs 0.335 left, right, both; a poly
  over field; union of boxes; the oxide's edge on and across x = 20/21/40/42; fifty
  flat/array.
- TGO.d: 0.34 vs 0.335 in x, from the gate's width edge (the cap nearer), with the Activ
  0.1 from the oxide; corners 0.339/0.3465; (1000, 1000); a diamond's corner and a
  chamfer at 0.335; gaps on and across x = 20/21/40/42; fifty flat/array.
- TGO.e: 0.86 vs 0.855 in x, y; corners 0.854/0.8655 and (0.855, 0.3); a 300 µm bar;
  (1000, 1000); two 45° strips 0.855/0.866, a diamond's corner 0.855/0.865; gaps
  straddling, ending on and starting on x = 20, straddling 21, 40, 42 and y = 20; fifty
  flat/array.
- TGO.f: 0.86 vs 0.855 in x, y; the 0.005 sliver; 300 µm; diamond 0.849/0.870, 45° strip,
  chamfered box; unions (overlap, slices, ring side, a 10 × 10 grid); bars on and across
  x = 20/21/40/42 and one across x = 20; fifty flat/array.
- pSD.a, nSDB.a: the same suite at 0.31/0.305 (diamond 0.304/0.318).
- pSD.b, nSDB.b: the same space suite at 0.31/0.305 (corners 0.304/0.3154), nSDB.b's
  notches included.
- pSD.c: 0.18 vs 0.175 on one side, all sides; flush; Activ without pSD, abutted by a
  pSD, 0.5 from one; abutted NWell ties with 0.18/0.175 on the body's three sides;
  chamfer 0.173/0.18, diamond in square 0.175/0.18, square in diamond 0.173; pSD, Activ
  and well as unions; on and across x = 20/21/40/42; fifty in one well flat/array;
  300 µm; (1000, 1000).
- pSD.c1: 0.03 vs 0.025; chamfer 0.025; under PWell:block; on and across x = 20/21/40;
  fifty flat/array.
- pSD.d: 0.18 vs 0.175 in x, y; corners 0.1768/0.1838; abutting; drawn nSD; in NWell;
  (1000, 1000); a well-crossing Activ's PWell part; a tie's N+ part and a second pSD;
  chamfer 0.177, diamond corner 0.175; gaps on and across x = 20/21/40/42; fifty
  flat/array.
- pSD.d1: 0.03 vs 0.025 in x, y; corner 0.0283; abutting tab; chamfer 0.028; across
  x = 20/21/40; fifty in one well flat/array.
- pSD.e: 0.30, 0.305 vs 0.295; Activ as two boxes; the abutment on x = 20 and across 21;
  fifty flat/array.
- pSD.f: 0.30 vs 0.295; tab as two boxes; tab under drawn nSD (both tools recognise the
  default N+ tab as well); the abutment on x = 20, across 21, on 40; fifty in one well
  flat/array.
- pSD.g: 0.09 vs 0.0885 on a P+ part and on an N+ tab; on x = 20 and across 21; fifty N+
  tabs in one well flat/array.
- pSD.i/i1: 0.30 vs 0.295 left, right, both, width; 0.40 vs 0.395 left, right (pSD out of
  the oxide), width; chamfer 0.293; pSD and Activ as unions; on and across
  x = 20/21/40/42; fifty in one well flat/array.
- pSD.j/j1: 0.30 vs 0.295 from the gate's side (a pSD abutting the Activ's end), 0.295
  under the Activ over the cap and with the cap 0.18; nSD:block Activ (no NFET); N-gate in
  NWell; 0.40 vs 0.395; a 45° wall 0.293 and a diamond's corner 0.295; on and across
  x = 20/21/40/42; fifty flat/array.
- pSD.k: 0.25 vs 0.2475; diamonds 0.245/0.2592; union of two boxes (0.24); abutting boxes
  adding to 0.25; across x = 20/21/40 (one region each); corner touch (two); (1000,
  1000); a 0.2 sliver; fifty flat/array.
- pSD.l: 0.25 vs 0.2475; an L-shaped hole of 0.24; across x = 20/21; (1000, 1000); fifty
  flat/array.
- pSD.m, pSD.n: 0.18 vs 0.175 on an Rsil's poly and an Rppd's body (the resistor deck
  owns the recognition; one layout each).
- nSDB.c: 0.31 vs 0.305 in x, y; corner 0.304; abutting and overlapping pSD (nSDB.d); a
  pSD in a ring's hole; (1000, 1000); chamfer 0.304, diamond corner 0.305; gaps on and
  across x = 20/21/40/42; fifty flat/array.
- nSDB.e: inside, half over, 0.005 over; abutting, corner touch, in a ring's hole; half
  over an edge on x = 20; in a block across 21; (1000, 1000); a 0.16 × 0.5 bar; fifty
  flat/array.
- Not tested: pSD.m/n beyond the bound (the resistor deck's report); SRAM and the
  Schottky/bipolar exemptions (note H); a P+Activ crossing the well's edge under pSD.c
  (a NW.c device); PWell:drawing (reserved).
