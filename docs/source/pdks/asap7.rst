.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

ASAP7
=====


Overview
--------

``asap7`` is the 7 nm predictive FinFET PDK from Arizona State University,
release 1p7. The Calibre deck is not public, so the rules follow the design rule
manual (``asap7_drm_201207a.pdf``), with the `KLayout port
<https://github.com/laurentc2/ASAP7_for_KLayout>`_ read alongside it. Layouts are read
at the real 7 nm scale on a 0.25 nm grid, as the standard-cell libraries ship; a
layout drawn 4× up for Calibre must be scaled down first.

The decks are per layer (``well``, ``fin``, ``gate`` … ``m9``, ``v9``), plus
``geometry`` for non-orthogonal shapes. The suites are ``main`` (every deck), ``feol``
(through V0, what a standard cell holds) and ``beol`` (M1 to the pad).


One-way layers
--------------

Most layers route one way, so the manual gives them one width and space across the
track and another along it. These rules read ``facing: x`` (vertical walls, a
horizontal span) or ``facing: y``; a corner-to-corner space (``M1.S.6``,
``LIG.LISD.S.7``) reads ``facing: none``. A via "enclosed on at least two opposite
sides" is ``sides: opposite``, and "the same width as M2" is a ``max_enclosure`` of
nothing on ``sides: opposite``.

The M1–M3, LISD, LIG and M8/M9 spacings depend on the lengths of the two facing edges
(side to side, tip to side, tip to tip), so they read edge layers sorted by length
(``virtual_layers`` in ``pdk.yml``).


What is not checked
-------------------

Each deck's header lists the rules of its section it does not check, and why: mainly
the routing grids and tracks. ``ACTIVE.W.2`` and
``SDT.W.3`` read every vertical ACTIVE and SDT edge against the whole multiples of
27 nm; ``ACTIVE.LUP.1`` reads the
reach from a gated ACTIVE to a tap under the well's own implant, 30 µm round and, in a
well, confined to that well, the way the IHP latch-up rules are read.
``V0.LIG.AUX.2`` is read from figure 3.11.2(c) as "a V0 crosses its LIG": the part
of the V0 off the LIG must be two pieces, one either side, so the rule needs no
LIG direction.
M4–M7 even width multiples are checked through each layer's maximum width;
widths above that maximum are rejected independently. Their even-track-count
restrictions and some conditional tip-spacing rules remain unimplemented.
V0–V3 ``S.1`` reads vias that face each other, on one track or on
neighbouring tracks overlapping in projection, at 18 nm. Vias on neighbouring
tracks that do not overlap meet corner to corner, which ``S.2``–``S.4`` read by
end-cap at 23, 30 or 27 nm; the 27 nm in the ``S.1`` row is ``S.4``'s value.
``FIN.S.1``, an exact 27 nm pitch, is checked only as a space of at least 20 nm.

Via end-caps and local-interconnect nets
----------------------------------------

V0–V3 corner spacing distinguishes two, one and zero 5 nm upper-metal end-caps
at the corners facing the gap: respectively 23, 27 and 30 nm. A cap at the far
end of a via does not relax the gap. The landing metal remains whole even when
it intersects SRAMDRC elsewhere. Exactly 5 nm qualifies. ``V0.LISD.EN.3`` reads
the portion of a non-SRAM via overlapping LISD that interacts with LIG, requiring
3 nm on an opposite pair while allowing the rest of the via to protrude.
As with ``V0.LISD.EN.2``, this reads the manual's "minimum enclosure" as a lower
bound, despite the table's ``==`` symbol: the standard-cell libraries put V0s on
LISD bars with tens of nanometres along the bar and as little as 2 nm across it, so
no pair of sides is exactly 3 nm.

V4–V6 ``W.1`` is exact along the upper metal's length, as the manual words it: the
via's walls running with M5 and M7 (vertical) or M6 (horizontal) must be exactly 24
or 32 nm, and across the metal ``AUX.2`` holds the via to the metal's width. The
manual never says which way M8 runs, so ``V7.W.1`` reads the via's short side.
V8/V9 corner spacing shares ``S.1``'s 57 nm, which reads every direction.

``LIG.LISD.S.6``, ``LIG.LISD.S.7`` and ``LIG.SDT.S.8`` use electrical connectivity,
including remote routing through M1–M9 and the pad. Contact intersections keep
net-extraction anchors on actual overlapping material, including partial V0
landings. SRAM conductors participate in connectivity; only the measured subjects
are filtered. ``ACTIVE.S.2A`` reads the nets too, through the source/drain regions - the ACTIVE a
gate crosses, less the gate - which reach the graph through their SDT contacts. It
reads the 1 nm lips of the regions flanking a break under 92 nm, the lips of one
ACTIVE polygon being related and no pair; a lip faces across its own break alone, so
the region behind an exempt same-net pair, which an unshielded space check would
otherwise reach, is never read. These four rules are skipped
with ``--no-connectivity``.

SRAM applicability
------------------

DRM section 1.2.2, convention 7 makes non-SRAM geometry the default scope of a
rule unless its wording says otherwise. This applies throughout the decks,
including ordinary LISD/LIG, via and M4–M9 checks, not only M1–M3.

SRAM membership requires **positive-area overlap** with ``SRAMDRC``. Both ordinary
and explicitly SRAM-scoped checks use this partition. Edge-only and point-only
contact remain non-SRAM. This is a deliberate conservative deviation from the
manual's convention 4: the manual counts shared-edge contact as interaction, but
excludes isolated vertex contact. The general engine ``interacting`` operator also
counts vertex contact, so it is not the predicate used for SRAM membership here.

Classification keeps or drops a whole merged polygon, including its geometry
outside the marker and across tile boundaries. Edge layers are extracted after
selection, preserving original edge lengths. A marker on the remote end of a wire
can therefore exempt an ordinary width or spacing violation outside the array.
This does not exempt other layers connected to that wire through vias, and marker
locations alone cannot determine whether a violation should be exempt.

The rules use the selected subjects as follows:

* Ordinary single-layer checks read non-SRAM polygons. Ordinary spacing between
  two subjects reads a pair only when both are non-SRAM. No extra SRAM/non-SRAM
  boundary-spacing requirement is inferred. This is the literal convention-7
  interpretation; mixed-pair behavior has not been verified against Calibre.
* Via checks select the via instance, retaining full landing metal and LIG/LISD
  references. A non-SRAM via still sees metal that overlaps SRAMDRC somewhere
  else. This includes ``V0.LIG.EN.4``, whose existing implementation measures LIG
  walls inside the selected V0. Other enclosure/extension checks select the
  enclosed subject and keep the enclosing reference layer whole. Extension checks
  select FIN, ACTIVE or cut-GATE subjects; where the engine measures a channel
  fragment, selection precedes that intersection. Coverage predicates likewise
  retain their full reference geometry.
* Explicit qualifications take precedence. ACTIVE/WELL and implant enclosures
  select ACTIVE as their subject; SRAM SDT overlap and LIG/GATE overlap select SDT
  or LIG, respectively, retaining their full reference layers. The existing
  SRAM-specific limits and ACTIVE/LISD/LIG boundary-contact prohibitions remain
  active. ``SRAM.SDT.ACTIVE.OV.3`` and ``SRAM.SDT.LISD.OV.4`` both require 17 nm
  vertical overlap, with separate checks for missing or touching-only references;
  a width check alone cannot reject an empty intersection. Boundary prohibitions
  test contact independently of the membership filter.
* The SRAMDRC marker's own non-orthogonal-geometry check remains active everywhere
  as a supplemental marker-integrity check. A marker is not exempted against
  itself. ``SRAMVT`` is a threshold-adjust layer, not the SRAMDRC marker; its name
  alone does not give it SRAM scope.

``ACTIVE.S.2A`` is read in SRAM as well as outside it, as its own note requires.


Readings
--------

A few rules are read differently from the manual's literal wording, because the
shipped standard-cell libraries (verified with the Calibre deck) contradict that
wording:

* ``V0.LIG.EN.4`` is worded as the LIG's enclosure by the V0. The libraries draw both
  that case (a V0 over a 16 nm LIG rail, 1 nm past it on each side) and a V0 flush with
  the end of a wider LIG. The rule reads the LIG walls inside the via and skips the
  flush one. The manual asks for two opposite sides; with ``V0.LIG.AUX.2`` holding a
  V0 across its LIG, the only LIG walls inside the via are that pair.
* ``SDT.ACTIVE.AUX.2``, "SDT horizontal edges must coincide with ACTIVE horizontal
  edges", is read as "an SDT does not end inside its ACTIVE". Where an ACTIVE steps,
  the libraries run the SDT on past it.
* The select, VT and well minimum widths fire on the 54 nm half-width filler and tap
  cells when these are checked on their own. Placed between other cells, they are part
  of a wider shape.

Tests
-----

``tests/asap7.rs`` uses synthetic cases to check both SRAM SDT overlaps at 17 nm,
empty references, local via end-cap qualification, partial V0 landing, remote net
connections, even metal widths, and V1's asymmetric 5/2 nm enclosure. Cases include
rotation, negative coordinates, SRAM selection, and geometry on or crossing tile
boundaries.

The generated fixtures in ``tests/data/asap7/generated/`` also run in ordinary
Rust CI. They cover the WELL rows, FIN, GATE and GCUT width, spacing, area,
extension and shape rules, each gate with a partner one pitch along; every ACTIVE
rule, the well thresholds inside, outside and across the SRAM marker among them;
every SDT rule, the SRAM overlaps with absent and touching references among them;
every LISD and LIG rule, the net-gated LIG spacings with a same-net pair joined
through V0 and M1 among them;
every select and VT rule on all five layers; the geometry rule, a 45° chamfer on
every layer it reads, and the same chamfers exempt under the SRAM marker; width, spacing, area and edge rules on every metal, including
the length-class, corner, notch, even-width, wide-line and bent-wire cases; and every via level
in a whole stack: width, spacing, corner spacing by end-cap, both enclosures, the
flush upper metal, coverage, V0 on LIG and partly on LISD, and each spacing and
enclosure defect under the SRAM marker. Three routed upper-metal nets with a
deliberately defective companion check the full main suite.

``gen/asap7/`` separates ACTIVE, SDT, via, metal and routing patterns into modules,
drawn with the shared helpers and the ASAP7 builders in ``gen/asap7/patterns.rs``.
Each fixture, ``<deck>/<rule>.<case>.bad.gds.gz``, holds the shape exactly at the
limit and the ones one DBU past it. The limits come from the manual, and generation
verifies that the deck values agree with them. Regenerate with
``just gen-testdata-for asap7``.

Expected violation counts live in ``tests/asap7.rs``. A fixture must report only
its target rule, with the expected count; where two rules overlap by their nature -
a via missing a layer also fails its enclosure, say - the test names the other rule
and ignores it. A test there also checks that every rule in every deck has a fixture,
and that every fixture names a rule its deck still has.
