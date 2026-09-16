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
regions is computed (segment-to-segment, not just vertex-to-vertex, so it's exact for
any polygon shape, not only rectangles). Two shapes meeting at an isolated point are a
gap of zero and a violation; two shapes drawn edge to edge along a run abut and are not.

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
   metal actually running alongside is narrow or short.

``pairs``
   Optional. ``disjoint`` (the default) measures pairs that share no area at their
   closest approach. ``overlapping`` is for a rule whose two shapes overlap by
   definition (GF180 ``S.PL.5b_MV``: the space from a poly to the COMP it gates), where
   the gap meant is between facing edges elsewhere along the same two shapes.

``metric``
   Optional. ``euclidian`` (the default) or ``square``, KLayout's L-infinity metric for a
   rule worded "must not fall within a d × d square at the corner".


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap (from the closest point on one
region to the closest point on the other).


KLayout equivalent
------------------

``Region#space(value)`` for the same-layer form; ``Region#separation(other, value)`` for
the two-layer form. The gates have no single operator: a deck builds ``angle: bent`` as
a ``space`` restricted to edges matching a 45° angle filter, ``width``/``length`` as a
``space(value, Projection)`` combined with a ``width`` filter on each side, and ``net``
needs a ``Netter``-based extraction driving a ``separation`` between per-net region
sets — the shipped IHP deck omits its potential-dependent well rules rather than
approximate them.


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
