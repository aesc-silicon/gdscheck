.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

antenna_ratio
=============

Net-aware. The classic plasma-induced gate-damage check: the conductor area electrically
tied to a gate, divided by that gate's area, must stay under a limit — too much exposed
conductor collects charge during plasma processing and can punch through the thin gate
oxide before the device is finished and protected by a diode.


Semantics
---------

Mirrors IHP's reference ``antenna.drc``. The ratio is accumulated **cumulatively** up the
interconnect stack: at each metal or via level the net is the connectivity through all
connect steps up to that level (see :doc:`../architecture`'s net-extraction section), so
the gate-area denominator grows as higher layers merge separate gates onto one net, and
the numerator is that level's own conductor area on the net. A gate's ratio is the sum
over the levels of *(this level's conductor area on the net) / (this level's gate area
on the net)*. The sum is monotonic as the stack is walked, so flagging the final sum is
what KLayout flagging a violation at any level comes to. A ratio over ``value`` is a
violation; one at the value meets the maximum, as every rule of a manual is met at its
value (IHP's own deck reads its cumulative rules as ``>=`` and its per-level ones as
``>``).

A gate's net optionally carries a **protection diode** — a diffusion diode tied to the
same net, of at least the size the diode rule asks for (``diode_area``, 0.16 µm² by
default, Ant.g's floor; a diode that meets that rule is a protection diode). ``diode: without`` checks the nets
without one against the strict limit, ``diode: with`` the nets with one against the
relaxed limit; without the param every net is checked.

GF180's model differs in two ways, both off unless asked for: ``metric: sidewall``
measures a metal by its perimeter times ``thickness`` rather than its plan area, and
``diode_factor`` credits a diode by adding that many times its area to the denominator
instead of switching limits.


Layers
------

The conductors, one per level being accumulated: ``[Metal1, Metal2, …]`` for the metal
stack, ``[Via1, Via2, …]`` for the vias, or a single pre-metal layer such as poly over
field or the contacts.


Parameters
----------

``diode``
   Optional. ``with`` or ``without``: which nets the rule is about, by whether a
   protection diode is tied to them. Decided once, on the full net.

``metric``
   Optional. ``area`` (the default) or ``sidewall``, perimeter times ``thickness``.

``thickness``
   With ``metric: sidewall``: the metal's thickness in µm.

``diode_factor``
   Optional. Add this many times the diode area on the net to the gate area.

``diode_area``
   Optional, µm². The least diode area that protects a net; 0.16 by default.

Layer params:

``gate``
   Required. The gate layer (the channel poly, e.g. ``GatPolyOverActiv``).

``gate_net_of``
   The conductor a gate's net is looked up on — a gate sits on poly, which the channel
   marker is not. Defaults to ``gate``.

``diode_1``, ``diode_2``, ``diode_3``
   The protection-diode marker layers, and ``diode_1_net_of``, ``diode_2_net_of``,
   ``diode_3_net_of`` the conductor each is looked up on (a diode sits on diffusion).
   GF180 names an n-diode, a p-diode and the well.

``antenna_net_of``
   The conductor a conductor's own net is looked up on, when the conductor is a derived
   layer that is in no connect step (poly over field, looked up on poly). Defaults to the
   conductor itself, which a metal, via or contact is.

``level`` / ``level_before``
   Read every conductor at one level instead of each at its own step: through the
   first connect step of the ``level`` layer, or short of that of the ``level_before``
   layer. IHP's Ant.a and Ant.c read the pre-metal net as ``level: Activ``; GF180 reads a
   via3 antenna before metal4 connects as ``level_before: metal4_drawn``, and the poly
   antenna alone as ``level_before: poly2_drawn``. Naming the layer, not the step's
   number, means a step added to the connect list moves no rule.


Violation markers
-----------------

One point marker per gate region whose cumulative ratio reaches ``value``, at the gate
region's marker, reporting the ratio, the gate area and whether a diode was found.


KLayout equivalent
------------------

KLayout's ``antenna_check``, one call per level; here the levels are walked explicitly,
one connectivity partition per conductor.


Examples
--------

.. code-block:: yaml

    - id: Ant.a
      check: antenna_ratio
      layers: [AntPolyField]
      value: 200.0
      layer_params:
        gate: GatPolyOverActiv
        gate_net_of: GatPoly
        antenna_net_of: GatPoly
        level: Activ

    - id: Ant.b
      check: antenna_ratio
      layers: [Metal1, Metal2, Metal3, Metal4, Metal5, TopMetal1, TopMetal2]
      value: 200.0
      params:
        diode: without
      layer_params:
        gate: GatPolyOverActiv
        gate_net_of: GatPoly
        diode_1: AntDiode
        diode_1_net_of: Activ

    - id: ANT.16_i_ANT.2
      check: antenna_ratio
      layers: [metal1_drawn]
      value: 400.0
      params:
        metric: sidewall
        thickness: 0.54
        diode_factor: 2
      layer_params:
        gate: nom_gate
        gate_net_of: poly2_drawn
        diode_1: n_diode
        diode_1_net_of: ncomp_con
        diode_2: p_diode
        diode_2_net_of: pcomp_con
        diode_3: nwell
        level: metal1_drawn
