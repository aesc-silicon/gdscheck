.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_count
=========

Maximum number of connected regions of a layer on the whole chip — the emitters of one
bipolar flavour, say, which a foundry caps per chip. The mirror of :doc:`min_count`.


Semantics
---------

As :doc:`min_count`, with more than ``value`` regions reported.


Layers, parameters, markers
---------------------------

As :doc:`min_count`.


KLayout equivalent
------------------

``CHIP.interacting(layer, value + 1)``.


Example
-------

.. code-block:: yaml

    - id: npn13G2.bR
      check: max_count
      layers: [EmitG2]
      value: 4000
