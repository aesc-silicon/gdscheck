.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_space
=========

Maximum proximity-coverage distance: every part of a target layer must lie within
``value`` of a reference layer. Used for latch-up rules such as "every source/drain
diffusion must be within 20 µm of a well tie."


Semantics
---------

This is a coverage/reach check, not a spacing check: it doesn't flag two regions that
are *too close*, it flags parts of ``layers[0]`` that are *too far* from any region of
``layers[1]``. Both layers are tiled (never globally unioned, so a dense reference layer
like ``Cont`` stays memory-bounded); for each gap found between the target and its
nearest reference geometry, a point is reported wherever that gap exceeds ``value``.


Layers
------

- ``layers[0]`` — the target that must stay close to something.
- ``layers[1]`` — the reference geometry it must reach.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the maximum allowed distance to the nearest
reference region).


Violation markers
------------------

One point marker per proximity gap found, at the far point of ``layers[0]`` that
exceeds ``value`` from the nearest ``layers[1]`` region.


KLayout equivalent
-------------------

Not a single built-in operator — conceptually the inverse of ``Region#separation``:
KLayout decks typically express this as ``target.not_interacting(reference.sized(value))``
or an equivalent "shrink the reach and see what falls outside it" construction.


Example
-------

.. code-block:: yaml

    - id: LU.a
      check: max_space
      layers: [PsdActivInNWell, NActivInNWell]
      value: 20.00
