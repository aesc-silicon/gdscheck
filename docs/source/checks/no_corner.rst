.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

no_corner
=========

Forbidden corners: vertices where the boundary turns by a given angle, or by any angle in
a range. The companion to :doc:`no_angle`, and not a substitute for it.

Semantics
---------

The two ask different questions. :doc:`no_angle` looks at one edge and asks whether its
*direction* is allowed; this looks at a vertex and asks whether the *turn between two
edges* is. A right-angle bend is built from a 0° edge and a 90° edge, both perfectly
legal orientations, so no setting of ``no_angle`` can see it without also flagging every
straight orthogonal run on the die. GF180's ``PL.6`` is the rule that needs it: "90 degree
bends on the COMP are not allowed".

``value`` is the forbidden turn in degrees, matched on its absolute size so that a convex
and a concave bend of the same sharpness both count, within ``tolerance``. With ``min``
and ``max`` the rule forbids a range of turns instead and ``value`` is not read: "no
corner sharper than a right angle" is ``min: 90.005`` and ``max: 180``.

Corners are taken from the *merged* layer, so a bend that only exists because two drawn
shapes were butted together counts, and one that two shapes merge away does not. A right
angle is exact — the dot product of two integer edges is zero or it is not; any other
turn is a float angle against a float bound, which is what ``tolerance`` is for.


Layers
------

- ``layers[0]`` — the layer whose corners are read.
- ``layers[1]`` — optional. Only corners lying inside this layer count, sampled as a
  0.1 µm square around the vertex, so a bend within 0.1 µm of the layer's own edge is
  not flagged (GF180's reference seeds each corner with a dot grown by 0.1 µm).


Parameters
----------

``tolerance``
   Optional, degrees, default 1. How far a turn may differ from ``value`` and still count.

``min`` / ``max``
   Optional, degrees. Forbid every turn from ``min`` to ``max`` inclusive; either one
   left out is 0 or 180.

``outside`` (a layer param)
   Optional. Corners inside this layer are exempt.


Violation markers
-----------------

One point marker per forbidden corner, at the vertex.


KLayout equivalent
------------------

``corners(angle)`` (unioned with ``corners(-angle)``), or ``corners(min..max)``.


Example
-------

.. code-block:: yaml

    - id: PL.6
      check: no_corner
      layers: [poly2_drawn, comp_no_ymtp]
      value: 90.0
      layer_params:
        outside: ymtp_mk
