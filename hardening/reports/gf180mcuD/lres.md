<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# `lres` — N+ poly resistor (low sheet rho), manual section 10.2

Layouts: `gen/gf180mcuD/resistors.rs` (shared with `pres`).
Fixtures: `tests/data/gf180mcuD/generated/lres/LRES.*.h*.gds.gz`.
Cases: `hardening_lres` in `tests/gf180mcuD.rs`.
Oracle: the upstream runset's `rule_decks/lres.rb`.

Section 10.2 is section 10.1 word for word with Nplus in place of Pplus: nine rules, the
same descriptions, the same values, `LRES.8` (current density) in the manual's own "rules
not coded" appendix. `show-deck` claims all of them and names nothing 10.2 does not. So
every layout of `pres` is drawn here on the N+ implant, the results are the same to the
marker, and **findings 1 to 5 of `hardening/reports/gf180mcuD/pres.md` hold here
unchanged**, under the names LRES.2, LRES.3, LRES.4, LRES.6 and LRES.9a:

| # | rule | layout | what |
|---|---|---|---|
| 1 | LRES.2 | `LRES.2.h1` | a serpentine's own 0.395 arm gap is not a space between resistors; both tools silent |
| 2 | LRES.3 | `LRES.3.h2` | a COMP abutting the body is a space of nothing and is dropped; KLayout catches it |
| 3 | LRES.4 | `LRES.4.h1` | "unrelated Poly2" is read as "touches no block anywhere", so a block five micron away exempts a neighbour |
| 4 | LRES.6 | `LRES.6.h2`, `LRES.6.h3` | the block overlap is read on the end walls too, and the rule says "in width direction" |
| 5 | LRES.9a | `LRES.9a.h2` | the marking's length is only checked where its edges fall inside the Poly2 |

Every layout was run at tiles 20, 7 and 100. **No count in this deck depends on the tile
size.** The two readings recorded at the end of `pres.md` — an abutting RES_MK still makes
the bar a resistor, and LRES.9b's "both sides over 80 µm" being the marking's extent rather
than its edges — hold here too, marker for marker (`LRES.1.h2`, `LRES.9b.h1`,
`LRES.9b.h2`).

The rest of this report is the one place the two sections part.

---

## Finding 6 — a bar under a RESISTOR marker is an N+ resistor *and* a high-sheet one

**Rule.** LRES.1, "Minimum width of Poly2 resistor: 0.8", and what the deck runs it on.

**Layout.** `LRES.1.h1`, four bars of 0.795 µm, one above the other: the full device
(Poly2 ∩ Nplus, touching a block, touching RES_MK); the same without RES_MK; the same
without the block; and the full device with a RESISTOR marker over it.

**gdscheck.** 4 markers at every tile size — two walls each on the first bar and on the
RESISTOR-marked bar.
**KLayout.** 2, one per bar, the same two bars.

**Verdict: both tools agree, and this is a real asymmetry in the manual, not a defect.**
Section 10.1 ends its P+ recognition with `not_interacting(resistor)` — a P+ bar under a
RESISTOR marker is the high-sheet device of 10.3 and 10.1 lets it go. Section 10.2 has no
such clause, in either deck. And 10.3 does not take the bar either: the high-sheet device
is recognised through Pplus, so an N+ bar under a RESISTOR marker is nothing to 10.3 (the
whole `LRES.1.h1` layout reports `LRES.1` and nothing else in the full runset). It stays an
N+ resistor, marker or no marker, and the width rule holds it to 0.8.

That is a defensible reading — a RESISTOR marker on an N+ bar has no meaning the manual
gives it, and the bar is still an unsalicided N+ poly resistor — but it is worth knowing
that the marker is silently ignored rather than rejected. Recorded here because the
expected value for `lres_1_h1` is four where the identical `pres_1_h1` is two, and that
difference is deliberate.

---

## Tested and clean — no need to redo

The same list as `pres.md`, on Nplus, with the same counts:

- **Recognition** (`LRES.1.h1`): a bar with no RES_MK and a bar with no block are silent.
- **The heads** (`LRES.1.h3`): a 1.0 body with a 0.6 head outside the block fires; the
  width is measured on the whole bar.
- **Tile lines** (`LRES.1.h4`, `LRES.2.h2`): the 0.795 bar across x = 20, across x = 40 and
  42, and across y = 20; a 0.395 gap straddling y = 40. Six and one marker at 20, 7 and 100.
- **Both metrics on LRES.3** (`LRES.3.h1`): 0.595 straight fires, 0.6 clean, 0.594 on the
  diagonal fires.
- **LRES.5** (`LRES.5.h1`): the implant stopping in the middle of the block, so the
  resistor's own wall is the implant's edge — `enclosure 0.0000 µm < 0.30 µm`, one marker
  either side.
- **LRES.6 read the right way** (`LRES.6.h1`): 0.275 above the body and 0.28 below, one
  marker.
- **LRES.7** (`LRES.7.h1`, `LRES.7.h2`): a contact abutting the block and one inside it;
  a contact 0.2149 µm from a salicide island on the diagonal. Both tools fire on all three.
- **LRES.9a's four markings** (`LRES.9a.h1`): on the outline exactly (clean), 0.6 narrow,
  0.6 short, 0.9 long. Three markers either side.
- The engine family's generic classes were not redrawn.

---

## Resolution (2026-09-22)

Findings 1-5 are section 10.1's and were fixed there; see the resolution of
`hardening/reports/gf180mcuD/pres.md`, which lists each fix and what it did to this deck's
counts (LRES.3 9 -> 10, LRES.9a 9 -> 11 on the foundry layout; LRES.6's two cases kept with
the tools' reading).

Finding 6 needed nothing: an N+ bar under a RESISTOR marker stays an N+ resistor because
10.2 has no clause handing it to 10.3 and 10.3 does not recognise it, and both tools read
it so. `lres_1_h1` keeps its four markers where `pres_1_h1` has two.
