.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

Glossary
========

.. glossary::

   Deck
      A YAML file of rules for one PDK subsystem (e.g. ``metal2.yml``). See
      :doc:`pdk-authoring`.

   Suite
      A named, curated selection of rules imported from one or more decks, optionally
      restricted to a whitelist of rule ids per deck. Selected with ``--suite`` (mutually
      exclusive with ``--deck``). See :doc:`pdk-authoring`.

   Rule
      One entry in a deck: an ``id``, a ``check`` type, one or more ``layers``, a
      ``value``, and optional ``params``/``text``/``ignore``. See :doc:`pdk-authoring`.

   Check
      The algorithm a rule's ``check:`` field selects (e.g. ``min_space``,
      ``min_density``) — see :doc:`checks/index` for the full reference.

   Layer
      A named alias for a GDS ``(gds_layer, gds_datatype)`` pair, declared in a PDK's
      layer table. Rules and virtual-layer definitions reference layers by name.

   Virtual layer
      A derived layer computed from drawn (or other virtual) layers via a boolean,
      selection or morphological operator, declared in ``pdk.yml``. See
      :doc:`virtual-ops`.

   Eager (virtual layer)
      The default virtual-layer evaluation mode: computed once, up front, as ordinary
      boundaries. See :doc:`virtual-ops`.

   Lazy (virtual layer)
      The ``mode: lazy`` virtual-layer evaluation mode: registered with the tiled merge
      cache and built per tile on first use. See :doc:`virtual-ops` and
      :doc:`architecture`.

   FlatLayout
      The whole requested top cell's geometry (boundaries and text labels), with every
      ``StructRef``/``ArrayRef`` resolved to top-cell coordinates by composing affine
      transforms down the hierarchy. See :doc:`architecture`.

   Tile
      A fixed-size (20 µm) square region of the chip; the unit the merge cache merges,
      composes and stitches independently, in parallel. See :doc:`architecture`.

   Halo
      The margin of geometry around a tile's core that its local merge sees, so a shape
      straddling the tile boundary is computed correctly. Sized per layer, from the
      largest distance any rule measures on it. See :doc:`architecture`.

   Core
      The tile's own (non-halo) rectangle — the region whose merged output actually
      belongs to that tile, as opposed to geometry only visible for halo purposes. See
      :doc:`architecture`.

   Region stitching
      Reconstructing a layer's whole connected regions (area, marker point, or a
      predicate) from independently-computed tile-local pieces, by joining pieces whose
      shared tile-core edge shows continuous coverage. See :doc:`architecture`.

   Net-aware check
      A check that needs electrical connectivity (which shapes are the same net), e.g.
      the antenna-ratio family. Requires the PDK's ``connectivity:`` graph and runs unless
      ``--no-connectivity`` is passed. See :doc:`checks/antenna_ratio`.

   Connectivity graph
      A PDK's declared list of connector (via/contact) layers and the conductor layers
      each one electrically bridges, used to extract nets from geometry alone. See
      :doc:`pdk-authoring`.

   Marker
      One reported violation's geometry in a ``.lyrdb`` report: an edge, a point, or none
      (a global/whole-chip check). See :doc:`reports`.

   lyrdb
      KLayout's XML report-database format, written by ``--report`` and loadable in
      KLayout's Marker Browser. See :doc:`reports`.
