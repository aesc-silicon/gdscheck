.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

forbidden
=========

Every shape on any of the rule's layers is a violation. Used for layers the process
simply doesn't support in this flow (e.g. bipolar/LDMOS/flash device layers that have no
business appearing in a standard digital/analog design).


Semantics
---------

No booleans, no merging — each raw GDS boundary on any listed layer is flagged as-is.
This is the simplest possible check: "this layer must not be drawn at all."


Layers
------

One or more layers; any shape on any of them is forbidden.


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per raw boundary shape, at its centroid.


KLayout equivalent
------------------

Reporting each layer's shapes directly with no filter — the DRC-DSL equivalent of
``layer.output("id", "description")`` with no operation applied.


Example
-------

.. code-block:: yaml

    - id: forbidden
      check: forbidden
      layers: [BiWind, PEmWind, BasPoly, DeepCo, PEmPoly, EmPoly, LDMOS, PBiWind, NoDRC, Flash, ColWind]
      value: 0.0
