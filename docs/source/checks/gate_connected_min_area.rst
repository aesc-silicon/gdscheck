.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

gate_connected_min_area
=======================

Net-aware — needs electrical connectivity (the default; disabled by ``--no-connectivity``,
in which case this check is skipped with a message) and a PDK connect graph
(``connectivity:`` in ``pdk.yml``) that reaches from the gate layer to the marker layer(s).

Flags a marker-layer region that is (a) electrically connected to a gate and (b) smaller
than a minimum area. Its motivating use is the antenna-protection diode (``Ant.g``): a
``dantenna``/``dpantenna`` device tied to a gate must be at least 0.16 µm² to actually
bleed off enough charge to protect it — an undersized "protection" diode is really no
protection at all.


Semantics
---------

For each region of every marker layer:

1. Compute its area (µm²); skip it if it already meets ``value`` — only undersized regions
   are candidates.
2. Resolve its net through ``marker_net_layer`` (a diode sits on diffusion).
3. Flag it only if that net also carries at least one gate region, resolved through
   ``gate_net_layer`` (a gate sits on ``GatPoly``).

So a small marker region that isn't electrically tied to any gate is not an error — the
rule only cares about undersized structures that were *meant* to protect a gate.


Layers
------

Positional: ``[gate, marker layers…]`` — one gate layer, then one or more marker layers
(checked independently, sharing the same gate-net lookup).


Parameters
----------

``gate_net_layer`` / ``gate_net_layer_dt``
   Raw GDS layer/datatype a gate region's net is resolved through. Defaults to the gate
   layer itself.

``marker_net_layer`` / ``marker_net_layer_dt``
   Raw GDS layer/datatype a marker region's net is resolved through (e.g. ``1`` for
   ``Activ``, since a diode sits on diffusion). Defaults to ``(0, 0)`` — almost always
   worth setting explicitly.


Violation markers
------------------

One point marker per undersized, gate-connected marker region, at its centroid, reporting
its area and the required minimum.


KLayout equivalent
------------------

Equivalent to filtering a marker layer by ``area < value`` and then by
``interacting``/net-membership against the gate net — the net-membership part needs
KLayout's connectivity extraction (``antenna``/net DRC), the same engine
:doc:`antenna_ratio` uses.


Example
-------

.. code-block:: yaml

    - id: Ant.g
      check: gate_connected_min_area
      layers: [GatPolyOverActiv, AntDantenna, AntDpantenna]
      value: 0.16
      params:
        gate_net_layer: 5
        marker_net_layer: 1
