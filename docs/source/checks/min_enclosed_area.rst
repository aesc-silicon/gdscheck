.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_enclosed_area
==================

A hole — an empty region fully surrounded by the layer — must be at least ``value`` µm².
Tiny holes (slivers left by a boolean operation, or a genuinely undersized ring opening)
are reported.


Semantics
---------

Holes live directly in each tile's merged region representation (an outer contour plus
its hole contours), so this check simply measures each hole's area and flags the ones
under ``value``. Since holes are already tracked per merged region, no separate
whole-chip reconstruction is needed — a hole entirely local to one tile's merge is
measured there, owned by whichever tile core contains its centroid.


Layers
------

One or more layers; holes on every listed layer are checked independently against the
same ``value``.


Parameters
----------

None beyond ``layers`` and ``value`` (µm², the minimum required enclosed area).


Violation markers
------------------

One point marker per undersized hole, at its centroid.


KLayout equivalent
------------------

``layer.with_holes`` combined with an area filter on the hole itself (KLayout has no
single "minimum hole area" operator; it's typically expressed as extracting holes via
`Region#holes` and filtering by `with_area`).


Example
-------

.. code-block:: yaml

    - id: Act.e
      check: min_enclosed_area
      layers: [Activ]
      value: 0.15
