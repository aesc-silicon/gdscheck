.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_45_width
============

Minimum width for a metal trace that runs at 45°, checked only where the diagonal run is
longer than ``bent_length`` — short chamfers/corner cuts are exempt. Used e.g. for IHP's
``M5.g`` (Metal5 45°-bend minimum width).


Semantics
---------

Reuses the shared facing-edge width scan (see :doc:`min_width`), but with the rectilinear
passes skipped entirely (``oblique_only``) — axis-aligned metal is already covered by an
ordinary :doc:`min_width` rule on the same layer, so this check only measures the oblique
(anti-parallel diagonal edge pair) pass, and only reports a pair whose projected overlap
run is longer than ``bent_length`` (converted to DBU). A short diagonal chamfer at a
corner, too short to be a real bent trace, is ignored regardless of how narrow it is.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

``bent_length``
   Minimum run length (µm) a diagonal wall pair must have, projected onto its own
   direction, before its width is even considered. Optional, defaults to ``0.5``.


Violation markers
------------------

One edge marker for **each of the two facing diagonal walls** of any 45° width below
``value`` (two markers per violation location), spanning the offending run.


KLayout equivalent
------------------

``Region#width(value, angle_limit(45))`` restricted to diagonal edges, combined with a
minimum-length filter on the edge pair — no single built-in operator does both at once,
so the reference deck composes it from several steps; gdscheck's version folds the
length gate directly into the oblique width scan.


Example
-------

.. code-block:: yaml

    - id: M5.g
      check: min_45_width
      layers: [Metal5]
      value: 0.24
      params:
        bent_length: 0.50
