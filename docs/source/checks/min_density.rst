.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_density
===========

The combined density of one or more layers, over the whole chip, must be at least
``value`` (%).


Semantics
---------

The listed layers' merged coverage (overlapping/nested shapes are not double-counted) is
summed and divided by a chip-area denominator. For the standard density rules the listed
layers are disjoint (drawing / filler / mask occupy different datatypes), so the sum is
just their union area.

The denominator is the chip's bounding box — either the bounding box of *every* shape in
the design, or, if the ``boundary`` layer param is set, that specific layer's bounding
box instead. The bounding box, not the layer's own drawn/merged area, is what's used: a
seal ring is drawn as a hollow frame around the die, so its own merged area is only the
thin frame material and would wildly undercount the region it's meant to stand in for.
Using its bounding box instead gives the true die extent. See :doc:`min_windowed_density`
for the tiled version of this same convention.

This is a single, whole-chip percentage — not per-region and not per-tile. See
:doc:`min_windowed_density` for a sliding-window variant, and :doc:`min_region_density`
for a per-connected-region variant.


Layers
------

One or more layers; their merged areas are summed for the numerator.


Parameters
----------

``layer_params: boundary``
   The layer whose bounding box is the density denominator, by name — a seal ring or a
   die outline. Optional; if omitted, or the layer has no shapes, the bounding box of
   every shape in the design is used instead.


Violation markers
------------------

One chip-wide (global, no coordinates) marker if the computed density falls below
``value``.


KLayout equivalent
------------------

``layer.with_density(nil, value, tile_boundary)`` — a single-tile (whole-chip) density
check, with the tile boundary set from the seal-ring layer rather than the raw layout
bounding box.


Example
-------

.. code-block:: yaml

    - id: TM2.c
      check: min_density
      layers: [TopMetal2, TopMetal2.filler, TopMetal2.mask]
      value: 25.00
      layer_params:
        boundary: EdgeSeal.boundary
