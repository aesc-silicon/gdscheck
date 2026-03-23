// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embed the `pdks/` tree (process definitions and their decks) into the binary
//! so `--process <name>` works without any external files.  Generates a static
//! slice of `(relative_path, file_contents)` via `include_str!`.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let pdks = Path::new(&manifest).join("pdks");
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut entries = String::new();
    visit(&pdks, &pdks, &mut entries);

    let generated = format!(
        "/// (relative path under `pdks/`, file contents) for every embedded PDK file.\n\
         pub static EMBEDDED_PDKS: &[(&str, &str)] = &[\n{entries}];\n"
    );
    fs::write(Path::new(&out_dir).join("embedded_pdks.rs"), generated).unwrap();

    println!("cargo:rerun-if-changed={}", pdks.display());
}

fn visit(root: &Path, dir: &Path, out: &mut String) {
    let mut items: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    items.sort();
    for path in items {
        if path.is_dir() {
            visit(root, &path, out);
        } else if path.extension().is_some_and(|x| x == "yml" || x == "yaml") {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let abs = path.to_string_lossy();
            writeln!(out, "    ({rel:?}, include_str!({abs:?})),").unwrap();
        }
    }
}
