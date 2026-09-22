.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

max_density
===========

The combined density of one or more layers must be at most ``value`` (%): over the
whole chip, in every window of a grid laid over it, or in each large connected region
of a base layer, as ``scope`` says.


Semantics
---------

The dual of :doc:`min_density`, with the same scopes, denominators and ``boundary``
convention: the listed layers' merged coverage is summed and divided by the die - the
``boundary`` layer's own polygons, or the chip's bounding box where no boundary is
named - by the die area within each window, or by each wide base region's filled area. A ``min_density``/``max_density`` pair on the same layers, scope
and boundary forms the usual "keep fill density within [floor, ceiling]" rule pair, as
one chip-wide number or window by window.


Layers
------

As :doc:`min_density`: one or more layers summed for the numerator, or base and feature
layer with ``scope: region``.


Parameters
----------

As :doc:`min_density`: ``scope`` (``chip``, ``window``, ``region``), ``window``,
``min_size`` and ``layer_params: boundary``.


Violation markers
------------------

As :doc:`min_density`: one chip-wide marker, one edge marker per offending window, or
one point marker per region over the ceiling.


KLayout equivalent
------------------

``layer.without_density(value, nil, tile_boundary)`` for the chip and
``layer.without_density(value, nil, tile_size, tile_boundary)`` per window.


Examples
--------

.. code-block:: yaml

    - id: TM2.d
      check: max_density
      layers: [TopMetal2, TopMetal2.filler, TopMetal2.mask]
      value: 70.00
      layer_params:
        boundary: EdgeSeal.boundary
    - id: M4Fil.k
      check: max_density
      layers: [Metal4, Metal4.filler, Metal4.mask]
      value: 75.00
      params:
        scope: window
        window: 800.0
      layer_params:
        boundary: EdgeSeal.boundary
