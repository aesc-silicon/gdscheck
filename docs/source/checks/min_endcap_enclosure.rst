.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_endcap_enclosure
====================

Like :doc:`min_enclosure`, but the margin is only required on at least **one** side
rather than all of them — the wire *endcap* case (e.g. a via sitting at the corner of its
metal, where the metal only needs to run past it on one side).


Semantics
---------

Shares `min_enclosure`'s engine (same containment test), but instead of the facing
edge-pair scan it uses the enclosing region's bounding-box margin on each side of the
enclosed shape and takes the *best* (largest) one — a region violates only if *every*
side is short. This bounding-box reduction is deliberate: an edge-to-contour distance is
corner-limited and would understate a long endcap run.

In practice this rule is paired with an ordinary `min_enclosure` entry at a much smaller
value covering the rest of the shape's sides (e.g. IHP's `V1.c` at 0.01 µm alongside
`V1.c1` at 0.05 µm) — together they express "mostly flush is fine, but at least one side
needs real margin."


Layers
------

Two layers, positional — same roles as :doc:`min_enclosure`:

1. ``layers[0]`` — the enclosing region.
2. ``layers[1]`` — the enclosed shapes being measured.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required margin on the best side).


Violation markers
------------------

One edge marker per offending shape, along its first contour edge (a location marker —
the endcap reduction doesn't identify a single "worst wall" the way the facing-pair scan
does, since it's driven by the bounding box).


KLayout equivalent
------------------

No single built-in KLayout primitive for "at least one side" enclosure; it's the same
`enclosed` family, restricted here to a bounding-box side-margin comparison instead of
per-edge facing pairs.


Example
-------

.. code-block:: yaml

    # V1.c allows the via to be nearly flush with Metal1 on most sides (0.01 µm); V1.c1
    # additionally requires a real endcap (0.05 µm) on at least one side.
    - id: V1.c
      check: min_enclosure
      layers: [Metal1, Via1NoSealring]
      value: 0.01
    - id: V1.c1
      check: min_endcap_enclosure
      layers: [Metal1, Via1NoSealring]
      value: 0.05
