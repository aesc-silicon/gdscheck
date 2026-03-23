.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_space_bent
==============

A wider minimum space between two regions where at least one has a 45°-bent (diagonal)
wall *near the gap* (e.g. IHP ``M5.i``).


Semantics
---------

Reuses the same tiled region-pair spacing engine as :doc:`min_space`, but gates each
candidate pair with an extra condition: the pair only violates if one of the two
regions has a diagonal edge within the gap distance of the *other* region. This is a
locality condition, not a whole-polygon one — a long net that runs at 45° in one place
and Manhattan elsewhere only gets the wider spacing where it's actually angled;
elsewhere on the same polygon, ordinary ``min_space`` semantics apply.


Layers
------

One layer (same-layer spacing) or two, exactly as in :doc:`min_space`.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required gap wherever a diagonal
wall is present near it).


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap — same convention as
:doc:`min_space`.


KLayout equivalent
------------------

Not a single built-in operator: KLayout decks typically build this as a plain
``space(value)`` restricted (via ``interacting``) to edges matching a 45° angle filter.


Example
-------

.. code-block:: yaml

    - id: M5.i
      check: min_space_bent
      layers: [Metal5]
      value: 0.24
