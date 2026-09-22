<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / nat: hardening report

Deck `nat` against section 10.5 of the GF180MCU design manual, "Native Vt NMOS
(Optional)".  The section lists twelve rules, NAT.1 to NAT.12, and every one of them can
be measured on geometry - none is a guideline and none appears in Appendix B.  `show-deck`
lists fourteen entries under twelve ids (NAT.7 and NAT.9 each take two checks), so
**there is no coverage finding here**: the deck claims the whole section.

A native transistor is an N+ active clear of every well, a poly gate across it, and a NAT
marker over the pair; Dualgate over the marker makes it the 6 V device, which is the only
thing that tells NAT.4 from NAT.5.  Eight of the twelve rules read the marker against
something outside it and four read what lies under it, so almost every fixture is a row of
whole devices 30 µm apart, each bent one way, with the probe shape parked outside the
marker - a bar of anything left under a marker answers to NAT.11 or NAT.12 rather than to
the rule it was drawn for.

16 layouts under `tests/data/gf180mcuD/generated/nat/NAT.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/nat.rs`, each with a `#[case]` in `hardening_nat`.  Every layout ran at
tiles 20, 7 and 100 and through the upstream runset (`hardening/oracle-gf180.sh`, with
`DECKS=nat`).  **No count moved with the tile size**, on any layout, including `NAT.7.h1`,
whose five marker gaps are centred on the lines at x = 20, 21, 40, 42 and y = 20.

One note on reading the oracle here: NAT.6's marker line carries no µm figure ("nat6_comp
regions on 2 different nets lie under this nat"), so the oracle's filter drops it and
prints gdscheck's NAT.6 column as 0 on every layout.  The counts below are from
`gdscheck run --deck nat -v`, not from that column.

## Findings

### 1. An active that runs out of its marker is not an enclosure violation (false negative, both tools)

NAT.1: *Min. NAT Overlap of COMP of Native Vt NMOS* - 2 µm.

`NAT.1.h1` is three devices 30 µm apart.  The first has the marker 2.0 µm past the active
on every side; the second 1.995 on the right wall alone; the third is the good device with
a 1 × 3.2 µm N+ tab off the right of its active, running out through the marker's right
edge at x = 80.2 and ending 1 µm beyond it.

| device | geometry | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| 1 | marker 2.0 all round | - | - |
| 2 | marker 1.995 on the right wall | NAT.1, at (48, 10)-(48, 12) | NAT.1, edge pair (48,12;48,10)/(49.995,12;49.995,9.859) |
| 3 | active crosses the marker's edge | **nothing** | **nothing** |

The marker holds the crossing tab by nothing at all, which is 2 µm short of what NAT.1
asks, and the settled reading of an enclosure says so directly: it needs a `forbidden`
crossing half for a shape that runs out of the enclosing layer.  Upstream reaches the
same blind spot from the other side - `ncomp.outside(nwell).interacting(nat).enclosed(nat,
2.um)` has nothing to measure where the enclosing layer stops - so the two tools agree and
both are wrong.  The case expects NAT.1 twice.

The device is not an academic one: an N+ strap running out of the native marker to a
neighbouring device is exactly what the rule is there to forbid, and drawing it that way
is how a designer would get an unimplanted channel with an implanted tail.

### 2. The marker's 2 µm is read by projection, not euclidian (false negative)

NAT.1, again.

`NAT.1.h2` is one device whose marker's top-right corner is chamfered along x + y = 32.82,
so the active's own top-right corner at (18, 12) stands 1.994 µm from the 45° wall while
both axis distances from that corner to the marker are still 2.2 µm.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | NAT.1 × 2, the two edge pairs of the chamfer |

The settled reading of 2026-09-21 is that an enclosure is the closest approach from the
enclosed shape's boundary to the enclosing one, `metric: euclidian`, and that a 45° wall
passing under the value from a corner fires.  Upstream asks for `euclidian` in so many
words (`enclosed(nat, 2.um, euclidian)`) and reports it.  The deck's `NAT.1` entry carries
`interacting_only: 1` and no metric, and stays silent.  I believe the manual and KLayout:
the case expects NAT.1.

### 3. A shared edge is not reported as a space (false negative, four rules)

NAT.2 (*Space to unrelated COMP (outside NAT)* - 0.3 µm), NAT.3 (*Space to NWell edge* -
0.5 µm) and NAT.9's second half (*min spacing of un-related poly from the NAT layer* -
0.3 µm) are `min_space` from the marker to a neighbour.  In each fixture the last probe is
drawn flush against the marker's right edge, sharing it.

| fixture | probe | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| `NAT.2.h1` | COMP 0.3 off the marker | - | - |
| | COMP 0.295 off it | NAT.2 at (50.2, 10)-(50.495, 10) | NAT.2 |
| | COMP sharing the marker's edge at x = 80.2 | **nothing** | NAT.2, edge pair (80.2,12.3;80.2,9.7)/(80.2,10;80.2,12) |
| | COMP 0.22 × 0.2 off the marker's corner, 0.2966 euclidian | NAT.2 at (110.2, 14.2)-(110.42, 14.4) | NAT.2 × 2 |
| `NAT.3.h1` | NWELL 0.5 / 0.495 / sharing the edge | NAT.3 once, for the 0.495 | NAT.3 twice |
| `NAT.9.h1` | poly 0.3 / 0.295 / sharing the edge | NAT.9 once | NAT.9 twice |
| `NAT.10.h1` | a well abutting the marker's edge (third device) | **nothing** | NAT.3 |

A shared edge is a space of nothing, and nothing is less than 0.3 or 0.5 - the settled
reading (`abutting: report`) says the pair fires.  None of the three entries in the deck
carries the parameter.  The euclidian corner in `NAT.2.h1` is read right by both tools, so
this is the abutting class alone, not a metric question.  The cases expect NAT.2 three
times, NAT.3 twice, NAT.9 twice, and NAT.3 alongside NAT.10 twice on `NAT.10.h1`.

### 4. Two actives strapped to one node are still two potentials (false positive, both tools)

NAT.6: *Two or more COMPs if connected to different potential are not allowed under same
NAT layer.*

`NAT.6.h1` and `NAT.6.h2` are the same geometry: one marker holding two N+ actives, each
with its own poly gate, each with a 0.22 µm contact in it.  In `h1` each contact carries
its own Metal1 plate; in `h2` a single Metal1 plate covers both, which shorts the two
actives into one node.  Nothing else in either layout fires - the whole `core` suite on
`h2` reports only NAT.6 and the two DF.14_LV taps - so the contacts and the strap are
well-formed.

| fixture | the two actives | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| `NAT.6.h1` | two nodes | NAT.6 ("regions on 2 different nets") | NAT.6 × 2 (the two COMP polygons) |
| `NAT.6.h2` | one node, strapped | NAT.6, same message | NAT.6 × 2 |

The rule forbids two COMPs *at different potential*; two COMPs at the same potential under
one marker are a two-finger native device with its sources tied, which is the ordinary way
to draw one.  `h2` is legal and both tools report it.

For gdscheck the reason is visible in the deck without opening the engine: `NAT.6` is
`max_nets_under [nat, nat6_comp]` with `net_of: comp`, and `comp` is not one of the
conductors the PDK's `connectivity:` list names - the graph carries `ncomp_con`,
`pcomp_con` and `natcomp_con`, never the drawn `comp`.  A layer outside the graph resolves
to no net, so each region counts as its own and the check degenerates into "two COMPs
under one marker".  `natcomp_con` is the layer upstream reads for the same rule and is in
the graph already.  The case expects nothing on `h2`.

KLayout is wrong the same way on this layout and I could not get it to see the strap
either, so the oracle offers no second opinion here; the manual's wording is the whole
argument.

## Read and left alone

- **Marker granularity.**  `min_gate_length` gives gdscheck one marker per wall and
  KLayout one edge pair: NAT.4 and NAT.5 are 2 against 1 on `NAT.4.h1` and `NAT.4.h2`.
  `min_space` at a corner is 1 against 2 on `NAT.2.h1`.  Settled; the violating structures
  agree.
- **NAT.8 on a marker Dualgate merely abuts.**  `NAT.8.h1`'s third device has Dualgate
  flush against the marker's right edge and over none of it.  Both tools stay silent, and
  that is right: NAT.8's zero micron is the overlap a *5 V/6 V* native device owes its
  thick-oxide marker, and a marker with no Dualgate over it is a 3.3 V device.  The deck's
  `nat_over_dualgate` is `overlapping`, which is the settled LV/MV split; upstream's
  `not_outside(dualgate)` would be the stricter `interacting` reading, but KLayout's
  `outside` treats the abutting marker as outside and it reports nothing either.  The same
  device in `NAT.4.h2` takes NAT.4, the 3.3 V channel rule, in both tools.
- **A point touch is an intersection.**  `NAT.11.h1`'s second device has its gate meeting
  the active's top edge but not crossing it, and `NAT.12.h1`'s second probe touches the
  active at the single point (18, 12).  Both tools read `interacting` as satisfied by the
  touch, so NAT.11 and NAT.12 stay quiet.  Kept: the manual's "not intersecting to Poly2"
  is about a device with no gate at all, and a touch is the boundary case both decks were
  written to the same way.
- **NAT.6 on `NAT.11.h1`.**  Its third device puts a P+ active and the N+ one under one
  marker with nothing tying them, which is two potentials and fires NAT.6 in both tools.
  That is the rule working as written; it rides along in the case.

## Tested and clean

Drawn at the value and one step past it, and answered exactly as the manual reads at all
three tile sizes and in agreement with the runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| NAT.1 | marker 2.0 / 1.995 past the active, on one wall | `NAT.1.h1` |
| NAT.2 | unrelated COMP 0.3 / 0.295 off the marker, and a 0.2966 euclidian corner | `NAT.2.h1` |
| NAT.3 | N-well 0.5 / 0.495 off the marker | `NAT.3.h1` |
| NAT.4 | 3.3 V channel 1.8 / 1.795, and 1.795 under a Dualgate that only abuts | `NAT.4.h1`, `NAT.4.h2` |
| NAT.5 | 6 V channel 1.8 / 1.795, the marker covered whole by Dualgate | `NAT.4.h1` |
| NAT.6 | two unconnected actives under one marker | `NAT.6.h1` |
| NAT.7 | marker gaps 0.74 / 0.735 and a 0.735 slot, on the lines at x = 20, 21, 40, 42 and y = 20 | `NAT.7.h1` |
| NAT.8 | marker covered whole / covered from its middle / abutted | `NAT.8.h1` |
| NAT.9 | unrelated poly 0.3 / 0.295 off the marker; two gates of one marker bridged by field poly | `NAT.9.h1`, `NAT.9.h2` |
| NAT.10 | a well with one corner over the marker, and one wholly inside it | `NAT.10.h1` |
| NAT.11 | an N+ active under the marker with no gate; one whose gate abuts; a P+ active | `NAT.11.h1` |
| NAT.12 | poly under the marker reaching no active; poly over an active under RES_MK | `NAT.12.h1` |

The deck's own good/bad pairs were left as they were; nothing this round drew moved them.
