// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generated test patterns for GF180MCU variant D.
//!
//! Every fixture here comes in two halves. The **good** one is drawn to be legal and must
//! produce nothing; the **bad** one contains a known number of real violations and must
//! produce exactly that. A check that only ever ran against foundry test cases can pass
//! for the wrong reason — the two engines disagree on how many markers one violation is
//! worth, so a count that is close enough looks like agreement. A pattern we drew
//! ourselves has no such ambiguity: the good half is a hard zero.

mod acute;
mod antenna;
mod bjt;
mod comp;
mod contact;
mod cup;
mod density;
mod dnwell;
mod dualgate;
mod dummy_exclude;
mod dummy_fill;
mod efuse;
mod esd;
mod guard_ring;
mod hres;
mod lres;
mod lvpwell;
mod mcell;
mod metal;
mod metaltop;
mod mim_b;
mod nat;
mod nplus;
mod nwell;
mod offgrid;
mod otp_mk;
mod poly2;
mod pplus;
mod pres;
mod sab;
mod sram_3p3;
mod sram_5p0;
mod via;
mod ymtp_mk;

use gdscheck::pdk::PdkConfig;

/// Shapes are placed away from the origin so a sign error shows up as a wrong answer
/// rather than as a shape that happens to straddle (0, 0).
pub const OFFSET: f64 = 10.0;

/// The manufacturing grid, and the value both decks check against.
pub const GRID: f64 = 0.005;

pub fn generate(pdk: &PdkConfig) {
    mim_b::generate(pdk);
    nat::generate(pdk);
    offgrid::generate(pdk);
    nplus::generate(pdk);
    nwell::generate(pdk);
    sab::generate(pdk);
    otp_mk::generate(pdk);
    acute::generate(pdk);
    antenna::generate(pdk);
    bjt::generate(pdk);
    comp::generate(pdk);
    contact::generate(pdk);
    cup::generate(pdk);
    density::generate(pdk);
    dnwell::generate(pdk);
    dualgate::generate(pdk);
    efuse::generate(pdk);
    esd::generate(pdk);
    guard_ring::generate(pdk);
    hres::generate(pdk);
    lres::generate(pdk);
    dummy_fill::generate(pdk);
    dummy_exclude::generate(pdk);
    metal::generate(pdk);
    lvpwell::generate(pdk);
    mcell::generate(pdk);
    metaltop::generate(pdk);
    poly2::generate(pdk);
    pplus::generate(pdk);
    pres::generate(pdk);
    sram_3p3::generate(pdk);
    sram_5p0::generate(pdk);
    via::generate(pdk);
    ymtp_mk::generate(pdk);
}

/// An axis-aligned square with its corners cut at 45°, every vertex on the grid.
///
/// Legal under both decks and deliberately so: the off-grid check must not object to a
/// diagonal, and the acute check must not object to a 45° one. A plain rectangle would
/// test neither.
pub fn octagon(layer: (i16, i16), x0: f64, y0: f64, side: f64, cut: f64) -> gds21::GdsElement {
    let (x1, y1) = (x0 + side, y0 + side);
    crate::helpers::poly(
        layer,
        &[
            (x0 + cut, y0),
            (x1 - cut, y0),
            (x1, y0 + cut),
            (x1, y1 - cut),
            (x1 - cut, y1),
            (x0 + cut, y1),
            (x0, y1 - cut),
            (x0, y0 + cut),
        ],
    )
}

/// Every rule in `deck`, as (rule id, primary layer key).
///
/// Read from the deck rather than from a list kept here, so a layer added to the deck
/// gets a fixture without anyone remembering to add one — the failure mode a hand-kept
/// list invites is a rule with no pattern behind it.
pub fn rules_of(pdk: &PdkConfig, deck: &str) -> Vec<(String, (i16, i16))> {
    pdk.load_deck(deck)
        .unwrap_or_else(|e| panic!("failed to load deck '{deck}': {e}"))
        .iter()
        .map(|r| {
            let l = &r.layers[0];
            (r.id.clone(), (l.gds_layer as i16, l.gds_datatype as i16))
        })
        .collect()
}
