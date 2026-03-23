.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_windowed_density
====================

The chip is split into ``window`` × ``window`` µm tiles, and the combined density of one
or more layers must be at least ``value`` (%) in *every* tile.


Semantics
---------

Coverage is computed on the shared merge cache (so overlapping/nested shapes are not
double-counted) and summed exactly via polygon clipping to each window, accumulated from
the cache's own tiles so a region spanning cache tiles is still counted once. This is the
tiled analogue of :doc:`min_density`: instead of one chip-wide percentage, every
``window``-sized tile gets its own.

The tile grid always starts at the chip's raw bounding box — the first window is
``[0, window)`` in each axis, regardless of ``boundary_layer``. Real chip dimensions are
rarely an exact multiple of the window size, so the last row/column of tiles is usually
smaller than a full window, and may extend past the actual seal ring entirely.

That's what the optional ``boundary_layer``/``boundary_datatype`` params are for — the
same convention :doc:`min_density` uses. They restrict *which area within each window
counts as checkable* to that layer's bounding box, not its drawn area (a seal ring is a
hollow frame; its own merged area would wildly undercount the die it encloses). Without
this, an edge or corner window that falls outside the seal ring — or only partly inside
it — is measured against its full nominal window area and can fail the density floor for
a reason that has nothing to do with underfill: there was simply nothing there to fill. A
window with zero overlap with the boundary box is skipped outright (density is undefined
for a zero-area denominator, not a violation).

This historical failure mode is worth calling out explicitly, since it's exactly the bug
this param set was added to fix: on a 900×900 µm die with an 800 µm window and no
``boundary_layer``, the last row and column of tiles are measured against a doubled
(200 µm-tall) denominator, of which only the first 100 µm can ever hold real geometry —
reading roughly half the true density and falsely tripping the floor.


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

``layer.with_density(nil, value, tile_size, tile_boundary)`` — a genuinely tiled density
check (``tile_count`` left at its default, one tile per window), with the tile boundary
set from the seal-ring layer.


Example
-------

.. code-block:: yaml

    - id: M4Fil.h
      check: min_windowed_density
      layers: [Metal4, Metal4.filler, Metal4.mask]
      value: 25.00
      params:
        window: 800.0
        boundary_layer: 39  # EdgeSeal — density is only meaningful inside the seal ring
