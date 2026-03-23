.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

forbidden_unless_labeled
========================

A *candidate* device that lands in a *forbidden* region is an error, unless it belongs to
an isolation structure tagged by a text label. Every layer role is supplied positionally,
so the check is layer-name-agnostic and reusable across PDKs.

Its first (and so far only) user is IHP SG13G2's ``Ant.h``: an isolated n-type diffusion
diode (a Schottky/antenna-protection structure) is forbidden inside an NWell unless it
sits inside an ``isolbox``-labelled isolation cluster.


Semantics
---------

The rule derives a chain of intermediate regions from the ten positional layers::

    nactiv_con   = Activ − GatPoly − (pSD ∪ nSD.block)
    cand         = nactiv_con ∩ Recog.diode − Recog.esd
    nact_nwell   = nactiv_con ∩ (NWell ∪ PWell.block)
    schottky     = nBuLay ∩ Recog.diode ∩ NWell ∩ nSD.block
    isolbox_1    = (nBuLay      interacting Recog.diode interacting nact_nwell)
                 ∪ (Recog.diode interacting nBuLay      interacting nact_nwell)
    isolbox      = isolbox_1 not_interacting schottky, then interacting text ``<text>``
    error        = (cand not_interacting isolbox) ∩ NWell

A violation is reported for each connected ``error`` region: a candidate diffusion diode
that lies in an NWell and is not part of an isolation cluster carrying the exemption
label.

Every step is a lazy, tile-bounded :doc:`virtual layer <../virtual-ops>` — the plain
union/difference/intersection steps are local per tile; the ``interacting`` /
``not_interacting`` / ``with_text`` steps use the same whole-region stitching that
:doc:`min_region_density <min_region_density>`-style area checks rely on, so a dense,
chip-wide layer like ``Activ`` or ``GatPoly`` is never unioned globally. This matters in
practice: an earlier implementation that globally merged the (anchor-clipped) dense
layers worked on small blocks but ran out of memory on a full SoC top cell, where
antenna-diode markers are as numerous and widespread as the diffusion itself.


Layers
------

Ten positional roles, in this exact order:

.. list-table::
   :header-rows: 1

   * - Position
     - Role
     - IHP SG13G2 example
   * - 0
     - Base diffusion
     - ``Activ``
   * - 1
     - Gate poly (subtracted from the base)
     - ``GatPoly``
   * - 2
     - p-type source/drain implant (subtracted)
     - ``pSD``
   * - 3
     - n-type source/drain block (subtracted; also used in ``schottky``)
     - ``nSD.block``
   * - 4
     - Diode device-recognition marker
     - ``Recog.diode``
   * - 5
     - ESD device-recognition marker (exempted from the candidate set)
     - ``Recog.esd``
   * - 6
     - N-well
     - ``NWell``
   * - 7
     - P-well block
     - ``PWell.block``
   * - 8
     - Buried layer (isolation-cluster anchor)
     - ``nBuLay``
   * - 9
     - Text label layer (the exemption tag lives here)
     - ``TEXT``


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention). The exemption label comes
from the rule's ``text`` field, not ``params``.

``text``
   The label string that exempts a candidate's isolation cluster. Matched
   case-insensitively; a trailing ``*`` makes it a prefix match (the same convention as
   the connectivity engine's text lookups). Required — an absent or empty ``text`` means
   nothing is ever exempted.


Violation markers
------------------

One point marker per connected ``error`` region, at a representative point inside it
(the centroid of its largest tile-clipped piece). A region spanning several tiles is
still reported once.


KLayout equivalent
------------------

Mirrors the derivation in IHP's reference ``antenna.drc`` for the unlabelled-Schottky-diode
rule: the same chain of ``AND``/``NOT``/``interacting``/``interacting_with_text``
operators, evaluated here as tiled virtual layers rather than one-shot whole-chip
booleans.


Example
-------

.. code-block:: yaml

    - id: Ant.h
      check: forbidden_unless_labeled
      layers: [Activ, GatPoly, pSD, nSD.block, Recog.diode, Recog.esd, NWell, PWell.block, nBuLay, TEXT]
      value: 0.0
      text: isolbox
