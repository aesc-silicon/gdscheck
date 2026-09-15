.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Virtual layers
==============

Derived layers, declared in ``pdk.yml`` under ``virtual_layers:`` and referenced by rules
like any drawn layer. Each is a name and the sentence that makes it:

.. code-block:: yaml

   virtual_layers:
     poly_otp:      poly2_drawn and otp_mk
     opl3a_poly:    (tgate or poly_field_otp_all) and otp_mk
     sab_otp_lv:    (sab and otp_mk) not_interacting v5_xtor
     nat4_gate:     poly_nat_lv grow 0.5 inside ngate
     pl7_nom:       (nom_gate edges inside_part comp) with_angle min 25 max 65
     mdp3c_edges:   mdp3c_ncomp edges centers 99%
     vdd_pins:      Metal1.pin with_text "VDD*" TEXT
     top_metal:     metal5


Reading a sentence
------------------

A sentence is read **from left to right**: an operand, then operations, each applied to
everything before it. Parentheses group; nothing else does, so there is no table of
precedences. ``a grow 0.5 inside b`` grows ``a`` and then keeps the grown regions inside
``b``; ``a and b or c`` is the union of ``c`` with the intersection of ``a`` and ``b``,
and ``a and (b or c)`` is the other thing.

An operation is a word, then its values, then — for the operations that take one — a
second operand:

* **Values** are a number, a percentage or a quoted string, as the operation wants:
  ``grow 0.5``, ``centers 99%``, ``with_text "VDD*"``.
* **Bounds** are ``min`` and ``max``, each followed by a number: ``with_area min
  0.1444``, ``with_angle min 25 max 65``, ``interacting min 2 b``. A bare number where a
  bound is expected means *exactly*: ``interacting 2 b`` is exactly two neighbours,
  ``with_length 0.5`` exactly half a micron.
* **The second operand** is a name or a parenthesised sentence: ``a inside b``,
  ``a and (b or c)``.

A name alone is an alias — ``top_metal: metal5`` — and reads like one.

Distances are in µm, areas in µm², angles in degrees. The words are the operation names
below, under their KLayout names, so a foundry deck's ``a.and(b).not(c)`` is written ``a
and b not c``. What a word means follows from what stands **left** of it: ``and`` on two
regions is an area intersection, on two edge layers the collinear overlap; ``edges``
turns a region into its boundary, and from there on the sentence is about edges. A word
applied to the wrong kind of layer, an unknown name, or a missing operand is refused when
the PDK loads, with the column it happened at.

Every operation in a sentence becomes one derived layer of the tiled merge cache, named
after its owner (``opl3a_poly#1``) where it has no name of its own; a chain of one word
(``a and b and c``) is the single n-ary operation it reads as. Naming an intermediate
result is therefore only a matter of reading: name it when it is a thing in its own right
or when a second sentence needs it.


Where a layer is built
----------------------

A virtual layer is built **per tile** in the merge cache, from its sources' tiles, on
first use. It costs nothing until a rule needs it, and its memory profile is the same
tile+halo bound as a drawn layer — which is what lets a chain over a dense, chip-wide
layer (``Activ``, the metals) stay bounded on a full chip.

The exceptions are decided by what reads the layer, not declared. A check that reads the
layout rather than the cache — ``no_ring``, ``ring_covers_boundary``, ``max_vertices``,
and ``forbidden`` with ``op: beyond`` — needs its layers as shapes in the layout, so a virtual layer such a rule names is materialised up front instead
(``pdk.rs`` → ``compute_virtual_layers``), as is one made by ``inside_ring``, which only
exists that way. Only ``union``, ``intersection``, ``difference``, ``close`` and
``inside_ring`` over **drawn** layers can be built like that; a rule that asks for
anything else is refused before the layout is read, naming the rule and the operation.


Booleans
--------

``or`` (``union``)
   The union of the operands. Duplicate polygons contributed by more than one source
   (e.g. two layers that happen to carry the same pad shape) are only counted once.

``and`` (``intersection``)
   The geometric AND — empty wherever any operand is empty. Used for device-recognition
   layers: ``CuPillarPad: Passiv.pillar and dfpad``.

``not`` (``difference``)
   The first operand minus every other one: ``ContNoSealring: Cont not EdgeSeal``.


Region selectors
----------------

These keep or drop *whole regions* of the left operand according to how they relate to
the right one. All resolve region membership by stitching each source's tile-local
pieces into whole connected regions first (see :doc:`architecture`), so a region is kept
or dropped as a unit and never split by a tile boundary. Each has a negated form that
keeps exactly the regions the positive form drops.

The names follow KLayout's, including its distinction between *overlapping* and
*interacting* — they differ only on zero-area contact, and choosing the wrong one is a
silent error, so both exist under the names a rule author reading a foundry deck expects:

.. list-table::
   :header-rows: 1
   :widths: 34 66

   * - Word
     - Keeps the regions of the left operand that…
   * - ``overlapping`` / ``not_overlapping``
     - share positive area with the right operand. Also spelled ``not_outside`` /
       ``outside``, KLayout's aliases for the same two relations.
   * - ``interacting`` / ``not_interacting``
     - share area **or** touch the right operand — a zero-area contact counts. The right
       operand may also be an edge layer: the regions a boundary segment touches.
   * - ``inside`` / ``not_inside``
     - lie entirely within the right operand. ``inside`` is the one selector that reduces
       with *and* across a region's pieces: every piece must be covered, where the others
       need only one piece to match.
   * - ``covering`` / ``not_covering``
     - contain a whole region of the right operand.

``overlapping``, ``interacting`` and ``covering`` take a **neighbour count**: ``metal1
interacting 2 pad`` keeps the metal touching exactly two pads, ``interacting min 2`` at
least two, ``interacting min 2 max 4`` the span. Without a count, at least one.

An empty right operand matches nothing, so the four positive words yield an empty layer
and the four negated ones pass the left operand through unchanged — ``inside`` included,
since nothing is contained in nothing.


Sizing
------

``grow r``
   Morphological dilation by ``r`` µm, outward only. Pair with ``interacting`` /
   ``not_interacting`` downstream when the grown reach is a proximity test.

``within r``
   ``grow r`` plus one grid step of reach (5 nm). For a *selection radius* —
   "everything within r of this" — where a shape at exactly ``r`` merely touches the
   grown region, and a whole-region test reads a zero-area touch as no overlap; the hair
   of extra reach turns that into a hairline overlap and the shape is selected. Wanted
   nowhere else: a band that decides which of two limits applies, or a region a rule
   must not reach into, is judged wrong by any slack at all.

``shrink r``
   Morphological erosion by ``r`` µm. A region narrower than ``2 × r`` disappears.

``close r``
   Closing (dilate then erode): regions whose gap is under ``2 × r`` merge into one.
   Used for same-net merging (NWell tie regions closer together than the rule distance).
   A closed pair of regions that stays separate keeps its *true* outer edges, so a
   downstream ``min_space`` still measures the real gap correctly.

``open r``
   Opening (erode then dilate): removes every part of a region narrower than ``2 × r``.
   A region that vanishes entirely had no spot wider than that anywhere — ``open``
   followed by ``not_interacting`` against the opened layer says "minimum width
   somewhere".

``grow_x r`` / ``grow_y r`` / ``shrink_x r`` / ``shrink_y r``
   Sizing along one axis, leaving the other exactly as drawn — KLayout's ``sized(r, 0)``
   and kin. Chaining all four is an opening with a box structuring element, the standard
   wide-metal idiom, keeping only regions at least ``2 × r`` across in *both* axes:

   .. code-block:: yaml

      WideM1: Metal1 shrink_x 5.0 shrink_y 5.0 grow_x 5.0 grow_y 5.0


Shape filters
-------------

Unary words that keep the regions of a certain shape:

``square`` / ``not_square``
   A single (approximately) square region.

``rectangle`` / ``not_rectangle``
   A filled axis-aligned rectangle; a region with a hole is not filled and so is not one.
   ``square`` is the stricter special case.

``not_circle`` / ``not_circle_or_octagon``
   Not an (approximately) regular circle, or neither a circle nor a regular octagon —
   for flagging disallowed bond-pad shapes.

``with_holes``
   Regions containing at least one hole — a ring-shaped NWell encircling an iso-PWell.

``holes``
   The hole area of each region, as filled polygons (KLayout's ``.holes``) — the
   interior of a substrate-tie ring. A hole only materialises in a tile whose bucket
   assembles the *whole* ring, so this serves device-scale rings, not a chip-perimeter
   ring whose "hole" is the entire die.

``extents``
   Each region's bounding box, filled.

``with_area min a max b``
   Regions whose area (µm²) is in the half-open range ``[min, max)``; either bound may be
   omitted, both may not.

``with_bbox_min min a max b`` / ``with_bbox_max min a max b``
   Filters on a region's bounding box: ``with_bbox_min`` tests the **shorter** side,
   ``with_bbox_max`` the **longer** one, both in µm and half-open — a side exactly on
   ``min`` is kept, one exactly on ``max`` is not. ``NarrowSlots: MetalSlot with_bbox_min
   max 2.0`` keeps the slots under 2 µm across.

``with_text "pattern" TEXT``
   Regions containing a text label on the ``TEXT`` layer matching the pattern — a
   trailing ``*`` makes it a case-insensitive prefix match, otherwise a case-insensitive
   exact match. Reads the layout's label records directly. KLayout's
   ``interacting_with_text``.

``enclosure_above d`` / ``enclosure_below d`` / ``separation_below d``
   The parts of the left operand whose enclosure by (or separation from) the right one is
   above or below ``d`` µm — a measurement kept as geometry rather than reported, so a
   rule can ask where it lies.

``inside_ring``
   The part of the left operand within the area *enclosed* by the right one, a ring: the
   ring's own holes are filled first (a seal frame becomes "frame + interior"), since
   there is usually no drawn layer for that interior. Built up front from drawn layers
   only, as the section above says. Not to be confused with the ``inside`` selector: this
   one clips and fills, that one keeps or drops whole regions.


Edge layers
-----------

A region's atom is a filled area: booleans combine areas, selectors keep or drop whole
regions, and the checks measure between facing walls. That leaves a class of rules
unsayable, because they are about *one piece of a boundary* rather than about a region. A
transistor's channel width is the length of the Activ boundary running under the gate; no
region has that length, and the source/drain region's own width is a different quantity.

``edges`` turns a region into its boundary segments, and every word after it in the
sentence is an edge operation. An edge layer takes a synthetic number from the same range
and is referenced by rules exactly like any other layer — a check that accepts one says
so, and rejects a region rather than silently reporting nothing.

.. code-block:: yaml

   channel_edges:   SourceDrain edges and (GatPoly edges)
   res_mk_off_hres: (res_mk edges not (hres_outline edges)) inside_part poly2_drawn
   mdn4a_narrow:    ldnmos_body width_below 4.0

The parentheses around ``GatPoly edges`` are the left-to-right rule at work: without
them, ``and GatPoly`` would try to intersect the edges with a region, and the loader
would say so.

.. list-table::
   :header-rows: 1
   :widths: 28 16 56

   * - Word
     - Right operand
     - Result
   * - ``edges``
     - —
     - every contour segment of the region, holes included.
   * - ``width_below d``
     - —
     - the facing walls of the region closer than ``d`` µm, both walls of each pair —
       a width measurement kept as geometry.
   * - ``and``
     - edges
     - the stretches the two share **collinearly**. Not an area intersection: two layers
       that overlap in area but share no wall have no common edge at all, and a partial
       overlap yields the fragment rather than either whole segment.
   * - ``not``
     - edges
     - the first with every shared stretch removed, so one segment can become two.
   * - ``or`` (``join``)
     - edges
     - every segment of both, kept as they are.
   * - ``inside_part`` / ``outside_part``
     - region
     - the parts lying inside (or outside) the region, **cut** at its boundary. Every
       region selector keeps or drops a region whole; these keep a piece of a segment.
   * - ``interacting`` / ``not_interacting``
     - region
     - the segments that touch (or do not touch) the region.
   * - ``interacting_edges`` / ``not_interacting_edges``
     - edges
     - the segments that touch (or do not touch) a segment of the other layer.
   * - ``with_length min a max b`` / ``without_length …``
     - —
     - segments whose length (µm) is in the half-open range ``[min, max)``, or the
       complement. A bare number is an exact length.
   * - ``with_angle min a max b`` / ``without_angle …``
     - —
     - segments whose orientation in degrees is in ``[min, max]``, normalised to
       ``[0, 180)`` so a wall reads the same whichever way its contour is walked.
   * - ``centers 99%`` / ``centers 0.5``
     - —
     - the middle part of each segment: a percentage of its length, an absolute length in
       µm, or both (the longer wins). Two abutting regions share a vertex, so an
       ``interacting`` between their edges answers yes for reasons that have nothing to
       do with the rule; shaving a percent off each end makes contact mean overlap.

Checks that take an edge layer: ``min_edge_length`` and ``max_edge_length``.

.. note::

   An edge *expression* upstream is not always an edge *problem*. ``poly.edges.and(
   gate.edges).width(v)`` is the gate-length shape — the poly's own width, measured only
   between the walls it shares with the gate outline — which ``min_gate_length`` answers
   on the region, choosing the walls, and no edge layer at all; ``walls: unshared`` picks
   the other kind, ``outside:`` cuts them to a region's exterior and ``angle: bent`` keeps
   the 45° runs, which between them say every channel-length rule in GF180. Reach for an
   edge layer when the rule measures a property of the boundary itself: a segment's
   length, or which edge of a shape is the line end.
