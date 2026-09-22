<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / metaltop: hardening report

Deck `metaltop` against section 7.15 of the GF180MCU design manual.  Variant D is the
11K top metal on a five-level stack, so the top metal *is* Metal5 and the starred column
applies: 0.44 width, 0.46 space, 0.6 to wide metal, 0.5625 µm² of area.  Section 7.16's
3 µm option (MT30.x) belongs to another variant and is rightly absent from `show-deck`.

9 layouts under `tests/data/gf180mcuD/generated/metaltop/MT.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/metaltop.rs`, each with a `#[case]` in `hardening_metaltop`.  Every layout
ran at tiles 20, 7 and 100 and through the upstream runset.  No count moved with the tile
size.

## Findings

### 1. The wide-metal space is blind to a slot cut into the plate (false negative)

MT.2b is the metal deck's Mn.2b at 0.6, written the same way and blind the same way: a
0.595 slot cut into a 30 x 20 plate (`MT.2b.h3`) was measured by neither tool.  See the
metal report's finding 1; the fix is the same `pairs: any`, and the case now expects the
one violation the manual asks for.

## Notes that are not findings

- The top metal is the drawn layer *and* the dummy fill: a 0.435 dummy bar is as narrow
  as a drawn one, a 0.3 drawn bar abutting a 0.3 dummy bar is one 0.6 conductor, and the
  slot, blocked, label and resistor datatypes are markers rather than metal
  (`MT.1.h1`, four markers, both tools).
- Metal5 is under MT.1's 0.44 and under none of section 7.13's Mn rules: `MT.1.h2` is
  silent on the `metal` deck and reports twice on this one.
- A chamfered corner and a narrow stub do not stop a 12 x 12 plate being wide, and a
  neighbour that faces only the stub owes MT.2a's 0.46 rather than MT.2b's 0.6
  (`MT.2b.h2`).

## Tested and found clean or correct (no need to redo)

MT.1 at 0.435/0.44 on the drawn and the dummy datatype and on the four marker
datatypes; MT.2a at 0.455/0.46 against dummy fill and against a blocked-fill marker;
MT.2b's "length & width > 10 µm" read strictly, the chamfer and stub cases, and the gap
opening on x = 20, 21, 40 and 42; MT.4 at 0.5625 µm² and the ring whose outline covers
more ground than its metal.
