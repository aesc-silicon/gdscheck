<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / nplus: hardening report

Deck `nplus` against the GF180MCU design manual, section 7.8 (NP.1-NP.12).  17 layouts,
`tests/data/gf180mcuD/generated/nplus/NP.*.h<n>.gds.gz`, drawn by `gen/gf180mcuD/nplus.rs`
(`hardening`), each with a `#[case]` in the `hardening_nplus` table of
`tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through the upstream KLayout runset (`hardening/oracle-gf180.sh`, `DECKS=nplus,pplus`;
several layouts carry a DNWELL, on which the full runset aborts).

`show-deck` lists every rule of the section: NP.1, NP.2, NP.3a, NP.3bi, NP.3bii, NP.3ci,
NP.3cii, NP.3d, NP.3e, NP.4a, NP.4b, NP.5a, NP.5b, NP.5ci, NP.5cii, NP.5di, NP.5dii,
NP.6, NP.7, NP.8a, NP.8b, NP.9, NP.10, NP.11, NP.12.  The manual's two-value rules are
split the way upstream splits them (NP.3b/c and NP.5c/d into `i`/`ii`), and the values
match the manual in every case.  Nothing is missing.

Reading the numbers: gdscheck reports one marker per measured pair of walls, KLayout one
per edge pair; no count moved with the tile size in any layout, including the notch and
gap drawn across x = 20/21/40 (`NP.2.h1`), the classifier band that falls at x = 20.429
(`NP.3ci.h2`) and the butted edge at x = 20.2 (`NP.11.h1`).

Test status on the engine as of this report: 3 of the 17 cases fail, on findings 1, 2
and 3; the other 14 pass.

## Findings

### 1. A shared edge is a space of nothing, and NP.7 does not report it (false negative)

Manual, NP.7: "Space to unrelated unsalicided Poly2 - 0.18".  A marker wall drawn on the
wall of an unsalicided poly is at a space of zero, which is the settled reading for this
process ("a shared edge is a space of nothing and is reported").

Layout: `NP.10.h1` (c) - a poly bar at x 20..21, y 10..11 under a salicide block
(x 19.9..21.1, y 9.9..11.1), with the N+ marker at x 18.6..20.0, y 9.8..11.2.  The
marker's right wall and the unsalicided poly's left wall are the same line, x = 20.

- gdscheck @20/7/100: nothing.  Neither NP.7 (the space) nor NP.9 (the overlap) fires -
  the two shapes touch but do not overlap, so `min_space` sees a distance of zero and
  says nothing, and `np_9_crossing` needs an area overlap.
- KLayout: `NP.7`, `edge-pair: (20,11.18;20,9.82)/(20,10;20,11)` - the marker's wall
  against the poly's wall, at zero.
- Verdict: KLayout is right; NP.7 must fire.  The same shape one step further left
  (0.175) does fire on both sides, so this is the zero-distance case alone.  Case
  `np_10_h1`.  The mirror is `PP.10.h1` (c) in the `pplus` report.

### 2. NP.10 has no "crossing" half: a marker wall that cuts an unsalicided COMP is silent (false negative)

Manual, NP.10: "Overlap of unsalicided COMP - 0.18".  An unsalicided COMP the marker
covers only in part is overlapped by nothing on the other side.

Layout: `NP.10.h1` (b) - a COMP at x 15..16, y 10..11 under a block at x 14.9..16.1,
with the marker at x 14.6..15.5: it covers the left half of the unsalicided COMP and
stops inside it.

- gdscheck @20/7/100: `NP.5b` only (the N+ active's own right edge, extension zero,
  `(15.5,10.5)`).  No NP.10.
- KLayout: the same - `NP.5b` and no `NP.10`.
- Verdict: both should report NP.10.  The deck gives NP.9 a `forbidden np_9_crossing`
  layer for exactly this case on unsalicided poly (`(poly_sab_only overlapping nplus)
  less nplus`) and NP.10 has no such companion; upstream has the same asymmetry
  (`np9_l2` exists, no `np10_l2`).  The manual's two sentences are the same sentence on
  two layers.  Case `np_10_h1`.  The mirror is `PP.10.h1` (b).

### 3. The butted-pair exemption is whole-marker, so an unrelated P+ active goes unmeasured (false negative)

Manual, NP.3a: "Space to PCOMP for PCOMP: (1) Inside Nwell (2) Outside LVPWELL but
inside DNWELL - 0.16".  The manual's zero-space exemptions are NP.3d ("Min/max space to
a butted PCOMP") and NP.3e ("Space to *related* PCOMP edge adjacent to a butting edge") -
both about the butting pair itself.

Layout: `NP.3a.h1` - one L-shaped marker (foot x 10..12, y 10..11; arm x 10..20,
y 11..12).  In the foot, a COMP at x 10.3..13.5 is N+ to x = 12 and P+ beyond it: a
legal butted pair.  At the far end of the arm, 8 um away, an unrelated P+ active sits in
a deep well at x 20.155..21.155 - 0.155 from the marker's wall.

- gdscheck @20/7/100: clean.
- KLayout: clean (`DECKS=nplus,pplus`; only `PP.5b` fires, on the far active's own
  implant).
- Control `NP.3a.h2`, the identical scene with the butted pair deleted: both report
  `NP.3a` at `(20,11)-(20.155,11)`.
- Verdict: NP.3a should fire in `h1` too.  The deck's `np3_nplus` is `nplus
  not_interacting ncomp_butted`, which takes the whole marker polygon out of NP.3a/3b/3c
  as soon as it touches any butted N+/P+ pair anywhere; upstream's `np3_nplus =
  nplus.not_interacting(ncomp_butted)` is the same.  On a real cell a single butted
  substrate tie would switch the spacing rule off for every implant island connected to
  it.  Case `np_3a_h1`.  The mirror is `PP.3a.h1`.

### 4. NP.9 applies to a marked poly resistor; PP.9 does not (inconsistency)

Manual, NP.9 and PP.9 carry the same sentence, "Overlap of unsalicided Poly2 - 0.18",
and neither section mentions a resistor.

Layout: `NP.9.h1` - two poly bars under salicide blocks with the marker 0.175 past the
left wall of each; the first (x 10..12) also lies under RES_MK (layer 62/0), the second
(x 20..22) does not.  `PP.9.h1` in the sibling deck is the same drawing on the P+ side.

- gdscheck @20/7/100: `NP.9` twice - the marked bar and the bare one.  On the P+ side,
  `PP.9` once: the marked bar is exempt (`poly_sab_no_res = (poly2 not_interacting
  resistor) and sab`).
- KLayout: identical, and for the same reason - `np9_l1` reads `poly2_drawn.and(sab)`
  while `pp9_l1` reads `poly2_drawn.not_interacting(resistor).and(sab)`.
- Verdict: the exemption is the right reading and NP.9 is missing it.  A poly resistor's
  body is meant to be unimplanted under the salicide block and its heads implanted, so
  the implant edge that crosses it is the device's dimension, not this rule's - which is
  the argument `pdks/gf180mcuD/pdk.yml` already makes for PP.9.  Whatever is decided, the
  two rules should not differ.  The expected value of case `np_9_h1` is one NP.9 (the
  bare bar), so it fails at present; `pp_9_h1` passes.

## Settled readings, recorded so the next round does not redo them

- **NP.11 and PP.11 are complementary halves of one sentence.**  The manual forbids a
  butting N+/PCOMP edge "within 0.43um of Nwell edge (for outside DNWELL) and of LVPWELL
  edge (for inside DNWELL case)"; NP.11's layers take the 0.429 collar *outside* the
  N-well and the band *inside* the P-well, PP.11's the other two.  A butting edge is the
  same line on both implants, so the pair together covers both sides of both walls.
  `NP.11.h1` draws butting edges 0.2 outside an N-well wall, 0.2 inside it and 0.5
  outside it; gdscheck and KLayout both report `NP.11` for the first and `PP.11` for the
  second, and nothing for the third.  Not a finding.
- **NP.4a reads projection.**  `NP.4a.h1` puts a butting edge round the corner from a
  P-channel gate: horizontal at y = 12.7, x 11.2..12.28, with the gate's left wall at
  x = 12.5, y 11.5..12.5 - 0.297 corner to corner, and no wall of the gate facing it.
  Both tools are silent, which is what "at a butting edge parallel to gate" asks for.
  `NP.4a.h2` faces the same edge at the gate's wall: 0.315 fires on both sides, 0.32 is
  clean.
- **NP.12's reach follows the poly.**  `NP.12.h1` (a) is a U of poly whose left leg
  carries the gate and whose right leg is 0.22 of air from it but ~5 um along the
  conductor; the marker covers the right leg and reaches inside the plain 0.32 disc round
  the gate.  Both tools are silent.  (b) is a straight bar with the marker 0.315 up it,
  one step inside the 3 x 0.10633 = 0.319 reach: both report NP.12.
- **NP.5a's `forbidden np_5a_crossing` layer cannot fire.**  `ngate` is
  `nactive and tgate`, `nactive` is `ncomp less all_nwell` and `ncomp` is `comp and
  nplus`, so a gate is inside the marker by construction and
  `(ngate overlapping nplus) less nplus` is always empty; upstream's `np5a_l2` is the
  same shape and equally unreachable.  It is not a wrong answer - `NP.5a.h1` (b), a gate
  whose poly bar the marker's wall cuts, is caught by the enclosure at zero (three
  NP.5a markers in both tools) - but the rule tests nothing and could be dropped.
- **The 0.429 classifier is read per region, not per shape.**  `NP.3ci.h2` runs a 2 um
  P+ active out of an N-well that ends at x = 20, so the active straddles the collar's
  edge at x = 20.429, with the marker 0.075 above it all along.  Both tools report
  NP.3ci on the near half (0.075 < 0.16) and NP.3cii on the far half (0.075 < 0.08).
- **The NP.2 notch is counted once.**  The deck runs `min_space` and `min_notch` on
  `nplus`; `NP.2.h1` has a 0.395 notch (across x = 20), a 0.4 notch (across x = 40) and a
  0.395 gap (across x = 21), and both tools report exactly two NP.2.

## Tested and clean

- NP.1/NP.2/NP.8a/NP.8b at the bound: `NP.2.h1`, `NP.8b.h1` (holes of 0.35 exactly,
  0.3465, and 0.348 built from four merging boxes across x = 20; two NP.8b in both tools).
- NP.3bi/bii at the P-well band: `NP.3bi.h1`, four scenes - in the band at 0.155 (fires)
  and 0.16, in the core at 0.075 (fires) and 0.08.
- NP.3ci/cii at the N-well collar: `NP.3ci.h1`, same four.
- NP.5b against NP.5dii: `NP.5b.h1`, two identical taps 0.1 past the COMP - in the field
  NP.5b wants 0.16 and fires, 2 um inside an N-well NP.5dii wants 0.02 and is clean.
- NP.5di against NP.5dii at the 0.429 band: `NP.5b.h2`, four taps in one well at 0.015
  (fires, core), 0.02, 0.155 (fires, band) and 0.16.
- NP.6 at the bound, with the three walls the P+ active shares with the COMP: `NP.6.h1`,
  an N+ half 0.215 long (fires) and one 0.22 long.  The shared walls are an overlap of
  nothing and neither tool counts them.
- NP.10 at the bound: `NP.10.h1` (a), 0.175 past an unsalicided COMP.
