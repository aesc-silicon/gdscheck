.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

min_overlap
===========

Minimum overlap between two layers, KLayout's ``overlap`` check: where two regions share
area, how deeply they penetrate each other. A salicide block that must cover the COMP it
blocks by 0.22 µm has to reach that far past the COMP's edge, not merely touch it.


Semantics
---------

This is :doc:`min_space` measured the other way round. Both scan the same facing edge
pairs — two walls whose outward normals oppose — and differ only in which side of them the
rule is about: the empty ground outside, or the material of both inside. Each pair of
regions that share area is read for its shallowest facing overlap that is genuinely
material of both (a pair reaching across a notch in one of them is not), and an overlap
under ``value`` is a violation, once per pair. The measurement is exact on the grid, as
:doc:`min_space`'s is.

A region wholly inside another has no walls facing across it and no penetration to read;
KLayout reports nothing there either.


Layers
------

Two layers, positional: every region of ``layers[0]`` against every region of
``layers[1]`` it shares area with.


Parameters
----------

None beyond ``layers`` and ``value`` (µm, the minimum required overlap).


Violation markers
-----------------

One edge marker per violating pair, drawn across the overlap from the wall of one region
to the wall of the other.


KLayout equivalent
------------------

``a.overlap(b, value)``.


Example
-------

.. code-block:: yaml

    - id: SB.11
      check: min_overlap
      layers: [sab_out_otp, comp_sab]
      value: 0.22
