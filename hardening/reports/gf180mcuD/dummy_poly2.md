<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dummy_poly2: hardening report

Deck `dummy_poly2` against section 13.2 of the GF180MCU design manual (DPF, dummy Poly2
fill).  36 layouts under `tests/data/gf180mcuD/generated/dummy_poly2/DPF.*.h<n>.gds.gz`,
drawn by `gen/gf180mcuD/dummy_fill.rs`, each with a `#[case]` in the
`hardening_dummy_poly2` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck
at tiles 20, 7 and 100 and through the upstream KLayout runset
(`hardening/oracle-gf180.sh`, `DECKS=dummy_poly2`).

**No count moved with the tile size, on any layout, on any rule.**

`show-deck` lists seventeen entries: DPF.1 as a `forbidden` on the poly fill that covers
no dummy COMP, DPF.2b as a space and a notch, and DPF.4, 5, 6a-d, 8, 9, 11, 12, 13, 14,
16 and 19 as spaces.  That is the foundry's `dummy_poly2.rb` plus DPF.12 and DPF.13,
which upstream has commented out behind a `DUMMY_SUB_PREV` flag ("wafer.space moves all
fill shapes to active metal layers") although the manual marks both "For DRC" - gdscheck
is right to carry them and the oracle is simply silent on those two layouts.  Missing
against the manual: DPF.7 (25.7 µm to the scribe line), DPF.10 (truncated squares),
DPF.15 (no fill under IND_MK), DPF.17 (none under MTPMARK) and DPF.18 (none under
PMNDMY), all marked "For DRC" and none of them in Appendix B's "rules not coded" list.

Every dummy poly square in these layouts carries the dummy COMP core DPF.1 asks for, so
that rule has nothing to say in the other thirty-five.  As in section 13.1 the `h1` of
each distance rule is four probes - the value, a 0.005 µm step past it, and the same gap
corner to corner either side of the value - and gdscheck answered all fourteen correctly;
the section is euclidian throughout.  The findings are all about shapes that touch, and
about the five rules that are not there.

## Findings

### 1. An abutting pair and an overlapping pair are not measured at all (false negative)

Manual: "DPF.4  Space of dummy poly2 to COMP  3.2", and the same sentence for poly2 (5),
RES_MK (19.7), Pad (6.7), NDMY (29.7), Metal1 (2), Metal2 (2), IND_MK (3), MTPMARK (3)
and PMNDMY (8).

Layouts `DPF.4.h2`, `DPF.5.h2`, `DPF.8.h2`, `DPF.9.h2`, `DPF.11.h2`, `DPF.12.h2`,
`DPF.13.h2`, `DPF.14.h2`, `DPF.16.h2`, `DPF.19.h2`: a fill square sharing one edge with
the partner shape, and a second fill square lying half over its partner.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DPF.4.h2` | 0 / 0 / 0 | 1 |
| `DPF.5.h2` | 0 / 0 / 0 | 1 |
| `DPF.8.h2` | 0 / 0 / 0 | 1 |
| `DPF.9.h2` | 0 / 0 / 0 | 1 |
| `DPF.11.h2` | 0 / 0 / 0 | 1 |
| `DPF.12.h2`, `DPF.13.h2` | 0 / 0 / 0 | 0 (rule not coded upstream) |
| `DPF.14.h2` | 0 / 0 / 0 | 1 |
| `DPF.16.h2` | 0 / 0 / 0 | 1 |
| `DPF.19.h2` | 0 / 0 / 0 | 1 |

KLayout's single marker is always the abutting pair, read as a separation of zero.

Verdict: two violations in each.  The abutting one is the round's settled reading - a
shared edge is a space of nothing - and KLayout agrees.  The overlapping one is the same
sentence taken seriously: DPF.12 and DPF.13 are the clearest case, because those two name
a *lateral* distance between a fill and a conductor a level away, and a fill with the
circuit Metal1 running straight over it has none of that distance at all.

### 2. The space to a well boundary is read only from outside the well (false negative)

Manual: "DPF.6a  Space of dummy poly2 to Nwell boundary  1", and the same for DNWELL (2),
LVPWELL (1) and Dualgate (1).

Layouts `DPF.6a.h2`, `DPF.6b.h2`, `DPF.6c.h2`, `DPF.6d.h2`: one well or Dualgate box with
three fills in it - one a step nearer the boundary than the value, one deep inside, one
hanging over the far boundary.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DPF.6a.h2` | 0 / 0 / 0 | 0 |
| `DPF.6b.h2` | 0 / 0 / 0 | 0 |
| `DPF.6c.h2` | 0 / 0 / 0 | 0 |
| `DPF.6d.h2` | 0 / 0 / 0 | 0 |

Verdict: two violations in each, the deep one clean.  The reasoning is section 13.1's -
DCF.1a calls what DCF.6a-e define "the Region", a region set by a distance to a boundary
is a band that straddles it, and 13.2's rules are the same rules at a smaller value -
and it is written out in the `dummy_comp` report, finding 2.

### 3. DPF.15, DPF.17 and DPF.18 are missing: fill under a marker (no rule)

Manual: "DPF.15  Dummy poly2 should not exist under IND_MK layer", "DPF.17  Dummy poly2
should not exist under MTPMARK layer", "DPF.18  Dummy poly2 cannot exit under the marking
layer "PMNDMY"" - three rule numbers of their own, all three "For DRC".

Layouts `DPF.15.h1`, `DPF.17.h1`, `DPF.18.h1`: a fill square in the middle of a 14 µm
marker.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DPF.15.h1` | 0 / 0 / 0 | 0 |
| `DPF.17.h1` | 0 / 0 / 0 | 0 |
| `DPF.18.h1` | 0 / 0 / 0 | 0 |

Verdict: `DPF.15`, `DPF.17` and `DPF.18` respectively.  The deck carries the *space* to
each of those three markers (DPF.14, DPF.16, DPF.19) and not the prohibition beside it,
which is the case the markers are actually drawn for.  Neither tool has them.  PMNDMY is
the layer the section's own preamble names as the one that excludes dummy poly
generation, so DPF.18 is the rule the whole exclusion mechanism rests on.

### 4. DPF.10 is missing: truncated dummy poly2 (no rule)

Manual: "DPF.1  ... The size of the dummy poly2 is 5.6 umX 5.6um square" and "DPF.10
Remove truncated dummy squares  For DRC".

Layout `DPF.10.h1`: a 5.6 x 5.6 fill over its 5.0 core (clean) and a 5.6 x 3.0 one over a
matching core.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DPF.10.h1` | 0 / 0 / 0 | 0 |

Verdict: `DPF.10`.  Section 13.1's finding 5 applies word for word, including the caveat
that the deck's own good/bad patterns draw 2 µm fill and would have to grow first.

### 5. DPF.7 is missing: the space to the scribe line (no rule)

Manual: "DPF.7  Space from dummy poly2 in prime die to scribe line  25.7  For DRC".

Layout `DPF.7.h1`: an 80 x 60 PR_BNDRY polygon with a fill 25.7 µm inside its left edge
(clean) and one 25.695 µm inside its right edge.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DPF.7.h1` | 0 / 0 / 0 | 0 |

Verdict: `DPF.7` once, for the same reason as DCF.7a: the scribe line begins where the
die ends, and the die is the PR_BNDRY polygon.

## Read and left alone

- DPF.1's own column says "For Drawing", and both gdscheck and the foundry's runset check
  it anyway, as `poly2_dummy.not_covering(comp_dummy)`.  Keeping it is the stricter and
  more useful reading and it is left as it is.  `DPF.1.h1` draws the four relations that
  matter: a 5.6 square over its 5.0 core (clean), a core slid half out from under it
  (fires), no core at all (fires), and a core drawn to the fill's own outline - which
  both tools read as covered, and which is right, because the poly is on the COMP.
- The manual's DPF.1 also fixes the oversize at 0.3 µm per side; nobody checks that the
  ring is 0.3 rather than 0.2, and with DPF.10 coded (finding 4) the fill's size would be
  pinned from the other end anyway.
- KLayout reports 3 markers where gdscheck reports 2 on every `h1`: it cuts the
  corner-to-corner violation into two edge pairs.  One gap is one violation.
- DPF.2a, DPF.3 (the drawing algorithm's 2.4 µm pitch and 1.6 µm stagger) are marked "For
  Drawing" and are not gaps.
- DPF.12 and DPF.13 fire in gdscheck and not in the runset (`DPF.12.h1`, 2 against 0).
  The manual marks both "For DRC"; upstream disabled them for a fill flow that moves fill
  onto the active layers.  gdscheck is right.

## Tested and clean (no need to redo)

DPF.2b at 1.1 / 1.095 wall to wall and 1.1031 / 1.0960 corner to corner, as a pair and as
a notch; the same 1.095 gap opening on x = 40 and on y = 20, a notch open across x = 20,
and a 1.1 gap on x = 42 that stays clean - at tiles 20, 7 and 100.  DPF.4 (3.2), DPF.5
(5.0), DPF.6a (1.0), DPF.6b (2.0), DPF.6c (1.0), DPF.6d (1.0), DPF.8 (19.7), DPF.9 (6.7),
DPF.11 (29.7), DPF.12 (2.0), DPF.13 (2.0), DPF.14 (3.0), DPF.16 (3.0) and DPF.19 (8.0),
each at the value, one step under it, and both ways round corner to corner.  DPF.1's four
covering relations.  A fill 25.7 µm inside PR_BNDRY.  A 5.6 µm square on its own.

---

## Resolution (2026-09-22)

Section 13.2 is 13.1 on Poly2, and the fixes landed together; see the resolution of
`hardening/reports/gf180mcuD/dummy_comp.md` for the reasoning of each.

- Every space carries `abutting: report`, and DPF.4, 5, 8, 9, 11, 12 and 13 gained the
  `forbidden` half for the fill that lies on its partner.
- DPF.6a-d read the well's boundary from both sides (enclosure plus crossing half).
- DPF.15, DPF.17 and DPF.18 are in the deck now, as the prohibitions beside DPF.14's,
  DPF.16's and DPF.19's spaces - which is why the `h2` of each of those three reports one
  of each id.
- DPF.7 and DPF.10 are Appendix B's, with DCF.7a and DCF.10; their layouts stay with the
  cases expecting nothing.
