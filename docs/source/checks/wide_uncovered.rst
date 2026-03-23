.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

wide_uncovered
==============

A connected region of a base layer that is *wide* (contains a spot at least ``value`` µm
across, in every direction) must enclose at least one shape of a feature layer; a wide
region with none is a violation. Used for IHP's ``Slt.c``: a metal plate wider than
30 µm must contain slots (to avoid CMP dishing / plating stress), unless it's exempted
(pads, MIM, inductors).


Semantics
---------

Runs on ``MergedCache::plate_regions``, the same tiled, halo-bounded region analysis
:doc:`min_region_density` uses: "wide" is determined by a tiled erosion of radius
``value / 2`` (a region that survives erosion by half the width threshold has a spot at
least ``value`` across somewhere), and each region's enclosed feature area is accumulated
from the same per-tile pass — so a dense base layer (a chip-wide metal plate) is never
globally unioned. A region is flagged only when it's wide **and** its feature area is
(effectively) zero.


Layers
------

``layers[0]``
   The base layer to analyze for width (e.g. ``Metal2NoExempt`` — metal with
   pad/MIM/inductor areas already excluded upstream).
``layers[1]``
   The feature layer a wide region must contain (e.g. ``Metal2.slit``).


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum width that requires the feature).


Violation markers
------------------

One point violation per wide, feature-less region, at a point actually on the flagged
wide spot (not just the region's overall centroid).


KLayout equivalent
------------------

Roughly ``base.sized(-value/2).sized(value/2).not_interacting(feature)`` — an erosion
("is there a wide spot at all") composed with "does this region contain the required
feature."


Example
-------

.. code-block:: yaml

    - id: Slt.c
      check: wide_uncovered
      layers: [Metal1NoExempt, Metal1.slit]
      value: 30.00
