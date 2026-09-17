// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A PDK outside the binary: found by name under a search directory, extending a
//! bundled base by its bare name, reading the base's decks through the embedded copy,
//! and shadowing a bundled process of the same name.

use gdscheck::pdk::{Origin, PdkConfig, list_processes, resolve_process};
use std::path::PathBuf;

/// A fresh directory of PDKs for one test, dropped with it.
struct Tree(PathBuf);

impl Tree {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("gdscheck-ext-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tree(dir)
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ACME: &str = "name: ACME on SG13G2
version: \"0.1\"
extends: ihp-sg13g2
decks:
  - name: activ
    path: ../ihp-sg13g2/decks/activ.yml
  - name: own
    path: decks/own.yml
";

const OWN: &str = "rules:
  - id: OWN.1
    check: min_width
    layers: [Activ]
    value: 5.0
";

#[test]
fn a_process_on_the_path_extends_a_bundled_base_by_name() {
    let t = Tree::new("acme");
    t.write("acme/pdk.yml", ACME);
    t.write("acme/decks/own.yml", OWN);
    let r = resolve_process("acme", std::slice::from_ref(&t.0)).unwrap();
    assert_eq!(r.origin, Origin::External(t.0.join("acme/pdk.yml")));
    let pdk = PdkConfig::for_process(&r.spec).unwrap();
    assert_eq!(pdk.name, "ACME on SG13G2");
    // The base's layers came along, and its deck reads through the embedded copy.
    assert!(
        pdk.layer("Activ").is_some(),
        "Activ inherited from the base"
    );
    assert!(!pdk.load_deck("activ").unwrap().is_empty());
    let own = pdk.load_deck("own").unwrap();
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].id, "OWN.1");
}

#[test]
fn a_process_on_the_path_shadows_a_bundled_one() {
    let t = Tree::new("shadow");
    t.write(
        "ihp-sg13g2/pdk.yml",
        "name: Shadow\nversion: \"0\"\nlayers:\n  - {name: Own, gds_layer: 1, gds_datatype: 0}\ndecks: []\n",
    );
    let r = resolve_process("ihp-sg13g2", std::slice::from_ref(&t.0)).unwrap();
    assert!(matches!(r.origin, Origin::External(_)));
    let pdk = PdkConfig::for_process(&r.spec).unwrap();
    assert_eq!(pdk.name, "Shadow");
    assert!(pdk.layer("Own").is_some() && pdk.layer("Activ").is_none());
    // Without the path the bundled one is what the name means.
    assert_eq!(
        resolve_process("ihp-sg13g2", &[]).unwrap().origin,
        Origin::Embedded
    );
    let listed = list_processes(std::slice::from_ref(&t.0));
    let names: Vec<&str> = listed.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names.iter().filter(|n| **n == "ihp-sg13g2").count(), 2);
    assert!(
        matches!(listed[0].1, Origin::External(_)),
        "the path comes first"
    );
}

#[test]
fn an_unknown_process_names_what_was_searched() {
    let t = Tree::new("unknown");
    let e = resolve_process("nope", std::slice::from_ref(&t.0)).unwrap_err();
    assert!(e.contains("ihp-sg13g2"), "{e}");
    assert!(e.contains(&t.0.display().to_string()), "{e}");
    // A path is a path: a directory is not a pdk.yml.
    let e = resolve_process(&t.0.display().to_string(), &[]).unwrap_err();
    assert!(e.contains("not a readable pdk.yml"), "{e}");
}
