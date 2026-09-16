.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_space
=========

Maximum proximity distance: everything of a target layer must lie within ``value`` of a
reference layer. Used for latch-up rules such as "every source/drain diffusion must be
within 20 µm of a well tie", for well-tap rules such as "a diffusion must have a tap
within 15 µm", and for guard-ring rules such as "every edge of the well is within 15 µm
of the ring". ``scope`` says what has to be in reach: every part, every polygon or every
edge.


Semantics
---------

This is a coverage/reach check, not a spacing check: it doesn't flag two regions that
are *too close*, it flags what of ``layers[0]`` is *too far* from any region of
``layers[1]``. It reads backwards from the rest of the engine — a spacing rule fails on
the closest pair and is satisfied by there being nothing nearby, while this one fails on
the absence of a neighbour, so an empty reference layer makes everything a violation
rather than nothing.

``scope: part`` (the default)
   Every part of the target must lie within ``value`` of the reference; any part that
   does not is flagged, wherever on a shape it is. The reference is grown by ``value``
   with a square structuring element (what KLayout's ``sized`` gives rectilinear
   geometry) and subtracted from the target; what is left is stitched across tiles and
   reported once per gap.

``scope: polygon``
   Every polygon of the target must have *some* part within ``value`` of the reference;
   a polygon that has none is flagged as a whole. The same grown reference, tested per
   region: a wide active block passes once its tap is in reach of any of it, where
   ``part`` would flag the far side of the block. A polygon exactly ``value`` away is in
   reach, as KLayout's ``interacting`` counts a touch; one a DBU further is not.

``scope: edge``
   Every edge of ``layers[0]``, an edge layer, must have ``layers[1]`` within ``value``
   at some point along it; an edge with none is flagged. The partner is gathered by tile
   lookup over the edge's own box grown by the limit, since an edge is not clipped to a
   tile.

Both region readings run on the tiled merge as core pieces — neither layer pays the
value as a halo, and a dense reference such as the contacts on ties is never globally
unioned.


Layers
------

- ``layers[0]`` — the target that must stay close to something (an edge layer under
  ``scope: edge``).
- ``layers[1]`` — the reference geometry it must reach.


Parameters
----------

``scope``
   Optional. ``part`` (the default), ``polygon`` or ``edge``, as above.

``within``
   Optional layer param (under ``layer_params``), with ``scope: polygon``. Confines the
   reach to that layer: the reference is grown in half-micron steps and cut back to the
   layer after each, so the reach goes round a slot in the layer and never crosses the
   gap to another region of it. This is how the GF180 well-tap rules read — the tap is
   grown inside the well, in steps under the well's own spacing — so a diffusion on the
   far leg of a U-shaped well has no tap in reach across the slot however near the tap
   lies, and a tap in a neighbouring well is no tap for this one.


Violation markers
-----------------

Under ``part``, one point marker per gap, in the part of the target beyond reach. Under
``polygon``, one point marker per unreached polygon. Under ``edge``, the edge itself.


KLayout equivalent
------------------

Not a single built-in operator. ``part`` is ``target.not(reference.sized(value))``;
``polygon`` is ``target.not_interacting(reference.sized(value))``, the construction the
GF180 deck uses for its well-tap rules; ``edge`` is the complement of a ``separation``
in its edge form — measure where the separation is within the limit and keep the edges
that do not touch it.


Examples
--------

.. code-block:: yaml

    - id: LU.a
      check: max_space
      layers: [PsdActivInNWell, NActivInNWell]
      value: 20.00

    - id: DF.13_MV
      check: max_space
      layers: [pactive_56v, ntap]
      value: 15.0
      params:
        scope: polygon
      layer_params:
        within: nwell

    - id: MDP.4b
      check: max_space
      layers: [mdp4b_edges, mdp4b_pcomp]
      value: 15.0
      params:
        scope: edge
