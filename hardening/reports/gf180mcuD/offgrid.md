<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / offgrid: hardening report

Deck `offgrid` against the GF180MCU design manual, section 7.1, "Design Geometry Rules"
(`gf180mcu_drm/drm_07_02.txt`).  The section is three sentences; two of them are this deck:

> **GRID** — The design grid must be an integer multiple of 0.005 µm.
>
> **OFFGRID** — All edge boundaries must be snapped to the grid defined above.

Eight layouts, `tests/data/gf180mcuD/generated/offgrid/OFFGRID.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/offgrid.rs`, plus the two shared with the `acute` deck
(`acute/ACUTE.h5` and `acute/ACUTE.h7`), each with a `#[case]` in the `hardening_offgrid`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through GlobalFoundries' own KLayout runset
(`DECKS=geom hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

Neither GRID nor OFFGRID appears in Appendix B (`drm_16.txt`): both are coded rules.
Section 3.4 repeats the advice — "Consistent layout all designs on a 0.005 µm grid will
avoid off-grid and snapping issues during database fracturing" — and adds nothing to check.

## What the two sentences say, and what they leave to the engine

GRID fixes the number and OFFGRID applies it to the drawn geometry; there is one check to
make, `x mod 0.005 == 0 and y mod 0.005 == 0` at every vertex of every drawn layer, with no
exemption, no layer left out and no tolerance.  The deck's 93 rules all carry
`check: offgrid, value: 0.005`, which is the sentence exactly.

Three things the sentences do not decide, decided here from the layouts and confirmed
against the runset:

- **The database unit is not the grid.**  The layouts are written at 1 nm dbu, so the finest
  off-grid offset that exists is 0.001 µm and "half a grid step", 0.0025, is not a
  representable vertex at all.  `OFFGRID.h2` walks all four representable offsets instead,
  and a half step appears only where the *engine* makes one — on a tile line, finding 0
  below.  (The foundry's runset carries a separate `DBU` rule, "the design database unit
  must be 0.001", which is not a rule of section 7.1 and has no number in the manual's
  table; gdscheck has no equivalent, correctly.)
- **A marker is one off-grid vertex**, on both tools:
  `comp: off-grid vertex (grid = 0.0050 µm) at (14.0030, 10.0000) µm`.  An orthogonal box
  with one off-grid coordinate therefore always counts two, since that coordinate appears at
  two corners.  Both tools cut it the same way and every count below is comparable.
- **What is read is the merged geometry**, not the drawn boxes — see *Read and left alone*.

**No count moved with the tile size.**  Ten layouts × three tiles, every rule identical.
`OFFGRID.h4` is the fixture built to break this and did not: see finding 0.

Test status on the engine as of this report: 1 of the 10 new cases fails (`OFFGRID.h7`), on
finding 1, which is a coverage gap and not a wrong answer.  The other 9 pass, as do the
deck's 93 existing good/bad pairs.

## Findings

### 0. The tile lines do not invent an off-grid vertex (no finding — the negative result)

This is the defect class the brief expects of these two decks, so it is recorded even though
nothing came of it.  gdscheck measures in tiles; a tile cut gives a shape vertices it does
not have, and a cut across a *slanted* edge lands wherever the geometry says, which need not
be on the grid.

`OFFGRID.h4` (a) is a wedge with vertices (19, 10), (21, 10) and (21, 10.005) — every one on
the grid.  Its hypotenuse crosses x = 20 at y = 10.0025, which is not only off the grid but
not even representable at 1 nm, so a cut there must round to 10.002 or 10.003, both off the
grid.  A marker on that point would be the engine talking about itself.

| structure | geometry | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `h4` (a) | on-grid wedge, hypotenuse crossing x = 20 at y = 10.0025 | 0 / 0 / 0 | 0 |
| `h4` (b) | 3 nm off-grid box straddling x = 20 | 2 / 2 / 2 | — |
| `h4` (c) | 3 nm off-grid box straddling x = 42 (a 7 µm tile line, not a 20 µm one) | 2 / 2 / 2 | — |
| `h4` (d) | 3 nm off-grid box whose right edge lies *on* x = 20, off-grid corner on the line | 2 / 2 / 2 | — |
| `h4` total | | 6 / 6 / 6 | 6 |

gdscheck's six markers are at (19, 12.003), (21, 12.003), (41, 14.003), (43, 14.003),
(18, 16.003) and (20, 16.003) — the eight-corner-coordinates arithmetic of (b), (c) and (d)
and nothing else.  Nothing at x = 20 or x = 42 that the shapes did not put there, nothing
doubled where a box is cut in half, and (d)'s corner sitting exactly on the tile line is
reported once, not twice.

Verdict: clean, at 20, 7 and 100 µm.  The one rule KLayout reports on this layout and
gdscheck does not is `comp_ACUTE` on (a)'s 0.14° hypotenuse, which is the `acute` report's
finding 1 and nothing to do with this deck.

### 1. Five drawn layers are in neither geometry deck

`show-deck --deck offgrid` lists 93 rules, one per layer, and `--deck acute` lists the same
93 — the two decks agree with each other exactly, name nothing that is not a drawn layer,
and between them cover 93 of the 98 entries in `pdks/gf180mcuD/pdk.yml` that have a
`gds_layer`.  The five neither deck names are

| layer | GDS | checked by |
| --- | --- | --- |
| `metal1_dummy` | 34/4 | `dummy_metal` (DM1.1), `density` |
| `metal2_dummy` | 36/4 | `dummy_metal`, `density` |
| `metal3_slot` | 42/3 | `mslot` (MSLOT3.x) |
| `metal4_slot` | 46/3 | `mslot` |
| `metal5_slot` | 81/3 | `mslot` |

Every one of them carries its own rules in another deck of this same PDK, so they are drawn,
manufactured geometry and not layers the process lacks.  The gap is exactly complementary —
the decks have `metal1_slot` and `metal2_slot` but not the M1/M2 *dummy* layers, and
`metal3_dummy`, `metal4_dummy`, `metal5_dummy` but not the M3/M4/M5 *slot* layers — the
signature of a copy-paste oversight rather than of a decision.

`OFFGRID.h7` draws a box with a 3 nm off-grid right edge on `metal1_dummy` and another on
`metal3_slot`:

| layout | off-grid vertices drawn | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `OFFGRID.h7` | 4 | 0 (the deck has no rule to fire) | 0 (the runset has none either) |

The same gap is in the foundry's runset, and for the same five layers: `geom.rb`'s
`geom_m1m2` section lists `metal1_slot` and `metal2_slot` and no dummies, while `geom_m3`,
`geom_m4` and `geom_m5` list the dummies and no slots.  Compared layer for layer (the
`geom_metaltop` section excluded, since variant D stops at Metal5) the runset's list and the
decks' are **identical, all 93 names** — gdscheck is a faithful port, and the port inherited
the oversight.

The manual gives nothing to lean on: 7.1 says "all edge boundaries", names no layer and no
class, Appendix B does not list OFFGRID, and section 3.4's reason — "will avoid off-grid and
snapping issues during database fracturing" — is about the fracturing of the mask, which
does not care whether a polygon is a signal wire, a dummy or a slot.

Verdict: a coverage gap in gdscheck, shared with the upstream runset.  Each deck should
carry 98 rules, not 93.  The case expects two `metal1_dummy_OFFGRID` and two
`metal3_slot_OFFGRID` and fails until they exist.  (This is the same finding as the `acute`
report's finding 2; it is one gap in two decks, and fixing it means the same five layers
twice.)

## Read and left alone

1. **The check is exact, in both directions and at any coordinate** (`OFFGRID.h2`,
   `OFFGRID.h3`).  Offsets of 1, 2, 3 and 4 nm all fire and 5 nm does not; a 3 nm offset at
   x = −12.003 fires, and so does one at x = 1002.003, where a remainder taken in the wrong
   sign convention or a float with its last digits spent would have gone quiet.  Eight and
   four markers, matching KLayout exactly.
2. **45° geometry is not off-grid geometry** (`OFFGRID.h1`).  An octagon's and a diamond's
   45° vertices land on the grid like any other, and a box one grid step (0.005 µm) wide is
   clean.  Both tools silent.
3. **An off-grid placement is an off-grid shape** (`OFFGRID.h5`).  One on-grid 0.5 µm box in
   a cell: placed at an on-grid point it is clean; placed at (20.003, 10) all four of its
   vertices fire; placed as a three-column array on an on-grid origin with a 1.002 µm pitch,
   the first copy is clean and the second and third fire four each.  Twelve markers,
   matching KLayout.  The rule reads the geometry after the hierarchy is resolved, which is
   the answer the manual needs — the mask sees the placed copy, not the cell.
4. **A rotation is the case these rules exist for** (`acute/ACUTE.h5`).  A 45° turn of an
   on-grid orthogonal box keeps every *edge* on the 45° lattice — so the `acute` deck is
   silent, correctly — while putting three of its four vertices at 0.707 and 1.414 µm.  Both
   tools report three.  A 30° turn does both: four illegal edges and three off-grid
   vertices.  gdscheck's markers are at (19.2930, 10.7070), (20.0000, 11.4140),
   (20.7070, 10.7070), (29.5000, 10.8660), (30.3660, 11.3660) and (30.8660, 10.5000) — the
   corner that stays on the placement point stays on the grid in both copies, which is
   right.
5. **A text is not an edge boundary** (`OFFGRID.h6`).  A label placed at (10.003, 10.003) on
   `comp_label`, with an on-grid boundary beside it on the same layer, is clean on both
   tools.  OFFGRID speaks of "edge boundaries"; a text has none.  Nothing is manufactured
   from a label either.
6. **Merging happens before the check** (`OFFGRID.h8`).  An off-grid box laid across an
   on-grid one so that its off-grid edge sticks out gives the union two off-grid vertices,
   and both tools report two.  An off-grid box drawn wholly *inside* an on-grid one gives
   none: after the merge nothing of it reaches a boundary, and the manual constrains
   boundaries.  Two markers for the layout, not four, on both tools.  This is worth knowing
   before someone reports the swallowed box as a miss: it is the reading the sentence asks
   for, but it does mean a designer's off-grid data can hide under a larger shape.

## Tested and clean

- On-grid geometry of every kind the layer allows: orthogonal, 45°, one grid step wide,
  a thousand microns out, in the negative quadrant (`h1`).
- Offsets of 1, 2, 3, 4 and 5 nm (`h2`).
- The same offset at negative coordinates and at x ≈ 1000 (`h3`).
- The tile lines: a cut across a slanted edge at a half-grid point, off-grid boxes straddling
  x = 20 and x = 42, and an off-grid corner sitting on x = 20 — six markers at 20, 7 and
  100 µm (`h4`).
- A cell placed off-grid and an array with an off-grid pitch (`h5`).
- An off-grid text on a label layer (`h6`).
- An off-grid box protruding from an on-grid one, and one swallowed by it (`h8`).
- The `acute` deck's rotation fixture (`acute/ACUTE.h5`) and its off-lattice, off-grid wedge
  (`acute/ACUTE.h7`), both agreeing with KLayout marker for marker.
