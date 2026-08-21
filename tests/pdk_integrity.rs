// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structural checks that every embedded PDK must satisfy, run over all of them.
//!
//! These catch the config mistakes that produce a *false clean* rather than an error —
//! the dangerous kind, because the run still reports success:
//!
//! * A connect-graph entry naming a layer that does not exist is silently dropped
//!   (`pdk.rs`, `from_yaml`), so net extraction quietly stops bridging through it and
//!   every net-aware rule downstream passes on nets that are wrong.
//! * An eager virtual layer whose source is itself virtual resolves to nothing:
//!   `compute_virtual_layers` reads only what the flattened layout already holds, and
//!   nothing warns.
//!
//! Each is cheap to assert and impossible to spot by reading a 600-line `pdk.yml`, so
//! they belong here rather than in any one PDK's test file.

use gdscheck::parse_virtual_op;
use gdscheck::pdk::{PdkConfig, VirtualMode};
use std::collections::HashSet;

/// A PDK's own `pdk.yml` as raw YAML.  The parsed `PdkConfig` has already discarded the
/// layer *names* in the connect graph (they are resolved to GDS keys, and unresolvable
/// ones are dropped), so checking those needs the source text.
fn raw_pdk_yml(process: &str) -> serde_yml::Value {
    let path = format!("{}/pdks/{}/pdk.yml", env!("CARGO_MANIFEST_DIR"), process);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_yml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Every embedded PDK is covered automatically, so adding one cannot skip these checks.
fn processes() -> Vec<&'static str> {
    let v = PdkConfig::embedded_processes();
    assert!(!v.is_empty(), "no embedded PDKs found");
    v
}

/// Every deck and suite the PDK advertises must actually load. `list-decks` reads the
/// index alone, so an unreadable or malformed deck file otherwise only surfaces when
/// someone runs that deck.
#[test]
fn every_declared_deck_and_suite_loads() {
    for process in processes() {
        let pdk = PdkConfig::for_process(process).unwrap_or_else(|e| panic!("{process}: {e}"));
        for deck in &pdk.decks {
            pdk.load_deck(&deck.name)
                .unwrap_or_else(|e| panic!("{process}: deck '{}': {e}", deck.name));
        }
        for suite in &pdk.suites {
            pdk.load_suite(&suite.name)
                .unwrap_or_else(|e| panic!("{process}: suite '{}': {e}", suite.name));
        }
    }
}

/// Every virtual layer's `op` must parse, with whatever `radius`/`min`/`max` the entry
/// carries. A typo'd op is already a hard error at run time (`run_drc`), but only once a
/// deck references that layer — assert it up front for all of them.
#[test]
fn every_virtual_layer_op_parses() {
    for process in processes() {
        let pdk = PdkConfig::for_process(process).unwrap();
        for vl in &pdk.virtual_layers {
            // Eager ops are dispatched by name in `compute_virtual_layers`, not through
            // `parse_virtual_op`, so only the lazy ones are checked here.
            if vl.mode != VirtualMode::Lazy {
                continue;
            }
            parse_virtual_op(&vl.op, vl.radius, vl.min, vl.max, 0.001)
                .unwrap_or_else(|e| panic!("{process}: virtual layer '{}': {e}", vl.name));
        }
    }
}

/// Every source layer a virtual layer names must resolve. An unresolved source is
/// reported to stderr at run time and the layer comes out empty, which reads as "no
/// violations" rather than as a broken PDK.
#[test]
fn every_virtual_layer_source_resolves() {
    for process in processes() {
        let pdk = PdkConfig::for_process(process).unwrap();
        for vl in &pdk.virtual_layers {
            assert!(
                pdk.layer(&vl.name).is_some(),
                "{process}: virtual layer '{}' has no assigned layer number",
                vl.name
            );
            for src in &vl.layers {
                assert!(
                    pdk.layer(src).is_some(),
                    "{process}: virtual layer '{}' names unknown source '{src}'",
                    vl.name
                );
            }
            assert!(
                !vl.layers.contains(&vl.name),
                "{process}: virtual layer '{}' references itself",
                vl.name
            );
        }
    }
}

/// A virtual layer chain is only legal when the *consumer* is lazy: eager layers are
/// materialised from the flattened layout in one pass, so a source that is itself
/// virtual has not been built yet and contributes nothing.
#[test]
fn eager_virtual_layers_do_not_chain() {
    for process in processes() {
        let pdk = PdkConfig::for_process(process).unwrap();
        let virtual_names: HashSet<&str> =
            pdk.virtual_layers.iter().map(|v| v.name.as_str()).collect();
        for vl in &pdk.virtual_layers {
            if vl.mode == VirtualMode::Lazy {
                continue;
            }
            for src in &vl.layers {
                assert!(
                    !virtual_names.contains(src.as_str()),
                    "{process}: eager virtual layer '{}' sources the virtual layer \
                     '{src}', which is not materialised yet — mark '{}' `mode: lazy`",
                    vl.name,
                    vl.name
                );
            }
        }
    }
}

/// Every name in the connect graph must resolve. `from_yaml` drops a spec naming an
/// unknown layer without complaint, so a single typo silently removes a level from the
/// via stack — and the antenna rules that cut the stack at a fixed level then read a
/// different net than the deck author meant.
#[test]
fn every_connect_graph_layer_resolves() {
    for process in processes() {
        let pdk = PdkConfig::for_process(process).unwrap();
        let raw = raw_pdk_yml(process);
        let Some(specs) = raw.get("connectivity").and_then(|c| c.as_sequence()) else {
            continue; // a PDK may declare no connect graph
        };

        for (i, spec) in specs.iter().enumerate() {
            let connector = spec
                .get("connector")
                .and_then(|c| c.as_str())
                .unwrap_or_else(|| panic!("{process}: connectivity[{i}] has no connector"));
            assert!(
                pdk.layer(connector).is_some(),
                "{process}: connectivity[{i}] names unknown connector '{connector}'"
            );
            let layers = spec
                .get("layers")
                .and_then(|l| l.as_sequence())
                .unwrap_or_else(|| panic!("{process}: connectivity[{i}] has no layers"));
            for l in layers {
                let name = l.as_str().unwrap();
                assert!(
                    pdk.layer(name).is_some(),
                    "{process}: connectivity[{i}] ('{connector}') names unknown layer '{name}'"
                );
            }
        }

        // Nothing was dropped on the way in: the resolved graph is as long as declared.
        assert_eq!(
            pdk.connectivity.len(),
            specs.len(),
            "{process}: {} of {} connect steps were dropped as unresolvable",
            specs.len() - pdk.connectivity.len(),
            specs.len()
        );
    }
}
