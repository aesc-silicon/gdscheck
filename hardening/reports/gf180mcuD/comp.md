<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / comp: hardening report

Deck `comp` against the GF180MCU design manual, section 7.5 (`gf180mcu_drm/drm_07_06.txt`,
DF.1a to DF.19).  34 layouts, `tests/data/gf180mcuD/generated/comp/<rule>.h<n>.gds.gz`,
drawn by the `hardening` section of `gen/gf180mcuD/comp.rs`, each with a `#[case]` in the
`hardening_comp` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through the foundry's own runset
(`hardening/oracle-gf180.sh <layout> TOP 20 7 100`).  136 KB of fixtures.

`show-deck` lists 47 entries for the section's 44 rules: DF.3a and DF.3c each appear twice
(a space and a notch under one id), and DF.1a/2a/3a/3c/4a-4e/5/6/7/8/13/14/16/17/19 each
appear as an `_LV` and an `_MV` rule.  Nothing the manual asks for is missing: DF.1b is
marked "not coded" in the manual itself, and DF.15a/b say "can be detected by ERC, not by
DRC".  DF.3c_MV has no value in the manual ("NA"); both the deck and the upstream runset
turn it into "a DRC_BJT marker holding two 5 V COMPs is forbidden", which is upstream's
invention, not the manual's rule.

Reading the numbers below: gdscheck cuts a `min_width` violation into one marker per
narrow wall (two per bar), a space, a notch and a `max_space` into one per violating pair,
an area and a `forbidden` into one per region, an enclosure into one per short edge;
KLayout gives one edge pair per rule per edge, so a corner-to-corner gap comes as two
there and a width as one.  **No count moved with the tile size in any of the 34 layouts.**

Test status on the engine as of this report: 14 of the 34 cases fail, all on the findings
below; the other 20 pass.

## Findings

### 1. A COMP that is in neither voltage column (false negative, KLayout too)

Manual: the LAYOUT RULE column of 7.5 has a 3.3 V and a 5 V/6 V value; 7.6 DV.4 says
"circuits covered by Dualgate layer will have 5V/6V gate oxide", and DV.7 that "COMP
(except substrate tap) can not be partially overlapped by Dualgate".  Every COMP has a
width, a space and an N-well spacing; which column applies depends on the device, and no
COMP is exempt from both.

The deck reads 3.3 V as `comp NOT_INTERACTING v5_xtor NOT_INTERACTING dualgate` and
5 V as `comp OVERLAPPING dualgate` (upstream's own wording).  Three kinds of COMP fall
between the two:

- A COMP that *touches* a Dualgate edge without overlapping it.  `DF.1a.h1`, a 0.215 wide
  bar at (5, 7)-(5.215, 9) whose left wall lies on the Dualgate edge x = 5: interacting
  excludes it from the 3.3 V layer, overlapping (area) excludes it from the 5 V one.
- A COMP under V5_XTOR with no Dualgate over it.  `DF.1a.h1`, the 0.215 bar at
  (7.5, 7.2)-(7.715, 8.8) under V5_XTOR (7, 7)-(9, 9).
- A pair of COMPs of different voltages.  Each rule intersects one voltage's layer with
  itself, so a 3.3 V shape and a 5 V shape are never measured against each other:
  `DF.3a.h1` at (3, 7)-(3.275, 8), a 0.275 space; `DF.16.h1` at (14, 5.425) and
  (14, 12.425), 0.425 from an N-well; `DF.17.h1` at (15.115, 7.5) and (15.115, 12.5),
  0.115 from an N-well.

- gdscheck @20/7/100: `DF.1a.h1` DF.1a_LV 2 (the one bar at x = 4), DF.1a_MV 8; the two
  bars above are reported by nothing.  `DF.3a.h1` DF.3a_LV 3, the mixed pair silent.
  `DF.16.h1` DF.16_LV 1, `DF.17.h1` DF.17_LV 1, both mixed pairs silent.
- KLayout: the same, exactly (the derivations are upstream's).
- Verdict: a false negative in both tools.  The smaller of the two values certainly
  applies to every one of these shapes: 0.215 is under the 3.3 V minimum whatever the
  device, and a 0.275 space breaks the 3.3 V rule even if one neighbour is a 5 V device.
  The cases carry the 3.3 V rule for all of them.

### 2. The max-space reach is a box, and one grid step short when confined (false negative)

Manual: "DF.13 Max distance of Nwell tap (NCOMP inside Nwell) from (PCOMP inside Nwell)
20 / 15", "DF.14 Max distance of substrate tap (PCOMP outside Nwell) from (NCOMP outside
Nwell) 20 / 15".  A distance of 20 µm is 20 µm in any direction.

Two defects, found with one gap per layout so nothing else is in reach:

- **Diagonal.**  A tap placed d µm right and d µm up from the active's corner.  gdscheck
  is silent for d = 14.145 (euclidian 20.004), d = 16 (22.6) and d = 19 (26.9), and fires
  only when one axis alone passes the value: the reach is an axis-aligned box of ±20, not
  a circle.  In `DF.14.h1` the tap at (17.145, 97.145) and the one at (22, 182) are both
  out of reach of their source/drains and neither is reported; `DF.13.h2` has the same at
  (65, 23) in the well (44, 2)-(70, 28).
- **The bound.**  DF.13 carries the reach `within: nwell`; at exactly one grid step past
  the value it stays silent.  A P+ source/drain with its N+ tap 20.005 away in the same
  well is clean to gdscheck and fires at 20.01.  `DF.13.h1` at (2.5, 9)-(3.5, 11), and
  `DF.13.h2` at (18.5, 35)-(19.5, 37).  DF.14, which has no `within`, fires at 20.005.

- gdscheck @20/7/100: `DF.13.h1` DF.13_LV 0, DF.13_MV 1; `DF.13.h2` DF.13_LV 4 of 5;
  `DF.14.h1` DF.14_LV 1 of 3.
- KLayout: DF.13_LV 1, 6 and DF.14_LV 3 - it fires on every one of them.  Upstream
  approximates the reach with an octagon (DF.13) and a diamond (DF.14) plus a euclidian
  `sep`, so its reach is at most the true circle and never larger.
- Verdict: gdscheck's is larger than the manual's in the diagonal and smaller by one grid
  step along an axis when confined.  Both are false negatives; the diagonal one lets a
  source/drain 26.9 µm from its tap pass.

### 3. An enclosure under a 45° wall is not seen (false negative)

Manual: "DF.4b Min. DNWELL overlap of NCOMP well tap 0.62 / 0.66".  Settled reading
(SPEC, 2026-09-21): the enclosure of a shape lying inside another is the closest approach
from its boundary to the enclosing one, so a 45° wall passing under the value fires.

Layout `DF.4b.h1`: a deep well whose bottom-left corner is cut off along x + y = 21
(vertices (19, 2), (24, 2), (24, 8), (18, 8), (18, 3)) holding an N+ tap
(18.62, 2.62)-(20.62, 4.62).  The tap's straight margins are exactly 0.62; its bottom-left
corner is 0.17 from the chamfer.

- gdscheck @20/7/100: DF.4b_LV 1 - only the 0.615 rectangle at (10.615, 4); the chamfer is
  silent.
- KLayout: DF.4b_LV 3 - the 0.615 one plus two edge pairs on the chamfer,
  `(18.62,2.62;18.62,3.257)/(18.62,2.38;18,3)` and `(19.257,2.62;18.62,2.62)/(19,2;18.38,2.62)`.
- Verdict: gdscheck is wrong; 0.17 of deep well under the tap's corner is not 0.62 of
  overlap.

### 4. A hole with an island in it keeps the island's area (false negative)

Manual: "DF.10 Min. field area (um2) 0.26".  Field is what COMP does not cover.

Layout `DF.10.h1`: a P+ ring (2, 6)-(4, 8) with a 0.7 x 0.7 hole, and a 0.5 x 0.5 P+
island in the middle of that hole.  The field left is 0.49 - 0.25 = 0.24 µm².  (The island
stands 0.1 from the ring, which is the DF.3a violation the layout also carries.)

- gdscheck @20/7/100: DF.10 4 - the four empty holes; the hole round the island is read as
  0.49 µm² and passes.
- KLayout: DF.10 5, the fifth being the ring-with-island polygon
  `(2.65,6.65;...;3.35,6.65/2.75,6.75;3.25,6.75;3.25,7.25;2.75,7.25)` - upstream writes the
  rule `comp.holes.not(comp)`.
- Verdict: gdscheck is wrong; the island is COMP and cannot be field.

### 5. A gate running past the active's end leaves no source/drain (false negative, KLayout too)

Manual: "DF.6 Min. COMP extend beyond gate (it also means source/drain overhang)
0.24 / 0.4".

Layout `DF.6.h2`: an N+ active (2, 2)-(3.2, 3) crossed by a gate (2.9, 1.7)-(3.5, 3.3) -
the poly runs out over the active's right edge, so on that side the transistor has no
drain at all: an overhang of 0.

- gdscheck @20/7/100: DF.6_LV 4 - the three 0.235 overhangs and the shared-gate one;
  nothing on the right-hand end.
- KLayout: DF.6_LV 4, the same four (`enclosing` measures only where the COMP encloses the
  poly, and here it does not).
- Verdict: both are wrong by the manual.  Zero overhang is the worst case of the rule the
  manual states, and a transistor with one terminal missing is exactly what it forbids.

### 6. Butted MOSCAP is only seen when the marker lies on the N+ half (false negative, KLayout too)

Manual: "DF.3b Min./Max. NCOMP Space to PCOMP in the same well for butted COMP (MOSCAP
butting is not allowed) 0".

The deck's `df3b_moscap` is `ncomp_butted AND mos_cap_mk` where `ncomp_butted` is the *N+*
part of a butted pair, so a MOS_CAP_MK over the P+ half alone intersects nothing.

Layout `DF.3b.h1`, three butted actives (18, y)-(22, y+2) with the implant boundary on
x = 20: the marker over the whole of the first, over the P+ half of the second
(20.5, 5.5)-(22.5, 8.5), over the N+ half of the third (17.5, 9.5)-(19.5, 12.5).

- gdscheck @20/7/100: DF.3b 5 - the whole-marker and N+-half cases fire, the P+-half case
  does not.
- KLayout: DF.3b 5, the same five.
- Verdict: both are wrong; the manual forbids butting a MOSCAP whichever half of the pair
  carries the capacitor marker.

### 7. A COMP touching the DRC_BJT marker from outside is counted as being in the BJT area (false positive, KLayout too)

Manual: "DF.3c Min. COMP Space in BJT area (area marked by DRC_BJT layer) 0.32 / NA".
There is no 5 V value; the deck and upstream both read the 5 V case as "a DRC_BJT marker
interacting with two 5 V COMPs is forbidden".

Layout `DF.3c.h1`: a marker (1.5, 13)-(4, 15.5) holding one COMP (2, 13.5)-(3, 14.5), with
a second COMP (4, 13.5)-(5, 14.5) whose left wall lies on the marker's right edge.

- gdscheck @20/7/100: DF.3c_MV 3 - the intended pair plus this one.
- KLayout: DF.3c_MV 3, the same.
- Verdict: a COMP outside the marker is not in the BJT area; `interacting` counting a
  touch puts it there.  Minor, and the whole 5 V rule is upstream's invention.

### 8. DF.11 measures the active's width, not the butting edge (false positive, KLayout too)

Manual: "DF.11 Min. Length of butting COMP edge 0.3".  The butting edge is the N+/P+
boundary inside the COMP; its *length* is bounded.

The deck follows upstream: `comp INTERACTING ncomp_butted` under a 0.30 `min_width`, which
measures the active, not the boundary.

Layout `DF.11.h1`: (a) a 2 x 0.25 active with the boundary running along its length, so
the butting edge is 2.0 long; (b) a 1 µm plate with a 0.25 wide finger, the boundary
1.0 long; (c) the intended 0.295 butts across a bar and across x = 20.

- gdscheck @20/7/100: DF.11 8 - four bars, two markers each, (a) and (b) included.
- KLayout: DF.11 4 edge pairs, the same four bars.
- Verdict: (a) and (b) are legal by the manual - a 0.25 wide active is above DF.1a's 0.22
  and both butting edges are far over 0.3.  The case carries the manual's 4 (the two
  0.295 butts, two markers each).

### 9. One touch of SCHOTTKY_DIODE exempts a whole COMP from DF.12 (false negative, KLayout too)

Manual: "DF.12 COMP not covered by Nplus or Pplus is forbidden (except those COMP under
marking)".  The exception is for COMP *under* the marking.

The deck's `comp_no_schottky` is `comp NOT_INTERACTING schottky_diode`: a marker touching
any part of a COMP drops all of it.

Layout `DF.12.h1`: a bare COMP (11, 2)-(13, 3) with SCHOTTKY_DIODE over
(10.8, 1.8)-(12, 3.2) - half the active is neither implanted nor marked.

- gdscheck @20/7/100: DF.12 2; this active is silent.
- KLayout: DF.12 2, the same two.
- Verdict: both are wrong by the letter; the half outside the marking is a COMP with no
  implant.

### 10. DF.13 reaches into a deep well but confines the tap's reach to the drawn N-well (false positive, KLayout too)

Manual: "DF.13 Max distance of Nwell tap (NCOMP inside Nwell) from (PCOMP inside Nwell)".
Both shapes are named as being inside an *NWELL*.

The deck's `pactive` is `pcomp AND all_nwell`, and `all_nwell` is `dnwell_n OR nwell`, so
a P+ inside a deep well with no drawn N-well is a `pactive`; the reach, however, runs
`within: nwell` - the drawn layer - so no tap can ever reach it.

Layout `DF.13.h2`: a deep well (18, 2)-(30, 8) with a P+ source/drain (20, 4)-(22, 6) and
an N+ tap (25, 4)-(27, 6) 3 µm away, no drawn N-well.  `DF.3b.h1` carries the same shape.

- gdscheck @20/7/100: DF.13_LV on the pair, at (21, 5).
- KLayout: the same, polygon `(20,4;20,6;22,6;22,4)` - it sizes inside `nwell` too.
- Verdict: either the rule applies to a deep well, and then the tap 3 µm away in the same
  N-region answers for it, or it does not and the shape is out of scope.  Reporting the
  source/drain while refusing to see its tap is neither.  The cases read the manual's
  NWELL and expect nothing.

### 11. A tap abutting or crossing the N-well edge is no space at all (false negative)

Manual: "DF.17 Min. space from (Nwell Outside DNWELL) to (PCOMP outside Nwell and DNWELL)
0.12 / 0.16".

Layout `DF.17.h1`: an N-well (12, 2)-(15, 5) with a P+ tap (15, 2.5)-(17, 4.5) butted
against its right edge - the two touch along x = 15 and the space between them is 0.  A
separate probe (not committed) put the tap 1 µm *over* the well instead, so the derived
`pcomp_out_nw_dn` begins on the well's edge; gdscheck is silent there too.

- gdscheck @20/7/100: DF.17_LV 1 - only the 0.115 pair; the butted tap is silent.
- KLayout: DF.17_LV 2, the second `(15,2.5;15,4.5)/(15,4.62;15,2.38)`.
- Verdict: gdscheck is wrong for the abutting case - there is no space at all, which is
  less than 0.12 - and this looks like the coincident-edge skip of the IHP round reaching
  into a plain `min_space`.  The overlapping case is arguable (the coincident edge is the
  derivation's cut) and is not in a fixture.

## Tested and clean

Everything below was drawn, run at three tile sizes and agreed on by both tools; no need
to redo it.

- **The bound in both columns**, for every rule with two values: the value is clean and
  one grid step under (or over) it fires - DF.1a (0.22/0.215, 0.3/0.295), DF.1c
  (1.0/0.995), DF.2a (0.22/0.215, 0.3/0.295), DF.2b (100/100.005), DF.3a
  (0.28/0.275, 0.36/0.355), DF.3c_LV (0.32/0.315), DF.4a (0.12/0.115, 0.16/0.155), DF.4b
  (0.62/0.615, 0.66/0.655), DF.4c (0.43/0.425, 0.6/0.595), DF.4d (0.12/0.115,
  0.16/0.155), DF.4e (0.93/0.925, 1.1/1.095), DF.5 (0.12/0.115, 0.16/0.155), DF.6
  (0.24/0.235, 0.4/0.395), DF.7 (0.43/0.425, 0.6/0.595), DF.8 (0.43/0.425, 0.6/0.595),
  DF.9 (0.2025/0.2002), DF.10 (0.2601/0.25), DF.11 (0.3/0.295), DF.13_MV (15/15.005),
  DF.14 (20/20.005, 15/15.005), DF.16 (0.43/0.425, 0.6/0.595), DF.17 (0.12/0.115,
  0.16/0.155), DF.18 (2.5/2.495), DF.19 (3.2/3.195, 3.28/3.275).
- **The wrong column never fires**: a 0.25 wide bar, a 0.30 space and a 0.3 overhang are
  clean at 3.3 V and fire at 5 V; every rule pair was drawn this way.
- **What makes a COMP 5 V**: Dualgate over the whole of it, over half of it (DF.1a.h1
  x = 18, DF.17.h1), over one corner of a well or a deep well (DF.4a.h1 x = 26,
  DF.4c.h1 y = 10), and 12 µm of marker at the far end of a 300 µm shape, 280 µm and
  fourteen tiles from the geometry under test (DF.1a.h2, DF.3a.h2, DF.4a.h2, DF.16.h2) -
  the answer never depended on which tile the marker lay in.
- **The markers that take a shape out of a rule**: MOS_CAP_MK (DF.1c, DF.2b - including a
  marker that covers only part of an active, where the rest keeps its own width),
  OTP_MK (DF.3a, DF.9), DRC_BJT (DF.3c), RES_MK and MVSD (DF.6), YMTP_MK (DF.4d, DF.16),
  SRAMCORE (DF.4c, DF.8, DF.16), SCHOTTKY_DIODE (DF.12), MVSD (DF.1a_MV).  Each was drawn
  both where the marker holds and where it just fails to.
- **Euclidian space**: the corner-to-corner diagonals of DF.18 (1.765 each way = 2.496
  fires, 1.77 = 2.503 clean) and DF.19 (2.262 each way = 3.199 fires) are measured as the
  true distance by both tools.
- **The derivations**: an active drawn as two boxes is one active under its gate
  (DF.2a.h2); a union of two boxes makes the area (DF.9.h1); a ring drawn as four walls
  and the same ring as one keyhole polygon give the same hole (DF.10.h1); a notch open to
  the outside is not a hole (DF.10.h1); a gate with a notch in its side does not shorten
  the channel (DF.2a.h2); an N+ exactly coincident with the COMP covers it (DF.12.h1);
  two 120 µm plates joined by a 30 µm bar are two wide places, not one (DF.2b.h1).
- **The tile lines**: a channel edge on and across x = 20 (DF.2a.h2), an area straddling
  x = 20 and x = 40 (DF.9.h1), a hole on and across x = 20 and x = 40 (DF.10.h1), a
  butting edge on x = 20 (DF.11.h1), an implant edge on x = 20 (DF.12.h1), an implant
  overlap across x = 20 (DF.3b.h1), gate edges on x = 20, x = 21, x = 41 and x = 42
  (DF.6.h2), tap distances spanning two tile lines (DF.13.h2, DF.14.h2).  Not one count
  moved between tiles 20, 7 and 100.
- **Marker granularity**, not a finding (SPEC's settled readings): gdscheck gives two
  markers per narrow bar where KLayout gives one edge pair (DF.1a, DF.1c, DF.11), one per
  facing-wall pair where KLayout gives one per edge pair (DF.3a, DF.10.h1's island at 0.1
  from the ring: 1 against 4), and one per short enclosure edge where a 45° approach gives
  KLayout two (DF.18, DF.19).
