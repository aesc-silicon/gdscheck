<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / npn, sdiod: hardening report

The bipolar and Schottky decks against SG13G2 Layout Rules Rev. 0.4: section 6.1
(npnG2.b-e, the substrate tie; npn13G2.a, npn13G2L.a/b, npn13G2V.a/b, the emitter
lengths) and section 6.7 (Sdiod.a-e), with section 4.2's derived layers.  Every
mandatory rule of both sections is in its deck: npnG2.a defines the tie, npnG2.f allows
ties to overlap, the `*R` rules (npn13G2.bR, npn13G2L.cR, npn13G2V.cR, emitters per
chip) are recommended and deliberately left out.  45 layouts,
`tests/data/ihp-sg13g2/{npn,sdiod}/<RULE>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/npn_hardening.rs`, each with a `#[case]` in `tests/ihp-sg13g2.rs`.
Every layout ran through gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`ci/hardening/oracle-ihp.sh`).

The devices are drawn as IHP's reference cells (`libs.ref/sg13g2_pr/gds/sg13g2_pr.gds`)
have them.  The npn13G2 abstract is a TRANS box filling the hole of a pSD ring 0.9 wide
whose Activ ring 0.5 wide lies 0.2 inside the pSD on both edges (so the pSD's hole *is*
the TRANS, and the tie's Cont sits 0.27 from it); the abstract carries no EmWind, so the
emitter windows are drawn as the deck reads them, 0.07 (0.12 for V) wide inside the
TRANS, with the flavour label and the E-labelled Metal2 pin.  Where a neighbour must fit
in the hole, the hole is grown around the TRANS.  The schottky_nbl1 is a 0.30 x 1.00
ContBar under PWell:block, nSD:block and SalBlock at exactly 0.25, 0.40 and 0.45 in a
drawn nBuLay, ringed by an NWell whose hole is the PWell:block, a PWell:block ring, a
P+Activ tie ring, ThickGateOx and Recog:diode - the full stack, because KLayout's
recognition wants all of it (the tie ring's hole covering nBuLay-not-NWell under
Recog:diode, ThickGateOx, SalBlock and nSD:block), and with it KLayout reads Sdiod.a-e
on these layouts.

Reading the numbers: gdscheck gives one space marker per shape pair (a ring, a comb or
a U facing the TRANS is one), one enclosure marker per under-enclosed wall or one for a
closed run of them, one length marker per window or bar; the KLayout column adds the
driver's and the maximal deck's reports, so most rules count twice there, and its
hierarchical run counts an array cell once.  KLayout's emitter-length rules are dead
code (`ext_with_length` adds 1 µm, not 1 dbu, in its `>` branch: the minimum windows
are contradictory, the maxima fire only 1 µm late), so the npn13G2* column is 0
throughout and the oracle says nothing about the emitter rules.

Test status on the engine as of this report: 7 of the 45 new cases fail, all on
findings 1-6 below; the other 38 pass.  No count moved with the tile size except in
finding 5, and every flat/array pair agreed.  `helpers::shift` (and so `flat_array`)
now moves text elements with the boundaries; it left them behind before, which no
earlier generator noticed because none arrayed a labelled device.

## Resolution (2026-09-21)

Fixed, engine: 5 (a space report is one per pair of regions, whichever tiles read it:
the reports of one pair - known by the kinship index's id, or by a point of a shape
whole inside some core - keep the lowest, then leftmost; the SalBlock U is one at every
tile, as the ring and the comb were).

Fixed, deck: 1 (a second npnG2.b entry on `NpnTransOutsideTie`, the labelled TRANS
not lying whole in a labelled pSD hole: the crossing TRANS, the TRANS with no tie and
the broken ring fire; the pSD-only ring holds a hole and is left to the tie's own Activ
rule), 2 (npnG2.c reads the P+Activ *ring* round the hole - `PsdActivRings`, the P+Activ
with a hole, overlapping the grown hole - without `skip_coincident` or the extension
reading: the flush Activ and the one sticking out of the pSD are enclosed by nothing,
the island 0.45 away is not the tie's), 3 (`abutting: related` on npnG2.d's N+Activ
entry: a corner touch is related like an edge), 4 (npnG2.d reads `nBuLayDerived`,
section 4.2's nBuLay), 6 (the Schottky diode is a ContBar `inside` the four-layer stack,
"enclosed by", with the nBuLay drawn or generated).  The older emitter fixtures'
minimal devices - a labelled TRANS with no tie - are npnG2.b now and set it aside.

Open: 7 (section 6.7's "do not apply" list - NW.c1, NW.e1, PWB.f1, CntB.a, LU.d on a
recognised Schottky diode - lives in five other decks; an exemption layer subtracted
from each of those rules' layers is the way, not done here).

## Findings

### 1. npnG2.b reads only the labelled holes without a TRANS

Rule: npnG2.b, "NPN Substrate-Tie must enclose TRANS".

Layout `npn/npnG2.b.h2.gds.gz`: four devices whose tie does not enclose the TRANS - a
TRANS crossing the ring (half in the hole, half over the pSD and beyond), a labelled
TRANS with no tie at all, a TRANS in a ring with a 0.5 gap in its right wall, a TRANS in
a pSD-only ring (no Activ, so no tie by npnG2.a).  gdscheck: clean at 20, 7, 100.
KLayout: clean (only pSD.c1 and the pin rules).

Both tools implement "a labelled tie hole with no TRANS in it" (gdscheck's
`NpnTieNoTrans`, KLayout's `subst_tie_hole_w_npn.ext_interacting(TRANS, inverted)`).
The manual's sentence is the other direction: the TRANS must be enclosed.  A TRANS
sticking out of its ring, or with a broken ring, is a modified device and the rule that
says so is this one.  Verdict: four npnG2.b.  (The labelled empty holes of
`npnG2.b.h1` - four boxes, one cut polygon, an octagon, a hole with a P+ island - fire
in both tools, as do the six on the tile lines of h3 and the fifty of h4/h5.)

### 2. npnG2.c: an Activ edge on the pSD edge is no enclosure

Rule: npnG2.c, "pSD enclosure of Activ inside NPN Substrate-Tie 0.20".

Layout `npn/npnG2.c.h2.gds.gz`: (a) at (10, 10) the Activ ring flush with the pSD's
hole wall (its inner edge on the TRANS edge, enclosure 0 on that side); (b) at (24, 10)
the Activ ring sticking 0.1 out of the pSD's outer right wall (the P+ part ends on the
pSD edge, the bare 0.1 is N+Activ 0.9 from the TRANS); (c) at (38, 10) a P+Activ
island 0.45 from the hole's right wall with 0.19 pSD around it.  gdscheck at 20, 7,
100: one npnG2.c at (41.05, 9.7-10.3) - the island (c) - and one npnG2.d at
(26.45-27.35, 7.55) - the N+ sliver of (b), right.  KLayout: no npnG2.c; npnG2.d.N_Activ
twice on (b)'s sliver.

(a) and (b) are silent in both: gdscheck's `skip_coincident` and its
`PsdActiv = Activ AND pSD` (whose edge is the pSD's), KLayout's
`consider_intersecting_edges: false`.  The manual's enclosure of a flush edge is 0, and
0 < 0.20.  Verdict: (a) and (b) fire.  (c) is the deck's `NpnTieHoleGrown` (the hole
grown by 0.5) reading "Activ inside NPN Substrate-Tie" as any P+Activ near the ring;
the tie is the ring (npnG2.a), a separate island 0.45 away is not its Activ, and pSD.c
(0.18) is content with 0.19.  Verdict: (c) is clean.  The case expects
`npnG2.c, npnG2.c, npnG2.d`.

### 3. npnG2.d: a corner-point contact is read as unrelated

Rule: npnG2.d, "Min. unrelated N+Activ ... space to TRANS 1.21"; "unrelated - two
regions which do not touch each other" (glossary).

Layout `npn/npnG2.d.h2.gds.gz`, the device at (52, 10): an N+Activ abutting the
TRANS's right wall (gap 0) and an N+Activ box whose lower-left corner is the TRANS's
upper-right corner (53.5, 11.5).  gdscheck at 20, 7, 100: the abutting box is silent
(related), the corner-point box fires, `space 0.0000 µm < 1.21 µm ... at (53.5, 11.5)`.
KLayout reports both (its `ext_separation` counts touching pairs, the settled
difference).

Two regions sharing a point touch each other; by the glossary that is related, the same
as sharing a wall.  Verdict: clean; the case expects three npnG2.d (the drawn-nSD
N+Activ at 1.0, the nSD:block in the hole at 1.0, the nSD:block 1.205 outside a
reference tie).  Low weight - a kissing corner is a legitimate reading either way, but
the two contacts should read alike.

### 4. npnG2.d reads the drawn nBuLay, not section 4.2's

Rule: npnG2.d, "Min. unrelated ... nBuLay ... space to TRANS 1.21"; section 4.2,
nBuLay = ((NWell >= 3.0 sized by 1.0/side) OR nBuLay:drawing) AND NOT nBuLay:block (the
settled inward reading: the generated nBuLay lies 1.0 inside a wide well).

Layout `npn/npnG2.d.h8.gds.gz`: a 4.0 wide NWell overlapping the TRANS by 0.5 at
(10, 10) - the well touches, so it is related; its generated nBuLay begins 0.5 outside
the TRANS, does not touch it, and 0.5 < 1.21.  Beside it the same well under
nBuLay:block (26, 10), a 2.0 wide well (42, 10, no generated nBuLay) and the 4.0 well
with a drawn nBuLay overlapping the TRANS (58, 10; one region, related).  gdscheck:
clean at 20, 7, 100.  KLayout: `npnG2.d.nBuLay` once, `(12, 9-11)/(11.5, 8.5-11.5)`,
the first device only (its `nBuLayGen_nBuLay = nBuLay OR generated`).

The deck's npnG2.d entry names `nBuLay`; the filler rules already read the derivation.
Verdict: one npnG2.d.  (A well entirely outside the TRANS is caught by the NWell entry
first - the generated nBuLay is never closer than its well - so this only shows on a
well that overlaps or abuts the TRANS.)

### 5. npnG2.d2: a U's count depends on the tile

Rule: npnG2.d2, "Min. unrelated SalBlock space to TRANS 0.90".

Layout `npn/npnG2.d2.h1.gds.gz`, the device at (38, 10): a SalBlock U, one polygon,
whose two arms run over the TRANS's top and bottom walls at 0.895 (x = 37 to 41.4,
joined by a bar at x = 41.0-41.4, 1.5 from the TRANS's right wall).  gdscheck: 3
markers at tile 20 (the 0.895 bar of the device at (10, 10) plus `(37, 7.605-8.5)` and
`(37, 11.5-12.395)`), 2 at tiles 7 and 100 (the U once, `(37, 7.605-8.5)`).  Also 3 at
tiles 10, 13 and 19, 2 at 21 and 50: the U counts twice whenever a tile line (x = 38,
39, 40) cuts it, once when none does.  KLayout: 8 (four edge pairs, twice).

Elsewhere in this deck one polygon facing the TRANS is one marker at every tile: the
NWell ring around the TRANS at 1.205 of `npnG2.d.h4` (x = 35.295 to 40.705, cut by
x = 40 at tile 20) and its three-tooth comb both count once at every tile.  Verdict: the case expects two npnG2.d2,
the engine's own per-pair count; what matters is that it does not move with the tile.

### 6. Sdiod: recognition is "ContBar enclosed by", and section 4.2's nBuLay

Rule: section 6.7, "schottky_nbl1 = ContBar enclosed by (SalBlock and nSD:block and
PWell:block and nBuLay)"; Sdiod.d, "Min. and max. ContBar width inside nBuLay 0.30".

Layout `sdiod/Sdiod.d.h2.gds.gz`: three bars in the full stack, none a diode by the
manual's sentence - a 0.295 wide bar with no nBuLay at all (10, 10); a 0.295 bar the
drawn nBuLay's edge cuts through lengthwise, x = 20 (20, 10); a 0.3 x 1.0 bar whose
right wall sticks 0.1 out of the PWell:block (30, 10).  gdscheck at 20, 7, 100: one
Sdiod.d, `width 0.2950 at (19.9975, 10)`, the half-covered bar.  KLayout: the same one
Sdiod.d (driver, `(19.85-20.145, 9.5-10.5)`); the bar crossing the block is silent in
both.

The deck's `SchottkyContBar` is the ContBar *overlapping* the four-layer intersection;
KLayout's is any ContBar in the tie ring's hole.  The manual's word is "enclosed".
The bar crossing the block is consistent - no Sdiod.a from either tool - and the first
bar is no diode in either.  Verdict: clean; the case expects nothing.  This is a
reading question rather than a bug, and the other decks (CntB, nSDB, PWB) take over an
unrecognised bar; it is recorded so the recognition is decided once.

Layout `sdiod/Sdiod.d.h3.gds.gz`: a 0.295 bar in the SalBlock/nSD:block/PWell:block
stack at their exact margins inside a 6.0 wide NWell and no drawn nBuLay.  Section
4.2's generated nBuLay (the well inset by 1.0) encloses the stack.  gdscheck: clean.
KLayout: clean (it cannot recognise a diode without its markers).  Verdict: by 4.2 a
diode, one Sdiod.d; low weight, the same question as finding 4.

### 7. Section 6.7's "do not apply" list

Rule: section 6.7, "The following rules do not apply: NW.c1, NW.e1, PWB.f1, CntB.a,
LU.d".

The oracle runs the core suite, so every Schottky layout also went through the other
decks.  On the seven reference-exact diodes of `sdiod/Sdiod.a.h1.gds.gz` (the only
deviation each is one PWell:block margin) gdscheck reports CntB.a x7, PWB.f1 x7,
NW.e1 x14, LU.d x7, and nSDB.e x7 (its Cont is in the nSD:block by construction);
KLayout reports none of these on a recognised diode (it does report CntB.a and nSDB.e
on the unrecognised no-nBuLay bar of `Sdiod.d.h2`).  NW.c1 did not come up (no P+Activ
in the NWell ring).  The exemptions belong to the contbar, pwellblock, nwell, lu and
nsdblock decks, not to this one; recorded here because section 6.7 is where the manual
states them.  Also seen there: the NW.e1 and PWB.f1 counts on `Sdiod.a.h2`, `h3`, `h5`
and `h6` move with the tile (NW.e1 15/14/14 on h3, PWB.f1 50/48/50 on h5) - for those
decks' hardening.

## Observations, both tools agreeing or KLayout alone

- **nSD:block in the tie's hole.** `npn/npnG2.d.h2.gds.gz`, device (80, 10): an
  nSD:block in the hole 1.0 from the TRANS.  gdscheck fires; KLayout's rule is
  `nSD_block.ext_outside(subst_tie_hole_w_npn).ext_separation(...)`, a block in the hole
  is exempt.  The manual has no such exemption; gdscheck is right.
- **Two bars in one Schottky stack.** `sdiod/Sdiod.a.h4.gds.gz`: two 0.3 x 1.0 bars 0.5
  apart under one PWell:block/nSD:block/SalBlock at the exact margins around the pair.
  gdscheck: Sdiod.a, b, c max twice each (each bar's inner side is enclosed by 1.05,
  1.2, 1.25).  KLayout: silent - its max is "block outside the bars sized by the value",
  empty for a gap up to twice the value, and it has no Sdiod.a max at all.  The
  manual's figure has one bar; "enclosure of ContBar" per bar gives gdscheck's answer,
  which the case expects.
- **KLayout's emitter rules** are dead (above); gdscheck's min/max lengths at 0.895,
  0.9, 0.905 (G2), 0.995, 1.0, 2.5, 2.505 (L), 0.995, 1.0, 5.0, 5.005 (V) are as the
  table says, orientation does not matter, the 0.9 x 0.9 square is a 0.9, a window of
  two abutting halves is one window, a window crossing the TRANS's wall is read whole
  (0.9 clean, 0.895 fires), ten in a row count ten, "npn13G2L" is not "npn13G2" and
  "npn13G2C"/"npn13G2X" are no flavour, and a labelled TRANS without a tie still gets
  its length checked.  A 45° window was not drawn: 0.07 x 0.9 has no on-grid rotation.
- **Recognition by label.** A TRANS in a tie whose hole carries no `npn*` text gets no
  npnG2 rule in either tool (`npnG2.d.h2` device (66, 10), `npnG2.d1.h1` (66, 10),
  `npnG2.e.h1` (80, 10)); a label in the hole but outside the TRANS gives the tie but
  no flavour (`npn13G2.a.h2`).  As section 6.1 says ("TRANS layer in combination with
  TEXT labels"), noted so nobody expects otherwise.

## Tested and clean

- npnG2.b: labelled empty rings as boxes, a cut polygon, an octagon; an unlabelled
  empty ring silent; straddling x = 20 and 100, walls on 21, 40, 42, at (1000, 1000);
  fifty flat and as an array.
- npnG2.c: 0.195 on one outer wall, on one hole wall, all round (one run), an octagonal
  tie's four diagonals at 0.1945, a pSD corner chamfered 0.1945 from the Activ corner
  (euclidian, settled); 0.2 as boxes, as one cut polygon and as an octagon clean; the
  0.195 gap straddling 20, 40, 100, walls on 21 and 42, at (1000, 1000); fifty flat and
  as an array.  KLayout agrees everywhere (its npnG2.c wants the TRANS touching the pSD,
  which the reference geometry gives).
- npnG2.d: each of N+Activ, NWell, PWell:block, nBuLay, nSD:block at 1.205 fires and at
  1.21 is clean; P+Activ at 1.0 clean; Activ under an abutting nSD:block with no nSD is
  no N+ (4.2), with drawn nSD it is; abutting N+Activ and overlapping NWell related;
  corner to corner 0.85/0.85 (1.202) fires and 0.86/0.86 (1.216) does not; a chamfer
  1.205 from the TRANS corner, a diamond tip at 1.205 and a 45° NWell strip at 1.205
  fire; 1.205 in x with 2.0 in y past the corner clean; two overlapping boxes one
  marker, a three-tooth comb one, a ring around the TRANS one; two ties sharing a wall
  (npnG2.f) with an NWell 1.205 from one TRANS, one; the gap straddling 20, 42, 100,
  walls on 21 and 40, at (1000, 1000); fifty flat and as an array.
- npnG2.d1: GatPoly 0.895 fires, 0.9 clean, in SRAM at 0.5 clean, the part outside SRAM
  at 0.5 fires, abutting clean, a 45° strip at 0.895 fires, 0.63/0.63 (0.891) fires and
  0.64/0.64 (0.905) does not; the tile lines and (1000, 1000).
- npnG2.d2: 0.895 fires, 0.9 clean, abutting and overlapping clean; fifty flat and as
  an array.
- npnG2.e: 0.265 fires, 0.27 clean, abutting and inside the TRANS clean, 0.185/0.185
  (0.262) fires and 0.195/0.195 (0.276) does not, a 0.16 x 0.5 ContBar at 0.265 fires,
  the tie's own Cont 0.07 inside the reference Activ ring (0.27) clean; the tile lines
  and (1000, 1000); fifty flat and as an array.
- npn13G2.a: as under the emitter observation; a 0.895 window straddling x = 20,
  crossing 21, ending on 40, starting on 42, straddling 100, at (1000, 1000); fifty
  flat and as an array.
- Sdiod.a/b/c: one wall one step under (min) and over (max), exact clean, two opposite
  walls under (two), all four under (one run), all four over (one), a chamfered block
  corner passing one step under the value from the bar's corner (euclidian, settled);
  the PWell:block as four overlapping boxes clean, a plate 2.0 past the bar and one 0.8
  past it under the NWell ring fire the max, a separate block 0.5 away is not the
  bar's; the 0.245 margin with the block wall on x = 20, the bar straddling 21, the bar
  wall on 40, the gap straddling 42, the bar's top wall on y = 20, straddling 100, at
  (1000, 1000); fifty flat and as an array.  KLayout agrees on every Sdiod.a-c min and
  on the b/c max of single bars (its Sdiod.a has no max).
- Sdiod.d/e: 0.295 and 0.305 wide fire, 0.995 and 1.005 long fire, a 0.3 x 0.305 bar
  fires the length, a 0.295 of two overlapping boxes fires once; a lying 1.0 x 0.3 is
  clean, a 0.3 x 0.3 square is a Cont and no bar, a 0.3 x 1.0 of two abutting halves is
  one bar.  KLayout agrees on all.
