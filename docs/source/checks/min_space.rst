.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_space
=========

Minimum spacing between merged regions on one layer, or between two layers. Params
narrow the pairs the rule is about: those with a 45° wall at the gap, those on one net or
on two, those facing each other along a long run with a wide line on one side.


Semantics
---------

``layers`` takes one entry (same-layer spacing) or two (inter-layer spacing). Both
layers are merged first (so overlapping/nested shapes don't create spurious internal
gaps), then every pair of regions is checked: for same-layer spacing, all distinct
region pairs; for two layers, every region of ``layers[0]`` against every region of
``layers[1]``.

For each candidate pair, a bounding-box pre-filter skips pairs that can't be within
``value`` before doing real geometry work. Surviving pairs are tested for overlap first
— an overlap (or one region sitting inside another's hole) is not a spacing violation
and is skipped. Otherwise the true closest edge-to-edge distance between the two
regions is computed segment to segment, exactly: the coordinates are integers on the
grid, so an axis-aligned gap is a difference of two of them and a diagonal one is
compared squared as a ratio of two integers, against ``value`` rounded up to whole DBU
once. Whether a pair overlaps, touches or abuts is likewise a matter of signs, never of a
tolerance. Two shapes meeting at an isolated point are a gap of zero and a violation; two
shapes drawn edge to edge along a run abut and are not. Only the ``square`` metric still
measures in floating point, with half a DBU of slack.

Each violation is owned by the tile whose core contains the gap's midpoint, so a pair
visible from several overlapping halo tiles is reported exactly once.

Every gate asks about the violating gap itself, not about the shapes at large. A long
net that runs at 45° in one place and Manhattan elsewhere only gets the bent spacing
where it is actually angled; a wide power rail running alongside a wire at the clean
gap does not lend its width to a narrow tooth that dips below it elsewhere on the same
polygon. The gates combine: a rule may ask for all of them at once, and a pair must
satisfy every one it asks for.

Declared on two *edge* layers (see ``edge_layers`` in :doc:`../virtual-ops`), the plain
rule measures between segments instead: two edges pair when they are parallel, project
onto each other and face across empty ground. The gates read regions and have no meaning
there.


Layers
------

- Same-layer form: one layer — checks every region against every other region of it.
- Two-layer form: ``layers[0]``, ``layers[1]`` — checks every region of the first
  against every region of the second.


Parameters
----------

``angle``
   Optional. ``bent`` restricts the rule to pairs where one of the two regions has a
   diagonal (45°) edge within the gap distance of the other (IHP ``M1.i``: a wider space
   next to a bent line).

``net``
   Optional. ``same`` restricts the rule to pairs whose two regions resolve to one net;
   ``different`` to pairs that do not. This models "different potential" spacing, where
   conductors at one potential may sit closer than conductors at different potentials:
   the deck states a small ``net: same`` minimum beside a larger ``net: different`` one
   (GF180 ``DN.2a``/``DN.2b``, IHP ``NW.b``/``NW.b1``). The net is looked up at each
   region's marker on the rule's own layer, so the layer must be a conductor in the
   PDK's connect graph; a rule with a ``net`` gate is net-aware, runs net extraction,
   and is skipped under ``--no-connectivity``.

   The two words fail in opposite directions on a pair whose nets do not resolve — a
   marker in no region, a layer outside the connect graph, an untied floating well.
   ``different`` reports it, as the plain geometric rule would, so an unresolved net can
   only ever cost a false positive on the larger minimum. ``same`` goes quiet on it,
   because an unresolved pair is not known to be one net; nothing is lost, since a pair
   below the small minimum is below the large one too, and the companion rule reports
   it. That relies on the rules being written as a pair, which is how they come.

   If the rule's layer is a *merged* derived layer (a ``close``, as with NW.b1's
   ``NWellMergedNoSRAM``), put that merged layer in the connect graph rather than the
   drawn layer: a merged region's marker can land in a gap the close filled in.

``width`` / ``length``
   Optional, µm, alone or together. The rule then applies only where some pair of facing
   edges, one from each region, runs alongside for a projected overlap of more than
   ``length`` with real metal depth of more than ``width`` behind at least one of them
   (IHP ``M1.e``: 0.22 µm between lines wider than 0.3 µm running parallel for more
   than 1.0 µm; ``CntB.b1``: 0.36 µm between bars running parallel for more than
   5 µm, at any width). Either one left out is zero: ``width`` alone binds any parallel
   run of a wide enough line, ``length`` alone any line running parallel long enough.
   Both are measured on the facing edges, not the bounding boxes — an L-shaped or
   stepped pad's box can look wide, or overlap a neighbour for tens of microns, while the
   metal actually running alongside is narrow or short. The run along a wall is the
   joined stretch of every wall facing it under the value - a neighbour whose wall
   steps from 4 to 4.5 µm away, or carries a nick, is still one line running alongside
   (KLayout's projection reads each edge pair on its own and sees two shorter runs).
   The depth is read along the wall, stretch by stretch, so a 0.2 µm line carrying a
   0.5 µm part on its far side is wide along that part, whichever side the part is on.
   A run longer than the tile is read on the pair's regions assembled out to
   ``value + length + width`` around the gap, so the answer does not depend on the tile.

``pairs``
   Optional. ``disjoint`` (the default) measures pairs that share no area at their
   closest approach. ``overlapping`` is for a rule whose two shapes overlap by
   definition (GF180 ``S.PL.5b_MV``: the space from a poly to the COMP it gates), where
   the gap meant is between facing edges elsewhere along the same two shapes.

``metric``
   Optional. ``euclidian`` (the default) or ``square``, KLayout's L-infinity metric for a
   rule worded "must not fall within a d × d square at the corner".

``abutting``
   Optional. ``ignore`` (the default) or ``report``. Two shapes of the two layers drawn
   edge to edge share a run of boundary and no gap; by default that is no violation,
   as a butted tie is drawn edge to edge by design and the GF180 decks read every
   abutment so. ``report`` reads it as a space of nothing: IHP's ``Cnt.e`` (Activ
   against a gate contact), ``Cnt.f``, ``Cnt.g1`` and ``NW.d`` (external N+Activ at the
   well edge) do, and IHP's KLayout deck reports the shared edge. A contact at an
   isolated point - two shapes meeting corner to corner - is a space of zero under
   both. ``related`` is the third reading, IHP's glossary's "unrelated: two regions
   which do not touch each other": two shapes that touch anywhere, along an edge or at
   a corner, are related and no pair at all, wherever else they face each other
   (``pSD.d``: an L-shaped pSD abutting an N+Activ on one edge and 0.175 from the other
   is clean). The touch is looked for across every tile, so a pair touching in one
   tile is no pair in the next.

``gap_outside``
   Optional layer param (under ``layer_params``). A pair whose whole gap - the segment
   between its two closest points - lies inside that layer is not reported. For a rule
   that measures a *particular* material between two shapes: IHP ``NW.b1`` is the width
   of PWell between two wells, and PWell is what is neither NWell nor PWell:block, so a
   gap filled by a PWell block has no PWell to be too narrow (the block-to-well space is
   ``PWB.c``'s), while a block strip in the middle of the gap leaves PWell either side
   and the pair stands. IHP's KLayout deck clips the markers to PWell the same way.

``rows`` / ``cols``
   Optional, "more than N", either one defaulting to ``3``. The rule is then about the
   vias packed into an array larger than ``rows`` × ``cols``, which etch and fill
   differently from a lone pair and want a larger space than the ordinary via rule
   (IHP ``V1.b1``, ``Cnt.b1``; GF180 ``V1.2b``). Detecting an array mirrors the
   reference decks' morphological test, a close followed by an erosion by half a block,
   read directly on the vias: a *run* is a maximal chain of horizontally tight vias, it
   qualifies once longer than ``cols``, and qualifying runs whose x-extents overlap and
   whose vertical gap is tight stack; a stack deeper than ``rows`` is an array. A via
   ring round a pad has long runs but never stacks more than two deep, a single row or
   column never stacks, so neither is an array. Vias are read as rectangles from the
   tiled cache, deduplicated by extent, and an array spans tiles freely. With the gate
   the other gates do not apply.

   ``axes``
      ``1`` (default): the space must reach ``value`` in at least one axis, so only an
      array tight in both directions violates, and it is reported once at its centroid.
      ``2``: the space must hold in both axes, so any tight pair inside an array
      violates, reported as an edge across the first tight gap found under a part of
      the stack at least ``rows`` deep.
   ``pitch``
      With ``axes: 2``, the gap in µm up to which two vias still belong to one array;
      defaults to ``value``. GF180 closes the layer by 0.2 µm, which bridges 0.4.
   ``count``
      The smallest array, in vias, the rule applies to ("interacting with 16 or more
      vias"). Default 0.
   ``min_extent``
      The smallest side of the array's bounding box in µm for it to count: GF180 words
      "4x4 or larger" as a box three vias and three spaces across in every direction.
   ``projection``
      How far two vias must overlap across a gap, in µm, to be neighbours at all
      (KLayout ``projecting >= x``); staggered rows overlapping by less are not.


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap (from the closest point on one
region to the closest point on the other). With ``rows``/``cols``, one marker per array:
a point at its centroid under ``axes: 1``, an edge across the first tight gap under
``axes: 2``.


KLayout equivalent
------------------

``Region#space(value)`` for the same-layer form; ``Region#separation(other, value)`` for
the two-layer form. The gates have no single operator: a deck builds ``angle: bent`` as
a ``space`` restricted to edges matching a 45° angle filter, ``width``/``length`` as a
``space(value, Projection)`` combined with a ``width`` filter on each side, and ``net``
needs a ``Netter``-based extraction driving a ``separation`` between per-net region
sets — the shipped IHP deck omits its potential-dependent well rules rather than
approximate them. The array gate mirrors the decks' array-density idiom: a close of the
via layer by about half the spacing, eroded back by half an array block, which only
survives where vias are dense in both directions, as opposed to a plain ``space(value)``
that a via ring or a single row would also trip.


Examples
--------

.. code-block:: yaml

    - id: Act.b
      check: min_space
      layers: [Activ]
      value: 0.21

    - id: M1.e
      check: min_space
      layers: [Metal1]
      value: 0.22
      params:
        width: 0.3
        length: 1.0

    - id: M1.i
      check: min_space
      layers: [Metal1]
      value: 0.22
      params:
        angle: bent

    - id: NW.b1
      check: min_space
      layers: [NWellMergedNoSRAM]
      value: 1.80
      params:
        net: different

    - id: V1.b1
      check: min_space
      layers: [Via1]
      value: 0.29
      params:
        rows: 3
        cols: 3
