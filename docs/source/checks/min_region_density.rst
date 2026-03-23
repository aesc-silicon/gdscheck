.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_region_density
==================

For each connected region of a *base* layer that's big enough to matter, the fraction of
its area covered by a *feature* layer must be at least ``value`` (%).


Semantics
---------

Unlike :doc:`min_density` and :doc:`min_windowed_density`, which measure a chip-wide or
per-window percentage, this measures density *per connected region* of the base layer —
each qualifying region gets its own pass/fail, so a single starved region can't be
averaged out by well-covered neighbours elsewhere on the chip.

"Big enough to matter" means at least ``min_size`` µm across in every direction, found by
a tiled erosion of radius ``min_size / 2`` (``MergedCache::plate_regions``): a region that
fully vanishes under that erosion has no spot wide enough to qualify and is skipped
entirely, regardless of its total area (a long, thin sliver never trips this rule, no
matter how large). Both the base and feature layers stay on the cache's tiled
representation throughout — a large metal plate is never globally unioned.

For each region that *is* wide enough, the check compares the feature layer's enclosed
area against the base region's true filled area (not its bounding box) and reports a
violation if the ratio falls short.


Layers
------

Two layers, positional:

1. Base layer — the region a percentage is measured over (e.g. a large metal plate).
2. Feature layer — the layer whose coverage inside each base region is measured (e.g.
   metal slits).


Parameters
----------

``min_size``
   The minimum span (µm) a base region's widest spot must have to be checked at all.
   Defaults to ``35.0``.


Violation markers
------------------

One point marker per undersized region, at a point that always lands on the region's
"wide" spot (not just anywhere in its bounding box).


KLayout equivalent
------------------

Not a single built-in operator — mirrors a hand-rolled per-plate density measurement (as
used for slit-density rules like IHP's Slt.i): find wide plates via a sized/unsized
erosion round-trip, then measure feature coverage inside each one individually rather
than as one chip-wide ratio.


Example
-------

.. code-block:: yaml

    - id: Slt.i
      check: min_region_density
      layers: [Metal1NoExempt, Metal1.slit]
      value: 6.00
      params:
        min_size: 35.0
