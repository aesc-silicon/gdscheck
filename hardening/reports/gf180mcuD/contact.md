<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / contact: hardening report

Deck `contact` against the GF180MCU design manual, section 7.12 (CO.1 to CO.11,
`gf180mcu_drm/drm_07_13.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  21
layouts, `tests/data/gf180mcuD/generated/contact/CO.<rule>.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/contact.rs`, each with a `#[case]` in the
`hardening_contact` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at
tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

`show-deck` lists CO.1, CO.2a (space and notch), CO.2b, CO.3, CO.4, CO.5a, CO.5b, CO.6,
CO.6a, CO.6b, CO.7, CO.8, CO.9, CO.10 and CO.11 - every rule of the section, with CO.3 and
CO.4 each carrying a `forbidden` half for the contact that crosses the layer's edge.
Nothing in the section is missing.  The third item of CO.6 (0.12 µm Metal1 overlap on all
sides) is the guideline the manual marks "\* Appendix B: Rules not coded"; see note 3
below.  Section 7.12 has no 3.3 V / 5 V split, so no rule here is `_LV` / `_MV`, and
`show-deck` correctly has none.

Marker cuts: gdscheck reports two `exact_width` markers per contact that is off in one
direction (one per wall), one per deficient side for an enclosure, one per pair for a
space and one per region for a `forbidden`; KLayout reports one edge per `without_length`,
a polygon per enclosure failure and one edge pair per space.  Counts below are of the
markers each tool actually printed; where they differ for a reason other than cutting, it
is a finding.

No count moved with the tile size: 21 layouts × 3 tiles, every rule identical.
`CO.2b.h3` and `CO.9.h2` put their patterns on x = 20, 21, 40, 42 and y = 21 on purpose -
the array cluster and the N+/P+ butting edge are both derived across the cut, and both
survive it.

Test status on the engine as of this report: 8 of the 21 new cases fail, each on a finding
below (CO.2b.h1/h2/h3, CO.3.h1, CO.4.h1, CO.9.h1, CO.10.h1, CO.11.h1).  The other 13 pass,
as do the 25 older contact cases.

## Findings

### 1. The enclosure rules do not see a 45° wall of the enclosing layer (CO.3, CO.4)

*CO.3*, "Poly2 overlap of contact 0.07"; *CO.4*, the same 0.07 of COMP.  The enclosure of
a shape lying inside another is the closest approach from its boundary to the enclosing
one (SPEC, settled 2026-09-21).  Both rules miss it when the enclosing layer's corner is
chamfered.

`CO.3.h1` / `CO.4.h1` draw a 1 µm square of poly (of COMP) at (3, 8)-(4, 9) whose top
right corner is cut on `x + y = 12.95`, with the contact at (3.71, 8.71)-(3.93, 8.93) -
0.07 from both straight edges, at the bound.  The chamfer's perpendicular distance to the
contact's corner (3.93, 8.93) is 0.09 / √2 = **0.0636**, under the value; the foot of the
perpendicular, (3.975, 8.975), lies on the chamfer segment.

| layout | gdscheck @20/7/100 | KLayout | markers |
| --- | --- | --- | --- |
| `CO.3.h1` | 2 × CO.3 | 4 × CO.3 | gdscheck: (8.0, 3.0) and (13.0, 3.0) - the 0.065 and the flush case.  KLayout: those two plus two at (3.92, 8.929) and (3.929, 8.92), the chamfer |
| `CO.4.h1` | 2 × CO.4 | 4 × CO.4 | the same coordinates |

The control in the same layouts, a 0.04 chamfer on `x + y = 12.96` at 0.10 / √2 = 0.0707,
is clean in both tools.

Verdict: a false negative in gdscheck, the same class as the dualgate report's finding 1 -
`min_enclosure` with `interacting_only` and a 45° wall of the *enclosing* shape passing
under the value from a corner of the enclosed one.  KLayout's `enclosed(..., euclidian)`
gets it right here.

### 2. CO.2b reports at most one violating pair per array

*CO.2b*, "Space in 4x4 or larger contact array 0.28".  `CO.2b.h1` draws a 4 × 4 array at
0.30 whose third row gap is 0.275: four distinct contact pairs, one per column, each
0.275 apart.  gdscheck prints exactly one marker, KLayout four.

| layout | pairs drawn | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `CO.2b.h1` | 4 (one array) | 1 | 4 |
| `CO.2b.h3` | 12 (three arrays, on the tile lines) | 3 | 12 |

gdscheck's single marker in `CO.2b.h1` is `4×4 contact array (1.78×1.76 µm): space 0.2750
µm < 0.28 µm at (3.1100, 4.2600)-(3.1100, 4.5350)` - the first column's pair.  In
`CO.2b.h3` the three markers are one per array, at x = 3.11, 19.21 and 41.21.

This is more than a cut: four contact pairs in four different columns are four violating
structures, and a second, independent deficiency elsewhere in the same array is invisible.

Verdict: under-reporting in gdscheck; one marker per array, however many pairs are short.

### 3. A contact that belongs to the array's cluster but not to its grid is never measured

Still CO.2b.  `CO.2b.h2` puts a seventeenth contact 0.275 to the right of one row of an
otherwise legal 4 × 4 array (every array gap 0.30).  Upstream sizes the contacts by 0.16,
merges and shrinks back, and applies 0.28 to every contact *interacting* the resulting
cluster, so the extra contact is in the array.  gdscheck is silent on the whole layout
except for structure (d).

| structure | geometry | gdscheck | KLayout |
| --- | --- | --- | --- |
| (a) | 17th contact 0.275 right of a legal 4 × 4 | silent | fires, edge pair `(5.055,3.52;5.055,3.74)\|(4.78,3.74;4.78,3.52)` |
| (b) | the same beside a 3 × 3 - no array, CO.2a's 0.25 applies | clean | clean |
| (c) | a contact 0.20 across and 0.19 up off the array's top right contact, 0.2762 corner to corner | silent | silent (finding 4) |
| (d) | a 4 × 4 with its own 0.275 row gap *and* a 17th contact 0.275 beside it | 1 | 5 |

gdscheck's marker in (d) reads `4×4 contact array (2.27×1.76 µm)` - a bounding box 2.27
wide, which only the extra contact can make (the 4 × 4 alone is 1.78).  So the extra
contact *is* in the cluster gdscheck found; its gap is simply never measured.

Verdict: a false negative in gdscheck, distinct from finding 2: the array's own grid pairs
are measured (one of them), a pair involving a cluster member off that grid is not.

### 4. A corner-to-corner pair inside an array is not read (CO.2b), in either tool

Structure (c) of `CO.2b.h2`: a contact set 0.20 across and 0.19 up from the array's top
right contact.  The two are 0.2762 apart corner to corner - over CO.2a's 0.25 and under
CO.2b's 0.28 - and the sized-and-merged closure puts the extra contact in the cluster
(their 0.16-grown boxes overlap by 0.12 in each direction).  Both tools are silent.

The contact rule is `space(0.28, euclidian)` upstream, with no projection condition at all -
unlike the via rule, which carries `projecting >= 0.26` and would exempt this pair.  Width
and space are euclidian including the chord across a corner (SPEC, settled), so 0.2762 is a
CO.2b violation.

Verdict: I believe the manual says it should fire, and say so although both tools are
silent.  KLayout's silence has its own cause - the diagonal bump makes the closure narrow,
which its `loc_exc` guard can reach - and is not evidence for gdscheck.  Low priority: it
cannot be reached until finding 3 is fixed, since the pair involves a cluster member off
the grid.

### 5. A space of nothing between a poly contact and COMP is not reported (CO.8)

*CO.8*, "Space from Poly2 contact to COMP 0.17".  A shared edge is a space of nothing and
is reported (settled in the GF180 rounds).  Two layouts put a contact on poly with one of
its edges lying exactly on a COMP edge; gdscheck is silent on both and KLayout fires.

| layout | geometry | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `CO.10.h1` (c) | a contact on the poly finger whose bottom edge is the COMP's top edge, at y = 4.0, x 15.99-16.21 | silent | fires, edge pair `(16.21,4;15.99,4)/(15.82,4;16.38,4)` |
| `CO.11.h1` (d) | a contact half on a COMP and half on a poly that abut at x = 4.0 | silent | fires, edge pair `(4,8;4,8.22)/(4,8.39;4,7.83)` |

Both layouts also carry what gdscheck does report - CO.10 for the contact on the gate in
the first, CO.3 and CO.4 for the contact crossing both edges in the second - so the CO.8
marker is the only difference.

Verdict: a false negative in gdscheck; `min_space` between two regions that share an edge
gives nothing where the settled reading and KLayout both give 0.

### 6. CO.5a / CO.5b have no "crossing" half (both tools)

*CO.5a*, "Nplus overlap of contact on COMP (only for contacts to butted Nplus and Pplus
COMP areas) 0.1"; CO.5b the same for Pplus.  `CO.9.h1` (a) centres a contact on the butting
edge of a butted N+/P+ COMP: it has 0.11 of N+ overlap on one side and **nothing** on the
other, and the same for P+.  Both tools report CO.9 alone.

CO.3 and CO.4 each carry a `forbidden` half in the deck for exactly this shape - a contact
that overlaps the layer and runs outside it, which no margin describes - and CO.5a / CO.5b
do not; upstream's `co5a_l1 = co_ncomp_check.enclosed(co_5a_ncomp_butted, 0.1, euclidian)`
has no `.or` half either.

Verdict: a false negative, but a shadowed one: any contact that crosses the N+/P+ boundary
is already CO.9, and any contact that crosses the COMP's outer edge is already CO.4.  The
case is written as the manual reads (CO.5a, CO.5b and CO.9 for structure (a)); fixing it is
low priority.

### 7. CO.9 reads "interacting", so a contact abutting the butting edge fires (both tools)

*CO.9*, "Contact on NCOMP to PCOMP butting edge is forbidden (contact must not straddle
butting edge)".  `CO.9.h1` (b) puts a contact wholly on the P side with its left edge lying
on the butting edge.  It straddles nothing, and the manual's parenthesis is about
straddling; both tools fire CO.9 (gdscheck at (5.11, 6.70), KLayout the polygon
`(5,6.59;5.22,6.81)`).

The layout is illegal anyway - the contact is 0 from the N+ implant and both tools report
CO.5b for it - so the reading costs nothing in practice.  The case keeps the tools' agreed
answer.  Recording it here only so nobody re-derives it: **a contact touching the butting
edge is CO.9**, as upstream's `contact.interacting(ncomp.edges.and(pcomp.edges))` says.

## Notes, not findings

1. **CO.9's butting edges are the raw coincident ones.**  `ncomp_pcomp_butting_edges` is
   `ncomp.edges and pcomp.edges`, where DF.11 uses `df11_butting_edges`, the same less
   `comp.edges`.  Where the two implants *overlap* over a COMP (`CO.5.h2` (a)), the
   implants' regions share a stretch of the COMP's own top and bottom contour, and those
   stubs are butting edges for CO.9.  A contact interacting one of them would have to reach
   the COMP boundary, which is already CO.4, so nothing is reachable through it today.
   gdscheck and upstream agree, and neither fires on `CO.5.h2`.
2. **"< 0.34 µm" is strict in both tools.**  Upstream writes `metal1.width(0.34.um +
   1.dbu)` for CO.6a, which looks like it means "≤ 0.34", but the cap it then selects is
   `metal1.edges.with_length(nil, 0.34.um)`, which excludes an edge of exactly 0.34 - and a
   straight track's cap edge is as long as the track is wide.  `CO.6a.h1` confirms it: a
   0.34 µm track with its cap 0.055 past the contact is clean in gdscheck *and* in KLayout,
   while the 0.33 and 0.30 tracks beside it fire in both.  gdscheck's `max_width: 0.34`,
   read exclusively, matches the manual's "Metal1 (< 0.34µm)" and the runset's behaviour.
3. **CO.6's third item has no recommended-deck counterpart.**  The manual's CO.6 III,
   "Minimum Metal1 overlap of contact on all sides for minimum contact resistance variation
   (guideline) 0.12", is the contact twin of V*n*.3 III and V*n*.4 III, which `show-deck`
   does carry, in `via_recommended` as `V<n>.3.3` / `V<n>.4.3`.  There is no
   `contact_recommended` deck and no `CO.6.3` rule.  All three are Appendix B "rules not
   coded", so this is a consistency question for the `recommended` suite, not a missing
   mandatory rule.

## Tested and clean

Everything below was drawn, run at three tile sizes and through the runset, and both tools
gave the answer the manual asks for.  No need to redo it.

- **CO.1** at the bound in both directions: 0.22 square clean, 0.225 tall, 0.225 wide,
  0.215 wide and 0.215 tall each two markers (`CO.1.h1`, 8 / 8).  A contact that is not a
  square (`CO.1.h2`): two abutting squares merging into a 0.44 × 0.22 bar fires, an L with
  0.22 arms fires (gdscheck four markers, one per 0.44 run; KLayout two, one per 0.44
  edge), and one 0.22 square drawn as two overlapping boxes is clean.
- **CO.2b's threshold**: a 4 × 4 at 0.275 is an array; twelve contacts in three columns are
  not; sixteen contacts in a single row are not, although sixteen is a 4 × 4's count
  (`CO.2b.h1`).  A contact beside a 3 × 3 takes CO.2a's 0.25, not 0.28 (`CO.2b.h2` (b)).
- **CO.3 / CO.4** at 0.07 (clean), 0.065 and flush; the contact the layer's edge cuts
  (CO.3 or CO.4 for crossing, CO.11 for the part on field oxide); the contact wholly
  outside with its edge on the layer's, which is CO.11 alone and not an enclosure at all -
  both tools agree, markers at (4, 3.4) and (9, 3.4) in `CO.3.h2` / `CO.4.h2`; the SRAMCORE
  marker exempting a 0.065 margin.
- **CO.5a / CO.5b** at 0.1 (clean) and 0.095 on either side of the butting edge; 0.095 from
  a side of the N+ COMP that is *not* the butting edge, which fires because the rule is the
  implant's overlap of the contact on every side, while CO.4's 0.07 is satisfied
  (`CO.5.h1`, three markers, both tools).  Implants that overlap by 0.2 and implants 0.2
  apart are not butted, and 0.095 is clean under both (`CO.5.h2`).
- **CO.6** at 0.005 (clean), flush, absent altogether and covering half the contact - the
  last three all fire and the first does not (`CO.6.h1`, 3 / 3).
- **CO.6a**: tracks 0.34 (not a narrow line), 0.35, 0.33 and 0.30 wide with the cap 0.055
  past the contact, and 0.34 with a 0.06 cap (`CO.6a.h1`, 2 / 2; see note 2).  The note's
  branch length: a 0.22 µm branch off a wide plate is not a line end, 0.24 and 0.30 are
  (`CO.6a.h2`, 2 / 2).
- **CO.6b**: the trigger is "< 0.04", so 0.035 triggers and 0.04 does not; the adjacent
  side is clean at 0.06 and fires at 0.055; two adjacent short sides fire; two *opposite*
  short sides with generous sides between them are clean (`CO.6b.h1`, 2 / 2).
- **CO.7** at 0.15 (clean) and 0.145; a poly line 0.10 from the contact that runs beside
  the COMP and never over it is no gate and is clean; OTP_MK exempts, both over the whole
  cell and over the gate alone (`CO.7.h1`, 1 / 1).
- **CO.8** at 0.17 (clean) and 0.165, and 0.165 from a COMP that the same poly crosses
  elsewhere - the rule measures to COMP whether or not the poly is a gate there
  (`CO.8.h1`, 2 / 2).
- **CO.9**: a straddling contact fires; a contact across implants that overlap by 0.2,
  where no edges coincide and there is no butting edge, is clean, and so is CO.5
  (`CO.9.h1`).  Butting edges on x = 20, 21, 40 and 42 all fire, at every tile size
  (`CO.9.h2`, 4 / 4).
- **CO.10**: a contact on the gate fires; the same contact 0.17 up the poly, clear of the
  COMP, is clean; RES_MK over the whole poly-COMP overlap takes it out of the gate layer
  and the contact on it is clean in both tools (`CO.10.h1`).
- **CO.11**: a contact on neither poly nor COMP fires; one on poly alone is a legal poly
  contact; one 0.005 off the COMP's edge fires on the sliver, with CO.4 for crossing; one
  shared by an abutting COMP and poly has no field oxide under it and CO.11 is silent
  (`CO.11.h1`).
- **Tile invariance**: 21 layouts × tiles 20, 7 and 100, no rule's count moved.
