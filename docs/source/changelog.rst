.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Changelog
=========

``gdscheck`` is pre-1.0 and under active development. Each release lists its
user-visible changes: new checks, new PDKs, breaking deck/CLI changes and notable
correctness fixes. The commit history is the full record of what changed and why.

Unreleased
----------

* IHP SG13G2 and SG13CMOS5L: an ``sram`` deck that checks the rules ``main`` skips under
  the SRAM marker, at the values of IHP's own SRAM bit cells.

v0.2.0 (2026-10-05)
-------------------

First beta release.

PDKs
~~~~

* New: GlobalFoundries GF180MCU, variants A and D (``gf180mcuA``, ``gf180mcuD``).
* New: ICsprout 55nm (``icsprout55``), following the foundry's Calibre runset.
* IHP SG13G2 and SG13CMOS5L: every deck hardened rule by rule against IHP's
  KLayout deck, and IHP's recommended (``*R``) rules added as decks and a suite of
  their own.
* PDK-level cell waivers: a waived violation is still reported, marked as waived,
  but does not fail the run.

Command line
~~~~~~~~~~~~

* The exit status says the outcome: ``0`` pass, ``2`` violations, ``3`` incomplete
  (rules skipped for memory), ``1`` an error. A ``Status:`` line says the same.
* ``--memory SIZE`` (``GDSCHECK_MEMORY``) caps a run. Without it, the run plans within
  its cgroup's or the machine's memory, and says which rules it could not fit rather
  than being killed.
* ``gdscheck stats`` prints the shapes per layer and the memory plan of a run, and
  stops.
* ``--tile UM`` (``GDSCHECK_TILE_UM``) sets the merge cache's tile size.
* ``--pdk-path DIR`` (``GDSCHECK_PDK_PATH``) searches directories of PDKs before the
  embedded ones; ``gdscheck list-processes`` lists them all.

Breaking changes to PDKs and decks
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Derived layers are sentences: ``virtual_layers`` maps a name to its derivation
  (``ngate: nactive and tgate``), and ``edge_layers`` is folded into it. The list form
  with ``op``, ``mode`` and ``layers`` is no longer read.
* ``max_width`` measures the narrowest span by default.

Checks
~~~~~~

* New: ``count``, ``grow_round`` (a grow whose reach is round), density windows a
  ``step`` apart and over several layers, ``max_space`` with ``scope`` and ``within``,
  and abutment and ``over`` readings for space and enclosure.
* Every result is independent of the merge cache's tile size; the test suite runs at
  7 µm as well as the default 20 µm.
* Many correctness fixes, in particular around 45° geometry, enclosures read at
  corners and across tile lines, and nets joined by connectors.

Performance
~~~~~~~~~~~

* Large designs run in far less memory: a compact flattened layout, shared tiles,
  nets freed after the net-aware rules, and a merge cache sized to what is left.
