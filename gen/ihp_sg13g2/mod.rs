// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod activ;
mod offgrid;
mod tgo;
mod gatpoly;
mod cont;
mod metal;
mod via;
mod topvia;
mod topmetal;
mod passiv;
mod pin;
mod lbe;
mod pad;
mod contbar;
mod salblock;
mod nsdblock;
mod psd;
mod resistor;
mod nmosi;
mod npn;
mod sdiod;
mod sealring;
mod nwell;
mod pwellblock;
mod nbulay;
mod nbulayblock;
mod extblock;
mod slit;
mod lu;
mod antenna;
mod mim;
mod connectivity;
mod forbidden;

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
