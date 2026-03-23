.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Virtual layer operations
========================

Derived layers computed from drawn layers, declared in ``pdk.yml`` (``virtual_layers:``) and referenced by rules like any drawn layer.


Global vs. lazy evaluation
----------------------------

Each entry in ``virtual_layers:`` takes an optional ``mode``:

.. code-block:: yaml

   virtual_layers:
     - name: Pad
       op: union
       layers: [Passiv, Passiv.sbump, Passiv.pillar, dfpad]
     - name: TopMetal2NotIND
       op: difference
       mode: lazy
       layers: [TopMetal2, IND]

* **Eager (default, no ``mode:``)** — computed once, up front, right after the top cell
  is flattened (``pdk.rs`` → ``compute_virtual_layers``), as an ordinary set of boundaries
  inserted into the flattened layout under the virtual layer's synthetic ``(gds_layer,
  gds_datatype)``. Every downstream check reads it exactly like a drawn layer, with no
  further cost. Only five ops are supported eagerly: ``union``, ``intersection``
  (``and``), ``difference`` (``not``), ``inside``, and ``close``.
* **Lazy (``mode: lazy``)** — registered with the tiled :doc:`merge cache
  <architecture>` instead: a synthetic key built per tile, on first use, from its
  (recursively resolved) source layers' *tiles*. Costs nothing until a rule actually needs
  it, and its memory profile is the same tile+halo bound as a drawn layer — required for
  any derivation chain that touches a dense, chip-wide layer (``Activ``, ``GatPoly``,
  metals), since eager evaluation unions the *whole chip* in one shot and does not scale
  to those. All fifteen ops below are available lazily; only the five above are available
  eagerly.

A rule can only reference a lazily-evaluated layer once the deck actually runs a check
against it — see :doc:`faq` if a lazy virtual layer produces no results or an unexpected
empty layer.


``union``
---------

The union of all source layers. Duplicate polygons contributed by more than one source
(e.g. two layers that happen to carry the same pad shape) are only counted once.

*Eager and lazy.*


``intersection``
-----------------

The geometric AND of all source layers — empty wherever any source layer is empty. Used
for device-recognition layers (e.g. ``CuPillarPad = Passiv.pillar AND dfpad``). YAML
alias: ``and``.

*Eager and lazy.*


``difference``
---------------

The first source layer minus every other source layer (e.g.
``ContNoSealring = Cont NOT EdgeSeal``). YAML alias: ``not``.

*Eager and lazy.*


``interacting``
-----------------

Lazy only. Keeps whole regions of ``layers[0]`` that touch (overlap) any region of
``layers[1]``, region membership resolved by stitching each source's tile-local pieces
into whole connected regions first (see :doc:`architecture`) so a region is kept or
dropped as a unit, never split by a tile boundary. Mirrors KLayout's
``Region#interacting``.


``not_interacting``
----------------------

Lazy only. The complement of ``interacting``: keeps whole regions of ``layers[0]`` that
touch *no* region of ``layers[1]``. Mirrors KLayout's
``interacting(..., inverted: true)``.


``covering``
------------

Lazy only. Keeps whole regions of ``layers[0]`` that fully contain a region of
``layers[1]``. Computed the same way as ``interacting`` — every real use in the bundled
PDKs has ``layers[1] ⊆ layers[0]`` already, where "covers" and "interacts" coincide.


``grow``
--------

Lazy only. One-directional morphological dilation by ``radius`` µm (the def's ``radius``
field). Only grows outward — pair with ``not_interacting``/``interacting`` downstream if
you need the grown reach purely as a proximity test.


``close``
---------

Morphological closing (dilate then erode) by ``radius`` µm: regions whose gap is under
``2 × radius`` merge into one. Used for same-net merging (e.g. NWell tie regions closer
together than the rule distance). A closed pair of regions that stays separate keeps its
*true* outer edges, so a downstream ``min_space`` still measures the real gap between the
(now-merged) regions correctly.

*Eager and lazy* — the only morphological op available eagerly.


``open``
--------

Lazy only. Morphological opening (erode then dilate) by ``radius`` µm: removes every part
of a region narrower than ``2 × radius``. A region that vanishes entirely had no spot
wider than that anywhere — pairing ``open`` with a downstream ``not_interacting`` against
the *opened* layer implements "minimum width somewhere" style rules.


``holes``
---------

Lazy only. Unary: the hole area of each source region, as filled polygons (KLayout's
``.holes``) — e.g. the interior of a substrate-tie ring. Declare the def's ``radius`` as
the maximum expected ring extent: a hole only materialises correctly in a tile whose
bucket assembles the *whole* ring, so this only works for device-scale rings, not a
chip-perimeter ring (whose "hole" is the entire die and is not tile-local).


``with_holes``
--------------

Lazy only. Unary shape filter: keeps source regions that contain at least one hole
(KLayout's ``.with_holes``) — e.g. a ring-shaped NWell encircling an iso-PWell. Same
tile-locality limit as ``holes``.


``with_text``
-------------

Lazy only. Keeps whole regions of ``layers[0]`` containing a text label on ``layers[1]``
(a TEXT layer) matching the def's ``text`` pattern — a trailing ``*`` makes it a
case-insensitive prefix match, otherwise a case-insensitive exact match. Reads the
layout's text/label records directly rather than any layer's polygon tiles. Mirrors
KLayout's ``interacting_with_text``.


``square``
----------

Lazy only. Unary shape filter: keeps regions that are (approximately) a single square.


``not_square``
--------------

Lazy only. The complement of ``square``.


``not_circle``
--------------

Lazy only. Unary shape filter: keeps regions that are not a (approximately) regular
circle — e.g. flagging disallowed bond-pad shapes.


``not_circle_or_octagon``
----------------------------

Lazy only. Like ``not_circle``, but also excludes regular octagons.


``inside``
----------

Eager only, two layers exactly: ``layers[0]`` (the target) and ``layers[1]`` (a ring).
The part of the target that lies within the area *enclosed* by the ring — the ring's own
holes are filled first (so a seal frame becomes "frame + interior"), since there is
usually no drawn layer for that interior region. Both sources must be real (drawn)
layers; ``inside`` does not support one virtual layer feeding another.
