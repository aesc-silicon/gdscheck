.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_edge_length
===============

Every segment of the layer must be at least ``value`` long.

Semantics
---------

On an edge layer this is the check that makes the channel-width family sayable (GF180
``DF.2a``): a transistor's W is the length of the Activ boundary running under the gate,
which is a property of that segment and of no region. Build the segment with
``edge_layers:`` (``edges``, then ``and`` against the gate outline), then bound it here.
On a polygon layer it is "no edge shorter than so much", a rule of its own in finer
processes; the segments read are the merged region's, the ones a mask sees.

A length is the root of a sum of two squares of integers and is compared squared against
``value`` rounded up to the grid, exactly. A segment seen from two tiles is reported once,
by the tile owning its midpoint.


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

    - id: DF.2a_LV
      check: min_edge_length
      layers: [channel_edges_3p3v]
      value: 0.22
