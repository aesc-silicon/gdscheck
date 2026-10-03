<!--
SPDX-FileCopyrightText: 2026 aesc silicon
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# ASAP7 KLayout comparison

## Coverage update — 2026-10-02

The expanded corpus contains **100 generated layouts, the standard-cell library,
and the real `srambank_32b` macro**. All **203 manual-derived target verdicts
across 22 rules** pass in gdscheck. The pinned KLayout port agrees on **171/203**;
the remaining 32 verdicts differ. All 102 cases preserve marker rules, locations
and multiplicities between 20 and 7 micrometre gdscheck tiles. These targeted
results do not measure whole-PDK coverage or establish signoff parity.

This update adds the missing 17 nm `SRAM.SDT.LISD.OV.4` check and rejects empty
or touching-only SDT overlaps with LISD and ACTIVE. V0–V3 corner spacing now
selects 23/27/30 nm from the two gap-facing 5 nm end-caps, including the exact
threshold. Other changes cover partial V0–LISD enclosure, net-aware LIG spacing,
and forbidden even M4–M7 width multiples. Engine fixes preserve exact edge-length
selection and tile-invariant region and partial-enclosure marker locations.

| Full layout | gdscheck markers | KLayout markers |
|---|---:|---:|
| Standard cells | 34 | 68 |
| SRAM bank | 1,052 | 107,544 |

The standard-cell baseline dropped from 35 to 34: the old `LIG.SDT.S.8` finding
in `DFFASRHQNx1` connects through the extracted interconnect and is now correctly
excluded by the different-net condition. The SRAM bank's explicit SDT overlap
checks pass; its SDT-only CI baseline retains eight ordinary `SDT.LISD.AUX.4`
findings. Neither full-layout count is a claim that the layout is clean. The
large SRAM difference includes differing rule coverage and SRAM policy; its
individual findings have not all been adjudicated.

The reference pin and macro hash below are unchanged. Completed KLayout 0.30.2
reports were reused after checking the fixture hashes, reference revision/macro
hash, and parsed raw reports. Every gdscheck case was rerun at both tile sizes
with the final frozen binary. Source baseline: `963a993f63f7837bb290a450408e92bfc9a5655f`
plus the uncommitted coverage fixes; binary SHA-256:
`76aab32449576b8a47e18011066714f91b862d39ac280a889df248b5c08f46b2`.
Local detailed artifact: `/tmp/gdscheck-gap-review/oracle-verified-final/comparison.json`.
The standard runner below reproduces the full comparison without cached reports.

Known gaps remain: routing grids and track counts, ACTIVE/SDT width increments,
transistor-aware `ACTIVE.S.2A`, latch-up reach, `V0.LIG.AUX.2`, and the 27 nm
unaligned parallel-track branch of V0–V3 `S.1`. Conditional metal tip rules also
remain incomplete. See the deck headers and PDK documentation for the scope;
the new checks do not close those gaps.

Validation: all 3,075 Rust tests pass at both tile sizes, as do the seven
comparison-tool tests, Clippy with warnings denied, and the Sphinx documentation
build with warnings treated as errors.

## Historical initial comparison

This report records the initial comparison before the deck-wide convention-7 audit.
The current decks apply ordinary rules to non-SRAM subjects across all layers and
require positive-area SRAMDRC overlap for membership, retaining edge/vertex contact
in ordinary checks. See the PDK's **SRAM applicability** documentation and
`hardening/asap7/sram-scope.tsv`. The measurements in this historical section predate that policy. The current
coverage-limited results are recorded above.

The initial corpus agrees on **80 of 89 targeted verdicts across nine rules**.
All gdscheck expectations pass, and marker rule/location/multiplicity is invariant
between 20 and 7 micrometre tiles. This is a coverage-limited result, not a
whole-PDK parity percentage. The reference port has omissions and differing rule
interpretations; matching it unconditionally would weaken some checks.

Reference: [ASAP7_for_KLayout at ef77f08](https://github.com/laurentc2/ASAP7_for_KLayout/tree/ef77f080381cab3993428492a3dcb41044c3f518),
unmodified macro SHA-256 `b614f48ffebf07e371b0b9bdc7abfb21ae09a206b29e9e9149ea2740ff998cf5`.
KLayout version: 0.30.2. Manual: `asap7_drm_201207a.pdf`, sections 3.5, 3.7, 3.11,
3.14, 3.18 and 3.19. gdscheck's engine/deck baseline is `c84cd85`, with the new
corpus/comparison tooling added. No engine or deck behavior changed for this run.

Reproduce with `hardening/oracle-asap7.sh --pdk /path/to/ASAP7_for_KLayout`.
See [setup and output semantics](../../asap7/README.md). The runner records exact
input and binary hashes and source status in its JSON artifact. Each GDS is
flattened before both checkers run, preserving scale and coordinates.

## Targeted verdicts

| Rule | Agree / exercised | Disagreements |
|---|---:|---|
| ACTIVE.WELL.S.4 | 18 / 18 | None on these cases |
| SRAM.ACTIVE.WELL.S.5 | 18 / 18 | None on these cases |
| ACTIVE.WELL.EN.1 | 14 / 18 | KLayout applies ordinary enclosure to clipped crossing ACTIVE |
| SRAM.ACTIVE.WELL.EN.2 | 18 / 18 | None on these cases |
| SRAM.SDT.ACTIVE.OV.3 | 2 / 3 | KLayout does not flag 16.75 nm overlap |
| V1.M1.EN.1 | 4 / 6 | KLayout misses split-axis and 4.75/2 nm cases |
| M4.W.1 | 3 / 3 | None on these cases |
| M8.W.2 | 2 / 3 | KLayout reports M8.W.3 instead at length 400 nm |
| V8.M9.EN.2 | 1 / 2 | KLayout misses the 19.75 nm enclosure defect |

Pass verdicts mean the **targeted rule** must not fire, not that an isolated
pattern is clean under every rule. Incidental violations are retained in the
comparison artifacts. Only `routed_clean` is required to be globally clean;
`routed_bad` must produce exactly one gdscheck violation, V8.M9.EN.2.

## Findings from generated cases

### SRAM marker classification

`active_enclosure_crossing_53`, `_54`, `_55` and `_107` have a 32 by 27 nm
ACTIVE polygon crossing the SRAMDRC boundary at its midpoint, enclosed by WELL
with margins 13.25, 13.5, 13.75 and 26.75 nm respectively. The manual applies
SRAM rules to the whole polygon that interacts with SRAMDRC. The ordinary
ACTIVE.WELL.EN.1 must remain inactive; only the 13.25 nm case fails the relaxed
SRAM.ACTIVE.WELL.EN.2 rule.

KLayout additionally reports ordinary enclosure on the clipped non-SRAM part.
For `_54`, its ordinary-enclosure marker box is
`(6.9765, 6.9765)-(7.006, 7.0305)` micrometres. It also reports incidental
ACTIVE area and SRAM boundary-touch findings on the fragments. gdscheck's
whole-polygon classification agrees with the manual's interacting-polygon wording.
Keep that interpretation; do not clip ACTIVE to imitate this port.

The source of the difference is the port's layer-selection expression:

```ruby
(activesram, activenosram) = active.andnot(sramdrc)
```

This geometrically splits ACTIVE at the marker boundary. gdscheck instead selects
whole polygons with `ACTIVE interacting SRAMDRC` and
`ACTIVE not_interacting SRAMDRC`. Both approaches agree for polygons entirely
inside or entirely outside the marker; they differ when a polygon crosses it.

For example, `_54` places half of the 32 by 27 nm ACTIVE rectangle inside SRAMDRC,
with 13.5 nm WELL enclosure on every side. gdscheck accepts its enclosure under
SRAM.ACTIVE.WELL.EN.2. The port applies ACTIVE.WELL.EN.1 to the outside fragment
and demands 27 nm there. At 13.25 nm (`_53`), gdscheck still rejects the enclosure
under the relaxed SRAM rule; this classification is not an exemption from checking.

**Assessment: gdscheck is more consistent with the written manual.** Table 3.5.1
describes the relaxed enclosure as applying to ACTIVE interacting with SRAMDRC.
The adjacent area rules in Table 3.5.2 explicitly describe an ACTIVE polygon that
interacts with SRAMDRC, reinforcing whole-polygon selection. Confidence in this
reading is fairly high, but confirmation against the original Calibre deck remains
unavailable. The likely issue is the community port's selection expression;
KLayout's geometry engine is performing the clipping that expression requests.

### SRAM SDT overlap

`sdt_overlap_67` has 16.75 nm of vertical SDT-ACTIVE overlap. It must fail the
17 nm minimum; the 17 and 17.25 nm companions must pass. gdscheck reports the two
overlap walls, at approximately y=20.00025 and y=20.017 micrometres. KLayout does
not implement SRAM.SDT.ACTIVE.OV.3 in this revision; its SRAM SDT width check does
not substitute for an overlap check. Retain gdscheck's overlap rule.

### Asymmetric V1 enclosure

`v1_split_axes` gives M1 margins 5/0 nm horizontally and 2/2 nm vertically.
`v1_large_under` gives 4.75/2 nm horizontally. Both fail the required 5/2 nm on
one opposite pair. gdscheck flags each; the port misses both under V1.M1.EN.1.
An example gdscheck marker is the V1 wall
`(19.99, 19.99)-(19.99, 20.008)` micrometres. The port instead reports incidental
V1.M2.EN.2 on these patterns. Retain the same-pair interpretation already covered
by the Rust regressions; investigate the port's enclosure construction separately.

### M8 length-conditioned width

`m8_long_width_239` draws a 400 by 59.75 nm wire. At length 400 nm the manual
requires 60 nm width under M8.W.2. gdscheck reports its two long walls. KLayout
reports those locations under M8.W.3 (the 80 nm rule), and also flags the 60 and
60.25 nm companions under M8.W.3. Its length windows are shifted relative to the
manual's conditions. This is not a safe rule-name alias: renaming would hide the
false positives on the passing companions. Keep the discrepancy visible.

### Routed upper-metal example

`routed_clean` connects three pairs of orthogonal wires through V4, V6 and V8,
covering M4 through M9. Both checkers report zero violations.
`routed_bad` shortens the M9 wire's upper end, leaving V8 margins of 80/19.75 nm
vertically and 10/10 nm horizontally. Neither opposite pair meets 20 nm.
gdscheck reports one V8.M9.EN.2 marker at
`(21.97, 19.97)-(21.97, 20.01)` micrometres; KLayout reports no violations.
The port's opposite-side enclosure expression misses this case. Retain the
manual-derived failing expectation.

## Existing standard-cell library

The flattened LVT library produces 35 gdscheck markers and 68 KLayout markers.
Rule aliases have been applied below; locations are compared as bounding boxes.

| Rule | gdscheck | KLayout | Assessment |
|---|---:|---:|---|
| FIN.W.2 | 20 | 10 | Same locations, two walls versus one edge pair |
| GATE.ACTIVE.S.4 | 0 | 1 | Unresolved library discrepancy |
| GATE.S.3 | 2 | 0 | Port's neighbour construction misses isolated gates |
| LIG.LISD.S.7 | 0 | 8 | Unresolved library discrepancy |
| LIG.S.4-5 | 0 | 10 | Unresolved library discrepancy |
| LIG.SDT.S.8 | 1 | 1 | Same location |
| LVT.W.1 | 2 | 0 | Port does not check LVT minimum width |
| M1.S.2 | 0 | 14 | Unresolved library discrepancy |
| NSELECT.W.1 | 4 | 2 | Same locations, different marker decomposition |
| PSELECT.W.1 | 4 | 2 | Same locations, different marker decomposition |
| V0.S.1 | 0 | 13 | Unresolved library discrepancy |
| V1.S.4 | 0 | 6 | End-cap-conditioned spacing is a documented gdscheck gap |
| WELL.W.1 | 2 | 1 | Same locations, different marker decomposition |

The extra library findings are a review backlog, not waivers or proof that either
checker is right. Minimize each into a manual-derived case before changing a rule.
The port's GATE labels are swapped; the comparison normalizes them using their
descriptions while preserving the original category in JSON.

## Remaining coverage

This corpus deliberately starts with SRAM conditions, recent enclosure fixes and
upper routing. It does not cover every rule, a full SRAM macro, all cell contexts,
or all routing/net configurations. Track/grid restrictions, width multiples,
net-aware ACTIVE spacing, latch-up reach and conditional via spacings remain
documented gdscheck gaps. Add targeted passing and failing cases as these are
implemented, and record reference-port omissions separately from agreement.

Validation: all 3,003 Rust tests passed; ASAP7's 18 tests also passed with 7 micrometre
tiles. Seven comparison-tool tests passed. Formatting, Clippy and the Sphinx
documentation build passed. Strict comparison mode correctly failed on the
known routed enclosure discrepancy. All 53
generated GDS files plus the manifest regenerated byte-for-byte identically.
