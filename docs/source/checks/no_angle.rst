.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

no_angle
========

Flags forbidden edge angles on a layer. By default, every non-orthogonal (not
horizontal/vertical) edge is forbidden; with the ``angle`` param, only edges at one
specific orientation are. Used for IHP's ``Gat.f``: a 45° gate over active area is not
allowed.


Semantics
---------

Run over the tiled merge cache. Typically pointed at an intersection layer (e.g.
``GatPolyOverActiv``) so only the part of the gate actually crossing the channel is
inspected, rather than the whole gate polygon. Each merged region's edges (rectilinear
and oblique alike) are classified by their angle, folded into ``[0°, 180°)``:

- With no ``angle`` param: any edge that isn't within ``tolerance`` degrees of 0° or 90°
  is flagged.
- With ``angle`` set: only edges within ``tolerance`` degrees of that orientation (or its
  180° complement) are flagged — e.g. to forbid *only* 45° edges while tolerating other
  non-orthogonal angles.


Layers
------

One layer — every edge of its merged geometry is checked.


Parameters
----------

``angle``
   Optional. The one forbidden orientation, in degrees. If omitted, every non-orthogonal
   edge is forbidden.

``tolerance``
   Optional, in degrees. How close an edge's angle must be to the forbidden orientation,
   or to a multiple of ``step``, to count as matching. It defaults to ``1.0`` with an
   ``angle`` param, where the slack widens the net - an edge that misses the forbidden
   orientation by half a degree is the orientation the rule is about - and to ``0`` in
   the ``step`` form, where it would hole it: GF180's ACUTE is "all shapes must be
   orthogonal or on a 45°", and geometry sits on a grid, so an edge is on the lattice
   exactly or it is not. A degree of slack there is also scale-dependent, since one grid
   step off over a short edge is a larger angle than over a long one.


Violation markers
------------------

One edge marker per forbidden edge, spanning its full length.


KLayout equivalent
------------------

``layer.edges.with_angle(0, 90, false)`` (non-orthogonal edges) or
``layer.edges.with_angle(angle)`` for a specific forbidden orientation.


Example
-------

.. code-block:: yaml

    - id: Gat.f
      check: no_angle
      layers: [GatPolyOverActiv]
      value: 0.0
