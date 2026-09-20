<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / cont: hardening report

Deck `cont` against SG13G2 Layout Rules Rev. 0.4, section 5.14 (Cnt.a-Cnt.j) and section
8.1.2 (Cnt.c inside DigiBnd).  64 layouts, `tests/data/ihp-sg13g2/cont/Cnt.*.h<n>.gds.gz`,
drawn by `gen/ihp_sg13g2/cont.rs` (`hardening`), each with a `#[case]` in the `cont` table
of `tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through IHP's KLayout decks (`ci/hardening/oracle-ihp.sh`).

Reading the numbers below: gdscheck reports four markers per off-size square for Cnt.a
(`exact_width`, one per wall), one marker per violating pair for the space rules, one
marker per under-enclosed Cont for the enclosure rules (its violating walls form one
connected run; a Cont short on two opposite walls of a long strip gets two) and one per
Cont for the `forbidden` rules.  KLayout's column in the oracle adds the driver's and the
maximal deck's reports; Cnt.a, Cnt.b, Cnt.c, Cnt.d, Cnt.e, Cnt.g1 and Cnt.g2 run in the
driver only, Cnt.b1, Cnt.f, Cnt.g, Cnt.h and Cnt.j in both, so those five count twice
there, and its hierarchical run counts an array cell once.  No count moved with the tile
size in any layout, and every flat/array pair agreed.

Test status on the engine as of this report: 10 of the 64 new cases fail, all on the
findings below; the other 54 pass.

## Findings

### 1. Cnt.c has no DigiBnd variant, and section 8.1.2 is not in the deck (false positive; rule missing)

Manual, section 8.1.2: "Cnt.c  Min. Activ enclosure of Cont inside DigiBnd  0.05".
`show-deck` lists Cnt.c at 0.07 and nothing for DigiBnd; the nwell deck names its
variants NW.c1.dig, NW.d1.dig, NW.e1.dig.

Layout `Cnt.c.h8`: under a DigiBnd plate, Conts with Activ margins 0.05 (2, 2), 0.045 on
the right (4, 2) and 0.065 (6, 2); outside DigiBnd 0.065 (12, 2); an Activ in the hole of a
DigiBnd frame with 0.065 (15, 2); the DigiBnd edge through the Activ 0.03 right of a Cont
whose margins are 0.10 (18, 2); the DigiBnd edge through the Cont, margins 0.10 (22, 2); a
0.10 × 0.10 DigiBnd inside a Cont with 0.065 (24, 2); a digital 0.045 straddling x = 40.

- gdscheck @20/7/100: Cnt.c 7: 0.0500 at (1.92, 2.08), 0.0450 at (4.08), 0.0650 at (5.92),
  0.0650 at (12.08), 0.0650 at (15.08), 0.0650 at (23.92), 0.0450 at (40.08).
- KLayout: Cnt.c at (12.08), (15.08) and four edge pairs round the Cont at 24 (it clips
  Activ by DigiBnd, so an Activ with a DigiBnd hole in it is analog Activ with a hole);
  Cnt.c.digibnd at (4.08), (40.08) and at (18.08)-(18.11) - the last a false positive of
  the same clipping: the Activ runs 0.10 past the Cont, the DigiBnd edge 0.03.
- Verdict: 0.045 fires the digital variant twice, 0.065 fires Cnt.c outside DigiBnd and
  in the frame's hole; the rest is clean (a DigiBnd overlapping the Cont makes it
  digital by the settled reading).  Expected Cnt.c.dig × 2 + Cnt.c × 2 (`cnt_c_h8`).

### 2. Cnt.b1 does not see an array whose gaps are not all alike (false negative)

Manual: "Cnt.b1  Min. Cont space in a contact array of more than 4 rows and more then 4
columns  0.20", note 1: "Cnt.b1 is only required in one direction.  The distance of the
other direction must be at least Cnt.b."

Layout `Cnt.b1.h2`, the 5 × 5 at (6, 6): columns 0.18 apart throughout, rows at y = 6.0,
6.34, 6.70, 7.04, 7.40, so the row gaps are 0.18, 0.20, 0.18, 0.20.  Neither direction is
0.20 throughout.

- gdscheck @20/7/100: nothing for it (the other four arrays of the layout - 6 × 6, 5 × 10,
  a staggered 5 × 5 and a 5 × 5 drawn as a 3- and a 2-column block - are reported).
- KLayout: Cnt.b1 twice, the polygons over rows 1-2 (y 6.0-6.5) and rows 3-4 (6.7-7.2),
  in both decks.
- Verdict: fires.  In `cnt_b1_h2` (expected 6 with finding 3).

### 3. Cnt.b1 and a 5 × 5 array missing its centre (both silent; the manual says otherwise)

Layout `Cnt.b1.h2`, the array at (2, 6): a 5 × 5 at 0.18/0.18 with the middle Cont
removed, 24 Conts in five rows and five columns.

- gdscheck: silent.  KLayout: silent (its array detection sizes the Conts into one blob
  and asks it to be 1.34 wide; the hole breaks the blob).
- Verdict: five rows and five columns of Conts 0.18 apart are "an array of more than 4
  rows and more than 4 columns" whichever site is empty; the rule's concern (a dense
  field of contacts) does not go away with one of 25.  I read it as a violation, and the
  case says so; it is the least certain finding here, and if the PDK owner reads
  "array" as "full array" the case should drop one Cnt.b1.

### 4. A touch is a space of zero: Cnt.e, Cnt.f and Cnt.g1 stay silent when the other layer abuts the Cont (false negative)

Manual: "Cnt.e  Min. Cont on GatPoly space to Activ  0.14", "Cnt.f  Min. Cont on Activ
space to GatPoly  0.11", "Cnt.g1  Min. pSD space to Cont on nSD-Activ  0.09".  A shape
whose edge lies on the Cont's edge is 0 away.

Layouts: `Cnt.e.h1` (16, 2): Activ (16.08, 1.5)-(16.58, 2.5) against a gate contact's
right edge; `Cnt.f.h1` (16, 2): GatPoly (16.08, 1.5)-(16.58, 2.5) against a diffusion
contact; `Cnt.g1.h1` (16, 2): pSD against a Cont on N+Activ; `Cnt.d.h8` (4, 2): a Cont
straddling the seam of an abutting GatPoly and Activ; `Cnt.j.h1` (8, 2) and `Cnt.j.h2`
(19.92, 11): a Cont on GatPoly abutting Activ (no overlap, so rightly no Cnt.j).

- gdscheck @20/7/100: nothing at any of them (the same layouts' 0.135/0.105/0.085 gaps
  and corner-to-corner gaps are reported).
- KLayout: Cnt.e at (16.08, 1.92)-(16.08, 2.08), Cnt.f at (16.08 ...), Cnt.g1 at (16.08
  ...), and in `Cnt.d.h8` Cnt.e and Cnt.f at x = 4.0, in `Cnt.j.h1` Cnt.e at x = 8.08 -
  the shared edge as the edge pair.
- Verdict: all fire.  This is the class the nwell report's finding 5 found for NW.d ("a
  space check has no pair at a touch").  In `cnt_e_h1` (7), `cnt_f_h1` (8), `cnt_g1_h1`
  (6, with finding 5), `cnt_d_h8`, `cnt_j_h1`, `cnt_j_h2`.

### 5. Cnt.g1 asks for drawn nSD: a Cont on plain Activ is not "on nSD-Activ" to it (false negative)

Manual, section 4.2: "nSD = NOT (pSD OR nSD:block) OR nSD:drawing", "N+Activ = Activ AND
nSD".  An Activ with nothing drawn on it is N+; that is the ordinary NMOS source/drain.
The deck's `ContOnNsdActiv` is `ContSquare ∩ Activ ∩ nSD` with nSD the drawn layer.

Layout `Cnt.g1.h1` (18, 2): a Cont on a 0.30 Activ square with no nSD drawn, pSD
(18.165, 1.5)-(18.665, 2.5) 0.085 away; beside it (22, 2) the same under nSD:block
(clean: not N+) and (26, 2) a well tie with drawn nSD in an NWell (fires).  Layout
`Cnt.g2.h1` (16, 2): the pSD edge through the middle of a Cont on Activ - the covered
half is on pSD-Activ enclosed by 0 (Cnt.g2), the bare half is on nSD-Activ 0 from pSD.

- gdscheck @20/7/100: `Cnt.g1.h1` Cnt.g1 4 (4.165, 8.14, 12.14, 26.165); nothing at 18.
  `Cnt.g2.h1`: Cnt.g2 7, no Cnt.g1.
- KLayout: Cnt.g1 at (18.08, 1.92)-(18.165, ...) as well (its nSD-Activ is `activ not
  (psd or nsd_block)`); in `Cnt.g2.h1` Cnt.g1 at (16, 1.92)-(16, 2.08) and Cnt.g2.
- Verdict: the plain-Activ Cont fires; the pSD-through-the-middle Cont is Cnt.g2 and
  Cnt.g1 (the latter also a touch, finding 4).  Expected 6 in `cnt_g1_h1`, Cnt.g2 × 7 +
  Cnt.g1 in `cnt_g2_h1`.  The matching NW.d reading (undoped Activ is N+) is already in
  the nwell deck's `NActFull`.

### 6. Two Cont squares touching at a corner (gdscheck: a bar; KLayout: nothing; the manual: Cnt.b)

Manual: "Cnt.b  Min. Cont space  0.18"; section 5.14 is about square Cont, 5.15 about
"any Cont shape not being a square".

Layout `Cnt.b.h5` (2, 2): two 0.16 squares sharing the corner (2.16, 2.16); beside them a
square 0.175 from a 0.16 × 0.40 bar (CntB.b2's, 0.22) and a 0.155 square 0.175 from a
0.16 one (Cnt.b once, Cnt.a four walls).

- gdscheck @20/7/100: Cnt.b 1 (the 0.155/0.16 pair), Cnt.a 4; the corner pair is merged
  into one non-square shape and reported by the contbar deck (CntB.a, CntB.a1, CntB.b).
- KLayout: Cnt.b 1 (the same pair), CntB.b2 for the bar; nothing at all for the corner
  pair - its merge keeps them two squares, and its `space` has no pair for edges that
  meet at a point.
- Verdict: two square Conts 0 apart are Cnt.b; the case (`cnt_b_h5`) expects two Cnt.b.
  gdscheck's reading (the union is one malformed bar) flags the layout too, just under
  other rules; KLayout lets it through.  If the PDK owner prefers the union reading the
  case should drop one Cnt.b.

## Notes that are not findings

- A. Markers.  For the enclosure rules gdscheck gives one marker per under-enclosed Cont
  when the short walls are adjacent (0.065 on all four sides: one; right and top: one)
  and one per wall when they are not (a Cont on a 0.29 strip: two); KLayout gives one per
  edge pair (four, two, two).  The set of violating Conts is the same in every layout.
  Cnt.j over two 0.05 Activ fingers (`Cnt.j.h1` (10, 2)) is two markers in both tools,
  one per overlap piece; the cases carry these counts.
- B. 45° enclosure.  `Cnt.c.h2`, `Cnt.d.h2` (a chamfer and a long 45° wall passing 0.064
  from the Cont's corner, controls at 0.071) and `Cnt.g2.h2` (a pSD chamfer at 0.085,
  control 0.092): gdscheck is clean, as the settled projection reading says.  IHP's
  driver deck runs these three rules as `enclosed(..., euclidian)`, not `ext_enclosed`,
  and reports the 0.064 chamfer (two edge pairs at (2.071, 2.08)-(2.08, 2.08) / (2.104,
  2.146)-(2.146, 2.104)), the 0.064 wall and the 0.085 pSD chamfer.  The cases follow the
  settled reading; if the PDK owner wants Cont enclosure euclidian like IHP's deck,
  `cnt_c_h2`, `cnt_d_h2` gain two markers each and `cnt_g2_h2` one.
- C. Crossing Conts.  A Cont partly outside its Activ, GatPoly or pSD is reported by
  gdscheck as the enclosure rule at 0.0000 (and Cnt.g where nothing else covers it);
  KLayout's `enclosed` does not report a crossing Cont for Cnt.c/Cnt.d (Cnt.g catches the
  wholly bare part) but does for Cnt.g2 (it clips the Cont to pSD-Activ first).  The
  manual's enclosure of such a Cont is negative; the cases expect the enclosure rule.
  The same reading gives Cnt.c for the Activ half of the Cont on the GatPoly/Activ seam
  (`Cnt.d.h8`) and for the 0.005 strip, the 0.005 corner and the two finger pieces of
  `Cnt.j.h1`; KLayout gives Cnt.c only where a piece's edge lies on the Activ edge.
- D. Coincident edges.  A Cont edge on the Activ/GatPoly/pSD/Metal1 edge is an
  enclosure of 0 in both tools (Cnt.c, Cnt.d, Cnt.g2 fire; `Cnt.g.h2` (19.92, 6.5) is
  within Activ for Cnt.g and Cnt.c at 0) and Metal1 exactly coincident with a Cont covers
  it (Cnt.h clean).
- E. Cnt.b1 counts.  gdscheck reports one marker per array ("5×5 ContSquare array
  (1.52×1.52 µm) below 0.20 µm"); KLayout one polygon per array per deck, or two where
  two separate row pairs are short (finding 2).  A 5 × 5 built as a `GdsArrayRef` of a
  one-Cont cell, a 5 × 5 drawn as a 3-column and a 2-column group and a staggered 5 × 5
  are all one array to both tools.  A 5 × 5 at 0.175 both ways is Cnt.b1 and forty Cnt.b.
- F. Contbar deck, seen in passing (not this deck's, for whoever hardens `contbar`): in
  `Cnt.a.h1` the 0.16 × 0.165 rectangle and the square missing a 0.005 corner are CntB.a
  to KLayout's maximal deck and CntB.a1 to gdscheck; in `Cnt.g1.h5` a 0.16 × 0.50 bar on
  N+Activ 0.085 from pSD is CntB.g1 to gdscheck and nothing to KLayout.
- G. KLayout's Cnt.c.digibnd on `Cnt.c.h8` (18, 2) is a false positive of its own
  (finding 1); its hierarchical run counts an array cell once (51 for 50 instances).

## Tested and found clean or correct (no need to redo)

- Cnt.a: a 0.16 square drawn as one box, four abutting quadrants, two overlapping
  boxes, eight vertices, twice on top of itself, clockwise; 0.155 and 0.165 (four walls);
  a 0.16 × 0.165 rectangle and a notched square are not squares (no Cnt.a); 0.155
  squares straddling x = 20/21/40/42, y = 20, ending on x = 20, at (1000, 1000); fifty
  flat and as `GdsArrayRef`.
- Cnt.b: 0.18 vs 0.175 in x and y; 0.177 vs 0.184 corner to corner; 0.10/0.15 (0.180
  euclidian, 0.15 in the axis) clean; a row of three (two pairs); gaps straddling,
  beginning on and ending on the tile lines; (1000, 1000); fifty flat/array; a square
  0.175 from a bar is not Cnt.b.
- Cnt.b1: 5 × 5 at 0.18 (fires), at 0.20 in either direction (clean), 4 × 5, 5 × 4, 4 × 4
  (clean), 0.195 (fires), 0.20/0.20 (clean); 6 × 6, 5 × 10, staggered, 3 + 2 columns;
  arrays straddling x = 20/21/40/42/14, the corner (20, 20), (1000, 1000); fifty flat and
  as an array of arrays; a 5 × 5 as a `GdsArrayRef` of one Cont; 0.175 → Cnt.b1 + Cnt.b.
- Cnt.c: 0.07 vs 0.065 per side, all sides, corner, 0.005, 0; crossing (Cnt.c + Cnt.g);
  unions (abutting, overlapping, 4 × 4 grid) clean, union at 0.065, duplicate Activ,
  ring wall; margins on/across the tile lines incl. an Activ edge on x = 20; fifty
  flat/array; 300 µm strips; (1000, 1000); 45° per the settled reading.
- Cnt.d: the same set with GatPoly; a gate contact with 0.065 poly (Cnt.d + Cnt.j); a
  Cont half on poly and half on nothing (Cnt.d + Cnt.g).
- Cnt.e: 0.14/0.135; 0.141/0.134 corner to corner; 0.144 (0.12 in the axis) clean;
  0.134 corner to a 45° wall fires, 0.141 clean; the gate's own Activ at 0.22/0.135; Activ
  on both sides; gaps on/across the tile lines four ways at x = 20; fifty flat/array; a
  300 µm Activ; (1000, 1000); a bar on poly is CntB.e's.
- Cnt.f: 0.11/0.105; 0.113/0.106 corner to corner; 0.114 (0.09 in the axis) clean;
  0.106/0.113 to a 45° wall; a gate at 0.105, two gates, a field poly 0.035 outside the
  Activ; tile lines; fifty flat/array; 300 µm poly; (1000, 1000); a bar is CntB.f's.
- Cnt.g: bare, 0.005 out, half out, in a ring's hole, abutting from outside, (1000,
  1000); within Activ, GatPoly, abutting boxes, four quadrants; a gate contact over
  Activ is Cnt.j not Cnt.g; tile lines incl. an Activ ending on x = 20 under the Cont;
  fifty flat/array.
- Cnt.g1: 0.09/0.085; 0.092/0.085 corner to corner; 0.094 (0.08 in the axis) clean;
  0.085/0.092 to a 45° wall; nSD:block is not N+; a well tie is; tile lines; fifty
  flat/array; 300 µm pSD; (1000, 1000); a bar is CntB.g1's.
- Cnt.g2: 0.09/0.085 per side, all sides, corner, 0.005, 0, pSD ending on the Activ
  edge (0.07); pSD as abutting halves and overlapping boxes, union at 0.085, duplicate;
  tile lines; fifty flat/array; 300 µm pSD strip; (1000, 1000); chamfers per the
  settled reading.
- Cnt.h: no Metal1, a 0.005 sliver, half covered, in a ring's hole, abutting from
  outside, (1000, 1000); covered with margins, coincident, abutting halves, overlapping
  boxes, two boxes meeting on x = 20; tile lines; fifty flat/array.
- Cnt.j: in a gate, a 0.005 strip, a 0.005 × 0.005 corner, two Activ fingers, a gate over
  abutting Activ boxes, (1000, 1000); abutting is not over; tile lines incl. Activ
  beginning on x = 20 under the Cont's right half; fifty flat/array.
- Not tested: the SRAM marker (the deck excludes Cont in SRAM from Cnt.c; section 8.3 is
  "work in progress"), the SVaricap exclusion of IHP's deck (not in the manual's
  section 5.14), Cont in the seal ring (excluded by `ContNoSealring`, section 5.19's).
