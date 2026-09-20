<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / nwell: hardening report

Deck `nwell` against SG13G2 Layout Rules Rev. 0.4, section 5.1 (NW.a-NW.f1) and section
8.1.1 (the DigiBnd variants).  69 layouts, `tests/data/ihp-sg13g2/nwell/NW.*.h<n>.gds.gz`,
drawn by `gen/ihp_sg13g2/nwell.rs` (`hardening`), each with a `#[case]` in the `nwell`
table of `tests/ihp-sg13g2.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through IHP's KLayout decks (`ci/hardening/oracle-ihp.sh`).

Reading the numbers below: gdscheck reports one marker per edge for `min_width` (two per
narrow bar) and one per violating pair otherwise; KLayout's column in the oracle adds the
driver's and the maximal deck's reports, so it is twice gdscheck's for the same logical
set (and its hierarchical run counts an array cell once).  No count moved with the tile
size in any layout, and every flat/array pair agreed.

Test status on the engine as of this report: 16 of the 69 new cases fail, all on the
findings below; the other 53 pass.

## Findings

### 1. NW.b has no notch half (false negative)

Manual: "NW.b  Min. NWell space or notch (same net) ... 0.62".  The deck runs `min_space`
only; the other "space or notch" rules of this PDK (Act.b, PWB.b, M1.b, ...) carry a
second `min_notch` entry, NW.b does not.

Layout `NW.b.h3`: a straight U notch and a straight-vs-45° notch at 0.615 (the helpers'
patterns, 0.62 controls beside them), a comb with three 0.615 slots, a 0.615 slot cut into
a plate, a keyhole-drawn ring with a 0.615 hole; plus two Ls facing across 0.615 and an
island 0.615 from a ring's inner wall (those two are separate shapes).

- gdscheck @20/7/100: NW.b 2 (the Ls and the island only).
- KLayout: NW.b 9 (all seven notch cases, some as two edge pairs), NW.b1 2 (the Ls and the
  island, which it takes for different nets - see note A).
- Verdict: the nine notches violate NW.b.  Expected 11 (`nw_b_h3`).

### 2. NW.b1 misses a gap of exactly 0.62 (false negative at the boundary)

Manual: NW.b merges regions "separated by *less than* this value"; two regions exactly
0.62 apart stay two regions, and on different nets NW.b1 (1.80) applies.

Layout `NW.b1.h1`, bare wells: gaps 0.62, 0.625, 1.795 (fire) and 1.80 (clean).

- gdscheck: NW.b1 at 0.625 and 1.795; nothing at 0.62 (the merged layer closes gaps up to
  and including 0.62 - the existing `NW.b` case even documents this).  NW.b does not fire
  either (0.62 is its limit), so the 0.62 pair is reported by nobody.
- KLayout: NW.b1 at 0.62, 0.625 and 1.795.
- Verdict: 0.62 fires NW.b1.  In `nw_b1_h1`.

### 3. NW.b1 is blind near corners: the merged layer's corners are beveled (false negative)

Layout `NW.b1.h1` also holds diagonal pairs at 1.28/1.28 (1.810, clean), 1.27/1.27
(1.796), 1.20/1.20 (1.697) and 0.90/0.90 (1.273); layout `NW.b1.h4` a diamond tip 1.795
from a wall (1.80 clean beside it) and two 45° strips 1.796 apart (1.803 clean).

- gdscheck: nothing for 1.796 or 1.697; the 0.90/0.90 pair is reported as "1.4538" and
  "1.6097" (two markers, one of them with a vertex at (12.28, 13.152), a point outside
  both boxes); the diamond tip is silent; the strips fire (1.7961).  The reported
  distances fit a merged layer whose corners are cut by 0.128 µm = 0.31·(√2 − 1): the
  `close` behind `NWellMergedNoSRAM` bevels every corner, so a corner-to-corner gap reads
  0.18 too long and a diamond tip loses more.
- KLayout: NW.b1 for 1.796, 1.697, 1.273, the tip and the strips.
- Verdict: all fire.  Expected 6 in `nw_b1_h1` and 2 in `nw_b1_h4`.

### 4. NW.b1 ignores PWell:block (false positive, debatable)

Manual, section 4.2: PWell = NOT (NWell OR PWell:block); NW.b1 is "Min. PWell width
between NWell regions".

Layout `NW.b1.h8`: two wells 1.00 apart with PWell:block over the whole gap, and two with a
0.50 block strip in the middle (0.25 of PWell on either side).

- gdscheck: NW.b1 for both pairs.
- KLayout: only the half-blocked pair (its NW.b1 is clipped to PWell), two markers on the
  two 0.25 strips.
- Verdict: with the gap fully blocked there is no PWell whose width NW.b1 could measure;
  the block-to-well space is PWB.c's.  Expected 1 (`nw_b1_h8`).  Debatable in that the
  punch-through the rule guards against does not care what the gap is called; if the
  PDK owner prefers the physical reading the case should be flipped, not the engine.

### 5. NW.d does not see an N+Activ that crosses the well edge (false negative)

Manual: "NW.d  Min. NWell space to external N+Activ ... 0.31"; note 1 allows Activ to
cross well boundaries only "in some ESD protection layouts".

Layout `NW.c.h3`: a P+Activ half in, half out of a well, and an N+Activ half in, half out.

- gdscheck: NW.c for the P+ crossing (enclosure 0.0000); nothing for the N+ crossing -
  `min_space` reports no distance for shapes that overlap.
- KLayout: NW.c and NW.f for the P+ crossing, NW.d and NW.e for the N+ crossing (its
  separation and enclosure checks run with `consider_overlaps_as_errors`).
- Verdict: the N+Activ outside the well is external N+Activ at distance zero: NW.d.  Not
  NW.e, since the tie is not "surrounded entirely by NWell".  Expected NW.c + NW.d
  (`nw_c_h3`, NW.e/NW.f ignored: whether the P+ crossing is also NW.f is a labelling
  question the manual's "inside PWell" leaves open).

### 6. NW.e drops a tie whose edge lies on the well edge (false negative)

Manual: "Min. NWell enclosure of NWell tie surrounded entirely by NWell in N+Activ 0.24".

Layout `NW.e.h8`: a 0.7 × 0.4 plain-Activ tie in a 1 × 1 well, its right edge on the
well's right edge, 0.3 margins elsewhere.

- gdscheck: clean (the `skip_clipped` parameter treats the coincident edge as a clipped
  tie).
- KLayout: NW.e (coincident edges are overlap errors for it).
- Verdict: the tie is entirely inside the well - nothing crosses out - and enclosed by
  0.00 on one side: NW.e.  Expected 1 (`nw_e_h8`).  Compare `NW.e.gds.gz`'s crossing tie,
  which is rightly skipped.

### 7. NW.e takes undoped Activ for a well tie (false positive)

Manual, section 4.2: N+Activ = Activ AND nSD with nSD = NOT (pSD OR nSD:block) OR
nSD:drawing; P+Activ = Activ AND pSD.

Layout `NW.e.h9`: Activ under nSD:block with neither nSD nor pSD drawn, 0.20 from the
well edge.

- gdscheck: NW.e ("NActivInNWell" is every non-pSD Activ in the well).
- KLayout: NW.c (it folds undoped Activ into PAct, the other extreme).
- Verdict: neither N+ nor P+, so neither rule; clean.  Expected none (`nw_e_h9`).  The
  matching NW.d case (Activ under nSD:block outside the well) is already right, see
  `NW.d.gds.gz`.

### 8. NW.f takes a P+Activ under PWell:block for a substrate tie (false positive)

Manual, section 4.2: substrate tie = (Activ AND pSD) inside PWell, PWell = NOT (NWell OR
PWell:block).

Layout `NW.f.h2`: among others, a P+Activ wholly under PWell:block 0.235 from a well, and
one with the block over its right half only.

- gdscheck: NW.f for both.
- KLayout: NW.f for the half-blocked one only.
- Verdict: the fully blocked P+Activ is in no PWell, so it is no substrate tie; the
  well-to-block space is PWB.c's.  Expected 4 × NW.f in `nw_f_h2`, gdscheck gives 5.

### 9. NW.f1 has no DigiBnd variant (false positive, and a rule the deck does not list)

Manual, section 8.1.1: inside DigiBnd "NW.f1  Min. NWell space to substrate tie in P+Activ
inside ThickGateOx inside DigiBnd  0.24".  `show-deck` lists NW.c1.dig, NW.d1.dig and
NW.e1.dig but no NW.f1.dig; NW.f1 runs at 0.62 everywhere.

Layout `NW.f.h7`: inside a DigiBnd, a TGO substrate tie 0.30 from a well and one 0.235.

- gdscheck: NW.f1 for both (0.30 and 0.235).
- KLayout: NW.f1.digibnd for the 0.235 one; 0.30 clean.
- Verdict: 0.30 is clean; 0.235 fires the digital variant.  Expected NW.f1.dig, following
  the deck's naming of the other three (`nw_f_h7`).

### 10. Enclosure is measured by projection only: chamfers and 45° walls near a corner (false negative, both tools)

Manual: NW.c/NW.c1/NW.e/NW.e1 give an enclosure value; nothing says the measurement is
restricted to parallel edges.

Layouts `NW.c.h1`, `NW.c1.h1`, `NW.e.h1`, `NW.e1.h1`: the NWell's corner chamfered along a
45° line that passes 0.269 (NW.c), 0.615 (NW.c1), 0.233 (NW.e), 0.615 (NW.e1) from the
Activ's corner while both axis-aligned margins are well over the value; controls with the
chamfer one grid step further out (0.311, 0.622, 0.240, 0.622); in `NW.c.h1` and `NW.e.h1`
also the same line as a full 45° wall.  Parallel 45° edges (an Activ chamfer facing a
NWell chamfer) sit beside them.

- gdscheck: the parallel-chamfer cases fire with the right distance (0.3041, 0.2333); the
  corner-to-chamfer and corner-to-wall cases are silent.
- KLayout: the same - its `ext_enclosed` forces the projection metric.
- Verdict: the P+Activ corner is 0.269 from the well boundary and the manual's 0.31 is a
  distance; I read these as violations.  IHP's own deck disagrees, so this is the least
  certain finding here; the cases (`nw_c_h1`, `nw_c1_h1`, `nw_e_h1`, `nw_e1_h1`) carry the
  euclidean answer.  The space rules (NW.b, NW.d, NW.f, NW.b1 on straight shapes) do
  measure corner to edge correctly in both tools.

### 11. NW.a reports chords across a 135° corner (false positive)

Manual: "NW.a  Min. NWell width  0.62".

Layout `NW.a.h2`: an octagon with 0.62 between opposite straight faces and 0.622 between
opposite diagonal faces (a legal shape), beside a 0.615 diamond, strip and octagon.

- gdscheck: NW.a "width 0.4754" eight times on the legal octagon, between the endpoints of
  adjacent edges that meet at 135° (e.g. (2.69, 8.87)-(2.87, 9.31), a chord across the
  corner, not a distance between facing edges); the same eight "0.4727" on the 0.615
  octagon on top of its four real 0.6152 markers.
- KLayout: nothing on the legal octagon; the narrow shapes only.
- Verdict: the legal octagon is clean.  Expected 10 (`nw_a_h2`: 4 + 2 + 4 real markers).
  The octagon's edges are short (0.18 and 0.26); the chord is under 0.62 only because of
  that, so a large 45°-cornered well does not show it, a small one does.

### 12. "Inside DigiBnd" for a device the DigiBnd edge cuts through (both tools relax; the manual does not)

Manual, section 8.1: "DigiBnd must enclose the complete layout of the digital
components"; the relaxed values apply to P+Activ/N+Activ "inside DigiBnd".

Layouts `NW.c1.h1`, `NW.d.h7`, `NW.e.h7`, `NW.f.h7`: a DigiBnd whose edge runs through
the Activ, device margins/gaps 0.45 (legal digital, illegal analog).  Beside it a DigiBnd
frame with the device in its hole, and (NW.d.h7) a DigiBnd over the Activ but not the
well.

- gdscheck: the cut-through device is digital (clean at 0.45) for c1/d1/e1; for f1 it
  fires the strict rule, but only because f1 has no digital split at all (finding 9).
  The device in the frame's hole is analog (NW.c1/NW.d1/NW.e1/NW.f1 fire at 0.45).  The
  Activ-only DigiBnd is digital (clean).
- KLayout: cut-through → digital for c1/d1/e1 (`not_outside`), but for f1 the driver clips
  the tie at the DigiBnd edge and fires NW.f1 on the part outside; frame's hole → digital
  (it explicitly fills DigiBnd holes); Activ-only → digital.
- Verdict: a device the DigiBnd edge cuts is not inside DigiBnd, so the 0.62 of section
  5.1 applies and 0.45 fires (in `nw_c1_h1`, `nw_d_h7`, `nw_e_h7`, `nw_f_h7`).  The frame's
  hole is not DigiBnd either: gdscheck is right and KLayout wrong.  A DigiBnd over the
  Activ alone satisfies the wording, clean.  The cut-through reading is strict; if the
  PDK owner prefers "touches DigiBnd" the four cases should drop one strict marker each.

## Notes that are not findings

- A. NW.b/NW.b1 labels for bare wells under 0.62 apart.  gdscheck: NW.b (net-blind) and
  no NW.b1 (the merged layer closes the gap, "regions separated by less than this value
  will be merged").  KLayout: NW.b1 (unconnected wells are different nets) and NW.b only
  on one net.  Both readings are coherent with the text; both flag every such gap.  The
  NW.b cases expect NW.b and ignore NW.b1; `NW.b.h9` (two wells 0.50 apart on different
  Metal1 nets, two on one net) expects NW.b twice.
- B. NW.b1 through the connectivity model is right in every variant drawn: ties on
  separate Metal1 fire, ties joined via Via1/Metal2 do not, P+Activ "taps" with Cont and
  strap do not connect the wells, N+Activ under a strap without Cont does not either, a U,
  a bridge and a tied island in a ring are one net (`NW.b1.h2`, `NW.b1.h3`).  KLayout agrees.
- C. Counts: `min_width` gives two markers per narrow bar (one per long edge) - the
  existing `NW.a` case already expects that; the hardening cases follow it.
- D. KLayout's hierarchical run reports an array cell's violation once (51 for 50
  instances); gdscheck reports 50 flat and 50 through the `GdsArrayRef`.

## Tested and found clean or correct (no need to redo)

- NW.a: 0.62 vs 0.615 in x and y; 300 µm bars across every tile line (1 violation); 0.622
  vs 0.615 diamond, 45° strip and octagon (apart from finding 11); chamfered box and L;
  unions of overlapping/abutting/gridded boxes at 0.62 and 0.615; a ring with one narrow
  side; islands in a ring; bars on/straddling x = 20/21/40/42 and an L cornered on 20;
  fifty flat and as GdsArrayRef; a 0.005 sliver; (1000, 1000); a comb; a U.
- NW.b: 0.615 vs 0.62; diagonal 0.43 (0.608) vs 0.44 (0.622) - corner to corner is
  measured; corner-on 0.615; diamond tip, 45° strips, chamfer-to-corner, tip-to-tip;
  unions; gaps on/straddling tile lines incl. a corner on the line; fifty flat/array;
  sliver, 300 µm bars, (1000, 1000); nets.
- NW.b1: 0.625/1.795/1.80; nets (note B); tile lines; fifty flat/array; 45° strips.
- NW.c: 0.305/0.31; parallel chamfers 0.304/0.311; unions of P+Activ and of NWell; plain
  Activ is not P+; pSD over half an Activ; a ring's hole; P+ crossing the well edge;
  tile lines incl. a 10 µm shape; fifty flat/array; TGO over half/abutting/0.005 sliver
  (NW.c vs NW.c1 split, both right); sliver, 300 µm, (1000, 1000).
- NW.c1: 0.615/0.62; NW.c1.dig 0.305/0.31; tile lines under one DigiBnd; frame hole
  (strict, right); fifty flat/array.
- NW.d: 0.305/0.31; diagonal 0.304/0.311; diamond tip; corner to 45° wall 0.304/0.311;
  parallel 45° edges; tie inside the well is not external; P+Activ is not N+; nSD under
  nSD:block is N+; ring hole 0.305/0.31; abutting boxes; two-box well; U-notch; tile lines;
  fifty flat/array; sliver, 300 µm, (1000, 1000); TGO half/abut/sliver; NW.d1.dig
  0.305/0.31; DigiBnd over the Activ only.
- NW.d1: 0.615/0.62; diagonal 0.615/0.622; diamond tip; fifty flat/array.
- NW.e: 0.235/0.24; parallel chamfers 0.233/0.240; P+ is NW.c's; drawn-nSD tie; ring body
  0.235/0.24; two-box tie; two-box well 0.235/0.24 (each box alone would clip the tie);
  tile lines; fifty flat/array; sliver, 300 µm, (1000, 1000); TGO half/abut/sliver;
  NW.e1.dig 0.235/0.24.
- NW.e1: 0.615/0.62; fifty flat/array.
- NW.f: 0.235/0.24; diagonal 0.233/0.240; diamond tip; corner to 45° wall 0.233/0.240;
  P+ inside the well is NW.c's, plain Activ is NW.d's; half under PWell:block; ring hole
  0.235/0.24; U-notch; union to union; tile lines; fifty flat/array; sliver, 300 µm,
  (1000, 1000); TGO half/abut/sliver.
- NW.f1: 0.615/0.62; diagonal 0.615/0.622; diamond tip; fifty flat/array.
- Not tested: the SRAM marker (the deck excludes it from NW.b1/NW.c/NW.d, the manual's
  SRAM section is "work in progress"); nBuLay-mediated well connections (nbulay deck).
