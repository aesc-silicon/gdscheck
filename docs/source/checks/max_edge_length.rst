.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_edge_length
===============

No segment of the layer may be longer than ``value``. The mirror of
:doc:`min_edge_length`.

Semantics
---------

As :doc:`min_edge_length`, compared against ``value`` rounded down to the grid.


Layers
------

One layer: an *edge* layer declared under ``edge_layers:`` — boundary segments — or a
polygon layer, in which case every segment of every merged region's boundary is read,
holes included.


Parameters
----------

None beyond ``layers`` and ``value`` (µm).


Violation markers
-----------------

One edge marker per offending segment, on the segment itself, so a marker lands on the
boundary the rule is about rather than on the region behind it.


KLayout equivalent
------------------

``edges.with_length(...)`` on the segments in question.


Example
-------

.. code-block:: yaml

    - id: FIN.l
      check: max_edge_length
      layers: [Fin]
      value: 50.0
