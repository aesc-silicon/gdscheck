.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_track_run
=============

Two wires on adjacent routing tracks that run alongside each other at all must run
alongside for at least the value. ASAP7's ``M4.S.5`` (and M5-M7's) is this rule: a
wire on the next track over shares 44 nm or more of its run with its neighbour, or
none.


Semantics
---------

Each connected region of the layer is read whole, across tiles, as its bounding box -
only regions that are rectangles, since a one-way metal may not bend (``M4.AUX.3``)
and a bent region is that rule's business. Two wires are on adjacent tracks when their
long walls face each other across ``facing`` with a gap greater than zero and under
``within``. The run they share is the overlap of their extents along the track; a run
greater than zero and under ``value`` is a violation. Two wires sharing no run are tip
to tip across the tracks, which a corner spacing reads (``M4.S.3``).


Layers
------

One layer: the one-way metal, apart from any region the manual exempts (ASAP7's
``M4NoSram``).


Parameters
----------

``value`` (µm)
   The least run two adjacent wires may share.

``facing`` (``x`` or ``y``, required)
   The axis across which the wires' long walls face each other: ``y`` on a layer that
   routes along x, the same as its width and side-space rules.

``within`` (µm, required)
   The gap under which two wires are on adjacent tracks. ASAP7 sets it to three minimum
   widths, the least gap another wire fits into with its space either side: two
   minimum-width wires on neighbouring tracks are one width apart and adjacent, two
   tracks apart three widths and not, and a three-width wire is two widths from the
   nearest wire beside it, with no track free between.


Violation markers
-----------------

One edge per pair, along the shared run, midway across the gap.


KLayout equivalent
------------------

ASAP7's KLayout deck reads ``M4.S.5`` as
``m4.space(25.nm, projection).edges.with_angle(0).with_length(0..44.nm)``: the
projected edges of a space under 25 nm, two per pair, so only minimum-width neighbours.


Example
-------

.. code-block:: yaml

    - id: M4.S.5
      check: min_track_run
      layers: [M4NoSram]
      value: 0.044
      params:
        facing: y
        within: 0.072
