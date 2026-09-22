<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / sram_3p3: hardening report

Deck `sram_3p3` against the GF180MCU design manual, section 11.2 (`gf180mcu_drm/drm_11_2.txt`,
"3.3V SRAM"), with chapter 7's base rules (`drm_07_06.txt` for DF, `drm_07_13.txt` for CO,
`drm_07_14.txt` for M1) as the values the section relaxes, and the derived layers of
`pdks/gf180mcuD/pdk.yml`.  Twelve layouts,
`tests/data/gf180mcuD/generated/sram_3p3/S.*.h<n>.gds.gz`, drawn by the `hardening` half of
`gen/gf180mcuD/sram_3p3.rs`, each with a `#[case]` in the `hardening_sram_3p3` table of
`tests/gf180mcuD.rs` (two of them additionally in `hardening_sram_base`).  Every layout ran
through gdscheck at tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`DECKS=sram_3p3 hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

## Coverage

`show-deck` lists S.CO.3_LV, S.CO.4_LV, S.CO.6_ii_LV, S.DF.4c_LV, S.DF.16_LV and
S.M1.1_LV - all six rules of section 11.2, with S.CO.3_LV, S.CO.4_LV and S.DF.4c_LV each
carrying a `forbidden` half for the shape that runs out of the enclosing layer.  Nothing in
the section is missing.

## How the section reads

Section 11.2 is a *relaxation*: "3.3V SRAM cells without marking layer V5_XTOR should follow
below specific rules which is different from 3.3V/(5V)6V rules".  Six values change inside
the marker; every other rule of chapter 7 stays as it is.  So each fixture carries both
halves - the same geometry under the marker and beside it - and the half outside answers to
the base rule (CO.3 0.07, CO.4 0.07, CO.6 II 0.06, DF.4c 0.43, DF.16 0.43, M1.1 0.23).

Marker cuts: gdscheck reports one `min_enclosure` marker per deficient enclosure however many
of the shape's sides are short, one per `min_space` pair, one per `forbidden` region and two
per narrow `min_width` bar (one per wall); KLayout reports one polygon per deficient *side*
(four for a contact short on all four) and one edge pair per space.  Counts below are the
markers each tool printed; a difference that is not a cut is a finding.

**No count moved with the tile size**: twelve layouts x three tile sizes, every rule
identical.  `S.CO.3_LV.h5` puts four 0.035 poly margins on x = 20, x = 40, x = 42 and
y = 20 and a 0.395 N-well gap straddling x = 20 on purpose; all five survive every tile.

Test status on the engine as of this report: 7 of the 12 cases in `hardening_sram_3p3` fail,
each on a finding below (h2, S.CO.4_LV.h1, S.DF.4c_LV.h1/h2/h3, S.DF.16_LV.h1, S.CO.3_LV.h4),
and both cases in `hardening_sram_base` fail on finding 3.  The five that pass and the twelve
older `good`/`bad` cases are unchanged.

## Findings

### 1. The three LV enclosures do not see a 45° wall of the enclosing layer

*S.CO.3_LV*, "Poly2 overlap of contact 0.04"; *S.CO.4_LV*, "COMP overlap of contact 0.03";
*S.DF.4c_LV*, "Min. (Nwell overlap of PCOMP) outside DNWELL 0.4".  The enclosure of a shape
lying inside another is the closest approach from its boundary to the enclosing one (SPEC,
settled 2026-09-21).  All three miss it when the enclosing layer's corner is chamfered.  The
same class was reported and fixed for the base CO.3 / CO.4 (`contact.md`, finding 1, resolved
with `metric: euclidian`); the three LV rules here carry no `metric` at all, while their 5 V
twins in `sram_5p0` do - and `sram_5p0`'s catch every one of these shapes.

| layout | geometry | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `S.CO.3_LV.h2` | poly margin 0.04 on both straight walls, the corner chamfered 0.05 back - the 45° wall passes 0.0212 from the contact's corner; a 0.02 chamfer at 0.0424 is the control | **0** | 2 |
| `S.CO.4_LV.h1` | COMP margin 0.03 straight, chamfer 0.03 back (0.0212); control 0.015 (0.0318) | 1 (the 0.025 margin only) | 6 |
| `S.DF.4c_LV.h1` | N-well over P+ active by 0.45... 0.4 straight, the well's corner chamfered 0.4 back (0.2828 from the active's corner); control 0.2 (0.4243) | 2 (0.395 and the crossing) | 7 |

gdscheck's markers in `S.CO.3_LV.h2`: none.  KLayout's two are
`(12.192,10.219;12.21,10.261;12.251,10.22;12.221,10.219)` and its mirror at the same corner -
the chamfered cell at x = 12 - and the x = 14 control is clean in both.  In `S.DF.4c_LV.h1`
gdscheck prints `enclosure 0.3950 < 0.40 at (16.3950, 13.6050)-(16.3950, 10.3950)` and
`s_df_4c_lv_crossing present at (38.2500, 12.0000)`, and nothing at the chamfered cell
(x = 22..26); KLayout's markers there are `(25.599,13.433;…)` and `(25.433,13.599;…)`.

Verdict: a false negative in gdscheck, three times.  KLayout's `enclosed(..., euclidian)` is
right and the manual's "overlap of" is the margin all round the shape, not only where two
walls face.

### 2. S.DF.16_LV is silent where the active's wall lies on the well's

*S.DF.16_LV*, "Min. space from (Nwell outside DNWELL) to (NCOMP outside Nwell and DNWELL)
0.4".  A shared edge is a space of nothing and is reported (settled in the GF180 rounds,
`abutting: report`).  `S.DF.16_LV.h1` (e) puts an N+ active at (41, 10)-(42, 11) with its
left wall on the N-well's right wall at x = 41.

| layout | gdscheck @20/7/100 | KLayout |
| --- | --- | --- |
| `S.DF.16_LV.h1` | 2 - `0.3950 at (20.0000, 10.0000)-(20.3950, 10.0000)` and `0.3960 at (34.0000, 13.0000)-(34.2800, 13.2800)` | 4 - those two plus the edge pair `(41,10;41,11)/(41,11.4;41,10)` |

Everything else in the layout is right in both tools: 0.4 clean, 0.395 fires, the
corner-to-corner pair at 0.4243 clean and the one at 0.396 fires, so the euclidian metric and
the corner chord are both in place.

Verdict: a false negative in gdscheck - `min_space` without `abutting: report`, the class of
`contact.md` finding 5, here on S.DF.16_LV.

### 3. The four contact and metal rules read the bare SRAMCORE, so they fire in a 5 V core

Section 11.2 is for "3.3V SRAM cells **without** marking layer V5_XTOR"; 11.1 is for the 5 V
cells with it.  The deck derives `poly_sram`, `comp_sram`, `contact_sram` and `metal1_sram`
from `sramcore` alone, with no voltage condition at all, while `nwell_n_dn_sram`,
`pcomp_out_dn_sram` and `ncomp_out_nw_dn_sram` come from `sram_lv`.  So S.CO.3_LV, S.CO.4_LV,
S.CO.6_ii_LV and S.M1.1_LV apply to every SRAM core whatever marks it.

`S.CO.3_LV.h4` draws five cores, each with the same 0.035 poly margin on a contact: (a) the
marker alone, (b) the marker under Dualgate, (c) under Dualgate and V5_XTOR, (d) under
V5_XTOR, (e) with Dualgate abutting but not over it.

| core | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| (a) bare | S.CO.3_LV | fires, (11.0, 11.22) | fires (4 sides) |
| (b) Dualgate, no V5_XTOR | S.CO.3_LV (no V5_XTOR ⇒ 11.2) | fires, (15.0, 11.22) | silent |
| (c) Dualgate + V5_XTOR | silent - 11.1's core, which has no S.CO.3_MV, so CO.3's 0.07 | fires, (25.0, 11.22) | silent |
| (d) V5_XTOR, no Dualgate | silent - same | fires, (29.0, 11.22) | silent |
| (e) Dualgate abutting | S.CO.3_LV - a marker abutting is not over (SPEC, settled) | fires, (33.0, 11.22) | silent |
| total | 3 | **5** | 1 |

The same layout run through the `contact` deck is silent altogether: `contact_no_sram` is
`contact` less the *whole* of `sramcore`, so the two 5 V cores answer to no CO.3 anywhere.
`hardening_sram_base::case_1` asks for the two CO.3 the manual gives them and gets `[]`.
The twin on the 5 V side is `sram_5p0.md` finding 3.

The mirror image is visible on the 5 V fixtures: `sram_5p0/S.CO.4_MV.h2`, a 5 V core, reports
**S.CO.4_LV** as well as S.CO.4_MV under gdscheck (`--deck sram_3p3` on it gives one
S.CO.4_LV); KLayout reports only S.CO.4_MV.

Verdict: two faults in one.  (i) A false positive: the four rules must be gated on the
voltage class, as upstream's `metal1 = metal1.and(sram_lv)` does.  (ii) A false negative
underneath it: whatever is not section 11's stays chapter 7's, so a 5 V SRAM core owes CO.3
0.07, CO.6 0.005, CO.6a and CO.6b, and a 3.3 V one owes CO.6, CO.6a and CO.6b - none of which
either tool applies, because the base contact rules subtract the marker outright.

### 4. The two classes are neither complementary nor exhaustive

`sram_lv` is `sramcore not_overlapping dualgate`; `sram_mv` is `sramcore overlapping v5_xtor`.
A core under Dualgate without V5_XTOR belongs to neither, and the base rules have already
dropped the marker, so it answers to nothing at all.  Upstream has the same hole differently
placed - its `sram_lv` is `sramcore.not_interacting(v5_xtor).not_interacting(dualgate)`, which
also drops the core Dualgate merely abuts.

`S.DF.4c_LV.h3` is the five cores of finding 3 again, measured by a rule the deck *does*
derive from `sram_lv`: the Dualgate cores (b) and (c) and the V5_XTOR core (d) hold their P+
active at 0.395, the bare core (a) and the abutting one (e) at 0.4.

| core | enclosure | manual | gdscheck | KLayout |
| --- | --- | --- | --- | --- |
| (a) bare | 0.4 | clean | clean | clean |
| (b) Dualgate | 0.395 | S.DF.4c_LV | silent | silent |
| (c) Dualgate + V5_XTOR | 0.395 | silent (11.1's 0.45, met) | silent | silent |
| (d) V5_XTOR | 0.4 | silent here - it is 11.1's core, and 0.4 is S.DF.4c_MV's business | silent (by luck: `sram_lv` keeps it, and 0.4 meets 0.4) | silent |
| (e) Dualgate abutting | 0.395 | S.DF.4c_LV | fires, (38.395, …) | silent |
| total | | 2 | **1** | 0 |

The settled reading of this round's predecessors - "LV/MV rule pairs are complementary classes:
`overlapping`/`not_overlapping` the Dualgate marker, no v5_xtor step" - would close the hole
by making `sram_mv` `sramcore overlapping dualgate`.  Section 11 is the one place in the
manual where V5_XTOR is named in the prose of the rule ("cells **with** marking layer
V5_XTOR", "**without** marking layer V5_XTOR"), which is why this is worth a second look
rather than a silent alignment; either way, the pair as it stands leaves a class of cores
unchecked.  KLayout additionally drops core (e), which the settled "abutting is not over"
reading says is an ordinary 3.3 V core.

Verdict: a false negative, and a decision for whoever merges: make the two classes
complementary on Dualgate (the settled reading), or on V5_XTOR (the section's prose).  Not a
count question - the *three* cores gdscheck names are not the three the manual names.

### 5. The marker cuts the N-well, and the cut invents a wall

SramCore "is used to mark SRAM cells" - it names cells, and a marker naming a cell selects
whole regions (SPEC, settled).  `nwell_n_dn_sram` is `nwell_n_dn` **intersected** with
`sram_lv`, so a well that runs out of the marker gets a boundary where the marker ends, and
the enclosure is measured to that boundary.

`S.DF.4c_LV.h2` (a): a 6 x 6 µm N-well at (10, 10)-(16, 16) holding a 4 x 4 P+ active at
(11, 11)-(15, 15) - an enclosure of 1.0 on every side - with the SRAMCORE running
(9, 9)-(15.2, 17), ending 0.2 to the right of the active.  (b) is the same well wholly under
a marker, and is clean in both tools.

| layout | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `S.DF.4c_LV.h2` | clean - the well holds the active by 1.0 | 1: `enclosure 0.2000 < 0.40 at (15.0000, 11.0000)-(15.0000, 15.0000)` | 1: `(15.201,10.653;14.999,10.999;14.999,15.001;15.201,15.347)` |

Verdict: a false positive in both tools.  An SRAM array's well runs across the whole array
and past whatever cell boundary the marker draws; requiring it to end inside the marker is
not a rule anybody wrote.  The pair to measure should be selected by the *enclosed* shape
lying in the marker, with the enclosing well read whole.  The same cut is in the base deck
(`nwell_n_dn_no_sram = nwell_n_dn − sramcore`, and upstream's `nwell_n_dn.not(sramcore)`),
where it invents the same wall on the other side of the marker - that one belongs to the
`comp` deck's owner.

## Read and left alone

- **S.CO.6_ii_LV's trigger and value look swapped, and are not.**  The manual writes "If
  Metal1 overlaps contact by < 0.04 µm on one side, adjacent metal1 edges Overlap - 0.02":
  the trigger is chapter 7's 0.04 and the *adjacent* margin drops from CO.6b's 0.06 to 0.02.
  The deck has `value: 0.04, trigger: 0.02`, and so does upstream
  (`enclosed(metal1_sram, 0.02, projection)` then `enclosed(metal1_sram.edges, 0.04)`).  The
  predicate is symmetric over an unordered adjacent pair - "one side under T and its
  neighbour under V" is "min < min(T,V) and max < max(T,V)" either way round - so the two
  readings give the same answer on every rectangle.  `S.CO.6_ii_LV.h1` confirms it on four:
  0.035/0.02 clean, 0.035/0.015 fires, 0.04/0.015 clean (no trigger), 0.015/0.03 fires; both
  tools print exactly those two, gdscheck at (12.0, 10.0)-(12.22, 10.0) and
  (16.0, 10.22)-(16.0, 10.0).  Nothing to change; recorded so nobody "fixes" it.
- **Marker granularity.**  A contact short on all four sides is one marker in gdscheck and
  four in KLayout (`S.CO.3_LV.h1`: 1 vs 4, `S.CO.3_LV.h5`: 4 vs 16).  One deficient margin is
  one violating structure; the cases follow gdscheck's cut, as the settled reading says.
  `min_width` keeps its two markers per narrow bar (`S.M1.1_LV.h1`, gdscheck 2, KLayout 1).

## Tested and clean

Everything below was drawn, run at three tile sizes and through the runset, and both tools
gave the answer the manual asks for.

- **S.CO.3_LV** at 0.04 (clean) and 0.035 inside the marker; 0.04 and 0.075 outside it, where
  CO.3's 0.07 fires on the first and not the second (`S.CO.3_LV.h1`, gdscheck 1 + CO.3 1).
- **S.CO.4_LV** at 0.03 (clean) and 0.025 (`S.CO.4_LV.h1`).
- **The crossing halves**: a contact the poly's edge cuts is S.CO.3_LV, a contact the COMP's
  edge cuts is S.CO.4_LV, and a contact wholly outside the layer with its edge on the layer's
  is neither (`S.CO.3_LV.h3`, gdscheck 1 + 1, KLayout 1 + 1).
- **S.CO.6_ii_LV**: the four configurations above (`S.CO.6_ii_LV.h1`, 2 / 2).
- **S.M1.1_LV**: tracks 0.22 (clean), 0.215, 0.225 (clean) inside the marker and 0.215 outside
  it, where M1.1's 0.23 fires instead (`S.M1.1_LV.h1`, S.M1.1_LV 2 and M1.1 2).
- **S.DF.4c_LV** at 0.4 (clean) and 0.395, and the P+ active running 0.5 out of the well,
  which is the `forbidden` half (`S.DF.4c_LV.h1`).
- **S.DF.16_LV** at 0.4 (clean), 0.395, corner to corner at 0.4243 (clean) and 0.396 - the
  euclidian chord across a corner is read (`S.DF.16_LV.h1`).
- **Tile invariance**: twelve layouts x tiles 20, 7 and 100, no rule's count moved; four
  contacts and an N-well gap deliberately laid across x = 20, x = 40, x = 42 and y = 20 in
  `S.CO.3_LV.h5` give 4 + 1 at every tile.
