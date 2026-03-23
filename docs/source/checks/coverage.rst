.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

coverage
========

Every region of the target layer (``layers[0]``) must lie inside the union of the
remaining layers; any part sticking out is a violation. Used for "device must be
covered by" rules such as a pin marker that must sit on real metal.


Semantics
---------

Computed as the ``Difference`` residual ``target − (cover₁ ∪ cover₂ ∪ …)`` over the
cached tiles — see :doc:`../virtual-ops`. Unlike :doc:`min_enclosure`, this handles
"covered by A *or* B" directly (a union of covers, not a single enclosing layer) and
flags a target region that misses the cover *entirely*, not just one that's short on
margin at an edge.


Layers
------

``layers[0]``
   The target layer that must be covered.
``layers[1..]``
   One or more cover layers; the target only has to lie inside their *union*.


Parameters
----------

None. ``value`` is unused (set it to ``0.0`` by convention).


Violation markers
------------------

One violation per residual region — the part of the target left over after subtracting
all cover layers — owned by the tile containing its centroid.


KLayout equivalent
------------------

``target.not(cover1.join(cover2, …))`` — a plain ``NOT`` against the joined covers.


Example
-------

.. code-block:: yaml

    - id: Pin.a
      check: coverage
      layers: [Activ.pin, Activ]
      value: 0.0
