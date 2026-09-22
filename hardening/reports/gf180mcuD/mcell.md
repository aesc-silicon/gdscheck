<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / mcell: hardening report

Deck `mcell` against section 7.17 of the GF180MCU design manual, "Mcell".  Four rules
over one marker layer, MC = MTP cell implant, drawn on 11/17:

| Rule | Description | Value |
| --- | --- | --- |
| MC.1 | Width | 0.4 |
| MC.2 | Space | 0.4 |
| MC.3 | Minimum Mcell area | 0.35 µm² |
| MC.4 | Minimum area enclosed by Mcell | 0.35 µm² |

`show-deck` lists all four (`.2` as a `min_space` and a `min_notch` under one id, which is
this engine's split of KLayout's `space`), with the manual's values.  Nothing in the
section is missing and nothing is invented.

4 layouts under `tests/data/gf180mcuD/generated/mcell/MC.*.h1.gds.gz`, drawn by
`gen/gf180mcuD/mcell.rs`, each with a `#[case]` in `hardening_mcell`.  Every layout ran
through gdscheck at tiles 20, 7 and 100 and through the upstream runset.  No count moved
with the tile size.

## Findings

None.  Every bound, both area readings and the layer itself came out as the manual asks,
and the only difference from the runset is the settled marker cut.

## Notes that are not findings

- `min_width` reports one marker per narrow wall where KLayout reports one per edge pair:
  a 0.395 µm bar is 2 markers here and 1 there (`MC.width.h1`, `MC.layer.h1`).  That is
  the round's settled cut, not a disagreement about the layout.
- MC.3 reads the merged region, not the drawn boxes: two abutting 0.4 x 0.4 boxes are one
  0.32 µm² region and one violation, not two, and two abutting 0.4 x 0.45 boxes are
  0.36 µm² and clean (`MC.area.h1`).  Both tools agree.
- MC.3's bound is exclusive on both readings of the width: 0.5 x 0.7 and 0.4 x 0.875 are
  exactly 0.35 µm² and clean; 0.5 x 0.695 and 0.4 x 0.87 fire.
- MC.4 is the hole and only a real hole.  A 0.5 x 0.7 hole is exactly 0.35 µm² and clean,
  0.5 x 0.695 fires, and a C whose inside reaches the outside through a channel is not a
  hole however small its inside is - through a 0.4 µm channel it is clean, through a
  0.395 one it is MC.2's space between the two arm ends and still not MC.4
  (`MC.hole.h1`).  Both tools agree on all four.
- The marker is 11/17 alone: the same 0.395 µm bar on 11/39 (MVSD), 11/16 and 11/18 is
  read by nothing (`MC.layer.h1`).
- A ring's own area cannot be pushed under 0.35 µm² while its arms stay at MC.1's 0.4 -
  with 0.4 arms the ring is at least 0.64 µm² whatever the hole - so whether MC.3 reads
  the net area or the outline cannot be asked with a legal shape.  Not tested.

## Tested and found clean or correct (no need to redo)

MC.1 at 0.4/0.395; MC.2's space and its notch at 0.4/0.395, and the C's 0.4/0.395
channel; MC.3 at 0.35 µm² from two directions and on a merged pair of boxes; MC.4 at
0.35 µm² and against an opening that is not a hole; the layer's datatype against its
three neighbours on layer 11.
