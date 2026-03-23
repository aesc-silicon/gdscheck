.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_enclosure
=============

The mirror image of :doc:`min_enclosure`: every shape on the enclosed layer
(``layers[1]``) that sits inside an enclosing region (``layers[0]``) must have **no more**
than ``value`` µm of margin on any side.


Semantics
---------

Uses the same facing-edge-pair (projection metric) margin computation as
:doc:`min_enclosure`, but the *worst* margin here means the largest one, and a shape not
contained by any enclosing region isn't a "too much margin" case — it's silently skipped,
since that absence is `min_enclosure`'s concern, not this rule's. There's no
``skip_coincident``/``skip_clipped`` equivalent: a coincident (0-margin) edge can never
exceed a maximum bound, so it's harmless to include, and there's no "wall reality check"
either — a fake wall (from a per-tile union truncated at its halo) can only *shrink* the
measured worst margin, which for a maximum bound errs toward passing rather than a false
violation.

Typically paired with a `min_enclosure` entry at the same value, pinning a device's
margin to (near) exactly that value rather than just bounding it from below.


Layers
------

Two layers, positional — same roles as :doc:`min_enclosure`:

1. ``layers[0]`` — the enclosing region.
2. ``layers[1]`` — the enclosed shapes being measured.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the maximum allowed margin).


Violation markers
------------------

One edge marker per offending shape, along the facing outer wall responsible for the
largest (worst) margin.


KLayout equivalent
------------------

``inner.enclosed(outer, value, metric: RBA::Region::projection)`` filtered to the
*maximum*-margin direction — KLayout doesn't have a dedicated "max enclosure" primitive
either; this is the same projection-metric measurement as `min_enclosure`, just compared
against an upper bound.


Example
-------

.. code-block:: yaml

    # A Schottky diode's PWell.block enclosure is pinned to exactly 0.25 µm: the min_
    # enclosure entry sets the floor, this one the ceiling.
    - id: Sdiod.a
      check: min_enclosure
      layers: [PWell.block, SchottkyContBar]
      value: 0.25
      params:
        interacting_only: 1
    - id: Sdiod.a
      check: max_enclosure
      layers: [PWell.block, SchottkyContBar]
      value: 0.25
