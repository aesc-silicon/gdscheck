<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / nwell: hardening report

Deck `nwell` against the GF180MCU design manual, section 7.4 (NW.1a-NW.7).  23 layouts,
`tests/data/gf180mcuD/generated/nwell/NW.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/nwell.rs` (`hardening`), each with a `#[case]` in the `hardening_nwell`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through the upstream KLayout runset (`hardening/oracle-gf180.sh`, with
`DECKS=dnwell,lvpwell,nwell` - see the note at the end on why not `all`).

`show-deck` lists every rule of the section that is a DRC check: NW.1a, NW.1b, NW.2a,
NW.2b (each LV and MV), NW.3, NW.4, NW.5 (LV and MV), NW.6.  NW.7 is the manual's own
"not coded" (LVS marker) and is absent on both sides.  Nothing is missing.

Reading the numbers: gdscheck's `min_width` reports one marker per wall (two per narrow
bar), its space and enclosure checks one per pair or side, its `forbidden` one per
shape; KLayout's column is one run of the runset.  No count moved with the tile size in
any layout (the 30 µm well of `NW.1a.h3` and the straddling pairs of `NW.2b.h7`
included).

Test status on the engine as of this report: 14 of the 23 cases fail, all on the
findings below; the other 9 pass.

## Findings

### 1. A well the Dualgate marker abuts is neither the 3.3 V nor the 5 V kind (false negative)

Manual: the 5 V/6 V column applies where the Dualgate marker lies over the shape
(section 7.6, DV.4: "circuits covered by Dualgate layer will have 5V/6V gate oxide");
everything else is the 3.3 V column.  A marker beside a well, sharing an edge with it,
does not cover it.

Layouts: `NW.1a.h1` (d), a 0.855 × 3 well at x = 14-14.855 with Dualgate from x = 14.855
on; `NW.1a.h3`, the same with a 30 µm well and the marker abutting its end at x = 35;
`NW.1b.h2` (b), a 1.995 resistor well at x = 8-9.995 with the marker from x = 9.995;
`NW.2b.h4`, two nets 1.395 apart with the marker abutting the left well's left edge at
x = 4; `NW.5.h4`, a well held by 0.495 with the marker abutting its left edge at x = 4.

- gdscheck @20/7/100: nothing for any of them (the neighbouring wells' markers fire as
  expected).
- KLayout: nothing for the width cases either (its `nw_lv` is `not_interacting(dualgate)`
  and its `nw_mv` is `overlapping(dualgate)`, so a touching marker excludes the well
  from both); NW.2b_LV for `NW.2b.h4` (its space rules filter the *violation*
  polygon, `(6.999,1.999;8.396,5.001)`, which the marker at x ≤ 4 does not touch).
- Verdict: all five are the 3.3 V rule - NW.1a_LV twice in `NW.1a.h1` (walls at x = 14
  and 14.855), NW.1a_LV twice in `NW.1a.h3`, NW.1b_LV twice in `NW.1b.h2`, NW.2b_LV in
  `NW.2b.h4`, NW.5_LV in `NW.5.h4`.  The deck's LV layer is "not interacting Dualgate"
  where the manual's is "not covered by Dualgate": a well the marker only touches falls
  through both layers.  Cases `nw_1a_h1`, `nw_1a_h3`, `nw_1b_h2`, `nw_2b_h4`,
  `nw_5_h4`.

### 2. A pair of mixed voltage is checked by nobody (false negative)

Manual: NW.2a_MV 0.74 and NW.2b_MV 1.7 are the 5 V well's spaces; a 5 V well next to a
3.3 V well is still a 5 V well.

Layouts: `NW.2a.h2`, a one-net pair 0.7 apart at y = 10 with Dualgate over the left well
only, reaching 0.3 into the gap (x = 1.5-5.3); `NW.2b.h3`, a two-net pair 1.695 apart
with the same marker, and a control pair at 1.7.

- gdscheck @20/7/100: nothing for either mixed pair (the plain MV pairs in the same
  layouts fire).  The MV layer holds the left well alone and the LV layer the right one
  alone, and a space check on a layer with one shape has nothing to measure.
- KLayout: NW.2a_MV `(4.999,9.999;5.701,13.001)` and NW.2b_MV `(4.999,1.999;6.696,5.001)`
  - it measures on the whole `nwell` layer and sorts the violation polygon by whether it
  overlaps Dualgate, which the marker's 0.3 reach into the gap makes true.
- Verdict: NW.2a_MV in `NW.2a.h2` (expected 2 with the bound pair), NW.2b_MV in
  `NW.2b.h3`.  The stricter of the two wells' values applies to a gap between them.
  Cases `nw_2a_h2`, `nw_2b_h3`.  Note that KLayout's reading hinges on the marker
  reaching into the gap; a marker ending on the well's edge (which DV.6's 0.24
  enclosure of COMP makes unlikely for a real device) would leave it silent too.

### 3. NW.3 is silent when the well abuts the deep well (false negative)

Manual: "NW.3  Min. Nwell to DNWELL space  3.1".

Layout `NW.3.h1` (c): a 3 × 3 well at x = 2-5 and a DNWELL from x = 5, sharing the edge
x = 5, y = 22-25.  Beside it (b) at 3.095, (d) 3.097 corner to corner, (f) a well in a
DNWELL ring's hole 3.095 from the wall, all of which fire, and (a) 3.1 and (e) 3.111,
clean.

- gdscheck @20/7/100: NW.3 three times - (b), (d), (f); nothing for (c).
- KLayout: NW.3 for (c), edge pair `(5,25;5,22)/(5,22;5,25)`, and for the others (eight
  edge pairs); also two slivers of NW.5_LV at the touching corners, which are its
  `enclosed` check reading the touch as an enclosure of zero and are noise.
- Verdict: a space of nothing is under 3.1; NW.3 four times.  The same class as IHP's
  NW.d finding (a space check has no pair at a touch).  Case `nw_3_h1`.  Whether the
  touch should be NW.3 or NW.5 is a labelling question; it is not NW.5, since the well
  is not inside the deep well, and it must be something.

### 4. A resistor drawn as NW.7 says is not a resistor to NW.6 (false negative)

Manual: "NW.6  Nwell resistors can only exist outside DNWELL"; NW.7: "RES_MK length
shall be coincide with resistor length (Touching COMP each side) and width covering the
width of Nwell".  So the marker of a well resistor runs from one COMP head to the other
and the well extends past it at both ends.

Layout `NW.6.h2`: a 3 × 8 well at (2, 2)-(5, 10) in a DNWELL holding it by 1.0, an N+
COMP head at each end (y = 2.2-2.8 and 9.2-9.8), RES_MK (1.7, 3.0)-(5.3, 9.0).
`NW.6.h1` beside it has the marker overhanging the well on every side, which fires.

- gdscheck @20/7/100: nothing (`nwell_res_in_dnwell` is `nwell inside res_mk`, and this
  well is not inside its marker).
- KLayout: nothing, for the same reason (`nwell.inside(res_mk).and(dnwell)`).
- Verdict: NW.6.  The same deck recognises a resistor for NW.1b by the marker reaching
  beyond the well (`res_mk interacting nwell, not inside nwell`) and then measures
  `nwell ∩ res_mk`; NW.6 should read the same way (`nwell ∩ nw_res_mk ∩ dnwell`).  With
  the current derivation NW.6 only fires on a marker that overhangs the well at both
  ends, which NW.7 says not to draw.  Case `nw_6_h2`.

### 5. A 3.3 V well crossing the deep well's edge is also reported as the 5 V rule (false positive, labelling)

Manual: NW.5 has one value for both columns; a 3.3 V well is under NW.5_LV.

Layout `NW.5.h2`: a 3 × 3 well at x = 2-5, no Dualgate anywhere, in a DNWELL ending at
x = 3.5.  `NW.5.h3` (b) is the 5 V twin under Dualgate.

- gdscheck @20/7/100: NW.5_LV and NW.5_MV, one each, on the same 1.5 × 3 piece.
- KLayout: the same two.  Its `nw5_l2` for the MV rule takes plain `nwell` where the LV
  one takes `nw_lv`; the deck copies that ("that asymmetry is upstream's").
- Verdict: NW.5_LV once.  Case `nw_5_h2`.  Low: the same piece, one label too many.

### 6. Two wells in one deep well are not two potentials (false positive, both tools)

Manual, section 7.4: "by default all Nwell inside DNWELL will be shorted together
through DNWELL"; NW.2a and NW.2b are headed "Outside DNWELL".

Layouts: `NW.2b.h5`, two 3 × 3 wells 1.395 apart with a tap and a Metal1 plate each,
in a DNWELL holding them by 1.0; `NW.2b.h6`, the same two wells with no tap of their
own and the DNWELL tapped once beside them.

- gdscheck @20/7/100: nothing for h5 (the taps join each well to the deep well's net,
  and the deep well joins the two); NW.2b_LV for h6, `(5,2)-(6.395,2)`.
- KLayout: the same on both.
- Verdict: clean both.  The deep well is what shorts the wells, not their taps; a well
  with no tap of its own inside a deep well is on the deep well's net.  The
  connectivity ties `nwell` to `dnwell` only through an N+ diffusion (`ntap_dn`), never
  by touching.  Case `nw_2b_h6`.  Low: a real PMOS body always has a tap, so h5 is the
  case that occurs, and it passes.

### 7. NW.2a's YMTP exemption reads the wells where the runset reads the gap (false positive, debatable)

Manual (section 10.13): YMTP_MK marks the YMTP cell, in which the Nwell space rule is
Y.NW.2b at 1.0 "(Outside DNWELL, Inside YMTP_MK)".  The runset drops NW.2a's violation
polygons that lie inside the marker; the deck drops *wells* inside the marker.

Layout `NW.2a.h3`: a one-net pair 0.595 apart with the marker over both wells and the
gap (y = 2), and a second with the marker over the gap only, reaching 0.5 into each well
but not over them (y = 10, marker x = 4.5-6.1).

- gdscheck @20/7/100: NW.2a_LV for the second pair, `(5,10)-(5.595,10)`; nothing for the
  first.  (Y.NW.2b_LV fires on both from the `ymtp_mk` deck, as it should.)
- KLayout: nothing for either (the violation polygon `(4.999,9.999;5.596,13.001)` is
  inside the marker).
- Verdict: clean, and low.  The Y rule takes over inside the marker (its layer is
  `nwell ∩ ymtp_mk`, so it does measure this gap), and the deck's own comment says to
  revisit this if a YMTP design disagrees.  Case `nw_2a_h3`.  A marker that covers a
  gap and not the wells is not how a cell marker is drawn, so this may never matter.

### 8. NW.1b measures the marked region's shorter side, not the well's width (false positive, both tools, debatable)

Manual: "NW.1b  Min. Nwell Width as a resistor  2"; NW.7: "Width of the resistor
determined by Nwell width. Length by COMP to COMP space."

Layout `NW.1b.h1` (e): a 3 × 4 well at (26, 2)-(29, 6) with heads at y = 2.5 and 5.5
and the marker between them, (25.7, 3.0)-(29.3, 4.5) - a resistor 3.0 wide and 1.5
long.

- gdscheck @20/7/100: NW.1b_LV twice, "width 1.5000" on the walls y = 3 and y = 4.5.
- KLayout: NW.1b_LV once, the same pair of walls.
- Verdict: clean by the letter - the resistor's width is the well's, 3.0, and 1.5 is
  its length.  Both tools measure `nwell ∩ res_mk` as a bare shape.  Case `nw_1b_h1`
  (expected 4: (a) and (c)).  Debatable in that a 1.5 µm well resistor is an odd
  device; if the owner keeps the tools' reading the case should be flipped to 6.

### 9. A well under V5_XTOR alone is checked by nobody (false negative, debatable)

Manual, section 4.1: "V5_XTOR is used as a marking layer to define 5V devices";
section 7.4 never mentions it, and its 5 V column is Dualgate's (DV.4).

Layout `NW.1a.h2`: a 0.855 well under V5_XTOR with no Dualgate, and one under both.

- gdscheck @20/7/100: NW.1a_MV twice for the second; nothing for the first (`nw_lv`
  is "not interacting V5_XTOR", `nw_mv` is "overlapping Dualgate").
- KLayout: the same (the deck copies its `nw_lv`).
- Verdict: NW.1a_LV twice for the first - a well no Dualgate covers is a 3.3 V well
  whatever other marker lies on it; the value is 0.86 in both columns anyway, so the
  exclusion buys nothing and loses a check.  Case `nw_1a_h2`.  Low: a 5 V device
  without Dualgate is a marking error other decks catch.

## Notes that are not findings

- A. Two nets 0.595 apart (`NW.2b.h1` (c)) are NW.2b_LV and NW.2a_LV in gdscheck (its
  NW.2a is not net-gated) and NW.2b_LV alone in KLayout (`props_eq`).  The gap is
  reported either way; the case ignores NW.2a_LV.  Same class as IHP's settled NW.b /
  NW.b1 reading.
- B. The upstream runset cannot be run with `DECKS=all` on a layout that holds a DNWELL:
  it aborts at DN.2a with "'nets': The given layer is not an original layer used in
  netlist extraction" (klayout 0.30.9 in the container), and every deck after `dnwell`
  alphabetically - `lvpwell` and `nwell` among them - is lost.  With
  `DECKS=dnwell,lvpwell,nwell` it runs.  The KLayout column of every layout here comes
  from that selection; the density noise is gone with it.

## Tested and found clean or correct (no need to redo)

- NW.1a: the bound (0.855 fires, 0.86 clean) on a bare well, under Dualgate, and half
  under Dualgate (MV); a 30 µm well with Dualgate over its far end is MV once at every
  tile size (`NW.1a.h1`, `NW.1a.h3`).
- NW.1b: the marker overhanging on every side, and NW.7's head-to-head marker (fire at
  1.995, clean at 2.0); a marker wholly inside the well is not a resistor; the 5 V
  resistor under Dualgate (`NW.1b.h1`, `NW.1b.h2`).
- NW.2a: the bound at 0.595/0.6 and 0.735/0.74 on one net, a 0.595 slot (notch) and a
  0.6 one; the YMTP marker over both wells (`NW.2a.h1`, `NW.2a.h2`, `NW.2a.h3`).
- NW.2b: the bound at 1.395/1.4 and 1.695/1.7 on two nets; what connects - a Via1 and
  Metal2 strap between the plates (one net) - and what does not - a plate over the other
  well without a contact, a tap without a contact, a P+ diffusion for a tap
  (`NW.2b.h1`, `NW.2b.h2`); two tapped wells in one DNWELL are one net (`NW.2b.h5`);
  gaps and straps straddling x = 20, 21, 40, 42 (`NW.2b.h7`).
- NW.3: 3.1 clean, 3.095 fires, corner to corner 3.097 fires and 3.111 clean, a well in
  a DNWELL ring's hole (`NW.3.h1`).
- NW.4: 0.005 overlap fires, abutting is clean, overlap inside a DNWELL fires
  (`NW.4.h1`).
- NW.5: 0.5 clean, 0.495 fires, edge on edge fires, a chamfered DNWELL corner 0.495 from
  the well's corner fires and 0.502 is clean (euclidian, as settled); the MV twins under
  Dualgate, crossing included (`NW.5.h1`, `NW.5.h3`).
- NW.6: the marker overhanging on every side in a DNWELL fires; no marker, clean
  (`NW.6.h1`).

## Resolution (2026-09-22)

Judged against the manual's text and the upstream runset; the engine and the deck were
changed where the drawing was right, and every case now passes.

- **1 (Dualgate abutting) and 9 (V5_XTOR alone)**: fixed in the deck.  The 5 V column is
  Dualgate's (DV.4), so `nw_lv` is now `not_overlapping [nwell, dualgate]` and `nw_mv`
  stays `overlapping`: the two classes are complementary, and no well falls through
  both.  Upstream's extra `not_interacting v5_xtor` step is gone with it - a well marked
  V5_XTOR and not covered by Dualgate is a 3.3 V well, and excluding it bought nothing
  but a silent rule.
- **2 (mixed-voltage pair)**: fixed in the deck.  NW.2a_MV and NW.2b_MV each get a
  second entry on the layer pair `[nw_mv, nw_lv]`, which is exactly the gap between a
  5 V well and a 3.3 V one; the stricter value applies to it.
- **3 (NW.3 at a touch)**: fixed in the deck, `abutting: report` - the settled reading
  since IHP's NW.d, a shared edge is a space of nothing.
- **4 (NW.6 resistor)**: fixed in the deck.  `nwell_in_res_mk` is now `nwell ∩
  nw_res_mk`, the same resistor NW.1b recognises (NW.7's marker, from head to head,
  the well running past it), in place of `nwell inside res_mk`.
- **5 (NW.5_MV on an LV well)**: fixed in the deck; the crossing half reads `nw_mv`
  where it read every well.  The foundry case's NW.5_MV drops 25 -> 18 with it.
- **6 (two wells in one deep well)**: fixed in the deck.  NW.2b now runs on
  `nw_lv_out_dnwell` / `nw_mv_out_dnwell`: the manual heads both spacing rules "Outside
  DNWELL", and wells inside a deep well are shorted through it, so a gap between two of
  them is never one of different potential.  The layers were added to the `ntap`
  connector's list, since a region on a layer outside the connectivity graph resolves
  to no net and a `different`-net rule then reports every pair.
- **7 (YMTP marker over the gap)**: kept, and the case flipped.  The base rule keeps a
  pair whose wells are not inside the marker; the Y rule measures the same gap from its
  own layer (`nwell ∩ ymtp_mk`), and a cell marker that covers a gap but not the cell's
  wells is not how YMTP_MK is drawn.  The deck's comment on the exemption stays.
- **8 (NW.1b on the marked region's short side)**: kept, and the case flipped to 6.
  Both tools measure the marked region as drawn, and a 1.5 µm-long well resistor is not
  a device; reading the well's own width through the marker would need the rule to
  measure one layer and report inside another.

Foundry case `nwell.gds.gz` after the changes: NW.3 14 -> 15 (the touch), NW.5_MV
25 -> 18 (the LV wells no longer labelled MV); every other count unchanged.  The
reference design's findings are unchanged.  Suite green at tiles 20, 7 and 100.
