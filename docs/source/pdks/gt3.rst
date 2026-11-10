.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

GT3
===


Overview
--------

``gt3`` is the 3 nm GAAFET PDK from the Georgia Institute of Technology (initial
release). The decks translate its IC Validator runset (``icv_runset/gt3_main.drc.rs``
and the files it includes) rule for rule, as `azadnaeemi/GT3
<https://github.com/azadnaeemi/GT3>`_ has it at ``fff4a84``. That revision removed
VG.ACT.1, the 6 nm VG-to-ACT space, which is not checked here either. There is one
deck per included file, and the ids are the ones the runset's rule descriptions give.
Layouts are read at the real 3 nm scale on a 0.5 nm grid.

The runset stops at M6. V6 to RDL, the cut spacings the runset names but does not
check, and the manufacturing grid come from the technology LEF
(``techlef/gt3_tech.lef``), with ids marked ``.LEF``. The suites are ``main`` (every
deck), ``feol`` (through V0, what a standard cell holds) and ``beol`` (M1 to RDL).


Translation
-----------

* ICV's ``direction = HORIZONTAL`` reads ``facing: x``, ``VERTICAL`` reads
  ``facing: y``, and ``external_corner1`` reads ``facing: none``. A space with
  ``extension = RADIAL`` also reads corner to corner, as ``facing: none``: ACT.3 is
  read across y and corner to corner.
* ``enclose`` is a ``min_enclosure`` read where the two layers interact.
* An exact width (``internal1 == v``, then the layer ``not`` the result) is an
  ``exact_width`` on that axis.
* SDCON.2, the tip-to-tip space between contacts on different nets, is net-aware. It
  runs on the runset's connect database, declared in ``pdk.yml``, less two steps.
  ``incremental_connect({gPOLY, aACT})`` joins every uncut gate to the active it
  crosses, and through it the gates sharing an active to each other.
  ``incremental_connect({aGATE, aVG})`` joins a VG to the whole gate, through any
  GCUT. A gate is insulated from its channel and a GCUT cuts it, so neither step joins
  conductors, and both are left out. SDCON.2 is then stricter than the runset: two
  contacts reached through M0 and VG from gates sharing an active are one net
  upstream, and two here.

Each deck's header lists the rules the runset names but does not check. Three rules are
read more closely than the runset does:

* GATE.2, an exact 27 nm gate space, is read as the 42 nm gate pitch it makes with
  GATE.1's 15 nm gate: every gate is centred on a multiple of 42 nm, as in the
  library, and no two are closer than 27 nm. A design drawn in another frame sets the
  rule's ``offset``. A gate missing from a row is reported too: two gates facing each
  other across x within 1 µm must be 27 nm apart. Gates further apart read as separate
  blocks, and a row's ends are never read. Upstream reads GATE.2 as the layer's extent
  less the gates and the 27 nm gaps, which reports a gap wider than 27 nm as well as a
  narrower one, and notes it misfires on gates out of line. :doc:`gt2n` reads its
  GATE.2 the same way.
* A via's edges must lie on its metals' outlines, which is how
  ``not_inside_touching_edge`` reads: a via inside a wider metal is off the metal's
  outline, just as a via sticking out of it is.
* ACT.6 requires every part of an active's end to lie in a DUMMY; the runset's
  ``not_interacting_edge`` passes an end the DUMMY covers only in part.

Upstream typos are corrected: ``BRP.1`` is ``BPR.1``, and ``V4.M3.EN`` is
``V4.M5.EN``.


Manufacturing grid
------------------

The technology LEF's ``MANUFACTURINGGRID 0.0005`` requires a 0.5 nm grid. The
``offgrid`` deck checks both coordinates of the merged geometry on all 41 drawing
layers, including NWELL. Pin annotations are excluded. Each off-grid vertex produces a
point marker under ``FEOL.offgrid.LEF`` (NWELL to V0) or ``BEOL.offgrid.LEF`` (M1 to
RDL). ``main`` includes both rules, ``feol`` and ``beol`` only their own, and
``--deck offgrid`` runs just the manufacturing-grid checks. The generated grid
fixtures use a 0.25 nm database unit to preserve off-grid coordinates.


Above M6: the tech LEF
----------------------

The runset checks nothing from V6 up. The decks ``v6`` to ``v13``, ``m7`` to ``m13`` and
``rdl`` take their rules from the technology LEF (`techlef/gt3_tech.lef
<https://github.com/azadnaeemi/GT3/blob/fff4a84248421acac92f252389e93bc9d748c342/techlef/gt3_tech.lef>`_)
instead, and every id is marked ``.LEF``, so a marker says which source it answers to.
:doc:`gt2n`'s LEF states the same rules there, and GT2N reads them with these decks.

Each metal reads as the runset reads M6, because M6's LEF block states the same rules
as the runset does, with one exception: ``.1`` is the ``WIDTH``, ``.2`` the length from
``MINSIZE``, ``.3`` the ``SPACING`` across the wires, ``.4`` the tip-to-tip space, the
``LEF58_EOLKEEPOUT``'s forward extension, ``.5`` the ``LEF58_CORNERSPACING``, and
``.6`` no bend, the ``LEF58_RECTONLY``. The exception is M6's length, 55 nm in the
runset (M6.2) and 57 nm in the LEF's ``MINSIZE`` and ``AREA``; M6 keeps the runset's.
Values in nm:

.. list-table::
   :header-rows: 1

   * - Layer
     - Direction
     - ``.1`` width
     - ``.2`` length
     - ``.3`` space
     - ``.4`` tip
     - ``.5`` corner
   * - M7, M9
     - vertical
     - 38
     - 57
     - 38
     - 55
     - 45
   * - M8
     - horizontal
     - 38
     - 57
     - 38
     - 55
     - 45
   * - M10, M11
     - horizontal, vertical
     - 56
     - none
     - 56
     - 80
     - 66
   * - M12, M13
     - horizontal, vertical
     - 360
     - none
     - 360
     - 450
     - 450
   * - RDL
     - horizontal
     - 1600
     - none
     - 1600
     - none
     - none

M10 to M13 have no LEF ``MINSIZE`` or ``AREA``, so no ``.2``. RDL has only a LEF
``WIDTH`` and a ``SPACING``: its ``.3`` is read every way, and it has no ``.4``, ``.5`` or
``.6``. ``.4`` reads every tip, as M6.4 does; the LEF's keep-out reads only a line end
narrower than its ``EOLKEEPOUT`` width (39, 57 and 361 nm), which every minimum-width
wire is.

A via's size is the cut its ``V<n>_0`` template and ``Via<n>Array`` rule draw, not
the cut layer's ``WIDTH``, as V5.1 and V5.2 hold V5 to its 21 × 38 nm template and
not its 21 nm ``WIDTH``. The two disagree from V9 up: V9's cut is 38 × 56 nm, V11's
56 × 360, V12's 360 × 360 and V13's 360 × 1600, where each ``WIDTH`` is 38 or 56 nm.
Each via has:

* ``.1`` and ``.2``, its width across x and y: the template's cut.
* ``.5``, the cut layer's ``SPACING``, read every way: 38 nm for V6 to V9, 56 nm for
  V10 to V13. The runset's via space (V1-V5's ``.6``) has no LEF value, and
  ``Via<n>Array``'s ``SPACING`` is the pitch of the cuts in an array a router
  generates, not a limit on two vias.
* ``.7`` and ``.8``, the cut inside its lower and its upper metal. The LEF's
  ``ENCLOSURE`` of 0 across a metal requires the cut inside it, which is stricter than
  the runset's V5.7 and V5.8, where the cut only has to touch. The LEF does not require
  a cut's walls on its metals' outlines, so the runset's ``.3`` and ``.4`` have no
  counterpart.
* ``V<n>.M<k>.EN.LEF``, each metal's enclosure along its length, from
  ``Via<n>Array``: 8 nm for V6 to V8, 10 for V9, 12 for V10 and V11, 20 for V12, and
  for V13, 40 by M13 and 200 by RDL. Every ``V<n>_0`` template meets them.

The LEF's routing ``PITCH`` is not a check here, as M1 to M6 have none.


Below V6: the LEF's cut spacing
-------------------------------

The runset names a spacing for V1 to V5 (``V<n>.5``) and for VG (``VG.6``) but checks
neither, and it has no spacing for VSD. The tech LEF gives each of these cut layers a
``SPACING``, which ``V<n>.5.LEF``, ``VG.6.LEF`` and ``VSD.6.LEF`` read every way, as
``.5`` does above M6. Values in nm:

.. list-table::
   :header-rows: 1

   * - Cut
     - V1
     - V2
     - V3
     - V4
     - V5
     - VG
     - VSD
   * - ``SPACING``
     - 12
     - 12
     - 14
     - 21
     - 21
     - 12
     - 12

The LEF's VBPR ``SPACING`` of 12 nm is not read. ``gt3_6t_dffasync_x1_rvt`` lands six
pairs of VBPRs on its middle rail, one from the SDCON above and one from the SDCON
below, 10 nm apart; GT2N's cells draw the same pairs, and its later LEF has no VBPR
cut layer at all. The runset has no VBPR spacing.

For V0 the runset gives two V0s on one M0 track 14 nm, under a second ``V0.7`` that it
never checks. ``V0.7a`` checks it, across x only: M0 is one V0 tall, so V0s on adjacent
tracks never share a y. The LEF's 12 nm adds nothing to that.

Everything else the LEF states for M0 to M6 and V0 to V5, the runset already checks at
the same values: each metal's width, length (``MINSIZE`` and ``AREA``), spacing,
tip-to-tip space (``LEF58_EOLKEEPOUT``), corner spacing and no bend, and each via's
``ENCLOSURE`` along its metals (``V<n>.M<k>.EN``). M6's length is the one difference,
covered above.


Tests
-----

Every rule in every deck has a generated pattern, written by ``gen/gt`` under
``tests/data/gt3/generated/<deck>/`` (``just gen-testdata-for gt3``). Each pattern
draws the shape exactly at the source's limit, then one 0.5 nm DBU past it, inside
whatever else its deck reads, such as a VG in its gate and M0. The generator is shared
with :doc:`gt2n`, whose runset is GT3's with seven values changed; it holds GT3's values
from ``icv_runset/Include/gt3.*.drc.rs`` and ``techlef/gt3_tech.lef``, and stops if a
deck disagrees. ``tests/gt3.rs`` names the count each pattern reports, and any other
rule the drawing must also break.

Run over the 6-track RVT library with every cell abutted into rows on its placement
boundary, the way a placer would, ``main`` reports one issue that is the library's
own: placed beside gt3_6t_oa211_x1_rvt, an M0 of gt3_6t_nor3_x2_rvt ends 16.5 nm
from the neighbour's, under M0.4's 17 nm. Every gate in it is on GATE.2's pitch, and
none is missing from a row.
