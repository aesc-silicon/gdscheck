.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

ASAP7
=====


Overview
--------

``asap7`` is the 7 nm predictive FinFET PDK from Arizona State University,
release 1p7. The Calibre deck is not public, so the rules follow the design rule
manual (``asap7_drm_201207a.pdf``), with the `KLayout port
<https://github.com/laurentc2/ASAP7_for_KLayout>`_ read alongside it. Layouts are read
at the real 7 nm scale on a 0.25 nm grid, as the standard-cell libraries ship; a
layout drawn 4× up for Calibre must be scaled down first.

The decks are per layer (``well``, ``fin``, ``gate`` … ``m9``, ``v9``), plus
``geometry`` for non-orthogonal shapes. The suites are ``main`` (every deck), ``feol``
(through V0, what a standard cell holds) and ``beol`` (M1 to the pad).


One-way layers
--------------

Most layers route one way, so the manual gives them one width and space across the
track and another along it. These rules read ``facing: x`` (vertical walls, a
horizontal span) or ``facing: y``; a corner-to-corner space (``M1.S.6``,
``LIG.LISD.S.7``) reads ``facing: none``. A via "enclosed on at least two opposite
sides" is ``sides: opposite``, and "the same width as M2" is a ``max_enclosure`` of
nothing on ``sides: opposite``.

The M1–M3, LISD, LIG and M8/M9 spacings depend on the lengths of the two facing edges
(side to side, tip to side, tip to tip), so they read edge layers sorted by length
(``virtual_layers`` in ``pdk.yml``).


What is not checked
-------------------

Each deck's header lists the rules of its section it does not check, and why: mainly
the routing grids and tracks, the width-multiple rules, the net-aware ``ACTIVE.S.2A``,
the latch-up reach ``ACTIVE.LUP.1``, and the via spacings that depend on an end-cap.
``FIN.S.1``, an exact 27 nm pitch, is checked only as a space of at least 20 nm.


Readings
--------

A few rules are read differently from the manual's literal wording, because the
shipped standard-cell libraries (verified with the Calibre deck) contradict that
wording:

* ``V0.LIG.EN.4`` is worded as the LIG's enclosure by the V0. The libraries draw both
  that case (a V0 over a 16 nm LIG rail, 1 nm past it on each side) and a V0 flush with
  the end of a wider LIG. The rule reads the LIG walls inside the via and skips the
  flush one.
* ``SDT.ACTIVE.AUX.2``, "SDT horizontal edges must coincide with ACTIVE horizontal
  edges", is read as "an SDT does not end inside its ACTIVE". Where an ACTIVE steps,
  the libraries run the SDT on past it.
* The select, VT and well minimum widths fire on the 54 nm half-width filler and tap
  cells when these are checked on their own. Placed between other cells, they are part
  of a wider shape.

Tests
-----

``tests/asap7.rs`` runs the full suite over the whole ASAP7 LVT standard-cell library,
vendored under ``tests/data/asap7/static/`` (BSD-3-Clause). It asserts the reviewed
regression baseline, including isolated filler-cell context effects. Synthetic cases also 
check the SRAM SDT–ACTIVE overlap at 17 nm and V1's asymmetric 5/2 nm enclosure on 
the same opposite pair, including rotated pairs and shapes crossing tile boundaries.

The generated fixtures in ``tests/data/asap7/generated/`` also run in ordinary
Rust CI. They exercise SRAM marker classification and both well-rule thresholds,
SDT overlap, asymmetric V1 enclosure, M4 width, length-dependent M8 width, and
three routed upper-metal nets with a deliberately defective companion.

``hardening/oracle-asap7.sh --pdk /path/to/ASAP7_for_KLayout`` compares the library
and these fixtures against the pinned KLayout port at 20 and 7 micrometre tiles.
It saves both checker reports, normalized marker locations, rule counts and
manual-derived target verdicts. See ``hardening/asap7/README.md`` for setup and
``hardening/reports/asap7/parity.md`` for the initial findings. Agreement on these
targeted cases is not a whole-PDK parity measurement; missing checks and known
differences remain explicitly documented.
