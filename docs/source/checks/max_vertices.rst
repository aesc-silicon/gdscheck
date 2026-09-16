.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_vertices
============

No polygon as drawn may have more vertices than ``value`` — the stream and mask
constraint every tape-out deck carries at 200, 600 or 8191.

Semantics
---------

Read on the drawn polygons, not the merge: the limit is on what is written to the
stream, and a merge joins shapes into regions that were never one polygon. A boundary's
closing point repeats its first and is not a vertex.


Layers
------

One layer, drawn.


Parameters
----------

None beyond ``layers`` and ``value`` (the largest vertex count allowed).


Violation markers
-----------------

One point marker per polygon over the limit, at its first vertex.


KLayout equivalent
------------------

None built in; a script over ``each_shape`` counting points.


Example
-------

.. code-block:: yaml

    - id: STREAM.v
      check: max_vertices
      layers: [Metal1]
      value: 600
