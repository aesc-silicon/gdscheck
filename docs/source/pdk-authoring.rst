.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

PDK authoring guide
===================

A PDK is a directory with one ``pdk.yml`` (the layer table, decks, suites, virtual
layers and connectivity graph) plus a ``decks/`` and ``suites/`` subdirectory of rule
YAML files. See :doc:`pdks/ihp-sg13g2` for a complete, working example to read
alongside this guide.


Anatomy of pdk.yml
--------------------

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
     Pad: Passiv or Passiv.sbump or Passiv.pillar or dfpad

   connectivity:
     - connector: Cont
       layers: [Metal1]

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

Deck and suite paths are relative to the PDK file. ``name``/``version`` are free text,
surfaced by ``gdscheck run`` at the top of its console output.


The layer table
-----------------

``layers:`` maps a name to a GDS ``(gds_layer, gds_datatype)`` pair. Rules, virtual-layer
definitions and the connectivity graph all reference layers by name — the mapping to GDS
numbers is resolved once, at PDK load time. A convention worth following (used throughout
the bundled PDKs): name auxiliary datatypes of the same drawing layer with a dotted
suffix, e.g. ``Metal2`` (datatype 0), ``Metal2.filler`` (22), ``Metal2.mask`` (20),
``Metal2.pin`` (2) — it reads naturally in a rule's ``layers: [...]`` list and keeps
related entries visually grouped.


Decks
-----

A deck (``decks/<name>.yml``) is a flat list of rules:

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

Each rule needs ``id``, ``check`` (one of the names in :doc:`checks/index`), ``layers``
(at least one; a second layer turns most single-layer checks into an inter-layer check),
and ``value`` (µm for widths/spaces, µm² for areas, % for densities — see the specific
check's reference page). ``params`` and ``text`` are optional, check-specific (see
*Rule parameters* below). ``ignore`` names layers whose shapes a check should skip
(e.g. excluding the passivation ring from a ``forbidden`` past the seal ring).

A rule id may repeat across multiple entries in the same deck (e.g. IHP's ``TM2.b`` is
both a ``min_space`` and a ``min_notch`` rule) — both fire under the same reported id,
since KLayout-style report categories are keyed by id, not by list position.


Suites
------

A suite (``suites/<name>.yml``) imports rules from one or more decks by name, optionally
restricted to a whitelist of ids (``rules``) and/or with a blacklist removed
(``exclude``), without duplicating rule definitions:

.. code-block:: yaml

   include:
     - deck: metal1
       rules: [M1.a, M1.b, M1.j, M1.k]   # only these ids from metal1
     - deck: cont                        # no `rules:` → the whole cont deck
     - deck: metal2
       exclude: [M2.j, M2.k]             # the whole metal2 deck *except* these

``rules`` is applied first, then ``exclude`` — so an include may combine both. An id in
either list that doesn't exist in the named deck is a load-time **error** (a suite typo
can never silently drop a check, nor silently fail to drop one). An id that matches
several entries in the deck (see *Decks* above) keeps or drops all of them together. A
suite only *selects* rules — it can never override a rule's ``value`` or ``params``,
which live solely in the deck.


Virtual layers
--------------

``virtual_layers:`` declares derived layers, each a name and the sentence that makes it
from drawn or other derived layers — ``poly_otp: poly2_drawn and otp_mk``,
``nat4_gate: poly_nat_lv grow 0.5 inside ngate``, ``channel: SourceDrain edges and
(GatPoly edges)``. A sentence reads from left to right, parentheses group, values follow
their word and bounds are ``min``/``max``; see :doc:`virtual-ops` for the words. A region
and an edge layer are told apart by the words used, so both live in this one block. Every
derived layer is assigned a synthetic GDS layer number automatically (from 30000, in the
order declared) and is referenced by rules exactly like a drawn layer. Whether a layer is
built per tile in the merge cache or materialised in the layout follows from the checks
that read it, and is not declared.


The connectivity graph
-------------------------

``connectivity:`` declares how net extraction bridges layers, for the net-aware checks
(:doc:`checks/antenna_ratio`, :doc:`checks/min_area` with ``net: connected``) — see
:doc:`architecture` for how extraction works. Each entry is a *connector* layer (a via or
contact) and the conductor layers it joins where it overlaps them:

.. code-block:: yaml

   connectivity:
     - connector: Cont
       layers: [GatPoly]
     - connector: Cont
       layers: [Activ]
     - connector: Cont
       layers: [Metal1]
     - connector: Via1
       layers: [Metal1]
     - connector: Via1
       layers: [Metal2]

A conductor layer's own connected regions don't need a graph entry — lateral routing on
one layer is already resolved by region stitching; only the vertical via/contact stack
needs declaring. If a PDK has no net-aware checks in any of its decks, ``connectivity:``
can be omitted entirely (net extraction never runs).


Deriving a process with extends
-----------------------------------

.. code-block:: yaml

   name: IHP SG13CMOS5L
   extends: ../ihp-sg13g2/pdk.yml
   decks:
     - name: cont
       path: decks/cont.yml
   suites:
     - name: main
       path: suites/main.yml

``extends`` (a path relative to this file, or a bare process name for the base beside
it — ``extends: ihp-sg13g2`` reads as ``../ihp-sg13g2/pdk.yml``) inherits the base PDK's
``layers`` and ``virtual_layers`` — this file's own entries are appended after them, and a
``virtual_layers`` entry under the same name as one in the base *replaces* it (the
child's sentence wins). Everything else — ``decks``, ``suites``, ``connectivity`` — is
never inherited; a derived process states its own deck list and connect graph explicitly,
even if it reuses most of the base's rules. One level only: the base file may not itself
``extend`` another.

This is the pattern for a process variant that shares most of a foundry's device and
recognition layers but has its own rule set (a different metal stack, different design
rules, or — as with SG13CMOS5L — a restricted set of forbidden layers).

The same pattern works outside the source tree. A PDK kept on the PDK path (see
:doc:`usage`, *Selecting the process*) can extend a bundled base by name, and any file
it references but does not carry — the base's ``pdk.yml``, a deck such as
``../ihp-sg13g2/decks/activ.yml`` — is read from the copy embedded in the binary. An
external directory therefore holds only the ``pdk.yml`` and the decks it adds or
replaces. The fallback is for files the tree lacks, so a PDK that shadows a bundled
name cannot at the same time extend the one it shadows: ``../ihp-sg13g2/pdk.yml`` from
a directory called ``ihp-sg13g2`` is the file itself.


Rule parameters
-----------------

``params:`` is a flat map from a name to a number or a word — every check-specific knob
beyond the universal ``value`` lives here: ``window: 700`` for the windowed-density
checks, ``rows: 3`` for array spacing, ``sides: adjacent`` or ``metric: square`` for an
enclosure, ``angle: bent`` for a width. YAML decides which kind a value is (``0.5`` is a
number, ``bent`` a word), and a check that asked for the other kind says so. Avoid
YAML's boolean words — ``on``, ``off``, ``yes``, ``no`` — as mode values, or quote them.
The exact keys a given check reads, with their defaults, are listed on that check's
reference page under **Parameters** — params the check doesn't recognise are silently
ignored, so a typo'd param name fails quietly rather than erroring; double-check the
reference page's exact key spelling.

``layer_params:`` names a *layer* as a parameter, e.g. ``outside: nwell`` for a gate
length. It is its own block because a layer name and a mode word look alike, and only
this block is resolved against the PDK's layers; each entry arrives in the check as
``<name>`` and ``<name>_dt``.

``text:`` is a separate, sibling field for the handful of checks that need a text/label
pattern — :doc:`checks/forbidden_unless_labeled` is the only current user.


Validating a new PDK
-----------------------

There's no PDK-specific test harness beyond what ``gdscheck`` itself gives you:

1. ``gdscheck list-decks``/``list-suites --process <path-to-pdk.yml>`` to confirm every
   deck and suite loads and parses.
2. ``gdscheck show-deck --process <path> --deck <name>`` for each deck, to eyeball every
   rule's resolved layers/value/params before running it on a real layout.
3. Run each deck (or the full ``main`` suite, if defined) against a real design and
   sanity-check the violation counts against a reference DRC engine for at least a few
   rules — see :doc:`contributing` (*Validating against the reference KLayout deck*) for
   the workflow this project itself uses to cross-check new/changed checks.
4. If the PDK is meant to ship in-tree, follow :doc:`contributing` for the test-fixture
   generator convention (a synthetic pass/fail GDS pair per rule, asserted by an
   integration test) rather than relying solely on real-design spot checks.
