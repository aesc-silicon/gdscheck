.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_count
===========

Exactly ``value`` connected regions of a layer on the whole chip. The exact
counterpart of :doc:`min_count` and :doc:`max_count`.


Semantics
---------

As :doc:`min_count`, with any other number of regions reported.


Layers, parameters, markers
---------------------------

As :doc:`min_count`.


Example
-------

.. code-block:: yaml

    - id: SEAL.one
      check: exact_count
      layers: [EdgeSeal]
      value: 1
