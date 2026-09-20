<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / gatpoly: hardening report

Deck `gatpoly` against SG13G2 Layout Rules Rev. 0.4, section 5.8 (Gat.a-Gat.g) and section
5.9 (GFil.a-GFil.j), with the derived layers of section 4.2 (Gate, NFET, PFET).  92
layouts, `tests/data/ihp-sg13g2/gatpoly/<rule>.h<n>.gds.gz`, drawn by
`gen/ihp_sg13g2/gatpoly.rs` (`hardening`), each with a `#[case]` in the `gatpoly` table of
`tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and through
IHP's KLayout decks (`ci/hardening/oracle-ihp.sh`).  `show-deck` lists every rule of both
sections.

Reading the numbers below: gdscheck reports one marker per wall for `min_width` and the
gate-length checks (two per narrow gate), one per pair otherwise; KLayout's column adds the
driver's and the maximal deck's reports, and cuts corner pairs into two edge pairs.
KLayout's driver runs Gat.a, Gat.b, Gat.d, Gat.a1, Gat.a2, Gat.g, GFil.d, GFil.e and
GFil.i once, and its maximal table (also the oracle's maximal run) Gat.a3, Gat.a4, Gat.b1,
Gat.c, Gat.e and Gat.f, so those count twice; GFil.a/b/c/f/j run once (the driver's
maximal table).  Every flat/array pair agreed in both tools; no count moved with the tile
size except in finding 10.

Test status on the engine as of this report: 28 of the 92 new cases fail, all on the
findings below (finding 10 fails at tiles 7 and 100 only); the other 64 pass, as do the 26
older cases.

## Resolution (2026-09-20)

Fixed, engine: 6 (`length` on a 45° pair is each wall's own length, as KLayout's
`with_length`, not the stretch the two share), 7 (density layers read as their union -
the activ report's finding 9), 10 (GFil.a reads `span: narrowest`, one report per
stitched region, whatever the tile).  Fixed, deck: 1 (the NFET gate is GatPoly over `NActFull`, N+ by drawn nSD or by default), 2
(Gat.f is a `forbidden` on the gate regions that are no rectangle, KLayout's reading,
one report per gate), 3 (a `forbidden` on the polys lying whole inside the active, as
KLayout's `GatPoly.inside(Activ)`, for Gat.c and GFil.j), 4 and 5 (`abutting: report` on
Gat.d, GFil.d, GFil.e and GFil.f).

Settled on KLayout's reading, cases flipped: the nSD:block gate of finding 1 (note A: no
N+, no NFET), the stub ending inside the active of finding 3 (no cap to read in either
tool), the overlap halves of finding 5 (shared area is no pair), 8 (Gat.b1 between gate
regions, both tools), 9 (a pSD gate is a PFET's without a well, as the maximal deck has
it), the straight gate over a stepped active (note E: its gate region is no rectangle),
the chamfered L of note G (one wall under 0.39), and the half-oxide gate of Gat.a3.h1
(read between the poly's walls only; a TGO edge through a gate is TGO.c's).  The 50 x
5.005 x 5 fillers of GFil.a.h3/h4 are 5.005 x 5.005 now, since 5.005 x 5 is 5.0 wide.

Open: 11.  KLayout's projection metric pairs a wall at an angle - each wall cut to the
part within the value of the other, perpendicular to the other and with its feet on it,
the margin their closest approach (0.141 on the chamfered cap, the perpendicular to the
chamfer; a chamfer passing a corner has its feet beyond the wall and is no pair, so the
nwell report's settled reading holds).  A reading of it was built and taken out again:
measured on the enclosed shape's piece in a tile it depended on where the tile line cut
the wall, and it moved four GF180 counts (LPW.3 +2, PRES.6/LRES.6 +2, S.CO.4_LV +1 - all
corners lying on or nanometres inside a slanted outer wall) whose standing needs a
look of their own.  The case carries the parallel-walls reading.  On the way the PL.7
good pattern turned out to be a PL.4 violator by KLayout's euclidian PL.4 (its 45°
gate's wall passed 0.14 from the active's corner); the bar starts 0.15 further in now.

## Findings

### 1. Gat.a1 and Gat.a3 do not see an NFET drawn without nSD (false negative, the common case)

Manual, section 4.2: "NFET  GatPoly over (Activ AND NOT pSD)"; nSD is "NOT (pSD OR nSD:block)
OR nSD:drawing", and note 1 says nSD:drawing is only valid where pSD and nSD are identical
(the rhigh resistor).  Every ordinary NFET is a bare Activ under GatPoly with no nSD drawn;
IHP's driver derives its NFET gate that way (`nactiv = activ NOT (pSD OR nSD:block)`).

The deck runs Gat.a1 on `GatPolyOverNsdActivNoTGO` and Gat.a3 on `GatPolyOverNsdActivTGO`,
both `[GatPoly, Activ, nSD]` with the drawn nSD.

Layouts `Gat.a1.h1`: 0.125 gates on a bare Activ, with nSD drawn, under nSD:block;
`Gat.a1.h2`-`h6` and `Gat.a3.h1`-`h4`: every other NFET pattern drawn bare (a dumbbell, a
notched gate, four fingers, gates across tile lines, fifty flat and as an array, a 300 µm
wide one, one at (1000, 1000), 0.445 gates under ThickGateOx).

- gdscheck @20/7/100: Gat.a1 and Gat.a3 only for the gate with nSD drawn (2 markers each
  in `h1`, at x = 10.9/11.025 and 10.75/11.195); nothing in any other layout.
- KLayout: Gat.a1 for the bare and the nSD gate of `Gat.a1.h1`, and for every gate of
  `h2`-`h6` (4, 8, 50, 1 [the array cell], 3); Gat.a3 in `Gat.a3.h1` for the bare, the nSD
  and the half-oxide gate (6) and in `h2`-`h4` (12, 100, 51); Gat.a3 for the 3.3 V NFET at
  0.395 in `Gat.a4.h1`.
- Verdict: the NFET is Activ AND NOT pSD under GatPoly.  Expected Gat.a1 × 6 in `gat_a1_h1`
  (bare, nSD, nSD:block - the manual's NFET has no nSD:block term; KLayout leaves that one
  out, note A), 8/16/100/100/6 in `h2`-`h6`; Gat.a3 × 8 in `gat_a3_h1` and 12/100/100 in
  `h2`-`h4`; Gat.a3 × 2 in `gat_a4_h1`.  The older `Gat.b1` fixture (0.3 gates under
  ThickGateOx, "no implant, so the gate-length rules don't apply") is a Gat.a3 too by this
  reading; its case was left alone.

### 2. Gat.f does not see a 90° bend (false negative)

Manual: "Gat.f  45-degree and 90-degree angles for GatPoly on Activ area are not allowed".
The deck runs `no_angle` on `GatPolyOverActiv`, which reports 45° edges only.

Layout `Gat.f.h1`: an L gate bending inside the Activ, a T (a stub inside the Activ), a gate
with a 0.035 notch over the channel, a gate stepping from 0.3 to 0.5 inside the Activ.
`Gat.f.h2`: an L whose bend is 0.005 inside the Activ.  `Gat.f.h4`: L gates cornered on
x = 20 and 40, straddling 42, at (1000, 1000).  `Gat.f.h5`/`h6`: fifty L gates, flat and as
an array.

- gdscheck: nothing for any 90° bend; the 45° gates of `h3`/`h4` are reported, one marker
  per 45° gate edge.
- KLayout: Gat.f on every bent gate (8, 4, 10, 100, 51 = twice the bent gates; its rule is
  "the gate region is not a rectangle").
- Verdict: all fire.  Expected 4 in `gat_f_h1`, 1 in `gat_f_h2`, 6 in `gat_f_h4` (four Ls
  and the two edges of a 45° gate), 50 in `gat_f_h5`/`h6`; one marker per bent gate.

### 3. Gat.c and GFil.j miss a gate that ends inside the Activ (false negative)

Manual: "Gat.c  Min. GatPoly extension over Activ (end cap)  0.18", "GFil.j  Min.
GatPoly:filler extension over Activ:filler (end cap)  0.18".  A gate that stops short of
the Activ edge has no end cap at all.

Layout `Gat.c.h2`: a gate stub ending 0.4 inside the Activ (poly (5.3, 1.82)-(5.6, 2.6) on
Activ (5, 2)-(6, 3)) and a poly wholly inside the Activ ((8.3, 2.3)-(8.6, 2.7)).  Layout
`GFil.j.h1` has the same two for GatPoly:filler over Activ:filler ((14.1, 1.82)-(14.9, 3.0)
and (17.1, 2.3)-(17.9, 3.2) on 1 × 1.5 Activ:fillers).  Beside them a gate ending exactly
on the Activ edge (cap 0.00) and a gate whose side lies on the Activ's right edge.

- gdscheck: Gat.c for the 0.00 cap and for the side on the edge (both "enclosure 0.0000"),
  and for the 0.175 corner; nothing for the stub or the inside poly.  GFil.j the same (7 of
  9).
- KLayout: Gat.c for the inside poly ("polygon: (8.3,2.3;...)" from `GatPoly.inside(Activ)`,
  its second Gat.c check) but not for the stub; GFil.j has no such half and reports the
  same 7 as gdscheck.
- Verdict: a negative extension is under 0.18.  Expected 5 in `gat_c_h2` (0.00, stub,
  inside, the 0.175 corner, the side on the edge) and 9 in `gfil_j_h1`.

### 4. Gat.d does not see a poly abutting the Activ (false negative)

Manual: "Gat.d  Min. GatPoly space to Activ  0.07"; the glossary: "abut - two edges of two
different layers touching each other".  A touching edge is a space of 0.00.

Layout `Gat.d.h3`: poly (3.0, 2.3)-(3.3, 2.6) abutting the right edge of Activ (2, 2)-(3, 3);
poly (6, 3)-(6.3, 3.3) touching Activ (5, 2)-(6, 3) at the corner point only.

- gdscheck: the corner point fires ("space 0.0000 at (6, 3)"); the abutting edge is silent.
- KLayout: both ("(3,2.3;3,2.6)/(3,2.67;3,2.23)" and two edge pairs at (6, 3)).
- Verdict: both are Gat.d.  Expected 6 in `gat_d_h3`.  The filler rules show the same
  class, finding 5.

### 5. GFil.d, GFil.e and GFil.f do not see a filler touching or overlapping the other layer (false negative)

Manual: "Min. GatPoly:filler space to Activ, GatPoly, Cont, pSD, nSD:block, SalBlock 1.10",
"... to NWell, nBuLay 1.10", "... to TRANS 1.10".  A filler under a pSD or over a GatPoly is
at distance zero from it.

Layouts `GFil.d.h2` (a filler abutting an Activ, overlapping a pSD by 0.2, wholly under a
pSD, overlapping a GatPoly), `GFil.e.h2` (abutting an NWell, wholly inside an NWell, half
over an nBuLay), `GFil.f.h1` (abutting a TRANS, inside a TRANS).

- gdscheck: nothing for any of the nine; the 1.095 gaps beside them all fire.
- KLayout: the three abutting ones ("(3,2;3,3)/(3,3;3,2)" in `GFil.d.h2`, "(3,2;3,4)/(3,3;3,2)"
  in `GFil.e.h2`, "(3,7;3,6)/(3,6;3,7)" in `GFil.f.h1`); nothing for the six overlaps (its
  `sep` does not report overlapping shapes).
- Verdict: all nine fire.  Expected 10 in `gfil_d_h2`, 7 in `gfil_e_h2`, 8 in `gfil_f_h1`.
  The NW.d resolution ("external N+ meeting the well is a `forbidden`") is the same
  answer: a `min_space` needs a pair, and touch or overlap has none.

### 6. Gat.g measures the bend length along an axis, not along the edge (false negative)

Manual: "Gat.g  Min. GatPoly width for 45-degree bent shapes if the bend GatPoly length is
> 0.39 µm  0.16".  The bend's length is the length of its 45° edge.

Layout `Gat.g.h1`: 0.1591-wide 45° bands (`band45`, `wt` 0.225) rising h = 0.45, 0.28, 0.39,
0.40, 0.385 and 0.275, i.e. with 45° walls 0.636, 0.396, 0.552, 0.566, 0.544 and 0.389 long.

- gdscheck: Gat.g for h = 0.45, 0.39 and 0.40 (two markers each, at x = 5, 14, 17);
  nothing for h = 0.28 (walls 0.396) or h = 0.385 (walls 0.544).  The threshold sits between
  h = 0.385 and h = 0.39: the `length: 0.39` parameter is compared with the wall's x (or y)
  extent, a factor √2 short of its length.
- KLayout: the five bands with walls over 0.39 (`with_length(0.39, nil)` takes the edge
  length); h = 0.275 clean.
- Verdict: walls 0.396 and 0.544 long are bends longer than 0.39.  Expected 10 in
  `gat_g_h1` (five bands); h = 0.275 stays clean.

### 7. GFil.g counts GatPoly and GatPoly:filler twice where they coincide (false negative)

Manual: "GFil.g  Min. global GatPoly density [%]  15.00"; the deck's layers are `[GatPoly,
GatPoly.filler]`.

Layout `GFil.g.h1`: a 1000 × 1000 EdgeSeal boundary, a 10 % GatPoly stripe and a 10 %
GatPoly:filler stripe over the same area.

- gdscheck: "Density: 20.00%", no violation.
- KLayout: the oracle runs without density.
- Verdict: the poly covers 10 % of the boundary.  Expected GFil.g (`gfil_g_h1`).  Fillers
  may not overlap GatPoly (GFil.d), so a legal layout never shows it; a density over
  several layers still ought to take their union.

### 8. Gat.b1 does not read the figure: the gate polys of two 3.3 V transistors (false negative, both tools)

Manual: "Gat.b1  Min. space between unrelated 3.3 V GatPoly over Activ regions  0.25".
Figure 5.8 draws b1 between the *ends of the two gate polys* of two separate transistors
under ThickGateOx, well outside both Activs.

Layout `Gat.b1.h2`: two 3.3 V transistors side by side, their horizontal gate polys ending
0.245 apart (poly (1.7, 2.7)-(3.3, 3.2) and (3.545, 2.7)-(5.145, 3.2), Activs 0.845 apart);
the same at 0.25; and one poly crossing two Activs 0.245 apart (gate regions 0.245 apart).

- gdscheck: Gat.b1 once, for the two Activs ("(16, 2.25)-(16.245, 2.25)"); the figure's pair
  is silent (its `GatPolyOverActivTGO` regions are 0.845 apart).
- KLayout: the same (`GP_mosHV = Gate.not_outside(ThickGateOx)`, spaced 0.25).
- Verdict: by the figure the rule is the poly-to-poly space of 3.3 V gates, 0.25 in place of
  Gat.b's 0.18, and the first pair fires.  Expected 2 in `gat_b1_h2`.  This reads the figure
  against a rule text both tools read the other way; if the PDK owner prefers the region
  reading the case should drop to 1.

### 9. Gat.a2 and Gat.a4 take a pSD gate without an NWell for a PFET (false positive, pedantic)

Manual, section 4.2: "PFET  GatPoly over ((Activ AND pSD) inside NWell)".

Layouts `Gat.a2.h1` and `Gat.a4.h1`: a pSD Activ with no NWell, and a pSD Activ crossing the
well edge, gates 0.125 (a2) and 0.395 (a4), beside proper PFETs.

- gdscheck: Gat.a2 and Gat.a4 for all three devices each (`GatPolyOverPsdActiv` has no NWell
  term).
- KLayout: Gat.a2 for the PFET and the crossing device (its driver clips `pactiv AND
  nwell`); Gat.a4 for all three (the maximal deck's PGate is any gate that is not an NGate).
- Verdict: by section 4.2 neither odd device is a PFET; for a2 it hardly matters (Gat.a
  fires at 0.125 anyway), for a4 a 0.395 pSD gate without a well is reported under a rule
  that does not apply.  Expected Gat.a2 × 2 + Gat.a4 × 2 in `gat_a2_h1`, Gat.a4 × 2 +
  Gat.a3 × 2 in `gat_a4_h1`.  Least important here: the devices are broken in other ways
  (NW.c fires on the crossing one).

### 10. GFil.a's count depends on the tile size for a merged bar (TILE-DEPENDENT)

Layout `GFil.a.h5`: a 300 × 5.005 GatPoly:filler bar merged with a 13 × 13 ring hanging
below it at x = 46..59 (hole 3 × 3), and a second bar merged with a 5 × 5.5 box at
x = 46..51.

- gdscheck @20: 22 markers; @7: 23; @100: 26.  At 100 the extra markers are partial copies
  of the bars' long edges, cut where the hanging shape joins: "(2, 12)-(46, 12)",
  "(2, 17.005)-(46, 17.005)", "(2, 30)-(46, 30)", "(2, 35.005)-(46, 35.005)", next to the
  full-length "(2, 12)-(302, 12)" that every tile size reports.
- KLayout: 2 (one polygon per merged shape, from its 2.5 shrink, note B).
- Verdict: whatever the marker cut, it must not move with the tile.  `gfil_a_h5` expects the
  tile-20 count (22) and fails at 7 and 100.  The first draft of `GFil.a.h1` had the bar
  crossing the ring by accident and showed the same (46/46/48).

### 11. Gat.c passes a cap whose corner is chamfered below the value (false negative)

Manual: Gat.c 0.18, the extension over Activ.

Layout `Gat.c.h3`: a gate (5.3, 1.82)-(5.6, 3.18) over Activ (5, 2)-(6, 3) whose top-right
corner is cut along (5.6, 3.1)-(5.52, 3.18): at x = 5.6 the poly ends 0.10 above the Activ
edge.  Beside it the same chamfer at 0.19 (clean).

- gdscheck: silent (6 of 7 in the layout).
- KLayout: Gat.c, "(5.445,3;5.5,3)/(5.52,3.18;5.6,3.1)".
- Verdict: the cap is 0.10 at the corner.  Expected 7 in `gat_c_h3`.  This is the
  projection-metric question of the nwell report (settled reading 1) seen from the end of
  the gate: here the chamfer faces the Activ's own edge, not a corner of it, and IHP's
  `ext_enclosed` does report it.

## Notes that are not findings

- A. Gat.a1 on an Activ under nSD:block (`Gat.a1.h1`, x = 6..8, y = 6..7): the manual's NFET
  is "Activ AND NOT pSD", so the case expects Gat.a1; KLayout leaves it out (its NFET is
  Activ NOT (pSD OR nSD:block)) and gdscheck too.  An NFET without its implant is no
  device; the case follows the letter and the count is in finding 1.
- B. GFil.a "max width": gdscheck reports every pair of opposite edges more than 5.00
  apart, so a 5 × 6 filler, a 1 × 300 stripe and an L spanning 10 all fire; KLayout's deck
  shrinks by 2.5 and reports only shapes over 5.00 in both directions (2 in `GFil.a.h1`:
  the 5.005 × 5.005 box and the 300 × 5.005 bar).  The manual's width (figure 4.1) is the
  distance between opposite edges, and IHP's own filler is 5 × 1.4, the largest box under
  either reading.  The cases follow gdscheck (`GFil.a.h1`/`h2`, 28 and 8 markers, named
  in the case comments), and the 300 µm bars in the other GFil layouts are ignored as
  GFil.a.
- C. GFil.c "space" (not "space or notch"): a U-shaped filler with a 0.795 slot
  (`GFil.c.h1`) is clean in gdscheck and expected clean; KLayout's `space` reports it
  ("(3.795,7;3.795,9)|(3,9;3,7)").  Fillers are boxes, so nothing hangs on it.
- D. GFil.e and the derived nBuLay: section 4.2 makes "(NWell ≥ 3.0 µm) sized by 1.0
  µm/side" part of nBuLay; IHP's driver reads the sizing inward (`sized(-1.495).sized(0.495)`),
  so a filler 1.6 from a 5 × 5 NWell is clean (`GFil.e.h1`); both tools agree.
- E. A straight 0.5 gate over an L-shaped Activ whose step lies under the gate
  (`Gat.f.h2`): KLayout reports Gat.f (its gate region is not a rectangle), gdscheck does
  not, and the case expects nothing: the GatPoly has no angle on the Activ there.  Both
  tools report the step edge as a Gat.c when the gate is narrower than 0.18 + step, which
  is right.
- F. A gate whose side runs along the Activ edge (`Gat.c.h2`, poly (3.7, 5.7)-(4.0, 8.3) on
  Activ (2, 6)-(4, 8)): both tools report Gat.c with enclosure 0.00 and the case counts
  it: the Activ edge under the poly is extended over by nothing.
- G. Gat.g on a chamfered L (`Gat.g.h2`, outer 45° edge 0.495, inner 0.368, 0.1556 apart):
  the case expects Gat.g, reading the bend's length on its outside; both tools are silent
  (KLayout pairs only edges that are each ≥ 0.39).  A reading; the Z routes beside it
  (both walls 0.636) fire in both tools, up and mirrored down.
- H. Marker cuts: a ring whose outer walls are 5.005 apart gives four GFil.a markers (each
  wall split by the hole); a 300 × 5.005 bar four (two per dimension); KLayout cuts every
  corner-to-corner pair into two edge pairs and a grid-to-bar gap into one per grid box.
  None of it is a logical difference.
- I. The 45° strips of `Gat.a.h2` (3 µm long, 0.127 and 0.134 wide) are Gat.g in both tools:
  a long 45° strip is a "45-degree bent shape" of length > 0.39.  Ignored there.
- J. GFil.i: KLayout's rule is a 400 × 400 bounding box (it reports the 800 × 199 bar, the
  L and the union in `GFil.i.h1`); the manual's column says "area (µm²)" and gdscheck reads
  the area (160000).  The case follows the manual.
- L. A U-shaped 3.3 V poly whose base lies over the Activ (`Gat.b1.h3`, second U) is one
  gate region; its legs are related and the case expects no Gat.b1 there (gdscheck
  agrees).  KLayout reports the 0.245 between the legs (its `space` includes notches).
- K. KLayout reports Gat.d on 45° polys that cross the Activ edge (`Gat.f.h3`/`h4`, the
  existing `Gat.f`); there is no space there and gdscheck is right to be silent.

## Tested and found clean or correct (no need to redo)

- Gat.a: 0.13 vs 0.125 in x and y; 300 µm bars; a 0.005 sliver; (1000, 1000); diamond and
  45° strip 0.134/0.127; chamfered box and L; unions of overlapping/abutting/gridded boxes;
  a dogbone neck; a ring wall; an island in a ring; bars on/straddling x = 20/21/40/42 and
  an L cornered on 20; fifty flat and as GdsArrayRef; a comb and a U.
- Gat.a1/a2/a3/a4 (with the implant gdscheck reads): 0.13/0.125, 0.45/0.445, 0.40/0.395;
  a dumbbell; a notched gate; a 0.5 gate on a 0.12-tall Activ is not a 0.12 gate; four
  fingers; gates on/straddling tile lines and a horizontal gate across x = 20; fifty
  flat/array; 300 µm wide; (1000, 1000); a gate over a 0.005 sliver of Activ; the 1.2 V
  rules do not fire under ThickGateOx and the 3.3 V ones do not fire without it; a PFET at
  0.445 is not a Gat.a3; a gate with the oxide over half of it is a 3.3 V gate (both tools).
- Gat.b: 0.18/0.175; diagonal 0.184/0.177; corner-on; half-overlapping 0.18; diamond tip,
  parallel 45° strips, chamfer to corner, tip to tip; every notch shape (U, comb, slot,
  Ls, ring hole, island) - `min_notch` is there and right; unions and a grid; tile lines
  incl. a corner pair on (20, 20); fifty flat/array; gate fingers 0.175 apart; a poly past
  a cap; sliver; 300 µm; (1000, 1000).
- Gat.b1: 0.25/0.245 on one Activ; 1.2 V fingers at 0.245 are clean; a finger outside the
  oxide beside a 3.3 V one is clean; 0.175 is Gat.b1 and Gat.b; a U poly with two gate
  regions fires, a U over the Activ is one region; tile lines; (1000, 1000); 300 µm gate
  regions; fifty flat/array.
- Gat.c: 0.18/0.175 top, bottom, both, horizontal; 300 µm Activ and 300 µm gate; cap 0.00;
  a poly over an Activ corner 0.5/0.175; a T head; a gate over two Activs; caps across
  x = 20/40/42, a horizontal cap across 20, an Activ edge on 20; fifty flat/array; (1000,
  1000); an Activ seam under the gate; a gate drawn as two boxes counts once; SRAM excluded
  (deck).
- Gat.d: 0.07/0.065; diagonal 0.0707/0.0636; corner-on; above/below; diamond tip; 45° wall
  past a corner; box corner to an Activ chamfer; parallel 45° walls; corner-point touch;
  a gate past a second Activ; a gate short of its Activ; a poly in a U (two markers); an
  overlap is a gate, not a space; tile lines incl. a corner pair on (20, 20); (1000,
  1000); 300 µm; fifty flat/array.
- Gat.e: 0.09/0.0885; a 0.0897 bar; 0.0882/0.0925 diamonds; unions (once), abutting halves,
  a grid; a ring's hole counts against its area; an L; squares across tile lines (the
  area is the whole shape's); (1000, 1000); a 300 µm bar; fifty flat/array.
- Gat.f: 45° gates (a bend inside, a jog, a corner clip, a full crossing across x = 20);
  a 45° bend outside the Activ, a chamfer outside, a 45° poly beside an Activ, a bend 0.07
  outside are clean; a stepped Activ under a straight gate (note E).
- Gat.g: 0.1626/0.1591 on 0.636 walls; 0.389 walls clean; Z routes up and mirrored down
  (clockwise); 0.354 jogs clean; the jog straddling x = 20, starting on 20, across 40/42,
  at (1000, 1000); fifty flat/array.
- GFil.a: 5 × 5 vs 5.005 in x, y and both; unions; an L spanning 5/5.005; a 4 × 4 grid; a
  ring 5/5.005; squares across x = 20/40, (1000, 1000); diamonds 4.95/5.02; strips 4.95
  wide, 5.02 wide, 5.09 long; a chamfered box; fifty flat/array.
- GFil.b: 0.70/0.695 in x and y; diamond and strip 0.707/0.693; unions; a bar across
  x = 20; 300 µm; (1000, 1000); fifty flat/array.
- GFil.c: 0.80/0.795; diagonal 0.806/0.792; a diamond tip; an island in a ring; gaps across
  x = 20/40; (1000, 1000); 300 µm; fifty flat/array.
- GFil.d/e/f: 1.10/1.095 and 1.103/1.089 to every named layer (Cont as a 0.16 square);
  gaps across x = 20/40, ending on 20, a corner pair; (1000, 1000); 300 µm; fifty
  flat/array; the wide-NWell nBuLay question (note D).
- GFil.i: 400 × 400 (160000) clean, 400.005 × 400 fires; 800 × 199 and an L in a 500 × 500
  box are clean by area; a 500 × 400 union fires once.
- GFil.j: 0.18/0.175 (bottom, both, horizontal); cap 0.00; across x = 20; (1000, 1000);
  fifty flat/array.
- Not tested: the SRAM marker beyond one excluded cap (the manual's SRAM section is work in
  progress); the SVaricap/rfmos text markers that KLayout's Gat.a3/a4/f exclude; Gat.b1
  diagonal gate-region pairs (two gates whose regions meet corner to corner put their
  polys under 0.18, a Gat.b first).
