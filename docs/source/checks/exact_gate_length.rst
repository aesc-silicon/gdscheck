.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_gate_length
=================

The same span as :doc:`min_gate_length`, required to equal ``value``: the width of a
region between chosen walls, for a device whose gate is drawn at one fixed length.


Semantics, layers, parameters
-----------------------------

As :doc:`min_gate_length`, with ``≠`` in place of ``<``. Like :doc:`exact_width`, a
pinch or an acute tip is not reported — a width of zero is not a span that could equal
anything.


Example
-------

.. code-block:: yaml

    - id: X.1
      check: exact_gate_length
      layers: [gate_poly, channel]
      value: 0.18
