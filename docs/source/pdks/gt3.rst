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

The suites are ``main`` (every deck), ``feol`` (through V0, what a standard cell holds)
and ``beol`` (M1 to M6). The runset has no rules for V6 and above, and neither has
this PDK.


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
  runs on the runset's own connect database, declared in ``pdk.yml``.

Each deck's header lists the rules the runset names but does not check. GATE.2, an
exact 27 nm gate space, is checked as a minimum, which is how the runset effectively
reads it too. A via's edges must lie on its metals' outlines, which is how
``not_inside_touching_edge`` reads: a via inside a wider metal is off the metal's
outline, just as a via sticking out of it is. ACT.6 requires every part of an active's
end to lie in a DUMMY; the runset's ``not_interacting_edge`` passes an end the DUMMY
covers only in part. Upstream typos are corrected: ``BRP.1`` is ``BPR.1``, and
``V4.M3.EN`` is ``V4.M5.EN``.


Tests
-----

Every rule in every deck has a generated pattern, written by ``gen/gt`` under
``tests/data/gt3/generated/<deck>/`` (``just gen-testdata-for gt3``). Each pattern
draws the shape exactly at the runset's limit, then one 0.5 nm DBU past it, inside
whatever else its deck reads, such as a VG in its gate and M0. The generator is shared
with :doc:`gt2n`, whose runset is GT3's with seven values changed; it holds GT3's values
from ``icv_runset/Include/gt3.*.drc.rs`` and stops if a deck disagrees.
``tests/gt3.rs`` names the count each pattern reports, and any other rule the drawing
must also break.

Run over the 6-track RVT library with every cell abutted into rows on its placement
boundary, the way a placer would, ``main`` reports one issue that is the library's
own: placed beside gt3_6t_oa211_x1_rvt, an M0 of gt3_6t_nor3_x2_rvt ends 16.5 nm
from the neighbour's, under M0.4's 17 nm.
