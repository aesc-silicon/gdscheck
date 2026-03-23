.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_extension
=============

Where the cover layer (``layers[0]``) sits over a target region (``layers[1]``), it must
extend at least ``value`` µm past the target's long edges — its width direction. The
target's short edges (its ends) are exempt, since the target legitimately runs out past
the cover there.


Semantics
---------

This is a directional, edge-local check rather than a whole-shape enclosure: it walks
every contour edge of the target and only considers the ones the cover actually overlaps
*just inside* — a target edge with no cover just behind it (the target's short/end edges)
never enters the check at all, so they don't need special-casing by shape.

For each qualifying edge, the engine samples along its length and measures how far the
cover reaches outward past it (capped at ``value``); any span where the cover falls short
becomes a violation, reported as a span along the boundary rather than a single point.
This exact-for-axis-aligned-rectangles approach is what makes "long edges only" work
without the caller having to say which edges are long: an edge only enters the
computation where the cover overlaps its *inside*, which for a rectangular target lying
across a rectangular cover only happens on the two long sides.


Layers
------

Two layers, positional:

1. ``layers[0]`` — the cover (must extend beyond the target).
2. ``layers[1]`` — the target being crossed/covered.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required extension past each long
edge).


Violation markers
------------------

One edge marker per under-extended span along a qualifying target edge, reporting the
worst (smallest) measured extension in that span.


KLayout equivalent
------------------

No single built-in KLayout primitive matches this directly; conceptually closest to an
``extended`` / ``enclosing`` check restricted to a target's long edges, which upstream
IHP decks typically express as a handwritten multi-step DRC script rather than one
operator.


Example
-------

.. code-block:: yaml

    # SalBlock must extend 0.20 µm past the long edges of the Activ/GatPoly it crosses;
    # the target's own ends are exempt.
    - id: Sal.c
      check: min_extension
      layers: [SalBlock, ActivOrGatPoly]
      value: 0.20
