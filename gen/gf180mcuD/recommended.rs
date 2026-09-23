// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardening layouts for the three guidance decks the via guideline's round found
//! missing: `contact_recommended` (CO.6 III), `comp_recommended` (DF.1b) and
//! `poly2_recommended` (PL.3b).  All three are Appendix B's - the manual documents them
//! and does not check them, and neither does the foundry's runset - so the oracle has no
//! opinion on these layouts and the manual's sentence is the whole argument.
//!
//! Each fixture is the rule's own value and the grid step past it, with the shape that
//! the *mandatory* twin of the rule would pass drawn beside it: DF.1b's 0.3 against
//! DF.1a's 0.22, PL.3b's 0.26 against PL.3a's 0.24, CO.6 III's 0.12 against CO.6's 0.005.
//! That is what a guideline is for - the geometry is legal and the parameter is not what
//! it could be - and it is why these decks stay out of `main` and `core`.

use super::OFFSET;
use crate::helpers::{layer, library, rect, write_gz};
use gds21::GdsElement;
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/gf180mcuD/generated/recommended";
/// The manufacturing grid.
const D: f64 = 0.005;

fn hwrite(name: &str, elems: Vec<GdsElement>) {
    write_gz(&format!("{DIR}/{name}.gds.gz"), library("TOP", elems));
}

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("pattern dir");
    let (comp, poly, m1, cont) = (
        layer(pdk, "comp"),
        layer(pdk, "poly2_drawn"),
        layer(pdk, "metal1_drawn"),
        layer(pdk, "contact"),
    );
    let (res_mk, dualgate) = (layer(pdk, "res_mk"), layer(pdk, "dualgate"));
    let o = OFFSET;

    // CO.6iii: Metal1 over a 0.22 µm contact by 0.12 on every side, and by 0.115.  The
    // third contact keeps CO.6's own 0.06, which the guideline is silent about being
    // legal: it is a contact drawn to the mandatory rule and no further.
    hwrite("CO.6iii.h1", {
        let cs = 0.22;
        let mut v = vec![];
        for (i, e) in [0.12, 0.115, 0.06].into_iter().enumerate() {
            let x = o + i as f64 * 5.0;
            v.push(rect(cont, x, o, x + cs, o + cs));
            v.push(rect(m1, x - e, o - e, x + cs + e, o + cs + e));
        }
        v
    });

    // DF.1b: an active under RES_MK 0.3 µm wide and one 0.295 - and, beside them, a
    // 0.295 µm active with no marking, which is a wire and not a resistor.
    hwrite("DF.1b.h1", {
        let mut v = vec![];
        for (i, (w, marked)) in [(0.3, true), (0.295, true), (0.295, false)]
            .into_iter()
            .enumerate()
        {
            let x = o + i as f64 * 5.0;
            v.push(rect(comp, x, o, x + 4.0, o + w));
            if marked {
                v.push(rect(res_mk, x - 0.2, o - 0.2, x + 4.2, o + w + 0.2));
            }
        }
        v
    });

    // PL.3b: two gates over one active 0.26 µm apart and 0.255, at 3.3 V; the same pair
    // at 0.4 and 0.395 under Dualgate; and a 0.255 gap between two polys *off* the
    // active, which is PL.3a's and not this rule's.
    hwrite("PL.3b.h1", {
        let gate = |v: &mut Vec<GdsElement>, x: f64, y: f64, g: f64, on_comp: bool| {
            if on_comp {
                v.push(rect(comp, x - 0.5, y - 0.3, x + 1.5 + g + 0.5, y + 1.3));
            }
            v.push(rect(poly, x, y, x + 0.5, y + 1.0));
            v.push(rect(poly, x + 0.5 + g, y, x + 1.0 + g, y + 1.0));
        };
        let mut v = vec![];
        gate(&mut v, o, o, 0.26, true);
        gate(&mut v, o + 6.0, o, 0.26 - D, true);
        gate(&mut v, o, o + 6.0, 0.4, true);
        gate(&mut v, o + 6.0, o + 6.0, 0.4 - D, true);
        gate(&mut v, o + 12.0, o, 0.26 - D, false);
        // The 5 V pair is the one Dualgate covers; it reaches 0.5 past the active.
        v.push(rect(dualgate, o - 1.5, o + 4.5, o + 10.0, o + 8.0));
        v
    });
}
