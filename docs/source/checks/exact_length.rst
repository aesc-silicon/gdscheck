.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_length
============

Bounding-box **length** (the long side of a merged region's axis-aligned bounding box)
must be exactly ``value``. The exact counterpart of :doc:`min_length` and
:doc:`max_length`.

Semantics
---------

As :doc:`min_length`, with ``length = max(x1 - x0, y1 - y0)`` compared for equality
against ``value`` rounded to the grid.


Layers
------

One layer.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the required length).


Violation markers
-----------------

One point marker per region whose length differs, at the region's marker.


KLayout equivalent
------------------

None directly; a ``with_bbox_height`` selection inverted.


Example
-------

.. code-block:: yaml

    - id: BAR.l
      check: exact_length
      layers: [ContBar]
      value: 0.5
