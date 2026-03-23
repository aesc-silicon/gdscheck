.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_density
===========

The combined density of one or more layers, over the whole chip, must be at most
``value`` (%).


Semantics
---------

The dual of :doc:`min_density`, sharing the same denominator logic: the listed layers'
merged coverage is summed and divided by the chip's bounding box (or, with
``boundary_layer`` set, that layer's bounding box — not its own drawn area, for the same
hollow-seal-ring reason described in :doc:`min_density`).

Together, a ``min_density``/``max_density`` pair on the same layers forms the usual
"keep fill density within [floor, ceiling]" rule pair.


Layers
------

One or more layers; their merged areas are summed for the numerator.


Parameters
----------

``boundary_layer``
   GDS layer number for the density denominator's bounding box. Optional — if omitted (or
   the layer has no shapes), the bounding box of every shape in the design is used
   instead.

``boundary_datatype``
   GDS datatype paired with ``boundary_layer``. Optional, defaults to ``0``.


Violation markers
------------------

One chip-wide (global, no coordinates) marker if the computed density exceeds ``value``.


KLayout equivalent
------------------

``layer.without_density(value, nil, tile_boundary)`` — a single-tile (whole-chip) density
check, with the tile boundary set from the seal-ring layer rather than the raw layout
bounding box.


Example
-------

.. code-block:: yaml

    - id: TM2.d
      check: max_density
      layers: [TopMetal2, TopMetal2.filler, TopMetal2.mask]
      value: 70.00
      params:
        boundary_layer: 39
