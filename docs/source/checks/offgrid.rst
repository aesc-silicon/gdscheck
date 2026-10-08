.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

offgrid
=======

Every vertex of every shape on the rule's layers must land on a grid; any that don't are
violations. Used across the board for grid-conformance rules, e.g. IHP's
``Activ.offgrid`` at a 5 nm grid, and with ``facing`` and ``offset`` for a routing
layer's own grid - ASAP7's ``M4.AUX.1``, every horizontal edge of M4 at a multiple of
24 nm from an offset set per design.


Semantics
---------

Reads the merged layer, not the drawn shapes: a vertex inside another shape of the same
layer is nobody's corner once the layer is one region, and that is what a foundry's check
reads. ``value`` (µm) is the grid in DBU; a vertex whose coordinate is not ``offset`` plus
a whole multiple of it is flagged. Without ``facing`` both coordinates are read; with it,
the one across that axis - ``facing: y`` reads the y of every vertex, which is where the
horizontal edges run.

Each vertex is counted in the tile whose core owns it, so a shape across a tile line is
read once, and the cut the tiling makes at the line adds no vertex of its own.


Layers
------

One or more layers; every vertex on every one of them is checked against the same grid.


Parameters
----------

``value`` (µm)
   The grid - e.g. ``0.005`` for a 5 nm grid. Must resolve to at least 1 DBU; a smaller
   grid than the design's own database unit is rejected.

``facing`` (``x`` or ``y``, optional)
   Read only the coordinate across this axis. Absent, both coordinates are read.

``offset`` (µm, default ``0``)
   Where the grid starts. On the DBU grid.


Violation markers
------------------

One point violation per off-grid vertex.


KLayout equivalent
------------------

``layer.ongrid(grid)`` flags vertices not on the given grid; ``ongrid(gx, gy)`` with one
of them ``0`` reads one coordinate, as ``facing`` does. Neither takes an offset.


Example
-------

.. code-block:: yaml

    - id: Activ.offgrid
      check: offgrid
      value: 0.005
      layers: [Activ, Activ.mask, Activ.filler]

    - id: M4.AUX.1
      check: offgrid
      layers: [M4NoSram]
      value: 0.024
      params:
        facing: y
        offset: 0.0
