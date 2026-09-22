<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / lvpwell: hardening report

Deck `lvpwell` against the GF180MCU design manual, section 7.3 (LPW.1-LPW.5 inside
DNWELL, LPW.11-LPW.12 outside).  20 layouts,
`tests/data/gf180mcuD/generated/lvpwell/LPW.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/lvpwell.rs` (`hardening`), each with a `#[case]` in the
`hardening_lvpwell` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through the upstream KLayout runset (`hardening/oracle-gf180.sh`
with `DECKS=dnwell,lvpwell,nwell`; with `all` the runset aborts on any DNWELL layout -
see note B of the nwell report).

`show-deck` lists LPW.1, LPW.2a, LPW.2b (each LV and MV; 2b with a notch half), LPW.3,
LPW.5, LPW.11 and LPW.12.  LPW.4 is the manual's own "not coded" and Note.1 is a
guide; both absent on both sides.  Nothing is missing.

Reading the numbers: gdscheck's `min_width` reports one marker per wall, its space and
enclosure checks one per pair or side, its `forbidden` one per shape.  No count moved
with the tile size in any layout (the 30 µm well of `LPW.1.h4` and the straddling pairs
of `LPW.2.h6` included).

Test status on the engine as of this report: 9 of the 20 cases fail, all on the findings
below; the other 11 pass.

## Findings

### 1. LPW.3 has no crossing half (false negative)

Manual: "LPW.3  Min. DNWELL enclose LVPWELL  2.5".  A well that runs out of the deep
well is not enclosed by it.

Layouts: `LPW.3.h2`, a 2 × 2 well at (3, 3)-(5, 5) in a DNWELL ending at x = 4 that
holds it by 2.5 everywhere else; `LPW.1.h4`, a 0.595 × 30 well from x = 5 to 35 whose
last 5 µm lie in a DNWELL from x = 30.

- gdscheck @20/7/100: nothing for LPW.3 in either (LPW.1_LV fires on the long well as
  it should).  The check is `min_enclosure` with `interacting_only`, which has no
  margin to measure on the crossing side.
- KLayout: LPW.3 `(4,3;5,5)` and `(5,2;30,2.595)` - the part outside, from its
  `lpw3_l2 = lvpwell_dn.not(dnwell)`.
- Verdict: LPW.3 once in each.  The nwell deck gives NW.5 exactly this second half
  (`nw5_lv_crossing`, a `forbidden` on `nwell overlapping dnwell, minus dnwell`);
  LPW.3 needs the same.  Cases `lpw_3_h2`, `lpw_1_h4`.

### 2. LPW.3 reads the enclosure by projection and misses a chamfered corner (false negative)

Settled reading (2026-09-21): enclosure is the closest approach from the enclosed
boundary to the enclosing one.  Every IHP enclosure rule and this PDK's NW.5 carry
`metric: euclidian`; LPW.3 does not.

Layout `LPW.3.h1` (d): a 2 × 2 well at (3, 17) held by 2.5, the DNWELL's top-right
corner chamfered 1.5 each way, so the well's corner (5, 19) is 3.5/√2 = 2.475 from the
chamfer.  (e) beside it, chamfered 1.4 (2.546), is the control.

- gdscheck @20/7/100: nothing for (d); (b) 2.495 and (c) edge on edge fire.
- KLayout: LPW.3 for (d), two polygons at `(4.963,18.999;7.001,20.5)`, from its
  euclidian `enclosing`; nothing for (e).
- Verdict: LPW.3 three times.  Case `lpw_3_h1`.  A one-line deck change
  (`metric: euclidian`), if the settled reading holds for this PDK too.

### 3. LPW.11 is silent when the well abuts the deep well (false negative)

Manual: "LPW.11  Min (LVPWELL outside DNWELL) space to DNWELL  1.5".

Layouts: `LPW.11.h1` (c), a 2 × 3 well at x = 2-4 and a DNWELL from x = 4, sharing the
edge x = 4, y = 22-25; `LPW.11.h2`, a 0.595 × 3 well at x = 2-2.595 with a DNWELL from
x = 2.595.

- gdscheck @20/7/100: nothing for the touch in either; in h1, LPW.11 for (b) 1.495, (d)
  1.499 corner to corner and (f) the well in a DNWELL ring's hole.  In h2, LPW.1_LV
  twice instead - the touching well is "interacting" the deep well and so lands in
  `lvpwell_dn`, table A's layer.
- KLayout: LPW.11 for both touches (`(4,25;4,22)/(4,22;4,25)`,
  `(2.595,5;2.595,2)/(2.595,2;2.595,5)`) plus LPW.1_LV on h2 for the same reason, plus
  an LPW.3 sliver on each from its `enclosing` reading the touch as a zero enclosure.
- Verdict: LPW.11 four times in h1, once in h2.  The same class as finding 3 of the
  nwell report (NW.3) and IHP's NW.d: a space check has no pair at a touch.  Cases
  `lpw_11_h1`, `lpw_11_h2`.  The h2 case also says the 0.595 well is *not* LPW.1_LV: a
  well outside the deep well is table B's, and a well that touches the deep well from
  outside is outside it.  Both tools' "inside DNWELL" is `interacting`, which a touch
  satisfies; `overlapping` would not.  Whether the touch is also LPW.3 is ignored in the
  case.

### 4. A pair of mixed voltage is checked by nobody (false negative)

Manual: LPW.2a_MV 1.7 is the 5 V well's space to a well at another potential; a 5 V
well beside a 3.3 V well is still a 5 V well.

Layout `LPW.2a.h3`: a two-net pair 1.695 apart in a DNWELL with Dualgate over the left
well only, reaching 0.3 into the gap (x = 1.5-5.3); a control pair at 1.7 beside it.

- gdscheck @20/7/100: nothing.  The MV layer holds the left well alone and the LV layer
  the right one alone.
- KLayout: LPW.2a_MV `(4.999,1.999;6.696,5.001)` - it measures the whole `lvpwell_dn`
  and sorts the violation polygon by whether it overlaps Dualgate.
- Verdict: LPW.2a_MV.  Same as finding 2 of the nwell report.  Case `lpw_2a_h3`.

### 5. A well the Dualgate marker abuts is neither the 3.3 V nor the 5 V kind for the space rules (false negative)

Manual: the 5 V column applies where Dualgate lies over the shape; a marker sharing an
edge with the well does not cover it.

Layout `LPW.2a.h4`: a two-net pair 1.395 apart in a DNWELL with Dualgate abutting the
left well's left edge at x = 4 (marker x = 2-4).

- gdscheck @20/7/100: nothing.  `lvpwell_dn_lv` is "not interacting Dualgate", which a
  touch fails, and `lvpwell_dn_mv` is "overlapping Dualgate", which it also fails; the
  left well is on neither layer.
- KLayout: LPW.2a_LV `(6.999,1.999;8.396,5.001)` (it filters the violation polygon,
  which the marker does not touch).
- Verdict: LPW.2a_LV.  Same as finding 1 of the nwell report.  Case `lpw_2a_h4`.  Note
  the width rule is *not* affected here: LPW.1_LV runs on all of `lvpwell_dn`, so a
  0.7 well the marker abuts is LV and clean in gdscheck (`LPW.1.h1` (f)) - where
  KLayout, whose `lvpwell_dn_mv` is `interacting(dualgate)`, wrongly calls it MV and
  fires LPW.1_MV.

### 6. LPW.1_LV is also reported on a 5 V well (false positive, labelling, both tools)

Manual: a 5 V well's width is LPW.1_MV's 0.74; the 3.3 V column is not its.

Layout `LPW.1.h3`: a 0.595 × 3 well under Dualgate in a DNWELL.

- gdscheck @20/7/100: LPW.1_MV twice and LPW.1_LV twice, the same two walls.
- KLayout: the same (its LPW.1_LV runs on all of `lvpwell_dn`, and the deck copies
  that; the nwell deck's NW.1a_LV, by contrast, runs on `nw_lv` only).
- Verdict: LPW.1_MV twice.  Case `lpw_1_h3`.  Low: the same walls, one label too many;
  the fix is `layers: [lvpwell_dn_lv]` for LPW.1_LV, but see finding 5 - that layer
  drops a well the marker abuts, so the touching case has to be settled first.

### 7. A 5 V well resistor outside a deep well is not reported (false negative, both tools)

Manual: "LPW.5  LVPWELL resistors must be enclosed by DNWELL" - one column, no
exemption; "If LVPWELL is designed as a resistor, it is not allowed to be placed outside
DNWELL."

Layout `LPW.5.h2`: a 2 × 8 well with a P+ COMP head at each end and RES_MK between them
(the LPW.4 drawing, which fires in `LPW.5.h1` (a)), under Dualgate, no deep well.

- gdscheck @20/7/100: nothing (`lvpwell_res` subtracts `poly2_drawn ∪ dualgate ∪
  resistor ∪ sab`, so a marked well under Dualgate is no resistor).
- KLayout: nothing (the same `well_exclude`).
- Verdict: LPW.5.  Case `lpw_5_h2`.  Debatable: upstream's `well_exclude` may be
  there to keep an MV NMOS's body, which sits under Dualgate and might carry a RES_MK
  for some other device, from being read as a resistor; but a resistor with COMP heads,
  a marker and nothing else is a resistor whatever its voltage.

## Notes that are not findings

- A. Two nets 0.855 apart would be LPW.2a and LPW.2b in gdscheck (its LPW.2b is not
  net-gated) and LPW.2a alone in KLayout; not drawn, same class as the nwell report's
  note A.
- B. KLayout fires LPW.1_MV on a 0.7 well the Dualgate abuts (`LPW.1.h1` (f), edge pair
  `(22,2;22,5)|(22.7,5;22.7,2)`): its `lvpwell_dn_mv` is `interacting(dualgate)`.  The
  manual says 3.3 V, gdscheck says clean.

## Tested and found clean or correct (no need to redo)

- LPW.1: 0.595 bare fires, 0.6 clean; 0.735 under Dualgate fires, 0.74 clean, 0.735
  half under Dualgate fires; a 0.7 well the marker abuts is clean (`LPW.1.h1`); a
  0.595 well with no deep well near it, and one 5 µm from a deep well, are clean - table
  B has no width rule (`LPW.1.h2`); a 30 µm 0.595 well entering a DNWELL is narrow once
  at every tile size (`LPW.1.h4`).
- LPW.2a: 1.395 on two nets fires, 1.4 clean, 1.695/1.7 under Dualgate; a Via1 and
  Metal2 strap makes one net; a plate over the other well without a contact and an N+
  diffusion for a tap do not (`LPW.2a.h1`, `LPW.2a.h2`); gaps and straps straddling
  x = 20, 21, 40, 42 (`LPW.2.h6`); two bare wells in a DNWELL are two potentials
  (`LPW.2.h7`).
- LPW.2b: 0.855 on one net fires, 0.86 clean, a 0.855 slot fires (notch), 0.86 clean,
  LV and under Dualgate (`LPW.2b.h1`, `LPW.2b.h2`).
- Outside any deep well: two nets at 0.855, a 0.855 slot, two nets at 1.395 - no space
  rule, clean (`LPW.2.h5`).
- LPW.3: 2.5 clean, 2.495 fires, edge on edge fires, a 2.546 chamfer clean (`LPW.3.h1`).
- LPW.5: the LPW.4 drawing with no deep well fires; in a deep well, clean; a marker
  over a well with no COMP is not a resistor (`LPW.5.h1`).
- LPW.11: 1.5 clean, 1.495 fires, 1.499 corner to corner fires and 1.506 clean, a well in
  a DNWELL ring's hole 1.495 from the wall fires (`LPW.11.h1`).
- LPW.12: 0.005 overlap fires, abutting is clean, an overlap inside a deep well is NW.4's
  and not LPW.12 (`LPW.12.h1`).

## Resolution (2026-09-22)

Judged against the manual's text and the upstream runset; every case now passes.

- **1 (LPW.3 crossing)**: fixed in the deck.  LPW.3 gets the second half NW.5 already
  had: `forbidden` on `lpw3_crossing` = the part of a well overlapping the deep well
  that lies outside it.
- **2 (LPW.3 by projection)**: fixed in the deck, `metric: euclidian` - the settled
  reading for every enclosure since 2026-09-21.
- **3 (LPW.11 at a touch, and the touching well's table)**: fixed in the deck.
  `abutting: report` on LPW.11, and `lvpwell_dn` is now `overlapping [lvpwell, dnwell]`
  rather than `interacting`: a well that only abuts the deep well is not inside it, so
  it is table B's and not table A's.
- **4 (mixed-voltage pair)**: fixed in the deck, a second LPW.2a_MV entry on the layer
  pair `[lvpwell_dn_mv, lvpwell_dn_lv]`.
- **5 (Dualgate abutting)**: fixed in the deck; `lvpwell_dn_lv` is
  `not_overlapping [lvpwell_dn, dualgate]`, complementary to `lvpwell_dn_mv`, and the
  `v5_xtor` step is gone.  Same as the nwell report's finding 1.
- **6 (LPW.1_LV on a 5 V well)**: fixed in the deck; LPW.1_LV runs on
  `lvpwell_dn_lv`, which finding 5 made complete.
- **7 (5 V resistor outside the deep well)**: fixed in the deck.  Dualgate is off the
  `lpw_well_exclude` list: the manual has one column for LPW.5 and no exemption, and a
  well with a RES_MK and a COMP head at each end is a resistor whatever its voltage.

Foundry case `lvpwell.gds.gz` after the changes: LPW.11 14 -> 15 (the touch), LPW.1_LV
21 -> 13 (only the 3.3 V wells), LPW.3 71 -> 90 (the euclidian metric and the crossing
half); every other count unchanged.  The reference design's findings are unchanged.
Suite green at tiles 20, 7 and 100.
