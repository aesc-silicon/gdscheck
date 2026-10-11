.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

offtrack
========

Every region on the rule's layer must be centred on a track: the midpoint of its extent
across one axis lies a whole number of pitches from an offset. The routing-track rules
of a one-way metal are this reading - ASAP7's ``M4.AUX.2`` (a minimum-width wire lies on
a track every 48 nm) and ``M4.W.4`` (a wider wire spans an odd number of tracks, which
is the same wire centred on one) - and so is an exact gate or fin pitch, ``GATE.S.1``'s
54 nm.


Semantics
---------

Each connected region of the layer is read whole, across tiles, as its bounding box; the
extent across the ``facing`` axis is the box's span that way, and its midpoint is the
region's centreline. The centreline is on a track when ``(centre - offset)`` is a whole
multiple of ``value``; otherwise the region is a violation. Centres and tracks are read
in half-DBU, so a region whose extent is an odd number of DBU has its centre read
exactly - half a DBU off its track is off it, not rounded onto it.

The reading is of the box, so a bent region is read by the extent of the whole bend. On
a layer that may not bend (``M4.AUX.3``) every region is a rectangle and the box is the
wire.


Layers
------

One layer. A rule on a one-way metal names the layer apart from any region the manual
exempts (ASAP7's ``M4NoSram``), and a rule that applies to one class of wire names a
selection: ``M4Track: M4NoSram with_bbox_min max 0.02425`` is the minimum-width wires
and ``M4Wide: M4NoSram not M4Track`` the rest.


Parameters
----------

``value`` (µm)
   The pitch of the tracks. At least one DBU.

``facing`` (``x`` or ``y``, required)
   The axis across which the tracks lie - the axis the extent is read along. A
   horizontal routing layer's tracks lie across ``y``; its wires' width is read the same
   way, so the rule's ``facing`` is the width rule's.

``offset`` (µm, default ``0``)
   Where the first track's centreline lies. On a whole or half DBU. ASAP7's manual gives
   the offset of a track's *edge* and sets it per design; the centreline offset is that
   plus half a minimum width - 12 nm on M4 with the manual's default of 0.


Violation markers
-----------------

One point violation per off-track region, at the region's marker, saying how far off
its centre is.


KLayout equivalent
------------------

None built in. ASAP7's KLayout deck leaves ``M4.AUX.2`` uncoded; a script would take
``region.bbox`` per merged polygon and test its centre modulo the pitch.


Example
-------

.. code-block:: yaml

    - id: M4.AUX.2
      check: offtrack
      layers: [M4Track]
      value: 0.048
      params:
        facing: y
        offset: 0.012
