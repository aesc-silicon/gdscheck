.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

must_interact
=============

Every region of the target layer (``layers[0]``) must overlap at least one shape from
the remaining (partner) layers; a region that doesn't is a violation. Used for "device
must contain its via/contact" rules, e.g. every MIM cap needs a landing via.


Semantics
---------

The target and all partner layers are merged **globally** — not through the tiled
:doc:`../virtual-ops` cache — and whole target regions are kept when they touch *none*
of the merged partner geometry. This is safe specifically because the layers this check
is used for are sparse device/via layers: MIM caps, vias, and similar countable
structures, not dense chip-wide layers.

That assumption matters: :doc:`forbidden_unless_labeled` used to make the same
whole-chip-merge choice for its own (dense) layers and had to move off it once real
antenna-diode markers turned out to be as numerous and widespread as the diffusion
itself, on a full SoC top cell. If a future rule ever needs ``must_interact`` on a dense
layer, it would need the same tiled/region-stitched treatment.


Layers
------

``layers[0]``
   The target layer; every one of its regions must have a partner.
``layers[1..]``
   One or more partner layers (the target only needs to overlap *any* of them).


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per target region with no interacting partner, at its centroid.


KLayout equivalent
------------------

``target.not_interacting(partner1.join(partner2, …))``.


Example
-------

.. code-block:: yaml

    - id: MIM.h
      check: must_interact
      layers: [MIM, TopVia1, Vmim]
      value: 0.0
