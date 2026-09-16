.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_density
===========

The combined density of one or more layers must be at least ``value`` (%): over the
whole chip, in every window of a grid laid over it, or in each large connected region
of a base layer, as ``scope`` says.


Semantics
---------

The listed layers' merged coverage (overlapping and nested shapes are not
double-counted) is summed on the shared merge cache and divided by the area the scope
names. For the standard density rules the listed layers are disjoint (drawing, filler
and mask occupy different datatypes), so the sum is just their union area.

``scope: chip`` (default)
   One chip-wide percentage. The denominator is the chip's bounding box: the box of
   *every* shape in the design, or, with the ``boundary`` layer param, that layer's
   bounding box. The bounding box, not the layer's own merged area, is what is used: a
   seal ring is drawn as a hollow frame around the die, so its own area is the thin
   frame and would wildly undercount the region it stands for.
``scope: window``
   The chip is split into ``window`` × ``window`` µm tiles and every tile gets its own
   percentage, summed exactly by clipping the cache's regions to each window, so a region
   spanning cache tiles is still counted once. The grid always starts at the chip's raw
   bounding box, whatever ``boundary`` says. Chip dimensions are rarely a multiple of
   the window, so the last row and column of tiles are usually smaller than a full
   window and may extend past the seal ring entirely. That is what ``boundary`` is for
   here: it restricts what part of each window counts to the boundary layer's bounding
   box. Without it an edge or corner window that falls outside the seal ring is measured
   against its full nominal area and fails the floor for a reason that has nothing to do
   with underfill. On a 900×900 µm die with an 800 µm window the last row was measured
   against a 200 µm tall denominator of which only 100 µm could hold geometry, reading
   half the true density. A window with no overlap with the boundary box is skipped, not
   failed.
``scope: region``
   Two layers, positional: a *base* layer whose connected regions are measured, and a
   *feature* layer whose coverage inside each of them is the density. Only regions at
   least ``min_size`` µm across in every direction are checked, found by a tiled erosion
   of radius ``min_size / 2``: a region that vanishes under it has no spot wide enough
   and is skipped whatever its total area, so a long thin sliver never trips the rule.
   The percentage is the feature's area inside the region against the region's true
   filled area, not its bounding box, and each region passes or fails on its own, so a
   single starved plate cannot be averaged out by well-covered neighbours. Both layers
   stay on the cache's tiles throughout; a large metal plate is never globally unioned.


Layers
------

``scope: chip``, ``scope: window``
   One or more layers; their merged areas are summed for the numerator.
``scope: region``
   ``layers[0]`` the base layer (a large metal plate), ``layers[1]`` the feature layer
   measured inside it (metal slits).


Parameters
----------

``scope``
   ``chip`` (default), ``window`` or ``region``.
``window``
   The tile size in µm, square; required with ``scope: window``.
``min_size``
   With ``scope: region``: the span in µm a base region's widest spot must have to be
   checked at all. Defaults to ``35.0``.
``layer_params: boundary``
   The layer whose bounding box is the denominator (``chip``) or bounds each window's
   denominator (``window``), by name: a seal ring or a die outline. Optional; without
   it the bounding box of every shape in the design, or each window's full nominal area,
   serves.


Violation markers
------------------

``chip``: one chip-wide marker without coordinates if the density falls below
``value``. ``window``: one edge marker per offending window, spanning its four corners,
reporting the window's density. ``region``: one point marker per starved region, at its
wide spot.


KLayout equivalent
------------------

``layer.with_density(nil, value, tile_boundary)`` for the chip, a single tile;
``layer.with_density(nil, value, tile_size, tile_boundary)`` per window, one tile per
window. The region scope is not a built-in operator; it mirrors a hand-rolled per-plate
measurement such as IHP's Slt.i: find the wide plates by a sized and unsized erosion
round trip, then measure the feature's coverage inside each one.


Examples
--------

.. code-block:: yaml

    - id: TM2.c
      check: min_density
      layers: [TopMetal2, TopMetal2.filler, TopMetal2.mask]
      value: 25.00
      layer_params:
        boundary: EdgeSeal.boundary
    - id: M4Fil.h
      check: min_density
      layers: [Metal4, Metal4.filler, Metal4.mask]
      value: 25.00
      params:
        scope: window
        window: 800.0
      layer_params:
        boundary: EdgeSeal.boundary  # density is only meaningful inside the seal ring
    - id: Slt.i
      check: min_density
      layers: [Metal1NoExempt, Metal1.slit]
      value: 6.00
      params:
        scope: region
        min_size: 35.0
