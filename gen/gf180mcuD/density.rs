// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Global density: a good and a bad pattern for every rule in the `density` deck.
//!
//! These rules are the one family here that cannot be tested a layer at a time.  Each is
//! a coverage fraction of the whole die, so a pattern that draws only the layer under
//! test leaves every *other* layer at zero and trips all of them at once.  Each fixture
//! therefore lays down a passing baseline on every layer the deck reads and moves one of
//! them, which is also how a real die fails these: everything is filled and one layer is
//! short.
//!
//! A pattern is a boundary box with horizontal stripes across it, so the measured density
//! is the summed stripe height over the box side and the number is readable off the
//! source.  The drawn layer carries all of it and the dummy layer stays empty; the deck
//! sums the pair, and which of the two the coverage comes from is not what these check.

use crate::helpers::{density_pattern, layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/density";
/// The die, and the denominator of every fraction here.
const SIZE: f64 = 100.0;
/// Clears every floor in the deck (the highest is 30%) and stays under DCF.1d's 70% cap.
const BASE: f64 = 40.0;

/// Every layer the deck measures, and the rule that floors it.
const LAYERS: &[(&str, &str)] = &[
    ("comp", "DCF.1b"),
    ("poly2_drawn", "PL.8"),
    ("metal1_drawn", "M1.4"),
    ("metal2_drawn", "M2.4"),
    ("metal3_drawn", "M3.4"),
    ("metal4_drawn", "M4.4"),
    ("metal5_drawn", "M5.4"),
];

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");

    // One fixture per floor: everything at BASE, and the rule's own layer taken under.
    for &(lname, id) in LAYERS {
        for (name, pct) in [("good", BASE), ("bad", 10.0)] {
            write(pdk, &format!("{id}.{name}"), Some((lname, pct)));
        }
    }
    // MT.3 measures the top metal that M5.4 does, so it shares M5.4's geometry; the two
    // are one measurement under two names and no pattern can separate them.
    for (name, pct) in [("good", BASE), ("bad", 10.0)] {
        write(pdk, &format!("MT.3.{name}"), Some(("metal5_drawn", pct)));
    }
    // DCF.1d is the ceiling over the floor DCF.1b sets, so its bad half goes *up*.
    for (name, pct) in [("good", BASE), ("bad", 75.0)] {
        write(pdk, &format!("DCF.1d.{name}"), Some(("comp", pct)));
    }

    hardening(pdk);
}

/// The die with every layer at [`BASE`], except `moved`, which is set as given.
fn write(pdk: &PdkConfig, name: &str, moved: Option<(&str, f64)>) {
    // Every stripe starts at the bottom, so they overlap: density is measured per layer,
    // never between them, and stacking keeps each fraction readable on its own.
    let stripes: Vec<((i16, i16), f64, f64)> = LAYERS
        .iter()
        .map(|&(lname, _)| {
            let pct = match moved {
                Some((m, p)) if m == lname => p,
                _ => BASE,
            };
            (layer(pdk, lname), 0.0, SIZE * pct / 100.0)
        })
        .collect();
    write_gz(
        &format!("{DIR}/{name}.gds.gz"),
        library("TOP", density_pattern((0, 0), SIZE, &stripes)),
    );
}

// --- Hardening (hardening/SPEC.md) ------------------------------------------------
//
// The deck's own conditions, not the check's: which layers each rule sums, what the
// percentage is taken over, and what happens when the die is not a plain square.  Every
// fixture carries all seven measured layers, because all nine rules run on every layout
// and a layer left empty trips its own rule; only the layer a fixture is about moves.

/// The five metal layers, which share one floor.
const METALS: &[&str] = &[
    "metal1_drawn",
    "metal2_drawn",
    "metal3_drawn",
    "metal4_drawn",
    "metal5_drawn",
];

/// A full-width band of `pct` percent of the die on one layer, laid from the bottom.
fn band(pdk: &PdkConfig, lname: &str, pct: f64) -> GdsElement {
    rect(layer(pdk, lname), 0.0, 0.0, SIZE, SIZE * pct / 100.0)
}

/// The die square on `pr_bndry`.
fn die(pdk: &PdkConfig) -> GdsElement {
    rect(layer(pdk, "pr_bndry"), 0.0, 0.0, SIZE, SIZE)
}

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

/// Every measured layer at `pct`, on the plain die.
fn all_at(pdk: &PdkConfig, pct: f64) -> Vec<GdsElement> {
    let mut v = vec![die(pdk)];
    v.extend(LAYERS.iter().map(|&(l, _)| band(pdk, l, pct)));
    v
}

fn hardening(pdk: &PdkConfig) {
    // Each rule's own bound on one die: DCF.1b's 25%, PL.8's 14%, the metals' 30%.  The
    // manual reads the metal floors as "> 30%" and the other two as ">="; the foundry's
    // own runset reads every one of them as ">=", and a die exactly on the number is
    // what a fill run aims at.
    let at_bound = |step: f64| {
        let mut v = vec![die(pdk)];
        v.push(band(pdk, "comp", 25.0 - step));
        v.push(band(pdk, "poly2_drawn", 14.0 - step));
        for l in METALS {
            v.push(band(pdk, l, 30.0 - step));
        }
        v
    };
    // DCF.1b.h1: every floor met exactly.  Nothing fires.
    hwrite("DCF.1b.h1", at_bound(0.0));
    // DCF.1b.h2: every floor missed by one grid step of band height (0.005 µm over a
    // 100 µm die is 0.005%).  All eight floors fire, MT.3 beside M5.4 on the one layer.
    hwrite("DCF.1b.h2", at_bound(0.005));

    // DCF.1d.h1: COMP exactly on the 70% ceiling, every floor clear.  Clean.
    {
        let mut v = all_at(pdk, BASE);
        v.push(band(pdk, "comp", 70.0));
        hwrite("DCF.1d.h1", v);
    }
    // DCF.1d.h2: COMP one step over the ceiling.
    {
        let mut v = all_at(pdk, BASE);
        v.push(band(pdk, "comp", 70.005));
        hwrite("DCF.1d.h2", v);
    }

    // DCF.1b.h3: the drawn layer and the dummy layer cover the *same* 40%.  Each rule
    // sums a drawn layer and its dummy fill, and the manual's number is coverage - the
    // area the layer covers - so the overlap counts once.  Added instead of joined,
    // COMP would read 80% and DCF.1d would fire.
    {
        let mut v = all_at(pdk, BASE);
        for l in ["comp_dummy", "poly2_dummy", "metal1_dummy"] {
            v.push(band(pdk, l, BASE));
        }
        hwrite("DCF.1b.h3", v);
    }
    // DCF.1b.h4: the coverage sits entirely on the dummy layers - COMP, Poly2 and Metal1
    // are drawn nowhere and their fill carries the die.  That is what a fill run leaves
    // behind, and section 13 is the fill that makes the density.  Clean.
    {
        let mut v = vec![die(pdk)];
        for d in ["comp_dummy", "poly2_dummy", "metal1_dummy"] {
            v.push(band(pdk, d, BASE));
        }
        for l in [
            "metal2_drawn",
            "metal3_drawn",
            "metal4_drawn",
            "metal5_drawn",
        ] {
            v.push(band(pdk, l, BASE));
        }
        hwrite("DCF.1b.h4", v);
    }

    // DCF.1d.h3: the die at 40% on every layer, plus a 5000 µm² COMP block 100 µm to the
    // right of the boundary and one straddling the boundary's right edge.  The
    // percentage is taken over the die, so what lies outside the die is not in it:
    // counted whole, COMP would read over 90% and DCF.1d would fire.
    {
        let mut v = all_at(pdk, BASE);
        let comp = layer(pdk, "comp");
        v.push(rect(comp, SIZE + 100.0, 0.0, SIZE + 200.0, 50.0));
        v.push(rect(comp, SIZE - 5.0, 60.0, SIZE + 5.0, 70.0));
        hwrite("DCF.1d.h3", v);
    }

    // DCF.1d.h4: the L-shaped die of h6 again, with every layer covering the whole lower
    // half of it - 5000 µm², two thirds of a 7500 µm² die - and COMP additionally filling
    // 2400 µm² of the notch, the corner of the bounding box that is *not* die.  COMP
    // covers 66.67% of the die and the ceiling is 70%: clean.  Counted over the box,
    // with the notch in the numerator, COMP would read 74% and DCF.1d would fire.
    {
        let b = layer(pdk, "pr_bndry");
        let mut v = vec![
            rect(b, 0.0, 0.0, SIZE, SIZE / 2.0),
            rect(b, 0.0, SIZE / 2.0, SIZE / 2.0, SIZE),
        ];
        for &(l, _) in LAYERS {
            v.push(rect(layer(pdk, l), 0.0, 0.0, SIZE, SIZE / 2.0));
        }
        v.push(rect(layer(pdk, "comp"), 50.0, 50.0, 98.0, SIZE));
        hwrite("DCF.1d.h4", v);
    }

    // DCF.1b.h5: no boundary drawn at all - the layer 0/0 that many a real GDS has never
    // seen.  The die is then what the layout covers, which a 0.5 µm COMP square in the
    // far corner pins to the same 100 µm square as everywhere else here; every layer is
    // 40% of it.  Nothing fires, and nothing errors either.
    {
        let mut v: Vec<GdsElement> = LAYERS.iter().map(|&(l, _)| band(pdk, l, BASE)).collect();
        v.push(rect(layer(pdk, "comp"), SIZE - 0.5, SIZE - 0.5, SIZE, SIZE));
        hwrite("DCF.1b.h5", v);
    }

    // DCF.1b.h6: an L-shaped die - two boundary boxes meeting along an edge, 7500 µm² of
    // die in a 10 000 µm² bounding box.  Every layer covers 2600 µm² of the L: 34.67% of
    // the die, 26% of the box.  Read over the box, all six metal floors would fire.
    {
        let b = layer(pdk, "pr_bndry");
        let mut v = vec![
            rect(b, 0.0, 0.0, SIZE, SIZE / 2.0),
            rect(b, 0.0, SIZE / 2.0, SIZE / 2.0, SIZE),
        ];
        v.extend(
            LAYERS
                .iter()
                .map(|&(l, _)| rect(layer(pdk, l), 0.0, 0.0, 52.0, 50.0)),
        );
        hwrite("DCF.1b.h6", v);
    }

    // DCF.1b.h7: two dies side by side with a 10 µm street between them - 10 000 µm² of
    // boundary in an 11 000 µm² box.  Every layer covers 3000 µm²: 30% of the boundary,
    // 27.3% of the box.  Read over the box the metal floors would fire.
    {
        let b = layer(pdk, "pr_bndry");
        let mut v = vec![
            rect(b, 0.0, 0.0, 50.0, SIZE),
            rect(b, 60.0, 0.0, 110.0, SIZE),
        ];
        for &(l, _) in LAYERS {
            let ll = layer(pdk, l);
            v.push(rect(ll, 0.0, 0.0, 50.0, 30.0));
            v.push(rect(ll, 60.0, 0.0, 110.0, 30.0));
        }
        hwrite("DCF.1b.h7", v);
    }

    // DCF.1b.h9: a 400 µm die whose lower 160 µm is solid on every layer and whose upper
    // 240 µm is bare - 40% of the die, with a 200 µm square of it empty.  The rules in
    // chapter 7 and 13.1 are die-wide ("over the entire die", "global density"); the
    // 200 µm window stepped by 100 µm in section 13.3 is the recipe for *generating*
    // dummy metal, not a rule.  Clean.
    {
        let side = 400.0;
        let b = layer(pdk, "pr_bndry");
        let mut v = vec![rect(b, 0.0, 0.0, side, side)];
        v.extend(
            LAYERS
                .iter()
                .map(|&(l, _)| rect(layer(pdk, l), 0.0, 0.0, side, side * 0.4)),
        );
        hwrite("DCF.1b.h9", v);
    }

    // DCF.1b.h8: every layer at its exact bound again, but drawn as upright stripes that
    // straddle the tile lines at 20, 21, 40 and 42, on a die whose own edges sit at 3
    // and 103.  The same percentages, cut differently.
    {
        let b = layer(pdk, "pr_bndry");
        let mut v = vec![rect(b, 3.0, 3.0, 103.0, 103.0)];
        // Stripe widths summing to the rule's percentage of the 100 µm die.
        let cols = |total: f64| -> Vec<(f64, f64)> {
            let mut out = vec![(18.0, 23.0), (38.0, 43.0)];
            let mut left = total - 10.0;
            let mut x = 60.0;
            while left > 0.0 {
                let w = left.min(10.0);
                out.push((x, x + w));
                x += 15.0;
                left -= w;
            }
            out
        };
        let mut put = |lname: &str, pct: f64| {
            let ll = layer(pdk, lname);
            for (x0, x1) in cols(pct) {
                v.push(rect(ll, x0, 3.0, x1, 103.0));
            }
        };
        put("comp", 25.0);
        put("poly2_drawn", 14.0);
        for l in METALS {
            put(l, 30.0);
        }
        hwrite("DCF.1b.h8", v);
    }
}
