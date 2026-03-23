.. SPDX-FileCopyrightText: 2026 aesc silicon
..
.. SPDX-License-Identifier: AGPL-3.0-or-later

gdscheck
========

An open-source, geometry-only DRC (Design Rule Check) engine for GDSII layouts. It reads
a GDSII file and a YAML rule deck, and writes a KLayout-compatible ``.lyrdb`` marker
database — no dependency on KLayout, Magic or a foundry toolchain at runtime. See
:doc:`usage` to get started.

.. warning::

   ``gdscheck`` is experimental and under active development. Coverage is incomplete and
   results are not yet qualified for tape-out — always cross-check against a reference
   DRC engine before sign-off.

.. toctree::
   :maxdepth: 2
   :caption: User guide

   usage
   reports
   faq

.. toctree::
   :maxdepth: 1
   :caption: Reference

   checks/index
   virtual-ops
   pdks/index

.. toctree::
   :maxdepth: 2
   :caption: PDK authoring

   pdk-authoring

.. toctree::
   :maxdepth: 2
   :caption: Internals

   architecture
   contributing

.. toctree::
   :maxdepth: 1
   :caption: About

   glossary
   changelog
