<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / poly2: hardening report

Deck `poly2` against the GF180MCU design manual, section 7.7 (PL.1 to PL.12,
`gf180mcu_drm/drm_07_08.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  22
layouts, `tests/data/gf180mcuD/generated/poly2/<rule>.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/poly2.rs`, each with a `#[case]` in the
`hardening_poly2` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`hardening/oracle-gf180.sh`); the KLayout column below is a run of the `poly2` deck alone
(`DECKS=poly2`), so the density and neighbouring-deck markers are out of it.

`show-deck` lists every rule of the section that is a DRC check.  PL.3b, PL.8 (a density
rule, in the `density` deck), PL.10 and PL.12_LV are Appendix B "rules not coded" or not
this deck's; the manual's PL.5a and PL.5b are one measurement and the deck carries both
ids, as the runset does.  Nothing in section 7.7 is missing from the deck.

Reading the numbers: gdscheck reports one marker per wall for `min_width` and the
gate-length rules (two per narrow gate), one per pair for a space or an enclosure, one per
corner for PL.6 and one per region for a `forbidden`; KLayout cuts a width or a space into
one edge pair and a corner pair into two.  No count moved with the tile size anywhere in
this deck - 22 layouts x 3 tiles, every rule identical.

Test status on the engine as of this report: 12 of the 22 new cases fail, each on a
finding below; the other 10 pass, as do the 27 older poly2 cases.

## Findings

### 1. A poly that only *touches* the Dualgate is read in no voltage column at all

*Rules PL.1, PL.2, PL.4, PL.5.*  The manual splits every one of them into a 3.3 V and a
5 V/6 V value, and the marker is what says which: "This layer defines the 5V/6V area"
(7.6).  A poly lying wholly outside the marker is a 3.3 V interconnect, whether or not its
edge happens to coincide with the marker's.

The deck (and the runset it follows) builds the low-voltage layer as
`poly2_drawn.not_interacting(v5_xtor).not_interacting(dualgate)` and the medium-voltage one
as `poly2_drawn.overlapping(dualgate)`.  A poly that touches the marker without overlapping
it is in neither: `not_interacting` drops it from the 3.3 V set and `overlapping` never
picks it up.  Four layouts, each a violation that nothing reports:

| layout | geometry | gdscheck (20/7/100) | KLayout | manual |
| --- | --- | --- | --- | --- |
| `PL.1.h1` | a 0.175 poly at x = 14.825-15.0, the marker's left edge on x = 15 | `PL.1_LV` 2 of 4 | 1 of 2 | fires |
| `PL.2.h3` | two 0.275 gates whose poly ends on the marker's edge (2, 6) and on its corner (9.275, 6) | 0 | 0 | `PL.2_LV` x4 |
| `PL.4.h3` | a 0.215 end cap, the poly ending on the marker's edge at (2, 4)-(6, 8) | 0 | 0 | `PL.4_LV` |
| `PL.5.h3` | a field poly 0.095 from an active, its end on the marker's edge | 0 | 0 | `PL.5a_LV`, `PL.5b_LV` |

Verdict: a false negative of the deck, shared with the runset, and the shape that triggers
it (a poly butting up against the marker's edge) is ordinary layout.  PL.7 shows the fix is
not a general one: its layer is `tgate.not(dualgate)`, a geometric difference and not a
whole-region selector, and `PL.7.h2` - the same touching gate - fires in both tools.

### 2. A poly crossing the PLFUSE marker's edge is under no width rule

*Rule PL.1 / PL.1a.*  "Interconnect Width (outside PLFUSE)" is 0.18 at 3.3 V and 0.2 at
5 V/6 V; inside the fuse marker it is 0.18 at every voltage.  The deck selects whole
regions: `not_overlapping(plfuse)` for the outside rule and `inside(plfuse)` for the
inside one.  A poly that runs out of the marker is neither, so no width applies to it.

`PL.1.h2` draws two: a 0.19 medium-voltage poly crossing the marker's edge at x = 15.5-19
(inside the fuse 0.19 is legal, outside it is not) and a 0.175 low-voltage one crossing at
x = 22-25.  gdscheck reports `PL.1_LV` 2 and `PL.1_MV` 2 (the two polys that lie entirely
outside); the manual asks for 4 of each, the outside part of the crossing polys included.
KLayout reports the same 1 + 1.  A poly *abutting* the marker from outside (x = 26.825-27)
is outside and fires in both tools, which is the reading the manual supports.

Verdict: a false negative, shared with the runset.  It is the same shape of defect as
finding 1 on a different marker.

### 3. The YMTP cut leaves a stub that fires as an interconnect

*Rule PL.1_LV.*  The rule exempts the YMTP marker (`poly2_lv.outside(plfuse).not(ymtp_mk)`
upstream, `poly2_lv_out_plfuse = ... difference ymtp_mk` in the deck), and the exemption is
a geometric subtraction: the marker is cut out of the poly before the width is read, which
invents walls along the marker's own edge.

`PL.1.h3` draws a 0.4 wide poly at x = 9.0-9.4 running from y = 2 to y = 6.1, with a YMTP
marker up to y = 6.  The poly is 0.4 wide everywhere; the cut leaves a 0.4 x 0.1 rectangle
sticking out of the marker, and both tools measure *that* rectangle: gdscheck 2 markers
("width 0.1000 µm < 0.1800 µm" at (9, 6)-(9.4, 6) and (9, 6.1)-(9.4, 6.1)), KLayout 1 edge
pair.  The layout's only real violation is the 0.175 poly at x = 3 running out of a second
marker, which both tools also find.

Verdict: a false positive, shared with the runset.  gdscheck reports 4 where the manual
asks for 2.  The same class as the PL.6 comment in `pdk.yml` about not subtracting the
marker before looking for corners; here it is subtracted before looking for walls.

### 4. A poly crossing an active under RES_MK is read as two polys 0.23 apart

*Rule PL.3a.*  "Space on COMP / Space on Field", 0.24 both.  The deck's layer is
`tgate` (poly over comp, less `res_mk`) unioned with the field poly (poly less comp).
Where a resistor marker covers the crossing, the on-active piece is dropped from `tgate`
and never returned by the field half, so a single poly crossing a narrow marked active
becomes two pieces with a hole between them.

`PL.3a.h1` draws a 0.4 poly crossing a 0.23 wide COMP at x = 12-15 under a RES_MK.
gdscheck reports `PL.3a` "space 0.2300 µm < 0.24 µm" at (13, 2)-(13, 2.23); KLayout reports
the same pair.  The manual's rule is a space between polys, and there is one poly here.

Verdict: a false positive, shared with the runset.  gdscheck reports 6 where the manual
asks for 5.  The layout's other five - two gates 0.235 apart, a gate against a field poly,
a 0.235 slot cut across the active's edge, and gaps straddling x = 20 and starting on
x = 40 - are right in both tools, and a 0.24 gap from x = 42 is clean.

### 5. PL.4_MV is not checked inside the SRAM core marker

*Rule PL.4.*  "Extension beyond COMP to form Poly2 end cap", 0.22 at every voltage; the
manual's section 7.7 names no exemption, and section 11 has no PL rules of its own.  The
deck's `poly_pl_mv` is `poly2_mv less the markers, not_overlapping(sramcore)` - the SRAM
core selector belongs to PL.5a_MV / PL.5b_MV upstream (`poly_pl_mv.outside(sramcore)
.separation(...)`), where it is written per rule, and PL.4_MV upstream has no such filter.

`PL.4.h1` draws two medium-voltage gates with a 0.215 cap: one under a plain Dualgate at
x = 17-19 and one under a Dualgate and an SRAMCORE marker at x = 22.5-24.5.  gdscheck
reports one `PL.4_MV`, KLayout two.

Verdict: a false negative of the deck, not shared with the runset - the SRAM core filter
has been pulled up from the two rules that carry it into the shared layer.

### 6. An end cap over a chamfered active corner is not measured

*Rule PL.4_LV.*  The extension is read at the closest approach (SPEC, settled 2026-09-21).

`PL.4.h2` draws a COMP whose top-right corner is chamfered at 45° (`x + y = 15.6`, the
chamfer from (13, 2.6) to (12.6, 3)) and a 0.3 wide gate at x = 12.65-12.95 ending at
y = 3.255, so the poly's top-left corner stands 0.2157 from the chamfer, perpendicular.
gdscheck is silent; the manual asks for a `PL.4_LV`.  A second, identical pair at
x = 16.65 with the end at y = 3.655 stands 0.4985 off and is clean.

KLayout fires on *both*, and its markers say why it is no help here: its pairs are
`(12.73,2.87;12.95,2.65)/(12.95,2.961;12.95,2.65)` and the same at 16.95 - the chamfer
against the poly's *side* wall where the active's edge leaves the poly, not against the
cap.  Its 0.4985 report is a false positive of its own.

Verdict: a false negative in gdscheck, on an open class the IHP round already named
(gatpoly report, finding 11: the extension rules on angled walls).  The rectilinear half of
the layout - 0.215 caps starting on x = 20 and x = 40, straddling x = 20 and x = 42, and a
0.22 cap from x = 42 - agrees in both tools at every tile size, as do the caps chamfered at
45° from 0.25 and 0.35 up the poly's own wall (the chamfer only recedes from the active) and
the stub and the poly lying wholly inside the active, which have no cap to read.

### 7. The space to a *related* active is skipped where the two shapes overlap elsewhere

*Rule PL.5b.*  "Space from field Poly2 to related COMP", 0.1 at 3.3 V - the related
active is the gate's own, so the pair whose distance is measured is a poly and a COMP that
overlap somewhere else in the same shape.

`PL.5.h1` draws it twice: an L-shaped active (28, 2)-(31, 4) with its arm at x = 29.905,
crossed by a gate at x = 30.0-30.4, and the same as a U at x = 44-49 whose left arm is
0.095 from the gate at x = 46.  gdscheck reports nothing for either; KLayout reports
`PL.5a_LV` and `PL.5b_LV` on both (`(30,2.469;30,4.031)/(29.905,4;29.905,2.5)` and the
same at 46).  Taking the same two shapes apart shows what decides it: with the arm drawn
as a *separate* COMP box 0.3 above the bar, or with the poly not crossing the active at
all, gdscheck reports the pair; as soon as the two shapes overlap anywhere, the 0.095 gap
elsewhere is gone.

Verdict: a false negative in gdscheck, and PL.5b exists precisely for this configuration.
gdscheck reports 5 of each id where the manual asks for 7.  The rest of the layout - the
0.095 and 0.099 pairs, 0.295 and 0.2 under a Dualgate, gaps from x = 20, across x = 20 and
across x = 42, with 0.1, 0.106, 0.3 and 0.1 from x = 42 clean - agrees in both tools.

### 8. A 90° bend within 0.1 of the active's edge is not counted

*Rule PL.6.*  "90 deg bends on the COMP are not allowed."  The rule is about a vertex, and
a vertex either lies on the active or it does not.  Upstream probes it with a square:
`poly2_drawn.corners(90.0).sized(0.1).…inside(comp.not(ymtp_mk))`, so a corner needs 0.1 of
active on every side to count; the deck reads the same margin.

`PL.6.h1` puts an L's convex elbow 0.05 inside the active's right edge (at (11.95, 3), the
active ending at x = 12): both tools report only the concave elbow 0.5 further in.
`PL.6.h3` repeats it at 0.1 (at (17.9, 3), the edge at x = 18) and at 0.095 (at (25.905, 3),
the edge at x = 26), with the same result: 1 marker each where the manual asks for 2.
gdscheck reports 14 and 3 where the manual asks for 15 and 5.

Verdict: a false negative, shared with the runset and inherited from it; a bend 0.05 from
the active's edge is a bend on the active.  Everything else about PL.6 came out right in
both tools and at every tile size: the two elbows of an L (2), a convex corner 0.005
*outside* the active (only the concave one), a T stub ending inside (4), a step in the
poly's width (2), an elbow inside a YMTP marker (0), an elbow in the hole of an active ring
drawn as four boxes (0), an elbow on the seam of two abutting active boxes (2), an L drawn
as two overlapping boxes (2 - the union's corners, not the boxes'), a convex elbow exactly
on the active's edge (not on the active), both elbows on the edges at the active's corner
(0), and `PL.6.h2`'s seven elbows on and beside the tile lines - x = 20, ±0.005, x = 40,
(40, 40), (21, 21) and x = 42 - 14 markers at tiles 20, 7 and 100 alike.

## Tested and clean

Everything below agreed between gdscheck and the runset (up to each tool's marker cut) and
did not move with the tile size; no need to redo it.

* **PL.1 / PL.1a, the columns and the bound** (`PL.1.h1`, `PL.1.h2`): 0.175 fires the
  3.3 V width and 0.18 is clean; under a Dualgate 0.195 and the 3.3 V-legal 0.18 fire and
  0.2 is clean; a poly crossing the marker's edge is medium voltage; inside PLFUSE 0.175
  fires at both voltages and 0.19 is clean at 5 V, where outside it would not be.
* **PL.2, the channel** (`PL.2.h1`): 0.28 clean, 0.275 fires at both walls; a horizontal
  gate; one poly over two actives is two gates; a poly bitten to 0.275 only over the active
  is a 0.275 channel; walls on x = 20 and x = 21, gates straddling x = 20, x = 40 and
  x = 42.  22 markers at every tile.
* **PL.2_MV, the four device classes** (`PL.2.h2`): 6 V N at 0.695 / 0.7, 6 V P at 0.545 /
  0.55, 5 V N at 0.595 / 0.6 and 5 V P at 0.495 / 0.5, all under one marker with a V5_XTOR
  over the 5 V pair; a gate with no implant and an N+ gate in an N-well are no device and
  are silent in both tools.
* **PL.2 on 45° gates** (`PL.2.h4`): the channel is read between slanted walls too - 0.272
  fires, 0.286 is clean - including a bar across x = 20 and one running up-left.
* **PL.4, the rectilinear cases** (`PL.4.h1`, `PL.4.h2`): 0.215 at the bottom, at both ends,
  on a horizontal gate's left, under a Dualgate; 0.22 clean; caps on and across the tile
  lines.
* **PL.5, the metrics and the columns** (`PL.5.h1`, `PL.5.h2`): 0.095 beside an active,
  corner to corner at 0.099 (0.106 clean), 0.295 and 0.2 under a Dualgate (0.3 clean), and
  a poly band 0.092 from a chamfered active corner (0.1025 clean).
* **PL.7** (`PL.7.h1`, `PL.7.h2`): 0.2934 fires and 0.3005 is clean at 3.3 V, 0.693 fires
  and 0.70004 is clean under a Dualgate, with no implant needed; a bar across x = 20 and
  x = 21; a bar running up-left; an N+ 6 V bar firing PL.7_MV and PL.2_MV together; and the
  gate whose end touches the marker, which stays 3.3 V here (finding 1's counter-example).
* **PL.9** (`PL.9.h1`): a poly across the marker's edge, two boxes meeting on the edge, a
  poly bridging two markers, one leaving the hole of a marker ring drawn as four boxes, a
  300 µm poly reaching a marker at x = 250, the edge on x = 20, 21, 40 and 42, and a poly
  0.005 outside - 10 in both tools; a poly inside, one abutting the edge from outside, one
  in the ring's hole and one under a V5_XTOR alone are clean.
* **PL.11** (`PL.11.h1`): 0.005 off a Dualgate, 0.005 short of one across a tile line, and
  one at (1000, 1000) fire; abutting, overlapping, inside an OTP marker, and two V5_XTOR
  boxes of which one touches are clean.
* **PL.12** (`PL.12.h1`): an active under a gate half outside its V5_XTOR, one abutting the
  marker from outside, and one running out of it past x = 20 fire; inside, enclosed by 0,
  and an active with no gate are clean.

## KLayout-only noise worth knowing

On any layout with a 45° gate (`PL.2.h4`, `PL.7.h1`, and the deck's existing
`PL.7_LV.good`), the runset reports PL.1, PL.4, PL.5a and PL.5b on the wedges where the
slanted poly crosses the active's edge - 8 markers per bar on a layout whose only violation
is the gate width.  gdscheck is silent there, and the manual is on its side: PL.7 exists to
give a 45° gate its own width, so the geometry it describes cannot be a PL.4 or PL.5
violation by construction.  The same artefact is what makes KLayout's PL.4 unusable on a
chamfered active corner (finding 6).
