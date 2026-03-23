.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

IHP SG13CMOS5L
==============


Overview
--------

``ihp-sg13cmos5l`` is IHP's reduced CMOS-only process derived from SG13G2: no HBT
module, and a 4-metal (M1-M4 + TopMetal1) back-end instead of the full M1-M5 +
TopMetal1-2 stack. It's declared as a genuine derived PDK (``extends:
../ihp-sg13g2/pdk.yml``, see :doc:`../pdk-authoring`) — layer numbering is identical
between the two processes, so most decks are reused verbatim, and only the handful that
actually differ have their own local files under ``pdks/ihp-sg13cmos5l/decks/``.

Ships the same three suites as SG13G2 (``main``, ``precheck``, ``fill``), scoped to the
CMOS5L deck list.


Differences from SG13G2
--------------------------

* No HBT module and no MIM capacitors — the ``npn``, ``sdiod``, ``nmosi``, ``nbulay``,
  ``nbulayblock`` and ``mim`` decks don't exist for this process (their layers are
  forbidden, see below), so there is nothing for them to check.
* A 4-metal stack: no ``Metal5``, ``via4``, ``topvia2``, ``topmetal2`` decks.
* ``TopVia1`` lands directly on **Metal4** (there's no Metal5 underneath it as there is
  in SG13G2) — ``TV1.c`` encloses ``TopVia1`` in Metal4 at 0.10 µm instead of Metal5.
* Bond pads and the passivation opening are enclosed by **TopMetal1**, the process's top
  metal, instead of TopMetal2 — ``Pad.i`` uses a ``DfpadNoTopMetal1`` virtual layer;
  ``Padb.c``/``Padc.c`` enclose in TopMetal1. Values are unchanged from SG13G2.
* The antenna connect graph ends at TopVia1↔Metal4/TopMetal1 (one fewer metal level than
  SG13G2's chain) — the first five connect steps are identical, so net-prefix-derived
  parameters are unaffected.
* Chapter 8 ``DigiBnd`` splitting extends slightly further than SG13G2: **``Cnt.c``**
  gets an explicit digital variant (``Cnt.c.Digi``, 0.05 µm vs. the analog 0.07 µm) and
  **``NW.f1``** gets a ``.dig`` variant (0.24 µm vs. 0.62 µm analog) — both via the same
  ``interacting``/``not_interacting`` DigiBnd-split pattern SG13G2 already uses for
  ``NW.c1``/``NW.d1``/``NW.e1``.


Forbidden layers
-------------------

The ``forbidden`` deck extends SG13G2's own forbidden-layer list with everything the
reduced process doesn't have:

.. code-block:: yaml

   - id: forbidden
     check: forbidden
     layers: [BiWind, PEmWind, BasPoly, DeepCo, PEmPoly, EmPoly, LDMOS, PBiWind, NoDRC,
              Flash, ColWind,
              Metal5, Metal5.filler, Via4, TopMetal2, TopMetal2.filler, TopVia2,
              TRANS, nBuLay, MIM, Vmim]
     value: 0.0

Any shape at all on ``Metal5``/``Via4``/``TopMetal2``/``TopVia2`` (the removed metal
levels), ``TRANS``/``nBuLay`` (the HBT module) or ``MIM``/``Vmim`` (MIM capacitors) is a
violation — a SG13G2 design that happens to use any of these will fail this rule
outright when run under SG13CMOS5L (see :doc:`../faq`). That's expected: it means the
design isn't compatible with the restricted process, not a checker misconfiguration.


Digital (DigiBnd) rule relaxations
--------------------------------------

Inside a ``DigiBnd`` region (a digital-block boundary marker), several HV-device
(``ThickGateOx``) well-enclosure and well-spacing rules relax to their LV values — the
same mechanism SG13G2 already applies to ``NW.c1``/``NW.d1``/``NW.e1``, extended here to
``Cnt.c`` and ``NW.f1``:

.. list-table::
   :header-rows: 1

   * - Rule
     - Analog (outside DigiBnd)
     - Digital (``.dig``/``.Digi``, inside DigiBnd)
   * - ``Cnt.c``
     - 0.07 µm
     - 0.05 µm
   * - ``NW.c1``
     - 0.62 µm
     - 0.31 µm
   * - ``NW.d1``
     - 0.62 µm
     - 0.31 µm
   * - ``NW.e1``
     - 0.62 µm
     - 0.24 µm
   * - ``NW.f1``
     - 0.62 µm
     - 0.24 µm

Each split is implemented as a pair of ``interacting``/``not_interacting`` virtual
layers against ``DigiBnd`` (see :doc:`../virtual-ops`), so the strict and relaxed
variants run on disjoint geometry — a shape can only ever be measured by one of the two.


Deck and rule coverage
-------------------------

Decks reused, byte-for-byte identical to SG13G2 (same rule ids, values and layers):
``offgrid``, ``pin``, ``lbe``, ``activ``, ``tgo``, ``gatpoly``, ``extblock``,
``contbar``, ``salblock``, ``nsdblock``, ``psd``, ``resistor``, ``pwellblock``,
``metal1``–``metal4``, ``via1``–``via3``, ``passiv``, ``sealring``, ``slit``, ``lu``.

Decks with local, process-specific differences (see above):
``forbidden``, ``pad``, ``cont``, ``nwell``, ``topvia1``, ``topmetal1``, ``antenna``.

Not present (the underlying layers/devices don't exist in this process): ``nmosi``,
``npn``, ``sdiod``, ``nbulay``, ``nbulayblock``, ``metal5``, ``via4``, ``topvia2``,
``topmetal2``, ``mim``.
