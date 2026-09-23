<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / topvia1, topvia2: hardening report

Decks `topvia1` and `topvia2` against SG13G2 Layout Rules Rev. 0.4, sections 5.21
(TV1.a-TV1.d) and 5.24 (TV2.a-TV2.d), with section 6.10's sentence on EdgeSeal ("the
corresponding standard metal and via rules are not checked within EdgeSeal regions").
The two decks share their rule text with different values (0.42/0.42/0.10/0.42 and
0.90/1.06/0.50/0.50), so `gen/ihp_sg13g2/topvia.rs` draws each theme once for
both: 29 layouts per deck (`tests/data/ihp-sg13g2/topvia<n>/TV<n>.*.h<k>.gds.gz`), 58 in
all, each with a `#[case]` in the `topvia1`/`topvia2` tables of `tests/ihp-sg13g2.rs`.
Every layout ran through gdscheck at tiles 20, 7 and 100 and through IHP's KLayout decks
(`hardening/oracle-ihp.sh`).  TopVia is 90° only (section 3.1), so the 45° geometry
is the enclosing metal's.

`show-deck` lists TV<n>.a (`exact_width` on `TopVia<n>NoSealring`), TV<n>.b (`min_space`
on `TopVia<n>`), TV<n>.c and TV<n>.d (`min_enclosure` of `TopVia<n>NoSealring` by the
metal below and above): every rule of both sections.  The layer of TV<n>.b is finding 1.

Reading the numbers below: gdscheck reports one marker per wall of an off-size pair for
`exact_width` (two for a bar or a short square in one direction, four for a short
square), one per pair for a space, one per under-enclosed via for an enclosure when the
short walls are adjacent and one per wall when they are opposite.  KLayout's column is
the driver's topvia table (the maximal deck has no TV rules): TV.a is
`without_bbox_min` joined with `without_bbox_max`, so a short square counts twice there;
TV.b is one edge pair per pair (two or three for a corner-to-corner pair); TV.c/TV.d one
per edge pair; its `enclosed` never reports a via with no metal under it, and its
hierarchical run counts an array cell once.  TV1 and TV2 behaved identically on every
layout.  No count moved with the tile size in any layout, and every flat/array pair
agreed.

Test status on the engine as of this report: 6 of the 58 new cases fail (3 per deck),
all on the findings below; the other 52 pass.

## Resolution (2026-09-20)

Fixed, deck: 1 (every rule of the via and topvia decks - Via1-4, TopVia1-2, and
cmos5l's topvia1 - runs on `<Via>NoSealring`; TV.b and V.b/b1 had the drawn layer).

Open, cases carry the engine's answer: 2 - a 45° wall crossing over the via's wall
is no pair under the parallel-walls projection reading, the gatpoly report's finding
11 (a faithful reading of KLayout's angled projection was built and taken out again as
tile-dependent on core-cut pieces); the gdscheck owner's call, with the GF180 counts it
would move.

## Decided (2026-09-21, gdscheck owner)

2: the 45° wall over the via's wall fires, and the two controls cut under the value from the corner with it - enclosure is the closest approach (`metric: euclidian`), decided 2026-09-21.

## Findings

### 1. TV<n>.b is checked inside EdgeSeal (false positive)

Manual, section 6.10: "Please be aware that corresponding standard metal and via rules
are not checked within EdgeSeal regions."  The deck runs TV<n>.a, c and d on
`TopVia<n>NoSealring` and TV<n>.b on `TopVia<n>`; IHP's deck runs all four on
`topvia<n>_drw.not(edgeseal_drw)`.  (The via decks `via1`-`via4` carry the same
split.)

Layout `TV<n>.b.h6`: an EdgeSeal plate (1, 1)-(9, 9); under it two vias `s − 0.005`
apart at (2, 2); a via inside the seal against its edge x = 9 and one `s − 0.005`
outside it at y = 5; a pair `s − 0.005` apart outside at (12, 2).

- gdscheck @20/7/100: TV<n>.b 3 - (2.42, 2.0)-(2.835, 2.0) in the seal, (9.0, 5.0)-(9.415,
  5.0) across the edge, (12.42, 2.0)-(12.835, 2.0) outside.
- KLayout: TV<n>.b 1, "(12.835,2;12.835,2.42)/(12.42,2.42;12.42,2)", the outside pair.
- Verdict: the pair in the seal is not checked; the outside pair fires.  The pair across
  the edge is the seal's via against the design's - the via within EdgeSeal is not the
  rule's layer, so no pair, as the deck reads the section for the metals ("cut at the
  seal's edge like the vias") and as IHP's deck has it.  Expected 1 (`tv<n>_b_h6`).  In
  a real die the sealring's via ring and the design's vias are kept apart by Seal.b
  (4.90 to the seal's Activ), so the across-the-edge case is academic; the in-seal case
  is real for any seal ring drawn with two via rings.

### 2. An enclosure is not read under a 45° metal wall crossing over the via's wall (false negative)

Manual: "TV1.c  Min. Metal5 enclosure of TopVia1  0.10", "TV1.d  Min. TopMetal1
enclosure of TopVia1  0.42", "TV2.c  Min. TopMetal1 enclosure of TopVia2  0.50", "TV2.d
Min. TopMetal2 enclosure of TopVia2  0.50".

Layout `TV<n>.c.h2` / `TV<n>.d.h2`: the metal box's top-right corner is cut along
x + y = k.  (b) at (6, 2): the cut passes through the point e/2 above the via's
top-right corner - the metal over the right end of the via's top wall is e/2 and the
metal beside the top end of its right wall is e/2 (TV1.c: the chamfer from (7.02, 1.87)
to (5.87, 3.02), 0.05 over the via (6, 2)-(6.42, 2.42)); (c) at (14, 2) the same as a
long 45° wall (box margins 3); (e) at (2, 12) and (f) at (14, 12) the same cut at the
bottom-left corner.  Controls: (a) at (2, 2), the cut `e − 0.005` (euclidian) from the
corner, (e − 0.005)·√2 ≥ e over the wall, and (d) at (22, 2), the cut exactly `e` over
the corner - both clean by the settled projection reading.

- gdscheck @20/7/100: nothing, in all four decks' worth of values (TV1.c 0.10, TV1.d
  0.42, TV2.c 0.50, TV2.d 0.50).
- KLayout (`enclosed(..., euclidian)`): TV1.c at (b) "(6.329,2.42;6.42,2.42)/(6.37,2.52;
  6.47,2.42)" and "(6.42,2.42;6.42,2.329)/(6.42,2.47;6.52,2.37)", at (c) the same at
  14.329/14.42, at (e) "(2.091,12;2,12)/(2.05,11.9;1.95,12)" and its mirror, at (f) at
  14.091/14; TV2.c at (c) "(14.443,2.9;14.9,2.9)/(14.65,3.4;15.15,2.9)" and so on; plus
  (a) and (d), which its euclidian metric reports and the settled reading does not.
- Verdict: (b), (c), (e) and (f) fire.  This is not the settled corner case: there the
  axis-aligned margin of every point of both walls stays at or over the value and only
  the diagonal from the corner is short; here a stretch of the top wall e/2 long (0.05
  for TV1.c, 0.21 for TV1.d, 0.25 for TV2) has under `e` of metal above it, e/2 at its
  end, under the value by any reading.  A 45° wall over a via's wall is what a
  chamfered metal corner near a via looks like.
  Expected 4 (`tv<n>_c_h2`, `tv<n>_d_h2`); (a) and (d) stay clean.  The Cont and Via
  reports drew only the corner case, so the same miss is likely in Cnt.c/d/g2 and
  V(n).c/c1 with a 45° Activ, GatPoly or Metal wall.

## Notes that are not findings

- A. A via with no metal is TV.c/TV.d to gdscheck ("shape ... not enclosed") and
  nothing to KLayout (`enclosed` has no pair; there is no "TopVia on Metal" rule like
  Cnt.g to catch it there).  `TV<n>.c.h1` (34, 2), `h3` (22, 2, in a frame's hole),
  and the via 0.05 past the metal's edge at `h1` (30, 2) are reported by gdscheck
  alone; the manual's enclosure of such a via is negative or none, and the cases expect
  the rule.  The same shows in the TV.a and TV.b layouts, which draw bare vias and
  ignore TV.c/TV.d.
- B. Two vias sharing a corner (`TV<n>.a.h2` (20.5, 2)) are TV.b "space 0.0000" once to
  gdscheck, and to KLayout one merged 8-vertex polygon reported as TV.a (twice, bbox 2w)
  and TV.b (twice, the edges meeting at the corner).  The cont report's finding 6 settled
  the corner pair on one space marker; the case expects TV.b once and no TV.a.
- C. A via with a 0.005 × 0.2 notch in its top wall (`TV<n>.a.h2` (26.5, 2)) is TV.a to
  gdscheck (six walls: the 0.415 lip, the 0.1/0.12 arms) and TV.b to KLayout (its
  `space` reads the notch, its bbox is still w × w).  The manual's TV.b says "space", not
  "space or notch" as the metal rules do; the shape is not a w square and the case
  expects TV.a (six).  A via with a 0.005 bump is TV.a in both (gdscheck four walls,
  KLayout bbox max).
- D. The settled 45° reading, seen here: (a) and (d) of `TV<n>.c.h2`/`d.h2` (the cut
  `e − 0.005` euclidian from the corner with (e − 0.005)·√2 over the wall; the cut
  exactly `e` over the corner) are clean in gdscheck and reported by IHP's driver, which
  runs TV.c/TV.d as `enclosed(..., euclidian)`.  As in the cont report's note B; if the
  gdscheck owner wants via enclosure euclidian like IHP's deck, `tv<n>_c_h2` and `tv<n>_d_h2`
  gain two markers each.
- E. Counts.  TV.a: two per bar or per one-direction miss, four per short square, four
  for an L of three quarters, six for the notched square; KLayout one polygon per
  shape, twice for a short square.  TV.b: a 3 × 3 at `s − 0.005` is twelve pairs in both;
  a square in an L's inner corner is one marker to gdscheck and two edge pairs to
  KLayout.  TV.c/TV.d: a via short on all four sides is one marker to gdscheck and four
  edge pairs to KLayout; short left and right is two in both; a via on a 300 µm strip
  short top and bottom is two in both.

## Tested and found clean or correct (no need to redo)

Each item holds for TV1 and TV2 alike.

- TV.a: the value vs one step short and one step long in x and in y (two walls), a short
  square (four), a w × 2w bar, a 0.005 sliver, a 300 µm bar, a short square at (1000,
  1000); a w square drawn as two halves, two overlapping boxes, twice, four quarters,
  clockwise (clean); two abutting squares (a bar), an L, a notch, a bump (notes B, C);
  short squares on, straddling and starting on x = 20, straddling 21, on 40, straddling
  42, straddling y = 20 (four walls each), w straddling 20 clean; fifty flat and as
  GdsArrayRef; a short square under EdgeSeal and a w square within it on its edge
  (clean), a w square straddling the seal's edge (the outer half, two walls), a short
  square outside on the edge and one in an EdgeSeal frame's hole (four each).
- TV.b: `s − 0.005` vs `s` in x and y; corner to corner one grid step either side of
  s/√2; corner-on at `s − 0.005` vs `s`; a row of three (two); a 3 × 3 at `s` (clean) and
  at `s − 0.005` (twelve); (1000, 1000); a bar to a square, a square in an L's corner,
  two 300 µm bars (one each), two abutting squares (no pair); gaps on, straddling and
  starting on x = 20, straddling 21, on 40, straddling 42, straddling y = 20, `s`
  straddling 20 clean; fifty flat/array; the outside pair of the seal layout.
- TV.c/TV.d: `e` all round clean; `e − 0.005` right (one), all round (one), right and
  top (one), left and right (two); 0.005; 0 (via edge on the metal edge); a via 0.05
  past the edge; a bare via; (1000, 1000); the metal as two abutting boxes with the
  seam under the via, as a 4 × 4 grid, the via as two halves or two overlapping boxes
  (clean); overlapping metal boxes whose union is short, the metal drawn twice (once),
  a via on a frame's wall, a via in the hole, two via halves short (once each);
  margins straddling, ending on and beginning on x = 20, `e` straddling 20 clean, at 21,
  40, 42, y = 20; fifty flat/array; three vias on a 300 µm strip at `e` (clean) and at
  `e − 0.005` top and bottom (two each); the seal: `e − 0.005` all round and a bare via
  under EdgeSeal (clean), a via straddling the seal's edge with the outer piece short,
  a via outside on the edge, one at (12, 2) (one each).
- Not tested: TopVia2 under Pad (Pad.kR, recommended, section 6.9's), TopVia1 on a
  MIM (section 6.11's), the sealring's own via rings (Seal.c2/c3, section 6.10's).
