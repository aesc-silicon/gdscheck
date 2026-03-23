.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_length
==========

Bounding-box **length** (the long side of a merged region's axis-aligned bounding box)
must be at most ``value``. The dual of :doc:`max_dim`.


Semantics
---------

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

Not a single built-in KLayout ``Region`` operator — a bounding-box measurement, analogous
to filtering on ``Region#extents.width``/``.height`` (whichever is larger) in a KLayout
DRC script.


Example
-------

.. code-block:: yaml

    - id: npn13G2.a
      check: max_length
      layers: [EmitG2]
      value: 0.90
