# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# gdscheck developer tasks — run with `just <recipe>` (https://github.com/casey/just)

# List available recipes (default).
default:
    @just --list

# Run the full test suite (release, like CI).
test:
    cargo test --release

# The suite at a 7 µm tile: a result that holds at 20 and not at 7 depends on where
# the tile lines fall, which no result may.
test-tile:
    GDSCHECK_TILE_UM=7 cargo test --release

# Lint with clippy, warnings as errors.
clippy:
    cargo clippy --release -- -D warnings

# Build the release binary.
build:
    cargo build --release

# Regenerate every PDK's generated test fixtures.
gen-testdata: (gen-testdata-for "ihp-sg13g2") (gen-testdata-for "gf180mcuD")

# Regenerate one PDK's generated test fixtures.
gen-testdata-for pdk:
    cargo run --release --features dev-tools --bin gen-testdata -- --pdk pdks/{{pdk}}/pdk.yml

# Run the main suite over the reference designs of one PDK (needs ../reference-designs).
designs process="ihp-sg13g2":
    ci/run-designs.sh {{process}}

# Format, lint, and test — the pre-commit gate.
check: clippy test
    cargo fmt --check
