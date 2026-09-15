.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_gate_length
===============

The same span as :doc:`min_gate_length`, bounded from above: the width of a region between
chosen walls must not exceed ``value``. GF180 caps an LDMOS channel at 20 µm and the
device's width at 50, and no region carries either on its own — a rectangle has two
widths, and only the choice of walls says which is meant.


Semantics, layers, parameters
-----------------------------

As :doc:`min_gate_length`, with ``>`` in place of ``<``. The LDMOS body is the gate poly over
the active with the drift marker taken out, so the walls it shares with the active's
boundary stand at the ends of the channel and the distance between them is the device's
width; the unshared ones stand across it.


Example
-------

.. code-block:: yaml

    - id: MDN.3b
      check: max_gate_length
      layers: [ldnmos_body, ncomp]
      value: 20.0
      params:
        walls: unshared
    - id: MDN.4b
      check: max_gate_length
      layers: [ldnmos_body, ncomp]
      value: 50.0
