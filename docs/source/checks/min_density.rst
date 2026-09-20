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
double-counted, and several layers are read as their union) is summed on the shared
merge cache and divided by the area the scope names.

``scope: chip`` (default)
   One chip-wide percentage. The denominator is the chip's bounding box: the box of
   *every* shape in the design, or, with the ``boundary`` layer param, that layer's
   bounding box. The bounding box, not the layer's own merged area, is what is used: a
   seal ring is drawn as a hollow frame around the die, so its own area is the thin
   frame and would wildly undercount the region it stands for.
``scope: window``
   "Any ``window`` × ``window`` µm area": the windows slide over the die a merge tile at
   a time (20 µm by default) from the die's corner, and one more is laid against each
   far edge, so every part of the die lies in some whole window and no window is a
   clipped remainder. The die is the ``boundary`` layer's bounding box when the rule
   names one, else the chip's raw bounding box; a die narrower than the window in a
   direction gets one window there, cut to the die. The coverage is summed once per
   merge tile and the windows on the tile grid are read off a summed-area table, so the
   number of windows costs nothing; the edge-anchored windows are clipped exactly.
   Overlapping violating windows are one violation, reported at the worst of them. IHP's
   own deck lays its windows in steps of half a window with the same edge anchoring; the
   finer step here only finds what falls between its windows.
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
