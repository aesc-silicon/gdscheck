.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_edge_length
=================

Every segment of the layer must be exactly ``value`` long. The exact counterpart of
:doc:`min_edge_length` and :doc:`max_edge_length`.

Semantics
---------

As :doc:`min_edge_length`, compared for equality against ``value`` rounded to the grid.


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

    - id: CH.w
      check: exact_edge_length
      layers: [channel_edges]
      value: 0.5
