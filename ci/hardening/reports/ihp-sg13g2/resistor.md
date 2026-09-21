<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / resistor: hardening report

The resistor deck against SG13G2 Layout Rules Rev. 0.4: section 6.2 (Rsil, Rsil.a-f),
6.3 (Rppd, Rppd.a-e) and 6.4 (Rhigh, Rhi.a-f), with section 4.2's derivations.  Every
rule of the three sections is in the deck.  69 layouts,
`tests/data/ihp-sg13g2/resistor/<RULE>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/resistor_hardening.rs`, each with a `#[case]` in the resistor table of
`tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through IHP's KLayout decks (`ci/hardening/oracle-ihp.sh`).

A resistor is drawn as figures 6.3-6.5 draw it: a GatPoly stripe (0.5 wide, 2.0 under
the block unless the rule needs otherwise), the block over the body - RES for Rsil,
coincident with the stripe's long edges; SalBlock for Rppd and Rhigh, 0.20 past the
stripe on both sides (Sal.c) - a 0.5 head of poly beyond the block at each end with a
0.16 Cont on it (0.12 from the RES, 0.20 from the SalBlock) under a Metal1 square,
EXTBlock round the whole poly by 0.18, and for Rppd and Rhigh pSD (and nSD) round the
whole poly by 0.18.  The width, the EXTBlock enclosure, the block length and the Cont
gap use one kit each, drawn for every kind.

Reading the numbers: gdscheck gives one `min_width` marker per wall (two per narrow body,
four per diamond), one space marker per pair, one enclosure marker per under-enclosed
side or one for adjacent sides, one forbidden-region marker per piece.  The KLayout
column of the oracle adds the driver's and the maximal deck's reports; the driver runs
every resistor rule but the width and length ones, so those count once there and the
rest twice, an edge pair per marker, and the hierarchical run counts an array cell once
(51 for 50).

Test status on the engine as of this report: 18 of the 77 new cases fail, all on the
findings below; the other 59 pass.  No count moved with the tile size except in
finding 4, and every flat/array pair agreed.

## Resolution (2026-09-21)

Fixed, engine: 4 (a selection layer's copies are rebroadcast to the halo the layer is
built at - it was rebroadcast to the halo the *running* rule had raised for it, none
when the layer was first built as a source of another (Rsil.c's), so the tile past a
poly ending on a tile line had no copy of it and Rsil.d lost the pair; the fix reaches
every selection, region-filter and holes layer).

Fixed, deck: 2 (`abutting: report` on Rsil.b, Rsil.d, Rppd.c, Rhi.d), 3 (the enclosure
rules Rsil.e, Rppd.d, Rhi.e, Rppd.b, Rhi.c read the resistor's whole poly as a
contained shape: a head running out of the layer, a layer over the body only, or no
EXTBlock at all is "not enclosed", once per shape; Rsil = RES + GatPoly, the EXTBlock
no longer part of the recognition), 5 (Rppd.b on `GPRppdExtended`, the whole poly), 7
(Rhi.b compares the resistor's whole pSD and nSD shapes: `RhighNsd` less pSD and
`RhighPsd` less nSD, so a pSD past the nSD anywhere along the resistor fires; the
half-covered Rhigh's Rhi.d artefact is gone with `SalBlockRhigh`/`SalBlockRppd` read as
the whole SalBlock of the resistor, which also settles the length rules' extra markers).

Kept, cases flipped: 6 (length and width read the same two dimensions, as both tools:
a second id, never a miss) and the bare nSD:drawing of 7 (note 1's letter against the
N+ ties every design draws with nSD; KLayout does not flag it either - OPEN).

Open: 1 (the manual's Rsil.c is the RES stopping short of the poly's *long* edge, which
wants the resistor's length direction - the heads run out of the RES at both ends by
construction, so neither an enclosure nor a protrusion tells the long side from the
ends without it; the deck keeps IHP's reading, RES outside the poly and under no Cont).

Decided (2026-09-21, PDK owner): 1 - the manual's reading.  The long side is told from
the ends without a direction: the poly outside the RES is the heads and, where the RES
stops short of a long edge, a thin strip along it, and an opening by 0.245 keeps the
full-width heads and not the strip (`RsilResInset`).  `Rsil.c.h1` reads 3, `Rsil.c.h2`
2; the older `Rsil.c` fixture draws the inset.

## Findings

### 1. Rsil.c reads "RES extension over GatPoly" backwards: a RES past the poly's edge fires, a RES short of it does not

Manual: "Rsil.c  Min. RES extension over GatPoly  0.00", figure 6.3 with `c` on the
resistor's long side where the RES and GatPoly edges coincide; figure 4.1's "Min. Layer A
extension of Layer B" is A reaching past B's edge.  A RES that stops inside the poly's
long edge extends over it by less than 0.00 and fires; a RES past the edge extends over it
by more and is clean (as Sal.c's SalBlock past the stripe is).  The heads run out of the
RES at both ends by construction, so the rule reads on the long sides.  `show-deck`:
Rsil.c is `forbidden [RsilResProtrusionNoCont]`, RES outside the resistor's poly and not
under a Cont - IHP's reading (`rsil_gatpoly.ext_enclosed(RES, 1.0) ... ext_outside(Cont)`,
which flags a RES up to 1.0 past the poly), not the manual's.

Layout `Rsil.c.h1` (0.6 bodies): RES coincident at (2, 2); RES ending 0.05 inside the top
edge at (2, 5) (RES (2, 5)-(4, 5.55)); RES 0.05 past the top edge at (2, 8); RES 1.5 past
it at (2, 11); RES 0.05 inside both edges at (2, 14).  `Rsil.c.h2`: the inset across
x = 20 and at (1000, 1000), the outset across x = 40.

- gdscheck @20/7/100: `Rsil.c.h1` 2, at (3, 8.625) and (3, 12.35) - the two outsets;
  nothing at (2, 5) or (2, 14).  `Rsil.c.h2`: 1 at (40, 2.625), the outset; nothing at
  the insets.  Also in `Rsil.a.h1` (3, 8.52): the RES over a 0.06 slot cut into the 0.55
  body is a "protrusion".
- KLayout: `Rsil.c.h1` polygon (2, 8.6)-(4, 8.65) - the 0.05 outset only; the 1.5 outset
  is past its 1.0 window; nothing at the insets.
- Verdict: the insets fire (once per side), the outsets are clean.  A RES narrower than
  its poly is also not Rsil.a (the GatPoly is 0.6 wide) and not Rsil.f (the RES is 2.0
  long); the derived `RsilAll` (GatPoly ∩ RES ∩ EXTBlock) would report both if the RES
  went under 0.5, which these layouts avoid.  In `rsil_c_h1`, `rsil_c_h2`; `rsil_a_h1`
  ignores Rsil.c.

### 2. A touch is a space of nothing: Rsil.b, Rsil.d, Rppd.c and Rhi.d stay silent when the other shape abuts (false negative)

Manual: "Rsil.b  Min. RES space to Cont  0.12", "Rsil.d  Min. pSD space to GatPoly
0.18", "Rppd.c / Rhi.d  Min. and max. SalBlock space to Cont  0.20".  A shape whose edge
lies on the other's is 0 away.  The block report's finding 1 found this class for nine
rules, which now carry `abutting: report`; these four do not.

Layouts: `Rsil.b.h1` (2, 14): a Cont (1.84, 14.17)-(2, 14.33) abutting the RES's end at
x = 2; `Rsil.d.h1` (2, 14): a 1 × 1 pSD from (4.5, 14) abutting the right head's end;
`Rppd.c.h1` and `Rhi.d.h1` (10, 2): a Cont (9.84, 2.17)-(10, 2.33) abutting the SalBlock's
end at x = 10.

- gdscheck @20/7/100: nothing at any of them; the 0.005-under gaps and the corner-to-corner
  gaps of the same layouts are reported.
- KLayout: the coincident pair for all four: Rsil.b (2, 14.05)-(2, 14.45)/(2, 14.17)-
  (2, 14.33); Rsil.d (4.5, 14.5)-(4.5, 14)/(4.5, 14)-(4.5, 14.68); Rppd.c and Rhi.d
  (10, 2.33)-(10, 2.17)/(10, 2)-(10, 2.5).
- Verdict: all four fire.  The Cont crossing the RES's end (`Rsil.b.h1` (10, 2), gap
  -0.05) shares area with it and is no pair, as the block report settled for Sal.e.  In
  `rsil_b_h1`, `rsil_d_h1`, `rppd_c_h1`, `rhi_d_h1`.

### 3. An enclosing layer the poly runs out of, or that is not there, encloses it by nothing; nothing is reported (false negative)

Manual: "Rsil.e / Rppd.d / Rhi.e  Min. EXTBlock enclosure of GatPoly  0.18", "Rppd.b
Min. pSD enclosure of GatPoly  0.18", "Rhi.c  Min. pSD and nSD enclosure of GatPoly
0.18".  Figures 6.3-6.5 draw the EXTBlock and the implants round the whole poly, heads
and Conts included.  A head sticking out of the layer is enclosed by less than 0 on that
side; a resistor with no EXTBlock at all is enclosed by nothing on every side.  The
gatpoly report's finding 3 (a gate ending inside the Activ has no end cap) is the same
class.  `show-deck`: the five rules are `min_enclosure` with `interacting_only`, and the
Rsil recognition (`RsilBase` = GatPoly ∩ RES ∩ EXTBlock) needs an EXTBlock, though the
manual's is "Rsil = RES + GatPoly".

Layouts, in every `<rule>.h1` of the enclosure kits (`Rsil.e`, `Rppd.d`, `Rhi.e`,
`Rppd.b`, `Rhi.c`): (10, 2) the right head running 0.1 out of the layer (poly to
x = 12.5, layer to 12.4); (10, 5) the layer over the body only, (10, 5)-(12, 5.5) plus
0.18 sideways, both heads out; (10, 8) no EXTBlock at all (the EXTBlock kits only).
`Rsil.d.h1` (10, 8): a resistor with no EXTBlock and a pSD 0.175 below it.

- gdscheck @20/7/100: nothing for the crossing head in any of the five; nothing for the
  body-only EXTBlock (three kits) and the body-only pSD-and-nSD (`Rhi.c.h1`); `Rppd.b.h1`
  reports the body-only pSD as two "enclosure 0.0000" markers at (10, 5.5)-(10, 5) and
  (12, 5)-(12, 5.5), its coincident edges (its rule reads the body, finding 5).  Nothing
  for the absent EXTBlock in the three kits; in `Rsil.e.h1` and `Rsil.d.h1` the
  EXTBlock-less resistor is no Rsil at all and gets one Rsil.c for its whole RES instead
  ((11, 8.25)), and its pSD at 0.175 is no Rsil.d.
- KLayout: the same silence on all of it (its `ext_enclosed` has
  `consider_intersecting_edges: false` on Rppd.d and Rhi.e, its Rsil recognition needs
  the EXTBlock too); `Rppd.b` the same two coincident pairs as gdscheck.
- Verdict: the crossing head fires once, the body-only layer twice (both heads out), the
  absent EXTBlock once (enclosed by nothing) and the EXTBlock-less Rsil is a Rsil: its
  Rsil.d fires, its RES is no protrusion.  The 0.175 margin at a head's end fires
  (`Rsil.e.h1` (10, 14): gdscheck and KLayout both report it - the crossing case is the
  one missed).  In `rsil_e_h1`, `rppd_d_h1`, `rhi_e_h1`, `rppd_b_h1`, `rhi_c_h1`,
  `rsil_d_h1`.

### 4. Rsil.d loses a pair whose poly ends on a tile line, at that tile size (tile-dependent)

Layout `Rsil.d.h2`: pSD 1 × 1 boxes 0.175 from the resistor's poly - the head ending at
x = 19.9 with pSD from 20.075 (across the line), the head ending exactly on x = 42 with
pSD from 42.175, on x = 60 with pSD from 60.175, a head starting exactly on x = 80 with
pSD ending at 79.825, a 300 µm body with pSD below its far end, and a resistor at
(1000, 1000) whose body's bottom edge lies on y = 1000 with pSD below it.

- gdscheck, `--deck resistor`: @20 3 (x = 20, x = 42, the 300 µm one), @7 5 (all but
  x = 42), @100 5 (all but y = 1000).  The pair is lost exactly when its poly's edge lies
  on a line of the tiling in use: 42 = 6 × 7, 60 = 3 × 20, 80 = 4 × 20, 1000 = 50 × 20 =
  10 × 100; x = 20 (poly ending at 19.9) is found at every tile.  With `--suite core`
  all six are found at every tile - the answer also depends on which decks run.
- KLayout: all six pairs ((19.9, 2.5)-(19.9, 2)/(20.075, 2)-(20.075, 2.542), (42, 5.5)-
  (42, 5)/(42.175, 5)-(42.175, 5.542), (60, ...), (80, ...), (301.042, 10)-(299.958, 10)/
  (300, 9.825)-(301, 9.825), (1001.542, 1000)-(1000.458, 1000)/(1000.5, 999.825)-
  (1001.5, 999.825)).
- The same geometry on drawn or intersected layers is found at every tile: `Rsil.b.h2`
  (a Cont ending on x = 60, RES from 60.115; RES starting on x = 42) and `Rppd.c.h2`
  (a Cont ending on x = 60, SalBlock from 60.195) are 5 at 20, 7 and 100.  Rsil.d's
  `GPRsilExtended` is the poly selected by overlapping the recognition region, which
  lies 0.5 further from the line than the head's end.
- Verdict: six.  In `rsil_d_h2`.
- Aside, another deck: on an interim `Rppd.a.h3` whose 0.495 bodies had their heads
  snapped 0.005 higher than the body, the salblock deck's Sal.c (the "enclosure 0.0000"
  of the 0.005 step at the SalBlock's end) was 7 at 20 and 8 at 100, the marker at
  (40, 2)-(40, 2.005) - the SalBlock ending on x = 40 - being the one lost at 20; and the
  steps at x = 5, 305, 1000 and 1002 were never reported.  The committed layouts have no
  such step.

### 5. Rppd.b reads the body, not the resistor's GatPoly: the head end is never measured (false negative)

Manual: "Rppd.b  Min. pSD enclosure of GatPoly  0.18", figure 6.4 with `b,d` at the
Cont's end of the head: the pSD encloses the poly, heads included, as the EXTBlock does.
`show-deck`: Rppd.b reads `[pSD, RppdAll]`, the poly under pSD and SalBlock - the body -
where Rppd.d reads `GPRppdExtended`, the whole poly, and Rhi.c `GPRhighExtended`.

Layouts: `Rppd.b.h1` (10, 14): pSD 0.175 from the right head's end ((12.5, 14)-(12.5,
14.5) against pSD ending at 12.675); (2, 11): a pSD chamfer passing 0.177 from the
head's corner (4.5, 11.5); `Rppd.b.h2` (17.5, 2): 0.175 at the head's end on x = 20.

- gdscheck @20/7/100: nothing for any of the three; the same three in `Rhi.c.h1`/`h2`
  (whole-poly layer) report the head end (12.5, 14)-(12.5, 14.5), the chamfer (4.5,
  11.5)-(4.625, 11.625) and the x = 20 end.
- KLayout: nothing either (`Rppd_all.ext_enclosed(pSD, 0.18)`, the body); its Rhi.c
  reports the head end and the x = 20 end (not the chamfer, settled).
- Verdict: all three fire.  In `rppd_b_h1`, `rppd_b_h2`.

### 6. The length rules read the block's narrower dimension: a narrow body is Rsil.f/Rppd.e/Rhi.f too, and a short block is Rsil.a/Rppd.a/Rhi.a too (false positive, duplicate id)

Manual: "Rsil.f  Min. RES length  0.50", "Rppd.e / Rhi.f  Min. SalBlock length  0.50",
`f` and `e` measured along the stripe in the figures; "Rsil.a / Rppd.a / Rhi.a  Min.
GatPoly width  0.50", `a` across it.  `show-deck`: the six are `min_width` on the block
(`RES`, `SalBlockRppd`, `SalBlockRhigh`) and on the body (`RsilAll`, `RppdAll`, `RhighA`),
which have the same two dimensions.

Layouts: `Rsil.f.h1` (2, 5): a 0.5 body under a 0.495 RES; (2, 8): a 0.6 body under a
0.495 RES; (2, 11): a 0.495 body under a 3.0 RES.  The same in `Rppd.e.h1` and
`Rhi.f.h1` with SalBlock, and every 0.495 body of the width kits.

- gdscheck @20/7/100: `Rsil.f.h1` Rsil.f 6 and Rsil.a 6 - each of the three twice under
  both ids; `Rsil.a.h1`-`h5` Rsil.f as often as Rsil.a; the 45° strip and the diamond of
  `Rsil.a.h2` are Rsil.f six times.  Rppd, Rhigh the same.
- KLayout: the same (`RES.ext_width`, `Rsil_all.ext_width`).
- Verdict: the 0.495-long blocks are the length rule's (twice each, the two end walls),
  the 0.495 body the width rule's; a 2.0 RES on a 0.495 body has length 2.0.  The true
  violation is always reported under the right id as well, so this costs a second id, not
  a miss.  In `rsil_f_h1`, `rppd_e_h1`, `rhi_f_h1`; the other width and length cases
  ignore the sibling rule.

### 7. Rhi.b reads nSD past pSD only: pSD past nSD, and nSD:drawing outside any Rhigh, are silent (false negative)

Manual: "Rhi.b  pSD and nSD are identical (Note 1)", note 1: "nSD:drawing is only
permitted within Rhigh resistors", section 4.2's footnote: "nSD as a drawing layer only
valid if pSD and nSD are identical".  `show-deck`: Rhi.b is `forbidden
[NsdMismatchAtRhigh]`, `NsdMismatch` = nSD − pSD, touching the recognition region.

Layouts `Rhi.b.h1`: (2, 5) nSD 0.005 past pSD on top; (2, 8) nSD 0.005 short of pSD on
top (pSD at 0.30, so Rhi.c holds); (2, 11) nSD 0.1 past pSD all round; (2, 14) nSD
ending at x = 3.0 in the body's middle, the right half under pSD alone; (10, 2) a bare
nSD:drawing (9.8, 1.8)-(11.2, 3.2) over an Activ, no resistor near.  `Rhi.b.h2`: nSD
0.005 past a pSD ending on x = 20, nSD 0.005 short of a pSD ending on x = 42.

- gdscheck @20/7/100: `Rhi.b.h1` 2 - (3, 5.6825) and (1.43, 10.77), the two nSD-past-pSD;
  nothing for (2, 8), (2, 14) or (10, 2).  `Rhi.b.h2` 2 (x = 20, (1000, 1000)); the x = 42
  short nSD is Rhi.c only ((41.82, 5)-(41.82, 5.5), enclosure 0.175 - right).  The
  half-covered resistor of (2, 14): no Rhi.c for the poly running out of pSD-and-nSD
  (finding 3), and a Rhi.d "too far" for its right Cont at (4.28, 14.25) - the Cont is
  0.20 from the drawn SalBlock, but `SalBlockRhigh` is the SalBlock over the recognised
  half only.
- KLayout: the same two Rhi.b polygons ((1.32, 5.68)-(4.68, 5.685) and the 0.1 ring),
  nothing for the short nSD, the half, or the bare nSD; the same Rhi.d polygon at
  (4.2, 14.17)-(4.36, 14.33).
- Verdict: (2, 8), (2, 14) and (10, 2) fire (five Rhi.b in `Rhi.b.h1`), (2, 14) is Rhi.c
  too and not Rhi.d; the x = 42 short nSD of `Rhi.b.h2` is Rhi.b and Rhi.c.  The bare
  nSD is note 1's letter; the existing `rhi_b` case reads an isolated nSD as clean after
  KLayout - one of the two has to give.  In `rhi_b_h1`, `rhi_b_h2`.

## Notes

- A. KLayout reports the Cont 0.5 beside the body's line on a 1.5 wide head (`Rppd.c.h1`
  and `Rhi.d.h1` (2, 14), Cont (1.64, 14.67)-(1.8, 14.83)) as "too far": its
  `SalBlock_Rppd.ext_extended(0.2)` grows the clipped block's straight edges only, so a
  Cont past the body's corner sees no block.  gdscheck's grown region covers the corner
  and is silent; the Cont is 0.20 from the drawn SalBlock, which extends 0.2 past the
  stripe.  gdscheck is right.
- B. KLayout's Rsil.c window: a RES 1.5 past the poly (`Rsil.c.h1` (2, 11)) is outside
  its `ext_enclosed(RES, 1.0)` and silent; gdscheck reports it.  Neither is the manual's
  answer (finding 1).
- C. Rppd.c's "max": a second Cont row at 0.54 (`Rppd.c.h1` (2, 11)) and a Cont at 1.0
  (10, 5) fire in both tools, as the manual's "min. and max. 0.20" says; a resistor head
  holds one row of Conts at exactly 0.20.
- D. The 0.205 gap (`Rppd.c.h1` (2, 8)) fires in both tools and 0.20 is clean: the grow
  of 0.20 with its 0.005 slack reads the bound right.
- E. The 45° bodies (`*.a.h2`): a strip 0.495 across its walls and a 0.495 diamond fire
  2 and 4 times in both tools, 0.502 is clean in both.  The chamfer 0.177 from the head's
  corner (`*.e.h1`, `*.d.h1`, `Rhi.c.h1` (2, 11)) fires in gdscheck only, 0.184 is clean
  in both - the settled reading.
- F. The recognition: a body on PolyRes with GatPoly heads (IHP's pcells; `*.a.h1`
  (10, 2)) is a resistor in both tools; a 0.495 head neck outside the block (`*.a.h1`
  (10, 5)) is not the body and clean in both; a 0.06 slot into a 0.55 body (0.49 under
  it) fires twice in both.  Not drawn: a Rsil under NWell or nBuLay, an Rppd over Activ
  (both tools drop them from the recognition; the manual's device definitions do not).
- G. The kits' 300 µm bodies and (1000, 1000) spots, and every flat/array pair, agree at
  20, 7 and 100 (finding 4 excepted); KLayout's hierarchical run counts an array cell
  once (51 for 50).

## Tested and found clean or correct (no need to redo)

- Rsil.a, Rppd.a, Rhi.a: 0.50 clean, 0.495 two walls; the slot; a body of 0.25 + 0.25
  boxes clean, 0.25 + 0.245 fires; PolyRes body; the head neck; the 45° strip and the
  diamond either side of the bound; bodies across x = 20, ending on x = 40, starting on
  x = 42, vertical across y = 20, 300 µm, (1000, 1000); fifty flat and as an array.
- Rsil.b: 0.12 clean, 0.115 fires; 0.08/0.08 corner to corner (0.113) fires, 0.115/0.06
  (0.130) clean; a bare Cont 0.115 below fires, at 0.12 clean; a Cont crossing the RES's
  end clean (shares area); the gap across x = 20, the RES on x = 42, the Cont on x = 60,
  300 µm, (1000, 1000); fifty flat and as an array.
- Rsil.d: 0.18 clean, 0.175 fires; 0.12/0.12 (0.170) fires, 0.13/0.13 (0.184) clean; pSD
  0.175 from a plain poly without RES clean; an Rppd's pSD 0.175 from the head fires;
  fifty flat and as an array.
- Rsil.e, Rppd.d, Rhi.e: 0.18 clean, 0.175 on one side one marker, 0.175 all round one
  marker; the chamfers; 0.175 at the head's end; two overlapping boxes enclosing by 0.18
  clean; 0.175 on the right with the poly ending on x = 20, 0.18 across x = 40 clean,
  0.175 across x = 42, 300 µm, (1000, 1000); fifty flat and as an array.
- Rppd.b (body side), Rhi.c: 0.175 on top and all round, the tile-line and array kits.
- Rppd.c, Rhi.d: 0.20 clean, 0.195 fires, 0.205 fires, the second row, the Cont at 1.0,
  0.195 on both heads, the wide head with the offset Cont clean; 0.195 across x = 20,
  the Cont on x = 40 at 0.20 clean, the block on x = 42, the Cont on x = 60, 300 µm,
  (1000, 1000); fifty flat and as an array.
- Rsil.f, Rppd.e, Rhi.f: 0.50 clean, 0.495 fires (two walls), also on a 0.6 body; blocks
  ending on x = 20, across x = 40, starting on x = 42, at (1000, 1000); fifty flat and
  as an array.
- Rhi.b: identical clean; nSD 0.005 past pSD fires, 0.1 all round one marker; on x = 20,
  identical across x = 40 clean, (1000, 1000); fifty flat and as an array.
