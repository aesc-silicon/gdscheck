// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generated rule patterns for Georgia Tech's two PDKs, GT3 and GT2N, in nm on the
//! 0.5 nm grid of their libraries.
//!
//! GT2N extends GT3, and its runset is GT3's with seven values changed, so one generator
//! draws both: each pattern takes those seven from the PDK's [`Process`] and every
//! other value, the same in both runsets, from where it is drawn. Both PDKs' decks add
//! to the runset - GATE.2's pitch, V0.7a, the tech LEF's rules above M6, its cut
//! spacings below and its manufacturing grid - and the two LEFs differ only in VG's and
//! VSD's cut spacings, also in [`Process`]. Only GT2N draws the
//! threshold-voltage layers, the backside and the technology file's rules.
//!
//! Each family writes good/bad GDS fixtures under
//! `tests/data/<process>/generated/<deck>/`. Expectations live in `tests/gt3.rs` and
//! `tests/gt2n.rs`. Limits come from each PDK's IC Validator runset
//! (`icv_runset/Include/gt3.*.drc.rs`, `gt2.*.drc.rs`), and above M6, where the runset
//! stops, from its technology LEF (`techlef/gt3_tech.lef`, `techlib/gt2_tech.lef`), with
//! GT2N's additional well and area limits from `techlib/gt2_techfile.tf`; the generator
//! checks the decks against these sources, so a value changed in a deck - either PDK's
//! own, or one GT2N shares with GT3 - stops it.

mod beol;
mod feol;
mod mol;
mod offgrid;
mod techfile;

use crate::helpers::{library, poly, rect, write_gz};
use gds21::{GdsElement, GdsUnits};
use gdscheck::pdk::{PdkConfig, RuleDefinition};

const DBU: f64 = 0.5;

/// One of the two PDKs: its name, whether it is GT2N, the seven values in which GT2N's
/// runset differs from GT3's, each as its own runset states it, and the two cut
/// spacings in which the two tech LEFs differ.
pub struct Process {
    /// The process, as `pdks/<name>` and `tests/data/<name>` spell it.
    name: &'static str,
    /// GT2N has what GT3 has not: the threshold-voltage layers, two active sheets, the
    /// backside stack and the technology file's rules.
    gt2n: bool,
    /// ACT.1, an active's least height.
    act_1: f64,
    /// DUMMY.1, a diffusion-break marker's exact width.
    dummy_1: f64,
    /// GATE.1, a gate's exact width.
    gate_1: f64,
    /// GATE.2, the space between gates.
    gate_2: f64,
    /// SDCON.1, a source/drain contact's exact width.
    sdcon_1: f64,
    /// VBPR.2, a rail via's exact width across x.
    vbpr_2: f64,
    /// VG.1, a gate contact's exact width.
    vg_1: f64,
    /// VG.6.LEF, the LEF's VG cut spacing.
    vg_6_lef: f64,
    /// VSD.6.LEF, the LEF's VSD cut spacing.
    vsd_6_lef: f64,
}

/// GT3, from `icv_runset/Include/gt3.*.drc.rs` and `techlef/gt3_tech.lef` at
/// azadnaeemi/GT3 fff4a84.
pub const GT3: Process = Process {
    name: "gt3",
    gt2n: false,
    act_1: 10.0,
    dummy_1: 15.0,
    gate_1: 15.0,
    gate_2: 27.0,
    sdcon_1: 15.0,
    vbpr_2: 15.0,
    vg_1: 15.0,
    vg_6_lef: 12.0,
    vsd_6_lef: 12.0,
};

/// GT2N, from `icv_runset/Include/gt2.*.drc.rs` and `techlib/gt2_tech.lef` at
/// azadnaeemi/GT2N 5bc689f.
pub const GT2N: Process = Process {
    name: "gt2n",
    gt2n: true,
    act_1: 8.0,
    dummy_1: 14.0,
    gate_1: 14.0,
    gate_2: 28.0,
    sdcon_1: 16.0,
    vbpr_2: 16.0,
    vg_1: 14.0,
    vg_6_lef: 28.0,
    vsd_6_lef: 29.0,
};

/// `v` nm in the units of the shared helpers, which draw in µm on a 1 nm DBU. GT3 and
/// GT2N draw on a 0.5 nm DBU, so each helper nanometre stands for one DBU here, and
/// [`Corpus::write`] stamps the 0.5 nm units on the result.
fn nm(v: f64) -> f64 {
    v * 2.0 / 1000.0
}

/// The smallest `dy` on the grid that puts the corners of two shapes `dx` apart in x
/// at least `limit` apart, the pair for a corner-to-corner rule: `dy` reads clean and
/// `dy` less a DBU does not. The grid holds 15 and 45 nm diagonals exactly (9 by 12, 27
/// by 36); for 16, 18, 24 and 27 nm the clean corner is the nearest one past the limit.
fn corner_rise(limit: f64, dx: f64) -> f64 {
    let mut dy = DBU;
    while dx * dx + dy * dy < limit * limit {
        dy += DBU;
    }
    assert!(dx * dx + (dy - DBU) * (dy - DBU) < limit * limit);
    dy
}

/// Room between the rows of a pattern: past every spacing any of its shapes answers
/// to, but BPR.3's 10 µm along a rail.
const ROOM: f64 = 500.0;

/// A `w`×`h` box with its lower-left corner at (x, y), in nm.
fn bx(layer: (i16, i16), x: f64, y: f64, w: f64, h: f64) -> GdsElement {
    rect(layer, nm(x), nm(y), nm(x + w), nm(y + h))
}

/// A polygon through `pts`, in nm, closed for you.
fn pg(layer: (i16, i16), pts: &[(f64, f64)]) -> GdsElement {
    let pts: Vec<_> = pts.iter().map(|&(x, y)| (nm(x), nm(y))).collect();
    poly(layer, &pts)
}

/// Rows of a pattern, stacked upward from `y` with [`ROOM`] between them, or more for
/// a layer whose spacing is wider.
struct Rows {
    y: f64,
    room: f64,
}

impl Rows {
    fn new(y: f64) -> Self {
        Rows::spaced(y, ROOM)
    }

    fn spaced(y: f64, room: f64) -> Self {
        Rows { y, room }
    }

    /// The bottom of a row `h` tall, the next row starting above it.
    fn next(&mut self, h: f64) -> f64 {
        let y = self.y;
        self.y += h + self.room;
        y
    }
}

struct Corpus<'a> {
    pdk: &'a PdkConfig,
    process: &'a Process,
    rules: Vec<RuleDefinition>,
}

impl Corpus<'_> {
    /// Where the fixtures go: `tests/data/<process>/generated`.
    fn dir(&self) -> String {
        format!("tests/data/{}/generated", self.process.name)
    }

    /// GATE.1's gate width, and the pitch it makes with GATE.2's space, which the
    /// libraries draw their gates on: 42 nm in both PDKs.
    fn gate(&self) -> (f64, f64) {
        let w = self.src("GATE.1", "exact_width", self.process.gate_1);
        let s = self.src("GATE.2", "min_space", self.process.gate_2);
        (w, w + s)
    }

    /// The source's `nm` for rule `id` - the runset's, or for a `.LEF` rule the tech
    /// LEF's - after checking every `check` entry of it in the deck says the same.
    fn src(&self, id: &str, check: &str, nm: f64) -> f64 {
        let found: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.id == id && r.check == check)
            .collect();
        assert!(!found.is_empty(), "{id} has no {check} entry in any deck");
        for r in found {
            assert!(
                (r.value * 1000.0 - nm).abs() < 1e-6,
                "{id} {check}: deck value is {} nm, the source says {nm} nm",
                r.value * 1000.0
            );
        }
        nm
    }

    /// A minimum area from the technology file, in nm² (deck values are µm²).
    fn src_area(&self, id: &str, nm2: f64) -> f64 {
        let rule = self
            .rules
            .iter()
            .find(|r| r.id == id && r.check == "min_area")
            .unwrap_or_else(|| panic!("{id} has no min_area entry"));
        assert!(
            (rule.value * 1e6 - nm2).abs() < 1e-6,
            "{id}: deck value is {} nm², the technology file says {nm2} nm²",
            rule.value * 1e6
        );
        nm2
    }

    /// As [`Self::src`], for the numeric param `key`.
    fn src_param(&self, id: &str, check: &str, key: &str, nm: f64) -> f64 {
        let found: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.id == id && r.check == check)
            .collect();
        assert!(!found.is_empty(), "{id} has no {check} entry in any deck");
        for r in found {
            let um = r
                .num(key)
                .unwrap_or_else(|| panic!("{id} {check} has no numeric {key}"));
            assert!(
                (um * 1000.0 - nm).abs() < 1e-6,
                "{id} {check}: deck {key} is {} nm, the runset says {nm} nm",
                um * 1000.0
            );
        }
        nm
    }

    /// The radius in nm of the sized virtual layer `name`, one step of a sentence.
    fn radius(&self, name: &str) -> f64 {
        let def = self
            .pdk
            .virtual_layers
            .iter()
            .find(|v| v.name == name)
            .unwrap_or_else(|| panic!("no virtual layer {name}"));
        def.radius.unwrap_or_else(|| panic!("{name} has no radius")) * 1000.0
    }

    fn layer(&self, name: &str) -> (i16, i16) {
        crate::helpers::layer(self.pdk, name)
    }

    /// Write `elems`, drawn in [`nm`] units, as `<deck>/<name>.<good|bad>.gds.gz`.
    fn write(&self, deck: &str, name: &str, bad: bool, elems: Vec<GdsElement>) {
        let mut lib = library("TOP", elems);
        lib.units = GdsUnits(0.0005, 0.5e-9);
        let dir = format!("{}/{deck}", self.dir());
        std::fs::create_dir_all(&dir).expect("pattern directory");
        let polarity = if bad { "bad" } else { "good" };
        write_gz(&format!("{dir}/{name}.{polarity}.gds.gz"), lib);
    }
}

pub fn generate(pdk: &PdkConfig, process: &Process) {
    assert_eq!(
        pdk.name.to_lowercase(),
        process.name,
        "the PDK is not {}",
        process.name
    );
    let rules = pdk
        .decks
        .iter()
        .flat_map(|d| {
            pdk.load_deck(&d.name)
                .unwrap_or_else(|e| panic!("failed to load deck '{}': {e}", d.name))
        })
        .collect();
    let c = Corpus {
        pdk,
        process,
        rules,
    };
    if process.gt2n {
        techfile::generate(&c);
    }
    offgrid::generate(&c);
    feol::generate(&c);
    mol::generate(&c);
    beol::generate(&c);
}
