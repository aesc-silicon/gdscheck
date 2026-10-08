.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

IHP SG13G2
==========


Overview
--------

The bundled ``ihp-sg13g2`` PDK covers IHP's open-source SG13G2 SiGe BiCMOS process:
the full front-end device set (isolated NMOS, bipolar npn HBT, Schottky diodes,
resistors), the M1-M5/TopMetal1-2 back-end stack with vias, latch-up, metal slotting,
seal-ring, bond-pad and the §7.1 antenna rule family. Ships seven suites:

.. list-table::
   :header-rows: 1

   * - Suite
     - Coverage
   * - ``main``
     - Every per-layer deck.
   * - ``core``
     - Geometric DRC only: ``main`` minus the antenna deck and every density/fill
       rule. Use on blocks that aren't dummy-filled yet so min-density checks don't
       false-fail on the missing fill.
   * - ``precheck``
     - IHP's published open-source precheck subset.
   * - ``density``
     - Dummy-fill density checks only.
   * - ``antenna``
     - Antenna checks only (§7.1, ``Ant.a``–``Ant.i``). Needs net extraction.
   * - ``recommended``
     - The recommended (``*R``) rules only — see *Recommended rules* below. Not part of
       ``main``.
   * - ``sram``
     - The rules ``main`` skips under the SRAM marker, at the values of IHP's own bit
       cells — see *SRAM rules* below. Not part of ``main``.

Cross-checked throughout development against IHP's own KLayout reference decks (the
per-topic ``.lydrc`` scripts and the combined "maximal" deck) run in a container, and
against the process rule-deck PDF directly where the KLayout source and the PDF text
disagree (see *Documented divergences* below) — the rule text is treated as the
ultimate authority, not whichever KLayout idiom happened to implement it.


Deck and rule coverage
-------------------------

.. list-table::
   :header-rows: 1
   :widths: 20 80

   * - Deck
     - Coverage
   * - ``offgrid``
     - Off-grid geometry (5 nm grid).
   * - ``forbidden``
     - Forbidden layers.
   * - ``pin``
     - Pins and labels.
   * - ``lbe``
     - LBE layer.
   * - ``pad``
     - Bond pads.
   * - ``activ``
     - Active area.
   * - ``tgo``
     - Thick gate oxide (HV devices).
   * - ``gatpoly``
     - Gate poly.
   * - ``extblock``
     - Extension-implant block (EXTBlock).
   * - ``cont`` / ``contbar``
     - Contacts / contact bars.
   * - ``salblock``
     - Salicide block.
   * - ``nsdblock`` / ``psd``
     - n+/p+ source-drain implant.
   * - ``resistor``
     - Poly resistors — Rsil, Rppd, Rhigh, all complete.
   * - ``nmosi``
     - Isolated NMOS (nBuLay-isolated PWell) — complete.
   * - ``npn``
     - Bipolar npn HBT (npnG2, npn13G2/L/V) — complete.
   * - ``sdiod``
     - Schottky diode — complete (all 5 rules).
   * - ``nwell`` / ``pwellblock``
     - N-well / P-well block, including the chapter 8 ``DigiBnd`` HV/LV split.
   * - ``nbulay`` / ``nbulayblock``
     - N-buried layer.
   * - ``metal1``–``metal5``, ``via1``–``via4``, ``topvia1``, ``topmetal1``,
       ``topvia2``, ``topmetal2``
     - Full metal/via stack: width, space, notch, density (plain and windowed), fill.
   * - ``passiv``
     - Passivation.
   * - ``sealring``
     - Seal ring.
   * - ``slit``
     - Metal slotting.
   * - ``lu``
     - Latch-up.
   * - ``antenna``
     - Antenna, §7.1 (``Ant.a``–``Ant.i``) — see below.
   * - ``mim``
     - MIM capacitor.

All chapter 5–8 mandatory *geometric* rules are implemented. Run ``gdscheck show-deck
--process ihp-sg13g2 --deck <name>`` for the exact, current rule list of any deck — this
page tracks coverage at the topic level, not a rule-by-rule inventory that would go stale
the moment a deck changes.


Deliberately skipped rules
----------------------------

* ``npnG2.a``/``npnG2.f`` are definitions, not independently checkable rules.
* ``Pad.eR``/``Pad.fR`` (recommended pad exit width and length) — see *Recommended
  rules*.
* ``Padb.e``/``Padc.e`` (bond-pad pitch) are not separate rules: pitch is exactly opening
  size plus spacing (verified numerically against the PDK's own pad table), and the
  process documentation states pitch is "not checked during DRC."
* ``Pad.m`` (SBumpPad/CuPillarPad exclusivity) has no rule-deck number at all — it's a
  KLayout-tooling-only check, not implemented.


Recommended rules
-----------------

The manual's recommended rules are advisory: a layout may break them and still be
manufactured. They live in their own decks under ``decks/recommended/`` and run only
through the ``recommended`` suite (or ``--deck``), so they never mix with ``main``'s
violations.

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Deck
     - Rules
   * - ``pad_recommended``
     - ``Pad.aR``, ``Pad.bR``, ``Pad.dR``, ``Pad.d1R``, ``Pad.gR``, ``Pad.jR``,
       ``Pad.kR``.
   * - ``topmetal2_recommended``
     - ``TM2.bR``.
   * - ``mim_recommended``
     - ``MIM.gR``.
   * - ``npn_recommended``
     - ``npn13G2.bR``, ``npn13G2L.cR``, ``npn13G2V.cR`` (:doc:`../checks/max_count`).

Not implemented: ``Pad.eR``/``Pad.fR``, the width and length of a metal where it leaves
the pad (KLayout checks ``Pad.fR`` alone, by extending the exit edges; gdscheck has no
edge-to-region extension yet).

``Pad.dR``/``Pad.d1R`` grow the pad opening with round corners and look for the seal's
Activ or the chip's Activ in reach, rather than measuring a space: a 25 µm space halo on
Activ does not fit in memory on a full chip. ``Pad.gR`` follows the manual, TopMetal1
within dfpad, where KLayout encloses TopVia2 in Metal5. ``npn13G2V.cR`` counts emitters
drawn on EmWiHV (as IHP's pcell draws them) as well as EmWind.


SRAM rules
----------

Under the SRAM marker (``SRAM``, 25/0) IHP's KLayout deck, and ``main`` with it, does
not check a group of front-end rules: the layout rules leave their SRAM values "TBD".
The ``sram`` deck checks them again under the marker, at the smallest value IHP's own
SRAM bit cells (``sg13g2_sram``, 1P and 2P) use. A cell drawn to it is no tighter than
one IHP ships, and all 28 macros of ``sg13g2_sram`` pass it. Run it with ``--deck sram``
next to a ``core`` or ``main`` run on layouts that draw the marker.

.. list-table::
   :header-rows: 1
   :widths: 25 20 55

   * - Rule
     - Value
     - Note
   * - ``SRAM.Gat.c``
     - 0.13
     - GatPoly endcap over Activ (``Gat.c`` 0.18).
   * - ``SRAM.NW.c``
     - 0.27
     - NWell enclosure of P+Activ (``NW.c`` 0.31).
   * - ``SRAM.NW.d``
     - 0.27
     - NWell space to N+Activ (``NW.d`` 0.31).
   * - ``SRAM.Cnt.c``
     - 0.006
     - Activ enclosure of Cont, KLayout's ``Cnt.c.SRAM`` (``Cnt.c`` 0.07).
   * - ``SRAM.Cnt.g1``/``g2``, ``SRAM.pSD.e``/``g``/``i``/``j``, ``SRAM.LU.c``/``c1``,
       ``SRAM.npnG2.d1``
     - public
     - IHP's bit cells meet the public value, which stays.
   * - ``SRAM.TGO``
     - —
     - No ThickGateOx under the marker: the thick-oxide rules (``NW.d1``, ``pSD.i1``,
       ``pSD.j1``) have no SRAM values.

The values follow from IHP's shipped cells, not from a published rule: they show what
IHP qualifies, not the process limit.


Documented divergences from the KLayout deck
------------------------------------------------

A few rules are implemented against the rule-deck PDF text rather than the shipped
KLayout script, where the two disagree:

* **npn13G2.a/L.a/V.a** (minimum emitter dimension) — KLayout's ``ext_with_length``
  helper has an off-by-one-µm bug in its ``>`` branch (it adds 1 *database unit* worth
  of intent but the value is already in µm), leaving the shipped min-side rules with an
  empty, never-satisfiable range. gdscheck follows the PDF's stated limits exactly. The
  max-side rules are unaffected in practice (they fire correctly, just 1 µm later than
  the PDF says).
* **nmosi.g** — a SalBlock exactly flush (zero overlap) with an nSD:block region over a
  PWell tap fires here; the shipped KLayout script is silent on this exact case (a
  degenerate zero-area marker vanishes under its ``AND`` formulation). Real bad layouts
  aren't drawn perfectly flush, so this mostly matters for synthetic edge cases.
* **pSD.f** — an L-shaped diffusion tab hugging (but never extending 0.30 µm past) a P+
  implant boundary fires here; KLayout's "bad band" formulation only covers the region
  directly in front of the abutment edge, so a tab reaching the boundary sideways escapes
  its check.
* **Rhi.b** — KLayout's device recognition for this rule (``ext_covering``, strict
  containment) is empty for essentially every realistic resistor (one whose poly reaches
  its own contacts, extending past the implant stack) — a hole in the shipped check on
  real layouts. gdscheck's recognition uses touching-containment instead, following the
  PDF text that nSD drawing is only permitted within Rhigh resistors.
* **``covering``'s semantics** are deliberately loose (touching), not KLayout's strict
  containment (``self.covering(other.inside(self))``) — safe wherever containment is
  structurally guaranteed (most of the resistor-recognition chain), and the mechanism
  behind the Rhi.b divergence above. See :doc:`../virtual-ops`.
* **NW.b / NW.b1** (well spacing at the same / at different potential) are not in the
  shipped KLayout script at all — it omits both potential-dependent rules rather than
  approximate them. gdscheck implements both: NW.b as a plain geometric
  :doc:`../checks/min_space` at 0.62 µm, and NW.b1 as the same check at 1.80 µm with
  ``net: different``, gated on extracted nets. The
  ``NActivInNWell`` connect step ties a well to the tap sitting in it (and from there
  through Cont to Metal1), so wells strapped together no longer read as different-net.
  It has to be N+Activ rather than plain ``Activ``: P+Activ in an NWell is a PMOS
  source/drain, which must not tie the well to the diffusion net. NW.b1 is read on the
  drawn NWell, not on wells merged below NW.b's 0.62: a merge of wells on two nets
  shorts them, and that is the case the rule exists for - FMD_QNC_psoc-soc abuts two
  SRAM macros whose wells, on two supplies, are 0.40 apart. Gaps inside the SRAM
  marker are not read (``gap_outside``), as the pSD and well rules are not read in the
  bit cells. Note this makes NW.b1 a net-aware rule, so the
  ``core`` suite now runs net extraction; ``--no-connectivity`` skips it (and NW.b1).
  Both well steps are appended after the metal stack: an antenna level names the layer
  whose first connect step it reads through, and a step added at the end changes no
  level, where one inserted into the stack would — anything added to the connect graph
  later belongs at the end for the same reason.
* **NBL.b / NBL.c** are the buried-layer twins of the pair above, and upstream omits them
  for the same reason. gdscheck implements NBL.b as a plain 1.50 µm
  :doc:`../checks/min_space` and NBL.c as the same check at 3.20 µm with
  ``net: different``. The ``NWellNBuLay`` connect step carries the
  net down the sinker — the NWell-ring/nBuLay overlap that nmosi.d sizes at 0.62 µm — so
  a buried layer takes the net of the well above it, and through the tap step, of
  whatever ties that well.
* **NBL.d** is net-dependent too — "Min. PWell width between nBuLay and NWell (different
  net)" — and is :doc:`../checks/min_space` with ``net: different`` in its two-layer form. It needs
  nothing new in the connect graph; nBuLay and NWell are both in it for NBL.c.
* **"unrelated" is not net language.** NBL.e/f ("space to *unrelated* N+/P+Activ") read as
  though they were, but §4.1 defines *unrelated* as "two regions which do not touch each
  other" — geometric. Plain :doc:`../checks/min_space` already implements exactly that,
  since the engine skips overlapping and touching pairs, so NBL.e/f stay geometric, as do
  Gat.b1, pSD.d, Sal.d and NBLB.d. Only "(different net)" in a rule's text means nets.
* **NBL.c / NBL.d and NW.b1 measure a gap, not PWell width.** The PDF words all three as
  "Min. PWell width between …", where PWell is the derived ``NOT (NWell OR PWell:block)
  OR PWell:drawing``. gdscheck measures the plain region-to-region gap instead. The two
  agree whenever the intervening space really is PWell, and diverge when something else
  sits in the gap — an NWell between two different-net nBuLay regions leaves less PWell
  than the raw gap suggests, so gdscheck can under-report there. NBL.d covers the
  nBuLay-to-NWell distance separately.


Known upstream deck issues
----------------------------

* **npn13G2.a/L.a/V.a off-by-one** — see above; a real bug in IHP's shipped
  ``ext_with_length`` helper, not a gdscheck defect. Reproduced and confirmed in a
  container against the actual reference deck.
* **Rhi.b recognition gap** — see above; the shipped rule's strict-containment device
  recognition makes it effectively inert on realistic layouts.
* **pSD.c / pSD.c1 degenerate markers** — KLayout's default ``ext_enclosed`` settings
  (``consider_intersecting_edges: true``) report spurious zero-area crossing-edge markers
  on every abutted-tie structure; gdscheck's coincident-edge classification
  (:doc:`../checks/min_enclosure`, ``skip_coincident``) is quiet on these by design.
