.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_extension
=============

The mirror image of :doc:`min_extension`: where the cover layer (``layers[0]``) sits over
a target region (``layers[1]``), it may extend at most ``value`` µm past the target's
edges it crosses.


Semantics
---------

The same edge-local scan as :doc:`min_extension`: a target edge counts only where the
cover overlaps the target just inside it, so the target's free edges are exempt of
themselves. Along each qualifying edge the cover is probed just past ``value`` outward;
any span where it is still there is a violation, reported with the largest extension
measured in the span. An extension of exactly ``value`` is not too much.


Layers
------

Two layers, positional, as in :doc:`min_extension`:

1. ``layers[0]`` — the cover.
2. ``layers[1]`` — the target being crossed.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the maximum allowed extension).


Violation markers
------------------

One edge marker per over-extended span along a qualifying target edge.


KLayout equivalent
------------------

None directly, as for :doc:`min_extension`.


Example
-------

.. code-block:: yaml

    - id: X.max
      check: max_extension
      layers: [Cover, Track]
      value: 0.5
