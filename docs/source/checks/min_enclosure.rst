.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_enclosure
=============

Every shape on the enclosed layer (``layers[1]``) must sit inside an enclosing region
(``layers[0]``) with at least ``value`` µm of margin — on all sides, or on the sides
``sides`` names: at least one (a wire's endcap round a via), the sides bordering a short
one, or the side facing a narrow track's tip.


Semantics
---------

For each enclosed shape, the engine finds the enclosing region(s) that fully contain it
and measures the margin as a facing-edge-pair scan: each inner edge is paired with
parallel outer edges that have positive projected overlap onto it (KLayout's
``projection`` metric, not a raw closest-point distance), and the worst (smallest) margin
among those pairs is the shape's measured enclosure. A shape not contained by any
enclosing region is itself a violation ("not enclosed at all") unless ``interacting_only``
says otherwise.

The measurement is exact on the grid: the coordinates are integers, so a margin between
parallel walls is a ratio of two integers and one read at a chamfer under the
``euclidian`` metric is compared squared as one, against ``value`` rounded up to whole
DBU once. Whether a vertex is inside, on or off the enclosing contour is a matter of
signs, never of a tolerance. The one reading that is not exact is the per-side margin
``sides: adjacent`` takes off a wall at another angle than the side, found by
interpolation.

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

``sides``
   Optional. Which sides of the enclosed shape have to make the margin. All four read
   the same per-side margins and differ only in the verdict.

   ``all`` (the default)
      Every side: the worst side must reach ``value``.

   ``any``
      At least one side: the *best* side must reach ``value`` — the wire endcap. A via
      at a metal corner is an endcap on one side, with the ordinary rule at a much
      smaller value covering the rest (IHP ``V1.c`` at 0.01 µm alongside ``V1.c1`` at
      0.05 µm with ``sides: any``: mostly flush is fine, but one side needs real
      margin). The margin is the enclosing region's bounding-box margin on each side,
      deliberately: an edge-to-contour distance is corner-limited and would understate
      a long endcap run.

   ``adjacent``
      A side enclosed by less than ``trigger`` is allowed only if the sides bordering it
      reach ``value`` (GF180 ``S.CO.6_ii``: a contact may sit flush on one side when the
      two sides next to it have the full margin).

   ``line_end``
      Only the side facing a *line end* of the enclosing layer — the cap across the tip
      of a track narrower than ``max_width`` that runs for at least ``min_length``
      (GF180 ``CO.6a``). This is the one mode whose condition comes from the enclosing
      layer's own shape: a narrow line's tip pulls back during processing, so metal that
      merely reaches the via on paper may not reach it on silicon. The sidewalls are the
      ordinary rule's business.

``metric``
   Optional. ``projection`` (the default) pairs each inner edge with the parallel outer
   edges that project onto it; ``euclidian`` reads the closest approach, which is
   KLayout's own default. The two agree on orthogonal geometry and part company at any
   corner that is not square.

``trigger``
   With ``sides: adjacent``: the margin below which a side starts asking something of
   the sides bordering it.

``max_width`` / ``min_length``
   With ``sides: line_end``: what counts as a narrow track, and how far it must run
   before it is a line rather than a notch.

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
  the worst margin. Under ``sides: any`` the marker sits on the shape's first contour
  edge, since the bounding-box reduction names no single wall.


KLayout equivalent
------------------

``inner.enclosed(outer, value, metric: RBA::Region::projection, ...)`` — ``skip_coincident``
mirrors ``consider_intersecting_edges: false`` / ``without_distance(0)``; ``skip_clipped``
has no single built-in KLayout flag and is a region-level generalization of the same idea.
Neither ``sides: any`` nor ``adjacent`` is a single operator; ``line_end`` is the
``enclosed`` of the via against the enclosing layer's line-end edges alone.


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

.. code-block:: yaml

    # V1.c allows the via to be nearly flush with Metal1 on most sides; V1.c1 requires a
    # real endcap on at least one side.
    - id: V1.c
      check: min_enclosure
      layers: [Metal1, Via1NoSealring]
      value: 0.01
    - id: V1.c1
      check: min_enclosure
      layers: [Metal1, Via1NoSealring]
      value: 0.05
      params:
        sides: any
