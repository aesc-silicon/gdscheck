.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_array_space
===============

Via/contact-array spacing: in an array larger than ``rows``×``cols``, the spacing must
reach ``value`` in at least one axis (the other axis only needs the ordinary
:doc:`min_space` rule). Equivalently, this flags an array that is tight (below
``value``) in **both** axes simultaneously and exceeds the row/column thresholds (e.g.
IHP ``V1.b1``, ``V2.b1``, ``Cnt.b1``).


Semantics
---------

Detecting genuine two-dimensional density mirrors the reference deck's morphological
test (close, then erode by half an array-block extent), implemented here directly on
via geometry:

- A **run** is a maximal chain of horizontally tight vias — each edge gap under
  ``value`` and each pair's y-extents overlapping.
- A run *qualifies* once it's longer than ``cols``.
- Qualifying runs are then stacked: a **stack** is a maximal chain of qualifying runs
  whose x-extents overlap and whose vertical edge gap is under ``value``.
- A stack deeper than ``rows`` is the violation.

This is exactly "tight in both directions": a via **ring** (e.g. around a bond pad) has
long runs but never stacks more than two deep; a single row or column never stacks at
all; and relaxing either axis to ``value`` or more breaks either the runs or the
stacking — all stay clean.

Operates on the whole (global) via/contact layer rather than the tiled merge cache,
since an array can span merge tiles; vias are read as individual rectangles (deduplicated
by extent) rather than boolean-merged, since they don't touch.


Layers
------

One layer — the via or contact array being checked.


Parameters
----------

``rows``
   Optional, default ``3``. Array-size threshold: a stack must exceed this many
   qualifying runs to violate ("more than N").

``cols``
   Optional, default ``3``. Array-size threshold: a run must exceed this many vias to
   qualify.


Violation markers
------------------

One point marker per violating stack, at the centroid of all vias in it, with a message
noting the stack's row×column extent.


KLayout equivalent
-------------------

Mirrors the reference deck's array-density idiom: a morphological close of the via
layer by roughly half the spacing, eroded back by half an array-block extent, which
only survives where vias are genuinely dense in both directions — as opposed to a
plain ``space(value)`` check, which a via ring or single row/column would also trip.


Example
-------

.. code-block:: yaml

    - id: V1.b1
      check: min_array_space
      layers: [Via1]
      value: 0.29
      params:
        rows: 3
        cols: 3
