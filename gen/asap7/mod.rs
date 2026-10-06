// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generated ASAP7 rule patterns, in nm on the 0.25 nm grid.
//!
//! Each family writes good/bad GDS fixtures under `tests/data/asap7/generated/<deck>/`.
//! Expectations live in `tests/asap7.rs`. Limits come from
//! asap7_drm_201207a.pdf; the generator checks the decks against those fixed limits.

mod active;
mod fin;
mod gate;
mod gcut;
mod geometry;
mod lig;
mod lisd;
mod metal;
mod patterns;
mod routing;
mod sdt;
mod select;
mod via;
mod well;

use crate::helpers::{library, rect, write_gz};
use gds21::{GdsElement, GdsUnits};
use gdscheck::pdk::{Param, PdkConfig, RuleDefinition};

const DIR: &str = "tests/data/asap7/generated";
const DBU: f64 = 0.25;
type Shape<'a> = (&'a str, [f64; 4]);

/// `v` nm in the units of the shared helpers, which draw in µm on a 1 nm DBU. ASAP7
/// needs a 0.25 nm DBU, so each helper nanometre stands for one ASAP7 DBU here, and
/// [`Corpus::write`] stamps the 0.25 nm units on the result.
fn nm(v: f64) -> f64 {
    v * 4.0 / 1000.0
}

/// One DBU under `limit`, on it, and one over.
fn around(limit: f64) -> [f64; 3] {
    [limit - DBU, limit, limit + DBU]
}

struct Corpus<'a> {
    pdk: &'a PdkConfig,
    rules: Vec<RuleDefinition>,
}

impl Corpus<'_> {
    /// The manual's `nm` for rule `id`, after checking every `check` entry of it in the
    /// deck says the same. An id can carry several checks (min_width plus forbidden).
    fn drm(&self, id: &str, check: &str, nm: f64) -> f64 {
        self.agree(id, check, "value", nm, |r| Some(r.value))
    }

    /// The manual's `nm` for the exact length the edge layer `name` keeps or drops,
    /// after checking the deck's layer says the same.
    fn drm_edge_length(&self, name: &str, nm: f64) -> f64 {
        let def = self
            .pdk
            .edge_layers
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no edge layer {name}"));
        let um = def.min.unwrap_or_else(|| panic!("{name} has no length"));
        assert!(
            (um * 1000.0 - nm).abs() < 1e-6 && def.max == def.min,
            "{name}: deck length is {} nm, the DRM says {nm} nm",
            um * 1000.0
        );
        nm
    }

    /// As [`Self::drm`], for an area in nm² (the deck writes µm²).
    fn drm_area(&self, id: &str, check: &str, nm2: f64) -> f64 {
        self.agree(id, check, "value", nm2, |r| Some(r.value * 1000.0))
    }

    /// As [`Self::drm`], for the numeric param `key`.
    fn drm_param(&self, id: &str, check: &str, key: &str, nm: f64) -> f64 {
        self.agree(id, check, key, nm, |r| match r.params.get(key) {
            Some(Param::Num(v)) => Some(*v),
            _ => None,
        })
    }

    fn agree(
        &self,
        id: &str,
        check: &str,
        what: &str,
        nm: f64,
        read: impl Fn(&RuleDefinition) -> Option<f64>,
    ) -> f64 {
        let found: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.id == id && r.check == check)
            .collect();
        assert!(!found.is_empty(), "{id} has no {check} entry in any deck");
        for r in found {
            let um = read(r).unwrap_or_else(|| panic!("{id} {check} has no numeric {what}"));
            assert!(
                (um * 1000.0 - nm).abs() < 1e-6,
                "{id} {check}: deck {what} is {} nm, the DRM says {nm} nm",
                um * 1000.0
            );
        }
        nm
    }

    fn layer(&self, name: &str) -> (i16, i16) {
        crate::helpers::layer(self.pdk, name)
    }

    /// One rectangle `[x0, y0, x1, y1]` nm of `layer`, shifted by `origin` nm.
    fn rect(&self, layer: &str, [x0, y0, x1, y1]: [f64; 4], origin: f64) -> GdsElement {
        let at = |v: f64| nm(origin + v);
        rect(self.layer(layer), at(x0), at(y0), at(x1), at(y1))
    }

    fn layout(&self, deck: &str, name: &str, bad: bool, shapes: &[Shape<'_>], origin: f64) {
        let elems = shapes
            .iter()
            .map(|&(layer, r)| self.rect(layer, r, origin))
            .collect();
        self.write(deck, name, bad, elems);
    }

    /// Write `elems`, drawn in [`nm`] units, as `<deck>/<name>.<good|bad>.gds.gz`.
    fn write(&self, deck: &str, name: &str, bad: bool, elems: Vec<GdsElement>) {
        let mut lib = library("TOP", elems);
        lib.units = GdsUnits(0.00025, 0.25e-9);
        let dir = format!("{DIR}/{deck}");
        std::fs::create_dir_all(&dir).expect("pattern directory");
        let polarity = if bad { "bad" } else { "good" };
        write_gz(&format!("{dir}/{name}.{polarity}.gds.gz"), lib);
    }
}

pub fn generate(pdk: &PdkConfig) {
    let rules = pdk
        .decks
        .iter()
        .flat_map(|d| {
            pdk.load_deck(&d.name)
                .unwrap_or_else(|e| panic!("failed to load deck '{}': {e}", d.name))
        })
        .collect();
    let c = Corpus { pdk, rules };
    well::generate(&c);
    fin::generate(&c);
    gate::generate(&c);
    gcut::generate(&c);
    active::generate(&c);
    sdt::generate(&c);
    lisd::generate(&c);
    lig::generate(&c);
    select::generate(&c);
    geometry::generate(&c);
    via::generate(&c);
    metal::generate(&c);
    routing::generate(&c);
}
