<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dummy_comp: hardening report

Deck `dummy_comp` against section 13.1 of the GF180MCU design manual (DCF, dummy COMP
fill).  23 layouts under `tests/data/gf180mcuD/generated/dummy_comp/DCF.*.h<n>.gds.gz`,
drawn by `gen/gf180mcuD/dummy_fill.rs`, each with a `#[case]` in the
`hardening_dummy_comp` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck
at tiles 20, 7 and 100 and through the upstream KLayout runset
(`hardening/oracle-gf180.sh`, `DECKS=dummy_comp`).

**No count moved with the tile size, on any layout, on any rule.**

`show-deck` lists eleven entries: DCF.2b as a space and a notch, and DCF.4, 5, 6a, 6b,
6c, 6d, 8a, 11a and 12 as spaces from the fill to one other layer.  That is exactly the
foundry's own `dummy_comp.rb`, and both stop short of the section in the same places.
Against the manual the section has more in it: DCF.1c/DCF.10 (the fill is a 5 x 5 square
and a truncated one shall not exist, "For DRC"), DCF.7a (26 µm from the prime die's fill
to the scribe line, "For DRC"), DCF.13 (no fill under IND_MK), and the "at least one row
of dummy COMP between the marker and the transistor channel" halves of DCF.8b, DCF.11b
and DCF.13.  Appendix B's "rules not coded" list names nothing from section 13, so none
of these are exempt there.  DCF.1b and DCF.1d are the global densities and live in the
`density` deck, where the last round hardened them.

Four shapes to a layout is the rule here: the `h1` of every distance rule is the value
(clean), one 0.005 µm step past it, the same gap taken corner to corner just under the
value, and the corner gap one step wider (clean).  gdscheck answered all nine of those
correctly - the section is euclidian throughout, as the runset's `separation(..., 
euclidian)` also has it - so what follows is all about what happens where the two shapes
*touch*.

## Findings

### 1. An abutting pair and an overlapping pair are not measured at all (false negative)

Manual: "DCF.4  Space from dummy COMP to COMP (circuit COMP).  3.5" - and the same
sentence for DCF.5 (poly2, 1.5), DCF.8a (RES_MK, 3.5), DCF.11a (NDMY, 3.5) and DCF.12
(IND_MK, 3.0).

Layouts `DCF.4.h2`, `DCF.5.h2`, `DCF.8a.h2`, `DCF.11a.h2`, `DCF.12.h2`: a 2 µm fill
square sharing one edge with the partner shape, and a second fill square lying half over
its partner.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.4.h2` | 0 / 0 / 0 | 1 |
| `DCF.5.h2` | 0 / 0 / 0 | 1 |
| `DCF.8a.h2` | 0 / 0 / 0 | 1 |
| `DCF.11a.h2` | 0 / 0 / 0 | 1 |
| `DCF.12.h2` | 0 / 0 / 0 | 1 |

KLayout's single marker is the abutting pair - `edge-pair: (12,12;12,10)/(12,10;12,12)`
in `DCF.4.h2`, the shared edge read as a separation of zero.  Neither tool says anything
about the overlapping pair.

Verdict: two violations in each.  The abutting one is the round's settled reading - a
shared edge is a space of nothing, `abutting: report` - and KLayout agrees with it here.
The overlapping one follows from the same sentence: a rule that asks for 3.5 µm of space
cannot be satisfied by a fill lying *on* the circuit COMP, which has none of it, and the
whole purpose of section 13.1 is that dummy fill keeps out of the way of circuit
features.  KLayout misses it because `separation` needs two edges facing across empty
ground and an overlap gives it none.

### 2. The space to a well boundary is read only from outside the well (false negative)

Manual: "DCF.6a  Space from dummy COMP to Nwell boundary  1.3", and the same for DNWELL
(4), LVPWELL (1.3) and Dualgate (1.3).  DCF.1a names what those four rules carve out:
"All area between active polygons (COMP) ... must be filled with Dummy COMP except area
marked by NDMY, RES_MK, Pad and IND_MK, as well as the **Region define by DCF.6a, 6b,
6c, 6d, 6e**".

Layouts `DCF.6a.h2`, `DCF.6b.h2`, `DCF.6c.h2`, `DCF.6d.h2`: one well (or Dualgate) box
with three fill squares in it - one a step nearer the boundary than the value
(1.295 / 3.995 / 1.295 / 1.295), one as deep inside the box as it goes, and one hanging
over the far boundary.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.6a.h2` | 0 / 0 / 0 | 0 |
| `DCF.6b.h2` | 0 / 0 / 0 | 0 |
| `DCF.6c.h2` | 0 / 0 / 0 | 0 |
| `DCF.6d.h2` | 0 / 0 / 0 | 0 |

Verdict: two violations in each - the near one and the one crossing the boundary - and
the deep one clean.  A rule whose subject is a *boundary* and whose region DCF.1a names
as a region is a band that straddles the boundary; that is the only reading under which
the second half of DCF.1a's sentence has any work to do, since fill is generated across
the whole prime die and a well is part of it.  Read the other way - fill must never be
inside a well at all - the near and the crossing fills are violations too, so the case
expects the same answer either way, and only the deep fill separates the two readings
(it is drawn, and expected clean, because fill over a well is ordinary).  Both tools
implement a plain outside-facing separation and see none of it.

### 3. A fill wholly under RES_MK or NDMY is not reported (false negative)

Manual: "DCF.8a  Space from dummy COMP to Resistor marking layer (RES_MK).  **Dummy COMP
should not exit under RES_MK**  3.5" and "DCF.11a  **Dummy COMP cannot exit under "NDMY"
marking layer** and space from dummy COMP to dummy COMP excluding layer (NDMY)  3.5".

Layouts `DCF.8a.h2` and `DCF.11a.h2`, third probe: a 2 µm fill square in the middle of an
8 µm marker.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.8a.h2` third probe | 0 / 0 / 0 | 0 |
| `DCF.11a.h2` third probe | 0 / 0 / 0 | 0 |

Verdict: one violation each, on top of finding 1's two, which is why those two cases
expect three.  The manual states the prohibition in as many words and gives it the same
rule number as the space; a fill entirely inside the marker is the case it is written
for, and it is the one a marker is most likely to be drawn around.  The foundry's deck
codes only the `separation` half and its `output` string drops the sentence.

### 4. DCF.13 is missing: dummy COMP under IND_MK (no rule)

Manual: "DCF.13  Dummy COMP should not exist under IND_MK layer ..."  (the rest of the
cell is the 80 µm "one row between IND_MK and the transistor channel" clause).

Layout `DCF.13.h1`: a 2 µm fill square inside a 14 µm IND_MK.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.13.h1` | 0 / 0 / 0 | 0 |

Verdict: `DCF.13`.  The deck has DCF.12 (the 3 µm space to IND_MK) and nothing for the
prohibition, which is a separate rule number in the manual and measurable as it stands.
Neither tool carries it.

### 5. DCF.10 is missing: truncated dummy COMP (no rule)

Manual: "DCF.1c  A staggered array of dummy COMP 5 umX5 um squares is created ...  5X5"
and "DCF.10  Remove truncated dummy squares (Truncated Dummy COMP shall not exist)", both
marked for DRC.

Layout `DCF.10.h1`: a 5 x 5 dummy COMP (clean), a 5 x 2.5 half square, and a 5 x 5 with a
3 x 3 bite out of one corner.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.10.h1` | 0 / 0 / 0 | 0 |

Verdict: `DCF.10` twice.  The two rules together fix the fill's shape, and a shape that
is not the 5 µm square is the truncated one DCF.10 forbids.  A caveat for whoever codes
it: the deck's own good/bad patterns draw the fill as 2 µm squares (`FILL` in
`gen/gf180mcuD/dummy_fill.rs`), so a size rule would fire on every one of them - the
fixtures would have to grow to 5 µm first.  That is a cost, not an argument: the rule is
in the manual, it is marked for DRC, and Appendix B does not excuse it.

### 6. DCF.7a is missing: the space to the scribe line (no rule)

Manual: "DCF.7a  Space from dummy COMP in prime die including guard ring to the scribe
line  26  For DRC".

Layout `DCF.7a.h1`: an 80 x 60 PR_BNDRY polygon with a fill square 26 µm inside its left
edge (clean) and one 25.995 µm inside its right edge.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DCF.7a.h1` | 0 / 0 / 0 | 0 |

Verdict: `DCF.7a` once.  At chip level the scribe line begins where the die ends, and the
die is the PR_BNDRY polygon the density round settled on - so the rule is the fill's
distance to that polygon's edge, measured from inside.  DCF.7b, DCF.7c and DCF.7d are
about the frame and the SLM test structures of the scribe line itself and have nothing to
measure in a chip layout; they are not counted as gaps here.

## Read and left alone

- KLayout reports 3 markers where gdscheck reports 2 on every `h1` layout: it cuts the
  corner-to-corner violation into two edge pairs (in `DCF.4.h1`,
  `(49.99,12;50,12)/(52.48,14.47;52.47,14.47)` and `(50,12;50,11.99)/(52.47,14.47;
  52.47,14.48)`), gdscheck names the corner once.  One gap is one violation.
- DCF.2b is one rule with two gdscheck entries, a space and a notch; a U whose opening is
  1.895 reports under the same id as a pair 1.895 apart, and the case counts them
  together.
- DCF.9 ("Space from dummy COMP to Pad 7") says "Guideline only, no DRC" in its own last
  column, and DCF.2a and DCF.3 say "For Drawing"; none is a gap.
- DCF.8b, DCF.11b and DCF.13 each end in a clause asking for "at least one row of dummy
  COMP (or substrate tie) between the marker and the transistor channel active" when the
  marker is over 80 µm both ways.  That is a rule about a fill row that the DRC deck has
  no way to recognise as such, and no fixture is drawn for it; DCF.13's first sentence,
  which is plain geometry, is finding 4.
- DCF.1a - all ground between COMP polygons more than 20 µm apart must be filled - is the
  fill-coverage rule that the `density` deck's DCF.1b answers in aggregate.  Coded
  literally it would need the fill generator's own footprint, and it is left alone here.

## Tested and clean (no need to redo)

DCF.2b at 1.9 / 1.895 wall to wall and 1.9021 / 1.8950 corner to corner, as a pair and as
a notch; the same 1.895 gap opening on x = 40 and on y = 20, a notch open across x = 20,
a square crossing x = 42, and a 1.9 gap on x = 42 that stays clean - at tiles 20, 7 and
100.  DCF.4 (3.5), DCF.5 (1.5), DCF.6a (1.3), DCF.6b (4.0), DCF.6c (1.3), DCF.6d (1.3),
DCF.8a (3.5), DCF.11a (3.5) and DCF.12 (3.0), each at the value, one step under it, and
both ways round corner to corner.  A fill 26 µm inside PR_BNDRY.  A 5 x 5 dummy COMP on
its own.

---

## Resolution (2026-09-22)

Findings 1, 2 and 3 were fixed in the PDK; findings 5 and 6 were withdrawn, and 4 coded.

- **Finding 1.** Every space of the deck carries `params: abutting: report`, and DCF.4
  and DCF.5 gained a `forbidden` half on the fill that lies over its partner. A shared
  edge is a space of nothing and an overlap is less than nothing.
- **Finding 2.** DCF.6a-d read the boundary from both sides: a `min_enclosure`
  (euclidian, `interacting_only`) holds a fill inside the well the same distance from its
  edge, and a `forbidden` half reports the fill that crosses it. `DCF.6a.h2` and its three
  siblings report two each, and the fill deep inside stays clean.
- **Finding 3.** DCF.8a and DCF.11a carry the prohibition their own text states, as a
  `forbidden` on the fill overlapping the marker - which covers the fill wholly inside it
  as well as the one half over it.
- **Finding 4.** DCF.13 is in the deck now, as the prohibition beside DCF.12's space: a
  fill *on* IND_MK is DCF.13's, a fill too close to it is DCF.12's, which is why
  `DCF.12.h2` reports one of each.
- **Findings 5 and 6 withdrawn.** DCF.10 and DCF.7a are in the manual's own Appendix B.
  The report read the appendix as naming nothing from section 13; it names DE.1, DCF.2a,
  DCF.3, DCF.7a-d, DCF.9, DCF.10, DPF.2a, DPF.3, DPF.7, DPF.10, DM.2a, DM.9 and DM.10
  (`drm_16.txt`), and the asterisk on a rule number in the section's own table is what
  puts it there - the same convention as PRES.8* in section 10.1. Both layouts stay, with
  their cases expecting nothing, as the proof that the deck is silent by decision.

The deck header now says which rules of the section it carries and why the rest are out.
