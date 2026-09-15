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
full description of the passes), just with the opposite predicate: a measured width
greater than ``value`` is the violation instead of one narrower than it. ``value`` is put
on the grid once, rounded down, and every span is compared to it as an integer. The
mixed pass, the corner pass and the pinch and acute-corner readings are left out: a
chamfer facing a straight wall has no single width to be too large, and a width of zero
is under any maximum.

The far wall of a wide span has to be in the tile's halo to be found, and the halo of a
layer is the largest ``value`` of any distance rule on it (see :doc:`../architecture`),
so a maximum finds every span up to its own value and reports the ones over it that its
halo still reaches.


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
