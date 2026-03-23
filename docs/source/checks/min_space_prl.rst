.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_space_prl
=============

Width- and parallel-run-conditional spacing: a wider minimum space applies only between
lines that are both *wide* and run *parallel* for a long distance (e.g. IHP ``TM2.bR``,
``M5.e``/``M5.f``, ``CntB.b1``).


Semantics
---------

Reuses the tiled region-pair spacing engine from :doc:`min_space`, gated by an extra
condition on each candidate pair: ``value`` only applies where some pair of facing edges
(one from each region) satisfies *all* of —

- their perpendicular separation is under the gap being measured;
- their projected overlap ("parallel run") exceeds ``parallel_run``;
- at least one of the two edges has real metal *depth* behind it greater than
  ``wide_width`` (the local line width at that edge, not the region's bounding-box
  size — an L-shaped or stepped pad's box can look wide for tens of microns while the
  metal running alongside a neighbour is actually narrow there, which must not trigger
  the wide-line spacing).

Because line width and parallel run are measured from real facing edges rather than
bounding boxes, this stays exact for L-shaped, stepped, or comb-like layouts (common in
I/O cells and pad rings), not just plain rectangles.


Layers
------

One layer (same-layer spacing) or two, exactly as in :doc:`min_space`.


Parameters
----------

``wide_width``
   Required. µm. A facing edge only makes the pair "wide" if the metal depth behind it
   exceeds this.

``parallel_run``
   Required. µm. The facing-edge pair's projected overlap must exceed this for the
   conditional spacing to apply.


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap — same convention as
:doc:`min_space`.


KLayout equivalent
------------------

Not a single built-in operator: KLayout decks typically express this with
``Region#space(value, Region::Projection).with_angle(...)`` combined with a
``width`` filter on each side, or an equivalent parallel-run/width-conditional
construction.


Example
-------

.. code-block:: yaml

    - id: TM2.bR
      check: min_space_prl
      layers: [TopMetal2NotIND]
      value: 5.00
      params:
        wide_width: 5.00
        parallel_run: 50.00
