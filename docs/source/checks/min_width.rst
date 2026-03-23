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
  the way :doc:`min_45_width` is.

A width is only reported once its projected overlap between the two walls is real (not
just touching at a point), and only from the tile whose core contains the gap's midpoint,
so a wall pair straddling several tiles is reported exactly once.

This scan measures **facing walls of one region**, unlike :doc:`min_dim`, which measures
a whole region's bounding box — a min_width rule catches a narrow neck anywhere in an
irregular shape; min_dim only catches the shape's bounding box being too narrow overall.

It does *not* pair a rectilinear wall against an oblique one (a diagonal stroke closing in
on a straight stem) — only rectilinear-vs-rectilinear and oblique-vs-oblique pairs are
currently measured for width. :doc:`min_notch` had the same limitation for its dual
(the empty-space case) until it was closed by adding a mixed rectilinear/oblique pass;
the width side of that gap is still open.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

None — only ``value`` (µm).


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
