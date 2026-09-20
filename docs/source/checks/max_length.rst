.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_length
==========

Bounding-box **length** (the long side of a merged region's axis-aligned bounding box)
must be at most ``value``. The dual of :doc:`max_dim`.


Semantics
---------

The extent is a difference of two grid coordinates and is compared exactly against
``value`` on the grid.

For each merged region on the rule's layer, take its axis-aligned bounding box
``(x0, y0)-(x1, y1)`` and compute ``length = max(x1 - x0, y1 - y0)``. A region whose
length exceeds ``value`` is a violation.

Commonly paired with :doc:`min_length` to bound a stripe-like feature's length to a
range, as IHP's NPN transistor emitter-length rules do (``[0.90, 0.90]``,
``[1.00, 2.50]``, ``[1.00, 5.00]`` for the three emitter variants).


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

None — only ``value`` (µm).


Violation markers
------------------

One point marker per offending merged region, at its centroid (owned by the tile whose
core contains that centroid).


KLayout equivalent
------------------

``Region#with_bbox_max(value + one grid step, nil)``: the polygons whose bounding box's
larger side is over ``value``, which is how IHP's deck reads its metal-filler maxima
(``MnFil.a2``, ``TMnFil.a1``) and ``LBE.b``.


Example
-------

.. code-block:: yaml

    - id: npn13G2.a
      check: max_length
      layers: [EmitG2]
      value: 0.90
