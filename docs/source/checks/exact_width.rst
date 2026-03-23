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
:doc:`min_width` for the full rectilinear + oblique pass description), with the predicate
``|measured − value| > 0`` (within a half-grid tolerance): any width that differs from
``value`` in either direction is a violation.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

None — only ``value`` (µm).


Violation markers
------------------

One edge marker for **each of the two facing walls** of any width that differs from
``value`` (two markers per violation location), at the actual wall geometry.


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
