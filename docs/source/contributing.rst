.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Contributing
============


Development setup
-------------------

Requires a recent Rust toolchain (edition 2024). The ``justfile`` collects the common
developer tasks:

.. code-block:: bash

   just build          # cargo build --release
   just test           # cargo test --release
   just clippy         # cargo clippy --release -- -D warnings
   just gen-testdata   # regenerate the IHP SG13G2 test fixtures
   just check          # clippy + test + cargo fmt --check — the pre-commit gate

Run ``just check`` before opening a change — it's the same gate CI runs.


Adding a rule to a deck
--------------------------

Add an entry to the relevant ``pdks/<process>/decks/<name>.yml`` (see
:doc:`pdk-authoring` for the full rule format), then regenerate and extend the test
fixtures (see *Test fixtures and the generator* below) rather than hand-crafting a GDS —
the generator's pattern helpers already cover the common shapes (min/max width, spacing,
notch, enclosure, density) parametrically. Confirm the new rule with ``gdscheck
show-deck`` before running it on anything real.


Adding a new check type
--------------------------

A check is a module under ``src/checks/`` exposing a ``run`` function, dispatched by
name from ``checks::run_rule`` (``src/checks/mod.rs``) on the rule's ``check:`` string.
Signature depends on what the check needs:

* Pure geometry, no merge cache: ``fn run(rule, layout: &FlatLayout, dbu_to_um: f64) -> Vec<Violation>``.
* Needs the tiled merge cache: add ``merged: &mut MergedCache``.
* Needs per-run memoisation (e.g. an expensive whole-chip number several rules share):
  add ``cache: &mut Cache``.
* Net-aware: add ``conn: Option<&Connectivity>`` and list the check's name in
  ``NET_AWARE_CHECKS`` (``lib.rs``) so connectivity is built lazily when the deck needs it,
  and skipped cleanly under ``--no-connectivity``.

**Prefer the tiled merge cache over a global merge** unless every layer the check reads is
genuinely sparse (isolated vias, contacts, device markers — see
:doc:`checks/must_interact` for a deliberate, documented example of when a global merge is
safe). A check that globally unions a dense, chip-wide layer works on a small test
pattern and then runs out of memory the first time someone points it at a real SoC —
exactly the failure class :doc:`checks/forbidden_unless_labeled` was rewritten to avoid
(see :doc:`architecture`, *Region stitching* and *Lazy virtual layers*, for the primitives
available: ``MergedCache::regions``/``stitch_labeled`` for whole-region area/marker/
predicate aggregation, ``register_virtual`` for a derivation chain expressed as tiled
virtual layers).

Register the new check name in ``checks::run_rule``'s match, write its
:doc:`reference page <checks/index>` (the six-section template every existing page
follows — copy the closest existing check's structure), and add it to
``docs/source/checks/index.rst``'s toctree.


Test fixtures and the generator
-----------------------------------

``gen/`` is a separate binary (``gen-testdata``) that writes synthetic pass/fail GDS
fixtures into ``tests/data/<pdk>/<deck>/`` for every rule, using parametric pattern
helpers in ``gen/helpers.rs`` (``min_width_pattern``, ``space_pattern``,
``notch_pattern``, ``density_pattern``, ``enclosure_pattern``, …) — read the existing
helper doc comments before writing a new one; most new fixture needs are a variation on
an existing pattern's parameters, not a genuinely new shape.

The convention (see any existing ``gen/ihp_sg13g2/*.rs`` generator function): one function
per rule id, writing a small number of shapes designed to sit exactly on the pass/fail
boundary (a clean case at exactly the limit, a failing case a half-grid-unit past it) —
not realistic-looking layout, the smallest shape that unambiguously exercises the rule.
``tests/ihp-sg13g2.rs`` then asserts the *exact set* of rule ids a fixture triggers (via
an ``ignore`` list for expected incidental violations from unrelated rules on the same
shapes), so both false negatives and false positives are caught.

When you add or change a generator function, run ``just gen-testdata`` to regenerate the
affected fixtures, then ``just test`` — always verify the *actual* violation list against
the real binary (``gdscheck run --deck <name> --input <fixture> --verbose``) before
encoding an assertion, rather than assuming a fixture produces what you intended.


Validating against the reference KLayout deck
--------------------------------------------------

For a rule with a KLayout reference implementation (IHP publishes ``.lydrc`` scripts for
SG13G2), cross-check a real design with both engines before trusting a new/changed check.
Reading the reference script directly — not just its rule-name description — matters:
the same DRC-DSL operator (e.g. ``Region#space``) can mean something subtly different
than the obvious gdscheck equivalent (see :doc:`checks/min_notch`'s KLayout-equivalent
section for a worked example, where KLayout's single ``space`` operator folds together
what gdscheck reports as two separate checks).

When counts genuinely differ, verify by pulling the real geometry at the reported
coordinates (a small KLayout Ruby/Python script querying the actual GDS) rather than
trusting either tool's report blindly — several real discrepancies this project has hit
turned out to be one tool reporting a genuine duplicate (KLayout's two-pass angle-metric
scan reporting the same notch twice) rather than the other tool missing something.


Parity testing between PDKs
-------------------------------

A derived PDK (``extends:``, see :doc:`pdk-authoring`) inherits its base's layer and
virtual-layer tables but declares its own decks — there's no automatic guarantee a rule
that exists in both processes stays behaviourally identical as either evolves. When
changing a check or a shared virtual layer, re-run the affected deck against both PDKs'
test fixtures (``cargo test`` already covers this if fixtures exist for both; see
``tests/ihp-sg13cmos5l.rs`` alongside ``tests/ihp-sg13g2.rs``) rather than assuming a fix
verified against one PDK holds for the other.


Coding conventions
--------------------

* Follow the message convention in ``violation.rs``'s module doc for any new
  ``Violation``: name the layer(s), state ``<measured> <cmp> <limit>`` with units, end
  with the location — consistency here is what makes report diffing and KLayout
  cross-checks tractable.
* Prefer extending an existing shared primitive (``helper.rs``'s facing-wall scan,
  region-spacing engine, or boolean-residual/extension engines; ``merge.rs``'s tiled
  virtual-layer and region-stitching machinery) over writing a new one-off algorithm —
  most new checks are a variation on an existing measurement, not a new kind of geometry
  problem.
* Comments explain *why*, not *what* — a hidden constraint, a subtle invariant, a
  workaround for a specific upstream quirk. Well-named code and the check's own reference
  page should already answer *what*.
* Run ``just check`` (clippy + tests + ``cargo fmt --check``) before considering a change
  done.
