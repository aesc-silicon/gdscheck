<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / pplus: hardening report

Deck `pplus` against the GF180MCU design manual, section 7.9 (PP.1-PP.12).  14 layouts,
`tests/data/gf180mcuD/generated/pplus/PP.*.h<n>.gds.gz`, drawn by `gen/gf180mcuD/pplus.rs`
(`hardening`), each with a `#[case]` in the `hardening_pplus` table of
`tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through the upstream KLayout runset (`hardening/oracle-gf180.sh`, `DECKS=nplus,pplus`).

`show-deck` lists every rule of the section: PP.1, PP.2, PP.3a, PP.3bi, PP.3bii, PP.3ci,
PP.3cii, PP.3d, PP.3e, PP.4a, PP.4b, PP.5a, PP.5b, PP.5ci, PP.5cii, PP.5di, PP.5dii,
PP.6, PP.7, PP.8a, PP.8b, PP.9, PP.10, PP.11, PP.12.  The values match the manual,
including the two pairs the manual states in the opposite order from 7.8 (PP.3b and
PP.3c put the looser 0.08 first, PP.5c and PP.5d put the tighter 0.02 first); the deck
has them the right way round.  Nothing is missing.

Section 7.9 is 7.8 with the wells swapped, and the deck reads it that way: where the N+
rules take the collar *outside* an N-well the P+ rules take the band *inside* it, and
where the N+ rules take the band inside a P-well the P+ rules take the collar outside
it.  Three of the four findings below are therefore the same defects the `nplus` report
records, confirmed on the mirror; findings 1 and 2 are the P+ deck's own.

Test status on the engine as of this report: 3 of the 14 cases fail, on findings 2, 3
and 4; the other 11 pass.

## Findings

### 1. PP.5d exempts every marker that touches GUARD_RING_MK (false negative)

Manual, PP.5di: "Extension beyond COMP: For Outside DNWELL (i) For Pplus to NWELL space
>= 0.43um for Pfield or LVPWELL tap - 0.02".  Section 7.9 says nothing about a guard
ring, and the N+ section has no equivalent exemption for NP.5.

Layout: `PP.5di.h1` - two identical P-taps in the field, COMP 1 x 1, the marker drawn
0.015 past the COMP's left wall.  The second (x 16..17) lies under a GUARD_RING_MK
rectangle (layer 167/5, x 15.5..17.5, y 9.5..11.5).

- gdscheck @20/7/100: `PP.5di` once, `(9.985,10.5)-(10,10.5)` - the bare tap.  Nothing
  for the marked one.
- KLayout: the same one marker (the guard-ring rules GR.1/GR.2/GR.6/GR.11 that gdscheck
  raises are other decks, outside `DECKS=nplus,pplus`).
- Verdict: both should report the marked tap too.  The deck's `pplus_no_guard_ring` is
  `pplus not_interacting guard_ring_mk`, mirroring upstream's
  `pplus.not_interacting(guard_ring_mk)` - the marker polygon drops out of PP.5di and
  PP.5dii entirely, for its whole outline, as soon as it touches the marker.  A guard
  ring's implant still has to reach past its COMP, and the exemption is not in the
  manual.  Whether section 12 (Scribe Line & Guard Ring) is meant to take over here is
  worth confirming; nothing in it replaces PP.5d.  Case `pp_5di_h1`.

### 2. A marker wall drawn on a well wall: gdscheck reads it out of the well, KLayout into it

Manual, PP.5b: "Extension beyond COMP for COMP (1) Inside NWELL (2) outside LVPWELL but
inside DNWELL - 0.16".  The condition is on the COMP.

Layout: `PP.5b.h1` - an N-well at x 10..15, y 10..15, and a P-tap just outside it: COMP
x 15.1..16.1, y 11..12, marker x 15.0..16.3, y 10.7..12.3.  The marker's left wall and
the well's right wall are the same line, x = 15, and the marker is 0.1 short of the COMP.

- gdscheck @20/7/100: `PP.5dii` only, `(15,11.5)-(15.1,11.5)` - the COMP is outside the
  deep well, the marker is 0 from the N-well, so the 0.16 half of PP.5d applies and 0.1
  fails it.
- KLayout: `PP.5dii` and also `PP.5b`.  `pp5b_pplus_slct.and(nwell)` keeps an edge that
  lies on the well's boundary, so upstream treats the marker's left wall as "inside
  NWELL" and measures the COMP's wall against it a second time.
- Verdict: gdscheck is right.  The COMP is outside the N-well, so PP.5b's condition does
  not hold; the violation is real but it is PP.5dii's, once.  Case `pp_5b_h1` passes.
  (The same divergence shows up incidentally in the `nplus` fixture `NP.3ci.h2`, where
  the P+ marker's wall is drawn on the N-well's wall.)

### 3. The butted-pair exemption is whole-marker, so an unrelated N+ active goes unmeasured (false negative)

Manual, PP.3a: "Space to NCOMP for NCOMP (1) inside LVPWELL (2) outside NWELL and
DNWELL - 0.16".  The zero-space exemptions in the manual are PP.3d and PP.3e, both about
the butting pair itself.

Layout: `PP.3a.h1` - one L-shaped marker (foot x 10..12, y 10..11; arm x 10..20,
y 11..12).  In the foot, a COMP at x 10.3..13.5 is P+ to x = 12 and N+ beyond it: a legal
butted pair.  At the far end of the arm, an unrelated N+ active sits at x 20.155..21.155,
0.155 from the marker's wall.

- gdscheck @20/7/100: clean.
- KLayout: clean (only `NP.5b`, on the far active's own implant).
- Control `PP.3a.h2`, the identical scene without the butted pair: both report `PP.3a`
  at `(20,11)-(20.155,11)`.
- Verdict: PP.3a should fire in `h1`.  `pp3_pplus` is `pplus not_interacting
  pcomp_butted`, matching upstream; a single butted substrate tie switches the spacing
  rule off for the whole implant island it touches.  Case `pp_3a_h1`.  This is finding 3
  of the `nplus` report on the other implant.

### 4. PP.7's space of nothing, and PP.10's missing "crossing" half (false negatives)

Both are the `nplus` report's findings 1 and 2, drawn on the P+ side in `PP.10.h1`:

- (c) a poly bar at x 20..21 under a salicide block with the marker at x 18.6..20.0 -
  the two walls are the same line, x = 20.  gdscheck is silent; KLayout reports `PP.7`.
  The manual's PP.7 is a space of 0.18, and a shared edge is a space of nothing.
- (b) a COMP at x 15..16 under a block with the marker at x 14.6..15.5, cutting the
  unsalicided COMP in half.  Both tools report only `PP.5di` (the P+ active's own wall,
  extension zero) and no `PP.10`, although the overlap on the right is nothing.  PP.9 has
  a `forbidden pp_9_crossing` layer for exactly this case on unsalicided poly; PP.10 has
  none, and neither does upstream.

Case `pp_10_h1`, expected `PP.10` twice plus `PP.5di` and `PP.7`.

## Settled readings, recorded so the next round does not redo them

- **PP.11 and NP.11 are complementary halves of one sentence.**  `PP.11.h1` draws butting
  edges 0.2 inside an N-well wall, 0.2 outside it (at the tile line x = 20) and 0.5
  inside it; both tools report `PP.11` for the first, `NP.11` for the second and nothing
  for the third.  PP.11 takes the band inside the N-well and the collar outside the
  P-well, NP.11 the other two; a butting edge is the same line on both implants, so
  between them the manual's "within 0.43um" of either wall is covered on both sides.
- **PP.4a reads projection.**  `PP.4a.h1` puts a butting edge round the corner from an
  N-channel gate - horizontal at y = 12.7, x 11.2..12.28, the gate's left wall at
  x = 12.5, y 11.5..12.5, 0.297 corner to corner and facing nothing.  Both tools are
  silent, which is what "at a butting edge parallel to gate" asks.  `PP.4a.h2` faces the
  edge at the gate's wall: 0.315 fires, 0.32 is clean.
- **PP.12's reach follows the poly.**  `PP.12.h1` (a): a U whose right leg is 0.22 of air
  from the gate but ~5 um along the conductor, covered by the marker and inside the plain
  0.32 disc.  Both silent.  (b): the marker 0.315 up a straight bar, one step inside the
  3 x 0.10633 = 0.319 reach.  Both report PP.12.
- **PP.5a's `forbidden pp_5a_crossing` layer cannot fire.**  `pgate` is `pactive and
  tgate`, `pactive` is `pcomp and all_nwell` and `pcomp` is `comp and pplus`, so a gate
  lies inside the marker by construction.  `PP.5a.h1` (b), a gate whose poly bar the
  marker's wall cuts, is caught by the enclosure at zero instead (three PP.5a in both
  tools).  Same as NP.5a; the rule could be dropped.
- **An N-well tap owes its extension to PP.5b and PP.5dii at once.**  In `PP.5a.h1` (b)
  the cut leaves the P+ active's right wall with no extension, and both tools report
  `PP.5b` and `PP.5dii` there, each at 0.16.  The manual's own conditions overlap - a
  COMP inside an N-well is also a COMP outside the deep well within 0.43 of an N-well -
  so the duplication is the manual's, not the deck's.

## Tested and clean

- PP.3ci/cii at the N-well band: `PP.3ci.h1`, four scenes - an N+ active in the 0.429
  rim with the marker 0.155 away (PP.3cii, 0.16) and 0.16 away, one in the core with the
  marker 0.075 away (PP.3ci, 0.08) and 0.08 away.
- PP.5ci/cii at the P-well band inside a deep well: `PP.5ci.h1`, four taps - 0.015 past
  the COMP in the core (PP.5ci), 0.02 there, 0.155 past a COMP in the band (PP.5cii),
  0.16 there.
- PP.6 at the bound: `PP.6.h1`, a P+ half 0.215 long (fires) and one 0.22 long; the three
  walls the N+ active shares with the COMP are an overlap of nothing and neither tool
  counts them.
- PP.9's resistor exemption: `PP.9.h1`, two unsalicided poly bars with the marker 0.175
  past each; the one under RES_MK is exempt, the bare one fires.  The N+ twin has no such
  exemption - see finding 4 of the `nplus` report.
- PP.10 at the bound: `PP.10.h1` (a), 0.175 past an unsalicided COMP.
- Tile invariance: no count moved between tiles 20, 7 and 100 on any of the 14 layouts,
  including the butting edge at x = 20.2 (`PP.11.h1`) and the well and marker walls that
  land on x = 20 in the same layout.

## Resolution (2026-09-22)

- **1 (the guard-ring exemption)**: kept and open.  It is upstream's reading, the
  manual has no such exemption, and section 12's own rules cover the ring; whether
  PP.5d should reach into a guard ring is the gdscheck owner's call.
- **2**: gdscheck was right; nothing to do.
- **3 (the butted-pair exemption)** and **4 (PP.7, PP.10)**: see the nplus report's
  resolution - PP.7 reports a touch now, PP.10 has its crossing half, and the butted
  exemption is kept and open.
