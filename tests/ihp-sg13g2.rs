// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use gdscheck::run_drc;
use rstest::rstest;

const DATA: &str = "tests/data/ihp-sg13g2";

fn drc(pdk: &str, deck: &str, gds: &str, topcell: &str, ignore: &[&str]) -> Vec<String> {
    let path = format!("{DATA}/{gds}");
    let violations = run_drc(&path, pdk, &[deck], None, topcell, true).expect("DRC run failed");
    let mut ids: Vec<String> = violations
        .into_iter()
        .filter(|v| !ignore.contains(&v.rule_id.as_str()))
        .map(|v| v.rule_id)
        .collect();
    ids.sort();
    ids
}

const PDK_IHP: &str = "ihp-sg13g2";

// --- Activ ---

const DECK_ACTIV: &str = "activ";

/// The density rules, which every small layout trips, plus whatever else a hardening
/// layout draws on purpose (a sliver under Act.a, a ring under Act.e, ...).
fn dens(extra: &[&'static str]) -> Vec<&'static str> {
    [&["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"][..], extra].concat()
}

#[rstest]
#[case("activ/Act.a.gds.gz", "TOP", vec!["Act.a", "Act.a", "Act.a", "Act.a"], vec!["Act.d", "AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.b.space.gds.gz", "TOP", vec!["Act.b", "Act.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.b.notch.gds.gz", "TOP", vec!["Act.b", "Act.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.c.gds.gz", "TOP", vec!["Act.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.e.gds.gz", "TOP", vec!["Act.e"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.d.gds.gz", "TOP", vec!["Act.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/Act.d.merge.gds.gz", "TOP", vec![], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
// The width is the narrowest dimension: the 5.005 × 5.0 and 5.0 × 5.005 fillers are 5.0
// wide and clean, the 5.005 × 5.005 one fires.
#[case("activ/AFil.a.gds.gz", "TOP", vec!["AFil.a"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.a1.gds.gz", "TOP", vec!["AFil.a1", "AFil.a1", "AFil.a1", "AFil.a1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.b.gds.gz", "TOP", vec!["AFil.b", "AFil.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.cont.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.gatpoly.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c1.gds.gz", "TOP", vec!["AFil.c1", "AFil.c1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nwell.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nbulay.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.e.gds.gz", "TOP", vec!["AFil.e", "AFil.e"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.i.gds.gz", "TOP", vec!["AFil.i", "AFil.i"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.j.gds.gz", "TOP", vec!["AFil.j"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3", "AFil.i"])]
#[case("activ/AFil.g.gds.gz", "TOP", vec![], vec!["AFil.a", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.g.fail.gds.gz", "TOP", vec!["AFil.g"], vec!["AFil.a", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.g1.gds.gz", "TOP", vec![], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g1.fail.gds.gz", "TOP", vec!["AFil.g1"], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g2.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g2.fail.gds.gz", "TOP", vec!["AFil.g2"], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g3.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g1"])]
#[case("activ/AFil.g3.fail.gds.gz", "TOP", vec!["AFil.g3"], vec!["Act.b", "AFil.g1"])]
#[case::afil_g2_boundary_ok("activ/AFil.g2.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::afil_g2_boundary_ring("activ/AFil.g2.boundary_ring.gds.gz", "TOP", vec![], vec![])]
// --- Hardening (ci/hardening/SPEC.md): expected values are the manual's answer, not the
// engine's; the reasoning is in ci/hardening/reports/ihp-sg13g2/activ.md.  A min_width
// violation counts two markers per narrow bar (one per long edge), as the Act.a case above;
// max_width likewise one per wall (four on a square).
// Three 0.145 bars (x, y, 300 µm long across every tile line) → two markers each.
#[case::act_a_h1("activ/Act.a.h1.gds.gz", "TOP", vec!["Act.a"; 6], dens(&[]))]
// 45°: a 0.148 diamond (4) and a 0.148 45° strip (2) fire; 0.156 and the chamfers are clean.
#[case::act_a_h2("activ/Act.a.h2.gds.gz", "TOP", vec!["Act.a"; 6], dens(&["Act.d"]))]
// Unions 0.145 wide (overlapping boxes, abutting slices, one ring side, an island) fire;
// unions 0.15 wide and a bar drawn as a 3 × 10 grid are clean.
#[case::act_a_h3("activ/Act.a.h3.gds.gz", "TOP", vec!["Act.a"; 8], dens(&[]))]
// Ten 0.145 bars on, across and straddling x = 20/21/40/42 plus an L cornered on x = 20.
#[case::act_a_h4("activ/Act.a.h4.gds.gz", "TOP", vec!["Act.a"; 20], dens(&[]))]
// Fifty 0.145 bars, flat and as a GdsArrayRef.
#[case::act_a_h5("activ/Act.a.h5.gds.gz", "TOP", vec!["Act.a"; 100], dens(&[]))]
#[case::act_a_h6("activ/Act.a.h6.gds.gz", "TOP", vec!["Act.a"; 100], dens(&[]))]
// A 0.005 sliver and a 0.145 bar at (1000, 1000).
#[case::act_a_h7("activ/Act.a.h7.gds.gz", "TOP", vec!["Act.a"; 4], dens(&["Act.d"]))]
// A comb with three 0.145 teeth; a U with 0.15 arms is clean.
#[case::act_a_h8("activ/Act.a.h8.gds.gz", "TOP", vec!["Act.a"; 6], dens(&[]))]
// Gap 0.205, a 0.145/0.145 diagonal (0.205) and a corner-on 0.205 fire; 0.21 and the
// 0.15/0.15 diagonal (0.212) are clean.
#[case::act_b_h1("activ/Act.b.h1.gds.gz", "TOP", vec!["Act.b"; 3], dens(&[]))]
// 45°: diamond tip to wall, two 45° strips, chamfer to corner, tip to tip at 0.205.
#[case::act_b_h2("activ/Act.b.h2.gds.gz", "TOP", vec!["Act.b"; 4], dens(&[]))]
// "Space or notch": straight and 45° notches (2 + 2), a comb with three 0.205 slots, a slot
// in a plate, a keyhole ring with a 0.205 hole, two facing Ls, an island 0.205 from a ring.
#[case::act_b_h3("activ/Act.b.h3.gds.gz", "TOP", vec!["Act.b"; 11], dens(&[]))]
// Overlapping, abutting and gridded boxes each 0.205 from a third box: one each.
#[case::act_b_h4("activ/Act.b.h4.gds.gz", "TOP", vec!["Act.b"; 3], dens(&[]))]
// Ten 0.205 gaps on, across and straddling x = 20/21/40/42, incl. a corner on x = 20.
#[case::act_b_h5("activ/Act.b.h5.gds.gz", "TOP", vec!["Act.b"; 10], dens(&[]))]
// Fifty 0.205 pairs, flat and as a GdsArrayRef.
#[case::act_b_h6("activ/Act.b.h6.gds.gz", "TOP", vec!["Act.b"; 50], dens(&[]))]
#[case::act_b_h7("activ/Act.b.h7.gds.gz", "TOP", vec!["Act.b"; 50], dens(&[]))]
// A 0.005 sliver 0.205 from a box, two 300 µm bars 0.205 apart, a pair at (1000, 1000).
#[case::act_b_h8("activ/Act.b.h8.gds.gz", "TOP", vec!["Act.b"; 3], dens(&["Act.a", "Act.d"]))]
// Activ 0.205 from Activ:filler is AFil.c1's and from Activ.mask nobody's; P+ to N+ Activ
// and two Activ under one GatPoly are Act.b.
#[case::act_b_h9("activ/Act.b.h9.gds.gz", "TOP", vec!["Act.b"; 2], dens(&["AFil.c1"]))]
// S/D extensions of 0.225: right, left, top (horizontal gate) and both sides (2).
#[case::act_c_h1("activ/Act.c.h1.gds.gz", "TOP", vec!["Act.c"; 5], dens(&[]))]
// Transistors turned by 45° with a 0.2298 and a 0.099 S/D fire; 0.2333 is clean; a chamfer
// passing 0.17 from a straight gate's corner with the walls 0.64 apart is clean (projection).
#[case::act_c_h2("activ/Act.c.h2.gds.gz", "TOP", vec!["Act.c"; 2], dens(&[]))]
// A gate from two overlapping poly boxes reads the union (0.225 fires once); a hole 0.16
// from the gate is a 0.16 S/D.  GatPoly beside or abutting the Activ, an Activ S/D from
// two abutting boxes, a gate across a sliver are nothing; the Activ past a gate *end*
// inside the Activ reads as a 0.10 extension, in KLayout too (report, finding 1).
#[case::act_c_h3("activ/Act.c.h3.gds.gz", "TOP", vec!["Act.c"; 3], dens(&["Act.a", "Act.d"]))]
// Nine 0.225 S/Ds on, across and straddling x = 20/21/40/42, two of them 10 µm wide.
#[case::act_c_h4("activ/Act.c.h4.gds.gz", "TOP", vec!["Act.c"; 9], dens(&[]))]
// Fifty transistors with a 0.225 S/D, flat and as a GdsArrayRef.
#[case::act_c_h5("activ/Act.c.h5.gds.gz", "TOP", vec!["Act.c"; 50], dens(&[]))]
#[case::act_c_h6("activ/Act.c.h6.gds.gz", "TOP", vec!["Act.c"; 50], dens(&[]))]
// A 300 µm transistor, a 300 µm gate across a small Activ, one at (1000, 1000).
#[case::act_c_h7("activ/Act.c.h7.gds.gz", "TOP", vec!["Act.c"; 3], dens(&[]))]
// Two fingers with a 0.225 outer S/D; one gate across two Activs, the upper 0.615 wide.
#[case::act_c_h8("activ/Act.c.h8.gds.gz", "TOP", vec!["Act.c"; 2], dens(&[]))]
// 0.1205 and 0.121 boxes, a 0.121 L and a 0.120 diamond fire; 0.122 and 0.125 are clean.
#[case::act_d_h1("activ/Act.d.h1.gds.gz", "TOP", vec!["Act.d"; 4], dens(&[]))]
// The area is the union's: overlapping boxes (0.12), a 6 × 8 grid (0.12), abutting boxes
// (0.12), a ring's material (0.12), an island (0.09) fire once each; two 0.09 boxes
// touching at one corner are two regions (2) - report, finding 2.
#[case::act_d_h2("activ/Act.d.h2.gds.gz", "TOP", vec!["Act.d"; 7], dens(&["Act.a", "Act.b", "Act.e"]))]
// Nine 0.12 shapes on, across and straddling x = 20/21/40/42; 0.122 and 0.123 are clean.
#[case::act_d_h3("activ/Act.d.h3.gds.gz", "TOP", vec!["Act.d"; 9], dens(&[]))]
// Fifty 0.12 boxes, flat and as a GdsArrayRef.
#[case::act_d_h4("activ/Act.d.h4.gds.gz", "TOP", vec!["Act.d"; 50], dens(&[]))]
#[case::act_d_h5("activ/Act.d.h5.gds.gz", "TOP", vec!["Act.d"; 50], dens(&[]))]
// A 0.01 sliver and a 0.12 box at (1000, 1000); a 0.005 × 30 sliver (0.15) is clean.
#[case::act_d_h6("activ/Act.d.h6.gds.gz", "TOP", vec!["Act.d"; 2], dens(&["Act.a"]))]
// Four 0.05 chamfers take a 0.1225 box to 0.1175; a 0.16 box with 0.1 chamfers keeps 0.14.
#[case::act_d_h7("activ/Act.d.h7.gds.gz", "TOP", vec!["Act.d"; 1], dens(&[]))]
// Holes of 0.1485, 0.149 (0.25 × 0.595), a 0.146 diamond and a 0.14 chamfered square fire;
// 0.15, 0.157 and 0.16 are clean.
#[case::act_e_h1("activ/Act.e.h1.gds.gz", "TOP", vec!["Act.e"; 4], dens(&[]))]
// Two Cs abutting close a 0.12 hole; a box on two walls leaves an L of 0.1491; a ring in a
// ring's hole (0.078); a ring of 88 boxes; a keyhole drawn either way round: one each.  A
// 0.34 island in a 0.5 hole leaves 0.1344 of enclosed area (1) - report, finding 3.  Two
// Cs 0.005 apart leave no hole.
#[case::act_e_h2("activ/Act.e.h2.gds.gz", "TOP", vec!["Act.e"; 7], dens(&["Act.b", "Act.d"]))]
// Seven 0.12 holes on, across and straddling x = 20/21/40/42, one in a 100 µm ring; a 0.15
// hole straddling 20, a 5.4 µm hole across it and an open U are clean.
#[case::act_e_h3("activ/Act.e.h3.gds.gz", "TOP", vec!["Act.e"; 8], dens(&[]))]
// Fifty 0.12 holes, flat and as a GdsArrayRef.
#[case::act_e_h4("activ/Act.e.h4.gds.gz", "TOP", vec!["Act.e"; 50], dens(&[]))]
#[case::act_e_h5("activ/Act.e.h5.gds.gz", "TOP", vec!["Act.e"; 50], dens(&[]))]
// A 0.005 × 0.5 hole and a 0.12 hole at (1000, 1000); a 300 µm ring's hole is clean.
#[case::act_e_h6("activ/Act.e.h6.gds.gz", "TOP", vec!["Act.e"; 2], dens(&["Act.b"]))]
// A 5.005 square fires, once (a maximum's `span: narrowest` reports the part a 5 × 5
// square fits in, one per region).  A 5.005 × 5.0 filler is 5.0 wide, a 3 × 20 bar 3, an
// L with 3-wide arms 3: clean - the width is the smaller span (report, finding 4).
#[case::afil_a_h1("activ/AFil.a.h1.gds.gz", "TOP", vec!["AFil.a"; 1], dens(&[]))]
// A 5.02 diamond, a 5.02 45° strip, a 5.005 × 6 union and a 5.005 square from four boxes
// fire, one each; 4.95, a 5.0 × 6 union and a 6 × 6 filler with a hole (2.75 wide) are
// clean.
#[case::afil_a_h2("activ/AFil.a.h2.gds.gz", "TOP", vec!["AFil.a"; 4], dens(&[]))]
// Seven 5.005 squares on, across and straddling x = 20/21/40/42 and a 5.005-tall 30 µm
// bar, one each; a 5.0 square straddling 20 and a 5.0-tall bar are clean.
#[case::afil_a_h3("activ/AFil.a.h3.gds.gz", "TOP", vec!["AFil.a"; 8], dens(&[]))]
// Fifty 5.005 squares, flat and as a GdsArrayRef.
#[case::afil_a_h4("activ/AFil.a.h4.gds.gz", "TOP", vec!["AFil.a"; 50], dens(&[]))]
#[case::afil_a_h5("activ/AFil.a.h5.gds.gz", "TOP", vec!["AFil.a"; 50], dens(&[]))]
// A 300 × 5.005 bar and a 5.005 square at (1000, 1000), one each.
#[case::afil_a_h6("activ/AFil.a.h6.gds.gz", "TOP", vec!["AFil.a"; 2], dens(&[]))]
// Three 0.995 bars (x, y, 300 µm long) → two markers each.  The 300 µm bars also draw
// AFil.a on their length (finding 4), set aside here and in the cases below.
#[case::afil_a1_h1("activ/AFil.a1.h1.gds.gz", "TOP", vec!["AFil.a1"; 6], dens(&["AFil.a"]))]
// 45°: a 0.99 diamond (4) and a 0.99 strip (2) fire; 1.004 and a chamfered box are clean.
#[case::afil_a1_h2("activ/AFil.a1.h2.gds.gz", "TOP", vec!["AFil.a1"; 6], dens(&[]))]
// Unions 0.995 wide (overlap, slices, one ring side, an island) fire; 1.0 unions and a grid
// are clean.
#[case::afil_a1_h3("activ/AFil.a1.h3.gds.gz", "TOP", vec!["AFil.a1"; 8], dens(&["AFil.a"]))]
// Ten 0.995 bars on, across and straddling x = 20/21/40/42 plus an L cornered on x = 20.
#[case::afil_a1_h4("activ/AFil.a1.h4.gds.gz", "TOP", vec!["AFil.a1"; 20], dens(&["AFil.a"]))]
// Fifty 0.995 bars, flat and as a GdsArrayRef.
#[case::afil_a1_h5("activ/AFil.a1.h5.gds.gz", "TOP", vec!["AFil.a1"; 100], dens(&[]))]
#[case::afil_a1_h6("activ/AFil.a1.h6.gds.gz", "TOP", vec!["AFil.a1"; 100], dens(&[]))]
// A 0.005 sliver and a 0.995 bar at (1000, 1000).
#[case::afil_a1_h7("activ/AFil.a1.h7.gds.gz", "TOP", vec!["AFil.a1"; 4], dens(&[]))]
// A comb with three 0.995 teeth; a U with 1.0 arms is clean.
#[case::afil_a1_h8("activ/AFil.a1.h8.gds.gz", "TOP", vec!["AFil.a1"; 6], dens(&["AFil.a"]))]
// Gap 0.415, a 0.295/0.295 diagonal (0.417) and a corner-on 0.415 fire; 0.42/0.424 clean.
#[case::afil_b_h1("activ/AFil.b.h1.gds.gz", "TOP", vec!["AFil.b"; 3], dens(&[]))]
// 45°: diamond tip to wall, two 45° strips, chamfer to corner, tip to tip at 0.415.
#[case::afil_b_h2("activ/AFil.b.h2.gds.gz", "TOP", vec!["AFil.b"; 4], dens(&["AFil.a"]))]
// "Space" without "or notch": 0.415 notches into a filler are not AFil.b's; two facing Ls
// and an island in a ring at 0.415 are (report, note A).
#[case::afil_b_h3("activ/AFil.b.h3.gds.gz", "TOP", vec!["AFil.b"; 2], dens(&[]))]
// Overlapping, abutting and gridded boxes each 0.415 from a third filler: one each.
#[case::afil_b_h4("activ/AFil.b.h4.gds.gz", "TOP", vec!["AFil.b"; 3], dens(&[]))]
// Ten 0.415 gaps on, across and straddling x = 20/21/40/42, incl. a corner on x = 20.
#[case::afil_b_h5("activ/AFil.b.h5.gds.gz", "TOP", vec!["AFil.b"; 10], dens(&["AFil.a"]))]
// Fifty 0.415 pairs, flat and as a GdsArrayRef.
#[case::afil_b_h6("activ/AFil.b.h6.gds.gz", "TOP", vec!["AFil.b"; 50], dens(&[]))]
#[case::afil_b_h7("activ/AFil.b.h7.gds.gz", "TOP", vec!["AFil.b"; 50], dens(&[]))]
// A 0.005 sliver 0.415 from a filler, two 300 µm bars 0.415 apart, a pair at (1000, 1000).
#[case::afil_b_h8("activ/AFil.b.h8.gds.gz", "TOP", vec!["AFil.b"; 3], dens(&["AFil.a", "AFil.a1"]))]
// A filler 0.415 from an Activ is AFil.c1's, from an Activ.mask nobody's.
#[case::afil_b_h9("activ/AFil.b.h9.gds.gz", "TOP", vec![], dens(&["AFil.c1"]))]
// A Cont and a GatPoly 1.095 away, a Cont and a GatPoly at a 0.775/0.775 diagonal (1.096)
// fire; 1.10 and 0.78/0.78 (1.103) are clean.
#[case::afil_c_h1("activ/AFil.c.h1.gds.gz", "TOP", vec!["AFil.c"; 4], dens(&[]))]
// 45°: a GatPoly chamfer 1.096 from a filler's corner and a GatPoly diamond tip at 1.095
// fire; a chamfer at 1.103 is clean.
#[case::afil_c_h2("activ/AFil.c.h2.gds.gz", "TOP", vec!["AFil.c"; 2], dens(&[]))]
// A Cont bar at 1.095 and a Cont on an Activ at 1.095 fire; a GatPoly:filler at 1.095 is
// not GatPoly; a Cont overlapping the filler's edge shares area with it and is no pair,
// as KLayout has it (report, finding 5).
#[case::afil_c_h3("activ/AFil.c.h3.gds.gz", "TOP", vec!["AFil.c"; 2], dens(&[]))]
// Eight 1.095 gaps on, across and straddling x = 20/21/40/42, one to a 10 µm GatPoly.
#[case::afil_c_h4("activ/AFil.c.h4.gds.gz", "TOP", vec!["AFil.c"; 8], dens(&["AFil.a"]))]
// Fifty filler/Cont pairs at 1.095, flat and as a GdsArrayRef.
#[case::afil_c_h5("activ/AFil.c.h5.gds.gz", "TOP", vec!["AFil.c"; 50], dens(&[]))]
#[case::afil_c_h6("activ/AFil.c.h6.gds.gz", "TOP", vec!["AFil.c"; 50], dens(&[]))]
// A 300 µm GatPoly 1.095 from a 300 µm filler, a pair at (1000, 1000).
#[case::afil_c_h7("activ/AFil.c.h7.gds.gz", "TOP", vec!["AFil.c"; 2], dens(&["AFil.a"]))]
// Gap 0.415, a 0.295/0.295 diagonal (0.417) and a corner-on 0.415 fire; 0.42/0.424 clean.
#[case::afil_c1_h1("activ/AFil.c1.h1.gds.gz", "TOP", vec!["AFil.c1"; 3], dens(&[]))]
// 45°: an Activ diamond tip, an Activ strip against a filler strip, a filler chamfer
// against an Activ corner, all at 0.415-0.417.
#[case::afil_c1_h2("activ/AFil.c1.h2.gds.gz", "TOP", vec!["AFil.c1"; 3], dens(&["AFil.a"]))]
// Activ.mask is not Activ, a filler is AFil.b's; an Activ abutting a filler is at no
// distance (`abutting: report`), one overlapping it shares area and is no pair, as
// KLayout has it (report, finding 5).
#[case::afil_c1_h3("activ/AFil.c1.h3.gds.gz", "TOP", vec!["AFil.c1"; 1], dens(&["AFil.b"]))]
// Eight 0.415 gaps on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_c1_h4("activ/AFil.c1.h4.gds.gz", "TOP", vec!["AFil.c1"; 8], dens(&["AFil.a"]))]
// Fifty filler/Activ pairs at 0.415, flat and as a GdsArrayRef.
#[case::afil_c1_h5("activ/AFil.c1.h5.gds.gz", "TOP", vec!["AFil.c1"; 50], dens(&[]))]
#[case::afil_c1_h6("activ/AFil.c1.h6.gds.gz", "TOP", vec!["AFil.c1"; 50], dens(&[]))]
// A 0.005 Activ sliver 0.415 from a filler, 300 µm bars 0.415 apart, a pair at (1000, 1000).
#[case::afil_c1_h7("activ/AFil.c1.h7.gds.gz", "TOP", vec!["AFil.c1"; 3], dens(&["AFil.a", "Act.a", "Act.d"]))]
// NWell and nBuLay at 0.995 and at a 0.705/0.705 diagonal (0.997) fire; 1.0/1.004 clean.
#[case::afil_d_h1("activ/AFil.d.h1.gds.gz", "TOP", vec!["AFil.d"; 4], dens(&[]))]
// Figure 5.6 measures "d" inside the well too: a filler 0.5 inside a NWell and one 0.5
// inside an nBuLay fire (an enclosure entry on the fillers inside), 1.0 inside is clean;
// a filler crossing the edge is in neither reading, as KLayout has it (report, finding 6).
#[case::afil_d_h2("activ/AFil.d.h2.gds.gz", "TOP", vec!["AFil.d"; 2], dens(&[]))]
// nBuLay is read as drawn, as IHP's deck reads it (report, finding 7 - open): a filler
// 1.5 from a 3.0 µm NWell has no drawn nBuLay near it and is clean, a drawn nBuLay
// under nBuLay:block still counts (0.5 away: fires), the half-blocked one fires for
// both fillers 0.5 from it.  Section 4.2's derived nBuLay would read 1, 0 and 1.
#[case::afil_d_h3("activ/AFil.d.h3.gds.gz", "TOP", vec!["AFil.d"; 3], dens(&[]))]
// 45°: a NWell diamond tip at 0.995 and a NWell chamfer 0.997 from a corner fire; 1.004 clean.
#[case::afil_d_h4("activ/AFil.d.h4.gds.gz", "TOP", vec!["AFil.d"; 2], dens(&[]))]
// Eight 0.995 gaps on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_d_h5("activ/AFil.d.h5.gds.gz", "TOP", vec!["AFil.d"; 8], dens(&["AFil.a"]))]
// Fifty filler/NWell pairs at 0.995, flat and as a GdsArrayRef.
#[case::afil_d_h6("activ/AFil.d.h6.gds.gz", "TOP", vec!["AFil.d"; 50], dens(&[]))]
#[case::afil_d_h7("activ/AFil.d.h7.gds.gz", "TOP", vec!["AFil.d"; 50], dens(&[]))]
// A 300 µm NWell 0.995 from a 300 µm filler, a pair at (1000, 1000).
#[case::afil_d_h8("activ/AFil.d.h8.gds.gz", "TOP", vec!["AFil.d"; 2], dens(&["AFil.a"]))]
// TRANS at 0.995, at a 0.705/0.705 diagonal (0.997) and corner-on fire; 1.0/1.004 clean.
#[case::afil_e_h1("activ/AFil.e.h1.gds.gz", "TOP", vec!["AFil.e"; 3], dens(&[]))]
// A filler well inside a TRANS is enclosed by more than the value and one crossing its
// edge is in neither reading, as KLayout has it (report, finding 6); a TRANS diamond
// tip at 0.995 fires.
#[case::afil_e_h2("activ/AFil.e.h2.gds.gz", "TOP", vec!["AFil.e"; 1], dens(&[]))]
// Eight 0.995 gaps on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_e_h3("activ/AFil.e.h3.gds.gz", "TOP", vec!["AFil.e"; 8], dens(&["AFil.a"]))]
// Fifty filler/TRANS pairs at 0.995, flat and as a GdsArrayRef.
#[case::afil_e_h4("activ/AFil.e.h4.gds.gz", "TOP", vec!["AFil.e"; 50], dens(&[]))]
#[case::afil_e_h5("activ/AFil.e.h5.gds.gz", "TOP", vec!["AFil.e"; 50], dens(&[]))]
// A 300 µm TRANS 0.995 from a 300 µm filler, a pair at (1000, 1000).
#[case::afil_e_h6("activ/AFil.e.h6.gds.gz", "TOP", vec!["AFil.e"; 2], dens(&["AFil.a"]))]
// PWell:block at 1.495, at a 1.06/1.06 diagonal (1.499) and corner-on fire; 1.5/1.506 clean.
#[case::afil_i_h1("activ/AFil.i.h1.gds.gz", "TOP", vec!["AFil.i"; 3], dens(&[]))]
// "Space to edges": a filler inside a block 1.495 from its edge fires (an enclosure
// entry; 1.5 is clean), a filler crossing the edge is in neither reading, as KLayout
// has it (report, finding 8).
#[case::afil_i_h2("activ/AFil.i.h2.gds.gz", "TOP", vec!["AFil.i"; 1], dens(&[]))]
// 45°: a block chamfer 1.499 from a filler's corner and a block diamond tip at 1.495 fire.
#[case::afil_i_h3("activ/AFil.i.h3.gds.gz", "TOP", vec!["AFil.i"; 2], dens(&[]))]
// Eight 1.495 gaps on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_i_h4("activ/AFil.i.h4.gds.gz", "TOP", vec!["AFil.i"; 8], dens(&["AFil.a"]))]
// Fifty filler/block pairs at 1.495, flat and as a GdsArrayRef.
#[case::afil_i_h5("activ/AFil.i.h5.gds.gz", "TOP", vec!["AFil.i"; 50], dens(&[]))]
#[case::afil_i_h6("activ/AFil.i.h6.gds.gz", "TOP", vec!["AFil.i"; 50], dens(&[]))]
// A 300 µm block 1.495 from a 300 µm filler, a pair at (1000, 1000).
#[case::afil_i_h7("activ/AFil.i.h7.gds.gz", "TOP", vec!["AFil.i"; 2], dens(&["AFil.a"]))]
// nSD:block at 0.245, SalBlock at 0.245, both at 0.245 (one edge, one marker) and an
// nSD:block ending on the filler's edge fire; 0.25 on both is clean.
#[case::afil_j_h1("activ/AFil.j.h1.gds.gz", "TOP", vec!["AFil.j"; 4], dens(&[]))]
// An nSD:block from two boxes whose union encloses by 0.245 fires; by 0.25 is clean, as are
// a filler outside any block, one crossing or touching the block's edge with 0.25 all
// round, and one half under the block (those three are AFil.i's).
#[case::afil_j_h2("activ/AFil.j.h2.gds.gz", "TOP", vec!["AFil.j"; 1], dens(&["AFil.i"]))]
// Parallel chamfers: nSD:block 0.244 from the filler's chamfer fires, SalBlock at 0.251 is
// clean; a chamfer 0.20 from a square corner with the walls at 0.30 is clean (projection).
#[case::afil_j_h3("activ/AFil.j.h3.gds.gz", "TOP", vec!["AFil.j"; 1], dens(&[]))]
// Eight 0.245 margins on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_j_h4("activ/AFil.j.h4.gds.gz", "TOP", vec!["AFil.j"; 8], dens(&["AFil.a"]))]
// Fifty fillers with a 0.245 margin, flat and as a GdsArrayRef.
#[case::afil_j_h5("activ/AFil.j.h5.gds.gz", "TOP", vec!["AFil.j"; 50], dens(&[]))]
#[case::afil_j_h6("activ/AFil.j.h6.gds.gz", "TOP", vec!["AFil.j"; 50], dens(&[]))]
// A 300 µm filler with a 0.245 SalBlock margin, a cell at (1000, 1000).
#[case::afil_j_h7("activ/AFil.j.h7.gds.gz", "TOP", vec!["AFil.j"; 2], dens(&["AFil.a"]))]
// The same 30 % of stripes on Activ, Activ:filler and Activ.mask is 30 % of Activ, under
// AFil.g's 35 %; summing the layers reads 90 % and fires g1/g3 instead - report, finding 9.
#[case::afil_g_h1("activ/AFil.g.h1.gds.gz", "TOP", vec!["AFil.g"], vec!["AFil.a"])]
// "Any 800 × 800 area": a 700 µm hole at (150, 150) in a full plate leaves 23.4 % in the
// window at (100, 100) (AFil.g2; the count is the tool's cut of a sliding window) while
// the windows on the 800 grid are fine; 51 % globally.  The 200 µm strips left over on the
// 800 grid are no 800 × 800 area and draw no AFil.g3 - report, finding 10.
#[case::afil_g2_h2("activ/AFil.g2.h2.gds.gz", "TOP", vec!["AFil.g2"], vec![])]
// A 650 µm hole in a 970-tall plate: 54.75 % globally, every window between 32.8 and 39 %.
#[case::afil_g2_h3("activ/AFil.g2.h3.gds.gz", "TOP", vec![], vec![])]
fn test_activ(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_ACTIV, gds, topcell, &ignore), expected);
}

// --- ThickGateOxide ---

const DECK_TGO: &str = "tgo";

#[rstest]
#[case("tgo/TGO.a.gds.gz", "TOP", vec!["TGO.a"], vec![])]
#[case("tgo/TGO.b.gds.gz", "TOP", vec!["TGO.b", "TGO.b"], vec![])]
#[case("tgo/TGO.c.gds.gz", "TOP", vec!["TGO.c"], vec![])]
#[case("tgo/TGO.d.gds.gz", "TOP", vec!["TGO.d"], vec![])]
#[case("tgo/TGO.e.gds.gz", "TOP", vec!["TGO.e", "TGO.e"], vec![])]
#[case("tgo/TGO.f.gds.gz", "TOP", vec!["TGO.f"; 4], vec![])]
fn test_tgo(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TGO, gds, topcell, &ignore), expected);
}

// --- GatPoly ---

const DECK_GAT: &str = "gatpoly";

#[rstest]
#[case("gatpoly/Gat.a.gds.gz", "TOP", vec!["Gat.a", "Gat.a", "Gat.a", "Gat.a"], vec!["Gat.e", "GFil.g"])]
#[case("gatpoly/Gat.a1.gds.gz", "TOP", vec!["Gat.a1", "Gat.a1"], vec!["Gat.a", "GFil.g"])]
#[case("gatpoly/Gat.a2.gds.gz", "TOP", vec!["Gat.a2", "Gat.a2"], vec!["Gat.a", "GFil.g"])]
#[case("gatpoly/Gat.a3.gds.gz", "TOP", vec!["Gat.a3", "Gat.a3"], vec!["GFil.g"])]
#[case("gatpoly/Gat.a4.gds.gz", "TOP", vec!["Gat.a4", "Gat.a4"], vec!["GFil.g"])]
#[case("gatpoly/Gat.b.space.gds.gz", "TOP", vec!["Gat.b", "Gat.b"], vec!["GFil.g"])]
#[case("gatpoly/Gat.b.notch.gds.gz", "TOP", vec!["Gat.b", "Gat.b"], vec!["GFil.g"])]
// The 0.3 µm gates under ThickGateOx are 3.3 V NFETs, Gat.a3 at 0.45; ignored here.
#[case("gatpoly/Gat.b1.gds.gz", "TOP", vec!["Gat.b1"], vec!["GFil.g", "Gat.a3"])]
#[case("gatpoly/Gat.c.gds.gz", "TOP", vec!["Gat.c"], vec!["GFil.g"])]
#[case("gatpoly/Gat.d.gds.gz", "TOP", vec!["Gat.d", "Gat.d"], vec!["GFil.g"])]
#[case("gatpoly/Gat.e.gds.gz", "TOP", vec!["Gat.e"], vec!["GFil.g"])]
// Gat.f and Gat.g draw 45° poly with square ends: the acute tips are Gat.a widths, as
// they are for KLayout, and are set aside here.
// Gat.f is the gate region that is no rectangle, one report per gate.
#[case("gatpoly/Gat.f.gds.gz", "TOP", vec!["Gat.f"], vec!["GFil.g", "Gat.c", "Gat.e", "Gat.a"])]
#[case("gatpoly/Gat.g.gds.gz", "TOP", vec!["Gat.g", "Gat.g"], vec!["Gat.e", "GFil.g", "Gat.a"])]
// The width is the narrowest dimension: only the 5.005 × 5.005 filler fires.
#[case("gatpoly/GFil.a.gds.gz", "TOP", vec!["GFil.a"], vec!["GFil.g"])]
#[case("gatpoly/GFil.b.gds.gz", "TOP", vec!["GFil.b"; 4], vec!["GFil.g"])]
#[case("gatpoly/GFil.c.gds.gz", "TOP", vec!["GFil.c", "GFil.c"], vec!["GFil.g"])]
#[case("gatpoly/GFil.d.gds.gz", "TOP", vec!["GFil.d"; 12], vec!["GFil.g"])]
#[case("gatpoly/GFil.e.nwell.gds.gz", "TOP", vec!["GFil.e", "GFil.e"], vec!["GFil.g"])]
#[case("gatpoly/GFil.e.nbulay.gds.gz", "TOP", vec!["GFil.e", "GFil.e"], vec!["GFil.g"])]
#[case("gatpoly/GFil.f.gds.gz", "TOP", vec!["GFil.f", "GFil.f"], vec!["GFil.g"])]
#[case("gatpoly/GFil.i.gds.gz", "TOP", vec!["GFil.i"], vec!["GFil.g"])]
#[case("gatpoly/GFil.j.gds.gz", "TOP", vec!["GFil.j"], vec!["GFil.g"])]
#[case("gatpoly/GFil.g.gds.gz", "TOP", vec![], vec!["GFil.a"])]
#[case("gatpoly/GFil.g.fail.gds.gz", "TOP", vec!["GFil.g"], vec!["GFil.a"])]
#[case::gfil_g_boundary_ok("gatpoly/GFil.g.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::gfil_g_boundary_ring("gatpoly/GFil.g.boundary_ring.gds.gz", "TOP", vec![], vec![])]
// --- Hardening (ci/hardening/SPEC.md): expected values are the manual's answer, not the
// engine's; the reasoning is in ci/hardening/reports/ihp-sg13g2/gatpoly.md.  Counts follow
// the engine's marker cuts where it is right: two per narrow wall pair (min_width and the
// gate-length rules), one per space or enclosure pair, one per area, one per forbidden
// 45° gate edge and one per 90°-bent gate.  GFil.g is ignored throughout (every small
// layout is under 15 % dense); a 300 µm filler bar is a GFil.a by its length and is
// ignored where it serves another rule.
// 0.125 bars in x and y, a 300 µm bar, a 0.005 sliver, a bar at (1000, 1000); 0.13 clean.
#[case::gat_a_h1("gatpoly/Gat.a.h1.gds.gz", "TOP", vec!["Gat.a"; 10], vec!["GFil.g", "Gat.e"])]
// 45°: a 0.127 diamond (4) and strip (2) fire; 0.134 and the chamfered shapes are clean.
// The 3 µm 45° strips are Gat.g by both tools' reading of "45-degree bent shapes".
#[case::gat_a_h2("gatpoly/Gat.a.h2.gds.gz", "TOP", vec!["Gat.a"; 6], vec!["GFil.g", "Gat.e", "Gat.g"])]
// Unions 0.125 wide (overlapping boxes, abutting slices, a dogbone neck, one ring wall, an
// island in a ring) fire once each; 0.13 unions and a 2 × 10 grid are clean.
#[case::gat_a_h3("gatpoly/Gat.a.h3.gds.gz", "TOP", vec!["Gat.a"; 10], vec!["GFil.g"])]
// Ten 0.125 bars on, across and straddling x = 20/21/40/42 plus an L cornered on x = 20.
#[case::gat_a_h4("gatpoly/Gat.a.h4.gds.gz", "TOP", vec!["Gat.a"; 20], vec!["GFil.g"])]
// Fifty 0.125 bars, flat and as a GdsArrayRef.
#[case::gat_a_h5("gatpoly/Gat.a.h5.gds.gz", "TOP", vec!["Gat.a"; 100], vec!["GFil.g"])]
#[case::gat_a_h6("gatpoly/Gat.a.h6.gds.gz", "TOP", vec!["Gat.a"; 100], vec!["GFil.g"])]
// A comb with three 0.125 teeth; a U with 0.13 arms is clean.
#[case::gat_a_h7("gatpoly/Gat.a.h7.gds.gz", "TOP", vec!["Gat.a"; 6], vec!["GFil.g"])]
// NFET = GatPoly over N+Activ, N+ by drawn nSD or by default: 0.125 gates on a bare Activ
// and with nSD drawn are Gat.a1; under nSD:block the Activ is no N+ and the gate none, as
// KLayout reads it (report, note A); the PFET's is Gat.a2, the 3.3 V NFET's Gat.a3.
#[case::gat_a1_h1("gatpoly/Gat.a1.h1.gds.gz", "TOP",
    [vec!["Gat.a1"; 4], vec!["Gat.a2"; 2], vec!["Gat.a3"; 2]].concat(), vec!["GFil.g", "Gat.a"])]
// A dumbbell gate, a notched gate (0.125 over part of the channel; the notch is a Gat.f)
// and two 0.125 fingers of four fire; a 0.125 neck outside the Activ and a 0.5 gate on a
// 0.12 Activ are clean.
#[case::gat_a1_h2("gatpoly/Gat.a1.h2.gds.gz", "TOP",
    [vec!["Gat.a1"; 8], vec!["Gat.f"]].concat(), vec!["GFil.g", "Gat.a"])]
// Eight 0.125 gates on, across and straddling x = 20/21/40/42 and a horizontal one across
// 20; a 0.13 gate straddling 20 is clean.
#[case::gat_a1_h3("gatpoly/Gat.a1.h3.gds.gz", "TOP", vec!["Gat.a1"; 16], vec!["GFil.g", "Gat.a"])]
// Fifty 0.125 NFETs, flat and as a GdsArrayRef.
#[case::gat_a1_h4("gatpoly/Gat.a1.h4.gds.gz", "TOP", vec!["Gat.a1"; 100], vec!["GFil.g", "Gat.a"])]
#[case::gat_a1_h5("gatpoly/Gat.a1.h5.gds.gz", "TOP", vec!["Gat.a1"; 100], vec!["GFil.g", "Gat.a"])]
// A 300 µm wide NFET, one at (1000, 1000), a gate over a 0.005 sliver of Activ.
#[case::gat_a1_h6("gatpoly/Gat.a1.h6.gds.gz", "TOP", vec!["Gat.a1"; 6], vec!["GFil.g", "Gat.a"])]
// A pSD gate is read as a PFET's whether or not a well holds it (report, finding 9 -
// pedantic, KLayout's maximal deck does the same): the PFET, the pSD without a well and
// the pSD crossing the well edge are Gat.a2 at 0.125; the 3.3 V one is Gat.a4.
#[case::gat_a2_h1("gatpoly/Gat.a2.h1.gds.gz", "TOP",
    [vec!["Gat.a2"; 6], vec!["Gat.a4"; 2]].concat(), vec!["GFil.g", "Gat.a"])]
// Two 0.125 fingers of four and a dumbbell PFET gate.
#[case::gat_a2_h2("gatpoly/Gat.a2.h2.gds.gz", "TOP", vec!["Gat.a2"; 6], vec!["GFil.g", "Gat.a"])]
// Fifty 0.125 PFETs, flat and as a GdsArrayRef.
#[case::gat_a2_h3("gatpoly/Gat.a2.h3.gds.gz", "TOP", vec!["Gat.a2"; 100], vec!["GFil.g", "Gat.a"])]
#[case::gat_a2_h4("gatpoly/Gat.a2.h4.gds.gz", "TOP", vec!["Gat.a2"; 100], vec!["GFil.g", "Gat.a"])]
// 0.445 gates under ThickGateOx on a bare Activ and with nSD fire; under nSD:block the
// Activ is no N+; a gate the oxide covers by half is read between the poly's walls only,
// and a TGO edge through a gate is TGO.c's (KLayout reads its 0.25 piece as a 0.25
// gate); 1.2 V and PFET gates at 0.445 are clean.
#[case::gat_a3_h1("gatpoly/Gat.a3.h1.gds.gz", "TOP", vec!["Gat.a3"; 4], vec!["GFil.g"])]
// Two 0.445 fingers of four, a dumbbell, gates straddling/ending on x = 20 and a horizontal
// one across it; 0.45 straddling 20 is clean.
#[case::gat_a3_h2("gatpoly/Gat.a3.h2.gds.gz", "TOP", vec!["Gat.a3"; 12], vec!["GFil.g"])]
// Fifty 0.445 3.3 V NFETs, flat and as a GdsArrayRef.
#[case::gat_a3_h3("gatpoly/Gat.a3.h3.gds.gz", "TOP", vec!["Gat.a3"; 100], vec!["GFil.g"])]
#[case::gat_a3_h4("gatpoly/Gat.a3.h4.gds.gz", "TOP", vec!["Gat.a3"; 100], vec!["GFil.g"])]
// The 3.3 V PFET, the pSD without a well and the pSD crossing the well are Gat.a4 at
// 0.395 (finding 9, as KLayout's maximal deck reads them); the 3.3 V NFET at 0.395 is Gat.a3.
#[case::gat_a4_h1("gatpoly/Gat.a4.h1.gds.gz", "TOP",
    [vec!["Gat.a3"; 2], vec!["Gat.a4"; 6]].concat(), vec!["GFil.g"])]
// Two 0.395 fingers of four, a dumbbell, gates straddling/ending on x = 20 and a horizontal
// one across it; 0.40 straddling 20 is clean.
#[case::gat_a4_h2("gatpoly/Gat.a4.h2.gds.gz", "TOP", vec!["Gat.a4"; 12], vec!["GFil.g"])]
// Fifty 0.395 3.3 V PFETs, flat and as a GdsArrayRef.
#[case::gat_a4_h3("gatpoly/Gat.a4.h3.gds.gz", "TOP", vec!["Gat.a4"; 100], vec!["GFil.g"])]
#[case::gat_a4_h4("gatpoly/Gat.a4.h4.gds.gz", "TOP", vec!["Gat.a4"; 100], vec!["GFil.g"])]
// Gap 0.175, a 0.125/0.125 diagonal (0.177), a corner-on 0.175 and a vertical 0.175 fire;
// 0.18, the 0.13/0.13 diagonal (0.184) and 0.18 half-overlapping are clean.
#[case::gat_b_h1("gatpoly/Gat.b.h1.gds.gz", "TOP", vec!["Gat.b"; 4], vec!["GFil.g"])]
// 45°: diamond tip to wall 0.175, two 45° strips 0.173, chamfer to corner 0.177, tip to tip
// 0.175 fire; 0.18/0.184 controls are clean.
#[case::gat_b_h2("gatpoly/Gat.b.h2.gds.gz", "TOP", vec!["Gat.b"; 4], vec!["GFil.g"])]
// "Space or notch": a U slot, three comb slots, a slot in a plate, two facing Ls, a ring
// hole and an island in a ring, all 0.175.
#[case::gat_b_h3("gatpoly/Gat.b.h3.gds.gz", "TOP", vec!["Gat.b"; 8], vec!["GFil.g"])]
// Two unions 0.175 apart and a grid 0.175 from a bar fire once each; 0.175 gaps on, across
// and straddling x = 20/21/40/42, a horizontal gap across 20 and a corner pair on (20, 20).
#[case::gat_b_h4("gatpoly/Gat.b.h4.gds.gz", "TOP", vec!["Gat.b"; 10], vec!["GFil.g"])]
// Fifty 0.175 gaps, flat and as a GdsArrayRef.
#[case::gat_b_h5("gatpoly/Gat.b.h5.gds.gz", "TOP", vec!["Gat.b"; 50], vec!["GFil.g"])]
#[case::gat_b_h6("gatpoly/Gat.b.h6.gds.gz", "TOP", vec!["Gat.b"; 50], vec!["GFil.g"])]
// Two gate fingers 0.175 apart, a poly 0.175 past a gate's cap, a sliver 0.175 from a bar,
// two 300 µm bars, a pair at (1000, 1000).
#[case::gat_b_h7("gatpoly/Gat.b.h7.gds.gz", "TOP", vec!["Gat.b"; 5], vec!["GFil.g", "Gat.a", "Gat.e"])]
// 3.3 V fingers 0.245 apart fire, 0.25 is clean, 1.2 V fingers at 0.245 are clean, a 3.3 V
// finger beside one outside the oxide is clean; 0.175 apart is a Gat.b1 and a Gat.b.
#[case::gat_b1_h1("gatpoly/Gat.b1.h1.gds.gz", "TOP", vec!["Gat.b1", "Gat.b1", "Gat.b"], vec!["GFil.g"])]
// Gat.b1 is read between gate regions, as both tools read it (report, finding 8 - open):
// two 3.3 V transistors whose polys end 0.245 apart have gate regions 0.845 apart and are
// clean; one poly over two Activs 0.245 apart fires; 0.25 is clean.
#[case::gat_b1_h2("gatpoly/Gat.b1.h2.gds.gz", "TOP", vec!["Gat.b1"; 1], vec!["GFil.g"])]
// A U poly whose legs make two gate regions 0.245 apart fires; a U whose base lies over the
// Activ is one region (and a Gat.f).
#[case::gat_b1_h3("gatpoly/Gat.b1.h3.gds.gz", "TOP", vec!["Gat.b1"], vec!["GFil.g", "Gat.f"])]
// 0.245 finger gaps straddling/ending on x = 20/21/40/42, at (1000, 1000), and two 300 µm
// gate regions 0.245 apart.
#[case::gat_b1_h4("gatpoly/Gat.b1.h4.gds.gz", "TOP", vec!["Gat.b1"; 7], vec!["GFil.g"])]
// Fifty 0.245 finger pairs, flat and as a GdsArrayRef.
#[case::gat_b1_h5("gatpoly/Gat.b1.h5.gds.gz", "TOP", vec!["Gat.b1"; 50], vec!["GFil.g"])]
#[case::gat_b1_h6("gatpoly/Gat.b1.h6.gds.gz", "TOP", vec!["Gat.b1"; 50], vec!["GFil.g"])]
// 0.175 caps: bottom, both ends (2), a horizontal gate's left, a gate on a 300 µm Activ;
// 0.18 caps are clean.
#[case::gat_c_h1("gatpoly/Gat.c.h1.gds.gz", "TOP", vec!["Gat.c"; 5], vec!["GFil.g"])]
// No cap: a gate ending on the Activ edge, a poly wholly inside (a `forbidden` on the
// polys inside, as KLayout's `GatPoly.inside(Activ)`), a poly over an Activ corner
// extending 0.175, a gate side on the Activ edge; a stub ending inside the Activ has no
// cap to read in either tool (report, finding 3); 0.5 over the corner is clean.
#[case::gat_c_h2("gatpoly/Gat.c.h2.gds.gz", "TOP", vec!["Gat.c"; 4], vec!["GFil.g"])]
// A gate over two Activs 0.175 short and five 0.175 caps on and across x = 20/40/42 fire;
// a 0.375 T head and a 0.19 chamfer are clean.  A cap chamfered down to 0.10 at its
// corner is not read: the chamfer is no wall parallel to the Activ's edge, and IHP's
// `ext_enclosed` (projection) does pair it at 0.141 - report, finding 11, open with the
// settled projection reading.
#[case::gat_c_h3("gatpoly/Gat.c.h3.gds.gz", "TOP", vec!["Gat.c"; 6], vec!["GFil.g"])]
// Fifty 0.175 caps, flat and as a GdsArrayRef.
#[case::gat_c_h4("gatpoly/Gat.c.h4.gds.gz", "TOP", vec!["Gat.c"; 50], vec!["GFil.g"])]
#[case::gat_c_h5("gatpoly/Gat.c.h5.gds.gz", "TOP", vec!["Gat.c"; 50], vec!["GFil.g"])]
// A 0.175 cap at (1000, 1000), across an Activ seam, and a gate drawn as two boxes (once);
// under SRAM the deck does not check.
#[case::gat_c_h6("gatpoly/Gat.c.h6.gds.gz", "TOP", vec!["Gat.c"; 3], vec!["GFil.g"])]
// 0.065 beside, a 0.045/0.045 diagonal (0.0636), corner-on, above and below fire; 0.07 and
// the 0.05/0.05 diagonal (0.0707) are clean.
#[case::gat_d_h1("gatpoly/Gat.d.h1.gds.gz", "TOP", vec!["Gat.d"; 5], vec!["GFil.g"])]
// 45°: diamond tip 0.065, a 45° wall 0.0636 from an Activ corner, a box corner 0.0636 from
// an Activ chamfer, parallel 45° walls 0.0636 apart fire; 0.07/0.0707 controls are clean.
#[case::gat_d_h2("gatpoly/Gat.d.h2.gds.gz", "TOP", vec!["Gat.d"; 4], vec!["GFil.g"])]
// A poly abutting the Activ edge (space 0.00, `abutting: report`) and one touching its
// corner fire; a gate 0.065 past a second Activ, a gate 0.065 short of its Activ, a poly
// 0.065 from both arms of a U (one pair, the U being one region); a poly overlapping the
// Activ by 0.05 is a gate (Gat.c, not Gat.d).
#[case::gat_d_h3("gatpoly/Gat.d.h3.gds.gz", "TOP", vec!["Gat.d"; 5], vec!["GFil.g", "Gat.c"])]
// 0.065 gaps on, across and straddling x = 20/21/40/42, a horizontal one across 20, a
// corner pair on (20, 20), one at (1000, 1000), a 300 µm pair; 0.07 across 20 is clean.
#[case::gat_d_h4("gatpoly/Gat.d.h4.gds.gz", "TOP", vec!["Gat.d"; 10], vec!["GFil.g"])]
// Fifty 0.065 gaps, flat and as a GdsArrayRef.
#[case::gat_d_h5("gatpoly/Gat.d.h5.gds.gz", "TOP", vec!["Gat.d"; 50], vec!["GFil.g"])]
#[case::gat_d_h6("gatpoly/Gat.d.h6.gds.gz", "TOP", vec!["Gat.d"; 50], vec!["GFil.g"])]
// 0.0885, a 0.0897 bar, a 0.0882 diamond, a 0.0885 union and a 0.0884 ring fire; 0.09 (a
// box, a union, abutting halves, a grid) and an L of 0.1001 are clean.  The ring's 0.04
// hole is a Gat.b notch.
#[case::gat_e_h1("gatpoly/Gat.e.h1.gds.gz", "TOP", vec!["Gat.e"; 5], vec!["GFil.g", "Gat.b"])]
// 0.0885 squares straddling x = 20, starting on 20, straddling 40 and 42, one at
// (1000, 1000), a 0.0897 bar across 20; 0.09 squares across tile lines are clean.
#[case::gat_e_h2("gatpoly/Gat.e.h2.gds.gz", "TOP", vec!["Gat.e"; 6], vec!["GFil.g"])]
// Fifty 0.0885 squares, flat and as a GdsArrayRef.
#[case::gat_e_h3("gatpoly/Gat.e.h3.gds.gz", "TOP", vec!["Gat.e"; 50], vec!["GFil.g"])]
#[case::gat_e_h4("gatpoly/Gat.e.h4.gds.gz", "TOP", vec!["Gat.e"; 50], vec!["GFil.g"])]
// 90° bends over the Activ: an L gate, a T stub, a notched gate (also Gat.a1) and a gate
// stepping from 0.3 to 0.5 inside the Activ.
#[case::gat_f_h1("gatpoly/Gat.f.h1.gds.gz", "TOP", vec!["Gat.f"; 4], vec!["GFil.g", "Gat.a1", "Gat.a"])]
// Gat.f is the gate region that is no rectangle: the L gate whose bend is 0.005 inside
// the Activ fires, and so does a straight gate over a stepped Activ (its gate region has
// the step), as KLayout has it (report, note E); a bend 0.07 outside, a chamfer outside
// and a 45° poly beside are clean.
#[case::gat_f_h2("gatpoly/Gat.f.h2.gds.gz", "TOP", vec!["Gat.f"; 2], vec!["GFil.g"])]
// 45° over the Activ: a 45° bend inside, a 45° jog inside, a band clipping an Activ
// corner, one each; a 45° bend outside is clean.
#[case::gat_f_h3("gatpoly/Gat.f.h3.gds.gz", "TOP", vec!["Gat.f"; 3], vec!["GFil.g", "Gat.a", "Gat.c", "Gat.e"])]
// L gates cornered on x = 20 and 40, straddling 42, at (1000, 1000), a 45° gate across
// x = 20, one each; a straight gate across 20 is clean.
#[case::gat_f_h4("gatpoly/Gat.f.h4.gds.gz", "TOP", vec!["Gat.f"; 5], vec!["GFil.g", "Gat.a", "Gat.c", "Gat.e"])]
// Fifty L gates, flat and as a GdsArrayRef.
#[case::gat_f_h5("gatpoly/Gat.f.h5.gds.gz", "TOP", vec!["Gat.f"; 50], vec!["GFil.g"])]
#[case::gat_f_h6("gatpoly/Gat.f.h6.gds.gz", "TOP", vec!["Gat.f"; 50], vec!["GFil.g"])]
// 0.1591-wide 45° bands with walls 0.636, 0.396, 0.552, 0.566 and 0.544 long fire (two
// walls each); 0.1626 wide, and 0.1591 with 0.389 walls, are clean.
#[case::gat_g_h1("gatpoly/Gat.g.h1.gds.gz", "TOP", vec!["Gat.g"; 10], vec!["GFil.g", "Gat.a", "Gat.e"])]
// Z routes 0.155 wide with a 0.1556 jog 0.636 long fire, up and mirrored down; a 0.354 jog
// and a 0.1626 jog are clean; an L with a chamfered corner 0.1556 across, outer 45° edge
// 0.495 and inner 0.368, has one wall under 0.39 and is clean, as KLayout reads it
// (report, note G).
#[case::gat_g_h2("gatpoly/Gat.g.h2.gds.gz", "TOP", vec!["Gat.g"; 4], vec!["GFil.g"])]
// The firing Z route with its jog straddling x = 20, starting on 20, across 40 and 42, at
// (1000, 1000).
#[case::gat_g_h3("gatpoly/Gat.g.h3.gds.gz", "TOP", vec!["Gat.g"; 10], vec!["GFil.g"])]
// Fifty firing Z routes, flat and as a GdsArrayRef.
#[case::gat_g_h4("gatpoly/Gat.g.h4.gds.gz", "TOP", vec!["Gat.g"; 100], vec!["GFil.g"])]
#[case::gat_g_h5("gatpoly/Gat.g.h5.gds.gz", "TOP", vec!["Gat.g"; 100], vec!["GFil.g"])]
// The width is the narrowest dimension (`span: narrowest`, KLayout's 2.5 shrink): only
// the 5.005 × 5.005 box and the 300 × 5.005 bar are over 5.00 in every direction, one
// report each; 5.005 × 5, the 300 × 5 bar, the union, the L, the ring and the 5.005 × 5
// squares across the tile lines are 5.0 wide and clean (report, note B).
#[case::gfil_a_h1("gatpoly/GFil.a.h1.gds.gz", "TOP", vec!["GFil.a"; 2], vec!["GFil.g"])]
// 45°: a diamond 5.02 across both diagonals fires, once; a strip 5.02 wide by 4.24 and a
// strip 5.09 long by 3.5 are under 5.00 across, and clean, like 4.95 and a chamfered
// 5 × 5.
#[case::gfil_a_h2("gatpoly/GFil.a.h2.gds.gz", "TOP", vec!["GFil.a"; 1], vec!["GFil.g"])]
// Fifty 5.005 × 5.005 boxes, flat and as a GdsArrayRef.
#[case::gfil_a_h3("gatpoly/GFil.a.h3.gds.gz", "TOP", vec!["GFil.a"; 50], vec!["GFil.g"])]
#[case::gfil_a_h4("gatpoly/GFil.a.h4.gds.gz", "TOP", vec!["GFil.a"; 50], vec!["GFil.g"])]
// A 300 µm bar merged with a ring below it and one merged with a box: one report per
// merged shape, whatever the tile size (report, finding 10).
#[case::gfil_a_h5("gatpoly/GFil.a.h5.gds.gz", "TOP", vec!["GFil.a"; 2], vec!["GFil.g"])]
// 0.695 in x and y, a 0.693 diamond (4) and strip (2), a 0.695 union, a 0.695 bar across
// x = 20, a 300 µm bar, one at (1000, 1000); 0.70 and 0.707 shapes are clean.
#[case::gfil_b_h1("gatpoly/GFil.b.h1.gds.gz", "TOP", vec!["GFil.b"; 18], vec!["GFil.g", "GFil.a"])]
// Fifty 0.695 bars, flat and as a GdsArrayRef.
#[case::gfil_b_h2("gatpoly/GFil.b.h2.gds.gz", "TOP", vec!["GFil.b"; 100], vec!["GFil.g"])]
#[case::gfil_b_h3("gatpoly/GFil.b.h3.gds.gz", "TOP", vec!["GFil.b"; 100], vec!["GFil.g"])]
// 0.795, a 0.56/0.56 diagonal (0.792), a diamond tip 0.795 from a wall, an island 0.795
// from a ring, gaps across x = 20 and 40, one at (1000, 1000), 300 µm bars fire; 0.80,
// 0.806 are clean, and so is a 0.795 U slot: the manual says "space", not "space or notch".
#[case::gfil_c_h1("gatpoly/GFil.c.h1.gds.gz", "TOP", vec!["GFil.c"; 8], vec!["GFil.g", "GFil.a"])]
// Fifty 0.795 gaps, flat and as a GdsArrayRef.
#[case::gfil_c_h2("gatpoly/GFil.c.h2.gds.gz", "TOP", vec!["GFil.c"; 50], vec!["GFil.g"])]
#[case::gfil_c_h3("gatpoly/GFil.c.h3.gds.gz", "TOP", vec!["GFil.c"; 50], vec!["GFil.g"])]
// Per layer (Activ, GatPoly, Cont, pSD, nSD:block, SalBlock): 1.095 and a 0.77/0.77
// diagonal (1.089) fire, 1.10 and 1.103 are clean.
#[case::gfil_d_h1("gatpoly/GFil.d.h1.gds.gz", "TOP", vec!["GFil.d"; 12], vec!["GFil.g"])]
// A filler abutting an Activ (space 0.00, `abutting: report`), 1.095 across x = 20 and
// 40, a gap ending on 20, a corner pair near (20, 14), one at (1000, 1000), a 300 µm pair;
// a filler overlapping a pSD, under a pSD or overlapping a GatPoly shares area and is no
// pair, as KLayout has it (report, finding 5).
#[case::gfil_d_h2("gatpoly/GFil.d.h2.gds.gz", "TOP", vec!["GFil.d"; 8], vec!["GFil.g", "GFil.a"])]
// Fifty 1.095 filler-to-Activ gaps, flat and as a GdsArrayRef.
#[case::gfil_d_h3("gatpoly/GFil.d.h3.gds.gz", "TOP", vec!["GFil.d"; 50], vec!["GFil.g"])]
#[case::gfil_d_h4("gatpoly/GFil.d.h4.gds.gz", "TOP", vec!["GFil.d"; 50], vec!["GFil.g"])]
// To NWell and to nBuLay: 1.095 and 1.089 fire, 1.10 and 1.103 are clean; fillers 1.6 from
// a 5 × 5 well and 1.5 from a 2-wide well are clean (nBuLay derives inward).
#[case::gfil_e_h1("gatpoly/GFil.e.h1.gds.gz", "TOP", vec!["GFil.e"; 4], vec!["GFil.g"])]
// A filler abutting an NWell (space 0.00), 1.095 across x = 20 and 40, one at (1000,
// 1000), a 300 µm pair; a filler inside an NWell or half over an nBuLay shares area and is
// no pair (finding 5).
#[case::gfil_e_h2("gatpoly/GFil.e.h2.gds.gz", "TOP", vec!["GFil.e"; 5], vec!["GFil.g", "GFil.a"])]
// Fifty 1.095 filler-to-NWell gaps, flat and as a GdsArrayRef.
#[case::gfil_e_h3("gatpoly/GFil.e.h3.gds.gz", "TOP", vec!["GFil.e"; 50], vec!["GFil.g"])]
#[case::gfil_e_h4("gatpoly/GFil.e.h4.gds.gz", "TOP", vec!["GFil.e"; 50], vec!["GFil.g"])]
// 1.095 and 1.089 to TRANS, a filler abutting a TRANS, 1.095 across x = 20 and 40, one at
// (1000, 1000), a 300 µm pair; 1.10 and 1.103 are clean, and a filler inside a TRANS is
// no pair (finding 5).
#[case::gfil_f_h1("gatpoly/GFil.f.h1.gds.gz", "TOP", vec!["GFil.f"; 7], vec!["GFil.g", "GFil.a"])]
// Fifty 1.095 filler-to-TRANS gaps, flat and as a GdsArrayRef.
#[case::gfil_f_h2("gatpoly/GFil.f.h2.gds.gz", "TOP", vec!["GFil.f"; 50], vec!["GFil.g"])]
#[case::gfil_f_h3("gatpoly/GFil.f.h3.gds.gz", "TOP", vec!["GFil.f"; 50], vec!["GFil.g"])]
// A 10 % GatPoly stripe under a coincident 10 % filler stripe is 10 % of GatPoly, under
// the 15 % floor.
#[case::gfil_g_h1("gatpoly/GFil.g.h1.gds.gz", "TOP", vec!["GFil.g"], vec!["GFil.a", "GFil.d"])]
// The manual gives an area: 400.005 × 400 and a 500 × 400 union fire; 400 × 400, 800 × 199
// and an L in a 500 × 500 box are clean.
#[case::gfil_i_h1("gatpoly/GFil.i.h1.gds.gz", "TOP", vec!["GFil.i"; 2], vec!["GFil.g"])]
// 0.175 caps (bottom; both ends: 2; horizontal; across x = 20; at (1000, 1000)), a filler
// ending on the Activ:filler edge and a filler wholly inside (a `forbidden`); a stub
// ending inside has no cap to read (finding 3); 0.18 is clean.
#[case::gfil_j_h1("gatpoly/GFil.j.h1.gds.gz", "TOP", vec!["GFil.j"; 8], vec!["GFil.g"])]
// Fifty 0.175 caps, flat and as a GdsArrayRef.
#[case::gfil_j_h2("gatpoly/GFil.j.h2.gds.gz", "TOP", vec!["GFil.j"; 50], vec!["GFil.g"])]
#[case::gfil_j_h3("gatpoly/GFil.j.h3.gds.gz", "TOP", vec!["GFil.j"; 50], vec!["GFil.g"])]
fn test_gatpoly(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_GAT, gds, topcell, &ignore), expected);
}

// --- Cont ---

const DECK_CNT: &str = "cont";

#[rstest]
#[case::cnt_a("cont/Cnt.a.gds.gz", "TOP", vec!["Cnt.a"; 8], vec!["Cnt.c", "Cnt.d", "Cnt.g", "Cnt.h"])]
#[case::cnt_b("cont/Cnt.b.gds.gz", "TOP", vec!["Cnt.b", "Cnt.b"], vec!["Cnt.c", "Cnt.d", "Cnt.g", "Cnt.h"])]
#[case::cnt_e("cont/Cnt.e.gds.gz", "TOP", vec!["Cnt.e"], vec!["Cnt.c", "Cnt.h"])]
#[case::cnt_f("cont/Cnt.f.gds.gz", "TOP", vec!["Cnt.f"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_g("cont/Cnt.g.gds.gz", "TOP", vec!["Cnt.g"], vec!["Cnt.c", "Cnt.d", "Cnt.h"])]
#[case::cnt_g1("cont/Cnt.g1.gds.gz", "TOP", vec!["Cnt.g1"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_g2("cont/Cnt.g2.gds.gz", "TOP", vec!["Cnt.g2"], vec!["Cnt.d", "Cnt.h"])]
#[case::cnt_h("cont/Cnt.h.gds.gz", "TOP", vec!["Cnt.h"], vec!["Cnt.c", "Cnt.d", "Cnt.g"])]
#[case::cnt_j("cont/Cnt.j.gds.gz", "TOP", vec!["Cnt.j"], vec!["Cnt.c", "Cnt.h"])]
// --- Hardening (ci/hardening/SPEC.md): expected values are the manual's answer, not the
// engine's; the reasoning is in ci/hardening/reports/ihp-sg13g2/cont.md.  Every layout
// carries Activ/GatPoly and Metal1 under its Conts where the rule under test does not
// need them bare, so no case ignores a rule.  Cnt.a counts four walls per off-size
// square; an enclosure rule counts one marker per under-enclosed Cont (its violating
// walls are one connected run); a space rule one per pair.
// A 0.16 square drawn six ways is clean; a clockwise 0.155, a 0.155 and a 0.165 square
// fail four walls each; a 0.16 × 0.165 rectangle and a notched square are ContBars.
#[case::cnt_a_h1("cont/Cnt.a.h1.gds.gz", "TOP", vec!["Cnt.a"; 12], vec![])]
// 0.155 squares straddling x = 20/21/40/42, y = 20, ending on x = 20 and at (1000, 1000).
#[case::cnt_a_h2("cont/Cnt.a.h2.gds.gz", "TOP", vec!["Cnt.a"; 28], vec![])]
// Fifty 0.155 squares, flat and as a GdsArrayRef.
#[case::cnt_a_h3("cont/Cnt.a.h3.gds.gz", "TOP", vec!["Cnt.a"; 200], vec![])]
#[case::cnt_a_h4("cont/Cnt.a.h4.gds.gz", "TOP", vec!["Cnt.a"; 200], vec![])]
// 0.175 in x, in y, 0.177 corner to corner, and a row of three (two pairs); 0.18, 0.184
// diagonal and 0.10/0.15 (0.180 euclidian) are clean.
#[case::cnt_b_h1("cont/Cnt.b.h1.gds.gz", "TOP", vec!["Cnt.b"; 5], vec![])]
// 0.175 gaps straddling, starting on and ending on the tile lines, and at (1000, 1000).
#[case::cnt_b_h2("cont/Cnt.b.h2.gds.gz", "TOP", vec!["Cnt.b"; 9], vec![])]
// Fifty 0.175 pairs, flat and as a GdsArrayRef.
#[case::cnt_b_h3("cont/Cnt.b.h3.gds.gz", "TOP", vec!["Cnt.b"; 50], vec![])]
#[case::cnt_b_h4("cont/Cnt.b.h4.gds.gz", "TOP", vec!["Cnt.b"; 50], vec![])]
// Two squares touching at a corner merge into one non-square Cont, which is the contbar
// deck's (CntB.a, CntB.a1, CntB.b) and not Cnt.b's - KLayout lets the pair through
// altogether; a 0.155 square 0.175 from a 0.16 one is Cnt.b and four Cnt.a; a square
// 0.175 from a bar is CntB.b2's (report, finding 6).
#[case::cnt_b_h5("cont/Cnt.b.h5.gds.gz", "TOP", vec!["Cnt.a", "Cnt.a", "Cnt.a", "Cnt.a", "Cnt.b"], vec![])]
// 5 × 5 at 0.18/0.18 and at 0.195/0.195 fire; 5 × 5 with 0.20 in one direction, 4 × 5,
// 5 × 4 and 4 × 4 at 0.18 are clean.
#[case::cnt_b1_h1("cont/Cnt.b1.h1.gds.gz", "TOP", vec!["Cnt.b1"; 2], vec![])]
// 6 × 6, 5 × 10, a 5 × 5 whose row gaps alternate 0.18/0.20 (no direction relaxed
// throughout), a staggered 5 × 5 (0.1803 corner to corner between rows) and a 5 × 5
// drawn as a 3- and a 2-column block fire; a 5 × 5 missing its centre has a row of two
// runs of two and is no array of five, as KLayout has it (report, findings 2, 3).
#[case::cnt_b1_h2("cont/Cnt.b1.h2.gds.gz", "TOP", vec!["Cnt.b1"; 5], vec![])]
// 5 × 5 arrays at 0.18 well inside a tile, straddling x = 20/21/40/42/14, the corner
// (20, 20), and at (1000, 1000).
#[case::cnt_b1_h3("cont/Cnt.b1.h3.gds.gz", "TOP", vec!["Cnt.b1"; 8], vec![])]
// Fifty 5 × 5 arrays at 0.18, flat and as a GdsArrayRef of a cell holding one array.
#[case::cnt_b1_h4("cont/Cnt.b1.h4.gds.gz", "TOP", vec!["Cnt.b1"; 50], vec![])]
#[case::cnt_b1_h5("cont/Cnt.b1.h5.gds.gz", "TOP", vec!["Cnt.b1"; 50], vec![])]
// A 5 × 5 as a GdsArrayRef of a one-Cont cell at 0.18 gaps fires; a flat 5 × 5 at 0.175
// both ways is Cnt.b1 and forty Cnt.b; a 5 × 5 with 0.20 in x is clean.
#[case::cnt_b1_h6("cont/Cnt.b1.h6.gds.gz", "TOP",
    [vec!["Cnt.b1"; 2], vec!["Cnt.b"; 40]].concat(), vec![])]
// Activ margins: 0.07 clean; 0.065 right, 0.065 all round, 0.065 right and top, 0.005,
// 0 (Cont edge on the Activ edge) fire once each; a Cont 0.05 past the edge is Cnt.c
// and Cnt.g.
#[case::cnt_c_h1("cont/Cnt.c.h1.gds.gz", "TOP", vec!["Cnt.c", "Cnt.c", "Cnt.c", "Cnt.c", "Cnt.c", "Cnt.c", "Cnt.g"], vec![])]
// A chamfer and a 45° wall passing 0.064 from the Cont's corner with both axis margins
// fine: clean by the settled projection reading (report, note B).
#[case::cnt_c_h2("cont/Cnt.c.h2.gds.gz", "TOP", vec![], vec![])]
// Unions (abutting, overlapping, a 4 × 4 grid) enclose by their union; a union with
// 0.065, an Activ drawn twice with 0.065 (once) and a ring wall 0.065 from its hole fire.
#[case::cnt_c_h3("cont/Cnt.c.h3.gds.gz", "TOP", vec!["Cnt.c"; 3], vec![])]
// 0.065 margins straddling, ending on and beginning on the tile lines; 0.07 across x = 20 is clean.
#[case::cnt_c_h4("cont/Cnt.c.h4.gds.gz", "TOP", vec!["Cnt.c"; 8], vec![])]
// Fifty Conts with 0.065 on the right, flat and as a GdsArrayRef.
#[case::cnt_c_h5("cont/Cnt.c.h5.gds.gz", "TOP", vec!["Cnt.c"; 50], vec![])]
#[case::cnt_c_h6("cont/Cnt.c.h6.gds.gz", "TOP", vec!["Cnt.c"; 50], vec![])]
// Three Conts on a 300 µm strip 0.065 top and bottom (two walls each), one at (1000, 1000).
#[case::cnt_c_h7("cont/Cnt.c.h7.gds.gz", "TOP", vec!["Cnt.c"; 7], vec![])]
// Section 8.1.2: under DigiBnd 0.05 is the value - 0.045 fires the digital variant (twice:
// once inside a tile, once straddling x = 40), 0.05 and 0.065 are clean; 0.065 outside
// DigiBnd and in a DigiBnd frame's hole fire Cnt.c; the DigiBnd edge through the Activ
// or through the Cont, and a DigiBnd inside the Cont, are digital and clean (finding 1).
#[case::cnt_c_h8("cont/Cnt.c.h8.gds.gz", "TOP", vec!["Cnt.c", "Cnt.c", "Cnt.c.dig", "Cnt.c.dig"], vec![])]
// GatPoly margins, as Cnt.c.h1.
#[case::cnt_d_h1("cont/Cnt.d.h1.gds.gz", "TOP", vec!["Cnt.d", "Cnt.d", "Cnt.d", "Cnt.d", "Cnt.d", "Cnt.d", "Cnt.g"], vec![])]
// 45° cuts of the GatPoly near the Cont's corner: clean by the settled projection reading.
#[case::cnt_d_h2("cont/Cnt.d.h2.gds.gz", "TOP", vec![], vec![])]
// Merged GatPoly, as Cnt.c.h3.
#[case::cnt_d_h3("cont/Cnt.d.h3.gds.gz", "TOP", vec!["Cnt.d"; 3], vec![])]
// Tile lines, as Cnt.c.h4.
#[case::cnt_d_h4("cont/Cnt.d.h4.gds.gz", "TOP", vec!["Cnt.d"; 8], vec![])]
// Fifty Conts with 0.065 of GatPoly on the right, flat and as a GdsArrayRef.
#[case::cnt_d_h5("cont/Cnt.d.h5.gds.gz", "TOP", vec!["Cnt.d"; 50], vec![])]
#[case::cnt_d_h6("cont/Cnt.d.h6.gds.gz", "TOP", vec!["Cnt.d"; 50], vec![])]
// 300 µm GatPoly strips and (1000, 1000), as Cnt.c.h7.
#[case::cnt_d_h7("cont/Cnt.d.h7.gds.gz", "TOP", vec!["Cnt.d"; 7], vec![])]
// A gate contact with 0.065 of poly is Cnt.d and Cnt.j; a Cont straddling the seam of an
// abutting GatPoly and Activ is enclosed by neither (Cnt.d, Cnt.c) and 0 from each
// (Cnt.e, Cnt.f), covered by their union (no Cnt.g), overlapping nothing (no Cnt.j); a
// Cont half on poly and half on nothing is Cnt.d and Cnt.g (finding 4).
#[case::cnt_d_h8("cont/Cnt.d.h8.gds.gz", "TOP",
    vec!["Cnt.c", "Cnt.d", "Cnt.d", "Cnt.d", "Cnt.e", "Cnt.f", "Cnt.g", "Cnt.j"], vec![])]
// Activ 0.135 from a gate contact, 0.134 corner to corner, 0.134 corner to a 45° wall,
// abutting (0: finding 4), the gate's own Activ at 0.135, and Activ 0.135 on both sides
// (two); 0.14, 0.141 diagonal, 0.144 (0.12 in the axis), the 0.141 wall and 0.22 are clean.
#[case::cnt_e_h1("cont/Cnt.e.h1.gds.gz", "TOP", vec!["Cnt.e"; 7], vec![])]
// 0.135 gaps straddling x = 20 four ways, x = 21/40/42/14 and y = 20; 0.14 across x = 20 is clean.
#[case::cnt_e_h2("cont/Cnt.e.h2.gds.gz", "TOP", vec!["Cnt.e"; 9], vec![])]
// Fifty gate contacts 0.135 from Activ, flat and as a GdsArrayRef.
#[case::cnt_e_h3("cont/Cnt.e.h3.gds.gz", "TOP", vec!["Cnt.e"; 50], vec![])]
#[case::cnt_e_h4("cont/Cnt.e.h4.gds.gz", "TOP", vec!["Cnt.e"; 50], vec![])]
// A 300 µm Activ strip 0.135 from a gate contact, one at (1000, 1000); a ContBar on poly
// 0.135 from Activ is CntB.e's.
#[case::cnt_e_h5("cont/Cnt.e.h5.gds.gz", "TOP", vec!["Cnt.e"; 2], vec![])]
// GatPoly 0.105 from a diffusion contact, 0.106 corner to corner, 0.106 corner to a 45°
// wall, abutting (0: finding 4), a gate at 0.105, two gates at 0.105 (two), a field poly
// at 0.105; 0.11, 0.113 diagonal, 0.114 (0.09 in the axis) and the 0.113 wall are clean.
#[case::cnt_f_h1("cont/Cnt.f.h1.gds.gz", "TOP", vec!["Cnt.f"; 8], vec![])]
// 0.105 gaps on and across the tile lines; 0.11 across x = 20 is clean.
#[case::cnt_f_h2("cont/Cnt.f.h2.gds.gz", "TOP", vec!["Cnt.f"; 9], vec![])]
// Fifty diffusion contacts 0.105 from GatPoly, flat and as a GdsArrayRef.
#[case::cnt_f_h3("cont/Cnt.f.h3.gds.gz", "TOP", vec!["Cnt.f"; 50], vec![])]
#[case::cnt_f_h4("cont/Cnt.f.h4.gds.gz", "TOP", vec!["Cnt.f"; 50], vec![])]
// A 300 µm GatPoly strip 0.105 from a diffusion contact, one at (1000, 1000); a ContBar
// on Activ 0.105 from GatPoly is CntB.f's.
#[case::cnt_f_h5("cont/Cnt.f.h5.gds.gz", "TOP", vec!["Cnt.f"; 2], vec![])]
// A bare Cont, one 0.005 past the Activ edge and one half outside (those two also Cnt.c,
// enclosed by 0), one in an Activ ring's hole, one abutting Activ from outside and one at
// (1000, 1000) fire; Conts in Activ, in GatPoly, over abutting Activ boxes and over four
// quadrants are within; a Cont on Activ under GatPoly is Cnt.j, not Cnt.g.
#[case::cnt_g_h1("cont/Cnt.g.h1.gds.gz", "TOP",
    vec!["Cnt.c", "Cnt.c", "Cnt.g", "Cnt.g", "Cnt.g", "Cnt.g", "Cnt.g", "Cnt.g", "Cnt.j"], vec![])]
// Bare Conts straddling the tile lines (six) and a Cont straddling x = 20 whose Activ
// ends on the line (Cnt.g and Cnt.c); a Cont ending on x = 20 with its Activ ending there
// is within, but enclosed by 0 (Cnt.c).
#[case::cnt_g_h2("cont/Cnt.g.h2.gds.gz", "TOP",
    [vec!["Cnt.c"; 2], vec!["Cnt.g"; 7]].concat(), vec![])]
// Fifty bare Conts, flat and as a GdsArrayRef.
#[case::cnt_g_h3("cont/Cnt.g.h3.gds.gz", "TOP", vec!["Cnt.g"; 50], vec![])]
#[case::cnt_g_h4("cont/Cnt.g.h4.gds.gz", "TOP", vec!["Cnt.g"; 50], vec![])]
// pSD 0.085 from a Cont on nSD-Activ, 0.085 corner to corner, 0.085 corner to a 45° wall,
// abutting (0: finding 4), 0.085 from a Cont on plain Activ (section 4.2: nSD-Activ,
// finding 5) and from a well tie; 0.09, 0.092 diagonal, 0.094 (0.08 in the axis), the
// 0.092 wall and a Cont on Activ under nSD:block are clean.
#[case::cnt_g1_h1("cont/Cnt.g1.h1.gds.gz", "TOP", vec!["Cnt.g1"; 6], vec![])]
// 0.085 gaps on and across the tile lines; 0.09 across x = 20 is clean.
#[case::cnt_g1_h2("cont/Cnt.g1.h2.gds.gz", "TOP", vec!["Cnt.g1"; 9], vec![])]
// Fifty Conts on nSD-Activ 0.085 from pSD, flat and as a GdsArrayRef.
#[case::cnt_g1_h3("cont/Cnt.g1.h3.gds.gz", "TOP", vec!["Cnt.g1"; 50], vec![])]
#[case::cnt_g1_h4("cont/Cnt.g1.h4.gds.gz", "TOP", vec!["Cnt.g1"; 50], vec![])]
// A 300 µm pSD strip 0.085 from a Cont on nSD-Activ, one at (1000, 1000); a ContBar on
// nSD-Activ 0.085 from pSD is CntB.g1's.
#[case::cnt_g1_h5("cont/Cnt.g1.h5.gds.gz", "TOP", vec!["Cnt.g1"; 2], vec![])]
// pSD margins: 0.09 clean; 0.085 right, all round, right and top, 0.005, 0, and a pSD
// ending on the Activ edge 0.07 from the Cont fire once each; the pSD edge through the
// Cont fires Cnt.g2 and, the bare half being on nSD-Activ 0 from pSD, Cnt.g1 (finding 5).
#[case::cnt_g2_h1("cont/Cnt.g2.h1.gds.gz", "TOP",
    [vec!["Cnt.g2"; 7], vec!["Cnt.g1"]].concat(), vec![])]
// pSD chamfers 0.085/0.092 from the Cont corner are clean by the settled reading; pSD
// as abutting halves and as overlapping boxes is clean; a union with 0.085 and a pSD
// drawn twice with 0.085 fire once each.
#[case::cnt_g2_h2("cont/Cnt.g2.h2.gds.gz", "TOP", vec!["Cnt.g2"; 2], vec![])]
// 0.085 pSD margins on and across the tile lines; 0.09 across x = 20 is clean.
#[case::cnt_g2_h3("cont/Cnt.g2.h3.gds.gz", "TOP", vec!["Cnt.g2"; 8], vec![])]
// Fifty Conts with 0.085 of pSD on the right, flat and as a GdsArrayRef.
#[case::cnt_g2_h4("cont/Cnt.g2.h4.gds.gz", "TOP", vec!["Cnt.g2"; 50], vec![])]
#[case::cnt_g2_h5("cont/Cnt.g2.h5.gds.gz", "TOP", vec!["Cnt.g2"; 50], vec![])]
// Three Conts on a 300 µm pSD strip 0.085 top and bottom (two walls each), one at (1000, 1000).
#[case::cnt_g2_h6("cont/Cnt.g2.h6.gds.gz", "TOP", vec!["Cnt.g2"; 7], vec![])]
// No Metal1, a 0.005 sliver uncovered, half covered, in a Metal1 ring's hole, Metal1
// abutting from outside, and a bare Cont at (1000, 1000) fire; Metal1 with margins,
// coincident, as abutting halves and as overlapping boxes covers.
#[case::cnt_h_h1("cont/Cnt.h.h1.gds.gz", "TOP", vec!["Cnt.h"; 6], vec![])]
// Bare Conts straddling the tile lines (six) and one straddling x = 20 whose Metal1 ends
// on the line; Metal1 ending where the Cont ends, and two Metal1 boxes meeting on the
// line under the Cont, cover.
#[case::cnt_h_h2("cont/Cnt.h.h2.gds.gz", "TOP", vec!["Cnt.h"; 7], vec![])]
// Fifty bare Conts, flat and as a GdsArrayRef.
#[case::cnt_h_h3("cont/Cnt.h.h3.gds.gz", "TOP", vec!["Cnt.h"; 50], vec![])]
#[case::cnt_h_h4("cont/Cnt.h.h4.gds.gz", "TOP", vec!["Cnt.h"; 50], vec![])]
// A Cont in a gate, one overlapping Activ by a 0.005 strip, one by a 0.005 × 0.005 corner,
// one over two 0.05 Activ fingers (two overlap pieces, two markers), one on a gate over
// two abutting Activ boxes and one at (1000, 1000) fire Cnt.j; the strip, the corner and
// the fingers are Conts on Activ enclosed by 0 (Cnt.c: one, one, four walls); the Cont
// on poly abutting Activ is not over it, but 0 from it (Cnt.e, finding 4).
#[case::cnt_j_h1("cont/Cnt.j.h1.gds.gz", "TOP",
    [vec!["Cnt.c"; 6], vec!["Cnt.e"], vec!["Cnt.j"; 7]].concat(), vec![])]
// Gate contacts straddling the tile lines (six) and a Cont on poly whose Activ begins on
// x = 20 under its right half (Cnt.j, and Cnt.c for the half on Activ); one whose Activ
// begins where the Cont ends abuts it: no Cnt.j, but Cnt.e at 0 (finding 4).
#[case::cnt_j_h2("cont/Cnt.j.h2.gds.gz", "TOP",
    [vec!["Cnt.c"], vec!["Cnt.e"], vec!["Cnt.j"; 7]].concat(), vec![])]
// Fifty gate contacts, flat and as a GdsArrayRef.
#[case::cnt_j_h3("cont/Cnt.j.h3.gds.gz", "TOP", vec!["Cnt.j"; 50], vec![])]
#[case::cnt_j_h4("cont/Cnt.j.h4.gds.gz", "TOP", vec!["Cnt.j"; 50], vec![])]
fn test_cont(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_CNT, gds, topcell, &ignore), expected);
}

// --- ContBar ---

const DECK_CNTB: &str = "contbar";

#[rstest]
#[case::cntb_a("contbar/CntB.a.gds.gz", "TOP", vec!["CntB.a"; 2], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_a1("contbar/CntB.a1.gds.gz", "TOP", vec!["CntB.a1"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_b("contbar/CntB.b.gds.gz", "TOP", vec!["CntB.b"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_b1("contbar/CntB.b1.gds.gz", "TOP", vec!["CntB.b1"], vec!["CntB.c", "CntB.d", "CntB.g", "CntB.h", "CntB.h1"])]
#[case::cntb_c("contbar/CntB.c.gds.gz", "TOP", vec!["CntB.c"], vec!["CntB.d", "CntB.h", "CntB.h1"])]
#[case::cntb_h1("contbar/CntB.h1.gds.gz", "TOP", vec!["CntB.h1"], vec!["CntB.c", "CntB.d", "CntB.g"])]
#[case::cntb_g("contbar/CntB.g.gds.gz", "TOP", vec!["CntB.g"], vec!["CntB.c", "CntB.d", "CntB.h", "CntB.h1"])]
#[case::cntb_j("contbar/CntB.j.gds.gz", "TOP", vec!["CntB.j"], vec!["CntB.c", "CntB.h", "CntB.h1"])]
fn test_contbar(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_CNTB, gds, topcell, &ignore), expected);
}

// --- SalBlock ---

const DECK_SAL: &str = "salblock";

#[rstest]
#[case::sal_a("salblock/Sal.a.gds.gz", "TOP", vec!["Sal.a"; 4], vec![])]
#[case::sal_b_space("salblock/Sal.b.space.gds.gz", "TOP", vec!["Sal.b", "Sal.b"], vec![])]
#[case::sal_b_notch("salblock/Sal.b.notch.gds.gz", "TOP", vec!["Sal.b", "Sal.b"], vec![])]
#[case::sal_c("salblock/Sal.c.gds.gz", "TOP", vec!["Sal.c"], vec![])]
#[case::sal_d("salblock/Sal.d.gds.gz", "TOP", vec!["Sal.d", "Sal.d"], vec![])]
#[case::sal_e("salblock/Sal.e.gds.gz", "TOP", vec!["Sal.e", "Sal.e"], vec![])]
fn test_salblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SAL, gds, topcell, &ignore), expected);
}

// --- nSD:block ---

const DECK_NSDB: &str = "nsdblock";

#[rstest]
#[case::nsdb_a("nsdblock/nSDB.a.gds.gz", "TOP", vec!["nSDB.a"; 4], vec![])]
#[case::nsdb_b_space("nsdblock/nSDB.b.space.gds.gz", "TOP", vec!["nSDB.b", "nSDB.b"], vec![])]
#[case::nsdb_b_notch("nsdblock/nSDB.b.notch.gds.gz", "TOP", vec!["nSDB.b", "nSDB.b"], vec![])]
#[case::nsdb_c("nsdblock/nSDB.c.gds.gz", "TOP", vec!["nSDB.c", "nSDB.c"], vec![])]
#[case::nsdb_e("nsdblock/nSDB.e.gds.gz", "TOP", vec!["nSDB.e"], vec![])]
fn test_nsdblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NSDB, gds, topcell, &ignore), expected);
}

// --- nBuLay:block ---

const DECK_NBLB: &str = "nbulayblock";

#[rstest]
#[case::nblb_a("nbulayblock/NBLB.a.gds.gz", "TOP", vec!["NBLB.a"; 4], vec!["NBLB.c"])]
#[case::nblb_b_space("nbulayblock/NBLB.b.space.gds.gz", "TOP", vec!["NBLB.b", "NBLB.b"], vec!["NBLB.c"])]
#[case::nblb_b_notch("nbulayblock/NBLB.b.notch.gds.gz", "TOP", vec!["NBLB.b", "NBLB.b"], vec!["NBLB.c"])]
#[case::nblb_c("nbulayblock/NBLB.c.gds.gz", "TOP", vec!["NBLB.c"; 4], vec![])]
#[case::nblb_d("nbulayblock/NBLB.d.gds.gz", "TOP", vec!["NBLB.d", "NBLB.d"], vec!["NBLB.c"])]
fn test_nbulayblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NBLB, gds, topcell, &ignore), expected);
}

// --- nBuLay ---

const DECK_NBL: &str = "nbulay";

#[rstest]
#[case::nbl_a("nbulay/NBL.a.gds.gz", "TOP", vec!["NBL.a"; 4], vec![])]
// 1.20 gap fires; the exactly-1.50 gap is clean (and closes in nBuLayMerged, so no NBL.c).
#[case::nbl_b("nbulay/NBL.b.gds.gz", "TOP", vec!["NBL.b"], vec![])]
// The 1.00 "merged" pair now also legitimately draws the new NBL.b — ignored here.
#[case::nbl_c("nbulay/NBL.c.gds.gz", "TOP", vec!["NBL.c"], vec!["NBL.b"])]
// Same-net regression, the nBuLay twin of NW.b1.same_net: two pairs at a 2.00 µm gap,
// only the bare one different-net.  The second pair's buried layers are shorted through
// their NWell sinkers, taps and a Metal1 strap, so NBL.c fires exactly once.
#[case::nbl_c_same_net("nbulay/NBL.c.same_net.gds.gz", "TOP", vec!["NBL.c"], vec![])]
#[case::nbl_d("nbulay/NBL.d.gds.gz", "TOP", vec!["NBL.d", "NBL.d"], vec![])]
#[case::nbl_e("nbulay/NBL.e.gds.gz", "TOP", vec!["NBL.e", "NBL.e"], vec![])]
#[case::nbl_f("nbulay/NBL.f.gds.gz", "TOP", vec!["NBL.f", "NBL.f"], vec![])]
// NBL.d is the two-layer "different net" rule: a bare row, and a row where the NWell is
// strapped to the buried layer through its sinker, a tap and a Metal1 plate — so only the
// bare row fires.
#[case::nbl_d_same_net("nbulay/NBL.d.same_net.gds.gz", "TOP", vec!["NBL.d"], vec![])]
fn test_nbulay(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NBL, gds, topcell, &ignore), expected);
}

// --- PWell:block ---

const DECK_PWB: &str = "pwellblock";

#[rstest]
#[case::pwb_a("pwellblock/PWB.a.gds.gz", "TOP", vec!["PWB.a"; 4], vec![])]
#[case::pwb_b_space("pwellblock/PWB.b.space.gds.gz", "TOP", vec!["PWB.b", "PWB.b"], vec![])]
#[case::pwb_b_notch("pwellblock/PWB.b.notch.gds.gz", "TOP", vec!["PWB.b", "PWB.b"], vec![])]
#[case::pwb_c("pwellblock/PWB.c.gds.gz", "TOP", vec!["PWB.c", "PWB.c"], vec![])]
#[case::pwb_e("pwellblock/PWB.e.gds.gz", "TOP", vec!["PWB.e"], vec![])]
#[case::pwb_e1("pwellblock/PWB.e1.gds.gz", "TOP", vec!["PWB.e1"], vec![])]
#[case::pwb_f("pwellblock/PWB.f.gds.gz", "TOP", vec!["PWB.f"], vec![])]
#[case::pwb_f1("pwellblock/PWB.f1.gds.gz", "TOP", vec!["PWB.f1"], vec![])]
fn test_pwellblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PWB, gds, topcell, &ignore), expected);
}

// --- NWell ---

const DECK_NW: &str = "nwell";

#[rstest]
#[case::nw_a("nwell/NW.a.gds.gz", "TOP", vec!["NW.a"; 4], vec![])]
// 0.50 gap fires NW.b; the exactly-0.62 gap is NW.b's limit and stays two regions
// ("separated by less than this value will be merged"), so on no common net it is
// NW.b1's.  KLayout reads both pairs as NW.b1.
#[case::nw_b("nwell/NW.b.gds.gz", "TOP", vec!["NW.b", "NW.b1"], vec![])]
// The 0.50 "merged" pair now also legitimately draws the new NW.b — ignored here.
#[case::nw_b1("nwell/NW.b1.gds.gz", "TOP", vec!["NW.b1"], vec!["NW.b"])]
// Same-net regression (GitHub report): two NWell pairs, both at a 1.00 µm gap.  Only the
// bare pair is different-net; the second pair's wells are shorted by an N+Activ tie →
// Cont → Metal1 strap → Cont → tie, so NW.b1 must fire exactly once.  Currently fires
// twice: `min_space` is geometric and NWell is absent from the connectivity model, so the
// `close`-based same-net merge (radius 0.31) cannot see a tie beyond a 0.62 µm gap.
#[case::nw_b1_same_net("nwell/NW.b1.same_net.gds.gz", "TOP", vec!["NW.b1"], vec![])]
// The 0.20-margin tie fires (the same edge pair as KLayout), the 0.30 tie is clean, and
// the NWell-crossing tie is no tie "surrounded entirely by NWell": no NW.e, but its part
// outside the well is external N+Activ at no distance, NW.d - as KLayout has it.
#[case::nw_e("nwell/NW.e.gds.gz", "TOP", vec!["NW.d", "NW.e"], vec![])]
#[case::nw_e1("nwell/NW.e1.gds.gz", "TOP", vec!["NW.e1"], vec![])]
#[case::nw_f("nwell/NW.f.gds.gz", "TOP", vec!["NW.f"], vec![])]
#[case::nw_f1("nwell/NW.f1.gds.gz", "TOP", vec!["NW.f1"], vec![])]
#[case::nw_c("nwell/NW.c.gds.gz", "TOP", vec!["NW.c"], vec![])]
#[case::nw_c1("nwell/NW.c1.gds.gz", "TOP", vec!["NW.c1"], vec![])]
// Drawn-nSD and plain undoped Activ (N+ by default) both fire at 0.30; the 0.31 gap and
// the Activ-under-nSD:block canary stay clean.  KLayout marks the same two instances.
#[case::nw_d("nwell/NW.d.gds.gz", "TOP", vec!["NW.d"; 2], vec![])]
#[case::nw_d1("nwell/NW.d1.gds.gz", "TOP", vec!["NW.d1"], vec![])]
// Chapter 8: inside a DigiBnd the HV rules relax to 0.31/0.31/0.24 (.dig variants); the
// relaxed-clean instances prove the strict values no longer apply there, and the strict
// NW.d1 still fires outside.  All four match KLayout 1:1.
#[case::nw_dig("nwell/NW.dig.gds.gz", "TOP",
    vec!["NW.c1.dig", "NW.d1", "NW.d1.dig", "NW.e1.dig"], vec![])]
// --- Hardening (ci/hardening/SPEC.md): expected values are the manual's answer, not the
// engine's; the reasoning is in ci/hardening/reports/ihp-sg13g2/nwell.md.  A min_width
// violation counts two markers per narrow bar (one per long edge), as the NW.a case above.
// Three 0.615 bars (x, y, 300 µm long across every tile line) → two markers each.
#[case::nw_a_h1("nwell/NW.a.h1.gds.gz", "TOP", vec!["NW.a"; 6], vec![])]
// 45°: diamond, 45° strip and octagon 0.615 across their diagonal faces fire (4 + 2 + 4
// markers); the 0.622 diamond/strip and the chamfered shapes are clean.  A small octagon
// with 0.62 across its flats is not: the chord from the end of a diagonal to the end of
// the wall two edges on is a 0.475 stretch of material between two boundary points, and
// the euclidian width reads it - eight chords on each octagon, KLayout's `ext_width`
// reports the same eight (hardening report, finding 11).
#[case::nw_a_h2("nwell/NW.a.h2.gds.gz", "TOP", vec!["NW.a"; 26], vec![])]
// Merged unions 0.615 wide (overlapping boxes, abutting slices, one ring side, an island
// in a ring) fire; unions 0.62 wide and a bar drawn as a 4 × 10 grid are clean.
#[case::nw_a_h3("nwell/NW.a.h3.gds.gz", "TOP", vec!["NW.a"; 8], vec!["NW.b1"])]
// Ten 0.615 bars on, across and straddling x = 20/21/40/42 plus an L cornered on x = 20.
#[case::nw_a_h4("nwell/NW.a.h4.gds.gz", "TOP", vec!["NW.a"; 20], vec![])]
// Fifty 0.615 bars, flat and as a GdsArrayRef.
#[case::nw_a_h5("nwell/NW.a.h5.gds.gz", "TOP", vec!["NW.a"; 100], vec![])]
#[case::nw_a_h6("nwell/NW.a.h6.gds.gz", "TOP", vec!["NW.a"; 100], vec![])]
// A 0.005 sliver and a 0.615 bar at (1000, 1000).
#[case::nw_a_h7("nwell/NW.a.h7.gds.gz", "TOP", vec!["NW.a"; 4], vec![])]
// A comb with three 0.615 teeth; a U with 0.62 arms is clean.
#[case::nw_a_h8("nwell/NW.a.h8.gds.gz", "TOP", vec!["NW.a"; 6], vec![])]
// Gap 0.615, a 0.43/0.43 diagonal (0.608) and a corner-to-corner 0.615 fire; 0.62 and the
// 0.44/0.44 diagonal (0.622) are clean.  Bare wells under 1.80 apart also draw NW.b1.
#[case::nw_b_h1("nwell/NW.b.h1.gds.gz", "TOP", vec!["NW.b"; 3], vec!["NW.b1"])]
// 45°: diamond tip to wall, two 45° strips, chamfer to corner, tip to tip at 0.615/0.601.
#[case::nw_b_h2("nwell/NW.b.h2.gds.gz", "TOP", vec!["NW.b"; 4], vec!["NW.b1"])]
// "Space or notch": straight and 45° notches (2 + 2), a comb with three 0.615 slots, a slot
// in a plate, two facing Ls, a keyhole ring with a 0.615 hole, an island 0.615 from a ring.
// The deck has no min_notch half for NW.b, so only the Ls and the island are reported.
#[case::nw_b_h3("nwell/NW.b.h3.gds.gz", "TOP", vec!["NW.b"; 11], vec!["NW.b1"])]
// Overlapping, abutting and gridded boxes each 0.615 from a third box: one each.
#[case::nw_b_h4("nwell/NW.b.h4.gds.gz", "TOP", vec!["NW.b"; 3], vec!["NW.b1"])]
// Ten 0.615 gaps on, across and straddling x = 20/21/40/42, incl. a corner on x = 20.
#[case::nw_b_h5("nwell/NW.b.h5.gds.gz", "TOP", vec!["NW.b"; 10], vec!["NW.b1"])]
// Fifty 0.615 pairs, flat and as a GdsArrayRef.
#[case::nw_b_h6("nwell/NW.b.h6.gds.gz", "TOP", vec!["NW.b"; 50], vec![])]
#[case::nw_b_h7("nwell/NW.b.h7.gds.gz", "TOP", vec!["NW.b"; 50], vec![])]
// A 0.005 sliver 0.615 from a box, two 300 µm bars 0.615 apart, a pair at (1000, 1000).
#[case::nw_b_h8("nwell/NW.b.h8.gds.gz", "TOP", vec!["NW.b"; 3], vec!["NW.a"])]
// Two wells 0.50 apart on different Metal1 nets and two on one net: closer than 0.62 they
// merge, so both pairs are NW.b whatever the net (KLayout calls the first NW.b1).
#[case::nw_b_h9("nwell/NW.b.h9.gds.gz", "TOP", vec!["NW.b"; 2], vec![])]
// Bare wells: gaps 0.62 (NW.b's limit - not merged, so NW.b1 applies), 0.625, 1.795 and
// diagonals 1.27/1.27 (1.796), 1.20/1.20 (1.697), 0.90/0.90 (1.273) fire; 1.80 and
// 1.28/1.28 (1.810) are clean.  The merge used to take the exact-0.62 gap and to bevel
// the merged layer's corners by 0.128, which read 1.796 and 1.697 as clean.
#[case::nw_b1_h1("nwell/NW.b1.h1.gds.gz", "TOP", vec!["NW.b1"; 6], vec!["NW.b"])]
// Nets: ties on separate Metal1, ties joined via Metal2 (clean), P+Activ "ties" with Cont
// and a strap (no well connection), N+Activ taps under a strap without Cont.
#[case::nw_b1_h2("nwell/NW.b1.h2.gds.gz", "TOP", vec!["NW.b1"; 3], vec![])]
// One net through the well itself (a U, a bridge, a tied island in a ring) is clean; the
// same island untied fires once.
#[case::nw_b1_h3("nwell/NW.b1.h3.gds.gz", "TOP", vec!["NW.b1"], vec![])]
// 45°: a diamond tip 1.795 from a wall and two 45° strips 1.796 apart fire; 1.80 and
// 1.803 are clean.  The merged layer's beveled tip hides the diamond.
#[case::nw_b1_h4("nwell/NW.b1.h4.gds.gz", "TOP", vec!["NW.b1"; 2], vec!["NW.b"])]
// Eight 1.795 gaps on, across and straddling x = 20/21/40/42.
#[case::nw_b1_h5("nwell/NW.b1.h5.gds.gz", "TOP", vec!["NW.b1"; 8], vec![])]
// Fifty bare pairs 1.00 apart, flat and as a GdsArrayRef.
#[case::nw_b1_h6("nwell/NW.b1.h6.gds.gz", "TOP", vec!["NW.b1"; 50], vec![])]
#[case::nw_b1_h7("nwell/NW.b1.h7.gds.gz", "TOP", vec!["NW.b1"; 50], vec![])]
// PWell:block filling a 1.00 gap leaves no PWell between the wells (section 4.2), so
// NW.b1 has nothing to measure; a 0.50 block strip leaves 0.25 of PWell either side → fires.
#[case::nw_b1_h8("nwell/NW.b1.h8.gds.gz", "TOP", vec!["NW.b1"], vec![])]
// A 0.305 margin and parallel chamfers 0.304 apart fire; the 0.311 chamfers are clean.
// A chamfer or a 45° wall passing 0.269 from the Activ's *corner* is not read: enclosure
// is measured by projection, between walls, as IHP's `ext_enclosed` reads it (hardening
// report, finding 10 - open).
#[case::nw_c_h1("nwell/NW.c.h1.gds.gz", "TOP", vec!["NW.c"; 2], vec![])]
// Unions: P+Activ from two boxes (0.31 clean, 0.305 fires), NWell from two boxes (clean),
// plain Activ is N+ (clean), pSD over half the Activ (the P+ half fires), a P+Activ 0.305
// from a ring's hole.
#[case::nw_c_h2("nwell/NW.c.h2.gds.gz", "TOP", vec!["NW.c"; 3], vec![])]
// Activ crossing the well edge: P+Activ is not enclosed (NW.c); N+Activ is external at
// zero distance (NW.d - a forbidden entry on external N+ meeting the well, since a
// space check has no pair at a touch).
#[case::nw_c_h3("nwell/NW.c.h3.gds.gz", "TOP", vec!["NW.c", "NW.d"], vec!["NW.e", "NW.f"])]
// Nine 0.305 margins on, across and straddling x = 20/21/40/42, one 10 µm long.
#[case::nw_c_h4("nwell/NW.c.h4.gds.gz", "TOP", vec!["NW.c"; 9], vec![])]
// Fifty 0.305 margins, flat and as a GdsArrayRef.
#[case::nw_c_h5("nwell/NW.c.h5.gds.gz", "TOP", vec!["NW.c"; 50], vec![])]
#[case::nw_c_h6("nwell/NW.c.h6.gds.gz", "TOP", vec!["NW.c"; 50], vec![])]
// TGO over half the P+Activ: the TGO half is NW.c1's, the other NW.c's; TGO abutting
// without overlap leaves NW.c alone; a 0.005 TGO overlap makes a sliver NW.c1 must see.
#[case::nw_c_h7("nwell/NW.c.h7.gds.gz", "TOP", vec!["NW.c", "NW.c", "NW.c", "NW.c1", "NW.c1"], vec![])]
// A 0.005 sliver, a 300 µm P+Activ with a 0.305 bottom margin, one at (1000, 1000).
#[case::nw_c_h8("nwell/NW.c.h8.gds.gz", "TOP", vec!["NW.c"; 3], vec![])]
// 0.615 fires, 0.62 is clean; a chamfer 0.615 from the corner is not read (projection,
// finding 10); inside DigiBnd 0.305 → NW.c1.dig; a device the DigiBnd edge cuts through
// is inside as far as it overlaps, as KLayout reads it (finding 12 - open), so 0.45 is
// clean there; a DigiBnd frame with the device in its hole is not inside → NW.c1 at 0.45.
#[case::nw_c1_h1("nwell/NW.c1.h1.gds.gz", "TOP", vec!["NW.c1", "NW.c1", "NW.c1.dig"], vec![])]
// Seven 0.305 margins under one DigiBnd on/across x = 10/20/21/40/42 → NW.c1.dig; a 0.615
// margin outside it → NW.c1.
#[case::nw_c1_h2("nwell/NW.c1.h2.gds.gz", "TOP",
    vec!["NW.c1", "NW.c1.dig", "NW.c1.dig", "NW.c1.dig", "NW.c1.dig", "NW.c1.dig", "NW.c1.dig", "NW.c1.dig"], vec![])]
// Fifty 0.615 margins under TGO, flat and as a GdsArrayRef.
#[case::nw_c1_h3("nwell/NW.c1.h3.gds.gz", "TOP", vec!["NW.c1"; 50], vec![])]
#[case::nw_c1_h4("nwell/NW.c1.h4.gds.gz", "TOP", vec!["NW.c1"; 50], vec![])]
// Gap 0.305, a 0.215/0.215 diagonal (0.304), a diamond tip at 0.305, a corner 0.304 from a
// 45° wall and parallel 45° edges 0.304 apart fire; 0.311 and 0.31 are clean.
#[case::nw_d_h1("nwell/NW.d.h1.gds.gz", "TOP", vec!["NW.d"; 5], vec![])]
// Conditions: a tie inside the well and a P+Activ outside are not NW.d's; nSD under
// nSD:block is N+; N+Activ 0.305 from a ring's inner wall, as abutting boxes, against a
// two-box well and in a U-notch fire; 0.31 in the hole is clean.
#[case::nw_d_h2("nwell/NW.d.h2.gds.gz", "TOP", vec!["NW.d"; 5], vec![])]
// Eight 0.305 gaps on, across and straddling x = 20/21/40/42, one 10 µm long.
#[case::nw_d_h3("nwell/NW.d.h3.gds.gz", "TOP", vec!["NW.d"; 8], vec![])]
// Fifty 0.305 gaps, flat and as a GdsArrayRef.
#[case::nw_d_h4("nwell/NW.d.h4.gds.gz", "TOP", vec!["NW.d"; 50], vec![])]
#[case::nw_d_h5("nwell/NW.d.h5.gds.gz", "TOP", vec!["NW.d"; 50], vec![])]
// A 0.005 Activ sliver, a 300 µm Activ 0.305 below a 300 µm well, a pair at (1000, 1000).
#[case::nw_d_h6("nwell/NW.d.h6.gds.gz", "TOP", vec!["NW.d"; 3], vec![])]
// TGO over half the Activ (NW.d1 for that half), TGO abutting (NW.d only), a 0.005 TGO
// sliver (NW.d and NW.d1); inside DigiBnd 0.305 → NW.d1.dig; an Activ the DigiBnd edge
// cuts through is inside (finding 12 - open) → clean at 0.45; a DigiBnd frame with the
// Activ in its hole is not inside → NW.d1 at 0.45; DigiBnd over the Activ but not the
// well is inside → clean at 0.45.
#[case::nw_d_h7("nwell/NW.d.h7.gds.gz", "TOP",
    vec!["NW.d", "NW.d", "NW.d1", "NW.d1", "NW.d1", "NW.d1.dig"], vec![])]
// 0.615, a 0.435/0.435 diagonal (0.615) and a diamond tip at 0.615 fire; 0.62/0.622 clean.
#[case::nw_d1_h1("nwell/NW.d1.h1.gds.gz", "TOP", vec!["NW.d1"; 3], vec![])]
// Fifty 0.615 gaps under TGO, flat and as a GdsArrayRef.
#[case::nw_d1_h2("nwell/NW.d1.h2.gds.gz", "TOP", vec!["NW.d1"; 50], vec![])]
#[case::nw_d1_h3("nwell/NW.d1.h3.gds.gz", "TOP", vec!["NW.d1"; 50], vec![])]
// A 0.235 margin and parallel chamfers 0.233 apart fire; 0.24/0.240 are clean.  A
// chamfer or 45° wall 0.233 from the tie's corner is not read (projection, finding 10).
#[case::nw_e_h1("nwell/NW.e.h1.gds.gz", "TOP", vec!["NW.e"; 2], vec![])]
// P+Activ at 0.20 is NW.c's; a drawn-nSD tie at 0.235, a tie 0.235 from a ring's hole, a
// two-box tie with a 0.235 union margin and a tie under a two-box well at 0.235 fire; the
// 0.24 ring and two-box well are clean.
#[case::nw_e_h2("nwell/NW.e.h2.gds.gz", "TOP", vec!["NW.c", "NW.e", "NW.e", "NW.e", "NW.e"], vec![])]
// Eight 0.235 margins on, across and straddling x = 20/21/40/42, one 10 µm long.
#[case::nw_e_h3("nwell/NW.e.h3.gds.gz", "TOP", vec!["NW.e"; 8], vec![])]
// Fifty 0.235 margins, flat and as a GdsArrayRef.
#[case::nw_e_h4("nwell/NW.e.h4.gds.gz", "TOP", vec!["NW.e"; 50], vec![])]
#[case::nw_e_h5("nwell/NW.e.h5.gds.gz", "TOP", vec!["NW.e"; 50], vec![])]
// A 0.005 tie, a 300 µm tie with a 0.235 bottom margin, one at (1000, 1000).
#[case::nw_e_h6("nwell/NW.e.h6.gds.gz", "TOP", vec!["NW.e"; 3], vec!["NW.a"])]
// TGO over half the tie (NW.e1 for that half), TGO abutting (NW.e only), a 0.005 TGO
// sliver (NW.e and NW.e1); inside DigiBnd 0.235 → NW.e1.dig; a tie the DigiBnd edge cuts
// through is inside (finding 12 - open) → clean at 0.45; a DigiBnd frame with the tie in
// its hole is not inside → NW.e1 at 0.45.
#[case::nw_e_h7("nwell/NW.e.h7.gds.gz", "TOP",
    vec!["NW.e", "NW.e", "NW.e1", "NW.e1", "NW.e1", "NW.e1.dig"], vec![])]
// A tie whose edge lies on the well edge: surrounded entirely, zero enclosure → NW.e (the
// engine's skip_clipped drops it).
#[case::nw_e_h8("nwell/NW.e.h8.gds.gz", "TOP", vec!["NW.e"], vec![])]
// Activ under nSD:block without nSD or pSD is neither N+ nor P+ (section 4.2): no tie, no
// NW.e at 0.20 (the engine takes every non-pSD Activ in the well for a tie).
#[case::nw_e_h9("nwell/NW.e.h9.gds.gz", "TOP", vec![], vec![])]
// 0.615 fires, 0.62 is clean; a chamfer 0.615 from the tie's corner is not read
// (projection, finding 10), 0.622 is clean.
#[case::nw_e1_h1("nwell/NW.e1.h1.gds.gz", "TOP", vec!["NW.e1"; 1], vec![])]
// Fifty 0.615 margins under TGO, flat and as a GdsArrayRef.
#[case::nw_e1_h2("nwell/NW.e1.h2.gds.gz", "TOP", vec!["NW.e1"; 50], vec![])]
#[case::nw_e1_h3("nwell/NW.e1.h3.gds.gz", "TOP", vec!["NW.e1"; 50], vec![])]
// Gap 0.235, a 0.165/0.165 diagonal (0.233), a diamond tip at 0.235 and a corner 0.233
// from a 45° wall fire; 0.24/0.240 are clean.
#[case::nw_f_h1("nwell/NW.f.h1.gds.gz", "TOP", vec!["NW.f"; 4], vec![])]
// P+Activ inside the well is NW.c's, plain Activ is NW.d's; P+Activ under PWell:block is
// no substrate tie (section 4.2) → clean, half under it → fires; 0.235 in a ring's hole,
// in a U-notch and union-to-union fire; 0.24 in the hole is clean.  The engine reports
// the PWell:block tie too.
#[case::nw_f_h2("nwell/NW.f.h2.gds.gz", "TOP", vec!["NW.c", "NW.d", "NW.f", "NW.f", "NW.f", "NW.f"], vec![])]
// Eight 0.235 gaps on, across and straddling x = 20/21/40/42, one 10 µm long.
#[case::nw_f_h3("nwell/NW.f.h3.gds.gz", "TOP", vec!["NW.f"; 8], vec![])]
// Fifty 0.235 gaps, flat and as a GdsArrayRef.
#[case::nw_f_h4("nwell/NW.f.h4.gds.gz", "TOP", vec!["NW.f"; 50], vec![])]
#[case::nw_f_h5("nwell/NW.f.h5.gds.gz", "TOP", vec!["NW.f"; 50], vec![])]
// A 0.005 P+Activ sliver, a 300 µm tie 0.235 below a 300 µm well, a pair at (1000, 1000).
#[case::nw_f_h6("nwell/NW.f.h6.gds.gz", "TOP", vec!["NW.f"; 3], vec![])]
// TGO over half the tie (NW.f1 for that half), TGO abutting (NW.f only), a 0.005 TGO
// sliver (NW.f and NW.f1); section 8.1.1 relaxes NW.f1 to 0.24 inside DigiBnd: 0.30 is
// clean and 0.235 is the digital variant NW.f1.dig; a tie the DigiBnd edge cuts through
// is inside (finding 12 - open) → clean at 0.45; a DigiBnd frame with the tie in its hole
// is not inside → NW.f1 at 0.45.
#[case::nw_f_h7("nwell/NW.f.h7.gds.gz", "TOP",
    vec!["NW.f", "NW.f", "NW.f1", "NW.f1", "NW.f1", "NW.f1.dig"], vec![])]
// 0.615, a 0.435/0.435 diagonal (0.615) and a diamond tip at 0.615 fire; 0.62/0.622 clean.
#[case::nw_f1_h1("nwell/NW.f1.h1.gds.gz", "TOP", vec!["NW.f1"; 3], vec![])]
// Fifty 0.615 gaps under TGO, flat and as a GdsArrayRef.
#[case::nw_f1_h2("nwell/NW.f1.h2.gds.gz", "TOP", vec!["NW.f1"; 50], vec![])]
#[case::nw_f1_h3("nwell/NW.f1.h3.gds.gz", "TOP", vec!["NW.f1"; 50], vec![])]
fn test_nwell(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NW, gds, topcell, &ignore), expected);
}

// --- Metal1 ---

const DECK_M1: &str = "metal1";

/// The density rules, which every small layout trips, plus whatever else a hardening
/// layout draws on purpose.
fn m1dens(extra: &[&'static str]) -> Vec<&'static str> {
    [&["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"][..], extra].concat()
}

#[rstest]
#[case::m1_a("metal1/M1.a.gds.gz", "TOP", vec!["M1.a", "M1.a", "M1.a", "M1.a"], vec!["M1.d", "M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_space("metal1/M1.b.space.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_notch("metal1/M1.b.notch.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_corner("metal1/M1.corner.gds.gz", "TOP", vec!["M1.a", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_j_ok("metal1/M1.j.gds.gz", "TOP", vec![], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.h", "M1Fil.k"])]
#[case::m1_j_fail("metal1/M1.j.fail.gds.gz", "TOP", vec!["M1.j"], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.h", "M1Fil.k"])]
#[case::m1_k_ok("metal1/M1.k.gds.gz", "TOP", vec![], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.h", "M1Fil.k"])]
#[case::m1_k_fail("metal1/M1.k.fail.gds.gz", "TOP", vec!["M1.k"], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.h", "M1Fil.k"])]
#[case::m1fil_c("metal1/M1Fil.c.gds.gz", "TOP", vec!["M1Fil.c", "M1Fil.c"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1fil_h_ok("metal1/M1Fil.h.gds.gz", "TOP", vec![], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_fail("metal1/M1Fil.h.fail.gds.gz", "TOP", vec!["M1Fil.h"], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_boundary_ok("metal1/M1Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_h_boundary_fail("metal1/M1Fil.h.boundary_fail.gds.gz", "TOP", vec!["M1Fil.h"], vec!["M1.j"])]
#[case::m1fil_h_boundary_ring("metal1/M1Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_k_ok("metal1/M1Fil.k.gds.gz", "TOP", vec![], vec!["M1.b", "M1.k"])]
#[case::m1fil_k_fail("metal1/M1Fil.k.fail.gds.gz", "TOP", vec!["M1Fil.k"], vec!["M1.b", "M1.k"])]
// --- Hardening (ci/hardening/SPEC.md): expected values are the manual's answer, not the
// engine's; the reasoning is in ci/hardening/reports/ihp-sg13g2/metal1.md.  Counts follow
// the engine's marker cuts where it is right: one per wall for min_width and max_width
// (two per narrow bar, four per oversized square), one per pair for a space rule, one per
// shape for area and enclosure.  The density rules are ignored throughout (`m1dens`).  A
// wide pair under 0.22 with a run over 1.0 is M1.e as well as M1.b, and a 45° pair under
// 0.22 is M1.i as well; both are expected where they occur.
// 0.155 bars in x and y, a 300 µm bar, a 0.005 sliver, a bar at (1000, 1000); 0.16 clean.
#[case::m1_a_h1("metal1/M1.a.h1.gds.gz", "TOP", vec!["M1.a"; 10], m1dens(&["M1.d"]))]
// 45°: a 0.1556 diamond (4) and strip (2) fire; 0.1626 and the chamfered shapes are clean.
// The small diamonds are under M1.d's area and the 3 µm strips are M1.g, both ignored.
#[case::m1_a_h2("metal1/M1.a.h2.gds.gz", "TOP", vec!["M1.a"; 6], m1dens(&["M1.d", "M1.g"]))]
// Unions 0.155 wide (overlapping boxes, abutting slices, one ring wall, an island) fire
// two walls each; 0.16 unions and a 4 × 10 grid are clean.
#[case::m1_a_h3("metal1/M1.a.h3.gds.gz", "TOP", vec!["M1.a"; 8], m1dens(&[]))]
// Ten 0.155 bars on, across and straddling x = 20/21/40/42 plus an L cornered on x = 20.
#[case::m1_a_h4("metal1/M1.a.h4.gds.gz", "TOP", vec!["M1.a"; 20], m1dens(&[]))]
// Fifty 0.155 bars, flat and as a GdsArrayRef.
#[case::m1_a_h5("metal1/M1.a.h5.gds.gz", "TOP", vec!["M1.a"; 100], m1dens(&[]))]
#[case::m1_a_h6("metal1/M1.a.h6.gds.gz", "TOP", vec!["M1.a"; 100], m1dens(&[]))]
// A comb with three 0.155 teeth; a U with 0.16 arms is clean.
#[case::m1_a_h7("metal1/M1.a.h7.gds.gz", "TOP", vec!["M1.a"; 6], m1dens(&[]))]
// Gap 0.175, a 0.125/0.125 diagonal (0.1768) and a corner-on 0.175 fire; 0.18 and the
// 0.13/0.13 diagonal (0.1838) are clean.
#[case::m1_b_h1("metal1/M1.b.h1.gds.gz", "TOP", vec!["M1.b"; 3], m1dens(&[]))]
// 45° at 0.175: diamond tip to wall, two 0.566 strips (M1.e too: wide, run 2.83), chamfer
// to corner, tip to tip; every pair is M1.i as well (ignored here, M1.i has its own cases).
#[case::m1_b_h2("metal1/M1.b.h2.gds.gz", "TOP", vec!["M1.b", "M1.b", "M1.b", "M1.b", "M1.e"], m1dens(&["M1.i"]))]
// "Space or notch": a straight and a 45° notch, a comb with three 0.175 slots, a slot in
// a plate, a keyhole ring with a 0.175 hole, two facing Ls (0.5 arms: M1.e too), an
// island 0.175 from a ring.  Nine M1.b.
#[case::m1_b_h3("metal1/M1.b.h3.gds.gz", "TOP", vec!["M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.e"], m1dens(&["M1.i"]))]
// Overlapping, abutting and gridded boxes each 0.175 from a third box: one each.
#[case::m1_b_h4("metal1/M1.b.h4.gds.gz", "TOP", vec!["M1.b"; 3], m1dens(&[]))]
// Ten 0.175 gaps on, across and straddling x = 20/21/40/42 incl. a corner pair across
// (20, 20); the two 10 µm pairs are M1.e as well.
#[case::m1_b_h5("metal1/M1.b.h5.gds.gz", "TOP", vec!["M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.b", "M1.e", "M1.e"], m1dens(&[]))]
// Fifty 0.175 pairs, flat and as a GdsArrayRef.
#[case::m1_b_h6("metal1/M1.b.h6.gds.gz", "TOP", vec!["M1.b"; 50], m1dens(&[]))]
#[case::m1_b_h7("metal1/M1.b.h7.gds.gz", "TOP", vec!["M1.b"; 50], m1dens(&[]))]
// A 0.005 sliver 0.175 from a box, two 300 µm bars 0.175 apart (M1.e too), a pair at
// (1000, 1000).
#[case::m1_b_h8("metal1/M1.b.h8.gds.gz", "TOP", vec!["M1.b", "M1.b", "M1.b", "M1.e"], m1dens(&["M1.a", "M1.d"]))]
// Metal1 0.175 from Metal1:filler is M1Fil.c's, from Metal1:mask nobody's; two shapes on
// one net (Via1/Metal2) and two with Conts are M1.b.
#[case::m1_b_h9("metal1/M1.b.h9.gds.gz", "TOP", vec!["M1.b", "M1.b", "M1Fil.c"], m1dens(&[]))]
// Conts sticking 0.005 out on each side, one half out, one with no Metal1: six; flush
// Conts and a 0.16 line are clean.  M1.c1 is not the question here.
#[case::m1_c_h1("metal1/M1.c.h1.gds.gz", "TOP", vec!["M1.c"; 6], m1dens(&["M1.c1"]))]
// A Cont across a 0.005 gap, one across a ring's 0.1 hole, one whose corner a chamfer
// cuts: three.  Seams, overlaps, a chamfer through the corner and a covering grid are
// clean.  The gap is M1.b, the hole an M1.b notch, the small pads M1.d and M1.c1.
#[case::m1_c_h2("metal1/M1.c.h2.gds.gz", "TOP", vec!["M1.c"; 3], m1dens(&["M1.b", "M1.c1", "M1.d"]))]
// Conts sticking 0.005 out at metal ends on x = 10/20/21/40/42 and at (1000, 1000).
#[case::m1_c_h3("metal1/M1.c.h3.gds.gz", "TOP", vec!["M1.c"; 6], m1dens(&["M1.c1"]))]
// Fifty Conts sticking 0.005 out, flat and as a GdsArrayRef.
#[case::m1_c_h4("metal1/M1.c.h4.gds.gz", "TOP", vec!["M1.c"; 50], m1dens(&["M1.c1"]))]
#[case::m1_c_h5("metal1/M1.c.h5.gds.gz", "TOP", vec!["M1.c"; 50], m1dens(&["M1.c1"]))]
// Endcaps of 0.045 and 0.00 on a 0.16 line, a Cont at the end of a 0.25 line (0.045 all
// round but the far side) and a 0.05/0.045 stub: four.  0.05 endcaps, a Cont mid-line
// and one short side alone (0.30 and 0.26 lines) are clean.  The stub is under M1.d.
#[case::m1_c1_h1("metal1/M1.c1.h1.gds.gz", "TOP", vec!["M1.c1"; 4], m1dens(&["M1.d"]))]
// Plate corners: flush on two adjacent sides, 0.045/0.045, a 0.02 pad, 0.05 left with
// 0.02 elsewhere: four.  One short side, or two opposite ones, is a line running past:
// clean.  The pads are under M1.d.
#[case::m1_c1_h2("metal1/M1.c1.h2.gds.gz", "TOP", vec!["M1.c1"; 4], m1dens(&["M1.d"]))]
// A Cont under two abutting boxes with 0.045 on the top and the right fires; in an L's
// inner corner and under a chamfer passing 0.078 from its corner (projection) it is clean.
#[case::m1_c1_h3("metal1/M1.c1.h3.gds.gz", "TOP", vec!["M1.c1"; 1], m1dens(&["M1.d"]))]
// 0.045 endcaps at line ends on x = 10/20/21/40/42, at (1000, 1000) and straddling 20.
#[case::m1_c1_h4("metal1/M1.c1.h4.gds.gz", "TOP", vec!["M1.c1"; 7], m1dens(&[]))]
// Fifty 0.045 endcaps, flat and as a GdsArrayRef.
#[case::m1_c1_h5("metal1/M1.c1.h5.gds.gz", "TOP", vec!["M1.c1"; 50], m1dens(&[]))]
#[case::m1_c1_h6("metal1/M1.c1.h6.gds.gz", "TOP", vec!["M1.c1"; 50], m1dens(&[]))]
// 0.0885, a 0.0896 line, an L of 0.0864, a 0.0882 diamond, a chamfered 0.085: five; 0.09,
// 0.0904 and a 0.0925 diamond are clean.
#[case::m1_d_h1("metal1/M1.d.h1.gds.gz", "TOP", vec!["M1.d"; 5], m1dens(&[]))]
// A union of 0.07 once, two boxes meeting at a corner twice (the pinch is M1.a and M1.b,
// as the M1.corner case), an island of 0.04; abutting boxes (0.12) and a grid are clean.
#[case::m1_d_h2("metal1/M1.d.h2.gds.gz", "TOP", vec!["M1.d", "M1.d", "M1.d", "M1.d", "M1.a", "M1.b"], m1dens(&[]))]
// 0.0885 boxes on, across and straddling x = 20/21/40/42, a 0.0896 bar across 20, one at
// (1000, 1000); a 0.09 box straddling 20 is clean.
#[case::m1_d_h3("metal1/M1.d.h3.gds.gz", "TOP", vec!["M1.d"; 9], m1dens(&[]))]
// Fifty 0.08 boxes, flat and as a GdsArrayRef.
#[case::m1_d_h4("metal1/M1.d.h4.gds.gz", "TOP", vec!["M1.d"; 50], m1dens(&[]))]
#[case::m1_d_h5("metal1/M1.d.h5.gds.gz", "TOP", vec!["M1.d"; 50], m1dens(&[]))]
// A 0.005 × 0.5 sliver fires; a 0.005 × 18 sliver (0.09) and a 300 µm bar are clean.
#[case::m1_d_h6("metal1/M1.d.h6.gds.gz", "TOP", vec!["M1.d"; 1], m1dens(&["M1.a"]))]
// A 0.305 line beside a 0.16 line at 0.20, two 0.5 lines at 0.215, a plate beside a line:
// three; 0.30 wide, two 0.16 lines and 0.22 are clean.
#[case::m1_e_h1("metal1/M1.e.h1.gds.gz", "TOP", vec!["M1.e"; 3], m1dens(&[]))]
// Runs of 1.005 (aligned, and as the shared part of two 3 µm lines) fire, 1.0 does not;
// stubs of 0.8 and 0.6 are clean; a line broken into two 2.35 pieces fires twice.
#[case::m1_e_h2("metal1/M1.e.h2.gds.gz", "TOP", vec!["M1.e"; 4], m1dens(&[]))]
// A 0.5 pad 1.5 long in a 0.16 line, and an L's arm, beside a line at 0.20 fire; pads 0.8
// and 1.0 long do not - the run is the wide part's.
#[case::m1_e_h3("metal1/M1.e.h3.gds.gz", "TOP", vec!["M1.e"; 2], m1dens(&[]))]
// 45° pairs at 0.2015 with walls 2.83 and 1.06 long fire (the run is along the walls),
// 0.85 does not; plates stepped to share 1.005 fire, 1.0 not; corner to corner and end-on
// are clean.  The 0.17 strips are M1.g and every 45° pair is M1.i.
#[case::m1_e_h4("metal1/M1.e.h4.gds.gz", "TOP", vec!["M1.e", "M1.e", "M1.e", "M1.g", "M1.g", "M1.g", "M1.g", "M1.g", "M1.g", "M1.i", "M1.i", "M1.i"], m1dens(&[]))]
// A wide line from two overlapping or two abutting boxes fires, 0.16 + 0.14 (0.30) does
// not; a neighbour drawn as two halves or three collinear boxes fires once each.
#[case::m1_e_h5("metal1/M1.e.h5.gds.gz", "TOP", vec!["M1.e"; 4], m1dens(&[]))]
// Eleven 0.305/0.16 pairs at 0.20 on, across and straddling x = 20/21/40/42, running
// across 20 and 40, a 1.005 run ending on 20, one at (1000, 1000).
#[case::m1_e_h6("metal1/M1.e.h6.gds.gz", "TOP", vec!["M1.e"; 11], m1dens(&[]))]
// Fifty 0.5/0.16 pairs at 0.20, flat and as a GdsArrayRef.
#[case::m1_e_h7("metal1/M1.e.h7.gds.gz", "TOP", vec!["M1.e"; 50], m1dens(&[]))]
#[case::m1_e_h8("metal1/M1.e.h8.gds.gz", "TOP", vec!["M1.e"; 50], m1dens(&[]))]
// Two wide lines on one net, a 300 µm pair and a 0.005 sliver 0.20 from a wide line fire;
// a U with a 0.20 slot between 0.5 arms is a notch, not a "space of lines" (report).
#[case::m1_e_h9("metal1/M1.e.h9.gds.gz", "TOP", vec!["M1.e"; 3], m1dens(&["M1.a", "M1.d"]))]
// 10.005 wide at 0.5, at 0.595, and a run of 10.005 fire; 10.0 wide, 0.60 and a run of
// 10.0 are clean.
#[case::m1_f_h1("metal1/M1.f.h1.gds.gz", "TOP", vec!["M1.f"; 3], m1dens(&[]))]
// A 12 µm pad in a 0.5 line, plates stepped to share 10.005, a 45° pair with 10.6 walls
// spanning 7.5 in x: three; an 8 µm pad, a shared 10.0 and 8.5 walls are clean.
#[case::m1_f_h2("metal1/M1.f.h2.gds.gz", "TOP", vec!["M1.f"; 3], m1dens(&[]))]
// Pairs on, across and straddling x = 20/40/42, a horizontal pair across 20 and 40, one
// at (1000, 1000), a 300 µm pair.
#[case::m1_f_h3("metal1/M1.f.h3.gds.gz", "TOP", vec!["M1.f"; 8], m1dens(&[]))]
// Fifty 10.005/0.5 pairs at 0.5, flat and as a GdsArrayRef.
#[case::m1_f_h4("metal1/M1.f.h4.gds.gz", "TOP", vec!["M1.f"; 50], m1dens(&[]))]
#[case::m1_f_h5("metal1/M1.f.h5.gds.gz", "TOP", vec!["M1.f"; 50], m1dens(&[]))]
// Two plates on one net and a line along an L's 10.005 arm fire; a U of 10.005 arms with
// a 0.5 slot is a notch (report).
#[case::m1_f_h6("metal1/M1.f.h6.gds.gz", "TOP", vec!["M1.f"; 2], m1dens(&[]))]
// 0.198 strips with 4.24 and 0.509 walls fire (two walls each), 0.2015 and 0.495 walls
// are clean; a 0.155 strip is M1.a and M1.g; 0.198 diamonds have short edges (M1.d).
#[case::m1_g_h1("metal1/M1.g.h1.gds.gz", "TOP", vec!["M1.g", "M1.g", "M1.g", "M1.g", "M1.g", "M1.g", "M1.a", "M1.a"], m1dens(&["M1.d"]))]
// Z routes with a 0.198 jog of 0.509 walls, up and mirrored down, and a chamfered L with
// 0.566/0.509 walls fire; 0.495 walls, 0.2015 and an L whose inner wall is 0.4525 are clean.
#[case::m1_g_h2("metal1/M1.g.h2.gds.gz", "TOP", vec!["M1.g"; 6], m1dens(&[]))]
// The firing Z with its jog straddling 20, starting on 20, across 40 and 42, at (1000, 1000).
#[case::m1_g_h3("metal1/M1.g.h3.gds.gz", "TOP", vec!["M1.g"; 10], m1dens(&[]))]
// Fifty firing Z routes, flat and as a GdsArrayRef.
#[case::m1_g_h4("metal1/M1.g.h4.gds.gz", "TOP", vec!["M1.g"; 100], m1dens(&[]))]
#[case::m1_g_h5("metal1/M1.g.h5.gds.gz", "TOP", vec!["M1.g"; 100], m1dens(&[]))]
// A 300 µm 0.198 strip and a 0.007 45° sliver (M1.a, M1.d too).
#[case::m1_g_h6("metal1/M1.g.h6.gds.gz", "TOP", vec!["M1.g"; 4], m1dens(&["M1.a", "M1.d"]))]
// 0.2157 strips (M1.e too: wide, run 2.83), corner to chamfer 0.2121, tip 0.215 above a
// wall, a strip's tip 0.215 from a wall, a corner 0.2157 from a 45° wall: five; 0.2227,
// 0.2263, 0.22 and 0.2298 are clean.
#[case::m1_i_h1("metal1/M1.i.h1.gds.gz", "TOP", vec!["M1.i", "M1.i", "M1.i", "M1.i", "M1.i", "M1.e"], m1dens(&[]))]
// The figure's jog beside a plate's 45° wall and two strips on one net fire (M1.e too:
// a wide shape, run over 1.0); a 45° U with a 0.2157 slot is a notch (report).
#[case::m1_i_h2("metal1/M1.i.h2.gds.gz", "TOP", vec!["M1.i", "M1.i", "M1.e", "M1.e"], m1dens(&[]))]
// Strip pairs straddling 20, ending on 20, across 40 and 42, at (1000, 1000); M1.e too.
#[case::m1_i_h3("metal1/M1.i.h3.gds.gz", "TOP", vec!["M1.i", "M1.i", "M1.i", "M1.i", "M1.i", "M1.e", "M1.e", "M1.e", "M1.e", "M1.e"], m1dens(&[]))]
// Fifty strip pairs (walls 2.12: M1.e too), flat and as a GdsArrayRef.
#[case::m1_i_h4("metal1/M1.i.h4.gds.gz", "TOP", [vec!["M1.i"; 50], vec!["M1.e"; 50]].concat(), m1dens(&[]))]
#[case::m1_i_h5("metal1/M1.i.h5.gds.gz", "TOP", [vec!["M1.i"; 50], vec!["M1.e"; 50]].concat(), m1dens(&[]))]
// A 300 µm pair and a 0.007 sliver 0.2157 from a strip (both M1.e too; the sliver M1.a,
// M1.d and M1.g).
#[case::m1_i_h6("metal1/M1.i.h6.gds.gz", "TOP", vec!["M1.i", "M1.i", "M1.e", "M1.e"], m1dens(&["M1.a", "M1.d", "M1.g"]))]
// Strips 0.1768 apart are M1.b, M1.e and M1.i; a filler corner 0.2157 from a strip is
// M1Fil.c's.
#[case::m1_i_h7("metal1/M1.i.h7.gds.gz", "TOP", vec!["M1.i", "M1.b", "M1.e", "M1Fil.c"], m1dens(&[]))]
// Section 6.10: nothing is checked inside an EdgeSeal; a 0.155 bar outside and one
// crossing the seal's edge fire (the metal is cut at the seal's edge, as the vias are).
#[case::m1_seal_h1("metal1/M1.seal.h1.gds.gz", "TOP", vec!["M1.a"; 4], m1dens(&[]))]
// 30 % of the die on Metal1, Metal1:filler and Metal1:mask alike: M1.j, nothing else.
#[case::m1_j_h1("metal1/M1.j.h1.gds.gz", "TOP", vec!["M1.j"], vec!["M1Fil.a2"])]
// 36 % Metal1 with 4.8 % of the die cut out by Metal1:slit: 31.2 % of metal, M1.j.
#[case::m1_j_h2("metal1/M1.j.h2.gds.gz", "TOP", vec!["M1.j"], vec![])]
// 0.995 fillers in x and y, a 300 µm bar, a 0.005 sliver, one at (1000, 1000).
#[case::m1fil_a1_h1("metal1/M1Fil.a1.h1.gds.gz", "TOP", vec!["M1Fil.a1"; 10], m1dens(&["M1Fil.a2"]))]
// A 0.99 diamond (4) and strip (2) fire; 1.004 and a chamfered box are clean.
#[case::m1fil_a1_h2("metal1/M1Fil.a1.h2.gds.gz", "TOP", vec!["M1Fil.a1"; 6], m1dens(&[]))]
// Unions 0.995 wide (overlap, slices, a ring wall, an island) fire two walls each.
#[case::m1fil_a1_h3("metal1/M1Fil.a1.h3.gds.gz", "TOP", vec!["M1Fil.a1"; 8], m1dens(&[]))]
// Ten 0.995 bars on, across and straddling x = 20/21/40/42 and an L cornered on 20.
#[case::m1fil_a1_h4("metal1/M1Fil.a1.h4.gds.gz", "TOP", vec!["M1Fil.a1"; 20], m1dens(&["M1Fil.a2"]))]
// Fifty 0.995 bars, flat and as a GdsArrayRef.
#[case::m1fil_a1_h5("metal1/M1Fil.a1.h5.gds.gz", "TOP", vec!["M1Fil.a1"; 100], m1dens(&[]))]
#[case::m1fil_a1_h6("metal1/M1Fil.a1.h6.gds.gz", "TOP", vec!["M1Fil.a1"; 100], m1dens(&[]))]
// A comb with three 0.995 teeth; a U with 1.0 arms is clean.
#[case::m1fil_a1_h7("metal1/M1Fil.a1.h7.gds.gz", "TOP", vec!["M1Fil.a1"; 6], m1dens(&[]))]
// 5.005 × 5, 5 × 5.005, 5.005 × 5.005, a 5.005 union, an L spanning 5.005, a ring 5.005
// across and a diamond spanning 5.2 (3.68 between its walls) fire, one marker each: the
// bounding box's long side, KLayout's `with_bbox_max`; 5 × 5 is clean.
#[case::m1fil_a2_h1("metal1/M1Fil.a2.h1.gds.gz", "TOP", vec!["M1Fil.a2"; 7], m1dens(&[]))]
// 5.005 boxes straddling 20 and 40 and at (1000, 1000); a 5 × 3 straddling 20 is clean.
#[case::m1fil_a2_h2("metal1/M1Fil.a2.h2.gds.gz", "TOP", vec!["M1Fil.a2"; 3], m1dens(&[]))]
// Fifty 5.005 × 2 fillers, flat and as a GdsArrayRef.
#[case::m1fil_a2_h3("metal1/M1Fil.a2.h3.gds.gz", "TOP", vec!["M1Fil.a2"; 50], m1dens(&[]))]
#[case::m1fil_a2_h4("metal1/M1Fil.a2.h4.gds.gz", "TOP", vec!["M1Fil.a2"; 50], m1dens(&[]))]
// Gap 0.415, a 0.29/0.29 diagonal (0.410), corner-on 0.415; 0.42 and 0.424 are clean.
#[case::m1fil_b_h1("metal1/M1Fil.b.h1.gds.gz", "TOP", vec!["M1Fil.b"; 3], m1dens(&[]))]
// 45° at 0.415: tip to wall, parallel strips, chamfer to corner, tip to tip.
#[case::m1fil_b_h2("metal1/M1Fil.b.h2.gds.gz", "TOP", vec!["M1Fil.b"; 4], m1dens(&[]))]
// Facing Ls and an island in a ring at 0.415 fire; a U's 0.415 slot is a notch, and
// M1Fil.b says "space" (report).
#[case::m1fil_b_h3("metal1/M1Fil.b.h3.gds.gz", "TOP", vec!["M1Fil.b"; 2], m1dens(&[]))]
// Overlapping, abutting and gridded fillers each 0.415 from a third: one each.
#[case::m1fil_b_h4("metal1/M1Fil.b.h4.gds.gz", "TOP", vec!["M1Fil.b"; 3], m1dens(&[]))]
// Ten 0.415 gaps on, across and straddling x = 20/21/40/42 incl. a corner pair.
#[case::m1fil_b_h5("metal1/M1Fil.b.h5.gds.gz", "TOP", vec!["M1Fil.b"; 10], m1dens(&["M1Fil.a2"]))]
// Fifty 0.415 pairs, flat and as a GdsArrayRef.
#[case::m1fil_b_h6("metal1/M1Fil.b.h6.gds.gz", "TOP", vec!["M1Fil.b"; 50], m1dens(&[]))]
#[case::m1fil_b_h7("metal1/M1Fil.b.h7.gds.gz", "TOP", vec!["M1Fil.b"; 50], m1dens(&[]))]
// A 0.005 sliver 0.415 from a filler, two 300 µm bars 0.415 apart, a pair at (1000, 1000).
#[case::m1fil_b_h8("metal1/M1Fil.b.h8.gds.gz", "TOP", vec!["M1Fil.b"; 3], m1dens(&["M1Fil.a1", "M1Fil.a2"]))]
// A filler 0.415 from Metal1 is M1Fil.c's, from Metal1:mask nobody's; overlapping and
// abutting fillers are one filler (5.5 wide, M1Fil.a2).
#[case::m1fil_b_h9("metal1/M1Fil.b.h9.gds.gz", "TOP", vec!["M1Fil.c"], m1dens(&["M1Fil.a2"]))]
// 0.415, a 0.410 diagonal and corner-on 0.415 fire; 0.42 and 0.424 are clean.
#[case::m1fil_c_h1("metal1/M1Fil.c.h1.gds.gz", "TOP", vec!["M1Fil.c"; 3], m1dens(&[]))]
// 45°: Metal1 tip to filler, filler chamfer to Metal1 corner, parallel strips, strip tip
// to filler wall, all under 0.42.
#[case::m1fil_c_h2("metal1/M1Fil.c.h2.gds.gz", "TOP", vec!["M1Fil.c"; 4], m1dens(&[]))]
// A filler abutting Metal1 along an edge and one touching it at a corner point are 0.00
// from it; a filler crossing the Metal1 edge or inside it shares area and is no pair.
#[case::m1fil_c_h3("metal1/M1Fil.c.h3.gds.gz", "TOP", vec!["M1Fil.c"; 2], m1dens(&[]))]
// Ten 0.415 gaps on, across and straddling x = 20/21/40/42 incl. a corner pair.
#[case::m1fil_c_h4("metal1/M1Fil.c.h4.gds.gz", "TOP", vec!["M1Fil.c"; 10], m1dens(&["M1Fil.a2"]))]
// Fifty 0.415 pairs, flat and as a GdsArrayRef.
#[case::m1fil_c_h5("metal1/M1Fil.c.h5.gds.gz", "TOP", vec!["M1Fil.c"; 50], m1dens(&[]))]
#[case::m1fil_c_h6("metal1/M1Fil.c.h6.gds.gz", "TOP", vec!["M1Fil.c"; 50], m1dens(&[]))]
// A 0.005 Metal1 sliver, a 300 µm pair, a pair at (1000, 1000).
#[case::m1fil_c_h7("metal1/M1Fil.c.h7.gds.gz", "TOP", vec!["M1Fil.c"; 3], m1dens(&["M1.a", "M1.d", "M1Fil.a2"]))]
// 0.995, a 0.99 diagonal, corner-on 0.995, a TRANS diamond tip at 0.995 and an abutting
// TRANS fire; 1.0, 1.004, a crossing and an enclosed filler are clean.
#[case::m1fil_d_h1("metal1/M1Fil.d.h1.gds.gz", "TOP", vec!["M1Fil.d"; 5], m1dens(&[]))]
// Eleven 0.995 gaps on, across and straddling x = 20/21/40/42, at (1000, 1000), 300 µm.
#[case::m1fil_d_h2("metal1/M1Fil.d.h2.gds.gz", "TOP", vec!["M1Fil.d"; 11], m1dens(&["M1Fil.a2"]))]
// Fifty 0.995 pairs, flat and as a GdsArrayRef.
#[case::m1fil_d_h3("metal1/M1Fil.d.h3.gds.gz", "TOP", vec!["M1Fil.d"; 50], m1dens(&[]))]
#[case::m1fil_d_h4("metal1/M1Fil.d.h4.gds.gz", "TOP", vec!["M1Fil.d"; 50], m1dens(&[]))]
// A Metal5:filler 0.995 from a TRANS is not the metal1 deck's business.
#[case::m1fil_d_h5("metal1/M1Fil.d.h5.gds.gz", "TOP", vec![], m1dens(&[]))]
fn test_metal1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M1, gds, topcell, &ignore), expected);
}

// --- Metal2 ---

const DECK_M2: &str = "metal2";

#[rstest]
#[case::m2_a("metal2/M2.a.gds.gz", "TOP", vec!["M2.a", "M2.a", "M2.a", "M2.a"], vec!["M2.d", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_b_space("metal2/M2.b.space.gds.gz", "TOP", vec!["M2.b", "M2.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_b_notch("metal2/M2.b.notch.gds.gz", "TOP", vec!["M2.b", "M2.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_corner("metal2/M2.corner.gds.gz", "TOP", vec!["M2.a", "M2.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_c("metal2/M2.c.gds.gz", "TOP", vec!["M2.c"; 4], vec!["M2.c1", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_c1("metal2/M2.c1.gds.gz", "TOP", vec!["M2.c1"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_d("metal2/M2.d.gds.gz", "TOP", vec!["M2.d"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_e_fail("metal2/M2.e.fail.gds.gz", "TOP", vec!["M2.e"; 2], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_e_ok("metal2/M2.e.gds.gz", "TOP", vec![], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_f_fail("metal2/M2.f.fail.gds.gz", "TOP", vec!["M2.f"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_f_ok("metal2/M2.f.gds.gz", "TOP", vec![], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
// The 45° fixtures for M*.g and M*.i end their diagonal bars square to the axes, which
// leaves an acute 45° corner at each tip.  KLayout's `width` reports an acute corner
// at any value - the wedge narrows to nothing - and so does M*.a here; the fixture is
// about the bent width, so M*.a is set aside.
#[case::m2_g("metal2/M2.g.gds.gz", "TOP", vec!["M2.g", "M2.g"], vec!["M2.a", "M2.d", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_i("metal2/M2.i.gds.gz", "TOP", vec!["M2.i"], vec!["M2.a", "M2.b", "M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2_j_ok("metal2/M2.j.gds.gz", "TOP", vec![], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.h", "M2Fil.k"])]
#[case::m2_j_fail("metal2/M2.j.fail.gds.gz", "TOP", vec!["M2.j"], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.h", "M2Fil.k"])]
#[case::m2_k_ok("metal2/M2.k.gds.gz", "TOP", vec![], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.h", "M2Fil.k"])]
#[case::m2_k_fail("metal2/M2.k.fail.gds.gz", "TOP", vec!["M2.k"], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_c("metal2/M2Fil.c.gds.gz", "TOP", vec!["M2Fil.c", "M2Fil.c"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a1("metal2/M2Fil.a1.gds.gz", "TOP", vec!["M2Fil.a1"; 4], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a2("metal2/M2Fil.a2.gds.gz", "TOP", vec!["M2Fil.a2"; 2], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_b("metal2/M2Fil.b.gds.gz", "TOP", vec!["M2Fil.b", "M2Fil.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_d("metal2/M2Fil.d.gds.gz", "TOP", vec!["M2Fil.d", "M2Fil.d"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_h_ok("metal2/M2Fil.h.gds.gz", "TOP", vec![], vec!["M2.b", "M2.j"])]
#[case::m2fil_h_fail("metal2/M2Fil.h.fail.gds.gz", "TOP", vec!["M2Fil.h"], vec!["M2.b", "M2.j"])]
#[case::m2fil_k_ok("metal2/M2Fil.k.gds.gz", "TOP", vec![], vec!["M2.b", "M2.k"])]
#[case::m2fil_k_fail("metal2/M2Fil.k.fail.gds.gz", "TOP", vec!["M2Fil.k"], vec!["M2.b", "M2.k"])]
#[case::m2fil_h_boundary_ok("metal2/M2Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m2fil_h_boundary_fail("metal2/M2Fil.h.boundary_fail.gds.gz", "TOP", vec!["M2Fil.h"], vec!["M2.j"])]
#[case::m2fil_h_boundary_ring("metal2/M2Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M2, gds, topcell, &ignore), expected);
}

// --- Metal3 ---

const DECK_M3: &str = "metal3";

#[rstest]
#[case::m3_a("metal3/M3.a.gds.gz", "TOP", vec!["M3.a", "M3.a", "M3.a", "M3.a"], vec!["M3.d", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_b_space("metal3/M3.b.space.gds.gz", "TOP", vec!["M3.b", "M3.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_b_notch("metal3/M3.b.notch.gds.gz", "TOP", vec!["M3.b", "M3.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_corner("metal3/M3.corner.gds.gz", "TOP", vec!["M3.a", "M3.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_c("metal3/M3.c.gds.gz", "TOP", vec!["M3.c"; 4], vec!["M3.c1", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_c1("metal3/M3.c1.gds.gz", "TOP", vec!["M3.c1"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_d("metal3/M3.d.gds.gz", "TOP", vec!["M3.d"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_e_fail("metal3/M3.e.fail.gds.gz", "TOP", vec!["M3.e"; 2], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_e_ok("metal3/M3.e.gds.gz", "TOP", vec![], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_f_fail("metal3/M3.f.fail.gds.gz", "TOP", vec!["M3.f"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_f_ok("metal3/M3.f.gds.gz", "TOP", vec![], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_g("metal3/M3.g.gds.gz", "TOP", vec!["M3.g", "M3.g"], vec!["M3.a", "M3.d", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_i("metal3/M3.i.gds.gz", "TOP", vec!["M3.i"], vec!["M3.a", "M3.b", "M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3_j_ok("metal3/M3.j.gds.gz", "TOP", vec![], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.h", "M3Fil.k"])]
#[case::m3_j_fail("metal3/M3.j.fail.gds.gz", "TOP", vec!["M3.j"], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.h", "M3Fil.k"])]
#[case::m3_k_ok("metal3/M3.k.gds.gz", "TOP", vec![], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.h", "M3Fil.k"])]
#[case::m3_k_fail("metal3/M3.k.fail.gds.gz", "TOP", vec!["M3.k"], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_c("metal3/M3Fil.c.gds.gz", "TOP", vec!["M3Fil.c", "M3Fil.c"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a1("metal3/M3Fil.a1.gds.gz", "TOP", vec!["M3Fil.a1"; 4], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a2("metal3/M3Fil.a2.gds.gz", "TOP", vec!["M3Fil.a2"; 2], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_b("metal3/M3Fil.b.gds.gz", "TOP", vec!["M3Fil.b", "M3Fil.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_d("metal3/M3Fil.d.gds.gz", "TOP", vec!["M3Fil.d", "M3Fil.d"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_h_ok("metal3/M3Fil.h.gds.gz", "TOP", vec![], vec!["M3.b", "M3.j"])]
#[case::m3fil_h_fail("metal3/M3Fil.h.fail.gds.gz", "TOP", vec!["M3Fil.h"], vec!["M3.b", "M3.j"])]
#[case::m3fil_k_ok("metal3/M3Fil.k.gds.gz", "TOP", vec![], vec!["M3.b", "M3.k"])]
#[case::m3fil_k_fail("metal3/M3Fil.k.fail.gds.gz", "TOP", vec!["M3Fil.k"], vec!["M3.b", "M3.k"])]
#[case::m3fil_h_boundary_ok("metal3/M3Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m3fil_h_boundary_fail("metal3/M3Fil.h.boundary_fail.gds.gz", "TOP", vec!["M3Fil.h"], vec!["M3.j"])]
#[case::m3fil_h_boundary_ring("metal3/M3Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal3(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M3, gds, topcell, &ignore), expected);
}

// --- Metal4 ---

const DECK_M4: &str = "metal4";

#[rstest]
#[case::m4_a("metal4/M4.a.gds.gz", "TOP", vec!["M4.a", "M4.a", "M4.a", "M4.a"], vec!["M4.d", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_b_space("metal4/M4.b.space.gds.gz", "TOP", vec!["M4.b", "M4.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_b_notch("metal4/M4.b.notch.gds.gz", "TOP", vec!["M4.b", "M4.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_corner("metal4/M4.corner.gds.gz", "TOP", vec!["M4.a", "M4.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_c("metal4/M4.c.gds.gz", "TOP", vec!["M4.c"; 4], vec!["M4.c1", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_c1("metal4/M4.c1.gds.gz", "TOP", vec!["M4.c1"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_d("metal4/M4.d.gds.gz", "TOP", vec!["M4.d"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_e_fail("metal4/M4.e.fail.gds.gz", "TOP", vec!["M4.e"; 2], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_e_ok("metal4/M4.e.gds.gz", "TOP", vec![], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_f_fail("metal4/M4.f.fail.gds.gz", "TOP", vec!["M4.f"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_f_ok("metal4/M4.f.gds.gz", "TOP", vec![], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_g("metal4/M4.g.gds.gz", "TOP", vec!["M4.g", "M4.g"], vec!["M4.a", "M4.d", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_i("metal4/M4.i.gds.gz", "TOP", vec!["M4.i"], vec!["M4.a", "M4.b", "M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4_j_ok("metal4/M4.j.gds.gz", "TOP", vec![], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.h", "M4Fil.k"])]
#[case::m4_j_fail("metal4/M4.j.fail.gds.gz", "TOP", vec!["M4.j"], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.h", "M4Fil.k"])]
#[case::m4_k_ok("metal4/M4.k.gds.gz", "TOP", vec![], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.h", "M4Fil.k"])]
#[case::m4_k_fail("metal4/M4.k.fail.gds.gz", "TOP", vec!["M4.k"], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_c("metal4/M4Fil.c.gds.gz", "TOP", vec!["M4Fil.c", "M4Fil.c"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a1("metal4/M4Fil.a1.gds.gz", "TOP", vec!["M4Fil.a1"; 4], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a2("metal4/M4Fil.a2.gds.gz", "TOP", vec!["M4Fil.a2"; 2], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_b("metal4/M4Fil.b.gds.gz", "TOP", vec!["M4Fil.b", "M4Fil.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_d("metal4/M4Fil.d.gds.gz", "TOP", vec!["M4Fil.d", "M4Fil.d"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_h_ok("metal4/M4Fil.h.gds.gz", "TOP", vec![], vec!["M4.b", "M4.j"])]
#[case::m4fil_h_fail("metal4/M4Fil.h.fail.gds.gz", "TOP", vec!["M4Fil.h"], vec!["M4.b", "M4.j"])]
#[case::m4fil_k_ok("metal4/M4Fil.k.gds.gz", "TOP", vec![], vec!["M4.b", "M4.k"])]
#[case::m4fil_k_fail("metal4/M4Fil.k.fail.gds.gz", "TOP", vec!["M4Fil.k"], vec!["M4.b", "M4.k"])]
#[case::m4fil_h_boundary_ok("metal4/M4Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m4fil_h_boundary_fail("metal4/M4Fil.h.boundary_fail.gds.gz", "TOP", vec!["M4Fil.h"], vec!["M4.j"])]
#[case::m4fil_h_boundary_ring("metal4/M4Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal4(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M4, gds, topcell, &ignore), expected);
}

// --- Metal5 ---

const DECK_M5: &str = "metal5";

#[rstest]
#[case::m5_a("metal5/M5.a.gds.gz", "TOP", vec!["M5.a", "M5.a", "M5.a", "M5.a"], vec!["M5.d", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_b_space("metal5/M5.b.space.gds.gz", "TOP", vec!["M5.b", "M5.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_b_notch("metal5/M5.b.notch.gds.gz", "TOP", vec!["M5.b", "M5.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_corner("metal5/M5.corner.gds.gz", "TOP", vec!["M5.a", "M5.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_c("metal5/M5.c.gds.gz", "TOP", vec!["M5.c"; 4], vec!["M5.c1", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_c1("metal5/M5.c1.gds.gz", "TOP", vec!["M5.c1"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_d("metal5/M5.d.gds.gz", "TOP", vec!["M5.d"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_e_fail("metal5/M5.e.fail.gds.gz", "TOP", vec!["M5.e"; 2], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_e_ok("metal5/M5.e.gds.gz", "TOP", vec![], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_f_fail("metal5/M5.f.fail.gds.gz", "TOP", vec!["M5.f"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_f_ok("metal5/M5.f.gds.gz", "TOP", vec![], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_g("metal5/M5.g.gds.gz", "TOP", vec!["M5.g", "M5.g"], vec!["M5.a", "M5.d", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_i("metal5/M5.i.gds.gz", "TOP", vec!["M5.i"], vec!["M5.a", "M5.b", "M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5_j_ok("metal5/M5.j.gds.gz", "TOP", vec![], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.h", "M5Fil.k"])]
#[case::m5_j_fail("metal5/M5.j.fail.gds.gz", "TOP", vec!["M5.j"], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.h", "M5Fil.k"])]
#[case::m5_k_ok("metal5/M5.k.gds.gz", "TOP", vec![], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.h", "M5Fil.k"])]
#[case::m5_k_fail("metal5/M5.k.fail.gds.gz", "TOP", vec!["M5.k"], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_c("metal5/M5Fil.c.gds.gz", "TOP", vec!["M5Fil.c", "M5Fil.c"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a1("metal5/M5Fil.a1.gds.gz", "TOP", vec!["M5Fil.a1"; 4], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a2("metal5/M5Fil.a2.gds.gz", "TOP", vec!["M5Fil.a2"; 2], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_b("metal5/M5Fil.b.gds.gz", "TOP", vec!["M5Fil.b", "M5Fil.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_d("metal5/M5Fil.d.gds.gz", "TOP", vec!["M5Fil.d", "M5Fil.d"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_h_ok("metal5/M5Fil.h.gds.gz", "TOP", vec![], vec!["M5.b", "M5.j"])]
#[case::m5fil_h_fail("metal5/M5Fil.h.fail.gds.gz", "TOP", vec!["M5Fil.h"], vec!["M5.b", "M5.j"])]
#[case::m5fil_k_ok("metal5/M5Fil.k.gds.gz", "TOP", vec![], vec!["M5.b", "M5.k"])]
#[case::m5fil_k_fail("metal5/M5Fil.k.fail.gds.gz", "TOP", vec!["M5Fil.k"], vec!["M5.b", "M5.k"])]
#[case::m5fil_h_boundary_ok("metal5/M5Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m5fil_h_boundary_fail("metal5/M5Fil.h.boundary_fail.gds.gz", "TOP", vec!["M5Fil.h"], vec!["M5.j"])]
#[case::m5fil_h_boundary_ring("metal5/M5Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
fn test_metal5(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_M5, gds, topcell, &ignore), expected);
}

// --- Metal(n=2-5): the hardening layouts ---
//
// Section 5.17 (Mn.a-Mn.k) and section 5.18 (MnFil.*) are one rule set on four layers, and
// so is every layout: `metal<n>/M<n>.<rule>.h<k>.gds.gz` carries the same geometry for
// n = 2..5 (Via(n-1) and Metal(n-1) below).  The rule ids are given as suffixes and the
// layer index is a second axis of the table, so a layer that answers differently fails on
// its own.  Expected values are the manual's (ci/hardening/reports/ihp-sg13g2/metaln.md);
// `mn_dens` sets the density rules aside, which every small layout trips.

/// The density rules, which every small layout trips, plus whatever else a layout draws
/// on purpose (a sliver under Mn.a and Mn.d, a short 45° strip under Mn.d, ...).
fn mn_dens(extra: &[&'static str]) -> Vec<&'static str> {
    [&[".j", ".k", "Fil.h", "Fil.k"][..], extra].concat()
}

#[rstest]
// Mn.a: 0.20 legal, 0.195 fires in x and y; a 300 µm bar once (two markers per bar).
#[case::mn_a_h1(".a.h1", vec![".a"; 6], mn_dens(&[]))]
// 45°: a 0.198 diamond (4 walls) and a 0.198 strip (2) fire; the 0.205 ones, a chamfered box
// and an L with a chamfered inner corner are clean.  The narrow strips are also Mn.g's.
#[case::mn_a_h2(".a.h2", vec![".a"; 6], mn_dens(&[".d", ".g"]))]
// Unions: a 0.195 union of two boxes, three slices plus a 0.045, a ring's 0.195 side and a
// 0.195 island in a ring fire; the 0.20 union, four slices and a gridded bar are clean.
#[case::mn_a_h3(".a.h3", vec![".a"; 8], mn_dens(&[]))]
// Tile lines: ten 0.195 bars and an L on, across and beside x = 20/21/40/42.
#[case::mn_a_h4(".a.h4", vec![".a"; 20], mn_dens(&[]))]
// Fifty 0.195 bars, flat and as an array reference.
#[case::mn_a_h5(".a.h5", vec![".a"; 100], mn_dens(&[]))]
#[case::mn_a_h6(".a.h6", vec![".a"; 100], mn_dens(&[]))]
// A 0.005 sliver and a bar at (1000, 1000).
#[case::mn_a_h7(".a.h7", vec![".a"; 4], mn_dens(&[".d"]))]
// A comb with three 0.195 teeth; a U with 0.20 arms is clean.
#[case::mn_a_h8(".a.h8", vec![".a"; 6], mn_dens(&[]))]
// Section 6.10: metal rules are not checked within EdgeSeal.  The 0.195 bar and the 0.205
// pair under the seal are exempt; the same outside fire (finding 3).
#[case::mn_a_h9(".a.h9", vec![".a", ".a", ".b"], mn_dens(&[]))]
// Mn.b: 0.205 fires in x, y, corner to corner (0.2051) and corner-on; 0.21, 0.212 and a
// 0.2/0.2 diagonal (0.283 euclidian) are clean.
#[case::mn_b_h1(".b.h1", vec![".b"; 4], mn_dens(&[]))]
// 45°: a diamond tip at 0.205, two strips at 0.205, a chamfer 0.205 from a corner and two
// tips 0.205 apart fire Mn.b; those and the 0.21/0.212 variants fire Mn.i (a 45° edge).
#[case::mn_b_h2(".b.h2", [vec![".b"; 4], vec![".i"; 6]].concat(), mn_dens(&[]))]
// Notches: a U (2), a straight-vs-45° foot (2), a comb (3), a slot (1), a keyhole hole (1),
// two facing Ls (1, also Mn.e: 1.0-wide arms, run 1.8) and an island in a ring (1).
#[case::mn_b_h3(".b.h3", [vec![".b"; 11], vec![".e"]].concat(), mn_dens(&[]))]
// Unions: a two-box union and a gridded box each face a neighbour at 0.205.
#[case::mn_b_h4(".b.h4", vec![".b"; 2], mn_dens(&[]))]
// Tile lines: 0.205 gaps on, straddling and across x = 20/21/40/42, a corner pair on 20 (8);
// the 10 µm bars are 1.0 wide, so their gap is Mn.e's too.
#[case::mn_b_h5(".b.h5", [vec![".b"; 8], vec![".e"]].concat(), mn_dens(&[]))]
// Fifty 0.205 pairs, flat and as an array (a 1.0 run: Mn.e stays quiet).
#[case::mn_b_h6(".b.h6", vec![".b"; 50], mn_dens(&[]))]
#[case::mn_b_h7(".b.h7", vec![".b"; 50], mn_dens(&[]))]
// A 0.005 sliver 0.205 from a box, 300 µm bars 0.205 apart, a pair at (1000, 1000); the
// wide box and the wide bars draw Mn.e as well.
#[case::mn_b_h8(".b.h8", vec![".b", ".b", ".b", ".e", ".e"], mn_dens(&[".a", ".d"]))]
// No net condition: a pair strapped through Via(n-1)/Metal(n-1) fires like the bare one.
#[case::mn_b_h9(".b.h9", vec![".b", ".b", ".e", ".e"], mn_dens(&[]))]
// Mn.c: a via on the line's edge, one 0.005 out and one in a pad's corner fire; 0.005 all
// round is clean.
#[case::mn_c_h1(".c.h1", vec![".c"; 3], mn_dens(&[".c1"]))]
// A bare via, a via half out of a line's end and a via beside a line: none is enclosed.
#[case::mn_c_h2(".c.h2", vec![".c"; 3], mn_dens(&[".c1"]))]
// A chamfer through the via's corner (touch, 0.000) and one cutting it fire; the chamfer
// passing 0.0035 from the corner with 0.005 walls is the settled projection reading (finding 4).
// The chamfer through the via's corner is OPEN (the projection reading pairs no wall with
// the corner touch; KLayout reports it) - the case carries the current answer, one.
#[case::mn_c_h3(".c.h3", vec![".c"; 1], mn_dens(&[".c1"]))]
// Unions: two boxes enclosing the via by 0.005 together are clean, by 0.000 fire once; ten
// abutting slices are clean.
#[case::mn_c_h4(".c.h4", vec![".c"; 1], mn_dens(&[]))]
// Tile lines: vias on the line's edge straddling x = 20/21/40/42 and one with its edge on
// 20; the via ending 0.005 short of x = 20 sits 0.005 from its line's end with 0.005
// sides, which is Mn.c1's (finding 1).
#[case::mn_c_h5(".c.h5", [vec![".c"; 5], vec![".c1"]].concat(), mn_dens(&[]))]
// Fifty vias on a line's edge, flat and as an array.
#[case::mn_c_h6(".c.h6", vec![".c"; 50], mn_dens(&[]))]
#[case::mn_c_h7(".c.h7", vec![".c"; 50], mn_dens(&[]))]
// A via on a line's edge at (1000, 1000) and on a 300 µm line.
#[case::mn_c_h8(".c.h8", vec![".c"; 2], mn_dens(&[]))]
// A via on the line's edge under an EdgeSeal is exempt; outside and across the seal's
// edge it fires.
#[case::mn_c_h9(".c.h9", vec![".c"; 2], mn_dens(&[]))]
// Mn.c1: line-end vias with a 0.045 endcap (right, top, left, the outer of two) and one with
// 0.000 (also Mn.c) fire; 0.05 and a via mid-line are clean (finding 1).
#[case::mn_c1_h1(".c1.h1", [vec![".c1"; 5], vec![".c"]].concat(), mn_dens(&[]))]
// Corners (note 1): outer margins (0.045, 0.005) and (0.005, 0.005) fire; (0.05, 0.005),
// (0.05, 0.05) and (0.005, 0.05) are clean.
#[case::mn_c1_h2(".c1.h2", vec![".c1"; 2], mn_dens(&[]))]
// Pads: one 0.05 side and three 0.005, two adjacent 0.05 sides, 0.045 all round and 0.005
// all round fire; two opposite 0.05 sides, three and four are clean.  All under Mn.d.
#[case::mn_c1_h3(".c1.h3", vec![".c1"; 4], mn_dens(&[".d"]))]
// A 0.045 endcap on a 0.5-wide line's end (0.155 sides: an opposite pair over 0.05), a
// 0.05 endcap, a T and a cross are clean.
#[case::mn_c1_h4(".c1.h4", vec![], mn_dens(&[]))]
// Tile lines: 0.045 endcaps ending on x = 20/21/40, vias straddling 20 and 42; 0.05 on 20 clean.
#[case::mn_c1_h5(".c1.h5", vec![".c1"; 5], mn_dens(&[]))]
// Fifty 0.045 endcaps, flat and as an array.
#[case::mn_c1_h6(".c1.h6", vec![".c1"; 50], mn_dens(&[]))]
#[case::mn_c1_h7(".c1.h7", vec![".c1"; 50], mn_dens(&[]))]
// A 0.045 endcap at (1000, 1000) and at the far end of a 300 µm line.
#[case::mn_c1_h8(".c1.h8", vec![".c1"; 2], mn_dens(&[]))]
// Mn.d: 0.142 fires as a box (x, y), an L and a 0.1405 diamond; 0.144 and 0.1458 are clean.
#[case::mn_d_h1(".d.h1", vec![".d"; 4], mn_dens(&[]))]
// Unions: a cross whose union is 0.12, two corner-touching 0.09 boxes (two regions), an
// island in a ring and a 0.14 bar fire; abutting boxes, a grid, a ring and 0.144 are clean.
#[case::mn_d_h2(".d.h2", vec![".d"; 5], mn_dens(&[".a", ".b"]))]
// Tile lines: 0.14 bars on, across and beside x = 20/21/40/42 and across y = 20; 0.144 across 20 clean.
#[case::mn_d_h3(".d.h3", vec![".d"; 8], mn_dens(&[]))]
// Fifty 0.14 bars, flat and as an array.
#[case::mn_d_h4(".d.h4", vec![".d"; 50], mn_dens(&[]))]
#[case::mn_d_h5(".d.h5", vec![".d"; 50], mn_dens(&[]))]
// A 0.01 sliver and a 0.14 bar at (1000, 1000) fire; a 0.144 sliver and a 300 µm line are clean.
#[case::mn_d_h6(".d.h6", vec![".d"; 2], mn_dens(&[".a"]))]
// A 0.14 bar under an EdgeSeal is exempt (section 6.10); one outside fires (finding 3).
#[case::mn_d_h7(".d.h7", vec![".d"; 1], mn_dens(&[]))]
// Mn.e: 0.235 fires (x and y), 0.24 is clean; 0.395 is wide, 0.39 is not; a 1.005 run fires,
// 1.0 does not.
#[case::mn_e_h1(".e.h1", vec![".e"; 4], mn_dens(&[]))]
// Wide/narrow, narrow/wide and wide/wide fire, narrow/narrow does not; a wide line between
// two narrow ones fires twice.
#[case::mn_e_h2(".e.h2", vec![".e"; 5], mn_dens(&[]))]
// A 1.005 bump, a 1.005 stagger and a line whose 0.5-wide part sits on its far side (the
// facing wall straight, run 2) fire; a 0.8 bump and a 1.0 stagger are clean (finding 2).
#[case::mn_e_h3(".e.h3", vec![".e"; 3], mn_dens(&[]))]
// An L pad, a pad stepping wider on its far side, a plate beside a line and a plate's end
// fire; a near-side 0.6 step and a 0.5 line's end are clean (finding 2).
#[case::mn_e_h4(".e.h4", vec![".e"; 4], mn_dens(&[]))]
// 45°: two 0.509 strips and a 0.509/0.2404 pair at 0.2333 fire Mn.e and Mn.i; two 0.2404
// strips at 0.2333 fire Mn.i only; 0.2404 apart is clean.
#[case::mn_e_h5(".e.h5", vec![".e", ".e", ".i", ".i", ".i"], mn_dens(&[]))]
// Unions: a 0.395 union and a run of two abutting pieces fire; a 0.39 union and a neighbour
// split into two 0.8 lines are clean.
#[case::mn_e_h6(".e.h6", vec![".e"; 2], mn_dens(&[]))]
// A wide U with a 0.235 notch: "space", not "space or notch" (note A).
#[case::mn_e_h7(".e.h7", vec![], mn_dens(&[]))]
// Tile lines: runs split by x = 20/21/40, gaps straddling 20/42, a wall on 20, a 10 µm run
// and a plate cornered on (20, 20) fire; a 1.0 run centred on 20 is clean.
#[case::mn_e_h8(".e.h8", vec![".e"; 8], mn_dens(&[]))]
// Fifty wide/narrow pairs, flat and as an array.
#[case::mn_e_h9(".e.h9", vec![".e"; 50], mn_dens(&[]))]
#[case::mn_e_h10(".e.h10", vec![".e"; 50], mn_dens(&[]))]
// A 300 µm pair and a pair at (1000, 1000).
#[case::mn_e_h11(".e.h11", vec![".e"; 2], mn_dens(&[]))]
// Two wide lines 0.205 apart: Mn.b and Mn.e.
#[case::mn_e_h12(".e.h12", vec![".b", ".e"], mn_dens(&[]))]
// Mn.f: 0.595 fires, 0.60 is clean; 10.005 is wide, 10.0 is not; a 10.005 run fires, 10.0 not.
#[case::mn_f_h1(".f.h1", vec![".f"; 3], mn_dens(&[]))]
// A narrow line beside a plate, an L plate's arm and a 10.005 pad on a line fire; two 10 × 12
// plates and a 10.005 × 9 pad are clean.
#[case::mn_f_h2(".f.h2", vec![".f"; 3], mn_dens(&[]))]
// Tile lines: runs across y = 20, gaps straddling x = 20/42, walls on 20/40, a 10.005 run
// split by y = 20 fire; a 10.0 run split by y = 20 is clean.
#[case::mn_f_h3(".f.h3", vec![".f"; 6], mn_dens(&[]))]
// Fifty plate pairs, flat and as an array.
#[case::mn_f_h4(".f.h4", vec![".f"; 50], mn_dens(&[]))]
#[case::mn_f_h5(".f.h5", vec![".f"; 50], mn_dens(&[]))]
// 300 µm plates and a pair at (1000, 1000).
#[case::mn_f_h6(".f.h6", vec![".f"; 2], mn_dens(&[]))]
// 45°: 12-wide strips 0.594 apart fire, 0.601 apart are clean.
#[case::mn_f_h7(".f.h7", vec![".f"; 1], mn_dens(&[]))]
// Mn.g: a 0.2333 strip with 1.41 walls and one with 0.502 walls fire (two markers each);
// 0.2404 and 0.495 walls are clean.  The short strips are under Mn.d.
#[case::mn_g_h1(".g.h1", vec![".g"; 4], mn_dens(&[".d"]))]
// A 0.2 line's 45° jog with 0.509 walls and a pad's arm with 0.707/0.502 walls fire; a
// 0.495 jog, a 0.24 line's jog and an arm with 0.601/0.396 walls (settled: each wall its
// own length) are clean.
#[case::mn_g_h2(".g.h2", vec![".g"; 4], mn_dens(&[]))]
// An L with chamfered bend: 0.707/0.544 walls fire; 0.566/0.403 and 0.424/0.262 are clean;
// a 0.2333 diamond's walls are short.
#[case::mn_g_h3(".g.h3", vec![".g"; 2], mn_dens(&[".d"]))]
// A 300 µm strip (once), strips across x = 20/21/40/42 and y = 20 with 0.502 walls the tile
// line splits (seven strips, two markers each); 0.495 walls across 20 are clean.
#[case::mn_g_h4(".g.h4", vec![".g"; 14], mn_dens(&[".d"]))]
// Fifty 0.2333 strips, flat and as an array.
#[case::mn_g_h5(".g.h5", vec![".g"; 100], mn_dens(&[]))]
#[case::mn_g_h6(".g.h6", vec![".g"; 100], mn_dens(&[]))]
// A strip at (1000, 1000); a 0.198 strip under Mn.a as well.
#[case::mn_g_h7(".g.h7", vec![".g", ".g", ".g", ".g", ".a", ".a"], mn_dens(&[]))]
// Mn.i: 0.304 strips 0.2333 apart fire, 0.2404 apart are clean.
#[case::mn_i_h1(".i.h1", vec![".i"; 1], mn_dens(&[]))]
// A corner 0.2333 from a 45° wall, a tip 0.235 above a wall and a chamfer 0.2333 from a
// corner fire (Mn.b clean); 0.2404/0.24 and a straight corner pair are clean.
#[case::mn_i_h2(".i.h2", vec![".i"; 3], mn_dens(&[]))]
// A strip's square end 0.235 above a wall fires; a hairpin's 0.2333 notch is "space" (note A).
#[case::mn_i_h3(".i.h3", vec![".i"; 1], mn_dens(&[]))]
// Tile lines: strip pairs across x = 20/21/40/42, inside a tile, and a corner on 20 against a 45° wall.
#[case::mn_i_h4(".i.h4", vec![".i"; 6], mn_dens(&[]))]
// Fifty strip pairs, flat and as an array.
#[case::mn_i_h5(".i.h5", vec![".i"; 50], mn_dens(&[]))]
#[case::mn_i_h6(".i.h6", vec![".i"; 50], mn_dens(&[]))]
// 300 µm strips and a pair at (1000, 1000).
#[case::mn_i_h7(".i.h7", vec![".i"; 2], mn_dens(&[]))]
// Strips 0.2051 apart: Mn.b and Mn.i.
#[case::mn_i_h8(".i.h8", vec![".b", ".i"], mn_dens(&[]))]
// Mn.j: 30 % stripes drawn on all three density layers are 30 % of the die, not 90.
#[case::mn_j_h1(".j.h1", vec![".j"], vec!["Fil.a2", "Fil.b", "Fil.c"])]
// MnFil.h: a 700 hole in a 51 % die; the 800 window holding it reads 23.4 %.
#[case::mnfil_h_h1("Fil.h.h1", vec!["Fil.h"], vec![])]
// MnFil.a1: 0.995 bars (x, y, a union) and a 0.99 diamond fire; 1.0, 1.004 and an L are clean.
#[case::mnfil_a1_h1("Fil.a1.h1", vec!["Fil.a1"; 10], mn_dens(&[]))]
// MnFil.a2: a 5.005 × 5.005 square fires (four walls); 5.005 × 5.0, a 3 × 20 bar, an L with
// 3-wide arms and a frame with 2.25 walls are under 5 wide (finding 6).
// The metal-filler maximum is the bounding box's long side, as upstream's
// `with_bbox_max` (OPEN, the PDK owner's call): all five shapes, one marker each.
#[case::mnfil_a2_h1("Fil.a2.h1", vec!["Fil.a2"; 5], mn_dens(&[]))]
// MnFil.b: 0.415 (x, y), 0.41 corner to corner and a tip at 0.415 fire; 0.42, 0.424 and a
// U's 0.415 notch (note A) are clean.
#[case::mnfil_b_h1("Fil.b.h1", vec!["Fil.b"; 4], mn_dens(&[]))]
// Tile lines: 0.415 gaps straddling x = 20/21/40/42 and inside a tile.
#[case::mnfil_b_h2("Fil.b.h2", vec!["Fil.b"; 5], mn_dens(&[]))]
// Fifty 0.415 pairs, flat and as an array.
#[case::mnfil_b_h3("Fil.b.h3", vec!["Fil.b"; 50], mn_dens(&[]))]
#[case::mnfil_b_h4("Fil.b.h4", vec!["Fil.b"; 50], mn_dens(&[]))]
// MnFil.c: 0.415 (x, y), an abutting metal (0.000) and 0.41 corner to corner fire; 0.42 and
// an overlapping metal (settled: no pair) are clean (finding 5).
#[case::mnfil_c_h1("Fil.c.h1", vec!["Fil.c"; 4], mn_dens(&[]))]
// Tile lines: metal 0.415 from a filler across x = 20/21/40/42 and inside a tile.
#[case::mnfil_c_h2("Fil.c.h2", vec!["Fil.c"; 5], mn_dens(&[]))]
// Fifty filler/metal pairs, flat and as an array.
#[case::mnfil_c_h3("Fil.c.h3", vec!["Fil.c"; 50], mn_dens(&[]))]
#[case::mnfil_c_h4("Fil.c.h4", vec!["Fil.c"; 50], mn_dens(&[]))]
// MnFil.d: 0.995 (x, y), 0.99 corner to corner and a filler inside a TRANS marker fire;
// 1.004 and a filler across the marker's edge (settled: no pair) are clean (finding 5).
// The filler well inside the TRANS (2 µm from its edges) is clean, as settled for
// AFil.e: the marker encloses it by more than the value.
#[case::mnfil_d_h1("Fil.d.h1", vec!["Fil.d"; 3], mn_dens(&[]))]
fn test_metaln_hardening(
    #[case] gds: &str,
    #[case] expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
    #[values(2, 3, 4, 5)] n: u32,
) {
    let deck = format!("metal{n}");
    let gds = format!("metal{n}/M{n}{gds}.gds.gz");
    let id = |s: &&str| format!("M{n}{s}");
    let ignore: Vec<String> = ignore.iter().map(id).collect();
    let ignore: Vec<&str> = ignore.iter().map(String::as_str).collect();
    let mut expected: Vec<String> = expected.iter().map(id).collect();
    expected.sort();
    assert_eq!(drc(PDK_IHP, &deck, &gds, "TOP", &ignore), expected);
}

// --- Via1 ---

const DECK_V1: &str = "via1";

#[rstest]
#[case::v1_a("via1/V1.a.gds.gz", "TOP", vec!["V1.a"; 8], vec!["V1.c", "V1.c1"])]
#[case::v1_b("via1/V1.b.gds.gz", "TOP", vec!["V1.b", "V1.b"], vec!["V1.c", "V1.c1"])]
#[case::v1_c("via1/V1.c.gds.gz", "TOP", vec!["V1.c"; 4], vec!["V1.a", "V1.b", "V1.c1"])]
#[case::v1_c1("via1/V1.c1.gds.gz", "TOP", vec!["V1.c1"], vec!["V1.a", "V1.b", "V1.c"])]
#[case::v1_b1_fail("via1/V1.b1.fail.gds.gz", "TOP", vec!["V1.b1"], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
#[case::v1_b1_ok("via1/V1.b1.gds.gz", "TOP", vec![], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
#[case::v1_b1_ring("via1/V1.b1.ring.gds.gz", "TOP", vec![], vec!["V1.a", "V1.b", "V1.c", "V1.c1"])]
fn test_via1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V1, gds, topcell, &ignore), expected);
}

// --- Via2 ---

const DECK_V2: &str = "via2";

#[rstest]
#[case::v2_a("via2/V2.a.gds.gz", "TOP", vec!["V2.a"; 8], vec!["V2.c", "V2.c1"])]
#[case::v2_b("via2/V2.b.gds.gz", "TOP", vec!["V2.b", "V2.b"], vec!["V2.c", "V2.c1"])]
#[case::v2_c("via2/V2.c.gds.gz", "TOP", vec!["V2.c"; 4], vec!["V2.a", "V2.b", "V2.c1"])]
#[case::v2_c1("via2/V2.c1.gds.gz", "TOP", vec!["V2.c1"], vec!["V2.a", "V2.b", "V2.c"])]
#[case::v2_b1_fail("via2/V2.b1.fail.gds.gz", "TOP", vec!["V2.b1"], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
#[case::v2_b1_ok("via2/V2.b1.gds.gz", "TOP", vec![], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
#[case::v2_b1_ring("via2/V2.b1.ring.gds.gz", "TOP", vec![], vec!["V2.a", "V2.b", "V2.c", "V2.c1"])]
fn test_via2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V2, gds, topcell, &ignore), expected);
}

// --- Via3 ---

const DECK_V3: &str = "via3";

#[rstest]
#[case::v3_a("via3/V3.a.gds.gz", "TOP", vec!["V3.a"; 8], vec!["V3.c", "V3.c1"])]
#[case::v3_b("via3/V3.b.gds.gz", "TOP", vec!["V3.b", "V3.b"], vec!["V3.c", "V3.c1"])]
#[case::v3_c("via3/V3.c.gds.gz", "TOP", vec!["V3.c"; 4], vec!["V3.a", "V3.b", "V3.c1"])]
#[case::v3_c1("via3/V3.c1.gds.gz", "TOP", vec!["V3.c1"], vec!["V3.a", "V3.b", "V3.c"])]
#[case::v3_b1_fail("via3/V3.b1.fail.gds.gz", "TOP", vec!["V3.b1"], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
#[case::v3_b1_ok("via3/V3.b1.gds.gz", "TOP", vec![], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
#[case::v3_b1_ring("via3/V3.b1.ring.gds.gz", "TOP", vec![], vec!["V3.a", "V3.b", "V3.c", "V3.c1"])]
fn test_via3(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V3, gds, topcell, &ignore), expected);
}

// --- Via4 ---

const DECK_V4: &str = "via4";

#[rstest]
#[case::v4_a("via4/V4.a.gds.gz", "TOP", vec!["V4.a"; 8], vec!["V4.c", "V4.c1"])]
#[case::v4_b("via4/V4.b.gds.gz", "TOP", vec!["V4.b", "V4.b"], vec!["V4.c", "V4.c1"])]
#[case::v4_c("via4/V4.c.gds.gz", "TOP", vec!["V4.c"; 4], vec!["V4.a", "V4.b", "V4.c1"])]
#[case::v4_c1("via4/V4.c1.gds.gz", "TOP", vec!["V4.c1"], vec!["V4.a", "V4.b", "V4.c"])]
#[case::v4_b1_fail("via4/V4.b1.fail.gds.gz", "TOP", vec!["V4.b1"], vec!["V4.a", "V4.b", "V4.c", "V4.c1"])]
#[case::v4_b1_ok("via4/V4.b1.gds.gz", "TOP", vec![], vec!["V4.a", "V4.b", "V4.c", "V4.c1"])]
fn test_via4(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_V4, gds, topcell, &ignore), expected);
}

// --- TopVia1 ---

const DECK_TV1: &str = "topvia1";

#[rstest]
#[case::tv1_a("topvia1/TV1.a.gds.gz", "TOP", vec!["TV1.a"; 8], vec!["TV1.c", "TV1.d"])]
#[case::tv1_b("topvia1/TV1.b.gds.gz", "TOP", vec!["TV1.b", "TV1.b"], vec!["TV1.c", "TV1.d"])]
#[case::tv1_c("topvia1/TV1.c.gds.gz", "TOP", vec!["TV1.c"; 4], vec!["TV1.a", "TV1.b", "TV1.d"])]
#[case::tv1_d("topvia1/TV1.d.gds.gz", "TOP", vec!["TV1.d"; 4], vec!["TV1.a", "TV1.b", "TV1.c"])]
fn test_topvia1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TV1, gds, topcell, &ignore), expected);
}

// --- TopVia2 ---

const DECK_TV2: &str = "topvia2";

#[rstest]
#[case::tv2_a("topvia2/TV2.a.gds.gz", "TOP", vec!["TV2.a"; 8], vec!["TV2.c", "TV2.d"])]
#[case::tv2_b("topvia2/TV2.b.gds.gz", "TOP", vec!["TV2.b", "TV2.b"], vec!["TV2.c", "TV2.d"])]
#[case::tv2_c("topvia2/TV2.c.gds.gz", "TOP", vec!["TV2.c"; 4], vec!["TV2.a", "TV2.b", "TV2.d"])]
#[case::tv2_d("topvia2/TV2.d.gds.gz", "TOP", vec!["TV2.d"; 4], vec!["TV2.a", "TV2.b", "TV2.c"])]
fn test_topvia2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TV2, gds, topcell, &ignore), expected);
}

// --- TopMetal1 ---

const DECK_TM1: &str = "topmetal1";

#[rstest]
#[case::tm1_a("topmetal1/TM1.a.gds.gz", "TOP", vec!["TM1.a", "TM1.a", "TM1.a", "TM1.a"], vec!["TM1.c", "TM1.d"])]
#[case::tm1_b_space("topmetal1/TM1.b.space.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.c", "TM1.d"])]
#[case::tm1_b_notch("topmetal1/TM1.b.notch.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.a", "TM1.c", "TM1.d"])]
#[case::tm1_b_mixed_notch("topmetal1/TM1.b.mixed_notch.gds.gz", "TOP", vec!["TM1.b", "TM1.b"], vec!["TM1.a", "TM1.c", "TM1.d"])]
#[case::tm1_c_ok("topmetal1/TM1.c.gds.gz", "TOP", vec![], vec!["TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_c_fail("topmetal1/TM1.c.fail.gds.gz", "TOP", vec!["TM1.c"], vec!["TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_d_ok("topmetal1/TM1.d.gds.gz", "TOP", vec![], vec!["TM1.b", "TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1_d_fail("topmetal1/TM1.d.fail.gds.gz", "TOP", vec!["TM1.d"], vec!["TM1.b", "TM1Fil.a", "TM1Fil.a1"])]
#[case::tm1fil_c("topmetal1/TM1Fil.c.gds.gz", "TOP", vec!["TM1Fil.c", "TM1Fil.c"], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a("topmetal1/TM1Fil.a.gds.gz", "TOP", vec!["TM1Fil.a"; 4], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a1("topmetal1/TM1Fil.a1.gds.gz", "TOP", vec!["TM1Fil.a1"; 2], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_b("topmetal1/TM1Fil.b.gds.gz", "TOP", vec!["TM1Fil.b"; 2], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_d("topmetal1/TM1Fil.d.gds.gz", "TOP", vec!["TM1Fil.d"; 2], vec!["TM1.c", "TM1.d"])]
// Hardening (ci/hardening/reports/ihp-sg13g2/topmetal.md).  TM1.a: 1.635 bars in x and y, a
// 300 µm bar, a 0.005 sliver, a bar at (1000, 1000): two markers each; 1.64 is clean.
#[case::tm1_a_h1("topmetal1/TM1.a.h1.gds.gz", "TOP", vec!["TM1.a"; 10], vec!["TM1.c", "TM1.d"])]
// A diamond (4) and a 45° strip (2) one grid step under the width; the on-grid step
// above, a chamfered box and an L with a chamfered inner corner are clean.
#[case::tm1_a_h2("topmetal1/TM1.a.h2.gds.gz", "TOP", vec!["TM1.a"; 6], vec!["TM1.c", "TM1.d"])]
// Unions one step narrow (overlap, slices, one ring side, an island) fire once each; the
// unions at the width and a gridded bar are clean.
#[case::tm1_a_h3("topmetal1/TM1.a.h3.gds.gz", "TOP", vec!["TM1.a"; 8], vec!["TM1.c", "TM1.d"])]
// Ten narrow bars on, across and straddling x = 20/21/40/42 plus an L cornered on 20.
#[case::tm1_a_h4("topmetal1/TM1.a.h4.gds.gz", "TOP", vec!["TM1.a"; 20], vec!["TM1.c", "TM1.d"])]
// Fifty narrow bars, flat and as a GdsArrayRef.
#[case::tm1_a_h5("topmetal1/TM1.a.h5.gds.gz", "TOP", vec!["TM1.a"; 100], vec!["TM1.c", "TM1.d"])]
#[case::tm1_a_h6("topmetal1/TM1.a.h6.gds.gz", "TOP", vec!["TM1.a"; 100], vec!["TM1.c", "TM1.d"])]
// A comb with three narrow teeth; a U at the width is clean.
#[case::tm1_a_h7("topmetal1/TM1.a.h7.gds.gz", "TOP", vec!["TM1.a"; 6], vec!["TM1.c", "TM1.d"])]
// TM1.b: a gap one step under 1.64 in x and y, a corner-to-corner pair under it and a
// corner-on pair; the gaps at 1.64 (straight, diagonal, corner-on) are clean.
#[case::tm1_b_h1("topmetal1/TM1.b.h1.gds.gz", "TOP", vec!["TM1.b"; 4], vec!["TM1.c", "TM1.d"])]
// 45°: a diamond tip to a wall, two 45° strips, a chamfer to a corner, tip to tip, each
// one step under; the controls at the value are clean.
#[case::tm1_b_h2("topmetal1/TM1.b.h2.gds.gz", "TOP", vec!["TM1.b"; 4], vec!["TM1.c", "TM1.d"])]
// "Space or notch": a U notch, a straight-vs-45° notch, a comb (two slots), a slot, a
// keyhole hole, two facing Ls and an island in a ring, all one step under.
#[case::tm1_b_h3("topmetal1/TM1.b.h3.gds.gz", "TOP", vec!["TM1.b"; 8], vec!["TM1.c", "TM1.d"])]
// Two unions and a gridded plate one step under: once each; overlapping boxes are one shape.
#[case::tm1_b_h4("topmetal1/TM1.b.h4.gds.gz", "TOP", vec!["TM1.b"; 2], vec!["TM1.c", "TM1.d"])]
// Eleven gaps on, across and straddling x = 20/21/40/42 and y = 20/21, two corner-on.
#[case::tm1_b_h5("topmetal1/TM1.b.h5.gds.gz", "TOP", vec!["TM1.b"; 11], vec!["TM1.c", "TM1.d"])]
// Fifty pairs, flat and as a GdsArrayRef.
#[case::tm1_b_h6("topmetal1/TM1.b.h6.gds.gz", "TOP", vec!["TM1.b"; 50], vec!["TM1.c", "TM1.d"])]
#[case::tm1_b_h7("topmetal1/TM1.b.h7.gds.gz", "TOP", vec!["TM1.b"; 50], vec!["TM1.c", "TM1.d"])]
// A 0.005 sliver one step from a plate (its width is TM1.a's), 300 µm bars, (1000, 1000).
#[case::tm1_b_h8("topmetal1/TM1.b.h8.gds.gz", "TOP", vec!["TM1.b"; 3], vec!["TM1.c", "TM1.d", "TM1.a"])]
// The rule names no net: a pair joined through a via and the metal below fires like the
// bare pair beside it.
#[case::tm1_b_h9("topmetal1/TM1.b.h9.gds.gz", "TOP", vec!["TM1.b"; 2], vec!["TM1.c", "TM1.d"])]
// TM1.c/TM1.d: 30 % stripes on the metal, its filler and its mask alike are 30 %, not 90 %
// (the fillers' 1000 µm stripes are TM1Fil.a1's).
#[case::tm1_c_h1("topmetal1/TM1.c.h1.gds.gz", "TOP", vec![], vec!["TM1Fil.a1"])]
// 25.000 % inside the boundary and a plate beside it: the plate outside is not counted
// (report, finding 6).
#[case::tm1_c_h2("topmetal1/TM1.c.h2.gds.gz", "TOP", vec![], vec![])]
// A plate reaching in from below: 25.000 % of it inside is clean, 24.995 % fires (finding 6).
#[case::tm1_c_h3("topmetal1/TM1.c.h3.gds.gz", "TOP", vec![], vec![])]
#[case::tm1_c_h4("topmetal1/TM1.c.h4.gds.gz", "TOP", vec!["TM1.c"], vec![])]
// Stripes placed by a GdsArrayRef: 30 % clean, 24.995 % fires.
#[case::tm1_c_h5("topmetal1/TM1.c.h5.gds.gz", "TOP", vec![], vec![])]
#[case::tm1_c_h6("topmetal1/TM1.c.h6.gds.gz", "TOP", vec!["TM1.c"], vec![])]
// Metal and mask overlapping: their union is 70.000 % (clean) and 70.005 % (fires).
#[case::tm1_d_h1("topmetal1/TM1.d.h1.gds.gz", "TOP", vec![], vec![])]
#[case::tm1_d_h2("topmetal1/TM1.d.h2.gds.gz", "TOP", vec!["TM1.d"], vec![])]
// TM1Fil.a: 4.995 in x and y, a 0.005 sliver, a 4.995 bar at (1000, 1000): two markers each.
#[case::tm1fil_a_h1("topmetal1/TM1Fil.a.h1.gds.gz", "TOP", vec!["TM1Fil.a"; 8], vec!["TM1.c", "TM1.d"])]
// A 4.999 diamond (4) and strip (2) fire; 5.006 are clean.  6 × 6 boxes with a corner
// chamfered by 2, 3.5 and 4 are 6 − c wide between the chamfer and the opposite walls
// (two markers each, report note A); chamfered by 1 (5.0) clean.
#[case::tm1fil_a_h2("topmetal1/TM1Fil.a.h2.gds.gz", "TOP", vec!["TM1Fil.a"; 12], vec!["TM1.c", "TM1.d"])]
// Unions 4.995 wide (overlap, slices, one ring wall, an island) fire once each; the rings'
// spans are TM1Fil.a1's.
#[case::tm1fil_a_h3("topmetal1/TM1Fil.a.h3.gds.gz", "TOP", vec!["TM1Fil.a"; 8], vec!["TM1.c", "TM1.d", "TM1Fil.a1"])]
// Ten 4.995 bars on, across and straddling x = 20/21/40/42 plus an L cornered on 20.
#[case::tm1fil_a_h4("topmetal1/TM1Fil.a.h4.gds.gz", "TOP", vec!["TM1Fil.a"; 20], vec!["TM1.c", "TM1.d"])]
// Fifty 4.995 fillers, flat and as a GdsArrayRef.
#[case::tm1fil_a_h5("topmetal1/TM1Fil.a.h5.gds.gz", "TOP", vec!["TM1Fil.a"; 100], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a_h6("topmetal1/TM1Fil.a.h6.gds.gz", "TOP", vec!["TM1Fil.a"; 100], vec!["TM1.c", "TM1.d"])]
// A comb with three 4.995 teeth; a U with 5.0 arms is clean; both span over 10 (TM1Fil.a1's).
#[case::tm1fil_a_h7("topmetal1/TM1Fil.a.h7.gds.gz", "TOP", vec!["TM1Fil.a"; 6], vec!["TM1.c", "TM1.d", "TM1Fil.a1"])]
// TM1Fil.a1 is the filler's long side (figure 5.23): 10.005 × 10, 10 × 10.005, 6 × 10.005, a
// 6 × 300 bar and a bar at (1000, 1000) fire, one marker each (the bounding box's long
// side, KLayout's `with_bbox_max`); 10 × 10 is clean.
#[case::tm1fil_a1_h1("topmetal1/TM1Fil.a1.h1.gds.gz", "TOP", vec!["TM1Fil.a1"; 5], vec!["TM1.c", "TM1.d"])]
// A diamond spanning 10.01 and a 45° strip spanning 10.245 fire, one shape each (the
// count is the tool's cut; report, finding 2); 10.0 and 8.245 spans are clean.
#[case::tm1fil_a1_h2("topmetal1/TM1Fil.a1.h2.gds.gz", "TOP", vec!["TM1Fil.a1"; 2], vec!["TM1.c", "TM1.d"])]
// A 6 × 10.005 union, an abutting 10.005, a gridded 10.005 bar, an L spanning 12 and a
// plus spanning 14 fire; the 6 × 10 union is clean.
#[case::tm1fil_a1_h3("topmetal1/TM1Fil.a1.h3.gds.gz", "TOP", vec!["TM1Fil.a1"; 5], vec!["TM1.c", "TM1.d"])]
// Ten 10.005 fillers on, across and straddling x = 20/21/40/42 and y = 20/21, an L cornered on 20.
#[case::tm1fil_a1_h4("topmetal1/TM1Fil.a1.h4.gds.gz", "TOP", vec!["TM1Fil.a1"; 10], vec!["TM1.c", "TM1.d"])]
// Fifty 10.005 fillers, flat and as a GdsArrayRef.
#[case::tm1fil_a1_h5("topmetal1/TM1Fil.a1.h5.gds.gz", "TOP", vec!["TM1Fil.a1"; 50], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_a1_h6("topmetal1/TM1Fil.a1.h6.gds.gz", "TOP", vec!["TM1Fil.a1"; 50], vec!["TM1.c", "TM1.d"])]
// A 0.005 × 10.005 sliver: TM1Fil.a1 on its length, TM1Fil.a on its width.
#[case::tm1fil_a1_h7("topmetal1/TM1Fil.a1.h7.gds.gz", "TOP", vec!["TM1Fil.a1"], vec!["TM1.c", "TM1.d", "TM1Fil.a"])]
// TM1Fil.b: 2.995 in x and y, a 2.998 diagonal, a corner-on 2.995; 3.0 and 3.005 are clean.
#[case::tm1fil_b_h1("topmetal1/TM1Fil.b.h1.gds.gz", "TOP", vec!["TM1Fil.b"; 4], vec!["TM1.c", "TM1.d"])]
// 45°: a tip to a wall, two 45° strips, a chamfer to a corner, tip to tip, all 2.995;
// the chamfered fillers are under TM1Fil.a themselves (note A), set aside here.
#[case::tm1fil_b_h2("topmetal1/TM1Fil.b.h2.gds.gz", "TOP", vec!["TM1Fil.b"; 4], vec!["TM1.c", "TM1.d", "TM1Fil.a"])]
// TM1Fil.b is "space", not "space or notch": a U with a 2.995 notch draws nothing; facing Ls
// and an island in a ring at 2.995 fire.  All three span over 10 (TM1Fil.a1's).
#[case::tm1fil_b_h3("topmetal1/TM1Fil.b.h3.gds.gz", "TOP", vec!["TM1Fil.b"; 2], vec!["TM1.c", "TM1.d", "TM1Fil.a1"])]
// Two unions and a gridded plate 2.995 apart: once each.
#[case::tm1fil_b_h4("topmetal1/TM1Fil.b.h4.gds.gz", "TOP", vec!["TM1Fil.b"; 2], vec!["TM1.c", "TM1.d"])]
// Ten 2.995 gaps on, across and straddling x = 20/21/40/42 and y = 20/21, one corner-on.
#[case::tm1fil_b_h5("topmetal1/TM1Fil.b.h5.gds.gz", "TOP", vec!["TM1Fil.b"; 10], vec!["TM1.c", "TM1.d"])]
// Fifty pairs 2.995 apart, flat and as a GdsArrayRef.
#[case::tm1fil_b_h6("topmetal1/TM1Fil.b.h6.gds.gz", "TOP", vec!["TM1Fil.b"; 50], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_b_h7("topmetal1/TM1Fil.b.h7.gds.gz", "TOP", vec!["TM1Fil.b"; 50], vec!["TM1.c", "TM1.d"])]
// A 0.005 sliver 2.995 from a filler (its width is TM1Fil.a's), 6 × 10 fillers 2.995 apart,
// a pair at (1000, 1000).
#[case::tm1fil_b_h8("topmetal1/TM1Fil.b.h8.gds.gz", "TOP", vec!["TM1Fil.b"; 3], vec!["TM1.c", "TM1.d", "TM1Fil.a"])]
// TM1Fil.c: 2.995 with the metal right, left and above, a 2.998 diagonal, a corner-on; 3.0
// and 3.005 are clean, and so is a TM1.mask shape 2.995 away (the rule names the drawing layer).
#[case::tm1fil_c_h1("topmetal1/TM1Fil.c.h1.gds.gz", "TOP", vec!["TM1Fil.c"; 5], vec!["TM1.c", "TM1.d"])]
// 45°: a metal tip to a filler wall, a metal strip to a filler strip, a metal chamfer to a
// filler corner, a filler chamfer to a metal corner, all 2.995; the chamfered fillers'
// own TM1Fil.a (note A) set aside.
#[case::tm1fil_c_h2("topmetal1/TM1Fil.c.h2.gds.gz", "TOP", vec!["TM1Fil.c"; 4], vec!["TM1.c", "TM1.d", "TM1Fil.a"])]
// A filler abutting a metal is a space of zero (report, finding 3); a filler across a
// metal's edge and one inside a plate share area and are no pair.
#[case::tm1fil_c_h3("topmetal1/TM1Fil.c.h3.gds.gz", "TOP", vec!["TM1Fil.c"; 1], vec!["TM1.c", "TM1.d"])]
// Nine 2.995 filler/metal gaps on, across and straddling x = 20/21/40/42 and y = 20, one corner-on.
#[case::tm1fil_c_h4("topmetal1/TM1Fil.c.h4.gds.gz", "TOP", vec!["TM1Fil.c"; 9], vec!["TM1.c", "TM1.d"])]
// Fifty filler/metal pairs 2.995 apart, flat and as a GdsArrayRef.
#[case::tm1fil_c_h5("topmetal1/TM1Fil.c.h5.gds.gz", "TOP", vec!["TM1Fil.c"; 50], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_c_h6("topmetal1/TM1Fil.c.h6.gds.gz", "TOP", vec!["TM1Fil.c"; 50], vec!["TM1.c", "TM1.d"])]
// A 300 µm metal bar 2.995 from a filler, a 0.005 metal sliver 2.995 away (TM1.a's width),
// a pair at (1000, 1000).
#[case::tm1fil_c_h7("topmetal1/TM1Fil.c.h7.gds.gz", "TOP", vec!["TM1Fil.c"; 3], vec!["TM1.c", "TM1.d", "TM1.a"])]
// TM1Fil.d: 4.895 in x and y, a 4.893 diagonal, a corner-on 4.895; 4.9 and 4.900 are clean.
#[case::tm1fil_d_h1("topmetal1/TM1Fil.d.h1.gds.gz", "TOP", vec!["TM1Fil.d"; 4], vec!["TM1.c", "TM1.d"])]
// 45°: a TRANS tip to a filler wall at 4.895, a TRANS chamfer to a filler corner, a filler
// chamfer to a TRANS corner and TRANS/filler strips at 4.893; the chamfered filler's
// own TM1Fil.a (note A) set aside.
#[case::tm1fil_d_h2("topmetal1/TM1Fil.d.h2.gds.gz", "TOP", vec!["TM1Fil.d"; 4], vec!["TM1.c", "TM1.d", "TM1Fil.a"])]
// A filler inside a TRANS 2.0 from its edge fires (report, finding 4); one 7 µm inside and
// one across the edge do not.
#[case::tm1fil_d_h3("topmetal1/TM1Fil.d.h3.gds.gz", "TOP", vec!["TM1Fil.d"; 1], vec!["TM1.c", "TM1.d"])]
// Nine 4.895 filler/TRANS gaps on, across and straddling x = 20/21/40/42 and y = 20, one corner-on.
#[case::tm1fil_d_h4("topmetal1/TM1Fil.d.h4.gds.gz", "TOP", vec!["TM1Fil.d"; 9], vec!["TM1.c", "TM1.d"])]
// Fifty filler/TRANS pairs 4.895 apart, flat and as a GdsArrayRef.
#[case::tm1fil_d_h5("topmetal1/TM1Fil.d.h5.gds.gz", "TOP", vec!["TM1Fil.d"; 50], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_d_h6("topmetal1/TM1Fil.d.h6.gds.gz", "TOP", vec!["TM1Fil.d"; 50], vec!["TM1.c", "TM1.d"])]
// A 300 µm TRANS bar 4.895 from a filler, a pair at (1000, 1000).
#[case::tm1fil_d_h7("topmetal1/TM1Fil.d.h7.gds.gz", "TOP", vec!["TM1Fil.d"; 2], vec!["TM1.c", "TM1.d"])]
fn test_topmetal1(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TM1, gds, topcell, &ignore), expected);
}

// --- TopMetal2 ---

const DECK_TM2: &str = "topmetal2";

#[rstest]
#[case::tm2_a("topmetal2/TM2.a.gds.gz", "TOP", vec!["TM2.a", "TM2.a", "TM2.a", "TM2.a"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_b_space("topmetal2/TM2.b.space.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_b_notch("topmetal2/TM2.b.notch.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.a", "TM2.c", "TM2.d"])]
#[case::tm2_b_mixed_notch("topmetal2/TM2.b.mixed_notch.gds.gz", "TOP", vec!["TM2.b", "TM2.b"], vec!["TM2.a", "TM2.c", "TM2.d"])]
#[case::tm2_c_ok("topmetal2/TM2.c.gds.gz", "TOP", vec![], vec!["TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_c_fail("topmetal2/TM2.c.fail.gds.gz", "TOP", vec!["TM2.c"], vec!["TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_d_ok("topmetal2/TM2.d.gds.gz", "TOP", vec![], vec!["TM2.b", "TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2_d_fail("topmetal2/TM2.d.fail.gds.gz", "TOP", vec!["TM2.d"], vec!["TM2.b", "TM2Fil.a", "TM2Fil.a1"])]
#[case::tm2fil_c("topmetal2/TM2Fil.c.gds.gz", "TOP", vec!["TM2Fil.c", "TM2Fil.c"], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a("topmetal2/TM2Fil.a.gds.gz", "TOP", vec!["TM2Fil.a"; 4], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a1("topmetal2/TM2Fil.a1.gds.gz", "TOP", vec!["TM2Fil.a1"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_b("topmetal2/TM2Fil.b.gds.gz", "TOP", vec!["TM2Fil.b"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_d("topmetal2/TM2Fil.d.gds.gz", "TOP", vec!["TM2Fil.d"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_fail("topmetal2/TM2.bR.fail.gds.gz", "TOP", vec!["TM2.bR"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ok("topmetal2/TM2.bR.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ind("topmetal2/TM2.bR.ind.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
// Hardening (ci/hardening/reports/ihp-sg13g2/topmetal.md).  TM2.a: 1.995 bars in x and y, a
// 300 µm bar, a 0.005 sliver, a bar at (1000, 1000): two markers each; 2.00 is clean.
#[case::tm2_a_h1("topmetal2/TM2.a.h1.gds.gz", "TOP", vec!["TM2.a"; 10], vec!["TM2.c", "TM2.d"])]
// A diamond (4) and a 45° strip (2) one grid step under the width; the on-grid step
// above, a chamfered box and an L with a chamfered inner corner are clean.
#[case::tm2_a_h2("topmetal2/TM2.a.h2.gds.gz", "TOP", vec!["TM2.a"; 6], vec!["TM2.c", "TM2.d"])]
// Unions one step narrow (overlap, slices, one ring side, an island) fire once each; the
// unions at the width and a gridded bar are clean.
#[case::tm2_a_h3("topmetal2/TM2.a.h3.gds.gz", "TOP", vec!["TM2.a"; 8], vec!["TM2.c", "TM2.d"])]
// Ten narrow bars on, across and straddling x = 20/21/40/42 plus an L cornered on 20.
#[case::tm2_a_h4("topmetal2/TM2.a.h4.gds.gz", "TOP", vec!["TM2.a"; 20], vec!["TM2.c", "TM2.d"])]
// Fifty narrow bars, flat and as a GdsArrayRef.
#[case::tm2_a_h5("topmetal2/TM2.a.h5.gds.gz", "TOP", vec!["TM2.a"; 100], vec!["TM2.c", "TM2.d"])]
#[case::tm2_a_h6("topmetal2/TM2.a.h6.gds.gz", "TOP", vec!["TM2.a"; 100], vec!["TM2.c", "TM2.d"])]
// A comb with three narrow teeth; a U at the width is clean.
#[case::tm2_a_h7("topmetal2/TM2.a.h7.gds.gz", "TOP", vec!["TM2.a"; 6], vec!["TM2.c", "TM2.d"])]
// TM2.b: a gap one step under 2.00 in x and y, a corner-to-corner pair under it and a
// corner-on pair; the gaps at 2.00 (straight, diagonal, corner-on) are clean.
#[case::tm2_b_h1("topmetal2/TM2.b.h1.gds.gz", "TOP", vec!["TM2.b"; 4], vec!["TM2.c", "TM2.d"])]
// 45°: a diamond tip to a wall, two 45° strips, a chamfer to a corner, tip to tip, each
// one step under; the controls at the value are clean.
#[case::tm2_b_h2("topmetal2/TM2.b.h2.gds.gz", "TOP", vec!["TM2.b"; 4], vec!["TM2.c", "TM2.d"])]
// "Space or notch": a U notch, a straight-vs-45° notch, a comb (two slots), a slot, a
// keyhole hole, two facing Ls and an island in a ring, all one step under.
#[case::tm2_b_h3("topmetal2/TM2.b.h3.gds.gz", "TOP", vec!["TM2.b"; 8], vec!["TM2.c", "TM2.d"])]
// Two unions and a gridded plate one step under: once each; overlapping boxes are one shape.
#[case::tm2_b_h4("topmetal2/TM2.b.h4.gds.gz", "TOP", vec!["TM2.b"; 2], vec!["TM2.c", "TM2.d"])]
// Eleven gaps on, across and straddling x = 20/21/40/42 and y = 20/21, two corner-on.
#[case::tm2_b_h5("topmetal2/TM2.b.h5.gds.gz", "TOP", vec!["TM2.b"; 11], vec!["TM2.c", "TM2.d"])]
// Fifty pairs, flat and as a GdsArrayRef.
#[case::tm2_b_h6("topmetal2/TM2.b.h6.gds.gz", "TOP", vec!["TM2.b"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2_b_h7("topmetal2/TM2.b.h7.gds.gz", "TOP", vec!["TM2.b"; 50], vec!["TM2.c", "TM2.d"])]
// A 0.005 sliver one step from a plate (its width is TM2.a's), 300 µm bars, (1000, 1000).
#[case::tm2_b_h8("topmetal2/TM2.b.h8.gds.gz", "TOP", vec!["TM2.b"; 3], vec!["TM2.c", "TM2.d", "TM2.a"])]
// The rule names no net: a pair joined through a via and the metal below fires like the
// bare pair beside it.
#[case::tm2_b_h9("topmetal2/TM2.b.h9.gds.gz", "TOP", vec!["TM2.b"; 2], vec!["TM2.c", "TM2.d"])]
// TM2.c/TM2.d: 30 % stripes on the metal, its filler and its mask alike are 30 %, not 90 %
// (the fillers' 1000 µm stripes are TM2Fil.a1's).
#[case::tm2_c_h1("topmetal2/TM2.c.h1.gds.gz", "TOP", vec![], vec!["TM2Fil.a1"])]
// 25.000 % inside the boundary and a plate beside it: the plate outside is not counted
// (report, finding 6).
#[case::tm2_c_h2("topmetal2/TM2.c.h2.gds.gz", "TOP", vec![], vec![])]
// A plate reaching in from below: 25.000 % of it inside is clean, 24.995 % fires (finding 6).
#[case::tm2_c_h3("topmetal2/TM2.c.h3.gds.gz", "TOP", vec![], vec![])]
#[case::tm2_c_h4("topmetal2/TM2.c.h4.gds.gz", "TOP", vec!["TM2.c"], vec![])]
// Stripes placed by a GdsArrayRef: 30 % clean, 24.995 % fires.
#[case::tm2_c_h5("topmetal2/TM2.c.h5.gds.gz", "TOP", vec![], vec![])]
#[case::tm2_c_h6("topmetal2/TM2.c.h6.gds.gz", "TOP", vec!["TM2.c"], vec![])]
// Metal and mask overlapping: their union is 70.000 % (clean) and 70.005 % (fires).
#[case::tm2_d_h1("topmetal2/TM2.d.h1.gds.gz", "TOP", vec![], vec![])]
#[case::tm2_d_h2("topmetal2/TM2.d.h2.gds.gz", "TOP", vec!["TM2.d"], vec![])]
// TM2Fil.a: 4.995 in x and y, a 0.005 sliver, a 4.995 bar at (1000, 1000): two markers each.
#[case::tm2fil_a_h1("topmetal2/TM2Fil.a.h1.gds.gz", "TOP", vec!["TM2Fil.a"; 8], vec!["TM2.c", "TM2.d"])]
// A 4.999 diamond (4) and strip (2) fire; 5.006 are clean.  6 × 6 boxes with a corner
// chamfered by 2, 3.5 and 4 are 6 − c wide between the chamfer and the opposite walls
// (two markers each, report note A); chamfered by 1 (5.0) clean.
#[case::tm2fil_a_h2("topmetal2/TM2Fil.a.h2.gds.gz", "TOP", vec!["TM2Fil.a"; 12], vec!["TM2.c", "TM2.d"])]
// Unions 4.995 wide (overlap, slices, one ring wall, an island) fire once each; the rings'
// spans are TM2Fil.a1's.
#[case::tm2fil_a_h3("topmetal2/TM2Fil.a.h3.gds.gz", "TOP", vec!["TM2Fil.a"; 8], vec!["TM2.c", "TM2.d", "TM2Fil.a1"])]
// Ten 4.995 bars on, across and straddling x = 20/21/40/42 plus an L cornered on 20.
#[case::tm2fil_a_h4("topmetal2/TM2Fil.a.h4.gds.gz", "TOP", vec!["TM2Fil.a"; 20], vec!["TM2.c", "TM2.d"])]
// Fifty 4.995 fillers, flat and as a GdsArrayRef.
#[case::tm2fil_a_h5("topmetal2/TM2Fil.a.h5.gds.gz", "TOP", vec!["TM2Fil.a"; 100], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a_h6("topmetal2/TM2Fil.a.h6.gds.gz", "TOP", vec!["TM2Fil.a"; 100], vec!["TM2.c", "TM2.d"])]
// A comb with three 4.995 teeth; a U with 5.0 arms is clean; both span over 10 (TM2Fil.a1's).
#[case::tm2fil_a_h7("topmetal2/TM2Fil.a.h7.gds.gz", "TOP", vec!["TM2Fil.a"; 6], vec!["TM2.c", "TM2.d", "TM2Fil.a1"])]
// TM2Fil.a1 is the filler's long side (figure 5.23): 10.005 × 10, 10 × 10.005, 6 × 10.005, a
// 6 × 300 bar and a bar at (1000, 1000) fire, one marker each (the bounding box's long
// side, KLayout's `with_bbox_max`); 10 × 10 is clean.
#[case::tm2fil_a1_h1("topmetal2/TM2Fil.a1.h1.gds.gz", "TOP", vec!["TM2Fil.a1"; 5], vec!["TM2.c", "TM2.d"])]
// A diamond spanning 10.01 and a 45° strip spanning 10.245 fire, one shape each (the
// count is the tool's cut; report, finding 2); 10.0 and 8.245 spans are clean.
#[case::tm2fil_a1_h2("topmetal2/TM2Fil.a1.h2.gds.gz", "TOP", vec!["TM2Fil.a1"; 2], vec!["TM2.c", "TM2.d"])]
// A 6 × 10.005 union, an abutting 10.005, a gridded 10.005 bar, an L spanning 12 and a
// plus spanning 14 fire; the 6 × 10 union is clean.
#[case::tm2fil_a1_h3("topmetal2/TM2Fil.a1.h3.gds.gz", "TOP", vec!["TM2Fil.a1"; 5], vec!["TM2.c", "TM2.d"])]
// Ten 10.005 fillers on, across and straddling x = 20/21/40/42 and y = 20/21, an L cornered on 20.
#[case::tm2fil_a1_h4("topmetal2/TM2Fil.a1.h4.gds.gz", "TOP", vec!["TM2Fil.a1"; 10], vec!["TM2.c", "TM2.d"])]
// Fifty 10.005 fillers, flat and as a GdsArrayRef.
#[case::tm2fil_a1_h5("topmetal2/TM2Fil.a1.h5.gds.gz", "TOP", vec!["TM2Fil.a1"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_a1_h6("topmetal2/TM2Fil.a1.h6.gds.gz", "TOP", vec!["TM2Fil.a1"; 50], vec!["TM2.c", "TM2.d"])]
// A 0.005 × 10.005 sliver: TM2Fil.a1 on its length, TM2Fil.a on its width.
#[case::tm2fil_a1_h7("topmetal2/TM2Fil.a1.h7.gds.gz", "TOP", vec!["TM2Fil.a1"], vec!["TM2.c", "TM2.d", "TM2Fil.a"])]
// TM2Fil.b: 2.995 in x and y, a 2.998 diagonal, a corner-on 2.995; 3.0 and 3.005 are clean.
#[case::tm2fil_b_h1("topmetal2/TM2Fil.b.h1.gds.gz", "TOP", vec!["TM2Fil.b"; 4], vec!["TM2.c", "TM2.d"])]
// 45°: a tip to a wall, two 45° strips, a chamfer to a corner, tip to tip, all 2.995;
// the chamfered fillers are under TM2Fil.a themselves (note A), set aside here.
#[case::tm2fil_b_h2("topmetal2/TM2Fil.b.h2.gds.gz", "TOP", vec!["TM2Fil.b"; 4], vec!["TM2.c", "TM2.d", "TM2Fil.a"])]
// TM2Fil.b is "space", not "space or notch": a U with a 2.995 notch draws nothing; facing Ls
// and an island in a ring at 2.995 fire.  All three span over 10 (TM2Fil.a1's).
#[case::tm2fil_b_h3("topmetal2/TM2Fil.b.h3.gds.gz", "TOP", vec!["TM2Fil.b"; 2], vec!["TM2.c", "TM2.d", "TM2Fil.a1"])]
// Two unions and a gridded plate 2.995 apart: once each.
#[case::tm2fil_b_h4("topmetal2/TM2Fil.b.h4.gds.gz", "TOP", vec!["TM2Fil.b"; 2], vec!["TM2.c", "TM2.d"])]
// Ten 2.995 gaps on, across and straddling x = 20/21/40/42 and y = 20/21, one corner-on.
#[case::tm2fil_b_h5("topmetal2/TM2Fil.b.h5.gds.gz", "TOP", vec!["TM2Fil.b"; 10], vec!["TM2.c", "TM2.d"])]
// Fifty pairs 2.995 apart, flat and as a GdsArrayRef.
#[case::tm2fil_b_h6("topmetal2/TM2Fil.b.h6.gds.gz", "TOP", vec!["TM2Fil.b"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_b_h7("topmetal2/TM2Fil.b.h7.gds.gz", "TOP", vec!["TM2Fil.b"; 50], vec!["TM2.c", "TM2.d"])]
// A 0.005 sliver 2.995 from a filler (its width is TM2Fil.a's), 6 × 10 fillers 2.995 apart,
// a pair at (1000, 1000).
#[case::tm2fil_b_h8("topmetal2/TM2Fil.b.h8.gds.gz", "TOP", vec!["TM2Fil.b"; 3], vec!["TM2.c", "TM2.d", "TM2Fil.a"])]
// TM2Fil.c: 2.995 with the metal right, left and above, a 2.998 diagonal, a corner-on; 3.0
// and 3.005 are clean, and so is a TM2.mask shape 2.995 away (the rule names the drawing layer).
#[case::tm2fil_c_h1("topmetal2/TM2Fil.c.h1.gds.gz", "TOP", vec!["TM2Fil.c"; 5], vec!["TM2.c", "TM2.d"])]
// 45°: a metal tip to a filler wall, a metal strip to a filler strip, a metal chamfer to a
// filler corner, a filler chamfer to a metal corner, all 2.995; the chamfered fillers'
// own TM2Fil.a (note A) set aside.
#[case::tm2fil_c_h2("topmetal2/TM2Fil.c.h2.gds.gz", "TOP", vec!["TM2Fil.c"; 4], vec!["TM2.c", "TM2.d", "TM2Fil.a"])]
// A filler abutting a metal is a space of zero (report, finding 3); a filler across a
// metal's edge and one inside a plate share area and are no pair.
#[case::tm2fil_c_h3("topmetal2/TM2Fil.c.h3.gds.gz", "TOP", vec!["TM2Fil.c"; 1], vec!["TM2.c", "TM2.d"])]
// Nine 2.995 filler/metal gaps on, across and straddling x = 20/21/40/42 and y = 20, one corner-on.
#[case::tm2fil_c_h4("topmetal2/TM2Fil.c.h4.gds.gz", "TOP", vec!["TM2Fil.c"; 9], vec!["TM2.c", "TM2.d"])]
// Fifty filler/metal pairs 2.995 apart, flat and as a GdsArrayRef.
#[case::tm2fil_c_h5("topmetal2/TM2Fil.c.h5.gds.gz", "TOP", vec!["TM2Fil.c"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_c_h6("topmetal2/TM2Fil.c.h6.gds.gz", "TOP", vec!["TM2Fil.c"; 50], vec!["TM2.c", "TM2.d"])]
// A 300 µm metal bar 2.995 from a filler, a 0.005 metal sliver 2.995 away (TM2.a's width),
// a pair at (1000, 1000).
#[case::tm2fil_c_h7("topmetal2/TM2Fil.c.h7.gds.gz", "TOP", vec!["TM2Fil.c"; 3], vec!["TM2.c", "TM2.d", "TM2.a"])]
// TM2Fil.d: 4.895 in x and y, a 4.893 diagonal, a corner-on 4.895; 4.9 and 4.900 are clean.
#[case::tm2fil_d_h1("topmetal2/TM2Fil.d.h1.gds.gz", "TOP", vec!["TM2Fil.d"; 4], vec!["TM2.c", "TM2.d"])]
// 45°: a TRANS tip to a filler wall at 4.895, a TRANS chamfer to a filler corner, a filler
// chamfer to a TRANS corner and TRANS/filler strips at 4.893; the chamfered filler's
// own TM2Fil.a (note A) set aside.
#[case::tm2fil_d_h2("topmetal2/TM2Fil.d.h2.gds.gz", "TOP", vec!["TM2Fil.d"; 4], vec!["TM2.c", "TM2.d", "TM2Fil.a"])]
// A filler inside a TRANS 2.0 from its edge fires (report, finding 4); one 7 µm inside and
// one across the edge do not.
#[case::tm2fil_d_h3("topmetal2/TM2Fil.d.h3.gds.gz", "TOP", vec!["TM2Fil.d"; 1], vec!["TM2.c", "TM2.d"])]
// Nine 4.895 filler/TRANS gaps on, across and straddling x = 20/21/40/42 and y = 20, one corner-on.
#[case::tm2fil_d_h4("topmetal2/TM2Fil.d.h4.gds.gz", "TOP", vec!["TM2Fil.d"; 9], vec!["TM2.c", "TM2.d"])]
// Fifty filler/TRANS pairs 4.895 apart, flat and as a GdsArrayRef.
#[case::tm2fil_d_h5("topmetal2/TM2Fil.d.h5.gds.gz", "TOP", vec!["TM2Fil.d"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_d_h6("topmetal2/TM2Fil.d.h6.gds.gz", "TOP", vec!["TM2Fil.d"; 50], vec!["TM2.c", "TM2.d"])]
// A 300 µm TRANS bar 4.895 from a filler, a pair at (1000, 1000).
#[case::tm2fil_d_h7("topmetal2/TM2Fil.d.h7.gds.gz", "TOP", vec!["TM2Fil.d"; 2], vec!["TM2.c", "TM2.d"])]
// TM2.bR: a 5.005 line beside a 2 line at gap 4 over 60, two 6 lines at 4.995 over 60, two
// 6 lines at 4 over 50.005 fire; a 6/2 pair over 50.0 and a 5.0/2 pair over 60 are clean.
#[case::tm2_br_h1("topmetal2/TM2.bR.h1.gds.gz", "TOP", vec!["TM2.bR"; 3], vec!["TM2.c", "TM2.d"])]
// The run is the overlap: 50.005 fires, 50.0 and a 20 µm neighbour do not.
#[case::tm2_br_h2("topmetal2/TM2.bR.h2.gds.gz", "TOP", vec!["TM2.bR"; 1], vec!["TM2.c", "TM2.d"])]
// Beside a 6 line 60 long: a neighbour whose facing wall steps 4/4.5 at mid-height, one
// with a 0.005 nick (a TM2.b notch of 0.5 as well), one drawn as two abutting 30 µm boxes,
// a 6 line drawn as two 3 strips beside a 2 line (report, finding 5).  The two-box
// neighbour and the two-strip line fire; the run is read per facing wall pair, as
// KLayout's projection, so the step and the nick each cut it to 30 (OPEN: the manual
// speaks of lines).
#[case::tm2_br_h3("topmetal2/TM2.bR.h3.gds.gz", "TOP", vec!["TM2.bR", "TM2.bR", "TM2.b"], vec!["TM2.c", "TM2.d"])]
// Two 6-wide 45° strips 60 side by side at 3.999 and the figure's plate beside a 2 line at
// 4 fire; 5.003, a T (run 2) and a strip crossing at 45° are clean.
#[case::tm2_br_h4("topmetal2/TM2.bR.h4.gds.gz", "TOP", vec!["TM2.bR"; 2], vec!["TM2.c", "TM2.d"])]
// IND over 5 µm of a 60 run leaves 55 (fires); over 20 it leaves 40 (clean, report, finding
// 8); a wide line within IND beside a narrow one outside is not checked.
#[case::tm2_br_h5("topmetal2/TM2.bR.h5.gds.gz", "TOP", vec!["TM2.bR"; 1], vec!["TM2.c", "TM2.d"])]
// Two 6 lines 1.995 apart over 60: TM2.b and TM2.bR both.
#[case::tm2_br_h6("topmetal2/TM2.bR.h6.gds.gz", "TOP", vec!["TM2.b", "TM2.bR"], vec!["TM2.c", "TM2.d"])]
// Fifty pairs of 6 lines at 4 over 60, flat and as a GdsArrayRef.
#[case::tm2_br_h7("topmetal2/TM2.bR.h7.gds.gz", "TOP", vec!["TM2.bR"; 50], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_h8("topmetal2/TM2.bR.h8.gds.gz", "TOP", vec!["TM2.bR"; 50], vec!["TM2.c", "TM2.d"])]
// Five 50.005 runs starting at x = 0, 20, 19.995, 7 and 40; a 50.0 run from 20 is clean.
#[case::tm2_br_h9("topmetal2/TM2.bR.h9.gds.gz", "TOP", vec!["TM2.bR"; 5], vec!["TM2.c", "TM2.d"])]
// A 4 line 6 wide over 55 of its 60 beside a 4 line at gap 4 fires (report, finding 9);
// 6 wide over 20 (far or near side) does not; a 300 µm pair and a pair at (1000, 1000).
#[case::tm2_br_h10("topmetal2/TM2.bR.h10.gds.gz", "TOP", vec!["TM2.bR"; 3], vec!["TM2.c", "TM2.d"])]
// h3's stepped, nicked and two-box neighbours again as 2 lines beside 6 lines (the pairing
// KLayout's shielded check sees): the two-box neighbour fires, the step and the nick read
// per wall pair (as h3), plus the nick's TM2.b notch.
#[case::tm2_br_h11("topmetal2/TM2.bR.h11.gds.gz", "TOP", vec!["TM2.bR", "TM2.b"], vec!["TM2.c", "TM2.d"])]
fn test_topmetal2(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_TM2, gds, topcell, &ignore), expected);
}

// --- Passiv ---

const DECK_PAS: &str = "passiv";

#[rstest]
#[case::pas_a("passiv/Pas.a.gds.gz", "TOP", vec!["Pas.a", "Pas.a", "Pas.a", "Pas.a"], vec![])]
#[case::pas_b_space("passiv/Pas.b.space.gds.gz", "TOP", vec!["Pas.b", "Pas.b"], vec![])]
#[case::pas_b_notch("passiv/Pas.b.notch.gds.gz", "TOP", vec!["Pas.b", "Pas.b"], vec![])]
#[case::pas_c("passiv/Pas.c.gds.gz", "TOP", vec!["Pas.c"; 4], vec![])]
fn test_passiv(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PAS, gds, topcell, &ignore), expected);
}

// --- Pin ---

const DECK_PIN: &str = "pin";

#[rstest]
#[case::pin_a("pin/Pin.a.gds.gz", "TOP", vec!["Pin.a"; 4], vec![])]
#[case::pin_b("pin/Pin.b.gds.gz", "TOP", vec!["Pin.b"; 4], vec![])]
#[case::pin_e("pin/Pin.e.gds.gz", "TOP", vec!["Pin.e"; 4], vec![])]
#[case::pin_f_m2("pin/Pin.f.m2.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m3("pin/Pin.f.m3.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m4("pin/Pin.f.m4.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_f_m5("pin/Pin.f.m5.gds.gz", "TOP", vec!["Pin.f"; 4], vec![])]
#[case::pin_g("pin/Pin.g.gds.gz", "TOP", vec!["Pin.g"; 4], vec![])]
#[case::pin_h("pin/Pin.h.gds.gz", "TOP", vec!["Pin.h"; 4], vec![])]
fn test_pin(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PIN, gds, topcell, &ignore), expected);
}

// --- LBE ---

const DECK_LBE: &str = "lbe";

#[rstest]
#[case::lbe_a("lbe/LBE.a.gds.gz", "TOP", vec!["LBE.a"; 4], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_b("lbe/LBE.b.gds.gz", "TOP", vec!["LBE.b"; 2], vec!["LBE.b1", "LBE.i"])]
#[case::lbe_b1("lbe/LBE.b1.gds.gz", "TOP", vec!["LBE.b1"; 2], vec!["LBE.i"])]
#[case::lbe_b1_merge("lbe/LBE.b1.merge.gds.gz", "TOP", vec!["LBE.b1"], vec!["LBE.i"])]
#[case::lbe_b2("lbe/LBE.b2.gds.gz", "TOP", vec!["LBE.b2"], vec!["LBE.i"])]
#[case::lbe_c_space("lbe/LBE.c.space.gds.gz", "TOP", vec!["LBE.c"; 2], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_c_notch("lbe/LBE.c.notch.gds.gz", "TOP", vec!["LBE.c"; 2], vec!["LBE.a", "LBE.i"])]
#[case::lbe_d_space("lbe/LBE.d.space.gds.gz", "TOP", vec!["LBE.d"; 2], vec!["LBE.i", "LBE.b2"])]
#[case::lbe_e("lbe/LBE.e.gds.gz", "TOP", vec!["LBE.e"; 2], vec!["LBE.i"])]
#[case::lbe_f("lbe/LBE.f.gds.gz", "TOP", vec!["LBE.f"], vec!["LBE.i"])]
#[case::lbe_h("lbe/LBE.h.gds.gz", "TOP", vec!["LBE.h"], vec!["LBE.i", "LBE.c"])]
#[case::lbe_h_open("lbe/LBE.h.open.gds.gz", "TOP", vec![], vec!["LBE.i", "LBE.c"])]
#[case::lbe_i("lbe/LBE.i.gds.gz", "TOP", vec![], vec![])]
#[case::lbe_i_fail("lbe/LBE.i.fail.gds.gz", "TOP", vec!["LBE.i"], vec![])]
fn test_lbe(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_LBE, gds, topcell, &ignore), expected);
}

// --- EXTBlock ---

const DECK_EXTB: &str = "extblock";

#[rstest]
#[case::extb_a("extblock/EXTB.a.gds.gz", "TOP", vec!["EXTB.a"; 4], vec![])]
#[case::extb_b_space("extblock/EXTB.b.space.gds.gz", "TOP", vec!["EXTB.b"; 2], vec![])]
#[case::extb_b_notch("extblock/EXTB.b.notch.gds.gz", "TOP", vec!["EXTB.b"; 2], vec![])]
#[case::extb_c("extblock/EXTB.c.gds.gz", "TOP", vec!["EXTB.c"; 2], vec![])]
fn test_extblock(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_EXTB, gds, topcell, &ignore), expected);
}

// --- Pad ---

const DECK_PAD: &str = "pad";

#[rstest]
#[case::pad_a1("pad/Pad.a1.gds.gz", "TOP", vec!["Pad.a1"], vec!["Pad.d", "Padb.a", "Padc.a", "Pad.i"])]
#[case::pad_d("pad/Pad.d.gds.gz", "TOP", vec!["Pad.d"; 1], vec!["Pad.a1", "Padb.a", "Padc.a", "Pad.i"])]
#[case::pad_i("pad/Pad.i.gds.gz", "TOP", vec!["Pad.i"; 1], vec![])]
#[case::padb_a("pad/Padb.a.gds.gz", "TOP", vec!["Padb.a"; 8], vec!["Padb.c", "Padc.a", "Padc.b", "Padc.c", "Pad.i", "Padb.f"])]
#[case::padb_b("pad/Padb.b.gds.gz", "TOP", vec!["Padb.b"; 2], vec!["Padb.c", "Padc.a", "Padc.b", "Padc.c", "Pad.i", "Padb.f"])]
#[case::padb_c("pad/Padb.c.gds.gz", "TOP", vec!["Padb.c"; 4], vec!["Padb.a", "Padc.a", "Padc.b", "Padc.c", "Padb.f"])]
#[case::padb_d("pad/Padb.d.gds.gz", "TOP", vec!["Padb.d"; 1], vec!["Padb.f"])]
// Pad at 25 µm from the seal-Activ fires (1/1 exact match vs KLayout); at exactly 30.0 µm
// clean in both.  The square pads trip Padc.f (circle-only, BEOL rule) — ignored here.
#[case::padc_d("pad/Padc.d.gds.gz", "TOP", vec!["Padc.d"; 1], vec!["Padc.f"])]
#[case::padc_a("pad/Padc.a.gds.gz", "TOP", vec!["Padc.a"; 8], vec!["Padc.c", "Padb.a", "Padb.b", "Padb.c", "Pad.i", "Padc.f"])]
#[case::padc_b("pad/Padc.b.gds.gz", "TOP", vec!["Padc.b"; 2], vec!["Padc.c", "Padb.a", "Padb.b", "Padb.c", "Pad.i", "Padc.f"])]
#[case::padc_c("pad/Padc.c.gds.gz", "TOP", vec!["Padc.c"; 4], vec!["Padc.a", "Padb.a", "Padb.b", "Padb.c", "Padc.f"])]
// A square SBumpPad violates (not circle/octagon); an octagon and a circle both pass.
// Padb.a/b collateral is the pre-existing facing-edge width/space scan applied to the
// non-rectangular octagon/circle shapes — orthogonal to what this case tests.
#[case::padb_f("pad/Padb.f.gds.gz", "TOP", vec!["Padb.f"; 1], vec!["Padb.a", "Padb.b"])]
// A square AND an octagon CuPillarPad both violate (only circle is allowed); a circle passes.
#[case::padc_f("pad/Padc.f.gds.gz", "TOP", vec!["Padc.f"; 2], vec!["Padc.a"])]
fn test_pad(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PAD, gds, topcell, &ignore), expected);
}

// --- pSD (p+ S/D implant) ---

const DECK_PSD: &str = "psd";

#[rstest]
#[case::psd_a("psd/pSD.a.gds.gz", "TOP", vec!["pSD.a"; 2], vec!["pSD.b", "pSD.d", "pSD.k"])]
#[case::psd_b("psd/pSD.b.gds.gz", "TOP", vec!["pSD.b"; 2], vec!["pSD.a", "pSD.d", "pSD.k"])]
#[case::psd_d("psd/pSD.d.gds.gz", "TOP", vec!["pSD.d"; 2], vec!["pSD.a", "pSD.b", "pSD.k"])]
#[case::psd_k("psd/pSD.k.gds.gz", "TOP", vec!["pSD.k"; 1], vec!["pSD.a", "pSD.b", "pSD.l"])]
#[case::psd_l("psd/pSD.l.gds.gz", "TOP", vec!["pSD.l"; 1], vec!["pSD.a", "pSD.b", "pSD.k"])]
// pSD.m/n reuse the resistor-recognition fixtures (pSD too close to / not enclosing a resistor).
#[case::psd_m("resistor/RsilBody.gds.gz", "TOP", vec!["pSD.m"; 1], vec![])]
#[case::psd_n("resistor/RppdBody.gds.gz", "TOP", vec!["pSD.n"; 1], vec![])]
#[case::psd_g("psd/pSD.g.gds.gz", "TOP", vec!["pSD.g"; 2], vec![])]
// Four abutted ties: exact-0.30 sliver (clean), uniform 0.20 (fires), 0.20 with a wide
// pocket (clean — ≥0.30 at one position suffices), 0.2×0.5 tab (fires — width, not
// length, is the metric).  Validated 2/2 exact-location match vs KLayout.
#[case::psd_e("psd/pSD.e.gds.gz", "TOP", vec!["pSD.e"; 2], vec![])]
// Six abutted NWell ties, tab depth is the metric: 0.20/0.295/wide-0.20 fire, 0.30 and
// the 0.2-wide × 0.45-deep tab are clean (all five match KLayout exactly); the L-shaped
// tab fires HERE ONLY (PDF-first — see the deck comment).
#[case::psd_f("psd/pSD.f.gds.gz", "TOP", vec!["pSD.f"; 4], vec![])]
// Margins 0.02 and flush-0 fire, 0.05 and exactly-0.03 clean, the crossing abutted tie
// is clean (protruding part ignored) while a crossing tie with a 0.02 lateral margin
// fires — all validated 1:1 against the FEOL driver (rule is not in the maximal deck).
#[case::psd_c1("psd/pSD.c1.gds.gz", "TOP", vec!["pSD.c1"; 3], vec![])]
fn test_psd(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_PSD, gds, topcell, &ignore), expected);
}

// --- poly resistors (Rsil/Rppd/Rhigh) ---

const DECK_RESISTOR: &str = "resistor";

#[rstest]
#[case::rsil("resistor/Rsil.gds.gz", "TOP", vec!["Rsil.b", "Rsil.f", "Rsil.f"], vec!["Rsil.c"])]
#[case::rsil_body("resistor/RsilBody.gds.gz", "TOP",
    vec!["Rsil.a", "Rsil.a", "Rsil.d", "Rsil.e", "Rsil.f", "Rsil.f"], vec![])]
#[case::rsil_c("resistor/Rsil.c.gds.gz", "TOP", vec!["Rsil.c"; 1], vec![])]
#[case::rppd_body("resistor/RppdBody.gds.gz", "TOP",
    vec!["Rppd.a", "Rppd.a", "Rppd.b", "Rppd.d", "Rppd.e", "Rppd.e"], vec![])]
// gap = 0.20 (boundary-exact, clean), 0.10 (< 0.20, min_space), 0.50 (> 0.20, too far).
// Rppd.b collateral: pSD drawn flush with SalBlock (coincident edge, same class as
// RppdBody above) — not the point of this case.
#[case::rppd_c("resistor/Rppd.c.gds.gz", "TOP", vec!["Rppd.c"; 2], vec!["Rppd.b"])]
#[case::rhigh("resistor/Rhigh.gds.gz", "TOP",
    vec!["Rhi.a", "Rhi.a", "Rhi.c", "Rhi.c", "Rhi.f", "Rhi.f"], vec![])]
// Same three-gap pattern as Rppd.c.  Rhi.c collateral (GatPoly extends past pSDnSD)
// fires legitimately since run_enclosure measures partially-overlapping shapes — as
// KLayout always did on this geometry — but it is not this case's concern.
#[case::rhi_d("resistor/Rhi.d.gds.gz", "TOP", vec!["Rhi.d"; 2], vec!["Rhi.c"])]
// nSD overhang at a realistic resistor (fires — stricter than shipped KLayout, see the
// fixture doc) and at a poly-inside-stack body (fires in both tools, exact-location
// validated); identical-implant body and isolated nSD blob stay clean.  Rhi.c ×4: the
// resistor GatPoly sticks out of the implant stack with flush edges on two opposite
// sides of each of two shapes — KLayout marks the same 4 per-edge markers at the
// identical coordinates; measured since run_enclosure's partial-overlap branch landed.
#[case::rhi_b("resistor/Rhi.b.gds.gz", "TOP",
    vec!["Rhi.b", "Rhi.b", "Rhi.c", "Rhi.c", "Rhi.c", "Rhi.c"], vec![])]
fn test_resistor(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_RESISTOR, gds, topcell, &ignore), expected);
}

// --- isolated NMOS (nmosi) ---

const DECK_NMOSI: &str = "nmosi";

#[rstest]
#[case::nmosi_b("nmosi/nmosi.b.gds.gz", "TOP", vec!["nmosi.b"], vec![])]
// Ring gap 0.34 fires, ring gap 0.50 clean, and the plain (hole-free) NWell 0.20 from
// an iso-PWell Activ stays silent — only `NWell.with_holes` anchors this rule.
#[case::nmosi_c("nmosi/nmosi.c.gds.gz", "TOP", vec!["nmosi.c"], vec![])]
#[case::nmosi_d("nmosi/nmosi.d.gds.gz", "TOP", vec!["nmosi.d", "nmosi.d"], vec![])]
#[case::nmosi_f("nmosi/nmosi.f.gds.gz", "TOP", vec!["nmosi.f", "nmosi.f"], vec!["nmosi.g"])]
// SalBlock extension past nSD:block over the ptap: 0.05 fires (matches KLayout's marker
// exactly), 0.20 clean, flush-0.00 fires here only (PDF-first; KLayout's coincident-pair
// marker is zero-area and vanishes under its .and(Activ) — see the deck comment).
#[case::nmosi_g("nmosi/nmosi.g.gds.gz", "TOP", vec!["nmosi.g"; 2], vec![])]
fn test_nmosi(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NMOSI, gds, topcell, &ignore), expected);
}

// --- NPN bipolar (npnG2 substrate ties, npn13G2* emitters) ---

const DECK_NPN: &str = "npn";

#[rstest]
// The tie ring is under-enclosed by 0.05 the whole way round; npnG2.c reports it
// twice, once for the ring's outer boundary and once for its hole, each a closed run of
// short walls (KLayout reports the same violation as 8 edge pairs — same device, same
// rule).
#[case::npn_g2("npn/npnG2.gds.gz", "TOP",
    vec!["npnG2.b", "npnG2.c", "npnG2.c", "npnG2.d", "npnG2.e"], vec![])]
// One marker per emitter case: min/max for G2 (=0.90), L (1.00..2.50), V (1.00..5.00).
// KLayout's shipped emitter rules are dead code (µm/dbu bug in ext_with_length's
// ">" branch) — limits here follow the PDF; see the deck comment.
#[case::npn_13g2("npn/npn13G2.gds.gz", "TOP",
    vec!["npn13G2.a", "npn13G2.a", "npn13G2L.a", "npn13G2L.b", "npn13G2V.a", "npn13G2V.b"],
    vec![])]
fn test_npn(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_NPN, gds, topcell, &ignore), expected);
}

// --- Schottky diode (Sdiod) ---

const DECK_SDIOD: &str = "sdiod";

#[rstest]
#[case::sdiod("sdiod/Sdiod.gds.gz", "TOP",
    vec!["Sdiod.a", "Sdiod.b", "Sdiod.c", "Sdiod.d", "Sdiod.e"], vec![])]
fn test_sdiod(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SDIOD, gds, topcell, &ignore), expected);
}

// --- metal slits ---

const DECK_SLIT: &str = "slit";

#[rstest]
#[case("slit/Slt.a.gds.gz", "TOP", vec!["Slt.a"; 2], vec![])]
#[case("slit/Slt.b.gds.gz", "TOP", vec!["Slt.b"; 2], vec![])]
#[case("slit/Slt.c.gds.gz", "TOP", vec!["Slt.c"], vec![])]
// A TopMetal2 slit on the pad and a Metal1 slit under it fire (the pad region is the
// whole dfpad shape, on every layer); a slotted plate away from any pad and one under
// a dfpad without a Passiv opening (not a pad) stay clean.
#[case("slit/Slt.e.gds.gz", "TOP", vec!["Slt.e"; 2], vec![])]
#[case("slit/Slt.f.gds.gz", "TOP", vec!["Slt.f"], vec![])]
#[case("slit/Slt.h1.gds.gz", "TOP", vec!["Slt.h1"], vec![])]
#[case("slit/Slt.i.gds.gz", "TOP", vec!["Slt.i"], vec![])]
fn test_slit(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SLIT, gds, topcell, &ignore), expected);
}

// --- latch-up ---
//
// LU.c/LU.d (and LU.c1/LU.d1) enforce the identical "tie Activ within 6 µm of its Cont"
// constraint, so a tie-extension fixture trips both ids of the pair.

const DECK_LU: &str = "lu";

#[rstest]
#[case("lu/LU.a.gds.gz", "TOP", vec!["LU.a"], vec![])]
#[case("lu/LU.b.gds.gz", "TOP", vec!["LU.b"], vec![])]
#[case("lu/LU.c.gds.gz", "TOP", vec!["LU.c", "LU.d"], vec![])]
#[case("lu/LU.c1.gds.gz", "TOP", vec!["LU.c1", "LU.d1"], vec![])]
fn test_lu(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_LU, gds, topcell, &ignore), expected);
}

// --- antenna ---
//
// Ant.i is purely geometric (p-diode in the PWell fires; one correctly placed in an NWell
// stays clean).  Ant.b/e and Ant.d/f are net-aware (antenna_ratio over the connectivity
// engine): in the Ant.b fixture gate A's bare Metal1 antenna trips Ant.b, while gate B's
// identical antenna also reaches a diffusion diode, so the relaxed Ant.e limit keeps it
// clean.  (Ant.a/c/g/h still pending.)

const DECK_ANTENNA: &str = "antenna";

#[rstest]
#[case("antenna/Ant.i.gds.gz", "TOP", vec!["Ant.i"], vec![])]
#[case("antenna/Ant.b.gds.gz", "TOP", vec!["Ant.b"], vec![])]
// Per-level cumulative: G1's Metal1 antenna trips Ant.b even though it merges with G2 at
// Metal2 (a final-net ratio would dilute below the limit). Validated against KLayout.
#[case("antenna/Ant.merge.gds.gz", "TOP", vec!["Ant.b"], vec![])]
// Pre-metal antennas: a poly-over-field gate trips Ant.a, a large-contact gate trips Ant.c.
#[case("antenna/Ant.ac.gds.gz", "TOP", vec!["Ant.a", "Ant.c"], vec![])]
// An undersized gate-connected protection diode trips Ant.g.
#[case("antenna/Ant.g.gds.gz", "TOP", vec!["Ant.g"], vec![])]
// A bare n-diode in an NWell trips Ant.h; an identical one inside a text-tagged `isolbox`
// is exempt. Exercises text-label matching + region-interaction selectors.
#[case("antenna/Ant.h.gds.gz", "TOP", vec!["Ant.h"], vec![])]
fn test_antenna(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_ANTENNA, gds, topcell, &ignore), expected);
}

// --- sealring ---

const DECK_SEALRING: &str = "sealring";

#[rstest]
#[case::seal_l_clean("static/sealring/Seal.l.gds.gz", "TOP", vec![], vec![])]
#[case::seal_l_fail("static/sealring/Seal.l.fail.gds.gz", "TOP", vec!["Seal.l"; 2], vec![])]
#[case::seal_n_clean("static/sealring/Seal.n.gds.gz", "TOP", vec![], vec![])]
#[case::seal_n_fail("static/sealring/Seal.n.fail.gds.gz", "TOP", vec!["Seal.n"], vec!["Seal.e"])]
// A thin passiv ring 0.5 µm from a seal-Activ: Seal.e (frame < 4.20) + Seal.f (space < 1.00).
// Seal.n (ring-covering) is incidental to the synthetic ring and ignored here.
#[case::seal_ef("sealring/Seal.ef.gds.gz", "TOP", { let mut v = vec!["Seal.e"; 8]; v.push("Seal.f"); v }, vec!["Seal.n"])]
#[case::seal_b("sealring/Seal.b.gds.gz", "TOP", vec!["Seal.b"; 2], vec!["Seal.l"])]
// Two seal frames: Cont ring at 0.80 from the frame edge (< 1.30) and at 1.50 (clean).
// Seal.l is synthetic two-frame collateral.
//
// One marker: the Cont ring's four outer walls are all 0.80 from the frame, and four
// short walls that meet at their corners are one run of walls, reported once at its
// worst.  KLayout reports one edge pair per side; the case used to read three, which
// were the pieces the tiling cut the ring into, and moved with the tile size.
#[case::seal_d("sealring/Seal.d.gds.gz", "TOP", vec!["Seal.d"; 1], vec!["Seal.l"])]
fn test_sealring(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_SEALRING, gds, topcell, &ignore), expected);
}

// --- MIM ---
//
// One layout exercising the MIM rules; MIM.b/f/h match KLayout exactly (MIM.c/d live in the
// BEOL sub-deck, MIM.a differs only in min_width edge granularity).

const DECK_MIM: &str = "mim";

#[rstest]
#[case("mim/MIM.gds.gz", "TOP",
    vec!["MIM.a", "MIM.a", "MIM.a", "MIM.a", "MIM.b", "MIM.d", "MIM.d", "MIM.d", "MIM.f", "MIM.h"],
    vec![])]
#[case("mim/MIM.gR.gds.gz", "TOP", vec!["MIM.gR"], vec![])]
fn test_mim(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(drc(PDK_IHP, DECK_MIM, gds, topcell, &ignore), expected);
}

// --- forbidden ---

const DECK_FORBIDDEN: &str = "forbidden";

#[rstest]
#[case("forbidden.gds.gz", "TOP", vec!["forbidden"; 11], vec![])]
fn test_forbidden(
    #[case] gds: &str,
    #[case] topcell: &str,
    #[case] mut expected: Vec<&str>,
    #[case] ignore: Vec<&str>,
) {
    expected.sort();
    assert_eq!(
        drc(PDK_IHP, DECK_FORBIDDEN, gds, topcell, &ignore),
        expected
    );
}

// --- offgrid ---
//
// Each fixture places one off-grid shape on a single layer; its shifted right edge
// has two off-grid vertices, so exactly that rule fires twice.  The layer set mirrors
// `gen/ihp_sg13g2/offgrid.rs` (and the IHP reference deck).

const DECK_OFFGRID: &str = "offgrid";

/// Primary layer of every offgrid rule; the rule id is `<layer>.offgrid`.
const OFFGRID_LAYERS: &[&str] = &[
    "Activ",
    "GatPoly",
    "PolyRes",
    "Cont",
    "nSD",
    "pSD",
    "SalBlock",
    "ThickGateOx",
    "NLDB",
    "PLDB",
    "NLDD",
    "PLDD",
    "NExt",
    "PExt",
    "NExtHV",
    "PExtHV",
    "EXTBlock",
    "NWell",
    "PWell",
    "nBuLay",
    "nBuLayCut",
    "isoNWell",
    "INLDPWL",
    "IC",
    "Substrate",
    "Metal1",
    "Metal2",
    "Metal3",
    "Metal4",
    "Metal5",
    "Via1",
    "Via2",
    "Via3",
    "Via4",
    "MIM",
    "Vmim",
    "TopVia1",
    "TopMetal1",
    "TopVia2",
    "TopMetal2",
    "Passiv",
    "AntMetal1",
    "BackMetal1",
    "BackPassiv",
    "AlCuStop",
    "DeepVia",
    "LBE",
    "BiWind",
    "PEmWind",
    "BasPoly",
    "EmWind",
    "EmWiHV",
    "EmPoly",
    "PEmPoly",
    "PBiWind",
    "DeepCo",
    "ColOpen",
    "ColWind",
    "CtrGat",
    "LDMOS",
    "FBE",
    "FGEtch",
    "FGImp",
    "FLM",
    "HafniumOx",
    "ThinFilmRes",
    "GraphGate",
    "MEMPAD",
    "MEMVia",
    "RFMEM",
    "SNSRing",
    "Sensor",
    "SNSArms",
    "SNSCMOSVia",
    "SNSBotVia",
    "SNSTopVia",
    "prBoundary",
    "Exchange0",
    "Exchange1",
    "Exchange2",
    "Exchange3",
    "Exchange4",
];

#[test]
fn test_offgrid() {
    for name in OFFGRID_LAYERS {
        let id = format!("{name}.offgrid");
        let gds = format!("offgrid/{id}.gds.gz");
        assert_eq!(
            drc(PDK_IHP, DECK_OFFGRID, &gds, "TOP", &[]),
            vec![id.as_str(); 2],
            "offgrid rule {id} should fire exactly twice on its own fixture",
        );
    }
}
