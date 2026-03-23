.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Getting started
===============

``gdscheck`` is a standalone geometry-only DRC engine: it reads a GDSII layout and a
YAML rule deck, and writes a KLayout-compatible ``.lyrdb`` marker database. It has no
dependency on KLayout, Magic or a foundry toolchain at runtime.


Installation
------------

Requires a recent Rust toolchain (edition 2024).

.. code-block:: bash

   cargo install gdscheck

This installs the ``gdscheck`` binary — the DRC engine.

Local build from source
^^^^^^^^^^^^^^^^^^^^^^^^

To build from a checkout instead (e.g. to work on ``gdscheck`` itself):

.. code-block:: bash

   cargo build --release

This produces ``gdscheck`` in ``target/release/``. Building with the ``dev-tools``
feature also produces ``gen-testdata``, which regenerates the per-rule test-pattern
GDS files used by the integration test suite (see :doc:`contributing`):

.. code-block:: bash

   cargo build --release --features dev-tools --bin gen-testdata


Running DRC
-----------

.. code-block:: bash

   gdscheck run \
       --input   <layout.gds[.gz]> \
       --process ihp-sg13g2 \
       --suite   main \
       --topcell <top-cell-name> \
       --report  report.lyrdb

Input GDS may be plain or gzip-compressed (detected from the magic bytes, not the file
extension). Pass exactly one of ``--suite <name>`` (a curated rule selection) or
``--deck <name[,name...]>`` (one or more per-layer decks) — the two are mutually
exclusive.

Other subcommands inspect a PDK without running a check:

.. code-block:: bash

   gdscheck list-decks  --process ihp-sg13g2
   gdscheck list-suites --process ihp-sg13g2
   gdscheck show-deck   --process ihp-sg13g2 --deck metal1

``show-deck`` prints every rule in a deck (id, check, layers, value, params) — useful
for confirming exactly what a suite pulls in before running it on a real layout.


Decks and suites
----------------

A **deck** is a YAML file of rules for one PDK subsystem (e.g. ``metal2.yml``,
``cont.yml``). A **suite** is a named, curated selection of rules imported from one or
more decks — e.g. ``main`` (everything) or ``precheck`` (a fast subset) — without
duplicating rule definitions. Suites are listed under a PDK's ``suites:`` section and
selected with ``--suite``; decks are selected directly with ``--deck``.

See :doc:`pdk-authoring` for the full deck/suite YAML format.


Selecting the process
----------------------

``--process`` accepts either an embedded PDK name (``ihp-sg13g2``, ``ihp-sg13cmos5l`` —
built into the binary, no external files needed) or a filesystem path to a ``pdk.yml``
for an out-of-tree or custom PDK. See :doc:`pdks/index` for the bundled PDKs and
:doc:`pdk-authoring` for writing your own.


Writing reports
----------------

Pass ``--report <path>.lyrdb`` to write a KLayout report database alongside the
console summary. Without ``--report``, violations are still printed to stdout (or the
run reports ``DRC clean.``) but nothing is written to disk. See :doc:`reports` for the
file format and how markers map to rule violations.

By default the console prints only a per-rule violation count; pass ``--verbose`` (or
``-v``) to print every individual violation's message. The ``.lyrdb`` report, when
requested, always contains full per-violation detail regardless of ``--verbose``.


Connectivity and net-aware checks
----------------------------------

A handful of checks are *net-aware* — they need to know which shapes are electrically
the same net (antenna-ratio rules, gate-connected protection-diode sizing). For these,
``gdscheck`` extracts nets from geometry alone, driven by the PDK's declared
``connectivity:`` graph (which connector layers — vias, contacts — bridge which
conductor layers). Net extraction is lazy: it only runs if the deck actually contains a
net-aware check, so a purely geometric run (e.g. a single metal deck) never pays for it.

Pass ``--no-connectivity`` to disable it explicitly; net-aware checks are then skipped
with a message instead of running (geometry-only checks are unaffected). Use this to
get a fast geometry-only pass, or when a design's connect graph doesn't resolve cleanly.


Command-line reference
-----------------------

``run``
^^^^^^^

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - Option
     - Meaning
   * - ``-i, --input``
     - Input GDS file (plain or ``.gz``).
   * - ``-p, --process``
     - PDK process name (embedded) or a path to a ``pdk.yml``.
   * - ``-d, --deck``
     - Per-layer deck(s), comma-separated and/or repeated. Mutually exclusive with ``--suite``.
   * - ``-s, --suite``
     - A curated rule selection. Mutually exclusive with ``--deck``.
   * - ``-t, --topcell``
     - Name of the top cell to flatten and check.
   * - ``-r, --report``
     - Optional output ``.lyrdb`` report path.
   * - ``--threads``
     - Worker threads (``0`` = all logical cores, the default).
   * - ``--no-connectivity``
     - Disable net extraction; net-aware checks are skipped.
   * - ``-v, --verbose``
     - Print every violation's message, not just per-rule counts.

``list-decks`` / ``list-suites``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Take only ``-p, --process``.

``show-deck``
^^^^^^^^^^^^^^

Takes ``-p, --process`` and ``-d, --deck`` (the deck to print).
