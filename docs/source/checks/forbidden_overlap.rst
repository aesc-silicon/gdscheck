.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

forbidden_overlap
=================

The geometric intersection of all the rule's layers must be empty; any overlap is a
violation. Used for "A on B is not allowed" rules such as a contact landing on both
gate poly and diffusion at once.


Semantics
---------

Computed as the ``Intersection`` residual over the cached tiles — see
:doc:`../virtual-ops`. This is the inverse predicate of :doc:`coverage`: coverage flags
the part of a layer *outside* a required cover, this flags the part *inside* a layer it
must avoid.


Layers
------

Two or more layers; every pairwise/collective overlap among all of them is checked at
once (the intersection of *all* listed layers must be empty).


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One violation per residual region — the overlapping area itself — owned by the tile
containing its centroid.


KLayout equivalent
------------------

``layer1.and(layer2, …)`` reported directly — a plain ``AND`` of all the listed layers.


Example
-------

.. code-block:: yaml

    - id: Cnt.j
      check: forbidden_overlap
      layers: [ContOnGatPoly, Activ]
      value: 0.0
