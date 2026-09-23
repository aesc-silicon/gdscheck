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
  goes for `docs/` and `ci/`.  If a question seems to need the code, the answer is a
  layout that asks it.
- Read freely: the rule manual (`SG13G2_os_layout_rules.pdf`, `pdftotext` is
  installed), `pdks/<process>/` (the deck YAMLs say which rules gdscheck claims, with
  their values and parameters), `gen/` (how fixtures are drawn), `tests/` (how they are
  asserted), and the KLayout deck under `$PDKS/ihp-sg13g2/libs.tech/klayout/tech/drc/`
  when the oracle's verdict needs explaining.
- Write only to `gen/`, `tests/` and `hardening/reports/`.
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
hardening/oracle-ihp.sh <layout.gds.gz> [topcell] [tile ...]     # default tiles 20 7
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
  (`gds21::GdsArrayRef`) as well as flat: hierarchy must not change the answer.  This
  one is the engine family's (`gen/engine`, every check family has it); a deck draws
  it only for a rule whose layer derivation could read hierarchy differently.
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

## Where a pattern belongs

A pattern that asks something of the *check* - the bound and the step past it, the two
metrics, a 45° wall, boxes that merge, a notch, a tile line, an array, a shape far off -
is the same on every layer and in every PDK, and belongs in the engine family:
`gen/engine/<check>.rs` drawing on the engine PDK's generic layers
(`tests/data/engine/pdk.yml`: Inner, Outer, Via, Diode), with its case in
`tests/engine_checks.rs`.  Drawn there once, it covers every deck of every PDK, and the
GF180 round starts with it already tested.  A deck's own fixtures are for what only that
deck has: the rule's conditions (a marker, a gate, a device), the layer derivations, the
values and the exemptions.  Two decks that are the same template on two layers -
Metal2 and Metal3, Via1 and Via2 - share one set of fixtures, the layers drawn at the
same place in one file where the decks read only their own layers of it (`metaln/`,
`vian/`).  The IHP rounds of 2026-09 drew everything per deck; their array layouts were
taken out once the engine family had them (2026-09-22), and the rest is sorted into the
engine family as it comes up.  Since 2026-09-23 every check has its classes there -
the engine decks name every check the registry knows, `overlap` included - and the
engine family is where a class found wanting on one PDK is drawn for all of them.
Once the two-layer space classes were there too (the plain pair, a touch under each
`abutting`, a pair under each `pairs`), the IHP fixtures that were only classes - the
45°, union, tile-line and bound variants of a plain width or space rule, 126 files -
were taken out (2026-09-23), so a deck's fixtures now read as its conditions and its
values, and a class is drawn once in `gen/engine/`.

## GF180MCU

The brief above is written for IHP; on GlobalFoundries' GF180MCU (process `gf180mcuD`)
the same round runs with these in place of the IHP names:

- **The manual** is the public design manual, rendered to text under `gf180mcu_drm/` in
  the main checkout (`/home/daniel/work/aesc/gdscheck/gf180mcu_drm/`, read it from
  there - it is not in git): `drm_07_02.txt` to `drm_07_18.txt` are chapter 7's layout
  rules (7.1 geometry, 7.2 DNWELL, 7.3 LVPWELL, 7.4 NWELL, 7.5 COMP, 7.6 Dualgate, 7.7
  Poly2, 7.8 Nplus, 7.9 Pplus, 7.10 SAB, 7.11 ESD, 7.12 Contact, 7.13 Metal, 7.14 Via,
  7.15 MetalTop, 7.16 3 µm MetalTop, 7.17 Mcell), `drm_09*.txt` the density and antenna
  chapters, `drm_10*.txt` the devices (10.1 PRES, 10.2 LRES, 10.3 HRES, 10.4 MIM, 10.5
  NAT, 10.6 BJT, 10.8 dummy exclude, 10.10 OTP, 10.11 EFUSE, 10.12 LDMOS, 10.13 YMTP)
  and `drm_12_3.txt` the guard ring.  A rule table reads as one line per cell - rule,
  description, the 3.3 V value, the 5 V/6 V value - and the figures did not survive the
  rendering.  Every rule with two values is two rules in the deck, `<id>_LV` under the
  3.3 V reading and `<id>_MV` where the Dualgate marker lies over the shape.
- **The foundry's deck** is the upstream KLayout runset from the ciel checkout,
  `~/.ciel/ciel/gf180mcu/versions/*/gf180mcuD/libs.tech/klayout/tech/drc/rule_decks/<deck>.rb`,
  the second reading of every rule (its `output` strings are the manual's wording).
- **The oracle** is `hardening/oracle-gf180.sh <layout> [topcell] [tile ...]`, the runset
  in the elements container (its klayout has what the runset needs, the machine's does
  not) plus gdscheck's main suite - about two seconds.  The runset runs the density
  rules on any layout (M1.4-M5.4, MT.3, PL.8, DCF.1b ...): noise unless the layout is
  about them.  Its counts are one run's, not two.
- **The layouts** go in `gen/gf180mcuD/<deck>.rs` beside the deck's good/bad pairs,
  which must stay as they are, written to
  `tests/data/gf180mcuD/generated/<deck>/<RULE>.h<n>.gds.gz`; regenerate with
  `target/release/gen-testdata --pdk pdks/gf180mcuD/pdk.yml`.  `show-deck` is
  `target/release/gdscheck show-deck --process gf180mcuD --deck <deck>`.
- **The cases** go in `tests/gf180mcuD.rs` under `// --- Hardening ---`: one `#[rstest]`
  table per deck, `hardening_<deck>`, each case
  `#[case::<rule>_h<n>("<deck>/<RULE>.h<n>.gds.gz", "TOP", vec![...expected ids...],
  vec![...ids to ignore...])` against `hardening(deck, gds, topcell, &ignore)`, which
  runs the one deck and returns the sorted rule ids.
- **The report** goes in `hardening/reports/gf180mcuD/<deck>.md`.
- The grid is 0.005 µm, as at IHP; the `offgrid` deck reads every layer.

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
4. `hardening/reports/ihp-sg13g2/<deck>.md`: the report - one section per finding,
   with the rule, the manual's wording, the layout (fixture name and a sentence of
   geometry), gdscheck's answer at each tile size, KLayout's answer, and your verdict.
   Then a short list of what you tested and found clean, so nobody redoes it.

Your final message is the report's findings in brief, plus the branch name and the
worktree path.  Nothing else is needed from you: the fix, and the update of the
expected values where gdscheck turns out to be right, is someone else's job.
