.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_space_different_net
=======================

Minimum spacing between merged regions that are **not** electrically the same net.


Semantics
---------

Geometrically identical to :doc:`min_space` — same merge, same tiling, same exact
closest-edge distance, same one-violation-per-pair ownership by the gap midpoint. The
only difference is a net gate applied to each pair that would otherwise be reported:
if both regions resolve to the *same* net, the pair is not a violation.

This models "different potential" spacing rules, where a process allows conductors at
one potential to sit closer than conductors at different potentials. The same-potential
minimum is normally a separate, plain :doc:`min_space` rule at a smaller value (for IHP
SG13G2: NW.b at 0.62 µm alongside NW.b1 at 1.80 µm).

The net of a region is looked up at its marker point on the rule's own layer, so
``layers[0]`` (and ``layers[1]`` in the two-layer form) must appear as a **conductor in
the PDK's connect graph**. See :doc:`../architecture` for net extraction.

.. note::

   If the rule's layer is a *merged* derived layer (a ``close``, as with NW.b1's
   ``NWellMergedNoSRAM``), put that merged layer in the connect graph rather than the
   drawn layer. A merged region's marker can land in a gap the close filled in — inside
   the merged layer, but outside any drawn shape — where a lookup on the drawn layer
   would not resolve.

Every uncertain case fails conservative and reports, exactly as the plain geometric rule
would: a marker that resolves to no region, a layer missing from the connect graph, or a
region with no net of its own (an untied, floating well). A wrong or missing net can
therefore only ever cost a false positive, never a false clean.

This is a net-aware check: it is skipped with a message when connectivity is disabled
(``--no-connectivity``), and its presence in a deck causes net extraction to run.


Layers
------

- Same-layer form: one layer — checks every region against every other region of it.
- Two-layer form: ``layers[0]``, ``layers[1]`` — each side's net is resolved on its own
  layer.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required gap).


Violation markers
-----------------

One edge marker per violating pair, drawn across the gap.


KLayout equivalent
------------------

None directly. The shipped IHP deck omits its two potential-dependent NWell rules
(NW.b and NW.b1) rather than approximate them; expressing this in KLayout needs a
``Netter``-based connectivity extraction driving a ``separation`` between per-net
region sets.


Example
-------

.. code-block:: yaml

    - id: NW.b1
      check: min_space_different_net
      layers: [NWellMergedNoSRAM]
      value: 1.80
