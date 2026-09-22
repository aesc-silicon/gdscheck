<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# `pres` — P+ poly resistor, manual section 10.1

Layouts: `gen/gf180mcuD/resistors.rs` (shared with `lres`, drawn once and written into
both decks' directories with the implant and the rule prefix exchanged).
Fixtures: `tests/data/gf180mcuD/generated/pres/PRES.*.h*.gds.gz`.
Cases: `hardening_pres` in `tests/gf180mcuD.rs`.
Oracle: `hardening/oracle-gf180.sh <layout> TOP 20 7 100`, the upstream runset's
`rule_decks/pres.rb`.

Every layout was run at tiles 20, 7 and 100. **No count anywhere in this deck depends on
the tile size**, including the three fixtures drawn across x = 20, x = 40/42 and y = 20,
and the 140 µm markings of `PRES.9b`.

The deck claims all nine rules of section 10.1 (`PRES.8`, maximum current density, is in
the manual's own "rules not coded" appendix). `show-deck` names nothing 10.1 does not.

Section 10.2 is section 10.1 word for word with Nplus in place of Pplus, so findings 1–5
below hold for `lres` as well; `hardening/reports/gf180mcuD/lres.md` says where the two
decks part.

---

## Finding 1 — the serpentine's own gap is not a space between resistors

**Rule.** PRES.2, "Minimum space between Poly2 resistors: 0.4".

**Layout.** `PRES.2.h1`: two U-shaped resistors, each one polygon — two 6 µm arms 1 µm
wide joined at the left end, with the block and the marking over the whole of it. The top
U's arms are 0.395 µm apart, the bottom U's 0.4.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** Clean.

**Verdict: a false negative in both, and the layout is the normal one.** A poly resistor
of any length is drawn folded, and the gap between two arms of the same serpentine is
exactly the space PRES.2 names — it is the space that has to hold the sheet apart. The
upstream deck writes the rule as `pres_poly.isolated(0.4.um, euclidian)`, and `isolated`
is a space *between separate polygons*; a folded resistor is one polygon and the check
cannot see inside it. gdscheck mirrors that: the deck has `min_space` on `pres_poly` and
no `min_notch`.

The same section of the manual, in the same words, is coded the other way for the
high-sheet resistor: HRES.3 ("Minimum space between Poly2 resistors: 0.4") is
`hres_poly.space(0.4)` upstream and `min_space` **plus** `min_notch` in gdscheck, and the
same serpentine drawn as an HRES (`hres/HRES.3.h1`) is reported by both tools. One
sentence, two readings. I believe the HRES reading: the notch is the gap.

The expected value in `hardening_pres` is one `PRES.2` — one gap, one violation, the
granularity HRES.3 already reports it at.

---

## Finding 2 — a COMP abutting the resistor is a space of nothing and is dropped

**Rule.** PRES.3, "Minimum space from Poly2 resistor to COMP: 0.6".

**Layout.** `PRES.3.h2`, three devices: a COMP whose left edge is the resistor's right
edge (shared edge, no overlap); a COMP overlapping the body by 0.5 µm; a COMP wholly
inside the body.

**gdscheck.** 2 markers at every tile size — `comp_on_pres` at (16.5, 18.5) for the
overlapping COMP and at (13.0, 26.5) for the one inside. Nothing for the abutting COMP.
**KLayout.** 3.

**Verdict: a false negative.** A shared edge is a space of nothing, and a space of nothing
is under 0.6 — that is the settled reading of these rounds. Upstream catches it through
`comp.not_outside(pres_poly)`, which keeps a COMP that touches; gdscheck's second half of
PRES.3 is `forbidden` on `comp_on_pres`, and that layer is `overlapping`, which wants
area. The `min_space` half does not pick the touching pair up either. Expected: three.

---

## Finding 3 — "unrelated Poly2" is read as "Poly2 that touches no block anywhere"

**Rule.** PRES.4, "Minimum space from Poly2 resistor to unrelated Poly2: 0.6".

**Layout.** `PRES.4.h1`, three devices. Each has a bare poly bar off its right end: the
first at 0.595 µm, the second at 0.6, the third at 0.595 with a small salicide block
touching the bar's *far* end, five micron from the resistor and not touching the resistor
or its implant or its marking.

**gdscheck.** 1 marker at every tile size, for the first device.
**KLayout.** 1.

**Verdict: a false negative, shared with upstream.** The third bar is unrelated to this
resistor by every meaning of the word — no implant, no marking, not part of the device —
but the deck derives `poly2_unrelated` as `poly2_drawn not_interacting sab`, so a block
somewhere else on that bar takes it out of the rule. Upstream writes the same thing,
`poly2_drawn.not_interacting(sab)`. The derivation is a proxy for "part of a resistor",
and it is too generous: a bar under a block is not necessarily part of *this* device, and
nothing else in the deck measures the gap to it. Expected: two.

The same proxy is in HRES.5 and is drawn there (`hres/HRES.5.h1`).

---

## Finding 4 — PRES.6 is read on all four walls, and the manual says the width direction

**Rule.** PRES.6, "Minimum salicide block overlap of Poly2 resistor **in width
direction**: 0.28".

**Layouts.**
- `PRES.6.h3`: a resistor whose block runs 0.15 µm past the *left* end of the bar — shared
  with whatever lies to the left — and ends inside the bar on the right, where the head and
  its contact are. The overlap in the width direction is 0.35 the whole way.
- `PRES.6.h2`: the same thing at both ends, 0.15 µm past each, no contacts.

**gdscheck.** `PRES.6.h3`: one marker, `enclosure 0.1500 µm < 0.28 µm` at (10.0, 11.0)–
(10.0, 10.0) — the bar's left end wall. `PRES.6.h2`: two, one per end wall. Tile-invariant.
**KLayout.** 1 and 2, the same walls.

**Verdict: a false positive, shared with upstream.** The qualifier "in width direction" is
in the rule because in the length direction the block deliberately stops inside the bar —
that is what defines the resistor's length and leaves the heads salicided. It is the two
long walls that have to be covered with margin, so the salicide is blocked across the whole
conducting path. A block that runs *past* the end of the bar covers more, not less, and
`PRES.6.h3` is a layout somebody draws. Both tools read `enclosed` on every wall the block
covers and so report the end. Expected: clean.

`PRES.6.h1` — 0.275 above the body and 0.28 below — is the rule read the right way, and
both tools give one marker for it.

---

## Finding 5 — PRES.9a only sees a marking whose edges land inside the Poly2

**Rule.** PRES.9, "Pplus Poly2 resistor shall be covered by RES_MK marking. RES_MK length
shall be **coincide** with resistor length (Defined by SAB length) and width **covering**
the width of Poly2."

**Layout.** `PRES.9a.h2`: a bar 6 µm long whose block runs from x + 2.5 to x + 3.5, so the
resistor is 1 µm long, under a RES_MK that clears the whole bar on every side — 6.6 µm of
marking over 1 µm of resistor.

**gdscheck.** Clean, at 20, 7 and 100.
**KLayout.** Clean.

**Verdict: a false negative, shared with upstream, and a judgement call.** The manual uses
two different words on purpose: the width has to *cover*, so overhanging in the width
direction is right, but the length has to *coincide* with the block's. Both decks code the
whole sentence as one thing — the marking's edges that do not lie on the resistor's outline
and do fall inside the Poly2 — which catches a marking that stops short or that runs past
the block but not past the bar, and lets a marking that clears the bar entirely through.

This is the reading the shipped `PRES.9a.good` pair rests on (its marking clears the body
by 0.3 on every side while the block is inset 1.4), so accepting the finding means that
pair moves too. `PRES.9a.h1` is the four-way version of the same question and shows where
the coded rule does work: the marking on the outline exactly is clean, and narrow, short
and over-long-but-inside all fire, in both tools, three markers each.

---

## Two readings worth recording, not findings

**A RES_MK that only abuts makes the bar a resistor.** `PRES.1.h2`: a 0.795 µm bar whose
marking shares its left edge and covers none of it is a resistor in both tools (`res_mk`
enters through `interacting`), and the same marking 0.005 µm clear is not. The manual says
the resistor "shall be covered by RES_MK marking", so a marking that covers nothing is not
one — but taking that literally would mean the bar is not a resistor and PRES.1 would go
silent on a 0.795 µm resistor, which is worse. Both tools call it a resistor; gdscheck then
reports PRES.9a on it and KLayout does not, because gdscheck counts the marking's edge
lying *on* the Poly2's boundary as inside it and KLayout's `inside_part` does not. Here the
manual backs gdscheck — that marking coincides with nothing — but the two tools read
`inside_part` differently on a coincident edge, and that is worth knowing. The expected
value keeps both markers and the PRES.9a.

**PRES.9b: both sides over 80 µm is the marking's extent, not its edges.** `PRES.9b.h1`:
an octagon 140 µm across in x and in y — 16238 µm², over the 15000 bound — with a 100 µm
square marking 19.9 µm off its right side. gdscheck fires once; KLayout is silent, because
upstream selects `res_mk.with_area(15000.001).edges.with_length(80.001, nil)` and the
octagon's longest edge is 58 µm. The manual says "both side (X and Y) are greater than
80um", which is the marking's X and Y extent. I believe gdscheck. `PRES.9b.h2` — one
110 × 160 µm marking with a 19.9 µm notch cut 70 µm into it — is clean in both, rightly:
the rule is a spacing "to adjacent RES_MK layer" and a notch has no adjacent marking on
the other side of it.

---

## Tested and clean — no need to redo

- **Recognition** (`PRES.1.h1`): four 0.795 µm bars — the full device fires; the same
  without RES_MK, without the block, and with a RESISTOR marker over it are all silent.
  The RESISTOR marker is where 10.1 and 10.2 part: the same bar fires under `lres`.
- **The heads** (`PRES.1.h3`): a 1.0 µm body under the block with a 0.6 µm head outside it
  — the width is measured on the whole bar, heads included, in both tools. Correct: the
  rule says "width of Poly2 resistor" and the head is Poly2 of the resistor.
- **Tile lines** (`PRES.1.h4`, `PRES.2.h2`): the 0.795 bar across x = 20, across x = 40 and
  42, and across y = 20 while ending exactly on x = 21; a 0.395 gap straddling y = 40. Six
  and one marker at every tile size. The layer the rules run on is built by three
  `interacting` steps and none of them moved.
- **Both metrics on PRES.3** (`PRES.3.h1`): 0.595 straight off the end fires, 0.6 is clean,
  and a corner 0.42 in x and 0.42 in y away — 0.594 as a distance, nothing facing square on
  — fires in both tools. Upstream's `separation(comp, 0.6.um, euclidian)` and gdscheck agree
  (KLayout cuts the diagonal gap into two slivers, gdscheck into one marker).
- **PRES.5, the implant running out across the body** (`PRES.5.h1`): the implant covers the
  bar as far as x + 3.0 and stops in the middle of the block, so the resistor's own right
  wall is the implant's edge. `enclosure 0.0000 µm < 0.30 µm` at (13.0, 10.0)–(13.0, 11.0),
  one marker, both tools. The coincident wall is read, not skipped.
- **PRES.7 against the block** (`PRES.7.h1`): a contact abutting the block's edge and one
  wholly inside it, two markers in both tools. (`PRES.7.h2`) a contact 0.2149 µm from a
  salicide island on the diagonal — 0.152 in x and 0.152 in y, so nothing faces it square
  on — fires in both: KLayout's `separation` without a metric is euclidian too.
- **PRES.9a's four markings** (`PRES.9a.h1`): on the resistor's outline exactly — clean in
  both, which is the manual's own drawing; 0.6 µm narrow; 0.6 µm short at each end; 0.9 µm
  past the block at each end. Three markers either side.
- The engine family's classes (the bound and the step past it, the metrics, 45° walls,
  unions, holes, arrays, a shape far off) were not redrawn here.

---

## Resolution (2026-09-22)

Findings 1, 2, 3 and 5 were fixed in the PDK; finding 4 was read again and kept, with the
reason on its cases. Section 10.2 is this section word for word, so every fix landed on
`lres` in the same commit and `lres.md` carries the same resolution.

- **Finding 1 (PRES.2).** The deck takes a `min_notch 0.4` beside the space, as HRES.3
  already had. A folded resistor is one polygon and its inner gap is the space the rule
  names; the serpentine of `PRES.2.h1` reports one PRES.2 now, at the granularity HRES.3
  reports the identical geometry.
- **Finding 2 (PRES.3).** The space entry carries `params: abutting: report` - a shared
  edge is a space of nothing, and nothing is under 0.6. Read there rather than on
  `comp_on_pres`, which stays `overlapping`: a COMP that only touches a corner is already
  a space of zero to the space half, and widening the overlap layer to `interacting` would
  have reported it twice. `PRES.3.h2` reports three; the foundry layout goes 9 -> 10.
- **Finding 3 (PRES.4).** `poly2_unrelated` is the union of two selections: Poly2 under no
  block and Poly2 no marking names. Either one alone is too generous - `sab` exempts a bar
  whose block lies at its far end (the fixture), `res_mk` would exempt every bar a marking
  happens to reach (six of the foundry layout's eight). `PRES.4.h1` reports two, and the
  foundry counts stand where they were.
- **Finding 4 (PRES.6).** Kept. Separating a bar's width from its length is the device's
  orientation, and neither deck has it: to an enclosure the block's own end wall and the
  resistor's are the same wall, and the layouts where the two differ - a block running past
  the bar's end - have no salicided head there, so they are not a resistor either.
  `PRES.6.h2` expects two markers and `PRES.6.h3` one, which is what both tools report, and
  the reading is written above the cases.
- **Finding 5 (PRES.9a).** `pres9a_viol` gains a second half: a marking that lies over the
  salicided head. The rule asks the marking's length to coincide with the resistor's -
  which the same sentence defines as the block's - so a marking that reaches out over the
  heads is longer than the resistor it marks, whatever its edges do. This is the process's
  own shape: `ppolyf_u` in the PDK's pcell library draws `res_mk` on the block's span
  exactly and `sab` overhanging the Poly2's width, and the fixtures and the shipped
  patterns are drawn that way now. `PRES.9a.h2` reports one. The foundry layout goes 5 -> 7:
  the two new ones are 97 µm of RES_MK over a 2 µm bar, at (±200, 685), which the runset
  misses because its test only looks at marking edges that fall inside the Poly2.
  The marking's reference outline is `poly2 ∧ sab` now - the implant left it, because an
  implant that stops in the middle of the block is PRES.5's finding and made the marking's
  edge, which lies on the block's, look like an edge of its own.

Both readings recorded at the end of the findings stand: an abutting RES_MK still makes
the bar a resistor, and PRES.9b's "both sides over 80 µm" is the marking's extent.
