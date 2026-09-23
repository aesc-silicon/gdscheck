<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / acute: hardening report

Deck `acute` against the GF180MCU design manual, section 7.1, "Design Geometry Rules"
(`gf180mcu_drm/drm_07_02.txt`).  The section is three sentences; this deck is one of them:

> **ACUTE** — All shapes must be orthogonal or on a 45° unless otherwise stated.

Eight layouts, `tests/data/gf180mcuD/generated/acute/ACUTE.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/acute.rs`, each with a `#[case]` in the
`hardening_acute` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`DECKS=geom hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

Neither GRID, nor OFFGRID, nor ACUTE appears in Appendix B (`drm_16.txt`): all three are
coded rules, and nothing in 7.1 carries an asterisk.

## What the sentence says

It constrains the **direction an edge runs in**, not the angle at which two edges meet.  A
shape every edge of which runs at 0°, 45°, 90° or 135° "is orthogonal or on a 45°", and it
satisfies the sentence even where two such edges close an acute corner — a wedge with a 45°
tip, an arrowhead, a 45° V-notch.  The foundry's own runset reads it the same way; its
whole implementation is

```ruby
layer.edges.without_angle(0).without_angle(45).without_angle(90).without_angle(-45)
    .output("#{name}_ACUTE", "ACUTE : non 45 degree angle #{name}")
```

— four edge directions, no corner arithmetic anywhere.  The rule's *name* is the industry's
(a non-lattice edge usually arrives as an acute corner); its *content* is the lattice.
`ACUTE.h1` and `ACUTE.h2` draw the corner cases of that reading and both tools are silent on
both, which settles it.  Section 3.4's neighbour, 3.6 — "Only 90 deg and 45 deg bends are
allowed for poly and metal lines" — could be pressed into the other reading, since the turn
at a 45° tip is 135°, but 3.6 is layout advice about routed lines in chapter 3, carries no
rule number, and is not what either tool codes.  Left as the lattice reading.

gdscheck's marker is one per illegal edge, printed with the edge's own direction
(`comp: forbidden (26.6°) edge at (12.0000, 11.0000)-(10.0000, 10.0000) µm`); KLayout's is
one per illegal edge as well, so the counts are directly comparable here — unusually for
this pair of tools, every table below compares like with like.

**No count moved with the tile size.**  Eight layouts × three tiles, every rule identical,
including `ACUTE.h4`, which puts the same 26.57° wedge across x = 20, across x = 42 and
across y = 20 on purpose.  A tile cut splits a slanted edge into two collinear halves and
gives each half an endpoint the shape does not have; it neither doubled a marker nor lost
one, and the orthogonal box straddling both cuts stayed clean.

Test status on the engine as of this report: 3 of the 8 new cases fail (`ACUTE.h3`,
`ACUTE.h6`, `ACUTE.h8`), on findings 1 and 2.  The other 5 pass, as do the deck's 93
existing good/bad pairs.

## Findings

### 1. An edge within 1° of the lattice is accepted

*ACUTE*, "All shapes must be orthogonal or on a 45°."  gdscheck's `no_angle` check tolerates
a deviation of up to 1° from the nearest multiple of 45°.  45.14° passes.  89.9° passes.
0.1° passes.

`ACUTE.h3` draws five right triangles whose hypotenuse is the only edge under test, at
26.57°, 45.14°, 44.86°, 89.9° and 0.1°:

| layout | edges drawn off the lattice | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `ACUTE.h3` | 5 (26.57°, 45.14°, 44.86°, 89.9°, 0.1°) | 1 | 5 |
| `ACUTE.h8` | 11 (0.14° to 13.00° off 45°) | 6 | 11 |
| `offgrid/OFFGRID.h4` | 1 (0.14°, incidental to that fixture) | 0 | 1 |

gdscheck's only marker in `ACUTE.h3` is the 26.57° one, `comp: forbidden (26.6°) edge at
(12.0000, 11.0000)-(10.0000, 10.0000) µm`.  The four others are silent.

`ACUTE.h8` brackets the threshold.  Eleven wedges lean away from 45° by 0.14°, 0.29°, 0.57°,
0.85°, 0.98°, 1.02°, 1.12°, 2.73°, 5.19°, 9.46° and 13.00°; every vertex is on the 5 nm
grid, so all eleven are slopes a real layout can be drawn to.  gdscheck reports the six from
1.02° upwards and is silent on the five below; the pair at 0.98° and 1.02° — hypotenuses
4.140 and 4.145 µm high over a 4.000 µm base — fixes the cut-off at one degree to within a
thirtieth of a degree.

Why the manual is against the tolerance: 45.14° is not 45°, and on a 5 nm grid it is not a
rounding artefact either — a 1.000 × 1.005 µm wedge is an exactly representable shape that a
layout tool will happily produce from a mis-snapped guide line, and it is precisely the
mistake this rule is in the manual to catch.  The tolerance also makes the rule
scale-dependent in a way the sentence is not: a step of one grid unit over 0.2 µm is 1.4°
and fires, the same step over 0.3 µm is 0.95° and does not.  And it silently swallows the
near-vertical and near-horizontal edges that fracturing hates most: `ACUTE.h3`'s 89.9° edge
is a 2.86 µm wall leaning 5 nm out of true, which is a genuine OPC and mask-write problem
and is reported by the foundry's own deck.

Verdict: a false negative in gdscheck.  KLayout's `without_angle` is exact and the manual
gives no tolerance; the deck's `step: 45` should be read exactly, on the drawn edge.

### 2. Five drawn layers are in neither geometry deck

`show-deck --deck acute` lists 93 rules, one per layer, and `--deck offgrid` lists the same
93 — the two decks agree with each other exactly.  `pdks/gf180mcuD/pdk.yml` has 98 entries
with a `gds_layer`.  The five that no rule of either deck names are

| layer | GDS | checked by |
| --- | --- | --- |
| `metal1_dummy` | 34/4 | `dummy_metal` (DM1.1), `density` |
| `metal2_dummy` | 36/4 | `dummy_metal`, `density` |
| `metal3_slot` | 42/3 | `mslot` (MSLOT3.x) |
| `metal4_slot` | 46/3 | `mslot` |
| `metal5_slot` | 81/3 | `mslot` |

These are not layers the process does not have: every one carries its own rules in another
deck of this same PDK, so they are drawn, manufactured geometry.  The gap is exactly
complementary — the decks have `metal1_slot` and `metal2_slot` but not the M1/M2 *dummy*
layers, and `metal3_dummy`, `metal4_dummy`, `metal5_dummy` but not the M3/M4/M5 *slot*
layers — which is the signature of a copy-paste oversight rather than of a decision.

`ACUTE.h6` draws a 26.57° wedge on `metal1_dummy` and another on `metal3_slot`:

| layout | gdscheck @20/7/100 | KLayout |
| --- | --- | --- |
| `ACUTE.h6` | 0 (the deck has no rule to fire) | 0 (the runset has none either) |

The same gap is in the foundry's runset, and for the same five layers: `geom.rb`'s
`geom_m1m2` section lists `metal1_slot` and `metal2_slot` and no dummies, while `geom_m3`,
`geom_m4` and `geom_m5` list the dummies and no slots.  Compared layer for layer (the
`geom_metaltop` section excluded, since variant D stops at Metal5) the runset's list and the
decks' are **identical, all 93 names**.  So gdscheck is a faithful port — and the port
inherited the oversight.

The manual gives no exemption to lean on: 7.1 says "all shapes", names no layer and no
class, and Appendix B does not list ACUTE.  Dummy fill and slotting are shapes that reach
the mask like any other; an off-lattice dummy is as much an OPC problem as an off-lattice
signal wire.

Verdict: a coverage gap in gdscheck, shared with the upstream runset.  The two decks should
each carry 98 rules, not 93.  The case expects `metal1_dummy_ACUTE` and `metal3_slot_ACUTE`
and fails until they exist.

## Read and left alone

1. **The 45° acute corner is legal** (`ACUTE.h1`, `ACUTE.h2`).  A wedge with a 45° tip, an
   arrowhead with two 45° base corners and a plate with a 45° V-notch are all clean on both
   tools, and the manual's sentence is about edges.  See *What the sentence says*.
2. **A 180° vertex is not an angle.**  `ACUTE.h1` draws a box with a redundant vertex in the
   middle of its top edge; both tools are silent.  180° is a multiple of 45, and a check
   reading turns rather than directions would have to special-case it.
3. **A 45° rotation is legal, a 30° one is not** (`ACUTE.h5`).  One on-grid 1 × 1 box in a
   cell, placed as drawn, at 45° and at 30°.  The 45° copy's four edges land exactly on 45°
   and 135° and both tools are silent; the 30° copy's four edges are at 30° and 120° and
   both tools report four.  gdscheck's markers name them individually
   (`forbidden (30.0°) edge at (30.0000, 10.0000)-(30.8660, 10.5000) µm`).  The 45° copy is
   not clean under the *other* deck — its vertices are at 0.707 µm — which is the pair of
   answers this fixture exists to pin down.
4. **A rotated placement is resolved, not skipped.**  Same fixture: the illegal edges are
   inside a `CELL` that is orthogonal as drawn, and both tools find them in the top cell.
5. **The deck reads one layer per rule.**  The 93 good/bad pairs already do this layer by
   layer; `ACUTE.h6`'s shapes on `metal1_dummy` and `metal3_slot` drew no marker from any
   *other* layer's rule either.

## Tested and clean

- A box, an octagon, a diamond and a grid-aligned chamfer: 0°, 45°, 90° and 135° (`h1`).
- A redundant collinear vertex (`h1`).
- A 45° wedge tip, an arrowhead, a 45° V-notch (`h2`).
- The 26.57° wedge inside a tile, across x = 20, across x = 42, across y = 20, and an
  orthogonal box straddling two cuts — four markers at every tile size (`h4`).
- A cell placed at 0°, 45° and 30° (`h5`).
- One polygon that is off the lattice and off the grid at once: one `comp_ACUTE` here, two
  `comp_OFFGRID` in the offgrid table, both tools agreeing on both counts (`h7`).

---

## Resolution (2026-09-23)

Both findings were fixed.

- **Finding 1, the 1° tolerance.** `no_angle`'s `step` form is exact now - the default
  tolerance is 1e-6 degrees, which is where `atan2` lands on a 45° edge of equal legs and
  a thousandth of the smallest deviation a layout can draw (one grid step across a 4 µm
  edge is 0.07°). The `angle` form keeps its degree: there the slack widens the net rather
  than holing it, and no deck uses that form today. `ACUTE.h3` reports five and `ACUTE.h8`
  eleven, as the manual reads them and as the runset does. The three reference designs are
  unchanged, so no real geometry was leaning on the tolerance.
- **The coverage gap** (shared with `offgrid.md`): metal1_dummy, metal2_dummy,
  metal3_slot, metal4_slot and metal5_slot are in both decks now, 98 rules each, one per
  drawn layer. `ACUTE.h6` reports its two markers.

The reading the round settled - that the ACUTE sentence constrains the direction an edge
runs in and not the angle two edges meet at, so a 45° wedge is legal - is recorded and
followed; section 3.6's "only 90 and 45 deg bends" carries no rule number and neither tool
codes it.
