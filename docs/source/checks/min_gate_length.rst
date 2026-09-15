.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_gate_length
===============

Purely geometric (no connectivity needed). The minimum width of a region measured between
*chosen* walls — :doc:`max_gate_length` and :doc:`exact_gate_length` bound the same span
the other ways: the gate poly's length where it forms a device gate — as opposed to
:doc:`min_width`, which would measure the poly's width everywhere — and never the channel
*width* W, which the active's edges set and which are no walls of the poly.


Semantics
---------

A gate has two kinds of wall. The ones the poly brought with it stand across the channel,
and the distance between them is the gate's length; the ones the active cut stand at the
ends, and the distance between them is the transistor's width. Both are widths of one
region, and which walls take part is the whole difference.

So the rule names the region (``layers[0]``) and a reference (``layers[1]``), and runs the
facing-wall width scan of :doc:`min_width` over the region, cutting every pair it finds
to the stretch along which **both** walls lie on the reference's boundary — or, with
``walls: unshared``, along which neither does. A pair with no such stretch is dropped.
Collinearity is exact on the grid.

For a poly stripe and a channel mask such as ``GatPolyOverNsdActivNoTGO`` (poly ∩ active
∩ N+, outside thick oxide), the stripe's two long walls lie on the mask's boundary exactly
where they cross the active, and every gate along the stripe is found from the tile it
falls in — however long the stripe. For an LDMOS body against its active, the walls
*unshared* with the active are the ones across the channel.


Layers
------

Positional, exactly two:

1. The region whose width is measured (``GatPoly``, ``tgate``, an LDMOS body).
2. The reference whose boundary chooses the walls: a channel mask, the active, ``comp``.


Parameters
----------

``walls``
   ``shared`` (default) keeps the stretches on the reference boundary; ``unshared`` the
   ones off it.

``angle``
   ``bent`` measures only between 45° walls — GF180's PL.7, the width across a gate that
   bends.

``layer_params: outside``
   A region the kept stretches must lie outside of — GF180's NAT.4 reads the native
   gate's length off the poly walls that are not under the well.

``length``
   As for :doc:`min_width`, read on the kept stretch: the gate is measured only where the
   walls it shares with the reference run longer than this — a length rule that binds
   only channels wider than so much.


Violation markers
------------------

One edge marker per wall of every under-length span, cut to the stretch the filter kept.


KLayout equivalent
------------------

``poly.edges.and(gate.edges).width(value)`` — and, for the other options,
``.not(region)`` on the edges, ``edges.inside_part(comp)`` and ``with_angle`` — read on
the region rather than on an edge collection, so the material between the walls is known
rather than inferred.


Example
-------

.. code-block:: yaml

    - id: Gat.a1
      check: min_gate_length
      layers: [GatPoly, GatPolyOverNsdActivNoTGO]
      value: 0.13
    - id: PL.2_LV
      check: min_gate_length
      layers: [tgate_pl2_lv, comp]
      value: 0.28
      params:
        walls: unshared
    - id: NAT.4
      check: min_gate_length
      layers: [poly_nat_lv, ngate]
      value: 1.8
      layer_params:
        outside: nwell
