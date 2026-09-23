<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / otp_mk: hardening report

Deck `otp_mk` against section 10.10 of the GF180MCU design manual, "OTP_MK Mark Layer".

## Coverage

The section's table lists twenty rows.  Five carry a `*` and a value of 0 and the
section's own note says so: *"Rules allowed minimum overlap and spacing are 0, so DRC deck
will not check those rules"* - O.PL.5a, O.PL.5b, O.SB.5a, O.SB.12 and O.SB.15b.  O.SB.5b's
5 V column is `0*` and Appendix B lists O.SB.5b_MV among the rules not coded (as it does
O.SB.15b, and "O.SB.13", whose text there - *"Min Salicide block overlap with poly2 = 0"* -
is O.SB.12's description, not the area rule's).  That leaves sixteen rules with something
to measure: O.DF.3a, O.DF.6, O.DF.9, O.PL.2, O.PL.3a, O.PL.4, O.SB.2, O.SB.3, O.SB.4,
O.SB.5b_LV, O.SB.9, O.SB.11, O.SB.13_LV, O.SB.13_MV, O.CO.7 and O.PL.ORT.  `show-deck`
lists twenty entries under exactly those sixteen ids.  **No coverage finding.**

## What was drawn

Every fixture is one or more marked cells - a COMP with a horizontal poly line across it -
plus whatever the rule measures against, each cell under its own marker with 1 µm to
spare.  The gate lines run horizontally throughout because O.PL.ORT forbids the other
orientation and a turned gate anywhere would answer for the fixture; its own fixture is
the only one that turns one.

17 layouts under `tests/data/gf180mcuD/generated/otp_mk/O.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/otp_mk.rs`, each with a `#[case]` in `hardening_otp_mk`.  Every layout ran
at tiles 20, 7 and 100 and through the upstream runset (`DECKS=otp_mk`).  **No count moved
with the tile size**, on any layout, including the six gaps laid across the lines at
x = 20, 21, 40, 42 and y = 20 in `O.DF.3a.h1`, `O.PL.3a.h1` and `O.SB.2.h1`.

## Findings

### 1. A 45° active edge eats the source/drain overhang unseen (false negative)

O.DF.6: *Min. COMP extend beyond poly2 (it also means source/drain overhang)* - 0.22 µm.

`O.DF.6.h2` is three 1 × 1.4 µm actives, each with a 0.22 µm gate line across the middle
(y = 10.59 to 10.81) reaching 0.2 µm past it left and right.  Each has its top-left corner
cut away at 45°, starting lower each time.

| device | the cut | active over the gate's top wall, at its left end | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- | --- |
| 1 | (10, 11.03) to (10.37, 11.4) | 0.22 straight up, 0.1556 to the 45° wall | - | - |
| 2 | (20, 11.0) to (20.4, 11.4) | 0.19 straight up | - | - |
| 3 | (30, 10.83) to (30.57, 11.4) | 0.02 straight up | **nothing** | O.DF.6, edge pair (30.02,10.81;30.291,10.81)/(30,10.83;30.2,11.03) |

The third device is the whole argument: the gate line's top wall has two hundredths of a
micron of active above its left end where the rule asks 0.22, and there is no metric under
which that passes.  gdscheck reports nothing on it.  The first device is the settled
reading of 2026-09-21 applied to an extension - the enclosure is euclidian, so a 45° wall
passing under the value from a corner fires - and neither tool reads it; upstream asks for
`projection` on this rule by name (`poly_otp.enclosed(comp_otp, 0.22.um, projection)`),
which is why it misses the first two and catches the third.  The case expects O.DF.6 three
times.

The shape is an ordinary one: chamfering the corner of a source island to clear a
neighbour is how the corner gets drawn, and it is exactly where the overhang is thinnest.

### 2. The marker is cut into the layers, so a shape it half-covers is measured in halves (false positive, both tools)

Section 10.10's first line is *"This layer is used to mark 3.3V OTP cells"*, and the
settled reading of a marker naming a cell is that it selects whole regions - never cut a
region geometrically where the rule names a cell, because the cut invents walls.  Both
decks build `comp_otp` as `comp ∧ otp_mk` instead, and two fixtures show what that costs.

`O.DF.9.h1` is three actives.  The first is 0.38 × 0.38 = 0.1444 µm², the value exactly;
the second 0.38 × 0.375 = 0.1425; the third is a 1 × 1 µm active - seven times the
minimum - with the marker covering only a 0.3 µm square of its bottom-left corner.

| active | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- |
| 0.1444 µm² | - | - |
| 0.1425 µm² | O.DF.9, area 0.1425 at (20.19, 10.1875) | O.DF.9, polygon (20,10)-(20.38,10.375) |
| 1 µm², marker over a 0.3 corner | O.DF.9, **area 0.09** at (30.15, 10.15) | O.DF.9, polygon (30,10)-(30.3,10.3) |

`O.DF.3a.h2` is one solid 3 × 3 µm COMP with no notch anywhere in it, under a marker
shaped like a U whose 0.2 µm slot runs in from the right.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | O.DF.3a, notch 0.2 at (12, 10.9)-(12, 11.1) |
| KLayout runset | O.DF.3a, edge pair (11,10.9;13,10.9)/(13,11.1;11,11.1) |

The notch is the marker's, not the active's.  A COMP of 1 µm² is a COMP of 1 µm² whatever
shape the marker over it has, and a solid plate has no notch to measure; the cases expect
one O.DF.9 on `h1` and nothing on `h2`.  The two rules that read a COMP *as a region* -
its area and the space around it - want `comp interacting otp_mk`; the rules that read one
shape against another across the marker (O.DF.6, O.PL.4, O.SB.11) are unaffected either
way, because the marker in a real cell covers both.

### 3. A shared edge is not reported as a space (false negative, three rules)

O.SB.3 (*Min. space from salicide block to unrelated COMP* - 0.09 µm), O.SB.4 (*Min. space
from salicide block to contact* - 0.03 µm) and O.CO.7 (*Min. space from COMP contact to
Poly2 on COMP* - 0.13 µm) each have a probe drawn flush against the shape it is measured
from.

| fixture | probe | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| `O.SB.3.h1` | 0.09 / 0.085 / flush | O.SB.3 once, at (21.6, 10)-(21.685, 10) | O.SB.3 twice, the second at x = 31.6 |
| `O.SB.4.h1` | 0.03 / 0.025 / flush / under the block | O.SB.4 twice (0.025 and the contact under the block at (40.71, 10.51)) | O.SB.4 three times, the extra at x = 31.6 |
| `O.CO.7.h1` | 0.13 / 0.125 / flush | O.CO.7 once, at (21, 10.62)-(21, 10.745) | O.CO.7 twice, the second at x = 31 |

A shared edge is a space of nothing, and nothing is under every one of those three values;
the settled reading (`abutting: report`) says the pair fires.  None of the three entries
carries the parameter.  The same class is finding 3 of the `nat` report.

Note that the equivalent probe is *not* drawn for O.DF.3a, O.SB.2 and O.PL.3a: those three
measure one layer against itself, and two shapes drawn edge to edge on one layer are one
region after merging, with no space between them at all.  Both tools are right to stay
silent there, and the fixtures keep the probe as a clean case.

### 4. A Dualgate OTP cell falls between O.SB.13's two voltage classes (no rule at all, both tools)

O.SB.13: *Min. area of silicide block (µm²)* - 1.488 at 3.3 V, 2 at 5 V.

The deck derives `sab_otp_lv` as `sab_otp not_overlapping dualgate` and `sab_otp_mv` as
`sab_otp ∧ v5_xtor`.  Those two are not complementary: a block under Dualgate and no
V5_XTOR is in neither, and its area is never measured.  `O.SB.13.h1`'s fifth probe is
exactly that - a 1 µm² block in an OTP cell that Dualgate covers.

| block | marking | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| 1.2 × 1.24 = 1.488 | none | - | - |
| 1.2 × 1.235 = 1.482 | none | O.SB.13_LV, area 1.482 | O.SB.13_LV |
| 1.25 × 1.6 = 2.0 | V5_XTOR | - | - |
| 1.25 × 1.595 = 1.99375 | V5_XTOR | O.SB.13_MV, area 1.9937 | O.SB.13_MV |
| 1.0 × 1.0 = 1.0 | Dualgate | **nothing** | **nothing** |

The settled reading is that an LV/MV pair is the complementary pair `not_overlapping` /
`overlapping` the Dualgate marker, so the last block is the 5 V one and 1 µm² is half of
the two square microns it owes.  Upstream has the same hole for the same reason
(`not_interacting(v5_xtor).not_interacting(dualgate)` against `and(v5_xtor)`), so the
oracle agrees with gdscheck and both leave the block unchecked.  The case expects
O.SB.13_MV twice.

The deck is also inconsistent with itself here: O.SB.5b_LV splits on Dualgate alone
(`sab_otp not_overlapping dualgate`) while O.SB.13_MV splits on V5_XTOR, so the two
"3.3 V" classes of one deck are different sets.  Whichever marker is chosen, the LV and MV
halves of a rule should be the two halves of one split.

## Read and left alone

- **Marker granularity.**  `min_gate_length` and `min_enclosure` give gdscheck one marker
  per wall against KLayout's one edge pair: O.PL.2 is 2 against 1 on `O.PL.2.h1`, and
  O.PL.4 is 2 against 2 on `O.PL.4.h1` only because both of that gate's end caps are
  short.  `min_space` at a corner is the other way, 1 against 2 on `O.DF.3a.h1`.  Settled;
  the violating structures agree everywhere.
- **O.SB.5b_LV on a cell marked with V5_XTOR and no Dualgate.**  `O.SB.5b.h1`'s fourth
  probe puts the 0.095 gap under V5_XTOR alone.  gdscheck reads it as the 3.3 V cell and
  fires; KLayout reads V5_XTOR as the 5 V marker and stays silent.  Kept as gdscheck has
  it, by the settled LV/MV reading: the classes split on Dualgate, which is the thick
  oxide, and no V5_XTOR step comes into it.  It is worth recording that this section is
  the one place in the manual that says otherwise - note 1 of the table reads *"Rule of 5V
  OTP is set by utilizing 'V5_XTOR' and 'OTP_MK' marking layer"* - and that the deck
  already follows that note for O.SB.13_MV and O.PL.ORT.  Whoever settles finding 4 should
  settle this sentence with it.
- **O.SB.11 on a block that overlaps nothing.**  KLayout adds an O.SB.11 marker wherever a
  block shares an edge with a COMP (`O.SB.3.h1`'s third probe, `O.SB.4.h1`'s third), even
  when that block overlaps another COMP by 0.3 µm elsewhere.  gdscheck stays silent and is
  right: *"Min. salicide block overlap with COMP"* measures how far a block reaches into
  the COMP it blocks, and a block that blocks no COMP has no overlap to bound - otherwise
  every block over field poly would be a violation.
- **O.SB.9 on a poly line that runs out of the block's ends.**  `O.SB.9.h1`'s third probe
  is a block sitting on a length of gate line, the line running out through both its
  vertical edges.  Both tools stay silent, and that is how every unsalicided segment is
  drawn: the extension the rule asks for is across the line, and the cut edges along it
  are coincident by construction (the deck's `skip_coincident: 1`).
- **O.PL.ORT under V5_XTOR.**  `O.PL.ORT.h1`'s third device is a turned gate in a cell
  V5_XTOR covers; both tools exempt it, which is the rule's own column ("DRC" at 3.3 V,
  "NA" at 5 V).
- **O.DF.6 and O.PL.ORT riding along on `O.PL.4.h1`.**  Its third device has the gate line
  stopping 0.2 µm inside the active, which is no end cap at all (O.DF.6 reads the 0.2 of
  active beyond the poly's end as the overhang) and leaves a channel edge running along y
  (O.PL.ORT).  Both tools report both, at the same place; the case keeps them.

## Tested and clean

Drawn at the value and one step past it, and answered exactly as the manual reads at all
three tile sizes and in agreement with the runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| O.DF.3a | actives 0.24 / 0.235 apart on four tile lines, a 0.2343 euclidian corner, and a merged pair | `O.DF.3a.h1` |
| O.DF.6 | the overhang 0.22 / 0.215 | `O.DF.6.h1` |
| O.DF.9 | 0.1444 µm² / 0.1425 µm² | `O.DF.9.h1` |
| O.PL.2 | the channel 0.22 / 0.215 | `O.PL.2.h1` |
| O.PL.3a | two gate lines 0.18 / 0.175 apart, a 0.175 slot on the line at x = 21, and a merged pair | `O.PL.3a.h1` |
| O.PL.4 | the end cap 0.14 / 0.135, both ends | `O.PL.4.h1` |
| O.PL.ORT | a horizontal gate, a turned one, a turned one under V5_XTOR | `O.PL.ORT.h1` |
| O.SB.2 | blocks 0.28 / 0.275 apart across x = 42, a 0.275 slot across x = 21, a merged pair | `O.SB.2.h1` |
| O.SB.3 | block to unrelated active 0.09 / 0.085 | `O.SB.3.h1` |
| O.SB.4 | contact 0.03 / 0.025 from the block, and a contact under it | `O.SB.4.h1` |
| O.SB.5b_LV | block to an uncovered gate line 0.1 / 0.095, and the same under Dualgate (the 5 V rule, not coded) | `O.SB.5b.h1` |
| O.SB.9 | the block reaching 0.1 / 0.095 past the line it blocks | `O.SB.9.h1` |
| O.SB.11 | the block covering the active 0.04 / 0.035 | `O.SB.11.h1` |
| O.SB.13_LV | 1.488 µm² / 1.482 µm² | `O.SB.13.h1` |
| O.SB.13_MV | 2.0 µm² / 1.99375 µm² under V5_XTOR | `O.SB.13.h1` |
| O.CO.7 | contact to gate line 0.13 / 0.125 | `O.CO.7.h1` |

The deck's own good/bad pairs were left as they were; nothing this round drew moved them.

---

## Resolution (2026-09-22)

- **Finding 1 (O.DF.6).** The rule reads `metric: euclidian` now, which finds the third
  device - 0.02 µm of active over the gate's top wall where the rule asks 0.22, and both
  tools were blind to it - and the second, 0.19 µm. The first device is kept and open:
  every straight margin clears 0.22 while the chamfer comes within 0.1556, and neither
  tool reports it. Two things about that fixture are worth knowing for whoever picks it
  up: the devices moved from x = 10, 20, 30 to 12, 22, 32, because the margin this rule
  reads runs along the active's left edge and on the 20 µm tile line the second device's
  0.19 went unread, which is a gdscheck defect of its own, reproducible by moving the
  fixture back.

### The tile-line miss, traced (open)

Three copies of that second device at x = 20, 24 and 42 report at every tile size except
the one whose lines fall on them: at tile 20 the x = 20 device is silent, at tile 7 the
x = 42 one is, at tile 100 all three report.  The mechanism is in the enclosure scan's
`uncut`, which exists so that a margin read from a vertex the tiling made is re-read on
the whole wall and kept by the tile that owns the closest point.  Where the crossing
point lies *on* the line, no tile sees that wall whole - the shape crosses there and
every copy stops at it - so `uncut` finds a clip rather than a wall, defers to "the tile
past it", and every tile defers.

The obvious repair - let the tile that owns the point half-open (`Core::owns_from`) keep
the read on its own piece - was tried and reverted: it finds the missing pair at every
tile size, but the piece's read is not the whole wall's, and on the `lvpwell` foundry
layout LPW.3 became tile-dependent (90 / 92 / 94 at 100 / 20 / 7) where it had been 90
everywhere.  A repair has to give the owning tile the *wall*, not its piece - by way of a
halo on the enclosed layer of at least the rule's value, with the double report that then
arrives from both sides of the line collapsed - and that is a piece of engine work of its
own.  Left for the gdscheck owner.
- **Finding 2.** `comp_otp`, `sab_otp` and `poly_otp` are `overlapping` selections: the
  marker names cells and selects whole regions. `O.DF.9.h1` reports the 0.1425 µm² active
  and not the 1 µm² one whose corner the marker clips, and `O.DF.3a.h2` - a solid plate
  under a U-shaped marker - is clean.
- **Finding 3.** O.SB.3, O.SB.4 and O.CO.7 carry `abutting: report`.
- **Finding 4.** `sab_otp_lv` and `sab_otp_mv` split on V5_XTOR and are complementary, as
  note 1 of the section has it ("Rule of 5V OTP is set by utilizing 'V5_XTOR' and 'OTP_MK'
  marking layer") and as section 11 words the same split for its SRAM cells. The Dualgate
  cell's 1 µm² block is a 3.3 V cell's and owes O.SB.13_LV's 1.488, which it now gets.
