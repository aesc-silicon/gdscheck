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

SRAM applicability
------------------

DRM section 1.2.2, convention 7 makes non-SRAM geometry the default scope of a
rule unless its wording says otherwise. This applies throughout the decks,
including ordinary LISD/LIG, via and M4–M9 checks, not only M1–M3. The applicability
inventory in ``hardening/asap7/sram-scope.tsv`` records every implemented check,
including repeated rule IDs, its scope, layer operands and the basis for that
choice. Adding or changing a check requires reviewing its inventory entry.

SRAM membership requires **positive-area overlap** with ``SRAMDRC``. Both ordinary
and explicitly SRAM-scoped checks use this partition. Edge-only and point-only
contact remain non-SRAM. This is a deliberate conservative deviation from the
manual's convention 4: the manual counts shared-edge contact as interaction, but
excludes isolated vertex contact. The general engine ``interacting`` operator also
counts vertex contact, so it is not the predicate used for SRAM membership here.

Classification keeps or drops a whole merged polygon, including its geometry
outside the marker and across tile boundaries. Edge layers are extracted after
selection, preserving original edge lengths. A marker on the remote end of a wire
can therefore exempt an ordinary width or spacing violation outside the array.
This does not exempt other layers connected to that wire through vias, and marker
locations alone cannot determine whether a violation should be exempt.

The rules use the selected subjects as follows:

* Ordinary single-layer checks read non-SRAM polygons. Ordinary spacing between
  two subjects reads a pair only when both are non-SRAM. No extra SRAM/non-SRAM
  boundary-spacing requirement is inferred. This is the literal convention-7
  interpretation; mixed-pair behavior has not been verified against Calibre.
* Via checks select the via instance, retaining full landing metal and LIG/LISD
  references. A non-SRAM via still sees metal that overlaps SRAMDRC somewhere
  else. This includes ``V0.LIG.EN.4``, whose existing implementation measures LIG
  walls inside the selected V0. Other enclosure/extension checks select the
  enclosed subject and keep the enclosing reference layer whole. Extension checks
  select FIN, ACTIVE or cut-GATE subjects; where the engine measures a channel
  fragment, selection precedes that intersection. Coverage predicates likewise
  retain their full reference geometry.
* Explicit qualifications take precedence. ACTIVE/WELL and implant enclosures
  select ACTIVE as their subject; SRAM SDT overlap and LIG/GATE overlap select SDT
  or LIG, respectively, retaining their full reference layers. The existing
  SRAM-specific limits and ACTIVE/LISD/LIG boundary-contact prohibitions remain
  active. These prohibitions test contact independently of the membership filter.
* The SRAMDRC marker's own non-orthogonal-geometry check remains active everywhere
  as a supplemental marker-integrity check. A marker is not exempted against
  itself. ``SRAMVT`` is a threshold-adjust layer, not the SRAMDRC marker; its name
  alone does not give it SRAM scope.

Rules absent from the implementation remain coverage gaps. In particular,
``ACTIVE.S.2A`` is not implemented; its explicit note requiring the rule within
SRAM must be honored if it is added. The inventory is an implementation audit,
not evidence of Calibre parity or permission to waive fabrication requirements.


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
