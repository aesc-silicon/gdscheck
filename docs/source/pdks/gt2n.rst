.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

GT2N
====


Overview
--------

``gt2n`` is the 2 nm nanosheet PDK with backside power delivery from the Georgia
Institute of Technology (initial release). The decks follow `azadnaeemi/GT2N
<https://github.com/azadnaeemi/GT2N>`_ at ``5bc689f``. Its IC Validator runset
(``icv_runset/gt2_main.drc.rs``) is :doc:`gt3`'s with seven values changed: a 14 nm
gate on a 42 nm pitch, a 16 nm SDCON and VBPR, and an 8 nm active. It also adds a deck
for the five threshold-voltage layers (ELVT, ULVT, SVT, HVT and SRAMVT; LVT is the
default and has no layer).

The PDK ``extends: gt3``. The decks that changed live under ``pdks/gt2n/decks/``,
and the rest are GT3's files. The suites match GT3's: ``main``, ``feol`` and ``beol``.
The runset stops at M6 and checks none of the backside; ``main`` and ``beol`` go on to
RDL, and a fourth suite, ``backside``, checks BV0 to BRDL, both from the technology LEF.
The technology file supplies additional well and minimum-area constraints.

Two rules are read more closely than the runset does, as :doc:`gt3` reads them:

* GATE.2, an exact 28 nm gate space, is read as the 42 nm gate pitch it makes with
  GATE.1's 14 nm gate: every gate is centred on a multiple of 42 nm, as in all ten
  libraries, and no two are closer than 28 nm. A design drawn in another frame sets
  the rule's ``offset``. A gate missing from a row is reported too: two gates facing
  each other across x within 1 µm must be 28 nm apart. Gates further apart read as
  separate blocks, and a row's ends are never read. An empty site in a row is a
  missing gate, which ``gt2_6t_filler`` closes.
* ACT.6 requires every part of an active's end to lie in a DUMMY. The runset reports
  only an end that touches no DUMMY at all.


Manufacturing grid
------------------

The technology LEF's ``MANUFACTURINGGRID 0.0005`` requires a 0.5 nm grid. The
``offgrid`` deck checks both coordinates of the merged geometry on all 56 drawing
layers, including NWELL, the five threshold-voltage layers and the backside stack.
Pin annotations are excluded. Each off-grid vertex produces a point marker under
``FEOL.offgrid.LEF``, ``BEOL.offgrid.LEF`` or ``BACKSIDE.offgrid.LEF``.

``main`` includes all three rules; ``feol``, ``beol`` and ``backside`` include only
their own group. ``--deck offgrid`` runs just the manufacturing-grid checks. This
is separate from ``GATE.2``'s gate-centre pitch. The generated grid fixtures use a
0.25 nm database unit to preserve off-grid coordinates, and exercise every drawing
layer with passing shapes and offsets in x, y and both coordinates.


Technology-file constraints
---------------------------

The ``techfile`` deck adds constraints from the ``foundry/spacings`` section of
`techlib/gt2_techfile.tf
<https://github.com/azadnaeemi/GT2N/blob/5bc689fd70e4f2d7abf3952e17f308072bc2b0f6/techlib/gt2_techfile.tf>`_,
at the same revision as the LEF audit. Its 15 rule IDs end in ``.TF``. NWELL has a
42 nm minimum width in every direction (``NWELL.width.TF``), and a 42 nm minimum
spacing between regions and across notches (``NWELL.space.TF``). Each layer below
also has a ``<layer>.area.TF`` rule:

.. list-table::
   :header-rows: 1

   * - Layers
     - Minimum area (µm²)
     - Suite
   * - NWELL
     - 0.003024
     - feol
   * - ACT
     - 0.000420
     - feol
   * - BPR
     - 0.000100
     - feol
   * - M10, M11
     - 0.004480
     - beol
   * - M12, M13
     - 0.194400
     - beol
   * - RDL
     - 3.840000
     - beol
   * - BM1, BM2
     - 0.004480
     - backside
   * - BM3, BM4
     - 0.194400
     - backside
   * - BRDL
     - 3.840000
     - backside

Areas are measured on each merged region, subtracting holes. Overlapping or
abutting pieces contribute to the same region; separate islands each have to meet
the limit. These are area limits, not fixed minimum lengths: a wider wire may be
shorter. ``main`` includes all 15 rules, and the other suites include only their
own layers. ``--deck techfile`` runs these additional checks alone.

The three sources do not fully agree. The IC Validator runset takes precedence
where it defines a rule, followed by the LEF; the technology file fills gaps:

* The technology file's ACT width is 10 nm, versus the runset's 8 nm. Its GATE width
  and spacing are 15 and 27 nm, versus 14 and 28 nm. Keep the runset's dimensions
  and directional spacing checks.
* The technology file's GATE area of 0.006 µm² would reject every merged gate in
  the 13 nm SVT library when its cells are checked individually (areas 0.002016 to
  0.004032 µm²). It is not enabled. The runset has no gate-length check.
* M9's area is 0.002204 µm² in the technology file, versus the LEF's 0.002166 µm²
  and 38 × 57 nm ``MINSIZE``. Keep the LEF's limit.
* M0–M3's technology-file areas equal the existing width × length limits. M4–M8's
  are smaller than those limits. They add no restriction to the rectangles those
  decks require. ACT's explicit area is retained even though ordinary rectangles
  meeting its width and length already exceed it.
* The remaining width and spacing entries duplicate or are weaker than existing
  ICV/LEF values. Their directional interpretations and via-template dimensions
  remain authoritative. For example, the technology file's V3 width and spacing
  are 12 nm, versus 14 nm in the LEF; even its route-spec section uses 14 nm.

The manufacturing grid is already checked by ``offgrid``. Routing pitches and
offsets describe routing tracks and are not additional geometry checks; via
templates remain governed by the LEF reconciliation below.

All 72 cells in each of the ten standard-cell libraries, checked as separate
instances, pass these additional constraints. The routed ``gt2n/aes`` design also
passes the ``techfile`` deck. Its routing stops at M9 and BM2, so the higher layers
are covered by generated fixtures.


Above M6: the tech LEF
----------------------

The runset checks nothing from V6 up. GT2N's technology LEF (``techlib/gt2_tech.lef``,
at the revision below) states the same rules from V6 to RDL as GT3's, so ``v6`` to
``v13``, ``m7`` to ``m13`` and ``rdl`` are :doc:`gt3`'s decks, read as its *Above M6*
section describes, with every id marked ``.LEF``. The two LEFs differ there only in
what is not a check: GT2N's via templates draw some of their metals larger, and its
``Via<n>Array`` ``SPACING`` equals the cut where GT3's is the cut pitch.

Run over OpenROAD-flow-scripts' routed ``gt2n/aes``, which routes up to M9, these
decks report nothing.


Below V6: the LEF's cut spacing
-------------------------------

The runset names a spacing for V1 to V5 (``V<n>.5``) and for VG (``VG.6``) but checks
neither, and it has no spacing for VSD. The tech LEF gives each of these cut layers a
``SPACING``, which ``V<n>.5.LEF``, ``VG.6.LEF`` and ``VSD.6.LEF`` read every way, as
:doc:`gt3` does. V1 to V5's are GT3's, so ``v0`` to ``v5`` are GT3's decks; VG's and
VSD's are GT2N's own, in its ``vg`` and ``vsd``. Values in nm:

.. list-table::
   :header-rows: 1

   * - Cut
     - V1
     - V2
     - V3
     - V4
     - V5
     - VG
     - VSD
   * - ``SPACING``
     - 12
     - 12
     - 14
     - 21
     - 21
     - 28
     - 29

GT2N's LEF has no VBPR cut layer, so there is no VBPR spacing. ``V0.7a``, two V0s on
one M0 track 14 nm apart, is GT3's. Everything else the LEF states for M0 to M6 and V0
to V5, the runset already checks at the same values, as in GT3.

The ten standard-cell libraries and the routed ``gt2n/aes`` read clean under these
rules.


Backside power
--------------

The backside stack, BV0 to BRDL, connects to the buried power rail from below:

.. code-block:: text

   BRDL --BV4-- BM4 --BV3-- BM3 --BV2-- BM2 --BV1-- BM1 --BV0-- BPR --VBPR-- SDCON

These are distinct GDS layers; projected overlap between two metals connects them
only through a declared via. The connect graph includes the whole stack, so
``SDCON.2`` applies its different-net spacing after following backside connections.
Separate power rails remain separate unless geometry actually connects them.

The runset checks none of the backside. The ``backside`` suite (also in ``main``)
checks BV0 to BRDL from the technology LEF, read as the frontside above M6 is, with
every id marked ``.LEF``. The ``bpr`` and ``vbpr`` decks check the buried rail and its
connection to SDCON, from the runset; the LEF's BPR (32 nm wide, 112 nm apart, no
bend) agrees with them.

Backside rule sources
~~~~~~~~~~~~~~~~~~~~~

The audit uses GT2N revision ``5bc689fd70e4f2d7abf3952e17f308072bc2b0f6``:

* The `IC Validator runset
  <https://github.com/azadnaeemi/GT2N/blob/5bc689fd70e4f2d7abf3952e17f308072bc2b0f6/icv_runset/gt2_main.drc.rs>`_
  declares the backside layers and connect steps, but its included rule files
  contain no checks for BV0, BM1–BM4, BV1–BV4 or BRDL.
* The `technology LEF
  <https://github.com/azadnaeemi/GT2N/blob/5bc689fd70e4f2d7abf3952e17f308072bc2b0f6/techlib/gt2_tech.lef>`_
  supplies routing and cut-layer declarations, fixed via templates (``BV<n>_0``) and
  via-array rules (``BVia<n>Array``). The decks are translated from these.

The metals take the frontside's values: BM1 and BM2 are M10 and M11's, BM3 and BM4 are
M12 and M13's, and BRDL is RDL's. Values in nm:

.. list-table::
   :header-rows: 1

   * - Layer
     - GDS layer
     - Direction
     - ``.1`` width
     - ``.3`` space
     - ``.4`` tip
     - ``.5`` corner
   * - BM1, BM2
     - 920, 940
     - vertical, horizontal
     - 56
     - 56
     - 80
     - 66
   * - BM3, BM4
     - 960, 980
     - vertical, horizontal
     - 360
     - 360
     - 450
     - 450
   * - BRDL
     - 1000
     - vertical
     - 1600
     - 1600
     - none
     - none

None has a LEF ``MINSIZE``, so none has a ``.2``; ``techfile`` checks their areas.
BRDL has no ``RECTONLY``, and its
``.3`` is read every way.

A via is read as V6 to V13 are: its template's cut (``.1``, ``.2``), the cut layer's
``SPACING`` read every way (``.5``), the cut inside the metal nearer the devices
(``.7``) and the farther one (``.8``), and each metal's enclosure along its length
from the array rule (``BV<n>.<metal>.EN.LEF``):

.. list-table::
   :header-rows: 1

   * - Via
     - GDS layer
     - Joins
     - Cut
     - ``.5`` space
     - Enclosure
   * - BV0
     - 910
     - BPR, BM1
     - 40 × 32
     - 32
     - 10 by BPR, 10 by BM1
   * - BV1
     - 930
     - BM1, BM2
     - 56 × 56
     - 56
     - 12, 12
   * - BV2
     - 950
     - BM2, BM3
     - 360 × 56
     - 56
     - 12, 12
   * - BV3
     - 970
     - BM3, BM4
     - 360 × 360
     - 56
     - 20, 20
   * - BV4
     - 990
     - BM4, BRDL
     - 1600 × 360
     - 56
     - 40 by BM4, 200 by BRDL

Where a template and its array rule disagree, the array rule's enclosure is read. That
is what OpenROAD generates the power network's vias from:

* ``BV0_0`` runs BPR 1 nm past its cut; ``BVia0Array`` asks for 10 nm. The routed
  ``gt2n/aes`` below draws every BV0 from ``BVia0Array``, at 10 nm.
* ``BV2_0`` draws its BM2 as 80 × 384 nm, which does not cover its 360 nm-wide cut:
  it looks like the frontside ``V11_0``'s M11 with one value changed. ``BVia2Array``
  runs BM2 12 nm past the cut along its length, and a ``BV2_0`` drawn as the template
  stands fails ``BV2.8.LEF`` and ``BV2.BM2.EN.LEF``.

The cut-layer ``WIDTH`` gives way to the template's cut, as on the frontside: BV0's
cut is 40 nm across x against a 32 nm ``WIDTH``, and BV2 to BV4's cuts are 360 or
1600 nm against 56. Each array rule's ``SPACING`` is a cut pitch no wider than the cut,
so it says nothing about a gap.

OpenROAD-flow-scripts' routed ``gt2n/aes`` draws its power network on the backside:
16,569 BV0s, 6,447 BM1 shapes, 7,056 BV1s and 924 BM2 shapes. The ``backside``
suite reports nothing on it. Nothing in it reaches BV2 or beyond, so those decks are
tested only by their generated patterns.


Tests
-----

Run over the 13 nm SVT library with every cell abutted into rows on its placement
boundary, the way a placer would, ``main`` reports one issue that is the library's
own: ``gt2_6t_mux2_x1`` lands a V0 flush with the end of its M1, under V0.M1.1's
4 nm. The other nine libraries give the same result.

There is no VG-to-ACT rule. The runset's VG.ACT.1, a 6 nm vertical space, was removed
upstream in GT2N 5bc689f and GT3 fff4a84, so a gate contact may sit near or over an
active: ``gt2_6t_decapcc``'s two VGs 5 nm from an active on the 13 nm sheets, and 4 nm
over one on the 31 nm sheets, as one of ``gt2_6t_mux2_x1``'s is by 1.5 nm, are clean.

The 13 nm and 31 nm sheets share one placement site, but not one ACT: the 13 nm cells
draw it at y 35–48 nm and the 31 nm cells at y 26–57 nm. The filler and the backside
tap (``tapbspdn``) are one 42 nm pitch wide and draw their ACT across the whole cell,
so a 31 nm one between 13 nm cells stands 9 nm proud of them above and below. Each
protruding piece is one pitch across and fails ACT.2's 84 nm, as it would upstream. A
13 nm one-pitch cell is clean beside either sheet. The decap, two pitches wide, stands
proud by exactly 84 nm and is clean from either sheet.

A flow that mixes the sheets has to pick its one-pitch cells with this in mind.
OpenROAD's fill insertion takes the first filler of each width listed in
``FILL_CELLS``. ORFS's ``gt2n`` platform loads both sheets, 31 nm first; its
``FILL_CELLS`` listed every sheet's filler and its ``TAP_CELL_NAME`` followed the
primary sheet, so every filler and tap was a 31 nm cell. On ``gt2n/aes``, with mostly
13 nm logic, that gave 9,480 ACT.2 markers, every one on a 31 nm filler or tap between
13 nm cells. Taking the fillers and the tap from the 13 nm sheet whenever it is
loaded clears them all and leaves synthesis, placement and routing unchanged. GT2N's
own example flow names ``gt2_6t_filler_w31_lvt`` as its filler, so a 13 nm design
built from it should set ``FILL_CELLS`` to the matching 13 nm filler. The
``ACT.2.sheets`` pattern draws all four filler-and-neighbour pairings.

Every rule in every deck also has a generated pattern, written by ``gen/gt`` (shared
with GT3) under ``tests/data/gt2n/generated/<deck>/`` (``just gen-testdata-for
gt2n``). Each pattern
draws the shape exactly at the runset's limit, then one 0.5 nm DBU past it, inside
whatever else its deck reads: a VG in its gate and M0, a V0 flush with both wires.
The generator reads its limits from ``icv_runset/Include/gt2.*.drc.rs``, and above M6
from ``techlib/gt2_tech.lef``, with well and area limits from
``techlib/gt2_techfile.tf``, and stops if a deck disagrees, including a deck GT2N
shares with GT3. The test names the count each
pattern reports, and any other rule the drawing must also break, such as the bend in
a notch.

Technology-file patterns cover both width directions, spacing across x, y and
corners, notches, exact minimum areas and one-DBU-short shapes. Area cases also
exercise merged pieces, separate islands, holes, negative coordinates and the
7 µm and 20 µm tile boundaries. Suite tests check that these rules run only on
the layers belonging to that suite.

The standard-cell libraries contain no BV0–BRDL geometry. Separate synthetic tests
exercise two SDCONs joined only through the entire backside stack. Removing each
via in turn, on either branch, must disconnect the contacts and make ``SDCON.2``
report their 19 nm gap. These cases run across 7 µm and 20 µm tile boundaries.
Additional tests keep separate power rails apart and check two BM1 rails crossed
by BM2 with zero, one or two BV1s. The synthetic pads test connectivity, and run only
the ``sdcon`` deck: they do not meet the backside decks' rules.
