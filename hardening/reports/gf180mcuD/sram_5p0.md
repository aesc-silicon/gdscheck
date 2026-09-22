<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / sram_5p0: hardening report

Deck `sram_5p0` against the GF180MCU design manual, section 11.1 (`gf180mcu_drm/drm_11_1.txt`,
"5V SRAM"), with chapter 7's base rules (`drm_07_06.txt` for DF, `drm_07_08.txt` for PL,
`drm_07_13.txt` for CO) as the values the section relaxes, and the derived layers of
`pdks/gf180mcuD/pdk.yml`.  Twelve layouts,
`tests/data/gf180mcuD/generated/sram_5p0/S.*.h<n>.gds.gz`, drawn by the `hardening` half of
`gen/gf180mcuD/sram_5p0.rs`, each with a `#[case]` in the `hardening_sram_5p0` table of
`tests/gf180mcuD.rs` (one of them additionally in `hardening_sram_base`).  Every layout ran
through gdscheck at tiles 20, 7 and 100 and through the upstream KLayout runset
(`DECKS=sram_5p0 hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

## Coverage

`show-deck` lists S.CO.4_MV, S.DF.4c_MV, S.DF.6_MV, S.DF.7_MV, S.DF.8_MV, S.DF.16_MV,
S.PL.5a_MV and S.PL.5b_MV - all eight rules of section 11.1, with S.CO.4_MV, S.DF.4c_MV and
S.DF.8_MV each carrying a `forbidden` half for the shape that runs out of the enclosing
layer.  Nothing in the section is missing.  There is no S.CO.3_MV and no S.M1.1_MV in the
manual either; what happens to CO.3 and M1.1 inside a 5 V core is finding 3.

## How the section reads

"5V SRAM cells with marking layer V5_XTOR should follow below specific rules which is
different from 3.3V/(5V)6V rules."  Eight values change; everything else in chapter 7 stays.
Every layout here draws a Dualgate over the whole of it - a 5 V core is a 5 V device whether
or not it is in an SRAM - and puts the SRAMCORE and V5_XTOR pair over one part only, so the
geometry outside the core answers to the base `_MV` rule at its larger value (CO.4 0.07,
DF.4c 0.6, DF.6 0.4, DF.7 0.6, DF.8 0.6, DF.16 0.6, PL.5a/5b 0.3).

Marker cuts, as in the 3.3 V report: gdscheck gives one `min_enclosure` marker per deficient
enclosure, KLayout one polygon per deficient side; one `min_space` pair each, KLayout
sometimes splitting a corner pair in two.

**No count moved with the tile size**: twelve layouts x three tile sizes, every rule
identical.  `S.CO.4_MV.h4` puts three 0.035 contact margins on x = 20, x = 42 and y = 20 and
a 0.445 N-well gap straddling x = 40; all four survive every tile.

Test status on the engine as of this report: 4 of the 12 cases fail (S.DF.7_MV.h1,
S.DF.16_MV.h1, S.PL.5a_MV.h1, S.PL.5b_MV.h1) plus the one `hardening_sram_base` case for
finding 3.  The rest pass, as do the sixteen older `good`/`bad` cases.

## Findings

### 1. The three space rules are silent where the two walls lie on one another

*S.DF.7_MV* 0.45, *S.DF.16_MV* 0.45, *S.PL.5a_MV* 0.12.  A shared edge is a space of nothing
and is reported (settled in the GF180 rounds, `abutting: report`); none of the three carries
it.  The class is `contact.md`'s finding 5 and `sram_3p3.md`'s finding 2, here three more
times.

| layout | the abutting structure | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `S.DF.7_MV.h1` (e) | a P+ active (41, 10)-(41.5, 11) with its left wall on the LVPWELL's right wall at x = 41, both inside the deep well | 2 (`0.4450 at (20.0, 10.0)-(20.445, 10.0)`, `0.4384 at (34.0, 13.0)-(34.31, 13.31)`) | 4 - those plus `(41,10;41,11)/(41,11.45;41,10)` |
| `S.DF.16_MV.h1` (e) | an N+ active (41, 10)-(42, 11) with its left wall on the N-well's at x = 41 | 2 | 4 |
| `S.PL.5a_MV.h1` (f) | field poly (40, 10)-(41, 11) with its right wall on an N+ active's left wall at x = 41 | 3 | 6 |

A *corner* touch is not the same case and gdscheck does report it: `S.PL.5a_MV.h1` (e) puts
the poly's corner on the active's corner at (35, 11) and gdscheck prints
`space 0.0000 < 0.12 at (35.0000, 11.0000)-(35.0000, 11.0000)`.  So the gap is specific to a
shared *edge*, a facing pair of zero length in the perpendicular direction.

Verdict: a false negative in gdscheck, three times.  Zero space is a space, and the manual
gives no exemption for a shape drawn edge to edge with the one it must stand clear of.

### 2. S.PL.5b_MV's `pairs: overlapping` loses the gap to an unrelated active

*S.PL.5a_MV*, "Space from field Poly2 to unrelated COMP / Spacer from field Poly2 to
Guard-ring 0.12"; *S.PL.5b_MV*, "Space from field Poly2 to related COMP 0.12".  The two
values are the same, so between them they have to answer for *every* gap under 0.12 from a
piece of field poly to an active - which is exactly how the deck reads the base pair
(`poly2.yml`: PL.5a_MV and PL.5b_MV on the same layers with `pairs: any`, "the related active
is the gate's own, so the two shapes overlap where the gate crosses and the gap meant is
elsewhere along them").

The SRAM pair splits them instead: S.PL.5a_MV runs on `poly_sram_field`, the poly that
overlaps *no* active, and S.PL.5b_MV on all the SRAM poly but with `pairs: overlapping`,
which measures only a poly and an active that overlap **each other**.  A gate poly whose
field stretch passes close to a *second* active falls between the two: 5a has let the whole
poly go because it is a gate somewhere, and 5b never looks at that pair.

`S.PL.5b_MV.h1` (c): an active at (26, 10)-(28, 11), a poly (26.5, 9.7)-(26.9, 13) crossing it
and running up past it, and a second N+ active at (27.015, 12)-(29, 13).  The gap from the
poly's right wall to the second active is 0.115.

| layout | structure | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `S.PL.5b_MV.h1` | (a) the gate's field stretch 0.115 into its own active's slot | fires, `0.1150 at (12.4850, 12.2)-(12.6, 12.2)` | fires, `(12.485,12.4;12.485,12)/(12.6,11.966;12.6,12.434)` |
| | (b) the same at 0.12 | clean | clean |
| | (c) the same gate 0.115 from a second active | **silent** | fires, `(26.9,13;26.9,11.966)/(27.015,12;27.015,13)` |
| total | | 1 | 2 |

S.PL.5a_MV is silent on (c) too, in both tools - upstream's
`poly_sram.not_overlapping(comp_sram)` drops the whole poly for the same reason - but
upstream's 5b catches it, because it reads `poly_sram.overlapping(comp_sram).separation(
comp_sram, 0.12)`: the *poly* must overlap some active, and the separation is then to any.

Verdict: a false negative in gdscheck.  The manual's two rules carry one value between them
and no gap under it may escape; `pairs: overlapping` on S.PL.5b_MV is stricter than upstream
and stricter than the base deck's own reading of the same pair of rules.

### 3. Chapter 7's contact rules are switched off for the whole SRAMCORE

Section 11.1 relaxes CO.4 and nothing else of section 7.12: there is no S.CO.3_MV, no
S.CO.6_MV, no S.M1.1_MV.  Everything the section does not name stays as chapter 7 wrote it.
But the base deck's `contact_no_sram` is `contact` less *all* of `sramcore` (upstream:
`main_contact = contact.not(sramcore)`), so inside a 5 V SRAM core no CO.3, CO.4, CO.6b or
CO.6a is measured at all, and `sram_5p0` supplies only CO.4's replacement.

`S.CO.4_MV.h3` draws five contacts with the same 0.035 COMP margin: (a) a core wholly under
V5_XTOR, (b) a core V5_XTOR covers in part, (c) a core V5_XTOR only abuts, (d) a bare core,
(e) no core at all - all five under the layout's Dualgate.

| structure | manual | gdscheck `sram_5p0` | gdscheck `contact` | KLayout |
| --- | --- | --- | --- | --- |
| (a) core under V5_XTOR | S.CO.4_MV | fires (11.0, 11.22) | - | fires |
| (b) core half under V5_XTOR | S.CO.4_MV - the marker selects the whole region | fires (15.0, 11.22) | - | fires |
| (c) V5_XTOR abutting | not 11.1's; CO.4 0.07 | silent (right) | **silent** | silent |
| (d) bare core | not 11.1's; CO.4 0.07 | silent (right) | **silent** | silent |
| (e) no core | CO.4 0.07 | silent (right) | fires (33.0, 11.22) | silent |
| total | S.CO.4_MV 2, CO.4 3 | 2 | 1 | 2 |

`hardening_sram_base::case_2` asks the `contact` deck for the three CO.4 the manual gives (c),
(d) and (e) and gets one.  The twin on the 3.3 V side is `sram_3p3.md` finding 3, which also
records the other half of that fault - the 3.3 V rules firing inside a 5 V core, visible here
as `S.CO.4_MV.h2` reporting **S.CO.4_LV** as well under `--deck sram_3p3`.

Verdict: a false negative in gdscheck and in upstream alike.  A marker that relaxes some rules
must not delete the rest: the base rules should drop only the shapes a replacement exists for.

## Read and left alone

- **S.DF.8_MV's N+ active comes from the bare SRAMCORE and it does not matter.**
  `ncomp_dn_sram` is `ncomp_in_dnwell ∩ sramcore`, with no voltage condition (upstream:
  `ncomp.inside(dnwell).and(sramcore)`), which looks like the fault of `sram_3p3` finding 3.
  It is harmless: the *enclosing* layer, `lvpwell_dn_sram_mv`, is gated on `sram_mv`, so with
  no V5_XTOR there is nothing to be enclosed by.  `S.DF.8_MV.h2` confirms it - a 3.3 V core
  (no V5_XTOR, no Dualgate) holding its active at 0.44, legal under chapter 7's 0.43 for a
  3.3 V SRAM and short of 11.1's 0.45 - and both tools are silent.
- **KLayout reports DF.6 for a poly that only abuts an active.**  In `S.PL.5a_MV.h1` (f) the
  field poly shares a wall with the N+ active and KLayout prints two S.DF.6_MV edge pairs,
  `(40.68,11;41,11)/(41,11;41.32,11)` and its twin at y = 10.  There is no gate there - the
  two shapes do not overlap and no channel exists - so this is KLayout's false positive, from
  `poly_sram.enclosed(comp_sram, 0.32)` treating a touching poly as enclosed by zero.
  gdscheck is silent, rightly, and the case does not carry it.
- **Marker granularity**: a contact short on all four sides is one marker in gdscheck and four
  in KLayout (`S.CO.4_MV.h3` 2 vs 8, `S.CO.4_MV.h4` 3 vs 12); KLayout splits a
  corner-to-corner space into two edge pairs.  One deficient margin is one violating
  structure; the cases follow gdscheck's cut.

## Tested and clean

- **S.CO.4_MV** at 0.04 (clean) and 0.035; a 45° COMP corner passing 0.0283 from the
  contact's corner (fires) and one at 0.046 (clean) - this rule has `metric: euclidian` and
  gets the chamfer right where its 3.3 V twin does not; and 0.035 outside the core, where
  CO.4's 0.07 fires (`S.CO.4_MV.h1`, 2 / 6).
- **S.CO.4_MV's crossing half**: a contact the COMP's edge cuts fires; a contact outside the
  COMP with its edge on the COMP's is no enclosure and is clean (`S.CO.4_MV.h2`, 1 / 1).
- **S.DF.4c_MV** at 0.45 (clean), 0.445, a 45° well corner at 0.318 (fires) and at 0.495
  (clean), and the active running out of the well (`S.DF.4c_MV.h1`, 3 / 7).
- **S.DF.6_MV**, the source/drain overhang, at 0.32 (clean) and 0.315; the same at 0.7 with
  the active's corner chamfered to pass 0.354 from the gate's wall (clean) and 0.212 (fires).
  Both tools agree; gdscheck measures the second as 0.3000 in the axis at
  (30.3, 13.0)-(30.6, 13.0), KLayout as the edge pair `(30.3,13.111;30.3,12.889)/(30.6,13;
  30.62,12.98)` (`S.DF.6_MV.h1`, 2 / 2).
- **S.DF.7_MV** at 0.45 (clean), 0.445, corner to corner at 0.4525 (clean) and 0.4384
  (fires) - the euclidian chord across a corner is read (`S.DF.7_MV.h1`).
- **S.DF.8_MV** at 0.45 (clean), 0.445, a 45° P-well corner at 0.318 (fires) and 0.495
  (clean), and the active running out of the P-well (`S.DF.8_MV.h1`, 3 / 7).
- **S.DF.16_MV** at 0.45 (clean), 0.445, and the two corner pairs (`S.DF.16_MV.h1`).
- **S.PL.5a_MV** at 0.12 (clean), 0.115, corner to corner at 0.1216 (clean) and 0.1131
  (fires), and a corner touch at 0 (`S.PL.5a_MV.h1`).
- **S.PL.5b_MV** on the gate's own active: the field stretch 0.115 into the slot fires and
  0.12 is clean (`S.PL.5b_MV.h1`).
- **The voltage class**: a core V5_XTOR covers in part is wholly 11.1's (the marker selects
  the region, it does not cut it); a core V5_XTOR only abuts is not (`S.CO.4_MV.h3`).
- **Tile invariance**: twelve layouts x tiles 20, 7 and 100, no rule's count moved.

---

## Resolution (2026-09-22)

- **Finding 1.** S.DF.7_MV, S.DF.16_MV and S.PL.5a_MV carry `abutting: report`.
- **Finding 2.** S.PL.5a_MV and S.PL.5b_MV are one measurement under two names now, both
  on `poly_sram_mv` with `pairs: any` - which is how chapter 7's PL.5a_MV and PL.5b_MV are
  written, for the same reason: 11.1 gives the two rules one value between them, so no gap
  under 0.12 may escape and each gap carries both ids. Read on the poly that gates no
  active, a gate over one active passing 0.115 µm from a second fell between them.
  `S.PL.5a_MV.h1` and `S.PL.5b_MV.h1` report both ids on every gap; the foundry layout goes
  6 -> 10 and 4 -> 10.
- **Finding 3** is the twin of `sram_3p3.md`'s finding 3 and was fixed with it: the base
  rules' exemptions name the class that replaces them, and the 5 V class is now SRAMCORE
  *with* V5_XTOR, as 11.1's own prose has it.

The MV layers already read euclidian and already selected by `sram_mv`; what changed for
them is that the marker selects whole regions rather than cutting them, and that
`ncomp_dn_sram` (S.DF.8_MV's active) comes from `sram_mv` rather than the bare marker.
