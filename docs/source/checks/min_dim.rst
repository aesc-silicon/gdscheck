.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_dim
=======

Bounding-box **width** (the short side of a merged region's axis-aligned bounding box)
must be at least ``value``. The dual of :doc:`min_length`, and the bounding-box
counterpart of :doc:`min_width` — see that page for when to use which.


Semantics
---------

For each merged region on the rule's layer, take its axis-aligned bounding box
``(x0, y0)-(x1, y1)`` and compute ``width = min(x1 - x0, y1 - y0)``. A region whose width
is below ``value`` is a violation.

Because this measures the *whole region's* bounding box rather than scanning facing
walls, it is exact only for simple, roughly rectangular features (contact bars, emitter
stripes) — it cannot distinguish a feature's width from its length the way a facing-wall
scan would, which is the point: it lets a rule bound one dimension without the other
dimension's rule firing on it. For arbitrary metal shapes where every narrow neck must be
flagged regardless of orientation, use :doc:`min_width` instead.


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

Not a single built-in KLayout ``Region`` operator — this is a bounding-box measurement
(``Region#extents`` sizes), not the min-space/min-width edge-facing metric KLayout's
``width``/``space`` use. Closest to filtering on ``Region#extents.width``/``.height`` in a
KLayout DRC script.


Example
-------

.. code-block:: yaml

    - id: CntB.a
      check: min_dim
      layers: [ContBar]
      value: 0.16
