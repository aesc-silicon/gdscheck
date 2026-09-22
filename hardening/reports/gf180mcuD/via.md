<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / via: hardening report

Deck `via` against the GF180MCU design manual, section 7.14 (V*n*.1 to V*n*.5,
`gf180mcu_drm/drm_07_15.txt`), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  15
layouts, `tests/data/gf180mcuD/generated/via/V<n>.<rule>.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/via.rs`, each with a `#[case]` in the `hardening_via`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through GlobalFoundries' own KLayout runset
(`hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

`show-deck` lists 40 rules: V*n*.1, V*n*.2a (space and notch), V*n*.2b, V*n*.3a (Via1) or
V*n*.3b, V*n*.3c, V*n*.3d, V*n*.4a, V*n*.4b and V*n*.4c for *n* = 1 to 4, plus the eight
guideline rules of `via_recommended` (V*n*.3.3 and V*n*.4.3, the manual's 0.12 "all sides"
item, Appendix B "not coded").  Nothing in the section is missing:

- **There is no V5.** Variant D's stack ends at Metal5, so Via5 (Metal5 to MetalTop) does
  not exist in it.  The runset says the same - each level's rules run only
  `if ctx.metal_level_numerical > lvl`, which is `5 > lvl`, so it evaluates Via1 to Via4 -
  and the deck's title, "Via 1-4", matches.  (The brief's "the deck's V5 rules are the
  variant's top via" does not apply: there are none, correctly.)
- **V*n*.5**, "Vian stack (2) over contact is permitted", is a permission, not a check.
  `V1.5.h1` draws a contact with Via1, Via2 and Via3 on one centre and the deck is silent,
  which is the right answer.
- Section 7.14 has no 3.3 V / 5 V split, so no rule here is `_LV` / `_MV`, and `show-deck`
  correctly has none.

Marker cuts: gdscheck reports two `exact_width` markers per via that is off in one
direction (one per wall), one per deficient side for an enclosure, one per pair for a
space; KLayout one edge per `without_length`, a polygon per enclosure failure, one edge
pair per space.

No count moved with the tile size: 15 layouts × 3 tiles, every rule identical.
`V1.2b.h4` puts its arrays on x = 20, x = 42 and y = 21 on purpose - the cluster that makes
a group an array is derived across the cut, and it survives.

Test status on the engine as of this report: 4 of the 15 new cases fail
(`V1.2b.h1`/`h2`/`h3`/`h4`), all on findings 1 and 2, which are one and the same code path.
The other 11 pass, as do the 44 older via cases.

## Findings

### 1. V*n*.2b reports at most one violating pair per array

*V1.2b*, "Space in 4x4 or larger Via1 array 0.36".  `V1.2b.h1` draws a 4 × 4 array at 0.38
whose third row gap is 0.355: four distinct via pairs, one per column, each 0.355 apart.
gdscheck prints exactly one marker, KLayout four.

| layout | pairs drawn | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `V1.2b.h1` | 4 (one array) | 1 | 4 |
| `V1.2b.h4` | 12 (three arrays, on the tile lines) | 3 | 12 |

gdscheck's marker in `V1.2b.h2` reads `4×4 via1 array (2.79×2.16 µm): space 0.3550 µm <
0.36 µm at (10.1300, 10.9000)-(10.1300, 11.2550)` - the first column's pair of the one
array in that layout with a deficient internal gap.

Four via pairs in four different columns are four violating structures, and a second,
independent deficiency elsewhere in the same array is invisible.  This is the same defect
the contact report records as its finding 2, on the same check.

Verdict: under-reporting in gdscheck; one marker per array, however many pairs are short.

### 2. A via that belongs to the array's cluster but not to its grid is never measured

Still V*n*.2b.  Upstream sizes the vias by 0.2, merges and shrinks back, and applies 0.36
to every via *interacting* the cluster, so a via within 0.4 of the array is in the array.

| layout / structure | geometry | gdscheck | KLayout |
| --- | --- | --- | --- |
| `V1.2b.h2` (a) | a 17th via 0.355 right of a legal 4 × 4 at 0.38 | silent | fires |
| `V1.2b.h2` (b) | the same beside a 3 × 3 - no array, V1.2a's 0.26 applies | clean | clean |
| `V1.2b.h2` (c) | a 17th via 0.42 away - beyond the 0.4 merge reach, not in the array, and 0.42 clears 0.36 anyway | clean | clean |
| `V1.2b.h2` (d) | a 4 × 4 with its own 0.355 row gap *and* a 17th via 0.355 beside it | 1 | 5 |
| `V1.2b.h3` (a) | a 17th via 0.30 right of a legal 4 × 4, level with a row | silent | fires, edge pair `(5.48,3.64;5.48,3.9)\|(5.18,3.9;5.18,3.64)` |

gdscheck's marker in (d) reads `4×4 via1 array (2.79×2.16 µm)` - a bounding box 2.79 wide
where the 4 × 4 alone is 2.18, so the extra via *is* in the cluster gdscheck found.  Its
gap is simply never measured.

Verdict: a false negative in gdscheck, distinct from finding 1: the array's own grid pairs
are measured (one of them), a pair involving a cluster member off that grid is not.  This
is the same defect the contact report records as its finding 3.

### 3. V*n*.2b's "projecting >= 0.26" condition cannot be read until finding 2 is fixed

The via array rule is `selected.space(0.36.um, projecting >= 0.26.um)` upstream - it reads
a pair only where the two vias face each other over a full via side - where the contact
rule CO.2b is plain `euclidian` with no such condition.  gdscheck's deck carries the
condition as `projection: 0.26`.

Reaching it needs a pair that is *inside* an array and does not fully face: on a regular
grid every orthogonal pair faces over the whole 0.26, so the pair has to be a via off the
grid - which finding 2 makes invisible.  `V1.2b.h3` draws all three cases anyway, so they
are ready when finding 2 is fixed; KLayout's reading of them is recorded here:

| structure | facing | KLayout |
| --- | --- | --- |
| (a) 17th via 0.30 right of a row, level with it | 0.26 | fires |
| (b) the same one grid step (0.005) up | 0.255 | clean |
| (c) the same 0.15 up | 0.11 | clean |

So the condition is `>=`: a pair facing over exactly 0.26 is read, one facing over 0.255 is
not, and in (b) and (c) V1.2a's 0.26 is cleared by the 0.30 gap, leaving them legal.  The
case expects one V1.2b, which is (a).

Verdict: not a defect of its own - it is finding 2 seen from another side - but the
`projection` parameter is untested on this engine until that is fixed.

## Notes, not findings

1. **V*n*.3c and V*n*.4b can only be read alone at exactly 0.34 µm - which is not a narrow
   line.**  A 0.26 via in a track *w* wide has (*w* − 0.26) / 2 of margin above and below;
   V*n*.3d triggers when a side is under 0.04, i.e. whenever *w* < 0.34.  And a line end
   needs *w* < 0.34 (note 2).  The two conditions do not overlap, so every real end-of-line
   violation on a via comes with a V*n*.3d (or V*n*.4c) marker beside it.  `V1.3c.h1`,
   `V1.3c.h2` and `V1.4b.h1` therefore carry `V1.3d` / `V1.4c` in their ignore list; both
   tools produce exactly the same pair of counts on all three.  Worth knowing before anyone
   reads a V*n*.3c count as "the number of bad line ends".
2. **"< 0.34 µm" is strict in both tools.**  Upstream writes `lower_metal.width(0.34.um +
   1.dbu)`, which looks like "≤ 0.34", but the cap it then selects is
   `.edges.with_length(nil, 0.34.um)`, which excludes an edge of exactly 0.34 - and a
   straight track's cap edge is as long as the track is wide.  `V1.3c.h1` confirms it: a
   0.34 µm track with its cap 0.055 past the via is clean in gdscheck *and* in KLayout,
   while the 0.32 track beside it fires in both.  gdscheck's `max_width: 0.34`, read
   exclusively, matches the manual's "Metaln (< 0.34μm)" and the runset's behaviour.
3. **Via1's lower margin really is zero, and V*n*.3d still applies to it.**  The manual
   gives V*n*.3 as "0.00 (n = 1), 0.01 (2 ≤ n ≤ 5)", and `V1.3a.h1` shows what that costs:
   Metal1 flush with the via on one side, with 0.06 above and below, is clean, while
   Metal1 drawn as the via's own square - flush on all four sides - is clean for V1.3a and
   fires V1.3d, because every side then overlaps by less than 0.04 and no side reaches
   0.06.  Both tools agree.  Worth stating because "overlap ≥ 0" reads like "anything
   goes".

## Tested and clean

Everything below was drawn, run at three tile sizes and through the runset, and both tools
gave the answer the manual asks for.  No need to redo it.

- **V1.1** at the bound in both directions: 0.26 square clean, 0.265 tall, 0.265 wide,
  0.255 wide and 0.255 tall each two markers (`V1.1.h1`, 8 / 8).  A via that is not a
  square (`V1.1.h2`): two abutting squares merging into a 0.52 × 0.26 bar fires, an L with
  0.26 arms fires (gdscheck four markers, one per 0.52 run; KLayout two, one per 0.52
  edge), and one 0.26 square drawn as two overlapping boxes is clean.
- **V1.2b's threshold**: a 4 × 4 at 0.355 is an array; twelve vias in three columns are
  not (their bounding box is 1.54, under the 1.86 the rule's own edge length asks); sixteen
  vias in a single row are not, although sixteen is a 4 × 4's count (`V1.2b.h1`).  A via
  0.42 from a 4 × 4 is outside the cluster and takes V1.2a's 0.26 (`V1.2b.h2` (c)).
- **V1.3a**, the one level whose lower metal may sit flush: flush on one side with 0.06
  above and below is clean, the via 0.005 outside Metal1 fires, a via with no Metal1 at all
  fires, and Metal1 drawn as the via's own square fires V1.3d (`V1.3a.h1`, 2 × V1.3a +
  1 × V1.3d in both tools).
- **V2.3b**, the 0.01 every level above Via1 asks of the metal below: 0.01 clean, 0.005
  fires, flush fires - the margin Via1 is allowed and Via2 is not (`V2.3b.h1`, 2 / 2).
- **V1.4a**, the 0.01 of the metal above: 0.01 clean, 0.005 fires, no Metal2 at all fires
  (`V1.4a.h1`, 2 / 2).
- **V1.3c / V1.4b**: tracks 0.34 (not a narrow line), 0.35 and 0.32 wide with the cap 0.055
  past the via, and 0.34 with a 0.06 cap (`V1.3c.h1`, `V1.4b.h1`; see notes 1 and 2).  The
  note's branch length, which is 0.28 for a via where the contact's is 0.24: a 0.275 µm
  branch off a wide plate is not a line end, 0.28 exactly and 0.35 are (`V1.3c.h2`,
  2 × V1.3c in both tools).
- **V1.3d / V1.4c**: the trigger is "< 0.04", so 0.035 triggers and 0.04 does not; the
  adjacent side is clean at 0.06 and fires at 0.055; two adjacent short sides fire; two
  *opposite* short sides with generous sides between them are clean (`V1.3d.h1` 2 / 2,
  `V1.4c.h1` 1 / 1).
- **V1.5**, the permitted stack: a contact with Via1, Via2 and Via3 on one centre, every
  metal over them, and the deck silent (`V1.5.h1`).
- **Tile invariance**: 15 layouts × tiles 20, 7 and 100, no rule's count moved.
