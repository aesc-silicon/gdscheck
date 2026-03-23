.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

offgrid
=======

Every vertex of every shape on the rule's layers must land on the manufacturing grid;
any that don't are violations. Used across the board for grid-conformance rules, e.g.
IHP's ``Activ.offgrid`` at a 5 nm grid.


Semantics
---------

Operates directly on raw (unmerged) GDS boundaries — no tiling or merging needed, since
each vertex is checked independently against the grid. ``value`` (µm) is converted to a
DBU grid size; a vertex whose x or y coordinate isn't an exact multiple of that grid size
is flagged.


Layers
------

One or more layers; every vertex on every one of them is checked against the same grid.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the grid size — e.g. ``0.005`` for a 5 nm
grid). Must resolve to at least 1 DBU; a smaller grid than the design's own database
unit is rejected.


Violation markers
------------------

One point violation per off-grid vertex.


KLayout equivalent
------------------

``layer.ongrid(grid)`` — flags vertices not on the given grid.


Example
-------

.. code-block:: yaml

    - id: Activ.offgrid
      check: offgrid
      value: 0.005
      layers: [Activ, Activ.mask, Activ.filler]
