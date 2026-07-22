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
seal-ring, bond-pad and the §7.1 antenna rule family. Ships three suites:

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

* **Recommended (``*R``) rules** — e.g. ``Pad.aR/bR/dR/d1R/eR/fR/gR/jR/kR``, the
  resistor-family recommended variants, ``npn*.a/f`` (definitional, not independently
  checkable rules) — are advisory in the process documentation, not DRC-enforced, and are
  out of scope.
* ``Padb.e``/``Padc.e`` (bond-pad pitch) are not separate rules: pitch is exactly opening
  size plus spacing (verified numerically against the PDK's own pad table), and the
  process documentation states pitch is "not checked during DRC."
* ``Pad.m`` (SBumpPad/CuPillarPad exclusivity) has no rule-deck number at all — it's a
  KLayout-tooling-only check, not implemented.


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
* **NW.b1** (different-net well spacing) is net-blind: the geometric superset gdscheck
  implements can over-fire outside SRAM macros where legitimate same-net well gaps in
  the 0.62–1.80 µm range would otherwise be excluded by net awareness. A documented,
  accepted gap rather than a silent one.


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
