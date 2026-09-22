<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / cup: hardening report

Deck `cup` against section 9.3 of the GF180MCU design manual, "Circuit-Under-Pad (CUP)
Rules".  The section governs what may be drawn under a bond pad when active circuits are
allowed there; of its thirteen entries, only two carry a number that a layout can be
measured against on the metal layers - CUP.2, minimum width of the metal line used for
bond pads, 1 µm, and CUP.3, minimum space of the same metal (slots), 1 µm - and those are
the two the deck lists, once per metal level.  Variant D is a five-level stack, so
Metal1..Metal5; upstream's sixth level is MetalTop, which this variant does not register.

Neither rule is about the metal on its own.  Upstream reads

```ruby
metal_drawn.interacting(pad).not_interacting(guard_ring_mk).width(1.um).interacting(pad)
```

so the metal is picked out by a PAD marker landing on it and by no guard ring claiming it,
and then only the measurements that themselves reach the pad are kept.  All four halves of
that sentence are what the patterns below ask about.

17 layouts under `tests/data/gf180mcuD/generated/cup/CUP.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/cup.rs`, each with a `#[case]` in `hardening_cup`.  Every layout ran at
tiles 20, 7 and 100 and through the upstream runset.  **No count moved with the tile
size.**  `min_width` reports one marker per wall in gdscheck and one edge pair in KLayout,
so one narrow line is 2 against 1 throughout; `min_notch` is 1 against 1.

## Findings

### 1. A narrow line is dropped once it runs about 20 µm past the pad (false negative)

`CUP.2.h3` - one 0.995 µm wide Metal1 line from x = 10 to x = 45, with a 6 µm PAD marker
over x = 12..18 of it, covering the line's full width there.  The line is the metal line
used for the bond pad and it is under the 1 µm minimum.

| tool | markers |
| --- | --- |
| gdscheck @ 20 / 7 / 100 | 0 |
| KLayout runset | 1 |

Four further layouts fix what the answer turns on, and it is not the tile size (the zero
above is the same at 7, 20, 30, 40, 60, 100 and 1000 µm tiles) and not the pad's size:

| layout | line | pad | gdscheck | KLayout |
| --- | --- | --- | --- | --- |
| `CUP.2.h7` | x 10..18 | x 12..14 | 2 | 1 |
| `CUP.2.h11` | x 10..18 | x 12..18 | 2 | 1 |
| `CUP.2.h9` | x 10..30 | x 12..18 | 2 | 1 |
| `CUP.2.h3` | x 10..45 | x 12..18 | **0** | 1 |
| `CUP.2.h10` | x 10..45 | x 12..14 | **0** | 1 |
| `CUP.2.h6` | x 10..45 | x 9.5..45.5 | 2 | 1 |
| `CUP.2.h13` | x 10..45 | x 24.5..30.5 | 2 | 1 |

`CUP.2.h12` puts five lines of 16, 20, 24, 28 and 32 µm side by side under one pad
position (x = 12..18) and brackets the cut-off: the first four report two walls each and
the 32 µm line - whose far end is 24 µm past the pad - reports nothing.  With the pad
moved to the middle of the same 35 µm line (`h13`, every end within 14.5 µm of the pad) the
violation comes back.  So the marker is kept only while it stays within a fixed reach of
about 20 µm of the pad, measured along the marker, and dropped beyond it.

This is the shape a real bond pad has: the pad sits on a metal line that runs on into the
circuit.  A narrow line under the pad is silently accepted as soon as it is long enough,
and the same violation is reported or not depending on where along the line the pad sits -
a count that changes when the same geometry is drawn differently.

Verdict: KLayout is right; the manual asks about the metal line used for the bond pad, and
a marker that touches the pad is that line however far the line runs on.  `h3`, `h10` and
`h12` expect the manual's answer and fail on the current engine.

The control for the other direction is `CUP.2.h2`: an L whose 3 µm block is under the pad
and whose 0.995 µm arm runs 32 µm away from it, the narrow stretch never under the pad.
Both tools are silent, which is the right answer - a slot or a whisker out in the circuit
is the metal deck's business, not CUP's.

### 2. A measurement whose walls do not themselves meet the pad is dropped (false negative)

Two layouts, one class: gdscheck decides `interacting` on the wall marker, upstream on the
measurement between the two walls.

`CUP.2.h5` - a 0.995 x 8 µm upright Metal1 bar with the PAD marker abutting its left edge
(x 8..10 against the bar's 10..10.995), sharing an edge and covering none of the bar.

`CUP.2.h8` - the same bar with the PAD marker *inside* its width, x 10.2..10.8, reaching
neither wall.

| layout | gdscheck @ 20 / 7 / 100 | KLayout |
| --- | --- | --- |
| `CUP.2.h5` | 0 | 1 |
| `CUP.2.h8` | 0 | 1 |

In both the metal is selected - `interacting(pad)` counts a touch, as `CUP.2.h1` row 1
confirms from the other side (a GUARD_RING_MK merely abutting the bar exempts it) - so the
bar is in `metalN_pad_nogr` and the width is measured.  What is dropped is the marker, by
the rule's second `interacting(pad)`.

Verdict: upstream is right.  The rule keeps the measurements that are the pad's, and a
0.995 µm measurement lying under the pad is the pad's whether or not its two walls touch
the marker.  `h8` in particular is the pad marker drawn smaller than the line it sits on,
which is contrived; `h5` is not - a PAD marker abutting the metal it names is an ordinary
drawing mistake and one the rule should still catch.  Both cases expect two markers (one
per wall, this deck's granularity) and fail on the current engine.

### 3. Eight checkable rules of section 9.3 are in neither deck

`show-deck` lists CUP.2 and CUP.3.  The manual's table also gives, with numbers and
without the "rules not coded" star:

| rule | what it says | value |
| --- | --- | --- |
| CUP.5 | top vias directly underneath the pad opening | Not allowed |
| CUP.6 | min top via space to pad opening | 0.5 |
| CUP.7c | vias in clusters or arrays of at most 3x3; sea of via not allowed | - |
| CUP.7d | min via space in via arrays | 0.3 |
| CUP.7e | min space between via arrays | 0.5 |
| CUP.8 | Top_Via-1 directly underneath the Pad mask | Not allowed |
| CUP.8a | min Top_Via-1 space to pad opening | 0.5 |
| CUP.9 | 1LM, 2LM and 3LM process with CUP | Not allowed |

Starred and rightly absent are CUP.4 (circuits not allowed under a pad - "cannot be
checked by DRC deck" per the section's own note 1) and CUP.7b; CUP.1, CUP.1a, CUP.7a and
CUP.7f are guidelines.

Upstream's `cup.rb` implements none of the eight either, so this is not a gdscheck
regression against the reference - but they are geometric rules with values, on layers
this PDK has (Via4 as the top via, Via3 as Top_Via-1 on a five-level stack, and the PAD
marker), and CUP.5/CUP.8 in particular are the ones that keep a bond from cratering.
Recording them here so the gap is a decision rather than an oversight.

## Notes that are not findings

- The guard-ring exemption is per polygon and by contact: a GUARD_RING_MK abutting the
  bar takes the whole bar out (`CUP.2.h1` rows 1 and 4, on Metal1 and Metal5), the same
  marker 0.505 µm clear of it does not (row 2), and a marker touching the *pad* but not
  the metal does not (row 3).  Both tools agree on all four.
- The rules read the drawn layer only.  `CUP.2.h4` draws the 0.995 µm bar as Metal1 and
  Metal5 *dummy* fill under a pad; both tools are silent, which is right - fill is not a
  bond pad's metal line.
- CUP.3's pad reach behaves: `CUP.3.h1` draws a C with a 0.995 µm slot under the pad
  (fires), the same slot with the pad stopping 0.5 µm short of it (clean in both tools),
  and a 1.0 µm slot under the pad (clean).  The slot is short enough that finding 1 does
  not reach it.
- Variant D's level mapping is right: five levels, Metal1..Metal5, no MetalTop.  Upstream
  gates its sixth level on `metal_level_numerical >= 6`, which this variant is not.

## Tested and found clean or correct (no need to redo)

CUP.2 at 0.995/1.000 on all five levels; the guard-ring exemption by contact and its
misses, on Metal1 and Metal5; the pad touching the metal rather than covering it (the
*layer* selection, which is right - see finding 2 for the marker); drawn against dummy
metal on both ends of the stack; a narrow arm far from the pad (correctly silent); a
narrow line with the pad over the whole of it and with the pad on its middle, both across
the tile lines at 20 and 42; five line lengths under one pad position.  CUP.3 at
0.995/1.000 with the pad over the slot, with the pad short of it, and with a guard ring on
the polygon.  No count in the deck moved with the tile size.

## Resolution (2026-09-22)

- **1 (a narrow line dropped past the pad)**: fixed in the engine.  The
  `interacting` layer param kept a violation by sampling its marker at five points, so a
  six-micron pad on a thirty-five micron line fell between them; the marker's segment is
  tested against the layer exactly now, tile by tile over its own box.  `CUP.2.h3`,
  `h10` and `h12` report what the manual asks.
- **2 (a measurement whose walls do not meet the pad)**: kept and open.  The filter
  keeps a violation by its *marker*, and a width marker is the wall - upstream keeps it
  by the measurement between the two walls, which no marker carries.  `CUP.2.h5` and
  `h8` hold the engine's reading and name this note.
- **3 (eight rules of section 9.3 in neither deck)**: recorded for the gdscheck owner.
  They are geometric rules with values on layers this PDK has, and upstream codes none
  of them either, so it is a porting decision rather than a regression.
