<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dualgate: hardening report

Deck `dualgate` against the GF180MCU design manual, section 7.6 (DV.1 to DV.9,
`gf180mcu_drm/drm_07_07.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  8
layouts, `tests/data/gf180mcuD/generated/dualgate/DV.<n>.h1.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/dualgate.rs`, each with a `#[case]` in the
`hardening_dualgate` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset.

`show-deck` lists DV.1, DV.2, DV.3, DV.5, DV.6, DV.7, DV.8 and DV.9 - every rule of the
section that is a DRC check.  DV.4 is Appendix B's "not coded" (a statement about the gate
oxide, not a geometry); the manual's own note says DV.9 "can be detected by ERC, not by
DRC", and both tools nevertheless check it as the runset writes it, on the N-well that
carries both a 5/6 V and a 3.3 V PMOS gate.  Nothing in the section is missing.

**The oracle needs `DECKS=dualgate` here.**  The runset's `dnwell` deck calls `nets` on a
layout with no connectivity and aborts the whole run before `dualgate` executes, which
reads as "KLayout is silent on DV.1" in a full-deck run (`gf180.log`: *'nets': The given
layer is not an original layer used in netlist extraction*).  Every KLayout number below
comes from a run of the `dualgate` deck alone.

Marker cuts: gdscheck reports one per wall for `min_width` (two per narrow marker), one per
pair for a space or an enclosure and one per region for a `forbidden`; KLayout reports one
edge pair per width or space, a polygon per enclosure failure and a polygon per forbidden
region, and cuts a corner pair into two.  No count moved with the tile size - 8 layouts x 3
tiles, every rule identical.

Test status on the engine as of this report: 5 of the 8 new cases fail, each on a finding
below; DV.2, DV.5 and DV.9 pass, as do the 9 older dualgate cases.

## Findings

### 1. The enclosure rules do not see a 45° marker wall (DV.1, DV.6, DV.8)

The enclosure of a shape lying inside another is the closest approach from its boundary to
the enclosing one (SPEC, settled 2026-09-21).  All three "Dualgate must enclose X" rules
miss it when the marker's corner is chamfered.

| layout | geometry | gdscheck | KLayout |
| --- | --- | --- | --- |
| `DV.1.h1` | marker chamfered on `x + y = 52.7`, the deep well's corner at (45, 7): 0.4950 perpendicular | silent (6 of 7) | fires, polygon at (44.999, 6.992) |
| `DV.6.h1` | marker chamfered on `x + y = 65.33`, the active's corner at (60, 5): 0.2333 | silent (7 of 8) | fires, polygon at (59.999, 4.99) |
| `DV.8.h1` | marker chamfered on `x + y = 41.55`, the poly's corner at (36, 5): 0.3889 | silent (7 of 8) | fires, polygon at (35.999, 4.983) |

Verdict: a false negative in gdscheck, the same class the IHP round left open on the
extension rules (gatpoly report, finding 11) and here on plain `min_enclosure` with
`interacting_only`, where a 45° wall of the *enclosing* shape passes under the value from a
corner of the enclosed one.  DV.3, the space rule on the same shapes, gets it right
(`DV.3.h1` measures 0.2333 from a chamfered marker corner to an active's corner and fires),
so it is the enclosure side alone.

### 2. An active abutting the marker is 0 away, and is not reported

*Rule DV.3*, "Min. Dualgate to COMP space [unrelated] 0.24".  `DV.3.h1` puts a COMP at
(19, 3)-(21, 5) with the marker's right edge on x = 19: the two share an edge, so the
space is zero.  gdscheck reports 5 markers, none of them this one; KLayout reports the pair
`(19,5.24;19,2.76)/(19,3;19,5)`.  The active is outside the marker (`comp_outside_dualgate`
picks it up in both tools), so no other rule covers it either: DV.6 and DV.7 are about
actives the marker touches from *inside*, and this one it does not overlap at all.

Verdict: a false negative, on the convention that shapes sharing a boundary have no
spacing.  Which way the convention should go is the engine owner's call, but the manual's
reading is plain - zero is less than 0.24 - and the deck's own `PL.5` fixture
(`pl5_touching_corner_is_a_zero_separation`) already settles the corner version the other
way, so a shared *edge* being silent while a shared *point* fires is at least inconsistent.
gdscheck reports 5 where the manual asks for 6.  (KLayout's 7 is those 6 with the chamfer
corner counted as two degenerate pairs.)

### 3. A marker that covers one active is excused from straddling another

*Rule DV.7*, "COMP (except substrate tap) can not be partially overlapped by Dualgate."
The deck builds `dv7_viol` as `overlapping(dualgate, comp_dv)` less
`covering(dualgate, comp_dv)`: a marker that completely covers *any* active is removed from
the violation set, whatever it does to the rest.

`DV.7.h1` draws a marker at (10, 2)-(16, 6) holding an active at (11, 3)-(13, 5) and
straddling a second at (15, 3)-(18, 5).  gdscheck is silent on that marker (3 markers where
the manual asks for 4); KLayout reports it, polygon (10, 2; 16, 6).  A minimal two-active
layout reproduces it on its own: gdscheck reports only the DV.6 crossing, KLayout both
DV.6 and DV.7.

Verdict: a false negative in gdscheck, not shared with the runset.  A marker covering one
device says nothing about the next device along, and this is the normal shape of the
mistake the rule exists to catch: one marker drawn over a row of transistors, ending in the
middle of the last one.

## Tested and clean

Everything below agreed between gdscheck and the runset (up to each tool's marker cut) and
did not move with the tile size.

* **DV.1** (`DV.1.h1`): 0.495 fires and 0.5 is clean; a deep well crossing the marker's
  edge fires on the part outside; one abutting the marker from outside is a 3.3 V well and
  is clean; one sharing the marker's edge is enclosed by 0 and fires; the well's edge on
  x = 20 with the marker 0.495 past it, and the marker's edge on x = 20 with the well 0.495
  inside, both fire; 0.5 at x = 42 is clean; a second well in the same marker at 0.495
  fires on its own.
* **DV.2** (`DV.2.h1`): 0.435 between two markers, a 0.435 notch in a U, corner to corner
  at 0.438, 0.435 straddling x = 20 and 0.435 from x = 42 fire; 0.44, 0.4455 and two
  overlapping boxes (one marker) are clean.  5 markers in gdscheck, 6 in KLayout - the
  corner pair cut in two.
* **DV.3** (`DV.3.h1`): 0.235 from an active outside the marker, from a substrate tap
  (which this rule does not excuse), from a chamfered marker corner at 0.2333, with the
  active's edge on x = 20 and with the marker's edge on x = 20; 0.24 and 0.24 at x = 42 are
  clean; an active partly inside the marker is DV.6's and DV.7's, not this rule's.
* **DV.5** (`DV.5.h1`): 0.695 in x and in y, a 45° strip 0.693 across and a 0.695 bar
  across x = 20 fire (two walls each in gdscheck, one edge pair each in KLayout); 0.7,
  0.700 and 0.7 across x = 42 are clean.
* **DV.6** (`DV.6.h1`): 0.235 for an N+ active; an N+ tap in an N-well is no substrate tap
  and fires; a P+ active crossing the N-well's edge is no substrate tap either and fires;
  an active sharing the marker's edge is enclosed by 0 and fires; one crossing the edge
  fires (with DV.7); the active's edge on x = 20 and the marker's edge on x = 20 both fire;
  0.24, 0.24 at x = 42, a substrate tap 0.1 inside, a substrate tap crossing the marker's
  edge and one abutting the N-well from outside are clean.
* **DV.7** (`DV.7.h1`): a marker straddling an active fires, and so do one across x = 20
  and one across x = 42; a straddled substrate tap is excused; an active sharing the
  marker's edge is covered, not straddled; one abutting the marker from outside is DV.3's;
  and a marker drawn as two abutting boxes is one marker and covers the active across the
  seam.
* **DV.8** (`DV.8.h1`): 0.395 fires and 0.4 is clean; a poly crossing the marker's edge
  fires; one abutting from outside is clean; one sharing the edge is enclosed by 0 and
  fires; a poly under a marker ring drawn as four boxes, 0.395 from the hole, fires while
  the poly *in* the hole is outside and clean; the poly's edge on x = 20 and the marker's
  edge on x = 20 fire, 0.4 at x = 42 is clean; a 300 µm poly running into a marker at
  x = 250 fires once.
* **DV.9** (`DV.9.h1`): one N-well carrying a 6 V and a 3.3 V PMOS fires once; an N-well
  drawn as two abutting boxes is one well and fires; a 3.3 V gate that a second marker
  merely straddles is still 3.3 V and fires (the straddle itself is DV.6, DV.7 and DV.8); a
  well with two 3.3 V gates beside the 6 V one is still one report; a 44 µm well with the
  6 V gate at x = 2 and the 3.3 V one at x = 42 fires across every tile line; two separate
  wells 1.4 apart are clean, and a 3.3 V gate under a V5_XTOR with no Dualgate is excused
  in both tools (`pgate.not_inside(v5_xtor)`), which is PL.11's business, not this rule's.

## Resolution (2026-09-22)

- **1 (the 45° marker wall)**: fixed in the deck; DV.1, DV.6 and DV.8 read
  `metric: euclidian`, the settled reading for every enclosure since 2026-09-21.
- **2 (a COMP abutting the marker)**: fixed in the deck, `abutting: report` on DV.3.
- **3 (a marker excused by the active it covers)**: fixed in the deck.  DV.7 is about
  the COMP - "COMP can not be partially overlapped by Dualgate" - so the violation is
  an active the marker reaches into and does not cover, read on the active and not on
  the marker.

Foundry case `dualgate.gds.gz`: DV.1 4 -> 5, DV.3 2 -> 4, DV.6 4 -> 5, DV.8 7 -> 9.
