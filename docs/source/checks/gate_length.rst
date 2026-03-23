.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

gate_length
===========

Purely geometric (no connectivity needed). The minimum gate *length* — the poly width
measured only where the poly actually forms a device gate of a given type — as opposed
to :doc:`min_width`, which would measure GatPoly's width everywhere, and instead of the
channel *width* W (set by the perpendicular ``Activ`` edges), which must never be
mistaken for the length L.


Semantics
---------

Runs the same facing-wall width scan as :doc:`min_width` (see
:doc:`../architecture`'s tiled-merge section), but masked: a measured width is only kept
if its midpoint lies inside ``layers[1]``, a derived "gate region" layer such as
``GatPolyOverNsdActivNoTGO`` (GatPoly, intersected with N+ diffusion, outside a thick-gate-
oxide device). Measuring the poly directly — not the Activ-clipped channel — gives the
true gate length; different device flavours (N/P, thin/thick oxide) get their own rule
instance with their own mask layer and, typically, their own minimum length.


Layers
------

Positional, exactly two:

1. The poly layer whose width is measured (``GatPoly``).
2. A derived mask layer selecting which parts of the poly count as *this* device type's
   gate (e.g. ``GatPolyOverPsdActivTGO`` for a P-type thick-gate-oxide device). Only
   widths whose midpoint falls inside this mask are checked.


Parameters
----------

None beyond the standard ``value`` (the minimum length, µm).


Violation markers
------------------

One edge marker per side of every under-length gate span found (both walls of the
narrow poly, like :doc:`min_width`), but only where the span's midpoint lies inside the
mask layer.


KLayout equivalent
------------------

Corresponds to a ``width`` check on the poly layer, restricted (``and``/masked) to the
per-device-type gate-region derived layer — not a single built-in KLayout primitive, but
the same idiom KLayout DRC decks use to separate gate length from channel width.


Example
-------

Four device flavours, each with its own gate-region mask and minimum length:

.. code-block:: yaml

    - id: Gat.a1
      check: gate_length
      layers: [GatPoly, GatPolyOverNsdActivNoTGO]
      value: 0.13
    - id: Gat.a3
      check: gate_length
      layers: [GatPoly, GatPolyOverNsdActivTGO]
      value: 0.45
