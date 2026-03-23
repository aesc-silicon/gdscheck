.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Troubleshooting & FAQ
=====================


Topcell not found
------------------

``gdscheck run`` errors immediately with ``Topcell '<name>' not found in library`` if
``--topcell`` doesn't exactly match a top-level structure name in the GDS (case-sensitive,
no wildcard matching). List the library's structures with KLayout or ``klayout -zz`` if
you're not sure of the exact name — GDS structure names are frequently longer or
differently-cased than the "obvious" guess (an EDA tool's exported top cell is often
named after the project, not just e.g. ``TOP``).


Compressed inputs
-------------------

``--input`` accepts both plain and gzip-compressed GDS transparently — compression is
detected from the file's magic bytes, not its extension, so a ``.gds`` file that's
actually gzipped (or vice versa) still works.


Off-grid geometry
-------------------

The ``offgrid`` check (:doc:`checks/offgrid`) flags vertices that don't land on a
manufacturing grid; ``value`` is the grid size in µm. If ``value`` rounds to less than
one database unit, the check logs a warning and reports nothing for that rule (there's no
meaningful sub-DBU grid to check against) — this usually means a stale or wrong ``value``
in the deck rather than a real design issue.


Lazy virtual layer errors
----------------------------

A lazy (``mode: lazy``) virtual layer (:doc:`virtual-ops`) is only resolved when a rule
actually references it — if you see ``source layer '<name>' not found`` at load time, the
layer name in a virtual-layer definition's ``layers:`` list doesn't resolve against the
PDK's layer table (a typo, or a layer defined in a base PDK that an ``extends`` chain
didn't pick up — remember ``extends`` only inherits ``layers``/``virtual_layers``, not
``decks``/``suites``/``connectivity``). Check the exact spelling with ``gdscheck
show-deck`` against the deck that references the virtual layer, and confirm the source
layer appears in ``gdscheck list-decks``' underlying ``pdk.yml`` layer table.

If a lazy virtual layer instead resolves to an unexpectedly *empty* result, check that
every source in its derivation chain has its own halo requirement satisfied — a layer
referenced only by the virtual-layer chain (not directly by any distance-based rule) gets
the run's baseline halo unless the chain's halo need was propagated to it (see
:doc:`architecture`, *Per-layer halos*).


Memory and thread tuning
---------------------------

``gdscheck`` bounds memory per layer via the tiled merge cache (:doc:`architecture`), so
peak memory should scale with local geometry density, not total chip size. If a run still
grows memory unexpectedly:

* Check whether a distance-based rule (``min_space``, ``max_width``, ``min_enclosure``,
  …) references a *dense* layer with an unusually large ``value`` — that inflates the
  layer's halo for every rule sharing it, not just the one that needs it.
* A check that globally merges a layer rather than using the tiled cache (documented on
  its own reference page when that's the case, e.g. :doc:`checks/must_interact`) is only
  safe for genuinely sparse layers (vias, contacts, isolated markers) — if such a check is
  pointed at a dense, chip-wide layer on a large design, expect it to scale poorly; that's
  a check-implementation limitation worth reporting, not something to work around by
  itself.

``--threads N`` caps the ``rayon`` pool (default: all logical cores) — useful to leave
headroom on a shared machine, or to get single-threaded, deterministic timing for
profiling. Threading is exact (see :doc:`architecture`, *Parallelism*), so it never
affects results, only wall-clock time.
