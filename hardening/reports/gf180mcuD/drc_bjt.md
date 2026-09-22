<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / drc_bjt: hardening report

Deck `drc_bjt` against the GF180MCU design manual, section 10.7 (`gf180mcu_drm/drm_10_07.txt`,
"DRC_BJT Mark Layer": BJT.1, BJT.2, BJT.3), with the derived layers of
`pdks/gf180mcuD/pdk.yml`.  Six layouts,
`tests/data/gf180mcuD/generated/drc_bjt/BJT.<n>.h<m>.gds.gz`, drawn by the `hardening` half of
`gen/gf180mcuD/bjt.rs`, each with a `#[case]` in the `hardening_drc_bjt` table of
`tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and through
the upstream KLayout runset (`DECKS=drc_bjt hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

## Coverage

`show-deck` lists BJT.1, BJT.2 and BJT.3 - all three rules of the section.  Nothing is
missing.  Two of them are `forbidden` on a derived layer (`bjt1_viol`, `bjt2_viol`), which is
how a "minimum overlap of 0" reads: the marker must contain the shape, with no margin.

## How the section reads

DRC_BJT "is used to mark vertical NPN and PNP transistors to avoid design rule violation".
BJT.1 applies to an NPN, and the deck recognises one the way upstream does: a marker that
meets an emitter (N+ ∩ LVS_BJT ∩ DNWELL ∩ DRC_BJT), a base (P+ ∩ LVPWELL ∩ DNWELL ∩ DRC_BJT)
and a collector (N+ ∩ DNWELL ∩ DRC_BJT, less LVS_BJT).  So most of the work is recognition and
most of what is drawn here is each way of *not* being the device.  BJT.2 and BJT.3 are not
conditioned on a device at all - any marker answers for the substrate P+ it touches and for
its clearance to any unrelated active.

Marker cuts: gdscheck reports one `forbidden` marker per offending region and one `min_space`
marker per pair; KLayout reports the whole marker polygon for BJT.1 and BJT.2 and one edge
pair per space, splitting a corner pair in two.

**No count moved with the tile size**: six layouts x three tile sizes, every rule identical.
`BJT.3.h2` puts the same 0.095 gap on x = 20, x = 42 and y = 20 on purpose and all three
survive every tile.

Test status on the engine as of this report: 3 of the 6 cases fail (BJT.1.h3, BJT.2.h1,
BJT.3.h1), each on a finding below; the other 3 pass, as do the six older `good`/`bad` cases.

## Findings

### 1. `not_covering` passes as soon as one shape is covered

*BJT.1*, "Min. DRC_BJT overlap of DNWELL for NPN BJT 0"; *BJT.2*, "Min. DRC_BJT overlap of
PCOM in Psub 0".  Both read "the marker must contain every one of these shapes".  gdscheck's
`not_covering` lets a marker off as soon as it contains **one** polygon of the other layer,
however many more it touches and leaves hanging out.  KLayout requires all of them.

`BJT.2.h1` draws five markers, each with its own arrangement of substrate P+:

| structure | geometry | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- |
| (a) | a tap (11, 11)-(12, 12) wholly inside the marker (10, 10)-(13, 13) | clean | clean | clean |
| (b) | a tap (18, 11)-(20, 12) the marker's edge cuts at x = 19 | BJT.2 | fires, (17.5, 11.5) | fires, `(16,10;16,13;19,13;19,10)` |
| (c) | a marker whose only P+ is inside an N-well - no PCOM in the substrate at all | BJT.2 | fires, (23.5, 11.5) | fires, `(22,10;…;25,10)` |
| (d) | one tap (28.5, 11)-(29, 11.5) inside **and** a second (30, 11)-(32, 12) crossing out | BJT.2 | **silent** | fires, `(28,10;…;31,10)` |
| (e) | a tap (36, 11)-(37, 12) whose right wall lies on the marker's at x = 37 - an overlap of exactly 0 | clean | clean | clean |
| total | | 3 | **2** | 3 |

`BJT.1.h3` is the same question on BJT.1: a complete NPN in a deep well wholly inside its
marker, plus a second deep well (18.3, 11)-(21, 15) that runs out of the marker's right edge
at x = 19.

| layout | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `BJT.1.h3` | BJT.1 - the marker does not cover that DNWELL | **0** | 1, `(10,10;10,16;19,16;19,10)` |

Verdict: a false negative in gdscheck, on both rules.  "Min. DRC_BJT overlap of DNWELL" and
"of PCOM in Psub" read distributively - each such shape has to be overlapped - and the
derivations already narrow the set to the shapes that touch the marker (`pcomp_psub
interacting drc_bjt`).  Covering one and not the next is not the rule.  BJT.2 (d) is the
clearer of the two: the second tap is as much "PCOM in Psub" under the marker as the first.
BJT.1.h3 is the weaker case - one could argue the rule means the NPN's *own* deep well - but
a deep well hanging out from under a BJT marker is what the rule exists to stop, and the two
should be decided together since they are one engine primitive.

### 2. BJT.3 is silent where the active's wall lies on the marker's

*BJT.3*, "Minimum space of DRC_BJT layer to unrelated COMP 0.1".  A shared edge is a space of
nothing and is reported (settled in the GF180 rounds, `abutting: report`); the rule does not
carry it.  Same class as `contact.md` finding 5.

`BJT.3.h1` (e): an N+ active (37, 10)-(38, 11) with its left wall on a marker's right wall at
x = 37.

| layout | gdscheck @20/7/100 | KLayout |
| --- | --- | --- |
| `BJT.3.h1` | 2 - `0.0950 at (19.0, 10.0)-(19.095, 10.0)` and `0.0990 at (31.0, 13.0)-(31.07, 13.07)` | 4 - those plus `(37,10;37,11)/(37,11.1;37,10)`, the corner pair counted twice |

Everything else in the layout is right in both tools: 0.1 clean, 0.095 fires, the
corner-to-corner pair at 0.1018 clean and the one at 0.0990 fires, and an active that
overlaps the marker is related and exempt in both.

Verdict: a false negative in gdscheck.  An unrelated active drawn wall-to-wall with the BJT
marker is 0 from it, and 0 is under 0.1.

## Read and left alone

- **A marker with no substrate P+ at all fires BJT.2** (`BJT.2.h1` (c)).  `not_covering` of an
  empty set is the whole marker in both tools.  It reads oddly - "min overlap 0" of something
  that is not there - but it is the right answer for the layer's purpose: a DRC_BJT marker
  with no PCOM in the substrate under it marks no vertical transistor, and both tools say so.
  Kept as the expected answer.
- **The base of a vertical NPN counts as "PCOM in Psub".**  `pcomp_psub` is P+ outside
  `all_nwell`, and `all_nwell` is `nwell ∪ (dnwell − lvpwell)`, so a P+ in the LVPWELL inside
  a deep well - electrically an isolated p-well, not the substrate - is "Psub" for this rule.
  Upstream derives it identically.  It is also what makes BJT.2 useful on an NPN, whose only
  P+ terminal is that base, so the reading stands.
- **An active that overlaps the marker is exempt whole.**  `comp_outside_bjt` is `comp
  not_overlapping drc_bjt` (upstream `comp.outside(drc_bjt)`), so an active that runs half
  under the marker is related and the part of it outside is never measured against the
  marker's edge.  `BJT.3.h1` (f) draws one, clean in both tools.  That is what "unrelated"
  asks for.
- **Marker granularity**: BJT.1 and BJT.2 are one marker per offending region in gdscheck and
  the whole marker polygon in KLayout; BJT.3's corner pair is one marker in gdscheck and two
  edge pairs in KLayout.

## Tested and clean

- **BJT.1's recognition** (`BJT.1.h1`, five devices, 1 / 1 in both tools): a complete NPN
  whose deep well runs 1 µm past the marker fires; the same layout without the LVS_BJT that
  makes the emitter an emitter, without the base, and without the collector are each not a
  transistor and each clean; a complete NPN with its deep well inside the marker is clean.
- **BJT.1's bound, which is zero** (`BJT.1.h2`, 1 / 1): the deep well drawn flush with the
  marker on all four sides is an overlap of exactly 0 and clean; 0.005 µm past it fires; the
  well wholly inside is clean.
- **BJT.2** at the bound: a tap inside is clean, a tap whose wall lies on the marker's is an
  overlap of 0 and clean, a tap the marker's edge cuts fires (`BJT.2.h1` (a), (b), (e)).
- **BJT.3** at 0.1 (clean) and 0.095; corner to corner at 0.1018 (clean) and 0.0990 (fires) -
  the euclidian chord across a corner is read; an overlapping active exempt (`BJT.3.h1`).
- **Tile invariance**: six layouts x tiles 20, 7 and 100, no rule's count moved; the three
  0.095 gaps of `BJT.3.h2` straddle x = 20, x = 42 and y = 20 and give 3 at every tile in both
  tools.
