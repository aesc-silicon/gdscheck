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

Runs on the same per-tile merged cache and facing-wall scan as :doc:`min_width`, but
inverted: instead of measuring the metal between two facing walls, it measures the
*empty* gap between them. A notch narrower than ``value`` is a violation; each distinct
notch is reported once. Three passes run, covering every combination of wall
orientation:

- **Rectilinear scan.** Two vertical walls (a horizontal-direction notch) or two
  horizontal walls (a vertical-direction notch), found by sweeping bands across the
  region and pairing facing edges whose empty span is under ``value``.
- **Oblique pass** (``oblique_notches``). Two mutually anti-parallel diagonal walls —
  the same style of scan, generalized to non-axis-aligned edges, with the gap measured
  along each edge's own normal and the reported span limited to where the two edges'
  projected overlap actually runs.
- **Mixed pass** (``mixed_notches``). One rectilinear wall facing one diagonal
  wall — the case neither of the above two passes can see, since they only ever pair
  edges of the *same* orientation class. This is the shape a diagonal stroke makes
  closing in on a straight stem (the tip of a "V", the leg of a "K" or "M" — this is
  exactly what a diagonal font-glyph stroke does, which is how the gap was originally
  found). It uses the general segment-to-segment closest-point primitive (the same one
  :doc:`min_space` uses for inter-region gaps) rather than a parallel-corridor
  projection, since the pair isn't parallel at all; the result is only accepted as a
  real notch when each edge's closest point on the other lies on its own *empty* (right
  of direction-of-travel) side — every edge in a merged region carries metal on its left,
  a convention shared by all three passes — so a pair that's actually metal-filled
  between the two walls is correctly rejected.


Layers
------

One layer — the notch is measured within its own merged geometry (holes included).


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required notch width).


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
