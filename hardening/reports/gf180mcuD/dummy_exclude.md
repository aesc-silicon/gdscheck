<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dummy_exclude: hardening report

Deck `dummy_exclude` against section 10.8 of the GF180MCU design manual (DE, the dummy
exclude markers NDMY and PMNDMY).  5 layouts under
`tests/data/gf180mcuD/generated/dummy_exclude/DE.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/dummy_exclude.rs`, each with a `#[case]` in the `hardening_dummy_exclude`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through the upstream KLayout runset (`hardening/oracle-gf180.sh`,
`DECKS=dummy_exclude`).

**No count moved with the tile size, on any layout, on any rule.**

`show-deck` lists four entries for the section's three rules: DE.2 (`min_width` 0.8 on
NDMY ∪ PMNDMY), DE.3 (`max_area` 15000 on NDMY) and DE.4 (`min_space` and `min_notch` 20
on NDMY).  Every rule of the section that can be measured is there - DE.1 is the
"draw these only if necessary" paragraph, and upstream says so too ("Rule DE.1 is not a
DRC check").  The two findings are both about *how* a rule is read, not about a missing
one.

## Findings

### 1. DE.2 is read on the union of the two markers, which hides a narrow one (false negative)

Manual: "DE.2  Minimum NDMY or PMNDMY size (x or y dimension in um)  0.8".

Layout `DE.2.h2`: a 0.795 x 5 NDMY strip lying on a 20 x 20 PMNDMY plate, and, 55 µm
away, a 0.795 x 5 PMNDMY strip lying on a 20 x 20 NDMY plate.  Each narrow strip is
inside the other layer's plate, so the union of the two layers is a 20 µm square with
nothing narrow about it.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DE.2.h2` | 0 / 0 / 0 | 0 |
| `DE.2.h1` (the same widths, one layer at a time) | 4 / 4 / 4 | 2 |

Verdict: two violations, four markers by gdscheck's own one-per-wall convention.  The
rule reads "minimum NDMY **or** PMNDMY size": it is the size of an NDMY, and the size of
a PMNDMY, not the size of whatever the two together cover.  The two layers do different
jobs - NDMY excludes dummy COMP, PMNDMY excludes dummy poly and dummy metal - so a shape
that is 0.795 µm on one of them is an 0.795 µm exclude region for that fill whatever the
other layer does, and the two overlap for good reasons all the time.  Both gdscheck and
the foundry's `ndmy.or(pmndmy).width(0.8.um)` merge them first and see nothing.

### 2. DE.3's area cap is applied on its own, without the clause that relieves it (false positive)

Manual: "DE.3  Maximum NDMY size (um2)  15000.  If size greater than 15000um2 then two
sides should not be greater than (um)  80."

Layout `DE.3.h1`: four NDMY rectangles, 200 µm tall - 75 wide (15000 µm², exactly the
cap), 80 wide (16000 µm², over the cap, one side over 80), 100 wide (20000 µm², two sides
over 80) and 80.005 wide (16001 µm², two sides over 80 by one grid step).

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DE.3.h1` | 3 / 3 / 3 | 28 |

Both tools flag the 80 x 200 marker; gdscheck flags the three over-area shapes and
KLayout outputs every edge of them plus every edge over 80.001 µm long, which is what its
`ndmy.with_area(15_000.um, nil).edges.join(...)` comes to.

Verdict: two violations - the 100 x 200 and the 80.005 x 200 - and the 80 x 200 clean.
The rule has two halves and the second one is not a second restriction: 80 x 80 is 6400
µm², so *any* rectangle over 15000 µm² already has a side over 80, and read as a further
restriction the clause would forbid nothing that the cap does not already forbid and
would be dead text.  Read as the relief it is written as - over the cap is allowed
provided the marker is long rather than broad - it says something, and it says the useful
thing: a long thin exclude strip along a sensitive line is permitted, a big square block
of excluded die is not.  Both tools implement the cap alone; upstream's second line takes
the edges of shapes it has already flagged, so it changes no verdict either.

## Read and left alone

- DE.3 and DE.4 name NDMY only, and the deck reads NDMY only - a PMNDMY of any size and
  two PMNDMY markers at any distance are clean in both tools, which is what the manual
  says.
- "Merge if space is less" in DE.4 is an instruction to the person drawing the markers;
  what reaches the deck is a gap under 20 µm, and flagging it is right.  Both tools do.
- `min_width` gives one marker per wall, so a 0.795 x 5 bar counts twice; `DE.2.h1`
  reports 4 against KLayout's 2 for the same two bars.  That is the round's settled
  marker-granularity difference, not a finding.
- KLayout reports 3 markers where gdscheck reports 2 on `DE.4.h1`: it cuts the
  corner-to-corner violation into two edge pairs.  One gap is one violation.

## Tested and clean (no need to redo)

DE.2 at 0.8 / 0.795 on NDMY and on PMNDMY separately.  DE.3 at exactly 15000 µm².  DE.4
at 20 / 19.995 wall to wall and 20.004 / 19.997 corner to corner; the same 19.995 gap
opening on x = 40, a notch open across x = 140, a gap opening on y = 20 and a 20 µm gap
on x = 40 that stays clean - at tiles 20, 7 and 100.

---

## Resolution (2026-09-22)

Both findings were fixed in the deck.

- **Finding 1.** DE.2 is two entries, one on NDMY and one on PMNDMY, because the rule is
  the size of an NDMY *or* of a PMNDMY and each answers for itself. `DE.2.h2` reports the
  narrow strip of each layer where the union hid it.
- **Finding 2.** DE.3 is a `forbidden` on the marking that is both over 15000 µm² and over
  80 µm on *both* sides, built the way PRES.9b's big marking is (`with_area` then
  `with_bbox_min`). The cap alone made the rule's second sentence dead text; this is the
  reading under which it has work to do, and `DE.3.h1` reports the two markings that break
  it and passes the 80 x 200 one.

DE.1 is in Appendix B, with the starred rules of section 13 - it is prose about when to
draw the markers at all, and there is nothing to measure.
