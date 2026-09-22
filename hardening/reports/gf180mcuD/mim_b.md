<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / mim_b: hardening report

Deck `mim_b` against section 10.4.2 of the GF180MCU design manual (MIM option B, the
variant this PDK build selects).  On variant D the top metal is Metal5, so the option
sits between Metal5 and Metal4: FuseTop is the top plate, Metal4 the bottom plate, Via4
the "Vian-1" that connects both, Via3 the "Vian-2" that may not.

The section's own note 1 is what makes this deck unlike any other: nine of its eleven
rules measure against a layer nobody draws, the **virtual bottom plate**

> ((FuseTop @1.06) AND (Metaln-1 interact FuseTop))

so the patterns below are mostly about where that plate ends - which is the 1.06 µm
oversize, not the metal's own edge - and about the selections the other rules make.

23 layouts under `tests/data/gf180mcuD/generated/mim_b/MIMTM.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/mim_b.rs`, each with a `#[case]` in `hardening_mim_b`.  Every layout ran
at tiles 20, 7 and 100 and through the upstream runset
(`~/.ciel/.../gf180mcuD/libs.tech/klayout/tech/drc/rule_decks/mim_b.rb`).  **No count
moved with the tile size**, the 100 x 100 µm plates of `MIMTM.8b.h1` and the 102 x 100 µm
shared plate of `MIMTM.11.h2` included - the per-plate area sum holds across five tile
lines.

## Findings

### 1. The virtual bottom plate is grown by 1.065 µm, not the manual's 1.06 (false positive)

The manual's note 1 and upstream's `fusetop.sized(1.06.um)` both put the virtual plate's
edge exactly 1.06 µm outside FuseTop.  gdscheck's `fusetop_oversize` carries
`slack: 0.005` on top of the 1.06 radius, and the extra 0.005 is *added* to the plate.
Every margin read against the virtual plate is therefore 0.005 µm tight, which is one
grid step - exactly the distance the bound is drawn at.

Two layouts show it.

`MIMTM.1.h2` - two capacitors whose Metal4 runs 2.0 µm past a 6 x 6 FuseTop plate (so the
oversize, not the metal, is what bounds the virtual plate), each with a routing Metal4 bar
beside it.  The left bar stands 1.200 µm from the true virtual edge, the right one
1.195 µm.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 2 |
| KLayout runset | 1 |

gdscheck's own values give it away - FuseTop's right edge is at x = 11.0:

```
[MIMTM.1] space 1.1950 µm < 1.20 µm between topmin1_metal and mimtm_virtual
          at (12.0650, 5.0000)-(13.2600, 5.0000) µm
[MIMTM.1] space 1.1900 µm < 1.20 µm between topmin1_metal and mimtm_virtual
          at (37.0650, 5.0000)-(38.2550, 5.0000) µm
```

The virtual edge is reported at 12.065, i.e. FuseTop + 1.065.  The left capacitor's gap is
1.200 µm and legal; gdscheck reads it as 1.195 and reports it.  The right capacitor's gap
is genuinely 1.195 and both tools report it, but gdscheck prints 1.190.

`MIMTM.2.h3` - the same 0.005 in the rule's *selection* rather than its measurement.
MIMTM.2 applies to "Vian-1 within 1.06um oversize of FuseTop layer".  Left: a Via4 whose
inner edge sits exactly on the 1.06 oversize, with 0.395 µm of Metal4 beyond it.  Touching
the oversize is not being within it, so the via is not a bottom-plate via and the 0.395
is not this rule's business (upstream's `top_via.overlapping(mimtm_virtual)` needs area).
Right: a Via4 0.01 µm inside the oversize with the same 0.395, which is.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 2 |
| KLayout runset | 1 |

gdscheck's 1.065 plate gives the left via 0.005 µm of overlap, selects it and reports it.

Verdict: the manual and upstream agree on 1.06 and I believe them.  The cases expect the
manual's answer (one marker each), so both fail on the current engine.  `MIMTM.1.h1` and
`MIMTM.3.h3` are the controls: a capacitor whose metal overhangs by 2.0 µm and nothing
else on the layout is clean in both tools, and its plate overlap is read as 1.06 (well
over MIMTM.3's 0.6), so the slack does no harm where the metal is the narrower of the two.

### 2. A spacing of nothing on a shared edge is not reported (false negative)

`MIMTM.4.h2` - a Via4 abutting the top plate's right edge from outside, sharing 0.26 µm of
edge with it, sitting on the bottom-plate metal.  MIMTM.5 is "minimum spacing between top
plate and the Vian-1 connecting to the bottom plate: 0.4"; the spacing here is nothing,
and a via touching the top plate shorts the capacitor.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | 1 (`MIMTM.5`, edge-pair `(11,8.16;11,7.1)/(11,7.5;11,7.76)`) |

Both tools are silent on MIMTM.4 here, and rightly so: the plate does not overlap the via
at all, so there is nothing for an overlap rule to measure (upstream's crossing half
`top_via.not_outside(fusetop)` also needs area).  MIMTM.5 is the rule that owns this
geometry, and gdscheck's `min_space` drops it.

The companion `MIMTM.5.h2` shows it is the shared *edge* and not zero as such: a Via4
whose corner sits on the plate's corner is reported, `space 0.0000 µm < 0.40 µm at
(11.0000, 11.0000)-(11.0000, 11.0000)`.  So a point of contact is a spacing of nothing and
a line of contact is not, which cannot be right either way round.

Verdict: a shared edge is a space of nothing and is reported (the round's settled
reading); gdscheck's two-layer `min_space` misses it.  `MIMTM.4.h2` expects `MIMTM.5` and
fails on the current engine.

## Notes that are not findings

### MIMTM.10 reads the bottom plate as the metal under FuseTop, not the whole plate

MIMTM.10 forbids "any Vian-2 touching MIM bottom plate Metaln-1".  Per note 1 the bottom
plate is the virtual plate, which reaches 1.06 µm past FuseTop; both gdscheck
(`topmin1_via AND topmin1_metal AND fusetop`) and upstream (`topmin1_via.and(topmin1_metal
.and(fusetop))`) read only the metal *under FuseTop*.  `MIMTM.10.h1` draws a Via3 in that
1.06 µm ring and a Via3 straddling the plate's edge; both tools report one marker, the
straddler's.  A Via3 landing in the ring therefore passes, although it is on the same
Metal4 polygon the rule is protecting.  The two tools agree, the manual is ambiguous, and
the case is pinned to what they both do; raising it would be a deck change, not an engine
fix.

### MIMTM.12 is correctly absent

The section's twelfth rule - mark the capacitor's length with MIM_L_MK - is starred in the
manual ("rules not coded") and upstream says so in a comment where the check would be.
`show-deck` lists eleven rules with 8a/8b split, which is the whole coded section.  The
deck YAML's header says "All 12 rules in this section", which overcounts by one.

### Marker granularity

One logical violation is one gdscheck marker for every rule in this deck; upstream cuts
an enclosure into one marker per violating side, so `MIMTM.4.bad` is 1 against 2 and
`MIMTM.5.h2`'s corner is 1 against 2.  Everywhere else in this round the two counts
matched.

## Tested and found clean or correct (no need to redo)

- **MIMTM.1**: the bound at 1.195/1.200 from the virtual plate; a capacitor whose metal
  overhangs the oversize with nothing beside it; the 1.195 gap straddling x = 20 and
  x = 42 (tile-invariant).
- **MIMTM.2**: 0.395/0.400 of Metal4 around a bottom-plate via; a via outside the metal
  sharing its edge (no area in the virtual plate, not selected); a via held by 0.395 in
  routing metal far from any plate (not within the oversize); a via in the metal but past
  the oversize; the crossing half, a via straddling the metal's outer edge.
- **MIMTM.3**: 0.595/0.600 of overlap; a FuseTop plate with no Metal4 under it at all
  (fires - the virtual plate is empty); a plate the metal covers only half of (the
  crossing half); a plate whose metal stops flush with its edge (an overlap of nothing,
  fires); a plate whose metal overhangs by 2.0 (the overlap is the oversize's 1.06).
- **MIMTM.4**: 0.395/0.400 inside the plate; a via straddling the plate's edge (crossing);
  a via abutting it from outside (no overlap - correctly not MIMTM.4).
- **MIMTM.5**: 0.395/0.400 from the plate on the bottom metal; a via 0.395 from a plate
  whose metal stops flush with it, landing on bare silicon - not a via "connecting to the
  bottom plate", correctly silent.
- **MIMTM.6**: a U-shaped top plate whose opening is 0.595 (fires as a notch) and 0.600.
- **MIMTM.7**: CAP_MK exactly coincident with the plate (an enclosure of 0, clean);
  CAP_MK 0.5 short of one edge; CAP_MK as a frame with a hole over the plate; CAP_MK as
  two abutting boxes whose union covers the plate (the marker is the union).
- **MIMTM.8a**: 25.0 µm² exactly and 24.975, as a rectangle and as an L drawn from two
  boxes (the area is the merged shape's).
- **MIMTM.8b**: 10000 µm² exactly (100 x 100) and 10000.5, across five tile lines.
- **MIMTM.9**: 0.495/0.500 between two vias on the plate; a via on the plate and one
  straddling its edge 0.495 apart - the straddler is not `inside` the plate, so the pitch
  rule has only one via and is correctly silent.
- **MIMTM.10**: a Via3 straddling the plate's edge.
- **MIMTM.11**: two 50 x 100 capacitors on one bottom plate summing to exactly 10000 µm²
  (clean) and to 10000.25 (fires); two 60 x 100 capacitors on *separate* bottom plates
  summing to 12000 (clean - the cap is per plate, not per layout).

## Resolution (2026-09-22)

- **1 (the virtual plate's 5 nm of slack)**: fixed in the deck.  `fusetop_oversize` is
  grown by the manual's 1.06 and no more; the slack a *selection* radius wants put every
  margin in the section one grid step tight and made a via whose inner edge lay on the
  oversize a bottom-plate via.  The foundry case's MIMTM.2 goes 9 -> 7, which is
  KLayout's count exactly.
- **2 (a via abutting the plate)**: fixed in the deck, `abutting: report` on MIMTM.5
  (the foundry case 8 -> 11).
- The MIMTM.10 and MIMTM.12 notes are recorded and need nothing: the first is the
  reading both tools share, the second is the manual's own "not coded".
