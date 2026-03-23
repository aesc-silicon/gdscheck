.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

ring_covers_boundary
====================

A boundary layer (typically a seal ring's outline) must be fully enclosed by an
**unbroken** ring of another layer. A gap in the ring is a violation. Used for IHP's
``Seal.n``: the seal-ring passivation must form a continuous ring around the die edge.


Semantics
---------

Both layers are merged (globally — this is a chip-perimeter structure, drawn once, not a
dense layer). A point is considered enclosed by the ring if it falls inside a *hole* of
some merged ring region — i.e. surrounded by ring material on every side. Every vertex of
the merged boundary layer is tested this way; if the ring has a gap, the hole that would
have enclosed the boundary opens up into the exterior instead, and the boundary vertices
exposed through that gap are no longer "inside a hole" — they're flagged.


Layers
------

``layers[0]``
   The ring layer (e.g. ``Passiv``) that must form an unbroken loop.
``layers[1]``
   The boundary layer it must enclose (e.g. ``EdgeSeal``).


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per merged boundary region with an exposed vertex, at the first
offending vertex found.


KLayout equivalent
------------------

Conceptually ``boundary.not(ring.holes())`` — the parts of the boundary not sitting
inside a hole of the ring layer.


Example
-------

.. code-block:: yaml

    - id: Seal.n
      check: ring_covers_boundary
      layers: [Passiv, EdgeSeal]
      value: 0.0
