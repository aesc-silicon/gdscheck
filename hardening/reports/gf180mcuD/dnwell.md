<!--
SPDX-FileCopyrightText: 2026 aesc silicon

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# gf180mcuD / dnwell: hardening report

Deck `dnwell` against the GF180MCU design manual, section 7.2 (DN.1-DN.3).  10 layouts,
`tests/data/gf180mcuD/generated/dnwell/DN.*.h<n>.gds.gz`, drawn by
`gen/gf180mcuD/dnwell.rs` (`hardening`), each with a `#[case]` in the `hardening_dnwell`
table of `tests/gf180mcuD.rs`.  Every layout ran through gdscheck at tiles 20, 7 and 100
and through the upstream KLayout runset (`hardening/oracle-gf180.sh` with
`DECKS=dnwell,lvpwell,nwell`; with `all` the runset aborts on any DNWELL layout - see
note B of the nwell report).

`show-deck` lists DN.1, DN.2a (space and notch), DN.2b and DN.3.  Note.1 of the section
("both 3.3V and 5V/6V transistors are not allowed in the same DNWELL") is a layout
guide, not a rule, and is absent on both sides.  Nothing is missing.

No count moved with the tile size in any layout - the 30 µm rings of `DN.3.h3` and the
straddling pairs of `DN.2a.h3` included - and gdscheck agreed with the manual on every
layout.  All 10 cases pass.

## Findings

None against gdscheck.  One against the runset, recorded so nobody chases it:

### A. KLayout ties a deep well through an N+ diffusion in the P-well it holds (runset false negative)

Manual: DN.2b is the space between deep wells at different potentials.  An N+
diffusion inside an LVPWELL inside the deep well is an NMOS terminal in the isolated
P-well - a junction away from the deep well - and strapping it to another deep well's
tap ties nothing.

Layout `DN.2b.h2` (a): two 7 × 7 wells 5.415 apart at y = 2, the left one tapped in its
N region, the right one holding a 2 × 2 LVPWELL (2.5 inside) with a contacted 0.6 N+
diffusion in it, one Metal1 plate over both contacts.

- gdscheck @20/7/100: DN.2b `(9,2)-(14.415,2)` - its tap layer is `ncomp ∩ (dnwell −
  lvpwell)`, so the diffusion in the P-well is no tap and the wells are two nets.
- KLayout: nothing (its `connect(dnwell, ncomp_con)` joins the deep well to any N+
  diffusion that overlaps it, the one in the P-well included, so the wells are one net
  and 5.415 is over DN.2a's 2.5).
- Verdict: DN.2b; gdscheck is right.  Case `dn_2b_h2` (expected 3, with the plate over
  the other well and the P+ diffusion for a tap).

## Notes that are not findings

- The runset's DN.3 and gdscheck's agree on every ring drawn: the C, the ring with an N+
  wall, one ring around two wells (both wells fire - the interior touches two), an
  N-well or an N+ diffusion beside the well inside the ring (shared, fires); and on
  every ring that is a ring: a P+ diffusion beside the well and an N-well inside it, a
  ring abutting the well, an octagonal ring, one drawn as sixteen boxes, a ring inside a
  ring.  "Directly surrounded" is read as upstream reads it; the manual's word does
  not settle whether one ring around two wells is enough, and the tools say no.
- Two wells with no tap at all (`DN.2b.h3`, 2.495 apart) are DN.2b in both tools:
  unconnected is different potentials.  Consistent with the nwell deck's reading of
  bare wells.

## Tested and found clean or correct (no need to redo)

- DN.1: 1.695 fires (one marker per wall), 1.7 clean, in rings (`DN.1.h1`).
- DN.2a: 2.495 fires on one net, 2.5 clean; a 2.495 slot into one well fires (notch),
  2.5 clean (`DN.2a.h1`); two wells tapped only through the N-wells they hold and one
  plate are one net, 2.495 fires (`DN.2a.h2`); one-net 2.495 gaps with the strap
  crossing x = 20 and x = 42, two-net 5.415 gaps straddling x = 40 and x = 21
  (`DN.2a.h3`).
- DN.2b: 5.415 fires on two nets, 5.42 clean, 2.495 on two nets is DN.2b and not DN.2a
  (`DN.2b.h1`); a plate over the other well without a contact and a P+ diffusion for a
  tap are no connection (`DN.2b.h2`); bare wells (`DN.2b.h3`).
- DN.3: the five rings that fail and the five that hold, above (`DN.3.h1`, `DN.3.h2`);
  a 30 µm ring across the tile lines holds, the same with a 0.5 gap fails, at every
  tile size (`DN.3.h3`).
