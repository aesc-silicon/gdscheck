<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / ymtp_mk: hardening report

Deck `ymtp_mk` against section 10.13 of the GF180MCU design manual, "YMTP_MK Mark Layer
Rules".

## Coverage

The section's table has eleven rows.  Two are exemptions and not checks - Y.DF.4d
(*"(Nwell overlap of NCOMP) outside DNWELL (inside YMTP_MK) **is allowed**"*) and Y.PL.6
(*"90 deg bends on the COMP are not allowed **(except YMTP_MK)**"*) - and both are
correctly absent from the deck and from upstream.  Of the remaining nine, seven carry two
columns and two carry one (Y.DF.6 and Y.PL.4 are 5 V only, Y.PL.1's width is 3.3 V only,
and Y.LU.3 is 3.3 V only).  `show-deck` lists sixteen entries under ten ids: Y.NW.2b_LV,
Y.NW.2b_MV, Y.DF.6_MV, Y.DF.16_LV, Y.DF.16_MV, Y.PL.1_LV, Y.PL.1_MV, Y.PL.2_LV,
Y.PL.2_MV, Y.PL.4_MV, Y.PL.5a_LV/MV and Y.PL.5b_LV/MV.

**Y.LU.3 is missing** - finding 1 below.

## What was drawn

Every fixture is one or more probes, each under its own marker with a micron to spare, and
the same geometry is drawn twice wherever the two voltage classes have different numbers -
once under a marker Dualgate covers and once under one it does not.  Every MV probe
carrying poly is shielded with PLFUSE, because Y.PL.1_MV forbids poly in a 5 V marker
outright and PLFUSE is the one thing it excludes.

8 layouts under `tests/data/gf180mcuD/generated/ymtp_mk/Y.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/ymtp_mk.rs`, each with a `#[case]` in `hardening_ymtp_mk`.  Every layout
ran at tiles 20, 7 and 100 and through the upstream runset (`DECKS=ymtp_mk`).  **No count
moved with the tile size**, on any layout, including `Y.NW.2b.h1`, whose three well gaps
and one notch straddle the lines at x = 21, x = 40 and y = 20.

## Findings

### 1. Y.LU.3 is in the manual and in no deck (coverage)

Y.LU.3, the last row of the section: *"This rule is to check: (a) Max. Psub tap space to
every point on the boundary of NCOMP outside NWELL. (b) For within 50 µm from the (NCOMP
in Psub), Minimum NWELL to (NCOMP outside Nwell) space is defined as y, (c) If y < 1.0 µm,
then Max. Psub tap space to every point on the boundary of NCOMP outside NWELL (inside
YMTP_MK)"* - 20 µm at 3.3 V, NA at 5 V.

It is a geometric rule: the same max-space measurement as chapter 14's LU.3 and as DF.14,
with the marker choosing the number.  `show-deck --deck ymtp_mk` does not list it, and it
is not in Appendix B - the only latchup rules that appendix excuses are LU.7 to LU.10,
*"Guideline and not codeable"*, and its chapter-13 entry reads *"RELIABILITY GUIDELINES
(**Except Latchup rules**)"*, which says the latchup rules are meant to be coded.
Upstream has no latchup deck at all: `rule_decks/` holds forty files and none of them
outputs an `LU.*` id, so the oracle has nothing to say about it either.

The practical exposure is small, and that is worth writing down so the reader can judge
the priority.  `comp`'s DF.14_LV - *"Max distance of substrate tap (PCOMP outside Nwell)
from (NCOMP outside Nwell)"*, 20 µm - is `max_space [nactive_3p3v, ptap] 20` with nothing
excluding YMTP_MK, so the 20 µm limit Y.LU.3 names is already enforced inside the marker
by another deck.  What Y.LU.3 adds is the *condition*: chapter 14's LU.3 tightens the
20 µm to 15 µm when the well is closer than 1 µm to the active, and this row says the
marker keeps it at 20.  With neither LU.3 nor Y.LU.3 coded, nothing in either tool
tightens it in the first place, so the marker's relaxation has nothing to relax.  No
fixture is drawn for it: any layout that would exercise it fires DF.14_LV from the `comp`
deck, which is the right answer for the wrong reason.

### 2. A shared edge is not reported as a space (false negative, two rules)

Y.DF.16 (*Min. space from (Nwell outside DNWELL) to (unrelated NCOMP outside Nwell and
DNWELL) (inside YMTP_MK)* - 0.27 µm at 3.3 V) and Y.PL.5a/5b (*Space from field Poly2 to
unrelated / related COMP (inside YMTP_MK)* - 0.04 µm at 3.3 V) each have a probe drawn
flush against the shape they are measured from.

| fixture | probe | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| `Y.DF.16.h1` | well 0.27 off the active | - | - |
| | well 0.265 off it | Y.DF.16_LV at (22, 5)-(22.265, 5) | Y.DF.16_LV, edge pair at x = 22 |
| | well flush against it, at x = 32 | **nothing** | Y.DF.16_LV, edge pair (32,7;32,5)/(32,5;32,7) |
| `Y.PL.5.h1` | poly 0.04 / 0.035 / flush off the active | Y.PL.5a_LV and Y.PL.5b_LV once each, for the 0.035 | both ids twice, the second pair at x = 32 |

A shared edge is a space of nothing, which is under both 0.27 and 0.04; the settled
reading (`abutting: report`) says the pair fires.  Neither entry carries the parameter.
The same class is finding 3 of the `nat` report and finding 3 of the `otp_mk` report; it
is one parameter on a good many entries across the PDK.

### 3. The marker is cut into the layers, so its own outline becomes a notch (false positive, both tools)

Y.NW.2b: *Min. Nwell Space (Outside DNWELL, Inside YMTP_MK) [Different potential]* - 1 µm.

Section 10.13's first line is *"This layer is used to mark YMTP cell only"*, and the
settled reading of a marker naming a cell is that it selects whole regions.  Both decks
build the layer as `nwell_n_dn ∧ ymtp_mk` instead.  `Y.NW.2b.h2`'s third probe is one
solid 4 × 3 µm N-well with no notch anywhere in it, under a marker shaped like a U whose
0.5 µm slot runs in from the right.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | Y.NW.2b_LV, notch 0.5 at (53, 5.75)-(53, 6.25) |
| KLayout runset | Y.NW.2b_LV, edge pair (52,5.75;54,5.75)/(54,6.25;52,6.25) |

The well is one plate at one potential and the rule is about the space between wells at
*different* potential; there is no second well and no gap.  The notch is the marker's.
The same class is finding 2 of the `otp_mk` report, where the marker cut also costs a COMP
its area.  The case expects only the one real violation in that layout.

## Read and left alone

- **A marker Dualgate merely abuts is the 3.3 V marker.**  `Y.NW.2b.h2`'s second probe is
  a 0.995 µm well gap under a marker with Dualgate flush against its left edge and over
  none of it.  gdscheck reports Y.NW.2b_LV; KLayout reports nothing at all, because its
  `ymtp_mk_lv` is `not_interacting(dualgate)` and its `ymtp_mk_mv` is
  `overlapping(dualgate)`, so a marker that touches without overlapping falls out of both
  classes and gets no rule.  gdscheck's complementary pair is the settled reading and the
  right one: the gap is measured under one number or the other, never under neither.
- **Y.PL.1_MV: "NA" read as "none allowed".**  The manual's 5 V column for Y.PL.1 is `NA`,
  and upstream turns that into an output layer of its own - every poly in a 5 V marker
  outside PLFUSE is the violation.  gdscheck follows (`Y.PL.1_MV forbidden
  poly_ymtp_mv_out_pf`), and `Y.PL.1.h1`'s fourth probe, a perfectly wide 0.5 µm line,
  fires in both tools.  Kept, but recorded: the same `NA` in Y.DF.6's 3.3 V column and
  Y.PL.4's is read by both decks as "no rule here", so the document's one word is being
  given two opposite meanings three rows apart.  Only the manual's author can settle it;
  the fixture pins today's reading so that a change to it is visible.
- **Y.PL.1_LV riding along with Y.PL.2_LV.**  A channel shorter than 0.13 µm is also a
  poly line narrower than 0.13 µm, and Y.PL.1 reads all poly in the marker, gates
  included, exactly as chapter 7's PL.1 does.  `Y.PL.2.h1`'s two 0.125 gates therefore
  carry Y.PL.1_LV as well (four markers, one per wall); OTP_MK exempts the third gate from
  Y.PL.2 but not from Y.PL.1, which is how both decks have it.
- **Y.PL.5a and Y.PL.5b are one measurement.**  The manual gives them the same number and
  distinguishes only *unrelated* from *related* COMP, which neither deck resolves; both
  emit both ids on every violation.  Judged by violating structures, the two tools agree.
- **Marker granularity.**  `min_width` and `min_gate_length` give gdscheck one marker per
  wall against KLayout's one edge pair: Y.PL.1_LV is 2 against 1 on `Y.PL.1.h1`, Y.PL.2_LV
  and Y.PL.2_MV 2 against 1 each, Y.PL.4_MV and Y.DF.6_MV 2 against 2 (both walls of those
  devices are short).  Settled.
- **The DNWELL exemption.**  `Y.NW.2b.h2`'s first probe is a 0.995 µm well gap inside a
  DNWELL; both tools stay silent, which is the rule's own "(Outside DNWELL)".

## Tested and clean

Drawn at the value and one step past it, and answered exactly as the manual reads at all
three tile sizes and in agreement with the runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| Y.NW.2b_LV | wells 1.0 / 0.995 apart across x = 21, a 0.995 notch across y = 20, a pair inside DNWELL, a pair under a marker Dualgate abuts | `Y.NW.2b.h1`, `Y.NW.2b.h2` |
| Y.NW.2b_MV | wells 0.995 apart across x = 40 under Dualgate | `Y.NW.2b.h1` |
| Y.DF.6_MV | the source/drain overhang 0.15 / 0.145, with the OTP_MK and the 3.3 V exemptions | `Y.DF.6.h1` |
| Y.DF.16_LV / _MV | active to well 0.27 / 0.265 and 0.23 / 0.225 | `Y.DF.16.h1` |
| Y.PL.1_LV | a poly line 0.13 / 0.125 wide, and a 0.125 line PLFUSE reaches | `Y.PL.1.h1` |
| Y.PL.1_MV | a 0.5 line in a 5 V marker, and the same line under PLFUSE | `Y.PL.1.h1` |
| Y.PL.2_LV / _MV | the channel 0.13 / 0.125 and 0.47 / 0.465, with the OTP_MK exemption | `Y.PL.2.h1` |
| Y.PL.4_MV | the end cap 0.16 / 0.155, with the 3.3 V exemption | `Y.PL.4.h1` |
| Y.PL.5a / 5b, LV and MV | field poly to active 0.04 / 0.035 and 0.2 / 0.195 | `Y.PL.5.h1` |

The deck's own good/bad pairs were left as they were; nothing this round drew moved them.
