// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The LEF's 0.5 nm manufacturing grid, drawn on a finer 0.25 nm DBU so a
//! half-grid error survives the GDS round trip. Every drawing layer is exercised.

use super::{Corpus, DIR, library, rect, write_gz};
use gds21::GdsUnits;

pub(super) fn generate(c: &Corpus<'_>) {
    let dir = format!("{DIR}/offgrid");
    std::fs::create_dir_all(&dir).expect("pattern directory");
    for rule in c.rules.iter().filter(|r| r.check == "offgrid") {
        c.src(&rule.id, "offgrid", 0.5);
        for bad in [false, true] {
            let mut elems = Vec::new();
            for layer in &rule.layers {
                let key = c.layer(&layer.name);
                // The helpers draw integer nm; each becomes one 0.25 nm DBU.
                // Odd half-nm coordinates, including negatives, must stay clean.
                elems.push(rect(key, -0.042, -0.042, -0.002, -0.002));
                if bad {
                    // Shift in x alone, y alone, then both: four vertices each.
                    elems.push(rect(key, 0.001, 0.0, 0.041, 0.040));
                    elems.push(rect(key, 0.100, 0.001, 0.140, 0.041));
                    elems.push(rect(key, -0.141, -0.141, -0.101, -0.101));
                }
            }
            let mut lib = library("TOP", elems);
            lib.units = GdsUnits(0.00025, 0.25e-9);
            let polarity = if bad { "bad" } else { "good" };
            write_gz(&format!("{dir}/{}.grid.{polarity}.gds.gz", rule.id), lib);
        }
    }
}
