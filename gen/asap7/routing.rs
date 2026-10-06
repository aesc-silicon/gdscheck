// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{Corpus, DBU};

pub(super) fn generate(c: &Corpus<'_>) {
    // Three routed nets exercise all upper metals M4-M9 and V4/V6/V8.
    // The companions shorten one M9 end to give V8 exactly 20 nm, then only 19.75 nm.
    // V8 spans y = -20..20, so its top enclosure is m9_top - 20.
    let v8_en = c.drm("V8.M9.EN.2", "min_enclosure", 20.0);
    for (name, m9_top, bad) in [
        ("routed", 100.0, false),
        ("routed_exact", 20.0 + v8_en, false),
        ("routed", 20.0 + v8_en - DBU, true),
    ] {
        c.layout(
            "v8",
            &format!("V8.M9.EN.2.{name}"),
            bad,
            &[
                ("M4", [-100.0, -12.0, 100.0, 12.0]),
                ("M5", [-12.0, -100.0, 12.0, 100.0]),
                ("V4", [-12.0, -12.0, 12.0, 12.0]),
                ("M6", [900.0, -16.0, 1100.0, 16.0]),
                ("M7", [984.0, -100.0, 1016.0, 100.0]),
                ("V6", [984.0, -16.0, 1016.0, 16.0]),
                ("M8", [1900.0, -30.0, 2100.0, 30.0]),
                ("M9", [1970.0, -100.0, 2030.0, m9_top]),
                ("V8", [1980.0, -20.0, 2020.0, 20.0]),
            ],
            19990.0,
        );
    }
}
