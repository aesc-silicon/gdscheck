.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

nonempty
========

Every connected region of the layer is a violation — the layer itself *is* the error.
Used for marker rules whose condition is "this geometry must not exist," typically a
derived virtual layer whose whole purpose is to compute the forbidden set.


Semantics
---------

Regions are read from :doc:`../virtual-ops`'s shared, tile-stitched region cache
(``MergedCache::regions``), so a region spanning several tiles is still reported once.
There's no boolean step here — the check trusts that the layer it's given (often a
multi-step virtual layer) already *is* exactly the forbidden set, e.g. antenna ``Ant.i``'s
``AntIError = pactiv_con ∩ Recog.diode − Recog.esd − (NWell ∪ PWell.block)``, the set of
p-diodes sitting outside any well.


Layers
------

One layer — every connected region on it is flagged.


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per connected region, at its marker (a representative point from its
largest tile-clipped piece).


KLayout equivalent
------------------

Reporting a derived layer's regions directly with no further filter — the DRC-DSL
equivalent of ``layer.output("id", "description")`` on an already-fully-derived layer.


Example
-------

.. code-block:: yaml

    - id: npnG2.b
      check: nonempty
      layers: [NpnTieNoTrans]
      value: 0.0
