.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

exact_width
===========

Every facing-wall width measured on the layer must equal ``value`` exactly (neither
narrower nor wider) — used for fixed-size features such as contacts and vias, where any
deviation is a manufacturing/process violation rather than a spacing concern.


Semantics
---------

Runs the same shared facing-edge width scan as :doc:`min_width`/:doc:`max_width` (see
:doc:`min_width` for the full description of the passes), with the predicate
``measured ≠ value``: any width that differs from ``value`` in either direction is a
violation. The comparison is exact — ``value`` is put on the grid once, to the nearest
DBU, and every span is compared to it as an integer — so a via drawn one nanometre off
is reported and one drawn to size is not. The mixed pass (a chamfer facing a straight
wall), the corner pass and the pinch and acute-corner readings are left out: none of
those is a span that could equal anything.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

``angle`` and ``length``
   As for :doc:`min_width`: ``angle: bent`` restricts the rule to 45° runs, ``length`` to
   wall pairs sharing more than that run.


Violation markers
------------------

One edge marker for **each of the two facing walls** of any width that differs from
``value`` (two markers per violation location), at the actual wall geometry. A pinch or
an acute corner is not a span and is not reported here; :doc:`min_width` reports those.


KLayout equivalent
------------------

``Region#width(value, value)`` — KLayout's width check with equal lower and upper bounds,
i.e. an exact-width constraint.


Example
-------

.. code-block:: yaml

    - id: Cnt.a
      check: exact_width
      layers: [ContSquare]
      value: 0.16
