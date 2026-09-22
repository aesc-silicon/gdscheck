<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / antenna: hardening report

Deck `antenna` against section 8.0 of the GF180MCU design manual, "Antenna Ratio Rules".
The section is one table of limits - ANT.1 Poly2 200, ANT.2..ANT.7 the metals 400,
ANT.8 contact 10, ANT.9..ANT.13 the vias 20, ANT.14 and ANT.15 the same for a MIM cap -
plus ANT.16, which is not a limit but the diode relief that rewrites the denominator of
all of them.  The deck's 24 rules are ANT.1, ANT.8 and then ANT.16 applied to each metal
and via level three ways: `_i_` against a thin gate (multiplying factor 2), `_ii_`
against a thick one (factor 15), `_iii_` against a MIM-B top plate (factor 15).

Two ratios are measured.  A metal or Poly2 conductor is scored on the manual's "perimeter
area", `2[(t·z) + (t·y)]` = perimeter × thickness, with the thicknesses from the table at
the foot of section 8.0 (Poly2 0.2, Metal1..Metal5 0.54, MetalTop 0.69/0.99/1.19/3.035 µm
by option).  A contact or via is scored on plan area.  The denominator is the gate oxide
area on the node, or the FuseTop area for a MIM.

What the deck does *not* list is right: ANT.7 (MetalTop) and ANT.13 (Via5) need a sixth
level this stack has not got, and ANT.14/ANT.15 for MIM option A need a cap between
Metal2 and Metal3, which variant D (`mim_option: B`) has not.  Variant D's MetalTop is
Metal5, and the deck reads Metal5 at 1.19 µm - the 11K Å MetalTop of `options.rb`'s
`'D' => { metal_top: '11K', mim_option: 'B', metal_level: '5LM' }` - not at an
intermediate metal's 0.54.  `ANT.16_i_ANT.6.h1` pins that.

Twenty layouts under `tests/data/gf180mcuD/generated/antenna/ANT.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/antenna.rs`, each with a `#[case]` in `hardening_antenna`.  The gate
throughout is 0.21 µm of Poly2 across 0.315 µm of COMP - 0.06615 µm², chosen so the bounds
land on the 0.005 µm grid - and a metal antenna is one 0.66 µm wide bar, perimeter
2(len + 0.66).  Every layout ran at tiles 20, 7 and 100 and through the upstream runset
with `DECKS=all` (the runset's antenna decks abort without the other decks' connectivity).
**No count moved with the tile size**, on any of them.  Marker counts differ by
construction - gdscheck reports one per violating node, KLayout one per shape or cut, so
ANT.8's fourteen contacts are 1 against 14 and ANT.9's twenty vias 1 against 21 - and
only the set of rules that fire is compared below.

## Findings

### 1. A MIM-B cap contacted the way section 10.4.2 draws it is invisible to the MIM antenna rules (false negative)

Section 10.4.2 defines MIM option B: "FuseTop layer still used to defines the top plate of
MiM capacitor and Metaln-1 layer defines MiM bottom plate (n > 2, n is the top metal
number)", and the top plate is taken up through "Vian-1".  On this five-level stack
n = 5: bottom plate Metal4, top plate FuseTop, and the plate connected by **Via4** to
Metal5.  MIMTM.4 is "Minimum MiM top plate (FuseTop) overlap of Vian-1", MIMTM.9 is the
sea of Via4 on the top plate, and MIMTM.10 says outright that the bottom plate "can only
be connected through the higher Via (Vian-1)" and that no Vian-2 - Via3 - may touch it.
There is no legal MIM-B layout in which Via3 reaches the cap.

`ANT.16_iii_ANT.14_M5_MIMB.h1` draws that cap: a 3 µm Metal4 bottom plate, a 0.5 µm
square FuseTop over it (0.25 µm² of MIM), one Via4 on the plate, and a 41.84 µm Metal5 bar
off the via.  Perimeter 85 µm, 85 × 1.19 / 0.25 = **404.6**, over ANT.14's 400.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | 1 (`ANT.16_iii_ANT.14_M5_MIMB`) |

`ANT.16_iii_ANT.15_V4_MIMB.h1` is the same cap with a sea of seventy-five Via4 on the
bottom plate's arm beside it: 76 × 0.0676 / 0.25 = **20.6**, over ANT.15's 20.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | 76 (`ANT.16_iii_ANT.15_V4_MIMB`) |

The cause is visible in `pdks/gf180mcuD/pdk.yml`, whose connectivity list ties the plate
in once, at the Via3 step:

```yaml
  - connector: via3              # 16
    layers: [fusetop]
```

with the comment "MIM option B ties the cap plate in at the Via3 phase
(`connect(via3, fusetop)` in antenna.rb)".  That is true of upstream's *via3 and metal4*
blocks, but upstream states the connection once per rule block and moves it up as the
block climbs: `antenna_via3` and `antenna_metal4` say `connect(via3, fusetop) if
ctx.mim_option == 'B'`, while `antenna_via4` and `antenna_metal5` - the blocks that emit
V4_MIMB and M5_MIMB - say `connect(via4, fusetop)`.  A single `via3` entry cannot serve
both, so on this deck the plate is reachable only through a Via3 that MIMTM.10 forbids,
and the two rules that matter for a real MIM-B cap never see a node at all.  The deck's
existing `ANT.16_iii_*.bad` fixtures pass because they contact the plate at Via3
(`gen/gf180mcuD/antenna.rs`, `Node::Fuse` starts its riser at level 3), which is the one
arrangement the manual rules out.

Verdict: **gdscheck is wrong, KLayout is right.**  The fix has to let the plate join at
Via4 for the Via4 and Metal5 rules; whether it should *also* keep the Via3 attachment for
V3_MIMB and M4_MIMB is upstream's own oddity - those two rules are emitted by blocks whose
`connect(via3, fusetop)` describes a cap one level lower than variant D has, and on a
legal layout they can never fire either.  The conservative fix is a second entry for
FuseTop at the Via4 step; the V3/M4 pair then stays as unreachable as it is upstream.

## Tested and clean

Everything else in the deck answered as the manual says, and agreed with the upstream
runset rule for rule.

- **The level each ratio is read at, and the cut between one level and the next.**  This
  is the deck's central condition - "An antenna includes all conducting structures on the
  same layer and the same electrical node that connect to an active area only via layers
  below it" - and it is the manual's own repair for a violation ("break the metal close to
  the gate and jog the metal to an upper metal level").  `ANT.16_i_ANT.2.h1` jogs a 408
  antenna up to Metal2 and back down to Metal1: clean on both tools, where
  `ANT.16_i_ANT.2.h2`, the same bar hung straight off the gate's contact, fires on both.
  `ANT.16_i_ANT.9.h2` does it for vias - one Via1 on the gate's Metal1, twenty on an
  island the gate reaches only through Metal2, 1.02 instead of 21.5 - against
  `ANT.16_i_ANT.9.h1`'s twenty on the gate's own pad, 20.4, which fires.  `ANT.1.h2` does
  it at the bottom of the stack: a gateless 40 µm Poly2 pad strapped to the gate through
  Metal1 is not added to the gate's Poly2 (71, not 316), because Poly2's ratio is read
  before the contact level exists.  `ANT.8.h2` does the same for contact.
- **What counts as the gate.**  `ANT.16_ii_ANT.2.h2` puts a 17.86 µm bar (302 against a
  whole gate, 604 against half of one) over two gates, with Dualgate covering the left
  half of the first and stopping exactly on the left edge of the second's Poly2.  The
  marker names an area, so the first gate is half thin and half thick and both halves are
  over 400 - `ANT.16_i_ANT.2` and `ANT.16_ii_ANT.2` both fire, on both tools - and the
  gate the marker merely abuts is thin and whole, and clean.  `ANT.1.h1` shows the other
  side of the split: ANT.1 is read on `tgate`, every gate thick or thin, so the thick
  gate's 200.7 fires just like the thin one's (two markers, both tools), while the 198.3
  between them stays quiet.  `ANT.1.h4` covers a gate with RES_MK: a marked gate is a
  resistor body, the gate layer takes it out, and a 200.7 Poly2 antenna and a 408 Metal1
  bar on it have no denominator and fire nothing.  Both tools.
- **The diodes, and their factors.**  `ANT.16_i_ANT.2.h3` is a 91.84 µm Metal1 bar at
  1510.  Put a 0.36 µm square of N+ COMP on the node under it (`h4`) and ANT.16 case (b) 1
  makes the denominator 0.06615 + 2 × 0.1296 = 0.32535 and the ratio 307: clean.  The
  factor is what saves it - at 1 the same layout is 510.  `h5` swaps the diode for a 1 µm²
  N-well tied to the node by an N+ tap, the PMOS side's protection diode, and the well's
  own area counts: 48.  `h6` is the control: a P+ diode in a 1 µm N-well that nothing
  ties, under a 184.34 µm bar.  Only the 0.1296 µm² of P+ diffusion is on the node, so it
  is 614 and fires; had the untied well been counted it would have been 86 and clean.
  `ANT.16_ii_ANT.2.h1` is that same bar and that same N+ diode on a gate Dualgate covers,
  where case (b) 2's factor of 15 gives 0.06615 + 15 × 0.1296 = 2.010 and a ratio of 99:
  clean, where the thin gate's factor of 2 would have been 614.  Both tools agree on all
  five.
- **No diode relief where the manual gives none.**  ANT.16 is written for "Metaln or
  Vian"; Poly2 and contact are outside it, and the deck gives ANT.1 and ANT.8 no diode
  parameters.  `ANT.1.h3` hangs an N+ diode off the Metal1 that straps a 200.7 Poly2
  antenna's gate: still fires, on both tools.
- **The values, and the thicknesses.**  ANT.1 at 200 (`ANT.1.h1`, 200.7 fires and 198.3
  does not), ANT.8 at 10 (`ANT.8.h1` fourteen contacts = 10.24 fires, `h2`'s thirteen =
  9.51 does not), ANT.9 at 20 (`ANT.16_i_ANT.9.h1`, twenty vias = 20.4), ANT.2 at 400
  (`ANT.16_i_ANT.2.h2`, 408, where 24.0 µm instead of 24.34 would be 391.8).  Metal5's
  thickness is the MetalTop one: `ANT.16_i_ANT.6.h1`'s 10.84 µm bar is 23 × 1.19 / 0.06615
  = 413.8 and fires, where at 0.54 µm it would be 187.8 and silent.
- **What is not the gate's node.**  `ANT.8.h2` also puts four contacts on the
  transistor's own source/drain COMP.  The diffusion is a different node from the gate
  poly, so they are not added to the gate's thirteen - added they would be 12.4, over the
  limit.  Clean on both tools.
- **The tile lines.**  `ANT.16_i_ANT.2.h7` is the 408 antenna with the gate sitting on the
  tile line at x = 20 and the bar running across the lines at 21, 40 and 42: one
  violation at 20, 7 and 100 µm tiles.  No fixture in this deck moved with the tile size.

## Resolution (2026-09-22)

Fixed in the PDK. FuseTop gets a second connect entry, at the Via4 phase (step 19, between
Via4's own metal and Metal5's), and `ANT.16_iii_ANT.15_V4_MIMB` reads the net one step
further up - `level_before: metal5_drawn`, as `V3_MIMB` already reads
`level_before: metal4_drawn` - because `level: via4` stops at the first Via4 step, which
is the one that bridges Metal4. With both, a cap contacted the way section 10.4.2 draws it
is on the same net as the Via4 on it: `ANT.16_iii_ANT.14_M5_MIMB.h1` reports its rule and
`ANT.16_iii_ANT.15_V4_MIMB.h1` reports its, at tiles 20, 7 and 100, as the runset does.

The V3/M4 pair keeps the Via3 entry and stays as unreachable on a legal layout as it is
upstream. `tests/pdk_integrity.rs` pins the new connect order, so the levels cannot drift
without a test saying so.
