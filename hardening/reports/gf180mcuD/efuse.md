<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / efuse: hardening report

Deck `efuse` against section 10.11 of the GF180MCU design manual, "0.18 µm MCU eFuse
Design Rules".

## Coverage

The section lists twenty-seven rules, EF.01 to EF.22b, every one of them measurable on
geometry, and none of them appears in Appendix B.  `show-deck` lists thirty-four entries
under exactly those twenty-seven ids (EF.02, EF.04a, EF.05, EF.10, EF.11, EF.12 and EF.17
each take two checks).  **No coverage finding.**

## What was drawn

An eFuse is not a shape but a chain of them: a cathode pad, a narrow link and an anode
pad, all poly, all inside EFUSE_MK and P+, with PLFUSE over the link and LVS_SOURCE over
the anode - and the deck tells the three parts apart by which of those four layers covers
them.  A bar of any one layer on its own is not a fuse, so every probe here is a whole
one, with one thing about it moved.

What makes this section different from the others in chapter 10 is that nine of its rules
read *"Min. Max."* and **fix** a dimension rather than bounding it: EF.02's 0.18 µm link
width, EF.03's 1.26 length, EF.06 to EF.09's four pad dimensions, EF.21's 5.53 µm of poly
end to end and EF.22a/EF.22b's 1.04 and 0.44 shoulders.  A fixed dimension has a step past
it on *both* sides, and the deck's own bad halves only ever take the short one, so this
round takes the long one throughout.  EF.16a and EF.16b fix a *count* the same way, so
five contacts are drawn as well as three.

16 layouts under `tests/data/gf180mcuD/generated/efuse/EF.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/efuse.rs`, each with a `#[case]` in `hardening_efuse`.  Every layout ran at
tiles 20, 7 and 100 and through the upstream runset (`DECKS=efuse`).  **No count moved
with the tile size**, on any layout, including `EF.17.h1`, whose two marker gaps are
centred on the lines at x = 20 and x = 40.

`EF.00.h1` is the fuse the manual describes with nothing wrong with it: both tools report
nothing at all on it, which is what makes the rest of the round readable.

## Findings

### 1. A shared edge is not reported as a space (false negative, three rules)

EF.12 (*Min. Space of Cathode Contact to PLFUSE end* - 0.155 µm), EF.13 (*Min. Space of
Anode Contact to PLFUSE end* - 0.14 µm) and EF.20 (*Min. PLFUSE space to COMP, Nplus, ESD,
SAB, Resistor* - 2.73 µm) each have a probe drawn flush against the shape they are
measured from.

| fixture | probe | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- | --- |
| `EF.12.h1` | contacts 0.155 from the link's end | - | - |
| | 0.15 | EF.12 at (31.69, 11.04)-(31.84, 11.04) | EF.12 |
| | flush against the link's end | **EF.15 only** | EF.12 *and* EF.15 |
| `EF.13.h1` | 0.14 / 0.135 / flush at the anode | EF.13 once, EF.15 once | EF.13 twice, EF.15 once |
| `EF.20.h1` | active 2.73 / 2.725 / flush against the link's wall | EF.20 once, at (31.84, 11.22)-(31.84, 13.945) | EF.20 twice, the second edge pair at (51.84,11.22;53.1,11.22)/(53.84,11.22;51.84,11.22) |
| `EF.15.h1` | contact 0.005 short of the link, one sharing its end edge, one meeting its corner | EF.12 twice - the 0.005 and the **corner** - EF.15 twice | EF.12 four times, EF.15 twice |

A shared edge is a space of nothing, which is under 0.155, 0.14 and 2.73 alike; the
settled reading (`abutting: report`) says the pair fires.  None of the three entries
carries the parameter.

`EF.15.h1` is the sharper form of it: gdscheck reports a space of 0.0000 µm where the
contact meets the link at a single *point*, and nothing at all where it meets it along a
whole *edge*.  One of those two is a space of nothing and the other is not, and it is the
edge that is the real one - a contact lying against the end of the fuse is the layout this
rule exists to forbid, and EF.15 catches it only because it asks a different question
("no contact may touch PLFUSE").  The same class is finding 3 of `nat`, finding 3 of
`otp_mk` and finding 2 of `ymtp_mk`; on this deck it is three more entries.

## Read and left alone

- **Marker granularity.**  gdscheck's `min_width` gives one marker per wall against
  KLayout's one edge pair (EF.02 is 2 + 1 against 6 on `EF.02.h1`), and the exact-length
  rules are edge counts on both sides and agree exactly (EF.03 4 / 4, EF.06 to EF.09 2 / 2
  each, EF.21 4 / 4, EF.22a 1 / 1, EF.22b 2 / 2).  `min_space` at a corner is 1 against 2.
  Settled; the violating structures agree everywhere.
- **EF.02's long side does not mistake a length for a width.**  `EF.02.h1`'s bars are 2 µm
  long, and neither the 0.18 nor the 0.185 bar is reported as 2 µm wide; the 0.185 one is
  reported once as `width 0.1850 > 0.1800`, which is the max half of the fixed dimension
  doing exactly what the manual asks.
- **EF.19's zero space allows a shared edge.**  `EF.19.h1`'s first probe lays Metal1 flush
  against the link's long wall.  Both tools stay silent, and that is right: *"Min. PLFUSE
  space to Metal1, Metal2"* is 0, so a space of nothing satisfies it, and EF.18's *"no
  cross with any … Metal1, Metal2"* is about crossing, not touching.  The deck's
  `overlapping` reading is the one the manual supports; upstream's `not(plfuse.outside(…))`
  would be the stricter `interacting` one, and on this layout KLayout agrees with
  gdscheck anyway.  Metal over the link fires both ids in both tools.
- **EF.14's zero enclosure likewise.**  `EF.14.h1`'s first fuse has its marker ending
  exactly on the anode's right edge, where LVS_SOURCE ends too; both tools stay silent,
  and the source that runs 1 µm past the marker fires in both.
- **EF.10 and EF.11 read pad against pad of the same kind.**  The manual says *"Min.
  Cathode Poly2 to Poly2 space"* and *"Min. Anode Poly2 to Poly2 space"*, which reads as
  the pad against *any* poly; both decks implement `cathode.space(0.26)` and
  `anode.space(0.26)`, which is the pad against another pad of its own kind.  The fixtures
  are drawn the way both decks read it - two fuses cathode to cathode and two anode to
  anode, at 0.26 and at 0.255 - and both tools answer identically.  Recorded, not
  reported: a poly bar in the marker that is neither PLFUSE nor LVS_SOURCE *is* cathode by
  the section's own definition, so the two readings differ only for poly outside the
  marker, and EF.01 forbids poly that meets the marker and is not inside it.
- **EF.16a "at each ends".**  The manual reads *"Cathode must contain exact number of
  Contacts at each ends: 4"*; both decks count the contacts in the pad and ask for exactly
  four, without any notion of an end.  `EF.16.h1` is drawn to that reading - three, four
  and five in one pad - and both tools agree on all four probes.  A fuse with four
  contacts bunched at one end of the cathode would pass both, which is worth someone's
  judgement but is not a difference between the tools.
- **Moving one dimension moves the ones derived from it.**  The section's numbers are
  consistent with each other - 1.84 + 1.26 + 2.43 is EF.21's 5.53, and the shoulders left
  when the 0.18 link meets the 2.26 and 1.06 pads are EF.22a's 1.04 and EF.22b's 0.44 - so
  a fuse with one dimension wrong breaks two or three rules, not one.  Both tools break
  the same ones in the same places; the cases list them all.
- **The anode's width is bent by two grid steps, not one.**  The anode grows either side
  of the fuse's centre line, so 1.065 µm would put its own edges half a grid off; the
  fixture uses 1.07.  Every other dimension moves by the single 0.005 step.

## Tested and clean

Drawn at the value and, where the manual fixes it, one step past it on each side, and
answered exactly as the manual reads at all three tile sizes and in agreement with the
runset:

| rule | what was drawn | fixture |
| --- | --- | --- |
| all 27 | the fuse the section describes, with nothing wrong | `EF.00.h1` |
| EF.02 | a 2 µm PLFUSE bar 0.18 / 0.175 / 0.185 wide | `EF.02.h1` |
| EF.03, EF.21 | the link 1.26 / 1.255 / 1.265 long | `EF.03.h1` |
| EF.06, EF.07 | the cathode 2.26 / 2.265 wide and 1.84 / 1.845 long | `EF.06.h1` |
| EF.08, EF.09 | the anode 1.06 / 1.07 wide and 2.43 / 2.435 long | `EF.06.h1` |
| EF.22a, EF.22b | the shoulders 1.04 and 0.44, narrowed by widening each pad | `EF.06.h1` |
| EF.10 | two fuses cathode to cathode, 0.26 / 0.255 | `EF.10.h1` |
| EF.11 | two fuses anode to anode, 0.26 / 0.255 | `EF.11.h1` |
| EF.12 | the cathode's contacts 0.155 / 0.15 from the link | `EF.12.h1` |
| EF.13 | the anode's contacts 0.14 / 0.135 from the link | `EF.13.h1` |
| EF.14 | LVS_SOURCE flush with the marker's edge, and 1 µm past it | `EF.14.h1` |
| EF.15 | a contact 0.005 short of the link, one on its edge, one on its corner | `EF.15.h1` |
| EF.16a, EF.16b | four contacts per pad, three on a cathode, five on a cathode, five on an anode | `EF.16.h1` |
| EF.17 | markers 0.26 / 0.255 apart across x = 20 and x = 40, and a 0.255 slot | `EF.17.h1` |
| EF.18, EF.19 | Metal1 flush against the link, and over it | `EF.19.h1` |
| EF.20 | an active 2.73 / 2.725 from the link, and Nplus, ESD, SAB and Resistor at 2.725 | `EF.20.h1`, `EF.20.h2` |

EF.01, EF.04a to EF.04d and EF.05 keep the good/bad pairs the deck already had: they are
`forbidden` on a derived layer with no number to step past, and every fixture of this
round carries their clean half - the marker inside P+, the link coinciding with the poly
and touching both pads, three rectangles, the source exactly on the anode - which
`EF.00.h1` shows both tools reading as legal.

## Note on reading the oracle here

`hardening/oracle-gf180.sh` was run with `DECKS=efuse`, which quiets the runset's other
decks but not gdscheck's suite: a fuse's contacts trip CO.6 and its P+ trips PP and DF
rules from the `contact`, `pplus` and `comp` decks on every layout.  Those columns are
`gdscheck-only` noise; the cases run the one deck through the `hardening` helper and see
none of it.

---

## Resolution (2026-09-22)

The one finding - the shared edge - was fixed: EF.12, EF.13, EF.15 and EF.20 carry
`abutting: report`, and `EF.15.h1`'s contact lying against the link's end edge is reported
beside the one that meets it at a point. On the foundry layout EF.12 goes 21 -> 29, EF.13
5 -> 8 and EF.20 55 -> 65.

The four readings recorded at the end of the report - EF.10/EF.11 as pad-against-pad,
EF.16a counting contacts over the whole pad, and the two "NA" columns - are left as they
are, for the gdscheck owner.
