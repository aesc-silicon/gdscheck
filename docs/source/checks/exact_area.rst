.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_area
==========

Each connected region of a layer — or each hole, container or the chip, as ``scope``
says — must have exactly ``value`` of area. The exact counterpart of :doc:`min_area` and
:doc:`max_area`; rarely a rule of its own, a fixed via being :doc:`exact_width`'s.


Semantics
---------

As :doc:`min_area`, with an area other than ``value``, rounded to the nearest half
DBU², reported.


Layers, parameters, markers
---------------------------

As :doc:`min_area`.


Example
-------

.. code-block:: yaml

    - id: CAP.a
      check: exact_area
      layers: [MIMunit]
      value: 25.0
