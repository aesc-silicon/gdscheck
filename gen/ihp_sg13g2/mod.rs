// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod activ;
mod antenna;
mod connectivity;
mod cont;
mod contbar;
mod extblock;
mod forbidden;
mod gatpoly;
mod lbe;
mod lu;
mod metal;
mod mim;
mod nbulay;
mod nbulayblock;
mod nmosi;
mod npn;
mod nsdblock;
mod nwell;
mod offgrid;
mod pad;
mod passiv;
mod pin;
mod psd;
mod pwellblock;
mod resistor;
mod salblock;
mod sdiod;
mod sealring;
mod slit;
mod tgo;
mod topmetal;
mod topvia;
mod via;

use gdscheck::pdk::PdkConfig;

/// Default offset used to position patterns so neighbours don't overlap.
pub(super) const OFFSET: f64 = 20.0;
/// Default gap overshoot for the violating neighbours in space patterns.
pub(super) const SPACE_DELTA: f64 = -0.005;

pub fn generate(pdk: &PdkConfig) {
    activ::generate(pdk);
    offgrid::generate(pdk);
    tgo::generate(pdk);
    gatpoly::generate(pdk);
    cont::generate(pdk);
    metal::generate(pdk);
    via::generate(pdk);
    topvia::generate(pdk);
    topmetal::generate(pdk);
    passiv::generate(pdk);
    pin::generate(pdk);
    lbe::generate(pdk);
    pad::generate(pdk);
    contbar::generate(pdk);
    salblock::generate(pdk);
    nsdblock::generate(pdk);
    psd::generate(pdk);
    resistor::generate(pdk);
    nmosi::generate(pdk);
    npn::generate(pdk);
    sdiod::generate(pdk);
    sealring::generate(pdk);
    nwell::generate(pdk);
    pwellblock::generate(pdk);
    nbulay::generate(pdk);
    nbulayblock::generate(pdk);
    extblock::generate(pdk);
    slit::generate(pdk);
    lu::generate(pdk);
    antenna::generate(pdk);
    mim::generate(pdk);
    connectivity::generate(pdk);
    forbidden::generate(pdk);
}
