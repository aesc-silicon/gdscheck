.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_area
========

Minimum area of each connected region of a layer — or of each hole through one, of
everything of a second layer inside each region of the first, or of the layer on the
whole chip, as ``scope`` says. With ``net: connected``, only the regions on a net that
also carries another layer.


Semantics
---------

Regions are reconstructed from the shared merge by stitching the per-tile merge across
tile borders, so a layer of any density is measured whole in bounded memory. A region
(hole, container, chip) whose area is under ``value`` is reported once, at its marker.

The bound is put on the half grid an area lives on once — a minimum rounded up, a
maximum down, an exact value to the nearest half DBU² — and compared exactly. The tiling
cuts a shape at grid points on every axis-aligned and 45° wall, so the pieces' areas sum
to the region's exactly there; a wall at another angle can be off by a fraction of a DBU²
where a tile line crosses it.


Layers
------

One layer, or several measured alike — except under ``scope: contained``, which takes
two: ``layers[0]`` the container, ``layers[1]`` the contained layer.


Parameters
----------

``touching``
   Optional, with ``scope: region``. ``joined`` (the default): two shapes meeting at
   one point are one region, as KLayout merges them. ``separate``: they are two, each
   with its own area - a point joins no material, and IHP's area rules read it so
   (KLayout's ``min_coherence``).

``scope``
   Optional. What the area is summed over.

   ``region`` (the default)
      Each connected region of the layer, shapes that overlap or abut forming one
      region and measured together.

   ``hole``
      Each hole through a region — an empty area the layer surrounds entirely (IHP
      ``Act.e``, GF180 ``DF.10``: a tiny hole in active is a violation).

   ``contained``
      Everything of ``layers[1]`` inside each region of ``layers[0]``, summed per
      region: "these may share a plate, as long as the total on that one plate stays
      under the cap" (GF180 ``MIM.11``, where several MIM capacitors may share a bottom
      plate but their areas add up against ``MIM.8b``'s limit).

   ``chip``
      Everything of the layer on the chip, summed once (IHP ``MIM.gR``, the recommended
      total MIM area).

``net``
   Optional, with ``scope: region``. ``connected`` narrows the rule to regions on a net
   that also carries some region of the layer the ``connected`` layer param names — the
   antenna protection diode that must be big enough to protect the gate it is tied to
   (IHP ``Ant.g``). Nets are looked up on conductors of the connect graph, which a
   marker layer is not: ``net_of`` names the conductor under the measured layer (a diode
   sits on Activ), ``connected_net_of`` the one under the connected layer (a gate on
   GatPoly); each defaults to the layer itself. A rule with ``net`` is net-aware: it runs
   net extraction and is skipped under ``--no-connectivity``.


Violation markers
-----------------

One point marker per region, hole or container under the bound, at its marker; one per
layer under ``scope: chip``, at the first region's.


KLayout equivalent
------------------

``with_area(0, value)`` on the regions; ``holes.with_area(...)`` for the holes;
``area < value`` on the whole layer for the chip. The contained sum and the connected
net have no single operator.


Examples
--------

.. code-block:: yaml

    - id: M1.d
      check: min_area
      layers: [Metal1]
      value: 0.09

    - id: Act.e
      check: min_area
      layers: [Activ]
      value: 0.15
      params:
        scope: hole

    - id: Ant.g
      check: min_area
      layers: [AntDantenna, AntDpantenna]
      value: 0.16
      params:
        net: connected
      layer_params:
        connected: GatPolyOverActiv
        connected_net_of: GatPoly
        net_of: Activ
