.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_width
=========

Every place metal is wider than ``value`` (measured wall-to-wall, perpendicular to the
metal's run) is a violation. Commonly used to cap filler/dummy-fill tile size, e.g. IHP's
``TM2Fil.a1`` (TopMetal2 filler maximum width).


Semantics
---------

Runs the same shared facing-edge width scan as :doc:`min_width` (see that page for the
full rectilinear + oblique pass description), just with the opposite predicate: a
measured width greater than ``value`` is the violation instead of one narrower than it.


Layers
------

A single layer, ``layers[0]``.


Parameters
----------

None — only ``value`` (µm).


Violation markers
------------------

One edge marker for **each of the two facing walls** of any width above ``value`` (two
markers per violation location), at the actual wall geometry.


KLayout equivalent
------------------

``Region#width(value)`` filtered to the "wider than" side — KLayout's own facing-edge
width check.


Example
-------

.. code-block:: yaml

    - id: TM2Fil.a1
      check: max_width
      layers: [TopMetal2.filler]
      value: 10.00
