// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::nmosi::{P, ring4};
use super::{OFFSET, SPACE_DELTA};
use crate::helpers::{diamond, enclosure_pattern, layer, library, rect, text, write_gz};
use gdscheck::pdk::PdkConfig;

const DIR: &str = "tests/data/ihp-sg13g2/pin";

pub fn generate(pdk: &PdkConfig) {
    std::fs::create_dir_all(DIR).expect("failed to create output directory");

    pin_a(pdk);
    pin_b(pdk);
    pin_e(pdk);
    for index in 2..6 {
        pin_f(pdk, index);
    }
    pin_g(pdk);
    pin_h(pdk);

    hardening(pdk);
}

fn pin_a(pdk: &PdkConfig) {
    let main = layer(pdk, "Activ");
    let pin = layer(pdk, "Activ.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.a.gds.gz"), library("TOP", elems));
}

fn pin_b(pdk: &PdkConfig) {
    let main = layer(pdk, "GatPoly");
    let pin = layer(pdk, "GatPoly.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.b.gds.gz"), library("TOP", elems));
}

fn pin_e(pdk: &PdkConfig) {
    let main = layer(pdk, "Metal1");
    let pin = layer(pdk, "Metal1.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.e.gds.gz"), library("TOP", elems));
}

fn pin_f(pdk: &PdkConfig, index: i32) {
    let main = layer(pdk, &format!("Metal{index}"));
    let pin = layer(pdk, &format!("Metal{index}.pin"));
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(
        &format!("{DIR}/Pin.f.m{index}.gds.gz"),
        library("TOP", elems),
    );
}

fn pin_g(pdk: &PdkConfig) {
    let main = layer(pdk, "TopMetal1");
    let pin = layer(pdk, "TopMetal1.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.g.gds.gz"), library("TOP", elems));
}

fn pin_h(pdk: &PdkConfig) {
    let main = layer(pdk, "TopMetal2");
    let pin = layer(pdk, "TopMetal2.pin");
    let elems = enclosure_pattern(main, pin, 0.0, 0.5, 5.0, OFFSET, SPACE_DELTA);
    write_gz(&format!("{DIR}/Pin.h.gds.gz"), library("TOP", elems));
}

// --- Hardening (hardening/SPEC.md) -------------------------------------------
//
// Hardening layouts for the last three decks of the SG13G2 set: section 6.5 (nmosi and
// nmosiHV, nmosi.b-nmosi.g, with section 4.2's Iso-PWell-Activ), section 7.4 (Pin.a-Pin.h)
// and section 7.1 (Ant.a-Ant.i, the net-aware antenna ratios).  Every layout is
// `tests/data/ihp-sg13g2/<deck>/<RULE>.h<k>.gds.gz`.
//
// The nmosi layouts share one structure, `iso`: an Activ in the hole of a closed NWell
// ring, the whole on nBuLay, so that the Activ is Iso-PWell-Activ (Activ AND nBuLay AND
// PWell) and the ring is what the manual tests the rules inside of.  The antenna
// layouts share one gate, `gate`: a 0.2 µm GatPoly strip over a 1.0 × 0.5 Activ (gate
// area 0.1 µm², 0.12 µm² of poly over field) with a 0.1 × 0.1 Cont on the poly; the
// areas of the antennas are what the boxes draw, so every ratio is computed in the
// comment next to it.

const PIN: &str = "tests/data/ihp-sg13g2/pin";

/// Pin.a-Pin.h - "Min. <layer> enclosure of <layer>:pin  0.00": "Pin areas must be
/// fully covered by drawing."
fn pin(p: &P) {
    let pin1 = (p.m1.0, 2);
    // h1, the bound on Metal1: the pin coincident with the metal on all four sides
    // (clean); the pin 0.005 past the metal on the right (fires); on top (fires); the pin
    // 0.5 clear of the metal (fires); the pin 0.1 inside the metal (clean); a 0.005 ×
    // 0.005 pin outside the metal (fires).
    let e = vec![
        rect(p.m1, 2.0, 2.0, 4.0, 3.0),
        rect(pin1, 2.0, 2.0, 4.0, 3.0),
        rect(p.m1, 6.0, 2.0, 8.0, 3.0),
        rect(pin1, 6.0, 2.0, 8.005, 3.0),
        rect(p.m1, 10.0, 2.0, 12.0, 3.0),
        rect(pin1, 10.0, 2.0, 12.0, 3.005),
        rect(p.m1, 14.0, 2.0, 16.0, 3.0),
        rect(pin1, 16.5, 2.0, 18.0, 3.0),
        rect(p.m1, 2.0, 6.0, 4.0, 7.0),
        rect(pin1, 2.1, 6.1, 3.9, 6.9),
        rect(p.m1, 6.0, 6.0, 8.0, 7.0),
        rect(pin1, 9.0, 6.0, 9.005, 6.005),
    ];
    write_gz(&format!("{PIN}/Pin.e.h1.gds.gz"), library("TOP", e));

    // h2, the drawing: a pin over two abutting metal boxes (clean); over two overlapping
    // ones (clean); the pin as four tiles over one metal (clean); a pin across a metal
    // ring's hole (fires on the hole); a pin over the ring's leg reaching 0.2 into the
    // hole (fires); a diamond pin inscribed in a square metal (clean); a square pin whose
    // corners poke 0.005 out of a diamond metal (fires four times); a diamond metal with a
    // diamond pin 0.005 larger (fires four times).
    let mut e = vec![
        rect(p.m1, 2.0, 2.0, 3.0, 3.0),
        rect(p.m1, 3.0, 2.0, 4.0, 3.0),
        rect(pin1, 2.2, 2.2, 3.8, 2.8),
        rect(p.m1, 6.0, 2.0, 7.2, 3.0),
        rect(p.m1, 6.8, 2.0, 8.0, 3.0),
        rect(pin1, 6.2, 2.2, 7.8, 2.8),
        rect(p.m1, 10.0, 2.0, 12.0, 3.0),
        rect(pin1, 10.0, 2.0, 11.0, 2.5),
        rect(pin1, 11.0, 2.0, 12.0, 2.5),
        rect(pin1, 10.0, 2.5, 11.0, 3.0),
        rect(pin1, 11.0, 2.5, 12.0, 3.0),
    ];
    e.extend(ring4(p.m1, (14.0, 2.0, 18.0, 6.0), (15.0, 3.0, 17.0, 5.0)));
    e.push(rect(pin1, 14.0, 3.5, 18.0, 4.5));
    e.extend(ring4(p.m1, (2.0, 6.0, 6.0, 10.0), (3.0, 7.0, 5.0, 9.0)));
    e.push(rect(pin1, 5.0, 7.5, 6.0, 8.5));
    e.push(rect(pin1, 4.8, 7.5, 5.2, 8.5));
    e.push(rect(p.m1, 8.0, 6.0, 12.0, 10.0));
    e.push(diamond(pin1, 10.0, 8.0, 2.0));
    e.push(diamond(p.m1, 16.0, 8.0, 2.0));
    e.push(rect(pin1, 15.0, 7.0, 17.005, 9.005));
    e.push(diamond(p.m1, 4.0, 14.0, 2.0));
    e.push(diamond(pin1, 4.0, 14.0, 2.005));
    write_gz(&format!("{PIN}/Pin.e.h2.gds.gz"), library("TOP", e));

    // h3, the tile lines: pin and metal both ending on x = 20 (clean); the metal ending
    // at 19.995 under a pin ending on x = 20 (fires); a pin across x = 40 under a metal
    // across it (clean); a pin from 41 to 43 over a metal ending at 41.995 (fires); a
    // 300 µm pin on a 300 µm metal across every line (clean); an uncovered pin at
    // (1000, 1000) (fires); a text on Metal1:pin and one on Metal1:label with no metal
    // (texts are not areas; clean).
    let e = vec![
        rect(p.m1, 18.0, 2.0, 20.0, 3.0),
        rect(pin1, 18.0, 2.0, 20.0, 3.0),
        rect(p.m1, 18.0, 6.0, 19.995, 7.0),
        rect(pin1, 18.0, 6.0, 20.0, 7.0),
        rect(p.m1, 38.0, 2.0, 42.0, 3.0),
        rect(pin1, 39.0, 2.0, 41.0, 3.0),
        rect(p.m1, 40.0, 6.0, 41.995, 7.0),
        rect(pin1, 41.0, 6.0, 43.0, 7.0),
        rect(p.m1, 5.0, 12.0, 305.0, 13.0),
        rect(pin1, 5.0, 12.0, 305.0, 13.0),
        rect(pin1, 1000.0, 1000.0, 1001.0, 1001.0),
        text(pin1, "A", 10.0, 16.0),
        text(p.m1_label, "B", 12.0, 16.0),
    ];
    write_gz(&format!("{PIN}/Pin.e.h3.gds.gz"), library("TOP", e));

    // all.h1, every layer of the table: a pin 0.005 past its drawing (fires on Pin.a,
    // Pin.b, Pin.e, Pin.f four times, Pin.g, Pin.h) and a covered pin (clean) on each; a
    // Metal1:pin over Metal2 only (fires, Pin.e); pins on NWell:pin and Passiv:pin with
    // no drawing (no rule; clean).
    let mut e = vec![];
    let layers = [p.activ, p.gp, p.m1, p.m2, p.m3, p.m4, p.m5, p.tm1, p.tm2];
    for (i, l) in layers.iter().enumerate() {
        let y = 2.0 + 3.0 * i as f64;
        e.push(rect(*l, 2.0, y, 4.0, y + 1.0));
        e.push(rect((l.0, 2), 2.0, y, 4.005, y + 1.0));
        e.push(rect(*l, 6.0, y, 8.0, y + 1.0));
        e.push(rect((l.0, 2), 6.1, y + 0.1, 7.9, y + 0.9));
    }
    e.push(rect(p.m2, 10.0, 2.0, 12.0, 3.0));
    e.push(rect(pin1, 10.0, 2.0, 12.0, 3.0));
    e.push(rect((p.nw.0, 2), 10.0, 5.0, 12.0, 6.0));
    e.push(rect((9, 2), 10.0, 8.0, 12.0, 9.0));
    write_gz(&format!("{PIN}/Pin.all.h1.gds.gz"), library("TOP", e));
}
fn hardening(pdk: &PdkConfig) {
    std::fs::create_dir_all("tests/data/ihp-sg13g2/pin")
        .expect("failed to create output directory");
    let p = P::new(pdk);
    pin(&p);
}
