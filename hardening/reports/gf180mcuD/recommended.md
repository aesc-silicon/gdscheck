<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / the guidance decks: CO.6 III, DF.1b and PL.3b

The `via_recommended` round ended with a coverage finding of its own: the `recommended`
suite carried the via guidance and none of its siblings, while Appendix B
(`gf180mcu_drm/drm_16.txt`) lists three more guideline rules in the same words, a
paragraph apart. This closes it. Layouts: `gen/gf180mcuD/recommended.rs`; fixtures
`tests/data/gf180mcuD/generated/recommended/`; cases in `hardening_recommended`.

## What a guideline is, and why it has its own suite

Each of the four is the twin of a mandatory rule at a larger value, and the manual's own
column says what for: "for minimum contact resistance variation", "with low sheet
resistivity". The geometry a guideline reports is legal and manufacturable; what it is not
is optimal. The foundry's runset carries none of them, so the oracle has no opinion on
these layouts and the manual's sentence is the whole argument.

- **CO.6 III** (section 7.12), Metal1 over a contact on all sides: 0.12 µm, where CO.6
  asks 0.005 and CO.6a and CO.6b ask 0.06 at a line end and beside a flush side.
- **DF.1b** (7.5), the width of an active used as a resistor: 0.3 µm at either voltage,
  where DF.1a asks 0.22 at 3.3 V of any active. "As a resistor" is what RES_MK marks, and
  the marking names the device, so the layer selects the whole active.
- **PL.3b** (7.7), the poly gap over an active: 0.26 µm at 3.3 V and 0.4 at 5 V, where
  PL.3a asks 0.24 of the same gap on COMP and on the field alike. Only the gate half of
  PL.3a carries the larger value - upstream writes PL.3a as `tgate.or(poly2 outside comp)`
  - so the layer is the poly over the active, by voltage class, whole.

On the gf180mcuD reference design CO.6iii alone reports 4 297 020 contacts and PL.3b_MV
15 251 gaps, beside the via guidance's 1.4 million. That is what these rules read like on
a real design drawn to the mandatory values, and it is why the suite stays out of `main`
and `core`: a run that mixed them with real violations would hide the real ones.

## Tested

`CO.6iii.h1`: Metal1 round a 0.22 µm contact by 0.12 (clean), 0.115, and 0.06 - the last
drawn to CO.6's own value and no further, which is the layout the guideline is written
for. Two markers.

`DF.1b.h1`: an active under RES_MK 0.3 µm wide (clean) and 0.295, and a 0.295 µm active
with no marking - a wire, which DF.1a holds to 0.22 and this rule does not read at all.
Two markers, one per wall of the narrow bar.

`PL.3b.h1`: two gates over one active 0.26 µm apart (clean) and 0.255; the same pair at
0.4 and 0.395 under Dualgate; and a 0.255 gap between two polys *off* the active, which is
PL.3a's gap and not this rule's. One marker per class.

No count moves with the tile size. Nothing in `main` or `core` changed: the reference
design still reports its 4872 waived CUP.3 and nothing else.
