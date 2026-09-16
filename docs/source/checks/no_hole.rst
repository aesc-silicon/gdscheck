.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

no_hole
=======

A region of the layer must not have a hole — a MIM plate, a bond pad, a fill shape with
a void in it.

Semantics
---------

Every hole of every merged region is a violation. Holes are read on the merged layer, so
a ring drawn as four bars is a region with a hole, and a shape drawn with a hole that a
second shape fills is not. A hole a tile line runs through is reported once, by the tile
owning its centroid.


Layers
------

One layer.


Parameters
----------

None beyond ``layers``; ``value`` is not read.


Violation markers
-----------------

One point marker per hole, at its centroid.


KLayout equivalent
------------------

``layer.holes``.


Example
-------

.. code-block:: yaml

    - id: MIM.h
      check: no_hole
      layers: [MIM]
      value: 0.0
