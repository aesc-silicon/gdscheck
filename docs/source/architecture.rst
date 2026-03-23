.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Architecture
============

.. code-block:: text

    GDS file ──▶ load_gds ──▶ flatten (apply transforms) ──▶ FlatLayout
                                                                  │
                            compute *eager* virtual layers ◀──────┤
                                                                  ▼
    PDK + deck (YAML) ──▶ rules ──▶ run each check ──▶ Violations ──▶ .lyrdb
                                        │
                                        ▼
                          MergedCache (tiled merge, per-layer halo,
                          lazy virtual layers, region stitching,
                          net extraction) — shared across all rules


Design goals
------------

``gdscheck`` targets real, full-chip layouts, not just small test patterns. That drives
two constraints that shape everything below:

* **Memory must stay bounded regardless of chip size.** A dense layer like ``Activ`` or
  ``GatPoly`` on a full SoC top cell can carry many millions of shapes; no single check
  may ever union or hold that in memory all at once.
* **A single run pays once for shared geometry.** A deck touches the same layer from
  several rules (e.g. ``Metal2`` from a width, a space and a density rule); the merged
  geometry for a layer is computed once and reused, not recomputed per rule.


Hierarchy flattening
---------------------

``flatten.rs`` resolves every ``StructRef``/``ArrayRef`` in the cell tree under the
requested top cell by composing affine transforms (translation, rotation, reflection,
magnification, and array repetition) down the hierarchy, producing a single
``FlatLayout`` — boundaries and text labels indexed by ``(gds_layer, gds_datatype)`` in
top-cell coordinates. Only the layers actually referenced by the deck (plus any layers
feeding a referenced virtual layer, transitively, plus the connectivity graph's layers if
net-aware checks are running) are flattened — an unreferenced layer never enters memory.


The tiled merge cache
-----------------------

``MergedCache`` (``merge.rs``) is the shared, per-layer merged-geometry cache every
geometric check reads from. Instead of unioning a whole layer's shapes in one operation,
it splits the chip into fixed-size tiles (``TILE_UM``, 20 µm) and merges each tile's
shapes independently — a *tile*'s output only depends on shapes within that tile plus a
*halo* margin around it, so tiles merge in parallel and none of them ever holds more than
a local neighbourhood's worth of geometry.

A check calls ``MergedCache::ensure`` for a layer; the first check to touch a layer pays
for its tiled merge, every later check reading the same layer reuses the cached result.


Per-layer halos
----------------

A tile's halo must be at least as large as the biggest distance any rule measures on that
layer, or a shape just across the tile boundary could be missed and a real violation (or
a real pass) computed wrong at the tile's edge. The halo is **per layer, not per run**:
before the merge cache is built, ``lib.rs`` scans every distance-based rule (``min_width``,
``max_width``, ``exact_width``, ``min_space``, ``min_notch``, ``min_enclosure``,
``max_enclosure``) and records, per layer, the largest ``value`` referencing it. A deck-wide
halo would let one coarse rule (e.g. a 1500 µm ``max_width`` on a guard layer) inflate the
merge of every fine layer in the deck — for a dense layer that difference is the
difference between a normal run and one that never finishes.

A ``min_space``/``min_notch`` rule referencing a layer with **no shapes at all** in the
current design is skipped when computing halo (an empty partner must not inflate the other
layer's halo just because a rule exists on paper).


Region stitching
-----------------

Some questions can't be answered from one tile in isolation: "is this whole connected
region wide enough," "does this whole region touch a text label anywhere," "does this
whole region interact with another layer." ``stitch_regions``/``stitch_labeled``
(``merge.rs``) answer these without ever reconstructing or globally unioning the source
geometry: each tile independently computes the piece of every region that falls in its
*core* (as opposed to its halo), and pieces in adjacent tiles are joined with a union-find
whenever their shared core edge shows continuous coverage. A region's aggregate property
— total area, a representative marker point, or a caller's boolean predicate ("does any
piece touch layer B") — is then just an OR/sum reduction over its pieces, which stays cheap
however many tiles a real-world region spans.

This is what lets checks like windowed-density's plate analysis, or the
``interacting``/``not_interacting``/``with_text`` virtual-layer selectors, run correctly
on chip-spanning regions while only ever touching tile-local, halo-bounded geometry.


Lazy virtual layers
---------------------

A PDK's ``virtual_layers:`` (declared in ``pdk.yml``, see :doc:`virtual-ops`) come in two
evaluation modes:

* **Eager** (the default) — computed once, up front, as ordinary boundaries inserted
  into the flattened layout (``pdk.rs`` → ``compute_virtual_layers``). Fine for small or
  sparse derived layers.
* **Lazy** (``mode: lazy``) — registered with the ``MergedCache`` as a ``TiledVirtual``:
  a synthetic ``(layer, datatype)`` key built per tile, on first ``ensure``, by applying a
  boolean/selection/morphological op to its source layers' tiles (each recursively
  ``ensure``\ d in turn). A lazy virtual layer costs nothing until something actually
  asks for it, and its
  memory profile is the same tile+halo bound as any drawn layer — the mode a dense,
  multi-step derivation (like the antenna forbidden-region chain in
  :doc:`checks/forbidden_unless_labeled`) must use to stay bounded on a full chip.

A lazy virtual layer that feeds another must have its own halo raised to cover the
downstream layer's needs, transitively — ``lib.rs`` propagates this before the merge cache
is constructed.


Net extraction
---------------

Net-aware checks (the antenna-ratio family, gate-connected minimum area) need to know
which shapes are electrically the same net. ``connectivity.rs`` extracts nets from
geometry alone: the PDK declares a list of connect specs, each a *connector* layer (a via
or contact) and the conductor layers it bridges. A layer's own connected regions are
already available via ``stitch_labeled``; a connector sits inside every layer it joins, so
a single point of the connector lands in one region of each — those regions are unioned
into one net via a union-find over ``(layer, region)`` nodes. Extraction is lazy: it only
runs if the deck actually has a net-aware check (see ``NET_AWARE_CHECKS``), so a
geometry-only deck never pays for it, and it can be disabled outright with
``--no-connectivity``.


Memory bounding and eviction
------------------------------

Two further mechanisms keep peak memory flat as a deck grows:

* **Eviction.** ``lib.rs`` records the last rule index that references each layer before
  running any rule. Right after that rule runs, the layer's cached tiles (and any stitched
  regions) are dropped from the ``MergedCache``. A deck touching dozens of layers only
  keeps a handful resident at any point, not all of them at once.
* **Per-check-run scoping.** Only layers actually reachable from the current deck's rules
  (transitively through virtual-layer sources and the connectivity graph) are flattened
  from the GDS in the first place — a layer the deck never references never exists in
  memory at all.


Parallelism
-----------

Every tile-level operation (raw-layer merge, virtual-layer composition, region stitching,
per-window density) fans out across ``rayon``'s thread pool, one task per tile key; the
``--threads`` flag (default: all logical cores) controls the pool size. Because tiles are
independent (correctness depends only on the halo, not on execution order), this
parallelism is exact, not an approximation — the result of a threaded run is identical to
a single-threaded one.
