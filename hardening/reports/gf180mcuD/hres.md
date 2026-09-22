<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# `hres` — high-sheet poly resistor (PHRES), manual section 10.3

Layouts: the hardening half of `gen/gf180mcuD/hres.rs`.
Fixtures: `tests/data/gf180mcuD/generated/hres/HRES.*.h*.gds.gz`.
Cases: `hardening_hres` in `tests/gf180mcuD.rs`.
Oracle: `hardening/oracle-gf180.sh <layout> TOP 20 7 100`, the upstream runset's
`rule_decks/hres.rb`.

The deck claims all twelve rules of section 10.3 (`HRES.11`, maximum current density, is in
the manual's own "rules not coded" appendix) and names nothing 10.3 does not.

Every layout was run at tiles 20, 7 and 100. **No count anywhere in this deck depends on
the tile size**, including the three bars drawn across x = 20, x = 40/42 and y = 20, and the
140 µm markings of `HRES.12b`.

## The shape everything is drawn on

The manual is explicit about what a PHRES looks like: "resistor width is determined by
Poly2 width and the resistor length is determined by **Pplus to Pplus space** on Poly2. In
order to realize this resistor 'Resistor' layer must be drawn covering the Poly2." So the
implant does *not* cover the bar. It comes in from each end as a head, stops 0.1 µm over
the salicide block — that is HRES.10's value, min and max — and the resistor is the bare
Poly2 between the two heads. Every layout here is built on that shape: an 8 µm bar 1.2 µm
wide, a block from x + 2 to x + 6 overhanging 0.4 in the width direction, a P+ head at each
end reaching 0.1 over the block, a RESISTOR marker 0.5 past the bar, a contact on each
head.

The deck's own good/bad pairs are drawn the other way, with the implant over the whole bar,
and three of the findings below only appear on the real shape.

---

## Finding 6 — a hole in the salicide block over the body is not reported

**Rule.** HRES.9, "Minimum salicide block overlap of Poly2 resistor in width direction:
0.28".

**Layout.** `HRES.9.h1`: the device with a 0.5 × 0.4 µm hole cut in the block over the
middle of the body, well inside the bar.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** 1 marker (upstream's `hres9_sab.holes.and(hres_poly)` term).

**Verdict: a false negative.** Over that half micron the block overlaps the resistor by
nothing at all — the salicide is not blocked there and the resistor is shorted through it.
gdscheck's HRES.9 is only the `min_enclosure` of `hres_poly` within `hres9_sab` with
`interacting_only`, which reads the walls that lie under the cover; the hole's walls are
walls of the *block*, not of the poly, and no wall of the poly is anywhere near it.
Upstream carries two extra terms for exactly this, and one of them catches it.

---

## Finding 7 — a block edge that stops inside the resistor's width is reported by neither

**Rule.** HRES.9, same wording.

**Layout.** `HRES.9.h3`: a 0.5 µm notch bitten out of the block from above, ending 0.4 µm
*inside* the bar. Over that half micron the block covers the bottom of the resistor and
stops short of its top edge, so the top of the resistor is salicided and the overlap in the
width direction is less than nothing.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** Clean.

**Verdict: a false negative in both.** This is the case upstream's
`hres9_bad_inside_edge` term is written for — the block's edge inside the poly with clear
block on one side of it — but the term extends the edge by one nanometre and asks it to
touch the block outside the poly, so it only fires when the edge stops just inside the bar,
not 0.4 µm in. gdscheck's `min_enclosure` with `interacting_only` skips the poly's top wall
over that stretch, because that wall is not under the cover at all. The rule wants the
block to cover the resistor across its full width with 0.28 to spare; here it covers two
thirds of it. Expected: one.

Findings 6 and 7 are the same complaint from two sides: HRES.9 is read only where the block
already covers the resistor, and never says anything about where it does not.

---

## Finding 8 — an overlap of nothing is not under the minimum

**Rule.** HRES.10, "Minimum & maximum Pplus overlap of SAB: 0.1".

**Layout.** `HRES.10.h2`: the left P+ head stopping 0.1 µm *short* of the block instead of
reaching 0.1 µm over it. The right head is where it belongs.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** 1 marker (upstream's `pplus.not_overlapping(sab).edges` term, gated on
`interacting(hres_poly)`).

**Verdict: a false negative.** The rule fixes the overlap at 0.1 from below as well as from
above, and an overlap of zero is below it. gdscheck derives `hres10_overlap` as
`pplus ∩ hres9_sab` and puts `min_width 0.1` on it: where the two do not meet there is no
region to measure and the rule has nothing to say. The head that does not reach the block
also leaves 0.1 µm of unsalicided, unimplanted poly between the block and the head, which
is the thing the 0.1 µm overlap exists to prevent.

---

## Finding 9 — HRES.10's maximum half does not read the overlap the manual means

**Rule.** HRES.10, same.

**Layout.** `HRES.10.h3`: two devices, one with the left head 0.1 µm over the block (clean)
and one with it 0.105 µm over.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** 1 marker, `polygon: (12,19.599;…;12.106,…)` — the 0.105 µm strip on the second
device.

**Verdict: a false negative.** The overlap the manual fixes is the depth of Pplus over the
block, and on a real PHRES that is a strip in the *length* direction: 0.1 µm wide, as tall
as the block. gdscheck reads the minimum off that strip (`min_width` on `hres10_overlap`,
and `HRES.10.h1` shows it working — the 0.095 strip gives two markers) but reads the maximum
as `max_enclosure` of `hres9_sab` within `pplus`, which asks how far the implant reaches
*past* the block. On the deck's own bad pair, where the implant covers the whole bar and so
does contain the block, those two happen to be the same number; on a real PHRES the implant
does not contain the block and `max_enclosure` has nothing to measure, so 0.105 goes
through. Upstream writes both halves as one `pplus.and(sab).drc(width != 0.1.um)` and
catches it.

`HRES.10.h1` (0.1 and 0.095) and `HRES.10.h3` (0.1 and 0.105) are deliberately the same
layout with one number changed, so the pair shows the asymmetry on its own.

---

## Finding 3 (shared with 10.1 and 10.2) — "unrelated Poly2" is too generous

**Rule.** HRES.5, "Minimum RESISTOR space to unrelated Poly2: 0.3".

**Layout.** `HRES.5.h1`, three devices with a bare poly bar off the RESISTOR marker's right
edge: at 0.295 µm, at 0.3, and at 0.295 with a small salicide block touching the bar's far
end, well clear of the device.

**gdscheck.** 1 marker at every tile size, for the first device.
**KLayout.** 1.

**Verdict: a false negative, shared with upstream** — the same derivation
(`poly2_drawn not_interacting sab`) and the same argument as finding 3 of
`hardening/reports/gf180mcuD/pres.md`: a block somewhere else on a bar does not make that
bar part of *this* resistor, and nothing else measures the gap to it. Expected: two.

---

## Finding 1 (the other side of it) — HRES.3 has the notch that PRES.2 and LRES.2 lack

`HRES.3.h1` is the serpentine of `pres/PRES.2.h1` and `lres/LRES.2.h1` drawn as a
high-sheet resistor: one polygon, two arms 0.395 µm apart. Here both tools report it —
gdscheck `notch 0.3950 µm < 0.40 µm` at (14.6, 11.2)–(14.6, 11.595), upstream
`hres_poly.space(0.4.um, euclidian)`. The identical geometry under 10.1 and 10.2, whose
rule reads "Minimum space between Poly2 resistors: 0.4" in exactly the same words, is
reported by neither, because those two are coded with `isolated`. The HRES reading is the
right one and the fixture is here so the two can be compared side by side.

---

## A reading worth recording, not a finding

**A bar with no implant on it at all is a high-sheet resistor.** `HRES.2.h1`, fifth device:
a 0.995 µm bar whose P+ merely abuts its left edge and covers none of it, under a block, a
RES_MK and a RESISTOR marker. Both tools call it an HRES and fire HRES.2 on it — upstream
because `hres1_poly` is `poly2_drawn.interacting(pplus)`, gdscheck because `hres_poly_pp`
is the same `interacting`. Where 10.1 recognises its device as `Poly2 **and** Pplus`, 10.3
only asks the two to touch. That is arguably right for this device: the manual says the
resistor is the Poly2 *between* the implanted heads, so it must not be asked to lie inside
the implant. But it also means an untouched poly bar next to any P+ region counts, and the
recognition rests on the RESISTOR marker alone. Both tools agree; recorded so the
`hres_2_h1` expectation is not mistaken for a defect.

**HRES.12b: both sides over 80 µm is the marking's extent, not its edges.**
`HRES.12b.h1`, the same octagon as `PRES.9b.h1`: 140 µm across in x and in y, 16238 µm²,
19.9 µm from a 100 µm square. gdscheck fires once, KLayout is silent because upstream keeps
only edges over 80 µm long and the octagon's longest is 58. The manual says "both side (X
and Y) are greater than 80um". I believe gdscheck. (The rule is on `res_mk` alone, so the
full runset reports `PRES.9b` and `LRES.9b` on this layout too; the deck-only case sees
`HRES.12b`.)

---

## Tested and clean — no need to redo

- **Recognition** (`HRES.2.h1`): five 0.995 µm bars — the full device fires; without the
  RESISTOR marker, without RES_MK and without the block are all silent; the bar whose
  implant only abuts fires (see above). Four markers gdscheck, two KLayout (one per bar).
  The bar without the RESISTOR marker is a P+ resistor instead — the full runset reports
  `PRES.5` twice on this layout, in both tools, for the implant edges that cut across it —
  which is the two sections handing the device over as they should.
- **Tile lines** (`HRES.2.h2`): the 0.995 bar across x = 20, across x = 40 and 42, and
  across y = 20. Six markers at 20, 7 and 100.
- **HRES.1, the marker's own space** (`HRES.1.h1`): a 0.395 µm notch in the RESISTOR
  marker, 0.2 µm deep and 0.5 µm clear of the bar so HRES.4's 0.4 still holds —
  `notch 0.3950 µm < 0.40 µm` at (13.0, 11.8), one marker either side.
- **HRES.4, the marker running out across the bar** (`HRES.4.h1`): the RESISTOR marker
  covers the left half of the bar and stops. One marker either side
  (`hres_4_crossing` here, `hres_poly.not_outside(resistor).not(resistor)` upstream).
- **HRES.6** (`HRES.6.h1`): COMP 0.295 µm off the marker's right edge fires, 0.3 is clean,
  and a COMP lying under the marker but clear of the bar is a space of nothing and fires
  through `comp_on_hres`. Two markers either side. (Contrast finding 2 of `pres.md`: PRES.3
  drops the *abutting* COMP, and this one overlaps.)
- **HRES.7** (`HRES.7.h1`): the left head drawn as an L that runs the whole way to its 0.1
  over the block below y + 0.6 and stops at x + 1.1 above it, so the contact's top-right
  corner sticks out of the implant while HRES.10 stays clean. `hres_7_crossing` at
  (11.16, 10.655), one marker either side.
- **HRES.8** (`HRES.8.h1`): a contact on the body, wholly inside the block — one marker
  either side. (`HRES.8.h2`) a contact on the head abutting the block's edge: that also
  leaves it 0.1 µm from the implant's edge, which is where HRES.10 puts that edge, so the
  layout is HRES.7 and HRES.8 at once and both tools report both.
- **HRES.9 read the right way** (`HRES.9.h2`): the block overhanging 0.275 above the bar and
  0.28 below it — one marker either side.
- **HRES.10's minimum** (`HRES.10.h1`): 0.1 clean, 0.095 fires, two markers (one per wall of
  the strip) against KLayout's two.
- **HRES.12a** (`HRES.12a.h1`): three markings on the real device — on the body exactly,
  which is the manual's own drawing and is clean in both; 0.6 µm narrow, so its long edges
  run inside the Poly2; and 0.9 µm short at each end. Two markers either side.
- The engine family's generic classes (the bound and the step past it, the two metrics, 45°
  walls, unions, holes, arrays, a shape far off) were not redrawn here.
