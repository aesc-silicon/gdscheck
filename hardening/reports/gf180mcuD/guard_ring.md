<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / guard_ring: hardening report

Deck `guard_ring` against the GF180MCU design manual, section 12.4 (GR.1 to GR.11,
`gf180mcu_drm/drm_12_4.txt`), read with the scribe line and die seal chapter those rules
sit in (`drm_12.txt`, `drm_12_1.txt`, `drm_12_2.txt`, `drm_12_3.txt`), and with the
derived layers of `pdks/gf180mcuD/pdk.yml`.  17 layouts,
`tests/data/gf180mcuD/generated/guard_ring/GR.<n>.h<n>.gds.gz`, drawn by the `hardening`
half of `gen/gf180mcuD/guard_ring.rs`, each with a `#[case]` in the
`hardening_guard_ring` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck
at tiles 20, 7 and 100 and through GlobalFoundries' own KLayout runset
(`DECKS=guard_ring hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

Every fixture is a ring, because the rule is about one: a rectangular annulus written as a
keyhole polygon, with the marker, the active and the implant on one outline, the five
metals inside it and a pad on the band.  Its own numbers are the rules' bounds met
exactly - 16 µm of active against GR.6's minimum and 12 µm of metal against GR.4's - so a
fixture that does not say otherwise is a ring the whole section is content with, and the
perturbation is the entire layout.  The outline starts at 19, which puts the band across
the tile lines at 20, 21 and 28 and the hole's walls at 35, so that a die shape 9.995 µm
off one of them has its gap across x = 35, x = 40 and x = 42 at once.

## What the runset computes

`rule_decks/guard_ring.rb` has two registrations, both tagged `all`, so a default run
evaluates both:

| rule | manual | runset | gdscheck |
| --- | --- | --- | --- |
| GR.1 | Min/Max GUARD_RING_MK overlap of guard ring comp: 0 | `guard_ring_mk.not_in(comp.interacting(guard_ring_mk))` | two `forbidden` layers, `gr1_mk_viol` and `gr1_comp_viol` |
| GR.2 | Min space to prime die COMP, NWELL, Poly2, Metal 1-5 and metal Top: 10 | `<layer>.separation(guard_ring_mk, 10.um)`, eight layers | eight `min_space` rules |
| GR.3 | Minimum Pplus overlap of PCOMP inside guard ring: 0 | `guard_ring_comp.not_in(pplus)` | `forbidden` on `gr3_viol` |
| GR.4 | Minimum metal-n width (n = 1 to 6): 12 | `metal.not_outside(guard_ring_mk).width(12.um)`, per level | five `min_width` rules |
| GR.5 | Min metal-n outer edge space to die edge: 3 / **-** | **not computed** | **not in the deck** |
| GR.6 | Min PCOMP width: 16 | `comp.not_outside(guard_ring_mk).width(16.um)` | `min_width` on `pcomp_in_guard_ring` |
| GR.7 | Minimum Contact to Contact spacing: 0.7 | **not computed** | `min_space` on `contact_in_guard_ring` |
| GR.8 | Minimum via to via spacing: 0.7 | **not computed** | four `min_space` rules, Via1 to Via4 |
| GR.9 | All Metal & Vias shall exist in the seal ring area; all Vias in a staggered row | **not computed** | not in the deck |
| GR.10 | There must be rows of contact in the seal ring area | **not computed** | not in the deck |
| GR.11 | Pad opening on top of GUARD_RING_MK layer | `guard_ring_mk.not_interacting(pad)`, in the `wedge` registration | `forbidden` on `guard_ring_no_pad` |

So the oracle is a second opinion on GR.1, GR.2, GR.3, GR.4, GR.6 and GR.11, and on
**GR.5, GR.7, GR.8, GR.9 and GR.10 it is silent**.  That silence is not agreement: on
GR.7 and GR.8 - which gdscheck does implement and the runset does not - the manual is the
whole argument, and the fixtures below are scored against it alone.

## Coverage

`show-deck --process gf180mcuD --deck guard_ring` lists 23 rules: GR.1 twice, GR.2 eight
times, GR.3, GR.4 five times, GR.6, GR.7, GR.8 four times and GR.11.

- **GR.1's "Min/Max" is two bounds and the deck has both.**  A minimum and a maximum that
  are both zero is a statement that the marker and the ring's active must be the same
  shape, and it breaks from either side.  `gr1_mk_viol` is the marker outside the active,
  `gr1_comp_viol` the active outside the marker; `GR.1.h1` breaks one each and both are
  reported.  (The runset gets both directions out of one `not_in`, which is a shape
  identity test, and reads the same 2.)
- **GR.2's layer list is complete.**  COMP, NWELL, Poly2, Metal1, 2, 3, 4, 5 - eight
  members, and "metal Top" is Metal5 here, because variant D's stack ends there
  (`top_metal` is a single-source union of `metal5` in `pdk.yml`).  The runset says the
  same: its loop is `next unless ctx.metal_level_numerical >= lvl`, so level 6 (`metaltop`)
  never registers, and the marker it produces for the top of the stack reads
  `GR.2 : Min GUARD_RING_MK space to prime die Metal5: 10`.  `GR.2.h3` and `GR.2.h4` put
  one block of each of the eight 9.995 µm from the marker and read eight violations
  between them, one per layer, named.
- **GR.4 is per metal level and there are five of them.**  `GR.4.h1` puts every level's
  band 0.005 under the value and reads all five.
- **GR.8 is per via level and there are four.**  Variant D has no Via5; `GR.8.h1` reads
  Via1 to Via4.
- **GR.9 and GR.10 are starred in the manual** and the section's own note says so twice
  over ("Vias in staggered row is not checked by DRC deck", "Rows of contact is not
  checked by DRC deck"); Appendix B repeats it.  Not a gap.
- **GR.5 is absent, and that follows from which column the deck implements.**  The
  manual's table has two: Solder Bump and Other Cases.  GR.5 reads 3 under Solder Bump and
  **"-"** under Other Cases; GR.11 reads "Not allowed" under Solder Bump and "Required"
  under Other Cases.  gdscheck's GR.11 fires on a marker *without* a pad - `guard_ring_no_pad`
  is `guard_ring_mk not_interacting pad` - so it is the "Required" reading, the Other
  Cases column, and the runset agrees by putting GR.11 in a registration called
  `guard_ring_wedge`.  Under that column GR.5 has no value to check.  `GR.5.h1` draws the
  ring's metal 2.995 µm in from the marker's outer edge, which is GR.5 broken on all five
  levels under the other column, and the layout is correctly clean.
  What is missing is not the rule but the statement: nothing in the deck says it is the
  Other Cases variant, and a design going to solder bump needs GR.5 at 3 µm on five levels
  and GR.11 with the opposite sign.  The deck's header says only "Not ported: GR.5, GR.9,
  GR.10", which reads like three gaps where there is one variant choice and two starred
  rules.  Worth a sentence there, and a second variant of the deck if bumped parts are
  ever in scope.

## Findings

### 1. A die shape whose wall is the marker's wall is a space of nothing, and gdscheck does not read it

*GR.2*, "Min GUARD_RING_MK space to prime die COMP, NWELL, Poly2, Metal 1, 2, 3, 4, 5 and
metal Top: 10".

`GR.2.h2` (a) is a Metal1 block in the die whose left wall is exactly the marker's hole
wall at x = 35, from y = 50 to y = 64.  `GR.4.h2` (a) is the same thing as a 2 µm trace.
There is no space at all between the die metal and the guard ring marker, and the rule
asks for ten microns of it.

| layout / structure | geometry | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- |
| `GR.2.h2` (a) | a 14 µm Metal1 block, left wall on x = 35 | GR.2 | silent | fires, `(35,50;35,64)/(35,74;35,40)` |
| `GR.4.h2` (a) | a 2 µm Metal1 trace, left wall on x = 35 | GR.2 | silent | fires, `(35,50;35,52)/(35,62;35,40)` |

KLayout prints the edge pair with both edges on x = 35 - a separation of zero, which it
reports.  gdscheck reports nothing on either.

Verdict: a false negative in gdscheck, and the settled reading says so - "a shared edge is
a space of nothing (`abutting: report`)".  This is the class the efuse report records as
its finding 1, on GR.2's layers.  The runset is on the manual's side here.

### 2. A die metal plate lying across the marker is reported by nothing

Still GR.2.  `GR.2.h2` (b) is a 20 µm Metal1 square whose left edge is at x = 33, so it
crosses the marker's hole wall at x = 35 and sits 2 µm deep on the guard ring.  It is 20 µm
across, so GR.4 has nothing to say about it however the marker is read to reach, and it is
not COMP, so GR.1's coincidence never looks at it.

| layout | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `GR.2.h2` (b) | GR.2 | silent | silent |

Both tools read GR.2 as a rule about a gap, and where the two layers overlap there is no
gap to measure.  The consequence is that the section as coded has nothing to say about the
worst version of what GR.2 exists to prevent: a die plate not ten microns from the ring
but *on* it.  The 10 µm in the table is a minimum, and a clearance of −2 µm does not meet
it.

Verdict: a hole in the rule as both tools code it.  gdscheck is no worse than the runset,
and the fix is a reading rather than a bug - either GR.2 measures overlapping pairs as a
clearance of zero (which would also settle finding 1), or the deck needs a `forbidden`
half for a die layer intersecting the marker, the way the enclosure rules of other decks
carry a crossing half.  Recorded here so that whoever fixes finding 1 knows this sits
behind it.

### 3. A pad that only touches the marker is accepted as a pad opening on it

*GR.11*, "Pad opening **on top of** GUARD_RING_MK layer: Required".

`GR.11.h1` draws three 60 × 50 µm rings 30 µm apart: the first with its pad on the band,
the second with no pad at all, the third with a pad drawn just outside the marker, sharing
its outer edge.

| ring | pad | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- |
| 1 | on the band | clean | clean | clean |
| 2 | none | GR.11 | fires | fires |
| 3 | abutting the marker's outer edge | GR.11 | silent | silent |

Both tools select the marker with an `interacting` test - gdscheck's
`guard_ring_no_pad` is `not_interacting`, the runset's is `guard_ring_mk.not_interacting(pad)` -
and touching counts as interacting.  The manual's phrase is "on top of", and the rule's
purpose, from 12.3.3, is that the ring can be reached by a down bond: a pad lying beside
the ring with no opening over it is exactly the layout the rule forbids, and it passes.

Verdict: a false negative, shared with the runset.  A boundary reading rather than a
measurement, and the smallest of the findings here, but the rule is a yes/no question
about one shape lying on another and the answer on a shared edge is no.

### 4. KLayout's GR.3 asks for the implant to be the ring's active exactly; the manual asks for a minimum

*GR.3*, "**Minimum** Pplus overlap of PCOMP inside guard ring: 0".

`GR.3.h1` draws the implant 1 µm larger than the ring's active on both edges of the band,
so the active is covered everywhere with a micron to spare.

| layout | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- |
| `GR.3.h1` | clean | clean | fires, `polygon: (19,19;19,139;179,139;179,19/35,35;163,35;163,123;35,123)` - the whole comp annulus |

The runset's rule is `guard_ring_comp.not_in(pplus)`, and `not_in` is a shape identity
test: it keeps every polygon of the first layer that is not *equal* to a polygon of the
second.  The comp annulus and the wider implant annulus are different shapes, so the comp
ring comes out whole.  The same operator is right for GR.1, whose minimum *and* maximum are
zero, and wrong for GR.3, which only has a minimum.

Verdict: gdscheck is right.  Its `gr3_viol` is `comp_in_guard_ring` less `pplus`, which is
empty when the implant covers the active and is the uncovered strip when it does not
(`GR.3.h2`, 1 marker in both tools).  A ring whose P+ is drawn generously is legal and
common, and the runset flags every one of them.

### 5. GR.6 measures PCOMP, which is what the rule names; KLayout measures COMP

*GR.6*, "Min **PCOMP** width: 16".

`GR.6.h2` draws the ring's COMP 16 µm wide with the implant covering only 15.995 of it, so
the COMP meets the bound and the P+ active does not.  `GR.3.h2` is the same divergence
seen from the implant's side.

| layout | COMP width | PCOMP width | manual | gdscheck @20/7/100 | KLayout |
| --- | --- | --- | --- | --- | --- |
| `GR.6.h2` | 16.000 | 15.995 | GR.3 + GR.6 | GR.3 1, GR.6 2 | GR.3 1, GR.6 **0** |
| `GR.3.h2` | 16.000 | 15.995 all round | GR.3 + GR.6 | GR.3 1, GR.6 8 | GR.3 1, GR.6 **0** |

The runset's GR.6 is `comp.not_outside(guard_ring_mk).width(16.um)` - plain COMP.
gdscheck's is `pcomp_in_guard_ring`, the intersection of COMP and Pplus overlapping the
marker.

Verdict: gdscheck is right, and the manual is unambiguous - the rule says PCOMP, the
section's table in 12.2 lists the ring's active as PCOMP, and GR.3 exists precisely to
make sure that active is P+.  A ring whose implant falls short is narrow where it counts,
and KLayout reports only that the implant is short, not that the ring is thin.  The two
rules do travel together - no drawing separates them, which is why the pair is recorded in
`tests/gf180mcuD.rs` - but they say different things and only gdscheck says the second.

## Read and left alone

1. **Marker granularity.**  gdscheck's `min_width` gives one marker per wall, so a narrow
   bar is two; KLayout gives one edge pair.  `GR.4.h1` is 10 against 5 (five levels, one
   narrow bar each), `GR.6.h1` 2 against 1.  A corner-to-corner space is one violation in
   gdscheck and two edge pairs in KLayout (`GR.2.h1`: 2 against 3, KLayout's extra being
   the second pair of the same corner, `(73,50;73,50.141)/(66,43;66,42.859)` and
   `(73.141,50;73,50)/(65.859,43;66,43)`).  Counts judged by violating structures
   throughout.
2. **A trace that only butts against the marker is not ring metal, and both tools agree.**
   `GR.4.h2` (a) is a 2 µm Metal1 trace whose left wall is the marker's hole wall.  The
   manual says the rules are checked "for the real ring area recognized by
   GUARD_RING_MK", and a trace outside the marker is not that area.  gdscheck selects with
   `overlapping` and the runset with `not_outside`, and neither picks it up - GR.4 is
   silent in both.  (What it is not clean of is GR.2; that is finding 1.)
3. **A trace that starts on the ring's own metal is ring metal, and drags GR.4 onto its
   width.**  `GR.4.h2` (b) is a 2 µm Metal2 trace starting at x = 29, inside the ring's own
   Metal2 band, and running out through the marker into the die; both tools report it.
   Section 12.3.2 says "The guard ring metal may be butted with the Vss bus" and sets the
   bus's width by current density rather than by GR.4 - so a design that does exactly what
   12.3.2 permits gets a GR.4 flag it has to waive.  Left alone, on two grounds: chapter 12's
   guidelines are in Appendix B as not coded ("SCRIBE LINE & GUARD RING GUIDELINES (Except
   Guard Ring design rule check)") while 12.4 is the check, and cutting the merged region
   at the marker's edge to spare the bus would invent walls where the rule names a region.
4. **A shape outside the ring is measured like the die's.**  `GR.2.h2` (c) is a Metal1
   block 9.995 µm beyond the ring's *outer* wall - the scribe-line side, not the prime die
   the rule names - and both tools report it.  Neither can tell inside from outside without
   a die boundary, and "prime die" is context rather than geometry.  The reading is right
   for any real layout, where what lies outside the seal ring is the saw street.
5. **The ring's own material never fires GR.2.**  The marker, the active and the implant
   are coincident and the metals lie inside the marker; nested and coincident shapes are
   not a gap, and no fixture with a correct ring produces a GR.2 marker from the ring
   itself.  `GR.5.h1`, whose metal is inset 2.995 µm inside the marker on both edges, is
   the strongest version of that and is clean.
6. **GR.7 and GR.8 are gdscheck's alone.**  The runset computes neither, so every count
   below is scored against section 12.4's table and section 12.2's contact and via
   coordinates and nothing else.

## Tested and clean

Everything below was drawn, run at three tile sizes and through the runset, and gdscheck
gave the answer the manual asks for.  No need to redo it.

- **GR.1, both bounds** (`GR.1.h1`): the marker 0.005 inside the active's edge and 0.005
  outside it, one ring each, 2 markers, KLayout 2.
- **GR.1, one shape drawn many ways** (`GR.1.h2`): the marker as one keyhole annulus, the
  active as the eight pieces a real ring is drawn as, the implant as four overlapping bars
  stretched over the corners - the same region three times, and both tools silent.
- **GR.2, both metrics** (`GR.2.h1`): measured off a 16 µm bump the ring turns into the
  die, so that a corner-to-corner pair has no facing wall at all.  9.8995 µm as the crow
  flies fires, 10.0056 is clean, a wall at exactly 10.000 is clean and one at 9.995 fires.
  A space is euclidian in both tools.
- **GR.2, every layer it names** (`GR.2.h3`, `GR.2.h4`): Metal1 and Metal5 in the first,
  COMP, NWELL, Poly2, Metal2, Metal3 and Metal4 in the second, each one block at 9.995 -
  eight violations, eight layer names, gdscheck and KLayout identical.  Metal1 at exactly
  10.000 in the same layout is clean.
- **GR.3** (`GR.3.h2`): the implant 0.005 short of the active on the outer edge all round,
  1 marker in both tools.  Its GR.6 companion is ignored in the case; the pair is recorded
  in `tests/gf180mcuD.rs`.
- **GR.4, every level** (`GR.4.h1`): all five bands 11.995 wide on the bottom bar, 10
  markers (two per narrow bar), KLayout 5.
- **GR.6, the bound** (`GR.6.h1`): the active, the implant and the marker together 15.995
  on the bottom bar, 2 markers.
- **GR.6, the hole is not a width** (`GR.6.h3`): a ring 46 µm square with a 16 µm band, so
  its hole is 14 µm across - under the value and empty.  Both tools silent, which is the
  answer: a width is a width of material, and the walls of a ring's hole face each other
  across nothing.
- **GR.7, the ring's contacts** (`GR.7.h1`): 0.700 apart clean, 0.695 fires; a *staggered*
  pair offset 0.42 along x and 0.55 along y, which face each other over nothing at all and
  whose only distance is the 0.6920 µm diagonal, fires - section 12.2 puts the ring's
  contacts "in staggered formation with separation of 0.7µm", so the diagonal is the
  separation that counts; the same pair at 0.50 and 0.50, 0.7071 apart, is clean; a 0.695
  pair with its gap across the tile line at x = 21 fires; and a pair 0.3 µm apart out in
  the die, where the ring's rule does not reach, is clean.  3 markers.
- **GR.8, every via level** (`GR.8.h1`): one 0.695 pair on each of Via1 to Via4, 4 markers,
  and a Via1 pair out in the die clean.
- **GR.11** (`GR.11.h1`): a marker with a pad on its band is clean and one without fires,
  per region - three rings in one layout and only the ones that want a pad are named.
- **Tile invariance**: 17 layouts × tiles 20, 7 and 100, every rule's count identical; the
  oracle printed no `TILE-DEPENDENT` on any of them.  The ring's band lies across x = 20,
  21 and 28, its hole's walls at 35, and the 9.995 µm gaps of `GR.2.h3` run across x = 35,
  40 and 42 - a 20 µm line, a 7 µm line and both again - on purpose.

Test status on the engine as of this report: 3 of the 17 new cases fail - `GR.2.h2` and
`GR.4.h2` on finding 1 (and `GR.2.h2` on finding 2 as well), `GR.11.h1` on finding 3.  The
other 14 pass, as do the 16 older guard_ring good/bad pairs and the seal-ring test.
