.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

antenna_ratio
=============

Net-aware. The classic plasma-induced-gate-damage antenna check: the conductor area
electrically tied to a gate, divided by that gate's area, must stay under a limit — too
much exposed conductor collects charge during plasma processing and can punch through
the thin gate oxide before the device is finished and protected by a diode.


Semantics
---------

Mirrors IHP's reference ``antenna.drc``. The antenna ratio is accumulated **cumulatively**
up the interconnect stack: at each metal/via level, the net is "connectivity through all
layers up to and including that level" (see :doc:`../architecture`'s net-extraction
section), so the gate-area denominator grows as higher layers merge separate gates onto
one net, and the antenna-area numerator is that level's own conductor area on the net. The
ratio for a gate is the sum, across all levels, of *(this level's conductor area on the
net) / (this level's gate area on the net)* — not a single final-level snapshot. Because
the cumulative sum is monotonic as the stack is walked, flagging only the final
accumulated ratio is equivalent to KLayout flagging a violation at *any* intermediate
level.

A gate's net optionally carries a **protection diode** — a diffusion diode tied to the
same net, above a fixed 0.16 µm² floor (the antenna-protection-diode sizing threshold,
matching :doc:`gate_connected_min_area`'s ``Ant.g``). ``require_diode`` selects which
population a rule instance checks:

- ``require_diode: 0`` — nets *without* a diode, checked against the strict limit.
- ``require_diode: 1`` — nets *with* a diode, checked against a much more relaxed limit
  (a diode bleeds off charge, so a protected net tolerates far more antenna area).
- omitted — every net is checked, diode or not.

A fixed connectivity level (``level``) can be given instead of the per-antenna-layer
cumulative level — used for pre-metal checks (``Ant.a``/``Ant.c``) where the "antenna" is
itself a derived, non-metal layer (poly over field oxide, or the contact count) rather
than a step in the metal/via stack.


Layers
------

Positional: ``[gate, antenna conductors…, diode]``.

.. list-table::
   :header-rows: 1

   * - Position
     - Role
   * - 0
     - The gate layer (device channel poly, e.g. ``GatPolyOverActiv``)
   * - 1 .. antenna_layers
     - The antenna conductor(s) — one entry per interconnect level being accumulated
       (``Metal1``, ``Metal2``, …, or a single pre-metal layer)
   * - last (optional)
     - The protection-diode marker layer (e.g. ``AntDiode``); omit to check every net
       regardless of diode presence

``antenna_layers`` (see below) says how many trailing entries after the gate are antenna
conductors; anything left over is the diode layer.


Parameters
----------

``antenna_layers``
   How many layers after the gate are antenna conductors (default: all remaining layers,
   i.e. no diode layer).

``gate_net_layer`` / ``gate_net_layer_dt``
   Raw GDS layer/datatype (not a layer *name*) that a gate region's net is resolved
   through — a gate sits on the poly layer, e.g. ``5`` for ``GatPoly``. Defaults to the
   gate layer itself.

``antenna_net_layer`` / ``antenna_net_layer_dt``
   Raw GDS layer/datatype an antenna region's net is resolved through, when the antenna
   layer is itself a derived layer not present in the connect graph (e.g. poly-over-field
   resolved through ``GatPoly``). Defaults to the antenna layer itself — the common case,
   since a metal/via/contact layer *is* a connect-graph member.

``diode_net_layer`` / ``diode_net_layer_dt``
   Raw GDS layer/datatype the diode marker's net is resolved through (a diode sits on
   diffusion, e.g. ``1`` for ``Activ``). No diode-presence test is done if omitted.

``require_diode``
   ``0`` = only nets *without* a protection diode; ``1`` = only nets *with* one; omitted =
   every net. Diode presence is decided once, on the *full* net (not per level): at least
   0.16 µm² of diode-marker area electrically tied to the gate.

``level``
   Fix the connectivity level (a prefix index into the PDK's ordered connect-step list)
   instead of using each antenna layer's own step in the stack. Used when the "antenna" is
   a pre-metal / non-stack-level layer (e.g. contact count, or poly over field oxide).


Violation markers
------------------

One point marker per gate region whose cumulative ratio reaches ``value``, at the gate
region's centroid, reporting the accumulated ratio, the gate area, and whether a
protection diode was found on the net.


KLayout equivalent
------------------

The area-ratio accumulation KLayout's ``antenna`` DRC function performs per net per
level; here the levels are walked explicitly (one connectivity partition per antenna
layer) rather than through a single built-in operator.


Example
-------

Pre-metal ratio (poly over field oxide), fixed at the contact connectivity level:

.. code-block:: yaml

    - id: Ant.a
      check: antenna_ratio
      layers: [GatPolyOverActiv, AntPolyField]
      value: 200.0
      params:
        antenna_layers: 1
        gate_net_layer: 5
        antenna_net_layer: 5
        level: 2

Cumulative metal-stack ratio, strict limit for nets with no protection diode:

.. code-block:: yaml

    - id: Ant.b
      check: antenna_ratio
      layers: [GatPolyOverActiv, Metal1, Metal2, Metal3, Metal4, Metal5, TopMetal1, TopMetal2, AntDiode]
      value: 200.0
      params:
        antenna_layers: 7
        gate_net_layer: 5
        diode_net_layer: 1
        require_diode: 0

Same conductors, relaxed limit for nets that *do* carry a protection diode:

.. code-block:: yaml

    - id: Ant.e
      check: antenna_ratio
      layers: [GatPolyOverActiv, Metal1, Metal2, Metal3, Metal4, Metal5, TopMetal1, TopMetal2, AntDiode]
      value: 20000.0
      params:
        antenna_layers: 7
        gate_net_layer: 5
        diode_net_layer: 1
        require_diode: 1
