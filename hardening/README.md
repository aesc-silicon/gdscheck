<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Hardening

How gdscheck's decks were tested against the rule manuals by someone who has never
seen the engine, and what came of it.

- `SPEC.md` - the brief a pattern agent works from: draw the rules' edge cases from the
  manual, run them through gdscheck at three tile sizes and through the foundry's own
  KLayout deck, report every disagreement, commit the layouts as test cases expecting
  the manual's answer.  Its *settled readings* are the decisions taken so far.
- `oracle-ihp.sh` - the second opinion: a layout through gdscheck and IHP's KLayout
  decks in the elements container, one line per rule either side reported.
- `oracle-gf180.sh` - the same for GF180MCU, through the upstream `gf180mcu.drc`
  runset from the ciel checkout, in the same container.
- `reports/<process>/<deck>.md` - one report per round: the findings, the oracle's
  answers, the verdicts, and a *Resolution* section saying what was fixed in the engine
  or the deck, what was kept and why, and what the gdscheck owner decided.

The reports are kept because the decisions in them are the deck's rationale: where the
manual, the foundry deck and this engine disagree, the report says who was believed and
why.  A later round - a new manual revision, a better model drawing the patterns - starts
from the settled readings and the list of what was tested and found clean, not from
scratch.
