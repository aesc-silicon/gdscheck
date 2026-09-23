<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / pwellblock, nbulay, nbulayblock, extblock, salblock, contbar: hardening report

The six block decks against SG13G2 Layout Rules Rev. 0.4: section 5.2 (PWB.a-PWB.f1), 5.3
(NBL.a-NBL.f), 5.4 (NBLB.a-NBLB.d), 5.12 (EXTB.a-EXTB.c), 5.13 (Sal.a-Sal.e) and 5.15
(CntB.a-CntB.j), with section 4.2's derived layers.  Every rule of the six sections is in
its deck (PWB.d, "overlap is allowed", is no check).  180 layouts,
`tests/data/ihp-sg13g2/<deck>/<RULE>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/{pwellblock,nbulay,nbulayblock,extblock,salblock,contbar}.rs`, each with a `#[case]` in its deck's table of
`tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through IHP's KLayout decks (`hardening/oracle-ihp.sh`).

Reading the numbers: gdscheck gives one `min_width` marker per wall (two per narrow bar,
four per diamond or L), one space marker per pair, one enclosure marker per
under-enclosed side or one for adjacent sides; the KLayout column of the oracle adds the
driver's and the maximal deck's reports, so a rule both run counts twice there, an edge
pair per marker, and its hierarchical run counts an array cell once.  Of the six
sections, KLayout's maximal deck runs everything but PWB.e-f1, NBL.b-f, CntB.b1 and
CntB.h1, which only the driver runs (NBL.b-f with connectivity).

Test status on the engine as of this report: 41 of the 218 new cases fail, all on the
findings below; the other 177 pass.  No count moved with the tile size except in
finding 4, and every flat/array pair agreed.

## Resolution (2026-09-21)

Fixed, deck: 1 (`abutting: report` on PWB.e/e1/f/f1, EXTB.c, Sal.d/e, CntB.e/f/g1;
the rules worded "unrelated" - NBL.e/f, NBLB.d - take the glossary's reading instead,
`abutting: related`: a shape touching anywhere is related and no pair), 2 (`NActiv` is
section 4.2's N+Activ everywhere, and the PWell:block excluded from the N+/P+ Activ in
PWell - the implant report's findings 6 and 7), 3 (`min_notch` for NBL.b), 5 (the
merge closes gaps under 1.50, radius 0.7475: two nBuLay exactly 1.50 apart are NBL.c's),
6 (`gap_outside: PWell.block` on NBL.c and NBL.d), 8 and 9 (the generated nBuLay of
section 4.2 is read *inward* - the well shrunk by 1.5 and grown by 0.5, IHP's reading,
which the activ deck's filler derivation had the wrong way round; NBL.c/d read
`nBuLayRegions`, drawn and generated closed together; NBLB.c reads `nBuLayAll` with
`interacting_only`, NBLB.d `nBuLayAll`), 11's PWB.c (a second entry with `pairs:
overlapping` reads the well's far arm against the block over its near one).

Fixed, engine: 4 (the marker of a parallel pair is the shared stretch's lowest end,
one point for every tile - a fix of the metaln round that reaches this case).

Kept, cases flipped: 10 (`unrelated` is the glossary's "do not touch", not a net: the
nBuLay's own tie 0.5 outside it fires), 12 (CntB.a reads the bar's box, as both tools),
Sal.d's U (11, both tools silent), the block crossing the nBuLay edge (an extension
under NBLB.c), and the Cont crossing the SalBlock edge (shares area, no pair).  A wide
well beside another is two NBL.d pairs (each well's generated nBuLay against the
other well).

Open: 7 (a PWell:block adjoining a region extends it, figure 5.3 and KLayout; the
engine reads the block as no PWell only - `NBL.c.h9`, `NBL.d.h7` carry the engine's
answer).

Decided (2026-09-21, gdscheck owner): 7 - the block adjoining a region extends it
(`nBuLayRegionsExt`, `NWellExt`); `NBL.c.h9` reads 1, `NBL.d.h7` 2.

## Findings

### 1. A touch is a space of nothing: nine two-layer space rules stay silent when the other shape abuts (false negative)

Manual: "Min. PWell:block space to (N+Activ ...) in PWell 0.31" (PWB.e, likewise PWB.f),
"Min. nBuLay space to unrelated N+Activ 1.00" (NBL.e, likewise NBL.f), "Min. nBuLay:block
space to unrelated nBuLay 1.50" (NBLB.d), "Min. EXTBlock space to pSD 0.31" (EXTB.c),
"Min. SalBlock space to unrelated Activ or GatPoly 0.20" (Sal.d), "Min. SalBlock space to
Cont 0.20" (Sal.e), "Min. ContBar on GatPoly space to Activ 0.14" (CntB.e), "Min. ContBar
on Activ space to GatPoly 0.11" (CntB.f), "Min. pSD space to ContBar on nSD-Activ 0.09"
(CntB.g1).  A shape whose edge lies on the other's edge is 0 away.  The cont and nwell
reports found this class for Cnt.e/f/g1 and NW.d, which now carry `abutting: report`;
these rules do not.

Layouts: the last pair of every `<rule>.h1` of the kit (`PWB.e.h1` (40, 2), `PWB.f.h1`
(38.8, 2), `NBL.e.h1` (77, 2), `NBL.f.h1` (70, 2), `EXTB.c.h1` (38.4, 2), `Sal.d.h1`
(30.5, 2), `CntB.e.h1`, `CntB.f.h1`, `CntB.g1.h1`), `NBLB.d.h5` (21, 3): a block abutting
an nBuLay from outside, `Sal.e.h1` (15, 2.42): a Cont abutting a block, (20.92, 2.42): a
Cont crossing the block's edge, `CntB.g.h1` (8, 2): a bar on the seam of an abutting
Activ and GatPoly, `CntB.j.h1` (10.16, 2) and `CntB.j.h2` (20, 12): a bar on poly
abutting Activ, `NBL.e.h5` (22.75, 11.75) and `NBL.f.h5` (30.75, 3.75): an Activ crossing
the nBuLay edge, `CntB.g2.h1` (16, 2) and `CntB.g2.h5`: the pSD edge through a bar.

- gdscheck @20/7/100: nothing at any of them; the same layouts' 0.005-under gaps and
  diagonal gaps are reported.
- KLayout: the coincident edge as the edge pair for PWB.e ((40.025, 2)-(40.025, 2.5)),
  PWB.f ((38.815, 2)-(38.815, 2.5)), NBL.e ((89.935, 2)-(89.935, 2.5)), EXTB.c ((40.025,
  2)-(40.025, 2.5)), Sal.d ((38.145, 2)-(38.145, 2.5)), Sal.e ((15, 2.42)-(15, 2.58)),
  CntB.e ((34.035, 2)-(34.035, 2.5)), CntB.f ((33.525, 2)-(33.525, 2.5)), NBLB.d ((21,
  3)-(21, 5)), the seam of `CntB.g.h1` (CntB.e and CntB.f), `CntB.j.h1`/`h2` (CntB.e at
  (10.16, 2) and (20, 12)); nothing for NBL.f (its polygon output drops the zero-area
  pair) and CntB.g1 (note H); nothing for the crossing shapes (its separation checks
  skip overlapping polygons).
- Verdict: all fire.  The crossing Activ of `NBL.e.h5`/`NBL.f.h5` and the crossing Cont of
  `Sal.e.h1` follow the nwell report's reading of NW.d (the part outside is 0 away); a
  crossing nBuLay:block is NBLB.c's (finding 8), not NBLB.d's.  In `pwb_e_h1`, `pwb_f_h1`,
  `nbl_e_h1`, `nbl_f_h1`, `nbl_e_h5`, `nbl_f_h5`, `nblb_d_h5`, `extb_c_h1`, `sal_d_h1`,
  `sal_e_h1`, `cntb_e_h1`, `cntb_f_h1`, `cntb_g_h1`, `cntb_g1_h1`, `cntb_g2_h1`,
  `cntb_g2_h5`, `cntb_j_h1`, `cntb_j_h2`.

### 2. Plain Activ is not N+Activ to PWB.e, NBL.e and CntB.g1 (false negative), and Activ under nSD:block is to NBL.e (false positive)

Manual, section 4.2: "nSD = NOT (pSD OR nSD:block) OR nSD:drawing", "N+Activ = Activ AND
nSD".  An Activ with nothing drawn on it is N+; that is every NMOS source/drain and every
substrate-side contact.  `show-deck`: PWB.e reads `NsdActivInPWellNoTGO`, PWB.e1
`NsdActivInPWellTGO`, CntB.g1 `ContBarOnNsdActiv` - all built on the drawn nSD; NBL.e
reads `NActiv` = Activ minus pSD, which takes an Activ under nSD:block for N+.  The cont
report's finding 5 fixed the same for Cnt.g1 (`NActFull`); the nwell deck's NW.d/NW.e use
it too.

Layouts: every `PWB.e.h*` and `CntB.g1.h*` (plain Activ throughout); `PWB.e.h5` (a row of
1 × 1 blocks with 0.5 Activs 0.305 to the right: plain at (3.305, 2.25), drawn nSD at
7.305, nSD:block at 11.305, pSD at 15.305, in a well at 19.305, crossing a well edge at
23.305, under ThickGateOx at 28.305, ThickGateOx abutting at 32.305, ThickGateOx over the
near half of a 1.0 Activ at 36.5, half under the block at 39.75, wholly under it at
43.25); `PWB.e1.h1`; `PWB.f.h5` (7.235, 2.25: pSD over the far half); `NBL.e.h5` (Activ
0.995 right of 4 × 4 nBuLays: plain at (6.995, 3.75), drawn nSD at 14.995, nSD:block at
22.995, pSD at 30.995); `CntB.g1.h5` (pSD 0.085 from a bar on plain Activ at (2, 2), on
drawn nSD at (4, 2), under nSD:block at (6, 2), on P+ at (8, 2)).

- gdscheck @20/7/100: `PWB.e.h1`-`h4`, `PWB.e1.h1`, `CntB.g1.h1`-`h4`: nothing.
  `PWB.e.h5`: PWB.e 1 (the drawn nSD only).  `PWB.f.h5`: PWB.f 1, no PWB.e.  `NBL.e.h5`:
  NBL.e at 6.995, 14.995 and at 22.995 (the nSD:block Activ).  `CntB.g1.h5`: CntB.g1 1
  (drawn nSD).
- KLayout (driver; `nact_fet = activ not (psd or nsd_block) and pwell`, `nactiv` the
  same without the well): `PWB.e.h1` eight edge pairs on the five structures, `PWB.e.h5`
  PWB.e at x = 3, 7, 23, 32 and 40 (the coincident edge), PWB.e1 at 28 and 36.5;
  `PWB.e1.h1` 1; `PWB.f.h5` PWB.e 1 and PWB.f 1; `NBL.e.h5` NBL.e at 6.995 and 14.995,
  nothing at 22.995; CntB.g1 nothing anywhere, drawn nSD included (note H).
- Verdict: plain Activ fires and nSD:block Activ does not.  Expected 5 in `pwb_e_h1`, 10
  in `pwb_e_h2`, 50 in `pwb_e_h3`/`h4`, PWB.e × 5 + PWB.e1 × 2 in `pwb_e_h5` (the well
  and the block cases clean: "in PWell"), 1 in `pwb_e1_h1`, PWB.e + PWB.f in `pwb_f_h5`,
  NBL.e × 3 in `nbl_e_h5` (with findings 1 and 10), 5/10/50/50 in `cntb_g1_h1`-`h4`, 2 in
  `cntb_g1_h5`, CntB.g2 × 8 + CntB.g1 in `cntb_g2_h1`, CntB.g1 + CntB.g2 in `cntb_g2_h5`.
  ThickGateOx over half an Activ splits it (e1 for the covered half, e for the bare
  half), as the nwell deck's NW.c/c1 do; both tools agree.

### 3. NBL.b has no notch half (false negative)

Manual: "NBL.b  Min. nBuLay space or notch (same net)  1.50".  The deck runs `min_space`
only; PWB.b, NBLB.b, EXTB.b and Sal.b carry a second `min_notch` entry (and the nwell
report's finding 1 added NW.b's).

Layout `NBL.b.h2`: a straight U notch and a straight-vs-45° notch at 1.495 (the helpers'
patterns, 1.5 controls beside them), a comb with three 1.495 slots at (2, 16.3), a ring
with a 1.495 hole at (15.1, 16.3); plus a square 1.495 from a ring's inner wall and two
unions 1.495 apart (separate shapes).

- gdscheck @20/7/100: NBL.b 2 (the island and the unions).
- KLayout: NBL.b 8 (the eight notches, one net each), NBL.c 2 (the island and the
  unions, unconnected).
- Verdict: the eight notches are NBL.b.  Expected 10 (`nbl_b_h2`).

### 4. A square in a ring's hole 1.495 from two walls: the count moves with the tile size

Manual: NBL.b 1.50.

Layout `NBL.b.h6`: a 1.1-thick nBuLay ring (2, 2)-(10.195, 10.195) with the hole
(3.1, 3.1)-(9.095, 9.095) and a 3 × 3 square at (4.6, 4.6): 1.5 from the left and bottom
walls, 1.495 from the right and top.

- gdscheck @20: NBL.b 1 ((4.6, 9.095)-(4.6, 7.6), the top gap); @7: 2 (the top and
  (9.095, 4.6)-(7.6, 4.6), the right gap); @100: 1 (the top).  The same shape in
  `NBL.b.h2` before the island was moved to one bad wall: two markers at 20 and 7, one at
  50, 100 and 200.  `PWB.b.h2`'s twin (0.615 on two walls) gave one at every tile.
- KLayout: NBL.c, one polygon around the island (unconnected regions).
- Verdict: two walls under the value, and one count whatever the tile.  The case
  (`nbl_b_h6`) expects 2, the engine's own answer where both walls are seen; if one
  marker per pair of shapes is the convention it should be one at every tile.

### 5. Two bare nBuLay regions exactly 1.50 apart are reported by nobody (false negative at the boundary)

Manual: NBL.b's 1.50 is a space (1.50 is legal); NBL.c "Min. PWell width between nBuLay
regions (different net) 3.20".  Two unconnected regions 1.50 apart are on different nets
with 1.50 of PWell between them.  The deck's `nBuLayMerged` closes gaps up to and
including 1.50 (radius 0.75), and the existing `NBL.b` case documents it; the nwell
report's finding 2 moved NW.b1's close to one step under half the value for the same
reason.

Layout `NBL.c.h8`: 3 × 3 pairs at 1.495, 1.5, 1.505, 3.195 and 3.2 (y = 2, 9, 16, 23, 30).

- gdscheck @20/7/100: NBL.b at 1.495; NBL.c at 1.505 and 3.195; nothing at 1.5.
- KLayout: NBL.c at 1.495, 1.5, 1.505 and 3.195 (all different nets to it; it has no
  NBL.b for unconnected regions).
- Verdict: 1.5 fires NBL.c.  Expected NBL.b + NBL.c × 3 (`nbl_c_h8`).  The settled
  reading keeps 1.495 as NBL.b (gdscheck) or NBL.c (KLayout); see note A.

### 6. NBL.c and NBL.d ignore PWell:block: the fully blocked gap (false positive, debatable)

Manual, section 4.2: PWell = NOT (NWell OR PWell:block); NBL.c and NBL.d are "Min. PWell
width between ...".  The nwell report's finding 4 (NW.b1, the same wording) was settled
the same way: `gap_outside: NWellOrPWellBlk`; `show-deck` shows no such parameter on
NBL.c or NBL.d.

Layouts `NBL.c.h6`: two nBuLay 2.0 apart with a block over the whole gap (2, 2) and two
with a 0.7 block strip in the middle (14, 2); `NBL.d.h5`: a well 2.0 from an nBuLay under
a full block (2, 9) and under a 0.7 strip (13, 9).

- gdscheck @20/7/100: NBL.c for both pairs, NBL.d for both pairs.
- KLayout (driver, `... .and(pwell)`): the half-blocked pairs only (the 0.65 strips).
- Verdict: with the gap fully blocked there is no PWell whose width the rule could
  measure; the block-to-region distance is the block's own rule.  Expected 1 in
  `nbl_c_h6`, 1 in `nbl_d_h5`.  Debatable exactly as NW.b1's was; if the gdscheck owner wants
  the physical reading here the cases flip, not the engine.

### 7. NBL.c and NBL.d do not measure from a PWell:block that adjoins a region (false negative)

Manual, figure 5.3: c is drawn from an nBuLay to the far edge of a PWell:block that
adjoins the other nBuLay, and d from an nBuLay to a PWell:block that adjoins the NWell
(figure 5.1 draws NW.b1 the same way).  The block is not PWell, so the PWell strip runs
from the block's far edge.

Layouts `NBL.c.h9`: nBuLay (2, 2)-(5, 5) and (10, 2)-(13, 5) with a block (4, 1.5)-(6.805,
5.5) overlapping the left one and 3.195 from the right (fires); the same at 3.2 (20, 2)
(clean); a block 3.195 from an nBuLay but adjoining none (2, 12) (clean).  `NBL.d.h7`: an
nBuLay (2, 2)-(5, 5) with a well (9, 3)-(11, 4) 4.0 away and a block (7.195, 1.5)-(9.5,
5.5) adjoining the well, 2.195 from the nBuLay; and a block (17.5, 1.5)-(19.805, 5.5)
adjoining the nBuLay (15, 2)-(18, 5), 2.195 from the well (22, 3).

- gdscheck @20/7/100: nothing in either.
- KLayout (driver, `pwell_block.interacting(nbulay).sep(nbulay, 3.2)` and
  `pwell_block.interacting(nwell).sep(nbulay, 2.2)`): NBL.c 1 at 3.195 ((6.805, 1.8)-(10,
  5)) and nothing at 3.2 or for the lone block; NBL.d 1 for the block adjoining the well
  ((5, 2)-(7.195, 5)); nothing for the block adjoining the nBuLay (its term looks one way
  only).
- Verdict: the figure's reading.  Expected 1 in `nbl_c_h9`, 2 in `nbl_d_h7` (the block
  adjoining either region: the PWell between is 2.195 both ways).

### 8. NBLB.c reports every block no drawn nBuLay encloses (false positive)

Manual: "NBLB.c  Min. nBuLay enclosure of nBuLay:block  1.00"; section 5.4: "nBuLay:block
is used for generating NWell structures, which are prevented from nBuLay implant".  The
ordinary block lies on a wide well that has no drawn nBuLay at all (section 4.2's
generated one).  The existing `nblb_a`, `nblb_b_*` and `nblb_d` cases already ignore
NBLB.c.

Layout `NBLB.c.h5`: a bare 2 × 2 block at (2, 2); a block 2.5 inside a 10 × 10 well (10.5,
4.5); a block 1.5 inside a 10 × 10 well (23.5, 3.5); a block over a whole 6 × 6 well and
1.0 beyond it (35, 1).  Also every block of the NBLB.a/b/d kits.

- gdscheck @20/7/100: NBLB.c "shape not enclosed by nBuLay" for all four, and for
  every block of the kits (`NBLB.a.h2`: 9 at tiles 20 and 100, 10 at 7).
- KLayout: nothing in `NBLB.c.h5` and in every kit layout (its `ext_enclosed` has no
  pair for a block far from nBuLay); in `NBLB.d.h5` NBLB.c for the crossing block only
  ((12, 3)-(14, 5), through `ext_overlapping`) and NBLB.d for the abutting one.
- Verdict: a block with no nBuLay near it violates nothing; a block under 1.0 from a
  drawn nBuLay's edge is NBLB.d's (1.5).  Expected 1 in `nblb_c_h5` (the block 1.5
  inside the well, finding 9) and NBLB.c once in `nblb_d_h5` (the crossing block).  The
  kits' cases ignore NBLB.c as the existing ones do.

### 9. The generated nBuLay: NBL.c, NBL.d, NBLB.c and NBLB.d read the drawn layer only, and the derivation's direction (false negative)

Manual, section 4.2: "nBuLay = (((NWell ≥ 3.0 µm) sized by 1.0 µm/side) OR nBuLay:drawing)
AND NOT nBuLay:block"; NBL.c/d note 1: "drawn as well as generated nBuLay regions are
considered (see 4.2)".  The deck's NBL rules read `nBuLay` and `nBuLayMerged`, the NBLB
rules `nBuLay`; the derivation exists (`nBuLayDerived`) for the filler rules only, where
the settled reading put it - grown by 1.0 (`NWellWideSized`, `grow 1.0`).

IHP's decks shrink: the driver's `nwell_drw.sized(-1.495).sized(0.495)` and the maximal
deck's `NWell.sized(-1.5).sized(0.5)` are the ≥ 3.0 filter (−1.5, +1.5) followed by
−1.0.  The rule set is coherent only that way: with the well inset by 1.0, NBL.c's 3.2
between a wide well's nBuLay and a drawn one is NBL.d's 2.2 from the well, NBL.d's 2.2
between two wells is 1.2 (under NW.b1's 1.8) and NBL.e's 1.0 to N+Activ ends at the
well's edge, where the tie is the well's own net.  Grown by 1.0, NBL.e would forbid
unrelated N+Activ within 2.0 of every wide well (NW.d allows 0.31) and NBL.f every
substrate tie within 1.5 (NW.f allows 0.24).  So the manual's "sized by 1.0 µm/side" is
an inset, and the filler rules' `NWellWideSized` grows the wrong way (not this deck's
rule; noted for the activ deck's owner).

Layouts (all with the inset): `NBL.c.h7`: a 6 × 6 well (2, 2) and a drawn nBuLay 2.195
away (10.195, 2) (the generated nBuLay is 3.195 from the drawn one, the well 2.195); two
6 × 6 wells 1.15 apart (2, 12), (9.15, 12) (generated regions 3.15 apart, and each 2.15
from the other well); two 1.2 apart (20, 12), (27.2, 12) (3.2); a 2.5-wide well (20, 2)
2.195 from a drawn nBuLay.  `NBL.d.h6`:
a 6 × 6 well and a 1 × 1 well 1.15 apart (2, 2), (9.15, 4.5) (2.15); 1.5 apart (14, 2),
(21.5, 4.5) (2.5); a 2.5-wide well 1.15 from a 1 × 1 well (26, 2).  `NBLB.c.h5` (23.5,
3.5): a block 1.5 inside a 10 × 10 well (the ring of generated nBuLay round it is 0.5,
NBL.a's width; the block at (10.5, 4.5) 2.5 inside leaves 1.5).  `NBLB.d.h5`: a block 0.4
outside a 6 × 6 well (8.4, 12) (1.4 from the generated nBuLay) and 0.5 outside (20.5, 12).

- gdscheck @20/7/100: `NBL.c.h7` NBL.d 2 (the well and the 2.5-wide well to the drawn
  nBuLay), no NBL.c; `NBL.d.h6` nothing; `NBLB.c.h5`/`NBLB.d.h5` finding 8's markers only.
- KLayout (driver, generated inset): `NBL.c.h7` NBL.c 2 (the well to the drawn nBuLay,
  (8, 2.9)-(10.195, 5); the two wells, (8, 13)-(9.15, 17)) and NBL.d 3 (the two wells to
  the drawn nBuLay, and the 1.15 pair again: 2.15 from either well's generated nBuLay to
  the other well); `NBL.d.h6` NBL.d 1 ((8, 4.25)-(9.15, 5.75)); the maximal deck's
  NBLB.c/d read the drawn layer only, nothing - but its NBL.a, which reads the generated
  nBuLay less the block, reports the 0.5 ring round the block 1.5 inside the well of
  `NBLB.c.h5` (eight edges, "width 0.5 < 1.0") and nothing round the block 2.5 inside or
  the block over the whole well: IHP's own derivation, inset, with the block cut out.
- Verdict: expected NBL.c × 2 + NBL.d × 3 in `nbl_c_h7`, 1 in `nbl_d_h6`, 1 in
  `nblb_c_h5`, NBLB.d × 2 in `nblb_d_h5` (with finding 1).  The direction is the crux:
  under the outset `NBL.d.h6`'s 1.5 pair fires instead of its 1.15 pair, `NBLB.c.h5`'s
  1.5-inside block is clean and `NBLB.d.h5`'s 0.4-outside block is NBLB.c (the block
  crosses the grown nBuLay).

### 10. NBL.e and NBL.f take the nBuLay's own well tie and diffusion for unrelated (false positive)

Manual: "Min. nBuLay space to unrelated N+Activ 1.00", "... unrelated P+Activ 0.50".
`show-deck`: neither NBL.e nor NBL.f has `net: different` (NBL.c and NBL.d have).  The
nmosi ring (section 6.5) is an NWell on the nBuLay's edge with the well ties in it.

Layouts `NBL.e.h5`: a 4 × 4 nBuLay (2, 10) with a well (5, 11)-(8, 13) over its right
edge and an N+ tap with Cont and Metal1 at (6.75, 12) - 0.5 outside the nBuLay, in the
well that is the nBuLay's net; an nBuLay (11, 10) with a sinker (12, 11)-(14, 13), a tap
in it at (13, 12) and an N+ tap in PWell at (15.75, 12) 0.5 from the nBuLay, one Metal1
strap over both.  `NBL.f.h5`: a P+Activ (6.3, 3.75)-(6.8, 4.25) in a well (5, 3)-(8, 5)
over the nBuLay's edge, 0.3 outside it (a PMOS's diffusion); a P+ tie at (15.55, 4) 0.3
from the nBuLay, strapped to the tap in the nBuLay's sinker.

- gdscheck @20/7/100: NBL.e "0.5000" at (6, 11.75) and (15, 11.75); NBL.f "0.3000" at
  (6, 3.75) and (15, 3.75).
- KLayout (driver, `props_ne`; `ptap` is P+ in PWell only): `NBL.e.h5` NBL.e at 6.995
  and 14.995 only (the plain and the drawn-nSD Activ), `NBL.f.h5` NBL.f 1 (the P+ half at
  23.495) and NBL.e 1 - none of the four.
- Verdict: all four clean.  The taps are the nBuLay's net (through its well, through
  the strap); "unrelated" is the manual's word for it, and the deck's NBL.c/d already
  read it as `net: different`.  The PMOS diffusion in the nBuLay's well is the least
  certain of the four: it is not the nBuLay's net, but it is the well's device and no
  P+Activ "in PWell"; IHP's deck never looks at P+ in a well for NBL.f.  In `nbl_e_h5`,
  `nbl_f_h5`.

### 11. A polygon that overlaps the block and faces it elsewhere: PWB.c (false negative) and Sal.d (false negative, debatable)

Manual, figure 5.2: the well at the right overlaps the block (d, "overlap is allowed")
and its upper arm keeps c from the block's top edge - one polygon, both.  Sal.d: "Min.
SalBlock space to unrelated Activ or GatPoly 0.20".

Layouts `PWB.c.h5`: a U-shaped well whose lower arm and connector overlap a 3 × 2 block
and whose upper arm is 0.615 above the block's top edge (block (2, 8), arm at y = 10.615);
the same at 0.62 (block (10, 8)); a well ring 0.615 from the block's right edge and 1.0
elsewhere (28, 8).  `Sal.d.h5` (6, 2): a U-shaped Activ whose left arm is under a block
(extended by 0.5 all round) and whose right arm is 0.195 from the block's right edge.

- gdscheck @20/7/100: `PWB.c.h5` PWB.c 1 (the ring); the U is silent, at 0.615 as at
  0.62.  `Sal.d.h5` Sal.d 2 (the GatPoly and the plain Activ), the U's arm silent.
- KLayout: PWB.c 1 (the ring, (30.615, 7.9)-(30.615, 10.1); its
  `PWellBlock_unrelatedNWell` drops every well that touches the block); Sal.d 2, the
  same two as gdscheck, the U's arm silent as well.
- Verdict: the U well fires PWB.c: the manual draws exactly this.  Expected 2
  (`pwb_c_h5`).  The U Activ's uncovered arm fires Sal.d (expected 3, `sal_d_h5`): the
  block covers one arm and neither extends over (Sal.c) nor keeps 0.2 from (Sal.d) the
  other; "unrelated" describes the arm, not the polygon.  The engine's `min_space` skips
  a pair once it overlaps anywhere, which both layouts show.  The Sal.d half is the less
  certain: both tools read "unrelated" per polygon; if the gdscheck owner does too, `sal_d_h5`
  drops to 2 - the PWB.c half stands on the figure.

### 12. CntB.a: a nick in a bar's side is missed, an L and a T of 0.16 arms are reported (false negative, false positive)

Manual: "CntB.a  Min. and max. ContBar width  0.16"; "Any Cont shape not being a square
shape is considered a ContBar".

Layout `CntB.a.h1`: a 0.16 × 0.5 bar with a 0.005 × 0.1 nick in its right side (12, 2)
(0.155 wide there); an L (14, 2) and a T (15, 2) of 0.16 arms 0.5 long; beside them the
plain bound (0.155, 0.165 both ways; 0.335 and 0.165 long), a 0.17 square, a bar as
halves, as overlapping boxes, as a square abutting a bar, two bars overlapping sideways
into 0.26, and a 45° bar of width 0.163.

- gdscheck @20/7/100: nothing for the nick; CntB.a "width 0.5000 > 0.16" for the L and
  "0.6600" for the T (the extent of the shape, not a width); "0.6150" for the 45° bar
  (right for the wrong number: it is 0.163); the rest as the manual.  The 0.17 square
  is clean (a square is Cont's).
- KLayout: CntB.a for the nick ((12, 2.16)-(12, 2.34) against (12.155, 2.2)-(12.155,
  2.3), its `ext_width(0.16)`), the 0.155, 0.165 and 0.26 bars and the 45° bar; nothing
  for the L or the T (its opening by 0.08 leaves nothing of them); CntB.a and CntB.a1 for
  the 0.17 square (the maximal deck's `ContBar` is any Cont with area over 0.16²); and
  CntB.b for the nick's 0.1-wide notch.
- Verdict: the nick fires, the L and the T do not.  Expected CntB.a × 7 + CntB.a1 × 4
  (`cntb_a_h1`).  The nick as a notch is not asked: CntB.b is "Min. ContBar space" with
  no "or notch"; the 45° bar is an angle violation besides (section 3.2, KLayout's
  `cont_drw_Angle90`).

## Notes that are not findings

- A. NBL.b and NBL.c labels for bare regions under 1.5 apart: gdscheck NBL.b (the
  merged layer closes them), KLayout NBL.c (unconnected is different nets) - the settled
  reading of NW.b/NW.b1.  The merged layer does not close a diagonal 1.499 gap, a diamond
  tip 1.495 from a wall or a chamfer 1.499 from a corner, so those pairs get both labels
  from gdscheck (`NBL.b.h1` (28.4, 5), (61.1, 3.5), (85.6, 6.1)); harmless.  The NBL.b
  cases ignore NBL.c, since their controls at 1.5 and over are NBL.c by the manual
  (finding 5); the NBL.c cases take the bare pairs as different nets.
- B. NBLB.c's count for bare blocks moves with the tile (`NBLB.a.h2`: 9, 10, 9 at 20, 7,
  100) - all false positives of finding 8, so not counted as a tile finding.
- C. Sal.c on a crossing shape is an extension, not an enclosure: an Activ running out
  of the block with 0.7 beside it (`Sal.c.h1` (13.5, 2)) is clean in gdscheck, as it
  should be; NBLB.c and CntB.c/d/g2/h1 report the crossing shape (enclosed by nothing on
  that side), as the cont report's note C has it.  A bar half out of its Activ is
  CntB.c and CntB.g, half out of its Metal1 CntB.h1 and CntB.h, half out of pSD CntB.g2
  and CntB.g1 (finding 1); a bare bar is CntB.h and CntB.h1 both (enclosed by less than
  0.05; KLayout's CntB.h1 has no pair for it and reports the half-covered and the
  coincident bars only), and CntB.g without CntB.c (no bar on Activ to enclose).
- D. Sal.c reads the union: an Activ 0.195 from the block's edge wholly under a GatPoly
  that runs out of the block is clean in gdscheck (`Sal.c.h5` (20.2, 2.195)); KLayout
  checks Activ and GatPoly one by one and reports it.  The manual says "Activ or
  GatPoly"; gdscheck's reading is the letter's.
- E. CntB.b1's run: gdscheck joins the stretches along one stepped wall (`CntB.b1.h1`
  (13.015, 2.2): a bar whose upper half jogs 0.05 closer, 3.0 at 0.355 and 3.0 at 0.305,
  one run of 6, fires; KLayout's projection reads each edge pair alone and is silent, as
  the settled reading says) and does not join three collinear 1.9 bars facing a 6.5 bar
  (each pair's common run is 1.9: clean in both), which is the manual's pairwise "common
  run".  A run of exactly 5.0 is clean, 5.005 fires, and a 5.005 run cut by x = 20 fires
  at every tile.  The jogged bar is also CntB.a in both tools: the chord across the jog,
  from the lower segment's left wall to the upper's right wall, is 0.11 - KLayout's
  `ext_width` reports that pair, gdscheck reports "width 0.2100 > 0.16" at (13.12, 5.2),
  the other chord; the same shape, the settled chord reading, two numbers.
- F. ThickGateOx over half an Activ splits PWB.e/e1 (and f/f1) by the part, both
  tools; a ThickGateOx abutting the Activ leaves it in e.  Not tested: the DigiBnd
  variants (section 8.1 has none for these six sections).
- G. The kits' 300 µm shapes and (1000, 1000) spots, and every flat/array pair, agree
  at 20, 7 and 100; KLayout's hierarchical run counts an array cell once (51 for 50).
- H. KLayout's CntB.g1 (maximal deck, `pSD.ext_separation(ContBar_NAct, 0.09, max_angle:
  0, ...)`) reports nothing in any `CntB.g1.*` layout, not even the bar on drawn nSD of
  `CntB.g1.h5` or the pSD edge through the bar of `CntB.g2.h5`; the cont report's note F
  saw the same.  Its silence on this rule is no evidence either way.
- I. KLayout's enclosure checks miss the chamfers passing under the value from a corner
  (`Sal.c.h1` (14.58, 3.2), `NBLB.c.h1`, `CntB.c.h1`, ...), as the settled reading says;
  gdscheck reports them.

## Tested and found clean or correct (no need to redo)

- PWB.a, NBL.a, NBLB.a, EXTB.a, Sal.a (the width kit): the bound both ways; a diamond
  and a 45° strip one step under and over; an L; a chamfered box; unions of overlapping
  and abutting boxes and quadrants; a bar of three boxes; a frame's narrow side; a
  clockwise square; an island in a ring; a 0.005 sliver; bars on and across x = 20, 21,
  40, 42 and y = 20; a 300 µm bar; (1000, 1000); fifty flat and as a GdsArrayRef.
- PWB.b, NBLB.b, EXTB.b, Sal.b (the space kit) and NBL.b's space half: the bound; corner
  to corner under and over; 0.1 in x with the value in y; a diamond tip; a corner to a
  chamfer; parallel chamfers; a U notch, a straight-vs-45° notch, a comb, a ring's hole,
  an island, two unions; gaps on and across the tile lines and a corner on (60, 20);
  300 µm bars; (1000, 1000); fifty flat and array.
- NBL.c: the same on bare regions at 3.195/3.2, with the corner on (80, 20); the
  strapped pairs are the existing `NBL.c.same_net`.
- PWB.c, NBL.d, NBLB.d, EXTB.c, Sal.d, Sal.e, CntB.b2, CntB.e, CntB.f (the two-layer
  kit): the bound; corner to corner; 0.1 in x; the free layer's diamond tip and chamfer;
  tile lines; a 300 µm strip; (1000, 1000); fifty flat and array.  PWB.c: a well
  overlapping, inside and around a block (PWB.d), a well abutting the block.  NBL.d: a
  well overlapping the nBuLay (a sinker) and one abutting it are one net.  Sal.e: a
  Cont inside the block (a Schottky's), a bar 0.195 away.
- PWB.e/e1/f/f1: the values; N+ in a well and under the block are not "in PWell"; the
  P+ half and the N+ half of one Activ; ThickGateOx over, abutting, over half.
- NBL.e/f: P+Activ is NBL.f's, not NBL.e's; the strapped and the well-tied taps (with
  finding 10's verdict).
- NBLB.c, Sal.c, CntB.c, CntB.d, CntB.g2, CntB.h1 (the enclosure kit): each side, all
  sides, a chamfer under and over (euclidian, settled), a coincident edge, the crossing
  shape; margins on and across the tile lines, a 300 µm strip, (1000, 1000); fifty flat
  and array.  Sal.c: an Activ and a GatPoly running out of the block at 0.195/0.2; the
  union (note D).
- CntB.a/a1: the bound both ways, the 0.17 square, halves, overlapping boxes, an
  abutting square, the 0.26 overlap, the 45° bar; bars on and across the tile lines, a
  0.155 × 300 bar, a 0.16 × 5 bar across x = 20; fifty of each flat and array.
- CntB.b: side by side, end to end, end to side, corner to corner, the two 5.5 bars
  (CntB.b and CntB.b1); tile lines; 300 µm bars; fifty.
- CntB.b1: 0.355/0.36 on 6 bars; runs of 5.0/5.005; offsets to 4.5/5.005; the jog; five
  bars; a T; runs cut by x = 20, 21, 7, 40, 42, y = 20; 300 µm bars; fifty.
- CntB.g: bare, past the Activ edge, half out of poly, the seam, a ring's hole,
  abutting, under poly (CntB.j), tile lines, fifty.  CntB.h: no Metal1, a 0.005 strip,
  half, a ring's hole, abutting, coincident, margins, halves, overlapping boxes, tile
  lines, two Metal1 boxes meeting on x = 20, fifty.  CntB.j: a gate, a 0.005 strip, a
  0.005 corner, two fingers, abutting, 0.14 away, tile lines, fifty.
