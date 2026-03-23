.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_windowed_density
====================

The chip is split into ``window`` × ``window`` µm tiles, and the combined density of one
or more layers must be at most ``value`` (%) in *every* tile.


Semantics
---------

The dual of :doc:`min_windowed_density`, sharing the same tiling and denominator logic —
see that page for the full explanation of the ``boundary_layer``/``boundary_datatype``
params and why they use a bounding box rather than the boundary layer's own drawn area.
A ``min_windowed_density``/``max_windowed_density`` pair on the same layers, window and
boundary forms the usual "keep local fill density within [floor, ceiling]" rule pair,
evaluated tile-by-tile rather than as one chip-wide number.


Layers
------

One or more layers; their merged areas are summed for the numerator, per window.


Parameters
----------

``window``
   Required. The tile size in µm (square tiles).

``boundary_layer``
   GDS layer number for the per-window density denominator's bounding box (e.g. a seal
   ring). Optional — if omitted, each window's full nominal area is the denominator.

``boundary_datatype``
   GDS datatype paired with ``boundary_layer``. Optional, defaults to ``0``.


Violation markers
------------------

One edge marker per offending window, spanning its four corners, reporting the window's
actual density.


KLayout equivalent
------------------

``layer.without_density(value, nil, tile_size, tile_boundary)`` — a genuinely tiled
density check (``tile_count`` left at its default, one tile per window), with the tile
boundary set from the seal-ring layer.


Example
-------

.. code-block:: yaml

    - id: M4Fil.k
      check: max_windowed_density
      layers: [Metal4, Metal4.filler, Metal4.mask]
      value: 75.00
      params:
        window: 800.0
        boundary_layer: 39  # EdgeSeal — density is only meaningful inside the seal ring
