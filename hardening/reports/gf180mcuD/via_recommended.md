<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / via_recommended: hardening report

Deck `via_recommended` against the GF180MCU design manual, section 7.14
(`gf180mcu_drm/drm_07_15.txt`), third item of V*n*.3 and of V*n*.4, with the derived
layers of `pdks/gf180mcuD/pdk.yml`.  10 layouts,
`tests/data/gf180mcuD/generated/via_recommended/V<n>.<rule>.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/via_recommended.rs`, each with a `#[case]` in the
`hardening_via_recommended` table of `tests/gf180mcuD.rs`.  Every layout ran through
gdscheck at tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

## What this deck is

**A guideline deck, not a coverage gap.**  Three things say so and they all agree.

1. The manual's own table prints the item as "Minimum Metal[n] overlap of Vian on all
   sides for minimum Vian resistance **variation (guideline)**", and puts an asterisk on
   the rule number.
2. That asterisk is a pointer to Appendix B, "Rules not coded"
   (`gf180mcu_drm/drm_16.txt`), which lists `Vn.3iii` and `Vn.4iii` by name:
   "Minimum metal-n overlap of via-n on all sides for min. via-n resistance variation
   (guideline)".
3. The foundry's runset does not compute them.  `rule_decks/via.rb` registers exactly
   `V<n>.1`, `2a`, `2b`, `3a` (Via1) or `3b`, `3c`, `3d`, `4a`, `4b` and `4c` per level
   (`VIA_RULE_DEFS_1` / `VIA_RULE_DEFS_2_5`); there is no `via_rule_3_3` or
   `via_rule_4_3` anywhere in the tree.

So the oracle gives no second opinion on any rule of this deck.  In the tables below
KLayout's column is zero throughout, and that zero means "the runset has no such rule",
not "the runset disagrees".  What the layouts can still settle against the runset is
whether the *geometry* is what it is meant to be, and it is: on `V1.3.3.h3` and
`V1.3.3.h4` the mandatory V1.3a / V1.4a fire in both tools with identical counts
(2/2, 3/3 and 1/1), which is the runset confirming that the vias drawn as "no overlap at
all" really have none.

gdscheck keeps the deck out of `main` and `core` and in a suite of its own
(`pdks/gf180mcuD/suites/recommended.yml`), for the reason the suite's own header gives:
these fire on most of the geometry they cover.  That placement is right, and the round is
therefore about the **values, the layers and the deck's completeness**, which is what
follows.

## Coverage

`show-deck --process gf180mcuD --deck via_recommended` lists 8 rules: `V<n>.3.3` and
`V<n>.4.3` for *n* = 1 to 4, all `min_enclosure`, all 0.12.

- **The value is right.**  The manual's V*n*.3 column reads 0.00 (n = 1) / 0.01
  (2 ≤ n ≤ 5), 0.06, 0.06, **0.12**, one per numbered item; V*n*.4 reads 0.01, 0.06,
  0.06, **0.12**.  The fourth number of each is item III, and it is the same 0.12 on
  every level and for both metals - no 3.3 V / 5 V split, no per-level variation.
- **The layers are right.**  V*n*.3.3 takes the metal *below* the via and V*n*.4.3 the
  metal *above* it: `[metal1, via1]`, `[metal2, via1]`, `[metal2, via2]` ... up to
  `[metal5, via4]`.  `V2.3.3.h1`, `V3.3.3.h1` and `V4.3.3.h1` put a 0.115 margin on the
  lower metal and another on the upper metal of one via and read exactly one of each id,
  so no level is wired to its neighbour's metal.
- **The level range is right.**  Variant D's stack ends at Metal5, so there is no Via5;
  the runset's `next unless ctx.metal_level_numerical > lvl` evaluates the same four
  levels.  Metal5 *is* the variant's MetalTop (`top_metal` is a single-source union of
  `metal5` in `pdk.yml`), so `V4.4.3`'s `[metal5, via4]` is the manual's
  "Metal[n+1] overlap" at the top of the stack.
- **The deck is complete for section 7.14** - eight rules, nothing of the section
  missing.  Its *suite* is not; see finding 3.

## Findings

### 1. The guideline reads projection, so a 45° metal corner is never measured

*V*n*.3.3 / V*n*.4.3*, "Minimum Metal[n] overlap of Vian **on all sides**".

An enclosure is read euclidian - the closest approach from the enclosed shape's boundary
to the enclosing one - and a chamfer or a 45° wall passing under the value from a corner
fires (the settled reading of 2026-09-21).  Neither rule of this deck carries
`metric: euclidian`, and the answer it gives is the projection one.

`V1.3.3.h2` draws a Metal1 square *m* clear of the via on every axis with its top-right
corner chamfered so that *k* is cut off each side.  The chamfer's perpendicular distance
to the via's corner is (2*m* − *k*)/√2 while every axis margin stays at *m*:

| structure | *m*, *k* | perpendicular approach | nearest projection | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- | --- | --- |
| (a) | 0.20, 0.230 | 0.1202 | 0.165 | clean | silent | no such rule |
| (b) | 0.20, 0.235 | 0.1167 | 0.165 | V1.3.3 | silent | no such rule |
| (c) | 0.15, 0.150 | 0.1061 | 0.150 | V1.3.3 | silent | no such rule |
| (d) | 0.15, 0.150, on Metal2 | 0.1061 | 0.150 | V1.4.3 | silent | no such rule |

(c) and (d) are the ones that cannot be argued away: the chamfer ends exactly level with
the via's top and right edges, so no wall of the via faces it at all and every projection
in the structure is the full 0.15.  The metal is 0.1061 µm from the via's corner and the
deck says nothing.

The engine can do this - the engine family's `min_enclosure` deck carries an `ENC.eucl`
rule beside `ENC.proj` and its `shapes` pattern reads 6 markers under the first and 1
under the second, the difference being exactly the chamfers and the diamond.  What is
missing is the parameter on this deck's rules.  Worth noting that the mandatory `via`
deck's V*n*.3a / 3b / 4a carry no `metric` either, so the same corner is unmeasured
there; that deck's round did not draw a chamfer.

Verdict: a false negative in the deck.  A via whose metal corner is cut at 45° is exactly
the layout the guideline exists for - the corner is where the current crowds - and the
manual's "on all sides" is a statement about the metal all round the via, not about four
edge pairs.

### 2. gdscheck stops at one marker per via, however many of its sides are short

Same rules.  `V1.3.3.h1` draws six vias, each with its own Metal1 box and a generous
Metal2 over it; `V1.4.3.h1` is the same with the metals swapped.

| structure | Metal1 margins [l, r, b, t] | sides under 0.12 | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- | --- |
| (a) | 0.12, 0.12, 0.12, 0.12 | 0 | clean | clean | no such rule |
| (b) | 0.115, 0.12, 0.12, 0.12 | 1 | 1 | 1 | no such rule |
| (c) | 0.115, 0.12, 0.115, 0.12 | 2 | 2 | **1** | no such rule |
| (d) | 0.115 all four | 4 | 4 | **1** | no such rule |
| (e) | 0.12, 0.12, 0.12, 1.0 | 0 | clean | clean | no such rule |
| (f) | 0.0, 0.12, 0.12, 0.12 | 1 | 1 | 1 | no such rule |

Total: the manual's 8, gdscheck's 4, at every tile size, on both fixtures.  gdscheck's
marker for (c) is `enclosure 0.1150 µm < 0.12 µm of via1 within metal1 at
(16.0000, 10.0000)-(16.2600, 10.0000)` - the bottom wall - and the left wall, equally
short, is never reported; for (d) it is again the bottom wall alone.

This is the enclosed shape, not the enclosing region: `V1.3.3.h6` puts two vias under one
Metal1 region, one 0.115 from its left wall and one 0.115 under a notch cut into its top,
and both are reported.  So the cap is one marker per (via, metal) pair.

The engine's own convention is one marker per deficient side - `gen/engine/min_enclosure.rs`'s
`bound` pattern draws the 0.495 margin on the left, the right, the bottom and the top of
*four separate squares* and the case expects 4 - but no engine pattern puts two short
sides on one shape, so the cap has never been seen there.

Verdict: under-reporting in gdscheck.  Two short sides are two sides the guideline names,
and a designer fixing the one wall gdscheck prints still has a via that is short on
another.  The class belongs in the engine family (`min_enclosure`, a shape short on two
adjacent sides and on all four), since nothing about it is GF180's.

### 3. The recommended suite has the via guidelines and none of their siblings

Appendix B lists four geometric guidelines with values.  Two of them are this deck.  The
other two, and the contact twin of this very rule, are nowhere in `pdks/gf180mcuD/`:

| Appendix B entry | manual | value (3.3 V / 5 V) | in gdscheck |
| --- | --- | --- | --- |
| `Vn.3iii`, `Vn.4iii` | 7.14 | 0.12 | `via_recommended`, 8 rules |
| `CO.6iii` "Minimum Metal1 overlap of contact on all sides for minimum contact resistance variation (guideline)" | 7.12, `drm_07_13.txt` | 0.12 | absent |
| `DF.1b` "Min. COMP Width as a resistor with low sheet resistivity" | 7.5, `drm_07_06.txt` | 0.3 / 0.3 | absent |
| `PL.3b` "Poly2 Space on COMP for low active sheet resistivity (guideline)" | 7.7, `drm_07_08.txt` | 0.26 / 0.4 | absent |

`CO.6iii` is the same sentence as `Vn.3iii` with Metal1 and contact in place of Metal*n*
and Via*n*, the same 0.12, in the same appendix list two lines apart.  The `contact` deck
has `CO.6` (0.005), `CO.6a` and `CO.6b` - items I and II of the manual's CO.6 - and stops
there.  `suites/recommended.yml` includes one deck.

Verdict: a coverage gap, in the suite rather than in the deck.  `via_recommended` covers
its own section completely; the guidance suite it was made for covers a quarter of what
the manual marks as guidance.  Whether the other three are worth coding is a judgement
for whoever merges this - `DF.1b` and `PL.3b` are 3.3 V / 5 V pairs and would need the
`_LV` / `_MV` split - but the asymmetry is not one anybody chose.

## Read and left alone

1. **"Overlap of nothing" fires, and does so without a `forbidden` half.**  The enclosure
   rules of other decks need a crossing half for a shape that runs out of the enclosing
   layer; `min_enclosure` here reports it itself, as
   `shape on via1 not enclosed by metal1 at (10.1300, 10.1300)`.  `V1.3.3.h3` draws all
   three ways to have no overlap on a side - the metal stopping half way across the via,
   ending flush with its edge, and absent altogether - once below and once above, and all
   six are reported.  The flush one reads `enclosure 0.0000 µm < 0.12 µm` rather than
   "not enclosed", which is the same answer said differently: the via is inside the
   metal, the overlap on that side is zero.  KLayout's V1.3a (2) and V1.4a (3) on the
   same fixture confirm the geometry - V1.3a is silent on the flush structure because a
   via flush inside Metal1 *is* enclosed, and V1.4a is not because it also asks 0.01.
2. **A metal ring whose hole holds the via is an enclosure of nothing.**  `V1.3.3.h4` (d)
   puts a via alone in the hole of a Metal1 annulus whose inner wall is 0.12 from it on
   every side.  The distance to the boundary is exactly the value, but there is no
   overlap at all, and gdscheck says `shape on via1 not enclosed by metal1`.  That is the
   settled reading ("a corner touching the enclosing boundary is an enclosure of
   nothing"), read at the extreme, and it is right: the manual asks for *overlap*.
   KLayout's V1.3a agrees (1 marker).
3. **A hole in the metal is a wall of the metal.**  `V1.3.3.h4` (b) and (c) cut a 0.1 µm
   hole into a plate that is 0.3 clear of the via, its nearest wall 0.115 and 0.12 from
   the via's right edge.  The first fires and the second does not, which is the answer the
   manual asks for - the metal within the margin has to be metal.
4. **The union, not the boxes.**  `V1.3.3.h4` (a) draws the 0.12 plate as four
   overlapping boxes whose seams run straight across the via, and it is clean.

## Tested and clean

Everything below was drawn, run at three tile sizes and through the runset, and gdscheck
gave the answer the manual asks for.  No need to redo it.

- **The bound**, one deficient side at a time: 0.12 clean, 0.115 fires, a margin of 1.0
  clean, a side flush with the via fires - against Metal1 (`V1.3.3.h1`) and against
  Metal2 (`V1.4.3.h1`).  Only the multi-side structures of those two fixtures disagree
  (finding 2).
- **The levels**: Via2 / Metal2 / Metal3, Via3 / Metal3 / Metal4 and Via4 / Metal4 /
  Metal5, each with one short margin below and one above and a second, generous via of
  the same level beside it (`V2.3.3.h1`, `V3.3.3.h1`, `V4.3.3.h1`, one of each id).
- **Two vias under one plate** (`V1.3.3.h6`): both reported, so nothing about the rule is
  per-region.
- **The tile lines** (`V1.3.3.h5`): the same 0.115 margin with its measured gap laid
  across x = 20, x = 42 and y = 20 - three markers at every tile size.
- **Tile invariance**: 10 layouts × tiles 20, 7 and 100, every rule's count and every
  marker's coordinates identical.  Nothing here moved with the tile size.

Test status on the engine as of this report: 3 of the 10 new cases fail
(`V1.3.3.h1`, `V1.4.3.h1` on finding 2, `V1.3.3.h2` on finding 1).  The other 7 pass, as
do the 16 older `via_recommended` good/bad pairs in `via_patterns_cover_every_rule`.
