<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dummy_metal: hardening report

Deck `dummy_metal` against section 13.3 of the GF180MCU design manual (DM, dummy metal
fill).  11 layouts under `tests/data/gf180mcuD/generated/dummy_metal/DM*.h<n>.gds.gz`,
drawn by `gen/gf180mcuD/dummy_fill.rs`, each with a `#[case]` in the
`hardening_dummy_metal` table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck
at tiles 20, 7 and 100 and through the upstream KLayout runset
(`hardening/oracle-gf180.sh`, `DECKS=dummy`; the runset registers the section as
`dummy_metal1` … `dummy_metal6`, so `DECKS=dummy_metal` selects nothing).

**No count moved with the tile size, on any layout, on any rule.**

`show-deck` lists four entries per level on all five levels of variant D's stack:
DM<n>.2b as a space and a notch (0.98), DM<n>.3 (2.0 to the circuit metal of the same
level) and DM<n>.8 (6.0 to the union of FuseTop, POLYFUSE, FUSEWINDOW_D, PMNDMY, MTPMK
and OTP_MK).  That is exactly the foundry's `dummy_metal.rb`.  Against the manual the
section has four more rules that are plain geometry: DM.1 (the fill's width and length
are 2.0 µm), DM.4 and DM.5 (1 µm to the metal level above and to the one below, which for
Metal1 is Poly2), and DM.6 and DM.7 (no overlap with either).  Appendix B's "rules not
coded" list names nothing from section 13.

The five levels are one template, and `DM.levels.h1` confirms it: a 0.975 dummy-to-dummy
gap and a 1.995 dummy-to-drawn gap on each of Metal1 to Metal5 give ten violations, both
tools, at every tile size.  The rest of the battery is drawn on Metal1.

## Findings

### 1. An abutting pair and an overlapping pair are not measured at all (false negative)

Manual: "DM.3  Minimum space between dummy metal and circuit Metal line  2.0um" and
"DM.8  ... Space from these Structures  6um".

Layouts `DM1.3.h2` and `DM1.8.h2`: a 2 µm fill square sharing one edge with the circuit
metal (resp. the OTP_MK marker), and a second fill square lying half over it.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DM1.3.h2` | 0 / 0 / 0 | 1 |
| `DM1.8.h2` | 0 / 0 / 0 | 1 |

KLayout's single marker is the abutting pair, read as a separation of zero.

Verdict: two violations in each (three in `DM1.8.h2`, with finding 2).  A shared edge is
a space of nothing - the round's settled reading, and KLayout's here - and a fill lying on
the circuit metal has less space than that, not more.  For DM.8 the overlapping case is
not even an inference: the rule opens with "There should not be any dummy metal pattern
fill in the following areas".

### 2. A fill wholly under a DM.8 marker is not reported (false negative)

Manual: "DM.8  **There should not be any dummy metal pattern fill in the following
areas**  1. MIM CAP area (recognized by Fuse Top) 2. Poly fuse area (POLYFUSE) 3. Metal
fuse area (FUSEWINDOW_D) 4. Dummy metal exclusion area (PMNDMY) 5. MTP mark area (MTPMK)
6. OTP mark area (OTP_MK).  Space from these Structures  6um".

Layout `DM1.8.h2`, third probe: a 2 µm fill square in the middle of a 14 µm OTP_MK.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DM1.8.h2` third probe | 0 / 0 / 0 | 0 |

Verdict: one violation, which is why the case expects three.  The 6 µm is the *second*
half of DM.8; the first half is the prohibition, and both decks code only the second.
PMNDMY is in the list, so as things stand a design can put its dummy metal exclusion
marker over the fill and no rule notices.

### 3. DM.1 is missing: the dummy metal's size (no rule)

Manual: "DM.1  Min/Max Dummy metal line width/length  2.0 um", and the section's
generation method: "Dummy metal size: 2.0um x 2.0 um; space is 1.2um".

Layout `DM.1.h1`: a 2.0 x 2.0 dummy Metal1 square (clean), a 1.995 x 2.0, a 2.005 x 2.0
and a 2.0 x 4.0 bar.

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DM.1.h1` | 0 / 0 / 0 | 0 |

Verdict: `DM1.1` three times.  The rule is a minimum *and* a maximum on both dimensions -
"Min/Max ... width/length" - so the narrow square, the wide square and the long bar are
each a violation and only the 2.0 square is clean.  Neither tool has the rule.  The same
caveat as section 13.1's DCF.10: the deck's own good/bad patterns draw 2 µm fill squares,
which happen to be exactly the legal size, so coding DM.1 costs nothing there.

### 4. DM.4, DM.5, DM.6 and DM.7 are missing: the neighbouring metal levels (no rule)

Manual: "DM.4  Dummy Metal space to Subsequent Metal layer (E.g. Dummy M1 space to M2)
1um", "DM.5  Dummy Metal space to Previous Metal layer (E.g. Dummy M2 space to M1; Dummy
M1 space to Poly2)  1um", "DM.6  No overlap of Dummy Metal with the Subsequent Metal
layers", "DM.7  No overlap of Dummy Metal with the Previous layers".

Layouts `DM1.4.h1` (dummy Metal1 against circuit Metal2 at 1.0, 0.995, and overlapping)
and `DM1.5.h1` (the same against Poly2, Metal1's previous layer).

| layout | gdscheck @20 / @7 / @100 | KLayout |
| --- | --- | --- |
| `DM1.4.h1` | 0 / 0 / 0 | 0 |
| `DM1.5.h1` | 0 / 0 / 0 | 0 |

Verdict: `DM1.4` and `DM1.6` in the first, `DM1.5` and `DM1.7` in the second.  The
foundry's runset has both pairs written out and commented behind the same
`DUMMY_SUB_PREV` flag as DPF.12/13, with the note that "wafer.space moves all fill shapes
to active metal layers" - a property of one fill flow, not of the manual.  The manual
states them plainly, and they are the rules that keep fill from stacking on the circuit
above and below it.  Drawn on Metal1 here; the same four apply at every level, with
Poly2 as Metal1's previous layer and the level above as its subsequent one.

## Read and left alone

- Section 13.3's rule table is headed "LAYOUT GUIDELINE" throughout, but the section
  distinguishes inside itself - DM.2a is "for Layout" and DM.2b "for DRC" - so the
  heading is not a blanket exemption, and Appendix B, which is where exemptions are
  actually recorded, lists none of section 13.
- DM.2c (2.0 µm of fill space "for thick top metal (3um Top metal)") belongs to the 3 µm
  MetalTop option of section 7.16, whose MT30 rules the `metaltop` round already found
  rightly absent from variant D.  Not a gap.
- DM.9 (consecutive levels' fill patterns must not be exact replicates, offset 0.5 µm)
  and DM.10 (offset between fill of the same level, 0.5 µm) are the fill generator's own
  placement recipe, both starred, and there is nothing in a finished layout for them to
  measure that DM.1 and DM.2b do not already pin down.  Left alone.
- KLayout reports 3 markers where gdscheck reports 2 on `DM1.2b.h1`, `DM1.3.h1` and
  `DM1.8.h1`: it cuts the corner-to-corner violation into two edge pairs.  One gap is one
  violation.
- The six layers of DM.8 are one union in the deck and six `separation` calls upstream;
  `DM1.8.h3` puts a fill 5.995 µm from each of the five that `DM1.8.h1` does not use, and
  both tools report five.
- The "200 µm window at a step of 100 µm" and the 30 % trigger in the section's preamble
  are the dummy-metal generation recipe, not a rule; the density round settled that.

## Tested and clean (no need to redo)

DM<n>.2b at 0.98 / 0.975 wall to wall and 0.9829 / 0.9758 corner to corner, as a pair and
as a notch; the same 0.975 gap opening on x = 40 and on y = 20, a notch open across
x = 20, and a 0.98 gap on x = 42 that stays clean - at tiles 20, 7 and 100.  DM<n>.3 at
2.0 / 1.995 and both corner gaps.  DM<n>.8 at 6.0 / 5.995 and both corner gaps, against
OTP_MK and against the other five layers of the union.  The five levels running the same
template on the same geometry.
