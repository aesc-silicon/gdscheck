.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_enclosure
=============

Every shape on the enclosed layer (``layers[1]``) must sit inside an enclosing region
(``layers[0]``) with at least ``value`` µm of margin on **all** sides.


Semantics
---------

For each enclosed shape, the engine finds the enclosing region(s) that fully contain it
and measures the margin as a facing-edge-pair scan: each inner edge is paired with
parallel outer edges that have positive projected overlap onto it (KLayout's
``projection`` metric, not a raw closest-point distance), and the worst (smallest) margin
among those pairs is the shape's measured enclosure. A shape not contained by any
enclosing region is itself a violation ("not enclosed at all") unless ``interacting_only``
says otherwise.

An inner edge that lies exactly on the enclosing contour — a *coincident* segment, offset
~0 — is geometrically ambiguous: it's either a genuinely flush 0-margin violation, or an
artifact of the enclosed layer having been clipped to the enclosing one by an
``intersection`` virtual layer (in which case the "0 margin" isn't a real design fact, just
where the boolean cut it off). The geometry can't tell these apart, and neither can
KLayout — its rules pick per-flag. Two params mirror that choice:

- ``skip_coincident`` drops just the coincident *edge pairs*, still measuring the
  region's other, real margins. Used when a region is deliberately clipped to its
  enclosing layer on one side (e.g. a seal-ring conductor intersected with ``EdgeSeal``)
  but must still be checked on the sides that aren't.
- ``skip_clipped`` is the stronger, region-level form: if *any* edge of a region is
  coincident with the enclosing contour, the whole region is skipped, not just that edge
  pair. This encodes a "surrounded entirely by" semantics — a shape that reaches the
  enclosing boundary at all is out of scope for this rule (typically because crossing the
  boundary makes it a different rule's concern, e.g. IHP's NW.e explicitly title-scopes
  itself to ties *surrounded entirely by* NWell; one that crosses the edge is NW.d
  territory instead).

Neither flag is on by default: an untouched coincident edge is treated as a real 0-margin
violation (e.g. Cnt.c, where a contact genuinely straddling the edge of its active area is
exactly the defect being caught).


Layers
------

Two layers, positional:

1. ``layers[0]`` — the enclosing region.
2. ``layers[1]`` — the enclosed shapes being measured.


Parameters
----------

``interacting_only``
   Off by default (every enclosed shape must be fully inside some enclosing region, or
   it's a violation). When set, a shape that overlaps no enclosing region at all is out
   of scope for this rule entirely (skipped, not flagged) — mirrors KLayout's
   ``enclosed`` only checking shapes that actually interact with the enclosing layer (a
   via nowhere near a MIM cap isn't "a MIM via" to begin with). A shape that *partially*
   overlaps an enclosing region is still measured, using only the facing pairs on its
   contained side.

``skip_coincident``
   Off by default. Ignore inner/outer edge pairs that are coincident (flush, ~0 offset)
   — see Semantics.

``skip_clipped``
   Off by default. Skip the whole enclosed region if any of its edges is coincident with
   the enclosing contour — see Semantics. Also halo-robust: whether an edge is clipped is
   a purely local property of the region, unlike its remaining margins, whose owning tile
   can shift with per-suite halo differences.


Violation markers
------------------

- Fully unenclosed shape (no ``interacting_only``): one point marker at the shape's
  centroid.
- Contained (or, under ``interacting_only``, partially overlapping) shape whose measured
  margin is below ``value``: one edge marker along the facing outer wall responsible for
  the worst margin.


KLayout equivalent
------------------

``inner.enclosed(outer, value, metric: RBA::Region::projection, ...)`` — ``skip_coincident``
mirrors ``consider_intersecting_edges: false`` / ``without_distance(0)``; ``skip_clipped``
has no single built-in KLayout flag and is a region-level generalization of the same idea.


Example
-------

.. code-block:: yaml

    - id: Cnt.c
      check: min_enclosure
      layers: [Activ, ContOnActivNoSRAM]
      value: 0.07

.. code-block:: yaml

    # NW.e: only ties fully surrounded by NWell are in scope — one crossing the
    # boundary is NW.d's concern instead.
    - id: NW.e
      check: min_enclosure
      layers: [NWell, NActivInNWellNoTGO]
      value: 0.24
      params:
        skip_clipped: 1

.. code-block:: yaml

    # Seal.d: SealActiv is itself an `Activ AND EdgeSeal` intersection, so its own
    # inward edge is a clip artifact, not a real margin — skip_coincident drops just
    # that edge while still checking the genuine cross-frame margins.
    - id: Seal.d
      check: min_enclosure
      layers: [SealActiv, SealCont]
      value: 1.30
      params:
        skip_coincident: 1
        interacting_only: 1
