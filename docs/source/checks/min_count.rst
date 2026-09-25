.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_count
=========

Minimum number of connected regions of a layer on the whole chip.


Semantics
---------

The regions of each layer are stitched across tile borders, so a shape a tile line cuts
counts once, and shapes that overlap or abut form one region. Fewer than ``value``
regions is a violation.


Layers
------

One or more. Each is counted on its own.


Parameters
----------

None.


Violation markers
-----------------

One point per layer out of bounds, at its first region, or at the origin when it has
none.


KLayout equivalent
------------------

``CHIP.not_interacting(layer, value, nil)``.


Example
-------

.. code-block:: yaml

    - id: SEAL.cnt
      check: min_count
      layers: [EdgeSeal]
      value: 1
