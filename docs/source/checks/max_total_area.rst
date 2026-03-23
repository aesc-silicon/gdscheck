.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_total_area
==============

The *sum* of every connected region's area on a layer, across the whole chip, must be at
most ``value`` (µm²).


Semantics
---------

Unlike :doc:`max_area`, which caps each region individually, this sums the area of every
region (again reconstructed via ``MergedCache::regions``, so a dense layer is never
globally unioned) and compares the running total against the limit. One violation is
reported for the whole chip if the total is exceeded — not one per region.

Used for chip-wide caps where many small, individually-compliant regions could still add
up to a problem, e.g. a recommended ceiling on total MIM capacitor area per die.


Layers
------

One layer per rule entry (repeat the rule for multiple layers). The total is computed
independently per listed layer.


Parameters
----------

None. ``value`` is the maximum total area in µm², summed over every region of the layer.


Violation markers
------------------

At most one point marker per layer (not per region), anchored at the first region's
marker point, reporting the actual total area found.


KLayout equivalent
------------------

``layer.area > value`` — KLayout's whole-region-set area sum, as opposed to
``with_area`` which filters individual regions.


Example
-------

.. code-block:: yaml

    - id: MIM.gR
      check: max_total_area
      layers: [MIM]
      value: 174800.00
