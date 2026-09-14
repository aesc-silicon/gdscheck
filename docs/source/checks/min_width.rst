.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_width
=========

Every place metal narrows below ``value`` (measured wall-to-wall, perpendicular to the
metal's run) is a violation. The most common width rule — e.g. IHP's ``TM2.a``
(TopMetal2 minimum width).


Semantics
---------

Runs the shared facing-edge width scan (:doc:`shared with max_width and exact_width
<max_width>`) over each merged region's contours (outer ring and holes):

- **Rectilinear pass**: every vertical edge is classified as a left or right wall of
  metal (by which side the fill is on); at each distinct y-band, adjacent left/right
  walls are paired and the gap between them is the metal's width there. The same is done
  for horizontal edges, sweeping x-bands, to catch vertical spans. Together these two
  passes measure ordinary Manhattan metal widths in both directions.
- **Oblique pass**: pairs of mutually anti-parallel diagonal edges (same absolute angle,
  opposite direction) are measured directly, so a 45° trace's width is caught too — this
  pass always runs, even for a plain ``min_width`` rule; it isn't gated on a "bent length"
  the way ``angle: bent`` is. Anti-parallel is read as far as the grid can say: a
  boolean cuts a 45° wall and rounds its new end to the DBU, so the two sides of one bar
  can come out 0.05° apart, and a pair whose gap drifts by under a DBU and a half over
  the run it shares still bounds a width.
- **Mixed pass**: a diagonal edge facing an axis-aligned one — the chamfered corner of a
  well against the straight wall opposite — is measured at its closest approach.
- **Corner pass**: two facing walls whose projections do not overlap — the steps of a
  jog, the two stubs either side of a chamfer — are measured from the near end of one to
  the near end of the other, as KLayout's euclidian metric reads them.
- **Pinches and acute corners**: a vertex the layer touches itself at (two pieces corner
  to corner, a notch tip on a straight wall, a contour through one point twice) is a
  width of zero, reported as a point; so is a corner with less than a right angle of
  material in it, since the wedge narrows to nothing at the tip.

A width is only reported once its projected overlap between the two walls is real (not
just touching at a point), and only from the tile whose core contains the gap's midpoint,
so a wall pair straddling several tiles is reported exactly once.

This scan measures **facing walls of one region**, unlike :doc:`min_dim`, which measures
a whole region's bounding box — a min_width rule catches a narrow neck anywhere in an
irregular shape; min_dim only catches the shape's bounding box being too narrow overall.

The comparison is exact. The rule's µm value is put on the grid once — rounded up for a
minimum, down for a maximum, to nearest for an exact width — and after that an
axis-aligned span is compared as the integer it is, an oblique one squared as a ratio of
integers. A 45° bar drawn 226 DBU apart in y is 159.81 DBU wide, and a 160 rule reports
it.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

``str_params: angle``
   ``bent`` measures only the 45° runs, and only where a run is longer than
   ``bent_length``. A fab may ask more width of a diagonal than of a straight trace, since
   the grid resolves a diagonal as a staircase; that rule sits beside the plain one with
   its own value, and a short chamfer at a corner is not a trace and is left to the plain
   rule.

``params: bent_length``
   With ``angle: bent``: the run (µm) a diagonal wall pair must share before its width is
   considered. Optional, defaults to ``0.5``.


Violation markers
------------------

One edge marker for **each of the two facing walls** of any width below ``value`` (so two
markers per violation location, one on each wall), at the actual wall geometry.


KLayout equivalent
------------------

``Region#width(value)`` — KLayout's own facing-edge width check, including its handling
of 45° edges under the default (90°) angle limit.


Example
-------

.. code-block:: yaml

    - id: TM2.a
      check: min_width
      layers: [TopMetal2]
      value: 2.00
    - id: M2.g
      check: min_width
      layers: [Metal2]
      value: 0.24
      str_params:
        angle: bent
      params:
        bent_length: 0.50
