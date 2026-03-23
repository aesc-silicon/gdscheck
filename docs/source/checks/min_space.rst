.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_space
=========

Minimum spacing between merged regions on one layer, or between two layers.


Semantics
---------

``layers`` takes one entry (same-layer spacing) or two (inter-layer spacing). Both
layers are merged first (so overlapping/nested shapes don't create spurious internal
gaps), then every pair of regions is checked: for same-layer spacing, all distinct
region pairs; for two layers, every region of ``layers[0]`` against every region of
``layers[1]``.

For each candidate pair, a bounding-box pre-filter skips pairs that can't be within
``value`` before doing real geometry work. Surviving pairs are tested for overlap first
— an overlap (or one region sitting inside another's hole) is not a spacing violation
and is skipped. Otherwise the true closest edge-to-edge distance between the two
regions is computed (segment-to-segment, not just vertex-to-vertex, so it's exact for
any polygon shape, not only rectangles). A gap of exactly ``0`` (touching) is never a
violation; a gap strictly between ``0`` and ``value`` is.

Each violation is owned by the tile whose core contains the gap's midpoint, so a pair
visible from several overlapping halo tiles is reported exactly once.


Layers
------

- Same-layer form: one layer — checks every region against every other region of it.
- Two-layer form: ``layers[0]``, ``layers[1]`` — checks every region of the first
  against every region of the second.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required gap).


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap (from the closest point on one
region to the closest point on the other).


KLayout equivalent
------------------

``Region#space(value)`` for the same-layer form; ``Region#separation(other, value)`` for
the two-layer form.


Example
-------

.. code-block:: yaml

    - id: Act.b
      check: min_space
      layers: [Activ]
      value: 0.21
