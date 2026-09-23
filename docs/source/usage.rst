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

``--threads`` caps the worker threads (default: every core). ``--tile <µm>``, or
``GDSCHECK_TILE_UM``, sets the tile the merge cache works in (default 20): every layer is
merged, stitched and measured per tile of that size with a halo round it. A smaller tile
bounds memory tighter and cuts the geometry into more pieces, a larger one holds more of
a dense layer whole and copies less of it into halos. The result must not depend on it;
a run that differs between two tile sizes is a bug worth reporting.

``GDSCHECK_WAVE=N`` lets up to ``N`` rules run side by side over the shared merge cache,
within the memory plan; the default is four, ``1`` runs them one at a time (see
:doc:`architecture`, *Parallelism*). The result does not depend on it.

``--memory <size>``, or ``GDSCHECK_MEMORY``, is what the run may take (``12G``,
``800M``). Without it the run plans within the tightest cgroup limit above it — a
container, a ``systemd-run`` scope, a CI runner — or, with none, the machine's
``MemTotal``. Either way the plan keeps a margin under the amount: a twentieth (at
least 256 MB) for what a burst of allocation adds between two readings of the resident
set, and a tenth of the rest for a rule's own working set. The flattened layout and the
nets stay resident for the whole run; the merge cache is sized to what the plan leaves
after them, and the run says so in one line (``Memory: planning within 10.3 GB (cgroup
limit 12.0 GB), 7.1 GB resident, 3.2 GB for the merge cache``). A smaller cache means
layers are merged again when a later rule needs them: slower, same result. A limit the
resident part already exceeds turns the cache off and says so on stderr.

The run keeps to its limit as it goes: the net-aware rules run first and the nets are
freed after them, the cache gives back what a rule overshot the plan by, and a rule
whose layers would take twice what is left is not checked but recorded — under its
rule in the summary (``N rule(s) not checked:``) and in the report, tagged ``skipped``
— with exit status ``3``. A run that outgrows the limit anyway ends with a message
saying where it was and what helps, where the kernel would have killed it silently.
The run also caps glibc's malloc arenas at its thread count (``MALLOC_ARENA_MAX`` in the
environment overrides it): glibc opens up to eight per core and each keeps the slack of
what was freed in it, a gigabyte on the gf180 reference design on a 32-core machine, at no
cost in time.

``gdscheck stats`` is where to start on such a run: with the same ``--input``,
``--process``, ``--topcell`` and ``--suite`` or ``--deck`` it flattens the layout,
prints the shapes of every layer the rules read, largest first, with the memory limit
and what is resident then, and stops — the layer a hundred times the size of the others
is the one a slow or killed run is about.

Other subcommands inspect a PDK without running a check:

.. code-block:: bash

   gdscheck list-processes
   gdscheck list-decks  --process ihp-sg13g2
   gdscheck list-suites --process ihp-sg13g2
   gdscheck show-deck   --process ihp-sg13g2 --deck metal1
   gdscheck stats       --process ihp-sg13g2 --suite main --topcell TOP --input chip.gds.gz

``show-deck`` prints every rule in a deck (id, check, layers, value, params) — useful
for confirming exactly what a suite pulls in before running it on a real layout.
``stats`` flattens the layout for a suite or decks and prints what a run would take:
the shapes of every layer the rules read, largest first, and the memory limit with
what is resident then (see *Memory* below).


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

``--process`` accepts an embedded PDK name (``ihp-sg13g2``, ``ihp-sg13cmos5l`` — built
into the binary, no external files needed), a filesystem path to a ``pdk.yml``, or the
name of a PDK on the *PDK path*. See :doc:`pdks/index` for the bundled PDKs and
:doc:`pdk-authoring` for writing your own.

The PDK path is where PDKs that cannot live in the binary go — a commercial process
under NDA, a company's own variant of a bundled one. It is a list of directories, each
holding processes the way the source tree's ``pdks/`` does: ``<dir>/<process>/pdk.yml``
with the decks beside it. Name directories with ``--pdk-path <dir>`` (before the
subcommand, repeatable) or in the ``GDSCHECK_PDK_PATH`` environment variable, separated
like ``PATH``; the option's directories are searched first, then the variable's, then
the embedded PDKs. A process on the path shadows a bundled one of the same name, and
``list-processes`` prints every process with where it comes from, marking the shadowed
ones.

.. code-block:: bash

   export GDSCHECK_PDK_PATH=/opt/pdks/gdscheck
   gdscheck list-processes
   gdscheck run --process acme-28 --suite main --topcell TOP --input chip.gds.gz

An external PDK can build on a bundled one: ``extends: ihp-sg13g2`` inherits the base's
layers, and a deck path such as ``../ihp-sg13g2/decks/activ.yml`` that does not exist
beside the external ``pdk.yml`` is read from the embedded copy, so the external tree
carries only what it adds or changes. The ``pdk.yml`` format is not frozen while
``gdscheck`` is pre-1.0: an external PDK may need touching up after an update, and a
load error names the file and field.


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
the same net (antenna-ratio rules, gate-connected protection-diode sizing, different-net
spacing such as IHP's NW.b1). For these,
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
   * - ``--memory``
     - Memory the run may take (``12G``); default: the cgroup's limit or ``MemTotal``.
   * - ``--no-connectivity``
     - Disable net extraction; net-aware checks are skipped.
   * - ``-v, --verbose``
     - Print every violation's message, not just per-rule counts.

Exit status: ``0`` when the layout is clean, ``2`` when violations were found, ``1`` on
any error (unreadable input, unknown PDK, deck or suite, failed report write), ``3``
when the report is incomplete: a rule was not checked because the memory it needed was
not there (see below), whatever else the report holds — a CI that reads ``2`` as "fix
the layout" must not read a missing rule as one. A violation the PDK waives is reported
but does not fail the run: a layout whose only findings are waived exits with ``0``. The
run's last line says the same in words — ``Status: PASS``, ``Status: FAIL (12
violation(s))``, ``Status: INCOMPLETE (1 rule(s) not checked for memory)`` — for whoever
reads the terminal and not the exit code.

``list-decks`` / ``list-suites``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Take only ``-p, --process``.

``show-deck``
^^^^^^^^^^^^^^

Takes ``-p, --process`` and ``-d, --deck`` (the deck to print).
