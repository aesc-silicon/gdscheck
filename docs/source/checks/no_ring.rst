.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

no_ring
=======

The layer must not form a closed ring — any enclosed empty region (a hole surrounded on
all sides by the layer's own geometry) is a violation. Used for IHP's ``LBE.h``: the
laser-bond-enhancement layer must not enclose empty space.


Semantics
---------

Builds a coordinate grid from every polygon vertex on the layer, adds a one-DBU sentinel
strip around the outside, and flood-fills from a cell known to be exterior (the sentinel
corner) through every uncovered grid cell reachable from outside. Any uncovered cell
*not* reached that way is enclosed — surrounded by the layer on every side — and its
connected component is one violation. This is a plain point-in-polygon grid-and-flood-fill
over raw (unmerged) boundaries, not the tiled merge cache; it's used on layers small/sparse
enough that this is cheap.


Layers
------

One layer — its own geometry is checked for enclosed holes.


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per enclosed hole, at the centroid of its grid cells.


KLayout equivalent
------------------

``layer.holes()`` reported directly (the layer's own enclosed interior), roughly
equivalent to merging the layer and flagging any hole in the result.


Example
-------

.. code-block:: yaml

    - id: LBE.h
      check: no_ring
      layers: [LBE]
      value: 0.00
