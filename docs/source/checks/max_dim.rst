.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_dim
=======

Bounding-box **width** (the short side of a merged region's axis-aligned bounding box)
must be at most ``value``. The dual of :doc:`max_length`, and the bounding-box
counterpart of :doc:`max_width`.


Semantics
---------

For each merged region on the rule's layer, take its axis-aligned bounding box
``(x0, y0)-(x1, y1)`` and compute ``width = min(x1 - x0, y1 - y0)``. A region whose width
exceeds ``value`` is a violation.

As with :doc:`min_dim`, this is a whole-region bounding-box measurement, not a facing-wall
scan — exact for simple rectangular features, and useful precisely because it leaves the
region's other dimension (its length) to a separate rule.


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

Not a single built-in KLayout ``Region`` operator — a bounding-box measurement
(``Region#extents`` sizes), analogous to filtering on ``Region#extents.width``/``.height``
in a KLayout DRC script.


Example
-------

.. code-block:: yaml

    - id: CntB.a
      check: max_dim
      layers: [ContBar]
      value: 0.16
