.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_area
========

The area of each connected region of a layer must be at most ``value`` (µm²).


Semantics
---------

The dual of :doc:`min_area`: every connected region (shapes that overlap or abut, merged
and stitched across tile borders via ``MergedCache::regions``) is measured once, and
flagged if its area exceeds the ceiling. Used for device-scale caps where a single
oversized region (rather than the chip-wide total) is the concern — e.g. a single MIM
capacitor plate that's too large.

See :doc:`max_total_area` for capping the *sum* of every region's area instead of each
region individually.


Layers
------

One layer per rule entry (repeat the rule for multiple layers). Every region of every
listed layer is checked independently.


Parameters
----------

None. ``value`` is the maximum region area in µm².


Violation markers
------------------

One point marker per oversized region, at its centroid-derived marker point.


KLayout equivalent
------------------

``layer.with_area(value, nil)`` (regions strictly above the ceiling) — KLayout's
``area > value`` filter on a merged region.


Example
-------

.. code-block:: yaml

    - id: MIM.g
      check: max_area
      layers: [MIM]
      value: 5625.00
