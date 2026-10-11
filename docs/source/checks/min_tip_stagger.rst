.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_tip_stagger
===============

Of two wires on adjacent routing tracks that run alongside each other, the tips on one
side must be aligned or at least the value apart along the track. ASAP7's ``M4.S.4``
(and M5-M7's) is this rule, the tip-to-tip spacing of two wires on adjacent tracks
that share a parallel run - drawn in DRM figure 3.14.2(d) between the two left tips.


Semantics
---------

The wires and their pairs are those of :doc:`min_track_run`: rectangular regions read
whole, two of them adjacent when their long walls face across ``facing`` with a gap
under ``within``, and here only pairs sharing a run greater than zero. For each end -
the low tips, then the high tips - the offset between the two wires' tips along the
track is a violation when it is greater than zero and under ``value``. Aligned tips are
clean: the manual draws two wires on adjacent tracks with their ends flush
(``M4.S.1``).


Layers
------

One layer, as for :doc:`min_track_run`.


Parameters
----------

``value`` (µm)
   The least offset between two tips on one side that are not aligned.

``facing`` (``x`` or ``y``, required)
   As for :doc:`min_track_run`.

``within`` (µm, required)
   As for :doc:`min_track_run`.


Violation markers
-----------------

One edge per staggered end, between the two tips, midway across the gap; a pair
staggered short at both ends gives two.


KLayout equivalent
------------------

ASAP7's KLayout deck reads ``M4.S.4`` as a projected notch under 40 nm in M4 grown
48 nm across the track, less the same-track ``M4.S.2`` gaps: walls facing each other,
not two tips on one side.


Example
-------

.. code-block:: yaml

    - id: M4.S.4
      check: min_tip_stagger
      layers: [M4NoSram]
      value: 0.04
      params:
        facing: y
        within: 0.072
