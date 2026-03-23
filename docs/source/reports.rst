.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Reports and markers
===================


The lyrdb report format
------------------------

``--report <path>.lyrdb`` writes KLayout's XML report-database format (``report.rs``).
Every violation becomes an ``<item>``, grouped under a ``<category>`` tree keyed by rule
id: ids of the form ``Parent.child`` (e.g. ``M2.a``) become a two-level chain
(``M2`` → ``a``), and a three-level id like ``Cnt.c.Digi`` becomes a three-level chain —
KLayout resolves ``<category>`` as a dot-separated path, so the tree depth always matches
the id's dot count. Each item's category description is the check's human-readable
violation description (e.g. "Minimum space violation"), and its ``<values>`` carry the
marker geometry plus a ``text:`` field with the full violation message.

The report's ``<cells>`` section lists just the run's top cell (``gdscheck`` flattens the
whole hierarchy before checking, so all violations are reported in top-cell coordinates —
there is no per-instance cell breakdown).


Opening reports in KLayout
----------------------------

*Tools → Marker Browser → Load* (or drag the ``.lyrdb`` file onto an open layout view).
KLayout resolves each marker against the currently loaded layout by coordinates, so open
the same GDS the report was generated from. The marker browser lets you step through
violations grouped by category, matching the rule-id tree above.


Marker types (edge, point, global)
-------------------------------------

Every ``Violation`` (``violation.rs``) carries one of three geometry kinds:

* **Edge** — a line segment, ``(x1, y1)-(x2, y2)`` µm. Used by spacing/width/notch/
  enclosure/extension checks to mark the two facing walls (or the single span) that
  failed; for a spacing violation this is drawn across the gap itself, not along either
  region's boundary.
* **Point** — a single ``(x, y)`` µm location, generally a region's centroid or marker
  point. Used by area, density-region, boolean-residual and net-aware checks, where the
  violation is "this whole region," not a specific measured span.
* **None (global)** — no geometry at all. Used by whole-chip checks like
  :doc:`checks/min_density`/:doc:`checks/max_density`, where the violation is a single
  percentage over the entire chip and there is no specific location to mark.

The message convention (documented in ``violation.rs``) is consistent across every check:
name the layer(s) involved, state the failing comparison as ``<measured> <cmp> <limit>``
with units, and end with the location in the format above. The rule id itself is not part
of the message — the console printer prefixes ``[<rule-id>]``, and the lyrdb report
carries it as the item's category.


Marker granularity
--------------------

"One marker per what" varies by check family, and is documented on each check's
reference page (:doc:`checks/index`) under **Violation markers**. As a rule of thumb:

* Facing-wall checks (width, space, notch, enclosure) report **one marker per distinct
  violating pair**, deduplicated across tiles by ownership (a gap seen from two
  overlapping tiles' halos is only reported by the tile whose core contains its
  midpoint).
* Region checks (area, density-region, boolean residuals) report **one marker per
  connected region**, stitched across tile borders (see :doc:`architecture`) so a region
  spanning several tiles is never reported more than once.
* Whole-chip checks (plain density) report **at most one marker**, since there is only
  one number to compare.


Comparing against KLayout marker counts
-------------------------------------------

When cross-checking a rule against a reference KLayout DRC deck, expect the *set* of
flagged locations to match, not necessarily the exact marker count. Two differences are
expected and not bugs:

* KLayout's ``space``/``width`` operators can internally run separate 90°/180°
  angle-metric passes and report the same physical notch or gap twice, as overlapping
  edge-pairs from each pass; ``gdscheck`` reports it once, per its "one marker per
  distinct violation" convention.
* A region-level check (e.g. :doc:`checks/min_enclosure`) reports one marker per
  violating *edge pair*, while a different KLayout formulation might report one per
  polygon — read each check's **Violation markers** section for the exact convention
  before diffing counts mechanically.

When in doubt, verify by inspecting the actual reported coordinates against the layout in
KLayout, not just the raw count.
