.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_notch
=========

Minimum notch — the dual of :doc:`min_width`: a concave gap between two facing,
inward-pointing walls of the **same** merged region (as opposed to :doc:`min_space`,
which measures the gap between two *different* regions).


Semantics
---------

Runs on the same per-tile merged cache and the same facing-wall scan as :doc:`min_width`,
turned the other way round: instead of measuring the material between two walls facing
each other, it measures the *empty* gap between two walls facing away from each other,
which is a slot cut into the region or a thin hole through it. A notch narrower than
``value`` is a violation; each notch is reported once, and the comparison is exact on
the grid (the value rounds up to whole DBU once, and every span is an integer). Three
passes run, covering every combination of wall orientation:

- **Rectilinear scan.** Two vertical walls (a horizontal-direction notch) or two
  horizontal walls (a vertical-direction notch), found by sweeping bands across the
  region and pairing facing edges whose empty span is under ``value``.
- **Oblique pass.** Two anti-parallel diagonal walls, with the gap measured along each
  edge's own normal over the run the two edges share.
- **Mixed pass.** One rectilinear wall facing one diagonal wall across the gap — the
  shape a diagonal stroke makes closing in on a straight stem, the tip of a "V" or the
  leg of a "K". The pair is measured at its closest approach and accepted only when each
  wall's closest point on the other lies on its own empty side.

What the width scan reads that a notch has no use for is left out: a plain box has no
notch, and the corner and acute-tip readings are about material narrowing to nothing.


Layers
------

One layer — the notch is measured within its own merged geometry (holes included).


Parameters
----------

``angle``
   Optional. ``bent`` restricts the rule to the 45° gaps alone: the oblique pass runs and
   the axis-aligned ones stay the plain rule's business.

``length``
   Optional, µm. Only wall pairs sharing more than this much run count — a notch asked
   only of slots longer than so much, or of a bent gap long enough to be a slot rather
   than a chamfer.


Violation markers
------------------

One edge marker per notch, spanning the gap between the two facing walls at their
narrowest point.


KLayout equivalent
-------------------

``Region#space(value)`` on a single region — KLayout's native spacing check reports both
inter-polygon gaps and same-polygon (concave) notches from one operator; gdscheck splits
these into :doc:`min_space` and ``min_notch`` as two separate rules sharing one rule ID
(e.g. IHP's ``TM2.b`` is ``min_space`` + ``min_notch`` together).


Example
-------

.. code-block:: yaml

    - id: TM2.b
      check: min_notch
      layers: [TopMetal2]
      value: 2.00
