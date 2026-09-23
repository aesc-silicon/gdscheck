<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ihp-sg13g2 / offgrid, forbidden: hardening report

Decks `offgrid` (every drawn layer on the 0.005 µm grid) and `forbidden` (the layers
the process does not offer), drawn by `gen/ihp_sg13g2/{offgrid,forbidden}.rs` in the
main session (2026-09-21) - nine layouts, each with a `#[case]` in `tests/ihp-sg13g2.rs`,
every one run at tiles 20, 7 and 100.  No KLayout column: IHP's deck has no grid check
in the driver and the forbidden layers are its own list.

## Findings

None.  Every off-grid vertex is reported once whatever the tile size: edges off by
0.001, 0.003, 0.004 and 0.006 (two vertices each), a box with all four vertices off,
negative coordinates (-0.003 off, -0.005 on), a diamond's vertex 0.002 off, a 45°
strip's far end 0.003 off (its two vertices; the on-grid diamond, strip and chamfer
are clean), vertices 0.002 past x = 20 and 0.002 short of 40, a box straddling x = 100
with its far edge off, boxes at (1000.003, 1000) and (-1000, -1000.002), fifty boxes
with an off-grid edge flat and as a `GdsArrayRef`, and fifty on-grid boxes placed by an
array of pitch 2.003 - every placed copy off but the origin and the fifth column's
first row (5 × 2.003 = 10.015 is on the grid): 192.  A forbidden shape is reported
once whether it crosses x = 20 and 40, lies at (1000, 1000), overlaps another
forbidden layer, is a 0.005 µm square, or comes fifty at once flat or as an array.
