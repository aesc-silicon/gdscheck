<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / nmosi, pin, antenna: hardening report

Decks `nmosi`, `pin` and `antenna` against SG13G2 Layout Rules Rev. 0.4, sections 6.5
(nmosi and nmosiHV, nmosi.b-nmosi.g, with section 4.2's Iso-PWell-Activ), 7.4 (Pin.a-Pin.h)
and 7.1 (Ant.a-Ant.i).  51 layouts, `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`,
drawn by `gen/ihp_sg13g2/{nmosi,pin,antenna}.rs`, each with a `#[case]` in `test_nmosi`,
`test_pin` or `test_antenna` of `tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck
at tiles 20, 7 and 100 and through IHP's KLayout decks: the nmosi and pin layouts with
`hardening/oracle-ihp.sh` (the driver's tables plus the maximal deck; the nmosi rules
live in the maximal deck only, which the driver runs as well, so they count twice there),
the antenna layouts with the same container running `rule_decks/antenna.drc` in deep
mode, as the driver does with `--antenna` (the oracle script does not pass it).  No
count moved with the tile size in any layout, every flat/array pair agreed, and no
layout crashed or printed an error.

`show-deck` lists every rule of the three sections: nmosi.b, c, d, f, g (nmosi.e is
struck from Rev. 0.4); Pin.a, b, e, f (one per Metal2-5), g, h; Ant.a-Ant.i.  The
nmosi deck reads the drawn nBuLay, not section 4.2's derivation; that makes no
difference to this deck, since the generated nBuLay lies inside its NWell (it cannot be
under an Iso-PWell-Activ, and NWell AND generated nBuLay is never narrower than 1.0).

Reading the counts below: gdscheck's `min_enclosure` and `min_space` report one marker
per pair of shapes for axis-aligned walls (a box short on all four sides is one marker,
two Activs in one hole two) and one per 45° wall for the enclosure (the diamond nBuLay
of `nmosi.b.h2` is four, the diamond hole of `nmosi.c.h2` one); `min_width` one per
wall (a narrow leg is two); the forbidden nmosi.g and Pin rules one per region; the
antenna ratios one per gate (two gates on one violating net are two markers, as in
KLayout).  KLayout's nmosi counts are edge pairs
doubled by the two runs; its Pin.f is named per layer (`Pin.f_M2` ...); its cumulative
Ant.b/d/e/f are named per level (`Ant.b_Metal1` ...) and a net over the limit at one
level is reported at every level above; its Ant.a and Ant.c report the gate and the
antenna polygon, two per net.

Test status on the engine as of this report: 13 of the 51 cases fail, all on the
findings below (nmosi: 5 of 32; pin: 0 of 6; antenna: 8 of 13).

## Resolution (2026-09-21)

Fixed, engine: 1 (a ratio at the value meets the maximum, as every rule of the manual
is met at its value: the six ratio rules read `>` alike, a hair past the value since
20.0 / 0.1 in floating point is not always 200), 2 (a diode of the size the diode rule
asks for protects - `diode_area`, 0.16 by default, read `>=`), 9 (`over` reads just
outside the inner wall, where the margin lies, and a closest approach at a corner just
past the corner the way it was read; a touch at a corner has no margin to lie anywhere
and the wall along the touching boundary is read on its own - so an nSD:block flush
with the Activ wants no SalBlock past it there).

Fixed, deck: 5 (a second Ant.i entry on the p-diode under drawn PWell, section 4.2's
PWell wherever it is drawn), 7 and 8 (nmosi.c reads `NWellNmosi` - the ring, and an
NWell island lying in a ring's hole, inside the closed ring where the section's rules
apply; a plain NWell with no ring stays untested - with `abutting: report`: the Activ
running into the ring is 0 from it), 9 (nmosi.g is a `min_enclosure` of the nSD:block
by the SalBlock, 0.15, `metric: euclidian`, read `over: Activ`: the chamfer 0.1556 from
the block's corner is clean, 0.1414 fires, and a block with no SalBlock at all is
enclosed by nothing).

Kept: 10 (the block is read whole, as IHP's deck reads it: the 0.5 tab outside the
Activ fires, 2 walls).

Open, the owner's call (the cases carry the engine's reading, which is IHP's): 3 (a
diode anywhere on the final net protects every level; the per-level reading of figure
7.1 would count it from the level it joins at), 4 (Ant.g carries the table's 0.16
alone; note 4's PDarea = 0.02 x Vn_area / gate area is in neither tool), 6 (section
6.5's "only tested inside a closed ring of NWell AND nBuLay" is not implemented, nor by
IHP's deck: the patterns on a bare nBuLay and in a C fire as they would in a ring).

Decided (2026-09-21, gdscheck owner): 3 - the diode is read level by level, on the net as it
is at that level: `Ant.b.h6` fires at Metal1 ("no diode at Metal1"); 4 - the table's
0.16 stays, as KLayout has it.  6 - implemented: the Iso-PWell-Activ lies in a hole of
the NWell-and-nBuLay rings, nmosi.d reads the ring whose hole holds one;
`nmosi.ring.h1` and `nmosi.d.h6` read nothing, the older fixtures sit in rings now.

Decided cases: `nmosi_b_h7` + nmosi.c (the crossing Activ, finding 8), `nmosi_f_h6` 4,
`nmosi_d_h6` 18 and `nmosi_ring_h1` 2/6/4/4 (finding 6), `ant_b_h6` and `ant_g_h2`
clean (findings 3 and 4).

## Findings

### 1. The antenna ratios fire at exactly the value (false positive), and not all of them

Manual: "Ant.b  Max. ratio of cumulative metal area ... 200.00", "Ant.c  Max. ratio of
Cont area ... 20.00", "Ant.d ... 20.00", "Ant.e ... 20000.00".  A maximum of 200 is met
by 200, as every other rule of the manual is met at its value.

Layouts: `Ant.b.h1`, gate (8, 2) under 4.0 × 5.0 = 20.0 µm² of Metal1 on a 0.1 µm² gate;
`Ant.b.h5`, gate (14, 2) under Metal1 5 + Metal2 8 + Metal3 7 = 20.0; `Ant.b.h4`, gate
(30, 2) with a 0.25 diode under 40 × 50 = 2000 µm²; `Ant.d.h1`, gate (2, 2) with a
2.0 × 1.0 Via1, and the two gates (2, 28), (5, 28) on one net with a 4.0 × 1.0 Via1
(4.0 / 0.2); `Ant.ac.h1`, gate (2, 20) with a 1.0 × 1.99 Cont plus its 0.01 poly Cont,
2.0 in all.  Beside each, one grid step past (200.2, 20005, 20.1, 20.05) and one short.

- gdscheck @20/7/100: Ant.b "cumulative antenna ratio 200.0 ≥ 200 at (8.5, 2.25)" and
  "at (14.5, 2.25)"; Ant.e "20000.0 ≥ 20000 at (30.5, 2.25)"; Ant.d "20.0 ≥ 20 at
  (2.5, 2.25)", "(2.5, 28.25)", "(5.5, 28.25)"; Ant.c "20.0 ≥ 20 at (2.5, 20.25)".  Yet
  Ant.a at exactly 200.0 (`Ant.ac.h1`, gate (2, 2) with 0.12 µm² of field poly and a
  4.0 × 4.97 pad, 20.0 in all) is clean, and a pad of 4.0 × 4.971 (20.004, printed as
  "200.0") fires.
- KLayout: its cumulative rules are written `>=` (`selected_if("... value('m1_ratio') >=
  200")`) and report all of the above (`Ant.b_Metal1` with `m1_ratio 200` on the gate
  (8.4, 2)-(8.6, 2.5), `Ant.d_Via1` on (2.4, 2), (2.4, 28), (5.4, 28), `Ant.e_Metal1`
  with 20000); its `antenna_check` for Ant.a and Ant.c is strict and reports neither the
  200.0 nor the 20.0 (Ant.c has two nets, 20.05 and 25.1).
- Verdict: a ratio at the value is clean.  Expected Ant.b × 1 (`ant_b_h1`), Ant.b × 2
  (`ant_b_h5`), Ant.e × 1 (`ant_b_h4`), Ant.d × 5 (`ant_d_h1`), Ant.c × 2 (`ant_ac_h1`).
  The comparison should be the same for all six ratio rules; today Ant.a differs from
  Ant.c.

### 2. A protection diode of exactly 0.16 µm² does not protect (false positive)

Manual: "Ant.g  Size of protection diode (µm²)  0.16"; "Ant.b ... (without protection
diode)", "Ant.e ... (with protection diode)".

Layout `Ant.b.h4`: three gates under 6.0 × 5.0 = 30 µm² of Metal1 (ratio 300) whose
Metal1 also covers an n-diode (Activ AND Recog:diode, no pSD, no NWell) with a Cont in
it: 0.5 × 0.5 = 0.25 µm² at x = 2, 0.4 × 0.4 = 0.16 at x = 10, 0.4 × 0.395 = 0.158 at
x = 18.

- gdscheck @20/7/100: Ant.b "300.0 ≥ 200 at (10.5, 2.25)" and "at (18.5, 2.25)"; Ant.g
  "region area 0.1580 µm² < 0.1600" on the x = 18 diode only; nothing on the 0.25.
- KLayout: the same (`has_diode` is `area(d) > 0.16`, so the 0.16 diode gives 0;
  `Ant.b_Metal1` on the gates at (10.4, 2) and (18.4, 2); Ant.g on (22.5, 2.65) only).
- Verdict: the 0.16 diode satisfies Ant.g in both tools and is therefore a protection
  diode; the net at x = 10 is an Ant.e net at 300 and clean.  Expected Ant.b × 1, Ant.e
  × 1 (the 20005 of finding 1), Ant.g × 1 (`ant_b_h4`).  The diode test should be
  "≥ 0.16", the complement of Ant.g.

### 3. The level a diode joins the net at (a reading)

Manual: figure 7.1 and its formula sum the ratio level by level, each metal over the
gate area of the net as it is at that level; "with protection diode" is not placed at
a level.

Layout `Ant.b.h6`: a gate under 30 µm² of Metal1 (300); a Via1 at (7.5, 4.0) takes the
Metal1 up to a Metal2 strip (7.4, 3.9)-(10.6, 4.3), which comes down a Via1 at (10.3,
4.0) onto a separate Metal1 pad (10, 3.5)-(11, 4.5) over a 0.25 µm² n-diode.  At the
Metal1 level the gate's net is the 30 µm² alone; the diode joins at Metal2.

- gdscheck @20/7/100: nothing.
- KLayout: nothing (`diode_presence` is evaluated once, after every `connect`, on the
  final net).
- Verdict: a reading, the gdscheck owner's call.  The Metal1 etch happens before Metal2
  exists, so at that level the antenna has no discharge path; the per-level sum of
  figure 7.1 taken to its end says the diode counts from the level it joins at, and
  the case expects Ant.b × 1 (`ant_b_h6`).  KLayout's reading (a diode anywhere on the
  final net protects every level) is the common simplification.

### 4. Note 4's diode size is not applied (a reading)

Manual: "Ant.g  Size of protection diode (µm²) (Note 4)  0.16"; "4. PDarea (µm²) = 0.02
x (Vn_area / (GatPoly over Activ)_area)"; "2. Vn_area = cumulative area Cont, Via1 to
TopVia2".

Layout `Ant.g.h2`: a gate whose Metal1 carries a 2.0 × 2.0 = 4.0 µm² Via1 (with the
0.01 poly Cont, Vn_area / gate = 40.1, under Ant.f's 500 with a diode) and a 0.25 µm²
n-diode: by note 4 the diode must be 0.02 × 40.1 = 0.80 µm².

- gdscheck @20/7/100: nothing (Ant.g is `min_area 0.16` on the gate-connected diode).
- KLayout: nothing (`with_area(0, 0.16)`).
- Verdict: I read the table's 0.16 as the floor and note 4 as the size a net with a
  large via area needs, so the case expects Ant.g × 1 (`ant_g_h2`); neither tool has
  the formula.  The gdscheck owner's call whether Ant.g carries note 4 or the 0.16 alone.

### 5. A p-diode under drawn PWell inside an NWell (marginal)

Manual: "Ant.i  dpantenna in PWell not allowed"; section 4.2: "PWell = NOT (NWell OR
PWell:block) OR PWell:drawing".

Layout `Ant.i.h2`: Activ AND pSD AND Recog:diode (2, 2)-(3, 3) in an NWell (1, 1)-(4, 4)
under a drawn PWell (1.5, 1.5)-(3.5, 3.5).

- gdscheck @20/7/100: nothing (`AntIError` subtracts NWell and PWell:block).
- KLayout: nothing (`.not(nwell_drw.join(pwell_block))`).
- Verdict: by section 4.2 the diode is in PWell and Ant.i fires; expected Ant.i × 1
  (`ant_i_h2`).  Drawn PWell inside an NWell is an odd layout and the gdscheck owner may
  decide the derivation is not worth it.

### 6. The nmosi rules are tested outside a closed ring (a reading)

Manual, section 6.5: "These rules will only be tested inside a closed ring of NWell AND
nBuLay."  And nmosi.d: "Min. NWell-nBuLay width forming an unbroken ring around any
Iso-PWell-Activ".

Layout `nmosi.ring.h1`: every rule's violating pattern with no NWell at all (the Activ
on a bare nBuLay: nmosi.b at 1.0, nmosi.f's 0.615 strip, nmosi.g's 0.145) and in a C -
the ring missing its right leg (nmosi.b 1.235, nmosi.c 0.385, nmosi.d 0.615, nmosi.f,
nmosi.g).  Layout `nmosi.d.h6`: NWell AND nBuLay 0.5 wide with no Activ anywhere, a
0.615 C round an Activ, a closed 0.615 ring round nothing, a 0.5 NWell finger crossing
the nBuLay's edge beside a proper 0.8 ring.

- gdscheck @20/7/100: `nmosi.ring.h1` nmosi.b 2, nmosi.d 6, nmosi.f 4, nmosi.g 4;
  `nmosi.d.h6` nmosi.d 18 (every finger, the C and the empty ring).
- KLayout: the same (nmosi.b 5 edge pairs per run - the bare-nBuLay Activ on four sides
  and the C; nmosi.d 9 per run in `nmosi.d.h6`).  Its `Iso_PWell_Act` is `Activ AND
  nBuLay NOT (NWell OR PWell_block)` with no ring condition, its nmosi.d is
  `NWell_nBuLay.ext_width(0.62)` on every piece.
- Verdict: a reading; the cases expect nothing (`nmosi_ring_h1`, `nmosi_d_h6`), as the
  manual's sentence says, and both tools disagree.  The condition is cheap to state and
  hard to mean: a C round an nmos leaves the PWell unisolated, so the isolation rules
  have nothing to protect, but an nBuLay 1.0 past an Activ is also simply a bad nmosi in
  the making.  The gdscheck owner's call; if the sentence is not to be implemented, the two
  cases carry the engine's counts (2/6/4/4 and 18).

### 7. An NWell island inside the ring is not read by nmosi.c (false negative, both tools)

Manual: "nmosi.c  Min. NWell space to Iso-PWell-Activ  0.39".

Layout `nmosi.c.h7`, x = 2: the Activ (2, 4)-(4, 5) in a ring 2.0 away, with a plain
NWell (4.35, 3.5)-(5.35, 5.5) inside the hole, 0.35 from the Activ's right wall (0.35
also clears NW.d's 0.31).

- gdscheck @20/7/100: nothing under nmosi.c (the rule's layer is `NWellRing`, NWell
  `with_holes`).
- KLayout: nothing (`ext_separation(NWell.with_holes, 0.39)`).
- Verdict: the island lies inside the closed ring, where the section's rules apply, and
  it is NWell 0.35 from an Iso-PWell-Activ; expected nmosi.c on it (`nmosi_c_h7`).  An
  NWell inside an isolated PWell is rare (a PMOS in the isolated tub), which is why the
  existing `nmosi.c` fixture's canary reads a hole-free NWell as out of scope; the
  manual does not.

### 8. An Activ touching the ring is not an nmosi.c (false negative)

Manual: "nmosi.c  Min. NWell space to Iso-PWell-Activ  0.39".

Layout `nmosi.c.h7`, x = 9: the Activ (9, 4)-(11.9, 5) runs 0.3 into the ring's right
leg (inner wall x = 11.6); the Iso-PWell-Activ is the part outside the NWell, whose
right edge is the NWell's wall.  The same crossing on the left in `nmosi.b.h7`, x = 2,
y = 12 (wall x = 1.6).

- gdscheck @20/7/100: nmosi.c nothing; nmosi.b "enclosure 1.0000 µm < 1.24 ... at (11.6,
  4)-(11.6, 5)" (the piece's edge on the wall is 1.0 from the nBuLay; right).
- KLayout: nmosi.c three edge pairs, "(11.6,5;11.6,4)/(11.6,3.61;11.6,5.39)" and the two
  corners; nmosi.b as gdscheck.
- Verdict: the space is 0, under 0.39; expected nmosi.c on it and nmosi.b once
  (`nmosi_c_h7`, together with finding 7 and the ptap).  This is the coincident-edge
  class of `min_space`: two layers whose shapes abut are at space 0, not at no space.

### 9. nmosi.g's grow has square corners (false positive; the settled euclidian reading)

Manual: "nmosi.g  Min. SalBlock overlap of nSD:block over Activ  0.15".

Layout `nmosi.g.h1`, y = 12: a 1.0 square nSD:block (x + 1, 12.5)-(x + 2, 13.5) in the
middle of a 3 × 2 Activ, the SalBlock 0.15 past it on every side with its top-right
corner chamfered along x + y = k: at x = 16 the chamfer passes 0.1·√2 = 0.1414 from the
block's corner (fires), at x = 24 0.11·√2 = 0.1556 (clean under the closest-approach
reading settled for enclosures).

- gdscheck @20/7/100: "NmosiGBad present at (18.1167, 13.6167)" and "at (26.1233,
  13.6233)" - both.  `NmosiGBad` grows the block by 0.145 with square corners, so the
  grown corner reaches 0.205 diagonally and the 0.1556 chamfer leaves a sliver.
- KLayout: neither (`ext_enclosed`, projection), and neither the instance at x = 16,
  y = 4 with no SalBlock at all, which gdscheck reports (right: the overlap is 0).
- Verdict: 0.1556 ≥ 0.15 is clean; expected nmosi.g × 5 (`nmosi_g_h1`).  The grow
  should be round (or the rule a `min_enclosure` with `metric: euclidian`).

### 10. An nSD:block's tab outside the Activ (a reading)

Manual: "nmosi.f  Min. nSD:block width to separate ptap in nmosi  0.62".

Layout `nmosi.f.h6`, x = 9, y = 12: a 0.8 wide nSD:block (10, 11.8)-(10.8, 14.2)
through the Activ (9, 12)-(12, 14), with a 0.5 wide tab (10, 14.2)-(10.5, 15) on the
nBuLay above the Activ.

- gdscheck @20/7/100: nmosi.f "width 0.5000 µm < 0.6200 at (10, 14.2)-(10, 15)" and
  "(10.5, 14.2)-(10.5, 15)" (`nSDBlockIso` keeps the whole shape).
- KLayout: one edge pair "(10,13.833;10,15)/(10.5,15;10.5,14.2)" (`not_outside`, the
  whole shape too).
- Verdict: a reading; the block separates the ptap where it lies over the Activ, and
  there it is 0.8; the case expects nmosi.f on the 0.005-overlap strip only
  (`nmosi_f_h6`).  Both tools read the whole shape; if that is the reading to keep, the
  case carries 4.

## Notes that are not findings

- A. A forbidden region's marker point moves with the tile while the count does not:
  `nmosi.g.h2`'s 300 µm band is "at (220, 5.1475)" at tiles 20 and 100 and "at (73.5,
  5.1475)" at 7; `nmosi.f.h2`'s bite "(3.6288, 13)" / "(3.705, 12.6667)"; `Pin.e.h3`
  "(19.995, 6.5)" / "(19.9975, 6.0)"; `Ant.h.h1` "(20.5, 8.5)" / "(20.0, 8.5)".  The
  region is assembled from tiles in a different order; a cosmetic difference.
- B. KLayout's projection reading of nmosi.b misses every 45° case: the chamfer 1.2374
  from the Activ's corner and the diamond nBuLay of `nmosi.b.h2` (0 there, 5 here),
  the cross whose re-entrant corners are 1.2304 away in `nmosi.b.h6` (2 per run there,
  6 here); the settled euclidian reading, gdscheck right.  Its nmosi.c
  (`max_angle: 180`) does read them: the tab corner at 0.3818, the 45° wall at 0.3889
  and the diamond hole of `nmosi.c.h2` agree with gdscheck, and both leave 0.396 alone.
- C. KLayout's nmosi.g misses a block with no SalBlock at all (`nmosi.g.h1`, x = 16,
  y = 4: gdscheck "NmosiGBad present at (16.825, 5)", KLayout nothing) - its
  `SalBlock_Iso_PWell_Act` is empty and `ext_enclosed` by nothing reports nothing.
  gdscheck right.
- D. nmosi.d agrees everywhere the ring is a ring: the bound (0.615 fires, 0.62 clean),
  the nBuLay cutting a 1.0 leg to 0.615, the octagonal ring's 45° legs 0.6152 apart
  (0.6223 clean), a two-box union, a bite, a hole in a leg, the tile lines, fifty flat
  and as an array, the 300 µm ring.
- E. nmosi.f agrees everywhere but finding 10: strips, stubs, an L, unions, a 45° strip
  0.6152 wide (0.6223 clean), a bite, the tile lines, arrays; a strip abutting the
  Activ's edge is not over it (clean in both), 0.005 over it fires in both.  The nmosi.f
  layouts draw no SalBlock, so nmosi.g fires on them in gdscheck (KLayout's note C);
  the cases set it aside.
- F. Pin: every layout agrees with KLayout - the bound (coincident clean, 0.005 past
  fires), unions, tiles, a ring's hole, the diamonds, the tile lines, texts on the pin
  and label types (not areas), fifty flat and as an array, every layer of the table, a
  Metal1:pin over Metal2.  No finding.
- G. The antenna per-level sum matches KLayout and figure 7.1 (`Ant.b.h3`: G1 195 + 10
  at Metal2 fires at 205 while the whole net would be 112.5; `Ant.b.h5`: 50 + 80 + 80
  fires at Metal3, 190 clean, a stack to TopMetal2 fires at 222.5), and so does the
  level of Ant.a and Ant.c: poly connects poly and Cont, not Metal1, so of two gates
  strapped by Metal1 only the one with the 25 µm² poly pad (251.2) or the 2.5 µm² Cont
  (25.1) fires, and a Cont on the Activ beside a gate is not on its net (`Ant.ac.h1`).
  One poly over two gates sums the gates (151.6, clean).
- H. Areas are the unions drawn: Metal1 as two boxes overlapping by 4.02 is 19.98
  (clean), the gate's Activ as two overlapping boxes is still 0.1 (`Ant.b.h3`); the
  0.16 diode as two abutting boxes is 0.16 (`Ant.g.h1`).  Ant.c counts the gate's own
  poly Cont; the layouts' Cont areas include it.
- I. The tile lines: wires across x = 20 and 40 (200.2 fires, 199.8 clean), a wire
  ending on x = 40, an L across x = 20 and y = 40, a 40.2 × 0.05 Via1 across x = 20 and
  40, an n-diode and a p-diode across x = 20 - the same at 20, 7 and 100.
- J. Ant.g agrees with KLayout: 0.158 through Metal1, through Metal2, a p-diode in
  NWell, an Activ under a larger marker and a marker on a larger Activ fire; 0.16, the
  two-box 0.16 and an unconnected 0.1 × 0.1 diode are clean.  Ant.h agrees: bare in
  NWell, half in, the "isolbox" text outside its nBuLay, on Metal1:label, "isolbox2"
  fire; Recog:esd, "isolbox", "ISOLBOX", nSD:block over the Activ, PWell:block with no
  NWell are clean.  Ant.i agrees: bare and half out of an NWell fire; NWell,
  PWell:block, Recog:esd, a ptap with no marker are clean.
- K. What "diode" is: both tools count any non-gate Activ on the net (`AntDiode` is
  N+ or P+ Activ without GatPoly, KLayout's `diode` the same), with or without a
  Recog:diode marker - a source/drain diffusion protects.  The layouts mark theirs.

## Tested and clean

- nmosi.b: 1.24 exactly on every side, on x = 60; chamfer and diamond 1.2445; a two-box
  union at 1.24, a 4 × 3 grid of tiles, the cross at 1.2445; the Activ under
  PWell:block.  Fires as expected: 1.235 right/top/all round, chamfer and diamond
  1.2374, unions and slices at 1.235, the cross at 1.2304, tile lines, (1000, 1000), 300
  µm, fifty flat and as an array, a ptap, an nmosiHV Activ.
- nmosi.c: 0.39 exactly; tab corner and 45° wall at 0.396; a plain NWell with no ring,
  a ring with no nBuLay, the Activ under PWell:block.  Fires: 0.385 on each side and
  all round, the tab corner 0.3818, the 45° wall and diamond hole 0.3889, the keyhole
  ring, eight boxes, a tab, a step, two Activs, the Activ as two boxes, tile lines,
  (1000, 1000), 300 µm, fifty, a ptap.
- nmosi.d: 0.62 exactly; 0.6223 octagon; a two-box union at 0.62.  Fires as in note D.
- nmosi.f: 0.62; 0.6223 45°; a union at 0.62; no nBuLay; PWell:block; abutting.
- nmosi.g: 0.15; a union at 0.15; block and SalBlock flush with the Activ; no nBuLay;
  PWell:block.  Fires: 0.145, no SalBlock, 0.145 on top inside the Activ, the 0.1414
  chamfer, a union at 0.145, tile lines, (1000, 1000), 300 µm, fifty, the band under a
  block flush with the Activ's edge.
- Pin: as note F.
- Antenna: 199.8, 200.0 (finding 1), 19.9, 20.0, 190, 20000; two gates under 30 µm²;
  the diodes of note J and finding 2; the whole of notes G-K.
