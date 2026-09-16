.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

forbidden
=========

What a rule forbids outright. An operation over the rule's layers has to come out
empty, and every region it leaves is a violation. Without an ``op`` the layers themselves
are forbidden: a derived layer whose whole purpose is to compute the error set, or a
layer the process simply doesn't support in this flow. With one, the residual of a
boolean or a selection over the listed layers is.


Semantics
---------

``op`` chooses what is read:

``(none)``
   Every connected region of every listed layer. Used for marker rules whose condition
   is "this geometry must not exist", typically a derived layer such as antenna
   ``Ant.i``'s ``pactiv_con ∩ Recog.diode − Recog.esd − (NWell ∪ PWell.block)``, the set
   of p-diodes sitting outside any well; and for device layers a flow doesn't support at
   all. An *edge* layer is read the same way, one violation per segment.
``uncovered``
   The part of ``layers[0]`` that the union of the remaining layers leaves bare:
   ``target − (cover₁ ∪ cover₂ ∪ …)``. Unlike :doc:`min_enclosure`, this handles
   "covered by A *or* B" directly and flags a target that misses the cover entirely, not
   just one short on margin. With ``scope: polygon`` the whole target polygon is reported
   whenever any part of it lies outside the covers, once, instead of its bare parts.
``overlap``
   The geometric intersection of all listed layers: "A on B is not allowed".
``apart``
   The regions of ``layers[0]`` that touch none of the remaining layers: "every MIM cap
   needs its via". A shared edge counts as touching. With ``rows`` and ``cols`` a region
   is apart unless it holds an array of the one partner layer at least that large, at
   one location: the rows and columns are only an array if every crossing holds a
   partner, so four vias in a line or bent into an L do not pass for a 2×2, and
   ``value`` is how far apart two rows may be and still be one location (GF180
   ``MT30.8``: a via is small against thick top metal, one of them is not a
   connection). Vias are lined up into rows and columns by their markers, with a tenth
   of ``value`` as the tolerance for the drift within a row.
``touching``
   The regions of ``layers[0]`` that touch any of the remaining layers, a shared edge
   included: "marker X must not touch Y".
``beyond``
   Every shape on every *other* layer of the layout past the outer edge of
   ``layers[0]``, typically the seal ring. The boundary layer is merged, its largest
   region taken as the ring, and the bounding box of that region's outer contour is the
   edge nothing may cross. A shape with a vertex outside is flagged once, at that vertex.
   Layers listed in the rule's ``ignore`` are skipped.

Every reading but ``beyond`` is evaluated as a derived layer on the tiled merge cache
(:doc:`../virtual-ops`) and read as stitched regions, so a region spanning several tiles
is reported once and no layer is unioned globally. ``beyond`` reads the raw layout, since
it looks at every layer there is; it is the one check that needs the whole layout
flattened, and a deck using it cannot point it at a lazily built derived layer.

An exemption by label: a rule with a ``text`` and a ``label`` layer param drops every
residual region carrying a text on that layer which matches ``text`` (exact, or a prefix
when it ends in ``*``). A marked structure is the designer's word that the region is
meant.


Layers
------

``(none)``, ``beyond``
   One or more layers, all forbidden; for ``beyond`` the one boundary layer.
``uncovered``, ``apart``, ``touching``
   ``layers[0]`` is the target, ``layers[1..]`` the covers or partners, read as their
   union.
``overlap``
   Two or more layers, whose common intersection must be empty.


Parameters
----------

``op``
   ``uncovered``, ``overlap``, ``apart``, ``touching`` or ``beyond``; absent for the
   layers themselves.
``scope``
   ``part`` (default) or ``polygon``; with ``op: uncovered`` only.
``rows`` / ``cols``
   With ``op: apart`` only: the array of the partner a region must hold, either one
   defaulting to ``2``; ``value`` is then the reach between two rows of it in µm.
``text`` with ``layer_params: label``
   The exemption label and the text layer it is drawn on.

``value`` is unused otherwise (set it to ``0.0`` by convention).


Violation markers
------------------

One point violation per residual region, at a representative point inside it; one edge
violation per segment of a forbidden edge layer; for ``beyond``, one point per offending
shape at its first vertex outside, tagged with the raw GDS layer and datatype it came
from.


KLayout equivalent
------------------

``layer.output(...)`` on an already derived layer; ``target.not(c1.join(c2, …))``;
``target.not_inside(c1.join(c2, …))`` for ``scope: polygon``; ``a.and(b, …)``;
``target.not_interacting(p1.join(p2, …))``; ``target.interacting(p1.join(p2, …))``; and
every drawn shape outside the ring's extent, filtered by an ignore list, for ``beyond``.
The exemption is ``.not_interacting(candidates.interacting_with_text(...))``.


Examples
--------

.. code-block:: yaml

    - id: npnG2.b
      check: forbidden
      layers: [NpnTieNoTrans]
      value: 0.0
    - id: Cnt.g
      check: forbidden
      layers: [ContSquare, Activ, GatPoly]
      value: 0.0
      params:
        op: uncovered
    - id: Cnt.j
      check: forbidden
      layers: [ContOnGatPoly, Activ]
      value: 0.0
      params:
        op: overlap
    - id: MIM.h
      check: forbidden
      layers: [MIM, TopVia1, Vmim]
      value: 0.0
      params:
        op: apart
    - id: MT30.8
      check: forbidden
      layers: [mt308_x, mt308_via]
      value: 2.0
      params:
        op: apart
        rows: 2
        cols: 2
    - id: Seal.l
      check: forbidden
      layers: [EdgeSeal]
      value: 0.0
      params:
        op: beyond
      ignore:
        - Passiv
        - Pad
