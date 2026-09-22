<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / lvs_bjt: hardening report

Deck `lvs_bjt` against the GF180MCU design manual, section 10.9 (`gf180mcu_drm/drm_10_09.txt`,
"LVS_BJT Mark Layer": LVS_BJT.1), with the derived layers of `pdks/gf180mcuD/pdk.yml`.  Three
layouts, `tests/data/gf180mcuD/generated/lvs_bjt/LVS_BJT.1.h<n>.gds.gz`, drawn by the
`hardening` half of `gen/gf180mcuD/bjt.rs`, each with a `#[case]` in the `hardening_lvs_bjt`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100 and
through the upstream KLayout runset
(`DECKS=lvs_bjt hardening/oracle-gf180.sh <layout> TOP 20 7 100`).

## Coverage

`show-deck` lists LVS_BJT.1 - the section's only rule.  Nothing is missing.  "Minimum LVS_BJT
enclosure of NPN or PNP Emitter COMP layers is 0" is a `forbidden` on `lvsbjt1_viol`, the part
of an emitter that the marker does not cover: an enclosure of 0 is containment with no margin,
so any part of the emitter outside the marker is the violation.

## How the section reads

The layer "is used in LVS tool to identify Emitter, Base, and Collector of NPN and PNP
transistor", and the only rule asks that the marker enclose the emitters.  All of the work is
deciding which COMP is an emitter.  The deck reads it as upstream does: an NPN emitter is an
N+ active that the LVS marker interacts *and* that lies inside a DNWELL; a PNP emitter is a P+
active the marker interacts inside an NWELL.  So the fixtures are one of each, plus every way
of not being one - the wrong implant, the wrong well, no well, and an active the well's edge
cuts.

**No count moved with the tile size**: three layouts x three tile sizes, every rule identical.
`LVS_BJT.1.h3` puts the same half-covered emitter on x = 20, x = 42 and y = 20 on purpose and
all three survive every tile.

Test status on the engine as of this report: all three cases pass, as do the two older
`good`/`bad` cases.  **No findings.**

## Findings

None.  gdscheck and the upstream runset agree on every structure drawn, and the manual agrees
with both.

## Read and left alone

- **A marker that only abuts an active still names it an emitter.**  `LVS_BJT.1.h1` (c) puts
  an N+ active (19, 11)-(20, 12) inside a deep well with the LVS_BJT marker at
  (18, 11)-(19, 12) - sharing the wall at x = 19, overlapping by nothing.  The derivation is
  `ncomp interacting lvs_bjt`, and `interacting` includes a touch, so the active is an emitter
  and the whole of it lies outside the marker: both tools fire (gdscheck `lvsbjt1_viol present
  at (19.0000, 11.5000)`, KLayout the polygon `(19,11;19,12;20,12;20,11)`).  It is the right
  advice - a marker drawn beside an emitter instead of over it is a mistake, and the rule is
  what tells you - so the case keeps it, deliberately.
- **An active the well's edge cuts is not an emitter.**  `LVS_BJT.1.h1` (d) draws an N+ active
  (24, 11)-(26, 12) running out of the deep well at x = 25 with the marker over its left half;
  `LVS_BJT.1.h2` (e) does the same with a P+ active and an N-well.  Both are clean in both
  tools: `inside dnwell` / `inside nwell` is a whole-region test, and an active that straddles
  the well boundary is no vertical transistor's emitter.  A designer who meant it to be one
  has a well problem, not an LVS_BJT problem.
- **Marker granularity**: gdscheck reports one `forbidden` marker per uncovered region, and
  KLayout the same polygon; the counts match exactly on all three layouts (3 / 3, 1 / 1,
  3 / 3).

## Tested and clean

- **The NPN emitter** (`LVS_BJT.1.h1`, 3 / 3): an N+ active in a deep well with the marker over
  all of it is clean; over half of it fires; only abutting it fires (see above); an active the
  deep well's edge cuts is no emitter and is clean; an active outside every well with the
  marker over half of it is no emitter and is clean; the marker leaving a 0.005 µm sliver of
  the active's top edge uncovered fires - gdscheck at (31.5, 11.9975), KLayout
  `(31,11.995;31,12;32,12;32,11.995)`.
- **The PNP emitter** (`LVS_BJT.1.h2`, 1 / 1): a P+ active in an N-well with the marker over
  all of it is clean and over half of it fires; the same P+ active in a *deep* N-well instead
  is no PNP emitter and is clean; an N+ active in the N-well with the marker over half of it
  is neither an NPN emitter (no DNWELL) nor a PNP one (wrong implant) and is clean; a P+
  active the N-well's edge cuts is clean.
- **Tile invariance**: three layouts x tiles 20, 7 and 100, no count moved; the three
  half-covered emitters of `LVS_BJT.1.h3` straddle x = 20, x = 42 and y = 20 and give 3 at
  every tile in both tools.
