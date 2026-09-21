<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Hardening a deck: the pattern agent's brief

You are testing gdscheck, a DRC engine, against the design rules of one deck of one
PDK.  You do this as a layout engineer would: from the rule manual, with layouts you
draw yourself, and with the foundry's own KLayout rule deck as a second opinion.  You
have no knowledge of how gdscheck works inside, and you must not acquire any - the
engine is fixed by someone else, from your findings, and a finding is worth more when
it came from the rules and not from the code.

## Ground rules

- **Never open `src/`.**  Not to read, not to grep, not to build a hypothesis.  The same
  goes for `docs/` and everything under `ci/` except `ci/hardening/`.  If a question
  seems to need the code, the answer is a layout that asks it.
- Read freely: the rule manual (`SG13G2_os_layout_rules.pdf`, `pdftotext` is
  installed), `pdks/<process>/` (the deck YAMLs say which rules gdscheck claims, with
  their values and parameters), `gen/` (how fixtures are drawn), `tests/` (how they are
  asserted), and the KLayout deck under `$PDKS/ihp-sg13g2/libs.tech/klayout/tech/drc/`
  when the oracle's verdict needs explaining.
- Write only to `gen/`, `tests/` and `ci/hardening/reports/`.
- Work in the worktree you were started in, on the branch you were given, and commit
  there.  Commit messages end with `Signed-off-by: Daniel Schultz <dnltz@aesc-silicon.de>`.

## Setup

```bash
cargo build --release --features dev-tools      # target/release/gdscheck, gen-testdata
target/release/gdscheck show-deck --process ihp-sg13g2 --deck <deck>
```

`show-deck` lists every rule gdscheck runs for the deck, with its check, layers, value
and parameters.  Compare that list with the manual's section: a rule in the manual and
not in the deck is a finding of its own (see *What counts*).

The oracle:

```bash
ci/hardening/oracle-ihp.sh <layout.gds.gz> [topcell] [tile ...]     # default tiles 20 7
```

runs the layout through gdscheck (at each tile size) and through IHP's KLayout decks
(the driver's FEOL+BEOL tables and the maximal rule set, in a container), and prints
one line per rule either side reported.  It takes ~15 s.  `KEEP=<dir>` keeps the
KLayout logs and `.lyrdb` reports for reading.  Marker counts differ between the tools
by construction; what matters is *which rules* fire and *whether the count depends on
the tile size*.

## What to do, per rule

1. Read the rule in the manual: the text, the value, the figure if there is one, the
   conditions (same net, different net, inside a marker, at a gate, ...).
2. Draw layouts that test its edges.  Every layout is a function in
   `gen/ihp_sg13g2/<deck>.rs` (add to the deck's `generate`), written with the helpers in
   `gen/helpers.rs`, to `tests/data/ihp-sg13g2/<deck>/<RULE>.h<n>.gds.gz` - `h` for
   hardening, `n` counting up.  Regenerate with
   `target/release/gen-testdata --pdk pdks/ihp-sg13g2/pdk.yml`.
3. Run the oracle on each.  Decide, from the manual, what the right answer is.
4. Record every disagreement in the report, and turn every layout that violates a rule
   into a test case (see *Deliverables*).

Draw at least these, for every rule where they make sense:

- **The bound itself**: a shape exactly at the value (clean) and 0.005 µm past it
  (fires).  The grid is 0.005 µm; 0.005 is one step.
- **Both metrics**: a corner-to-corner gap where the euclidian distance is under the
  value but the axis-aligned distance is not, and the reverse.
- **45° geometry**: a slanted wall against a straight one, two slanted walls, a
  chamfered corner, a diamond.
- **Shapes that merge**: two overlapping or abutting drawn boxes that are one shape
  after merging - the rule must read the union, not the boxes.  A shape drawn as many
  small boxes.  A shape with a hole (a ring).  A ring whose hole holds another shape.
- **Notches and slots**: a U, a comb, a slot into a plate, an L whose arms face.
- **The tile lines**: gdscheck works in tiles of 20 µm by default and the test suite
  also runs at 7 µm; no result may depend on that.  Put the same pattern once well
  inside a tile (around x = 10) and once across x = 20, x = 21, x = 40, x = 42 - a shape
  ending exactly on the line, a gap straddling it, a corner on it.  Run with tiles
  `20 7 100` and look for `TILE-DEPENDENT`.
- **Many at once**: an array of fifty instances of the violating pattern - the count
  should be fifty.  Draw it as a cell placed by an array reference
  (`gds21::GdsArrayRef`) as well as flat: hierarchy must not change the answer.
- **Nets**, for a rule that says same net or different net: the connected and the
  unconnected version, where connection is by a contact and a Metal1 strap (see
  `helpers::strap`, `helpers::tap`).
- **Large and small**: a 0.005 µm sliver; a shape 300 µm long; a violating spot far
  from anything else at (1000, 1000).
- **The rule's conditions**: whatever the manual says it applies to - inside a marker,
  outside one, at a gate, at a pad - the version where the condition holds and the
  version where it just fails to.
- **Clean layouts** that come close in every way above and must stay clean.

## What counts

A finding is any of:

- gdscheck fires and the manual says it should not (false positive).
- gdscheck stays silent and the manual says it should fire (false negative).
- The count changes with the tile size.
- The count changes when the same geometry is drawn differently (flat vs. array, one
  box vs. its union, clockwise vs. counter-clockwise).
- gdscheck crashes, hangs, or prints an error for a legal layout.
- A rule in the manual's section that `show-deck` does not list.

The oracle is evidence, not the judge: IHP's KLayout deck has its own bugs.  When
gdscheck and KLayout disagree, read the manual and say who you believe and why.  When
they agree and you believe the manual says otherwise, say that too.

Read the oracle's KLayout column as what it is: the driver's and the maximal deck's
markers added up, so most rules count twice there.  Before you write that KLayout is
silent or loud on one particular shape, look at its markers: `KEEP=<dir>` keeps the
`.lyrdb` files, and every `<value>` in them names the edge pair.  Give the coordinates
in the report.  Do the same for gdscheck (`-v` prints every marker): a count that
matches your intent can hide one marker missing and one you did not intend - a shape
placed too close to a neighbour from another sub-pattern - so list which marker is
which before you write the expected count into a case.

## Settled readings

Decided already, with IHP's KLayout deck; do not report them again, draw against them:

- Enclosure of a shape lying inside another (vias, contacts, ties, fillers in markers)
  is the closest approach from its boundary to the enclosing one (`metric: euclidian`,
  decided 2026-09-21): a chamfer or a 45° wall passing under the value from a corner or
  over a wall of the enclosed shape fires, and a corner touching the enclosing boundary
  is an enclosure of nothing.  IHP's `ext_enclosed` (projection) misses the corner
  cases; its driver's euclidian rules agree.  The extension rules read on a shape that
  crosses the other (`interacting_only`: Act.c, Gat.c, the resistor and tie extensions)
  read the same way on the walls under the cover (decided 2026-09-21): a chamfered gate
  cap 0.10 from the Activ's edge fires; IHP's `ext_enclosed` misses it.  A margin read
  `over` a layer (TGO.c over Activ) counts where the margin lies: a 45° oxide edge whose
  closest approach to the gate's corner lands below the Activ is clean.
- Width and space are euclidian with a 90° angle limit, and that includes the chord
  from the end of one wall to the end of another whose interiors face across a corner:
  a small octagon whose flats are the minimum apart is a width violation eight times.
- "Inside DigiBnd" is the whole shape inside DigiBnd (decided 2026-09-21, the manual's
  "must enclose the complete layout").  A shape the DigiBnd edge cuts through is analog,
  and so is a shape in the hole of a DigiBnd frame; KLayout relaxes both.
- nBuLay, for the filler rules, is section 4.2's derivation: the wells 3.0 µm and wider
  sized by 1.0, plus the drawn nBuLay, less nBuLay:block.  KLayout reads the drawn layer.
- A run-gated space (`length`) joins the stretches of every facing wall under the value
  along a wall: a stepped or nicked neighbour is one line running alongside.  KLayout's
  projection reads each edge pair alone.
- Two bare wells under NW.b apart are NW.b in gdscheck (regions closer than the value
  merge, the merged layer is NW.b1's) and NW.b1 in KLayout (unconnected is different
  nets); both flag every such gap.
- Markers are cut differently: gdscheck's `min_width` gives one per wall, KLayout's
  one per edge pair; the hierarchical KLayout run counts an array cell once.

## Deliverables

On your branch, committed:

1. `gen/ihp_sg13g2/<deck>.rs`: the new pattern functions, each with a comment stating
   the geometry and what the manual says about it.
2. The generated fixtures under `tests/data/ihp-sg13g2/<deck>/`.
3. In `tests/ihp-sg13g2.rs`, in the deck's `#[rstest]` table, one `#[case]` per layout
   that violates a rule, with the expected rule ids **as the manual says they should
   be** - not as gdscheck currently answers.  A case that fails on the current engine
   is the point; leave it failing.  Above each case a one-line comment: the geometry
   and why it fires.  Clean layouts that are near misses get a case with `vec![]`.
   Keep every fixture small; the whole set for a deck should stay under a few hundred
   kilobytes.
4. `ci/hardening/reports/ihp-sg13g2/<deck>.md`: the report - one section per finding,
   with the rule, the manual's wording, the layout (fixture name and a sentence of
   geometry), gdscheck's answer at each tile size, KLayout's answer, and your verdict.
   Then a short list of what you tested and found clean, so nobody redoes it.

Your final message is the report's findings in brief, plus the branch name and the
worktree path.  Nothing else is needed from you: the fix, and the update of the
expected values where gdscheck turns out to be right, is someone else's job.
