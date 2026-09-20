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
#[case("activ/AFil.a.gds.gz", "TOP", vec!["AFil.a", "AFil.a", "AFil.a", "AFil.a"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.a1.gds.gz", "TOP", vec!["AFil.a1", "AFil.a1", "AFil.a1", "AFil.a1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.b.gds.gz", "TOP", vec!["AFil.b", "AFil.b"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.cont.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c.gatpoly.gds.gz", "TOP", vec!["AFil.c", "AFil.c"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.c1.gds.gz", "TOP", vec!["AFil.c1", "AFil.c1"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nwell.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.d.nbulay.gds.gz", "TOP", vec!["AFil.d", "AFil.d"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.e.gds.gz", "TOP", vec!["AFil.e", "AFil.e"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.i.gds.gz", "TOP", vec!["AFil.i", "AFil.i"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.j.gds.gz", "TOP", vec!["AFil.j"], vec!["AFil.g", "AFil.g1", "AFil.g2", "AFil.g3"])]
#[case("activ/AFil.g.gds.gz", "TOP", vec![], vec!["AFil.a"])]
#[case("activ/AFil.g.fail.gds.gz", "TOP", vec!["AFil.g"], vec!["AFil.a"])]
#[case("activ/AFil.g1.gds.gz", "TOP", vec![], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g1.fail.gds.gz", "TOP", vec!["AFil.g1"], vec!["AFil.a", "AFil.g3"])]
#[case("activ/AFil.g2.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g2.fail.gds.gz", "TOP", vec!["AFil.g2"; 4], vec!["Act.b", "AFil.g"])]
#[case("activ/AFil.g3.gds.gz", "TOP", vec![], vec!["Act.b", "AFil.g1"])]
#[case("activ/AFil.g3.fail.gds.gz", "TOP", vec!["AFil.g3"; 4], vec!["Act.b", "AFil.g1"])]
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
// inside the Activ is no drain/source either (Gat.c's business) - report, finding 1.
#[case::act_c_h3("activ/Act.c.h3.gds.gz", "TOP", vec!["Act.c"; 2], dens(&["Act.a", "Act.d"]))]
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
// A 5.005 square fires (four walls).  A 5.005 × 5.0 filler is 5.0 wide, a 3 × 20 bar 3, an L
// with 3-wide arms 3: clean - the width is the smaller span (report, finding 4).
#[case::afil_a_h1("activ/AFil.a.h1.gds.gz", "TOP", vec!["AFil.a"; 4], dens(&[]))]
// A 5.02 diamond (4), a 5.02 45° strip (2), a 5.005 × 6 union (2) and a 5.005 square from
// four boxes (4) fire; 4.95, a 5.0 × 6 union and a 6 × 6 filler with a hole (2.75 wide) are
// clean.
#[case::afil_a_h2("activ/AFil.a.h2.gds.gz", "TOP", vec!["AFil.a"; 12], dens(&[]))]
// Seven 5.005 squares on, across and straddling x = 20/21/40/42 (4 each) and a 5.005-tall
// 30 µm bar (2); a 5.0 square straddling 20 and a 5.0-tall bar are clean.
#[case::afil_a_h3("activ/AFil.a.h3.gds.gz", "TOP", vec!["AFil.a"; 30], dens(&[]))]
// Fifty 5.005 squares, flat and as a GdsArrayRef.
#[case::afil_a_h4("activ/AFil.a.h4.gds.gz", "TOP", vec!["AFil.a"; 200], dens(&[]))]
#[case::afil_a_h5("activ/AFil.a.h5.gds.gz", "TOP", vec!["AFil.a"; 200], dens(&[]))]
// A 300 × 5.005 bar (2) and a 5.005 square at (1000, 1000) (4).
#[case::afil_a_h6("activ/AFil.a.h6.gds.gz", "TOP", vec!["AFil.a"; 6], dens(&[]))]
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
// not GatPoly; a Cont overlapping the filler's edge is at no distance (fires) - report,
// finding 5.
#[case::afil_c_h3("activ/AFil.c.h3.gds.gz", "TOP", vec!["AFil.c"; 3], dens(&[]))]
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
// Activ.mask is not Activ, a filler is AFil.b's; an Activ abutting a filler and one
// overlapping it are at no distance (fire) - report, finding 5.
#[case::afil_c1_h3("activ/AFil.c1.h3.gds.gz", "TOP", vec!["AFil.c1"; 2], dens(&["AFil.b"]))]
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
// inside an nBuLay fire, 1.0 inside is clean; fillers crossing a NWell's and an nBuLay's
// edge are at no distance (fire) - report, finding 6.
#[case::afil_d_h2("activ/AFil.d.h2.gds.gz", "TOP", vec!["AFil.d"; 4], dens(&[]))]
// Section 4.2's nBuLay: a filler 1.5 from a 3.0 µm NWell is 0.5 from its derived nBuLay
// (fires), 1.5 from a 2.995 well and 2.0 from a 3.0 well are clean; a drawn nBuLay under
// nBuLay:block is none (0.5 away: clean), a half-blocked one fires for the filler 0.5 from
// its open half only - report, finding 7.
#[case::afil_d_h3("activ/AFil.d.h3.gds.gz", "TOP", vec!["AFil.d"; 2], dens(&[]))]
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
// A filler inside a TRANS and one crossing its edge are at no distance (fire) - report,
// finding 6; a TRANS diamond tip at 0.995 fires.
#[case::afil_e_h2("activ/AFil.e.h2.gds.gz", "TOP", vec!["AFil.e"; 3], dens(&[]))]
// Eight 0.995 gaps on, across and straddling x = 20/21/40/42, one along a 10 µm filler.
#[case::afil_e_h3("activ/AFil.e.h3.gds.gz", "TOP", vec!["AFil.e"; 8], dens(&["AFil.a"]))]
// Fifty filler/TRANS pairs at 0.995, flat and as a GdsArrayRef.
#[case::afil_e_h4("activ/AFil.e.h4.gds.gz", "TOP", vec!["AFil.e"; 50], dens(&[]))]
#[case::afil_e_h5("activ/AFil.e.h5.gds.gz", "TOP", vec!["AFil.e"; 50], dens(&[]))]
// A 300 µm TRANS 0.995 from a 300 µm filler, a pair at (1000, 1000).
#[case::afil_e_h6("activ/AFil.e.h6.gds.gz", "TOP", vec!["AFil.e"; 2], dens(&["AFil.a"]))]
// PWell:block at 1.495, at a 1.06/1.06 diagonal (1.499) and corner-on fire; 1.5/1.506 clean.
#[case::afil_i_h1("activ/AFil.i.h1.gds.gz", "TOP", vec!["AFil.i"; 3], dens(&[]))]
// "Space to edges": a filler inside a block 1.495 from its edge fires (1.5 is clean), a
// filler crossing the edge is at no distance (fires) - report, finding 8.
#[case::afil_i_h2("activ/AFil.i.h2.gds.gz", "TOP", vec!["AFil.i"; 2], dens(&[]))]
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
#[case("gatpoly/Gat.b1.gds.gz", "TOP", vec!["Gat.b1"], vec!["GFil.g"])]
#[case("gatpoly/Gat.c.gds.gz", "TOP", vec!["Gat.c"], vec!["GFil.g"])]
#[case("gatpoly/Gat.d.gds.gz", "TOP", vec!["Gat.d", "Gat.d"], vec!["GFil.g"])]
#[case("gatpoly/Gat.e.gds.gz", "TOP", vec!["Gat.e"], vec!["GFil.g"])]
// Gat.f and Gat.g draw 45° poly with square ends: the acute tips are Gat.a widths, as
// they are for KLayout, and are set aside here.
#[case("gatpoly/Gat.f.gds.gz", "TOP", vec!["Gat.f", "Gat.f"], vec!["GFil.g", "Gat.c", "Gat.e", "Gat.a"])]
#[case("gatpoly/Gat.g.gds.gz", "TOP", vec!["Gat.g", "Gat.g"], vec!["Gat.e", "GFil.g", "Gat.a"])]
#[case("gatpoly/GFil.a.gds.gz", "TOP", vec!["GFil.a"; 4], vec!["GFil.g"])]
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

#[rstest]
#[case::m1_a("metal1/M1.a.gds.gz", "TOP", vec!["M1.a", "M1.a", "M1.a", "M1.a"], vec!["M1.d", "M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_space("metal1/M1.b.space.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_b_notch("metal1/M1.b.notch.gds.gz", "TOP", vec!["M1.b", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_corner("metal1/M1.corner.gds.gz", "TOP", vec!["M1.a", "M1.b"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1_j_ok("metal1/M1.j.gds.gz", "TOP", vec![], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_j_fail("metal1/M1.j.fail.gds.gz", "TOP", vec!["M1.j"], vec!["M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_k_ok("metal1/M1.k.gds.gz", "TOP", vec![], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1_k_fail("metal1/M1.k.fail.gds.gz", "TOP", vec!["M1.k"], vec!["M1Fil.k", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2", "M1Fil.a2"])]
#[case::m1fil_c("metal1/M1Fil.c.gds.gz", "TOP", vec!["M1Fil.c", "M1Fil.c"], vec!["M1.j", "M1.k", "M1Fil.h", "M1Fil.k"])]
#[case::m1fil_h_ok("metal1/M1Fil.h.gds.gz", "TOP", vec![], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_fail("metal1/M1Fil.h.fail.gds.gz", "TOP", vec!["M1Fil.h"; 4], vec!["M1.b", "M1.j"])]
#[case::m1fil_h_boundary_ok("metal1/M1Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_h_boundary_fail("metal1/M1Fil.h.boundary_fail.gds.gz", "TOP", vec!["M1Fil.h"], vec![])]
#[case::m1fil_h_boundary_ring("metal1/M1Fil.h.boundary_ring.gds.gz", "TOP", vec![], vec![])]
#[case::m1fil_k_ok("metal1/M1Fil.k.gds.gz", "TOP", vec![], vec!["M1.b", "M1.k"])]
#[case::m1fil_k_fail("metal1/M1Fil.k.fail.gds.gz", "TOP", vec!["M1Fil.k"; 4], vec!["M1.b", "M1.k"])]
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
#[case::m2_j_ok("metal2/M2.j.gds.gz", "TOP", vec![], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_j_fail("metal2/M2.j.fail.gds.gz", "TOP", vec!["M2.j"], vec!["M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_k_ok("metal2/M2.k.gds.gz", "TOP", vec![], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2_k_fail("metal2/M2.k.fail.gds.gz", "TOP", vec!["M2.k"], vec!["M2Fil.k", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2", "M2Fil.a2"])]
#[case::m2fil_c("metal2/M2Fil.c.gds.gz", "TOP", vec!["M2Fil.c", "M2Fil.c"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a1("metal2/M2Fil.a1.gds.gz", "TOP", vec!["M2Fil.a1"; 4], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_a2("metal2/M2Fil.a2.gds.gz", "TOP", vec!["M2Fil.a2"; 4], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_b("metal2/M2Fil.b.gds.gz", "TOP", vec!["M2Fil.b", "M2Fil.b"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_d("metal2/M2Fil.d.gds.gz", "TOP", vec!["M2Fil.d", "M2Fil.d"], vec!["M2.j", "M2.k", "M2Fil.h", "M2Fil.k"])]
#[case::m2fil_h_ok("metal2/M2Fil.h.gds.gz", "TOP", vec![], vec!["M2.b", "M2.j"])]
#[case::m2fil_h_fail("metal2/M2Fil.h.fail.gds.gz", "TOP", vec!["M2Fil.h"; 4], vec!["M2.b", "M2.j"])]
#[case::m2fil_k_ok("metal2/M2Fil.k.gds.gz", "TOP", vec![], vec!["M2.b", "M2.k"])]
#[case::m2fil_k_fail("metal2/M2Fil.k.fail.gds.gz", "TOP", vec!["M2Fil.k"; 4], vec!["M2.b", "M2.k"])]
#[case::m2fil_h_boundary_ok("metal2/M2Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m2fil_h_boundary_fail("metal2/M2Fil.h.boundary_fail.gds.gz", "TOP", vec!["M2Fil.h"], vec![])]
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
#[case::m3_j_ok("metal3/M3.j.gds.gz", "TOP", vec![], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_j_fail("metal3/M3.j.fail.gds.gz", "TOP", vec!["M3.j"], vec!["M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_k_ok("metal3/M3.k.gds.gz", "TOP", vec![], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3_k_fail("metal3/M3.k.fail.gds.gz", "TOP", vec!["M3.k"], vec!["M3Fil.k", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2", "M3Fil.a2"])]
#[case::m3fil_c("metal3/M3Fil.c.gds.gz", "TOP", vec!["M3Fil.c", "M3Fil.c"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a1("metal3/M3Fil.a1.gds.gz", "TOP", vec!["M3Fil.a1"; 4], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_a2("metal3/M3Fil.a2.gds.gz", "TOP", vec!["M3Fil.a2"; 4], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_b("metal3/M3Fil.b.gds.gz", "TOP", vec!["M3Fil.b", "M3Fil.b"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_d("metal3/M3Fil.d.gds.gz", "TOP", vec!["M3Fil.d", "M3Fil.d"], vec!["M3.j", "M3.k", "M3Fil.h", "M3Fil.k"])]
#[case::m3fil_h_ok("metal3/M3Fil.h.gds.gz", "TOP", vec![], vec!["M3.b", "M3.j"])]
#[case::m3fil_h_fail("metal3/M3Fil.h.fail.gds.gz", "TOP", vec!["M3Fil.h"; 4], vec!["M3.b", "M3.j"])]
#[case::m3fil_k_ok("metal3/M3Fil.k.gds.gz", "TOP", vec![], vec!["M3.b", "M3.k"])]
#[case::m3fil_k_fail("metal3/M3Fil.k.fail.gds.gz", "TOP", vec!["M3Fil.k"; 4], vec!["M3.b", "M3.k"])]
#[case::m3fil_h_boundary_ok("metal3/M3Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m3fil_h_boundary_fail("metal3/M3Fil.h.boundary_fail.gds.gz", "TOP", vec!["M3Fil.h"], vec![])]
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
#[case::m4_j_ok("metal4/M4.j.gds.gz", "TOP", vec![], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_j_fail("metal4/M4.j.fail.gds.gz", "TOP", vec!["M4.j"], vec!["M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_k_ok("metal4/M4.k.gds.gz", "TOP", vec![], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4_k_fail("metal4/M4.k.fail.gds.gz", "TOP", vec!["M4.k"], vec!["M4Fil.k", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2", "M4Fil.a2"])]
#[case::m4fil_c("metal4/M4Fil.c.gds.gz", "TOP", vec!["M4Fil.c", "M4Fil.c"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a1("metal4/M4Fil.a1.gds.gz", "TOP", vec!["M4Fil.a1"; 4], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_a2("metal4/M4Fil.a2.gds.gz", "TOP", vec!["M4Fil.a2"; 4], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_b("metal4/M4Fil.b.gds.gz", "TOP", vec!["M4Fil.b", "M4Fil.b"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_d("metal4/M4Fil.d.gds.gz", "TOP", vec!["M4Fil.d", "M4Fil.d"], vec!["M4.j", "M4.k", "M4Fil.h", "M4Fil.k"])]
#[case::m4fil_h_ok("metal4/M4Fil.h.gds.gz", "TOP", vec![], vec!["M4.b", "M4.j"])]
#[case::m4fil_h_fail("metal4/M4Fil.h.fail.gds.gz", "TOP", vec!["M4Fil.h"; 4], vec!["M4.b", "M4.j"])]
#[case::m4fil_k_ok("metal4/M4Fil.k.gds.gz", "TOP", vec![], vec!["M4.b", "M4.k"])]
#[case::m4fil_k_fail("metal4/M4Fil.k.fail.gds.gz", "TOP", vec!["M4Fil.k"; 4], vec!["M4.b", "M4.k"])]
#[case::m4fil_h_boundary_ok("metal4/M4Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m4fil_h_boundary_fail("metal4/M4Fil.h.boundary_fail.gds.gz", "TOP", vec!["M4Fil.h"], vec![])]
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
#[case::m5_j_ok("metal5/M5.j.gds.gz", "TOP", vec![], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_j_fail("metal5/M5.j.fail.gds.gz", "TOP", vec!["M5.j"], vec!["M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_k_ok("metal5/M5.k.gds.gz", "TOP", vec![], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5_k_fail("metal5/M5.k.fail.gds.gz", "TOP", vec!["M5.k"], vec!["M5Fil.k", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2", "M5Fil.a2"])]
#[case::m5fil_c("metal5/M5Fil.c.gds.gz", "TOP", vec!["M5Fil.c", "M5Fil.c"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a1("metal5/M5Fil.a1.gds.gz", "TOP", vec!["M5Fil.a1"; 4], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_a2("metal5/M5Fil.a2.gds.gz", "TOP", vec!["M5Fil.a2"; 4], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_b("metal5/M5Fil.b.gds.gz", "TOP", vec!["M5Fil.b", "M5Fil.b"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_d("metal5/M5Fil.d.gds.gz", "TOP", vec!["M5Fil.d", "M5Fil.d"], vec!["M5.j", "M5.k", "M5Fil.h", "M5Fil.k"])]
#[case::m5fil_h_ok("metal5/M5Fil.h.gds.gz", "TOP", vec![], vec!["M5.b", "M5.j"])]
#[case::m5fil_h_fail("metal5/M5Fil.h.fail.gds.gz", "TOP", vec!["M5Fil.h"; 4], vec!["M5.b", "M5.j"])]
#[case::m5fil_k_ok("metal5/M5Fil.k.gds.gz", "TOP", vec![], vec!["M5.b", "M5.k"])]
#[case::m5fil_k_fail("metal5/M5Fil.k.fail.gds.gz", "TOP", vec!["M5Fil.k"; 4], vec!["M5.b", "M5.k"])]
#[case::m5fil_h_boundary_ok("metal5/M5Fil.h.boundary_ok.gds.gz", "TOP", vec![], vec![])]
#[case::m5fil_h_boundary_fail("metal5/M5Fil.h.boundary_fail.gds.gz", "TOP", vec!["M5Fil.h"], vec![])]
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
#[case::tm1fil_a1("topmetal1/TM1Fil.a1.gds.gz", "TOP", vec!["TM1Fil.a1"; 4], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_b("topmetal1/TM1Fil.b.gds.gz", "TOP", vec!["TM1Fil.b"; 2], vec!["TM1.c", "TM1.d"])]
#[case::tm1fil_d("topmetal1/TM1Fil.d.gds.gz", "TOP", vec!["TM1Fil.d"; 2], vec!["TM1.c", "TM1.d"])]
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
#[case::tm2fil_a1("topmetal2/TM2Fil.a1.gds.gz", "TOP", vec!["TM2Fil.a1"; 4], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_b("topmetal2/TM2Fil.b.gds.gz", "TOP", vec!["TM2Fil.b"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2fil_d("topmetal2/TM2Fil.d.gds.gz", "TOP", vec!["TM2Fil.d"; 2], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_fail("topmetal2/TM2.bR.fail.gds.gz", "TOP", vec!["TM2.bR"], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ok("topmetal2/TM2.bR.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
#[case::tm2_br_ind("topmetal2/TM2.bR.ind.gds.gz", "TOP", vec![], vec!["TM2.c", "TM2.d"])]
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
#[case::lbe_b("lbe/LBE.b.gds.gz", "TOP", vec!["LBE.b"; 4], vec!["LBE.b1", "LBE.i"])]
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
#[case::pad_a1("pad/Pad.a1.gds.gz", "TOP", vec!["Pad.a1"; 4], vec!["Pad.d", "Padb.a", "Padc.a", "Pad.i"])]
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
