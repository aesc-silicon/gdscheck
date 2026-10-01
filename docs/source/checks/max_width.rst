.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_width
=========

Every place a shape is wider than ``value`` in every direction - where a ``value`` ×
``value`` square fits inside it - is a violation. A wire is as wide as it is across,
however long it runs: a 1 µm wire, L, T or cross of any length is 1 µm wide. Commonly
used to cap pad and filler sizes, e.g. IHP's ``Pad.a1`` and ``AFil.a``.


Semantics
---------

By default (``span: narrowest``) the layer is opened by half the value, KLayout's
``sized(-v/2).sized(v/2)``: what survives is too wide, and each surviving region is one
violation. The opening is taken per tile, where a copy is exact to the halo, and the
pieces are stitched, so a wide shape across tiles is one report whatever the tile.

With ``span: any`` the rule runs the shared facing-edge width scan of :doc:`min_width`
instead, with the opposite predicate: every pair of facing walls further apart than
``value`` is a violation. A wire's two ends face each other across its length, so this
reading reports every wire longer than the value; it is for shapes whose every span is
bounded, such as fixed-size cuts. ``value`` is put on the grid once, rounded down. The
mixed pass, the corner pass and the pinch and acute-corner readings are left out: a
chamfer facing a straight wall has no single width to be too large, and a width of zero
is under any maximum.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

``angle`` and ``length``
   As for :doc:`min_width`: ``angle: bent`` restricts the rule to 45° runs, ``length`` to
   wall pairs sharing more than that run.

``span``
   Optional. ``narrowest`` (the default) reads a shape's width as its narrowest
   dimension, as above. ``any`` reports every wall-to-wall span over the value, so a
   1 × 300 µm stripe is 300 wide. A maximum on the bounding box's long side is
   :doc:`max_length` (IHP's metal-filler and LBE maxima); one on a segment's length,
   such as IHP's ``Slt.b``, is :doc:`max_edge_length`.


Violation markers
------------------

With ``span: narrowest``, one point marker per region that is too wide. With
``span: any``, one edge marker for **each of the two facing walls** of any width above
``value``, at the actual wall geometry.


KLayout equivalent
------------------

``layer.sized(-value/2).sized(value/2)`` - the opening. ``span: any`` has no single
KLayout counterpart; it is the facing-edge scan of ``Region#width`` with the bound
reversed.


Example
-------

.. code-block:: yaml

    - id: Pad.a1
      check: max_width
      layers: [PassivDfpad]
      value: 150.00
