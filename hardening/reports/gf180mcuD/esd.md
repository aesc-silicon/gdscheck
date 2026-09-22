<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / esd: hardening report

Deck `esd` against the GF180MCU design manual, section 7.11 (ESD.1 to ESD.10 and ESD.PL,
`gf180mcu_drm/drm_07_12.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  9
layouts, `tests/data/gf180mcuD/generated/esd/ESD.<rule>.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/esd.rs`, each with a `#[case]` in the `hardening_esd`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through GlobalFoundries' own KLayout runset (`hardening/oracle-gf180.sh <layout> TOP
20 7 100`); the KLayout column below counts only the section's own ids.

`show-deck` lists 15 checks for the 11 rules of the section: ESD.2 carries a space and a
notch, ESD.5a and ESD.5b the area and the hole area, and ESD.3b and ESD.7 are the same
statement twice (as upstream also has them).  Nothing in section 7.11 is missing from the
deck.

Reading the numbers: gdscheck reports one marker per wall for `min_width` and
`min_gate_length` (two per narrow gate), one per pair for a space, an enclosure or an
overlap, and one per region for a `forbidden`; KLayout cuts each into one edge pair or one
polygon.  **No count moved with the tile size anywhere in this deck** - 9 layouts x 3
tiles, every rule identical, and five of the nine layouts straddle x = 20.

Test status on the engine as of this report: 3 of the 9 new cases fail, each on a finding
below; the other 6 pass, and the deck's older good/bad pairs
(`every_rule_in_the_deck_fires`, `static_fixture`) are untouched and still green.

## Findings

### 1. A shared edge is not a space

*Rule ESD.3a, "Minimum space to NCOMP, 0.6".*  The settled GF180 reading is that a shared
edge is a space of nothing and is reported.  `ESD.3a.h1` (c) butts a bare N+ active
(8.8,10)-(10.8,12) against the implant's right edge at x = 8.8.  The implant is not on
that active and owes it the 0.6; the space is zero.

| layout | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- |
| `ESD.3a.h1` | `ESD.3a` 1, only the 0.595 pair at x = 8.8-9.395 | `ESD.3a` 2, the second `(8.8,12.6;8.8,9.4)/(8.8,10;8.8,12)` | `ESD.3a` twice |

Verdict: a false negative of the engine, the same class as finding 1 of the `sab` report,
and the same one-grid-step-wider layout (0.005) fires.  Note the contrast inside this
section: a butted **P+** active is legal by ESD.3b ("min/max space to a butted PCOMP, 0"),
and both tools are rightly silent there (`ESD.3b.h1` (a)); a butted **N+** one is not, and
only KLayout says so.

### 2. A half-covered implant satisfies ESD.9

*Rule ESD.9, "ESD implant layer must be overlapped by Dualgate layer (as ESD implant
option is only for 5V/6V devices)".*  The parenthetical is the reason: the implant exists
for the 5 V/6 V devices, and Dualgate is what marks the 5 V/6 V area (7.6).  An implant
half outside the window has half of itself on a 3.3 V device, which is what the rule is
there to stop.

`ESD.9.h1` (b) draws a device at x = 16 whose implant runs 15.2 to 22.8 with Dualgate over
14.7 to 19.0 - the left 3.8 µm of the implant is covered and the right 3.8 µm is not.
Both decks select the implant as a whole (`esd_no_dualgate` is `esd not_overlapping
dualgate`, upstream `esd.not_overlapping(dualgate)`), so any overlap at all is enough.

| layout | geometry | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- | --- |
| `ESD.9.h1` | Dualgate over the whole implant / over its left half / abutting its edge | `ESD.9` 1, the abutting cell at (5, 11.5) | 1, `(1.2,9.2;8.8,13.8)` | `ESD.9` twice |

Verdict: a false negative shared with the runset.  The abutting case - Dualgate whose left
edge is on the implant's right edge - is read correctly by both: a marker beside the
implant is not over it.

### 3. The extension rules miss a step corner

*Rules ESD.4a ("Extension beyond NCOMP, 0.24") and ESD.6 ("Extension perpendicular to
Poly2 gate, 0.45").*  The settled reading for an enclosure is the closest approach,
euclidian: a corner of the enclosing boundary passing under the value from the enclosed
shape's corner fires.

`ESD.6.h2` puts the gate at the active's right end and stops it on the active's top edge,
so the gate's and the N+ active's upper right corners are both at (8, 5).  The implant's
right side steps back above that corner, from x = 8.85 up to y = 5.05 and then to x = 8.2:
the step's inner corner is at (8.2, 5.05), which is 0.2 in x and 0.05 in y from (8, 5) -
0.206 euclidian.  No two walls face each other across that step, which is the point: the
implant's boundary comes to 0.206 of both the active and the gate, under 0.24 and under
0.45, and there is no projection pair to be found.

| layout | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- |
| `ESD.6.h2` | 0 | `ESD.4a` 2, `(7.965,5;8,5)/(8.2,5.05;8.235,5.05)` and `(8,5;8,4.917)/(8.2,5.133;8.2,5.05)` - one logical violation cut in two | `ESD.4a` and `ESD.6` |

Verdict: a false negative of the engine on ESD.4a, where KLayout's euclidian `enclosed`
catches the corner and gdscheck does not.  ESD.6 is missed by both, upstream because it
reads that rule with `projection` (`esd_edges.enclosing(..., 0.45, projection)`), which
can never see a corner; the manual's word is "perpendicular", which argues for projection,
but a boundary 0.206 from the gate is not an extension of 0.45 in any metric.

### 4. KLayout's ESD.4b fires where the implant overlaps nothing (gdscheck is right)

*Rule ESD.4b, "Minimum overlap of an ESD implant edge to a COMP, 0.45".*  Upstream's
`esd.overlap(comp, 0.45)` reports a COMP the implant merely touches, where the overlap is
not small but absent.  That contradicts ESD.3b, which sets the space to a butted P+ active
to exactly zero: the drawing the section blesses cannot also be an ESD.4b violation.
gdscheck's `min_overlap` is silent on all three, which is the right answer.

| layout | the touching COMP | gdscheck | KLayout |
| --- | --- | --- | --- |
| `ESD.3a.h1` (c) | a butted N+ active at x = 8.8 | 0 | `ESD.4b` 1 |
| `ESD.3b.h1` (a) | a butted P+ active at x = 8.8 | 0 | `ESD.4b` 1 (of 2; the other is the real 0.005 overlap) |
| `ESD.4a.h1` (b) | a butted P+ active at x = 22.235 | 0 | `ESD.4b` 1 |

Verdict: an upstream over-read, recorded so the KLayout column of these three layouts is
not mistaken for a gdscheck miss.  The cases keep gdscheck's answer.

## Tested and clean

Everything below was drawn, run at three tile sizes and behaves as the manual says, in
both tools.

- **The butted P+ active**, which is the section's own idiom.  `ESD.3b.h1`: butted against
  the implant's edge is the zero space ESD.3b asks for and fires neither ESD.3b nor ESD.7
  in either tool (KLayout's `not_outside` does not count a shared edge as inside); 0.005
  under the implant fires ESD.3b and ESD.7, and ESD.4b with them, the overlap being 0.005;
  0.005 clear of it fires ESD.8 alone.  ESD.8 is silent on the butted case in both tools,
  so the section does not contradict itself there.
- **The edge a butted P+ active lets off**, ESD.4a's own condition.  `ESD.4a.h1` gives two
  devices whose implant reaches only 0.235 past the N+ active on the right: the bare one
  fires, and the one whose right edge is shared with a butted P+ active is clean, that
  edge being dropped from the measuring set (`esd_edges_no_pcomp`, upstream
  `esd_edges.not_interacting(pcomp)`).  The other three edges keep their 0.8.
- **Which poly ESD.6 measures from**.  `ESD.6.h1` gives four devices, each with a second
  poly whose right wall stands 0.445 or 0.45 from the implant's edge: lifted off the
  active into the implant's top margin (not a gate, clean), across the active under RES_MK
  (`tgate` is `poly2 and comp not res_mk`, so not a transistor, clean), across it bare
  (fires) and bare at 0.45 (clean).  Both tools, identically.
- **ESD.8's related implant**.  `ESD.8.h1`: the device's own N+ drawn 0.1 inside the
  implant's edge is contained, not faced, and is clean; an unrelated P+ 0.295 outside
  fires; 0.3 is clean; one crossing the implant's edge is clean.
- **ESD.10 is about a marker that covers part of an active, not about its absence.**
  `ESD.10.h1`: no LVS_IO at all is clean, LVS_IO over the active's left half leaves the
  right half as the violation, LVS_IO abutting the active's right edge covers nothing and
  is clean in both tools, LVS_IO over the whole active is clean.
- **ESD.pl's two conditions**.  `ESD.pl.h1`: a 0.795 gate on an ESD device under Dualgate
  fires (gdscheck one marker per wall, KLayout one edge pair); the same gate on a
  transistor 2 µm clear of any implant is clean, the rule being stated of the device the
  implant is on; a gate whose poly touches the implant's edge fires, "interacting" taking
  a shared edge; and a 0.795 gate on an implant with no Dualgate over it is not a 5 V gate
  (`poly2_mv` is `poly2_drawn overlapping dualgate`), so ESD.pl is silent and ESD.9 is the
  only marker.
- **ESD.3a's other cases**: 0.595 fires, 0.6 is clean, an N+ active crossing the implant's
  edge is an active the implant lies on and not one it stands off - clean in both.
- **The tile lines**: `ESD.3a.h1`, `ESD.3b.h1`, `ESD.6.h1`, `ESD.8.h1` and `ESD.pl.h1` all
  straddle x = 20 (`ESD.6.h1` also x = 40); at tiles 20, 7 and 100 every count is the same.

## Resolution (2026-09-22)

- **1 (ESD.3a at a touch)**: fixed in the deck, `abutting: report`.  ESD.3b's blessing
  of the butted P+ active is a rule of its own and is untouched.
- **2 (a half-covered implant)**: fixed in the deck; `esd_no_dualgate` is the part of
  the implant outside the marker, so an implant half on a 3.3 V device is reported.
- **3 (a step corner)**: kept and open.  The edge-layer extensions read projection, and
  no two walls face each other across a step; giving them a closest-approach reading is
  an engine change of its own, and the same open class the IHP gatpoly round named.
  ESD.4a and ESD.6 do carry `metric: euclidian` now, which the polygon path honours.
- **4**: gdscheck was right; nothing to do.
