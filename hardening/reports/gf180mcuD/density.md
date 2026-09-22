<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / density: hardening report

Deck `density`, nine rules gathered from three places in the GF180MCU design manual:

| rule | manual | wording | value |
| --- | --- | --- | --- |
| DCF.1b | 13.1, Dummy COMP rules | "Minimum global density for active layers (COMP + Dummy COMP)" | 25% |
| DCF.1d | 13.1 | "Maximum global density for active layers (COMP + Dummy COMP)" | 70% |
| PL.8 | 7.7, Poly2 | "Poly2 coverage over the entire die shall be >= 14%.  Dummy poly2 lines must be added to meet the minimum poly2 density requirement." | 14% |
| M1.4 … M5.4 | 7.13, Metaln | "Metaln coverage over the entire die shall be >30% … Customer needs to ensure enough dummy metal to satisfy Metaln coverage" | 30% |
| MT.3 | 7.15, MetalTop | "MetalTop coverage over the entire die shall be >30%" | 30% |

Variant D is a five-level stack whose MetalTop *is* Metal5, so M5.4 and MT.3 are one
measurement under two names; the upstream runset emits both for the same reason
(`metal_level_numerical >= 5` and `metal_top != '30K'`, both true).  MT30.7, the 3 µm
MetalTop version of the same rule in section 7.16, belongs to the 30K option and is
correctly absent.  There are no other density rules in the manual, and no rule the deck
lists that the manual does not have.

Twelve layouts under `tests/data/gf180mcuD/generated/density/DCF.1*.h<n>.gds.gz`, drawn
by `gen/gf180mcuD/density.rs`, each with a `#[case]` in `hardening_density`.  Every rule
runs on every layout, so each fixture carries all seven measured layers and moves only
the one it is about.  Every layout ran at tiles 20, 7 and 100 and through the upstream
runset.  **No count moved with the tile size**, on any of them.

One note on the oracle: `hardening/oracle-gf180.sh` reports gdscheck's density markers as
zero throughout.  Its filter keeps only marker lines carrying `µm`, and a density marker
carries a percentage.  The gdscheck columns below are from
`gdscheck run --deck density -v`, not from the oracle's table; the KLayout column is the
oracle's.

## Findings

### 1. The percentage is taken over the boundary's bounding box, not over the boundary (false positive, and a false negative)

`DCF.1b.h6` - PR_BNDRY drawn as two boxes meeting along an edge, `(0,0)-(100,50)` and
`(0,50)-(50,100)`: an L-shaped die of 7500 µm² whose bounding box is 10 000 µm².  Every
measured layer covers `(0,0)-(52,50)`, 2600 µm², which lies wholly inside the L.  The
coverage of the die is 2600 / 7500 = **34.67%**, over every floor in the deck.

| tool | reported |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 26.00% on every layer; M1.4, M2.4, M3.4, M4.4, M5.4, MT.3 |
| KLayout runset | 26.00%; the same six |

2600 / 10 000 = 26% is the bounding box, not the die.

`DCF.1b.h7` is the same defect on a shape that is not exotic: two dies side by side with a
10 µm street between them, `(0,0)-(50,100)` and `(60,0)-(110,100)`, 10 000 µm² of
boundary in an 11 000 µm² box.  Every layer covers 3000 µm², 30% of the boundary and
exactly on the metal floor.  gdscheck reports 27.27% and fires the same six rules;
KLayout the same.  The street - which is not die at all - is a tenth of the denominator.

`DCF.1d.h4` shows the same box used as the *numerator*, and this one is a ceiling
violation invented out of nothing.  The L-shaped die of h6 again, every layer covering the
whole lower half of it (5000 µm², 66.67% of a 7500 µm² die, under DCF.1d's 70% ceiling),
plus 2400 µm² of COMP in the notch - the corner of the bounding box that is not die.

| tool | reported |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | COMP 74.00%; DCF.1d |
| KLayout runset | DCF.1d |

(5000 + 2400) / 10 000 = 74%.  Geometry that lies outside the die is counted into the
die's coverage, and the die's own area is inflated to the box at the same time.

Verdict: **gdscheck is wrong on all three.**  The deck names a boundary layer
(`layer_params: {boundary: pr_bndry}`) and the manual's subject is "the entire die" and
"global density"; the only reading of a coverage fraction that means anything is
*(area of the layer inside the boundary) / (area of the boundary)*.  Both numbers should
come from the boundary polygon.  Today both come from its bounding box, so a die that is
not one solid rectangle is mismeasured in both directions - too low when the box is bigger
than the die (h6, h7: six false positives each), too high when geometry sits in a part of
the box that is not die (h4: one false positive, and by the same token a real DCF.1b
shortfall could be papered over by fill outside the die).

KLayout agrees with gdscheck on all three, but it is not evidence here: the upstream
runset has no notion of PR_BNDRY at all.  It computes `chip_area = extent.sized(0.0).area`
- the bounding box of everything drawn - so it cannot tell an L-shaped die from its box
even in principle.  `DCF.1d.h3` below is where that difference shows, and there gdscheck
is the one in the right.

### 2. (Not a finding, recorded as the reading) A shape outside the die is not counted; KLayout counts it and mismeasures the die

`DCF.1d.h3` - the plain 100 µm die, every layer at 40%, plus a 5000 µm² COMP block 100 µm
to the right of the boundary and one 100 µm² block straddling the boundary's right edge.

| tool | reported |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | COMP 40.50%, the rest 40.00%; clean |
| KLayout runset | M1.4, M2.4, M3.4, M4.4, M5.4, MT.3, PL.8 |

gdscheck counts the half of the straddler that is inside (4000 + 50 = 4050 µm²) and drops
the far block.  KLayout's `extent` grows to 300 × 100 µm, so every layer's 4000 µm² is
divided by 30 000 and the floors collapse.  **gdscheck is right**; this is the case
finding 1's fix must not break, and it is why the fix is "use the boundary polygon", not
"use the layout extent".

## Tested and clean

- **The bounds, exactly and one step past.**  `DCF.1b.h1` puts every rule on its own
  number at once - COMP 25%, Poly2 14%, each metal 30% - and is clean; `DCF.1b.h2` takes
  0.005 µm off every band (0.005% of a 100 µm die) and all eight floors fire together.
  `DCF.1d.h1`/`h2` do the ceiling the same way at 70% and 70.005%.  The manual words the
  metal floors as "> 30%" and the other two as ">=", so a die at exactly 30.000% is a
  violation read strictly; neither tool reads it that way (upstream's test is
  `ratio * 100 < 30`), and a fill run aims at the number itself.  Left as it is.
- **Which layer belongs to which rule, and the summing.**  `DCF.1b.h2` pins all eight
  floors at once, MT.3 beside M5.4 on Metal5.  `DCF.1b.h4` draws COMP, Poly2 and Metal1
  nowhere and lets their dummy layers (22/4, 30/4, 34/4) carry 40% of the die: clean, both
  tools.  That is the manual's own arrangement - section 13 exists so that fill can make
  the density - and it is what upstream computes (`metal1 = metal1_drawn + metal1_dummy`).
- **The drawn layer and its fill overlapping.**  `DCF.1b.h3` covers the same 40% band
  twice, once on COMP and once on COMP dummy (and the same on Poly2 and Metal1): 40%, not
  80%, and DCF.1d stays quiet.  Coverage is the area a layer covers; the pair is joined,
  not added.  Both tools agree.
- **No boundary drawn.**  `DCF.1b.h5` has no 0/0 at all, which is how plenty of real GDS
  arrives.  The die falls back to what the layout covers, every layer reads 40%, nothing
  fires and nothing errors.  Same as upstream, which always works that way.
- **The tile lines.**  `DCF.1b.h8` is the exact-bound die again, drawn as upright stripes
  straddling x = 18..23 and 38..43 so that the tile lines at 20, 21, 40 and 42 cut them,
  on a die whose own edges sit at 3 and 103.  Clean at 20, 7 and 100 µm tiles, as is every
  other fixture here.
- **A sliding window.**  `DCF.1b.h9` is a 400 µm die whose lower 160 µm is solid and whose
  upper 240 µm is empty: 40% overall, with a 200 µm square of it at zero.  Clean, and it
  should be - the DRC rules are die-wide ("over the entire die", "global density"), and
  the 200 µm window stepped by 100 µm in section 13.3 is the recipe for *generating* dummy
  metal, not a rule to check.  The deck has no window parameter and needs none.
- **Exempting markers.**  There are none to test.  NDMY, RES_MK, IND_MK, PMNDMY and the
  Pad keep-outs in sections 13.1-13.3 govern where dummy fill may be placed (DCF.1a,
  DCF.8a, DCF.11a, DPF.18, DM.8); DCF.1b, DCF.1d, PL.8 and Mn.4 have no exclusion and
  upstream applies none.

## Resolution (2026-09-22)

Finding 1 was fixed in the engine (`src/checks/density/mod.rs`): the die is the
`boundary` layer's own polygons, not their bounding box, on both sides of the fraction.
The coverage counted is what lies inside them and the area divided by is theirs, so
`DCF.1b.h6` reads 34.67%, `DCF.1b.h7` 30.00% and `DCF.1d.h4` 66.67%, and all three
layouts are clean at tiles 20, 7 and 100 - which is what the three cases expect.

The windowed scope still lays its grid over the boundary's box, so the windows fall the
same way whatever the die's shape; what each window is measured against is the die area
inside it, and a window with no die in it is skipped rather than failed.

A boundary drawn as a *ring* is read as what it rings: its holes are filled first, off
the stitched layer, so a seal ring cut by tile lines is one ring. That keeps IHP's
`AFil.g2.boundary_ring` and the five metal fixtures like it reading the die they enclose,
which was the reason the box was used in the first place.

Finding 2 is the case the fix had to keep, and does: `DCF.1d.h3`'s block 100 µm outside
the boundary is still not counted, and the die is still the boundary rather than the
layout's extent. The reference designs are unchanged - both PDKs draw their boundary as
one filled rectangle, where box and polygon are the same thing.

The oracle note stands: `hardening/oracle-gf180.sh` shows gdscheck's density markers as
zero because its filter keeps only marker lines carrying `µm`, and a density marker
carries a percentage. Reading this deck's fixtures needs `gdscheck run --deck density -v`
beside the oracle's KLayout column.
