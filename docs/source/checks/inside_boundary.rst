.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

inside_boundary
===============

Every shape on every other layer must lie inside the outer edge of a boundary layer
(typically the seal ring). Anything sticking out beyond it is a violation. Used for
IHP's ``Seal.l``: nothing in the design may extend past the die's edge seal.


Semantics
---------

The boundary layer (usually drawn as many segments) is merged; its largest merged region
by area is taken as the real ring, and the bounding box of that region's outer contour is
the reference edge. Every vertex of every shape on every *other* GDS layer/datatype is
then checked against that bounding box (with a half-DBU tolerance); a shape with any
vertex outside is flagged once, at that vertex. Layers listed in the rule's ``ignore``
are skipped entirely — IHP, for instance, doesn't check the edge-seal passivation ring
itself, or the pad layer, against this rule.


Layers
------

One layer — the boundary/seal ring. Every *other* layer in the design is checked against
it implicitly (there's no explicit "target" list).


Parameters
----------

None beyond the rule's ``ignore`` list (layer names to exempt from the boundary check
entirely — not a ``params`` entry, a top-level rule field alongside ``layers``).


Violation markers
------------------

One point violation per offending shape (the first vertex found outside the boundary),
tagged with the raw GDS layer/datatype it came from.


KLayout equivalent
------------------

Conceptually the inverse of :doc:`ring_covers_boundary` — every drawn shape ``outside``
the seal ring's bounding extent, across every layer in the layout, filtered by an
ignore-list.


Example
-------

.. code-block:: yaml

    - id: Seal.l
      check: inside_boundary
      layers: [EdgeSeal]
      value: 0.0
      ignore:
        - Passiv
        - Pad
        - EdgeSeal.boundary
