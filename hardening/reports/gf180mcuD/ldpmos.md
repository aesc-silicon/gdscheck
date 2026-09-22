<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / ldpmos: hardening report

Deck `ldpmos` against section 10.12.2 of the GF180MCU design manual, "10V LDPMOS rules".
The section lists 38 rules a layout can be measured against (MDP.14 is a pointer to
chapter 7, MDP.9 and MDP.13 are headings, and MDP.17's four numbered notes are the two
checks upstream writes as MDP.17a and MDP.17c).  `show-deck` lists 44 entries under 35
ids: everything but MDP.3, MDP.3c and MDP.9c, which the deck's header names as not
ported.  **No other rule of the section is missing**, so there is no coverage finding.

The P-side device is the N-side one built the other way up - a deep N-well, an N+ guard
ring inside it, P+ source and drain, two markers over both, and a drift (MVPSD) that has
to hold its drain and overlap its channel by a fixed amount - and it is the unit every
fixture is drawn in, because nearly every rule of the section measures one part of it
against another.  Each layout is a row of whole devices 60 µm apart, one per reading: the
value the manual names, the 0.005 µm step past it, and, where the manual fixes a dimension
rather than bounding it (MDP.10's 0.4), the step short of it as well.

15 layouts under `tests/data/gf180mcuD/generated/ldpmos/MDP.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/ldpmos.rs`, each with a `#[case]` in `hardening_ldpmos`.  Every layout ran
at tiles 20, 7 and 100 and through the upstream runset.  **No count moved with the tile
size**, on any layout.

## Findings

### 1. MDP.10 reports an overlap over the fixed 0.4 and not one under it (false negative)

MDP.10: *Min/Max MVPSD overlap onto the channel (LDMOS_XTOR AND COMP AND POLY2 AND Pplus)*
- 0.4 µm.  Min *and* max: the manual fixes the overlap, so a channel the drift reaches
0.395 into is as much a violation as one it reaches 0.405 into.

`MDP.10.h1` is three devices whose active ends 0.4, 0.395 and 0.405 µm inside the drift;
nothing else about them differs, because the gate follows the active's end and the drift's
own edges stay where they were.

| device | overlap | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| 1 | 0.400 | - | - |
| 2 | 0.395 | **nothing** | MDP.10 |
| 3 | 0.405 | MDP.10 (`mdp10_wide` at (142.2025, 14)) | MDP.10 |

The deck reads MDP.10 as two `forbidden` layers: `mdp10_wide`, the overlap opened by 0.2,
which survives only where the region is thicker than 0.4 and so catches the long case, and
`mdp10_no_channel`, a drift with no channel under it at all.  Nothing measures the short
case.  The N side's MDN.11 is the same rule on the same arithmetic and does carry it -
`min_width [mdn11_uniform] 0.4` - and `ldnmos/MDN.11.h1` shows it working: 0.395 there
fires twice, once per wall of the narrow strip.  Upstream tests the overlap for equality
(`mvpsd.drc(overlap(...) == 0.4)`) and reports both.  I believe the manual and upstream;
the case expects MDP.10 twice.

### 2. MDP.10b, the equal-potential space, is reported on a different-potential gap (false positive)

MDP.10a: *Min MVPSD space within LDMOS_XTOR marking [diff potential]* - 2 µm.  MDP.10b:
*Min MVPSD space [same potential]* - 1 µm.

`MDP.10b.h1` is five devices, each with a mirrored pair of real drifts in its guard ring's
hole (a drift with no channel under it would answer to MDP.10 instead).  The
same-potential pairs share one drain active, which is what puts them on one node.

| device | gap | one node | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- | --- |
| 1 | 1.0 | yes | - | - |
| 2 | 0.995 | yes | MDP.10b | MDP.10b |
| 3 | 2.0 | no | - | - |
| 4 | 1.995 | no | MDP.10a | MDP.10a |
| 5 | 0.995 | no | **MDP.10b** and MDP.10a (both at (258.0, 23.0)-(258.995, 23.0)) | MDP.10a |

The fifth pair is at two potentials, so MDP.10a is the rule that governs its gap;
MDP.10b's 1 µm is the number for two drifts at one potential.  The deck lists MDP.10b as
`min_space [mvpsd] 1` with no `net` parameter while MDP.10a carries `net: different`, so
every gap under 1 µm is MDP.10b whatever its nodes.  Upstream splits one measurement
between the two ids (`conn_space(mvpsd, 1, 2)`).  The same class is finding 2 of the
`ldnmos` report, on MDN.2a.

## Read and left alone

- **MDP.3b on a butted pair: KLayout reports it, gdscheck does not, and the manual is on
  gdscheck's side.**  Every butted N+/P+ probe drawn here - two on `MDP.3ai.h1`, two on
  `MDP.9ei.h1` - is a klayout-only MDP.3b marker.  MDP.3b reads *Min NCOMP space to PCOMP
  in DNWELL ... **Use butted source and DNWELL contacts otherwise** and that is best for
  Latch-up immunity as well: 0.4*, so butting is the sanctioned alternative to the 0.4 and
  a shared edge is not a violation of it; upstream's `separation` reports the coincident
  edges as a zero gap regardless.  Worth confirming that the silence is by design rather
  than by accident, since it is the one place in this section where a zero gap is meant to
  pass and the deck expresses it by *omitting* the `forbidden [*_touch]` companion that
  MDP.3ai, MDP.3aii, MDP.9ei and MDP.9eii all carry.
- **Marker granularity.**  `min_width` is one per wall in gdscheck against one edge pair
  in KLayout (MDP.9a 2:1, MDP.16a 2:1); `max_length` is one per polygon against one per
  long wall (MDP.13a 1:2); `min_enclosure` is one per enclosed shape against one per edge
  pair (MDP.12 1:4 - the deep well is 0.655 short on all four sides of one ring; MDP.9b
  2:4).  Settled; the violating structures agree everywhere.
- **MDP.3 and MDP.3c, klayout-only.**  Declared not ported.
- **MDN.17 fires on every one of these layouts**, in both tools and at the same count.
  That is the `ldnmos` deck reading an LDPMOS device, and it is finding 4 of the `ldnmos`
  report; the cases here run the `ldpmos` deck alone, so it does not reach them.

## Tested and clean

Drawn at the value and one step past it, and found to answer exactly as the manual reads,
at all three tile sizes and in agreement with the runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| MDP.1 | channel 0.6 / 0.595 (with MDP.9a's 1.195 gate) | `MDP.1.h1` |
| MDP.1a | channel 20.0 / 20.005 | `MDP.1.h1` |
| MDP.2 | width 4.0 / 3.995 | `MDP.2.h1` |
| MDP.3ai | guard-ring tap touching nothing, 1.0 / 0.995 off the drift | `MDP.3ai.h1` |
| MDP.3aii | the same butted to a P+, 0.92 / 0.915 | `MDP.3ai.h1` |
| MDP.3b | an N+ and a P+ in the deep well, 0.4 / 0.395 apart | `MDP.3b.h1` |
| MDP.4a | P+ active outside the deep well, 2.5 / 2.495 from it | `MDP.4a.h1` |
| MDP.5a | Dualgate holding a P+ tab 0.5 / 0.495 | `MDP.5a.h1` |
| MDP.7 | N-well 2.0 / 1.995 outside the LDMOS marker | `MDP.7.h1` |
| MDP.8 | N+ active 1.5 / 1.495 outside the marker | `MDP.7.h1` |
| MDP.9a | gate 1.2 / 1.195 wide | `MDP.1.h1` |
| MDP.9b | end cap 0.4 / 0.395, both ends | `MDP.9b.h1` |
| MDP.9ei | tap touching nothing, 0.4 / 0.395 off the gate | `MDP.9ei.h1` |
| MDP.9eii | tap butted to a P+, 0.32 / 0.315 off the gate | `MDP.9ei.h1` |
| MDP.10 | overlap 0.405 (the long half of the fixed 0.4) | `MDP.10.h1` |
| MDP.10a / MDP.10b | 1.0, 0.995 sharing a drain; 2.0, 1.995 mirrored and untied | `MDP.10b.h1` |
| MDP.11 | drift holding the drain 0.8 / 0.795, in the drain direction and across the width | `MDP.11.h1` |
| MDP.12 | deep well holding the guard ring 0.66 / 0.655 | `MDP.12.h1` |
| MDP.13a | width 50.0 / 50.005 | `MDP.2.h1` |
| MDP.15 | a second deep well 6.0 / 5.995 from the one carrying the drift | `MDP.15.h1` |
| MDP.16a | drain active 0.22 / 0.215 | `MDP.16a.h1` |
| MDP.16b | contact flush with the drain's edge (the 0 µm the rule asks) / 0.005 past it | `MDP.16a.h1` |

MDP.3d, MDP.4, MDP.4b, MDP.5, MDP.6, MDP.6a, MDP.9d, MDP.9f, MDP.13b, MDP.13c, MDP.17a and
MDP.17c keep the good/bad pairs the deck already had; nothing this round drew moved them.

## Resolution (2026-09-22)

Both findings were fixed in the PDK.

- **Finding 1 (MDP.10).** The rule fixes the drift's overlap of the channel at 0.4, and
  the deck read only the parts over it (`mdp10_wide`). The parts under it are a width now:
  `mdp10_uniform` - the overlap regions that are not the wide ones - carries a
  `min_width 0.4` entry, mirroring MDN.11, which the N side already read that way.
  `MDP.10.h1` reports three markers instead of two.
  The foundry layout goes 16 -> 30. The upstream rule exempts a whole drift that overlaps
  its channel by 0.4 *anywhere* (`mvpsd_mv.not_interacting(mdp10_exclude)`), so the laps of
  0.01 and 0.03 µm elsewhere on such a drift are reported by gdscheck alone; each of them
  is an overlap that is not the fixed 0.4, which is what the rule asks.
- **Finding 2 (MDP.10b).** The entry carries `params: net: same`, as MDN.2a does on the N
  side. A gap between two drifts on different nodes is MDP.10a's 2 µm, and it was reported
  under both ids. The foundry layout goes 4 -> 2, which is the runset's count there.
