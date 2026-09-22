<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / ldnmos: hardening report

Deck `ldnmos` against section 10.12.1 of the GF180MCU design manual, "10V LDNMOS rules".
The section lists 35 rules with a value or a condition a layout can be measured against
(MDN.16 is a pointer to chapter 7, and MDN.10 and MDN.13 are headings).  `show-deck`
lists 45 entries under 33 ids: everything but MDN.5c and MDN.10d, which the deck's header
names as not ported - the earlier session proved KLayout's own computation of those two
disagrees with its vendored report.  **No other rule of the section is missing**, so there
is no coverage finding here.

What this round drew is the deck's own arithmetic rather than the engine's classes.  An
LDNMOS is not a shape: it is a source, a gate, a drift under the gate's far end, a drain
island inside the drift and a P+ guard ring round the lot, and almost every rule of the
section measures one of those against another.  So every fixture is a row of whole
devices 60 µm apart, each bent one way - the value the manual names on one device, the
0.005 µm step past it on the next, and, where the manual *fixes* a dimension instead of
bounding it (MDN.10c's 0.2, MDN.11's 0.4), the step short of it as well.

15 layouts under `tests/data/gf180mcuD/generated/ldnmos/MDN.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/ldnmos.rs`, each with a `#[case]` in `hardening_ldnmos`.  Every layout ran
at tiles 20, 7 and 100 and through the upstream runset (`hardening/oracle-gf180.sh`).
**No count moved with the tile size**, on any layout, including the five whose spacing
gaps are centred on the tile lines at x = 10, 20, 21, 40 and 42.

## Findings

### 1. An active that runs out of the drift is reported only when the drift has no other drain (false negative)

MDN.12: *Min MVSD enclose NCOMP in the LDNMOS drain and in the direction along the
transistor width* - 0.5 µm.

`MDN.12.h1` is four devices.  The first has a 6 µm drain in a 7 µm drift (0.5 either side,
the bound); the second a 6.01 µm one (0.495); the third a drain with a tab that runs out
through the drift's top edge; the fourth the same tab plus a second bare island, 1 × 3 µm,
left well inside the drift.

| device | geometry | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| 1 | drain 6.0, enclosure 0.5 | - | - |
| 2 | drain 6.01, enclosure 0.495 | MDN.12 × 2 (one per wall, at (87, 13.7475) and (87, 20.2525)) | MDN.12 × 2 |
| 3 | drain runs out, nothing else in the drift | MDN.12, MDN.11 (both at (145.5, 17)) | MDN.12, MDN.11 (the drift polygon) |
| 4 | drain runs out, a second island inside | **nothing** | MDN.12, MDN.11 (the drift polygon, (202,13.5)-(209,20.5)) |

An N+ active that crosses the drift's boundary is held by it by nothing, which is less
than the half micron MDN.12 asks, so the fourth device is a violation of MDN.12 and the
settled reading of an enclosure - it needs a half for a shape that runs out of the
enclosing layer - says so directly.  The deck reaches the third device only through its
other half, `mdn12_no_drain` (a drift covering no bare active at all); once a drain is put
back in the drift that half goes quiet and `mdn12_short`, an `enclosure_below`, never sees
the shape that crosses out.  I believe the manual: the case should fire.

KLayout arrives at the right answer by the wrong route - it reports the whole drift under
`mdn12_b = mvsd_mdn.not_covering(...)` for the third *and* the fourth device, so its
`not_covering` behaves as though it wanted the drift to cover every bare active it meets,
not just one.  The case's expected value is MDN.12 four times and MDN.11 once.

### 2. MDN.2a, the equal-potential space, is reported on a different-potential gap (false positive)

MDN.2a: *Min MVSD space [Same Potential]* - 1 µm.  MDN.2b: *[Diff Potential]* - 2 µm.

`MDN.2b.h1` is five pairs of drift bars, each pair's gap centred on a tile line, with
nothing else in the layout (no marker, so the rest of the section has nothing to read):

| pair | gap | tied to one node | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- | --- |
| x = 10 | 1.0 | yes | - | - |
| x = 20 | 0.995 | yes | MDN.2a | MDN.2a |
| x = 21 | 2.0 | no | - | - |
| x = 40 | 1.995 | no | MDN.2b | MDN.2b |
| x = 42 | 0.995 | no | **MDN.2a** and MDN.2b | MDN.2b |

The last pair stands at two potentials, so the rule that governs its gap is MDN.2b and
only MDN.2b; MDN.2a is the number for two drifts at the *same* potential and has nothing
to say about it.  The deck lists MDN.2a as `min_space [mvsd] 1` with no `net` parameter
while MDN.2b carries `net: different`, so every gap under 1 µm is MDN.2a whatever its
nodes.  Upstream splits one measurement between the two ids (`conn_space(mvsd, 1, 2)`) and
reports each gap once, which is what the manual reads like.  The same class is finding 2
of the `ldpmos` report, on MDP.10b.

### 3. A body tap that meets the source at a corner is read as butted (both tools)

MDN.5ai is 1 µm *(source and body tap non-butted)*, MDN.5aii 0.92 µm *(butted)* - one gap
between the P+ tap and the drain drift under two numbers, with the arrangement of the tap
and the source picking between them.

`MDN.5aii.h2` is one device with a P+ tab 0.95 µm above the drift whose top-right corner
is exactly the bottom-left corner of an N+ island - they meet at one point and share no
edge.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | 0 |

Both tools decide the split with `interacting`, which a point touch satisfies, so the tap
is butted and 0.95 clears 0.92.  A source and a body tap that touch at a corner are not
butted in any sense the manual's figures show - butting is a shared edge, which is what
lets the two numbers differ by 0.08 µm at all - so the tap is MDN.5ai's and 0.95 breaks
it.  The case expects MDN.5ai.  This is the least certain of the four; it is recorded
because nothing in either deck distinguishes a corner from an edge, and the neighbouring
`MDN.5ai.h1` shows that everything else about the split is read exactly right (a tap at
0.995 unbutted fires MDN.5ai, at 0.915 butted fires MDN.5aii, and a butted one at 0.995 -
over one number, under the other - is clean in both tools).

### 4. MDN.17 fires on a device that is not an LDNMOS (both tools)

MDN.17: *It is recommended to surround the LDNMOS transistor with non-broken Psub guard
ring.*

Every `ldpmos` hardening layout carries LDNMOS markers: running the `ldnmos` deck on
`ldpmos/MDP.12.h1` (an LDPMOS: deep well, N+ guard ring, P+ source and drain, MVPSD, no
MVSD anywhere in the layout) gives

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | MDN.17 × 4, at (10.47, 21.5), (18.8, 17), (70.47, 21.5), (78.8, 17) |
| KLayout runset | MDN.17 × 4 |

- and the same on all fourteen, 4 to 20 markers each, the two tools agreeing every time.
The reason is the same in both decks: the guard ring the rule wants is
`pcomp.holes ∧ (ldmos_xtor interacting mvsd)`, and with no MVSD in the layout that layer
is empty, so *every* N+ active and every poly under the marker is "not inside" it.  A
layout with no LDNMOS in it cannot break a rule about LDNMOS transistors; the device
material the rule reads (`ncomp_ld ∪ poly_ld ∪ mvsd_mdn`) should be gated on the marker
holding a drift, the way `mdn17_viol_b` already is.  Not turned into a case of its own -
it belongs to whichever deck's fixtures are run - but it is why the `ldpmos` fixtures
would be noisy if they were ever run through both decks at once.

## Read and left alone

- **Marker granularity.**  `min_width` gives one marker per wall in gdscheck and one edge
  pair in KLayout: MDN.10a 2 against 1 on `MDN.3a.h1`, MDN.15a 2 against 1 on
  `MDN.15a.h1`, MDN.11 2 against 1 for the 0.395 overlap.  `max_length` goes the other
  way: MDN.13a is 1 in gdscheck against KLayout's 2, one per long wall of the finger.
  Settled; judged by violating structures, which agree everywhere.
- **MDN.10d, klayout-only 3 markers on `MDN.10c.h1`.**  Declared not ported.
- **MDN.10c, klayout-only 2 markers on `MDN.6a.h1`.**  KLayout's error region for MDN.10c
  is the drain's own box, `ncomp_mdn.sized(0.36).sized(-0.36).extents`.  The probe island
  of that fixture sits 0.7 µm above the device's active - inside the 0.72 µm that closing
  radius spans - so the box closes over the two and grows up past the gate's end cap, and
  the 0.5 µm end cap is then read as an overhang that is not 0.2.  The manual's MDN.10c is
  the overhang *towards the LDNMOS drain COMP*; the end cap is MDN.10b's 0.4 and the
  fixture keeps it there.  gdscheck is right to stay silent.
- **MDN.11 on a drift with no drain left in it** (`MDN.12.h1`, third device).  Both tools
  add MDN.11 to MDN.12 there.  MDN.11's text is only about the 0.4 overlap, but the
  reading that a drift is a drain region and must hold a drain is upstream's and the
  deck's alike, and the case keeps it.

## Tested and clean

Drawn at the value and one step past it, and found to answer exactly as the manual reads,
at all three tile sizes and in agreement with the runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| MDN.2a / MDN.2b | 1.0, 0.995 tied; 2.0, 1.995 untied, gaps on tile lines | `MDN.2b.h1` |
| MDN.3a | channel 0.6 / 0.595 (with MDN.10a's 1.195 gate) | `MDN.3a.h1` |
| MDN.3b | channel 20.0 / 20.005 | `MDN.3a.h1` |
| MDN.4a | width 4.0 / 3.995 | `MDN.4a.h1` |
| MDN.4b, MDN.13a | width 50.0 / 50.005 | `MDN.4a.h1` |
| MDN.5ai | tap touching nothing, 1.0 / 0.995 off the drift | `MDN.5ai.h1` |
| MDN.5aii | tap butted to an N+, 0.92 / 0.915, and a butted one at 0.995 | `MDN.5ai.h1` |
| MDN.6a | active 0.5 / 0.495 inside Dualgate | `MDN.6a.h1` |
| MDN.8a / MDN.8b | 1.0, 0.995 tied; 2.0, 1.995 untied; a drift lying on a well, which is one node and MDN.8a alone (KLayout adds MDN.8b there - it joins the overlap to both ids unconditionally - and the manual is on gdscheck's side) | `MDN.8a.h1` |
| MDN.9 | stray active 4.0 / 3.995 from the drift | `MDN.9.h1` |
| MDN.10a | gate 1.2 / 1.195 wide | `MDN.3a.h1` |
| MDN.10b | end cap 0.4 / 0.395, both ends | `MDN.10b.h1` |
| MDN.10c | overhang 0.2 / 0.195 / 0.205 - fixed, so short and long both fire | `MDN.10c.h1` |
| MDN.10ei | tap touching nothing, 0.4 / 0.395 off the gate | `MDN.10ei.h1` |
| MDN.10eii | tap butted to an N+, 0.32 / 0.315 off the gate | `MDN.10ei.h1` |
| MDN.11 | overlap 0.4 / 0.395 / 0.405 - fixed, so both directions fire | `MDN.11.h1` |
| MDN.12 | drift holding the drain 0.5 / 0.495 | `MDN.12.h1` |
| MDN.14 | drift 6.0 / 5.995 from a deep well, and lying on one | `MDN.14.h1` |
| MDN.15a | drain active 0.22 / 0.215 | `MDN.15a.h1` |
| MDN.15b | contact flush with the drain's edge (the 0 µm the rule asks) / 0.005 past it | `MDN.15a.h1` |

MDN.6, MDN.7, MDN.7a, MDN.10f, MDN.13a-d and MDN.17 keep the good/bad pairs the deck
already had; nothing this round drew moved them.

## Resolution (2026-09-22)

Findings 1 and 2 were fixed in the PDK; findings 3 and 4 were read again and kept, with
the reason written where it can be seen.

- **Finding 1 (MDN.12).** `mdn12_viol` gains a crossing half: `mdn12_crossing`, the part
  of a bare N+ that overlaps the drift and lies outside it. An enclosure reads only where
  the enclosing layer holds the shape, so `mdn12_short` had nothing to measure on a shape
  that runs out through the edge, and the other half - a drift covering no bare active at
  all - went quiet as soon as a second drain sat inside. `MDN.12.h1`'s fourth device
  fires now, and the case expects MDN.12 four times and MDN.11 once. On the foundry
  layout the union drops one marker (19 -> 18): a crossing piece that joins two regions
  the deck already reported, which is granularity, not a lost violation.
- **Finding 2 (MDN.2a).** The entry carries `params: net: same`. A gap between two drifts
  on different nodes is MDN.2b's and is measured at its 2 µm, and the layout's fifth pair
  reports MDN.2b alone. The foundry layout goes 24 -> 23.
- **Finding 3 (the corner touch).** Kept as both tools read it: `interacting` is satisfied
  by a point, so the pair takes MDN.5aii's 0.92 and 0.95 passes. `MDN.5aii.h2` expects
  nothing, with the reading written above the case. Telling a corner from a shared edge
  would need a separate layer op on both sides of the split, and the gap it changes is
  0.08 µm.
- **Finding 4 (MDN.17 on an LDPMOS).** Left as it is: the fixtures of each deck run under
  that deck, so nothing reads an LDPMOS layout through `ldnmos`. The note stays because
  it explains why a combined run would be noisy.

The patterns of this deck drew three actives straddling their drift's edge, so the drift
would *meet* them without enclosing them; with MDN.12 now reading that edge they are drawn
inside the drift, and `device_bar`'s plain drift takes two islands rather than one, so it
still meets four actives and is neither MDN.13d's drain nor its multi-finger device.
`MDN.5aii`'s butted N+ moved to the far side of a wide tap, 4 µm clear of every drift,
where MDN.9 has nothing to say about it either.
