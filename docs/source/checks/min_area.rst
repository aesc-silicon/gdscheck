.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_area
========

The area of each connected region of a layer must be at least ``value`` (µm²).


Semantics
---------

Shapes that overlap or abut form one region and are measured together — this is not a
per-shape check. Regions are reconstructed from the shared merge cache by stitching the
per-tile merge across tile borders (``MergedCache::regions``), so even a region spanning
many tiles is measured exactly once, with bounded memory regardless of layer density.

Each region smaller than ``value`` is flagged once, at a representative point inside it.


Layers
------

One layer per rule entry (repeat the rule for multiple layers). Every region of every
listed layer is checked independently.


Parameters
----------

None. ``value`` is the minimum region area in µm².


Violation markers
------------------

One point marker per undersized region, at its centroid-derived marker point.


KLayout equivalent
------------------

``layer.with_area(nil, value)`` (regions strictly below the floor) — KLayout's
``area < value`` filter on a merged region.


Example
-------

.. code-block:: yaml

    - id: Gat.e
      check: min_area
      layers: [GatPoly]
      value: 0.09
