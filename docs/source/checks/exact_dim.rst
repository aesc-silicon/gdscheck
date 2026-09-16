.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_dim
=========

Bounding-box **width** (the short side of a merged region's axis-aligned bounding box)
must be exactly ``value``: a fixed-size feature such as a slot or a contact bar of one
width. The exact counterpart of :doc:`min_dim` and :doc:`max_dim`, and the bounding-box
counterpart of :doc:`exact_width`.

Semantics
---------

As :doc:`min_dim`, with ``width = min(x1 - x0, y1 - y0)`` compared for equality against
``value`` rounded to the grid. Exact for axis-aligned features; a 45° bar's box is not the
bar.


Layers
------

One layer.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the required width).


Violation markers
-----------------

One point marker per region whose width differs, at the region's marker.


KLayout equivalent
------------------

None directly; a ``with_bbox_width`` selection inverted.


Example
-------

.. code-block:: yaml

    - id: SLOT.w
      check: exact_dim
      layers: [metal1_slot_rect]
      value: 2.0
