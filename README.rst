.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

========
gdscheck
========

A fast, open-source **DRC (Design Rule Check) engine for GDSII layouts**,
written in Rust.

``gdscheck`` is part of an effort to build fast, open-source sign-off tooling
that is **independent of commercial DRC engines**. Rules are
described in plain YAML decks, the geometry engine is parallelised with
`rayon`, and results are written as a KLayout-compatible report database so they
can still be visualised in any tool that reads ``.lyrdb``.

.. warning::

   ``gdscheck`` is **experimental and under active development** (``v0.1.0``).
   Coverage is incomplete and results are not yet qualified for tape-out. Always
   cross-check against a reference DRC engine before sign-off.


Features
========

* **Self-contained** — reads GDSII directly (plain or gzip-compressed) via the
  ``gds21`` crate; no dependency on KLayout, Magic or a foundry toolchain.
* **Declarative rule decks** — rules are YAML; no scripting required to express
  a standard rule set.
* **PDK-driven** — a PDK file maps layer *names* to GDS ``(layer, datatype)``
  pairs and lists the available decks and suites; rules reference layers by name.
* **Suites** — bundle a curated selection of rules from several decks (e.g. a
  ``precheck`` subset or a ``main`` full run) without duplicating rule definitions.
* **Hierarchy aware** — the full cell tree (``StructRef`` / ``ArrayRef``,
  including rotation, reflection, magnification and arrays) is flattened into a
  single coordinate frame before checking.
* **Virtual layers** — derive new layers from existing ones (e.g. a ``Pad``
  layer as the union of several passivation/bond layers).
* **Parallel** — every check fans out across cores with ``rayon``; spacing
  checks use bounding-box pre-filters and a sweep with binary search to stay
  near-linear on real layouts.
* **KLayout-compatible output** — violations are written to a ``.lyrdb`` marker
  database, grouped into a category tree by rule id, and can be loaded straight
  into KLayout's marker browser.


How it works
============

The pipeline (``src/lib.rs`` → ``run_drc``) is::

    GDS file ──▶ load_gds ──▶ flatten (apply transforms) ──▶ FlatLayout
                                                                  │
                           compute virtual layers ◀───────────────┤
                                                                  ▼
    PDK + deck (YAML) ──▶ rules ──▶ run each check ──▶ Violations ──▶ .lyrdb

1. **Load** the GDS library (``load_gds``); gzip is detected from the magic
   bytes.
2. **Flatten** the named top cell (``flatten.rs``). Each ``StructRef`` /
   ``ArrayRef`` is resolved by composing affine transforms down the hierarchy,
   producing a ``FlatLayout`` — boundaries indexed by ``(gds_layer,
   gds_datatype)`` in top-cell coordinates.
3. **Materialise virtual layers** (``pdk.rs`` → ``compute_virtual_layers``) from
   the flattened geometry.
4. **Run rules** (``checks/mod.rs`` → ``run_rule``) in deck order. Each check
   reads the layers it needs from the ``FlatLayout`` and returns a list of
   ``Violation``\ s. A small ``Cache`` memoises shared results (e.g. per-layer
   area for density checks).
5. **Report** (``report.rs``). Violations are printed to stdout and, if
   ``--report`` is given, serialised to a ``.lyrdb`` XML report database.


Installation
============

Requires a recent Rust toolchain (edition 2024).

.. code-block:: bash

   cargo install gdscheck

This installs the ``gdscheck`` binary. For a local build from source (and the
``gen-testdata`` test-fixture generator), see the *Getting started* guide in the
documentation (``docs/source/usage.rst``).


Usage
=====

``gdscheck`` is organised into subcommands:

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - Command
     - Purpose
   * - ``run``
     - Run DRC on a layout.
   * - ``list-decks``
     - Print the per-layer decks available in a PDK.
   * - ``list-suites``
     - Print the curated suites available in a PDK.
   * - ``show-deck``
     - Print every rule defined in a deck.

.. code-block:: bash

   gdscheck run \
       --input   <layout.gds[.gz]> \
       --process ihp-sg13g2 \
       --suite   main \
       --topcell <top-cell-name> \
       --report  report.lyrdb

Pass exactly one of ``--suite <name>`` (a curated rule selection) or
``--deck <name[,name...]>`` (one or more per-layer decks); the two are mutually
exclusive.

The PDK files are embedded in the binary, so ``--process ihp-sg13g2`` works with
no external files.  ``--process`` also accepts a path to a ``pdk.yml`` for a
custom or out-of-tree PDK.

Example, using the bundled IHP SG13G2 PDK and sample design:

.. code-block:: bash

   ./target/release/gdscheck run \
       --input   i2c-gpio-expander.gds.gz \
       --process ihp-sg13g2 \
       --deck    metal2 \
       --topcell <top-cell-name> \
       --report  report.lyrdb

Open ``report.lyrdb`` in KLayout (*Tools → Marker Browser → Load*) to step
through the violations.

Discover what a PDK offers without running a check:

.. code-block:: bash

   gdscheck list-decks  --process ihp-sg13g2
   gdscheck list-suites --process ihp-sg13g2
   gdscheck show-deck   --process ihp-sg13g2 --deck metal1

``run`` options
---------------

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - Option
     - Meaning
   * - ``-i, --input``
     - Input GDS file (plain or ``.gz``).
   * - ``-p, --process``
     - PDK process name (e.g. ``ihp-sg13g2``, embedded) or a path to a ``pdk.yml``.
   * - ``-d, --deck``
     - Per-layer deck(s) to run, comma-separated and/or repeated
       (e.g. ``metal2`` or ``metal1,via1``). Mutually exclusive with ``--suite``.
   * - ``-s, --suite``
     - A curated rule selection to run (e.g. ``main`` or ``precheck``).
       Mutually exclusive with ``--deck``. See `Suites`_.
   * - ``-t, --topcell``
     - Name of the top cell to flatten and check.
   * - ``-r, --report``
     - Optional output ``.lyrdb`` report path.
   * - ``--threads``
     - Worker threads (``0`` = all logical cores, the default).

``list-decks`` and ``list-suites`` take only ``-p, --process``; ``show-deck`` also
takes ``-d, --deck`` (the deck to dump).


PDK, deck and suite format
==========================

A **PDK file** declares the layer table, the available decks and suites, and any
virtual layers:

.. code-block:: yaml

   name: IHP SG13G2
   version: "1.0"

   suites:
     - name: main
       path: suites/main.yml
       description: Full DRC
     - name: precheck
       path: suites/precheck.yml
       description: Precheck subset

   decks:
     - name: metal2
       path: decks/metal2.yml
       description: Metal 2

   virtual_layers:
     - name: Pad
       op: union
       layers: [Passiv, Passiv.sbump, Passiv.pillar, dfpad]

   layers:
     - name: Metal2
       gds_layer: 10
       gds_datatype: 0
     - name: Metal2.filler
       gds_layer: 10
       gds_datatype: 22
     - name: EdgeSeal
       gds_layer: 39
       gds_datatype: 0

A **deck** is a list of rules. Each rule references layers by name, names a
check, and supplies a ``value`` plus optional ``params``:

.. code-block:: yaml

   rules:
     - id: M2.a               # minimum width
       check: min_width
       layers: [Metal2]
       value: 0.20

     - id: M2.b               # minimum same-layer spacing
       check: min_space
       layers: [Metal2]
       value: 0.21

     - id: M2.c               # windowed density floor over 200 µm tiles
       check: min_windowed_density
       layers: [Metal2, Metal2.filler]
       value: 20.0
       params:
         window: 200.0

* ``layers`` are resolved against the PDK layer table (and virtual layers).
* A second layer turns spacing/enclosure checks into **inter-layer** checks.
* ``value`` is in µm (widths/spaces), µm² (areas) or % (densities) depending on
  the check.

Each deck and suite entry may carry an optional ``description``; ``gdscheck
list-decks`` / ``list-suites`` print it next to the name, and ``show-deck`` prints
it in its header. Inspect a deck's rules with ``gdscheck show-deck --process
<pdk> --deck <name>``.


Suites
------

A **suite** is a named run that imports rules from one or more decks, so a single
``--suite`` value can pull together exactly the checks you want without
duplicating rule definitions. Suites are listed under ``suites:`` in the PDK and
selected with ``--suite``; per-layer decks are selected with ``--deck`` (the two
flags are mutually exclusive).

Each ``include`` names a deck. Add a ``rules`` whitelist to import only specific
rule ids from that deck, or omit it to import the whole deck:

.. code-block:: yaml

   # suites/precheck.yml
   include:
     - deck: metal1
       rules: [M1.a, M1.b, M1.j, M1.k]   # only these ids from metal1
     - deck: cont                        # no `rules:` → the whole cont deck
     - deck: forbidden

* A rule id may map to **several** entries in a deck (e.g. ``M1.b`` is both a
  ``min_space`` and a ``min_notch`` rule); the whitelist keeps all of them.
* An unknown id in a ``rules`` list is an **error**, so a typo cannot silently
  drop a check.
* Because a suite imports rule *definitions*, each rule has a single canonical
  value in its deck — a suite selects rules, it never overrides their values.

The bundled IHP SG13G2 PDK ships three suites: ``main`` (every per-layer deck),
``precheck`` (the open-source IHP precheck subset) and ``fill`` (the dummy-fill
density checks). Run ``gdscheck list-suites --process ihp-sg13g2`` to see them.


Supported checks
================

.. list-table::
   :header-rows: 1
   :widths: 26 74

   * - Check
     - Meaning
   * - ``min_width``
     - Layer features must be at least ``value`` µm wide.
   * - ``max_width``
     - Layer features must be at most ``value`` µm wide.
   * - ``exact_width``
     - Layer features must be exactly ``value`` µm wide.
   * - ``min_space``
     - Spacing within a layer (or between two layers) ≥ ``value`` µm.
   * - ``min_notch``
     - Same-polygon notch spacing ≥ ``value`` µm.
   * - ``min_area``
     - Polygon area ≥ ``value`` µm².
   * - ``max_area``
     - Polygon area ≤ ``value`` µm².
   * - ``min_enclosure``
     - ``layers[0]`` enclosed by ``layers[1]`` with clearance ≥ ``value`` µm.
   * - ``min_density``
     - Layer density over the chip/boundary area ≥ ``value`` %.
   * - ``max_density``
     - Layer density over the chip/boundary area ≤ ``value`` %.
   * - ``min_windowed_density``
     - Density ≥ ``value`` % in every ``window``×``window`` µm tile.
   * - ``max_windowed_density``
     - Density ≤ ``value`` % in every ``window``×``window`` µm tile.
   * - ``offgrid``
     - All vertices must lie on the ``value`` µm manufacturing grid.
   * - ``forbidden``
     - The layer(s) must be empty; any shape is a violation.
   * - ``inside_boundary``
     - Every shape must lie inside the boundary shape (e.g. EdgeSeal).
   * - ``ring_covers_boundary``
     - A ring layer must provide gap-free coverage of the boundary edges.
   * - ``no_ring``
     - The layer must not form a closed ring.

Density-check parameters
------------------------

* ``min_density`` / ``max_density`` accept ``boundary_layer`` (and optional
  ``boundary_datatype``); the denominator area is the bounding box of that layer
  (typically ``EdgeSeal``, GDS layer 39) instead of the whole layout.
* ``*_windowed_density`` require a ``window`` parameter (tile size in µm).


Output
======

Violations are printed to stdout and can be written to a ``.lyrdb`` report
(KLayout's XML report-database format) with ``--report``:

* Rule ids of the form ``Parent.child`` (e.g. ``M2.a``) are grouped into a
  two-level category tree.
* Each violation carries geometry — a **point**, an **edge** (the closest pair
  of edges for spacing), or **none** (global checks).


Testing
=======

``gdscheck`` ships a test-pattern generator and an integration test suite that
exercises each rule with both passing and failing layouts.

.. code-block:: bash

   # (Re)generate per-rule test GDS files into tests/data/
   cargo run --release --bin gen-testdata -- --pdk pdks/ihp-sg13g2/pdk.yml

   # Run the integration tests
   cargo test

The suite (``tests/ihp-sg13g2.rs``) runs a deck against each pattern and asserts
the exact set of triggered rule ids, so both false negatives and false
positives are caught.


Project layout
==============

.. list-table::
   :widths: 32 68

   * - ``src/main.rs``
     - CLI entry point.
   * - ``src/lib.rs``
     - ``load_gds`` / ``run_drc`` orchestration.
   * - ``src/flatten.rs``
     - Hierarchy flattening and affine transforms.
   * - ``src/layout.rs``
     - ``FlatLayout`` — boundaries by layer/datatype.
   * - ``src/pdk.rs``
     - PDK / deck parsing, virtual layers.
   * - ``src/checks/``
     - One module per DRC check.
   * - ``src/violation.rs``
     - Violation + geometry types.
   * - ``src/report.rs``
     - ``.lyrdb`` writer.
   * - ``src/cache.rs``
     - Memoisation across rules.
   * - ``pdks/ihp-sg13g2/``
     - IHP SG13G2 layer map and rule decks.
   * - ``gen/``
     - Test-pattern generator (``gen-testdata``).
   * - ``tests/``
     - Integration tests and test data.
